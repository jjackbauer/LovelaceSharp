using System.Diagnostics;
using Lovelace.Suite;

namespace Lovelace.Suite.Tests;

/// <summary>
/// N17: a single long-running numeric statement must observe the caller's cancellation token.
/// The seam is the ambient <c>Lovelace.Abstractions.Cancellation</c> scope that
/// <see cref="SuiteEngine.EvaluateAsync(string, TextWriter?, CancellationToken)"/> installs
/// (SuiteEngine.cs:223) together with the <see cref="EvaluationCancelledException"/> it raises.
/// </summary>
/// <para>
/// <c>Category=Timing</c>: both cases below are WALL-CLOCK verdicts. CI's instrumented loop
/// excludes this category and runs it in its own uninstrumented step, because the coverage
/// collector inflates a 1.5 s budget into tens of seconds and turns promptness into a flake
/// (measured: 29 s under the collector on a loaded machine, 1-4 s without). It is excluded,
/// never skipped, and it still runs on every push.
/// </para>
[Trait("Category", "Timing")]
public class CancellationObservationTests
{
    /// <summary>Test-side fence: an implementation that never observes cancellation must FAIL
    /// this test, not hang it. The fence is deliberately far above the budget so a slow machine
    /// cannot turn promptness into a flake.</summary>
    private static readonly TimeSpan Fence = TimeSpan.FromSeconds(30);

    [Fact]
    public async Task Evaluate_GivenLongNaturalPowerAndShortBudget_CancelsPromptly()
    {
        var engine = new SuiteEngine();
        using var cts = new CancellationTokenSource(TimeSpan.FromMilliseconds(1500));

        var stopwatch = Stopwatch.StartNew();
        // Task.Run is load-bearing: EvaluateAsync runs synchronously on the caller's thread until
        // its first genuinely asynchronous await, so without the offload a non-observing
        // implementation would block the test thread and the fence below could never fire.
        var evaluation = Task.Run(() => engine.EvaluateAsync("2^1000000000", null, cts.Token));
        var settled = await Task.WhenAny(evaluation, Task.Delay(Fence));
        stopwatch.Stop();

        Assert.True(settled == evaluation,
            $"cancellation was not observed: the 1500 ms budget expired but '2^1000000000' was still " +
            $"running after {stopwatch.Elapsed.TotalSeconds:F1} s (test fence {Fence.TotalSeconds:F0} s).");

        var cancelled = await Assert.ThrowsAsync<EvaluationCancelledException>(() => evaluation);
        Assert.True(stopwatch.ElapsedMilliseconds < 20_000,
            $"cancellation was observed but only after {stopwatch.ElapsedMilliseconds} ms; the budget was 1500 ms.");
        Assert.True(cancelled.CancellationToken.IsCancellationRequested);
    }

    [Fact]
    public async Task Evaluate_GivenLongFactorialAndShortBudget_CancelsPromptly()
    {
        var engine = new SuiteEngine();
        using var cts = new CancellationTokenSource(TimeSpan.FromMilliseconds(1500));

        var stopwatch = Stopwatch.StartNew();
        var evaluation = Task.Run(() => engine.EvaluateAsync("2000000!", null, cts.Token));
        var settled = await Task.WhenAny(evaluation, Task.Delay(Fence));
        stopwatch.Stop();

        Assert.True(settled == evaluation,
            $"cancellation was not observed: the 1500 ms budget expired but 2000000! was still " +
            $"running after {stopwatch.Elapsed.TotalSeconds:F1} s (test fence {Fence.TotalSeconds:F0} s).");

        await Assert.ThrowsAsync<EvaluationCancelledException>(() => evaluation);
        Assert.True(stopwatch.ElapsedMilliseconds < 20_000,
            $"cancellation was observed but only after {stopwatch.ElapsedMilliseconds} ms; the budget was 1500 ms.");
    }
}
