using Cplx = global::Lovelace.Complex.Complex;
using Cplx64 = global::Lovelace.Complex.LComplex64;
using Cplx128 = global::Lovelace.Complex.LComplex128;
using Lovelace.Dsp;
using Lovelace.Real;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Dsp.Tests;

/// <summary>
/// Functional tests for <see cref="FixedDsp"/>: the fixed-width (LComplex64/LComplex128)
/// whole-array operations, their parity with <see cref="DspMath"/>, and the
/// promote-on-overflow contract (<see cref="LRealPromoteException"/> instead of rounding).
/// </summary>
[Collection("DSP precision")]
public class FixedDspTests
{
    private static Cplx C(string re, string im = "0") => new(new Rl(re), new Rl(im));

    [Fact]
    public void Convolve64_GivenTwoOnes_ProducesTriangle()
    {
        var x = new[] { Cplx64.One, Cplx64.One };
        var result = FixedDsp.Convolve(x, x);

        Assert.Equal(new[] { Cplx64.One, Cplx64.Parse("2"), Cplx64.One }, result);
    }

    [Fact]
    public void Convolve64_Given123SelfConvolution_MatchesDspMath()
    {
        var x64 = new[] { Cplx64.Parse("1"), Cplx64.Parse("2"), Cplx64.Parse("3") };
        var fixedResult = FixedDsp.Convolve(x64, x64);
        var classResult = DspMath.Convolve(new[] { C("1"), C("2"), C("3") }, new[] { C("1"), C("2"), C("3") });

        Assert.Equal(classResult, fixedResult.Select(v => v.ToComplex()).ToArray());
    }

    [Fact]
    public void Convolve128_Given123SelfConvolution_MatchesDspMath()
    {
        var x128 = new[] { Cplx128.Parse("1"), Cplx128.Parse("2"), Cplx128.Parse("3") };
        var fixedResult = FixedDsp.Convolve(x128, x128);
        var classResult = DspMath.Convolve(new[] { C("1"), C("2"), C("3") }, new[] { C("1"), C("2"), C("3") });

        Assert.Equal(classResult, fixedResult.Select(v => v.ToComplex()).ToArray());
    }

    [Fact]
    public void Convolve64_GivenWideOperands_ThrowsPromoteException()
    {
        var big = Cplx64.Parse("9999999999999999999");   // 19 significant digits; the square needs 38
        Assert.Throws<LRealPromoteException>(() => FixedDsp.Convolve(new[] { big }, new[] { big }));
    }

    [Fact]
    public void Convolve128_GivenWideOperands_ThrowsPromoteException()
    {
        var big = Cplx128.Parse("99999999999999999999999999999999999999");   // 38 digits; the square needs 76
        Assert.Throws<LRealPromoteException>(() => FixedDsp.Convolve(new[] { big }, new[] { big }));
    }

    [Fact]
    public void ImpulseResponse64_GivenFir_MatchesTaps()
    {
        var a = new[] { Cplx64.One };
        var b = new[] { Cplx64.Parse("1"), Cplx64.Parse("2"), Cplx64.Parse("3") };
        var result = FixedDsp.ImpulseResponse(a, b, 5);

        Assert.Equal(
            new[] { Cplx64.Parse("1"), Cplx64.Parse("2"), Cplx64.Parse("3"), Cplx64.Zero, Cplx64.Zero },
            result);
    }

    [Fact]
    public void ImpulseResponse128_GivenFir_MatchesTaps()
    {
        var a = new[] { Cplx128.One };
        var b = new[] { Cplx128.Parse("1"), Cplx128.Parse("2"), Cplx128.Parse("3") };
        var result = FixedDsp.ImpulseResponse(a, b, 5);

        Assert.Equal(
            new[] { Cplx128.Parse("1"), Cplx128.Parse("2"), Cplx128.Parse("3"), Cplx128.Zero, Cplx128.Zero },
            result);
    }

    [Fact]
    public void MovingAverage64_GivenPowerOfTwoWindow_MatchesDspMath()
    {
        var x64 = new Cplx64[8];
        var xc = new Cplx[8];
        for (int i = 0; i < 8; i++)
        {
            x64[i] = Cplx64.Parse((i % 7).ToString());
            xc[i] = C((i % 7).ToString());
        }

        var fixedResult = FixedDsp.MovingAverage(x64, 4);
        var classResult = Signal.Sample(new DspMath.MovingAverage(4, new Sequence(0, 7, xc)), 0, 7);

        Assert.Equal(classResult, fixedResult.Select(v => v.ToComplex()).ToArray());
    }

    [Fact]
    public void MovingAverage128_GivenPowerOfTwoWindow_MatchesDspMath()
    {
        var x128 = new Cplx128[8];
        var xc = new Cplx[8];
        for (int i = 0; i < 8; i++)
        {
            x128[i] = Cplx128.Parse((i % 7).ToString());
            xc[i] = C((i % 7).ToString());
        }

        var fixedResult = FixedDsp.MovingAverage(x128, 4);
        var classResult = Signal.Sample(new DspMath.MovingAverage(4, new Sequence(0, 7, xc)), 0, 7);

        Assert.Equal(classResult, fixedResult.Select(v => v.ToComplex()).ToArray());
    }

    [Fact]
    public void MovingAverage64_GivenNonPositiveWindow_ThrowsArgumentOutOfRange()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => FixedDsp.MovingAverage(new[] { Cplx64.One }, 0));
    }
}
