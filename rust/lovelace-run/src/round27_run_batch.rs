//! Round-27 ports of the Run/Suite gaps at the WIRE surface.
//!
//! Every test drives the runner's own \`execute\`/\`envelope_for\` pair through
//! \`parity_batch::run_args\` (and the production decoders for the file/stdin surfaces), so a
//! wire assertion cannot pass on a flow the binary does not run.

use super::parity_batch::*;
use super::*;
use lovelace_json::Json;

fn with_large_stack<T: Send + 'static>(probe: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(probe)
        .expect("probe thread")
        .join()
        .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
}

fn run_script(script: &str) -> RunResult {
    let script = script.to_string();
    with_large_stack(move || {
        run_args(&[
            "--eval",
            &script,
            "--json",
            "--omit-functions",
            "--omit-variables",
        ])
    })
}

fn run_script_with_variables(script: &str) -> RunResult {
    let script = script.to_string();
    with_large_stack(move || run_args(&["--eval", &script, "--json", "--omit-functions"]))
}

fn run_args_of(args: &[&str]) -> RunResult {
    let args: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
    with_large_stack(move || {
        let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        run_args(&refs)
    })
}

fn text(json: &Json) -> String {
    match json {
        Json::Str(value) => value.clone(),
        other => other.to_string(),
    }
}

fn field<'a>(json: &'a Json, name: &str) -> &'a Json {
    match json {
        Json::Obj(fields) => fields
            .iter()
            .find(|(key, _)| key.as_str() == name)
            .map(|(_, value)| value)
            .unwrap_or_else(|| panic!("json has no field '{name}': {}", json)),
        other => panic!("not an object: {}", other),
    }
}

fn optional_field<'a>(json: &'a Json, name: &str) -> Option<&'a Json> {
    match json {
        Json::Obj(fields) => fields
            .iter()
            .find(|(key, _)| key.as_str() == name)
            .map(|(_, value)| value),
        _ => None,
    }
}

fn array(json: &Json) -> &[Json] {
    match json {
        Json::Arr(items) => items,
        other => panic!("not an array: {}", other),
    }
}

fn structured(result: &RunResult) -> &Json {
    field(field(&result.env, "result"), "structured")
}

/// A record's field value by name (the structured projection publishes a named-field array).
fn record_field<'a>(json: &'a Json, name: &str) -> &'a Json {
    for entry in array(field(json, "fields")) {
        if text(field(entry, "name")) == name {
            return field(entry, "value");
        }
    }
    panic!("record has no field '{name}': {}", json)
}

fn diagnostic_position(result: &RunResult) -> usize {
    text(field(
        &array(field(&result.env, "diagnostics"))[0],
        "position",
    ))
    .parse()
    .expect("a position")
}

fn timing_positions(result: &RunResult) -> Vec<usize> {
    array(field(&result.env, "timings"))
        .iter()
        .map(|timing| text(field(timing, "position")).parse().expect("a position"))
        .collect()
}

// ---------------------------------------------------------------------------
// Item 7 — elapsed and elapsedTime come from ONE unit selector (EnvelopeDurationTests)
// ---------------------------------------------------------------------------

const UNITS: [&str; 6] = ["ns", "µs", "ms", "s", "min", "h"];

/// The reference's invariant: the human string and the structured pair name the SAME unit and
/// the SAME value, on success and on failure alike.
fn assert_same_scale(result: &RunResult, what: &str) {
    let human = text(field(&result.env, "elapsed"));
    let structured = field(&result.env, "elapsedTime");
    let mut parts = human.split(' ');
    let value: f64 = parts.next().expect("a value").parse().expect("a number");
    let unit = parts.next().expect("a unit");
    assert_eq!(
        parts.next(),
        None,
        "{what}: '{human}' has more than two fields"
    );
    assert!(
        UNITS.contains(&unit),
        "{what}: '{human}' names an unknown unit"
    );
    assert_eq!(
        text(field(structured, "unit")),
        unit,
        "{what}: '{human}' vs {}",
        structured
    );
    let structured_value: f64 = text(field(structured, "value")).parse().expect("a number");
    assert_eq!(
        value, structured_value,
        "{what}: '{human}' vs {}",
        structured
    );
}

