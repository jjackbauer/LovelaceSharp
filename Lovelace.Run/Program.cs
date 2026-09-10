using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Text.Json.Serialization.Metadata;
using Lovelace.Dsp;
using Lovelace.Suite;

// Protocol constants are declared before the entry statement: a declaration after "return" is
// unreachable code (CS0162), and the duplicated using was a standing CS0105 warning.
const int ProtocolVersion = 1;
const int MathIrVersion = 2;

return await ProgramMain(args);

// ---------------------------------------------------------------------------
// Lovelace.Run — a non-interactive script runner over Lovelace.Suite.
//
// Evaluates a script (from --eval, --file, or --stdin) and emits a single JSON
// envelope on stdout so it can be driven by scripts, tests, or the DSH plugin.
//
//   Lovelace.Run --eval "x = 1..10" --json
//   Lovelace.Run script.ls --plot-dir out
//
// Protocol contract (versioned):
//   * stdout carries the envelope and NOTHING else. Anything the script prints
//     with print() is captured and returned in the envelope's "output" array.
//   * Every value is serialized structurally — arrays carry their shape, symbolic
//     values carry BOTH their canonical and pretty forms, complex values carry
//     re/im, domains are domain values, and an absent field is null.
//   * JSON keys are camelCase; structured record FIELD NAMES are snake_case.
//
// Exit codes: 0 success, 1 script/diagnostic error, 2 usage error.
//
// JSON is emitted via the source-generated RunJsonContext (see below), which is
// required for Native AOT: the reflection-based serializer is trimmed away there.
// ---------------------------------------------------------------------------

static async Task<int> ProgramMain(string[] args)
{
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
                if (++i >= args.Length) return Usage("--eval requires a script argument.");
                eval = args[i];
                break;
            case "--file":
                if (++i >= args.Length) return Usage("--file requires a path argument.");
                file = args[i];
                break;
            case "--stdin":
                stdinMode = true;
                break;
            case "--cancel-after":
                if (++i >= args.Length) return Usage("--cancel-after requires a millisecond argument.");
                if (!int.TryParse(args[i], out int ms) || ms <= 0)
                    return Usage("--cancel-after requires a positive millisecond count.");
                cancelAfterMs = ms;
                break;
            case "--omit-variables":
                // payload control for agent loops, mirroring --omit-functions: the variables array
                // is state the agent usually already tracks
                omitVariables = true;
                break;
            case "--print-budget":
                if (++i >= args.Length) return Usage("--print-budget requires a node count.");
                if (!int.TryParse(args[i], out int nodes) || nodes <= 0)
                    return Usage("--print-budget requires a positive node count.");
                printBudget = nodes;
                break;
            case "--omit-functions":
                // the registry dump (~100 entries) is payload noise for agent loops
                omitFunctions = true;
                break;
            case "--plot-dir":
                if (++i >= args.Length) return Usage("--plot-dir requires a directory argument.");
                plotDir = args[i];
                break;
            case "--plot-file":
                if (++i >= args.Length) return Usage("--plot-file requires a name argument.");
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
                PrintUsage(Console.Out);
                return 0;
            default:
                // First bare argument is treated as a script file path.
                if (file is null && !args[i].StartsWith('-'))
                {
                    file = args[i];
                }
                else
                {
                    return Usage($"Unknown argument '{args[i]}'.");
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
            return WriteError(json, "FileReadError", "ParseError", $"Cannot read script file '{file}': {ex.Message}", recoverable: true, Array.Empty<DiagnosticDto>(), "0 ms");
        }
    }
    else if (stdinMode)
    {
        source = await Console.In.ReadToEndAsync();
    }
    else
    {
        return Usage("No script provided. Use --eval <script>, --file <path>, --stdin, or a bare file path.");
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
            engine.OperationTimings
                .Select(t => new TimingDto(
                    t.Position,
                    new DurationDto(t.ElapsedScale.Value, t.ElapsedScale.Unit),
                    t.Result.Kind.ToString(),
                    t.Output.Length > 0))
                .ToArray());

        if (json)
            WriteJson(envelope, RunJsonContext.Default.RunEnvelopeDto);
        else
            PrintText(envelope);

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
        return WriteError(json, code, category, ex.Message, recoverable, diagnostics,
            engine.LastElapsedDisplay, partialOutput, partialVariables);
    }
}

