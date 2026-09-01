using System.Text;
using System.Text.RegularExpressions;
using Lovelace.Suite;
using Nat = global::Lovelace.Natural.Natural;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Console.Repl;

/// <summary>
/// Orchestrates the interactive REPL over the <see cref="SuiteEngine"/>: reads
/// input (with multi-line block accumulation), dispatches special commands, and
/// prints results. All language logic lives in <c>Lovelace.Suite</c>.
/// </summary>
public sealed class ReplSession
{
    private readonly SuiteEngine _engine = new();
    private readonly LineEditor _lineEditor = new();
    private bool _exitRequested;

    // -----------------------------------------------------------------
    // Help text
    // -----------------------------------------------------------------

    private const string HelpText = """
        LovelaceSharp REPL — help
        ─────────────────────────────────────────────────────────────────
        Statements:
          func name(a, b) { … }   define a function (or: func f(x) = expr)
          if (c) { … } else { … } conditional
          while (c) { … }         loop          for i in a..b { … }  loop
          return [expr]           return from a function
          print(expr)             write a value (interpolate with $"{expr}")

        Operators (high to low precedence):
          !         postfix factorial              e.g.  5!
          [i]       index (0-based)                e.g.  v[0], m[i, j]
          - +       unary negation / identity      e.g.  -x
          ..        range (inclusive)              e.g.  1..5, 1..2..7
          ^         power (right-associative)      e.g.  2 ^ 10
          * / %     multiplicative                 e.g.  a * b
          + -       additive                       e.g.  a + b
          == != > < >= <=   comparison             e.g.  a > b
          =         assignment (right-assoc)       e.g.  x = 42

        Arrays:
          [1, 2, 3]  vector          [[1, 2], [3, 4]]  matrix
          [[[1,2],[3,4]],[[5,6],[7,8]]]  rank-3 (N-D)
          sum(a[, axis])  prod(a[, axis])  min(a[, axis])  max(a[, axis])
          mean(a[, axis])  norm(a[, axis])
          dot(a, b)  cross(a, b)  matmul(a, b)  det(m)  inv(m)  trace(m)
          zeros(d…)  ones(d…)  eye(n)  reshape(a, d…)  shape(a)  rank(a)
          numel(a)  len(a)  flatten(a)  transpose(a[, perm])  squeeze(a)
          concat(a, b[, axis])  append(a, b)

        Built-in functions:
          abs(x)  inv(x)  divrem(a, b)  is_even(x)  is_odd(x)  sign(x)
          sqrt(x)  pi([digits])  setprecision(n)  print(x)  plot(y) / plot(x, y[, "title"])

        Special commands:
          vars                     list all variables
          funcs                    list all functions
          clear                    delete all variables
          delete <name>            delete one variable
          run <file>               execute a script file
          set precision <n>        Real computation decimal places
          set display <n>          Real / Natural display digits
          help                     show this text
          exit / quit              leave the REPL
        ─────────────────────────────────────────────────────────────────
        """;

    // -----------------------------------------------------------------
    // Public entry point
    // -----------------------------------------------------------------

    public async Task RunAsync()
    {
        while (true)
        {
            string? line = _lineEditor.ReadLine("» ");
            if (line is null) break;

            string trimmed = line.Trim();
            if (string.IsNullOrEmpty(trimmed)) continue;

            // Accumulate continuation lines while braces are unbalanced.
            var buffer = new StringBuilder(trimmed);
            while (!BracesBalanced(buffer.ToString()))
            {
                string? more = _lineEditor.ReadLine("… ");
                if (more is null)
                {
                    System.Console.WriteLine("Bye!");
                    return;
                }
                buffer.Append('\n').Append(more);
            }

            string source = buffer.ToString();

            if (await HandleSpecialCommandAsync(source))
            {
                if (_exitRequested) break;
                continue;
            }

            try
            {
                var result = await _engine.EvaluateAsync(source);
                if (result.Kind != ValueKind.Void)
                    PrintResult(result, _engine.LastElapsedDisplay);
            }
            catch (Exception ex)
            {
                PrintError(source, ex.Message, _engine.LastElapsedDisplay);
            }
        }

        System.Console.WriteLine("Bye!");
    }

    // -----------------------------------------------------------------
    // Special command dispatcher
    // -----------------------------------------------------------------

