using System.Collections.Immutable;
using Rat = global::Lovelace.Rational.Rational;

namespace Lovelace.Symbolics;

public enum Tristate { True, False, Unknown }

public enum Domain { Integer, Rational, Real, Complex }

public enum SymbolPredicate
{
    Positive, Negative, NonNegative, NonPositive, NonZero, Even, Odd, Finite,
}

public abstract record Assumption;

public sealed record SymbolDomainAssumption(Symbol S, Domain D) : Assumption;

public sealed record SymbolPropertyAssumption(Symbol S, SymbolPredicate P) : Assumption;

public sealed record SymbolRelationAssumption(Symbol S, RelOp Op, Expr Bound) : Assumption;

public sealed record ExpressionPropertyAssumption(Expr E, SymbolPredicate P) : Assumption;

/// <summary>
/// Interval membership condition: lower &lt; e &lt; upper (open/closed ends optional). Bounds are
/// expressions (typically constants like Pi). Proved True by an identical assumed atom, or by
/// constant evaluation when e is provably constant (e.g. Im(z) = 0 for provably real z).
/// </summary>
public sealed record IntervalAssumption(Expr E, Expr? Lower, bool LowerOpen, Expr? Upper, bool UpperOpen) : Assumption;

/// <summary>Thrown when adding an assumption that contradicts the current set.</summary>
public sealed class AssumptionContradictionException : Exception
{
    public AssumptionContradictionException(string message) : base(message) { }
}

/// <summary>
/// An immutable, content-hashed set of assumptions. Queries return a tri-state
/// (<see cref="Tristate"/>): True/False only when provable/provably-negated by the set,
/// Unknown otherwise (no closed-world assumption).
/// </summary>
public sealed class AssumptionSet : IEquatable<AssumptionSet>
{
    private readonly ImmutableArray<Assumption> _atoms;
    private readonly bool _unsatisfiable;

    private AssumptionSet(ImmutableArray<Assumption> atoms, bool unsatisfiable = false)
    {
        _atoms = atoms;
        _unsatisfiable = unsatisfiable;
    }

    public static readonly AssumptionSet Empty = new(ImmutableArray<Assumption>.Empty);

    /// <summary>Sentinel for a provably contradictory condition set (no model exists).
    /// Distinct from <see cref="Empty"/>: an unsatisfiable branch is not the absence of
    /// conditions — it is a contradiction that must be pruned or reported.</summary>
    public static readonly AssumptionSet Unsatisfiable = new(ImmutableArray<Assumption>.Empty, unsatisfiable: true);

    public ImmutableArray<Assumption> Atoms => _atoms;

    /// <summary>True when this set is the <see cref="Unsatisfiable"/> sentinel.</summary>
    public bool IsUnsatisfiable => _unsatisfiable;

    public bool Contains(Assumption a) => _atoms.Contains(a);

    /// <summary>Returns a new set including <paramref name="a"/>; throws on contradiction.
    /// Adding to the Unsatisfiable sentinel stays Unsatisfiable.</summary>
    public AssumptionSet Add(Assumption a)
    {
        if (IsUnsatisfiable)
            return this;
        if (_atoms.Contains(a))
            return this;
        var next = _atoms.Add(a).Sort(static (x, y) => string.CompareOrdinal(x.ToString(), y.ToString()));
        var probe = new AssumptionSet(next);
        var negation = Negate(a);
        if (negation is not null && probe.Ask(negation) == Tristate.True)
            throw new AssumptionContradictionException(
                $"Assumption {a} contradicts the existing assumptions (its negation {negation} is provable).");
        return probe;
    }

    public bool Equals(AssumptionSet? other) =>
        other is not null && other._unsatisfiable == _unsatisfiable &&
        other._atoms.Length == _atoms.Length && _atoms.SequenceEqual(other._atoms);

    public override bool Equals(object? obj) => obj is AssumptionSet s && Equals(s);

    public override int GetHashCode()
    {
        int h = _unsatisfiable ? 0x5EED : 0;
        foreach (var a in _atoms) h = System.HashCode.Combine(h, a.GetHashCode());
        return h;
    }

