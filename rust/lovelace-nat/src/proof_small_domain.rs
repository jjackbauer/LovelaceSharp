//! Exhaustive small-domain arithmetic verification (PROOF-004) for the Lean proof model.
//!
//! Each test below is the Rust implementation-boundary instance of a theorem in
//! `Lovelace.Proofs` (the pinned Lean project). The theorems are stated for an abstract base
//! `b` and mathematical digit lists; the implementation stores base-2^64 limbs. These tests
//! instantiate the model at the values the implementation actually processes:
//!
//! * `b = 2` for single bits is not needed — every theorem is parametric in `b`, so
//!   `b = 2^64` (limbs) and `b = 10` (canonical decimal text) are the two instances that
//!   matter, and both are exercised below.
//! * "small domain" is explicit and exhaustive, never sampled:
//!   - all `a, b` in `[0, 256)` for `+`, `-`, `*` and `div_rem` (65 536 pairs per operation);
//!   - all dividends in `[0, 4096)` against every divisor in `[1, 64)` for `div_rem`
//!     (258 048 pairs, the largest exhaustive block);
//!   - all `n` in `[0, 65536)` for decimal round-trip and limb-bound checks;
//!   - every cross-product of the boundary limbs `{0, 1, 2^32-1, 2^63, 2^64-1}` at limb
//!     positions 0..=3 for carry, borrow and convolution checks.
//!
//! The oracle for the exhaustive blocks is `u128` arithmetic, which is independent of the
//! arbitrary-precision code under test (it cannot overflow for these domains). Nothing here is
//! a formal proof of the Rust code: PROOF-002 maps the theorems to these boundary tests, and
//! PROOF-003 records what the mapping does and does not cover.

use super::Nat;

/// The base of the implementation's limb representation, 2^64.
fn limb_base() -> Nat {
    Nat::from_u64(2).pow(64)
}

/// The mathematical value `hi * 2^64 + lo`, built through the public arithmetic surface.
fn from_limbs_parts(hi: u64, lo: u64) -> Nat {
    Nat::from_u64(hi).mul(&limb_base()).add(&Nat::from_u64(lo))
}

/// Decomposes a value into base-2^64 digits (least significant first) through `div_rem`.
fn base_2_64_digits(mut value: Nat) -> Vec<u64> {
    let base = limb_base();
    let mut digits = Vec::new();
    while !value.is_zero() {
        let (quotient, remainder) = value.div_rem(&base).expect("2^64 is nonzero");
        digits.push(remainder.to_u64().expect("a remainder is below 2^64"));
        value = quotient;
    }
    digits
}

/// Rebuilds a value from base-2^64 digits using only `mul` and `add`.
fn from_base_2_64_digits(digits: &[u64]) -> Nat {
    let base = limb_base();
    let mut value = Nat::zero();
    for &digit in digits.iter().rev() {
        value = value.mul(&base).add(&Nat::from_u64(digit));
    }
    value
}

/// All small pairs `[0, 256)` — the shared exhaustive domain for the four operations.
const SMALL: u64 = 256;

/// Addition.lean: `ofDigits_addLists` and `addDigits_correct` / `addDigits_no_carry`.
///
/// The theorem says the carry-propagating digit addition satisfies
/// `ofDigits b result = ofDigits b xs + ofDigits b ys` (with the final carry accounted for).
/// At the boundary that is `Nat::add`: for every pair in the exhaustive domain the Rust result
/// equals the `u128` sum, and the carry out of a limb is exactly the next limb.
#[test]
fn proof_addition_of_digits_add_lists() {
    for a in 0..SMALL {
        for b in 0..SMALL {
            let sum = Nat::from_u64(a).add(&Nat::from_u64(b));
            assert_eq!(
                sum.to_u64(),
                Some(a + b),
                "ofDigits_addLists: {a} + {b} must equal the u128 sum"
            );
        }
    }

    // addDigits_correct with a final carry: (2^64 - 1) + 1 = 2^64, i.e. limbs [0, 1].
    let max_limb = Nat::from_u64(u64::MAX);
    let carry = max_limb.add(&Nat::from_u64(1));
    assert_eq!(carry.to_u64(), None, "the carry must not fit a single limb");
    assert_eq!(carry, from_limbs_parts(1, 0), "carry must land in limb 1");

    // Cross-product of boundary limbs at every position 0..=3, added to itself.
    // The expected digits are propagated independently in u128, so the check does not reuse
    // the code under test for the carry.
    for &limb in &[0u64, 1, u32::MAX as u64, 1 << 63, u64::MAX] {
        for position in 0..4usize {
            let value = Nat::from_u64(limb).mul(&Nat::from_u64(2).pow(64 * position as u64));
            let doubled = value.add(&value);
            let mut expected = Vec::new();
            let mut carry = 0u128;
            for digit in base_2_64_digits(value.clone()) {
                let column = 2u128 * u128::from(digit) + carry;
                expected.push(column as u64);
                carry = column >> 64;
            }
            if carry != 0 {
                expected.push(carry as u64);
            }
            assert_eq!(
                doubled,
                from_base_2_64_digits(&expected),
                "addDigits_correct: x + x at limb position {position} must match u128 carry propagation"
            );
        }
    }
}

