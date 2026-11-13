//! Stage 1 tests: exact rational arithmetic and Lovelace literal round trips.
//!
//! Mirrors Lovelace.Knowledge.Tests/ExactNumberTests.cs.

mod common;

use lovelace_know::exact::ExactNumber;

#[test]
fn add_periodic_thirds_gives_two_thirds() {
    let third = ExactNumber::parse("0.(3)").expect("0.(3) parses");
    let sum = third.add(&third);
    assert_eq!(sum.to_lovelace_literal(), "0.(6)");
}

#[test]
fn parse_periodic_is_exact_fraction() {
    let third = ExactNumber::parse("0.(3)").expect("0.(3) parses");
    let sixth = ExactNumber::parse("0.1(6)").expect("0.1(6) parses");
    assert_eq!(sixth.add(&sixth), third);
    assert_eq!(sixth.add(&sixth).to_lovelace_literal(), "0.(3)");
    assert_eq!(third.to_lovelace_literal(), "0.(3)");
    assert_eq!(sixth.to_lovelace_literal(), "0.1(6)");
}

#[test]
fn format_one_seventh_shortest_period() {
    let seventh = ExactNumber::new(
        lovelace_int::Int::from_i64(1),
        lovelace_nat::Nat::from_u64(7),
    )
    .expect("7 is not zero");
    assert_eq!(seventh.to_lovelace_literal(), "0.(142857)");
}

#[test]
fn round_trips_canonical_literal() {
    for literal in [
        "0", "42", "-7", "3.14", "-1.5", "0.25", "0.5", "2.5", "0.(3)", "0.1(6)", "-1.2(34)",
    ] {
        let parsed = ExactNumber::parse(literal).expect("the literal parses");
        assert_eq!(
            parsed.to_lovelace_literal(),
            literal,
            "round trip of {literal}"
        );
    }
}

#[test]
fn compare_periodic_vs_decimal() {
    let third = ExactNumber::parse("0.(3)").expect("0.(3) parses");
    let decimal = ExactNumber::parse("0.333").expect("0.333 parses");
    assert!(third > decimal);
    assert_eq!(ExactNumber::parse("0.3333(3)").expect("parses"), third);
    assert!(
        ExactNumber::parse("-1.5").expect("parses") < ExactNumber::parse("-0.5").expect("parses")
    );
}

#[test]
fn integers_are_integral_and_signed() {
    let value = ExactNumber::parse("-7").expect("parses");
    assert!(value.is_integer());
    assert!(value.is_negative());
    assert!(!value.is_zero());
    assert_eq!(value.to_lovelace_literal(), "-7");
    assert!(ExactNumber::parse("0").expect("parses").is_zero());
    assert!(!ExactNumber::parse("0.25").expect("parses").is_integer());
}

#[test]
fn arithmetic_is_exact() {
    let quarter = ExactNumber::parse("0.25").expect("parses");
    let third = ExactNumber::parse("0.(3)").expect("parses");
    let product = quarter.subtract(&third);
    assert_eq!(product.to_lovelace_literal(), "-0.08(3)");
    assert_eq!(product.add(&third), quarter);
}

#[test]
fn rejects_malformed_literals() {
    for literal in ["", " ", "1.2.3", "0.()", "0.(3", "abc", "1..2"] {
        assert!(
            ExactNumber::parse(literal).is_err(),
            "{literal} must not parse"
        );
    }
}
