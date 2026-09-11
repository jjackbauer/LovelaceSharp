using System.Globalization;
using System.Numerics;
using Lovelace.Real;

namespace Lovelace.Real.Tests;

/// <summary>
/// PROPERTY (stated in the name of every test): <c>(p / q) * q == p</c> exactly, for every exact
/// rational dividend <c>p</c> and every exact decimal divisor <c>q</c> — integer divisors whose
/// reciprocal is periodic (17, 6, 97, ...) and decimal divisors (0.75, 0.95, 0.9, ...) alike.
///
/// Why this must hold: a <see cref="Real"/> is always an exact rational. A periodic value stores
/// its repeating block (<see cref="Real.PeriodStart"/> / <see cref="Real.PeriodLength"/>), and a
/// non-periodic value stores a finite decimal; both are fractions with an exact value. Multiplying
/// by first expanding a periodic operand to a fixed number of fractional digits truncates that
/// operand, so <c>1/17</c> (expanded to 1000 places) is slightly LESS than 1/17 and times 17 gives
/// <c>0.999...984</c> instead of 1 — an arithmetic result that contradicts the identity
/// <c>(a/b)*b == a</c>. The fix multiplies the exact fractions and re-derives the period of the
/// product, so the identity holds for every operand the type can represent exactly.
///
/// These tests assert the identity twice and independently:
///   (a) on the <see cref="Real"/> itself (<c>==</c> and <c>CompareTo</c>), and
///   (b) on the exact rational the result denotes, reconstructed in the test from the PUBLIC
///       value state (magnitude string, Exponent, PeriodStart, PeriodLength) with
///       <see cref="BigInteger"/> — deliberately NOT via any production helper, so a bug in the
///       production period-to-fraction conversion cannot make the test agree with itself.
///
/// Precision is pinned to the documented default (1000 computation / 100 display places) with
/// <see cref="Real.WithPrecision"/>: the computation cap is process-global, and the property is a
/// statement about exact arithmetic at that cap, not about whatever cap a parallel test last set.
/// </summary>
public class RealMultiplyPeriodicExactTests
{
    private const long PinnedComputationDecimalPlaces = 1000L;
    private const long PinnedDisplayDecimalPlaces = 100L;

    private static IDisposable PinnedPrecision() =>
        Real.WithPrecision(PinnedComputationDecimalPlaces, PinnedDisplayDecimalPlaces);

    private static Real Exact(string literal) => new Real(literal);

    private static Real ExactInt(int value) => new Real(value.ToString(CultureInfo.InvariantCulture));

    // -------------------------------------------------------------------------
    // Corpus
    // -------------------------------------------------------------------------

    /// <summary>
    /// The divisor corpus for the <c>(p/q)*q == p</c> property: every integer in 2..60 (which
    /// exhibits periods of length 1..58 — 1/17 has period 16, 1/59 has period 58), five larger
    /// primes (97, 101, 109, 127, 199 — periods 96, 4, 108, 42, 99), and the decimal divisors an
    /// adversarial audit used (0.75, 0.95, 0.9, ... whose reciprocals are periodic). Every divisor
    /// here has a reciprocal whose period is far below the 1000-digit computation cap, so the
    /// QUOTIENT is exact and the identity is a statement about the product alone.
    /// </summary>
    public static IEnumerable<object[]> ExactDivisorCorpus()
    {
        var divisors = new List<string>();
        for (int q = 2; q <= 60; q++)
            divisors.Add(q.ToString(CultureInfo.InvariantCulture));
        divisors.AddRange(new[] { "97", "101", "109", "127", "199" });
        divisors.AddRange(new[]
        {
            "0.75", "0.9", "0.95", "0.55", "0.35", "0.98", "0.94",
            "0.71", "0.49", "0.99999", "1.25", "0.125", "0.05", "2.5",
        });

        int[] numerators = { 1, 2, 7, 123, 9999 };
        foreach (string divisor in divisors)
            foreach (int p in numerators)
                yield return new object[] { p, divisor };
    }

    // -------------------------------------------------------------------------
    // The property
    // -------------------------------------------------------------------------

