//! Round-28 ports of the Run/Suite wire rows the round-19 refusals left deferred and the
//! round-27 production fixes unblock.
//!
//! Every test drives the runner's own execute/envelope pair through `parity_batch::run_args`,
//! so a wire assertion cannot pass on a flow the binary does not run. A row whose C# assertion
//! the Rust wire does not meet is refused in the round report, never mapped to a weaker test.

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

fn array(json: &Json) -> &[Json] {
    match json {
        Json::Arr(items) => items,
        other => panic!("not an array: {}", other),
    }
}

fn boolean(json: &Json) -> bool {
    matches!(json, Json::Bool(true))
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

// ---------------------------------------------------------------------------
// IntegerArgumentBoundaryMatrixTests (48 deferred rows)
// ---------------------------------------------------------------------------

#[test]
fn deferred_boundary_cells_land_on_the_declared_outcome() {
    // (script, builtin, must_accept: true = must answer, false = must refuse, None = the
    // call site's own bound decides)
    let mut cells: Vec<(String, &'static str, Option<bool>)> = Vec::new();
    cells.push(("norm([1,2,3], 0)".to_string(), "norm", Some(true)));
    for literal in ["2.5", "0.5", "2.9999", "(5/2)"] {
        cells.push((
            format!("x = symbol(\"x\"); evalir(compile(x*1/3, [x]), [1], {literal})"),
            "evalir",
            Some(false),
        ));
        cells.push((
            format!("x = symbol(\"x\"); evalir_batch(compile(x*1/3, [x]), [1, 2], {literal})"),
            "evalir_batch",
            Some(false),
        ));
    }
    cells.push(("eye(9223372036854775808)".to_string(), "eye", None));
    cells.push(("1..9223372036854775808".to_string(), "range", None));
    for literal in ["1", "2", "3"] {
        cells.push((format!("eye(2, {literal})"), "eye", Some(true)));
    }
    cells.push(("mean([1,2,3], 0)".to_string(), "mean", Some(true)));

    for (script, builtin, must_accept) in cells {
        let result = run_script(&script);
        let ok = boolean(field(&result.env, "ok"));
        let where_ = format!("{script} [{builtin}]",);
        if ok {
            assert_ne!(
                must_accept,
                Some(false),
                "{where_}: an argument that is not an exact integer was ACCEPTED"
            );
            assert_eq!(result.exit, 0, "{where_}");
            continue;
        }
        assert_eq!(result.exit, 1, "{where_}: a refusal is exit 1");
        let code = text(field(&result.env, "code"));
        let category = text(field(&result.env, "category"));
        let message = text(field(&result.env, "message"));
        assert_ne!(code, "InternalError", "{where_}: {message}");
        assert_ne!(category, "InternalInvariantFailure", "{where_}: {message}");
        assert!(
            boolean(field(&result.env, "recoverable")),
            "{where_}: not recoverable: {message}"
        );
        assert_ne!(
            must_accept,
            Some(true),
            "{where_}: an accepted integer was refused: {message}"
        );
        if code == "InvalidArgument" {
            assert_eq!(category, "TypeMismatch", "{where_}: {message}");
            assert!(
                message.starts_with(&format!("{builtin}()")),
                "{where_}: {message}"
            );
            assert!(
                !message.contains("(Parameter '"),
                "{where_}: raw framework text: {message}"
            );
        }
    }
}

#[test]
fn deferred_inventory_templates_reach_the_argument_they_name() {
    // (builtin, template, anchor) — the C# Inventory/NonRegistryInventory rows whose
    // template-liveness cell was deferred.
    let rows: &[(&str, &str, &str)] = &[
        ("zeros", "zeros({0})", "a dimension"),
        ("ones", "ones({0})", "a dimension"),
        ("norm", "norm([1, 2, 3], {0})", "(axis)"),
        (
            "series",
            "x = symbol(\"x\"); series(sin(x), x, 0, {0})",
            "(order)",
        ),
        (
            "evalir",
            "x = symbol(\"x\"); evalir(compile(x*1/3, [x]), [1], {0})",
            "(digits)",
        ),
        (
            "evalir_batch",
            "x = symbol(\"x\"); evalir_batch(compile(x*1/3, [x]), [1, 2], {0})",
            "(digits)",
        ),
        ("eye", "eye({0})", "(rows)"),
        ("eye", "eye(2, {0})", "(cols)"),
        ("index", "[1, 2, 3][{0}]", "'index'"),
        ("reshape", "reshape([1, 2, 3, 4], {0})", "a dimension"),
        ("mean", "mean([1, 2, 3], {0})", "(axis)"),
    ];
    for (builtin, template, anchor) in rows {
        let script = template.replace("{0}", "\"sentinel\"");
        let result = run_script(&script);
        assert!(
            !boolean(field(&result.env, "ok")),
            "{script}: the sentinel was accepted, so the row is dead"
        );
        let code = text(field(&result.env, "code"));
        let category = text(field(&result.env, "category"));
        let message = text(field(&result.env, "message"));
        assert!(
            code == "InvalidArgument" || code == "InvalidOperation",
            "{script}: {code}/{category}: {message}"
        );
        assert!(
            category == "TypeMismatch" || category == "DomainError",
            "{script}: {code}/{category}: {message}"
        );
        assert!(
            message.starts_with(&format!("{builtin}()")),
            "{script}: {message}"
        );
        assert!(message.contains(anchor), "{script}: {message}");
    }
}

#[test]
fn accepted_evalir_digits_change_the_answer() {
    for (low, high) in [("1", "2"), ("2", "3"), ("1", "3")] {
        let script = |digits: &str| {
            format!("x = symbol(\"x\"); print(evalir(compile(x*1/3, [x]), [1], {digits}))")
        };
        let low_result = run_script(&script(low));
        let high_result = run_script(&script(high));
        assert!(
            boolean(field(&low_result.env, "ok")),
            "evalir(..., {low}) was refused"
        );
        assert!(
            boolean(field(&high_result.env, "ok")),
            "evalir(..., {high}) was refused"
        );
        let low_value = text(&array(field(&low_result.env, "output"))[0]);
        let high_value = text(&array(field(&high_result.env, "output"))[0]);
        assert_ne!(low_value, high_value, "{low} vs {high}");
        assert!(
            high_value.len() > low_value.len(),
            "a larger count publishes a longer answer: {low}='{low_value}', {high}='{high_value}'"
        );
    }
}

// ---------------------------------------------------------------------------
// WideArgumentRefusalTests (5 deferred rows)
// ---------------------------------------------------------------------------

fn assert_not_raw_clr(message: &str) {
    assert!(!message.contains("Int64"), "{message}");
    assert!(!message.to_lowercase().contains("overflow"), "{message}");
    assert!(!message.contains("Parameter 'order'"), "{message}");
}

#[test]
fn wide_argument_refusals_are_typed_argument_errors() {
    const HUGE: &str = "123456789012345678901234567890";
    const BOUND: &str = "9223372036854775807";
    let cases: Vec<(&str, String, &str)> = vec![
        ("index", format!("[1, 2, 3][{HUGE}]"), HUGE),
        (
            "series",
            "x = symbol(\"x\"); series(sin(x), x, 0, -1)".to_string(),
            "-1",
        ),
        (
            "series",
            format!("x = symbol(\"x\"); series(sin(x), x, 0, {HUGE})"),
            HUGE,
        ),
        ("zeros", "zeros(-1)".to_string(), "-1"),
        ("zeros", "zeros(2, -1)".to_string(), "argument 2"),
    ];
    for (what, script, needle) in cases {
        let result = run_script(&script);
        assert_eq!(result.exit, 1, "{what}: {script}");
        assert!(
            !boolean(field(&result.env, "ok")),
            "{what}: the script answered"
        );
        assert_eq!(
            text(field(&result.env, "code")),
            "InvalidArgument",
            "{what}: {script}"
        );
        assert_eq!(
            text(field(&result.env, "category")),
            "TypeMismatch",
            "{what}: {script}"
        );
        let message = text(field(&result.env, "message"));
        assert!(message.contains(needle), "{what}: {message}");
        if what == "index" {
            assert!(message.contains(BOUND), "{what}: {message}");
        }
        assert_not_raw_clr(&message);
    }
}

// ---------------------------------------------------------------------------
// EnvelopeDurationTests (5 deferred rows)
// ---------------------------------------------------------------------------

#[test]
fn multi_statement_error_envelope_carries_both_structural_durations() {
    let result = run_script("x = symbol(\"x\"); solve(x^2 + 1 == 0, x, integer)");
    assert_eq!(result.exit, 1);
    assert!(!boolean(field(&result.env, "ok")));
    assert_same_scale(&result, "multi-statement error");
    let timings = array(field(&result.env, "timings"));
    assert_eq!(timings.len(), 2);
    assert_eq!(text(field(&timings[0], "position")), "0");
    assert_eq!(text(field(&timings[0], "resultKind")), "Symbolic");
    assert_eq!(text(field(&timings[1], "position")), "17");
    assert_eq!(text(field(&timings[1], "resultKind")), "Void");
    for timing in timings {
        assert!(matches!(
            field(timing, "position"),
            Json::Num(_) | Json::Str(_)
        ));
        let unit = text(field(field(timing, "elapsed"), "unit"));
        assert!(UNITS.contains(&unit.as_str()), "{unit}");
        assert!(matches!(
            field(timing, "resultKind"),
            Json::Num(_) | Json::Str(_)
        ));
        assert!(matches!(field(timing, "hasOutput"), Json::Bool(_)));
    }
}

#[test]
fn single_statement_error_envelope_carries_one_timing() {
    let result = run_script("solve(symbol(\"x\")^2 + 1 == 0, symbol(\"x\"), integer)");
    assert_eq!(result.exit, 1);
    assert!(!boolean(field(&result.env, "ok")));
    assert_same_scale(&result, "single-statement error");
    let timings = array(field(&result.env, "timings"));
    assert_eq!(timings.len(), 1);
    assert_eq!(text(field(&timings[0], "position")), "0");
    assert!(UNITS.contains(&text(field(field(&timings[0], "elapsed"), "unit")).as_str()));
}

#[test]
fn error_envelope_without_any_statement_carries_an_empty_timings_array() {
    let result = run_script("1 +");
    assert_eq!(result.exit, 1);
    assert!(!boolean(field(&result.env, "ok")));
    assert_same_scale(&result, "parse error");
    assert!(array(field(&result.env, "timings")).is_empty());
}

#[test]
fn file_read_error_envelope_carries_the_same_structural_durations() {
    let missing = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../target/tmp/round28-does-not-exist.ls"
    );
    let result = run_args_of(&["--file", missing, "--omit-functions"]);
    assert_eq!(result.exit, 1);
    assert_eq!(text(field(&result.env, "code")), "FileReadError");
    assert_same_scale(&result, "file read error");
    assert!(array(field(&result.env, "timings")).is_empty());
}

#[test]
fn success_and_error_envelopes_carry_the_same_duration_keys() {
    let ok = run_script("x = symbol(\"x\"); solve(x^2 - 4 == 0, x)");
    let bad = run_script("x = symbol(\"x\"); solve(x^2 + 1 == 0, x, integer)");
    assert_eq!(ok.exit, 0);
    assert_eq!(bad.exit, 1);
    for envelope in [&ok.env, &bad.env] {
        assert!(matches!(
            field(envelope, "elapsed"),
            Json::Str(_) | Json::Num(_)
        ));
        assert!(matches!(field(envelope, "elapsedTime"), Json::Obj(_)));
        assert!(matches!(field(envelope, "timings"), Json::Arr(_)));
    }
    assert_eq!(
        array(field(&ok.env, "timings")).len(),
        array(field(&bad.env, "timings")).len()
    );
}

// ---------------------------------------------------------------------------
// SurfacePositionAgreementTests + SourcePositionTests.LeadingBom (5 deferred rows)
// ---------------------------------------------------------------------------

const FAILING: &str = "det(1)";

fn find_utf16(source: &str, needle: &str) -> usize {
    let units: Vec<u16> = source.encode_utf16().collect();
    let needle_units: Vec<u16> = needle.encode_utf16().collect();
    units
        .windows(needle_units.len())
        .position(|window| window == needle_units.as_slice())
        .expect("the needle is in the source")
}

fn surface_file(name: &str, bytes: &[u8]) -> String {
    let dir = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../target/tmp/round28"
    ));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(name);
    std::fs::write(&path, bytes).expect("write the surface file");
    path.to_string_lossy().to_string()
}

