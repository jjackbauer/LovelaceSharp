using Rat = global::Lovelace.Rational.Rational;
using Int = global::Lovelace.Integer.Integer;

namespace Lovelace.Symbolics;

/// <summary>Expand, collect, and coefficient extraction.</summary>
public static class Algebra
{
    /// <summary>Fully expands products of sums (budgeted).</summary>
    public static Expr Expand(Expr e, ExprContext? ctx = null, int maxTerms = 100_000)
    {
        ctx ??= Exprs.Current;
        return ExpandImpl(e, ctx, maxTerms);
    }

    private static Expr ExpandImpl(Expr e, ExprContext ctx, int maxTerms)
    {
        switch (e)
        {
            case AddExpr a:
                return Exprs.Add(a.Terms.Select(t => ExpandImpl(t, ctx, maxTerms)));
            case MultiplyExpr m:
            {
                var factors = m.Factors.Select(f => ExpandImpl(f, ctx, maxTerms)).ToArray();
                Expr result = Exprs.One;
                foreach (var f in factors)
                    result = Distribute(result, f, ctx, maxTerms);
                return result;
            }
            case PowerExpr p:
            {
                var b = ExpandImpl(p.Base, ctx, maxTerms);
                var ex = ExpandImpl(p.Exponent, ctx, maxTerms);
                if (ex is RationalConstantExpr rc && rc.Value.IsInteger && !rc.Value.IsNegative)
                {
                    int n = (int)rc.Value.ToInteger().ToInt64Saturating();
                    Expr result = Exprs.One;
                    var x = b;
                    while (n > 0)
                    {
                        Lovelace.Abstractions.Cancellation.ThrowIfCancellationRequested();
                        if ((n & 1) == 1)
                            result = Distribute(result, x, ctx, maxTerms);
                        x = Distribute(x, x, ctx, maxTerms);
                        n >>= 1;
                    }
                    return result;
                }
                return Exprs.Power(b, ex);
            }
            case FunctionExpr f:
                return Exprs.Function(f.Function, f.Arguments.Select(a => ExpandImpl(a, ctx, maxTerms)).ToArray());
            default:
                return e;
        }
    }

    private static Expr Distribute(Expr a, Expr b, ExprContext ctx, int maxTerms)
    {
        if (a is AddExpr addA && b is AddExpr addB)
        {
            if ((long)addA.Terms.Length * addB.Terms.Length > maxTerms)
                throw new BudgetExceededException("expand");
            var terms = new List<Expr>();
            foreach (var ta in addA.Terms)
                foreach (var tb in addB.Terms)
                    terms.Add(Exprs.Multiply(ta, tb));
            return Exprs.Add(terms);
        }
        if (a is AddExpr addA2)
        {
            if (addA2.Terms.Length > maxTerms)
                throw new BudgetExceededException("expand");
            return Exprs.Add(addA2.Terms.Select(t => Exprs.Multiply(t, b)));
        }
        if (b is AddExpr addB2)
        {
            if (addB2.Terms.Length > maxTerms)
                throw new BudgetExceededException("expand");
            return Exprs.Add(addB2.Terms.Select(t => Exprs.Multiply(a, t)));
        }
        return Exprs.Multiply(a, b);
    }

    /// <summary>
    /// Collects terms by power of x. Returns the rewritten expression when every term is a
    /// polynomial in x; otherwise the input unchanged.
    /// </summary>
    public static Expr Collect(Expr e, Symbol x, ExprContext? ctx = null)
    {
        if (!TryCoefficients(e, x, out var coeffs) || coeffs.Count <= 1)
            return e;
        var terms = new List<Expr>();
        for (int k = 0; k < coeffs.Count; k++)
        {
            var c = coeffs[k];
            if (c is RationalConstantExpr rc && rc.Value.IsZero)
                continue;
            terms.Add(k == 0 ? c : Exprs.Multiply(c, Exprs.Power(Exprs.Symbol(x), Exprs.Integer(k))));
        }
        return Exprs.Add(terms);
    }

