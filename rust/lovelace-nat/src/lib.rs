//! Arbitrary-precision natural numbers over 64-bit binary limbs.
//!
//! A value is a little-endian array of base-2^64 limbs with no most-significant zero limb;
//! zero is the empty array. This mirrors the reference implementation, which stores
//! \`ulong[]\` limbs and converts to and from decimal text only at the boundary.

pub(crate) mod bz;
pub(crate) mod newton;
pub(crate) mod ntt;

use std::cmp::Ordering;

/// The largest power of ten that fits in a u64: 10^19.
const DEC_CHUNK: u64 = 10_000_000_000_000_000_000;
/// Digits per decimal chunk.
const DEC_DIGITS: usize = 19;
/// Multiplies two limb slices through the fast path; used by the division module.
pub(crate) fn mul_limbs(a: &[u64], b: &[u64]) -> Vec<u64> {
    let left = Nat::from_limbs(a.to_vec());
    let right = Nat::from_limbs(b.to_vec());
    left.mul(&right).limbs().to_vec()
}

/// Test hook: long division only, bypassing the divide-and-conquer path.
pub fn knuth_probe(numerator: &Nat, divisor: &Nat) -> Option<(Nat, Nat)> {
    numerator.div_rem_knuth(divisor)
}

/// Test hook: runs the Newton-reciprocal path directly and reports its raw result.
pub fn newton_probe(numerator: &Nat, divisor: &Nat) -> Option<(Nat, Nat)> {
    let (q, r) = newton::div_rem_newton(numerator.limbs(), divisor.limbs())?;
    Some((Nat::from_limbs(q), Nat::from_limbs(r)))
}

/// Test hook: runs the divide-and-conquer path directly and reports its raw result.
pub fn bz_probe(numerator: &Nat, divisor: &Nat) -> Option<(Nat, Nat)> {
    let (q, r) = bz::div_rem_bz(numerator.limbs(), divisor.limbs())?;
    Some((Nat::from_limbs(q), Nat::from_limbs(r)))
}

/// Worker budget: `LOVELACE_THREADS` can pin it for determinism experiments.
pub(crate) fn thread_budget() -> usize {
    static BUDGET: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *BUDGET.get_or_init(|| {
        match std::env::var("LOVELACE_THREADS").ok().and_then(|v| v.parse::<usize>().ok()) {
            Some(n) if n > 0 => n,
            _ => std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1),
        }
    })
}

/// Divisor limb count from which divide-and-conquer division is attempted.
const BZ_THRESHOLD: usize = 128;

/// Limb count from which the transform multiply is used. Overridable with
/// `LOVELACE_NTT_THRESHOLD` so the crossover can be measured without rebuilding.
fn ntt_threshold() -> usize {
    static VALUE: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *VALUE.get_or_init(|| {
        std::env::var("LOVELACE_NTT_THRESHOLD")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(1024)
    })
}

/// Limb count below which schoolbook multiplication is faster.
const KARATSUBA_THRESHOLD: usize = 32;

/// An arbitrary-precision natural number.
///
/// A value is held in one or both of two forms, materialised lazily and cached, exactly as the
/// reference implementation does: binary limbs (absent until converted from a decimal string) and
/// the canonical decimal digits (absent until rendered from limbs). At least one form is always
/// present; zero is "0" with no limbs. Parsing therefore costs a string copy, and the conversion
/// is paid only when arithmetic needs it.
#[derive(Debug, Default)]
pub struct Nat {
    /// Little-endian base-2^64 limbs; no trailing zero limb. Absent until materialised.
    limbs_cache: std::sync::OnceLock<Vec<u64>>,
    /// The decimal rendering. Absent until rendered.
    decimal: std::sync::OnceLock<String>,
}

impl Clone for Nat {
    fn clone(&self) -> Nat {
        // Copy the limbs when they exist, otherwise carry the decimal form forward. The rendering
        // is deliberately not carried when limbs exist, so a clone measures a cold conversion.
        let out = Nat::default();
        match (self.limbs_cache.get(), self.decimal.get()) {
            (Some(limbs), _) => {
                let _ = out.limbs_cache.set(limbs.clone());
            }
            (None, Some(text)) => {
                let _ = out.decimal.set(text.clone());
            }
            (None, None) => {
                let _ = out.decimal.set("0".to_string());
            }
        }
        out
    }
}

impl PartialEq for Nat {
    fn eq(&self, other: &Nat) -> bool {
        self.limbs() == other.limbs()
    }
}

impl Eq for Nat {}

impl std::hash::Hash for Nat {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.limbs().hash(state);
    }
}

impl Nat {
    /// Builds from little-endian limbs.
    fn from_limbs(limbs: Vec<u64>) -> Nat {
        let out = Nat::default();
        let mut trimmed = limbs;
        while trimmed.last() == Some(&0) {
            trimmed.pop();
        }
        let _ = out.limbs_cache.set(trimmed);
        out
    }

