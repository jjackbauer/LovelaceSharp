//! Round 26: counted parity for the trigonometric kernel against `Lovelace.Real.Tests`.
//!
//! The C# tests call `Real.Sin`/`Real.Cos` under an ambient `WithPrecision` scope and read the
//! published fractional digits off `ToString()`. The Rust kernel takes the budget explicitly and
//! returns the value truncated to that budget; `sin_cos_raw` exposes the untruncated pair, so
//! slicing its first `digits` fractional characters reproduces exactly what the reference publishes
//! for that display window (including a trailing zero the canonical Rust renderer would drop).
//!
//! Rows whose assertion the Rust kernel does not satisfy are NOT mapped here; the round's report
//! records them as refusals with the measured difference.

use crate::Real;

fn real(text: &str) -> Real {
    Real::from_str(text).unwrap_or_else(|| panic!("literal {text}"))
}

fn fractional(value: &Real) -> String {
    let text = value.to_string();
    match text.split_once('.') {
        Some((_, frac)) => frac.to_string(),
        None => String::new(),
    }
}

/// The first `digits` fractional digits the reference publishes for `sin(x)`.
fn published_sin(x: &Real, digits: usize) -> String {
    let raw = x.sin_cos_raw(digits).0;
    fractional(&raw).chars().take(digits).collect()
}

fn published_cos(x: &Real, digits: usize) -> String {
    let raw = x.sin_cos_raw(digits).1;
    fractional(&raw).chars().take(digits).collect()
}

fn assert_prefix(got: &str, expected: &str, label: &str) {
    assert!(
        got.len() >= expected.len(),
        "{label}: published {} fractional digits, need at least {}: {got}",
        got.len(),
        expected.len()
    );
    assert!(
        got.starts_with(expected),
        "{label}: published digits are not the true expansion\n  got      {got}\n  expected {expected}"
    );
}

fn tiny(places: usize) -> Real {
    real(&format!("0.{}1", "0".repeat(places - 1)))
}

fn below(actual: &Real, limit: &Real) -> bool {
    actual.cmp(limit) == std::cmp::Ordering::Less
}

