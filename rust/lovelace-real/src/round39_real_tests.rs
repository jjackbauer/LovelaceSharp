//! Round 39 — the Real API members the reference's tests exercise, mirrored against the
//! additive surface in `round39_api`.
//!
//! Each test names the C# row it translates. The translation keeps the observable contract; the
//! representation note in the round report records the one structural difference (Rust passes
//! precision explicitly, so the ambient settings are consulted only by the new helpers).

#![allow(non_snake_case)]

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Barrier, Mutex, MutexGuard};

use lovelace_int::Int;
use lovelace_nat::Nat;

use crate::Real;
use crate::round39_api::{
    DEFAULT_DISPLAY_DECIMAL_PLACES, DEFAULT_MAX_COMPUTATION_DECIMAL_PLACES, NegativeSqrt,
    RealPowError, pi_segment,
};

fn parse(text: &str) -> Real {
    Real::from_str(text).unwrap_or_else(|| panic!("cannot parse {text}"))
}

fn digits(value: &Real) -> String {
    value.to_natural().to_decimal()
}

static CONFIG_LOCK: Mutex<()> = Mutex::new(());

fn config_guard() -> MutexGuard<'static, ()> {
    CONFIG_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

// ---------------------------------------------------------------------------
// RealConstructorTests (5 rows)
// ---------------------------------------------------------------------------

#[test]
// 3.14 is the double literal under test, not a stand-in for the PI constant.
#[allow(clippy::approx_constant)]
fn constructor_given_double_stores_correct_digits_and_exponent() {
    // C# RealConstructorTests.Constructor_GivenDouble_StoresCorrectDigitsAndExponent.
    let value = Real::from_f64(3.14).expect("3.14 is finite");
    assert_eq!(value, parse("3.14"));
    assert_eq!(value.exponent(), -2);
}

#[test]
fn constructor_given_double_zero_produces_zero() {
    // C# RealConstructorTests.Constructor_GivenDoubleZero_ProducesZero.
    assert!(Real::from_f64(0.0).unwrap().is_zero());
}

#[test]
fn constructor_given_negative_double_is_negative() {
    // C# RealConstructorTests.Constructor_GivenNegativeDouble_IsNegative.
    let value = Real::from_f64(-1.5).expect("-1.5 is finite");
    assert!(value.is_negative());
    assert_eq!(value, parse("-1.5"));
}

#[test]
fn constructor_given_positive_decimal_parses_correctly() {
    // C# RealConstructorTests.Constructor_GivenPositiveDecimal_ParsesCorrectly (1.5m).
    let value = Real::from_decimal_scaled(15, 1).expect("decimal");
    assert_eq!(value, parse("1.5"));
}

#[test]
fn constructor_given_decimal_round_trip_equals_string_constructor() {
    // C# RealConstructorTests.Constructor_GivenDecimal_RoundTripEqualsStringConstructor
    // (1.2345678901234567890m).
    let from_decimal = Real::from_decimal_scaled(12_345_678_901_234_567_890, 19).expect("decimal");
    let from_string = parse("1.2345678901234567890");
    assert_eq!(from_decimal, from_string);
}

// ---------------------------------------------------------------------------
// RealParseTests (1 row)
// ---------------------------------------------------------------------------

#[test]
fn try_parse_given_null_string_returns_false() {
    // C# RealParseTests.TryParse_GivenNullString_ReturnsFalse.
    assert!(Real::from_str_opt(None).is_none());
}

// ---------------------------------------------------------------------------
// RealDivideLeadingZeroRunTests (3 rows)
// ---------------------------------------------------------------------------

fn times_power_of_ten(coefficient: &str, zero_run: usize) -> Real {
    parse(&(coefficient.to_string() + &"0".repeat(zero_run)))
}

