//! Exact multiplication by number-theoretic transform.
//!
//! The operands are cut into 32-bit digits, convolved modulo three NTT-friendly primes, and
//! recombined by the Chinese remainder theorem. The whole path is integer arithmetic: no
//! floating point value is formed anywhere, so the product is exact for every input.

use std::sync::OnceLock;

/// First transform prime; 3 is a primitive root and p1 - 1 = 119 * 2^23.
const P1: u64 = 998_244_353;
/// Second transform prime; 3 is a primitive root and p2 - 1 = 479 * 2^21.
const P2: u64 = 1_004_535_809;
/// Third transform prime; 3 is a primitive root and p3 - 1 = 7 * 2^26.
const P3: u64 = 469_762_049;
/// A primitive root modulo all three primes.
const ROOT: u64 = 3;

/// Bits per digit: each 64-bit limb splits into a low and a high 32-bit digit.
const DIGIT_BITS: u32 = 32;
/// Mask isolating a single digit.
const DIGIT_MASK: u64 = (1 << DIGIT_BITS) - 1;
/// Largest possible digit, as the coefficient-bound arithmetic needs it.
const MAX_DIGIT: u128 = DIGIT_MASK as u128;

/// The largest power of two dividing every p - 1; p2 - 1 = 479 * 2^21 is the tightest.
const MAX_TRANSFORM_LOG2: u32 = 21;

/// p1 * p2 * p3: every convolution coefficient must stay below this to reconstruct exactly.
const CRT_RANGE: u128 = (P1 as u128) * (P2 as u128) * (P3 as u128);

/// The most 32-bit digit products that may overlap in one coefficient.
///
/// A coefficient sums at most min(digit_len_a, digit_len_b) products of two digits, so a
/// convolution that respects this bound reconstructs unambiguously from the three residues.
const MAX_TERMS: u128 = CRT_RANGE / (MAX_DIGIT * MAX_DIGIT);

/// Largest limb count an operand may have and still go through a single transform.
///
/// Two operands of this length need a 2^18-point transform, which all three primes support.
pub(crate) const MAX_NTT_LIMBS: usize = 1 << 16;

/// Multiplies two little-endian base-2^64 limb vectors, exactly.
///
/// Both inputs must have no trailing zero limb; the empty slice is zero. The result is the
/// product in the same form, with trailing zero limbs removed.
pub(crate) fn mul_ntt(a: &[u64], b: &[u64]) -> Vec<u64> {
    mul_ntt_with_limit(a, b, MAX_TRANSFORM_LOG2)
}

/// Multiplies exactly, using no transform larger than 2^limit_log2.
///
/// When the operands would need a larger transform, the longer one is split at a limb
/// boundary and the two partial products are added with a limb shift. That keeps the function
/// exact for every input while the fast path stays one transform per prime.
fn mul_ntt_with_limit(a: &[u64], b: &[u64], limit_log2: u32) -> Vec<u64> {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    if fits_transform(a.len(), b.len(), limit_log2) {
        let digits_a = to_digits(a);
        let digits_b = to_digits(b);
        if digits_a.is_empty() || digits_b.is_empty() {
            return Vec::new();
        }
        return mul_digits(&digits_a, &digits_b);
    }
    if a.len() == 1 && b.len() == 1 {
        // Only reachable with a deliberately tiny limit; a one-limb-by-one-limb product
        // cannot be split any further.
        let product = (a[0] as u128) * (b[0] as u128);
        let mut out = vec![product as u64];
        if product >> 64 != 0 {
            out.push((product >> 64) as u64);
        }
        return out;
    }
    let (low, high, other, offset) = if a.len() >= b.len() {
        let half = a.len() / 2;
        (&a[..half], &a[half..], b, half)
    } else {
        let half = b.len() / 2;
        (&b[..half], &b[half..], a, half)
    };
    let mut product = mul_ntt_with_limit(low, other, limit_log2);
    let high_part = mul_ntt_with_limit(high, other, limit_log2);
    add_shifted(&mut product, &high_part, offset);
    while product.last() == Some(&0) {
        product.pop();
    }
    product
}

