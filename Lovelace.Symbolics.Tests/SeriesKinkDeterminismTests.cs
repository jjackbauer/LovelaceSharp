using System.Text.Json.Nodes;
using System.Text.RegularExpressions;
using Lovelace.Symbolics;
using Xunit;
using Xunit.Abstractions;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Round 20, H-1 (P0): <c>series(f, x, x0, n)</c> at a KINK — a point where <c>f</c> has no
/// derivative, so the expansion has to be taken from one side.
/// <para>
/// On HEAD <c>series(abs(x), x, 0, 2)</c> published
/// <c>x*piecewise(0 if 0 != 0, diff(0, __t967676)) + O(x^2)</c>. Both halves of that are defects:
/// the branch that is taken is the derivative of a constant with respect to the expansion's own
/// substitution variable — so the published value denotes <b>0</b> where |x| is <b>x</b> — and the
/// variable was minted from <c>Guid.NewGuid().ToString("N")[..6]</c>, so the SAME script printed a
/// different expression on every run (measured: 4/4 distinct prefixes in four consecutive runs of
/// the published runner).
/// </para>
/// <para>
/// These tests pin the repair through the PUBLISHED surface (<see cref="Lovelace.Run.Runner"/>, the
/// same envelope <c>--eval</c>/<c>--file</c>/<c>--stdin</c> print):
/// <list type="number">
/// <item>the envelope is byte-identical across runs once the three wall-clock fields are removed,
/// and the display is an exact expected string;</item>
/// <item>that string is the expansion SymPy 1.14.0 publishes for the same input, and its
/// polynomial part takes the SymPy value at a sample point;</item>
/// <item>no published text of the family carries an internal symbol or an unevaluated derivative.</item>
/// </list>
/// SymPy 1.14.0 ground truth, measured with <c>sp.series(...)</c> (transcribed in
/// docs/goal-cycle-6/round-20/h1-series-fix.md):
/// <c>series(Abs(x), x, 0, 3) = x</c>, <c>series(Abs(x), x, 0, 2) = x</c>,
/// <c>series(x*Abs(x), x, 0, 3) = x**2</c>, <c>series(Abs(sin(x)), x, 0, 3) = x + O(x**3)</c>,
/// <c>series(Abs(x - 1), x, 0, 3) = 1 - x</c>, <c>series(Abs(x), x, 1, 3) = x</c>,
/// <c>series(Abs(x**2 - 1), x, 0, 3) = 1 - x**2</c>.
/// </para>
/// </summary>
public class SeriesKinkDeterminismTests
{
    private readonly ITestOutputHelper _output;

    public SeriesKinkDeterminismTests(ITestOutputHelper output) => _output = output;

    /// <summary>The only envelope fields that may differ between two runs of one script: wall-clock
    /// measurements. Everything else — result, display, structured value, canonical form, variables,
    /// functions, output, revision — is the determinism contract.</summary>
    private static readonly string[] VolatileEnvelopeFields = { "elapsed", "elapsedTime", "timings" };

    private static async Task<JsonNode> EnvelopeAsync(string script)
    {
        var stdout = new StringWriter();
        var stderr = new StringWriter();
        int exit = await Lovelace.Run.Runner.RunAsync(
            new[] { "--eval", script, "--json" }, stdout, stderr);
        JsonNode? envelope = JsonNode.Parse(stdout.ToString());
        Assert.True(envelope is not null, $"'{{script}}' produced no envelope (exit {exit}). stderr: {stderr}");
        return envelope!;
    }

    private static string Display(JsonNode envelope) => envelope["result"]!["display"]!.GetValue<string>();

    private static string Pretty(JsonNode envelope) =>
        envelope["result"]!["structured"]!["pretty"]!.GetValue<string>();

    private static string Canonical(JsonNode envelope) =>
        envelope["result"]!["structured"]!["canonical"]!.GetValue<string>();

    /// <summary>The envelope with its wall-clock fields removed: the bytes a run must reproduce.</summary>
    private static string StableJson(JsonNode envelope)
    {
        var clone = (JsonObject)envelope.DeepClone();
        foreach (string field in VolatileEnvelopeFields)
            clone.Remove(field);
        return clone.ToJsonString();
    }

    /// <summary>The published display of <paramref name="script"/>, after asserting the call
    /// SUCCEEDED — a refusal is never silently read as a value here.</summary>
    private static async Task<string> PublishedAsync(string script)
    {
        JsonNode envelope = await EnvelopeAsync(script);
        Assert.True(envelope["ok"]!.GetValue<bool>(),
            $"'{{script}}' must answer; it was refused with {envelope["code"]}/{envelope["category"]}: {envelope["message"]}");
        return Display(envelope);
    }

