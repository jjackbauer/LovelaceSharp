//! Newton-reciprocal division, ported limb for limb from the reference implementation
//! (`Natural.cs` `Reciprocal`, `ReciprocalBase`, `CorrectReciprocal`, `DivRemNewton`).
//!
//! The reciprocal is computed at half precision, squared with one multiply, shifted to the
//! requested scale, and then brought to the exact floor by a bounded correction loop. The property
//! that makes it trustworthy is testable without an oracle: `d*r <= 2^k < d*(r+1)`.

use std::cmp::Ordering;

/// Bits of result below which the reciprocal is seeded from the top limb.
const BASE_BITS: isize = 64;

/// `floor(2^k / d)` for a normalized `d` and `k >= bit_length(d)`.
pub(crate) fn reciprocal(d: &[u64], k: usize) -> Vec<u64> {
    let n = bit_length(d) as isize;
    let q = k as isize - n + 1;
    if q <= BASE_BITS {
        return reciprocal_base(d, n as usize, k);
    }
    let q0 = (q + 1) / 2;
    let f0 = n + q0 - 1;
    let x0 = reciprocal(d, f0 as usize);
    let dx0 = mul(d, &x0);
    let two = power_of_two(f0 as usize + 1);
    let m = sub_raw(&two, &dx0);
    let mut x = mul(&x0, &m);
    let shift = 2 * f0 - k as isize;
    if shift > 0 {
        x = shift_right_bits(&x, shift as usize);
    }
    correct(x, d, k)
}

fn reciprocal_base(d: &[u64], n: usize, k: usize) -> Vec<u64> {
    let d_top = d[d.len() - 1];
    let two64: u128 = 1u128 << 64;
    let q1 = two64 / d_top as u128;
    let r1 = two64 % d_top as u128;
    let x0 = (q1 << 64) + ((r1 << 64) / d_top as u128);
    let mut x = trim_vec(vec![x0 as u64, (x0 >> 64) as u64]);
    let shift = n as isize + 64 - k as isize;
    if shift > 0 {
        x = shift_right_bits(&x, shift as usize);
    }
    correct(x, d, k)
}

fn correct(mut x: Vec<u64>, d: &[u64], k: usize) -> Vec<u64> {
    let limit = power_of_two(k);
    let mut product = mul(&x, d);
    while cmp(&product, &limit) == Ordering::Greater {
        x = sub_one(&x);
        product = mul(&x, d);
    }
    let mut next = add_one(&x);
    let mut next_product = mul(&next, d);
    while cmp(&next_product, &limit) != Ordering::Greater {
        x = next;
        next = add_one(&x);
        next_product = mul(&next, d);
    }
    x
}

/// Exact quotient and remainder, or None when the divisor is zero.
pub(crate) fn div_rem_newton(num: &[u64], den: &[u64]) -> Option<(Vec<u64>, Vec<u64>)> {
    let num = trim(num);
    let den = trim(den);
    if den.is_empty() {
        return None;
    }
    match cmp(num, den) {
        Ordering::Less => return Some((Vec::new(), num.to_vec())),
        Ordering::Equal => return Some((vec![1], Vec::new())),
        Ordering::Greater => {}
    }
    let shift = den[den.len() - 1].leading_zeros() as usize;
    let vn = trim_vec(shift_left_bits(den, shift));
    let un = trim_vec(shift_left_bits(num, shift));
    let n_bits = bit_length(&un);
    let k = n_bits + 64;
    let r = reciprocal(&vn, k);
    let quotient = shift_right_bits(&mul(&un, &r), k);
    let product = mul(&quotient, &vn);
    let mut remainder = sub_raw(&un, &product);
    let mut quotient = quotient;
    let mut steps = 0usize;
    while cmp(&remainder, &vn) != Ordering::Less {
        if steps > 8 {
            return None;
        }
        remainder = sub_raw(&remainder, &vn);
        quotient = add_one(&quotient);
        steps += 1;
    }
    let remainder = shift_right_bits(&remainder, shift);
    Some((trim_vec(quotient), trim_vec(remainder)))
}

fn bit_length(value: &[u64]) -> usize {
    match value.last() {
        None => 0,
        Some(top) => value.len() * 64 - top.leading_zeros() as usize,
    }
}

fn power_of_two(k: usize) -> Vec<u64> {
    let mut out = vec![0u64; k / 64 + 1];
    out[k / 64] = 1u64 << (k % 64);
    out
}

fn trim(slice: &[u64]) -> &[u64] {
    let mut end = slice.len();
    while end > 0 && slice[end - 1] == 0 {
        end -= 1;
    }
    &slice[..end]
}

fn trim_vec(mut value: Vec<u64>) -> Vec<u64> {
    while value.last() == Some(&0) {
        value.pop();
    }
    value
}

