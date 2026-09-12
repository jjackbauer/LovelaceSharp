using Lovelace.Abstractions;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Suite.Tests;

/// <summary>
/// K-3 (round-21 audit K): ONE <see cref="SymbolicsPlugin"/> instance loaded into TWO engines.
/// The assumption builtins document themselves as session-scoped ("assumes ... for the session"),
/// and a session is the engine the plugin was loaded into - so the second engine must answer from
/// its OWN assumption set. The existing <see cref="CrossEngineIsolationTests"/> builds a FRESH
/// plugin per engine and therefore never exercises the shared-instance path; these tests do, on
/// both surfaces that read the store (<c>simplify</c> and <c>inspect(...).assumptions</c>).
/// </summary>
public class SharedPluginInstanceAssumptionIsolationTests
{
    /// <summary>The audit's wiring: one plugin instance, two engines, accepted without a warning.</summary>
    private static (SuiteEngine A, SuiteEngine B) TwoEnginesOnePlugin()
    {
        var symbolics = new SymbolicsPlugin();
        var a = new SuiteEngine();
        a.LoadPlugin(symbolics);
        var b = new SuiteEngine();
        b.LoadPlugin(symbolics);
        return (a, b);
    }

    [Fact]
    public async Task OnePluginTwoEngines_AssumptionInA_DoesNotDecideB()
    {
        var (a, b) = TwoEnginesOnePlugin();
        await a.EvaluateAsync("x = symbol(\"x\"); assume(x > 0)");

        // engine A answers from its own set: the single-engine behaviour, unchanged
        Assert.True((await a.EvaluateAsync("simplify(x > 0)")).AsBoolean(),
            "engine A lost its own assumption (x > 0)");

        // engine B never assumed anything: both questions stay unevaluated relations
        var bPositive = await b.EvaluateAsync("x = symbol(\"x\"); simplify(x > 0)");
        var bNegative = await b.EvaluateAsync("simplify(x < 0)");
        Assert.Equal("x > 0", b.FormatValue(bPositive));
        Assert.Equal("x < 0", b.FormatValue(bNegative));

        // and the engine that did assume is still governed by its own set alone: x > 0 is True
        // there AND its mirror x < 0 is decided FALSE there (and only there)
        Assert.True((await a.EvaluateAsync("simplify(x > 0)")).AsBoolean());
        Assert.False((await a.EvaluateAsync("simplify(x < 0)")).AsBoolean());
    }

    [Fact]
    public async Task OnePluginTwoEngines_EveryAssumptionReaderAnswersForTheAskingEngine()
    {
        var (a, b) = TwoEnginesOnePlugin();
        await a.EvaluateAsync("x = symbol(\"x\"); assume(x > 0)");

        // inspect(...).assumptions rides the ISymbolicInspectionBridge the engine was given
        Assert.NotEmpty((await a.EvaluateAsync("inspect(x).assumptions")).AsVector());
        Assert.Empty((await b.EvaluateAsync("x = symbol(\"x\"); inspect(x).assumptions")).AsVector());

        // assumptions() rides the builtin closure the engine was given
        Assert.NotEmpty((await a.EvaluateAsync("assumptions().assumptions")).AsVector());
        Assert.Empty((await b.EvaluateAsync("assumptions().assumptions")).AsVector());

        // assume_clear() on B must not clear A's session either
        await b.EvaluateAsync("assume_clear()");
        Assert.NotEmpty((await a.EvaluateAsync("assumptions().assumptions")).AsVector());
    }

    /// <summary>
    /// The interleaving variant: engine A's assumption is COMMITTED while A's evaluation is still
    /// in flight (a host builtin holds A inside its own evaluation). B asks then. The lock A holds
    /// is A's own evaluation gate, so nothing serialises B behind A - B answers for B.
    /// </summary>
    [Fact]
    public async Task OnePluginTwoEngines_AssumptionCommittedWhileAIsRunning_DoesNotDecideB()
    {
        var (a, b) = TwoEnginesOnePlugin();
        using var committed = new ManualResetEventSlim(false);
        using var release = new ManualResetEventSlim(false);
        a.RegisterBuiltin("hold_gate", [], _ =>
        {
            committed.Set();
            release.Wait(TimeSpan.FromSeconds(30));
            return new Value(true);
        });

        var aRun = Task.Run(() => a.EvaluateAsync("x = symbol(\"x\"); assume(x > 0); hold_gate()"));
        Assert.True(committed.Wait(TimeSpan.FromSeconds(30)), "engine A never reached hold_gate()");
        try
        {
            // A's assumption is committed and A is still inside its own evaluation: B must not see it
            var bAnswer = await b.EvaluateAsync("x = symbol(\"x\"); simplify(x > 0)");
            Assert.Equal("x > 0", b.FormatValue(bAnswer));
            Assert.Empty((await b.EvaluateAsync("inspect(x).assumptions")).AsVector());
        }
        finally
        {
            release.Set();
        }

        await aRun;
        Assert.True((await a.EvaluateAsync("simplify(x > 0)")).AsBoolean(),
            "engine A lost its own assumption after the interleaved evaluation");
    }

    /// <summary>
    /// BEHAVIOUR PRESERVED, NOT FIXED HERE: a scope the immediate caller installed on the plugin's
    /// context (<c>ExprContext.WithAssumptions</c>) has always had precedence inside an engine
    /// evaluation and still does — the engine installs its own store only when the caller has not
    /// installed one. This test passes on BOTH trees; it is the guard that closing the cross-engine
    /// leak did not narrow a single-engine surface.
    /// </summary>
    [Fact]
    public async Task CallerInstalledAssumptionScope_KeepsPrecedenceInsideAnEngineEvaluation()
    {
        var symbolics = new SymbolicsPlugin();
        var engine = new SuiteEngine();
        engine.LoadPlugin(symbolics);
        await engine.EvaluateAsync("x = symbol(\"x\")");

        var scoped = AssumptionSet.Empty.Add(
            new SymbolPropertyAssumption(symbolics.Context.Symbol("x"), SymbolPredicate.Positive));
        using (symbolics.Context.WithAssumptions(scoped))
        {
            Assert.True((await engine.EvaluateAsync("simplify(x > 0)")).AsBoolean(),
                "a caller-installed scope lost its precedence inside the engine's evaluation");
        }

        // the scope is gone: the engine's own (empty) session is what answers again
        Assert.Equal("x > 0", engine.FormatValue(await engine.EvaluateAsync("simplify(x > 0)")));
    }
}
