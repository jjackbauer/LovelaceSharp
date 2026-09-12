using Lovelace.Abstractions;
using Xunit;

namespace Lovelace.Suite.Tests;

/// <summary>
/// The RECURSION budget: the interpreter's own call path. Cycle 5 bounded the nesting of the INPUT
/// (the parser descent, the parsed tree, a run-time value); a user function that calls itself is
/// deep in none of those — every statement of the source is flat, the parsed tree of the body is a
/// handful of levels, and the value it returns is a scalar. The depth that grows is the
/// INTERPRETER's own native recursion, and it is unbounded: measured on the pre-fix tree,
/// <c>func f(n) { if (n == 0) { return 0 }; return f(n - 1) }</c> + <c>f(432)</c> answers and
/// <c>f(433)</c> terminates the process with 0xC00000FD, 0 bytes on stdout and ~2 MB of
/// "Stack overflow." on stderr.
/// <para>
/// The budget is ONE number over the interpreter's whole evaluation walk rather than a
/// call-level counter, because the recursion LEVEL is not the whole cost: a deep expression inside
/// the recursive body is walked at EVERY level, so 120 nested <c>if</c>s in the body kill the
/// process at recursion depth 5 (measured) while the same 120 <c>if</c>s at the top level answer.
/// </para>
/// <para>
/// The budget and the depth are asserted as LITERALS (never through
/// <c>InputDepth.MaxEvaluationDepth</c>) so this file compiles against the pre-fix tree as well.
/// The refusal type is the cycle-5 <see cref="InputDepthExceededException"/>, which exists on both
/// trees; on the pre-fix tree nothing throws it and every refusal test below fails on the value the
/// interpreter returned instead.
/// </para>
/// </summary>
public class RecursionDepthGuardTests
{
    /// <summary>The interpreter's native evaluation-depth budget in expression-tree units
    /// (Lovelace.Abstractions.InputDepth.MaxEvaluationDepth).</summary>
    private const int MaxEvaluationDepth = 512;

    /// <summary>
    /// The audit's shape: <c>f(n)</c> returns <c>f(n - 1)</c> until zero. One call level costs four
    /// units of the budget plus the two expression levels the body walks, so the deepest depth this
    /// exact function may be called at is <see cref="DeepestAcceptedDepth"/> = 85.
    /// </summary>
    private static string Recursion(int depth) =>
        "func f(n) { if (n == 0) { return 0 }; return f(n - 1) }; f(" + depth + ")";

    /// <summary>The deepest call of <see cref="Recursion"/> the budget admits (measured: 85 answers,
    /// 86 is refused).</summary>
    private const int DeepestAcceptedDepth = 85;

    /// <summary>A chain of <paramref name="terms"/> <c>+ 1</c> terms appended to the recursive call,
    /// so every call level walks a <paramref name="terms"/>-deep expression on the way down.</summary>
    private static string RecursionWithDeepBody(int depth, int terms) =>
        "func f(n) { if (n == 0) { return 0 }; return f(n - 1)" + string.Concat(Enumerable.Repeat(" + 1", terms)) + " }; f("
        + depth + ")";

    private static Task<Value> Evaluate(string script) => new SuiteEngine().EvaluateAsync(script);

    /// <summary>Asserts the typed budget stop: the cycle-5 refusal type, carrying the limit it broke
    /// and the depth it measured, with a message that names both.</summary>
    private static void AssertRefusal(Exception? ex, string shape)
    {
        Assert.NotNull(ex);
        Assert.IsType<InputDepthExceededException>(ex);
        var refusal = (InputDepthExceededException)ex!;
        Assert.True(refusal.Limit == MaxEvaluationDepth,
            $"{shape}: the refusal names the evaluation budget of {MaxEvaluationDepth}, but reported {refusal.Limit}.");
        Assert.True(refusal.Depth > refusal.Limit,
            $"{shape}: the observed depth must be past the budget, but was {refusal.Depth} against {refusal.Limit}.");
        Assert.Contains("evaluation depth ", refusal.Message);
        Assert.Contains($"exceeds the maximum supported evaluation depth of {MaxEvaluationDepth}", refusal.Message);
        Assert.Contains($"evaluation depth {refusal.Depth}", refusal.Message);
    }

    // ------------------------------------------------------------------
    // The finding: plain recursion
    // ------------------------------------------------------------------

