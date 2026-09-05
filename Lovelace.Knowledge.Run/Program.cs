using System.Text.Json;
using Lovelace.Knowledge;

return await ProgramMain(args);

// ---------------------------------------------------------------------------
// Lovelace.Knowledge.Run — the MGIR observation-driven behavioral graph CLI.
//
// JSON-over-stdio (the Lovelace.Run pattern): read a CliRequest on stdin (or
// --eval / --file), run the command, write a CliResponse on stdout.
//
//   Lovelace.Knowledge.Run --eval '{"command":"converge","graphPath":"g.json"}'
//   Lovelace.Knowledge.Run --stdin
//
// Exit codes: 0 success, 1 command/execution error, 2 usage error.
// ---------------------------------------------------------------------------
static async Task<int> ProgramMain(string[] args)
{
    string? eval = null;
    string? file = null;
    bool stdinMode = false;

    for (int i = 0; i < args.Length; i++)
    {
        switch (args[i])
        {
            case "--eval":
                if (++i >= args.Length) return Usage("--eval requires a JSON request argument.");
                eval = args[i];
                break;
            case "--file":
                if (++i >= args.Length) return Usage("--file requires a path argument.");
                file = args[i];
                break;
            case "--stdin":
                stdinMode = true;
                break;
            case "--help":
            case "-h":
                PrintUsage(Console.Out);
                return 0;
            default:
                return Usage("Unknown argument '" + args[i] + "'.");
        }
    }

    string json;
    if (eval is not null) json = eval;
    else if (file is not null)
    {
        try { json = await File.ReadAllTextAsync(file); }
        catch (Exception ex) { Write(new CliResponse(false, "", null, null, null, null, null, null, null, null, "cannot read request file: " + ex.Message)); return 1; }
    }
    else if (stdinMode) json = await Console.In.ReadToEndAsync();
    else return Usage("No request provided. Use --eval <json>, --file <path>, or --stdin.");

    CliRequest? request;
    try { request = JsonSerializer.Deserialize(json, KnowledgeJsonContext.Default.CliRequest); }
    catch (Exception ex) { Write(new CliResponse(false, "", null, null, null, null, null, null, null, null, "invalid request JSON: " + ex.Message)); return 1; }
    if (request is null) { Write(new CliResponse(false, "", null, null, null, null, null, null, null, null, "empty request")); return 1; }

    try
    {
        var response = await DispatchAsync(request);
        Write(response);
        return response.Ok ? 0 : 1;
    }
    catch (Exception ex)
    {
        Write(new CliResponse(false, request.Command, null, null, null, null, null, null, null, null, ex.Message));
        return 1;
    }
}

static async Task<CliResponse> DispatchAsync(CliRequest req) => req.Command switch
{
    "config" => ConfigCommand(req),
    "sample" => await SampleCommand(req),
    "reduce" => ReduceCommand(req),
    "converge" => await ConvergeCommand(req),
    "query" => QueryCommand(req),
    "help" => new CliResponse(true, "help", HelpText(), null, null, null, null, null, null, null, null),
    _ => new CliResponse(false, req.Command, null, null, null, null, null, null, null, null, "unknown command '" + req.Command + "'"),
};

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

static CliResponse ConfigCommand(CliRequest req)
{
    var config = ResolveConfig(req);
    var summary = "config resolved: seed=" + config.Seed + ", batch=" + config.BatchSize
        + ", maxSamples=" + config.MaxSamples + ", ops=" + config.Operations.Count
        + ", naturals=" + config.NaturalValues.Count + ", integers=" + config.IntegerValues.Count
        + ", reals=" + config.RealValues.Count;
    return new CliResponse(true, "config", summary, config, null, null, null, null, null, null, null);
}

static async Task<CliResponse> SampleCommand(CliRequest req)
{
    var config = ResolveConfig(req);
    int count = req.BatchSize ?? config.BatchSize;
    var runner = new ProcessScriptRunner(ResolveRunner(req.Runner));
    var specs = Proposal.RandomSamples(config.Seed, config, count);
    var records = await Sampler.ExecuteAsync(runner, specs, 0);
    var summary = "executed " + records.Count + " samples: " + records.Count(s => s.Success)
        + " ok, " + records.Count(s => !s.Success) + " errors, "
        + records.Select(s => s.Sigma).Distinct().Count() + " distinct plane classes";
    return new CliResponse(true, "sample", summary, config, null, null, null, null, null, records, null);
}

static CliResponse ReduceCommand(CliRequest req)
{
    var graph = LoadGraph(req);
    var reduction = Reducer.Reduce(graph.Samples, graph.Config);
    var metrics = graph.Metrics ?? Convergence.Measure(reduction, graph.Samples, graph.Config,
        new List<int>(), new List<int>());
    var reduced = graph with
    {
        Planes = reduction.Planes,
        Boundaries = reduction.Boundaries,
        Frontiers = reduction.Frontiers,
        Metrics = metrics,
    };
    return new CliResponse(true, "reduce", Query.Summary(reduced), graph.Config, null,
        reduction.Planes, reduction.Boundaries, reduction.Frontiers, metrics, null, null);
}

