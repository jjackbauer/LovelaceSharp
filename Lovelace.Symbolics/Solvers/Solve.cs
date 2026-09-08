using Rat = global::Lovelace.Rational.Rational;
using Int = global::Lovelace.Integer.Integer;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Symbolics;

public enum SolutionKind { Exact, Empty, Unevaluated }

public sealed record Solution(Expr Value, AssumptionSet Conditions);

public sealed class SolutionSet
{
    public SolutionKind Kind { get; }
    public List<Solution> Solutions { get; } = new();
    public string? Note { get; }

    public SolutionSet(SolutionKind kind, string? note = null)
    {
        Kind = kind;
        Note = note;
    }
}

/// <summary>Equation solving: structural dispatch over polynomials, rationals, and elementary compositions.</summary>
public static class Solvers
{
    public static SolutionSet Solve(Expr equation, Symbol x, ExprContext? ctx = null)
    {
        ctx ??= Exprs.Current;
        Expr f;
        if (equation is RelationExpr r && r.Op == RelOp.Eq)
            f = Exprs.Subtract(r.Left, r.Right);
        else if (equation is RelationExpr)
            return new SolutionSet(SolutionKind.Unevaluated, "Inequality solving is not supported in v1.");
        else
            f = equation;

        // 1. polynomial in x
        if (Polynomial.TryFromExpr(f, ctx, new[] { x }, out var poly, out _))
            return SolvePolynomial(poly, x, ctx);

        // 2. rational function: solve numerator == 0, keep denominator != 0 conditions
        if (RationalFunctions.TryRationalize(f, ctx, out var num, out var den) &&
            Polynomial.TryFromExpr(num, ctx, new[] { x }, out var np, out _) &&
            Polynomial.TryFromExpr(den, ctx, new[] { x }, out var dp, out _))
        {
            if (!np.IsZero)
            {
                var inner = SolvePolynomial(np, x, ctx);
                var denExpr = dp.ToExpr();
                var cond = AssumptionSet.Empty.Add(new ExpressionPropertyAssumption(denExpr, SymbolPredicate.NonZero));
                var set = new SolutionSet(SolutionKind.Exact);
                foreach (var sol in inner.Solutions)
                    set.Solutions.Add(sol with { Conditions = Merge(sol.Conditions, cond) });
                return set;
            }
            return new SolutionSet(SolutionKind.Empty);
        }

        // 3. linear in x with arbitrary (non-polynomial) coefficients
        if (Algebra.TryCoefficients(f, x, out var coeffs) && coeffs.Count <= 2)
        {
            var c1 = coeffs.Count == 2 ? coeffs[1] : Exprs.Zero;
            var c0 = coeffs[0];
            if (!(c1 is RationalConstantExpr r1 && r1.Value.IsZero))
            {
                var set = new SolutionSet(SolutionKind.Exact);
                var root = Exprs.Divide(Exprs.Negate(c0), c1);
                var cond = AssumptionSet.Empty.Add(new ExpressionPropertyAssumption(c1, SymbolPredicate.NonZero));
                set.Solutions.Add(new Solution(root, cond));
                return set;
            }
        }

        // 4. elementary compositions
        var elementary = SolveElementary(f, x, ctx);
        if (elementary is not null)
            return elementary;

        return new SolutionSet(SolutionKind.Unevaluated, "No solver for this structure.");
    }

    private static AssumptionSet Merge(AssumptionSet a, AssumptionSet b)
    {
        var result = a;
        foreach (var atom in b.Atoms)
        {
            try
            {
                result = result.Add(atom);
            }
            catch (AssumptionContradictionException)
            {
                // contradictory conditions keep the first set (solver-level best effort)
            }
        }
        return result;
    }

