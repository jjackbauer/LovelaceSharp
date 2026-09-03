using System.Collections.Concurrent;

namespace Lovelace.Studio;

/// <summary>
/// In-memory registry of live sessions, keyed by session id. Sessions are created on
/// demand, looked up by the <c>X-Session-Id</c> header, and evicted after an idle timeout.
/// Persistence is intentionally in-memory only (lost on server restart).
/// </summary>
public sealed class SessionRegistry
{
    private readonly ConcurrentDictionary<string, Session> _sessions = new();
    private readonly TimeSpan _idleTimeout = TimeSpan.FromHours(2);

    /// <summary>Creates and registers a new session.</summary>
    public Session Create()
    {
        var session = new Session();
        _sessions[session.Id] = session;
        return session;
    }

    /// <summary>Looks up a session and refreshes its last-access time; returns <see langword="null"/> if absent.</summary>
    public Session? Get(string sessionId)
    {
        if (string.IsNullOrEmpty(sessionId))
            return null;

        if (_sessions.TryGetValue(sessionId, out var session))
        {
            session.LastAccessed = DateTimeOffset.UtcNow;
            return session;
        }
        return null;
    }

    /// <summary>Removes a session; returns <see langword="true"/> if it existed.</summary>
    public bool Remove(string sessionId) => _sessions.TryRemove(sessionId, out _);

    /// <summary>Number of live sessions.</summary>
    public int Count => _sessions.Count;

    /// <summary>Evicts sessions idle beyond the timeout. Call occasionally or on create.</summary>
    public void Sweep()
    {
        var cutoff = DateTimeOffset.UtcNow - _idleTimeout;
        foreach (var (id, session) in _sessions)
        {
            if (session.LastAccessed < cutoff)
                _sessions.TryRemove(id, out _);
        }
    }
}
