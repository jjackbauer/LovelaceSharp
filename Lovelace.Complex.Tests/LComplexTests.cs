using Lovelace.Complex;
using Lovelace.Real;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Complex.Tests;

/// <summary>
/// Functional tests for the fixed-width complex structs (<see cref="LComplex64"/> over
/// <c>LReal64</c> and <see cref="LComplex128"/> over <c>LReal128</c>): exact arithmetic,
/// conversion to/from <see cref="Complex"/>, and promotion on overflow.
/// </summary>
public class LComplex64Tests
{
    [Fact]
    public void Add_GivenTwoComplex_ReturnsComponentwiseSum()
    {
        Assert.Equal(LComplex64.Parse("4+6i"), LComplex64.Parse("1+2i") + LComplex64.Parse("3+4i"));
    }

    [Fact]
    public void ImaginaryUnit_GivenISquared_ReturnsMinusOne()
    {
        Assert.Equal(-LComplex64.One, LComplex64.I * LComplex64.I);
    }

    [Fact]
    public void Conjugate_GivenComplex_NegatesImaginary()
    {
        Assert.Equal(LComplex64.Parse("2-3i"), LComplex64.Parse("2+3i").Conjugate);
    }

    [Fact]
    public void Reciprocal_GivenComplex_MultipliesToOne()
    {
        var z = LComplex64.Parse("1+2i");
        Assert.Equal(LComplex64.One, z * z.Reciprocal);
    }

    [Fact]
    public void TryFromComplex_GivenFittingComplex_RoundTrips()
    {
        var source = new Complex(new Rl("1.5"), new Rl("-2.5"));
        Assert.True(LComplex64.TryFromComplex(source, out var narrow));
        Assert.Equal(source, narrow.ToComplex());
    }

    [Fact]
    public void TryFromComplex_GivenTooManyDigits_ReturnsFalse()
    {
        // 20 nines exceed ulong.MaxValue, so the real component does not fit LReal64.
        var source = new Complex(new Rl("99999999999999999999"), new Rl("0"));
        Assert.False(LComplex64.TryFromComplex(source, out _));
    }

    [Fact]
    public void Multiply_GivenOverflow_ThrowsPromoteException()
    {
        var z = LComplex64.Parse("9999999999");
        Assert.Throws<LRealPromoteException>(() => z * z);
    }

    [Theory]
    [InlineData("2")]
    [InlineData("4i")]
    [InlineData("1.5 + 0.5i")]
    [InlineData("-2+3i")]
    public void Parse_GivenToStringOutput_ProducesEqualValue(string text)
    {
        var z = LComplex64.Parse(text);
        Assert.Equal(z, LComplex64.Parse(z.ToString()));
    }
}

/// <summary>Same coverage as <see cref="LComplex64Tests"/> for the 128-bit component form.</summary>
public class LComplex128Tests
{
    [Fact]
    public void Add_GivenTwoComplex_ReturnsComponentwiseSum()
    {
        Assert.Equal(LComplex128.Parse("4+6i"), LComplex128.Parse("1+2i") + LComplex128.Parse("3+4i"));
    }

    [Fact]
    public void ImaginaryUnit_GivenISquared_ReturnsMinusOne()
    {
        Assert.Equal(-LComplex128.One, LComplex128.I * LComplex128.I);
    }

    [Fact]
    public void Conjugate_GivenComplex_NegatesImaginary()
    {
        Assert.Equal(LComplex128.Parse("2-3i"), LComplex128.Parse("2+3i").Conjugate);
    }

    [Fact]
    public void Reciprocal_GivenComplex_MultipliesToOne()
    {
        var z = LComplex128.Parse("1+2i");
        Assert.Equal(LComplex128.One, z * z.Reciprocal);
    }

    [Fact]
    public void TryFromComplex_GivenFittingComplex_RoundTrips()
    {
        var source = new Complex(new Rl("1.5"), new Rl("-2.5"));
        Assert.True(LComplex128.TryFromComplex(source, out var narrow));
        Assert.Equal(source, narrow.ToComplex());
    }

    [Fact]
    public void TryFromComplex_GivenTooManyDigits_ReturnsFalse()
    {
        // 39 nines exceed UInt128.MaxValue, so the real component does not fit LReal128.
        var source = new Complex(new Rl("999999999999999999999999999999999999999"), new Rl("0"));
        Assert.False(LComplex128.TryFromComplex(source, out _));
    }

    [Fact]
    public void Multiply_GivenOverflow_ThrowsPromoteException()
    {
        var z = LComplex128.Parse("99999999999999999999");   // 20 digits, exceeds 38 only when squared
        Assert.Throws<LRealPromoteException>(() => z * z);
    }

    [Theory]
    [InlineData("2")]
    [InlineData("4i")]
    [InlineData("1.5 + 0.5i")]
    [InlineData("-2+3i")]
    public void Parse_GivenToStringOutput_ProducesEqualValue(string text)
    {
        var z = LComplex128.Parse(text);
        Assert.Equal(z, LComplex128.Parse(z.ToString()));
    }
}
