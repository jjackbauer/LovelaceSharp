using System.Globalization;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization.Metadata;
using Lovelace.Dsp;
using Lovelace.Suite;
// the precision scope the structured projection is rendered under (see StructuredValue)
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Run;

/// <summary>
/// Lovelace.Run — a non-interactive script runner over Lovelace.Suite.
///
/// Evaluates a script (from --eval, --file, or --stdin) and emits a single JSON
/// envelope on stdout so it can be driven by scripts, tests, or the DSH plugin.
///
///   Lovelace.Run --eval "x = 1..10" --json
///   Lovelace.Run script.ls --plot-dir out
///
/// Protocol contract (versioned):
///   * stdout carries the envelope and NOTHING else. Anything the script prints
///     with print() is captured and returned in the envelope's "output" array.
///   * Every value is serialized structurally — arrays carry their shape, symbolic
///     values carry BOTH their canonical and pretty forms, complex values carry
///     re/im, domains are domain values, and an absent field is null.
///   * JSON keys are camelCase; structured record FIELD NAMES are snake_case.
///
/// Exit codes: 0 success, 1 script/diagnostic error, 2 usage error.
///
/// JSON is emitted via the source-generated RunJsonContext (see RunProtocol.cs), which is
/// required for Native AOT: the reflection-based serializer is trimmed away there.
///
/// The entry point takes its writers explicitly (rather than touching Console directly)
/// so a test can capture the envelope in-process; Program.cs passes Console.Out and
/// Console.Error, which is byte-for-byte what the runner emitted before the extraction.
/// </summary>
public static class Runner
{
    private const int ProtocolVersion = 1;
    private const int MathIrVersion = 2;

    /// <summary>Evaluates the script named by <paramref name="args"/> and writes the envelope to
    /// <paramref name="stdout"/>. Returns the process exit code (0 success, 1 script/diagnostic
    /// error, 2 usage error).</summary>
    /// <param name="args">Command-line arguments, exactly as the process received them.</param>
    /// <param name="stdout">Receives the JSON envelope (and nothing else) or the --text summary.</param>
    /// <param name="stderr">Receives usage and non-JSON error text.</param>
    /// <param name="stdin">Script source for --stdin; defaults to <see cref="Console.In"/>.</param>
    public static async Task<int> RunAsync(string[] args, TextWriter stdout, TextWriter stderr,
        TextReader? stdin = null)
    {
        stdin ??= Console.In;

        string? eval = null;
        string? file = null;
        string? plotDir = null;
        string? plotFile = null;
        bool stdinMode = false;
        bool json = true;
        bool omitFunctions = false;
        bool omitVariables = false;
        int? cancelAfterMs = null;
        int? printBudget = null;

        for (int i = 0; i < args.Length; i++)
        {
            switch (args[i])
            {
                case "--eval":
                    if (++i >= args.Length) return Usage(stderr, "--eval requires a script argument.");
                    eval = args[i];
                    break;
                case "--file":
                    if (++i >= args.Length) return Usage(stderr, "--file requires a path argument.");
                    file = args[i];
                    break;
                case "--stdin":
                    stdinMode = true;
                    break;
                case "--cancel-after":
                    if (++i >= args.Length) return Usage(stderr, "--cancel-after requires a millisecond argument.");
                    if (!int.TryParse(args[i], out int ms) || ms <= 0)
                        return Usage(stderr, "--cancel-after requires a positive millisecond count.");
                    cancelAfterMs = ms;
                    break;
                case "--omit-variables":
                    // payload control for agent loops, mirroring --omit-functions: the variables array
                    // is state the agent usually already tracks
                    omitVariables = true;
                    break;
                case "--print-budget":
                    if (++i >= args.Length) return Usage(stderr, "--print-budget requires a node count.");
                    if (!int.TryParse(args[i], out int nodes) || nodes <= 0)
                        return Usage(stderr, "--print-budget requires a positive node count.");
                    printBudget = nodes;
                    break;
                case "--omit-functions":
                    // the registry dump (~100 entries) is payload noise for agent loops
                    omitFunctions = true;
                    break;
                case "--plot-dir":
                    if (++i >= args.Length) return Usage(stderr, "--plot-dir requires a directory argument.");
                    plotDir = args[i];
                    break;
                case "--plot-file":
                    if (++i >= args.Length) return Usage(stderr, "--plot-file requires a name argument.");
                    plotFile = args[i];
                    break;
                case "--text":
                    json = false;
                    break;
                case "--json":
                    json = true;
                    break;
                case "--help":
                case "-h":
                    PrintUsage(stdout);
                    return 0;
                default:
                    // First bare argument is treated as a script file path.
                    if (file is null && !args[i].StartsWith('-'))
                    {
                        file = args[i];
                    }
                    else
                    {
                        return Usage(stderr, $"Unknown argument '{args[i]}'.");
                    }
                    break;
            }
        }

        string source;
        if (eval is not null)
        {
            source = eval;
        }
        else if (file is not null)
        {
            try
            {
                source = await File.ReadAllTextAsync(file);
            }
            catch (Exception ex)
            {
                // no engine ran and no statement executed, so the durations are zero and empty
                return WriteError(stdout, stderr, json, "FileReadError", "ParseError", $"Cannot read script file '{file}': {ex.Message}", recoverable: true, Array.Empty<DiagnosticDto>(), TimeSpan.Zero, Array.Empty<TimingDto>());
            }
        }
        else if (stdinMode)
        {
            source = await stdin.ReadToEndAsync();
        }
        else
        {
            return Usage(stderr, "No script provided. Use --eval <script>, --file <path>, --stdin, or a bare file path.");
        }

        var engine = new SuiteEngine();
        engine.LoadPlugin(new DspPlugin());
        var symbolics = new Lovelace.Symbolics.SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new Lovelace.MathIR.MathIRPlugin(symbolics));
        if (plotDir is not null) engine.PlotOutputDirectory = plotDir;
        if (plotFile is not null) engine.PlotFileName = plotFile;