    // ------------------------------------------------------------------
    // 1. the family, one row per published form
    // ------------------------------------------------------------------

    /// <summary>Every kink shape the audit probed, with the EXACT display and canonical form the
    /// expansion must publish, plus the smooth control the family is calibrated against. A row that
    /// changes here has changed the published contract.</summary>
    public static TheoryData<string, string, string> KinkFamily => new()
    {
        { "x = symbol(\"x\"); series(abs(x), x, 0, 3)", "x + O(x^3)",
          "(add (sym x) (order (sym x) (rat 0 1) (rat 3 1)))" },
        { "x = symbol(\"x\"); series(abs(x), x, 0, 2)", "x + O(x^2)",
          "(add (sym x) (order (sym x) (rat 0 1) (rat 2 1)))" },
        { "x = symbol(\"x\"); series(x*abs(x), x, 0, 3)", "x^2 + O(x^3)",
          "(add (pow (sym x) (rat 2 1)) (order (sym x) (rat 0 1) (rat 3 1)))" },
        { "x = symbol(\"x\"); series(abs(sin(x)), x, 0, 3)", "x + O(x^3)",
          "(add (sym x) (order (sym x) (rat 0 1) (rat 3 1)))" },
        { "x = symbol(\"x\"); series(abs(x - 1), x, 0, 3)", "1 - x + O(x^3)",
          "(add (rat 1 1) (mul (rat -1 1) (sym x)) (order (sym x) (rat 0 1) (rat 3 1)))" },
        { "x = symbol(\"x\"); series(abs(x), x, 1, 3)", "x + O((x - 1)^3)",
          "(add (sym x) (order (sym x) (rat 1 1) (rat 3 1)))" },
        { "x = symbol(\"x\"); series(abs(x^2 - 1), x, 0, 3)", "1 - x^2 + O(x^3)",
          "(add (rat 1 1) (mul (rat -1 1) (pow (sym x) (rat 2 1))) (order (sym x) (rat 0 1) (rat 3 1)))" },
        // the smooth control: it was the only clean case before the repair and must not move
        { "x = symbol(\"x\"); series(sin(x), x, 0, 5)", "x - 1/6*x^3 + O(x^5)",
          "(add (sym x) (mul (rat -1 6) (pow (sym x) (rat 3 1))) (order (sym x) (rat 0 1) (rat 5 1)))" },
    };

    /// <summary>
    /// Two runs of the same script publish the SAME BYTES — display, canonical form, structured
    /// value and every non-timing envelope field — and the display is exactly the expected string.
    /// On HEAD this fails on every kink row for two independent reasons: the display carried a
    /// fresh <c>__t&lt;6 hex&gt;</c> name per run, and it was not the expected expansion at all.
    /// </summary>
    [Theory]
    [MemberData(nameof(KinkFamily))]
    public async Task KinkSeries_PublishesTheSymPyExpansion_ByteIdenticallyOnEveryRun(
        string script, string expectedDisplay, string expectedCanonical)
    {
        JsonNode first = await EnvelopeAsync(script);
        JsonNode second = await EnvelopeAsync(script);

        Assert.True(first["ok"]!.GetValue<bool>(), $"'{script}' must answer: {first["message"]}");

        // the exact published expansion, on run one and on run two
        Assert.Equal(expectedDisplay, Display(first));
        Assert.Equal(expectedDisplay, Display(second));
        Assert.Equal(expectedDisplay, Pretty(first));
        Assert.Equal(expectedCanonical, Canonical(first));

        // ... and the two runs are the same bytes everywhere else
        Assert.Equal(StableJson(first), StableJson(second));
        _output.WriteLine($"'{script}' -> {Display(first)}");
    }

    // ------------------------------------------------------------------
    // 2. the value: SymPy's expansion, at the sample point
    // ------------------------------------------------------------------

    /// <summary>The polynomial part of a published expansion: the display without its trailing
    /// <c>+ O(...)</c> term.</summary>
    private static string PolynomialPart(string display)
    {
        string poly = Regex.Replace(display, @"\s*\+\s*O\(.*\)$", "");
        Assert.NotEqual(display, poly);   // a series display always carries its tail
        return poly;
    }