    /// <summary>Univariate polynomial solving: linear/quadratic/cubic formulas, RootOf otherwise.</summary>
    public static SolutionSet SolvePolynomial(Polynomial poly, Symbol x, ExprContext ctx)
    {
        if (poly.IsZero)
            return new SolutionSet(SolutionKind.Unevaluated, "0 = 0: every value is a solution.");
        var factors = Factoring.FactorPoly(poly);
        var set = new SolutionSet(SolutionKind.Exact);
        foreach (var (factor, mult) in factors.Factors)
        {
            int deg = factor.TotalDegree;
            var roots = deg switch
            {
                1 => new[] { LinearRoot(factor) },
                2 => QuadraticRoots(factor),
                3 => CubicRoots(factor, ctx),
                _ => Enumerable.Range(0, deg).Select(i => (Expr)Exprs.RootOf(factor, i)).ToArray(),
            };
            foreach (var r in roots)
                set.Solutions.Add(new Solution(r, AssumptionSet.Empty));
        }
        if (set.Solutions.Count == 0)
            return new SolutionSet(SolutionKind.Empty);
        return set;
    }

    private static Expr LinearRoot(Polynomial p)
    {
        var c0 = p.ConstantTerm;
        var c1 = p.LeadingCoefficient(MonomialOrder.Lex);
        return Exprs.Divide(Exprs.Rational(Rat.Negate(c0)), Exprs.Rational(c1));
    }

    private static Expr[] QuadraticRoots(Polynomial p)
    {
        // a x^2 + b x + c
        var c = p.ConstantTerm;
        Rat b = Rat.Zero, a = Rat.Zero;
        foreach (var (m, coef) in p.Terms)
        {
            int d = m.TotalDegree;
            if (d == 1) b = coef;
            else if (d == 2) a = coef;
        }
        var disc = b * b - Rat.FromLong(4L) * a * c;
        var sqrtDisc = Exprs.Power(Exprs.Rational(disc), Exprs.Rational(1, 2));
        var twoA = Exprs.Rational(Rat.FromLong(2L) * a);
        var negB = Exprs.Rational(Rat.Negate(b));
        return new[]
        {
            Exprs.Divide(Exprs.Add(negB, sqrtDisc), twoA),
            Exprs.Divide(Exprs.Subtract(negB, sqrtDisc), twoA),
        };
    }

    private static Expr[] CubicRoots(Polynomial p, ExprContext ctx)
    {
        // normalize to monic x^3 + A x^2 + B x + C
        Rat A = Rat.Zero, B = Rat.Zero, C = Rat.Zero;
        var lc = p.LeadingCoefficient(MonomialOrder.Lex);
        foreach (var (m, coef) in p.Terms)
        {
            var cn = coef / lc;
            int d = m.TotalDegree;
            if (d == 2) A = cn;
            else if (d == 1) B = cn;
            else if (d == 0) C = cn;
        }
        // depress: x = t - A/3  →  t^3 + P t + Q
        var a3 = A / Rat.FromLong(3L);
        var P = B - A * A / Rat.FromLong(3L);
        var Q = Rat.FromLong(2L) * A * A * A / Rat.FromLong(27L) - A * B / Rat.FromLong(3L) + C;
        var disc = Q * Q / Rat.FromLong(4L) + P * P * P / Rat.FromLong(27L);

        Expr[] ts;
        if (disc >= Rat.Zero)
        {
            // one real root via Cardano
            var halfQ = Rat.Negate(Q) / Rat.FromLong(2L);
            var sqrtD = Exprs.Power(Exprs.Rational(disc), Exprs.Rational(1, 2));
            var u = Exprs.Power(Exprs.Add(Exprs.Rational(halfQ), sqrtD), Exprs.Rational(1, 3));
            var v = Exprs.Power(Exprs.Subtract(Exprs.Rational(halfQ), sqrtD), Exprs.Rational(1, 3));
            // omega = -1/2 + i*sqrt(3)/2
            var omega = Exprs.Add(
                Exprs.Rational(-1, 2),
                Exprs.Multiply(Exprs.Divide(Exprs.Power(Exprs.Rational(3L), Exprs.Rational(1, 2)), Exprs.Rational(2L)), Exprs.I));
            var omega2 = Exprs.Subtract(
                Exprs.Rational(-1, 2),
                Exprs.Multiply(Exprs.Divide(Exprs.Power(Exprs.Rational(3L), Exprs.Rational(1, 2)), Exprs.Rational(2L)), Exprs.I));
            ts = new[]
            {
                Exprs.Add(u, v),
                Exprs.Add(Exprs.Multiply(omega, u), Exprs.Multiply(omega2, v)),
                Exprs.Add(Exprs.Multiply(omega2, u), Exprs.Multiply(omega, v)),
            };
        }
        else
        {
            // three real roots via trigonometry
            var m = Exprs.Multiply(Exprs.Rational(Rat.FromLong(2L)), ExprsSqrt(Exprs.Rational(Rat.Negate(P) / Rat.FromLong(3L))));
            var theta = Exprs.Divide(
                Exprs.Function(ctx.Function("acos"), Exprs.Divide(
                    Exprs.Multiply(Exprs.Rational(Rat.FromLong(3L) * Q), ExprsSqrt(Exprs.Rational(Rat.FromLong(-3L) / P))),
                    Exprs.Multiply(Exprs.Rational(Rat.FromLong(2L) * P), Exprs.Rational(Rat.One)))),
                Exprs.Rational(3L));
            ts = Enumerable.Range(0, 3).Select(k =>
            {
                var angle = Exprs.Add(theta, Exprs.Divide(Exprs.Multiply(Exprs.Rational(2L * k), Exprs.Pi), Exprs.Rational(3L)));
                return Exprs.Multiply(m, Exprs.Function(ctx.Function("cos"), angle));
            }).ToArray();
        }

        var shift = Exprs.Rational(Rat.Negate(a3));
        return ts.Select(t => Exprs.Add(t, shift)).ToArray();
    }

