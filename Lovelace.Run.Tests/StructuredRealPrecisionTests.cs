using System.Text.Json.Nodes;

namespace Lovelace.Run.Tests;

/// <summary>
/// L1 (audit B — <c>docs/goal-cycle-6/round-13/audit-B-lattice.md:40</c>): the structured payload
/// silently truncated every Real at 100 decimal places, with no truncation marker.
/// <c>setprecision(1100); pi(1100)</c> rendered 1 100 decimals in the display and in the captured
/// print channel while <c>result.structured.value</c> carried exactly 100 (102 characters) and no
/// field said the payload had been cut.
///
/// <para><b>The contract pinned here: (a) the structured value carries the value's OWN STORED digits,
/// in full.</b> The repository documents no bound on a structured Real:
/// <list type="bullet">
/// <item><c>docs/symbolics/dsh-protocol.md:28-41</c> lists the value forms and bounds no Real
/// <c>value</c>; the one structured bound the document does define (<c>--print-budget</c>,
/// <c>:17-20</c>) must cross as <c>truncated</c>/<c>truncationReason</c>/<c>budget</c> and is
/// described as "never silently". The whole document exists so that "an agent never has to parse a
/// display string to recover mathematical meaning" (<c>:3-5</c>).</item>
/// <item><c>Lovelace.Suite/StructuredProjection.cs:168-182</c> renders the Real as
/// <c>value.ToString()</c> and comments only on the exactness flag, never on a digit bound.</item>
/// <item><c>Lovelace.Real/Real.cs:54-58</c> documents <c>DisplayDecimalPlaces</c> as a DISPLAY
/// control — "Controls how many fractional digits appear in <c>ToString()</c> for non-periodic
/// values" — and that is exactly how the symbolic tier treats it: the value-to-literal crossing
/// reads the "full-precision magnitude and exponent — never the display-truncated string
/// (<c>ToString</c> respects the ambient display scope)" (<c>Lovelace.Symbolics/Expr.cs:104-112</c>).
/// </item>
/// </list>
/// The machine payload therefore carries the digits the value stores, whatever the display setting in
/// force would have shown; it never pads beyond them and never claims a truncation it did not do. The
/// digit counts asserted below are the counts the requests produce, cross-checked against mpmath
/// (<c>mp.dps=1200</c>, <c>Category=Timing</c>-free).</para>
/// </summary>
public class StructuredRealPrecisionTests
{
    /// <summary>pi's first 60 fractional digits (mpmath, <c>mp.dps = 1200</c>).</summary>
    private const string PiFirst60 =
        "141592653589793238462643383279502884197169399375105820974944";

    /// <summary>pi's fractional digits 981-1000 — the tail a 1 000-place request must own.</summary>
    private const string Pi981To1000 = "66111959092164201989";

    /// <summary>pi's fractional digits 1081-1100 — the tail only a 1 100-place request has.</summary>
    private const string Pi1081To1100 = "24972177528347913151";

    private static async Task<JsonNode> RunAsync(string script, params string[] extra)
    {
        var (exitCode, stdout, stderr) = await TestSupport.RunScriptAsync(script, extra);
        Assert.True(exitCode == 0, $"'{script}' exited {exitCode}. stderr: {stderr.Trim()}");
        return TestSupport.ParseExactlyOneJsonDocument(stdout, $"stdout of '{script}'");
    }

    private static JsonNode StructuredResult(JsonNode envelope) => envelope["result"]!["structured"]!;

    private static JsonNode ResultVariable(JsonNode envelope) =>
        envelope["variables"]!.AsArray().Single(v => v!["name"]!.GetValue<string>() == "_")!;

    private static string RealValue(JsonNode structured) => structured["value"]!.GetValue<string>();

    /// <summary>The fractional-digit count of a decimal rendering — the unit the finding measures in.
    /// A <c>typed</c> string carries the type suffix after a space, which is not a digit.</summary>
    private static int Decimals(string rendering)
    {
        int suffix = rendering.IndexOf(' ');
        if (suffix >= 0)
            rendering = rendering[..suffix];
        int dot = rendering.IndexOf('.');
        return dot < 0 ? 0 : rendering.Length - dot - 1;
    }

