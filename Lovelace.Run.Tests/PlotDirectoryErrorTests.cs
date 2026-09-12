using System.Diagnostics;
using System.Text;
using System.Text.Json.Nodes;

namespace Lovelace.Run.Tests;

/// <summary>
/// F1-C (P0, docs/goal-cycle-6/round-13/audit-C-hostile.md:19-78): a --plot-dir the process cannot
/// turn into a directory ABORTED the runner — exit 0xC0000409 with ZERO bytes on stdout and an
/// unhandled IOException on stderr — because <c>Directory.CreateDirectory(engine.PlotOutputDirectory)</c>
/// (Runner.cs:168) sat OUTSIDE the runner's try (Runner.cs:177). The contract it broke is the
/// protocol document's own: the envelope goes on stdout and the exit codes are 0 success /
/// 1 script-or-diagnostic error / 2 usage error (docs/symbolics/dsh-protocol.md:9-10, :207-209,
/// Lovelace.Run/Program.cs:18, Lovelace.Run/Runner.cs:27).
///
/// These tests drive the REAL process, for the same reason StdoutPurityTests does: an in-process call
/// throws instead of returning an exit code, so only a process can show the abort, and only a process
/// can show that stdout was empty. Every assertion is on structure the protocol documents, never on
/// message prose: code, category, recoverable, protocolVersion, and the structural duration pair that
/// invariant 4 keeps on the error path too.
///
/// The empty-string row travels through ProcessStartInfo.ArgumentList because PowerShell 5.1 silently
/// DROPS an empty argument to a native command (audit C, docs/goal-cycle-6/round-13/audit-C-hostile.md:51-52);
/// ArgumentList passes it as the two-quote empty argument the process API preserves.
/// </summary>
public class PlotDirectoryErrorTests
{
    /// <summary>Generous ceiling: process start dominates, the script is trivial.</summary>
    private static readonly TimeSpan ProcessTimeout = TimeSpan.FromMinutes(2);

    /// <summary>The one taxonomy, mirrored from <c>Lovelace.Abstractions.ErrorCategory</c>
    /// (Lovelace.Abstractions/Diagnostic.cs:12-21), so a category is proven to be a member of the
    /// documented set and not merely a non-empty string.</summary>
    private static readonly string[] ErrorCategories =
    {
        "ParseError", "DomainError", "UnsupportedOperation", "BudgetExceeded", "NoSolution",
        "TypeMismatch", "InternalInvariantFailure",
    };

    /// <summary>An existing regular FILE where a directory is required: the row audit C reproduced
    /// with <c>C:\Windows\notepad.exe</c>, made hermetic here with a file the test owns.</summary>
    [RequiresRunnerProcessFact]
    public async Task PlotDirectoryNamingAnExistingFile_IsAnErrorEnvelopeWithExitOne()
    {
        string existingFile = Path.Combine(Path.GetTempPath(),
            "lovelace-plotdir-existing-file-" + Guid.NewGuid().ToString("N") + ".txt");
        File.WriteAllText(existingFile, "this is a file, not a directory");
        try
        {
            await AssertPlotDirectoryRefusedAsync(existingFile);
        }
        finally
        {
            File.Delete(existingFile);
        }
    }

    /// <summary>The empty value: the parser accepts any non-missing value, and an empty path is a
    /// caller mistake that must still cross as an envelope with a code and a category.</summary>
    [RequiresRunnerProcessFact]
    public async Task EmptyPlotDirectory_IsAnErrorEnvelopeWithExitOne()
    {
        Assert.False(Directory.Exists(string.Empty), "the empty path must not name a directory");
        await AssertPlotDirectoryRefusedAsync(string.Empty);
    }

    /// <summary>A path Windows refuses to turn into a directory (illegal characters), the fourth of
    /// audit C's values after the file, the DLL and the README.</summary>
    [RequiresRunnerProcessFact]
    public async Task PlotDirectoryWithInvalidCharacters_IsAnErrorEnvelopeWithExitOne()
    {
        string invalid = Path.Combine(Path.GetTempPath(),
            "lovelace-plotdir-invalid-" + Guid.NewGuid().ToString("N") + "<bad>|?*");
        await AssertPlotDirectoryRefusedAsync(invalid);
    }

