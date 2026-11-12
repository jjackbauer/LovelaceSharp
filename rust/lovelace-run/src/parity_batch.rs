//! Parity ports of the C# runner's wire-contract tests (Lovelace.Run.Tests).
//!
//! These tests drive the runner's OWN envelope construction in-process: \`run\` below parses the
//! arguments, evaluates through \`Engine\`, and builds the same JSON document \`main\` prints. The C#
//! assertion is the contract; each test repeats it with the same concrete input.

use super::*;
use lovelace_json::Json;

pub struct RunResult {
    pub exit: i32,
    pub stdout: String,
    pub env: Json,
}

fn args_owned(args: &[&str]) -> Vec<String> {
    args.iter().map(|s| (*s).to_string()).collect()
}

pub fn plot_dir() -> String {
    let dir = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../target/tmp/parity-run-tests"
    );
    let _ = std::fs::create_dir_all(dir);
    dir.to_string()
}

/// The published envelope for one argv, built by the same functions \`main\` uses.
pub fn run_args(args: &[&str]) -> RunResult {
    let owned = args_owned(args);
    let opts = parse_args(&owned).expect("test argv must parse");
    if opts.list_functions || opts.describe_function.is_some() {
        let env = registry_query_envelope(&opts);
        let ok = matches!(env.get("ok"), Some(Json::Bool(true)));
        return RunResult {
            exit: if ok { 0 } else { 1 },
            stdout: env.to_string(),
            env,
        };
    }
    let source = match &opts.source_file {
        // The SAME decoder main uses: a leading byte-order mark is kept as U+FEFF (the engine
        // strips it), and UTF-16/UTF-32 files decode from their own mark.
        Some(path) => match std::fs::read(path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| decode_script_bytes(&bytes))
        {
            Ok(text) => text,
            Err(e) => {
                let env = pre_engine_error_envelope(
                    "FileReadError",
                    "TypeMismatch",
                    format!("Cannot read script file '{path}': {e}"),
                );
                return RunResult {
                    exit: 1,
                    stdout: env.to_string(),
                    env,
                };
            }
        },
        None => opts.source.clone().unwrap_or_default(),
    };
    let mut engine = Engine::new();
    if let Some(dir) = &opts.plot_dir {
        engine.set_plot_dir(dir);
        // Mirror main: a directory the process cannot create crosses as
        // PlotDirectoryError/TypeMismatch instead of being silently ignored.
        if let Err(e) = std::fs::create_dir_all(dir) {
            let env = pre_engine_error_envelope(
                "PlotDirectoryError",
                "TypeMismatch",
                format!("Cannot use plot directory '{dir}': {e}"),
            );
            return RunResult {
                exit: 1,
                stdout: env.to_string(),
                env,
            };
        }
    }
    if let Some(name) = &opts.plot_file {
        engine.set_plot_file(name);
    }
    // The engine run and the envelope are the SAME two functions `main` uses, so a wire
    // assertion here cannot pass on a flow the binary does not run. The script identity mirrors
    // the surface main would name for the same argv.
    let identity = match &opts.source_file {
        Some(path) => SourceIdentity::file(path.clone()),
        None => match opts.origin {
            SourceOrigin::Stdin => SourceIdentity::stdin(),
            _ => SourceIdentity::eval(),
        },
    };
    let script = ScriptSource::new(identity, source);
    let execution = execute(&mut engine, &opts, script.text());
    let (env, exit) = envelope_for(&mut engine, &opts, &script, execution);
    RunResult {
        exit,
        stdout: env.to_string(),
        env,
    }
}

/// The same JSON the production registry query prints, returned instead of printed.
fn registry_query_envelope(opts: &Options) -> Json {
    if let Some(name) = &opts.describe_function {
        if let Some(info) = registry::find(name) {
            let mut fields = preamble(true);
            fields.push(("function", structured::descriptor_json(info)));
            fields.push(("total", Json::Num("1".to_string())));
            fields.push(("returned", Json::Num("1".to_string())));
            fields.push(("truncated", Json::Bool(false)));
            return Json::obj(fields);
        }
        let mut fields = preamble(false);
        fields.push(("code", Json::Str("UnknownFunction".to_string())));
        fields.push(("category", Json::Str("UnsupportedOperation".to_string())));
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
        fields.push(("message", Json::Str(message)));
        fields.push(("recoverable", Json::Bool(true)));
        fields.push(("diagnostics", Json::Arr(Vec::new())));
        let (zero_value, zero_unit) = scale_duration(std::time::Duration::ZERO);
        fields.push(("elapsed", Json::Str(format!("{zero_value} {zero_unit}"))));
        fields.push(("elapsedTime", duration_json(std::time::Duration::ZERO)));
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
        return Json::obj(fields);
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
    Json::obj(fields)
}

pub fn get<'a>(j: &'a Json, key: &str) -> &'a Json {
    j.get(key)
        .unwrap_or_else(|| panic!("missing key '{key}' in {}", j))
}