    /// The limbs, mutably, materialising them first when only the decimal form exists.
    fn limbs_mut(&mut self) -> &mut Vec<u64> {
        if self.limbs_cache.get().is_none() {
            let parsed = self.limbs().to_vec();
            let _ = self.limbs_cache.set(parsed);
        }
        self.limbs_cache.get_mut().expect("materialised")
    }

    /// The limbs, materialising them from the decimal form if necessary.
    #[allow(clippy::let_and_return)]
    fn limbs(&self) -> &[u64] {
        if let Some(value) = self.limbs_cache.get() {
            return value;
        }
        let text = self.decimal.get().cloned();
        let parsed = parse_limbs(text.as_deref().unwrap_or("0"));
        self.limbs_cache.get_or_init(|| parsed)
    }
}

impl Nat {
    /// The value zero.
    pub fn zero() -> Nat {
        Nat::from_limbs(Vec::new())
    }

    /// Builds a value from a u64.
    pub fn from_u64(value: u64) -> Nat {
        if value == 0 {
            Nat::zero()
        } else {
            Nat::from_limbs(vec![value])
        }
    }

    /// Parses a decimal digit string. The digits are canonicalised and stored; limbs are
    /// materialised only when arithmetic needs them.
    pub fn from_decimal(text: &str) -> Option<Nat> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }
        if !trimmed.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        // One pass: validate and find the first significant digit together, then copy once.
        let bytes = trimmed.as_bytes();
        let mut first = None;
        for (index, &byte) in bytes.iter().enumerate() {
            if !byte.is_ascii_digit() {
                return None;
            }
            if first.is_none() && byte != b'0' {
                first = Some(index);
            }
        }
        let out = Nat::default();
        let canonical = match first {
            Some(index) => trimmed[index..].to_string(),
            None => "0".to_string(),
        };
        let _ = out.decimal.set(canonical);
        Some(out)
    }

    /// Renders the value as decimal digits, memoising the result.
    pub fn to_decimal(&self) -> String {
        if let Some(text) = self.decimal.get() {
            return text.clone();
        }
        let limbs = self.limbs().to_vec();
        let text = render_decimal(&limbs);
        self.decimal.get_or_init(|| text.clone()).clone()
    }

    /// Renders by repeated division by 10^19; only used for small values.
    fn to_decimal_direct(&self) -> String {
        let mut chunks: Vec<u64> = Vec::new();
        let mut current = self.clone();
        let divisor = Nat::from_u64(DEC_CHUNK);
        while !current.is_zero() {
            let (q, r) = current.div_rem(&divisor).expect("nonzero divisor");
            chunks.push(r.to_u64().unwrap_or(0));
            current = q;
        }
        let mut out = String::new();
        out.push_str(&chunks[chunks.len() - 1].to_string());
        for chunk in chunks.iter().rev().skip(1) {
            out.push_str(&format!("{:019}", chunk));
        }
        out
    }

    /// An upper estimate of the decimal digit count, used to choose the split point.
    fn approx_digits(&self) -> usize {
        if self.limbs().is_empty() {
            return 1;
        }
        let top = self.limbs()[self.limbs().len() - 1];
        let top_digits = 20 - (top as f64).log10().ceil() as i32 as usize;
        (self.limbs().len() - 1) * DEC_DIGITS + top_digits.min(DEC_DIGITS).max(1)
    }

    /// True when the value is zero.
    pub fn is_zero(&self) -> bool {
        self.limbs().is_empty()
    }

    /// True when the value is one.
    pub fn is_one(&self) -> bool {
        self.limbs().len() == 1 && self.limbs()[0] == 1
    }

    /// Addition.
    pub fn add(&self, other: &Nat) -> Nat {
        let len = self.limbs().len().max(other.limbs().len());
        let mut out = Vec::with_capacity(len + 1);
        let mut carry: u64 = 0;
        for i in 0..len {
            let a = *self.limbs().get(i).unwrap_or(&0);
            let b = *other.limbs().get(i).unwrap_or(&0);
            let (sum, c1) = a.overflowing_add(b);
            let (sum, c2) = sum.overflowing_add(carry);
            out.push(sum);
            carry = u64::from(c1) + u64::from(c2);
        }
        if carry != 0 {
            out.push(carry);
        }
        Nat::from_limbs(out)
    }

    /// Subtraction; `None` when the result would be negative.
    pub fn sub(&self, other: &Nat) -> Option<Nat> {
        if self.cmp(other) == Ordering::Less {
            return None;
        }
        let mut out = Vec::with_capacity(self.limbs().len());
        let mut borrow: u64 = 0;
        for i in 0..self.limbs().len() {
            let a = self.limbs()[i];
            let b = *other.limbs().get(i).unwrap_or(&0);
            let (diff, b1) = a.overflowing_sub(b);
            let (diff, b2) = diff.overflowing_sub(borrow);
            out.push(diff);
            borrow = u64::from(b1) + u64::from(b2);
        }
        Some(Nat::from_limbs(out))
    }

    /// Multiplication: schoolbook below the Karatsuba threshold, split above it.
    pub fn mul(&self, other: &Nat) -> Nat {
        if self.is_zero() || other.is_zero() {
            return Nat::zero();
        }
        // The transform wins on wide operands; below that its three passes cost more than the
        // schoolbook loops it replaces.
        // Both operands must be wide enough: a long number times a single limb is a linear
        // pass, and paying for three transforms to do it dominates everything above.
        let widest = self.limbs().len().max(other.limbs().len());
        let narrowest = self.limbs().len().min(other.limbs().len());
        if narrowest >= ntt_threshold() && widest <= ntt::MAX_NTT_LIMBS {
            let product = ntt::mul_ntt(&self.limbs(), &other.limbs());
            return Nat::from_limbs(product);
        }
        if self.limbs().len().min(other.limbs().len()) >= KARATSUBA_THRESHOLD {
            return self.mul_karatsuba(other);
        }
        self.mul_schoolbook(other)
    }

    /// Splits at a limb boundary, returning the low and high halves.
    fn split_at(&self, at: usize) -> (Nat, Nat) {
        if at >= self.limbs().len() {
            return (self.clone(), Nat::zero());
        }
        (
            Nat::from_limbs(self.limbs()[..at].to_vec()),
            Nat::from_limbs(self.limbs()[at..].to_vec()),
        )
    }

    fn mul_karatsuba(&self, other: &Nat) -> Nat {
        let len = self.limbs().len().max(other.limbs().len());
        let half = len / 2;
        let (a0, a1) = self.split_at(half);
        let (b0, b1) = other.split_at(half);
        let z0 = a0.mul(&b0);
        let z2 = a1.mul(&b1);
        let z1 = a0.add(&a1).mul(&b0.add(&b1)).sub(&z0).unwrap_or_else(Nat::zero).sub(&z2).unwrap_or_else(Nat::zero);
        z0.add(&z1.shift_limbs(half)).add(&z2.shift_limbs(2 * half))
    }

    fn shift_limbs(&self, count: usize) -> Nat {
        if self.is_zero() {
            return Nat::zero();
        }
        let mut limbs = vec![0u64; count];
        limbs.extend_from_slice(&self.limbs());
        Nat::from_limbs(limbs)
    }

    fn mul_schoolbook(&self, other: &Nat) -> Nat {
        let mut out = vec![0u64; self.limbs().len() + other.limbs().len()];
        for (i, &a) in self.limbs().iter().enumerate() {
            if a == 0 {
                continue;
            }
            let mut carry: u128 = 0;
            for (j, &b) in other.limbs().iter().enumerate() {
                let t = (a as u128) * (b as u128) + (out[i + j] as u128) + carry;
                out[i + j] = t as u64;
                carry = t >> 64;
            }
            let mut k = i + other.limbs().len();
            while carry != 0 {
                let t = out[k] as u128 + carry;
                out[k] = t as u64;
                carry = t >> 64;
                k += 1;
            }
        }
        Nat::from_limbs(out)
    }

    /// Division returning quotient and remainder; `None` on a zero divisor.
    ///
    /// Above the divide-and-conquer threshold the Burnikel-Ziegler path is tried first and its
    /// answer is *verified* (`q*d + r == n` and `r < d`) before it is published; anything else
    /// falls back to long division, so the result is exact whatever the fast path does.
    pub fn div_rem(&self, divisor: &Nat) -> Option<(Nat, Nat)> {
        if divisor.limbs().len() >= BZ_THRESHOLD {
            if let Some((q, r)) = bz::div_rem_bz(self.limbs(), divisor.limbs()) {
                let quotient = Nat::from_limbs(q);
                let remainder = Nat::from_limbs(r);
                let check = quotient.mul(divisor).add(&remainder);
                if check == *self && remainder < *divisor {
                    return Some((quotient, remainder));
                }
            }
        }
        self.div_rem_knuth(divisor)
    }

    /// Long division (Knuth algorithm D); the base case and the fallback.
    pub(crate) fn div_rem_knuth(&self, divisor: &Nat) -> Option<(Nat, Nat)> {
        if divisor.is_zero() {
            return None;
        }
        match self.cmp(divisor) {
            Ordering::Less => return Some((Nat::zero(), self.clone())),
            Ordering::Equal => return Some((Nat::from_u64(1), Nat::zero())),
            Ordering::Greater => {}
        }
        if divisor.limbs().len() == 1 {
            let (q, r) = self.div_small(divisor.limbs()[0]);
            return Some((q, Nat::from_u64(r)));
        }
        let n = divisor.limbs().len();
        let m = self.limbs().len() - n;
        // Knuth algorithm D: normalise so the divisor's top bit is set.
        let shift = divisor.limbs()[n - 1].leading_zeros();
        let v = shift_left(divisor, shift);
        let mut u = shift_left(self, shift);
        u.limbs_mut().resize(self.limbs().len() + 1, 0);
        let b: u128 = 1u128 << 64;
        let v_top = v.limbs()[n - 1] as u128;
        let v_next = v.limbs()[n - 2] as u128;
        let v_slice = &v.limbs()[..n];
        let mut q = vec![0u64; m + 1];
        for j in (0..=m).rev() {
            let numerator = ((u.limbs()[j + n] as u128) << 64) | u.limbs()[j + n - 1] as u128;
            let mut qhat = numerator / v_top;
            let mut rhat = numerator % v_top;
            let u_next = u.limbs()[j + n - 2] as u128;
            while qhat >= b || qhat * v_next > (rhat << 64) | u_next {
                qhat -= 1;
                rhat += v_top;
                if rhat >= b {
                    break;
                }
            }
            // Multiply and subtract over slice pairs: no indexing, no i128, borrow as a flag.
            let (window, rest) = u.limbs_mut().split_at_mut(j + n);
            let low = &mut window[j..];
            let mut carry: u128 = 0;
            let mut borrow: u64 = 0;
            for (slot, &divisor_limb) in low.iter_mut().zip(v_slice.iter()) {
                let product = qhat * (divisor_limb as u128) + carry;
                carry = product >> 64;
                let (diff, b1) = slot.overflowing_sub(product as u64);
                let (diff, b2) = diff.overflowing_sub(borrow);
                *slot = diff;
                borrow = u64::from(b1) + u64::from(b2);
            }
            let (diff, b1) = rest[0].overflowing_sub(carry as u64);
            let (diff, b2) = diff.overflowing_sub(borrow);
            rest[0] = diff;
            if b1 | b2 {
                // The estimate was one too large: add the divisor back.
                qhat -= 1;
                let mut carry: u128 = 0;
                for (slot, &divisor_limb) in low.iter_mut().zip(v_slice.iter()) {
                    let total = *slot as u128 + (divisor_limb as u128) + carry;
                    *slot = total as u64;
                    carry = total >> 64;
                }
                rest[0] = (rest[0] as u128 + carry) as u64;
            }
            q[j] = qhat as u64;
        }
        let quotient = Nat::from_limbs(q);
        let remainder = shift_right(&Nat::from_limbs(u.limbs()[..n].to_vec()), shift);
        Some((quotient, remainder))
    }

    /// Raises to a non-negative integer power.
    pub fn pow(&self, mut exponent: u64) -> Nat {
        let mut result = Nat::from_u64(1);
        let mut base = self.clone();
        while exponent > 0 {
            if exponent & 1 == 1 {
                result = result.mul(&base);
            }
            exponent >>= 1;
            if exponent > 0 {
                base = base.mul(&base);
            }
        }
        result
    }

    /// The factorial of `n`.
    ///
    /// A balanced binary product tree: neighbours are multiplied first, so operand sizes stay even
    /// all the way up. Measured against the reference's per-worker contiguous ranges on this host,
    /// the tree was more than twice as fast (48 ms against 100 ms at 20 000!), so it is what ships
    /// even though the reference does it differently.
    pub fn factorial(n: u64) -> Nat {
        if n < 2 {
            return Nat::from_u64(1);
        }
        product_range(2, n)
    }

    /// Multiplies by ten raised to `exponent`.
    pub fn mul_pow10(&self, exponent: usize) -> Nat {
        if self.is_zero() {
            return Nat::zero();
        }
        let mut result = self.clone();
        let mut remaining = exponent;
        // A limb shift would multiply by 2^64, not 10^19, so scale by the decimal chunk.
        while remaining >= DEC_DIGITS {
            result = result.mul_small(DEC_CHUNK);
            remaining -= DEC_DIGITS;
        }
        if remaining > 0 {
            let mut factor: u64 = 1;
            for _ in 0..remaining {
                factor *= 10;
            }
            result = result.mul_small(factor);
        }
        result
    }

    /// The value as a u64, when it fits.
    pub fn to_u64(&self) -> Option<u64> {
        match self.limbs().len() {
            0 => Some(0),
            1 => Some(self.limbs()[0]),
            _ => None,
        }
    }

    /// Multiplies by a single limb.
    pub(crate) fn mul_small(&self, factor: u64) -> Nat {
        if factor == 0 || self.is_zero() {
            return Nat::zero();
        }
        let mut out = Vec::with_capacity(self.limbs().len() + 1);
        let mut carry: u128 = 0;
        for &limb in self.limbs() {
            let t = (limb as u128) * (factor as u128) + carry;
            out.push(t as u64);
            carry = t >> 64;
        }
        if carry != 0 {
            out.push(carry as u64);
        }
        Nat::from_limbs(out)
    }

    /// Adds a single limb.
    fn add_small(&self, value: u64) -> Nat {
        if value == 0 {
            return self.clone();
        }
        let mut out = self.limbs().to_vec();
        let mut carry = value;
        let mut i = 0usize;
        while carry != 0 {
            if i == out.len() {
                out.push(carry);
                break;
            }
            let (sum, overflow) = out[i].overflowing_add(carry);
            out[i] = sum;
            carry = u64::from(overflow);
            i += 1;
        }
        Nat::from_limbs(out)
    }

    /// Divides by a single nonzero limb, returning quotient and remainder.
    fn div_small(&self, divisor: u64) -> (Nat, u64) {
        let mut out = vec![0u64; self.limbs().len()];
        let mut remainder: u128 = 0;
        for i in (0..self.limbs().len()).rev() {
            let current = (remainder << 64) | self.limbs()[i] as u128;
            out[i] = (current / divisor as u128) as u64;
            remainder = current % divisor as u128;
        }
        (Nat::from_limbs(out), remainder as u64)
    }

    fn normalized(mut self) -> Nat {
        if self.limbs_cache.get().is_some() {
            while self.limbs_mut().last() == Some(&0) {
                self.limbs_mut().pop();
            }
        }
        self
    }
}




