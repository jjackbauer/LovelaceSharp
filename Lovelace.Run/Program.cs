using System.Text.Json;
using System.Text.Json.Serialization;
using System.Text.Json.Serialization.Metadata;
using Lovelace.Suite;

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
            WriteJson(new FileReadErrorDto(false, $"Cannot read script file '{file}': {ex.Message}"), RunJsonContext.Default.FileReadErrorDto);
            return 1;
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
    if (plotDir is not null) engine.PlotOutputDirectory = plotDir;
    if (plotFile is not null) engine.PlotFileName = plotFile;

    // plot() writes into PlotOutputDirectory without creating it, so ensure a
    // fresh --plot-dir works (a no-op when the directory already exists).
    Directory.CreateDirectory(engine.PlotOutputDirectory);

    try
    {
        var result = await engine.EvaluateAsync(ScriptSource.ToSemicolonStatements(source));

        var snapshot = engine.CaptureState();
        var variables = snapshot.Variables.Values
            .OrderBy(v => v.Name, StringComparer.Ordinal)
            .Select(v => new VariableDto(v.Name, v.Kind.ToString(), v.Display))
            .ToArray();
        var functions = snapshot.Functions.Values
            .OrderBy(f => f.Name, StringComparer.Ordinal)
            .Select(f => new FunctionDto(f.Name, f.Parameters.ToArray(), f.IsBuiltin))
            .ToArray();

        PlotDto? plot = null;
        if (engine.LastPlot is { } capture)
        {
            string path = Path.GetFullPath(Path.Combine(engine.PlotOutputDirectory, engine.PlotFileName));
            plot = new PlotDto(path, capture.Title ?? string.Empty, capture.Svg ?? string.Empty);
        }

        ResultDto? resultPayload = result.Kind == ValueKind.Void
            ? null
            : new ResultDto(result.Kind.ToString(), ValueFormatter.Format(result), ValueFormatter.FormatTyped(result));

        var envelope = new RunEnvelopeDto(
            true,
            snapshot.Revision,
            resultPayload,
            variables,
            functions,
            plot,
            engine.LastElapsedDisplay);

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

        if (json)
            WriteJson(new RunErrorDto(false, ex.Message, diagnostics, engine.LastElapsedDisplay), RunJsonContext.Default.RunErrorDto);
        else
            Console.Error.WriteLine($"Error: {ex.Message}");

        return 1;
    }
}

static void PrintText(RunEnvelopeDto envelope) =>
    Console.WriteLine(JsonSerializer.Serialize(envelope, RunJsonPrettyContext.Default.RunEnvelopeDto));

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
        "  --json               emit JSON (default)\n" +
        "  --text               emit a human-readable form\n" +
        "  --help, -h           show this help");
}

// ---------------------------------------------------------------------------
// JSON envelope DTOs. These replace the anonymous types used previously, which
// the reflection-based serializer cannot serialize under Native AOT.
// ---------------------------------------------------------------------------
internal sealed record VariableDto(string Name, string Kind, string Display);
internal sealed record FunctionDto(string Name, string[] Parameters, bool Builtin);
internal sealed record PlotDto(string Path, string Title, string Svg);
internal sealed record ResultDto(string Kind, string Display, string Typed);
internal sealed record DiagnosticDto(string Message, int Position, int Line, int Column);
internal sealed record RunEnvelopeDto(
    bool Ok,
    long Revision,
    ResultDto? Result,
    VariableDto[] Variables,
    FunctionDto[] Functions,
    PlotDto? Plot,
    string Elapsed);
internal sealed record RunErrorDto(bool Ok, string Message, DiagnosticDto[] Diagnostics, string Elapsed);
internal sealed record FileReadErrorDto(bool Ok, string Message);

[JsonSourceGenerationOptions(PropertyNamingPolicy = JsonKnownNamingPolicy.CamelCase)]
[JsonSerializable(typeof(RunEnvelopeDto))]
[JsonSerializable(typeof(RunErrorDto))]
[JsonSerializable(typeof(FileReadErrorDto))]
[JsonSerializable(typeof(ResultDto))]
[JsonSerializable(typeof(VariableDto))]
[JsonSerializable(typeof(FunctionDto))]
[JsonSerializable(typeof(PlotDto))]
[JsonSerializable(typeof(DiagnosticDto))]
internal sealed partial class RunJsonContext : JsonSerializerContext
{
}

[JsonSourceGenerationOptions(PropertyNamingPolicy = JsonKnownNamingPolicy.CamelCase, WriteIndented = true)]
[JsonSerializable(typeof(RunEnvelopeDto))]
internal sealed partial class RunJsonPrettyContext : JsonSerializerContext
{
}
