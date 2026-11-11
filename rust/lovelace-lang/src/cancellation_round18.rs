//! Round-18 cancellation ports: the engine observes the caller's deadline INSIDE its kernels.
//!
//! Each test repeats a C# assertion against the same workload, and measures the wall time as well
//! as the failure, because "stopped" without a promptness bound is not the contract: the budget
//! must mean something inside a single statement.
//!
//! Sources (all in `Lovelace.Suite.Tests`/`Lovelace.Run.Tests`):
//!   * `ArrayKernelCancellationTests` — the range reduction, the growing product, the matrix
//!     product, the while loop, the elementwise map, and the in-budget control;
//!   * `CancellationObservationTests` — the Natural power/factorial cases are REFUSED here: their
//!     loops live in `lovelace-nat`, which this round's scope excludes, so they can only be
//!     stopped at the statement boundary (recorded in the round-18 report).

use std::time::{Duration, Instant};

use lovelace_symbolics::{cancel, Rat, SolveOutcome};

use crate::{Engine, Value};

/// Test-side fence: a non-observing implementation FAILS rather than hangs. Far above every budget
/// and far below the un-budgeted cost of the work, so a loaded machine cannot flip the verdict.
const FENCE: Duration = Duration::from_secs(8);

fn code(error: &crate::LangError) -> &'static str {
    error.code
}

/// Runs `script` under `budget` and returns the failure plus the measured wall time. A run that
/// does NOT cancel fails the test with the time it took, never a hung process.
fn expect_cancelled(engine: &mut Engine, script: &str, budget_ms: u64) -> (crate::LangError, Duration) {
    engine.set_cancel_after(Some(Duration::from_millis(budget_ms)));
    let started = Instant::now();
    let outcome = engine.eval(script);
    let wall = started.elapsed();
    let error = match outcome {
        Ok(value) => panic!(
            "cancellation was not observed: '{script}' returned {} after {:.2?} under a {budget_ms} ms budget",
            value.typed(),
            wall
        ),
        Err(error) => error,
    };
    assert!(
        wall < FENCE,
        "cancellation took {:.2?} for '{script}' under a {budget_ms} ms budget (fence {:.0?})",
        wall,
        FENCE
    );
    assert_eq!(code(&error), "Cancelled", "'{script}': the failure is not a cancellation: {error:?}");
    assert_eq!(error.category, "BudgetExceeded");
    assert!(engine.was_cancelled(), "'{script}': the token was not observed");
    (error, wall)
}

// ---------------------------------------------------------------------------
// ArrayKernelCancellationTests
// ---------------------------------------------------------------------------

#[test]
fn sum_over_range_cancels_inside_the_range_and_reduction() {
    // ~3.7 s unbudgeted: the range build alone is a million boxed numbers before it refuses.
    let mut engine = Engine::new();
    let _ = expect_cancelled(&mut engine, "sum(1..10000000)", 100);
}

#[test]
fn product_over_range_cancels_inside_the_growing_product() {
    // ~290 ms unbudgeted: the accumulator reaches 20000!, so the late iterations grow too. The
    // C# row uses 1..200000 (22.5 s); the Rust port bounds the FAILURE cost (a non-observing
    // implementation returns in ~0.3 s instead of running for half a minute) while keeping the
    // work several times the budget.
    let mut engine = Engine::new();
    let _ = expect_cancelled(&mut engine, "prod(1..20000)", 50);
}

#[test]
fn matrix_product_cancels_inside_the_triple_loop() {
    // ~18 s unbudgeted: 64 million boxed multiply-adds.
    let mut engine = Engine::new();
    let _ = expect_cancelled(&mut engine, "matmul(eye(400), eye(400))", 100);
}

#[test]
fn while_loop_cancels_between_iterations() {
    // ~3 s unbudgeted: the body is one cheap statement and the whole loop is ONE statement, so the
    // statement-boundary check alone never sees the deadline. The C# row runs 100M iterations; the
    // Rust port bounds the failure cost with 1M (still 15x the budget).
    let mut engine = Engine::new();
    let _ = expect_cancelled(&mut engine, "i = 0; while (i < 1000000) { i = i + 1 }; i", 200);
}

#[test]
fn elementwise_map_cancels_inside_the_map() {
    // ~0.7 s unbudgeted: two boxed five-hundred-thousand-element vectors and the sum over them.
    let mut engine = Engine::new();
    let _ = expect_cancelled(&mut engine, "a = zeros(1000000); b = a + a; sum(b)", 100);
}

#[test]
fn run_inside_budget_still_returns_the_result() {
    // The negative control: polling changes nothing when the work fits the budget.
    let mut engine = Engine::new();
    engine.set_cancel_after(Some(Duration::from_secs(60)));
    let value = engine.eval("sum(1..1000)").expect("an in-budget run answers");
    assert_eq!(value.display(), "500500");
    assert!(!engine.was_cancelled());
}

#[test]
fn an_unbudgeted_run_is_unchanged() {
    // The objective's hard constraint: with no deadline armed the result is the run it always was,
    // and the token reports nothing.
    let mut engine = Engine::new();
    assert_eq!(engine.cancel_after(), None);
    let value = engine.eval("sum(1..1000)").expect("an un-budgeted run answers");
    assert_eq!(value.display(), "500500");
    assert!(!engine.was_cancelled());
}