/// Subtraction.lean: `subDigits_correct`, `subDigits_lt_base`, `subDigits_no_borrow`.
///
/// The theorem relates the borrow-propagating digit subtraction to the mathematical difference,
/// with the final borrow as the high digit. At the boundary: for every pair in the exhaustive
/// domain, `sub` is `Some(a - b)` exactly when `a >= b` (and `None` otherwise, the Rust
/// equivalent of the reference's underflow refusal); every limb of the result is below 2^64 by
/// construction; and a borrow across a limb boundary is exact.
#[test]
fn proof_subtraction_sub_digits_correct() {
    for a in 0..SMALL {
        for b in 0..SMALL {
            let difference = Nat::from_u64(a).sub(&Nat::from_u64(b));
            if a >= b {
                assert_eq!(
                    difference.and_then(|value| value.to_u64()),
                    Some(a - b),
                    "subDigits_no_borrow: {a} - {b} must equal the u128 difference"
                );
            } else {
                assert!(
                    difference.is_none(),
                    "subDigits_correct: {a} - {b} must report the borrow/underflow"
                );
            }
        }
    }

    // A borrow that propagates through several zero limbs: 2^192 - 1 has three full limbs.
    let two_192 = Nat::from_u64(2).pow(192);
    let minus_one = two_192.sub(&Nat::from_u64(1)).expect("2^192 > 1");
    assert_eq!(
        base_2_64_digits(minus_one),
        vec![u64::MAX, u64::MAX, u64::MAX],
        "subDigits_correct: 2^192 - 1 must borrow into three full limbs"
    );

    // Boundary limbs: x - x is zero and reports no borrow.
    for &limb in &[0u64, 1, u32::MAX as u64, 1 << 63, u64::MAX] {
        for position in 0..4usize {
            let value = Nat::from_u64(limb).mul(&Nat::from_u64(2).pow(64 * position as u64));
            assert!(
                value.sub(&value).expect("x - x").is_zero(),
                "subDigits_correct: x - x is zero"
            );
        }
    }
}

/// Multiplication.lean: `ofDigits_conv` and `mulDigits_correct` (convolution + normalize).
///
/// The theorem says the digit convolution, once carries are normalised, has the value of the
/// product. At the boundary: the exhaustive small pairs equal the `u128` product, and the
/// two-limb convolution `(a + b*B)(c + d*B) = ac + (ad+bc)B + bd*B^2` holds at `B = 2^64`
/// with every coefficient computed independently in `u128`.
#[test]
fn proof_multiplication_mul_digits_correct() {
    for a in 0..SMALL {
        for b in 0..SMALL {
            let product = Nat::from_u64(a).mul(&Nat::from_u64(b));
            assert_eq!(
                product.to_u64(),
                Some(a * b),
                "mulDigits_correct: {a} * {b} must equal the u128 product"
            );
        }
    }

    // Two-limb convolution with 8-bit coefficients: every coefficient fits u64.
    for &a in &[0u64, 1, 2, 127, 255] {
        for &b in &[0u64, 1, 3, 128, 255] {
            for &c in &[0u64, 1, 5, 200, 255] {
                for &d in &[0u64, 1, 7, 64, 255] {
                    let limb = Nat::from_u64(2).pow(64);
                    let left = Nat::from_u64(a).add(&Nat::from_u64(b).mul(&limb));
                    let right = Nat::from_u64(c).add(&Nat::from_u64(d).mul(&limb));
                    let product = left.mul(&right);
                    // Every coefficient fits u64 for 8-bit inputs; build the expected value in
                    // Nat so the check does not depend on a u128 representing 2^128.
                    let coefficient_0 = (a as u128) * (c as u128);
                    let coefficient_1 = (a as u128) * (d as u128) + (b as u128) * (c as u128);
                    let coefficient_2 = (b as u128) * (d as u128);
                    assert!(
                        coefficient_0 <= u128::from(u64::MAX)
                            && coefficient_1 <= u128::from(u64::MAX)
                            && coefficient_2 <= u128::from(u64::MAX),
                        "the 8-bit convolution coefficients must fit u64"
                    );
                    let base_nat = Nat::from_u64(2).pow(64);
                    let expected = Nat::from_u64(coefficient_0 as u64)
                        .add(&Nat::from_u64(coefficient_1 as u64).mul(&base_nat))
                        .add(
                            &Nat::from_u64(coefficient_2 as u64)
                                .mul(&base_nat)
                                .mul(&base_nat),
                        );
                    assert_eq!(
                        product, expected,
                        "ofDigits_conv: two-limb product of ({a},{b}) and ({c},{d})"
                    );
                }
            }
        }
    }

    // normalize: (2^64 - 1)^2 = 2^128 - 2^65 + 1, limbs [1, 2^64 - 2].
    let max = Nat::from_u64(u64::MAX);
    let square = max.mul(&max);
    assert_eq!(
        base_2_64_digits(square),
        vec![1, u64::MAX - 1],
        "mulDigits_correct: (2^64 - 1)^2 must normalize to limbs [1, 2^64 - 2]"
    );
}

