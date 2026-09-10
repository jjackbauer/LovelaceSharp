using Lovelace.Abstractions;
using Lovelace.Suite;

namespace Lovelace.Suite;

/// <summary>
/// The plugin-aware help/introspection surface: derives the REPL's <c>help</c>/<c>funcs</c>
/// output, Studio completions, and the DSH tool schema from the live function registry plus
/// the builtin descriptors (plugin-provided and <see cref="CoreBuiltinMetadata"/>). No help
/// text is hard-coded in the REPL; this service is the single renderer.
/// </summary>
public sealed class HelpService
{
    /// <summary>
    /// The documented prelude contract for <see cref="BuiltinDescriptor.Examples"/>: a doctest
    /// runner evaluates these statements once, on a fresh engine, before each example, so an
    /// example may use <c>x</c>/<c>y</c>/<c>t</c> without defining them itself. The set is
    /// deliberately tiny and stable — adding to it widens what every example may silently
    /// assume, so descriptors that need anything else must bind it inside the example.
    /// </summary>
    public static readonly IReadOnlyList<string> ExamplePrelude = new[]
    {
        "x = symbol(\"x\")",
        "y = symbol(\"y\")",
        "t = symbol(\"t\")",
    };

    /// <summary>The prelude as one runnable, <c>;</c>-separated statement list.</summary>
    public static string ExamplePreludeSource => string.Join("; ", ExamplePrelude);

    private readonly SuiteEngine _engine;

    public HelpService(SuiteEngine engine) => _engine = engine;

    /// <summary>Cached catalog plus the registry size it was built from. The registry is
    /// add-only (plugins load builtins), so the count is a sound invalidation stamp: help and
    /// funcs no longer re-sort and re-allocate the whole registry on every call.</summary>
    private List<(string Name, BuiltinDescriptor Descriptor, string? Plugin)>? _catalog;
    private int _catalogStamp = -1;

    /// <summary>All builtins with their descriptors (synthesized entries where a builtin has
    /// none, so every builtin is discoverable). Deterministic order: name.
    /// <para>Cached: callers must treat the result as read-only.</para></summary>
    private List<(string Name, BuiltinDescriptor Descriptor, string? Plugin)> Catalog()
    {
        int stamp = _engine.Functions.Count;
        if (_catalog is not null && _catalogStamp == stamp)
            return _catalog;

        var list = new List<(string, BuiltinDescriptor, string?)>();
        foreach (var (name, fn) in _engine.Functions.OrderBy(kv => kv.Key, StringComparer.Ordinal))
        {
            if (!fn.IsBuiltin)
                continue;
            var descriptor = _engine.InterpreterBuiltinDescriptors.TryGetValue(name, out var d)
                ? d
                : CoreBuiltinMetadata.TryGet(name)
                ?? new BuiltinDescriptor(name, fn.Parameters, BuiltinCategories.Core,
                    "(no summary registered)", Array.Empty<string>(), "Value");
            list.Add((name, descriptor, fn.PluginName));
        }
        _catalog = list;
        _catalogStamp = stamp;
        return list;
    }

    /// <summary>The top-level help view: major categories with their function counts.</summary>
    public string Overview()
    {
        var catalog = Catalog();
        var sb = new System.Text.StringBuilder();
        sb.AppendLine("LovelaceSharp — help");
        sb.AppendLine("─────────────────────────────────────────────────");
        sb.AppendLine("Categories:");
        foreach (var category in BuiltinCategories.Order)
        {
            var functions = catalog.Where(c => c.Descriptor.Category == category).ToList();
            if (functions.Count == 0)
                continue;
            sb.AppendLine($"  {category,-16} {functions.Count} function(s)");
        }
        sb.AppendLine();
        sb.AppendLine("help <category>       list a category's functions");
        sb.AppendLine("help <function>       signature, summary, examples");
        sb.AppendLine("funcs [category]      categorized function listing");
        sb.AppendLine("vars / clear / delete / run <file> / set precision|display|pretty");
        sb.AppendLine("Language statements & operators: docs/Language.md");
        sb.AppendLine();
        sb.AppendLine("Every function example runs after this prelude:");
        sb.AppendLine("  " + ExamplePreludeSource);
        return sb.ToString().TrimEnd('\n');
    }

