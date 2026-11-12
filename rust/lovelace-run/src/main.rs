//! The non-interactive script runner and the agent-facing wire protocol.
//!
//! The contract this binary keeps is the one an external harness already depends on: one JSON
//! document on stdout and nothing else, a stable field set, and three exit codes
//! (0 success, 1 script or diagnostic failure, 2 usage). The --text flag selects human output.
//!
//! Positions published in the envelope are UTF-16 code-unit offsets into the caller's text,
//! which is what the C# runner publishes and what a consumer slices with.

use std::io::{Read, Write};

use lovelace_json::Json;
use lovelace_lang::{Engine, registry, structured};

use script_source::{ScriptSource, SourceIdentity, SourceOrigin};

const PROTOCOL_VERSION: &str = "1";
const SYMBOLIC_FORMAT_VERSION: &str = "#!lovelace-sym 1";
const MATH_IR_VERSION: &str = "2";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Json,
    Text,
}

struct Options {
    source: Option<String>,
    /// Which host surface the script text came from, so a diagnostic can name its source.
    origin: SourceOrigin,
    /// A script path to read after argument parsing, so an unreadable path can cross as the
    /// documented pre-engine failure envelope instead of a usage error on stderr.
    source_file: Option<String>,
    plot_dir: Option<String>,
    plot_file: Option<String>,
    mode: Mode,
    omit_functions: bool,
    omit_variables: bool,
    print_budget: Option<usize>,
    cancel_after: Option<u64>,
    list_functions: bool,
    function_pattern: Option<String>,
    describe_function: Option<String>,
    function_limit: usize,
}

fn usage() -> String {
    let mut s = String::new();
    s.push_str("lovelace-run - evaluate a Lovelace script\n\n");
    s.push_str(
        "USAGE:\n  lovelace-run (--eval <src> | --file <path> | --stdin | <path>) [options]\n\n",
    );
    s.push_str("OPTIONS:\n");
    s.push_str("  --eval <src>          evaluate the given source\n");
    s.push_str("  --file <path>         evaluate a file\n");
    s.push_str("  --stdin               read the script from standard input\n");
    s.push_str("  --plot-dir <dir>      directory for plot output\n");
    s.push_str("  --plot-file <name>    plot file name (default plot.svg)\n");
    s.push_str("  --cancel-after <ms>   cancel the evaluation after the given time, returning the partial result\n");
    s.push_str("  --print-budget <n>    node budget for structured print output\n");
    s.push_str("  --omit-functions      publish an empty functions array\n");
    s.push_str("  --omit-variables      publish an empty variables array\n");
    s.push_str("  --functions [pattern] list the builtin registry (no evaluation)\n");
    s.push_str("  --function <name>     describe one builtin (no evaluation)\n");
    s.push_str("  --function-limit <n>  bound a registry listing\n");
    s.push_str("  --json                machine mode (default)\n");
    s.push_str("  --text                human mode\n");
    s.push_str("  --help, -h            this message\n");
    s
}

fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut o = Options {
        source: None,
        origin: SourceOrigin::Eval,
        source_file: None,
        plot_dir: None,
        plot_file: None,
        mode: Mode::Json,
        omit_functions: false,
        omit_variables: false,
        print_budget: None,
        cancel_after: None,
        list_functions: false,
        function_pattern: None,
        describe_function: None,
        function_limit: 50,
    };
    let mut bare: Option<String> = None;
    let mut i = 0usize;
    while i < args.len() {
        let a = args[i].clone();
        let need = |i: usize| -> Result<String, String> {
            args.get(i + 1)
                .cloned()
                .ok_or_else(|| format!("{} requires a value", args[i]))
        };
        match a.as_str() {
            "--eval" => {
                o.source = Some(need(i)?);
                o.origin = SourceOrigin::Eval;
                i += 2;
            }
            "--file" => {
                o.source_file = Some(need(i)?);
                o.origin = SourceOrigin::File;
                i += 2;
            }
            "--stdin" => {
                let mut bytes = Vec::new();
                std::io::stdin()
                    .read_to_end(&mut bytes)
                    .map_err(|e| format!("cannot read stdin: {e}"))?;
                // The characters the reader produced, exactly as produced: a UTF-8 byte-order
                // mark is source text (the ENGINE strips it and maps positions back), not
                // transport for this surface to consume.
                let text =
                    decode_stdin_bytes(bytes).map_err(|e| format!("cannot read stdin: {e}"))?;
                o.source = Some(text);
                o.origin = SourceOrigin::Stdin;
                i += 1;
            }
            "--plot-dir" => {
                o.plot_dir = Some(need(i)?);
                i += 2;
            }
            "--plot-file" => {
                o.plot_file = Some(need(i)?);
                i += 2;
            }
            "--cancel-after" => {
                let v = need(i)?;
                let ms = v.parse::<u64>().map_err(|_| {
                    "--cancel-after requires a positive millisecond count".to_string()
                })?;
                if ms == 0 {
                    return Err("--cancel-after requires a positive millisecond count".to_string());
                }
                o.cancel_after = Some(ms);
                i += 2;
            }
            "--print-budget" => {
                let v = need(i)?;
                let nodes = v
                    .parse::<usize>()
                    .map_err(|_| "--print-budget requires a positive node count".to_string())?;
                if nodes == 0 {
                    return Err("--print-budget requires a positive node count".to_string());
                }
                o.print_budget = Some(nodes);
                i += 2;
            }
            "--omit-functions" => {
                o.omit_functions = true;
                i += 1;
            }
            "--omit-variables" => {
                o.omit_variables = true;
                i += 1;
            }
            "--functions" => {
                o.list_functions = true;
                if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                    o.function_pattern = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--function" => {
                o.describe_function = Some(need(i)?);
                i += 2;
            }
            "--function-limit" => {
                let v = need(i)?;
                let limit = v
                    .parse::<usize>()
                    .map_err(|_| "--function-limit requires a positive count".to_string())?;
                if limit == 0 {
                    return Err("--function-limit requires a positive count".to_string());
                }
                o.function_limit = limit;
                i += 2;
            }
            "--json" => {
                o.mode = Mode::Json;
                i += 1;
            }
            "--text" => {
                o.mode = Mode::Text;
                i += 1;
            }
            "--help" | "-h" => {
                print!("{}", usage());
                std::process::exit(0);
            }
            other if other.starts_with('-') => return Err(format!("unknown option '{other}'")),
            other => {
                if bare.is_some() {
                    return Err("more than one script path given".to_string());
                }
                bare = Some(other.to_string());
                i += 1;
            }
        }
    }
    if o.source.is_none()
        && o.source_file.is_none()
        && !o.list_functions
        && o.describe_function.is_none()
    {
        if let Some(p) = bare {
            o.source_file = Some(p);
            o.origin = SourceOrigin::File;
        }
    } else if bare.is_some() && (o.source.is_some() || o.source_file.is_some()) {
        return Err("a script path was given together with another source".to_string());
    }
    if o.source.is_none()
        && o.source_file.is_none()
        && !o.list_functions
        && o.describe_function.is_none()
    {
        return Err("no script given (use --eval, --file, --stdin, or a path)".to_string());
    }
    Ok(o)
}

/// The native stack the runner gives the interpreter.
///
/// Evaluation is a tree-walking recursion: every user call level costs native stack, so the
/// stack — not the syntax — bounds the recursion a script may use. The reference accepts the
/// documented deepest recursion (`f(85)`: four budget units per call level against a budget of
/// 512) and this runner must accept the same script instead of dying with 0xC00000FD. The
/// reservation is virtual address space committed lazily by the OS, not resident memory.
const RUNNER_STACK_BYTES: usize = 256 * 1024 * 1024;

fn main() {
    // Everything — argument handling, parsing, evaluation, rendering — runs on this one thread,
    // so the stack is the same for a registry query and for a script.
    let worker = std::thread::Builder::new()
        .name("lovelace-run".to_string())
        .stack_size(RUNNER_STACK_BYTES)
        .spawn(run)
        .expect("cannot start the runner thread");
    if let Err(payload) = worker.join() {
        // Preserve the default panic contract: the hook already printed the message, and
        // resuming here keeps the process exit code a panic (101) rather than a plain failure.
        std::panic::resume_unwind(payload);
    }
}

