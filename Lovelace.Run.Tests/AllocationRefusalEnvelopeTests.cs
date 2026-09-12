using System.Diagnostics;
using System.Text;
using System.Text.Json.Nodes;

namespace Lovelace.Run.Tests;

/// <summary>
/// F2-C through the REAL process (docs/goal-cycle-6/round-13/audit-C-hostile.md:82-127): the published
/// binary answered <c>zeros(1000000000)</c> with <c>exit 1</c>,
/// <c>code:"InternalError"</c>, <c>category:"InternalInvariantFailure"</c>, <c>recoverable:false</c> and the
/// CLR's own "Insufficient memory to continue the execution of the program." on a machine with 39 GB free.
/// A request the process cannot serve is a budget stop: it must cross with a stable code, a taxonomy
/// category and <c>recoverable:true</c>, and — the point of the guard — it must be refused BEFORE the
/// multi-gigabyte allocation is attempted, so this test never performs that allocation itself (the child
/// process answers from the budget check and the envelope's message names both the request and the limit).
///
/// The process boundary is deliberate, exactly like PlotDirectoryErrorTests: only a process shows the exit
/// code, the byte count on stdout and the absence of an unhandled exception.
/// </summary>
public class AllocationRefusalEnvelopeTests
{
    /// <summary>Generous ceiling: process start dominates a refused request.</summary>
    private static readonly TimeSpan ProcessTimeout = TimeSpan.FromMinutes(2);

    /// <summary>The one taxonomy, mirrored from <c>Lovelace.Abstractions.ErrorCategory</c>, so a category is
    /// proven to be a member of the documented set and not merely a non-empty string.</summary>
    private static readonly string[] ErrorCategories =
    {
        "ParseError", "DomainError", "UnsupportedOperation", "BudgetExceeded", "NoSolution",
        "TypeMismatch", "InternalInvariantFailure",
    };

    /// <summary>The audit's own shape: 10^9 Value references (8 GiB at 8 bytes each).</summary>
    [RequiresRunnerProcessFact]
    public async Task HugeZeros_CrossesAsARecoverableBudgetRefusal()
    {
        await AssertRefusedAsync("zeros(1000000000)", "1000000000");
    }

    /// <summary>A shape whose element product (10^18) can never be allocated at all.</summary>
    [RequiresRunnerProcessFact]
    public async Task HugeZerosProduct_CrossesAsARecoverableBudgetRefusal()
    {
        await AssertRefusedAsync("zeros(1000000000, 1000000000)", "1000000000000000000");
    }

    /// <summary>The success path is not allowed to regress: an ordinary zeros() still answers.</summary>
    [RequiresRunnerProcessFact]
    public async Task OrdinaryZeros_StillSucceeds()
    {
        RunnerProcessResult result = await RunRunnerAsync(
            "--eval", "zeros(2, 3)", "--json", "--omit-functions", "--omit-variables");

        Assert.True(result.ExitCode == 0,
            $"zeros(2, 3) must still succeed; exit {result.ExitCode}, stdout {result.StdoutBytes} B, " +
            $"stderr: {Trim(result.Stderr)}");
        JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(result.Stdout, "stdout for zeros(2, 3)");
        Assert.True(envelope["ok"]!.GetValue<bool>(), envelope.ToJsonString());
        Assert.Equal("[[0, 0, 0], [0, 0, 0]] (Array)", envelope["result"]!["typed"]!.GetValue<string>());
    }

    private static async Task AssertRefusedAsync(string script, string expectedCountInMessage)
    {
        RunnerProcessResult result = await RunRunnerAsync(
            "--eval", script, "--json", "--omit-functions", "--omit-variables");

        string context =
            $"--eval '{script}': exit {result.ExitCode}, stdout {result.StdoutBytes} B, " +
            $"stderr {result.StderrBytes} B; stderr: {Trim(result.Stderr)}";

        // the exact signature of F2-C: the process answered with an internal invariant failure
        Assert.True(result.StdoutBytes > 0, $"stdout must never be empty. {context}");
        Assert.DoesNotContain("Unhandled exception", result.Stderr);
        Assert.True(result.ExitCode == 1,
            $"a refused request is a script/diagnostic error: exit 1 per docs/symbolics/dsh-protocol.md:209. {context}");

        JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(result.Stdout, $"stdout for --eval '{script}'");

        Assert.Equal(1, envelope["protocolVersion"]!.GetValue<int>());
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"a refused request is not a success: {envelope.ToJsonString()}");
        Assert.Equal("AllocationRefused", envelope["code"]!.GetValue<string>());

