using Nat = global::Lovelace.Natural.Natural;
using Int = global::Lovelace.Integer.Integer;
using Rl = global::Lovelace.Real.Real;
using Lovelace.Real;
using Lovelace.Symbolics;
using SymExpr = Lovelace.Symbolics.Expr;
using Cplx = global::Lovelace.Complex.Complex;

namespace Lovelace.Suite;

/// <summary>
/// Scalar numeric arithmetic and comparison shared by the interpreter and the
/// vector/array operation layer. Every operation widens along
/// <c>Natural → Integer → Real</c> and dispatches to the arbitrary-precision types.
/// </summary>
public static class NumericOps
{
    /// <summary>A canonical zero (Natural 0).</summary>
    public static Value Zero => new(new Nat(0));

    /// <summary>A canonical one (Natural 1).</summary>
    public static Value One => new(new Nat(1));

    /// <summary>Whether a value is one of the three numeric kinds.</summary>
    public static bool IsNumeric(Value value) =>
        value.Kind is ValueKind.Natural or ValueKind.Integer or ValueKind.Real;

    /// <summary>
    /// Applies a scalar binary operator after widening both operands to the wider
    /// numeric kind. Natural subtraction auto-widens on underflow; division is exact
    /// (a non-exact quotient becomes a Real with period detection).
    /// </summary>
    public static Value Apply(BinaryOp op, Value left, Value right)
    {
        if (left.Kind == ValueKind.Symbolic || right.Kind == ValueKind.Symbolic)
            return ApplySymbolic(op, left, right);

        (left, right) = Value.WidenPair(left, right);

        return (op, left.Kind) switch
        {
            // ---- Natural arithmetic ----
            (BinaryOp.Add,      ValueKind.Natural) => new Value(left.AsNatural() + right.AsNatural()),
            (BinaryOp.Subtract, ValueKind.Natural) => SubtractNatural(left, right),
            (BinaryOp.Multiply, ValueKind.Natural) => new Value(left.AsNatural() * right.AsNatural()),
            (BinaryOp.Divide,   ValueKind.Natural) => DivideNatural(left, right),
            (BinaryOp.Modulo,   ValueKind.Natural) => new Value(left.AsNatural() % right.AsNatural()),
            (BinaryOp.Power,    ValueKind.Natural) => new Value(left.AsNatural().Pow(right.AsNatural())),

            // ---- Integer arithmetic ----
            (BinaryOp.Add,      ValueKind.Integer) => new Value(left.AsInteger() + right.AsInteger()),
            (BinaryOp.Subtract, ValueKind.Integer) => new Value(left.AsInteger() - right.AsInteger()),
            (BinaryOp.Multiply, ValueKind.Integer) => new Value(left.AsInteger() * right.AsInteger()),
            (BinaryOp.Divide,   ValueKind.Integer) => DivideInteger(left, right),
            (BinaryOp.Modulo,   ValueKind.Integer) => new Value(left.AsInteger() % right.AsInteger()),
            (BinaryOp.Power,    ValueKind.Integer) => IntegerPower(left.AsInteger(), right.AsInteger()),

            // ---- Real arithmetic ----
            (BinaryOp.Add,      ValueKind.Real) => ApplyRealBinary(BinaryOp.Add, left.AsReal(), right.AsReal()),
            (BinaryOp.Subtract, ValueKind.Real) => ApplyRealBinary(BinaryOp.Subtract, left.AsReal(), right.AsReal()),
            (BinaryOp.Multiply, ValueKind.Real) => ApplyRealBinary(BinaryOp.Multiply, left.AsReal(), right.AsReal()),
            (BinaryOp.Divide,   ValueKind.Real) => ApplyRealBinary(BinaryOp.Divide, left.AsReal(), right.AsReal()),
            (BinaryOp.Modulo,   ValueKind.Real) => new Value(left.AsReal() % right.AsReal()),
            (BinaryOp.Power,    ValueKind.Real) => PowerReal(left.AsReal(), right.AsReal()),

            (_, ValueKind.Complex) => throw new InvalidOperationException(
                $"Operator '{op}' is not supported for Complex; use re()/im()/conj()/abs() to bridge back to Real."),

            _ => throw new InvalidOperationException(
                $"Operator '{op}' is not supported for type '{left.Kind}'."),
        };
    }

    /// <summary>The exact Real value 1/2, the one exponent that has an exact complex closed form
    /// over a negative base.</summary>
    private static readonly Rl OneHalf = Rl.Parse("0.5", null);

