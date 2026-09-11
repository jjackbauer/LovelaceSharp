using System.Text;
using System.Text.Json.Nodes;
using Xunit;

namespace Lovelace.Run.Tests;

/// <summary>
/// A value built by REPETITION — <c>x = 1; x = [x]; x = [x]; …</c> — is valid input whose VALUE is
/// deeply nested while the source and the parsed tree stay flat, so neither of the input budgets
/// (Lovelace.Abstractions.InputDepth) can see it. On the pre-fix tree every level of that value is a
/// native recursion in the renderer: the variable capture dies first, the process is terminated by
/// the OS with 0xC00000FD, stdout carries NO envelope at all, and the agent-facing protocol has
/// nothing to report. The guard under test turns that death into one JSON envelope with a non-zero
/// exit code and a code/category that names the condition.
/// <para>
/// The code, the category and the budget are asserted as WIRE STRINGS (never as a reference to the
/// exception type or to <c>InputDepth.MaxValueDepth</c>) so this file compiles against the pre-fix
/// tree as well — that is what makes the control-tree evidence (the test host dying with
/// 0xC00000FD) reproducible with it.
/// </para>
/// </summary>
public class ValueDepthEnvelopeTests
{
    /// <summary>The value-nesting budget (Lovelace.Abstractions.InputDepth.MaxValueDepth).</summary>
    private const int MaxValueDepth = 1024;

    /// <summary>The wire code naming a depth refusal.</summary>
    private const string DepthCode = "DepthExceeded";

    /// <summary>Its category: an exhausted budget, not a domain error and not an invariant failure.</summary>
    private const string DepthCategory = "BudgetExceeded";

    /// <summary>
    /// The script the audit filed: <c>x = 1</c> then <paramref name="wraps"/> assignments of
    /// <c>x = [x]</c>. The scalar leaf is level 1, so the value left in <c>x</c> — and the result the
    /// last assignment produced — nests <c>wraps + 1</c> levels.
    /// </summary>
    private static string WrapScript(int wraps)
    {
        var sb = new StringBuilder("x = 1");
        for (int i = 0; i < wraps; i++)
            sb.Append("; x = [x]");
        return sb.ToString();
    }

    // ------------------------------------------------------------------
    // The audit's exact script
    // ------------------------------------------------------------------

