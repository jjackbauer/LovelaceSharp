using System.Collections.Concurrent;
using Lovelace.Suite;

namespace Lovelace.Studio;

/// <summary>
/// A single interactive session: one <see cref="SuiteEngine"/> (variables, functions,
/// revision), its own precision, a gate serializing that session's runs, and a slot for
/// the incremental computation cache. Sessions are independent and may run concurrently.
/// </summary>
public sealed class Session
{
    /// <summary>Default precision for a freshly created session (decimal places, applied to both computation and display).</summary>
    public const long DefaultPrecision = 1000L;

    /// <summary>Opaque, URL-safe session id.</summary>
    public string Id { get; } = Guid.NewGuid().ToString("N");

    /// <summary>The engine that owns this session's variables and functions.</summary>
    public SuiteEngine Engine { get; } = new();

    /// <summary>Serializes evaluations within this session (one run at a time per session).</summary>
    internal SemaphoreSlim Gate { get; } = new(1, 1);

    /// <summary>Last activity timestamp, used for idle eviction.</summary>
    public DateTimeOffset LastAccessed { get; set; } = DateTimeOffset.UtcNow;

    /// <summary>Per-session incremental computation cache.</summary>
    internal ComputationCache Cache { get; } = new();

    /// <summary>Runs (in-flight and recent) keyed by run id.</summary>
    internal ConcurrentDictionary<string, RunState> Runs { get; } = new();

    public Session()
    {
        // A new session starts at the single "precision" knob default and exposes the DSP builtins.
        Engine.SetPrecision(DefaultPrecision);
        Engine.RegisterDspBuiltins();
        // Track (re)definitions so cached statements depending on a function are invalidated.
        Engine.FunctionDefined += (_, e) => Cache.NoteFunctionDefined(e.Definition.Name);
    }

    /// <summary>The session's precision (computation == display, the single knob).</summary>
    public long Precision => Engine.ComputationDecimalPlaces;
}
