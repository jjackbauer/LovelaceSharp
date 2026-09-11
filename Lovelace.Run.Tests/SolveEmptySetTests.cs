using System.Text.Json.Nodes;
using Xunit;

namespace Lovelace.Run.Tests;

/// <summary>
/// docs/symbolics/dsh-protocol.md:166-170 — "NoSolutions means the solution set over the requested
/// domain is provably empty ... A provably empty set is a complete answer, so it reports
/// complete: true / completeness: Complete."
/// <para>
/// The equation <c>0 == 1</c> is a RELATION the caller wrote, but its two sides are concrete, so the
/// interpreter used to fold it to the Boolean false before the symbolic kernel saw it — and the
/// kernel refused it as "argument 1 must be a symbolic expression; got Boolean". A relation written
/// in the arguments of a symbolic builtin crosses as the relation it is, so the solver can prove
/// emptiness and answer NoSolutions.
/// </para>
/// </summary>
public class SolveEmptySetTests
{
    private static async Task<(int ExitCode, JsonNode Envelope)> RunAsync(string script)
    {
        var stdout = new StringWriter();
        var stderr = new StringWriter();
        int exitCode = await Runner.RunAsync(
            new[] { "--eval", script, "--omit-functions", "--omit-variables" }, stdout, stderr);
        return (exitCode, TestSupport.ParseExactlyOneJsonDocument(stdout.ToString(), $"stdout of '{script}'"));
    }

    private static JsonObject Fields(JsonNode structured) => TestSupport.FieldsByName(structured);

    [Fact]
    public async Task Solve_OfAProvablyFalseEquation_AnswersTheEmptySet_NotADomainError()
    {
        var (exitCode, envelope) = await RunAsync("x = symbol(\"x\"); solve(0 == 1, x)");

        Assert.True(exitCode == 0, $"solve(0 == 1, x) must answer, not fail: {envelope.ToJsonString()}");
        Assert.True(envelope["ok"]!.GetValue<bool>());
        Assert.Equal("Text", envelope["result"]!["kind"]!.GetValue<string>());
        Assert.Equal("the equation reduces to a nonzero constant.",
            envelope["result"]!["structured"]!["value"]!.GetValue<string>());
        // the old failure mode: the folded Boolean reaching the kernel as an argument type error
        Assert.DoesNotContain("Boolean", envelope["result"]!["structured"]!["value"]!.GetValue<string>());
    }

    [Fact]
    public async Task SolveFull_OfAProvablyFalseEquation_ReportsNoSolutionsComplete()
    {
        var (exitCode, envelope) = await RunAsync("x = symbol(\"x\"); solve_full(2 == 3, x)");

        Assert.True(exitCode == 0, $"solve_full(2 == 3, x) must answer, not fail: {envelope.ToJsonString()}");
        JsonNode structured = envelope["result"]!["structured"]!;
        Assert.Equal("Record", structured["kind"]!.GetValue<string>());
        Assert.Equal("SolveResult", structured["type"]!.GetValue<string>());

        JsonObject fields = Fields(structured);
        Assert.Equal("Enum", fields["status"]!["kind"]!.GetValue<string>());
        Assert.Equal("SolveStatus", fields["status"]!["type"]!.GetValue<string>());
        Assert.Equal("NoSolutions", fields["status"]!["value"]!.GetValue<string>());

        // complete is the derived convenience flag; completeness is the authoritative enum
        Assert.Equal("true", fields["complete"]!["value"]!.GetValue<string>());
        Assert.Equal("Completeness", fields["completeness"]!["type"]!.GetValue<string>());
        Assert.Equal("Complete", fields["completeness"]!["value"]!.GetValue<string>());

        Assert.Equal(0, fields["solutions"]!["shape"]!.AsArray()[0]!.GetValue<int>());

        // the emptiness carries its PROOF as a diagnostic of the no-solution class
        JsonArray diagnostics = fields["diagnostics"]!["elements"]!.AsArray();
        Assert.True(diagnostics.Count >= 1, "a NoSolutions answer reports why the set is empty");
        JsonObject diagnostic = TestSupport.FieldsByName(diagnostics[0]!);
        Assert.Equal("Enum", diagnostic["category"]!["kind"]!.GetValue<string>());
        Assert.Equal("ErrorCategory", diagnostic["category"]!["type"]!.GetValue<string>());
        Assert.Equal("NoSolution", diagnostic["category"]!["value"]!.GetValue<string>());
        Assert.Equal("solve.no-solutions", diagnostic["code"]!["value"]!.GetValue<string>());
    }

    /// <summary>The same relation, in a system: the emptiness is proved for the whole system.</summary>
    [Fact]
    public async Task SolveSystem_OfAProvablyFalseEquation_AnswersTheEmptySet()
    {
        var (exitCode, envelope) = await RunAsync(
            "x = symbol(\"x\"); y = symbol(\"y\"); solve_system([0 == 1], [x, y])");

        Assert.True(exitCode == 0, $"solve_system([0 == 1], [x, y]) must answer: {envelope.ToJsonString()}");
        JsonObject fields = Fields(envelope["result"]!["structured"]!);
        Assert.Equal("NoSolutions", fields["status"]!["value"]!.GetValue<string>());
        Assert.Equal("true", fields["complete"]!["value"]!.GetValue<string>());
        Assert.Equal("Complete", fields["completeness"]!["value"]!.GetValue<string>());
    }

    /// <summary>The narrowed rule, pinned: only SYMBOLIC consumers receive the relation. A builtin
    /// that declares a Boolean result still consumes the evaluated condition, so the language's
    /// branching semantics are untouched — and a core builtin still sees the folded Boolean.</summary>
    [Fact]
    public async Task TheFoldedBooleanIsUnchangedOutsideTheSymbolicChannel()
    {
        var (eqExit, eq) = await RunAsync("type(0 == 1)");
        Assert.Equal(0, eqExit);
        Assert.Equal("Boolean", eq["result"]!["structured"]!["value"]!.GetValue<string>());

        var (trueExit, isTrue) = await RunAsync("1 == 1");
        Assert.Equal(0, trueExit);
        Assert.Equal("Boolean", isTrue["result"]!["kind"]!.GetValue<string>());
        Assert.Equal("true", isTrue["result"]!["structured"]!["value"]!.GetValue<string>());

        var (notExit, not) = await RunAsync("not(0 == 1)");
        Assert.Equal(0, notExit);
        Assert.Equal("Boolean", not["result"]!["kind"]!.GetValue<string>());
        Assert.Equal("true", not["result"]!["structured"]!["value"]!.GetValue<string>());
    }
}
