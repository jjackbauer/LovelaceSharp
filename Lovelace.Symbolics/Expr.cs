using System.Collections.Immutable;
using Int = global::Lovelace.Integer.Integer;
using Rat = global::Lovelace.Rational.Rational;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Symbolics;

public enum NodeKind
{
    Symbol, IntegerConstant, RationalConstant, RealConstant, ComplexConstant,
    NamedConstant, Add, Multiply, Power, Function, Relation, Piecewise,
    Derivative, Integral, RootOf,
}

public enum RelOp { Eq, Ne, Lt, Le, Gt, Ge }

public enum NamedConstant { Pi, E, I, Infinity }

/// <summary>A Piecewise branch: value applies while guard holds (first-match-wins semantics).</summary>
public sealed record PiecewiseBranch(Expr Guard, Expr Value);

/// <summary>
/// A canonical approximate real literal: <c>Digits × 10^Exponent10</c> with no trailing
/// fractional zeros. Identity/ordering are defined on this pair — never on Real.ToString.
/// </summary>
public readonly struct RealLiteral : IEquatable<RealLiteral>
{
    public Int Digits { get; }
    public int Exponent10 { get; }

    public RealLiteral(Int digits, int exponent10)
    {
        // normalize: strip trailing decimal zeros when exponent < 0
        if (!Int.IsZero(digits) && exponent10 < 0)
        {
            while (!Int.IsZero(digits) && (digits % new Int(10L)) == Int.Zero)
            {
                digits = digits / new Int(10L);
                exponent10++;
            }
        }
        Digits = digits;
        Exponent10 = exponent10;
    }

    public static RealLiteral FromLong(long value) => new(new Int(value), 0);

    public static RealLiteral Parse(string text)
    {
        // plain decimal: "-3.14", "2", "0.5"
        var neg = text.StartsWith('-');
        if (neg) text = text[1..];
        var dot = text.IndexOf('.');
        int exp = 0;
        string digits;
        if (dot < 0)
        {
            digits = text;
        }
        else
        {
            digits = text[..dot] + text[(dot + 1)..];
            exp = -(text.Length - dot - 1);
        }
        var d = Int.Parse(digits, null);
        if (neg) d = d.Negate();
        return new RealLiteral(d, exp);
    }

    public static RealLiteral FromReal(Rl value)
    {
        var s = value.ToString();
        if (s.Contains('('))
            throw new InvalidOperationException("Periodic Real values convert to Rational, not RealLiteral.");
        return Parse(s);
    }

    /// <summary>
    /// Exact conversion from the Real's full-precision magnitude and exponent — never the
    /// display-truncated string (ToString respects the ambient display scope).
    /// </summary>
    public static RealLiteral FromRealExact(Rl value)
    {
        var d = new Int(value.ToNatural());
        if (Rl.IsNegative(value))
            d = d.Negate();
        long exp = value.Exponent;
        return new RealLiteral(d, (int)Math.Clamp(exp, int.MinValue, int.MaxValue));
    }

    /// <summary>Exact value as a rational (finite decimal ⇒ exact).</summary>
    public Rat ToRational()
    {
        var r = Rat.From(Digits);
        if (Exponent10 >= 0)
        {
            var ten = Rat.FromLong(10);
            for (int i = 0; i < Exponent10; i++)
                r = r * ten;
            return r;
        }
        else
        {
            var ten = Rat.FromLong(10);
            for (int i = 0; i < -Exponent10; i++)
                r = r / ten;
            return r;
        }
    }

    public static RealLiteral FromRational(Rat value, int maxFractionalDigits = 64)
    {
        // finite decimal approximation of a rational (used only in the approximate tier)
        var s = value.ToDecimalString(maxFractionalDigits);
        return Parse(s);
    }

    public Rl ToReal() => Rl.Parse(ToString(), null);

    public bool Equals(RealLiteral other) => Digits == other.Digits && Exponent10 == other.Exponent10;

    public static bool operator ==(RealLiteral a, RealLiteral b) => a.Equals(b);
    public static bool operator !=(RealLiteral a, RealLiteral b) => !a.Equals(b);

    public override bool Equals(object? obj) => obj is RealLiteral r && Equals(r);

    public override int GetHashCode() => HashCode.Combine(Digits, Exponent10);

    public int CompareTo(RealLiteral other) => ToRational().CompareTo(other.ToRational());

    public override string ToString()
    {
        var sign = Int.IsNegative(Digits) ? "-" : "";
        var abs = Int.Abs(Digits).ToString();
        if (Exponent10 == 0)
            return sign + abs;
        if (Exponent10 > 0)
            return sign + abs + new string('0', Exponent10);
        var point = abs.Length + Exponent10;
        if (point > 0)
            return sign + abs[..point] + "." + abs[point..];
        return sign + "0." + new string('0', -point) + abs;
    }
}

