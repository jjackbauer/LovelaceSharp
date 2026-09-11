using System.Text.Json.Nodes;

namespace Lovelace.Run.Tests;

/// <summary>
/// <c>inspect(v).exact</c> answers the SAME question the value's own wire form answers with its
/// <c>exact</c> flag, for every kind where exactness is a property: a Real carries the provenance
/// its arithmetic recorded, and a Complex is exact when both parts are (A1-N-09).
///
/// <para>It used to be Null for both — <c>inspect(1/3)</c> reported <c>"exact":{"kind":"Null"}</c>
/// for a value whose Real form says <c>"exact":true</c>, so "is this number exact?" could not be
/// asked through the inspection record at all, and the exact <c>1/3</c> was indistinguishable from
/// the truncated <c>sqrt(2)</c>.</para>
/// </summary>
public class InspectionExactnessTests
{
    private static async Task<JsonObject> InspectAsync(string valueSource)
    {
        var (exitCode, stdout, stderr) = await TestSupport.RunScriptAsync(
            $"v = {valueSource}; i = inspect(v)", "--omit-functions");
        Assert.True(exitCode == 0, $"inspect({valueSource}) exited {exitCode}. stderr: {stderr.Trim()}");

        JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(stdout, $"inspect({valueSource}) envelope");
        return TestSupport.FieldsByName(envelope["result"]!["structured"]!);
    }

    [Theory]
    [InlineData("1", true)]                 // Natural: unchanged by this round
    [InlineData("1/3", true)]               // an exact terminating quotient
    [InlineData("1/(10^19)", true)]         // exact past the 18-place shape
    [InlineData("sqrt(2)", false)]          // truncated irrational
    [InlineData("(-4)^(1/2)", true)]        // Symbolic exactness: unchanged by this round
    [InlineData("dft([1, 2, 3])[0]", true)]  // Complex, both parts exact
    [InlineData("dft([1, 2, 3])[1]", false)] // Complex, one truncated part
    public async Task Inspect_AnswersExactnessAsABoolean(string valueSource, bool expected)
    {
        JsonObject fields = await InspectAsync(valueSource);

        JsonNode exact = fields["exact"]!;
        Assert.True(exact["kind"]!.GetValue<string>() == "Boolean",
            $"inspect({valueSource}).exact is {exact.ToJsonString()}, not a Boolean — the question " +
            "cannot be answered from the record");
        // a Boolean VALUE crosses with its member name as text, exactly like every other
        // Boolean on the wire ({"kind":"Boolean","value":"true"}), so the assertion is on the name
        Assert.Equal(expected ? "true" : "false", exact["value"]!.GetValue<string>());
    }

    /// <summary>The agreement property A1-N-09 is about: the record and the value's own form are
    /// two surfaces of ONE answer, for every kind that carries the flag.</summary>
    [Theory]
    [InlineData("1/3")]
    [InlineData("1/(10^19)")]
    [InlineData("sqrt(2)")]
    [InlineData("1/1009")]
    [InlineData("dft([1, 2, 3])[0]")]
    [InlineData("dft([1, 2, 3])[1]")]
    public async Task InspectExact_AgreesWithTheValuesOwnExactFlag(string valueSource)
    {
        var (exitCode, stdout, _) = await TestSupport.RunScriptAsync(
            $"v = {valueSource}; i = inspect(v)", "--omit-functions");
        Assert.Equal(0, exitCode);

        JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(stdout, "agreement envelope");
        JsonNode value = envelope["variables"]!.AsArray()
            .First(v => v!["name"]!.GetValue<string>() == "v")!["structured"]!;
        JsonNode record = TestSupport.FieldsByName(envelope["result"]!["structured"]!)["exact"]!;

        Assert.True(value["exact"] is not null, $"the value form of {valueSource} carries no exact flag");
        Assert.Equal(value["exact"]!.GetValue<bool>(), record["value"]!.GetValue<string>() == "true");
    }

    /// <summary>Precision changes the answer through the SAME rule: an 18-place truncation of
    /// 1/1009 is not exact, and its inspection record must say so too (the old shape test that
    /// called it exact is the reason the two surfaces could disagree).</summary>
    [Fact]
    public async Task TruncatedQuotientAtTheBudget_IsNotExactInEitherSurface()
    {
        var (exitCode, stdout, _) = await TestSupport.RunScriptAsync(
            "setprecision(18); v = 1/1009; i = inspect(v)", "--omit-functions");
        Assert.Equal(0, exitCode);

        JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(stdout, "truncation envelope");
        JsonNode value = envelope["variables"]!.AsArray()
            .First(v => v!["name"]!.GetValue<string>() == "v")!["structured"]!;
        JsonNode record = TestSupport.FieldsByName(envelope["result"]!["structured"]!)["exact"]!;

        Assert.False(value["exact"]!.GetValue<bool>());
        Assert.Equal("Boolean", record["kind"]!.GetValue<string>());
        Assert.Equal("false", record["value"]!.GetValue<string>());
    }

    /// <summary>Text, booleans, arrays and records are still Null: exactness is a property of the
    /// numeric/symbolic kinds, and an absent answer is <c>{"kind":"Null"}</c>, never an empty
    /// string or a fabricated false.</summary>
    [Theory]
    [InlineData("\"hi\"")]
    [InlineData("is_even(4)")]
    [InlineData("[1, 2]")]
    public async Task Inspect_KeepsNullWhereExactnessDoesNotApply(string valueSource)
    {
        JsonObject fields = await InspectAsync(valueSource);
        Assert.Equal("Null", fields["exact"]!["kind"]!.GetValue<string>());
    }
}
