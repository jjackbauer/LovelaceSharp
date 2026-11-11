//! The 32 mirrored C# falsification-harness cases.

pub(crate) use super::boundary::*;
pub(crate) use super::gate::*;
pub(crate) use super::looping::*;
pub(crate) use super::observation::*;
pub(crate) use super::probe::*;
pub(crate) use super::probeset::*;

use std::cmp::Ordering;

use lovelace_real::Real;
use lovelace_symbolics::assumptions::AssumptionSet;
use lovelace_symbolics::{Expr, Rat};

use crate::complex::Complex;
use crate::numeric::Num;

mod gate_api;
mod loop_api;
mod probe_api;

/// The gate's historical tolerance.
pub(crate) fn tolerance() -> ComparisonTolerance {
    ComparisonTolerance::ten_to_the_minus_eleven()
}

/// Every real probe value, legacy first.
pub(crate) fn all_points() -> Vec<Rat> {
    frozen().into_iter().map(|point| point.value).collect()
}

/// The C# FuzzIdentity: every point must agree, and no side may throw unexpectedly.
pub(crate) fn fuzz_identity(rule_id: &str, lhs: &Expr, rhs: &Expr, points: &[Rat]) {
    let stated = tolerance();
    let mut unexpected: Vec<String> = Vec::new();
    for point in points {
        let value = Num::Exact(point.clone());
        let left = match default_eval(lhs, "x", &value) {
            Ok(value) => value,
            Err(EvalFailure::Undefined(_)) => continue,
            Err(failure) => {
                unexpected.push(format!(
                    "{rule_id} at x={}: {}",
                    rat_text(point),
                    failure.describe()
                ));
                continue;
            }
        };
        let right = match default_eval(rhs, "x", &value) {
            Ok(value) => value,
            Err(EvalFailure::Undefined(_)) => continue,
            Err(failure) => {
                unexpected.push(format!(
                    "{rule_id} at x={}: {}",
                    rat_text(point),
                    failure.describe()
                ));
                continue;
            }
        };
        let (agrees, _) =
            ObservationClassifier::agrees(&left, &right, &stated).expect("the delta compares");
        assert!(
            agrees,
            "Rule '{rule_id}' falsified: {lhs} != {rhs} at x={}",
            rat_text(point)
        );
    }
    assert!(
        unexpected.is_empty(),
        "rule(s) threw an unexpected exception instead of being falsified: {}",
        unexpected.join(" | ")
    );
}

/// A complex numeric value at an exact terminating decimal pair.
pub(crate) fn complex_value(re: &str, im: &str) -> Num {
    Num::Complex(Complex::new(
        Real::from_str(re).expect("an exact real part"),
        Real::from_str(im).expect("an exact imaginary part"),
    ))
}
