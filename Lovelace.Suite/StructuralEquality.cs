using Lovelace.Abstractions;
using Lovelace.Symbolics;

namespace Lovelace.Suite;

/// <summary>
/// Deep structural equality over <see cref="Value"/>: records compare by type name and field
/// order, arrays by shape and then elements, symbolic values by their CANONICAL form (never their
/// pretty text), and scalars by numeric value. Alignment-plan §82 asked for this because tests
/// otherwise assert field-by-field and silently stop covering a structure when a field is added.
/// </summary>
public static class StructuralEquality
{
    /// <summary>True when the two values are structurally identical.</summary>
    public static bool ValuesEqual(Value a, Value b) => FirstDifference(a, b) is null;

    /// <summary>True when the two records are structurally identical.</summary>
    public static bool RecordsEqual(RecordValue a, RecordValue b) =>
        FirstDifference(new Value(a), new Value(b)) is null;

    /// <summary>
    /// The first structural difference, as <c>path: reason</c>, or null when the values are equal.
    /// A test failure should say WHICH nested field differs, not merely that two blobs are unequal.
    /// </summary>
    public static string? FirstDifference(Value a, Value b, string path = "$")
    {
        if (a.Kind != b.Kind)
            return $"{path}: kind {a.Kind} != {b.Kind}";

        switch (a.Kind)
        {
            case ValueKind.Natural:
            case ValueKind.Integer:
            case ValueKind.Real:
                return NumericOps.Compare(a, b) == 0 ? null : $"{path}: {a.Kind} value differs";
            case ValueKind.Complex:
                var (ca, cb) = (a.AsComplex(), b.AsComplex());
                if (NumericOps.Compare(new Value(ca.Re), new Value(cb.Re)) != 0) return $"{path}.re: differs";
                if (NumericOps.Compare(new Value(ca.Im), new Value(cb.Im)) != 0) return $"{path}.im: differs";
                return null;
            case ValueKind.Boolean:
                return a.AsBoolean() == b.AsBoolean() ? null : $"{path}: boolean differs";
            case ValueKind.Text:
                return string.Equals(a.AsText(), b.AsText(), StringComparison.Ordinal)
                    ? null : $"{path}: text differs";
            case ValueKind.Enum:
            {
                // equal iff BOTH the declared enum type and the member match: SolveStatus.Partial
                // is not Completeness.Partial, which is the whole point of carrying the type
                var (ea, eb) = (a.AsEnum(), b.AsEnum());
                if (ea.TypeName != eb.TypeName)
                    return $"{path}: enum type {ea.TypeName} != {eb.TypeName}";
                return string.Equals(ea.Name, eb.Name, StringComparison.Ordinal)
                    ? null : $"{path}: enum {ea.TypeName} value {ea.Name} != {eb.Name}";
            }
            case ValueKind.Domain:
                return a.AsDomain() == b.AsDomain() ? null : $"{path}: domain differs";
            case ValueKind.Function:
                return a.AsFunction().Name == b.AsFunction().Name ? null : $"{path}: function differs";
            case ValueKind.Symbolic:
                // canonical form, never the pretty text: two renderings of one expression are equal
                return Printing.CanonicalPrint(a.AsSymbolic()) == Printing.CanonicalPrint(b.AsSymbolic())
                    ? null : $"{path}: symbolic canonical differs";
            case ValueKind.Vector:
            case ValueKind.Array:
            {
                var (va, vb) = (a.AsArrayValue(), b.AsArrayValue());
                var (sa, sb) = (va.Shape.ToArray(), vb.Shape.ToArray());
                if (!sa.SequenceEqual(sb))
                    return $"{path}: shape [{string.Join(",", sa)}] != [{string.Join(",", sb)}]";
                var ea = a.AsVector();
                var eb = b.AsVector();
                for (int i = 0; i < ea.Count; i++)
                {
                    var diff = FirstDifference(ea[i], eb[i], $"{path}[{i}]");
                    if (diff is not null) return diff;
                }
                return null;
            }
            case ValueKind.Record:
            {
                var (ra, rb) = (a.AsRecord(), b.AsRecord());
                if (ra.TypeName != rb.TypeName) return $"{path}: record type {ra.TypeName} != {rb.TypeName}";
                if (ra.Fields.Count != rb.Fields.Count)
                    return $"{path}: {ra.TypeName} has {ra.Fields.Count} fields, other has {rb.Fields.Count}";
                for (int i = 0; i < ra.Fields.Count; i++)
                {
                    var (fa, fb) = (ra.Fields[i], rb.Fields[i]);
                    if (fa.Name != fb.Name)
                        return $"{path}.{ra.TypeName}[{i}]: field name {fa.Name} != {fb.Name}";
                    var diff = FirstDifference((Value)fa.Value!, (Value)fb.Value!, $"{path}.{fa.Name}");
                    if (diff is not null) return diff;
                }
                return null;
            }
            default:
                return null;   // Void and any other valueless kind
        }
    }
}
