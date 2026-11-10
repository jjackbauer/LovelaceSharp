//! Burnikel-Ziegler divide-and-conquer division.
//!
//! Ported from the reference implementation in OpenJDK's `MutableBigInteger`
//! (`divideAndConquer`, `divide2n1n`, `divide3n2n`), with 64-bit limbs in place of 32-bit ints.
//! The public entry point is always followed by an exact verification in `Nat::div_rem`, so a
//! defect here can cost time but never correctness.
//!
//! Limbs are little-endian base 2^64, canonical (no most-significant zero limb).

use std::cmp::Ordering;

/// Divisor length below which the recursion stops and long division runs.
pub(crate) const BASE_CASE_LIMBS: usize = 64;

/// Exact quotient and remainder, or None when the divisor is zero.
pub(crate) fn div_rem_bz(num: &[u64], den: &[u64]) -> Option<(Vec<u64>, Vec<u64>)> {
    let num = trim(num);
    let den = trim(den);
    if den.is_empty() {
        return None;
    }
    match cmp_limbs(num, den) {
        Ordering::Less => return Some((Vec::new(), num.to_vec())),
        Ordering::Equal => return Some((vec![1], Vec::new())),
        Ordering::Greater => {}
    }
    let sigma = den[den.len() - 1].leading_zeros() as usize;
    let b = trim_vec(shift_left_bits(den, sigma));
    let a = trim_vec(shift_left_bits(num, sigma));
    let n = b.len();
    let (q, r) = divide_and_conquer(&a, &b, n);
    let r = shift_right_bits(&r, sigma);
    Some((trim_vec(q), trim_vec(r)))
}

/// Algorithm 3: divides a number of at most t blocks by an n-limb divisor.
fn divide_and_conquer(a: &[u64], b: &[u64], n: usize) -> (Vec<u64>, Vec<u64>) {
    let bits = bit_length(a);
    let t = ((bits + n * 64) / (n * 64)).max(2);
    let block = |index: usize| -> Vec<u64> {
        let start = index * n;
        if start >= a.len() {
            return Vec::new();
        }
        let end = ((index + 1) * n).min(a.len());
        trim_vec(a[start..end].to_vec())
    };
    let a1 = block(t - 1);
    let mut z = join_blocks(&block(t - 2), &a1, n);
    let mut quotient: Vec<u64> = Vec::new();
    for i in (1..=t - 2).rev() {
        let (qi, ri) = divide_2n_1n(&z, b);
        z = join_blocks(&block(i - 1), &ri, n);
        quotient = add_shifted(&quotient, &qi, i * n);
    }
    let (qi, ri) = divide_2n_1n(&z, b);
    quotient = add_shifted(&quotient, &qi, 0);
    (trim_vec(quotient), trim_vec(ri))
}

/// Algorithm 1: divides a 2n-limb number by an n-limb number.
fn divide_2n_1n(a: &[u64], b: &[u64]) -> (Vec<u64>, Vec<u64>) {
    let n = b.len();
    if n % 2 != 0 || n < BASE_CASE_LIMBS {
        return knuth(a, b);
    }
    let half = n / 2;
    let a_upper = shift_right_limbs(a, half);
    let a4 = low_limbs(a, half);
    let (q1, r1) = divide_3n_2n(&a_upper, b);
    // [r1, a4]: r1 is the high block, a4 the low one.
    let combined = join_blocks(&a4, &r1, half);
    let (q2, r2) = divide_3n_2n(&combined, b);
    let quotient = add_shifted(&q2, &q1, half);
    (trim_vec(quotient), trim_vec(r2))
}

/// Algorithm 2: divides a 3n-limb number by a 2n-limb number.
fn divide_3n_2n(a: &[u64], b: &[u64]) -> (Vec<u64>, Vec<u64>) {
    let n = b.len() / 2;
    let a12 = shift_right_limbs(a, n);
    let a3 = low_limbs(a, n);
    let b1 = shift_right_limbs(b, n);
    let b2 = low_limbs(b, n);

    let b_shifted = shift_left_limbs(b, n);
    let (quotient, r, d) = if cmp_limbs(a, &b_shifted) == Ordering::Less {
        let (q, r) = divide_2n_1n(&a12, &b1);
        let d = mul_limbs(&q, &b2);
        (q, r, d)
    } else {
        // quotient = beta^n - 1; r = a12 + b1 - b1*beta^n; d = b2*beta^n - b2
        let q = vec![u64::MAX; n];
        let big_b1 = shift_left_limbs(&b1, n);
        let r = add_limbs(&sub_limbs(&a12, &big_b1).unwrap_or_default(), &b1);
        let d = sub_limbs(&shift_left_limbs(&b2, n), &b2).unwrap_or_default();
        (q, r, d)
    };
    let mut quotient = quotient;
    // r*beta^n + a3.
    let mut r = join_blocks(&a3, &r, n);
    while cmp_limbs(&r, &d) == Ordering::Less {
        r = add_limbs(&r, b);
        quotient = sub_one(&quotient);
    }
    let r = sub_limbs(&r, &d).unwrap_or_default();
    (trim_vec(quotient), trim_vec(r))
}