#[test]
fn a_deadline_does_not_leak_into_the_next_run() {
    // The token is scoped to one evaluation: the second engine run is un-budgeted even though the
    // first one consumed its deadline.
    let mut engine = Engine::new();
    let (error, _) = expect_cancelled(&mut engine, "sum(1..10000000)", 50);
    assert_eq!(error.message, crate::CANCELLED_MESSAGE);
    // The caller disarms (the runner arms per run); a leftover armed deadline must not exist.
    engine.set_cancel_after(None);
    let value = engine.eval("sum(1..1000)").expect("the next run answers");
    assert_eq!(value.display(), "500500");
    assert!(!engine.was_cancelled());
}

// ---------------------------------------------------------------------------
// Symbolic kernels: expand, simplify, solve, factor, integrate
// ---------------------------------------------------------------------------

#[test]
fn expand_cancels_inside_the_distribution_loop() {
    // (x+1)^200 expansion is ~5 s of distributive multiplication unbudgeted.
    let mut engine = Engine::new();
    engine.eval("x = symbol(\"x\")").expect("symbol");
    let _ = expect_cancelled(&mut engine, "expand((x+1)^200)", 25);
}

#[test]
fn simplify_cancels_inside_the_walk() {
    // A pre-built wide sum: 200k distinct terms. The safe simplifier rebuilds every term through
    // the canonical adder, so the walk is quadratic and cannot finish inside the budget. The value
    // is built WITHOUT a deadline, so the stop belongs to the simplify walk itself.
    let mut engine = Engine::new();
    let terms: Vec<lovelace_symbolics::Expr> = (0..200_000)
        .map(|i| lovelace_symbolics::Expr::pow(
            lovelace_symbolics::Expr::sym("x"),
            lovelace_symbolics::Expr::num(i as i64),
        ))
        .collect();
    engine.set_var("big", Value::Symbolic(lovelace_symbolics::Expr::Add(terms)));
    let _ = expect_cancelled(&mut engine, "simplify(big)", 20);
}

#[test]
fn solve_consults_the_deadline() {
    // The solver's own poll. The Rust solver answers the CLI-scale workloads in about a
    // millisecond, so the evidence here is the kernel's verdict under an armed deadline: it
    // abandons the search (no answer) and the token records the observation. The CLI-scale probe
    // in the round-18 report stops inside the expansion that builds the argument.
    let coeffs = vec![
        Rat::from_i64(-999_999_999_989),
        Rat::zero(),
        Rat::zero(),
        Rat::one(),
    ];
    let _scope = cancel::arm(Some(Instant::now()));
    let outcome = lovelace_symbolics::solve_polynomial(&coeffs);
    assert!(
        matches!(outcome, SolveOutcome::Unevaluated),
        "a solver under an expired deadline must abandon the search, got {outcome:?}"
    );
    assert!(cancel::observed(), "the solver did not consult the deadline token");
}

#[test]
fn symbolic_builtins_refuse_at_the_call_seam_when_the_deadline_is_spent() {
    // Every symbolic builtin checks the token before starting its work (and its inner walk polls
    // too); under an expired deadline it refuses instead of returning a partial answer. The call
    // goes through the builtin seam DIRECTLY so the statement-boundary check cannot answer for it.
    let mut engine = Engine::new();
    let x = engine.eval("symbol(\"x\")").expect("symbol");
    let _scope = cancel::arm(Some(Instant::now()));
    for name in ["factor", "integrate", "simplify", "expand", "solve", "collect", "series"] {
        let args = match name {
            "integrate" | "solve" => vec![x.clone(), x.clone()],
            "collect" | "series" => vec![x.clone(), x.clone()],
            _ => vec![x.clone()],
        };
        let outcome = crate::builtins::call(&mut engine, name, args, 0);
        let error = outcome.err().unwrap_or_else(|| {
            panic!("{name} answered under an expired deadline (the token was not consulted)")
        });
        assert_eq!(error.code, "Cancelled", "{name}: {error:?}");
    }
}

// ---------------------------------------------------------------------------
// Position and state
// ---------------------------------------------------------------------------

#[test]
fn cancellation_names_the_statement_that_was_running() {
    let mut engine = Engine::new();
    // A generous budget: the assertion is about WHICH statement is named, not how fast the stop
    // is, and a loaded test host must not be able to cancel the trivial first statement instead.
    let (error, _) = expect_cancelled(&mut engine, "x = symbol(\"x\"); sum(1..10000000)", 500);
    // The stopped statement starts at 17 ("x = symbol("x"); " is 17 units); the completed
    // prefix ends at the first statement, position 0. Both are published to the caller.
    assert_eq!(error.position, 17);
    assert_eq!(engine.last_completed_position(), 0);
    let timings = engine.operation_timings();
    assert_eq!(timings.len(), 2, "the stopped statement still has its timing entry");
    assert_eq!(timings[0].position, 0);
    assert_eq!(timings[1].position, 17);
    assert_eq!(timings[1].result_kind, "Void");
}

#[test]
fn a_cancelled_run_keeps_the_partial_state_it_committed() {
    let mut engine = Engine::new();
    let _ = expect_cancelled(
        &mut engine,
        "x = symbol(\"x\"); print(\"hello\"); sum(1..10000000)",
        50,
    );
    assert_eq!(engine.take_output(), vec!["hello".to_string()]);
    let names: Vec<String> = engine.variable_values().into_iter().map(|(name, _)| name).collect();
    assert_eq!(names, vec!["x".to_string()]);
}

#[test]
fn the_engine_reports_the_reference_cancellation_message() {
    let mut engine = Engine::new();
    let (error, _) = expect_cancelled(&mut engine, "sum(1..10000000)", 50);
    assert_eq!(error.message, "the evaluation was cancelled by the caller.");
}
