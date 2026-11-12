//! Round 82 — the evalf-precision family, at the WIRE.
//!
//! The two `Lovelace.Run.Tests` census rows this module closes are one script each, and both are
//! about the DIGITS the machine payload carries rather than the digits the human rendering shows:
//!
//! * `Run.EvalfDigitCapHonestyTests/ARequestAtTheCap_IsNotMarked#0` — `evalf(pi, 1000)` is AT the
//!   builtin's 1000-place computation cap, so it is honoured in full and must cross UNMARKED. The
//!   value carries 1000 fractional digits and publishes no `truncated`/`truncationReason`/`budget`.
//! * `Run.StructuredRealPrecisionTests/EvalfCountAboveTheAmbientDisplayPrecision_CrossesInFull#0` —
//!   the same script stores 1000 digits at the DEFAULT ambient display precision (100), so the
//!   payload carries 1000 while `result.display` shows 100, and the `_` variable holds the same
//!   value.
//!
//! The boundary rows the cap defines are asserted beside them: one place BELOW the cap (999), AT
//! it (1000) and one place ABOVE it (1001), plus the exact control that is never marked and the
//! in-scope `setprecision` control the cap is not.
//!
//! Every test drives the runner's own `parity_batch::run_args` envelope, so no assertion can pass
//! on a flow the binary does not run.

use super::parity_batch::*;

/// pi's first 60 fractional digits (mpmath, `mp.dps = 1200`).
const PI_FIRST_60: &str = "141592653589793238462643383279502884197169399375105820974944";

/// pi's fractional digits 981-1000 — the tail a 1000-place request must own.
const PI_981_TO_1000: &str = "66111959092164201989";

/// The builtin's decimal-place computation cap, as the descriptor and the wire state it.
const CAP: usize = 1000;

/// One script through the runner's own argv, WITHOUT `--omit-variables`: these rows read the
/// `_` variable beside the result, so the envelope must carry it.
fn run(script: &str) -> RunResult {
    let result = run_args(&["--eval", script, "--omit-functions"]);
    assert_eq!(
        result.exit, 0,
        "'{script}' exited {}: {}",
        result.exit, result.stdout
    );
    assert!(
        boolean(get(&result.env, "ok")),
        "'{script}': {}",
        result.env
    );
    result
}

fn structured(result: &RunResult) -> &lovelace_json::Json {
    get(get(&result.env, "result"), "structured")
}

fn real_value(structured: &lovelace_json::Json) -> &str {
    text(get(structured, "value"))
}

/// The `_` variable's structured payload: the same value, one projection.
fn result_variable(result: &RunResult) -> &lovelace_json::Json {
    let variables = array(get(&result.env, "variables"));
    let entry = variables
        .iter()
        .find(|v| text(get(v, "name")) == "_")
        .unwrap_or_else(|| panic!("no '_' variable in {}", result.env));
    get(entry, "structured")
}

fn assert_unmarked(structured: &lovelace_json::Json, what: &str) {
    assert!(
        opt(structured, "truncated").is_none(),
        "{what}: the cap did not cut anything, so it must not claim a truncation: {structured}"
    );
    assert!(
        opt(structured, "truncationReason").is_none(),
        "{what}: a reason with no truncation: {structured}"
    );
    assert!(
        opt(structured, "budget").is_none(),
        "{what}: a budget with no truncation: {structured}"
    );
}

/// Census: `Lovelace.Run.Tests/Lovelace.Run.Tests.EvalfDigitCapHonestyTests/ARequestAtTheCap_IsNotMarked#0`.
///
/// A request AT the cap is honoured in full: the value carries the 1000 places it asked for, and
/// the clamp notice the over-cap request carries is absent.
#[test]
fn a_request_at_the_cap_is_honoured_in_full_and_unmarked() {
    let result = run("evalf(pi, 1000)");
    let payload = structured(&result);

    let value = real_value(payload);
    assert_eq!(
        CAP,
        decimals(value),
        "evalf(pi, 1000) carries {} fractional digits, not {CAP}: {}",
        decimals(value),
        &value[..value.len().min(120)]
    );
    assert!(
        value.starts_with(&format!("3.{PI_FIRST_60}")),
        "the request must own pi's own leading digits: {}",
        &value[..value.len().min(120)]
    );
    assert!(
        value.ends_with(PI_981_TO_1000),
        "the 1000-place request must own digits 981-1000: {}",
        &value[value.len().saturating_sub(40)..]
    );
    assert!(!boolean(get(payload, "exact")), "pi is not exact");
    assert_unmarked(payload, "evalf(pi, 1000)");

    // The human rendering is bounded by the ambient display precision (100), and the variable
    // holding the same value reports the same 1000 digits: one value, one projection.
    let display = text(get(get(&result.env, "result"), "display"));
    assert_eq!(100, decimals(display), "the display bound is 100 places");
    assert_eq!(
        value,
        real_value(result_variable(&result)),
        "the '_' variable must carry the same digits as the result"
    );
}