    /// <summary>
    /// Extracts the coefficients c_k (expressions free of x) such that e = sum c_k x^k.
    /// Returns false when e is not polynomial in x.
    /// </summary>
    public static bool TryCoefficients(Expr e, Symbol x, out List<Expr> coeffs)
    {
        coeffs = new List<Expr>();
        // walk the canonical sum of terms; each term is coef * monomial in x (power k)
        var terms = e is AddExpr add ? add.Terms.ToArray() : new[] { e };
        var byPower = new Dictionary<int, Expr>();
        foreach (var term in terms)
        {
            if (!TrySplitTermPower(term, x, out var k, out var rest))
                return false;
            byPower.TryGetValue(k, out var prev);
            byPower[k] = prev is null ? rest : Exprs.Add(prev, rest);
        }
        if (byPower.Count == 0)
        {
            coeffs.Add(Exprs.Zero);
            return true;
        }
        int maxK = byPower.Keys.Max();
        for (int k = 0; k <= maxK; k++)
            coeffs.Add(byPower.TryGetValue(k, out var c) ? c : Exprs.Zero);
        return true;
    }

    /// <summary>Splits a canonical term into (power of x, remainder free of x); false when not a monomial in x.</summary>
    private static bool TrySplitTermPower(Expr term, Symbol x, out int power, out Expr rest)
    {
        power = 0;
        rest = Exprs.One;
        var factors = new List<Expr>();
        void AddFactor(Expr f)
        {
            if (f is RationalConstantExpr rc && rc.Value.IsOne)
                return;
            factors.Add(f);
        }

        if (term is PowerExpr p && p.Base is SymbolExpr sb && sb.Symbol.Name == x.Name)
        {
            if (p.Exponent is RationalConstantExpr pr && pr.Value.IsInteger && !pr.Value.IsNegative)
            {
                power = (int)pr.Value.ToInteger().ToInt64Saturating();
                rest = Exprs.One;
                return true;
            }
            rest = term;
            return false;
        }
        if (term is SymbolExpr s && s.Symbol.Name == x.Name)
        {
            power = 1;
            rest = Exprs.One;
            return true;
        }

        if (term is MultiplyExpr m)
        {
            foreach (var f in m.Factors)
            {
                if (f is PowerExpr fp && fp.Base is SymbolExpr fsb && fsb.Symbol.Name == x.Name)
                {
                    if (fp.Exponent is RationalConstantExpr fpr && fpr.Value.IsInteger && !fpr.Value.IsNegative)
                    {
                        power += (int)fpr.Value.ToInteger().ToInt64Saturating();
                        continue;
                    }
                    rest = term;
                    return false;
                }
                if (f is SymbolExpr fs && fs.Symbol.Name == x.Name)
                {
                    power += 1;
                    continue;
                }
                AddFactor(f);
            }
            rest = factors.Count == 0 ? Exprs.One : factors.Count == 1 ? factors[0] : Exprs.Multiply(factors);
            return true;
        }

        // constant term: only when the term does not mention x at all
        if (!Calculus.FreeOf(term, x))
        {
            rest = term;
            return false;
        }
        rest = term;
        return true;
    }

    /// <summary>Horner form of a polynomial in x (coefficients may be arbitrary expressions free of x).</summary>
    public static Expr Horner(Expr e, Symbol x)
    {
        if (!TryCoefficients(e, x, out var coeffs) || coeffs.Count <= 1)
            return e;
        Expr result = coeffs[^1];
        for (int k = coeffs.Count - 2; k >= 0; k--)
            result = Exprs.Add(coeffs[k], Exprs.Multiply(Exprs.Symbol(x), result));
        return result;
    }
}

/// <summary>Thrown when an expansion or conversion exceeds its configured budget.</summary>
public sealed class BudgetExceededException : Exception
{
    public BudgetExceededException(string operation)
        : base($"Budget exceeded during '{operation}'.") { }
}
