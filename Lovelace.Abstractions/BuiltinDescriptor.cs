namespace Lovelace.Abstractions;

/// <summary>
/// The single source of truth for a builtin's discoverability metadata: help text, function
/// listings, Studio autocomplete, DSH tool schemas, and the generated documentation catalog
/// all derive from these descriptors (never from hand-maintained parallel catalogs).
/// </summary>
/// <param name="Name">Builtin name (matches the registered name exactly).</param>
/// <param name="Parameters">Ordered parameter names (the registration signature — not duplicated).</param>
/// <param name="Category">One of the fixed categories (e.g. "Calculus", "Solving").</param>
/// <param name="Summary">One-line description.</param>
/// <param name="Examples">Executable lovelace snippets (kept in sync with the doctests).</param>
/// <param name="ReturnKind">What the convenience form returns (ValueKind name or a record type name).</param>
/// <param name="Related">See-also function names.</param>
/// <param name="Variadic">True when the last parameter may repeat / be omitted.</param>
/// <param name="MinArity">Minimum argument count (-1 ⇒ exactly <see cref="Parameters"/>.Count).</param>
public sealed record BuiltinDescriptor(
    string Name,
    IReadOnlyList<string> Parameters,
    string Category,
    string Summary,
    IReadOnlyList<string> Examples,
    string ReturnKind,
    IReadOnlyList<string>? Related = null,
    bool Variadic = false,
    int MinArity = -1)
{
    public IReadOnlyList<string> Related { get; } = Related ?? Array.Empty<string>();
}

/// <summary>The fixed help/funcs categories. Unclassified core builtins report "Core".</summary>
public static class BuiltinCategories
{
    public const string Language = "Language";
    public const string Arrays = "Arrays";
    public const string Numerics = "Numerics";
    public const string Symbolics = "Symbolics";
    public const string Calculus = "Calculus";
    public const string Solving = "Solving";
    public const string LinearAlgebra = "Linear Algebra";
    public const string Optimization = "Optimization";
    public const string Compilation = "Compilation";
    public const string Dsp = "DSP";
    public const string Introspection = "Introspection";
    public const string Core = "Core";

    /// <summary>Display order for help/funcs.</summary>
    public static readonly IReadOnlyList<string> Order = new[]
    {
        Language, Arrays, Numerics, Symbolics, Calculus, Solving,
        LinearAlgebra, Optimization, Compilation, Dsp, Introspection, Core,
    };
}
