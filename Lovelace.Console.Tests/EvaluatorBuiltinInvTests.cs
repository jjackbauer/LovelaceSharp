using Lovelace.Console.Repl;
using Nat = global::Lovelace.Natural.Natural;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Console.Tests;

/// <summary>
/// Tests for <see cref="Evaluator.Evaluate"/> — built-in function <c>inv(x)</c>.
/// (Test plan items 77–78.)
/// </summary>
public class EvaluatorBuiltinInvTests
{
    private readonly Evaluator _evaluator = new();

    // -----------------------------------------------------------------------
    // Helper: build CallExpr("inv", singleArg)
    // -----------------------------------------------------------------------

    private static CallExpr InvCall(Expr argument) =>
        new("inv", [argument]);

    // -----------------------------------------------------------------------
    // Test 77 — inv(Natural(4)) → Real("0.25")
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenInvOfNatural_WidensToRealAndInverts()
    {
        var expr = InvCall(new LiteralExpr("4"));

        var result = _evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Real, result.Kind);
        Assert.Equal(Rl.Parse("0.25", null), result.AsReal());
    }

    // -----------------------------------------------------------------------
    // Test 78 — inv(Natural(0)) → DivideByZeroException
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenInvOfZero_ThrowsDivideByZeroException()
    {
        var expr = InvCall(new LiteralExpr("0"));

        Assert.Throws<DivideByZeroException>(() => _evaluator.Evaluate(expr));
    }
}