    /// <summary>
    /// Real-domain power with ONE exact complex fallback. The Real type owns the real-domain
    /// contract and keeps refusing every value it cannot represent — including <c>2^(1/2)</c>,
    /// whose result is a real irrational it will not approximate, and every other non-integer
    /// exponent. The single rejected input with an exact closed form outside that contract is an
    /// exact negative base under the PRINCIPAL square root: <c>(-a)^(1/2) = i·sqrt(a)</c>, which is
    /// the branch SymPy returns. Only that input is routed to the symbolic complex path; everything
    /// else keeps the existing typed failure.
    /// </summary>
    private static Value PowerReal(Rl baseValue, Rl exponent)
    {
        if (Rl.IsNegative(baseValue) && exponent == OneHalf)
            return new Value(Exprs.Power(
                Exprs.Rational(RationalReal.FromReal(baseValue)),
                Exprs.Rational(1, 2)));
        return new Value(baseValue.Pow(exponent));
    }

    /// <summary>
    /// Real arithmetic fast path: try LReal64 (narrowest/fastest), then LReal128, falling back
    /// to the arbitrary-precision class Real on any promotion. Exactness is preserved because the
    /// limited types throw rather than round. Active only when limited precision is requested.
    /// </summary>
    private static Value ApplyRealBinary(BinaryOp op, Rl left, Rl right)
    {
        if (Rl.MaxComputationDecimalPlaces <= 37)
        {
            if (LReal64.TryFromReal(left, out var a64) && LReal64.TryFromReal(right, out var b64))
            {
                try
                {
                    LReal64 r = op switch
                    {
                        BinaryOp.Add => a64 + b64,
                        BinaryOp.Subtract => a64 - b64,
                        BinaryOp.Multiply => a64 * b64,
                        BinaryOp.Divide => a64 / b64,
                        _ => throw new LRealPromoteException("not a fast-path op")
                    };
                    return new Value(r.ToReal());
                }
                catch (LRealPromoteException) { }
            }

            if (LReal128.TryFromReal(left, out var a128) && LReal128.TryFromReal(right, out var b128))
            {
                try
                {
                    LReal128 r = op switch
                    {
                        BinaryOp.Add => a128 + b128,
                        BinaryOp.Subtract => a128 - b128,
                        BinaryOp.Multiply => a128 * b128,
                        BinaryOp.Divide => a128 / b128,
                        _ => throw new LRealPromoteException("not a fast-path op")
                    };
                    return new Value(r.ToReal());
                }
                catch (LRealPromoteException) { }
            }
        }

        return op switch
        {
            BinaryOp.Add => new Value(left + right),
            BinaryOp.Subtract => new Value(left - right),
            BinaryOp.Multiply => new Value(left * right),
            BinaryOp.Divide => new Value(left / right),
            _ => throw new InvalidOperationException($"Operator '{op}' is not supported for Real.")
        };
    }

    /// <summary>Converts a language value into a symbolic expression (canonical factories).</summary>
    public static SymExpr ToExpr(Value v) => v.Kind switch
    {
        ValueKind.Natural => Exprs.Integer(new Int(v.AsNatural())),
        ValueKind.Integer => Exprs.Integer(v.AsInteger()),
        ValueKind.Real => RealToExpr(v.AsReal()),
        ValueKind.Complex => Exprs.Add(
            RealToExpr(v.AsComplex().Re),
            Exprs.Multiply(RealToExpr(v.AsComplex().Im), Exprs.I)),
        ValueKind.Symbolic => v.AsSymbolic(),
        _ => throw new InvalidOperationException($"Cannot convert '{v.Kind}' to a symbolic expression."),
    };

    /// <summary>Exact Real literals become rationals (INV-09); truncated values stay approximate.</summary>
    private static SymExpr RealToExpr(Rl r) =>
        r.IsPeriodic || -r.Exponent <= 18
            ? Exprs.Rational(RationalReal.FromReal(r))
            : Exprs.Real(RealLiteral.FromRealExact(r));

    private static Value ApplySymbolic(BinaryOp op, Value left, Value right)
    {
        var l = ToExpr(left);
        var r = ToExpr(right);
        SymExpr result = op switch
        {
            BinaryOp.Add => Exprs.Add(l, r),
            BinaryOp.Subtract => Exprs.Subtract(l, r),
            BinaryOp.Multiply => Exprs.Multiply(l, r),
            BinaryOp.Divide => Exprs.Divide(l, r),
            BinaryOp.Power => Exprs.Power(l, r),
            BinaryOp.Equal => Exprs.Relation(RelOp.Eq, l, r),
            BinaryOp.NotEqual => Exprs.Relation(RelOp.Ne, l, r),
            BinaryOp.Greater => Exprs.Relation(RelOp.Gt, l, r),
            BinaryOp.Less => Exprs.Relation(RelOp.Lt, l, r),
            BinaryOp.GreaterEqual => Exprs.Relation(RelOp.Ge, l, r),
            BinaryOp.LessEqual => Exprs.Relation(RelOp.Le, l, r),
            _ => throw new InvalidOperationException($"Operator '{op}' is not supported for symbolic values."),
        };
        return new Value(result);
    }

