using Lovelace.Real;

namespace Lovelace.Real.Tests;

/// <summary>
/// Functional tests covering <see cref="Real.Divide"/> / <c>operator/</c>.
///
/// Checklist item:
///   - static Real Divide(Real left, Real right) / operator/
/// </summary>
public class RealDivideTests
{
    // -------------------------------------------------------------------------
    // Exact (non-periodic) division
    // -------------------------------------------------------------------------

    [Fact]
    public void Divide_GivenExactDivision_ReturnsExactResult()
    {
        // 6.0 (mag=60, exp=-1) / 2.0 (mag=20, exp=-1)
        // 60 / 20 = 3 exactly, exponentAdjustment = 0 → result "3", IsPeriodic == false
        Real result = Real.Parse("6.0") / Real.Parse("2.0");
        Assert.False(result.IsPeriodic);
        Assert.Equal(new Real("3"), result);
    }

    // -------------------------------------------------------------------------
    // Repeating decimals — period detection
    // -------------------------------------------------------------------------

    [Fact]
    public void Divide_GivenOneOverThree_ReturnsPeriodicResult()
    {
        // 1 / 3 = 0.(3)
        Real result = Real.Parse("1") / Real.Parse("3");
        Assert.True(result.IsPeriodic);
        Assert.Equal(0L, result.PeriodStart);
        Assert.Equal(1L, result.PeriodLength);
        Assert.Equal(Real.Parse("0.(3)"), result);
    }

    [Fact]
    public void Divide_GivenOneOverSeven_ReturnsCorrectPeriod()
    {
        // 1 / 7 = 0.(142857)
        Real result = Real.Parse("1") / Real.Parse("7");
        Assert.Equal(6L, result.PeriodLength);
        Assert.Equal(Real.Parse("0.(142857)"), result);
    }

    [Fact]
    public void Divide_GivenOneOverSix_ReturnsMixedPeriod()
    {
        // 1 / 6 = 0.1(6) — one non-repeating fractional digit then the period
        Real result = Real.Parse("1") / Real.Parse("6");
        Assert.Equal(1L, result.PeriodStart);
        Assert.Equal(1L, result.PeriodLength);
        Assert.Equal(Real.Parse("0.1(6)"), result);
    }

    // -------------------------------------------------------------------------
    // Signs
    // -------------------------------------------------------------------------

    [Fact]
    public void Divide_GivenNegativeDividend_ReturnsNegativeQuotient()
    {
        // -6.0 / 2.0 = -3
        Real result = Real.Parse("-6.0") / Real.Parse("2.0");
        Assert.True(Real.IsNegative(result));
        Assert.Equal(new Real("-3"), result);
    }

    [Fact]
    public void Divide_GivenNegativeDivisor_ReturnsNegativeQuotient()
    {
        // 6.0 / -2.0 = -3
        Real result = Real.Parse("6.0") / Real.Parse("-2.0");
        Assert.True(Real.IsNegative(result));
        Assert.Equal(new Real("-3"), result);
    }

    [Fact]
    public void Divide_GivenBothNegative_ReturnsPositiveQuotient()
    {
        // -6.0 / -2.0 = 3
        Real result = Real.Parse("-6.0") / Real.Parse("-2.0");
        Assert.False(Real.IsNegative(result));
        Assert.Equal(new Real("3"), result);
    }

    // -------------------------------------------------------------------------
    // Division by zero
    // -------------------------------------------------------------------------

    [Fact]
    public void Divide_GivenDivisorZero_ThrowsDivideByZeroException()
    {
        Assert.Throws<DivideByZeroException>(() => Real.Parse("1.0") / Real.Parse("0.0"));
    }

    // -------------------------------------------------------------------------
    // Negative periodic result
    // -------------------------------------------------------------------------

    [Fact]
    public void Divide_GivenNegativeOneOverThree_IsNegativeAndPeriodic()
    {
        // -1 / 3 = -0.(3)
        Real result = Real.Parse("-1") / Real.Parse("3");
        Assert.True(Real.IsNegative(result));
        Assert.True(result.IsPeriodic);
        Assert.Equal(Real.Parse("-0.(3)"), result);
    }

    // -------------------------------------------------------------------------
    // Periodic operands — Option C: periodic guard (Divide_GivenPeriodic*)
    // -------------------------------------------------------------------------

    [Fact]
    public void Divide_GivenPeriodicDividendByNonPeriodic_ReturnsCorrectQuotient()
    {
        // 0.(3) = 1/3; 1/3 ÷ 3 = 1/9 = 0.(1)
        // Without the periodic guard, ToNatural() gives Nat("3") and Exponent=-1,
        // so the raw division 3÷3=1 with exponent adjustment -1 produces 0.1 (= 1/10), not 0.(1).
        Real result = Real.Parse("0.(3)") / Real.Parse("3");
        Assert.Equal(Real.Parse("0.(1)"), result);
    }

    [Fact]
    public void Divide_GivenNonPeriodicByPeriodic_ExponentCompensatedCorrectly()
    {
        // 1 ÷ 0.(1) = 1 ÷ (1/9) = 9
        // Without the guard, ToNatural() gives Nat("1") for 0.(1) and Exponent=-1, so
        // raw 1÷1=1 with exponent adjustment +1 yields 10, not 9.
        Real result = Real.Parse("1") / Real.Parse("0.(1)");
        Assert.Equal(Real.Parse("9"), result);
    }

    [Fact]
    public void Divide_GivenNonPeriodicByPeriodic_PreservesCorrectness()
    {
        // 1 ÷ 0.(3) = 1 ÷ (1/3) = 3  — result must be non-periodic
        // Without the guard, the remainder loop finds a spurious period of length 1,
        // yielding Real(Nat("3"), IsPeriodic=true) which is not equal to Real("3").
        Real result = Real.Parse("1") / Real.Parse("0.(3)");
        Assert.Equal(Real.Parse("3"), result);
        Assert.False(result.IsPeriodic);
    }
}
