using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Scoped assumptions are flow-local (Cycle 2 P0). The brief requires a two-concurrent-scopes test:
/// with the previous field save/restore, two overlapping scopes interleaved and the last Dispose
/// restored a stale set. This test opens both scopes at the same moment and checks that each flow
/// sees its own set — and that the context's base set is untouched afterwards.
/// </summary>
public class AssumptionScopeIsolationTests
{
    [Fact]
    public async Task TwoConcurrentScopes_AreIsolated()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        var x = ctx.Symbol("x");
        var positive = AssumptionSet.Empty.Add(new SymbolRelationAssumption(x, RelOp.Gt, Exprs.Zero));
        var negative = AssumptionSet.Empty.Add(new SymbolRelationAssumption(x, RelOp.Lt, Exprs.Zero));
        var query = new SymbolRelationAssumption(x, RelOp.Gt, Exprs.Zero);

        // both scopes are open simultaneously: each task blocks until the other has entered its scope
        using var both = new Barrier(2);

        Tristate AskInsideScope(AssumptionSet scoped)
        {
            using var scope = ctx.WithAssumptions(scoped);
            both.SignalAndWait();
            return ctx.Assumptions.Ask(query);
        }

        var inPositive = Task.Run(() => AskInsideScope(positive));
        var inNegative = Task.Run(() => AskInsideScope(negative));
        var results = await Task.WhenAll(inPositive, inNegative);

        // the positive flow proves x > 0; the negative flow cannot (its own set says x < 0). Under
        // the old shared-field implementation one of these two would see the other's set.
        Assert.Equal(Tristate.True, results[0]);
        Assert.NotEqual(Tristate.True, results[1]);
        Assert.Equal(Tristate.False, results[1]);

        // the base set is untouched: scopes are flow-local, not a mutation of shared state
        Assert.Equal(AssumptionSet.Empty, ctx.Assumptions);
    }

    [Fact]
    public async Task NestedScopes_RestoreTheEnclosingScopeOnDispose()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        var x = ctx.Symbol("x");
        var outer = AssumptionSet.Empty.Add(new SymbolRelationAssumption(x, RelOp.Gt, Exprs.Zero));
        var inner = AssumptionSet.Empty.Add(new SymbolRelationAssumption(x, RelOp.Lt, Exprs.Zero));

        using (ctx.WithAssumptions(outer))
        {
            Assert.Equal(Tristate.True, ctx.Assumptions.Ask(new SymbolRelationAssumption(x, RelOp.Gt, Exprs.Zero)));
            using (ctx.WithAssumptions(inner))
            {
                Assert.Equal(Tristate.True, ctx.Assumptions.Ask(new SymbolRelationAssumption(x, RelOp.Lt, Exprs.Zero)));
            }
            // the inner scope ended: the enclosing one is visible again
            Assert.Equal(Tristate.True, ctx.Assumptions.Ask(new SymbolRelationAssumption(x, RelOp.Gt, Exprs.Zero)));
        }

        Assert.Equal(AssumptionSet.Empty, ctx.Assumptions);
        await Task.CompletedTask;
    }
}
