using System.Globalization;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization.Metadata;
using Lovelace.Dsp;
using Lovelace.Suite;
using BuiltinDescriptor = Lovelace.Abstractions.BuiltinDescriptor;
using BuiltinCategories = Lovelace.Abstractions.BuiltinCategories;

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
        // registry query state (ADV-08): a READ of the live registry, answered before any script is touched
        bool listFunctions = false;
        string? functionPattern = null;
        string? describeFunction = null;
        int functionLimit = 50;

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
                case "--functions":
                    // a registry LISTING: no script, no evaluation. The pattern is optional, and the next
                    // argument is taken as one only when it does not look like a flag — so --functions
                    // alone and --functions det are both the listing.
                    listFunctions = true;
                    if (i + 1 < args.Length && !args[i + 1].StartsWith('-'))
                        functionPattern = args[++i];
                    break;
                case "--function":
                    if (++i >= args.Length) return Usage(stderr, "--function requires a name argument.");
                    describeFunction = args[i];
                    break;
                case "--function-limit":
                    if (++i >= args.Length) return Usage(stderr, "--function-limit requires a count argument.");
                    if (!int.TryParse(args[i], out int limit) || limit <= 0)
                        return Usage(stderr, "--function-limit requires a positive count.");
                    functionLimit = limit;
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

        // ONE reader for all three surfaces, and one definition of what their text is: the
        // caller's own characters, a leading byte-order mark included, so byte-identical input
        // reports identical positions whichever surface carried it (see ScriptText).
        //
        // A REGISTRY QUERY IS ANSWERED BEFORE ANYTHING IS READ OR PARSED. It is a read of the live
        // registry, so it costs no evaluation, no revision bump and no statement timing — which is why
        // the flags are handled here rather than inside the evaluation path.
        if (listFunctions || describeFunction is not null)
            return await RunRegistryQueryAsync(functionPattern, listFunctions, describeFunction,
                functionLimit, stdout, stderr);

        string source;
        if (eval is not null)
        {
            source = ScriptText.FromArgument(eval);
        }
        else if (file is not null)
        {
            try
            {
                // NOT File.ReadAllText: it detects the encoding from the byte-order mark and consumes
                // it, which silently handed this surface a text one character shorter than the same
                // bytes on --eval/--stdin and moved every published position by one (G-2).
                source = await ScriptText.FromFileAsync(file);
            }
            catch (Exception ex)
            {
                // no engine ran and no statement executed, so the durations are zero and empty.
                // A path the process could not read is a property of the caller's ARGUMENT, not a
                // lexer/parser refusal — nothing was parsed at all, so the ParseError category sent an
                // agent to fix syntax that does not exist (audit E, F6). The code stays specific
                // (FileReadError) and the category follows the sibling caller-level paths, which
                // already cross as TypeMismatch (PlotDirectoryError, PlotFileError).
                return WriteError(stdout, stderr, json, "FileReadError", "TypeMismatch",
                    $"Cannot read script file '{file}': {ex.Message}", recoverable: true,
                    Array.Empty<DiagnosticDto>(), TimeSpan.Zero, Array.Empty<TimingDto>(),
                    Array.Empty<string>());
            }
        }
        else if (stdinMode)
        {
            source = await ScriptText.FromStandardInputAsync(stdin);
        }
        else
        {
            return Usage(stderr, "No script provided. Use --eval <script>, --file <path>, --stdin, or a bare file path.");
        }

        // The protocol's positions index into the text the CALLER supplied; the engine indexes
        // into the semicolon-joined text it is handed. One map per run, built from the caller's own
        // text, so every position the envelope publishes (timings, diagnostics, the overrun
        // diagnostic) is translated by the same rule (see ScriptPositions).
        var script = new ScriptPositions(source);

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
        // The DEADLINE's own clock, started BEFORE the token that arms it. The engine's elapsed clock
        // starts later (it measures the evaluation), so a stop the deadline caused could be measured
        // INSIDE the budget and publish "stopped:true" next to "exceeded:false" with an empty
        // diagnostics array — nothing in the envelope then said the budget had been consumed
        // (round-20 audit I, I-1). The order matters and a test caught it: starting this clock AFTER
        // the token source meant the clock read epsilon LESS than the budget at the instant the token
        // fired, so the overshoot still computed to zero for exactly the runs this is for. Started
        // first, the clock has necessarily passed the budget by the time any stop can be observed.
        var deadlineClock = cancelAfterMs is null ? null : System.Diagnostics.Stopwatch.StartNew();
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
                    recoverable: true, Array.Empty<DiagnosticDto>(), TimeSpan.Zero, Array.Empty<TimingDto>(),
                    Array.Empty<string>());
            }

            var result = await engine.EvaluateAsync(
                script.EngineSource, output, cancellation.Token);

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
                    .Select(f => FunctionEntry(engine, f))
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
                    engine.ProjectValue(result, structuredBudget));

            // the deadline verdict is derived from the SAME elapsed time the envelope publishes, so
            // budget and elapsed are directly comparable (see CancellationDto)
            var deadline = CancellationLedger(cancelAfterMs, engine.LastElapsed, stopped: false,
                deadlineClock?.Elapsed);
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
                Timings(engine.OperationTimings, script),
                deadline,
                deadline is { Exceeded: true }
                    ? new[] { OverrunDiagnostic(deadline, script, engine.OperationTimings, completed: true) }
                    : null,
                Artifacts(engine));

            if (json)
                WriteJson(stdout, envelope, RunJsonContext.Default.RunEnvelopeDto);
            else
                PrintText(stdout, envelope);

            return 0;
        }
        catch (Exception ex)
        {
            var diagnostics = engine.Diagnostics
                .Select(d => PublishDiagnostic(script, d, engine.OperationTimings))
                .ToArray();
            // Which LAYER the failure belongs to (audit E, F6). The engine tokenizes and parses the
            // WHOLE source before it executes the first statement, so when the text it was handed does
            // not parse, the failure that ended the run IS the parser's refusal. The phase is not
            // readable from the exception TYPE — the tokenizer and the parser refuse with the same
            // InvalidOperationException a domain failure uses (Lovelace.Suite/Tokenizer.cs:123,
            // Parser.cs:64-71) — so the phase is measured, on the failure path, by asking the engine
            // to parse that same text again.
            var (code, category, recoverable) = Classify(ex, parsePhase: !SourceParses(engine, script.EngineSource));
            // a run that blew its budget before failing must not look like one that respected it:
            // the verdict travels with the failure, and an overrun adds its own diagnostic
            var failedDeadline = CancellationLedger(cancelAfterMs, engine.LastElapsed,
                stopped: code == "Cancelled", deadlineClock?.Elapsed);
            if (failedDeadline is { Exceeded: true })
                diagnostics = diagnostics
                    .Append(OverrunDiagnostic(failedDeadline, script, engine.OperationTimings, completed: false))
                    .ToArray();
            // Everything the script printed before the run ended crosses in the documented "output"
            // array: invariant 1 (docs/symbolics/dsh-protocol.md:9-10) is not scoped to a successful
            // run, and the captured text is already in hand (audit E, F5). The cancelled envelope
            // ADDITIONALLY keeps the partial* pair it has always published, so no recorded shape
            // regresses.
            string[] committedOutput = SplitLines(output.ToString());
            // a cancelled run is not a failed run: the host reports the structured status together
            // with everything the engine had already committed
            string[]? partialOutput = code == "Cancelled" ? committedOutput : null;

            // ADV-09: THE COMMITTED VARIABLE TABLE IS PUBLISHED ON EVERY FAILURE, not only on a
            // cancellation. The statements that returned before the failure committed their bindings, and
            // the engine holds them — the harnessability benchmark measured partialVariables at 0 %
            // coverage on ordinary failures and named it F3, because an agent repairing a multi-statement
            // program could not see how far it had got. The field keeps the NAME and the SHAPE it has on
            // the cancelled path (one entry per binding, `structured` included), so a consumer written
            // against that path reads this one unchanged; what moved is only WHEN it is present. It is the
            // committed prefix of an INCOMPLETE run, which is what "partial" has always meant here.
            //
            // `partialOutput` stays cancellation-only: on every other failure the committed lines are in
            // `output[]`, and publishing the same array twice under two names is the duplication audit R-3
            // recorded rather than a second fact.
            VariableDto[] partialVariables = ProjectVariables(engine, PrintBudgetOrNull(printBudget));
            // a failed evaluation reports the SAME durations a successful one does: the elapsed pair
            // from one unit selector and one timing entry per statement that ran before the failure
            var unresolved = ex as Lovelace.Suite.UnknownFunctionException;
            return WriteError(stdout, stderr, json, code, category, ex.Message, recoverable, diagnostics,
                engine.LastElapsed, Timings(engine.OperationTimings, script), committedOutput,
                partialOutput, partialVariables, failedDeadline,
                unresolved?.WireSuggestions, unresolved?.WireHints, unresolved?.RegistrySize,
                CallSignature(ex, engine), Artifacts(engine));
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

    /// <summary>
    /// Whether the text the engine was handed parses. <see cref="Classify"/> reads the phase off this
    /// probe, and the probe is one call of the SAME public parse entry the engine itself runs before
    /// it executes anything (<see cref="SuiteEngine.Parse"/>): a text that does not parse ends the
    /// evaluation before the first statement, so the failure that ended the run IS that refusal.
    /// Nothing else is inferred from it: the probe's own exception is discarded, and the envelope's
    /// message, diagnostics and durations all come from the exception the evaluation really threw.
    /// </summary>
    private static bool SourceParses(SuiteEngine engine, string source)
    {
        try
        {
            engine.Parse(source);
            return true;
        }
        catch (Exception)
        {
            return false;
        }
    }

    /// <summary>Maps a failure onto the stable error taxonomy. Message matching is never required
    /// by a consumer: the code and category are structural.
    /// <para>
    /// <paramref name="parsePhase"/> says the failure came from the tokenizer/parser. Those refuse a
    /// source with an <see cref="InvalidOperationException"/> — the SAME type a domain failure uses —
    /// so the type alone cannot place the failure on a layer (audit E, F6;
    /// docs/symbolics/dsh-protocol.md:207-209 spells the categories from the one <c>ErrorCategory</c>
    /// taxonomy, in which a lexer/parser refusal is <c>ParseError</c>). The flag therefore changes the
    /// meaning of that ONE type: every other arm names a condition the phase cannot change, and a
    /// parse-phase DEPTH refusal keeps its own <c>DepthExceeded</c>/<c>BudgetExceeded</c> arm above.
    /// </para></summary>
    private static (string Code, string Category, bool Recoverable) Classify(Exception ex, bool parsePhase) => ex switch
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
        // The parser's own refusal: a syntax error is a ParseError, not a domain problem an agent
        // would try to fix by changing the mathematics. The tokenizer's string refusal reaches the
        // FormatException arm above and already carried the ParseError CATEGORY.
        // A name the engine does not register is a VOCABULARY failure, not a domain one: the caller
        // cannot fix it by changing the mathematics, only by using a name that exists. The taxonomy
        // is FROZEN at its seven members (Lovelace.Symbolics.Tests/DiagnosticContractTests), so a new
        // failure class gets a new CODE - the fine-grained field a consumer matches - and the coarse
        // category is the one that already means "this runtime does not offer that operation". The
        // registry's answer (suggestions/hints/registrySize) travels with it either way.
        Lovelace.Suite.UnknownFunctionException => ("UnknownFunction", "UnsupportedOperation", true),
        InvalidOperationException => parsePhase
            ? ("ParseError", "ParseError", true)
            : ("InvalidOperation", "DomainError", true),
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
        string[] output, string[]? partialOutput = null, VariableDto[]? partialVariables = null,
        CancellationDto? cancellation = null, string[]? suggestions = null, string[]? hints = null,
        int? registrySize = null, CallSignatureDto? signature = null, ArtifactDto[]? artifacts = null)
    {
        if (json)
        {
            // the error envelope carries the human string AND the structural duration produced by the
            // SAME unit selector (Timing.Scale behind both Timing.Format and Duration), so the two
            // forms can never disagree — exactly like the success envelope
            WriteJson(stdout, new RunErrorDto(ProtocolVersion, Lovelace.Symbolics.Printing.FormatHeader, MathIrVersion,
                false, code, category, message, recoverable, diagnostics,
                Lovelace.Suite.Timing.Format(elapsed), Duration(elapsed), timings, output,
                partialOutput, partialVariables, cancellation, suggestions, hints, registrySize, signature,
                artifacts),
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
    private static CancellationDto? CancellationLedger(int? budgetMs, TimeSpan elapsed, bool stopped,
        TimeSpan? deadlineElapsed = null)
    {
        if (budgetMs is not { } budget)
            return null;

        // elapsedMs stays the ENGINE's measurement — the same value the envelope publishes as
        // elapsedTime, so a consumer never reconciles two clocks (and the test that pins that
        // agreement keeps its teeth). The OVERSHOOT, however, belongs to the deadline when the
        // deadline is what ended the run: the token is armed before the evaluation starts, so the
        // engine's measurement can land INSIDE the budget while the stop the caller asked for had
        // already fired. Reading only the engine's number there published "stopped:true" beside
        // "exceeded:false"/"excessMs":0 and suppressed the overrun diagnostic, so no field said the
        // budget had been consumed (round-20 audit I, I-1).
        double elapsedMs = elapsed.TotalMilliseconds;
        double engineExcess = Math.Max(0, elapsedMs - budget);
        double deadlineExcess = stopped && deadlineElapsed is { } deadline
            ? Math.Max(0, deadline.TotalMilliseconds - budget)
            : 0;
        double excessMs = Math.Max(engineExcess, deadlineExcess);
        return new CancellationDto(budget, Math.Round(elapsedMs, 3), stopped, excessMs > 0,
            Math.Round(excessMs, 3));
    }

    /// <summary>
    /// The diagnostic that names an exceeded deadline — published on the success envelope's
    /// <c>diagnostics</c> array and appended to the failure envelope's, so a consumer that reads only
    /// diagnostics still cannot mistake an overrun for a normal run. The position is the last
    /// top-level statement that ran (the statement the deadline passed inside), translated into the
    /// caller's source by the same map every other published position uses.
    /// </summary>
    private static DiagnosticDto OverrunDiagnostic(CancellationDto ledger, ScriptPositions script,
        IReadOnlyList<OperationTiming> timings, bool completed)
    {
        int enginePosition = timings.Count > 0 ? timings[timings.Count - 1].Position : 0;
        var (position, line, column) = Locate(script, enginePosition);
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

    /// <summary>
    /// One engine diagnostic in the caller's coordinates. A lexer/parser refusal carries the only
    /// position the engine ever knows by itself (scraped from its "at position N" message); a RUNTIME
    /// failure carries none (<c>SuiteEngine.ToDiagnostic</c> defaults to 0), because the exception it
    /// is built from is not annotated. But the interpreter records the offset of the statement that
    /// was running when it threw, and the failing statement is always the LAST timing — its
    /// <c>finally</c> adds the entry on the way out (<c>Lovelace.Suite/Interpreter.cs:333-341</c>) — so
    /// whenever any statement ran, the last timing IS the failure's source position. Only a failure
    /// before the first statement (a tokenizer/parser refusal) falls back to the engine's own value.
    /// </summary>
    private static DiagnosticDto PublishDiagnostic(ScriptPositions script, Diagnostic diagnostic,
        IReadOnlyList<OperationTiming> timings)
    {
        int enginePosition = timings.Count > 0
            ? timings[timings.Count - 1].Position
            : diagnostic.Position;
        var (position, line, column) = Locate(script, enginePosition);
        return new DiagnosticDto(diagnostic.Message, position, line, column);
    }

    /// <summary>An engine offset as a caller offset together with its 1-based line and column: the
    /// one translation every published position goes through.</summary>
    private static (int Position, int Line, int Column) Locate(ScriptPositions script, int enginePosition)
    {
        int position = script.ToSourceOffset(enginePosition);
        var (line, column) = script.LineColumn(position);
        return (position, line, column);
    }

    /// <summary>A millisecond count as an invariant-culture string: the envelope must read the same
    /// on a machine whose decimal separator is a comma.</summary>
    private static string FormatMs(double milliseconds) =>
        milliseconds.ToString("0.###", CultureInfo.InvariantCulture);

    private static Lovelace.Symbolics.Printing.PrintBudget? PrintBudgetOrNull(int? maxNodes) =>
        maxNodes is { } nodes ? new Lovelace.Symbolics.Printing.PrintBudget(MaxNodes: nodes) : null;

    /// <summary>
    /// The structured form of one captured variable — the SAME projection the result carries, read
    /// off the LIVE value through the engine's own façade (<see cref="SuiteEngine.ProjectValue"/>),
    /// which is the one projection an embedding host is pointed at too: the DSH runner is a host
    /// like any other, so a value it publishes and a value Studio publishes cannot disagree
    /// (round-21 audit K, K-2). The snapshot and the live dictionary are the same capture: no
    /// evaluation runs between them, so the value is always there; a lookup that somehow misses
    /// still crosses as <c>{"kind":"Null"}</c> rather than dropping the entry.
    /// </summary>
    private static StructuredValueDto ProjectVariable(SuiteEngine engine, string name,
        Lovelace.Symbolics.Printing.PrintBudget? budget) =>
        engine.TryGetVariable(name, out var value)
            ? engine.ProjectValue(value, budget)
            : new StructuredValueDto("Null");

    /// <summary>
    /// Answers a REGISTRY QUERY — <c>--functions [pattern]</c> or <c>--function &lt;name&gt;</c> — from a
    /// freshly wired engine, and returns true when it handled the invocation.
    /// <para>
    /// This is a READ, not an evaluation. Nothing the caller passed is parsed as a script, no statement
    /// runs, no statement timing is produced, and the engine's revision is untouched — which is the whole
    /// point of the surface: before it, the only way to ask the runtime what it can do was to evaluate a
    /// script and read the registry out of the envelope, at ~15 KB of payload and the cost of an
    /// evaluation (ADV-08, F1: name resolution was the measured bottleneck of the harnessability
    /// benchmark, and the L3 arm spent 185 registry lookups over 20 runs to work around exactly this).
    /// </para>
    /// <para>
    /// The listing is bounded and reports its own truncation, so a broad pattern cannot flood an agent's
    /// context; an unknown NAME is the documented <c>UnknownFunction</c> refusal with the registry's
    /// suggestions, so discovery and repair speak the same grammar.
    /// </para>
    /// </summary>
    private static async Task<int> RunRegistryQueryAsync(string? listPattern, bool listRequested,
        string? describeName, int limit, TextWriter stdout, TextWriter stderr)
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new DspPlugin());
        var symbolics = new Lovelace.Symbolics.SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new Lovelace.MathIR.MathIRPlugin(symbolics));

        if (describeName is not null)
        {
            if (!engine.Functions.TryGetValue(describeName, out FunctionDefinition? definition))
            {
                // the SAME refusal grammar a call to that name produces, so an agent that queries a name
                // it half-remembers learns the same repair it would learn by calling it
                IReadOnlyList<string> suggestions = NameSuggestions.Suggest(engine.Functions.Keys, describeName);
                await WriteErrorAsync(stdout, stderr, "UnknownFunction", "UnsupportedOperation",
                    UnknownFunctionMessage(describeName, suggestions), recoverable: true,
                    Array.Empty<DiagnosticDto>(), TimeSpan.Zero, Array.Empty<TimingDto>(),
                    Array.Empty<string>(), suggestions.Count == 0 ? null : suggestions.ToArray(),
                    registrySize: engine.Functions.Count);
                return 1;
            }

            WriteJson(stdout, new RegistryQueryDto(ProtocolVersion,
                Lovelace.Symbolics.Printing.FormatHeader, MathIrVersion, true, Pattern: null,
                Functions: null, Function: FunctionEntry(engine, definition), Total: 1, Returned: 1,
                Truncated: false), RunJsonContext.Default.RegistryQueryDto);
            return 0;
        }

        string pattern = listPattern ?? string.Empty;
        RegistryFunctionDto[] matches = engine.Functions.Values
            .Where(f => f.Name.Contains(pattern, StringComparison.OrdinalIgnoreCase))
            .OrderBy(f => f.Name, StringComparer.Ordinal)
            .Select(f =>
            {
                var (min, _) = f.DeclaredArity();
                BuiltinDescriptor? descriptor =
                    engine.InterpreterBuiltinDescriptors.TryGetValue(f.Name, out BuiltinDescriptor? registered)
                        ? registered
                        : CoreBuiltinMetadata.TryGet(f.Name);
                return new RegistryFunctionDto(f.Name, f.Parameters.ToArray(), min, f.Parameters.Count,
                    f.Variadic, descriptor?.Category ?? BuiltinCategories.Core);
            })
            .ToArray();

        RegistryFunctionDto[] page = matches.Length <= limit ? matches : matches[..limit];
        WriteJson(stdout, new RegistryQueryDto(ProtocolVersion, Lovelace.Symbolics.Printing.FormatHeader,
            MathIrVersion, true, listRequested ? pattern : null, page, Function: null,
            Total: matches.Length, Returned: page.Length, Truncated: page.Length < matches.Length),
            RunJsonContext.Default.RegistryQueryDto);
        return 0;
    }

    /// <summary>The one unresolved-name sentence, so a registry query and a failed CALL cannot describe the
    /// same situation two ways.</summary>
    private static string UnknownFunctionMessage(string name, IReadOnlyList<string> suggestions)
    {
        string message = $"Unknown function '{name}'.";
        if (suggestions.Count == 1)
            message += $" Did you mean '{suggestions[0]}'?";
        else if (suggestions.Count > 1)
            message += " Did you mean one of: " + string.Join(", ", suggestions.Select(s => $"'{s}'")) + "?";
        return message;
    }

    /// <summary>The JSON-only error write a registry query needs: no engine ran, so there is no output, no
    /// timing and no partial state — but the envelope is the same shape every other failure publishes.</summary>
    private static Task WriteErrorAsync(TextWriter stdout, TextWriter stderr, string code, string category,
        string message, bool recoverable, DiagnosticDto[] diagnostics, TimeSpan elapsed, TimingDto[] timings,
        string[] output, string[]? suggestions = null, int? registrySize = null)
    {
        WriteJson(stdout, new RunErrorDto(ProtocolVersion, Lovelace.Symbolics.Printing.FormatHeader,
            MathIrVersion, false, code, category, message, recoverable, diagnostics,
            Lovelace.Suite.Timing.Format(elapsed), Duration(elapsed), timings, output,
            Suggestions: suggestions, RegistrySize: registrySize), RunJsonContext.Default.RunErrorDto);
        return Task.CompletedTask;
    }

    /// <summary>
    /// One registry entry: the declared signature, the arity metadata the call-site validator computes
    /// the arity contract from (A2-F15), and the descriptor metadata that makes the registry a DISCOVERY
    /// surface rather than a list of names (ADV-08). The lower bound is RESOLVED
    /// (<see cref="FunctionDefinition.DeclaredArity"/>), so a consumer never has to know the internal
    /// "exactly the declared count" sentinel, and the upper bound is the declared parameter count unless
    /// <c>variadic</c> removes it.
    /// <para>
    /// The metadata is resolved the SAME way <c>HelpService</c> resolves it — the plugin's own descriptor
    /// first, then the core catalogue, then nothing — so the wire, the help text and the Studio completion
    /// list can never describe one function three ways. A USER-DEFINED function has no descriptor at all,
    /// which is why every metadata field is nullable and omitted rather than synthesised.
    /// </para>
    /// </summary>
    private static FunctionDto FunctionEntry(SuiteEngine engine, FunctionDefinition f)
    {
        var (min, _) = f.DeclaredArity();

        BuiltinDescriptor? descriptor =
            engine.InterpreterBuiltinDescriptors.TryGetValue(f.Name, out BuiltinDescriptor? registered)
                ? registered
                : CoreBuiltinMetadata.TryGet(f.Name);

        return new FunctionDto(
            f.Name, f.Parameters.ToArray(), min, f.Parameters.Count, f.Variadic, f.IsBuiltin, f.PluginName,
            descriptor?.Category ?? BuiltinCategories.Core,
            // "a summary was not registered" is nothing to say, so the field is omitted rather than sent
            // as the placeholder sentence the fallback descriptor carries
            descriptor is null || descriptor.Summary == NoSummaryRegistered ? null : descriptor.Summary,
            descriptor?.ReturnKind,
            descriptor is { Related.Count: > 0 } ? descriptor.Related.ToArray() : null,
            descriptor is { Examples.Count: > 0 } ? descriptor.Examples.ToArray() : null);
    }

    /// <summary>
    /// The CALL-SITE signature a refusal was measured against, or <see langword="null"/> when the failure is
    /// not one (ADV-07).
    /// <para>
    /// The two call-site refusal types already publish everything an agent needs to repair the call —
    /// <c>BuiltinArityException</c> carries the builtin, both counts and the variadic flag;
    /// <c>BuiltinShapeException</c> carries the position, the expectation and the kind that arrived — and
    /// this projects them onto the wire. The declared PARAMETER NAMES come from the live registry rather than
    /// from the exception, because the exception names the position and the registry names what sits there,
    /// so the signature a failure publishes and the signature <c>functions[]</c> publishes for the same
    /// builtin cannot drift.
    /// </para>
    /// <para>
    /// A refusal that is neither of those — an integer-argument contract violation, a domain refusal —
    /// carries no signature: the field is OMITTED rather than sent empty, so "this failure has no signature"
    /// and "this failure has an empty signature" cannot look alike.
    /// </para>
    /// </summary>
    private static CallSignatureDto? CallSignature(Exception ex, SuiteEngine engine)
    {
        switch (ex)
        {
            case Lovelace.Suite.BuiltinArityException arity:
                FunctionDefinition? definition =
                    engine.Functions.TryGetValue(arity.Builtin, out FunctionDefinition? byName) ? byName : null;
                return new CallSignatureDto(
                    arity.Builtin,
                    definition?.Parameters.ToArray(),
                    arity.ExpectedMin,
                    // the exception publishes int.MaxValue for a variadic tail; the wire publishes the same
                    // count the registry publishes, so a consumer adding 1 to an upper bound never overflows
                    arity.Variadic ? null : arity.ExpectedMax,
                    arity.Variadic,
                    arity.Actual);

            case Lovelace.Suite.BuiltinShapeException shape:
                FunctionDefinition? owner =
                    engine.Functions.TryGetValue(shape.Builtin, out FunctionDefinition? defined) ? defined : null;
                return new CallSignatureDto(
                    shape.Builtin,
                    owner?.Parameters.ToArray(),
                    ExpectedMin: owner is null ? null : owner.DeclaredArity().Min,
                    ExpectedMax: owner is { Variadic: false } ? owner.Parameters.Count : null,
                    Variadic: owner?.Variadic,
                    Position: shape.Position,
                    Parameter: owner is not null && shape.Position >= 1 && shape.Position <= owner.Parameters.Count
                        ? owner.Parameters[shape.Position - 1]
                        : null,
                    ExpectedShape: shape.Expected,
                    ArrivedKind: shape.Arrived.ToString());

            default:
                return null;
        }
    }
    /// <summary>
    /// The variable table an envelope publishes, projected the SAME way on every path, so the live table and
    /// the partial one cannot describe a variable two ways.
    /// </summary>
    private static VariableDto[] ProjectVariables(SuiteEngine engine,
        Lovelace.Symbolics.Printing.PrintBudget? structuredBudget)
    {
        var table = new List<VariableDto>();
        foreach (var (name, value) in engine.Variables.OrderBy(v => v.Key, StringComparer.Ordinal))
        {
            try
            {
                // the display string and the structured projection are taken from the SAME value under the
                // SAME budget the success path uses
                table.Add(new VariableDto(name, value.Kind.ToString(), engine.FormatValue(value),
                    engine.ProjectValue(value, structuredBudget)));
            }
            catch (Lovelace.Abstractions.InputDepthExceededException)
            {
                // A BINDING TOO DEEP TO RENDER IS STILL A COMMITTED BINDING, and it must not take the
                // failure path down with it. The depth guard exists to stop a recursive walk exhausting
                // the stack, so a value that trips it cannot be formatted — and a failure envelope that
                // threw while describing itself would replace a typed refusal with an InternalError (the
                // ValueDepthEnvelopeTests caught exactly that). The entry stays, with the display the
                // guard itself produced and no structured form: present, named, and honest about the part
                // that could not be read.
                table.Add(new VariableDto(name, value.Kind.ToString(), TooDeepToRender,
                    ProjectEmpty(value.Kind)));
            }
        }

        return table.ToArray();
    }

    /// <summary>What a binding's display says when the depth guard refused to render it.</summary>
    private const string TooDeepToRender = "(too deeply nested to render)";

    /// <summary>The structured form of a value that could not be projected: the KIND is known, the value is
    /// not, so the projection says exactly that rather than inventing an empty array of some shape.</summary>
    private static StructuredValueDto ProjectEmpty(Lovelace.Suite.ValueKind kind) =>
        new(kind.ToString());

    /// <summary>
    /// Every file the evaluation committed, as structured artifacts (ADV-09).
    /// <para>
    /// A statement that RETURNED wrote its file, so the file is committed state: this is a projection of what
    /// the engine already knows it wrote, published on the success path and the failure path alike. A failure
    /// in a LATER statement cannot un-write an earlier one, which is why the same array appears in both
    /// envelopes — before this, <c>plot([1, 2, 3]); det(1)</c> left <c>plot.svg</c> on disk with nothing in the
    /// error envelope saying so (round-23 audit N, N-5).
    /// </para>
    /// <para>
    /// The SIZE is read from the file rather than assumed: the engine's plot capture proves a file was
    /// written, not how many bytes it holds, and a size the runtime did not read is a size it must not
    /// publish. When the file cannot be read the field is <see langword="null"/> — absent, never zero.
    /// </para>
    /// </summary>
    private static ArtifactDto[] Artifacts(SuiteEngine engine)
    {
        if (engine.LastPlot is not { } capture)
            return Array.Empty<ArtifactDto>();

        string path = Path.GetFullPath(Path.Combine(engine.PlotOutputDirectory, engine.PlotFileName));
        long? bytes = null;
        try
        {
            var info = new FileInfo(path);
            if (info.Exists)
                bytes = info.Length;
        }
        catch (Exception)
        {
            // a path the process cannot stat tells a consumer nothing about the artifact, and inventing a
            // size for it would be worse than saying nothing: the field stays absent.
        }

        return new[] { new ArtifactDto("plot", path, capture.Title, bytes) };
    }
    /// <summary>The one placeholder a descriptor may carry for a summary it does not have, named so the
    /// wire can OMIT the field instead of publishing the placeholder as if it were documentation.</summary>
    private const string NoSummaryRegistered = "(no summary registered)";

    private static DurationDto Duration(TimeSpan elapsed)
    {
        var (value, unit) = Lovelace.Suite.Timing.Scale(elapsed);
        return new DurationDto(value, unit);
    }

    /// <summary>One wire timing per top-level statement that ran, in statement order, each carrying
    /// the statement's offset in the CALLER's source (the engine indexes into the semicolon-joined
    /// text, which is not the caller's text for a CRLF or BOM-prefixed script — see ScriptPositions).</summary>
    private static TimingDto[] Timings(IReadOnlyList<Lovelace.Suite.OperationTiming> timings,
        ScriptPositions script) =>
        timings.Select(t => new TimingDto(
            script.ToSourceOffset(t.Position),
            new DurationDto(t.ElapsedScale.Value, t.ElapsedScale.Unit),
            t.Result.Kind.ToString(),
            t.Output.Length > 0))
        .ToArray();

    /// <summary>
    /// The script's captured print output as LINES. The capture path terminates EVERY printed line
    /// with <see cref="Environment.NewLine"/>, so the text ends with the terminator of the last line
    /// printed — and exactly that ONE terminator is removed here, which also drops the \r the host's
    /// line separator contributed (an artifact of the platform, not something the script printed).
    /// Removing MORE than one used to delete a blank line the script really did print: audit N's N-1
    /// (wave 5, P1) — <c>print("a"); print()</c> published <c>output: ["a"]</c> while BOTH timing
    /// entries said <c>hasOutput: true</c>, so the envelope contradicted its own machine-readable
    /// flag and the line was unrecoverable — while an interior blank line was kept all along. The
    /// envelope's output array carries the lines, never their terminators. The two Studio hosts
    /// (<c>EngineHost.SplitLines</c>, <c>IncrementalRunner.SplitLines</c>) already keep one trailing
    /// blank line; this is the same answer on the CLI.
    /// </summary>
    private static string[] SplitLines(string text)
    {
        if (text.Length == 0)
            return Array.Empty<string>();

        if (text.EndsWith("\r\n", StringComparison.Ordinal))
            text = text[..^2];
        else if (text[^1] is '\n' or '\r')
            text = text[..^1];

        string[] lines = text.Split('\n');
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
            "  Lovelace.Run --functions [pattern]   list registered names (no script, no evaluation)\n" +
            "  Lovelace.Run --function <name>       describe one registered function\n" +
            "\n" +
            "Options:\n" +
            "  --eval <script>      evaluate the given script text\n" +
            "  --file <path>        read the script from a file\n" +
            "  --stdin              read the script from standard input\n" +
            "  --plot-dir <dir>     directory for plot() SVG output\n" +
            "  --plot-file <name>   filename for plot() SVG output (default: plot.svg)\n" +
            "  --functions [pat]    query the builtin registry and exit: a bounded listing (name, arity,\n" +
            "                       category) of the names matching the optional case-insensitive pattern.\n" +
            "                       No script is read or evaluated, so this is the cheap discovery surface.\n" +
            "  --function <name>    describe ONE registered function: signature, arity, category, summary,\n" +
            "                       return kind, see-also names and runnable examples. An unknown name is\n" +
            "                       the same UnknownFunction refusal a call to it produces, with suggestions.\n" +
            "  --function-limit <n> page size for --functions (default 50); the answer reports total,\n" +
            "                       returned and truncated, so a page is never mistaken for the whole list\n" +
            "  --omit-functions     send an EMPTY builtin registry, not the populated one (agent loops)\n" +
            "  --omit-variables     send an EMPTY variables array, not the populated one (agent loops)\n" +
            "  --print-budget <n>   abbreviate structured renderings beyond n nodes, reporting the truncation\n" +
            "  --cancel-after <ms>  cancel the evaluation after the given time, returning the partial result;\n" +
            "                       the envelope's cancellation block reports the deadline verdict\n" +
            "  --json               emit JSON (default)\n" +
            "  --text               emit a human-readable summary\n" +
            "  --help, -h           show this help");
    }
}