    public static Assumption? Negate(Assumption a) => a switch
    {
        SymbolPropertyAssumption p => p.P switch
        {
            SymbolPredicate.Positive => p with { P = SymbolPredicate.NonPositive },
            SymbolPredicate.NonPositive => p with { P = SymbolPredicate.Positive },
            SymbolPredicate.NonNegative => p with { P = SymbolPredicate.Negative },
            SymbolPredicate.Negative => p with { P = SymbolPredicate.NonNegative },
            SymbolPredicate.Even => p with { P = SymbolPredicate.Odd },
            SymbolPredicate.Odd => p with { P = SymbolPredicate.Even },
            _ => null,
        },
        SymbolRelationAssumption r => r.Op switch
        {
            RelOp.Eq => r with { Op = RelOp.Ne },
            RelOp.Ne => r with { Op = RelOp.Eq },
            RelOp.Gt => r with { Op = RelOp.Le },
            RelOp.Le => r with { Op = RelOp.Gt },
            RelOp.Lt => r with { Op = RelOp.Ge },
            RelOp.Ge => r with { Op = RelOp.Lt },
            _ => null,
        },
        _ => null,
    };

    /// <summary>Tri-state query over the assumption set (memoized per set by the caller).
    /// Derived answers (bound reasoning, inference tables) dominate the literal
    /// negation-containment check: on a contradictory set both a query and its negation can be
    /// provable, and the positive derivation is the one that matters for contradiction
    /// detection in <see cref="Add"/>.</summary>
    public Tristate Ask(Assumption predicate)
    {
        if (_atoms.Contains(predicate))
            return Tristate.True;

        var derived = predicate switch
        {
            // ---- symbol properties: implication lattice ----
            SymbolPropertyAssumption { S: var s, P: SymbolPredicate.Positive } =>
                Inferred(Ask(new SymbolRelationAssumption(s, RelOp.Gt, Exprs.Zero))),
            SymbolPropertyAssumption { S: var s, P: SymbolPredicate.NonNegative } =>
                Or(
                    Ask(new SymbolPropertyAssumption(s, SymbolPredicate.Positive)),
                    Ask(new SymbolRelationAssumption(s, RelOp.Ge, Exprs.Zero))),
            SymbolPropertyAssumption { S: var s, P: SymbolPredicate.Negative } =>
                Ask(new SymbolRelationAssumption(s, RelOp.Lt, Exprs.Zero)),
            SymbolPropertyAssumption { S: var s, P: SymbolPredicate.NonPositive } =>
                Or(
                    Ask(new SymbolPropertyAssumption(s, SymbolPredicate.Negative)),
                    Ask(new SymbolRelationAssumption(s, RelOp.Le, Exprs.Zero))),
            SymbolPropertyAssumption { S: var s, P: SymbolPredicate.NonZero } =>
                Or(
                    Ask(new SymbolPropertyAssumption(s, SymbolPredicate.Positive)),
                    Ask(new SymbolPropertyAssumption(s, SymbolPredicate.Negative)),
                    Ask(new SymbolPropertyAssumption(s, SymbolPredicate.Odd)),
                    Ask(new SymbolRelationAssumption(s, RelOp.Ne, Exprs.Zero))),
            SymbolPropertyAssumption { P: SymbolPredicate.Finite } =>
                Or(
                    Ask(new SymbolDomainAssumption(((SymbolPropertyAssumption)predicate).S, Domain.Real)),
                    Ask(new SymbolDomainAssumption(((SymbolPropertyAssumption)predicate).S, Domain.Rational)),
                    Ask(new SymbolDomainAssumption(((SymbolPropertyAssumption)predicate).S, Domain.Integer))),

            // ---- domain lattice (only narrower domains infer wider ones) ----
            // Even/Odd imply Integer (never the reverse)
            SymbolDomainAssumption { S: var s, D: Domain.Integer } =>
                Or(
                    Ask(new SymbolPropertyAssumption(s, SymbolPredicate.Even)),
                    Ask(new SymbolPropertyAssumption(s, SymbolPredicate.Odd))),
            SymbolDomainAssumption { S: var s, D: Domain.Complex } =>
                Or(
                    Ask(new SymbolDomainAssumption(s, Domain.Real)),
                    Ask(new SymbolDomainAssumption(s, Domain.Rational)),
                    Ask(new SymbolDomainAssumption(s, Domain.Integer))),
            SymbolDomainAssumption { S: var s, D: Domain.Real } =>
                Or(
                    Ask(new SymbolDomainAssumption(s, Domain.Integer)),
                    Ask(new SymbolDomainAssumption(s, Domain.Rational))),
            SymbolDomainAssumption { S: var s, D: Domain.Rational } =>
                Ask(new SymbolDomainAssumption(s, Domain.Integer)),

            // ---- expression properties (function-level inference) ----
            ExpressionPropertyAssumption { P: SymbolPredicate.NonNegative } =>
                AskExprPredicate(predicate, SymbolPredicate.NonNegative),
            ExpressionPropertyAssumption { P: SymbolPredicate.Positive } =>
                AskExprPredicate(predicate, SymbolPredicate.Positive),
            ExpressionPropertyAssumption { P: SymbolPredicate.NonZero } =>
                AskExprPredicate(predicate, SymbolPredicate.NonZero),
            ExpressionPropertyAssumption { P: SymbolPredicate.Finite } =>
                AskExprPredicate(predicate, SymbolPredicate.Finite),
            IntervalAssumption i => AskInterval(i),

            // ---- symbol relations: constant-bound reasoning ----
            SymbolRelationAssumption r => AskRelation(r),

            _ => Tristate.Unknown,
        };
        if (derived == Tristate.True)
            return Tristate.True;

        var negation = Negate(predicate);
        if (negation is not null && _atoms.Contains(negation))
            return Tristate.False;
        return derived;
    }

