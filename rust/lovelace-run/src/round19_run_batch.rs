//! Round-19 ports of the `Lovelace.Run.Tests` rows that now reproduce.
//!
//! Every test drives the runner's own `execute`/`envelope_for` pair through
//! `parity_batch::run_args`, so an assertion cannot pass on a flow the binary does not run.
//! The C# fact or theory row is the contract; each test below repeats its concrete assertions
//! (exit code, code/category, message, structured field, position or signature).

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

fn run_args_of(args: &[&str]) -> RunResult {
    let args: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
    with_large_stack(move || {
        let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        run_args(&refs)
    })
}

fn code(r: &RunResult) -> &str {
    text(get(&r.env, "code"))
}

fn category(r: &RunResult) -> &str {
    text(get(&r.env, "category"))
}

fn message(r: &RunResult) -> &str {
    text(get(&r.env, "message"))
}

fn diagnostics(r: &RunResult) -> &[Json] {
    array(get(&r.env, "diagnostics"))
}

fn timings(r: &RunResult) -> &[Json] {
    array(get(&r.env, "timings"))
}

fn structured(r: &RunResult) -> &Json {
    get(get(&r.env, "result"), "structured")
}

fn field<'a>(st: &'a Json, name: &str) -> &'a Json {
    for f in array(get(st, "fields")) {
        if text(get(f, "name")) == name {
            return get(f, "value");
        }
    }
    panic!(
        "structured record has no field '{name}': {}",
        st.to_string()
    )
}

fn decimals(rendering: &str) -> usize {
    let rendering = rendering.split(' ').next().unwrap_or(rendering);
    match rendering.find('.') {
        Some(dot) => rendering.len() - dot - 1,
        None => 0,
    }
}

fn real_value(r: &RunResult) -> &str {
    text(get(structured(r), "value"))
}

fn assert_timing_shape(t: &Json, what: &str) {
    assert!(
        opt(t, "position").is_some(),
        "{what}: a timing carries its source position"
    );
    let elapsed = get(t, "elapsed");
    let unit = text(get(elapsed, "unit"));
    assert!(
        ["ns", "us", "µs", "ms", "s", "min", "h"].contains(&unit),
        "{what}: unknown timing unit {unit}"
    );
    assert!(
        opt(t, "resultKind").is_some(),
        "{what}: a timing carries the result kind"
    );
    assert!(
        opt(t, "hasOutput").is_some(),
        "{what}: a timing says whether the statement printed"
    );
}

// ---------------------------------------------------------------------------
// EnginePrecisionDigitRefusalTests
// ---------------------------------------------------------------------------

#[test]
fn run_engine_precision_request_above_the_engine_precision_is_the_documented_argument_error() {
    for builtin in ["pi", "e"] {
        let script = format!("setprecision(20); {builtin}(500)");
        let r = run_script(&script);
        let entries = timings(&r);
        assert_eq!(
            entries.len(),
            2,
            "{builtin}: one engine, two statements: {}",
            r.env.to_string()
        );
        let expected_position = script.find(&format!("{builtin}(")).unwrap() as i64;
        assert_eq!(number(get(&entries[1], "position")), expected_position);
        assert_eq!(r.exit, 1);
        assert!(!boolean(get(&r.env, "ok")));
        assert!(opt(&r.env, "result").is_none());
        assert_eq!(code(&r), "InvalidArgument");
        assert_eq!(category(&r), "TypeMismatch");
        assert!(boolean(get(&r.env, "recoverable")));
        assert_eq!(
            message(&r),
            format!(
                "{builtin}(): argument 1 (digits) must be a Natural or Integer between 1 and 20; got 500."
            )
        );
        let diag = &diagnostics(&r)[0];
        let text = text(get(diag, "message"));
        assert!(text.contains("500") && text.contains("20"));
        assert!(!text.contains("Parameter"));
    }
}

#[test]
fn run_engine_precision_default_request_is_answered_in_full() {
    const PI_HEAD: &str = "141592653589793238462643383279502884197169399375105820974944";
    const PI_TAIL: &str = "89122793818301194912";
    const E_HEAD: &str = "718281828459045235360287471352662497757247093699959574966967";
    const E_TAIL: &str = "93163688923009879312";
    for (builtin, head, tail) in [("pi", PI_HEAD, PI_TAIL), ("e", E_HEAD, E_TAIL)] {
        let r = run_script(&format!("{builtin}(500)"));
        assert_eq!(
            r.exit,
            0,
            "{builtin}(500) must answer: {}",
            r.env.to_string()
        );
        let st = structured(&r);
        assert_eq!(text(get(st, "kind")), "Real");
        let value = text(get(st, "value"));
        assert_eq!(decimals(value), 500);
        assert!(
            value.contains(head) && value.ends_with(tail),
            "{builtin}: {value}"
        );
        assert!(!boolean(get(st, "exact")));
    }
}

#[test]
fn run_engine_precision_request_inside_the_engine_precision_still_succeeds() {
    let cases = [
        (
            "setprecision(20); pi(20)",
            20usize,
            Some("3.14159265358979323846"),
        ),
        ("setprecision(20); pi(10)", 10, Some("3.1415926535")),
        (
            "setprecision(20); e(20)",
            20,
            Some("2.71828182845904523536"),
        ),
        ("setprecision(2000); pi(500)", 500, None),
    ];
    for (script, digits, expected) in cases {
        let r = run_script(script);
        assert_eq!(
            r.exit,
            0,
            "{script} exited {}: {}",
            r.exit,
            r.env.to_string()
        );
        assert!(boolean(get(&r.env, "ok")));
        let value = real_value(&r);
        assert_eq!(decimals(value), digits, "{script}: {value}");
        if let Some(expected) = expected {
            assert_eq!(value, expected);
        } else {
            assert!(
                value.starts_with("3.141592653589793238462643383279502884197169399375105820974944")
            );
            assert!(value.ends_with("89122793818301194912"));
        }
    }
}

#[test]
fn run_engine_precision_count_below_one_is_the_same_typed_refusal() {
    for (builtin, digits) in [("pi", "0"), ("pi", "-3"), ("e", "0"), ("e", "-7")] {
        let r = run_script(&format!("{builtin}({digits})"));
        assert_eq!(r.exit, 1);
        assert!(!boolean(get(&r.env, "ok")));
        assert_eq!(code(&r), "InvalidArgument");
        assert_eq!(category(&r), "TypeMismatch");
        assert!(boolean(get(&r.env, "recoverable")));
        assert_eq!(
            message(&r),
            format!(
                "{builtin}(): argument 1 (digits) must be a Natural or Integer between 1 and 1000; got {digits}."
            )
        );
    }
}

#[test]
fn run_engine_precision_count_wider_than_int64_is_the_same_typed_refusal() {
    for builtin in ["pi", "e"] {
        let r = run_script(&format!("{builtin}(10^30)"));
        assert_eq!(r.exit, 1);
        assert!(!boolean(get(&r.env, "ok")));
        assert_eq!(code(&r), "InvalidArgument");
        assert_eq!(category(&r), "TypeMismatch");
        let text = message(&r);
        assert!(text.contains("1000000000000000000000000000000"), "{text}");
        assert!(
            text.contains(&format!("{builtin}(): argument 1 (digits)")),
            "{text}"
        );
    }
}

