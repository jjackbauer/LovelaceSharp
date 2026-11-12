//! Round 82 — the evalf-precision family, at the LANGUAGE tier.
//!
//! The two `Lovelace.Symbolics.Tests` census rows this module closes are read off the published
//! projection (`structured::project_value`, the same projection the wire publishes), never off a
//! re-derivation:
//!
//! * `Symbolics.EvalfTruncatedConstantExactnessTests/ExactControls_AreUntouched#0` — the exact
//!   controls the truncation repair must not sweep up: `evalf(1/2, 40)` keeps the rational 1/2,
//!   `2+3` stays the exact 5, `sin(0)` is the EXACT symbolic zero (not the unevaluated node), and
//!   `evalf(sqrt(2), 40)` stays inexact with no rational form.
//! * `Symbolics.EvalfWorkingPrecisionHonoursRequestedDecimalsTests/ValuesBelowTheComputationCap_StayInexact#0`
//!   — a magnitude below the computation window is correctly 0 at the requested resolution, but
//!   the route that produced it is a truncation, so the answer must stay INEXACT and publish no
//!   rational form. `evalf(sin(pi(30)), 40)` is the row that was publishing
//!   `0.0000000000000000000000000000005028841971`: the argument is a 30-place truncation of pi,
//!   and the reference FOLDS `sin` of an inexact real constant at the system computation precision
//!   before `evalf` ever sees it, so the value that reaches `evalf` is already the fold's zero.
//!
//! The digit-cap boundary is asserted here too (one place below, at and above the 1000-place cap),
//! because the boundary is a property of the builtin and not of the runner.

use crate::structured;
use crate::{Engine, Value};

fn eval(script: &str) -> Value {
    Engine::new()
        .eval(script)
        .unwrap_or_else(|error| panic!("'{script}' failed: {}", error.message))
}

fn projected(script: &str) -> lovelace_json::Json {
    let mut engine = Engine::new();
    let value = engine
        .eval(script)
        .unwrap_or_else(|error| panic!("'{script}' failed: {}", error.message));
    structured::project_value(&engine, &value, None)
}

fn text<'a>(json: &'a lovelace_json::Json, key: &str) -> Option<&'a str> {
    json.get(key).and_then(lovelace_json::Json::as_str)
}

fn boolean(json: &lovelace_json::Json, key: &str) -> bool {
    matches!(json.get(key), Some(lovelace_json::Json::Bool(true)))
}

fn number(json: &lovelace_json::Json, key: &str) -> i64 {
    match json.get(key) {
        Some(lovelace_json::Json::Num(text)) => text.parse().expect("integer budget"),
        other => panic!("{key} is not a number: {other:?}"),
    }
}

/// The fractional digit count of a decimal rendering — the unit every row here measures in.
fn decimals(rendering: &str) -> usize {
    let bare = rendering.split(' ').next().unwrap_or(rendering);
    match bare.find('.') {
        Some(dot) => bare.len() - dot - 1,
        None => 0,
    }
}

/// Census: `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.EvalfTruncatedConstantExactnessTests/ExactControls_AreUntouched#0`.
#[test]
fn evalf_exact_controls_are_untouched_after_the_precision_repair() {
    // evalf(1/2, 40): the exact half keeps its rational form and its tier.
    let half = eval("evalf(1/2, 40)");
    assert_eq!("Real", half.kind_name());
    let half = projected("evalf(1/2, 40)");
    assert_eq!(Some("0.5"), text(&half, "value"));
    assert!(boolean(&half, "exact"), "1/2 is exact and must stay exact");
    assert_eq!(Some("1"), text(&half, "numerator"));
    assert_eq!(Some("2"), text(&half, "denominator"));

    // 2+3 is integer arithmetic and stays an exact 5.
    let sum = projected("2+3");
    assert!(boolean(&sum, "exact"), "2+3 must stay exact");
    assert_eq!(Some("5"), text(&sum, "value"));

    // sin(0) is the EXACT symbolic zero: the function folds at its defining point, so the node
    // the value carries is the rational 0 (one node, rational domain), never sin(0).
    let sin_zero = eval("sin(0)");
    assert_eq!("Symbolic", sin_zero.kind_name());
    let sin_zero = projected("sin(0)");
    assert!(
        boolean(&sin_zero, "exact"),
        "sin(0) is an exact special point: {sin_zero:?}"
    );
    assert_eq!(
        Some("0"),
        text(&sin_zero, "pretty"),
        "sin(0) must fold to the constant, not cross as the unevaluated node"
    );
    assert_eq!(Some("(rat 0 1)"), text(&sin_zero, "canonical"));
    assert_eq!(Some("rational"), text(&sin_zero, "domain"));
    assert_eq!(1, number(&sin_zero, "nodeCount"));

    // The truncated sqrt(2) stays inexact with no rational form.
    let sqrt_two = projected("evalf(sqrt(2), 40)");
    assert_eq!(
        Some("1.4142135623730950488016887242096980785696"),
        text(&sqrt_two, "value")
    );
    assert!(
        !boolean(&sqrt_two, "exact"),
        "the truncated sqrt(2) must stay inexact"
    );
    assert!(sqrt_two.get("numerator").is_none());
    assert!(sqrt_two.get("denominator").is_none());
}