    /// <summary>
    /// Bound reasoning over assumed symbol relations: a relation query is True when some
    /// assumed constant-bound atom on the same symbol implies it, False when every comparable
    /// assumed atom implies its negation, and Unknown otherwise (e.g. x &gt; 5 proves x &gt; 0
    /// and refutes x &lt; 5, while x &gt; 5 says nothing about x &lt; 7).
    /// </summary>
    private Tristate AskRelation(SymbolRelationAssumption q)
    {
        if (Exprs.NumericToRational(q.Bound) is null)
            return Tristate.Unknown;
        bool anyImplies = false, allRefute = true, any = false;
        foreach (var atom in _atoms)
        {
            if (atom is not SymbolRelationAssumption sr || sr.S.Name != q.S.Name)
                continue;
            if (Exprs.NumericToRational(sr.Bound) is null)
                continue;
            any = true;
            var (implies, refutes) = CompareBounds(sr, q);
            anyImplies |= implies;
            allRefute &= refutes;
        }
        if (!any)
            return Tristate.Unknown;
        if (anyImplies)
            return Tristate.True;
        return allRefute ? Tristate.False : Tristate.Unknown;
    }

    /// <summary>Does the value set of the assumed relation imply / refute the query relation?</summary>
    private static (bool Implies, bool Refutes) CompareBounds(SymbolRelationAssumption a, SymbolRelationAssumption q)
    {
        var ab = Exprs.NumericToRational(a.Bound)!;
        var qb = Exprs.NumericToRational(q.Bound)!;
        return a.Op switch
        {
            RelOp.Eq => (Satisfies(ab, q.Op, qb), !Satisfies(ab, q.Op, qb)),
            RelOp.Ne => (q.Op == RelOp.Ne && ab == qb, q.Op == RelOp.Eq && ab == qb),
            RelOp.Gt => (ImpliesLower(ab, strict: true, q.Op, qb), RefutesLower(ab, strict: true, q.Op, qb)),
            RelOp.Ge => (ImpliesLower(ab, strict: false, q.Op, qb), RefutesLower(ab, strict: false, q.Op, qb)),
            RelOp.Lt => (ImpliesUpper(ab, strict: true, q.Op, qb), RefutesUpper(ab, strict: true, q.Op, qb)),
            RelOp.Le => (ImpliesUpper(ab, strict: false, q.Op, qb), RefutesUpper(ab, strict: false, q.Op, qb)),
            _ => (false, false),
        };
    }

