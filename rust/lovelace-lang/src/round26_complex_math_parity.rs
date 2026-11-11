//! Round 26: counted parity for `Lovelace.Complex.Tests.ComplexMath` rows.
//!
//! The reference `ComplexMath` is a static surface over `Real`; the Rust port exposes the real
//! entries on `Real` (`exp`, `ln`, `sin`, `cos`, `tan`, `atan`, `atan2`, `asin`, `acos`,
//! `asinh`, `acosh`, `atanh`, `pow_real`) and the complex entries in `crate::complex_math`.
//! Each test reproduces the C# assertion — the same tolerance, the same value or the same
//! exactness flag — at the same 50-place budget the C# file uses.
//!
//! The reference's `Sin(x, digits)` returns the guarded series value and only the renderer
//! truncates; `sin_cos_raw` is that value, so the pi-resolution rows compare the raw pair with
//! the high-precision table exactly as the C# test does. Refused rows are recorded in the round
//! report, not mapped.

use lovelace_real::Real;

/// This module pins the reference ComplexMath kernel (the numeric evaluator's SinCosAtPrecision
/// pair), which is NOT Real.Sin/Real.Cos: the two answer differently for an exact table angle and
/// for a huge argument. The extension methods name the kernel under test explicitly so a call site
/// cannot silently move to the other one.
trait ComplexMathTrig {
    fn cm_sin(&self, digits: usize) -> Real;
    fn cm_cos(&self, digits: usize) -> Real;
}

impl ComplexMathTrig for Real {
    fn cm_sin(&self, digits: usize) -> Real {
        lovelace_real::transcendental::truncate_frac(&self.sin_cos_raw(digits).0, digits)
    }

    fn cm_cos(&self, digits: usize) -> Real {
        lovelace_real::transcendental::truncate_frac(&self.sin_cos_raw(digits).1, digits)
    }
}

use crate::complex::Complex;
use crate::complex_math as cm;

fn r(text: &str) -> Real {
    Real::from_str(text).unwrap_or_else(|| panic!("literal {text}"))
}

fn c(re: &str, im: &str) -> Complex {
    Complex::new(r(re), r(im))
}

fn tiny(places: usize) -> Real {
    r(&format!("0.{}1", "0".repeat(places - 1)))
}

fn below(actual: &Real, limit: &Real) -> bool {
    actual.cmp(limit) == std::cmp::Ordering::Less
}

fn close(expected: &Real, actual: &Real, digits: usize) -> bool {
    below(&actual.sub(expected).abs(), &tiny(digits))
}

fn close_complex(expected: &Complex, actual: &Complex, digits: usize) -> bool {
    close(expected.re(), actual.re(), digits) && close(expected.im(), actual.im(), digits)
}

fn assert_complex_inexact(label: &str, value: &Complex) {
    assert!(
        !value.re().is_exact(),
        "{label}: the real part claims exactness"
    );
    assert!(
        !value.im().is_exact(),
        "{label}: the imaginary part claims exactness"
    );
}

// ---------------------------------------------------------------------------
// ComplexMathTests (31 rows)
// ---------------------------------------------------------------------------

#[test]
fn complex_math_exp_ln_and_pow_on_reals() {
    // Exp_GivenZero_ReturnsOne, Exp_GivenOne_MatchesETo45Digits,
    // Exp_GivenLnOfTwo_ReturnsTwoTo45Digits, Ln_GivenOne_ReturnsZero,
    // Ln_GivenNegative_ThrowsArgumentException, Ln_GivenZero_ThrowsArgumentException,
    // Pow_GivenTwoAndHalf_Squared_ReturnsTwoTo45Digits,
    // Pow_GivenNegativeBase_ThrowsArgumentException.
    assert_eq!(Real::zero().exp(50), Real::from_i64(1));
    assert!(close(&Real::e(50), &Real::from_i64(1).exp(50), 45));
    let ln_two = r("2").ln(50).expect("ln(2)");
    assert!(close(&r("2"), &ln_two.exp(50), 45));
    assert_eq!(r("1").ln(50).expect("ln(1)"), Real::zero());
    assert!(r("-1").ln(50).is_none(), "ln(-1) must be refused");
    assert!(Real::zero().ln(50).is_none(), "ln(0) must be refused");
    let root = r("2").pow_real(&r("0.5"), 50).expect("2^0.5");
    assert!(close(&r("2"), &root.mul(&root), 45));
    assert!(
        r("-1").pow_real(&r("0.5"), 50).is_none(),
        "(-1)^0.5 must be refused"
    );
}

