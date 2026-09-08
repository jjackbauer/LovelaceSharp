using Lovelace.Symbolics.Rewriting;
using Rat = global::Lovelace.Rational.Rational;

namespace Lovelace.Symbolics;

/// <summary>
/// simplify(): phase-ordered conditional rewrite groups over the canonical kernel
/// (architecture §7.5). Every rule carries a stable id and an assumption precondition.
/// </summary>
public static class Simplify
{
    public sealed record Options(bool Trace = false, int MaxSteps = 400);

    public static Expr SimplifyExpr(Expr e, ExprContext? ctx = null, Options? options = null)
    {
        ctx ??= Exprs.Current;
        options ??= new Options();

        var registry = BuildRegistry(ctx);
        var budget = new RewriteEngine.Budget { MaxSteps = options.MaxSteps };

        var cur = e;
        foreach (var group in new[] { "trig", "power", "logexp", "abs" })
        {
            var rules = registry.Group(group);
            cur = RewriteEngine.Apply(cur, ctx, rules, budget);
        }
        return cur;
    }

    private static RewriteRuleRegistry BuildRegistry(ExprContext ctx)
    {
        var reg = new RewriteRuleRegistry();

        // trig: sin(x)^2 + cos(x)^2 → 1  (canonical term order sorts the cos term first)
        var x = new WildPat("x");
        reg.Register(new RewriteRule(
            "trig.pythagorean-sin2-cos2",
            "trig",
            new AddPat(new Pattern[]
            {
                new PowPat(new FunPat("cos", new Pattern[] { x }), new LiteralPat(Exprs.Integer(2))),
                new PowPat(new FunPat("sin", new Pattern[] { new WildPat("y") }), new LiteralPat(Exprs.Integer(2))),
            }),
            (m, c) => m.Get("x") == m.Get("y"),
            (m, c) => Exprs.One));

        // power: sqrt(x^2) under assumptions
        var w = new WildPat("w");
        reg.Register(new RewriteRule(
            "pow.sqrt-square-nonnegative",
            "power",
            new PowPat(new PowPat(w, new LiteralPat(Exprs.Integer(2))), new LiteralPat(Exprs.Rational(1, 2))),
            (m, c) => m.Get("w") is SymbolExpr sx &&
                      c.Assumptions.Ask(new SymbolPropertyAssumption(sx.Symbol, SymbolPredicate.NonNegative)) == Tristate.True,
            (m, c) => m.Get("w")));
        reg.Register(new RewriteRule(
            "pow.sqrt-square-real",
            "power",
            new PowPat(new PowPat(new WildPat("w2"), new LiteralPat(Exprs.Integer(2))), new LiteralPat(Exprs.Rational(1, 2))),
            (m, c) => m.Get("w2") is SymbolExpr sx2 &&
                      c.Assumptions.Ask(new SymbolPropertyAssumption(sx2.Symbol, SymbolPredicate.NonNegative)) != Tristate.True &&
                      c.Assumptions.Ask(new SymbolDomainAssumption(sx2.Symbol, Domain.Real)) == Tristate.True,
            (m, c) => Exprs.Function(c.Function("abs"), m.Get("w2"))));

        // logexp: exp(log(x)) → x ; log(exp(x)) → x
        var lx = new WildPat("lx");
        reg.Register(new RewriteRule(
            "logexp.exp-log",
            "logexp",
            new FunPat("exp", new Pattern[] { new FunPat("log", new Pattern[] { lx }) }),
            (m, c) => true,
            (m, c) => m.Get("lx")));
        reg.Register(new RewriteRule(
            "logexp.log-exp",
            "logexp",
            new FunPat("log", new Pattern[] { new FunPat("exp", new Pattern[] { new WildPat("lx2") }) }),
            (m, c) => true,
            (m, c) => m.Get("lx2")));

        // abs: abs(x)^2 → x^2 ; abs(x)·abs(x) handled canonically; abs(-x) → abs(x)
        var aw = new WildPat("aw");
        reg.Register(new RewriteRule(
            "abs.abs-square",
            "abs",
            new PowPat(new FunPat("abs", new Pattern[] { aw }), new LiteralPat(Exprs.Integer(2))),
            (m, c) => true,
            (m, c) => Exprs.Power(m.Get("aw"), Exprs.Integer(2))));
        reg.Register(new RewriteRule(
            "abs.abs-neg",
            "abs",
            new FunPat("abs", new Pattern[] { new MulPat(new Pattern[] { new LiteralPat(Exprs.Rational(Rat.MinusOne)), new WildPat("aw2") }) }),
            (m, c) => true,
            (m, c) => Exprs.Function(c.Function("abs"), m.Get("aw2"))));

        return reg;
    }
}
