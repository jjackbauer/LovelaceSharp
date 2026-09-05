using System.Diagnostics;

namespace Lovelace.Knowledge;

/// <summary>Output of a single runner invocation.</summary>
public sealed record RunnerOutput(int ExitCode, string Stdout, string Stderr);

/// <summary>Abstraction over "execute one Lovelace script, return the JSON envelope".</summary>
public interface IScriptRunner
{
    Task<RunnerOutput> RunAsync(string script, CancellationToken ct = default);
}

/// <summary>
/// Executes a script by spawning the published Lovelace.Run binary with the
/// script on stdin (--stdin --json) and capturing the JSON envelope on stdout.
/// </summary>
public sealed class ProcessScriptRunner : IScriptRunner
{
    private readonly string _runnerPath;
    private readonly int _timeoutMs;

    public ProcessScriptRunner(string runnerPath, int timeoutMs = 30000)
    {
        _runnerPath = runnerPath;
        _timeoutMs = timeoutMs;
    }

    public async Task<RunnerOutput> RunAsync(string script, CancellationToken ct = default)
    {
        var psi = new ProcessStartInfo
        {
            FileName = _runnerPath,
            UseShellExecute = false,
            RedirectStandardInput = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            CreateNoWindow = true,
        };
        psi.ArgumentList.Add("--stdin");
        psi.ArgumentList.Add("--json");

        using var process = new Process { StartInfo = psi };
        process.Start();

        await process.StandardInput.WriteAsync(script.AsMemory(), ct);
        process.StandardInput.Close();

        var stdoutTask = process.StandardOutput.ReadToEndAsync(ct);
        var stderrTask = process.StandardError.ReadToEndAsync(ct);

        using var cts = CancellationTokenSource.CreateLinkedTokenSource(ct);
        cts.CancelAfter(_timeoutMs);
        try
        {
            await process.WaitForExitAsync(cts.Token);
        }
        catch (OperationCanceledException)
        {
            try { process.Kill(entireProcessTree: true); } catch { }
            throw new TimeoutException("Runner exceeded " + _timeoutMs + " ms for script: " + script);
        }

        string stdout = await stdoutTask;
        string stderr = await stderrTask;
        return new RunnerOutput(process.ExitCode, stdout, stderr);
    }
}

/// <summary>A no-op runner used by unit tests and the pure config/reduce paths.</summary>
public sealed class NullRunner : IScriptRunner
{
    public Task<RunnerOutput> RunAsync(string script, CancellationToken ct = default) =>
        Task.FromResult(new RunnerOutput(0, "", ""));
}
