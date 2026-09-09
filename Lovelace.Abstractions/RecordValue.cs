namespace Lovelace.Abstractions;

// -------------------------------------------------------------------------
// Structured record values (the DX-convergence object model)
// -------------------------------------------------------------------------

/// <summary>One named field of a <see cref="RecordValue"/>. Field order is declaration order —
/// the machine-readable contract of a record type is its ordered field list.</summary>
public sealed record RecordField(string Name, object? Value);

/// <summary>
/// A first-class structured result: an ordered set of named fields tagged with a stable
/// <see cref="TypeName"/> (e.g. "SolveResult", "TransformResult"). Plugins construct these
/// over the Modus payload boundary (they live in Abstractions so no plugin references Suite);
/// the Suite core maps them onto <c>ValueKind.Record</c>. One value kind represents every
/// structured result so plugins can extend the vocabulary without new kinds, and
/// serialization stays AOT-safe over the closed set of type names.
/// </summary>
public sealed class RecordValue
{
    public string TypeName { get; }

    private readonly IReadOnlyList<RecordField> _fields;

    public RecordValue(string typeName, IReadOnlyList<RecordField> fields)
    {
        ArgumentNullException.ThrowIfNull(typeName);
        ArgumentNullException.ThrowIfNull(fields);
        TypeName = typeName;
        _fields = fields;
    }

    public RecordValue(string typeName, params RecordField[] fields)
        : this(typeName, (IReadOnlyList<RecordField>)fields)
    {
    }

    /// <summary>Fields in declaration order (deterministic serialization contract).</summary>
    public IReadOnlyList<RecordField> Fields => _fields;

    /// <summary>Case-sensitive field lookup (property access).</summary>
    public bool TryGetField(string name, out object? value)
    {
        foreach (var f in _fields)
        {
            if (string.Equals(f.Name, name, StringComparison.Ordinal))
            {
                value = f.Value;
                return true;
            }
        }
        value = null;
        return false;
    }
}

/// <summary>First-class mathematical domains for solver/assumption options
/// (<c>solve(expr, x, real)</c>, <c>symbol(name, integer)</c>). Not magic strings.</summary>
public enum MathDomain
{
    Integer,
    Rational,
    Real,
    Complex,
}
