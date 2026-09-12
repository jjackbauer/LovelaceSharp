using System.Diagnostics;
using Lovelace.Real;

using Int = Lovelace.Integer.Integer;

namespace Lovelace.Real.Tests;

/// <summary>
/// The special-angle fast path may not answer an INEXACT argument with the exact value of the angle
/// it merely matches.
///
/// <para><b>The defect these pin.</b> <see cref="Real.Sin(Real)"/> and <see cref="Real.Cos(Real)"/>
/// looked the reduced argument up in the 16-row table of rational multiples of π and returned the
/// table's exact value whenever the DIGITS matched — without asking whether the argument was the
/// angle or only a truncation of π that happens to agree with the ambient π digit for digit.
/// <c>pi(30)</c> is such a truncation (<see cref="Real.PiTo(long)"/> marks it inexact), and under the
/// wire's 30-place plugin budget it IS <see cref="Real.Pi"/>, so <c>cos(pi(30))</c> came back as
/// exactly −1 and <c>cos(2*pi(30))</c> as exactly 1, both <c>exact:true</c>, where the true values
/// deviate from ±1 in the 61st decimal.  The same shortcut answered <c>sin(pi(30))</c> with 0 (the
/// value is 5.03e-31), <c>sin(pi(30)/2)</c> with 1 (the value is 1 − 3.16e-62) and
/// <c>sin(pi(30)/6)</c> with 1/2 (the value is 1/2 − 7.26e-32).</para>
///
/// <para><b>Ground truth.</b> Every <c>True*</c> constant below is mpmath at 340 dps
/// (<c>mp.dps = 340</c>), with <c>P</c> = π TRUNCATED at the stated number of fractional places —
/// the same truncation <see cref="Real.PiTo(long)"/> performs — and the value computed from that
/// truncated P with mpmath's own <c>cos</c>/<c>sin</c>/<c>tan</c>.  Script, raw values and the
/// per-case error/distance table: <c>docs/goal-cycle-6/round-4/</c> (gt.json and the implementation
/// report).  The constants carry ~230 significant digits, so every comparison below is limited by
/// the implementation, never by the ground truth.</para>
///
/// <para><b>Tolerances.</b> Each comparison bound is the requested precision plus five places
/// (the convention this project's other π-resolution property tests use), tightened case by case to
/// the largest power of ten that still clears the series' own measured error by at least a factor of
/// ten.  Where the table's exact value sits FARTHER from the truth than the series' error — the
/// 30-place <c>sin(P/2)</c> family — the bound cannot separate value from value, and the case is
/// carried by the exactness flag and by the inequality against the table's number; those cases say
/// so in their own doc comment.  The measured errors are in the round-4 report.</para>
/// </summary>
public class RealSpecialAngleInexactArgumentTests
{
    // ---------------------------------------------------------------- ground truth (mpmath, 340 dps)

    /// <summary>cos(π truncated at 30 places); the table's −1 differs from it only in the 61st decimal.</summary>
    private const string TrueCosP30 = "-0.9999999999999999999999999999999999999999999999999999999999998735537421186443267632593548461102582682660653910166946445624451147058940912781367193007250663402050118136351269388463977161686110850145938257531073387748180666682303614519";

    private const string TrueCos2P30 = "0.9999999999999999999999999999999999999999999999999999999999994942149684745773070530374193844410330730642615640667785782498124361358407617159664565662313402488070800648480966120264578298849727055839960287981755627134248462392408375018";

    /// <summary>sin(π truncated at 30 places) — 5.0288…e-31, not the table's 0.</summary>
    private const string TrueSinP30 = "0.0000000000000000000000000000005028841971693993751058209749445923078164062862089986280348253209211263555679578210743560732324203253166004585576499201460295881288391502095749357270305866881838884639730840698091750210686567634012590852135128491844406752699142625772";

    private const string TrueSin2P30 = "-0.000000000000000000000000000001005768394338798750211641949889184615632812572417997256069650514666602951655143488901008703296294945229310278109042912397356432683010871974694493227661963593533038059317351503871987504124961650538482065314760184335219067216414619059";

    /// <summary>sin(π truncated at 30 places / 2) — 1 − 3.16e-62, not the table's 1.</summary>
    private const string TrueSinHalfP30 = "0.9999999999999999999999999999999999999999999999999999999999999683884355296610816908148387115275645670665163477541736611406107790309693916226057488976292185399264555657477256588265858826949436894761523105470621951638131276377829961121";

