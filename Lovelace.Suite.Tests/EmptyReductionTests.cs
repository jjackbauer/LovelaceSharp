using Lovelace.Suite;
using Nat = global::Lovelace.Natural.Natural;

namespace Lovelace.Suite.Tests;

/// <summary>Pins the Stage-3 empty-reduction semantics (D6).</summary>
public class EmptyReductionTests
{
    private static async Task<Value> Eval(string source) => await new SuiteEngine().EvaluateAsync(source);

    [Fact]
    public async Task Sum_Empty_ReturnsZero()
    {
        var result = await Eval("sum([])");

        Assert.Equal(ValueKind.Natural, result.Kind);
        Assert.Equal(Nat.Parse("0", null), result.AsNatural());
    }

    [Fact]
    public async Task Prod_Empty_ReturnsOne()
    {
        var result = await Eval("prod([])");

        Assert.Equal(ValueKind.Natural, result.Kind);
        Assert.Equal(Nat.Parse("1", null), result.AsNatural());
    }

    [Fact]
    public async Task Min_Empty_Throws()
    {
        await Assert.ThrowsAsync<InvalidOperationException>(() => Eval("min([])"));
    }

    [Fact]
    public async Task Max_Empty_Throws()
    {
        await Assert.ThrowsAsync<InvalidOperationException>(() => Eval("max([])"));
    }

    [Fact]
    public async Task Zeros_ZeroDimension_ProducesEmptyVector()
    {
        var result = await Eval("zeros(0)");

        Assert.Equal(ValueKind.Vector, result.Kind);
        Assert.Equal(0, result.AsVector().Count);
    }

    [Fact]
    public async Task ReshapeEmpty_ToZeroDimensions_Works()
    {
        var result = await Eval("reshape(zeros(0), 2, 0)");

        Assert.Equal(ValueKind.Array, result.Kind);
        Assert.Equal(new long[] { 2, 0 }, result.AsArrayValue().Shape.ToArray());
    }

    [Fact]
    public async Task Shape_OfEmpty_Works()
    {
        Assert.Equal("[0] (Vector)", ValueFormatter.FormatTyped(await Eval("shape(zeros(0))")));
    }
}
