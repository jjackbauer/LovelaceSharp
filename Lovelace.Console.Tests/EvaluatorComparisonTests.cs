using Lovelace.Console.Repl;
using Nat = global::Lovelace.Natural.Natural;
using Int = global::Lovelace.Integer.Integer;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Console.Tests;

/// <summary>
/// Tests for <see cref="Evaluator.Evaluate"/> — comparison operators
/// (==, !=, &gt;, &lt;, &gt;=, &lt;=) returning Boolean <see cref="Value"/>.
/// (Test plan items 61–65.)
/// </summary>
public class EvaluatorComparisonTests
{
    private readonly Evaluator _evaluator = new();

    // Helper: build a BinaryExpr from two literal strings and an operator.
    private static BinaryExpr Bin(string left, BinaryOp op, string right) =>
        new(new LiteralExpr(left), op, new LiteralExpr(right));

    // -----------------------------------------------------------------------
    // Test 61 — Natural(5) == Natural(5) → Boolean(true)
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenEqualValues_DoubleEqualsReturnsTrue()
    {
        var expr = Bin("5", BinaryOp.Equal, "5");

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Boolean, result.Kind);
        Assert.True(result.AsBoolean());
    }

    // -----------------------------------------------------------------------
    // Test 62 — Natural(3) != Natural(5) → Boolean(true)
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenDifferentValues_NotEqualsReturnsTrue()
    {
        var expr = Bin("3", BinaryOp.NotEqual, "5");

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Boolean, result.Kind);
        Assert.True(result.AsBoolean());
    }

    // -----------------------------------------------------------------------
    // Test 63 — Natural(5) > Natural(3) → Boolean(true)
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenGreater_ReturnsBooleanTrue()
    {
        var expr = Bin("5", BinaryOp.Greater, "3");

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Boolean, result.Kind);
        Assert.True(result.AsBoolean());
    }

    // -----------------------------------------------------------------------
    // Test 64 — Natural(5) < Natural(3) → Boolean(false)
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenLess_ReturnsBooleanFalse()
    {
        var expr = Bin("5", BinaryOp.Less, "3");

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Boolean, result.Kind);
        Assert.False(result.AsBoolean());
    }

    // -----------------------------------------------------------------------
    // Test 65 — Natural(5) == Real("5.0") → widens Natural to Real, returns true
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenCrossTypeComparison_WidensFirst()
    {
        // "5" → Natural(5); "5.0" → Real(5.0)
        // WidenPair promotes Natural(5) → Real(5); Real(5).CompareTo(Real(5.0)) == 0.
        var expr = Bin("5", BinaryOp.Equal, "5.0");

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Boolean, result.Kind);
        Assert.True(result.AsBoolean());
    }

    // -----------------------------------------------------------------------
    // Additional coverage — GreaterEqual and LessEqual operators
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenGreaterEqual_ReturnsTrueWhenEqual()
    {
        var expr = Bin("5", BinaryOp.GreaterEqual, "5");

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Boolean, result.Kind);
        Assert.True(result.AsBoolean());
    }

    [Fact]
    public void Evaluate_GivenLessEqual_ReturnsTrueWhenLess()
    {
        var expr = Bin("3", BinaryOp.LessEqual, "5");

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Boolean, result.Kind);
        Assert.True(result.AsBoolean());
    }

    [Fact]
    public void Evaluate_GivenDoubleEquals_ReturnsFalseWhenDifferent()
    {
        var expr = Bin("3", BinaryOp.Equal, "7");

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Boolean, result.Kind);
        Assert.False(result.AsBoolean());
    }
}