#[test]
fn success_and_error_envelopes_publish_one_scale() {
    let success = run_script("1+1");
    assert_eq!(success.exit, 0);
    assert_same_scale(&success, "success");

    let parse = run_script("1 +");
    assert_eq!(parse.exit, 1);
    assert_same_scale(&parse, "parse error");

    let failure = run_script("x = symbol(\"x\"); solve(x^2 + 1 == 0, x, integer)");
    assert_eq!(failure.exit, 1);
    assert_same_scale(&failure, "multi-statement error");
}

#[test]
fn a_sub_millisecond_run_uses_the_microsecond_scale_in_both_fields() {
    // A one-statement run answers far below a millisecond on this machine; whatever the magnitude,
    // the two published forms must name the same unit (the C# row that made this a finding).
    let result = run_script("det(1)");
    assert_eq!(result.exit, 1);
    assert_same_scale(&result, "sub-millisecond failure");
    let unit = text(field(field(&result.env, "elapsedTime"), "unit"));
    assert!(unit == "ns" || unit == "µs" || unit == "ms", "unit {unit}");
}

#[test]
fn a_pre_engine_failure_publishes_the_zero_pair_from_the_same_selector() {
    let missing = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../target/tmp/round27-does-not-exist.ls"
    );
    let result = run_args_of(&["--file", missing, "--omit-functions"]);
    assert_eq!(result.exit, 1);
    assert_eq!(text(field(&result.env, "code")), "FileReadError");
    assert_same_scale(&result, "file read error");
    let elapsed = text(field(&result.env, "elapsed"));
    assert_eq!(elapsed, "0 ns", "the zero pair must be the selector's own");
}

// ---------------------------------------------------------------------------
// Item 5 — the three surfaces report the SAME positions for the same text
// (SurfacePositionAgreementTests, SourcePositionTests.LeadingBom)
// ---------------------------------------------------------------------------

const SCRIPT: &str = "a = 1\nb = 2\ndet(1)";

fn surface_dir() -> std::path::PathBuf {
    let dir = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../target/tmp/round27"
    ));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn write_surface(name: &str, bytes: &[u8]) -> String {
    let path = surface_dir().join(name);
    std::fs::write(&path, bytes).expect("write the surface file");
    path.to_string_lossy().to_string()
}

#[test]
fn eval_file_and_stdin_agree_on_a_bom_prefixed_script() {
    // --eval: the argument string, BOM and all.
    let eval = run_script(&format!("\u{feff}{SCRIPT}"));
    assert_eq!(eval.exit, 1);
    assert_eq!(diagnostic_position(&eval), 13);
    assert_eq!(timing_positions(&eval), vec![1, 7, 13]);

    // --file: the caller's UTF-8 bytes, BOM kept as its own character.
    let mut utf8 = Vec::new();
    utf8.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    utf8.extend_from_slice(SCRIPT.as_bytes());
    let path = write_surface("bom-utf8.lv", &utf8);
    let file = run_args_of(&["--file", &path, "--omit-functions", "--omit-variables"]);
    assert_eq!(file.exit, 1);
    assert_eq!(diagnostic_position(&file), 13);
    assert_eq!(timing_positions(&file), vec![1, 7, 13]);

    // --stdin: the production decoder hands the engine the characters a pipe produced.
    let mut stdin_bytes = Vec::new();
    stdin_bytes.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    stdin_bytes.extend_from_slice(SCRIPT.as_bytes());
    let stdin_text = decode_stdin_bytes(stdin_bytes).expect("valid UTF-8 on stdin");
    let mut engine = Engine::new();
    let error = engine.eval(&stdin_text).expect_err("det(1) refuses");
    assert_eq!(error.position, 13);
    assert_eq!(
        engine
            .operation_timings()
            .iter()
            .map(|timing| timing.position)
            .collect::<Vec<_>>(),
        vec![1, 7, 13]
    );
}

