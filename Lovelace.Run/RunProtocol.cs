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
    // the SAME structural duration pair a success envelope carries: the human string next to the
    // unit-scaled {value, unit}, produced by the same unit selector, so no consumer parses a suffix
    string Elapsed,
    DurationDto ElapsedTime,
    // one entry per top-level statement that ran (empty when none did), exactly as on success
    TimingDto[] Timings,
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
