using Lovelace.Suite;
using Nat = global::Lovelace.Natural.Natural;
using Int = global::Lovelace.Integer.Integer;

namespace Lovelace.Suite.Tests;

public class FunctionTests
{
    [Fact]
    public async Task Evaluate_GivenExpressionBodiedFunction_ReturnsResult()
    {
        var engine = new SuiteEngine();

        await engine.EvaluateAsync("func square(x) = x ^ 2");
        var result = await engine.EvaluateAsync("square(5)");

        Assert.Equal(ValueKind.Natural, result.Kind);
        Assert.Equal(Nat.Parse("25", null), result.AsNatural());
    }

    [Fact]
    public async Task Evaluate_GivenBlockBodiedFunction_ReturnsLastExpression()
    {
        var engine = new SuiteEngine();

        await engine.EvaluateAsync("func f(a, b) { c = a + b; c * 2 }");
        var result = await engine.EvaluateAsync("f(3, 4)");

        Assert.Equal(Nat.Parse("14", null), result.AsNatural());
    }

    [Fact]
    public async Task Evaluate_GivenExplicitReturn_ReturnsEarly()
    {
        var engine = new SuiteEngine();

        await engine.EvaluateAsync("func f(x) { if (x > 0) { return 1 }; return -1 }");

        Assert.Equal(Nat.Parse("1", null), (await engine.EvaluateAsync("f(5)")).AsNatural());
        Assert.Equal(Int.Parse("-1", null), (await engine.EvaluateAsync("f(-5)")).AsInteger());
    }

    [Fact]
    public async Task Evaluate_GivenRecursiveFunction_ComputesFactorial()
    {
        var engine = new SuiteEngine();

        await engine.EvaluateAsync("func fact(n) { if (n == 0) { return 1 }; n * fact(n - 1) }");
        var result = await engine.EvaluateAsync("fact(6)");

        Assert.Equal(Nat.Parse("720", null), result.AsNatural());
    }

    [Fact]
    public async Task Evaluate_GivenFunctionLocalVariable_DoesNotLeakToGlobal()
    {
        var engine = new SuiteEngine();

        await engine.EvaluateAsync("func f() { local = 42 }");
        await engine.EvaluateAsync("f()");

        Assert.False(engine.Variables.ContainsKey("local"));
    }

    [Fact]
    public async Task Evaluate_GivenParameterShadowsGlobal_GlobalUnchanged()
    {
        var engine = new SuiteEngine();
        await engine.EvaluateAsync("x = 10");
        await engine.EvaluateAsync("func f(x) { x = x + 1 }");
        await engine.EvaluateAsync("f(1)");

        Assert.Equal(Nat.Parse("10", null), engine.Variables["x"].AsNatural());
    }

    [Fact]
    public async Task Evaluate_GivenWrongArity_Throws()
    {
        var engine = new SuiteEngine();
        await engine.EvaluateAsync("func f(a) { a }");

        await Assert.ThrowsAsync<InvalidOperationException>(() => engine.EvaluateAsync("f(1, 2)"));
    }
}
