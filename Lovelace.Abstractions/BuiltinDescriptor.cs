namespace Lovelace.Abstractions;

/// <summary>
/// The single source of truth for a builtin's discoverability metadata: help text, function
/// listings, Studio autocomplete, DSH tool schemas, and the generated documentation catalog
/// all derive from these descriptors (never from hand-maintained parallel catalogs).
/// <para>
/// The arity declared here is one source of truth for three surfaces at once: help/completion
/// rendering, DSH tool schemas, and the call-site validator in <c>Lovelace.Suite.ModusHost</c>
/// (<see cref="Parameters"/>.Count is the upper bound, <paramref name="MinArity"/> the lower bound,
/// and <paramref name="Variadic"/> removes the upper bound).
/// </para>
/// </summary>
/// <param name="Name">Builtin name (matches the registered name exactly).</param>
/// <param name="Parameters">Ordered parameter names (the registration signature — not duplicated).</param>
/// <param name="Category">One of the fixed categories (e.g. "Calculus", "Solving").</param>
/// <param name="Summary">One-line description.</param>
/// <param name="Examples">Executable lovelace snippets (kept in sync with the doctests).</param>
/// <param name="ReturnKind">What the convenience form returns (ValueKind name or a record type name).</param>
/// <param name="Related">See-also function names.</param>
/// <param name="Variadic">True when the last declared parameter may repeat, which removes the
/// upper bound on the argument count (the lower bound stays <paramref name="MinArity"/>, or
/// <see cref="Parameters"/>.Count when that is -1).</param>
/// <param name="MinArity">Minimum argument count (-1 ⇒ exactly <see cref="Parameters"/>.Count).
/// The host ENFORCES this declared arity at the call site, before the implementation body runs, so a
/// builtin that legitimately accepts fewer arguments than it declares must say so here.</param>
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