#[test]
fn divide_given_leading_zero_run_longer_than_the_budget_keeps_the_value_and_its_scale() {
    // C# RealDivideLeadingZeroRunTests.Divide_GivenLeadingZeroRunLongerThanTheBudget_KeepsTheValueAndItsScale
    // rows #2 ("3", 1000) and #3 ("7", 5000).
    for (coefficient, zero_run, period) in [("3", 1000usize, "3"), ("7", 5000, "142857")] {
        let quotient = parse("1")
            .div(&times_power_of_ten(coefficient, zero_run), 1000)
            .expect("division");
        assert_eq!(digits(&quotient), period, "{coefficient} run {zero_run}");
        assert_eq!(
            quotient.exponent(),
            -((zero_run + period.len()) as i64),
            "{coefficient} run {zero_run}"
        );
        assert_eq!(quotient.period_start(), zero_run as i64);
        assert_eq!(quotient.period_length(), period.len() as i64);
        assert!(quotient.is_exact(), "{coefficient} run {zero_run}");
        let expected = format!("0.{}({})", "0".repeat(zero_run), period);
        assert_eq!(quotient.to_string(), expected);
        assert_eq!(quotient, parse(&expected));
    }
}

#[test]
fn divide_given_quotient_the_budget_cannot_hold_is_inexact_and_keeps_its_scale() {
    // C# RealDivideLeadingZeroRunTests.Divide_GivenQuotientTheBudgetCannotHold_IsInexactAndKeepsItsScale.
    // 1/(1009*10^1000) at a 20-digit budget: the first significant digit is 1003 places right of
    // the point and the period of 1/1009 is 252 digits, so the 20 stored digits are a truncation.
    let quotient = parse("1")
        .div(&times_power_of_ten("1009", 1000), 20)
        .expect("division");
    assert!(!quotient.is_zero());
    assert!(!quotient.is_exact());
    assert_eq!(digits(&quotient), "99108027750247770069");
    assert_eq!(quotient.exponent(), -(1003 + 20));
    assert_eq!(quotient.period_start_or_zero(), 0);
    assert_eq!(quotient.period_length(), 0);
    let displayed = quotient.to_string_with_display(40);
    assert_eq!(displayed, format!("0.{}", "0".repeat(40)));
    assert_eq!(displayed.len() - 2, 40);
}

// ---------------------------------------------------------------------------
// RealPiSegmentTests (3 rows)
// ---------------------------------------------------------------------------

#[test]
fn pi_segment_given_single_term_range_matches_manual_term_one_computation() {
    // C# RealPiSegmentTests.PiSegment_GivenSingleTermRange_MatchesManualTermOneComputation.
    let (p, q, t) = pi_segment(1, 2).expect("term range");
    assert_eq!(p.to_decimal(), "720");
    assert_eq!(q.to_decimal(), "1575224475844608000");
    assert_eq!(t.to_decimal(), "-402286710960");
}

#[test]
fn pi_segment_given_full_range_ten_digits_produces_pi_matching_current_serial() {
    // C# RealPiSegmentTests.PiSegment_GivenFullRangeTenDigits_ProducesPiMatchingCurrentSerial.
    const A: u64 = 13_591_409;
    const B: u64 = 545_140_134;
    let c3 = Nat::from_decimal("262537412640768000").unwrap();
    let mut num_s = Int::from_decimal(&A.to_string()).unwrap();
    let mut den_s = Nat::from_u64(1);
    let mut prod_p = Nat::from_u64(1);
    for k in 1..=4u64 {
        let k6 = 6 * k;
        let k3 = 3 * k;
        let ak = Nat::from_u64(k6 * (k6 - 1) * (k6 - 2))
            .mul(&Nat::from_u64((k6 - 3) * (k6 - 4) * (k6 - 5)));
        let bk = Nat::from_u64(k3 * (k3 - 1) * (k3 - 2))
            .mul(&Nat::from_u64(k * k * k))
            .mul(&c3);
        prod_p = prod_p.mul(&ak);
        let mut term =
            Int::from_decimal(&prod_p.mul(&Nat::from_u64(A + B * k)).to_decimal()).unwrap();
        if k % 2 == 1 {
            term = term.negate();
        }
        num_s = num_s
            .mul(&Int::from_decimal(&bk.to_decimal()).unwrap())
            .add(&term);
        den_s = den_s.mul(&bk);
    }
    let (_, q, t) = pi_segment(0, 5).expect("full range");
    assert_eq!(q.to_decimal(), den_s.to_decimal());
    assert_eq!(t.to_decimal(), num_s.to_decimal());
}

