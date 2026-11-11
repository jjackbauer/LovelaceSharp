//! Error-taxonomy and failure-envelope ports from Lovelace.Run.Tests.
//!
//! Each test repeats a C# assertion against the runner's own envelope: the seven-member
//! ErrorCategory vocabulary, the diagnostic code that names each class, and the failure-path
//! fields (signature, suggestions/hints/registrySize, print-budget metadata, depth refusals and
//! the timing of the statement that FAILED).

use super::parity_batch::*;
use super::*;
use lovelace_json::Json;

/// Runs a probe on a stack large enough for the recursive interpreter, exactly as the runner's
/// production thread reserves one (the test harness's own threads are far smaller).
fn with_large_stack<T: Send + 'static>(probe: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(probe)
        .expect("probe thread")
        .join()
        .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
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

fn diagnostics_messages(r: &RunResult) -> Vec<String> {
    array(get(&r.env, "diagnostics"))
        .iter()
        .map(|d| text(get(d, "message")).to_string())
        .collect()
}

fn timing(r: &RunResult, index: usize) -> (&Json, i64, &str, bool) {
    let t = &array(get(&r.env, "timings"))[index];
    (
        t,
        number(get(t, "position")),
        text(get(t, "resultKind")),
        boolean(get(t, "hasOutput")),
    )
}

// ---------------------------------------------------------------------------
// ErrorCategory taxonomy: code + category for each class the C# tests exercise
// ---------------------------------------------------------------------------

#[test]
fn unknown_function_is_unsupported_operation_with_the_registrys_answer() {
    let r = envelope("determinant([[1,2],[3,4]])", &[]);
    assert_eq!(r.exit, 1);
    assert_eq!(code(&r), "UnknownFunction");
    assert_eq!(category(&r), "UnsupportedOperation");
    assert!(boolean(get(&r.env, "recoverable")));
    let suggestions: Vec<&str> = array(get(&r.env, "suggestions")).iter().map(|s| text(s)).collect();
    assert!(suggestions.contains(&"det"), "suggestions: {suggestions:?}");
    assert!(number(get(&r.env, "registrySize")) > 0);
    assert!(message(&r).contains("Did you mean"));
    assert!(diagnostics_messages(&r)[0].contains("Did you mean"));
}

#[test]
fn syntax_name_carries_a_hint_and_no_function_suggestion() {
    let r = envelope("range(5)", &[]);
    assert_eq!(code(&r), "UnknownFunction");
    assert_eq!(category(&r), "UnsupportedOperation");
    let hints: Vec<&str> = array(get(&r.env, "hints")).iter().map(|h| text(h)).collect();
    assert!(hints.iter().any(|h| h.contains("1..n")), "hints: {hints:?}");
    assert!(message(&r).contains("1..n"));
    assert!(opt(&r.env, "suggestions").is_none());
}

#[test]
fn a_name_nothing_is_close_to_keeps_the_plain_message() {
    let r = envelope("nosuchfn(1)", &[]);
    assert_eq!(message(&r), "Unknown function 'nosuchfn'.");
    assert!(opt(&r.env, "suggestions").is_none());
    assert!(opt(&r.env, "hints").is_none());
}

#[test]
fn unsupported_operation_has_its_own_category() {
    let r = envelope("2 ^ 0.5", &[]);
    assert_eq!(r.exit, 1);
    assert_eq!(code(&r), "UnsupportedOperation");
    assert_eq!(category(&r), "UnsupportedOperation");
    assert_eq!(message(&r), "Non-integer exponents are not yet supported.");
    assert!(opt(&r.env, "signature").is_none());
}

#[test]
fn division_by_zero_is_a_domain_error() {
    let r = envelope("1/0", &[]);
    assert_eq!(code(&r), "DivisionByZero");
    assert_eq!(category(&r), "DomainError");
    assert_eq!(message(&r), "Cannot divide by zero.");
}