#[test]
fn run_engine_precision_refusal_is_stable_over_repetitions_and_never_a_framework_message() {
    for _ in 0..5 {
        let r = run_script("setprecision(20); pi(500)");
        assert_eq!(r.exit, 1);
        assert!(!boolean(get(&r.env, "ok")));
        assert_eq!(code(&r), "InvalidArgument");
        assert_eq!(category(&r), "TypeMismatch");
        assert_eq!(
            message(&r),
            "pi(): argument 1 (digits) must be a Natural or Integer between 1 and 20; got 500."
        );
    }
}

#[test]
fn run_engine_precision_of_one_run_does_not_leak_into_the_next() {
    let refused = run_script("setprecision(20); pi(500)");
    assert_eq!(refused.exit, 1);
    assert!(!boolean(get(&refused.env, "ok")));
    let answered = run_script("pi(500)");
    assert_eq!(
        answered.exit,
        0,
        "the next run must answer: {}",
        answered.env.to_string()
    );
    assert_eq!(decimals(real_value(&answered)), 500);
}

// ---------------------------------------------------------------------------
// BuiltinSurfaceContractTests
// ---------------------------------------------------------------------------

#[test]
fn run_evalf_honours_the_digit_count_for_an_already_numeric_argument() {
    let cases = [
        ("evalf(sqrt(2), 5)", "1.41421"),
        ("evalf(pi, 5)", "3.14159"),
        ("evalf(1/3, 5)", "0.33333"),
        (
            "evalf(sqrt(2), 50)",
            "1.41421356237309504880168872420969807856967187537694",
        ),
        (
            "setprecision(20); evalf(sqrt(2), 20)",
            "1.4142135623730950488",
        ),
    ];
    for (script, expected) in cases {
        let r = run_script(script);
        assert_eq!(
            r.exit,
            0,
            "{script} exited {}: {}",
            r.exit,
            r.env.to_string()
        );
        let st = structured(&r);
        assert_eq!(text(get(st, "kind")), "Real");
        assert_eq!(text(get(st, "value")), expected, "{script}");
    }
}

#[test]
fn run_evalf_refuses_a_digit_count_it_cannot_honour_as_a_recoverable_argument_error() {
    let prefix = "evalf(): argument 2 (digits) must be a Natural or Integer digit count between 1 and 2147483647; got ";
    for script in [
        "evalf(sin(1), 0)",
        "evalf(cos(1), 0)",
        "evalf(exp(1), 0)",
        "evalf(1/3, 0)",
        "evalf(sqrt(2), 0)",
        "evalf(pi, 0)",
        "evalf(sin(1), -1)",
        "evalf(sin(1), -100)",
        "evalf(1/3, 2147483648)",
        "evalf(sin(1), 10^30)",
    ] {
        let r = run_script(script);
        assert_eq!(r.exit, 1, "{script} must refuse: {}", r.env.to_string());
        assert!(!boolean(get(&r.env, "ok")));
        assert_eq!(code(&r), "InvalidArgument", "{script}");
        assert_eq!(category(&r), "TypeMismatch", "{script}");
        assert!(boolean(get(&r.env, "recoverable")));
        assert!(message(&r).starts_with(prefix), "{script}: {}", message(&r));
        assert_ne!(
            message(&r),
            "Object reference not set to an instance of an object."
        );
    }
}

#[test]
fn run_capabilities_domain_statement_is_scoped_to_solve_and_integer_domains_still_work() {
    let r = run_script("capabilities()");
    assert_eq!(r.exit, 0);
    let st = structured(&r);
    assert!(opt(st, "fields").is_some());
    for absent in ["supported_domains", "unsupported_domains"] {
        let has = array(get(st, "fields"))
            .iter()
            .any(|f| text(get(f, "name")) == absent);
        assert!(!has, "the runtime-wide domain claim must be gone");
    }
    let domains = |name: &str| -> Vec<String> {
        array(get(field(st, name), "elements"))
            .iter()
            .map(|d| text(get(d, "domain")).to_string())
            .collect()
    };
    assert_eq!(domains("solve_domains_accepted"), vec!["real", "complex"]);
    assert_eq!(
        domains("solve_domains_refused"),
        vec!["integer", "rational"]
    );
    for domain in ["integer", "rational"] {
        let refused = run_script(&format!(
            "x = symbol(\"x\"); solve(x^2 - 2 == 0, x, {domain})"
        ));
        assert_eq!(refused.exit, 1, "the statement refuses {domain}");
        assert!(!boolean(get(&refused.env, "ok")));
        assert!(message(&refused).contains(&format!("got {domain}")));
    }
    for domain in ["real", "complex"] {
        let accepted = run_script(&format!(
            "x = symbol(\"x\"); solve(x^2 - 2 == 0, x, {domain})"
        ));
        assert_eq!(accepted.exit, 0);
        assert_eq!(
            text(get(field(structured(&accepted), "domain"), "domain")),
            domain
        );
    }
    for domain in ["real", "complex", "integer", "rational"] {
        let symbol = run_script(&format!("symbol(\"s\", {domain})"));
        assert_eq!(symbol.exit, 0);
        assert!(boolean(get(&symbol.env, "ok")));
    }
}

// ---------------------------------------------------------------------------
// LibraryProjectionAgreementTests — the Rust façade is `structured::project_value`
// ---------------------------------------------------------------------------

fn project(script: &str, precision: Option<usize>) -> Json {
    let mut engine = Engine::new();
    if let Some(precision) = precision {
        engine.set_precision(precision);
    }
    let value = engine
        .eval(script)
        .unwrap_or_else(|e| panic!("'{script}' failed: {e}"));
    structured::project_value(&engine, &value, None)
}

fn project_value(script: &str, precision: Option<usize>) -> lovelace_lang::Value {
    let mut engine = Engine::new();
    if let Some(precision) = precision {
        engine.set_precision(precision);
    }
    engine
        .eval(script)
        .unwrap_or_else(|e| panic!("'{script}' failed: {e}"))
}

fn opt_text<'a>(j: &'a Json, key: &str) -> Option<&'a str> {
    opt(j, key).and_then(|v| v.as_str())
}

fn opt_bool(j: &Json, key: &str) -> Option<bool> {
    opt(j, key).map(boolean)
}

fn opt_number(j: &Json, key: &str) -> Option<i64> {
    opt(j, key).map(number)
}

