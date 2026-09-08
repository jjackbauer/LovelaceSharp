using Lovelace.Complex;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Complex.Tests;

/// <summary>
/// Functional tests for <see cref="ComplexMath"/> (SYM-13b): real and complex elementary
/// functions computed at arbitrary precision with no IEEE floating point.
/// </summary>
public class ComplexMathTests
{
    // ------------------------------------------------------------------
    // Exp / Ln / Pow (real)
    // ------------------------------------------------------------------

    [Fact]
    public void Exp_GivenZero_ReturnsOne()
    {
        using var scope = Rl.WithPrecision(50, 15);
        Assert.Equal(Rl.One, ComplexMath.Exp(Rl.Zero));
    }

    [Fact]
    public void Exp_GivenOne_MatchesETo45Digits()
    {
        using var scope = Rl.WithPrecision(50, 15);
        AssertClose(Rl.E, ComplexMath.Exp(Rl.One), 45);
    }

    [Fact]
    public void Exp_GivenLnOfTwo_ReturnsTwoTo45Digits()
    {
        using var scope = Rl.WithPrecision(50, 15);
        AssertClose(new Rl("2"), ComplexMath.Exp(ComplexMath.Ln(new Rl("2"))), 45);
    }

    [Fact]
    public void Ln_GivenOne_ReturnsZero()
    {
        using var scope = Rl.WithPrecision(50, 15);
        Assert.Equal(Rl.Zero, ComplexMath.Ln(Rl.One));
    }

    [Fact]
    public void Ln_GivenNegative_ThrowsArgumentException()
    {
        using var scope = Rl.WithPrecision(50, 15);
        Assert.Throws<ArgumentException>(() => ComplexMath.Ln(new Rl("-1")));
    }

    [Fact]
    public void Ln_GivenZero_ThrowsArgumentException()
    {
        using var scope = Rl.WithPrecision(50, 15);
        Assert.Throws<ArgumentException>(() => ComplexMath.Ln(Rl.Zero));
    }

    // ------------------------------------------------------------------
    // Sin / Cos / Tan / Atan (real)
    // ------------------------------------------------------------------

    [Fact]
    public void Sin_GivenPiOverTwo_ReturnsOne()
    {
        using var scope = Rl.WithPrecision(50, 15);
        Assert.Equal(Rl.One, ComplexMath.Sin(Rl.Pi / new Rl("2")));
    }

    [Fact]
    public void Sin_GivenPi_ReturnsZero()
    {
        using var scope = Rl.WithPrecision(50, 15);
        Assert.Equal(Rl.Zero, ComplexMath.Sin(Rl.Pi));
    }

    [Fact]
    public void Cos_GivenZero_ReturnsOne()
    {
        using var scope = Rl.WithPrecision(50, 15);
        Assert.Equal(Rl.One, ComplexMath.Cos(Rl.Zero));
    }

    [Fact]
    public void Cos_GivenPi_ReturnsNegativeOne()
    {
        using var scope = Rl.WithPrecision(50, 15);
        Assert.Equal(Rl.NegativeOne, ComplexMath.Cos(Rl.Pi));
    }

    [Fact]
    public void Tan_GivenPiOverFour_ReturnsOneTo45Digits()
    {
        using var scope = Rl.WithPrecision(50, 15);
        AssertClose(Rl.One, ComplexMath.Tan(Rl.Pi / new Rl("4")), 45);
    }

    [Fact]
    public void Atan_GivenOne_ReturnsPiOverFourTo45Digits()
    {
        using var scope = Rl.WithPrecision(50, 15);
        AssertClose(Rl.Pi / new Rl("4"), ComplexMath.Atan(Rl.One), 45);
    }

    [Fact]
    public void Atan2_GivenYOneXZero_ReturnsPiOverTwo()
    {
        using var scope = Rl.WithPrecision(50, 15);
        Assert.Equal(Rl.Pi / new Rl("2"), ComplexMath.Atan2(Rl.One, Rl.Zero));
    }

    [Fact]
    public void Atan2_GivenYZeroXNegativeOne_ReturnsPi()
    {
        using var scope = Rl.WithPrecision(50, 15);
        Assert.Equal(Rl.Pi, ComplexMath.Atan2(Rl.Zero, new Rl("-1")));
    }

    // ------------------------------------------------------------------
    // Pow (real)
    // ------------------------------------------------------------------

    [Fact]
    public void Pow_GivenTwoAndHalf_Squared_ReturnsTwoTo45Digits()
    {
        using var scope = Rl.WithPrecision(50, 15);
        Rl root = ComplexMath.Pow(new Rl("2"), new Rl("0.5"));
        AssertClose(new Rl("2"), root * root, 45);
    }

    [Fact]
    public void Pow_GivenNegativeBase_ThrowsArgumentException()
    {
        using var scope = Rl.WithPrecision(50, 15);
        Assert.Throws<ArgumentException>(() => ComplexMath.Pow(new Rl("-1"), new Rl("0.5")));
    }

    // ------------------------------------------------------------------
    // Complex functions
    // ------------------------------------------------------------------

    [Fact]
    public void Exp_GivenIPi_ReturnsNegativeOneTo45Digits()
    {
        using var scope = Rl.WithPrecision(50, 15);
        var iPi = new Complex(Rl.Zero, Rl.Pi);
        AssertClose(new Complex(Rl.NegativeOne, Rl.Zero), ComplexMath.Exp(iPi), 45);
    }

