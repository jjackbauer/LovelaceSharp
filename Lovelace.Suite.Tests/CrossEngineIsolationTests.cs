using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Suite.Tests;

/// <summary>
/// Cross-engine isolation (Cycle 2, §22). Each engine owns its plugin instances, so assumptions,
/// variables and the output writer must not leak between engines evaluating concurrently. The
/// per-engine gate serialises evaluations *within* an engine; these tests exercise the across case.
/// </summary>
public class CrossEngineIsolationTests
{
    private static SuiteEngine Engine()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new SymbolicsPlugin());
        return engine;
    }

    [Fact]
    public async Task ConcurrentEngines_KeepTheirOwnAssumptions()
    {
        var a = Engine();
        var b = Engine();
        await a.EvaluateAsync("x = symbol(\"x\"); assume(x > 0)");
        await b.EvaluateAsync("x = symbol(\"x\"); assume(x < 0)");

        // evaluated concurrently: each engine must answer from its OWN assumption set
        var (aPositive, bNegative) = (a.EvaluateAsync("simplify(x > 0)"), b.EvaluateAsync("simplify(x < 0)"));
        var results = await Task.WhenAll(aPositive, bNegative);

        Assert.True(results[0].AsBoolean(), "engine A lost its own assumption (x > 0)");
        Assert.True(results[1].AsBoolean(), "engine B lost its own assumption (x < 0)");

        // and the opposite question is False in each: no shared set, no cross-talk
        var aNegative = await a.EvaluateAsync("simplify(x < 0)");
        var bPositive = await b.EvaluateAsync("simplify(x > 0)");
        Assert.False(aNegative.AsBoolean());
        Assert.False(bPositive.AsBoolean());
    }

    [Fact]
    public async Task ConcurrentEngines_CaptureTheirOwnPrintOutput()
    {
        var a = Engine();
        var b = Engine();
        var writerA = new StringWriter();
        var writerB = new StringWriter();

        var runA = a.EvaluateAsync("print(\"from-a\")", writerA);
        var runB = b.EvaluateAsync("print(\"from-b\")", writerB);
        await Task.WhenAll(runA, runB);

        Assert.Contains("from-a", writerA.ToString());
        Assert.DoesNotContain("from-b", writerA.ToString());
        Assert.Contains("from-b", writerB.ToString());
        Assert.DoesNotContain("from-a", writerB.ToString());
    }

    [Fact]
    public async Task ConcurrentEngines_KeepTheirOwnVariables()
    {
        var a = Engine();
        var b = Engine();
        var runA = a.EvaluateAsync("shared = 1; print(\"a\")");
        var runB = b.EvaluateAsync("shared = 2; print(\"b\")");
        await Task.WhenAll(runA, runB);

        Assert.Equal("1", a.FormatValue(a.Variables["shared"]));
        Assert.Equal("2", b.FormatValue(b.Variables["shared"]));
    }
}
