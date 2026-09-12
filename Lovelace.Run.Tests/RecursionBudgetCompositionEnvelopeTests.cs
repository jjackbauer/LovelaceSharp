using System.Diagnostics;
using System.Text;
using System.Text.Json.Nodes;
using System.Text.RegularExpressions;
using Xunit;

namespace Lovelace.Run.Tests;

/// <summary>
/// M-2 on the wire (round-22 audit M, P1). The published binary refused <c>{ 1; f(85) }</c> with
/// <c>DepthExceeded</c> "evaluation depth 513" while <c>{ f(85); 1 }</c> answered, so the documented
/// boundary the depth guard was tuned around (EVD-262: f(85) answers, f(86) is refused) was a
/// property of the statement ORDER inside the block rather than of the call. The audit's exact
/// programs m01/m02 are run here through a real runner process, alongside the refusal's own depth.
/// </summary>
public class RecursionBudgetCompositionEnvelopeTests
{
    /// <summary>The interpreter's evaluation budget (Lovelace.Abstractions.InputDepth).</summary>
    private const int MaxEvaluationDepth = 512;

    private const string DepthCode = "DepthExceeded";
    private const string DepthCategory = "BudgetExceeded";

    /// <summary>Audit M's DEF: the recursive function on its own line.</summary>
    private const string Definition =
        "func f(n) { if (n == 0) { return 0 }; return f(n - 1) }\n";

    private static readonly TimeSpan ProcessTimeout = TimeSpan.FromMinutes(2);

    private static string Recursion(int depth) => Definition + "f(" + depth + ")";

    /// <summary>
    /// Runs the runner as a real process over a script file, exactly as a consumer starts it, and
    /// returns the exit code with both streams as raw BYTE counts and decoded text.
    /// </summary>
    private static async Task<(int ExitCode, string Stdout, string Stderr, int StdoutBytes)> RunInARealProcessAsync(
        string script, string tag)
    {
        string runnerAssembly = Path.Combine(AppContext.BaseDirectory, "Lovelace.Run.dll");
        Assert.True(File.Exists(runnerAssembly), $"missing runner assembly: {runnerAssembly}");
        string scriptPath = Path.Combine(Path.GetTempPath(), $"m2-{tag}-{Guid.NewGuid():N}.ls");
        File.WriteAllText(scriptPath, script);

        try
        {
            var startInfo = new ProcessStartInfo("dotnet")
            {
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
            };
            startInfo.ArgumentList.Add("exec");
            startInfo.ArgumentList.Add(runnerAssembly);
            startInfo.ArgumentList.Add("--file");
            startInfo.ArgumentList.Add(scriptPath);
            startInfo.ArgumentList.Add("--json");
            startInfo.ArgumentList.Add("--omit-functions");
            startInfo.ArgumentList.Add("--omit-variables");

            using var process = Process.Start(startInfo)!;
            var stdoutBytes = new MemoryStream();
            var stderrBytes = new MemoryStream();
            Task stdoutTask = process.StandardOutput.BaseStream.CopyToAsync(stdoutBytes);
            Task stderrTask = process.StandardError.BaseStream.CopyToAsync(stderrBytes);
            using var timeout = new CancellationTokenSource(ProcessTimeout);
            await process.WaitForExitAsync(timeout.Token);
            await Task.WhenAll(stdoutTask, stderrTask);

            return (process.ExitCode, Encoding.UTF8.GetString(stdoutBytes.ToArray()),
                Encoding.UTF8.GetString(stderrBytes.ToArray()), (int)stdoutBytes.Length);
        }
        finally
        {
            File.Delete(scriptPath);
        }
    }

    private static JsonNode Envelope(string stdout, int bytes)
    {
        Assert.True(bytes > 0, "the runner must put an envelope on stdout");
        return TestSupport.ParseExactlyOneJsonDocument(stdout, "runner stdout");
    }