    [Fact]
    public void SinSquaredPlusCosSquared_GivenComplex_ReturnsOne()
    {
        using var scope = Rl.WithPrecision(50, 15);
        var z = new Complex(new Rl("0.3"), new Rl("0.4"));
        Complex s = ComplexMath.Sin(z);
        Complex c = ComplexMath.Cos(z);
        AssertClose(Complex.One, s * s + c * c, 45);
    }

    [Fact]
    public void Log_GivenExpOfZ_ReturnsZTo40Digits()
    {
        using var scope = Rl.WithPrecision(50, 15);
        var z = new Complex(new Rl("0.2"), new Rl("0.3"));
        AssertClose(z, ComplexMath.Log(ComplexMath.Exp(z)), 40);
    }

    [Fact]
    public void Sqrt_GivenZ_Squared_ReturnsZ()
    {
        using var scope = Rl.WithPrecision(50, 15);
        var z = new Complex(new Rl("0.5"), new Rl("0.25"));
        Complex r = ComplexMath.Sqrt(z);
        AssertClose(z, r * r, 45);
    }

    [Fact]
    public void Pow_GivenZAndTwo_ReturnsZSquared()
    {
        using var scope = Rl.WithPrecision(50, 15);
        var z = new Complex(new Rl("0.5"), new Rl("0.25"));
        var result = ComplexMath.Pow(z, new Complex(new Rl("2")));
        AssertClose(z * z, result, 45);
    }

    [Fact]
    public void Log_GivenZero_ThrowsArgumentException()
    {
        using var scope = Rl.WithPrecision(50, 15);
        Assert.Throws<ArgumentException>(() => ComplexMath.Log(Complex.Zero));
    }

    // ------------------------------------------------------------------
    // Inverse trigonometric / hyperbolic (real)
    // ------------------------------------------------------------------

    [Fact]
    public void AsinReal_GivenOne_ReturnsPiOverTwo()
    {
        using var scope = Rl.WithPrecision(50, 15);
        Assert.Equal(Rl.Pi / new Rl("2"), ComplexMath.AsinReal(Rl.One));
    }

    [Fact]
    public void Sin_GivenAsinRealOfPointFive_ReturnsPointFiveTo45Digits()
    {
        using var scope = Rl.WithPrecision(50, 15);
        AssertClose(new Rl("0.5"), ComplexMath.Sin(ComplexMath.AsinReal(new Rl("0.5"))), 45);
    }

    [Fact]
    public void AcosReal_GivenZero_ReturnsPiOverTwo()
    {
        using var scope = Rl.WithPrecision(50, 15);
        Assert.Equal(Rl.Pi / new Rl("2"), ComplexMath.AcosReal(Rl.Zero));
    }

    [Fact]
    public void Exp_GivenAsinhRealOfPointThree_MatchesIdentityTo40Digits()
    {
        using var scope = Rl.WithPrecision(50, 15);
        var x = new Rl("0.3");
        AssertClose(x + Rl.Sqrt(x * x + Rl.One), ComplexMath.Exp(ComplexMath.AsinhReal(x)), 40);
    }

    [Fact]
    public void AcoshReal_GivenTwo_CoshRoundtripReturnsTwo()
    {
        using var scope = Rl.WithPrecision(50, 15);
        var two = new Rl("2");
        Rl a = ComplexMath.AcoshReal(two);
        Rl cosh = (ComplexMath.Exp(a) + ComplexMath.Exp(-a)) / two;
        AssertClose(two, cosh, 45);
    }

    [Fact]
    public void AtanhReal_GivenPointThree_TanhRoundtripReturnsPointThree()
    {
        using var scope = Rl.WithPrecision(50, 15);
        var x = new Rl("0.3");
        Rl t = ComplexMath.AtanhReal(x);
        Rl e2t = ComplexMath.Exp(new Rl("2") * t);
        Rl tanh = (e2t - Rl.One) / (e2t + Rl.One);
        AssertClose(x, tanh, 45);
    }

    [Fact]
    public void AsinReal_GivenTwo_ThrowsArgumentException()
    {
        using var scope = Rl.WithPrecision(50, 15);
        Assert.Throws<ArgumentException>(() => ComplexMath.AsinReal(new Rl("2")));
    }

    [Fact]
    public void AcoshReal_GivenPointFive_ThrowsArgumentException()
    {
        using var scope = Rl.WithPrecision(50, 15);
        Assert.Throws<ArgumentException>(() => ComplexMath.AcoshReal(new Rl("0.5")));
    }

    [Fact]
    public void AtanhReal_GivenOne_ThrowsArgumentException()
    {
        using var scope = Rl.WithPrecision(50, 15);
        Assert.Throws<ArgumentException>(() => ComplexMath.AtanhReal(Rl.One));
    }

    // ------------------------------------------------------------------
    // Tolerance helper
    // ------------------------------------------------------------------

    private static void AssertClose(Rl expected, Rl actual, long digits)
    {
        Rl diff = Rl.Abs(actual - expected);
        Rl tol = Rl.Parse("0." + new string('0', (int)(digits - 1)) + "1");
        Assert.True(diff < tol, $"Expected |actual - expected| < 1e-{digits}; diff = {diff}.");
    }

    private static void AssertClose(Complex expected, Complex actual, long digits)
    {
        AssertClose(expected.Re, actual.Re, digits);
        AssertClose(expected.Im, actual.Im, digits);
    }
}
