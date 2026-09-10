using Rat = global::Lovelace.Rational.Rational;
using Int = global::Lovelace.Integer.Integer;

namespace Lovelace.Symbolics;

public enum LimitStatus { Value, PlusInfinity, MinusInfinity, DoesNotExist, Unevaluated, Failed }

public enum LimitDirection { TwoSided, FromLeft, FromRight }

public sealed record LimitResult(
    LimitStatus Status,
    Expr? Value = null,
    LimitResult? FromLeft = null,
    LimitResult? FromRight = null,
    string? FailureReason = null)
{
    /// <summary>Conditions under which <see cref="Value"/> holds. Usually empty (a limit at a
    /// point is unconditional); a one-sided value at a branch point carries the side constraint.</summary>
    public AssumptionSet Conditions { get; init; } = AssumptionSet.Empty;

    /// <summary>Exactness of the reported value: an exact constant, an algebraic closed form, or
    /// an approximation produced by numeric evaluation.</summary>
    public SolutionExactness Exactness { get; init; } = SolutionExactness.Exact;

    public static LimitResult Of(Expr v) =>
        new(LimitStatus.Value, v) { Exactness = ExactnessOf(v) };

    public static LimitResult PlusInf => new(LimitStatus.PlusInfinity);
    public static LimitResult MinusInf => new(LimitStatus.MinusInfinity);
    public static LimitResult Dne(LimitResult left, LimitResult right) =>
        new(LimitStatus.DoesNotExist, FromLeft: left, FromRight: right);
    public static LimitResult Uneval(string reason) => new(LimitStatus.Unevaluated, FailureReason: reason);

    private static SolutionExactness ExactnessOf(Expr v) => v switch
    {
        IntegerConstantExpr or RationalConstantExpr => SolutionExactness.Exact,
        RealConstantExpr => SolutionExactness.Approximate,
        _ => SolutionExactness.AlgebraicExact,
    };
}

/// <summary>
/// Symbolic limits: direct substitution, exact rational-infinity degree comparison, and local
/// series analysis. Pole directions come from the local leading-order behavior — denominator
/// multiplicity parity plus the leading coefficient ratio sign — never from the numerator sign
/// alone. One-sided limits are first-class; a two-sided limit whose sides disagree is reported
/// as DoesNotExist with both one-sided results attached.
/// </summary>
public static class Limits
{
    public static LimitResult Limit(Expr f, Symbol x, Expr point, LimitDirection direction, ExprContext? ctx = null)
    {
        ctx ??= Exprs.Current;

        // infinite limits: rational functions by degree comparison, else t = 1/x substitution
        if (point is NamedConstantExpr { Constant: NamedConstant.Infinity })
            return AtInfinity(f, x, plus: true, direction, ctx);
        if (IsNegInfinity(point))
            return AtInfinity(f, x, plus: false, direction, ctx);

        // 1. direct substitution (numerator/denominator separately: 0/0 must not fold to 0)
        try
        {
            var (n, d) = SplitFraction(f);
            var nSub = Evaluation.Substitute(n, ctx, new Dictionary<Symbol, Expr> { [x] = point });
            var dSub = Evaluation.Substitute(d, ctx, new Dictionary<Symbol, Expr> { [x] = point });
            var nv = Evaluation.EvaluateToNum(nSub, ctx, new Dictionary<Symbol, Num>());
            var dv = Evaluation.EvaluateToNum(dSub, ctx, new Dictionary<Symbol, Num>());
            if (!NumOps.IsZero(dv))
                return LimitResult.Of(Evaluation.NumToExpr(NumOps.Divide(nv, dv)));
            // nonzero/zero or 0/0: local series decides (pole analysis handles direction)
            return SeriesLimit(f, n, d, x, point, direction, ctx);
        }
        catch (EvaluationException)
        {
            // the point is not numerically substitutable (pole, branch cut, undefined at the
            // point): the series path decides. Defects are not swallowed here.
        }
        try
        {
            var (n, d) = SplitFraction(f);
            return SeriesLimit(f, n, d, x, point, direction, ctx);
        }
        catch (EvaluationException ex)
        {
            // an expected mathematical non-answer (the series could not be evaluated at the
            // point). Anything else is an internal defect and propagates (§76): an internal
            // error must never masquerade as "unevaluated".
            return LimitResult.Uneval(ex.Message);
        }
    }

