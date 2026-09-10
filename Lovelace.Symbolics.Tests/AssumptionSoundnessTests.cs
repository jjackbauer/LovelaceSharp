using Lovelace.Symbolics;
using Xunit;
using Rat = Lovelace.Rational.Rational;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Assumption-engine soundness: the bound lattice is checked against a reference truth table,
/// contradictions are closed across property/relation/domain atoms, an unsatisfiable set is
/// visible as such, and unevaluable interval bounds are never treated as satisfied.
/// </summary>
public class AssumptionSoundnessTests
{
    private static readonly RelOp[] Ops =
        { RelOp.Eq, RelOp.Ne, RelOp.Gt, RelOp.Ge, RelOp.Lt, RelOp.Le };

    private static readonly Rat[] Bounds = { Rat.FromLong(4), Rat.FromLong(5), Rat.FromLong(6) };

    private static readonly Rat[] Witnesses =
    {
        Rat.FromLong(-100), Rat.FromLong(0), Rat.FromLong(3), Rat.FromLong(4),
        Rat.From(9, 2), Rat.FromLong(5), Rat.From(11, 2), Rat.FromLong(6), Rat.FromLong(100),
    };

    private static bool Satisfies(RelOp op, Rat v, Rat b) => op switch
    {
        RelOp.Eq => v == b,
        RelOp.Ne => v != b,
        RelOp.Gt => v > b,
        RelOp.Ge => v >= b,
        RelOp.Lt => v < b,
        RelOp.Le => v <= b,
        _ => false,
    };

    /// <summary>Every operator pair over a sample of bounds agrees with the reference truth table.</summary>
    [Fact]
    public void BoundReasoning_MatchesReferenceTruthTable()
    {
        var ctx = new ExprContext();
        var x = ctx.Symbol("x");
        var failures = new List<string>();
        foreach (var aop in Ops)
        {
            foreach (var ab in Bounds)
            {
                var set = AssumptionSet.Empty.Add(new SymbolRelationAssumption(x, aop, Exprs.Rational(ab)));
                foreach (var qop in Ops)
                {
                    foreach (var qb in Bounds)
                    {
                        var got = set.Ask(new SymbolRelationAssumption(x, qop, Exprs.Rational(qb)));
                        var inSet = Witnesses.Where(w => Satisfies(aop, w, ab)).ToArray();
                        bool all = inSet.Length > 0 && inSet.All(w => Satisfies(qop, w, qb));
                        bool none = inSet.All(w => !Satisfies(qop, w, qb));
                        var expected = all ? Tristate.True : none ? Tristate.False : Tristate.Unknown;
                        if (expected != Tristate.Unknown && got != expected)
                            failures.Add($"assume(x {aop} {ab}), ask(x {qop} {qb}): expected {expected}, got {got}");
                        if (expected == Tristate.Unknown && got != Tristate.Unknown)
                            failures.Add($"over-claim: assume(x {aop} {ab}), ask(x {qop} {qb}): got {got}");
                    }
                }
            }
        }
        Assert.True(failures.Count == 0, string.Join("; ", failures));
    }

    /// <summary>Every atom pair is accepted exactly when the reference model is non-empty.</summary>
    [Fact]
    public void ContradictionDetection_IsNeitherMissedNorInvented()
    {
        var ctx = new ExprContext();
        var x = ctx.Symbol("x");
        var failures = new List<string>();
        foreach (var o1 in Ops)
        {
            foreach (var b1 in Bounds)
            {
                foreach (var o2 in Ops)
                {
                    foreach (var b2 in Bounds)
                    {
                        bool contradicted = false;
                        try
                        {
                            AssumptionSet.Empty
                                .Add(new SymbolRelationAssumption(x, o1, Exprs.Rational(b1)))
                                .Add(new SymbolRelationAssumption(x, o2, Exprs.Rational(b2)));
                        }
                        catch (AssumptionContradictionException)
                        {
                            contradicted = true;
                        }
                        bool hasModel = Witnesses.Any(w => Satisfies(o1, w, b1) && Satisfies(o2, w, b2));
                        if (hasModel && contradicted)
                            failures.Add($"false contradiction ({o1} {b1}) & ({o2} {b2})");
                        if (!hasModel && !contradicted)
                            failures.Add($"missed contradiction ({o1} {b1}) & ({o2} {b2})");
                    }
                }
            }
        }
        Assert.True(failures.Count == 0, string.Join("; ", failures));
    }