fn run() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let opts = match parse_args(&args) {
        Ok(o) => o,
        Err(msg) => {
            eprintln!("error: {msg}\n\n{}", usage());
            std::process::exit(2);
        }
    };

    // A REGISTRY QUERY IS A READ: no script is parsed, nothing is evaluated, and the revision is
    // untouched. It is answered before any creation of directories or evaluation.
    if opts.list_functions || opts.describe_function.is_some() {
        registry_query(&opts);
        return;
    }

    // The script text is read BEFORE any engine exists: a path the process cannot read is a
    // property of the caller's argument, so it crosses as FileReadError/TypeMismatch with the
    // documented envelope rather than as a usage error on stderr.
    let source = if let Some(path) = &opts.source_file {
        match std::fs::read(path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| decode_script_bytes(&bytes))
        {
            Ok(text) => text,
            Err(e) => emit_pre_engine_error(
                &opts,
                "FileReadError",
                "TypeMismatch",
                format!("Cannot read script file '{path}': {e}"),
            ),
        }
    } else {
        opts.source.clone().unwrap_or_default()
    };

    // The source identity is host metadata: the wire diagnostic keeps its four fields, but the
    // Diagnostic value the envelope is built from names the surface the text came from.
    let identity = match &opts.source_file {
        Some(path) => SourceIdentity::file(path.clone()),
        None => match opts.origin {
            SourceOrigin::Stdin => SourceIdentity::stdin(),
            _ => SourceIdentity::eval(),
        },
    };
    let script = ScriptSource::new(identity, source);

    let mut engine = Engine::new();
    if let Some(dir) = &opts.plot_dir {
        engine.set_plot_dir(dir);
        // plot() writes without creating the directory, so a fresh --plot-dir must be created.
        // A directory the process cannot create is a caller-level failure, not a crash: it crosses
        // as PlotDirectoryError/TypeMismatch with the same envelope shape.
        if let Err(e) = std::fs::create_dir_all(dir) {
            emit_pre_engine_error(
                &opts,
                "PlotDirectoryError",
                "TypeMismatch",
                format!("Cannot use plot directory '{dir}': {e}"),
            );
        }
    }
    if let Some(name) = &opts.plot_file {
        engine.set_plot_file(name);
    }
    let execution = execute(&mut engine, &opts, script.text());

    match (opts.mode, &execution) {
        (Mode::Json, _) => {
            let (env, exit) = envelope_for(&mut engine, &opts, &script, execution);
            print_json(&env);
            if exit != 0 {
                std::process::exit(1);
            }
        }
        (
            Mode::Text,
            Execution {
                outcome: Ok(value),
                elapsed_ms,
                ..
            },
        ) => {
            println!("= {}", engine.format_typed(value));
            for line in engine.take_output() {
                println!("{line}");
            }
            if !opts.omit_variables {
                for (name, v) in engine.variable_values() {
                    println!("  {} = {}", name, engine.format(&v));
                }
            }
            if let Some(budget) = opts.cancel_after {
                if *elapsed_ms > budget as f64 {
                    let excess = (*elapsed_ms - budget as f64).max(0.0);
                    println!(
                        "! cancellation deadline exceeded: --cancel-after {budget} ms, elapsed {} ms ({} ms over budget); the deadline did not stop it.",
                        format_ms(*elapsed_ms),
                        format_ms(excess)
                    );
                }
            }
        }
        (
            Mode::Text,
            Execution {
                outcome: Err(err), ..
            },
        ) => {
            eprintln!("Error [{}/{}]: {}", err.code, err.category, err.message);
            std::process::exit(1);
        }
    }
}

/// The engine's verdict on one script plus the two clocks the envelope publishes.
struct Execution {
    outcome: Result<lovelace_lang::Value, lovelace_lang::LangError>,
    elapsed: std::time::Duration,
    elapsed_ms: f64,
}

/// Runs the script under the caller's deadline. The SAME function serves the binary and the
/// in-process wire tests, so an envelope assertion cannot pass on a flow `main` does not run.
fn execute(engine: &mut Engine, opts: &Options, source: &str) -> Execution {
    engine.set_cancel_after(opts.cancel_after.map(std::time::Duration::from_millis));
    let started = std::time::Instant::now();
    let outcome = engine.eval(source);
    let elapsed = started.elapsed();
    let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
    Execution {
        outcome,
        elapsed,
        elapsed_ms,
    }
}

/// The envelope and exit code for one execution.
///
/// A kernel that observed the deadline unwinds with the structured `Cancelled` failure: that is
/// the cancelled envelope (exit 1, ledger `stopped:true`, the statement it was running as the
/// diagnostic position). A run that finished between two polls is a SUCCESS envelope whose ledger
/// reports the overrun truthfully (`stopped:false`, `exceeded:true`) — the reference never turns
/// a completed evaluation into a cancellation.
fn envelope_for(
    engine: &mut Engine,
    opts: &Options,
    script: &ScriptSource,
    execution: Execution,
) -> (Json, i32) {
    let Execution {
        outcome,
        elapsed,
        elapsed_ms,
    } = execution;
    match outcome {
        Ok(value) => (
            success_envelope(engine, &value, opts, script, elapsed, elapsed_ms),
            0,
        ),
        Err(err) if err.code == "Cancelled" => (
            cancelled_envelope(engine, opts, script, &err, elapsed, elapsed_ms),
            1,
        ),
        Err(err) => (
            failure_envelope(engine, &err, script, elapsed, elapsed_ms, opts),
            1,
        ),
    }
}

fn print_json(env: &Json) {
    let mut out = std::io::stdout();
    let _ = writeln!(out, "{}", env);
    let _ = out.flush();
}

/// A failure raised before any engine exists: no diagnostics, no statement timing, no output.
/// The elapsed pair is present and zero, exactly as the reference publishes it on this path.
fn pre_engine_error_envelope(code: &str, category: &str, message: String) -> Json {
    let mut fields = preamble(false);
    fields.push(("code", Json::Str(code.to_string())));
    fields.push(("category", Json::Str(category.to_string())));
    fields.push(("message", Json::Str(message)));
    fields.push(("recoverable", Json::Bool(true)));
    fields.push(("diagnostics", Json::Arr(Vec::new())));
    let (elapsed_text, elapsed_time) = elapsed_fields(std::time::Duration::ZERO);
    fields.push(("elapsed", Json::Str(elapsed_text)));
    fields.push(("elapsedTime", elapsed_time));
    fields.push(("timings", Json::Arr(Vec::new())));
    fields.push(("output", Json::Arr(Vec::new())));
    Json::obj(fields)
}

