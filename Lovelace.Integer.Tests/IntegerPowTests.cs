using Lovelace.Integer;

namespace Lovelace.Integer.Tests;

/// <summary>
/// Functional tests for Integer.Pow.
/// ChecklistItem: Pow(Integer exponent)
/// </summary>
public class IntegerPowTests
{
    [Fact]
    public void Pow_GivenPositiveBaseAndPositiveExponent_ReturnsCorrectResult()
    {
        var result = new Integer(2L).Pow(new Integer(10L));
        Assert.Equal(new Integer(1024L), result);
    }

    [Fact]
    public void Pow_GivenNegativeBaseAndEvenExponent_ReturnsPositive()
    {
        var result = new Integer(-2L).Pow(new Integer(4L));
        Assert.Equal(new Integer(16L), result);
        Assert.True(Integer.IsPositive(result));
    }

    [Fact]
    public void Pow_GivenNegativeBaseAndOddExponent_ReturnsNegative()
    {
        var result = new Integer(-2L).Pow(new Integer(3L));
        Assert.Equal(new Integer(-8L), result);
        Assert.True(Integer.IsNegative(result));
    }

    [Fact]
    public void Pow_GivenZeroExponent_Throws()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new Integer(2L).Pow(new Integer(0L)));
    }

    [Fact]
    public void Pow_GivenNegativeExponent_Throws()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new Integer(2L).Pow(new Integer(-1L)));
    }

    [Fact]
    public void Pow_GivenZeroBase_Throws()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new Integer(0L).Pow(new Integer(3L)));
    }
}