/// <summary>
/// Immutable symbolic expression DAG. Equality is structural; the canonical factories
/// (Exprs) hash-cons nodes through an <see cref="ExprContext"/>, so nodes obtained from
/// factories with equal structure are reference-equal.
/// </summary>
public abstract class Expr : IEquatable<Expr>
{
    internal int _hash;
    internal int _nodeCount;
    internal bool _isExact;

    public NodeKind Kind { get; internal init; }

    /// <summary>Cached structural hash (canonical child order only).</summary>
    public int StructuralHash => _hash;

    /// <summary>Number of nodes in the DAG subtree (budgets use this).</summary>
    public int NodeCount => _nodeCount;

    /// <summary>True when the subtree contains no approximate RealConstant and no transcendental value.</summary>
    public bool IsExact => _isExact;

    public abstract bool Equals(Expr? other);

    // ergonomic lifts into the canonical constructors
    public static implicit operator Expr(Symbol s) => Exprs.Symbol(s);
    public static implicit operator Expr(long v) => Exprs.Integer(v);
    public static implicit operator Expr(int v) => Exprs.Integer(v);
    public static implicit operator Expr(Rat r) => Exprs.Rational(r);

    public sealed override bool Equals(object? obj) => obj is Expr e && Equals(e);
    public sealed override int GetHashCode() => _hash;

    internal static int Combine(int kind, params int[] parts)
    {
        var h = new HashCode();
        h.Add(kind);
        foreach (var p in parts)
            h.Add(p);
        return h.ToHashCode();
    }
}

public sealed class SymbolExpr : Expr
{
    public Symbol Symbol { get; }
    internal SymbolExpr(Symbol symbol) { Symbol = symbol; Kind = NodeKind.Symbol; }
    public override bool Equals(Expr? other) => other is SymbolExpr s && s.Symbol.Name == Symbol.Name;
}

public sealed class IntegerConstantExpr : Expr
{
    public Int Value { get; }
    internal IntegerConstantExpr(Int value) { Value = value; Kind = NodeKind.IntegerConstant; }
    public override bool Equals(Expr? other) => other is IntegerConstantExpr i && i.Value == Value;
}

public sealed class RationalConstantExpr : Expr
{
    public Rat Value { get; }
    internal RationalConstantExpr(Rat value) { Value = value; Kind = NodeKind.RationalConstant; }
    public override bool Equals(Expr? other) => other is RationalConstantExpr r && r.Value == Value;
}

public sealed class RealConstantExpr : Expr
{
    public RealLiteral Value { get; }
    internal RealConstantExpr(RealLiteral value) { Value = value; Kind = NodeKind.RealConstant; }
    public override bool Equals(Expr? other) => other is RealConstantExpr r && r.Value == Value;
}

public sealed class ComplexConstantExpr : Expr
{
    public Rat Re { get; }
    public Rat Im { get; }
    internal ComplexConstantExpr(Rat re, Rat im) { Re = re; Im = im; Kind = NodeKind.ComplexConstant; }
    public override bool Equals(Expr? other) => other is ComplexConstantExpr c && c.Re == Re && c.Im == Im;
}

public sealed class NamedConstantExpr : Expr
{
    public NamedConstant Constant { get; }
    internal NamedConstantExpr(NamedConstant constant) { Constant = constant; Kind = NodeKind.NamedConstant; }
    public override bool Equals(Expr? other) => other is NamedConstantExpr n && n.Constant == Constant;
}

