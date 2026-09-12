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
