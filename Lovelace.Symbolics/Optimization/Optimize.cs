namespace Lovelace.Symbolics;

/// <summary>
/// Optimization policies. Policies are explicit: Horner reassociation changes evaluation
/// order and therefore floating-point error characteristics — it is OptimizationOnly, never
/// algebraic equality. PrecisionAware forbids reassociating polynomials with approximate
/// (Real) coefficients, whose rounding the reassociation would change.
/// </summary>
public sealed record OptimizeOptions(
    bool Horner = true,
    bool CommonSubexpressionElimination = true,
    bool PrecisionAware = true,
    bool PowerReduction = false);

/// <summary>Structured optimization outcome with before/after costs and the applied policy.</summary>
public sealed record OptimizationResult(
    Expr Expression,
    int NodesBefore,
    int NodesAfter,
    int SharedSubtrees,
    int HornerRewrites,
    OptimizeOptions Options);

/// <summary>
/// Equivalence-preserving optimization: CSE accounting (the interned DAG is already shared;
/// lowering reuses shared nodes), policy-gated Hornerization, and power-chain hints at
/// lowering. Deterministic. The e-graph optimizer is a later layer behind the same shape.
/// </summary>
public static class Optimizer
{
    public sealed record Stats(int NodesBefore, int NodesAfter, int SharedSubtrees, int HornerRewrites);

    public sealed record Result(Expr Expression, Stats Stats);

    public static Result Optimize(Expr e, Symbol[] parameters, ExprContext? ctx = null, bool horner = true)
    {
        var result = OptimizeDetailed(e, parameters, ctx, new OptimizeOptions(Horner: horner));
        return new Result(result.Expression, new Stats(
            result.NodesBefore, result.NodesAfter, result.SharedSubtrees, result.HornerRewrites));
    }

    public static OptimizationResult OptimizeDetailed(
        Expr e, Symbol[] parameters, ExprContext? ctx = null, OptimizeOptions? options = null)
    {
        ctx ??= Exprs.Current;
        options ??= new OptimizeOptions();
        int before = e.NodeCount;

        // CSE accounting over the interned DAG
        var counts = new Dictionary<Expr, int>();
        Count(e, counts);
        int shared = counts.Values.Sum(c => Math.Max(0, c - 1));

        var cur = e;
        int rewrites = 0;
        if (options.Horner)
        {
            foreach (var p in parameters)
            {
                var hornered = Hornerize(cur, p, parameters, options.PrecisionAware);
                if (!hornered.Equals(cur))
                {
                    rewrites++;
                    cur = hornered;
                }
            }
        }

        return new OptimizationResult(cur, before, cur.NodeCount, shared, rewrites, options);
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
    private static Expr Hornerize(Expr e, Symbol p, Symbol[] parameters, bool precisionAware)
    {
        switch (e)
        {
            case AddExpr add:
            {
                var hornered = add.Terms.Select(t => Hornerize(t, p, parameters, precisionAware)).ToArray();
                var rebuilt = Exprs.Add(hornered);
                if (Algebra.TryCoefficients(rebuilt, p, out var coeffs) && coeffs.Count >= 2)
                {
                    // PrecisionAware: reassociating approximate-coefficient polynomials changes
                    // their rounding — skip unless every coefficient is exact
                    if (precisionAware && coeffs.Any(ContainsApproximate))
                        return rebuilt;
                    Expr result = coeffs[^1];
                    for (int k = coeffs.Count - 2; k >= 0; k--)
                        result = Exprs.Add(coeffs[k], Exprs.Multiply(Exprs.Symbol(p), result));
                    return result;
                }
                return rebuilt;
            }
            case MultiplyExpr m:
                return Exprs.Multiply(m.Factors.Select(f => Hornerize(f, p, parameters, precisionAware)));
            case PowerExpr pw:
                return Exprs.Power(Hornerize(pw.Base, p, parameters, precisionAware), Hornerize(pw.Exponent, p, parameters, precisionAware));
            case FunctionExpr f:
                return Exprs.Function(f.Function, f.Arguments.Select(a => Hornerize(a, p, parameters, precisionAware)).ToArray());
            default:
                return e;
        }
    }

    private static bool ContainsApproximate(Expr e)
    {
        switch (e)
        {
            case RealConstantExpr: return true;
            case AddExpr a: return a.Terms.Any(ContainsApproximate);
            case MultiplyExpr m: return m.Factors.Any(ContainsApproximate);
            case PowerExpr p: return ContainsApproximate(p.Base) || ContainsApproximate(p.Exponent);
            case FunctionExpr f: return f.Arguments.Any(ContainsApproximate);
            default: return false;
        }
    }
}
