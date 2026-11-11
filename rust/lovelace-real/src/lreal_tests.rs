//! Parity rows for the fixed-width real tier: every assertion is the C# LReal64Tests /
//! LReal128Tests assertion transcribed from source. The C# tests compare the fixed-width value's
//! rendering against class Real's rendering for the same expression, and require the promotion
//! rows to refuse rather than round; these tests do the same, with the census row id in the
//! failure message.

use std::cmp::Ordering;

use crate::Real;
use crate::lreal::{LReal64, LReal128, LRealError};

fn real(text: &str) -> Real {
    Real::from_str(text).unwrap_or_else(|| panic!("literal {text}"))
}

fn r64(text: &str) -> LReal64 {
    LReal64::parse(text).unwrap_or_else(|_| panic!("LReal64 literal {text}"))
}

fn r128(text: &str) -> LReal128 {
    LReal128::parse(text).unwrap_or_else(|_| panic!("LReal128 literal {text}"))
}

fn divide_real(a: &str, b: &str) -> String {
    real(a)
        .div(&real(b), 1000)
        .unwrap_or_else(|| panic!("Real division {a}/{b}"))
        .to_string()
}

// ---------------------------------------------------------------------------
// LReal64Tests (58 census rows)
// ---------------------------------------------------------------------------

/// LReal64Tests.Parse_GivenValue_RoundTripsLikeReal (21 rows).
#[test]
fn lreal64_parse_round_trips_like_real() {
    for literal in [
        "0",
        "1",
        "-1",
        "42",
        "-42",
        "3.14",
        "0.5",
        "0.05",
        "1.5",
        "10.5",
        "100",
        "100.0",
        "0.10",
        "1234567890123456789",
        "0.1234567890123456789",
        "-0.0001",
        "0.(3)",
        "0.1(6)",
        "0.(142857)",
        "1.(3)",
        "0.(9)",
    ] {
        assert_eq!(
            r64(literal).to_string(),
            real(literal).to_string(),
            "LReal64 parse {literal}"
        );
    }
}

/// LReal64Tests.Add_GivenPair_MatchesReal (6 rows).
#[test]
fn lreal64_add_matches_real() {
    for (a, b) in [
        ("1", "2"),
        ("0.5", "0.25"),
        ("0.1", "0.2"),
        ("1.5", "2.25"),
        ("10", "100"),
        ("3.14", "2.71"),
    ] {
        let fixed = r64(a).add(&r64(b)).expect("add fits");
        let wide = real(a).add(&real(b));
        assert_eq!(fixed.to_string(), wide.to_string(), "LReal64 add {a}+{b}");
    }
}

/// LReal64Tests.Subtract_GivenPair_MatchesReal (5 rows).
#[test]
fn lreal64_subtract_matches_real() {
    for (a, b) in [
        ("1", "2"),
        ("3", "5"),
        ("0.5", "0.25"),
        ("0.3", "0.1"),
        ("10", "3"),
    ] {
        let fixed = r64(a).sub(&r64(b)).expect("subtract fits");
        let wide = real(a).sub(&real(b));
        assert_eq!(
            fixed.to_string(),
            wide.to_string(),
            "LReal64 subtract {a}-{b}"
        );
    }
}

/// LReal64Tests.Multiply_GivenPair_MatchesReal (4 rows).
#[test]
fn lreal64_multiply_matches_real() {
    for (a, b) in [
        ("2", "3"),
        ("0.1", "0.2"),
        ("1.5", "2.0"),
        ("12345678", "87654321"),
    ] {
        let fixed = r64(a).mul(&r64(b)).expect("product fits");
        let wide = real(a).mul(&real(b));
        assert_eq!(
            fixed.to_string(),
            wide.to_string(),
            "LReal64 multiply {a}*{b}"
        );
    }
}

/// LReal64Tests.Divide_GivenPair_MatchesReal (11 rows).
#[test]
fn lreal64_divide_matches_real() {
    for (a, b) in [
        ("1", "3"),
        ("1", "7"),
        ("1", "6"),
        ("1", "9"),
        ("2", "3"),
        ("3", "7"),
        ("7", "12"),
        ("10", "11"),
        ("1", "17"),
        ("3", "16"),
        ("100", "13"),
    ] {
        let fixed = r64(a).div(&r64(b)).expect("period fits the width");
        assert_eq!(
            fixed.to_string(),
            divide_real(a, b),
            "LReal64 divide {a}/{b}"
        );
    }
}

/// LReal64Tests.Add_Periodic_MatchesReal (1 row, three assertions).
#[test]
fn lreal64_add_periodic_matches_real() {
    for (a, b) in [("0.(3)", "0.(3)"), ("0.(3)", "0.1(6)"), ("0.(3)", "2")] {
        let fixed = r64(a).add(&r64(b)).expect("periodic sum fits");
        let wide = real(a).add(&real(b));
        assert_eq!(
            fixed.to_string(),
            wide.to_string(),
            "LReal64 periodic add {a}+{b}"
        );
    }
    let product = r64("0.(3)").mul(&r64("2")).expect("periodic product fits");
    assert_eq!(
        product.to_string(),
        real("0.(3)").mul(&real("2")).to_string()
    );
}

/// LReal64Tests.Add_Tenths_IsExact (1 row).
#[test]
fn lreal64_add_tenths_is_exact() {
    assert_eq!(
        r64("0.1").add(&r64("0.2")).expect("fits").to_string(),
        "0.3"
    );
}

