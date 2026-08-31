using Lovelace.Suite;
using Int = global::Lovelace.Integer.Integer;

namespace Lovelace.Suite.Tests;

/// <summary>
/// Tests for <see cref="Interpreter.Evaluate"/> — built-in function <c>sign(x)</c>.
/// Widens the argument to at least <see cref="ValueKind.Integer"/>, reads
/// <see cref="Int.Sign"/>, and returns an <see cref="ValueKind.Integer"/> <see cref="Value"/>.
/// (Test plan items 84–86.)
/// </summary>
public class InterpreterBuiltinSignTests
{
    private readonly Interpreter _evaluator = new();

    // -----------------------------------------------------------------------
    // Helper
    // -----------------------------------------------------------------------

    private static CallExpr SignCall(Expr argument) =>
        new("sign", [argument]);

    // -----------------------------------------------------------------------
    // Test 84 — sign(Integer(-5)) → Integer(-1)
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenSignOfNegative_ReturnsNegativeOne()
    {
        // UnaryExpr(Negate, LiteralExpr("5")) evaluates to Integer(-5).
        var negFive = new UnaryExpr(UnaryOp.Negate, new LiteralExpr("5"));
        var expr = SignCall(negFive);

        var result = await _evaluator.EvaluateAsync(expr);

        Assert.Equal(ValueKind.Integer, result.Kind);
        Assert.Equal(Int.Parse("-1", null), result.AsInteger());
    }

    // -----------------------------------------------------------------------
    // Test 85 — sign(Integer(0)) → Integer(0)
    // LiteralExpr("0") → Natural(0); widened to Integer(0) by sign().
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenSignOfZero_ReturnsZero()
    {
        var expr = SignCall(new LiteralExpr("0"));

        var result = await _evaluator.EvaluateAsync(expr);

        Assert.Equal(ValueKind.Integer, result.Kind);
        Assert.Equal(Int.Parse("0", null), result.AsInteger());
    }

    // -----------------------------------------------------------------------
    // Test 86 — sign(Natural(5)) → Integer(1)
    // Natural(5) widens to Integer(5); .Sign is 1.
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenSignOfPositive_ReturnsOne()
    {
        var expr = SignCall(new LiteralExpr("5"));

        var result = await _evaluator.EvaluateAsync(expr);

        Assert.Equal(ValueKind.Integer, result.Kind);
        Assert.Equal(Int.Parse("1", null), result.AsInteger());
    }

    // -----------------------------------------------------------------------
    // Additional — sign(Integer(-100)) → Integer(-1)
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenSignOfLargeNegative_ReturnsNegativeOne()
    {
        var negHundred = new UnaryExpr(UnaryOp.Negate, new LiteralExpr("100"));
        var expr = SignCall(negHundred);

        var result = await _evaluator.EvaluateAsync(expr);

        Assert.Equal(ValueKind.Integer, result.Kind);
        Assert.Equal(Int.Parse("-1", null), result.AsInteger());
    }
}
