using Lovelace.Abstractions;
using Nat = Lovelace.Natural.Natural;

namespace Lovelace.Natural;

/// <summary>
/// The exact <see cref="IField{T}"/> identity over <see cref="Nat"/> — the field the core
/// injects into plugin kernels targeting <c>DType.Natural</c>. Singleton, AOT-safe (no
/// reflection). Natural has no negation and no square-root identity; both decline with
/// <see cref="NotSupportedException"/> rather than rounding.
/// </summary>
public sealed class NaturalField : IField<Nat>
{
    /// <summary>The shared instance.</summary>
    public static readonly NaturalField Instance = new();

    private NaturalField() { }

    public Nat Zero => new(0UL);
    public Nat One => new(1UL);
    public Nat FromLong(long value) =>
        value >= 0 ? new Nat((ulong)value) : throw new ArgumentOutOfRangeException(nameof(value), "Natural has no negative values.");
    public Nat Add(Nat a, Nat b) => a + b;
    public Nat Subtract(Nat a, Nat b) => a - b;
    public Nat Multiply(Nat a, Nat b) => a * b;
    public Nat Divide(Nat a, Nat b) => a / b;
    public Nat Negate(Nat a) =>
        throw new NotSupportedException("Natural has no negation; use Integer for signed arithmetic.");
    public bool IsZero(Nat a) => Nat.IsZero(a);
    public int Compare(Nat a, Nat b) => a.CompareTo(b);
    public Nat Sqrt(Nat a) =>
        throw new NotSupportedException("Natural has no exact square-root identity; use Real for sqrt.");
}
