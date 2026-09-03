using Lovelace.Suite;

namespace Lovelace.Studio;

/// <summary>Mutable progress/result store for a single in-flight or completed run. Polled by the front-end.</summary>
internal sealed class RunState
{
    private readonly object _sync = new();

    public string Id { get; } = Guid.NewGuid().ToString("N");
    public required string SessionId { get; init; }
    public required string Text { get; init; }
    public CancellationTokenSource? Cts;

    private int _status;      // 0 queued, 1 running, 2 finished, 3 error, 4 cancelled
    private int _total;
    private int _completed;
    private int _reused;
    private int _currentIndex = -1;
    private string? _currentLabel;
    private double? _subProgress;
    private string? _subLabel;
    private readonly List<RunStep> _steps = new();
    private StateSnapshot? _snapshot;
    private Value? _result;
    private readonly List<(string Message, int Position)> _diagnostics = new();
    private TimeSpan _elapsed;

    public string StatusText => _status switch
    {
        0 => "queued", 1 => "running", 2 => "finished", 3 => "error", 4 => "cancelled", _ => "unknown",
    };

    public void SetQueued(int total) { lock (_sync) { _status = 0; _total = total; } }

    public void SetRunning() { lock (_sync) { _status = 1; } }

    public void BeginStep(int index, string label)
    {
        lock (_sync) { _currentIndex = index; _currentLabel = label; _subProgress = null; _subLabel = null; }
    }

    public void SetSubProgress(string? label, double? fraction)
    {
        lock (_sync) { _subLabel = label; _subProgress = fraction; }
    }

    public void CompleteStep(RunStep step, StateSnapshot snapshot, Value? result, int reused, TimeSpan elapsed)
    {
        lock (_sync)
        {
            _steps.Add(step);
            _completed = _steps.Count;
            _reused = reused;
            _snapshot = snapshot;
            _result = result;
            _elapsed = elapsed;
        }
    }

    public void FinishError((string Message, int Position) diagnostic)
    {
        lock (_sync) { _status = 3; _diagnostics.Add(diagnostic); }
    }

    public void FinishCancelled() { lock (_sync) { _status = 4; } }

    public void Finish() { lock (_sync) { _status = 2; } }

    /// <summary>Returns an immutable snapshot of the current run state.</summary>
    public RunSnapshot Capture()
    {
        lock (_sync)
        {
            return new RunSnapshot(
                StatusText, _total, _completed, _reused, _currentIndex, _currentLabel,
                _subProgress, _subLabel, _steps.ToArray(), _snapshot, _result, _diagnostics.ToArray(), _elapsed);
        }
    }
}

/// <summary>Immutable snapshot of a run's progress and result.</summary>
internal sealed record RunSnapshot(
    string Status,
    int TotalStatements,
    int CompletedStatements,
    int ReusedCount,
    int CurrentIndex,
    string? CurrentLabel,
    double? SubProgress,
    string? SubLabel,
    RunStep[] Steps,
    StateSnapshot? Snapshot,
    Value? Result,
    (string Message, int Position)[] Diagnostics,
    TimeSpan Elapsed);
