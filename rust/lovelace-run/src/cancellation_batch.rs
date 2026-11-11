//! Round-18 wire ports: the cancelled envelope, its ledger, and the diagnostic that names it.
//!
//! Every test drives the SAME `execute`/`envelope_for` pair `main` uses (through
//! `parity_batch::run_args`), so an assertion cannot pass on a flow the binary does not run.
//!
//! Sources (all in `Lovelace.Run.Tests`):
//!   * `CancellationBudgetTests` — the sum, the matrix product, the in-budget control, the
//!     no-budget control, the machine-readable diagnostic, and the five-attempt verdict check;
//!   * `PrintOutputOnFailureTests.TheCancelledPath` — output/partialOutput/partialVariables.

use super::parity_batch::*;
use lovelace_json::Json;

fn as_f64(j: &Json) -> f64 {
    match j {
        Json::Num(raw) => raw
            .parse::<f64>()
            .unwrap_or_else(|_| panic!("not a number: {j:?}")),
        other => panic!("not a number: {other:?}"),
    }
}

/// The envelope's own elapsed in milliseconds, from the documented duration pair.
fn envelope_ms(env: &Json) -> f64 {
    let elapsed = get(env, "elapsedTime");
    let value = as_f64(get(elapsed, "value"));
    if text(get(elapsed, "unit")) == "s" {
        value * 1000.0
    } else {
        value
    }
}

fn ledger(env: &Json) -> &Json {
    opt(env, "cancellation")
        .unwrap_or_else(|| panic!("the envelope carries no cancellation ledger: {env:?}"))
}

fn diagnostics(env: &Json) -> &[Json] {
    array(get(env, "diagnostics"))
}

fn overrun_diagnostics(env: &Json) -> Vec<String> {
    diagnostics(env)
        .iter()
        .map(|d| text(get(d, "message")).to_string())
        .filter(|m| m.contains("cancellation deadline exceeded"))
        .collect()
}

/// The ledger's verdict, its numbers and its diagnostics must never disagree (C# I-1): `exceeded`
/// is exactly "excess is positive", a negative excess is impossible, and the overrun diagnostic is
/// present exactly when the deadline is reported as exceeded.
fn assert_ledger_consistency(env: &Json, budget: u64) {
    let ledger = ledger(env);
    assert_eq!(number(get(ledger, "budgetMs")), budget as i64);
    let exceeded = boolean(get(ledger, "exceeded"));
    let excess = as_f64(get(ledger, "excessMs"));
    assert!(excess >= 0.0, "a negative excess was reported: {ledger:?}");
    assert_eq!(
        exceeded,
        excess > 0.0,
        "exceeded and excessMs disagree: {ledger:?}"
    );
    assert_eq!(
        overrun_diagnostics(env).len(),
        if exceeded { 1 } else { 0 },
        "the diagnostics and the ledger disagree: {env:?}"
    );
    // The ledger describes THIS run: its clock and the envelope's are the same measurement up to
    // the skew between the stop and the statement's end.
    let ledger_ms = as_f64(get(ledger, "elapsedMs"));
    let total_ms = envelope_ms(env);
    assert!(
        (ledger_ms - total_ms).abs() <= (1.0f64).max(total_ms * 0.01),
        "the ledger's elapsedMs ({ledger_ms}) and the envelope's elapsedTime ({total_ms} ms) are not the same measurement"
    );
}

fn cancelled_run(script: &str, budget_ms: &str) -> RunResult {
    run_args(&[
        "--eval",
        script,
        "--json",
        "--omit-functions",
        "--omit-variables",
        "--cancel-after",
        budget_ms,
    ])
}

// ---------------------------------------------------------------------------
// CancellationBudgetTests
// ---------------------------------------------------------------------------

#[test]
fn run_given_budget_far_below_the_sum_stops_inside_the_kernel_and_says_so() {
    let result = cancelled_run("sum(1..10000000)", "100");
    let env = &result.env;
    assert_eq!(result.exit, 1, "a cancelled run exits 1: {env:?}");
    assert!(!boolean(get(env, "ok")));
    assert_eq!(text(get(env, "code")), "Cancelled");
    assert_eq!(text(get(env, "category")), "BudgetExceeded");
    assert!(boolean(get(env, "recoverable")));
    assert_eq!(
        text(get(env, "message")),
        "the evaluation was cancelled by the caller."
    );
    assert!(
        boolean(get(ledger(env), "stopped")),
        "the deadline did not stop the run: {env:?}"
    );
    assert_ledger_consistency(env, 100);
    // Promptness is read from the LEDGER, which the kernel writes, not from the harness clock.
    assert!(
        as_f64(get(ledger(env), "elapsedMs")) < 2500.0,
        "a 100 ms budget took {} ms on a sum that needs seconds unbudgeted: {env:?}",
        text(get(ledger(env), "elapsedMs"))
    );
}