    /// <summary>The finding's own repro, at the digits the script asked for: 1 100 places in, 1 100
    /// places out — the same 1 100 the human rendering and the variable's structured form carry.</summary>
    [Fact]
    public async Task RequestedPrecisionBeyondTheOldCap_CrossesEveryStoredDigit()
    {
        JsonNode envelope = await RunAsync("setprecision(1100); pi(1100)", "--omit-functions");

        JsonNode structured = StructuredResult(envelope);
        string value = RealValue(structured);
        Assert.Equal(1100, Decimals(value));
        Assert.StartsWith("3." + PiFirst60, value, StringComparison.Ordinal);
        Assert.EndsWith(Pi1081To1100, value, StringComparison.Ordinal);

        // the value is not claimable as exact just because every stored digit crossed, and a
        // truncated irrational still publishes no rational form
        Assert.False(structured["exact"]!.GetValue<bool>());
        Assert.Null(structured["numerator"]);

        // nothing was cut, so nothing may be marked as cut
        Assert.Null(structured["truncated"]);

        // the same value in scope crosses identically: one projection, one digit count
        Assert.Equal(value, RealValue(ResultVariable(envelope)["structured"]!));
    }

    /// <summary>A count the machine API was asked for is part of the VALUE, not of the display
    /// setting: <c>evalf(pi, 1000)</c> stores 1 000 digits at the DEFAULT ambient precision, so the
    /// payload carries 1 000 even though the human rendering in force shows 100.</summary>
    [Fact]
    public async Task EvalfCountAboveTheAmbientDisplayPrecision_CrossesInFull()
    {
        JsonNode envelope = await RunAsync("evalf(pi, 1000)", "--omit-functions");

        string value = RealValue(StructuredResult(envelope));
        Assert.Equal(1000, Decimals(value));
        Assert.StartsWith("3." + PiFirst60, value, StringComparison.Ordinal);
        Assert.EndsWith(Pi981To1000, value, StringComparison.Ordinal);
        Assert.Equal(value, RealValue(ResultVariable(envelope)["structured"]!));
    }

    /// <summary>The counts BELOW the old 100-place cap were already whole (the finding's own control
    /// cases): the fix must leave them exactly where they were, neither cut nor padded.</summary>
    [Theory]
    [InlineData("pi(30)", 30)]
    [InlineData("evalf(pi, 40)", 40)]
    [InlineData("setprecision(1100); sqrt(2)", 1100)]
    [InlineData("setprecision(1100); e(1100)", 1100)]
    public async Task CountsAtOrBelowTheRequest_AreCarriedExactly(string script, int expectedDigits)
    {
        JsonNode envelope = await RunAsync(script, "--omit-functions");
        Assert.Equal(expectedDigits, Decimals(RealValue(StructuredResult(envelope))));
        Assert.Equal(expectedDigits, Decimals(RealValue(ResultVariable(envelope)["structured"]!)));
    }

    /// <summary>The payload carries the digits the value HAS and no more: a 5-place request crosses
    /// as five places, never padded out to a bound.</summary>
    [Fact]
    public async Task StoredDigitsAreNotPaddedToAnyBound()
    {
        JsonNode envelope = await RunAsync("evalf(sqrt(2), 5)", "--omit-functions");
        Assert.Equal("1.41421", RealValue(StructuredResult(envelope)));
    }

    /// <summary>Nested Reals are not a special case: the rendering room belongs to the projection,
    /// so every Real inside a container crosses with its stored digits too.</summary>
    [Fact]
    public async Task NestedReals_InsideAnArray_CrossInFullToo()
    {
        JsonNode envelope = await RunAsync("setprecision(1100); [pi(1100), e(1100)]", "--omit-functions");

        JsonNode elements = StructuredResult(envelope)["elements"]!;
        Assert.Equal(1100, Decimals(RealValue(elements[0]!)));
        Assert.Equal(1100, Decimals(RealValue(elements[1]!)));
    }

    /// <summary>The human rendering of the result follows the script's precision too — the setting
    /// the identical value in <c>variables[]</c> is already rendered under (A2-F24's "one set of
    /// settings"), never the process default the result path used to fall back to.</summary>
    [Fact]
    public async Task TheResultsDisplay_FollowsTheScriptsPrecision()
    {
        JsonNode envelope = await RunAsync("setprecision(1100); pi(1100)", "--omit-functions");

        Assert.Equal(1100, Decimals(envelope["result"]!["display"]!.GetValue<string>()));
        Assert.Equal(1100, Decimals(envelope["result"]!["typed"]!.GetValue<string>()));
        Assert.Equal(1100, Decimals(ResultVariable(envelope)["display"]!.GetValue<string>()));
    }
}
