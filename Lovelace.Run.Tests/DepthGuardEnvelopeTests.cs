using System.Text.Json.Nodes;
using Xunit;

namespace Lovelace.Run.Tests;

/// <summary>
/// Valid-but-DEEP input must be refused structurally, never by dying: on the pre-guard tree every
/// shape below reaches a native stack overflow, the process is terminated by the OS with
/// 0xC00000FD, stdout carries NO envelope at all, and the agent-facing protocol has nothing to
/// report. The guard under test turns each of those deaths into one JSON envelope with a
/// non-zero exit code and a code/category that names the condition.
/// <para>
/// The code/category names are asserted as WIRE STRINGS (not as a reference to the exception type)
/// so this file compiles against the pre-fix tree as well — that is what makes the control-tree
/// evidence possible.
/// </para>
/// </summary>
public class DepthGuardEnvelopeTests
{
    /// <summary>The nesting budget: a parser descent or a canonical symbolic expression
    /// (Lovelace.Abstractions.InputDepth.Max).</summary>
    private const int MaxDepth = 256;

    /// <summary>The parsed-tree budget: only the interpreter's walk over the PARSED tree reads it,
    /// and only a flat left-associative chain can reach it without nesting
    /// (Lovelace.Abstractions.InputDepth.MaxParsedTreeDepth).</summary>
    private const int MaxParsedTreeDepth = 512;

    /// <summary>The wire code naming a depth refusal.</summary>
    private const string DepthCode = "DepthExceeded";

    /// <summary>Its category: an exhausted budget, not a domain error and not an invariant failure.</summary>
    private const string DepthCategory = "BudgetExceeded";

    private static string Repeat(string text, int count) => string.Concat(Enumerable.Repeat(text, count));

    // ------------------------------------------------------------------
    // The shapes from the cycle-4 audit, at the published depth of 3000
    // ------------------------------------------------------------------

    public static IEnumerable<object[]> DeepShapes()
    {
        yield return new object[] { "nested parentheses", Repeat("(", 3000) + "1" + Repeat(")", 3000) + ";" };
        yield return new object[] { "unary minus chain", Repeat("-", 3000) + "1;" };
        yield return new object[] { "nested abs() calls", Repeat("abs(", 3000) + "1" + Repeat(")", 3000) + ";" };
        yield return new object[] { "nested (+1) groups", Repeat("(", 3000) + "1" + Repeat("+1)", 3000) + ";" };
        yield return new object[] { "nested list literals", Repeat("[", 3000) + "1" + Repeat("]", 3000) + ";" };
        yield return new object[] { "nested sqrt() calls", Repeat("sqrt(", 3000) + "1" + Repeat(")", 3000) + ";" };
        yield return new object[] { "flat subtraction chain (20000)", "1" + Repeat("-1", 20000) + ";" };
        yield return new object[] { "flat addition chain (20000)", "1" + Repeat("+1", 20000) + ";" };
    }

