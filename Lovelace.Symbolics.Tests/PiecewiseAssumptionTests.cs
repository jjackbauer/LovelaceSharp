using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// §21: a Piecewise guard over a SYMBOLIC value is decided by the active assumptions, not only
/// numerically. Before this, `x >= 0` with x unbound reached numeric evaluation, threw, and came
/// back Unknown — so the branch was "undecidable" and the whole evaluation failed, no matter what
/// the assumptions said. NumPy-esque numeric probing cannot decide it; the lattice can.
/// </summary>
public class PiecewiseAssumptionTests
{
    private static ExprContext NewContext()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        return ctx;
    }

    private static Expr PiecewiseAbs(ExprContext ctx, Symbol x) => Exprs.Piecewise(
        new[] { new PiecewiseBranch(Exprs.Relation(RelOp.Ge, Exprs.Symbol(x), Exprs.Zero), Exprs.Integer(1)) },
        Exprs.Integer(2));

    [Fact]
    public void RefutedGuard_TakesTheOtherwiseBranch()
    {
        var ctx = NewContext();
        var x = ctx.Symbol("x");
        var pw = PiecewiseAbs(ctx, x);
        var assumptions = AssumptionSet.Empty.Add(new SymbolRelationAssumption(x, RelOp.Lt, Exprs.Zero));

        using (ctx.WithAssumptions(assumptions))
        {
            var value = Evaluation.EvaluateToNum(pw, ctx, new Dictionary<Symbol, Num>());
            Assert.Equal(0, NumOps.Compare(value, NumOps.FromLong(2L)));
        }
    }

    [Fact]
    public void ProvenGuard_TakesTheBranch()
    {
        var ctx = NewContext();
        var x = ctx.Symbol("x");
        var pw = PiecewiseAbs(ctx, x);
        var assumptions = AssumptionSet.Empty.Add(new SymbolRelationAssumption(x, RelOp.Gt, Exprs.Zero));

        using (ctx.WithAssumptions(assumptions))
        {
            var value = Evaluation.EvaluateToNum(pw, ctx, new Dictionary<Symbol, Num>());
            Assert.Equal(0, NumOps.Compare(value, NumOps.FromLong(1L)));
        }
    }

    [Fact]
    public void MirroredSpelling_IsAlsoDecided()
    {
        var ctx = NewContext();
        var x = ctx.Symbol("x");
        // the guard is written constant-first: 0 < x, the same fact as x > 0
        var pw = Exprs.Piecewise(
            new[] { new PiecewiseBranch(Exprs.Relation(RelOp.Lt, Exprs.Zero, Exprs.Symbol(x)), Exprs.Integer(1)) },
            Exprs.Integer(2));
        var assumptions = AssumptionSet.Empty.Add(new SymbolRelationAssumption(x, RelOp.Gt, Exprs.Zero));

        using (ctx.WithAssumptions(assumptions))
        {
            var value = Evaluation.EvaluateToNum(pw, ctx, new Dictionary<Symbol, Num>());
            Assert.Equal(0, NumOps.Compare(value, NumOps.FromLong(1L)));
        }
    }

    [Fact]
    public void WithoutAssumptions_TheGuardIsStillUndecidable()
    {
        var ctx = NewContext();
        var x = ctx.Symbol("x");
        var pw = PiecewiseAbs(ctx, x);
        // nothing known about x: the guard is genuinely undecidable and must stay an error rather
        // than silently picking a branch
        Assert.Throws<EvaluationException>(
            () => Evaluation.EvaluateToNum(pw, ctx, new Dictionary<Symbol, Num>()));
    }
}
