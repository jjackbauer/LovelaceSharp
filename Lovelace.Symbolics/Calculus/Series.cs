using Rat = global::Lovelace.Rational.Rational;
using Int = global::Lovelace.Integer.Integer;

namespace Lovelace.Symbolics;

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

    public Series(Symbol variable, Expr point, Expr[] coefficients)
    {
        Variable = variable;
        Point = point;
        Coefficients = coefficients;
    }

    public static Series Of(Expr f, Symbol x, Expr x0, int order, ExprContext? ctx = null)
    {
        ctx ??= Exprs.Current;
        if (order < 1)
            throw new ArgumentOutOfRangeException(nameof(order));

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

    private static Series OfNoSplit(Expr f, Symbol x, Expr x0, int order, ExprContext ctx)
    {
        // substitute x = t + x0 (fresh t), then compute Maclaurin coefficients in t
        var t = ctx.Symbol("__t" + Guid.NewGuid().ToString("N")[..6]);
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
            terms.Add(k == 0 ? c : Exprs.Multiply(c, Exprs.Power(dx, Exprs.Integer(k))));
        }
        return terms.Count == 0 ? Exprs.Zero : Exprs.Add(terms);
    }

    public static Series Add(Series a, Series b)
    {
        int n = Math.Min(a.Order, b.Order);
        var c = new Expr[n];
        for (int i = 0; i < n; i++)
            c[i] = Exprs.Add(a.Coefficients[i], b.Coefficients[i]);
        return new Series(a.Variable, a.Point, c);
    }

    public static Series Negate(Series a)
    {
        return new Series(a.Variable, a.Point, a.Coefficients.Select(c => Exprs.Negate(c)).ToArray());
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
        return new Series(a.Variable, a.Point, c);
    }

    /// <summary>
    /// Power-series division a/b, Laurent-aware: the leading indices of a and b are
    /// aligned so quotients like sin(x)/x work.
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
        return new Series(a.Variable, a.Point, q);
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