    /// <summary>cos(π truncated at 30 places / 3) — a half plus 1.45e-31, not the table's 1/2.</summary>
    private const string TrueCosThirdP30 = "0.5000000000000000000000000000001451701633034807840667972564990790399572234138730749223216065893667845061006246308157139667247595531003958295169765280012929976675543130059811258438526013156694015456790467685447678585641945744404427421";

    /// <summary>sin(π truncated at 30 places / 6) — the numeric core of the wire's
    /// <c>evalf(sin(pi(30)/6), 40)</c>: 1/2 − 7.26e-32, not the table's 1/2.</summary>
    private const string TrueSinSixthP30 = "0.4999999999999999999999999999999274149183482596079666013717504552114273099032437443413123152929890880371225449876641610003019023861706117708309105266718149037568450828129310851875637250669751644019993061792886470726174131954280989849";

    private const string TrueCosQuarterP30 = "0.7071067811865475244008443621049379374913284464649432231286337302782740462449497187793138196563046615711663622660038985684590993311757290506240751102708355007358637469973946899933022433760218191266896203239089316891652440366567221679";

    /// <summary>tan(π truncated at 30 places / 4) — 1 − 2.51e-31, not the table's 1.</summary>
    private const string TrueTanQuarterP30 = "0.9999999999999999999999999999997485579014153003124470895125277354576562671958138098711438757960779135357645768161331978923779418842050842120965058814150239082565733486468973943571299001788430177565564084853900921820189004180236690451";

    private const string TrueCosP40 = "-0.9999999999999999999999999999999999999999999999999999999999999999999999999999999975918633674607779713555948386217724988571354592044184668854712876650446698806342527201672734305306684598601279806785021064372780394052236274355536837492";
    private const string TrueCos2P40 = "0.9999999999999999999999999999999999999999999999999999999999999999999999999999999903674534698431118854223793544870899954285418368176738675418851506601786795225370224789131756768108022607427881326144200722214334407239480818349388866652";
    private const string TrueSinP40 = "0.00000000000000000000000000000000000000006939937510582097494459230781640628620899862803482534211706798214808651328230664703813688372802955238227883379938682748614183153846092455921498198635152076955405803296898545982091990990840165753000145928895408242418741599152685441593";
    private const string TrueSinHalfP40 = "0.9999999999999999999999999999999999999999999999999999999999999999999999999999999993979658418651944928388987096554431247142838648011046167213678219162611674701585629988192545770906651083821839293899315946331894898028206947949396060675";
    private const string TrueCosThirdP40 = "0.5000000000000000000000000000000000000000200338739494687764555005597184764000186832222745402939331401224122685863320624623155880913533477846398785508273055031074866389786352918885605173644489383045731397124643075265856172111930085121";
    private const string TrueTanQuarterP40 = "0.9999999999999999999999999999999999999999653003124470895125277038460917968568955012880167454637469731700272470879157219621552859626644539929337282360923183986252597444840343243039729168128883661640583104956431787644993543287014981579";

    private const string TrueCosP100 = "-0.9999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999966258459411031427995468000444634";
    private const string TrueCos2P100 = "0.9999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999865033837644125711981872001778535";
    private const string TrueSinP100 = "0.00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000008214808651328230664709384460955058223172535940812848111745028410270193852110555964462294895493038196442881097566593344612847564823378678316527120190914564856692346034861045432664821339360726024914127363219357342651503191962095052082";
    private const string TrueSinHalfP100 = "0.9999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999991564614852757856998867000111158";
    private const string TrueCosThirdP100 = "0.5000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000237141099309281027618424854340063310009282719998994914381630782539335875956513661932804331116405630273185259296293827079255918125776";
    private const string TrueTanQuarterP100 = "0.9999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999589259567433588466764530776952247088841373202959357594412748579486490307394472201776885255225348090186291330268912475770490621647673";

    /// <summary>sin(1), cos(1), sin(1/2), cos(1/2), sin(1/3): mpmath at 200 dps — the
    /// exact-argument controls, which the fix must not move.</summary>
    private const string TrueSinOne = "0.841470984807896506652502321630298999622563060798371065672752";
    private const string TrueCosOne = "0.540302305868139717400936607442976603732310420617922227670097";
    private const string TrueSinHalf = "0.479425538604203000273287935215571388081803367940600675188617";
    private const string TrueCosHalf = "0.877582561890372716116281582603829651991645197109744052997611";
    private const string TrueSinThird = "0.327194696796152244173344085267620606064301406893759791590056";

    // ---------------------------------------------------------------- helpers