#[test]
fn a_domain_refusal_keeps_invalid_operation_and_domain_error() {
    let r = envelope("x = symbol(\"x\"); solve(x^2 + 1 == 0, x, integer)", &[]);
    assert_eq!(code(&r), "InvalidOperation");
    assert_eq!(category(&r), "DomainError");
    assert_eq!(message(&r), "solve(): currently supports domains real and complex; got integer.");
    assert!(diagnostics_messages(&r)[0].contains("solve()"));
}

#[test]
fn parser_refusal_is_parse_error_with_the_parser_message_and_no_timing() {
    let r = envelope("1+1;\n2+2;\n)", &[]);
    assert_eq!(code(&r), "ParseError");
    assert_eq!(category(&r), "ParseError");
    assert_eq!(
        message(&r),
        "Unexpected token ')' at position 10: expected a number, string, identifier, '[', or '('."
    );
    assert!(boolean(get(&r.env, "recoverable")));
    assert!(array(get(&r.env, "timings")).is_empty());
}

#[test]
fn tokenizer_string_refusal_is_invalid_input_parse_error_at_the_opening_quote() {
    let r = envelope("\"abc", &[]);
    assert_eq!(code(&r), "InvalidInput");
    assert_eq!(category(&r), "ParseError");
    assert_eq!(message(&r), "Unterminated string literal at position 0.");
    let d = &array(get(&r.env, "diagnostics"))[0];
    assert_eq!(number(get(d, "position")), 0);
    assert_eq!(number(get(d, "line")), 1);
    assert_eq!(number(get(d, "column")), 1);
}

// ---------------------------------------------------------------------------
// Call-site signatures
// ---------------------------------------------------------------------------

#[test]
fn arity_refusal_publishes_the_declared_signature() {
    let r = envelope("dot(1)", &[]);
    assert_eq!(code(&r), "InvalidArgument");
    assert_eq!(category(&r), "TypeMismatch");
    assert_eq!(message(&r), "dot(): expected 2 arguments; got 1.");
    let sig = get(&r.env, "signature");
    assert_eq!(text(get(sig, "function")), "dot");
    let params: Vec<&str> = array(get(sig, "parameters")).iter().map(|p| text(p)).collect();
    assert_eq!(params, vec!["a", "b"]);
    assert_eq!(number(get(sig, "expectedMin")), 2);
    assert_eq!(number(get(sig, "expectedMax")), 2);
    assert!(!boolean(get(sig, "variadic")));
    assert_eq!(number(get(sig, "actual")), 1);
}

#[test]
fn arity_counts_agree_with_the_registry_for_every_probed_builtin() {
    for (builtin, script) in [
        ("matmul", "matmul([1, 2])"),
        ("cross", "cross([1, 2, 3])"),
        ("append", "append([1, 2])"),
        ("conv", "conv([1, 2])"),
        ("concat", "concat([1])"),
    ] {
        let info = registry::find(builtin).unwrap_or_else(|| panic!("no registry entry for {builtin}"));
        let r = envelope(script, &[]);
        let sig = get(&r.env, "signature");
        assert_eq!(text(get(sig, "function")), builtin);
        assert_eq!(number(get(sig, "expectedMin")), info.min_arity as i64, "{script}");
        assert_eq!(boolean(get(sig, "variadic")), info.variadic, "{script}");
        assert!(number(get(sig, "actual")) < info.min_arity as i64, "{script}");
    }
}

#[test]
fn shape_refusal_publishes_the_offending_argument() {
    let r = envelope("det(1)", &[]);
    assert_eq!(code(&r), "InvalidArgument");
    assert_eq!(category(&r), "TypeMismatch");
    let sig = get(&r.env, "signature");
    assert_eq!(text(get(sig, "function")), "det");
    assert_eq!(number(get(sig, "position")), 1);
    assert_eq!(text(get(sig, "parameter")), "m");
    assert_eq!(text(get(sig, "expectedShape")), "a square matrix");
    assert_eq!(text(get(sig, "arrivedKind")), "Natural");
}

// ---------------------------------------------------------------------------
// Failure-path timings and print output
// ---------------------------------------------------------------------------