pub fn opt<'a>(j: &'a Json, key: &str) -> Option<&'a Json> {
    j.get(key)
}

pub fn text(j: &Json) -> &str {
    j.as_str().unwrap_or_else(|| panic!("not a string: {}", j))
}

pub fn boolean(j: &Json) -> bool {
    match j {
        Json::Bool(b) => *b,
        other => panic!("not a bool: {}", other),
    }
}

pub fn number(j: &Json) -> i64 {
    match j {
        Json::Num(s) => s.parse().unwrap_or_else(|_| panic!("not an integer: {s}")),
        other => panic!("not a number: {}", other),
    }
}

pub fn array(j: &Json) -> &[Json] {
    j.as_arr().unwrap_or_else(|| panic!("not an array: {}", j))
}

pub fn record_field<'a>(record: &'a Json, name: &str) -> &'a Json {
    for field in array(get(record, "fields")) {
        if text(get(field, "name")) == name {
            return get(field, "value");
        }
    }
    panic!("record has no field '{name}': {}", record);
}

pub fn decimals(rendering: &str) -> usize {
    let bare = rendering.split(' ').next().unwrap_or(rendering);
    match bare.find('.') {
        Some(dot) => bare.len() - dot - 1,
        None => 0,
    }
}

pub fn envelope(script: &str, extra: &[&str]) -> RunResult {
    let mut args = vec!["--eval", script, "--omit-functions", "--omit-variables"];
    args.extend_from_slice(extra);
    run_args(&args)
}

fn matrix_case(builtin: &str, param: &str) {
    let row = MATRIX
        .iter()
        .find(|(b, p, _, _)| *b == builtin && *p == param)
        .unwrap_or_else(|| panic!("no matrix row {builtin}:{param}"));
    let template = row.2;
    for (literal, must) in row.3 {
        let script = template.replace("{0}", literal);
        let result = run_args(&[
            "--eval",
            &script,
            "--omit-functions",
            "--omit-variables",
            "--plot-dir",
            &plot_dir(),
            "--cancel-after",
            "30000",
        ]);
        let env = &result.env;
        let ok = boolean(get(env, "ok"));
        let where_ = format!("{script} [{builtin}:{param}={literal}]");
        if !ok {
            let code = opt(env, "code").map(text).unwrap_or("(absent)");
            let category = opt(env, "category").map(text).unwrap_or("(absent)");
            let message = opt(env, "message").map(text).unwrap_or("");
            let recoverable = opt(env, "recoverable").map(boolean).unwrap_or(false);
            assert!(
                code != "InternalError" && category != "InternalInvariantFailure",
                "{where_}: a caller-side argument mistake crossed as an INTERNAL failure \
                 (code={code}, category={category}, recoverable={recoverable}, message='{message}')"
            );
            assert!(
                recoverable,
                "{where_}: the refusal is not recoverable (code={code}, category={category}, message='{message}')"
            );
            assert!(
                *must != 1,
                "{where_}: this literal denotes an exact integer the call site accepts, but it was refused (code={code}, message='{message}')"
            );
            assert_eq!(result.exit, 1, "{where_}: a refusal must exit 1");
            if code == "Cancelled" {
                assert_eq!(
                    category, "BudgetExceeded",
                    "{where_}: a cancellation must carry category BudgetExceeded"
                );
                continue;
            }
            if code == "InvalidArgument" {
                assert_eq!(
                    category, "TypeMismatch",
                    "{where_}: an argument refusal must carry category TypeMismatch"
                );
                assert!(
                    message.starts_with(&format!("{builtin}()")),
                    "{where_}: the refusal must name the builtin as '{builtin}()'"
                );
                assert!(
                    !message.contains("(Parameter '"),
                    "{where_}: the refusal carries raw framework text"
                );
            }
            continue;
        }
        assert!(
            *must != 0,
            "{where_}: an argument that is not an exact integer was ACCEPTED — the silent-coercion class"
        );
        assert_eq!(result.exit, 0, "{where_}: an accepted call must exit 0");
    }
}