/// True when one transform of at most 2^limit_log2 points can produce an exact product.
fn fits_transform(len_a: usize, len_b: usize, limit_log2: u32) -> bool {
    if len_a > MAX_NTT_LIMBS || len_b > MAX_NTT_LIMBS {
        return false;
    }
    let convolution_len = match len_a.checked_add(len_b) {
        Some(sum) => sum - 1,
        None => return false,
    };
    // Every limb contributes two digits, so the digit convolution is 2*(len_a+len_b)-1 long.
    let convolution_len = match convolution_len.checked_mul(2) {
        Some(len) => len + 1,
        None => return false,
    };
    let transform_len = match convolution_len.checked_next_power_of_two() {
        Some(len) => len,
        None => return false,
    };
    if transform_len > (1usize << limit_log2) {
        return false;
    }
    let terms = (2 * len_a.min(len_b)) as u128;
    terms * MAX_DIGIT * MAX_DIGIT < CRT_RANGE
}

/// Adds addend into accumulator starting at limb offset.
fn add_shifted(accumulator: &mut Vec<u64>, addend: &[u64], offset: usize) {
    if addend.is_empty() {
        return;
    }
    if accumulator.len() < offset + addend.len() {
        accumulator.resize(offset + addend.len(), 0);
    }
    let mut carry: u128 = 0;
    for (i, &limb) in addend.iter().enumerate() {
        let total = accumulator[offset + i] as u128 + limb as u128 + carry;
        accumulator[offset + i] = total as u64;
        carry = total >> 64;
    }
    let mut index = offset + addend.len();
    while carry != 0 {
        if index == accumulator.len() {
            accumulator.push(0);
        }
        let total = accumulator[index] as u128 + carry;
        accumulator[index] = total as u64;
        carry = total >> 64;
        index += 1;
    }
}

/// Splits limbs into little-endian 32-bit digits, dropping high zero digits.
fn to_digits(limbs: &[u64]) -> Vec<u32> {
    let mut digits = Vec::with_capacity(limbs.len() * 2);
    for &limb in limbs {
        digits.push((limb & DIGIT_MASK) as u32);
        digits.push((limb >> DIGIT_BITS) as u32);
    }
    while digits.last() == Some(&0) {
        digits.pop();
    }
    digits
}

/// Convolves two digit vectors, reconstructing the coefficients from three prime residues.
fn mul_digits(a: &[u32], b: &[u32]) -> Vec<u64> {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let convolution_len = a.len() + b.len() - 1;
    let transform_len = convolution_len.next_power_of_two();
    // The three moduli are independent, so their transforms run concurrently when the
    // transform is large enough to pay for the threads.
    let parallel = transform_len >= PARALLEL_NTT_LENGTH && crate::thread_budget() > 1;
    let (residues_p1, residues_p2, residues_p3) = if parallel {
        std::thread::scope(|scope| {
            let first = scope.spawn(|| convolve_mod(a, b, transform_len, P1));
            let second = scope.spawn(|| convolve_mod(a, b, transform_len, P2));
            let third = convolve_mod(a, b, transform_len, P3);
            (
                first.join().expect("worker"),
                second.join().expect("worker"),
                third,
            )
        })
    } else {
        (
            convolve_mod(a, b, transform_len, P1),
            convolve_mod(a, b, transform_len, P2),
            convolve_mod(a, b, transform_len, P3),
        )
    };
    crt_pack(&residues_p1, &residues_p2, &residues_p3, convolution_len)
}

/// Transform length from which the three moduli are processed on separate workers.
const PARALLEL_NTT_LENGTH: usize = 8192;

/// Transform length from which the two forward transforms run on separate workers.
const PARALLEL_PAIR_LENGTH: usize = 8192;

/// Butterfly block size from which a stage is split across workers.
const PARALLEL_BUTTERFLY_MIN: usize = 1 << 20;

