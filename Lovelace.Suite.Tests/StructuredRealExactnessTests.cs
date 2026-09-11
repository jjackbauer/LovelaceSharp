using System.Numerics;
using Lovelace.MathIR;
using Lovelace.Real;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Suite.Tests;

/// <summary>
/// The wire's <c>exact</c> flag as an end-to-end property, at the one boundary an agent reads.
///
/// <para>The flag used to be a SHAPE guess read off the value — <c>IsZero || IsPeriodic ||
/// −Exponent ≤ 18</c> — so it certified an 18-digit truncation of <c>1/1009</c> as exact (with a
/// numerator and denominator an agent could mistake for the number) and called the exact
/// <c>1/(10^19)</c> inexact.  These tests state the property the payload actually promises: an exact
/// payload carries the exact value, and a truncation is never certified.  The assertions are
/// identities over generated quotients and precisions, not a list of pinned strings.</para>
/// </summary>
public class StructuredRealExactnessTests
{
    private static StructuredValueDto Structured(string source)
    {
        var engine = new SuiteEngine();
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        return StructuredProjection.ToStructured(engine.Evaluate(source));
    }

    private static (BigInteger Num, BigInteger Den) Reduced(BigInteger numerator, BigInteger denominator)
    {
        if (denominator.Sign < 0) { numerator = -numerator; denominator = -denominator; }
        BigInteger gcd = BigInteger.GreatestCommonDivisor(BigInteger.Abs(numerator), denominator);
        return gcd.IsZero ? (numerator, denominator) : (numerator / gcd, denominator / gcd);
    }

    // ---------------------------------------------------------------------
    // The soundness property: exact means exact
    // ---------------------------------------------------------------------

    public static IEnumerable<object[]> GeneratedQuotients()
    {
        (long Num, long Den)[] quotients =
        {
            (1, 2), (1, 4), (1, 8), (1, 1024), (1, 1009), (1, 17), (22, 7), (1, 3), (355, 113), (3, 2),
        };
        foreach (long precision in new long[] { 5, 18, 30, 37, 50, 1000 })
            foreach ((long num, long den) in quotients)
                yield return new object[] { num, den, precision };
    }

    /// <summary>If the wire says exact, the numerator/denominator it published must BE the quotient.
    /// A truncation that slips through here is a wrong value certified as the right one.</summary>
    [Theory]
    [MemberData(nameof(GeneratedQuotients))]
    public void ExactPayload_AlwaysCarriesTheExactQuotient(long numerator, long denominator, long precision)
    {
        StructuredValueDto sv = Structured($"setprecision({precision}); {numerator}/{denominator}");

        if (sv.Exact != true)
            return;

        Assert.Equal("Real", sv.Kind);
        Assert.NotNull(sv.Numerator);
        Assert.NotNull(sv.Denominator);

        var published = Reduced(BigInteger.Parse(sv.Numerator!), BigInteger.Parse(sv.Denominator!));
        Assert.Equal(Reduced(numerator, denominator), published);
    }

    public static IEnumerable<object[]> GeneratedTerminatingDenominators()
    {
        (int Twos, int Fives)[] denominators = { (1, 0), (3, 0), (0, 3), (1, 1), (20, 0), (0, 20), (20, 20), (64, 0), (0, 64), (100, 3) };
        foreach (long precision in new long[] { 1, 5, 18, 30, 1000 })
            foreach ((int twos, int fives) in denominators)
                yield return new object[] { twos, fives, precision };
    }

    /// <summary>The completeness property for the decidable family: a denominator of the form
    /// <c>2^a·5^b</c> terminates, so the quotient is exact at EVERY budget and the payload must say
    /// so — including expansions longer than the budget that used to come back as 0.</summary>
    [Theory]
    [MemberData(nameof(GeneratedTerminatingDenominators))]
    public void TerminatingQuotient_IsAlwaysCertifiedExact(int twos, int fives, long precision)
    {
        BigInteger denominator = BigInteger.Pow(2, twos) * BigInteger.Pow(5, fives);

        StructuredValueDto sv = Structured($"setprecision({precision}); 1/{denominator}");

        Assert.True(sv.Exact == true,
            $"1/(2^{twos}·5^{fives}) at {precision} places was not certified exact: value={sv.Value}");
        var published = Reduced(BigInteger.Parse(sv.Numerator!), BigInteger.Parse(sv.Denominator!));
        Assert.Equal(Reduced(BigInteger.One, denominator), published);
    }

    // ---------------------------------------------------------------------
    // The reported cases, stated directly
    // ---------------------------------------------------------------------

