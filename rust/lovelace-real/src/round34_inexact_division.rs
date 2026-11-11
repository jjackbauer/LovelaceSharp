//! Round 34 — the reference's division at the computation cap.
//!
//! `Real.Divide` stops its digit loop when the budgeted POSITION is reached, BEFORE the
//! repeated-remainder test that would run on the next iteration. A quotient whose period closes
//! exactly AT the cap is therefore a TRUNCATION: its stored digits are the budgeted digits, it is
//! inexact, and it publishes no period metadata. Looking one iteration ahead (the previous Rust
//! behaviour) found that period and returned an exact periodic value the reference never produces:
//! `5*pi/3` at 20 places came back `5.2359877559829887307(6)` where the reference stores
//! `5.23598775598298873076`.
//!
//! The reference also decides "terminating" from the DENOMINATOR alone (`TryTerminateExactly`):
//! it does not cancel the numerator against it. When the two share a factor, the quotient can be a
//! finite decimal that finishes just PAST the cap; the reference still spends the cap in its digit
//! loop and truncates. `pi(100)/6` is that shape: the stored `pi(100)` magnitude happens to be
//! divisible by 3, so the exact value has 101 fractional digits, but the reference publishes the
//! 100-digit truncation. Reducing first would publish the extra digit and make the 100-place
//! pi-resolution row miss its sin/cos table value.
//!
//! Every expected string below was measured with the pinned C# assembly
//! (`rust/target/tmp/inexactprobe`, untracked, driving `Lovelace.Real` under
//! `Real.WithPrecision(places, places)`) and is reproduced here as the durable contract.

use crate::Real;
use crate::transcendental::{e_constant, special_angle_candidate};

fn r(text: &str) -> Real {
    Real::from_str(text).expect("a decimal literal")
}

fn assert_case(label: &str, value: &Real, display: &str, exact: bool) {
    assert_eq!(value.to_string(), display, "{label}: rendered value");
    assert_eq!(value.is_exact(), exact, "{label}: exactness");
}

/// The five inexact-operand probes and the five budgets the contract fixes, plus the four exact
/// controls the change must leave alone. `pi*5/3 at 50` is the refusal table's second row.
#[test]
fn inexact_operand_quotients_are_the_reference_truncations_at_the_budget() {
    let three = Real::from_i64(3);
    let pi20 = Real::pi(20);

    assert_case(
        "5*pi/3 at 20 places",
        &pi20
            .mul(&Real::from_i64(5))
            .div(&three, 20)
            .expect("quotient"),
        "5.23598775598298873076",
        false,
    );
    assert_case(
        "pi/3 at 50 places",
        &Real::pi(50).div(&three, 50).expect("quotient"),
        "1.04719755119659774615421446109316762806572313312503",
        false,
    );
    assert_case(
        "pi/7 at 30 places",
        &Real::pi(30).div(&Real::from_i64(7), 30).expect("quotient"),
        "0.448798950512827605494663340468",
        false,
    );
    assert_case(
        "2*pi/3 at 20 places",
        &pi20
            .mul(&Real::from_i64(2))
            .div(&three, 20)
            .expect("quotient"),
        // The reference's `Normalize` drops the trailing zero the truncation ends on.
        "2.0943951023931954923",
        false,
    );
    assert_case(
        "e/3 at 20 places",
        &e_constant(20).div(&three, 20).expect("quotient"),
        "0.90609394281968174512",
        false,
    );
    assert_case(
        "5*pi/3 at 50 places",
        &Real::pi(50)
            .mul(&Real::from_i64(5))
            .div(&three, 50)
            .expect("quotient"),
        "5.23598775598298873077107230546583814032861566562516",
        false,
    );

    // Exact operands keep their current exact/periodic behaviour at these budgets.
    assert_case(
        "1/3 at 20 places",
        &Real::from_i64(1).div(&three, 20).expect("quotient"),
        "0.(3)",
        true,
    );
    assert_case(
        "1/7 at 20 places",
        &Real::from_i64(1)
            .div(&Real::from_i64(7), 20)
            .expect("quotient"),
        "0.(142857)",
        true,
    );
    assert_case(
        "10/3 at 20 places",
        &Real::from_i64(10).div(&three, 20).expect("quotient"),
        "3.(3)",
        true,
    );
    assert_case(
        "1/4 at 20 places",
        &Real::from_i64(1)
            .div(&Real::from_i64(4), 20)
            .expect("quotient"),
        "0.25",
        true,
    );
}

/// The reference's loop measures the budget, not the operand's provenance: an EXACT operand whose
/// period closes at the cap is truncated too. Measured on the pinned assembly: 1/7 at 5 and 6
/// places is `0.14285` / `0.142857` (inexact, no period); at 7 places it is `0.(142857)` and
/// exact. This is the control that keeps the boundary rule from being implemented as "inexact
/// operands only", which the wide-budget probe falsifies (inexact operands are periodic when the
/// period closes before the cap).
#[test]
fn the_reference_truncates_a_period_that_closes_exactly_at_the_cap() {
    let seven = Real::from_i64(7);
    assert_case(
        "1/7 at 5 places",
        &Real::from_i64(1).div(&seven, 5).expect("quotient"),
        "0.14285",
        false,
    );
    assert_case(
        "1/7 at 6 places",
        &Real::from_i64(1).div(&seven, 6).expect("quotient"),
        "0.142857",
        false,
    );
    assert_case(
        "1/7 at 7 places",
        &Real::from_i64(1).div(&seven, 7).expect("quotient"),
        "0.(142857)",
        true,
    );
}

