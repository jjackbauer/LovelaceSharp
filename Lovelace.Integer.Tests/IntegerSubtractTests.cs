using Lovelace.Integer;

namespace Lovelace.Integer.Tests;

/// <summary>
/// Functional tests for <see cref="Integer.Subtract"/> and <c>operator-</c>.
/// Test plan items 34–39 from the requirements document.
/// </summary>
public class IntegerSubtractTests
{
    // -------------------------------------------------------------------------
    // Subtract — positive operands
    // -------------------------------------------------------------------------

    [Fact]
    public void Subtract_GivenPositiveFromLargerPositive_ReturnsPositive()
    {
        // 10 - 3 = 7
        var result = new Integer(10L) - new Integer(3L);
        Assert.Equal(new Integer(7L), result);
        Assert.True(Integer.IsPositive(result));
    }

    [Fact]
    public void Subtract_GivenPositiveFromSmallerPositive_ReturnsNegative()
    {
        // 3 - 10 = -7
        var result = new Integer(3L) - new Integer(10L);
        Assert.Equal(new Integer(-7L), result);
        Assert.True(Integer.IsNegative(result));
    }

    // -------------------------------------------------------------------------
    // Subtract — mixed signs
    // -------------------------------------------------------------------------

    [Fact]
    public void Subtract_GivenNegativeFromPositive_AddsAbsoluteValues()
    {
        // 3 - (-4) = 7  (subtracting a negative adds)
        var result = new Integer(3L) - new Integer(-4L);
        Assert.Equal(new Integer(7L), result);
        Assert.True(Integer.IsPositive(result));
    }

    [Fact]
    public void Subtract_GivenNegativeFromNegative_AccountsForDoubleNegative()
    {
        // -3 - (-4) = 1  (−3 + 4)
        var result = new Integer(-3L) - new Integer(-4L);
        Assert.Equal(new Integer(1L), result);
        Assert.True(Integer.IsPositive(result));
    }

    [Fact]
    public void Subtract_GivenPositiveFromNegative_ReturnsMoreNegative()
    {
        // -3 - 4 = -7
        var result = new Integer(-3L) - new Integer(4L);
        Assert.Equal(new Integer(-7L), result);
        Assert.True(Integer.IsNegative(result));
    }

    // -------------------------------------------------------------------------
    // Subtract — boundary / identity cases
    // -------------------------------------------------------------------------

    [Fact]
    public void Subtract_GivenEqualValues_ReturnsZero()
    {
        var a = new Integer(42L);
        Assert.True(Integer.IsZero(a - a));
    }

    [Fact]
    public void Subtract_GivenEqualNegativeValues_ReturnsZero()
    {
        var a = new Integer(-99L);
        Assert.True(Integer.IsZero(a - a));
    }

    [Fact]
    public void Subtract_GivenZeroSubtrahend_ReturnsOriginalValue()
    {
        // n - 0 = n
        var n = new Integer(17L);
        Assert.Equal(n, n - new Integer(0L));
    }

    [Fact]
    public void Subtract_GivenNegativeZeroSubtrahend_ReturnsOriginalValue()
    {
        // n - Integer(0) = n, even for negative n
        var n = new Integer(-17L);
        Assert.Equal(n, n - new Integer(0L));
    }

    [Fact]
    public void Subtract_GivenZeroMinuend_ReturnsNegatedSubtrahend()
    {
        // 0 - 5 = -5
        var result = new Integer(0L) - new Integer(5L);
        Assert.Equal(new Integer(-5L), result);
    }

    // -------------------------------------------------------------------------
    // operator- (binary)
    // -------------------------------------------------------------------------

    [Fact]
    public void SubtractOperator_IsEquivalentToSubtractMethod()
    {
        var a = new Integer(100L);
        var b = new Integer(37L);
        Assert.Equal(a.Subtract(b), a - b);
    }

    // -------------------------------------------------------------------------
    // Large values
    // -------------------------------------------------------------------------

    [Fact]
    public void Subtract_GivenLargeValues_ReturnsCorrectResult()
    {
        // 1_000_000_000 - 999_999_999 = 1
        var a = new Integer(1_000_000_000L);
        var b = new Integer(999_999_999L);
        Assert.Equal(new Integer(1L), a - b);
    }
}
