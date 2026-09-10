using Lovelace.Suite;

namespace Lovelace.Studio;

/// <summary>
/// Pure projection of a per-session <see cref="SuiteEngine"/> onto the HTTP DTOs. Holds no
/// language logic; it renders the engine's state, logs, plots, and diagnostics, and drives the
/// incremental runner synchronously (tests) or as a pollable background run.
/// </summary>
public sealed class EngineHost
{
    private readonly SessionRegistry _sessions;
    private readonly IncrementalRunner _runner = new();

    public EngineHost(SessionRegistry sessions) => _sessions = sessions;

    /// <summary>Returns the session for <paramref name="sessionId"/>, or creates a new one when the id is null/unknown.</summary>
    public Session ResolveSession(string? sessionId)
    {
        if (!string.IsNullOrEmpty(sessionId) && _sessions.Get(sessionId) is { } existing)
            return existing;
        return _sessions.Create();
    }

    public Session? TryGetSession(string sessionId) => _sessions.Get(sessionId);

    public bool RemoveSession(string sessionId) => _sessions.Remove(sessionId);

    /// <summary>Synchronous evaluate (used by tests and non-UI callers): runs to completion and returns the full response.</summary>
    public async Task<EvaluateResponse> EvaluateAsync(Session session, string source)
    {
        await session.Gate.WaitAsync();
        try
        {
            var outcome = await _runner.RunAsync(session, source);
            return BuildResponse(session.Engine, outcome.Text, outcome.Snapshot, outcome.Result, outcome.Plot, outcome.Diagnostics, outcome.Steps, outcome.Elapsed, outcome.ReusedCount);
        }
        finally
        {
            session.Gate.Release();
        }
    }

    /// <summary>Starts a background run and returns its run id immediately. Throws if a run is already active.</summary>
    public StartRunResponse StartRun(Session session, string source)
    {
        foreach (var r in session.Runs.Values)
            if (r.StatusText is "queued" or "running")
                throw new InvalidOperationException("A run is already in progress for this session.");

        var state = new RunState
        {
            SessionId = session.Id,
            Text = source.Replace("\r\n", "\n").Replace('\r', '\n'),
        };
        state.Cts = new CancellationTokenSource();
        session.Runs[state.Id] = state;

        _ = Task.Run(async () =>
        {
            await session.Gate.WaitAsync();
            try
            {
                await _runner.RunAsync(session, source, state, state.Cts.Token);
            }
            catch (Exception ex)
            {
                state.FinishError((ex.Message, 0));
            }
            finally
            {
                session.Gate.Release();
            }
        });

        return new StartRunResponse(state.Id, session.Id);
    }

    /// <summary>Returns the current status/progress/result of a run, or <see langword="null"/> if unknown.</summary>
    public RunStatusResponse? GetRun(Session session, string runId)
    {
        if (!session.Runs.TryGetValue(runId, out var state))
            return null;

        var snap = state.Capture();
        var snapshot = snap.Snapshot ?? session.Engine.CaptureState();
        var response = BuildResponse(session.Engine, state.Text, snapshot, snap.Result, session.Engine.LastPlot, snap.Diagnostics, snap.Steps, snap.Elapsed, snap.ReusedCount);
        return new RunStatusResponse(
            snap.Status, snap.TotalStatements, snap.CompletedStatements, snap.ReusedCount,
            snap.CurrentIndex, snap.CurrentLabel, snap.SubProgress, snap.SubLabel, response);
    }

    /// <summary>Requests cancellation of an in-flight run. Returns <see langword="false"/> if the run is unknown.</summary>
    public bool CancelRun(Session session, string runId)
    {
        if (!session.Runs.TryGetValue(runId, out var state))
            return false;
        state.Cts?.Cancel();
        return true;
    }

    /// <summary>
    /// Symbolic inspection surface: evaluates a single statement, then projects the resulting
    /// symbolic value onto its canonical/pretty forms, expression tree, assumptions, the
    /// simplification trace, and the lowered MathIR.
    /// </summary>
    public async Task<SymbolicInspectResponse> InspectSymbolicAsync(Session session, string source)
    {
        await session.Gate.WaitAsync();
        try
        {
            var result = await session.Engine.EvaluateAsync(source);
            if (result.Kind != ValueKind.Symbolic)
                return new SymbolicInspectResponse(null, null, null, Array.Empty<string>(), Array.Empty<string>(), null,
                    new[] { $"The last value is {result.Kind}, not a symbolic expression." },
                    StructuredProjection.ToStructured(result));

            var expr = result.AsSymbolic();
            var ctx = session.Symbolics.Context;
            var trace = Lovelace.Symbolics.Simplify.Transform(expr, ctx, new Lovelace.Symbolics.Simplify.Options(Trace: true));

            string? mathir = null;
            try
            {
                var symbols = CollectSymbols(expr).Select(ctx.Symbol).ToArray();
                var program = Lovelace.MathIR.Lowering.Lower(expr, ctx, symbols);
                mathir = program.Serialize().TrimEnd('\n');
            }
            catch (Exception)
            {
                // not lowerable (e.g. an unevaluated integral): leave the MathIR view empty
            }

            return new SymbolicInspectResponse(
                Lovelace.Symbolics.Printing.CanonicalPrint(expr),
                Lovelace.Symbolics.Printing.PrettyPrint(expr),
                BuildTree(expr),
                session.Symbolics.Context.Assumptions.Atoms.Select(a => a.ToString()!).ToArray(),
                trace.Steps.Select(s => s.ToString()).ToArray(),
                mathir,
                Array.Empty<string>(),
                StructuredProjection.ToStructured(result));
        }
        catch (Exception ex)
        {
            return new SymbolicInspectResponse(null, null, null, Array.Empty<string>(), Array.Empty<string>(), null,
                new[] { ex.Message }, null);
        }
        finally
        {
            session.Gate.Release();
        }
    }

