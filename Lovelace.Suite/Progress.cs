namespace Lovelace.Suite;

/// <summary>A progress update from a long-running operation: a label and a 0..1 fraction.</summary>
public readonly record struct OperationProgress(string Label, double Fraction);

/// <summary>
/// A synchronous <see cref="IProgress{T}"/> that invokes the handler inline (no thread-pool hop).
/// Used so progress from CPU-bound parallel work is delivered immediately instead of being queued
/// behind that work on the thread pool.
/// </summary>
public sealed class SyncProgress<T> : IProgress<T>
{
    private readonly Action<T> _report;
    public SyncProgress(Action<T> report) => _report = report;
    public void Report(T value) => _report(value);
}
