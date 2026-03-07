using Lovelace.Console.Repl;

namespace Lovelace.Console.Tests;

/// <summary>
/// Tests for <see cref="Evaluator.Evaluate"/> — built-in functions
/// <c>is_even(x)</c> and <c>is_odd(x)</c>, which call the static
/// <c>IsEvenInteger</c> / <c>IsOddInteger</c> methods on the inner numeric
/// type and return a <see cref="ValueKind.Boolean"/> <see cref="Value"/>.
/// (Test plan items 82–83.)
/// </summary>
public class EvaluatorBuiltinIsEvenIsOddTests
{
    private readonly Evaluator _evaluator = new();

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    private static CallExpr IsEvenCall(Expr argument) =>
        new("is_even", [argument]);

    private static CallExpr IsOddCall(Expr argument) =>
        new("is_odd", [argument]);

    // -----------------------------------------------------------------------
    // Test 82 — is_even(Natural(4)) → Boolean(true)
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenIsEvenOfEvenNumber_ReturnsTrue()
    {
        var expr = IsEvenCall(new LiteralExpr("4"));

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Boolean, result.Kind);
        Assert.True(result.AsBoolean());
    }

    // -----------------------------------------------------------------------
    // Additional: is_even(Natural(3)) → Boolean(false)
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenIsEvenOfOddNumber_ReturnsFalse()
    {
        var expr = IsEvenCall(new LiteralExpr("3"));

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Boolean, result.Kind);
        Assert.False(result.AsBoolean());
    }

    // -----------------------------------------------------------------------
    // Additional: is_even(Integer(-4)) → Boolean(true)
    // Integer(-4) is even.
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenIsEvenOfEvenNegativeInteger_ReturnsTrue()
    {
        // UnaryExpr(Negate, LiteralExpr("4")) evaluates to Integer(-4).
        var negFour = new UnaryExpr(UnaryOp.Negate, new LiteralExpr("4"));
        var expr = IsEvenCall(negFour);

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Boolean, result.Kind);
        Assert.True(result.AsBoolean());
    }

    // -----------------------------------------------------------------------
    // Test 83 — is_odd(Natural(3)) → Boolean(true)
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenIsOddOfOddNumber_ReturnsTrue()
    {
        var expr = IsOddCall(new LiteralExpr("3"));

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Boolean, result.Kind);
        Assert.True(result.AsBoolean());
    }

    // -----------------------------------------------------------------------
    // Additional: is_odd(Natural(4)) → Boolean(false)
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenIsOddOfEvenNumber_ReturnsFalse()
    {
        var expr = IsOddCall(new LiteralExpr("4"));

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Boolean, result.Kind);
        Assert.False(result.AsBoolean());
    }

    // -----------------------------------------------------------------------
    // Additional: is_odd(Integer(-3)) → Boolean(true)
    // Integer(-3) is odd.
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenIsOddOfOddNegativeInteger_ReturnsTrue()
    {
        // UnaryExpr(Negate, LiteralExpr("3")) evaluates to Integer(-3).
        var negThree = new UnaryExpr(UnaryOp.Negate, new LiteralExpr("3"));
        var expr = IsOddCall(negThree);

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Boolean, result.Kind);
        Assert.True(result.AsBoolean());
    }
}