#[test]
fn complex_math_real_trig_at_fifty_places() {
    // Sin_GivenPiOverTwo_ReturnsOne, Sin_GivenPi_ReturnsZero, Cos_GivenZero_ReturnsOne,
    // Cos_GivenPi_ReturnsNegativeOne, Tan_GivenPiOverFour_ReturnsOneTo45Digits,
    // Atan_GivenOne_ReturnsPiOverFourTo45Digits, Atan2_GivenYOneXZero_ReturnsPiOverTwo,
    // Atan2_GivenYZeroXNegativeOne_ReturnsPi.
    let pi = Real::pi(50);
    let half_pi = pi.div(&Real::from_i64(2), 50).expect("pi/2");
    assert_eq!(half_pi.cm_sin(50), Real::from_i64(1));
    assert_eq!(pi.cm_sin(50), Real::zero());
    assert_eq!(Real::zero().cm_cos(50), Real::from_i64(1));
    assert_eq!(pi.cm_cos(50), Real::from_i64(-1));
    let quarter = pi.div(&Real::from_i64(4), 50).expect("pi/4");
    assert!(close(&r("1"), &quarter.tan(50).expect("tan(pi/4)"), 45));
    assert!(close(&quarter, &r("1").atan(50), 45));
    assert_eq!(Real::atan2(&r("1"), &Real::zero(), 50), half_pi);
    assert_eq!(Real::atan2(&Real::zero(), &r("-1"), 50), pi);
}

#[test]
fn complex_math_complex_functions_round_trip() {
    // Exp_GivenIPi_ReturnsNegativeOneTo45Digits, SinSquaredPlusCosSquared_GivenComplex_ReturnsOne,
    // Log_GivenExpOfZ_ReturnsZTo40Digits, Sqrt_GivenZ_Squared_ReturnsZ,
    // Pow_GivenZAndTwo_ReturnsZSquared, Log_GivenZero_ThrowsArgumentException.
    let i_pi = Complex::new(Real::zero(), Real::pi(50));
    assert!(close_complex(&c("-1", "0"), &cm::exp(&i_pi, 50), 45));

    let z34 = c("0.3", "0.4");
    let sin = cm::sin(&z34, 50);
    let cos = cm::cos(&z34, 50);
    assert!(close_complex(
        &Complex::one(),
        &sin.mul(&sin).add(&cos.mul(&cos)),
        45
    ));

    let z = c("0.2", "0.3");
    assert!(close_complex(
        &z,
        &cm::log(&cm::exp(&z, 50), 50).expect("log(exp z)"),
        40
    ));

    let zq = c("0.5", "0.25");
    let root = cm::sqrt(&zq, 50);
    assert!(close_complex(&zq, &root.mul(&root), 45));
    assert!(close_complex(
        &zq.mul(&zq),
        &cm::pow(&zq, &c("2", "0"), 50).expect("z^2"),
        45
    ));
    assert!(cm::log(&Complex::zero(), 50).is_none());
}

#[test]
fn complex_math_inverse_trig_and_hyperbolic() {
    // AsinReal_GivenOne_ReturnsPiOverTwo, Sin_GivenAsinRealOfPointFive_ReturnsPointFiveTo45Digits,
    // AcosReal_GivenZero_ReturnsPiOverTwo, Exp_GivenAsinhRealOfPointThree_MatchesIdentityTo40Digits,
    // AcoshReal_GivenTwo_CoshRoundtripReturnsTwo,
    // AtanhReal_GivenPointThree_TanhRoundtripReturnsPointThree,
    // AsinReal_GivenTwo_ThrowsArgumentException, AcoshReal_GivenPointFive_ThrowsArgumentException,
    // AtanhReal_GivenOne_ThrowsArgumentException.
    let pi = Real::pi(50);
    let half_pi = pi.div(&Real::from_i64(2), 50).expect("pi/2");
    assert_eq!(r("1").asin(50).expect("asin(1)"), half_pi);
    assert!(close(
        &r("0.5"),
        &r("0.5").asin(50).expect("asin(0.5)").cm_sin(50),
        45
    ));
    assert_eq!(Real::zero().acos(50).expect("acos(0)"), half_pi);

    let x = r("0.3");
    let identity = x.add(&x.mul(&x).add(&Real::from_i64(1)).sqrt(50));
    assert!(close(&identity, &x.asinh(50).exp(50), 40));

    let two = r("2");
    let a = two.acosh(50).expect("acosh(2)");
    let cosh = a.exp(50).add(&a.neg().exp(50)).div(&two, 50).expect("cosh");
    assert!(close(&two, &cosh, 45));

    let t = x.atanh(50).expect("atanh(0.3)");
    let e2t = r("2").mul(&t).exp(50);
    let tanh = e2t
        .sub(&Real::from_i64(1))
        .div(&e2t.add(&Real::from_i64(1)), 50)
        .expect("tanh");
    assert!(close(&x, &tanh, 45));

    assert!(r("2").asin(50).is_none());
    assert!(r("2").acos(50).is_none());
    assert!(r("0.5").acosh(50).is_none());
    assert!(r("1").atanh(50).is_none());
}

