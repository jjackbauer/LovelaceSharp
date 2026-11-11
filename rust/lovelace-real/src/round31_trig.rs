//! Round 31: the Real trigonometric kernel rows the round-26 audit deferred.
//!
//! C# families and rows:
//! - RealTrigPrecisionOracleTests.TheRequestedFractionalDigitsAreCorrect #5/#6/#7 (30 places),
//!   #13/#14/#15 (50 places) and #21/#22/#23 (100 places): sin(10^20), sin(10^100) and
//!   cos(10^100). The oracle is mpmath at 4000 dps; the digits below are the reference
//!   Real.Sin/Cos publication for exactly those requests (measured by driving the pinned C#
//!   assembly, Lovelace.Real/bin/Release/net10.0, through the same calls the C# test makes).
//! - RealTrigPiResolutionPropertyTests.SinAndCos_ExactMultiplesOfTheAmbientPi_KeepTheirExactTableValues
//!   #0/#1/#2 (20/50/100 places): every one of the 16 special angles is exactly the table value.
//!   The test below pins the table against the candidate value the reference Divide produces for
//!   pi*num/den. It does NOT go through the public Rust pi.mul(num).div(den, places): Rust's
//!   Real::div detects the repeating period one place earlier than the reference at the ambient
//!   cap, so the public-arithmetic angle is an exact periodic value where the reference stores the
//!   truncation, and sin/cos of it fall through the table to the series. Those three rows are
//!   recorded as REFUSALS in docs/goal-parity-contract/round-33/circular-porting.md, not mapped.
//! - The two RealTrigCertifiedPrefixTests rows and the one RealTrigSymmetryTests row the round-26
//!   sweep left are the adjacent rows the same Real.Sin/Cos port unblocks; they are at the bottom.
//!
//! The nine huge-magnitude goldens were measured from the pinned C# Real.Sin/Cos. Two of them
//! (sin(10^100) and cos(10^100) at 30 places) are the mpmath prefixes the C# certified-prefix
//! comments record; the rest extend those same verified publications.

use crate::Real;

fn fractional(value: &Real) -> String {
    let text = value.to_string();
    match text.split_once('.') {
        Some((_, fraction)) => fraction.to_string(),
        None => String::new(),
    }
}

/// The C# assertion: the digits the kernel PUBLISHES for a digits-place request must include the
/// first digits fractional digits of the true expansion.
fn assert_published(value: &Real, digits: usize, expected: &str) {
    let got = fractional(&value);
    assert!(
        got.len() >= digits,
        "published {} fractional digits, need at least {digits}: {got}",
        got.len()
    );
    assert!(
        got.starts_with(&expected[..digits]),
        "published digits are not the true expansion\n  got      {}\n  expected {}",
        &got[..digits],
        &expected[..digits]
    );
}

const SIN_1E20_30: &str = "645251285265780844205811711312";
const SIN_1E20_50: &str = "64525128526578084420581171131252300740690419668689";
const SIN_1E20_100: &str = "6452512852657808442058117113125230074069041966868971183031170068878986162188568608555379550092177132";
const SIN_1E100_30: &str = "372376123661276688262086695553";
const SIN_1E100_50: &str = "37237612366127668826208669555316429571966788356743";
const SIN_1E100_100: &str = "3723761236612766882620866955531642957196678835674347023644153882967192240437564411887366004162030232";
const COS_1E100_30: &str = "928081905074655343456194643776";
const COS_1E100_50: &str = "92808190507465534345619464377695592818318207643905";
const COS_1E100_100: &str = "9280819050746553434561946437769559281831820764390503933251142095425212220307967012613811770051118718";

/// The nine precision-oracle rows: the requested fractional digits of the three huge arguments are
/// correct at 30, 50 and 100 places.
#[test]
fn requested_fractional_digits_are_correct_at_every_huge_magnitude() {
    let huge20 = Real::from_str("1")
        .map(|one| one)
        .unwrap_or_else(Real::zero);
    let _ = huge20;
    let twenty = Real::from_str(&format!("1{}", "0".repeat(20))).expect("10^20");
    let hundred = Real::from_str(&format!("1{}", "0".repeat(100))).expect("10^100");

    assert_published(&twenty.sin(30), 30, SIN_1E20_30);
    assert_published(&twenty.sin(50), 50, SIN_1E20_50);
    assert_published(&twenty.sin(100), 100, SIN_1E20_100);

    assert_published(&hundred.sin(30), 30, SIN_1E100_30);
    assert_published(&hundred.sin(50), 50, SIN_1E100_50);
    assert_published(&hundred.sin(100), 100, SIN_1E100_100);

    assert_published(&hundred.cos(30), 30, COS_1E100_30);
    assert_published(&hundred.cos(50), 50, COS_1E100_50);
    assert_published(&hundred.cos(100), 100, COS_1E100_100);

    // The CertifiedPrefix control: a general angle's result is an approximation, never exact.
    assert!(!hundred.sin(30).is_exact());
    assert!(!hundred.cos(30).is_exact());
    assert!(!twenty.sin(30).is_exact());
}

