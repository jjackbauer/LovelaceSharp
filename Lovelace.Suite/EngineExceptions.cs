namespace Lovelace.Suite;

/// <summary>
/// Raised when a host starts an evaluation on an engine that is already evaluating in the same
/// async flow. The engine serialises evaluations per engine, so a nested call cannot proceed
/// without deadlocking on the gate; it is reported as a structured, recoverable diagnostic
/// (<c>ReentrancyNotSupported</c>) instead of hanging. Create a second engine for the nested
/// evaluation.
/// </summary>
public sealed class ReentrancyNotSupportedException : InvalidOperationException
{
    public ReentrancyNotSupportedException()
        : base("this engine is already evaluating in the current flow: nested EvaluateAsync on one "
               + "engine is not supported (it would deadlock on the evaluation gate). Create a "
               + "second engine for the nested evaluation.")
    {
    }
}

/// <summary>
/// A plot output path the process cannot write: the file name comes from the caller
/// (<c>--plot-file</c> / the engine's <c>PlotFileName</c>) and the directory from the caller
/// (<c>--plot-dir</c> / <c>PlotOutputDirectory</c>), so an unwritable combination is a property of the
/// ARGUMENT, like an unreadable script file — never an internal invariant failure (F3-C,
/// <c>docs/goal-cycle-6/round-13/audit-C-hostile.md:136-172</c>). The runner classifies it as the
/// recoverable <c>PlotFileError</c>/<c>TypeMismatch</c>, the twin of the <c>PlotDirectoryError</c> the
/// plot-directory preflight already publishes.
/// </summary>
public sealed class PlotFileWriteException : Exception
{
    /// <summary>The path the caller asked for (the plot directory joined with the plot file name).</summary>
    public string Path { get; }

    /// <param name="path">The path the caller asked for.</param>
    /// <param name="cause">The underlying path/IO failure; its message is kept in this one so the
    /// diagnostic never loses the OS's own sentence.</param>
    public PlotFileWriteException(string path, Exception cause)
        : base($"Cannot write the plot file '{path}': {cause.Message}", cause)
    {
        Path = path;
    }
}

/// <summary>
/// An evaluation stopped by the caller's <see cref="CancellationToken"/>. Distinct from a generic
/// cancellation: the host can report it as a structured <c>Cancelled</c> result, and the engine
/// state it already committed (variables, captured output) is the partial result.
/// </summary>
public sealed class EvaluationCancelledException : OperationCanceledException
{
    public EvaluationCancelledException(CancellationToken token)
        : base("the evaluation was cancelled by the caller.", token)
    {
    }
}

/// <summary>
/// A call to a name the engine does not register. Distinct from a domain failure: nothing about the
/// caller's mathematics is wrong, the vocabulary is. It derives from
/// <see cref="InvalidOperationException"/> so every host that already handles an unresolved call
/// keeps working, and it carries the registry's answer with it - the closest registered names, any
/// syntax note that applies, and how many names the engine does have.
/// <para>
/// The message is unchanged when the engine has nothing to offer (<c>Unknown function 'x'.</c>), so
/// no recorded message contract regresses; the structured fields travel either way.
/// </para>
/// </summary>
public sealed class UnknownFunctionException : InvalidOperationException
{
    /// <summary>The name as written in the source.</summary>
    public string FunctionName { get; }

    /// <summary>Registered names closest to <see cref="FunctionName"/>, best first (may be empty).</summary>
    public IReadOnlyList<string> Suggestions { get; }

    /// <summary>Syntax notes for a name that is not a missing builtin (may be empty).</summary>
    public IReadOnlyList<string> Hints { get; }

    /// <summary>How many names the engine registers, so a caller can size its ignorance.</summary>
    public int RegistrySize { get; }

    /// <summary>
    /// The suggestions as the wire carries them: null when there are none, because the optional field
    /// is OMITTED rather than sent as an empty array — "nothing is close" and "nobody looked" must not
    /// look alike to a consumer.
    /// </summary>
    public string[]? WireSuggestions => Suggestions.Count == 0 ? null : Suggestions.ToArray();

    /// <summary>The hints as the wire carries them: null when there are none (see <see cref="WireSuggestions"/>).</summary>
    public string[]? WireHints => Hints.Count == 0 ? null : Hints.ToArray();

    /// <param name="functionName">The unresolved name, as written.</param>
    /// <param name="suggestions">Registered names close to it; empty when none are.</param>
    /// <param name="hints">Syntax notes that apply; empty when none do.</param>
    /// <param name="registrySize">The number of registered names.</param>
    public UnknownFunctionException(string functionName, IReadOnlyList<string> suggestions,
        IReadOnlyList<string> hints, int registrySize)
        : base(Describe(functionName, suggestions, hints))
    {
        FunctionName = functionName;
        Suggestions = suggestions;
        Hints = hints;
        RegistrySize = registrySize;
    }

    private static string Describe(string functionName, IReadOnlyList<string> suggestions,
        IReadOnlyList<string> hints)
    {
        var message = $"Unknown function '{functionName}'.";
        if (suggestions.Count == 1)
            message += $" Did you mean '{suggestions[0]}'?";
        else if (suggestions.Count > 1)
            message += " Did you mean one of: " +
                string.Join(", ", suggestions.Select(name => $"'{name}'")) + "?";
        if (hints.Count > 0)
            message += " " + hints[0];
        return message;
    }
}
