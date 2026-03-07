using Lovelace.Console.Repl;
using Int = global::Lovelace.Integer.Integer;
using Nat = global::Lovelace.Natural.Natural;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Console.Tests;

/// <summary>
/// Tests for <see cref="Evaluator.Evaluate"/> — postfix factorial (<c>!</c>).
/// (Test plan items 70–73.)
/// </summary>
public class EvaluatorFactorialTests
{
    private readonly Evaluator _evaluator = new();

    // -----------------------------------------------------------------------
    // Test 70 — Natural factorial → returns Natural
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenNaturalFactorial_ReturnsNatural()
    {
        // 5! = 120; LiteralExpr("5") evaluates to Natural(5).
        var expr = new PostfixExpr(new LiteralExpr("5"), PostfixOp.Factorial);

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Natural, result.Kind);
        Assert.Equal(Nat.Parse("120", null), result.AsNatural());
    }

    // -----------------------------------------------------------------------
    // Test 71 — Integer factorial → returns Integer
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenIntegerFactorial_ReturnsInteger()
    {
        // Produce Integer(5) via double-negation of Natural(5):
        //   -(-LiteralExpr("5")) → Integer(-5) then negated → Integer(5).
        var integerFive = new UnaryExpr(
            UnaryOp.Negate,
            new UnaryExpr(UnaryOp.Negate, new LiteralExpr("5")));
        var expr = new PostfixExpr(integerFive, PostfixOp.Factorial);

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Integer, result.Kind);
        Assert.Equal(Int.Parse("120", null), result.AsInteger());
    }

    // -----------------------------------------------------------------------
    // Test 72 — Real factorial → throws descriptive error
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenRealFactorial_ThrowsDescriptiveError()
    {
        // Real("3.14") — contains '.', so literal evaluates to Real.
        var expr = new PostfixExpr(new LiteralExpr("3.14"), PostfixOp.Factorial);

        var ex = Assert.Throws<InvalidOperationException>(() => _evaluator.Evaluate(expr));
        Assert.Contains("Real", ex.Message);
    }

    // -----------------------------------------------------------------------
    // Test 73 — Negative Integer factorial → throws InvalidOperationException
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenNegativeIntegerFactorial_ThrowsInvalidOperation()
    {
        // Produce Integer(-1) via UnaryExpr(Negate, LiteralExpr("1")).
        var integerMinusOne = new UnaryExpr(UnaryOp.Negate, new LiteralExpr("1"));
        var expr = new PostfixExpr(integerMinusOne, PostfixOp.Factorial);

        var ex = Assert.Throws<InvalidOperationException>(() => _evaluator.Evaluate(expr));
        Assert.Equal("Factorial is not defined for negative integers.", ex.Message);
    }
}
