using Lovelace.Real;

namespace Lovelace.Real.Tests;

/// <summary>
/// Functional tests covering <see cref="Real.Negate"/> / unary <c>operator-</c>.
///
/// Checklist item covered:
///   - static Real Negate(Real value) / unary operator-
/// </summary>
public class RealNegationTests
{
    [Fact]
    public void Negate_GivenPositiveValue_ReturnsNegative()
    {
        // -3.14 should produce IsNegative == true and ToString == "-3.14"
        Real r = new Real("3.14");
        Real neg = -r;
        Assert.True(Real.IsNegative(neg));
        Assert.Equal(new Real("-3.14"), neg);
    }

    [Fact]
    public void Negate_GivenNegativeValue_ReturnsPositive()
    {
        // -(-2.5) should produce IsNegative == false and ToString == "2.5"
        Real r = new Real("-2.5");
        Real pos = -r;
        Assert.False(Real.IsNegative(pos));
        Assert.Equal(new Real("2.5"), pos);
    }

    [Fact]
    public void Negate_GivenZero_ReturnsZero()
    {
        // -0.0 is zero (sign of zero is not significant)
        Real r = new Real(0.0);
        Real neg = -r;
        Assert.True(Real.IsZero(neg));
    }

    [Fact]
    public void Negate_ReturnsReal_NotInteger_AndPreservesExponent()
    {
        // The return type is Real, and Exponent must survive negation.
        Real r = new Real("3.14");   // Exponent == -2
        Real neg = -r;
        Assert.Equal(-2L, neg.Exponent);
        Assert.Equal(new Real("-3.14"), neg);
    }

    [Fact]
    public void Negate_GivenPeriodicValue_PreservesPeriodMetadata()
    {
        // Negating 0.(3) must keep PeriodStart, PeriodLength, and yield "-0.(3)".
        Real r = Real.Parse("0.(3)");
        Real neg = -r;
        Assert.True(neg.IsPeriodic);
        Assert.Equal(r.PeriodStart,  neg.PeriodStart);
        Assert.Equal(r.PeriodLength, neg.PeriodLength);
        Assert.Equal(Real.Parse("-0.(3)"), neg);
    }

    [Fact]
    public void Negate_IsOwnInverse()
    {
        // Double negation must return to the original value.
        Real r = new Real("7.5");
        Assert.Equal(new Real("7.5"), -(-r));
    }
}
