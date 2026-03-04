using Lovelace.Integer;

namespace Lovelace.Integer.Tests;

/// <summary>
/// Functional tests for Integer.Factorial.
/// ChecklistItem: Factorial()
/// </summary>
public class IntegerFactorialTests
{
    [Fact]
    public void Factorial_GivenPositiveInteger_ReturnsCorrectValue()
    {
        var result = new Integer(5L).Factorial();
        Assert.Equal(new Integer(120L), result);
    }

    [Fact]
    public void Factorial_GivenZero_ReturnsOne()
    {
        var result = new Integer(0L).Factorial();
        Assert.Equal(new Integer(1L), result);
    }

    [Fact]
    public void Factorial_GivenNegativeInteger_Throws()
    {
        Assert.Throws<InvalidOperationException>(() => new Integer(-1L).Factorial());
    }

    [Fact]
    public void Factorial_GivenOne_ReturnsOne()
    {
        var result = new Integer(1L).Factorial();
        Assert.Equal(new Integer(1L), result);
    }

    [Fact]
    public void Factorial_GivenTen_ReturnsCorrectValue()
    {
        var result = new Integer(10L).Factorial();
        Assert.Equal(new Integer(3628800L), result);
    }
}