    [Theory]
    [MemberData(nameof(DeepShapes))]
    public async Task DeepInput_IsATypedRefusal_NeverAProcessDeath(string what, string script)
    {
        var (exitCode, envelope, stdout) = await RunAsync(script);

        Assert.True(exitCode != 0, $"{what}: a refused input must exit non-zero (got {exitCode})");
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"{what}: ok must be false. Envelope: {stdout}");
        Assert.Equal(DepthCode, envelope["code"]!.GetValue<string>());
        Assert.Equal(DepthCategory, envelope["category"]!.GetValue<string>());
        Assert.True(envelope["recoverable"]!.GetValue<bool>(),
            $"{what}: the caller can retry with a shallower input, so the refusal is recoverable");
        // the message names the measured depth and the budget it broke
        Assert.Matches(
            @"depth \d+ exceeds the maximum supported nesting depth of (256|512)\.",
            envelope["message"]!.GetValue<string>());
    }

    /// <summary>
    /// A SHALLOW script that builds a deep symbolic VALUE at run time (nothing deep in the source,
    /// nothing deep in the parsed AST): without a guard at canonical construction this dies in
    /// Lovelace.Symbolics.Printing.Pretty while the result is being rendered.
    /// </summary>
    [Fact]
    public async Task LoopBuiltExpressionDepth_IsATypedRefusal()
    {
        var (exitCode, envelope, stdout) = await RunAsync(
            "e = symbol(\"x\"); for i in 1..1000 { e = abs(e) };");

        Assert.True(exitCode != 0, $"a value deeper than the budget must exit non-zero (got {exitCode})");
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"ok must be false. Envelope: {stdout}");
        Assert.Equal(DepthCode, envelope["code"]!.GetValue<string>());
        Assert.Equal(DepthCategory, envelope["category"]!.GetValue<string>());
    }

    // ------------------------------------------------------------------
    // The boundary: limit-1 accepted, limit+1 refused
    // ------------------------------------------------------------------

    /// <summary>Nested parentheses measuring EXACTLY <paramref name="depth"/>: the guard counts
    /// the statement and the outer expression before the first group, so <c>depth - 2</c> groups
    /// measure <paramref name="depth"/>. (Verified against the reported depth of the refusal at
    /// <c>depth + 1</c>: 254 groups are accepted, 255 report "depth 257".)</summary>
    private static string ParensAtDepth(int depth) =>
        Repeat("(", depth - 2) + "1" + Repeat(")", depth - 2) + ";";

    /// <summary>A flat left-associative chain measuring exactly <paramref name="depth"/>: the tree
    /// is program → statement → one BinaryExpr per operator → the leaf, so <c>depth - 3</c>
    /// operators measure <paramref name="depth"/>. (Verified: 253 operators are accepted, 254
    /// report "tree depth 257".)</summary>
    private static string FlatChainAtDepth(int depth) => "1" + Repeat("-1", depth - 3) + ";";

    [Fact]
    public async Task InputAtTheLimit_IsAccepted()
    {
        var (exitCode, envelope, stdout) = await RunAsync(ParensAtDepth(MaxDepth));
        Assert.True(envelope["ok"]!.GetValue<bool>(),
            $"depth {MaxDepth} is exactly the budget and must be accepted. exit={exitCode} stdout={stdout}");
        Assert.Equal(0, exitCode);
    }

    [Fact]
    public async Task InputOnePastTheLimit_IsRefused()
    {
        var (exitCode, envelope, stdout) = await RunAsync(ParensAtDepth(MaxDepth + 1));
        Assert.True(exitCode != 0, $"depth {MaxDepth + 1} is past the budget (exit {exitCode}). stdout={stdout}");
        Assert.Equal(DepthCode, envelope["code"]!.GetValue<string>());
        Assert.Contains((MaxDepth + 1).ToString(), envelope["message"]!.GetValue<string>());
    }

    [Fact]
    public async Task FlatChainAtTheLimit_IsAccepted()
    {
        var (exitCode, envelope, stdout) = await RunAsync(FlatChainAtDepth(MaxParsedTreeDepth));
        Assert.True(envelope["ok"]!.GetValue<bool>(),
            $"a flat chain of depth {MaxParsedTreeDepth} must be accepted. exit={exitCode} stdout={stdout}");
        Assert.Equal(0, exitCode);
    }

    [Fact]
    public async Task FlatChainOnePastTheLimit_IsRefused()
    {
        var (exitCode, envelope, stdout) = await RunAsync(FlatChainAtDepth(MaxParsedTreeDepth + 1));
        Assert.True(exitCode != 0,
            $"a flat chain of depth {MaxParsedTreeDepth + 1} must be refused (exit {exitCode}). stdout={stdout}");
        Assert.Equal(DepthCode, envelope["code"]!.GetValue<string>());
        Assert.Contains((MaxParsedTreeDepth + 1).ToString(), envelope["message"]!.GetValue<string>());
    }

    /// <summary>A flat chain is legitimate, WIDE input: the shipped suites sum 420 terms, and that
    /// shape must keep working (its parsed tree is one level per operator, so it is measured
    /// against the parsed-tree budget, not the nesting budget).</summary>
    [Fact]
    public async Task AFlatSumOfHundredsOfTerms_IsStillAccepted()
    {
        var terms = Enumerable.Range(1, 420).Select(i => "exp(log(x+" + i + "))");
        var (exitCode, envelope, stdout) = await RunAsync(
            "x = symbol(\"x\"); simplify_full(" + string.Join(" + ", terms) + ")");

        Assert.Equal(0, exitCode);
        Assert.True(envelope["ok"]!.GetValue<bool>(), $"a 420-term flat sum must still run. Envelope: {stdout}");
    }

    // ------------------------------------------------------------------
    // The guard must not reject ordinary programs
    // ------------------------------------------------------------------

    /// <summary>5000 flat statements, each one level deep in the AST, is ordinary, WIDE input: a
    /// depth budget must not touch it (this is the "wide is not deep" control).</summary>
    [Fact]
    public async Task AWideProgramOfShallowStatements_IsStillAccepted()
    {
        string script = "e = 0;" + Repeat("e = e + 1;", 5000);
        var (exitCode, envelope, stdout) = await RunAsync(script);

        Assert.Equal(0, exitCode);
        Assert.True(envelope["ok"]!.GetValue<bool>(), $"5000 shallow statements must still run. Envelope: {stdout}");
        Assert.Equal("5000", envelope["result"]!["display"]!.GetValue<string>());
    }

    /// <summary>Ordinary nested arithmetic keeps working, and the values are unchanged.</summary>
    [Fact]
    public async Task OrdinaryNestedArithmetic_IsUnchanged()
    {
        var (exitCode, envelope, stdout) = await RunAsync(
            "x = symbol(\"x\"); e = -(-(-(1 + 2*3))) + sqrt(sqrt(16)) + abs(-2); e;");

        Assert.Equal(0, exitCode);
        Assert.True(envelope["ok"]!.GetValue<bool>(), $"ordinary arithmetic must still run. Envelope: {stdout}");
    }

    private static async Task<(int ExitCode, JsonNode Envelope, string Stdout)> RunAsync(string script)
    {
        var stdout = new StringWriter();
        var stderr = new StringWriter();
        int exitCode = await Runner.RunAsync(
            new[] { "--eval", script, "--omit-functions", "--omit-variables" }, stdout, stderr);
        string text = stdout.ToString();
        return (exitCode, TestSupport.ParseExactlyOneJsonDocument(text, "runner stdout"), text);
    }
}
