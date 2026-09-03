using Lovelace.Suite;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Suite.Tests;

/// <summary>
/// Functional tests for the built-in function <c>setprecision(n)</c>.
/// Checklist items:
///   - Register "setprecision" in Interpreter.RegisterBuiltins
///   - BuiltinSetprecision — 1 Natural or Integer argument sets both
///     Real.MaxComputationDecimalPlaces and Real.DisplayDecimalPlaces
///   - BuiltinSetprecision — lifting the cap lets pi(n) exceed the 1000 default
///   - BuiltinSetprecision — reject Real argument with InvalidOperationException
///   - BuiltinSetprecision — reject n ≤ 0 with InvalidOperationException
///   - BuiltinSetprecision — reject argument count ≠ 1 with InvalidOperationException
/// </summary>
public class InterpreterBuiltinSetprecisionTests
{
    private readonly Interpreter _evaluator = new();

    // -----------------------------------------------------------------------
    // Natural argument sets both precision settings.
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenNaturalArgument_SetsComputationAndDisplayPrecision()
    {
        var expr = new CallExpr("setprecision", [new LiteralExpr("2500")]);

        var result = await _evaluator.EvaluateAsync(expr);

        Assert.Equal(ValueKind.Void, result.Kind);
        Assert.Equal(2500L, _evaluator.ComputationDecimalPlaces);
        Assert.Equal(2500L, _evaluator.DisplayDecimalPlaces);
    }

    // -----------------------------------------------------------------------
    // Integer argument behaves like the Natural case.
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenIntegerArgument_SetsComputationAndDisplayPrecision()
    {
        // Double-negate "2500": Natural(2500) → Integer(-2500) → Integer(2500).
        var posInt = new UnaryExpr(UnaryOp.Negate, new UnaryExpr(UnaryOp.Negate, new LiteralExpr("2500")));
        var expr = new CallExpr("setprecision", [posInt]);

        var result = await _evaluator.EvaluateAsync(expr);

        Assert.Equal(ValueKind.Void, result.Kind);
        Assert.Equal(2500L, _evaluator.ComputationDecimalPlaces);
        Assert.Equal(2500L, _evaluator.DisplayDecimalPlaces);
    }

    // -----------------------------------------------------------------------
    // Lifting the cap above the 1000 default lets pi(n) compute more digits.
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenPrecisionAboveDefault_AllowsPiToComputeThatManyDigits()
    {
        long savedMax = Rl.MaxComputationDecimalPlaces;
        long savedDisplay = Rl.DisplayDecimalPlaces;
        try
        {
            await _evaluator.EvaluateAsync(new CallExpr("setprecision", [new LiteralExpr("1100")]));

            var result = await _evaluator.EvaluateAsync(new CallExpr("pi", [new LiteralExpr("1100")]));

            Assert.Equal(ValueKind.Real, result.Kind);
            Assert.Equal(-1100L, result.AsReal().Exponent);
        }
        finally
        {
            Rl.MaxComputationDecimalPlaces = savedMax;
            Rl.DisplayDecimalPlaces = savedDisplay;
        }
    }

    // -----------------------------------------------------------------------
    // Reject non-Natural/Integer arguments.
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenRealArgument_ThrowsInvalidOperationException()
    {
        var expr = new CallExpr("setprecision", [new LiteralExpr("3.0")]);

        await Assert.ThrowsAsync<InvalidOperationException>(async () => await _evaluator.EvaluateAsync(expr));
    }

    // -----------------------------------------------------------------------
    // Reject non-positive digit counts.
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenZero_ThrowsInvalidOperationException()
    {
        var expr = new CallExpr("setprecision", [new LiteralExpr("0")]);

        await Assert.ThrowsAsync<InvalidOperationException>(async () => await _evaluator.EvaluateAsync(expr));
    }

    [Fact]
    public async Task Evaluate_GivenNegative_ThrowsInvalidOperationException()
    {
        var neg = new UnaryExpr(UnaryOp.Negate, new LiteralExpr("10"));
        var expr = new CallExpr("setprecision", [neg]);

        await Assert.ThrowsAsync<InvalidOperationException>(async () => await _evaluator.EvaluateAsync(expr));
    }

    // -----------------------------------------------------------------------
    // Reject wrong arity.
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenTooManyArguments_ThrowsInvalidOperationException()
    {
        var expr = new CallExpr("setprecision", [new LiteralExpr("10"), new LiteralExpr("20")]);

        await Assert.ThrowsAsync<InvalidOperationException>(async () => await _evaluator.EvaluateAsync(expr));
    }
}
