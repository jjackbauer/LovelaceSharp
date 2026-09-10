using System.Collections.Immutable;

namespace Lovelace.Symbolics.Rewriting;

/// <summary>
/// Rewrite-rule classification (kernel constitution): Universal rules are mathematically
/// valid over the represented domain with no side conditions; Conditional rules fire only
/// when their preconditions are proven True; DomainSpecific rules additionally require a
/// declared domain assumption; Approximate rules change values within a tolerance and must
/// be surfaced as approximate; OptimizationOnly rules may change evaluation order or
/// floating-point error characteristics without changing the intended algorithm class.
/// </summary>
public enum RuleClassification
{
    Universal,
    Conditional,
    DomainSpecific,
    Approximate,
    OptimizationOnly,
}

/// <summary>
/// Explicit rule-applicability protocol. A rule mismatch is a VALUE, never an exception: only a
/// genuinely expected probe failure (see <see cref="RuleEvaluationException"/>) is caught, and any
/// other exception is a defect that propagates.
/// </summary>
public enum RuleApplicability
{
    /// <summary>The pattern/precondition does not hold for this node.</summary>
    NotApplicable,

    /// <summary>The rule holds unconditionally for this node.</summary>
    Applicable,

    /// <summary>The rule holds only under the conditions returned by the rule's condition builder.</summary>
    ApplicableWithConditions,

    /// <summary>Applicability cannot be decided. Unknown never licenses a rewrite in any mode.</summary>
    Unknown,
}

/// <summary>Thrown by a precondition probe for a state the rule deliberately handles as "cannot
/// decide". This is the ONLY exception the rewrite engine catches.</summary>
public sealed class RuleEvaluationException : Exception
{
    public RuleEvaluationException(string message) : base(message) { }
}

/// <summary>Optional per-attempt diagnostics (enabled only when tracing is requested).</summary>
public sealed record RuleAttemptDiagnostic(
    string RuleId,
    bool PatternMatched,
    RuleApplicability Precondition,
    string? RejectionReason);

/// <summary>One applied rewrite: machine-readable provenance for traces and falsification.</summary>
public sealed record RewriteStep(
    string RuleId,
    RuleClassification Classification,
    Expr Before,
    Expr After,
    AssumptionSet Conditions)
{
    public override string ToString() =>
        $"{RuleId} [{Classification}]: {Before} -> {After}{(Conditions.Atoms.Length > 0 ? " given " + string.Join("; ", Conditions.Atoms) : "")}";
}

// ---------------------------------------------------------------------------
// Patterns
// ---------------------------------------------------------------------------

public abstract record Pattern;

/// <summary>Matches exactly the given (wildcard-free) node, by structural equality.</summary>
public sealed record LiteralPat(Expr E) : Pattern;

/// <summary>Matches any single node; binds it to the name (first binding wins).</summary>
public sealed record WildPat(string Name) : Pattern;

/// <summary>Matches 0..n contiguous siblings inside an n-ary pattern (Add/Mul/Function args).</summary>
public sealed record SeqWildPat(string Name) : Pattern;

public sealed record AddPat(Pattern[] Terms) : Pattern;
public sealed record MulPat(Pattern[] Factors) : Pattern;
public sealed record PowPat(Pattern B, Pattern E) : Pattern;
public sealed record FunPat(string Name, Pattern[] Args) : Pattern;

public sealed class Match
{
    private readonly Dictionary<string, Expr> _binds = new(StringComparer.Ordinal);
    private readonly Dictionary<string, Expr[]> _seq = new(StringComparer.Ordinal);

    public Expr Get(string name)
    {
        if (_binds.TryGetValue(name, out var e))
            return e;
        throw new InvalidOperationException($"Pattern variable '{name}' is not bound.");
    }

    public Expr[] GetSeq(string name)
    {
        if (_seq.TryGetValue(name, out var e))
            return e;
        throw new InvalidOperationException($"Sequence variable '{name}' is not bound.");
    }

    public bool TryGet(string name, out Expr e) => _binds.TryGetValue(name, out e!);

    internal bool Bind(string name, Expr e)
    {
        if (_binds.TryGetValue(name, out var prev))
            return prev == e;
        _binds[name] = e;
        return true;
    }

    internal bool BindSeq(string name, Expr[] e)
    {
        if (_seq.TryGetValue(name, out var prev))
            return prev.SequenceEqual(e);
        _seq[name] = e;
        return true;
    }

