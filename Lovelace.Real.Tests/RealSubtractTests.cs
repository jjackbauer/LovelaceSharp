using Lovelace.Real;

namespace Lovelace.Real.Tests;

/// <summary>
/// Functional tests covering <see cref="Real.Subtract"/> / binary <c>operator-</c>.
///
/// Checklist item covered:
///   - static Real Subtract(Real left, Real right) / operator-
/// </summary>
public class RealSubtractTests
{
    [Fact]
    public void Subtract_GivenLargerMinusSmaller_ReturnsPositiveResult()
    {
        // 5.0 - 3.2 = 1.8   (same exponent -1,  50 - 32 = 18)
        Real result = new Real("5.0") - new Real("3.2");
        Assert.Equal(new Real("1.8"), result);
    }

    [Fact]
    public void Subtract_GivenValueMinusItself_ReturnsZero()
    {
        // a - a = 0 for any a
        Real a = new Real("5.123");
        Real result = a - a;
        Assert.True(Real.IsZero(result));
    }

    [Fact]
    public void Subtract_GivenSmallerMinusLarger_ReturnsNegativeResult()
    {
        // 1.0 - 2.5 = -1.5
        Real result = new Real("1.0") - new Real("2.5");
        Assert.Equal(new Real("-1.5"), result);
    }

    [Fact]
    public void Subtract_GivenDifferentExponents_AlignsBeforeSubtracting()
    {
        // 1.0 (exp -1) - 0.001 (exp -3) = 0.999
        Real result = new Real("1.0") - new Real("0.001");
        Assert.Equal(new Real("0.999"), result);
    }

    [Fact]
    public void Subtract_GivenZeroMinuend_ReturnsNegatedSubtrahend()
    {
        // 0 - 3.5 = -3.5
        Real result = new Real() - new Real("3.5");
        Assert.Equal(new Real("-3.5"), result);
    }

    [Fact]
    public void Subtract_GivenBothNegative_ReturnsCorrectResult()
    {
        // (-1.0) - (-2.5) = 1.5
        Real result = new Real("-1.0") - new Real("-2.5");
        Assert.Equal(new Real("1.5"), result);
    }

    [Fact]
    public void Subtract_ResultExponent_IsMinOfBothExponents()
    {
        // 1.5 (exp -1) - 0.001 (exp -3): resultExp = min(-1,-3) = -3
        Real left   = Real.Parse("1.5");
        Real right  = Real.Parse("0.001");
        Real result = left - right;
        Assert.Equal(-3L, result.Exponent);
        Assert.Equal(new Real("1.499"), result);
    }
}
