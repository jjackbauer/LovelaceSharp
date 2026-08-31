namespace Lovelace.Suite;

/// <summary>A 1-based source range (start and end line/column).</summary>
public sealed record SourceSpan(int StartLine, int StartColumn, int EndLine, int EndColumn);

/// <summary>An error or warning with its source position.</summary>
public sealed record Diagnostic(string Message, int Position, int Line, int Column)
{
    public override string ToString() => $"(line {Line}, col {Column}) {Message}";
}

/// <summary>Event payload for a variable change (define, reassign, or remove).</summary>
public sealed class VariableChangedEventArgs : EventArgs
{
    public string Name { get; }
    public Value Value { get; }
    public bool Removed { get; }

    public VariableChangedEventArgs(string name, Value value, bool removed = false)
    {
        Name = name;
        Value = value;
        Removed = removed;
    }
}

/// <summary>Event payload for a function definition.</summary>
public sealed class FunctionDefinedEventArgs : EventArgs
{
    public FunctionDefinition Definition { get; }

    public FunctionDefinedEventArgs(FunctionDefinition definition) => Definition = definition;
}

/// <summary>Serializable variable view used by <see cref="StateSnapshot"/>.</summary>
public sealed record StateVariable(string Name, ValueKind Kind, string Display);

/// <summary>Serializable function view used by <see cref="StateSnapshot"/>.</summary>
public sealed record StateFunction(
    string Name,
    IReadOnlyList<string> Parameters,
    bool IsBuiltin,
    SourceSpan? Span);

/// <summary>
/// An immutable capture of the engine's variables and functions, with a revision
/// counter so hosts can detect staleness.
/// </summary>
public sealed class StateSnapshot
{
    public long Revision { get; }
    public IReadOnlyDictionary<string, StateVariable> Variables { get; }
    public IReadOnlyDictionary<string, StateFunction> Functions { get; }

    public StateSnapshot(
        long revision,
        IReadOnlyDictionary<string, StateVariable> variables,
        IReadOnlyDictionary<string, StateFunction> functions)
    {
        Revision = revision;
        Variables = variables;
        Functions = functions;
    }
}