#[test]
fn every_surface_reports_the_same_positions_for_the_same_text() {
    let cases: &[(&str, usize, usize, usize)] = &[
        ("a = 1\nb = 2\ndet(1)", 0, 6, 12),
        ("\u{feff}a = 1\nb = 2\ndet(1)", 1, 7, 13),
        ("a = 1\r\nb = 2\r\ndet(1)", 0, 7, 14),
        ("\u{feff}a = 1\r\nb = 2\r\ndet(1)", 1, 8, 15),
    ];
    for (source, first, second, failing) in cases {
        assert_eq!(find_utf16(source, "a = 1"), *first);
        assert_eq!(find_utf16(source, "b = 2"), *second);
        assert_eq!(find_utf16(source, FAILING), *failing);

        let eval = run_script(source);
        assert_eq!(eval.exit, 1, "--eval {source:?}");
        assert_eq!(diagnostic_position(&eval), *failing, "--eval {source:?}");
        assert_eq!(
            timing_positions(&eval),
            vec![*first, *second, *failing],
            "--eval {source:?}"
        );

        let bytes = source.as_bytes();
        let path = surface_file(&format!("surface-{failing}.lv"), bytes);
        let file = run_args_of(&["--file", &path, "--omit-functions", "--omit-variables"]);
        assert_eq!(file.exit, 1, "--file {source:?}");
        assert_eq!(diagnostic_position(&file), *failing, "--file {source:?}");
        assert_eq!(
            timing_positions(&file),
            vec![*first, *second, *failing],
            "--file {source:?}"
        );

        // The C# --stdin surface hands the runner the characters a reader produced; the
        // production decoder is the same seam round 27 pinned.
        let decoded = decode_stdin_bytes(bytes.to_vec()).expect("valid UTF-8 on stdin");
        let mut engine = Engine::new();
        let error = engine.eval(&decoded).expect_err("det(1) refuses");
        assert_eq!(error.position, *failing, "--stdin {source:?}");
        assert_eq!(
            engine
                .operation_timings()
                .iter()
                .map(|timing| timing.position)
                .collect::<Vec<_>>(),
            vec![*first, *second, *failing],
            "--stdin {source:?}"
        );

        let line = text(field(&array(field(&eval.env, "diagnostics"))[0], "line"));
        let column = text(field(&array(field(&eval.env, "diagnostics"))[0], "column"));
        assert_eq!(
            (line.as_str(), column.as_str()),
            ("3", "1"),
            "--eval {source:?}"
        );
        let units: Vec<u16> = source.encode_utf16().collect();
        let slice: String =
            String::from_utf16(&units[*failing..*failing + FAILING.len()]).expect("slice");
        assert_eq!(slice, FAILING);
    }
}

