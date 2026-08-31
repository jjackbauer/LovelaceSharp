using Lovelace.Suite;

namespace Lovelace.Studio;

/// <summary>
/// Pure projection of a <see cref="SuiteEngine"/> onto the HTTP DTOs. Holds no
/// language logic; it only renders the engine's state, logs, plots, and diagnostics.
/// </summary>
/// <remarks>
/// The editor is newline-separated while the engine's program grammar is
/// semicolon-separated (mirroring the REPL, which submits one line at a time).
/// <see cref="ToSemicolonStatements"/> rewrites top-level newlines to <c>;</c> so a
/// multi-line script evaluates as one program. The rewrite is length-preserving, so
/// the engine's <c>position</c> diagnostics still index into the original source; only
/// line/column are recomputed here (they depend on where the newlines were).
/// </remarks>
public sealed class EngineHost
{
    private readonly SuiteEngine _engine;
    private readonly SemaphoreSlim _gate = new(1, 1);

    public EngineHost(SuiteEngine engine) => _engine = engine;

    /// <summary>Evaluates <paramref name="source"/> and returns the full round-trip response.</summary>
    public async Task<EvaluateResponse> EvaluateAsync(string source)
    {
        await _gate.WaitAsync();
        try
        {
            // Normalize CRLF/CR to LF so positions map 1:1 to what the editor shows.
            string text = source.Replace("\r\n", "\n").Replace('\r', '\n');

            var logs = new StringWriter();
            _engine.ResetPlotCapture();

            Value? result = null;
            Exception? error = null;
            try
            {
                result = await _engine.EvaluateAsync(ScriptSource.ToSemicolonStatements(text), logs);
            }
            catch (Exception ex)
            {
                error = ex;
            }

            return BuildResponse(text, result, logs, error);
        }
        finally
        {
            _gate.Release();
        }
    }

    /// <summary>Returns the current variables/functions snapshot.</summary>
    public StateResponse GetState() => ToState(_engine.CaptureState());

    /// <summary>Clears all variables (functions remain).</summary>
    public StateResponse ClearVariables()
    {
        _engine.Clear();
        return GetState();
    }

    /// <summary>Removes one variable.</summary>
    public StateResponse DeleteVariable(string name)
    {
        _engine.RemoveVariable(name);
        return GetState();
    }

    private EvaluateResponse BuildResponse(string source, Value? result, StringWriter logs, Exception? error)
    {
        var snapshot = _engine.CaptureState();

        var variables = ToVariables(snapshot);
        var functions = ToFunctions(snapshot);

        var plot = _engine.LastPlot;
        var plotPayload = plot?.Svg is null
            ? null
            : new PlotPayload(plot.Svg, plot.Title ?? string.Empty);

        var diagnostics = (error is null ? Array.Empty<Diagnostic>() : _engine.Diagnostics.ToArray())
            .Select(d =>
            {
                var (line, column) = ComputeLineColumn(source, d.Position);
                return new DiagnosticRow(d.Message, d.Position, line, column);
            })
            .ToArray();

        var resultPayload = result is null
            ? null
            : new ValueResult(result.Kind.ToString(), ValueFormatter.Format(result), ValueFormatter.FormatTyped(result));

        return new EvaluateResponse(
            resultPayload,
            variables,
            functions,
            SplitLines(logs.ToString()),
            plotPayload,
            diagnostics,
            snapshot.Revision);
    }

    private static StateResponse ToState(StateSnapshot snapshot) =>
        new(snapshot.Revision, ToVariables(snapshot), ToFunctions(snapshot));

    private static VariableRow[] ToVariables(StateSnapshot snapshot) =>
        snapshot.Variables.Values
            .OrderBy(v => v.Name, StringComparer.Ordinal)
            .Select(v => new VariableRow(v.Name, v.Kind.ToString(), v.Display))
            .ToArray();

    private static FunctionRow[] ToFunctions(StateSnapshot snapshot) =>
        snapshot.Functions.Values
            .OrderBy(f => f.Name, StringComparer.Ordinal)
            .Select(f => new FunctionRow(f.Name, f.Parameters.ToArray(), f.IsBuiltin, f.Span))
            .ToArray();

    private static string[] SplitLines(string text)
    {
        string normalized = text.Replace("\r\n", "\n").Replace('\r', '\n');
        var lines = normalized.Split('\n');
        if (lines.Length > 0 && lines[^1].Length == 0)
            return lines[..^1];
        return lines;
    }

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
