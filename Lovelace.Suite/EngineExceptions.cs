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
