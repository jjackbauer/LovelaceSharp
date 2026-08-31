using Lovelace.Suite;
using Nat = global::Lovelace.Natural.Natural;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Suite.Tests;

/// <summary>
/// Tests for <see cref="Interpreter.Evaluate"/> — literal evaluation with type inference.
/// (Test plan items 45–47.)
/// </summary>
public class InterpreterLiteralTests
{
    private readonly Interpreter _evaluator = new();

    // -----------------------------------------------------------------------
    // Whole number literal → Natural (Test 45)
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenWholeNumberLiteral_ReturnsNaturalValue()
    {
        var expr = new LiteralExpr("42");

        var result = await _evaluator.EvaluateAsync(expr);

        Assert.Equal(ValueKind.Natural, result.Kind);
        Assert.Equal(Nat.Parse("42", null), result.AsNatural());
    }

    // -----------------------------------------------------------------------
    // Decimal literal (contains '.') → Real (Test 46)
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenDecimalLiteral_ReturnsRealValue()
    {
        var expr = new LiteralExpr("3.14");

        var result = await _evaluator.EvaluateAsync(expr);

        Assert.Equal(ValueKind.Real, result.Kind);
        Assert.Equal(Rl.Parse("3.14", null), result.AsReal());
    }

    // -----------------------------------------------------------------------
    // Periodic literal (contains '(') → Real (Test 47)
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenPeriodicLiteral_ReturnsRealValue()
    {
        var expr = new LiteralExpr("0.(3)");

        var result = await _evaluator.EvaluateAsync(expr);

        Assert.Equal(ValueKind.Real, result.Kind);
        Assert.Equal(Rl.Parse("0.(3)", null), result.AsReal());
    }
}
