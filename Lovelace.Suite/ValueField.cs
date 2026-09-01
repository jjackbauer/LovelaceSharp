using Lovelace.Arrays;
using Nat = global::Lovelace.Natural.Natural;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Suite;

/// <summary>
/// The <see cref="IField{T}"/> implementation over the language's widened <see cref="Value"/>
/// union. Bridges <see cref="Lovelace.Array"/>'s generic numeric algorithms to the exact
/// arithmetic in <see cref="NumericOps"/>.
/// </summary>
public sealed class ValueField : IField<Value>
{
    public static readonly ValueField Instance = new();

    private ValueField() { }

    public Value Zero => NumericOps.Zero;

    public Value One => NumericOps.One;

    public Value FromLong(long value) => new Value(Nat.Parse(value.ToString(), null));

    public Value Add(Value a, Value b) => NumericOps.Apply(BinaryOp.Add, a, b);

    public Value Subtract(Value a, Value b) => NumericOps.Apply(BinaryOp.Subtract, a, b);

    public Value Multiply(Value a, Value b) => NumericOps.Apply(BinaryOp.Multiply, a, b);

    public Value Divide(Value a, Value b) => NumericOps.Apply(BinaryOp.Divide, a, b);

    public Value Negate(Value a) => NumericOps.Negate(a);

    public bool IsZero(Value a) => NumericOps.IsZero(a);

    public int Compare(Value a, Value b) => NumericOps.Compare(a, b);

    public Value Sqrt(Value a) => new Value(Rl.Sqrt(a.Widen(ValueKind.Real).AsReal()));
}