#[test]
fn run_library_facade_and_cli_agree_on_one_structured_value() {
    for script in [
        "setprecision(1100); pi(1100)",
        "setprecision(1100); sqrt(2)",
        "setprecision(1100); e(1100)",
        "evalf(pi, 1000)",
        "pi(30)",
        "evalf(sqrt(2), 5)",
        "sqrt(2)",
        "1/7",
        "1/8",
    ] {
        let facade = project(script, None);
        let cli = run_script(script);
        assert_eq!(cli.exit, 0, "{script}: {}", cli.env.to_string());
        let cli = structured(&cli);
        assert_eq!(
            text(get(cli, "kind")),
            text(get(&facade, "kind")),
            "{script}"
        );
        assert_eq!(
            opt_text(cli, "value"),
            opt_text(&facade, "value"),
            "{script}"
        );
        assert_eq!(
            opt_bool(cli, "exact"),
            opt_bool(&facade, "exact"),
            "{script}"
        );
        assert_eq!(
            opt_bool(cli, "truncated"),
            opt_bool(&facade, "truncated"),
            "{script}"
        );
        assert_eq!(
            opt_text(cli, "truncationReason"),
            opt_text(&facade, "truncationReason"),
            "{script}"
        );
        assert_eq!(
            opt_number(cli, "budget"),
            opt_number(&facade, "budget"),
            "{script}"
        );
        assert_eq!(
            opt_text(cli, "numerator"),
            opt_text(&facade, "numerator"),
            "{script}"
        );
        assert_eq!(
            opt_text(cli, "denominator"),
            opt_text(&facade, "denominator"),
            "{script}"
        );
    }
}

#[test]
fn run_library_facade_untruncated_value_is_neither_cut_nor_padded() {
    let small = project("evalf(sqrt(2), 5)", None);
    assert_eq!(text(get(&small, "value")), "1.41421");
    let thirty = project("pi(30)", None);
    assert_eq!(decimals(text(get(&thirty, "value"))), 30);
}

#[test]
fn run_library_facade_exact_real_still_crosses_as_its_rational() {
    let dto = project("1/8", None);
    assert_eq!(text(get(&dto, "value")), "0.125");
    assert!(boolean(get(&dto, "exact")));
    assert_eq!(text(get(&dto, "numerator")), "1");
    assert_eq!(text(get(&dto, "denominator")), "8");
    assert!(opt(&dto, "truncated").is_none());
}

#[test]
fn run_library_facade_and_cli_publish_the_same_digits_for_the_findings_script() {
    let facade = project("pi(1100)", Some(1100));
    let cli = run_script("setprecision(1100); pi(1100)");
    assert_eq!(cli.exit, 0);
    let cli = structured(&cli);
    assert_eq!(text(get(cli, "value")), text(get(&facade, "value")));
    assert_eq!(decimals(text(get(&facade, "value"))), 1100);
}

#[test]
fn run_library_facade_nested_reals_cross_in_full_too() {
    let dto = project("[pi(1100), e(1100)]", Some(1100));
    let elements = array(get(&dto, "elements"));
    assert_eq!(decimals(text(get(&elements[0], "value"))), 1100);
    assert_eq!(decimals(text(get(&elements[1], "value"))), 1100);
}

// ---------------------------------------------------------------------------
// InspectionExactnessTests
// ---------------------------------------------------------------------------

#[test]
fn run_inspection_answers_exactness_as_a_boolean() {
    for (source, expected) in [("1", true), ("1/3", true), ("1/(10^19)", true)] {
        let script = format!("v = {source}; i = inspect(v)");
        let r = run_script(&script);
        assert_eq!(
            r.exit,
            0,
            "inspect({source}) exited {}: {}",
            r.exit,
            r.env.to_string()
        );
        let exact = field(structured(&r), "exact");
        assert_eq!(
            text(get(exact, "kind")),
            "Boolean",
            "inspect({source}).exact is not a Boolean"
        );
        assert_eq!(
            text(get(exact, "value")),
            if expected { "true" } else { "false" }
        );
    }
}

#[test]
fn run_inspection_exact_agrees_with_the_values_own_exact_flag() {
    for source in ["1/3", "1/(10^19)", "1/1009"] {
        let script = format!("v = {source}; i = inspect(v)");
        let r = run_args_of(&["--eval", &script, "--json", "--omit-functions"]);
        assert_eq!(r.exit, 0);
        let value = array(get(&r.env, "variables"))
            .iter()
            .find(|v| text(get(v, "name")) == "v")
            .map(|v| get(v, "structured"))
            .expect("the variable form of v is published");
        let record = field(structured(&r), "exact");
        let value_exact = boolean(get(value, "exact"));
        assert_eq!(
            text(get(record, "value")) == "true",
            value_exact,
            "{source}"
        );
    }
}

// ---------------------------------------------------------------------------
// SourcePositionTests
// ---------------------------------------------------------------------------

fn diagnostic(r: &RunResult) -> &Json {
    &diagnostics(r)[0]
}

#[test]
fn run_source_position_runtime_failure_in_the_third_statement_names_that_statement_lf() {
    let source = "a = 1\nb = 2\ndet(1)";
    let failing = source.find("det(1)").unwrap() as i64;
    assert_eq!(failing, 12);
    let r = run_script(source);
    assert_eq!(r.exit, 1);
    assert_eq!(number(get(diagnostic(&r), "position")), failing);
    assert_eq!(number(get(diagnostic(&r), "line")), 3);
    assert_eq!(number(get(diagnostic(&r), "column")), 1);
    let positions: Vec<i64> = timings(&r)
        .iter()
        .map(|t| number(get(t, "position")))
        .collect();
    assert_eq!(positions, vec![0, 6, 12]);
    assert_eq!(*positions.last().unwrap(), failing);
}

#[test]
fn run_source_position_runtime_failure_in_the_third_statement_names_that_statement_crlf() {
    let source = "a = 1\r\nb = 2\r\ndet(1)";
    let failing = source.find("det(1)").unwrap() as i64;
    assert_eq!(failing, 14);
    let r = run_script(source);
    assert_eq!(r.exit, 1);
    assert_eq!(number(get(diagnostic(&r), "position")), failing);
    assert_eq!(number(get(diagnostic(&r), "line")), 3);
    assert_eq!(number(get(diagnostic(&r), "column")), 1);
    let positions: Vec<i64> = timings(&r)
        .iter()
        .map(|t| number(get(t, "position")))
        .collect();
    assert_eq!(positions, vec![0, 7, 14]);
}

#[test]
fn run_source_position_single_statement_failure_keeps_position_zero_line_one_column_one() {
    let r = run_script("det(1)");
    assert_eq!(r.exit, 1);
    assert_eq!(number(get(diagnostic(&r), "position")), 0);
    assert_eq!(number(get(diagnostic(&r), "line")), 1);
    assert_eq!(number(get(diagnostic(&r), "column")), 1);
    let positions: Vec<i64> = timings(&r)
        .iter()
        .map(|t| number(get(t, "position")))
        .collect();
    assert_eq!(positions, vec![0]);
}