#[test]
fn a_utf16_file_reports_the_same_positions_as_the_same_characters_on_eval() {
    // The file's MARK supplies the U+FEFF the decoder keeps, so the body is the script itself
    // (the C# test's Encoding.Unicode.GetBytes(source[1..])).
    // UTF-16 LE with its own byte-order mark: FF FE followed by the text's UTF-16 units.
    let mut utf16le = vec![0xFF, 0xFE];
    for unit in SCRIPT.encode_utf16() {
        utf16le.extend_from_slice(&unit.to_le_bytes());
    }
    let path = write_surface("bom-utf16le.lv", &utf16le);
    let file = run_args_of(&["--file", &path, "--omit-functions", "--omit-variables"]);
    assert_eq!(file.exit, 1);
    assert_eq!(diagnostic_position(&file), 13);
    assert_eq!(timing_positions(&file), vec![1, 7, 13]);

    // UTF-16 BE: FE FF followed by the same units, big-endian.
    let mut utf16be = vec![0xFE, 0xFF];
    for unit in SCRIPT.encode_utf16() {
        utf16be.extend_from_slice(&unit.to_be_bytes());
    }
    let path = write_surface("bom-utf16be.lv", &utf16be);
    let file = run_args_of(&["--file", &path, "--omit-functions", "--omit-variables"]);
    assert_eq!(file.exit, 1);
    assert_eq!(diagnostic_position(&file), 13);
    assert_eq!(timing_positions(&file), vec![1, 7, 13]);
}

#[test]
fn a_surrogate_pair_counts_two_utf16_units_before_the_failing_statement() {
    let result = run_script("s = \"\u{1F600}\"; det(1)");
    assert_eq!(result.exit, 1);
    assert_eq!(diagnostic_position(&result), 10);
    assert_eq!(timing_positions(&result), vec![0, 10]);
    // the consumer's slice: the reported position names the failing statement in the caller's text
    let source = "s = \"\u{1F600}\"; det(1)";
    let units: Vec<u16> = source.encode_utf16().collect();
    let start = diagnostic_position(&result);
    let slice: String = String::from_utf16(&units[start..start + 3]).expect("a UTF-16 slice");
    assert_eq!(slice, "det");
}

// ---------------------------------------------------------------------------
// Item 1 and item 2 at the wire (PluginIntegerArgumentWireTests,
// WideArgumentRefusalTests)
// ---------------------------------------------------------------------------

#[test]
fn evalir_digits_reach_the_wire_answer() {
    let two = run_script("x = symbol(\"x\"); print(evalir(compile(x*1/3, [x]), [1], 2))");
    assert_eq!(two.exit, 0, "{}", two.stdout);
    assert_eq!(
        array(field(&two.env, "output"))[0],
        Json::Str("0.33".to_string())
    );

    let forty = run_script("x = symbol(\"x\"); print(evalir(compile(x*1/3, [x]), [1], 40))");
    assert_eq!(forty.exit, 0, "{}", forty.stdout);
    assert_eq!(
        array(field(&forty.env, "output"))[0],
        Json::Str(format!("0.{}", "3".repeat(40)))
    );
}

#[test]
fn evalir_digit_refusals_cross_with_the_argument_pair() {
    let result = run_script("x = symbol(\"x\"); print(evalir(compile(x*1/3, [x]), [1], 2.9))");
    assert_eq!(result.exit, 1);
    assert_eq!(text(field(&result.env, "code")), "InvalidArgument");
    assert_eq!(text(field(&result.env, "category")), "TypeMismatch");
    assert_eq!(
        text(field(&result.env, "message")),
        "evalir(): argument 3 (digits) must be a Natural or Integer that denotes an exact integer; got 2.9."
    );
}

