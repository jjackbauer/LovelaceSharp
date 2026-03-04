using Lovelace.Integer;

namespace Lovelace.Integer.Tests;

/// <summary>
/// Functional tests for Integer.Increment, Integer.Decrement, operator++, operator--.
/// ChecklistItem: Increment() / Decrement() / operator++ / operator--
/// </summary>
public class IntegerIncrementTests
{
    [Fact]
    public void Increment_GivenPositiveValue_IncrementsByOne()
    {
        var result = new Integer(5L).Increment();
        Assert.Equal(new Integer(6L), result);
    }

    [Fact]
    public void Increment_GivenNegativeValue_MovesTowardsZero()
    {
        var result = new Integer(-1L).Increment();
        Assert.True(Integer.IsZero(result));
    }

    [Fact]
    public void Increment_GivenZero_ReturnsOne()
    {
        var result = new Integer(0L).Increment();
        Assert.Equal(new Integer(1L), result);
    }

    [Fact]
    public void Decrement_GivenPositiveValue_DecrementsByOne()
    {
        var result = new Integer(5L).Decrement();
        Assert.Equal(new Integer(4L), result);
    }

    [Fact]
    public void Decrement_GivenZero_ReturnsNegativeOne()
    {
        var result = new Integer(0L).Decrement();
        Assert.Equal(new Integer(-1L), result);
    }

    [Fact]
    public void Decrement_GivenNegativeValue_DecrementsFurtherNegative()
    {
        var result = new Integer(-2L).Decrement();
        Assert.Equal(new Integer(-3L), result);
    }

    [Fact]
    public void PreIncrementOperator_MutatesAndReturnsNewValue()
    {
        var x = new Integer(5L);
        var y = ++x;
        Assert.Equal(new Integer(6L), x);
        Assert.Equal(new Integer(6L), y);
    }

    [Fact]
    public void PreDecrementOperator_MutatesAndReturnsNewValue()
    {
        var x = new Integer(5L);
        var y = --x;
        Assert.Equal(new Integer(4L), x);
        Assert.Equal(new Integer(4L), y);
    }
}