#[test]
fn run_given_budget_far_below_the_matrix_product_stops_inside_the_kernel_and_says_so() {
    let result = run_args(&[
        "--eval",
        "matmul(eye(400), eye(400))",
        "--json",
        "--omit-functions",
        "--omit-variables",
        "--print-budget",
        "20",
        "--cancel-after",
        "100",
    ]);
    let env = &result.env;
    assert_eq!(result.exit, 1, "a cancelled run exits 1: {env:?}");
    assert!(!boolean(get(env, "ok")));
    assert_eq!(text(get(env, "code")), "Cancelled");
    assert!(
        boolean(get(ledger(env), "stopped")),
        "the deadline did not stop the run: {env:?}"
    );
    assert!(
        as_f64(get(ledger(env), "elapsedMs")) < 2500.0,
        "a 100 ms budget took {} ms on matmul(eye(400), eye(400)): {env:?}",
        text(get(ledger(env), "elapsedMs"))
    );
}

#[test]
fn run_given_work_inside_the_budget_succeeds_normally_and_reports_no_excess() {
    let result = cancelled_run("sum(1..1000)", "60000");
    let env = &result.env;
    assert_eq!(result.exit, 0, "an in-budget run succeeds: {env:?}");
    assert!(boolean(get(env, "ok")));
    assert_eq!(text(get(get(env, "result"), "display")), "500500");
    let ledger = ledger(env);
    assert_eq!(number(get(ledger, "budgetMs")), 60000);
    assert!(
        !boolean(get(ledger, "stopped")),
        "the deadline did not stop it: {ledger:?}"
    );
    assert!(
        !boolean(get(ledger, "exceeded")),
        "the run did not exceed its budget: {ledger:?}"
    );
    assert_eq!(as_f64(get(ledger, "excessMs")), 0.0);
}

#[test]
fn run_given_no_budget_carries_no_cancellation_block_at_all() {
    let result = run_args(&[
        "--eval",
        "sum(1..1000)",
        "--json",
        "--omit-functions",
        "--omit-variables",
    ]);
    let env = &result.env;
    assert_eq!(result.exit, 0);
    assert!(boolean(get(env, "ok")));
    assert!(
        opt(env, "cancellation").is_none(),
        "an un-budgeted run carries no ledger: {env:?}"
    );
    assert!(
        opt(env, "diagnostics").is_none(),
        "an un-budgeted run carries no diagnostics: {env:?}"
    );
}

#[test]
fn run_given_exceeded_deadline_carries_a_machine_readable_diagnostic() {
    let result = cancelled_run("sum(1..10000000)", "100");
    let env = &result.env;
    assert_eq!(text(get(env, "code")), "Cancelled");
    // The ledger and the diagnostics cannot disagree (checked above); additionally the diagnostic
    // that IS present must be the runner's cancellation entry with position/line/column.
    if boolean(get(ledger(env), "exceeded")) {
        let diagnostics = diagnostics(env);
        assert!(!diagnostics.is_empty());
        let entry = &diagnostics[0];
        assert!(text(get(entry, "message")).contains("cancellation deadline exceeded"));
        assert_eq!(number(get(entry, "position")), 0);
        assert_eq!(number(get(entry, "line")), 1);
        assert_eq!(number(get(entry, "column")), 1);
    } else {
        assert!(diagnostics(env).is_empty());
    }
}

#[test]
fn run_given_a_deadline_reports_its_verdict_consistently() {
    for attempt in 1..=5 {
        let result = cancelled_run("sum(1..10000000)", "100");
        let env = &result.env;
        assert_eq!(result.exit, 1, "attempt {attempt}: {env:?}");
        assert_eq!(text(get(env, "code")), "Cancelled");
        assert!(
            boolean(get(ledger(env), "stopped")),
            "attempt {attempt}: the deadline did not stop the run: {env:?}"
        );
        assert_ledger_consistency(env, 100);
    }
}

// ---------------------------------------------------------------------------
// PrintOutputOnFailureTests.TheCancelledPath
// ---------------------------------------------------------------------------

#[test]
fn the_cancelled_path_keeps_partial_output_and_carries_the_documented_output() {
    let result = cancelled_run(
        "x = symbol(\"x\"); print(\"hello\"); sum(1..10000000)",
        "100",
    );
    let env = &result.env;
    assert_eq!(result.exit, 1);
    assert_eq!(text(get(env, "code")), "Cancelled");
    assert_eq!(text(get(env, "category")), "BudgetExceeded");
    let output: Vec<&str> = array(get(env, "output")).iter().map(text).collect();
    let partial: Vec<&str> = array(get(env, "partialOutput")).iter().map(text).collect();
    assert_eq!(output, vec!["hello"]);
    assert_eq!(partial, output, "the two arrays are the same lines");
    let variables: Vec<&str> = array(get(env, "partialVariables"))
        .iter()
        .map(|v| text(get(v, "name")))
        .collect();
    assert_eq!(variables, vec!["x"], "only the committed prefix crosses");
}

