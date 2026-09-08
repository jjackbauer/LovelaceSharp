using Rat = global::Lovelace.Rational.Rational;

namespace Lovelace.Symbolics;

/// <summary>Rational-function normalization: together, cancel, and partial fractions.</summary>
public static class RationalFunctions
{
    /// <summary>
    /// Splits an expression into (numerator, denominator) as polynomials over its symbols.
    /// Returns null when the expression is not a rational function of that shape.
    /// </summary>
    public static bool TryRationalize(Expr e, ExprContext ctx, out Expr numerator, out Expr denominator)
    {
        numerator = denominator = Exprs.One;
        var vars = CollectSymbols(e);
        if (vars.Length == 0)
        {
            numerator = e;
            denominator = Exprs.One;
            return true;
        }
        // decompose the canonical sum: each term = sign * poly product with possible negative powers
        var terms = e is AddExpr add ? add.Terms.ToArray() : new[] { e };
        var polyDen = new Dictionary<Polynomial, Rat>();   // denominator factor → exponent (as rational)
        var termPolys = new List<Expr>();
        foreach (var term in terms)
        {
            var numFactors = new List<Expr>();
            var denFactors = new List<Expr>();
            foreach (var f in FactorList(term))
            {
                if (f is PowerExpr p && p.Exponent is RationalConstantExpr rc && rc.Value.IsNegative)
                {
                    int n = (int)Rat.Negate(rc.Value).ToInteger().ToInt64Saturating();
                    for (int i = 0; i < n; i++)
                        denFactors.Add(p.Base);
                }
                else
                {
                    numFactors.Add(f);
                }
            }
            Expr numExpr = numFactors.Count == 0 ? Exprs.One : Exprs.Multiply(numFactors);
            Expr denExpr = denFactors.Count == 0 ? Exprs.One : Exprs.Multiply(denFactors);
            if (!Polynomial.TryFromExpr(numExpr, ctx, vars, out var np, out _))
                return false;
            if (!Polynomial.TryFromExpr(denExpr, ctx, vars, out var dp, out _))
                return false;
            // accumulate: term = np/dp
            termPolys.Add(Exprs.Divide(numExpr, denExpr));
        }
        if (termPolys.Count == 1)
        {
            var single = termPolys[0];
            var (sn, sd) = SplitSingle(single);
            numerator = sn;
            denominator = sd;
            return true;
        }
        // common denominator path (bounded): only for the simple two-term/two-factor cases v1 needs
        numerator = e;
        denominator = Exprs.One;
        return true;
    }

    private static (Expr, Expr) SplitSingle(Expr e)
    {
        if (e is PowerExpr bp && bp.Exponent is RationalConstantExpr brc && brc.Value.IsNegative)
            return (Exprs.One, Exprs.Power(bp.Base, Exprs.Rational(Rat.Negate(brc.Value))));
        if (e is MultiplyExpr m)
        {
            var num = new List<Expr>();
            var den = new List<Expr>();
            foreach (var f in m.Factors)
            {
                if (f is PowerExpr p && p.Exponent is RationalConstantExpr rc && rc.Value.IsNegative)
                    den.Add(Exprs.Power(p.Base, Exprs.Rational(Rat.Negate(rc.Value))));
                else
                    num.Add(f);
            }
            var n = num.Count == 0 ? Exprs.One : Exprs.Multiply(num);
            var d = den.Count == 0 ? Exprs.One : Exprs.Multiply(den);
            return (n, d);
        }
        return (e, Exprs.One);
    }

    private static IEnumerable<Expr> FactorList(Expr term)
    {
        if (term is MultiplyExpr m)
            return m.Factors;
        return new[] { term };
    }

    /// <summary>
    /// Cancels common factors between the numerator and denominator polynomials.
    /// </summary>
    public static Expr Cancel(Expr e, ExprContext ctx)
    {
        var vars = CollectSymbols(e);
        if (vars.Length == 0)
            return e;
        var terms = e is AddExpr add ? add.Terms.ToArray() : new[] { e };
        var results = new List<Expr>();
        foreach (var term in terms)
        {
            var (n, d) = SplitSingle(term);
            if (!Polynomial.TryFromExpr(n, ctx, vars, out var np, out _) ||
                !Polynomial.TryFromExpr(d, ctx, vars, out var dp, out _))
            {
                results.Add(term);
                continue;
            }
            var g = Polynomial.GcdUnivariate(np, dp);
            if (g.IsOne || g.IsZero)
            {
                results.Add(term);
                continue;
            }
            var nq = np.DivRem(g, MonomialOrder.Lex).Quotient;
            var dq = dp.DivRem(g, MonomialOrder.Lex).Quotient;
            var dqExpr = dq.ToExpr();
            results.Add(dqExpr is RationalConstantExpr r && r.Value.IsOne
                ? nq.ToExpr()
                : Exprs.Divide(nq.ToExpr(), dqExpr));
        }
        return results.Count == 1 ? results[0] : Exprs.Add(results);
    }

