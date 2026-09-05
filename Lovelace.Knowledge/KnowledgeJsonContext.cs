using System.Text.Json.Serialization;

namespace Lovelace.Knowledge;

/// <summary>
/// Source-generated JSON context for Native AOT (no reflection). Lists every
/// type serialized to the persisted graph or over the CLI's JSON API.
/// </summary>
[JsonSourceGenerationOptions(PropertyNamingPolicy = JsonKnownNamingPolicy.CamelCase, WriteIndented = true)]
[JsonSerializable(typeof(Graph))]
[JsonSerializable(typeof(KnowledgeConfig))]
[JsonSerializable(typeof(Metrics))]
[JsonSerializable(typeof(Plane))]
[JsonSerializable(typeof(BoundaryEdge))]
[JsonSerializable(typeof(Frontier))]
[JsonSerializable(typeof(SampleRecord))]
[JsonSerializable(typeof(Guard))]
[JsonSerializable(typeof(BoundEvidence))]
[JsonSerializable(typeof(CoverageCell))]
[JsonSerializable(typeof(Operand))]
[JsonSerializable(typeof(CliRequest))]
[JsonSerializable(typeof(CliResponse))]
[JsonSerializable(typeof(List<SampleRecord>))]
[JsonSerializable(typeof(List<Plane>))]
[JsonSerializable(typeof(List<BoundaryEdge>))]
[JsonSerializable(typeof(List<Frontier>))]
[JsonSerializable(typeof(List<string>))]
public sealed partial class KnowledgeJsonContext : JsonSerializerContext
{
}
