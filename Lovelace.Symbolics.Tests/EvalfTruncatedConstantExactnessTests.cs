using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// A CONSTANT THAT IS ALREADY A TRUNCATION MUST NOT CROSS AS AN EXACT RATIONAL.
///
/// <para>THE DEFECT THIS FILE PINS (cycle 6, row 11). <c>pi(n)</c>/<c>e(n)</c> answer an INEXACT
/// Real — <see cref="Lovelace.Real.Real.IsExact"/> is false, and the wire says so for
/// <c>pi(30)</c> itself. When such a value is handed to <c>evalf</c> it arrives as an already-numeric
/// payload, and <c>SymbolicsPlugin.NumericAtRequestedPrecision</c> projects it onto the exact
/// RATIONAL tier (<c>Exprs.Rational(RationalReal.FromReal(r))</c>) to re-materialise it at the
/// requested digit count. The rational tier has no provenance to lose, so the value came back out
/// of <c>NumToPayload</c> as a fresh Real whose exactness was read off the WIDTH of its rendering:
/// <c>evalf(pi(30), 40)</c> published
/// <c>{"kind":"Real","value":"3.141592653589793238462643383279","exact":true,
/// "numerator":"3141592653589793238462643383279","denominator":"1000000000000000000000000000000"}</c>,
/// <c>evalf(pi(1), 40)</c> published <c>3.1</c> exact with <c>31/10</c>, and
/// <c>evalf(e(30), 40)</c> published the same shape — while <c>pi(30)</c>/<c>e(30)</c> beside them
/// answer <c>exact:false</c>. A truncation of an inexact constant cannot be exact: the digits are a
/// prefix of the constant, not the constant.</para>
///
/// <para>THE VALUE IS NOT THE SUBJECT OF THIS ROUND. Every case below pins the DIGITS it published
/// before the repair as well as the flag, so a repair that "fixed" the claim by changing the number
/// fails here. The request is honoured exactly as it was: a count narrower than the constant
/// truncates to that count, a count wider than it leaves the constant's own digits alone.</para>
///
/// <para>NOTHING HERE IS SATISFIED BY CLEARING THE FLAG EVERYWHERE. The exact controls are asserted
/// beside the leak — <c>evalf(1/2, 40)</c> with numerator 1 and denominator 2, <c>2+3</c>,
/// <c>sin(0)</c>, and <c>evalf(sqrt(2), 40)</c> which is inexact and carries no rational form.</para>
/// </summary>
public class EvalfTruncatedConstantExactnessTests
{
    /// <summary>One engine per case: the projection is the SAME one the wire publishes
    /// (<see cref="StructuredProjection"/>), so "exact:false with no numerator/denominator" is
    /// asserted on the published form and not on a re-derivation of it.</summary>
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
    /// The cross product of (truncated constant, requested count), with the VALUE each case
    /// published before the repair — measured on the defective tree and pinned here so the flag is
    /// the only thing that may move.
    /// </summary>
    [Theory]
    [InlineData("pi(1)", 1, "3.1")]
    [InlineData("pi(1)", 5, "3.1")]
    [InlineData("pi(1)", 40, "3.1")]
    [InlineData("pi(30)", 5, "3.14159")]
    [InlineData("pi(30)", 29, "3.14159265358979323846264338327")]
    [InlineData("pi(30)", 30, "3.141592653589793238462643383279")]
    [InlineData("pi(30)", 40, "3.141592653589793238462643383279")]
    [InlineData("pi(30)", 100, "3.141592653589793238462643383279")]
    [InlineData("e(1)", 5, "2.7")]
    [InlineData("e(1)", 40, "2.7")]
    [InlineData("e(30)", 7, "2.7182818")]
    [InlineData("e(30)", 29, "2.71828182845904523536028747135")]
    [InlineData("e(30)", 40, "2.718281828459045235360287471352")]
    [InlineData("e(30)", 100, "2.718281828459045235360287471352")]
    public void TruncatedConstant_IsNeverPublishedAsAnExactRational(string constant, int digits, string expected)
    {
        var (value, structured) = Evaluate($"evalf({constant}, {digits})");

        Assert.Equal(ValueKind.Real, value.Kind);
        Assert.Equal(expected, structured.Value);
        Assert.True(structured.Exact == false,
            $"evalf({constant}, {digits}) is a truncation of an inexact constant and must not claim " +
            $"exactness; published {structured.Value} with exact={structured.Exact}");
        Assert.Null(structured.Numerator);
        Assert.Null(structured.Denominator);
        Assert.False(value.AsReal().IsExact);
    }

