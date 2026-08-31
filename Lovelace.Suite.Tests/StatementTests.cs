using Lovelace.Suite;
using Nat = global::Lovelace.Natural.Natural;
using Int = global::Lovelace.Integer.Integer;

namespace Lovelace.Suite.Tests;

public class StatementTests
{
    private static async Task<Value> Eval(SuiteEngine engine, string source) =>
        await engine.EvaluateAsync(source);

    [Fact]
    public async Task Evaluate_GivenBlock_ReturnsLastExpressionValue()
    {
        var engine = new SuiteEngine();

        var result = await Eval(engine, "{ 1; 2; 3 }");

        Assert.Equal(ValueKind.Natural, result.Kind);
        Assert.Equal(Nat.Parse("3", null), result.AsNatural());
    }

    [Fact]
    public async Task Evaluate_GivenIfElseTrueBranch_SelectsTrueBranch()
    {
        var engine = new SuiteEngine();

        var result = await Eval(engine, "if (2 > 1) { 10 } else { 20 }");

        Assert.Equal(Nat.Parse("10", null), result.AsNatural());
    }

    [Fact]
    public async Task Evaluate_GivenIfElseFalseBranch_SelectsFalseBranch()
    {
        var engine = new SuiteEngine();

        var result = await Eval(engine, "if (2 < 1) { 10 } else { 20 }");

        Assert.Equal(Nat.Parse("20", null), result.AsNatural());
    }

    [Fact]
    public async Task Evaluate_GivenWhileLoop_AccumulatesCounter()
    {
        var engine = new SuiteEngine();

        var result = await Eval(engine, "i = 0; while (i < 5) { i = i + 1 }; i");

        Assert.Equal(Nat.Parse("5", null), result.AsNatural());
    }

    [Fact]
    public async Task Evaluate_GivenForRange_IteratesInclusiveBounds()
    {
        var engine = new SuiteEngine();

        var result = await Eval(engine, "sum = 0; for i in 1..4 { sum = sum + i }; sum");

        Assert.Equal(Nat.Parse("10", null), result.AsNatural());
    }

    [Fact]
    public async Task Evaluate_GivenForBreak_StopsEarly()
    {
        var engine = new SuiteEngine();

        var result = await Eval(engine, "n = 0; for i in 1..10 { n = i; if (i == 3) { break } }; n");

        Assert.Equal(Nat.Parse("3", null), result.AsNatural());
    }

    [Fact]
    public async Task Evaluate_GivenForContinue_SkipsElement()
    {
        var engine = new SuiteEngine();

        var result = await Eval(engine, "sum = 0; for i in 1..5 { if (i == 3) { continue }; sum = sum + i }; sum");

        // 1 + 2 + 4 + 5 = 12
        Assert.Equal(Nat.Parse("12", null), result.AsNatural());
    }

    [Fact]
    public async Task Evaluate_GivenReturnAtTopLevel_ReturnsValue()
    {
        var engine = new SuiteEngine();

        var result = await Eval(engine, "return 42");

        Assert.Equal(Nat.Parse("42", null), result.AsNatural());
    }

    [Fact]
    public async Task Evaluate_GivenVariableAssignmentInsideBlock_DoesNotLeak()
    {
        var engine = new SuiteEngine();

        await Eval(engine, "{ z = 99 }");

        Assert.False(engine.Variables.ContainsKey("z"));
    }

    [Fact]
    public async Task Evaluate_GivenBreakOutsideLoop_Throws()
    {
        var engine = new SuiteEngine();

        await Assert.ThrowsAsync<InvalidOperationException>(() => Eval(engine, "break"));
    }

    [Fact]
    public async Task Evaluate_GivenRange_ProducesNaturalVector()
    {
        var engine = new SuiteEngine();

        var result = await Eval(engine, "1..5");

        Assert.Equal(ValueKind.Vector, result.Kind);
        Assert.Equal(5, result.AsVector().Count);
        Assert.Equal(ValueKind.Natural, result.AsVector()[0].Kind);
        Assert.Equal(Nat.Parse("1", null), result.AsVector()[0].AsNatural());
        Assert.Equal(Nat.Parse("5", null), result.AsVector()[4].AsNatural());
    }

    [Fact]
    public async Task Evaluate_GivenSteppedRange_ProducesProgression()
    {
        var engine = new SuiteEngine();

        var result = await Eval(engine, "1..2..7");

        Assert.Equal(4, result.AsVector().Count);
        Assert.Equal(ValueKind.Integer, result.AsVector()[3].Kind);
        Assert.Equal(Int.Parse("7", null), result.AsVector()[3].AsInteger());
    }

    [Fact]
    public async Task Evaluate_GivenNegativeRange_ProducesIntegers()
    {
        var engine = new SuiteEngine();

        var result = await Eval(engine, "-2..2");

        Assert.Equal(ValueKind.Integer, result.AsVector()[0].Kind);
        Assert.Equal(Int.Parse("-2", null), result.AsVector()[0].AsInteger());
    }
}
