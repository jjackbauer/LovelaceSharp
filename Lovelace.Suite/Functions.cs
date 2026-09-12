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

    /// <summary>
    /// The declared LOWER bound on the argument count (-1 ⇒ the declared parameter list is the
    /// exact count). This is the declaration the call-site validator in <see cref="Interpreter"/>
    /// enforces, together with <see cref="Parameters"/>.Count (the upper bound) and
    /// <see cref="Variadic"/>: a builtin that legitimately accepts FEWER arguments than it declares
    /// says so here instead of the validator being widened.
    /// </summary>
    public int MinArity { get; }

    /// <summary>
    /// True when the last declared parameter may repeat, which removes the upper bound on the
    /// argument count. The lower bound stays <see cref="MinArity"/> (or the declared count).
    /// </summary>
    public bool Variadic { get; }

    /// <summary>Source location, when known.</summary>
    public SourceSpan? Span { get; }

    /// <summary>Optional documentation text.</summary>
    public string? Documentation { get; }

    /// <summary>
    /// The Modus plugin that registered this builtin, when it arrived through the plugin seam
    /// (<see langword="null"/> for core builtins and user functions). Stamped by
    /// <see cref="ModusHost"/>.
    /// </summary>
    public string? PluginName { get; internal set; }

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
        // a user function takes exactly its declared parameters: there is no shorter or longer form
        MinArity = -1;
        Variadic = false;
    }

    /// <summary>Creates a built-in function definition.</summary>
    /// <param name="name">Builtin name as written in source.</param>
    /// <param name="parameters">Ordered parameter names; the list is the declared signature.</param>
    /// <param name="builtin">Native implementation.</param>
    /// <param name="minArity">Declared lower bound on the argument count (-1 ⇒ exactly
    /// <paramref name="parameters"/>.Count).</param>
    /// <param name="variadic">True when the last declared parameter may repeat.</param>
    public FunctionDefinition(string name, IReadOnlyList<string> parameters, BuiltinFunction builtin,
        int minArity = -1, bool variadic = false)
    {
        Name = name;
        Parameters = parameters;
        Body = Array.Empty<Statement>();
        Builtin = builtin;
        MinArity = minArity;
        Variadic = variadic;
    }

    public override string ToString() =>
        $"{Name}({string.Join(", ", Parameters)})" + (IsBuiltin ? " [builtin]" : string.Empty);

    /// <summary>
    /// The declared lower bound on the argument count (the declared parameter list is the upper
    /// bound, unless <see cref="Variadic"/> removes it). These two values plus the parameter list
    /// are the whole arity declaration the call-site validator reads.
    /// </summary>
    public (int Min, int Max) DeclaredArity()
    {
        int declared = Parameters.Count;
        int min = MinArity >= 0 ? Math.Min(MinArity, declared) : declared;
        return (min, Variadic ? int.MaxValue : declared);
    }
}

/// <summary>
/// A call-site arity failure: typed and structural rather than a framework index error. It derives
/// from <see cref="ArgumentException"/> so the runner's taxonomy classifies it as a RECOVERABLE
/// argument error (code <c>InvalidArgument</c>, category <c>TypeMismatch</c>) instead of an internal
/// invariant failure, and it names the builtin and both counts so the caller can fix the call
/// without parsing prose:
/// <c>compile_full(): expected 2 arguments; got 1.</c>
/// <para>
/// Thrown by the ONE call-site validator (<see cref="Interpreter"/>) from the builtin's declared
/// metadata — never by a builtin body after it has already indexed an argument.
/// </para>
/// </summary>
public sealed class BuiltinArityException : ArgumentException
{
    public BuiltinArityException(string builtin, int expectedMin, int expectedMax, bool variadic, int actual)
        : base(Describe(builtin, expectedMin, expectedMax, variadic, actual))
    {
        Builtin = builtin;
        ExpectedMin = expectedMin;
        ExpectedMax = expectedMax;
        Variadic = variadic;
        Actual = actual;
    }

    /// <summary>The builtin whose declared arity the call violated.</summary>
    public string Builtin { get; }

    /// <summary>The declared minimum argument count.</summary>
    public int ExpectedMin { get; }

    /// <summary>The declared upper bound (<see cref="int.MaxValue"/> when <see cref="Variadic"/>).</summary>
    public int ExpectedMax { get; }

    /// <summary>True when the last declared parameter may repeat.</summary>
    public bool Variadic { get; }

    /// <summary>The argument count the call actually supplied.</summary>
    public int Actual { get; }

    private static string Describe(string builtin, int min, int max, bool variadic, int actual)
    {
        string expected = variadic
            ? $"at least {min} {Argument(min)}"
            : min == max
                ? $"{min} {Argument(min)}"
                : $"{min} to {max} arguments";
        return $"{builtin}(): expected {expected}; got {actual}.";
    }

    private static string Argument(int count) => count == 1 ? "argument" : "arguments";
}

/// <summary>
/// A call-site SHAPE failure: the argument the builtin's declared parameter names is not the KIND
/// of thing the body requires — a bare scalar where a matrix is required, a symbolic value where a
/// text name is required, a non-permutation where an axis order is required. Like
/// <see cref="BuiltinArityException"/> it derives from <see cref="ArgumentException"/>, so the
/// runner's taxonomy classifies the call as the documented RECOVERABLE argument error
/// (<c>code: InvalidArgument</c>, <c>category: TypeMismatch</c>) — never as a domain refusal, and
/// never, as the direct cast it replaced did, as an internal invariant failure carrying the raw CLR
/// message (audit D, finding F1).
/// <para>
/// The message is the one grammar every argument-shape refusal shares, so a caller reads the
/// builtin, the 1-based position, what was required and what actually arrived without parsing
/// prose: <c>det(): argument 1 must be an array or vector; got Natural.</c>
/// </para>
/// </summary>
public sealed class BuiltinShapeException : ArgumentException
{
    public BuiltinShapeException(string builtin, int position, string expected, ValueKind arrived)
        : base(Describe(builtin, position, expected, arrived))
    {
        Builtin = builtin;
        Position = position;
        Expected = expected;
        Arrived = arrived;
    }

    /// <summary>The builtin whose argument shape the call violated.</summary>
    public string Builtin { get; }

    /// <summary>The 1-based position of the offending argument.</summary>
    public int Position { get; }

    /// <summary>What that position requires, as prose.</summary>
    public string Expected { get; }

    /// <summary>The kind that actually arrived in that position.</summary>
    public ValueKind Arrived { get; }

    private static string Describe(string builtin, int position, string expected, ValueKind arrived) =>
        $"{builtin}(): argument {position} must be {expected}; got {arrived}.";
}