macro_rules! matrix_test {
    ($name:ident, $builtin:literal, $param:literal) => {
        #[test]
        fn $name() {
            matrix_case($builtin, $param);
        }
    };
}

macro_rules! matrix_rows {
    ($(($name:ident, $builtin:literal, $param:literal)),* $(,)?) => {
        $(matrix_test!($name, $builtin, $param);)*
        #[test]
        fn every_passing_matrix_row_is_exercised() {
            let count: usize = MATRIX.iter().map(|(_, _, _, cells)| cells.len()).sum();
            assert_eq!(count, 272, "the matrix table lost cells");
        }
    };
}

matrix_rows![
    (matrix_zeros_dims, "zeros", "dims"),
    (matrix_ones_dims, "ones", "dims"),
    (matrix_eye_rows, "eye", "rows"),
    (matrix_eye_cols, "eye", "cols"),
    (matrix_reshape_dims, "reshape", "dims"),
    (matrix_sum_axis, "sum", "axis"),
    (matrix_prod_axis, "prod", "axis"),
    (matrix_min_axis, "min", "axis"),
    (matrix_max_axis, "max", "axis"),
    (matrix_mean_axis, "mean", "axis"),
    (matrix_norm_axis, "norm", "axis"),
    (matrix_concat_axis, "concat", "axis"),
    (matrix_setprecision_digits, "setprecision", "digits"),
    (matrix_pi_digits, "pi", "digits"),
    (matrix_e_digits, "e", "digits"),
    (matrix_evalf_digits, "evalf", "digits"),
    (matrix_series_order, "series", "order"),
    (matrix_evalir_digits, "evalir", "digits"),
    (matrix_evalir_batch_digits, "evalir_batch", "digits"),
    (matrix_movingavg_w, "movingavg", "w"),
    (matrix_delay_k, "delay", "k"),
    (matrix_dft_n, "dft", "n"),
    (matrix_impulse_n, "impulse", "n"),
    (matrix_step_n, "step", "n"),
    (matrix_exponential_n, "exponential", "n"),
    (matrix_cosine_n, "cosine", "n"),
    (matrix_powerseries_n, "powerseries", "n"),
    (matrix_noise_n, "noise", "n"),
    (matrix_noise_seed, "noise", "seed"),
    (matrix_index_index, "index", "index"),
    (matrix_range_bound, "range", "bound"),
];

/// One matrix builtin row: name, parameter, expression and its case table.
type MatrixCase<'a> = (&'a str, &'a str, &'a str, &'a [(&'a str, i8)]);

