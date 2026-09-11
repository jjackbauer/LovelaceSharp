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
}
