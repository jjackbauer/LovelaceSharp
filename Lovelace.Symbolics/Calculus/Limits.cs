using Rat = global::Lovelace.Rational.Rational;
using Int = global::Lovelace.Integer.Integer;

namespace Lovelace.Symbolics;

public enum LimitStatus { Value, PlusInfinity, MinusInfinity, DoesNotExist, Unevaluated, Failed }

public enum LimitDirection { TwoSided, FromLeft, FromRight }

/// <summary>
/// Exactness of the ANSWER a limit record carries — never of a value it does not carry. A record
/// the engine did not determine (<see cref="LimitStatus.Unevaluated"/>, <see cref="LimitStatus.Failed"/>)
/// carries <see cref="None"/>: there is no result for an exactness to describe. The members a
/// determined record can carry keep the names the frozen <c>SolutionExactness</c> vocabulary
/// already publishes on the wire, so every previous answer is spelled exactly as before.
/// </summary>
public enum LimitExactness
{
    /// <summary>No answer at all: no value, no side, nothing an exactness could describe.</summary>
    None,
    /// <summary>An exact constant value (integer or rational).</summary>
    Exact,
    /// <summary>An exact closed form that is not a rational (e.g. e, e^2), or an exact +-infinity
    /// determination.</summary>
    AlgebraicExact,
    /// <summary>A numeric approximation produced by the evaluator.</summary>
    Approximate,
}

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

    /// <summary>
    /// Exactness of the answer this record carries, DERIVED from what it carries instead of
    /// defaulted at construction. The previous default (<c>Exact</c>, inherited by the
    /// <see cref="Uneval"/> / <see cref="Dne"/> / infinity constructors) made a record with no
    /// value, no side and <c>exists: Null</c> claim an exactness about no result — a claim nothing
    /// in the record backstops.
    /// <para>A determined value reports its own exactness; a divergence is an exact +-infinity
    /// determination; a <see cref="LimitStatus.DoesNotExist"/> determination reports the exactness
    /// of the two one-sided results that prove the disagreement (they ARE the content of that
    /// record, so an approximate side makes the determination approximate); everything the engine
    /// did not determine reports <see cref="LimitExactness.None"/>.</para>
    /// </summary>
    public LimitExactness Exactness => Status switch
    {
        LimitStatus.Value => ExactnessOf(Value!),
        LimitStatus.PlusInfinity or LimitStatus.MinusInfinity => LimitExactness.Exact,
        LimitStatus.DoesNotExist => LeastExact(FromLeft, FromRight),
        _ => LimitExactness.None,   // Unevaluated / Failed: the record carries no answer
    };

    public static LimitResult Of(Expr v) => new(LimitStatus.Value, v);

    public static LimitResult PlusInf => new(LimitStatus.PlusInfinity);
    public static LimitResult MinusInf => new(LimitStatus.MinusInfinity);
    public static LimitResult Dne(LimitResult left, LimitResult right) =>
        new(LimitStatus.DoesNotExist, FromLeft: left, FromRight: right);
    public static LimitResult Uneval(string reason) => new(LimitStatus.Unevaluated, FailureReason: reason);

    private static LimitExactness ExactnessOf(Expr v) => v switch
    {
        IntegerConstantExpr or RationalConstantExpr => LimitExactness.Exact,
        RealConstantExpr => LimitExactness.Approximate,
        _ => LimitExactness.AlgebraicExact,
    };

    /// <summary>The strongest exactness both sides of a DoesNotExist determination support: the two
    /// sides are that record's content, so the less exact of them is what it can claim, and a side
    /// with no answer at all leaves nothing to claim.</summary>
    private static LimitExactness LeastExact(LimitResult? left, LimitResult? right)
    {
        if (left is null || right is null)
            return LimitExactness.None;
        LimitExactness a = left.Exactness;
        LimitExactness b = right.Exactness;
        if (a == LimitExactness.None || b == LimitExactness.None)
            return LimitExactness.None;
        if (a == LimitExactness.Approximate || b == LimitExactness.Approximate)
            return LimitExactness.Approximate;
        if (a == LimitExactness.AlgebraicExact || b == LimitExactness.AlgebraicExact)
            return LimitExactness.AlgebraicExact;
        return LimitExactness.Exact;
    }
}

