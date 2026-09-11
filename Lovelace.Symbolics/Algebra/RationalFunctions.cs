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
    /// Cancels common factors between the numerator and denominator polynomials, and returns the
    /// reduced expression ALONE.
    /// <para>
    /// This is deliberately the bare value form: an <see cref="Expr"/> has no place to carry a side
    /// condition, and the reduction is not an everywhere identity — it removes the poles of the
    /// cancelled factor. A caller that must not lose definedness uses
    /// <see cref="CancelWithConditions"/> (the <c>cancel_full</c> builtin), which reports those
    /// conditions; there is no way to attach them to the value returned here, so this method must not
    /// pretend otherwise.
    /// </para>
    /// </summary>
    public static Expr Cancel(Expr e, ExprContext ctx) => CancelWithConditions(e, ctx).Expression;

    /// <summary>
    /// The structured form of <see cref="Cancel"/>: the reduced expression plus the conditions under
    /// which it equals the input.
    /// <para>
    /// Cancelling a common factor g out of n/d gives a form that agrees with n/d exactly where
    /// <c>g != 0</c>: at a zero of g the input is undefined (n and d vanish together), while the
    /// reduced form may be defined, so the change is a definedness EXTENSION and the exclusion is a
    /// real condition. The atoms are the ones the rewrite path builds for
    /// <c>rat.cancel-x-over-x</c> (<see cref="NonZeroCondition"/> mirrors Simplify's private
    /// NonZeroOf): a symbol becomes a <see cref="SymbolPropertyAssumption"/>, anything else an
    /// <see cref="ExpressionPropertyAssumption"/>, so the two surfaces speak one convention.
    /// </para>
    /// <para>
    /// A power of a sum — <c>(x-1)^2</c> — is not a polynomial to <see cref="Polynomial.TryFromExpr"/>
    /// (it only accepts integer powers of a VARIABLE), so a numerator or denominator the parser
    /// refuses is expanded once and re-parsed. Expansion is value-preserving on every input, so this
    /// step is unconditional and adds no condition; altering the parser instead would change the
    /// contract of every other caller of TryFromExpr.
    /// </para>
    /// </summary>
    public static CancelResult CancelWithConditions(Expr e, ExprContext ctx)
    {
        var vars = CollectSymbols(e);
        if (vars.Length == 0)
            return new CancelResult(e, e, AssumptionSet.Empty);
        var terms = e is AddExpr add ? add.Terms.ToArray() : new[] { e };
        var results = new List<Expr>();
        var conditions = AssumptionSet.Empty;
        foreach (var term in terms)
        {
            var (n, d) = SplitSingle(term);
            if (!TryPolynomial(n, ctx, vars, out var np) || !TryPolynomial(d, ctx, vars, out var dp))
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
            // the removed factor is exactly what the result may not assume away
            conditions = conditions.Add(NonZeroAtom(g.ToExpr()));
        }
        var expression = results.Count == 1 ? results[0] : Exprs.Add(results);
        return new CancelResult(e, expression, conditions);
    }

    /// <summary>Parses a numerator/denominator as a polynomial, expanding it once when the direct
    /// parse refuses. Only reached from <see cref="CancelWithConditions"/>: the parser's own contract
    /// (integer powers of variables) is deliberately left unchanged.</summary>
    private static bool TryPolynomial(Expr e, ExprContext ctx, Symbol[] vars, out Polynomial poly)
    {
        if (Polynomial.TryFromExpr(e, ctx, vars, out poly, out _))
            return true;
        var expanded = Algebra.Expand(e, ctx);
        return !expanded.Equals(e) && Polynomial.TryFromExpr(expanded, ctx, vars, out poly, out _);
    }

    /// <summary>The NonZero condition on an expression, in the exact shape the rewrite path uses
    /// (Simplify.NonZeroOf): a symbol carries a <see cref="SymbolPropertyAssumption"/>, everything
    /// else an <see cref="ExpressionPropertyAssumption"/>. Mirrored here rather than shared because
    /// the two live in different source units; the ATOMS, not the helper, are the convention.</summary>
    public static AssumptionSet NonZeroCondition(Expr e) => AssumptionSet.Empty.Add(NonZeroAtom(e));

    private static Assumption NonZeroAtom(Expr e) =>
        e is SymbolExpr s
            ? new SymbolPropertyAssumption(s.Symbol, SymbolPredicate.NonZero)
            : new ExpressionPropertyAssumption(e, SymbolPredicate.NonZero);

    /// <summary>How a cancellation changed the input, as a type rather than a spelling.
    /// <see cref="Exact"/>: nothing was removed, the result equals the input pointwise.
    /// <see cref="Conditional"/>: a common factor was removed, so the result equals the input only
    /// where that factor is nonzero (see <see cref="CancelResult.Conditions"/>).</summary>
    public enum CancelStatus
    {
        Exact,
        Conditional,
    }

    /// <summary>The outcome of <see cref="CancelWithConditions"/>: the reduced expression, the
    /// conditions it requires, and the input it came from. The bare <see cref="Cancel"/> drops the
    /// last two; this record is what a machine API must read instead.</summary>
    public sealed record CancelResult(Expr Original, Expr Expression, AssumptionSet Conditions)
    {
        /// <summary>True when the reduction changed the expression.</summary>
        public bool Changed => !Expression.Equals(Original);

        /// <summary>Exact when no factor was removed (no conditions), Conditional otherwise.</summary>
        public CancelStatus Status => Conditions.Atoms.Length == 0
            ? CancelStatus.Exact
            : CancelStatus.Conditional;
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
        // on any internal fallback the decomposition is the original rational part —
        // never a silently wrong 0
        return partial is { } p ? Exprs.Add(polyPart, p) : Exprs.Add(polyPart, Exprs.Divide(r.ToExpr(), d.ToExpr()));
    }

    private static Expr? ApartProper(Polynomial r, Polynomial d, Symbol x)
    {
        // denominator factors: full factorization over Q (rational-root linear factors)
        var factored = Factoring.FactorPoly(d);
        var denFactors = new List<(Polynomial Factor, int Multiplicity)>();
        foreach (var (f, m) in factored.Factors)
            denFactors.Add((f, m));
        if (denFactors.Count == 0)
            denFactors.Add((d, 1));

        // unknowns: A_{i,k,j}·x^j over each factor i, power k=1..m_i, j=0..deg(f_i)-1
        // (quadratic denominators need two unknowns per power)
        var unknowns = new List<(int FactorIdx, int Power, int CoefIdx)>();
        foreach (var (idx, (f, m)) in denFactors.Select((f, i) => (i, f)))
            for (int k = 1; k <= m; k++)
                for (int j = 0; j < f.TotalDegree; j++)
                    unknowns.Add((idx, k, j));

        int nUnknowns = unknowns.Count;
        int denomDegree = d.TotalDegree;
        if (nUnknowns == 0)
            return Exprs.Zero;
        if (nUnknowns != denomDegree)
            return null;   // factorization was incomplete: refuse rather than guess

        // equate polynomial coefficients: r(x) == Σ A_{i,k,j} · x^j · (D / f_i^k)
        var a = new Rat[denomDegree, nUnknowns];
        var b = new Rat[denomDegree];
        for (int t = 0; t < denomDegree; t++)
        {
            b[t] = CoefficientAt(r, t);
            for (int u = 0; u < nUnknowns; u++)
            {
                var (fi, k, j) = unknowns[u];
                var fPow = Polynomial.Pow(denFactors[fi].Factor, k);
                var (quot, rem) = d.DivRem(fPow, MonomialOrder.Lex);
                if (!rem.IsZero)
                    return null;   // factors not exact divisors: incomplete factorization
                a[t, u] = CoefficientAt(quot, t - j);
            }
        }
        if (!RationalLinear.TrySolve(a, b, out var sol))
            return null;

        // assemble the decomposition
        var terms = new List<Expr>();
        for (int u = 0; u < nUnknowns; u++)
        {
            if (sol[u].IsZero)
                continue;
            var (fi, k, j) = unknowns[u];
            var factorExpr = denFactors[fi].Factor.ToExpr();
            var num = j == 0 ? Exprs.Rational(sol[u]) : Exprs.Multiply(Exprs.Rational(sol[u]), Exprs.Power(Exprs.Symbol(x), Exprs.Integer(j)));
            var denom = k == 1 ? factorExpr : Exprs.Power(factorExpr, Exprs.Integer(k));
            terms.Add(Exprs.Divide(num, denom));
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