    [Fact]
    public void TruncatedQuotientAtTheBudget_IsNotCertifiedExact()
    {
        StructuredValueDto sv = Structured("setprecision(18); 1/1009");

        // the truncating path really is the one under test
        Assert.Equal("0.000991080277502477", sv.Value);
        Assert.True(sv.Exact != true, "an 18-digit truncation of 1/1009 was certified exact");
        Assert.Null(sv.Numerator);
        Assert.Null(sv.Denominator);
    }

    [Fact]
    public void ExactQuotientPastTheEighteenPlaceShape_IsCertifiedExact()
    {
        StructuredValueDto sv = Structured("1/(10^19)");

        Assert.True(sv.Exact == true, "the exact 1/(10^19) was not certified exact");
        Assert.Equal("1", sv.Numerator);
        Assert.Equal("10000000000000000000", sv.Denominator);
    }

    [Fact]
    public void PeriodicQuotient_IsCertifiedExactWithItsFraction()
    {
        StructuredValueDto sv = Structured("1/17");

        Assert.Equal("0.(0588235294117647)", sv.Value);
        Assert.True(sv.Exact == true);
        Assert.Equal("1", sv.Numerator);
        Assert.Equal("17", sv.Denominator);
    }

    [Fact]
    public void PeriodicQuotientTimesItsDivisor_IsExactlyOne()
    {
        StructuredValueDto sv = Structured("setprecision(18); (1/17)*17");

        Assert.Equal("1", sv.Value);
        Assert.True(sv.Exact == true);
        Assert.Equal("1", sv.Numerator);
        Assert.Equal("1", sv.Denominator);
    }

    [Theory]
    [InlineData("2^-5000", "2")]
    [InlineData("10^-1001", "10")]
    [InlineData("2^-100000", "2")]
    public void TerminatingQuotientBeyondTheDigitBudget_IsCertifiedExact(string source, string baseText)
    {
        int exponent = -int.Parse(source[(source.IndexOf('^') + 1)..]);
        BigInteger expectedDenominator = BigInteger.Pow(int.Parse(baseText), exponent);

        StructuredValueDto sv = Structured(source);

        Assert.True(sv.Exact == true, source + " was not certified exact");
        Assert.Equal("1", sv.Numerator);
        Assert.Equal(expectedDenominator.ToString(), sv.Denominator);
        Assert.True(sv.Value!.StartsWith("0.0000", StringComparison.Ordinal),
            source + " rendered as " + sv.Value[..Math.Min(40, sv.Value.Length)]);
    }

    [Theory]
    [InlineData("pi")]
    [InlineData("e")]
    [InlineData("sqrt(2)")]
    [InlineData("setprecision(18); pi")]
    [InlineData("setprecision(18); e")]
    [InlineData("setprecision(18); sqrt(2)")]
    [InlineData("evalf(1/3, 50)")]
    public void TruncatedIrrational_IsNotCertifiedExact(string source)
    {
        StructuredValueDto sv = Structured(source);

        Assert.True(sv.Exact != true, source + " was certified exact although its digits are truncated");
        Assert.Null(sv.Numerator);
        Assert.Null(sv.Denominator);
    }

    [Fact]
    public void ExactSquareRoot_IsStillCertifiedExact()
    {
        StructuredValueDto sv = Structured("sqrt(4)");

        Assert.Equal("2", sv.Value);
        Assert.True(sv.Exact == true);
        Assert.Equal("2", sv.Numerator);
        Assert.Equal("1", sv.Denominator);
    }

    /// <summary>The fixed-width tier is part of the tower: its own truncating path (expanding a
    /// periodic operand to the working width) must not launder a wrong value into an exact Real when
    /// it promotes back through <c>ToReal</c>.  Here <c>0.(0588235294117647)·17</c> at 18 places is
    /// <c>0.999999999999999985</c>, which the old shape rule read as exact.</summary>
    [Fact]
    public void FixedWidthTruncation_IsNotPublishedAsAnExactReal()
    {
        LReal64 periodic = LReal64.Parse("0.(0588235294117647)");
        LReal64 product = periodic * LReal64.Parse("17");

        Assert.True(product != LReal64.One, "the fixed-width product was not the truncated 0.999…985");

        StructuredValueDto sv = StructuredProjection.ToStructured(new Value(product.ToReal()));

        Assert.True(sv.Exact != true,
            "the fixed-width truncation " + product.ToReal().ToNatural().ToString() + "e" + product.ToReal().Exponent +
            " was published as an exact Real");
    }
}
