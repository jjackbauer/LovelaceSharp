using Lovelace.Real;

namespace Lovelace.Real.Tests;

/// <summary>
/// Functional tests covering <see cref="Real.Invert"/>.
///
/// Checklist item:
///   - Real Invert() — reciprocal 1 / this [depends on Divide]
/// </summary>
public class RealInvertTests
{
    // -------------------------------------------------------------------------
    // Non-periodic results
    // -------------------------------------------------------------------------

    [Fact]
    public void Invert_GivenTwo_ReturnsHalf()
    {
        // Real.One (mag=1, exp=0) / Real("2.0") (mag=20, exp=-1)
        // exponentAdjustment = 0 − (−1) = 1
        // long division: 1/20 → 0.05 ... wait, 1÷20: 0 integer, fracDigits: 10÷20=0 rem10, 100÷20=5 rem0 → "05"
        // resultExponent = −2 + 1 = −1  → "0.5"
        Real result = new Real("2.0").Invert();
        Assert.Equal(new Real("0.5"), result);
        Assert.False(result.IsPeriodic);
    }

    [Fact]
    public void Invert_GivenOne_ReturnsOne()
    {
        // Real.One / Real("1.0") (mag=10, exp=−1)
        // exponentAdjustment = 0 − (−1) = 1
        // long division: 1÷10 → quotient=0, fracDigit: 10÷10=1 rem=0 → fracStr="1"
        // allDigits="01" → Natural parses to mag=1; fracLen=1; resultExponent=−1+1=0 → "1"
        Real result = new Real("1.0").Invert();
        Assert.Equal(new Real("1"), result);
    }

    [Fact]
    public void Invert_GivenFraction_ReturnsCorrectResult()
    {
        // Real.One / Real("0.5") (mag=5, exp=−1)
        // exponentAdjustment = 0 − (−1) = 1
        // 1÷5: quotient=0, fraction: 10÷5=2 rem=0 → fracStr="2"
        // allDigits="02" → mag=2; fracLen=1; resultExponent=−1+1=0 → "2"
        Real result = new Real("0.5").Invert();
        Assert.Equal(new Real("2"), result);
        Assert.False(result.IsPeriodic);
    }

    // -------------------------------------------------------------------------
    // Negative reciprocal
    // -------------------------------------------------------------------------

    [Fact]
    public void Invert_GivenNegativeValue_ReturnsNegativeReciprocal()
    {
        // 1 / (−2.0) → result is negative
        Real result = new Real("-2.0").Invert();
        Assert.True(Real.IsNegative(result));
        Assert.Equal(new Real("-0.5"), result);
    }

    // -------------------------------------------------------------------------
    // Periodic reciprocal
    // -------------------------------------------------------------------------

    [Fact]
    public void Invert_GivenThree_ReturnsPeriodicOneThird()
    {
        // Real.One / Real.Parse("3") = 0.(3)
        Real result = Real.Parse("3").Invert();
        Assert.True(result.IsPeriodic);
        Assert.Equal(0L, result.PeriodStart);
        Assert.Equal(1L, result.PeriodLength);
        Assert.Equal(Real.Parse("0.(3)"), result);
    }

    // -------------------------------------------------------------------------
    // Division by zero
    // -------------------------------------------------------------------------

    [Fact]
    public void Invert_GivenZero_ThrowsException()
    {
        // 1 / 0 must throw DivideByZeroException
        Assert.Throws<DivideByZeroException>(() => new Real("0.0").Invert());
    }
}
