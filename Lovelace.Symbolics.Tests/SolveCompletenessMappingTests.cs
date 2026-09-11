using Lovelace.Abstractions;
using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// The (complete, completeness) pair is derived by ONE function of the effective status
/// (<c>SolveCompletenessMapping.Of</c>), so the two fields of a result record cannot disagree, and
/// the two result records cannot drift apart. This file drives every status the LANGUAGE can reach
/// through the published records and exercises the remaining status (BudgetExceeded, which the
/// solver cannot currently produce) directly against the mapping.
/// </summary>
public class SolveCompletenessMappingTests
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

    // ------------------------------------------------------------------
    // The mapping itself: every status of the kernel's SolveStatus enum
    // ------------------------------------------------------------------

    [Theory]
    [InlineData(SolveStatus.Solved, true, Completeness.Complete)]
    [InlineData(SolveStatus.NoSolutions, true, Completeness.Complete)]
    [InlineData(SolveStatus.Partial, false, Completeness.Partial)]
    [InlineData(SolveStatus.Unevaluated, false, Completeness.Unknown)]
    public void EveryStatusCarriesTheDocumentedPair(SolveStatus status, bool complete, Completeness completeness) =>
        Assert.Equal((complete, completeness), SolveCompletenessMapping.Of(status));

    /// <summary>BudgetExceeded is the one status the kernel cannot currently produce (nothing ever
    /// sets <c>SolutionSet.BudgetExceeded</c> before <c>WithBudgetExceeded</c> reads it), so it is
    /// reached here through the mapping directly: incomplete, carrying the kernel's Completeness for
    /// the partial subset it stopped on.</summary>
    [Fact]
    public void BudgetExceeded_IsIncomplete_WithTheKernelCompletenessOfThePartialSubset()
    {
        Assert.Equal((false, Completeness.Unknown), SolveCompletenessMapping.Of(SolveStatus.BudgetExceeded));
        Assert.Equal((false, Completeness.Partial),
            SolveCompletenessMapping.Of(SolveStatus.BudgetExceeded, Completeness.Partial));
    }

    // ------------------------------------------------------------------
    // The records the language produces: observed pair == mapping(status)
    // ------------------------------------------------------------------

    [Theory]
    [InlineData("solve_full(x^2 - 4 == 0, x)", "Solved", true, "Complete")]
    [InlineData("solve_full(x^4 - x^2 - 1 == 0, x)", "Partial", false, "Partial")]
    [InlineData("solve_full((x^2-1)/(x^2-1) == 0, x)", "NoSolutions", true, "Complete")]
    [InlineData("solve_full(x/x == 0, x)", "NoSolutions", true, "Complete")]
    [InlineData("solve_full(x^2 + 1 == 0, x, real)", "NoSolutions", true, "Complete")]
    [InlineData("solve_full(x^4 + 1 == 0, x)", "Unevaluated", false, "Unknown")]
    public void LanguageStatuses_ReportExactlyWhatTheMappingReturns(
        string expression, string status, bool complete, string completeness)
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");

        var r = engine.Evaluate(expression).AsRecord();
        var observed = (F(r, "status").AsEnum().Name, F(r, "complete").AsBoolean(), F(r, "completeness").AsEnum().Name);

        Assert.Equal((status, complete, completeness), observed);
        // the record's two fields are the mapping's output for the status it reports
        Assert.Equal((observed.Item2, observed.Item3),
            Describe(SolveCompletenessMapping.Of(Enum.Parse<SolveStatus>(observed.Item1))));
    }

    private static (bool Complete, string Completeness) Describe((bool Complete, Completeness Completeness) pair) =>
        (pair.Complete, pair.Completeness.ToString());
}