/// <summary>
/// Symbolic limits: direct substitution, exact rational-infinity degree comparison, local
/// series analysis, and the indeterminate exponential family. Pole directions come from the local
/// leading-order behavior — denominator multiplicity parity plus the leading coefficient ratio sign
/// — never from the numerator sign alone. One-sided limits are first-class; a two-sided limit whose
/// sides disagree is reported as DoesNotExist with both one-sided results attached.
/// <para>
/// Direct substitution is a proof only when every subexpression HAS a value at the point. A power
/// whose exponent is singular there does not: <c>(1 + u)^v</c> with <c>u → 0</c> and
/// <c>v → ∞</c> is the standard indeterminate form, and the canonicaliser folds it to a number
/// before anyone can object (<c>Power(1, 1/0)</c> is <c>1</c>), so substituting first used to
/// publish <c>1</c> for <c>limit((1 + 1/x)^x, x, ∞)</c> — whose value is <c>e</c> — with
/// <c>exists: true</c> and an exactness claim attached. Such a limit is now answered exactly, as
/// <c>e^(lim u·v)</c>, or refused with a typed <see cref="LimitStatus.Unevaluated"/>; it is never
/// answered with the value at a point where the expression has none.
/// </para>
/// </summary>
public static class Limits
{
    public static LimitResult Limit(Expr f, Symbol x, Expr point, LimitDirection direction, ExprContext? ctx = null) =>
        LimitCore(f, x, point, direction, ctx ?? Exprs.Current);

