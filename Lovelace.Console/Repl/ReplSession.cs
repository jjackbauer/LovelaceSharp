using System.Text;
using System.Text.RegularExpressions;
using Lovelace.Dsp;
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

    public ReplSession()
    {
        _engine.LoadPlugin(new DspPlugin());
        var symbolics = new Lovelace.Symbolics.SymbolicsPlugin();
        _engine.LoadPlugin(symbolics);
        _engine.LoadPlugin(new Lovelace.MathIR.MathIRPlugin(symbolics));
    }

    // -----------------------------------------------------------------
    // Help / discoverability (derived from the live builtin registry —
    // no hard-coded help text; HelpService is the single renderer)
    // -----------------------------------------------------------------

    private void PrintHelp(string? arg)
    {
        var help = _engine.Help;
        if (arg is null)
        {
            System.Console.WriteLine(help.Overview());
            return;
        }
        System.Console.WriteLine(help.Lookup(arg.Trim()) ?? $"No help for '{arg}'. Try 'help' for the category list.");
    }

    private void PrintFuncs(string? category)
    {
        System.Console.WriteLine(_engine.Help.Funcs(category));
    }

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
            PrintHelp(null);
            return true;
        }

        if (source.StartsWith("help ", StringComparison.Ordinal))
        {
            PrintHelp(source["help ".Length..]);
            return true;
        }

        if (source is "vars")
        {
            PrintVars();
            return true;
        }

        if (source is "funcs")
        {
            PrintFuncs(null);
            return true;
        }

        if (source.StartsWith("funcs ", StringComparison.Ordinal))
        {
            PrintFuncs(source["funcs ".Length..].Trim());
            return true;
        }

        if (source is "set pretty unicode")
        {
            _engine.UnicodeOutput = true;
            System.Console.WriteLine("Pretty output: unicode.");
            return true;
        }

        if (source is "set pretty ascii")
        {
            _engine.UnicodeOutput = false;
            System.Console.WriteLine("Pretty output: ascii.");
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
                _engine.ComputationDecimalPlaces = n;
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
                _engine.DisplayDecimalPlaces = n;
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
