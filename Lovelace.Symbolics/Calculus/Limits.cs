using Rat = global::Lovelace.Rational.Rational;
using Int = global::Lovelace.Integer.Integer;

namespace Lovelace.Symbolics;

public enum LimitStatus { Value, PlusInfinity, MinusInfinity, Unevaluated, Failed }

public enum LimitDirection { TwoSided, FromLeft, FromRight }

public sealed record LimitResult(
    LimitStatus Status,
    Expr? Value = null,
    LimitResult? FromLeft = null,
    LimitResult? FromRight = null,
    string? FailureReason = null)
{
    public static LimitResult Of(Expr v) => new(LimitStatus.Value, v);
    public static LimitResult PlusInf => new(LimitStatus.PlusInfinity);
    public static LimitResult MinusInf => new(LimitStatus.MinusInfinity);
    public static LimitResult Uneval(string reason) => new(LimitStatus.Unevaluated, FailureReason: reason);
}

/// <summary>Symbolic limits via direct substitution, rational cancellation, and series.</summary>
public static class Limits
{
    public static LimitResult Limit(Expr f, Symbol x, Expr point, LimitDirection direction, ExprContext? ctx = null)
    {
        ctx ??= Exprs.Current;

        // infinite limit: rational functions by degree comparison, else t = 1/x
        if (point is NamedConstantExpr { Constant: NamedConstant.Infinity })
        {
            var rational = RationalInfinity(f, x, ctx);
            if (rational is not null)
                return rational;
            var t = ctx.Symbol("__t");
            var g = Evaluation.Substitute(f, ctx, new Dictionary<Symbol, Expr> { [x] = Exprs.Divide(Exprs.One, Exprs.Symbol(t)) });
            return Limit(g, t, Exprs.Zero, LimitDirection.FromRight, ctx);
        }

        // 1. direct substitution (numerator/denominator separately: 0/0 must not fold to 0)
        try
        {
            var (n, d) = SplitFraction(f);
            var nSub = Evaluation.Substitute(n, ctx, new Dictionary<Symbol, Expr> { [x] = point });
            var dSub = Evaluation.Substitute(d, ctx, new Dictionary<Symbol, Expr> { [x] = point });
            var nv = Evaluation.EvaluateToNum(nSub, ctx, new Dictionary<Symbol, Num>());
            var dv = Evaluation.EvaluateToNum(dSub, ctx, new Dictionary<Symbol, Num>());
            if (NumOps.IsZero(dv))
            {
                if (NumOps.IsZero(nv))
                    throw new InvalidOperationException("0/0 indeterminate form.");
                // nonzero / zero: infinite, sign from numerator and direction
                bool pos = NumOps.Compare(nv, new NumInt(new Int(0L))) > 0;
                return pos ? LimitResult.PlusInf : LimitResult.MinusInf;
            }
            return LimitResult.Of(Evaluation.NumToExpr(NumOps.Divide(nv, dv)));
        }
        catch (Exception)
        {
            // fall through to series
        }

        // 2. series at the point: numerator and denominator separately, then divide
        //    (canonical 0/0 -> 0 folding would destroy the information otherwise)
        try
        {
            Series s;
            var (n, d) = SplitFraction(f);
            if (d is RationalConstantExpr { Value.IsOne: true })
            {
                s = Series.Of(f, x, point, 8, ctx);
            }
            else
            {
                var sn = Series.Of(n, x, point, 8, ctx);
                var sd = Series.Of(d, x, point, 8, ctx);
                s = Series.Divide(sn, sd);
            }
            int lead = s.LeadingIndex();
            if (lead >= 0)
            {
                var c = s.Coefficients[lead];
                if (lead == 0)
                {
                    // the coefficient must be an actual constant, not a degenerate
                    // expression that failed to evaluate at the point
                    if (Evaluation.ConstantToNum(c) is null)
                        return LimitResult.Uneval("coefficient does not evaluate at the point");
                    return LimitResult.Of(c);
                }
                if (lead > 0)
                    return LimitResult.Of(Exprs.Zero);
                // negative leading order: infinite; the coefficient sign decides the side
                try
                {
                    var num = Evaluation.EvaluateToNum(c, ctx, new Dictionary<Symbol, Num>());
                    bool positive = NumOps.Compare(num, new NumInt(new Int(0L))) > 0;
                    return positive ? LimitResult.PlusInf : LimitResult.MinusInf;
                }
                catch (Exception)
                {
                    return LimitResult.Uneval("leading coefficient is symbolic");
                }
            }
            return LimitResult.Of(Exprs.Zero);
        }
        catch (Exception ex)
        {
            return LimitResult.Uneval(ex.Message);
        }
    }

    /// <summary>Limit of a rational function as x → +∞ by degree comparison.</summary>
    private static LimitResult? RationalInfinity(Expr f, Symbol x, ExprContext ctx)
    {
        var (n, d) = SplitFraction(f);
        if (!Polynomial.TryFromExpr(n, ctx, new[] { x }, out var nP, out _) ||
            !Polynomial.TryFromExpr(d, ctx, new[] { x }, out var dP, out _))
            return null;
        int degN = nP.TotalDegree;
        int degD = dP.TotalDegree;
        if (degN < degD)
            return LimitResult.Of(Exprs.Zero);
        var ln = nP.LeadingCoefficient(MonomialOrder.Lex);
        var ld = dP.LeadingCoefficient(MonomialOrder.Lex);
        if (degN > degD)
        {
            var ratio = ln / ld;
            return ratio.IsNegative ? LimitResult.MinusInf : LimitResult.PlusInf;
        }
        return LimitResult.Of(Exprs.Rational(ln / ld));
    }

    private static (Expr Numerator, Expr Denominator) SplitFraction(Expr f)
    {
        if (f is PowerExpr bp && bp.Exponent is RationalConstantExpr brc && brc.Value.IsNegative)
            return (Exprs.One, Exprs.Power(bp.Base, Exprs.Rational(Rat.Negate(brc.Value))));
        if (f is MultiplyExpr m)
        {
            var num = new List<Expr>();
            var den = new List<Expr>();
            foreach (var factor in m.Factors)
            {
                if (factor is PowerExpr p && p.Exponent is RationalConstantExpr rc && rc.Value.IsNegative)
                    den.Add(Exprs.Power(p.Base, Exprs.Rational(Rat.Negate(rc.Value))));
                else
                    num.Add(factor);
            }
            var n = num.Count == 0 ? Exprs.One : Exprs.Multiply(num);
            var d = den.Count == 0 ? Exprs.One : Exprs.Multiply(den);
            return (n, d);
        }
        return (f, Exprs.One);
    }
}
