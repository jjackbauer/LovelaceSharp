using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Dsp.Tests;

/// <summary>
/// Pins the arbitrary-precision budget for the whole DSP test assembly once, before any test runs,
/// and restores it afterward. The <see cref="Rl.Pi"/> cache is a process-wide <c>Lazy</c> computed
/// at <see cref="Rl.MaxComputationDecimalPlaces"/> on first access, so the fixture materializes it
/// eagerly at a reduced budget (100 computation / 50 display digits) to keep <c>Sin</c>/<c>Cos</c>
/// argument reduction fast for the entire suite. Every DSP test class joins the same collection, so
/// this setup runs once ahead of them and the mutable global statics are never raced in parallel.
/// </summary>
public sealed class DspPrecisionFixture : IDisposable
{
    private readonly long _savedMax;
    private readonly long _savedDisplay;

    public DspPrecisionFixture()
    {
        _savedMax = Rl.MaxComputationDecimalPlaces;
        _savedDisplay = Rl.DisplayDecimalPlaces;
        Rl.MaxComputationDecimalPlaces = 100;
        Rl.DisplayDecimalPlaces = 50;
        _ = Rl.Pi;   // force the lazy cache at 100 computation digits before tests run
    }

    public void Dispose()
    {
        Rl.MaxComputationDecimalPlaces = _savedMax;
        Rl.DisplayDecimalPlaces = _savedDisplay;
    }
}

/// <summary>Serializes the DSP test classes and runs <see cref="DspPrecisionFixture"/> first.</summary>
[CollectionDefinition("DSP precision")]
public sealed class DspPrecisionCollection : ICollectionFixture<DspPrecisionFixture>
{
}
