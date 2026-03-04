using Lovelace.Integer;

namespace Lovelace.Integer.Tests;

/// <summary>
/// Functional tests for Integer equality and comparison operators.
/// ChecklistItem: Equals / CompareTo / operator== / != / > / >= / < / <=
/// </summary>
public class IntegerEqualityTests
{
    // -------------------------------------------------------------------------
    // Equals / operator== / operator!=
    // -------------------------------------------------------------------------

    [Fact]
    public void Equals_GivenSameValue_ReturnsTrue()
    {
        Assert.Equal(new Integer(42L), new Integer(42L));
        Assert.True(new Integer(42L) == new Integer(42L));
    }

    [Fact]
    public void Equals_GivenSameMagnitudeDifferentSign_ReturnsFalse()
    {
        Assert.NotEqual(new Integer(42L), new Integer(-42L));
        Assert.True(new Integer(42L) != new Integer(-42L));
    }

    [Fact]
    public void Equals_GivenTwoZeros_ReturnsTrue()
    {
        var a = new Integer(0L);
        var b = new Integer(0L);
        Assert.Equal(a, b);
        Assert.True(a == b);
    }

    [Fact]
    public void Equals_GivenReferenceToSelf_ReturnsTrue()
    {
        var x = new Integer(7L);
        Assert.True(x.Equals(x));
    }

    // -------------------------------------------------------------------------
    // CompareTo
    // -------------------------------------------------------------------------

    [Fact]
    public void CompareTo_GivenPositiveGreaterThanNegative_ReturnsPositive()
    {
        Assert.True(new Integer(1L).CompareTo(new Integer(-1L)) > 0);
    }

    [Fact]
    public void CompareTo_GivenNegativeLessThanPositive_ReturnsNegative()
    {
        Assert.True(new Integer(-1L).CompareTo(new Integer(1L)) < 0);
    }

    [Fact]
    public void CompareTo_GivenEqualValues_ReturnsZero()
    {
        Assert.Equal(0, new Integer(5L).CompareTo(new Integer(5L)));
    }

    [Fact]
    public void CompareTo_GivenTwoNegatives_LargerMagnitudeIsSmaller()
    {
        // -10 < -3
        Assert.True(new Integer(-10L).CompareTo(new Integer(-3L)) < 0);
    }

    // -------------------------------------------------------------------------
    // Relational operators
    // -------------------------------------------------------------------------

    [Fact]
    public void GreaterThanOperator_GivenPositiveVsNegative_ReturnsTrue()
    {
        Assert.True(new Integer(1L) > new Integer(-1L));
    }

    [Fact]
    public void LessThanOperator_GivenSameSign_ComparesCorrectly()
    {
        Assert.True(new Integer(3L) < new Integer(5L));
    }

    [Fact]
    public void GreaterThanOrEqual_GivenEqualValues_ReturnsTrue()
    {
        Assert.True(new Integer(3L) >= new Integer(3L));
    }

    [Fact]
    public void LessThanOrEqual_GivenSmallerValue_ReturnsTrue()
    {
        Assert.True(new Integer(-5L) <= new Integer(-4L));
    }

    [Fact]
    public void LessThanOperator_GivenNegativeVsPositive_ReturnsTrue()
    {
        Assert.True(new Integer(-1L) < new Integer(1L));
    }

    [Fact]
    public void GreaterThanOrEqual_GivenLargerValue_ReturnsTrue()
    {
        Assert.True(new Integer(10L) >= new Integer(5L));
    }
}
