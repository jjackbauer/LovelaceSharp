namespace Lovelace.Knowledge;

/// <summary>JSON-over-stdio request to the CLI.</summary>
public sealed record CliRequest(
    string Command,
    KnowledgeConfig? Config,
    string? GraphPath,
    string? Runner,
    long? Seed,
    int? BatchSize,
    int? MaxSamples,
    string? Query);

/// <summary>JSON-over-stdio response from the CLI. Unused fields are null per command.</summary>
public sealed record CliResponse(
    bool Ok,
    string Command,
    string? Summary,
    KnowledgeConfig? Config,
    Graph? Graph,
    List<Plane>? Planes,
    List<BoundaryEdge>? Boundaries,
    List<Frontier>? Frontiers,
    Metrics? Metrics,
    List<SampleRecord>? Samples,
    string? Error);
