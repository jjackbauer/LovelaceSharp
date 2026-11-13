//! Exhaustive small-domain signed-integer verification (PROOF-004, Int side).
//!
//! `lovelace-nat` carries the exhaustive digit-algorithm tests for the Lean model. The signed
//! tower on top of it — sign plus `Nat` magnitude, no negative zero — has no Lean module, but the
//! same row requires exhaustive small-domain verification of the arithmetic that the language
//! actually exposes, including the ordering laws. This module supplies that half:
//!
//! * every `a, b` in `[-64, 64]` (16 641 pairs) for `+`, `-`, `*`, `div_rem_signed` and
//!   ordering, each result compared against the `i128` value (an independent oracle that cannot
//!   overflow on this domain);
//! * every `a, b, c` in `[-16, 16]` (35 937 triples) for associativity, distributivity,
//!   transitivity and antisymmetry — identity and inverse laws are checked on the pair domain;
//! * the limb boundaries `2^64 - 1`, `2^64`, `2^64 + 1` for sign-carrying products, against
//!   decimal constants computed independently of the code under test.
//!
//! These are boundary tests, not a proof: [proof-theorem-map.md] records what the Lean mapping
//! covers and where the signed tower sits outside it.
//!
//! [proof-theorem-map.md]: ../../../docs/parity/proof-theorem-map.md

use super::Int;

/// Exclusive bound of the exhaustive pair domain: values are `-SMALL..=SMALL`.
const SMALL: i64 = 64;

/// Exclusive bound of the exhaustive triple domain: values are `-TRIPLE..=TRIPLE`.
const TRIPLE: i64 = 16;

/// Builds the case value through the public constructor.
fn int(value: i64) -> Int {
    Int::from_i64(value)
}

/// Every value of the pair domain, ascending.
fn pair_domain() -> Vec<i64> {
    (-SMALL..=SMALL).collect()
}

/// Every value of the triple domain, ascending.
fn triple_domain() -> Vec<i64> {
    (-TRIPLE..=TRIPLE).collect()
}

/// Addition.lean `addDigits_correct` and Subtraction.lean `subDigits_correct` lifted through the
/// sign, plus the group laws (`a + 0 = a`, `a - a = 0`, `a + (-a) = 0`).
///
/// The Rust contract is exact signed addition; the Lean theorems constrain the magnitude steps the
/// code delegates to `Nat::add`/`Nat::sub`. Every pair in the domain is checked against `i128`.
#[test]
fn proof_int_addition_and_subtraction_match_i128_exhaustively() {
    for &a in &pair_domain() {
        for &b in &pair_domain() {
            let left = int(a);
            let right = int(b);

            assert_eq!(
                left.add(&right).to_decimal(),
                (i128::from(a) + i128::from(b)).to_string(),
                "signed addition: {a} + {b} must equal the i128 sum"
            );
            assert_eq!(
                left.sub(&right).to_decimal(),
                (i128::from(a) - i128::from(b)).to_string(),
                "signed subtraction: {a} - {b} must equal the i128 difference"
            );

            // Commutativity and the additive identity.
            assert_eq!(left.add(&right), right.add(&left), "{a} + {b} must commute");
            assert_eq!(left.add(&Int::zero()), left, "{a} + 0 must be {a}");
            assert_eq!(left.sub(&left), Int::zero(), "{a} - {a} must be zero");

            // Additive inverse, and no negative zero anywhere in the domain.
            let inverse = left.add(&left.negate());
            assert!(inverse.is_zero(), "{a} + (-{a}) must be zero");
            assert_eq!(
                inverse.sign(),
                0,
                "zero must be non-negative after {a} + (-{a})"
            );
        }
    }
}