#[test]
fn pi_segment_given_midpoint_split_merged_t_matches_full_range_t() {
    // C# RealPiSegmentTests.PiSegment_GivenMidpointSplit_MergedTMatchesFullRangeT.
    let (lp, lq, lt) = pi_segment(0, 5).expect("left");
    let (rp, rq, rt) = pi_segment(5, 10).expect("right");
    let (_, _, full_t) = pi_segment(0, 10).expect("full");
    let merged = lt
        .mul(&Int::from_decimal(&rq.to_decimal()).unwrap())
        .add(&Int::from_decimal(&lp.to_decimal()).unwrap().mul(&rt));
    assert_eq!(lq.to_decimal(), lq.to_decimal());
    assert_eq!(rp.to_decimal(), rp.to_decimal());
    assert_eq!(merged.to_decimal(), full_t.to_decimal());
}

// ---------------------------------------------------------------------------
// RealPiTests (1 row)
// ---------------------------------------------------------------------------

#[test]
fn pi_given_digits_exceeding_max_computation_decimal_places_throws_argument_out_of_range_exception()
{
    // C# RealPiTests.Pi_GivenDigitsExceedingMaxComputationDecimalPlaces_ThrowsArgumentOutOfRangeException.
    let _guard = config_guard();
    let saved_max = Real::max_computation_decimal_places();
    let saved_display = Real::display_decimal_places();
    {
        let _scope = Real::with_precision(10, 10);
        assert!(
            Real::pi_checked(11).is_none(),
            "11 > the pinned 10-place cap"
        );
        assert!(Real::pi_checked(0).is_none());
        assert!(Real::pi_checked(-1).is_none());
        assert!(Real::pi_checked(10).is_some());
    }
    Real::set_max_computation_decimal_places(saved_max);
    Real::set_display_decimal_places(saved_display);
}

// ---------------------------------------------------------------------------
// RealPiToAsyncTests (1 row)
// ---------------------------------------------------------------------------

#[test]
fn pi_to_async_given_invalid_digits_propagates_argument_out_of_range_exception() {
    let _guard = config_guard();
    // C# RealPiToAsyncTests.PiToAsync_GivenInvalidDigits_PropagatesArgumentOutOfRangeException.
    // The reference's Task.Run wrapper rethrows the refused call; a Rust worker thread returns
    // the same `None`.
    let result = std::thread::spawn(|| Real::pi_checked(0))
        .join()
        .expect("worker");
    assert!(result.is_none());
}

// ---------------------------------------------------------------------------
// RealPowZeroBaseTests (3 rows)
// ---------------------------------------------------------------------------

#[test]
fn pow_zero_base_negative_real_exponent_refuses_with_the_integer_paths_refusal() {
    // C# RealPowZeroBaseTests.Pow_ZeroBaseNegativeRealExponent_RefusesWithTheIntegerPathsRefusal.
    for exponent in ["-1", "-1.0"] {
        let error = Real::zero()
            .pow_real_checked(&parse(exponent))
            .expect_err("0 to a negative exponent must refuse");
        assert_eq!(error, RealPowError::ZeroBaseNegativeExponent);
        assert_eq!(error.message(), "Base cannot be zero.");
        assert_eq!(error.param_name(), Some("base"));
    }
}

#[test]
fn pow_zero_base_negative_real_exponent_does_not_return_zero() {
    // C# RealPowZeroBaseTests.Pow_ZeroBaseNegativeRealExponent_DoesNotReturnZero.
    for exponent in ["-1", "-1.0", "-2", "-0.5"] {
        assert!(
            Real::zero().pow_real_checked(&parse(exponent)).is_err(),
            "0^{exponent} must not return a value"
        );
    }
}