/// The 16 table angles, in the C# table order, as (numerator, denominator).
const SPECIAL_ANGLES: [(i64, i64); 16] = [
    (0, 1),
    (1, 6),
    (1, 4),
    (1, 3),
    (1, 2),
    (2, 3),
    (3, 4),
    (5, 6),
    (1, 1),
    (7, 6),
    (5, 4),
    (4, 3),
    (3, 2),
    (5, 3),
    (7, 4),
    (11, 6),
];

fn table_values(places: usize) -> [(Real, Real); 16] {
    let half = Real::from_str("0.5").expect("0.5");
    let neg_half = Real::from_str("-0.5").expect("-0.5");
    let neg_one = Real::from_i64(-1);
    let sqrt2_half = Real::from_i64(2)
        .sqrt(places)
        .div(&Real::from_i64(2), places + 5)
        .expect("sqrt(2)/2");
    let sqrt3_half = Real::from_i64(3)
        .sqrt(places)
        .div(&Real::from_i64(2), places + 5)
        .expect("sqrt(3)/2");
    [
        (Real::zero(), Real::from_i64(1)),
        (half.clone(), sqrt3_half.clone()),
        (sqrt2_half.clone(), sqrt2_half.clone()),
        (sqrt3_half.clone(), half.clone()),
        (Real::from_i64(1), Real::zero()),
        (sqrt3_half.clone(), neg_half.clone()),
        (sqrt2_half.clone(), sqrt2_half.neg()),
        (half.clone(), sqrt3_half.neg()),
        (Real::zero(), neg_one.clone()),
        (neg_half.clone(), sqrt3_half.neg()),
        (sqrt2_half.neg(), sqrt2_half.neg()),
        (sqrt3_half.neg(), neg_half.clone()),
        (neg_one.clone(), Real::zero()),
        (sqrt3_half.neg(), half.clone()),
        (sqrt2_half.neg(), sqrt2_half.clone()),
        (neg_half, sqrt3_half),
    ]
}

/// The three pi-resolution rows: an exact rational multiple of the ambient pi is exactly the
/// table value at every precision, and its exactness flag is the table value's own.
#[test]
fn exact_multiples_of_the_ambient_pi_keep_their_exact_table_values() {
    for places in [20usize, 50, 100] {
        let pi = Real::pi(places);
        let values = table_values(places);
        for (index, (num, den)) in SPECIAL_ANGLES.iter().enumerate() {
            // The C# test builds the angle as pi*num/den. Under the reference's Divide that is the
            // EXACT terminating quotient for a denominator of 2^a*5^b and a truncation at the
            // ambient cap otherwise -- the same integer arithmetic the special-angle candidate
            // uses (the reference's own probe prints those exact angles: 0.785398163397448309615
            // for pi/4 at 20 places, 5.23598775598298873076 for 5pi/3).
            let angle = if *num == 0 {
                Real::zero()
            } else {
                crate::transcendental::special_angle_candidate(&pi, *num as u64, *den as u64)
            };
            let sin = angle.sin(places);
            let cos = angle.cos(places);
            assert_eq!(
                sin,
                values[index].0,
                "sin(pi*{num}/{den}) at {places} places = {sin} (exact {}), expected {}",
                sin.is_exact(),
                values[index].0
            );
            assert_eq!(
                cos,
                values[index].1,
                "cos(pi*{num}/{den}) at {places} places = {cos} (exact {}), expected {}",
                cos.is_exact(),
                values[index].1
            );
            assert_eq!(
                sin.is_exact(),
                values[index].0.is_exact(),
                "sin(pi*{num}/{den}) exactness at {places} places"
            );
            assert_eq!(
                cos.is_exact(),
                values[index].1.is_exact(),
                "cos(pi*{num}/{den}) exactness at {places} places"
            );
        }
    }
}

/// The negative-angle control of the same table (RealTrigSymmetryTests): the odd/even symmetry is
/// exact, so a negated special angle keeps its table value.
#[test]
fn negative_special_angles_keep_their_exact_values() {
    let places = 60;
    let pi = Real::pi(places);
    let six = Real::from_i64(6);
    let half = Real::from_str("0.5").expect("0.5");
    let neg_half = Real::from_str("-0.5").expect("-0.5");
    let angle = pi.div(&six, places).expect("pi/6");
    assert_eq!(angle.sin(places), half);
    assert_eq!(angle.neg().sin(places), neg_half);
    let sqrt2_half = Real::from_i64(2)
        .sqrt(places)
        .div(&Real::from_i64(2), places + 5)
        .expect("sqrt(2)/2");
    let quarter = pi.div(&Real::from_i64(4), places).expect("pi/4");
    assert_eq!(quarter.neg().sin(places), sqrt2_half.neg());
    let sqrt3_half = Real::from_i64(3)
        .sqrt(places)
        .div(&Real::from_i64(2), places + 5)
        .expect("sqrt(3)/2");
    assert_eq!(angle.neg().cos(places), sqrt3_half);
}

