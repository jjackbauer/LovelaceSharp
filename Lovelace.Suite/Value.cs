using Nat = global::Lovelace.Natural.Natural;
using Int = global::Lovelace.Integer.Integer;
using Rl = global::Lovelace.Real.Real;
using Cplx = global::Lovelace.Complex.Complex;
using Lovelace.Arrays;
using Lovelace.Abstractions;

namespace Lovelace.Suite;

// -------------------------------------------------------------------------
// ValueKind — discriminated union tag
// -------------------------------------------------------------------------

/// <summary>
/// Identifies which type a <see cref="Value"/> holds.
/// The numeric kinds are ordered from narrowest (Natural=0) to widest (Real=2);
/// the remaining kinds are non-numeric and are excluded from widening arithmetic.
/// <see cref="Complex"/> is a domain type outside the widening lattice.
/// </summary>
public enum ValueKind
{
    Natural,
    Integer,
    Real,
    Boolean,
    Text,
    Vector,
    Function,
    Void,
    Array,
    Complex,
    Symbolic,
    Record,
    Domain,
}

// -------------------------------------------------------------------------
// Value — type-discriminated wrapper
// -------------------------------------------------------------------------

/// <summary>
/// Holds one of the numeric types, a <see cref="bool"/>, a <see cref="string"/>,
/// a <see cref="System.Collections.Generic.IReadOnlyList{T}"/> of values, or a
/// <see cref="FunctionDefinition"/>, together with a <see cref="ValueKind"/> tag.
/// <para>
/// The three numeric kinds form a widening chain: <c>Natural → Integer → Real</c>.
/// </para>
/// </summary>
public sealed class Value
{
    private readonly object _inner;

    // -----------------------------------------------------------------
    // Constructors
    // -----------------------------------------------------------------

    /// <summary>Wraps a <see cref="Nat"/> value.</summary>
    public Value(Nat value)
    {
        _inner = value;
        Kind = ValueKind.Natural;
    }

    /// <summary>Wraps an <see cref="Int"/> value.</summary>
    public Value(Int value)
    {
        _inner = value;
        Kind = ValueKind.Integer;
    }

    /// <summary>Wraps a <see cref="Rl"/> value.</summary>
    public Value(Rl value)
    {
        _inner = value;
        Kind = ValueKind.Real;
    }

    /// <summary>Wraps a complex value.</summary>
    public Value(Cplx value)
    {
        _inner = value;
        Kind = ValueKind.Complex;
    }

    /// <summary>Wraps a symbolic expression (Lovelace.Symbolics).</summary>
    public Value(Lovelace.Symbolics.Expr symbolic)
    {
        _inner = symbolic;
        Kind = ValueKind.Symbolic;
    }

    /// <summary>Wraps a <see cref="bool"/> value.</summary>
    public Value(bool value)
    {
        _inner = value;
        Kind = ValueKind.Boolean;
    }

    /// <summary>Wraps a pre-formatted <see cref="string"/> result.</summary>
    public Value(string text)
    {
        _inner = text;
        Kind = ValueKind.Text;
    }

    /// <summary>Wraps a vector of values (rank 1).</summary>
    public Value(IReadOnlyList<Value> elements)
        : this(TypedArrayAdapter.FromElements(elements), ValueKind.Vector)
    {
    }

    /// <summary>Wraps an N-dimensional array (rank &gt;= 2). Kept for source-compat; adapts to the typed path.</summary>
    public Value(NdArray<Value> array) : this(TypedArrayAdapter.FromNdArray(array), ValueKind.Array)
    {
    }

    /// <summary>Wraps a typed array value (the Stage-2+ representation behind <see cref="ValueKind.Array"/>).</summary>
    public Value(ArrayValue array) : this(array, ValueKind.Array)
    {
    }

    /// <summary>Wraps a typed array value with an explicit presentation kind (Vector vs Array).</summary>
    internal Value(ArrayValue array, ValueKind kind)
    {
        _inner = array;
        Kind = kind;
    }

    /// <summary>Wraps a first-class function reference.</summary>
    public Value(FunctionDefinition function)
    {
        _inner = function;
        Kind = ValueKind.Function;
    }

    /// <summary>Wraps a structured record value (e.g. SolveResult). Field payloads are
    /// mapped onto <see cref="Value"/> at construction so property access sees Values.</summary>
    public Value(RecordValue record)
    {
        var fields = record.Fields
            .Select(f => new RecordField(f.Name, PayloadMap.Wrap(f.Value)))
            .ToArray();
        _inner = new RecordValue(record.TypeName, fields);
        Kind = ValueKind.Record;
    }

    /// <summary>Wraps a first-class mathematical domain (<c>real</c>, <c>complex</c>, …).</summary>
    public Value(MathDomain domain)
    {
        _inner = domain;
        Kind = ValueKind.Domain;
    }

    private Value(ValueKind voidKind)
    {
        _inner = null!;
        Kind = voidKind;
    }

    /// <summary>The singleton <c>void</c> value — result of statements that produce no value.</summary>
    public static Value Void { get; } = new Value(ValueKind.Void);

    // -----------------------------------------------------------------
    // Properties
    // -----------------------------------------------------------------

    /// <summary>Kind tag identifying which type is stored.</summary>
    public ValueKind Kind { get; }

    // -----------------------------------------------------------------
    // Inner-value accessors
    // -----------------------------------------------------------------

    /// <summary>Returns the stored value cast to <see cref="Nat"/>.</summary>
    public Nat AsNatural() => (Nat)_inner;