    private static bool Satisfies(Rat v, RelOp op, Rat b) => op switch
    {
        RelOp.Eq => v == b,
        RelOp.Ne => v != b,
        RelOp.Gt => v > b,
        RelOp.Ge => v >= b,
        RelOp.Lt => v < b,
        RelOp.Le => v <= b,
        _ => false,
    };

    /// <summary>Does a lower bound (x &gt; ab if strict, else x ≥ ab) imply the query?</summary>
    private static bool ImpliesLower(Rat ab, bool strict, RelOp q, Rat qb) => q switch
    {
        RelOp.Gt => strict ? ab >= qb : ab > qb,
        RelOp.Ge => ab >= qb,
        RelOp.Ne => strict ? ab >= qb : ab > qb,
        _ => false,   // a lower bound never implies an upper bound or an equality
    };

    /// <summary>Does a lower bound refute the query (every value above ab violates it)?
    /// A ray never refutes a ≠ query: it always contains values different from qb.</summary>
    private static bool RefutesLower(Rat ab, bool strict, RelOp q, Rat qb) => q switch
    {
        RelOp.Le or RelOp.Lt or RelOp.Eq => strict ? ab >= qb : ab > qb,
        _ => false,
    };

    /// <summary>Does an upper bound (x &lt; ab if strict, else x ≤ ab) imply the query?</summary>
    private static bool ImpliesUpper(Rat ab, bool strict, RelOp q, Rat qb) => q switch
    {
        RelOp.Lt => strict ? ab <= qb : ab < qb,
        RelOp.Le => ab <= qb,
        RelOp.Ne => strict ? ab <= qb : ab < qb,
        _ => false,   // an upper bound never implies a lower bound or an equality
    };

    /// <summary>Does an upper bound refute the query (every value below ab violates it)?
    /// A ray never refutes a ≠ query: it always contains values different from qb.</summary>
    private static bool RefutesUpper(Rat ab, bool strict, RelOp q, Rat qb) => q switch
    {
        RelOp.Ge or RelOp.Gt or RelOp.Eq => strict ? ab <= qb : ab < qb,
        _ => false,
    };

    private static Tristate Inferred(Tristate t) => t;

    private static Tristate Or(params Tristate[] states)
    {
        bool anyUnknown = false;
        foreach (var s in states)
        {
            if (s == Tristate.True) return Tristate.True;
            if (s == Tristate.Unknown) anyUnknown = true;
        }
        return anyUnknown ? Tristate.Unknown : Tristate.False;
    }

    private Tristate AskExprPredicate(Assumption predicate, SymbolPredicate p)
    {
        var e = ((ExpressionPropertyAssumption)predicate).E;
        // a bare symbol delegates to its property atom
        if (e is SymbolExpr ssx)
            return Ask(new SymbolPropertyAssumption(ssx.Symbol, p));
        // abs(x) is nonnegative; positive iff x ≠ 0 — valid over C as well
        if (e is FunctionExpr f && f.Function.Name == "abs")
        {
            if (p == SymbolPredicate.NonNegative) return Tristate.True;
            if (p == SymbolPredicate.Positive || p == SymbolPredicate.NonZero)
            {
                var arg = f.Arguments[0];
                return arg is SymbolExpr sx
                    ? Ask(new SymbolPropertyAssumption(sx.Symbol, SymbolPredicate.NonZero))
                    : Tristate.Unknown;
            }
        }
        // exp(x) is positive (and nonzero) — only when x is provably real: over C, exp(z) is
        // not an ordered real (exp(i·π) = -1)
        if (e is FunctionExpr fe && fe.Function.Name == "exp" && p is SymbolPredicate.Positive or SymbolPredicate.NonZero)
            return ProvablyReal(fe.Arguments[0]) ? Tristate.True : Tristate.Unknown;
        // log(f) is finite (defined) where f > 0 (principal branch)
        if (e is FunctionExpr lf && lf.Function.Name == "log" && lf.Arguments.Length == 1 && p == SymbolPredicate.Finite)
            return Ask(new ExpressionPropertyAssumption(lf.Arguments[0], SymbolPredicate.Positive));
        // sqrt(x) is nonnegative (principal branch) — only under provable real nonnegativity
        if (e is PowerExpr pw && pw.Exponent is RationalConstantExpr r && r.Value == RatOneHalf && p == SymbolPredicate.NonNegative)
        {
            var b = pw.Base;
            if (!ProvablyReal(b))
                return Tristate.Unknown;
            if (b is SymbolExpr sb)
                return Ask(new SymbolPropertyAssumption(sb.Symbol, SymbolPredicate.NonNegative));
            if (Exprs.NumericToRational(b) is { } bv)
                return bv.IsNegative ? Tristate.False : Tristate.True;
            return Tristate.Unknown;
        }
        // x^even is nonnegative — only for provably real bases (over C, z^2 is not ordered)
        if (e is PowerExpr pw2 && pw2.Exponent is RationalConstantExpr r2 && r2.Value.IsInteger && r2.Value.ToInteger().IsEvenInteger() && p == SymbolPredicate.NonNegative)
            return ProvablyReal(pw2.Base) ? Tristate.True : Tristate.Unknown;
        return Tristate.Unknown;
    }