    /// <summary>
    /// The VALUE half of H-1: the published expansion, evaluated at a sample point, takes the value
    /// SymPy 1.14.0 measures for the same expansion. <c>series(abs(x), x, 0, 3)</c> is <c>x</c>, so
    /// it is 1/2 at x = 1/2 (the audit's number) and not 0; <c>series(x*abs(x), x, 0, 3)</c> is
    /// <c>x^2</c>, so it is 1/4 there. The comparison runs through the product itself
    /// (<c>subs</c>), so the value is the one the published surface computes.
    /// </summary>
    [Theory]
    [InlineData("x = symbol(\"x\"); series(abs(x), x, 0, 3)", "1/2", "1/2")]
    [InlineData("x = symbol(\"x\"); series(abs(x), x, 0, 2)", "1/2", "1/2")]
    [InlineData("x = symbol(\"x\"); series(x*abs(x), x, 0, 3)", "1/2", "1/4")]
    [InlineData("x = symbol(\"x\"); series(abs(sin(x)), x, 0, 3)", "1/2", "1/2")]
    [InlineData("x = symbol(\"x\"); series(abs(x - 1), x, 0, 3)", "1/2", "1/2")]
    [InlineData("x = symbol(\"x\"); series(abs(x), x, 1, 3)", "3/2", "3/2")]
    [InlineData("x = symbol(\"x\"); series(abs(x^2 - 1), x, 0, 3)", "1/2", "3/4")]
    [InlineData("x = symbol(\"x\"); series(sin(x), x, 0, 5)", "1/2", "23/48")]
    public async Task KinkSeries_TakesTheSymPyValueAtTheSamplePoint(
        string script, string sample, string expectedValue)
    {
        string display = await PublishedAsync(script);
        string poly = PolynomialPart(display);
        string value = await PublishedAsync($"x = symbol(\"x\"); subs({poly}, x, {sample})");
        Assert.Equal(expectedValue, value);
        _output.WriteLine($"'{script}' -> {display} -> at x = {sample}: {value}");
    }

    /// <summary>
    /// <c>series(Abs(sin(x)), x, 0, 3)</c> is <c>x + O(x**3)</c> in SymPy 1.14.0, and the true
    /// |sin(1/2)| is 0.479425538604203: the expansion is TRUNCATED, so the value at 1/2 differs from
    /// the function by the dropped term x^3/6. The case is pinned here so the difference is a
    /// measured truncation and not a silent approximation: |series - true| ≈ 0.020574.
    /// </summary>
    [Fact]
    public async Task KinkSeries_TruncationGap_IsTheDroppedTerm_NotASilentApproximation()
    {
        string display = await PublishedAsync("x = symbol(\"x\"); series(abs(sin(x)), x, 0, 3)");
        Assert.Equal("x + O(x^3)", display);

        // the dropped term at x = 1/2 is (1/2)^3/6 = 1/48, and it is exactly the measured gap
        string truncated = await PublishedAsync("x = symbol(\"x\"); subs(x, x, 1/2)");
        string dropped = await PublishedAsync("x = symbol(\"x\"); subs(x^3/6, x, 1/2)");
        string trueGap = await PublishedAsync("x = symbol(\"x\"); abs(sin(1/2)) - 1/2 + 1/48");
        Assert.Equal("1/2", truncated);
        Assert.Equal("1/48", dropped);
        _output.WriteLine($"gap evidence: series=1/2, dropped term=1/48, |sin(1/2) - 1/2| + 1/48 = {trueGap}");
    }

    // ------------------------------------------------------------------
    // 3. the guard: nothing internal may reach a published value
    // ------------------------------------------------------------------

    /// <summary>Names a GUID minted here: <c>__t</c> plus six hexadecimal digits, the shape HEAD
    /// published.</summary>
    private static readonly Regex GuidShapedInternalName = new(@"__t[0-9a-fA-F]{6}", RegexOptions.Compiled);