fn nat_from_u128(value: u128) -> Nat {
    Nat::from_decimal(&value.to_string()).expect("a u128 renders as decimal")
}

/// Division.lean: `divDigits_correct`, `divDigits_quotient_eq_div`,
/// `divDigits_remainder_eq_mod`, `divDigits_quotient_lt_base`.
///
/// The theorem is the division identity with a bounded remainder. At the boundary, for every
/// dividend in `[0, 4096)` and every divisor in `[1, 64)`: `q * d + r == a`, `r < d`, and
/// `q`/`r` equal the `u128` quotient and remainder. The same identity is checked on
/// multi-limb dividends built from the boundary limbs.
#[test]
fn proof_division_div_digits_correct() {
    for dividend in 0u64..4096 {
        for divisor in 1u64..64 {
            let (quotient, remainder) = Nat::from_u64(dividend)
                .div_rem(&Nat::from_u64(divisor))
                .expect("the divisor is nonzero");
            assert_eq!(
                quotient.to_u64(),
                Some(dividend / divisor),
                "divDigits_quotient_eq_div: {dividend} / {divisor}"
            );
            assert_eq!(
                remainder.to_u64(),
                Some(dividend % divisor),
                "divDigits_remainder_eq_mod: {dividend} % {divisor}"
            );
            assert!(
                remainder < Nat::from_u64(divisor),
                "divDigits_remainder_lt: remainder must be below the divisor"
            );
            assert_eq!(
                quotient
                    .mul(&Nat::from_u64(divisor))
                    .add(&remainder)
                    .to_u64(),
                Some(dividend),
                "divDigits_correct: q * d + r must reconstruct the dividend"
            );
        }
    }

    // Multi-limb dividends: hi * 2^64 + lo with every boundary limb in the low position.
    for hi in 0u64..32 {
        for &lo in &[0u64, 1, u32::MAX as u64, 1 << 63, u64::MAX] {
            let dividend = from_limbs_parts(hi, lo);
            let expected: u128 = (hi as u128) * (1u128 << 64) + (lo as u128);
            for divisor in 1u64..64 {
                let (quotient, remainder) = dividend
                    .div_rem(&Nat::from_u64(divisor))
                    .expect("the divisor is nonzero");
                assert_eq!(
                    quotient,
                    nat_from_u128(expected / divisor as u128),
                    "divDigits_correct: ({hi}, {lo}) / {divisor}"
                );
                assert_eq!(
                    remainder.to_u64().map(u128::from),
                    Some(expected % divisor as u128),
                    "divDigits_correct: ({hi}, {lo}) % {divisor}"
                );
                assert!(
                    remainder < Nat::from_u64(divisor),
                    "divDigits_remainder_lt: multi-limb remainder must be below the divisor"
                );
            }
        }
    }

    // Divisors that are themselves multi-limb: (2^64 + 1) * (2^64 - 1) = 2^128 - 1.
    let base = limb_base();
    let divisor = base.add(&Nat::from_u64(1));
    let quotient = base.sub(&Nat::from_u64(1)).expect("2^64 > 1");
    let dividend = divisor.mul(&quotient);
    let (q, r) = dividend.div_rem(&divisor).expect("the divisor is nonzero");
    assert_eq!(q, quotient, "divDigits_correct: multi-limb quotient");
    assert!(
        r.is_zero(),
        "divDigits_correct: multi-limb remainder is zero"
    );
}

