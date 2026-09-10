namespace Lovelace.Abstractions;

/// <summary>
/// The Suite↔Symbolics introspection seam: <c>inspect()</c> lives in the core, but only the
/// symbolic plugin knows the active assumptions. A plugin supplies this bridge at load time and
/// the core asks it for the assumptions that actually constrain an expression's symbols, so
/// introspection reports them as structured condition leaves rather than as text.
/// </summary>
public interface ISymbolicInspectionBridge
{
    /// <summary>Structured condition leaves (payloads) for the active assumptions that mention any
    /// symbol free in <paramref name="expression"/> (a symbolic <c>Expr</c> payload). Returns an
    /// empty array when the expression has no free symbols or nothing constrains them.</summary>
    object?[] RelevantAssumptions(object expression);
}
