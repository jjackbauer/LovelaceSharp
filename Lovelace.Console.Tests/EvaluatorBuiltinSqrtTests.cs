using Lovelace.Console.Repl;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Console.Tests;

/// <summary>
/// Functional tests for <see cref="Evaluator.Evaluate"/> — built-in function <c>sqrt(x)</c>.
/// Checklist items:
///   - Register "sqrt" in Evaluator.EvaluateCall switch
///   - BuiltinSqrt — exactly 1 argument; widen to Real; reject wrong arity
/// </summary>
public class EvaluatorBuiltinSqrtTests
{
    private readonly Evaluator _evaluator = new();

    // -----------------------------------------------------------------------
    // Helper
    // -----------------------------------------------------------------------

    private static CallExpr SqrtCall(Expr argument) => new("sqrt", [argument]);

    // -----------------------------------------------------------------------
    // Test 19 — sqrt(Natural(4)) → Real(2)
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenSqrtOfNatural_ReturnsRealSquareRoot()
    {
        // LiteralExpr("4") evaluates to Natural(4); sqrt widens to Real and returns 2.
        var expr = SqrtCall(new LiteralExpr("4"));

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Real, result.Kind);
        Assert.Equal(new Rl("2"), result.AsReal());
    }

    // -----------------------------------------------------------------------
    // Test 20 — sqrt(Integer(16)) → Real(4)
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenSqrtOfPositiveInteger_ReturnsRealSquareRoot()
    {
        // Double-negate "16": Natural(16) → Integer(-16) → Integer(16).
        var posIntSixteen = new UnaryExpr(
            UnaryOp.Negate,
            new UnaryExpr(UnaryOp.Negate, new LiteralExpr("16")));
        var expr = SqrtCall(posIntSixteen);

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Real, result.Kind);
        Assert.Equal(new Rl("4"), result.AsReal());
    }

    // -----------------------------------------------------------------------
    // Test 21 — sqrt(Real("4.0")) → Real("2") via the Real argument path
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenSqrtOfReal_ReturnsRealSquareRoot()
    {
        // LiteralExpr("4.0") evaluates to Real (contains '.'); sqrt of a perfect square
        // converges in one Newton-Raphson step and returns exactly 2.
        var expr = SqrtCall(new LiteralExpr("4.0"));

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Real, result.Kind);
        Assert.Equal(new Rl("2"), result.AsReal());
    }

    // -----------------------------------------------------------------------
    // Test 22 — sqrt() with no arguments → InvalidOperationException
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenSqrtWithNoArguments_ThrowsInvalidOperationException()
    {
        var expr = new CallExpr("sqrt", []);

        Assert.Throws<InvalidOperationException>(() => _evaluator.Evaluate(expr));
    }

    // -----------------------------------------------------------------------
    // Test 23 — sqrt(4, 9) with two arguments → InvalidOperationException
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenSqrtWithTooManyArguments_ThrowsInvalidOperationException()
    {
        var expr = new CallExpr("sqrt", [new LiteralExpr("4"), new LiteralExpr("9")]);

        Assert.Throws<InvalidOperationException>(() => _evaluator.Evaluate(expr));
    }
}
