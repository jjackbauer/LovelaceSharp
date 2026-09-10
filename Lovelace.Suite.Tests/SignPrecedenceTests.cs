using Lovelace.Suite;
using Lovelace.Symbolics;
using Int = global::Lovelace.Integer.Integer;

namespace Lovelace.Suite.Tests;

/// <summary>
/// Sign/operator precedence (Cycle 2, defect N1). A leading unary sign binds looser than
/// <c>^</c> — the mathematical convention, so <c>-2^2</c> is −4, not 4 — but tighter than
/// the range operator, so <c>-1..2</c> stays a range from −1 to 2. Before this fix
/// <c>-x^2</c> parsed as <c>(-x)^2</c>, which silently changed the sign of every even power
/// of a negated base (including <c>exp(-x^2)</c>).
/// </summary>
public class SignPrecedenceTests
{
    private static Expr Parse(string input) => new Parser().Parse(new Tokenizer().Tokenize(input));

    /// <summary>Evaluates <paramref name="source"/> and returns the engine's rendered value,
    /// which is type-agnostic across the Natural/Integer widening the language applies.</summary>
    private static async Task<string> EvalText(string source)
    {
        var engine = new SuiteEngine();
        return engine.FormatValue(await engine.EvaluateAsync(source));
    }

    // -----------------------------------------------------------------------
    // AST shape
    // -----------------------------------------------------------------------

    [Fact]
    public void Parse_NegatedPower_SignWrapsThePower()
    {
        var expr = Parse("-x^2");
        var unary = Assert.IsType<UnaryExpr>(expr);
        Assert.Equal(UnaryOp.Negate, unary.Op);
        var power = Assert.IsType<BinaryExpr>(unary.Operand);
        Assert.Equal(BinaryOp.Power, power.Op);
        Assert.Equal("x", Assert.IsType<VariableExpr>(power.Left).Name);
        Assert.Equal("2", Assert.IsType<LiteralExpr>(power.Right).RawText);
    }

    [Fact]
    public void Parse_ParenthesisedNegatedBase_PowerAppliesToTheNegation()
    {
        var expr = Parse("(-x)^2");
        var power = Assert.IsType<BinaryExpr>(expr);
        Assert.Equal(BinaryOp.Power, power.Op);
        var unary = Assert.IsType<UnaryExpr>(power.Left);
        Assert.Equal(UnaryOp.Negate, unary.Op);
    }

    [Fact]
    public void Parse_NegativeExponent_ParsesAsSignedPower()
    {
        var expr = Parse("2^-1");
        var power = Assert.IsType<BinaryExpr>(expr);
        Assert.Equal(BinaryOp.Power, power.Op);
        var exponent = Assert.IsType<UnaryExpr>(power.Right);
        Assert.Equal(UnaryOp.Negate, exponent.Op);
    }

    [Fact]
    public void Parse_SignedRangeStart_IsNotANegatedRange()
    {
        var expr = Parse("-1..2");
        var range = Assert.IsType<RangeExpr>(expr);
        var first = Assert.IsType<UnaryExpr>(range.Start);
        Assert.Equal(UnaryOp.Negate, first.Op);
        Assert.Equal("1", Assert.IsType<LiteralExpr>(first.Operand).RawText);
        Assert.Equal("2", Assert.IsType<LiteralExpr>(range.End!).RawText);
    }

    [Fact]
    public void Parse_RangeStillBindsTighterThanPower()
    {
        var expr = Parse("1..10^2");
        var power = Assert.IsType<BinaryExpr>(expr);
        Assert.Equal(BinaryOp.Power, power.Op);
        Assert.IsType<RangeExpr>(power.Left);
    }

    [Fact]
    public void Parse_PostfixStillBindsTighterThanSign()
    {
        var expr = Parse("-5!");
        var unary = Assert.IsType<UnaryExpr>(expr);
        Assert.Equal(UnaryOp.Negate, unary.Op);
        Assert.Equal(PostfixOp.Factorial, Assert.IsType<PostfixExpr>(unary.Operand).Op);
    }

    // -----------------------------------------------------------------------
    // Evaluation
    // -----------------------------------------------------------------------

    [Theory]
    [InlineData("-2^2", "-4")]
    [InlineData("-2^3", "-8")]
    [InlineData("(-2)^2", "4")]
    [InlineData("0-2^2", "-4")]
    [InlineData("10 - 2^2", "6")]
    [InlineData("2^3^2", "512")]
    [InlineData("-5!", "-120")]
    [InlineData("+2^2", "4")]
    public async Task Evaluate_SignAndPowerFollowMathConvention(string source, string expected)
    {
        Assert.Equal(expected, await EvalText(source));
    }

    [Fact]
    public async Task Evaluate_SymbolicNegatedPower_HasNegativeValueAtTheSamplePoint()
    {
        // the end-to-end regression: -x^2 at x = 3 must be -9 (it used to be 9)
        var engine = new SuiteEngine();
        engine.LoadPlugin(new SymbolicsPlugin());
        await engine.EvaluateAsync("x = symbol(\"x\")");
        var value = await engine.EvaluateAsync("subs(-x^2, x, 3)");
        Assert.Equal("-9", Printing.PrettyPrint(value.AsSymbolic()));
    }

    [Fact]
    public async Task Evaluate_SymbolicCanonicalForm_KeepsTheSignOutsideThePower()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new SymbolicsPlugin());
        await engine.EvaluateAsync("x = symbol(\"x\")");
        var value = await engine.EvaluateAsync("-x^2");
        var canonical = Printing.CanonicalPrint(value.AsSymbolic());
        // (mul (rat -1 1) (pow (sym x) (rat 2 1))) — the negation wraps the power
        Assert.Contains("(pow (sym x) (rat 2 1))", canonical);
        Assert.DoesNotContain("(pow (mul", canonical);
    }
}
