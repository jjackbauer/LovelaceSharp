using System.Text.Json.Serialization;

namespace Lovelace.Knowledge;

// ---------------------------------------------------------------------------
// Enumerations (serialized as strings via JsonStringEnumConverter)
// ---------------------------------------------------------------------------

[JsonConverter(typeof(JsonStringEnumConverter<NumberDomain>))]
public enum NumberDomain { Natural, Integer, Real }

[JsonConverter(typeof(JsonStringEnumConverter<Operation>))]
public enum Operation
{
    Add, Subtract, Multiply, Divide, Modulo, Power,
    Equal, NotEqual, Greater, Less, GreaterEqual, LessEqual,
}

[JsonConverter(typeof(JsonStringEnumConverter<SamplingKind>))]
public enum SamplingKind { Random, Sweep, Refine, Validate }

/// <summary>Confidence levels from white-paper §5.1, ordered weakest → strongest.</summary>
[JsonConverter(typeof(JsonStringEnumConverter<Confidence>))]
public enum Confidence { Hypothesized, Observed, Repeated, Bounded, Conformant, Proven }

// ---------------------------------------------------------------------------
// Operand / sample
// ---------------------------------------------------------------------------

/// <summary>An operand: its numeric domain and its Lovelace literal text.</summary>
public sealed record Operand(NumberDomain Domain, string Literal)
{
    public static Operand Natural(string literal) => new(NumberDomain.Natural, literal);
    public static Operand Integer(string literal) => new(NumberDomain.Integer, literal);
    public static Operand Real(string literal) => new(NumberDomain.Real, literal);
}

/// <summary>
/// One executed sample: the coordinate (script + operands), its canonical
/// observation σ, and its provenance. All numeric fields are Lovelace literals.
/// </summary>
public sealed record SampleRecord(
    int Index,
    string Script,
    Operation Op,
    Operand Left,
    Operand Right,
    string? SweepId,
    string? SweptSide,
    string? AxisPos,
    string Sigma,
    bool Success,
    string? Kind,
    string? Typed,
    string? ErrorMessage,
    SamplingKind SamplingKind,
    double Weight);

// ---------------------------------------------------------------------------
// Graph model
// ---------------------------------------------------------------------------

/// <summary>A behavior plane: one canonical observation class and its support.</summary>
public sealed record Plane(
    string Sigma,
    string? Kind,
    string? ErrorMessage,
    int Support,
    Confidence Confidence,
    List<int> SampleIndices);

/// <summary>
/// A fitted guard on a swept operand variable. <c>Kind</c> is one of:
/// <c>"threshold"</c> (uniform low/high regions), <c>"equality"</c> (e.g. divisor == 0),
/// or <c>"composite"</c> (no simple uniform predicate is supported by the data).
/// </summary>
public sealed record Guard(string Variable, string Relop, string Threshold, string Expression, string Kind);

/// <summary>A single bounding sample on one side of a boundary.</summary>
public sealed record BoundEvidence(string Side, int SampleIndex, string AxisPos, string Sigma);

/// <summary>A boundary between two adjacent behavior planes.</summary>
public sealed record BoundaryEdge(
    string Id,
    string FromPlane,
    string ToPlane,
    string Operation,
    string SweptSide,
    Guard Guard,
    Confidence Confidence,
    List<BoundEvidence> Evidence);

/// <summary>An explicitly unresolved region (white-paper §5 open-world rule).</summary>
public sealed record Frontier(
    string Id,
    string Kind,
    string Description,
    string? Operation,
    string? SweptSide,
    string? Anchor,
    string? Low,
    string? High);

/// <summary>Per (operation, domain-pair) sample coverage for C4.</summary>
public sealed record CoverageCell(
    string Key,
    string Operation,
    string LeftDomain,
    string RightDomain,
    int Samples,
    double Weight);

/// <summary>Convergence metrics C1–C4 plus the stopping result.</summary>
public sealed record Metrics(
    bool Converged,
    string? StopReason,
    int TotalSamples,
    int PlaneCount,
    // C1 — plane saturation
    int C1NewPlanesLastK,
    double C1NewPlaneRate,
    bool C1Saturated,
    // C2 — boundary stability
    int C2TotalBoundaries,
    int C2StableBoundaries,
    bool C2Stable,
    // C3 — prediction agreement on held-out near-boundary probes
    int C3HeldOutCount,
    int C3AgreedCount,
    double C3Agreement,
    bool C3Agreed,
    // C4 — coverage
    bool C4Covered,
    List<CoverageCell> Coverage);

/// <summary>
/// The full domain/proposal/threshold configuration. This is the ONLY agent input
/// (P3): sampling domain, proposal distribution, and convergence thresholds.
/// </summary>
public sealed record KnowledgeConfig(
    long Seed,
    int BatchSize,
    int MaxSamples,
    double C1NewPlaneRateThreshold,
    int C1WindowBatches,
    int C2MinConfirmations,
    double C3AgreementThreshold,
    int C4MinSupportPerPlane,
    int MinRandomSamples,
    List<string> NaturalValues,
    List<string> IntegerValues,
    List<string> RealValues,
    List<Operation> Operations,
    List<Operation> SweepOperations);

/// <summary>The persisted graph: samples (provenance) plus the reduced structure.</summary>
public sealed record Graph(
    long Seed,
    int Version,
    KnowledgeConfig Config,
    List<SampleRecord> Samples,
    List<Plane> Planes,
    List<BoundaryEdge> Boundaries,
    List<Frontier> Frontiers,
    Metrics? Metrics);

/// <summary>Frontier kind tags (white-paper §5 open-world rule).</summary>
public static class FrontierKinds
{
    public const string UnresolvedBoundary = "unresolved-boundary";
    public const string LowSupport = "low-support";
    public const string WeakDimension = "weak-dimension";
    public const string UnsampledInterval = "unsampled-interval";
}

public static class OperationNames
{
    public static string ToSymbol(Operation op) => op switch
    {
        Operation.Add => "+",
        Operation.Subtract => "-",
        Operation.Multiply => "*",
        Operation.Divide => "/",
        Operation.Modulo => "%",
        Operation.Power => "^",
        Operation.Equal => "==",
        Operation.NotEqual => "!=",
        Operation.Greater => ">",
        Operation.Less => "<",
        Operation.GreaterEqual => ">=",
        Operation.LessEqual => "<=",
        _ => op.ToString(),
    };
}
