using Lovelace.Symbolics.Rewriting;
using Rat = global::Lovelace.Rational.Rational;

namespace Lovelace.Symbolics;

/// <summary>
/// The disposition of a transformation: <see cref="Satisfied"/> (the rewrite completed under its
/// collected side conditions), <see cref="BudgetExceeded"/> (rewriting stopped at the step
/// budget with a partial result) or <see cref="Unsatisfiable"/> (the applied rules carried
/// contradictory requirements, so the branch has no model).
/// <para>
/// A real kernel enum rather than a computed C# string: the DSH wire form is an Enum value that
/// carries this type's name, so a consumer switches on a type instead of matching spellings.
/// </para>
/// </summary>
public enum TransformStatus
{
    Satisfied,
    BudgetExceeded,
    Unsatisfiable,
}

/// <summary>
/// A transformation outcome: the transformed expression, the side conditions the result
/// requires to equal the input pointwise (definedness delta), and the provenance trace.
/// <see cref="TransformStatus.BudgetExceeded"/> reports that rewriting stopped at the step budget
/// with a partial result. <see cref="AssumptionSet.Unsatisfiable"/> conditions mean the applied
/// rules carried contradictory requirements (the branch has no model).
/// </summary>
public sealed record TransformResult(
    Expr Expression,
    AssumptionSet Conditions,
    IReadOnlyList<RewriteStep> Steps,
    bool BudgetExceeded = false)
{
    /// <summary>The expression the transformation started from.</summary>
    public Expr Original { get; init; } = Expression;

    /// <summary>Structured status: Satisfied, BudgetExceeded, or Unsatisfiable.</summary>
    public TransformStatus Status => Conditions.IsUnsatisfiable
        ? TransformStatus.Unsatisfiable
        : BudgetExceeded ? TransformStatus.BudgetExceeded : TransformStatus.Satisfied;

    /// <summary>Machine-readable budget identity when <see cref="BudgetExceeded"/> is set.</summary>
    public string? BudgetKind { get; init; }

    /// <summary>Per-attempt rule diagnostics; empty unless explicitly requested.</summary>
    public IReadOnlyList<Rewriting.RuleAttemptDiagnostic> Attempts { get; init; }
        = Array.Empty<Rewriting.RuleAttemptDiagnostic>();
}

/// <summary>
/// simplify(): phase-ordered conditional rewrite groups over the canonical kernel
/// (architecture §7.5). Every rule carries a stable id, an assumption precondition, and a
/// semantic classification.
///
/// Two entry points implement the safe/full split:
/// <list type="bullet">
/// <item><see cref="SimplifyExpr"/> — the safe convenience API. It applies only Universal
/// rules and rules whose declared side conditions are already proven by the active
/// assumption set (Unknown never licenses a rewrite), so its result needs no conditions.</item>
/// <item><see cref="Transform"/> — the full API. It applies every matching rule and returns
/// the expression together with the unresolved side conditions and the provenance steps.</item>
/// </list>
/// </summary>
public static class Simplify
{
    public sealed record Options(bool Trace = false, int MaxSteps = 400, bool Diagnose = false);

    public static Expr SimplifyExpr(Expr e, ExprContext? ctx = null, Options? options = null)
        => TransformCore(e, ctx, options, safe: true).Expression;

    public static TransformResult Transform(Expr e, ExprContext? ctx = null, Options? options = null)
        => TransformCore(e, ctx, options, safe: false);

