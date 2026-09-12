using System.Text.Json.Nodes;
using Xunit;

namespace Lovelace.Run.Tests;

/// <summary>
/// The session assumption store is ONE store. The <c>assume*</c> descriptors promise their atom
/// holds "for the rest of the session" (Lovelace.Symbolics\SymbolicsPlugin.cs:129-139) and the
/// simplification path already reads it (<c>assume(x &gt; 0); simplify(sqrt(x^2))</c> answers
/// <c>x</c>). A <c>solve</c> in the SAME session therefore cannot publish <c>Solved</c> /
/// <c>complete: true</c> with a solution the store refutes, nor an empty <c>conditions</c> array
/// for a set it cannot decide: <c>SolveStatus.Solved</c> is the solver's own claim that the
/// represented set is the COMPLETE set over the domain the session works in
/// (Lovelace.Symbolics\Solvers\Solve.cs:11-24), and the per-solution / per-family
/// <c>conditions</c> array is the contract's place to carry a caveat
/// (docs/symbolics/dsh-protocol.md:117-131).
/// <para>Round-22 audit M-3 (docs/goal-cycle-6/round-22/audit-M-programs.md) and EVD-329. Every
/// script here is the audit's own text plus its controls.</para>
/// </summary>
public class SolveSessionAssumptionTests
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

    /// <summary>The solver record a script published, with the run's exit code and ok flag checked.</summary>
    private static async Task<JsonObject> SolveFields(string script, string recordType = "SolveResult")
    {
        var (exitCode, envelope) = await RunAsync(script);
        Assert.True(exitCode == 0, $"'{script}' must answer, not fail: {envelope.ToJsonString()}");
        Assert.True(envelope["ok"]!.GetValue<bool>(), $"'{script}' must answer ok: {envelope.ToJsonString()}");
        JsonNode structured = envelope["result"]!["structured"]!;
        Assert.Equal("Record", structured["kind"]!.GetValue<string>());
        Assert.Equal(recordType, structured["type"]!.GetValue<string>());
        return Fields(structured);
    }

    /// <summary>Every assignment a SystemSolveResult published, as "x=1, y=2" per assignment.</summary>
    private static string[] Assignments(JsonObject fields) =>
        ElementFields(fields, "solutions")
            .Select(s => string.Join(", ", s["bindings"]!["elements"]!.AsArray()
                .Select(b =>
                {
                    JsonObject binding = TestSupport.FieldsByName(b!);
                    return binding["name"]!["pretty"]!.GetValue<string>() + "=" +
                           binding["value"]!["pretty"]!.GetValue<string>();
                })))
            .ToArray();

    /// <summary>The pretty printing of every published solution value, in published order.</summary>
    private static string[] SolutionValues(JsonObject fields) =>
        ElementFields(fields, "solutions").Select(f => f["value"]!["pretty"]!.GetValue<string>()).ToArray();

    /// <summary>The pretty printing of every condition any solution carries.</summary>
    private static string[] SolutionConditions(JsonObject fields) =>
        ElementFields(fields, "solutions")
            .SelectMany(f => f["conditions"]!["elements"]!.AsArray())
            .Select(Pretty).ToArray();

    private static string[] FamilyConditions(JsonObject fields) =>
        ElementFields(fields, "families")
            .SelectMany(f => f["conditions"]!["elements"]!.AsArray())
            .Select(Pretty).ToArray();

    private static string Pretty(JsonNode? leaf) =>
        leaf!["pretty"] is { } p ? p.GetValue<string>() : leaf.ToJsonString();

    private static IEnumerable<JsonObject> ElementFields(JsonObject fields, string name) =>
        fields[name]!["elements"]!.AsArray().Select(e => TestSupport.FieldsByName(e!));

    // ------------------------------------------------------------------
    // The finding: the store is ignored by the solver
    // ------------------------------------------------------------------

    [Fact]
    public async Task Solve_DoesNotPublishARootTheSessionAssumptionExcludes()
    {
        // x == 3 has the single root 3 and the session's own store says x > 5: the solution set
        // over the session's domain is provably EMPTY, which is a COMPLETE answer (NoSolutions),
        // not a Solved set containing a value the session's assumptions exclude.
        JsonObject fields = await SolveFields("x = symbol(\"x\"); assume(x > 5); solve(x == 3, x)");

        Assert.Equal("Enum", fields["status"]!["kind"]!.GetValue<string>());
        Assert.Equal("SolveStatus", fields["status"]!["type"]!.GetValue<string>());
        Assert.Equal("NoSolutions", fields["status"]!["value"]!.GetValue<string>());
        Assert.Equal("true", fields["complete"]!["value"]!.GetValue<string>());
        Assert.Equal("Complete", fields["completeness"]!["value"]!.GetValue<string>());
        Assert.Equal(0, fields["solutions"]!["shape"]!.AsArray()[0]!.GetValue<int>());
    }

    [Fact]
    public async Task Solve_KeepsOnlyTheRootsTheAssumptionAdmits()
    {
        // assume_positive(x) stores the atom x > 0 (SymbolicsPlugin.cs:370), so of {-2, 2} only 2
        // survives. The answer stays COMPLETE: every other root is refuted by the store, so the
        // represented set is still the whole solution set over this session's domain.
        JsonObject positive = await SolveFields("x = symbol(\"x\"); assume_positive(x); solve(x^2 == 4, x)");
        Assert.Equal("Solved", positive["status"]!["value"]!.GetValue<string>());
        Assert.Equal("true", positive["complete"]!["value"]!.GetValue<string>());
        Assert.Equal(new[] { "2" }, SolutionValues(positive));
        Assert.Empty(SolutionConditions(positive));

        JsonObject negative = await SolveFields("x = symbol(\"x\"); assume(x < 0); solve(x^2 == 4, x)");
        Assert.Equal("Solved", negative["status"]!["value"]!.GetValue<string>());
        Assert.Equal(new[] { "-2" }, SolutionValues(negative));
    }

    [Fact]
    public async Task Solve_HonoursAnEqualityAssumption()
    {
        JsonObject fields = await SolveFields("x = symbol(\"x\"); assume(x == 2); solve(x^2 == 4, x)");
        Assert.Equal("Solved", fields["status"]!["value"]!.GetValue<string>());
        Assert.Equal(new[] { "2" }, SolutionValues(fields));
    }

    [Fact]
    public async Task Solve_HonoursADomainAssumption()
    {
        // assume_real(x) / assume_integer(x) restrict the session's domain: i and -i are not real
        // and 1/2 is not an integer, so neither equation has a solution in THIS session.
        JsonObject real = await SolveFields("x = symbol(\"x\"); assume_real(x); solve(x^2 + 1 == 0, x)");
        Assert.Equal("NoSolutions", real["status"]!["value"]!.GetValue<string>());
        Assert.Equal("true", real["complete"]!["value"]!.GetValue<string>());
        Assert.Equal(0, real["solutions"]!["shape"]!.AsArray()[0]!.GetValue<int>());

        JsonObject integer = await SolveFields("x = symbol(\"x\"); assume_integer(x); solve(x == 1/2, x)");
        Assert.Equal("NoSolutions", integer["status"]!["value"]!.GetValue<string>());
        Assert.Equal(0, integer["solutions"]!["shape"]!.AsArray()[0]!.GetValue<int>());
    }

    /// <summary>The seam itself: in ONE session, the store that makes simplify() answer <c>x</c>
    /// must also decide what solve() may publish.</summary>
    [Fact]
    public async Task OneSession_OneAssumptionStore_ForSimplifyAndSolve()
    {
        JsonObject solved = await SolveFields("x = symbol(\"x\"); assume(x > 0); solve(x^2 == 4, x)");
        Assert.Equal(new[] { "2" }, SolutionValues(solved));

        // the SAME store, read by the simplification path (audit M's own control row)
        var (exitCode, envelope) = await RunAsync(
            "x = symbol(\"x\"); assume(x > 0); simplify(sqrt(x^2))");
        Assert.Equal(0, exitCode);
        Assert.Equal("x", envelope["result"]!["structured"]!["pretty"]!.GetValue<string>());
    }

    /// <summary>An assumption the store cannot decide at a root is carried as that root's
    /// CONDITION, never dropped and never published as unconditional.</summary>
    [Fact]
    public async Task AResponseThatCannotDecideIsCarriedAsACondition()
    {
        // a family template k*pi cannot be tested against x > 5 one value at a time, so the
        // assumption rides on the family
        JsonObject family = await SolveFields("x = symbol(\"x\"); assume(x > 5); solve(sin(x) == 0, x)");
        Assert.Equal("Solved", family["status"]!["value"]!.GetValue<string>());
        Assert.Equal(1, family["families"]!["shape"]!.AsArray()[0]!.GetValue<int>());
        Assert.Equal(new[] { "x > 5" }, FamilyConditions(family));
    }

    /// <summary>solve_system publishes the SAME session store's verdict: an assignment the store
    /// refutes is not a solution of the session's system.</summary>
    [Fact]
    public async Task SystemSolve_HonoursTheSameStore()
    {
        JsonObject refuted = await SolveFields(
            "x = symbol(\"x\"); assume(x > 5); solve_system([x == 3], [x])", "SystemSolveResult");
        Assert.Equal("NoSolutions", refuted["status"]!["value"]!.GetValue<string>());
        Assert.Equal("true", refuted["complete"]!["value"]!.GetValue<string>());
        Assert.Equal(0, refuted["solutions"]!["shape"]!.AsArray()[0]!.GetValue<int>());

        JsonObject filtered = await SolveFields(
            "x = symbol(\"x\"); y = symbol(\"y\"); assume(y > 0); " +
            "solve_system([x == 1, y^2 == 4], [x, y])", "SystemSolveResult");
        Assert.Equal("Solved", filtered["status"]!["value"]!.GetValue<string>());
        Assert.Equal(new[] { "x=1, y=2" }, Assignments(filtered));
    }

    // ------------------------------------------------------------------
    // Controls: an assumption that admits every root, the empty store, and another symbol
    // ------------------------------------------------------------------

    [Fact]
    public async Task AnAssumptionThatAdmitsEveryRootPublishesTheSameSet()
    {
        JsonObject fields = await SolveFields("x = symbol(\"x\"); assume(x > -10); solve(x^2 == 4, x)");
        Assert.Equal("Solved", fields["status"]!["value"]!.GetValue<string>());
        Assert.Equal("true", fields["complete"]!["value"]!.GetValue<string>());
        Assert.Equal(new[] { "-2", "2" }, SolutionValues(fields));
        Assert.Empty(SolutionConditions(fields));
    }

    /// <summary>The store is read for the SOLVED variable only: an unrelated symbol's assumption
    /// is not a condition on x and must not alter x's solve.</summary>
    [Fact]
    public async Task AnAssumptionOnAnotherSymbolChangesNothing()
    {
        JsonObject fields = await SolveFields(
            "x = symbol(\"x\"); y = symbol(\"y\"); assume(y > 5); solve(x == 3, x)");
        Assert.Equal("Solved", fields["status"]!["value"]!.GetValue<string>());
        Assert.Equal(new[] { "3" }, SolutionValues(fields));
        Assert.Empty(SolutionConditions(fields));
    }

    [Fact]
    public async Task WithoutAssumptionsTheSolverPublishesExactlyWhatItDidBefore()
    {
        JsonObject fields = await SolveFields("x = symbol(\"x\"); solve(x^2 == 4, x)");
        Assert.Equal("Solved", fields["status"]!["value"]!.GetValue<string>());
        Assert.Equal(new[] { "-2", "2" }, SolutionValues(fields));
        Assert.Empty(SolutionConditions(fields));

        // an assumption-free solve still keeps the roots a domain assumption would exclude
        JsonObject complex = await SolveFields("x = symbol(\"x\"); solve(x^2 + 1 == 0, x)");
        Assert.Equal(new[] { "i", "-i" }, SolutionValues(complex));
        Assert.Empty(SolutionConditions(complex));
    }
}