    /// <summary>10^-<paramref name="places"/> as an exact decimal literal.</summary>
    private static Real TenToTheMinus(long places) =>
        Real.Parse("0." + new string('0', (int)places - 1) + "1");

    /// <summary>π truncated at <paramref name="places"/> places — built under a scope wide enough
    /// for the request whatever the ambient cap happens to be.</summary>
    private static Real PiTruncatedAt(long places)
    {
        using var scope = Real.WithPrecision(places, places);
        return Real.PiTo(places);
    }

    /// <summary>The value as (sign)(up to 20 leading digits)e(decimal exponent) — for failure
    /// messages, where a fixed-point rendering of a 1e-200 difference is unreadable.</summary>
    private static string Describe(Real value)
    {
        if (Real.IsZero(value)) return "0";
        string magnitude = value.ToNatural().ToString();
        string head = magnitude.Length > 20 ? magnitude[..20] : magnitude;
        return (Int.IsNegative(value) ? "-" : "") + head + "e" + (value.Exponent + magnitude.Length - 1);
    }

    /// <summary>
    /// Asserts <c>|actual − trueValue| &lt; 10^-places</c> against the mpmath decimal in
    /// <paramref name="trueDigits"/>.  The subtraction runs at 6000 places so the comparison itself
    /// is not truncated at the ambient precision under test.
    /// </summary>
    private static void AssertAgreesWithGroundTruth(Real actual, string trueDigits, long places, string what)
    {
        Real error;
        using (Real.WithPrecision(6000, 60))
            error = Real.Abs(actual - Real.Parse(trueDigits));

        Assert.True(error < TenToTheMinus(places),
            $"{what}: |value − true value| = {Describe(error)} is not below 10^-{places}. " +
            $"value = {Describe(actual)} (exact={actual.IsExact}); true value = {trueDigits}");
    }

    // ---------------------------------------------------------------- cos of a truncated π

    /// <summary>
    /// <c>cos(pi(n))</c> for n = 30, 40, 100, each evaluated at the ambient precision whose π the
    /// argument agrees with digit for digit — the exact shape that took the shortcut.  The result
    /// must not be the table's −1 and must not advertise exactness.  Tolerance 10^-(n+60): the
    /// series' measured error is 3.2e-102 / 2.4e-131 / 3.7e-233, the table's −1 is 1.26e-61 /
    /// 2.4e-81 / 3.4e-201 away from the truth, so the bound has teeth at every n.
    /// </summary>
    [Theory]
    [InlineData(30L, 90L, TrueCosP30)]
    [InlineData(40L, 120L, TrueCosP40)]
    [InlineData(100L, 220L, TrueCosP100)]
    public void Cos_OfTruncatedPi_IsNotTheTablesMinusOne(long places, long tolerancePlaces, string trueValue)
    {
        using var scope = Real.WithPrecision(places, places);
        Real actual = Real.Cos(PiTruncatedAt(places));

        Assert.False(actual.IsExact,
            $"cos(pi({places})) is the cosine of a TRUNCATION of π and cannot be exact; got {Describe(actual)}");
        Assert.NotEqual(new Real("-1"), actual);
        AssertAgreesWithGroundTruth(actual, trueValue, tolerancePlaces, $"cos(pi({places}))");
    }

    /// <summary><c>cos(2*pi(n))</c>: the argument reduces to zero against the ambient π, so the
    /// shortcut answered exactly 1.  The true value is 1 − 2δ², δ = π − π_truncated.</summary>
    [Theory]
    [InlineData(30L, 90L, TrueCos2P30)]
    [InlineData(40L, 120L, TrueCos2P40)]
    [InlineData(100L, 220L, TrueCos2P100)]
    public void Cos_OfTwiceTruncatedPi_IsNotTheTablesOne(long places, long tolerancePlaces, string trueValue)
    {
        using var scope = Real.WithPrecision(places, places);
        Real actual = Real.Cos(PiTruncatedAt(places) * new Real("2"));

        Assert.False(actual.IsExact,
            $"cos(2*pi({places})) is the cosine of a truncation and cannot be exact; got {Describe(actual)}");
        Assert.NotEqual(Real.One, actual);
        AssertAgreesWithGroundTruth(actual, trueValue, tolerancePlaces, $"cos(2*pi({places}))");
    }