fn knuth(a: &[u64], b: &[u64]) -> (Vec<u64>, Vec<u64>) {
    let left = crate::Nat::from_limbs(a.to_vec());
    let right = crate::Nat::from_limbs(b.to_vec());
    match left.div_rem_knuth(&right) {
        Some((q, r)) => (q.limbs().to_vec(), r.limbs().to_vec()),
        None => (Vec::new(), Vec::new()),
    }
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

fn cmp_limbs(a: &[u64], b: &[u64]) -> Ordering {
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

fn bit_length(value: &[u64]) -> usize {
    match value.last() {
        None => 0,
        Some(top) => value.len() * 64 - top.leading_zeros() as usize,
    }
}

fn low_limbs(a: &[u64], k: usize) -> Vec<u64> {
    let end = k.min(a.len());
    trim_vec(a[..end].to_vec())
}

fn shift_right_limbs(a: &[u64], k: usize) -> Vec<u64> {
    if k >= a.len() {
        return Vec::new();
    }
    trim_vec(a[k..].to_vec())
}

fn shift_left_limbs(a: &[u64], k: usize) -> Vec<u64> {
    if a.is_empty() {
        return Vec::new();
    }
    let mut out = vec![0u64; k];
    out.extend_from_slice(a);
    out
}

/// value = low * beta^k + high (high < beta^k).
fn join_blocks(low: &[u64], high: &[u64], k: usize) -> Vec<u64> {
    let mut out = low.to_vec();
    out.resize(k.max(out.len()), 0);
    out.extend_from_slice(high);
    trim_vec(out)
}

fn add_limbs(a: &[u64], b: &[u64]) -> Vec<u64> {
    let len = a.len().max(b.len());
    let mut out = vec![0u64; len + 1];
    let mut carry = 0u64;
    for i in 0..len {
        let x = a.get(i).copied().unwrap_or(0) as u128;
        let y = b.get(i).copied().unwrap_or(0) as u128;
        let total = x + y + carry as u128;
        out[i] = total as u64;
        carry = (total >> 64) as u64;
    }
    out[len] = carry;
    trim_vec(out)
}

fn sub_limbs(a: &[u64], b: &[u64]) -> Option<Vec<u64>> {
    if cmp_limbs(a, b) == Ordering::Less {
        return None;
    }
    let mut out = vec![0u64; a.len()];
    let mut borrow = 0u64;
    for i in 0..a.len() {
        let rhs = b.get(i).copied().unwrap_or(0);
        let (value, b1) = a[i].overflowing_sub(rhs);
        let (value, b2) = value.overflowing_sub(borrow);
        out[i] = value;
        borrow = u64::from(b1) + u64::from(b2);
    }
    Some(trim_vec(out))
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

/// accumulator += value * beta^offset
fn add_shifted(accumulator: &[u64], value: &[u64], offset: usize) -> Vec<u64> {
    if value.is_empty() {
        return accumulator.to_vec();
    }
    let mut shifted = vec![0u64; offset];
    shifted.extend_from_slice(value);
    add_limbs(accumulator, &trim_vec(shifted))
}

fn mul_limbs(a: &[u64], b: &[u64]) -> Vec<u64> {
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
            value.get(i + word + 1).map(|x| x << (64 - bits)).unwrap_or(0)
        };
        out[i] = low | high;
    }
    trim_vec(out)
}

#[cfg(test)]
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

        fn limbs(&mut self, len: usize) -> Vec<u64> {
            let mut out: Vec<u64> = (0..len).map(|_| self.next()).collect();
            if let Some(last) = out.last_mut() {
                *last |= 1 << 63;
            }
            out
        }
    }

    fn check(num: &[u64], den: &[u64], label: &str) {
        let (q, r) = div_rem_bz(num, den).expect("nonzero divisor");
        let left = crate::Nat::from_limbs(num.to_vec());
        let right = crate::Nat::from_limbs(den.to_vec());
        let (kq, kr) = left.div_rem_knuth(&right).expect("nonzero divisor");
        assert_eq!(
            crate::Nat::from_limbs(q.clone()),
            kq,
            "{label}: quotient differs from long division"
        );
        assert_eq!(
            crate::Nat::from_limbs(r.clone()),
            kr,
            "{label}: remainder differs from long division"
        );
        assert!(crate::Nat::from_limbs(r.clone()) < crate::Nat::from_limbs(den.to_vec()), "{label}: remainder not below the divisor");
    }

    #[test]
    fn Div_GivenRandomSizes_AgreesWithLongDivision() {
        let mut rng = Rng(0x243f_6a88_85a3_08d3);
        for case in 0..200usize {
            let den_len = 1 + (rng.next() as usize % 260);
            let num_len = den_len + (rng.next() as usize % 300);
            let den = rng.limbs(den_len);
            let mut num = rng.limbs(num_len);
            if cmp_limbs(&num, &den) != Ordering::Greater {
                num = add_limbs(&den, &[1]);
            }
            check(&num, &den, &format!("case {case} {num_len}/{den_len}"));
        }
    }

    #[test]
    fn Div_GivenExactMultiple_ReturnsZeroRemainder() {
        let mut rng = Rng(0x1319_8a2e_0370_7344);
        for case in 0..40usize {
            let den = rng.limbs(40 + case * 3);
            let factor = rng.limbs(3);
            let num = mul_limbs(&den, &factor);
            check(&num, &den, &format!("exact {case}"));
            let (_, r) = div_rem_bz(&num, &den).expect("nonzero");
            assert!(r.is_empty(), "exact multiple should leave no remainder");
        }
    }

    #[test]
    fn Div_GivenPowerOfTwoAndAllOnes_AgreesWithLongDivision() {
        let mut den = vec![0u64; 200];
        den[199] = 1 << 63;
        let num = vec![u64::MAX; 500];
        check(&num, &den, "power of two divisor");
        let den_ones = vec![u64::MAX; 150];
        check(&num, &den_ones, "all-ones divisor");
    }
}
