using Lovelace.Suite;
using Nat = global::Lovelace.Natural.Natural;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Suite.Tests;

/// <summary>
/// Tests for <see cref="Interpreter.Evaluate"/> — built-in function <c>inv(x)</c>.
/// (Test plan items 77–78.)
/// </summary>
public class InterpreterBuiltinInvTests
{
    private readonly Interpreter _evaluator = new();

    // -----------------------------------------------------------------------
    // Helper: build CallExpr("inv", singleArg)
    // -----------------------------------------------------------------------

    private static CallExpr InvCall(Expr argument) =>
        new("inv", [argument]);

    // -----------------------------------------------------------------------
    // Test 77 — inv(Natural(4)) → Real("0.25")
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenInvOfNatural_WidensToRealAndInverts()
    {
        var expr = InvCall(new LiteralExpr("4"));

        var result = await _evaluator.EvaluateAsync(expr);

        Assert.Equal(ValueKind.Real, result.Kind);
        Assert.Equal(Rl.Parse("0.25", null), result.AsReal());
    }

    // -----------------------------------------------------------------------
    // Test 78 — inv(Natural(0)) → DivideByZeroException
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenInvOfZero_ThrowsDivideByZeroException()
    {
        var expr = InvCall(new LiteralExpr("0"));

        await Assert.ThrowsAsync<DivideByZeroException>(async () => await _evaluator.EvaluateAsync(expr));
    }
}
