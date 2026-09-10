namespace Lovelace.Abstractions;

/// <summary>
/// Ambient cancellation for long-running kernels. The host installs a scope around an evaluation
/// (<see cref="Scope"/>) and every algorithm that can run for an unbounded time polls
/// <see cref="ThrowIfCancellationRequested"/> at its loop head. The token is flow-local, so a
/// cancelled evaluation cannot cancel a concurrent one on another flow.
/// </summary>
public static class Cancellation
{
    private static readonly AsyncLocal<CancellationToken> _token = new();

    /// <summary>The active token, or <see cref="CancellationToken.None"/> when no scope is open.</summary>
    public static CancellationToken Token => _token.Value;

    /// <summary>Throws <see cref="OperationCanceledException"/> when the caller cancelled.
    /// Kernels call this at loop heads; a partial result must be left in whatever state the
    /// caller can still read.</summary>
    public static void ThrowIfCancellationRequested() => _token.Value.ThrowIfCancellationRequested();

    /// <summary>Installs <paramref name="token"/> for the current flow until disposed.</summary>
    public static IDisposable Scope(CancellationToken token)
    {
        var previous = _token.Value;
        _token.Value = token;
        return new ScopeHandle(previous);
    }

    private sealed class ScopeHandle(CancellationToken previous) : IDisposable
    {
        private CancellationToken? _previous = previous;
        public void Dispose()
        {
            if (_previous is { } p)
            {
                _token.Value = p;
                _previous = null;
            }
        }
    }
}
