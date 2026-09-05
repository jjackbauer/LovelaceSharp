using Cplx = global::Lovelace.Complex.Complex;
using Lovelace.Dsp;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Dsp.Tests;

/// <summary>
/// Functional tests for the signal generators (<see cref="Impulse"/>, <see cref="Step"/>,
/// <see cref="Delay"/>, <see cref="Scalar"/>, <see cref="Sum"/>, <see cref="Product"/>,
/// <see cref="PowerSeries"/>), <see cref="DspMath.Convolve"/>, <see cref="DspMath.MovingAverage"/>,
/// <see cref="DspMath.ImpulseResponse"/>, and <see cref="Sequence"/>.
/// </summary>
[Collection("DSP precision")]
public class DspTests
{
    private static Cplx C(string re, string im = "0") => new(new Rl(re), new Rl(im));

    private static readonly ISignal Impulse = new Impulse();
    private static readonly ISignal Step = new Step();

    [Fact]
    public void Impulse_GivenZeroAndNonZero_IsOneOnlyAtZero()
    {
        Assert.Equal(Cplx.One, Impulse.Get(0));
        Assert.Equal(Cplx.Zero, Impulse.Get(1));
        Assert.Equal(Cplx.Zero, Impulse.Get(-1));
    }

    [Fact]
    public void Step_GivenNegativeAndNonNegative_IsOneForNonNegative()
    {
        Assert.Equal(Cplx.Zero, Step.Get(-1));
        Assert.Equal(Cplx.One, Step.Get(0));
        Assert.Equal(Cplx.One, Step.Get(5));
    }

    [Fact]
    public void Delay_GivenImpulseAndK_ShiftsSignal()
    {
        var d = new Delay(3, Impulse);
        Assert.Equal(Cplx.One, d.Get(3));
        Assert.Equal(Cplx.Zero, d.Get(2));
    }

    [Fact]
    public void Scalar_GivenStepAndK_Scales()
    {
        var s = new Scalar(C("2"), Step);
        Assert.Equal(C("2"), s.Get(0));
        Assert.Equal(C("2"), s.Get(3));
    }

    [Fact]
    public void Sum_GivenImpulseAndStep_AddsPointwise()
    {
        var s = new Sum(Impulse, Step);
        Assert.Equal(C("2"), s.Get(0));   // 1 + 1
        Assert.Equal(C("1"), s.Get(1));   // 0 + 1
    }

    [Fact]
    public void Product_GivenImpulseAndStep_MultipliesPointwise()
    {
        var p = new Product(Impulse, Step);
        Assert.Equal(C("1"), p.Get(0));   // 1 · 1
        Assert.Equal(C("0"), p.Get(1));   // 0 · 1
    }

    [Fact]
    public void PowerSeries_GivenKAndA_ComputesKnAPowN()
    {
        var p = new PowerSeries(C("2"), C("0.5"));
        Assert.Equal(C("0.75"), p.Get(3));   // 2 · 3 · 0.5³ = 6 · 0.125
        Assert.Equal(C("0"), p.Get(0));      // k · 0 · a⁰ = 0
    }

    [Fact]
    public void Convolve_GivenTwoOnes_ProducesTriangle()
    {
        var result = DspMath.Convolve(new[] { Cplx.One, Cplx.One }, new[] { Cplx.One, Cplx.One });
        Assert.Equal(new[] { Cplx.One, C("2"), Cplx.One }, result);
    }

    [Fact]
    public void Convolve_Given123SelfConvolution_ProducesSquare()
    {
        var result = DspMath.Convolve(new[] { C("1"), C("2"), C("3") }, new[] { C("1"), C("2"), C("3") });
        Assert.Equal(new[] { C("1"), C("4"), C("10"), C("12"), C("9") }, result);
    }

    [Fact]
    public void MovingAverage_GivenStep_ReturnsWindowAverage()
    {
        var m = new DspMath.MovingAverage(2, Step);
        Assert.Equal(C("0.5"), m.Get(0));   // (step(-1) + step(0)) / 2 = 0.5
        Assert.Equal(C("1"), m.Get(1));     // (step(0) + step(1)) / 2 = 1
    }

    [Fact]
    public void ImpulseResponse_GivenUnitFilter_PassesImpulse()
    {
        // a = [1], b = [1]: y[n] = x[n] (identity), so impulse response is [1, 0, 0].
        var h = DspMath.ImpulseResponse(new[] { Cplx.One }, new[] { Cplx.One }, 3);
        Assert.Equal(new[] { Cplx.One, Cplx.Zero, Cplx.Zero }, h);
    }

    [Fact]
    public void ImpulseResponse_GivenFirstOrderIir_MatchesHandComputed()
    {
        // y[n] = 0.1·x[n] + 0.9·y[n−1] ⇒ impulse response is 0.1·0.9ⁿ.
        var h = DspMath.ImpulseResponse(new[] { C("1"), C("-0.9") }, new[] { C("0.1") }, 4);
        Assert.Equal(new[] { C("0.1"), C("0.09"), C("0.081"), C("0.0729") }, h);
    }

    [Fact]
    public void ImpulseResponse_GivenFir_MatchesConvolution()
    {
        // a = [1] (no feedback) ⇒ impulse response is exactly the tap vector.
        var h = DspMath.ImpulseResponse(new[] { Cplx.One }, new[] { C("1"), C("2"), C("3") }, 5);
        Assert.Equal(new[] { C("1"), C("2"), C("3"), C("0"), C("0") }, h);
    }

    [Fact]
    public void Sequence_GivenLengthMismatch_ThrowsArgumentException()
    {
        Assert.Throws<ArgumentException>(() => new Sequence(0, 2, new[] { Cplx.One, Cplx.One }));
    }

    [Fact]
    public void Sequence_GivenIndexOutsideSupport_ReturnsZero()
    {
        var s = new Sequence(0, 1, new[] { C("5"), C("7") });
        Assert.Equal(C("5"), s.Get(0));
        Assert.Equal(C("7"), s.Get(1));
        Assert.Equal(Cplx.Zero, s.Get(-1));
        Assert.Equal(Cplx.Zero, s.Get(2));
    }
}
