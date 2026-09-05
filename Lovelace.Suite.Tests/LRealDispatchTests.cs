using Lovelace.Suite;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Suite.Tests;

// Real precision is process-global; isolate this collection so parallel tests cannot clobber it.
[CollectionDefinition("LRealPrecision", DisableParallelization = true)]
public sealed class LRealPrecisionCollection { }

[Collection("LRealPrecision")]
/// <summary>
/// Verifies the NumericOps fast path: when Real.MaxComputationDecimalPlaces is low, the engine
/// routes Real arithmetic through LReal64 (≤18) / LReal128 (≤37), producing results identical to
/// the arbitrary-precision class Real.
/// </summary>
public class LRealDispatchTests
{
    private static async Task<string> EngineResult(string source, long prec)
    {
        // Set the engine's own precision (not the process-global Real precision). The interpreter
        // wraps each evaluation in Rl.WithPrecision(ComputationDecimalPlaces, DisplayDecimalPlaces),
        // so the LReal64/LReal128 fast path in NumericOps sees the engine's precision — the global
        // setting would be overridden by that AsyncLocal scope and the fallback would never run.
        var engine = new SuiteEngine();
        engine.SetPrecision(prec);
        var result = await engine.EvaluateAsync(source);
        return engine.FormatValue(result);
    }

    private static string ClassReal(string expr, long prec)
    {
        // Compute with the class Real type and render at the SAME display precision as the engine,
        // inside an AsyncLocal scope so parallel tests cannot clobber the global display precision.
        using var scope = Rl.WithPrecision(prec, prec);
        return expr switch
        {
            "0.1 + 0.2" => (Rl.Parse("0.1") + Rl.Parse("0.2")).ToString(),
            "1 / 3" => (Rl.One / new Rl("3")).ToString(),
            "3.14 * 2.71" => (Rl.Parse("3.14") * Rl.Parse("2.71")).ToString(),
            "10 / 7" => (Rl.Parse("10") / Rl.Parse("7")).ToString(),
            "2.345678901234567 * 1.234567890123456" => (Rl.Parse("2.345678901234567") * Rl.Parse("1.234567890123456")).ToString(),
            _ => throw new System.ArgumentException()
        };
    }

    [Theory]
    [InlineData("0.1 + 0.2", 18L)]
    [InlineData("1 / 3", 18L)]
    [InlineData("3.14 * 2.71", 18L)]
    [InlineData("10 / 7", 18L)]
    [InlineData("2.345678901234567 * 1.234567890123456", 37L)] // 32 digits — LReal128 tier
    public async Task Arithmetic_AtLowPrecision_MatchesClassReal(string expr, long prec)
    {
        Assert.Equal(ClassReal(expr, prec), await EngineResult(expr, prec));
    }

    [Fact]
    public async Task DefaultPrecision_IsUnchanged()
    {
        // At the default precision (1000) the fast path is NOT used; 0.1+0.2 is still exact.
        Assert.Equal("0.3", await EngineResult("0.1 + 0.2", 1000));
    }
}