    /// <summary>The ambient <see cref="Real.Pi"/> is itself an inexact truncation — its own
    /// <see cref="Real.IsExact"/> says so — so the same rule applies to it: the table's −1 is the
    /// value of π, not the value of the 30-place decimal the constant carries.</summary>
    [Fact]
    public void Cos_OfTheAmbientPi_IsNotTheTablesMinusOne()
    {
        using var scope = Real.WithPrecision(30, 30);

        Assert.False(Real.Pi.IsExact, "the premise: the ambient π is a truncation");

        Real actual = Real.Cos(Real.Pi);

        Assert.False(actual.IsExact);
        Assert.NotEqual(new Real("-1"), actual);
        AssertAgreesWithGroundTruth(actual, TrueCosP30, 90L, "cos(the ambient π at 30 places)");
    }

    // ---------------------------------------------------------------- sin of a truncated π

    /// <summary><c>sin(pi(n))</c> is the DISTANCE from the truncation to π, not zero: the shortcut
    /// returned the table's exact 0.  Tolerance 10^-(n+40): the series' error is 6.3e-72 / 3.4e-91 /
    /// 6.7e-211, the table's 0 is 5.0e-31 / 6.9e-41 / 8.2e-101 away from the truth.</summary>
    [Theory]
    [InlineData(30L, 60L, TrueSinP30)]
    [InlineData(40L, 80L, TrueSinP40)]
    [InlineData(100L, 200L, TrueSinP100)]
    public void Sin_OfTruncatedPi_IsNotTheTablesZero(long places, long tolerancePlaces, string trueValue)
    {
        using var scope = Real.WithPrecision(places, places);
        Real actual = Real.Sin(PiTruncatedAt(places));

        Assert.False(actual.IsExact);
        Assert.NotEqual(Real.Zero, actual);
        AssertAgreesWithGroundTruth(actual, trueValue, tolerancePlaces, $"sin(pi({places}))");
    }

    /// <summary><c>sin(2*pi(n))</c>: negative, and 2δ in magnitude.</summary>
    [Fact]
    public void Sin_OfTwiceTruncatedPi_IsNotTheTablesZero()
    {
        using var scope = Real.WithPrecision(30, 30);
        Real actual = Real.Sin(PiTruncatedAt(30) * new Real("2"));

        Assert.False(actual.IsExact);
        Assert.NotEqual(Real.Zero, actual);
        // 2·P sits below 2π because P is short of π, so its sine is negative.
        Assert.True(actual < Real.Zero, $"sin(2*pi(30)) must be negative; got {Describe(actual)}");
        AssertAgreesWithGroundTruth(actual, TrueSin2P30, 60L, "sin(2*pi(30))");
    }

    /// <summary>
    /// <c>sin(pi(n)/2)</c>: the shortcut answered exactly 1 (the table's (1,2) row) for an argument
    /// whose sine is 1 − 3.16e-62 (30 places) / 1 − 8.4e-202 (100 places).
    /// <para>
    /// The comparison bound is 10^-(n+10), which is LOOSER than the table's distance from the truth
    /// in the 30- and 100-place cases (the series' own cancellation error there — 4.5e-45 at 30
    /// places — is larger than 3.16e-62): value cannot separate the two answers here.  What carries
    /// the case is the exactness flag and <c>value &lt; 1</c>, both of which the table's exact 1
    /// fails and the truncated angle's sine satisfies.
    /// </para>
    /// </summary>
    [Theory]
    [InlineData(30L, 40L, TrueSinHalfP30)]
    [InlineData(40L, 50L, TrueSinHalfP40)]
    [InlineData(100L, 110L, TrueSinHalfP100)]
    public void Sin_OfHalfTruncatedPi_IsNotTheTablesOne(long places, long tolerancePlaces, string trueValue)
    {
        using var scope = Real.WithPrecision(places, places);
        Real actual = Real.Sin(PiTruncatedAt(places) / new Real("2"));

        Assert.False(actual.IsExact);
        Assert.NotEqual(Real.One, actual);
        Assert.True(actual < Real.One, $"sin(pi({places})/2) is below 1; got {Describe(actual)}");
        AssertAgreesWithGroundTruth(actual, trueValue, tolerancePlaces, $"sin(pi({places})/2)");
    }