    /// <summary>Returns the stored value cast to <see cref="Int"/>.</summary>
    public Int AsInteger() => (Int)_inner;

    /// <summary>Returns the stored value cast to <see cref="Rl"/>.</summary>
    public Rl AsReal() => (Rl)_inner;

    public Cplx AsComplex() => (Cplx)_inner;

    public Lovelace.Symbolics.Expr AsSymbolic() => (Lovelace.Symbolics.Expr)_inner;

    /// <summary>Returns the stored value cast to <see cref="bool"/>.</summary>
    public bool AsBoolean() => (bool)_inner;

    /// <summary>Returns the stored value cast to <see cref="string"/>.</summary>
    public string AsText() => (string)_inner;

    /// <summary>Returns the stored value cast to a read-only list of values.</summary>
    public IReadOnlyList<Value> AsVector() => TypedArrayAdapter.ToElements(AsArrayValue());

    /// <summary>Returns the stored value cast to an <see cref="NdArray{T}"/> of values.</summary>
    public NdArray<Value> AsArray() => TypedArrayAdapter.ToNdArray(AsArrayValue());

    /// <summary>Returns the stored value cast to an <see cref="ArrayValue"/>.</summary>
    public ArrayValue AsArrayValue() => (ArrayValue)_inner;

    /// <summary>Returns the stored value cast to a <see cref="FunctionDefinition"/>.</summary>
    public FunctionDefinition AsFunction() => (FunctionDefinition)_inner;

    /// <summary>Returns the stored value cast to a <see cref="RecordValue"/>.</summary>
    public RecordValue AsRecord() => (RecordValue)_inner;

    /// <summary>Returns the stored value cast to a <see cref="MathDomain"/>.</summary>
    public MathDomain AsDomain() => (MathDomain)_inner;

    // -----------------------------------------------------------------
    // Widening
    // -----------------------------------------------------------------

    /// <summary>
    /// Promotes this value to <paramref name="target"/> kind along the chain
    /// <c>Natural → Integer → Real</c>. Passing the same kind returns <c>this</c>.
    /// Non-numeric kinds throw.
    /// </summary>
    public Value Widen(ValueKind target)
    {
        if (target == Kind)
            return this;

        if (!IsNumeric(Kind) || !IsNumeric(target))
        {
            string hint = target == ValueKind.Complex
                ? " Reductions over Complex arrays (sum/mean/dot/norm/matmul) are not supported; use re()/im()/conj()/abs() to bridge back to Real."
                : Kind == ValueKind.Complex
                    ? " Complex is a domain type; use re()/im()/conj()/abs() to bridge back to Real."
                    : Kind == ValueKind.Symbolic || target == ValueKind.Symbolic
                        ? " Symbolic is a domain type; use subs()/evalf() to bridge back to numeric values."
                        : string.Empty;
            throw new InvalidOperationException(
                $"Cannot widen from {Kind} to {target}: only numeric kinds (Natural, Integer, Real) support widening.{hint}");
        }

        if (target < Kind)
            throw new InvalidOperationException(
                $"Cannot narrow from {Kind} to {target}.");

        return (Kind, target) switch
        {
            (ValueKind.Natural, ValueKind.Integer) =>
                new Value(new Int(AsNatural())),

            (ValueKind.Natural, ValueKind.Real) =>
                new Value(new Rl(new Int(AsNatural()))),

            (ValueKind.Integer, ValueKind.Real) =>
                new Value(new Rl(AsInteger())),

            _ => throw new InvalidOperationException(
                $"Unsupported widening: {Kind} → {target}.")
        };
    }

    /// <summary>
    /// Widens both operands to <c>max(a.Kind, b.Kind)</c> and returns the pair.
    /// Both values are guaranteed to have the same <see cref="Kind"/> on return.
    /// Only valid for numeric kinds.
    /// </summary>
    public static (Value, Value) WidenPair(Value a, Value b)
    {
        var target = (ValueKind)Math.Max((int)a.Kind, (int)b.Kind);
        return (a.Widen(target), b.Widen(target));
    }

    private static bool IsNumeric(ValueKind kind) =>
        kind is ValueKind.Natural or ValueKind.Integer or ValueKind.Real;

    // -----------------------------------------------------------------
    // Formatting
    // -----------------------------------------------------------------

    /// <summary>
    /// Returns a string of the form <c>"Kind: value"</c>, e.g. <c>"Natural: 42"</c>.
    /// </summary>
    public override string ToString() => Kind switch
    {
        ValueKind.Natural => $"Natural: {_inner}",
        ValueKind.Integer => $"Integer: {_inner}",
        ValueKind.Real    => $"Real: {_inner}",
        ValueKind.Complex => $"Complex: {_inner}",
        ValueKind.Symbolic => $"Symbolic: {Lovelace.Symbolics.Printing.PrettyPrint(AsSymbolic())}",
        ValueKind.Boolean => $"Boolean: {_inner}",
        ValueKind.Text    => (string)_inner,
        ValueKind.Vector  => $"Vector: {ValueFormatter.Format(this)}",
        ValueKind.Array   => $"Array: {ValueFormatter.Format(this)}",
        ValueKind.Function => $"Function: {AsFunction().Name}",
        ValueKind.Record   => $"Record: {AsRecord().TypeName}",
        ValueKind.Domain   => $"Domain: {AsDomain().ToString().ToLowerInvariant()}",
        ValueKind.Void    => "Void",
        _                 => throw new InvalidOperationException($"Unknown kind: {Kind}"),
    };
}