// ---------------------------------------------------------------------------
// ComplexMathPiResolutionTests (9 rows)
// ---------------------------------------------------------------------------

#[test]
fn complex_math_pi_resolution_sin_integer_multiples() {
    // Sin_IntegerMultiplesOfPiTruncatedAtDPlaces_ReproduceTheDistanceToPi at 40/100/200 places.
    for places in [40usize, 100, 200] {
        let pi = Real::pi(places);
        let delta = Real::pi(places + 100).sub(&pi);
        let tolerance = tiny(places + 5);
        for k in 1..=6i64 {
            let actual = pi.mul(&Real::from_i64(k)).cm_sin(30);
            let mut expected = delta.mul(&Real::from_i64(k));
            if k % 2 == 0 {
                expected = expected.neg();
            }
            assert!(
                below(&actual.sub(&expected).abs(), &tolerance),
                "sin(pi({places})*{k}) at 30 places = {actual}, expected {expected}"
            );
        }
    }
}

#[test]
fn complex_math_pi_resolution_tan_of_half_pi() {
    // Tan_OfHalfPiTruncatedAtDPlaces_IsTheReciprocalOfTheHalfDistanceToPi.
    for places in [40usize, 100, 200] {
        let pi = Real::pi(places);
        let half_pi = pi.div(&Real::from_i64(2), places).expect("pi/2");
        let delta = Real::pi(places + 100).sub(&pi);
        let half_delta = delta
            .div(&Real::from_i64(2), places + 100)
            .expect("delta/2");
        let tolerance = tiny(places + 5);
        let cos = half_pi.cm_cos(30);
        let tan = half_pi.tan(30).expect("tan(pi/2)");
        assert!(
            cos.cmp(&Real::zero()) == std::cmp::Ordering::Greater,
            "cos(pi({places})/2) = {cos} must be positive"
        );
        assert!(
            below(&cos.sub(&half_delta).abs(), &tolerance),
            "cos(pi({places})/2) at 30 places = {cos}, expected {half_delta}"
        );
        assert!(
            below(&tan.mul(&cos).sub(&Real::from_i64(1)).abs(), &tiny(27)),
            "tan(pi({places})/2) * cos(pi({places})/2) must be 1"
        );
        assert!(
            tan.cmp(&Real::from_i64(1)) == std::cmp::Ordering::Greater,
            "tan(pi({places})/2) = {tan}"
        );
    }
}

#[test]
fn complex_math_pi_resolution_exact_ambient_multiples() {
    // SinCos_ExactMultiplesOfTheAmbientPi_KeepTheirExactValues at 20/50/100 places: value only;
    // reducing an exact multiple of the ambient pi multiplies by that pi, so exactness is not
    // asserted here (the reference comment says so explicitly).
    for places in [20usize, 50, 100] {
        let pi = Real::pi(places);
        let tolerance = tiny(places - 1);
        assert_eq!(pi.cm_sin(places), Real::zero());
        assert_eq!(pi.cm_cos(places), Real::from_i64(-1));
        let half = pi.div(&Real::from_i64(2), places).expect("pi/2");
        assert!(below(
            &half.cm_sin(places).sub(&Real::from_i64(1)).abs(),
            &tolerance
        ));
        let third = pi.div(&Real::from_i64(3), places).expect("pi/3");
        assert!(below(
            &third.cm_cos(places).sub(&r("0.5")).abs(),
            &tolerance
        ));
        let quarter = pi.div(&Real::from_i64(4), places).expect("pi/4");
        assert!(below(
            &quarter
                .tan(places)
                .expect("tan(pi/4)")
                .sub(&Real::from_i64(1))
                .abs(),
            &tolerance
        ));
    }
}

