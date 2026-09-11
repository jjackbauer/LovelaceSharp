using System.Diagnostics;
using System.Text.Json.Nodes;

namespace Lovelace.Run.Tests;

/// <summary>
/// Proves stdout purity through the REAL process: the in-process tests cannot see a stray
/// Console.Write that only a packaging or runtime change reintroduces. The runner is started
/// exactly as a consumer would start it.
/// </summary>
public class StdoutPurityTests
{
    private const string PrintedText = "hello from the script";

    /// <summary>Generous ceiling: publishing and process start dominate, the script is trivial.</summary>
    private static readonly TimeSpan ProcessTimeout = TimeSpan.FromMinutes(2);

    [RequiresRunnerProcessFact]
    public async Task RealProcessStdoutIsExactlyOneJsonDocumentAndPrintStaysInTheOutputArray()
    {
        string runnerAssembly = Path.Combine(AppContext.BaseDirectory, "Lovelace.Run.dll");
        string scriptPath = TestSupport.ScriptPath("print_purity");
        Assert.True(File.Exists(scriptPath), $"missing fixture script: {scriptPath}");

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
        startInfo.ArgumentList.Add("--omit-functions");

        using var process = Process.Start(startInfo)!;
        Task<string> stdoutTask = process.StandardOutput.ReadToEndAsync();
        Task<string> stderrTask = process.StandardError.ReadToEndAsync();
        using var timeout = new CancellationTokenSource(ProcessTimeout);
        await process.WaitForExitAsync(timeout.Token);
        string stdout = await stdoutTask;
        string stderr = await stderrTask;

        Assert.True(process.ExitCode == 0,
            $"runner exited {process.ExitCode}. stderr: {stderr.Trim()}");

        JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(stdout, "runner stdout");
        Assert.True(envelope["ok"]!.GetValue<bool>(), "the script succeeds");

        JsonArray output = envelope["output"]!.AsArray();
        Assert.Equal(new[] { PrintedText }, output.Select(n => n!.GetValue<string>()).ToArray());
        Assert.Equal("2", envelope["result"]!["structured"]!["value"]!.GetValue<string>());

        // Exactly one occurrence of the printed text in the whole stream, and the assertion above
        // places it in the output array: print() never leaks into the envelope's surroundings.
        Assert.Equal(1, CountOccurrences(stdout, PrintedText));
    }

    /// <summary>
    /// Reported as SKIPPED rather than failed when the runner assembly is not next to the test
    /// assembly. xUnit v2 has no dynamic skip (Xunit.Sdk.SkipException.ForSkip is documented as
    /// v3-only), so the precondition is evaluated when the attribute is constructed, and the
    /// run reports the reason verbatim.
    /// </summary>
    internal sealed class RequiresRunnerProcessFactAttribute : FactAttribute
    {
        public RequiresRunnerProcessFactAttribute()
        {
            string runnerAssembly = Path.Combine(AppContext.BaseDirectory, "Lovelace.Run.dll");
            if (!File.Exists(runnerAssembly))
            {
                Skip = $"Lovelace.Run.dll is not next to the test assembly ({runnerAssembly}); " +
                       "the real-process stdout purity check cannot run.";
            }
        }
    }

    private static int CountOccurrences(string haystack, string needle)
    {
        int count = 0;
        for (int index = haystack.IndexOf(needle, StringComparison.Ordinal);
             index >= 0;
             index = haystack.IndexOf(needle, index + needle.Length, StringComparison.Ordinal))
        {
            count++;
        }
        return count;
    }
}