#[test]
fn file_surface_keeps_the_byte_order_mark_for_a_utf16_file() {
    let source = "\u{feff}a = 1\nb = 2\ndet(1)";
    let failing = find_utf16(source, FAILING);
    assert_eq!(failing, 13);
    let mut bytes = vec![0xFF, 0xFE];
    for unit in source.chars().skip(1).collect::<String>().encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    let path = surface_file("surface-utf16le.lv", &bytes);
    let result = run_args_of(&["--file", &path, "--omit-functions", "--omit-variables"]);
    assert_eq!(result.exit, 1);
    assert_eq!(diagnostic_position(&result), failing);
}

// ---------------------------------------------------------------------------
// PluginIntegerArgumentWireTests (6 deferred rows)
// ---------------------------------------------------------------------------

#[test]
fn an_integral_digit_count_still_answers_the_requested_width() {
    let result = run_script("x = symbol(\"x\"); print(evalir(compile(x*1/3, [x]), [1], 40))");
    assert_eq!(result.exit, 0);
    assert!(boolean(field(&result.env, "ok")));
    assert_eq!(
        text(&array(field(&result.env, "output"))[0]),
        format!("0.{}", "3".repeat(40))
    );
}

#[test]
fn a_non_integral_digit_count_is_refused_rather_than_floored() {
    let result = run_script("x = symbol(\"x\"); print(evalir(compile(x*1/3, [x]), [1], 2.9))");
    assert_eq!(result.exit, 1);
    assert!(!boolean(field(&result.env, "ok")));
    assert_eq!(text(field(&result.env, "code")), "InvalidArgument");
    assert_eq!(text(field(&result.env, "category")), "TypeMismatch");
    assert!(boolean(field(&result.env, "recoverable")));
    assert!(text(field(&result.env, "message")).contains("2.9"));
}

