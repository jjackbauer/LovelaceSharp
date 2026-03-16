using Lovelace.Console.Repl;
using Int = global::Lovelace.Integer.Integer;
using Nat = global::Lovelace.Natural.Natural;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Console.Tests;

/// <summary>
/// Tests for <see cref="Evaluator.Evaluate"/> — unary negation and unary plus.
/// (Test plan items 66–69.)
/// </summary>
public class EvaluatorUnaryTests
{
    private readonly Evaluator _evaluator = new();

    // -----------------------------------------------------------------------
    // Test 66 — Negate Natural → widens to Integer, then negates
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenNegateNatural_WidensToIntegerAndNegates()
    {
        // UnaryOp.Negate applied to LiteralExpr("5") which evaluates to Natural(5).
        var expr = new UnaryExpr(UnaryOp.Negate, new LiteralExpr("5"));

        var result = await _evaluator.EvaluateAsync(expr);

        Assert.Equal(ValueKind.Integer, result.Kind);
        // Natural(5) widened to Integer(5), then negated → Integer(-5).
        Assert.Equal(Int.Parse("-5", null), result.AsInteger());
    }

    // -----------------------------------------------------------------------
    // Test 67 — Negate Integer → returns negated Integer (no additional widening)
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenNegateInteger_ReturnsNegatedInteger()
    {
        // Double-negate Natural(3): inner negate produces Integer(-3),
        // outer negate produces Integer(3) — exercising the Integer negation path.
        var innerNegate = new UnaryExpr(UnaryOp.Negate, new LiteralExpr("3"));
        var outerNegate = new UnaryExpr(UnaryOp.Negate, innerNegate);

        var result = await _evaluator.EvaluateAsync(outerNegate);

        Assert.Equal(ValueKind.Integer, result.Kind);
        Assert.Equal(Int.Parse("3", null), result.AsInteger());
    }

    // -----------------------------------------------------------------------
    // Test 68 — Negate Real → returns negated Real
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenNegateReal_ReturnsNegatedReal()
    {
        var expr = new UnaryExpr(UnaryOp.Negate, new LiteralExpr("3.14"));

        var result = await _evaluator.EvaluateAsync(expr);

        Assert.Equal(ValueKind.Real, result.Kind);
        Assert.Equal(Rl.Parse("-3.14", null), result.AsReal());
    }

    // -----------------------------------------------------------------------
    // Test 69 — Unary plus → returns the same Value unchanged
    // -----------------------------------------------------------------------

    [Theory]
    [InlineData("5")]
    [InlineData("0")]
    public async Task Evaluate_GivenUnaryPlusOnNatural_ReturnsSameValue(string rawText)
    {
        var literal = new LiteralExpr(rawText);
        var expected = await _evaluator.EvaluateAsync(literal);

        var result = await new Evaluator().EvaluateAsync(new UnaryExpr(UnaryOp.Plus, literal));

        Assert.Equal(expected.Kind, result.Kind);
        Assert.Equal(expected.AsNatural(), result.AsNatural());
    }

    [Fact]
    public async Task Evaluate_GivenUnaryPlusOnReal_ReturnsSameValue()
    {
        var expected = await _evaluator.EvaluateAsync(new LiteralExpr("2.71"));

        var result = await _evaluator.EvaluateAsync(new UnaryExpr(UnaryOp.Plus, new LiteralExpr("2.71")));

        Assert.Equal(ValueKind.Real, result.Kind);
        Assert.Equal(expected.AsReal(), result.AsReal());
    }
}
