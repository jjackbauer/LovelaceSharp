using Lovelace.Suite;

namespace Lovelace.Suite.Tests;

/// <summary>Pins the Stage-4 slice syntax <c>a[start:stop:step]</c> (STO-003).</summary>
public class SliceTests
{
    private static async Task<string> Typed(string source) =>
        ValueFormatter.FormatTyped(await new SuiteEngine().EvaluateAsync(source));

    [Fact]
    public async Task VectorSlice_StartStop() =>
        Assert.Equal("[1, 2, 3] (Vector)", await Typed("[0, 1, 2, 3, 4][1:4]"));

    [Fact]
    public async Task VectorSlice_WithStep() =>
        Assert.Equal("[0, 2, 4] (Vector)", await Typed("[0, 1, 2, 3, 4][::2]"));

    [Fact]
    public async Task MatrixSlice_Column() =>
        Assert.Equal("[2, 5] (Vector)", await Typed("[[1, 2, 3], [4, 5, 6]][:, 1]"));

    [Fact]
    public async Task MatrixSlice_Rows() =>
        Assert.Equal("[[4, 5, 6]] (Array)", await Typed("[[1, 2, 3], [4, 5, 6]][1:2]"));

    [Fact]
    public async Task SliceView_FeedsElementwise() =>
        Assert.Equal("[2, 6, 10] (Vector)", await Typed("[1, 2, 3, 4, 5][::2] * 2"));
}