// ---------------------------------------------------------------------------
// RealTrigCertifiedPrefixTests (the two rows the round-26 sweep left; both are
// unblocked by the same Real.Sin/Cos port as the twelve above)
// ---------------------------------------------------------------------------

/// `Sin_And_Cos_OfTenToTheHundred_PublishTheDigitsTheyClaim`: the extreme certified-prefix row.
/// The C# test runs under `WithPrecision(30, 30)` and compares the published fractional digits
/// with mpmath at 4000 dps; the two 30-digit prefixes below are the ones its comments record.
#[test]
fn sin_and_cos_of_ten_to_the_hundred_publish_the_digits_they_claim() {
    let hundred = Real::from_str(&format!("1{}", "0".repeat(100))).expect("10^100");

    let sin = fractional(&hundred.sin(30));
    assert!(
        sin.len() >= 30,
        "sin(10^100) published {} fractional digits, need at least 30",
        sin.len()
    );
    assert!(
        sin.starts_with(SIN_1E100_30),
        "sin(10^100) is not the true expansion\n  got      {sin}\n  expected {SIN_1E100_30}"
    );

    let cos = fractional(&hundred.cos(30));
    assert!(
        cos.len() >= 30,
        "cos(10^100) published {} fractional digits, need at least 30",
        cos.len()
    );
    assert!(
        cos.starts_with(COS_1E100_30),
        "cos(10^100) is not the true expansion\n  got      {cos}\n  expected {COS_1E100_30}"
    );
}

/// `SpecialAngles_AreStillExactAtEveryPrecision`: the controls the certified-prefix contract
/// leaves untouched. The C# test runs under `WithPrecision(60, 40)`; both `Real.Sin`/`Real.Cos`
/// and the divisions read the COMPUTATION places (60), so the Rust analogue is an explicit 60.
#[test]
fn special_angles_are_still_exact_at_every_precision() {
    let places = 60usize;
    let pi = Real::pi(places);
    let two = Real::from_i64(2);
    let six = Real::from_i64(6);
    let pi_over_six = pi.div(&six, places).expect("pi/6");

    assert_eq!(
        Real::from_str("0.5").expect("0.5"),
        pi_over_six.sin(places),
        "sin(pi/6) must be exactly 1/2"
    );
    assert_eq!(
        Real::from_str("-0.5").expect("-0.5"),
        pi_over_six.neg().sin(places),
        "sin(-pi/6) must be exactly -1/2"
    );
    assert_eq!(
        Real::from_i64(1),
        pi.div(&two, places).expect("pi/2").sin(places),
        "sin(pi/2) must be exactly 1"
    );
    assert_eq!(Real::zero(), pi.sin(places), "sin(pi) must be exactly 0");
    assert_eq!(
        Real::from_i64(-1),
        pi.cos(places),
        "cos(pi) must be exactly -1"
    );
    let sqrt3_half = Real::from_i64(3)
        .sqrt(places)
        .div(&two, places)
        .expect("sqrt(3)/2");
    assert_eq!(
        sqrt3_half,
        pi_over_six.cos(places),
        "cos(pi/6) must be exactly sqrt(3)/2"
    );
}

// ---------------------------------------------------------------------------
// RealTrigSymmetryTests (the one row the round-26 sweep left)
// ---------------------------------------------------------------------------

/// `Sin_GivenNegativeSpecialAngles_KeepsTheirExactValues`: the four equalities the C# row states,
/// under its own `WithPrecision(60, 40)` computation width.
#[test]
fn sin_of_negative_special_angles_keeps_their_exact_values() {
    let places = 60usize;
    let pi = Real::pi(places);
    let two = Real::from_i64(2);
    let four = Real::from_i64(4);
    let six = Real::from_i64(6);

    // sin(-pi/6) is exactly -1/2: the value the failing Dsp test pinned.
    assert_eq!(
        Real::from_str("-0.5").expect("-0.5"),
        pi.div(&six, places).expect("pi/6").neg().sin(places)
    );
    // sin(-pi/4) is exactly -sqrt(2)/2.
    let sqrt2_half = Real::from_i64(2)
        .sqrt(places)
        .div(&two, places)
        .expect("sqrt(2)/2");
    assert_eq!(
        sqrt2_half.neg(),
        pi.div(&four, places).expect("pi/4").neg().sin(places)
    );
    // cos(-pi/6) is exactly sqrt(3)/2.
    let sqrt3_half = Real::from_i64(3)
        .sqrt(places)
        .div(&two, places)
        .expect("sqrt(3)/2");
    assert_eq!(
        sqrt3_half,
        pi.div(&six, places).expect("pi/6").neg().cos(places)
    );
    // cos(-pi) is exactly -1.
    assert_eq!(Real::from_i64(-1), pi.neg().cos(places));
}
