using Lovelace.Console.Repl;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Console.Tests;

/// <summary>
/// Functional tests for <see cref="Evaluator.Evaluate"/> — built-in function <c>pi()</c>.
/// Checklist items:
///   - Register "pi" in Evaluator.EvaluateCall switch
///   - BuiltinPi — 0 arguments path uses Real.DisplayDecimalPlaces
///   - BuiltinPi — 1 Natural or Integer argument uses its value as digit count
///   - BuiltinPi — reject Real argument with InvalidOperationException
///   - BuiltinPi — reject argument count ≠ 0 and ≠ 1 with InvalidOperationException
/// </summary>
public class EvaluatorBuiltinPiTests
{
    private readonly Evaluator _evaluator = new();

    // -----------------------------------------------------------------------
    // Test 24 — pi() with 0 arguments uses Real.DisplayDecimalPlaces
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenPiWithNoArguments_ReturnsRealWithDisplayDecimalPlacesDigits()
    {
        // Temporarily reduce DisplayDecimalPlaces to 5 so the test runs fast.
        long saved = Rl.DisplayDecimalPlaces;
        Rl.DisplayDecimalPlaces = 5;
        try
        {
            var expr = new CallExpr("pi", []);

            var result = _evaluator.Evaluate(expr);

            Assert.Equal(ValueKind.Real, result.Kind);
            Assert.Equal("3.14159", result.AsReal().ToString());
        }
        finally
        {
            Rl.DisplayDecimalPlaces = saved;
        }
    }

    // -----------------------------------------------------------------------
    // Test 25 — pi(Natural(10)) → first 10 fractional digits of π
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenPiWithNaturalArgument_ReturnsRealWithRequestedDigits()
    {
        // LiteralExpr("10") evaluates to Natural(10).
        var expr = new CallExpr("pi", [new LiteralExpr("10")]);

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Real, result.Kind);
        Assert.Equal("3.1415926535", result.AsReal().ToString());
    }

    // -----------------------------------------------------------------------
    // Test 26 — pi(Integer(10)) → same as Natural case
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenPiWithIntegerArgument_ReturnsRealWithRequestedDigits()
    {
        // Double-negate "10": Natural(10) → Integer(-10) → Integer(10).
        var posIntTen = new UnaryExpr(
            UnaryOp.Negate,
            new UnaryExpr(UnaryOp.Negate, new LiteralExpr("10")));
        var expr = new CallExpr("pi", [posIntTen]);

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Real, result.Kind);
        Assert.Equal("3.1415926535", result.AsReal().ToString());
    }

    // -----------------------------------------------------------------------
    // Test 27 — pi(Real) → InvalidOperationException
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenPiWithRealArgument_ThrowsInvalidOperationException()
    {
        // LiteralExpr("3.0") evaluates to Real; digit counts must be Natural or Integer.
        var expr = new CallExpr("pi", [new LiteralExpr("3.0")]);

        Assert.Throws<InvalidOperationException>(() => _evaluator.Evaluate(expr));
    }

    // -----------------------------------------------------------------------
    // Test 28 — pi(10, 20) with two arguments → InvalidOperationException
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenPiWithTooManyArguments_ThrowsInvalidOperationException()
    {
        var expr = new CallExpr("pi", [new LiteralExpr("10"), new LiteralExpr("20")]);

        Assert.Throws<InvalidOperationException>(() => _evaluator.Evaluate(expr));
    }
}