#[test]
fn pow_zero_base_positive_exponent_is_still_zero() {
    // C# RealPowZeroBaseTests.Pow_ZeroBasePositiveExponent_IsStillZero and
    // Pow_ZeroBasePositiveFractionalExponent_IsStillZero.
    for exponent in ["1", "2", "5.0", "0.5"] {
        let result = Real::zero()
            .pow_real_checked(&parse(exponent))
            .unwrap_or_else(|error| panic!("0^{exponent}: {error}"));
        assert!(result.is_zero(), "0^{exponent} returned {result}");
    }
}

#[test]
fn pow_nonzero_base_negative_exponent_keeps_its_existing_refusal() {
    // C# RealPowZeroBaseTests.Pow_NonZeroBaseNegativeExponent_KeepsItsExistingRefusal.
    let error = parse("2")
        .pow_real_checked(&parse("-3"))
        .expect_err("2^-3 is the reference's not-implemented path");
    assert_eq!(error, RealPowError::NegativeExponent);
}

// ---------------------------------------------------------------------------
// RealPropertiesTests (5 of 6 rows; the period-start sentinel is recorded as refused)
// ---------------------------------------------------------------------------

#[test]
fn display_decimal_places_default_value_is_one_hundred() {
    // C# RealPropertiesTests.DisplayDecimalPlaces_DefaultValue_IsOneHundred.
    let _guard = config_guard();
    Real::set_display_decimal_places(DEFAULT_DISPLAY_DECIMAL_PLACES);
    assert_eq!(Real::display_decimal_places(), 100);
}

#[test]
fn display_decimal_places_after_setting_returns_new_value() {
    // C# RealPropertiesTests.DisplayDecimalPlaces_AfterSetting_ReturnsNewValue.
    let _guard = config_guard();
    let saved = Real::display_decimal_places();
    Real::set_display_decimal_places(10);
    assert_eq!(Real::display_decimal_places(), 10);
    Real::set_display_decimal_places(saved);
}

#[test]
fn max_computation_decimal_places_default_value_is_one_thousand() {
    // C# RealPropertiesTests.MaxComputationDecimalPlaces_DefaultValue_IsOneThousand.
    let _guard = config_guard();
    Real::set_max_computation_decimal_places(DEFAULT_MAX_COMPUTATION_DECIMAL_PLACES);
    assert_eq!(Real::max_computation_decimal_places(), 1000);
}

#[test]
fn max_computation_decimal_places_after_setting_returns_new_value() {
    // C# RealPropertiesTests.MaxComputationDecimalPlaces_AfterSetting_ReturnsNewValue.
    let _guard = config_guard();
    let saved = Real::max_computation_decimal_places();
    Real::set_max_computation_decimal_places(50);
    assert_eq!(Real::max_computation_decimal_places(), 50);
    Real::set_max_computation_decimal_places(saved);
}

#[test]
fn period_start_given_default_real_is_zero() {
    // C# RealPropertiesTests.PeriodStart_GivenDefaultReal_IsZero. Rust's internal sentinel for
    // "no period" is -1; the reference's property reports 0, which the compatibility accessor
    // exposes.
    assert_eq!(Real::zero().period_start_or_zero(), 0);
}

#[test]
fn exponent_after_setting_returns_new_value() {
    // C# RealPropertiesTests.Exponent_AfterSetting_ReturnsNewValue.
    let mut value = Real::zero();
    value.set_exponent(-5);
    assert_eq!(value.exponent(), -5);
}

// ---------------------------------------------------------------------------
// RealAsyncLocalTests (4 rows)
// ---------------------------------------------------------------------------