// ---------------------------------------------------------------------------
// ComplexMathProvenanceTests (4 rows)
// ---------------------------------------------------------------------------

#[test]
fn complex_math_truncated_transcendentals_are_not_exact() {
    // TruncatedTranscendentals_AreNotExact: five arguments, five functions, plus atan(1).
    for text in ["1", "0.5", "2"] {
        let x = r(text);
        assert!(!x.cm_sin(30).is_exact(), "sin({x}) claims exactness");
        assert!(!x.cm_cos(30).is_exact(), "cos({x}) claims exactness");
        assert!(
            !x.tan(30).expect("tan").is_exact(),
            "tan({x}) claims exactness"
        );
        assert!(!x.exp(30).is_exact(), "exp({x}) claims exactness");
        assert!(
            !x.ln(30).expect("ln").is_exact(),
            "ln({x}) claims exactness"
        );
    }
    let third = Real::from_i64(1).div(&Real::from_i64(3), 30).expect("1/3");
    let seventh_fifths = Real::from_i64(7).div(&Real::from_i64(5), 30).expect("7/5");
    for x in [third, seventh_fifths] {
        assert!(!x.cm_sin(30).is_exact(), "sin({x}) claims exactness");
        assert!(!x.cm_cos(30).is_exact(), "cos({x}) claims exactness");
        assert!(
            !x.tan(30).expect("tan").is_exact(),
            "tan({x}) claims exactness"
        );
        assert!(!x.exp(30).is_exact(), "exp({x}) claims exactness");
        assert!(
            !x.ln(30).expect("ln").is_exact(),
            "ln({x}) claims exactness"
        );
    }
    assert!(!r("1").atan(30).is_exact(), "atan(1) claims exactness");
}

#[test]
fn complex_math_exact_results_keep_their_exactness() {
    // ExactResults_KeepTheirExactness: the guards are exact, the pi multiples are value-only.
    assert!(Real::zero().cm_sin(30).is_exact());
    assert_eq!(Real::zero().cm_sin(30), Real::zero());
    assert!(Real::zero().cm_cos(30).is_exact());
    assert_eq!(Real::zero().cm_cos(30), Real::from_i64(1));
    assert!(Real::zero().exp(30).is_exact());
    assert_eq!(Real::zero().exp(30), Real::from_i64(1));
    let pi = Real::pi(30);
    assert_eq!(pi.cm_sin(30), Real::zero());
    assert_eq!(pi.cm_cos(30), Real::from_i64(-1));
}

#[test]
fn complex_math_tiny_values_are_not_flushed_to_zero() {
    // ValuesSmallerThanTheDigitWindow_AreNotFlushedToZero.
    let tiny_value = r(&format!("0.{}1", "0".repeat(99)));
    let exp_neg70 = Real::from_i64(-70).exp(30);
    let sin_tiny = tiny_value.cm_sin(30);
    let tan_tiny = tiny_value.tan(30).expect("tan(1e-100)");
    assert!(
        !sin_tiny.is_zero(),
        "sin(1e-100) = {sin_tiny} was flushed to zero"
    );
    assert!(
        below(
            &sin_tiny.sub(&tiny_value).abs(),
            &tiny_value.mul(&r("0.01"))
        ),
        "sin(1e-100) = {sin_tiny}, expected {tiny_value}"
    );
    assert!(
        !tan_tiny.is_zero(),
        "tan(1e-100) = {tan_tiny} was flushed to zero"
    );
    assert!(
        below(
            &tan_tiny.sub(&tiny_value).abs(),
            &tiny_value.mul(&r("0.01"))
        ),
        "tan(1e-100) = {tan_tiny}, expected {tiny_value}"
    );
    assert!(
        !exp_neg70.is_zero(),
        "exp(-70) = {exp_neg70} was flushed to zero"
    );
    assert!(
        exp_neg70.abs().cmp(&tiny(31)) == std::cmp::Ordering::Greater,
        "exp(-70) = {exp_neg70} is smaller than 1e-31"
    );
}

#[test]
fn complex_math_truncating_an_inexact_value_stays_inexact() {
    // TruncatingAnInexactValue_DoesNotMakeItExact.
    let inexact = r("1").cm_sin(30);
    assert!(!inexact.is_exact());
    assert!(!inexact.cm_cos(30).is_exact());
    assert!(!inexact.exp(30).is_exact());
}

