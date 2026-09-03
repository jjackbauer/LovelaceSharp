using System.Diagnostics;
using System.Security.Cryptography;
using System.Text;
using System.Text.RegularExpressions;
using Lovelace.Suite;

namespace Lovelace.Studio;

/// <summary>One executed (or reused) top-level statement's outcome.</summary>
internal sealed record RunStep(int Position, string Mode, Value? Result, string Output, TimeSpan Elapsed);

/// <summary>The full outcome of an incremental script run.</summary>
internal sealed class RunOutcome
{
    public required string Text { get; init; }
    public required Value? Result { get; init; }
    public required StateSnapshot Snapshot { get; init; }
    public required List<string> Logs { get; init; }
    public required PlotCapture? Plot { get; init; }
    public required List<(string Message, int Position)> Diagnostics { get; init; }
    public required List<RunStep> Steps { get; init; }
    public required TimeSpan Elapsed { get; init; }
    public required int ReusedCount { get; init; }
}

/// <summary>A cached statement outcome, keyed by its content+input signature.</summary>
internal sealed record CachedStep(Value? Result, string Output, TimeSpan Elapsed);

/// <summary>Per-session computation cache: cached statement outcomes + per-function revision counters.</summary>
internal sealed class ComputationCache
{
    private readonly Dictionary<string, int> _functionRevisions = new();
    private readonly Dictionary<string, CachedStep> _entries = new();
    private int _nextRevision;

    public void NoteFunctionDefined(string name) => _functionRevisions[name] = ++_nextRevision;

    public int FunctionRevision(string name) => _functionRevisions.TryGetValue(name, out var r) ? r : 0;

    public bool TryGet(string key, out CachedStep? step) => _entries.TryGetValue(key, out step);

    public void Put(string key, CachedStep step) => _entries[key] = step;

    public void Invalidate()
    {
        _entries.Clear();
        _functionRevisions.Clear();
    }
}

/// <summary>Collects the variables and functions a statement/expression syntactically reads.</summary>
internal static class ReadSetCollector
{
    private static readonly HashSet<string> SideEffectFunctions = new(StringComparer.Ordinal)
    {
        "print", "plot", "setprecision",
    };

    public static bool HasSideEffects(IEnumerable<string> funcs) => funcs.Any(SideEffectFunctions.Contains);

    public static void CollectStatement(Statement s, HashSet<string> vars, HashSet<string> funcs, HashSet<string> locals)
    {
        switch (s)
        {
            case ExpressionStatement es:
                CollectExpr(es.Expression, vars, funcs, locals);
                break;
            case BlockStatement b:
                foreach (var x in b.Statements) CollectStatement(x, vars, funcs, locals);
                break;
            case IfStatement i:
                CollectExpr(i.Condition, vars, funcs, locals);
                CollectStatement(i.Then, vars, funcs, locals);
                if (i.Else is not null) CollectStatement(i.Else, vars, funcs, locals);
                break;
            case WhileStatement w:
                CollectExpr(w.Condition, vars, funcs, locals);
                CollectStatement(w.Body, vars, funcs, locals);
                break;
            case ForStatement f:
                CollectExpr(f.Range, vars, funcs, locals);
                var inner = new HashSet<string>(locals) { f.Variable };
                CollectStatement(f.Body, vars, funcs, inner);
                break;
            case ReturnStatement r:
                if (r.Value is not null) CollectExpr(r.Value, vars, funcs, locals);
                break;
            case FunctionStatement:
            case BreakStatement:
            case ContinueStatement:
                break;
        }
    }

