using System.Globalization;
using System.Numerics;
using Lovelace.Real;

namespace Lovelace.Real.Tests;

/// <summary>
/// PROPERTY (stated in the name of every test): for the exact rationals <c>a = p/q</c> and
/// <c>b = r/s</c> a <see cref="Real"/> can hold, <c>a + b</c> denotes EXACTLY
/// <c>(p·s + r·q)/(q·s)</c> and <c>a - b</c> denotes EXACTLY <c>(p·s - r·q)/(q·s)</c>,
/// in lowest terms.
///
/// Why this must hold: a <see cref="Real"/> is always an exact rational — a periodic value stores its
/// repeating block, a non-periodic value stores a finite decimal, so the sum is a rational too.
/// Adding through a fixed expansion of each periodic operand to
/// <see cref="Real.MaxComputationDecimalPlaces"/> fractional digits TRUNCATES the operand, which is
/// how <c>1/3 - 0.333…3</c> (1000 threes) came out exactly 0 while the true difference is
/// <c>1/(3·10^1000)</c>: the 1000-digit expansion of 1/3 IS the literal. The fix adds the exact
/// fractions and re-derives the period of the sum, so the identity holds for every operand the type
/// represents exactly.
///
/// Every test asserts the value twice and independently:
///   (a) on the <see cref="Real"/> itself (<c>==</c> / <c>CompareTo</c>), and
///   (b) on the exact rational the result denotes, reconstructed in the test from the PUBLIC value
///       state (magnitude string, Exponent, PeriodStart, PeriodLength) with <see cref="BigInteger"/> —
///       deliberately NOT via any production helper, so a bug in the production fraction conversion
///       cannot make the test agree with itself.
/// The EXPECTED value is likewise derived test-side from p, q, r and s, never from the result.
///
/// Precision is pinned to the documented default (1000 computation / 100 display places) with
/// <see cref="Real.WithPrecision"/>: the computation cap is process-global, and the property is a
/// statement about exact arithmetic at that cap, not about whatever cap a parallel test last set.
/// </summary>
public class RealAddPeriodicExactTests
{
    private const long PinnedComputationDecimalPlaces = 1000L;
    private const long PinnedDisplayDecimalPlaces = 100L;

    private static IDisposable PinnedPrecision() =>
        Real.WithPrecision(PinnedComputationDecimalPlaces, PinnedDisplayDecimalPlaces);

    private static Real Exact(string literal) => new Real(literal);

    private static Real ExactInt(int value) => new Real(value.ToString(CultureInfo.InvariantCulture));

    private static Real Quotient(int numerator, int denominator) =>
        ExactInt(numerator) / ExactInt(denominator);

    // -------------------------------------------------------------------------
    // Corpus
    // -------------------------------------------------------------------------

    /// <summary>
    /// Denominators whose reciprocal is PERIODIC (each has a prime factor other than 2 and 5) and
    /// whose period is far below the 1000-digit computation cap, so the operand stored is exactly
    /// p/q. Every left operand of the corpus is therefore periodic — the path under test.
    /// </summary>
    private static readonly int[] PeriodicDenominators = { 3, 6, 7, 9, 11, 12, 13, 17, 19, 21, 37, 99 };

    /// <summary>
    /// Right-hand denominators: the periodic ones above (periodic + periodic) plus terminating ones
    /// (2, 4, 25, 1000), so the corpus covers periodic + terminating too.
    /// </summary>
    private static readonly int[] RightDenominators = { 3, 7, 17, 99, 2, 4, 25, 1000 };

    private static readonly int[] LeftNumerators = { 1, 2, -5 };

    /// <summary>Negative numerators are included, so sign handling is part of the property.</summary>
    private static readonly int[] RightNumerators = { 1, -1, 5 };

    public static IEnumerable<object[]> ExactRationalPairCorpus()
    {
        foreach (int q in PeriodicDenominators)
            foreach (int p in LeftNumerators)
                foreach (int s in RightDenominators)
                    foreach (int r in RightNumerators)
                        yield return new object[] { p, q, r, s };
    }

    // -------------------------------------------------------------------------
    // The property
    // -------------------------------------------------------------------------

