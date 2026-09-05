using Lovelace.Knowledge;

namespace Lovelace.Knowledge.Tests;

public class DeterminismTests
{
    [Fact]
    public void RandomSamples_SameSeed_SameScripts()
    {
        var config = DefaultConfig.Create();
        var a = Proposal.RandomSamples(config.Seed, config, 50).Select(s => s.Script).ToList();
        var b = Proposal.RandomSamples(config.Seed, config, 50).Select(s => s.Script).ToList();
        Assert.Equal(a, b);
    }

    [Fact]
    public void RandomSamples_DifferentSeed_DifferentScripts()
    {
        var config = DefaultConfig.Create();
        var a = Proposal.RandomSamples(1, config, 50).Select(s => s.Script).ToList();
        var b = Proposal.RandomSamples(2, config, 50).Select(s => s.Script).ToList();
        Assert.NotEqual(a, b);
    }

    [Fact]
    public void Proposal_BuildsSweeps_Deterministically()
    {
        var config = DefaultConfig.Create();
        var p1 = new Proposal(config.Seed, config);
        var p2 = new Proposal(config.Seed, config);
        Assert.Equal(p1.SweepCount, p2.SweepCount);
        Assert.True(p1.SweepCount > 0);
    }

    [Fact]
    public void GraphStore_RoundTrip_PreservesGraph()
    {
        var config = DefaultConfig.Create();
        var graph = GraphStore.New(config);
        graph.Samples.Add(new SampleRecord(0, "5 - 6", Operation.Subtract,
            Operand.Natural("5"), Operand.Natural("6"), "sub:5:h2", "right", "6",
            "Integer", true, "Integer", "-1 (Integer)", null, SamplingKind.Sweep, 1.0));

        var path = Path.Combine(Path.GetTempPath(), "knowledge-roundtrip-" + Guid.NewGuid().ToString("N") + ".json");
        try
        {
            GraphStore.Save(graph, path);
            var loaded = GraphStore.Load(path);
            Assert.Equal(graph.Seed, loaded.Seed);
            Assert.Equal(graph.Samples.Count, loaded.Samples.Count);
            Assert.Equal(graph.Samples[0].Script, loaded.Samples[0].Script);
            Assert.Equal(graph.Samples[0].Sigma, loaded.Samples[0].Sigma);
        }
        finally
        {
            File.Delete(path);
        }
    }
}