    /// <summary>
    /// PROPERTY: for every exact rational <c>p/q</c> in the corpus, <c>(p/q)*q</c> is EXACTLY
    /// <c>p</c> — equal as a <see cref="Real"/> and equal as a rational.
    /// </summary>
    [Theory]
    [MemberData(nameof(ExactDivisorCorpus))]
    public void Multiply_PeriodicQuotientTimesItsDivisor_Property_IsExactlyTheDividend(
        int numerator, string divisorLiteral)
    {
        using var precision = PinnedPrecision();

        Real p = ExactInt(numerator);
        Real q = Exact(divisorLiteral);

        Real quotient = p / q;
        Real product = quotient * q;

        string detail = string.Create(
            CultureInfo.InvariantCulture,
            $"p={numerator}, q={divisorLiteral}, p/q={Describe(quotient)}, (p/q)*q={Describe(product)}");

        // (a) The Real-level identity: (p/q)*q compares equal to p.
        Assert.True(product == p, $"(p/q)*q == p is false: {detail}");
        Assert.Equal(0, product.CompareTo(p));

        // (b) The exact-rational identity: the value the product denotes, reconstructed from its
        // own public state, equals p in lowest terms. This is what "exactly p" means for a type
        // whose values ARE rationals; it cannot be satisfied by 0.999...9 of any width.
        (BigInteger Num, BigInteger Den) expected = ToLowestTerms(new BigInteger(numerator), BigInteger.One);
        (BigInteger Num, BigInteger Den) actual = RationalOf(product);
        Assert.True(expected == actual,
            $"(p/q)*q denotes {Fraction(actual)}, expected {Fraction(expected)}: {detail}");
    }

    // -------------------------------------------------------------------------
    // The audit shapes (cycle-4 audit-P1-numeric.md, findings F1/F2/F3)
    // -------------------------------------------------------------------------

    /// <summary>
    /// PROPERTY: <c>(a/b)*b == a</c> for the five shapes the cycle-4 audit published as findings.
    /// </summary>
    [Theory]
    [InlineData(1, "17")]    // m29: 0.999...984, the 62-probe cluster
    [InlineData(2, "17")]    // k05: 1.999...999
    [InlineData(1, "6")]     // 0.1(6) * 6 — mixed period start
    [InlineData(1, "0.75")]  // m07/i16/i20: 0.999...975
    [InlineData(7, "0.95")]  // d36/m01/i17 family: 0.999...9(5)
    public void Multiply_DividendOverDivisorTimesDivisor_AuditShapes_AreExactlyTheDividend(
        int numerator, string divisorLiteral)
    {
        using var precision = PinnedPrecision();

        Real p = ExactInt(numerator);
        Real q = Exact(divisorLiteral);

        Real quotient = p / q;
        Real product = quotient * q;

        Assert.True(product == p, $"(p/q)*q == p is false: p={numerator}, q={divisorLiteral}, got {Describe(product)}");
        Assert.Equal(
            Fraction(ToLowestTerms(new BigInteger(numerator), BigInteger.One)),
            Fraction(RationalOf(product)));
    }

    /// <summary>
    /// PROPERTY: <c>(1/17)*17 == 1</c> exactly, and <c>1/17</c> still RENDERS as the exact
    /// periodic decimal <c>0.(0588235294117647)</c> — the fix must not pay for the identity by
    /// changing how the periodic operand itself is stored or printed.
    /// </summary>
    [Fact]
    public void Multiply_OneOverSeventeenTimesSeventeen_IsExactlyOne_AndOneOverSeventeenStillRendersItsPeriod()
    {
        using var precision = PinnedPrecision();

        Real seventeenth = Exact("1") / Exact("17");
        Assert.Equal("0.(0588235294117647)", seventeenth.ToString());

        Real x = seventeenth * Exact("17");
        Assert.True(x == Real.One, $"x = (1/17)*17 must equal 1, got {Describe(x)}");
        Assert.Equal("1", x.ToString());
        Assert.Equal(0, x.CompareTo(Real.One));
    }

    /// <summary>
    /// PROPERTY: <c>(4/3)*(3/4) == 1</c> — the audit's "decimal-free" shape: a periodic operand
    /// (4/3 = 1.(3)) times a terminating one (3/4 = 0.75), no decimal literal to parse.
    /// </summary>
    [Fact]
    public void Multiply_PeriodicTimesTerminating_FourThirdsTimesThreeQuarters_IsExactlyOne()
    {
        using var precision = PinnedPrecision();

        Real fourThirds = Exact("4") / Exact("3");
        Real threeQuarters = Exact("3") / Exact("4");
        Assert.True(fourThirds.IsPeriodic);
        Assert.False(threeQuarters.IsPeriodic);

        Real product = fourThirds * threeQuarters;
        Assert.True(product == Real.One, $"(4/3)*(3/4) must equal 1, got {Describe(product)}");
        Assert.Equal("1", product.ToString());
    }

