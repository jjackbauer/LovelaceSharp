//! Round-19 ports of the budget-refused depth rows in `Lovelace.Run.Tests`.
//!
//! The C# assertions are the contract: every test below drives the runner's own
//! `execute`/`envelope_for` pair (through `parity_batch::run_args`) and repeats the same
//! exit/code/category/message/depth assertions the C# fact or theory row makes. The depth
//! rows were refused in round 12 because the runner either died or answered the wrong
//! boundary; the in-kernel work and the depth guards now reproduce them.

use super::parity_batch::*;
use lovelace_json::Json;

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

fn in_thread<T: Send + 'static>(probe: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(probe)
        .expect("probe thread")
        .join()
        .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
}

fn with_large_stack<T: Send + 'static>(probe: impl FnOnce() -> T + Send + 'static) -> T {
    in_thread(probe)
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

fn structured(r: &RunResult) -> &Json {
    get(get(&r.env, "result"), "structured")
}

fn assert_depth_refusal(r: &RunResult, message_fragment: &str) {
    assert_eq!(r.exit, 1, "a refused depth exits 1: {}", r.env);
    assert!(!boolean(get(&r.env, "ok")));
    assert_eq!(code(r), "DepthExceeded");
    assert_eq!(category(r), "BudgetExceeded");
    assert!(
        boolean(get(&r.env, "recoverable")),
        "the caller can retry shallower"
    );
    assert!(
        message(r).contains(message_fragment),
        "the message must name the refusal ({message_fragment}): {}",
        message(r)
    );
}

// ---------------------------------------------------------------------------
// DepthGuardEnvelopeTests
// ---------------------------------------------------------------------------

#[test]
fn run_depth_guard_deep_inputs_are_typed_refusals_never_process_deaths() {
    let shapes: Vec<(&str, String)> = vec![
        (
            "nested parentheses",
            format!("{}1{};", "(".repeat(3000), ")".repeat(3000)),
        ),
        ("unary minus chain", format!("{}1;", "-".repeat(3000))),
        (
            "nested abs() calls",
            format!("{}1{};", "abs(".repeat(3000), ")".repeat(3000)),
        ),
        (
            "nested (+1) groups",
            format!("{}1{};", "(".repeat(3000), "+1)".repeat(3000)),
        ),
        (
            "nested list literals",
            format!("{}1{};", "[".repeat(3000), "]".repeat(3000)),
        ),
        (
            "nested sqrt() calls",
            format!("{}1{};", "sqrt(".repeat(3000), ")".repeat(3000)),
        ),
        (
            "flat subtraction chain (20000)",
            format!("1{};", "-1".repeat(20000)),
        ),
        (
            "flat addition chain (20000)",
            format!("1{};", "+1".repeat(20000)),
        ),
    ];
    for (what, script) in shapes {
        let r = run_script(&script);
        assert_ne!(r.exit, 0, "{what}: a refused input must exit non-zero");
        assert!(!boolean(get(&r.env, "ok")), "{what}: ok must be false");
        assert_eq!(code(&r), "DepthExceeded", "{what}: {}", r.env);
        assert_eq!(category(&r), "BudgetExceeded", "{what}");
        assert!(
            boolean(get(&r.env, "recoverable")),
            "{what}: the caller can retry shallower"
        );
        let msg = message(&r);
        assert!(
            msg.contains("depth ") && (msg.contains("of 256") || msg.contains("of 512")),
            "{what}: the message must name the measured depth and budget: {msg}"
        );
    }
}

#[test]
fn run_depth_guard_loop_built_expression_depth_is_a_typed_refusal() {
    let r = run_script("e = symbol(\"x\"); for i in 1..1000 { e = abs(e) };");
    assert_ne!(r.exit, 0);
    assert!(!boolean(get(&r.env, "ok")));
    assert_eq!(code(&r), "DepthExceeded");
    assert_eq!(category(&r), "BudgetExceeded");
}

#[test]
fn run_depth_guard_input_at_the_limit_is_accepted() {
    let script = format!("{}1{};", "(".repeat(254), ")".repeat(254));
    let r = run_script(&script);
    assert!(
        boolean(get(&r.env, "ok")),
        "depth 256 is exactly the budget: {}",
        r.env
    );
    assert_eq!(r.exit, 0);
}

#[test]
fn run_depth_guard_input_one_past_the_limit_is_refused() {
    let script = format!("{}1{};", "(".repeat(255), ")".repeat(255));
    let r = run_script(&script);
    assert_ne!(r.exit, 0);
    assert_eq!(code(&r), "DepthExceeded");
    assert!(
        message(&r).contains("257"),
        "the message names depth 257: {}",
        message(&r)
    );
}

#[test]
fn run_depth_guard_flat_chain_at_the_limit_is_accepted() {
    let script = format!("1{};", "-1".repeat(509));
    let r = run_script(&script);
    assert!(
        boolean(get(&r.env, "ok")),
        "a flat chain of depth 512 must be accepted: {}",
        r.env
    );
    assert_eq!(r.exit, 0);
}

#[test]
fn run_depth_guard_flat_chain_one_past_the_limit_is_refused() {
    let script = format!("1{};", "-1".repeat(510));
    let r = run_script(&script);
    assert_ne!(r.exit, 0);
    assert_eq!(code(&r), "DepthExceeded");
    assert!(
        message(&r).contains("513"),
        "the message names tree depth 513: {}",
        message(&r)
    );
}

#[test]
fn run_depth_guard_flat_sum_of_hundreds_of_terms_is_still_accepted() {
    let terms: Vec<String> = (1..=420).map(|i| format!("exp(log(x+{i}))")).collect();
    let script = format!("x = symbol(\"x\"); simplify_full({})", terms.join(" + "));
    let r = run_script(&script);
    assert_eq!(r.exit, 0);
    assert!(
        boolean(get(&r.env, "ok")),
        "a 420-term flat sum must still run: {}",
        r.env
    );
}

#[test]
fn run_depth_guard_wide_program_of_shallow_statements_is_still_accepted() {
    let script = format!("e = 0;{}", "e = e + 1;".repeat(5000));
    let r = run_script(&script);
    assert_eq!(
        r.exit, 0,
        "5000 shallow statements must still run: {}",
        r.env
    );
    assert!(boolean(get(&r.env, "ok")));
    assert_eq!(text(get(get(&r.env, "result"), "display")), "5000");
}

#[test]
fn run_depth_guard_ordinary_nested_arithmetic_is_unchanged() {
    let r = run_script("x = symbol(\"x\"); e = -(-(-(1 + 2*3))) + sqrt(sqrt(16)) + abs(-2); e;");
    assert_eq!(r.exit, 0);
    assert!(
        boolean(get(&r.env, "ok")),
        "ordinary arithmetic must still run: {}",
        r.env
    );
}

// ---------------------------------------------------------------------------
// ValueDepthEnvelopeTests
// ---------------------------------------------------------------------------

fn wrap_script(wraps: usize) -> String {
    let mut s = String::from("x = 1");
    for _ in 0..wraps {
        s.push_str("; x = [x]");
    }
    s
}

fn wrap_script_result(wraps: usize) -> String {
    let mut s = String::from("for i in 1..1 { x = 1");
    for _ in 0..wraps {
        s.push_str("; x = [x]");
    }
    s.push_str("; x }");
    s
}

#[test]
fn run_value_depth_audit_script_is_a_typed_refusal_with_a_well_formed_envelope() {
    let r = run_script(&wrap_script(20000));
    assert_eq!(r.exit, 1);
    assert!(!boolean(get(&r.env, "ok")));
    assert_eq!(code(&r), "DepthExceeded");
    assert_eq!(category(&r), "BudgetExceeded");
    assert!(boolean(get(&r.env, "recoverable")));
    assert_eq!(number(get(&r.env, "protocolVersion")), 1);
    assert!(message(&r).contains("value depth 20001"));
    assert!(message(&r).contains("maximum supported nesting depth of 1024"));
}

#[test]
fn run_value_depth_on_the_print_path_is_also_refused() {
    let script = format!("{}; print(x)", wrap_script(20000));
    let r = run_script(&script);
    assert_eq!(r.exit, 1);
    assert!(!boolean(get(&r.env, "ok")));
    assert_eq!(code(&r), "DepthExceeded");
    assert_eq!(category(&r), "BudgetExceeded");
}

#[test]
fn run_value_depth_deep_result_with_no_deep_variable_is_also_refused() {
    let r = run_script(&wrap_script_result(20000));
    assert_eq!(r.exit, 1);
    assert!(!boolean(get(&r.env, "ok")));
    assert_eq!(code(&r), "DepthExceeded");
    assert_eq!(category(&r), "BudgetExceeded");
}

#[test]
fn run_value_depth_at_the_limit_is_accepted() {
    let r = run_script(&wrap_script(1023));
    assert!(
        boolean(get(&r.env, "ok")),
        "value depth 1024 is the budget: {}",
        r.env
    );
    assert_eq!(r.exit, 0);
    assert!(opt(&r.env, "code").is_none());
}

#[test]
fn run_value_depth_one_past_the_limit_is_refused() {
    let r = run_script(&wrap_script(1024));
    assert_ne!(r.exit, 0);
    assert!(!boolean(get(&r.env, "ok")));
    assert_eq!(code(&r), "DepthExceeded");
    assert_eq!(category(&r), "BudgetExceeded");
    assert!(message(&r).contains("value depth 1025"), "{}", message(&r));
}

#[test]
fn run_value_depth_increasing_wrap_counts_answer_or_refuse_never_die() {
    for wraps in [1usize, 2, 8, 64, 512, 1023, 1024, 1025, 4096, 20000] {
        let r = run_script(&wrap_script(wraps));
        let depth = wraps + 1;
        if depth <= 1024 {
            assert!(
                boolean(get(&r.env, "ok")),
                "{wraps} wraps (depth {depth}) must answer"
            );
            assert_eq!(r.exit, 0);
        } else {
            assert!(
                !boolean(get(&r.env, "ok")),
                "{wraps} wraps (depth {depth}) must be refused"
            );
            assert_eq!(r.exit, 1);
            assert_eq!(code(&r), "DepthExceeded");
            assert_eq!(category(&r), "BudgetExceeded");
            assert!(boolean(get(&r.env, "recoverable")));
        }
    }
}

#[test]
fn run_value_depth_legitimate_values_are_not_refused() {
    let wide = run_script("v = 1..20000; v[19999]");
    assert_eq!(wide.exit, 0);
    assert!(
        boolean(get(&wide.env, "ok")),
        "a 20000-element flat vector must run"
    );
    let nested = run_script("x = 1; for i in 1..20 { x = [x] }; x");
    assert_eq!(nested.exit, 0);
    assert!(boolean(get(&nested.env, "ok")));
    assert_eq!(text(get(structured(&nested), "kind")), "Array");
}

// ---------------------------------------------------------------------------
// RecursionDepthEnvelopeTests
// ---------------------------------------------------------------------------

const RECURSION_FUNCTION: &str = "func f(n) { if (n == 0) { return 0 }; return f(n - 1) }\n";

fn recursion(depth: i64) -> String {
    format!("{RECURSION_FUNCTION}f({depth})")
}

#[test]
fn run_recursion_depth_deep_recursion_exits_nonzero_with_a_well_formed_envelope() {
    let r = run_script(&recursion(448));
    assert_eq!(r.exit, 1);
    assert!(!boolean(get(&r.env, "ok")));
    assert_eq!(code(&r), "DepthExceeded");
    assert_eq!(category(&r), "BudgetExceeded");
    assert!(boolean(get(&r.env, "recoverable")));
    assert_eq!(number(get(&r.env, "protocolVersion")), 1);
    assert!(message(&r).contains("exceeds the maximum supported evaluation depth of 512"));
    assert!(message(&r).contains("evaluation depth "));
}

#[test]
fn run_recursion_depth_boundary_answers_below_the_budget_and_refuses_above_it() {
    let accepted = run_script(&recursion(64));
    assert_eq!(accepted.exit, 0);
    assert!(boolean(get(&accepted.env, "ok")), "f(64) must still run");
    assert!(opt(&accepted.env, "code").is_none());

    let refused = run_script(&recursion(200));
    assert_eq!(refused.exit, 1);
    assert!(!boolean(get(&refused.env, "ok")), "f(200) must be refused");
    assert_eq!(code(&refused), "DepthExceeded");
    assert_eq!(category(&refused), "BudgetExceeded");
    assert!(boolean(get(&refused.env, "recoverable")));
}

#[test]
fn run_recursion_depth_increasing_depths_answer_or_refuse_never_die() {
    for depth in [0i64, 1, 8, 64, 85, 86, 200, 432, 433, 448, 2000, 100000] {
        let r = run_script(&recursion(depth));
        if depth <= 85 {
            assert!(
                boolean(get(&r.env, "ok")),
                "depth {depth} is within the budget and must answer"
            );
            assert_eq!(r.exit, 0);
        } else {
            assert!(
                !boolean(get(&r.env, "ok")),
                "depth {depth} is past the budget and must be refused"
            );
            assert_eq!(r.exit, 1);
            assert_eq!(code(&r), "DepthExceeded");
            assert_eq!(category(&r), "BudgetExceeded");
            assert!(boolean(get(&r.env, "recoverable")));
        }
    }
}

// ---------------------------------------------------------------------------
// RecursionBudgetCompositionEnvelopeTests
// ---------------------------------------------------------------------------

const DEFINITION: &str = "func f(n) { if (n == 0) { return 0 }; return f(n - 1) }\n";

#[test]
fn run_recursion_budget_preceding_sibling_does_not_spend_the_call_budget() {
    let script = format!("{DEFINITION}{{ 1; f(85) }}");
    let r = run_script(&script);
    assert_eq!(r.exit, 0);
    assert!(boolean(get(&r.env, "ok")), "m02 must answer");
    assert!(opt(&r.env, "code").is_none());
    assert_eq!(text(get(get(&r.env, "result"), "display")), "0");
}

#[test]
fn run_recursion_budget_both_block_orders_answer_at_the_documented_boundary() {
    let first = run_script(&format!("{DEFINITION}{{ f(85); 1 }}"));
    assert_eq!(first.exit, 0);
    assert!(boolean(get(&first.env, "ok")));
    assert_eq!(text(get(get(&first.env, "result"), "display")), "1");

    let second = run_script(&format!("{DEFINITION}{{ 1; f(85) }}"));
    assert_eq!(second.exit, 0);
    assert!(boolean(get(&second.env, "ok")));
    assert_eq!(text(get(get(&second.env, "result"), "display")), "0");
}

fn measured_evaluation_depth(r: &RunResult) -> i64 {
    let msg = message(r);
    let at = msg
        .find("evaluation depth ")
        .expect("the message names the evaluation depth")
        + "evaluation depth ".len();
    let tail = &msg[at..];
    let end = tail.find(' ').expect("a space terminates the number");
    tail[..end].parse().expect("the measured depth is a number")
}

#[test]
fn run_recursion_budget_refusal_reports_the_same_depth_in_both_block_orders() {
    let top = run_script(&recursion(86));
    assert_eq!(top.exit, 1);
    assert_depth_refusal(
        &top,
        "exceeds the maximum supported evaluation depth of 512",
    );
    let top_depth = measured_evaluation_depth(&top);
    assert!(
        top_depth > 512,
        "the measured depth must be past the budget, was {top_depth}"
    );

    let block = run_script(&format!("{DEFINITION}{{ 1; f(86) }}"));
    assert_eq!(block.exit, 1);
    assert_depth_refusal(
        &block,
        "exceeds the maximum supported evaluation depth of 512",
    );
    assert_eq!(top_depth, measured_evaluation_depth(&block));
}

#[test]
fn run_recursion_budget_live_wrapper_frame_still_refuses_the_same_call() {
    let script = format!("{DEFINITION}func g() {{ return f(85) }}\ng()");
    let r = run_script(&script);
    assert_eq!(r.exit, 1);
    assert_depth_refusal(&r, "exceeds the maximum supported evaluation depth of 512");
}