#[test]
fn the_failing_statement_is_still_a_timed_statement() {
    let r = envelope("x = symbol(\"x\"); solve(x^2 + 1 == 0, x, integer)", &[]);
    assert_eq!(r.exit, 1);
    let timings = array(get(&r.env, "timings"));
    assert_eq!(timings.len(), 2);
    let (_, position, kind, has_output) = timing(&r, 0);
    assert_eq!(position, 0);
    assert_eq!(kind, "Symbolic");
    assert!(!has_output);
    let (_, position, kind, has_output) = timing(&r, 1);
    assert_eq!(position, 17);
    assert_eq!(kind, "Void");
    assert!(!has_output);
}

#[test]
fn print_before_a_failure_crosses_in_output_and_marks_the_statement() {
    let r = envelope("print(\"before\"); print(\"also before\"); det(1); print(\"UNREACHED\")", &[]);
    assert_eq!(r.exit, 1);
    let output: Vec<&str> = array(get(&r.env, "output")).iter().map(|s| text(s)).collect();
    assert_eq!(output, vec!["before", "also before"]);
    assert!(!r.stdout.contains("UNREACHED"));
    let timings = array(get(&r.env, "timings"));
    assert_eq!(timings.len(), 3);
    assert!(timing(&r, 0).3);
    assert!(timing(&r, 1).3);
    assert!(!timing(&r, 2).3);
    assert_eq!(timing(&r, 2).2, "Void");
}

// ---------------------------------------------------------------------------
// Print-budget metadata
// ---------------------------------------------------------------------------

#[test]
fn an_over_budget_symbolic_says_so_and_names_the_budget() {
    let r = envelope("x = symbol(\"x\"); x^2", &["--print-budget", "1"]);
    assert_eq!(r.exit, 0);
    let structured = get(get(&r.env, "result"), "structured");
    assert!(boolean(get(structured, "truncated")));
    assert_eq!(text(get(structured, "truncationReason")), "node-budget");
    assert_eq!(number(get(structured, "budget")), 1);
    assert!(text(get(structured, "canonical")).ends_with(" …"));
}

#[test]
fn a_value_within_the_budget_has_no_truncation_fields() {
    let r = envelope("x = symbol(\"x\"); x^2", &[]);
    let structured = get(get(&r.env, "result"), "structured");
    assert!(opt(structured, "truncated").is_none());
    assert!(opt(structured, "truncationReason").is_none());
    assert!(opt(structured, "budget").is_none());
    assert_eq!(text(get(structured, "pretty")), "x^2");
}

// ---------------------------------------------------------------------------
// Depth refusals
// ---------------------------------------------------------------------------

#[test]
fn a_deep_parse_is_a_depth_budget_refusal_not_a_domain_error() {
    with_large_stack(|| {
        let script = format!("{}1{};", "(".repeat(3000), ")".repeat(3000));
        let r = envelope(&script, &[]);
        assert_eq!(code(&r), "DepthExceeded");
        assert_eq!(category(&r), "BudgetExceeded");
        assert!(message(&r).contains("expression nesting depth 257"));
        assert!(message(&r).contains("maximum supported nesting depth of 256"));
        assert!(array(get(&r.env, "timings")).is_empty());
    });
}

#[test]
fn a_deep_flat_chain_is_refused_by_the_parsed_tree_budget() {
    with_large_stack(|| {
        let script = format!("1{};", "-1".repeat(20000));
        let r = envelope(&script, &[]);
        assert_eq!(code(&r), "DepthExceeded");
        assert_eq!(category(&r), "BudgetExceeded");
        assert!(message(&r).contains("expression tree depth 513"));
    });
}

#[test]
fn recursion_answers_at_the_documented_boundary_and_is_refused_past_it() {
    with_large_stack(|| {
        const F: &str = "func f(n) { if (n == 0) { return 0 }; return f(n - 1) }\n";
        let accepted = envelope(&format!("{F}f(85)"), &[]);
        assert_eq!(accepted.exit, 0);
        assert!(boolean(get(&accepted.env, "ok")));

        let refused = envelope(&format!("{F}f(86)"), &[]);
        assert_eq!(refused.exit, 1);
        assert_eq!(code(&refused), "DepthExceeded");
        assert_eq!(category(&refused), "BudgetExceeded");
        assert!(message(&refused).contains("evaluation depth 513"));
        assert!(message(&refused).contains("maximum supported evaluation depth of 512"));
    });
}

