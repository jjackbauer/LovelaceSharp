using System.Diagnostics;
using System.Text;
using System.Text.Json.Nodes;

namespace Lovelace.Run.Tests;

/// <summary>
/// F3-C through the REAL process (docs/goal-cycle-6/round-13/audit-C-hostile.md:136-172): a plot WRITE the
/// process cannot perform crossed as <c>InternalError/InternalInvariantFailure</c>, <c>recoverable:false</c>,
/// with the raw CLR text — while the same class of failure on the read side (<c>--file</c>) is the recoverable
/// <c>FileReadError</c>. A caller-supplied output path that cannot be written is the plot-file twin of the
/// plot DIRECTORY refusal the runner already types (Lovelace.Run/Runner.cs:181-192): same treatment, code
/// <c>PlotFileError</c>, category <c>TypeMismatch</c>, <c>recoverable:true</c>, exit 1.
///
/// The empty-string row travels through ProcessStartInfo.ArgumentList because PowerShell 5.1 silently DROPS
/// an empty argument to a native command (audit C, docs/goal-cycle-6/round-13/audit-C-hostile.md:51-52).
/// </summary>
public class PlotFileErrorTests
{
    /// <summary>Generous ceiling: process start dominates, the script is trivial.</summary>
    private static readonly TimeSpan ProcessTimeout = TimeSpan.FromMinutes(2);

    /// <summary>The one taxonomy, mirrored from <c>Lovelace.Abstractions.ErrorCategory</c>.</summary>
    private static readonly string[] ErrorCategories =
    {
        "ParseError", "DomainError", "UnsupportedOperation", "BudgetExceeded", "NoSolution",
        "TypeMismatch", "InternalInvariantFailure",
    };

    [RequiresRunnerProcessFact]
    public async Task PlotFileUnderAMissingDirectory_IsAnErrorEnvelopeWithExitOne()
    {
        await AssertPlotFileRefusedAsync(Path.Combine("nope", "x.svg"));
    }

    /// <summary>The empty name resolves to the plot directory itself, which is not a writable file: the same
    /// caller-level failure, and it must cross the same way.</summary>
    [RequiresRunnerProcessFact]
    public async Task EmptyPlotFile_IsAnErrorEnvelopeWithExitOne()
    {
        await AssertPlotFileRefusedAsync(string.Empty);
    }

    /// <summary>The success path is not allowed to regress: a plot file that CAN be written still is, and the
    /// envelope names it.</summary>
    [RequiresRunnerProcessFact]
    public async Task WritablePlotFile_IsStillWritten()
    {
        string root = Path.Combine(Path.GetTempPath(), "lovelace-plotfile-ok-" + Guid.NewGuid().ToString("N"));
        string plotDirectory = Path.Combine(root, "plots");
        try
        {
            RunnerProcessResult result = await RunRunnerAsync(
                "--eval", "plot([1, 2, 3])", "--json", "--omit-functions", "--omit-variables",
                "--plot-dir", plotDirectory, "--plot-file", "ok.svg");

            Assert.True(result.ExitCode == 0,
                $"a writable --plot-file must still succeed; exit {result.ExitCode}, stdout {result.StdoutBytes} B, " +
                $"stderr: {Trim(result.Stderr)}");

            JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(result.Stdout, "stdout for a writable plot file");
            Assert.True(envelope["ok"]!.GetValue<bool>(), envelope.ToJsonString());

            string expectedPlot = Path.GetFullPath(Path.Combine(plotDirectory, "ok.svg"));
            Assert.True(File.Exists(expectedPlot), $"the plot file must be written to {expectedPlot}");
            Assert.Equal(expectedPlot, envelope["plot"]!["path"]!.GetValue<string>());
        }
        finally
        {
            if (Directory.Exists(root))
                Directory.Delete(root, recursive: true);
        }
    }

    private static async Task AssertPlotFileRefusedAsync(string plotFileName)
    {
        string plotDirectory = Path.Combine(Path.GetTempPath(), "lovelace-plotfile-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(plotDirectory);
        try
        {
            RunnerProcessResult result = await RunRunnerAsync(
                "--eval", "plot([1, 2, 3])", "--json", "--omit-functions", "--omit-variables",
                "--plot-dir", plotDirectory, "--plot-file", plotFileName);

            string context =
                $"--plot-file '{plotFileName}': exit {result.ExitCode}, stdout {result.StdoutBytes} B, " +
                $"stderr {result.StderrBytes} B; stderr: {Trim(result.Stderr)}";

            // the exact signature of F3-C: an internal invariant failure for a caller-supplied path
            Assert.True(result.StdoutBytes > 0, $"stdout must never be empty. {context}");
            Assert.DoesNotContain("Unhandled exception", result.Stderr);
            Assert.True(result.ExitCode == 1,
                $"a plot path the process cannot write is a script/diagnostic error: exit 1 per " +
                $"docs/symbolics/dsh-protocol.md:209. {context}");

            JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(result.Stdout,
                $"stdout for --plot-file '{plotFileName}'");

            Assert.Equal(1, envelope["protocolVersion"]!.GetValue<int>());
            Assert.False(envelope["ok"]!.GetValue<bool>(),
                $"a refused plot file is not a success: {envelope.ToJsonString()}");
            Assert.Equal("PlotFileError", envelope["code"]!.GetValue<string>());

            string category = envelope["category"]!.GetValue<string>();
            Assert.Contains(category, ErrorCategories);
            Assert.Equal("TypeMismatch", category);
            Assert.True(envelope["recoverable"]!.GetValue<bool>(),
                $"the caller can retry with a writable --plot-file, so the failure is recoverable: " +
                envelope.ToJsonString());

            // the refusal names the path the caller asked for (the file under the plot directory, or the
            // directory itself for the empty name) — never the raw CLR sentence alone
            string message = envelope["message"]!.GetValue<string>();
            Assert.Contains(Path.Combine(plotDirectory, plotFileName), message);

            Assert.NotNull(envelope["diagnostics"]);
            Assert.NotNull(envelope["elapsed"]);
            Assert.NotNull(envelope["elapsedTime"]);
            Assert.NotNull(envelope["elapsedTime"]!["value"]);
            Assert.NotNull(envelope["elapsedTime"]!["unit"]);
            Assert.NotNull(envelope["timings"]);
        }
        finally
        {
            Directory.Delete(plotDirectory, recursive: true);
        }
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
                       "the real-process plot-file check cannot run.";
            }
        }
    }
}
