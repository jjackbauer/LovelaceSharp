using System.Collections.Immutable;

namespace Lovelace.Symbolics;

/// <summary>Symbolic differentiation (memoized, canonical output, no full simplify).</summary>
public static class Calculus
{
    public static Expr Diff(Expr e, Symbol x, ExprContext? ctx = null)
    {
        ctx ??= Exprs.Current;
        return DiffWithMemo(e, x, ctx, new Dictionary<(Expr, string), Expr>());
    }

    private static Expr DiffWithMemo(Expr e, Symbol x, ExprContext ctx, Dictionary<(Expr, string), Expr> memo)
    {
        var key = (e, x.Name);
        if (memo.TryGetValue(key, out var cached))
            return cached;

        Expr result = e switch
        {
            SymbolExpr s => s.Symbol.Name == x.Name ? Exprs.One : Exprs.Zero,
            IntegerConstantExpr or RationalConstantExpr or RealConstantExpr or ComplexConstantExpr or NamedConstantExpr => Exprs.Zero,
            AddExpr a => Exprs.Add(a.Terms.Select(t => DiffWithMemo(t, x, ctx, memo))),
            MultiplyExpr m => DiffProduct(m.Factors, x, ctx, memo),
            PowerExpr p => DiffPower(p, x, ctx, memo),
            FunctionExpr f => DiffFunction(f, x, ctx, memo),
            PiecewiseExpr pw => Exprs.Piecewise(
                pw.Branches.Select(b => new PiecewiseBranch(b.Guard, DiffWithMemo(b.Value, x, ctx, memo))),
                DiffWithMemo(pw.Otherwise, x, ctx, memo)),
            IntegralExpr i => i.Variables.Any(v => v.Name == x.Name)
                ? i.Operand   // fundamental theorem of calculus
                : Exprs.Integral(DiffWithMemo(i.Operand, x, ctx, memo), i.Variables.ToArray()),
            DerivativeExpr d => Exprs.Derivative(d.Operand, d.Variables.Add(x).ToArray()),
            _ => Exprs.Derivative(e, x),   // relations/rootof stay unevaluated
        };

        memo[key] = result;
        return result;
    }

    private static Expr DiffProduct(ImmutableArray<Expr> factors, Symbol x, ExprContext ctx, Dictionary<(Expr, string), Expr> memo)
    {
        // product rule, left fold
        var terms = new List<Expr>();
        for (int i = 0; i < factors.Length; i++)
        {
            var others = new List<Expr>();
            for (int j = 0; j < factors.Length; j++)
                if (j != i) others.Add(factors[j]);
            var rest = others.Count == 0 ? Exprs.One
                : others.Count == 1 ? others[0]
                : Exprs.Multiply(others);
            terms.Add(Exprs.Multiply(DiffWithMemo(factors[i], x, ctx, memo), rest));
        }
        return Exprs.Add(terms);
    }

    private static Expr DiffPower(PowerExpr p, Symbol x, ExprContext ctx, Dictionary<(Expr, string), Expr> memo)
    {
        var b = p.Base;
        var e = p.Exponent;
        if (FreeOf(e, x))
        {
            // d/dx b^e = e * b^(e-1) * b'
            var eMinusOne = Exprs.Subtract(e, Exprs.One);
            return Exprs.Multiply(e, Exprs.Power(b, eMinusOne), DiffWithMemo(b, x, ctx, memo));
        }
        // general: b^e (e' ln b + e b'/b)
        var lnb = Exprs.Function(ctx.Function("log"), b);
        var inner = Exprs.Add(
            Exprs.Multiply(DiffWithMemo(e, x, ctx, memo), lnb),
            Exprs.Multiply(e, Exprs.Divide(DiffWithMemo(b, x, ctx, memo), b)));
        return Exprs.Multiply(Exprs.Power(b, e), inner);
    }

    private static Expr DiffFunction(FunctionExpr f, Symbol x, ExprContext ctx, Dictionary<(Expr, string), Expr> memo)
    {
        var def = ctx.Functions.Get(f.Function.Name);
        var arg = f.Arguments[0];
        var dArg = DiffWithMemo(arg, x, ctx, memo);

        // d|x|/dx = sign(x) for x ≠ 0, undefined at 0 — the condition is carried explicitly
        if (f.Function.Name == "abs")
        {
            var sign = Exprs.Function(ctx.Function("sign"), arg);
            var guard = Exprs.Relation(RelOp.Ne, arg, Exprs.Zero);
            var conditional = Exprs.Piecewise(
                new[] { new PiecewiseBranch(guard, sign) },
                Exprs.Derivative(f, x));
            return Exprs.Multiply(conditional, dArg);
        }

        if (def?.DerivativeTemplate is { } tpl)
            return Exprs.Multiply(tpl(arg), dArg);
        // unknown or nondifferentiable function (sign/floor/ceil/min/max): unevaluated
        // derivative of the full application — never a silent 0 and never a dropped argument
        if (arg is SymbolExpr sarg)
            return Exprs.Multiply(Exprs.Derivative(Exprs.Function(f.Function, f.Arguments.ToArray()), sarg.Symbol), dArg);
        return Exprs.Derivative(f, x);
    }

    /// <summary>True when the expression does not mention the symbol.</summary>
    public static bool FreeOf(Expr e, Symbol x)
    {
        switch (e)
        {
            case SymbolExpr s:
                return s.Symbol.Name != x.Name;
            case AddExpr a:
                return a.Terms.All(t => FreeOf(t, x));
            case MultiplyExpr m:
                return m.Factors.All(f => FreeOf(f, x));
            case PowerExpr p:
                return FreeOf(p.Base, x) && FreeOf(p.Exponent, x);
            case FunctionExpr f:
                return f.Arguments.All(a => FreeOf(a, x));
            case RelationExpr r:
                return FreeOf(r.Left, x) && FreeOf(r.Right, x);
            case PiecewiseExpr pw:
                return pw.Branches.All(b => FreeOf(b.Guard, x) && FreeOf(b.Value, x))
                       && FreeOf(pw.Otherwise, x);
            case DerivativeExpr d:
                return d.Variables.All(v => v.Name != x.Name) && FreeOf(d.Operand, x);
            case IntegralExpr i:
                return i.Variables.All(v => v.Name != x.Name) && FreeOf(i.Operand, x);
            case RootOfExpr ro:
                return ro.DefiningPolynomial.Order.Variables.All(v => v.Name != x.Name);
            default:
                return true;
        }
    }

    /// <summary>n-th order derivative (memoized).</summary>
    public static Expr DiffN(Expr e, Symbol x, int n, ExprContext? ctx = null)
    {
        ctx ??= Exprs.Current;
        var cur = e;
        for (int i = 0; i < n; i++)
            cur = Diff(cur, x, ctx);
        return cur;
    }
}
