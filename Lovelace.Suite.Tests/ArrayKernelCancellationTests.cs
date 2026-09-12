using System.Diagnostics;
using Lovelace.Suite;

namespace Lovelace.Suite.Tests;

/// <summary>
/// The numeric kernels must observe the caller's deadline, not just the statement boundary.
/// <see cref="CancellationObservationTests"/> covers the Natural-library algorithms (a power, a
/// factorial); this file covers the ARRAY path the audit found silent: a range reduction, a product
/// whose operands grow to six figures of digits, a matrix product, and a bare while loop.
///
/// Each test fences on wall time as well as on the exception, because the pre-fix behaviour was not
/// "no exception ever" but "the exception arrives after the whole kernel has run" — 3.4 s for the
/// 10M sum, 22.5 s for the 200000! product, 7.5 s for the 400x400 product and 42 s for the while
/// loop, each against a budget of tens or hundreds of milliseconds.
/// </summary>
public class ArrayKernelCancellationTests
{
    /// <summary>
    /// Test-side fence: a non-observing implementation FAILS rather than hangs. It is deliberately
    /// far above each budget and far below the un-budgeted cost of the work, so a slow machine
    /// cannot turn promptness into a flake and a fast one cannot hide the defect.
    /// </summary>
    private static readonly TimeSpan Fence = TimeSpan.FromSeconds(8);

    private static async Task<(TimeSpan Wall, EvaluationCancelledException Error)> ExpectPromptStop(
        string script, int budgetMs, string work)
    {
        var engine = new SuiteEngine();
        using var cts = new CancellationTokenSource(budgetMs);
        var stopwatch = Stopwatch.StartNew();

        // Task.Run is load-bearing: EvaluateAsync runs synchronously on the caller's thread until its
        // first genuine await, so without the offload the fence below could never fire.
        var evaluation = Task.Run(() => engine.EvaluateAsync(script, null, cts.Token));
        var settled = await Task.WhenAny(evaluation, Task.Delay(Fence));
        stopwatch.Stop();

        Assert.True(settled == evaluation,
            $"cancellation was not observed: the {budgetMs} ms budget expired but '{script}' was still " +
            $"running after {stopwatch.Elapsed.TotalSeconds:F1} s (fence {Fence.TotalSeconds:F0} s); " +
            $"{work}");

        var error = await Assert.ThrowsAsync<EvaluationCancelledException>(() => evaluation);
        Assert.True(error.CancellationToken.IsCancellationRequested);
        return (stopwatch.Elapsed, error);
    }

    [Fact]
    public async Task SumOverRange_GivenShortBudget_CancelsInsideTheRangeAndReduction()
    {
        // ~3.4 s unbudgeted: the range build alone is ten million boxed Naturals
        var (wall, _) = await ExpectPromptStop("sum(1..10000000)", 100,
            "sum(1..10000000) needs ~3.4 s unbudgeted");
        Assert.True(wall < Fence, $"cancellation took {wall.TotalSeconds:F2} s for a 100 ms budget");
    }

    [Fact]
    public async Task ProductOverRange_GivenShortBudget_CancelsInsideTheGrowingProduct()
    {
        // ~22.5 s unbudgeted: the accumulator reaches 200000! (973351 digits), so the late iterations
        // are milliseconds each and the poll stride is the only thing between the budget and a
        // minute-long overrun
        var (wall, _) = await ExpectPromptStop("prod(1..200000)", 200,
            "prod(1..200000) needs ~22.5 s unbudgeted");
        Assert.True(wall < Fence, $"cancellation took {wall.TotalSeconds:F2} s for a 200 ms budget");
    }

    [Fact]
    public async Task MatrixProduct_GivenShortBudget_CancelsInsideTheTripleLoop()
    {
        // ~7.5 s unbudgeted: 400^3 boxed multiply-adds
        var (wall, _) = await ExpectPromptStop("matmul(eye(400), eye(400))", 100,
            "matmul(eye(400), eye(400)) needs ~7.5 s unbudgeted");
        Assert.True(wall < Fence, $"cancellation took {wall.TotalSeconds:F2} s for a 100 ms budget");
    }

    [Fact]
    public async Task WhileLoop_GivenShortBudget_CancelsBetweenIterations()
    {
        // ~42 s unbudgeted: a body of one cheap statement, which used to loop for minutes without a
        // single look at the deadline (the statement-boundary check only runs between TOP-LEVEL
        // statements, and this loop is one statement)
        var (wall, _) = await ExpectPromptStop("i = 0; while (i < 100000000) { i = i + 1 }; i", 200,
            "a 100M-iteration while loop needs ~42 s unbudgeted");
        Assert.True(wall < Fence, $"cancellation took {wall.TotalSeconds:F2} s for a 200 ms budget");
    }

    [Fact]
    public async Task ElementwiseArrayArithmetic_GivenShortBudget_CancelsInsideTheMap()
    {
        // the elementwise kernel is a second per-element loop the audit's source sweep found
        // unpolled; zeros() fills in one Array.Fill (microseconds), so the work that runs past the
        // budget is the five-million-element map itself
        var (wall, _) = await ExpectPromptStop("a = zeros(5000000); b = a + a; sum(b)", 100,
            "elementwise addition over five million boxed Values needs ~0.6 s unbudgeted");
        Assert.True(wall < Fence, $"cancellation took {wall.TotalSeconds:F2} s for a 100 ms budget");
    }

    [Fact]
    public async Task RunInsideBudget_GivenGenerousBudget_StillReturnsTheResult()
    {
        // the negative control at the engine seam: polling must not disturb a run that fits
        var engine = new SuiteEngine();
        using var cts = new CancellationTokenSource(TimeSpan.FromSeconds(60));
        var result = await engine.EvaluateAsync("sum(1..1000)", null, cts.Token);
        Assert.Equal("500500", ValueFormatter.Format(result));
    }
}
