using Lovelace.Real;
using Xunit;

namespace Lovelace.Real.Tests;

public class RealPowTests
{
    // Pow_GivenPositiveIntegerExponent_ReturnsCorrectResult
    // 2.0^3: repeated multiply normalises trailing zeros → "8"
    [Fact]
    public void Pow_GivenPositiveIntegerExponent_ReturnsCorrectResult()
    {
        var result = new Real("2.0").Pow(new Real("3.0"));
        Assert.Equal(new Real("8"), result);
    }

    // Pow_GivenExponentZero_ReturnsOne
    // x^0 → returns Real.One which has Exponent=0 and digits "1" → ToString()=="1"
    [Fact]
    public void Pow_GivenExponentZero_ReturnsOne()
    {
        var result = new Real("999.0").Pow(new Real("0.0"));
        Assert.Equal(new Real("1"), result);
    }

    // Pow_GivenExponentOne_ReturnsSelf
    // x^1 → returns the base unchanged
    [Fact]
    public void Pow_GivenExponentOne_ReturnsSelf()
    {
        var result = new Real("7.5").Pow(new Real("1.0"));
        Assert.Equal(new Real("7.5"), result);
    }

    // Pow_GivenBaseZeroExponentPositive_ReturnsZero
    // 0^n → returns Real.Zero → ToString()=="0"
    [Fact]
    public void Pow_GivenBaseZeroExponentPositive_ReturnsZero()
    {
        var result = new Real("0.0").Pow(new Real("5.0"));
        Assert.Equal(new Real("0"), result);
    }

    // Pow_GivenNegativeBase_EvenExponent_ReturnsPositive
    // (-2.0)^2: multiply normalises trailing zeros → "4"; sign becomes positive
    [Fact]
    public void Pow_GivenNegativeBase_EvenExponent_ReturnsPositive()
    {
        var result = new Real("-2.0").Pow(new Real("2.0"));
        Assert.Equal(new Real("4"), result);
        Assert.False(Real.IsNegative(result));
    }
}
