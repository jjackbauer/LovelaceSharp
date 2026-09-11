using Lovelace.Abstractions;
using Nat = global::Lovelace.Natural.Natural;
using Int = global::Lovelace.Integer.Integer;
using Rl = global::Lovelace.Real.Real;
using Cplx = global::Lovelace.Complex.Complex;

namespace Lovelace.Suite;

/// <summary>
/// The core-owned Value ↔ plugin-payload mapping (the Modus seam). Plugins speak only
/// payload types (numerics, <see cref="Lovelace.Symbolics.Expr"/>, <see cref="RecordValue"/>,
/// <see cref="MathDomain"/>, arrays); the Suite core owns the boxing into <see cref="Value"/>.
/// </summary>
internal static class PayloadMap
{
    public static Value Wrap(object? payload) => payload switch
    {
        // Real derives from Integer, so match the narrowest reference types first.
        Value v => v,
        Rl real => new Value(real),
        Int integer => new Value(integer),
        Nat natural => new Value(natural),
        Cplx complex => new Value(complex),
        Lovelace.Symbolics.Expr expr => new Value(expr),
        bool boolean => new Value(boolean),
        string text => new Value(text),
        long l => new Value(new Int(l)),
        int i => new Value(new Int(i)),
        RecordValue record => new Value(record),
        MathDomain domain => new Value(domain),
        EnumValue enumValue => new Value(enumValue),
        ArrayValue array => new Value(array, array.Rank == 1 ? ValueKind.Vector : ValueKind.Array),
        IReadOnlyList<object?> elements => WrapArray(elements),
        null => Value.Void,   // absent structured field (e.g. no one-sided limit)
        _ => throw new InvalidOperationException($"A plugin builtin returned an unsupported result type '{payload?.GetType().Name ?? "null"}'."),
    };

    public static Value WrapArray(IReadOnlyList<object?> elements)
    {
        if (elements.Count > 0 && elements[0] is IReadOnlyList<object?>)
        {
            var (data, shape) = Flatten(elements);
            return new Value(TypedArrayAdapter.FromValues(data, shape));
        }
        var boxed = new Value[elements.Count];
        for (int i = 0; i < elements.Count; i++)
            boxed[i] = Wrap(elements[i]);
        return new Value(boxed);
    }

    /// <summary>Flattens a rectangular nested payload (rows of rows …) to a row-major Value
    /// list with its shape (rank ≥ 2). Ragged input is rejected.</summary>
    private static (Value[] Data, long[] Shape) Flatten(IReadOnlyList<object?> list)
    {
        if (list.Count == 0)
            return (Array.Empty<Value>(), new[] { 0L });
        if (list[0] is IReadOnlyList<object?> sub)
        {
            var (firstData, firstShape) = Flatten(sub);
            var all = new List<Value>(list.Count * firstData.Length);
            foreach (var item in list)
            {
                if (item is not IReadOnlyList<object?> row)
                    throw new InvalidOperationException("Ragged nested arrays are not supported.");
                var (d, sh) = Flatten(row);
                if (!sh.SequenceEqual(firstShape))
                    throw new InvalidOperationException("Ragged nested arrays are not supported.");
                all.AddRange(d);
            }
            return (all.ToArray(), new[] { (long)list.Count }.Concat(firstShape).ToArray());
        }
        return (list.Select(Wrap).ToArray(), new[] { (long)list.Count });
    }

    public static object? Unwrap(Value value) => value.Kind switch
    {
        ValueKind.Natural => value.AsNatural(),
        ValueKind.Integer => value.AsInteger(),
        ValueKind.Real => value.AsReal(),
        ValueKind.Complex => value.AsComplex(),
        ValueKind.Symbolic => value.AsSymbolic(),
        ValueKind.Boolean => value.AsBoolean(),
        ValueKind.Text => value.AsText(),
        ValueKind.Record => UnwrapRecord(value.AsRecord()),
        ValueKind.Domain => value.AsDomain(),
        ValueKind.Enum => value.AsEnum(),
        ValueKind.Vector or ValueKind.Array => UnwrapArray(value.AsArrayValue()),
        // an absent structured field (Value.Void) round-trips as null: Wrap(null) produced it,
        // so Unwrap must invert it instead of rejecting the payload
        ValueKind.Void => null,
        ValueKind.Function => value.AsFunction(),
        _ => throw new InvalidOperationException($"A plugin builtin received an unsupported argument kind '{value.Kind}'."),
    };

    /// <summary>Unwraps a record's fields back to payloads so plugins receive raw values
    /// (e.g. the ir text of a CompilationResult, not a boxed Value).</summary>
    private static object? UnwrapRecord(RecordValue record)
    {
        var fields = record.Fields
            .Select(f => new RecordField(f.Name, Unwrap((Value)f.Value!)))
            .ToArray();
        return new RecordValue(record.TypeName, fields);
    }

    private static IReadOnlyList<object?> UnwrapArray(ArrayValue array)
    {
        var elements = TypedArrayAdapter.ToElements(array);
        var payloads = new object?[elements.Count];
        for (int i = 0; i < elements.Count; i++)
            payloads[i] = Unwrap(elements[i]);
        // carry the shape across the boundary: a plugin that must validate the caller's structure
        // (evalir_batch rows) can, while list-only consumers see the same flat sequence as before
        return new PayloadArray(payloads, array.Shape.ToArray().Select(s => (long)s).ToArray());
    }
}
