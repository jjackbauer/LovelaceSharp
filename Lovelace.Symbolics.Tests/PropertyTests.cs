using Lovelace.MathIR;
using Lovelace.Symbolics;
using Xunit;
using Int = Lovelace.Integer.Integer;
using Rat = Lovelace.Rational.Rational;
using Rl = Lovelace.Real.Real;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Deterministic property tests (fixed seed, boundary-biased sampling): the invariants every
/// kernel layer must uphold. These are Phase 2 gates over the Phase 1 trusted semantics.
/// </summary>
public class PropertyTests
{
    private static ExprContext NewCtx()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        return ctx;
    }

    // ------------------------------------------------------------------
    // Generator: small random canonical expressions (polynomial/trig core)
    // ------------------------------------------------------------------

    private static Expr RandomExpr(Random rng, ExprContext ctx, int depth, string name)
    {
        var x = ctx.Symbol(name);
        if (depth <= 0)
        {
            return rng.Next(4) switch
            {
                0 => Exprs.Integer(rng.Next(-5, 6)),
                1 => Exprs.Rational(rng.Next(-5, 6), Math.Max(1, rng.Next(1, 5))),
                _ => x,
            };
        }
        var a = RandomExpr(rng, ctx, depth - 1, name);
        var b = RandomExpr(rng, ctx, depth - 1, name);
        return rng.Next(7) switch
        {
            0 => Exprs.Add(a, b),
            1 => Exprs.Multiply(a, b),
            2 => Exprs.Power(a, Exprs.Integer(rng.Next(0, 5))),
            3 => Exprs.Function(ctx.Function("sin"), a),
            4 => Exprs.Function(ctx.Function("exp"), a),
            5 => Exprs.Function(ctx.Function("abs"), a),
            _ => Exprs.Subtract(a, b),
        };
    }

    private static Num EvalPoint(Expr e, ExprContext ctx, Symbol x, Rat point)
    {
        using var scope = Rl.WithPrecision(50, 20);
        return Evaluation.EvaluateToNum(e, ctx, new Dictionary<Symbol, Num>
        {
            [x] = NumOps.FromRat(point),
        });
    }

    // ------------------------------------------------------------------
    // Canonicalization: canonical text round-trips and re-interns identically
    // ------------------------------------------------------------------

    [Fact]
    public void CanonicalText_RoundTripsAndReinterns()
    {
        var ctx = NewCtx();
        var rng = new Random(20260909);
        for (int i = 0; i < 200; i++)
        {
            var e = RandomExpr(rng, ctx, rng.Next(1, 5), "x");
            var text = Printing.CanonicalPrint(e);
            var back = Printing.CanonicalParse(text, ctx);
            Assert.Same(e, back);   // canonical form is unique up to interning
        }
    }

    // ------------------------------------------------------------------
    // Simplification: eval(simplify(f)) == eval(f) under the rule preconditions
    // ------------------------------------------------------------------

    [Fact]
    public void Simplify_PreservesValues_UnderRealAssumption()
    {
        var ctx = NewCtx();
        var rng = new Random(20260910);
        var x = ctx.Symbol("x");
        // run under the real assumption so the conditional rules actually fire
        using var assumptions = ctx.WithAssumptions(
            AssumptionSet.Empty.Add(new SymbolDomainAssumption(x, Domain.Real)));
        using var scope = Rl.WithPrecision(50, 20);
        var points = new[] { Rat.From(-7, 4), Rat.From(-1, 3), Rat.From(1, 5), Rat.From(3, 2), Rat.From(11, 4) };
        for (int i = 0; i < 150; i++)
        {
            var e = RandomExpr(rng, ctx, rng.Next(1, 4), "x");
            var simplified = Simplify.SimplifyExpr(e, ctx);
            foreach (var p in points)
            {
                var before = EvalPoint(e, ctx, x, p);
                var after = EvalPoint(simplified, ctx, x, p);
                Assert.True(NumOps.Compare(before, after) == 0,
                    $"simplify changed {Printing.PrettyPrint(e)} -> {Printing.PrettyPrint(simplified)} at x={p}");
            }
        }
    }

    // ------------------------------------------------------------------
    // Differentiation: symbolic derivative ≈ high-precision central difference
    // ------------------------------------------------------------------

    [Fact]
    public void Diff_MatchesNumericalDerivative()
    {
        var ctx = NewCtx();
        var rng = new Random(20260911);
        var x = ctx.Symbol("x");
        var h = Rl.Parse("0." + new string('0', 19) + "1", null);
        var points = new[] { Rat.From(-3, 2), Rat.From(1, 4), Rat.From(2, 3) };
        for (int i = 0; i < 60; i++)
        {
            var e = RandomExpr(rng, ctx, rng.Next(1, 4), "x");
            var d = Calculus.Diff(e, x, ctx);
            foreach (var p in points)
            {
                using var scope = Rl.WithPrecision(50, 20);
                var pv = NumOps.FromRat(p);
                var fPlus = Evaluation.EvaluateToNum(e, ctx, new Dictionary<Symbol, Num> { [x] = NumOps.Add(pv, NumOps.FromReal(h)) });
                var fMinus = Evaluation.EvaluateToNum(e, ctx, new Dictionary<Symbol, Num> { [x] = NumOps.Subtract(pv, NumOps.FromReal(h)) });
                var numeric = NumOps.Divide(NumOps.Subtract(fPlus, fMinus), NumOps.FromReal(Rl.Parse("2", null) * h));
                var symbolic = Evaluation.EvaluateToNum(d, ctx, new Dictionary<Symbol, Num> { [x] = pv });
                var tol = NumOps.FromReal(Rl.Parse("0." + new string('0', 9) + "1", null));
                var delta = NumOps.Abs(NumOps.Subtract(numeric, symbolic), ctx);
                Assert.True(NumOps.Compare(delta, tol) < 0,
                    $"d/dx {Printing.PrettyPrint(e)} = {Printing.PrettyPrint(d)} disagrees with numerics at x={p}");
            }
        }
    }

    // ------------------------------------------------------------------
    // Integration: diff(integrate(f)) == f whenever a closed form is returned
    // ------------------------------------------------------------------

    [Fact]
    public void Integrate_DiffRoundTrips()
    {
        var ctx = NewCtx();
        var rng = new Random(20260912);
        var x = ctx.Symbol("x");
        var integrands = new Func<Expr>[]
        {
            () => Exprs.Power(x, Exprs.Integer(rng.Next(0, 9))),
            () => Exprs.Add(Exprs.Power(x, 3), Exprs.Multiply(2, x), 1),
            () => Exprs.Function(ctx.Function("sin"), Exprs.Multiply(2, x)),
            () => Exprs.Function(ctx.Function("exp"), x),
            () => Exprs.Divide(Exprs.One, x),
            () => Exprs.Multiply(Exprs.Power(x, 2), Exprs.Function(ctx.Function("sin"), Exprs.Power(x, 3))),
        };
        foreach (var mk in integrands)
        {
            var f = mk();
            var result = Integration.Integrate(f, x, ctx);
            if (result is IntegralExpr)
                continue;   // unevaluated is an honest outcome
            var check = Algebra.Expand(Calculus.Diff(result, x, ctx), ctx);
            var expected = Algebra.Expand(f, ctx);
            Assert.Equal(Printing.CanonicalPrint(expected), Printing.CanonicalPrint(check));
        }
    }

    // ------------------------------------------------------------------
    // Solving: solution substituted into the equation → zero (exact cases)
    // ------------------------------------------------------------------

    [Fact]
    public void Solve_QuadraticRoots_SatisfyPolynomial()
    {
        var ctx = NewCtx();
        var rng = new Random(20260913);
        var x = ctx.Symbol("x");
        for (int i = 0; i < 40; i++)
        {
            var a = rng.Next(1, 7);
            var b = rng.Next(-7, 8);
            var c = rng.Next(-7, 8);
            var poly = Exprs.Add(
                Exprs.Multiply(a, Exprs.Power(x, 2)),
                Exprs.Multiply(b, x),
                c);
            var set = Solvers.Solve(Exprs.Relation(RelOp.Eq, poly, Exprs.Zero), x, ctx);
            Assert.Equal(SolutionKind.Exact, set.Kind);
            Assert.Equal(2, set.Solutions.Count);
            foreach (var sol in set.Solutions)
            {
                var at = Algebra.Expand(Evaluation.Substitute(poly, ctx, new Dictionary<Symbol, Expr> { [x] = sol.Value }), ctx);
                Assert.True(at is RationalConstantExpr rc && rc.Value.IsZero,
                    $"root {Printing.PrettyPrint(sol.Value)} does not satisfy the quadratic");
            }
        }
    }

    // ------------------------------------------------------------------
    // Optimization: eval(optimize(f)) == eval(f) at points
    // ------------------------------------------------------------------

    [Fact]
    public void Optimize_PreservesValues()
    {
        var ctx = NewCtx();
        var rng = new Random(20260914);
        var x = ctx.Symbol("x");
        using var scope = Rl.WithPrecision(50, 20);
        for (int i = 0; i < 40; i++)
        {
            var e = Exprs.Add(
                Exprs.Multiply(rng.Next(1, 6), Exprs.Power(x, rng.Next(1, 6))),
                Exprs.Multiply(rng.Next(1, 6), Exprs.Power(x, rng.Next(1, 6))),
                rng.Next(-5, 6));
            var optimized = Optimizer.Optimize(e, new[] { x }, ctx).Expression;
            foreach (var p in new[] { Rat.From(-2, 3), Rat.From(1, 2), Rat.From(5, 4) })
            {
                var before = EvalPoint(e, ctx, x, p);
                var after = EvalPoint(optimized, ctx, x, p);
                Assert.True(NumOps.Compare(before, after) == 0,
                    $"optimize changed {Printing.PrettyPrint(e)} -> {Printing.PrettyPrint(optimized)} at x={p}");
            }
        }
    }

    // ------------------------------------------------------------------
    // MathIR: eval(symbolic) == evalir(lower(symbolic)) over generated corpora
    // ------------------------------------------------------------------

    [Fact]
    public void MathIR_EvaluatesLikeSymbolic()
    {
        var ctx = NewCtx();
        var rng = new Random(20260915);
        var x = ctx.Symbol("x");
        using var scope = Rl.WithPrecision(50, 20);
        for (int i = 0; i < 60; i++)
        {
            var e = RandomExpr(rng, ctx, rng.Next(1, 4), "x");
            var prog = Lowering.Lower(e, ctx, new[] { x });
            foreach (var p in new[] { Rat.From(-3, 4), Rat.From(1, 3), Rat.From(7, 5) })
            {
                var pv = NumOps.FromRat(p);
                var direct = Evaluation.EvaluateToNum(e, ctx, new Dictionary<Symbol, Num> { [x] = pv });
                var lowered = IrEvaluator.Evaluate(prog, new Dictionary<Symbol, Num> { [x] = pv }, ctx);
                Assert.True(NumOps.Compare(direct, lowered) == 0,
                    $"MathIR disagrees with symbolic evaluation for {Printing.PrettyPrint(e)} at x={p}");
            }
        }
    }
}