    public static void CollectExpr(Expr e, HashSet<string> vars, HashSet<string> funcs, HashSet<string> locals)
    {
        switch (e)
        {
            case LiteralExpr:
            case StringExpr:
                break;
            case VariableExpr v:
                if (!locals.Contains(v.Name)) vars.Add(v.Name);
                break;
            case AssignExpr a:
                CollectExpr(a.Value, vars, funcs, locals);
                break;
            case BinaryExpr b:
                CollectExpr(b.Left, vars, funcs, locals);
                CollectExpr(b.Right, vars, funcs, locals);
                break;
            case UnaryExpr u:
                CollectExpr(u.Operand, vars, funcs, locals);
                break;
            case PostfixExpr p:
                CollectExpr(p.Operand, vars, funcs, locals);
                break;
            case CallExpr c:
                funcs.Add(c.FunctionName);
                foreach (var a in c.Arguments) CollectExpr(a, vars, funcs, locals);
                break;
            case RangeExpr r:
                CollectExpr(r.Start, vars, funcs, locals);
                if (r.Step is not null) CollectExpr(r.Step, vars, funcs, locals);
                CollectExpr(r.End, vars, funcs, locals);
                break;
            case IndexExpr i:
                CollectExpr(i.Target, vars, funcs, locals);
                foreach (var x in i.Indices) CollectExpr(x, vars, funcs, locals);
                break;
            case ListExpr l:
                foreach (var x in l.Elements) CollectExpr(x, vars, funcs, locals);
                break;
            case InterpolatedStringExpr s:
                foreach (var p in s.Parts)
                    if (p is ExpressionPart ep) CollectExpr(ep.Expression, vars, funcs, locals);
                break;
        }
    }
}

/// <summary>Canonical, full-precision hashing of a value (not display-truncated).</summary>
internal static class ValueHasher
{
    public static string Hash(Value v) => Sha256(Canonical(v));

    private static string Canonical(Value v) => v.Kind switch
    {
        ValueKind.Natural => "N:" + v.AsNatural().ToString(),
        ValueKind.Integer => "I:" + v.AsInteger().ToString(),
        ValueKind.Real => "R:" + v.AsReal().ToNatural().ToString() + ":" + v.AsReal().Exponent + ":" + v.AsReal().PeriodStart + ":" + v.AsReal().PeriodLength,
        ValueKind.Boolean => "B:" + (v.AsBoolean() ? "1" : "0"),
        ValueKind.Text => "T:" + v.AsText(),
        ValueKind.Vector => "V[" + string.Join(",", v.AsVector().Select(Canonical)) + "]",
        ValueKind.Array => "A(" + string.Join("x", v.AsArray().Shape) + ")[" + string.Join(",", v.AsArray().Data.Select(Canonical)) + "]",
        ValueKind.Function => "F:" + v.AsFunction().Name,
        ValueKind.Void => "Void",
        _ => v.ToString(),
    };

    private static string Sha256(string input) => Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(input)));
}

