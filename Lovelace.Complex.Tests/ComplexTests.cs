using Lovelace.Complex;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Complex.Tests;

/// <summary>
/// Functional tests for <see cref="Complex"/>: component-wise arithmetic, the imaginary unit,
/// conjugate, magnitude, reciprocal/division, equality, and the string round-trip.
/// </summary>
public class ComplexTests
{
    [Fact]
    public void Add_GivenTwoComplex_ReturnsComponentwiseSum()
    {
        var a = new Complex(new Rl("1"), new Rl("2"));
        var b = new Complex(new Rl("3"), new Rl("4"));
        Assert.Equal(new Complex(new Rl("4"), new Rl("6")), a + b);
    }

    [Fact]
    public void Multiply_GivenTwoComplex_ReturnsFoilProduct()
    {
        var a = new Complex(new Rl("1"), new Rl("2"));
        var b = new Complex(new Rl("3"), new Rl("4"));
        // (1+2i)(3+4i) = 3 + 4i + 6i + 8i² = -5 + 10i
        Assert.Equal(new Complex(new Rl("-5"), new Rl("10")), a * b);
    }

    [Fact]
    public void ImaginaryUnit_GivenISquared_ReturnsMinusOne()
    {
        Assert.Equal(-Complex.One, Complex.I * Complex.I);
    }

    [Fact]
    public void Conjugate_GivenComplex_NegatesImaginary()
    {
        var z = new Complex(new Rl("2"), new Rl("3"));
        Assert.Equal(new Complex(new Rl("2"), new Rl("-3")), z.Conjugate);
    }

    [Fact]
    public void Magnitude_Given345_ReturnsFive()
    {
        var z = new Complex(new Rl("3"), new Rl("4"));
        Assert.Equal(new Rl("5"), z.Magnitude);
        Assert.Equal(new Rl("25"), z.MagnitudeSquared);
    }

    [Fact]
    public void Reciprocal_GivenComplex_MultipliesToOne()
    {
        var z = new Complex(new Rl("1"), new Rl("2"));
        Assert.Equal(Complex.One, z * z.Reciprocal);
    }

    [Fact]
    public void Divide_GivenComplex_ReturnsExactQuotient()
    {
        var a = new Complex(new Rl("1"), new Rl("2"));
        var b = new Complex(new Rl("1"), new Rl("1"));
        // (1+2i)/(1+i) = (3+i)/2 = 1.5 + 0.5i
        Assert.Equal(new Complex(new Rl("1.5"), new Rl("0.5")), a / b);
    }

    [Fact]
    public void Equality_GivenComplex_IsComponentwise()
    {
        Assert.Equal(new Complex(new Rl("1"), new Rl("2")), new Complex(new Rl("1"), new Rl("2")));
        Assert.NotEqual(new Complex(new Rl("1"), new Rl("2")), new Complex(new Rl("1"), new Rl("3")));
    }

    [Theory]
    [InlineData("2", "2", "0")]
    [InlineData("4i", "0", "4")]
    [InlineData("-3i", "0", "-3")]
    [InlineData("i", "0", "1")]
    [InlineData("-i", "0", "-1")]
    [InlineData("1.5+0.5i", "1.5", "0.5")]
    [InlineData("1.5 - 0.5i", "1.5", "-0.5")]
    [InlineData("-2+3i", "-2", "3")]
    public void Parse_GivenStandardForms_ReturnsComponents(string text, string re, string im)
    {
        var z = Complex.Parse(text);
        Assert.Equal(new Rl(re), z.Re);
        Assert.Equal(new Rl(im), z.Im);
    }

    [Fact]
    public void ToString_GivenComplex_RendersStandardForms()
    {
        Assert.Equal("2", new Complex(new Rl("2"), new Rl("0")).ToString());
        Assert.Equal("4i", new Complex(new Rl("0"), new Rl("4")).ToString());
        Assert.Equal("1.5 + 0.5i", new Complex(new Rl("1.5"), new Rl("0.5")).ToString());
        Assert.Equal("1.5 - 0.5i", new Complex(new Rl("1.5"), new Rl("-0.5")).ToString());
    }

    [Theory]
    [InlineData("2")]
    [InlineData("4i")]
    [InlineData("1.5 + 0.5i")]
    [InlineData("-2+3i")]
    public void Parse_GivenToStringOutput_ProducesEqualValue(string text)
    {
        var z = Complex.Parse(text);
        Assert.Equal(z, Complex.Parse(z.ToString()));
    }

    [Fact]
    public void ETo_Given30Digits_MatchesKnownConstant()
    {
        Assert.StartsWith("2.71828182845904523536", Rl.ETo(30).ToString());
    }

    [Fact]
    public void PiTo_Given30Digits_MatchesKnownConstant()
    {
        Assert.StartsWith("3.14159265358979323846", Rl.PiTo(30).ToString());
    }

    [Fact]
    public void Constants_PiAndE_AreRealValued()
    {
        Assert.Equal(new Rl("0"), Complex.Pi.Im);
        Assert.Equal(new Rl("0"), Complex.E.Im);
        Assert.True(Complex.Pi.Re > new Rl("3"));
        Assert.True(Complex.E.Re > new Rl("2"));
    }

    // -------------------------------------------------------------------------
    // Fixed-width fast path (mirrors NumericOps.ApplyRealBinary)
    // -------------------------------------------------------------------------

    [Fact]
    public void Arithmetic_GivenLimitedPrecision_MatchesExactResult()
    {
        // ≤ 37 digits → the LComplex64/LComplex128 fast path is active.
        using var _ = Rl.WithPrecision(20, 20);

        var a = new Complex(new Rl("1"), new Rl("2"));
        var b = new Complex(new Rl("3"), new Rl("4"));

        Assert.Equal(new Complex(new Rl("4"), new Rl("6")), a + b);
        Assert.Equal(new Complex(new Rl("-2"), new Rl("-2")), a - b);
        Assert.Equal(new Complex(new Rl("-5"), new Rl("10")), a * b);
    }

    [Fact]
    public void Arithmetic_GivenOverflowAtLimitedPrecision_FallsBackExactly()
    {
        // 10-digit × 10-digit → 20 digits: overflows LComplex64 (19) but fits LComplex128 (38),
        // so the fast path promotes one width without rounding.
        using var _ = Rl.WithPrecision(20, 20);

        var a = new Complex(new Rl("9999999999"), new Rl("0"));
        var expected = new Complex(new Rl("99999999980000000001"), new Rl("0"));

        Assert.Equal(expected, a * a);
    }

    [Fact]
    public void Arithmetic_GivenOverflowBeyondFixedWidth_FallsBackToArbitraryPrecision()
    {
        // 37-digit × 37-digit → 74 digits: overflows LComplex128 (38), so the arbitrary-precision
        // class path produces the exact (unrounded) result.
        using var _ = Rl.WithPrecision(37, 37);

        var a = new Complex(new Rl("9999999999999999999999999999999999999"), new Rl("0"));
        var product = a * a;

        Assert.Equal(74, product.Re.ToNatural().ToString().Length);
        Assert.True(Rl.IsZero(product.Im));
    }
}
