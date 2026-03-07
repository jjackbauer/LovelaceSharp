using Nat = global::Lovelace.Natural.Natural;
using Int = global::Lovelace.Integer.Integer;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Console.Repl;

// -------------------------------------------------------------------------
// ValueKind — discriminated union tag
// -------------------------------------------------------------------------

/// <summary>
/// Identifies which numeric type (or Boolean) a <see cref="Value"/> holds.
/// The numeric kinds are ordered from narrowest (Natural=0) to widest (Real=2);
/// Boolean=3 is a non-numeric sentinel and is excluded from widening arithmetic.
/// Text=4 stores a pre-formatted result string (e.g. from <c>divrem</c>).
/// </summary>
public enum ValueKind
{
    Natural,
    Integer,
    Real,
    Boolean,
    Text,
}

// -------------------------------------------------------------------------
// Value — type-discriminated wrapper around the three numeric types
// -------------------------------------------------------------------------

/// <summary>
/// Holds one of <see cref="Nat"/>, <see cref="Int"/>, <see cref="Rl"/>, or
/// <see cref="bool"/> together with a <see cref="ValueKind"/> tag.
/// <para>
/// The three numeric kinds form a widening chain: <c>Natural → Integer → Real</c>.
/// <see cref="Widen"/> and <see cref="WidenPair"/> enforce this order.
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

    /// <summary>Wraps a <see cref="bool"/> value.</summary>
    public Value(bool value)
    {
        _inner = value;
        Kind = ValueKind.Boolean;
    }

    /// <summary>Wraps a pre-formatted <see cref="string"/> result (e.g. from <c>divrem</c>).</summary>
    public Value(string text)
    {
        _inner = text;
        Kind = ValueKind.Text;
    }

    // -----------------------------------------------------------------
    // Properties
    // -----------------------------------------------------------------

    /// <summary>Kind tag identifying which numeric type is stored.</summary>
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

    /// <summary>Returns the stored value cast to <see cref="bool"/>.</summary>
    public bool AsBoolean() => (bool)_inner;

    /// <summary>Returns the stored value cast to <see cref="string"/>.</summary>
    public string AsText() => (string)_inner;

    // -----------------------------------------------------------------
    // Widening
    // -----------------------------------------------------------------

    /// <summary>
    /// Promotes this value to <paramref name="target"/> kind.
    /// Only widening is supported along the chain <c>Natural → Integer → Real</c>.
    /// Passing the same kind returns <c>this</c> unchanged (no allocation).
    /// Narrowing or involving <see cref="ValueKind.Boolean"/> throws
    /// <see cref="InvalidOperationException"/>.
    /// </summary>
    public Value Widen(ValueKind target)
    {
        if (target == Kind)
            return this;

        if (Kind == ValueKind.Boolean || target == ValueKind.Boolean
            || Kind == ValueKind.Text || target == ValueKind.Text)
            throw new InvalidOperationException(
                $"Cannot widen from {Kind} to {target}: only numeric kinds (Natural, Integer, Real) support widening.");

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
    /// </summary>
    public static (Value, Value) WidenPair(Value a, Value b)
    {
        var target = (ValueKind)Math.Max((int)a.Kind, (int)b.Kind);
        return (a.Widen(target), b.Widen(target));
    }

    // -----------------------------------------------------------------
    // Formatting
    // -----------------------------------------------------------------

    /// <summary>
    /// Returns a string of the form <c>"Kind: value"</c>,
    /// e.g. <c>"Natural: 42"</c> or <c>"Integer: -3"</c>.
    /// </summary>
    public override string ToString() => Kind switch
    {
        ValueKind.Natural => $"Natural: {_inner}",
        ValueKind.Integer => $"Integer: {_inner}",
        ValueKind.Real    => $"Real: {_inner}",
        ValueKind.Boolean => $"Boolean: {_inner}",
        ValueKind.Text    => (string)_inner,
        _                 => throw new InvalidOperationException($"Unknown kind: {Kind}"),
    };
}