/// Writes the pre-engine failure in the selected mode and exits with the documented code 1.
fn emit_pre_engine_error(opts: &Options, code: &str, category: &str, message: String) -> ! {
    match opts.mode {
        Mode::Json => {
            print_json(&pre_engine_error_envelope(code, category, message));
        }
        Mode::Text => {
            eprintln!("Error [{code}/{category}]: {message}");
        }
    }
    std::process::exit(1);
}

fn preamble(ok: bool) -> Vec<(&'static str, Json)> {
    vec![
        ("protocolVersion", Json::Num(PROTOCOL_VERSION.to_string())),
        (
            "symbolicFormatVersion",
            Json::Str(SYMBOLIC_FORMAT_VERSION.to_string()),
        ),
        ("mathIrVersion", Json::Num(MATH_IR_VERSION.to_string())),
        ("ok", Json::Bool(ok)),
    ]
}

/// One variable in scope: its name, kind, display form and the SAME structured projection the
/// result carries.
fn variable_json(
    engine: &Engine,
    name: &str,
    value: &lovelace_lang::Value,
    budget: Option<usize>,
) -> Json {
    Json::obj(vec![
        ("name", Json::Str(name.to_string())),
        ("kind", Json::Str(value.kind_name().to_string())),
        ("display", Json::Str(engine.format(value))),
        (
            "structured",
            structured::project_value(engine, value, budget),
        ),
    ])
}

/// A duration as a unit-scaled value plus its unit, so an agent never parses a suffix.
fn duration_json(d: std::time::Duration) -> Json {
    let (value, unit) = scale_duration(d);
    Json::obj(vec![
        ("value", Json::Num(value)),
        ("unit", Json::Str(unit.to_string())),
    ])
}

/// The one unit selector behind both the human duration string and the structured pair.
///
/// It is the reference's `Lovelace.Suite.Timing.Scale`: the largest unit whose value is at
/// least one whole unit (ns, µs, ms, s, min, h), with a fractional value rounded to two
/// decimals and trailing zeros trimmed. One selector drives both published forms, so the human
/// `elapsed` string and the `elapsedTime` pair can never disagree about the scale.
fn scale_duration(d: std::time::Duration) -> (String, &'static str) {
    let total_nanoseconds = d.as_nanos() as f64;
    if total_nanoseconds < 1_000.0 {
        return (format!("{:.0}", total_nanoseconds), "ns");
    }
    let total_microseconds = total_nanoseconds / 1_000.0;
    if total_microseconds < 1_000.0 {
        return (format_two_decimals(total_microseconds), "µs");
    }
    let total_milliseconds = total_nanoseconds / 1_000_000.0;
    if total_milliseconds < 1_000.0 {
        return (format_two_decimals(total_milliseconds), "ms");
    }
    let total_seconds = total_nanoseconds / 1_000_000_000.0;
    if total_seconds < 60.0 {
        return (format_two_decimals(total_seconds), "s");
    }
    let total_minutes = total_seconds / 60.0;
    if total_minutes < 60.0 {
        return (format_two_decimals(total_minutes), "min");
    }
    (format_two_decimals(total_seconds / 3_600.0), "h")
}

/// Two decimals at most, trailing zeros trimmed — the reference's `"0.##"` format
/// (`Timing.FormatValue`).
fn format_two_decimals(value: f64) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    let mut text = format!("{rounded:.2}");
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
    text
}

/// The human `elapsed` string and the structured `elapsedTime` pair, from ONE unit selector.
fn elapsed_fields(d: std::time::Duration) -> (String, Json) {
    let (value, unit) = scale_duration(d);
    (format!("{value} {unit}"), duration_json(d))
}

/// One timed statement's wire entry.
fn timings_json(engine: &Engine) -> Json {
    Json::Arr(
        engine
            .operation_timings()
            .iter()
            .map(|t| {
                Json::obj(vec![
                    ("position", Json::Num(t.position.to_string())),
                    ("elapsed", duration_json(std::time::Duration::ZERO)),
                    ("resultKind", Json::Str(t.result_kind.clone())),
                    ("hasOutput", Json::Bool(t.has_output)),
                ])
            })
            .collect(),
    )
}

