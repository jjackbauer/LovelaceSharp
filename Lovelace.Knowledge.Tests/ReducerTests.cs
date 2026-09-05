using Lovelace.Knowledge;

namespace Lovelace.Knowledge.Tests;

public class ReducerTests
{
    private static KnowledgeConfig Config() => DefaultConfig.Create();

    private static SampleRecord Sweep(int i, Operation op, string left, string right, string sigma,
        string? kind = null, string? error = null, string? sweepId = "s:h2")
    {
        bool success = !sigma.StartsWith("err|", StringComparison.Ordinal);
        return new SampleRecord(i, left + " " + OperationNames.ToSymbol(op) + " " + right,
            op, Operand.Natural(left), Operand.Natural(right), sweepId, "right", right,
            sigma, success, kind, null, error, SamplingKind.Sweep, 1.0);
    }

    [Fact]
    public void Detect_NaturalSubtractionUnderflow_ThresholdGuard()
    {
        var samples = new List<SampleRecord>();
        for (int b = 0; b <= 5; b++)
            samples.Add(Sweep(b, Operation.Subtract, "5", b.ToString(), "Natural", "Natural", "sub:5:h2"));
        for (int b = 6; b <= 8; b++)
            samples.Add(Sweep(b, Operation.Subtract, "5", b.ToString(), "Integer", "Integer", "sub:5:h2"));

        var reduction = Reducer.Reduce(samples, Config());

        Assert.Equal(2, reduction.Planes.Count);
        var boundary = Assert.Single(reduction.Boundaries);
        Assert.Equal("Natural", boundary.FromPlane);
        Assert.Equal("Integer", boundary.ToPlane);
        Assert.Equal("threshold", boundary.Guard.Kind);
        Assert.Equal("right > left", boundary.Guard.Expression);
        Assert.Equal("5", boundary.Guard.Threshold);
    }

    [Fact]
    public void Detect_DivisionByZero_EqualityGuard()
    {
        var samples = new List<SampleRecord>
        {
            Sweep(0, Operation.Divide, "5", "0", "err|Cannot divide by zero.", error: "Cannot divide by zero.", sweepId: "div:5:h2"),
            Sweep(1, Operation.Divide, "5", "1", "Natural", "Natural", sweepId: "div:5:h2"),
            Sweep(2, Operation.Divide, "5", "2", "Real", "Real", sweepId: "div:5:h2"),
            Sweep(3, Operation.Divide, "5", "3", "Real", "Real", sweepId: "div:5:h2"),
        };

        var reduction = Reducer.Reduce(samples, Config());

        var zeroBoundary = Assert.Single(reduction.Boundaries, b => b.Guard.Kind == "equality");
        Assert.Equal("err|Cannot divide by zero.", zeroBoundary.FromPlane);
        Assert.Equal("Natural", zeroBoundary.ToPlane);
        Assert.Equal("right == 0", zeroBoundary.Guard.Expression);
        // Natural->Real is a non-uniform (composite) boundary, reported without a simple predicate.
        Assert.Contains(reduction.Boundaries, b => b.Guard.Kind == "composite");
    }

    [Fact]
    public void Detect_UnresolvedBoundary_MarksFrontier()
    {
        // b = 0 (Natural) and b = 2 (Integer) with a gap -> needs bisection.
        var samples = new List<SampleRecord>
        {
            Sweep(0, Operation.Subtract, "1", "0", "Natural", "Natural", "sub:1:h2"),
            Sweep(1, Operation.Subtract, "1", "2", "Integer", "Integer", "sub:1:h2"),
        };

        var reduction = Reducer.Reduce(samples, Config());

        Assert.Empty(reduction.Boundaries);
        Assert.Contains(reduction.Frontiers, f => f.Kind == FrontierKinds.UnresolvedBoundary);
    }

    [Fact]
    public void Cluster_SameSigma_SharesOnePlane()
    {
        var samples = new List<SampleRecord>
        {
            Sweep(0, Operation.Add, "1", "2", "Natural", "Natural", "a:h2"),
            Sweep(1, Operation.Add, "3", "4", "Natural", "Natural", "a:h2"),
            Sweep(2, Operation.Add, "1", "2", "Integer", "Integer", "a:h2"),
        };

        var reduction = Reducer.Reduce(samples, Config());

        var natural = Assert.Single(reduction.Planes, p => p.Sigma == "Natural");
        Assert.Equal(2, natural.Support);
    }
}