/// <summary>Maps a failure onto the stable error taxonomy. Message matching is never required
/// by a consumer: the code and category are structural.</summary>
static (string Code, string Category, bool Recoverable) Classify(Exception ex) => ex switch
{
    Lovelace.Suite.EvaluationCancelledException => ("Cancelled", "BudgetExceeded", true),
    Lovelace.Suite.ReentrancyNotSupportedException => ("ReentrancyNotSupported", "UnsupportedOperation", true),
    // a budget stop is a first-class outcome, not an invariant failure: the frozen contract makes
    // BudgetExceeded a status, and the caller can retry with a larger budget
    Lovelace.Symbolics.BudgetExceededException => ("BudgetExceeded", "BudgetExceeded", true),
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

static int WriteError(bool json, string code, string category, string message, bool recoverable,
    DiagnosticDto[] diagnostics, string elapsed,
    string[]? output = null, VariableDto[]? variables = null)
{
    if (json)
    {
        WriteJson(new RunErrorDto(ProtocolVersion, Lovelace.Symbolics.Printing.FormatHeader, MathIrVersion,
            false, code, category, message, recoverable, diagnostics, elapsed, output, variables),
            RunJsonContext.Default.RunErrorDto);
    }
    else
    {
        Console.Error.WriteLine($"Error [{code}/{category}]: {message}");
    }
    return 1;
}

static Lovelace.Symbolics.Printing.PrintBudget? PrintBudgetOrNull(int? maxNodes) =>
    maxNodes is { } nodes ? new Lovelace.Symbolics.Printing.PrintBudget(MaxNodes: nodes) : null;

static DurationDto Duration(TimeSpan elapsed)
{
    var (value, unit) = Lovelace.Suite.Timing.Scale(elapsed);
    return new DurationDto(value, unit);
}

static string[] SplitLines(string text) =>
    text.Length == 0 ? Array.Empty<string>() : text.TrimEnd('\r', '\n').Split('\n');

static void PrintText(RunEnvelopeDto envelope)
{
    var sb = new StringBuilder();
    if (envelope.Result is { } r)
        sb.AppendLine($"= {r.Typed}");
    foreach (var line in envelope.Output)
        sb.AppendLine(line);
    foreach (var v in envelope.Variables)
        sb.AppendLine($"  {v.Name} = {v.Display}");
    Console.Write(sb.ToString());
}

static void WriteJson(object value, JsonTypeInfo typeInfo) =>
    Console.WriteLine(JsonSerializer.Serialize(value, typeInfo));

static int Usage(string message)
{
    Console.Error.WriteLine($"Error: {message}");
    PrintUsage(Console.Error);
    return 2;
}

static void PrintUsage(TextWriter writer)
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

// ---------------------------------------------------------------------------
// Structured value projection lives in Lovelace.Suite.StructuredProjection: ONE
// implementation shared by this runner and Studio, so an agent and the IDE see
// identical structure.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// JSON envelope DTOs. These replace the anonymous types used previously, which
// the reflection-based serializer cannot serialize under Native AOT.
// ---------------------------------------------------------------------------
internal sealed record VariableDto(string Name, string Kind, string Display);

/// <summary>A duration as a unit-scaled value plus its unit, so an agent never parses a suffix.</summary>
internal sealed record DurationDto(double Value, string Unit);

/// <summary>One timed top-level statement: its zero-based source position, the elapsed time as a
/// structured duration, the kind of the value it produced, and whether it wrote print output.</summary>
internal sealed record TimingDto(int Position, DurationDto Elapsed, string ResultKind, bool HasOutput);
internal sealed record FunctionDto(string Name, string[] Parameters, bool Builtin, string? Plugin);
internal sealed record PlotDto(string Path, string Title, string Svg);
internal sealed record ResultDto(string Kind, string Display, string Typed, StructuredValueDto Structured);
internal sealed record DiagnosticDto(string Message, int Position, int Line, int Column);
internal sealed record RunEnvelopeDto(
    int ProtocolVersion,
    string SymbolicFormatVersion,
    int MathIrVersion,
    bool Ok,
    long Revision,
    ResultDto? Result,
    string[] Output,
    VariableDto[] Variables,
    FunctionDto[] Functions,
    PlotDto? Plot,
    string Elapsed,
    DurationDto ElapsedTime,
    TimingDto[] Timings);
internal sealed record RunErrorDto(
    int ProtocolVersion,
    string SymbolicFormatVersion,
    int MathIrVersion,
    bool Ok,
    string Code,
    string Category,
    string Message,
    bool Recoverable,
    DiagnosticDto[] Diagnostics,
    string Elapsed,
    // present only for a cancelled evaluation: everything the engine had already committed
    string[]? PartialOutput = null,
    VariableDto[]? PartialVariables = null);

[JsonSourceGenerationOptions(PropertyNamingPolicy = JsonKnownNamingPolicy.CamelCase,
    DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull)]
[JsonSerializable(typeof(RunEnvelopeDto))]
[JsonSerializable(typeof(RunErrorDto))]
[JsonSerializable(typeof(ResultDto))]
[JsonSerializable(typeof(StructuredValueDto))]
[JsonSerializable(typeof(StructuredFieldDto))]
[JsonSerializable(typeof(VariableDto))]
[JsonSerializable(typeof(FunctionDto))]
[JsonSerializable(typeof(PlotDto))]
[JsonSerializable(typeof(DiagnosticDto))]
internal sealed partial class RunJsonContext : JsonSerializerContext
{
}

[JsonSourceGenerationOptions(PropertyNamingPolicy = JsonKnownNamingPolicy.CamelCase, WriteIndented = true,
    DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull)]
[JsonSerializable(typeof(RunEnvelopeDto))]
internal sealed partial class RunJsonPrettyContext : JsonSerializerContext
{
}