    /// <summary>True when the expression is provably real-valued under the current assumptions.</summary>
    internal bool ProvablyReal(Expr e) => e switch
    {
        IntegerConstantExpr or RationalConstantExpr or RealConstantExpr => true,
        ComplexConstantExpr => false,
        NamedConstantExpr n => n.Constant != NamedConstant.I,
        SymbolExpr s => Ask(new SymbolDomainAssumption(s.Symbol, Domain.Real)) == Tristate.True,
        AddExpr a => a.Terms.All(ProvablyReal),
        MultiplyExpr m => m.Factors.All(ProvablyReal),
        PowerExpr p when p.Exponent is RationalConstantExpr re && re.Value.IsInteger => ProvablyReal(p.Base),
        FunctionExpr f => f.Function.Name is "re" or "im" or "abs" or "sign" or "floor" or "ceil"
            && f.Arguments.Length > 0 && ProvablyReal(f.Arguments[0]),
        _ => false,
    };

    private Tristate AskInterval(IntervalAssumption i)
    {
        // Im(z) = 0 when z is provably real: membership reduces to a constant check
        if (i.E is FunctionExpr f && f.Function.Name == "im" && f.Arguments.Length == 1 && ProvablyReal(f.Arguments[0]))
        {
            var zero = Exprs.Zero;
            bool ok = true;
            if (i.Lower is { } lo && Exprs.NumericToRational(lo) is { } loV)
                ok &= i.LowerOpen ? loV < Rat.Zero : loV <= Rat.Zero;
            if (i.Upper is { } up && Exprs.NumericToRational(up) is { } upV)
                ok &= i.UpperOpen ? Rat.Zero < upV : Rat.Zero <= upV;
            return ok ? Tristate.True : Tristate.False;
        }
        return Tristate.Unknown;
    }

    private static readonly Rat RatOneHalf = Rat.From(1, 2);
}

/// <summary>Structural best-known domain of an expression given a set of assumptions.</summary>
public static class Domains
{
    public static Domain DomainOf(Expr e, ExprContext ctx)
    {
        switch (e)
        {
            case IntegerConstantExpr: return Domain.Integer;
            case RationalConstantExpr: return Domain.Rational;
            case RealConstantExpr: return Domain.Real;
            case ComplexConstantExpr: return Domain.Complex;
            case NamedConstantExpr n: return n.Constant == NamedConstant.I ? Domain.Complex : Domain.Real;
            case SymbolExpr s: return DomainOfSymbol(s.Symbol, ctx);
            case AddExpr a: return Combine(a.Terms, ctx);
            case MultiplyExpr m: return Combine(m.Factors, ctx);
            case PowerExpr p:
            {
                var bd = DomainOf(p.Base, ctx);
                if (p.Exponent is RationalConstantExpr re && re.Value.IsInteger)
                {
                    if (re.Value.IsNegative)
                    {
                        // negative integer powers are rational-valued where defined:
                        // 1/x for x ∈ ℤ is rational, never integer (x^0 = 1 keeps the base domain)
                        return bd switch
                        {
                            Domain.Integer or Domain.Rational => Domain.Rational,
                            Domain.Real => Domain.Real,
                            _ => Domain.Complex,
                        };
                    }
                    return bd;
                }
                // non-integer exponent: the principal value is real only under provable
                // real nonnegativity of the base; otherwise the value is complex-valued
                if (bd != Domain.Complex && ProvablyReal(p.Base, ctx))
                {
                    if (p.Base is SymbolExpr sb &&
                        ctx.Assumptions.Ask(new SymbolPropertyAssumption(sb.Symbol, SymbolPredicate.NonNegative)) == Tristate.True)
                        return Domain.Real;
                    if (Exprs.NumericToRational(p.Base) is { } bv)
                        return bv.IsNegative ? Domain.Complex : Domain.Real;
                }
                return Domain.Complex;
            }
            case FunctionExpr f: return DomainOfFunction(f, ctx);
            case PiecewiseExpr pw:
            {
                var d = DomainOf(pw.Otherwise, ctx);
                foreach (var br in pw.Branches)
                    d = Max(d, DomainOf(br.Value, ctx));
                return d;
            }
            case DerivativeExpr d: return DomainOf(d.Operand, ctx);
            case IntegralExpr i: return DomainOf(i.Operand, ctx);
            case RootOfExpr: return Domain.Real;
            default: return Domain.Complex;
        }
    }

