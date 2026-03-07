using System.Text.RegularExpressions;
using Nat = global::Lovelace.Natural.Natural;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Console.Repl;

/// <summary>
/// Orchestrates the interactive REPL: reads a line, tokenizes, parses, evaluates,
/// and prints the result. Also handles built-in special commands.
/// </summary>
public sealed class ReplSession
{
    // -----------------------------------------------------------------
    // Fields
    // -----------------------------------------------------------------

    private readonly Evaluator _evaluator = new();
    private readonly LineEditor _lineEditor = new();
    private readonly Tokenizer _tokenizer = new();
    private readonly Parser _parser = new();

    // -----------------------------------------------------------------
    // Help text
    // -----------------------------------------------------------------

    private const string HelpText = """
        LovelaceSharp REPL — help
        ─────────────────────────────────────────────────────────────────
        Operators (high to low precedence):
          !         postfix factorial              e.g.  5!
          - +       unary negation / identity      e.g.  -x
          ^         power (right-associative)      e.g.  2 ^ 10
          * / % *   multiplicative                 e.g.  a * b
          + -       additive                       e.g.  a + b
          == != > < >= <=   comparison             e.g.  a > b
          =         assignment (right-assoc)       e.g.  x = 42

        Type inference:
          Whole-number literals → Natural
          Decimal / periodic literals (3.14, 0.(3)) → Real
          Results auto-widen: Natural → Integer → Real as needed

        Built-in functions:
          abs(x)         absolute value
          inv(x)         multiplicative inverse (1 / x)
          divrem(a, b)   integer division with remainder
          is_even(x)     true when x is even
          is_odd(x)      true when x is odd
          sign(x)        -1 / 0 / +1

        Special commands:
          vars                     list all variables
          clear                    delete all variables
          delete <name>            delete one variable
          set precision <n>        Real computation decimal places
          set display <n>          Real / Natural display digits
          help                     show this text
          exit / quit              leave the REPL
        ─────────────────────────────────────────────────────────────────
        """;

    // -----------------------------------------------------------------
    // Public entry point
    // -----------------------------------------------------------------

    /// <summary>
    /// Starts the REPL loop. Exits when the user types <c>exit</c>, <c>quit</c>,
    /// or presses Ctrl+C.
    /// </summary>
    public void Run()
    {
        Value? lastResult = null;

        while (true)
        {
            string? line = _lineEditor.ReadLine("» ");

            // Ctrl+C → exit
            if (line is null)
            {
                System.Console.WriteLine("Bye!");
                return;
            }

            string trimmed = line.Trim();
            if (string.IsNullOrEmpty(trimmed)) continue;

            // ── Special commands ─────────────────────────────────────────
            if (HandleSpecialCommand(trimmed, ref lastResult))
                continue;

            // ── Expression evaluation ─────────────────────────────────────
            try
            {
                var tokens = _tokenizer.Tokenize(trimmed);
                var expr = _parser.Parse(tokens);
                var result = _evaluator.Evaluate(expr);

                // Store in _ (last-result variable)
                lastResult = result;
                _evaluator.Evaluate(new AssignExpr("_", new LiteralExpr(GetRawText(result))));

                PrintResult(result);
            }
            catch (Exception ex)
            {
                PrintError(trimmed, ex.Message);
            }
        }
    }

    // -----------------------------------------------------------------
    // Special command dispatcher
    // -----------------------------------------------------------------

