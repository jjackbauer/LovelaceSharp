//! Ports of the remaining `Lovelace.Real.Tests` cases whose C# assertion this crate reproduces.
//!
//! The tests are in-crate because the C# assertions read the representation the public Rust API
//! keeps private (Exponent, PeriodStart, PeriodLength, the stored magnitude, IsExact). Reading the
//! same fields is the same assertion, not a weaker one.

use crate::Real;
use lovelace_int::Int;
use lovelace_nat::Nat;

fn parse(text: &str) -> Real {
    Real::from_str(text).unwrap_or_else(|| panic!("cannot parse {text}"))
}

fn nat(text: &str) -> Nat {
    Nat::from_decimal(text).unwrap_or_else(|| panic!("cannot parse nat {text}"))
}

fn digits(r: &Real) -> String {
    r.int.to_decimal()
}

// ---------------------------------------------------------------------------
// RealConstructorTests
// ---------------------------------------------------------------------------

#[test]
fn constructor_given_default_produces_zero_with_exponent_zero() {
    let r = Real::zero();
    assert!(r.is_zero());
    assert_eq!(r.exp, 0);
}

#[test]
fn constructor_given_string_integer_exponent_is_zero() {
    let r = parse("42");
    assert_eq!(r.exp, 0);
    assert_eq!(r, parse("42"));
}

#[test]
fn constructor_given_string_decimal_exponent_is_negative_place_count() {
    assert_eq!(parse("3.14").exp, -2);
}

#[test]
fn constructor_given_string_negative_decimal_stored_correctly() {
    let r = parse("-0.001");
    assert!(r.to_string().starts_with('-'), "not negative: {r}");
    assert_eq!(r, parse("-0.001"));
}

#[test]
fn constructor_given_real_copy_copies_all_fields() {
    let original = parse("0.(3)");
    let copy = original.clone();
    assert_eq!(original.exp, copy.exp);
    assert_eq!(original.period_start, copy.period_start);
    assert_eq!(original.period_len, copy.period_len);
    assert_eq!(original, copy);
}

#[test]
fn constructor_given_integer_exponent_is_zero() {
    let r = Real::from_int(&Int::from_i64(7));
    assert_eq!(r.exp, 0);
    assert_eq!(r, parse("7"));
}

#[test]
fn constructor_given_negative_integer_is_negative_and_exponent_zero() {
    let r = Real::from_int(&Int::from_i64(-42));
    assert!(r.to_string().starts_with('-'), "not negative: {r}");
    assert_eq!(r.exp, 0);
    assert_eq!(r, parse("-42"));
}

#[test]
fn assign_given_other_real_copies_all_fields() {
    let b = parse("3.14");
    let a = b.clone();
    assert_eq!(b.exp, a.exp);
    assert_eq!(b.period_start, a.period_start);
    assert_eq!(b.period_len, a.period_len);
    assert_eq!(b, a);
}

#[test]
fn assign_given_periodic_real_copies_period_metadata() {
    let periodic = parse("0.(3)");
    let copy = periodic.clone();
    assert_eq!(copy.period_start, 0);
    assert_eq!(copy.period_len, 1);
    assert!(copy.period_len > 0);
    assert_eq!(copy, parse("0.(3)"));
}

#[test]
fn constructor_given_read_only_span_integer_string_parses_correctly() {
    let r = parse("42");
    assert_eq!(r, parse("42"));
    assert_eq!(r.exp, 0);
}

#[test]
fn constructor_given_read_only_span_decimal_string_parses_correctly() {
    let r = parse("1.5");
    assert_eq!(r, parse("1.5"));
    assert_eq!(r.exp, -1);
}

#[test]
fn constructor_given_read_only_span_periodic_string_parses_correctly() {
    let r = parse("0.(3)");
    assert!(r.period_len > 0);
    assert_eq!(r.period_len, 1);
}

#[test]
fn constructor_given_read_only_span_negative_string_parses_correctly() {
    let r = parse("-2.5");
    assert!(r.to_string().starts_with('-'), "not negative: {r}");
    assert_eq!(r, parse("-2.5"));
}

#[test]
fn constructor_given_read_only_span_empty_throws_format_exception() {
    assert!(Real::from_str("").is_none(), "empty input must be refused");
}

