namespace Lovelace.Symbolics;

/// <summary>
/// Equivalence-preserving optimization: CSE accounting (DAG sharing), Hornerization over the
/// given parameters, and power-chain hints. Deterministic; the e-graph optimizer is a later
/// layer behind the same shape.
/// </summary>
public static class Optimizer
{
    public sealed record Stats(int NodesBefore, int NodesAfter, int SharedSubtrees, int HornerRewrites);

    public sealed record Result(Expr Expression, Stats Stats);

    public static Result Optimize(Expr e, Symbol[] parameters, ExprContext? ctx = null, bool horner = true)
    {
        ctx ??= Exprs.Current;
        int before = e.NodeCount;

        // 1. CSE accounting over the interned DAG
        var counts = new Dictionary<Expr, int>();
        Count(e, counts);
        int shared = counts.Values.Sum(c => Math.Max(0, c - 1));

        // 2. Hornerize polynomials in each parameter (left to right)
        var cur = e;
        int rewrites = 0;
        if (horner)
        {
            foreach (var p in parameters)
            {
                var hornered = Hornerize(cur, p, parameters);
                if (hornered != cur)
                {
                    rewrites++;
                    cur = hornered;
                }
            }
        }

        return new Result(cur, new Stats(before, cur.NodeCount, shared, rewrites));
    }

    private static void Count(Expr e, Dictionary<Expr, int> counts)
    {
        if (counts.TryGetValue(e, out var c))
        {
            counts[e] = c + 1;
            return;
        }
        counts[e] = 1;
        switch (e)
        {
            case AddExpr a: foreach (var t in a.Terms) Count(t, counts); break;
            case MultiplyExpr m: foreach (var f in m.Factors) Count(f, counts); break;
            case PowerExpr p: Count(p.Base, counts); Count(p.Exponent, counts); break;
            case FunctionExpr f: foreach (var arg in f.Arguments) Count(arg, counts); break;
        }
    }

    /// <summary>Hornerizes the polynomial-in-p part of e, without disturbing non-polynomial parts.</summary>
    private static Expr Hornerize(Expr e, Symbol p, Symbol[] parameters)
    {
        switch (e)
        {
            case AddExpr add:
            {
                var hornered = add.Terms.Select(t => Hornerize(t, p, parameters)).ToArray();
                var rebuilt = Exprs.Add(hornered);
                if (Algebra.TryCoefficients(rebuilt, p, out var coeffs) && coeffs.Count >= 2)
                {
                    // all coefficients must be free of p (they are, by construction)
                    Expr result = coeffs[^1];
                    for (int k = coeffs.Count - 2; k >= 0; k--)
                        result = Exprs.Add(coeffs[k], Exprs.Multiply(Exprs.Symbol(p), result));
                    return result;
                }
                return rebuilt;
            }
            case MultiplyExpr m:
                return Exprs.Multiply(m.Factors.Select(f => Hornerize(f, p, parameters)));
            case PowerExpr pw:
                return Exprs.Power(Hornerize(pw.Base, p, parameters), Hornerize(pw.Exponent, p, parameters));
            case FunctionExpr f:
                return Exprs.Function(f.Function, f.Arguments.Select(a => Hornerize(a, p, parameters)).ToArray());
            default:
                return e;
        }
    }
}