    /// <summary>
    /// The success path is not allowed to regress: a --plot-dir that CAN be created is still created
    /// (recursively — cycle 5 measured a missing directory being made, docs/goal-cycle-5/audit2/B3-surface.md:111)
    /// and still receives the plot file, with the envelope naming the path it wrote.
    /// </summary>
    [RequiresRunnerProcessFact]
    public async Task ValidPlotDirectory_StillReceivesThePlotFile()
    {
        string root = Path.Combine(Path.GetTempPath(), "lovelace-plotdir-ok-" + Guid.NewGuid().ToString("N"));
        string plotDirectory = Path.Combine(root, "nested", "plots");
        try
        {
            Assert.False(Directory.Exists(root), $"the probe directory must not exist before the run: {root}");

            RunnerProcessResult result = await RunRunnerAsync(
                "--eval", "plot([1, 2, 3])", "--json", "--omit-functions", "--omit-variables",
                "--plot-dir", plotDirectory);

            Assert.True(result.ExitCode == 0,
                $"a usable --plot-dir must still succeed; exit {result.ExitCode}, stdout {result.StdoutBytes} B, " +
                $"stderr: {Trim(result.Stderr)}");

            JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(result.Stdout,
                $"stdout for --plot-dir '{plotDirectory}'");
            Assert.True(envelope["ok"]!.GetValue<bool>(), envelope.ToJsonString());

            string expectedPlot = Path.GetFullPath(Path.Combine(plotDirectory, "plot.svg"));
            Assert.True(File.Exists(expectedPlot), $"the plot file must be written to {expectedPlot}");
            string svg = File.ReadAllText(expectedPlot);
            Assert.True(svg.Contains("<svg", StringComparison.OrdinalIgnoreCase),
                $"the plot file must carry SVG content; it is {svg.Length} characters");

            Assert.NotNull(envelope["plot"]);
            Assert.Equal(expectedPlot, envelope["plot"]!["path"]!.GetValue<string>());
        }
        finally
        {
            if (Directory.Exists(root))
                Directory.Delete(root, recursive: true);
        }
    }

    /// <summary>
    /// One refused directory, asserted through the process boundary: stdout is NEVER empty, the exit
    /// code stays inside the documented set, the envelope carries the stable code and a taxonomy
    /// category, and the crash signature ("Unhandled exception") is gone from stderr.
    /// </summary>
    private static async Task AssertPlotDirectoryRefusedAsync(string plotDirectory)
    {
        RunnerProcessResult result = await RunRunnerAsync(
            "--eval", "1", "--json", "--omit-functions", "--omit-variables", "--plot-dir", plotDirectory);

        string context =
            $"--plot-dir '{plotDirectory}': exit {result.ExitCode}, stdout {result.StdoutBytes} B, " +
            $"stderr {result.StderrBytes} B; stderr: {Trim(result.Stderr)}";

        // the exact signature of F1-C: the process died with ZERO bytes on stdout, so no code and no
        // category ever reached the caller
        Assert.True(result.StdoutBytes > 0, $"stdout must never be empty. {context}");
        Assert.DoesNotContain("Unhandled exception", result.Stderr);
        Assert.True(result.ExitCode == 1,
            $"a directory the caller named but the process cannot create is a script/diagnostic error: " +
            $"exit 1 per docs/symbolics/dsh-protocol.md:209. {context}");

        JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(result.Stdout,
            $"stdout for --plot-dir '{plotDirectory}'");

        Assert.Equal(1, envelope["protocolVersion"]!.GetValue<int>());
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"a refused plot directory is not a success: {envelope.ToJsonString()}");
        Assert.Equal("PlotDirectoryError", envelope["code"]!.GetValue<string>());

        string category = envelope["category"]!.GetValue<string>();
        Assert.Contains(category, ErrorCategories);
        Assert.Equal("TypeMismatch", category);
        Assert.True(envelope["recoverable"]!.GetValue<bool>(),
            $"the caller can retry with a usable --plot-dir, so the failure is recoverable: {envelope.ToJsonString()}");
        Assert.Contains(plotDirectory, envelope["message"]!.GetValue<string>());

        // invariant 4 is NOT scoped to success: the structural duration pair is present on the error
        // path too (docs/symbolics/dsh-protocol.md:202-205)
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
        // the raw streams, not the decoded readers: the stdout BYTE COUNT is the finding's own
        // measurement, so it is read off the pipe rather than inferred from a decoded string
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

    /// <summary>
    /// Reported as SKIPPED rather than failed when the runner assembly is not next to the test
    /// assembly, exactly like StdoutPurityTests' precondition (xUnit v2 has no dynamic skip).
    /// </summary>
    internal sealed class RequiresRunnerProcessFactAttribute : FactAttribute
    {
        public RequiresRunnerProcessFactAttribute()
        {
            string runnerAssembly = Path.Combine(AppContext.BaseDirectory, "Lovelace.Run.dll");
            if (!File.Exists(runnerAssembly))
            {
                Skip = $"Lovelace.Run.dll is not next to the test assembly ({runnerAssembly}); " +
                       "the real-process plot-directory check cannot run.";
            }
        }
    }
}
