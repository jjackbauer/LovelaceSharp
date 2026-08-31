using Lovelace.Suite;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Suite.Tests;

/// <summary>
/// Functional tests for <see cref="Interpreter.Evaluate"/> — built-in function <c>sqrt(x)</c>.
/// Checklist items:
///   - Register "sqrt" in Interpreter.EvaluateCall switch
///   - BuiltinSqrt — exactly 1 argument; widen to Real; reject wrong arity
/// </summary>
public class InterpreterBuiltinSqrtTests
{
    private readonly Interpreter _evaluator = new();

    // -----------------------------------------------------------------------
    // Helper
    // -----------------------------------------------------------------------

    private static CallExpr SqrtCall(Expr argument) => new("sqrt", [argument]);

    // -----------------------------------------------------------------------
    // Test 19 — sqrt(Natural(4)) → Real(2)
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenSqrtOfNatural_ReturnsRealSquareRoot()
    {
        // LiteralExpr("4") evaluates to Natural(4); sqrt widens to Real and returns 2.
        var expr = SqrtCall(new LiteralExpr("4"));

        var result = await _evaluator.EvaluateAsync(expr);

        Assert.Equal(ValueKind.Real, result.Kind);
        Assert.Equal(new Rl("2"), result.AsReal());
    }

    // -----------------------------------------------------------------------
    // Test 20 — sqrt(Integer(16)) → Real(4)
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenSqrtOfPositiveInteger_ReturnsRealSquareRoot()
    {
        // Double-negate "16": Natural(16) → Integer(-16) → Integer(16).
        var posIntSixteen = new UnaryExpr(
            UnaryOp.Negate,
            new UnaryExpr(UnaryOp.Negate, new LiteralExpr("16")));
        var expr = SqrtCall(posIntSixteen);

        var result = await _evaluator.EvaluateAsync(expr);

        Assert.Equal(ValueKind.Real, result.Kind);
        Assert.Equal(new Rl("4"), result.AsReal());
    }

    // -----------------------------------------------------------------------
    // Test 21 — sqrt(Real("4.0")) → Real("2") via the Real argument path
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenSqrtOfReal_ReturnsRealSquareRoot()
    {
        // LiteralExpr("4.0") evaluates to Real (contains '.'); sqrt of a perfect square
        // converges in one Newton-Raphson step and returns exactly 2.
        var expr = SqrtCall(new LiteralExpr("4.0"));

        var result = await _evaluator.EvaluateAsync(expr);

        Assert.Equal(ValueKind.Real, result.Kind);
        Assert.Equal(new Rl("2"), result.AsReal());
    }

    // -----------------------------------------------------------------------
    // Test 22 — sqrt() with no arguments → InvalidOperationException
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenSqrtWithNoArguments_ThrowsInvalidOperationException()
    {
        var expr = new CallExpr("sqrt", []);

        await Assert.ThrowsAsync<InvalidOperationException>(async () => await _evaluator.EvaluateAsync(expr));
    }

    // -----------------------------------------------------------------------
    // Test 23 — sqrt(4, 9) with two arguments → InvalidOperationException
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenSqrtWithTooManyArguments_ThrowsInvalidOperationException()
    {
        var expr = new CallExpr("sqrt", [new LiteralExpr("4"), new LiteralExpr("9")]);

        await Assert.ThrowsAsync<InvalidOperationException>(async () => await _evaluator.EvaluateAsync(expr));
    }
}