const MATRIX: &[MatrixCase<'_>] = &[
    (
        "zeros",
        "dims",
        "zeros({0})",
        &[
            ("1", 1),
            ("2", 1),
            ("3", 1),
            ("2.5", 0),
            ("0.5", 0),
            ("2.9999", 0),
            ("(5/2)", 0),
            ("2.0", -1),
            ("0", -1),
            ("-1", -1),
        ],
    ),
    (
        "ones",
        "dims",
        "ones({0})",
        &[
            ("1", 1),
            ("2", 1),
            ("3", 1),
            ("2.5", 0),
            ("0.5", 0),
            ("2.9999", 0),
            ("(5/2)", 0),
            ("2.0", -1),
            ("0", -1),
            ("-1", -1),
        ],
    ),
    (
        "eye",
        "rows",
        "eye({0})",
        &[
            ("1", 1),
            ("2", 1),
            ("3", 1),
            ("2.5", 0),
            ("0.5", 0),
            ("2.9999", 0),
            ("(5/2)", 0),
            ("2.0", -1),
            ("0", -1),
            ("-1", -1),
            ("10^30", -1),
        ],
    ),
    (
        "eye",
        "cols",
        "eye(2, {0})",
        &[
            ("2.5", 0),
            ("0.5", 0),
            ("2.9999", 0),
            ("(5/2)", 0),
            ("2.0", -1),
            ("0", -1),
            ("-1", -1),
            ("10^30", -1),
            ("9223372036854775808", -1),
        ],
    ),
    (
        "reshape",
        "dims",
        "reshape([1, 2, 3, 4], {0})",
        &[
            ("4", 1),
            ("2.5", 0),
            ("0.5", 0),
            ("2.9999", 0),
            ("(5/2)", 0),
            ("2.0", -1),
            ("0", -1),
            ("-1", -1),
            ("10^30", -1),
            ("9223372036854775808", -1),
        ],
    ),
    (
        "sum",
        "axis",
        "sum([[1, 2], [3, 4]], {0})",
        &[
            ("0", 1),
            ("1", 1),
            ("2.5", 0),
            ("0.5", 0),
            ("2.9999", 0),
            ("(5/2)", 0),
            ("2.0", -1),
            ("0", -1),
            ("-1", -1),
            ("10^30", -1),
            ("9223372036854775808", -1),
        ],
    ),
    (
        "prod",
        "axis",
        "prod([1, 2, 3], {0})",
        &[
            ("0", 1),
            ("2.5", 0),
            ("0.5", 0),
            ("2.9999", 0),
            ("(5/2)", 0),
            ("2.0", -1),
            ("0", -1),
            ("-1", -1),
            ("10^30", -1),
            ("9223372036854775808", -1),
        ],
    ),
    (
        "min",
        "axis",
        "min([1, 2, 3], {0})",
        &[
            ("0", 1),
            ("2.5", 0),
            ("0.5", 0),
            ("2.9999", 0),
            ("(5/2)", 0),
            ("2.0", -1),
            ("0", -1),
            ("-1", -1),
            ("10^30", -1),
            ("9223372036854775808", -1),
        ],
    ),
    (
        "max",
        "axis",
        "max([1, 2, 3], {0})",
        &[
            ("0", 1),
            ("2.5", 0),
            ("0.5", 0),
            ("2.9999", 0),
            ("(5/2)", 0),
            ("2.0", -1),
            ("0", -1),
            ("-1", -1),
            ("10^30", -1),
            ("9223372036854775808", -1),
        ],
    ),
    (
        "mean",
        "axis",
        "mean([1, 2, 3], {0})",
        &[
            ("2.5", 0),
            ("0.5", 0),
            ("2.9999", 0),
            ("(5/2)", 0),
            ("2.0", -1),
            ("0", -1),
            ("-1", -1),
            ("10^30", -1),
            ("9223372036854775808", -1),
        ],
    ),
    (
        "norm",
        "axis",
        "norm([1, 2, 3], {0})",
        &[
            ("2.5", 0),
            ("0.5", 0),
            ("2.9999", 0),
            ("(5/2)", 0),
            ("2.0", -1),
            ("0", -1),
            ("-1", -1),
            ("10^30", -1),
            ("9223372036854775808", -1),
        ],
    ),
    (
        "concat",
        "axis",
        "concat([1, 2], [3, 4], {0})",
        &[
            ("0", 1),
            ("2.5", 0),
            ("0.5", 0),
            ("2.9999", 0),
            ("(5/2)", 0),
            ("2.0", -1),
            ("0", -1),
            ("-1", -1),
            ("10^30", -1),
            ("9223372036854775808", -1),
        ],
    ),
    (
        "setprecision",
        "digits",
        "setprecision({0}); 1",
        &[
            ("1", 1),
            ("2", 1),
            ("3", 1),
            ("2.5", 0),
            ("0.5", 0),
            ("2.9999", 0),
            ("(5/2)", 0),
            ("2.0", -1),
            ("0", -1),
            ("-1", -1),
        ],
    ),
    (
        "pi",
        "digits",
        "pi({0})",
        &[
            ("1", 1),
            ("2", 1),
            ("3", 1),
            ("2.5", 0),
            ("0.5", 0),
            ("2.9999", 0),
            ("(5/2)", 0),
            ("2.0", -1),
            ("0", -1),
            ("-1", -1),
        ],
    ),
    (
        "e",
        "digits",
        "e({0})",
        &[
            ("2.5", 0),
            ("0.5", 0),
            ("2.9999", 0),
            ("(5/2)", 0),
            ("2.0", -1),
            ("0", -1),
            ("-1", -1),
        ],
    ),
    (
        "evalf",
        "digits",
        "evalf(1/3, {0})",
        &[
            ("1", 1),
            ("2", 1),
            ("3", 1),
            ("2.5", 0),
            ("0.5", 0),
            ("2.9999", 0),
            ("(5/2)", 0),
            ("2.0", -1),
            ("0", -1),
            ("-1", -1),
        ],
    ),
    (
        "series",
        "order",
        "x = symbol(\"x\"); series(sin(x), x, 0, {0})",
        &[
            ("1", 1),
            ("2", 1),
            ("3", 1),
            ("2.5", 0),
            ("0.5", 0),
            ("2.9999", 0),
            ("(5/2)", 0),
            ("2.0", -1),
            ("0", -1),
            ("-1", -1),
        ],
    ),
    (
        "evalir",
        "digits",
        "x = symbol(\"x\"); evalir(compile(x*1/3, [x]), [1], {0})",
        &[
            ("1", 1),
            ("2", 1),
            ("3", 1),
            ("2.0", -1),
            ("0", -1),
            ("-1", -1),
        ],
    ),
    (
        "evalir_batch",
        "digits",
        "x = symbol(\"x\"); evalir_batch(compile(x*1/3, [x]), [1], {0})",
        &[
            ("1", 1),
            ("2", 1),
            ("3", 1),
            ("2.0", -1),
            ("0", -1),
            ("-1", -1),
        ],
    ),
    (
        "movingavg",
        "w",
        "movingavg([1, 2, 3, 4, 5], {0})",
        &[
            ("2.5", 0),
            ("0.5", 0),
            ("2.9999", 0),
            ("(5/2)", 0),
            ("2.0", -1),
            ("0", -1),
            ("-1", -1),
        ],
    ),
    (
        "delay",
        "k",
        "delay([1, 2, 3, 4], {0})",
        &[
            ("2.5", 0),
            ("0.5", 0),
            ("2.9999", 0),
            ("(5/2)", 0),
            ("2.0", -1),
            ("0", -1),
            ("-1", -1),
        ],
    ),
    (
        "dft",
        "n",
        "dft([1, 2, 3, 4], {0})",
        &[
            ("4", 1),
            ("2.5", 0),
            ("0.5", 0),
            ("2.9999", 0),
            ("(5/2)", 0),
            ("2.0", -1),
            ("0", -1),
            ("-1", -1),
        ],
    ),
    (
        "impulse",
        "n",
        "impulse({0})",
        &[
            ("2.5", 0),
            ("0.5", 0),
            ("2.9999", 0),
            ("(5/2)", 0),
            ("2.0", -1),
            ("0", -1),
            ("-1", -1),
        ],
    ),
    (
        "step",
        "n",
        "step({0})",
        &[
            ("2.5", 0),
            ("0.5", 0),
            ("2.9999", 0),
            ("(5/2)", 0),
            ("2.0", -1),
            ("0", -1),
            ("-1", -1),
        ],
    ),
    (
        "exponential",
        "n",
        "exponential(0.5, {0})",
        &[
            ("2.5", 0),
            ("0.5", 0),
            ("2.9999", 0),
            ("(5/2)", 0),
            ("2.0", -1),
            ("0", -1),
            ("-1", -1),
        ],
    ),
    (
        "cosine",
        "n",
        "cosine(0.25, 0, {0})",
        &[
            ("2.5", 0),
            ("0.5", 0),
            ("2.9999", 0),
            ("(5/2)", 0),
            ("2.0", -1),
            ("0", -1),
            ("-1", -1),
        ],
    ),
    (
        "powerseries",
        "n",
        "powerseries(2, 0.5, {0})",
        &[
            ("2.5", 0),
            ("0.5", 0),
            ("2.9999", 0),
            ("(5/2)", 0),
            ("2.0", -1),
            ("0", -1),
            ("-1", -1),
        ],
    ),
    (
        "noise",
        "n",
        "noise(1, 0, {0})",
        &[
            ("2.5", 0),
            ("0.5", 0),
            ("2.9999", 0),
            ("(5/2)", 0),
            ("2.0", -1),
            ("0", -1),
            ("-1", -1),
        ],
    ),
    (
        "noise",
        "seed",
        "noise(1, 0, {0}, 2)",
        &[
            ("2.5", 0),
            ("0.5", 0),
            ("2.9999", 0),
            ("(5/2)", 0),
            ("2.0", -1),
            ("0", -1),
            ("-1", -1),
        ],
    ),
    (
        "index",
        "index",
        "[1, 2, 3][{0}]",
        &[
            ("0", 1),
            ("1", 1),
            ("2", 1),
            ("2.5", 0),
            ("0.5", 0),
            ("2.9999", 0),
            ("(5/2)", 0),
            ("2.0", -1),
            ("0", -1),
            ("-1", -1),
            ("10^30", -1),
            ("9223372036854775808", -1),
        ],
    ),
    (
        "range",
        "bound",
        "1..{0}",
        &[
            ("1", 1),
            ("2", 1),
            ("3", 1),
            ("2.5", 0),
            ("0.5", 0),
            ("2.9999", 0),
            ("(5/2)", 0),
            ("2.0", -1),
            ("0", -1),
            ("-1", -1),
            ("10^30", -1),
        ],
    ),
];
