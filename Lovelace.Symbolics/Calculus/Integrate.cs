using Rat = global::Lovelace.Rational.Rational;

namespace Lovelace.Symbolics;

public enum IntegrationKind { SolvedExact, SolvedConditional, Unevaluated }

/// <summary>
/// Structured integration outcome. Conditional results carry the conditions under which the
/// antiderivative is valid (e.g. ∫1/x dx = log(x) requires x > 0 over the reals).
/// </summary>
public sealed record IntegrationResult(
    IntegrationKind Kind,
    Expr Expression,
    AssumptionSet Conditions,
    string? Note = null)
{
    public static IntegrationResult Exact(Expr e) => new(IntegrationKind.SolvedExact, e, AssumptionSet.Empty);
    public static IntegrationResult Conditional(Expr e, AssumptionSet conditions) => new(IntegrationKind.SolvedConditional, e, conditions);
    public static IntegrationResult Unevaluated(Expr e) => new(IntegrationKind.Unevaluated, e, AssumptionSet.Empty);
}

/// <summary>
/// Tiered symbolic integration with mandatory self-verification: every closed form is
/// differentiated and checked against the integrand by canonical equality before being
/// returned; failures fall through to the next tier and finally to the unevaluated
/// <see cref="IntegralExpr"/>.
/// </summary>
public static class Integration
{
    public static Expr Integrate(Expr e, Symbol x, ExprContext? ctx = null)
        => IntegrateResult(e, x, ctx).Expression;

    /// <summary>Structured integration with status and the conditions its antiderivative needs.</summary>
    public static IntegrationResult IntegrateResult(Expr e, Symbol x, ExprContext? ctx = null)
    {
        ctx ??= Exprs.Current;
        var result = TryIntegrate(e, x, ctx, 0);
        if (result is not null && Verify(result, e, x, ctx))
        {
            // log(f) antiderivatives are real-valued only where f > 0
            var conditions = AssumptionSet.Empty;
            foreach (var logArg in LogArguments(result))
            {
                try
                {
                    conditions = conditions.Add(new ExpressionPropertyAssumption(logArg, SymbolPredicate.Positive));
                }
                catch (AssumptionContradictionException)
                {
                }
            }
            return conditions.Atoms.Length > 0
                ? IntegrationResult.Conditional(result, conditions)
                : IntegrationResult.Exact(result);
        }
        return IntegrationResult.Unevaluated(Exprs.Integral(e, x));
    }

    /// <summary>Collects the arguments of every log(f) in the expression.</summary>
    private static IEnumerable<Expr> LogArguments(Expr e)
    {
        switch (e)
        {
            case FunctionExpr f when f.Function.Name == "log" && f.Arguments.Length == 1:
                yield return f.Arguments[0];
                foreach (var inner in LogArguments(f.Arguments[0]))
                    yield return inner;
                break;
            case AddExpr a:
                foreach (var t in a.Terms)
                    foreach (var inner in LogArguments(t))
                        yield return inner;
                break;
            case MultiplyExpr m:
                foreach (var f2 in m.Factors)
                    foreach (var inner in LogArguments(f2))
                        yield return inner;
                break;
            case PowerExpr p:
                foreach (var inner in LogArguments(p.Base)) yield return inner;
                foreach (var inner in LogArguments(p.Exponent)) yield return inner;
                break;
            case FunctionExpr f2:
                foreach (var arg in f2.Arguments)
                    foreach (var inner in LogArguments(arg))
                        yield return inner;
                break;
        }
    }