    /// <summary>
    /// PROPERTY: <c>(p/q) + (r/s)</c> denotes exactly <c>(p·s + r·q)/(q·s)</c>, for periodic and
    /// terminating right-hand operands and for negative numerators.
    /// </summary>
    [Theory]
    [MemberData(nameof(ExactRationalPairCorpus))]
    public void Add_ExactRationalPair_Property_DenotesTheExactRationalSum(int p, int q, int r, int s)
    {
        using var precision = PinnedPrecision();

        Real left  = Quotient(p, q);
        Real right = Quotient(r, s);

        Assert.True(left.IsPeriodic,
            $"corpus defect: {p}/{q} is not periodic, so the periodic additive path is not under test");

        Real sum = left + right;

        (BigInteger Num, BigInteger Den) expected = ToLowestTerms(
            new BigInteger(p) * s + new BigInteger(r) * q,
            new BigInteger(q) * s);
        (BigInteger Num, BigInteger Den) actual = RationalOf(sum);

        string detail = string.Create(CultureInfo.InvariantCulture,
            $"({p}/{q}) + ({r}/{s}) = {Describe(sum)}");

        Assert.True(expected == actual,
            $"the sum denotes {Fraction(actual)}, expected {Fraction(expected)}: {detail}");

        // (a) The Real-level identity against a Real built from the independently derived fraction.
        // A cross-path check (it goes through Divide); (b) above is the independent one.
        Assert.True(sum == FromFraction(expected),
            $"the sum does not compare equal to {Fraction(expected)}: {detail}");
    }

    /// <summary>
    /// PROPERTY: <c>(p/q) - (r/s)</c> denotes exactly <c>(p·s - r·q)/(q·s)</c>. Subtract is
    /// <c>Add(left, Negate(right))</c>, so this is the same defect site.
    /// </summary>
    [Theory]
    [MemberData(nameof(ExactRationalPairCorpus))]
    public void Subtract_ExactRationalPair_Property_DenotesTheExactRationalDifference(int p, int q, int r, int s)
    {
        using var precision = PinnedPrecision();

        Real left  = Quotient(p, q);
        Real right = Quotient(r, s);

        Assert.True(left.IsPeriodic,
            $"corpus defect: {p}/{q} is not periodic, so the periodic additive path is not under test");

        Real difference = left - right;

        (BigInteger Num, BigInteger Den) expected = ToLowestTerms(
            new BigInteger(p) * s - new BigInteger(r) * q,
            new BigInteger(q) * s);
        (BigInteger Num, BigInteger Den) actual = RationalOf(difference);

        string detail = string.Create(CultureInfo.InvariantCulture,
            $"({p}/{q}) - ({r}/{s}) = {Describe(difference)}");

        Assert.True(expected == actual,
            $"the difference denotes {Fraction(actual)}, expected {Fraction(expected)}: {detail}");

        Assert.True(difference == FromFraction(expected),
            $"the difference does not compare equal to {Fraction(expected)}: {detail}");
    }

    /// <summary>
    /// The cancellation case that a fixed-width expansion cannot see: a periodic operand minus a
    /// terminating literal whose digits ARE its expansion. <c>x - x</c> must be exactly 0 for a
    /// genuinely periodic <c>x</c>, and the corpus above already covers it through p/q - r/s.
    /// </summary>
    [Fact]
    public void Subtract_PeriodicOperandMinusItsOwnTruncation_IsTheTruncationRemainder()
    {
        using var precision = PinnedPrecision();

        Real third = Exact("1") / Exact("3");
        Assert.True(third.IsPeriodic);

        Real truncated = Exact("0." + new string('3', 1000));

        // Guard the probe itself: the literal must be 1000 threes and nothing else, otherwise the
        // difference is not the 1e-1000-scale remainder this test is about.
        Assert.Equal(Fraction(ToLowestTerms(BigInteger.Pow(10, 1000) - 1, 3 * BigInteger.Pow(10, 1000))),
                     Fraction(RationalOf(truncated)));

        Real difference = third - truncated;

        Assert.False(Real.IsZero(difference), "1/3 - 0.333...3 (1000 threes) must not be exactly zero");
        Assert.False(difference == Real.Zero, "1/3 - 0.333...3 (1000 threes) must not be exactly zero");

        // 1/3 - (10^1000 - 1)/(3*10^1000) = 1/(3*10^1000), the exact rational, in lowest terms.
        (BigInteger Num, BigInteger Den) expected =
            ToLowestTerms(BigInteger.One, 3 * BigInteger.Pow(10, 1000));
        (BigInteger Num, BigInteger Den) actual = RationalOf(difference);

        Assert.True(expected == actual,
            $"the difference denotes {Fraction(actual)}, expected {Fraction(expected)}");
        Assert.Equal(Fraction(expected), Fraction(actual));
        Assert.Equal("1", expected.Num.ToString(CultureInfo.InvariantCulture));
        Assert.Equal("3" + new string('0', 1000), expected.Den.ToString(CultureInfo.InvariantCulture));

        // Canonical shape: the period begins after the 1000 zeros the subtraction cancels.
        Assert.True(difference.IsPeriodic);
        Assert.Equal(1000L, difference.PeriodStart);
        Assert.Equal(1L, difference.PeriodLength);
        Assert.Equal(-1001L, difference.Exponent);

        // Cross-check by an independent route: 3 * (1/3 - L) is exactly 10^-1000, which the literal
        // spelling of 10^-1000 (a 1 at the 1000th fractional place) must also denote.
        Real scaled = ExactInt(3) * difference;
        Assert.Equal(Fraction(ToLowestTerms(BigInteger.One, BigInteger.Pow(10, 1000))),
                     Fraction(RationalOf(scaled)));
        Assert.True(scaled == Exact("0." + new string('0', 999) + "1"),
            $"3 * (1/3 - L) = {Describe(scaled)}, expected 10^-1000 exactly");
    }