    /// <summary>
    /// The audit's exact shape at 200 levels. Pre-fix this RETURNS 0 (200 is under the measured
    /// death of 433, so it is a clean failure rather than a dead test host); it must instead come
    /// back as a typed refusal and must not return a value.
    /// </summary>
    [Fact]
    public async Task RecursionFarPastTheBudget_IsRefused_AndDoesNotReturnAValue()
    {
        Exception? ex = await Record.ExceptionAsync(() => Evaluate(Recursion(200)));

        AssertRefusal(ex, "f(200)");
        Assert.Contains("evaluation depth ", ex!.Message);
    }

    /// <summary>
    /// The boundary of the audit's shape: the deepest call the budget admits answers with the right
    /// value, and one level deeper is refused. Both are named so the published boundary is pinned.
    /// </summary>
    [Fact]
    public async Task RecursionAtTheDeepestAcceptedDepth_Answers()
    {
        Value value = await Evaluate(Recursion(DeepestAcceptedDepth));

        Assert.Equal("0", value.AsNatural().ToString());
    }

    [Fact]
    public async Task RecursionOneLevelPastTheDeepestAcceptedDepth_IsRefused()
    {
        Exception? ex = await Record.ExceptionAsync(() => Evaluate(Recursion(DeepestAcceptedDepth + 1)));

        AssertRefusal(ex, $"f({DeepestAcceptedDepth + 1})");
    }

    // ------------------------------------------------------------------
    // The shape a call-level counter cannot see
    // ------------------------------------------------------------------

    /// <summary>
    /// 10 call levels only — a call-level cap would admit them — but every level walks a 100-term
    /// chain on its way down, so the interpreter's real depth is 10 x 103 levels. Pre-fix this
    /// answers (measured); it must be refused, because at 450 terms the same shape kills the process
    /// at recursion depth 10 with 0xC00000FD and 0 bytes on stdout.
    /// </summary>
    [Fact]
    public async Task ADeepExpressionInsideTheRecursiveBody_IsRefused()
    {
        Exception? ex = await Record.ExceptionAsync(() => Evaluate(RecursionWithDeepBody(10, 100)));

        AssertRefusal(ex, "f(10) with a 100-term body chain");
    }

    // NOTE: the third shape — 120 nested ifs walked at every call level — has no in-process test
    // here ON PURPOSE. Pre-fix it does not return a value, it terminates the process (measured:
    // 0xC00000FD at call depth 5), so asserting it in-process would abort the whole test host and
    // hide every other result in this file. It is asserted where a death is observable as data:
    // Lovelace.Run.Tests/RecursionDepthEnvelopeTests.NestedBlocksInsideARecursiveBody_....

    // ------------------------------------------------------------------
    // The guard must not refuse ordinary recursion
    // ------------------------------------------------------------------

    /// <summary>Ordinary recursion keeps working: the shipped factorial shape, a mutual pair, and a
    /// Fibonacci tree whose call DEPTH is small while its call COUNT is large — the budget bounds
    /// depth, never work.</summary>
    [Fact]
    public async Task OrdinaryRecursion_KeepsWorking()
    {
        var engine = new SuiteEngine();
        await engine.EvaluateAsync("func fact(n) { if (n == 0) { return 1 }; n * fact(n - 1) }");
        Assert.Equal("3628800", (await engine.EvaluateAsync("fact(10)")).AsNatural().ToString());

        await engine.EvaluateAsync("func even(n) { if (n == 0) { return 1 }; return odd(n - 1) }");
        await engine.EvaluateAsync("func odd(n) { if (n == 0) { return 0 }; return even(n - 1) }");
        Assert.Equal("1", (await engine.EvaluateAsync("even(40)")).AsNatural().ToString());

        await engine.EvaluateAsync("func fib(n) { if (n < 2) { return n }; return fib(n - 1) + fib(n - 2) }");
        Assert.Equal("6765", (await engine.EvaluateAsync("fib(20)")).AsNatural().ToString());
    }

    /// <summary>A flat chain at the top level is still walked: the 512-level parsed-tree budget is
    /// untouched by the evaluation budget, which is larger than it.</summary>
    [Fact]
    public async Task ADeepFlatChain_IsStillEvaluated()
    {
        Value value = await Evaluate("1" + string.Concat(Enumerable.Repeat(" + 1", 500)));

        Assert.Equal("501", value.AsNatural().ToString());
    }
}