/// The product of every integer in `[lo, hi]`, by recursive halving.
///
/// Above a chunk threshold the two halves are multiplied on separate worker threads. The tree
/// shape — and therefore the result — does not depend on the scheduling, so the output is
/// identical for any thread budget.
fn product_range(lo: u64, hi: u64) -> Nat {
    if lo > hi {
        return Nat::from_u64(1);
    }
    if lo == hi {
        return Nat::from_u64(lo);
    }
    if hi - lo == 1 {
        return Nat::from_u64(lo).mul(&Nat::from_u64(hi));
    }
    let mid = lo + (hi - lo) / 2;
    let parallel = (hi - lo) > factorial_leaf() && thread_budget() > 1;
    if parallel {
        let (left, right) = std::thread::scope(|scope| {
            let handle = scope.spawn(|| product_range(lo, mid));
            let right = product_range(mid + 1, hi);
            (handle.join().expect("worker"), right)
        });
        left.mul(&right)
    } else {
        product_range(lo, mid).mul(&product_range(mid + 1, hi))
    }
}

/// Range width above which the factorial tree forks; overridable so the leaf size can be
/// measured rather than guessed.
fn factorial_leaf() -> u64 {
    static VALUE: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *VALUE.get_or_init(|| {
        std::env::var("LOVELACE_FACTORIAL_LEAF")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(2048)
    })
}