#[test]
fn constructor_given_read_only_span_malformed_throws_format_exception() {
    assert!(
        Real::from_str("abc").is_none(),
        "malformed input must be refused"
    );
}

// ---------------------------------------------------------------------------
// RealPropertiesTests (the three defaults whose Rust representation matches)
// ---------------------------------------------------------------------------

#[test]
fn exponent_given_default_real_is_zero() {
    assert_eq!(Real::zero().exp, 0);
}

#[test]
fn period_length_given_default_real_is_zero() {
    assert_eq!(Real::zero().period_len, 0);
}

#[test]
fn is_periodic_given_default_real_is_false() {
    assert_eq!(Real::zero().period_len, 0);
}

// ---------------------------------------------------------------------------
// RealToStringTests.ToString_GivenPeriodLongerThanStoredDigits_DoesNotThrow
// ---------------------------------------------------------------------------

#[test]
fn to_string_given_period_longer_than_stored_digits_does_not_throw() {
    let malformed = Real {
        int: Int::from_i64(1),
        exp: 0,
        period_start: 0,
        period_len: 2,
        inexact: false,
    };
    let text = malformed.to_string();
    assert!(!text.is_empty(), "rendering is empty");
    assert_eq!(
        malformed,
        parse(&text),
        "rendering does not reparse: {text}"
    );
}

// ---------------------------------------------------------------------------
// RealDivideLeadingZeroRunTests
// ---------------------------------------------------------------------------

fn times_power_of_ten(coefficient: &str, zero_run: usize) -> Real {
    parse(&(coefficient.to_string() + &"0".repeat(zero_run)))
}

#[test]
fn divide_given_quotient_inside_the_budget_still_reports_the_exact_period() {
    let quotient = parse("1")
        .div(&times_power_of_ten("3", 100), 1000)
        .expect("nonzero divisor");
    assert!(quotient.is_exact());
    assert_eq!(digits(&quotient), "3");
    assert_eq!(quotient.exp, -101);
    assert_eq!(quotient.period_start, 100);
    assert_eq!(quotient.period_len, 1);
}

// ===========================================================================
// Round 12 batch — Lovelace.Real.Tests rows whose assertion this crate
// reproduces (division scale/period, zero-base powers, copy semantics).
// ===========================================================================

/// C# RealDivideTests.cs:185 — Divide_GivenDivisorJustBelowOne_StoresTheQuotientAtItsOwnScale.
#[test]
fn divide_given_divisor_just_below_one_stores_the_quotient_at_its_own_scale() {
    let q = parse("1").div(&parse("0.9"), 39).expect("division");
    assert_eq!(q.int.to_decimal(), "11");
    assert_eq!(q.exp, -1);
    assert_eq!(q.period_start, 0);
    assert_eq!(q.period_len, 1);
}

/// C# RealDivideTests.cs:201 — Divide_GivenTwoDigitPeriod_StoresBothPeriodDigits.
#[test]
fn divide_given_two_digit_period_stores_both_period_digits() {
    let q = parse("1").div(&parse("0.99"), 39).expect("division");
    assert_eq!(q.int.to_decimal(), "101");
    assert_eq!(q.exp, -2);
    assert_eq!(q.period_start, 0);
    assert_eq!(q.period_len, 2);
}

/// C# RealPowZeroBaseTests.cs:62 — Pow_ZeroBaseNegativeExponent_Refuses ("-1", "-1.0", "-2", "-0.5").
#[test]
fn pow_zero_base_negative_exponent_refuses() {
    for exponent in [-1i64, -2] {
        assert!(
            Real::zero().pow_int(exponent).is_none(),
            "0^{exponent} must refuse, not answer"
        );
    }
}

/// C# RealPowZeroBaseTests.cs:71 — Pow_ZeroBasePositiveExponent_IsStillZero ("1", "2").
#[test]
fn pow_zero_base_positive_exponent_is_still_zero() {
    for exponent in [1i64, 2] {
        let result = Real::zero()
            .pow_int(exponent)
            .expect("0 to a positive power has a value");
        assert!(result.is_zero(), "0^{exponent} returned {result}");
    }
}