static async Task<CliResponse> ConvergeCommand(CliRequest req)
{
    var config = ResolveConfig(req);
    if (req.BatchSize is { } bs) config = config with { BatchSize = bs };
    if (req.MaxSamples is { } max) config = config with { MaxSamples = max };
    var path = ResolveGraphPath(req.GraphPath);
    var runner = new ProcessScriptRunner(ResolveRunner(req.Runner));

    Graph? existing = File.Exists(path) ? LoadGraph(req) : null;
    var graph = await ConvergeLoop.RunAsync(config, runner, existing);
    GraphStore.Save(graph, path);

    return new CliResponse(true, "converge", Query.Summary(graph) + "; graph=" + Path.GetFullPath(path),
        graph.Config, graph, null, null, null, graph.Metrics, null, null);
}

static CliResponse QueryCommand(CliRequest req)
{
    var graph = LoadGraph(req);
    var q = req.Query ?? "summary";
    switch (q)
    {
        case "summary":
            return new CliResponse(true, "query", Query.Summary(graph), graph.Config, null, null, null, null, graph.Metrics, null, null);
        case "planes":
            return new CliResponse(true, "query", graph.Planes.Count + " planes", graph.Config, null, graph.Planes, null, null, graph.Metrics, null, null);
        case "boundaries":
            return new CliResponse(true, "query", graph.Boundaries.Count + " boundaries", graph.Config, null, null, graph.Boundaries, null, graph.Metrics, null, null);
        case "frontiers":
            return new CliResponse(true, "query", graph.Frontiers.Count + " frontiers", graph.Config, null, null, null, graph.Frontiers, graph.Metrics, null, null);
        case "metrics":
            return new CliResponse(true, "query", Query.Summary(graph), graph.Config, null, null, null, null, graph.Metrics, null, null);
        case "graph":
            return new CliResponse(true, "query", Query.Summary(graph), graph.Config, graph, null, null, null, graph.Metrics, null, null);
        default:
            return new CliResponse(false, "query", null, null, null, null, null, null, null, null,
                "unknown query '" + q + "' (expected summary|planes|boundaries|frontiers|metrics|graph)");
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

static KnowledgeConfig ResolveConfig(CliRequest req)
{
    var config = req.Config ?? DefaultConfig.Create();
    if (req.Seed is { } seed) config = config with { Seed = seed };
    return config;
}

static Graph LoadGraph(CliRequest req)
{
    var path = ResolveGraphPath(req.GraphPath);
    if (!File.Exists(path))
        throw new FileNotFoundException("no graph at '" + path + "'; run 'converge' first");
    return GraphStore.Load(path);
}

static string ResolveGraphPath(string? path) => string.IsNullOrEmpty(path) ? "knowledge-graph.json" : path;

static string ResolveRunner(string? runner)
{
    if (!string.IsNullOrEmpty(runner)) return runner;
    var dir = Environment.CurrentDirectory;
    var exe = Path.Combine(dir, "Lovelace.Run", "bin", "Release", "net10.0", "publish", "Lovelace.Run.exe");
    if (File.Exists(exe)) return exe;
    var noext = Path.Combine(dir, "Lovelace.Run", "bin", "Release", "net10.0", "publish", "Lovelace.Run");
    if (File.Exists(noext)) return noext;
    throw new FileNotFoundException("Lovelace.Run binary not found; run 'make runner' or pass the 'runner' argument.");
}

static void Write(CliResponse response) =>
    Console.WriteLine(JsonSerializer.Serialize(response, KnowledgeJsonContext.Default.CliResponse));

static int Usage(string message)
{
    Console.Error.WriteLine("Error: " + message);
    PrintUsage(Console.Error);
    return 2;
}

static void PrintUsage(TextWriter writer)
{
    writer.WriteLine("Lovelace.Knowledge.Run — observation-driven behavioral graph discovery CLI.");
    writer.WriteLine();
    writer.WriteLine("Usage:");
    writer.WriteLine("  Lovelace.Knowledge.Run --eval <json request>");
    writer.WriteLine("  Lovelace.Knowledge.Run --file <request.json>");
    writer.WriteLine("  Lovelace.Knowledge.Run --stdin");
    writer.WriteLine();
    writer.WriteLine("Commands: config | sample | reduce | converge | query");
    writer.WriteLine();
    writer.WriteLine("Request fields: command, config, graphPath, runner, seed, batchSize, maxSamples, query");
    writer.WriteLine("  --help, -h   show this help");
}

static string HelpText() =>
    "Commands: config (resolve defaults), sample (draw+execute a random batch), " +
    "reduce (re-derive planes/boundaries/frontiers from a persisted graph), " +
    "converge (autonomous loop to C1-C4 thresholds), query (summary|planes|boundaries|frontiers|metrics|graph).";
