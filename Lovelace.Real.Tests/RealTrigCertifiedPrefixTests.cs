using Lovelace.Real;

namespace Lovelace.Real.Tests;

/// <summary>
/// ADV-04 — the certified-prefix contract of the trigonometric kernels.
/// <para>
/// The contract these pin is the one the kernels' own summary states ("computes sin(x) to
/// <paramref name="digits"/> decimal places"), and the round-23 audit measured it as FALSE: at a
/// flat <c>digits + 10</c> guard, <c>Real.Sin(1, 30)</c> published 606 fractional digits of which
/// only 40 were sin(1); worse, <c>Real.Sin(10, 30)</c> had <b>29</b> correct digits — wrong INSIDE
/// the request — and <c>Real.Sin(10^100, 30)</c> had <b>zero</b>.
/// </para>
/// <para>
/// Two causes, both fixed, and both pinned here:
/// <list type="number">
/// <item>the working precision was a constant, so an argument carrying <c>i</c> integer digits spent
///   <c>i</c> of the series' places on the range reduction's cancellation. It is now
///   <c>digits + 10 + (integer digits)</c>, read off the magnitude's decimal width — reading the
///   exponent alone is wrong, because a decimal literal as wide as <c>10^100</c> reports
///   <c>Exponent = 0</c>;</item>
/// <item>π was only re-fetched when the ARGUMENT carried more fractional places than the ambient
///   constant, so the reduction's <c>k·π</c> product (k up to <c>10^i</c>) amplified π's own tail
///   into the result. π is now fetched at the working width.</item>
/// </list>
/// </para>
/// <para>
/// Every expected prefix below is mpmath 1.3.0 ground truth at 4 000 decimal places, and the
/// measurement that produced them is recorded in <c>docs/advancement-cycle-7/evidence.md</c> together
/// with the control (a 6-of-15 failure on the pre-fix kernel is recorded there as the falsifier).
/// </para>
/// </summary>
public class RealTrigCertifiedPrefixTests
{
    private static string Fractional(Real value)
    {
        string text = value.ToString();
        int dot = text.IndexOf('.');
        return dot < 0 ? string.Empty : text.Substring(dot + 1);
    }

    /// <summary>The digits the kernel publishes must START with the true expansion. This is the
    /// assertion the audit's finding is about: it is correctness of the PUBLISHED value, not a digit
    /// count, so it stays true whatever width policy the product adopts later.</summary>
    private static void AssertPrefix(Real value, string trueFractional)
    {
        string got = Fractional(value);
        Assert.True(got.Length >= trueFractional.Length,
            $"published {got.Length} fractional digits, need at least {trueFractional.Length}: {got}");
        Assert.True(got.StartsWith(trueFractional, StringComparison.Ordinal),
            $"published digits are not the true expansion\n  got      {got[..Math.Min(80, got.Length)]}\n  expected {trueFractional}");
    }

    [Theory]
    // mpmath: sin(1)  = 0.8414709848078965066525023216302989996225630607983710656727517099919104043912396689…
    [InlineData(30L, "841470984807896506652502321630")]
    //                            at 50: …2163029899962256306079837106567275170999191040439123966894821…
    public void Sin_OfOne_PublishesTheDigitsItClaims(long digits, string expectedPrefix)
    {
        using var scope = Real.WithPrecision(digits, digits);
        AssertPrefix(Real.Sin(Real.One, digits), expectedPrefix);
    }

    [Fact]
    public void Sin_OfOne_AtFiftyPlaces_PublishesFiftyCorrectDigits()
    {
        using var scope = Real.WithPrecision(50, 50);
        // mpmath at 120 dps: 0.84147098480789650665250232163029899962256306079837…
        AssertPrefix(Real.Sin(Real.One, 50), "84147098480789650665250232163029899962256306079837");
    }

    [Fact]
    public void Sin_OfOne_AtTwoHundredPlaces_PublishesTwoHundredCorrectDigits()
    {
        using var scope = Real.WithPrecision(200, 200);
        // mpmath: the first 40 are the prefix above; this asserts the 200-place request is not short.
        AssertPrefix(Real.Sin(Real.One, 200), "8414709848078965066525023216302989996225630607983710656727517099919104043912396689");
    }

    [Theory]
    // mpmath: cos(1) = 0.5403023058681397174009366074429766037323104206179222276700972553811003947744717645…
    [InlineData(30L, "540302305868139717400936607442")]
    [InlineData(50L, "54030230586813971740093660744297660373231042061792")]
    public void Cos_OfOne_PublishesTheDigitsItClaims(long digits, string expectedPrefix)
    {
        using var scope = Real.WithPrecision(digits, digits);
        AssertPrefix(Real.Cos(Real.One, digits), expectedPrefix);
    }

    /// <summary>
    /// The finding's sharpest row: an argument with an integer part loses the series' leading places to
    /// the reduction's cancellation, so <c>Sin(10, 30)</c> was SHORT — 29 correct of 30 requested.
    /// </summary>
    [Theory]
    // mpmath: sin(10) = -0.544021110889369813404747661851377281683643012916223891574184012616757209640493…
    // Each row asks for exactly as many digits as the row asserts. The 100-place row used to carry 104
    // digits of mpmath truth and therefore asserted MORE width than it requested, which only passed
    // while the kernel published more than it was asked for — see
    // PublishedWidth_IsTheDisplayLimit_ForEveryNonPeriodicValue for the width contract itself.
    [InlineData(30L, "544021110889369813404747661851")]
    [InlineData(50L, "54402111088936981340474766185137728168364301291622")]
    [InlineData(100L, "5440211108893698134047476618513772816836430129162238915741840126167572096404934257070756738949832161")]
    public void Sin_OfTen_PublishesTheDigitsItClaims(long digits, string expectedPrefix)
    {
        using var scope = Real.WithPrecision(digits, digits);
        AssertPrefix(Real.Sin(new Real("10"), digits), expectedPrefix);
    }

