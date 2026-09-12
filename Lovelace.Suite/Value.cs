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
    /// <summary>A first-class enumerated value (<see cref="EnumValue"/>). APPENDED last: the
    /// numeric values of Natural/Integer/Real are load-bearing (the widening lattice) and are
    /// never renumbered.</summary>
    Enum,
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

    /// <summary>Wraps a first-class enumerated value (an enum-valued result field, e.g.
    /// <c>SolveStatus.Partial</c>) — the DSH protocol's <c>Enum</c> structured kind.</summary>
    public Value(EnumValue value)
    {
        _inner = value;
        Kind = ValueKind.Enum;
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
    public Nat AsNatural() => Require<Nat>("a Natural");

    /// <summary>Returns the stored value cast to <see cref="Int"/>.</summary>
    public Int AsInteger() => Require<Int>("an Integer");

    /// <summary>Returns the stored value cast to <see cref="Rl"/>.</summary>
    public Rl AsReal() => Require<Rl>("a Real");

    public Cplx AsComplex() => Require<Cplx>("a Complex");

    public Lovelace.Symbolics.Expr AsSymbolic() => Require<Lovelace.Symbolics.Expr>("a symbolic expression");

    /// <summary>Returns the stored value cast to <see cref="bool"/>.</summary>
    public bool AsBoolean() => _inner is bool value ? value : throw new ValueShapeException("a Boolean", this);

    /// <summary>Returns the stored value cast to <see cref="string"/>.</summary>
    public string AsText() => Require<string>("text");

    /// <summary>Returns the stored value cast to a read-only list of values.</summary>
    public IReadOnlyList<Value> AsVector() => TypedArrayAdapter.ToElements(AsArrayValue());

    /// <summary>Returns the stored value cast to an <see cref="NdArray{T}"/> of values.</summary>
    public NdArray<Value> AsArray() => TypedArrayAdapter.ToNdArray(AsArrayValue());

    /// <summary>Returns the stored value cast to an <see cref="ArrayValue"/>.</summary>
    public ArrayValue AsArrayValue() => Require<ArrayValue>("an array or vector");

    /// <summary>Returns the stored value cast to a <see cref="FunctionDefinition"/>.</summary>
    public FunctionDefinition AsFunction() => Require<FunctionDefinition>("a function");

    /// <summary>Returns the stored value cast to a <see cref="RecordValue"/>.</summary>
    public RecordValue AsRecord() => Require<RecordValue>("a record");

    /// <summary>Returns the stored value cast to a <see cref="MathDomain"/>.</summary>
    public MathDomain AsDomain() =>
        _inner is MathDomain domain ? domain : throw new ValueShapeException("a domain value such as real or complex", this);

    /// <summary>Returns the stored value cast to an <see cref="EnumValue"/>.</summary>
    public EnumValue AsEnum() => Require<EnumValue>("an enumerated value");

    /// <summary>The ONE coercion guard behind every <c>As…</c> accessor: the value is not the KIND
    /// the caller requires. It raises <see cref="ValueShapeException"/> — carrying the expectation
    /// and this value — instead of letting a raw CLR cast escape as <c>InvalidCastException</c>,
    /// which the runner can only report as an internal invariant failure (audit D, finding F1). The
    /// call-site guard in <see cref="Interpreter"/> turns it into the documented recoverable
    /// argument error naming the builtin, the argument position and the kind that arrived.</summary>
    private T Require<T>(string expected) where T : class =>
        _inner as T ?? throw new ValueShapeException(expected, this);

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
        ValueKind.Enum     => $"Enum: {AsEnum().TypeName}.{AsEnum().Name}",
        ValueKind.Void    => "Void",
        _                 => throw new InvalidOperationException($"Unknown kind: {Kind}"),
    };
}

/// <summary>
/// A coercion failure inside the engine: the value is not the KIND the caller requires, so the
/// <c>As…</c> accessors on <see cref="Value"/> raise this instead of a raw CLR cast — with the
/// expectation in the message rather than the framework's "Specified cast is not valid.".
/// <para>
/// The call-site guard in <see cref="Interpreter"/> catches it for a builtin body and re-raises
/// <see cref="BuiltinShapeException"/> (an <see cref="ArgumentException"/>, so the wire carries the
/// documented <c>InvalidArgument</c>/<c>TypeMismatch</c>), naming the builtin, the 1-based argument
/// position and the kind that arrived. <see cref="Offender"/> is the value that failed the
/// coercion: when it is one of the call's arguments the attribution is exact. A failure on an
/// ENGINE-INTERNAL value is not a caller mistake and keeps crossing as it did before — an internal
/// invariant failure — so a genuine engine bug is never disguised as a user error. Deriving from
/// <see cref="InvalidCastException"/> keeps that escape hatch (and every existing host that catches
/// the framework type) behaving exactly as it did.
/// </para>
/// </summary>
internal sealed class ValueShapeException(string expected, Value? offender = null) : InvalidCastException(
    offender is null
        ? $"a value is not {expected}."
        : $"a value of kind {offender.Kind} is not {expected}.")
{
    /// <summary>The kind the caller required, as prose: <c>an array or vector</c>, <c>a Real</c>.</summary>
    public string Expected { get; } = expected;

    /// <summary>The value that failed the coercion, or <see langword="null"/> when the failing value
    /// is not a single <see cref="Value"/> (a shape check inside the array kernel, for example).</summary>
    public Value? Offender { get; } = offender;
}
