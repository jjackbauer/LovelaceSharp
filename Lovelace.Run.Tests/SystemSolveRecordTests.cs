using System.Text.Json.Nodes;
using Xunit;

namespace Lovelace.Run.Tests;

/// <summary>
/// <c>solve_system</c> and <c>solve_system_full</c> are two call forms of ONE builtin family, and
/// the frozen contract declares ONE record for their result: <c>SystemSolveResult</c> — the same
/// shape <c>solve_system_full</c> publishes, including the <c>completeness</c> Enum the protocol
/// calls the authoritative field (<c>complete</c> is only its derived boolean, and the two are read
/// off ONE mapping so the two records cannot drift apart).
/// <para>
/// <c>solve_system</c> used to return the joined prose "x = 1, y = 1" — a Text value carrying
/// variables and values no consumer can read back — while the descriptor and the contract named the
/// record. The wire shape is pinned here, field by field and in order.
/// </para>
/// </summary>
public class SystemSolveRecordTests
{
    private static async Task<(int ExitCode, JsonNode Envelope)> RunAsync(string script)
    {
        var stdout = new StringWriter();
        var stderr = new StringWriter();
        int exitCode = await Runner.RunAsync(
            new[] { "--eval", script, "--omit-functions", "--omit-variables" }, stdout, stderr);
        return (exitCode, TestSupport.ParseExactlyOneJsonDocument(stdout.ToString(), $"stdout of '{script}'"));
    }

    private const string Declare = "x = symbol(\"x\"); y = symbol(\"y\"); ";

    /// <summary>The wire field order is the registry's field order, and every field is the declared
    /// KIND: an Enum of type SolveStatus, a Domain, the derived Boolean, an Enum of type
    /// Completeness, an Array of SystemSolution records, and a diagnostics Array — never Text.</summary>
    [Fact]
    public async Task SolveSystem_PublishesTheSystemSolveResultRecord_TheSchemaDeclares()
    {
        var (exitCode, envelope) = await RunAsync(
            Declare + "solve_system([x + y == 2, x - y == 0], [x, y])");

        Assert.True(exitCode == 0, $"solve_system exited {exitCode}: {envelope.ToJsonString()}");
        JsonNode structured = envelope["result"]!["structured"]!;
        Assert.Equal("Record", structured["kind"]!.GetValue<string>());
        Assert.Equal("SystemSolveResult", structured["type"]!.GetValue<string>());

        var names = structured["fields"]!.AsArray()
            .Select(f => f!["name"]!.GetValue<string>()).ToArray();
        Assert.Equal(
            new[] { "status", "domain", "complete", "completeness", "solutions", "diagnostics" },
            names);

        JsonObject fields = TestSupport.FieldsByName(structured);

        Assert.Equal("Enum", fields["status"]!["kind"]!.GetValue<string>());
        Assert.Equal("SolveStatus", fields["status"]!["type"]!.GetValue<string>());
        Assert.Equal("Solved", fields["status"]!["value"]!.GetValue<string>());

        Assert.Equal("Domain", fields["domain"]!["kind"]!.GetValue<string>());

        Assert.Equal("Boolean", fields["complete"]!["kind"]!.GetValue<string>());
        Assert.Equal("true", fields["complete"]!["value"]!.GetValue<string>());

        Assert.Equal("Enum", fields["completeness"]!["kind"]!.GetValue<string>());
        Assert.Equal("Completeness", fields["completeness"]!["type"]!.GetValue<string>());
        Assert.Equal("Complete", fields["completeness"]!["value"]!.GetValue<string>());

        JsonNode solutions = fields["solutions"]!;
        Assert.Equal("Array", solutions["kind"]!.GetValue<string>());
        var elements = solutions["elements"]!.AsArray();
        Assert.Single(elements);
        Assert.Equal("Record", elements[0]!["kind"]!.GetValue<string>());
        Assert.Equal("SystemSolution", elements[0]!["type"]!.GetValue<string>());
        JsonObject solutionFields = TestSupport.FieldsByName(elements[0]!);
        Assert.Equal("Array", solutionFields["bindings"]!["kind"]!.GetValue<string>());
        Assert.Equal(2, solutionFields["bindings"]!["elements"]!.AsArray().Count);
        Assert.Equal("Array", solutionFields["conditions"]!["kind"]!.GetValue<string>());
        Assert.Equal("Enum", solutionFields["exactness"]!["kind"]!.GetValue<string>());

        // an Array with zero elements — never Null, never the empty string, never prose
        JsonNode diagnostics = fields["diagnostics"]!;
        Assert.Equal("Array", diagnostics["kind"]!.GetValue<string>());
        Assert.Null(diagnostics["value"]);
        Assert.Empty(diagnostics["elements"]!.AsArray());
    }

    /// <summary>The pairing the protocol table declares, observed on the wire for <c>solve_system</c>:
    /// a solved system is complete, a PROVABLY EMPTY one is a complete answer too, and a system the
    /// polynomial fragment cannot represent is not — and the two fields never disagree.</summary>
    [Theory]
    [InlineData("solve_system([x + y == 2, x - y == 0], [x, y])", "Solved", "true", "Complete")]
    [InlineData("solve_system([x + y == 1, x + y == 2], [x, y])", "NoSolutions", "true", "Complete")]
    [InlineData("solve_system([sin(x) == 0], [x])", "Unevaluated", "false", "Unknown")]
    public async Task SolveSystem_StatusCompleteAndCompleteness_AreTheDocumentedRow(
        string call, string status, string complete, string completeness)
    {
        var (exitCode, envelope) = await RunAsync(Declare + call);

        Assert.True(exitCode == 0, $"'{call}' exited {exitCode}: {envelope.ToJsonString()}");
        JsonObject fields = TestSupport.FieldsByName(envelope["result"]!["structured"]!);

        Assert.Equal(status, fields["status"]!["value"]!.GetValue<string>());
        Assert.Equal(complete, fields["complete"]!["value"]!.GetValue<string>());
        Assert.Equal(completeness, fields["completeness"]!["value"]!.GetValue<string>());
        // the completeness field is never absent and never prose: it is the enum the contract names
        Assert.Equal("Completeness", fields["completeness"]!["type"]!.GetValue<string>());
    }

    /// <summary>The two call forms publish the SAME record for the same system: the prose form is
    /// gone, and the non-full builtin is not a second, narrower contract.</summary>
    [Fact]
    public async Task TheTwoSystemCallForms_PublishTheSameRecordShape()
    {
        const string system = "[x + y == 2, x - y == 0], [x, y]";
        var (firstExit, first) = await RunAsync(Declare + "solve_system(" + system + ")");
        var (secondExit, second) = await RunAsync(Declare + "solve_system_full(" + system + ")");

        Assert.True(firstExit == 0 && secondExit == 0, $"exits: {firstExit}/{secondExit}");
        JsonNode a = TestSupport.Normalise(first["result"]!["structured"]!);
        JsonNode b = TestSupport.Normalise(second["result"]!["structured"]!);
        Assert.Equal(b.ToJsonString(), a.ToJsonString());
    }
}
