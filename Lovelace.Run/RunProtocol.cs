using System.Text.Json.Serialization;
using System.Text.Json.Serialization.Metadata;
using Lovelace.Suite;

namespace Lovelace.Run;

// ---------------------------------------------------------------------------
// Structured value projection lives in Lovelace.Suite.StructuredProjection: ONE
// implementation shared by this runner and Studio, so an agent and the IDE see
// identical structure.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// JSON envelope DTOs. These replace the anonymous types used previously, which
// the reflection-based serializer cannot serialize under Native AOT.
// ---------------------------------------------------------------------------
/// <summary>One variable in scope. <c>structured</c> is the SAME projection the result carries
/// (<see cref="StructuredProjection"/>), so a variable's meaning is recoverable from structure
/// instead of from its human Display string (A2-F24): an agent reads
/// <c>variables[0].structured.canonical</c>, never <c>variables[0].display</c>. It is total — every
/// value kind a script can leave in scope projects to a structured form.</summary>
internal sealed record VariableDto(string Name, string Kind, string Display, StructuredValueDto Structured);

/// <summary>A duration as a unit-scaled value plus its unit, so an agent never parses a suffix.</summary>
internal sealed record DurationDto(double Value, string Unit);

/// <summary>One timed top-level statement: its zero-based source position, the elapsed time as a
/// structured duration, the kind of the value it produced, and whether it wrote print output.</summary>
internal sealed record TimingDto(int Position, DurationDto Elapsed, string ResultKind, bool HasOutput);
/// <summary>One entry of the builtin registry, carrying the arity metadata the call-site
/// validator computes the arity contract from (A2-F15): <c>parameters</c> is the declared
/// signature, <c>parameterCount</c> its length, <c>minArity</c> the declared LOWER bound (resolved —
/// a definition that declares no shorter form publishes its parameter count, never the internal
/// "exactly the declared count" sentinel), and <c>variadic</c> says the last parameter may repeat,
/// which removes the upper bound. An accepted call therefore satisfies
/// <c>minArity &lt;= n &amp;&amp; (variadic || n &lt;= parameterCount)</c> with no error message parsed.</summary>
internal sealed record FunctionDto(string Name, string[] Parameters, int MinArity, int ParameterCount,
    bool Variadic, bool Builtin, string? Plugin);
internal sealed record PlotDto(string Path, string Title, string Svg);
internal sealed record ResultDto(string Kind, string Display, string Typed, StructuredValueDto Structured);
internal sealed record DiagnosticDto(string Message, int Position, int Line, int Column);

/// <summary>The cancellation-budget ledger, published whenever <c>--cancel-after</c> was given (and
/// ABSENT — never a placeholder — when it was not, so an un-budgeted envelope is byte-for-byte what
/// it always was). Two independent facts, both machine-readable:
/// <list type="bullet">
/// <item><c>stopped</c> — the deadline actually stopped this evaluation: the envelope is a failure
/// with <c>code</c> <c>Cancelled</c> and <c>category</c> <c>BudgetExceeded</c>, carrying the partial
/// state. False means the evaluation ended on its own (it completed, or it failed for an unrelated
/// reason) and the deadline never got to stop it.</item>
/// <item><c>exceeded</c> — the evaluation consumed more than its budget, with <c>excessMs</c> saying
/// by how much. This is the fact an agent must never have to infer: <c>stopped=false</c> with
/// <c>exceeded=true</c> is a DROPPED deadline — before the kernels polled, <c>sum(1..10000000)</c>
/// under a 1 ms budget reported <c>ok</c> after 3368 ms with nothing in the envelope saying so.
/// <c>stopped=true</c> with <c>exceeded=true</c> is a deadline that was observed LATE; the excess is
/// the stop latency (kernel poll stride plus, on a small budget, the OS timer granularity).</item>
/// </list>
/// <c>elapsedMs</c> is the SAME wall time the envelope publishes as <c>elapsedTime</c>, in
/// milliseconds, so budget and elapsed are directly comparable: <c>excessMs = max(0, elapsedMs -
/// budgetMs)</c>. A run that overruns still reports the outcome it actually reached (<c>ok</c> plus
/// the result, or the error it hit); the ledger says the deadline did not hold.</summary>
internal sealed record CancellationDto(
    int BudgetMs, double ElapsedMs, bool Stopped, bool Exceeded, double ExcessMs);

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
    TimingDto[] Timings,
    // present only when --cancel-after was given: the deadline verdict (see CancellationDto)
    CancellationDto? Cancellation = null,
    // present only when the deadline was overrun: one diagnostic naming the overrun, so a consumer
    // that reads only the diagnostics array still learns that the budget was dropped
    DiagnosticDto[]? Diagnostics = null);
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
    // the SAME structural duration pair a success envelope carries: the human string next to the
    // unit-scaled {value, unit}, produced by the same unit selector, so no consumer parses a suffix
    string Elapsed,
    DurationDto ElapsedTime,
    // one entry per top-level statement that ran (empty when none did), exactly as on success
    TimingDto[] Timings,
    // present only for a cancelled evaluation: everything the engine had already committed
    string[]? PartialOutput = null,
    VariableDto[]? PartialVariables = null,
    // present only when --cancel-after was given: the deadline verdict (see CancellationDto). The
    // diagnosis travels with the FAILED envelope too, because a run that errored after blowing its
    // budget must not look like a run that respected it.
    CancellationDto? Cancellation = null);

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
[JsonSerializable(typeof(CancellationDto))]
internal sealed partial class RunJsonContext : JsonSerializerContext
{
}

[JsonSourceGenerationOptions(PropertyNamingPolicy = JsonKnownNamingPolicy.CamelCase, WriteIndented = true,
    DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull)]
[JsonSerializable(typeof(RunEnvelopeDto))]
internal sealed partial class RunJsonPrettyContext : JsonSerializerContext
{
}
