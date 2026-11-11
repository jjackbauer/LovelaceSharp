//! Round 23 symbolic-kernel and display-path regressions.
//!
//! These pin the two reachable failures the round-22 builtin sweep found outside its scope:
//! the degree multiplication at lovelace-symbolics/src/lib.rs:497 (a debug overflow) and the
//! display loop over a user-declared dimension (a non-returning walk). The oracle-side record
//! for the solver case is captured verbatim; the display case is measured as a return plus a
//! bounded length because the oracle process itself aborts without an envelope on that input.

use std::sync::mpsc;
use std::time::Duration;

use crate::{Engine, Value};

fn eval(script: &str) -> Value {
    Engine::new()
        .eval(script)
        .unwrap_or_else(|error| panic!("{script}: {}", error.message))
}

fn display(script: &str) -> String {
    eval(script).display()
}

const HUGE_POWER_SOLVE: &str =
    "x = symbol(\"x\"); solve_full((x^2 - 1)^9223372036854775807 == 0, x)";

/// The pinned dotnet oracle's exact result for HUGE_POWER_SOLVE.
const HUGE_POWER_ORACLE: &str = "SolveResult(status: Solved, variable: x, domain: complex, complete: True, completeness: Complete, solutions: [Solution(value: -1, conditions: [], multiplicity: 1, exactness: Exact), Solution(value: 1, conditions: [], multiplicity: 1, exactness: Exact)], families: [], common_conditions: [], represented_count: 2, unrepresented_count: 0, unrepresented_reason: , diagnostics: [])";

#[test]
fn solve_full_of_a_huge_power_of_a_sum_matches_the_oracle_record() {
    assert_eq!(display(HUGE_POWER_SOLVE), HUGE_POWER_ORACLE);
}

#[test]
fn a_monomial_above_the_materialization_bound_matches_the_oracle() {
    assert_eq!(
        display("x = symbol(\"x\"); solve_full(x^1000000 == 0, x)"),
        "SolveResult(status: Solved, variable: x, domain: complex, complete: True, completeness: Complete, solutions: [Solution(value: 0, conditions: [], multiplicity: 1000000, exactness: Exact)], families: [], common_conditions: [], represented_count: 1, unrepresented_count: 0, unrepresented_reason: , diagnostics: [])"
    );
}

#[test]
fn a_monomial_above_i32_is_the_oracles_nosolutions_record() {
    // The reference narrows the exponent to int; above i32::MAX the polynomial becomes a
    // nonzero constant and the record says NoSolutions.
    assert_eq!(
        display("x = symbol(\"x\"); solve_full(x^9223372036854775807 == 0, x)"),
        "SolveResult(status: NoSolutions, variable: x, domain: complex, complete: True, completeness: Complete, solutions: [], families: [], common_conditions: [], represented_count: 0, unrepresented_count: 0, unrepresented_reason: , diagnostics: [Diagnostic(code: solve.no-solutions, category: NoSolution, message: the equation reduces to a nonzero constant., recoverable: True, location: , details: [])])"
    );
}

#[test]
fn collect_of_a_huge_power_of_a_sum_returns_the_unchanged_expression() {
    assert_eq!(
        display("x = symbol(\"x\"); collect((x^2 - 1)^9223372036854775807, x)"),
        "(x^2 - 1)^9223372036854775807"
    );
}

#[test]
fn term_ordering_with_a_huge_degree_does_not_overflow() {
    let rendered = display("x = symbol(\"x\"); (x^2 - 1)^9223372036854775807 + 1");
    assert_eq!(rendered, "(x^2 - 1)^9223372036854775807 + 1");
    assert_eq!(
        display("x = symbol(\"x\"); 1 + (x^2 - 1)^9223372036854775807"),
        "(x^2 - 1)^9223372036854775807 + 1"
    );
}

#[test]
fn solve_full_with_a_coefficient_at_i64_min_returns_instead_of_panicking() {
    // The coefficient is far outside i64, so the point is that the solver RETURNS a well-formed
    // record instead of overflowing. Its positive discriminant now takes the exact-radical path
    // (x^2 + x - 9223372036854775808 has the two algebraic roots below); before that path existed
    // the same input returned Unevaluated, which was honest but less complete than the reference.
    let rendered = display("x = symbol(\"x\"); solve_full(x^2 + x - 9223372036854775808 == 0, x)");
    assert!(
        rendered.starts_with("SolveResult(status: Solved"),
        "{rendered}"
    );
    assert_eq!(
        rendered.matches("Solution(value: 1/2*").count(),
        2,
        "{rendered}"
    );
    assert!(
        rendered.contains("Solution(value: 1/2*(-sqrt(36893488147419103233) - 1)"),
        "{rendered}"
    );
    assert!(
        rendered.contains("Solution(value: 1/2*(sqrt(36893488147419103233) - 1)"),
        "{rendered}"
    );
}

#[test]
fn a_zero_width_huge_dimension_displays_within_a_timeout_and_a_node_budget() {
    // The value legitimately exists (numel = 0) and the DISPLAY is what used to walk 2^63-1
    // empty rows. Run the rendering on a worker and require the main thread to receive it: a
    // hanging display fails this test at recv_timeout instead of blocking the harness.
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let value = Engine::new()
            .eval("zeros(9223372036854775807, 0)")
            .expect("the zero-element shape remains constructible");
        let rendered = value.display();
        let typed = value.typed();
        let numel = Engine::new()
            .eval("numel(zeros(9223372036854775807, 0))")
            .expect("numel")
            .display();
        let _ = sender.send((rendered, typed, numel));
    });
    let (rendered, typed, numel) = receiver
        .recv_timeout(Duration::from_secs(10))
        .expect("the display of zeros(9223372036854775807, 0) did not return");
    assert_eq!(numel, "0");
    assert!(
        rendered.starts_with("[[], [], "),
        "{}",
        &rendered[..rendered.len().min(40)]
    );
    assert!(rendered.ends_with(']'));
    assert!(rendered.contains("..."), "the abbreviation is visible");
    assert!(
        rendered.len() <= 4 * crate::MAX_DISPLAY_NODES + 32,
        "rendering is bounded by the node budget, got {} bytes",
        rendered.len()
    );
    assert!(typed.ends_with("(Array)"));
}

#[test]
fn small_zero_width_arrays_render_exactly_as_before() {
    assert_eq!(display("zeros(2, 0)"), "[[], []]");
    assert_eq!(display("zeros(3, 0)"), "[[], [], []]");
    assert_eq!(display("zeros(2, 3)"), "[[0, 0, 0], [0, 0, 0]]");
}