// ---------------------------------------------------------------------------
// ComplexMathExpSqrtInexactArgumentTests (7 rows)
// ---------------------------------------------------------------------------

#[test]
fn complex_math_exp_of_an_inexact_zero_is_not_exact() {
    // ExpOfAnInexactZero_IsNotExact.
    let inexact_zero = Real::zero().mark_inexact();
    assert!(!inexact_zero.is_exact());
    let exp = inexact_zero.exp(40);
    assert_eq!(exp, Real::from_i64(1));
    assert!(!exp.is_exact(), "exp(inexact zero) claims exactness");
    assert_eq!(Real::zero().exp(40), Real::from_i64(1));
    assert!(Real::zero().exp(40).is_exact());
}

#[test]
fn complex_math_complex_exp_of_an_inexact_part_is_not_exact() {
    // ComplexExpOfAnInexactPart_IsNotExact rows (true,false), (false,true), (true,true).
    let inexact_zero = Real::zero().mark_inexact();
    for (real_inexact, imaginary_inexact) in [(true, false), (false, true), (true, true)] {
        let z = Complex::new(
            if real_inexact {
                inexact_zero.clone()
            } else {
                Real::zero()
            },
            if imaginary_inexact {
                inexact_zero.clone()
            } else {
                Real::zero()
            },
        );
        let exp = cm::exp(&z, 40);
        assert_eq!(exp.re(), &Real::from_i64(1));
        assert_eq!(exp.im(), &Real::zero());
        assert_complex_inexact("exp(inexact part)", &exp);
        let exact = cm::exp(&Complex::zero(), 40);
        assert_eq!(exact.re(), &Real::from_i64(1));
        assert_eq!(exact.im(), &Real::zero());
        assert!(
            exact.re().is_exact() && exact.im().is_exact(),
            "exp(0 + 0i) must stay exact"
        );
    }
}

#[test]
fn complex_math_complex_exp_of_an_inexact_nonzero_stays_inexact() {
    // ComplexExpOfAnInexactNonZero_StaysInexact.
    let z = Complex::new(Real::from_i64(1).mark_inexact(), Real::zero());
    assert!(!z.re().is_exact());
    assert_complex_inexact("exp(inexact 1 + 0i)", &cm::exp(&z, 40));
}

#[test]
fn complex_math_sqrt_of_an_inexact_argument_is_not_exact() {
    // SqrtOfAnInexactArgument_IsNotExact: inexact zero, inexact perfect square, inexact non-square.
    let inexact_zero = Complex::new(Real::zero().mark_inexact(), Real::zero());
    let root = cm::sqrt(&inexact_zero, 40);
    assert_eq!(root.re(), &Real::zero());
    assert_eq!(root.im(), &Real::zero());
    assert_complex_inexact("sqrt(inexact 0 + 0i)", &root);

    let inexact_four = Complex::new(r("4").mark_inexact(), Real::zero());
    let root = cm::sqrt(&inexact_four, 40);
    assert_eq!(root.re(), &r("2"));
    assert_eq!(root.im(), &Real::zero());
    assert_complex_inexact("sqrt(inexact 4 + 0i)", &root);

    let inexact_two = Complex::new(r("2").mark_inexact(), Real::zero());
    let root = cm::sqrt(&inexact_two, 40);
    assert_eq!(root.re(), &r("2").sqrt(40));
    assert_eq!(root.im(), &Real::zero());
    assert_complex_inexact("sqrt(inexact 2 + 0i)", &root);

    let exact_zero = cm::sqrt(&Complex::zero(), 40);
    assert_eq!(exact_zero.re(), &Real::zero());
    assert_eq!(exact_zero.im(), &Real::zero());
    assert!(exact_zero.re().is_exact() && exact_zero.im().is_exact());

    let exact_four = cm::sqrt(&c("4", "0"), 40);
    assert_eq!(exact_four.re(), &r("2"));
    assert_eq!(exact_four.im(), &Real::zero());
    assert!(exact_four.re().is_exact() && exact_four.im().is_exact());
}

#[test]
fn complex_math_sqrt_of_an_inexact_imaginary_part_is_not_exact() {
    // SqrtOfAnInexactImaginaryPart_IsNotExact.
    let z = Complex::new(r("9"), Real::zero().mark_inexact());
    assert!(!z.im().is_exact());
    let root = cm::sqrt(&z, 40);
    assert_eq!(root.re(), &r("3"));
    assert_eq!(root.im(), &Real::zero());
    assert_complex_inexact("sqrt(9 + inexact 0i)", &root);
}