#[test]
fn a_digit_count_outside_the_bound_is_refused() {
    for digits in ["0", "-5"] {
        let result = run_script(&format!(
            "x = symbol(\"x\"); print(evalir(compile(x*1/3, [x]), [1], {digits}))"
        ));
        assert_eq!(result.exit, 1, "{digits}");
        assert_eq!(
            text(field(&result.env, "code")),
            "InvalidArgument",
            "{digits}"
        );
        assert_eq!(
            text(field(&result.env, "category")),
            "TypeMismatch",
            "{digits}"
        );
        assert!(
            text(field(&result.env, "message")).contains(digits),
            "{digits}"
        );
    }
}

#[test]
fn a_digit_count_wider_than_int64_is_refused_and_named() {
    const WIDE: &str = "1000000000000000000000000000000";
    let result = run_script("x = symbol(\"x\"); print(evalir(compile(x*1/3, [x]), [1], 10^30))");
    assert_eq!(result.exit, 1);
    assert_eq!(text(field(&result.env, "code")), "InvalidArgument");
    assert_eq!(text(field(&result.env, "category")), "TypeMismatch");
    let message = text(field(&result.env, "message"));
    assert!(message.contains(WIDE), "{message}");
    assert!(!message.contains("too large or too small"), "{message}");
}