/// Census: `Lovelace.Run.Tests/Lovelace.Run.Tests.StructuredRealPrecisionTests/EvalfCountAboveTheAmbientDisplayPrecision_CrossesInFull#0`.
///
/// A count the machine API was asked for is part of the VALUE, not of the display setting: the
/// payload carries all 1000 stored digits even though the display in force shows 100.
#[test]
fn evalf_count_above_the_ambient_display_precision_crosses_in_full() {
    let result = run("evalf(pi, 1000)");
    let payload = structured(&result);

    let value = real_value(payload);
    assert_eq!(CAP, decimals(value), "the payload must carry 1000 places");
    assert!(
        value.starts_with(&format!("3.{PI_FIRST_60}")),
        "the payload opens with pi's first 60 fractional digits"
    );
    assert!(
        value.ends_with(PI_981_TO_1000),
        "the payload closes with pi's digits 981-1000"
    );
    assert_eq!(
        value,
        real_value(result_variable(&result)),
        "the variable in scope crosses identically"
    );
    // No truncation marker: the payload was not cut, it simply carries the value's own digits.
    assert_unmarked(payload, "evalf(pi, 1000) payload");
    assert_unmarked(result_variable(&result), "evalf(pi, 1000) variable");
}

/// The cap boundary itself: one place below, AT, and one place above the 1000-place cap.
///
/// Below and at the cap the request is honoured and nothing is claimed; the single place above it
/// is a request the builtin cannot deliver, so the value says which budget stopped it. A request
/// the builtin CAN honour in full is never marked, which is what makes the marker mean "digits
/// were DROPPED" rather than "the request was above the cap".
#[test]
fn the_digit_cap_boundary_cuts_only_above_the_cap() {
    for (request, expected) in [(999usize, 999usize), (1000, 1000)] {
        let script = format!("evalf(pi, {request})");
        let result = run(&script);
        let payload = structured(&result);
        assert_eq!(
            expected,
            decimals(real_value(payload)),
            "{script}: the count was honoured in full"
        );
        assert_unmarked(payload, &script);
    }

    let over = run("evalf(pi, 1001)");
    let payload = structured(&over);
    assert_eq!(
        CAP,
        decimals(real_value(payload)),
        "one place above the cap publishes the cap's digits"
    );
    assert_eq!(
        lovelace_json::Json::Bool(true),
        *get(payload, "truncated"),
        "a 1001-place request dropped a digit and must say so: {payload}"
    );
    assert_eq!("digit-cap", text(get(payload, "truncationReason")));
    assert_eq!(CAP as i64, number(get(payload, "budget")));
    // The variable holding the cut value reports the same fact.
    let variable = result_variable(&over);
    assert_eq!(lovelace_json::Json::Bool(true), *get(variable, "truncated"));
    assert_eq!("digit-cap", text(get(variable, "truncationReason")));
    assert_eq!(CAP as i64, number(get(variable, "budget")));

    // A periodic value cut at the same boundary reports the same three fields.
    let third = run("evalf(1/3, 1001)");
    let payload = structured(&third);
    assert_eq!(
        format!("0.{}", "3".repeat(CAP)),
        real_value(payload),
        "1/3 at one place above the cap is the cap's threes"
    );
    assert_eq!(lovelace_json::Json::Bool(true), *get(payload, "truncated"));
    assert_eq!("digit-cap", text(get(payload, "truncationReason")));
    assert_eq!(CAP as i64, number(get(payload, "budget")));
}

/// The two controls the marker must never swallow, in the same binary: an EXACT answer to an
/// over-cap request dropped nothing (evalf(1/2, 5000) is 0.5), and a request the engine can serve
/// in scope is not evalf's cap at all (setprecision(5000); sqrt(2) carries 5000 places unmarked).
#[test]
fn the_over_cap_controls_are_never_marked() {
    let exact = run("evalf(1/2, 5000)");
    let payload = structured(&exact);
    assert_eq!("0.5", real_value(payload));
    assert!(boolean(get(payload, "exact")));
    assert_unmarked(payload, "evalf(1/2, 5000)");

    let scope = run("setprecision(5000); sqrt(2)");
    let payload = structured(&scope);
    assert_eq!(
        5000,
        decimals(real_value(payload)),
        "the engine's own scope is not evalf's cap"
    );
    assert_unmarked(payload, "setprecision(5000); sqrt(2)");

    // ... while evalf asked for more than its cap inside that scope IS cut, at the cap.
    let cut = run("setprecision(5000); evalf(sqrt(2), 5000)");
    let payload = structured(&cut);
    assert_eq!(CAP, decimals(real_value(payload)));
    assert_eq!(lovelace_json::Json::Bool(true), *get(payload, "truncated"));
    assert_eq!("digit-cap", text(get(payload, "truncationReason")));
    assert_eq!(CAP as i64, number(get(payload, "budget")));
}