    /// <summary>
    /// Handles built-in REPL commands. Returns <see langword="true"/> if the
    /// line was a special command (and was handled), <see langword="false"/> if it
    /// should be treated as an expression.
    /// </summary>
    private bool HandleSpecialCommand(string line, ref Value? lastResult)
    {
        // exit / quit
        if (line is "exit" or "quit")
        {
            System.Console.WriteLine("Bye!");
            System.Environment.Exit(0);
            return true;
        }

        // help
        if (line is "help")
        {
            System.Console.WriteLine(HelpText);
            return true;
        }

        // vars
        if (line is "vars")
        {
            PrintVars();
            return true;
        }

        // clear
        if (line is "clear")
        {
            _evaluator.Clear();
            lastResult = null;
            System.Console.WriteLine("All variables cleared.");
            return true;
        }

        // delete <name>
        if (line.StartsWith("delete ", StringComparison.Ordinal))
        {
            string name = line["delete ".Length..].Trim();
            if (_evaluator.Remove(name))
                System.Console.WriteLine($"Variable '{name}' deleted.");
            else
                System.Console.WriteLine($"Variable '{name}' is not defined.");
            return true;
        }

        // set precision <n>
        if (line.StartsWith("set precision ", StringComparison.Ordinal))
        {
            string rest = line["set precision ".Length..].Trim();
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

        // set display <n>
        if (line.StartsWith("set display ", StringComparison.Ordinal))
        {
            string rest = line["set display ".Length..].Trim();
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

        return false;
    }

    // -----------------------------------------------------------------
    // Output helpers
    // -----------------------------------------------------------------

    private static void PrintResult(Value result)
    {
        string display = result.Kind switch
        {
            ValueKind.Natural => $"= {result.AsNatural()} (Natural)",
            ValueKind.Integer => $"= {result.AsInteger()} (Integer)",
            ValueKind.Real    => $"= {result.AsReal()} (Real)",
            ValueKind.Boolean => $"= {result.AsBoolean()} (Boolean)",
            ValueKind.Text    => $"= {result.AsText()}",
            _ => $"= {result}",
        };
        System.Console.WriteLine(display);
    }

    private void PrintVars()
    {
        var vars = _evaluator.Variables;
        if (vars.Count == 0)
        {
            System.Console.WriteLine("(no variables defined)");
            return;
        }

        foreach (var (name, value) in vars.OrderBy(kv => kv.Key))
        {
            string display = value.Kind switch
            {
                ValueKind.Natural => $"  {name} = {value.AsNatural()} (Natural)",
                ValueKind.Integer => $"  {name} = {value.AsInteger()} (Integer)",
                ValueKind.Real    => $"  {name} = {value.AsReal()} (Real)",
                ValueKind.Boolean => $"  {name} = {value.AsBoolean()} (Boolean)",
                ValueKind.Text    => $"  {name} = {value.AsText()}",
                _ => $"  {name} = {value}",
            };
            System.Console.WriteLine(display);
        }
    }

    /// <summary>
    /// Prints an error message, with a caret (<c>^</c>) under the error position
    /// when a position can be extracted from <paramref name="message"/>.
    /// </summary>
    private static void PrintError(string input, string message)
    {
        // Try to extract "at position N" from the exception message.
        var match = Regex.Match(message, @"at position (\d+)", RegexOptions.IgnoreCase);
        if (match.Success && int.TryParse(match.Groups[1].Value, out int pos))
        {
            System.Console.WriteLine(input);
            System.Console.WriteLine(new string(' ', pos) + "^");
        }

        System.Console.WriteLine($"Error: {message}");
    }

    // -----------------------------------------------------------------
    // Last-result helper
    // -----------------------------------------------------------------

    /// <summary>
    /// Returns a raw-text representation of <paramref name="value"/> suitable for
    /// storing back via a <see cref="LiteralExpr"/> so the evaluator's type inference
    /// reconstructs the correct kind for <c>_</c>.
    /// </summary>
    private static string GetRawText(Value value) => value.Kind switch
    {
        ValueKind.Natural => value.AsNatural().ToString(),
        ValueKind.Integer => value.AsInteger().ToString(),
        ValueKind.Real    => value.AsReal().ToString(),
        ValueKind.Boolean => value.AsBoolean() ? "1" : "0",
        ValueKind.Text    => value.AsText(),
        _ => value.ToString() ?? string.Empty,
    };
}