    private static Expr ExprsSqrt(Expr e) => Exprs.Power(e, Exprs.Rational(1, 2));

    /// <summary>Solves exp(u)=c, log(u)=c, u^n=c, sin/cos/tan(u)=c via registry inverses (principal branches).</summary>
    private static SolutionSet? SolveElementary(Expr f, Symbol x, ExprContext ctx)
    {
        // bare h(u) = 0 with h invertible (e.g. sin(x) = 0)
        if (f is (FunctionExpr or PowerExpr))
        {
            var direct = SolveInverse(f, Exprs.Zero, x, ctx);
            if (direct is not null)
                return direct;
        }

        // f is of the form h(u) + c with h invertible and c a folded constant
        if (f is AddExpr add)
        {
            foreach (var term in add.Terms)
            {
                Expr rhs;
                if (term is RationalConstantExpr rc0)
                    rhs = Exprs.Rational(Rat.Negate(rc0.Value));   // h(u) + c = 0  ⇒  h(u) = -c
                else if (term is MultiplyExpr m && m.Factors.Length == 2 &&
                         m.Factors[0] is RationalConstantExpr rc && rc.Value.IsMinusOne)
                    rhs = m.Factors[1];
                else
                    continue;
                var u = Exprs.Subtract(f, term);   // h(u-part)
                if (u is not (FunctionExpr or PowerExpr))
                    continue;
                var result = SolveInverse(u, rhs, x, ctx);
                if (result is not null)
                    return result;
            }
        }
        return null;
    }