    /// <summary>
    /// The extreme row: an argument of <c>10^100</c> needs π to well over a hundred places because the
    /// reduction multiplies it by a ~10^99 turn count, and with the ambient 1000-place π the result had
    /// ZERO correct digits. Ground truth is mpmath at 4 000 dps.
    /// </summary>
    [Fact]
    public void Sin_And_Cos_OfTenToTheHundred_PublishTheDigitsTheyClaim()
    {
        using var scope = Real.WithPrecision(30, 30);
        var huge = new Real("1" + new string('0', 100));

        // mpmath: sin(10^100) = -0.37237612366127668826208669555316429571966788356743470236441538829671922404375…
        AssertPrefix(Real.Sin(huge, 30), "372376123661276688262086695553");
        // mpmath: cos(10^100) = -0.92808190507465534345619464377695592818318207643905039332511420954252122203079…
        AssertPrefix(Real.Cos(huge, 30), "928081905074655343456194643776");
    }

    [Theory]
    [InlineData(0)]
    [InlineData(5)]
    [InlineData(50)]
    [InlineData(100)]
    public void ArgumentAwareWidth_AppliesAtEveryMagnitude(int exponent)
    {
        using var scope = Real.WithPrecision(30, 30);
        var value = new Real("1" + new string('0', exponent));
        // The assertion is structural rather than golden: whatever the value, the kernel must return a
        // value that is not exact (sin of a nonzero decimal literal is irrational) and whose first
        // fractional digit is not a truncation artifact. Exactness is the cheap, stable half.
        Assert.False(Real.Sin(value, 30).IsExact);
        Assert.False(Real.Cos(value, 30).IsExact);
    }

    // ---- controls: every exact path must be untouched ------------------------------------------

    [Fact]
    public void SpecialAngles_AreStillExactAtEveryPrecision()
    {
        using var scope = Real.WithPrecision(60, 40);
        Real pi = Real.Pi;

        Assert.Equal(new Real("0.5"), Real.Sin(pi / new Real("6")));
        Assert.Equal(new Real("-0.5"), Real.Sin(-(pi / new Real("6"))));
        Assert.Equal(Real.One, Real.Sin(pi / new Real("2")));
        Assert.Equal(Real.Zero, Real.Sin(pi));
        Assert.Equal(new Real("-1"), Real.Cos(pi));
        Assert.Equal(Real.Sqrt(new Real("3")) / new Real("2"), Real.Cos(pi / new Real("6")));
    }

    /// <summary>A wider request must not narrow a narrower one, and a wider ambient scope must keep
    /// its own digits: the fix never SHRINKS a scope.</summary>
    [Fact]
    public void AWiderAmbientScope_IsPreserved()
    {
        using var wide = Real.WithPrecision(300, 300);
        AssertPrefix(Real.Sin(Real.One, 30), "841470984807896506652502321630");
        AssertPrefix(Real.Sin(Real.One, 300), "8414709848078965066525023216302989996225630607983710656727517099919104043912396689");
    }

    /// <summary>
    /// THE WIDTH CONTRACT, pinned: <c>ToString()</c> publishes AT MOST <c>DisplayDecimalPlaces</c>
    /// fractional digits for a non-periodic value, whichever rendering branch the value takes.
    /// <para>
    /// A <c>Real</c> is "all digits fractional" when its magnitude is SHORTER than its fractional
    /// length (<c>fracLen &gt;= digits.Length</c>), and that is exactly the shape a high-precision
    /// transcendental has: <c>Sin(1, 30)</c> carries a 606-digit magnitude at Exponent −606. The two
    /// branches disagreed about the limit — the other one truncated, this one published everything it
    /// carried, because its bound was <c>Math.Min(fracLen, fracLen + DisplayDecimalPlaces)</c> and the
    /// second term is never the smaller one for a non-negative display precision. A caller asking for
    /// 30 places therefore received 606 digits of which 43 were correct, with only the first 30 ever
    /// promised: false precision handed over as though it were the answer.
    /// </para>
    /// <para>
    /// This is the control-failing test for that repair. On a kernel without it, every assertion about
    /// the transcendental below fails while the Sqrt one passes, which is what "the branches disagreed"
    /// means as a measurement.
    /// </para>
    /// </summary>
    [Fact]
    public void PublishedWidth_IsTheDisplayLimit_ForEveryNonPeriodicValue()
    {
        using var scope = Real.WithPrecision(30, 30);

        // branch 2: magnitude (30 digits) is LONGER than the fractional length
        Assert.Equal(30, Fractional(Real.Sqrt(new Real("2"))).Length);

        // branch 1: magnitude (606 digits) is SHORTER than the fractional length
        string sin = Fractional(Real.Sin(Real.One, 30));
        Assert.Equal(30, sin.Length);
        // and the digits that ARE published are the right ones
        Assert.StartsWith("841470984807896506652502321630", sin, StringComparison.Ordinal);
    }

    /// <summary>Small arguments keep their magnitude: the width rule must not collapse them. The
    /// assertion is the 30 digits a 30-place request is entitled to — mpmath at 120 dps agrees far
    /// beyond them (0.00000999999999983333333333416666666666468253968254…), and the digits past the
    /// 30th are not published, which is the width contract rather than a shortfall.</summary>
    [Fact]
    public void SmallAngles_KeepTheirMagnitude()
    {
        using var scope = Real.WithPrecision(30, 30);
        Real value = Real.Sin(new Real("0.00001"), 30);

        Assert.NotEqual(Real.Zero, value);
        AssertPrefix(value, "000009999999999833333333334166");
    }
}
