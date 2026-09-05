using Cplx = global::Lovelace.Complex.Complex;
using Lovelace.Dsp;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Dsp.Tests;

/// <summary>
/// Functional tests for <see cref="DspMath.Fft"/>: identity/DC cases, parity with
/// <see cref="DspMath.Dft"/>, spectral-peak placement, and error/empty edge cases.
/// </summary>
[Collection("DSP precision")]
public class FftTests
{
    private static Cplx C(string re, string im = "0") => new(new Rl(re), new Rl(im));
    private static Cplx Z(long re, long im = 0) => new(new Rl(re.ToString()), new Rl(im.ToString()));

    [Fact]
    public void Fft_GivenImpulseN1_ReturnsIdentity()
        => Assert.Equal(new[] { Cplx.One }, DspMath.Fft(new[] { Cplx.One }));

    [Fact]
    public void Fft_GivenImpulseN2_ReturnsAllOnes()
        => Assert.Equal(new[] { Cplx.One, Cplx.One }, DspMath.Fft(new[] { Cplx.One, Cplx.Zero }));

    [Fact]
    public void Fft_GivenConstantN2_ReturnsDcOnly()
        => Assert.Equal(new[] { C("2"), Cplx.Zero }, DspMath.Fft(new[] { Cplx.One, Cplx.One }));

    [Fact]
    public void Fft_GivenImpulseN4_ReturnsAllOnes()
        => Assert.Equal(
            new[] { Cplx.One, Cplx.One, Cplx.One, Cplx.One },
            DspMath.Fft(new[] { Cplx.One, Cplx.Zero, Cplx.Zero, Cplx.Zero }));

    [Fact]
    public void Fft_GivenConstantN4_ReturnsDcOnly()
        => Assert.Equal(
            new[] { C("4"), Cplx.Zero, Cplx.Zero, Cplx.Zero },
            DspMath.Fft(new[] { Cplx.One, Cplx.One, Cplx.One, Cplx.One }));

    [Fact]
    public void Fft_GivenStructuredN8_MatchesDft()
        => AssertMatchesDft(8);

    [Fact]
    public void Fft_GivenStructuredN16_MatchesDft()
        => AssertMatchesDft(16);

    [Fact]
    public void Fft_GivenStructuredN64_MatchesDft()
        => AssertMatchesDft(64);

    [Fact]
    public void Fft_GivenRealCosine_ReturnsExpectedSpectralPeaks()
    {
        // cos(2π·n/8) is a pure tone at bin 1 (and its conjugate image at bin 7).
        var cosine = new Cosine(new Rl("1") / new Rl("8"), new Rl("0"), 50);
        var x = Signal.Sample(cosine, 0, 7);
        var y = DspMath.Fft(x, 50);

        Assert.Equal(8, y.Length);
        // |X[1]|² and |X[7]|² ≈ (N/2)² = 16, every other bin ≈ 0. The √2-based samples are
        // truncated irrationals, so the peaks are exact only to the working precision.
        var tol = new Rl("0." + new string('0', 40) + "1");   // 10⁻⁴¹
        AssertClose(new Rl("16"), y[1].MagnitudeSquared, tol, "bin 1");
        AssertClose(new Rl("16"), y[7].MagnitudeSquared, tol, "bin 7");
        for (int k = 0; k < 8; k++)
        {
            if (k is 1 or 7) continue;
            AssertClose(new Rl("0"), y[k].MagnitudeSquared, tol, $"bin {k}");
        }
    }

    [Fact]
    public void Fft_GivenNonPowerOfTwo_ThrowsArgumentException()
    {
        var x = new[] { Cplx.One, Cplx.One, Cplx.One };
        Assert.Throws<ArgumentException>(() => DspMath.Fft(x));
    }

    [Fact]
    public void Fft_GivenEmpty_ReturnsEmpty()
        => Assert.Empty(DspMath.Fft([]));

    private static void AssertMatchesDft(int n)
    {
        // A deterministic complex signal with both real and imaginary structure.
        var x = new Cplx[n];
        for (int i = 0; i < n; i++)
            x[i] = Z(i % 7, i % 5);

        var fft = DspMath.Fft(x, 50);
        var dft = DspMath.Dft(x, 50);

        Assert.Equal(dft.Length, fft.Length);
        var tol = new Rl("0." + new string('0', 40) + "1");   // 10⁻⁴¹
        for (int i = 0; i < dft.Length; i++)
        {
            AssertClose(fft[i].Re, dft[i].Re, tol, $"Re at bin {i}");
            AssertClose(fft[i].Im, dft[i].Im, tol, $"Im at bin {i}");
        }
    }

    private static void AssertClose(Rl expected, Rl actual, Rl tol, string label)
    {
        Rl diff = Rl.Abs(actual - expected);
        Assert.True(diff <= tol, $"{label}: expected {expected}, got {actual} (diff {diff})");
    }
}