    private static SymbolicTreeNode BuildTree(Lovelace.Symbolics.Expr e)
    {
        var children = new List<SymbolicTreeNode>();
        switch (e)
        {
            case Lovelace.Symbolics.AddExpr a: children.AddRange(a.Terms.Select(BuildTree)); break;
            case Lovelace.Symbolics.MultiplyExpr m: children.AddRange(m.Factors.Select(BuildTree)); break;
            case Lovelace.Symbolics.PowerExpr p: children.Add(BuildTree(p.Base)); children.Add(BuildTree(p.Exponent)); break;
            case Lovelace.Symbolics.FunctionExpr f: children.AddRange(f.Arguments.Select(BuildTree)); break;
            case Lovelace.Symbolics.RelationExpr r: children.Add(BuildTree(r.Left)); children.Add(BuildTree(r.Right)); break;
            case Lovelace.Symbolics.PiecewiseExpr pw:
                children.AddRange(pw.Branches.Select(b => new SymbolicTreeNode("branch", "if " + Lovelace.Symbolics.Printing.PrettyPrint(b.Guard), new[] { BuildTree(b.Value) })));
                children.Add(BuildTree(pw.Otherwise));
                break;
        }
        return new SymbolicTreeNode(e.Kind.ToString(), Lovelace.Symbolics.Printing.PrettyPrint(e), children.ToArray());
    }

    private static SortedSet<string> CollectSymbols(Lovelace.Symbolics.Expr e)
    {
        var set = new SortedSet<string>(StringComparer.Ordinal);
        void Walk(Lovelace.Symbolics.Expr x)
        {
            switch (x)
            {
                case Lovelace.Symbolics.SymbolExpr s: set.Add(s.Symbol.Name); break;
                case Lovelace.Symbolics.AddExpr a: foreach (var t in a.Terms) Walk(t); break;
                case Lovelace.Symbolics.MultiplyExpr m: foreach (var f in m.Factors) Walk(f); break;
                case Lovelace.Symbolics.PowerExpr p: Walk(p.Base); Walk(p.Exponent); break;
                case Lovelace.Symbolics.FunctionExpr f: foreach (var arg in f.Arguments) Walk(arg); break;
                case Lovelace.Symbolics.RelationExpr r: Walk(r.Left); Walk(r.Right); break;
                case Lovelace.Symbolics.PiecewiseExpr pw:
                    foreach (var b in pw.Branches) { Walk(b.Guard); Walk(b.Value); }
                    Walk(pw.Otherwise);
                    break;
            }
        }
        Walk(e);
        return set;
    }

    private static readonly string[] Keywords = ["func", "if", "else", "while", "for", "in", "return", "break", "continue"];

    /// <summary>Returns the autocomplete catalog: keywords, built-ins, user functions, and live variables.</summary>
    public CompletionResponse GetCompletions(Session session)
    {
        var items = new List<CompletionItem>();

        foreach (var kw in Keywords)
            items.Add(new CompletionItem(kw, "keyword", kw));

        foreach (var f in session.Engine.Functions.Values.Where(f => f.IsBuiltin).OrderBy(f => f.Name, StringComparer.Ordinal))
            items.Add(new CompletionItem(f.Name, "builtin", f.Name + "(" + string.Join(", ", f.Parameters) + ")"));

        foreach (var f in session.Engine.Functions.Values.Where(f => !f.IsBuiltin).OrderBy(f => f.Name, StringComparer.Ordinal))
            items.Add(new CompletionItem(f.Name, "function", f.Name + "(" + string.Join(", ", f.Parameters) + ")"));

        foreach (var v in session.Engine.Variables.Keys.OrderBy(n => n, StringComparer.Ordinal))
        {
            string kind = session.Engine.TryGetVariable(v, out var val) ? val.Kind.ToString() : "unknown";
            items.Add(new CompletionItem(v, "variable", kind));
        }

        return new CompletionResponse(items.ToArray());
    }

    public StateResponse GetState(Session session) => ToState(session);

    public StateResponse ClearVariables(Session session)
    {
        session.Engine.Clear();
        session.Cache.Invalidate();
        return ToState(session);
    }

    public StateResponse DeleteVariable(Session session, string name)
    {
        session.Engine.RemoveVariable(name);
        session.Cache.Invalidate();
        return ToState(session);
    }

