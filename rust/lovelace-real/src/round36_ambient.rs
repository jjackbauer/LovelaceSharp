//! Round 36: the ambient-precision residual row of ComplexMathAmbientBoundaryTests.
//!
//! The C# row computes sin(2*pi(100)) under a 30-place scope and asserts the residual, its
//! closeness to mpmath's digits, and the 30-fractional-digit rendering. The Rust real tier takes
//! the digit count explicitly (Real::sin(digits), Real::pi(digits)); to_decimal_fixed is the
//! display window the reference's DisplayDecimalPlaces renders.

use crate::Real;

fn real(text: &str) -> Real {
    Real::from_str(text).unwrap_or_else(|| panic!("bad real {text}"))
}

#[test]
fn sin_of_a_two_pi_truncation_finer_than_the_ambient_precision_keeps_its_residual() {
    let zeros = "0".repeat(99);
    let expected_text = format!(
        "-0.{zeros}164296173026564613294187689219101164463450718816256962234900568205403877042111192892458979098607639288576219513318668922569512965"
    );
    let x = Real::pi(100).mul(&Real::from_i64(2));
    let actual = x.sin(30);
    let expected = real(&expected_text);

    assert!(!actual.is_zero(), "sin(2*pi(100)) at 30 places = 0, expected {expected}");
    let tolerance = real(&format!("0.{}1", "0".repeat(104)));
    assert!(
        actual.sub(&expected).abs() < tolerance,
        "sin(2*pi(100)) at 30 places = {actual}, expected {expected}"
    );

    // the display window is 30 fractional places
    let rendered_expected = expected.to_decimal_fixed(30);
    assert_eq!(30, rendered_expected.len() - "-0.".len());
    assert!(rendered_expected.starts_with("-0.0"), "{rendered_expected}");
    assert_eq!(rendered_expected, actual.to_decimal_fixed(30));
}
