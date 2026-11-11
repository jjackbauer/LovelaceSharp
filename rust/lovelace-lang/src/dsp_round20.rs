//! Round-20 DSP builtins: the eight C# signal constructors and sample-domain operations.
//!
//! The oracle's typed envelope is the contract. Exact sequences are asserted digit for digit;
//! a value whose magnitude is wider than the display bound is asserted against the oracle's
//! published 100 fractional digits, which is the observable both runners publish.

use crate::{Engine, LangError, Value, structured};

fn eval(source: &str) -> Result<Value, LangError> {
    let mut engine = Engine::new();
    engine.eval(source)
}

fn typed(source: &str) -> String {
    let value = eval(source).unwrap_or_else(|error| panic!("{source}: {}", error.message));
    value.typed()
}

fn error(source: &str) -> LangError {
    eval(source).expect_err(&format!("{source} must be refused"))
}

fn structured_elements(source: &str) -> Vec<lovelace_json::Json> {
    let mut engine = Engine::new();
    let value = engine
        .eval(source)
        .unwrap_or_else(|error| panic!("{source}: {}", error.message));
    let projected = structured::project_value(&engine, &value, None);
    projected
        .get("elements")
        .and_then(|e| e.as_arr())
        .unwrap_or_else(|| panic!("{source}: no elements in {projected:?}"))
        .to_vec()
}

fn element_field(source: &str, index: usize, field: &str) -> String {
    let elements = structured_elements(source);
    elements[index]
        .get(field)
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("{source}: element {index} has no string {field}"))
        .to_string()
}

fn element_exact(source: &str, index: usize) -> bool {
    let elements = structured_elements(source);
    matches!(
        elements[index].get("exact"),
        Some(lovelace_json::Json::Bool(true))
    )
}

// ---------------------------------------------------------------------------
// impulse / step
// ---------------------------------------------------------------------------

#[test]
fn impulse_given_zero_is_the_empty_vector() {
    assert_eq!(typed("impulse(0)"), "[] (Vector)");
}

#[test]
fn impulse_given_lengths_one_and_four_are_unit_impulses() {
    assert_eq!(typed("impulse(1)"), "[1] (Vector)");
    assert_eq!(typed("impulse(4)"), "[1, 0, 0, 0] (Vector)");
}

#[test]
fn step_given_lengths_zero_one_and_four_are_unit_steps() {
    assert_eq!(typed("step(0)"), "[] (Vector)");
    assert_eq!(typed("step(1)"), "[1] (Vector)");
    assert_eq!(typed("step(4)"), "[1, 1, 1, 1] (Vector)");
}

#[test]
fn impulse_and_step_accept_an_integral_real_count() {
    assert_eq!(typed("impulse(2.0)"), "[1, 0] (Vector)");
    assert_eq!(typed("step(1.00)"), "[1] (Vector)");
}

// ---------------------------------------------------------------------------
// delay
// ---------------------------------------------------------------------------

#[test]
fn delay_given_positive_zero_and_negative_offsets_shift_causally() {
    assert_eq!(typed("delay([1, 2, 3], 1)"), "[0, 1, 2] (Vector)");
    assert_eq!(typed("delay([1, 2, 3], 0)"), "[1, 2, 3] (Vector)");
    assert_eq!(typed("delay([1, 2, 3], -1)"), "[2, 3, 0] (Vector)");
}

#[test]
fn delay_given_an_offset_beyond_the_signal_is_all_zero() {
    assert_eq!(typed("delay([1, 2, 3], 5)"), "[0, 0, 0] (Vector)");
}

#[test]
fn delay_given_empty_signal_carries_the_sample_range_refusal() {
    let err = error("delay([], 1)");
    assert_eq!(err.code, "InvalidArgument");
    assert_eq!(err.category, "TypeMismatch");
    assert_eq!(err.message, "end must be ≥ begin. (Parameter 'end')");
}

// ---------------------------------------------------------------------------
// movingavg
// ---------------------------------------------------------------------------