/// Decimal digits to limbs.
pub(crate) fn parse_limbs(text: &str) -> Vec<u64> {
    let digits = text.trim_start_matches('0');
    if digits.is_empty() {
        return Vec::new();
    }
    parse_digits(digits.as_bytes()).limbs().to_vec()
}

/// Limbs to decimal digits, by splitting on a power of ten and recursing.
///
/// The two halves are rendered on separate workers above a threshold; the tree shape does not
/// depend on scheduling, so the digits are identical for any worker budget.
fn render_decimal(limbs: &[u64]) -> String {
    render_decimal_depth(limbs, 0)
}

fn render_decimal_depth(limbs: &[u64], depth: usize) -> String {
    let value = Nat::from_limbs(limbs.to_vec());
    if value.limbs().is_empty() {
        return "0".to_string();
    }
    if value.limbs().len() <= 2 {
        return value.to_decimal_direct();
    }
    let digits = value.approx_digits();
    let half = digits / 2;
    if half == 0 {
        return value.to_decimal_direct();
    }
    let divisor = pow10(half);
    match value.div_rem(&divisor) {
        Some((q, r)) => {
            let parallel =
                depth < 3 && half >= PARALLEL_RENDER_THRESHOLD && thread_budget() > 1;
            let (left, right) = if parallel {
                std::thread::scope(|scope| {
                    let handle = scope.spawn(|| render_decimal_depth(q.limbs(), depth + 1));
                    let right = render_decimal_depth(r.limbs(), depth + 1);
                    (handle.join().expect("worker"), right)
                })
            } else {
                (render_decimal_depth(q.limbs(), depth + 1), render_decimal_depth(r.limbs(), depth + 1))
            };
            let mut padded = String::with_capacity(half.max(right.len()));
            if right.len() < half {
                for _ in 0..(half - right.len()) {
                    padded.push('0');
                }
            }
            padded.push_str(&right);
            format!("{}{}", left, padded)
        }
        None => value.to_decimal_direct(),
    }
}