        // capture print() output: the envelope must be the only thing on stdout. Declared outside the
        // try so a cancelled evaluation can still return the output it produced (the partial result).
        var output = new StringWriter();
        using var cancellation = cancelAfterMs is { } budget
            ? new CancellationTokenSource(budget)
            : new CancellationTokenSource();

        try
        {
            // plot() writes into PlotOutputDirectory without creating it, so ensure a fresh --plot-dir
            // works (a no-op when the directory already exists). The preflight runs INSIDE the guarded
            // region on purpose: a directory the caller named but the process cannot create is a
            // caller-level failure, and it must cross as an error envelope with a code and a category —
            // never as an unhandled exception with an empty stdout. This call used to sit BEFORE the
            // try and aborted the process (F1-C: exit 0xC0000409, 0 bytes on stdout, no envelope).
            try
            {
                Directory.CreateDirectory(engine.PlotOutputDirectory);
            }
            catch (Exception ex) when (IsUnusablePlotDirectory(ex))
            {
                // no engine ran and no statement executed, so the durations are zero and empty — the
                // same shape the unreadable-script-file path publishes
                return WriteError(stdout, stderr, json, "PlotDirectoryError", "TypeMismatch",
                    $"Cannot use plot directory '{engine.PlotOutputDirectory}': {ex.Message}",
                    recoverable: true, Array.Empty<DiagnosticDto>(), TimeSpan.Zero, Array.Empty<TimingDto>());
            }

            var result = await engine.EvaluateAsync(
                ScriptSource.ToSemicolonStatements(source), output, cancellation.Token);

            var snapshot = engine.CaptureState();
            // one budget for every structured rendering in the envelope: the result AND each
            // variable, so a consumer never has to reconcile two abbreviation policies
            var structuredBudget = PrintBudgetOrNull(printBudget);
            var variables = omitVariables
                ? Array.Empty<VariableDto>()
                : snapshot.Variables.Values
                    .OrderBy(v => v.Name, StringComparer.Ordinal)
                    .Select(v => new VariableDto(v.Name, v.Kind.ToString(), v.Display,
                        ProjectVariable(engine, v.Name, structuredBudget)))
                    .ToArray();
            var functions = omitFunctions
                ? Array.Empty<FunctionDto>()
                : engine.Functions.Values
                    .OrderBy(f => f.Name, StringComparer.Ordinal)
                    .Select(FunctionEntry)
                    .ToArray();

            PlotDto? plot = null;
            if (engine.LastPlot is { } capture)
            {
                string path = Path.GetFullPath(Path.Combine(engine.PlotOutputDirectory, engine.PlotFileName));
                plot = new PlotDto(path, capture.Title ?? string.Empty, capture.Svg ?? string.Empty);
            }

            ResultDto? resultPayload = result.Kind == ValueKind.Void
                ? null
                : new ResultDto(result.Kind.ToString(), engine.FormatValue(result), engine.FormatValueTyped(result),
                    StructuredValue(engine, result, structuredBudget));

            // the deadline verdict is derived from the SAME elapsed time the envelope publishes, so
            // budget and elapsed are directly comparable (see CancellationDto)
            var deadline = CancellationLedger(cancelAfterMs, engine.LastElapsed, stopped: false);
            var envelope = new RunEnvelopeDto(
                ProtocolVersion,
                Lovelace.Symbolics.Printing.FormatHeader,
                MathIrVersion,
                true,
                snapshot.Revision,
                resultPayload,
                SplitLines(output.ToString()),
                variables,
                functions,
                plot,
                engine.LastElapsedDisplay,
                Duration(engine.LastElapsed),
                Timings(engine.OperationTimings),
                deadline,
                deadline is { Exceeded: true }
                    ? new[] { OverrunDiagnostic(deadline, source, engine.OperationTimings, completed: true) }
                    : null);

            if (json)
                WriteJson(stdout, envelope, RunJsonContext.Default.RunEnvelopeDto);
            else
                PrintText(stdout, envelope);

            return 0;
        }
        catch (Exception ex)
        {
            var diagnostics = engine.Diagnostics
                .Select(d => new DiagnosticDto(d.Message, d.Position, d.Line, d.Column))
                .ToArray();
            var (code, category, recoverable) = Classify(ex);
            // a run that blew its budget before failing must not look like one that respected it:
            // the verdict travels with the failure, and an overrun adds its own diagnostic
            var failedDeadline = CancellationLedger(cancelAfterMs, engine.LastElapsed, stopped: code == "Cancelled");
            if (failedDeadline is { Exceeded: true })
                diagnostics = diagnostics
                    .Append(OverrunDiagnostic(failedDeadline, source, engine.OperationTimings, completed: false))
                    .ToArray();
            // a cancelled run is not a failed run: the host reports the structured status together
            // with everything the engine had already committed
            string[]? partialOutput = code == "Cancelled" ? SplitLines(output.ToString()) : null;
            VariableDto[]? partialVariables = code == "Cancelled"
                ? engine.CaptureState().Variables.Values
                    .OrderBy(v => v.Name, StringComparer.Ordinal)
                    .Select(v => new VariableDto(v.Name, v.Kind.ToString(), v.Display,
                        ProjectVariable(engine, v.Name, PrintBudgetOrNull(printBudget))))
                    .ToArray()
                : null;
            // a failed evaluation reports the SAME durations a successful one does: the elapsed pair
            // from one unit selector and one timing entry per statement that ran before the failure
            return WriteError(stdout, stderr, json, code, category, ex.Message, recoverable, diagnostics,
                engine.LastElapsed, Timings(engine.OperationTimings), partialOutput, partialVariables, failedDeadline);
        }
    }

    /// <summary>
    /// The failures a caller-supplied plot directory can produce: an empty or malformed path
    /// (<see cref="ArgumentException"/> / <see cref="NotSupportedException"/>), a path that names an
    /// existing FILE or a volume that cannot be reached (<see cref="IOException"/>, which also covers
    /// <see cref="PathTooLongException"/>), and a missing permission
    /// (<see cref="UnauthorizedAccessException"/>). Each is a property of the ARGUMENT, not of the
    /// engine, so each crosses as the caller-level <c>PlotDirectoryError</c>/<c>TypeMismatch</c>
    /// failure instead of being reported as an internal invariant failure (F1-C).
    /// <para>
    /// The filter is deliberately narrow and never swallows the diagnostic: the exception object is
    /// what the envelope's message is built from, and any exception outside this set keeps
    /// propagating to the general handler below, which still answers with an envelope (code
    /// <c>InternalError</c>) rather than aborting the process or leaving stdout empty.
    /// </para>
    /// </summary>
    private static bool IsUnusablePlotDirectory(Exception ex) =>
        ex is ArgumentException or IOException or UnauthorizedAccessException or NotSupportedException;

    /// <summary>Maps a failure onto the stable error taxonomy. Message matching is never required
    /// by a consumer: the code and category are structural.</summary>
    private static (string Code, string Category, bool Recoverable) Classify(Exception ex) => ex switch
    {
        Lovelace.Suite.EvaluationCancelledException => ("Cancelled", "BudgetExceeded", true),
        Lovelace.Suite.ReentrancyNotSupportedException => ("ReentrancyNotSupported", "UnsupportedOperation", true),
        // a budget stop is a first-class outcome, not an invariant failure: the frozen contract makes
        // BudgetExceeded a status, and the caller can retry with a larger budget
        Lovelace.Symbolics.BudgetExceededException => ("BudgetExceeded", "BudgetExceeded", true),
        // A nesting refusal is a budget stop too, but it gets its OWN code: a consumer must be able
        // to tell "this input is nested too deeply" (a property of the input) from "the symbolic
        // computation ran out of steps" (a property of the work), and neither is a DomainError.
        Lovelace.Abstractions.InputDepthExceededException => ("DepthExceeded", "BudgetExceeded", true),
        // A single request larger than the engine's allocation budget is refused BEFORE it is attempted
        // (Lovelace.Suite.ArrayAllocationBudget), so it is the same kind of outcome as a depth refusal: a
        // budget stop with its own code, recoverable, and never the internal-invariant class the
        // OutOfMemoryException it used to be reached (F2-C).
        Lovelace.Suite.ArrayAllocationRefusedException => ("AllocationRefused", "BudgetExceeded", true),
        // the backstop for every allocating site the budget above does not cover (a range, a kernel, a
        // string builder): an allocation the process could not serve is still a resource budget the caller
        // can retry smaller — never "do not retry" about a merely-too-large request
        OutOfMemoryException => ("AllocationRefused", "BudgetExceeded", true),
        // An unwritable plot output path is a property of the caller's --plot-file (or of the engine's
        // default name under the caller's --plot-dir), exactly like the plot DIRECTORY preflight above, so
        // it crosses with its own code instead of the internal-invariant class (F3-C).
        Lovelace.Suite.PlotFileWriteException => ("PlotFileError", "TypeMismatch", true),
        Lovelace.Symbolics.EvaluationException => ("EvaluationError", "DomainError", true),
        Lovelace.Symbolics.AssumptionContradictionException => ("UnsatisfiableAssumptions", "DomainError", true),
        FormatException => ("InvalidInput", "ParseError", true),
        NotSupportedException => ("UnsupportedOperation", "UnsupportedOperation", true),
        InvalidOperationException => ("InvalidOperation", "DomainError", true),
        ArgumentException => ("InvalidArgument", "TypeMismatch", true),
        // framework types that still escape the kernel are classified honestly rather than being
        // reported as internal invariant failures: they are user-level errors and recoverable
        NotImplementedException => ("UnsupportedOperation", "UnsupportedOperation", true),
        DivideByZeroException => ("DivisionByZero", "DomainError", true),
        ArithmeticException => ("ArithmeticError", "DomainError", true),
        _ => ("InternalError", "InternalInvariantFailure", false),
    };

    private static int WriteError(TextWriter stdout, TextWriter stderr,
        bool json, string code, string category, string message, bool recoverable,
        DiagnosticDto[] diagnostics, TimeSpan elapsed, TimingDto[] timings,
        string[]? output = null, VariableDto[]? variables = null,
        CancellationDto? cancellation = null)
    {
        if (json)
        {
            // the error envelope carries the human string AND the structural duration produced by the
            // SAME unit selector (Timing.Scale behind both Timing.Format and Duration), so the two
            // forms can never disagree — exactly like the success envelope
            WriteJson(stdout, new RunErrorDto(ProtocolVersion, Lovelace.Symbolics.Printing.FormatHeader, MathIrVersion,
                false, code, category, message, recoverable, diagnostics,
                Lovelace.Suite.Timing.Format(elapsed), Duration(elapsed), timings, output, variables, cancellation),
                RunJsonContext.Default.RunErrorDto);
        }
        else
        {
            stderr.WriteLine($"Error [{code}/{category}]: {message}");
        }
        return 1;
    }


    // -----------------------------------------------------------------
    // Cancellation budget
    // -----------------------------------------------------------------

    /// <summary>
    /// The budget ledger for one run, or <see langword="null"/> when no <c>--cancel-after</c> budget
    /// was given (the envelope then carries no cancellation block at all, exactly as before this
    /// field existed). <paramref name="elapsed"/> is the engine's own measurement — the SAME value
    /// published as <c>elapsedTime</c> — so a consumer never has to reconcile two clocks, and
    /// <paramref name="stopped"/> says whether the deadline is what ended the run.
    /// </summary>
    private static CancellationDto? CancellationLedger(int? budgetMs, TimeSpan elapsed, bool stopped)
    {
        if (budgetMs is not { } budget)
            return null;

        double elapsedMs = elapsed.TotalMilliseconds;
        double excessMs = Math.Max(0, elapsedMs - budget);
        return new CancellationDto(budget, Math.Round(elapsedMs, 3), stopped, excessMs > 0,
            Math.Round(excessMs, 3));
    }

    /// <summary>
    /// The diagnostic that names an exceeded deadline — published on the success envelope's
    /// <c>diagnostics</c> array and appended to the failure envelope's, so a consumer that reads only
    /// diagnostics still cannot mistake an overrun for a normal run. The position is the last
    /// top-level statement that ran (the statement the deadline passed inside); the line/column rule
    /// is the one the engine uses (<c>SuiteEngine.ComputeLineColumn</c>), kept here because the
    /// runner owns the source text it was handed.
    /// </summary>
    private static DiagnosticDto OverrunDiagnostic(CancellationDto ledger, string source,
        IReadOnlyList<OperationTiming> timings, bool completed)
    {
        int position = timings.Count > 0 ? timings[timings.Count - 1].Position : 0;
        var (line, column) = ComputeLineColumn(source, position);
        string outcome = ledger.Stopped
            ? "the deadline was observed, but only after the excess had already been spent"
            : completed
                ? "the evaluation completed anyway"
                : "the evaluation then failed for another reason";
        return new DiagnosticDto(
            $"cancellation deadline exceeded: --cancel-after {ledger.BudgetMs} ms, " +
            $"elapsed {FormatMs(ledger.ElapsedMs)} ms " +
            $"({FormatMs(ledger.ExcessMs)} ms over budget); {outcome}.",
            position, line, column);
    }

    /// <summary>1-based line and column of a source offset, matching the engine's own rule.</summary>
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

    /// <summary>A millisecond count as an invariant-culture string: the envelope must read the same
    /// on a machine whose decimal separator is a comma.</summary>
    private static string FormatMs(double milliseconds) =>
        milliseconds.ToString("0.###", CultureInfo.InvariantCulture);

    private static Lovelace.Symbolics.Printing.PrintBudget? PrintBudgetOrNull(int? maxNodes) =>
        maxNodes is { } nodes ? new Lovelace.Symbolics.Printing.PrintBudget(MaxNodes: nodes) : null;

    /// <summary>
    /// The fractional-digit room a STRUCTURED rendering is given, as opposed to the display bound
    /// <see cref="Rl.DisplayDecimalPlaces"/> imposes on <c>ToString</c>. The machine payload carries
    /// the digits the value STORES, so the room must be larger than any value can be: a Real with a
    /// billion fractional digits is roughly a gigabyte of decimal text, and every route into one is
    /// bounded far below that (<c>pi</c>/<c>e</c> refuse counts past the 1000-place static cap, and
    /// <c>evalf</c> clamps its count to the same 1000). It is a guard against an unreachable value,
    /// not a policy bound: no digit a script asks for is ever cut at it.
    /// </summary>
    private const long StructuredDecimalDigits = 1_000_000_000L;

    /// <summary>
    /// The structured form of one value — the machine API's copy of it, and the SAME projection the
    /// result and every variable carry (L1, audit B). It is rendered with room for the digits the
    /// value stores rather than for the digits the display setting shows: <c>Real.ToString()</c>
    /// truncates a non-periodic fraction at <see cref="Rl.DisplayDecimalPlaces"/>, so a payload
    /// projected under the process default carried exactly 100 decimals for a
    /// <c>setprecision(1100); pi(1100)</c> that owns 1 100 — silently, with none of the DTO's
    /// truncation fields set, although the protocol requires a bounded structured rendering to say
    /// so. The display bound stays a DISPLAY bound: <c>display</c>/<c>typed</c> are rendered at the
    /// engine's precision by the caller, the structure is rendered in full.
    /// </summary>
    private static StructuredValueDto StructuredValue(SuiteEngine engine, Value value,
        Lovelace.Symbolics.Printing.PrintBudget? budget)
    {
        using var _ = Rl.WithPrecision(engine.ComputationDecimalPlaces, StructuredDecimalDigits);
        return StructuredProjection.ToStructured(value, budget);
    }

    /// <summary>
    /// The structured form of one captured variable — the SAME projection the result carries, read
    /// off the LIVE value under the engine's display precision so a variable's <c>display</c> string
    /// and its <c>structured</c> form come from one set of settings (A2-F24). The snapshot and the
    /// live dictionary are the same capture: no evaluation runs between them, so the value is always
    /// there; a lookup that somehow misses still crosses as <c>{"kind":"Null"}</c> rather than
    /// dropping the entry.
    /// </summary>
    private static StructuredValueDto ProjectVariable(SuiteEngine engine, string name,
        Lovelace.Symbolics.Printing.PrintBudget? budget) =>
        engine.TryGetVariable(name, out var value)
            ? StructuredValue(engine, value, budget)
            : new StructuredValueDto("Null");

    /// <summary>
    /// One registry entry: the declared signature plus the arity metadata the call-site validator
    /// computes the arity contract from (A2-F15). The lower bound is RESOLVED
    /// (<see cref="FunctionDefinition.DeclaredArity"/>), so a consumer never has to know the
    /// internal "exactly the declared count" sentinel, and the upper bound is the declared parameter
    /// count unless <c>variadic</c> removes it.
    /// </summary>
    private static FunctionDto FunctionEntry(FunctionDefinition f)
    {
        var (min, _) = f.DeclaredArity();
        return new FunctionDto(f.Name, f.Parameters.ToArray(), min, f.Parameters.Count,
            f.Variadic, f.IsBuiltin, f.PluginName);
    }

    private static DurationDto Duration(TimeSpan elapsed)
    {
        var (value, unit) = Lovelace.Suite.Timing.Scale(elapsed);
        return new DurationDto(value, unit);
    }

    /// <summary>One wire timing per top-level statement that ran, in statement order.</summary>
    private static TimingDto[] Timings(IReadOnlyList<Lovelace.Suite.OperationTiming> timings) =>
        timings.Select(t => new TimingDto(
            t.Position,
            new DurationDto(t.ElapsedScale.Value, t.ElapsedScale.Unit),
            t.Result.Kind.ToString(),
            t.Output.Length > 0))
        .ToArray();

    /// <summary>
    /// The script's captured print output as LINES. The capture path terminates every line with
    /// <see cref="Environment.NewLine"/>, so on Windows each element but the last used to keep the
    /// \r of the terminator — an artifact of the host's line separator, not something the script
    /// printed. The envelope's output array carries the line, never its terminator.
    /// </summary>
    private static string[] SplitLines(string text)
    {
        if (text.Length == 0)
            return Array.Empty<string>();

        string[] lines = text.TrimEnd('\r', '\n').Split('\n');
        for (int i = 0; i < lines.Length; i++)
            lines[i] = lines[i].TrimEnd('\r');
        return lines;
    }

    private static void PrintText(TextWriter stdout, RunEnvelopeDto envelope)
    {
        var sb = new StringBuilder();
        if (envelope.Result is { } r)
            sb.AppendLine($"= {r.Typed}");
        foreach (var line in envelope.Output)
            sb.AppendLine(line);
        foreach (var v in envelope.Variables)
            sb.AppendLine($"  {v.Name} = {v.Display}");
        if (envelope.Cancellation is { Exceeded: true } ledger)
            sb.AppendLine($"! cancellation deadline exceeded: --cancel-after {ledger.BudgetMs} ms, " +
                          $"elapsed {FormatMs(ledger.ElapsedMs)} ms ({FormatMs(ledger.ExcessMs)} ms over budget)" +
                          (ledger.Stopped ? "; the deadline was observed late." : "; the deadline did not stop it."));
        stdout.Write(sb.ToString());
    }

    private static void WriteJson(TextWriter stdout, object value, JsonTypeInfo typeInfo) =>
        stdout.WriteLine(JsonSerializer.Serialize(value, typeInfo));

    private static int Usage(TextWriter stderr, string message)
    {
        stderr.WriteLine($"Error: {message}");
        PrintUsage(stderr);
        return 2;
    }

    private static void PrintUsage(TextWriter writer)
    {
        writer.WriteLine(
            "Lovelace.Run — evaluate a Lovelace script and emit a JSON envelope.\n" +
            "\n" +
            "Usage:\n" +
            "  Lovelace.Run --eval \"<script>\" [options]\n" +
            "  Lovelace.Run <file.ls> [options]\n" +
            "  Lovelace.Run --stdin [options]\n" +
            "\n" +
            "Options:\n" +
            "  --eval <script>      evaluate the given script text\n" +
            "  --file <path>        read the script from a file\n" +
            "  --stdin              read the script from standard input\n" +
            "  --plot-dir <dir>     directory for plot() SVG output\n" +
            "  --plot-file <name>   filename for plot() SVG output (default: plot.svg)\n" +
            "  --omit-functions     omit the builtin registry from the envelope (agent loops)\n" +
            "  --omit-variables     omit the variables array from the envelope (agent loops)\n" +
            "  --print-budget <n>   abbreviate structured renderings beyond n nodes, reporting the truncation\n" +
            "  --cancel-after <ms>  cancel the evaluation after the given time, returning the partial result;\n" +
            "                       the envelope's cancellation block reports the deadline verdict\n" +
            "  --json               emit JSON (default)\n" +
            "  --text               emit a human-readable summary\n" +
            "  --help, -h           show this help");
    }
}
