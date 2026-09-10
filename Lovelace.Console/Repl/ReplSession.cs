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
/// <remarks>
/// The session is host-agnostic: input comes from the <see cref="LineEditor"/>
/// seam and every line of output goes to the injected <see cref="TextWriter"/>.
/// The parameterless constructor keeps the original console behaviour
/// (<see cref="System.Console.In"/> / <see cref="System.Console.Out"/> and
/// key-by-key interactive editing); injecting a <see cref="StringReader"/> and a
/// <see cref="StringWriter"/> yields a fully headless session whose transcript can
/// be asserted on.
/// </remarks>
public sealed class ReplSession
{
    private readonly SuiteEngine _engine = new();
    private readonly LineEditor _lineEditor;
    private readonly TextWriter _output;
    private bool _exitRequested;

    /// <summary>Creates a session bound to the real console (interactive editing,
    /// output to <see cref="System.Console.Out"/>).</summary>
    public ReplSession()
        : this(null, null)
    {
    }

    /// <summary>
    /// Creates a session over an injected input/output pair. Passing <c>null</c> for
    /// either stream falls back to <see cref="System.Console.In"/> /
    /// <see cref="System.Console.Out"/> and keeps the interactive console behaviour.
    /// With a real <paramref name="input"/> the line editor reads with
    /// <see cref="TextReader.ReadLine"/> instead of <c>Console.ReadKey</c>, so the
    /// REPL can be driven headlessly.
    /// </summary>
    /// <param name="input">Command source, or <c>null</c> for the real console.</param>
    /// <param name="output">Transcript sink, or <c>null</c> for the real console.</param>
    public ReplSession(TextReader? input, TextWriter? output)
    {
        _output = output ?? System.Console.Out;
        _lineEditor = input is null ? new LineEditor() : new LineEditor(input, _output);

        // Script-level 'print' output belongs to the same transcript as REPL output.
        _engine.Output = _output;

        _engine.LoadPlugin(new DspPlugin());
        var symbolics = new Lovelace.Symbolics.SymbolicsPlugin();
        _engine.LoadPlugin(symbolics);
        _engine.LoadPlugin(new Lovelace.MathIR.MathIRPlugin(symbolics));
    }

    /// <summary>The sink every line of session output is written to.</summary>
    public TextWriter Output => _output;

    // -----------------------------------------------------------------
    // Help / discoverability (derived from the live builtin registry —
    // no hard-coded help text; HelpService is the single renderer)
    // -----------------------------------------------------------------

    private void PrintHelp(string? arg)
    {
        var help = _engine.Help;
        if (arg is null)
        {
            _output.WriteLine(help.Overview());
            return;
        }
        _output.WriteLine(help.Lookup(arg.Trim()) ?? $"No help for '{arg}'. Try 'help' for the category list.");
    }

    private void PrintFuncs(string? category)
    {
        _output.WriteLine(_engine.Help.Funcs(category));
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
                    _output.WriteLine("Bye!");
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

        _output.WriteLine("Bye!");
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
            _output.WriteLine("Pretty output: unicode.");
            return true;
        }

        if (source is "set pretty ascii")
        {
            _engine.UnicodeOutput = false;
            _output.WriteLine("Pretty output: ascii.");
            return true;
        }

        if (source is "clear")
        {
            _engine.Clear();
            _output.WriteLine("All variables cleared.");
            return true;
        }

        if (source.StartsWith("delete ", StringComparison.Ordinal))
        {
            string name = source["delete ".Length..].Trim();
            if (_engine.RemoveVariable(name))
                _output.WriteLine($"Variable '{name}' deleted.");
            else
                _output.WriteLine($"Variable '{name}' is not defined.");
            return true;
        }

        if (source.StartsWith("set precision ", StringComparison.Ordinal))
        {
            string rest = source["set precision ".Length..].Trim();
            if (long.TryParse(rest, out long n) && n > 0)
            {
                _engine.ComputationDecimalPlaces = n;
                _engine.DisplayDecimalPlaces = n;
                _output.WriteLine($"Computation precision set to {n} decimal places.");
            }
            else
            {
                _output.WriteLine($"Invalid argument '{rest}': expected a positive integer.");
            }
            return true;
        }

        if (source.StartsWith("set display ", StringComparison.Ordinal))
        {
            string rest = source["set display ".Length..].Trim();
            if (long.TryParse(rest, out long n) && n > 0)
            {
                _engine.DisplayDecimalPlaces = n;
                _output.WriteLine($"Display digits set to {n}.");
            }
            else
            {
                _output.WriteLine($"Invalid argument '{rest}': expected a positive integer.");
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

    /// <summary>Results render through the ENGINE's formatting context: the host's Unicode and
    /// display-precision settings are honoured instead of being silently bypassed by the static
    /// formatter's defaults.</summary>
    private void PrintResult(Value result, string elapsed) =>
        _output.WriteLine($"= {_engine.FormatValueTyped(result)}   [{elapsed}]");

    private void PrintVars()
    {
        var vars = _engine.Variables;
        if (vars.Count == 0)
        {
            _output.WriteLine("(no variables defined)");
            return;
        }

        foreach (var (name, value) in vars.OrderBy(kv => kv.Key, StringComparer.Ordinal))
            _output.WriteLine($"  {name} = {_engine.FormatValueTyped(value)}");
    }

    /// <summary>
    /// Prints an error message, with a caret under the error position when one
    /// can be extracted from the message.
    /// </summary>
    private void PrintError(string input, string message, string elapsed)
    {
        var match = Regex.Match(message, @"at position (\d+)", RegexOptions.IgnoreCase);
        if (match.Success && int.TryParse(match.Groups[1].Value, out int pos))
        {
            _output.WriteLine(input);
            _output.WriteLine(new string(' ', pos) + "^");
        }

        _output.WriteLine($"Error: {message}   [{elapsed}]");
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
