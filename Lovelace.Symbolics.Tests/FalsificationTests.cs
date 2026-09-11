using Lovelace.Symbolics;
using Xunit;
using Rat = Lovelace.Rational.Rational;
using Rl = Lovelace.Real.Real;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Rewrite falsification (Phase 2): every shipped rewrite identity is attacked with
/// boundary-biased sample points. Sampling is falsification, not proof — evidence is labeled
/// FuzzVerified — but any disagreement here is a hard regression: the rule is wrong for its
/// declared domain and must be fixed or re-scoped.
/// </summary>
public class FalsificationTests
{
    // boundary-biased points: magnitudes around the singular/ordering boundaries of the rules.
    // Round 27 keeps every one of these 13 points, in this order, and widens the sweep with
    // near-pole / near-branch-cut / near-discontinuity / assumption-boundary regions
    // (see FalsificationRegions in FalsificationGateTests.cs). Superset only.
    internal static readonly Rat[] LegacyPoints =
    {
        Rat.From(-100, 1), Rat.From(-5, 1), Rat.From(-3, 2), Rat.From(-1, 1), Rat.From(-1, 3),
        Rat.From(-1, 1000), Rat.Zero, Rat.From(1, 1000), Rat.From(1, 3), Rat.From(1, 1),
        Rat.From(3, 2), Rat.From(5, 1), Rat.From(100, 1),
    };

    /// <summary>The widened sample set: the 13 legacy points plus every round-27 region.</summary>
    private static readonly Rat[] AllPoints = FalsificationRegions.AllRealPoints;

