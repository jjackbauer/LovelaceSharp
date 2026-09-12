using Lovelace.Abstractions;
using Xunit;

namespace Lovelace.Suite.Tests;

/// <summary>
/// M-2 (round-22 audit M, P1): the evaluation budget was spent by UNRELATED preceding statements.
/// <c>{ 1; f(85) }</c> was refused as "evaluation depth 513 exceeds the maximum supported
/// evaluation depth of 512" while <c>{ f(85); 1 }</c> answered, and the documented boundary
/// (<see cref="RecursionDepthGuardTests"/>: f(85) answers, f(86) is refused) fell to 84 for every
/// composition that charged a unit on the way to the call — <c>print(f(N))</c>, <c>f(N) + 0</c>,
/// <c>ident(f(N))</c>, <c>if (1 == 1) { f(N) }</c>, a loop body — and the refusal's own number moved
/// with the sibling count (513, then 514 at four siblings, 515 at five, 513 again at six).
/// <para>
/// The budget's own documentation says what it measures: "the depth that counts is the depth the
/// script's function bodies add below their entry" (Lovelace.Abstractions.InputDepth). A sibling
/// statement has already RETURNED when the call runs, so it is not part of that depth, and neither
/// is the statement/expression scaffold the call hangs from. The only thing that adds nesting is a
/// user-function frame that is still LIVE when the call runs — which is why the wrapper cases below
/// are pinned one level SHALLOWER per live frame, never deeper.
/// </para>
/// <para>
/// In-process on purpose: the refusal is an exception on both trees (never a process death here),
/// so every composition can be swept cheaply. The audit's exact wire programs are asserted in
/// Lovelace.Run.Tests/RecursionBudgetCompositionEnvelopeTests.cs.
/// </para>
/// </summary>
public class RecursionBudgetCompositionTests
{
    /// <summary>The interpreter's evaluation budget (Lovelace.Abstractions.InputDepth).</summary>
    private const int MaxEvaluationDepth = 512;

    /// <summary>The published boundary: f(85) answers and f(86) is refused.</summary>
    private const int DeepestAcceptedDepth = 85;

    /// <summary>The audit's shape, as its own statement.</summary>
    private const string RecursiveFunction = "func f(n) { if (n == 0) { return 0 }; return f(n - 1) }; ";

    /// <summary>The identity function, for the composition that passes the call as an ARGUMENT.</summary>
    private const string IdentityFunction = "func ident(x) { return x }; ";

    /// <summary>One user frame is LIVE above the call: g has not returned when f runs.</summary>
    private const string OneFrameWrapper = "func g() { return f({0}) }; g()";

    /// <summary>Two live frames above the call.</summary>
    private const string TwoFrameWrapper = "func g() { return f({0}) }; func h() { return g() }; h()";

    private static async Task<SuiteEngine> NewEngineAsync()
    {
        var engine = new SuiteEngine();
        await engine.EvaluateAsync(RecursiveFunction + IdentityFunction);
        return engine;
    }

    private static string Render(string body, int depth) => body.Replace("{0}", depth.ToString());

    /// <summary>
    /// The audit M table's compositions. The third column is the value the composition answers; it
    /// is empty when the shape's own result is not the recursive value (print answers Void).
    /// </summary>
    public static IEnumerable<object[]> Compositions()
    {
        yield return Case("top level", "f({0})");
        yield return Case("a block holding only the call", "{ f({0}) }");
        // the block's own value is its LAST statement, so this shape answers 1, not the call's 0
        yield return Case("a block with the call first", "{ f({0}); 1 }", "1");
        yield return Case("a block with one preceding sibling", "{ 1; f({0}) }");
        yield return Case("a block with two preceding siblings", "{ 1; 1; f({0}) }");
        yield return Case("a block with four preceding siblings", "{ 1; 1; 1; 1; f({0}) }");
        yield return Case("a block with five preceding siblings", "{ 1; 1; 1; 1; 1; f({0}) }");
        yield return Case("a block with twenty preceding siblings",
            "{ " + string.Concat(Enumerable.Repeat("1; ", 20)) + "f({0}) }");
        yield return Case("four top-level statements before the call", "1; 1; 1; 1; f({0})");
        yield return Case("three nested blocks", "{ { { f({0}) } } }");
        yield return Case("the argument of print", "print(f({0}))", "");
        yield return Case("the left operand of +", "f({0}) + 0");
        yield return Case("the argument of a user function", "ident(f({0}))");
        yield return Case("the body of an if", "if (1 == 1) { f({0}) }");
        yield return Case("the body of a for", "for i in 1..5 { f({0}) }");
        yield return Case("the body of a while", "i = 0; while (i < 5) { i = i + 1; f({0}) }");
    }

    private static object[] Case(string name, string body, string answer = "0") =>
        new object[] { name, body, answer };

    /// <summary>The same compositions WITHOUT the expected value: a MemberData row has to match the
    /// theory's parameters, and the refusal theory needs only the shape.</summary>
    public static IEnumerable<object[]> CompositionsOnly()
    {
        foreach (object[] row in Compositions())
            yield return new object[] { row[0], row[1] };
    }