/// One prime's contribution: forward transform, pointwise product, inverse transform.
fn convolve_mod(a: &[u32], b: &[u32], transform_len: usize, prime: u64) -> Vec<u64> {
    let mut left = vec![0u64; transform_len];
    let mut right = vec![0u64; transform_len];
    for (slot, &digit) in left.iter_mut().zip(a.iter()) {
        *slot = (digit as u64) % prime;
    }
    for (slot, &digit) in right.iter_mut().zip(b.iter()) {
        *slot = (digit as u64) % prime;
    }
    // The two forward transforms are independent; run them on separate workers when the
    // transform is big enough for the hand-off to pay for itself.
    if transform_len >= PARALLEL_PAIR_LENGTH && crate::thread_budget() > 1 {
        std::thread::scope(|scope| {
            let handle = scope.spawn(|| ntt(&mut left, prime, false));
            ntt(&mut right, prime, false);
            handle.join().expect("worker");
        });
    } else {
        ntt(&mut left, prime, false);
        ntt(&mut right, prime, false);
    }
    for (x, y) in left.iter_mut().zip(right.iter()) {
        *x = mul_mod(*x, *y, prime);
    }
    ntt(&mut left, prime, true);
    left
}

/// In-place iterative radix-2 number-theoretic transform.
///
/// The length must be a power of two dividing prime - 1. The forward transform uses
/// ROOT^((prime - 1) / len) as the primitive len-th root; the inverse uses its reciprocal
/// and scales by len^-1.
fn ntt(values: &mut [u64], prime: u64, inverse: bool) {
    let n = values.len();
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            values.swap(i, j);
        }
    }
    let mut len = 2usize;
    while len <= n {
        let mut root = pow_mod(ROOT, (prime - 1) / len as u64, prime);
        if inverse {
            root = pow_mod(root, prime - 2, prime);
        }
        let half = len / 2;
        // Blocks within a stage are independent, so a wide stage can be split across workers.
        let workers = if len <= PARALLEL_BUTTERFLY_MIN {
            1
        } else {
            crate::thread_budget().min(n / len).max(1)
        };
        if workers > 1 {
            let block_count = n / len;
            let per_worker = block_count.div_ceil(workers);
            let mut chunks: Vec<&mut [u64]> = values.chunks_mut(len).collect();
            std::thread::scope(|scope| {
                for group in chunks.chunks_mut(per_worker) {
                    let root = root;
                    let prime = prime;
                    scope.spawn(move || {
                        for block in group.iter_mut() {
                            let mut twist = 1u64;
                            for index in 0..half {
                                let even = block[index];
                                let odd = mul_mod(block[index + half], twist, prime);
                                block[index] = add_mod(even, odd, prime);
                                block[index + half] = sub_mod(even, odd, prime);
                                twist = mul_mod(twist, root, prime);
                            }
                        }
                    });
                }
            });
        }
        let mut block = 0usize;
        while workers == 1 && block < n {
            let mut twist = 1u64;
            for index in 0..half {
                let even = values[block + index];
                let odd = mul_mod(values[block + index + half], twist, prime);
                values[block + index] = add_mod(even, odd, prime);
                values[block + index + half] = sub_mod(even, odd, prime);
                twist = mul_mod(twist, root, prime);
            }
            block += len;
        }
        len <<= 1;
    }
    if inverse {
        let n_inverse = pow_mod(n as u64, prime - 2, prime);
        for value in values.iter_mut() {
            *value = mul_mod(*value, n_inverse, prime);
        }
    }
}

/// Precomputed Garner constants for the three-prime CRT.
struct Crt {
    /// p1^-1 mod p2.
    inverse_p1_mod_p2: u64,
    /// (p1 * p2)^-1 mod p3.
    inverse_p1p2_mod_p3: u64,
}

/// The CRT constants, computed once on first use.
fn crt_constants() -> &'static Crt {
    static CONSTANTS: OnceLock<Crt> = OnceLock::new();
    CONSTANTS.get_or_init(|| {
        let p1p2 = (P1 as u128) * (P2 as u128);
        Crt {
            inverse_p1_mod_p2: mod_inverse(P1 % P2, P2),
            inverse_p1p2_mod_p3: mod_inverse((p1p2 % P3 as u128) as u64, P3),
        }
    })
}