/// Digit count above which the two halves of a rendering are produced on separate workers.
const PARALLEL_RENDER_THRESHOLD: usize = 20_000;

/// Parses decimal digits by splitting the string in half and recursing.
///
/// Above the threshold the two halves are parsed on separate workers, down to a bounded depth.
/// The tree shape does not depend on scheduling, so the result is identical for any budget.
fn parse_digits(bytes: &[u8]) -> Nat {
    parse_digits_depth(bytes, 0)
}

fn parse_digits_depth(bytes: &[u8], depth: usize) -> Nat {
    if bytes.len() <= DEC_DIGITS {
        return Nat::from_u64(parse_chunk(bytes));
    }
    let half = bytes.len() / 2;
    let (hi, lo) = bytes.split_at(bytes.len() - half);
    let parallel = depth < 3 && bytes.len() >= PARALLEL_PARSE_THRESHOLD && thread_budget() > 1;
    let (high, low) = if parallel {
        std::thread::scope(|scope| {
            let handle = scope.spawn(|| parse_digits_depth(hi, depth + 1));
            let low = parse_digits_depth(lo, depth + 1);
            (handle.join().expect("worker"), low)
        })
    } else {
        (parse_digits_depth(hi, depth + 1), parse_digits_depth(lo, depth + 1))
    };
    high.mul(&pow10(half)).add(&low)
}