// ---------------------------------------------------------------------------
// ComplexMathInexactArgumentProvenanceTests (10 of 12 rows; 2 symbolic rows refused)
// ---------------------------------------------------------------------------

#[test]
fn complex_math_inexact_pi_at_its_own_scale_produces_inexact_results() {
    // InexactPiAtItsOwnScale_ProducesInexactResults rows 30, 60 and 100.
    for scale in [30usize, 60, 100] {
        let pi = Real::pi(scale);
        assert!(
            !pi.is_exact(),
            "pi({scale}) is a truncation and must be inexact"
        );
        let cos = pi.cm_cos(scale);
        let sin = pi.cm_sin(scale);
        let tan = pi.tan(scale).expect("tan(pi)");
        assert_eq!(cos, Real::from_i64(-1));
        assert_eq!(sin, Real::zero());
        assert_eq!(tan, Real::zero());
        assert!(!cos.is_exact(), "cos(pi({scale})) claims exactness");
        assert!(!sin.is_exact(), "sin(pi({scale})) claims exactness");
        assert!(!tan.is_exact(), "tan(pi({scale})) claims exactness");
    }
}

#[test]
fn complex_math_exactly_zero_reduction_of_an_inexact_pi_is_not_exact() {
    // ExactlyZeroReduction_OfAnInexactAmbientPi_IsNotExact.
    let pi = Real::pi(40);
    assert!(!pi.is_exact());
    let cos = pi.cm_cos(40);
    let sin = pi.cm_sin(40);
    let tan = pi.tan(40).expect("tan(pi)");
    assert_eq!(cos, Real::from_i64(-1));
    assert_eq!(sin, Real::zero());
    assert_eq!(tan, Real::zero());
    assert!(!cos.is_exact() && !sin.is_exact() && !tan.is_exact());
}

#[test]
fn complex_math_inexact_pi_at_another_scale_produces_inexact_results() {
    // InexactPiAtAnotherScale_ProducesInexactResults.
    let pi30 = Real::pi(30);
    assert!(!pi30.is_exact());
    assert!(!pi30.cm_cos(100).is_exact());
    assert!(!pi30.cm_sin(100).is_exact());
    assert!(!pi30.tan(100).expect("tan").is_exact());
}

#[test]
fn complex_math_exact_zero_argument_keeps_its_exact_results() {
    // ExactZeroArgument_KeepsItsExactResults.
    assert_eq!(Real::zero().cm_sin(40), Real::zero());
    assert_eq!(Real::zero().cm_cos(40), Real::from_i64(1));
    assert_eq!(Real::zero().tan(40).expect("tan(0)"), Real::zero());
    assert!(Real::zero().cm_sin(40).is_exact());
    assert!(Real::zero().cm_cos(40).is_exact());
    assert!(Real::zero().tan(40).expect("tan(0)").is_exact());
}

#[test]
fn complex_math_inexact_zero_argument_does_not_claim_exactness() {
    // InexactZeroArgument_DoesNotClaimExactness.
    let zero = Real::zero().mark_inexact();
    let sin = zero.cm_sin(40);
    let cos = zero.cm_cos(40);
    let tan = zero.tan(40).expect("tan(inexact 0)");
    assert_eq!(sin, Real::zero());
    assert_eq!(cos, Real::from_i64(1));
    assert_eq!(tan, Real::zero());
    assert!(!sin.is_exact(), "sin(inexact zero) claims exactness");
    assert!(!cos.is_exact(), "cos(inexact zero) claims exactness");
    assert!(!tan.is_exact(), "tan(inexact zero) claims exactness");
}

#[test]
fn complex_math_complex_trig_of_an_inexact_zero_part_is_not_exact() {
    // ComplexTrigOfAnInexactZeroPart_IsNotExact: all five complex functions, both components.
    let z = Complex::new(Real::zero(), Real::zero().mark_inexact());
    assert!(!z.im().is_exact());
    assert_complex_inexact("sin(0 + 0i)", &cm::sin(&z, 40));
    assert_complex_inexact("cos(0 + 0i)", &cm::cos(&z, 40));
    assert_complex_inexact("tan(0 + 0i)", &cm::tan(&z, 40).expect("tan"));
    assert_complex_inexact("sinh(0 + 0i)", &cm::sinh(&z, 40));
    assert_complex_inexact("cosh(0 + 0i)", &cm::cosh(&z, 40));
}

