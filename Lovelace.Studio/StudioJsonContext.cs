using System.Text.Json.Serialization;

namespace Lovelace.Studio;

/// <summary>
/// Source-generated JSON metadata for the Studio HTTP DTOs.
///
/// Native AOT trims the reflection-based serializer, so the minimal APIs must
/// resolve their types from this context instead. Every DTO the API reads or
/// writes is listed below; nested property types are generated transitively.
/// </summary>
[JsonSourceGenerationOptions(PropertyNamingPolicy = JsonKnownNamingPolicy.CamelCase)]
[JsonSerializable(typeof(EvaluateRequest))]
[JsonSerializable(typeof(SetPrecisionRequest))]
[JsonSerializable(typeof(SessionResponse))]
[JsonSerializable(typeof(StartRunResponse))]
[JsonSerializable(typeof(RunStatusResponse))]
[JsonSerializable(typeof(CompletionItem))]
[JsonSerializable(typeof(CompletionResponse))]
[JsonSerializable(typeof(EvaluateResponse))]
[JsonSerializable(typeof(StateResponse))]
[JsonSerializable(typeof(ValueResult))]
[JsonSerializable(typeof(VariableRow))]
[JsonSerializable(typeof(FunctionRow))]
[JsonSerializable(typeof(PlotPayload))]
[JsonSerializable(typeof(DiagnosticRow))]
[JsonSerializable(typeof(TimingRow))]
public partial class StudioJsonContext : JsonSerializerContext
{
}