    internal Match Clone()
    {
        var c = new Match();
        foreach (var (k, v) in _binds) c._binds[k] = v;
        foreach (var (k, v) in _seq) c._seq[k] = v;
        return c;
    }

    internal bool MergeFrom(Match other)
    {
        foreach (var (k, v) in other._binds)
        {
            if (_binds.TryGetValue(k, out var prev) && prev != v)
                return false;
            _binds[k] = v;
        }
        foreach (var (k, v) in other._seq)
        {
            if (_seq.TryGetValue(k, out var prev) && !prev.SequenceEqual(v))
                return false;
            _seq[k] = v;
        }
        return true;
    }
}

// ---------------------------------------------------------------------------
// Rules and registry
// ---------------------------------------------------------------------------

/// <summary>
/// A conditional rewrite rule with a stable id, a precondition evaluated against the
/// context assumptions, and a replacement builder.
/// </summary>
public sealed class RewriteRule
{
    public string Id { get; }
    public string Group { get; }
    public Pattern Pattern { get; }

    /// <summary>The rule's applicability decision. Never exception-based control flow.</summary>
    public Func<Match, ExprContext, RuleApplicability> Applicability { get; }

    public Func<Match, ExprContext, Expr> Replacement { get; }

    /// <summary>Semantic classification (kernel constitution). Defaults to Conditional: a rule
    /// must opt into Universal explicitly.</summary>
    public RuleClassification Classification { get; init; } = RuleClassification.Conditional;

    /// <summary>Declarative side conditions the rule's semantics depend on (for traces,
    /// falsification, and documentation).</summary>
    public AssumptionSet DeclaredConditions { get; init; } = AssumptionSet.Empty;

    /// <summary>Builds the concrete side conditions for a specific match (trace/provenance only).</summary>
    public Func<Match, ExprContext, AssumptionSet>? ConditionBuilder { get; init; }

    public RewriteRule(
        string id,
        string group,
        Pattern pattern,
        Func<Match, ExprContext, RuleApplicability> applicability,
        Func<Match, ExprContext, Expr> replacement)
    {
        Id = id;
        Group = group;
        Pattern = pattern;
        Applicability = applicability;
        Replacement = replacement;
    }

    /// <summary>Convenience overload for rules that only need a yes/no precondition.</summary>
    public RewriteRule(
        string id,
        string group,
        Pattern pattern,
        Func<Match, ExprContext, bool> precondition,
        Func<Match, ExprContext, Expr> replacement)
        : this(id, group, pattern,
            (m, c) => precondition(m, c) ? RuleApplicability.Applicable : RuleApplicability.NotApplicable,
            replacement)
    {
    }
}

public sealed class RewriteRuleRegistry
{
    private readonly List<RewriteRule> _rules = new();

    public void Register(RewriteRule rule) => _rules.Add(rule);

    public IReadOnlyList<RewriteRule> Group(string groupId) =>
        _rules.Where(r => r.Group == groupId).ToArray();

    public IReadOnlyList<RewriteRule> All => _rules;
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

/// <summary>
/// Deterministic budgeted rewriting: bottom-up traversal in canonical child order; at each
/// node rules are tried in registration order, first match whose precondition is True fires;
/// the result is rebuilt canonically and the node re-examined until fixed point or budget.
/// </summary>
public static class RewriteEngine
{
    public sealed class Budget
    {
        public int MaxSteps { get; init; } = 400;
        public int Steps { get; set; }
        public bool Exhausted => Steps >= MaxSteps;

        /// <summary>Machine-readable budget identity for structured diagnostics.</summary>
        public string Kind { get; init; } = "rewrite_steps";
    }

    public static Expr Apply(Expr e, ExprContext ctx, IReadOnlyList<RewriteRule> rules, Budget? budget = null)
        => Apply(e, ctx, rules, budget, trace: null, appliedConditions: null);

    /// <summary>Apply with optional provenance collection. Rule objects are not allocated when
    /// <paramref name="trace"/> is null, but a rule's condition builder is still consulted: the
    /// conditions are semantics, not provenance.</summary>
    public static Expr Apply(Expr e, ExprContext ctx, IReadOnlyList<RewriteRule> rules, Budget? budget, List<RewriteStep>? trace)
        => Apply(e, ctx, rules, budget, trace, appliedConditions: null);

