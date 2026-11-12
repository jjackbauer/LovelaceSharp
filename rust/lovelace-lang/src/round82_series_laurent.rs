//! Round 82: the series Laurent-tail family.
//!
//! Every test here is one deferred census row of
//! `Lovelace.Symbolics.Tests/SeriesLaurentAndBranchPointTests` whose C# assertion is the contract.
//! The script is the row's own input and the expected value is the oracle's measured answer, probed
//! with `dotnet exec Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll --file <row> --omit-functions`:
//!
//! `series(1/x, x, 0, 2) = x^-1 + O(x^2)`,
//! `series(1/x^2, x, 0, 2) = x^-2 + O(x^2)`,
//! `series(sin(x)/x, x, 0, 3) = 1 - 1/6*x^2 + O(x^3)`,
//! `series(sqrt(abs(x)), x, 0, 2) = sqrt(x)`.
//!
//! A pole is carried by a NEGATIVE leading power (it is a value, not a dropped term), the tail is
//! the truncation order and not the last published power, and a branch point publishes the
//! kink-resolved source function exactly as SymPy 1.14.0 does.

use lovelace_symbolics::Expr as Sym;

use crate::{Engine, Value};

fn eval(script: &str) -> Value {
    let mut engine = Engine::new();
    engine
        .eval(script)
        .unwrap_or_else(|error| panic!("script: {script} failed: {}", error.message))
}

fn symbolic(script: &str) -> Sym {
    match eval(script) {
        Value::Symbolic(expression) => expression,
        other => panic!(
            "script: {script} produced {} instead of a symbolic value",
            other.kind_name()
        ),
    }
}

// ---------------------------------------------------------------------------
// SeriesLaurentAndBranchPointTests/Series_AcrossAPoleOrABranchPoint_PublishesTheExpansion
// census cases #0, #1, #2 and #4 (case #3, series(sqrt(x)), was already mapped)
// ---------------------------------------------------------------------------

/// Census #0: `series(1/x, x, 0, 2)`. The pole is the leading power, not a coefficient, and the
/// tail stays the REQUESTED truncation (O(x^2)); before the fix Rust published `0`.
#[test]
fn series_across_a_pole_keeps_the_laurent_leading_power() {
    let published = symbolic("x = symbol(\"x\"); series(1/x, x, 0, 2)");
    assert_eq!(published.to_string(), "x^-1 + O(x^2)");
    assert_eq!(
        published.canonical(),
        "(add (pow (sym x) (rat -1 1)) (order (sym x) (rat 0 1) (rat 2 1)))"
    );
}

/// Census #1: `series(1/x^2, x, 0, 2)`. The denominator's leading term sits at index 2, beyond a
/// two-coefficient expansion; it still has to be carried, and the published tail is O(x^2) even
/// though the last power is x^-2.
#[test]
fn series_across_a_second_order_pole_keeps_two_negative_powers() {
    let published = symbolic("x = symbol(\"x\"); series(1/x^2, x, 0, 2)");
    assert_eq!(published.to_string(), "x^-2 + O(x^2)");
    assert_eq!(
        published.canonical(),
        "(add (pow (sym x) (rat -2 1)) (order (sym x) (rat 0 1) (rat 2 1)))"
    );
}

/// Census #2: `series(sin(x)/x, x, 0, 3)`. The numerator's x^3 coefficient is invisible to an
/// order-3 expansion of the numerator itself, so the quotient must ask for one more term: without
/// it the published expansion dropped -1/6*x^2 while still claiming O(x^3).
#[test]
fn series_at_a_removable_singularity_keeps_the_hidden_quadratic_term() {
    let published = symbolic("x = symbol(\"x\"); series(sin(x)/x, x, 0, 3)");
    assert_eq!(published.to_string(), "1 - 1/6*x^2 + O(x^3)");
    assert_eq!(
        published.canonical(),
        "(add (rat 1 1) (mul (rat -1 6) (pow (sym x) (rat 2 1))) (order (sym x) (rat 0 1) (rat 3 1)))"
    );
}

/// Census #4: `series(sqrt(abs(x)), x, 0, 2)`. At a branch point there are no value coefficients
/// and no tail to claim, so the KINK-RESOLVED source is published: `sqrt(x)`, not
/// `sqrt(abs(x))` and never a division by `sqrt(0)`.
#[test]
fn series_at_a_branch_point_publishes_the_kink_resolved_source() {
    let published = symbolic("x = symbol(\"x\"); series(sqrt(abs(x)), x, 0, 2)");
    assert_eq!(published.to_string(), "sqrt(x)");
    assert_eq!(published.canonical(), "(pow (sym x) (rat 1 2))");
    let canonical = published.canonical();
    assert!(!canonical.contains("abs"), "the kink survived: {canonical}");
    assert!(
        !canonical.contains("sqrt(0)"),
        "division by zero: {canonical}"
    );
    assert!(!published.to_string().contains("/0"), "division by zero");
}

// ---------------------------------------------------------------------------
// Boundary: the truncation lives exactly at the requested order, and every
// negative exponent the Laurent tail carries is published in ascending order.
// ---------------------------------------------------------------------------

/// The tail's degree is the REQUESTED order, never `leading power + coefficient count`: at order 1
/// a pole truncates at O(x), and at order 3 at O(x^3) with no invented x^2 term.
#[test]
fn series_order_boundary_truncates_at_the_requested_power() {
    let rows: &[(&str, &str)] = &[
        ("series(1/x, x, 0, 1)", "x^-1 + O(x)"),
        ("series(1/x^2, x, 0, 1)", "x^-2 + O(x)"),
        ("series(1/x, x, 0, 3)", "x^-1 + O(x^3)"),
        ("series(1/x^2, x, 0, 3)", "x^-2 + O(x^3)"),
        ("series(sin(x)/x, x, 0, 4)", "1 - 1/6*x^2 + O(x^4)"),
    ];
    for (call, expected) in rows {
        let script = format!("x = symbol(\"x\"); {call}");
        assert_eq!(symbolic(&script).to_string(), *expected, "{call}");
    }
}

/// A Laurent expansion of order n carries every power BELOW n, negative powers included, in
/// ascending order with the O-term last. A Laurent SUM the reference cannot split is published as
/// the source function unchanged (no O-term), which is the oracle's answer for that shape too.
#[test]
fn laurent_negative_exponents_publish_ascending_with_the_requested_tail() {
    let rows: &[(&str, &str)] = &[
        ("series(x^-3, x, 0, 2)", "x^-3 + O(x^2)"),
        ("series((1 + x)/x^2, x, 0, 2)", "x^-2 + x^-1 + O(x^2)"),
        ("series(1/x + 1/x^2, x, 0, 2)", "x^-1 + x^-2"),
    ];
    for (call, expected) in rows {
        let script = format!("x = symbol(\"x\"); {call}");
        assert_eq!(symbolic(&script).to_string(), *expected, "{call}");
    }
}
