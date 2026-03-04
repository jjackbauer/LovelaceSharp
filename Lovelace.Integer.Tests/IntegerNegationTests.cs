using Lovelace.Integer;

namespace Lovelace.Integer.Tests;

/// <summary>
/// Functional tests for Integer negation: Negate(), operator-(unary), operator+(unary).
/// ChecklistItem: Negation — Negate(), operator-(Integer), operator+(Integer)
/// </summary>
public class IntegerNegationTests
{
    // -------------------------------------------------------------------------
    // Negate()
    // -------------------------------------------------------------------------

    [Fact]
    public void Negate_GivenPositiveValue_ReturnsNegative()
    {
        var n = new Integer(5L);
        var result = n.Negate();
        Assert.True(Integer.IsNegative(result));
        Assert.Equal("-5", result.ToString());
    }

    [Fact]
    public void Negate_GivenNegativeValue_ReturnsPositive()
    {
        var n = new Integer(-5L);
        var result = n.Negate();
        Assert.True(Integer.IsPositive(result));
        Assert.Equal("5", result.ToString());
    }

    [Fact]
    public void Negate_GivenZero_ReturnsZeroAndPositive()
    {
        var n = new Integer(0L);
        var result = n.Negate();
        Assert.True(Integer.IsZero(result));
        Assert.False(Integer.IsNegative(result));
    }

    [Fact]
    public void Negate_IsItsOwnInverse()
    {
        var n = new Integer(42L);
        Assert.Equal(n, n.Negate().Negate());

        var m = new Integer(-99L);
        Assert.Equal(m, m.Negate().Negate());
    }

    // -------------------------------------------------------------------------
    // operator- (unary)
    // -------------------------------------------------------------------------

    [Fact]
    public void UnaryMinus_GivenPositive_NegatesCorrectly()
    {
        var n = new Integer(3L);
        var result = -n;
        Assert.Equal(new Integer(-3L), result);
    }

    [Fact]
    public void UnaryMinus_GivenNegative_ReturnsPositive()
    {
        var n = new Integer(-7L);
        var result = -n;
        Assert.Equal(new Integer(7L), result);
    }

    [Fact]
    public void UnaryMinus_GivenZero_ReturnsZero()
    {
        var n = new Integer(0L);
        var result = -n;
        Assert.True(Integer.IsZero(result));
        Assert.False(Integer.IsNegative(result));
    }

    // -------------------------------------------------------------------------
    // operator+ (unary)
    // -------------------------------------------------------------------------

    [Fact]
    public void UnaryPlus_GivenPositiveValue_ReturnsCopyWithSameSign()
    {
        var n = new Integer(5L);
        var result = +n;
        Assert.Equal(new Integer(5L), result);
        Assert.True(Integer.IsPositive(result));
    }

    [Fact]
    public void UnaryPlus_GivenNegativeValue_ReturnsCopyWithSameSign()
    {
        var n = new Integer(-5L);
        var result = +n;
        Assert.Equal(new Integer(-5L), result);
        Assert.True(Integer.IsNegative(result));
    }

    [Fact]
    public void UnaryPlus_GivenZero_ReturnsZero()
    {
        var n = new Integer(0L);
        var result = +n;
        Assert.True(Integer.IsZero(result));
    }
}
