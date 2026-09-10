using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Integer-domain interval emptiness (Cycle 2 item 13). An integer with fractional bounds can have
/// no model even though every atom is individually consistent: nothing lies strictly between 1/2
/// and 1. The negation test in <see cref="AssumptionSet.Add"/> cannot see this — the emptiness
/// comes from the domain — so it is decided separately. Over the reals the same bounds are
/// perfectly satisfiable, which is the control that keeps the rule from over-firing.
/// </summary>
public class IntegerIntervalContradictionTests
{
    private static (ExprContext Ctx, Symbol X) NewContext()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        return (ctx, ctx.Symbol("x"));
    }

    private static AssumptionSet IntegerSymbol(Symbol x) =>
        AssumptionSet.Empty.Add(new SymbolDomainAssumption(x, Domain.Integer));

    [Fact]
    public void Integer_StrictFractionalSandwich_IsContradictory()
    {
        var (_, x) = NewContext();
        var set = IntegerSymbol(x).Add(new SymbolRelationAssumption(x, RelOp.Gt, Exprs.Rational(1, 2)));

        var ex = Assert.Throws<AssumptionContradictionException>(
            () => set.Add(new SymbolRelationAssumption(x, RelOp.Lt, Exprs.One)));
        Assert.Contains("no integer", ex.Message);
        // the message renders the assumption, never C# record syntax
        Assert.DoesNotContain("SymbolRelationAssumption", ex.Message);
    }

    [Fact]
    public void Integer_UnitInterval_IsContradictory()
    {
        var (_, x) = NewContext();
        var set = IntegerSymbol(x).Add(new SymbolRelationAssumption(x, RelOp.Gt, Exprs.Zero));
        Assert.Throws<AssumptionContradictionException>(
            () => set.Add(new SymbolRelationAssumption(x, RelOp.Lt, Exprs.One)));
    }

    [Fact]
    public void Integer_NonEmptyInterval_IsAccepted()
    {
        var (_, x) = NewContext();
        var set = IntegerSymbol(x)
            .Add(new SymbolRelationAssumption(x, RelOp.Ge, Exprs.One))
            .Add(new SymbolRelationAssumption(x, RelOp.Le, Exprs.Integer(2)));
        Assert.False(set.IsUnsatisfiable);
    }

    [Fact]
    public void Integer_OpenBoundaryLeavesItsNeighbourReachable()
    {
        var (_, x) = NewContext();
        // x > 1 (open) and x <= 2 admits 2
        var set = IntegerSymbol(x)
            .Add(new SymbolRelationAssumption(x, RelOp.Gt, Exprs.One))
            .Add(new SymbolRelationAssumption(x, RelOp.Le, Exprs.Integer(2)));
        Assert.False(set.IsUnsatisfiable);
    }

    [Fact]
    public void Reals_WithTheSameFractionalBounds_StaySatisfiable()
    {
        var (_, x) = NewContext();
        // no integer-domain atom: 1/2 < x < 1 has real solutions and must be accepted
        var set = AssumptionSet.Empty
            .Add(new SymbolRelationAssumption(x, RelOp.Gt, Exprs.Rational(1, 2)))
            .Add(new SymbolRelationAssumption(x, RelOp.Lt, Exprs.One));
        Assert.False(set.IsUnsatisfiable);
    }
}
