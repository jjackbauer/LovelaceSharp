using Lovelace.Dsp;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Dsp.Tests;

/// <summary>
/// Verifies the arbitrary-precision <see cref="Rl.Sin(Rl)"/> / <see cref="Rl.Cos(Rl)"/>,
/// including the exact special-angle results (rational / <see cref="Rl.Sqrt(Rl)"/>-based)
/// that leverage Lovelace's exact representation.
/// </summary>
[Collection("DSP precision")]
public class TrigTests
{
    [Fact]
    public void Sin_GivenZero_IsExactlyZero()
        => Assert.Equal(new Rl("0"), Rl.Sin(new Rl("0")));

    [Fact]
    public void Sin_GivenPiOver6_IsExactlyOneHalf()
        => Assert.Equal(new Rl("0.5"), Rl.Sin(Rl.Pi / new Rl("6")));

    [Fact]
    public void Sin_GivenPiOver4_IsSqrt2Over2()
        => Assert.Equal(Rl.Sqrt(new Rl("2")) / new Rl("2"), Rl.Sin(Rl.Pi / new Rl("4")));

    [Fact]
    public void Sin_GivenPiOver2_IsExactlyOne()
        => Assert.Equal(new Rl("1"), Rl.Sin(Rl.Pi / new Rl("2")));

    [Fact]
    public void Sin_GivenPi_IsExactlyZero()
        => Assert.Equal(new Rl("0"), Rl.Sin(Rl.Pi));

    [Fact]
    public void Sin_GivenThreePiOver2_IsExactlyMinusOne()
        => Assert.Equal(new Rl("-1"), Rl.Sin(Rl.Pi * new Rl("3") / new Rl("2")));

    [Fact]
    public void Cos_GivenZero_IsExactlyOne()
        => Assert.Equal(new Rl("1"), Rl.Cos(new Rl("0")));

    [Fact]
    public void Cos_GivenPiOver3_IsExactlyOneHalf()
        => Assert.Equal(new Rl("0.5"), Rl.Cos(Rl.Pi / new Rl("3")));

    [Fact]
    public void Cos_GivenPiOver6_IsSqrt3Over2()
        => Assert.Equal(Rl.Sqrt(new Rl("3")) / new Rl("2"), Rl.Cos(Rl.Pi / new Rl("6")));

    [Fact]
    public void Cos_GivenPi_IsExactlyMinusOne()
        => Assert.Equal(new Rl("-1"), Rl.Cos(Rl.Pi));

    [Fact]
    public void Sin_GivenNegativeAngle_IsNegated()
        => Assert.Equal(-Rl.Sin(Rl.Pi / new Rl("6")), Rl.Sin(-(Rl.Pi / new Rl("6"))));

    [Fact]
    public void Sin_GivenGeneralAngle_MatchesKnownValue()
    {
        // sin(π/5) = sin(36°) ≈ 0.5877852522924731291687059546…
        Assert.StartsWith("0.58778525229247312916", Rl.Sin(Rl.Pi / new Rl("5"), 30).ToString());
    }
}