#[test]
fn movingavg_given_exact_windows_matches_the_oracle_sequences() {
    assert_eq!(
        typed("movingavg([1, 2, 3, 4], 2)"),
        "[0.5, 1.5, 2.5, 3.5] (Vector)"
    );
    assert_eq!(
        typed("movingavg([1, 2, 3, 4, 5], 3)"),
        "[0.(3), 1, 2, 3, 4] (Vector)"
    );
    assert_eq!(
        typed("movingavg([1, 2, 3], 7)"),
        "[0.(142857), 0.(428571), 0.(857142)] (Vector)"
    );
}

#[test]
fn movingavg_given_empty_or_short_signal_is_empty() {
    assert_eq!(typed("movingavg([], 3)"), "[] (Vector)");
}

#[test]
fn movingavg_given_fractional_samples_keeps_the_fixed_width_period_rendering() {
    // The oracle's fixed-width path reports 0.(8) for 2.5/3 (a malformed-period state);
    // the observable is preserved exactly.
    assert_eq!(typed("movingavg([2.5], 3)"), "[0.(8)] (Vector)");
    assert_eq!(typed("movingavg([0.1], 3)"), "[0.(0)] (Vector)");
}

#[test]
fn movingavg_given_periodic_samples_marks_the_fixed_width_result_inexact() {
    assert!(!element_exact("movingavg([1/3], 3)", 0));
    assert_eq!(typed("movingavg([1/3], 3)"), "[0.(1)] (Vector)");
}

#[test]
fn movingavg_given_precision_above_the_fixed_width_keeps_periodic_exactness() {
    assert!(element_exact("setprecision(50); movingavg([1/3], 3)", 0));
    assert_eq!(
        typed("setprecision(50); movingavg([1/3], 3)"),
        "[0.(1)] (Vector)"
    );
}

// ---------------------------------------------------------------------------
// exponential
// ---------------------------------------------------------------------------

#[test]
fn exponential_given_zero_length_is_empty() {
    assert_eq!(typed("exponential(0.5, 0)"), "[] (Vector)");
}

#[test]
fn exponential_given_zero_base_is_the_exact_constant_signal() {
    assert_eq!(typed("exponential(0, 3)"), "[1, 1, 1] (Vector)");
    assert!(element_exact("exponential(0, 3)", 0));
}

#[test]
fn exponential_given_half_matches_the_oracle_digits() {
    let second = element_field("exponential(0.5, 2)", 1, "re");
    assert!(
        second.starts_with("1.6487212707001281468486507878141635716537760393860552423690659505907263702979744082624533553446847255"),
        "{second}"
    );
    assert!(!element_exact("exponential(0.5, 2)", 1));
    assert!(element_exact("exponential(0.5, 2)", 0));
}

// ---------------------------------------------------------------------------
// cosine
// ---------------------------------------------------------------------------

#[test]
fn cosine_given_quarter_cycle_hits_the_exact_special_angles() {
    assert_eq!(
        typed("cosine(0.25, 0, 8)"),
        "[1, 0, -1, 0, 1, 0, -1, 0] (Vector)"
    );
    assert_eq!(typed("cosine(0.5, 0, 3)"), "[1, -1, 1] (Vector)");
    assert_eq!(typed("cosine(1/6, 0, 4)"), "[1, 0.5, -0.5, -1] (Vector)");
}

#[test]
fn cosine_given_zero_length_is_empty() {
    assert_eq!(typed("cosine(0.25, 0, 0)"), "[] (Vector)");
}

#[test]
fn cosine_given_general_frequency_matches_the_oracle_digits() {
    let second = element_field("cosine(0.1, 0, 2)", 1, "re");
    assert!(
        second.starts_with("0.8090169943749474241022934171828781764430960130108972599434590525276857908971012151389058336936375942"),
        "{second}"
    );
    assert!(!element_exact("cosine(0.1, 0, 2)", 1));
}

// ---------------------------------------------------------------------------
// powerseries
// ---------------------------------------------------------------------------