    /// <summary>One-line-per-function listing of a category (or all categories when null).</summary>
    public string Funcs(string? category)
    {
        var catalog = Catalog();
        var sb = new System.Text.StringBuilder();
        if (category is not null)
        {
            var resolved = ResolveCategory(category) ?? category;
            var functions = catalog.Where(c => string.Equals(c.Descriptor.Category, resolved, StringComparison.OrdinalIgnoreCase)).ToList();
            if (functions.Count > 0)
            {
                sb.AppendLine($"{CanonicalCategory(functions[0].Descriptor.Category)}");
                foreach (var (name, descriptor, _) in functions)
                    sb.AppendLine($"  {Signature(name, descriptor)}");
                return sb.ToString().TrimEnd('\n');
            }
            // not a category: fall back to a name search so `funcs solve` finds the solvers
            var matches = catalog
                .Where(c => c.Name.Contains(category, StringComparison.OrdinalIgnoreCase))
                .ToList();
            if (matches.Count > 0)
            {
                sb.AppendLine($"Matching '{category}':");
                foreach (var (name, descriptor, _) in matches)
                    sb.AppendLine($"  {Signature(name, descriptor)}  - {descriptor.Summary}");
                return sb.ToString().TrimEnd('\n');
            }
            return $"No category or function matching '{category}'. Known categories: {string.Join(", ", BuiltinCategories.Order)}.";
        }
        foreach (var cat in BuiltinCategories.Order)
        {
            var functions = catalog.Where(c => c.Descriptor.Category == cat).ToList();
            if (functions.Count == 0)
                continue;
            sb.AppendLine(cat);
            foreach (var (name, descriptor, _) in functions)
                sb.AppendLine($"  {Signature(name, descriptor)}");
            sb.AppendLine();
        }
        return sb.ToString().TrimEnd('\n');
    }

    /// <summary>Category listing (names + summaries).</summary>
    public string Category(string category)
    {
        var catalog = Catalog();
        var resolved = ResolveCategory(category);
        var functions = catalog
            .Where(c => string.Equals(c.Descriptor.Category, resolved ?? category, StringComparison.OrdinalIgnoreCase))
            .ToList();
        if (functions.Count == 0)
            return $"No category '{category}'. Known categories: {string.Join(", ", BuiltinCategories.Order)}.";
        var sb = new System.Text.StringBuilder();
        sb.AppendLine(CanonicalCategory(functions[0].Descriptor.Category));
        foreach (var (name, descriptor, _) in functions)
            sb.AppendLine($"  {Signature(name, descriptor)}");
        return sb.ToString().TrimEnd('\n');
    }

    /// <summary>Full help for one function: signature, summary, examples, return kind, see-also.</summary>
    public string? Function(string name)
    {
        if (!_engine.Functions.TryGetValue(name, out var fn) || !fn.IsBuiltin)
            return null;
        var descriptor = _engine.InterpreterBuiltinDescriptors.TryGetValue(name, out var d)
            ? d
            : CoreBuiltinMetadata.TryGet(name);
        if (descriptor is null)
            return $"{Signature(name, new BuiltinDescriptor(name, fn.Parameters, BuiltinCategories.Core, "(no summary registered)", Array.Empty<string>(), "Value"))}\n\n(no further help registered)";

        var sb = new System.Text.StringBuilder();
        sb.AppendLine(Signature(name, descriptor));
        sb.AppendLine();
        sb.AppendLine(descriptor.Summary);
        if (descriptor.Examples.Count > 0)
        {
            sb.AppendLine();
            sb.AppendLine("Examples:");
            foreach (var e in descriptor.Examples)
                sb.AppendLine($"    {e}");
        }
        sb.AppendLine();
        sb.AppendLine($"Returns: {descriptor.ReturnKind}");
        if (descriptor.Related.Count > 0)
            sb.AppendLine($"See also: {string.Join(", ", descriptor.Related)}");
        if (fn.PluginName is { } plugin)
            sb.AppendLine($"Plugin: {plugin}");
        return sb.ToString().TrimEnd('\n');
    }