/// Census: `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.EvalfWorkingPrecisionHonoursRequestedDecimalsTests/ValuesBelowTheComputationCap_StayInexact#0`.
#[test]
fn evalf_values_below_the_computation_cap_stay_inexact_zeros() {
    for expression in [
        "evalf(1/(3*10^1000), 30)",
        "evalf(1/(3*10^1001), 30)",
        "evalf(sin(pi(30)), 40)",
    ] {
        let value = eval(expression);
        assert_eq!("Real", value.kind_name(), "{expression}");
        let projection = projected(expression);
        assert_eq!(
            Some("0"),
            text(&projection, "value"),
            "{expression} is 0 at the requested resolution: {projection:?}"
        );
        assert!(
            !boolean(&projection, "exact"),
            "{expression} is a truncation's zero and must not claim exactness"
        );
        assert!(
            projection.get("numerator").is_none(),
            "{expression} publishes no rational form"
        );
        assert!(
            projection.get("denominator").is_none(),
            "{expression} publishes no rational form"
        );
    }
}

/// The production rule the third row above depends on: `sin` of an INEXACT real constant folds at
/// the system computation precision (30 places until `setprecision` moves it), so a 30-place
/// truncation of pi folds to the zero at that window and the wider `evalf` request can only ever
/// publish that zero. The same fold is what makes the neighbouring `sin(pi(30)/6)` answer
/// `0.4999…9` at 30 places instead of a 40-place reconstruction.
#[test]
fn sin_of_an_inexact_real_constant_folds_at_the_computation_precision() {
    let zero = projected("sin(pi(30))");
    assert_eq!(
        Some(format!("0.{}", "0".repeat(30)).as_str()),
        text(&zero, "pretty"),
        "sin(pi(30)) is the 30-place window's zero: {zero:?}"
    );
    assert!(
        !boolean(&zero, "exact"),
        "a fold of an approximation is an approximation"
    );

    for request in [30usize, 40, 100] {
        let script = format!("evalf(sin(pi(30)), {request})");
        let projection = projected(&script);
        assert_eq!(
            Some("0"),
            text(&projection, "value"),
            "{script} cannot add digits the argument never carried: {projection:?}"
        );
        assert!(!boolean(&projection, "exact"), "{script} claims exactness");
    }

    // The fold keeps the ambient 30 decimal places, so the ratio is 0.4999…9 and not 1/2.
    let sixth = projected("evalf(sin(pi(30)/6), 40)");
    assert_eq!(
        Some("0.499999999999999999999999999999"),
        text(&sixth, "value"),
        "sin(pi(30)/6) folds at 30 places: {sixth:?}"
    );
}

/// The cap boundary at the builtin: one place below (999), AT (1000) and one place above (1001)
/// the 1000-place computation cap. Below and at the cap nothing was dropped, so nothing is
/// claimed; above it the value names the budget that stopped it.
#[test]
fn evalf_digit_cap_boundary_is_at_the_cap_not_below_it() {
    for request in [999usize, 1000] {
        let script = format!("evalf(pi, {request})");
        let projection = projected(&script);
        assert_eq!(
            Some(request),
            text(&projection, "value").map(decimals),
            "{script} is honoured in full"
        );
        assert!(projection.get("truncated").is_none(), "{script}");
        assert!(projection.get("truncationReason").is_none(), "{script}");
        assert!(projection.get("budget").is_none(), "{script}");
    }

    let over = projected("evalf(pi, 1001)");
    assert_eq!(Some(1000), text(&over, "value").map(decimals));
    assert!(boolean(&over, "truncated"), "a 1001-place request drops");
    assert_eq!(Some("digit-cap"), text(&over, "truncationReason"));
    assert_eq!(1000, number(&over, "budget"));

    // The exact answer to an over-cap request dropped nothing and crosses unmarked.
    let exact = projected("evalf(1/2, 5000)");
    assert_eq!(Some("0.5"), text(&exact, "value"));
    assert!(boolean(&exact, "exact"));
    assert!(exact.get("truncated").is_none());
}
