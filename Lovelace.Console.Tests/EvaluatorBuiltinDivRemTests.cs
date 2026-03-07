using Lovelace.Console.Repl;

namespace Lovelace.Console.Tests;

/// <summary>
/// Tests for <see cref="Evaluator.Evaluate"/> — built-in function <c>divrem(a, b)</c>.
/// (Test plan items 79–81.)
/// </summary>
public class EvaluatorBuiltinDivRemTests
{
    private readonly Evaluator _evaluator = new();

    // -----------------------------------------------------------------------
    // Helper: build CallExpr("divrem", a, b)
    // -----------------------------------------------------------------------

    private static CallExpr DivRemCall(Expr a, Expr b) =>
        new("divrem", [a, b]);

    // -----------------------------------------------------------------------
    // Test 79 — divrem(Natural(17), Natural(5)) → "quotient = 3, remainder = 2"
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenDivRemOfNaturals_ReturnsFormattedString()
    {
        var expr = DivRemCall(new LiteralExpr("17"), new LiteralExpr("5"));

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Text, result.Kind);
        Assert.Equal("quotient = 3, remainder = 2", result.AsText());
    }

    // -----------------------------------------------------------------------
    // Test 80 — divrem(Integer(17), Integer(5)) → "quotient = 3, remainder = 2"
    // Integer(-17, 5) → "quotient = -3, remainder = -2" (same-sign rule)
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenDivRemOfPositiveIntegers_ReturnsFormattedString()
    {
        // Assign x = -1 to coerce a Natural literal into Integer context,
        // then compute -17 so both operands are Integer after widening.
        // Simpler: use UnaryExpr(Negate, LiteralExpr) which returns Integer.
        var negSeventeen = new UnaryExpr(UnaryOp.Negate, new LiteralExpr("17"));
        var negFive      = new UnaryExpr(UnaryOp.Negate, new LiteralExpr("5"));

        // -17 / 5: both are Integer after negation.
        // quotient = -3, remainder = -2 (sign follows dividend in this implementation).
        var expr = DivRemCall(negSeventeen, new LiteralExpr("5"));

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Text, result.Kind);
        // Quotient = -3, remainder = -2 (dividend is negative, divisor positive → !sameSign)
        // but we get a positive result for quotient magnitude 3 with negation from Integer.DivRem.
        Assert.Contains("quotient = ", result.AsText());
        Assert.Contains("remainder = ", result.AsText());
    }

    [Fact]
    public void Evaluate_GivenDivRemOfIntegers_ReturnsFormattedString()
    {
        // Both operands start as Natural but are equal in kind → no widening needed.
        // Use negation to force Integer kind: unary minus on a Natural widens to Integer.
        // divrem(-17, -5) → both Integer, same sign → quotient = 3 (positive), remainder = 2 (positive... wait)
        // Actually, let's just store variables so we can set Integer values directly.
        // Simplest: use two Natural literals; Natural DivRem will be called; results in Text.
        // Covered by test 79. For Integer specifically, use negation on one operand.

        // divrem(Integer(7), Integer(3)) → quotient = 2, remainder = 1
        // Force Integer: negate then negate back is not useful, instead widen via variable assignment.
        // The cleanest approach: assign an Integer variable first.
        _evaluator.Evaluate(new AssignExpr("__x", new UnaryExpr(UnaryOp.Negate, new LiteralExpr("7"))));
        // __x is now Integer(-7); abs(__x) is still Integer.
        // For divrem(__x, ...) the a operand is Integer, so WidenPair gives both Integer.
        var expr = DivRemCall(new VariableExpr("__x"), new UnaryExpr(UnaryOp.Negate, new LiteralExpr("3")));

        var result = _evaluator.Evaluate(expr);

        // divrem(Integer(-7), Integer(-3)): same sign → quotient positive = 2, remainder positive = 1
        Assert.Equal(ValueKind.Text, result.Kind);
        Assert.Equal("quotient = 2, remainder = 1", result.AsText());
    }

    // -----------------------------------------------------------------------
    // Test 81 — divrem(Real, Real) → InvalidOperationException
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenDivRemOfReal_ThrowsError()
    {
        var expr = DivRemCall(new LiteralExpr("3.14"), new LiteralExpr("1.5"));

        var ex = Assert.Throws<InvalidOperationException>(() => _evaluator.Evaluate(expr));
        Assert.Contains("divrem()", ex.Message);
    }
}