    /// <summary>Apply with optional provenance and per-application condition sinks. The
    /// condition sink is semantic bookkeeping and can be enabled independently of the trace.</summary>
    public static Expr Apply(
        Expr e, ExprContext ctx, IReadOnlyList<RewriteRule> rules, Budget? budget,
        List<RewriteStep>? trace, List<AssumptionSet>? appliedConditions)
        => Apply(e, ctx, rules, budget, trace, appliedConditions, diagnostics: null);

    /// <summary>Apply with optional per-attempt diagnostics (only allocated by the caller when
    /// tracing is enabled, so normal simplification pays nothing).</summary>
    public static Expr Apply(
        Expr e, ExprContext ctx, IReadOnlyList<RewriteRule> rules, Budget? budget,
        List<RewriteStep>? trace, List<AssumptionSet>? appliedConditions,
        List<RuleAttemptDiagnostic>? diagnostics)
    {
        budget ??= new Budget();
        Lovelace.Abstractions.Cancellation.ThrowIfCancellationRequested();
        return Walk(e, ctx, rules, budget, trace, appliedConditions, diagnostics);
    }

    private static Expr Walk(
        Expr node, ExprContext ctx, IReadOnlyList<RewriteRule> rules, Budget budget,
        List<RewriteStep>? trace, List<AssumptionSet>? appliedConditions,
        List<RuleAttemptDiagnostic>? diagnostics)
    {
        // rebuild children bottom-up
        switch (node)
        {
            case AddExpr a:
                node = Exprs.Add(a.Terms.Select(t => Walk(t, ctx, rules, budget, trace, appliedConditions, diagnostics)));
                break;
            case MultiplyExpr m:
                node = Exprs.Multiply(m.Factors.Select(f => Walk(f, ctx, rules, budget, trace, appliedConditions, diagnostics)));
                break;
            case PowerExpr p:
                node = Exprs.Power(Walk(p.Base, ctx, rules, budget, trace, appliedConditions, diagnostics), Walk(p.Exponent, ctx, rules, budget, trace, appliedConditions, diagnostics));
                break;
            case FunctionExpr f:
                node = Exprs.Function(f.Function, f.Arguments.Select(x => Walk(x, ctx, rules, budget, trace, appliedConditions, diagnostics)).ToArray());
                break;
            case RelationExpr r:
                node = Exprs.Relation(r.Op, Walk(r.Left, ctx, rules, budget, trace, appliedConditions, diagnostics), Walk(r.Right, ctx, rules, budget, trace, appliedConditions, diagnostics));
                break;
            case PiecewiseExpr pw:
                node = Exprs.Piecewise(
                    pw.Branches.Select(b => new PiecewiseBranch(Walk(b.Guard, ctx, rules, budget, trace, appliedConditions, diagnostics), Walk(b.Value, ctx, rules, budget, trace, appliedConditions, diagnostics))),
                    Walk(pw.Otherwise, ctx, rules, budget, trace, appliedConditions, diagnostics));
                break;
            case DerivativeExpr d:
                node = Exprs.Derivative(Walk(d.Operand, ctx, rules, budget, trace, appliedConditions, diagnostics), d.Variables.ToArray());
                break;
            case IntegralExpr i:
                node = Exprs.Integral(Walk(i.Operand, ctx, rules, budget, trace, appliedConditions, diagnostics), i.Variables.ToArray());
                break;
            case AndExpr an:
                node = Exprs.And(an.Operands.Select(o => Walk(o, ctx, rules, budget, trace, appliedConditions, diagnostics)));
                break;
            case OrExpr or2:
                node = Exprs.Or(or2.Operands.Select(o => Walk(o, ctx, rules, budget, trace, appliedConditions, diagnostics)));
                break;
            case NotExpr nt:
                node = Exprs.Not(Walk(nt.Operand, ctx, rules, budget, trace, appliedConditions, diagnostics));
                break;
            case OrderExpr o:
                node = Exprs.Order(
                    Walk(o.Variable, ctx, rules, budget, trace, appliedConditions, diagnostics),
                    Walk(o.Point, ctx, rules, budget, trace, appliedConditions, diagnostics),
                    Walk(o.Degree, ctx, rules, budget, trace, appliedConditions, diagnostics));
                break;
        }

        // fixed-point rule application at this node
        while (!budget.Exhausted)
        {
            bool fired = false;
            foreach (var rule in rules)
            {
                if (budget.Exhausted)
                    break;
                var m = new Match();
                if (!TryMatch(rule.Pattern, node, m))
                {
                    diagnostics?.Add(new RuleAttemptDiagnostic(rule.Id, false, RuleApplicability.NotApplicable, "pattern did not match"));
                    continue;
                }
                RuleApplicability applicability;
                try
                {
                    applicability = rule.Applicability(m, ctx);
                }
                catch (RuleEvaluationException ex)
                {
                    // the ONLY expected exceptional state: the rule itself declared that it
                    // cannot decide. Everything else is a defect and must propagate.
                    diagnostics?.Add(new RuleAttemptDiagnostic(rule.Id, true, RuleApplicability.Unknown, ex.Message));
                    continue;
                }
                if (applicability is RuleApplicability.NotApplicable or RuleApplicability.Unknown)
                {
                    diagnostics?.Add(new RuleAttemptDiagnostic(rule.Id, true, applicability,
                        applicability == RuleApplicability.Unknown ? "applicability unknown" : "precondition false"));
                    continue;
                }
                var next = rule.Replacement(m, ctx);
                // structural equality — nodes may come from different context pools
                if (!node.Equals(next))
                {
                    // the budget counts APPLIED rewrites, not matching attempts
                    budget.Steps++;
                    var conds = rule.ConditionBuilder?.Invoke(m, ctx) ?? rule.DeclaredConditions;
                    trace?.Add(new RewriteStep(rule.Id, rule.Classification, node, next, conds));
                    appliedConditions?.Add(conds);
                    node = next;
                    fired = true;
                    break;
                }
            }
            if (!fired)
                break;
        }
        return node;
    }

