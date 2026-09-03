using Lovelace.Suite;

namespace Lovelace.Suite.Tests;

/// <summary>Pins the Stage-4 right-aligned broadcasting semantics (BDC-001/002/003).</summary>
public class BroadcastTests
{
    private static async Task<Value> Eval(string source) => await new SuiteEngine().EvaluateAsync(source);
    private static async Task<string> Typed(string source) =>
        ValueFormatter.FormatTyped(await new SuiteEngine().EvaluateAsync(source));

    [Fact]
    public async Task RowVector_BroadcastsToMatrix()
    {
        Assert.Equal("[[2, 4], [4, 6]] (Array)", await Typed("[1, 2] + [[1, 2], [3, 4]]"));
    }

    [Fact]
    public async Task ColumnVector_BroadcastsToMatrix()
    {
        Assert.Equal("[[11, 21], [32, 42]] (Array)", await Typed("[[1], [2]] + [[10, 20], [30, 40]]"));
    }

    [Fact]
    public async Task ScalarBroadcast_StillWorks()
    {
        Assert.Equal("[10, 20, 30] (Vector)", await Typed("[1, 2, 3] * 10"));
    }

    [Fact]
    public async Task IncompatibleShapes_ThrowsBroadcastError()
    {
        await Assert.ThrowsAsync<InvalidOperationException>(() => Eval("[1, 2] + [1, 2, 3]"));
    }
}
