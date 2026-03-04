using Lovelace.Integer;

namespace Lovelace.Integer.Tests;

public class IntegerDivideTests
{
    // -------------------------------------------------------------------------
    // DivRem
    // -------------------------------------------------------------------------

    [Fact]
    public void DivRem_GivenPositiveDividendAndDivisor_ReturnsCorrectQuotientAndRemainder()
    {
        var dividend = new Integer(10L);
        var divisor = new Integer(3L);

        var quotient = dividend.DivRem(divisor, out var remainder);

        Assert.Equal(new Integer(3L), quotient);
        Assert.Equal(new Integer(1L), remainder);
    }

    [Fact]
    public void DivRem_GivenNegativeDividend_QuotientIsNegative()
    {
        var dividend = new Integer(-10L);
        var divisor = new Integer(3L);

        var quotient = dividend.DivRem(divisor, out _);

        Assert.True(Integer.IsNegative(quotient));
    }

    [Fact]
    public void DivRem_GivenBothNegative_QuotientIsPositive()
    {
        var dividend = new Integer(-10L);
        var divisor = new Integer(-3L);

        var quotient = dividend.DivRem(divisor, out _);

        Assert.True(Integer.IsPositive(quotient));
    }

    [Fact]
    public void DivRem_GivenExactDivision_RemainderIsZero()
    {
        var dividend = new Integer(12L);
        var divisor = new Integer(4L);

        dividend.DivRem(divisor, out var remainder);

        Assert.True(Integer.IsZero(remainder));
    }

    [Fact]
    public void DivRem_GivenNegativeDividend_ReturnsCorrectMagnitudes()
    {
        // -10 / 3 → quotient magnitude 3, remainder magnitude 1
        var quotient = new Integer(-10L).DivRem(new Integer(3L), out var remainder);

        Assert.Equal(new Integer(3L), Integer.Abs(quotient));
        Assert.Equal(new Integer(1L), Integer.Abs(remainder));
    }

    [Fact]
    public void DivRem_GivenBothNegative_ReturnsCorrectMagnitudes()
    {
        // -10 / -3 → quotient 3 (positive), remainder magnitude 1
        var quotient = new Integer(-10L).DivRem(new Integer(-3L), out var remainder);

        Assert.Equal(new Integer(3L), quotient);
        Assert.Equal(new Integer(1L), Integer.Abs(remainder));
    }

    // -------------------------------------------------------------------------
    // operator/
    // -------------------------------------------------------------------------

    [Fact]
    public void DivisionOperator_GivenPositives_ReturnsQuotient()
    {
        var result = new Integer(10L) / new Integer(3L);

        Assert.Equal(new Integer(3L), result);
    }

    [Fact]
    public void DivisionOperator_GivenNegativeDividend_ReturnsNegativeQuotient()
    {
        var result = new Integer(-10L) / new Integer(3L);

        Assert.True(Integer.IsNegative(result));
        Assert.Equal(new Integer(3L), Integer.Abs(result));
    }

    [Fact]
    public void DivisionOperator_GivenBothNegative_ReturnsPositiveQuotient()
    {
        var result = new Integer(-10L) / new Integer(-3L);

        Assert.True(Integer.IsPositive(result));
        Assert.Equal(new Integer(3L), result);
    }

    // -------------------------------------------------------------------------
    // operator%
    // -------------------------------------------------------------------------

    [Fact]
    public void ModulusOperator_GivenPositives_ReturnsRemainder()
    {
        var result = new Integer(10L) % new Integer(3L);

        Assert.Equal(new Integer(1L), result);
    }

    [Fact]
    public void ModulusOperator_GivenExactDivision_ReturnsZero()
    {
        var result = new Integer(12L) % new Integer(4L);

        Assert.True(Integer.IsZero(result));
    }

    [Fact]
    public void ModulusOperator_GivenLargeDividend_ReturnsCorrectRemainder()
    {
        // 100 / 7 = 14 remainder 2
        var result = new Integer(100L) % new Integer(7L);

        Assert.Equal(new Integer(2L), result);
    }
}
