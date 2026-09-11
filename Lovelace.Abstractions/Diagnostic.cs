namespace Lovelace.Abstractions;

// -------------------------------------------------------------------------
// Structured diagnostics (the machine-readable failure vocabulary)
// -------------------------------------------------------------------------

/// <summary>
/// The one error taxonomy a consumer switches on instead of matching message text (alignment
/// plan section C). The member NAME is the wire value and travels with the DECLARED enum type
/// name (<c>ErrorCategory</c>), so a category is never ambiguous with another enum's member.
/// </summary>
public enum ErrorCategory
{
    ParseError,
    DomainError,
    UnsupportedOperation,
    BudgetExceeded,
    NoSolution,
    TypeMismatch,
    InternalInvariantFailure,
}

/// <summary>
/// A 1-based source range for a diagnostic that has one. This is the WIRE-level location: it
/// lives in Abstractions (which no plugin escapes) rather than reusing the host's own
/// <c>Lovelace.Suite.SourceSpan</c>, which is unreachable from here.
/// </summary>
public sealed record DiagnosticLocation(int StartLine, int StartColumn, int EndLine, int EndColumn);

/// <summary>
/// One machine-readable diagnostic: a stable <c>Code</c> a consumer matches, the
/// <see cref="ErrorCategory"/> member, a human-readable <c>Message</c>, whether the caller can
/// continue, an optional source <c>Location</c>, and nested <c>Details</c>.
/// <para>
/// Two invariants are part of the shape: a kernel-level diagnostic has no source span at that
/// layer, so <c>Location</c> is null, and <c>Details</c> is an EMPTY LIST when there are none.
/// A Diagnostic never carries <c>""</c> where a null or an empty list is meant.
/// </para>
/// </summary>
public sealed record Diagnostic(
    string Code,
    ErrorCategory Category,
    string Message,
    bool Recoverable,
    DiagnosticLocation? Location,
    IReadOnlyList<Diagnostic> Details)
{
    /// <summary>The kernel-level construction: no source span exists below the host, and there
    /// are no nested details.</summary>
    public static Diagnostic Of(
        string code, ErrorCategory category, string message, bool recoverable = true) =>
        new(code, category, message, recoverable, null, Array.Empty<Diagnostic>());
}

/// <summary>
/// The ONE projection of a <see cref="Diagnostic"/> onto the wire, in Abstractions for the same
/// reason <see cref="RecordValue"/> is: Symbolics, MathIR and every later plugin must emit an
/// identical shape without referencing the host.
/// </summary>
public static class DiagnosticProjection
{
    /// <summary>The wire type name of a diagnostic record.</summary>
    public const string TypeName = "Diagnostic";

    /// <summary>The declared enum type name the <c>category</c> field carries.</summary>
    public const string CategoryTypeName = "ErrorCategory";

    /// <summary>The wire type name of a source-location record.</summary>
    public const string LocationTypeName = "DiagnosticLocation";

    /// <summary>Projects a diagnostic to a "Diagnostic" record with the field names
    /// <c>code</c>, <c>category</c>, <c>message</c>, <c>recoverable</c>, <c>location</c>,
    /// <c>details</c> — in that order, which is the contract. <c>category</c> is an Enum value of
    /// type <see cref="CategoryTypeName"/>; <c>location</c> is Null when absent.</summary>
    public static RecordValue ToRecordValue(Diagnostic diagnostic)
    {
        ArgumentNullException.ThrowIfNull(diagnostic);
        return new RecordValue(TypeName,
            new RecordField("code", diagnostic.Code),
            new RecordField("category", new EnumValue(CategoryTypeName, diagnostic.Category.ToString())),
            new RecordField("message", diagnostic.Message),
            new RecordField("recoverable", diagnostic.Recoverable),
            // Null, never "": the kernel has no source span to report
            new RecordField("location", diagnostic.Location is { } location ? ToRecordValue(location) : null),
            new RecordField("details", ToRecordValues(diagnostic.Details)));
    }

    /// <summary>Projects a diagnostic list to payload elements. An EMPTY list becomes an EMPTY
    /// ARRAY — never null and never an empty string — because the caller always emits the field.</summary>
    public static object?[] ToRecordValues(IReadOnlyList<Diagnostic> diagnostics)
    {
        ArgumentNullException.ThrowIfNull(diagnostics);
        if (diagnostics.Count == 0)
            return Array.Empty<object?>();
        var values = new object?[diagnostics.Count];
        for (int i = 0; i < diagnostics.Count; i++)
            values[i] = ToRecordValue(diagnostics[i]);
        return values;
    }

    /// <summary>Projects a source location to a "DiagnosticLocation" record.</summary>
    public static RecordValue ToRecordValue(DiagnosticLocation location)
    {
        ArgumentNullException.ThrowIfNull(location);
        return new RecordValue(LocationTypeName,
            new RecordField("start_line", location.StartLine),
            new RecordField("start_column", location.StartColumn),
            new RecordField("end_line", location.EndLine),
            new RecordField("end_column", location.EndColumn));
    }
}
