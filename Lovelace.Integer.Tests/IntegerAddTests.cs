using Lovelace.Integer;

namespace Lovelace.Integer.Tests;

/// <summary>
/// Functional tests for Integer.Add and operator+.
/// ChecklistItem: Add(Integer) / operator+(Integer, Integer)
/// </summary>
public class IntegerAddTests
{
    // -------------------------------------------------------------------------
    // Add — same-sign: add magnitudes, keep sign
    // -------------------------------------------------------------------------

    [Fact]
    public void Add_GivenTwoPositiveIntegers_ReturnsCorrectSum()
    {
        var a = new Integer(3L);
        var b = new Integer(4L);
        var result = a.Add(b);
        Assert.Equal(new Integer(7L), result);
    }

    [Fact]
    public void Add_GivenTwoNegativeIntegers_ReturnsNegativeSum()
    {
        var a = new Integer(-3L);
        var b = new Integer(-4L);
        var result = a.Add(b);
        Assert.Equal(new Integer(-7L), result);
        Assert.True(Integer.IsNegative(result));
    }

    // -------------------------------------------------------------------------
    // Add — different-sign: subtract magnitudes, sign follows the larger
    // -------------------------------------------------------------------------

    [Fact]
    public void Add_GivenPositiveAndNegativeWherePositiveLarger_ReturnsPositiveDifference()
    {
        var a = new Integer(10L);
        var b = new Integer(-3L);
        var result = a.Add(b);
        Assert.Equal(new Integer(7L), result);
        Assert.True(Integer.IsPositive(result));
    }

    [Fact]
    public void Add_GivenPositiveAndNegativeWhereNegativeLarger_ReturnsNegativeDifference()
    {
        var a = new Integer(3L);
        var b = new Integer(-10L);
        var result = a.Add(b);
        Assert.Equal(new Integer(-7L), result);
        Assert.True(Integer.IsNegative(result));
    }

    [Fact]
    public void Add_GivenOppositeValues_ReturnsZero()
    {
        var a = new Integer(5L);
        var b = new Integer(-5L);
        var result = a.Add(b);
        Assert.True(Integer.IsZero(result));
        Assert.False(Integer.IsNegative(result));
    }

    // -------------------------------------------------------------------------
    // Add — identity: adding zero
    // -------------------------------------------------------------------------

    [Fact]
    public void Add_GivenZeroOperandOnRight_ReturnsOriginalValue()
    {
        var a = new Integer(42L);
        var zero = new Integer(0L);
        var result = a.Add(zero);
        Assert.Equal(new Integer(42L), result);
    }

    [Fact]
    public void Add_GivenZeroOperandOnLeft_ReturnsOriginalValue()
    {
        var zero = new Integer(0L);
        var b = new Integer(-17L);
        var result = zero.Add(b);
        Assert.Equal(new Integer(-17L), result);
    }

    // -------------------------------------------------------------------------
    // operator+ — delegates to Add
    // -------------------------------------------------------------------------

    [Fact]
    public void OperatorPlus_GivenTwoPositives_ReturnsCorrectSum()
    {
        var a = new Integer(100L);
        var b = new Integer(23L);
        Assert.Equal(new Integer(123L), a + b);
    }

    [Fact]
    public void OperatorPlus_GivenPositiveAndNegative_ReturnsCorrectDifference()
    {
        var a = new Integer(10L);
        var b = new Integer(-3L);
        Assert.Equal(new Integer(7L), a + b);
    }

    [Fact]
    public void OperatorPlus_GivenLargeValues_ReturnsCorrectSum()
    {
        // Verifies no overflow — values exceed long.MaxValue when summed.
        var a = new Integer("99999999999999999999");
        var b = new Integer("1");
        Assert.Equal(new Integer("100000000000000000000"), a + b);
    }
}