    internal static bool Verify(Expr antiderivative, Expr integrand, Symbol x, ExprContext ctx)
    {
        // Differentiation is total over the expression DAG: exceptions here are defects and
        // must surface, not be swallowed into "verification failed".
        var d = Calculus.Diff(antiderivative, x, ctx);
        if (d == integrand)
            return true;
        // algebraic equivalence: expansion collapses like terms (e.g. e^x + (x-1)e^x = x·e^x)
        try
        {
            if (Algebra.Expand(d, ctx) == Algebra.Expand(integrand, ctx))
                return true;
        }
        catch (Exception)
        {
            // fall through to the rational check
        }
        // rational functions: bring the derivative's terms over the integrand's
        // denominator and compare polynomials (sum of fractions equality)
        var (in_, id) = SplitFraction(integrand);
        if (Polynomial.TryFromExpr(in_, ctx, new[] { x }, out var inP, out _) &&
            Polynomial.TryFromExpr(id, ctx, new[] { x }, out var idP, out _))
        {
            var terms = d is AddExpr da ? da.Terms.ToArray() : new[] { d };
            var sum = Polynomial.Zero(idP.Order);
            foreach (var t in terms)
            {
                var (tn, td) = SplitFraction(t);
                if (!Polynomial.TryFromExpr(tn, ctx, new[] { x }, out var tnP, out _) ||
                    !Polynomial.TryFromExpr(td, ctx, new[] { x }, out var tdP, out _))
                    return false;
                var (q, rem) = idP.DivRem(tdP, MonomialOrder.Lex);
                if (!rem.IsZero)
                    return false;
                sum = Polynomial.Add(sum, Polynomial.Multiply(tnP, q));
            }
            return sum.Equals(inP);
        }
        return false;
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

    internal static Expr? TryIntegrate(Expr e, Symbol x, ExprContext ctx, int depth)
    {
        if (depth > 24)
            return null;

        // tier 0: linearity
        if (e is AddExpr add)
        {
            var parts = new List<Expr>();
            foreach (var t in add.Terms)
            {
                var part = TryIntegrate(t, x, ctx, depth + 1);
                if (part is null)
                    return null;
                parts.Add(part);
            }
            return Exprs.Add(parts);
        }

        // constant term (free of x)
        if (Calculus.FreeOf(e, x))
            return Exprs.Multiply(e, Exprs.Symbol(x));

        // tier 1: monomial and table entries
        var table = TryTable(e, x, ctx);
        if (table is not null)
            return table;

        // tier 2/5: substitution patterns on products
        if (e is MultiplyExpr m)
        {
            var sub = TrySubstitution(m, x, ctx, depth);
            if (sub is not null)
                return sub;
        }

        // tier 6: integration by parts patterns
        var byParts = TryByParts(e, x, ctx, depth);
        if (byParts is not null)
            return byParts;

        // tier 2: rational functions via partial fractions
        var rational = TryRational(e, x, ctx);
        if (rational is not null)
            return rational;

        // tier 7: trig identities
        var trig = TryTrigIdentities(e, x, ctx, depth);
        if (trig is not null)
            return trig;

        return null;
    }

    private static Expr? TryTable(Expr e, Symbol x, ExprContext ctx)
    {
        // coefficient * table form: ∫ c·f = c·∫f (the SplitTerm coefficient is numeric)
        if (e is MultiplyExpr mc)
        {
            var (coef, rest) = Exprs.SplitTerm(e);
            if (!coef.IsOne)
            {
                var inner = TryTable(rest, x, ctx);
                if (inner is not null)
                    return Exprs.Multiply(Exprs.Rational(coef), inner);
            }
            return null;
        }
        // x^n
        if (e is SymbolExpr s && s.Symbol.Name == x.Name)
            return Exprs.Multiply(Exprs.Rational(1, 2), Exprs.Power(e, Exprs.Integer(2)));
        if (e is PowerExpr p && p.Base is SymbolExpr sb && sb.Symbol.Name == x.Name && p.Exponent is RationalConstantExpr pe)
        {
            var n = pe.Value;
            if (n.IsMinusOne)
                return LogOf(x);
            if (n != Rat.MinusOne)
            {
                var n1 = n + Rat.One;
                return Exprs.Divide(Exprs.Power(p.Base, Exprs.Rational(n1)), Exprs.Rational(n1));
            }
        }
        // f(a x + b) with f' constant: sin/cos/exp/log
        if (TryLinearArg(e, x, out var c, out var b, out var kind))
        {
            var arg = Exprs.Add(Exprs.Multiply(c, Exprs.Symbol(x)), b);
            switch (kind)
            {
                case "sin": return Exprs.Divide(Exprs.Negate(Exprs.Function(ctx.Function("cos"), arg)), c);
                case "cos": return Exprs.Divide(Exprs.Function(ctx.Function("sin"), arg), c);
                case "exp": return Exprs.Divide(Exprs.Function(ctx.Function("exp"), arg), c);
                case "sinh": return Exprs.Divide(Exprs.Function(ctx.Function("cosh"), arg), c);
                case "cosh": return Exprs.Divide(Exprs.Function(ctx.Function("sinh"), arg), c);
            }
        }
        // 1/x → log(x); 1/(1+x^2) → atan(x)
        if (e is PowerExpr p2 && p2.Base is SymbolExpr sb2 && sb2.Symbol.Name == x.Name && p2.Exponent is RationalConstantExpr pe2 && pe2.Value.IsMinusOne)
            return LogOf(x);
        // 1/(a·x + b) → log(a·x + b)/a
        if (e is PowerExpr plin && plin.Exponent is RationalConstantExpr pelin && pelin.Value.IsMinusOne &&
            IsLinearIn(plin.Base, x, out var cLin, out _))
        {
            return Exprs.Divide(Exprs.Function(ctx.Function("log"), plin.Base), cLin);
        }
        if (e is PowerExpr p3 && p3.Exponent is RationalConstantExpr pe3 && pe3.Value.IsMinusOne)
        {
            var baseInner = p3.Base;
            if (baseInner is AddExpr addInner && addInner.Terms.Length == 2)
            {
                // 1/(1+x^2) → atan(x)
                var t0 = addInner.Terms[0];
                var t1 = addInner.Terms[1];
                Expr one = Exprs.One, xsq = Exprs.Power(Exprs.Symbol(x), Exprs.Integer(2));
                if ((t0 == one && t1 == xsq) || (t1 == one && t0 == xsq))
                    return Exprs.Function(ctx.Function("atan"), Exprs.Symbol(x));
            }
        }
        return null;
    }

    /// <summary>Detects f(c·x + b) for table functions; returns (coefficient c, offset b, kind).</summary>
    private static bool TryLinearArg(Expr e, Symbol x, out Expr c, out Expr b, out string kind)
    {
        c = Exprs.One;
        b = Exprs.Zero;
        kind = "";
        if (e is FunctionExpr f && f.Arguments.Length == 1)
        {
            var arg = f.Arguments[0];
            if (IsLinearIn(arg, x, out c, out b))
            {
                kind = f.Function.Name;
                return kind is "sin" or "cos" or "exp" or "sinh" or "cosh";
            }
        }
        return false;
    }

    private static bool IsLinearIn(Expr arg, Symbol x, out Expr c, out Expr b)
    {
        c = Exprs.Zero;
        b = Exprs.Zero;
        var terms = arg is AddExpr add ? add.Terms.ToArray() : new[] { arg };
        foreach (var t in terms)
        {
            var (coef, rest) = Exprs.SplitTerm(t);
            if (rest == Exprs.Symbol(x))
            {
                c = Exprs.Add(c, Exprs.Rational(coef));
            }
            else if (Calculus.FreeOf(rest, x))
            {
                b = Exprs.Add(b, t);
            }
            else
            {
                return false;
            }
        }
        return !(c is RationalConstantExpr rc && rc.Value.IsZero);
    }

    private static Expr LogOf(Symbol x)
    {
        var ctx = Exprs.Current;
        return Exprs.Function(ctx.Function("log"), Exprs.Symbol(x));
    }

    // -----------------------------------------------------------------
    // u-substitution: g'(x)·f(g(x)) patterns
    // -----------------------------------------------------------------

    private static Expr? TrySubstitution(MultiplyExpr m, Symbol x, ExprContext ctx, int depth)
    {
        var factors = m.Factors.ToArray();
        // candidate g' groups: single factors and factor pairs (e.g. 2·x as the derivative of x²)
        var candidates = new List<(Expr Gp, Expr Remainder)>();
        for (int i = 0; i < factors.Length; i++)
        {
            var restI = factors.Where((_, idx) => idx != i).ToArray();
            candidates.Add((factors[i], Wrap(restI)));
        }
        for (int i = 0; i < factors.Length; i++)
            for (int j = i + 1; j < factors.Length; j++)
            {
                var restIJ = factors.Where((_, idx) => idx != i && idx != j).ToArray();
                candidates.Add((Exprs.Multiply(factors[i], factors[j]), Wrap(restIJ)));
            }

        foreach (var (gp, rest) in candidates)
        {
            // pattern A: rest = g^n with n != -1 and gp == c·g'
            if (rest is PowerExpr p && p.Exponent is RationalConstantExpr pe)
            {
                var n = pe.Value;
                var scale = MatchDerivative(p.Base, gp, x, ctx);
                if (scale is not null && !n.IsMinusOne)
                {
                    var n1 = n + Rat.One;
                    if (!n1.IsZero)
                        return Exprs.Multiply(scale, Exprs.Divide(Exprs.Power(p.Base, Exprs.Rational(n1)), Exprs.Rational(n1)));
                }
            }

            // pattern B: rest = f(g) from the table, gp == c·g'
            if (rest is FunctionExpr rf && rf.Arguments.Length == 1)
            {
                var g = rf.Arguments[0];
                var scale = MatchDerivative(g, gp, x, ctx);
                if (scale is not null)
                {
                    var substituted = TableWithArg(rf.Function.Name, g, ctx);
                    if (substituted is not null)
                        return Exprs.Multiply(scale, substituted);
                }
            }
        }
        return null;
    }

    private static Expr Wrap(Expr[] factors) =>
        factors.Length == 0 ? Exprs.One : factors.Length == 1 ? factors[0] : Exprs.Multiply(factors);

    /// <summary>Returns the scale s such that gp == s·g' (1 when equal); null when no match.</summary>
    private static Expr? MatchDerivative(Expr g, Expr gp, Symbol x, ExprContext ctx)
    {
        var dg = Calculus.Diff(g, x, ctx);
        if (dg == gp)
            return Exprs.One;
        var (dc, dr) = Exprs.SplitTerm(dg);
        if (dr == gp && !dc.IsZero)
            return Exprs.Divide(Exprs.One, Exprs.Rational(dc));
        return null;
    }

    private static Expr? TableWithArg(string fn, Expr g, ExprContext ctx)
    {
        // table integrals with g as the argument (assumes g' = 1 factor already consumed)
        return fn switch
        {
            "sin" => Exprs.Negate(Exprs.Function(ctx.Function("cos"), g)),
            "cos" => Exprs.Function(ctx.Function("sin"), g),
            "exp" => Exprs.Function(ctx.Function("exp"), g),
            "sinh" => Exprs.Function(ctx.Function("cosh"), g),
            "cosh" => Exprs.Function(ctx.Function("sinh"), g),
            _ => null,
        };
    }

    // -----------------------------------------------------------------
    // integration by parts patterns
    // -----------------------------------------------------------------

    private static Expr? TryByParts(Expr e, Symbol x, ExprContext ctx, int depth)
    {
        if (e is not MultiplyExpr m || m.Factors.Length != 2)
            return null;
        var (f0, f1) = (m.Factors[0], m.Factors[1]);
        var (coef0, rest0) = Exprs.SplitTerm(f0);
        var (coef1, rest1) = Exprs.SplitTerm(f1);

        Expr? Pow = null, Other = null;
        if (rest0 == Exprs.Symbol(x)) { Pow = f0; Other = f1; }
        else if (rest1 == Exprs.Symbol(x)) { Pow = f1; Other = f0; }
        if (Pow is null)
            return null;

        // x·e^x, x·sin(x), x·cos(x)
        var cExpr = Exprs.Rational((rest0 == Exprs.Symbol(x) ? coef0 : coef1));
        if (Other is FunctionExpr f && f.Arguments.Length == 1 && f.Arguments[0] == Exprs.Symbol(x))
        {
            switch (f.Function.Name)
            {
                case "exp":
                    return Exprs.Multiply(cExpr, Exprs.Subtract(Exprs.Symbol(x), Exprs.One),
                        Exprs.Function(ctx.Function("exp"), Exprs.Symbol(x)));
                case "sin":
                    return Exprs.Multiply(cExpr, Exprs.Subtract(
                        Exprs.Function(ctx.Function("sin"), Exprs.Symbol(x)),
                        Exprs.Multiply(Exprs.Symbol(x), Exprs.Function(ctx.Function("cos"), Exprs.Symbol(x)))));
                case "cos":
                    return Exprs.Multiply(cExpr, Exprs.Add(
                        Exprs.Function(ctx.Function("cos"), Exprs.Symbol(x)),
                        Exprs.Multiply(Exprs.Symbol(x), Exprs.Function(ctx.Function("sin"), Exprs.Symbol(x)))));
            }
        }
        // ln(x) alone and x^n·ln(x)
        if (e is FunctionExpr lnf && lnf.Function.Name == "log" && lnf.Arguments.Length == 1 && lnf.Arguments[0] == Exprs.Symbol(x))
            return Exprs.Subtract(Exprs.Multiply(Exprs.Symbol(x), e), Exprs.Symbol(x));
        if (Other is FunctionExpr lnf2 && lnf2.Function.Name == "log" && lnf2.Arguments.Length == 1 && lnf2.Arguments[0] == Exprs.Symbol(x)
            && rest0 == Exprs.Symbol(x))
        {
            var n = coef0;
            // ∫ x^n ln(x) dx = x^(n+1) (ln(x)/(n+1) - 1/(n+1)^2)
            if (n is Rat rat && !rat.IsMinusOne)
            {
                var n1 = rat + Rat.One;
                return Exprs.Multiply(
                    Exprs.Power(Exprs.Symbol(x), Exprs.Rational(n1)),
                    Exprs.Subtract(
                        Exprs.Divide(e, Exprs.Rational(n1)),
                        Exprs.Divide(Exprs.One, Exprs.Multiply(Exprs.Rational(n1), Exprs.Rational(n1)))));
            }
        }
        return null;
    }

    // -----------------------------------------------------------------
    // rational functions: partial fractions
    // -----------------------------------------------------------------

    private static Expr? TryRational(Expr e, Symbol x, ExprContext ctx)
    {
        if (!Polynomial.TryFromExpr(e, ctx, new[] { x }, out _, out _))
        {
            var apart = RationalFunctions.Apart(e, x, ctx);
            if (apart != e)
            {
                return TryIntegrate(apart, x, ctx, 8);
            }
        }
        return null;
    }

    // -----------------------------------------------------------------
    // trig identities
    // -----------------------------------------------------------------

    private static Expr? TryTrigIdentities(Expr e, Symbol x, ExprContext ctx, int depth)
    {
        // sin^2(x) → (1 - cos(2x))/2 ; cos^2(x) → (1 + cos(2x))/2
        if (e is PowerExpr p && p.Base is FunctionExpr f && f.Arguments.Length == 1 && f.Arguments[0] == Exprs.Symbol(x)
            && p.Exponent is RationalConstantExpr pe && pe.Value == Rat.FromLong(2L))
        {
            var cos2 = Exprs.Function(ctx.Function("cos"), Exprs.Multiply(Exprs.Integer(2), Exprs.Symbol(x)));
            return f.Function.Name switch
            {
                "sin" => Exprs.Divide(Exprs.Subtract(Exprs.One, cos2), Exprs.Integer(2)),
                "cos" => Exprs.Divide(Exprs.Add(Exprs.One, cos2), Exprs.Integer(2)),
                _ => null,
            };
        }
        return null;
    }
}