    public StateResponse SetPrecision(Session session, long digits)
    {
        if (digits <= 0)
            throw new InvalidOperationException($"precision must be a positive integer, but got {digits}.");
        session.Engine.SetPrecision(digits);
        session.Cache.Invalidate();
        return ToState(session);
    }

    /// <summary>
    /// Sets the engine's value-rendering mode (<c>engine.UnicodeOutput</c>). Values are unchanged,
    /// so the computation cache stays valid; only the rendering of every value does.
    /// </summary>
    public StateResponse SetFormat(Session session, bool unicode)
    {
        session.UnicodeOutput = unicode;
        return ToState(session);
    }

    private static EvaluateResponse BuildResponse(
        SuiteEngine engine,
        string text,
        StateSnapshot snapshot,
        Value? result,
        PlotCapture? plot,
        IReadOnlyList<(string Message, int Position)> diagnostics,
        IReadOnlyList<RunStep> steps,
        TimeSpan elapsed,
        int reused)
    {
        var variables = ToVariables(engine, snapshot);
        var functions = ToFunctions(snapshot);

        var plotPayload = plot?.Svg is null
            ? null
            : new PlotPayload(plot.Svg, plot.Title ?? string.Empty);

        var sourceLines = text.Split('\n');
        var diagnosticRows = diagnostics
            .Select(d =>
            {
                var (line, column) = ComputeLineColumn(text, d.Position);
                return new DiagnosticRow(d.Message, d.Position, line, column);
            })
            .ToArray();

        var logs = steps.SelectMany(s => SplitLines(s.Output)).ToArray();

        var resultPayload = result is null
            ? null
            : new ValueResult(
                result.Kind.ToString(),
                engine.FormatValue(result),
                engine.FormatValueTyped(result),
                StructuredProjection.ToStructured(result),
                TypeNameOf(result));

        var timings = steps
            .Select(s =>
            {
                int line = ComputeLineColumn(text, s.Position).Line;
                string src = line >= 1 && line <= sourceLines.Length ? sourceLines[line - 1].Trim() : string.Empty;
                bool hasValue = s.Result is not null && s.Result.Kind != ValueKind.Void;
                string? r = hasValue ? engine.FormatValue(s.Result!) : null;
                StructuredValueDto? structured = hasValue ? StructuredProjection.ToStructured(s.Result!) : null;
                string? output = s.Output.Length == 0 ? null : s.Output.TrimEnd('\r', '\n');
                return new TimingRow(line, src, r, output, Timing.Format(s.Elapsed), s.Mode, structured);
            })
            .ToArray();

        return new EvaluateResponse(
            resultPayload, variables, functions, logs, plotPayload, diagnosticRows,
            snapshot.Revision, Timing.Format(elapsed), timings, reused, engine.ComputationDecimalPlaces);
    }

    private static StateResponse ToState(Session session)
    {
        var snapshot = session.Engine.CaptureState();
        return new StateResponse(snapshot.Revision, ToVariables(session.Engine, snapshot), ToFunctions(snapshot),
            session.Precision, session.UnicodeOutput);
    }

    private static List<string> SplitLines(string text)
    {
        string normalized = text.Replace("\r\n", "\n").Replace('\r', '\n');
        var lines = normalized.Split('\n');
        if (lines.Length > 0 && lines[^1].Length == 0)
            return lines[..^1].ToList();
        return lines.ToList();
    }

    private static VariableRow[] ToVariables(SuiteEngine engine, StateSnapshot snapshot) =>
        snapshot.Variables.Values
            .OrderBy(v => v.Name, StringComparer.Ordinal)
            .Select(v =>
            {
                // The snapshot only carries rendered text; the live value supplies the one shared
                // structured projection, and the display is re-rendered under the session's format.
                if (engine.TryGetVariable(v.Name, out var value))
                    return new VariableRow(v.Name, v.Kind.ToString(), engine.FormatValue(value),
                        StructuredProjection.ToStructured(value), TypeNameOf(value));
                return new VariableRow(v.Name, v.Kind.ToString(), v.Display, null, null);
            })
            .ToArray();

    /// <summary>The record type name a value carries (e.g. <c>"SolveResult"</c>), or null for every other kind.</summary>
    private static string? TypeNameOf(Value value) =>
        value.Kind == ValueKind.Record ? value.AsRecord().TypeName : null;

    private static FunctionRow[] ToFunctions(StateSnapshot snapshot) =>
        snapshot.Functions.Values
            .OrderBy(f => f.Name, StringComparer.Ordinal)
            .Select(f => new FunctionRow(f.Name, f.Parameters.ToArray(), f.IsBuiltin, f.Span, f.Plugin))
            .ToArray();

    private static (int Line, int Column) ComputeLineColumn(string source, int position)
    {
        if (position < 0 || position > source.Length)
            return (1, position + 1);

        int line = 1;
        int lastNewline = -1;
        for (int i = 0; i < position && i < source.Length; i++)
        {
            if (source[i] == '\n')
            {
                line++;
                lastNewline = i;
            }
        }

        return (line, position - lastNewline);
    }
}
