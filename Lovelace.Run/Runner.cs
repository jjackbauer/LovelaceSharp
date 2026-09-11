using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization.Metadata;
using Lovelace.Dsp;
using Lovelace.Suite;

namespace Lovelace.Run;

/// <summary>
/// Lovelace.Run — a non-interactive script runner over Lovelace.Suite.
///
/// Evaluates a script (from --eval, --file, or --stdin) and emits a single JSON
/// envelope on stdout so it can be driven by scripts, tests, or the DSH plugin.
///
///   Lovelace.Run --eval "x = 1..10" --json
///   Lovelace.Run script.ls --plot-dir out
///
/// Protocol contract (versioned):
///   * stdout carries the envelope and NOTHING else. Anything the script prints
///     with print() is captured and returned in the envelope's "output" array.
///   * Every value is serialized structurally — arrays carry their shape, symbolic
///     values carry BOTH their canonical and pretty forms, complex values carry
///     re/im, domains are domain values, and an absent field is null.
///   * JSON keys are camelCase; structured record FIELD NAMES are snake_case.
///
/// Exit codes: 0 success, 1 script/diagnostic error, 2 usage error.
///
/// JSON is emitted via the source-generated RunJsonContext (see RunProtocol.cs), which is
/// required for Native AOT: the reflection-based serializer is trimmed away there.
///
/// The entry point takes its writers explicitly (rather than touching Console directly)
/// so a test can capture the envelope in-process; Program.cs passes Console.Out and
/// Console.Error, which is byte-for-byte what the runner emitted before the extraction.
/// </summary>
public static class Runner
{
    private const int ProtocolVersion = 1;
    private const int MathIrVersion = 2;

    /// <summary>Evaluates the script named by <paramref name="args"/> and writes the envelope to
    /// <paramref name="stdout"/>. Returns the process exit code (0 success, 1 script/diagnostic
    /// error, 2 usage error).</summary>
    /// <param name="args">Command-line arguments, exactly as the process received them.</param>
    /// <param name="stdout">Receives the JSON envelope (and nothing else) or the --text summary.</param>
    /// <param name="stderr">Receives usage and non-JSON error text.</param>
    /// <param name="stdin">Script source for --stdin; defaults to <see cref="Console.In"/>.</param>
    public static async Task<int> RunAsync(string[] args, TextWriter stdout, TextWriter stderr,
        TextReader? stdin = null)
    {
        stdin ??= Console.In;

        string? eval = null;
        string? file = null;
        string? plotDir = null;
        string? plotFile = null;
        bool stdinMode = false;
        bool json = true;
        bool omitFunctions = false;
        bool omitVariables = false;
        int? cancelAfterMs = null;
        int? printBudget = null;

        for (int i = 0; i < args.Length; i++)
        {
            switch (args[i])
            {
                case "--eval":
                    if (++i >= args.Length) return Usage(stderr, "--eval requires a script argument.");
                    eval = args[i];
                    break;
                case "--file":
                    if (++i >= args.Length) return Usage(stderr, "--file requires a path argument.");
                    file = args[i];
                    break;
                case "--stdin":
                    stdinMode = true;
                    break;
                case "--cancel-after":
                    if (++i >= args.Length) return Usage(stderr, "--cancel-after requires a millisecond argument.");
                    if (!int.TryParse(args[i], out int ms) || ms <= 0)
                        return Usage(stderr, "--cancel-after requires a positive millisecond count.");
                    cancelAfterMs = ms;
                    break;
                case "--omit-variables":
                    // payload control for agent loops, mirroring --omit-functions: the variables array
                    // is state the agent usually already tracks
                    omitVariables = true;
                    break;
                case "--print-budget":
                    if (++i >= args.Length) return Usage(stderr, "--print-budget requires a node count.");
                    if (!int.TryParse(args[i], out int nodes) || nodes <= 0)
                        return Usage(stderr, "--print-budget requires a positive node count.");
                    printBudget = nodes;
                    break;
                case "--omit-functions":
                    // the registry dump (~100 entries) is payload noise for agent loops
                    omitFunctions = true;
                    break;
                case "--plot-dir":
                    if (++i >= args.Length) return Usage(stderr, "--plot-dir requires a directory argument.");
                    plotDir = args[i];
                    break;
                case "--plot-file":
                    if (++i >= args.Length) return Usage(stderr, "--plot-file requires a name argument.");
                    plotFile = args[i];
                    break;
                case "--text":
                    json = false;
                    break;
                case "--json":
                    json = true;
                    break;
                case "--help":
                case "-h":
                    PrintUsage(stdout);
                    return 0;
                default:
                    // First bare argument is treated as a script file path.
                    if (file is null && !args[i].StartsWith('-'))
                    {
                        file = args[i];
                    }
                    else
                    {
                        return Usage(stderr, $"Unknown argument '{args[i]}'.");
                    }
                    break;
            }
        }

        string source;
        if (eval is not null)
        {
            source = eval;
        }
        else if (file is not null)
        {
            try
            {
                source = await File.ReadAllTextAsync(file);
            }
            catch (Exception ex)
            {
                // no engine ran and no statement executed, so the durations are zero and empty
                return WriteError(stdout, stderr, json, "FileReadError", "ParseError", $"Cannot read script file '{file}': {ex.Message}", recoverable: true, Array.Empty<DiagnosticDto>(), TimeSpan.Zero, Array.Empty<TimingDto>());
            }
        }
        else if (stdinMode)
        {
            source = await stdin.ReadToEndAsync();
        }
        else
        {
            return Usage(stderr, "No script provided. Use --eval <script>, --file <path>, --stdin, or a bare file path.");
        }

        var engine = new SuiteEngine();
        engine.LoadPlugin(new DspPlugin());
        var symbolics = new Lovelace.Symbolics.SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new Lovelace.MathIR.MathIRPlugin(symbolics));
        if (plotDir is not null) engine.PlotOutputDirectory = plotDir;
        if (plotFile is not null) engine.PlotFileName = plotFile;

