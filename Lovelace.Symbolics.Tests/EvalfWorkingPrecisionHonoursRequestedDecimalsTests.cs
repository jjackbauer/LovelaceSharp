using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// THE DIGIT COUNT A CALLER ASKS FOR IS THE DIGIT COUNT THEY GET.
///
/// <para>THE DEFECT THIS FILE PINS (cycle 6, audit finding F3). <c>evalf(f, N)</c> promises N
/// DECIMAL PLACES, but it ran the whole evaluation inside a precision scope whose computation
/// budget was <c>N</c> FRACTIONAL places. A value with <c>k</c> integer digits therefore carried
/// only about <c>N</c> significant digits of working room instead of <c>N + k</c>, and the
/// <c>k</c> decimal places nearest the point came out as noise. The trigger is an argument that
/// is NOT exactly representable at that budget — an exact rational whose decimal expansion does
/// not terminate — because the argument itself is materialised as a Real at the ambient budget
/// (<c>RationalReal.ToReal</c>), so an argument error of <c>10^-N</c> is amplified by the
/// derivative of the function (for <c>sinh</c>, by <c>cosh(x) ≈ 10^k</c>) into an error of
/// <c>10^(k-N)</c> in the result.</para>
///
/// <para>MEASURED BEFORE THE REPAIR, on the same three expressions used below: the digits agree
/// with the truth only through decimal <c>N - k</c>. <c>evalf(sinh(28/15), 30)</c> published
/// <c>3.156033243073207922905472664338</c> (truth ends <c>…339</c>), and
/// <c>evalf(sinh(34/3), 30)</c> published <c>41780.548053564065925188446077553412</c> against a
/// truth of <c>…567339</c> — wrong from the 26th decimal, exactly the five integer digits. The
/// 100-place call of the same expression is wrong from the 95th. A TERMINATING argument
/// (<c>sinh(11.5)</c>) is unaffected, which is what makes the trigger the argument's
/// representability and not the function or the ambient budget.</para>
///
/// <para>GROUND TRUTH. Every expected digit string below is a literal copied from mpmath 1.3.0,
/// computed at <c>mp.dps = 220</c> and truncated (not rounded) at the requested count —
/// <c>mp.nstr(mp.sinh(mpf(28)/15), 140)</c>, <c>mp.sinh(mpf(34)/3)</c>, <c>mp.sinh(mpf(65)/3)</c>
/// and <c>mp.sinh(mpf(23)/2)</c>, run with the repo's oracle interpreter
/// (<c>C:\Users\ricar\dev\.lovelace-tools\python</c>, mpmath 1.3.0). The three arguments are
/// chosen so the results carry 1, 5 and 10 integer digits: the loss tracks the magnitude, so a
/// repair that merely adds a fixed guard would pass one row and fail the others.</para>
///
/// <para>NOTHING HERE IS SATISFIED BY MOVING THE PRINTED DIGITS. The already-honoured requests
/// are asserted beside the defect (<c>sqrt(2)</c>, <c>1/3</c>, <c>pi</c>, <c>pi(30)</c> at 30
/// places, the exact half <c>1/2</c>, the exact integer <c>2</c>), and the values that fall below
/// the 1000-place cap are asserted to stay INEXACT rather than being promoted to exact
/// rationals.</para>
/// </summary>
public class EvalfWorkingPrecisionHonoursRequestedDecimalsTests
{
    /// <summary>One engine per case: the projection is the SAME one the wire publishes
    /// (<see cref="StructuredProjection"/>), so the digits and the flag are read off the published
    /// form and not off a re-derivation of it.</summary>
    private static (Value Value, StructuredValueDto Structured) Evaluate(string expression)
    {
        var engine = new SuiteEngine();
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        var value = engine.Evaluate(expression);
        return (value, StructuredProjection.ToStructured(value));
    }

    /// <summary>
    /// THE PROPERTY, over three magnitudes (1, 5 and 10 integer digits) and over four counts.
    /// The expected strings are the mpmath truncations described in the class comment; the
    /// 5-integer-digit row is the audit's own reproduction at 20, 30, 50 and 100 places.
    /// </summary>
    [Theory]
    // 1 integer digit — sinh(28/15) = 3.1560332430732079229054726643398808…
    [InlineData("sinh(28/15)", 30, "3.156033243073207922905472664339")]
    [InlineData("sinh(28/15)", 100,
        "3.1560332430732079229054726643398808098958171050319303402544132472926926925879526189618018339120690466")]
    // 5 integer digits — sinh(34/3) = 41780.5480535640659251884460775673391926…
    [InlineData("sinh(34/3)", 20, "41780.54805356406592518844")]
    [InlineData("sinh(34/3)", 30, "41780.548053564065925188446077567339")]
    [InlineData("sinh(34/3)", 50, "41780.54805356406592518844607756733919267583492738622536")]
    [InlineData("sinh(34/3)", 100,
        "41780.5480535640659251884460775673391926758349273862253644277306454458985775018289223615326471730513352561")]
    // 10 integer digits — sinh(65/3) = 1284351149.9657410942550053116749650793…
    [InlineData("sinh(65/3)", 30, "1284351149.965741094255005311674965079318")]
    [InlineData("sinh(65/3)", 100,
        "1284351149.9657410942550053116749650793184322151669392890793309161544689964010798237111274464943036288273900369")]
    // 145 integer digits — the case the fixed guard cannot cover, so the working precision has to be
    // re-sized from the value: sinh(1000/3) = 2909…06867.359401747177724438543006142556. Before the
    // repair not one of those 145 integer digits was right either (the published integer part was
    // …1319835464002395751158566363127121981327396…, wrong from its 31st digit).
    [InlineData("sinh(1000/3)", 30,
        "2909358940723497999622983496672289621777576895087778819744496982533636420191216688947898956693539789328530352713870924771820635012087408585306867.359401747177724438543006142556")]
    public void RequestedDecimalPlaces_AreTheDigitsDelivered(string body, int digits, string expected)
    {
        var expression = $"evalf({body}, {digits})";
        var (value, structured) = Evaluate(expression);

        Assert.Equal(ValueKind.Real, value.Kind);
        Assert.Equal(expected, structured.Value);

        // the delivered count is the requested count: the last decimal is part of the answer, so a
        // prefix of the truth that stops short of it is not a pass
        Assert.Equal(digits, DecimalsOf(structured.Value!));
        Assert.True(structured.Exact == false,
            $"{expression} is a transcendental of a non-terminating rational and must stay inexact");
    }