public sealed class AddExpr : Expr
{
    public ImmutableArray<Expr> Terms { get; }
    internal AddExpr(ImmutableArray<Expr> terms) { Terms = terms; Kind = NodeKind.Add; }
    public override bool Equals(Expr? other) =>
        other is AddExpr a && a.Terms.Length == Terms.Length && Terms.SequenceEqual(a.Terms);
}

public sealed class MultiplyExpr : Expr
{
    public ImmutableArray<Expr> Factors { get; }
    internal MultiplyExpr(ImmutableArray<Expr> factors) { Factors = factors; Kind = NodeKind.Multiply; }
    public override bool Equals(Expr? other) =>
        other is MultiplyExpr m && m.Factors.Length == Factors.Length && Factors.SequenceEqual(m.Factors);
}

public sealed class PowerExpr : Expr
{
    public Expr Base { get; }
    public Expr Exponent { get; }
    internal PowerExpr(Expr b, Expr e) { Base = b; Exponent = e; Kind = NodeKind.Power; }
    public override bool Equals(Expr? other) =>
        other is PowerExpr p && p.Base == Base && p.Exponent == Exponent;
}

public sealed class FunctionExpr : Expr
{
    public FunctionId Function { get; }
    public ImmutableArray<Expr> Arguments { get; }
    internal FunctionExpr(FunctionId function, ImmutableArray<Expr> arguments)
    { Function = function; Arguments = arguments; Kind = NodeKind.Function; }
    public override bool Equals(Expr? other) =>
        other is FunctionExpr f && f.Function.Name == Function.Name &&
        f.Arguments.Length == Arguments.Length && Arguments.SequenceEqual(f.Arguments);
}

public sealed class RelationExpr : Expr
{
    public RelOp Op { get; }
    public Expr Left { get; }
    public Expr Right { get; }
    internal RelationExpr(RelOp op, Expr left, Expr right)
    { Op = op; Left = left; Right = right; Kind = NodeKind.Relation; }
    public override bool Equals(Expr? other) =>
        other is RelationExpr r && r.Op == Op && r.Left == Left && r.Right == Right;
}

public sealed class PiecewiseExpr : Expr
{
    public ImmutableArray<PiecewiseBranch> Branches { get; }
    public Expr Otherwise { get; }
    internal PiecewiseExpr(ImmutableArray<PiecewiseBranch> branches, Expr otherwise)
    { Branches = branches; Otherwise = otherwise; Kind = NodeKind.Piecewise; }
    public override bool Equals(Expr? other) =>
        other is PiecewiseExpr p && p.Otherwise == Otherwise &&
        p.Branches.Length == Branches.Length && Branches.SequenceEqual(p.Branches);
}

public sealed class DerivativeExpr : Expr
{
    public Expr Operand { get; }
    public ImmutableArray<Symbol> Variables { get; }
    internal DerivativeExpr(Expr operand, ImmutableArray<Symbol> variables)
    { Operand = operand; Variables = variables; Kind = NodeKind.Derivative; }
    public override bool Equals(Expr? other) =>
        other is DerivativeExpr d && d.Operand == Operand && d.Variables.SequenceEqual(Variables);
}

public sealed class IntegralExpr : Expr
{
    public Expr Operand { get; }
    public ImmutableArray<Symbol> Variables { get; }
    internal IntegralExpr(Expr operand, ImmutableArray<Symbol> variables)
    { Operand = operand; Variables = variables; Kind = NodeKind.Integral; }
    public override bool Equals(Expr? other) =>
        other is IntegralExpr i && i.Operand == Operand && i.Variables.SequenceEqual(Variables);
}

/// <summary>An algebraic number: the <see cref="RootIndex"/>-th root of a square-free defining polynomial.</summary>
public sealed class RootOfExpr : Expr
{
    public Polynomial DefiningPolynomial { get; }
    public int RootIndex { get; }
    internal RootOfExpr(Polynomial definingPolynomial, int rootIndex)
    { DefiningPolynomial = definingPolynomial; RootIndex = rootIndex; Kind = NodeKind.RootOf; }
    public override bool Equals(Expr? other) =>
        other is RootOfExpr r && r.RootIndex == RootIndex && r.DefiningPolynomial.Equals(DefiningPolynomial);
}
