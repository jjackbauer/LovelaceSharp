using Rat = global::Lovelace.Rational.Rational;
using Int = global::Lovelace.Integer.Integer;

namespace Lovelace.Symbolics;

/// <summary>
/// How <see cref="Series.Of"/> treats a KINK at the expansion point: a subexpression whose sign
/// changes there, i.e. an <c>abs(g)</c> whose argument vanishes at the point.
/// </summary>
public enum SeriesKinkPolicy
{
    /// <summary>
    /// Resolve the kink by the sign of the argument's leading coefficient — the RIGHT-HAND
    /// expansion. That is SymPy's rule (<c>Abs._eval_nseries</c> multiplies the argument's series by
    /// <c>sign(arg.leadterm(x)[0])</c>) and SymPy's default direction (<c>dir='+'</c>), which is why
    /// <c>series(Abs(x), x, 0, 3)</c> is <c>x</c> rather than a refusal. This is the published
    /// <c>series()</c> contract.
    /// </summary>
    ResolveByLeadingSign,

    /// <summary>
    /// Leave the derivative node unevaluated. A caller that reasons about TWO-SIDED behaviour must
    /// not adopt a one-sided convention: <c>|x|/x</c> has no two-sided limit at 0 and the right-hand
    /// series would answer 1, so the limit engine keeps the unresolved nodes its leading-order
    /// analysis is written against (see <c>Limits.SeriesLimit</c>).
    /// </summary>
    LeaveUnevaluated,
}

/// <summary>
/// Truncated power series: c0 + c1·(x-x0) + … + c_{n-1}·(x-x0)^{n-1} + O((x-x0)^n).
/// Coefficients are expressions (constants after evaluation at numeric points).
/// </summary>
public sealed class Series
{
    public Symbol Variable { get; }
    public Expr Point { get; }
    public Expr[] Coefficients { get; }
    public int Order => Coefficients.Length;

    /// <summary>Order of the dropped tail: the truncation is O((x − point)^TruncationOrder).</summary>
    public int TruncationOrder => LeadingPower + Order;

    /// <summary>Power of (x − point) multiplying Coefficients[0] (0 for Taylor series;
    /// negative for Laurent tails; positive after dividing by vanishing denominators).</summary>
    public int LeadingPower { get; }

    /// <summary>
    /// True when every coefficient IS a value at the point, so the expansion is a power series —
    /// possibly LAURENT, since a pole is carried by a negative <see cref="LeadingPower"/> and not by
    /// a coefficient (<c>1/x</c> at 0 is <c>1/x + O(x^2)</c>).
    /// <para>
    /// False at a BRANCH POINT, where the coefficients are not values at all:
    /// <c>sqrt(t)</c> differentiates to <c>1/(2·sqrt(t))</c>, which at <c>t = 0</c> is
    /// <c>1/(2·sqrt(0))</c>. It is also false when a coefficient still carries the expansion's own
    /// substitution variable — a derivative that did not evaluate at the point (Round 20, H-1).
    /// <see cref="ToExpression"/> publishes the source function unchanged in that case rather than a
    /// junk expression: SymPy publishes exactly that for these inputs
    /// (<c>series(sqrt(x), x, 0, 2) = sqrt(x)</c>, <c>series(log(x), x, 0, 2) = log(x)</c>), and the
    /// input is never a wrong value.
    /// </para>
    /// </summary>
    public bool IsPowerSeries { get; }

    /// <summary>The function this expansion was built from, after kink resolution. It is published
    /// verbatim when <see cref="IsPowerSeries"/> is false and is otherwise only the record of where
    /// the expansion came from.</summary>
    private readonly Expr? _source;

    public Series(Symbol variable, Expr point, Expr[] coefficients, int leadingPower = 0)
        : this(variable, point, coefficients, leadingPower, source: null, isPowerSeries: true)
    {
    }

    private Series(Symbol variable, Expr point, Expr[] coefficients, int leadingPower, Expr? source, bool isPowerSeries)
    {
        Variable = variable;
        Point = point;
        Coefficients = coefficients;
        LeadingPower = leadingPower;
        _source = source;
        IsPowerSeries = isPowerSeries;
    }