/// The registry: every builtin descriptor plus any user-defined function the run registered.
fn functions_json(engine: &Engine) -> Json {
    let mut entries: Vec<(String, Json)> = registry::BUILTINS
        .iter()
        .map(|info| (info.name.to_string(), structured::descriptor_json(info)))
        .collect();
    for (name, params) in engine.functions() {
        if registry::find(&name).is_none() {
            entries.push((
                name.clone(),
                Json::obj(vec![
                    ("name", Json::Str(name.clone())),
                    ("parameters", Json::str_array(params.iter().cloned())),
                    ("minArity", Json::Num(params.len().to_string())),
                    ("parameterCount", Json::Num(params.len().to_string())),
                    ("variadic", Json::Bool(false)),
                    ("builtin", Json::Bool(false)),
                    ("category", Json::Str("Core".to_string())),
                ]),
            ));
        }
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    Json::Arr(entries.into_iter().map(|(_, json)| json).collect())
}

/// Every file the evaluation committed, as structured artifacts (the plot the engine wrote).
fn artifacts_json(engine: &Engine) -> Json {
    match engine.last_plot() {
        Some(capture) => {
            let bytes = std::fs::metadata(&capture.path).ok().map(|m| m.len());
            let mut fields = vec![
                ("kind", Json::Str("plot".to_string())),
                ("path", Json::Str(capture.path.clone())),
            ];
            if !capture.title.is_empty() {
                fields.push(("title", Json::Str(capture.title.clone())));
            }
            if let Some(size) = bytes {
                fields.push(("bytes", Json::Num(size.to_string())));
            }
            Json::Arr(vec![Json::obj(fields)])
        }
        None => Json::Arr(Vec::new()),
    }
}

/// The deadline ledger, or None when no budget was requested.
fn cancellation_json(budget: u64, elapsed_ms: f64, stopped: bool) -> Json {
    let excess = (elapsed_ms - budget as f64).max(0.0);
    Json::obj(vec![
        ("budgetMs", Json::Num(budget.to_string())),
        ("elapsedMs", Json::Num(format_ms(elapsed_ms))),
        ("stopped", Json::Bool(stopped)),
        ("exceeded", Json::Bool(excess > 0.0)),
        ("excessMs", Json::Num(format_ms(excess))),
    ])
}

/// The diagnostic that names an exceeded deadline, so a consumer reading only diagnostics cannot
/// mistake an overrun for a normal run.
fn overrun_diagnostic(
    budget: u64,
    elapsed_ms: f64,
    position: usize,
    script: &ScriptSource,
    stopped: bool,
    completed: bool,
) -> Json {
    let excess = (elapsed_ms - budget as f64).max(0.0);
    let outcome = if stopped {
        "the deadline was observed, but only after the excess had already been spent"
    } else if completed {
        "the evaluation completed anyway"
    } else {
        "the evaluation then failed for another reason"
    };
    script
        .diagnostic(
            format!(
                "cancellation deadline exceeded: --cancel-after {} ms, elapsed {} ms ({} ms over budget); {}.",
                budget,
                format_ms(elapsed_ms),
                format_ms(excess),
                outcome
            ),
            position,
        )
        .to_json()
}

fn success_envelope(
    engine: &mut Engine,
    value: &lovelace_lang::Value,
    opts: &Options,
    script: &ScriptSource,
    elapsed: std::time::Duration,
    elapsed_ms: f64,
) -> Json {
    let mut fields = preamble(true);
    fields.push(("revision", Json::Num(engine.revision().to_string())));
    let budget = opts.print_budget;
    // A Void result is an absent value: the field is omitted rather than published as a value.
    if !matches!(value, lovelace_lang::Value::Void) {
        fields.push((
            "result",
            Json::obj(vec![
                ("kind", Json::Str(value.kind_name().to_string())),
                ("display", Json::Str(engine.format(value))),
                ("typed", Json::Str(engine.format_typed(value))),
                (
                    "structured",
                    structured::project_value(engine, value, budget),
                ),
            ]),
        ));
    }
    let output = engine.take_output();
    fields.push(("output", Json::str_array(output)));
    fields.push((
        "variables",
        if opts.omit_variables {
            Json::Arr(Vec::new())
        } else {
            Json::Arr(
                engine
                    .variable_values()
                    .iter()
                    .map(|(name, v)| variable_json(engine, name, v, budget))
                    .collect(),
            )
        },
    ));
    fields.push((
        "functions",
        if opts.omit_functions {
            Json::Arr(Vec::new())
        } else {
            functions_json(engine)
        },
    ));
    if let Some(p) = engine.last_plot() {
        fields.push((
            "plot",
            Json::obj(vec![
                ("path", Json::Str(p.path.clone())),
                ("title", Json::Str(p.title.clone())),
                ("svg", Json::Str(p.svg.clone())),
            ]),
        ));
    }
    let (elapsed_text, elapsed_time) = elapsed_fields(elapsed);
    fields.push(("elapsed", Json::Str(elapsed_text)));
    fields.push(("elapsedTime", elapsed_time));
    fields.push(("timings", timings_json(engine)));
    if let Some(budget_ms) = opts.cancel_after {
        fields.push((
            "cancellation",
            cancellation_json(budget_ms, elapsed_ms, false),
        ));
        if (elapsed_ms - budget_ms as f64) > 0.0 {
            // The position is the last top-level statement that ran — the statement the deadline
            // passed inside — translated by the same rule every other published position uses.
            let position = engine
                .operation_timings()
                .last()
                .map(|timing| timing.position)
                .unwrap_or(0);
            fields.push((
                "diagnostics",
                Json::Arr(vec![overrun_diagnostic(
                    budget_ms, elapsed_ms, position, script, false, true,
                )]),
            ));
        }
    }
    fields.push(("artifacts", artifacts_json(engine)));
    Json::obj(fields)
}

/// A run a kernel stopped at the caller's deadline: the cancelled envelope, with the partial
/// state the statements that returned had committed.
///
/// The diagnostics arm is the reference's: the engine clears its own diagnostics on a cancellation
/// (they describe a defect the caller did not commit), and the runner adds the overrun diagnostic
/// ONLY when the stop landed past the budget. A prompt stop that measured inside its budget
/// therefore publishes an empty array — the ledger already says the deadline is what ended it.
fn cancelled_envelope(
    engine: &mut Engine,
    opts: &Options,
    script: &ScriptSource,
    error: &lovelace_lang::LangError,
    elapsed: std::time::Duration,
    elapsed_ms: f64,
) -> Json {
    let budget = opts.cancel_after.unwrap_or(0);
    let mut fields = preamble(false);
    fields.push(("code", Json::Str("Cancelled".to_string())));
    fields.push(("category", Json::Str("BudgetExceeded".to_string())));
    fields.push(("message", Json::Str(error.message.clone())));
    fields.push(("recoverable", Json::Bool(true)));
    let diagnostics = if (elapsed_ms - budget as f64) > 0.0 {
        // The position is the statement the deadline passed inside (the engine recorded it on the
        // cancellation), line/column its translation into the caller's text.
        // The reference positions this diagnostic at the last entry of timings[]; when its
        // deadline is observed at a statement boundary that is the last COMPLETED statement
        // (measured: RUN-010, position 0). A kernel stop always lands inside a statement, so the
        // completed prefix is what this runner can publish without inventing a position the
        // caller's text cannot be sliced at; the statement it *was* in is the last timings entry.
        vec![overrun_diagnostic(
            budget,
            elapsed_ms,
            engine.last_completed_position(),
            script,
            true,
            false,
        )]
    } else {
        Vec::new()
    };
    fields.push(("diagnostics", Json::Arr(diagnostics)));
    let (elapsed_text, elapsed_time) = elapsed_fields(elapsed);
    fields.push(("elapsed", Json::Str(elapsed_text)));
    fields.push(("elapsedTime", elapsed_time));
    fields.push(("timings", timings_json(engine)));
    // Invariant 1 is not scoped to a successful run: the lines the script printed before the stop
    // cross in the documented output array, and the cancelled envelope keeps the partialOutput pair
    // it has always published (the SAME lines, so the two cannot disagree).
    let committed = engine.take_output();
    fields.push(("output", Json::str_array(committed.clone())));
    fields.push(("partialOutput", Json::str_array(committed)));
    fields.push((
        "partialVariables",
        Json::Arr(
            engine
                .variable_values()
                .iter()
                .map(|(name, v)| variable_json(engine, name, v, opts.print_budget))
                .collect(),
        ),
    ));
    fields.push(("cancellation", cancellation_json(budget, elapsed_ms, true)));
    fields.push(("artifacts", artifacts_json(engine)));
    Json::obj(fields)
}

fn failure_envelope(
    engine: &mut Engine,
    err: &lovelace_lang::LangError,
    script: &ScriptSource,
    elapsed: std::time::Duration,
    elapsed_ms: f64,
    opts: &Options,
) -> Json {
    // A failure envelope publishes no revision: nothing committed is versioned by a run that
    // failed, and the reference's error shape does not carry the field.
    let mut fields = preamble(false);
    fields.push(("code", Json::Str(err.code.to_string())));
    fields.push(("category", Json::Str(err.category.to_string())));
    fields.push(("message", Json::Str(err.message.clone())));
    // An internal invariant failure is the one class the caller cannot recover from; every
    // classified user-level failure can be repaired and retried.
    fields.push((
        "recoverable",
        Json::Bool(err.category != "InternalInvariantFailure"),
    ));
    let mut diagnostics = vec![
        script
            .diagnostic(err.message.clone(), err.position)
            .to_json(),
    ];
    if let Some(budget) = opts.cancel_after {
        if (elapsed_ms - budget as f64) > 0.0 {
            diagnostics.push(overrun_diagnostic(
                budget,
                elapsed_ms,
                err.position,
                script,
                false,
                false,
            ));
        }
    }
    fields.push(("diagnostics", Json::Arr(diagnostics)));
    let (elapsed_text, elapsed_time) = elapsed_fields(elapsed);
    fields.push(("elapsed", Json::Str(elapsed_text)));
    fields.push(("elapsedTime", elapsed_time));
    fields.push(("timings", timings_json(engine)));
    let output = engine.take_output();
    fields.push(("output", Json::str_array(output)));
    // The committed variable table is published on every failure: the statements that returned
    // before the failure committed their bindings, and an agent repairing a program needs them.
    fields.push((
        "partialVariables",
        Json::Arr(
            engine
                .variable_values()
                .iter()
                .map(|(name, v)| variable_json(engine, name, v, opts.print_budget))
                .collect(),
        ),
    ));
    if let Some(budget) = opts.cancel_after {
        fields.push(("cancellation", cancellation_json(budget, elapsed_ms, false)));
    }
    // The registry's answer to an unresolved name, and the declared signature of a call-site
    // refusal: each is present only for the failure that has it, never as a placeholder.
    if !err.suggestions.is_empty() {
        fields.push(("suggestions", Json::str_array(err.suggestions.clone())));
    }
    if !err.hints.is_empty() {
        fields.push(("hints", Json::str_array(err.hints.clone())));
    }
    if let Some(size) = err.registry_size {
        fields.push(("registrySize", Json::Num(size.to_string())));
    }
    if let Some(signature) = &err.signature {
        fields.push(("signature", signature_json(signature)));
    }
    fields.push(("artifacts", artifacts_json(engine)));
    Json::obj(fields)
}

/// The call-site signature as the wire publishes it: every field is omitted when the failure does
/// not have it.
fn signature_json(signature: &lovelace_lang::ErrorSignature) -> Json {
    let mut fields: Vec<(&str, Json)> = vec![("function", Json::Str(signature.function.clone()))];
    if let Some(parameters) = &signature.parameters {
        fields.push(("parameters", Json::str_array(parameters.clone())));
    }
    if let Some(min) = signature.expected_min {
        fields.push(("expectedMin", Json::Num(min.to_string())));
    }
    if let Some(max) = signature.expected_max {
        fields.push(("expectedMax", Json::Num(max.to_string())));
    }
    if let Some(variadic) = signature.variadic {
        fields.push(("variadic", Json::Bool(variadic)));
    }
    if let Some(actual) = signature.actual {
        fields.push(("actual", Json::Num(actual.to_string())));
    }
    if let Some(position) = signature.position {
        fields.push(("position", Json::Num(position.to_string())));
    }
    if let Some(parameter) = &signature.parameter {
        fields.push(("parameter", Json::Str(parameter.clone())));
    }
    if let Some(expected) = &signature.expected_shape {
        fields.push(("expectedShape", Json::Str(expected.clone())));
    }
    if let Some(arrived) = &signature.arrived_kind {
        fields.push(("arrivedKind", Json::Str(arrived.clone())));
    }
    Json::obj(fields)
}

/// Answers --functions / --function from the static registry without evaluating anything.
fn registry_query(opts: &Options) {
    if let Some(name) = &opts.describe_function {
        if let Some(info) = registry::find(name) {
            let mut fields = preamble(true);
            fields.push(("function", structured::descriptor_json(info)));
            fields.push(("total", Json::Num("1".to_string())));
            fields.push(("returned", Json::Num("1".to_string())));
            fields.push(("truncated", Json::Bool(false)));
            print_json(&Json::obj(fields));
        } else {
            let suggestions = registry::suggest(name);
            let hints = registry::hints(name);
            let mut message = format!("Unknown function '{name}'.");
            if suggestions.len() == 1 {
                message.push_str(&format!(" Did you mean '{}'?", suggestions[0]));
            } else if suggestions.len() > 1 {
                let quoted: Vec<String> = suggestions.iter().map(|s| format!("'{s}'")).collect();
                message.push_str(&format!(" Did you mean one of: {}?", quoted.join(", ")));
            }
            for hint in &hints {
                message.push(' ');
                message.push_str(hint);
            }
            let mut fields = preamble(false);
            fields.push(("code", Json::Str("UnknownFunction".to_string())));
            fields.push(("category", Json::Str("UnsupportedOperation".to_string())));
            fields.push(("message", Json::Str(message)));
            fields.push(("recoverable", Json::Bool(true)));
            fields.push(("diagnostics", Json::Arr(Vec::new())));
            let (elapsed_text, elapsed_time) = elapsed_fields(std::time::Duration::ZERO);
            fields.push(("elapsed", Json::Str(elapsed_text)));
            fields.push(("elapsedTime", elapsed_time));
            fields.push(("timings", Json::Arr(Vec::new())));
            fields.push(("output", Json::Arr(Vec::new())));
            fields.push(("partialVariables", Json::Arr(Vec::new())));
            if !suggestions.is_empty() {
                fields.push(("suggestions", Json::str_array(suggestions)));
            }
            if !hints.is_empty() {
                fields.push(("hints", Json::str_array(hints)));
            }
            fields.push((
                "registrySize",
                Json::Num(registry::BUILTINS.len().to_string()),
            ));
            fields.push(("artifacts", Json::Arr(Vec::new())));
            print_json(&Json::obj(fields));
            std::process::exit(1);
        }
        return;
    }
    let pattern = opts.function_pattern.clone().unwrap_or_default();
    let matches = registry::matching(&pattern);
    let total = matches.len();
    let page: Vec<Json> = matches
        .iter()
        .take(opts.function_limit)
        .map(|info| structured::listing_json(info))
        .collect();
    let returned = page.len();
    let mut fields = preamble(true);
    fields.push(("pattern", Json::Str(pattern)));
    fields.push(("functions", Json::Arr(page)));
    fields.push(("total", Json::Num(total.to_string())));
    fields.push(("returned", Json::Num(returned.to_string())));
    fields.push(("truncated", Json::Bool(returned < total)));
    print_json(&Json::obj(fields));
}

/// The `--stdin` surface's text: the characters the reader produced. The process's standard
/// input is UTF-8 (the reference's Console.In encoding), and a leading byte-order mark is kept as
/// the U+FEFF character it is so the engine can strip it and map positions back.
fn decode_stdin_bytes(bytes: Vec<u8>) -> Result<String, String> {
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

/// Decodes a script file's BYTES into the caller's text, keeping a leading byte-order mark as
/// the U+FEFF character it is.
///
/// This is the reference's `ScriptText.Decode`: the encoding is detected from a byte-order
/// mark exactly as `File.ReadAllText` detects it (UTF-8, UTF-16 LE/BE or UTF-32 LE/BE, and
/// UTF-8 when there is no mark), but the mark is NOT consumed — all three runner surfaces
/// therefore receive the same characters for byte-identical input, and every published
/// position names the caller's own text. The ENGINE is the layer that drops the mark, because
/// its tokenizer does not accept U+FEFF.
fn decode_script_bytes(bytes: &[u8]) -> Result<String, String> {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return std::str::from_utf8(&bytes[3..])
            .map(|text| format!("﻿{text}"))
            .map_err(|e| e.to_string());
    }
    if bytes.starts_with(&[0xFF, 0xFE, 0x00, 0x00]) {
        return decode_utf32(&bytes[4..], true).map(|text| format!("﻿{text}"));
    }
    if bytes.starts_with(&[0x00, 0x00, 0xFE, 0xFF]) {
        return decode_utf32(&bytes[4..], false).map(|text| format!("﻿{text}"));
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        return decode_utf16(&bytes[2..], false).map(|text| format!("﻿{text}"));
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return decode_utf16(&bytes[2..], true).map(|text| format!("﻿{text}"));
    }
    std::str::from_utf8(bytes)
        .map(str::to_string)
        .map_err(|e| e.to_string())
}

fn decode_utf16(bytes: &[u8], little_endian: bool) -> Result<String, String> {
    if bytes.len() % 2 != 0 {
        return Err("the UTF-16 text has an odd byte count".to_string());
    }
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| {
            if little_endian {
                u16::from_le_bytes([pair[0], pair[1]])
            } else {
                u16::from_be_bytes([pair[0], pair[1]])
            }
        })
        .collect();
    String::from_utf16(&units).map_err(|e| e.to_string())
}

