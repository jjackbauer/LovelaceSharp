using System.Collections.Concurrent;
using Rat = global::Lovelace.Rational.Rational;

namespace Lovelace.Symbolics;

/// <summary>
/// An interned symbol identity: (dense id, canonical name). Equality is by canonical name
/// (ordinal), so identity agrees across contexts.
/// </summary>
public readonly struct Symbol : IEquatable<Symbol>, IComparable<Symbol>
{
    public int Id { get; }
    public string Name { get; }

    internal Symbol(int id, string name) { Id = id; Name = name; }

    public bool Equals(Symbol other) => string.Equals(Name, other.Name, StringComparison.Ordinal);
    public override bool Equals(object? obj) => obj is Symbol s && Equals(s);
    public override int GetHashCode() => StringComparer.Ordinal.GetHashCode(Name);
    public int CompareTo(Symbol other) => string.CompareOrdinal(Name, other.Name);
    public override string ToString() => Name;
}

/// <summary>An interned function identity: (dense id, canonical name); equality by name.</summary>
public readonly struct FunctionId : IEquatable<FunctionId>, IComparable<FunctionId>
{
    public int Id { get; }
    public string Name { get; }

    internal FunctionId(int id, string name) { Id = id; Name = name; }

    public bool Equals(FunctionId other) => string.Equals(Name, other.Name, StringComparison.Ordinal);
    public override bool Equals(object? obj) => obj is FunctionId f && Equals(f);
    public override int GetHashCode() => StringComparer.Ordinal.GetHashCode(Name);
    public int CompareTo(FunctionId other) => string.CompareOrdinal(Name, other.Name);
    public override string ToString() => Name;
}

/// <summary>
/// Per-context kernel state: the symbol/function tables, the hash-consing node pool, the
/// assumption set, and the function registry. Hosts create one context per engine/session.
/// </summary>
public sealed class ExprContext
{
    private readonly ConcurrentDictionary<string, int> _symbolIds = new(StringComparer.Ordinal);
    private readonly ConcurrentDictionary<int, string> _symbolNames = new();
    private readonly ConcurrentDictionary<string, int> _fnIds = new(StringComparer.Ordinal);
    private readonly ConcurrentDictionary<int, string> _fnNames = new();
    private readonly ConcurrentDictionary<Expr, Expr> _pool = new();

    /// <summary>The active assumption set (immutable; replace to extend).</summary>
    public AssumptionSet Assumptions { get; set; } = AssumptionSet.Empty;

    /// <summary>Registered function definitions (name → definition).</summary>
    public FunctionRegistry Functions { get; } = new();

    public ExprContext() => CoreFunctions.Register(this);

    /// <summary>Interns a symbol by canonical name.</summary>
    public Symbol Symbol(string name)
    {
        int id = _symbolIds.GetOrAdd(name, static (n, d) => d.Count, _symbolIds);
        _symbolNames[id] = name;
        return new Symbol(id, name);
    }

    public Symbol Symbol(int id) => new(id, _symbolNames[id]);

    /// <summary>Interns a function identity by canonical name.</summary>
    public FunctionId Function(string name)
    {
        int id = _fnIds.GetOrAdd(name, static (n, d) => d.Count, _fnIds);
        _fnNames[id] = name;
        return new FunctionId(id, name);
    }

    public FunctionId Function(int id) => new(id, _fnNames[id]);

    /// <summary>Hash-conses a canonical node: returns the existing equal instance when present.</summary>
    internal Expr Intern(Expr node) => _pool.GetOrAdd(node, node);
}

/// <summary>
/// The single total order over canonical expressions (§6.3 of the architecture). Used for
/// sorting Add/Multiply operands and for deterministic traversal. Never uses hash codes.
/// </summary>
public static class TermOrder
{
    public static bool IsNumericConstant(Expr e) =>
        e.Kind is NodeKind.IntegerConstant or NodeKind.RationalConstant
            or NodeKind.RealConstant or NodeKind.ComplexConstant;

    private static Rat ToRational(Expr e) => e switch
    {
        IntegerConstantExpr i => Rat.From(i.Value),
        RationalConstantExpr r => r.Value,
        RealConstantExpr r => r.Value.ToRational(),
        _ => Rat.Zero,
    };

    private static int KindRank(NodeKind k) => k switch
    {
        NodeKind.Symbol => 1,
        NodeKind.NamedConstant => 2,
        NodeKind.Function => 3,
        NodeKind.Power => 4,
        NodeKind.Multiply => 5,
        NodeKind.Add => 6,
        NodeKind.Relation => 7,
        NodeKind.Piecewise => 8,
        NodeKind.Derivative => 9,
        NodeKind.Integral => 10,
        NodeKind.RootOf => 11,
        _ => 0,
    };

