using Lovelace.Console.Repl;
using Nat = global::Lovelace.Natural.Natural;
using Int = global::Lovelace.Integer.Integer;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Console.Tests;

/// <summary>
/// Tests for <see cref="Evaluator.Evaluate"/> — binary arithmetic operators
/// (+, -, *, /, %, ^) with automatic widening via <see cref="Value.WidenPair"/>.
/// (Test plan items 51–58.)
/// </summary>
public class EvaluatorBinaryArithmeticTests
{
    private readonly Evaluator _evaluator = new();

    // Helper: build a BinaryExpr from two literal strings and an operator.
    private static BinaryExpr Bin(string left, BinaryOp op, string right) =>
        new(new LiteralExpr(left), op, new LiteralExpr(right));

    // -----------------------------------------------------------------------
    // Test 51 — Natural + Natural → Natural
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenNaturalPlusNatural_ReturnsNatural()
    {
        var expr = Bin("2", BinaryOp.Add, "3");

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Natural, result.Kind);
        Assert.Equal(Nat.Parse("5", null), result.AsNatural());
    }

    // -----------------------------------------------------------------------
    // Test 52 — Natural + Real → Real (cross-type widening through the evaluator)
    //   Note: Integer literals require Unary negation (not yet implemented at this
    //   checklist item), so Natural+Real is the narrowest cross-type scenario
    //   expressible from literal AST nodes here.
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenNaturalPlusReal_WidensAndReturnsReal()
    {
        // "5" → Natural(5) ; "3.0" → Real(3.0)
        // WidenPair promotes Natural(5) → Real(5); Real(5) + Real(3.0) = Real(8.0)
        var expr = Bin("5", BinaryOp.Add, "3.0");

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Real, result.Kind);
        Assert.Equal(Rl.Parse("8.0", null), result.AsReal());
    }

    // -----------------------------------------------------------------------
    // Test 53 — Real - Real → Real (Real subtraction branch dispatch)
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenRealMinusReal_ReturnsCorrectDifference()
    {
        // "5.0" → Real(5.0); "3.5" → Real(3.5); 5.0 − 3.5 = 1.5
        var expr = Bin("5.0", BinaryOp.Subtract, "3.5");

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Real, result.Kind);
        Assert.Equal(Rl.Parse("1.5", null), result.AsReal());
    }

    // -----------------------------------------------------------------------
    // Test 54 — Natural * Natural → Natural (correct product)
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenMultiply_ReturnsCorrectProduct()
    {
        var expr = Bin("12", BinaryOp.Multiply, "34");

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Natural, result.Kind);
        Assert.Equal(Nat.Parse("408", null), result.AsNatural());
    }

    // -----------------------------------------------------------------------
    // Test 55 — Natural / Natural → Natural (exact division)
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenDivide_ReturnsCorrectQuotient()
    {
        var expr = Bin("100", BinaryOp.Divide, "5");

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Natural, result.Kind);
        Assert.Equal(Nat.Parse("20", null), result.AsNatural());
    }

    // -----------------------------------------------------------------------
    // Test 56 — Natural / Natural(0) → DivideByZeroException
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenDivideByZero_ThrowsDivideByZeroException()
    {
        var expr = Bin("5", BinaryOp.Divide, "0");

        Assert.Throws<DivideByZeroException>(() => _evaluator.Evaluate(expr));
    }

    // -----------------------------------------------------------------------
    // Test 57 — Natural % Natural → natural remainder
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenModulo_ReturnsCorrectRemainder()
    {
        var expr = Bin("17", BinaryOp.Modulo, "5");

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Natural, result.Kind);
        Assert.Equal(Nat.Parse("2", null), result.AsNatural());
    }

    // -----------------------------------------------------------------------
    // Test 58 — Natural ^ Natural → correct power
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenPower_ReturnsCorrectResult()
    {
        var expr = Bin("2", BinaryOp.Power, "10");

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Natural, result.Kind);
        Assert.Equal(Nat.Parse("1024", null), result.AsNatural());
    }

    // -----------------------------------------------------------------------
    // Additional: Real + Real dispatch (covers evaluator's Real arithmetic branch)
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenRealPlusReal_ReturnsCorrectSum()
    {
        var expr = Bin("1.5", BinaryOp.Add, "2.5");

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Real, result.Kind);
        Assert.Equal(Rl.Parse("4.0", null), result.AsReal());
    }

    // -----------------------------------------------------------------------
    // Additional: Natural - Natural (non-underflow) → Natural
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenNaturalMinusSmaller_ReturnsNaturalDifference()
    {
        var expr = Bin("10", BinaryOp.Subtract, "3");

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Natural, result.Kind);
        Assert.Equal(Nat.Parse("7", null), result.AsNatural());
    }

    // -----------------------------------------------------------------------
    // Test 59 — Natural - larger Natural → auto-widens to Integer
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenNaturalMinusLarger_AutoWidensToInteger()
    {
        // Natural(3) - Natural(5) underflows; evaluator catches InvalidOperationException,
        // widens both to Integer and retries: Integer(3) - Integer(5) = Integer(-2).
        var expr = Bin("3", BinaryOp.Subtract, "5");

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Integer, result.Kind);
        Assert.Equal(Int.Parse("-2", null), result.AsInteger());
    }

    // -----------------------------------------------------------------------
    // Test 60 — Natural - equal Natural → Natural(0) (no underflow)
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenNaturalMinusEqual_ReturnsNaturalZero()
    {
        // Natural(5) - Natural(5) = 0, no underflow — stays Natural.
        var expr = Bin("5", BinaryOp.Subtract, "5");

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Natural, result.Kind);
        Assert.Equal(Nat.Parse("0", null), result.AsNatural());
    }
}