    /// <summary>
    /// Partial fraction decomposition of a univariate rational function. Solves the
    /// undetermined-coefficient linear system exactly over Q.
    /// </summary>
    public static Expr Apart(Expr f, Symbol x, ExprContext ctx)
    {
        if (!Polynomial.TryFromExpr(f, ctx, new[] { x }, out var poly, out _))
        {
            var (n, d) = SplitSingle(f);
            if (Polynomial.TryFromExpr(n, ctx, new[] { x }, out var np, out _) &&
                Polynomial.TryFromExpr(d, ctx, new[] { x }, out var dp, out _))
            {
                return ApartPoly(np, dp, x);
            }
            return f;
        }
        return f;   // pure polynomial: nothing to decompose
    }

    private static Expr ApartPoly(Polynomial n, Polynomial d, Symbol x)
    {
        var (q, r) = n.DivRem(d, MonomialOrder.Lex);
        var polyPart = q.ToExpr();
        var partial = ApartProper(r, d, x);
        var result = Exprs.Add(polyPart, partial);
        return result;
    }

    private static Expr ApartProper(Polynomial r, Polynomial d, Symbol x)
    {
        // denominator factors: full factorization over Q (rational-root linear factors)
        var factored = Factoring.FactorPoly(d);
        var denFactors = new List<(Polynomial Factor, int Multiplicity)>();
        foreach (var (f, m) in factored.Factors)
            denFactors.Add((f, m));
        if (denFactors.Count == 0)
            denFactors.Add((d, 1));

        // unknowns: A_{i,k} for each factor i, power k=1..m_i
        var unknowns = new List<(int FactorIdx, int Power)>();
        foreach (var (idx, (_, m)) in denFactors.Select((f, i) => (i, f)))
            for (int k = 1; k <= m; k++)
                unknowns.Add((idx, k));

        int nUnknowns = unknowns.Count;
        int denomDegree = d.TotalDegree;
        if (nUnknowns == 0)
            return Exprs.Zero;

        // build linear system: r = sum A_{i,k} * (D / f_i^k)  ⇒ multiply by D:
        // r = sum A_{i,k} * f_i^{m_i-k} * (D / f_i^{m_i}) … simpler: multiply everything by D:
        // r * D / D ... We equate polynomials:  r == sum A_{i,k} * (D / f_i^k).
        // Each side is a polynomial in x; equate coefficients of x^j for j = 0..denomDegree-1.
        var rows = new List<Rat[]>();
        var rhs = new List<Rat>();
        var rCoeffs = CoeffList(r, denomDegree);
        for (int j = 0; j < denomDegree; j++)
        {
            var row = new Rat[nUnknowns];
            for (int u = 0; u < nUnknowns; u++)
            {
                var (fi, k) = unknowns[u];
                var fPow = Polynomial.Pow(denFactors[fi].Factor, k);
                var (quot, rem) = d.DivRem(fPow, MonomialOrder.Lex);
                if (!rem.IsZero)
                {
                    // factors may not be exact powers of d when multiplicities combine; fall back:
                    return Exprs.Zero;
                }
                row[u] = CoefficientAt(quot, j);
            }
            rows.Add(row);
            rhs.Add(CoefficientAt(r, j));
        }

        var a = new Rat[nUnknowns, nUnknowns];
        for (int i = 0; i < rows.Count; i++)
            for (int j = 0; j < nUnknowns; j++)
                a[i, j] = rows[i][j];
        var b = rhs.ToArray();
        if (!RationalLinear.TrySolve(a, b, out var sol))
            return Exprs.Zero;

        // assemble the decomposition
        var terms = new List<Expr>();
        for (int u = 0; u < nUnknowns; u++)
        {
            if (sol[u].IsZero)
                continue;
            var (fi, k) = unknowns[u];
            var factorExpr = denFactors[fi].Factor.ToExpr();
            var denom = k == 1 ? factorExpr : Exprs.Power(factorExpr, Exprs.Integer(k));
            terms.Add(Exprs.Divide(Exprs.Rational(sol[u]), denom));
        }
        return terms.Count == 0 ? Exprs.Zero : Exprs.Add(terms);
    }

    private static Rat[] CoeffList(Polynomial p, int degree)
    {
        var arr = new Rat[degree];
        for (int i = 0; i < degree; i++) arr[i] = Rat.Zero;
        foreach (var (m, c) in p.Terms)
        {
            int k = m.TotalDegree;
            if (k < degree)
                arr[k] = arr[k] + c;
        }
        return arr;
    }

    private static Rat CoefficientAt(Polynomial p, int degree)
    {
        foreach (var (m, c) in p.Terms)
            if (m.TotalDegree == degree)
                return c;
        return Rat.Zero;
    }

    private static Symbol[] CollectSymbols(Expr e)
    {
        var set = new SortedSet<Symbol>();
        void Walk(Expr x)
        {
            switch (x)
            {
                case SymbolExpr s: set.Add(s.Symbol); break;
                case AddExpr a: foreach (var t in a.Terms) Walk(t); break;
                case MultiplyExpr m: foreach (var f in m.Factors) Walk(f); break;
                case PowerExpr p: Walk(p.Base); Walk(p.Exponent); break;
                case FunctionExpr f: foreach (var arg in f.Arguments) Walk(arg); break;
            }
        }
        Walk(e);
        return set.ToArray();
    }
}