    private static void AssertDepthRefusal(JsonNode envelope, string shape)
    {
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"{shape}: ok must be false");
        Assert.Equal(DepthCode, envelope["code"]!.GetValue<string>());
        Assert.Equal(DepthCategory, envelope["category"]!.GetValue<string>());
        Assert.True(envelope["recoverable"]!.GetValue<bool>(), $"{shape}: the refusal is recoverable");
        Assert.Contains($"exceeds the maximum supported evaluation depth of {MaxEvaluationDepth}",
            envelope["message"]!.GetValue<string>());
    }

    /// <summary>The depth the refusal measured, read off its own message.</summary>
    private static int MeasuredDepth(JsonNode envelope)
    {
        Match match = Regex.Match(envelope["message"]!.GetValue<string>(), @"evaluation depth (\d+) ");
        Assert.True(match.Success, $"the message must name the depth: {envelope["message"]}");
        return int.Parse(match.Groups[1].Value);
    }

    // ------------------------------------------------------------------
    // The finding: the audit's own two programs
    // ------------------------------------------------------------------

    /// <summary>
    /// m02 — <c>{ 1; f(85) }</c>: the program the published binary refused with
    /// <c>DepthExceeded</c>. The preceding constant statement has returned before the call runs, so
    /// it cannot spend the call's budget: this must answer 0.
    /// </summary>
    [Fact]
    public async Task APrecedingSiblingStatement_DoesNotSpendTheCallBudget()
    {
        var (exitCode, stdout, stderr, bytes) = await RunInARealProcessAsync(
            Definition + "{ 1; f(85) }", "m02");

        JsonNode envelope = Envelope(stdout, bytes);
        Assert.Equal(0, exitCode);
        Assert.True(envelope["ok"]!.GetValue<bool>(), $"m02 must answer. Envelope: {stdout}");
        Assert.Null(envelope["code"]);
        Assert.Equal("0", envelope["result"]!["display"]!.GetValue<string>());
        Assert.Equal("", stderr);
    }

    /// <summary>
    /// m01 and m02 at the documented boundary: the two programs differ only in the order of two
    /// statements at the same nesting depth, so both must answer — and the trailing-sibling one
    /// answers the trailing constant (1), which is what it returned before the fix too.
    /// </summary>
    [Fact]
    public async Task BothBlockOrders_AnswerAtTheDocumentedBoundary()
    {
        var (firstExit, firstStdout, _, firstBytes) = await RunInARealProcessAsync(
            Definition + "{ f(85); 1 }", "m01");
        JsonNode first = Envelope(firstStdout, firstBytes);
        Assert.Equal(0, firstExit);
        Assert.True(first["ok"]!.GetValue<bool>(), $"m01 must answer. Envelope: {firstStdout}");
        Assert.Equal("1", first["result"]!["display"]!.GetValue<string>());

        var (secondExit, secondStdout, _, secondBytes) = await RunInARealProcessAsync(
            Definition + "{ 1; f(85) }", "m02-again");
        JsonNode second = Envelope(secondStdout, secondBytes);
        Assert.Equal(0, secondExit);
        Assert.True(second["ok"]!.GetValue<bool>(), $"m02 must answer. Envelope: {secondStdout}");
        Assert.Equal("0", second["result"]!["display"]!.GetValue<string>());
    }

    /// <summary>
    /// The same call one level past the budget, in both block orders, must be refused with the SAME
    /// measured depth: the number the refusal publishes is a property of the call's nesting, not of
    /// the siblings around it.
    /// </summary>
    [Fact]
    public async Task TheRefusal_ReportsTheSameDepth_InBothBlockOrders()
    {
        var (topExit, topStdout, _, topBytes) = await RunInARealProcessAsync(Recursion(86), "top86");
        JsonNode top = Envelope(topStdout, topBytes);
        Assert.Equal(1, topExit);
        AssertDepthRefusal(top, "f(86) at top level");
        int topDepth = MeasuredDepth(top);
        Assert.True(topDepth > MaxEvaluationDepth, $"the measured depth must be past the budget, was {topDepth}");

        var (blockExit, blockStdout, _, blockBytes) = await RunInARealProcessAsync(
            Definition + "{ 1; f(86) }", "block86");
        JsonNode block = Envelope(blockStdout, blockBytes);
        Assert.Equal(1, blockExit);
        AssertDepthRefusal(block, "{ 1; f(86) }");

        Assert.Equal(topDepth, MeasuredDepth(block));
    }

    /// <summary>
    /// The guard keeps its teeth where the nesting is REAL: a wrapper function that is still on the
    /// stack when f runs adds a live user-function frame, so f(85) below it is past the budget and
    /// must still be refused (the measured boundary there is 84, one level per live frame).
    /// </summary>
    [Fact]
    public async Task ALiveWrapperFrame_StillRefusesTheSameCall()
    {
        var (exitCode, stdout, stderr, bytes) = await RunInARealProcessAsync(
            Definition + "func g() { return f(85) }\ng()", "wrapper");

        JsonNode envelope = Envelope(stdout, bytes);
        Assert.Equal(1, exitCode);
        AssertDepthRefusal(envelope, "f(85) under a live wrapper frame");
        Assert.Equal("", stderr);
    }
}
