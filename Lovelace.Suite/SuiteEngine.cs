using System.Diagnostics;
using System.Text.RegularExpressions;
using Rl = global::Lovelace.Real.Real;
using Lovelace.Abstractions;

namespace Lovelace.Suite;

/// <summary>
/// The public façade of the Lovelace suite: a string-level entry point over the
/// <see cref="Interpreter"/>, exposing the introspection interface (variables,
/// functions, snapshots, events) and diagnostics with source positions.
/// </summary>
public sealed class SuiteEngine
{
    private readonly Interpreter _interpreter = new();
    private readonly Tokenizer _tokenizer = new();
    private readonly Parser _parser = new();
    private readonly List<Diagnostic> _diagnostics = [];
    private readonly ModusHost _modus;
    private string _lastSource = string.Empty;

    public SuiteEngine()
    {
        _modus = new ModusHost(_interpreter);
    }

    // -----------------------------------------------------------------
    // Host settings (delegated to the interpreter)
    // -----------------------------------------------------------------

    /// <summary>Where <c>print</c> writes.</summary>
    public TextWriter Output
    {
        get => _interpreter.Output;
        set => _interpreter.Output = value;
    }

    /// <summary>Directory into which <c>plot</c> writes its SVG file.</summary>
    public string PlotOutputDirectory
    {
        get => _interpreter.PlotOutputDirectory;
        set => _interpreter.PlotOutputDirectory = value;
    }

    /// <summary>File name used by <c>plot</c>.</summary>
    public string PlotFileName
    {
        get => _interpreter.PlotFileName;
        set => _interpreter.PlotFileName = value;
    }

    /// <summary>The SVG and title of the most recently rendered plot, if any.</summary>
    public PlotCapture? LastPlot => _interpreter.LastPlot;

    /// <summary>Clears the last-plot capture (used by hosts to detect plots per run).</summary>
    public void ResetPlotCapture() => _interpreter.ResetPlotCapture();

    /// <summary>Computation precision (Real decimal places) for this engine.</summary>
    public long ComputationDecimalPlaces
    {
        get => _interpreter.ComputationDecimalPlaces;
        set => _interpreter.ComputationDecimalPlaces = value;
    }

    /// <summary>Display precision (Real fractional digits shown) for this engine.</summary>
    public long DisplayDecimalPlaces
    {
        get => _interpreter.DisplayDecimalPlaces;
        set => _interpreter.DisplayDecimalPlaces = value;
    }

    /// <summary>Sets both computation and display precision (the single precision knob).</summary>
    public void SetPrecision(long decimalPlaces) => _interpreter.SetPrecision(decimalPlaces);

    /// <summary>Optional sink for sub-operation progress, forwarded to the interpreter.</summary>
    public IProgress<OperationProgress>? ProgressReporter
    {
        get => _interpreter.ProgressReporter;
        set => _interpreter.ProgressReporter = value;
    }

    /// <summary>Formats a value at this engine's display precision.</summary>
    public string FormatValue(Value value)
    {
        using var _ = Rl.WithPrecision(ComputationDecimalPlaces, DisplayDecimalPlaces);
        return ValueFormatter.Format(value);
    }

    /// <summary>Formats a value with a type suffix at this engine's display precision.</summary>
    public string FormatValueTyped(Value value)
    {
        using var _ = Rl.WithPrecision(ComputationDecimalPlaces, DisplayDecimalPlaces);
        return ValueFormatter.FormatTyped(value);
    }

    /// <summary>Elapsed wall-clock time of the most recent evaluation.</summary>
    public TimeSpan LastElapsed { get; private set; }

    /// <summary><see cref="LastElapsed"/> rendered with an auto-scaled unit (ns/µs/ms/…).</summary>
    public string LastElapsedDisplay => Timing.Format(LastElapsed);

    /// <summary>Per-statement elapsed times from the most recent evaluation, in statement order.</summary>
    public IReadOnlyList<OperationTiming> OperationTimings => _interpreter.OperationTimings;

    /// <summary>Monotonic revision counter bumped on every state mutation.</summary>
    public long Revision => _interpreter.Revision;

    // -----------------------------------------------------------------
    // Introspection
    // -----------------------------------------------------------------

    /// <summary>Live view of all global variables (name → value + kind).</summary>
    public IReadOnlyDictionary<string, Value> Variables => _interpreter.Variables;

    /// <summary>Live view of all functions (name → definition).</summary>
    public IReadOnlyDictionary<string, FunctionDefinition> Functions => _interpreter.Functions;

    /// <summary>Diagnostics from the most recent failed operation.</summary>
    public IReadOnlyList<Diagnostic> Diagnostics => _diagnostics;

    /// <summary>Raised when a global variable is defined, reassigned, or removed.</summary>
    public event EventHandler<VariableChangedEventArgs>? VariableChanged
    {
        add => _interpreter.VariableChanged += value;
        remove => _interpreter.VariableChanged -= value;
    }

    /// <summary>Raised when a function is defined.</summary>
    public event EventHandler<FunctionDefinedEventArgs>? FunctionDefined
    {
        add => _interpreter.FunctionDefined += value;
        remove => _interpreter.FunctionDefined -= value;
    }

