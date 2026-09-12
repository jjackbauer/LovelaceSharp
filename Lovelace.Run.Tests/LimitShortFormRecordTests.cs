using System.Text.Json.Nodes;
using Xunit;

namespace Lovelace.Run.Tests;

/// <summary>
/// Audit D, finding F2 (cycle 6, P1): the SHORT limit builtins answered a refusal as a bare Text
/// value under <c>ok: true</c> — <c>limit(1/x, x, 0)</c> crossed as
/// <c>{"kind":"Text","value":"does not exist (left: -inf, right: +inf)"}</c>, with no code, no
/// category and nothing an agent could branch on without parsing English — while <c>limit_full</c>
/// on the SAME input published a LimitResult record whose <c>left</c>/<c>right</c> fields already
/// carried those two values.
/// <para>
/// This pins the fix through the PUBLISHED envelope: every shape <c>limit</c>, <c>limit_left</c>
/// and <c>limit_right</c> can produce is the SAME LimitResult record <c>limit_full</c> publishes —
/// the decision cycle 5 took for <c>solve</c>, taken for the same reason (see
/// <c>BuiltinSurfaceContractTests.Solve_ShortForm_PublishesTheSameSolveResultRecord_AsTheFullForm</c>):
/// a status must never have to be parsed out of prose.
/// </para>
/// </summary>
public class LimitShortFormRecordTests
{
    private static async Task<(int ExitCode, JsonNode Envelope)> RunAsync(string script)
    {
        var (exitCode, stdout, _) = await TestSupport.RunScriptAsync(
            script, "--omit-functions", "--omit-variables");
        return (exitCode, TestSupport.ParseExactlyOneJsonDocument(stdout, $"stdout of '{script}'"));
    }

    /// <summary>The short form's record, after asserting the result IS a LimitResult record: a
    /// regression back to a Text answer names itself here instead of dying on a missing "fields"
    /// array.</summary>
    private static JsonObject Record(JsonNode envelope)
    {
        JsonNode structured = envelope["result"]!["structured"]!;
        Assert.Equal("Record", structured["kind"]!.GetValue<string>());
        Assert.Equal("LimitResult", structured["type"]!.GetValue<string>());
        return TestSupport.FieldsByName(structured);
    }

    /// <summary>An enum-valued field's member, after asserting the field really is an Enum: the
    /// declared type name is the contract (a consumer must not guess the vocabulary from spelling).
    /// </summary>
    private static string Enum(JsonNode field, string typeName)
    {
        Assert.Equal("Enum", field["kind"]!.GetValue<string>());
        Assert.Equal(typeName, field["type"]!.GetValue<string>());
        return field["value"]!.GetValue<string>();
    }

    private static string Pretty(JsonNode field) => field["kind"]!.GetValue<string>() switch
    {
        "Null" => "(null)",
        "Symbolic" => field["pretty"]!.GetValue<string>(),
        _ => throw new Xunit.Sdk.XunitException(
            $"field is {field["kind"]}, not a Symbolic value or the absent-field Null"),
    };

    private static string[] Pretties(JsonNode array) =>
        array["elements"]!.AsArray().Select(e => Pretty(e!)).ToArray();

    // ------------------------------------------------------------------
    // 1. every shape is a record, and the record says the status
    // ------------------------------------------------------------------

    /// <summary>
    /// Every shape the three short forms can answer: proven non-existence, a one-sided divergence,
    /// an undetermined kernel result, and a determined value. Each one is a LimitResult record —
    /// never a Text value — and <c>status</c>/<c>exists</c> are the record's own authoritative
    /// claim, so "did it answer?" is answerable from structure alone.
    /// </summary>
    [Theory]
    [InlineData("x = symbol(\"x\"); limit(1/x, x, 0)", "DoesNotExist", "Boolean", "false")]
    [InlineData("x = symbol(\"x\"); limit_left(1/x, x, 0)", "MinusInfinity", "Boolean", "true")]
    [InlineData("x = symbol(\"x\"); limit_right(1/x, x, 0)", "PlusInfinity", "Boolean", "true")]
    [InlineData("x = symbol(\"x\"); limit(sin(x), x, inf)", "Unevaluated", "Null", "")]
    [InlineData("x = symbol(\"x\"); limit_left(sin(x), x, inf)", "Unevaluated", "Null", "")]
    [InlineData("x = symbol(\"x\"); limit_right(sin(x), x, inf)", "Unevaluated", "Null", "")]
    [InlineData("x = symbol(\"x\"); limit(sin(x)/x, x, 0)", "Value", "Boolean", "true")]
    [InlineData("x = symbol(\"x\"); limit_left(sin(x)/x, x, 0)", "Value", "Boolean", "true")]
    [InlineData("x = symbol(\"x\"); limit_right(sin(x)/x, x, 0)", "Value", "Boolean", "true")]
    public async Task ShortForm_PublishesALimitResultRecord_NeverProse(
        string script, string status, string existsKind, string existsValue)
    {
        var (exit, envelope) = await RunAsync(script);
        Assert.Equal(0, exit);
        Assert.True(envelope["ok"]!.GetValue<bool>(), $"'{script}' must still succeed");

        JsonObject fields = Record(envelope);
        Assert.Equal(status, Enum(fields["status"]!, "LimitStatus"));
        Assert.Equal(existsKind, fields["exists"]!["kind"]!.GetValue<string>());
        if (existsKind == "Boolean")
            Assert.Equal(existsValue, fields["exists"]!["value"]!.GetValue<string>());
    }