    /// <summary>True leading order of the expansion: LeadingPower + first nonzero coefficient index.</summary>
    public int LeadingOrder() => LeadingPower + LeadingIndex();

    public static Series Of(Expr f, Symbol x, Expr x0, int order, ExprContext? ctx = null,
        SeriesKinkPolicy kinks = SeriesKinkPolicy.ResolveByLeadingSign)
    {
        ctx ??= Exprs.Current;
        if (order < 1)
            throw new ArgumentException(
                $"a series expansion order must be at least 1 (the number of terms to carry, with " +
                $"the O-term); got {order}.");

        // a kink the expansion crosses is resolved BEFORE anything is differentiated: the machinery
        // can only see a derivative at a point where the function has one (Round 20, H-1)
        if (kinks == SeriesKinkPolicy.ResolveByLeadingSign)
            f = ResolveKinks(f, x, x0, order, ctx);

        // fractions: series of numerator and denominator separately, then divide
        // (canonical 0/0 folding would otherwise destroy the information at the point)
        var (fn, fd) = SplitFraction(f);
        if (fd is not RationalConstantExpr { Value.IsOne: true })
        {
            // A QUOTIENT needs both operands expanded PAST their own leading terms. Coefficient k of
            // a/b is (a[la+k] − Σ_{i<k} q_i·b[lb+k−i]) / b[lb], so a leading term that sits beyond
            // the requested order must still be carried by the operands: 1/x^2 has its leading term
            // at index 2 of the denominator (invisible to a 2-coefficient expansion, which is why
            // this used to throw "division by a zero series"), and the x^2 coefficient of sin(x)/x
            // reads sin(x)'s x^3 coefficient (invisible at order 3, which is why the term was
            // silently dropped).
            int la = LeadingIndexWithin(fn, x, x0, order, ctx);
            int lb = LeadingIndexWithin(fd, x, x0, order, ctx);
            var sn = OfNoSplit(fn, x, x0, order + Math.Max(lb, 0), ctx);
            var sd = OfNoSplit(fd, x, x0, order + Math.Max(2 * lb - la, 0), ctx);
            // the TRUNCATION power stays the requested order — SymPy's convention, in which the
            // published tail of 1/x at order 2 is O(x^2) — and the quotient is therefore built down
            // to its own leading power, negative powers included.
            var quotient = Divide(sn, sd, order);
            return new Series(quotient.Variable, quotient.Point, quotient.Coefficients, quotient.LeadingPower,
                source: f, isPowerSeries: quotient.IsPowerSeries);
        }
        return OfNoSplit(fn, x, x0, order, ctx);
    }