    public static bool TryMatch(Pattern p, Expr e, Match m)
    {
        switch (p)
        {
            case LiteralPat lit:
                return lit.E.Equals(e);
            case WildPat w:
                return m.Bind(w.Name, e);
            case AddPat ap when e is AddExpr add:
                return MatchNary(ap.Terms, add.Terms, m, seq => seq);
            case MulPat mp when e is MultiplyExpr mul:
                return MatchNary(mp.Factors, mul.Factors, m, seq => seq);
            case PowPat pp when e is PowerExpr pow:
                return TryMatch(pp.B, pow.Base, m) && TryMatch(pp.E, pow.Exponent, m);
            case FunPat fp when e is FunctionExpr fn:
                if (fn.Function.Name != fp.Name)
                    return false;
                return MatchNary(fp.Args, fn.Arguments, m, seq => seq);
            case SeqWildPat sw:
                return m.BindSeq(sw.Name, new[] { e });
            default:
                return false;
        }
    }

    private static bool MatchNary(Pattern[] pats, ImmutableArray<Expr> items, Match m, Func<ImmutableArray<Expr>, ImmutableArray<Expr>> pick)
    {
        int seqIdx = -1;
        for (int i = 0; i < pats.Length; i++)
        {
            if (pats[i] is SeqWildPat)
            {
                if (seqIdx >= 0)
                    throw new InvalidOperationException("At most one sequence wildcard per pattern list.");
                seqIdx = i;
            }
        }
        var seqName = seqIdx >= 0 ? ((SeqWildPat)pats[seqIdx]).Name : null;

        if (seqIdx < 0)
        {
            if (pats.Length != items.Length)
                return false;
            for (int i = 0; i < pats.Length; i++)
                if (!TryMatch(pats[i], items[i], m))
                    return false;
            return true;
        }

        // try every split of the sequence wildcard, leftmost-greedy first, against a clone
        int fixedLeft = seqIdx;
        int fixedRight = pats.Length - seqIdx - 1;
        int seqLen = items.Length - fixedLeft - fixedRight;
        for (int len = 0; len <= seqLen; len++)
        {
            var trial = m.Clone();
            bool ok = true;
            for (int i = 0; i < fixedLeft; i++)
            {
                if (!TryMatch(pats[i], items[i], trial))
                {
                    ok = false;
                    break;
                }
            }
            if (ok && !trial.BindSeq(seqName!, items.Skip(seqIdx).Take(len).ToArray()))
                ok = false;
            for (int i = 0; i < fixedRight && ok; i++)
            {
                if (!TryMatch(pats[pats.Length - fixedRight + i], items[items.Length - fixedRight + i], trial))
                    ok = false;
            }
            if (ok && m.MergeFrom(trial))
                return true;
        }
        return false;
    }
}