    // ------------------------------------------------------------------
    // 2. the short form is the SAME record the _full form publishes
    // ------------------------------------------------------------------

    /// <summary>
    /// Not merely "also a record": for the two-sided query the short form is the SAME record, field
    /// for field, that <c>limit_full</c> publishes — the guarantee <c>solve()</c> already gives, and
    /// the one that makes "read the status off the structure" true for every consumer.
    /// </summary>
    [Theory]
    [InlineData("limit(1/x, x, 0)", "limit_full(1/x, x, 0)")]
    [InlineData("limit(sin(x)/x, x, 0)", "limit_full(sin(x)/x, x, 0)")]
    [InlineData("limit(sin(x), x, inf)", "limit_full(sin(x), x, inf)")]
    public async Task TwoSidedShortForm_IsTheSameRecordAsTheFullForm(string shortCall, string fullCall)
    {
        var (shortExit, shortEnvelope) = await RunAsync($"x = symbol(\"x\"); {shortCall}");
        var (fullExit, fullEnvelope) = await RunAsync($"x = symbol(\"x\"); {fullCall}");
        Assert.Equal(0, shortExit);
        Assert.Equal(0, fullExit);

        TestSupport.AssertSameJson(
            TestSupport.Normalise(fullEnvelope["result"]!["structured"]),
            TestSupport.Normalise(shortEnvelope["result"]!["structured"]),
            $"{shortCall} vs {fullCall}");
    }

    // ------------------------------------------------------------------
    // 3. nothing the prose carried is lost
    // ------------------------------------------------------------------

    /// <summary>
    /// The old sentence "does not exist (left: -inf, right: +inf)" carried two values; they are the
    /// record's own <c>left</c>/<c>right</c> fields, each with the side constraint it holds under,
    /// and <c>value</c> is honestly Null (the two-sided limit does not exist). Nothing has to be
    /// recovered by parsing the display string.
    /// </summary>
    [Fact]
    public async Task NoLimit_KeepsEveryValueTheProseSpelledOut()
    {
        var (exit, envelope) = await RunAsync("x = symbol(\"x\"); limit(1/x, x, 0)");
        Assert.Equal(0, exit);
        JsonObject fields = Record(envelope);

        Assert.Equal("-inf", Pretty(fields["left"]!));
        Assert.Equal("inf", Pretty(fields["right"]!));
        Assert.Equal(new[] { "x < 0" }, Pretties(fields["left_conditions"]!));
        Assert.Equal(new[] { "x > 0" }, Pretties(fields["right_conditions"]!));
        Assert.Equal("Null", fields["value"]!["kind"]!.GetValue<string>());
        Assert.Equal("Exact", Enum(fields["exactness"]!, "SolutionExactness"));
        Assert.Empty(fields["diagnostics"]!["elements"]!.AsArray());
    }

