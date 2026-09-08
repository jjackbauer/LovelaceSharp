using Lovelace.Abstractions;
using Lovelace.Dsp;
using Lovelace.Suite;

namespace Lovelace.Suite.Tests;

/// <summary>
/// Verifies the opt-in DSP registration through the Modus plugin seam
/// (<see cref="SuiteEngine.LoadPlugin"/> + <see cref="DspPlugin"/>) and the complex accessors
/// (<c>re</c>/<c>im</c>/<c>conj</c>/<c>abs</c>). A bare engine must not expose the DSP builtins;
/// a registered engine must, and its complex results must be bridgeable back to the
/// <c>Natural → Integer → Real</c> lattice.
/// </summary>
public class DspPluginTests
{
    [Fact]
    public async Task DspPlugin_GivenUnregisteredEngine_ThrowsUnknownFunction()
    {
        var engine = new SuiteEngine();

        var ex = await Assert.ThrowsAsync<InvalidOperationException>(
            () => engine.EvaluateAsync("fft([1, 0, 0, 0])"));

        Assert.Equal("Unknown function 'fft'.", ex.Message);
    }

    [Fact]
    public async Task Fft_GivenImpulse_ReturnsAllOnes()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new DspPlugin());

        var result = await engine.EvaluateAsync("fft([1, 0, 0, 0])");

        Assert.Equal("[1, 1, 1, 1] (Vector)", ValueFormatter.FormatTyped(result));
    }

    [Fact]
    public async Task Fft_GivenImpulse_ReturnsComplexTypedVector()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new DspPlugin());

        var result = await engine.EvaluateAsync("fft([0, 1, 0, 0])");

        var array = result.AsArrayValue();
        Assert.Equal(DType.Complex, array.DType);
        Assert.Equal(ValueKind.Complex, ((Value)array.GetElement(0)).Kind);
    }

    [Fact]
    public async Task ComplexAccessors_GivenComplexValue_ReturnRealComponents()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new DspPlugin());

        // fft([0, 1, 0, 0])[1] = e^(-iπ/2) = -i
        Assert.Equal("0 (Real)", ValueFormatter.FormatTyped(await engine.EvaluateAsync("z = fft([0, 1, 0, 0])[1]; re(z)")));
        Assert.Equal("-1 (Real)", ValueFormatter.FormatTyped(await engine.EvaluateAsync("z = fft([0, 1, 0, 0])[1]; im(z)")));
        Assert.Equal("1 (Real)", ValueFormatter.FormatTyped(await engine.EvaluateAsync("z = fft([0, 1, 0, 0])[1]; abs(z)")));
        Assert.Equal("1i (Complex)", ValueFormatter.FormatTyped(await engine.EvaluateAsync("z = fft([0, 1, 0, 0])[1]; conj(z)")));
    }

    [Fact]
    public async Task Abs_GivenComplex_ReturnsMagnitude()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new DspPlugin());

        var result = await engine.EvaluateAsync("abs(fft([0, 1, 0, 0])[1])");

        Assert.Equal(ValueKind.Real, result.Kind);
        Assert.Equal("1 (Real)", ValueFormatter.FormatTyped(result));
    }

    [Fact]
    public async Task ComplexArithmetic_GivenComplex_ThrowsActionableError()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new DspPlugin());

        var ex = await Assert.ThrowsAsync<InvalidOperationException>(
            () => engine.EvaluateAsync("fft([0, 1, 0, 0])[1] + 1"));

        Assert.Contains("re()/im()/conj()/abs()", ex.Message);
    }

    [Fact]
    public async Task Abs_GivenComplexVector_ReturnsMagnitudeSpectrum()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new DspPlugin());

        var result = await engine.EvaluateAsync("abs(fft([0, 1, 0, 0]))");

        Assert.Equal("[1, 1, 1, 1] (Vector)", ValueFormatter.FormatTyped(result));
    }

    [Fact]
    public async Task ReImConj_GivenComplexVector_MapElementwise()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new DspPlugin());

        // fft([0, 1, 0, 0]) = [1, -i, -1, i]
        Assert.Equal("[1, 0, -1, 0] (Vector)", ValueFormatter.FormatTyped(await engine.EvaluateAsync("re(fft([0, 1, 0, 0]))")));
        Assert.Equal("[0, -1, 0, 1] (Vector)", ValueFormatter.FormatTyped(await engine.EvaluateAsync("im(fft([0, 1, 0, 0]))")));
        Assert.Equal("[1, 1i, -1, -1i] (Vector)", ValueFormatter.FormatTyped(await engine.EvaluateAsync("conj(fft([0, 1, 0, 0]))")));
    }

    [Fact]
    public async Task Filter_GivenSignal_MatchesConvolution()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new DspPlugin());

        // IIR a = [1, -0.5], b = [1]: y[n] = x[n] + 0.5·y[n-1]. Filtering returns len(x)
        // samples; full convolution returns len(x)+len(h)-1, so compare the prefix.
        var direct = (await engine.EvaluateAsync("filter([1, -0.5], [1], [1, 1, 1, 1])")).AsArrayValue();
        var viaConv = (await engine.EvaluateAsync("conv([1, 1, 1, 1], filter([1, -0.5], [1], 4))")).AsArrayValue();

        Assert.True(direct.Numel <= viaConv.Numel);
        for (long i = 0; i < direct.Numel; i++)
            Assert.Equal(((Value)viaConv.GetElement(i)).AsComplex(), ((Value)direct.GetElement(i)).AsComplex());
    }

    [Fact]
    public async Task Dft_GivenLength_MatchesPaddedSignal()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new DspPlugin());

        var padded = await engine.EvaluateAsync("dft([1, 2, 3], 4)");
        var viaList = await engine.EvaluateAsync("dft([1, 2, 3, 0])");

        Assert.Equal(ValueFormatter.FormatTyped(viaList), ValueFormatter.FormatTyped(padded));
    }

    [Fact]
    public async Task Filter_GivenLength_ReturnsImpulseResponse()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new DspPlugin());

        var result = await engine.EvaluateAsync("filter([1, -0.5], [1], 4)");

        Assert.Equal("[1, 0.5, 0.25, 0.125] (Vector)", ValueFormatter.FormatTyped(result));
    }

    [Fact]
    public async Task Noise_GivenDefaultSeed_IsReproducible()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new DspPlugin());

        var first = ValueFormatter.FormatTyped(await engine.EvaluateAsync("noise(1, 0, 3)"));
        var second = ValueFormatter.FormatTyped(await engine.EvaluateAsync("noise(1, 0, 3)"));
        var explicitSeed = ValueFormatter.FormatTyped(await engine.EvaluateAsync("noise(1, 0, 0, 3)"));

        Assert.Equal(first, second);
        Assert.Equal(first, explicitSeed);
    }

    [Fact]
    public async Task Impulse_GivenNegativeN_ThrowsActionableError()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new DspPlugin());

        var ex = await Assert.ThrowsAsync<InvalidOperationException>(
            () => engine.EvaluateAsync("impulse(-1)"));

        Assert.Contains("non-negative", ex.Message);
        Assert.Contains("impulse", ex.Message);
    }

    [Fact]
    public async Task Delay_GivenOutOfRangeK_ThrowsActionableError()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new DspPlugin());

        var ex = await Assert.ThrowsAsync<InvalidOperationException>(
            () => engine.EvaluateAsync("delay([1, 2, 3], 99999999999999999999)"));

        Assert.Contains("Int64 range", ex.Message);
    }

    [Fact]
    public async Task MovingAverage_GivenNonPositiveWindow_ThrowsActionableError()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new DspPlugin());

        var ex = await Assert.ThrowsAsync<InvalidOperationException>(
            () => engine.EvaluateAsync("movingavg([1, 2, 3], 0)"));

        Assert.Contains("positive window", ex.Message);
    }

    [Fact]
    public async Task Abs_GivenNonComplexArray_ThrowsActionableError()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new DspPlugin());

        var ex = await Assert.ThrowsAsync<InvalidOperationException>(
            () => engine.EvaluateAsync("abs([1, 2, 3])"));

        Assert.Contains("Complex array", ex.Message);
    }

    [Fact]
    public void DspPlugin_LoadedTwice_Throws()
    {
        var engine = new SuiteEngine();

        engine.LoadPlugin(new DspPlugin());
        var ex = Assert.Throws<InvalidOperationException>(() => engine.LoadPlugin(new DspPlugin()));

        Assert.Contains("already loaded", ex.Message);
    }

    [Fact]
    public void PluginBuiltin_GivenCoreNameCollision_Throws()
    {
        var engine = new SuiteEngine();

        var ex = Assert.Throws<InvalidOperationException>(() => engine.LoadPlugin(new ShadowingPlugin()));

        Assert.Contains("already registered", ex.Message);
    }

    [Fact]
    public async Task DspPlugin_GivenUntouchedKnob_ComputesAtFastBudget_AndPromotesWhenRaised()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new DspPlugin());

        Assert.False(engine.PrecisionExplicitlySet);

        var fast = await engine.EvaluateAsync("im(dft([1, 2, 3])[1])");
        int fastDigits = FractionalDigits(ValueFormatter.FormatTyped(fast));

        engine.SetPrecision(60);
        Assert.True(engine.PrecisionExplicitlySet);

        var promoted = await engine.EvaluateAsync("im(dft([1, 2, 3])[1])");
        int promotedDigits = FractionalDigits(ValueFormatter.FormatTyped(promoted));

        Assert.InRange(fastDigits, 15, 45);
        Assert.True(promotedDigits >= fastDigits + 15,
            $"expected silent promotion when the knob is raised (fast={fastDigits}, promoted={promotedDigits})");
    }

    private static int FractionalDigits(string typed)
    {
        int dot = typed.IndexOf('.');
        if (dot < 0)
            return 0;
        int end = typed.IndexOf(' ', dot);
        return (end < 0 ? typed.Length : end) - dot - 1;
    }

    /// <summary>A plugin that attempts to shadow a core builtin — registration must fail.</summary>
    private sealed class ShadowingPlugin : IModusPlugin
    {
        public string Name => "Shadowing";

        public void Register(IModusContext context) =>
            context.RegisterBuiltin("abs", new[] { "x" }, _ => null!);
    }
}