    /// <summary>
    /// Rewrites every kink the expansion crosses: an <c>abs(g)</c> is replaced by the branch the
    /// expansion follows, so no derivative is ever taken OF the kink. That is SymPy's rule exactly —
    /// <c>Abs._eval_nseries</c> multiplies the argument's own series by
    /// <c>sign(arg.leadterm(x)[0])</c>, the sign of the argument's leading COEFFICIENT — and it is
    /// why <c>series(abs(x), x, 0, 3)</c> is <c>x + O(x^3)</c>: the expansion is the RIGHT-HAND one,
    /// the side SymPy's default <c>dir='+'</c> names.
    /// <para>
    /// Without the rewrite the kernel differentiates the kink itself, and what it can produce is
    /// <c>piecewise(0 if 0 != 0, diff(0, t))</c> — an expression whose second branch is the
    /// unevaluated derivative of a constant WITH RESPECT TO THE EXPANSION'S OWN SUBSTITUTION
    /// VARIABLE, and whose value at the point denotes 0 while <c>|x|</c> is <c>x</c> there. The
    /// variable was also <c>Guid</c>-named in that path, so the published text changed on every run
    /// (H-1).
    /// </para>
    /// </summary>
    private static Expr ResolveKinks(Expr e, Symbol x, Expr x0, int order, ExprContext ctx)
    {
        switch (e)
        {
            case FunctionExpr f when f.Function.Name == "abs":
            {
                Expr g = ResolveKinks(f.Arguments[0], x, x0, order, ctx);
                Expr? c = LeadingCoefficient(g, x, x0, order, ctx);
                if (c is null)
                    return Exprs.Zero;   // g vanishes to the requested order, so |g| does too
                if (SignOf(c) is { } s)
                    return s > 0 ? g : s < 0 ? Exprs.Negate(g) : Exprs.Zero;
                // The leading coefficient is symbolic (|a·x| at 0: which side the expansion follows
                // depends on the sign of a). The branch is then sign(a)·a·x = |a|·x — the branch
                // SymPy takes — so the coefficient is COMPUTED, never guessed and never refused.
                return Exprs.Multiply(Exprs.Function(ctx.Function("sign"), c), g);
            }
            case AddExpr a:
                return Exprs.Add(a.Terms.Select(t => ResolveKinks(t, x, x0, order, ctx)));
            case MultiplyExpr m:
                return Exprs.Multiply(m.Factors.Select(t => ResolveKinks(t, x, x0, order, ctx)));
            case PowerExpr pw:
                return Exprs.Power(ResolveKinks(pw.Base, x, x0, order, ctx), ResolveKinks(pw.Exponent, x, x0, order, ctx));
            case FunctionExpr fn:
                return Exprs.Function(fn.Function, fn.Arguments.Select(t => ResolveKinks(t, x, x0, order, ctx)).ToArray());
            case RelationExpr r:
                return Exprs.Relation(r.Op, ResolveKinks(r.Left, x, x0, order, ctx), ResolveKinks(r.Right, x, x0, order, ctx));
            case PiecewiseExpr pwx:
                return Exprs.Piecewise(
                    pwx.Branches.Select(b => new PiecewiseBranch(
                        ResolveKinks(b.Guard, x, x0, order, ctx), ResolveKinks(b.Value, x, x0, order, ctx))),
                    ResolveKinks(pwx.Otherwise, x, x0, order, ctx));
            case DerivativeExpr d:
                return Exprs.Derivative(ResolveKinks(d.Operand, x, x0, order, ctx), d.Variables.ToArray());
            case IntegralExpr i:
                return Exprs.Integral(ResolveKinks(i.Operand, x, x0, order, ctx), i.Variables.ToArray());
            case NotExpr n:
                return Exprs.Not(ResolveKinks(n.Operand, x, x0, order, ctx));
            case AndExpr an:
                return Exprs.And(an.Operands.Select(t => ResolveKinks(t, x, x0, order, ctx)));
            case OrExpr or:
                return Exprs.Or(or.Operands.Select(t => ResolveKinks(t, x, x0, order, ctx)));
            default:
                return e;
        }
    }

    /// <summary>The leading (first nonzero) coefficient of g's own series at the point, or null when
    /// that series is zero to the requested order.</summary>
    private static Expr? LeadingCoefficient(Expr g, Symbol x, Expr x0, int order, ExprContext ctx)
    {
        Series s = Of(g, x, x0, Math.Max(order, 2), ctx);
        int i = s.LeadingIndex();
        return i < 0 ? null : s.Coefficients[i];
    }

    /// <summary>The sign of a numeric constant, or null when the expression is not one.</summary>
    private static int? SignOf(Expr e) => e switch
    {
        IntegerConstantExpr i => Int.IsZero(i.Value) ? 0 : (Int.IsNegative(i.Value) ? -1 : 1),
        RationalConstantExpr r => r.Value.Sign,
        _ => null,
    };

