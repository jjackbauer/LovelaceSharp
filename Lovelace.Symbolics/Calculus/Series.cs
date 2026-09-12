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

    public Series(Symbol variable, Expr point, Expr[] coefficients, int leadingPower = 0)
    {
        Variable = variable;
        Point = point;
        Coefficients = coefficients;
        LeadingPower = leadingPower;
    }

    /// <summary>True leading order of the expansion: LeadingPower + first nonzero coefficient index.</summary>
    public int LeadingOrder() => LeadingPower + LeadingIndex();

    public static Series Of(Expr f, Symbol x, Expr x0, int order, ExprContext? ctx = null,
        SeriesKinkPolicy kinks = SeriesKinkPolicy.ResolveByLeadingSign)
    {
        ctx ??= Exprs.Current;
        if (order < 1)
            throw new ArgumentOutOfRangeException(nameof(order));

        // a kink the expansion crosses is resolved BEFORE anything is differentiated: the machinery
        // can only see a derivative at a point where the function has one (Round 20, H-1)
        if (kinks == SeriesKinkPolicy.ResolveByLeadingSign)
            f = ResolveKinks(f, x, x0, order, ctx);

        // fractions: series of numerator and denominator separately, then divide
        // (canonical 0/0 folding would otherwise destroy the information at the point)
        var (fn, fd) = SplitFraction(f);
        if (fd is not RationalConstantExpr { Value.IsOne: true })
        {
            var sn = OfNoSplit(fn, x, x0, order, ctx);
            var sd = OfNoSplit(fd, x, x0, order, ctx);
            return Divide(sn, sd);
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

    private static Series OfNoSplit(Expr f, Symbol x, Expr x0, int order, ExprContext ctx)
    {
        // substitute x = t + x0 (fresh t), then compute Maclaurin coefficients in t
        var t = InternalVariable(f, x, x0, ctx);
        var shifted = Evaluation.Substitute(f, ctx, new Dictionary<Symbol, Expr> { [x] = Exprs.Add(Exprs.Symbol(t), x0) });

        var coeffs = new Expr[order];
        var bindings = new Dictionary<Symbol, Num> { [t] = new NumInt(new Int(0L)) };
        for (int k = 0; k < order; k++)
        {
            var dk = Calculus.DiffN(shifted, t, k, ctx);
            var at0 = Evaluation.Substitute(dk, ctx, new Dictionary<Symbol, Expr> { [t] = Exprs.Zero });
            // numeric evaluation folds constants; keep symbolic parts symbolic
            var evaluated = Evaluation.EvaluateToExpr(at0, ctx, bindings);
            coeffs[k] = Exprs.Divide(evaluated, FactorialExpr(k));
        }
        return new Series(x, x0, coeffs);
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
        return new Series(a.Variable, a.Point, c, lp);
    }

    public static Series Negate(Series a)
    {
        return new Series(a.Variable, a.Point, a.Coefficients.Select(c => Exprs.Negate(c)).ToArray(), a.LeadingPower);
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
        return new Series(a.Variable, a.Point, c, a.LeadingPower + b.LeadingPower);
    }

    /// <summary>
    /// Power-series division a/b, Laurent-aware: the quotient carries the leading-power
    /// offset (leading powers subtract), so (1−cos x)/x yields the correct first term x/2
    /// instead of a misindexed constant.
    /// </summary>
    public static Series Divide(Series a, Series b)
    {
        int la = a.LeadingIndex();
        int lb = b.LeadingIndex();
        if (lb < 0)
            throw new DivideByZeroException("Series division by a zero series.");
        int n = Math.Min(a.Order, b.Order);
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
        return new Series(a.Variable, a.Point, q, a.LeadingPower - b.LeadingPower);
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
