using System.Globalization;
using System.Text.Json.Nodes;
using Xunit;

namespace Lovelace.Run.Tests;

/// <summary>
/// docs/symbolics/dsh-protocol.md:21-23 — "Durations are structural: <c>elapsedTime</c> is
/// <c>{value, unit}</c> next to the human <c>elapsed</c> string, and <c>timings</c> carries one entry
/// per top-level statement, so an agent never parses a unit suffix." The invariant is not scoped to
/// success: an ERROR envelope carries the same two structural durations, and the human string and
/// the pair come from ONE unit selector (<see cref="Lovelace.Suite.Timing.Scale"/>) so they can
/// never disagree.
/// </summary>
public class EnvelopeDurationTests
{
    private static readonly string[] Units = { "ns", "µs", "ms", "s", "min", "h" };

    private static async Task<(int ExitCode, JsonNode Envelope, string Raw)> RunAsync(params string[] arguments)
    {
        var stdout = new StringWriter();
        var stderr = new StringWriter();
        int exitCode = await Runner.RunAsync(arguments, stdout, stderr);
        string raw = stdout.ToString();
        return (exitCode, TestSupport.ParseExactlyOneJsonDocument(raw, "the envelope"), raw);
    }

    private static Task<(int ExitCode, JsonNode Envelope, string Raw)> RunScriptAsync(string script) =>
        RunAsync("--eval", script, "--omit-functions", "--omit-variables");

    /// <summary>The structural duration and the human string must agree, from one unit selector.</summary>
    private static void AssertSameScale(JsonNode envelope, string what)
    {
        string human = envelope["elapsed"]!.GetValue<string>();
        JsonNode? structured = envelope["elapsedTime"];
        Assert.True(structured is not null, $"{what}: the error envelope carries no elapsedTime");

        string[] parts = human.Split(' ');
        Assert.Equal(2, parts.Length);
        Assert.Contains(parts[1], Units);
        Assert.Equal(parts[1], structured!["unit"]!.GetValue<string>());
        Assert.Equal(double.Parse(parts[0], CultureInfo.InvariantCulture),
            structured!["value"]!.GetValue<double>(), 2);
    }

    private static void AssertTimingShape(JsonNode timing, string what)
    {
        Assert.True(timing["position"] is not null, $"{what}: a timing carries its source position");
        Assert.True(timing["elapsed"] is not null, $"{what}: a timing carries a structural duration");
        Assert.Contains(timing["elapsed"]!["unit"]!.GetValue<string>(), Units);
        Assert.True(timing["resultKind"] is not null, $"{what}: a timing carries the result kind");
        Assert.True(timing["hasOutput"] is not null, $"{what}: a timing says whether the statement printed");
    }

    /// <summary>The acceptance probe: a two-statement script whose SECOND statement fails.</summary>
    [Fact]
    public async Task MultiStatementErrorEnvelope_CarriesBothStructuralDurations()
    {
        var (exitCode, envelope, _) = await RunScriptAsync(
            "x = symbol(\"x\"); solve(x^2 + 1 == 0, x, integer)");

        Assert.Equal(1, exitCode);
        Assert.False(envelope["ok"]!.GetValue<bool>());
        AssertSameScale(envelope, "multi-statement error");

        JsonArray timings = envelope["timings"]!.AsArray();
        Assert.Equal(2, timings.Count);
        Assert.Equal(0, timings[0]!["position"]!.GetValue<int>());
        Assert.Equal("Symbolic", timings[0]!["resultKind"]!.GetValue<string>());
        Assert.Equal(17, timings[1]!["position"]!.GetValue<int>());
        // the statement that FAILED is still a statement that ran
        Assert.Equal("Void", timings[1]!["resultKind"]!.GetValue<string>());
        foreach (JsonNode? timing in timings)
            AssertTimingShape(timing!, "multi-statement error");
    }

    /// <summary>A ONE-statement script that fails reports exactly one timed statement.</summary>
    [Fact]
    public async Task SingleStatementErrorEnvelope_CarriesOneTiming()
    {
        var (exitCode, envelope, _) = await RunScriptAsync(
            "solve(symbol(\"x\")^2 + 1 == 0, symbol(\"x\"), integer)");

        Assert.Equal(1, exitCode);
        Assert.False(envelope["ok"]!.GetValue<bool>());
        AssertSameScale(envelope, "single-statement error");

        JsonArray timings = envelope["timings"]!.AsArray();
        Assert.Single(timings);
        Assert.Equal(0, timings[0]!["position"]!.GetValue<int>());
        AssertTimingShape(timings[0]!, "single-statement error");
    }

    /// <summary>When NO statement ran (a parse error) the timing array is EMPTY — present, never
    /// absent, and never null.</summary>
    [Fact]
    public async Task ErrorEnvelopeWithoutAnyStatement_CarriesAnEmptyTimingsArray()
    {
        var (exitCode, envelope, _) = await RunScriptAsync("1 +");

        Assert.Equal(1, exitCode);
        Assert.False(envelope["ok"]!.GetValue<bool>());
        AssertSameScale(envelope, "parse error");
        Assert.Empty(envelope["timings"]!.AsArray());
    }

    /// <summary>An error raised before any engine exists (the script file cannot be read) still
    /// carries the structural pair, so an agent never has to special-case the shape.</summary>
    [Fact]
    public async Task FileReadErrorEnvelope_CarriesTheSameStructuralDurations()
    {
        var (exitCode, envelope, _) = await RunAsync(
            "--file", Path.Combine(TestSupport.FixturesDirectory, "does-not-exist.ls"), "--omit-functions");

        Assert.Equal(1, exitCode);
        Assert.Equal("FileReadError", envelope["code"]!.GetValue<string>());
        AssertSameScale(envelope, "file read error");
        Assert.Empty(envelope["timings"]!.AsArray());
    }

    /// <summary>Success and error envelopes expose the SAME duration keys: a consumer switches on
    /// <c>ok</c>, not on which duration fields happen to be present.</summary>
    [Fact]
    public async Task SuccessAndErrorEnvelopes_CarryTheSameDurationKeys()
    {
        var (okExit, ok, _) = await RunScriptAsync("x = symbol(\"x\"); solve(x^2 - 4 == 0, x)");
        var (badExit, bad, _) = await RunScriptAsync("x = symbol(\"x\"); solve(x^2 + 1 == 0, x, integer)");

        Assert.Equal(0, okExit);
        Assert.Equal(1, badExit);

        foreach (var envelope in new[] { ok, bad })
        {
            Assert.True(envelope["elapsed"] is not null);
            Assert.True(envelope["elapsedTime"] is not null);
            Assert.True(envelope["timings"] is not null);
        }

        Assert.Equal(ok["timings"]!.AsArray().Count, bad["timings"]!.AsArray().Count);
    }
}