#[test]
fn run_source_position_diagnostic_entry_keeps_its_four_field_shape() {
    let r = run_script("a = 1\nb = 2\ndet(1)");
    let entry = diagnostic(&r);
    let mut keys: Vec<&str> = match entry {
        Json::Obj(map) => map.keys().map(|s| s.as_str()).collect(),
        _ => panic!("diagnostic is not an object"),
    };
    keys.sort();
    assert_eq!(keys, vec!["column", "line", "message", "position"]);
    let mut envelope_keys: Vec<&str> = match &r.env {
        Json::Obj(map) => map.keys().map(|s| s.as_str()).collect(),
        _ => panic!("envelope is not an object"),
    };
    envelope_keys.sort();
    assert_eq!(
        envelope_keys,
        vec![
            "artifacts",
            "category",
            "code",
            "diagnostics",
            "elapsed",
            "elapsedTime",
            "mathIrVersion",
            "message",
            "ok",
            "output",
            "partialVariables",
            "protocolVersion",
            "recoverable",
            "signature",
            "symbolicFormatVersion",
            "timings",
        ]
    );
}

// ---------------------------------------------------------------------------
// CallSignatureDiagnosticsTests
// ---------------------------------------------------------------------------

#[test]
fn run_call_signature_an_arity_refusal_publishes_the_signature() {
    let r = run_script("dot(1)");
    assert_eq!(code(&r), "InvalidArgument");
    let signature = get(&r.env, "signature");
    assert_eq!(text(get(signature, "function")), "dot");
    let parameters: Vec<&str> = array(get(signature, "parameters"))
        .iter()
        .map(text)
        .collect();
    assert_eq!(parameters, vec!["a", "b"]);
    assert_eq!(number(get(signature, "expectedMin")), 2);
    assert_eq!(number(get(signature, "expectedMax")), 2);
    assert!(!boolean(get(signature, "variadic")));
    assert_eq!(number(get(signature, "actual")), 1);
}

#[test]
fn run_call_signature_a_shape_refusal_publishes_the_offending_argument() {
    let r = run_script("det(1)");
    let signature = get(&r.env, "signature");
    assert_eq!(text(get(signature, "function")), "det");
    assert_eq!(number(get(signature, "position")), 1);
    assert_eq!(text(get(signature, "parameter")), "m");
    assert_eq!(text(get(signature, "expectedShape")), "a square matrix");
    assert_eq!(text(get(signature, "arrivedKind")), "Natural");
}

#[test]
fn run_call_signature_a_failure_that_is_not_a_call_site_refusal_carries_no_signature() {
    let r = run_script("2 ^ 0.5");
    assert_eq!(code(&r), "UnsupportedOperation");
    assert!(opt(&r.env, "signature").is_none());
}

// ---------------------------------------------------------------------------
// UnknownFunctionEnvelopeTests
// ---------------------------------------------------------------------------

#[test]
fn run_unknown_name_crosses_as_name_resolution_with_the_registrys_answer() {
    let r = run_script("determinant([[1,2],[3,4]])");
    assert_eq!(r.exit, 1);
    assert!(!boolean(get(&r.env, "ok")));
    assert_eq!(code(&r), "UnknownFunction");
    assert_eq!(category(&r), "UnsupportedOperation");
    assert!(boolean(get(&r.env, "recoverable")));
    let suggestions: Vec<&str> = array(get(&r.env, "suggestions")).iter().map(text).collect();
    assert!(suggestions.contains(&"det"), "suggestions: {suggestions:?}");
    assert!(number(get(&r.env, "registrySize")) > 0);
    assert!(message(&r).contains("Did you mean"));
    assert!(text(get(&diagnostics(&r)[0], "message")).contains("Did you mean"));
}

#[test]
fn run_unknown_syntax_name_carries_a_note_and_no_function_suggestion() {
    let r = run_script("range(5)");
    assert_eq!(code(&r), "UnknownFunction");
    let hints: Vec<&str> = array(get(&r.env, "hints")).iter().map(text).collect();
    assert!(hints[0].contains("1..n"), "hints: {hints:?}");
    assert!(message(&r).contains("1..n"));
    assert!(opt(&r.env, "suggestions").is_none());
}

#[test]
fn run_unknown_name_nothing_is_close_to_keeps_its_message_and_omits_the_optional_fields() {
    let r = run_script("nosuchfn(1)");
    assert_eq!(code(&r), "UnknownFunction");
    assert_eq!(message(&r), "Unknown function 'nosuchfn'.");
    assert!(opt(&r.env, "suggestions").is_none());
    assert!(opt(&r.env, "hints").is_none());
}

// ---------------------------------------------------------------------------
// ArityValidationTests
// ---------------------------------------------------------------------------

#[test]
fn run_arity_declared_argument_count_still_succeeds() {
    let r = run_script("x = symbol(\"x\"); compile_full(x^2 + 1, [x])");
    assert_eq!(r.exit, 0);
    assert!(boolean(get(&r.env, "ok")));
    assert_eq!(text(get(structured(&r), "type")), "CompilationResult");
}

#[test]
fn run_arity_optional_trailing_parameters_stay_callable_in_the_shorter_form() {
    for script in [
        "x = symbol(\"x\"); solve(x^2 - 4 == 0, x)",
        "x = symbol(\"x\"); type(x)",
        "dft([0, 1, 0, 0])",
    ] {
        let r = run_script(script);
        assert_eq!(
            r.exit,
            0,
            "'{script}' exited {}: {}",
            r.exit,
            r.env.to_string()
        );
        assert!(boolean(get(&r.env, "ok")), "'{script}' must evaluate");
    }
}

// ---------------------------------------------------------------------------
// SystemBuiltinArgumentShapeTests — the declared list form still succeeds
// ---------------------------------------------------------------------------

#[test]
fn run_system_builtin_declared_list_form_still_succeeds() {
    for call in [
        "solve_system([x + y == 2, x - y == 0], [x, y])",
        "solve_system_full([x + y == 2, x - y == 0], [x, y])",
        "jacobian([x*y, x + y], [x, y])",
    ] {
        let script = format!("x = symbol(\"x\"); y = symbol(\"y\"); {call}");
        let r = run_script(&script);
        assert_eq!(
            r.exit,
            0,
            "'{call}' exited {}: {}",
            r.exit,
            r.env.to_string()
        );
        assert!(boolean(get(&r.env, "ok")), "'{call}' must evaluate");
    }
}

// ---------------------------------------------------------------------------
// WideArgumentRefusalTests
// ---------------------------------------------------------------------------

const HUGE_COUNT: &str = "123456789012345678901234567890";
const INT64_BOUND: &str = "9223372036854775807";

fn assert_not_raw_clr_text(text: &str) {
    assert!(!text.contains("Int64"), "{text}");
    assert!(!text.to_lowercase().contains("overflow"), "{text}");
    assert!(!text.contains("Parameter 'order'"), "{text}");
}

#[test]
fn run_wide_setprecision_count_wider_than_int64_is_an_argument_refusal() {
    let r = run_script(&format!("setprecision({HUGE_COUNT})"));
    assert_eq!(code(&r), "InvalidArgument");
    assert_eq!(category(&r), "TypeMismatch");
    assert!(message(&r).contains(HUGE_COUNT));
    assert!(message(&r).contains(INT64_BOUND));
    assert_not_raw_clr_text(message(&r));
}

