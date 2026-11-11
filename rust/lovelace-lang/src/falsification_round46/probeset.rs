//! The frozen battery and the generated probe supply.

use lovelace_symbolics::Rat;

use super::observation::{rat_text, rational};

/// The region tag of every generated point.
pub(crate) const GENERATED_REGION: &str = "generated-halving";

/// The region name the complex sweep carries.
pub(crate) const COMPLEX_REGION: &str = "complex";

/// The region tag of the extra exclusion witnesses.
pub(crate) const OUTSIDE_STRIP_REGION: &str = "complex-outside-strip";

/// One probe point offered to the falsification loop, with its region and rationale.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProbePoint {
    /// The region tag the point probes.
    pub(crate) region: String,
    /// The point's own short label.
    pub(crate) label: String,
    /// The exact rational value.
    pub(crate) value: Rat,
    /// Why this input is near the feature its region names.
    pub(crate) why: String,
}

/// One complex sample point.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ComplexProbe {
    /// The region tag.
    pub(crate) region: String,
    /// The exact real part, as a terminating decimal.
    pub(crate) re: String,
    /// The exact imaginary part, as a terminating decimal.
    pub(crate) im: String,
    /// The point's own short label.
    pub(crate) label: String,
    /// Why this input is near the feature its region names.
    pub(crate) why: String,
}

fn point(region: &str, value: Rat, label: &str, why: &str) -> ProbePoint {
    ProbePoint {
        region: region.to_string(),
        label: label.to_string(),
        value,
        why: why.to_string(),
    }
}

fn complex_probe(region: &str, re: &str, im: &str, label: &str, why: &str) -> ComplexProbe {
    ComplexProbe {
        region: region.to_string(),
        re: re.to_string(),
        im: im.to_string(),
        label: label.to_string(),
        why: why.to_string(),
    }
}

/// The 13 points the gate had before round 27, in their original order.
pub(crate) fn legacy_points() -> Vec<Rat> {
    vec![
        rational(-100, 1),
        rational(-5, 1),
        rational(-3, 2),
        rational(-1, 1),
        rational(-1, 3),
        rational(-1, 1000),
        Rat::zero(),
        rational(1, 1000),
        rational(1, 3),
        rational(1, 1),
        rational(3, 2),
        rational(5, 1),
        rational(100, 1),
    ]
}

