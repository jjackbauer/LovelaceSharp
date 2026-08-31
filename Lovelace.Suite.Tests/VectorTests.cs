using Lovelace.Suite;
using Nat = global::Lovelace.Natural.Natural;

namespace Lovelace.Suite.Tests;

public class VectorTests
{
    [Fact]
    public async Task Evaluate_GivenListLiteral_ProducesVector()
    {
        var engine = new SuiteEngine();

        var result = await engine.EvaluateAsync("[10, 20, 30]");

        Assert.Equal(ValueKind.Vector, result.Kind);
        Assert.Equal(3, result.AsVector().Count);
        Assert.Equal(Nat.Parse("10", null), result.AsVector()[0].AsNatural());
    }

    [Fact]
    public async Task Evaluate_GivenIndex_UsesZeroBasedIndexing()
    {
        var engine = new SuiteEngine();

        var result = await engine.EvaluateAsync("[10, 20, 30][0]");

        Assert.Equal(Nat.Parse("10", null), result.AsNatural());
    }

    [Fact]
    public async Task Evaluate_GivenIndexOutOfRange_Throws()
    {
        var engine = new SuiteEngine();

        await Assert.ThrowsAsync<InvalidOperationException>(() => engine.EvaluateAsync("[10, 20, 30][3]"));
    }

    [Fact]
    public async Task Evaluate_GivenVectorAddition_AppliesElementWise()
    {
        var engine = new SuiteEngine();

        var result = await engine.EvaluateAsync("[1, 2] + [10, 20]");

        Assert.Equal(ValueKind.Vector, result.Kind);
        Assert.Equal(Nat.Parse("11", null), result.AsVector()[0].AsNatural());
        Assert.Equal(Nat.Parse("22", null), result.AsVector()[1].AsNatural());
    }

    [Fact]
    public async Task Evaluate_GivenVectorScalarMultiply_Broadcasts()
    {
        var engine = new SuiteEngine();

        var result = await engine.EvaluateAsync("[1, 2, 3] * 10");

        Assert.Equal(3, result.AsVector().Count);
        Assert.Equal(Nat.Parse("30", null), result.AsVector()[2].AsNatural());
    }

    [Fact]
    public async Task Evaluate_GivenMismatchedVectorLengths_Throws()
    {
        var engine = new SuiteEngine();

        await Assert.ThrowsAsync<InvalidOperationException>(() => engine.EvaluateAsync("[1, 2] + [1, 2, 3]"));
    }

    [Fact]
    public async Task Evaluate_GivenLen_ReturnsVectorLength()
    {
        var engine = new SuiteEngine();

        var result = await engine.EvaluateAsync("len([5, 6, 7, 8])");

        Assert.Equal(Nat.Parse("4", null), result.AsNatural());
    }

    [Fact]
    public async Task Evaluate_GivenIndexedRangeElement_ReturnsExpectedValue()
    {
        var engine = new SuiteEngine();

        var result = await engine.EvaluateAsync("(1..5)[2]");

        Assert.Equal(Nat.Parse("3", null), result.AsNatural());
    }
}