#[test]
fn run_wide_zeros_with_zero_still_answers_an_empty_array() {
    let r = run_script("zeros(0)");
    assert_eq!(r.exit, 0);
    assert!(boolean(get(&r.env, "ok")));
    assert_eq!(text(get(get(&r.env, "result"), "display")), "[]");
}

#[test]
fn run_wide_indexing_with_a_negative_index_keeps_its_documented_refusal() {
    let r = run_script("[1, 2, 3][-1]");
    assert_eq!(code(&r), "InvalidOperation");
    assert!(
        message(&r).contains("Index -1 is out of range for vector of length 3."),
        "{}",
        message(&r)
    );
}

#[test]
fn run_wide_setprecision_non_positive_count_is_the_same_argument_refusal_as_pi() {
    let r = run_script("setprecision(-5)");
    assert_eq!(code(&r), "InvalidArgument");
    assert_eq!(category(&r), "TypeMismatch");
    assert!(message(&r).contains("setprecision(): argument 1 (digits)"));
    assert!(message(&r).contains("-5"));
    let pi = run_script("pi(-5)");
    assert_eq!(code(&pi), code(&r));
    assert_eq!(category(&pi), category(&r));
    assert!(message(&pi).contains("pi(): argument 1 (digits)"));
    assert!(message(&pi).contains("-5"));
}

#[test]
fn run_wide_pi_with_a_count_wider_than_int64_still_names_the_count_and_the_cap() {
    let r = run_script(&format!("pi({HUGE_COUNT})"));
    assert_eq!(code(&r), "InvalidArgument");
    assert_eq!(category(&r), "TypeMismatch");
    assert!(message(&r).contains(HUGE_COUNT));
}

// ---------------------------------------------------------------------------
// DivRemRecordTests
// ---------------------------------------------------------------------------

fn div_rem_structured(a: &str, b: &str) -> Json {
    let script = format!("A = {a}; B = {b}; divrem(A, B)");
    let r = run_args_of(&["--eval", &script, "--json", "--omit-functions"]);
    assert_eq!(
        r.exit,
        0,
        "divrem({a}, {b}) exited {}: {}",
        r.exit,
        r.env.to_string()
    );
    structured(&r).clone()
}

#[test]
fn run_divrem_returns_two_integer_fields_that_satisfy_the_division_identity() {
    let cases = [
        ("7", "2", "7", "2"),
        ("10^100", "7", &format!("1{}", "0".repeat(100)), "7"),
        ("-17", "5", "-17", "5"),
        ("17", "-5", "17", "-5"),
        ("-17", "-5", "-17", "-5"),
    ];
    for (a, b, a_decimal, b_decimal) in cases {
        let st = div_rem_structured(a, b);
        assert_eq!(text(get(&st, "kind")), "Record");
        assert_eq!(text(get(&st, "type")), "DivRemResult");
        let names: Vec<&str> = array(get(&st, "fields"))
            .iter()
            .map(|f| text(get(f, "name")))
            .collect();
        assert_eq!(names, vec!["quotient", "remainder"]);
        let quotient = field(&st, "quotient");
        let remainder = field(&st, "remainder");
        assert!(["Natural", "Integer"].contains(&text(get(quotient, "kind"))));
        assert!(["Natural", "Integer"].contains(&text(get(remainder, "kind"))));
        let a_mag = abs_decimal(a_decimal);
        let b_mag = abs_decimal(b_decimal);
        let q_mag = abs_decimal(text(get(quotient, "value")));
        let r_mag = abs_decimal(text(get(remainder, "value")));
        assert_eq!(
            a_mag,
            add_decimal(&mul_decimal(&q_mag, &b_mag), &r_mag),
            "divrem({a},{b}) identity"
        );
        assert_eq!(
            cmp_decimal(&r_mag, &b_mag),
            std::cmp::Ordering::Less,
            "|r| < |b|"
        );
        if text(get(quotient, "value")) != "0" {
            assert_eq!(
                sign(text(get(quotient, "value"))),
                sign(a_decimal) * sign(b_decimal)
            );
        }
        if text(get(quotient, "value")) != "0" && text(get(remainder, "value")) != "0" {
            assert_eq!(
                sign(text(get(quotient, "value"))),
                sign(text(get(remainder, "value")))
            );
        }
    }
}

#[test]
fn run_divrem_of_two_naturals_crosses_as_structure_not_prose() {
    let st = div_rem_structured("7", "2");
    assert_ne!(text(get(&st, "kind")), "Text");
    assert_eq!(text(get(field(&st, "quotient"), "value")), "3");
    assert_eq!(text(get(field(&st, "remainder"), "value")), "1");
    assert_eq!(text(get(field(&st, "quotient"), "kind")), "Natural");
}

#[test]
fn run_divrem_keeps_the_engines_documented_sign_rule() {
    for (a, b, quotient, remainder) in [
        ("17", "-5", "-3", "-2"),
        ("-17", "5", "-3", "-2"),
        ("-17", "-5", "3", "2"),
        ("1", "5", "0", "1"),
    ] {
        let st = div_rem_structured(a, b);
        assert_eq!(text(get(field(&st, "quotient"), "value")), quotient);
        assert_eq!(text(get(field(&st, "remainder"), "value")), remainder);
    }
}

fn normalize_decimal(text: &str) -> String {
    let negative = text.starts_with('-');
    let digits = text.trim_start_matches(['-', '+']).trim_start_matches('0');
    let digits = if digits.is_empty() { "0" } else { digits };
    if negative && digits != "0" {
        format!("-{digits}")
    } else {
        digits.to_string()
    }
}

fn abs_decimal(text: &str) -> String {
    normalize_decimal(text).trim_start_matches('-').to_string()
}

fn add_decimal(a: &str, b: &str) -> String {
    let a: Vec<u8> = abs_decimal(a).bytes().rev().map(|c| c - b'0').collect();
    let b: Vec<u8> = abs_decimal(b).bytes().rev().map(|c| c - b'0').collect();
    let mut out = Vec::new();
    let mut carry = 0u8;
    for i in 0..a.len().max(b.len()) {
        let sum = a.get(i).copied().unwrap_or(0) + b.get(i).copied().unwrap_or(0) + carry;
        out.push(sum % 10);
        carry = sum / 10;
    }
    if carry > 0 {
        out.push(carry);
    }
    let s: String = out.iter().rev().map(|d| (d + b'0') as char).collect();
    normalize_decimal(&s)
}

fn mul_decimal(a: &str, b: &str) -> String {
    let a: Vec<u8> = abs_decimal(a).bytes().rev().map(|c| c - b'0').collect();
    let b: Vec<u8> = abs_decimal(b).bytes().rev().map(|c| c - b'0').collect();
    let mut out = vec![0u8; a.len() + b.len()];
    for (i, x) in a.iter().enumerate() {
        let mut carry = 0u8;
        for (j, y) in b.iter().enumerate() {
            let v = out[i + j] + x * y + carry;
            out[i + j] = v % 10;
            carry = v / 10;
        }
        let mut k = i + b.len();
        while carry > 0 {
            let v = out[k] + carry;
            out[k] = v % 10;
            carry = v / 10;
            k += 1;
        }
    }
    while out.len() > 1 && out.last() == Some(&0) {
        out.pop();
    }
    let s: String = out.iter().rev().map(|d| (d + b'0') as char).collect();
    normalize_decimal(&s)
}