        string category = envelope["category"]!.GetValue<string>();
        Assert.Contains(category, ErrorCategories);
        Assert.Equal("BudgetExceeded", category);
        Assert.True(envelope["recoverable"]!.GetValue<bool>(),
            $"the caller can ask for a smaller shape, so the failure is recoverable: {envelope.ToJsonString()}");

        // the refusal comes from the budget check, not from the allocator: it names the request (and the
        // limit), which the CLR's own "Insufficient memory …" text never does
        string message = envelope["message"]!.GetValue<string>();
        Assert.Contains(expectedCountInMessage, message);
        Assert.DoesNotContain("Insufficient memory", message);

        // invariant 4 is NOT scoped to success: the structural duration pair is present on the error path too
        Assert.NotNull(envelope["diagnostics"]);
        Assert.NotNull(envelope["elapsed"]);
        Assert.NotNull(envelope["elapsedTime"]);
        Assert.NotNull(envelope["elapsedTime"]!["value"]);
        Assert.NotNull(envelope["elapsedTime"]!["unit"]);
        Assert.NotNull(envelope["timings"]);
    }

    private static async Task<RunnerProcessResult> RunRunnerAsync(params string[] arguments)
    {
        string runnerAssembly = Path.Combine(AppContext.BaseDirectory, "Lovelace.Run.dll");
        var startInfo = new ProcessStartInfo("dotnet")
        {
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
        };
        startInfo.ArgumentList.Add("exec");
        startInfo.ArgumentList.Add(runnerAssembly);
        foreach (string argument in arguments)
            startInfo.ArgumentList.Add(argument);

        using var process = Process.Start(startInfo)!;
        // the raw streams, not the decoded readers: the stdout BYTE COUNT is the finding's own measurement
        var stdoutBytes = new MemoryStream();
        var stderrBytes = new MemoryStream();
        Task stdoutTask = process.StandardOutput.BaseStream.CopyToAsync(stdoutBytes);
        Task stderrTask = process.StandardError.BaseStream.CopyToAsync(stderrBytes);
        using var timeout = new CancellationTokenSource(ProcessTimeout);
        await process.WaitForExitAsync(timeout.Token);
        await Task.WhenAll(stdoutTask, stderrTask);

        byte[] outBytes = stdoutBytes.ToArray();
        byte[] errBytes = stderrBytes.ToArray();
        return new RunnerProcessResult(process.ExitCode,
            Encoding.UTF8.GetString(outBytes), outBytes.Length,
            Encoding.UTF8.GetString(errBytes), errBytes.Length);
    }

    private static string Trim(string text)
    {
        string trimmed = text.Trim();
        return trimmed.Length <= 400 ? trimmed : trimmed.Substring(0, 400) + "...";
    }

    private sealed record RunnerProcessResult(
        int ExitCode, string Stdout, int StdoutBytes, string Stderr, int StderrBytes);

    /// <summary>Reported as SKIPPED rather than failed when the runner assembly is not next to the test
    /// assembly, exactly like StdoutPurityTests' precondition (xUnit v2 has no dynamic skip).</summary>
    internal sealed class RequiresRunnerProcessFactAttribute : FactAttribute
    {
        public RequiresRunnerProcessFactAttribute()
        {
            string runnerAssembly = Path.Combine(AppContext.BaseDirectory, "Lovelace.Run.dll");
            if (!File.Exists(runnerAssembly))
            {
                Skip = $"Lovelace.Run.dll is not next to the test assembly ({runnerAssembly}); " +
                       "the real-process allocation-refusal check cannot run.";
            }
        }
    }
}
