using Lovelace.Natural;

namespace Lovelace.Natural.Tests;

public class NaturalDivideTests
{
    // -------------------------------------------------------------------------
    // DivRem
    // -------------------------------------------------------------------------

    [Fact]
    public void DivRem_GivenExactDivision_ReturnsZeroRemainder()
    {
        var quotient = Natural.DivRem(new Natural(12UL), new Natural(4UL), out var remainder);

        Assert.Equal(new Natural(3UL), quotient);
        Assert.Equal(new Natural(), remainder);
    }

    [Fact]
    public void DivRem_GivenNonExactDivision_ReturnsCorrectQuotientAndRemainder()
    {
        var quotient = Natural.DivRem(new Natural(13UL), new Natural(4UL), out var remainder);

        Assert.Equal(new Natural(3UL), quotient);
        Assert.Equal(new Natural(1UL), remainder);
    }

    [Fact]
    public void DivRem_GivenDividendSmallerThanDivisor_ReturnsZeroQuotient()
    {
        var quotient = Natural.DivRem(new Natural(3UL), new Natural(10UL), out var remainder);

        Assert.Equal(new Natural(), quotient);
        Assert.Equal(new Natural(3UL), remainder);
    }

    [Fact]
    public void DivRem_GivenDivisorOne_ReturnsOriginalWithZeroRemainder()
    {
        var quotient = Natural.DivRem(new Natural(12345UL), new Natural(1UL), out var remainder);

        Assert.Equal(new Natural(12345UL), quotient);
        Assert.Equal(new Natural(), remainder);
    }

    [Fact]
    public void DivRem_GivenEqualOperands_ReturnsOneAndZeroRemainder()
    {
        var quotient = Natural.DivRem(new Natural(99UL), new Natural(99UL), out var remainder);

        Assert.Equal(new Natural(1UL), quotient);
        Assert.Equal(new Natural(), remainder);
    }

    [Fact]
    public void DivRem_GivenDivisorZero_ThrowsDivideByZeroException()
    {
        Assert.Throws<DivideByZeroException>(() =>
            Natural.DivRem(new Natural(5UL), new Natural(), out _));
    }

    [Theory]
    [InlineData(100UL, 3UL, 33UL, 1UL)]
    [InlineData(1000UL, 7UL, 142UL, 6UL)]
    [InlineData(999UL, 9UL, 111UL, 0UL)]
    [InlineData(0UL, 5UL, 0UL, 0UL)]
    public void DivRem_GivenVariousInputs_ReturnsCorrectQuotientAndRemainder(
        ulong dividend, ulong divisor, ulong expectedQuotient, ulong expectedRemainder)
    {
        var quotient = Natural.DivRem(new Natural(dividend), new Natural(divisor), out var remainder);

        Assert.Equal(new Natural(expectedQuotient), quotient);
        Assert.Equal(new Natural(expectedRemainder), remainder);
    }

    // -------------------------------------------------------------------------
    // operator /
    // -------------------------------------------------------------------------

    [Fact]
    public void OperatorDivide_GivenExactDivision_ReturnsQuotient()
    {
        var result = new Natural(20UL) / new Natural(4UL);

        Assert.Equal(new Natural(5UL), result);
    }

    [Fact]
    public void OperatorDivide_GivenNonExactDivision_ReturnsTruncatedQuotient()
    {
        var result = new Natural(17UL) / new Natural(3UL);

        Assert.Equal(new Natural(5UL), result);
    }

    [Fact]
    public void OperatorDivide_GivenDivisorZero_ThrowsDivideByZeroException()
    {
        Assert.Throws<DivideByZeroException>(() => new Natural(5UL) / new Natural());
    }

    // -------------------------------------------------------------------------
    // operator %
    // -------------------------------------------------------------------------

    [Fact]
    public void OperatorModulo_GivenNonExactDivision_ReturnsRemainder()
    {
        var result = new Natural(13UL) % new Natural(4UL);

        Assert.Equal(new Natural(1UL), result);
    }

    [Fact]
    public void OperatorModulo_GivenExactDivision_ReturnsZero()
    {
        var result = new Natural(12UL) % new Natural(4UL);

        Assert.Equal(new Natural(), result);
    }

    [Fact]
    public void OperatorModulo_GivenDividendSmallerThanDivisor_ReturnsDividend()
    {
        var result = new Natural(3UL) % new Natural(10UL);

        Assert.Equal(new Natural(3UL), result);
    }

    // -------------------------------------------------------------------------
    // Large numbers (within ulong range — multi-digit stress tests)
    // -------------------------------------------------------------------------

    [Fact]
    public void DivRem_GivenUlongMaxValueDividend_ProducesCorrectResult()
    {
        // 18446744073709551615 / 7 = 2635249153387078802 remainder 1
        // (2635249153387078802 * 7 = 18446744073709551614; 18446744073709551615 - 18446744073709551614 = 1)
        var dividend = new Natural(ulong.MaxValue);
        var divisor  = new Natural(7UL);

        var quotient = Natural.DivRem(dividend, divisor, out var remainder);

        Assert.Equal(new Natural(2635249153387078802UL), quotient);
        Assert.Equal(new Natural(1UL), remainder);
    }

    [Fact]
    public void DivRem_GivenProductAsDividend_ProducesZeroRemainder()
    {
        // Verify that (A * B) / B = A with zero remainder for large A, B
        var a = new Natural(1000000000UL);
        var b = new Natural(999999937UL); // b is prime

        var product = a * b;  // 999999937000000000
        var quotient = Natural.DivRem(product, b, out var remainder);

        Assert.Equal(a, quotient);
        Assert.Equal(new Natural(), remainder);
    }
}
