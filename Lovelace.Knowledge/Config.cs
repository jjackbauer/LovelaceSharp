namespace Lovelace.Knowledge;

/// <summary>
/// The default configuration for the first convergence run, including the §15
/// resolutions (Ω, σ granularity, thresholds, budget). See Lovelace.Knowledge/README.md.
/// </summary>
public static class DefaultConfig
{
    /// <summary>
    /// Default seed. Deterministic: config + seed reproduces the same graph (P5).
    /// </summary>
    public const long Seed = 20240617;

    public const int BatchSize = 64;
    public const int MaxSamples = 700;
    public const double C1NewPlaneRateThreshold = 0.01;
    public const int C1WindowBatches = 3;
    public const int C2MinConfirmations = 2;
    public const double C3AgreementThreshold = 1.0;
    public const int C4MinSupportPerPlane = 2;
    public const int MinRandomSamples = 100;

    public static KnowledgeConfig Create(long? seed = null)
    {
        var naturals = new List<string>();
        for (int i = 0; i <= 12; i++) naturals.Add(i.ToString());

        var integers = new List<string>();
        for (int i = -6; i <= 6; i++) integers.Add(i.ToString());

        var reals = new List<string>
        {
            "-1.5", "-0.5", "0.25", "0.5", "0.(3)", "0.1(6)", "1.5", "2.5",
        };

        var operations = new List<Operation>
        {
            Operation.Add, Operation.Subtract, Operation.Multiply, Operation.Divide,
            Operation.Modulo, Operation.Power,
            Operation.Equal, Operation.NotEqual, Operation.Greater, Operation.Less,
            Operation.GreaterEqual, Operation.LessEqual,
        };

        var sweeps = new List<Operation>
        {
            Operation.Subtract, Operation.Divide, Operation.Modulo,
            Operation.Greater, Operation.Less,
        };

        return new KnowledgeConfig(
            seed ?? Seed,
            BatchSize,
            MaxSamples,
            C1NewPlaneRateThreshold,
            C1WindowBatches,
            C2MinConfirmations,
            C3AgreementThreshold,
            C4MinSupportPerPlane,
            MinRandomSamples,
            naturals,
            integers,
            reals,
            operations,
            sweeps);
    }
}