    /// <summary>
    /// <c>sin(pi(30)/6)</c> — the numeric core of the wire expression
    /// <c>evalf(sin(pi(30)/6), 40)</c> that row 1 was opened on.  The argument is built at 40 places
    /// so the quotient is the terminating π₃₀/6 exactly (<c>Pi/6</c> truncated AT 30 places would be
    /// a different argument); sine of it is 1/2 − 7.26e-32, not the table's exactly 1/2.
    /// </summary>
    [Fact]
    public void Sin_OfSixthTruncatedPi_IsNotTheTablesHalf()
    {
        Real argument;
        using (Real.WithPrecision(40, 40))
            argument = PiTruncatedAt(30) / new Real("6");

        using var scope = Real.WithPrecision(30, 30);
        Real actual = Real.Sin(argument);

        Assert.False(actual.IsExact);
        Assert.NotEqual(new Real("0.5"), actual);
        AssertAgreesWithGroundTruth(actual, TrueSinSixthP30, 40L, "sin(pi(30)/6)");
    }

    // ---------------------------------------------------------------- cos of a truncated π fraction

    /// <summary><c>cos(pi(n)/3)</c>: the shortcut answered exactly 1/2; the true value carries
    /// +1.45e-31 (30 places) / +2.4e-101 (100 places) that a half does not have.  The measured
    /// series errors are 7.8e-48 / 1.5e-57 / 5.6e-118, so the bounds have teeth at every n.</summary>
    [Theory]
    [InlineData(30L, 40L, TrueCosThirdP30)]
    [InlineData(40L, 50L, TrueCosThirdP40)]
    [InlineData(100L, 110L, TrueCosThirdP100)]
    public void Cos_OfThirdTruncatedPi_IsNotTheTablesHalf(long places, long tolerancePlaces, string trueValue)
    {
        using var scope = Real.WithPrecision(places, places);
        Real actual = Real.Cos(PiTruncatedAt(places) / new Real("3"));

        Assert.False(actual.IsExact);
        Assert.NotEqual(new Real("0.5"), actual);
        AssertAgreesWithGroundTruth(actual, trueValue, tolerancePlaces, $"cos(pi({places})/3)");
    }

    /// <summary><c>cos(pi(30)/4)</c>: the table's √2/2 is a value of the ANGLE; the argument is the
    /// truncated quarter, whose cosine differs from it by 4.4e-32.  The series' error is 4.5e-46,
    /// so the 10^-40 bound separates the two answers.</summary>
    [Fact]
    public void Cos_OfQuarterTruncatedPi_IsNotTheTablesRoot()
    {
        using var scope = Real.WithPrecision(30, 30);
        Real actual = Real.Cos(PiTruncatedAt(30) / new Real("4"));

        Assert.False(actual.IsExact);
        Assert.NotEqual(Real.Sqrt(new Real("2")) / new Real("2"), actual);
        AssertAgreesWithGroundTruth(actual, TrueCosQuarterP30, 40L, "cos(pi(30)/4)");
    }

    // ---------------------------------------------------------------- tangent

    /// <summary>
    /// <c>tan(pi(n)/4)</c>.  <see cref="Real"/> exposes no tangent entry point (the language's
    /// <c>tan</c> is <c>sin/cos</c> at the same argument — see the round-4 report), so the quotient
    /// of the two functions this fix governs is the tangent the value is checked against: the
    /// shortcut made both halves the table's √2/2 and their quotient exactly 1, where the true
    /// tangent is 1 − 2.51e-31 (30 places) / 1 − 4.11e-101 (100 places).
    /// <para>
    /// The bound is the requested precision less one place: the QUOTIENT is a
    /// <see cref="Real.Divide"/> result, truncated at the ambient computation cap, so its own error
    /// is ~10^-n and dominates.  Here too the teeth are the inequality against 1 and the exactness of
    /// the two halves; the 30-place case is the one where the bound also separates the answers
    /// (2.51e-31 &gt; 10^-31) — at 40 and 100 places the quotient's truncation is the same size as
    /// the table's distance from the truth.
    /// </para>
    /// </summary>
    [Theory]
    [InlineData(30L, 31L, TrueTanQuarterP30)]
    [InlineData(40L, 40L, TrueTanQuarterP40)]
    [InlineData(100L, 100L, TrueTanQuarterP100)]
    public void Tan_OfQuarterTruncatedPi_IsNotTheTablesOne(long places, long tolerancePlaces, string trueValue)
    {
        using var scope = Real.WithPrecision(places, places);
        Real quarter = PiTruncatedAt(places) / new Real("4");
        Real sin = Real.Sin(quarter);
        Real cos = Real.Cos(quarter);
        Real tangent = Real.Divide(sin, cos);

        Assert.False(sin.IsExact);
        Assert.False(cos.IsExact);
        Assert.NotEqual(Real.One, tangent);
        AssertAgreesWithGroundTruth(tangent, trueValue, tolerancePlaces, $"tan(pi({places})/4)");
    }

