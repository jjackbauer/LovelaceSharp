using Lovelace.Abstractions;
using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// The frozen solver contract (alignment plan section D) says <c>NoSolutions</c> means the solution
/// set over the requested domain is PROVABLY EMPTY. A provably empty set is a complete answer, so
/// <c>NoSolutions</c> must report <c>complete: true</c> / <c>completeness: Complete</c> — the pairing
/// this file pins for every status the kernel can produce, for BOTH result records.
/// </summary>
public class SolveCompletenessPairingTests
{
    private static SuiteEngine NewEngine()
    {
        var engine = new SuiteEngine();
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        return engine;
    }

    private static Value F(RecordValue record, string name) =>
        (Value)record.Fields.First(f => f.Name == name).Value!;

    /// <summary>The whole published pairing of a SolveResult: the status member name, the derived
    /// boolean and the Completeness member name — never one of the three alone.</summary>
    private static (string Status, bool Complete, string Completeness) Pair(SuiteEngine engine, string expression)
    {
        var r = engine.Evaluate(expression).AsRecord();
        return (F(r, "status").AsEnum().Name, F(r, "complete").AsBoolean(), F(r, "completeness").AsEnum().Name);
    }

    [Theory]
    // every candidate root removed by the denominator condition
    [InlineData("solve_full((x^2-1)/(x^2-1) == 0, x)")]
    [InlineData("solve_full(x/x == 0, x)")]
    // provably empty over the REQUESTED domain (real), with complex roots
    [InlineData("solve_full(x^2 + 1 == 0, x, real)")]
    public void NoSolutions_IsACompleteAnswer(string expression)
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");

        var (status, complete, completeness) = Pair(engine, expression);

