namespace Lovelace.Suite;

/// <summary>
/// A lexical environment: a chain of name → value bindings. Lookup walks up the
/// parent chain; definition always binds in the current scope. This yields the
/// required scoping model: parameters and function/block locals shadow globals
/// and never leak outward, while the top-level (REPL/script) scope is global.
/// </summary>
public sealed class Scope
{
    private readonly Dictionary<string, Value> _values = new();

    /// <summary>The enclosing scope, or <see langword="null"/> for the global scope.</summary>
    public Scope? Parent { get; }

    public Scope(Scope? parent = null) => Parent = parent;

    /// <summary>A read-only view of this scope's own bindings (not parents).</summary>
    public IReadOnlyDictionary<string, Value> Values => _values;

    /// <summary>Looks up a name, walking up the parent chain.</summary>
    public bool TryGet(string name, out Value value)
    {
        for (Scope? s = this; s is not null; s = s.Parent)
        {
            if (s._values.TryGetValue(name, out value!))
                return true;
        }

        value = default!;
        return false;
    }

    /// <summary>Defines or overwrites a binding in the current scope only.</summary>
    public void Define(string name, Value value) => _values[name] = value;

    /// <summary>
    /// Assigns <paramref name="name"/> using standard block-scoped semantics:
    /// updates the nearest enclosing binding if one exists, otherwise defines a
    /// new binding in the current (innermost) scope. Returns the scope that was
    /// written to.
    /// </summary>
    public Scope Assign(string name, Value value)
    {
        for (Scope? s = this; s is not null; s = s.Parent)
        {
            if (s._values.ContainsKey(name))
            {
                s._values[name] = value;
                return s;
            }
        }

        _values[name] = value;
        return this;
    }

    /// <summary>Removes a binding from the current scope only.</summary>
    public bool Remove(string name) => _values.Remove(name);

    /// <summary>Removes all bindings from the current scope.</summary>
    public void Clear() => _values.Clear();
}