    /// <summary>
    /// PROPERTY: periodic times periodic is exact — both operands carry period metadata:
    /// <c>(1/3)*(2/3) == 0.(2)</c> and <c>(1/3)*(1/7) == 0.(047619)</c>.
    /// </summary>
    [Fact]
    public void Multiply_PeriodicTimesPeriodic_IsExact()
    {
        using var precision = PinnedPrecision();

        Real third = Exact("1") / Exact("3");
        Real twoThirds = Exact("2") / Exact("3");
        Assert.True(third.IsPeriodic);
        Assert.True(twoThirds.IsPeriodic);

        Real product = third * twoThirds;
        Assert.Equal("0.(2)", product.ToString());
        Assert.Equal(Fraction((new BigInteger(2), new BigInteger(9))), Fraction(RationalOf(product)));

        Real oneOverTwentyOne = third * (Exact("1") / Exact("7"));
        Assert.Equal("0.(047619)", oneOverTwentyOne.ToString());
        Assert.Equal(Fraction((BigInteger.One, new BigInteger(21))), Fraction(RationalOf(oneOverTwentyOne)));
    }

    /// <summary>
    /// PROPERTY: periodic times non-periodic stays exact when the product is rational:
    /// <c>0.(3)*3 == 1</c> and <c>0.(3)*0.5 == 0.1(6)</c>.
    /// </summary>
    [Fact]
    public void Multiply_PeriodicTimesNonPeriodic_IsExactWhenTheProductIsRational()
    {
        using var precision = PinnedPrecision();

        Real third = Exact("1") / Exact("3");
        Real thirdTimesThree = third * Exact("3");
        Assert.True(thirdTimesThree == Real.One, $"0.(3)*3 must equal 1, got {Describe(thirdTimesThree)}");

        Real thirdTimesHalf = third * Exact("0.5");
        Assert.Equal("0.1(6)", thirdTimesHalf.ToString());
        Assert.Equal(Fraction((BigInteger.One, new BigInteger(6))), Fraction(RationalOf(thirdTimesHalf)));
    }

    /// <summary>
    /// BOUNDARY: the type stores no irrationals. <c>1.4142...694</c> is sqrt(2) truncated to 50
    /// fractional digits — a finite decimal, hence a rational — and the exact path multiplies the
    /// STORED value exactly. The product inherits the operand's truncation; it is not, and is not
    /// claimed to be, the true sqrt(2)/3.
    /// </summary>
    [Fact]
    public void Multiply_PeriodicTimesTruncatedIrrational_IsExactForTheStoredRationalsOnly()
    {
        using var precision = PinnedPrecision();

        // sqrt(2) = 1.41421356237309504880168872420969807856967187537694 8073...  (the stored
        // operand stops at the 50th fractional digit, so it is strictly BELOW the true value).
        Real sqrt2Truncated = Exact("1.41421356237309504880168872420969807856967187537694");
        Real third = Exact("1") / Exact("3");
        Assert.True(third.IsPeriodic);

        Real product = sqrt2Truncated * third;

        (BigInteger Num, BigInteger Den) expected = Mul(RationalOf(sqrt2Truncated), RationalOf(third));
        Assert.Equal(Fraction(expected), Fraction(RationalOf(product)));

        // Cross-path consistency: multiplying by 1/3 agrees with dividing by 3 on the same stored
        // rational, and neither path upgrades the truncation into an exactness claim about sqrt(2).
        Assert.True(product == sqrt2Truncated / Exact("3"),
            $"truncated sqrt(2) * (1/3) must agree with truncated sqrt(2) / 3, got {Describe(product)}");
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

    private static (BigInteger Num, BigInteger Den) Mul(
        (BigInteger Num, BigInteger Den) a, (BigInteger Num, BigInteger Den) b) =>
        ToLowestTerms(a.Num * b.Num, a.Den * b.Den);

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