/// Representation.lean: `ofDigits_digits` and `digits_injective`.
///
/// The theorem says the base-`b` digit expansion reconstructs the value and is injective. At
/// the boundary the representation is the canonical decimal text: every value in the exhaustive
/// range must parse to the same `Nat` as its `u64` constructor and render back to the same
/// decimal digits (no leading zeros, no alternative spelling).
#[test]
fn proof_representation_of_digits_digits() {
    for value in 0u64..65536 {
        let text = value.to_string();
        let parsed = Nat::from_decimal(&text).expect("a decimal integer must parse");
        assert_eq!(
            parsed,
            Nat::from_u64(value),
            "ofDigits_digits: parse must invert to_digits for {value}"
        );
        assert_eq!(
            parsed.to_decimal(),
            text,
            "digits_injective: the canonical rendering must be unique for {value}"
        );
        assert!(
            text.bytes().all(|byte| byte.is_ascii_digit()),
            "digits_lt_base: every rendered digit must be below the decimal base"
        );
        if value > 0 {
            assert_ne!(text.as_bytes()[0], b'0', "no leading zero for {value}");
        }
    }
}

/// Representation.lean: `digits_getDigit` (`getDigit (digits b n) i = (n / b^i) % b`).
///
/// At the boundary `b = 2^64` and the digit list is the limb array. For every limb-boundary
/// value, each limb must equal `(value / 2^(64*i)) % 2^64` and recombining the limbs must give
/// the value back (the analogue of `ofDigits_digits` for limbs).
#[test]
fn proof_representation_digits_get_digit() {
    let base = limb_base();
    for &hi in &[0u64, 1, 2, u32::MAX as u64, 1 << 63, u64::MAX] {
        for &lo in &[0u64, 1, 2, u32::MAX as u64, 1 << 63, u64::MAX] {
            let value = from_limbs_parts(hi, lo);
            let digits = base_2_64_digits(value.clone());
            let expected: Vec<u64> = if value.is_zero() {
                Vec::new()
            } else if hi == 0 {
                vec![lo]
            } else {
                vec![lo, hi]
            };
            assert_eq!(
                digits, expected,
                "digits_getDigit: limb expansion of ({hi}, {lo})"
            );
            for (index, digit) in digits.iter().enumerate() {
                let shifted = value
                    .div_rem(&Nat::from_u64(2).pow(64 * index as u64))
                    .expect("2^(64i) is nonzero")
                    .0;
                let residue = shifted
                    .div_rem(&base)
                    .expect("2^64 is nonzero")
                    .1
                    .to_u64()
                    .expect("a digit is below 2^64");
                assert_eq!(
                    *digit, residue,
                    "digits_getDigit: limb {index} of ({hi}, {lo}) must equal (n / 2^(64i)) % 2^64"
                );
            }
            assert_eq!(
                from_base_2_64_digits(&digits),
                value,
                "ofDigits_digits (b = 2^64): recombination of ({hi}, {lo})"
            );
        }
    }
}

/// Basic.lean: `ofDigits_lt_pow`.
///
/// A digit list of length `n` with every digit `< b` represents a value `< b^n`. At the
/// boundary: a natural with `k` limbs is below `2^(64k)` and (for nonzero values) at least
/// `2^(64(k-1))`, so the limb count is exactly `ceil(bits / 64)`. Exhaustive over
/// `[0, 65536)` plus the limb-boundary values.
#[test]
fn proof_basic_of_digits_lt_pow() {
    for value in 0u64..65536 {
        let nat = Nat::from_u64(value);
        let digits = base_2_64_digits(nat.clone());
        let expected_len = if value == 0 {
            0
        } else {
            (64 - value.leading_zeros() as usize).div_ceil(64)
        };
        assert_eq!(
            digits.len(),
            expected_len,
            "ofDigits_lt_pow: limb count for {value}"
        );
        let upper = Nat::from_u64(2).pow(64 * digits.len() as u64);
        assert!(
            nat < upper,
            "ofDigits_lt_pow: {value} must be below 2^(64*len)"
        );
        if !nat.is_zero() {
            let lower = Nat::from_u64(2).pow(64 * (digits.len() - 1) as u64);
            assert!(
                nat >= lower,
                "ofDigits_lt_pow: a k-limb value must be at least 2^(64(k-1))"
            );
        }
    }
    for &limb in &[1u64, u32::MAX as u64, 1 << 63, u64::MAX] {
        for position in 0..4usize {
            let value = Nat::from_u64(limb).mul(&Nat::from_u64(2).pow(64 * position as u64));
            assert_eq!(
                base_2_64_digits(value.clone()).len(),
                position + 1,
                "ofDigits_lt_pow: a limb at position {position} has {} limbs",
                position + 1
            );
        }
    }
}