        Assert.Equal("NoSolutions", status);
        Assert.True(complete, $"{expression}: a provably empty solution set is a complete answer");
        Assert.Equal("Complete", completeness);
    }

    [Fact]
    public void Partial_IsNeverComplete()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");

        var r = engine.Evaluate("solve_full(x^4 - x^2 - 1 == 0, x)").AsRecord();

        Assert.Equal("Partial", F(r, "status").AsEnum().Name);
        Assert.False(F(r, "complete").AsBoolean());
        Assert.Equal("Partial", F(r, "completeness").AsEnum().Name);
        Assert.Equal("2", F(r, "unrepresented_count").AsInteger().ToString());
    }

    [Fact]
    public void Solved_IsComplete()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");

        var (status, complete, completeness) = Pair(engine, "solve_full(x^2 - 4 == 0, x)");

        Assert.Equal("Solved", status);
        Assert.True(complete);
        Assert.Equal("Complete", completeness);
    }

    [Fact]
    public void Unevaluated_IsNotComplete()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");

        var (status, complete, completeness) = Pair(engine, "solve_full(x^4 + 1 == 0, x)");

        Assert.Equal("Unevaluated", status);
        Assert.False(complete);
        Assert.Equal("Unknown", completeness);
    }

    /// <summary>SystemSolveResult derives its own status; an inconsistent system is provably empty
    /// over the requested domain and therefore a complete answer too.</summary>
    [Fact]
    public void SystemSolve_NoSolutions_IsACompleteAnswer()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\"); y = symbol(\"y\")");

        var r = engine.Evaluate("solve_system_full([x + y == 1, x + y == 2], [x, y])").AsRecord();

        Assert.Equal("SystemSolveResult", r.TypeName);
        Assert.Equal("NoSolutions", F(r, "status").AsEnum().Name);
        Assert.True(F(r, "complete").AsBoolean(),
            "an inconsistent polynomial system has a provably empty solution set");
    }

    /// <summary>
    /// The documented table (dsh-protocol.md:158-164) walked for EVERY member of
    /// <see cref="SolveStatus"/>, on BOTH records the contract says cannot drift apart:
    /// <c>SolveResult</c> (solve_full) and <c>SystemSolveResult</c> (solve_system_full). The walk is
    /// total over the enum — a member added without a documented row, and a member not classified
    /// for a record, both fail here — and every member a kernel CAN publish is driven end-to-end and
    /// asserted against the same row. A member a kernel cannot publish (the search-budget status)
    /// is still paired through the ONE mapping expression both record builders read, which is what
    /// makes the two records unable to disagree.
    /// </summary>
    [Fact]
    public void EverySolveStatus_PublishesTheDocumentedPair_OnBothRecords()
    {
        // the frozen table, transcribed from the protocol: status -> (complete, completeness).
        // BudgetExceeded is the one row whose completeness is the kernel's own subset claim.
        var documented = new Dictionary<SolveStatus, (bool Complete, string Completeness)>
        {
            [SolveStatus.Solved] = (true, "Complete"),
            [SolveStatus.NoSolutions] = (true, "Complete"),
            [SolveStatus.Partial] = (false, "Partial"),
            [SolveStatus.Unevaluated] = (false, "Unknown"),
            [SolveStatus.BudgetExceeded] = (false, "Partial"),
        };

        // the table is TOTAL over the enum: a member cannot slip in undocumented
        var members = Enum.GetValues<SolveStatus>();
        Assert.Equal(members.Length, documented.Count);
        Assert.All(members, m =>
            Assert.True(documented.ContainsKey(m), $"undocumented SolveStatus member '{m}'"));

        // how each record is driven to publish each member: a script, or null together with the
        // reason that kernel's own result type cannot publish it
        var solveProducers = new Dictionary<SolveStatus, string?>
        {
            [SolveStatus.Solved] = "solve_full(x^2 - 4 == 0, x)",
            [SolveStatus.NoSolutions] = "solve_full(x/x == 0, x)",
            [SolveStatus.Partial] = "solve_full(x^4 - x^2 - 1 == 0, x)",
            [SolveStatus.Unevaluated] = "solve_full(x^4 + 1 == 0, x)",
            // no declared solve_full parameter can set SolutionSet.BudgetExceeded (the search
            // budget is a kernel default), so this member is paired through the shared mapping
            [SolveStatus.BudgetExceeded] = null,
        };
        var systemProducers = new Dictionary<SolveStatus, string?>
        {
            [SolveStatus.Solved] = "solve_system_full([x + y == 2, x - y == 0], [x, y])",
            [SolveStatus.NoSolutions] = "solve_system_full([x + y == 1, x + y == 2], [x, y])",
            // the enumeration cap: 2^8 assignments against the 128-solution limit
            [SolveStatus.Partial] = SystemPartialSystem,
            [SolveStatus.Unevaluated] = "solve_system_full([sin(x) == 0], [x])",
            // SystemSolveResult.Status is a total function of Truncated/Solutions/Note and cannot
            // return BudgetExceeded; the pairing is asserted through the shared mapping instead
            [SolveStatus.BudgetExceeded] = null,
        };

        // every member is CLASSIFIED for both records, so a new member cannot be skipped silently
        Assert.All(members, m =>
        {
            Assert.True(solveProducers.ContainsKey(m), $"SolveResult: unclassified SolveStatus member '{m}'");
            Assert.True(systemProducers.ContainsKey(m), $"SystemSolveResult: unclassified SolveStatus member '{m}'");
        });

        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\"); y = symbol(\"y\"); " + SystemPartialDeclare);

        int published = 0;
        foreach (var status in members)
        {
            var expected = documented[status];

            // (1) the ONE mapping, for every member. BudgetExceeded is the pass-through row: the
            // kernel's own subset claim is published unchanged (Partial here, Unknown by default).
            Assert.Equal((expected.Complete, expected.Completeness),
                Describe(SolveCompletenessMapping.Of(status, Completeness.Partial)));
            if (status == SolveStatus.BudgetExceeded)
                Assert.Equal((false, "Unknown"), Describe(SolveCompletenessMapping.Of(status)));

            // (2) every record that CAN publish the member publishes exactly that row
            foreach (var (expression, typeName) in new[]
                     {
                         (solveProducers[status], "SolveResult"),
                         (systemProducers[status], "SystemSolveResult"),
                     })
            {
                if (expression is null)
                    continue;
                var record = engine.Evaluate(expression).AsRecord();
                Assert.Equal(typeName, record.TypeName);
                Assert.Equal(status.ToString(), F(record, "status").AsEnum().Name);
                Assert.Equal((expected.Complete, expected.Completeness),
                    (F(record, "complete").AsBoolean(), F(record, "completeness").AsEnum().Name));
                published++;
            }
        }

        // non-vacuity: the walk really drove both records (4 reachable members x 2 records)
        Assert.Equal(8, published);
    }

    /// <summary>The system whose 2^8 assignments hit the kernel's 128-solution enumeration cap,
    /// which is the ONE way SystemSolveResult publishes Partial.</summary>
    private const string SystemPartialDeclare =
        "x1 = symbol(\"x1\"); x2 = symbol(\"x2\"); x3 = symbol(\"x3\"); x4 = symbol(\"x4\"); " +
        "x5 = symbol(\"x5\"); x6 = symbol(\"x6\"); x7 = symbol(\"x7\"); x8 = symbol(\"x8\"); ";

    private const string SystemPartialSystem =
        "solve_system_full([x1^2 == 1, x2^2 == 1, x3^2 == 1, x4^2 == 1, x5^2 == 1, x6^2 == 1, " +
        "x7^2 == 1, x8^2 == 1], [x1, x2, x3, x4, x5, x6, x7, x8])";

    private static (bool Complete, string Completeness) Describe(
        (bool Complete, Completeness Completeness) pair) => (pair.Complete, pair.Completeness.ToString());
}