    /// <summary>Numeric comparison: -1, 0, or 1.</summary>
    public static int Compare(Value left, Value right)
    {
        (left, right) = Value.WidenPair(left, right);

        return left.Kind switch
        {
            ValueKind.Natural => left.AsNatural().CompareTo(right.AsNatural()),
            ValueKind.Integer => left.AsInteger().CompareTo(right.AsInteger()),
            ValueKind.Real    => left.AsReal().CompareTo(right.AsReal()),
            ValueKind.Complex => throw new InvalidOperationException(
                "Cannot compare Complex values; use abs()/re()/im() to compare their Real parts."),
            ValueKind.Symbolic => throw new InvalidOperationException(
                "Cannot order symbolic values; comparisons on symbolic values produce symbolic relations."),
            _ => throw new InvalidOperationException($"Cannot compare values of kind '{left.Kind}'."),
        };
    }

    /// <summary>Arithmetic negation, widening Natural to Integer first.</summary>
    public static Value Negate(Value value) => value.Kind switch
    {
        ValueKind.Natural => new Value(-value.Widen(ValueKind.Integer).AsInteger()),
        ValueKind.Integer => new Value(-value.AsInteger()),
        ValueKind.Real    => new Value(-value.AsReal()),
        ValueKind.Complex => throw new InvalidOperationException(
            "Negation is not supported for Complex; use conj() or re()/im() instead."),
        ValueKind.Symbolic => new Value(Exprs.Negate(value.AsSymbolic())),
        _ => throw new InvalidOperationException($"Negation is not supported for kind '{value.Kind}'."),
    };

    /// <summary>Whether a numeric value is zero.</summary>
    public static bool IsZero(Value value) => value.Kind switch
    {
        ValueKind.Natural => Nat.IsZero(value.AsNatural()),
        ValueKind.Integer => Int.IsZero(value.AsInteger()),
        ValueKind.Real    => Rl.IsZero(value.AsReal()),
        ValueKind.Complex => throw new InvalidOperationException(
            "Expected a numeric value; Complex is a domain type. Use re()/im()/abs() to bridge to Real."),
        ValueKind.Symbolic => false,
        _ => throw new InvalidOperationException($"Expected a numeric value, but got '{value.Kind}'."),
    };

    private static Value SubtractNatural(Value left, Value right)
    {
        try
        {
            return new Value(left.AsNatural() - right.AsNatural());
        }
        catch (InvalidOperationException)
        {
            var leftInt  = left.Widen(ValueKind.Integer);
            var rightInt = right.Widen(ValueKind.Integer);
            return new Value(leftInt.AsInteger() - rightInt.AsInteger());
        }
    }

    /// <summary>
    /// Integer raised to an integer power, with a NEGATIVE exponent supported as the reciprocal
    /// power: <c>x^-n</c> is <c>1 / x^n</c>, computed in exact Real arithmetic, so <c>2^-1</c> is
    /// the very value the language already produces for the literal <c>1/2</c> and <c>3^-2</c>
    /// the one it produces for <c>1/9</c>.
    /// <para>
    /// A zero base is delegated to <see cref="Int.Pow"/>: <c>0^-n</c> has no value in any field,
    /// and the integer type refuses it with a typed, recoverable error that names the base — so
    /// there is exactly one spelling of that failure in the codebase.
    /// </para>
    /// </summary>
    private static Value IntegerPower(Int baseValue, Int exponent)
    {
        if (!Int.IsNegative(exponent) || Int.IsZero(baseValue))
            return new Value(baseValue.Pow(exponent));

        var magnitude = baseValue.Pow(exponent.Negate());
        return new Value(Rl.Divide(
            new Value(Int.One).Widen(ValueKind.Real).AsReal(),
            new Value(magnitude).Widen(ValueKind.Real).AsReal()));
    }

    private static Value DivideNatural(Value left, Value right)
    {
        var quotient = Nat.DivRem(left.AsNatural(), right.AsNatural(), out var remainder);
        if (Nat.IsZero(remainder))
            return new Value(quotient);

        return new Value(Rl.Divide(
            left.Widen(ValueKind.Real).AsReal(),
            right.Widen(ValueKind.Real).AsReal()));
    }

    private static Value DivideInteger(Value left, Value right)
    {
        var quotient = left.AsInteger().DivRem(right.AsInteger(), out var remainder);
        if (Int.IsZero(remainder))
            return new Value(quotient);

        return new Value(Rl.Divide(
            left.Widen(ValueKind.Real).AsReal(),
            right.Widen(ValueKind.Real).AsReal()));
    }
}
