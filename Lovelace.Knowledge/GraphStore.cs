using System.Text.Json;

namespace Lovelace.Knowledge;

/// <summary>Graph persistence: the durable product of a convergence run.</summary>
public static class GraphStore
{
    public const int Version = 1;

    public static Graph New(KnowledgeConfig config) =>
        new(config.Seed, Version, config, new List<SampleRecord>(), new List<Plane>(),
            new List<BoundaryEdge>(), new List<Frontier>(), null);

    public static void Save(Graph graph, string path)
    {
        string json = JsonSerializer.Serialize(graph, KnowledgeJsonContext.Default.Graph);
        File.WriteAllText(path, json);
    }

    public static Graph Load(string path)
    {
        string json = File.ReadAllText(path);
        var graph = JsonSerializer.Deserialize(json, KnowledgeJsonContext.Default.Graph);
        return graph ?? throw new InvalidDataException("Graph file deserialized to null.");
    }
}