fn cmp(a: &[u64], b: &[u64]) -> Ordering {
    if a.len() != b.len() {
        return a.len().cmp(&b.len());
    }
    for i in (0..a.len()).rev() {
        if a[i] != b[i] {
            return a[i].cmp(&b[i]);
        }
    }
    Ordering::Equal
}

fn mul(a: &[u64], b: &[u64]) -> Vec<u64> {
    crate::mul_limbs(a, b)
}

fn shift_left_bits(value: &[u64], shift: usize) -> Vec<u64> {
    if shift == 0 {
        return value.to_vec();
    }
    let mut out = vec![0u64; value.len() + 1];
    let word = shift / 64;
    let bits = shift % 64;
    for (i, &limb) in value.iter().enumerate() {
        out[i + word] |= limb << bits;
        if bits != 0 {
            out[i + word + 1] |= limb >> (64 - bits);
        }
    }
    out
}

fn shift_right_bits(value: &[u64], shift: usize) -> Vec<u64> {
    if shift == 0 {
        return trim_vec(value.to_vec());
    }
    let word = shift / 64;
    let bits = shift % 64;
    if word >= value.len() {
        return Vec::new();
    }
    let mut out = vec![0u64; value.len() - word];
    for i in 0..out.len() {
        let low = value[i + word] >> bits;
        let high = if bits == 0 {
            0
        } else {
            value
                .get(i + word + 1)
                .map(|x| x << (64 - bits))
                .unwrap_or(0)
        };
        out[i] = low | high;
    }
    trim_vec(out)
}

fn sub_raw(a: &[u64], b: &[u64]) -> Vec<u64> {
    let mut out = vec![0u64; a.len()];
    let mut borrow = 0u64;
    for i in 0..a.len() {
        let rhs = b.get(i).copied().unwrap_or(0);
        let (value, b1) = a[i].overflowing_sub(rhs);
        let (value, b2) = value.overflowing_sub(borrow);
        out[i] = value;
        borrow = u64::from(b1) + u64::from(b2);
    }
    trim_vec(out)
}

fn add_one(value: &[u64]) -> Vec<u64> {
    let mut out = value.to_vec();
    for limb in &mut out {
        let (next, carry) = limb.overflowing_add(1);
        *limb = next;
        if !carry {
            return out;
        }
    }
    out.push(1);
    out
}

fn sub_one(value: &[u64]) -> Vec<u64> {
    let mut out = value.to_vec();
    for limb in &mut out {
        let (next, borrow) = limb.overflowing_sub(1);
        *limb = next;
        if !borrow {
            break;
        }
    }
    trim_vec(out)
}

#[cfg(test)]
// These test names mirror the C# census methods; parity/manifest-map.json keys on the exact
// lovelace-nat::newton::tests::... path, so renaming would break gate-manifest.
#[allow(non_snake_case)]
mod tests {
    use super::*;

    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        fn normalized(&mut self, len: usize) -> Vec<u64> {
            let mut out: Vec<u64> = (0..len).map(|_| self.next()).collect();
            if let Some(last) = out.last_mut() {
                *last |= 1 << 63;
            }
            trim_vec(out)
        }
    }

    #[test]
    fn Reciprocal_GivenRandomDivisors_IsTheExactFloor() {
        let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
        for case in 0..120usize {
            let len = 1 + (rng.next() as usize % 40);
            let d = rng.normalized(len);
            let n = bit_length(&d);
            let extra = rng.next() as usize % 200;
            let k = n + extra;
            let r = reciprocal(&d, k);
            let limit = power_of_two(k);
            // d*r <= 2^k < d*(r+1)
            assert!(
                cmp(&mul(&d, &r), &limit) != Ordering::Greater,
                "case {case}: d*r exceeded 2^k"
            );
            let above = add_one(&r);
            assert!(
                cmp(&mul(&d, &above), &limit) == Ordering::Greater,
                "case {case}: d*(r+1) did not exceed 2^k"
            );
        }
    }

    #[test]
    fn Div_GivenRandomOperands_AgreesWithLongDivision() {
        let mut rng = Rng(0x243f_6a88_85a3_08d3);
        for case in 0..80usize {
            let den_len = 1 + (rng.next() as usize % 60);
            let extra = rng.next() as usize % 70;
            let den = rng.normalized(den_len);
            let mut num = rng.normalized(den_len + extra);
            if cmp(&num, &den) != Ordering::Greater {
                num = add_one(&den);
            }
            let (q, r) = div_rem_newton(&num, &den).expect("nonzero divisor");
            let left = crate::Nat::from_limbs(num.clone());
            let right = crate::Nat::from_limbs(den.clone());
            let (kq, kr) = left.div_rem_knuth(&right).expect("nonzero divisor");
            assert_eq!(
                crate::Nat::from_limbs(q),
                kq,
                "case {case}: quotient differs"
            );
            assert_eq!(
                crate::Nat::from_limbs(r),
                kr,
                "case {case}: remainder differs"
            );
        }
    }
}
