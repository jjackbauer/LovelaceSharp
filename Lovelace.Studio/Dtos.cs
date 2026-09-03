using Lovelace.Suite;

namespace Lovelace.Studio;

/// <summary>Body for <c>POST /api/evaluate</c>.</summary>
public sealed record EvaluateRequest(string Source);

/// <summary>Body for <c>PUT /api/precision</c>.</summary>
public sealed record SetPrecisionRequest(long Digits);

/// <summary>Session metadata returned on create/resume.</summary>
public sealed record SessionResponse(string SessionId, long Precision, long Revision);

/// <summary>The value produced by the last statement, in three renderings.</summary>
public sealed record ValueResult(string Kind, string Display, string Typed);

/// <summary>A variable row for the workspace table.</summary>
public sealed record VariableRow(string Name, string Kind, string Display);

/// <summary>A function row for the workspace panel.</summary>
public sealed record FunctionRow(string Name, string[] Parameters, bool IsBuiltin, SourceSpan? Span);

/// <summary>An error diagnostic with its source position.</summary>
public sealed record DiagnosticRow(string Message, int Position, int Line, int Column);

/// <summary>An inline plot capture.</summary>
public sealed record PlotPayload(string Svg, string Title);

/// <summary>A single script operation: line number, source text, result value, print output, and elapsed time.</summary>
public sealed record TimingRow(int Line, string Text, string? Result, string? Output, string Elapsed, string Mode);

/// <summary>The full evaluate round-trip response.</summary>
public sealed record EvaluateResponse(
    ValueResult? Result,
    VariableRow[] Variables,
    FunctionRow[] Functions,
    string[] Logs,
    PlotPayload? Plot,
    DiagnosticRow[] Diagnostics,
    long Revision,
    string Elapsed,
    TimingRow[] Timings,
    int ReusedCount,
    long Precision);

/// <summary>The workspace snapshot response (variables + functions + revision + precision).</summary>
public sealed record StateResponse(long Revision, VariableRow[] Variables, FunctionRow[] Functions, long Precision);

/// <summary>A single autocomplete candidate.</summary>
public sealed record CompletionItem(string Label, string Kind, string Detail);

/// <summary>The autocomplete catalog response.</summary>
public sealed record CompletionResponse(CompletionItem[] Items);

/// <summary>Returned by <c>POST /api/evaluate</c> — a run id to poll, and the session id.</summary>
public sealed record StartRunResponse(string RunId, string SessionId);

/// <summary>Polled run status: progress fields plus the (partial or final) evaluate response.</summary>
public sealed record RunStatusResponse(
    string Status,
    int TotalStatements,
    int CompletedStatements,
    int ReusedCount,
    int CurrentIndex,
    string? CurrentLabel,
    double? SubProgress,
    string? SubLabel,
    EvaluateResponse Response);