    // ---------------------------------------------------------------- the other half: exact inputs

    /// <summary>An EXACT argument that is genuinely the angle keeps its exact value — and the
    /// exactness of the result is itself the observable proof that the table, not the series,
    /// answered: the general path marks its result inexact even for zero (<c>SinTaylor(0)</c> is
    /// passed through <see cref="Real.AsInexact"/>'s sibling in the series branch), so an exact zero
    /// can only come from the table.</summary>
    [Fact]
    public void Sin_GivenExactZero_IsExactlyZeroFromTheTable()
    {
        Real actual = Real.Sin(Real.Zero);

        Assert.Equal(Real.Zero, actual);
        Assert.True(actual.IsExact, "sin(0) must stay exact — the series branch marks its result inexact");
    }

    /// <summary>Likewise <c>cos(0) = 1</c>, exactly.</summary>
    [Fact]
    public void Cos_GivenExactZero_IsExactlyOneFromTheTable()
    {
        Real actual = Real.Cos(Real.Zero);

        Assert.Equal(Real.One, actual);
        Assert.True(actual.IsExact, "cos(0) must stay exact");
    }

    /// <summary>Exact arguments that are NOT special angles keep the general path's answer: the
    /// digits match the true value and the provenance stays inexact (a Taylor sum is a truncation).
    /// Nothing in the fix may move these.</summary>
    [Theory]
    [InlineData("1", TrueSinOne)]
    [InlineData("0.5", TrueSinHalf)]
    public void Sin_OfExactAngle_KeepsItsDigitsAndStaysHonest(string argument, string trueValue)
    {
        using var scope = Real.WithPrecision(1000, 40);
        Real actual = Real.Sin(Real.Parse(argument));

        Assert.False(actual.IsExact);
        AssertAgreesWithGroundTruth(actual, trueValue, 40L, $"sin({argument})");
    }

    [Theory]
    [InlineData("1", TrueCosOne)]
    [InlineData("0.5", TrueCosHalf)]
    public void Cos_OfExactAngle_KeepsItsDigitsAndStaysHonest(string argument, string trueValue)
    {
        using var scope = Real.WithPrecision(1000, 40);
        Real actual = Real.Cos(Real.Parse(argument));

        Assert.False(actual.IsExact);
        AssertAgreesWithGroundTruth(actual, trueValue, 40L, $"cos({argument})");
    }

    /// <summary>An exact rational angle (1/3, a periodic value) is exact and is not a special angle:
    /// the value is unchanged and the provenance stays honest.</summary>
    [Fact]
    public void Sin_OfExactRationalAngle_IsUnaffectedAndInexact()
    {
        using var scope = Real.WithPrecision(1000, 40);
        Real third = Real.One / new Real("3");

        Assert.True(third.IsExact);

        Real actual = Real.Sin(third);

        Assert.False(actual.IsExact);
        AssertAgreesWithGroundTruth(actual, TrueSinThird, 40L, "sin(1/3)");
    }

    // ---------------------------------------------------------------- the fast path is still fast

    /// <summary>
    /// The table is still consulted FIRST for an exact special angle, and answering from it costs
    /// nothing: 10 000 exact-zero calls of each function at 1000-place precision complete far inside
    /// the budget, which the series could not do (one Taylor run at that precision is milliseconds,
    /// so an implementation that bypassed the table would need minutes).  Measured with
    /// <see cref="Stopwatch"/> around the loop; the numbers are reported in the round-4 report.
    /// </summary>
    [Fact]
    [Trait("Category", "Timing")]
    public void ExactSpecialAngles_AreAnsweredFromTheTable_NotFromTheSeries()
    {
        using var scope = Real.WithPrecision(1000, 40);

        // Warm the JIT and the cached π.
        Assert.True(Real.Sin(Real.Zero).IsExact);
        Assert.True(Real.Cos(Real.Zero).IsExact);

        const int Calls = 10_000;
        var sw = Stopwatch.StartNew();
        bool allExact = true;
        for (int i = 0; i < Calls; i++)
        {
            allExact &= Real.Sin(Real.Zero).IsExact;
            allExact &= Real.Cos(Real.Zero).IsExact;
        }
        sw.Stop();

        Assert.True(allExact, "every exact-zero call must be answered by the table");
        Assert.True(sw.ElapsedMilliseconds < 5_000,
            $"{Calls} table hits for sin(0)/cos(0) took {sw.ElapsedMilliseconds} ms at 1000-place precision");
    }
}