/// Reconstructs one coefficient from its three residues.
///
/// Garner's scheme: x = r1 + p1*t1 + p1*p2*t2, with t1 < p2 and t2 < p3, so the value is
/// below p1*p2*p3 and fits in a u128 with room to spare.
fn crt_reconstruct(r1: u64, r2: u64, r3: u64, constants: &Crt) -> u128 {
    debug_assert!(r1 < P1 && r2 < P2 && r3 < P3);
    // p1 < p2, so P2 - r1 cannot underflow.
    let t1 = mul_mod(add_mod(r2, P2 - r1, P2), constants.inverse_p1_mod_p2, P2);
    let partial = r1 as u128 + (P1 as u128) * (t1 as u128);
    let partial_mod_p3 = (partial % P3 as u128) as u64;
    let t2 = mul_mod(
        add_mod(r3, P3 - partial_mod_p3, P3),
        constants.inverse_p1p2_mod_p3,
        P3,
    );
    let value = partial + (P1 as u128) * (P2 as u128) * (t2 as u128);
    debug_assert!(value < CRT_RANGE);
    value
}

/// Reconstructs coefficients, propagates carry in base 2^32, and packs limbs.
fn crt_pack(r1: &[u64], r2: &[u64], r3: &[u64], convolution_len: usize) -> Vec<u64> {
    let constants = crt_constants();
    let mut digits: Vec<u32> = Vec::with_capacity(convolution_len + 3);
    let mut carry: u128 = 0;
    for index in 0..convolution_len {
        let coefficient = crt_reconstruct(r1[index], r2[index], r3[index], constants) + carry;
        digits.push((coefficient & DIGIT_MASK as u128) as u32);
        carry = coefficient >> DIGIT_BITS;
    }
    while carry != 0 {
        digits.push((carry & DIGIT_MASK as u128) as u32);
        carry >>= DIGIT_BITS;
    }
    let mut limbs: Vec<u64> = Vec::with_capacity(digits.len() / 2 + 1);
    let mut index = 0usize;
    while index + 1 < digits.len() {
        limbs.push(digits[index] as u64 | ((digits[index + 1] as u64) << DIGIT_BITS));
        index += 2;
    }
    if index < digits.len() {
        limbs.push(digits[index] as u64);
    }
    while limbs.last() == Some(&0) {
        limbs.pop();
    }
    limbs
}

/// base^exponent mod prime.
fn pow_mod(mut base: u64, mut exponent: u64, prime: u64) -> u64 {
    base %= prime;
    let mut result = 1u64;
    while exponent > 0 {
        if exponent & 1 == 1 {
            result = mul_mod(result, base, prime);
        }
        base = mul_mod(base, base, prime);
        exponent >>= 1;
    }
    result
}

/// value^-1 mod prime, for prime moduli.
fn mod_inverse(value: u64, prime: u64) -> u64 {
    pow_mod(value, prime - 2, prime)
}

/// (x * y) mod prime. Every prime is below 2^30, so the product fits in a u64.
fn mul_mod(x: u64, y: u64, prime: u64) -> u64 {
    debug_assert!(x < prime && y < prime);
    (x * y) % prime
}

/// (x + y) mod prime.
fn add_mod(x: u64, y: u64, prime: u64) -> u64 {
    let sum = x + y;
    if sum >= prime { sum - prime } else { sum }
}

