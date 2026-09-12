using Lovelace.Suite;
using Nat = global::Lovelace.Natural.Natural;

namespace Lovelace.Suite.Tests;

/// <summary>
/// F4-C (docs/goal-cycle-6/round-13/audit-C-hostile.md:175-206): <c>len()</c> refused EVERY array with a
/// zero dimension — <c>len([[],[]])</c>, <c>len(zeros(2,0))</c>, <c>len(zeros(0,3))</c> — with
/// <c>InvalidArgument/TypeMismatch</c> "Array dimensions must be positive, but got 0. (Parameter 'shape')",
/// while the SAME value prints as <c>[[], []]</c> and reports <c>shape([[],[ ]]) = [2, 0]</c>. The message is the
/// positive-dimension rule of the generic kernel type (<c>Lovelace.Array/NdArray.cs:33-34</c>), reached
/// through <c>Value.AsArray()</c> (<c>Lovelace.Suite/Value.cs:209</c>) — not the language's shape contract.
///
/// The contract this pins, from the product's own documents:
///   * <c>Lovelace.Suite/CoreBuiltinMetadata.cs:78</c> — "Length of a vector (first dimension of an array)."
///   * <c>Lovelace.Suite/docs/Language.md:456-463</c> — <c>len([5, 6, 7, 8])</c> is <c>4</c>.
///   * <c>docs/architecture/typed-array-migration-plan.md:35</c> — decision D5, "Zero-length dimensions:
///     **Support** (<c>zeros(0)</c>, reshape-to-0, broadcasting edges)."
///   * <c>Lovelace.Suite.Tests/EmptyReductionTests.cs:41-57</c> — already pins D5: <c>zeros(0)</c> is an empty
///     vector and <c>reshape(zeros(0), 2, 0)</c> is a <c>[2, 0]</c> array.
///
/// So the first dimension of <c>[[],[]]</c> is 2 and of <c>zeros(0,3)</c> is 0: the array is admitted, and
/// <c>len</c> answers its first dimension.
/// </summary>
public class LenZeroDimensionTests
{
    private static async Task<Value> Eval(string source) => await new SuiteEngine().EvaluateAsync(source);

    [Theory]
    [InlineData("len([[],[]])", "2")]
    [InlineData("len([[]])", "1")]
    [InlineData("len(zeros(2,0))", "2")]
    [InlineData("len(zeros(0,3))", "0")]
    [InlineData("len(zeros(0,0))", "0")]
    [InlineData("len(zeros(1,0))", "1")]
    [InlineData("len(zeros(2,3))", "2")]
    [InlineData("len(zeros(2,3,4))", "2")]
    [InlineData("len([1,2,3])", "3")]
    [InlineData("len([])", "0")]
    [InlineData("len(1..0)", "0")]
    public async Task Len_IsTheFirstDimensionOfTheValue(string source, string expected)
    {
        Value result = await Eval(source);

        Assert.Equal(ValueKind.Natural, result.Kind);
        Assert.Equal(Nat.Parse(expected, null), result.AsNatural());
    }

    /// <summary>
    /// Requirement 3 of the round brief: the values <c>len</c> now answers must keep printing, shaping and
    /// indexing exactly as they did before the fix (the zero-dimension support of D5 is not narrowed).
    /// </summary>
    [Fact]
    public async Task ZeroSizedArrays_StillPrintShapeAndIndexAsBefore()
    {
        Assert.Equal("[[], []] (Array)", ValueFormatter.FormatTyped(await Eval("[[],[]]")));
        Assert.Equal("[[], []] (Array)", ValueFormatter.FormatTyped(await Eval("zeros(2,0)")));
        Assert.Equal("[2, 0] (Vector)", ValueFormatter.FormatTyped(await Eval("shape([[],[]])")));
        Assert.Equal("[] (Vector)", ValueFormatter.FormatTyped(await Eval("zeros(2,0)[0]")));
        Assert.Equal("0 (Natural)", ValueFormatter.FormatTyped(await Eval("numel(zeros(2,0))")));
        Assert.Equal("2 (Natural)", ValueFormatter.FormatTyped(await Eval("rank(zeros(2,0))")));
    }
}