    private static SolutionSet? SolveInverse(Expr h, Expr c, Symbol x, ExprContext ctx)
    {
        Expr? u = null;
        Expr? inverse = null;
        Expr? conditionExpr = null;
        switch (h)
        {
            case FunctionExpr f when f.Arguments.Length == 1:
            {
                u = f.Arguments[0];
                inverse = f.Function.Name switch
                {
                    "exp" => Exprs.Function(ctx.Function("log"), c),
                    "log" => Exprs.Function(ctx.Function("exp"), c),
                    "sin" => Exprs.Function(ctx.Function("asin"), c),
                    "cos" => Exprs.Function(ctx.Function("acos"), c),
                    "tan" => Exprs.Function(ctx.Function("atan"), c),
                    _ => null,
                };
                if (f.Function.Name == "exp")
                    conditionExpr = c;   // c != 0 for principal log? exp(u) = 0 has no solution
                break;
            }
            case PowerExpr p when p.Exponent is RationalConstantExpr pe && pe.Value != Rat.One:
            {
                u = p.Base;
                var n = pe.Value;
                if (n.IsZero)
                    return null;
                // u^n = c → u = c^(1/n) (principal branch)
                inverse = Exprs.Power(c, Exprs.Rational(Rat.One / n));
                break;
            }
        }
        if (u is null || inverse is null)
            return null;
        // u is the part containing x: solve u == inverse for x recursively
        var eq = Exprs.Subtract(u, inverse);
        var inner = Solve(eq, x, ctx);
        if (inner.Kind == SolutionKind.Exact)
        {
            var set = new SolutionSet(SolutionKind.Exact);
            foreach (var sol in inner.Solutions)
            {
                var conds = sol.Conditions;
                if (conditionExpr is not null)
                {
                    try
                    {
                        conds = conds.Add(new ExpressionPropertyAssumption(conditionExpr, SymbolPredicate.NonZero));
                    }
                    catch (AssumptionContradictionException)
                    {
                    }
                }
                set.Solutions.Add(new Solution(sol.Value, conds));
            }
            return set;
        }
        return inner;
    }
}

/// <summary>Numeric evaluation of RootOf via real-root isolation (bisection + Newton).</summary>
public static class Roots
{
    public static Rl N(Expr rootOf, long digits, ExprContext? ctx = null)
    {
        ctx ??= Exprs.Current;
        if (rootOf is not RootOfExpr r)
            throw new InvalidOperationException("N() requires a RootOf expression.");
        var poly = r.DefiningPolynomial;
        var (sqfree, _) = Factoring.PrimitivePart(poly);
        // Cauchy bound
        Rat bound = Rat.One;
        foreach (var (_, c) in sqfree.Terms)
        {
            var t = Rat.Abs(c / poly.LeadingCoefficient(MonomialOrder.Lex));
            if (t + Rat.One > bound)
                bound = t + Rat.One;
        }
        var realRoots = new List<Rl>();
        using (Rl.WithPrecision(digits + 10, Math.Min(digits, 50)))
        {
            long steps = 400;
            var lo = RationalReal.ToReal(Rat.Negate(bound), (int)Math.Min(digits, 100));
            var hi = RationalReal.ToReal(bound, (int)Math.Min(digits, 100));
            var prevSign = SignAt(sqfree, lo);
            var x = lo;
            var step = (hi - lo) / Rl.Parse(steps.ToString(), null);
            for (long i = 0; i <= steps; i++)
            {
                var next = i == steps ? hi : lo + step * Rl.Parse(i.ToString(), null);
                var sign = SignAt(sqfree, next);
                if (sign == 0)
                {
                    realRoots.Add(next);
                }
                else if (prevSign != 0 && sign != prevSign)
                {
                    realRoots.Add(RefineRoot(sqfree, x, next, digits + 6));
                }
                if (sign != 0)
                {
                    prevSign = sign;
                    x = next;
                }
            }
        }
        realRoots.Sort();
        if (r.RootIndex >= realRoots.Count)
            throw new InvalidOperationException(
                $"RootOf index {r.RootIndex} is out of range: the polynomial has {realRoots.Count} real roots; complex-root isolation is not supported in v1.");
        return realRoots[r.RootIndex];
    }

    private static int SignAt(Polynomial p, Rl x)
    {
        var r = p.EvaluateAt(new[] { RationalReal.FromReal(x) });
        return r.IsZero ? 0 : (r.IsNegative ? -1 : 1);
    }

    private static Rl RefineRoot(Polynomial p, Rl lo, Rl hi, long digits)
    {
        var two = Rl.Parse("2", null);
        for (int i = 0; i < 1000; i++)
        {
            var mid = (lo + hi) / two;
            var s = SignAt(p, mid);
            if (s == 0)
                return mid;
            if (SignAt(p, lo) * s < 0)
                hi = mid;
            else
                lo = mid;
        }
        return (lo + hi) / two;
    }
}
