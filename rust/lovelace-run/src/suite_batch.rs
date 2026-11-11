//! Ports from Lovelace.Suite.Tests: the array/function/builtin/statement facts whose Rust
//! behaviour is the same concrete outcome the C# test asserts.

use super::parity_batch::*;
use super::*;

fn eval(script: &str) -> RunResult {
    envelope(script, &[])
}

fn result_display(r: &RunResult) -> &str {
    text(get(get(&r.env, "result"), "display"))
}

fn variable_display<'a>(r: &'a RunResult, name: &str) -> &'a str {
    array(get(&r.env, "variables"))
        .iter()
        .find(|v| text(get(v, "name")) == name)
        .map(|v| text(get(v, "display")))
        .unwrap_or_else(|| panic!("no variable {name}"))
}

#[test]
fn suite_flat_list_produces_a_vector() {
    let r = eval("[1, 2, 3]");
    assert_eq!(r.exit, 0);
    assert_eq!(text(get(get(&r.env, "result"), "kind")), "Vector");
}

#[test]
fn suite_ragged_nested_list_reports_an_error() {
    let r = eval("[[1, 2], [3]]");
    assert_eq!(r.exit, 1);
    assert!(!boolean(get(&r.env, "ok")));
    assert_eq!(text(get(&r.env, "code")), "InvalidOperation");
}

#[test]
fn suite_index_out_of_range_reports_an_error() {
    let r = eval("[[1, 2]][0, 2]");
    assert_eq!(r.exit, 1);
    assert!(!boolean(get(&r.env, "ok")));
    assert_eq!(text(get(&r.env, "code")), "InvalidOperation");
}

#[test]
fn suite_inverse_of_a_singular_matrix_reports_an_error() {
    let r = eval("inv([[1, 0], [0, 0]])");
    assert_eq!(r.exit, 1);
    assert!(!boolean(get(&r.env, "ok")));
    assert_eq!(text(get(&r.env, "code")), "InvalidOperation");
}

#[test]
fn suite_function_parameter_shadows_global_without_changing_it() {
    let r = run_args(&["--eval", "x = 10; func f(x) { x = x + 1 }; f(1); x", "--omit-functions"]);
    assert_eq!(r.exit, 0);
    assert_eq!(result_display(&r), "10");
    assert_eq!(variable_display(&r, "x"), "10");
}

#[test]
fn suite_wrong_function_arity_reports_an_error() {
    let r = eval("func f(a) { a }; f(1, 2)");
    assert_eq!(r.exit, 1);
    assert!(!boolean(get(&r.env, "ok")));
    assert_eq!(text(get(&r.env, "code")), "InvalidOperation");
}

#[test]
fn suite_sqrt_of_a_positive_integer_returns_a_real() {
    let r = eval("sqrt(-(-16))");
    assert_eq!(r.exit, 0);
    assert_eq!(text(get(get(&r.env, "result"), "kind")), "Real");
    assert_eq!(result_display(&r), "4");
}

#[test]
fn suite_pi_with_an_integer_argument_returns_the_requested_digits() {
    let r = eval("pi(-(-10))");
    assert_eq!(r.exit, 0);
    assert_eq!(text(get(get(&r.env, "result"), "kind")), "Real");
    assert_eq!(result_display(&r), "3.1415926535");
}

#[test]
fn suite_pi_with_a_real_argument_is_refused_as_invalid_operation() {
    let r = eval("pi(3.0)");
    assert_eq!(r.exit, 1);
    assert!(!boolean(get(&r.env, "ok")));
    assert_eq!(text(get(&r.env, "code")), "InvalidOperation");
}

#[test]
fn suite_setprecision_with_a_real_argument_is_refused_as_invalid_operation() {
    let r = eval("setprecision(3.0)");
    assert_eq!(r.exit, 1);
    assert!(!boolean(get(&r.env, "ok")));
    assert_eq!(text(get(&r.env, "code")), "InvalidOperation");
}

#[test]
fn suite_setprecision_above_default_allows_pi_to_compute_that_many_digits() {
    let r = eval("setprecision(1100); pi(1100)");
    assert_eq!(r.exit, 0);
    assert_eq!(text(get(get(&r.env, "result"), "kind")), "Real");
    assert_eq!(decimals(result_display(&r)), 1100);
}

#[test]
fn suite_break_outside_a_loop_reports_an_error() {
    let r = eval("break");
    assert_eq!(r.exit, 1);
    assert!(!boolean(get(&r.env, "ok")));
    assert_eq!(text(get(&r.env, "code")), "InvalidOperation");
}

#[test]
fn suite_divide_by_zero_reports_divide_by_zero() {
    let r = eval("5/0");
    assert_eq!(r.exit, 1);
    assert!(!boolean(get(&r.env, "ok")));
    assert_eq!(text(get(&r.env, "code")), "DivisionByZero");
    assert_eq!(text(get(&r.env, "category")), "DomainError");
}

#[test]
fn suite_fresh_engine_includes_the_array_builtins() {
    for name in [
        "sum", "prod", "min", "max", "mean", "norm", "dot", "cross", "matmul", "det",
        "inv", "trace", "zeros", "ones", "eye", "reshape", "shape", "rank", "numel",
        "flatten", "transpose", "squeeze", "concat", "append",
    ] {
        let r = run_args(&["--functions", name]);
        assert_eq!(r.exit, 0);
        assert!(
            array(get(&r.env, "functions")).iter().any(|f| text(get(f, "name")) == name),
            "missing builtin: {name}"
        );
    }
}

#[test]
fn suite_empty_reductions_are_refused() {
    for script in ["max([])", "min([])"] {
        let r = eval(script);
        assert_eq!(r.exit, 1, "{script}");
        assert!(!boolean(get(&r.env, "ok")), "{script}");
        assert_eq!(text(get(&r.env, "code")), "InvalidOperation", "{script}");
    }
}

#[test]
fn suite_incompatible_broadcast_shapes_are_refused() {
    let r = eval("[1, 2] + [1, 2, 3]");
    assert_eq!(r.exit, 1);
    assert!(!boolean(get(&r.env, "ok")));
    assert_eq!(text(get(&r.env, "code")), "InvalidOperation");
}

// ---------------------------------------------------------------------------
// Lovelace.Run.Tests.ParseErrorClassificationTests: the tokenizer's string refusal
// ---------------------------------------------------------------------------

#[test]
fn run_tokenizer_string_refusal_carries_the_parse_error_category() {
    let r = run_args(&["--eval", "\"abc", "--omit-functions", "--omit-variables"]);
    assert_eq!(r.exit, 1);
    assert!(!boolean(get(&r.env, "ok")));
    assert_eq!(text(get(&r.env, "category")), "ParseError");
    assert!(boolean(get(&r.env, "recoverable")));
    assert!(
        text(get(&r.env, "message")).contains("Unterminated string literal"),
        "message: {}",
        text(get(&r.env, "message"))
    );
}