    public static int Compare(Expr a, Expr b)
    {
        bool an = IsNumericConstant(a), bn = IsNumericConstant(b);
        if (an && bn)
            return CompareNumbers(a, b);
        if (an) return -1;
        if (bn) return 1;

        int ra = KindRank(a.Kind), rb = KindRank(b.Kind);
        if (ra != rb) return ra.CompareTo(rb);

        switch (a.Kind)
        {
            case NodeKind.Symbol:
                return ((SymbolExpr)a).Symbol.CompareTo(((SymbolExpr)b).Symbol);
            case NodeKind.NamedConstant:
                return ((NamedConstantExpr)a).Constant.CompareTo(((NamedConstantExpr)b).Constant);
            case NodeKind.Function:
            {
                var fa = (FunctionExpr)a; var fb = (FunctionExpr)b;
                int c = fa.Function.CompareTo(fb.Function);
                return c != 0 ? c : CompareSeq(fa.Arguments, fb.Arguments);
            }
            case NodeKind.Power:
            {
                var pa = (PowerExpr)a; var pb = (PowerExpr)b;
                int c = Compare(pa.Base, pb.Base);
                return c != 0 ? c : Compare(pa.Exponent, pb.Exponent);
            }
            case NodeKind.Multiply:
                return CompareSeq(((MultiplyExpr)a).Factors, ((MultiplyExpr)b).Factors);
            case NodeKind.Add:
                return CompareSeq(((AddExpr)a).Terms, ((AddExpr)b).Terms);
            case NodeKind.Relation:
            {
                var ra2 = (RelationExpr)a; var rb2 = (RelationExpr)b;
                int c = ra2.Op.CompareTo(rb2.Op);
                if (c != 0) return c;
                c = Compare(ra2.Left, rb2.Left);
                return c != 0 ? c : Compare(ra2.Right, rb2.Right);
            }
            case NodeKind.Piecewise:
            {
                var pa = (PiecewiseExpr)a; var pb = (PiecewiseExpr)b;
                int c = Compare(pa.Otherwise, pb.Otherwise);
                if (c != 0) return c;
                return CompareSeqBranches(pa.Branches, pb.Branches);
            }
            case NodeKind.Derivative:
            case NodeKind.Integral:
            {
                Expr oa, ob; System.Collections.Immutable.ImmutableArray<Symbol> va, vb;
                if (a is DerivativeExpr da) { oa = da.Operand; va = da.Variables; }
                else { var ia = (IntegralExpr)a; oa = ia.Operand; va = ia.Variables; }
                if (b is DerivativeExpr db) { ob = db.Operand; vb = db.Variables; }
                else { var ib = (IntegralExpr)b; ob = ib.Operand; vb = ib.Variables; }
                int c = Compare(oa, ob);
                if (c != 0) return c;
                for (int i = 0; i < va.Length && i < vb.Length; i++)
                {
                    c = va[i].CompareTo(vb[i]);
                    if (c != 0) return c;
                }
                return va.Length.CompareTo(vb.Length);
            }
            case NodeKind.RootOf:
            {
                var ra2 = (RootOfExpr)a; var rb2 = (RootOfExpr)b;
                int c = ra2.DefiningPolynomial.CompareTo(rb2.DefiningPolynomial);
                return c != 0 ? c : ra2.RootIndex.CompareTo(rb2.RootIndex);
            }
            default:
                return 0;
        }
    }

    private static int CompareSeq(System.Collections.Immutable.ImmutableArray<Expr> a, System.Collections.Immutable.ImmutableArray<Expr> b)
    {
        for (int i = 0; i < a.Length && i < b.Length; i++)
        {
            int c = Compare(a[i], b[i]);
            if (c != 0) return c;
        }
        return a.Length.CompareTo(b.Length);
    }

    private static int CompareSeqBranches(System.Collections.Immutable.ImmutableArray<PiecewiseBranch> a, System.Collections.Immutable.ImmutableArray<PiecewiseBranch> b)
    {
        for (int i = 0; i < a.Length && i < b.Length; i++)
        {
            int c = Compare(a[i].Guard, b[i].Guard);
            if (c != 0) return c;
            c = Compare(a[i].Value, b[i].Value);
            if (c != 0) return c;
        }
        return a.Length.CompareTo(b.Length);
    }

    private static int CompareNumbers(Expr a, Expr b)
    {
        var rea = ToRational(a);
        var reb = ToRational(b);
        int c = rea.CompareTo(reb);
        if (c != 0) return c;
        var ima = a is ComplexConstantExpr ca ? ca.Im : Rat.Zero;
        var imb = b is ComplexConstantExpr cb ? cb.Im : Rat.Zero;
        c = ima.CompareTo(imb);
        if (c != 0) return c;
        // same value, different tiers: exact (rational) sorts before approximate (real), before complex
        int tier(Expr e) => e.Kind switch
        {
            NodeKind.IntegerConstant or NodeKind.RationalConstant => 0,
            NodeKind.RealConstant => 1,
            _ => 2,
        };
        return tier(a).CompareTo(tier(b));
    }
}
