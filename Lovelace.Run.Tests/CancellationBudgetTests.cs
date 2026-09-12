using System.Diagnostics;
using System.Text.Json.Nodes;

namespace Lovelace.Run.Tests;

/// <summary>
/// The <c>--cancel-after</c> budget, end to end through the runner's own entry point.
///
/// Before this contract existed the budget was honoured only between top-level statements, so a
/// single numeric statement ran to completion and reported <c>ok</c>: an independent audit measured
/// <c>sum(1..10000000)</c> under a 1 ms budget returning <c>ok</c> after 4.9-6.0 s (8/8 runs) and
/// <c>prod(1..200000)</c> under a 100 ms budget finishing the full 200000! after 47-58 s, with
/// nothing in the envelope saying the deadline had been dropped. Each test below therefore measures
/// the wall time as well as the envelope, because "stopped" without a promptness bound is not the
/// contract: the budget must MEAN something inside the kernel.
/// </summary>
/// <para>
/// <c>Category=Timing</c>: each case measures WALL TIME against a fence (see
/// <c>PromptnessFenceMs</c>) around a spawned runner process, so CI's instrumented loop excludes the
/// category and runs it uninstrumented in its own step. The fence is 2.5 s against work whose
/// un-budgeted cost is seconds: on a loaded runner with a coverage collector attached the process
/// start alone can cross it, which is what turned run #37 red. Excluded, never skipped.
/// </para>
[Trait("Category", "Timing")]
public class CancellationBudgetTests
{
    /// <summary>
    /// The fence for a run whose un-budgeted work is measured in seconds: the fix stops inside the
    /// kernel (tens of milliseconds), the pre-fix tree runs the kernel to completion (3.4 s for this
    /// sum, 7.5 s for the 400x400 product). The fence sits far above the fix and far below the
    /// unfixed work, so neither a slow machine nor a fast one can flip the result.
    /// </summary>
    private const int PromptnessFenceMs = 2500;

    private static JsonNode Envelope(string stdout, string what) =>
        TestSupport.ParseExactlyOneJsonDocument(stdout, what);

    [Fact]
    public async Task Run_GivenBudgetFarBelowTheSum_StopsInsideTheKernelAndSaysSo()
    {
        var stopwatch = Stopwatch.StartNew();
        var (exitCode, stdout, stderr) = await TestSupport.RunScriptAsync(
            "sum(1..10000000)", "--json", "--omit-functions", "--omit-variables", "--cancel-after", "100");
        stopwatch.Stop();

        JsonNode envelope = Envelope(stdout, "cancelled sum envelope");

        // the budget stopped the run: not a success, and not an ordinary failure either
        Assert.Equal(1, exitCode);
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"stdout: {stdout} stderr: {stderr}");
        Assert.Equal("Cancelled", envelope["code"]!.GetValue<string>());
        Assert.Equal("BudgetExceeded", envelope["category"]!.GetValue<string>());

        // the ledger: the deadline stopped it, and it says how far past the budget the stop landed
        JsonNode ledger = envelope["cancellation"]!;
        Assert.Equal(100, ledger["budgetMs"]!.GetValue<int>());
        Assert.True(ledger["stopped"]!.GetValue<bool>(), $"the deadline did not stop the run: {ledger.ToJsonString()}");
        Assert.True(ledger["exceeded"]!.GetValue<bool>(), $"the excess was not reported: {ledger.ToJsonString()}");
        Assert.True(ledger["excessMs"]!.GetValue<double>() > 0, $"excessMs: {ledger.ToJsonString()}");
        // The ledger's elapsed and the envelope's elapsed are two INDEPENDENT wall clocks around the
        // same run - the ledger's is taken where the kernel stopped, the envelope's where the
        // statement finished - so they differ by the skew between those two instants. Comparing them
        // to one decimal place (0.1 ms) demanded that two clocks agree to 0.1 ms and turned this case
        // red on the runner while the product was correct; the assertion below keeps what the line is
        // for (the ledger describes THIS run: a fabricated, stale or wrongly-scaled duration differs
        // by whole milliseconds to seconds, not by clock skew) while allowing a millisecond of skew
        // and one percent of the total.
        double ledgerMs = ledger["elapsedMs"]!.GetValue<double>();
        double envelopeMs = envelope["elapsedTime"]!["value"]!.GetValue<double>() *
            (envelope["elapsedTime"]!["unit"]!.GetValue<string>() == "s" ? 1000 : 1);
        Assert.True(Math.Abs(ledgerMs - envelopeMs) <= Math.Max(1.0, envelopeMs * 0.01),
            $"the ledger's elapsedMs ({ledgerMs}) and the envelope's elapsedTime ({envelopeMs} ms) " +
            "are not the same measurement");