fn cmp_decimal(a: &str, b: &str) -> std::cmp::Ordering {
    let a = abs_decimal(a);
    let b = abs_decimal(b);
    a.len().cmp(&b.len()).then_with(|| a.cmp(&b))
}

fn sign(text: &str) -> i32 {
    if text.starts_with('-') { -1 } else { 1 }
}

// ---------------------------------------------------------------------------
// ParseErrorClassificationTests
// ---------------------------------------------------------------------------

#[test]
fn run_parse_failure_in_a_multiline_script_crosses_as_parse_error() {
    let r = run_script("1+1;\n2+2;\n)");
    assert_eq!(r.exit, 1);
    assert!(!boolean(get(&r.env, "ok")));
    assert_eq!(code(&r), "ParseError");
    assert_eq!(category(&r), "ParseError");
    assert!(boolean(get(&r.env, "recoverable")));
    assert!(message(&r).contains("Unexpected token ')'"));
    assert!(timings(&r).is_empty());
}

#[test]
fn run_parse_failure_in_a_single_line_script_crosses_as_parse_error() {
    let r = run_script("1 +");
    assert_eq!(r.exit, 1);
    assert!(!boolean(get(&r.env, "ok")));
    assert_eq!(code(&r), "ParseError");
    assert_eq!(category(&r), "ParseError");
    assert!(boolean(get(&r.env, "recoverable")));
    assert!(message(&r).contains("expected a number"));
    assert!(timings(&r).is_empty());
}

#[test]
fn run_missing_script_file_is_not_a_parse_error() {
    let missing = format!("{}/c6-pd-missing-round19.ls", plot_dir());
    let r = run_args_of(&[
        "--file",
        &missing,
        "--json",
        "--omit-functions",
        "--omit-variables",
    ]);
    assert_eq!(r.exit, 1);
    assert!(!boolean(get(&r.env, "ok")));
    assert_eq!(code(&r), "FileReadError");
    assert_eq!(category(&r), "TypeMismatch");
    assert!(boolean(get(&r.env, "recoverable")));
    assert!(array(get(&r.env, "diagnostics")).is_empty());
    assert!(timings(&r).is_empty());
    assert!(array(get(&r.env, "output")).is_empty());
}

#[test]
fn run_failure_on_a_parsing_script_keeps_its_own_category() {
    let unknown = run_script("nosuchfn(1)");
    assert_eq!(unknown.exit, 1);
    assert_eq!(code(&unknown), "UnknownFunction");
    assert_eq!(category(&unknown), "UnsupportedOperation");
    let shape = run_script("det(1)");
    assert_eq!(shape.exit, 1);
    assert_eq!(code(&shape), "InvalidArgument");
    assert_eq!(category(&shape), "TypeMismatch");
}

#[test]
fn run_parse_phase_depth_refusal_is_not_reclassified_as_a_parse_error() {
    let deep = format!("{}1{};", "(".repeat(3000), ")".repeat(3000));
    let r = run_script(&deep);
    assert_eq!(r.exit, 1);
    assert!(!boolean(get(&r.env, "ok")));
    assert_eq!(code(&r), "DepthExceeded");
    assert_eq!(category(&r), "BudgetExceeded");
    assert!(message(&r).contains("nesting depth"));
}

// ---------------------------------------------------------------------------
// PlotDirectoryErrorTests / PlotFileErrorTests
// ---------------------------------------------------------------------------

#[test]
fn run_plot_directory_naming_an_existing_file_is_an_error_envelope_with_exit_one() {
    let dir = plot_dir();
    let existing = format!("{dir}/r19-existing-file.txt");
    std::fs::write(&existing, "this is a file, not a directory").unwrap();
    let r = run_args_of(&[
        "--eval",
        "1",
        "--json",
        "--omit-functions",
        "--omit-variables",
        "--plot-dir",
        &existing,
    ]);
    assert_eq!(r.exit, 1);
    assert!(!boolean(get(&r.env, "ok")));
    assert_eq!(code(&r), "PlotDirectoryError");
    assert_eq!(category(&r), "TypeMismatch");
    assert!(boolean(get(&r.env, "recoverable")));
    assert!(message(&r).contains(&existing));
    assert!(opt(&r.env, "elapsed").is_some());
    assert!(opt(&r.env, "elapsedTime").is_some());
    assert!(opt(&r.env, "timings").is_some());
    let _ = std::fs::remove_file(&existing);
}

#[test]
fn run_plot_directory_under_an_existing_file_is_an_error_envelope_with_exit_one() {
    let dir = plot_dir();
    let parent = format!("{dir}/r19-plot-parent.txt");
    std::fs::write(&parent, "not a directory").unwrap();
    let child = format!("{parent}/sub");
    let r = run_args_of(&[
        "--eval",
        "1",
        "--json",
        "--omit-functions",
        "--omit-variables",
        "--plot-dir",
        &child,
    ]);
    assert_eq!(r.exit, 1);
    assert_eq!(code(&r), "PlotDirectoryError");
    assert_eq!(category(&r), "TypeMismatch");
    assert!(message(&r).contains(&child));
    let _ = std::fs::remove_file(&parent);
}

