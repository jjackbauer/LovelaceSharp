using Lovelace.Real;
using Xunit;

using Int = Lovelace.Integer.Integer;

namespace Lovelace.Real.Tests;

/// <summary>
/// A quotient of two exact values whose decimal expansion TERMINATES is an exact value, whatever
/// the digit budget in force.  These are properties over the family of terminating denominators
/// (<c>2^a·5^b</c>) and over a range of budgets, not a list of pinned strings: the assertion is
/// always the algebraic identity <c>(a / b) · b == a</c>, which can only hold if the quotient the
/// tower returned is the true one.
///
/// <para>Regression: the fractional-digit loop of <see cref="Real.Divide(Real, Real)"/> charged the
/// quotient's LEADING ZEROS to <see cref="Real.MaxComputationDecimalPlaces"/>, so <c>1/2^100000</c>
/// filled the whole budget with zeros and came back as <c>0</c> — an exact operation returning a
/// wrong value — and any terminating expansion longer than the budget was truncated although it is
/// decidable exactly (<c>den = 2^a·5^b</c> implies <c>num/den = num·5^(a−b)/10^a</c>).</para>
/// </summary>
public class RealExactQuotientPropertyTests
{
    /// <summary>A small deterministic generator (no floating point, no framework randomness), so a
    /// failure names the same case on every run.</summary>
    private sealed class Lcg
    {
        private ulong _state = 0x2545F4914F6CDD1DUL;
        public int Next(int lowInclusive, int highExclusive)
        {
            _state = _state * 6364136223846793005UL + 1442695040888963407UL;
            return lowInclusive + (int)((_state >> 33) % (ulong)(highExclusive - lowInclusive));
        }
    }

    /// <summary>The exact integer 2^a·5^b — a denominator whose decimal expansion terminates.</summary>
    private static Int TerminatingDenominator(int twos, int fives)
    {
        Int two = new Int(2L);
        Int five = new Int(5L);
        Int denominator = Int.One;
        if (twos > 0) denominator = denominator * two.Pow(new Int(twos));
        if (fives > 0) denominator = denominator * five.Pow(new Int(fives));
        return denominator;
    }

    /// <summary>Every terminating quotient is exact at every budget: (n/d)·d must be n again.</summary>
    [Theory]
    [InlineData(1L)]
    [InlineData(2L)]
    [InlineData(5L)]
    [InlineData(18L)]
    [InlineData(30L)]
    [InlineData(37L)]
    [InlineData(100L)]
    public void Divide_TerminatingDenominator_ReturnsTheExactQuotient(long precision)
    {
        var rng = new Lcg();

        using var scope = Real.WithPrecision(precision, precision);

        for (int i = 0; i < 60; i++)
        {
            int twos = rng.Next(0, 95);
            int fives = rng.Next(0, 95);
            Int denominator = TerminatingDenominator(twos, fives);
            Int numerator = new Int((long)rng.Next(1, 1_000_000));

            Real quotient = Real.Divide(new Real(numerator), new Real(denominator));

            Assert.True(quotient * new Real(denominator) == new Real(numerator),
                $"({numerator}) / (2^{twos}·5^{fives}) at {precision} places did not multiply back: " +
                $"quotient = {(quotient.ToNatural().ToString().Length > 60 ? quotient.ToNatural().ToString()[..60] + "..." : quotient.ToString())}");
        }
    }

    /// <summary>The same quotient computed under a tight budget and under a generous one must be the
    /// same number: a terminating expansion has no budget-dependent digits.</summary>
    [Fact]
    public void Divide_TerminatingQuotient_DoesNotDependOnTheDigitBudget()
    {
        (long Numerator, int Twos, int Fives)[] cases =
        {
            (1L, 1, 0), (1L, 3, 0), (1L, 0, 1), (1L, 1, 1), (7L, 4, 2), (3L, 13, 7),
            (1L, 40, 0), (1L, 0, 40), (5L, 20, 20), (1L, 200, 0), (123456789L, 33, 17),
        };

        foreach ((long numerator, int twos, int fives) in cases)
        {
            Int denominator = TerminatingDenominator(twos, fives);
            Real tight, generous;

            using (Real.WithPrecision(1, 1))
                tight = Real.Divide(new Real(new Int(numerator)), new Real(denominator));

            using (Real.WithPrecision(1000, 1000))
                generous = Real.Divide(new Real(new Int(numerator)), new Real(denominator));

            Assert.True(tight == generous,
                $"{numerator}/(2^{twos}·5^{fives}) changed with the budget: " +
                $"{tight.ToNatural().ToString()}e{tight.Exponent} vs {generous.ToNatural().ToString()}e{generous.Exponent}");
        }
    }

    /// <summary>A denominator whose expansion terminates but whose digits run past the DEFAULT
    /// budget (the shape that used to return 0) is still exact: 1/2^4000 multiplies back.</summary>
    [Fact]
    public void Divide_TerminatingExpansionLongerThanTheDefaultBudget_IsExact()
    {
        Int denominator = new Int(2L).Pow(new Int(4000));

        Real quotient = Real.Divide(Real.One, new Real(denominator));

        Assert.True(quotient * new Real(denominator) == Real.One,
            "1/2^4000 (1205 fractional digits, past the 1000-place default) did not multiply back");
    }
}