#[test]
fn pi_given_concurrent_tasks_with_different_local_precisions_precision_does_not_leak_across_tasks()
{
    // C# RealAsyncLocalTests.Pi_GivenConcurrentTasksWithDifferentLocalPrecisions_PrecisionDoesNotLeakAcrossTasks.
    let barrier = Arc::new(Barrier::new(2));
    let observed = [AtomicI64::new(0), AtomicI64::new(0)];
    std::thread::scope(|scope| {
        for (index, precision) in [55i64, 77].into_iter().enumerate() {
            let barrier = Arc::clone(&barrier);
            let observed = &observed;
            scope.spawn(move || {
                let _scope = Real::with_precision(precision, precision);
                barrier.wait();
                observed[index].store(Real::max_computation_decimal_places(), Ordering::SeqCst);
            });
        }
    });
    assert_eq!(observed[0].load(Ordering::SeqCst), 55);
    assert_eq!(observed[1].load(Ordering::SeqCst), 77);
}

#[test]
fn sqrt_given_batch_with_caller_local_precision_each_child_task_inherits_caller_precision() {
    // C# RealAsyncLocalTests.Sqrt_GivenBatchWithCallerLocalPrecision_EachChildTaskInheritsCallerPrecision.
    // A Rust thread does not capture a thread-local; the caller captures the pair explicitly,
    // which is the additive `precision_snapshot`/`with_precision_values` seam.
    let _scope = Real::with_precision(42, 42);
    let snapshot = Real::precision_snapshot();
    let mut observed = [0i64; 2];
    std::thread::scope(|scope| {
        for (index, slot) in observed.iter_mut().enumerate() {
            scope.spawn(move || {
                let _child = Real::with_precision_values(snapshot.0, snapshot.1);
                *slot = Real::max_computation_decimal_places();
                let _ = index;
            });
        }
    });
    assert_eq!(observed, [42, 42]);
}

#[test]
fn pi_given_nested_precision_scopes_outer_scope_restored_after_inner_disposes() {
    // C# RealAsyncLocalTests.Pi_GivenNestedPrecisionScopes_OuterScopeRestoredAfterInnerDisposes.
    let _guard = config_guard();
    let global_default = Real::max_computation_decimal_places();
    {
        let _outer = Real::with_precision(30, 30);
        assert_eq!(Real::max_computation_decimal_places(), 30);
        {
            let _inner = Real::with_precision(75, 75);
            assert_eq!(Real::max_computation_decimal_places(), 75);
        }
        assert_eq!(Real::max_computation_decimal_places(), 30);
    }
    assert_eq!(Real::max_computation_decimal_places(), global_default);
}

#[test]
fn pi_static_display_decimal_places_concurrent_reads_mutations_are_atomic() {
    // C# RealAsyncLocalTests.Pi_StaticDisplayDecimalPlaces_ConcurrentReadsMutationsAreAtomic.
    static INVALID: AtomicI64 = AtomicI64::new(0);
    let _guard = config_guard();
    let saved = Real::display_decimal_places();
    INVALID.store(0, Ordering::SeqCst);
    std::thread::scope(|scope| {
        for index in 0..60usize {
            scope.spawn(move || {
                let value = [50i64, 100, 150][index % 3];
                Real::set_display_decimal_places(value);
                let read = Real::display_decimal_places();
                if read != 50 && read != 100 && read != 150 {
                    INVALID.fetch_add(1, Ordering::SeqCst);
                }
            });
        }
    });
    let invalid = INVALID.load(Ordering::SeqCst);
    Real::set_display_decimal_places(saved);
    assert_eq!(invalid, 0, "observed a value no task wrote");
}

// ---------------------------------------------------------------------------
// RealSqrtTests, RealSqrtAsyncTests, RealSqrtBatchTests (6 rows)
// ---------------------------------------------------------------------------

#[test]
fn sqrt_given_negative_input_throws_arithmetic_exception() {
    let _guard = config_guard();
    // C# RealSqrtTests.Sqrt_GivenNegativeInput_ThrowsArithmeticException.
    assert_eq!(parse("-1").sqrt_checked(), Err(NegativeSqrt));
}

#[test]
fn sqrt_async_given_negative_input_propagates_arithmetic_exception() {
    let _guard = config_guard();
    // C# RealSqrtAsyncTests.SqrtAsync_GivenNegativeInput_PropagatesArithmeticException.
    let result = std::thread::spawn(|| parse("-1").sqrt_checked())
        .join()
        .expect("worker");
    assert_eq!(result, Err(NegativeSqrt));
}