#[test]
fn a_live_wrapper_frame_costs_one_call_level() {
    with_large_stack(|| {
        const F: &str = "func f(n) { if (n == 0) { return 0 }; return f(n - 1) }\n";
        let r = envelope(&format!("{F}func g() {{ return f(85) }}\ng()"), &[]);
        assert_eq!(r.exit, 1);
        assert_eq!(code(&r), "DepthExceeded");
    });
}

#[test]
fn a_deep_symbolic_built_by_repetition_is_refused_with_the_symbolic_stage() {
    let r = envelope("e = symbol(\"x\"); for i in 1..1000 { e = abs(e) };", &[]);
    assert_eq!(code(&r), "DepthExceeded");
    assert_eq!(category(&r), "BudgetExceeded");
    assert!(message(&r).contains("symbolic expression depth 257"));
    assert!(message(&r).contains("maximum supported nesting depth of 256"));
}

#[test]
fn a_deep_run_time_value_is_refused_with_its_measured_depth() {
    let script = format!("x = 1{}", "; x = [x]".repeat(1100));
    let r = envelope(&script, &[]);
    assert_eq!(code(&r), "DepthExceeded");
    assert!(message(&r).contains("value depth 1101"));
    assert!(message(&r).contains("maximum supported nesting depth of 1024"));
}

// ---------------------------------------------------------------------------
// Pre-engine failures and the cancelled envelope
// ---------------------------------------------------------------------------

#[test]
fn an_unreadable_script_file_is_file_read_error_type_mismatch() {
    let missing = format!("{}/does-not-exist-round12.ls", plot_dir());
    let r = run_args(&["--file", &missing, "--omit-functions"]);
    assert_eq!(r.exit, 1);
    assert_eq!(code(&r), "FileReadError");
    assert_eq!(category(&r), "TypeMismatch");
    assert!(array(get(&r.env, "diagnostics")).is_empty());
    assert!(array(get(&r.env, "timings")).is_empty());
    assert!(array(get(&r.env, "output")).is_empty());
    assert!(get(&r.env, "elapsedTime").get("unit").is_some());
}

#[test]
fn cancelled_envelope_carries_the_committed_lines_in_both_arrays() {
    let args: Vec<String> = ["--eval", "print(\"hello\"); 1", "--cancel-after", "1"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let opts = parse_args(&args).unwrap();
    let mut engine = Engine::new();
    engine.eval("print(\"hello\"); 1").unwrap();
    let error = lovelace_lang::cancelled_error();
    let env = cancelled_envelope(
        &mut engine,
        &opts,
        "print(\"hello\"); 1",
        &error,
        std::time::Duration::from_millis(5),
        5.0,
    );
    assert_eq!(text(get(&env, "code")), "Cancelled");
    let output: Vec<&str> = array(get(&env, "output")).iter().map(|s| text(s)).collect();
    let partial: Vec<&str> = array(get(&env, "partialOutput")).iter().map(|s| text(s)).collect();
    assert_eq!(output, vec!["hello"]);
    assert_eq!(partial, vec!["hello"]);
}

#[test]
fn pre_engine_envelope_keeps_the_duration_pair_and_empty_arrays() {
    let env = pre_engine_error_envelope(
        "PlotDirectoryError",
        "TypeMismatch",
        "Cannot use plot directory 'x': denied.".to_string(),
    );
    assert_eq!(text(get(&env, "code")), "PlotDirectoryError");
    assert_eq!(text(get(&env, "category")), "TypeMismatch");
    assert!(boolean(get(&env, "recoverable")));
    assert!(array(get(&env, "timings")).is_empty());
    assert!(array(get(&env, "diagnostics")).is_empty());
    assert!(get(&env, "elapsedTime").get("unit").is_some());
}
