using Lovelace.Real;
using Xunit;

namespace Lovelace.Real.Tests;

/// <summary>
/// Functional tests for <see cref="Real.Multiply"/> / <c>operator*</c>.
/// </summary>
public class RealMultiplyTests
{
    [Fact]
    public void Multiply_GivenTwoPositiveDecimals_ReturnsCorrectProduct()
    {
        // 1.5 (magnitude=15, exp=-1) × 2.0 (magnitude=20, exp=-1)
        // → magnitude=300, exp=-2 → normalize trailing zeros → "3"
        Real a = new Real("1.5");
        Real b = new Real("2.0");
        Real result = a * b;
        Assert.Equal(new Real("3"), result);
        Assert.False(Real.IsNegative(result));
    }

    [Fact]
    public void Multiply_GivenPositiveAndNegative_ReturnsNegativeProduct()
    {
        // 3.0 × -2.0 → magnitude=600, exp=-2 → normalize → "-6"
        Real a = new Real("3.0");
        Real b = new Real("-2.0");
        Real result = a * b;
        Assert.Equal(new Real("-6"), result);
        Assert.True(Real.IsNegative(result));
    }

    [Fact]
    public void Multiply_GivenTwoNegatives_ReturnsPositiveProduct()
    {
        // -3.0 × -2.0 → positive, normalize trailing zeros → "6"
        Real a = new Real("-3.0");
        Real b = new Real("-2.0");
        Real result = a * b;
        Assert.False(Real.IsNegative(result));
        Assert.Equal(new Real("6"), result);
    }

    [Fact]
    public void Multiply_GivenExponents_SumsThemInResult()
    {
        // exp=-1 × exp=-2 → result exp=-3
        Real a = new Real("1.5");   // exp=-1
        Real b = new Real("1.25"); // exp=-2
        Real result = a * b;
        Assert.Equal(-3L, result.Exponent);
    }

    [Fact]
    public void Multiply_GivenZeroFactor_ReturnsZero()
    {
        Real a = new Real("0.0");
        Real b = new Real("999.9");
        Real result = a * b;
        Assert.True(Real.IsZero(result));
    }

    [Fact]
    public void Multiply_GivenFractionalValues_ProducesCorrectResult()
    {
        // 0.1 (magnitude=1, exp=-1) × 0.1 → magnitude=1, exp=-2 → "0.01"
        Real a = new Real("0.1");
        Real b = new Real("0.1");
        Real result = a * b;
        Assert.Equal(new Real("0.01"), result);
    }

    [Fact]
    public void Multiply_GivenIntegerValues_ReturnsIntegerProduct()
    {
        // 6 × 7 = 42; both have exp=0, result exp=0
        Real a = new Real("6");
        Real b = new Real("7");
        Real result = a * b;
        Assert.Equal(0L, result.Exponent);
        Assert.Equal(new Real("42"), result);
    }

    [Fact]
    public void Multiply_GivenMultiplicativeIdentity_ReturnsSelf()
    {
        // x × 1 = x (with exponent alignment)
        // "1" has exp=0, so result.Exponent = input.Exponent + 0 = input.Exponent
        Real a = new Real("3.14");
        Real one = new Real("1");
        Real result = a * one;
        Assert.Equal(a.Exponent, result.Exponent);
        Assert.Equal(a, result);
    }
}
