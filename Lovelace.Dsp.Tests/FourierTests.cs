using Cplx = global::Lovelace.Complex.Complex;
using Lovelace.Dsp;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Dsp.Tests;

/// <summary>
/// Functional tests for the transcendental signal surface: <see cref="Rl.Exp(Rl)"/>,
/// <see cref="Cplx.Exp(long)"/>, <see cref="DspMath.Dft"/>, <see cref="Cosine"/>,
/// <see cref="Exponential"/>, and <see cref="Noise"/>.
/// </summary>
[Collection("DSP precision")]
public class FourierTests
{
    private static Cplx C(string re, string im = "0") => new(new Rl(re), new Rl(im));

    [Fact]
    public void Exp_GivenZero_IsExactlyOne()
        => Assert.Equal(new Rl("1"), Rl.Exp(new Rl("0"), 30));

    [Fact]
    public void Exp_GivenOne_MatchesE()
        => Assert.StartsWith("2.71828182845904523536", Rl.Exp(new Rl("1"), 30).ToString());

    [Fact]
    public void ComplexExp_GivenZero_IsExactlyOne()
        => Assert.Equal(Cplx.One, Cplx.Zero.Exp(30));

    [Fact]
    public void Dft_GivenImpulseN4_ReturnsAllOnes()
    {
        var x = new[] { Cplx.One, Cplx.Zero, Cplx.Zero, Cplx.Zero };
        Assert.Equal(new[] { Cplx.One, Cplx.One, Cplx.One, Cplx.One }, DspMath.Dft(x));
    }

    [Fact]
    public void Dft_GivenConstantN4_ReturnsDcOnly()
    {
        var x = new[] { Cplx.One, Cplx.One, Cplx.One, Cplx.One };
        Assert.Equal(new[] { C("4"), Cplx.Zero, Cplx.Zero, Cplx.Zero }, DspMath.Dft(x));
    }

    [Fact]
    public void Dft_GivenImpulseN3_ReturnsAllOnes()
    {
        var x = new[] { Cplx.One, Cplx.Zero, Cplx.Zero };
        Assert.Equal(new[] { Cplx.One, Cplx.One, Cplx.One }, DspMath.Dft(x));
    }

    [Fact]
    public void Cosine_GivenN0_IsExactlyOne()
    {
        var cosine = new Cosine(new Rl("1") / new Rl("4"), new Rl("0"), 30);
        Assert.Equal(Cplx.One, cosine.Get(0));
    }

    [Fact]
    public void Exponential_GivenI_MatchesEuler()
    {
        // e^(i) = cos(1) + i·sin(1) ≈ 0.540302… + 0.841470…i
        var expon = new Exponential(C("0", "1"), 30);
        Cplx v = expon.Get(1);
        Assert.StartsWith("0.5403023058681397174", v.Re.ToString());
        Assert.StartsWith("0.8414709848078965066", v.Im.ToString());
    }

    [Fact]
    public void Noise_GivenSameSeed_IsReproducibleAndBounded()
    {
        var a = new Noise(new Rl("1"), new Rl("0"), 42, 20);
        var b = new Noise(new Rl("1"), new Rl("0"), 42, 20);

        Assert.Equal(a.Get(0), b.Get(0));
        Assert.Equal(a.Get(1), b.Get(1));

        Cplx sample = a.Get(3);
        Assert.True(sample.Re >= new Rl("0") && sample.Re < new Rl("1"));
        Assert.True(sample.Im >= new Rl("0") && sample.Im < new Rl("1"));
    }
}