    private static ExprContext NewCtx()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        return ctx;
    }

    private static void FuzzIdentity(
        string ruleId, Expr lhs, Expr rhs, ExprContext ctx, Symbol x,
        AssumptionSet assumptions, Rat[] points)
    {
        using var scope = Rl.WithPrecision(40, 20);
        var tol = NumOps.FromReal(Rl.Parse("0." + new string('0', 11) + "1", null));
        using var assumeScope = ctx.WithAssumptions(assumptions);
        var unexpected = new List<string>();
        foreach (var p in points)
        {
            Num lv, rv;
            try
            {
                lv = Evaluation.EvaluateToNum(lhs, ctx, new Dictionary<Symbol, Num> { [x] = NumOps.FromRat(p) });
                rv = Evaluation.EvaluateToNum(rhs, ctx, new Dictionary<Symbol, Num> { [x] = NumOps.FromRat(p) });
            }
            catch (EvaluationException)
            {
                continue;   // outside the rule's definedness domain — not a counterexample
            }
            catch (Exception ex)
            {
                // A rule is wrong by throwing too: an unexpected exception type is a defect, not
                // "not applicable". Only the kernel's typed domain failure licenses a skip.
                unexpected.Add($"{ruleId} at x={p}: {ex.GetType().Name}: {ex.Message}");
                continue;
            }
            var delta = NumOps.Abs(NumOps.Subtract(lv, rv), ctx);
            Assert.True(NumOps.Compare(delta, tol) < 0,
                $"Rule '{ruleId}' falsified: {Printing.PrettyPrint(lhs)} != {Printing.PrettyPrint(rhs)} at x={p}");
        }

        Assert.True(unexpected.Count == 0,
            "rule(s) threw an unexpected exception instead of being falsified: " + string.Join(" | ", unexpected));
    }

    [Fact]
    public void TrigPythagorean_Universal()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var lhs = Exprs.Add(
            Exprs.Power(Exprs.Function(ctx.Function("sin"), x), 2),
            Exprs.Power(Exprs.Function(ctx.Function("cos"), x), 2));
        FuzzIdentity("trig.pythagorean-sin2-cos2", lhs, Exprs.One, ctx, x, AssumptionSet.Empty, AllPoints);
    }

    [Fact]
    public void SqrtSquare_NonNegative()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var lhs = Exprs.Power(Exprs.Power(x, 2), Exprs.Rational(1, 2));
        var a = AssumptionSet.Empty
            .Add(new SymbolDomainAssumption(x, Domain.Real))
            .Add(new SymbolPropertyAssumption(x, SymbolPredicate.NonNegative));
        var points = AllPoints.Where(p => !p.IsNegative).ToArray();
        FuzzIdentity("pow.sqrt-square-nonnegative", lhs, x, ctx, x, a, points);
    }

    [Fact]
    public void SqrtSquare_Real()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var lhs = Exprs.Power(Exprs.Power(x, 2), Exprs.Rational(1, 2));
        var a = AssumptionSet.Empty.Add(new SymbolDomainAssumption(x, Domain.Real));
        FuzzIdentity("pow.sqrt-square-real", lhs, Exprs.Function(ctx.Function("abs"), x), ctx, x, a, AllPoints);
    }

    [Fact]
    public void ExpLog_Universal_OnPositivePoints()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var lhs = Exprs.Function(ctx.Function("exp"), Exprs.Function(ctx.Function("log"), x));
        var points = AllPoints.Where(p => p > Rat.Zero).ToArray();
        FuzzIdentity("logexp.exp-log", lhs, x, ctx, x, AssumptionSet.Empty, points);
    }

    [Fact]
    public void LogExp_Real()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var lhs = Exprs.Function(ctx.Function("log"), Exprs.Function(ctx.Function("exp"), x));
        var a = AssumptionSet.Empty.Add(new SymbolDomainAssumption(x, Domain.Real));
        FuzzIdentity("logexp.log-exp", lhs, x, ctx, x, a, AllPoints);
    }

    [Fact]
    public void AbsSquare_Real()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var lhs = Exprs.Power(Exprs.Function(ctx.Function("abs"), x), 2);
        var a = AssumptionSet.Empty.Add(new SymbolDomainAssumption(x, Domain.Real));
        FuzzIdentity("abs.abs-square", lhs, Exprs.Power(x, 2), ctx, x, a, AllPoints);
    }

    [Fact]
    public void AbsSquare_IsNotUniversal_OverComplex()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        // |z|^2 ≠ z^2 for complex z: the fuzzer must NOT treat the unconstrained identity as
        // valid — verified by evaluating both sides with a complex value
        var lhs = Exprs.Power(Exprs.Function(ctx.Function("abs"), x), 2);
        var rhs = Exprs.Power(x, 2);
        using var scope = Rl.WithPrecision(40, 20);
        var cplx = NumOps.FromComplex(new Lovelace.Complex.Complex(
            Rl.Parse("1.5", null), Rl.Parse("2", null)));
        var lv = Evaluation.EvaluateToNum(lhs, ctx, new Dictionary<Symbol, Num> { [x] = cplx });
        var rv = Evaluation.EvaluateToNum(rhs, ctx, new Dictionary<Symbol, Num> { [x] = cplx });
        // complex values are unordered: compare magnitudes of the difference
        var magnitude = NumOps.Abs(NumOps.Subtract(lv, rv), ctx);
        Assert.True(NumOps.Compare(magnitude, NumOps.FromLong(0L)) > 0);
    }

    [Fact]
    public void AbsNeg_Universal()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var lhs = Exprs.Function(ctx.Function("abs"), Exprs.Negate(x));
        FuzzIdentity("abs.abs-neg", lhs, Exprs.Function(ctx.Function("abs"), x), ctx, x, AssumptionSet.Empty, AllPoints);
    }

    [Fact]
    public void CancelXOverX_NonZeroPoints()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var lhs = Exprs.Divide(x, x);
        var points = AllPoints.Where(p => !p.IsZero).ToArray();
        FuzzIdentity("rat.cancel-x-over-x", lhs, Exprs.One, ctx, x, AssumptionSet.Empty, points);
    }

    [Fact]
    public void CancelZeroOverX_NonZeroPoints()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var lhs = Exprs.Divide(Exprs.Zero, x);
        var points = AllPoints.Where(p => !p.IsZero).ToArray();
        FuzzIdentity("rat.cancel-zero-over-x", lhs, Exprs.Zero, ctx, x, AssumptionSet.Empty, points);
    }
}