        // plot() writes into PlotOutputDirectory without creating it, so ensure a
        // fresh --plot-dir works (a no-op when the directory already exists).
        Directory.CreateDirectory(engine.PlotOutputDirectory);

        // capture print() output: the envelope must be the only thing on stdout. Declared outside the
        // try so a cancelled evaluation can still return the output it produced (the partial result).
        var output = new StringWriter();
        using var cancellation = cancelAfterMs is { } budget
            ? new CancellationTokenSource(budget)
            : new CancellationTokenSource();

        try
        {
            var result = await engine.EvaluateAsync(
                ScriptSource.ToSemicolonStatements(source), output, cancellation.Token);

            var snapshot = engine.CaptureState();
            var variables = omitVariables
                ? Array.Empty<VariableDto>()
                : snapshot.Variables.Values
                    .OrderBy(v => v.Name, StringComparer.Ordinal)
                    .Select(v => new VariableDto(v.Name, v.Kind.ToString(), v.Display))
                    .ToArray();
            var functions = omitFunctions
                ? Array.Empty<FunctionDto>()
                : snapshot.Functions.Values
                    .OrderBy(f => f.Name, StringComparer.Ordinal)
                    .Select(f => new FunctionDto(f.Name, f.Parameters.ToArray(), f.IsBuiltin, f.Plugin))
                    .ToArray();

            PlotDto? plot = null;
            if (engine.LastPlot is { } capture)
            {
                string path = Path.GetFullPath(Path.Combine(engine.PlotOutputDirectory, engine.PlotFileName));
                plot = new PlotDto(path, capture.Title ?? string.Empty, capture.Svg ?? string.Empty);
            }

            ResultDto? resultPayload = result.Kind == ValueKind.Void
                ? null
                : new ResultDto(result.Kind.ToString(), ValueFormatter.Format(result), ValueFormatter.FormatTyped(result),
                    StructuredProjection.ToStructured(result, PrintBudgetOrNull(printBudget)));

            var envelope = new RunEnvelopeDto(
                ProtocolVersion,
                Lovelace.Symbolics.Printing.FormatHeader,
                MathIrVersion,
                true,
                snapshot.Revision,
                resultPayload,
                SplitLines(output.ToString()),
                variables,
                functions,
                plot,
                engine.LastElapsedDisplay,
                Duration(engine.LastElapsed),
                Timings(engine.OperationTimings));

            if (json)
                WriteJson(stdout, envelope, RunJsonContext.Default.RunEnvelopeDto);
            else
                PrintText(stdout, envelope);

            return 0;
        }
        catch (Exception ex)
        {
            var diagnostics = engine.Diagnostics
                .Select(d => new DiagnosticDto(d.Message, d.Position, d.Line, d.Column))
                .ToArray();
            var (code, category, recoverable) = Classify(ex);
            // a cancelled run is not a failed run: the host reports the structured status together
            // with everything the engine had already committed
            string[]? partialOutput = code == "Cancelled" ? SplitLines(output.ToString()) : null;
            VariableDto[]? partialVariables = code == "Cancelled"
                ? engine.CaptureState().Variables.Values
                    .OrderBy(v => v.Name, StringComparer.Ordinal)
                    .Select(v => new VariableDto(v.Name, v.Kind.ToString(), v.Display))
                    .ToArray()
                : null;
            // a failed evaluation reports the SAME durations a successful one does: the elapsed pair
            // from one unit selector and one timing entry per statement that ran before the failure
            return WriteError(stdout, stderr, json, code, category, ex.Message, recoverable, diagnostics,
                engine.LastElapsed, Timings(engine.OperationTimings), partialOutput, partialVariables);
        }
    }

    /// <summary>Maps a failure onto the stable error taxonomy. Message matching is never required
    /// by a consumer: the code and category are structural.</summary>
    private static (string Code, string Category, bool Recoverable) Classify(Exception ex) => ex switch
    {
        Lovelace.Suite.EvaluationCancelledException => ("Cancelled", "BudgetExceeded", true),
        Lovelace.Suite.ReentrancyNotSupportedException => ("ReentrancyNotSupported", "UnsupportedOperation", true),
        // a budget stop is a first-class outcome, not an invariant failure: the frozen contract makes
        // BudgetExceeded a status, and the caller can retry with a larger budget
        Lovelace.Symbolics.BudgetExceededException => ("BudgetExceeded", "BudgetExceeded", true),
        // A nesting refusal is a budget stop too, but it gets its OWN code: a consumer must be able
        // to tell "this input is nested too deeply" (a property of the input) from "the symbolic
        // computation ran out of steps" (a property of the work), and neither is a DomainError.
        Lovelace.Abstractions.InputDepthExceededException => ("DepthExceeded", "BudgetExceeded", true),
        Lovelace.Symbolics.EvaluationException => ("EvaluationError", "DomainError", true),
        Lovelace.Symbolics.AssumptionContradictionException => ("UnsatisfiableAssumptions", "DomainError", true),
        FormatException => ("InvalidInput", "ParseError", true),
        NotSupportedException => ("UnsupportedOperation", "UnsupportedOperation", true),
        InvalidOperationException => ("InvalidOperation", "DomainError", true),
        ArgumentException => ("InvalidArgument", "TypeMismatch", true),
        // framework types that still escape the kernel are classified honestly rather than being
        // reported as internal invariant failures: they are user-level errors and recoverable
        NotImplementedException => ("UnsupportedOperation", "UnsupportedOperation", true),
        DivideByZeroException => ("DivisionByZero", "DomainError", true),
        ArithmeticException => ("ArithmeticError", "DomainError", true),
        _ => ("InternalError", "InternalInvariantFailure", false),
    };

    private static int WriteError(TextWriter stdout, TextWriter stderr,
        bool json, string code, string category, string message, bool recoverable,
        DiagnosticDto[] diagnostics, TimeSpan elapsed, TimingDto[] timings,
        string[]? output = null, VariableDto[]? variables = null)
    {
        if (json)
        {
            // the error envelope carries the human string AND the structural duration produced by the
            // SAME unit selector (Timing.Scale behind both Timing.Format and Duration), so the two
            // forms can never disagree — exactly like the success envelope
            WriteJson(stdout, new RunErrorDto(ProtocolVersion, Lovelace.Symbolics.Printing.FormatHeader, MathIrVersion,
                false, code, category, message, recoverable, diagnostics,
                Lovelace.Suite.Timing.Format(elapsed), Duration(elapsed), timings, output, variables),
                RunJsonContext.Default.RunErrorDto);
        }
        else
        {
            stderr.WriteLine($"Error [{code}/{category}]: {message}");
        }
        return 1;
    }

    private static Lovelace.Symbolics.Printing.PrintBudget? PrintBudgetOrNull(int? maxNodes) =>
        maxNodes is { } nodes ? new Lovelace.Symbolics.Printing.PrintBudget(MaxNodes: nodes) : null;

    private static DurationDto Duration(TimeSpan elapsed)
    {
        var (value, unit) = Lovelace.Suite.Timing.Scale(elapsed);
        return new DurationDto(value, unit);
    }

    /// <summary>One wire timing per top-level statement that ran, in statement order.</summary>
    private static TimingDto[] Timings(IReadOnlyList<Lovelace.Suite.OperationTiming> timings) =>
        timings.Select(t => new TimingDto(
            t.Position,
            new DurationDto(t.ElapsedScale.Value, t.ElapsedScale.Unit),
            t.Result.Kind.ToString(),
            t.Output.Length > 0))
        .ToArray();

    /// <summary>
    /// The script's captured print output as LINES. The capture path terminates every line with
    /// <see cref="Environment.NewLine"/>, so on Windows each element but the last used to keep the
    /// \r of the terminator — an artifact of the host's line separator, not something the script
    /// printed. The envelope's output array carries the line, never its terminator.
    /// </summary>
    private static string[] SplitLines(string text)
    {
        if (text.Length == 0)
            return Array.Empty<string>();

        string[] lines = text.TrimEnd('\r', '\n').Split('\n');
        for (int i = 0; i < lines.Length; i++)
            lines[i] = lines[i].TrimEnd('\r');
        return lines;
    }

    private static void PrintText(TextWriter stdout, RunEnvelopeDto envelope)
    {
        var sb = new StringBuilder();
        if (envelope.Result is { } r)
            sb.AppendLine($"= {r.Typed}");
        foreach (var line in envelope.Output)
            sb.AppendLine(line);
        foreach (var v in envelope.Variables)
            sb.AppendLine($"  {v.Name} = {v.Display}");
        stdout.Write(sb.ToString());
    }

    private static void WriteJson(TextWriter stdout, object value, JsonTypeInfo typeInfo) =>
        stdout.WriteLine(JsonSerializer.Serialize(value, typeInfo));

    private static int Usage(TextWriter stderr, string message)
    {
        stderr.WriteLine($"Error: {message}");
        PrintUsage(stderr);
        return 2;
    }

    private static void PrintUsage(TextWriter writer)
    {
        writer.WriteLine(
            "Lovelace.Run — evaluate a Lovelace script and emit a JSON envelope.\n" +
            "\n" +
            "Usage:\n" +
            "  Lovelace.Run --eval \"<script>\" [options]\n" +
            "  Lovelace.Run <file.ls> [options]\n" +
            "  Lovelace.Run --stdin [options]\n" +
            "\n" +
            "Options:\n" +
            "  --eval <script>      evaluate the given script text\n" +
            "  --file <path>        read the script from a file\n" +
            "  --stdin              read the script from standard input\n" +
            "  --plot-dir <dir>     directory for plot() SVG output\n" +
            "  --plot-file <name>   filename for plot() SVG output (default: plot.svg)\n" +
            "  --omit-functions     omit the builtin registry from the envelope (agent loops)\n" +
            "  --omit-variables     omit the variables array from the envelope (agent loops)\n" +
            "  --print-budget <n>   abbreviate structured renderings beyond n nodes, reporting the truncation\n" +
            "  --cancel-after <ms>  cancel the evaluation after the given time, returning the partial result\n" +
            "  --json               emit JSON (default)\n" +
            "  --text               emit a human-readable summary\n" +
            "  --help, -h           show this help");
    }
}