/// The frozen battery, in its own order: legacy, near-pole, branch-cut, discontinuity, boundary.
pub(crate) fn frozen() -> Vec<ProbePoint> {
    vec![
        point(
            "legacy-interior",
            rational(-100, 1),
            "-100",
            "large negative magnitude (ordering/overflow boundary)",
        ),
        point(
            "legacy-interior",
            rational(-5, 1),
            "-5",
            "interior negative",
        ),
        point(
            "legacy-interior",
            rational(-3, 2),
            "-3/2",
            "interior negative fractional",
        ),
        point(
            "legacy-interior",
            rational(-1, 1),
            "-1",
            "unit negative, sign boundary",
        ),
        point(
            "legacy-interior",
            rational(-1, 3),
            "-1/3",
            "small negative fraction",
        ),
        point(
            "legacy-interior",
            rational(-1, 1000),
            "-1/1000",
            "near-zero negative",
        ),
        point(
            "legacy-interior",
            Rat::zero(),
            "0",
            "exact zero: definedness boundary of x^-1",
        ),
        point(
            "legacy-interior",
            rational(1, 1000),
            "1/1000",
            "near-zero positive",
        ),
        point(
            "legacy-interior",
            rational(1, 3),
            "1/3",
            "small positive fraction",
        ),
        point("legacy-interior", rational(1, 1), "1", "unit positive"),
        point(
            "legacy-interior",
            rational(3, 2),
            "3/2",
            "interior positive fractional",
        ),
        point("legacy-interior", rational(5, 1), "5", "interior positive"),
        point(
            "legacy-interior",
            rational(100, 1),
            "100",
            "large positive magnitude",
        ),
        point(
            "near-pole",
            rational(1, 1_000_000),
            "1/10^6",
            "1e-6 above the pole at 0 (exact rational, epsilon = 1e-6)",
        ),
        point(
            "near-pole",
            rational(-1, 1_000_000),
            "-1/10^6",
            "1e-6 below the pole at 0 (exact rational, epsilon = 1e-6)",
        ),
        point(
            "near-pole",
            rational(1, 1_000_000_000),
            "1/10^9",
            "1e-9 above the pole at 0 (exact rational, epsilon = 1e-9)",
        ),
        point(
            "near-pole",
            rational(-1, 1_000_000_000),
            "-1/10^9",
            "1e-9 below the pole at 0 (exact rational, epsilon = 1e-9)",
        ),
        point(
            "near-branch-cut",
            rational(-1, 1_000_000),
            "-1/10^6",
            "negative side of the cut, 1e-6 from the branch point (exact)",
        ),
        point(
            "near-branch-cut",
            rational(1, 1_000_000),
            "1/10^6",
            "straddles the cut against -1/10^6 at the same magnitude (exact)",
        ),
        point(
            "near-branch-cut",
            rational(-1, 1_000_000_000),
            "-1/10^9",
            "negative side of the cut, 1e-9 from the branch point (exact)",
        ),
        point(
            "near-branch-cut",
            rational(1, 1_000_000_000),
            "1/10^9",
            "straddles the cut against -1/10^9 at the same magnitude (exact)",
        ),
        point(
            "near-branch-cut",
            rational(-1, 4),
            "-1/4",
            "on the cut: sqrt(1/4) = 1/2 exactly, so an exact-root path is exercised, not a rounded one",
        ),
        point(
            "near-branch-cut",
            rational(-4, 1),
            "-4",
            "on the cut: sqrt(4) = 2 exactly",
        ),
        point(
            "near-branch-cut",
            rational(-25, 1),
            "-25",
            "further out on the cut: sqrt(25) = 5 exactly",
        ),
        point(
            "near-discontinuity",
            rational(-1, 999),
            "-1/999",
            "1.001e-3 below the jump at 0 (odd denominator: exercises the rounded real path)",
        ),
        point(
            "near-discontinuity",
            rational(1, 999),
            "1/999",
            "1.001e-3 above the jump at 0",
        ),
        point(
            "near-discontinuity",
            rational(-1, 1_000_003),
            "-1/1000003",
            "1e-6 below the jump at 0 (prime denominator, distinct scale)",
        ),
        point(
            "near-discontinuity",
            rational(1, 1_000_003),
            "1/1000003",
            "1e-6 above the jump at 0",
        ),
        point(
            "assumption-boundary",
            rational(-1, 1_000_000),
            "-1/10^6",
            "outside x >= 0 by 1e-6 (a rule with that side condition must not be licensed here)",
        ),
        point(
            "assumption-boundary",
            rational(-1, 1_000_000_000),
            "-1/10^9",
            "outside x >= 0 by 1e-9 (just past the boundary)",
        ),
        point(
            "assumption-boundary",
            Rat::zero(),
            "0",
            "exactly on the boundary: NonNegative(x) holds, NonZero(x) does not",
        ),
        point(
            "assumption-boundary",
            rational(1, 1_000_000),
            "1/10^6",
            "inside x >= 0 by 1e-6",
        ),
        point(
            "assumption-boundary",
            rational(1, 1_000_000_000),
            "1/10^9",
            "inside x >= 0 by 1e-9 (just inside the boundary)",
        ),
    ]
}