    private static bool IsNegInfinity(Expr e) =>
        e is MultiplyExpr m && m.Factors.Length == 2 &&
        m.Factors[0] is RationalConstantExpr rc && rc.Value.IsMinusOne &&
        m.Factors[1] is NamedConstantExpr { Constant: NamedConstant.Infinity };

    /// <summary>Limit at ±∞: exact degree comparison for rational functions, else t = 1/x.</summary>
    private static LimitResult AtInfinity(Expr f, Symbol x, bool plus, LimitDirection direction, ExprContext ctx)
    {
        var rational = RationalInfinity(f, x, ctx, plus);
        if (rational is not null)
            return rational;
        var t = ctx.Symbol("__t");
        var g = Evaluation.Substitute(f, ctx, new Dictionary<Symbol, Expr> { [x] = Exprs.Divide(Exprs.One, Exprs.Symbol(t)) });
        // x → +∞ ⟺ t → 0⁺ ; x → −∞ ⟺ t → 0⁻
        return Limit(g, t, Exprs.Zero, plus ? LimitDirection.FromRight : LimitDirection.FromLeft, ctx);
    }

    /// <summary>
    /// Local series analysis at the point. Leading orders of numerator (on) and denominator (od):
    /// od == on → finite value cn/cd; od > on → 0; od &lt; on → pole whose side depends on the
    /// multiplicity parity m = on − od, the leading ratio sign, and the direction.
    /// </summary>
    private static LimitResult SeriesLimit(Expr f, Expr n, Expr d, Symbol x, Expr point, LimitDirection direction, ExprContext ctx)
    {
        bool denIsOne = d is RationalConstantExpr { Value.IsOne: true };
        var sn = Series.Of(denIsOne ? f : n, x, point, 10, ctx);
        var sd = denIsOne ? null : Series.Of(d, x, point, 10, ctx);
        int on = sn.LeadingOrder();
        int od = denIsOne ? 0 : sd!.LeadingOrder();

        // f ≈ cn·(x−a)^on / (cd·(x−a)^od)
        if (od < on)
            return LimitResult.Of(Exprs.Zero);

        if (od == on)
        {
            var cn = sn.Coefficients[sn.LeadingIndex()];
            var cd = denIsOne ? Exprs.One : sd!.Coefficients[sd.LeadingIndex()];
            var ratio = Exprs.Divide(cn, cd);
            if (Evaluation.ConstantToNum(ratio) is null)
                return LimitResult.Uneval("coefficient does not evaluate at the point");
            return LimitResult.Of(ratio);
        }

        // pole of order m = od − on > 0
        int m = od - on;
        var cn2 = sn.Coefficients[sn.LeadingIndex()];
        var cd2 = denIsOne ? Exprs.One : sd!.Coefficients[sd.LeadingIndex()];
        var ratio2 = Exprs.Divide(cn2, cd2);
        Num rv;
        try
        {
            rv = Evaluation.EvaluateToNum(ratio2, ctx, new Dictionary<Symbol, Num>());
        }
        catch (EvaluationException)
        {
            return LimitResult.Uneval("leading coefficient is symbolic");
        }
        bool pos = NumOps.Compare(rv, NumOps.FromLong(0L)) > 0;
        // x → a⁺: sign = sign(ratio); x → a⁻: sign = sign(ratio)·(−1)^m
        LimitResult FromRight() => pos ? LimitResult.PlusInf : LimitResult.MinusInf;
        LimitResult FromLeft() => (m % 2 == 0 ? pos : !pos) ? LimitResult.PlusInf : LimitResult.MinusInf;

        return direction switch
        {
            LimitDirection.FromRight => FromRight(),
            LimitDirection.FromLeft => FromLeft(),
            _ => m % 2 == 1
                ? LimitResult.Dne(FromLeft(), FromRight())   // odd pole: sides disagree
                : FromRight(),                               // even pole: both sides agree
        };
    }

    /// <summary>Limit of a rational function as x → ±∞ by degree comparison.</summary>
    private static LimitResult? RationalInfinity(Expr f, Symbol x, ExprContext ctx, bool plus)
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
            // x → +∞: sign is sign(ratio) regardless of degree parity (x^m → +∞).
            // x → −∞: sign(ratio)·(−1)^m.
            bool positive = plus
                ? !ratio.IsNegative
                : (ratio.IsNegative ? (degN - degD) % 2 == 1 : (degN - degD) % 2 == 0);
            return positive ? LimitResult.PlusInf : LimitResult.MinusInf;
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