/// Multiplication.lean `mulDigits_correct` lifted through the sign, plus the multiplicative
/// identities and the sign rule `sign(a*b) = sign(a)*sign(b)`.
#[test]
fn proof_int_multiplication_matches_i128_exhaustively() {
    for &a in &pair_domain() {
        for &b in &pair_domain() {
            let left = int(a);
            let right = int(b);

            assert_eq!(
                left.mul(&right).to_decimal(),
                (i128::from(a) * i128::from(b)).to_string(),
                "signed multiplication: {a} * {b} must equal the i128 product"
            );
            assert_eq!(left.mul(&right), right.mul(&left), "{a} * {b} must commute");
            assert_eq!(left.mul(&int(1)), left, "{a} * 1 must be {a}");
            assert!(
                left.mul(&Int::zero()).is_zero(),
                "{a} * 0 must be zero and must not carry a sign"
            );
            assert_eq!(
                i64::from(left.mul(&right).sign()),
                i64::from(left.sign()) * i64::from(right.sign()),
                "sign({a} * {b}) must be the product of the signs"
            );
        }
    }
}

/// Division.lean `divDigits_correct` under the reference's truncating convention:
/// `q * d + r = a`, `|r| < |d|` and `sign(r) = sign(a)`.
#[test]
fn proof_int_division_is_exact_truncating_and_bounded_exhaustively() {
    for &a in &pair_domain() {
        for &d in &pair_domain() {
            let dividend = int(a);
            let divisor = int(d);
            let Some((quotient, remainder)) = dividend.div_rem_signed(&divisor) else {
                assert_eq!(d, 0, "only a zero divisor may return None, got divisor {d}");
                continue;
            };

            // The identity is checked in i128 for the quotient/remainder and in Int for the
            // reconstruction, so neither side is trusted on its own.
            assert_eq!(
                quotient.to_decimal(),
                (i128::from(a) / i128::from(d)).to_string(),
                "signed quotient: {a} / {d} must truncate toward zero"
            );
            assert_eq!(
                remainder.to_decimal(),
                (i128::from(a) % i128::from(d)).to_string(),
                "signed remainder: {a} % {d} must take the dividend's sign"
            );
            assert_eq!(
                quotient.mul(&divisor).add(&remainder),
                dividend,
                "divDigits_correct: q*d + r must reconstruct {a} over {d}"
            );
            assert!(
                remainder.magnitude() < divisor.magnitude(),
                "divDigits_remainder_lt: |{a} % {d}| must stay below |{d}|"
            );
            if !remainder.is_zero() {
                assert_eq!(
                    remainder.sign(),
                    dividend.sign(),
                    "the remainder must carry the dividend's sign for {a} over {d}"
                );
            }
        }
    }
}

/// Ordering is total, antisymmetric and transitive over the exhaustive domains, and agrees with the
/// `i128` order (so the sign-first comparison is the mathematical one, not a rendering order).
#[test]
fn proof_int_ordering_is_total_exhaustively() {
    for &a in &pair_domain() {
        for &b in &pair_domain() {
            let left = int(a);
            let right = int(b);

            assert_eq!(
                left.cmp(&right),
                a.cmp(&b),
                "ordering: {a} vs {b} must agree with the i128 order"
            );
            // Trichotomy: exactly one of <, =, > holds.
            let less = left < right;
            let equal = left == right;
            let greater = left > right;
            assert_eq!(
                u8::from(less) + u8::from(equal) + u8::from(greater),
                1,
                "trichotomy: exactly one of {a} < {b}, {a} == {b}, {a} > {b} must hold"
            );
            // Antisymmetry.
            assert_eq!(
                left.cmp(&right),
                right.cmp(&left).reverse(),
                "antisymmetry: cmp({a}, {b}) must be the reverse of cmp({b}, {a})"
            );
            // The order is consistent with the difference and with negation.
            assert_eq!(
                (left.sub(&right)).sign(),
                match left.cmp(&right) {
                    core::cmp::Ordering::Less => -1,
                    core::cmp::Ordering::Equal => 0,
                    core::cmp::Ordering::Greater => 1,
                },
                "the sign of {a} - {b} must agree with the comparison"
            );
            assert_eq!(
                left.cmp(&right),
                right.negate().cmp(&left.negate()),
                "negation must preserve the order of {a} and {b}"
            );
        }
    }

    for &a in &triple_domain() {
        for &b in &triple_domain() {
            for &c in &triple_domain() {
                let (left, middle, right) = (int(a), int(b), int(c));
                if left <= middle && middle <= right {
                    assert!(
                        left <= right,
                        "transitivity: {a} <= {b} <= {c} must imply {a} <= {c}"
                    );
                }
                assert_eq!(
                    left.cmp(&middle),
                    (i128::from(a)).cmp(&i128::from(b)),
                    "triple-domain ordering: {a} vs {b} must agree with the i128 order"
                );
            }
        }
    }
}

