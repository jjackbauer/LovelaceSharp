using Lovelace.Real;

namespace Lovelace.Real.Tests;

/// <summary>
/// Functional tests for <c>operator%</c> (<see cref="Real"/>).
/// Semantics: <c>left - Truncate(left / right) * right</c> (truncated towards zero,
/// mirroring <see cref="System.Decimal"/> <c>%</c> semantics).
/// <c>Truncate</c> is tested indirectly as the prerequisite private helper.
///
/// Checklist items:
///   - private static Real Truncate(Real r) — strips fractional digits toward zero
///   - static Real operator%(Real, Real) — truncated remainder [IModulusOperators&lt;Real,Real,Real&gt;]
/// </summary>
public class RealModulusTests
{
    [Fact]
    public void Modulus_GivenTwoPositiveIntegers_ReturnsRemainder()
    {
        var result = Real.Parse("7") % Real.Parse("3");
        Assert.Equal(Real.Parse("1"), result);
    }

    [Fact]
    public void Modulus_GivenDividendLessThanDivisor_ReturnsDividend()
    {
        var result = Real.Parse("2") % Real.Parse("5");
        Assert.Equal(Real.Parse("2"), result);
    }

    [Fact]
    public void Modulus_GivenZeroDividend_ReturnsZero()
    {
        var result = Real.Parse("0") % Real.Parse("5");
        Assert.True(Real.IsZero(result));
    }

    [Fact]
    public void Modulus_GivenDivisorZero_ThrowsDivideByZeroException()
    {
        Assert.Throws<DivideByZeroException>(() => Real.Parse("5") % Real.Parse("0"));
    }

    [Fact]
    public void Modulus_GivenNegativeDividend_ReturnsNegativeRemainder()
    {
        // Truncated-towards-zero semantics: -7 / 3 = -2 (truncated), remainder = -7 - (-2)*3 = -1.
        var result = Real.Parse("-7") % Real.Parse("3");
        Assert.Equal(Real.Parse("-1"), result);
    }

    [Fact]
    public void Modulus_GivenNegativeDivisor_FollowsTruncatedSemantics()
    {
        // Truncated-towards-zero semantics: 7 / -3 = -2 (truncated), remainder = 7 - (-2)*(-3) = 1.
        var result = Real.Parse("7") % Real.Parse("-3");
        Assert.Equal(Real.Parse("1"), result);
    }

    [Fact]
    public void Modulus_GivenDecimalOperands_ReturnsCorrectRemainder()
    {
        // 2.5 / 1.2 = 2.0833... → Truncate = 2, remainder = 2.5 - 2*1.2 = 0.1.
        var result = Real.Parse("2.5") % Real.Parse("1.2");
        Assert.Equal(Real.Parse("0.1"), result);
    }

    [Fact]
    public void Modulus_GivenExactMultiple_ReturnsZero()
    {
        var result = Real.Parse("6") % Real.Parse("3");
        Assert.True(Real.IsZero(result));
    }
}
