namespace Lovelace.Abstractions;

// -------------------------------------------------------------------------
// Enumerated values (the DSH protocol's Enum structured kind)
// -------------------------------------------------------------------------

/// <summary>
/// A first-class enumerated value: the DECLARED enum type name (<c>SolveStatus</c>,
/// <c>Completeness</c>, <c>SolutionExactness</c>, …) together with the member name
/// (<c>Partial</c>, <c>Exact</c>, …). Plugins construct these over the Modus payload boundary —
/// they live in Abstractions so no plugin references Suite — and the Suite core maps them onto
/// <c>ValueKind.Enum</c>, exactly as <see cref="RecordValue"/> maps onto <c>ValueKind.Record</c>.
/// <para>
/// An enum-valued field must never cross the wire as text: <c>SolveStatus.Partial</c> and
/// <c>Completeness.Partial</c> are different answers with the same spelling, so an agent reading
/// a string would have to guess which enum it belongs to, and a renamed member would silently
/// re-spell the protocol. The declared type name is part of the value for that reason.
/// </para>
/// </summary>
public sealed record EnumValue(string TypeName, string Name);
