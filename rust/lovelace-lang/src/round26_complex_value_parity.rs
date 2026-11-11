//! Round 26: counted parity for the `Lovelace.Complex.Tests` value-type rows that the round-24
//! `Complex`/`LComplex64`/`LComplex128` surface already covers.
//!
//! Most rows resolve to the round-24 tests; the four tests here pin the few assertions that
//! module did not exercise (component-wise inequality, the E/Pi constants, and the LComplex128
//! Parse/ToString round trip). The `DenseArray<Complex>` rows are refused: this workspace has no
//! `DType`/`DenseArray` typed-array surface to assert against.

use lovelace_real::Real;

use crate::complex::{Complex, LComplex128};

fn r(text: &str) -> Real {
    Real::from_str(text).unwrap_or_else(|| panic!("literal {text}"))
}

fn c(re: &str, im: &str) -> Complex {
    Complex::new(r(re), r(im))
}

#[test]
fn real_e_and_pi_constants_match_their_known_prefixes() {
    // ComplexTests.ETo_Given30Digits_MatchesKnownConstant and
    // ComplexTests.PiTo_Given30Digits_MatchesKnownConstant.
    assert!(
        Real::e(30).to_string().starts_with("2.71828182845904523536"),
        "ETo(30) = {}",
        Real::e(30)
    );
    assert!(
        Real::pi(30).to_string().starts_with("3.14159265358979323846"),
        "PiTo(30) = {}",
        Real::pi(30)
    );
}

#[test]
fn complex_equality_is_componentwise() {
    // ComplexTests.Equality_GivenComplex_IsComponentwise, including the unequal component.
    assert_eq!(c("1", "2"), c("1", "2"));
    assert_ne!(c("1", "2"), c("1", "3"));
}

#[test]
fn complex_pi_and_e_constants_are_real_valued() {
    // ComplexTests.Constants_PiAndE_AreRealValued.
    assert_eq!(Complex::pi(30).im(), &Real::zero());
    assert_eq!(Complex::e(30).im(), &Real::zero());
    assert!(Complex::pi(30).re().cmp(&r("3")) == std::cmp::Ordering::Greater);
    assert!(Complex::e(30).re().cmp(&r("2")) == std::cmp::Ordering::Greater);
}

#[test]
fn lcomplex128_parse_and_display_round_trip() {
    // LComplex128Tests.Parse_GivenToStringOutput_ProducesEqualValue (4 rows).
    for text in ["2", "4i", "1.5 + 0.5i", "-2+3i"] {
        let z = LComplex128::parse(text).unwrap_or_else(|error| panic!("{text}: {error}"));
        assert_eq!(LComplex128::parse(&z.to_string()).unwrap(), z, "{text}");
    }
}
