//! Round 36: precbench AccuracyTests and the Real format-provider row.
//!
//! Every test is one deferred census row. The C# assertion is the contract; the Rust surface has
//! no global precision statics, so `SqrtAt(value, frac)` (C#: WithPrecision(frac) then Sqrt) is
//! the explicit `Real::sqrt(digits)` argument, and `Real.One / x` is `div(&x, DEFAULT)` at the
//! ambient default the C# operator uses.

use crate::Real;

fn real(text: &str) -> Real {
    Real::from_str(text).unwrap_or_else(|| panic!("cannot parse real {text}"))
}

fn sqrt_at(value: &str, digits: usize) -> Real {
    real(value).sqrt(digits)
}

fn relative_error(approx: &Real, exact: &Real) -> Real {
    let numerator = approx.sub(exact).abs();
    let denominator = exact.abs();
    numerator
        .div(&denominator, 60)
        .unwrap_or_else(|| panic!("relative error of {approx} against {exact} does not divide"))
}

fn default_digits() -> usize {
    Real::DEFAULT_COMPUTATION_DECIMAL_PLACES
}

fn times(value: &Real, factor: i64) -> Real {
    value.mul(&Real::from_i64(factor))
}

/// AccuracyTests.Sqrt_GivenTwo_AtP8AndP16_ProduceDifferentDigitCounts.
#[test]
fn sqrt_given_two_at_p8_and_p16_produce_different_digit_counts() {
    let p8 = sqrt_at("2", 7);
    let p16 = sqrt_at("2", 15);
    assert_eq!(-7, p8.exp, "8 significant digits == 1 integer + 7 fractional");
    assert_eq!(-15, p16.exp, "16 significant digits == 1 + 15");
}

/// AccuracyTests.Sqrt_GivenIrrational_AtP8_ErrorWithinFloatClass (#0..#2).
#[test]
fn sqrt_given_irrational_at_p8_error_within_float_class() {
    for input in ["2", "3", "5"] {
        let exact = sqrt_at(input, 100);
        let lovelace8 = sqrt_at(input, 7);
        let native_f = input.parse::<f32>().expect("float input").sqrt();

        let err_l = relative_error(&lovelace8, &exact);
        let err_f = relative_error(&real(&format!("{}", native_f as f64)), &exact);

        let tolerance = real("0.000001");
        assert!(
            err_l <= tolerance,
            "{input}: Lovelace8 relative error {err_l} exceeds 1e-6"
        );
        assert!(
            err_f <= tolerance,
            "{input}: float relative error {err_f} exceeds 1e-6"
        );
        assert!(
            err_l <= times(&err_f, 8),
            "{input}: Lovelace8 error {err_l} exceeds 8x float error {err_f}"
        );
    }
}

/// AccuracyTests.Sqrt_GivenIrrational_AtP16_ErrorWithinDoubleClass (#0..#2).
#[test]
fn sqrt_given_irrational_at_p16_error_within_double_class() {
    for input in ["2", "3", "5"] {
        let exact = sqrt_at(input, 100);
        let lovelace16 = sqrt_at(input, 15);
        let native_d = input.parse::<f64>().expect("double input").sqrt();

        let err_l = relative_error(&lovelace16, &exact);
        let err_d = relative_error(&real(&format!("{}", native_d)), &exact);

        let tolerance = real("0.00000000000001");
        assert!(
            err_l <= tolerance,
            "{input}: Lovelace16 relative error {err_l} exceeds 1e-14"
        );
        assert!(
            err_d <= tolerance,
            "{input}: double relative error {err_d} exceeds 1e-14"
        );
        assert!(
            err_l <= times(&err_d, 8),
            "{input}: Lovelace16 error {err_l} exceeds 8x double error {err_d}"
        );
    }
}

/// AccuracyTests.Divide_GivenOneThird_IsExactPeriodic.
#[test]
fn divide_given_one_third_is_exact_periodic() {
    let third = Real::from_i64(1)
        .div(&real("3"), default_digits())
        .expect("1/3 has a quotient");
    assert!(third.period_len > 0, "1/3 must be periodic");
    assert_eq!(1, third.period_len);
    assert_eq!("0.(3)", third.to_string());
}

/// AccuracyTests.Divide_GivenOneSeventh_IsExactPeriodic.
#[test]
fn divide_given_one_seventh_is_exact_periodic() {
    let seventh = Real::from_i64(1)
        .div(&real("7"), default_digits())
        .expect("1/7 has a quotient");
    assert_eq!("0.(142857)", seventh.to_string());
}

/// AccuracyTests.Divide_GivenOneSixth_IsExactMixedPeriod.
#[test]
fn divide_given_one_sixth_is_exact_mixed_period() {
    let sixth = Real::from_i64(1)
        .div(&real("6"), default_digits())
        .expect("1/6 has a quotient");
    assert_eq!("0.1(6)", sixth.to_string());
}

/// AccuracyTests.Add_GivenPointOneAndPointTwo_IsExactlyPointThree.
#[test]
fn add_given_point_one_and_point_two_is_exactly_point_three() {
    let sum = real("0.1").add(&real("0.2"));
    assert_eq!(real("0.3"), sum);
    assert_ne!(0.3f64, 0.1f64 + 0.2f64, "the same operation in double is inexact");
}

/// AccuracyTests.Parse_GivenPointOne_LovelaceExactFloatInexact.
#[test]
fn parse_given_point_one_lovelace_exact_float_inexact() {
    assert_eq!(real("0.1"), real("0.1"));
    assert_ne!(0.1f64, 0.1f32 as f64, "float cannot represent 0.1 exactly");
}

/// RealToStringTests.ToString_WithFormatAndProvider_DelegatesToParamlessToString.
///
/// The reference has one rendering implementation; the format-provider overload delegates to it.
/// In Rust the same two entry points are `Display` (format!) and the inherent `to_string`.
#[test]
fn to_string_with_format_provider_delegates_to_paramless_to_string() {
    for text in ["-7", "3.14", "0.(3)", "1.5", "0.1(6)", "0"] {
        let value = real(text);
        assert_eq!(value.to_string(), format!("{value}"), "{text}: Display differs");
    }
}

fn close(actual: &Real, expected: &str, tolerance: &str) -> bool {
    let difference = actual.sub(&real(expected)).abs();
    difference.cmp(&real(tolerance)) == std::cmp::Ordering::Less
}

/// NegativePowerEvaluationTests.ReciprocalOfARealJustBelowOne_KeepsItsQuotient (#2..#6).
///
/// The C# row evaluates the canonical power expression through the kernel; the Rust surface
/// computes the same reciprocal through the real tier, which is the layer the defect was in.
/// The value is the assertion and it is compared numerically, never through a rendering.
#[test]
fn reciprocal_of_a_real_just_below_one_keeps_its_quotient() {
    let tolerance = "0.00000000000000000001";
    for (literal, expected) in [
        ("0.9", "1.111111111111111111111111111111111111111"),
        ("0.99", "1.010101010101010101010101010101010101010"),
        ("0.9993142073433579945", "1.000686263290967473969590465527195755929"),
        ("0.99862888499828389309965043538706203025", "1.001372997539239477447020588085923996879"),
        ("0.5", "2.0"),
    ] {
        let value = real(literal)
            .pow_int(-1)
            .unwrap_or_else(|| panic!("{literal}^-1 must have a value"));
        assert!(
            close(&value, expected, tolerance),
            "{literal}^-1 = {value}, SymPy says {expected}"
        );
    }
}