        // 3.4 s of work under a 100 ms budget must come back promptly, not merely "eventually"
        Assert.True(stopwatch.ElapsedMilliseconds < PromptnessFenceMs,
            $"the 100 ms budget took {stopwatch.ElapsedMilliseconds} ms to take effect on a sum that " +
            $"needs ~3400 ms unbudgeted (fence {PromptnessFenceMs} ms). Envelope: {stdout}");
    }

    [Fact]
    public async Task Run_GivenBudgetFarBelowTheMatrixProduct_StopsInsideTheKernelAndSaysSo()
    {
        var stopwatch = Stopwatch.StartNew();
        var (exitCode, stdout, stderr) = await TestSupport.RunScriptAsync(
            "matmul(eye(400), eye(400))", "--json", "--omit-functions", "--omit-variables",
            "--print-budget", "20", "--cancel-after", "100");
        stopwatch.Stop();

        JsonNode envelope = Envelope(stdout, "cancelled matmul envelope");
        Assert.Equal(1, exitCode);
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"stdout: {stdout} stderr: {stderr}");
        Assert.Equal("Cancelled", envelope["code"]!.GetValue<string>());
        Assert.True(envelope["cancellation"]!["stopped"]!.GetValue<bool>(), stdout);

        Assert.True(stopwatch.ElapsedMilliseconds < PromptnessFenceMs,
            $"the 100 ms budget took {stopwatch.ElapsedMilliseconds} ms to take effect on matmul(eye(400), " +
            $"eye(400)) which needs ~7500 ms unbudgeted (fence {PromptnessFenceMs} ms). Envelope: {stdout}");
    }

    [Fact]
    public async Task Run_GivenWorkInsideTheBudget_SucceedsNormallyAndReportsNoExcess()
    {
        // the negative control: the budget changes nothing when the work fits, and the ledger says
        // exactly that instead of leaving a consumer to infer it
        var stopwatch = Stopwatch.StartNew();
        var (exitCode, stdout, stderr) = await TestSupport.RunScriptAsync(
            "sum(1..1000)", "--json", "--omit-functions", "--omit-variables", "--cancel-after", "60000");
        stopwatch.Stop();

        JsonNode envelope = Envelope(stdout, "in-budget sum envelope");
        Assert.Equal(0, exitCode);
        Assert.True(envelope["ok"]!.GetValue<bool>(), $"stdout: {stdout} stderr: {stderr}");
        Assert.Equal("500500", envelope["result"]!["display"]!.GetValue<string>());

        JsonNode ledger = envelope["cancellation"]!;
        Assert.Equal(60000, ledger["budgetMs"]!.GetValue<int>());
        Assert.False(ledger["stopped"]!.GetValue<bool>(), ledger.ToJsonString());
        Assert.False(ledger["exceeded"]!.GetValue<bool>(), ledger.ToJsonString());
        Assert.Equal(0, ledger["excessMs"]!.GetValue<double>());
        Assert.True(stopwatch.ElapsedMilliseconds < 10_000, $"a trivial sum took {stopwatch.ElapsedMilliseconds} ms");
    }

    [Fact]
    public async Task Run_GivenNoBudget_CarriesNoCancellationBlockAtAll()
    {
        // the un-budgeted envelope is byte-for-byte what it always was: the block appears only when
        // there is a deadline to report on (a placeholder would be a lie about a budget nobody set)
        var (exitCode, stdout, stderr) = await TestSupport.RunScriptAsync(
            "sum(1..1000)", "--json", "--omit-functions", "--omit-variables");

        JsonNode envelope = Envelope(stdout, "un-budgeted sum envelope");
        Assert.Equal(0, exitCode);
        Assert.True(envelope["ok"]!.GetValue<bool>(), $"stdout: {stdout} stderr: {stderr}");
        Assert.Null(envelope["cancellation"]);
        Assert.Null(envelope["diagnostics"]);
    }

    [Fact]
    public async Task Run_GivenExceededDeadline_CarriesAMachineReadableDiagnostic()
    {
        // a failure that ran past its budget must carry the accounting in the diagnostics array too,
        // so a consumer reading only diagnostics still learns the deadline was blown
        var (exitCode, stdout, stderr) = await TestSupport.RunScriptAsync(
            "sum(1..10000000)", "--json", "--omit-functions", "--omit-variables", "--cancel-after", "100");

        JsonNode envelope = Envelope(stdout, "cancelled sum envelope");
        Assert.Equal(1, exitCode);
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"stdout: {stdout} stderr: {stderr}");

        // the ledger and the diagnostics cannot disagree: an exceeded deadline ALWAYS leaves the
        // machine-readable diagnostic, and a deadline that was not exceeded leaves the array empty
        // (the engine clears its own diagnostics on a cancellation — this entry is the runner's)
        JsonArray diagnostics = envelope["diagnostics"]!.AsArray();
        bool exceeded = envelope["cancellation"]!["exceeded"]!.GetValue<bool>();
        if (exceeded)
        {
            Assert.Contains(diagnostics, d =>
                d!["message"]!.GetValue<string>().Contains("cancellation deadline exceeded", StringComparison.Ordinal));
        }
        else
        {
            Assert.Empty(diagnostics);
        }
    }
}
