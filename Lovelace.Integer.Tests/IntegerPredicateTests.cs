using Lovelace.Integer;

namespace Lovelace.Integer.Tests;

/// <summary>
/// Functional tests for Integer static predicates and the Sign property.
/// ChecklistItem: Static predicates (IsZero, IsPositive, IsNegative, IsEvenInteger,
/// IsOddInteger) and Sign property.
/// </summary>
public class IntegerPredicateTests
{
    // -------------------------------------------------------------------------
    // IsZero
    // -------------------------------------------------------------------------

    [Fact]
    public void IsZero_GivenDefaultInstance_ReturnsTrue()
    {
        var n = new Integer();
        Assert.True(Integer.IsZero(n));
    }

    [Fact]
    public void IsZero_GivenNonZeroValue_ReturnsFalse()
    {
        var n = new Integer(1L);
        Assert.False(Integer.IsZero(n));
    }

    [Fact]
    public void IsZero_GivenNegativeNonZeroValue_ReturnsFalse()
    {
        var n = new Integer(-5L);
        Assert.False(Integer.IsZero(n));
    }

    // -------------------------------------------------------------------------
    // IsPositive
    // -------------------------------------------------------------------------

    [Fact]
    public void IsPositive_GivenPositiveValue_ReturnsTrue()
    {
        var n = new Integer(5L);
        Assert.True(Integer.IsPositive(n));
    }

    [Fact]
    public void IsPositive_GivenNegativeValue_ReturnsFalse()
    {
        var n = new Integer(-5L);
        Assert.False(Integer.IsPositive(n));
    }

    [Fact]
    public void IsPositive_GivenZero_ReturnsTrue()
    {
        // Zero has no sign; IsPositive returns true for zero (not negative).
        var n = new Integer(0L);
        Assert.True(Integer.IsPositive(n));
    }

    // -------------------------------------------------------------------------
    // IsNegative
    // -------------------------------------------------------------------------

    [Fact]
    public void IsNegative_GivenNegativeValue_ReturnsTrue()
    {
        var n = new Integer(-1L);
        Assert.True(Integer.IsNegative(n));
    }

    [Fact]
    public void IsNegative_GivenPositiveValue_ReturnsFalse()
    {
        var n = new Integer(3L);
        Assert.False(Integer.IsNegative(n));
    }

    [Fact]
    public void IsNegative_GivenZero_ReturnsFalse()
    {
        // Zero is not negative (normalised sign).
        var n = new Integer(0L);
        Assert.False(Integer.IsNegative(n));
    }

    // -------------------------------------------------------------------------
    // IsEvenInteger
    // -------------------------------------------------------------------------

    [Fact]
    public void IsEvenInteger_GivenEvenPositiveValue_ReturnsTrue()
    {
        var n = new Integer(4L);
        Assert.True(Integer.IsEvenInteger(n));
    }

    [Fact]
    public void IsEvenInteger_GivenOddValue_ReturnsFalse()
    {
        var n = new Integer(3L);
        Assert.False(Integer.IsEvenInteger(n));
    }

    [Fact]
    public void IsEvenInteger_GivenEvenNegativeValue_ReturnsTrue()
    {
        // Sign does not affect parity; -4 is even.
        var n = new Integer(-4L);
        Assert.True(Integer.IsEvenInteger(n));
    }

    [Fact]
    public void IsEvenInteger_GivenZero_ReturnsTrue()
    {
        // 0 is even.
        var n = new Integer(0L);
        Assert.True(Integer.IsEvenInteger(n));
    }

    // -------------------------------------------------------------------------
    // IsOddInteger
    // -------------------------------------------------------------------------

    [Fact]
    public void IsOddInteger_GivenOddPositiveValue_ReturnsTrue()
    {
        var n = new Integer(7L);
        Assert.True(Integer.IsOddInteger(n));
    }

    [Fact]
    public void IsOddInteger_GivenOddNegativeValue_ReturnsTrue()
    {
        var n = new Integer(-3L);
        Assert.True(Integer.IsOddInteger(n));
    }

    [Fact]
    public void IsOddInteger_GivenEvenValue_ReturnsFalse()
    {
        var n = new Integer(6L);
        Assert.False(Integer.IsOddInteger(n));
    }

    [Fact]
    public void IsOddInteger_GivenZero_ReturnsFalse()
    {
        var n = new Integer(0L);
        Assert.False(Integer.IsOddInteger(n));
    }

    // -------------------------------------------------------------------------
    // Sign property
    // -------------------------------------------------------------------------

    [Fact]
    public void Sign_GivenPositive_ReturnsOne()
    {
        var n = new Integer(10L);
        Assert.Equal(1, n.Sign);
    }

    [Fact]
    public void Sign_GivenNegative_ReturnsMinusOne()
    {
        var n = new Integer(-10L);
        Assert.Equal(-1, n.Sign);
    }

    [Fact]
    public void Sign_GivenZero_ReturnsZero()
    {
        var n = new Integer(0L);
        Assert.Equal(0, n.Sign);
    }
}