    /// <summary>
    /// THE CONTROL THAT KEEPS THE TRIGGER HONEST: the same function and the same magnitude with an
    /// argument whose decimal expansion TERMINATES. It was already correct before the repair (the
    /// argument is representable at any budget, so no argument error is amplified), and it is the
    /// reason the defect is about the argument's representability rather than about <c>sinh</c>.
    /// </summary>
    [Theory]
    [InlineData(30, "49357.885500315201914739778692889335")]
    [InlineData(100,
        "49357.8855003152019147397786928893350025676060913315150701208567110467729030338740090072170635948405117304")]
    public void TerminatingArgument_KeepsDeliveringItsDigits(int digits, string expected)
    {
        var (value, structured) = Evaluate($"evalf(sinh(11.5), {digits})");

        Assert.Equal(ValueKind.Real, value.Kind);
        Assert.Equal(expected, structured.Value);
        Assert.Equal(digits, DecimalsOf(structured.Value!));
        Assert.True(structured.Exact == false,
            "sinh(11.5) is still irrational and must not claim exactness");
    }

    /// <summary>
    /// THE REQUESTS THE DEFECTIVE TREE ALREADY HONOURED, pinned so the repair cannot buy the
    /// trailing decimals by moving the leading ones. The <c>pi(30)</c> row is the case whose VALUE
    /// was explicitly pinned by <c>EvalfTruncatedConstantExactnessTests</c>; it is repeated here
    /// because the working-precision scope feeds it too.
    /// </summary>
    [Theory]
    [InlineData("sqrt(2)", 5, "1.41421")]
    [InlineData("sqrt(2)", 30, "1.414213562373095048801688724209")]
    [InlineData("1/3", 5, "0.33333")]
    [InlineData("1/3", 30, "0.333333333333333333333333333333")]
    [InlineData("pi", 30, "3.141592653589793238462643383279")]
    [InlineData("pi(30)", 30, "3.141592653589793238462643383279")]
    [InlineData("pi(30)", 40, "3.141592653589793238462643383279")]
    [InlineData("e(30)", 29, "2.71828182845904523536028747135")]
    public void AlreadyHonouredRequests_KeepTheirValues(string body, int digits, string expected)
    {
        var (value, structured) = Evaluate($"evalf({body}, {digits})");

        Assert.Equal(ValueKind.Real, value.Kind);
        Assert.Equal(expected, structured.Value);
    }

    /// <summary>
    /// The EXACT controls, read the same way, and on the same path: the working precision may rise,
    /// but an exact half must stay the exact rational 1/2 (no padding to the requested count, no
    /// rational form flattened into a decimal) and an exact integer must stay an Integer.
    /// </summary>
    [Fact]
    public void ExactControls_KeepTheirTierAndTheirValue()
    {
        var (half, halfStructure) = Evaluate("evalf(1/2, 40)");
        Assert.Equal(ValueKind.Real, half.Kind);
        Assert.Equal("0.5", halfStructure.Value);
        Assert.True(halfStructure.Exact, "1/2 is exact and must stay exact");
        Assert.Equal("1", halfStructure.Numerator);
        Assert.Equal("2", halfStructure.Denominator);

        var (two, twoStructure) = Evaluate("evalf(2, 30)");
        Assert.Equal(ValueKind.Integer, two.Kind);
        Assert.Equal("2", twoStructure.Value);
        Assert.True(twoStructure.Exact);
    }

    /// <summary>
    /// BELOW THE 1000-PLACE CAP, AND NOT CLAIMING OTHERWISE. A magnitude whose leading zeros exceed
    /// every digit the computation may carry is correctly 0 at the requested resolution — but the
    /// route that produced it is a truncation, so the answer must stay INEXACT and must publish no
    /// rational form. Raising the working precision must not turn these into exact zeros.
    /// </summary>
    [Fact]
    public void ValuesBelowTheComputationCap_StayInexact()
    {
        foreach (var expression in new[] { "evalf(1/(3*10^1000), 30)", "evalf(1/(3*10^1001), 30)",
                                           "evalf(sin(pi(30)), 40)" })
        {
            var (value, structured) = Evaluate(expression);

            Assert.Equal(ValueKind.Real, value.Kind);
            Assert.Equal("0", structured.Value);
            Assert.True(structured.Exact == false,
                expression + " is a truncation's zero and must not claim exactness");
            Assert.Null(structured.Numerator);
            Assert.Null(structured.Denominator);
        }
    }

    /// <summary>The decimal places a published rendering carries — 0 for an integer rendering.</summary>
    private static int DecimalsOf(string rendering)
    {
        int dot = rendering.IndexOf('.');
        return dot < 0 ? 0 : rendering.Length - dot - 1;
    }
}