/// <summary>
/// Splits a script into top-level statements and executes them incrementally: statements whose
/// content and inputs are unchanged are reused from the per-session cache; the rest recompute.
/// Optionally reports progress into a <see cref="RunState"/> and honours a cancellation token.
/// </summary>
internal sealed class IncrementalRunner
{
    public async Task<RunOutcome> RunAsync(Session session, string source, RunState? state = null, CancellationToken ct = default)
    {
        string text = source.Replace("\r\n", "\n").Replace('\r', '\n');
        string engineSource = ScriptSource.ToSemicolonStatements(text);

        var steps = new List<RunStep>();
        var logs = new List<string>();
        var diagnostics = new List<(string Message, int Position)>();
        Value? finalResult = null;
        int reused = 0;
        bool failed = false;

        var stopwatch = Stopwatch.StartNew();
        session.Engine.ResetPlotCapture();

        Lovelace.Suite.Program program;
        try
        {
            program = session.Engine.Parse(engineSource);
        }
        catch (Exception ex)
        {
            stopwatch.Stop();
            return ErrorOutcome(session, text, ex, PositionFrom(ex));
        }

        var slices = SplitSlices(engineSource, program);
        var functions = session.Engine.Functions;

        state?.SetQueued(slices.Count);
        state?.SetRunning();

        session.Engine.ProgressReporter = state is null
            ? null
            : new SyncProgress<OperationProgress>(p => state.SetSubProgress(p.Label, p.Fraction));

        for (int i = 0; i < slices.Count; i++)
        {
            if (ct.IsCancellationRequested)
            {
                failed = true;
                state?.FinishCancelled();
                break;
            }

            var slice = slices[i];
            state?.BeginStep(i, slice.Source.Trim());

            if (IsMemoizable(slice.Statement, out string? writeName))
            {
                var vars = new HashSet<string>(StringComparer.Ordinal);
                var funcs = new HashSet<string>(StringComparer.Ordinal);
                CollectReadSet(slice.Statement, functions, vars, funcs);

                if (!ReadSetCollector.HasSideEffects(funcs))
                {
                    string key = BuildSignature(slice, session, vars, funcs);
                    if (session.Cache.TryGet(key, out var cached) && cached is not null)
                    {
                        if (writeName is not null && cached.Result is not null)
                            session.Engine.SetVariable(writeName, cached.Result);

                        finalResult = cached.Result;
                        reused++;
                        if (cached.Output.Length > 0) logs.AddRange(SplitLines(cached.Output));
                        var reuseStep = new RunStep(slice.Position, "reuse", cached.Result, cached.Output, cached.Elapsed);
                        steps.Add(reuseStep);
                        state?.CompleteStep(reuseStep, session.Engine.CaptureState(), finalResult, reused, stopwatch.Elapsed);
                        continue;
                    }

                    var (step, result, error) = await ComputeAsync(session, slice);
                    steps.Add(step);
                    if (result is not null) finalResult = result;
                    if (step.Output.Length > 0) logs.AddRange(SplitLines(step.Output));
                    state?.CompleteStep(step, session.Engine.CaptureState(), finalResult, reused, stopwatch.Elapsed);
                    if (error is not null)
                    {
                        failed = true;
                        diagnostics.Add((error.Message, slice.Position + PositionFrom(error)));
                        state?.FinishError(diagnostics[^1]);
                        break;
                    }
                    session.Cache.Put(key, new CachedStep(result, step.Output, step.Elapsed));
                    continue;
                }
            }

            var (st, res, err) = await ComputeAsync(session, slice);
            steps.Add(st);
            if (res is not null) finalResult = res;
            if (st.Output.Length > 0) logs.AddRange(SplitLines(st.Output));
            state?.CompleteStep(st, session.Engine.CaptureState(), finalResult, reused, stopwatch.Elapsed);
            if (err is not null)
            {
                failed = true;
                diagnostics.Add((err.Message, slice.Position + PositionFrom(err)));
                state?.FinishError(diagnostics[^1]);
                break;
            }
        }

        stopwatch.Stop();

        if (!failed)
        {
            if (finalResult is not null && finalResult.Kind != ValueKind.Void)
                session.Engine.SetVariable("_", finalResult);
            state?.Finish();
        }

        return new RunOutcome
        {
            Text = text,
            Result = finalResult,
            Snapshot = session.Engine.CaptureState(),
            Logs = logs,
            Plot = session.Engine.LastPlot,
            Diagnostics = diagnostics,
            Steps = steps,
            Elapsed = stopwatch.Elapsed,
            ReusedCount = reused,
        };
    }

    private static List<(int Position, string Source, Statement Statement)> SplitSlices(string engineSource, Lovelace.Suite.Program program)
    {
        var slices = new List<(int, string, Statement)>();
        int n = program.Statements.Count;
        for (int i = 0; i < n; i++)
        {
            int start = program.StatementPositions[i];
            int end = i + 1 < n ? program.StatementPositions[i + 1] : engineSource.Length;
            if (end < start) end = start;
            string slice = engineSource.Substring(start, end - start);
            slices.Add((start, slice, program.Statements[i]));
        }
        return slices;
    }

    private static bool IsMemoizable(Statement statement, out string? writeName)
    {
        if (statement is ExpressionStatement { Expression: AssignExpr assign })
        {
            writeName = assign.Name;
            return true;
        }
        if (statement is ExpressionStatement)
        {
            writeName = null;
            return true;
        }
        writeName = null;
        return false;
    }