    /// <summary>
    /// The expansion's own substitution variable, the fresh <c>t</c> of <c>x = t + x0</c>. It is
    /// INTERNAL — every coefficient is computed by substituting it back out — and it is chosen
    /// deterministically and away from every symbol the input already carries, so that (a) the same
    /// script always meets the same name (Round 20, H-1: this used to be
    /// <c>"__t" + Guid.NewGuid().ToString("N")[..6]</c>, so any coefficient that survived the
    /// substitution printed differently on every run) and (b) the substitution can never capture a
    /// caller's own symbol.
    /// </summary>
    private static Symbol InternalVariable(Expr f, Symbol x, Expr x0, ExprContext ctx)
    {
        string name = "__t";
        for (int i = 1; !Calculus.FreeOf(f, ctx.Symbol(name)) || !Calculus.FreeOf(x0, ctx.Symbol(name)); i++)
            name = "__t" + i.ToString(System.Globalization.CultureInfo.InvariantCulture);
        return ctx.Symbol(name);
    }

    /// <summary>Upper bound on the doubling probe below, so an operand that is identically zero
    /// (whose leading term therefore does not exist at ANY order) cannot spin: past the bound the
    /// index is reported as -1 and the division reports the zero series it truly is.</summary>
    private const int MaxLeadingProbe = 64;

    /// <summary>The first index of g's Maclaurin series in t that carries a non-zero coefficient, or
    /// -1 when the series is zero through the probe bound. The probe DOUBLES because a leading term
    /// can sit beyond the requested order: x² is invisible to a two-coefficient expansion of the
    /// denominator of 1/x², which is why that expansion used to throw instead of answering.</summary>
    private static int LeadingIndexWithin(Expr g, Symbol x, Expr x0, int order, ExprContext ctx)
    {
        int probe = Math.Max(order, 1);
        while (true)
        {
            int index = OfNoSplit(g, x, x0, probe, ctx).LeadingIndex();
            if (index >= 0 || probe >= MaxLeadingProbe)
                return index;
            probe = Math.Min(probe * 2, MaxLeadingProbe);
        }
    }

    /// <summary>True when the expression contains a constant that is not a value at the point: a
    /// NEGATIVE power of a vanishing base — <c>1/0</c>, <c>0^(−1/2)</c>, <c>log(0)</c>'s derivative
    /// <c>1/0</c>. That is what differentiating a BRANCH POINT leaves behind (<c>sqrt(t)</c>'s
    /// derivative is <c>1/(2·sqrt(t))</c>, which at <c>t = 0</c> is <c>1/(2·sqrt(0))</c>), and it must
    /// never be published as a value.</summary>
    private static bool HasUndefinedValue(Expr e) => e switch
    {
        PowerExpr p => (IsNegativeExponent(p.Exponent) && IsZeroConstant(p.Base))
                       || HasUndefinedValue(p.Base) || HasUndefinedValue(p.Exponent),
        AddExpr a => a.Terms.Any(HasUndefinedValue),
        MultiplyExpr m => m.Factors.Any(HasUndefinedValue),
        FunctionExpr f => f.Arguments.Any(HasUndefinedValue),
        PiecewiseExpr pw => pw.Branches.Any(b => HasUndefinedValue(b.Guard) || HasUndefinedValue(b.Value))
                            || HasUndefinedValue(pw.Otherwise),
        RelationExpr r => HasUndefinedValue(r.Left) || HasUndefinedValue(r.Right),
        DerivativeExpr d => HasUndefinedValue(d.Operand),
        IntegralExpr i => HasUndefinedValue(i.Operand),
        NotExpr n => HasUndefinedValue(n.Operand),
        AndExpr an => an.Operands.Any(HasUndefinedValue),
        OrExpr or => or.Operands.Any(HasUndefinedValue),
        _ => false,
    };

    private static bool IsNegativeExponent(Expr e) => e switch
    {
        IntegerConstantExpr i => Int.IsNegative(i.Value),
        RationalConstantExpr r => r.Value.IsNegative,
        _ => false,
    };

    private static bool IsZeroConstant(Expr e) => e switch
    {
        IntegerConstantExpr i => Int.IsZero(i.Value),
        RationalConstantExpr r => r.Value.IsZero,
        _ => false,
    };

