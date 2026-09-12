using System.Diagnostics;
using System.Text;
using System.Text.Json.Nodes;
using Xunit;

namespace Lovelace.Run.Tests;

/// <summary>
/// The recursion budget on the WIRE, through a REAL runner process. The in-process envelope tests
/// cannot observe the pre-fix behaviour at all: a stack overflow is not an exception the host can
/// catch, so the pre-fix run of a deep script terminates the TEST HOST and no assertion ever runs.
/// Spawning the runner makes the defect observable as data instead — exit code 0xC00000FD (as
/// -1073741571) with ZERO bytes on stdout — which is what the assertions below turn into a
/// well-formed envelope naming DepthExceeded / BudgetExceeded.
/// </summary>
public class RecursionDepthEnvelopeTests
{
    /// <summary>The interpreter's evaluation budget (Lovelace.Abstractions.InputDepth.MaxEvaluationDepth).</summary>
    private const int MaxEvaluationDepth = 512;

    /// <summary>The wire code naming a depth refusal.</summary>
    private const string DepthCode = "DepthExceeded";

    /// <summary>Its category: an exhausted budget, not a domain error and not an invariant failure.</summary>
    private const string DepthCategory = "BudgetExceeded";

    /// <summary>Generous ceiling: process start dominates, the script is short.</summary>
    private static readonly TimeSpan ProcessTimeout = TimeSpan.FromMinutes(2);

    /// <summary>The audit's shape at <paramref name="depth"/>.</summary>
    private static string Recursion(int depth) =>
        "func f(n) { if (n == 0) { return 0 }; return f(n - 1) }\nf(" + depth + ")";

    /// <summary>
    /// Runs the runner as a real process over a script file, exactly as a consumer starts it, and
    /// returns the exit code with both streams as raw BYTE counts and decoded text.
    /// </summary>
    private static async Task<(int ExitCode, string Stdout, string Stderr, int StdoutBytes)> RunInARealProcessAsync(
        string script, string tag)
    {
        string runnerAssembly = Path.Combine(AppContext.BaseDirectory, "Lovelace.Run.dll");
        Assert.True(File.Exists(runnerAssembly), $"missing runner assembly: {runnerAssembly}");
        string scriptPath = Path.Combine(Path.GetTempPath(), $"ob11-{tag}-{Guid.NewGuid():N}.ls");
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

    // ------------------------------------------------------------------
    // The finding, on the wire
    // ------------------------------------------------------------------

    /// <summary>
    /// N=448 — the depth the bound was filed at. Pre-fix: exit -1073741571 (0xC00000FD) and ZERO
    /// bytes on stdout, so there is nothing for an agent to read; post-fix it must be one
    /// well-formed envelope with code DepthExceeded.
    /// </summary>
    [Fact]
    public async Task ADeepRecursion_ExitsNonZeroWithAWellFormedEnvelope()
    {
        var (exitCode, stdout, stderr, stdoutBytes) = await RunInARealProcessAsync(Recursion(448), "deep");

        Assert.True(stdoutBytes > 0,
            $"the runner must put an envelope on stdout; it wrote {stdoutBytes} bytes and exited {exitCode}. stderr: {Describe(stderr)}");
        JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(stdout, "runner stdout");
        Assert.Equal(1, exitCode);
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"ok must be false. Envelope: {stdout}");
        Assert.Equal(DepthCode, envelope["code"]!.GetValue<string>());
        Assert.Equal(DepthCategory, envelope["category"]!.GetValue<string>());
        Assert.True(envelope["recoverable"]!.GetValue<bool>(),
            "the caller can rewrite the function iteratively, so the refusal is recoverable");
        Assert.Equal(1, envelope["protocolVersion"]!.GetValue<int>());
        Assert.Equal("", stderr);
        // the message names the budget it broke and the depth it measured
        Assert.Contains($"exceeds the maximum supported evaluation depth of {MaxEvaluationDepth}",
            envelope["message"]!.GetValue<string>());
        Assert.Matches(@"evaluation depth \d+ ", envelope["message"]!.GetValue<string>());
    }

