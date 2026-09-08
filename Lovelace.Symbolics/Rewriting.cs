using System.Collections.Immutable;

namespace Lovelace.Symbolics.Rewriting;

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
    public Func<Match, ExprContext, bool> Precondition { get; }
    public Func<Match, ExprContext, Expr> Replacement { get; }

    public RewriteRule(
        string id,
        string group,
        Pattern pattern,
        Func<Match, ExprContext, bool> precondition,
        Func<Match, ExprContext, Expr> replacement)
    {
        Id = id;
        Group = group;
        Pattern = pattern;
        Precondition = precondition;
        Replacement = replacement;
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
    }

    public static Expr Apply(Expr e, ExprContext ctx, IReadOnlyList<RewriteRule> rules, Budget? budget = null)
    {
        budget ??= new Budget();
        return Walk(e, ctx, rules, budget);
    }

    private static Expr Walk(Expr node, ExprContext ctx, IReadOnlyList<RewriteRule> rules, Budget budget)
    {
        // rebuild children bottom-up
        switch (node)
        {
            case AddExpr a:
                node = Exprs.Add(a.Terms.Select(t => Walk(t, ctx, rules, budget)));
                break;
            case MultiplyExpr m:
                node = Exprs.Multiply(m.Factors.Select(f => Walk(f, ctx, rules, budget)));
                break;
            case PowerExpr p:
                node = Exprs.Power(Walk(p.Base, ctx, rules, budget), Walk(p.Exponent, ctx, rules, budget));
                break;
            case FunctionExpr f:
                node = Exprs.Function(f.Function, f.Arguments.Select(x => Walk(x, ctx, rules, budget)).ToArray());
                break;
            case RelationExpr r:
                node = Exprs.Relation(r.Op, Walk(r.Left, ctx, rules, budget), Walk(r.Right, ctx, rules, budget));
                break;
            case PiecewiseExpr pw:
                node = Exprs.Piecewise(
                    pw.Branches.Select(b => new PiecewiseBranch(Walk(b.Guard, ctx, rules, budget), Walk(b.Value, ctx, rules, budget))),
                    Walk(pw.Otherwise, ctx, rules, budget));
                break;
            case DerivativeExpr d:
                node = Exprs.Derivative(Walk(d.Operand, ctx, rules, budget), d.Variables.ToArray());
                break;
            case IntegralExpr i:
                node = Exprs.Integral(Walk(i.Operand, ctx, rules, budget), i.Variables.ToArray());
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
                    continue;
                bool ok;
                try
                {
                    ok = rule.Precondition(m, ctx);
                }
                catch (Exception)
                {
                    ok = false;
                }
                if (!ok)
                    continue;
                var next = rule.Replacement(m, ctx);
                budget.Steps++;
                if (next != node)
                {
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
                return lit.E == e;
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