    private static Series OfNoSplit(Expr f, Symbol x, Expr x0, int order, ExprContext ctx)
    {
        // substitute x = t + x0 (fresh t), then compute Maclaurin coefficients in t
        var t = InternalVariable(f, x, x0, ctx);
        var shifted = Evaluation.Substitute(f, ctx, new Dictionary<Symbol, Expr> { [x] = Exprs.Add(Exprs.Symbol(t), x0) });

        var coeffs = new Expr[order];
        var bindings = new Dictionary<Symbol, Num> { [t] = new NumInt(new Int(0L)) };
        bool isPowerSeries = true;
        for (int k = 0; k < order; k++)
        {
            var dk = Calculus.DiffN(shifted, t, k, ctx);
            var at0 = Evaluation.Substitute(dk, ctx, new Dictionary<Symbol, Expr> { [t] = Exprs.Zero });
            // numeric evaluation folds constants; keep symbolic parts symbolic
            var evaluated = Evaluation.EvaluateToExpr(at0, ctx, bindings);
            // a coefficient that still mentions t is a derivative that did not evaluate at the
            // point; one that holds a negative power of a vanishing base is a BRANCH POINT's
            // derivative. Neither is a value, so neither may be published (see ToExpression).
            if (!Calculus.FreeOf(evaluated, t) || HasUndefinedValue(evaluated))
                isPowerSeries = false;
            coeffs[k] = Exprs.Divide(evaluated, FactorialExpr(k));
        }
        return new Series(x, x0, coeffs, 0, source: f, isPowerSeries: isPowerSeries);
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

    private static Expr FactorialExpr(int k)
    {
        var n = new Int(1L);
        for (int i = 2; i <= k; i++)
            n = n * new Int(i);
        return Exprs.Integer(n);
    }

    public Expr ToExpression()
    {
        // No power series exists at this point (a branch point: sqrt(x) at 0 differentiates to
        // 1/(2·sqrt(0))). Publishing the coefficients would publish a division by zero as a value,
        // and publishing an O term would claim a truncation nobody computed, so the SOURCE function
        // is published unchanged — which is what SymPy publishes for these inputs
        // (series(sqrt(x), x, 0, 2) = sqrt(x), series(log(x), x, 0, 2) = log(x)) and is never wrong.
        if (!IsPowerSeries)
            return _source ?? throw new InvalidOperationException(
                "a series with no power series at the point must carry the function it was built from");

        var terms = new List<Expr>();
        var dx = Exprs.Subtract(Exprs.Symbol(Variable), Point);
        for (int k = 0; k < Coefficients.Length; k++)
        {
            var c = Coefficients[k];
            if (c is RationalConstantExpr rc && rc.Value.IsZero)
                continue;
            var pow = LeadingPower + k;
            terms.Add(pow == 0 ? c : Exprs.Multiply(c, Exprs.Power(dx, Exprs.Integer(pow))));
        }
        if (terms.Count == 0)
            return Exprs.Zero;
        terms.Add(Exprs.Order(Exprs.Symbol(Variable), Point, Exprs.Integer(TruncationOrder)));
        return Exprs.Add(terms);
    }

    public static Series Add(Series a, Series b)
    {
        // align on the smaller leading power
        int lp = Math.Min(a.LeadingPower, b.LeadingPower);
        int n = Math.Min(a.Order + (a.LeadingPower - lp), b.Order + (b.LeadingPower - lp));
        var c = new Expr[n];
        for (int i = 0; i < n; i++)
        {
            Expr sum = Exprs.Zero;
            int ia = i - (a.LeadingPower - lp);
            int ib = i - (b.LeadingPower - lp);
            if (ia >= 0 && ia < a.Order)
                sum = Exprs.Add(sum, a.Coefficients[ia]);
            if (ib >= 0 && ib < b.Order)
                sum = Exprs.Add(sum, b.Coefficients[ib]);
            c[i] = sum;
        }
        return new Series(a.Variable, a.Point, c, lp, a._source ?? b._source, a.IsPowerSeries && b.IsPowerSeries);
    }

    public static Series Negate(Series a)
    {
        return new Series(a.Variable, a.Point, a.Coefficients.Select(c => Exprs.Negate(c)).ToArray(), a.LeadingPower,
            a._source, a.IsPowerSeries);
    }

    public static Series Multiply(Series a, Series b)
    {
        int n = Math.Min(a.Order, b.Order);
        var c = new Expr[n];
        for (int k = 0; k < n; k++)
        {
            Expr sum = Exprs.Zero;
            for (int i = 0; i <= k; i++)
                if (i < a.Order && k - i < b.Order)
                    sum = Exprs.Add(sum, Exprs.Multiply(a.Coefficients[i], b.Coefficients[k - i]));
            c[k] = sum;
        }
        return new Series(a.Variable, a.Point, c, a.LeadingPower + b.LeadingPower,
            a._source ?? b._source, a.IsPowerSeries && b.IsPowerSeries);
    }

    /// <summary>
    /// Power-series division a/b, Laurent-aware: with <c>a = x^(LP_a)·Σ_{i≥la} a_i·x^i</c> and
    /// <c>b = x^(LP_b)·Σ_{j≥lb} b_j·x^j</c>, the quotient is
    /// <c>x^(LP_a − LP_b + la − lb)·Σ_k q_k·x^k</c> — so the LEADING-INDEX shift belongs in the
    /// leading power, not only the plain leading-power difference. That missing <c>(la − lb)</c> is
    /// why <c>series(1/x, x, 0, 2)</c> published <c>1 + O(x^2)</c>: the pole was computed and then
    /// announced at the wrong power, and the published expansion claimed 1 where the function is
    /// 1/x. This also makes <c>series(sin(x)/x, x, 0, 3)</c> carry its x² coefficient.
    /// <para>
    /// <paramref name="truncationPower"/> is the power of the TAIL, so the quotient carries every
    /// power below it (SymPy's convention: <c>series(1/x, x, 0, 2)</c> truncates at x², and
    /// <c>1/x²</c> therefore carries four coefficients, powers −2…1). Without it the coefficient
    /// count stays <c>min(a.Order, b.Order)</c>, which is what the Taylor callers want.
    /// </para>
    /// </summary>
    public static Series Divide(Series a, Series b, int? truncationPower = null)
    {
        int la = a.LeadingIndex();
        int lb = b.LeadingIndex();
        if (lb < 0)
            throw new DivideByZeroException("Series division by a zero series.");
        int leadingPower = a.LeadingPower + la - (b.LeadingPower + lb);
        int n = truncationPower is { } tail ? Math.Max(tail - leadingPower, 1) : Math.Min(a.Order, b.Order);
        bool isPowerSeries = a.IsPowerSeries && b.IsPowerSeries;
        Expr? source = a._source ?? b._source;
        if (la < 0)
        {
            // the numerator is zero through the order it was expanded to: the quotient is zero
            var zeros = new Expr[n];
            for (int i = 0; i < n; i++)
                zeros[i] = Exprs.Zero;
            return new Series(a.Variable, a.Point, zeros, leadingPower, source, isPowerSeries);
        }
        var b0 = b.Coefficients[lb];
        var q = new Expr[n];
        for (int k = 0; k < n; k++)
        {
            int ai = la + k;
            Expr sum = ai < a.Order ? a.Coefficients[ai] : Exprs.Zero;
            for (int i = 0; i < k; i++)
            {
                int bi = lb + k - i;
                if (bi < b.Order)
                    sum = Exprs.Subtract(sum, Exprs.Multiply(q[i], b.Coefficients[bi]));
            }
            q[k] = Exprs.Divide(sum, b0);
        }
        return new Series(a.Variable, a.Point, q, leadingPower, source, isPowerSeries);
    }

    /// <summary>Leading (first nonzero) coefficient index, or -1 when all coefficients are zero.</summary>
    public int LeadingIndex()
    {
        for (int i = 0; i < Coefficients.Length; i++)
        {
            if (Coefficients[i] is RationalConstantExpr rc && rc.Value.IsZero)
                continue;
            if (Coefficients[i] is IntegerConstantExpr ic && Int.IsZero(ic.Value))
                continue;
            return i;
        }
        return -1;
    }
}