#[test]
fn wide_argument_refusals_cross_as_typed_argument_errors() {
    let zeros = run_script("zeros(-1)");
    assert_eq!(zeros.exit, 1);
    assert_eq!(text(field(&zeros.env, "code")), "InvalidArgument");
    assert_eq!(text(field(&zeros.env, "category")), "TypeMismatch");
    assert!(text(field(&zeros.env, "message")).contains("-1"));

    let series = run_script("x = symbol(\"x\"); series(sin(x), x, 0, -1)");
    assert_eq!(series.exit, 1);
    assert_eq!(text(field(&series.env, "code")), "InvalidArgument");
    assert!(text(field(&series.env, "message")).contains("-1"));

    let index = run_script("[1, 2, 3][10^30]");
    assert_eq!(index.exit, 1);
    assert_eq!(text(field(&index.env, "code")), "InvalidArgument");
    assert!(text(field(&index.env, "message")).contains("9223372036854775807"));
}

// ---------------------------------------------------------------------------
// Item 6 at the wire (EvalfDigitCapHonestyTests)
// ---------------------------------------------------------------------------

#[test]
fn an_over_cap_evalf_marks_the_result_and_the_variable() {
    let result = run_script_with_variables("setprecision(5000); evalf(sqrt(2), 5000)");
    assert_eq!(result.exit, 0, "{}", result.stdout);
    let structured = structured(&result);
    assert!(matches!(
        optional_field(structured, "truncated"),
        Some(Json::Bool(true))
    ));
    assert_eq!(text(field(structured, "truncationReason")), "digit-cap");
    assert_eq!(text(field(structured, "budget")), "1000");

    let variables = array(field(&result.env, "variables"));
    let underscore = variables
        .iter()
        .find(|variable| text(field(variable, "name")) == "_")
        .expect("the result binding");
    let projected = field(underscore, "structured");
    assert!(matches!(
        optional_field(projected, "truncated"),
        Some(Json::Bool(true))
    ));
    assert_eq!(text(field(projected, "budget")), "1000");
}

#[test]
fn a_request_at_the_cap_crosses_unmarked() {
    let result = run_script("evalf(pi, 1000)");
    assert_eq!(result.exit, 0, "{}", result.stdout);
    assert!(optional_field(structured(&result), "truncated").is_none());
}

// ---------------------------------------------------------------------------
// Item 4 and item 8 at the wire
// ---------------------------------------------------------------------------

#[test]
fn solve_observes_the_session_assumption_on_the_wire() {
    let result = run_script("x = symbol(\"x\"); assume(x > 5); solve(x == 3, x)");
    assert_eq!(result.exit, 0, "{}", result.stdout);
    let structured = structured(&result);
    assert_eq!(
        text(field(record_field(structured, "status"), "value")),
        "NoSolutions"
    );
    assert_eq!(
        array(field(record_field(structured, "solutions"), "elements")).len(),
        0
    );
}

#[test]
fn eye_and_the_reductions_cross_their_two_argument_forms() {
    let eye = run_script("eye(2,3)");
    assert_eq!(eye.exit, 0);
    assert_eq!(
        text(field(field(&eye.env, "result"), "display")),
        "[[1, 0, 0], [0, 1, 0]]"
    );

    let mean = run_script("mean([[1,2],[3,4]], 1)");
    assert_eq!(mean.exit, 0);
    assert_eq!(
        text(field(field(&mean.env, "result"), "display")),
        "[1.5, 3.5]"
    );

    let refusal = run_script("norm(1)");
    assert_eq!(refusal.exit, 1);
    assert_eq!(text(field(&refusal.env, "code")), "InvalidArgument");
    assert!(text(field(field(&refusal.env, "signature"), "expectedShape")) == "an array or vector");
}
