using System.Text.Json.Nodes;

namespace Lovelace.Run.Tests;

/// <summary>
/// Round-20 audit I, I-2: <c>evalf</c>'s 1000-place computation cap was SILENT and the builtin's own
/// published summary denied it. <c>setprecision(5000); evalf(sqrt(2), 5000)</c> crossed a value
/// carrying 1000 decimals with no <c>truncated</c>/<c>truncationReason</c>/<c>budget</c> and no
/// diagnostic, while the SAME binary's <c>setprecision(5000); sqrt(2)</c> carried 5000 — and the
/// descriptor said the value "is bounded back to the N places that were asked for"
/// (<c>Lovelace.Symbolics/SymbolicsPlugin.cs:498-500</c>), which is false for N &gt; 1000.
///
/// <para>The 1000-place bound itself is DOCUMENTED (EVD-276 records it) and stays: what these tests
/// pin is the caller-visible contract around it. When the cap actually drops digits the value says
/// so on the wire — <c>truncated: true</c>, <c>truncationReason: "digit-cap"</c>,
/// <c>budget: 1000</c> — the same three fields a bounded RENDERING reports
/// (<c>docs/symbolics/dsh-protocol.md:17-20</c>). A value the builtin could answer in full is never
/// marked: a count at the cap, and an exact answer to an over-cap request, both cross with the
/// fields absent.</para>
/// </summary>
public class EvalfDigitCapHonestyTests
{
    /// <summary>The builtin's decimal-place computation cap, as the summary and the wire state it.</summary>
    private const int Cap = 1000;

    private static async Task<JsonNode> RunAsync(string script, params string[] extra)
    {
        var (exitCode, stdout, stderr) = await TestSupport.RunScriptAsync(script, extra);
        Assert.True(exitCode == 0, $"'{script}' exited {exitCode}. stderr: {stderr.Trim()}");
        return TestSupport.ParseExactlyOneJsonDocument(stdout, $"stdout of '{script}'");
    }

    private static JsonNode StructuredResult(JsonNode envelope) => envelope["result"]!["structured"]!;

    private static JsonNode ResultVariable(JsonNode envelope) =>
        envelope["variables"]!.AsArray().Single(v => v!["name"]!.GetValue<string>() == "_")!;

    private static int Decimals(string rendering)
    {
        int dot = rendering.IndexOf('.');
        return dot < 0 ? 0 : rendering.Length - dot - 1;
    }

    /// <summary>The finding's own repro, and the contract the fix states: the value still carries the
    /// documented 1000 places, and now says that the caller's 5000 were clamped to them.</summary>
    [Fact]
    public async Task ARequestBeyondTheCap_IsMarkedAsClamped()
    {
        JsonNode envelope = await RunAsync("setprecision(5000); evalf(sqrt(2), 5000)", "--omit-functions");
        JsonNode structured = StructuredResult(envelope);

        // the digits are the ones the cap allows, and they are the real digits of sqrt(2)
        string value = structured["value"]!.GetValue<string>();
        Assert.Equal(Cap, Decimals(value));
        Assert.StartsWith("1.4142135623730950488016887242096980785696718753769", value, StringComparison.Ordinal);
        Assert.False(structured["exact"]!.GetValue<bool>());

        // ... and the clamp is on the caller's side of the wire, in the protocol's own vocabulary
        Assert.True(structured["truncated"]?.GetValue<bool>() == true,
            "the 1000-place clamp crossed with no truncation flag");
        Assert.Equal("digit-cap", structured["truncationReason"]!.GetValue<string>());
        Assert.Equal(Cap, structured["budget"]!.GetValue<int>());

        // one value, one projection: the variable holding it reports the same fact
        JsonNode variable = ResultVariable(envelope)["structured"]!;
        Assert.Equal(value, variable["value"]!.GetValue<string>());
        Assert.True(variable["truncated"]?.GetValue<bool>() == true);
        Assert.Equal("digit-cap", variable["truncationReason"]!.GetValue<string>());
        Assert.Equal(Cap, variable["budget"]!.GetValue<int>());

        // the display is a rendering of the SAME value: 1000 places, unchanged by the marker
        Assert.Equal(Cap, Decimals(envelope["result"]!["display"]!.GetValue<string>()));
    }

    /// <summary>The audit's control, in the same binary and the same envelope shape: a plain
    /// expression under the raised precision carries the 5000 places it asked for (so the clamp the
    /// test above reports is <c>evalf</c>'s own, not the engine's) and reports no truncation.</summary>
    [Fact]
    public async Task APlainExpressionUnderTheSamePrecision_IsNotClamped()
    {
        JsonNode structured = StructuredResult(await RunAsync("setprecision(5000); sqrt(2)", "--omit-functions"));

        Assert.Equal(5000, Decimals(structured["value"]!.GetValue<string>()));
        Assert.Null(structured["truncated"]);
        Assert.Null(structured["truncationReason"]);
        Assert.Null(structured["budget"]);
    }

    /// <summary>The boundary the marker must NOT swallow: a request AT the cap is honoured in full,
    /// so nothing was clamped and nothing may be claimed. (<c>evalf(pi, 1000)</c> is the case
    /// <c>StructuredRealPrecisionTests</c> already pins at 1000 places.)</summary>
    [Fact]
    public async Task ARequestAtTheCap_IsNotMarked()
    {
        JsonNode structured = StructuredResult(await RunAsync("evalf(pi, 1000)", "--omit-functions"));

        Assert.Equal(Cap, Decimals(structured["value"]!.GetValue<string>()));
        Assert.Null(structured["truncated"]);
        Assert.Null(structured["truncationReason"]);
        Assert.Null(structured["budget"]);
    }

    /// <summary>The marker means "digits were DROPPED", not "the request was above the cap": an
    /// exact answer to an over-cap request (1/2 is 0.5, whatever count is asked for) is complete and
    /// crosses unmarked.</summary>
    [Fact]
    public async Task AnExactAnswerToAnOverCapRequest_IsNotMarked()
    {
        JsonNode structured = StructuredResult(await RunAsync("evalf(1/2, 5000)", "--omit-functions"));

        Assert.Equal("0.5", structured["value"]!.GetValue<string>());
        Assert.True(structured["exact"]!.GetValue<bool>());
        Assert.Null(structured["truncated"]);
        Assert.Null(structured["truncationReason"]);
        Assert.Null(structured["budget"]);
    }

    /// <summary>The smallest over-cap request there is — one place above the cap — is marked too:
    /// the cap is what decided the answer, so the value says so.</summary>
    [Fact]
    public async Task TheSmallestOverCapRequest_IsMarked()
    {
        JsonNode structured = StructuredResult(await RunAsync("evalf(1/3, 1001)", "--omit-functions"));

        Assert.Equal("0." + new string('3', Cap), structured["value"]!.GetValue<string>());
        Assert.True(structured["truncated"]?.GetValue<bool>() == true,
            "a 1001-place request crossed as if the count had been honoured: " +
            structured.ToJsonString());
        Assert.Equal("digit-cap", structured["truncationReason"]!.GetValue<string>());
        Assert.Equal(Cap, structured["budget"]!.GetValue<int>());
    }
}