#[test]
fn evalir_batch_carries_the_same_contract() {
    let result =
        run_script("x = symbol(\"x\"); print(evalir_batch(compile(x*1/3, [x]), [1, 2], 2.5))");
    assert_eq!(result.exit, 1);
    assert_eq!(text(field(&result.env, "code")), "InvalidArgument");
    assert_eq!(text(field(&result.env, "category")), "TypeMismatch");
    assert!(text(field(&result.env, "message")).contains("2.5"));
}

// ---------------------------------------------------------------------------
// TimingTests + TimingScaleTests (14 deferred rows; OperationTiming is refused)
// ---------------------------------------------------------------------------

fn duration_text(nanos: u64) -> String {
    let (value, unit) = scale_duration(std::time::Duration::from_nanos(nanos));
    format!("{value} {unit}")
}

#[test]
fn timing_format_rows() {
    assert_eq!(duration_text(0), "0 ns");
    assert_eq!(duration_text(500), "500 ns");
    assert_eq!(duration_text(1_500), "1.5 µs");
    assert_eq!(duration_text(5_000), "5 µs");
    assert_eq!(duration_text(12_500_000), "12.5 ms");
    assert_eq!(duration_text(2_500_000_000), "2.5 s");
    assert_eq!(duration_text(90_000_000_000), "1.5 min");
    assert_eq!(duration_text(8_100_000_000_000), "2.25 h");
}