/// C# RealConstructorTests.cs:84 — Constructor_GivenRealCopy_ProducesIndependentCopy.
#[test]
fn constructor_given_real_copy_produces_independent_copy() {
    let original = parse("3.14");
    let mut copy = original.clone();
    copy.exp = -99;
    assert_eq!(original.exp, -2);
}

/// C# RealConstructorTests.cs:144 — Assign_GivenOtherReal_ProducesDeepCopy.
#[test]
fn assign_given_other_real_produces_deep_copy() {
    let b = parse("3.14");
    let a = b.clone();
    let mut mutated = b.clone();
    mutated.exp = -99;
    assert_eq!(a.exp, -2);
}

// ===========================================================================
// Round 15 — Lovelace.Real.Tests rows the exactness/division repairs unblock.
// ===========================================================================

/// C# RealExactQuotientPropertyTests.cs:55 — Divide_TerminatingDenominator_ReturnsTheExactQuotient
/// (7 rows). The generator is the C# LCG, replayed literally so a failure names the same case.
#[test]
fn r15_divide_terminating_denominator_returns_the_exact_quotient() {
    for precision in [1usize, 2, 5, 18, 30, 37, 100] {
        let mut state: u64 = 0x2545F4914F6CDD1D;
        let mut next = |low: u64, high: u64| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            low + ((state >> 33) % (high - low))
        };
        for _ in 0..60 {
            let twos = next(0, 95);
            let fives = next(0, 95);
            let numerator = next(1, 1_000_000);
            let denominator = Nat::from_u64(2).pow(twos).mul(&Nat::from_u64(5).pow(fives));
            let denominator_int =
                Int::from_decimal(&denominator.to_decimal()).expect("denominator");
            let numerator_real = Real::from_int(&Int::from_i64(numerator as i64));
            let denominator_real = Real::from_int(&denominator_int);
            let quotient = numerator_real
                .div(&denominator_real, precision)
                .expect("terminating division");
            assert_eq!(
                quotient.mul(&denominator_real),
                numerator_real,
                "{numerator}/(2^{twos}*5^{fives}) at {precision} places did not multiply back"
            );
        }
    }
}

/// C# RealExactQuotientPropertyTests.cs:79 — Divide_TerminatingQuotient_DoesNotDependOnTheDigitBudget.
#[test]
fn r15_divide_terminating_quotient_does_not_depend_on_the_budget() {
    for (numerator, twos, fives) in [
        (1i64, 1u64, 0u64),
        (1, 3, 0),
        (1, 0, 1),
        (1, 1, 1),
        (7, 4, 2),
        (3, 13, 7),
        (1, 40, 0),
        (1, 0, 40),
        (5, 20, 20),
        (1, 200, 0),
        (123456789, 33, 17),
    ] {
        let denominator = Nat::from_u64(2).pow(twos).mul(&Nat::from_u64(5).pow(fives));
        let denominator_int = Int::from_decimal(&denominator.to_decimal()).expect("denominator");
        let numerator_real = Real::from_i64(numerator);
        let denominator_real = Real::from_int(&denominator_int);
        let tight = numerator_real
            .div(&denominator_real, 1)
            .expect("tight division");
        let generous = numerator_real
            .div(&denominator_real, 1000)
            .expect("generous division");
        assert_eq!(
            tight, generous,
            "{numerator}/(2^{twos}*5^{fives}) changed with the budget"
        );
    }
}

/// C# RealExactQuotientPropertyTests.cs:107 — Divide_TerminatingExpansionLongerThanTheDefaultBudget_IsExact.
#[test]
fn r15_divide_terminating_expansion_longer_than_the_default_budget_is_exact() {
    let denominator = Nat::from_u64(2).pow(4000);
    let denominator_int = Int::from_decimal(&denominator.to_decimal()).expect("denominator");
    let quotient = Real::from_i64(1)
        .div(&Real::from_int(&denominator_int), 1000)
        .expect("division");
    assert!(quotient.is_exact(), "1/2^4000 was marked inexact");
    assert_eq!(
        quotient.mul(&Real::from_int(&denominator_int)),
        Real::from_i64(1),
        "1/2^4000 did not multiply back"
    );
}