    // -----------------------------------------------------------------
    // Parsing / evaluation
    // -----------------------------------------------------------------

    /// <summary>Tokenizes and parses <paramref name="source"/> into a program of statements.</summary>
    public Program Parse(string source)
    {
        _lastSource = source;
        var tokens = _tokenizer.Tokenize(source);
        return _parser.ParseProgram(tokens);
    }

    /// <summary>Tokenizes and parses <paramref name="source"/> into a single expression.</summary>
    public Expr ParseExpression(string source)
    {
        _lastSource = source;
        var tokens = _tokenizer.Tokenize(source);
        return _parser.Parse(tokens);
    }

    /// <summary>
    /// Evaluates <paramref name="source"/> as a script/expression. On success the
    /// result (unless <c>void</c>) is stored in the <c>_</c> variable.
    /// </summary>
    /// <remarks>
    /// When <paramref name="output"/> is supplied, <c>print</c> output is captured
    /// to that writer for the duration of this call and the interpreter's previous
    /// <see cref="Output"/> is restored afterward.
    /// </remarks>
    public async Task<Value> EvaluateAsync(string source, TextWriter? output = null)
    {
        _diagnostics.Clear();
        _interpreter.ClearOperationTimings();
        _lastSource = source;

        var stopwatch = Stopwatch.StartNew();
        try
        {
            if (output is null)
                return await EvaluateCoreAsync(source);

            var previous = _interpreter.Output;
            _interpreter.Output = output;
            try
            {
                return await EvaluateCoreAsync(source);
            }
            finally
            {
                _interpreter.Output = previous;
            }
        }
        finally
        {
            stopwatch.Stop();
            LastElapsed = stopwatch.Elapsed;
        }
    }

    private async Task<Value> EvaluateCoreAsync(string source)
    {
        try
        {
            var tokens = _tokenizer.Tokenize(source);
            var program = _parser.ParseProgram(tokens);
            var result = await _interpreter.ExecuteAsync(program);

            if (result.Kind != ValueKind.Void)
                _interpreter.SetVariable("_", result);

            return result;
        }
        catch (Exception ex)
        {
            _diagnostics.Add(ToDiagnostic(ex));
            throw;
        }
    }

    /// <summary>Synchronous convenience wrapper over <see cref="EvaluateAsync"/>.</summary>
    public Value Evaluate(string source) => EvaluateAsync(source).GetAwaiter().GetResult();

    // -----------------------------------------------------------------
    // State mutation
    // -----------------------------------------------------------------

    /// <summary>Defines or overwrites a global variable.</summary>
    public void SetVariable(string name, Value value) => _interpreter.SetVariable(name, value);

    /// <summary>Looks up a global variable without throwing.</summary>
    public bool TryGetVariable(string name, out Value value) => _interpreter.Variables.TryGetValue(name, out value!);

    /// <summary>Removes a global variable.</summary>
    public bool RemoveVariable(string name) => _interpreter.Remove(name);

    /// <summary>Clears all global variables.</summary>
    public void Clear() => _interpreter.Clear();

    /// <summary>Registers a user or built-in function definition.</summary>
    public void DefineFunction(FunctionDefinition definition) => _interpreter.DefineFunction(definition);

    /// <summary>Registers a host-provided native function.</summary>
    public void RegisterBuiltin(string name, IReadOnlyList<string> parameters, Func<IReadOnlyList<Value>, Value> implementation) =>
        _interpreter.RegisterBuiltin(name, parameters, implementation);

    /// <summary>Loads a Modus plugin, registering its builtins and kernels.</summary>
    public void LoadPlugin(IModusPlugin plugin) => _modus.Load(plugin);

    /// <summary>Fallible kernel dispatch; returns false when no plugin kernel handles the request.</summary>
    public bool TryDispatchKernel<T>(ArrayOp op, ReadOnlySpan<T> left, ReadOnlySpan<T> right, Span<T> result)
        where T : unmanaged => _modus.TryDispatch(op, left, right, result);

    /// <summary>Captures an immutable snapshot of variables and functions.</summary>
    public StateSnapshot CaptureState()
    {
        using var precisionScope = Rl.WithPrecision(ComputationDecimalPlaces, DisplayDecimalPlaces);
        var variables = new Dictionary<string, StateVariable>();
        foreach (var (name, value) in _interpreter.Variables)
            variables[name] = new StateVariable(name, value.Kind, ValueFormatter.Format(value));

        var functions = new Dictionary<string, StateFunction>();
        foreach (var (name, fn) in _interpreter.Functions)
            functions[name] = new StateFunction(fn.Name, fn.Parameters, fn.IsBuiltin, fn.Span);

        return new StateSnapshot(_interpreter.Revision, variables, functions);
    }

    // -----------------------------------------------------------------
    // Diagnostics
    // -----------------------------------------------------------------

    private Diagnostic ToDiagnostic(Exception ex)
    {
        string message = ex.Message;
        int position = 0;

        var match = Regex.Match(message, @"at position (\d+)", RegexOptions.IgnoreCase);
        if (match.Success && int.TryParse(match.Groups[1].Value, out int p))
            position = p;

        var (line, column) = ComputeLineColumn(_lastSource, position);
        return new Diagnostic(message, position, line, column);
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
