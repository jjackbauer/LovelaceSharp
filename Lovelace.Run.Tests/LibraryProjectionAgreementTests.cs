using System.Text.Json.Nodes;
using Lovelace.Suite;
using Lovelace.Symbolics;

namespace Lovelace.Run.Tests;

/// <summary>
/// K-2 (round-21 audit K — the product assemblies driven as a LIBRARY, outside the CLI):
/// <see cref="SuiteEngine.ProjectValue"/>, the façade the class documentation points an embedding
/// host at, projected a Real under the engine's DISPLAY precision. The finding's own shape —
/// <c>ComputationDecimalPlaces = 1100; DisplayDecimalPlaces = 100</c> then <c>pi(1100)</c> — owns
/// 1 100 fractional digits, but the façade's structured <c>value</c> carried 100 of them with
/// <c>truncated: null</c>: a machine payload wrong by truncation whose own completeness flag denies
/// it. The SAME computation through the CLI (and through the Runner's own projection) publishes all
/// 1 100 — b3b74b8 landed that fix on the Runner's path only, so the two public entry points
/// published different payloads for one value.
///
/// <para>The contract pinned here is the one <c>docs/symbolics/dsh-protocol.md:17-28</c> states and
/// the Runner's projection already obeys: a structured Real carries the digits the value STORES,
/// whatever the display bound in force would have shown, and the three truncation fields are set
/// only when something really dropped digits (<c>evalf</c>'s documented 1000-place clamp). One
/// projection means a host and the CLI cannot publish different digits for one value.</para>
/// </summary>
public class LibraryProjectionAgreementTests
{
    /// <summary>pi's first 60 fractional digits (mpmath, <c>mp.dps = 1200</c>).</summary>
    private const string PiFirst60 =
        "141592653589793238462643383279502884197169399375105820974944";

    /// <summary>pi's fractional digits 1081-1100 — the tail only a 1 100-place request has.</summary>
    private const string Pi1081To1100 = "24972177528347913151";

