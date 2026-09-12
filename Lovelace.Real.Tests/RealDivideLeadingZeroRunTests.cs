using Lovelace.Real;

using Nat = Lovelace.Natural.Natural;

namespace Lovelace.Real.Tests;

/// <summary>
/// P-B3 / O-B10, first half: <see cref="Real.Divide"/> answered <c>0</c> for a nonzero quotient
/// whose leading fractional zeros outnumbered the digit budget.
/// <para>
/// The defect (observed at the round's pre-fix tree): <c>1/(3*10^1000)</c> →
/// <c>{"kind":"Real","value":"0","exact":false}</c> and <c>evalf(1/(3*10^1000), 30)</c> →
/// <c>{"kind":"Integer","value":"0","exact":true}</c>.  The neighbour inside the budget was
/// already right — <c>1/(3*10^100)</c> is the exact periodic <c>0.(3)</c> at scale 10^-100 with
/// numerator 1 / denominator 3·10^100 — so the loss begins where the quotient's first
/// significant digit lies further right than the digits the division accumulates.
/// </para>
/// <para>
/// A Real is digits × 10^Exponent plus an optional period, so the run of zeros is SCALE and not
/// a digit: the quotient's decimal point belongs in the result exponent, and the digit budget
/// must pay only for the digits that carry information.
/// </para>
/// <para>
/// Independent ground truth — mpmath 1.3.0 / SymPy 1.14.0, Python 3.12.14, 60–80 dps
/// (script <c>%T%\pb3_gt.py</c>, <c>%T% = C:\Users\ricar\AppData\Local\Temp\lov6</c>):
/// <c>1/(3*10^99) = 3.33333333333333333333333333333333333333333333e-100</c>,
/// <c>1/(3*10^100) = 3.33…e-101</c>,
/// <c>1/(3*10^1000) = 3.33…e-1001</c>,
/// <c>1/(7*10^5000) = 1.42857142857142857142857142857142857142857143e-5001</c>,
/// <c>1/(1009*10^1000) = 9.91080277502477700693756194252e-1004</c> with
/// <c>ord_1009(10) = 252</c> (so a 20-digit budget genuinely truncates it).
/// </para>
/// </summary>
public class RealDivideLeadingZeroRunTests
{
    /// <summary>Builds c·10^<paramref name="zeroRun"/> as the decimal literal it is.</summary>
    private static Real TimesPowerOfTen(string coefficient, int zeroRun) =>
        Real.Parse(coefficient + new string('0', zeroRun));

    // -------------------------------------------------------------------------
    // The quotient keeps its value and its scale when the zero run exceeds the budget
    // -------------------------------------------------------------------------

    [Theory]
    [InlineData("3", 99, "3")]        // 1/(3*10^99)   = 3.33…e-100   (inside the budget: already right)
    [InlineData("3", 100, "3")]       // 1/(3*10^100)  = 3.33…e-101   (inside the budget: already right)
    [InlineData("3", 1000, "3")]      // 1/(3*10^1000) = 3.33…e-1001  (the reported defect)
    [InlineData("7", 5000, "142857")] // 1/(7*10^5000) = 1.428571…e-5001
    public void Divide_GivenLeadingZeroRunLongerThanTheBudget_KeepsTheValueAndItsScale(
        string coefficient, int zeroRun, string period)
    {
        // The ambient budget of the probe: 1000 accumulated digits, 100 displayed.
        using var precision = Real.WithPrecision(1000, 100);

        Real quotient = Real.Parse("1") / TimesPowerOfTen(coefficient, zeroRun);

        // The first significant digit and the scale it sits at.  mpmath puts the digit block
        // "period" one place right of 10^-(zeroRun + period.Length) for both coefficients above:
        // 1/(c*10^n) = 0.(n zeros)(period).
        Assert.Equal(Nat.Parse(period, null), quotient.ToNatural());
        Assert.Equal(-(zeroRun + period.Length), quotient.Exponent);

        // The repeating block starts at the first stored fractional digit — the whole zero run is
        // carried by the exponent, so PeriodStart names the position the magnitude really stores.
        Assert.Equal((long)zeroRun, quotient.PeriodStart);
        Assert.Equal((long)period.Length, quotient.PeriodLength);

        // Exactly representable (one period block at a carried scale) ⇒ honest and exact.
        Assert.True(quotient.IsExact);

        // The value as it renders, and as an independent parse of the mpmath ground-truth shape.
        string expected = "0." + new string('0', zeroRun) + "(" + period + ")";
        Assert.Equal(expected, quotient.ToString());
        Assert.Equal(Real.Parse(expected), quotient);
    }

    // -------------------------------------------------------------------------
    // A genuine truncation stays inexact and keeps the scale it did compute
    // -------------------------------------------------------------------------

    [Fact]
    public void Divide_GivenQuotientTheBudgetCannotHold_IsInexactAndKeepsItsScale()
    {
        // 1/(1009*10^1000): the first significant digit is 1003 places right of the point, and the
        // period of 1/1009 is 252 digits (ord_1009(10) = 252), so a 20-digit budget truncates the
        // quotient rather than representing it.  Ground truth: 9.9108027750247770069…e-1004 — the
        // 20 stored digits below are that value truncated, not rounded.
        using var precision = Real.WithPrecision(20, 40);

        Real quotient = Real.Parse("1") / TimesPowerOfTen("1009", 1000);

        Assert.False(Real.IsZero(quotient));
        Assert.False(quotient.IsExact);                                    // a truncation says so
        Assert.Equal(Nat.Parse("99108027750247770069", null), quotient.ToNatural());
        Assert.Equal(-(1003L + 20L), quotient.Exponent);                   // 1003 zeros carried by the scale
        Assert.Equal(0L, quotient.PeriodStart);
        Assert.Equal(0L, quotient.PeriodLength);
        Assert.Equal("0." + new string('0', 1003) + "99108027750247770069", quotient.ToString());
    }

    // -------------------------------------------------------------------------
    // The neighbours that already worked keep working
    // -------------------------------------------------------------------------

    [Fact]
    public void Divide_GivenQuotientInsideTheBudget_StillReportsTheExactPeriod()
    {
        using var precision = Real.WithPrecision(1000, 100);

        // 1/(3*10^100) = 1/3 · 10^-100 — the same shape as the defect, one budget-width closer.
        Real quotient = Real.Parse("1") / TimesPowerOfTen("3", 100);

        Assert.True(quotient.IsExact);
        Assert.Equal(new Nat(3UL), quotient.ToNatural());
        Assert.Equal(-101L, quotient.Exponent);
        Assert.Equal(100L, quotient.PeriodStart);
        Assert.Equal(1L, quotient.PeriodLength);
    }
}