/// LReal64Tests.Divide_LongPeriod_Promotes (1 row).
#[test]
fn lreal64_divide_long_period_promotes() {
    assert!(
        matches!(r64("1").div(&r64("97")), Err(LRealError::Promote)),
        "1/97 needs a 96-digit period, longer than LReal64 carries"
    );
}

/// LReal64Tests.Add_Overflow_Promotes (1 row).
#[test]
fn lreal64_add_overflow_promotes() {
    assert!(matches!(
        r64("9999999999999999999").add(&r64("0.000000001")),
        Err(LRealError::Promote)
    ));
}

/// LReal64Tests.Multiply_Overflow_Promotes (1 row).
#[test]
fn lreal64_multiply_overflow_promotes() {
    assert!(matches!(
        r64("2.345678901234567").mul(&r64("1.234567890123456")),
        Err(LRealError::Promote)
    ));
}

/// LReal64Tests.Parse_TooManyDigits_Fails (1 row).
#[test]
fn lreal64_parse_too_many_digits_fails() {
    assert!(LReal64::try_parse("12345678901234567890123").is_none());
}

/// LReal64Tests.Compare_MatchesReal (5 rows).
#[test]
fn lreal64_compare_matches_real() {
    for (a, b) in [
        ("0.1", "0.2"),
        ("0.(3)", "0.33333"),
        ("0.1(6)", "0.16666"),
        ("-1", "1"),
        ("3.14", "3.14159"),
    ] {
        let fixed = r64(a).compare(&r64(b));
        let wide = real(a).cmp(&real(b));
        assert_eq!(fixed, wide, "LReal64 compare {a} vs {b}");
        assert_ne!(fixed, Ordering::Equal);
    }
}

// ---------------------------------------------------------------------------
// LReal128Tests (26 census rows)
// ---------------------------------------------------------------------------

/// LReal128Tests.Parse_GivenValue_RoundTripsLikeReal (10 rows).
#[test]
fn lreal128_parse_round_trips_like_real() {
    for literal in [
        "0",
        "1",
        "-42",
        "3.14",
        "0.5",
        "0.10",
        "12345678901234567890123456789012345678",
        "0.(3)",
        "0.1(6)",
        "0.(142857)",
    ] {
        assert_eq!(
            r128(literal).to_string(),
            real(literal).to_string(),
            "LReal128 parse {literal}"
        );
    }
}

/// LReal128Tests.Add_GivenPair_MatchesReal (4 rows).
#[test]
fn lreal128_add_matches_real() {
    for (a, b) in [
        ("1", "2"),
        ("0.1", "0.2"),
        ("3.14", "2.71"),
        ("9999999999999999999", "0.000000001"),
    ] {
        let fixed = r128(a).add(&r128(b)).expect("sum fits 38 digits");
        let wide = real(a).add(&real(b));
        assert_eq!(fixed.to_string(), wide.to_string(), "LReal128 add {a}+{b}");
    }
}

/// LReal128Tests.Multiply_GivenPair_MatchesReal (3 rows).
#[test]
fn lreal128_multiply_matches_real() {
    for (a, b) in [
        ("2.345678901234567", "1.234567890123456"),
        ("0.1", "0.2"),
        ("1234567890123456789", "9876543210987654321"),
    ] {
        let fixed = r128(a).mul(&r128(b)).expect("product fits 38 digits");
        let wide = real(a).mul(&real(b));
        assert_eq!(
            fixed.to_string(),
            wide.to_string(),
            "LReal128 multiply {a}*{b}"
        );
    }
}

/// LReal128Tests.Divide_GivenPair_MatchesReal (5 rows).
#[test]
fn lreal128_divide_matches_real() {
    for (a, b) in [
        ("1", "3"),
        ("1", "7"),
        ("1", "29"),
        ("1", "17"),
        ("100", "13"),
    ] {
        let fixed = r128(a).div(&r128(b)).expect("period fits 38 digits");
        assert_eq!(
            fixed.to_string(),
            divide_real(a, b),
            "LReal128 divide {a}/{b}"
        );
    }
}

/// LReal128Tests.Add_Periodic_MatchesReal (1 row).
#[test]
fn lreal128_add_periodic_matches_real() {
    for (a, b) in [("0.(3)", "0.(3)"), ("0.(3)", "0.1(6)")] {
        let fixed = r128(a).add(&r128(b)).expect("periodic sum fits");
        let wide = real(a).add(&real(b));
        assert_eq!(
            fixed.to_string(),
            wide.to_string(),
            "LReal128 periodic add {a}+{b}"
        );
    }
}

/// LReal128Tests.Add_Tenths_IsExact (1 row).
#[test]
fn lreal128_add_tenths_is_exact() {
    assert_eq!(
        r128("0.1").add(&r128("0.2")).expect("fits").to_string(),
        "0.3"
    );
}

/// LReal128Tests.Multiply_Overflow_Promotes (1 row).
#[test]
fn lreal128_multiply_overflow_promotes() {
    assert!(matches!(
        r128("123456789012345678901234567890").mul(&r128("123456789012345678901234567890")),
        Err(LRealError::Promote)
    ));
}

/// LReal128Tests.Divide_LongPeriod_Promotes (1 row).
#[test]
fn lreal128_divide_long_period_promotes() {
    assert!(matches!(
        r128("1").div(&r128("97")),
        Err(LRealError::Promote)
    ));
}
