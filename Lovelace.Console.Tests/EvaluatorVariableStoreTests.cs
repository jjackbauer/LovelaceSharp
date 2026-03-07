using Lovelace.Console.Repl;
using Nat = global::Lovelace.Natural.Natural;

namespace Lovelace.Console.Tests;

/// <summary>
/// Tests for <see cref="Evaluator"/> — variable store: get/set and undefined-variable error.
/// (Test plan items 48–50.)
/// </summary>
public class EvaluatorVariableStoreTests
{
    // Each test gets its own Evaluator so variable state cannot bleed between tests.

    // -----------------------------------------------------------------------
    // Assignment stores value and returns it (Test 48)
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenAssignment_StoresValueAndReturnsIt()
    {
        var evaluator = new Evaluator();
        var expr = new AssignExpr("x", new LiteralExpr("5"));

        var result = evaluator.Evaluate(expr);

        Assert.Equal(ValueKind.Natural, result.Kind);
        Assert.Equal(Nat.Parse("5", null), result.AsNatural());
    }

    // -----------------------------------------------------------------------
    // Variable reference returns the stored value (Test 49)
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenVariableReference_ReturnsStoredValue()
    {
        var evaluator = new Evaluator();
        // First store x = 5.
        evaluator.Evaluate(new AssignExpr("x", new LiteralExpr("5")));

        var result = evaluator.Evaluate(new VariableExpr("x"));

        Assert.Equal(ValueKind.Natural, result.Kind);
        Assert.Equal(Nat.Parse("5", null), result.AsNatural());
    }

    // -----------------------------------------------------------------------
    // Undefined variable throws a descriptive error (Test 50)
    // -----------------------------------------------------------------------

    [Fact]
    public void Evaluate_GivenUndefinedVariable_ThrowsError()
    {
        var evaluator = new Evaluator();

        var ex = Assert.Throws<InvalidOperationException>(
            () => evaluator.Evaluate(new VariableExpr("y")));

        Assert.Contains("y", ex.Message);
    }
}