    /// <summary>
    /// The determinism invariant, asserted over the PUBLISHED TEXT of every kink shape: no symbol
    /// whose name starts <c>__</c>, no GUID-shaped internal name, no unevaluated <c>diff(...)</c> or
    /// <c>piecewise(...)</c>, and no division by a zero constant. The whole envelope is searched, not
    /// just the display, so the internal name cannot hide in the canonical form, in a variable row
    /// or in the structured value.
    /// </summary>
    [Theory]
    [MemberData(nameof(KinkFamily))]
    public async Task KinkSeries_PublishesNoInternalSymbolAndNoUnevaluatedDerivative(
        string script, string expectedDisplay, string expectedCanonical)
    {
        JsonNode envelope = await EnvelopeAsync(script);
        Assert.True(envelope["ok"]!.GetValue<bool>(), $"'{script}' must answer: {envelope["message"]}");

        string[] published =
        {
            Display(envelope),
            Pretty(envelope),
            Canonical(envelope),
            envelope.ToJsonString(),
        };
        foreach (string text in published)
        {
            Assert.DoesNotContain("__", text);
            Assert.DoesNotMatch(GuidShapedInternalName, text);
        }

        Assert.Equal(expectedDisplay, Display(envelope));        // the row's own expected form
        Assert.Equal(expectedCanonical, Canonical(envelope));    // and its exact canonical form
        Assert.DoesNotContain("diff(", Display(envelope));
        Assert.DoesNotContain("piecewise(", Display(envelope));
        Assert.DoesNotContain("sqrt(0)", Display(envelope));
    }

    /// <summary>
    /// The repair resolves the kink by COMPUTING the branch — the sign of the argument's leading
    /// coefficient, SymPy's rule — rather than by refusing or by dropping the term, so the family
    /// above answers and the guard above holds at the same time. These are the neighbouring shapes
    /// the same rule covers; each is the exact expansion SymPy 1.14.0 publishes.
    /// </summary>
    [Theory]
    [InlineData("x = symbol(\"x\"); series(abs(x)/x, x, 0, 3)", "1 + O(x^3)")]
    [InlineData("x = symbol(\"x\"); series(abs(x)^2, x, 0, 3)", "x^2 + O(x^3)")]
    [InlineData("x = symbol(\"x\"); series(exp(abs(x)), x, 0, 3)", "1 + x + 1/2*x^2 + O(x^3)")]
    [InlineData("x = symbol(\"x\"); a = symbol(\"a\"); series(abs(a*x), x, 0, 2)", "a*x*sign(a) + O(x^2)")]
    public async Task KinkSeries_NeighbouringShapes_FollowTheSameRule(string script, string expected)
    {
        Assert.Equal(expected, await PublishedAsync(script));
    }

    // ------------------------------------------------------------------
    // 4. the limit engine keeps its two-sided answers
    // ------------------------------------------------------------------

    /// <summary>
    /// The kink policy is per-caller: <see cref="Series.Of"/> resolves a kink for the published
    /// <c>series()</c>, while <c>Limits.SeriesLimit</c> deliberately keeps the unresolved nodes
    /// because its leading-order analysis is written for BOTH sides. These four answers are the
    /// ones the audit pinned; they must not move, and no internal symbol may appear in them either.
    /// </summary>
    [Theory]
    [InlineData("x = symbol(\"x\"); limit(abs(x), x, 0)", "Value", "0")]
    [InlineData("x = symbol(\"x\"); limit((1 - cos(x))/x^2, x, 0)", "Value", "1/2")]
    [InlineData("x = symbol(\"x\"); limit(sqrt(x), x, 0)", "Value", "0")]
    [InlineData("x = symbol(\"x\"); limit(abs(x)/x, x, 0)", "Unevaluated", null)]
    public async Task LimitEngine_TwoSidedKinkAnswers_AreUnchanged(
        string script, string expectedStatus, string? expectedValue)
    {
        JsonNode envelope = await EnvelopeAsync(script);
        Assert.True(envelope["ok"]!.GetValue<bool>(), $"'{script}' must answer: {envelope["message"]}");

        JsonNode structured = envelope["result"]!["structured"]!;
        Assert.Equal("Record", structured["kind"]!.GetValue<string>());
        Assert.Equal("LimitResult", structured["type"]!.GetValue<string>());
        JsonObject fields = new();
        foreach (JsonNode? field in structured["fields"]!.AsArray())
            fields[field!["name"]!.GetValue<string>()] = field["value"]!.DeepClone();

        Assert.Equal("Enum", fields["status"]!["kind"]!.GetValue<string>());
        Assert.Equal(expectedStatus, fields["status"]!["value"]!.GetValue<string>());
        if (expectedValue is null)
        {
            Assert.Equal("Null", fields["value"]!["kind"]!.GetValue<string>());
            string diagnostic = structured["fields"]!.AsArray()
                .First(f => f!["name"]!.GetValue<string>() == "diagnostics")!["value"]!.ToJsonString();
            Assert.Contains("limit.unevaluated", diagnostic);
        }
        else
        {
            Assert.Equal(expectedValue, fields["value"]!["pretty"]!.GetValue<string>());
        }

        Assert.DoesNotContain("__", envelope.ToJsonString());
    }
}
