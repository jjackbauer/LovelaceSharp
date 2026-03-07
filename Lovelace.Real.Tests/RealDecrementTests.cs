using Lovelace.Real;

namespace Lovelace.Real.Tests;

/// <summary>
/// Functional tests for <c>operator--</c> (<see cref="Real"/>).
/// Implemented as <c>value - Real.One</c>.
///
/// Checklist item:
///   - operator-- — implement as value - Real.One [IDecrementOperators&lt;Real&gt;]
/// </summary>
public class RealDecrementTests
{
    [Fact]
    public void Decrement_GivenPositiveInteger_ReturnsPreviousInteger()
    {
        var r = Real.Parse("5");
        var result = --r;
        Assert.Equal(Real.Parse("4"), result);
    }

    [Fact]
    public void Decrement_GivenNegativeInteger_ReturnsValueMinusOne()
    {
        var r = Real.Parse("-5");
        var result = --r;
        Assert.Equal(Real.Parse("-6"), result);
    }

    [Fact]
    public void Decrement_GivenZero_ReturnsNegativeOne()
    {
        var r = Real.Zero;
        var result = --r;
        Assert.Equal(Real.NegativeOne, result);
    }

    [Fact]
    public void Decrement_GivenOne_ReturnsZero()
    {
        var r = Real.One;
        var result = --r;
        Assert.True(Real.IsZero(result));
    }

    [Fact]
    public void Decrement_GivenDecimalValue_ReturnsValueMinusOne()
    {
        var r = Real.Parse("2.5");
        var result = --r;
        Assert.Equal(Real.Parse("1.5"), result);
    }
}
