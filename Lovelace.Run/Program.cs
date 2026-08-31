using System.Text.Json;
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
            WriteJson(new { ok = false, message = $"Cannot read script file '{file}': {ex.Message}" });
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
            .Select(v => new { name = v.Name, kind = v.Kind.ToString(), display = v.Display })
            .ToArray();
        var functions = snapshot.Functions.Values
            .OrderBy(f => f.Name, StringComparer.Ordinal)
            .Select(f => new { name = f.Name, parameters = f.Parameters.ToArray(), builtin = f.IsBuiltin })
            .ToArray();

        object? plot = null;
        if (engine.LastPlot is { } capture)
        {
            string path = Path.GetFullPath(Path.Combine(engine.PlotOutputDirectory, engine.PlotFileName));
            plot = new { path, title = capture.Title ?? string.Empty, svg = capture.Svg ?? string.Empty };
        }

        object? resultPayload = result.Kind == ValueKind.Void
            ? null
            : new { kind = result.Kind.ToString(), display = ValueFormatter.Format(result), typed = ValueFormatter.FormatTyped(result) };

        var envelope = new
        {
            ok = true,
            revision = snapshot.Revision,
            result = resultPayload,
            variables,
            functions,
            plot,
        };

        if (json)
            WriteJson(envelope);
        else
            PrintText(envelope);

        return 0;
    }
    catch (Exception ex)
    {
        var diagnostics = engine.Diagnostics
            .Select(d => new { message = d.Message, position = d.Position, line = d.Line, column = d.Column })
            .ToArray();

        if (json)
            WriteJson(new { ok = false, message = ex.Message, diagnostics });
        else
            Console.Error.WriteLine($"Error: {ex.Message}");

        return 1;
    }
}

static void PrintText(object envelope)
{
    // Best-effort human rendering for the --text mode; not the machine path.
    Console.WriteLine(JsonSerializer.Serialize(envelope, JsonOptions(pretty: true)));
}

static void WriteJson(object value) =>
    Console.WriteLine(JsonSerializer.Serialize(value, JsonOptions(pretty: false)));

static JsonSerializerOptions JsonOptions(bool pretty) => new()
{
    PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
    WriteIndented = pretty,
};

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
