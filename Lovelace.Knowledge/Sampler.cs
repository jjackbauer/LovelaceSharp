namespace Lovelace.Knowledge;

/// <summary>
/// Executes a batch of sample specs against the real <c>Lovelace.Run</c> interface
/// and canonicalizes each envelope into a <see cref="SampleRecord"/> (σ = plane class).
/// </summary>
public static class Sampler
{
    public static async Task<List<SampleRecord>> ExecuteAsync(
        IScriptRunner runner,
        IReadOnlyList<SampleSpec> specs,
        int baseIndex,
        CancellationToken ct = default)
    {
        var records = new List<SampleRecord>(specs.Count);
        for (int i = 0; i < specs.Count; i++)
        {
            ct.ThrowIfCancellationRequested();
            var spec = specs[i];
            var output = await runner.RunAsync(spec.Script, ct);
            records.Add(ToRecord(spec, baseIndex + i, output));
        }
        return records;
    }

    private static SampleRecord ToRecord(SampleSpec spec, int index, RunnerOutput output)
    {
        Observation obs = TryCanonicalize(output);
        string sigma = CanonicalObservation.PlaneSigma(obs);
        return new SampleRecord(
            index,
            spec.Script,
            spec.Op,
            spec.Left,
            spec.Right,
            spec.SweepId,
            spec.SweptSide,
            spec.AxisPos,
            sigma,
            obs.Success,
            obs.Kind,
            obs.Typed,
            obs.ErrorMessage,
            spec.SamplingKind,
            spec.Weight);
    }

    private static Observation TryCanonicalize(RunnerOutput output)
    {
        // Lovelace.Run emits the JSON envelope on stdout even for script errors
        // (exit code 1), so parse stdout first regardless of the exit code.
        if (!string.IsNullOrWhiteSpace(output.Stdout))
        {
            try
            {
                return CanonicalObservation.FromRunnerOutput(output.Stdout);
            }
            catch (Exception)
            {
                // fall through to a synthesized error class
            }
        }

        string message = "runner failure (exit " + output.ExitCode + ")";
        if (!string.IsNullOrWhiteSpace(output.Stderr))
            message += ": " + output.Stderr.Trim();
        return new Observation(false, "err|" + message, null, null, message);
    }
}