    private static TransformResult TransformCore(Expr e, ExprContext? ctx, Options? options, bool safe)
    {
        ctx ??= Exprs.Current;
        options ??= new Options();

        var registry = BuildRegistry(ctx);
        var budget = new RewriteEngine.Budget { MaxSteps = options.MaxSteps };
        // conditions are semantics and always collected; the trace (before/after/rule ids) is
        // provenance and is collected only when requested
        var trace = options.Trace ? new List<RewriteStep>() : null;
        var appliedConditions = new List<AssumptionSet>();
        var attempts = options.Diagnose ? new List<RuleAttemptDiagnostic>() : null;

        var cur = e;
        foreach (var group in new[] { "trig", "power", "rat", "logexp", "abs" })
        {
            // a cancelled evaluation stops between passes; the caller keeps whatever partial
            // state it can read rather than waiting for all five groups
            Lovelace.Abstractions.Cancellation.ThrowIfCancellationRequested();
            // once the budget is exhausted there is nothing left to gain from another pass;
            // continuing would only rebuild the tree five more times
            if (budget.Exhausted)
                break;
            var rules = registry.Group(group);
            if (safe)
                rules = rules.Select(SafeWrap).ToArray();
            cur = RewriteEngine.Apply(cur, ctx, rules, budget, trace, appliedConditions, attempts);
        }

        // collect the declared side conditions of every applied rule; contradictory
        // requirements make the branch unsatisfiable rather than silently keeping the first
        var conditions = AssumptionSet.Empty;
        foreach (var s in appliedConditions)
        {
            foreach (var a in s.Atoms)
            {
                try { conditions = conditions.Add(a); }
                catch (AssumptionContradictionException)
                {
                    conditions = AssumptionSet.Unsatisfiable;
                    break;
                }
            }
            if (conditions.IsUnsatisfiable)
                break;
        }

        // §21: a requirement the ACTIVE assumptions already refute makes this branch
        // unsatisfiable. Without this the transform reports "satisfied, provided x != 0" on a
        // set that proves x == 0 — a condition set with no model presented as a live branch.
        if (!conditions.IsUnsatisfiable)
            conditions = RefutedByAssumptions(conditions, ctx.Assumptions)
                ? AssumptionSet.Unsatisfiable
                : conditions;
        return new TransformResult(cur, conditions,
            trace ?? (IReadOnlyList<RewriteStep>)Array.Empty<RewriteStep>(),
            budget.Exhausted)
        {
            Original = e,
            BudgetKind = budget.Exhausted ? budget.Kind : null,
            Attempts = attempts ?? (IReadOnlyList<RuleAttemptDiagnostic>)Array.Empty<RuleAttemptDiagnostic>(),
        };
    }

    /// <summary>True when the ambient assumption set proves the negation of one of
    /// <paramref name="collected"/>'s atoms, so the collected branch has no model.</summary>
    private static bool RefutedByAssumptions(AssumptionSet collected, AssumptionSet ambient)
    {
        if (ambient.IsUnsatisfiable)
            return true;
        foreach (var a in collected.Atoms)
        {
            // a property requirement is refuted by its relation form: NonZero(x) is refuted by
            // an assumption set that proves x != 0 false (the property lattice has no "Zero"
            // atom, so asking the property directly would answer Unknown)
            if (a is SymbolPropertyAssumption { S: var s, P: var p } &&
                AssumptionSet.RelationOf(s, p) is { } relation)
            {
                if (ambient.Ask(relation) == Tristate.False)
                    return true;
                continue;
            }
            var negation = AssumptionSet.Negate(a);
            if (negation is not null && ambient.Ask(negation) == Tristate.True)
                return true;
        }
        return false;
    }

    /// <summary>Safe-mode rule wrapper: a rule fires only when its declared side conditions
    /// are already provable from the active assumptions (Unknown never licenses a rewrite).</summary>
    private static RewriteRule SafeWrap(RewriteRule rule) => new(
        rule.Id, rule.Group, rule.Pattern,
        (m, c) =>
        {
            var a = rule.Applicability(m, c);
            if (a is RuleApplicability.NotApplicable or RuleApplicability.Unknown)
                return a;
            // Unknown never licenses a rewrite: an unresolved conditional side condition is
            // exactly what "safe" mode must refuse.
            return ConditionsProvable(rule, m, c) ? a : RuleApplicability.NotApplicable;
        },
        rule.Replacement)
    {
        Classification = rule.Classification,
        DeclaredConditions = rule.DeclaredConditions,
        ConditionBuilder = rule.ConditionBuilder,
    };

