using Lovelace.Abstractions;
using Lovelace.Suite;

namespace Lovelace.Suite.Tests;

/// <summary>
/// Tests for <see cref="Interpreter.Evaluate"/> — built-in function <c>divrem(a, b)</c>.
/// (Test plan items 79–81.)
///
/// <para>The result is a <c>DivRemResult</c> RECORD carrying <c>quotient</c> and <c>remainder</c> as
/// separate fields (A1-N-08). It used to be the display sentence
/// <c>"quotient = 3, remainder = 2"</c>, which forced every machine consumer to parse prose to
/// recover two integers; these tests pin the structure and the numbers, not the sentence.</para>
/// </summary>
public class InterpreterBuiltinDivRemTests
{
    private readonly Interpreter _evaluator = new();

    // -----------------------------------------------------------------------
    // Helper: build CallExpr("divrem", a, b) and read the result record
    // -----------------------------------------------------------------------

    private static CallExpr DivRemCall(Expr a, Expr b) =>
        new("divrem", [a, b]);

    /// <summary>Reads one field of the result, asserting the record shape on the way: a consumer
    /// that can name the fields it reads is the whole point of the change.</summary>
    private static Value Field(Value result, string name)
    {
        Assert.Equal(ValueKind.Record, result.Kind);
        RecordValue record = result.AsRecord();
        Assert.Equal("DivRemResult", record.TypeName);
        Assert.Equal(new[] { "quotient", "remainder" }, record.Fields.Select(f => f.Name).ToArray());
        Assert.True(record.TryGetField(name, out object? value), $"DivRemResult has no field '{name}'");
        return (Value)value!;
    }

    // -----------------------------------------------------------------------
    // Test 79 — divrem(Natural(17), Natural(5)) → quotient 3, remainder 2
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenDivRemOfNaturals_ReturnsAStructuredRecord()
    {
        var expr = DivRemCall(new LiteralExpr("17"), new LiteralExpr("5"));

        var result = await _evaluator.EvaluateAsync(expr);

        Assert.Equal(ValueKind.Record, result.Kind);
        Value quotient = Field(result, "quotient");
        Value remainder = Field(result, "remainder");
        Assert.Equal(ValueKind.Natural, quotient.Kind);
        Assert.Equal(ValueKind.Natural, remainder.Kind);
        Assert.Equal("3", quotient.AsNatural().ToString());
        Assert.Equal("2", remainder.AsNatural().ToString());
    }

    // -----------------------------------------------------------------------
    // Test 80 — divrem(Integer(-17), Integer(5)) → quotient -3, remainder -2
    // (the truncated quotient and the remainder that goes with it: a = q·b + r)
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenDivRemWithANegativeDividend_KeepsTheTruncatedSigns()
    {
        // -17 / 5: both operands are Integer after negation.
        var negSeventeen = new UnaryExpr(UnaryOp.Negate, new LiteralExpr("17"));
        var expr = DivRemCall(negSeventeen, new LiteralExpr("5"));

        var result = await _evaluator.EvaluateAsync(expr);

        Value quotient = Field(result, "quotient");
        Value remainder = Field(result, "remainder");
        Assert.Equal(ValueKind.Integer, quotient.Kind);
        Assert.Equal(ValueKind.Integer, remainder.Kind);
        // -17 = (-3)·5 + (-2)
        Assert.Equal("-3", quotient.AsInteger().ToString());
        Assert.Equal("-2", remainder.AsInteger().ToString());
    }

    [Fact]
    public async Task Evaluate_GivenDivRemOfIntegers_KeepsTheSameSignQuotientPositive()
    {
        // __x = -7 (Integer); divrem(__x, -3) therefore runs with two Integer operands.
        await _evaluator.EvaluateAsync(new AssignExpr("__x", new UnaryExpr(UnaryOp.Negate, new LiteralExpr("7"))));
        var expr = DivRemCall(new VariableExpr("__x"), new UnaryExpr(UnaryOp.Negate, new LiteralExpr("3")));

        var result = await _evaluator.EvaluateAsync(expr);

        // divrem(Integer(-7), Integer(-3)): same sign → quotient 2, remainder 1
        Assert.Equal("2", Field(result, "quotient").AsInteger().ToString());
        Assert.Equal("1", Field(result, "remainder").AsInteger().ToString());
    }

    // -----------------------------------------------------------------------
    // Test 81 — divrem(Real, Real) → InvalidOperationException
    // -----------------------------------------------------------------------

    [Fact]
    public async Task Evaluate_GivenDivRemOfReal_ThrowsError()
    {
        var expr = DivRemCall(new LiteralExpr("3.14"), new LiteralExpr("1.5"));

        var ex = await Assert.ThrowsAsync<InvalidOperationException>(async () => await _evaluator.EvaluateAsync(expr));
        Assert.Contains("divrem()", ex.Message);
    }
}