#[test]
fn run_valid_plot_directory_still_receives_the_plot_file() {
    let root = format!("{}/r19-plotdir-ok", plot_dir());
    let plot_directory = format!("{root}/nested/plots");
    let _ = std::fs::remove_dir_all(&root);
    let r = run_args_of(&[
        "--eval",
        "plot([1, 2, 3])",
        "--json",
        "--omit-functions",
        "--omit-variables",
        "--plot-dir",
        &plot_directory,
    ]);
    assert_eq!(
        r.exit,
        0,
        "a usable --plot-dir must still succeed: {}",
        r.env.to_string()
    );
    assert!(boolean(get(&r.env, "ok")));
    let plot = get(&r.env, "plot");
    let path = text(get(plot, "path"));
    assert!(
        std::path::Path::new(path).exists(),
        "the plot file must exist: {path}"
    );
    let svg = std::fs::read_to_string(path).unwrap();
    assert!(svg.to_lowercase().contains("<svg"));
    let expected = std::fs::canonicalize(format!("{plot_directory}/plot.svg"))
        .expect("the plot file canonicalizes");
    assert_eq!(std::fs::canonicalize(path).unwrap(), expected);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn run_writable_plot_file_is_still_written() {
    let root = format!("{}/r19-plotfile-ok", plot_dir());
    let plot_directory = format!("{root}/plots");
    let _ = std::fs::remove_dir_all(&root);
    let r = run_args_of(&[
        "--eval",
        "plot([1, 2, 3])",
        "--json",
        "--omit-functions",
        "--omit-variables",
        "--plot-dir",
        &plot_directory,
        "--plot-file",
        "ok.svg",
    ]);
    assert_eq!(
        r.exit,
        0,
        "a writable --plot-file must succeed: {}",
        r.env.to_string()
    );
    assert!(boolean(get(&r.env, "ok")));
    let expected = std::fs::canonicalize(std::path::Path::new(&plot_directory).join("ok.svg"))
        .expect("the plot file canonicalizes");
    assert!(
        expected.exists(),
        "the plot file must be written to {}",
        expected.display()
    );
    assert_eq!(
        std::fs::canonicalize(text(get(get(&r.env, "plot"), "path"))).unwrap(),
        expected
    );
    let _ = std::fs::remove_dir_all(&root);
}

// ---------------------------------------------------------------------------
// CommittedStateTests — the failed run publishes the committed bindings
// ---------------------------------------------------------------------------

#[test]
fn run_failure_publishes_the_committed_bindings() {
    let r = run_args_of(&[
        "--eval",
        "x = 42\ny = x + 1\ndet(1)",
        "--json",
        "--omit-functions",
    ]);
    assert_eq!(r.exit, 1);
    assert_eq!(code(&r), "InvalidArgument");
    let names: Vec<&str> = array(get(&r.env, "partialVariables"))
        .iter()
        .map(|v| text(get(v, "name")))
        .collect();
    assert_eq!(names, vec!["x", "y"]);
    let x = &array(get(&r.env, "partialVariables"))[0];
    assert_eq!(text(get(x, "kind")), "Natural");
    assert!(opt(x, "structured").is_some());
    assert_eq!(text(get(get(x, "structured"), "value")), "42");
}

// ---------------------------------------------------------------------------
// StructuredRealPrecisionTests
// ---------------------------------------------------------------------------

#[test]
fn run_structured_real_counts_at_or_below_the_request_are_carried_exactly() {
    for (script, expected) in [
        ("evalf(pi, 40)", 40usize),
        ("setprecision(1100); e(1100)", 1100),
    ] {
        let r = run_args_of(&["--eval", script, "--json", "--omit-functions"]);
        assert_eq!(r.exit, 0, "{script}: {}", r.env.to_string());
        assert_eq!(decimals(real_value(&r)), expected);
        let variable = array(get(&r.env, "variables"))
            .iter()
            .find(|v| text(get(v, "name")) == "_")
            .expect("the result variable");
        assert_eq!(
            decimals(text(get(get(variable, "structured"), "value"))),
            expected
        );
    }
}

#[test]
fn run_structured_real_nested_reals_inside_an_array_cross_in_full_too() {
    let r = run_args_of(&[
        "--eval",
        "setprecision(1100); [pi(1100), e(1100)]",
        "--json",
        "--omit-functions",
    ]);
    assert_eq!(r.exit, 0);
    let elements = array(get(structured(&r), "elements"));
    assert_eq!(decimals(text(get(&elements[0], "value"))), 1100);
    assert_eq!(decimals(text(get(&elements[1], "value"))), 1100);
}

// ---------------------------------------------------------------------------
// EvalfDigitCapHonestyTests
// ---------------------------------------------------------------------------

#[test]
fn run_evalf_plain_expression_under_the_same_precision_is_not_clamped() {
    let r = run_args_of(&[
        "--eval",
        "setprecision(5000); sqrt(2)",
        "--json",
        "--omit-functions",
    ]);
    assert_eq!(r.exit, 0);
    let st = structured(&r);
    assert_eq!(decimals(text(get(st, "value"))), 5000);
    assert!(opt(st, "truncated").is_none());
    assert!(opt(st, "truncationReason").is_none());
    assert!(opt(st, "budget").is_none());
}

#[test]
fn run_evalf_exact_answer_to_an_over_cap_request_is_not_marked() {
    let r = run_args_of(&["--eval", "evalf(1/2, 5000)", "--json", "--omit-functions"]);
    assert_eq!(r.exit, 0);
    let st = structured(&r);
    assert_eq!(text(get(st, "value")), "0.5");
    assert!(boolean(get(st, "exact")));
    assert!(opt(st, "truncated").is_none());
    assert!(opt(st, "truncationReason").is_none());
    assert!(opt(st, "budget").is_none());
}

// ---------------------------------------------------------------------------
// IntegerArgumentBoundaryMatrixTests — the cells whose exact outcome now holds
// ---------------------------------------------------------------------------

#[test]
fn run_matrix_e_digit_count_boundary_cells_are_accepted() {
    for literal in ["1", "2", "3"] {
        let r = run_script(&format!("e({literal})"));
        assert!(
            boolean(get(&r.env, "ok")),
            "e({literal}) must be accepted: {}",
            r.env.to_string()
        );
        assert_eq!(r.exit, 0);
    }
}

#[test]
fn run_matrix_inventory_templates_reach_the_argument_they_name() {
    let cases = [
        ("e", "e(\"sentinel\")", "(digits)"),
        ("pi", "pi(\"sentinel\")", "(digits)"),
        ("setprecision", "setprecision(\"sentinel\"); 1", "(digits)"),
        ("evalf", "evalf(1/3, \"sentinel\")", "(digits)"),
    ];
    for (builtin, script, anchor) in cases {
        let r = run_script(script);
        assert!(
            !boolean(get(&r.env, "ok")),
            "{script}: the sentinel was accepted"
        );
        let code = code(&r);
        let category = category(&r);
        let text = message(&r);
        assert!(
            code == "InvalidArgument" || code == "InvalidOperation",
            "{script}: expected a typed argument refusal, got {code}/{category} with '{text}'"
        );
        assert!(
            category == "TypeMismatch" || category == "DomainError",
            "{script}: expected a caller-side refusal category, got {category}"
        );
        assert!(
            text.starts_with(&format!("{builtin}()")),
            "{script}: {text}"
        );
        assert!(
            text.contains(anchor),
            "{script}: '{text}' must name {anchor}"
        );
    }
}

// ---------------------------------------------------------------------------
// PrintBudget tests (the three rows deferred in earlier rounds)
// ---------------------------------------------------------------------------

const LONG_NAME: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[test]
fn run_print_budget_identifier_longer_than_the_kernel_allowance_is_never_split() {
    let script = format!("{LONG_NAME} = symbol(\"{LONG_NAME}\"); {LONG_NAME} + 1");
    let r = run_args_of(&[
        "--eval",
        &script,
        "--json",
        "--omit-functions",
        "--omit-variables",
        "--print-budget",
        "1",
    ]);
    assert_eq!(r.exit, 0, "{}", r.env.to_string());
    let st = structured(&r);
    let pretty = text(get(st, "pretty"));
    let body = pretty.strip_suffix(" …").unwrap_or(pretty);
    assert_eq!(body, LONG_NAME);
    assert!(pretty.ends_with(" …"));
    assert!(boolean(get(st, "truncated")));
    assert_eq!(text(get(st, "truncationReason")), "node-budget");
    assert_eq!(number(get(st, "budget")), 1);

    let full = run_args_of(&[
        "--eval",
        &script,
        "--json",
        "--omit-functions",
        "--omit-variables",
    ]);
    let full_pretty = text(get(structured(&full), "pretty"));
    assert!(full_pretty.starts_with(body));
    assert!(full_pretty.len() > body.len());
}

#[test]
fn run_print_budget_unbudgeted_renderings_are_unchanged() {
    let script = format!("{LONG_NAME} = symbol(\"{LONG_NAME}\"); {LONG_NAME} + 1");
    let r = run_args_of(&[
        "--eval",
        &script,
        "--json",
        "--omit-functions",
        "--omit-variables",
        "--print-budget",
        "1",
    ]);
    assert_eq!(r.exit, 0);
    assert_eq!(
        text(get(get(&r.env, "result"), "display")),
        format!("{LONG_NAME} + 1")
    );
    assert!(text(get(get(&r.env, "result"), "typed")).starts_with(&format!("{LONG_NAME} + 1 ")));
    let unbudgeted = run_args_of(&[
        "--eval",
        &script,
        "--json",
        "--omit-functions",
        "--omit-variables",
    ]);
    let st = structured(&unbudgeted);
    assert!(opt(st, "truncated").is_none());
    assert_eq!(text(get(st, "pretty")), format!("{LONG_NAME} + 1"));
}

#[test]
fn run_print_budget_also_bounds_the_structured_variables() {
    let r = run_args_of(&[
        "--eval",
        "x = symbol(\"x\"); x^2",
        "--json",
        "--omit-functions",
        "--print-budget",
        "1",
    ]);
    assert_eq!(r.exit, 0);
    let result = structured(&r);
    let variables = array(get(&r.env, "variables"));
    let underscore = variables
        .iter()
        .find(|v| text(get(v, "name")) == "_")
        .expect("the result variable");
    let x_squared = get(underscore, "structured");
    assert!(
        boolean(get(x_squared, "truncated")),
        "the variable holding the result must be abbreviated"
    );
    assert_eq!(
        text(get(x_squared, "canonical")),
        text(get(result, "canonical"))
    );
    let x = variables
        .iter()
        .find(|v| text(get(v, "name")) == "x")
        .expect("x");
    let x_structured = get(x, "structured");
    assert!(opt(x_structured, "truncated").is_none());
    assert_eq!(text(get(x_structured, "canonical")), "(sym x)");
}

// ---------------------------------------------------------------------------
// FunctionArityMetadataTests / ArityPropertyTests / BuiltinArgumentShapeSweepTests
// ---------------------------------------------------------------------------

#[test]
fn run_function_arity_published_range_is_the_range_the_call_site_enforces() {
    let accepted = run_args_of(&["--eval", "concat([1], [2])", "--json", "--omit-functions"]);
    assert_eq!(
        accepted.exit,
        0,
        "concat with minArity arguments was refused: {}",
        accepted.env.to_string()
    );
    assert!(boolean(get(&accepted.env, "ok")));
    let refused = run_args_of(&["--eval", "concat([1])", "--json", "--omit-functions"]);
    assert_eq!(refused.exit, 1);
    assert!(!boolean(get(&refused.env, "ok")));
    assert_eq!(code(&refused), "InvalidArgument");
    assert_eq!(category(&refused), "TypeMismatch");
}

#[test]
fn run_arity_documented_shorter_or_longer_form_stays_callable() {
    let r = run_script("e()");
    assert_eq!(r.exit, 0, "e() must still evaluate: {}", r.env.to_string());
    assert!(boolean(get(&r.env, "ok")));
}

#[test]
fn run_shape_sweep_audit_det_refusal_names_the_builtin_and_position() {
    let r = run_script("det(1)");
    assert_eq!(r.exit, 1);
    assert!(!boolean(get(&r.env, "ok")));
    assert_eq!(code(&r), "InvalidArgument");
    assert_eq!(category(&r), "TypeMismatch");
    assert!(boolean(get(&r.env, "recoverable")));
    let text = message(&r);
    assert!(text.starts_with("det(): argument 1 must be "), "{text}");
    assert!(text.ends_with("; got Natural."), "{text}");
    assert!(!text.contains("cast"));
}

// ---------------------------------------------------------------------------
// SolveSessionAssumptionTests — the controls whose behaviour is unchanged
// ---------------------------------------------------------------------------

#[test]
fn run_solve_session_assumption_that_admits_every_root_publishes_the_same_set() {
    let r = run_script("x = symbol(\"x\"); assume(x > -10); solve(x^2 == 4, x)");
    assert_eq!(r.exit, 0);
    let st = structured(&r);
    assert_eq!(text(get(field(st, "status"), "value")), "Solved");
    assert_eq!(text(get(field(st, "complete"), "value")), "true");
    let values: Vec<String> = array(get(field(st, "solutions"), "elements"))
        .iter()
        .map(|s| text(get(field(s, "value"), "pretty")).to_string())
        .collect();
    assert_eq!(values, vec!["-2", "2"]);
    assert!(
        array(get(field(st, "solutions"), "elements"))
            .iter()
            .all(|s| array(get(field(s, "conditions"), "elements")).is_empty())
    );
}

#[test]
fn run_solve_session_assumption_on_another_symbol_changes_nothing() {
    let r = run_script("x = symbol(\"x\"); y = symbol(\"y\"); assume(y > 5); solve(x == 3, x)");
    assert_eq!(r.exit, 0);
    let st = structured(&r);
    assert_eq!(text(get(field(st, "status"), "value")), "Solved");
    let values: Vec<String> = array(get(field(st, "solutions"), "elements"))
        .iter()
        .map(|s| text(get(field(s, "value"), "pretty")).to_string())
        .collect();
    assert_eq!(values, vec!["3"]);
    assert!(
        array(get(field(st, "solutions"), "elements"))
            .iter()
            .all(|s| array(get(field(s, "conditions"), "elements")).is_empty())
    );
}

#[test]
fn run_solve_session_without_assumptions_publishes_exactly_what_it_did_before() {
    let r = run_script("x = symbol(\"x\"); solve(x^2 == 4, x)");
    assert_eq!(r.exit, 0);
    let st = structured(&r);
    assert_eq!(text(get(field(st, "status"), "value")), "Solved");
    let values: Vec<String> = array(get(field(st, "solutions"), "elements"))
        .iter()
        .map(|s| text(get(field(s, "value"), "pretty")).to_string())
        .collect();
    assert_eq!(values, vec!["-2", "2"]);
    let complex = run_script("x = symbol(\"x\"); solve(x^2 + 1 == 0, x)");
    assert_eq!(complex.exit, 0);
    let st = structured(&complex);
    let values: Vec<String> = array(get(field(st, "solutions"), "elements"))
        .iter()
        .map(|s| text(get(field(s, "value"), "pretty")).to_string())
        .collect();
    assert_eq!(values, vec!["i", "-i"]);
}