    [Fact]
    public void PropertyAndRelationAtoms_Contradict()
    {
        var ctx = new ExprContext();
        var x = ctx.Symbol("x");
        Assert.Throws<AssumptionContradictionException>(() => AssumptionSet.Empty
            .Add(new SymbolPropertyAssumption(x, SymbolPredicate.Positive))
            .Add(new SymbolRelationAssumption(x, RelOp.Le, Exprs.Zero)));
        Assert.Throws<AssumptionContradictionException>(() => AssumptionSet.Empty
            .Add(new SymbolPropertyAssumption(x, SymbolPredicate.Negative))
            .Add(new SymbolRelationAssumption(x, RelOp.Ge, Exprs.Zero)));
        Assert.Throws<AssumptionContradictionException>(() => AssumptionSet.Empty
            .Add(new SymbolPropertyAssumption(x, SymbolPredicate.NonNegative))
            .Add(new SymbolRelationAssumption(x, RelOp.Lt, Exprs.Zero)));
        Assert.Throws<AssumptionContradictionException>(() => AssumptionSet.Empty
            .Add(new SymbolPropertyAssumption(x, SymbolPredicate.NonZero))
            .Add(new SymbolRelationAssumption(x, RelOp.Eq, Exprs.Zero)));
        // the reverse order must be caught as well
        Assert.Throws<AssumptionContradictionException>(() => AssumptionSet.Empty
            .Add(new SymbolRelationAssumption(x, RelOp.Le, Exprs.Zero))
            .Add(new SymbolPropertyAssumption(x, SymbolPredicate.Positive)));
    }

    [Fact]
    public void UnsatisfiableSet_AnswersUnsatisfiable_NotUnknown()
    {
        var ctx = new ExprContext();
        var x = ctx.Symbol("x");
        Assert.Equal(Tristate.Unsatisfiable,
            AssumptionSet.Unsatisfiable.Ask(new SymbolRelationAssumption(x, RelOp.Gt, Exprs.Zero)));
        Assert.Equal(Tristate.Unsatisfiable,
            AssumptionSet.Unsatisfiable.Ask(new SymbolPropertyAssumption(x, SymbolPredicate.Positive)));
    }

    [Fact]
    public void IntervalBounds_WithUnevaluableBound_AreUnknown()
    {
        var ctx = new ExprContext();
        var x = ctx.Symbol("x");
        var set = AssumptionSet.Empty.Add(new SymbolDomainAssumption(x, Domain.Real));
        var im = Exprs.Function(ctx.Function("im"), Exprs.Symbol(x));
        // 0 IS in (-pi, pi]
        Assert.Equal(Tristate.True, set.Ask(new IntervalAssumption(im, Exprs.Negate(Exprs.Pi), true, Exprs.Pi, false)));
        // 0 is NOT in [pi, 2pi]: an unevaluable-looking bound must still decide correctly
        Assert.Equal(Tristate.False, set.Ask(new IntervalAssumption(im, Exprs.Pi, false, Exprs.Multiply(2, Exprs.Pi), false)));
        // a genuinely symbolic bound is unknown, never "satisfied"
        Assert.Equal(Tristate.Unknown, set.Ask(new IntervalAssumption(im,
            Exprs.Symbol(ctx.Symbol("a")), true, Exprs.Symbol(ctx.Symbol("b")), false)));
    }

    [Fact]
    public void RealValuedBounds_ParticipateInReasoning()
    {
        var ctx = new ExprContext();
        var x = ctx.Symbol("x");
        var set = AssumptionSet.Empty.Add(new SymbolRelationAssumption(x, RelOp.Ge, Exprs.Real(RealLiteral.Parse("5"))));
        Assert.Equal(Tristate.False, set.Ask(new SymbolRelationAssumption(x, RelOp.Lt, Exprs.Rational(Rat.FromLong(5)))));
        Assert.Equal(Tristate.True, set.Ask(new SymbolRelationAssumption(x, RelOp.Gt, Exprs.Rational(Rat.FromLong(4)))));
    }

    [Fact]
    public void DomainAtoms_MergeToTheNarrowest()
    {
        var ctx = new ExprContext();
        var x = ctx.Symbol("x");
        var set = AssumptionSet.Empty
            .Add(new SymbolDomainAssumption(x, Domain.Complex))
            .Add(new SymbolDomainAssumption(x, Domain.Integer));
        var assumptions = new ExprContext { Assumptions = set };
        Assert.Equal(Domain.Integer, Domains.DomainOfSymbol(x, assumptions));
    }
}