    /// <summary>A one-sided query publishes its answer as structure: the one-sided value is the
    /// record's <c>value</c>, with the status and the tri-state <c>exists</c> beside it — where the
    /// old answer was a bare Symbolic with no status at all.</summary>
    [Fact]
    public async Task OneSidedShortForms_PublishTheOneSidedAnswerAsStructure()
    {
        var (leftExit, leftEnvelope) = await RunAsync("x = symbol(\"x\"); limit_left(1/x, x, 0)");
        Assert.Equal(0, leftExit);
        JsonObject left = Record(leftEnvelope);
        Assert.Equal("MinusInfinity", Enum(left["status"]!, "LimitStatus"));
        Assert.Equal("-inf", Pretty(left["value"]!));
        Assert.Equal("Exact", Enum(left["exactness"]!, "SolutionExactness"));

        var (rightExit, rightEnvelope) = await RunAsync("x = symbol(\"x\"); limit_right(1/x, x, 0)");
        Assert.Equal(0, rightExit);
        JsonObject right = Record(rightEnvelope);
        Assert.Equal("PlusInfinity", Enum(right["status"]!, "LimitStatus"));
        Assert.Equal("inf", Pretty(right["value"]!));
        Assert.Equal("Exact", Enum(right["exactness"]!, "SolutionExactness"));
    }

    /// <summary>The successful case still answers the same value, now with its exactness and domain
    /// beside it in structure: 1 for sin(x)/x at 0, exact and rational.</summary>
    [Fact]
    public async Task Success_PublishesValueAndExactnessAsStructure()
    {
        var (exit, envelope) = await RunAsync("x = symbol(\"x\"); limit(sin(x)/x, x, 0)");
        Assert.Equal(0, exit);
        JsonObject fields = Record(envelope);

        Assert.Equal("Value", Enum(fields["status"]!, "LimitStatus"));
        Assert.Equal("1", Pretty(fields["value"]!));
        Assert.Equal("rational", fields["value"]!["domain"]!.GetValue<string>());
        Assert.True(fields["value"]!["exact"]!.GetValue<bool>());
        Assert.Equal("Exact", Enum(fields["exactness"]!, "SolutionExactness"));
        Assert.Empty(fields["diagnostics"]!["elements"]!.AsArray());
        Assert.Empty(fields["left_conditions"]!["elements"]!.AsArray());
        Assert.Empty(fields["right_conditions"]!["elements"]!.AsArray());
    }

    // ------------------------------------------------------------------
    // 4. a refusal carries the code and category the statement advertises
    // ------------------------------------------------------------------

    /// <summary>
    /// A refusal is a refusal in STRUCTURE, not in spelling. The kernel's reason rides in the
    /// record's <c>diagnostics</c> under the code and category <c>capabilities()</c> already
    /// advertises for this class (<c>limit.unevaluated</c> / UnsupportedOperation,
    /// operation_class <c>limit.unevaluated-in-record-diagnostics</c>), which is what the old
    /// "unevaluated: …" Text prefix could only say in English.
    /// </summary>
    [Theory]
    [InlineData("limit(sin(x), x, inf)", "coefficient does not evaluate at the point")]
    [InlineData("limit_left(sin(x), x, inf)", "coefficient does not evaluate at the point")]
    [InlineData("limit_right(sin(x), x, inf)", "coefficient does not evaluate at the point")]
    [InlineData("limit(a/x, x, 0)", "leading coefficient is symbolic")]
    [InlineData("limit_left(a/x, x, 0)", "leading coefficient is symbolic")]
    [InlineData("limit_right(a/x, x, 0)", "leading coefficient is symbolic")]
    public async Task Refusal_CarriesTheCodeAndCategoryTheStatementAdvertises(string call, string message)
    {
        var (exit, envelope) = await RunAsync($"a = symbol(\"a\"); x = symbol(\"x\"); {call}");
        Assert.Equal(0, exit);
        Assert.True(envelope["ok"]!.GetValue<bool>(), $"'{call}' is a refusal INSIDE a successful call");

        JsonObject fields = Record(envelope);
        Assert.Equal("Unevaluated", Enum(fields["status"]!, "LimitStatus"));
        Assert.Equal("Null", fields["exists"]!["kind"]!.GetValue<string>());

        JsonArray diagnostics = fields["diagnostics"]!["elements"]!.AsArray();
        Assert.Single(diagnostics);
        Assert.Equal("Diagnostic", diagnostics[0]!["type"]!.GetValue<string>());
        JsonObject diagnostic = TestSupport.FieldsByName(diagnostics[0]!);
        Assert.Equal("limit.unevaluated", diagnostic["code"]!["value"]!.GetValue<string>());
        Assert.Equal("UnsupportedOperation", Enum(diagnostic["category"]!, "ErrorCategory"));
        Assert.Equal(message, diagnostic["message"]!["value"]!.GetValue<string>());
        Assert.Equal("true", diagnostic["recoverable"]!["value"]!.GetValue<string>());
    }
}
