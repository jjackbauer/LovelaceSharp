using Lovelace.Suite;

namespace Lovelace.Studio;

/// <summary>Body for <c>POST /api/evaluate</c>.</summary>
public sealed record EvaluateRequest(string Source);

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

/// <summary>The full evaluate round-trip response.</summary>
public sealed record EvaluateResponse(
    ValueResult? Result,
    VariableRow[] Variables,
    FunctionRow[] Functions,
    string[] Logs,
    PlotPayload? Plot,
    DiagnosticRow[] Diagnostics,
    long Revision);

/// <summary>The workspace snapshot response (variables + functions + revision).</summary>
public sealed record StateResponse(long Revision, VariableRow[] Variables, FunctionRow[] Functions);
