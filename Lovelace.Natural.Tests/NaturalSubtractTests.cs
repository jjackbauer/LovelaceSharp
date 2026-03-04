using Lovelace.Natural;

namespace Lovelace.Natural.Tests;

/// <summary>
/// Functional tests for <see cref="Natural"/> subtraction:
/// <c>operator-</c> (<see cref="System.Numerics.ISubtractionOperators{T,T,T}"/>)
/// and decrement operators (<see cref="System.Numerics.IDecrementOperators{T}"/>).
/// Checklist items:
///   "static Natural Subtract(Natural left, Natural right) / operator-"
///   "operator-- (prefix &amp; postfix)"
/// </summary>
public class NaturalSubtractTests
{
    // ─────────────────────────────────────────────────────────────────────────
    // Subtract / operator-
    // ─────────────────────────────────────────────────────────────────────────

    [Fact]
    public void Subtract_GivenLargerMinusSmallerNumber_ReturnsCorrectDifference()
    {
        var left  = new Natural(10UL);
        var right = new Natural(3UL);
        var diff  = left - right;
        Assert.Equal("7", diff.ToString());
    }

    [Fact]
    public void Subtract_GivenEqualNumbers_ReturnsZero()
    {
        var left  = new Natural(5UL);
        var right = new Natural(5UL);
        var diff  = left - right;
        Assert.True(Natural.IsZero(diff));
    }

    [Fact]
    public void Subtract_GivenSubtractZero_ReturnsSelf()
    {
        var left  = new Natural(42UL);
        var right = new Natural(0UL);
        var diff  = left - right;
        Assert.Equal("42", diff.ToString());
    }

    [Theory]
    [InlineData(1000UL, 1UL,   "999")]
    [InlineData(1000UL, 100UL, "900")]
    [InlineData(1000UL, 999UL, "1")]
    public void Subtract_GivenBorrowPropagation_ProducesCorrectResult(ulong a, ulong b, string expected)
    {
        var diff = new Natural(a) - new Natural(b);
        Assert.Equal(expected, diff.ToString());
    }

    [Fact]
    public void Subtract_GivenSmallerMinusLarger_ThrowsInvalidOperationException()
    {
        var left  = new Natural(3UL);
        var right = new Natural(10UL);
        Assert.Throws<InvalidOperationException>(() => { var _ = left - right; });
    }

    [Fact]
    public void Subtract_GivenZeroMinusNonZero_ThrowsInvalidOperationException()
    {
        var left  = new Natural(0UL);
        var right = new Natural(1UL);
        Assert.Throws<InvalidOperationException>(() => { var _ = left - right; });
    }

    [Fact]
    public void Subtract_GivenLargeNumbers_ProducesCorrectResult()
    {
        // Both numbers are beyond ulong.MaxValue range — parsed from string once Parse is available.
        // Use construction-by-addition as a proxy to build large values.
        // 99999999999999999999 - 1 = 99999999999999999998
        // Build via repeated addition is too slow; use the ulong max known value instead.
        // ulong.MaxValue = 18446744073709551615
        // ulong.MaxValue - 1 = 18446744073709551614
        var a    = new Natural(ulong.MaxValue);
        var b    = new Natural(1UL);
        var diff = a - b;
        Assert.Equal("18446744073709551614", diff.ToString());
    }

    [Fact]
    public void Subtract_GivenResult_HasNoLeadingZeros()
    {
        // 100 - 99 = 1 (result must be "1", not "001")
        var diff = new Natural(100UL) - new Natural(99UL);
        Assert.Equal("1", diff.ToString());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // operator-- (prefix and postfix)
    // ─────────────────────────────────────────────────────────────────────────

    [Fact]
    public void DecrementPrefix_GivenOne_ReturnsZero()
    {
        var n = new Natural(1UL);
        var result = --n;
        Assert.True(Natural.IsZero(result));
        Assert.True(Natural.IsZero(n));
    }

    [Fact]
    public void DecrementPrefix_GivenTen_ReturnsNine()
    {
        var n = new Natural(10UL);
        var result = --n;
        Assert.Equal("9", result.ToString());
        Assert.Equal("9", n.ToString());
    }

    [Fact]
    public void DecrementPrefix_GivenLargeCarry_ProducesCorrectResult()
    {
        // 1000 - 1 = 999 (borrow propagates across three digits)
        var n = new Natural(1000UL);
        var result = --n;
        Assert.Equal("999", result.ToString());
    }

    [Fact]
    public void DecrementPostfix_GivenValue_ReturnsPreviousValue()
    {
        var n = new Natural(5UL);
        var prev = n--;
        Assert.Equal("5", prev.ToString());
        Assert.Equal("4", n.ToString());
    }

    [Fact]
    public void DecrementPrefix_GivenZero_ThrowsInvalidOperationException()
    {
        // Decrementing zero on ℕ₀ must throw because the result would be negative.
        var n = new Natural(0UL);
        Assert.Throws<InvalidOperationException>(() => { var _ = --n; });
    }
}