    /// <summary>
    /// A recursion the budget admits still answers, and one just past the deepest admitted level is
    /// refused: the same boundary as the in-process test, observed on the wire. Pre-fix both answer,
    /// so this fails on the exit code rather than on a process death.
    /// </summary>
    [Fact]
    public async Task TheBoundary_AnswersBelowTheBudget_AndRefusesAboveIt()
    {
        var (acceptedExit, acceptedStdout, _, acceptedBytes) = await RunInARealProcessAsync(Recursion(64), "accepted");
        Assert.True(acceptedBytes > 0, $"the accepted depth must produce an envelope, got {acceptedBytes} bytes");
        JsonNode accepted = TestSupport.ParseExactlyOneJsonDocument(acceptedStdout, "runner stdout");
        Assert.Equal(0, acceptedExit);
        Assert.True(accepted["ok"]!.GetValue<bool>(), $"f(64) must still run. Envelope: {acceptedStdout}");
        Assert.Null(accepted["code"]);

        var (refusedExit, refusedStdout, _, refusedBytes) = await RunInARealProcessAsync(Recursion(200), "refused");
        Assert.True(refusedBytes > 0, $"the refused depth must produce an envelope, got {refusedBytes} bytes");
        JsonNode refused = TestSupport.ParseExactlyOneJsonDocument(refusedStdout, "runner stdout");
        Assert.Equal(1, refusedExit);
        Assert.False(refused["ok"]!.GetValue<bool>(), $"f(200) must be refused. Envelope: {refusedStdout}");
        Assert.Equal(DepthCode, refused["code"]!.GetValue<string>());
        Assert.Equal(DepthCategory, refused["category"]!.GetValue<string>());
        Assert.True(refused["recoverable"]!.GetValue<bool>());
    }

    /// <summary>
    /// Nested <c>if</c>s — <paramref name="levels"/> of them — with the recursive call in the
    /// innermost block. Only 5 call levels, so a call-level cap admits them; every level walks the
    /// whole 240-level statement nest again, so the pre-fix process dies here (measured) with
    /// 0xC00000FD and 0 bytes on stdout.
    /// </summary>
    private static string RecursionInsideNestedIfs(int depth, int levels) =>
        "func f(n) { if (n == 0) { return 0 }; "
        + string.Concat(Enumerable.Repeat("if (1 == 1) { ", levels))
        + "return f(n - 1)"
        + string.Concat(Enumerable.Repeat(" }", levels))
        + " }\nf(" + depth + ")";

    [Fact]
    public async Task NestedBlocksInsideARecursiveBody_AreRefused_InsteadOfDying()
    {
        var (exitCode, stdout, stderr, stdoutBytes) = await RunInARealProcessAsync(
            RecursionInsideNestedIfs(5, 120), "nestedifs");

        Assert.True(stdoutBytes > 0,
            $"the runner must put an envelope on stdout; it wrote {stdoutBytes} bytes and exited {exitCode}. stderr: {Describe(stderr)}");
        JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(stdout, "runner stdout");
        Assert.Equal(1, exitCode);
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"ok must be false. Envelope: {stdout}");
        Assert.Equal(DepthCode, envelope["code"]!.GetValue<string>());
        Assert.Equal(DepthCategory, envelope["category"]!.GetValue<string>());
        Assert.True(envelope["recoverable"]!.GetValue<bool>());
        Assert.Equal("", stderr);
    }

    // ------------------------------------------------------------------
    // The property: increasing depth answers or refuses, and never dies
    // ------------------------------------------------------------------

    public static IEnumerable<object[]> IncreasingDepths()
    {
        foreach (int depth in new[] { 0, 1, 8, 64, 85, 86, 200, 432, 433, 448, 2000, 100000 })
            yield return new object[] { depth };
    }

    /// <summary>
    /// For increasing depth the runner must produce EITHER the correct answer OR a typed refusal,
    /// and the boundary must be the published one: every depth within the budget is answered, every
    /// depth past it is refused. A process death is not representable as an assertion failure, which
    /// is why the runner is spawned for real: pre-fix, every depth from 433 up exits 0xC00000FD with
    /// zero bytes on stdout and fails here as data.
    /// </summary>
    [Theory]
    [MemberData(nameof(IncreasingDepths))]
    public async Task IncreasingDepths_AnswerOrRefuse_NeverDie(int depth)
    {
        var (exitCode, stdout, stderr, stdoutBytes) = await RunInARealProcessAsync(Recursion(depth), $"sweep{depth}");

        Assert.True(stdoutBytes > 0,
            $"depth {depth}: the runner must put an envelope on stdout, but exited {exitCode} with {stdoutBytes} bytes. "
            + $"stderr: {Describe(stderr)}");
        JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(stdout, "runner stdout");

        if (depth <= 85)
        {
            Assert.True(envelope["ok"]!.GetValue<bool>(),
                $"depth {depth} is within the budget and must be answered. exit={exitCode} stdout={Describe(stdout)}");
            Assert.Equal(0, exitCode);
        }
        else
        {
            Assert.False(envelope["ok"]!.GetValue<bool>(),
                $"depth {depth} is past the budget and must be refused. stdout={Describe(stdout)}");
            Assert.Equal(1, exitCode);
            Assert.Equal(DepthCode, envelope["code"]!.GetValue<string>());
            Assert.Equal(DepthCategory, envelope["category"]!.GetValue<string>());
            Assert.True(envelope["recoverable"]!.GetValue<bool>());
        }
    }

    private static string Describe(string text) =>
        text.Length <= 240 ? text : text.Substring(0, 240) + "...";
}