fn decode_utf32(bytes: &[u8], little_endian: bool) -> Result<String, String> {
    if bytes.len() % 4 != 0 {
        return Err("the UTF-32 text has a byte count that is not a multiple of four".to_string());
    }
    let mut text = String::new();
    for chunk in bytes.chunks_exact(4) {
        let code = if little_endian {
            u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
        } else {
            u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
        };
        let ch = char::from_u32(code)
            .ok_or_else(|| format!("the UTF-32 text carries an invalid code point {code}"))?;
        text.push(ch);
    }
    Ok(text)
}

// The record-schema registry is an additive surface: the wire envelope is unchanged, so only the
// record_schema_batch ports consume it today. Kept outside cfg(test) so it is production code.
#[allow(dead_code)]
mod schemas;

// The host-side script source: the caller text, its normalised form, the map back, and the source
// identity a diagnostic names. Additive - the wire envelope is unchanged; the runner uses the
// diagnostic builder, so a failure is always paired with the surface it came from. The normaliser
// and engine-offset map are the host-facing half of the surface and have no wire consumer yet.
#[allow(dead_code)]
mod script_source;

#[cfg(test)]
mod parity_batch;

#[cfg(test)]
mod wire_batch;

#[cfg(test)]
mod golden_batch;

#[cfg(test)]
mod pretty_batch;

#[cfg(test)]
mod enum_batch;

#[cfg(test)]
mod dx_batch;

#[cfg(test)]
mod suite_batch;

#[cfg(test)]
mod error_taxonomy_batch;

#[cfg(test)]
mod cancellation_batch;

#[cfg(test)]
mod round19_depth_batch;

#[cfg(test)]
mod round19_run_batch;

#[cfg(test)]
mod dsp_round21;

#[cfg(test)]
mod round27_run_batch;

#[cfg(test)]
mod round28_run_batch;

#[cfg(test)]
mod round29_run_batch;

#[cfg(test)]
mod round30_run_batch;

#[cfg(test)]
mod round35_run_batch;

#[cfg(test)]
mod round45_final_sweep;

#[cfg(test)]
mod round50_resweep;

#[cfg(test)]
mod record_schema_batch;

/// A millisecond count as the wire's invariant number text (up to three decimals).
fn format_ms(ms: f64) -> String {
    let text = format!("{:.3}", ms);
    let trimmed = text.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}