    /// <summary>The reference wiring a host uses, exactly as the runner builds its engine
    /// (<c>Lovelace.Run/Runner.cs</c>).</summary>
    private static SuiteEngine NewEngine()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new Lovelace.Dsp.DspPlugin());
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new Lovelace.MathIR.MathIRPlugin(symbolics));
        return engine;
    }

    private static async Task<JsonNode> RunAsync(string script)
    {
        var (exitCode, stdout, stderr) = await TestSupport.RunScriptAsync(script, "--omit-functions");
        Assert.True(exitCode == 0, $"'{script}' exited {exitCode}. stderr: {stderr.Trim()}");
        return TestSupport.ParseExactlyOneJsonDocument(stdout, $"stdout of '{script}'");
    }

    /// <summary>The fractional-digit count of a decimal rendering — the unit the finding measures
    /// in. A <c>typed</c> string carries a type suffix after a space, which is not a digit.</summary>
    private static int Decimals(string rendering)
    {
        int suffix = rendering.IndexOf(' ');
        if (suffix >= 0)
            rendering = rendering[..suffix];
        int dot = rendering.IndexOf('.');
        return dot < 0 ? 0 : rendering.Length - dot - 1;
    }

    // ---------------------------------------------------------------------
    // The finding: the façade loses the digits the value owns
    // ---------------------------------------------------------------------

    /// <summary>The audit's exact shape, through the public façade: the display bound is below the
    /// computation bound, and the structured value must still carry every stored digit.</summary>
    [Fact]
    public void TheFacade_PublishesEveryStoredDigit_WhenTheDisplayBoundIsLower()
    {
        var engine = NewEngine();
        engine.ComputationDecimalPlaces = 1100;
        engine.DisplayDecimalPlaces = 100;

        Value value = engine.Evaluate("pi(1100)");
        StructuredValueDto dto = engine.ProjectValue(value);

        Assert.Equal("Real", dto.Kind);
        Assert.NotNull(dto.Value);
        Assert.Equal(1100, Decimals(dto.Value!));
        Assert.StartsWith("3." + PiFirst60, dto.Value!, StringComparison.Ordinal);
        Assert.EndsWith(Pi1081To1100, dto.Value!, StringComparison.Ordinal);

        // nothing was cut, so nothing may be marked as cut
        Assert.Null(dto.Truncated);
        Assert.Null(dto.TruncationReason);
        Assert.Null(dto.Budget);

        // and the DISPLAY bound stays a display bound: the human rendering is still the 100 the
        // engine's setting asks for, so the fix moved the payload and not the display
        Assert.Equal(100, Decimals(engine.FormatValue(value)));
    }

    // ---------------------------------------------------------------------
    // One projection: the façade and the CLI publish the same value
    // ---------------------------------------------------------------------

    /// <summary>The finding's own comparison: one computation, one engine setting, two public
    /// entry points. The digits the CLI puts on the wire are the digits the façade must publish.</summary>
    [Fact]
    public async Task TheFacade_AndTheCli_PublishTheSameDigits_ForTheFindingsScript()
    {
        var engine = NewEngine();
        engine.ComputationDecimalPlaces = 1100;
        engine.DisplayDecimalPlaces = 100;

        StructuredValueDto fromFacade = engine.ProjectValue(engine.Evaluate("pi(1100)"));

        JsonNode fromCli = (await RunAsync("setprecision(1100); pi(1100)"))["result"]!["structured"]!;

        Assert.Equal(fromCli["value"]!.GetValue<string>(), fromFacade.Value);
        Assert.Equal(1100, Decimals(fromFacade.Value!));
    }

    /// <summary>The agreement as a property over the shapes that reach a Real: a requested count
    /// above the old 100-digit display default, a count at the default computation cap, a short
    /// count, a periodic quotient, an exact quotient and a plain irrational.</summary>
    public static IEnumerable<object[]> SharedScripts()
    {
        yield return new object[] { "setprecision(1100); pi(1100)" };
        yield return new object[] { "setprecision(1100); sqrt(2)" };
        yield return new object[] { "setprecision(1100); e(1100)" };
        yield return new object[] { "evalf(pi, 1000)" };
        yield return new object[] { "pi(30)" };
        yield return new object[] { "evalf(sqrt(2), 5)" };
        yield return new object[] { "sqrt(2)" };
        yield return new object[] { "1/7" };
        yield return new object[] { "1/8" };
    }

    [Theory]
    [MemberData(nameof(SharedScripts))]
    public async Task TheFacade_AndTheCli_AgreeOnOneStructuredValue(string script)
    {
        var engine = NewEngine();
        StructuredValueDto facade = engine.ProjectValue(engine.Evaluate(script));

        JsonNode cli = (await RunAsync(script))["result"]!["structured"]!;

        Assert.Equal(cli["kind"]!.GetValue<string>(), facade.Kind);
        Assert.Equal(cli["value"]?.GetValue<string>(), facade.Value);
        Assert.Equal(cli["exact"]?.GetValue<bool>(), facade.Exact);
        Assert.Equal(cli["truncated"]?.GetValue<bool>(), facade.Truncated);
        Assert.Equal(cli["truncationReason"]?.GetValue<string>(), facade.TruncationReason);
        Assert.Equal(cli["budget"]?.GetValue<int>(), facade.Budget);
        Assert.Equal(cli["numerator"]?.GetValue<string>(), facade.Numerator);
        Assert.Equal(cli["denominator"]?.GetValue<string>(), facade.Denominator);
    }

    /// <summary>The room belongs to the projection, not to the result path: a Real nested in a
    /// container crosses with its stored digits through the façade too.</summary>
    [Fact]
    public void NestedReals_ThroughTheFacade_CrossInFullToo()
    {
        var engine = NewEngine();
        engine.ComputationDecimalPlaces = 1100;
        engine.DisplayDecimalPlaces = 100;

        StructuredValueDto dto = engine.ProjectValue(engine.Evaluate("[pi(1100), e(1100)]"));

        Assert.Equal("Array", dto.Kind);
        Assert.Equal(1100, Decimals(dto.Elements![0].Value!));
        Assert.Equal(1100, Decimals(dto.Elements![1].Value!));
    }

    // ---------------------------------------------------------------------
    // What must NOT move: the values that were already whole
    // ---------------------------------------------------------------------

    /// <summary>A value that stores fewer digits than the display bound shows is neither cut nor
    /// padded: the payload carries exactly the digits the value has.</summary>
    [Fact]
    public void AnUntruncatedValue_IsNeitherCutNorPadded_ThroughTheFacade()
    {
        var engine = NewEngine();
        Assert.Equal("1.41421", engine.ProjectValue(engine.Evaluate("evalf(sqrt(2), 5)")).Value);
        Assert.Equal(30, Decimals(engine.ProjectValue(engine.Evaluate("pi(30)")).Value!));
    }

    /// <summary>An exact Real is still published as its rational, with the three truncation fields
    /// absent — the fix moves digits, not provenance.</summary>
    [Fact]
    public void AnExactReal_StillCrossesAsItsRational_ThroughTheFacade()
    {
        var engine = NewEngine();
        StructuredValueDto dto = engine.ProjectValue(engine.Evaluate("1/8"));

        Assert.Equal("0.125", dto.Value);
        Assert.True(dto.Exact);
        Assert.Equal("1", dto.Numerator);
        Assert.Equal("8", dto.Denominator);
        Assert.Null(dto.Truncated);
    }

    // ---------------------------------------------------------------------
    // The truncation that IS real is still marked, on this surface too
    // ---------------------------------------------------------------------

    /// <summary>The one bound the ENGINE applies of its own accord — <c>evalf</c>'s documented
    /// 1000-place clamp — crosses on the façade exactly as it does on the CLI: the digits the value
    /// really has, and the protocol's three fields saying the caller's larger request was cut
    /// (<c>docs/symbolics/dsh-protocol.md:24-28</c>). The CLI half of this contract is pinned by
    /// <c>EvalfDigitCapHonestyTests</c>; this is the same fact through the library façade.</summary>
    [Fact]
    public async Task AClampedValue_IsMarked_ThroughTheFacade_AsItIsOnTheCli()
    {
        var engine = NewEngine();
        engine.ComputationDecimalPlaces = 5000;
        engine.DisplayDecimalPlaces = 100;

        StructuredValueDto facade = engine.ProjectValue(engine.Evaluate("evalf(sqrt(2), 5000)"));
        JsonNode cli = (await RunAsync("setprecision(5000); evalf(sqrt(2), 5000)"))["result"]!["structured"]!;

        Assert.Equal(1000, Decimals(facade.Value!));
        Assert.Equal(cli["value"]!.GetValue<string>(), facade.Value);
        Assert.True(facade.Truncated == true,
            $"the 1000-place clamp crossed the façade unmarked: {facade.Value!.Length - 2} digits");
        Assert.Equal("digit-cap", facade.TruncationReason);
        Assert.Equal(1000, facade.Budget);
    }
}