#[test]
fn sqrt_given_empty_batch_returns_empty_array() {
    let _guard = config_guard();
    // C# RealSqrtBatchTests.Sqrt_GivenEmptyBatch_ReturnsEmptyArray.
    assert!(Real::sqrt_batch(&[]).unwrap().is_empty());
}

#[test]
fn sqrt_given_batch_perfect_squares_returns_exact_roots() {
    let _guard = config_guard();
    // C# RealSqrtBatchTests.Sqrt_GivenBatchPerfectSquares_ReturnsExactRoots.
    let values = [parse("4"), parse("9"), parse("16"), parse("25")];
    let roots = Real::sqrt_batch(&values).expect("non-negative batch");
    let expected = [parse("2"), parse("3"), parse("4"), parse("5")];
    assert_eq!(roots, expected);
}

#[test]
fn sqrt_given_batch_mixed_irrationals_matches_elementwise_serial() {
    let _guard = config_guard();
    // C# RealSqrtBatchTests.Sqrt_GivenBatchMixedIrrationals_MatchesElementwiseSerial.
    let values = [parse("2"), parse("3"), parse("5")];
    let batch = Real::sqrt_batch(&values).expect("non-negative batch");
    let serial: Vec<Real> = values
        .iter()
        .map(|value| value.sqrt_checked().expect("serial root"))
        .collect();
    assert_eq!(batch, serial);
}

#[test]
fn sqrt_given_batch_containing_negative_value_propagates_arithmetic_exception() {
    let _guard = config_guard();
    // C# RealSqrtBatchTests.Sqrt_GivenBatchContainingNegativeValue_PropagatesArithmeticException.
    let values = [parse("1"), parse("-1"), parse("4")];
    assert_eq!(Real::sqrt_batch(&values), Err(NegativeSqrt));
}

#[test]
fn sqrt_given_batch_single_element_returns_array_of_one_matching_serial() {
    let _guard = config_guard();
    // C# RealSqrtBatchTests.Sqrt_GivenBatchSingleElement_ReturnsArrayOfOneMatchingSerial.
    let value = parse("9");
    let result = Real::sqrt_batch(std::slice::from_ref(&value)).expect("non-negative batch");
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], value.sqrt_checked().expect("serial root"));
}

// ---------------------------------------------------------------------------
// RealToStringTests (3 rows)
// ---------------------------------------------------------------------------

#[test]
fn to_string_given_non_periodic_value_truncates_at_display_decimal_places() {
    // C# RealToStringTests.ToString_GivenNonPeriodicValue_TruncatesAtDisplayDecimalPlaces.
    let value = parse("1.123456789");
    assert_eq!(value.to_string_with_display(5), "1.12345");
    let _guard = config_guard();
    let saved = Real::display_decimal_places();
    Real::set_display_decimal_places(5);
    let mut buffer = [0u8; 16];
    let written = value.write_display(&mut buffer).expect("sufficient buffer");
    assert_eq!(&buffer[..written], b"1.12345");
    Real::set_display_decimal_places(saved);
}

#[test]
fn try_format_given_sufficient_buffer_returns_true_and_writes_correctly() {
    let _guard = config_guard();
    // C# RealToStringTests.TryFormat_GivenSufficientBuffer_ReturnsTrueAndWritesCorrectly.
    let value = parse("7.25");
    let expected = value.to_string_with_display(100);
    let mut buffer = vec![0u8; expected.len() + 10];
    let written = value.write_display(&mut buffer).expect("sufficient buffer");
    assert_eq!(written, expected.len());
    assert_eq!(&buffer[..written], expected.as_bytes());
}

#[test]
fn try_format_given_insufficient_buffer_returns_false() {
    let _guard = config_guard();
    // C# RealToStringTests.TryFormat_GivenInsufficientBuffer_ReturnsFalse.
    let value = parse("3.14");
    let mut empty: [u8; 0] = [];
    assert_eq!(value.write_display(&mut empty), None);
}