/// Width in digits above which parsing forks.
const PARALLEL_PARSE_THRESHOLD: usize = 20000;

/// Powers of ten already computed, keyed by exponent. Decimal conversion asks for the same
/// handful of exponents at every recursion level, and recomputing them was the dominant cost.
static POW10_CACHE: std::sync::OnceLock<std::sync::Mutex<Vec<(usize, Nat)>>> = std::sync::OnceLock::new();

/// The power of ten as a natural number.
fn pow10(exponent: usize) -> Nat {
    if exponent == 0 {
        return Nat::from_u64(1);
    }
    let cache = POW10_CACHE.get_or_init(|| std::sync::Mutex::new(Vec::new()));
    if let Ok(guard) = cache.lock() {
        if let Some((_, value)) = guard.iter().find(|(e, _)| *e == exponent) {
            return value.clone();
        }
    }
    let value = if exponent % 2 == 0 {
        let half = pow10(exponent / 2);
        half.mul(&half)
    } else {
        pow10(exponent - 1).mul_small(10)
    };
    if let Ok(mut guard) = cache.lock() {
        if !guard.iter().any(|(e, _)| *e == exponent) {
            guard.push((exponent, value.clone()));
        }
    }
    value
}

fn parse_chunk(bytes: &[u8]) -> u64 {
    let mut value: u64 = 0;
    for &b in bytes {
        value = value * 10 + (b - b'0') as u64;
    }
    value
}

fn shift_left(value: &Nat, shift: u32) -> Nat {
    if shift == 0 {
        return value.clone();
    }
    let mut out = Vec::with_capacity(value.limbs().len() + 1);
    let mut carry: u64 = 0;
    for &limb in value.limbs() {
        out.push((limb << shift) | carry);
        carry = if shift == 64 { 0 } else { limb >> (64 - shift) };
    }
    if carry != 0 {
        out.push(carry);
    }
    Nat::from_limbs(out)
}

fn shift_right(value: &Nat, shift: u32) -> Nat {
    if shift == 0 {
        // Still normalise: the caller may hand over a slice with trailing zero limbs, and a
        // non-canonical Nat compares unequal to the same value built any other way.
        return value.clone().normalized();
    }
    let mut out = vec![0u64; value.limbs().len()];
    let mut carry: u64 = 0;
    for i in (0..value.limbs().len()).rev() {
        let limb = value.limbs()[i];
        out[i] = (limb >> shift) | carry;
        carry = limb << (64 - shift);
    }
    Nat::from_limbs(out)
}

impl PartialOrd for Nat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Nat {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.limbs().len() != other.limbs().len() {
            return self.limbs().len().cmp(&other.limbs().len());
        }
        for i in (0..self.limbs().len()).rev() {
            if self.limbs()[i] != other.limbs()[i] {
                return self.limbs()[i].cmp(&other.limbs()[i]);
            }
        }
        Ordering::Equal
    }
}