fn special_values(places: usize) -> Vec<(Real, Real)> {
    let half = real("0.5");
    let neg_half = real("-0.5");
    let neg_one = real("-1");
    let sqrt2_half = Real::from_i64(2)
        .sqrt(places)
        .div(&Real::from_i64(2), places)
        .expect("sqrt(2)/2");
    let sqrt3_half = Real::from_i64(3)
        .sqrt(places)
        .div(&Real::from_i64(2), places)
        .expect("sqrt(3)/2");
    vec![
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

// ---------------------------------------------------------------------------
// RealTrigCertifiedPrefixTests — the certified-prefix contract (15 rows)
// ---------------------------------------------------------------------------

#[test]
fn certified_sin_of_one_publishes_the_digits_it_claims() {
    // Sin_OfOne_PublishesTheDigitsItClaims(30, ...)
    let got = published_sin(&Real::from_i64(1), 30);
    assert_eq!(got, "841470984807896506652502321630");
}

#[test]
fn certified_sin_of_one_at_fifty_places_publishes_fifty_correct_digits() {
    // Sin_OfOne_AtFiftyPlaces_PublishesFiftyCorrectDigits
    assert_prefix(
        &published_sin(&Real::from_i64(1), 50),
        "84147098480789650665250232163029899962256306079837",
        "sin(1) at 50 places",
    );
}

#[test]
fn certified_sin_of_one_at_two_hundred_places_publishes_the_claimed_prefix() {
    // Sin_OfOne_AtTwoHundredPlaces_PublishesTwoHundredCorrectDigits: a 200-place request must not
    // be short, and its first 100 digits are the mpmath prefix.
    let got = published_sin(&Real::from_i64(1), 200);
    assert_eq!(got.len(), 200);
    assert_prefix(
        &got,
        "8414709848078965066525023216302989996225630607983710656727517099919104043912396689486397435430526958",
        "sin(1) at 200 places",
    );
}

#[test]
fn certified_cos_of_one_publishes_the_digits_it_claims() {
    // Cos_OfOne_PublishesTheDigitsItClaims rows 30 and 50.
    assert_eq!(published_cos(&Real::from_i64(1), 30), "540302305868139717400936607442");
    assert_eq!(
        published_cos(&Real::from_i64(1), 50),
        "54030230586813971740093660744297660373231042061792"
    );
}

#[test]
fn certified_sin_of_ten_publishes_the_digits_it_claims() {
    // The 30-place row is the audit's "29 correct of 30" defect; each request asserts exactly as
    // many digits as it asks for.
    let ten = Real::from_i64(10);
    assert_eq!(published_sin(&ten, 30), "544021110889369813404747661851");
    assert_eq!(
        published_sin(&ten, 50),
        "54402111088936981340474766185137728168364301291622"
    );
    assert_eq!(
        published_sin(&ten, 100),
        "5440211108893698134047476618513772816836430129162238915741840126167572096404934257070756738949832161"
    );
}

#[test]
fn certified_argument_aware_width_applies_at_every_magnitude() {
    // ArgumentAwareWidth_AppliesAtEveryMagnitude(0, 5, 50, 100): the structural half — sin/cos of a
    // non-zero decimal literal is irrational, so the result must not claim exactness at any width.
    for exponent in [0usize, 5, 50, 100] {
        let value = real(&format!("1{}", "0".repeat(exponent)));
        assert!(
            !value.sin(30).is_exact(),
            "sin(1e{exponent}) at 30 places claims exactness"
        );
        assert!(
            !value.cos(30).is_exact(),
            "cos(1e{exponent}) at 30 places claims exactness"
        );
    }
}

#[test]
fn certified_wider_ambient_scope_is_preserved() {
    // AWiderAmbientScope_IsPreserved: a wider request must not narrow a narrower one.
    assert_prefix(
        &published_sin(&Real::from_i64(1), 30),
        "841470984807896506652502321630",
        "sin(1) at 30 under a wider scope",
    );
    assert_prefix(
        &published_sin(&Real::from_i64(1), 300),
        "8414709848078965066525023216302989996225630607983710656727517099919104043912396689486397435430526958",
        "sin(1) at 300 under a wider scope",
    );
}

#[test]
fn certified_published_width_is_the_display_limit_for_non_periodic_values() {
    // PublishedWidth_IsTheDisplayLimit_ForEveryNonPeriodicValue: 30 fractional places, whichever
    // branch the renderer takes, and the digits that are published are the right ones.
    assert_eq!(fractional(&Real::from_i64(2).sqrt(30)).len(), 30);
    let sin = published_sin(&Real::from_i64(1), 30);
    assert_eq!(sin.len(), 30);
    assert!(sin.starts_with("841470984807896506652502321630"));
}

#[test]
fn certified_small_angles_keep_their_magnitude() {
    // SmallAngles_KeepTheirMagnitude: the width rule must not collapse the angle to zero.
    let value = real("0.00001").sin(30);
    assert_ne!(value, Real::zero());
    assert_prefix(
        &published_sin(&real("0.00001"), 30),
        "000009999999999833333333334166",
        "sin(0.00001) at 30 places",
    );
}

// ---------------------------------------------------------------------------
// RealTrigPrecisionOracleTests — the requested digits are correct (15 rows + 1 falsifier)
// ---------------------------------------------------------------------------

const SIN_1_100: &str = "8414709848078965066525023216302989996225630607983710656727517099919104043912396689486397435430526958";
const COS_1_100: &str = "5403023058681397174009366074429766037323104206179222276700972553811003947744717645179518560871830893";
const SIN_10_100: &str = "5440211108893698134047476618513772816836430129162238915741840126167572096404934257070756738949832161";
const COS_10_100: &str = "8390715290764524522588639478240648345199301651331685468359537310487925868662707684009337127604221389";
const SIN_1E6_100: &str = "3499935021712929521176524867807714690614066053287162738570590546446412263954505050656668976688940081";

#[test]
fn precision_oracle_sin_of_one_is_correct_at_every_requested_count() {
    // TheRequestedFractionalDigitsAreCorrect("Sin_1_30"/"Sin_1_50"/"Sin_1_100")
    let one = Real::from_i64(1);
    assert_eq!(published_sin(&one, 30), "841470984807896506652502321630");
    assert_eq!(
        published_sin(&one, 50),
        "84147098480789650665250232163029899962256306079837"
    );
    assert_eq!(published_sin(&one, 100), SIN_1_100);
}

#[test]
fn precision_oracle_cos_of_one_is_correct_at_every_requested_count() {
    // TheRequestedFractionalDigitsAreCorrect("Cos_1_30"/"Cos_1_50"/"Cos_1_100")
    let one = Real::from_i64(1);
    assert_eq!(published_cos(&one, 30), "540302305868139717400936607442");
    assert_eq!(
        published_cos(&one, 50),
        "54030230586813971740093660744297660373231042061792"
    );
    assert_eq!(published_cos(&one, 100), COS_1_100);
}

#[test]
fn precision_oracle_sin_of_ten_is_correct_at_every_requested_count() {
    // TheRequestedFractionalDigitsAreCorrect("Sin_10_30"/"Sin_10_50"/"Sin_10_100")
    let ten = Real::from_i64(10);
    assert_eq!(published_sin(&ten, 30), "544021110889369813404747661851");
    assert_eq!(
        published_sin(&ten, 50),
        "54402111088936981340474766185137728168364301291622"
    );
    assert_eq!(published_sin(&ten, 100), SIN_10_100);
}

#[test]
fn precision_oracle_cos_of_ten_is_correct_at_every_requested_count() {
    // TheRequestedFractionalDigitsAreCorrect("Cos_10_30"/"Cos_10_50"/"Cos_10_100")
    let ten = Real::from_i64(10);
    assert_eq!(published_cos(&ten, 30), "839071529076452452258863947824");
    assert_eq!(
        published_cos(&ten, 50),
        "83907152907645245225886394782406483451993016513316"
    );
    assert_eq!(published_cos(&ten, 100), COS_10_100);
}

#[test]
fn precision_oracle_sin_of_one_million_is_correct_at_every_requested_count() {
    // TheRequestedFractionalDigitsAreCorrect("Sin_1e6_30"/"Sin_1e6_50"/"Sin_1e6_100"):
    // the magnitudes a constant guard cannot pay for (1e6 has seven integer digits).
    let million = Real::from_i64(1_000_000);
    assert_eq!(published_sin(&million, 30), "349993502171292952117652486780");
    assert_eq!(
        published_sin(&million, 50),
        "34999350217129295211765248678077146906140660532871"
    );
    assert_eq!(published_sin(&million, 100), SIN_1E6_100);
}

fn count_correct_digits(published: &str, truth: &str) -> usize {
    let mut correct = 0;
    while correct < published.len()
        && correct < truth.len()
        && published.as_bytes()[correct] == truth.as_bytes()[correct]
    {
        correct += 1;
    }
    correct
}

#[test]
fn precision_oracle_rejects_a_truncated_value() {
    // TheOracleRejectsATruncatedValue: the counting machinery must catch a value that agrees for
    // 40 places and invents the rest, while not under-counting a meaningful prefix.
    let truncated = "841470984807896506652502321630298999622599999999999999999999";
    let correct = count_correct_digits(truncated, SIN_1_100);
    assert!(correct < 60, "the oracle accepted a deliberately wrong value ({correct} correct)");
    assert!(correct >= 30, "the oracle under-counted a value right for at least 30 places ({correct})");
}

// ---------------------------------------------------------------------------
// RealTrigPiResolutionPropertyTests — the finer-pi reduction (9 rows)
// ---------------------------------------------------------------------------

#[test]
fn pi_resolution_sin_integer_multiples_reproduce_the_distance_to_pi() {
    // Sin_IntegerMultiplesOfPiTruncatedAtDPlaces_ReproduceTheDistanceToPi at 40/100/200 places:
    // sin(k*pi(d)) = (-1)^(k+1) * k * (pi - pi(d)) within 10^-(d+5).
    for places in [40usize, 100, 200] {
        let pi = Real::pi(places);
        let delta = Real::pi(places + 100).sub(&pi);
        let tolerance = tiny(places + 5);
        for k in 1..=12i64 {
            let actual = pi.mul(&Real::from_i64(k)).sin(30);
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
fn pi_resolution_cos_of_half_pi_is_the_half_distance_to_pi() {
    // Cos_OfHalfPiTruncatedAtDPlaces_IsTheHalfDistanceToPi.
    for places in [40usize, 100, 200] {
        let pi = Real::pi(places);
        let half_pi = pi.div(&Real::from_i64(2), places).expect("pi/2");
        let delta = Real::pi(places + 100).sub(&pi);
        let half_delta = delta.div(&Real::from_i64(2), places + 100).expect("delta/2");
        let tolerance = tiny(places + 5);
        let actual = half_pi.cos(30);
        assert!(
            actual.cmp(&Real::zero()) == std::cmp::Ordering::Greater,
            "cos(pi({places})/2) = {actual} must be positive"
        );
        assert!(
            below(&actual.sub(&half_delta).abs(), &tolerance),
            "cos(pi({places})/2) at 30 places = {actual}, expected {half_delta}"
        );
    }
}

#[test]
fn pi_resolution_rational_multiples_stay_on_the_exact_angle() {
    // SinAndCos_RationalMultiplesOfPiTruncatedAtDPlaces_StayOnTheExactAngle: the angle is built as
    // a non-periodic decimal truncated well past the ambient budget; the budget asserted is
    // 10^-(30+5).
    const ANGLES: [(i64, i64); 16] = [
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
    for places in [50usize, 100, 200] {
        let high = places + 100;
        let pi = Real::pi(places);
        let values = special_values(high);
        let tolerance = tiny(35);
        for (index, (num, den)) in ANGLES.iter().enumerate() {
            if *num == 0 {
                continue;
            }
            let angle = pi
                .mul(&Real::from_i64(*num))
                .div(&Real::from_i64(*den), high)
                .expect("pi*num/den");
            // The reference's `Sin(x, digits)` returns the guarded series value; only the
            // renderer truncates to the display window. The raw pair is that value, so comparing
            // it with the high-precision table is the reference's assertion (not a weaker one).
            let (sin, cos) = angle.sin_cos_raw(30);
            let (expected_sin, expected_cos) = &values[index];
            assert!(
                below(&sin.sub(expected_sin).abs(), &tolerance),
                "sin(pi({places})*{num}/{den}) at 30 places = {sin}, exact value {expected_sin}"
            );
            assert!(
                below(&cos.sub(expected_cos).abs(), &tolerance),
                "cos(pi({places})*{num}/{den}) at 30 places = {cos}, exact value {expected_cos}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// RealTrigSymmetryTests — odd/even symmetry (2 rows)
// ---------------------------------------------------------------------------

fn symmetry_angles() -> Vec<Real> {
    let pi = Real::pi(60);
    let multiples = [
        (1i64, 6i64),
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
        (13, 6),
        (25, 4),
    ];
    let mut angles = Vec::new();
    for (num, den) in multiples {
        let angle = if num == 1 && den == 1 {
            pi.clone()
        } else {
            pi.mul(&Real::from_i64(num))
                .div(&Real::from_i64(den), 60)
                .expect("pi*num/den")
        };
        angles.push(angle);
    }
    for text in [
        "0.52359877559829887307710723054658381403286156656251763682915743205130273438103483310467247089035284466",
        "1",
        "2",
        "7",
        "0.5",
        "-3",
    ] {
        angles.push(real(text));
    }
    angles
}

#[test]
fn symmetry_sin_of_a_negative_angle_is_the_negated_sine_of_its_magnitude() {
    // Sin_GivenNegativeAngle_IsExactlyTheNegatedSineOfItsMagnitude.
    for angle in symmetry_angles() {
        assert_eq!(
            angle.sin(60),
            angle.neg().sin(60).neg(),
            "sin({angle}) != -sin(-{angle})"
        );
    }
}

#[test]
fn symmetry_cos_of_a_negative_angle_is_the_cosine_of_its_magnitude() {
    // Cos_GivenNegativeAngle_IsExactlyTheCosineOfItsMagnitude.
    for angle in symmetry_angles() {
        assert_eq!(
            angle.cos(60),
            angle.neg().cos(60),
            "cos({angle}) != cos(-{angle})"
        );
    }
}