    /// <summary>
    /// The boundary the exact additive path introduces: the sum is exact while its own period fits
    /// the computation cap, and the leading zeros a cancellation produces are carried by the
    /// exponent rather than charged to the digit budget. Each literal here is a different number of
    /// threes, and each difference is a different exact rational.
    /// </summary>
    [Theory]
    [InlineData(998)]
    [InlineData(999)]
    [InlineData(1000)]
    public void Subtract_OneThirdMinusNThrees_IsExactlyOneOverThreeTimesTenToTheN(int threes)
    {
        using var precision = PinnedPrecision();

        Real third = Exact("1") / Exact("3");
        Real literal = Exact("0." + new string('3', threes));

        Real difference = third - literal;

        (BigInteger Num, BigInteger Den) expected =
            ToLowestTerms(BigInteger.One, 3 * BigInteger.Pow(10, threes));
        (BigInteger Num, BigInteger Den) actual = RationalOf(difference);

        Assert.True(expected == actual,
            $"1/3 - 0.{threes} threes denotes {Fraction(actual)}, expected {Fraction(expected)}");
        Assert.Equal(1L, difference.PeriodLength);
        Assert.Equal((long)threes, difference.PeriodStart);
    }

    // -------------------------------------------------------------------------
    // Test-local exact-rational reconstruction (independent of production code)
    // -------------------------------------------------------------------------

    /// <summary>
    /// Reconstructs the exact rational denoted by a <see cref="Real"/> from its PUBLIC state,
    /// using <see cref="BigInteger"/> only:
    /// <list type="bullet">
    /// <item>non-periodic: <c>magnitude * 10^Exponent</c> (a finite decimal);</item>
    /// <item>periodic with period start <c>s</c> and length <c>p</c>: the stored magnitude is the
    /// integer part followed by the <c>s</c> non-repeating digits and one <c>p</c>-digit block, so
    /// with <c>block = magnitude mod 10^p</c> the value is
    /// <c>(magnitude*(10^p - 1) + block) / (10^(s+p) * (10^p - 1))</c> — the geometric series
    /// behind the repeating tail.</item>
    /// </list>
    /// </summary>
    private static (BigInteger Num, BigInteger Den) RationalOf(Real value)
    {
        BigInteger magnitude = BigInteger.Parse(value.ToNatural().ToString(), CultureInfo.InvariantCulture);
        BigInteger num;
        BigInteger den;

        if (!value.IsPeriodic)
        {
            num = magnitude;
            den = BigInteger.One;
            if (value.Exponent > 0L)
                num *= BigInteger.Pow(10, (int)value.Exponent);
            else if (value.Exponent < 0L)
                den = BigInteger.Pow(10, (int)(-value.Exponent));
        }
        else
        {
            long s = value.PeriodStart;
            long p = value.PeriodLength;
            if (value.Exponent != -(s + p))
                throw new InvalidOperationException(
                    $"periodic Real invariant violated: Exponent={value.Exponent}, PeriodStart={s}, PeriodLength={p}");

            BigInteger tenToP = BigInteger.Pow(10, (int)p);
            BigInteger block = magnitude % tenToP;
            num = magnitude * (tenToP - 1) + block;
            den = BigInteger.Pow(10, (int)(s + p)) * (tenToP - 1);
        }

        if (Real.IsNegative(value) && !num.IsZero)
            num = -num;
        return ToLowestTerms(num, den);
    }

    private static Real FromFraction((BigInteger Num, BigInteger Den) value) =>
        new Real(value.Num.ToString(CultureInfo.InvariantCulture))
        / new Real(value.Den.ToString(CultureInfo.InvariantCulture));

    private static (BigInteger Num, BigInteger Den) ToLowestTerms(BigInteger num, BigInteger den)
    {
        if (num.IsZero)
            return (BigInteger.Zero, BigInteger.One);
        if (den.Sign < 0)
        {
            num = -num;
            den = -den;
        }
        BigInteger g = BigInteger.GreatestCommonDivisor(num, den);
        if (g > BigInteger.One)
        {
            num /= g;
            den /= g;
        }
        return (num, den);
    }

    private static string Fraction((BigInteger Num, BigInteger Den) value) =>
        string.Create(CultureInfo.InvariantCulture, $"{value.Num}/{value.Den}");

    private static string Describe(Real value) =>
        $"'{value.ToString()}' (periodic={value.IsPeriodic}, exp={value.Exponent}, periodStart={value.PeriodStart}, periodLength={value.PeriodLength})";
}
