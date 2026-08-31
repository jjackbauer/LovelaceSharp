using Lovelace.Suite;
using Nat = global::Lovelace.Natural.Natural;

namespace Lovelace.Suite.Tests;

/// <summary>
/// Tests for <see cref="Interpreter"/> — variable store: get/set and undefined-variable error.
/// (Test plan items 48–50.)
/// </summary>
public class InterpreterVariableStoreTests
{
    // Each test gets its own Interpreter so variable state cannot bleed between tests.

    // -----------------------------------------------------------------------
    // Assignment stores value and returns it (Test 48)
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenAssignment_StoresValueAndReturnsIt()
    {
        var evaluator = new Interpreter();
        var expr = new AssignExpr("x", new LiteralExpr("5"));

        var result = await evaluator.EvaluateAsync(expr);

        Assert.Equal(ValueKind.Natural, result.Kind);
        Assert.Equal(Nat.Parse("5", null), result.AsNatural());
    }

    // -----------------------------------------------------------------------
    // Variable reference returns the stored value (Test 49)
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenVariableReference_ReturnsStoredValue()
    {
        var evaluator = new Interpreter();
        // First store x = 5.
        await evaluator.EvaluateAsync(new AssignExpr("x", new LiteralExpr("5")));

        var result = await evaluator.EvaluateAsync(new VariableExpr("x"));

        Assert.Equal(ValueKind.Natural, result.Kind);
        Assert.Equal(Nat.Parse("5", null), result.AsNatural());
    }

    // -----------------------------------------------------------------------
    // Undefined variable throws a descriptive error (Test 50)
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenUndefinedVariable_ThrowsError()
    {
        var evaluator = new Interpreter();

        var ex = await Assert.ThrowsAsync<InvalidOperationException>(
            async () => await evaluator.EvaluateAsync(new VariableExpr("y")));

        Assert.Contains("y", ex.Message);
    }
}
