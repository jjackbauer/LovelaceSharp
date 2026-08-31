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
        ValueKind.Function => $"Function: {value.AsFunction().Name}",
        ValueKind.Void     => string.Empty,
        _                  => value.ToString(),
    };

    /// <summary>Returns the value with a type suffix, e.g. <c>"42 (Natural)"</c>.</summary>
    public static string FormatTyped(Value value) => value.Kind switch
    {
        ValueKind.Natural  => $"{value.AsNatural()} (Natural)",
        ValueKind.Integer  => $"{value.AsInteger()} (Integer)",
        ValueKind.Real     => $"{value.AsReal()} (Real)",
        ValueKind.Boolean  => $"{value.AsBoolean()} (Boolean)",
        ValueKind.Text     => value.AsText(),
        ValueKind.Vector   => $"{Format(value)} (Vector)",
        ValueKind.Function => $"{Format(value)} (Function)",
        ValueKind.Void     => "(void)",
        _                  => value.ToString(),
    };
}
