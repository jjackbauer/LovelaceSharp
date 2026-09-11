using Lovelace.Abstractions;

namespace Lovelace.Suite;

/// <summary>
/// Canonical display formatting for <see cref="Value"/> instances, shared by
/// interpolation, <c>print</c>, the REPL, and vector rendering. Separates the
/// plain value text from the type-suffixed form used in the REPL.
/// <para>
/// Every public entry measures the value's nesting against <see cref="InputDepth.MaxValueDepth"/>
/// BEFORE it recurses (see <see cref="ValueDepth"/>): the rendering below is a native recursion over
/// the value's structure, and a value built by repetition at run time is deep in neither the source
/// nor the parsed tree, so no input budget has seen it. The recursion itself goes through the
/// private <c>…Core</c> methods, so one render measures once rather than once per level.
/// </para>
/// </summary>
public static class ValueFormatter
{
    /// <summary>Returns the plain display text of a value (no type suffix). <paramref name="unicode"/>
    /// selects the Unicode math glyphs (∞ √ π ≤ ≥ ≠) for symbolic rendering; ASCII is the default.</summary>
    public static string Format(Value value, bool unicode = false)
    {
        ValueDepth.EnsureWithin("value", value);
        return FormatCore(value, unicode);
    }

    private static string FormatCore(Value value, bool unicode = false) => value.Kind switch
    {
        ValueKind.Natural  => value.AsNatural().ToString(),
        ValueKind.Integer  => value.AsInteger().ToString(),
        ValueKind.Real     => value.AsReal().ToString(),
        ValueKind.Complex  => value.AsComplex().ToString(),
        ValueKind.Symbolic => Lovelace.Symbolics.Printing.PrettyPrint(value.AsSymbolic(),
            new Lovelace.Symbolics.Printing.PrintOptions(Unicode: unicode)),
        ValueKind.Boolean  => value.AsBoolean() ? "True" : "False",
        ValueKind.Text     => value.AsText(),
        // the member name, never "EnumValue { ... }": an enum reads exactly as the Text form it
        // replaces did, while the structured form carries the declared enum type
        ValueKind.Enum     => value.AsEnum().Name,
        ValueKind.Vector   => FormatArrayCore(value.AsArrayValue(), unicode),
        ValueKind.Array    => FormatArrayCore(value.AsArrayValue(), unicode),
        ValueKind.Function => $"Function: {value.AsFunction().Name}",
        ValueKind.Record   => FormatRecordCore(value.AsRecord(), unicode),
        ValueKind.Domain   => value.AsDomain().ToString().ToLowerInvariant(),
        ValueKind.Void     => string.Empty,
        _                  => value.ToString(),
    };

    /// <summary>Renders an N-dimensional array as nested brackets, e.g. <c>[[1, 2], [3, 4]]</c>.</summary>
    public static string FormatArray(ArrayValue array, bool unicode = false)
    {
        ValueDepth.EnsureWithin("value", array);
        return FormatArrayCore(array, unicode);
    }

    private static string FormatArrayCore(ArrayValue array, bool unicode = false)
    {
        long[] shape = array.Shape.ToArray();
        return FormatLevel(array, shape, 0, array.Rank, 0, unicode);
    }

    private static string FormatLevel(ArrayValue array, long[] shape, int dim, int rank, long offset, bool unicode)
    {
        if (dim == rank - 1)
        {
            var parts = new List<string>((int)shape[dim]);
            for (long i = 0; i < shape[dim]; i++)
                parts.Add(FormatCore((Value)array.GetElement(offset + i), unicode));
            return "[" + string.Join(", ", parts) + "]";
        }

        long stride = 1;
        for (int s = dim + 1; s < rank; s++)
            stride *= shape[s];
        var rows = new List<string>((int)shape[dim]);
        for (long i = 0; i < shape[dim]; i++)
            rows.Add(FormatLevel(array, shape, dim + 1, rank, offset + i * stride, unicode));
        return "[" + string.Join(", ", rows) + "]";
    }

    /// <summary>Renders a structured record compactly, e.g.
    /// <c>SolveResult(status: Solved, solutions: [-2, 2])</c>. Field order is declaration order.</summary>
    public static string FormatRecord(RecordValue record, bool unicode = false)
    {
        ValueDepth.EnsureWithin("value", record);
        return FormatRecordCore(record, unicode);
    }

    private static string FormatRecordCore(RecordValue record, bool unicode = false) =>
        record.TypeName + "(" + string.Join(", ", record.Fields.Select(f => $"{f.Name}: {FormatCore((Value)f.Value!, unicode)}")) + ")";

    /// <summary>Returns the value with a type suffix, e.g. <c>"42 (Natural)"</c>.</summary>
    public static string FormatTyped(Value value, bool unicode = false)
    {
        ValueDepth.EnsureWithin("value", value);
        return FormatTypedCore(value, unicode);
    }

    private static string FormatTypedCore(Value value, bool unicode = false) => value.Kind switch
    {
        ValueKind.Natural  => $"{value.AsNatural()} (Natural)",
        ValueKind.Integer  => $"{value.AsInteger()} (Integer)",
        ValueKind.Real     => $"{value.AsReal()} (Real)",
        ValueKind.Complex  => $"{value.AsComplex()} (Complex)",
        ValueKind.Symbolic => $"{FormatCore(value, unicode)} (Symbolic)",
        ValueKind.Boolean  => $"{value.AsBoolean()} (Boolean)",
        ValueKind.Text     => value.AsText(),
        // a bare member name, exactly as the Text kind it replaces rendered: the typed form of a
        // record's field stays readable ("status: Solved"), and the enum type is available from
        // the structured form and from type()
        ValueKind.Enum     => value.AsEnum().Name,
        ValueKind.Vector   => $"{FormatCore(value, unicode)} (Vector)",
        ValueKind.Array    => $"{FormatCore(value, unicode)} (Array)",
        ValueKind.Function => $"{FormatCore(value, unicode)} (Function)",
        ValueKind.Record   => $"{FormatCore(value, unicode)} ({value.AsRecord().TypeName})",
        ValueKind.Domain   => $"{FormatCore(value, unicode)} (Domain)",
        ValueKind.Void     => "(void)",
        _                  => value.ToString(),
    };
}