    private static void CollectReadSet(Statement statement, IReadOnlyDictionary<string, FunctionDefinition> functions, HashSet<string> vars, HashSet<string> funcs)
    {
        ReadSetCollector.CollectStatement(statement, vars, funcs, new HashSet<string>());

        var queue = new Queue<string>(funcs);
        var visited = new HashSet<string>(StringComparer.Ordinal);
        while (queue.Count > 0)
        {
            string fn = queue.Dequeue();
            if (!visited.Add(fn)) continue;
            if (!functions.TryGetValue(fn, out var def) || def.IsBuiltin) continue;

            var bodyVars = new HashSet<string>(StringComparer.Ordinal);
            var bodyFuncs = new HashSet<string>(StringComparer.Ordinal);
            var locals = new HashSet<string>(def.Parameters, StringComparer.Ordinal);
            foreach (var s in def.Body)
                ReadSetCollector.CollectStatement(s, bodyVars, bodyFuncs, locals);

            foreach (var v in bodyVars) vars.Add(v);
            foreach (var f in bodyFuncs)
            {
                funcs.Add(f);
                queue.Enqueue(f);
            }
        }
    }

    private static string BuildSignature((int Position, string Source, Statement Statement) slice, Session session, HashSet<string> vars, HashSet<string> funcs)
    {
        var sb = new StringBuilder();
        sb.Append(Sha256(slice.Source.Trim())).Append('|').Append(session.Engine.ComputationDecimalPlaces);

        foreach (var v in vars.OrderBy(x => x, StringComparer.Ordinal))
        {
            string h = session.Engine.TryGetVariable(v, out var val) ? ValueHasher.Hash(val) : "UNDEF";
            sb.Append('|').Append(v).Append('=').Append(h);
        }

        foreach (var f in funcs.OrderBy(x => x, StringComparer.Ordinal))
        {
            bool builtin = !session.Engine.Functions.TryGetValue(f, out var def) || def.IsBuiltin;
            sb.Append('|').Append(f).Append('=').Append(builtin ? "builtin" : session.Cache.FunctionRevision(f).ToString());
        }

        return Sha256(sb.ToString());
    }

    private async Task<(RunStep Step, Value? Result, Exception? Error)> ComputeAsync(Session session, (int Position, string Source, Statement Statement) slice)
    {
        var sw = Stopwatch.StartNew();
        var capture = new StringWriter();
        Value? result = null;
        try
        {
            result = await session.Engine.EvaluateAsync(slice.Source, capture);
            sw.Stop();
            return (new RunStep(slice.Position, "compute", result, capture.ToString(), sw.Elapsed), result, null);
        }
        catch (Exception ex)
        {
            sw.Stop();
            return (new RunStep(slice.Position, "compute", null, capture.ToString(), sw.Elapsed), null, ex);
        }
    }

    private static RunOutcome ErrorOutcome(Session session, string text, Exception ex, int position) =>
        new()
        {
            Text = text,
            Result = null,
            Snapshot = session.Engine.CaptureState(),
            Logs = new List<string>(),
            Plot = session.Engine.LastPlot,
            Diagnostics = new List<(string, int)> { (ex.Message, position) },
            Steps = new List<RunStep>(),
            Elapsed = TimeSpan.Zero,
            ReusedCount = 0,
        };

    private static int PositionFrom(Exception ex)
    {
        var m = Regex.Match(ex.Message, @"at position (\d+)", RegexOptions.IgnoreCase);
        return m.Success && int.TryParse(m.Groups[1].Value, out int p) ? p : 0;
    }

    private static string Sha256(string input) => Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(input)));

    private static List<string> SplitLines(string text)
    {
        string normalized = text.Replace("\r\n", "\n").Replace('\r', '\n');
        var lines = normalized.Split('\n');
        if (lines.Length > 0 && lines[^1].Length == 0)
            return lines[..^1].ToList();
        return lines.ToList();
    }
}
