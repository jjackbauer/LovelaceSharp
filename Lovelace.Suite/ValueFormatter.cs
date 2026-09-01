using Lovelace.Arrays;

namespace Lovelace.Suite;

/// <summary>
/// Canonical display formatting for <see cref="Value"/> instances, shared by
/// interpolation, <c>print</c>, the REPL, and vector rendering. Separates the
/// plain value text from the type-suffixed form used in the REPL.
/// </summary>
public static class ValueFormatter
{
    /// <summary>Returns the plain display text of a value (no type suffix).</summary>
    public static string Format(Value value) => value.Kind switch
    {
        ValueKind.Natural  => value.AsNatural().ToString(),
        ValueKind.Integer  => value.AsInteger().ToString(),
        ValueKind.Real     => value.AsReal().ToString(),
        ValueKind.Boolean  => value.AsBoolean() ? "True" : "False",
        ValueKind.Text     => value.AsText(),
        ValueKind.Vector   => "[" + string.Join(", ", value.AsVector().Select(Format)) + "]",
        ValueKind.Array    => FormatArray(value.AsArray()),
        ValueKind.Function => $"Function: {value.AsFunction().Name}",
        ValueKind.Void     => string.Empty,
        _                  => value.ToString(),
    };

    /// <summary>Renders an N-dimensional array as nested brackets, e.g. <c>[[1, 2], [3, 4]]</c>.</summary>
    public static string FormatArray(NdArray<Value> array)
    {
        long[] shape = array.Shape;
        return FormatLevel(shape, array.Data, 0, array.Rank, 0);
    }

    private static string FormatLevel(long[] shape, IReadOnlyList<Value> data, int dim, int rank, long offset)
    {
        if (dim == rank - 1)
        {
            var parts = new List<string>((int)shape[dim]);
            for (long i = 0; i < shape[dim]; i++)
                parts.Add(Format(data[(int)(offset + i)]));
            return "[" + string.Join(", ", parts) + "]";
        }

        long stride = 1;
        for (int s = dim + 1; s < rank; s++)
            stride *= shape[s];
        var rows = new List<string>((int)shape[dim]);
        for (long i = 0; i < shape[dim]; i++)
            rows.Add(FormatLevel(shape, data, dim + 1, rank, offset + i * stride));
        return "[" + string.Join(", ", rows) + "]";
    }

    /// <summary>Returns the value with a type suffix, e.g. <c>"42 (Natural)"</c>.</summary>
    public static string FormatTyped(Value value) => value.Kind switch
    {
        ValueKind.Natural  => $"{value.AsNatural()} (Natural)",
        ValueKind.Integer  => $"{value.AsInteger()} (Integer)",
        ValueKind.Real     => $"{value.AsReal()} (Real)",
        ValueKind.Boolean  => $"{value.AsBoolean()} (Boolean)",
        ValueKind.Text     => value.AsText(),
        ValueKind.Vector   => $"{Format(value)} (Vector)",
        ValueKind.Array    => $"{Format(value)} (Array)",
        ValueKind.Function => $"{Format(value)} (Function)",
        ValueKind.Void     => "(void)",
        _                  => value.ToString(),
    };
}
