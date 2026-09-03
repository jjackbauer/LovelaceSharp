using Lovelace.Suite;
using Nat = global::Lovelace.Natural.Natural;

namespace Lovelace.Suite.Tests;

/// <summary>
/// Pins the Stage-3 promotion semantics of the typed-array migration: D3 (mixed-literal
/// promotion), D1 (Natural subtraction narrow-back), D2 (Natural/Integer division
/// narrow-back), and whole-array elementwise promotion.
/// </summary>
public class TypedArrayPromotionTests
{
    private static async Task<Value> Eval(string source) => await new SuiteEngine().EvaluateAsync(source);

    [Fact]
    public async Task MixedNumericLiteral_PromotesToMaxKind()
    {
        var result = await Eval("[1, 2.5, 3]");

        Assert.Equal(ValueKind.Vector, result.Kind);
        Assert.Equal(3, result.AsVector().Count);
        Assert.All(result.AsVector(), e => Assert.Equal(ValueKind.Real, e.Kind));
    }

    [Fact]
    public async Task NestedMixedLiteral_PromotesHomogeneously()
    {
        var result = await Eval("[[1, 2.5], [3, 4]]");

        Assert.Equal(ValueKind.Array, result.Kind);
        Assert.Equal(new long[] { 2, 2 }, result.AsArray().Shape);
        Assert.All(result.AsArray().Data, e => Assert.Equal(ValueKind.Real, e.Kind));
    }

    [Fact]
    public async Task NaturalSubtraction_NoUnderflow_NarrowsBackToNatural()
    {
        var result = await Eval("[5, 3] - [4, 2]");

        Assert.Equal(ValueKind.Vector, result.Kind);
        Assert.All(result.AsVector(), e => Assert.Equal(ValueKind.Natural, e.Kind));
        Assert.Equal(Nat.Parse("1", null), result.AsVector()[0].AsNatural());
    }

    [Fact]
    public async Task NaturalSubtraction_Underflow_WidensToInteger()
    {
        var result = await Eval("[5, 3] - [6, 4]");

        Assert.Equal(ValueKind.Vector, result.Kind);
        Assert.All(result.AsVector(), e => Assert.Equal(ValueKind.Integer, e.Kind));
    }

    [Fact]
    public async Task NaturalDivision_Exact_NarrowsBackToNatural()
    {
        var result = await Eval("[4, 6] / [2, 2]");

        Assert.Equal(ValueKind.Vector, result.Kind);
        Assert.All(result.AsVector(), e => Assert.Equal(ValueKind.Natural, e.Kind));
        Assert.Equal(Nat.Parse("2", null), result.AsVector()[0].AsNatural());
    }

    [Fact]
    public async Task NaturalDivision_Inexact_WidensToReal()
    {
        var result = await Eval("[1, 2] / [2, 2]");

        Assert.Equal(ValueKind.Vector, result.Kind);
        Assert.All(result.AsVector(), e => Assert.Equal(ValueKind.Real, e.Kind));
    }

    [Fact]
    public async Task ElementwiseAdd_MixedKinds_PromotesToReal()
    {
        var result = await Eval("[1, 2] + [3.5, 4.5]");

        Assert.Equal(ValueKind.Vector, result.Kind);
        Assert.All(result.AsVector(), e => Assert.Equal(ValueKind.Real, e.Kind));
    }

    [Fact]
    public async Task RealDivision_ExactIntegerQuotient_NarrowsToNatural()
    {
        var result = await Eval("[2.5] / [0.25]");

        Assert.Equal(ValueKind.Vector, result.Kind);
        Assert.All(result.AsVector(), e => Assert.Equal(ValueKind.Natural, e.Kind));
        Assert.Equal(Nat.Parse("10", null), result.AsVector()[0].AsNatural());
    }

    [Fact]
    public async Task IntegerDivision_ExactNonNegative_NarrowsToNatural()
    {
        var result = await Eval("[-4, -6] / [-2, -2]");

        Assert.Equal(ValueKind.Vector, result.Kind);
        Assert.All(result.AsVector(), e => Assert.Equal(ValueKind.Natural, e.Kind));
        Assert.Equal(Nat.Parse("2", null), result.AsVector()[0].AsNatural());
    }

    [Fact]
    public async Task RealDivision_Inexact_StaysReal()
    {
        var result = await Eval("[1.0] / [3.0]");

        Assert.Equal(ValueKind.Vector, result.Kind);
        Assert.All(result.AsVector(), e => Assert.Equal(ValueKind.Real, e.Kind));
    }
}