    /// <summary>
    /// The deepest accepted call answers in EVERY composition: the constant statement preceding it
    /// has already returned, so it cannot spend the call's budget. Pre-fix this fails for all eight
    /// compositions that charge anything on the way to the call (the call is refused at 85).
    /// </summary>
    [Theory]
    [MemberData(nameof(Compositions))]
    public async Task TheDeepestAcceptedCall_Answers_InEveryComposition(string name, string body, string answer)
    {
        var engine = await NewEngineAsync();

        Value value = await engine.EvaluateAsync(Render(body, DeepestAcceptedDepth));

        if (answer.Length == 0)
            return;   // the shape answers, but its own value is not the call (print answers Void)

        string display = value.AsNatural().ToString();
        Assert.True(display == answer,
            $"{name}: the deepest accepted call must answer {answer}, but this composition answered {display}.");
    }

    /// <summary>
    /// One level past the published boundary is refused the same way in every composition: the same
    /// typed, named, recoverable budget stop the top-level shape produces.
    /// </summary>
    [Theory]
    [MemberData(nameof(CompositionsOnly))]
    public async Task OneLevelPastTheBoundary_IsRefused_InEveryComposition(string name, string body)
    {
        var engine = await NewEngineAsync();

        Exception? ex = await Record.ExceptionAsync(
            () => engine.EvaluateAsync(Render(body, DeepestAcceptedDepth + 1)));

        AssertRefusal(ex, name);
    }

    /// <summary>
    /// Where the boundary IS, as a number, per composition. Every composition that keeps no user
    /// frame live above the call admits exactly the documented 85; a composition that does keep one
    /// live frame admits one level less, and a second live frame one more. Monotone: extra live
    /// nesting can only LOWER the admitted depth, and nothing but live nesting moves it at all.
    /// </summary>
    [Fact]
    public async Task TheBoundary_MovesOnlyWithLiveNesting()
    {
        var engine = await NewEngineAsync();

        Assert.Equal(85, await DeepestAcceptedAsync(engine, "f({0})"));
        Assert.Equal(85, await DeepestAcceptedAsync(engine, "{ 1; f({0}) }"));
        Assert.Equal(85, await DeepestAcceptedAsync(engine,
            "{ " + string.Concat(Enumerable.Repeat("1; ", 20)) + "f({0}) }"));
        Assert.Equal(85, await DeepestAcceptedAsync(engine, "{ f({0}); 1 }"));
        Assert.Equal(85, await DeepestAcceptedAsync(engine, "1; 1; 1; 1; f({0})"));
        Assert.Equal(85, await DeepestAcceptedAsync(engine, "{ { { f({0}) } } }"));
        Assert.Equal(85, await DeepestAcceptedAsync(engine, "print(f({0}))"));
        Assert.Equal(85, await DeepestAcceptedAsync(engine, "f({0}) + 0"));
        Assert.Equal(85, await DeepestAcceptedAsync(engine, "ident(f({0}))"));
        Assert.Equal(85, await DeepestAcceptedAsync(engine, "if (1 == 1) { f({0}) }"));
        Assert.Equal(85, await DeepestAcceptedAsync(engine, "for i in 1..5 { f({0}) }"));
        Assert.Equal(85, await DeepestAcceptedAsync(engine, "i = 0; while (i < 5) { i = i + 1; f({0}) }"));

        // one LIVE frame above the call costs that frame's units: exactly one level less ...
        Assert.Equal(84, await DeepestAcceptedAsync(engine, OneFrameWrapper));
        // ... and a second live frame one more. Never MORE than the nesting-free boundary.
        Assert.Equal(83, await DeepestAcceptedAsync(engine, TwoFrameWrapper));
    }

    /// <summary>The largest call level this composition admits, found by scanning down from just
    /// past the boundary. The scan reuses one engine: every refusal is balanced by its own finallys.
    /// </summary>
    private static async Task<int> DeepestAcceptedAsync(SuiteEngine engine, string body)
    {
        for (int depth = DeepestAcceptedDepth + 5; depth >= 0; depth--)
        {
            Exception? ex = await Record.ExceptionAsync(() => engine.EvaluateAsync(Render(body, depth)));
            if (ex is null)
                return depth;
        }

        return -1;
    }

    /// <summary>Asserts the typed evaluation-budget stop.</summary>
    private static void AssertRefusal(Exception? ex, string shape)
    {
        Assert.NotNull(ex);
        Assert.IsType<InputDepthExceededException>(ex);
        var refusal = (InputDepthExceededException)ex!;
        Assert.True(refusal.Limit == MaxEvaluationDepth,
            $"{shape}: the refusal must name the evaluation budget of {MaxEvaluationDepth}, but reported {refusal.Limit}.");
        Assert.True(refusal.Depth > refusal.Limit,
            $"{shape}: the observed depth must be past the budget, but was {refusal.Depth} against {refusal.Limit}.");
        Assert.Contains($"exceeds the maximum supported evaluation depth of {MaxEvaluationDepth}", refusal.Message);
        Assert.Contains($"evaluation depth {refusal.Depth}", refusal.Message);
    }
}
