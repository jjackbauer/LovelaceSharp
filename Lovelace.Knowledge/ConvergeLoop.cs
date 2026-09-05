namespace Lovelace.Knowledge;

/// <summary>
/// The autonomous converge loop (§9): sample → execute → canonicalize → reduce →
/// merge → measure → (bias toward frontiers) → repeat, until C1–C4 thresholds are
/// met or the sample budget is exhausted. Deterministic for a fixed config + seed.
/// </summary>
public static class ConvergeLoop
{
    public static async Task<Graph> RunAsync(
        KnowledgeConfig config,
        IScriptRunner runner,
        Graph? existing,
        CancellationToken ct = default)
    {
        var graph = existing ?? GraphStore.New(config);
        var proposal = new Proposal(config.Seed, config);
        var existingSigmas = new HashSet<string>(graph.Samples.Select(s => s.Sigma), StringComparer.Ordinal);
        var newPlanesPerBatch = new List<int>();
        var batchSizes = new List<int>();
        int nextIndex = graph.Samples.Count;

        while (graph.Samples.Count < config.MaxSamples)
        {
            ct.ThrowIfCancellationRequested();
            var batch = proposal.NextBatch(graph, config);
            if (batch.Count == 0) break;

            var records = await Sampler.ExecuteAsync(runner, batch, nextIndex, ct);
            nextIndex += records.Count;

            int newPlanes = 0;
            foreach (var r in records)
                if (existingSigmas.Add(r.Sigma)) newPlanes++;

            graph.Samples.AddRange(records);
            newPlanesPerBatch.Add(newPlanes);
            batchSizes.Add(records.Count);

            graph = ReduceAndMeasure(graph, config, newPlanesPerBatch, batchSizes);
            if (graph.Metrics?.Converged == true) break;
        }

        graph = ReduceAndMeasure(graph, config, newPlanesPerBatch, batchSizes);
        return graph;
    }

    private static Graph ReduceAndMeasure(
        Graph graph, KnowledgeConfig config, List<int> newPlanesPerBatch, List<int> batchSizes)
    {
        var reduction = Reducer.Reduce(graph.Samples, config);
        var metrics = Convergence.Measure(reduction, graph.Samples, config, newPlanesPerBatch, batchSizes);
        return graph with
        {
            Planes = reduction.Planes,
            Boundaries = reduction.Boundaries,
            Frontiers = reduction.Frontiers,
            Metrics = metrics,
        };
    }
}
