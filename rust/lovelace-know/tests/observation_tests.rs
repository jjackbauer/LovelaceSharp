//! Stage 3 tests: canonicalization of the runner envelope and plane classification.
//!
//! Mirrors Lovelace.Knowledge.Tests/ObservationTests.cs.

mod common;

use lovelace_know::observation;

#[test]
fn canonicalize_success_real_keeps_kind_and_typed() {
    let stdout = r#"{"ok":true,"result":{"kind":"Real","typed":"0.5 (Real)"},"revision":3,"elapsed":"1 ms","variables":[]}"#;
    let observation = observation::from_runner_output(stdout).expect("the envelope parses");
    assert!(observation.success);
    assert_eq!(observation.sigma, "ok|Real|0.5 (Real)");
    assert_eq!(observation.kind.as_deref(), Some("Real"));
    assert_eq!(observation.typed.as_deref(), Some("0.5 (Real)"));
    assert!(observation.error_message.is_none());
    assert_eq!(observation::plane_sigma(&observation), "Real");
}

#[test]
fn canonicalize_error_keeps_message_only() {
    let stdout = r#"{"ok":false,"message":"Cannot divide by zero.","category":"DivisionByZero","code":"DivisionByZero"}"#;
    let observation = observation::from_runner_output(stdout).expect("the envelope parses");
    assert!(!observation.success);
    assert_eq!(observation.sigma, "err|Cannot divide by zero.");
    assert_eq!(
        observation.error_message.as_deref(),
        Some("Cannot divide by zero.")
    );
    assert_eq!(
        observation::plane_sigma(&observation),
        "err|Cannot divide by zero."
    );
    assert!(observation::is_error(&observation::plane_sigma(
        &observation
    )));
}

#[test]
fn plane_sigma_boolean_is_tagged() {
    let true_result = observation::from_runner_output(
        r#"{"ok":true,"result":{"kind":"Boolean","typed":"True (Boolean)"}}"#,
    )
    .expect("parses");
    let false_result = observation::from_runner_output(
        r#"{"ok":true,"result":{"kind":"Boolean","typed":"False (Boolean)"}}"#,
    )
    .expect("parses");
    assert_eq!(observation::plane_sigma(&true_result), "Boolean:True");
    assert_eq!(observation::plane_sigma(&false_result), "Boolean:False");
}

#[test]
fn plane_sigma_real_is_single_class() {
    let first = observation::from_runner_output(
        r#"{"ok":true,"result":{"kind":"Real","typed":"0.25 (Real)"}}"#,
    )
    .expect("parses");
    let second = observation::from_runner_output(
        r#"{"ok":true,"result":{"kind":"Real","typed":"0.1(6) (Real)"}}"#,
    )
    .expect("parses");
    assert_eq!(
        observation::plane_sigma(&first),
        observation::plane_sigma(&second)
    );
}

#[test]
fn success_without_result_is_void() {
    let observation = observation::from_runner_output(r#"{"ok":true}"#).expect("parses");
    assert_eq!(observation.sigma, "ok|Void|");
    assert_eq!(observation::plane_sigma(&observation), "Void");
}

#[test]
fn noise_is_discarded() {
    let left = observation::from_runner_output(
        r#"{"ok":true,"revision":1,"timings":[{"elapsed":{"unit":"ns","value":5}}],"result":{"kind":"Natural","typed":"1 (Natural)"}}"#,
    )
    .expect("parses");
    let right = observation::from_runner_output(
        r#"{"ok":true,"revision":99,"timings":[],"artifacts":["x"],"result":{"kind":"Natural","typed":"1 (Natural)"}}"#,
    )
    .expect("parses");
    assert_eq!(left.sigma, right.sigma);
    assert_eq!(
        observation::plane_sigma(&left),
        observation::plane_sigma(&right)
    );
}

#[test]
fn malformed_envelope_is_an_error() {
    assert!(observation::from_runner_output("not json").is_err());
    assert!(observation::from_runner_output("").is_err());
}
