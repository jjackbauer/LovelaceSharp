using Lovelace.Abstractions;
using Rl = Lovelace.Real.Real;

namespace Lovelace.Real;

/// <summary>
/// The exact <see cref="IField{T}"/> identity over <see cref="Rl"/> — the field the core
/// injects into plugin kernels targeting <c>DType.Real</c>. Singleton, AOT-safe (no reflection),
/// and the arithmetic inherits the active <c>Rl.WithPrecision</c> scope.
/// </summary>
public sealed class RealField : IField<Rl>
{
    /// <summary>The shared instance.</summary>
    public static readonly RealField Instance = new();

    private RealField() { }

    public Rl Zero => Rl.Zero;
    public Rl One => Rl.One;
    public Rl FromLong(long value) => new Rl(value.ToString(System.Globalization.CultureInfo.InvariantCulture));
    public Rl Add(Rl a, Rl b) => a + b;
    public Rl Subtract(Rl a, Rl b) => a - b;
    public Rl Multiply(Rl a, Rl b) => a * b;
    public Rl Divide(Rl a, Rl b) => a / b;
    public Rl Negate(Rl a) => -a;
    public bool IsZero(Rl a) => Rl.IsZero(a);
    public int Compare(Rl a, Rl b) => a.CompareTo(b);
    public Rl Sqrt(Rl a) => Rl.Sqrt(a);
}
