namespace Lovelace.Suite;

/// <summary>
/// A native (built-in) function implementation. Receives the already-evaluated
/// arguments.
/// </summary>
public delegate Task<Value> BuiltinFunction(IReadOnlyList<Value> arguments);

/// <summary>
/// Metadata and implementation for a user-defined or built-in function.
/// User functions carry a statement body; built-ins carry a delegate.
/// </summary>
public sealed class FunctionDefinition
{
    /// <summary>Function name as written in source.</summary>
    public string Name { get; }

    /// <summary>Ordered parameter names.</summary>
    public IReadOnlyList<string> Parameters { get; }

    /// <summary>Statement body (user functions only; empty for built-ins).</summary>
    public IReadOnlyList<Statement> Body { get; }

    /// <summary>Native implementation (built-ins only; <see langword="null"/> for user functions).</summary>
    public BuiltinFunction? Builtin { get; }

    /// <summary><see langword="true"/> for native/built-in functions.</summary>
    public bool IsBuiltin => Builtin is not null;

    /// <summary>Source location, when known.</summary>
    public SourceSpan? Span { get; }

    /// <summary>Optional documentation text.</summary>
    public string? Documentation { get; }

    /// <summary>Creates a user-defined function definition.</summary>
    public FunctionDefinition(
        string name,
        IReadOnlyList<string> parameters,
        IReadOnlyList<Statement> body,
        SourceSpan? span = null,
        string? documentation = null)
    {
        Name = name;
        Parameters = parameters;
        Body = body;
        Builtin = null;
        Span = span;
        Documentation = documentation;
    }

    /// <summary>Creates a built-in function definition.</summary>
    public FunctionDefinition(string name, IReadOnlyList<string> parameters, BuiltinFunction builtin)
    {
        Name = name;
        Parameters = parameters;
        Body = Array.Empty<Statement>();
        Builtin = builtin;
    }

    public override string ToString() =>
        $"{Name}({string.Join(", ", Parameters)})" + (IsBuiltin ? " [builtin]" : string.Empty);
}