#[test]
fn powerseries_given_documented_input_matches_the_oracle() {
    assert_eq!(typed("powerseries(2, 0.5, 4)"), "[0, 1, 1, 0.75] (Vector)");
    assert!(element_exact("powerseries(2, 0.5, 4)", 3));
}

#[test]
fn powerseries_given_zero_k_or_zero_a_stays_exact() {
    assert_eq!(typed("powerseries(0, 2, 4)"), "[0, 0, 0, 0] (Vector)");
    assert_eq!(typed("powerseries(2, 0, 4)"), "[0, 0, 0, 0] (Vector)");
    assert_eq!(typed("powerseries(2, 0.5, 0)"), "[] (Vector)");
}

#[test]
fn powerseries_given_periodic_operands_matches_the_fixed_width_provenance() {
    assert_eq!(
        typed("powerseries(2, 1/3, 3)"),
        "[0, 0.(6), 0.(4)] (Vector)"
    );
    assert!(!element_exact("powerseries(2, 1/3, 3)", 1));
    assert!(element_exact("powerseries(2, 1/3, 3)", 0));
}

// ---------------------------------------------------------------------------
// noise
// ---------------------------------------------------------------------------

#[test]
fn noise_given_seed_zero_is_the_documented_deterministic_sequence() {
    assert_eq!(
        typed("noise(1, 0, 3)"),
        "[0.78752594922464908963889065965 + 0.142967033957129734124764188786i, 0.628808168294678177191453648877 + 0.925632821066692909601169416526i, 0.038027993525502486174916686511 + 0.629502856590518655693044202144i] (Vector)"
    );
}

#[test]
fn noise_given_the_short_and_long_default_seed_forms_agree() {
    assert_eq!(typed("noise(1, 0, 3)"), typed("noise(1, 0, 0, 3)"));
}

#[test]
fn noise_given_explicit_seeds_is_reproducible_and_seed_sensitive() {
    assert_eq!(
        typed("noise(1, 0, 1, 1)"),
        "[0.214764391602396626779013717388 + 0.576098805529634664235136319094i] (Vector)"
    );
    assert_eq!(
        typed("noise(1, 0, -1, 2)"),
        "[0.214764391602396626779013717388 + 0.576098805529634664235136319094i, 0.601203539872114947914304333068 + 0.870082843113435301247118939686i] (Vector)"
    );
    assert_ne!(typed("noise(1, 0, 1, 1)"), typed("noise(1, 0, 1, 2)"));
}

#[test]
fn noise_given_zero_samples_is_empty_and_marks_components_inexact() {
    assert_eq!(typed("noise(1, 0, 0)"), "[] (Vector)");
    assert!(!element_exact("noise(1, 0, 1)", 0));
}

#[test]
fn noise_given_explicit_precision_draws_that_many_digits() {
    assert_eq!(
        typed("setprecision(20); noise(1, 0, 2)"),
        "[0.78752594922464908963 + 0.88906596501429670339i, 0.57129734124764188786 + 0.62880816829467817719i] (Vector)"
    );
}

#[test]
fn noise_given_zero_scale_is_the_displacement_signal() {
    assert_eq!(typed("noise(0, 1, 2)"), "[1 + 1i, 1 + 1i] (Vector)");
    assert!(!element_exact("noise(0, 1, 2)", 0));
}

// ---------------------------------------------------------------------------
// Argument contract
// ---------------------------------------------------------------------------

fn assert_argument_refusal(source: &str, expected: &str) {
    let err = error(source);
    assert_eq!(err.code, "InvalidArgument", "{source}: {}", err.message);
    assert_eq!(err.category, "TypeMismatch", "{source}: {}", err.message);
    assert_eq!(err.message, expected, "{source}");
}

#[test]
fn impulse_given_fractional_count_reports_the_exact_integer_contract() {
    assert_argument_refusal(
        "impulse(2.5)",
        "impulse(): argument 1 (n) must be a Natural or Integer that denotes an exact integer; got Real 2.5.",
    );
}

#[test]
fn impulse_given_negative_count_reports_the_non_negative_bound() {
    assert_argument_refusal(
        "impulse(-1)",
        "impulse(): argument 1 (n) must be a Natural or Integer that is a non-negative count (0 or more, at most 9223372036854775807); got -1.",
    );
}

