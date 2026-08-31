using Lovelace.Suite;
using Int = global::Lovelace.Integer.Integer;
using Nat = global::Lovelace.Natural.Natural;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Suite.Tests;

/// <summary>
/// Tests for <see cref="Interpreter.Evaluate"/> — built-in function <c>abs(x)</c>.
/// (Test plan items 74–76.)
/// </summary>
public class InterpreterBuiltinAbsTests
{
    private readonly Interpreter _evaluator = new();

    // -----------------------------------------------------------------------
    // Helper: build CallExpr("abs", singleArg)
    // -----------------------------------------------------------------------

    private static CallExpr AbsCall(Expr argument) =>
        new("abs", [argument]);

    // -----------------------------------------------------------------------
    // Test 74 — abs(Integer(-5)) → Integer(5)
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenAbsOfNegativeInteger_ReturnsPositiveInteger()
    {
        // UnaryExpr(Negate, LiteralExpr("5")) evaluates to Integer(-5)
        // because Natural does not support negation (widens to Integer first).
        var negFive = new UnaryExpr(UnaryOp.Negate, new LiteralExpr("5"));
        var expr = AbsCall(negFive);

        var result = await _evaluator.EvaluateAsync(expr);

        Assert.Equal(ValueKind.Integer, result.Kind);
        Assert.Equal(Int.Parse("5", null), result.AsInteger());
    }

    // -----------------------------------------------------------------------
    // Test 75 — abs(Natural(5)) → Natural(5)
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenAbsOfPositiveNatural_ReturnsSameNatural()
    {
        var expr = AbsCall(new LiteralExpr("5"));

        var result = await _evaluator.EvaluateAsync(expr);

        Assert.Equal(ValueKind.Natural, result.Kind);
        Assert.Equal(Nat.Parse("5", null), result.AsNatural());
    }

    // -----------------------------------------------------------------------
    // Test 76 — abs(Real("-3.14")) → Real("3.14")
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenAbsOfNegativeReal_ReturnsPositiveReal()
    {
        // UnaryExpr(Negate, LiteralExpr("3.14")) evaluates to Real(-3.14)
        // because the literal contains '.', producing a Real.
        var negReal = new UnaryExpr(UnaryOp.Negate, new LiteralExpr("3.14"));
        var expr = AbsCall(negReal);

        var result = await _evaluator.EvaluateAsync(expr);

        Assert.Equal(ValueKind.Real, result.Kind);
        Assert.Equal(Rl.Parse("3.14", null), result.AsReal());
    }
}