    /// <summary>Resolves a help argument as a function or a category (function wins, then a
    /// category alias, then the registered category spelling).</summary>
    public string? Lookup(string arg)
    {
        var fn = Function(arg);
        if (fn is not null)
            return fn;
        var category = Category(arg);
        if (!category.StartsWith("No category", StringComparison.Ordinal))
            return category;
        var alias = ResolveCategory(arg);
        return alias is null ? category : Category(alias);
    }

    /// <summary>Renders a signature that distinguishes required, optional and variadic
    /// parameters from the descriptor's arity metadata (one source of truth for validation,
    /// help and completion).</summary>
    internal static string Signature(string name, BuiltinDescriptor d)
    {
        int min = d.MinArity >= 0 ? Math.Min(d.MinArity, d.Parameters.Count) : d.Parameters.Count;
        var required = new List<string>();
        for (int i = 0; i < min; i++)
            required.Add(d.Parameters[i]);
        var optional = new List<string>();
        for (int i = min; i < d.Parameters.Count; i++)
            optional.Add(d.Variadic && i == d.Parameters.Count - 1 ? d.Parameters[i] + "..." : d.Parameters[i]);
        var text = name + "(" + string.Join(", ", required);
        if (optional.Count > 0)
            text += (required.Count > 0 ? " " : "") + "[, " + string.Join(", ", optional) + "]";
        return text + ")";
    }

    /// <summary>Reasonable synonyms so a user can find a category without knowing its exact
    /// registered spelling.</summary>
    private static readonly Dictionary<string, string> CategoryAliases = new(StringComparer.OrdinalIgnoreCase)
    {
        ["symbolic"] = BuiltinCategories.Symbolics,
        ["symbolics"] = BuiltinCategories.Symbolics,
        ["algebra"] = BuiltinCategories.Symbolics,
        ["cas"] = BuiltinCategories.Symbolics,
        ["calculus"] = BuiltinCategories.Calculus,
        ["matrices"] = BuiltinCategories.LinearAlgebra,
        ["matrix"] = BuiltinCategories.LinearAlgebra,
        ["linearalgebra"] = BuiltinCategories.LinearAlgebra,
        ["linear algebra"] = BuiltinCategories.LinearAlgebra,
        ["mathir"] = BuiltinCategories.Compilation,
        ["ir"] = BuiltinCategories.Compilation,
        ["compilation"] = BuiltinCategories.Compilation,
        ["solver"] = BuiltinCategories.Solving,
        ["solvers"] = BuiltinCategories.Solving,
        ["solving"] = BuiltinCategories.Solving,
        ["optimisation"] = BuiltinCategories.Optimization,
        ["dsp"] = BuiltinCategories.Dsp,
        ["signal"] = BuiltinCategories.Dsp,
        ["arrays"] = BuiltinCategories.Arrays,
        ["numeric"] = BuiltinCategories.Numerics,
        ["numbers"] = BuiltinCategories.Numerics,
        ["types"] = BuiltinCategories.Introspection,
        ["introspection"] = BuiltinCategories.Introspection,
        ["language"] = BuiltinCategories.Language,
        ["core"] = BuiltinCategories.Core,
    };

    private static string? ResolveCategory(string category) =>
        CategoryAliases.TryGetValue(category.Trim(), out var alias) ? alias : null;

    private static string CanonicalCategory(string category)
    {
        foreach (var c in BuiltinCategories.Order)
        {
            if (string.Equals(c, category, StringComparison.OrdinalIgnoreCase))
                return c;
        }
        return category;
    }
}