#[test]
fn timing_scale_picks_the_largest_whole_unit() {
    for (nanos, expected) in [
        (500u64, "ns"),
        (500_000, "µs"),
        (500_000_000, "ms"),
        (5_000_000_000, "s"),
        (300_000_000_000, "min"),
        (18_000_000_000_000, "h"),
    ] {
        let (_, unit) = scale_duration(std::time::Duration::from_nanos(nanos));
        assert_eq!(unit, expected, "{nanos} ns");
    }
}

#[test]
fn timing_scale_and_format_always_agree() {
    let cases: &[(u64, &str)] = &[
        (0, "ns"),
        (500, "ns"),
        (500_000, "µs"),
        (12_570_000, "ms"),
        (2_500_000_000, "s"),
        (90_000_000_000, "min"),
        (7_200_000_000_000, "h"),
    ];
    for (nanos, expected_unit) in cases {
        let (value, unit) = scale_duration(std::time::Duration::from_nanos(*nanos));
        let text = format!("{value} {unit}");
        assert_eq!(unit, *expected_unit, "{nanos} ns");
        assert!(text.ends_with(&format!(" {unit}")), "{text}");
        let numeric = text[..text.len() - unit.len() - 1]
            .parse::<f64>()
            .expect("a number");
        assert_eq!(
            numeric,
            value.parse::<f64>().expect("the scale value"),
            "{text}"
        );
    }
}

// ---------------------------------------------------------------------------
// RegistryQueryTests.AnUnknownNameIsTheSameRefusalACallProduces (1 row)
// ---------------------------------------------------------------------------

#[test]
fn an_unknown_registry_name_is_the_same_refusal_a_call_produces() {
    let result = run_args_of(&["--function", "determinant"]);
    assert_eq!(result.exit, 1);
    assert!(!boolean(field(&result.env, "ok")));
    assert_eq!(text(field(&result.env, "code")), "UnknownFunction");
    assert_eq!(text(field(&result.env, "category")), "UnsupportedOperation");
    let suggestions: Vec<String> = array(field(&result.env, "suggestions"))
        .iter()
        .map(text)
        .collect();
    assert_eq!(suggestions, vec!["det"]);
    assert!(
        text(field(&result.env, "registrySize"))
            .parse::<usize>()
            .expect("a count")
            >= 100
    );
}

// ---------------------------------------------------------------------------
// EvalfDigitCapHonestyTests (2 deferred rows; the at-cap row is refused)
// ---------------------------------------------------------------------------

fn decimal_places(rendering: &str) -> usize {
    match rendering.find('.') {
        Some(dot) => rendering.len() - dot - 1,
        None => 0,
    }
}

#[test]
fn a_request_beyond_the_cap_is_marked_as_clamped() {
    let result = run_script_with_variables("setprecision(5000); evalf(sqrt(2), 5000)");
    assert_eq!(result.exit, 0);
    let structured = field(field(&result.env, "result"), "structured");
    let value = text(field(structured, "value"));
    assert_eq!(decimal_places(&value), 1000);
    assert!(
        value.starts_with("1.4142135623730950488016887242096980785696718753769"),
        "{value}"
    );
    assert!(!boolean(field(structured, "exact")));
    assert!(boolean(field(structured, "truncated")));
    assert_eq!(text(field(structured, "truncationReason")), "digit-cap");
    assert_eq!(text(field(structured, "budget")), "1000");
    let variable = array(field(&result.env, "variables"))
        .iter()
        .find(|variable| text(field(variable, "name")) == "_")
        .expect("the result binding");
    let projected = field(variable, "structured");
    assert_eq!(text(field(projected, "value")), value);
    assert!(boolean(field(projected, "truncated")));
    assert_eq!(text(field(projected, "truncationReason")), "digit-cap");
    assert_eq!(text(field(projected, "budget")), "1000");
    assert_eq!(
        decimal_places(&text(field(field(&result.env, "result"), "display"))),
        1000
    );
}