#[test]
fn complex_math_complex_trig_of_an_exact_zero_keeps_its_exactness() {
    // ComplexTrigOfAnExactZero_KeepsItsExactness.
    let z = Complex::zero();
    let sin = cm::sin(&z, 40);
    assert_eq!(sin.re(), &Real::zero());
    assert_eq!(sin.im(), &Real::zero());
    assert!(sin.re().is_exact() && sin.im().is_exact());
    let cos = cm::cos(&z, 40);
    assert_eq!(cos.re(), &Real::from_i64(1));
    assert_eq!(cos.im(), &Real::zero());
    assert!(cos.re().is_exact() && cos.im().is_exact());
}

#[test]
fn complex_math_inverse_trig_of_an_inexact_zero_argument_is_not_exact() {
    // InverseTrigOfAnInexactZeroArgument_IsNotExact, with its exact controls.
    let zero = Real::zero().mark_inexact();
    assert_eq!(zero.atan(40), Real::zero());
    assert_eq!(Real::atan2(&zero, &Real::from_i64(1), 40), Real::zero());
    assert_eq!(zero.asin(40).expect("asin(0)"), Real::zero());
    assert!(
        !zero.atan(40).is_exact(),
        "atan(inexact zero) claims exactness"
    );
    assert!(!Real::atan2(&zero, &Real::from_i64(1), 40).is_exact());
    assert!(!zero.asin(40).expect("asin(0)").is_exact());

    assert_eq!(Real::zero().atan(40), Real::zero());
    assert_eq!(
        Real::atan2(&Real::zero(), &Real::from_i64(1), 40),
        Real::zero()
    );
    assert_eq!(Real::zero().asin(40).expect("asin(0)"), Real::zero());
    assert!(Real::zero().atan(40).is_exact());
    assert!(Real::atan2(&Real::zero(), &Real::from_i64(1), 40).is_exact());
    assert!(Real::zero().asin(40).expect("asin(0)").is_exact());
}

// ---------------------------------------------------------------------------
// ComplexMathAmbientBoundaryTests (5 of 6 rows; the two-pi rendering row is refused)
// ---------------------------------------------------------------------------

#[test]
fn complex_math_sin_of_pi_at_thirty_places_prints_the_mpmath_digits() {
    // Sin_OfPiAt30Places_PrintsTheMpmathDigitsAtTheAmbientPrecision.
    let x = Real::pi(30);
    let zeros = |count: usize| "0".repeat(count);
    let cases: [(usize, String, String); 4] = [
        (30, "0".to_string(), "0".to_string()),
        (31, format!("0.{}5", zeros(30)), format!("0.{}5", zeros(30))),
        (
            32,
            format!("0.{}50", zeros(30)),
            format!("0.{}5", zeros(30)),
        ),
        (
            60,
            format!("0.{}502884197169399375105820974944", zeros(30)),
            format!("0.{}502884197169399375105820974944", zeros(30)),
        ),
    ];
    for (ambient, expected, printed) in cases {
        let actual = cm::sin(&Complex::new(x.clone(), Real::zero()), ambient)
            .re()
            .clone();
        let expected = r(&expected);
        assert!(
            actual.sub(&expected).is_zero(),
            "sin(pi(30)) at {ambient} places = {actual}, expected {expected} (mpmath)"
        );
        assert!(
            actual.to_string().starts_with(&printed),
            "sin(pi(30)) at {ambient} places prints {actual}"
        );
    }
}

#[test]
fn complex_math_sin_and_cos_of_the_ambient_pi_stay_the_table_values() {
    // SinAndCos_OfTheAmbientPi_StayTheExactTableValues at 30, 31, 32 and 60.
    for ambient in [30usize, 31, 32, 60] {
        let pi = Real::pi(ambient);
        let sin = cm::sin(&Complex::new(pi.clone(), Real::zero()), ambient)
            .re()
            .clone();
        let cos = cm::cos(&Complex::new(pi.clone(), Real::zero()), ambient)
            .re()
            .clone();
        assert!(sin.is_zero(), "sin(pi) at {ambient} places = {sin}");
        assert_eq!(sin, Real::zero());
        assert_eq!(cos, Real::from_i64(-1));
    }
}
