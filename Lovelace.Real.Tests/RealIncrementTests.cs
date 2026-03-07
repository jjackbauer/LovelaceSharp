using Lovelace.Real;

namespace Lovelace.Real.Tests;

/// <summary>
/// Functional tests for <c>operator++</c> (<see cref="Real"/>).
/// Implemented as <c>value + Real.One</c>.
///
/// Checklist item:
///   - operator++ — implement as value + Real.One [IIncrementOperators&lt;Real&gt;]
/// </summary>
public class RealIncrementTests
{
    [Fact]
    public void Increment_GivenPositiveInteger_ReturnsNextInteger()
    {
        var r = Real.Parse("5");
        var result = ++r;
        Assert.Equal(Real.Parse("6"), result);
    }

    [Fact]
    public void Increment_GivenNegativeInteger_ReturnsValuePlusOne()
    {
        var r = Real.Parse("-5");
        var result = ++r;
        Assert.Equal(Real.Parse("-4"), result);
    }

    [Fact]
    public void Increment_GivenZero_ReturnsOne()
    {
        var r = Real.Zero;
        var result = ++r;
        Assert.Equal(Real.One, result);
    }

    [Fact]
    public void Increment_GivenNegativeOne_ReturnsZero()
    {
        var r = Real.NegativeOne;
        var result = ++r;
        Assert.True(Real.IsZero(result));
    }

    [Fact]
    public void Increment_GivenDecimalValue_ReturnsValuePlusOne()
    {
        var r = Real.Parse("2.5");
        var result = ++r;
        Assert.Equal(Real.Parse("3.5"), result);
    }
}