    /// <summary>
    /// THE CONTROL THAT KEEPS THE VALUE FIXED: a request WIDER than the constant's own digit count
    /// must still answer exactly the digits that constant carries. <c>pi(30)</c> at 40 places is
    /// the 30-place truncation, digit for digit — the repair may move the claim and nothing else.
    /// The identity is asserted against the constant's own value, and the premise (the constant is
    /// itself a truncation) is asserted first so the case cannot pass vacuously.
    /// </summary>
    [Theory]
    [InlineData("pi", 1, 5)]
    [InlineData("pi", 1, 40)]
    [InlineData("pi", 30, 40)]
    [InlineData("pi", 30, 100)]
    [InlineData("e", 1, 40)]
    [InlineData("e", 30, 40)]
    [InlineData("e", 30, 100)]
    public void RequestWiderThanTheConstantsOwnCount_CarriesExactlyTheConstantsDigits(string name, int ownDigits, int requested)
    {
        var (constant, constantStructure) = Evaluate($"{name}({ownDigits})");
        var (evaluated, evaluatedStructure) = Evaluate($"evalf({name}({ownDigits}), {requested})");

        Assert.Equal(ValueKind.Real, constant.Kind);
        Assert.False(constantStructure.Exact);
        Assert.True(constantStructure.Numerator is null && constantStructure.Denominator is null,
            $"{name}({ownDigits}) is a truncation and must not publish a rational form");

        Assert.Equal(constantStructure.Value, evaluatedStructure.Value);
        Assert.Equal(constant.AsReal(), evaluated.AsReal());
        Assert.True(evaluatedStructure.Exact == false,
            $"evalf({name}({ownDigits}), {requested}) = {evaluatedStructure.Value} claims exactness");
        Assert.Null(evaluatedStructure.Numerator);
        Assert.Null(evaluatedStructure.Denominator);
    }

    /// <summary>
    /// The exact controls, read the same way. These are the values that must NOT be swept up by the
    /// repair: an exact rational keeps its numerator/denominator, integer arithmetic stays exact,
    /// the exact <c>sin(0)</c> stays exact, and the truncated <c>sqrt(2)</c> stays inexact with no
    /// rational form.
    /// </summary>
    [Fact]
    public void ExactControls_AreUntouched()
    {
        var (half, halfStructure) = Evaluate("evalf(1/2, 40)");
        Assert.Equal(ValueKind.Real, half.Kind);
        Assert.Equal("0.5", halfStructure.Value);
        Assert.True(halfStructure.Exact);
        Assert.Equal("1", halfStructure.Numerator);
        Assert.Equal("2", halfStructure.Denominator);

        var (sum, sumStructure) = Evaluate("2+3");
        Assert.True(sumStructure.Exact, "2+3 must stay exact");
        Assert.Equal("5", sumStructure.Value);

        var (sinZero, sinZeroStructure) = Evaluate("sin(0)");
        Assert.Equal(ValueKind.Symbolic, sinZero.Kind);
        Assert.True(sinZeroStructure.Exact, "sin(0) must stay exact");
        Assert.Equal("0", sinZeroStructure.Pretty);

        var (sqrtTwo, sqrtTwoStructure) = Evaluate("evalf(sqrt(2), 40)");
        Assert.Equal(ValueKind.Real, sqrtTwo.Kind);
        Assert.Equal("1.4142135623730950488016887242096980785696", sqrtTwoStructure.Value);
        Assert.True(sqrtTwoStructure.Exact == false, "the truncated sqrt(2) must stay inexact");
        Assert.Null(sqrtTwoStructure.Numerator);
        Assert.Null(sqrtTwoStructure.Denominator);
    }
}
