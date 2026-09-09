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
    private readonly SuiteEngine _engine;

    public HelpService(SuiteEngine engine) => _engine = engine;

    /// <summary>All builtins with their descriptors (synthesized entries where a builtin has
    /// none, so every builtin is discoverable). Deterministic order: name.</summary>
    private List<(string Name, BuiltinDescriptor Descriptor, string? Plugin)> Catalog()
    {
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
        return sb.ToString().TrimEnd('\n');
    }

    /// <summary>One-line-per-function listing of a category (or all categories when null).</summary>
    public string Funcs(string? category)
    {
        var catalog = Catalog();
        var sb = new System.Text.StringBuilder();
        if (category is not null)
        {
            var functions = catalog.Where(c => string.Equals(c.Descriptor.Category, category, StringComparison.OrdinalIgnoreCase)).ToList();
            if (functions.Count == 0)
                return $"No category '{category}'. Known categories: {string.Join(", ", BuiltinCategories.Order)}.";
            sb.AppendLine($"{CanonicalCategory(functions[0].Descriptor.Category)}");
            foreach (var (name, descriptor, _) in functions)
                sb.AppendLine($"  {Signature(name, descriptor)}");
            return sb.ToString().TrimEnd('\n');
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
        var functions = catalog
            .Where(c => string.Equals(c.Descriptor.Category, category, StringComparison.OrdinalIgnoreCase))
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

    /// <summary>Resolves a help argument as a category or a function (function wins).</summary>
    public string? Lookup(string arg)
    {
        var fn = Function(arg);
        if (fn is not null)
            return fn;
        return Category(arg);
    }

    private static string Signature(string name, BuiltinDescriptor d) =>
        $"{name}({string.Join(", ", d.Parameters)})";

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