/// Ring laws over the exhaustive triple domain: associativity of `+` and `*`, distributivity, and
/// the decimal round trip that ties the value to its published text form.
#[test]
fn proof_int_ring_laws_hold_over_exhaustive_triples() {
    for &a in &triple_domain() {
        for &b in &triple_domain() {
            for &c in &triple_domain() {
                let (left, middle, right) = (int(a), int(b), int(c));

                assert_eq!(
                    left.add(&middle).add(&right),
                    left.add(&middle.add(&right)),
                    "associativity of +: ({a} + {b}) + {c}"
                );
                assert_eq!(
                    left.mul(&middle).mul(&right),
                    left.mul(&middle.mul(&right)),
                    "associativity of *: ({a} * {b}) * {c}"
                );
                assert_eq!(
                    left.mul(&middle.add(&right)),
                    left.mul(&middle).add(&left.mul(&right)),
                    "distributivity: {a} * ({b} + {c})"
                );
                assert_eq!(
                    left.mul(&middle).sub(&left.mul(&right)),
                    left.mul(&middle.sub(&right)),
                    "distributivity over -: {a} * ({b} - {c})"
                );
            }
        }
    }

    for &a in &pair_domain() {
        let value = int(a);
        assert_eq!(
            Int::from_decimal(&value.to_decimal()),
            Some(value.clone()),
            "decimal round trip must preserve {a}"
        );
    }
}

/// The limb boundary where the magnitude becomes multi-limb: sign-carrying products and
/// differences whose decimal constants are written out independently of the code under test.
#[test]
fn proof_int_limb_boundary_arithmetic_is_exact() {
    const MAX_LIMB: &str = "18446744073709551615"; // 2^64 - 1
    const LIMB_BASE: &str = "18446744073709551616"; // 2^64
    const TWO_128: &str = "340282366920938463463374607431768211456"; // 2^128
    const TWO_128_MINUS_ONE: &str = "340282366920938463463374607431768211455"; // 2^128 - 1

    let max_limb = Int::from_decimal(MAX_LIMB).expect("2^64 - 1 parses");
    let base = Int::from_decimal(LIMB_BASE).expect("2^64 parses");
    let above_base = base.add(&int(1));

    // (2^64 - 1) * (2^64 + 1) = 2^128 - 1, with the signs and the carry both exercised.
    assert_eq!(
        max_limb.mul(&above_base).to_decimal(),
        TWO_128_MINUS_ONE,
        "multi-limb product must be exact and positive"
    );
    assert_eq!(
        max_limb.negate().mul(&above_base).to_decimal(),
        format!("-{TWO_128_MINUS_ONE}"),
        "an odd number of negative factors must keep the sign"
    );
    assert_eq!(
        base.negate().mul(&base).to_decimal(),
        format!("-{TWO_128}"),
        "2^64 squared is 2^128 and must be negative when one factor is"
    );
    assert_eq!(
        max_limb.sub(&above_base).to_decimal(),
        "-2",
        "the difference across the limb boundary must be exact and negative"
    );
    assert_eq!(
        max_limb.add(&int(1)).to_decimal(),
        LIMB_BASE,
        "the carry out of a full limb must land in the next limb"
    );
    assert!(
        max_limb < base && max_limb.negate() < max_limb,
        "ordering must hold across the limb boundary and across the sign"
    );
}