    private static bool ConditionsProvable(RewriteRule rule, Match m, ExprContext c)
    {
        var conds = rule.ConditionBuilder?.Invoke(m, c) ?? rule.DeclaredConditions;
        if (conds.IsUnsatisfiable)
            return false;
        foreach (var a in conds.Atoms)
        {
            if (c.Assumptions.Ask(a) != Tristate.True)
                return false;
        }
        return true;
    }

    /// <summary>The shipped rule set, for validation, documentation and tests.</summary>
    public static IReadOnlyList<RewriteRule> RulesForTesting(ExprContext? ctx = null) =>
        BuildRegistry(ctx ?? Exprs.Current).All;

    /// <summary>The ids of every shipped rewrite rule, for tooling (the proof bridge) and for the
    /// invariant test that keeps <see cref="RewriteProofs"/> in step with the registry: a new rule
    /// without an obligation entry fails the build rather than silently escaping the bridge.</summary>
    public static IReadOnlyList<string> ShippedRuleIds(ExprContext ctx) =>
        BuildRegistry(ctx).All.Select(r => r.Id).ToArray();

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
            (m, c) => Exprs.One)
        {
            Classification = RuleClassification.Universal,
        });

        // power: sqrt(x^2) under assumptions
        var w = new WildPat("w");
        reg.Register(new RewriteRule(
            "pow.sqrt-square-nonnegative",
            "power",
            new PowPat(new PowPat(w, new LiteralPat(Exprs.Integer(2))), new LiteralPat(Exprs.Rational(1, 2))),
            (m, c) => m.Get("w") is SymbolExpr sx &&
                      c.Assumptions.Ask(new SymbolPropertyAssumption(sx.Symbol, SymbolPredicate.NonNegative)) == Tristate.True,
            (m, c) => m.Get("w"))
        {
            // sqrt(w^2) = w requires w >= 0: the condition is declared, not hidden in the lambda
            ConditionBuilder = (m, c) => AssumptionSet.Empty.Add(
                m.Get("w") is SymbolExpr sx
                    ? new SymbolPropertyAssumption(sx.Symbol, SymbolPredicate.NonNegative)
                    : new ExpressionPropertyAssumption(m.Get("w"), SymbolPredicate.NonNegative)),
        });
        reg.Register(new RewriteRule(
            "pow.sqrt-square-real",
            "power",
            new PowPat(new PowPat(new WildPat("w2"), new LiteralPat(Exprs.Integer(2))), new LiteralPat(Exprs.Rational(1, 2))),
            (m, c) => m.Get("w2") is SymbolExpr sx2 &&
                      c.Assumptions.Ask(new SymbolPropertyAssumption(sx2.Symbol, SymbolPredicate.NonNegative)) != Tristate.True &&
                      c.Assumptions.Ask(new SymbolDomainAssumption(sx2.Symbol, Domain.Real)) == Tristate.True,
            (m, c) => Exprs.Function(c.Function("abs"), m.Get("w2")))
        {
            // sqrt(w^2) = |w| is only valid over the reals (over C, sqrt(z^2) != |z|)
            Classification = RuleClassification.DomainSpecific,
            ConditionBuilder = (m, c) => AssumptionSet.Empty.Add(
                new SymbolDomainAssumption(((SymbolExpr)m.Get("w2")).Symbol, Domain.Real)),
        });

        // rat: pointwise cancellation with retained side conditions.
        // x·x⁻¹ → 1 requires x ≠ 0 (x/x is undefined at 0; the constructor deliberately keeps it).
        var cu = new WildPat("cu");
        var cv = new WildPat("cv");
        reg.Register(new RewriteRule(
            "rat.cancel-x-over-x",
            "rat",
            new MulPat(new Pattern[] { cu, new PowPat(cv, new LiteralPat(Exprs.Rational(Rat.MinusOne))) }),
            (m, c) => m.Get("cu").Equals(m.Get("cv")),
            (m, c) => Exprs.One)
        {
            Classification = RuleClassification.Conditional,
            ConditionBuilder = (m, c) => NonZeroOf(m.Get("cu")),
        });
        // 0·x⁻¹ → 0 requires x ≠ 0 (0/x is undefined at 0).
        var cv2 = new WildPat("cv2");
        reg.Register(new RewriteRule(
            "rat.cancel-zero-over-x",
            "rat",
            new MulPat(new Pattern[] { new LiteralPat(Exprs.Zero), new PowPat(cv2, new LiteralPat(Exprs.Rational(Rat.MinusOne))) }),
            (m, c) => true,
            (m, c) => Exprs.Zero)
        {
            Classification = RuleClassification.Conditional,
            ConditionBuilder = (m, c) => NonZeroOf(m.Get("cv2")),
        });

        // logexp: exp(log(x)) → x holds on log's domain; the rewrite expands definedness
        // exactly like x/x → 1, so it is Conditional and carries finiteness of log(x).
        // log(exp(z)) → z is NOT universal either: over C it holds only on the principal
        // strip Im z ∈ (−π, π]. That rule fires only when membership is proven True
        // (a provably real z has Im z = 0, so the real case works).
        var lx = new WildPat("lx");
        reg.Register(new RewriteRule(
            "logexp.exp-log",
            "logexp",
            new FunPat("exp", new Pattern[] { new FunPat("log", new Pattern[] { lx }) }),
            (m, c) => true,
            (m, c) => m.Get("lx"))
        {
            Classification = RuleClassification.Conditional,
            ConditionBuilder = (m, c) => AssumptionSet.Empty.Add(new ExpressionPropertyAssumption(
                Exprs.Function(c.Function("log"), m.Get("lx")), SymbolPredicate.Finite)),
        });
        reg.Register(new RewriteRule(
            "logexp.log-exp",
            "logexp",
            new FunPat("log", new Pattern[] { new FunPat("exp", new Pattern[] { new WildPat("lx2") }) }),
            (m, c) =>
            {
                var z = m.Get("lx2");
                var im = Exprs.Function(c.Function("im"), z);
                return c.Assumptions.Ask(new IntervalAssumption(im, Exprs.Negate(Exprs.Pi), true, Exprs.Pi, false)) == Tristate.True;
            },
            (m, c) => m.Get("lx2")));

        // abs: |x|² → x² is false over C (|z|² = z·conj(z)); it holds only for provably real x.
        // |−x| → |x| is universal.
        var aw = new WildPat("aw");
        reg.Register(new RewriteRule(
            "abs.abs-square",
            "abs",
            new PowPat(new FunPat("abs", new Pattern[] { aw }), new LiteralPat(Exprs.Integer(2))),
            (m, c) => m.Get("aw") is SymbolExpr sax &&
                      c.Assumptions.Ask(new SymbolDomainAssumption(sax.Symbol, Domain.Real)) == Tristate.True,
            (m, c) => Exprs.Power(m.Get("aw"), Exprs.Integer(2)))
        {
            Classification = RuleClassification.DomainSpecific,
            ConditionBuilder = (m, c) => AssumptionSet.Empty.Add(
                new SymbolDomainAssumption(((SymbolExpr)m.Get("aw")).Symbol, Domain.Real)),
        });
        reg.Register(new RewriteRule(
            "abs.abs-neg",
            "abs",
            new FunPat("abs", new Pattern[] { new MulPat(new Pattern[] { new LiteralPat(Exprs.Rational(Rat.MinusOne)), new WildPat("aw2") }) }),
            (m, c) => true,
            (m, c) => Exprs.Function(c.Function("abs"), m.Get("aw2")))
        {
            Classification = RuleClassification.Universal,
        });

        return reg;
    }

    private static AssumptionSet NonZeroOf(Expr e) => AssumptionSet.Empty.Add(
        e is SymbolExpr sx
            ? new SymbolPropertyAssumption(sx.Symbol, SymbolPredicate.NonZero)
            : new ExpressionPropertyAssumption(e, SymbolPredicate.NonZero));
}