#[test]
fn the_smallest_over_cap_request_is_marked_too() {
    let result = run_script("evalf(1/3, 1001)");
    assert_eq!(result.exit, 0);
    let structured = field(field(&result.env, "result"), "structured");
    assert_eq!(
        text(field(structured, "value")),
        format!("0.{}", "3".repeat(1000))
    );
    assert!(boolean(field(structured, "truncated")));
    assert_eq!(text(field(structured, "truncationReason")), "digit-cap");
    assert_eq!(text(field(structured, "budget")), "1000");
}

// ---------------------------------------------------------------------------
// LibraryProjectionAgreementTests.AClampedValue_IsMarked_ThroughTheFacade (1 row)
// ---------------------------------------------------------------------------

#[test]
fn a_clamped_value_is_marked_through_the_facade_as_it_is_on_the_cli() {
    let mut engine = Engine::new();
    engine.set_precision(5000);
    let value = engine.eval("evalf(sqrt(2), 5000)").expect("evalf answers");
    let facade = lovelace_lang::structured::project_value(&engine, &value, None);
    let cli = run_script("setprecision(5000); evalf(sqrt(2), 5000)");
    let cli_structured = field(field(&cli.env, "result"), "structured");
    let facade_value = text(field(&facade, "value"));
    assert_eq!(decimal_places(&facade_value), 1000);
    assert_eq!(facade_value, text(field(cli_structured, "value")));
    assert!(boolean(field(&facade, "truncated")));
    assert_eq!(text(field(&facade, "truncationReason")), "digit-cap");
    assert_eq!(text(field(&facade, "budget")), "1000");
}

// ---------------------------------------------------------------------------
// AllocationRefusalEnvelopeTests (3 deferred rows)
// ---------------------------------------------------------------------------

fn assert_budget_refusal(script: &str, count: &str) {
    let result = run_script(script);
    assert!(
        !result.stdout.is_empty(),
        "{script}: stdout must never be empty"
    );
    assert_eq!(result.exit, 1, "{script}");
    assert!(!boolean(field(&result.env, "ok")), "{script}");
    assert_eq!(
        text(field(&result.env, "code")),
        "AllocationRefused",
        "{script}"
    );
    assert_eq!(
        text(field(&result.env, "category")),
        "BudgetExceeded",
        "{script}"
    );
    assert!(boolean(field(&result.env, "recoverable")), "{script}");
    let message = text(field(&result.env, "message"));
    assert!(message.contains(count), "{script}: {message}");
    assert!(
        !message.contains("Insufficient memory"),
        "{script}: {message}"
    );
    assert!(matches!(field(&result.env, "diagnostics"), Json::Arr(_)));
    assert!(matches!(
        field(&result.env, "elapsed"),
        Json::Str(_) | Json::Num(_)
    ));
    assert!(matches!(field(&result.env, "elapsedTime"), Json::Obj(_)));
    assert!(matches!(field(&result.env, "timings"), Json::Arr(_)));
}

#[test]
fn huge_zeros_crosses_as_a_recoverable_budget_refusal() {
    assert_budget_refusal("zeros(1000000000)", "1000000000");
}

#[test]
fn huge_zeros_product_crosses_as_a_recoverable_budget_refusal() {
    assert_budget_refusal("zeros(1000000000, 1000000000)", "1000000000000000000");
}

#[test]
fn ordinary_zeros_still_succeeds() {
    let result = run_script("zeros(2, 3)");
    assert_eq!(result.exit, 0);
    assert!(boolean(field(&result.env, "ok")));
    assert_eq!(
        text(field(field(&result.env, "result"), "typed")),
        "[[0, 0, 0], [0, 0, 0]] (Array)"
    );
}