/// The generated points: [-1, 1] divided at every ladder step, de-duplicated by value.
pub(crate) fn generated() -> Vec<ProbePoint> {
    let mut points = Vec::new();
    let mut seen: Vec<Rat> = Vec::new();
    for step in [1_i64, 2, 3, 5, 7] {
        for k in (1 - step)..step {
            let value = rational(k, step);
            if seen.contains(&value) {
                continue;
            }
            seen.push(value.clone());
            points.push(ProbePoint {
                region: GENERATED_REGION.to_string(),
                label: format!("{}/{}", GENERATED_REGION, rat_text(&value)),
                value,
                why: format!(
                    "the {k} of {step} of [-1,1]: an exact rational at ladder step {step}"
                ),
            });
        }
    }
    points
}

/// Frozen first, generated after. The order is the contract.
pub(crate) fn all() -> Vec<ProbePoint> {
    let mut points = frozen();
    points.extend(generated());
    points
}

/// The 12 complex sweep points: quadrants, axes and small moduli.
pub(crate) fn complex_probes() -> Vec<ComplexProbe> {
    vec![
        complex_probe(
            COMPLEX_REGION,
            "1",
            "1",
            "1+i",
            "first quadrant, unit imaginary part",
        ),
        complex_probe(
            COMPLEX_REGION,
            "1",
            "-1",
            "1-i",
            "fourth quadrant, conjugate of 1+i",
        ),
        complex_probe(
            COMPLEX_REGION,
            "-1",
            "1",
            "-1+i",
            "second quadrant (negative real part: |z|^2 != z^2 territory)",
        ),
        complex_probe(
            COMPLEX_REGION,
            "-1",
            "-1",
            "-1-i",
            "third quadrant, conjugate of -1+i",
        ),
        complex_probe(
            COMPLEX_REGION,
            "0",
            "1",
            "i",
            "purely imaginary: re(z) = 0, |z| = 1",
        ),
        complex_probe(
            COMPLEX_REGION,
            "0",
            "-1",
            "-i",
            "purely imaginary, negative: conjugate of i",
        ),
        complex_probe(
            COMPLEX_REGION,
            "0.5",
            "0.5",
            "0.5+0.5i",
            "small modulus (|z| ~ 0.707) near the branch point",
        ),
        complex_probe(
            COMPLEX_REGION,
            "-0.5",
            "1.5",
            "-0.5+1.5i",
            "small negative real part, |Im| = 1.5 < pi",
        ),
        complex_probe(
            COMPLEX_REGION,
            "1.5",
            "-0.5",
            "1.5-0.5i",
            "mirror of the above across the real axis",
        ),
        complex_probe(
            COMPLEX_REGION,
            "2",
            "0.5",
            "2+0.5i",
            "moderate modulus with a small imaginary part",
        ),
        complex_probe(
            COMPLEX_REGION,
            "-2",
            "-0.5",
            "-2-0.5i",
            "moderate negative modulus, |Im| = 0.5 < pi",
        ),
        complex_probe(
            COMPLEX_REGION,
            "2",
            "0",
            "2+0i",
            "complex-typed value on the real axis: exercises the real/complex promotion path",
        ),
    ]
}

/// The exclusion witnesses: the ordinary sweep, then two points outside log's principal strip.
pub(crate) fn complex_exclusion_witnesses() -> Vec<ComplexProbe> {
    let mut witnesses = complex_probes();
    witnesses.push(complex_probe(
        OUTSIDE_STRIP_REGION,
        "0",
        "5",
        "5i",
        "outside log's principal strip (Im z = 5 > pi): the only place log(exp(z)) != z can be observed",
    ));
    witnesses.push(complex_probe(
        OUTSIDE_STRIP_REGION,
        "0",
        "-5",
        "-5i",
        "below the strip (Im z = -5 < -pi), the conjugate-side companion of 5i",
    ));
    witnesses
}

/// Every region a gate sweep must cover.
pub(crate) fn required_regions() -> Vec<&'static str> {
    vec![
        "legacy-interior",
        "near-pole",
        "near-branch-cut",
        "near-discontinuity",
        "assumption-boundary",
        COMPLEX_REGION,
    ]
}