    private static LimitResult LimitCore(Expr f, Symbol x, Expr point, LimitDirection direction, ExprContext ctx)
    {
        // infinite limits: rational functions by degree comparison, else t = 1/x substitution
        if (point is NamedConstantExpr { Constant: NamedConstant.Infinity })
            return AtInfinity(f, x, plus: true, direction, ctx);
        if (IsNegInfinity(point))
            return AtInfinity(f, x, plus: false, direction, ctx);

        // 0. an exponent with no value at the point. Substitution folds such a power to a number
        // regardless — Power(1, 1/0) is 1, and (b^e)^0 is 1 whatever b^e was doing — so the number
        // it computes is the value at a point where the expression has none, not a limit. Decide
        // the shape here, before anything can fold it.
        if (HasSingularExponent(f, x, point, ctx))
            return IndeterminateExponential(f, x, point, direction, ctx);

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

    /// <summary>
    /// <paramref name="f"/> has an exponent with no value at the point, so its substituted value is
    /// not a proof. When f is a product (or quotient) of the indeterminate exponential family
    /// <c>(1 + u)^v</c> with <c>u → 0</c> and a rest whose own limit exists, the product rule
    /// decides it; every other shape is refused with a typed non-answer. A finite wrong answer here
    /// would carry <c>exists: true</c> and an exactness claim, which is worse than the refusal.
    /// </summary>
    private static LimitResult IndeterminateExponential(Expr f, Symbol x, Expr point, LimitDirection direction, ExprContext ctx)
    {
        if (!TryIsolateExponentialFactor(f, x, point, ctx, out Expr rest, out PowerExpr? factor, out bool inDenominator))
            return LimitResult.Uneval(
                "a subexpression carries an exponent with no value at the point: the substituted value is not the limit");

        LimitResult? factorLimit = ExponentialFamily(factor!, x, point, ctx);
        if (factorLimit is null)
            return LimitResult.Uneval(
                "an exponent has no value at the point and the base does not substitute to 1, so the limit is not e^(lim u*v)");

        LimitResult restLimit = LimitCore(rest, x, point, direction, ctx);
        return Compose(restLimit, factorLimit, inDenominator);
    }

    /// <summary>
    /// Splits f as <c>R·P</c> (or <c>R/P</c>) where P is THE factor whose exponent has no value at
    /// the point, and R is everything else. False when f is not such a product: a SUM containing
    /// the shape (e.g. <c>(1 + x)^(1/x) − 1</c>) offers no product rule, a nested one
    /// (<c>(2^(1/x))^x</c>) is not a factor of its own, and two indeterminate factors at once are
    /// past what this rewrite can justify.
    /// </summary>
    private static bool TryIsolateExponentialFactor(
        Expr f, Symbol x, Expr point, ExprContext ctx, out Expr rest, out PowerExpr? factor, out bool inDenominator)
    {
        var (n, d) = SplitFraction(f);
        IReadOnlyList<Expr> numerator = n is MultiplyExpr nm ? nm.Factors : new Expr[] { n };
        IReadOnlyList<Expr> denominator = d is MultiplyExpr dm ? dm.Factors : new Expr[] { d };
        int nIndex = -1;
        int dIndex = -1;
        for (int i = 0; i < numerator.Count; i++)
        {
            if (!HasSingularExponent(numerator[i], x, point, ctx))
                continue;
            if (nIndex >= 0 || dIndex >= 0)
                return No(out rest, out factor, out inDenominator);
            nIndex = i;
        }
        for (int i = 0; i < denominator.Count; i++)
        {
            if (!HasSingularExponent(denominator[i], x, point, ctx))
                continue;
            if (nIndex >= 0 || dIndex >= 0)
                return No(out rest, out factor, out inDenominator);
            dIndex = i;
        }
        if (nIndex >= 0 && numerator[nIndex] is PowerExpr np)
        {
            rest = Exprs.Divide(Product(numerator, nIndex), d);
            factor = np;
            inDenominator = false;
            return true;
        }
        if (dIndex >= 0 && denominator[dIndex] is PowerExpr dp)
        {
            rest = Exprs.Divide(n, Product(denominator, dIndex));
            factor = dp;
            inDenominator = true;
            return true;
        }
        return No(out rest, out factor, out inDenominator);

        static bool No(out Expr rest, out PowerExpr? factor, out bool inDenominator)
        {
            rest = Exprs.One;
            factor = null;
            inDenominator = false;
            return false;
        }
    }

    private static Expr Product(IReadOnlyList<Expr> factors, int skip)
    {
        var kept = new List<Expr>(factors.Count);
        for (int i = 0; i < factors.Count; i++)
        {
            if (i != skip)
                kept.Add(factors[i]);
        }
        return kept.Count == 0 ? Exprs.One : Exprs.Multiply(kept);
    }

    /// <summary>
    /// The indeterminate exponential family: <c>P = (1 + u)^v</c> with <c>u → 0</c> at the point,
    /// whose value there does not exist. Since <c>(1 + u)^v = exp(v·ln(1 + u))</c> and
    /// <c>v·ln(1 + u) = (u·v)·(ln(1 + u)/u)</c> with <c>ln(1 + u)/u → 1</c> as <c>u → 0</c>, the
    /// limit is exactly <c>e^(lim u·v)</c> — the kernel never has to expand a logarithm. For
    /// <c>u = a/x</c> and <c>v = b·x</c> that is <c>e^(a·b)</c>, the closed form SymPy returns.
    /// <para>Both operands are decided by EXACT ALGEBRA and nothing else: u must substitute to 0
    /// (so the base is 1 at the point) and u·v must CANCEL to a form free of x, which makes it an
    /// exact constant whose limit is itself. The answer is therefore an exact closed form, never a
    /// float. An un-cancelled quotient is never substituted — 0/0 folds to 0 there, and removing
    /// the common factor is the one step that makes a value at the point meaningful — and the series
    /// machinery is never handed these operands either (a composite exponential is exactly the shape
    /// it can explode on, observed as a stack overflow on <c>(2^(1/x))^x</c>). A shape this test
    /// cannot decide is refused by the caller, not guessed.</para>
    /// <para>Returns null when the base does not substitute to 1 at the point: then P is some other
    /// power (e.g. <c>(2 + x)^(1/x)</c>) and the caller must refuse rather than guess.</para>
    /// </summary>
    private static LimitResult? ExponentialFamily(PowerExpr power, Symbol x, Expr point, ExprContext ctx)
    {
        Expr u = Exprs.Subtract(power.Base, Exprs.One);
        if (SubstituteToNum(u, x, point, ctx) is not { } uValue || !NumOps.IsZero(uValue))
            return null;

        // u = a/x and v = b·x cancel to the constant a·b — the exact exponent of e — while the
        // operands of a shape that does not cancel (u·v still mentions x) say nothing.
        Expr product = RationalFunctions.Cancel(Exprs.Multiply(u, power.Exponent), ctx);
        if (Mentions(product, x))
            return LimitResult.Uneval("(1 + u)^v with u -> 0, but the product u*v does not reduce to a constant");
        return LimitResult.Of(Exprs.Power(Exprs.E, product));
    }

    /// <summary>
    /// The product rule for the isolated factor: <c>lim R·P = (lim R)·(lim P)</c> and
    /// <c>lim R/P = (lim R)/(lim P)</c> when both limits exist. A rest that was not determined, or
    /// a zero divisor, is refused — the rewrite does not turn an undetermined product into a claim.
    /// </summary>
    private static LimitResult Compose(LimitResult rest, LimitResult factor, bool inDenominator)
    {
        if (rest.Status != LimitStatus.Value || factor.Status != LimitStatus.Value)
            return LimitResult.Uneval("the indeterminate factor's cofactor has no limit at the point");
        if (!inDenominator)
            return LimitResult.Of(Exprs.Multiply(rest.Value!, factor.Value!));
        if (IsZeroConstant(factor.Value!))
            return LimitResult.Uneval("the indeterminate factor is 0 in the denominator: 0/0 is not decided by this rewrite");
        return LimitResult.Of(Exprs.Divide(rest.Value!, factor.Value!));
    }

    /// <summary>
    /// True when <paramref name="e"/> contains a power whose EXPONENT mentions x and has no finite
    /// value at the point (a pole, a branch point, or another free parameter). Substitution folds
    /// such a power to a number before anyone can object — <c>Power(1, 1/0)</c> is <c>1</c> by the
    /// canonicaliser, and <c>(b^e)^0</c> is <c>1</c> whatever <c>b^e</c> was doing — so a finite
    /// quotient computed from it is the value at a point where the expression has none.
    /// </summary>
    private static bool HasSingularExponent(Expr e, Symbol x, Expr point, ExprContext ctx) => e switch
    {
        PowerExpr p =>
            (Mentions(p.Exponent, x) && !IsFiniteAtPoint(p.Exponent, x, point, ctx))
            || HasSingularExponent(p.Base, x, point, ctx)
            || HasSingularExponent(p.Exponent, x, point, ctx),
        AddExpr a => a.Terms.Any(t => HasSingularExponent(t, x, point, ctx)),
        MultiplyExpr m => m.Factors.Any(t => HasSingularExponent(t, x, point, ctx)),
        FunctionExpr f => f.Arguments.Any(t => HasSingularExponent(t, x, point, ctx)),
        PiecewiseExpr pw =>
            pw.Branches.Any(b => HasSingularExponent(b.Guard, x, point, ctx) || HasSingularExponent(b.Value, x, point, ctx))
            || HasSingularExponent(pw.Otherwise, x, point, ctx),
        RelationExpr r => HasSingularExponent(r.Left, x, point, ctx) || HasSingularExponent(r.Right, x, point, ctx),
        DerivativeExpr d => HasSingularExponent(d.Operand, x, point, ctx),
        IntegralExpr i => HasSingularExponent(i.Operand, x, point, ctx),
        NotExpr n => HasSingularExponent(n.Operand, x, point, ctx),
        AndExpr an => an.Operands.Any(t => HasSingularExponent(t, x, point, ctx)),
        OrExpr or => or.Operands.Any(t => HasSingularExponent(t, x, point, ctx)),
        _ => false,
    };

    /// <summary>Whether the expression has a finite numeric value at the point. A rejected
    /// substitution (a pole, a branch cut, a free parameter) and a refused numeric evaluation are
    /// both "no value": the exponent's value at the point is what decides whether the power may be
    /// substituted at all.</summary>
    private static bool IsFiniteAtPoint(Expr e, Symbol x, Expr point, ExprContext ctx)
    {
        try
        {
            var substituted = Evaluation.Substitute(e, ctx, new Dictionary<Symbol, Expr> { [x] = point });
            Evaluation.EvaluateToNum(substituted, ctx, new Dictionary<Symbol, Num>());
            return true;
        }
        catch (EvaluationException)
        {
            return false;
        }
    }

    /// <summary>The value of e at the point by direct substitution alone: null when the point is
    /// not numerically substitutable (a pole, a branch cut, an undefined constituent) or when the
    /// denominator vanishes there. The cheap, terminating half of the limit engine — used by the
    /// exponential family, which must never recurse back through the series machinery.</summary>
    private static Num? SubstituteToNum(Expr e, Symbol x, Expr point, ExprContext ctx)
    {
        try
        {
            var (n, d) = SplitFraction(e);
            var nv = Evaluation.EvaluateToNum(
                Evaluation.Substitute(n, ctx, new Dictionary<Symbol, Expr> { [x] = point }), ctx, new Dictionary<Symbol, Num>());
            var dv = Evaluation.EvaluateToNum(
                Evaluation.Substitute(d, ctx, new Dictionary<Symbol, Expr> { [x] = point }), ctx, new Dictionary<Symbol, Num>());
            return NumOps.IsZero(dv) ? null : NumOps.Divide(nv, dv);
        }
        catch (EvaluationException)
        {
            return null;
        }
    }

    /// <summary>True when x occurs anywhere in e (a limit's operands must be re-limited only when
    /// they actually depend on the point variable).</summary>
    private static bool Mentions(Expr e, Symbol x) => e switch
    {
        SymbolExpr s => s.Symbol.Name == x.Name,
        AddExpr a => a.Terms.Any(t => Mentions(t, x)),
        MultiplyExpr m => m.Factors.Any(t => Mentions(t, x)),
        PowerExpr p => Mentions(p.Base, x) || Mentions(p.Exponent, x),
        FunctionExpr f => f.Arguments.Any(t => Mentions(t, x)),
        PiecewiseExpr pw => pw.Branches.Any(b => Mentions(b.Guard, x) || Mentions(b.Value, x)) || Mentions(pw.Otherwise, x),
        RelationExpr r => Mentions(r.Left, x) || Mentions(r.Right, x),
        DerivativeExpr d => Mentions(d.Operand, x),
        IntegralExpr i => Mentions(i.Operand, x),
        OrderExpr o => Mentions(o.Variable, x) || Mentions(o.Point, x) || Mentions(o.Degree, x),
        NotExpr n => Mentions(n.Operand, x),
        AndExpr an => an.Operands.Any(t => Mentions(t, x)),
        OrExpr or => or.Operands.Any(t => Mentions(t, x)),
        _ => false,
    };

    /// <summary>True when the expression is the exact constant zero, the sense in which u → 0 is
    /// required before (1 + u)^v may be read as exp((u·v)·(ln(1 + u)/u)).</summary>
    private static bool IsZeroConstant(Expr e) => Evaluation.ConstantToNum(e) is { } n && NumOps.IsZero(n);

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
        return LimitCore(g, t, Exprs.Zero, plus ? LimitDirection.FromRight : LimitDirection.FromLeft, ctx);
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