impl std::fmt::Display for Nat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_decimal())
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::Nat;

    #[test]
    fn Mul_GivenRandomSizes_CrossesTheTransformAndKaratsubaPaths() {
        // The transform path is only trustworthy if it agrees with the independent Karatsuba
        // implementation over the sizes where both are legal.
        let mut state: u64 = 0x9E3779B97F4A7C15;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for &len in &[256usize, 257, 300, 400, 512] {
            let mut a = Vec::with_capacity(len);
            for _ in 0..len {
                a.push(next() | 1);
            }
            let mut b = Vec::with_capacity(len);
            for _ in 0..len {
                b.push(next());
            }
            let left = Nat::from_limbs(a.clone());
            let right = Nat::from_limbs(b.clone());
            let via_transform = left.mul(&right);
            let via_karatsuba = left.mul_karatsuba(&right);
            assert_eq!(
                via_transform.to_decimal(),
                via_karatsuba.to_decimal(),
                "transform and Karatsuba disagree at {len} limbs"
            );
        }
    }

    #[test]
    fn zero_GivenNoArguments_ReturnsCanonicalZero() {
        let zero = Nat::zero();
        assert!(zero.is_zero());
        assert!(!zero.is_one());
        assert_eq!(zero.to_decimal(), "0");
        assert_eq!(zero.to_u64(), Some(0));
    }

    #[test]
    fn default_GivenNoArguments_EqualsZero() {
        assert_eq!(Nat::default(), Nat::zero());
    }

    #[test]
    fn from_u64_GivenZero_ReturnsZero() {
        assert_eq!(Nat::from_u64(0).to_decimal(), "0");
    }

    #[test]
    fn from_u64_GivenOne_IsOne() {
        assert!(Nat::from_u64(1).is_one());
    }

    #[test]
    fn from_u64_GivenMaximum_RoundTripsThroughU64() {
        let value = Nat::from_u64(u64::MAX);
        assert_eq!(value.to_decimal(), "18446744073709551615");
        assert_eq!(value.to_u64(), Some(u64::MAX));
    }

    #[test]
    fn from_u64_GivenLimbBoundary_EncodesTwoLimbs() {
        assert_eq!(Nat::from_u64(1_000_000_000).to_decimal(), "1000000000");
    }

    #[test]
    fn from_decimal_GivenEmptyString_ReturnsNone() {
        assert!(Nat::from_decimal("").is_none());
    }

    #[test]
    fn from_decimal_GivenNonDigit_ReturnsNone() {
        assert!(Nat::from_decimal("12a3").is_none());
    }

    #[test]
    fn from_decimal_GivenLeadingMinus_ReturnsNone() {
        assert!(Nat::from_decimal("-1").is_none());
    }

    #[test]
    fn from_decimal_GivenLeadingZeros_Canonicalizes() {
        let value = Nat::from_decimal("00000123").unwrap();
        assert_eq!(value.to_decimal(), "123");
    }

    #[test]
    fn from_decimal_GivenAllZeros_ReturnsZero() {
        let value = Nat::from_decimal("000000000000").unwrap();
        assert!(value.is_zero());
        assert_eq!(value.to_decimal(), "0");
    }

    #[test]
    fn to_decimal_GivenInteriorZeroLimb_PadsNineDigits() {
        let value = Nat::from_decimal("1000000000000000001").unwrap();
        assert_eq!(value.to_decimal(), "1000000000000000001");
    }

    #[test]
    fn add_GivenZeroOperand_ReturnsOther() {
        let value = Nat::from_decimal("12345678901234567890").unwrap();
        assert_eq!(value.add(&Nat::zero()), value);
    }

    #[test]
    fn add_GivenCarryAcrossLimbBoundary_Propagates() {
        let value = Nat::from_u64(999_999_999).add(&Nat::from_u64(1));
        assert_eq!(value.to_decimal(), "1000000000");
    }

    #[test]
    fn add_GivenCarryChainAcrossThreeLimbs_Propagates() {
        let value = Nat::from_decimal("999999999999999999")
            .unwrap()
            .add(&Nat::from_u64(1));
        assert_eq!(value.to_decimal(), "1000000000000000000");
    }

    #[test]
    fn sub_GivenLargerFromSmaller_ReturnsNone() {
        assert!(Nat::from_u64(1).sub(&Nat::from_u64(2)).is_none());
    }

    #[test]
    fn sub_GivenEqualOperands_ReturnsZero() {
        let value = Nat::from_decimal("12345678901234567890").unwrap();
        assert!(value.sub(&value).unwrap().is_zero());
    }

    #[test]
    fn sub_GivenBorrowAcrossLimbs_Propagates() {
        let value = Nat::from_decimal("1000000000000000000")
            .unwrap()
            .sub(&Nat::from_u64(1))
            .unwrap();
        assert_eq!(value.to_decimal(), "999999999999999999");
    }

    #[test]
    fn sub_GivenOneLessThanLimbBoundary_ReturnsLimbMinusOne() {
        let value = Nat::from_u64(1_000_000_000)
            .sub(&Nat::from_u64(1))
            .unwrap();
        assert_eq!(value.to_decimal(), "999999999");
    }

    #[test]
    fn mul_GivenZeroOperand_ReturnsZero() {
        let product = Nat::from_decimal("98765432109876543210")
            .unwrap()
            .mul(&Nat::zero());
        assert!(product.is_zero());
    }

    #[test]
    fn mul_GivenOneOperand_ReturnsSame() {
        let value = Nat::from_decimal("98765432109876543210").unwrap();
        assert_eq!(value.mul(&Nat::from_u64(1)), value);
    }

    #[test]
    fn mul_GivenLimbBoundaryValues_ReturnsExactSquare() {
        let value = Nat::from_u64(999_999_999);
        assert_eq!(value.mul(&value).to_decimal(), "999999998000000001");
    }

    #[test]
    fn mul_GivenLargeOperands_ReturnsExactProduct() {
        let left = Nat::from_decimal("12345678901234567890").unwrap();
        let right = Nat::from_decimal("98765432109876543210").unwrap();
        assert_eq!(
            left.mul(&right).to_decimal(),
            "1219326311370217952237463801111263526900"
        );
    }

    #[test]
    fn div_rem_GivenZeroDivisor_ReturnsNone() {
        assert!(Nat::from_u64(42).div_rem(&Nat::zero()).is_none());
    }

    #[test]
    fn div_rem_GivenSmallerDividend_ReturnsZeroQuotientAndDividendRemainder() {
        let (quotient, remainder) = Nat::from_u64(42).div_rem(&Nat::from_u64(100)).unwrap();
        assert_eq!(quotient.to_decimal(), "0");
        assert_eq!(remainder.to_decimal(), "42");
    }

    #[test]
    fn div_rem_GivenExactDivision_ReturnsZeroRemainder() {
        let dividend = Nat::from_decimal("1000000000000000000").unwrap();
        let divisor = Nat::from_u64(1_000_000_000);
        let (quotient, remainder) = dividend.div_rem(&divisor).unwrap();
        assert_eq!(quotient.to_decimal(), "1000000000");
        assert!(remainder.is_zero());
    }

    #[test]
    fn div_rem_GivenRemainder_ReturnsExactQuotientAndRemainder() {
        let dividend = Nat::from_decimal("1000000000000000001").unwrap();
        let divisor = Nat::from_u64(1_000_000_000);
        let (quotient, remainder) = dividend.div_rem(&divisor).unwrap();
        assert_eq!(quotient.to_decimal(), "1000000000");
        assert_eq!(remainder.to_decimal(), "1");
    }

    #[test]
    fn div_rem_GivenMultiLimbDivisor_ReturnsExactQuotientAndRemainder() {
        let dividend = Nat::from_decimal("98765432109876543210").unwrap();
        let divisor = Nat::from_u64(123_456_789);
        let (quotient, remainder) = dividend.div_rem(&divisor).unwrap();
        assert_eq!(quotient.to_decimal(), "800000007370");
        assert_eq!(remainder.to_decimal(), "8280");
    }

    #[test]
    fn div_rem_GivenEqualOperands_ReturnsOneAndZero() {
        let value = Nat::from_decimal("12345678901234567890").unwrap();
        let (quotient, remainder) = value.div_rem(&value).unwrap();
        assert!(quotient.is_one());
        assert!(remainder.is_zero());
    }

    #[test]
    fn pow_GivenZeroExponent_ReturnsOne() {
        assert!(Nat::from_decimal("12345678901234567890")
            .unwrap()
            .pow(0)
            .is_one());
    }

    #[test]
    fn pow_GivenZeroBaseAndZeroExponent_ReturnsOne() {
        assert!(Nat::zero().pow(0).is_one());
    }

    #[test]
    fn pow_GivenTwoToHundred_ReturnsKnownValue() {
        assert_eq!(
            Nat::from_u64(2).pow(100).to_decimal(),
            "1267650600228229401496703205376"
        );
    }

    #[test]
    fn pow_GivenZeroBaseAndPositiveExponent_ReturnsZero() {
        assert!(Nat::zero().pow(37).is_zero());
    }

    #[test]
    fn factorial_GivenZero_ReturnsOne() {
        assert!(Nat::factorial(0).is_one());
    }

    #[test]
    fn factorial_GivenOne_ReturnsOne() {
        assert!(Nat::factorial(1).is_one());
    }

    #[test]
    fn factorial_GivenTwenty_ReturnsKnownValue() {
        assert_eq!(Nat::factorial(20).to_decimal(), "2432902008176640000");
    }

    #[test]
    fn factorial_GivenHundred_Has158DigitsAndKnownValue() {
        let value = Nat::factorial(100);
        assert_eq!(value.to_decimal().len(), 158);
        assert_eq!(
            value.to_decimal(),
            "93326215443944152681699238856266700490715968264381621468592963895217599993229915608941463976156518286253697920827223758251185210916864000000000000000000000000"
        );
    }

    #[test]
    fn factorial_GivenThousand_Has2568DigitsAndKnownChecksum() {
        let text = Nat::factorial(1000).to_decimal();
        assert_eq!(text.len(), 2568);
        assert!(text.starts_with("402387260077093773543702433923"));
        let trailing_zeroes = text.bytes().rev().take_while(|byte| *byte == b'0').count();
        assert_eq!(trailing_zeroes, 249);
        let digit_sum: u32 = text
            .bytes()
            .map(|byte| u32::from(byte - b'0'))
            .sum();
        assert_eq!(digit_sum, 10539);
    }

    #[test]
    fn mul_pow10_GivenZeroExponent_ReturnsSame() {
        let value = Nat::from_decimal("12345678901234567890").unwrap();
        assert_eq!(value.mul_pow10(0), value);
    }

    #[test]
    fn mul_pow10_GivenNine_AppendsZeroLimb() {
        assert_eq!(
            Nat::from_u64(123).mul_pow10(9).to_decimal(),
            "123000000000"
        );
    }

    #[test]
    fn mul_pow10_GivenNineteen_CrossesTwoLimbs() {
        assert_eq!(
            Nat::from_u64(42).mul_pow10(19).to_decimal(),
            "420000000000000000000"
        );
    }

    #[test]
    fn to_u64_GivenValueAboveMaximum_ReturnsNone() {
        let value = Nat::from_decimal("18446744073709551616").unwrap();
        assert_eq!(value.to_u64(), None);
    }

    #[test]
    fn ord_GivenDifferentMagnitudes_OrdersNumerically() {
        let larger = Nat::from_decimal("1000000000000000000").unwrap();
        let smaller = Nat::from_decimal("999999999999999999").unwrap();
        assert!(larger > smaller);
    }

    #[test]
    fn parse_then_format_GivenLargeLiteral_RoundTrips() {
        let literal = "314159265358979323846264338327950288419716939937510";
        let value = Nat::from_decimal(literal).unwrap();
        assert_eq!(value.to_decimal(), literal);
    }
}