    private async Task<bool> HandleSpecialCommandAsync(string source)
    {
        if (source is "exit" or "quit")
        {
            _exitRequested = true;
            return true;
        }

        if (source is "help")
        {
            System.Console.WriteLine(HelpText);
            return true;
        }

        if (source is "vars")
        {
            PrintVars();
            return true;
        }

        if (source is "funcs")
        {
            PrintFuncs();
            return true;
        }

        if (source is "clear")
        {
            _engine.Clear();
            System.Console.WriteLine("All variables cleared.");
            return true;
        }

        if (source.StartsWith("delete ", StringComparison.Ordinal))
        {
            string name = source["delete ".Length..].Trim();
            if (_engine.RemoveVariable(name))
                System.Console.WriteLine($"Variable '{name}' deleted.");
            else
                System.Console.WriteLine($"Variable '{name}' is not defined.");
            return true;
        }

        if (source.StartsWith("set precision ", StringComparison.Ordinal))
        {
            string rest = source["set precision ".Length..].Trim();
            if (long.TryParse(rest, out long n) && n > 0)
            {
                Rl.MaxComputationDecimalPlaces = n;
                System.Console.WriteLine($"Computation precision set to {n} decimal places.");
            }
            else
            {
                System.Console.WriteLine($"Invalid argument '{rest}': expected a positive integer.");
            }
            return true;
        }

        if (source.StartsWith("set display ", StringComparison.Ordinal))
        {
            string rest = source["set display ".Length..].Trim();
            if (long.TryParse(rest, out long n) && n > 0)
            {
                Rl.DisplayDecimalPlaces = n;
                Nat.DisplayDigits = n;
                System.Console.WriteLine($"Display digits set to {n}.");
            }
            else
            {
                System.Console.WriteLine($"Invalid argument '{rest}': expected a positive integer.");
            }
            return true;
        }

        if (source.StartsWith("run ", StringComparison.Ordinal))
        {
            string path = source["run ".Length..].Trim().Trim('"');
            try
            {
                string content = File.ReadAllText(path);
                var result = await _engine.EvaluateAsync(content);
                if (result.Kind != ValueKind.Void)
                    PrintResult(result, _engine.LastElapsedDisplay);
            }
            catch (Exception ex)
            {
                PrintError(source, ex.Message, _engine.LastElapsedDisplay);
            }
            return true;
        }

        return false;
    }

    // -----------------------------------------------------------------
    // Output helpers
    // -----------------------------------------------------------------

    private static void PrintResult(Value result, string elapsed) =>
        System.Console.WriteLine($"= {ValueFormatter.FormatTyped(result)}   [{elapsed}]");

    private void PrintVars()
    {
        var vars = _engine.Variables;
        if (vars.Count == 0)
        {
            System.Console.WriteLine("(no variables defined)");
            return;
        }

        foreach (var (name, value) in vars.OrderBy(kv => kv.Key))
            System.Console.WriteLine($"  {name} = {ValueFormatter.FormatTyped(value)}");
    }

    private void PrintFuncs()
    {
        var funcs = _engine.Functions;
        if (funcs.Count == 0)
        {
            System.Console.WriteLine("(no functions defined)");
            return;
        }

        foreach (var (name, fn) in funcs.OrderBy(kv => kv.Key))
        {
            string suffix = fn.IsBuiltin ? " [builtin]" : string.Empty;
            System.Console.WriteLine($"  {fn.Name}({string.Join(", ", fn.Parameters)}){suffix}");
        }
    }

    /// <summary>
    /// Prints an error message, with a caret under the error position when one
    /// can be extracted from the message.
    /// </summary>
    private static void PrintError(string input, string message, string elapsed)
    {
        var match = Regex.Match(message, @"at position (\d+)", RegexOptions.IgnoreCase);
        if (match.Success && int.TryParse(match.Groups[1].Value, out int pos))
        {
            System.Console.WriteLine(input);
            System.Console.WriteLine(new string(' ', pos) + "^");
        }

        System.Console.WriteLine($"Error: {message}   [{elapsed}]");
    }

    /// <summary>True when braces are balanced (used for multi-line accumulation).</summary>
    private static bool BracesBalanced(string text)
    {
        int depth = 0;
        foreach (char c in text)
        {
            if (c == '{') depth++;
            else if (c == '}') depth--;
        }
        return depth <= 0;
    }
}