/// The reference's terminating test (`TryTerminateExactly`) reads the DENOMINATOR only: it never
/// cancels the numerator. `pi(100)`'s stored magnitude is divisible by 3, so the exact value of
/// `pi(100)/6` has 101 fractional digits; the reference still truncates it at the 100-place cap
/// and the last stored digit is the 100th (`...446`), not the exact 101st (`...4465`).
#[test]
fn termination_is_decided_from_the_denominator_like_the_reference() {
    let pi100 = Real::pi(100);
    let quotient = pi100.div(&Real::from_i64(6), 100).expect("quotient");
    assert!(
        !quotient.is_exact(),
        "pi(100)/6 must be the cap truncation, not the exact decimal"
    );
    assert_eq!(
        quotient.exp, -100,
        "the cap must store exactly 100 fractional digits"
    );
    assert_eq!(
        quotient.to_string(),
        "0.5235987755982988730771072305465838140328615665625176368291574320513027343810348331046724708903528446"
    );
}

/// The rule is the BUDGET, not the operand's provenance: with room for the digit that closes the
/// period the reference publishes the period of an INEXACT quotient. Measured on the pinned
/// assembly under `WithPrecision(1000, 1000)`: `pi(20)*5/3` is
/// `5.2359877559829887307(6)`, exact false, PeriodStart 19, PeriodLength 1. This is the control
/// that keeps the boundary rule from being implemented as "truncate whenever an operand is
/// inexact", which the reference falsifies.
#[test]
fn an_inexact_quotient_is_still_periodic_when_its_period_closes_before_the_cap() {
    let value = Real::pi(20)
        .mul(&Real::from_i64(5))
        .div(&Real::from_i64(3), 1000)
        .expect("quotient");
    assert_eq!(value.to_string(), "5.2359877559829887307(6)");
    assert!(
        !value.is_exact(),
        "pi's provenance survives the periodic divide"
    );
    assert_eq!(value.period_start, 19);
    assert_eq!(value.period_len, 1);
}

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

/// The C# row's `SpecialValues(places)` table, built the same way: `sqrt(2)/2` and
/// `sqrt(3)/2` from the places-precision root under the places scope.
fn table_values(places: usize) -> [(Real, Real); 16] {
    let half = r("0.5");
    let neg_half = r("-0.5");
    let neg_one = Real::from_i64(-1);
    let sqrt2_half = Real::from_i64(2)
        .sqrt(places)
        .div(&Real::from_i64(2), places)
        .expect("sqrt(2)/2");
    let sqrt3_half = Real::from_i64(3)
        .sqrt(places)
        .div(&Real::from_i64(2), places)
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
        (neg_one, Real::zero()),
        (sqrt3_half.neg(), half.clone()),
        (sqrt2_half.neg(), sqrt2_half.clone()),
        (neg_half, sqrt3_half),
    ]
}

/// The three rows round 33 refused
/// (`RealTrigPiResolutionPropertyTests.SinAndCos_ExactMultiplesOfTheAmbientPi_KeepTheirExactTableValues`
/// #0/#1/#2), rewritten through the PUBLIC arithmetic the C# row uses: `angle = pi * num / den`
/// under `WithPrecision(places, places)`. The public-arithmetic angle must now be the reference's
/// own candidate value (the construction `TrySpecialAngle` compares against), and Sin/Cos must be
/// the exact table values with the table's exactness flags — at 20, 50 and 100 places for all 16
/// angles.
#[test]
fn exact_multiples_of_the_ambient_pi_match_the_reference_rows() {
    for places in [20usize, 50, 100] {
        let pi = Real::pi(places);
        let values = table_values(places);
        for (index, (num, den)) in ANGLES.iter().enumerate() {
            let angle = if *num == 0 {
                Real::zero()
            } else {
                pi.mul(&Real::from_i64(*num))
                    .div(&Real::from_i64(*den), places)
                    .expect("the angle quotient")
            };
            let reference = if *num == 0 {
                Real::zero()
            } else {
                special_angle_candidate(&pi, *num as u64, *den as u64)
            };
            assert_eq!(
                angle, reference,
                "pi*{num}/{den} at {places} places is not the reference's angle value: {angle}"
            );
            if *num != 0 {
                // pi is inexact, and the reference propagates that provenance through Divide.
                assert!(
                    !angle.is_exact(),
                    "pi*{num}/{den} at {places} places must carry pi's inexact provenance"
                );
            }

            let sin = angle.sin(places);
            let cos = angle.cos(places);
            assert_eq!(
                sin, values[index].0,
                "sin(pi*{num}/{den}) at {places} places = {sin}, expected {}",
                values[index].0
            );
            assert_eq!(
                cos, values[index].1,
                "cos(pi*{num}/{den}) at {places} places = {cos}, expected {}",
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
