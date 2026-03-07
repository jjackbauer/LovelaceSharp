using Lovelace.Console.Repl;
using Int = global::Lovelace.Integer.Integer;
using Nat = global::Lovelace.Natural.Natural;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Console.Tests;

/// <summary>
/// Tests for <see cref="Evaluator.Evaluate"/> — built-in function <c>abs(x)</c>.
/// (Test plan items 74–76.)
/// </summary>
public class EvaluatorBuiltinAbsTests
{
    private readonly Evaluator _evaluator = new();

    // -----------------------------------------------------------------------
    // Helper: build CallExpr("abs", singleArg)
    // -----------------------------------------------------------------------

    private static CallExpr AbsCall(Expr argument) =>
        new("abs", [argument]);

    // -----------------------------------------------------------------------
    // Test 74 — abs(Integer(-5)) → Integer(5)
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenAbsOfNegativeInteger_ReturnsPositiveInteger()
    {
        // UnaryExpr(Negate, LiteralExpr("5")) evaluates to Integer(-5)
        // because Natural does not support negation (widens to Integer first).
        var negFive = new UnaryExpr(UnaryOp.Negate, new LiteralExpr("5"));
        var expr = AbsCall(negFive);

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Integer, result.Kind);
        Assert.Equal(Int.Parse("5", null), result.AsInteger());
    }

    // -----------------------------------------------------------------------
    // Test 75 — abs(Natural(5)) → Natural(5)
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenAbsOfPositiveNatural_ReturnsSameNatural()
    {
        var expr = AbsCall(new LiteralExpr("5"));

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Natural, result.Kind);
        Assert.Equal(Nat.Parse("5", null), result.AsNatural());
    }

    // -----------------------------------------------------------------------
    // Test 76 — abs(Real("-3.14")) → Real("3.14")
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenAbsOfNegativeReal_ReturnsPositiveReal()
    {
        // UnaryExpr(Negate, LiteralExpr("3.14")) evaluates to Real(-3.14)
        // because the literal contains '.', producing a Real.
        var negReal = new UnaryExpr(UnaryOp.Negate, new LiteralExpr("3.14"));
        var expr = AbsCall(negReal);

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Real, result.Kind);
        Assert.Equal(Rl.Parse("3.14", null), result.AsReal());
    }
}
