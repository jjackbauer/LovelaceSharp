using Lovelace.Abstractions;
using Int = Lovelace.Integer.Integer;

namespace Lovelace.Integer;

/// <summary>
/// The exact <see cref="IField{T}"/> identity over <see cref="Int"/> — the field the core
/// injects into plugin kernels targeting <c>DType.Integer</c>. Singleton, AOT-safe (no
/// reflection). Square root has no integer identity; it declines with
/// <see cref="NotSupportedException"/> rather than rounding.
/// </summary>
public sealed class IntegerField : IField<Int>
{
    /// <summary>The shared instance.</summary>
    public static readonly IntegerField Instance = new();

    private IntegerField() { }

    public Int Zero => new(0);
    public Int One => new(1);
    public Int FromLong(long value) => new(value);
    public Int Add(Int a, Int b) => a + b;
    public Int Subtract(Int a, Int b) => a - b;
    public Int Multiply(Int a, Int b) => a * b;
    public Int Divide(Int a, Int b) => a / b;
    public Int Negate(Int a) => -a;
    public bool IsZero(Int a) => Int.IsZero(a);
    public int Compare(Int a, Int b) => a.CompareTo(b);
    public Int Sqrt(Int a) =>
        throw new NotSupportedException("Integer has no exact square-root identity; use Real for sqrt.");
}