    /// <summary>
    /// 20000 wraps: the finding. Pre-fix this is exit 3221225725 (0xC00000FD) with 0 bytes on
    /// stdout; the refusal must instead be a well-formed envelope naming the condition.
    /// </summary>
    [Fact]
    public async Task TheAuditScript_IsATypedRefusal_WithAWellFormedEnvelope()
    {
        var (exitCode, stdout, stderr) = await TestSupport.RunScriptAsync(
            WrapScript(20000), "--omit-functions", "--omit-variables");
        JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(stdout, "runner stdout");

        Assert.Equal(1, exitCode);
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"ok must be false. Envelope: {stdout}");
        Assert.Equal(DepthCode, envelope["code"]!.GetValue<string>());
        Assert.Equal(DepthCategory, envelope["category"]!.GetValue<string>());
        Assert.True(envelope["recoverable"]!.GetValue<bool>(),
            "the caller can rebuild the value with less nesting, so the refusal is recoverable");
        Assert.Equal(1, envelope["protocolVersion"]!.GetValue<int>());
        Assert.Equal("", stderr);
        // the message names the measured depth and the budget it broke
        Assert.Matches(
            @"value depth \d+ exceeds the maximum supported nesting depth of 1024\.",
            envelope["message"]!.GetValue<string>());
        // the measured depth is the real one: 20000 repetitions raise the array's RANK, and the
        // guard reports the value it actually measured rather than the budget it broke
        Assert.Contains("value depth 20001", envelope["message"]!.GetValue<string>());
    }

    /// <summary>
    /// The same script with a print appended dies on the pre-fix tree too — the overflow is not only
    /// the capture on the way out. The refusal must come from the print's own render.
    /// </summary>
    [Fact]
    public async Task ADeepValueOnThePrintPath_IsAlsoRefused()
    {
        var (exitCode, stdout, _) = await TestSupport.RunScriptAsync(
            WrapScript(20000) + "; print(x)", "--omit-functions", "--omit-variables");
        JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(stdout, "runner stdout");

        Assert.Equal(1, exitCode);
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"ok must be false. Envelope: {stdout}");
        Assert.Equal(DepthCode, envelope["code"]!.GetValue<string>());
        Assert.Equal(DepthCategory, envelope["category"]!.GetValue<string>());
    }

    /// <summary>
    /// A deep value that is NOT held by any variable when the script ends: the loop's block-local
    /// <c>x</c> is gone, so only the RESULT is deep and only the result projection can refuse it.
    /// This is the shape that proves the guard covers the result's own render, not merely the
    /// variable capture.
    /// </summary>
    [Fact]
    public async Task ADeepResultWithNoDeepVariable_IsAlsoRefused()
    {
        var sb = new StringBuilder("for i in 1..1 { x = 1");
        for (int i = 0; i < 20000; i++)
            sb.Append("; x = [x]");
        sb.Append("; x }");

        var (exitCode, stdout, _) = await TestSupport.RunScriptAsync(
            sb.ToString(), "--omit-functions", "--omit-variables");
        JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(stdout, "runner stdout");

        Assert.Equal(1, exitCode);
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"ok must be false. Envelope: {stdout}");
        Assert.Equal(DepthCode, envelope["code"]!.GetValue<string>());
        Assert.Equal(DepthCategory, envelope["category"]!.GetValue<string>());
    }

    // ------------------------------------------------------------------
    // The boundary: limit-1 accepted, limit+1 refused
    // ------------------------------------------------------------------

    /// <summary>Wraps whose value measures EXACTLY <paramref name="depth"/>: the scalar leaf is
    /// level 1, so <c>depth - 1</c> wraps measure <paramref name="depth"/>. (Verified against the
    /// reported depth of the refusal at <c>depth + 1</c>: 1023 wraps are accepted, 1024 report
    /// "value depth 1025".)</summary>
    private static string WrapsAtDepth(int depth) => WrapScript(depth - 1);

    [Fact]
    public async Task ValueAtTheLimit_IsAccepted()
    {
        var (exitCode, stdout, _) = await TestSupport.RunScriptAsync(
            WrapsAtDepth(MaxValueDepth), "--omit-functions", "--omit-variables");
        JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(stdout, "runner stdout");

        Assert.True(envelope["ok"]!.GetValue<bool>(),
            $"a value of depth {MaxValueDepth} is exactly the budget and must be accepted. exit={exitCode} stdout={stdout}");
        Assert.Equal(0, exitCode);
        Assert.Null(envelope["code"]);
    }

    [Fact]
    public async Task ValueOnePastTheLimit_IsRefused()
    {
        var (exitCode, stdout, _) = await TestSupport.RunScriptAsync(
            WrapsAtDepth(MaxValueDepth + 1), "--omit-functions", "--omit-variables");
        JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(stdout, "runner stdout");

        Assert.NotEqual(0, exitCode);
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"ok must be false. Envelope: {stdout}");
        Assert.Equal(DepthCode, envelope["code"]!.GetValue<string>());
        Assert.Equal(DepthCategory, envelope["category"]!.GetValue<string>());
        // the refusal names the depth it measured, so the boundary is observable on the wire
        Assert.Contains($"value depth {MaxValueDepth + 1}", envelope["message"]!.GetValue<string>());
    }

    // ------------------------------------------------------------------
    // The property: increasing wraps answer or refuse, and never die
    // ------------------------------------------------------------------

    public static IEnumerable<object[]> IncreasingWrapCounts()
    {
        foreach (int wraps in new[] { 1, 2, 8, 64, 512, 1023, 1024, 1025, 4096, 20000 })
            yield return new object[] { wraps };
    }

    /// <summary>
    /// For increasing wrap counts the runner must produce EITHER a correct envelope OR a typed
    /// refusal — and the boundary must be the published one: every count whose value fits the
    /// budget is answered, every count past it is refused. A process death is not representable as
    /// an assertion failure (the host dies with it), which is exactly why the pre-fix run of this
    /// theory is the evidence: it kills the test host at 20000 wraps.
    /// </summary>
    [Theory]
    [MemberData(nameof(IncreasingWrapCounts))]
    public async Task IncreasingWrapCounts_AnswerOrRefuse_NeverDie(int wraps)
    {
        var (exitCode, stdout, stderr) = await TestSupport.RunScriptAsync(
            WrapScript(wraps), "--omit-functions", "--omit-variables");
        JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(stdout, "runner stdout");
        int depth = wraps + 1;

        if (depth <= MaxValueDepth)
        {
            Assert.True(envelope["ok"]!.GetValue<bool>(),
                $"{wraps} wraps (value depth {depth}) is within the budget of {MaxValueDepth} and must be answered. "
                + $"exit={exitCode} stderr={stderr} stdout={Describe(stdout)}");
            Assert.Equal(0, exitCode);
        }
        else
        {
            Assert.False(envelope["ok"]!.GetValue<bool>(),
                $"{wraps} wraps (value depth {depth}) is past the budget of {MaxValueDepth} and must be refused. "
                + $"stdout={Describe(stdout)}");
            Assert.Equal(1, exitCode);
            Assert.Equal(DepthCode, envelope["code"]!.GetValue<string>());
            Assert.Equal(DepthCategory, envelope["category"]!.GetValue<string>());
            Assert.True(envelope["recoverable"]!.GetValue<bool>());
        }
    }

    private static string Describe(string text) =>
        text.Length <= 240 ? text : text.Substring(0, 240) + "...";

    // ------------------------------------------------------------------
    // The guard must not refuse legitimate values
    // ------------------------------------------------------------------

    /// <summary>Wide is not deep: a 20000-element flat vector and a handful of nested levels are
    /// ordinary values and must keep crossing unchanged.</summary>
    [Fact]
    public async Task LegitimateValues_AreNotRefused()
    {
        var (wideExit, wideStdout, _) = await TestSupport.RunScriptAsync(
            "v = 1..20000; v[19999]", "--omit-functions", "--omit-variables");
        JsonNode wide = TestSupport.ParseExactlyOneJsonDocument(wideStdout, "runner stdout");
        Assert.Equal(0, wideExit);
        Assert.True(wide["ok"]!.GetValue<bool>(), $"a 20000-element flat vector must still run. Envelope: {Describe(wideStdout)}");

        var (nestedExit, nestedStdout, _) = await TestSupport.RunScriptAsync(
            "x = 1; for i in 1..20 { x = [x] }; x", "--omit-functions", "--omit-variables");
        JsonNode nested = TestSupport.ParseExactlyOneJsonDocument(nestedStdout, "runner stdout");
        Assert.Equal(0, nestedExit);
        Assert.True(nested["ok"]!.GetValue<bool>(), $"a 21-level nested value must still run. Envelope: {Describe(nestedStdout)}");
        Assert.Equal("Array", nested["result"]!["kind"]!.GetValue<string>());
    }
}
