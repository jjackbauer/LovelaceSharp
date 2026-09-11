using Lovelace.Suite;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Suite.Tests;

// Real precision is process-global; isolate this collection so parallel tests cannot clobber it.
[Collection("LRealPrecision")]
/// <summary>
/// PROPERTY (stated in the name of every test): the limited-precision fast path in
/// <see cref="NumericOps"/> never changes a result — every expression evaluated by the engine at a
/// limited precision denotes the same value as the same expression evaluated with the
/// arbitrary-precision <see cref="Rl"/> type, including the PERIODIC operands the fixed-width
/// engines (LReal64/LReal128) can only approximate by expanding them to a fixed working width.
///
/// Why this must hold: the fast path is a speed decision, not a semantic one. A periodic Real is an
/// exact rational, and the fixed-width engines resolve it by expanding it to 18/38 fractional
/// digits, which truncates it: <c>(1/17)*17</c> came out of the fast path as
/// <c>0.999999999999999985</c> at precision 18 while the arbitrary-precision path returns exactly 1.
/// The fix has the fast path decline an operand it cannot handle exactly, so the periodic cases are
/// answered by the exact <see cref="Rl"/> paths (Add/Subtract/Multiply) instead.
///
/// These tests pin the tiers: 18 is the LReal64 tier, 37 the LReal128 tier, and 1000 the default
/// where no fast path runs at all.
/// </summary>
public class LRealPeriodicFastPathTests
{
    private static async Task<string> EngineResult(string source, long prec)
    {
        // Set the engine's own precision (not the process-global Real precision). The interpreter
        // wraps each evaluation in Rl.WithPrecision(ComputationDecimalPlaces, DisplayDecimalPlaces),
        // so the LReal64/LReal128 fast path in NumericOps sees the engine's precision.
        var engine = new SuiteEngine();
        engine.SetPrecision(prec);
        var result = await engine.EvaluateAsync(source);
        return engine.FormatValue(result);
    }

    private static Rl Real(string literal) => Rl.Parse(literal, null);

    /// <summary>
    /// The same expression on the class <see cref="Rl"/> type at the same precision — the definition
    /// the fast path must agree with. Every expression here involves at least one periodic operand
    /// (or a division that produces one), because those are exactly the operands the fixed-width
    /// engines can only approximate.
    /// </summary>
    private static string ClassReal(string expr, long prec)
    {
        using var scope = Rl.WithPrecision(prec, prec);
        Rl one = Rl.One;
        Rl value = expr switch
        {
            "(1/17)*17"          => (one / Real("17")) * Real("17"),
            "(1/6)*6"            => (one / Real("6")) * Real("6"),
            "(1/13)*(1/7)"       => (one / Real("13")) * (one / Real("7")),
            "(2/7)*(7/2)"        => (Real("2") / Real("7")) * (Real("7") / Real("2")),
            "(4/3)*(3/4)"        => (Real("4") / Real("3")) * (Real("3") / Real("4")),
            "(1/3)+(1/6)"        => (one / Real("3")) + (one / Real("6")),
            "(1/3)-(1/6)"        => (one / Real("3")) - (one / Real("6")),
            "(1/3)-(1/3)"        => (one / Real("3")) - (one / Real("3")),
            "(1/7)+(1/11)"       => (one / Real("7")) + (one / Real("11")),
            "(1/17)+(1/19)"      => (one / Real("17")) + (one / Real("19")),
            "0.1+(1/3)"          => Real("0.1") + (one / Real("3")),
            "(1/3)/7"            => (one / Real("3")) / Real("7"),
            _ => throw new System.ArgumentException($"no class-Real counterpart for '{expr}'")
        };
        return value.ToString();
    }

    public static IEnumerable<object[]> LimitedPrecisionCorpus()
    {
        string[] expressions =
        {
            "(1/17)*17", "(1/6)*6", "(1/13)*(1/7)", "(2/7)*(7/2)", "(4/3)*(3/4)",
            "(1/3)+(1/6)", "(1/3)-(1/6)", "(1/3)-(1/3)", "(1/7)+(1/11)", "(1/17)+(1/19)",
            "0.1+(1/3)", "(1/3)/7",
        };
        foreach (string expr in expressions)
        {
            yield return new object[] { expr, 18L }; // LReal64 tier
            yield return new object[] { expr, 37L }; // LReal128 tier
        }
    }

    [Theory]
    [MemberData(nameof(LimitedPrecisionCorpus))]
    public async Task PeriodicExpression_AtLimitedPrecision_MatchesClassReal(string expr, long prec)
    {
        Assert.Equal(ClassReal(expr, prec), await EngineResult(expr, prec));
    }

    /// <summary>
    /// The audit shape: <c>(1/17)*17</c> is exactly 1 at every precision — at 18 and 37 the fast
    /// path used to return <c>0.999999999999999985</c>, and both the display and an explicit
    /// comparison with 1 must now report the identity.
    /// </summary>
    [Theory]
    [InlineData(18L)]
    [InlineData(37L)]
    [InlineData(1000L)]
    public async Task Multiply_OneOverSeventeenTimesSeventeen_IsExactlyOne(long prec)
    {
        Assert.Equal("1", await EngineResult("(1/17)*17", prec));
        Assert.Equal("True", await EngineResult("(1/17)*17 == 1", prec));
        Assert.Equal("True", await EngineResult("(1/17)*17 - 1 == 0", prec));
    }

    /// <summary>
    /// The same identity for a periodic quotient whose period fills the LReal64 working width
    /// (1/17 has period 16, the narrowest tier's width is 18), and for one that does not.
    /// </summary>
    [Theory]
    [InlineData(18L)]
    [InlineData(37L)]
    public async Task Multiply_PeriodicQuotientTimesItsDivisor_IsExactlyTheDividend(long prec)
    {
        Assert.Equal("1", await EngineResult("(1/13)*(1/7)*91", prec));
        Assert.Equal("1", await EngineResult("(1/6)*6", prec));
        Assert.Equal("True", await EngineResult("(1/6)*6 == 1", prec));
    }
}