#[test]
fn movingavg_given_zero_or_negative_window_reports_the_positive_bound() {
    assert_argument_refusal(
        "movingavg([1, 2, 3], 0)",
        "movingavg(): argument 2 (w) must be a Natural or Integer that is a positive count (1 or more, at most 9223372036854775807); got 0.",
    );
    assert_argument_refusal(
        "movingavg([1, 2, 3], -1)",
        "movingavg(): argument 2 (w) must be a Natural or Integer that is a positive count (1 or more, at most 9223372036854775807); got -1.",
    );
}

#[test]
fn delay_given_fractional_offset_reports_the_signed_64_bit_contract() {
    assert_argument_refusal(
        "delay([1, 2, 3], 1.5)",
        "delay(): argument 2 (k) must be a Natural or Integer that denotes an exact integer; got Real 1.5.",
    );
}

#[test]
fn cosine_given_negative_count_reports_the_non_negative_bound() {
    assert_argument_refusal(
        "cosine(0.25, 0, -2)",
        "cosine(): argument 3 (n) must be a Natural or Integer that is a non-negative count (0 or more, at most 9223372036854775807); got -2.",
    );
}

#[test]
fn powerseries_given_fractional_count_reports_the_exact_integer_contract() {
    assert_argument_refusal(
        "powerseries(2, 0.5, 2.5)",
        "powerseries(): argument 3 (n) must be a Natural or Integer that denotes an exact integer; got Real 2.5.",
    );
}

#[test]
fn noise_given_out_of_range_seed_reports_the_int32_bound() {
    assert_argument_refusal(
        "noise(1, 0, 2147483648, 2)",
        "noise(): argument 3 (seed) must be a Natural or Integer between -2147483648 and 2147483647; got 2147483648.",
    );
}

#[test]
fn noise_given_two_arguments_publishes_the_declared_arity_signature() {
    let err = error("noise(1, 0)");
    assert_eq!(err.code, "InvalidArgument");
    assert_eq!(err.category, "TypeMismatch");
    assert_eq!(err.message, "noise(): expected 3 to 4 arguments; got 2.");
    let signature = err
        .signature
        .expect("an arity refusal carries its signature");
    assert_eq!(signature.function, "noise");
    assert_eq!(
        signature.parameters,
        Some(vec![
            "scale".to_string(),
            "disp".to_string(),
            "n".to_string(),
            "seed".to_string()
        ])
    );
    assert_eq!(signature.expected_min, Some(3));
    assert_eq!(signature.expected_max, Some(4));
    assert_eq!(signature.actual, Some(2));
}

#[test]
fn impulse_given_more_than_int_max_samples_reports_the_sample_range_refusal() {
    assert_argument_refusal(
        "impulse(2147483648)",
        "The sample range [0, 2147483647] spans more than 2147483647 samples. (Parameter 'end')",
    );
}

#[test]
fn movingavg_given_scalar_signal_reports_the_array_contract() {
    let err = error("movingavg(5, 2)");
    assert_eq!(err.code, "InvalidOperation");
    assert_eq!(err.category, "DomainError");
    assert_eq!(
        err.message,
        "DSP builtins expect an array argument, but got a scalar."
    );
}

#[test]
fn exponential_given_non_numeric_element_reports_the_payload_type() {
    let err = error("exponential([1, [2]], 2)");
    assert_eq!(err.code, "InvalidOperation");
    assert_eq!(err.category, "DomainError");
    assert_eq!(
        err.message,
        "DSP builtins expect numeric/complex elements, but got 'PayloadArray'."
    );
}

#[test]
fn delay_given_too_wide_count_reports_the_value_digits() {
    assert_argument_refusal(
        "delay([1, 2], 999999999999999999999999)",
        "delay(): argument 2 (k) must be a Natural or Integer that fits a signed 64-bit integer (at most 9223372036854775807 in absolute value); got 999999999999999999999999.",
    );
}
