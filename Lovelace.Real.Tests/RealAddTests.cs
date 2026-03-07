using Lovelace.Real;

namespace Lovelace.Real.Tests;

/// <summary>
/// Functional tests covering <see cref="Real.Add"/> / <c>operator+</c>
/// and the private helpers <c>GetDecimalDigit</c> and <c>ToInteger</c> (tested indirectly).
///
/// Checklist items covered:
///   - private byte GetDecimalDigit(long position)
///   - private Integer ToInteger(long zeros)
///   - static Real Add(Real left, Real right) / operator+
/// </summary>
public class RealAddTests
{
    // -------------------------------------------------------------------------
    // Non-periodic path
    // -------------------------------------------------------------------------

    [Fact]
    public void Add_GivenSameExponent_ReturnsCorrectSum()
    {
        // 1.5 + 2.3: both exponent -1, 15 + 23 = 38, result "3.8"
        Real result = new Real("1.5") + new Real("2.3");
        Assert.Equal(new Real("3.8"), result);
    }

    [Fact]
    public void Add_GivenDifferentExponents_AlignsAndReturnsCorrectSum()
    {
        // 1.5 (exp -1) + 0.25 (exp -2): shift 1.5 → 150, add 25 → 175, result "1.75"
        Real result = new Real("1.5") + new Real("0.25");
        Assert.Equal(new Real("1.75"), result);
    }

    [Fact]
    public void Add_GivenPositiveAndNegative_ReturnsCorrectResult()
    {
        // 3.0 + (-1.5) = 1.5
        Real result = new Real("3.0") + new Real("-1.5");
        Assert.Equal(new Real("1.5"), result);
    }

    [Fact]
    public void Add_GivenZeroAndValue_ReturnsValue()
    {
        // 0.0 + 7.77 = 7.77  (tests that zero operand is handled correctly)
        Real result = new Real("0.0") + new Real("7.77");
        Assert.Equal(new Real("7.77"), result);
    }

    [Fact]
    public void Add_GivenBothNegative_ReturnsSumWithNegativeSign()
    {
        // (-1.1) + (-2.2) = -3.3
        Real result = new Real("-1.1") + new Real("-2.2");
        Assert.Equal(new Real("-3.3"), result);
    }

    [Fact]
    public void Add_GivenResultExponent_IsMinOfBothExponents()
    {
        // 1.5 (exp -1) + 0.001 (exp -3): resultExp = min(-1, -3) = -3
        Real left   = Real.Parse("1.5");
        Real right  = Real.Parse("0.001");
        Real result = left + right;
        Assert.Equal(-3L, result.Exponent);
        Assert.Equal(new Real("1.501"), result);
    }

    [Fact]
    public void Add_GivenValueAndZeroOnRight_ReturnsLeftValue()
    {
        // 5.5 + 0 = 5.5  (zero on the right side)
        Real result = new Real("5.5") + new Real();
        Assert.Equal(new Real("5.5"), result);
    }

    [Fact]
    public void Add_GivenLargeCarry_PropagatesCorrectly()
    {
        // 0.9 + 0.1 = 1 (carry propagates into integer part; fractional zeros normalize away)
        Real result = Real.Parse("0.9") + Real.Parse("0.1");
        Assert.Equal(new Real("1"), result);
    }

    // -------------------------------------------------------------------------
    // Periodic path  (exercises GetDecimalDigit indirectly)
    // -------------------------------------------------------------------------

    [Fact]
    public void Add_GivenTwoPeriodicReals_DetectsPeriodInResult()
    {
        // 0.(3) + 0.(6) = 1/3 + 2/3 = 1
        // The digit-level sum is 0.999... which normalises to "1".
        Real left   = Real.Parse("0.(3)");
        Real right  = Real.Parse("0.(6)");
        Real result = left + right;
        Assert.Equal(new Real("1"), result);
        Assert.False(result.IsPeriodic);
    }

    [Fact]
    public void Add_GivenPeriodicAndNonPeriodic_ReturnsCorrectResult()
    {
        // 0.(3) + 0.1 = 1/3 + 1/10 = 13/30 = 0.4(3)
        Real left   = Real.Parse("0.(3)");
        Real right  = Real.Parse("0.1");
        Real result = left + right;
        Assert.Equal(Real.Parse("0.4(3)"), result);
        Assert.True(result.IsPeriodic);
        Assert.Equal(1L, result.PeriodStart);
        Assert.Equal(1L, result.PeriodLength);
    }

    [Fact]
    public void Add_GivenNegativePeriodicPlusPositive_ReturnsCorrectResult()
    {
        // -0.(3) + 1.0 = -1/3 + 1 = 2/3 = 0.(6)
        Real left   = Real.Parse("-0.(3)");
        Real right  = Real.Parse("1");
        Real result = left + right;
        Assert.Equal(Real.Parse("0.(6)"), result);
        Assert.True(result.IsPeriodic);
    }

    [Fact]
    public void Add_GivenPeriodic1Over6Plus1Over6_ReturnsOneThird()
    {
        // 0.1(6) + 0.1(6) = 1/6 + 1/6 = 1/3 = 0.(3)
        Real left   = Real.Parse("0.1(6)");
        Real right  = Real.Parse("0.1(6)");
        Real result = left + right;
        Assert.Equal(Real.Parse("0.(3)"), result);
        Assert.True(result.IsPeriodic);
        Assert.Equal(0L, result.PeriodStart);
        Assert.Equal(1L, result.PeriodLength);
    }
}