// ---------------------------------------------------------------------------
// The envelope shape, the diagnostic position and the completed-but-late arm
// ---------------------------------------------------------------------------

#[test]
fn a_cancelled_envelope_publishes_exactly_the_reference_key_set() {
    // Measured from the oracle at the pinned commit for the same argv:
    // protocolVersion, symbolicFormatVersion, mathIrVersion, ok, code, category, message,
    // recoverable, diagnostics, elapsed, elapsedTime, timings, output, partialOutput,
    // partialVariables, cancellation, artifacts.
    let result = cancelled_run("sum(1..10000000)", "100");
    let env = &result.env;
    for key in [
        "protocolVersion",
        "symbolicFormatVersion",
        "mathIrVersion",
        "ok",
        "code",
        "category",
        "message",
        "recoverable",
        "diagnostics",
        "elapsed",
        "elapsedTime",
        "timings",
        "output",
        "partialOutput",
        "partialVariables",
        "cancellation",
        "artifacts",
    ] {
        assert!(
            opt(env, key).is_some(),
            "the cancelled envelope lost '{key}': {env:?}"
        );
    }
    // A cancelled run commits nothing that a success envelope publishes.
    for key in ["revision", "result", "variables", "functions"] {
        assert!(
            opt(env, key).is_none(),
            "a cancelled envelope must not carry '{key}': {env:?}"
        );
    }
    assert_eq!(array(get(env, "artifacts")).len(), 0);
}

#[test]
fn the_diagnostic_position_is_the_completed_prefix_on_the_cli_cancellation_row() {
    // RUN-010 (`x = symbol("x"); expand((x+1)^40)`, 5 ms). The reference publishes the position of
    // the last entry of timings[]; measured, it cancels between statements there (position 0) and
    // this runner cancels inside the expansion, so the diagnostic names the completed prefix
    // (position 0) while timings[] still records the stopped statement at 17.
    let result = cancelled_run("x = symbol(\"x\"); expand((x+1)^40)", "5");
    let env = &result.env;
    assert_eq!(text(get(env, "code")), "Cancelled");
    let diagnostics = diagnostics(env);
    assert_eq!(diagnostics.len(), 1, "{env:?}");
    let entry = &diagnostics[0];
    assert!(text(get(entry, "message")).contains("cancellation deadline exceeded"));
    assert_eq!(number(get(entry, "position")), 0);
    assert_eq!(number(get(entry, "line")), 1);
    assert_eq!(number(get(entry, "column")), 1);
    // The stopped statement keeps its timing entry. WHICH statement was running depends on
    // machine load at a 5 ms budget (a scheduling stall can cancel the first statement), so the
    // assertion couples it to the committed prefix instead of assuming a stop point: no committed
    // variable means the first statement was the one that stopped (position 0); a committed `x`
    // means the second one did (position 17).
    let committed = array(get(env, "partialVariables")).len();
    let expected_stop = if committed == 0 { 0 } else { 17 };
    let timings = array(get(env, "timings"));
    assert!(
        !timings.is_empty(),
        "the stopped statement keeps its timing entry: {env:?}"
    );
    let last = &timings[timings.len() - 1];
    assert_eq!(
        number(get(last, "position")),
        expected_stop,
        "the stopped statement: {env:?}"
    );
    assert_eq!(text(get(last, "resultKind")), "Void");
}

#[test]
fn a_run_that_completes_past_its_deadline_is_a_success_with_a_truthful_ledger() {
    // An unpolled kernel (the Natural factorial lives outside this round's scope) can finish after
    // the budget without observing it. The reference never turns a completed evaluation into a
    // cancellation: the envelope stays ok, the ledger says stopped:false/exceeded:true, and the
    // diagnostic says the evaluation completed anyway.
    let result = run_args(&[
        "--eval",
        "200000!",
        "--json",
        "--omit-functions",
        "--omit-variables",
        "--print-budget",
        "5",
        "--cancel-after",
        // Far below the factorial's ~52 ms, and far above any plausible scheduling hiccup between
        // arming the deadline and the first statement's boundary check.
        "20",
    ]);
    let env = &result.env;
    assert_eq!(result.exit, 0, "a completed run is a success: {env:?}");
    assert!(boolean(get(env, "ok")));
    let ledger = ledger(env);
    assert!(!boolean(get(ledger, "stopped")));
    assert!(
        boolean(get(ledger, "exceeded")),
        "the ledger must say it overran: {ledger:?}"
    );
    let diagnostics = diagnostics(env);
    assert_eq!(diagnostics.len(), 1);
    assert!(
        text(get(&diagnostics[0], "message")).contains("the evaluation completed anyway"),
        "the diagnostic must say why a completed run carries an overrun: {env:?}"
    );
}