/// C# RealDivideLeadingZeroRunTests.cs:49 — Divide_GivenLeadingZeroRunLongerThanTheBudget_KeepsTheValueAndItsScale.
#[test]
fn r15_divide_leading_zero_run_keeps_the_value_and_its_scale() {
    for (coefficient, zero_run, period) in [("3", 99usize, "3"), ("3", 100, "3")] {
        let divisor_text = coefficient.to_string() + &"0".repeat(zero_run);
        let divisor = parse(&divisor_text);
        let quotient = parse("1").div(&divisor, 1000).expect("division");
        assert_eq!(
            digits(&quotient),
            period,
            "coefficient {coefficient} run {zero_run}"
        );
        assert_eq!(
            quotient.exp,
            -((zero_run + period.len()) as i64),
            "coefficient {coefficient} run {zero_run}"
        );
        assert_eq!(quotient.period_start, zero_run as i64);
        assert_eq!(quotient.period_len, period.len() as i64);
        assert!(
            quotient.is_exact(),
            "coefficient {coefficient} run {zero_run}"
        );
        let expected = format!("0.{}({})", "0".repeat(zero_run), period);
        assert_eq!(quotient.to_string(), expected);
        assert_eq!(quotient, parse(&expected));
    }
}

/// C# RealDivideExactnessPropertyTests.cs:68 — DividingPeriodicOperands_KeepsRepresentableQuotientsExact.
#[test]
fn r15_dividing_periodic_operands_keeps_representable_quotients_exact() {
    for (left_text, right_text, exp_num, exp_den) in [
        ("0.(3)", "0.(3)", "1", "1"),
        ("0.(9)", "2", "1", "2"),
        ("0.(3)", "0.5", "2", "3"),
        ("0.(142857)", "0.(142857)", "1", "1"),
        ("1.(3)", "0.(3)", "4", "1"),
        ("0.(6)", "0.(3)", "2", "1"),
        ("0.1(6)", "0.(3)", "1", "2"),
        ("0.(9)", "0.(3)", "3", "1"),
    ] {
        let expected = parse(exp_num).div(&parse(exp_den), 1000).expect("expected");
        let quotient = parse(left_text)
            .div(&parse(right_text), 1000)
            .expect("quotient");
        assert_eq!(quotient, expected, "{left_text}/{right_text}");
        assert!(quotient.is_exact(), "{left_text}/{right_text} was inexact");
    }

    let five_sixths = Real::from_i64(5)
        .div(&Real::from_i64(6), 1000)
        .expect("5/6");
    let seven_elevenths = Real::from_i64(7)
        .div(&Real::from_i64(11), 1000)
        .expect("7/11");
    let by_division = five_sixths.div(&seven_elevenths, 1000).expect("division");
    let by_multiplication = five_sixths.mul(
        &Real::from_i64(11)
            .div(&Real::from_i64(7), 1000)
            .expect("inverse"),
    );
    assert_eq!(by_division, by_multiplication);
    assert!(by_division.is_exact() && by_multiplication.is_exact());
    assert_eq!(
        by_division,
        Real::from_i64(55)
            .div(&Real::from_i64(42), 1000)
            .expect("55/42")
    );
}

/// C# RealDivideExactnessPropertyTests.cs:116 — QuotientsThatCannotBeRepresented_StayInexact.
#[test]
fn r15_quotients_that_cannot_be_represented_stay_inexact() {
    let beyond = Real::from_i64(1)
        .div(&Real::from_i64(3), 10)
        .expect("1/3")
        .div(&Real::from_i64(97), 10)
        .expect("1/291");
    assert!(!beyond.is_exact(), "1/291 at 10 places was reported exact");
    assert!(beyond > Real::zero() && beyond < Real::from_i64(1));

    let irrational = parse("2").sqrt(1000);
    let third = parse("0.(3)");
    assert!(!irrational.is_exact(), "sqrt(2) was reported exact");
    assert!(
        !irrational.div(&third, 1000).expect("sqrt/third").is_exact(),
        "sqrt(2)/0.(3) was reported exact"
    );
    assert!(
        !third.div(&irrational, 1000).expect("third/sqrt").is_exact(),
        "0.(3)/sqrt(2) was reported exact"
    );
}