    private static Domain Combine(ImmutableArray<Expr> items, ExprContext ctx)
    {
        var d = Domain.Integer;
        foreach (var i in items)
            d = Max(d, DomainOf(i, ctx));
        return d;
    }

    public static Domain Max(Domain a, Domain b) => (Domain)Math.Max((int)a, (int)b);

    /// <summary>True when the expression is provably real-valued under the context assumptions.</summary>
    public static bool ProvablyReal(Expr e, ExprContext ctx) => e switch
    {
        IntegerConstantExpr or RationalConstantExpr or RealConstantExpr => true,
        ComplexConstantExpr => false,
        NamedConstantExpr n => n.Constant != NamedConstant.I,
        SymbolExpr s => ctx.Assumptions.Ask(new SymbolDomainAssumption(s.Symbol, Domain.Real)) == Tristate.True,
        AddExpr a => a.Terms.All(t => ProvablyReal(t, ctx)),
        MultiplyExpr m => m.Factors.All(t => ProvablyReal(t, ctx)),
        PowerExpr p when p.Exponent is RationalConstantExpr re && re.Value.IsInteger => ProvablyReal(p.Base, ctx),
        FunctionExpr f => f.Function.Name is "re" or "im" or "abs" or "sign" or "floor" or "ceil"
            && f.Arguments.Length > 0 && ProvablyReal(f.Arguments[0], ctx),
        _ => false,
    };

    public static Domain DomainOfSymbol(Symbol s, ExprContext ctx)
    {
        foreach (var atom in ctx.Assumptions.Atoms)
        {
            if (atom is SymbolDomainAssumption d && d.S.Name == s.Name)
                return d.D;
        }
        return Domain.Complex;   // unconstrained symbols range over the complex field
    }

    private static Domain DomainOfFunction(FunctionExpr f, ExprContext ctx)
    {
        var arg = f.Arguments.Length > 0 ? DomainOf(f.Arguments[0], ctx) : Domain.Real;
        if (f.Function.Name == "abs")
            return arg == Domain.Complex ? Domain.Real : Domain.Real;
        return f.Function.Name switch
        {
            // transcendental functions of real arguments produce real (transcendental) values —
            // never the argument's narrower domain (sin(integer) is not an integer)
            "sin" or "cos" or "tan" or "exp" or "log" or "sinh" or "cosh" or "tanh"
                or "asin" or "acos" or "atan" or "asinh" or "acosh" or "atanh"
                => arg == Domain.Complex ? Domain.Complex : Domain.Real,
            // floor/ceil of a real argument stay Real (the evaluator returns Real); sign of a
            // real argument is an exact integer value
            "floor" or "ceil" => arg == Domain.Complex ? Domain.Complex : (arg == Domain.Integer ? Domain.Integer : Domain.Real),
            "re" or "im" => Domain.Real,
            "sign" => arg == Domain.Complex ? Domain.Complex : Domain.Integer,
            "min" or "max" => Combine(f.Arguments, ctx),
            _ => arg == Domain.Complex ? Domain.Complex : Domain.Real,
        };
    }
}