/// (x - y) mod prime.
fn sub_mod(x: u64, y: u64, prime: u64) -> u64 {
    if x >= y { x - y } else { x + prime - y }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Nat;

    /// A reference schoolbook multiplication, written independently of the transform code.
    ///
    /// Zero limbs are skipped: the arithmetic is the same, but the sparse cases used to reach
    /// the maximum length stay affordable.
    fn schoolbook(a: &[u64], b: &[u64]) -> Vec<u64> {
        if a.is_empty() || b.is_empty() {
            return Vec::new();
        }
        let mut out = vec![0u64; a.len() + b.len()];
        for (i, &x) in a.iter().enumerate() {
            if x == 0 {
                continue;
            }
            let mut carry: u128 = 0;
            for (j, &y) in b.iter().enumerate() {
                let total = (x as u128) * (y as u128) + (out[i + j] as u128) + carry;
                out[i + j] = total as u64;
                carry = total >> 64;
            }
            let mut k = i + b.len();
            while carry != 0 {
                let total = out[k] as u128 + carry;
                out[k] = total as u64;
                carry = total >> 64;
                k += 1;
            }
        }
        while out.last() == Some(&0) {
            out.pop();
        }
        out
    }

    /// Deterministic xorshift64 generator, so every random case is reproducible.
    struct XorShift(u64);

    impl XorShift {
        fn new(seed: u64) -> XorShift {
            XorShift(if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            })
        }

        fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            x
        }

        /// Exactly len limbs with a nonzero most significant limb.
        fn limbs(&mut self, len: usize) -> Vec<u64> {
            assert!(len > 0);
            let mut value: Vec<u64> = (0..len).map(|_| self.next()).collect();
            if value[len - 1] == 0 {
                value[len - 1] = 1;
            }
            value
        }
    }

    /// Asserts that the transform product equals the reference product.
    fn check(a: &[u64], b: &[u64]) {
        let got = mul_ntt(a, b);
        let want = schoolbook(a, b);
        assert_eq!(
            got.len(),
            want.len(),
            "length mismatch for {} x {} limbs",
            a.len(),
            b.len()
        );
        for (index, (got_limb, want_limb)) in got.iter().zip(want.iter()).enumerate() {
            assert_eq!(
                got_limb,
                want_limb,
                "limb {} differs for {} x {} limbs",
                index,
                a.len(),
                b.len()
            );
        }
        assert_ne!(got.last(), Some(&0), "result kept a trailing zero limb");
    }

    /// The square of 2^(64*len) - 1, whose coefficients exercise the full CRT range.
    fn all_ones_square(len: usize) -> Vec<u64> {
        let mut out = vec![0u64; 2 * len];
        out[0] = 1;
        out[len] = u64::MAX - 1;
        for limb in out.iter_mut().skip(len + 1) {
            *limb = u64::MAX;
        }
        out
    }

    #[test]
    fn agrees_with_schoolbook_on_single_limbs() {
        let values = [
            1u64,
            2,
            3,
            0xFFFF_FFFF,
            1 << 32,
            (1 << 32) + 1,
            u64::MAX - 1,
            u64::MAX,
        ];
        for &x in &values {
            for &y in &values {
                check(&[x], &[y]);
            }
        }
        // (2^64 - 1)^2 = 2^128 - 2^65 + 1.
        assert_eq!(mul_ntt(&[u64::MAX], &[u64::MAX]), vec![1, u64::MAX - 1]);
    }

    #[test]
    fn agrees_with_schoolbook_for_one_limb_times_many() {
        let mut rng = XorShift::new(0x0123_4567_89AB_CDEF);
        for len in [1usize, 2, 3, 5, 16, 63, 64, 65, 129, 300] {
            let a = [rng.next() | 1];
            let b = rng.limbs(len);
            check(&a, &b);
            check(&b, &a);
        }
        check(&[u64::MAX], &vec![u64::MAX; 300]);
        check(&[0xFFFF_FFFF], &vec![0xFFFF_FFFF_FFFF_FFFF; 300]);
    }

    #[test]
    fn agrees_with_schoolbook_on_random_lengths_up_to_300_limbs() {
        let mut rng = XorShift::new(0x9E37_79B9_7F4A_7C15);
        for case in 0..80 {
            let len_a = 1 + (rng.next() as usize) % 300;
            let len_b = 1 + (rng.next() as usize) % 300;
            let a = rng.limbs(len_a);
            let b = rng.limbs(len_b);
            let got = mul_ntt(&a, &b);
            let want = schoolbook(&a, &b);
            assert_eq!(got, want, "case {}: {} x {} limbs", case, len_a, len_b);
            assert_ne!(got.last(), Some(&0));
        }
    }

    #[test]
    fn agrees_with_schoolbook_when_low_halves_are_zero() {
        let mut rng = XorShift::new(0x1234_5678_9ABC_DEF0);
        for _ in 0..20 {
            let len_a = 1 + (rng.next() as usize) % 40;
            let len_b = 1 + (rng.next() as usize) % 40;
            // Every limb is odd << 32: the low 32-bit digit of every limb is zero.
            let a: Vec<u64> = (0..len_a).map(|_| (rng.next() | 1) << 32).collect();
            let b: Vec<u64> = (0..len_b).map(|_| (rng.next() | 1) << 32).collect();
            check(&a, &b);
        }
        // 2^64 * (2^64 - 1) = 2^128 - 2^64: a zero low limb and a carry out of it.
        assert_eq!(mul_ntt(&[0, 1], &[u64::MAX]), vec![0, u64::MAX]);
        check(&[0, 1], &[u64::MAX]);
    }

    #[test]
    fn agrees_with_schoolbook_when_carries_sweep_many_limbs() {
        for len in [1usize, 2, 3, 7, 64, 300] {
            check(&vec![u64::MAX; len], &vec![u64::MAX; len]);
            let mut rng = XorShift::new(0xDEAD_BEEF_0000_0001 ^ len as u64);
            let mut a = vec![u64::MAX; len];
            a[0] = 0xFFFF_FFFF;
            check(&a, &rng.limbs(len));
        }
        // A big shift times a dense operand carries through the whole result.
        let mut rng = XorShift::new(0xFEED_FACE_0000_0001);
        let a = rng.limbs(300);
        let mut b = vec![0u64; 257];
        b[256] = rng.next() | 1;
        check(&a, &b);
    }

    #[test]
    fn agrees_with_schoolbook_at_the_maximum_supported_length() {
        let len = MAX_NTT_LIMBS;
        assert!(fits_transform(len, len, MAX_TRANSFORM_LOG2));
        // A sparse operand keeps the schoolbook reference affordable while still forcing a
        // 2^18-point transform and a full-length result.
        let mut a = vec![0u64; len];
        for k in 0..32 {
            a[(k * (len - 1)) / 31] = u64::MAX ^ (k as u64);
        }
        a[len - 1] = u64::MAX;
        let mut rng = XorShift::new(0x5EED_0000_1234_5678);
        let b = rng.limbs(len);
        let got = mul_ntt(&a, &b);
        let want = schoolbook(&a, &b);
        assert_eq!(got.len(), want.len());
        assert_eq!(got, want);
        // The top coefficient may or may not carry into one extra limb.
        assert!(got.len() == 2 * len - 1 || got.len() == 2 * len);
    }

    #[test]
    fn produces_a_top_limb_of_exactly_u64_max() {
        // (2^64 + 1) * (2^64 - 1) = 2^128 - 1: the carry fills the top limb completely.
        let got = mul_ntt(&[1, 1], &[u64::MAX]);
        assert_eq!(got, vec![u64::MAX, u64::MAX]);
        assert_eq!(got.last(), Some(&u64::MAX));
        check(&[1, 1], &[u64::MAX]);
        // (2^128 - 1)^2 = 2^256 - 2^129 + 1 also ends in a top limb of all ones.
        let square = mul_ntt(&[u64::MAX, u64::MAX], &[u64::MAX, u64::MAX]);
        assert_eq!(square, vec![1, 0, u64::MAX - 1, u64::MAX]);
        assert_eq!(square.last(), Some(&u64::MAX));
    }

    #[test]
    fn reconstructs_coefficients_beyond_two_primes_near_the_bound() {
        let len = MAX_NTT_LIMBS;
        let a = vec![u64::MAX; len];
        let got = mul_ntt(&a, &a);
        assert_eq!(got, all_ones_square(len));
        // A middle coefficient is the sum of 2*len maximal digit products: far above p1*p2,
        // so this case would wrap under a two-prime or 64-bit CRT.
        let max_coefficient = (2 * len) as u128 * MAX_DIGIT * MAX_DIGIT;
        assert!(max_coefficient > (P1 as u128) * (P2 as u128));
        assert!(max_coefficient < CRT_RANGE);
        assert!((2 * len) as u128 <= MAX_TERMS);
    }

    #[test]
    fn split_fallback_matches_schoolbook_when_the_limit_is_tiny() {
        let mut rng = XorShift::new(0xC0FF_EE00_1234_5678);
        for _ in 0..40 {
            let len_a = 1 + (rng.next() as usize) % 40;
            let len_b = 1 + (rng.next() as usize) % 40;
            let a = rng.limbs(len_a);
            let b = rng.limbs(len_b);
            assert_eq!(mul_ntt_with_limit(&a, &b, 5), schoolbook(&a, &b));
        }
        let a = vec![u64::MAX; 300];
        let b = vec![u64::MAX; 37];
        assert_eq!(mul_ntt_with_limit(&a, &b, 5), schoolbook(&a, &b));
    }

    /// Builds a Nat from limbs using the crate's construction API.
    fn to_nat(limbs: &[u64]) -> Nat {
        let mut value = Nat::zero();
        for &limb in limbs.iter().rev() {
            value = value
                .mul_small(1 << 32)
                .mul_small(1 << 32)
                .add(&Nat::from_u64(limb));
        }
        value
    }

    #[test]
    fn agrees_with_the_existing_nat_multiplication() {
        // Cross-checks against the crate's own multiply (schoolbook below 32 limbs,
        // Karatsuba above), not just against the reference written in this module. The
        // lengths stay below the crate's transform threshold so its own algorithm, rather
        // than this module, produces the expected value.
        let mut rng = XorShift::new(0xA5A5_5A5A_1234_9876);
        for len_a in [1usize, 2, 5, 17, 31, 32, 33, 64, 129, 200] {
            let len_b = 1 + (rng.next() as usize) % 200;
            let a = rng.limbs(len_a);
            let b = rng.limbs(len_b);
            let got = mul_ntt(&a, &b);
            let want = to_nat(&a).mul(&to_nat(&b));
            assert_eq!(
                to_nat(&got),
                want,
                "mismatch for {} x {} limbs",
                len_a,
                len_b
            );
        }
    }

    #[test]
    fn zero_and_one_multiply_canonically() {
        assert!(mul_ntt(&[], &[]).is_empty());
        assert!(mul_ntt(&[], &[7, 9]).is_empty());
        assert!(mul_ntt(&[7, 9], &[]).is_empty());
        let value = [3u64, 0, u64::MAX];
        assert_eq!(mul_ntt(&[1], &value), value.to_vec());
        assert_eq!(mul_ntt(&value, &[1]), value.to_vec());
        // Low zero limbs are value, not padding: 5 * 2^128 keeps its two empty limbs.
        assert_eq!(mul_ntt(&[0, 0, 5], &[1]), vec![0, 0, 5]);
        assert_eq!(mul_ntt(&[0, 0, 5], &[1 << 32]), vec![0, 0, 5 << 32]);
    }

    #[test]
    fn transform_primes_support_the_chosen_lengths() {
        // 3 must be a primitive root: 3^((p-1)/q) != 1 for every prime q dividing p - 1.
        for (prime, factors) in [
            (P1, &[2u64, 7, 17][..]),
            (P2, &[2u64, 479][..]),
            (P3, &[2u64, 7][..]),
        ] {
            for &factor in factors {
                assert_ne!(
                    pow_mod(ROOT, (prime - 1) / factor, prime),
                    1,
                    "3 is not a primitive root modulo {}",
                    prime
                );
            }
            assert_eq!(pow_mod(ROOT, prime - 1, prime), 1);
        }
        let len = 1usize << MAX_TRANSFORM_LOG2;
        for prime in [P1, P2, P3] {
            assert_eq!((prime - 1) % len as u64, 0);
        }
        assert!((2 * MAX_NTT_LIMBS) < (1usize << MAX_TRANSFORM_LOG2));
    }
}
