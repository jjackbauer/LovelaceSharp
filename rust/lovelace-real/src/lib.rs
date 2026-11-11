//! Arbitrary-precision reals with exact periodic decimals.
//!
//! A value is `int * 10^exp`, plus optional period metadata: when the value has a repeating
//! decimal expansion, `period_start`/`period_len` describe the block that repeats forever and the
//! stored digits contain that block exactly once. `1/3` is therefore stored as the digits `3` with
//! exponent `-1` and a period of length one, and renders as `0.(3)` — never as a rounding.
//!
//! Arithmetic on terminating values is done directly on the scaled integers, which keeps
//! `sqrt`/`pi` fast. Arithmetic involving a repeating value converts to an exact rational first,
//! so `1/3 + 1/3` is `0.(6)` and not `0.6`.

use std::cell::Cell;
use std::cmp::Ordering;

use lovelace_int::Int;
use lovelace_nat::Nat;

// The DISPLAY digit cap (the reference's Real.DisplayDecimalPlaces, default 100): how many
// fractional digits a non-periodic value publishes. Library code defaults to NO cap so internal
// arithmetic and exact-value conversions keep every computed digit; the language engine scopes
// its display precision around the wire projection, which is where the reference applies it.
thread_local! {
    static DISPLAY_DECIMAL_PLACES: Cell<usize> = const { Cell::new(usize::MAX) };
}

/// The fractional-digit cap in force on this thread.
pub fn display_decimal_places() -> usize {
    DISPLAY_DECIMAL_PLACES.with(Cell::get)
}

/// Runs a closure with the display cap set to the given fractional digits, restoring the previous
/// cap afterwards (the reference's AsyncLocal Real.DisplayDecimalPlaces scope).
pub fn with_display_decimal_places<T>(places: usize, f: impl FnOnce() -> T) -> T {
    struct Restore(usize);
    impl Drop for Restore {
        fn drop(&mut self) {
            DISPLAY_DECIMAL_PLACES.with(|slot| slot.set(self.0));
        }
    }
    let previous = DISPLAY_DECIMAL_PLACES.with(|slot| slot.replace(places));
    let _restore = Restore(previous);
    f()
}

pub mod lreal;
pub mod round39_api;
pub mod transcendental;

#[cfg(test)]
mod lreal_tests;
#[cfg(test)]
mod round26_trig_parity;
#[cfg(test)]
mod round29_real_batch;
#[cfg(test)]
mod round30_real_batch;
#[cfg(test)]
mod round31_trig;
#[cfg(test)]
mod round34_inexact_division;
#[cfg(test)]
mod round36_ambient;
#[cfg(test)]
mod round36_precbench;
#[cfg(test)]
mod round39_real_tests;

/// Builds a real from an exact rational under the reference's computation cap.
///
/// The denominator's factors of ten are carried by the EXPONENT rather than spent as leading
/// fractional zeros, so a cancellation that lands a period 1000 places out (1/3 - 0.333…3) still
/// reports the exact period instead of a truncated decimal. The remaining expansion is detected
/// within [Real::DEFAULT_COMPUTATION_DECIMAL_PLACES], the same cap the reference applies.
fn fraction_real_exact(num: Int, den: &Nat) -> Real {
    // Cancel the tens the numerator and denominator SHARE (that is a no-op on the value), and
    // carry only the leftover tens on the exponent. Cancelling just the denominator would move
    // the decimal point into the middle of the quotient's integer digits (4/3 * 3/4 used to come
    // back as 1.00).
    let mut numerator = num;
    let mut denominator = den.clone();
    let mut scale = 0usize;
    while let Some((quotient, remainder)) = denominator.div_rem(&Nat::from_u64(10)) {
        if !remainder.is_zero() {
            break;
        }
        denominator = quotient;
        let (numerator_quotient, numerator_remainder) = numerator
            .div_rem(&Nat::from_u64(10))
            .expect("nonzero divisor");
        if numerator_remainder.is_zero() {
            numerator = numerator_quotient;
        } else {
            scale += 1;
        }
    }
    let mut value = fraction_real(
        numerator,
        denominator,
        Real::DEFAULT_COMPUTATION_DECIMAL_PLACES,
    );
    if scale > 0 {
        value.exp -= scale as i64;
        if value.period_len > 0 {
            value.period_start += scale as i64;
        }
    }
    // The computation cap bounds the FRACTIONAL EXPANSION, not just the period: a value whose
    // period only closes at position 1000 (1/3 - 0.333...3 with 1000 threes) needs 1001 places,
    // so the reference's conversion stopped at the cap and marks the value inexact even though
    // the detected period is kept. (998/999 threes stay exact; 1000 is the boundary.)
    if value.period_len > 0
        && value.period_start + value.period_len > Real::DEFAULT_COMPUTATION_DECIMAL_PLACES as i64
    {
        value.inexact = true;
    }
    // An exact rational publishes its canonical decimal form: no trailing fractional zeros.
    value.normalized()
}

/// Builds an integer from a natural magnitude.
fn int_from_nat(mag: &Nat) -> Int {
    Int::from_decimal(&mag.to_decimal()).unwrap_or_else(Int::zero)
}

/// An arbitrary-precision real number.
#[derive(Clone, Debug)]
pub struct Real {
    /// Signed integer holding every stored digit.
    int: Int,
    /// Decimal exponent: the value is `int * 10^exp`.
    exp: i64,
    /// Index of the first repeating digit, or -1 when the expansion terminates.
    period_start: i64,
    /// Length of the repeating block, or 0 when the expansion terminates.
    period_len: i64,
    /// Provenance, not value: true when producing this value dropped a digit (a truncated
    /// division, an irrational root, `pi`). Equality, comparison and rendering ignore it.
    inexact: bool,
}

impl Real {
    /// The value zero.
    pub fn zero() -> Real {
        Real {
            int: Int::zero(),
            exp: 0,
            period_start: -1,
            period_len: 0,
            inexact: false,
        }
    }

    /// Builds a real from a signed integer.
    pub fn from_int(value: &Int) -> Real {
        Real {
            int: value.clone(),
            exp: 0,
            period_start: -1,
            period_len: 0,
            inexact: false,
        }
    }

    /// Builds a real from an `i64`.
    pub fn from_i64(value: i64) -> Real {
        Real::from_int(&Int::from_i64(value))
    }

    /// Index of the first repeating fractional digit, or -1 when the expansion terminates.
    pub fn period_start(&self) -> i64 {
        self.period_start
    }

    /// Length of the repeating block, or 0 when the expansion terminates.
    pub fn period_length(&self) -> i64 {
        self.period_len
    }

    /// True when the value was produced without dropping a digit.
    ///
    /// This is provenance, not value: an irrational root and its truncated decimal are the same
    /// stored number, but only the latter is exact. Equality and rendering ignore the flag.
    pub fn is_exact(&self) -> bool {
        !self.inexact
    }

    /// Marks this value as the product of a truncation and returns it.
    pub fn mark_inexact(mut self) -> Real {
        self.inexact = true;
        self
    }

    /// Carries the provenance of two operands onto their result.
    fn with_provenance_of2(mut self, left: &Real, right: &Real) -> Real {
        if left.inexact || right.inexact {
            self.inexact = true;
        }
        self
    }

    /// True when the value has no fractional part.
    pub fn is_integer(&self) -> bool {
        if self.period_len > 0 {
            return false;
        }
        match self.exp.cmp(&0) {
            Ordering::Equal => true,
            Ordering::Greater => true,
            Ordering::Less => {
                // int must be divisible by 10^(-exp)
                let mut cur = self.int.clone();
                for _ in 0..(-self.exp) {
                    let (q, r) = cur.div_rem(&Nat::from_u64(10)).expect("nonzero divisor");
                    if !r.is_zero() {
                        return false;
                    }
                    cur = q;
                }
                true
            }
        }
    }

    /// Returns the value as an integer when it is integral.
    pub fn to_int_exact(&self) -> Option<Int> {
        if !self.is_integer() {
            return None;
        }
        let mut cur = self.int.clone();
        if self.exp > 0 {
            for _ in 0..self.exp {
                cur = cur.mul(&Int::from_i64(10));
            }
        } else if self.exp < 0 {
            for _ in 0..(-self.exp) {
                let (q, r) = cur.div_rem(&Nat::from_u64(10))?;
                if !r.is_zero() {
                    return None;
                }
                cur = q;
            }
        }
        Some(cur)
    }

    /// The exact rational this value equals, reduced to lowest terms: (numerator, denominator).
    ///
    /// A Real is a rational at heart (a terminating decimal or a detected period), so an exact
    /// value publishes as numerator/denominator rather than forcing a consumer to parse "0.(3)".
    pub fn to_rational(&self) -> (Int, Nat) {
        let (num, den) = self.to_fraction();
        let mut a = num.magnitude().clone();
        let mut b = den.clone();
        while !b.is_zero() {
            let remainder = a.div_rem(&b).map(|(_, r)| r).unwrap_or_else(Nat::zero);
            a = b;
            b = remainder;
        }
        if a.is_zero() || a.to_decimal() == "1" {
            return (num, den);
        }
        let (n, _) = num.div_rem(&a).unwrap_or((num.clone(), Int::from_i64(0)));
        let (d, _) = den.div_rem(&a).unwrap_or((den.clone(), Nat::zero()));
        (n, d)
    }

    /// True when the value is zero.
    pub fn is_zero(&self) -> bool {
        self.int.is_zero()
    }

    /// Absolute value.
    pub fn abs(&self) -> Real {
        let mut c = self.clone();
        if c.int.is_negative() {
            c.int = c.int.negate();
        }
        c
    }

    /// Negation.
    pub fn neg(&self) -> Real {
        let mut c = self.clone();
        c.int = c.int.negate();
        c
    }

    /// Number of stored fractional digits.
    fn frac_digits(&self) -> usize {
        if self.exp < 0 {
            (-self.exp) as usize
        } else {
            0
        }
    }

    /// Removes trailing fractional zeros from a terminating value.
    fn normalized(mut self) -> Real {
        if self.period_len > 0 {
            return self;
        }
        while self.exp < 0 {
            let (q, r) = match self.int.div_rem(&Nat::from_u64(10)) {
                Some(v) => v,
                None => return self,
            };
            if !r.is_zero() {
                break;
            }
            self.int = q;
            self.exp += 1;
        }
        self
    }

    /// Parses `3.14`, `-0.5`, `0.(3)`, `0.1(6)`, `2`.
    // The reference's `Real.FromString` is permissive and optional (a failed parse is a normal
    // answer, not an error value). `FromStr` would fix a different contract (Result + error type),
    // and every ported call site uses this exact name, so the inherent method stays.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(text: &str) -> Option<Real> {
        let text = text.trim();
        if text.is_empty() {
            return None;
        }
        let (negative, body) = match text.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, text.strip_prefix('+').unwrap_or(text)),
        };
        let (int_part, frac_part) = match body.split_once('.') {
            Some((a, b)) => (a, b),
            None => (body, ""),
        };
        if int_part.is_empty() && frac_part.is_empty() {
            return None;
        }
        if !int_part.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        // Fraction is either plain digits or digits followed by a parenthesised period.
        let (plain, period) = match frac_part.find('(') {
            Some(open) => {
                if !frac_part.ends_with(')') {
                    return None;
                }
                let inner = &frac_part[open + 1..frac_part.len() - 1];
                if inner.is_empty() || !inner.chars().all(|c| c.is_ascii_digit()) {
                    return None;
                }
                (&frac_part[..open], Some(inner))
            }
            None => (frac_part, None),
        };
        if !plain.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let mut digits = String::new();
        digits.push_str(int_part);
        digits.push_str(plain);
        let mut period_start: i64 = -1;
        let mut period_len: i64 = 0;
        if let Some(p) = period {
            period_start = plain.len() as i64;
            period_len = p.len() as i64;
            digits.push_str(p);
        }
        if digits.is_empty() {
            digits.push('0');
        }
        let magnitude = Nat::from_decimal(&digits)?;
        let mut int = int_from_nat(&magnitude);
        if negative {
            int = int.negate();
        }
        let mut value = Real {
            int,
            exp: -((digits.len() - int_part.len()) as i64),
            period_start,
            period_len,
            inexact: false,
        };
        value = value.normalized();
        Some(value)
    }

    /// Renders the canonical display form: `2`, `0.25`, `0.(3)`, `0.1(6)`, `3.14159`.
    fn render(&self) -> String {
        let mut out = String::new();
        if self.int.is_negative() {
            out.push('-');
        }
        let digits = self.int.magnitude().to_decimal();
        let frac = self.frac_digits();
        if frac == 0 {
            out.push_str(&digits);
            return out;
        }
        let mut padded = digits;
        while padded.len() <= frac {
            padded.insert(0, '0');
        }
        let split = padded.len() - frac;
        let (ip, fp) = padded.split_at(split);
        out.push_str(ip);
        if self.period_len > 0 {
            let s = self.period_start.max(0) as usize;
            let k = self.period_len as usize;
            let pre = &fp[..s.min(fp.len())];
            let per = &fp[s.min(fp.len())..(s + k).min(fp.len())];
            if !pre.is_empty() || !per.is_empty() {
                out.push('.');
                out.push_str(pre);
                out.push('(');
                out.push_str(per);
                out.push(')');
            }
        } else {
            // The stored scale IS the display scale: values are normalised when they are
            // computed, so a fixed-precision value such as pi(50) keeps its trailing zero.
            // A non-periodic value is truncated at the active display cap (the reference's
            // Real.DisplayDecimalPlaces); a periodic one is always rendered in full above.
            if !fp.is_empty() {
                let cap = display_decimal_places();
                let visible = &fp[..fp.len().min(cap)];
                if !visible.is_empty() {
                    out.push('.');
                    out.push_str(visible);
                }
            }
        }
        out
    }

    /// Truncates a terminating value to at most `digits` fractional places, mirroring the
    /// reference's DISPLAY cap (`Real.DisplayDecimalPlaces`), which bounds how many digits a
    /// non-periodic value publishes. A periodic value is returned unchanged: the reference renders
    /// its repeating block in full and applies the display limit only to a finite expansion.
    pub fn truncated_frac(&self, digits: usize) -> Real {
        if self.period_len > 0 || self.exp >= -(digits as i64) {
            return self.clone();
        }
        let drop = (-self.exp - digits as i64) as u64;
        let ten = Nat::from_u64(10).pow(drop);
        let Some((q, _)) = self.int.div_rem(&ten) else {
            return self.clone();
        };
        Real {
            int: q,
            exp: -(digits as i64),
            period_start: -1,
            period_len: 0,
            inexact: self.inexact,
        }
        .normalized()
    }

    /// Renders exactly `digits` fractional digits, expanding a repeating block and
    /// padding a terminating expansion with zeros.
    pub fn to_decimal_fixed(&self, digits: usize) -> String {
        let display = self.to_string();
        let (sign, body) = match display.strip_prefix('-') {
            Some(rest) => ("-", rest.to_string()),
            None => ("", display.clone()),
        };
        let (int_part, frac_part) = match body.split_once('.') {
            Some((i, f)) => (i.to_string(), f.to_string()),
            None => (body.clone(), String::new()),
        };
        let (pre, period) = match frac_part.find('(') {
            Some(open) => {
                let pre = frac_part[..open].to_string();
                let per = frac_part[open + 1..frac_part.len().saturating_sub(1)].to_string();
                (pre, Some(per))
            }
            None => (frac_part.clone(), None),
        };
        let mut frac = pre.clone();
        if let Some(per) = period {
            if per.is_empty() {
                while frac.len() < digits {
                    frac.push('0');
                }
            } else {
                let mut k = 0usize;
                while frac.len() < digits {
                    let ch = per.chars().nth(k % per.chars().count()).unwrap_or('0');
                    frac.push(ch);
                    k += 1;
                }
            }
        } else {
            while frac.len() < digits {
                frac.push('0');
            }
        }
        let frac: String = frac.chars().take(digits).collect();
        if digits == 0 {
            return format!("{}{}", sign, int_part);
        }
        format!("{}{}.{}", sign, int_part, frac)
    }

    /// Exact rational form `(numerator, denominator)` with a positive denominator.
    fn to_fraction(&self) -> (Int, Nat) {
        let ten = Nat::from_u64(10);
        if self.period_len == 0 {
            // int / 10^(-exp)
            let den = if self.exp >= 0 {
                Nat::from_u64(1)
            } else {
                ten.pow((-self.exp) as u64)
            };
            let num = if self.exp > 0 {
                self.int.mul(&int_from_nat(&ten.pow(self.exp as u64)))
            } else {
                self.int.clone()
            };
            return (num, den);
        }
        let l = self.frac_digits();
        let s = self.period_start.max(0) as usize;
        let k = self.period_len as usize;
        let mag = self.int.magnitude().to_decimal();
        let mut padded = mag;
        while padded.len() < l + 1 {
            padded.insert(0, '0');
        }
        let split = padded.len() - l;
        let (ip, fp) = padded.split_at(split);
        let int_part = Nat::from_decimal(ip).unwrap_or_else(Nat::zero);
        let pre = &fp[..s.min(fp.len())];
        let per = &fp[s.min(fp.len())..(s + k).min(fp.len())];
        let pre_n = if pre.is_empty() {
            Nat::zero()
        } else {
            Nat::from_decimal(pre).unwrap_or_else(Nat::zero)
        };
        let per_n = if per.is_empty() {
            Nat::zero()
        } else {
            Nat::from_decimal(per).unwrap_or_else(Nat::zero)
        };
        // value = int_part + pre/10^s + per/(10^s * (10^k - 1)), so over the common
        // denominator the repeating block contributes `per` itself, not `per * 10^s`.
        let ten_s = ten.pow(s as u64);
        let ten_k_minus_1 = Nat::from_u64(10).pow(k as u64);
        let nine = ten_k_minus_1
            .sub(&Nat::from_u64(1))
            .unwrap_or_else(Nat::zero);
        let den = ten_s.mul(&nine.clone());
        let mut num = int_part.mul(&den.clone());
        num = num.add(&pre_n.mul(&nine));
        num = num.add(&per_n);
        let num_int = if self.int.is_negative() {
            int_from_nat(&num).negate()
        } else {
            int_from_nat(&num)
        };
        (num_int, den)
    }

    /// The computation cap the reference applies to an exact rational
    /// (Real.MaxComputationDecimalPlaces): a period longer than this is truncated and the value
    /// is marked inexact. It bounds a NON-TERMINATING expansion only.
    pub const DEFAULT_COMPUTATION_DECIMAL_PLACES: usize = 1000;

    /// Builds a real from a rational, detecting the repeating period.
    ///
    /// This is the reference's `Divide` digit loop, including its two accounting rules:
    ///
    /// * A terminating expansion is exact at ANY budget (the reference answers it with
    ///   `TryTerminateExactly` before entering the loop), so it is expanded in full.
    /// * A quotient below one opens with a run of zeros. Those zeros are SCALE, not digits: they
    ///   are carried by the exponent and do not consume a stored place. While the run is shorter
    ///   than the budget it is walked normally; once the run ALONE fills the budget the reference
    ///   "frees" the accounting, and the budget then bounds the STORED digits instead. Without
    ///   that rule `sin(1e-100)/cos(1e-100)` spent the whole 30-place budget on the leading zeros
    ///   and flushed the quotient to zero.
    ///
    /// A period that closes exactly AT the cap is never discovered: the reference's `while`
    /// condition (`position < MaxComputationDecimalPlaces`) is tested before the repeated-remainder
    /// test that would run on the next iteration. The quotient is then the budgeted truncation,
    /// inexact, with no period metadata, and `Normalize` drops any trailing fractional zeros.
    pub fn from_fraction(num: &Nat, den: &Nat, max_digits: usize) -> Option<Real> {
        let (q, r) = num.div_rem(den)?;
        let mut int = int_from_nat(&q);
        if r.is_zero() {
            return Some(Real {
                int,
                exp: 0,
                period_start: -1,
                period_len: 0,
                inexact: false,
            });
        }
        let max_digits = max_digits.max(1);
        let terminating = terminates(&r, den);
        // Stored fractional digits; the opening zero run is NOT stored (it is scale).
        let mut digits: Vec<u8> = Vec::new();
        let mut seen: Vec<(Nat, usize)> = Vec::new();
        let mut rem = r;
        let mut period_start: i64 = -1;
        let mut period_len: i64 = 0;
        let mut leading_zeros = 0usize;
        let mut position = 0usize;
        let mut freed = false;
        let mut skipping_zeros = q.is_zero();
        loop {
            if rem.is_zero() {
                break;
            }
            // The reference's `while` condition: a terminating expansion has already been
            // answered exactly, so only a non-terminating one is bounded -- by decimal POSITION
            // until the leading zero run alone fills the budget, then by STORED digits.
            if !terminating {
                if freed {
                    if digits.len() >= max_digits {
                        break;
                    }
                } else if position >= max_digits {
                    break;
                }
            }
            if let Some((_, idx)) = seen.iter().find(|(v, _)| *v == rem) {
                period_start = *idx as i64;
                period_len = (position - *idx) as i64;
                break;
            }
            seen.push((rem.clone(), position));
            let scaled = rem.mul(&Nat::from_u64(10));
            let (d, nr) = scaled.div_rem(den)?;
            rem = nr;
            position += 1;
            let digit = d.to_u64().unwrap_or(0) as u8;
            if skipping_zeros && digit == 0 {
                leading_zeros += 1;
                if !freed && position >= max_digits {
                    freed = true;
                }
                continue;
            }
            skipping_zeros = false;
            digits.push(digit);
        }
        let truncated = period_len == 0 && !rem.is_zero();
        let mut text = String::new();
        for d in &digits {
            text.push((b'0' + *d) as char);
        }
        if text.is_empty() {
            text.push('0');
        }
        let mag = Nat::from_decimal(&text)?;
        // `int * 10^exp` is q `leading_zeros` places along plus the stored digits after them:
        // q*10^len + digits, at scale 10^-(leading_zeros + len).
        let scale = Nat::from_u64(10).pow(digits.len() as u64);
        int = int_from_nat(&q)
            .mul(&int_from_nat(&scale))
            .add(&int_from_nat(&mag));
        let mut value = Real {
            int,
            exp: -((leading_zeros + digits.len()) as i64),
            period_start,
            period_len,
            inexact: truncated,
        };
        // A non-periodic result publishes the reference's normalised form: `Divide` runs
        // `Normalize` on it, dropping trailing fractional zeros. At a 20-place budget 2*pi/3 is
        // therefore `2.0943951023931954923`, not `...9230`; a periodic value keeps its stored
        // block untouched (the reference does not normalize it either).
        if value.period_len == 0 {
            value = value.normalized();
        }
        Some(value)
    }

    /// Addition.
    pub fn add(&self, other: &Real) -> Real {
        if self.period_len == 0 && other.period_len == 0 {
            let (a, b, e) = align(self, other);
            let mut r = Real {
                int: a.add(&b),
                exp: e,
                period_start: -1,
                period_len: 0,
                inexact: self.inexact || other.inexact,
            };
            r = r.normalized();
            return r;
        }
        self.arithmetic_via_fraction(other, |a, b| (a.add(b), Nat::from_u64(1)))
    }

    /// Subtraction.
    pub fn sub(&self, other: &Real) -> Real {
        self.add(&other.neg())
    }

    /// Multiplication.
    pub fn mul(&self, other: &Real) -> Real {
        if self.period_len == 0 && other.period_len == 0 {
            let mut r = Real {
                int: self.int.mul(&other.int),
                exp: self.exp + other.exp,
                period_start: -1,
                period_len: 0,
                inexact: self.inexact || other.inexact,
            };
            r = r.normalized();
            return r;
        }
        let (an, ad) = self.to_fraction();
        let (bn, bd) = other.to_fraction();
        let num = an.mul(&bn);
        let den = ad.mul(&bd);
        fraction_real_exact(num, &den).with_provenance_of2(self, other)
    }

    /// Division. `None` on a zero divisor.
    pub fn div(&self, other: &Real, max_digits: usize) -> Option<Real> {
        if other.is_zero() {
            return None;
        }
        let (an, ad) = self.to_fraction();
        let (bn, bd) = other.to_fraction();
        // (an/ad) / (bn/bd) = an*bd / (ad*bn)
        let num = an.mul(&int_from_nat(&bd));
        let den = ad.mul(bn.magnitude());
        let quotient = if bn.is_negative() {
            fraction_real(num.negate(), den, max_digits)
        } else {
            fraction_real(num, den, max_digits)
        };
        Some(quotient.with_provenance_of2(self, other))
    }

    fn arithmetic_via_fraction(&self, other: &Real, _f: impl Fn(&Nat, &Nat) -> (Nat, Nat)) -> Real {
        let (an, ad) = self.to_fraction();
        let (bn, bd) = other.to_fraction();
        // an/ad + bn/bd = (an*bd + bn*ad) / (ad*bd)
        let left = an.mul(&int_from_nat(&bd));
        let right = bn.mul(&int_from_nat(&ad));
        let num = left.add(&right);
        let den = ad.mul(&bd);
        // The exact sum is re-derived as a period of the RESULT. Bounding the expansion by the
        // operands' own fractional digits truncated every sum whose period was longer than they
        // were (1/19 + 1/17 has a 144-digit period), publishing a wrong value as exact.
        fraction_real_exact(num, &den).with_provenance_of2(self, other)
    }

    /// Raises to an integer power.
    pub fn pow_int(&self, exp: i64) -> Option<Real> {
        if exp == 0 {
            return Some(Real::from_i64(1));
        }
        if exp < 0 {
            let base = self.pow_int(-exp)?;
            return Real::from_i64(1).div(&base, 1000);
        }
        let mut acc = Real::from_i64(1);
        for _ in 0..exp {
            acc = acc.mul(self);
        }
        Some(acc)
    }

    /// Square root truncated to `digits` fractional digits.
    ///
    /// Perfect squares come back exact and therefore render without a decimal point.
    pub fn sqrt(&self, digits: usize) -> Real {
        if self.int.is_negative() {
            return Real::zero();
        }
        if self.is_zero() {
            return Real::zero();
        }
        let (num, den) = self.to_fraction();
        // sqrt(num/den) = sqrt(num*den) / den, scaled by a power of ten for the result digits.
        let scaled = num.magnitude().mul(&den);
        let guard = 10usize;
        let shift = 2 * (digits + guard);
        let big = scaled.mul(&Nat::from_u64(10).pow(shift as u64));
        let root = isqrt(&big);
        let (scaled_root, _) = root.div_rem(&den).expect("nonzero divisor");
        let (q, _) = scaled_root
            .div_rem(&Nat::from_u64(10).pow(guard as u64))
            .expect("nonzero divisor");
        let text = q.to_decimal();
        let mut s = String::new();
        let want = digits + 1;
        if text.len() < want {
            for _ in 0..(want - text.len()) {
                s.push('0');
            }
        }
        s.push_str(&text);
        let split = s.len() - digits;
        let (ip, fp) = s.split_at(split);
        let mut assembled = String::from(ip);
        assembled.push('.');
        assembled.push_str(fp);
        let result = Real::from_str(&assembled).unwrap_or_else(Real::zero);
        // A root that does not square back to the stored value is a truncation of an irrational;
        // a perfect square keeps the input's provenance (an inexact input stays inexact).
        if result.mul(&result).cmp(self) == Ordering::Equal && self.is_exact() {
            result
        } else {
            result.mark_inexact()
        }
    }

    /// Pi truncated to `digits` fractional digits (Machin's formula).
    pub fn pi(digits: usize) -> Real {
        let guard = 12usize;
        let scale_exp = digits + guard;
        let scale = Nat::from_u64(10).pow(scale_exp as u64);
        let atan_inv = |x: u64| -> Nat {
            let mut total = Nat::zero();
            let mut power = scale.clone();
            let x_nat = Nat::from_u64(x);
            let x2 = x_nat.mul(&x_nat);
            let mut k: u64 = 0;
            loop {
                let (term, _) = power.div_rem(&x_nat).expect("nonzero divisor");
                let (t, _) = term
                    .div_rem(&Nat::from_u64(2 * k + 1))
                    .expect("nonzero divisor");
                if t.is_zero() {
                    break;
                }
                total = if k % 2 == 0 {
                    total.add(&t)
                } else {
                    total.sub(&t).unwrap_or_else(Nat::zero)
                };
                let (next, _) = power.div_rem(&x2).expect("nonzero divisor");
                power = next;
                k += 1;
                if k > 10_000 {
                    break;
                }
            }
            total
        };
        let a = atan_inv(5);
        let b = atan_inv(239);
        let sixteen_a = a.mul(&Nat::from_u64(16));
        let four_b = b.mul(&Nat::from_u64(4));
        let pi_scaled = sixteen_a.sub(&four_b).unwrap_or_else(Nat::zero);
        // pi_scaled is pi * 10^(digits+guard); drop the guard digits.
        let (truncated, _) = pi_scaled
            .div_rem(&Nat::from_u64(10).pow(guard as u64))
            .expect("nonzero divisor");
        // pi is irrational: the stored digits are a truncation, whatever their count.
        Real::from_scaled(int_from_nat(&truncated), -(digits as i64)).mark_inexact()
    }

    /// Builds a value with an explicit scale, without normalising (used by pi, whose
    /// published digits include a trailing zero at 50 places).
    fn from_scaled(int: Int, exp: i64) -> Real {
        Real {
            int,
            exp,
            period_start: -1,
            period_len: 0,
            inexact: false,
        }
    }
}

/// True when `num/den` has a finite decimal expansion **by the reference's rule**: the
/// DENOMINATOR's only prime factors are 2 and 5. Such a quotient is exact at ANY digit budget.
///
/// The reference (`TryTerminateExactly`) does NOT cancel the numerator against the denominator
/// first. That matters when the two share a factor: `pi(100)/6`, whose stored numerator happens to
/// be divisible by 3, is mathematically the 101-digit decimal `...4465`, but the reference's
/// denominator is still 2*3*10^100, so it takes the digit loop, spends the 100-place budget, and
/// stores the 100-digit truncation `...446` (inexact, no period). Reducing first would publish the
/// extra digit as exact and break the reference's row. The digit loop still answers exactly when
/// the remainder reaches zero inside the budget; only the "finish beyond the budget" case differs.
fn terminates(_num: &Nat, den: &Nat) -> bool {
    let mut d = den.clone();
    for prime in [2u64, 5u64] {
        loop {
            match d.div_rem(&Nat::from_u64(prime)) {
                Some((q, r)) if r.is_zero() => d = q,
                _ => break,
            }
        }
    }
    d.to_decimal() == "1"
}

fn fraction_real(num: Int, den: Nat, max_digits: usize) -> Real {
    let negative = num.is_negative();
    let mag = num.magnitude().clone();
    let value = Real::from_fraction(&mag, &den, max_digits).unwrap_or_else(Real::zero);
    if negative { value.neg() } else { value }
}

/// Aligns two terminating values to a common exponent.
fn align(a: &Real, b: &Real) -> (Int, Int, i64) {
    let e = a.exp.min(b.exp);
    let ten = |n: i64| {
        if n <= 0 {
            Nat::from_u64(1)
        } else {
            Nat::from_u64(10).pow(n as u64)
        }
    };
    let a_scaled = a.int.mul(&int_from_nat(&ten(a.exp - e)));
    let b_scaled = b.int.mul(&int_from_nat(&ten(b.exp - e)));
    (a_scaled, b_scaled, e)
}

/// Integer square root by Newton iteration.
fn isqrt(n: &Nat) -> Nat {
    if n.is_zero() {
        return Nat::zero();
    }
    let text = n.to_decimal();
    match text.len() {
        0 | 1 if text == "1" => return Nat::from_u64(1),
        _ => {}
    }
    // initial guess: 10^ceil(len/2)
    let guess_exp = text.len().div_ceil(2) as u64;
    let mut x = Nat::from_u64(10).pow(guess_exp);
    loop {
        let (q, _) = n.div_rem(&x).expect("nonzero divisor");
        let sum = x.add(&q);
        let (next, _) = sum.div_rem(&Nat::from_u64(2)).expect("nonzero divisor");
        if next.cmp(&x) != Ordering::Less {
            return x;
        }
        x = next;
    }
}

impl PartialEq for Real {
    fn eq(&self, other: &Self) -> bool {
        Ord::cmp(self, other) == Ordering::Equal
    }
}

impl Eq for Real {}

impl PartialOrd for Real {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for Real {
    /// Compares two reals exactly.
    fn cmp(&self, other: &Self) -> Ordering {
        let (an, ad) = self.to_fraction();
        let (bn, bd) = other.to_fraction();
        let left = an.mul(&int_from_nat(&bd));
        let right = bn.mul(&int_from_nat(&ad));
        left.cmp(&right)
    }
}

impl std::fmt::Display for Real {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.render())
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]
    use super::*;

    fn r(s: &str) -> Real {
        Real::from_str(s).unwrap()
    }

    #[test]
    fn ToString_GivenThird_RendersRepeatingPeriod() {
        let third = Real::from_fraction(&Nat::from_u64(1), &Nat::from_u64(3), 100).unwrap();
        assert_eq!(third.to_string(), "0.(3)");
    }

    #[test]
    fn ToString_GivenQuarter_RendersTerminating() {
        let v = Real::from_fraction(&Nat::from_u64(1), &Nat::from_u64(4), 100).unwrap();
        assert_eq!(v.to_string(), "0.25");
    }

    #[test]
    fn ToString_GivenSixth_RendersPreperiodThenPeriod() {
        let v = Real::from_fraction(&Nat::from_u64(1), &Nat::from_u64(6), 100).unwrap();
        assert_eq!(v.to_string(), "0.1(6)");
    }

    #[test]
    fn ToString_GivenSeventh_RendersSixDigitPeriod() {
        let v = Real::from_fraction(&Nat::from_u64(1), &Nat::from_u64(7), 100).unwrap();
        assert_eq!(v.to_string(), "0.(142857)");
    }

    #[test]
    fn ToString_GivenInteger_HasNoDecimalPoint() {
        assert_eq!(r("2").to_string(), "2");
        assert_eq!(Real::from_i64(-5).to_string(), "-5");
    }

    #[test]
    fn Add_GivenTwoTenthsAndTwoTenths_IsExact() {
        let a = r("0.1");
        let b = r("0.2");
        assert_eq!(a.add(&b).to_string(), "0.3");
    }

    #[test]
    fn Add_GivenTwoThirds_ProducesRepeatingSixth() {
        let third = r("0.(3)");
        let sum = third.add(&third);
        assert_eq!(sum.to_string(), "0.(6)");
    }

    #[test]
    fn Mul_GivenDecimalAndInteger_IsExact() {
        assert_eq!(r("3.14").mul(&Real::from_i64(2)).to_string(), "6.28");
    }

    #[test]
    fn Div_GivenOneByEight_Terminates() {
        let v = Real::from_i64(1).div(&Real::from_i64(8), 100).unwrap();
        assert_eq!(v.to_string(), "0.125");
    }

    #[test]
    fn Div_GivenOneByZero_ReturnsNone() {
        assert!(Real::from_i64(1).div(&Real::from_i64(0), 100).is_none());
    }

    #[test]
    fn Sqrt_GivenPerfectSquare_IsExactInteger() {
        assert_eq!(Real::from_i64(4).sqrt(20).to_string(), "2");
        assert_eq!(Real::from_i64(25).sqrt(20).to_string(), "5");
    }

    #[test]
    fn Sqrt_GivenTwo_MatchesDocumentedDigits() {
        assert_eq!(
            Real::from_i64(2).sqrt(20).to_string(),
            "1.4142135623730950488"
        );
    }

    #[test]
    fn Pi_GivenFive_MatchesDocumentedDigits() {
        assert_eq!(Real::pi(5).to_string(), "3.14159");
    }

    #[test]
    fn Pi_GivenFifty_TruncatesRatherThanRounds() {
        assert_eq!(
            Real::pi(50).to_string(),
            "3.14159265358979323846264338327950288419716939937510"
        );
    }

    #[test]
    fn FromStr_GivenPeriodNotation_RoundTrips() {
        assert_eq!(r("0.(3)").to_string(), "0.(3)");
        assert_eq!(r("0.1(6)").to_string(), "0.1(6)");
        assert_eq!(r("-2").to_string(), "-2");
    }

    #[test]
    fn to_string_GivenFortyTwo_ReturnsFortyTwo() {
        // RealToStringTests.ToString_GivenIntegerExponent: new Real("42") renders "42".
        assert_eq!(r("42").to_string(), "42");
    }

    #[test]
    fn to_string_GivenThreePointFourteen_ReturnsThreePointFourteen() {
        // RealToStringTests.ToString_GivenNegativeExponent: new Real("3.14") renders "3.14".
        assert_eq!(r("3.14").to_string(), "3.14");
    }

    #[test]
    fn to_string_GivenZeroPointZeroZeroZeroFive_ReturnsSame() {
        // RealToStringTests.ToString_GivenExponentLargerThanDigitCount:
        // new Real("0.0005") renders "0.0005".
        assert_eq!(r("0.0005").to_string(), "0.0005");
    }

    #[test]
    fn to_string_GivenNegativeOnePointFive_ReturnsMinusOnePointFive() {
        // RealToStringTests.ToString_GivenNegativeValue: new Real("-1.5") renders "-1.5".
        assert_eq!(r("-1.5").to_string(), "-1.5");
    }

    #[test]
    fn FromStr_GivenGarbage_ReturnsNone() {
        assert!(Real::from_str("").is_none());
        assert!(Real::from_str("abc").is_none());
        assert!(Real::from_str("1.(2").is_none());
    }

    #[test]
    fn Cmp_GivenDifferentScales_OrdersNumerically() {
        assert!(r("0.1").cmp(&r("0.09")) == Ordering::Greater);
        assert!(r("-1").cmp(&r("0")) == Ordering::Less);
    }

    #[test]
    fn IsInteger_GivenTrailingZeros_IsTrue() {
        assert!(r("2.000").is_integer());
        assert!(!r("2.5").is_integer());
        assert_eq!(r("2.500").to_int_exact(), None);
    }

    #[test]
    fn PowInt_GivenCubed_FollowsMultiplication() {
        assert_eq!(r("1.5").pow_int(3).unwrap().to_string(), "3.375");
    }

    #[test]
    fn Mul_GivenPeriodicAndItsDenominator_IsExactInteger() {
        let third = r("0.(3)");
        assert_eq!(third.mul(&Real::from_i64(3)).to_string(), "1");
    }
}

#[cfg(test)]
mod real_parity_round7;

#[cfg(test)]
mod real_parity_round10;

#[cfg(test)]
mod transcendental_round25;

#[cfg(test)]
mod real_parity_batch {
    #![allow(non_snake_case)]
    use super::*;

    fn r(s: &str) -> Real {
        Real::from_str(s).unwrap()
    }

    fn is_negative(v: &Real) -> bool {
        v.cmp(&Real::zero()) == Ordering::Less
    }

    fn is_positive(v: &Real) -> bool {
        v.cmp(&Real::zero()) == Ordering::Greater
    }

    fn is_periodic(v: &Real) -> bool {
        v.to_string().contains('(')
    }

    fn frac_digits(v: &Real) -> usize {
        match v.to_string().split_once('.') {
            Some((_, f)) => f.chars().filter(|c| c.is_ascii_digit()).count(),
            None => 0,
        }
    }

    fn gcd(mut a: Nat, mut b: Nat) -> Nat {
        while !b.is_zero() {
            let (_, rem) = a.div_rem(&b).unwrap();
            a = b;
            b = rem;
        }
        a
    }

    fn lowest_terms(num: Int, den: Nat) -> (Int, Nat) {
        if num.is_zero() {
            return (Int::zero(), Nat::from_u64(1));
        }
        let g = gcd(num.magnitude().clone(), den.clone());
        if g.is_one() {
            return (num, den);
        }
        let (n, _) = num.div_rem(&g).unwrap();
        let (d, _) = den.div_rem(&g).unwrap();
        (n, d)
    }

    fn check_add_row(p: i64, q: i64, r: i64, s: i64) {
        let left = Real::from_i64(p).div(&Real::from_i64(q), 1000).unwrap();
        let right = Real::from_i64(r).div(&Real::from_i64(s), 1000).unwrap();
        assert!(
            is_periodic(&left),
            "corpus defect: {p}/{q} is not periodic (renders {})",
            left
        );
        let sum = left.add(&right);
        let (en, ed) = lowest_terms(Int::from_i64(p * s + r * q), Nat::from_u64((q * s) as u64));
        let (an, ad) = sum.to_rational();
        assert!(
            an == en && ad == ed,
            "({p}/{q}) + ({r}/{s}) denotes {}/{} ({}), expected {}/{}",
            an.to_decimal(),
            ad.to_decimal(),
            sum,
            en.to_decimal(),
            ed.to_decimal()
        );
        let expected = fraction_real(en, ed, 1000);
        assert!(
            sum == expected,
            "({p}/{q}) + ({r}/{s}) = {}, expected {}",
            sum,
            expected
        );
    }

    fn check_sub_row(p: i64, q: i64, r: i64, s: i64) {
        let left = Real::from_i64(p).div(&Real::from_i64(q), 1000).unwrap();
        let right = Real::from_i64(r).div(&Real::from_i64(s), 1000).unwrap();
        assert!(
            is_periodic(&left),
            "corpus defect: {p}/{q} is not periodic (renders {})",
            left
        );
        let difference = left.sub(&right);
        let (en, ed) = lowest_terms(Int::from_i64(p * s - r * q), Nat::from_u64((q * s) as u64));
        let (an, ad) = difference.to_rational();
        assert!(
            an == en && ad == ed,
            "({p}/{q}) - ({r}/{s}) denotes {}/{} ({}), expected {}/{}",
            an.to_decimal(),
            ad.to_decimal(),
            difference,
            en.to_decimal(),
            ed.to_decimal()
        );
        let expected = fraction_real(en, ed, 1000);
        assert!(
            difference == expected,
            "({p}/{q}) - ({r}/{s}) = {}, expected {}",
            difference,
            expected
        );
    }

    fn check_mul_row(p: i64, divisor: &str) {
        let q = r(divisor);
        let quotient = Real::from_i64(p).div(&q, 1000).unwrap();
        let product = quotient.mul(&q);
        assert!(
            product == Real::from_i64(p),
            "p={p}, q={divisor}: (p/q)*q = {}, expected {p}",
            product
        );
        let (n, d) = product.to_rational();
        assert!(
            n == Int::from_i64(p) && d == Nat::from_u64(1),
            "p={p}, q={divisor}: (p/q)*q denotes {}/{}",
            n.to_decimal(),
            d.to_decimal()
        );
    }

    #[test]
    fn add_p1_q3_r1_s3() {
        check_add_row(1, 3, 1, 3);
    }

    #[test]
    fn add_p1_q3_rm1_s3() {
        check_add_row(1, 3, -1, 3);
    }

    #[test]
    fn add_p1_q3_rm1_s99() {
        check_add_row(1, 3, -1, 99);
    }

    #[test]
    fn add_p2_q6_rm1_s7() {
        check_add_row(2, 6, -1, 7);
    }

    #[test]
    fn add_p2_q6_r5_s7() {
        check_add_row(2, 6, 5, 7);
    }

    #[test]
    fn add_p2_q6_r1_s17() {
        check_add_row(2, 6, 1, 17);
    }

    #[test]
    fn add_p2_q6_rm1_s17() {
        check_add_row(2, 6, -1, 17);
    }

    #[test]
    fn add_p2_q6_r5_s17() {
        check_add_row(2, 6, 5, 17);
    }

    #[test]
    fn add_p2_q6_r1_s99() {
        check_add_row(2, 6, 1, 99);
    }

    #[test]
    fn add_p2_q6_rm1_s99() {
        check_add_row(2, 6, -1, 99);
    }

    #[test]
    fn add_p2_q6_r5_s99() {
        check_add_row(2, 6, 5, 99);
    }

    #[test]
    fn add_p1_q3_r5_s99() {
        check_add_row(1, 3, 5, 99);
    }

    #[test]
    fn add_p1_q7_r1_s3() {
        check_add_row(1, 7, 1, 3);
    }

    #[test]
    fn add_p1_q7_rm1_s3() {
        check_add_row(1, 7, -1, 3);
    }

    #[test]
    fn add_p1_q7_r5_s3() {
        check_add_row(1, 7, 5, 3);
    }

    #[test]
    fn add_p1_q7_r1_s7() {
        check_add_row(1, 7, 1, 7);
    }

    #[test]
    fn add_p1_q7_rm1_s7() {
        check_add_row(1, 7, -1, 7);
    }

    #[test]
    fn add_p1_q7_r5_s7() {
        check_add_row(1, 7, 5, 7);
    }

    #[test]
    fn add_p1_q7_r1_s17() {
        check_add_row(1, 7, 1, 17);
    }

    #[test]
    fn add_p1_q7_rm1_s17() {
        check_add_row(1, 7, -1, 17);
    }

    #[test]
    fn add_p1_q7_r5_s17() {
        check_add_row(1, 7, 5, 17);
    }

    #[test]
    fn add_p1_q7_r1_s99() {
        check_add_row(1, 7, 1, 99);
    }

    #[test]
    fn add_p1_q7_rm1_s99() {
        check_add_row(1, 7, -1, 99);
    }

    #[test]
    fn add_p1_q7_r5_s99() {
        check_add_row(1, 7, 5, 99);
    }

    #[test]
    fn add_p2_q7_r1_s3() {
        check_add_row(2, 7, 1, 3);
    }

    #[test]
    fn add_p2_q7_rm1_s3() {
        check_add_row(2, 7, -1, 3);
    }

    #[test]
    fn add_p2_q7_r5_s3() {
        check_add_row(2, 7, 5, 3);
    }

    #[test]
    fn add_p2_q7_r1_s7() {
        check_add_row(2, 7, 1, 7);
    }

    #[test]
    fn add_p2_q7_rm1_s7() {
        check_add_row(2, 7, -1, 7);
    }

    #[test]
    fn add_p2_q7_r5_s7() {
        check_add_row(2, 7, 5, 7);
    }

    #[test]
    fn add_p2_q7_r1_s17() {
        check_add_row(2, 7, 1, 17);
    }

    #[test]
    fn add_p2_q7_rm1_s17() {
        check_add_row(2, 7, -1, 17);
    }

    #[test]
    fn add_p2_q7_r5_s17() {
        check_add_row(2, 7, 5, 17);
    }

    #[test]
    fn add_p2_q7_r1_s99() {
        check_add_row(2, 7, 1, 99);
    }

    #[test]
    fn add_p2_q7_rm1_s99() {
        check_add_row(2, 7, -1, 99);
    }

    #[test]
    fn add_p2_q7_r5_s99() {
        check_add_row(2, 7, 5, 99);
    }

    #[test]
    fn add_pm5_q7_r1_s3() {
        check_add_row(-5, 7, 1, 3);
    }

    #[test]
    fn add_pm5_q7_rm1_s3() {
        check_add_row(-5, 7, -1, 3);
    }

    #[test]
    fn add_pm5_q7_r5_s3() {
        check_add_row(-5, 7, 5, 3);
    }

    #[test]
    fn add_pm5_q7_r1_s7() {
        check_add_row(-5, 7, 1, 7);
    }

    #[test]
    fn add_pm5_q7_rm1_s7() {
        check_add_row(-5, 7, -1, 7);
    }

    #[test]
    fn add_pm5_q7_r5_s7() {
        check_add_row(-5, 7, 5, 7);
    }

    #[test]
    fn add_pm5_q7_r1_s17() {
        check_add_row(-5, 7, 1, 17);
    }

    #[test]
    fn add_pm5_q7_rm1_s17() {
        check_add_row(-5, 7, -1, 17);
    }

    #[test]
    fn add_p1_q3_r5_s3() {
        check_add_row(1, 3, 5, 3);
    }

    #[test]
    fn add_pm5_q7_r5_s17() {
        check_add_row(-5, 7, 5, 17);
    }

    #[test]
    fn add_pm5_q7_r1_s99() {
        check_add_row(-5, 7, 1, 99);
    }

    #[test]
    fn add_pm5_q7_rm1_s99() {
        check_add_row(-5, 7, -1, 99);
    }

    #[test]
    fn add_pm5_q7_r5_s99() {
        check_add_row(-5, 7, 5, 99);
    }

    #[test]
    fn add_p1_q9_r1_s3() {
        check_add_row(1, 9, 1, 3);
    }

    #[test]
    fn add_p1_q9_rm1_s3() {
        check_add_row(1, 9, -1, 3);
    }

    #[test]
    fn add_p1_q9_r5_s3() {
        check_add_row(1, 9, 5, 3);
    }

    #[test]
    fn add_p1_q9_r1_s7() {
        check_add_row(1, 9, 1, 7);
    }

    #[test]
    fn add_p1_q9_rm1_s7() {
        check_add_row(1, 9, -1, 7);
    }

    #[test]
    fn add_p1_q9_r5_s7() {
        check_add_row(1, 9, 5, 7);
    }

    #[test]
    fn add_p1_q9_r1_s17() {
        check_add_row(1, 9, 1, 17);
    }

    #[test]
    fn add_p1_q9_rm1_s17() {
        check_add_row(1, 9, -1, 17);
    }

    #[test]
    fn add_p1_q9_r5_s17() {
        check_add_row(1, 9, 5, 17);
    }

    #[test]
    fn add_p1_q9_r1_s99() {
        check_add_row(1, 9, 1, 99);
    }

    #[test]
    fn add_p1_q9_rm1_s99() {
        check_add_row(1, 9, -1, 99);
    }

    #[test]
    fn add_p1_q9_r5_s99() {
        check_add_row(1, 9, 5, 99);
    }

    #[test]
    fn add_p2_q3_r1_s3() {
        check_add_row(2, 3, 1, 3);
    }

    #[test]
    fn add_p2_q9_r1_s3() {
        check_add_row(2, 9, 1, 3);
    }

    #[test]
    fn add_p2_q9_rm1_s3() {
        check_add_row(2, 9, -1, 3);
    }

    #[test]
    fn add_p2_q9_r5_s3() {
        check_add_row(2, 9, 5, 3);
    }

    #[test]
    fn add_p2_q9_r1_s7() {
        check_add_row(2, 9, 1, 7);
    }

    #[test]
    fn add_p2_q9_rm1_s7() {
        check_add_row(2, 9, -1, 7);
    }

    #[test]
    fn add_p2_q9_r5_s7() {
        check_add_row(2, 9, 5, 7);
    }

    #[test]
    fn add_p2_q9_r1_s17() {
        check_add_row(2, 9, 1, 17);
    }

    #[test]
    fn add_p2_q9_rm1_s17() {
        check_add_row(2, 9, -1, 17);
    }

    #[test]
    fn add_p2_q9_r5_s17() {
        check_add_row(2, 9, 5, 17);
    }

    #[test]
    fn add_p2_q9_r1_s99() {
        check_add_row(2, 9, 1, 99);
    }

    #[test]
    fn add_p2_q3_rm1_s3() {
        check_add_row(2, 3, -1, 3);
    }

    #[test]
    fn add_p2_q9_rm1_s99() {
        check_add_row(2, 9, -1, 99);
    }

    #[test]
    fn add_p2_q9_r5_s99() {
        check_add_row(2, 9, 5, 99);
    }

    #[test]
    fn add_p2_q3_r5_s3() {
        check_add_row(2, 3, 5, 3);
    }

    #[test]
    fn add_pm5_q9_r1_s3() {
        check_add_row(-5, 9, 1, 3);
    }

    #[test]
    fn add_pm5_q9_rm1_s3() {
        check_add_row(-5, 9, -1, 3);
    }

    #[test]
    fn add_pm5_q9_r5_s3() {
        check_add_row(-5, 9, 5, 3);
    }

    #[test]
    fn add_pm5_q9_r1_s7() {
        check_add_row(-5, 9, 1, 7);
    }

    #[test]
    fn add_pm5_q9_rm1_s7() {
        check_add_row(-5, 9, -1, 7);
    }

    #[test]
    fn add_pm5_q9_r5_s7() {
        check_add_row(-5, 9, 5, 7);
    }

    #[test]
    fn add_p2_q3_r1_s7() {
        check_add_row(2, 3, 1, 7);
    }

    #[test]
    fn add_pm5_q9_r1_s17() {
        check_add_row(-5, 9, 1, 17);
    }

    #[test]
    fn add_pm5_q9_rm1_s17() {
        check_add_row(-5, 9, -1, 17);
    }

    #[test]
    fn add_pm5_q9_r5_s17() {
        check_add_row(-5, 9, 5, 17);
    }

    #[test]
    fn add_pm5_q9_r1_s99() {
        check_add_row(-5, 9, 1, 99);
    }

    #[test]
    fn add_pm5_q9_rm1_s99() {
        check_add_row(-5, 9, -1, 99);
    }

    #[test]
    fn add_pm5_q9_r5_s99() {
        check_add_row(-5, 9, 5, 99);
    }

    #[test]
    fn add_p2_q3_rm1_s7() {
        check_add_row(2, 3, -1, 7);
    }

    #[test]
    fn add_p1_q11_r1_s3() {
        check_add_row(1, 11, 1, 3);
    }

    #[test]
    fn add_p1_q11_rm1_s3() {
        check_add_row(1, 11, -1, 3);
    }

    #[test]
    fn add_p2_q3_r5_s7() {
        check_add_row(2, 3, 5, 7);
    }

    #[test]
    fn add_p1_q11_r5_s3() {
        check_add_row(1, 11, 5, 3);
    }

    #[test]
    fn add_p1_q11_r1_s7() {
        check_add_row(1, 11, 1, 7);
    }

    #[test]
    fn add_p1_q11_rm1_s7() {
        check_add_row(1, 11, -1, 7);
    }

    #[test]
    fn add_p1_q11_r5_s7() {
        check_add_row(1, 11, 5, 7);
    }

    #[test]
    fn add_p1_q11_r1_s17() {
        check_add_row(1, 11, 1, 17);
    }

    #[test]
    fn add_p1_q11_rm1_s17() {
        check_add_row(1, 11, -1, 17);
    }

    #[test]
    fn add_p1_q11_r5_s17() {
        check_add_row(1, 11, 5, 17);
    }

    #[test]
    fn add_p1_q11_r1_s99() {
        check_add_row(1, 11, 1, 99);
    }

    #[test]
    fn add_p1_q11_rm1_s99() {
        check_add_row(1, 11, -1, 99);
    }

    #[test]
    fn add_p1_q11_r5_s99() {
        check_add_row(1, 11, 5, 99);
    }

    #[test]
    fn add_p1_q3_r1_s7() {
        check_add_row(1, 3, 1, 7);
    }

    #[test]
    fn add_p2_q3_r1_s17() {
        check_add_row(2, 3, 1, 17);
    }

    #[test]
    fn add_p2_q3_rm1_s17() {
        check_add_row(2, 3, -1, 17);
    }

    #[test]
    fn add_p2_q11_r1_s3() {
        check_add_row(2, 11, 1, 3);
    }

    #[test]
    fn add_p2_q11_rm1_s3() {
        check_add_row(2, 11, -1, 3);
    }

    #[test]
    fn add_p2_q11_r5_s3() {
        check_add_row(2, 11, 5, 3);
    }

    #[test]
    fn add_p2_q11_r1_s7() {
        check_add_row(2, 11, 1, 7);
    }

    #[test]
    fn add_p2_q11_rm1_s7() {
        check_add_row(2, 11, -1, 7);
    }

    #[test]
    fn add_p2_q11_r5_s7() {
        check_add_row(2, 11, 5, 7);
    }

    #[test]
    fn add_p2_q11_r1_s17() {
        check_add_row(2, 11, 1, 17);
    }

    #[test]
    fn add_p2_q11_rm1_s17() {
        check_add_row(2, 11, -1, 17);
    }

    #[test]
    fn add_p2_q3_r5_s17() {
        check_add_row(2, 3, 5, 17);
    }

    #[test]
    fn add_p2_q11_r5_s17() {
        check_add_row(2, 11, 5, 17);
    }

    #[test]
    fn add_p2_q11_r1_s99() {
        check_add_row(2, 11, 1, 99);
    }

    #[test]
    fn add_p2_q11_rm1_s99() {
        check_add_row(2, 11, -1, 99);
    }

    #[test]
    fn add_p2_q11_r5_s99() {
        check_add_row(2, 11, 5, 99);
    }

    #[test]
    fn add_p2_q3_r1_s99() {
        check_add_row(2, 3, 1, 99);
    }

    #[test]
    fn add_pm5_q11_r1_s3() {
        check_add_row(-5, 11, 1, 3);
    }

    #[test]
    fn add_pm5_q11_rm1_s3() {
        check_add_row(-5, 11, -1, 3);
    }

    #[test]
    fn add_pm5_q11_r5_s3() {
        check_add_row(-5, 11, 5, 3);
    }

    #[test]
    fn add_pm5_q11_r1_s7() {
        check_add_row(-5, 11, 1, 7);
    }

    #[test]
    fn add_p2_q3_rm1_s99() {
        check_add_row(2, 3, -1, 99);
    }

    #[test]
    fn add_pm5_q11_rm1_s7() {
        check_add_row(-5, 11, -1, 7);
    }

    #[test]
    fn add_pm5_q11_r5_s7() {
        check_add_row(-5, 11, 5, 7);
    }

    #[test]
    fn add_pm5_q11_r1_s17() {
        check_add_row(-5, 11, 1, 17);
    }

    #[test]
    fn add_pm5_q11_rm1_s17() {
        check_add_row(-5, 11, -1, 17);
    }

    #[test]
    fn add_pm5_q11_r5_s17() {
        check_add_row(-5, 11, 5, 17);
    }

    #[test]
    fn add_pm5_q11_r1_s99() {
        check_add_row(-5, 11, 1, 99);
    }

    #[test]
    fn add_pm5_q11_rm1_s99() {
        check_add_row(-5, 11, -1, 99);
    }

    #[test]
    fn add_pm5_q11_r5_s99() {
        check_add_row(-5, 11, 5, 99);
    }

    #[test]
    fn add_p2_q3_r5_s99() {
        check_add_row(2, 3, 5, 99);
    }

    #[test]
    fn add_p1_q3_rm1_s7() {
        check_add_row(1, 3, -1, 7);
    }

    #[test]
    fn add_p1_q13_r1_s3() {
        check_add_row(1, 13, 1, 3);
    }

    #[test]
    fn add_p1_q13_rm1_s3() {
        check_add_row(1, 13, -1, 3);
    }

    #[test]
    fn add_p1_q13_r5_s3() {
        check_add_row(1, 13, 5, 3);
    }

    #[test]
    fn add_p1_q13_r1_s7() {
        check_add_row(1, 13, 1, 7);
    }

    #[test]
    fn add_p1_q13_rm1_s7() {
        check_add_row(1, 13, -1, 7);
    }

    #[test]
    fn add_p1_q13_r5_s7() {
        check_add_row(1, 13, 5, 7);
    }

    #[test]
    fn add_p1_q13_r1_s17() {
        check_add_row(1, 13, 1, 17);
    }

    #[test]
    fn add_p1_q13_rm1_s17() {
        check_add_row(1, 13, -1, 17);
    }

    #[test]
    fn add_p1_q13_r5_s17() {
        check_add_row(1, 13, 5, 17);
    }

    #[test]
    fn add_p1_q13_r1_s99() {
        check_add_row(1, 13, 1, 99);
    }

    #[test]
    fn add_p1_q13_rm1_s99() {
        check_add_row(1, 13, -1, 99);
    }

    #[test]
    fn add_p1_q13_r5_s99() {
        check_add_row(1, 13, 5, 99);
    }

    #[test]
    fn add_p2_q13_r1_s3() {
        check_add_row(2, 13, 1, 3);
    }

    #[test]
    fn add_p2_q13_rm1_s3() {
        check_add_row(2, 13, -1, 3);
    }

    #[test]
    fn add_p2_q13_r5_s3() {
        check_add_row(2, 13, 5, 3);
    }

    #[test]
    fn add_p2_q13_r1_s7() {
        check_add_row(2, 13, 1, 7);
    }

    #[test]
    fn add_p2_q13_rm1_s7() {
        check_add_row(2, 13, -1, 7);
    }

    #[test]
    fn add_p2_q13_r5_s7() {
        check_add_row(2, 13, 5, 7);
    }

    #[test]
    fn add_p2_q13_r1_s17() {
        check_add_row(2, 13, 1, 17);
    }

    #[test]
    fn add_p2_q13_rm1_s17() {
        check_add_row(2, 13, -1, 17);
    }

    #[test]
    fn add_p2_q13_r5_s17() {
        check_add_row(2, 13, 5, 17);
    }

    #[test]
    fn add_p2_q13_r1_s99() {
        check_add_row(2, 13, 1, 99);
    }

    #[test]
    fn add_p2_q13_rm1_s99() {
        check_add_row(2, 13, -1, 99);
    }

    #[test]
    fn add_p2_q13_r5_s99() {
        check_add_row(2, 13, 5, 99);
    }

    #[test]
    fn add_pm5_q3_r1_s3() {
        check_add_row(-5, 3, 1, 3);
    }

    #[test]
    fn add_pm5_q13_r1_s3() {
        check_add_row(-5, 13, 1, 3);
    }

    #[test]
    fn add_pm5_q13_rm1_s3() {
        check_add_row(-5, 13, -1, 3);
    }

    #[test]
    fn add_pm5_q13_r5_s3() {
        check_add_row(-5, 13, 5, 3);
    }

    #[test]
    fn add_pm5_q13_r1_s7() {
        check_add_row(-5, 13, 1, 7);
    }

    #[test]
    fn add_pm5_q13_rm1_s7() {
        check_add_row(-5, 13, -1, 7);
    }

    #[test]
    fn add_pm5_q13_r5_s7() {
        check_add_row(-5, 13, 5, 7);
    }

    #[test]
    fn add_pm5_q13_r1_s17() {
        check_add_row(-5, 13, 1, 17);
    }

    #[test]
    fn add_pm5_q13_rm1_s17() {
        check_add_row(-5, 13, -1, 17);
    }

    #[test]
    fn add_pm5_q13_r5_s17() {
        check_add_row(-5, 13, 5, 17);
    }

    #[test]
    fn add_pm5_q13_r1_s99() {
        check_add_row(-5, 13, 1, 99);
    }

    #[test]
    fn add_pm5_q3_rm1_s3() {
        check_add_row(-5, 3, -1, 3);
    }

    #[test]
    fn add_pm5_q13_rm1_s99() {
        check_add_row(-5, 13, -1, 99);
    }

    #[test]
    fn add_pm5_q13_r5_s99() {
        check_add_row(-5, 13, 5, 99);
    }

    #[test]
    fn add_p1_q3_r5_s7() {
        check_add_row(1, 3, 5, 7);
    }

    #[test]
    fn add_pm5_q3_r5_s3() {
        check_add_row(-5, 3, 5, 3);
    }

    #[test]
    fn add_p1_q17_r1_s3() {
        check_add_row(1, 17, 1, 3);
    }

    #[test]
    fn add_p1_q17_rm1_s3() {
        check_add_row(1, 17, -1, 3);
    }

    #[test]
    fn add_p1_q17_r5_s3() {
        check_add_row(1, 17, 5, 3);
    }

    #[test]
    fn add_p1_q17_r1_s7() {
        check_add_row(1, 17, 1, 7);
    }

    #[test]
    fn add_p1_q17_rm1_s7() {
        check_add_row(1, 17, -1, 7);
    }

    #[test]
    fn add_p1_q17_r5_s7() {
        check_add_row(1, 17, 5, 7);
    }

    #[test]
    fn add_pm5_q3_r1_s7() {
        check_add_row(-5, 3, 1, 7);
    }

    #[test]
    fn add_p1_q17_r1_s17() {
        check_add_row(1, 17, 1, 17);
    }

    #[test]
    fn add_p1_q17_rm1_s17() {
        check_add_row(1, 17, -1, 17);
    }

    #[test]
    fn add_p1_q17_r5_s17() {
        check_add_row(1, 17, 5, 17);
    }

    #[test]
    fn add_p1_q17_r1_s99() {
        check_add_row(1, 17, 1, 99);
    }

    #[test]
    fn add_p1_q17_rm1_s99() {
        check_add_row(1, 17, -1, 99);
    }

    #[test]
    fn add_p1_q17_r5_s99() {
        check_add_row(1, 17, 5, 99);
    }

    #[test]
    fn add_pm5_q3_rm1_s7() {
        check_add_row(-5, 3, -1, 7);
    }

    #[test]
    fn add_p2_q17_r1_s3() {
        check_add_row(2, 17, 1, 3);
    }

    #[test]
    fn add_p2_q17_rm1_s3() {
        check_add_row(2, 17, -1, 3);
    }

    #[test]
    fn add_pm5_q3_r5_s7() {
        check_add_row(-5, 3, 5, 7);
    }

    #[test]
    fn add_p2_q17_r5_s3() {
        check_add_row(2, 17, 5, 3);
    }

    #[test]
    fn add_p2_q17_r1_s7() {
        check_add_row(2, 17, 1, 7);
    }

    #[test]
    fn add_p2_q17_rm1_s7() {
        check_add_row(2, 17, -1, 7);
    }

    #[test]
    fn add_p2_q17_r5_s7() {
        check_add_row(2, 17, 5, 7);
    }

    #[test]
    fn add_p2_q17_r1_s17() {
        check_add_row(2, 17, 1, 17);
    }

    #[test]
    fn add_p2_q17_rm1_s17() {
        check_add_row(2, 17, -1, 17);
    }

    #[test]
    fn add_p2_q17_r5_s17() {
        check_add_row(2, 17, 5, 17);
    }

    #[test]
    fn add_p2_q17_r1_s99() {
        check_add_row(2, 17, 1, 99);
    }

    #[test]
    fn add_p2_q17_rm1_s99() {
        check_add_row(2, 17, -1, 99);
    }

    #[test]
    fn add_p2_q17_r5_s99() {
        check_add_row(2, 17, 5, 99);
    }

    #[test]
    fn add_pm5_q3_r1_s17() {
        check_add_row(-5, 3, 1, 17);
    }

    #[test]
    fn add_pm5_q3_rm1_s17() {
        check_add_row(-5, 3, -1, 17);
    }

    #[test]
    fn add_pm5_q17_r1_s3() {
        check_add_row(-5, 17, 1, 3);
    }

    #[test]
    fn add_pm5_q17_rm1_s3() {
        check_add_row(-5, 17, -1, 3);
    }

    #[test]
    fn add_pm5_q17_r5_s3() {
        check_add_row(-5, 17, 5, 3);
    }

    #[test]
    fn add_pm5_q17_r1_s7() {
        check_add_row(-5, 17, 1, 7);
    }

    #[test]
    fn add_pm5_q17_rm1_s7() {
        check_add_row(-5, 17, -1, 7);
    }

    #[test]
    fn add_pm5_q17_r5_s7() {
        check_add_row(-5, 17, 5, 7);
    }

    #[test]
    fn add_pm5_q17_r1_s17() {
        check_add_row(-5, 17, 1, 17);
    }

    #[test]
    fn add_pm5_q17_rm1_s17() {
        check_add_row(-5, 17, -1, 17);
    }

    #[test]
    fn add_pm5_q3_r5_s17() {
        check_add_row(-5, 3, 5, 17);
    }

    #[test]
    fn add_pm5_q17_r5_s17() {
        check_add_row(-5, 17, 5, 17);
    }

    #[test]
    fn add_pm5_q17_r1_s99() {
        check_add_row(-5, 17, 1, 99);
    }

    #[test]
    fn add_pm5_q17_rm1_s99() {
        check_add_row(-5, 17, -1, 99);
    }

    #[test]
    fn add_pm5_q17_r5_s99() {
        check_add_row(-5, 17, 5, 99);
    }

    #[test]
    fn add_pm5_q3_r1_s99() {
        check_add_row(-5, 3, 1, 99);
    }

    #[test]
    fn add_p1_q19_r1_s3() {
        check_add_row(1, 19, 1, 3);
    }

    #[test]
    fn add_p1_q19_rm1_s3() {
        check_add_row(1, 19, -1, 3);
    }

    #[test]
    fn add_p1_q19_r5_s3() {
        check_add_row(1, 19, 5, 3);
    }

    #[test]
    fn add_p1_q19_r1_s7() {
        check_add_row(1, 19, 1, 7);
    }

    #[test]
    fn add_pm5_q3_rm1_s99() {
        check_add_row(-5, 3, -1, 99);
    }

    #[test]
    fn add_p1_q19_rm1_s7() {
        check_add_row(1, 19, -1, 7);
    }

    #[test]
    fn add_p1_q19_r5_s7() {
        check_add_row(1, 19, 5, 7);
    }

    #[test]
    fn add_p1_q19_r1_s99() {
        check_add_row(1, 19, 1, 99);
    }

    #[test]
    fn add_p1_q19_rm1_s99() {
        check_add_row(1, 19, -1, 99);
    }

    #[test]
    fn add_p1_q19_r5_s99() {
        check_add_row(1, 19, 5, 99);
    }

    #[test]
    fn add_pm5_q3_r5_s99() {
        check_add_row(-5, 3, 5, 99);
    }

    #[test]
    fn add_p1_q3_r1_s17() {
        check_add_row(1, 3, 1, 17);
    }

    #[test]
    fn add_p2_q19_r1_s3() {
        check_add_row(2, 19, 1, 3);
    }

    #[test]
    fn add_p2_q19_rm1_s3() {
        check_add_row(2, 19, -1, 3);
    }

    #[test]
    fn add_p2_q19_r5_s3() {
        check_add_row(2, 19, 5, 3);
    }

    #[test]
    fn add_p2_q19_r1_s7() {
        check_add_row(2, 19, 1, 7);
    }

    #[test]
    fn add_p2_q19_rm1_s7() {
        check_add_row(2, 19, -1, 7);
    }

    #[test]
    fn add_p2_q19_r5_s7() {
        check_add_row(2, 19, 5, 7);
    }

    #[test]
    fn add_p2_q19_r1_s99() {
        check_add_row(2, 19, 1, 99);
    }

    #[test]
    fn add_p2_q19_rm1_s99() {
        check_add_row(2, 19, -1, 99);
    }

    #[test]
    fn add_p2_q19_r5_s99() {
        check_add_row(2, 19, 5, 99);
    }

    #[test]
    fn add_pm5_q19_r1_s3() {
        check_add_row(-5, 19, 1, 3);
    }

    #[test]
    fn add_pm5_q19_rm1_s3() {
        check_add_row(-5, 19, -1, 3);
    }

    #[test]
    fn add_pm5_q19_r5_s3() {
        check_add_row(-5, 19, 5, 3);
    }

    #[test]
    fn add_pm5_q19_r1_s7() {
        check_add_row(-5, 19, 1, 7);
    }

    #[test]
    fn add_pm5_q19_rm1_s7() {
        check_add_row(-5, 19, -1, 7);
    }

    #[test]
    fn add_pm5_q19_r5_s7() {
        check_add_row(-5, 19, 5, 7);
    }

    #[test]
    fn add_pm5_q19_r1_s99() {
        check_add_row(-5, 19, 1, 99);
    }

    #[test]
    fn add_pm5_q19_rm1_s99() {
        check_add_row(-5, 19, -1, 99);
    }

    #[test]
    fn add_pm5_q19_r5_s99() {
        check_add_row(-5, 19, 5, 99);
    }

    #[test]
    fn add_p1_q21_r1_s3() {
        check_add_row(1, 21, 1, 3);
    }

    #[test]
    fn add_p1_q21_rm1_s3() {
        check_add_row(1, 21, -1, 3);
    }

    #[test]
    fn add_p1_q21_r5_s3() {
        check_add_row(1, 21, 5, 3);
    }

    #[test]
    fn add_p1_q21_r1_s7() {
        check_add_row(1, 21, 1, 7);
    }

    #[test]
    fn add_p1_q21_rm1_s7() {
        check_add_row(1, 21, -1, 7);
    }

    #[test]
    fn add_p1_q21_r5_s7() {
        check_add_row(1, 21, 5, 7);
    }

    #[test]
    fn add_p1_q21_r1_s17() {
        check_add_row(1, 21, 1, 17);
    }

    #[test]
    fn add_p1_q21_rm1_s17() {
        check_add_row(1, 21, -1, 17);
    }

    #[test]
    fn add_p1_q21_r5_s17() {
        check_add_row(1, 21, 5, 17);
    }

    #[test]
    fn add_p1_q21_r1_s99() {
        check_add_row(1, 21, 1, 99);
    }

    #[test]
    fn add_p1_q21_rm1_s99() {
        check_add_row(1, 21, -1, 99);
    }

    #[test]
    fn add_p1_q21_r5_s99() {
        check_add_row(1, 21, 5, 99);
    }

    #[test]
    fn add_p2_q21_r1_s3() {
        check_add_row(2, 21, 1, 3);
    }

    #[test]
    fn add_p2_q21_rm1_s3() {
        check_add_row(2, 21, -1, 3);
    }

    #[test]
    fn add_p2_q21_r5_s3() {
        check_add_row(2, 21, 5, 3);
    }

    #[test]
    fn add_p2_q21_r1_s7() {
        check_add_row(2, 21, 1, 7);
    }

    #[test]
    fn add_p2_q21_rm1_s7() {
        check_add_row(2, 21, -1, 7);
    }

    #[test]
    fn add_p2_q21_r5_s7() {
        check_add_row(2, 21, 5, 7);
    }

    #[test]
    fn add_p2_q21_r1_s17() {
        check_add_row(2, 21, 1, 17);
    }

    #[test]
    fn add_p2_q21_rm1_s17() {
        check_add_row(2, 21, -1, 17);
    }

    #[test]
    fn add_p2_q21_r5_s17() {
        check_add_row(2, 21, 5, 17);
    }

    #[test]
    fn add_p2_q21_r1_s99() {
        check_add_row(2, 21, 1, 99);
    }

    #[test]
    fn add_p2_q21_rm1_s99() {
        check_add_row(2, 21, -1, 99);
    }

    #[test]
    fn add_p2_q21_r5_s99() {
        check_add_row(2, 21, 5, 99);
    }

    #[test]
    fn add_pm5_q21_r1_s3() {
        check_add_row(-5, 21, 1, 3);
    }

    #[test]
    fn add_pm5_q21_rm1_s3() {
        check_add_row(-5, 21, -1, 3);
    }

    #[test]
    fn add_pm5_q21_r5_s3() {
        check_add_row(-5, 21, 5, 3);
    }

    #[test]
    fn add_pm5_q21_r1_s7() {
        check_add_row(-5, 21, 1, 7);
    }

    #[test]
    fn add_p1_q3_rm1_s17() {
        check_add_row(1, 3, -1, 17);
    }

    #[test]
    fn add_pm5_q21_rm1_s7() {
        check_add_row(-5, 21, -1, 7);
    }

    #[test]
    fn add_pm5_q21_r5_s7() {
        check_add_row(-5, 21, 5, 7);
    }

    #[test]
    fn add_pm5_q21_r1_s17() {
        check_add_row(-5, 21, 1, 17);
    }

    #[test]
    fn add_pm5_q21_rm1_s17() {
        check_add_row(-5, 21, -1, 17);
    }

    #[test]
    fn add_pm5_q21_r5_s17() {
        check_add_row(-5, 21, 5, 17);
    }

    #[test]
    fn add_pm5_q21_r1_s99() {
        check_add_row(-5, 21, 1, 99);
    }

    #[test]
    fn add_pm5_q21_rm1_s99() {
        check_add_row(-5, 21, -1, 99);
    }

    #[test]
    fn add_pm5_q21_r5_s99() {
        check_add_row(-5, 21, 5, 99);
    }

    #[test]
    fn add_p1_q37_r1_s3() {
        check_add_row(1, 37, 1, 3);
    }

    #[test]
    fn add_p1_q37_rm1_s3() {
        check_add_row(1, 37, -1, 3);
    }

    #[test]
    fn add_p1_q37_r5_s3() {
        check_add_row(1, 37, 5, 3);
    }

    #[test]
    fn add_p1_q37_r1_s7() {
        check_add_row(1, 37, 1, 7);
    }

    #[test]
    fn add_p1_q37_rm1_s7() {
        check_add_row(1, 37, -1, 7);
    }

    #[test]
    fn add_p1_q37_r5_s7() {
        check_add_row(1, 37, 5, 7);
    }

    #[test]
    fn add_p1_q37_r1_s17() {
        check_add_row(1, 37, 1, 17);
    }

    #[test]
    fn add_p1_q37_rm1_s17() {
        check_add_row(1, 37, -1, 17);
    }

    #[test]
    fn add_p1_q37_r5_s17() {
        check_add_row(1, 37, 5, 17);
    }

    #[test]
    fn add_p1_q37_r1_s99() {
        check_add_row(1, 37, 1, 99);
    }

    #[test]
    fn add_p1_q37_rm1_s99() {
        check_add_row(1, 37, -1, 99);
    }

    #[test]
    fn add_p1_q37_r5_s99() {
        check_add_row(1, 37, 5, 99);
    }

    #[test]
    fn add_p2_q37_r1_s3() {
        check_add_row(2, 37, 1, 3);
    }

    #[test]
    fn add_p2_q37_rm1_s3() {
        check_add_row(2, 37, -1, 3);
    }

    #[test]
    fn add_p2_q37_r5_s3() {
        check_add_row(2, 37, 5, 3);
    }

    #[test]
    fn add_p2_q37_r1_s7() {
        check_add_row(2, 37, 1, 7);
    }

    #[test]
    fn add_p2_q37_rm1_s7() {
        check_add_row(2, 37, -1, 7);
    }

    #[test]
    fn add_p2_q37_r5_s7() {
        check_add_row(2, 37, 5, 7);
    }

    #[test]
    fn add_p2_q37_r1_s17() {
        check_add_row(2, 37, 1, 17);
    }

    #[test]
    fn add_p2_q37_rm1_s17() {
        check_add_row(2, 37, -1, 17);
    }

    #[test]
    fn add_p2_q37_r5_s17() {
        check_add_row(2, 37, 5, 17);
    }

    #[test]
    fn add_p2_q37_r1_s99() {
        check_add_row(2, 37, 1, 99);
    }

    #[test]
    fn add_p2_q37_rm1_s99() {
        check_add_row(2, 37, -1, 99);
    }

    #[test]
    fn add_p2_q37_r5_s99() {
        check_add_row(2, 37, 5, 99);
    }

    #[test]
    fn add_pm5_q37_r1_s3() {
        check_add_row(-5, 37, 1, 3);
    }

    #[test]
    fn add_pm5_q37_rm1_s3() {
        check_add_row(-5, 37, -1, 3);
    }

    #[test]
    fn add_pm5_q37_r5_s3() {
        check_add_row(-5, 37, 5, 3);
    }

    #[test]
    fn add_pm5_q37_r1_s7() {
        check_add_row(-5, 37, 1, 7);
    }

    #[test]
    fn add_pm5_q37_rm1_s7() {
        check_add_row(-5, 37, -1, 7);
    }

    #[test]
    fn add_pm5_q37_r5_s7() {
        check_add_row(-5, 37, 5, 7);
    }

    #[test]
    fn add_pm5_q37_r1_s17() {
        check_add_row(-5, 37, 1, 17);
    }

    #[test]
    fn add_pm5_q37_rm1_s17() {
        check_add_row(-5, 37, -1, 17);
    }

    #[test]
    fn add_pm5_q37_r5_s17() {
        check_add_row(-5, 37, 5, 17);
    }

    #[test]
    fn add_pm5_q37_r1_s99() {
        check_add_row(-5, 37, 1, 99);
    }

    #[test]
    fn add_pm5_q37_rm1_s99() {
        check_add_row(-5, 37, -1, 99);
    }

    #[test]
    fn add_pm5_q37_r5_s99() {
        check_add_row(-5, 37, 5, 99);
    }

    #[test]
    fn add_p1_q99_r1_s3() {
        check_add_row(1, 99, 1, 3);
    }

    #[test]
    fn add_p1_q99_rm1_s3() {
        check_add_row(1, 99, -1, 3);
    }

    #[test]
    fn add_p1_q99_r5_s3() {
        check_add_row(1, 99, 5, 3);
    }

    #[test]
    fn add_p1_q99_r1_s7() {
        check_add_row(1, 99, 1, 7);
    }

    #[test]
    fn add_p1_q99_rm1_s7() {
        check_add_row(1, 99, -1, 7);
    }

    #[test]
    fn add_p1_q99_r5_s7() {
        check_add_row(1, 99, 5, 7);
    }

    #[test]
    fn add_p1_q99_r1_s17() {
        check_add_row(1, 99, 1, 17);
    }

    #[test]
    fn add_p1_q99_rm1_s17() {
        check_add_row(1, 99, -1, 17);
    }

    #[test]
    fn add_p1_q3_r5_s17() {
        check_add_row(1, 3, 5, 17);
    }

    #[test]
    fn add_p1_q99_r5_s17() {
        check_add_row(1, 99, 5, 17);
    }

    #[test]
    fn add_p1_q99_r1_s99() {
        check_add_row(1, 99, 1, 99);
    }

    #[test]
    fn add_p1_q99_rm1_s99() {
        check_add_row(1, 99, -1, 99);
    }

    #[test]
    fn add_p1_q99_r5_s99() {
        check_add_row(1, 99, 5, 99);
    }

    #[test]
    fn add_p2_q99_r1_s3() {
        check_add_row(2, 99, 1, 3);
    }

    #[test]
    fn add_p2_q99_rm1_s3() {
        check_add_row(2, 99, -1, 3);
    }

    #[test]
    fn add_p2_q99_r5_s3() {
        check_add_row(2, 99, 5, 3);
    }

    #[test]
    fn add_p2_q99_r1_s7() {
        check_add_row(2, 99, 1, 7);
    }

    #[test]
    fn add_p2_q99_rm1_s7() {
        check_add_row(2, 99, -1, 7);
    }

    #[test]
    fn add_p2_q99_r5_s7() {
        check_add_row(2, 99, 5, 7);
    }

    #[test]
    fn add_p2_q99_r1_s17() {
        check_add_row(2, 99, 1, 17);
    }

    #[test]
    fn add_p2_q99_rm1_s17() {
        check_add_row(2, 99, -1, 17);
    }

    #[test]
    fn add_p2_q99_r5_s17() {
        check_add_row(2, 99, 5, 17);
    }

    #[test]
    fn add_p2_q99_r1_s99() {
        check_add_row(2, 99, 1, 99);
    }

    #[test]
    fn add_p2_q99_rm1_s99() {
        check_add_row(2, 99, -1, 99);
    }

    #[test]
    fn add_p2_q99_r5_s99() {
        check_add_row(2, 99, 5, 99);
    }

    #[test]
    fn add_pm5_q99_r1_s3() {
        check_add_row(-5, 99, 1, 3);
    }

    #[test]
    fn add_pm5_q99_rm1_s3() {
        check_add_row(-5, 99, -1, 3);
    }

    #[test]
    fn add_pm5_q99_r5_s3() {
        check_add_row(-5, 99, 5, 3);
    }

    #[test]
    fn add_pm5_q99_r1_s7() {
        check_add_row(-5, 99, 1, 7);
    }

    #[test]
    fn add_pm5_q99_rm1_s7() {
        check_add_row(-5, 99, -1, 7);
    }

    #[test]
    fn add_pm5_q99_r5_s7() {
        check_add_row(-5, 99, 5, 7);
    }

    #[test]
    fn add_pm5_q99_r1_s17() {
        check_add_row(-5, 99, 1, 17);
    }

    #[test]
    fn add_pm5_q99_rm1_s17() {
        check_add_row(-5, 99, -1, 17);
    }

    #[test]
    fn add_pm5_q99_r5_s17() {
        check_add_row(-5, 99, 5, 17);
    }

    #[test]
    fn add_pm5_q99_r1_s99() {
        check_add_row(-5, 99, 1, 99);
    }

    #[test]
    fn add_pm5_q99_rm1_s99() {
        check_add_row(-5, 99, -1, 99);
    }

    #[test]
    fn add_pm5_q99_r5_s99() {
        check_add_row(-5, 99, 5, 99);
    }

    #[test]
    fn add_p1_q3_r1_s99() {
        check_add_row(1, 3, 1, 99);
    }

    #[test]
    fn add_p2_q6_r1_s3() {
        check_add_row(2, 6, 1, 3);
    }

    #[test]
    fn add_p2_q6_rm1_s3() {
        check_add_row(2, 6, -1, 3);
    }

    #[test]
    fn add_p2_q6_r5_s3() {
        check_add_row(2, 6, 5, 3);
    }

    #[test]
    fn add_p2_q6_r1_s7() {
        check_add_row(2, 6, 1, 7);
    }

    #[test]
    fn sub_p1_q3_r1_s3() {
        check_sub_row(1, 3, 1, 3);
    }

    #[test]
    fn sub_p1_q3_rm1_s3() {
        check_sub_row(1, 3, -1, 3);
    }

    #[test]
    fn sub_p1_q3_rm1_s99() {
        check_sub_row(1, 3, -1, 99);
    }

    #[test]
    fn sub_p2_q6_rm1_s7() {
        check_sub_row(2, 6, -1, 7);
    }

    #[test]
    fn sub_p2_q6_r5_s7() {
        check_sub_row(2, 6, 5, 7);
    }

    #[test]
    fn sub_p2_q6_r1_s17() {
        check_sub_row(2, 6, 1, 17);
    }

    #[test]
    fn sub_p2_q6_rm1_s17() {
        check_sub_row(2, 6, -1, 17);
    }

    #[test]
    fn sub_p2_q6_r5_s17() {
        check_sub_row(2, 6, 5, 17);
    }

    #[test]
    fn sub_p2_q6_r1_s99() {
        check_sub_row(2, 6, 1, 99);
    }

    #[test]
    fn sub_p2_q6_rm1_s99() {
        check_sub_row(2, 6, -1, 99);
    }

    #[test]
    fn sub_p2_q6_r5_s99() {
        check_sub_row(2, 6, 5, 99);
    }

    #[test]
    fn sub_p1_q3_r5_s99() {
        check_sub_row(1, 3, 5, 99);
    }

    #[test]
    fn sub_p1_q7_r1_s3() {
        check_sub_row(1, 7, 1, 3);
    }

    #[test]
    fn sub_p1_q7_rm1_s3() {
        check_sub_row(1, 7, -1, 3);
    }

    #[test]
    fn sub_p1_q7_r5_s3() {
        check_sub_row(1, 7, 5, 3);
    }

    #[test]
    fn sub_p1_q7_r1_s7() {
        check_sub_row(1, 7, 1, 7);
    }

    #[test]
    fn sub_p1_q7_rm1_s7() {
        check_sub_row(1, 7, -1, 7);
    }

    #[test]
    fn sub_p1_q7_r5_s7() {
        check_sub_row(1, 7, 5, 7);
    }

    #[test]
    fn sub_p1_q7_r1_s17() {
        check_sub_row(1, 7, 1, 17);
    }

    #[test]
    fn sub_p1_q7_rm1_s17() {
        check_sub_row(1, 7, -1, 17);
    }

    #[test]
    fn sub_p1_q7_r5_s17() {
        check_sub_row(1, 7, 5, 17);
    }

    #[test]
    fn sub_p1_q7_r1_s99() {
        check_sub_row(1, 7, 1, 99);
    }

    #[test]
    fn sub_p1_q7_rm1_s99() {
        check_sub_row(1, 7, -1, 99);
    }

    #[test]
    fn sub_p1_q7_r5_s99() {
        check_sub_row(1, 7, 5, 99);
    }

    #[test]
    fn sub_p2_q7_r1_s3() {
        check_sub_row(2, 7, 1, 3);
    }

    #[test]
    fn sub_p2_q7_rm1_s3() {
        check_sub_row(2, 7, -1, 3);
    }

    #[test]
    fn sub_p2_q7_r5_s3() {
        check_sub_row(2, 7, 5, 3);
    }

    #[test]
    fn sub_p2_q7_r1_s7() {
        check_sub_row(2, 7, 1, 7);
    }

    #[test]
    fn sub_p2_q7_rm1_s7() {
        check_sub_row(2, 7, -1, 7);
    }

    #[test]
    fn sub_p2_q7_r5_s7() {
        check_sub_row(2, 7, 5, 7);
    }

    #[test]
    fn sub_p2_q7_r1_s17() {
        check_sub_row(2, 7, 1, 17);
    }

    #[test]
    fn sub_p2_q7_rm1_s17() {
        check_sub_row(2, 7, -1, 17);
    }

    #[test]
    fn sub_p2_q7_r5_s17() {
        check_sub_row(2, 7, 5, 17);
    }

    #[test]
    fn sub_p2_q7_r1_s99() {
        check_sub_row(2, 7, 1, 99);
    }

    #[test]
    fn sub_p2_q7_rm1_s99() {
        check_sub_row(2, 7, -1, 99);
    }

    #[test]
    fn sub_p2_q7_r5_s99() {
        check_sub_row(2, 7, 5, 99);
    }

    #[test]
    fn sub_pm5_q7_r1_s3() {
        check_sub_row(-5, 7, 1, 3);
    }

    #[test]
    fn sub_pm5_q7_rm1_s3() {
        check_sub_row(-5, 7, -1, 3);
    }

    #[test]
    fn sub_pm5_q7_r5_s3() {
        check_sub_row(-5, 7, 5, 3);
    }

    #[test]
    fn sub_pm5_q7_r1_s7() {
        check_sub_row(-5, 7, 1, 7);
    }

    #[test]
    fn sub_pm5_q7_rm1_s7() {
        check_sub_row(-5, 7, -1, 7);
    }

    #[test]
    fn sub_pm5_q7_r5_s7() {
        check_sub_row(-5, 7, 5, 7);
    }

    #[test]
    fn sub_pm5_q7_r1_s17() {
        check_sub_row(-5, 7, 1, 17);
    }

    #[test]
    fn sub_pm5_q7_rm1_s17() {
        check_sub_row(-5, 7, -1, 17);
    }

    #[test]
    fn sub_p1_q3_r5_s3() {
        check_sub_row(1, 3, 5, 3);
    }

    #[test]
    fn sub_pm5_q7_r5_s17() {
        check_sub_row(-5, 7, 5, 17);
    }

    #[test]
    fn sub_pm5_q7_r1_s99() {
        check_sub_row(-5, 7, 1, 99);
    }

    #[test]
    fn sub_pm5_q7_rm1_s99() {
        check_sub_row(-5, 7, -1, 99);
    }

    #[test]
    fn sub_pm5_q7_r5_s99() {
        check_sub_row(-5, 7, 5, 99);
    }

    #[test]
    fn sub_p1_q9_r1_s3() {
        check_sub_row(1, 9, 1, 3);
    }

    #[test]
    fn sub_p1_q9_rm1_s3() {
        check_sub_row(1, 9, -1, 3);
    }

    #[test]
    fn sub_p1_q9_r5_s3() {
        check_sub_row(1, 9, 5, 3);
    }

    #[test]
    fn sub_p1_q9_r1_s7() {
        check_sub_row(1, 9, 1, 7);
    }

    #[test]
    fn sub_p1_q9_rm1_s7() {
        check_sub_row(1, 9, -1, 7);
    }

    #[test]
    fn sub_p1_q9_r5_s7() {
        check_sub_row(1, 9, 5, 7);
    }

    #[test]
    fn sub_p1_q9_r1_s17() {
        check_sub_row(1, 9, 1, 17);
    }

    #[test]
    fn sub_p1_q9_rm1_s17() {
        check_sub_row(1, 9, -1, 17);
    }

    #[test]
    fn sub_p1_q9_r5_s17() {
        check_sub_row(1, 9, 5, 17);
    }

    #[test]
    fn sub_p1_q9_r1_s99() {
        check_sub_row(1, 9, 1, 99);
    }

    #[test]
    fn sub_p1_q9_rm1_s99() {
        check_sub_row(1, 9, -1, 99);
    }

    #[test]
    fn sub_p1_q9_r5_s99() {
        check_sub_row(1, 9, 5, 99);
    }

    #[test]
    fn sub_p2_q3_r1_s3() {
        check_sub_row(2, 3, 1, 3);
    }

    #[test]
    fn sub_p2_q9_r1_s3() {
        check_sub_row(2, 9, 1, 3);
    }

    #[test]
    fn sub_p2_q9_rm1_s3() {
        check_sub_row(2, 9, -1, 3);
    }

    #[test]
    fn sub_p2_q9_r5_s3() {
        check_sub_row(2, 9, 5, 3);
    }

    #[test]
    fn sub_p2_q9_r1_s7() {
        check_sub_row(2, 9, 1, 7);
    }

    #[test]
    fn sub_p2_q9_rm1_s7() {
        check_sub_row(2, 9, -1, 7);
    }

    #[test]
    fn sub_p2_q9_r5_s7() {
        check_sub_row(2, 9, 5, 7);
    }

    #[test]
    fn sub_p2_q9_r1_s17() {
        check_sub_row(2, 9, 1, 17);
    }

    #[test]
    fn sub_p2_q9_rm1_s17() {
        check_sub_row(2, 9, -1, 17);
    }

    #[test]
    fn sub_p2_q9_r5_s17() {
        check_sub_row(2, 9, 5, 17);
    }

    #[test]
    fn sub_p2_q9_r1_s99() {
        check_sub_row(2, 9, 1, 99);
    }

    #[test]
    fn sub_p2_q3_rm1_s3() {
        check_sub_row(2, 3, -1, 3);
    }

    #[test]
    fn sub_p2_q9_rm1_s99() {
        check_sub_row(2, 9, -1, 99);
    }

    #[test]
    fn sub_p2_q9_r5_s99() {
        check_sub_row(2, 9, 5, 99);
    }

    #[test]
    fn sub_p2_q3_r5_s3() {
        check_sub_row(2, 3, 5, 3);
    }

    #[test]
    fn sub_pm5_q9_r1_s3() {
        check_sub_row(-5, 9, 1, 3);
    }

    #[test]
    fn sub_pm5_q9_rm1_s3() {
        check_sub_row(-5, 9, -1, 3);
    }

    #[test]
    fn sub_pm5_q9_r5_s3() {
        check_sub_row(-5, 9, 5, 3);
    }

    #[test]
    fn sub_pm5_q9_r1_s7() {
        check_sub_row(-5, 9, 1, 7);
    }

    #[test]
    fn sub_pm5_q9_rm1_s7() {
        check_sub_row(-5, 9, -1, 7);
    }

    #[test]
    fn sub_pm5_q9_r5_s7() {
        check_sub_row(-5, 9, 5, 7);
    }

    #[test]
    fn sub_p2_q3_r1_s7() {
        check_sub_row(2, 3, 1, 7);
    }

    #[test]
    fn sub_pm5_q9_r1_s17() {
        check_sub_row(-5, 9, 1, 17);
    }

    #[test]
    fn sub_pm5_q9_rm1_s17() {
        check_sub_row(-5, 9, -1, 17);
    }

    #[test]
    fn sub_pm5_q9_r5_s17() {
        check_sub_row(-5, 9, 5, 17);
    }

    #[test]
    fn sub_pm5_q9_r1_s99() {
        check_sub_row(-5, 9, 1, 99);
    }

    #[test]
    fn sub_pm5_q9_rm1_s99() {
        check_sub_row(-5, 9, -1, 99);
    }

    #[test]
    fn sub_pm5_q9_r5_s99() {
        check_sub_row(-5, 9, 5, 99);
    }

    #[test]
    fn sub_p2_q3_rm1_s7() {
        check_sub_row(2, 3, -1, 7);
    }

    #[test]
    fn sub_p1_q11_r1_s3() {
        check_sub_row(1, 11, 1, 3);
    }

    #[test]
    fn sub_p1_q11_rm1_s3() {
        check_sub_row(1, 11, -1, 3);
    }

    #[test]
    fn sub_p2_q3_r5_s7() {
        check_sub_row(2, 3, 5, 7);
    }

    #[test]
    fn sub_p1_q11_r5_s3() {
        check_sub_row(1, 11, 5, 3);
    }

    #[test]
    fn sub_p1_q11_r1_s7() {
        check_sub_row(1, 11, 1, 7);
    }

    #[test]
    fn sub_p1_q11_rm1_s7() {
        check_sub_row(1, 11, -1, 7);
    }

    #[test]
    fn sub_p1_q11_r5_s7() {
        check_sub_row(1, 11, 5, 7);
    }

    #[test]
    fn sub_p1_q11_r1_s17() {
        check_sub_row(1, 11, 1, 17);
    }

    #[test]
    fn sub_p1_q11_rm1_s17() {
        check_sub_row(1, 11, -1, 17);
    }

    #[test]
    fn sub_p1_q11_r5_s17() {
        check_sub_row(1, 11, 5, 17);
    }

    #[test]
    fn sub_p1_q11_r1_s99() {
        check_sub_row(1, 11, 1, 99);
    }

    #[test]
    fn sub_p1_q11_rm1_s99() {
        check_sub_row(1, 11, -1, 99);
    }

    #[test]
    fn sub_p1_q11_r5_s99() {
        check_sub_row(1, 11, 5, 99);
    }

    #[test]
    fn sub_p1_q3_r1_s7() {
        check_sub_row(1, 3, 1, 7);
    }

    #[test]
    fn sub_p2_q3_r1_s17() {
        check_sub_row(2, 3, 1, 17);
    }

    #[test]
    fn sub_p2_q3_rm1_s17() {
        check_sub_row(2, 3, -1, 17);
    }

    #[test]
    fn sub_p2_q11_r1_s3() {
        check_sub_row(2, 11, 1, 3);
    }

    #[test]
    fn sub_p2_q11_rm1_s3() {
        check_sub_row(2, 11, -1, 3);
    }

    #[test]
    fn sub_p2_q11_r5_s3() {
        check_sub_row(2, 11, 5, 3);
    }

    #[test]
    fn sub_p2_q11_r1_s7() {
        check_sub_row(2, 11, 1, 7);
    }

    #[test]
    fn sub_p2_q11_rm1_s7() {
        check_sub_row(2, 11, -1, 7);
    }

    #[test]
    fn sub_p2_q11_r5_s7() {
        check_sub_row(2, 11, 5, 7);
    }

    #[test]
    fn sub_p2_q11_r1_s17() {
        check_sub_row(2, 11, 1, 17);
    }

    #[test]
    fn sub_p2_q11_rm1_s17() {
        check_sub_row(2, 11, -1, 17);
    }

    #[test]
    fn sub_p2_q3_r5_s17() {
        check_sub_row(2, 3, 5, 17);
    }

    #[test]
    fn sub_p2_q11_r5_s17() {
        check_sub_row(2, 11, 5, 17);
    }

    #[test]
    fn sub_p2_q11_r1_s99() {
        check_sub_row(2, 11, 1, 99);
    }

    #[test]
    fn sub_p2_q11_rm1_s99() {
        check_sub_row(2, 11, -1, 99);
    }

    #[test]
    fn sub_p2_q11_r5_s99() {
        check_sub_row(2, 11, 5, 99);
    }

    #[test]
    fn sub_p2_q3_r1_s99() {
        check_sub_row(2, 3, 1, 99);
    }

    #[test]
    fn sub_pm5_q11_r1_s3() {
        check_sub_row(-5, 11, 1, 3);
    }

    #[test]
    fn sub_pm5_q11_rm1_s3() {
        check_sub_row(-5, 11, -1, 3);
    }

    #[test]
    fn sub_pm5_q11_r5_s3() {
        check_sub_row(-5, 11, 5, 3);
    }

    #[test]
    fn sub_pm5_q11_r1_s7() {
        check_sub_row(-5, 11, 1, 7);
    }

    #[test]
    fn sub_p2_q3_rm1_s99() {
        check_sub_row(2, 3, -1, 99);
    }

    #[test]
    fn sub_pm5_q11_rm1_s7() {
        check_sub_row(-5, 11, -1, 7);
    }

    #[test]
    fn sub_pm5_q11_r5_s7() {
        check_sub_row(-5, 11, 5, 7);
    }

    #[test]
    fn sub_pm5_q11_r1_s17() {
        check_sub_row(-5, 11, 1, 17);
    }

    #[test]
    fn sub_pm5_q11_rm1_s17() {
        check_sub_row(-5, 11, -1, 17);
    }

    #[test]
    fn sub_pm5_q11_r5_s17() {
        check_sub_row(-5, 11, 5, 17);
    }

    #[test]
    fn sub_pm5_q11_r1_s99() {
        check_sub_row(-5, 11, 1, 99);
    }

    #[test]
    fn sub_pm5_q11_rm1_s99() {
        check_sub_row(-5, 11, -1, 99);
    }

    #[test]
    fn sub_pm5_q11_r5_s99() {
        check_sub_row(-5, 11, 5, 99);
    }

    #[test]
    fn sub_p2_q3_r5_s99() {
        check_sub_row(2, 3, 5, 99);
    }

    #[test]
    fn sub_p1_q3_rm1_s7() {
        check_sub_row(1, 3, -1, 7);
    }

    #[test]
    fn sub_p1_q13_r1_s3() {
        check_sub_row(1, 13, 1, 3);
    }

    #[test]
    fn sub_p1_q13_rm1_s3() {
        check_sub_row(1, 13, -1, 3);
    }

    #[test]
    fn sub_p1_q13_r5_s3() {
        check_sub_row(1, 13, 5, 3);
    }

    #[test]
    fn sub_p1_q13_r1_s7() {
        check_sub_row(1, 13, 1, 7);
    }

    #[test]
    fn sub_p1_q13_rm1_s7() {
        check_sub_row(1, 13, -1, 7);
    }

    #[test]
    fn sub_p1_q13_r5_s7() {
        check_sub_row(1, 13, 5, 7);
    }

    #[test]
    fn sub_p1_q13_r1_s17() {
        check_sub_row(1, 13, 1, 17);
    }

    #[test]
    fn sub_p1_q13_rm1_s17() {
        check_sub_row(1, 13, -1, 17);
    }

    #[test]
    fn sub_p1_q13_r5_s17() {
        check_sub_row(1, 13, 5, 17);
    }

    #[test]
    fn sub_p1_q13_r1_s99() {
        check_sub_row(1, 13, 1, 99);
    }

    #[test]
    fn sub_p1_q13_rm1_s99() {
        check_sub_row(1, 13, -1, 99);
    }

    #[test]
    fn sub_p1_q13_r5_s99() {
        check_sub_row(1, 13, 5, 99);
    }

    #[test]
    fn sub_p2_q13_r1_s3() {
        check_sub_row(2, 13, 1, 3);
    }

    #[test]
    fn sub_p2_q13_rm1_s3() {
        check_sub_row(2, 13, -1, 3);
    }

    #[test]
    fn sub_p2_q13_r5_s3() {
        check_sub_row(2, 13, 5, 3);
    }

    #[test]
    fn sub_p2_q13_r1_s7() {
        check_sub_row(2, 13, 1, 7);
    }

    #[test]
    fn sub_p2_q13_rm1_s7() {
        check_sub_row(2, 13, -1, 7);
    }

    #[test]
    fn sub_p2_q13_r5_s7() {
        check_sub_row(2, 13, 5, 7);
    }

    #[test]
    fn sub_p2_q13_r1_s17() {
        check_sub_row(2, 13, 1, 17);
    }

    #[test]
    fn sub_p2_q13_rm1_s17() {
        check_sub_row(2, 13, -1, 17);
    }

    #[test]
    fn sub_p2_q13_r5_s17() {
        check_sub_row(2, 13, 5, 17);
    }

    #[test]
    fn sub_p2_q13_r1_s99() {
        check_sub_row(2, 13, 1, 99);
    }

    #[test]
    fn sub_p2_q13_rm1_s99() {
        check_sub_row(2, 13, -1, 99);
    }

    #[test]
    fn sub_p2_q13_r5_s99() {
        check_sub_row(2, 13, 5, 99);
    }

    #[test]
    fn sub_pm5_q3_r1_s3() {
        check_sub_row(-5, 3, 1, 3);
    }

    #[test]
    fn sub_pm5_q13_r1_s3() {
        check_sub_row(-5, 13, 1, 3);
    }

    #[test]
    fn sub_pm5_q13_rm1_s3() {
        check_sub_row(-5, 13, -1, 3);
    }

    #[test]
    fn sub_pm5_q13_r5_s3() {
        check_sub_row(-5, 13, 5, 3);
    }

    #[test]
    fn sub_pm5_q13_r1_s7() {
        check_sub_row(-5, 13, 1, 7);
    }

    #[test]
    fn sub_pm5_q13_rm1_s7() {
        check_sub_row(-5, 13, -1, 7);
    }

    #[test]
    fn sub_pm5_q13_r5_s7() {
        check_sub_row(-5, 13, 5, 7);
    }

    #[test]
    fn sub_pm5_q13_r1_s17() {
        check_sub_row(-5, 13, 1, 17);
    }

    #[test]
    fn sub_pm5_q13_rm1_s17() {
        check_sub_row(-5, 13, -1, 17);
    }

    #[test]
    fn sub_pm5_q13_r5_s17() {
        check_sub_row(-5, 13, 5, 17);
    }

    #[test]
    fn sub_pm5_q13_r1_s99() {
        check_sub_row(-5, 13, 1, 99);
    }

    #[test]
    fn sub_pm5_q3_rm1_s3() {
        check_sub_row(-5, 3, -1, 3);
    }

    #[test]
    fn sub_pm5_q13_rm1_s99() {
        check_sub_row(-5, 13, -1, 99);
    }

    #[test]
    fn sub_pm5_q13_r5_s99() {
        check_sub_row(-5, 13, 5, 99);
    }

    #[test]
    fn sub_p1_q3_r5_s7() {
        check_sub_row(1, 3, 5, 7);
    }

    #[test]
    fn sub_pm5_q3_r5_s3() {
        check_sub_row(-5, 3, 5, 3);
    }

    #[test]
    fn sub_p1_q17_r1_s3() {
        check_sub_row(1, 17, 1, 3);
    }

    #[test]
    fn sub_p1_q17_rm1_s3() {
        check_sub_row(1, 17, -1, 3);
    }

    #[test]
    fn sub_p1_q17_r5_s3() {
        check_sub_row(1, 17, 5, 3);
    }

    #[test]
    fn sub_p1_q17_r1_s7() {
        check_sub_row(1, 17, 1, 7);
    }

    #[test]
    fn sub_p1_q17_rm1_s7() {
        check_sub_row(1, 17, -1, 7);
    }

    #[test]
    fn sub_p1_q17_r5_s7() {
        check_sub_row(1, 17, 5, 7);
    }

    #[test]
    fn sub_pm5_q3_r1_s7() {
        check_sub_row(-5, 3, 1, 7);
    }

    #[test]
    fn sub_p1_q17_r1_s17() {
        check_sub_row(1, 17, 1, 17);
    }

    #[test]
    fn sub_p1_q17_rm1_s17() {
        check_sub_row(1, 17, -1, 17);
    }

    #[test]
    fn sub_p1_q17_r5_s17() {
        check_sub_row(1, 17, 5, 17);
    }

    #[test]
    fn sub_p1_q17_r1_s99() {
        check_sub_row(1, 17, 1, 99);
    }

    #[test]
    fn sub_p1_q17_rm1_s99() {
        check_sub_row(1, 17, -1, 99);
    }

    #[test]
    fn sub_p1_q17_r5_s99() {
        check_sub_row(1, 17, 5, 99);
    }

    #[test]
    fn sub_pm5_q3_rm1_s7() {
        check_sub_row(-5, 3, -1, 7);
    }

    #[test]
    fn sub_p2_q17_r1_s3() {
        check_sub_row(2, 17, 1, 3);
    }

    #[test]
    fn sub_p2_q17_rm1_s3() {
        check_sub_row(2, 17, -1, 3);
    }

    #[test]
    fn sub_pm5_q3_r5_s7() {
        check_sub_row(-5, 3, 5, 7);
    }

    #[test]
    fn sub_p2_q17_r5_s3() {
        check_sub_row(2, 17, 5, 3);
    }

    #[test]
    fn sub_p2_q17_r1_s7() {
        check_sub_row(2, 17, 1, 7);
    }

    #[test]
    fn sub_p2_q17_rm1_s7() {
        check_sub_row(2, 17, -1, 7);
    }

    #[test]
    fn sub_p2_q17_r5_s7() {
        check_sub_row(2, 17, 5, 7);
    }

    #[test]
    fn sub_p2_q17_r1_s17() {
        check_sub_row(2, 17, 1, 17);
    }

    #[test]
    fn sub_p2_q17_rm1_s17() {
        check_sub_row(2, 17, -1, 17);
    }

    #[test]
    fn sub_p2_q17_r5_s17() {
        check_sub_row(2, 17, 5, 17);
    }

    #[test]
    fn sub_p2_q17_r1_s99() {
        check_sub_row(2, 17, 1, 99);
    }

    #[test]
    fn sub_p2_q17_rm1_s99() {
        check_sub_row(2, 17, -1, 99);
    }

    #[test]
    fn sub_p2_q17_r5_s99() {
        check_sub_row(2, 17, 5, 99);
    }

    #[test]
    fn sub_pm5_q3_r1_s17() {
        check_sub_row(-5, 3, 1, 17);
    }

    #[test]
    fn sub_pm5_q3_rm1_s17() {
        check_sub_row(-5, 3, -1, 17);
    }

    #[test]
    fn sub_pm5_q17_r1_s3() {
        check_sub_row(-5, 17, 1, 3);
    }

    #[test]
    fn sub_pm5_q17_rm1_s3() {
        check_sub_row(-5, 17, -1, 3);
    }

    #[test]
    fn sub_pm5_q17_r5_s3() {
        check_sub_row(-5, 17, 5, 3);
    }

    #[test]
    fn sub_pm5_q17_r1_s7() {
        check_sub_row(-5, 17, 1, 7);
    }

    #[test]
    fn sub_pm5_q17_rm1_s7() {
        check_sub_row(-5, 17, -1, 7);
    }

    #[test]
    fn sub_pm5_q17_r5_s7() {
        check_sub_row(-5, 17, 5, 7);
    }

    #[test]
    fn sub_pm5_q17_r1_s17() {
        check_sub_row(-5, 17, 1, 17);
    }

    #[test]
    fn sub_pm5_q17_rm1_s17() {
        check_sub_row(-5, 17, -1, 17);
    }

    #[test]
    fn sub_pm5_q3_r5_s17() {
        check_sub_row(-5, 3, 5, 17);
    }

    #[test]
    fn sub_pm5_q17_r5_s17() {
        check_sub_row(-5, 17, 5, 17);
    }

    #[test]
    fn sub_pm5_q17_r1_s99() {
        check_sub_row(-5, 17, 1, 99);
    }

    #[test]
    fn sub_pm5_q17_rm1_s99() {
        check_sub_row(-5, 17, -1, 99);
    }

    #[test]
    fn sub_pm5_q17_r5_s99() {
        check_sub_row(-5, 17, 5, 99);
    }

    #[test]
    fn sub_pm5_q3_r1_s99() {
        check_sub_row(-5, 3, 1, 99);
    }

    #[test]
    fn sub_p1_q19_r1_s3() {
        check_sub_row(1, 19, 1, 3);
    }

    #[test]
    fn sub_p1_q19_rm1_s3() {
        check_sub_row(1, 19, -1, 3);
    }

    #[test]
    fn sub_p1_q19_r5_s3() {
        check_sub_row(1, 19, 5, 3);
    }

    #[test]
    fn sub_p1_q19_r1_s7() {
        check_sub_row(1, 19, 1, 7);
    }

    #[test]
    fn sub_pm5_q3_rm1_s99() {
        check_sub_row(-5, 3, -1, 99);
    }

    #[test]
    fn sub_p1_q19_rm1_s7() {
        check_sub_row(1, 19, -1, 7);
    }

    #[test]
    fn sub_p1_q19_r5_s7() {
        check_sub_row(1, 19, 5, 7);
    }

    #[test]
    fn sub_p1_q19_r1_s99() {
        check_sub_row(1, 19, 1, 99);
    }

    #[test]
    fn sub_p1_q19_rm1_s99() {
        check_sub_row(1, 19, -1, 99);
    }

    #[test]
    fn sub_p1_q19_r5_s99() {
        check_sub_row(1, 19, 5, 99);
    }

    #[test]
    fn sub_pm5_q3_r5_s99() {
        check_sub_row(-5, 3, 5, 99);
    }

    #[test]
    fn sub_p1_q3_r1_s17() {
        check_sub_row(1, 3, 1, 17);
    }

    #[test]
    fn sub_p2_q19_r1_s3() {
        check_sub_row(2, 19, 1, 3);
    }

    #[test]
    fn sub_p2_q19_rm1_s3() {
        check_sub_row(2, 19, -1, 3);
    }

    #[test]
    fn sub_p2_q19_r5_s3() {
        check_sub_row(2, 19, 5, 3);
    }

    #[test]
    fn sub_p2_q19_r1_s7() {
        check_sub_row(2, 19, 1, 7);
    }

    #[test]
    fn sub_p2_q19_rm1_s7() {
        check_sub_row(2, 19, -1, 7);
    }

    #[test]
    fn sub_p2_q19_r5_s7() {
        check_sub_row(2, 19, 5, 7);
    }

    #[test]
    fn sub_p2_q19_r1_s99() {
        check_sub_row(2, 19, 1, 99);
    }

    #[test]
    fn sub_p2_q19_rm1_s99() {
        check_sub_row(2, 19, -1, 99);
    }

    #[test]
    fn sub_p2_q19_r5_s99() {
        check_sub_row(2, 19, 5, 99);
    }

    #[test]
    fn sub_pm5_q19_r1_s3() {
        check_sub_row(-5, 19, 1, 3);
    }

    #[test]
    fn sub_pm5_q19_rm1_s3() {
        check_sub_row(-5, 19, -1, 3);
    }

    #[test]
    fn sub_pm5_q19_r5_s3() {
        check_sub_row(-5, 19, 5, 3);
    }

    #[test]
    fn sub_pm5_q19_r1_s7() {
        check_sub_row(-5, 19, 1, 7);
    }

    #[test]
    fn sub_pm5_q19_rm1_s7() {
        check_sub_row(-5, 19, -1, 7);
    }

    #[test]
    fn sub_pm5_q19_r5_s7() {
        check_sub_row(-5, 19, 5, 7);
    }

    #[test]
    fn sub_pm5_q19_r1_s99() {
        check_sub_row(-5, 19, 1, 99);
    }

    #[test]
    fn sub_pm5_q19_rm1_s99() {
        check_sub_row(-5, 19, -1, 99);
    }

    #[test]
    fn sub_pm5_q19_r5_s99() {
        check_sub_row(-5, 19, 5, 99);
    }

    #[test]
    fn sub_p1_q21_r1_s3() {
        check_sub_row(1, 21, 1, 3);
    }

    #[test]
    fn sub_p1_q21_rm1_s3() {
        check_sub_row(1, 21, -1, 3);
    }

    #[test]
    fn sub_p1_q21_r5_s3() {
        check_sub_row(1, 21, 5, 3);
    }

    #[test]
    fn sub_p1_q21_r1_s7() {
        check_sub_row(1, 21, 1, 7);
    }

    #[test]
    fn sub_p1_q21_rm1_s7() {
        check_sub_row(1, 21, -1, 7);
    }

    #[test]
    fn sub_p1_q21_r5_s7() {
        check_sub_row(1, 21, 5, 7);
    }

    #[test]
    fn sub_p1_q21_r1_s17() {
        check_sub_row(1, 21, 1, 17);
    }

    #[test]
    fn sub_p1_q21_rm1_s17() {
        check_sub_row(1, 21, -1, 17);
    }

    #[test]
    fn sub_p1_q21_r5_s17() {
        check_sub_row(1, 21, 5, 17);
    }

    #[test]
    fn sub_p1_q21_r1_s99() {
        check_sub_row(1, 21, 1, 99);
    }

    #[test]
    fn sub_p1_q21_rm1_s99() {
        check_sub_row(1, 21, -1, 99);
    }

    #[test]
    fn sub_p1_q21_r5_s99() {
        check_sub_row(1, 21, 5, 99);
    }

    #[test]
    fn sub_p2_q21_r1_s3() {
        check_sub_row(2, 21, 1, 3);
    }

    #[test]
    fn sub_p2_q21_rm1_s3() {
        check_sub_row(2, 21, -1, 3);
    }

    #[test]
    fn sub_p2_q21_r5_s3() {
        check_sub_row(2, 21, 5, 3);
    }

    #[test]
    fn sub_p2_q21_r1_s7() {
        check_sub_row(2, 21, 1, 7);
    }

    #[test]
    fn sub_p2_q21_rm1_s7() {
        check_sub_row(2, 21, -1, 7);
    }

    #[test]
    fn sub_p2_q21_r5_s7() {
        check_sub_row(2, 21, 5, 7);
    }

    #[test]
    fn sub_p2_q21_r1_s17() {
        check_sub_row(2, 21, 1, 17);
    }

    #[test]
    fn sub_p2_q21_rm1_s17() {
        check_sub_row(2, 21, -1, 17);
    }

    #[test]
    fn sub_p2_q21_r5_s17() {
        check_sub_row(2, 21, 5, 17);
    }

    #[test]
    fn sub_p2_q21_r1_s99() {
        check_sub_row(2, 21, 1, 99);
    }

    #[test]
    fn sub_p2_q21_rm1_s99() {
        check_sub_row(2, 21, -1, 99);
    }

    #[test]
    fn sub_p2_q21_r5_s99() {
        check_sub_row(2, 21, 5, 99);
    }

    #[test]
    fn sub_pm5_q21_r1_s3() {
        check_sub_row(-5, 21, 1, 3);
    }

    #[test]
    fn sub_pm5_q21_rm1_s3() {
        check_sub_row(-5, 21, -1, 3);
    }

    #[test]
    fn sub_pm5_q21_r5_s3() {
        check_sub_row(-5, 21, 5, 3);
    }

    #[test]
    fn sub_pm5_q21_r1_s7() {
        check_sub_row(-5, 21, 1, 7);
    }

    #[test]
    fn sub_p1_q3_rm1_s17() {
        check_sub_row(1, 3, -1, 17);
    }

    #[test]
    fn sub_pm5_q21_rm1_s7() {
        check_sub_row(-5, 21, -1, 7);
    }

    #[test]
    fn sub_pm5_q21_r5_s7() {
        check_sub_row(-5, 21, 5, 7);
    }

    #[test]
    fn sub_pm5_q21_r1_s17() {
        check_sub_row(-5, 21, 1, 17);
    }

    #[test]
    fn sub_pm5_q21_rm1_s17() {
        check_sub_row(-5, 21, -1, 17);
    }

    #[test]
    fn sub_pm5_q21_r5_s17() {
        check_sub_row(-5, 21, 5, 17);
    }

    #[test]
    fn sub_pm5_q21_r1_s99() {
        check_sub_row(-5, 21, 1, 99);
    }

    #[test]
    fn sub_pm5_q21_rm1_s99() {
        check_sub_row(-5, 21, -1, 99);
    }

    #[test]
    fn sub_pm5_q21_r5_s99() {
        check_sub_row(-5, 21, 5, 99);
    }

    #[test]
    fn sub_p1_q37_r1_s3() {
        check_sub_row(1, 37, 1, 3);
    }

    #[test]
    fn sub_p1_q37_rm1_s3() {
        check_sub_row(1, 37, -1, 3);
    }

    #[test]
    fn sub_p1_q37_r5_s3() {
        check_sub_row(1, 37, 5, 3);
    }

    #[test]
    fn sub_p1_q37_r1_s7() {
        check_sub_row(1, 37, 1, 7);
    }

    #[test]
    fn sub_p1_q37_rm1_s7() {
        check_sub_row(1, 37, -1, 7);
    }

    #[test]
    fn sub_p1_q37_r5_s7() {
        check_sub_row(1, 37, 5, 7);
    }

    #[test]
    fn sub_p1_q37_r1_s17() {
        check_sub_row(1, 37, 1, 17);
    }

    #[test]
    fn sub_p1_q37_rm1_s17() {
        check_sub_row(1, 37, -1, 17);
    }

    #[test]
    fn sub_p1_q37_r5_s17() {
        check_sub_row(1, 37, 5, 17);
    }

    #[test]
    fn sub_p1_q37_r1_s99() {
        check_sub_row(1, 37, 1, 99);
    }

    #[test]
    fn sub_p1_q37_rm1_s99() {
        check_sub_row(1, 37, -1, 99);
    }

    #[test]
    fn sub_p1_q37_r5_s99() {
        check_sub_row(1, 37, 5, 99);
    }

    #[test]
    fn sub_p2_q37_r1_s3() {
        check_sub_row(2, 37, 1, 3);
    }

    #[test]
    fn sub_p2_q37_rm1_s3() {
        check_sub_row(2, 37, -1, 3);
    }

    #[test]
    fn sub_p2_q37_r5_s3() {
        check_sub_row(2, 37, 5, 3);
    }

    #[test]
    fn sub_p2_q37_r1_s7() {
        check_sub_row(2, 37, 1, 7);
    }

    #[test]
    fn sub_p2_q37_rm1_s7() {
        check_sub_row(2, 37, -1, 7);
    }

    #[test]
    fn sub_p2_q37_r5_s7() {
        check_sub_row(2, 37, 5, 7);
    }

    #[test]
    fn sub_p2_q37_r1_s17() {
        check_sub_row(2, 37, 1, 17);
    }

    #[test]
    fn sub_p2_q37_rm1_s17() {
        check_sub_row(2, 37, -1, 17);
    }

    #[test]
    fn sub_p2_q37_r5_s17() {
        check_sub_row(2, 37, 5, 17);
    }

    #[test]
    fn sub_p2_q37_r1_s99() {
        check_sub_row(2, 37, 1, 99);
    }

    #[test]
    fn sub_p2_q37_rm1_s99() {
        check_sub_row(2, 37, -1, 99);
    }

    #[test]
    fn sub_p2_q37_r5_s99() {
        check_sub_row(2, 37, 5, 99);
    }

    #[test]
    fn sub_pm5_q37_r1_s3() {
        check_sub_row(-5, 37, 1, 3);
    }

    #[test]
    fn sub_pm5_q37_rm1_s3() {
        check_sub_row(-5, 37, -1, 3);
    }

    #[test]
    fn sub_pm5_q37_r5_s3() {
        check_sub_row(-5, 37, 5, 3);
    }

    #[test]
    fn sub_pm5_q37_r1_s7() {
        check_sub_row(-5, 37, 1, 7);
    }

    #[test]
    fn sub_pm5_q37_rm1_s7() {
        check_sub_row(-5, 37, -1, 7);
    }

    #[test]
    fn sub_pm5_q37_r5_s7() {
        check_sub_row(-5, 37, 5, 7);
    }

    #[test]
    fn sub_pm5_q37_r1_s17() {
        check_sub_row(-5, 37, 1, 17);
    }

    #[test]
    fn sub_pm5_q37_rm1_s17() {
        check_sub_row(-5, 37, -1, 17);
    }

    #[test]
    fn sub_pm5_q37_r5_s17() {
        check_sub_row(-5, 37, 5, 17);
    }

    #[test]
    fn sub_pm5_q37_r1_s99() {
        check_sub_row(-5, 37, 1, 99);
    }

    #[test]
    fn sub_pm5_q37_rm1_s99() {
        check_sub_row(-5, 37, -1, 99);
    }

    #[test]
    fn sub_pm5_q37_r5_s99() {
        check_sub_row(-5, 37, 5, 99);
    }

    #[test]
    fn sub_p1_q99_r1_s3() {
        check_sub_row(1, 99, 1, 3);
    }

    #[test]
    fn sub_p1_q99_rm1_s3() {
        check_sub_row(1, 99, -1, 3);
    }

    #[test]
    fn sub_p1_q99_r5_s3() {
        check_sub_row(1, 99, 5, 3);
    }

    #[test]
    fn sub_p1_q99_r1_s7() {
        check_sub_row(1, 99, 1, 7);
    }

    #[test]
    fn sub_p1_q99_rm1_s7() {
        check_sub_row(1, 99, -1, 7);
    }

    #[test]
    fn sub_p1_q99_r5_s7() {
        check_sub_row(1, 99, 5, 7);
    }

    #[test]
    fn sub_p1_q99_r1_s17() {
        check_sub_row(1, 99, 1, 17);
    }

    #[test]
    fn sub_p1_q99_rm1_s17() {
        check_sub_row(1, 99, -1, 17);
    }

    #[test]
    fn sub_p1_q3_r5_s17() {
        check_sub_row(1, 3, 5, 17);
    }

    #[test]
    fn sub_p1_q99_r5_s17() {
        check_sub_row(1, 99, 5, 17);
    }

    #[test]
    fn sub_p1_q99_r1_s99() {
        check_sub_row(1, 99, 1, 99);
    }

    #[test]
    fn sub_p1_q99_rm1_s99() {
        check_sub_row(1, 99, -1, 99);
    }

    #[test]
    fn sub_p1_q99_r5_s99() {
        check_sub_row(1, 99, 5, 99);
    }

    #[test]
    fn sub_p2_q99_r1_s3() {
        check_sub_row(2, 99, 1, 3);
    }

    #[test]
    fn sub_p2_q99_rm1_s3() {
        check_sub_row(2, 99, -1, 3);
    }

    #[test]
    fn sub_p2_q99_r5_s3() {
        check_sub_row(2, 99, 5, 3);
    }

    #[test]
    fn sub_p2_q99_r1_s7() {
        check_sub_row(2, 99, 1, 7);
    }

    #[test]
    fn sub_p2_q99_rm1_s7() {
        check_sub_row(2, 99, -1, 7);
    }

    #[test]
    fn sub_p2_q99_r5_s7() {
        check_sub_row(2, 99, 5, 7);
    }

    #[test]
    fn sub_p2_q99_r1_s17() {
        check_sub_row(2, 99, 1, 17);
    }

    #[test]
    fn sub_p2_q99_rm1_s17() {
        check_sub_row(2, 99, -1, 17);
    }

    #[test]
    fn sub_p2_q99_r5_s17() {
        check_sub_row(2, 99, 5, 17);
    }

    #[test]
    fn sub_p2_q99_r1_s99() {
        check_sub_row(2, 99, 1, 99);
    }

    #[test]
    fn sub_p2_q99_rm1_s99() {
        check_sub_row(2, 99, -1, 99);
    }

    #[test]
    fn sub_p2_q99_r5_s99() {
        check_sub_row(2, 99, 5, 99);
    }

    #[test]
    fn sub_pm5_q99_r1_s3() {
        check_sub_row(-5, 99, 1, 3);
    }

    #[test]
    fn sub_pm5_q99_rm1_s3() {
        check_sub_row(-5, 99, -1, 3);
    }

    #[test]
    fn sub_pm5_q99_r5_s3() {
        check_sub_row(-5, 99, 5, 3);
    }

    #[test]
    fn sub_pm5_q99_r1_s7() {
        check_sub_row(-5, 99, 1, 7);
    }

    #[test]
    fn sub_pm5_q99_rm1_s7() {
        check_sub_row(-5, 99, -1, 7);
    }

    #[test]
    fn sub_pm5_q99_r5_s7() {
        check_sub_row(-5, 99, 5, 7);
    }

    #[test]
    fn sub_pm5_q99_r1_s17() {
        check_sub_row(-5, 99, 1, 17);
    }

    #[test]
    fn sub_pm5_q99_rm1_s17() {
        check_sub_row(-5, 99, -1, 17);
    }

    #[test]
    fn sub_pm5_q99_r5_s17() {
        check_sub_row(-5, 99, 5, 17);
    }

    #[test]
    fn sub_pm5_q99_r1_s99() {
        check_sub_row(-5, 99, 1, 99);
    }

    #[test]
    fn sub_pm5_q99_rm1_s99() {
        check_sub_row(-5, 99, -1, 99);
    }

    #[test]
    fn sub_pm5_q99_r5_s99() {
        check_sub_row(-5, 99, 5, 99);
    }

    #[test]
    fn sub_p1_q3_r1_s99() {
        check_sub_row(1, 3, 1, 99);
    }

    #[test]
    fn sub_p2_q6_r1_s3() {
        check_sub_row(2, 6, 1, 3);
    }

    #[test]
    fn sub_p2_q6_rm1_s3() {
        check_sub_row(2, 6, -1, 3);
    }

    #[test]
    fn sub_p2_q6_r5_s3() {
        check_sub_row(2, 6, 5, 3);
    }

    #[test]
    fn sub_p2_q6_r1_s7() {
        check_sub_row(2, 6, 1, 7);
    }

    #[test]
    fn mul_p1_q2() {
        check_mul_row(1, "2");
    }

    #[test]
    fn mul_p2_q2() {
        check_mul_row(2, "2");
    }

    #[test]
    fn mul_p1_q4() {
        check_mul_row(1, "4");
    }

    #[test]
    fn mul_p2_q22() {
        check_mul_row(2, "22");
    }

    #[test]
    fn mul_p9999_q22() {
        check_mul_row(9999, "22");
    }

    #[test]
    fn mul_p1_q23() {
        check_mul_row(1, "23");
    }

    #[test]
    fn mul_p2_q23() {
        check_mul_row(2, "23");
    }

    #[test]
    fn mul_p7_q23() {
        check_mul_row(7, "23");
    }

    #[test]
    fn mul_p123_q23() {
        check_mul_row(123, "23");
    }

    #[test]
    fn mul_p9999_q23() {
        check_mul_row(9999, "23");
    }

    #[test]
    fn mul_p2_q4() {
        check_mul_row(2, "4");
    }

    #[test]
    fn mul_p123_q24() {
        check_mul_row(123, "24");
    }

    #[test]
    fn mul_p9999_q24() {
        check_mul_row(9999, "24");
    }

    #[test]
    fn mul_p1_q25() {
        check_mul_row(1, "25");
    }

    #[test]
    fn mul_p2_q25() {
        check_mul_row(2, "25");
    }

    #[test]
    fn mul_p7_q25() {
        check_mul_row(7, "25");
    }

    #[test]
    fn mul_p123_q25() {
        check_mul_row(123, "25");
    }

    #[test]
    fn mul_p9999_q25() {
        check_mul_row(9999, "25");
    }

    #[test]
    fn mul_p7_q4() {
        check_mul_row(7, "4");
    }

    #[test]
    fn mul_p2_q26() {
        check_mul_row(2, "26");
    }

    #[test]
    fn mul_p1_q27() {
        check_mul_row(1, "27");
    }

    #[test]
    fn mul_p2_q27() {
        check_mul_row(2, "27");
    }

    #[test]
    fn mul_p7_q27() {
        check_mul_row(7, "27");
    }

    #[test]
    fn mul_p123_q27() {
        check_mul_row(123, "27");
    }

    #[test]
    fn mul_p9999_q27() {
        check_mul_row(9999, "27");
    }

    #[test]
    fn mul_p123_q4() {
        check_mul_row(123, "4");
    }

    #[test]
    fn mul_p7_q28() {
        check_mul_row(7, "28");
    }

    #[test]
    fn mul_p1_q29() {
        check_mul_row(1, "29");
    }

    #[test]
    fn mul_p2_q29() {
        check_mul_row(2, "29");
    }

    #[test]
    fn mul_p7_q29() {
        check_mul_row(7, "29");
    }

    #[test]
    fn mul_p123_q29() {
        check_mul_row(123, "29");
    }

    #[test]
    fn mul_p9999_q29() {
        check_mul_row(9999, "29");
    }

    #[test]
    fn mul_p9999_q4() {
        check_mul_row(9999, "4");
    }

    #[test]
    fn mul_p123_q30() {
        check_mul_row(123, "30");
    }

    #[test]
    fn mul_p9999_q30() {
        check_mul_row(9999, "30");
    }

    #[test]
    fn mul_p1_q31() {
        check_mul_row(1, "31");
    }

    #[test]
    fn mul_p2_q31() {
        check_mul_row(2, "31");
    }

    #[test]
    fn mul_p7_q31() {
        check_mul_row(7, "31");
    }

    #[test]
    fn mul_p123_q31() {
        check_mul_row(123, "31");
    }

    #[test]
    fn mul_p9999_q31() {
        check_mul_row(9999, "31");
    }

    #[test]
    fn mul_p1_q5() {
        check_mul_row(1, "5");
    }

    #[test]
    fn mul_p1_q32() {
        check_mul_row(1, "32");
    }

    #[test]
    fn mul_p2_q32() {
        check_mul_row(2, "32");
    }

    #[test]
    fn mul_p7_q32() {
        check_mul_row(7, "32");
    }

    #[test]
    fn mul_p123_q32() {
        check_mul_row(123, "32");
    }

    #[test]
    fn mul_p9999_q32() {
        check_mul_row(9999, "32");
    }

    #[test]
    fn mul_p1_q33() {
        check_mul_row(1, "33");
    }

    #[test]
    fn mul_p2_q33() {
        check_mul_row(2, "33");
    }

    #[test]
    fn mul_p7_q33() {
        check_mul_row(7, "33");
    }

    #[test]
    fn mul_p123_q33() {
        check_mul_row(123, "33");
    }

    #[test]
    fn mul_p9999_q33() {
        check_mul_row(9999, "33");
    }

    #[test]
    fn mul_p2_q5() {
        check_mul_row(2, "5");
    }

    #[test]
    fn mul_p2_q34() {
        check_mul_row(2, "34");
    }

    #[test]
    fn mul_p7_q35() {
        check_mul_row(7, "35");
    }

    #[test]
    fn mul_p7_q5() {
        check_mul_row(7, "5");
    }

    #[test]
    fn mul_p9999_q36() {
        check_mul_row(9999, "36");
    }

    #[test]
    fn mul_p1_q37() {
        check_mul_row(1, "37");
    }

    #[test]
    fn mul_p2_q37() {
        check_mul_row(2, "37");
    }

    #[test]
    fn mul_p7_q37() {
        check_mul_row(7, "37");
    }

    #[test]
    fn mul_p123_q37() {
        check_mul_row(123, "37");
    }

    #[test]
    fn mul_p9999_q37() {
        check_mul_row(9999, "37");
    }

    #[test]
    fn mul_p123_q5() {
        check_mul_row(123, "5");
    }

    #[test]
    fn mul_p2_q38() {
        check_mul_row(2, "38");
    }

    #[test]
    fn mul_p1_q39() {
        check_mul_row(1, "39");
    }

    #[test]
    fn mul_p2_q39() {
        check_mul_row(2, "39");
    }

    #[test]
    fn mul_p7_q39() {
        check_mul_row(7, "39");
    }

    #[test]
    fn mul_p123_q39() {
        check_mul_row(123, "39");
    }

    #[test]
    fn mul_p9999_q39() {
        check_mul_row(9999, "39");
    }

    #[test]
    fn mul_p9999_q5() {
        check_mul_row(9999, "5");
    }

    #[test]
    fn mul_p1_q40() {
        check_mul_row(1, "40");
    }

    #[test]
    fn mul_p2_q40() {
        check_mul_row(2, "40");
    }

    #[test]
    fn mul_p7_q40() {
        check_mul_row(7, "40");
    }

    #[test]
    fn mul_p123_q40() {
        check_mul_row(123, "40");
    }

    #[test]
    fn mul_p9999_q40() {
        check_mul_row(9999, "40");
    }

    #[test]
    fn mul_p1_q41() {
        check_mul_row(1, "41");
    }

    #[test]
    fn mul_p2_q41() {
        check_mul_row(2, "41");
    }

    #[test]
    fn mul_p7_q41() {
        check_mul_row(7, "41");
    }

    #[test]
    fn mul_p123_q41() {
        check_mul_row(123, "41");
    }

    #[test]
    fn mul_p9999_q41() {
        check_mul_row(9999, "41");
    }

    #[test]
    fn mul_p7_q2() {
        check_mul_row(7, "2");
    }

    #[test]
    fn mul_p2_q42() {
        check_mul_row(2, "42");
    }

    #[test]
    fn mul_p1_q43() {
        check_mul_row(1, "43");
    }

    #[test]
    fn mul_p2_q43() {
        check_mul_row(2, "43");
    }

    #[test]
    fn mul_p7_q43() {
        check_mul_row(7, "43");
    }

    #[test]
    fn mul_p123_q43() {
        check_mul_row(123, "43");
    }

    #[test]
    fn mul_p9999_q43() {
        check_mul_row(9999, "43");
    }

    #[test]
    fn mul_p2_q6() {
        check_mul_row(2, "6");
    }

    #[test]
    fn mul_p9999_q44() {
        check_mul_row(9999, "44");
    }

    #[test]
    fn mul_p9999_q45() {
        check_mul_row(9999, "45");
    }

    #[test]
    fn mul_p2_q46() {
        check_mul_row(2, "46");
    }

    #[test]
    fn mul_p1_q47() {
        check_mul_row(1, "47");
    }

    #[test]
    fn mul_p2_q47() {
        check_mul_row(2, "47");
    }

    #[test]
    fn mul_p7_q47() {
        check_mul_row(7, "47");
    }

    #[test]
    fn mul_p123_q47() {
        check_mul_row(123, "47");
    }

    #[test]
    fn mul_p9999_q47() {
        check_mul_row(9999, "47");
    }

    #[test]
    fn mul_p123_q6() {
        check_mul_row(123, "6");
    }

    #[test]
    fn mul_p123_q48() {
        check_mul_row(123, "48");
    }

    #[test]
    fn mul_p9999_q48() {
        check_mul_row(9999, "48");
    }

    #[test]
    fn mul_p1_q49() {
        check_mul_row(1, "49");
    }

    #[test]
    fn mul_p2_q49() {
        check_mul_row(2, "49");
    }

    #[test]
    fn mul_p7_q49() {
        check_mul_row(7, "49");
    }

    #[test]
    fn mul_p123_q49() {
        check_mul_row(123, "49");
    }

    #[test]
    fn mul_p9999_q49() {
        check_mul_row(9999, "49");
    }

    #[test]
    fn mul_p9999_q6() {
        check_mul_row(9999, "6");
    }

    #[test]
    fn mul_p1_q50() {
        check_mul_row(1, "50");
    }

    #[test]
    fn mul_p2_q50() {
        check_mul_row(2, "50");
    }

    #[test]
    fn mul_p7_q50() {
        check_mul_row(7, "50");
    }

    #[test]
    fn mul_p123_q50() {
        check_mul_row(123, "50");
    }

    #[test]
    fn mul_p9999_q50() {
        check_mul_row(9999, "50");
    }

    #[test]
    fn mul_p1_q51() {
        check_mul_row(1, "51");
    }

    #[test]
    fn mul_p2_q51() {
        check_mul_row(2, "51");
    }

    #[test]
    fn mul_p7_q51() {
        check_mul_row(7, "51");
    }

    #[test]
    fn mul_p123_q51() {
        check_mul_row(123, "51");
    }

    #[test]
    fn mul_p9999_q51() {
        check_mul_row(9999, "51");
    }

    #[test]
    fn mul_p1_q7() {
        check_mul_row(1, "7");
    }

    #[test]
    fn mul_p1_q53() {
        check_mul_row(1, "53");
    }

    #[test]
    fn mul_p2_q53() {
        check_mul_row(2, "53");
    }

    #[test]
    fn mul_p7_q53() {
        check_mul_row(7, "53");
    }

    #[test]
    fn mul_p123_q53() {
        check_mul_row(123, "53");
    }

    #[test]
    fn mul_p9999_q53() {
        check_mul_row(9999, "53");
    }

    #[test]
    fn mul_p2_q7() {
        check_mul_row(2, "7");
    }

    #[test]
    fn mul_p2_q54() {
        check_mul_row(2, "54");
    }

    #[test]
    fn mul_p9999_q55() {
        check_mul_row(9999, "55");
    }

    #[test]
    fn mul_p7_q7() {
        check_mul_row(7, "7");
    }

    #[test]
    fn mul_p7_q56() {
        check_mul_row(7, "56");
    }

    #[test]
    fn mul_p1_q57() {
        check_mul_row(1, "57");
    }

    #[test]
    fn mul_p2_q57() {
        check_mul_row(2, "57");
    }

    #[test]
    fn mul_p7_q57() {
        check_mul_row(7, "57");
    }

    #[test]
    fn mul_p123_q57() {
        check_mul_row(123, "57");
    }

    #[test]
    fn mul_p9999_q57() {
        check_mul_row(9999, "57");
    }

    #[test]
    fn mul_p123_q7() {
        check_mul_row(123, "7");
    }

    #[test]
    fn mul_p2_q58() {
        check_mul_row(2, "58");
    }

    #[test]
    fn mul_p1_q59() {
        check_mul_row(1, "59");
    }

    #[test]
    fn mul_p2_q59() {
        check_mul_row(2, "59");
    }

    #[test]
    fn mul_p7_q59() {
        check_mul_row(7, "59");
    }

    #[test]
    fn mul_p123_q59() {
        check_mul_row(123, "59");
    }

    #[test]
    fn mul_p9999_q59() {
        check_mul_row(9999, "59");
    }

    #[test]
    fn mul_p9999_q7() {
        check_mul_row(9999, "7");
    }

    #[test]
    fn mul_p123_q60() {
        check_mul_row(123, "60");
    }

    #[test]
    fn mul_p9999_q60() {
        check_mul_row(9999, "60");
    }

    #[test]
    fn mul_p1_q97() {
        check_mul_row(1, "97");
    }

    #[test]
    fn mul_p2_q97() {
        check_mul_row(2, "97");
    }

    #[test]
    fn mul_p7_q97() {
        check_mul_row(7, "97");
    }

    #[test]
    fn mul_p123_q97() {
        check_mul_row(123, "97");
    }

    #[test]
    fn mul_p9999_q97() {
        check_mul_row(9999, "97");
    }

    #[test]
    fn mul_p123_q2() {
        check_mul_row(123, "2");
    }

    #[test]
    fn mul_p1_q8() {
        check_mul_row(1, "8");
    }

    #[test]
    fn mul_p1_q101() {
        check_mul_row(1, "101");
    }

    #[test]
    fn mul_p2_q101() {
        check_mul_row(2, "101");
    }

    #[test]
    fn mul_p7_q101() {
        check_mul_row(7, "101");
    }

    #[test]
    fn mul_p123_q101() {
        check_mul_row(123, "101");
    }

    #[test]
    fn mul_p9999_q101() {
        check_mul_row(9999, "101");
    }

    #[test]
    fn mul_p1_q109() {
        check_mul_row(1, "109");
    }

    #[test]
    fn mul_p2_q109() {
        check_mul_row(2, "109");
    }

    #[test]
    fn mul_p7_q109() {
        check_mul_row(7, "109");
    }

    #[test]
    fn mul_p123_q109() {
        check_mul_row(123, "109");
    }

    #[test]
    fn mul_p9999_q109() {
        check_mul_row(9999, "109");
    }

    #[test]
    fn mul_p2_q8() {
        check_mul_row(2, "8");
    }

    #[test]
    fn mul_p1_q127() {
        check_mul_row(1, "127");
    }

    #[test]
    fn mul_p2_q127() {
        check_mul_row(2, "127");
    }

    #[test]
    fn mul_p7_q127() {
        check_mul_row(7, "127");
    }

    #[test]
    fn mul_p123_q127() {
        check_mul_row(123, "127");
    }

    #[test]
    fn mul_p9999_q127() {
        check_mul_row(9999, "127");
    }

    #[test]
    fn mul_p1_q199() {
        check_mul_row(1, "199");
    }

    #[test]
    fn mul_p2_q199() {
        check_mul_row(2, "199");
    }

    #[test]
    fn mul_p7_q199() {
        check_mul_row(7, "199");
    }

    #[test]
    fn mul_p123_q199() {
        check_mul_row(123, "199");
    }

    #[test]
    fn mul_p9999_q199() {
        check_mul_row(9999, "199");
    }

    #[test]
    fn mul_p7_q8() {
        check_mul_row(7, "8");
    }

    #[test]
    fn mul_p1_q0_75() {
        check_mul_row(1, "0.75");
    }

    #[test]
    fn mul_p2_q0_75() {
        check_mul_row(2, "0.75");
    }

    #[test]
    fn mul_p7_q0_75() {
        check_mul_row(7, "0.75");
    }

    #[test]
    fn mul_p123_q0_75() {
        check_mul_row(123, "0.75");
    }

    #[test]
    fn mul_p9999_q0_75() {
        check_mul_row(9999, "0.75");
    }

    #[test]
    fn mul_p1_q0_9() {
        check_mul_row(1, "0.9");
    }

    #[test]
    fn mul_p2_q0_9() {
        check_mul_row(2, "0.9");
    }

    #[test]
    fn mul_p7_q0_9() {
        check_mul_row(7, "0.9");
    }

    #[test]
    fn mul_p123_q0_9() {
        check_mul_row(123, "0.9");
    }

    #[test]
    fn mul_p9999_q0_9() {
        check_mul_row(9999, "0.9");
    }

    #[test]
    fn mul_p123_q8() {
        check_mul_row(123, "8");
    }

    #[test]
    fn mul_p1_q0_95() {
        check_mul_row(1, "0.95");
    }

    #[test]
    fn mul_p2_q0_95() {
        check_mul_row(2, "0.95");
    }

    #[test]
    fn mul_p7_q0_95() {
        check_mul_row(7, "0.95");
    }

    #[test]
    fn mul_p123_q0_95() {
        check_mul_row(123, "0.95");
    }

    #[test]
    fn mul_p9999_q0_95() {
        check_mul_row(9999, "0.95");
    }

    #[test]
    fn mul_p1_q0_55() {
        check_mul_row(1, "0.55");
    }

    #[test]
    fn mul_p2_q0_55() {
        check_mul_row(2, "0.55");
    }

    #[test]
    fn mul_p7_q0_55() {
        check_mul_row(7, "0.55");
    }

    #[test]
    fn mul_p123_q0_55() {
        check_mul_row(123, "0.55");
    }

    #[test]
    fn mul_p9999_q0_55() {
        check_mul_row(9999, "0.55");
    }

    #[test]
    fn mul_p9999_q8() {
        check_mul_row(9999, "8");
    }

    #[test]
    fn mul_p1_q0_35() {
        check_mul_row(1, "0.35");
    }

    #[test]
    fn mul_p2_q0_35() {
        check_mul_row(2, "0.35");
    }

    #[test]
    fn mul_p7_q0_35() {
        check_mul_row(7, "0.35");
    }

    #[test]
    fn mul_p123_q0_35() {
        check_mul_row(123, "0.35");
    }

    #[test]
    fn mul_p9999_q0_35() {
        check_mul_row(9999, "0.35");
    }

    #[test]
    fn mul_p1_q0_98() {
        check_mul_row(1, "0.98");
    }

    #[test]
    fn mul_p2_q0_98() {
        check_mul_row(2, "0.98");
    }

    #[test]
    fn mul_p7_q0_98() {
        check_mul_row(7, "0.98");
    }

    #[test]
    fn mul_p123_q0_98() {
        check_mul_row(123, "0.98");
    }

    #[test]
    fn mul_p9999_q0_98() {
        check_mul_row(9999, "0.98");
    }

    #[test]
    fn mul_p1_q9() {
        check_mul_row(1, "9");
    }

    #[test]
    fn mul_p1_q0_94() {
        check_mul_row(1, "0.94");
    }

    #[test]
    fn mul_p2_q0_94() {
        check_mul_row(2, "0.94");
    }

    #[test]
    fn mul_p7_q0_94() {
        check_mul_row(7, "0.94");
    }

    #[test]
    fn mul_p123_q0_94() {
        check_mul_row(123, "0.94");
    }

    #[test]
    fn mul_p9999_q0_94() {
        check_mul_row(9999, "0.94");
    }

    #[test]
    fn mul_p1_q0_71() {
        check_mul_row(1, "0.71");
    }

    #[test]
    fn mul_p2_q0_71() {
        check_mul_row(2, "0.71");
    }

    #[test]
    fn mul_p7_q0_71() {
        check_mul_row(7, "0.71");
    }

    #[test]
    fn mul_p123_q0_71() {
        check_mul_row(123, "0.71");
    }

    #[test]
    fn mul_p9999_q0_71() {
        check_mul_row(9999, "0.71");
    }

    #[test]
    fn mul_p2_q9() {
        check_mul_row(2, "9");
    }

    #[test]
    fn mul_p1_q0_49() {
        check_mul_row(1, "0.49");
    }

    #[test]
    fn mul_p2_q0_49() {
        check_mul_row(2, "0.49");
    }

    #[test]
    fn mul_p7_q0_49() {
        check_mul_row(7, "0.49");
    }

    #[test]
    fn mul_p123_q0_49() {
        check_mul_row(123, "0.49");
    }

    #[test]
    fn mul_p9999_q0_49() {
        check_mul_row(9999, "0.49");
    }

    #[test]
    fn mul_p1_q0_99999() {
        check_mul_row(1, "0.99999");
    }

    #[test]
    fn mul_p2_q0_99999() {
        check_mul_row(2, "0.99999");
    }

    #[test]
    fn mul_p7_q0_99999() {
        check_mul_row(7, "0.99999");
    }

    #[test]
    fn mul_p123_q0_99999() {
        check_mul_row(123, "0.99999");
    }

    #[test]
    fn mul_p9999_q0_99999() {
        check_mul_row(9999, "0.99999");
    }

    #[test]
    fn mul_p7_q9() {
        check_mul_row(7, "9");
    }

    #[test]
    fn mul_p1_q1_25() {
        check_mul_row(1, "1.25");
    }

    #[test]
    fn mul_p2_q1_25() {
        check_mul_row(2, "1.25");
    }

    #[test]
    fn mul_p7_q1_25() {
        check_mul_row(7, "1.25");
    }

    #[test]
    fn mul_p123_q1_25() {
        check_mul_row(123, "1.25");
    }

    #[test]
    fn mul_p9999_q1_25() {
        check_mul_row(9999, "1.25");
    }

    #[test]
    fn mul_p1_q0_125() {
        check_mul_row(1, "0.125");
    }

    #[test]
    fn mul_p2_q0_125() {
        check_mul_row(2, "0.125");
    }

    #[test]
    fn mul_p7_q0_125() {
        check_mul_row(7, "0.125");
    }

    #[test]
    fn mul_p123_q0_125() {
        check_mul_row(123, "0.125");
    }

    #[test]
    fn mul_p9999_q0_125() {
        check_mul_row(9999, "0.125");
    }

    #[test]
    fn mul_p123_q9() {
        check_mul_row(123, "9");
    }

    #[test]
    fn mul_p1_q0_05() {
        check_mul_row(1, "0.05");
    }

    #[test]
    fn mul_p2_q0_05() {
        check_mul_row(2, "0.05");
    }

    #[test]
    fn mul_p7_q0_05() {
        check_mul_row(7, "0.05");
    }

    #[test]
    fn mul_p123_q0_05() {
        check_mul_row(123, "0.05");
    }

    #[test]
    fn mul_p9999_q0_05() {
        check_mul_row(9999, "0.05");
    }

    #[test]
    fn mul_p1_q2_5() {
        check_mul_row(1, "2.5");
    }

    #[test]
    fn mul_p2_q2_5() {
        check_mul_row(2, "2.5");
    }

    #[test]
    fn mul_p7_q2_5() {
        check_mul_row(7, "2.5");
    }

    #[test]
    fn mul_p123_q2_5() {
        check_mul_row(123, "2.5");
    }

    #[test]
    fn mul_p9999_q2_5() {
        check_mul_row(9999, "2.5");
    }

    #[test]
    fn mul_p9999_q9() {
        check_mul_row(9999, "9");
    }

    #[test]
    fn mul_p9999_q2() {
        check_mul_row(9999, "2");
    }

    #[test]
    fn mul_p1_q10() {
        check_mul_row(1, "10");
    }

    #[test]
    fn mul_p2_q10() {
        check_mul_row(2, "10");
    }

    #[test]
    fn mul_p7_q10() {
        check_mul_row(7, "10");
    }

    #[test]
    fn mul_p123_q10() {
        check_mul_row(123, "10");
    }

    #[test]
    fn mul_p9999_q10() {
        check_mul_row(9999, "10");
    }

    #[test]
    fn mul_p1_q11() {
        check_mul_row(1, "11");
    }

    #[test]
    fn mul_p2_q11() {
        check_mul_row(2, "11");
    }

    #[test]
    fn mul_p7_q11() {
        check_mul_row(7, "11");
    }

    #[test]
    fn mul_p123_q11() {
        check_mul_row(123, "11");
    }

    #[test]
    fn mul_p9999_q11() {
        check_mul_row(9999, "11");
    }

    #[test]
    fn mul_p1_q3() {
        check_mul_row(1, "3");
    }

    #[test]
    fn mul_p123_q12() {
        check_mul_row(123, "12");
    }

    #[test]
    fn mul_p9999_q12() {
        check_mul_row(9999, "12");
    }

    #[test]
    fn mul_p1_q13() {
        check_mul_row(1, "13");
    }

    #[test]
    fn mul_p2_q13() {
        check_mul_row(2, "13");
    }

    #[test]
    fn mul_p7_q13() {
        check_mul_row(7, "13");
    }

    #[test]
    fn mul_p123_q13() {
        check_mul_row(123, "13");
    }

    #[test]
    fn mul_p9999_q13() {
        check_mul_row(9999, "13");
    }

    #[test]
    fn mul_p2_q3() {
        check_mul_row(2, "3");
    }

    #[test]
    fn mul_p2_q14() {
        check_mul_row(2, "14");
    }

    #[test]
    fn mul_p7_q14() {
        check_mul_row(7, "14");
    }

    #[test]
    fn mul_p123_q15() {
        check_mul_row(123, "15");
    }

    #[test]
    fn mul_p9999_q15() {
        check_mul_row(9999, "15");
    }

    #[test]
    fn mul_p7_q3() {
        check_mul_row(7, "3");
    }

    #[test]
    fn mul_p1_q16() {
        check_mul_row(1, "16");
    }

    #[test]
    fn mul_p2_q16() {
        check_mul_row(2, "16");
    }

    #[test]
    fn mul_p7_q16() {
        check_mul_row(7, "16");
    }

    #[test]
    fn mul_p123_q16() {
        check_mul_row(123, "16");
    }

    #[test]
    fn mul_p9999_q16() {
        check_mul_row(9999, "16");
    }

    #[test]
    fn mul_p1_q17() {
        check_mul_row(1, "17");
    }

    #[test]
    fn mul_p2_q17() {
        check_mul_row(2, "17");
    }

    #[test]
    fn mul_p7_q17() {
        check_mul_row(7, "17");
    }

    #[test]
    fn mul_p123_q17() {
        check_mul_row(123, "17");
    }

    #[test]
    fn mul_p9999_q17() {
        check_mul_row(9999, "17");
    }

    #[test]
    fn mul_p123_q3() {
        check_mul_row(123, "3");
    }

    #[test]
    fn mul_p2_q18() {
        check_mul_row(2, "18");
    }

    #[test]
    fn mul_p9999_q18() {
        check_mul_row(9999, "18");
    }

    #[test]
    fn mul_p1_q19() {
        check_mul_row(1, "19");
    }

    #[test]
    fn mul_p2_q19() {
        check_mul_row(2, "19");
    }

    #[test]
    fn mul_p7_q19() {
        check_mul_row(7, "19");
    }

    #[test]
    fn mul_p123_q19() {
        check_mul_row(123, "19");
    }

    #[test]
    fn mul_p9999_q19() {
        check_mul_row(9999, "19");
    }

    #[test]
    fn mul_p9999_q3() {
        check_mul_row(9999, "3");
    }

    #[test]
    fn mul_p1_q20() {
        check_mul_row(1, "20");
    }

    #[test]
    fn mul_p2_q20() {
        check_mul_row(2, "20");
    }

    #[test]
    fn mul_p7_q20() {
        check_mul_row(7, "20");
    }

    #[test]
    fn mul_p123_q20() {
        check_mul_row(123, "20");
    }

    #[test]
    fn mul_p9999_q20() {
        check_mul_row(9999, "20");
    }

    #[test]
    fn mul_p1_q21() {
        check_mul_row(1, "21");
    }

    #[test]
    fn mul_p2_q21() {
        check_mul_row(2, "21");
    }

    #[test]
    fn mul_p7_q21() {
        check_mul_row(7, "21");
    }

    #[test]
    fn mul_p123_q21() {
        check_mul_row(123, "21");
    }

    #[test]
    fn mul_p9999_q21() {
        check_mul_row(9999, "21");
    }
    // ---------------------------------------------------------------------
    // Manual ports: RealAddTests.cs, RealSubtractTests.cs, RealMultiplyTests.cs,
    // RealNegationTests.cs, RealIncrementTests.cs, RealDecrementTests.cs,
    // RealInvertTests.cs, RealParseTests.cs, RealConstructorTests.cs,
    // RealToStringTests.cs, RealSqrtTests.cs, RealPiTests.cs, RealPowTests.cs,
    // RealPowZeroBaseTests.cs, RealPropertiesTests.cs, RealDivideTests.cs,
    // RealPeriodicIntegerValuePropertyTests.cs, RealSqrtPeriodicTests.cs,
    // RealSqrtSeedingTests.cs, RealSqrtAsyncTests.cs, RealPiAsyncTests.cs
    // ---------------------------------------------------------------------

    // RealAddTests.cs -----------------------------------------------------

    #[test]
    fn add_GivenSameExponent_ReturnsThreePointEight() {
        assert_eq!(r("1.5").add(&r("2.3")), r("3.8"));
    }

    #[test]
    fn add_GivenDifferentExponents_AlignsAndReturnsOnePointSevenFive() {
        assert_eq!(r("1.5").add(&r("0.25")), r("1.75"));
    }

    #[test]
    fn add_GivenPositiveAndNegative_ReturnsOnePointFive() {
        assert_eq!(r("3.0").add(&r("-1.5")), r("1.5"));
    }

    #[test]
    fn add_GivenZeroAndValue_ReturnsValue() {
        assert_eq!(r("0.0").add(&r("7.77")), r("7.77"));
    }

    #[test]
    fn add_GivenBothNegative_ReturnsMinusThreePointThree() {
        assert_eq!(r("-1.1").add(&r("-2.2")), r("-3.3"));
    }

    #[test]
    fn add_GivenThousandthsOperand_KeepsThreeFractionalDigits() {
        let result = r("1.5").add(&r("0.001"));
        assert_eq!(result, r("1.501"));
        assert_eq!(result.to_string(), "1.501");
        assert_eq!(frac_digits(&result), 3);
    }

    #[test]
    fn add_GivenValueAndDefaultZero_ReturnsValue() {
        assert_eq!(r("5.5").add(&Real::zero()), r("5.5"));
    }

    #[test]
    fn add_GivenNineThenOne_CarriesIntoTheIntegerPart() {
        assert_eq!(r("0.9").add(&r("0.1")), r("1"));
    }

    #[test]
    fn add_GivenTwoPeriodicReals_SumIsExactlyOne() {
        let result = r("0.(3)").add(&r("0.(6)"));
        assert_eq!(result, r("1"));
        assert!(!is_periodic(&result));
    }

    #[test]
    fn add_GivenThirdAndTenth_RendersZeroPointFourPeriodThree() {
        let result = r("0.(3)").add(&r("0.1"));
        assert_eq!(result, r("0.4(3)"));
        assert_eq!(result.to_string(), "0.4(3)");
        assert!(is_periodic(&result));
    }

    #[test]
    fn add_GivenNegativeThirdAndOne_RendersZeroPointSixPeriod() {
        let result = r("-0.(3)").add(&r("1"));
        assert_eq!(result, r("0.(6)"));
        assert!(is_periodic(&result));
    }

    // RealSubtractTests.cs ------------------------------------------------

    #[test]
    fn sub_GivenFiveMinusThreePointTwo_ReturnsOnePointEight() {
        assert_eq!(r("5.0").sub(&r("3.2")), r("1.8"));
    }

    #[test]
    fn sub_GivenValueMinusItself_IsZero() {
        let value = r("5.123");
        assert!(value.sub(&value).is_zero());
    }

    #[test]
    fn sub_GivenOneMinusTwoPointFive_ReturnsMinusOnePointFive() {
        assert_eq!(r("1.0").sub(&r("2.5")), r("-1.5"));
    }

    #[test]
    fn sub_GivenOneMinusOneThousandth_ReturnsZeroPointNineNineNine() {
        assert_eq!(r("1.0").sub(&r("0.001")), r("0.999"));
    }

    #[test]
    fn sub_GivenZeroMinusThreePointFive_ReturnsMinusThreePointFive() {
        assert_eq!(Real::zero().sub(&r("3.5")), r("-3.5"));
    }

    #[test]
    fn sub_GivenBothNegative_ReturnsOnePointFive() {
        assert_eq!(r("-1.0").sub(&r("-2.5")), r("1.5"));
    }

    #[test]
    fn sub_GivenThousandthsOperand_KeepsThreeFractionalDigits() {
        let result = r("1.5").sub(&r("0.001"));
        assert_eq!(result, r("1.499"));
        assert_eq!(result.to_string(), "1.499");
        assert_eq!(frac_digits(&result), 3);
    }

    // RealMultiplyTests.cs ------------------------------------------------

    #[test]
    fn mul_GivenOnePointFiveAndTwo_ReturnsThree() {
        let result = r("1.5").mul(&r("2.0"));
        assert_eq!(result, r("3"));
        assert!(!is_negative(&result));
    }

    #[test]
    fn mul_GivenThreeAndMinusTwo_ReturnsMinusSix() {
        let result = r("3.0").mul(&r("-2.0"));
        assert_eq!(result, r("-6"));
        assert!(is_negative(&result));
    }

    #[test]
    fn mul_GivenTwoNegatives_ReturnsSix() {
        let result = r("-3.0").mul(&r("-2.0"));
        assert!(!is_negative(&result));
        assert_eq!(result, r("6"));
    }

    #[test]
    fn mul_GivenOnePointFiveAndOnePointTwoFive_KeepsThreeFractionalDigits() {
        let result = r("1.5").mul(&r("1.25"));
        assert_eq!(result.to_string(), "1.875");
        assert_eq!(frac_digits(&result), 3);
    }

    #[test]
    fn mul_GivenZeroFactor_IsZero() {
        assert!(r("0.0").mul(&r("999.9")).is_zero());
    }

    #[test]
    fn mul_GivenTenthAndTenth_ReturnsHundredth() {
        assert_eq!(r("0.1").mul(&r("0.1")), r("0.01"));
    }

    #[test]
    fn mul_GivenSixAndSeven_ReturnsFortyTwoAtExponentZero() {
        let result = r("6").mul(&r("7"));
        assert_eq!(result, r("42"));
        assert_eq!(result.to_string(), "42");
    }

    #[test]
    fn mul_GivenValueAndOne_ReturnsSelf() {
        let value = r("3.14");
        let result = value.mul(&r("1"));
        assert_eq!(result, value);
        assert_eq!(result.to_string(), "3.14");
    }

    // RealNegationTests.cs ------------------------------------------------

    #[test]
    fn negate_GivenThreePointFourteen_RendersMinusThreePointFourteen() {
        let neg = r("3.14").neg();
        assert!(is_negative(&neg));
        assert_eq!(neg, r("-3.14"));
        assert_eq!(neg.to_string(), "-3.14");
    }

    #[test]
    fn negate_GivenMinusTwoPointFive_RendersTwoPointFive() {
        let pos = r("-2.5").neg();
        assert!(!is_negative(&pos));
        assert_eq!(pos, r("2.5"));
    }

    #[test]
    fn negate_GivenZero_IsZero() {
        assert!(Real::zero().neg().is_zero());
    }

    #[test]
    fn negate_GivenThreePointFourteen_KeepsTwoFractionalDigits() {
        let neg = r("3.14").neg();
        assert_eq!(neg.to_string(), "-3.14");
        assert_eq!(frac_digits(&neg), 2);
        assert_eq!(neg, r("-3.14"));
    }

    #[test]
    fn negate_GivenThird_KeepsPeriodAndRendersMinusZeroPointPeriodThree() {
        let third = r("0.(3)");
        let neg = third.neg();
        assert!(is_periodic(&neg));
        assert_eq!(neg.to_string(), "-0.(3)");
        assert_eq!(neg, r("-0.(3)"));
    }

    #[test]
    fn negate_GivenSevenPointFive_TwiceReturnsOriginal() {
        let value = r("7.5");
        assert_eq!(value.neg().neg(), value);
    }

    // RealIncrementTests.cs -----------------------------------------------

    fn increment(value: &Real) -> Real {
        value.add(&Real::from_i64(1))
    }

    #[test]
    fn increment_GivenFive_ReturnsSix() {
        assert_eq!(increment(&r("5")), r("6"));
    }

    #[test]
    fn increment_GivenMinusFive_ReturnsMinusFour() {
        assert_eq!(increment(&r("-5")), r("-4"));
    }

    #[test]
    fn increment_GivenZero_ReturnsOne() {
        assert_eq!(increment(&Real::zero()), Real::from_i64(1));
    }

    #[test]
    fn increment_GivenMinusOne_IsZero() {
        assert!(increment(&Real::from_i64(-1)).is_zero());
    }

    #[test]
    fn increment_GivenTwoPointFive_ReturnsThreePointFive() {
        assert_eq!(increment(&r("2.5")), r("3.5"));
    }

    // RealDecrementTests.cs -----------------------------------------------

    fn decrement(value: &Real) -> Real {
        value.sub(&Real::from_i64(1))
    }

    #[test]
    fn decrement_GivenFive_ReturnsFour() {
        assert_eq!(decrement(&r("5")), r("4"));
    }

    #[test]
    fn decrement_GivenMinusFive_ReturnsMinusSix() {
        assert_eq!(decrement(&r("-5")), r("-6"));
    }

    #[test]
    fn decrement_GivenZero_ReturnsNegativeOne() {
        assert_eq!(decrement(&Real::zero()), Real::from_i64(-1));
    }

    #[test]
    fn decrement_GivenOne_IsZero() {
        assert!(decrement(&Real::from_i64(1)).is_zero());
    }

    #[test]
    fn decrement_GivenTwoPointFive_ReturnsOnePointFive() {
        assert_eq!(decrement(&r("2.5")), r("1.5"));
    }

    // RealInvertTests.cs --------------------------------------------------

    fn invert(value: &Real) -> Option<Real> {
        Real::from_i64(1).div(value, 1000)
    }

    #[test]
    fn invert_GivenTwo_ReturnsHalfNotPeriodic() {
        let result = invert(&r("2.0")).unwrap();
        assert_eq!(result, r("0.5"));
        assert!(!is_periodic(&result));
    }

    #[test]
    fn invert_GivenOne_ReturnsOne() {
        assert_eq!(invert(&r("1.0")).unwrap(), r("1"));
    }

    #[test]
    fn invert_GivenHalf_ReturnsTwoNotPeriodic() {
        let result = invert(&r("0.5")).unwrap();
        assert_eq!(result, r("2"));
        assert!(!is_periodic(&result));
    }

    #[test]
    fn invert_GivenMinusTwo_ReturnsMinusHalf() {
        let result = invert(&r("-2.0")).unwrap();
        assert!(is_negative(&result));
        assert_eq!(result, r("-0.5"));
    }

    #[test]
    fn invert_GivenThree_ReturnsRepeatingThird() {
        let result = invert(&r("3")).unwrap();
        assert!(is_periodic(&result));
        assert_eq!(result.to_string(), "0.(3)");
        assert_eq!(result, r("0.(3)"));
    }

    #[test]
    fn invert_GivenZero_ReturnsNone() {
        assert!(invert(&r("0.0")).is_none());
    }

    // RealParseTests.cs ---------------------------------------------------

    #[test]
    fn parse_GivenIntegerString_RendersFortyTwoExponentZero() {
        let value = r("42");
        assert_eq!(value.to_string(), "42");
        assert_eq!(value, r("42"));
    }

    #[test]
    fn parse_GivenNegativeIntegerString_IsNegativeAndRendersMinusSeven() {
        let value = r("-7");
        assert!(is_negative(&value));
        assert_eq!(value.to_string(), "-7");
    }

    #[test]
    fn parse_GivenDecimalString_KeepsTwoFractionalDigits() {
        let value = r("3.14");
        assert_eq!(frac_digits(&value), 2);
        assert_eq!(value.to_string(), "3.14");
    }

    #[test]
    fn parse_GivenDecimalString_RoundTrips() {
        assert_eq!(r("3.14").to_string(), "3.14");
    }

    #[test]
    fn parse_GivenNegativeDecimalString_IsNegativeAndRendersMinusZeroPointZeroZeroOne() {
        let value = r("-0.001");
        assert!(is_negative(&value));
        assert_eq!(frac_digits(&value), 3);
        assert_eq!(value.to_string(), "-0.001");
    }

    #[test]
    fn parse_GivenLeadingZeros_StripsThem() {
        assert_eq!(r("007.5").to_string(), "7.5");
    }

    #[test]
    fn parse_GivenTrailingZeros_NormalizesToOnePointFive() {
        let value = r("1.50");
        assert_eq!(frac_digits(&value), 1);
        assert_eq!(value.to_string(), "1.5");
    }

    #[test]
    fn try_parse_GivenValidDecimalString_ReturnsValue() {
        let parsed = Real::from_str("3.14");
        assert!(parsed.is_some());
        assert_eq!(parsed.unwrap().to_string(), "3.14");
    }

    #[test]
    fn try_parse_GivenInvalidString_ReturnsNone() {
        assert!(Real::from_str("abc").is_none());
    }

    #[test]
    fn parse_GivenSignOnly_ReturnsNone() {
        assert!(Real::from_str("-").is_none());
    }

    #[test]
    fn parse_GivenEmptyString_ReturnsNone() {
        assert!(Real::from_str("").is_none());
    }

    #[test]
    fn parse_GivenPeriodicNotation_RendersZeroPointPeriodThree() {
        let value = r("0.(3)");
        assert!(is_periodic(&value));
        assert_eq!(value.to_string(), "0.(3)");
    }

    #[test]
    fn parse_GivenMixedPeriodic_RendersZeroPointOnePeriodSix() {
        let value = r("0.1(6)");
        assert!(is_periodic(&value));
        assert_eq!(value.to_string(), "0.1(6)");
    }

    #[test]
    fn parse_GivenNegativePeriodic_IsNegativeAndRendersMinusZeroPointPeriod142857() {
        let value = r("-0.(142857)");
        assert!(is_negative(&value));
        assert!(is_periodic(&value));
        assert_eq!(value.to_string(), "-0.(142857)");
    }

    #[test]
    fn try_parse_GivenPeriodicNotation_ReturnsPeriodic() {
        let parsed = Real::from_str("0.(3)").unwrap();
        assert!(is_periodic(&parsed));
    }

    #[test]
    fn parse_GivenUnclosedPeriod_ReturnsNone() {
        assert!(Real::from_str("0.(3").is_none());
    }

    #[test]
    fn parse_GivenEmptyPeriod_ReturnsNone() {
        assert!(Real::from_str("0.()").is_none());
    }

    #[test]
    fn try_parse_GivenSpan_ReturnsValue() {
        let parsed = Real::from_str("2.71828").unwrap();
        assert_eq!(parsed.to_string(), "2.71828");
    }

    #[test]
    fn try_parse_GivenInvalidSpan_ReturnsNone() {
        assert!(Real::from_str("not-a-number").is_none());
    }

    #[test]
    fn parse_GivenSpanDecimal_RendersNinePointNineNine() {
        let value = r("9.99");
        assert_eq!(frac_digits(&value), 2);
        assert_eq!(value.to_string(), "9.99");
    }

    // RealConstructorTests.cs ---------------------------------------------

    #[test]
    fn ctor_default_IsZeroWithExponentZero() {
        let value = Real::zero();
        assert!(value.is_zero());
        assert_eq!(value.to_string(), "0");
    }

    #[test]
    fn ctor_from_string_GivenInteger_RendersFortyTwo() {
        let value = r("42");
        assert_eq!(frac_digits(&value), 0);
        assert_eq!(value, r("42"));
    }

    #[test]
    fn ctor_from_string_GivenDecimal_KeepsTwoFractionalDigits() {
        assert_eq!(frac_digits(&r("3.14")), 2);
    }

    #[test]
    fn ctor_from_string_GivenNegativeDecimal_IsNegative() {
        let value = r("-0.001");
        assert!(is_negative(&value));
        assert_eq!(value, r("-0.001"));
    }

    #[test]
    fn ctor_from_span_GivenInteger_ParsesLikeParse() {
        let value = Real::from_str("42").unwrap();
        assert_eq!(value, r("42"));
        assert_eq!(frac_digits(&value), 0);
    }

    #[test]
    fn ctor_from_span_GivenDecimal_KeepsOneFractionalDigit() {
        let value = Real::from_str("1.5").unwrap();
        assert_eq!(frac_digits(&value), 1);
        assert_eq!(value, r("1.5"));
    }

    #[test]
    fn ctor_from_span_GivenPeriodic_IsPeriodic() {
        let value = Real::from_str("0.(3)").unwrap();
        assert!(is_periodic(&value));
        assert_eq!(value.to_string(), "0.(3)");
    }

    #[test]
    fn ctor_from_span_GivenNegative_IsNegative() {
        let value = Real::from_str("-2.5").unwrap();
        assert!(is_negative(&value));
        assert_eq!(value, r("-2.5"));
    }

    #[test]
    fn ctor_from_span_GivenEmpty_ReturnsNone() {
        assert!(Real::from_str("").is_none());
    }

    #[test]
    fn ctor_from_span_GivenMalformed_ReturnsNone() {
        assert!(Real::from_str("abc").is_none());
    }

    // RealToStringTests.cs ------------------------------------------------

    #[test]
    fn to_string_GivenDefaultZero_ReturnsZero() {
        assert_eq!(Real::zero().to_string(), "0");
    }

    #[test]
    fn to_string_GivenNegativePeriodic_IncludesSign() {
        assert_eq!(r("-0.(142857)").to_string(), "-0.(142857)");
    }

    #[test]
    fn to_string_GivenOneOverZeroPointNine_EmitsOnePointPeriodOne() {
        assert_eq!(r("1").div(&r("0.9"), 39).unwrap().to_string(), "1.(1)");
    }

    #[test]
    fn to_string_GivenOneOverZeroPointNineNine_EmitsOnePointPeriodZeroOne() {
        assert_eq!(r("1").div(&r("0.99"), 39).unwrap().to_string(), "1.(01)");
    }

    #[test]
    fn to_string_GivenTenOverNine_EmitsOnePointPeriodOne() {
        assert_eq!(r("10").div(&r("9"), 39).unwrap().to_string(), "1.(1)");
    }

    #[test]
    fn to_string_GivenOneOverThree_EmitsZeroPointPeriodThree() {
        assert_eq!(r("1").div(&r("3"), 39).unwrap().to_string(), "0.(3)");
    }

    #[test]
    fn to_string_GivenHalfOverThree_EmitsZeroPointOnePeriodSix() {
        assert_eq!(r("0.5").div(&r("3"), 39).unwrap().to_string(), "0.1(6)");
    }

    #[test]
    fn to_string_GivenOneOverZeroPointNine_ParsesBackToAnEqualValue() {
        let quotient = r("1").div(&r("0.9"), 39).unwrap();
        assert_eq!(Real::from_str(&quotient.to_string()).unwrap(), quotient);
    }

    #[test]
    fn to_string_GivenOneOverZeroPointNineNine_ParsesBackToAnEqualValue() {
        let quotient = r("1").div(&r("0.99"), 39).unwrap();
        assert_eq!(Real::from_str(&quotient.to_string()).unwrap(), quotient);
    }

    #[test]
    fn to_string_GivenTenOverNine_ParsesBackToAnEqualValue() {
        let quotient = r("10").div(&r("9"), 39).unwrap();
        assert_eq!(Real::from_str(&quotient.to_string()).unwrap(), quotient);
    }

    #[test]
    fn to_string_GivenOneOverThree_ParsesBackToAnEqualValue() {
        let quotient = r("1").div(&r("3"), 39).unwrap();
        assert_eq!(Real::from_str(&quotient.to_string()).unwrap(), quotient);
    }

    #[test]
    fn to_string_GivenHalfOverThree_ParsesBackToAnEqualValue() {
        let quotient = r("0.5").div(&r("3"), 39).unwrap();
        assert_eq!(Real::from_str(&quotient.to_string()).unwrap(), quotient);
    }

    // RealSqrtTests.cs ----------------------------------------------------

    #[test]
    fn sqrt_GivenPerfectSquareFour_ReturnsExactlyTwo() {
        assert_eq!(Real::from_i64(4).sqrt(1000), r("2"));
    }

    #[test]
    fn sqrt_GivenPerfectSquareNine_ReturnsExactlyThree() {
        assert_eq!(Real::from_i64(9).sqrt(1000), r("3"));
    }

    #[test]
    fn sqrt_GivenOne_ReturnsOne() {
        assert_eq!(Real::from_i64(1).sqrt(1000), r("1"));
    }

    #[test]
    fn sqrt_GivenZero_ReturnsZero() {
        assert_eq!(Real::zero().sqrt(1000), Real::zero());
    }

    #[test]
    fn sqrt_GivenTwo_StartsWithKnownDigits() {
        assert!(
            Real::from_i64(2)
                .sqrt(1000)
                .to_string()
                .starts_with("1.41421356237")
        );
    }

    #[test]
    fn sqrt_GivenTwo_IsNotPeriodic() {
        assert!(!is_periodic(&Real::from_i64(2).sqrt(1000)));
    }

    #[test]
    fn sqrt_GivenTwo_IsPositive() {
        assert!(is_positive(&Real::from_i64(2).sqrt(1000)));
    }

    #[test]
    fn sqrt_GivenPerfectSquareFour_RemainsExactWithGuardDigits() {
        assert_eq!(Real::from_i64(4).sqrt(1000), r("2"));
    }

    #[test]
    fn sqrt_GivenPerfectSquareNine_RemainsExactWithGuardDigits() {
        assert_eq!(Real::from_i64(9).sqrt(1000), r("3"));
    }

    #[test]
    fn sqrt_GivenPerfectSquareSixteen_RemainsExactWithGuardDigits() {
        assert_eq!(Real::from_i64(16).sqrt(1000), r("4"));
    }

    #[test]
    fn sqrt_GivenPerfectSquareTwentyFive_RemainsExactWithGuardDigits() {
        assert_eq!(Real::from_i64(25).sqrt(1000), r("5"));
    }

    #[test]
    fn sqrt_GivenQuarter_ReturnsExactlyHalf() {
        assert_eq!(r("0.25").sqrt(1000), r("0.5"));
    }

    #[test]
    fn sqrt_GivenTwo_Matches1000KnownDigits() {
        let text = Real::from_i64(2).sqrt(1000).to_string();
        assert!(text.starts_with("1.41421356237309504880168872420969807856967187537694"));
        assert!(frac_digits(&Real::from_i64(2).sqrt(1000)) <= 1000);
    }

    #[test]
    fn sqrt_GivenThree_Matches1000KnownDigits() {
        let text = Real::from_i64(3).sqrt(1000).to_string();
        assert!(text.starts_with("1.73205080756887729352744634150587236694280525381038"));
        assert!(frac_digits(&Real::from_i64(3).sqrt(1000)) <= 1000);
    }

    #[test]
    fn sqrt_GivenFive_Matches1000KnownDigits() {
        let text = Real::from_i64(5).sqrt(1000).to_string();
        assert!(text.starts_with("2.23606797749978969640917366873127623544061835961152"));
        assert!(frac_digits(&Real::from_i64(5).sqrt(1000)) <= 1000);
    }

    #[test]
    fn sqrt_GivenTen_Matches1000KnownDigits() {
        let text = Real::from_i64(10).sqrt(1000).to_string();
        assert!(text.starts_with("3.16227766016837933199889354443271853371955513932521"));
        assert!(frac_digits(&Real::from_i64(10).sqrt(1000)) <= 1000);
    }

    #[test]
    fn sqrt_GivenTwo_HasExactly1000FractionalDigits() {
        assert_eq!(frac_digits(&Real::from_i64(2).sqrt(1000)), 1000);
    }

    #[test]
    fn sqrt_GivenTwo_SquaredIsWithinOneE999() {
        let value = Real::from_i64(2);
        let root = value.sqrt(1000);
        let squared = root.mul(&root);
        let diff = squared.sub(&value).abs();
        let tolerance = r(&format!("0.{}1", "0".repeat(998)));
        assert!(diff.cmp(&tolerance) == Ordering::Less);
    }

    #[test]
    fn sqrt_GivenTwo_IsNotPeriodicAt1000Digits() {
        assert!(!is_periodic(&Real::from_i64(2).sqrt(1000)));
    }

    #[test]
    fn sqrt_GivenTwo_IsPositiveAt1000Digits() {
        assert!(is_positive(&Real::from_i64(2).sqrt(1000)));
    }

    // RealPiTests.cs ------------------------------------------------------

    #[test]
    fn pi_GivenOneDigit_ReturnsThreePointOne() {
        assert_eq!(Real::pi(1).to_string(), "3.1");
    }

    #[test]
    fn pi_GivenTenDigits_ReturnsKnownPrefix() {
        assert_eq!(Real::pi(10).to_string(), "3.1415926535");
    }

    #[test]
    fn pi_GivenFiftyDigits_MatchesReference() {
        assert_eq!(
            Real::pi(50).to_string(),
            "3.14159265358979323846264338327950288419716939937510"
        );
    }

    #[test]
    fn pi_GivenTenDigits_IsPositive() {
        assert!(is_positive(&Real::pi(10)));
    }

    #[test]
    fn pi_GivenTenDigits_IsNotPeriodic() {
        assert!(!is_periodic(&Real::pi(10)));
    }

    #[test]
    fn pi_GivenTenDigits_KeepsTenFractionalDigits() {
        assert_eq!(frac_digits(&Real::pi(10)), 10);
    }

    #[test]
    fn pi_GivenConcurrentCalls_AllReturnTheSameValue() {
        let handles: Vec<_> = (0..8)
            .map(|_| std::thread::spawn(|| Real::pi(10).to_string()))
            .collect();
        for handle in handles {
            assert_eq!(handle.join().unwrap(), "3.1415926535");
        }
    }

    // RealPowTests.cs -----------------------------------------------------

    #[test]
    fn pow_GivenTwoToTheThree_ReturnsEight() {
        assert_eq!(r("2.0").pow_int(3).unwrap(), r("8"));
    }

    #[test]
    fn pow_GivenNineNinetyNineToTheZero_ReturnsOne() {
        assert_eq!(r("999.0").pow_int(0).unwrap(), r("1"));
    }

    #[test]
    fn pow_GivenSevenPointFiveToOne_ReturnsSelf() {
        assert_eq!(r("7.5").pow_int(1).unwrap(), r("7.5"));
    }

    #[test]
    fn pow_GivenZeroToTheFive_ReturnsZero() {
        assert!(Real::zero().pow_int(5).unwrap().is_zero());
    }

    #[test]
    fn pow_GivenMinusTwoSquared_ReturnsFour() {
        let result = r("-2.0").pow_int(2).unwrap();
        assert_eq!(result, r("4"));
        assert!(!is_negative(&result));
    }

    // RealPowZeroBaseTests.cs ---------------------------------------------

    #[test]
    fn pow_zeroBase_GivenExponentOne_IsZero() {
        assert!(Real::zero().pow_int(1).unwrap().is_zero());
    }

    #[test]
    fn pow_zeroBase_GivenExponentTwo_IsZero() {
        assert!(Real::zero().pow_int(2).unwrap().is_zero());
    }

    #[test]
    fn pow_zeroBase_GivenExponentFive_IsZero() {
        assert!(Real::zero().pow_int(5).unwrap().is_zero());
    }

    // RealPropertiesTests.cs ----------------------------------------------

    #[test]
    fn properties_defaultReal_IsZero() {
        assert!(Real::zero().is_zero());
    }

    #[test]
    fn properties_defaultReal_IsNotNegative() {
        assert!(!is_negative(&Real::zero()));
    }

    #[test]
    fn properties_defaultReal_RendersZero() {
        assert_eq!(Real::zero().to_string(), "0");
    }

    // RealDivideTests.cs --------------------------------------------------

    #[test]
    fn div_GivenSixByTwo_ReturnsExactThree() {
        let quotient = r("6.0").div(&r("2.0"), 1000).unwrap();
        assert!(!is_periodic(&quotient));
        assert_eq!(quotient, r("3"));
    }

    #[test]
    fn div_GivenOneByThree_ReturnsRepeatingThird() {
        let quotient = r("1").div(&r("3"), 1000).unwrap();
        assert!(is_periodic(&quotient));
        assert_eq!(quotient.to_string(), "0.(3)");
        assert_eq!(quotient, r("0.(3)"));
    }

    #[test]
    fn div_GivenOneBySeven_ReturnsPeriod142857() {
        let quotient = r("1").div(&r("7"), 1000).unwrap();
        assert_eq!(quotient.to_string(), "0.(142857)");
        assert_eq!(quotient, r("0.(142857)"));
    }

    #[test]
    fn div_GivenOneBySix_ReturnsMixedPeriod() {
        let quotient = r("1").div(&r("6"), 1000).unwrap();
        assert_eq!(quotient.to_string(), "0.1(6)");
        assert_eq!(quotient, r("0.1(6)"));
    }

    #[test]
    fn div_GivenNegativeSixByTwo_ReturnsMinusThree() {
        let quotient = r("-6.0").div(&r("2.0"), 1000).unwrap();
        assert!(is_negative(&quotient));
        assert_eq!(quotient, r("-3"));
    }

    #[test]
    fn div_GivenSixByNegativeTwo_ReturnsMinusThree() {
        let quotient = r("6.0").div(&r("-2.0"), 1000).unwrap();
        assert!(is_negative(&quotient));
        assert_eq!(quotient, r("-3"));
    }

    #[test]
    fn div_GivenBothNegative_ReturnsThree() {
        let quotient = r("-6.0").div(&r("-2.0"), 1000).unwrap();
        assert!(!is_negative(&quotient));
        assert_eq!(quotient, r("3"));
    }

    #[test]
    fn div_GivenZeroDivisor_ReturnsNone() {
        assert!(r("1.0").div(&r("0.0"), 1000).is_none());
    }

    #[test]
    fn div_GivenMinusOneByThree_IsNegativeAndPeriodic() {
        let quotient = r("-1").div(&r("3"), 1000).unwrap();
        assert!(is_negative(&quotient));
        assert!(is_periodic(&quotient));
        assert_eq!(quotient, r("-0.(3)"));
    }

    #[test]
    fn div_GivenRepeatingThirdByThree_ReturnsRepeatingNinth() {
        assert_eq!(r("0.(3)").div(&r("3"), 1000).unwrap(), r("0.(1)"));
    }

    #[test]
    fn div_GivenOneByRepeatingNinth_ReturnsNine() {
        assert_eq!(r("1").div(&r("0.(1)"), 1000).unwrap(), r("9"));
    }

    #[test]
    fn div_GivenOneByRepeatingThird_ReturnsThree() {
        let quotient = r("1").div(&r("0.(3)"), 1000).unwrap();
        assert_eq!(quotient, r("3"));
        assert!(!is_periodic(&quotient));
    }

    #[test]
    fn div_GivenOneByZeroPointNine_ReturnsOnePointPeriodOne() {
        assert_eq!(r("1").div(&r("0.9"), 39).unwrap(), r("1.(1)"));
    }

    #[test]
    fn div_GivenTenByNine_ReturnsOnePointPeriodOne() {
        assert_eq!(r("10").div(&r("9"), 39).unwrap(), r("1.(1)"));
    }

    #[test]
    fn div_GivenOneByZeroPointNineNine_ReturnsOnePointPeriodZeroOne() {
        assert_eq!(r("1").div(&r("0.99"), 39).unwrap(), r("1.(01)"));
    }

    #[test]
    fn div_GivenHalfByThree_ReturnsZeroPointOnePeriodSix() {
        assert_eq!(r("0.5").div(&r("3"), 39).unwrap(), r("0.1(6)"));
    }

    #[test]
    fn div_GivenOneByZeroPointNine_StoresElevenAtScaleOne() {
        let quotient = r("1").div(&r("0.9"), 39).unwrap();
        assert_eq!(quotient.to_string(), "1.(1)");
    }

    #[test]
    fn div_GivenOneByZeroPointNineNine_StoresOneHundredOneAtScaleTwo() {
        let quotient = r("1").div(&r("0.99"), 39).unwrap();
        assert_eq!(quotient.to_string(), "1.(01)");
    }

    // -----------------------------------------------------------------------
    // Round 13 robustness — exact terminating quotients and the digit budget.
    //
    // The oracle's rule: a quotient over a denominator whose reduced form has no prime
    // factor other than 2 and 5 terminates, and is exact at EVERY digit budget; a
    // non-terminating quotient stores exactly the budget and is the one that says
    // "inexact". The old loop stored budget+1 digits (a guard digit) and marked the
    // terminating value inexact, which is the defect these rows pin.
    // -----------------------------------------------------------------------

    #[test]
    fn div_round13_exact_terminating_quotients_are_exact_with_no_guard_digit() {
        // The four oracle probes at a 2-place budget: 1/4, 1/8, 10/3 and 1/3.
        // 1/4 and 1/8 terminate; 10/3 and 1/3 carry a period and are exactly representable too.
        for (num, den, text, numerator, denominator) in [
            (1i64, 4i64, "0.25", "1", "4"),
            (1, 8, "0.125", "1", "8"),
            (10, 3, "3.(3)", "10", "3"),
            (1, 3, "0.(3)", "1", "3"),
        ] {
            let quotient = Real::from_i64(num).div(&Real::from_i64(den), 2).unwrap();
            assert!(
                quotient.is_exact(),
                "{num}/{den} at 2 places was marked inexact"
            );
            assert_eq!(quotient.to_string(), text, "{num}/{den} display");
            let (n, d) = quotient.to_rational();
            assert_eq!(n.to_decimal(), numerator, "{num}/{den} numerator");
            assert_eq!(d.to_decimal(), denominator, "{num}/{den} denominator");
        }
    }

    #[test]
    fn div_round13_terminating_quotient_longer_than_the_budget_stays_exact() {
        // 1/2048 = 0.00048828125: eleven fractional digits. At a ten-digit budget the old
        // code stored eleven digits (ten plus a guard) and reported inexact; the oracle's
        // value is the exact terminating quotient, so the budget must not touch it.
        let quotient = Real::from_i64(1).div(&Real::from_i64(2048), 10).unwrap();
        assert!(
            quotient.is_exact(),
            "1/2048 at 10 places was marked inexact"
        );
        assert_eq!(quotient.to_string(), "0.00048828125");
        assert_eq!(
            quotient.to_rational(),
            (Int::from_i64(1), Nat::from_u64(2048))
        );
    }

    #[test]
    fn div_round13_nonterminating_quotient_stores_exactly_the_budget() {
        // 1/291 = 0.0034364...: the period does not fit in ten digits, so the stored value is
        // the ten-digit truncation (NOT eleven), and it is inexact — the one legitimate use of
        // the flag. This is the control that keeps the terminating rule from widening exactness.
        let quotient = Real::from_i64(1).div(&Real::from_i64(291), 10).unwrap();
        assert!(
            !quotient.is_exact(),
            "1/291 at 10 places was reported exact"
        );
        assert_eq!(
            quotient.exp, -10,
            "stored {} digits, budget was 10",
            -quotient.exp
        );
        assert_eq!(quotient.to_string(), "0.0034364261");
    }

    #[test]
    fn div_round13_csharp_divisor_with_fractional_digits_rows() {
        // C# RealDivideTests.Divide_GivenDivisorWithFractionalDigits_ReturnsTheExactQuotient
        // (WithPrecision(39, 40)) and Divide_GivenQuotientSmallerThanItsScale_KeepsTheSignificant
        // Digits (WithPrecision(20, 40)). These are the three rows Round 12 refused because the
        // extra guard digit made the stored value one digit longer than the reference's.
        let long = r("1")
            .div(&r("0.9993142073433579945"), 39)
            .expect("division");
        assert_eq!(long, r("1.000686263290967473969590465527195755929"));
        assert_eq!(long.exp, -39, "39 places must store 39 digits, not 40");

        let scaled = r("1")
            .div(&r("0.99862888499828389309965043538706203025"), 20)
            .expect("division");
        assert_eq!(scaled, r("1.00137299753923947744"));
        assert_eq!(scaled.exp, -20, "20 places must store 20 digits, not 21");
    }

    #[test]
    fn div_round13_exact_long_terminating_quotient_is_not_truncated_by_the_default_budget() {
        // 1/(2^4000) has a 4000-digit terminating expansion: it is exact, so it is never
        // truncated (the old loop stopped at budget+1 = 1001 digits and marked it inexact).
        let divisor = Nat::from_u64(2).pow(4000);
        let quotient = Real::from_fraction(&Nat::from_u64(1), &divisor, 1000).unwrap();
        assert!(quotient.is_exact(), "1/2^4000 was marked inexact");
        let (numerator, denominator) = quotient.to_rational();
        assert_eq!(numerator.to_decimal(), "1");
        assert_eq!(denominator, divisor);
    }

    // RealPeriodicIntegerValuePropertyTests.cs ----------------------------

    fn assert_is_the_integer(literal: &str, value: Real, expected: Real) {
        assert!(value == expected, "'{literal}' == {expected} was false");
        assert!(expected == value);
        assert!(!(value != expected));
        assert!(!(value < expected));
        assert!(!(value > expected));
        assert!(value <= expected);
        assert!(value >= expected);
        assert_eq!(value.cmp(&expected), Ordering::Equal);
        let difference = value.sub(&expected);
        assert!(
            difference.is_zero(),
            "'{literal}' - {expected} = {difference}"
        );
        assert!(
            difference.is_exact(),
            "'{literal}' - {expected} lost exactness"
        );
        assert!(value.add(&value) == expected.add(&expected));
        assert!(value.mul(&Real::from_i64(3)) == expected.mul(&Real::from_i64(3)));
    }

    #[test]
    fn periodicLiteralOfNines_IsTheNextInteger() {
        for prefix in [0i64, 1, 2, 7, 12, 123, 9999] {
            for p in [1usize, 2, 3, 5, 8] {
                let literal = format!("{}.({})", prefix, "9".repeat(p));
                let expected = Real::from_i64(prefix + 1);
                assert_is_the_integer(&literal, r(&literal), expected.clone());
                let negated = format!("-{}", literal);
                assert_is_the_integer(&negated, r(&negated), expected.neg());
            }
        }
    }

    #[test]
    fn periodicUnit_AddedToAnInteger_IsTheNextInteger() {
        let unit = r("0.(9)");
        for k in 0i64..=8 {
            let sum = Real::from_i64(k).add(&unit);
            let expected = Real::from_i64(k + 1);
            assert_is_the_integer(&format!("{k} + 0.(9)"), sum.clone(), expected);
            assert!(sum == r(&format!("{k}.(9)")));
            assert!(sum == unit.mul(&Real::from_i64(k + 1)));
        }
    }

    #[test]
    fn periodicInteger_OrdersAgainstItsNeighbours() {
        for prefix in [0i64, 1, 2, 7, 12, 123, 9999] {
            for p in [1usize, 2, 3, 5, 8] {
                let value = r(&format!("{}.({})", prefix, "9".repeat(p)));
                let below = Real::from_i64(prefix).sub(&r("0.0000001"));
                let above = Real::from_i64(prefix + 1).add(&r("0.0000001"));
                assert!(below < value);
                assert!(value < above);
                assert!(value > below && value < above);
            }
        }
    }

    // RealSqrtPeriodicTests.cs --------------------------------------------

    #[test]
    fn sqrt_GivenOneNinthAsPeriodic_StartsWithOneThird() {
        assert!(r("0.(1)").sqrt(50).to_string().starts_with("0.3333333"));
    }

    #[test]
    fn sqrt_GivenFourNinthsAsPeriodic_StartsWithTwoThirds() {
        assert!(r("0.(4)").sqrt(50).to_string().starts_with("0.6666666"));
    }

    #[test]
    fn sqrt_GivenOneThirdAsPeriodic_MatchesSqrtOneThirdKnownDigits() {
        assert!(r("0.(3)").sqrt(50).to_string().starts_with("0.57735026"));
    }

    #[test]
    fn sqrt_GivenPeriodicAndExpandedEquivalent_ProduceSameResult() {
        assert!(r("0.(3)").sqrt(50).to_string().starts_with("0.57735026"));
        assert!(
            r("0.3333333333")
                .sqrt(50)
                .to_string()
                .starts_with("0.57735026")
        );
    }

    #[test]
    fn sqrt_GivenTwoNinthsAsPeriodic_MatchesSqrtTwoNinthsKnownDigits() {
        assert!(r("0.(2)").sqrt(50).to_string().starts_with("0.47140452"));
    }

    // RealSqrtSeedingTests.cs ---------------------------------------------

    #[test]
    fn sqrt_GivenVerySmallNumber_ConvergesCorrectly() {
        let value = r("0.0000000000000000000000000001");
        assert!(value.sqrt(1000).to_string().starts_with("0.00000000000001"));
    }

    #[test]
    fn sqrt_GivenVerySmallNumber_KeepsExponentMinusFourteen() {
        let value = r("0.0000000000000000000000000001");
        assert_eq!(frac_digits(&value.sqrt(1000)), 14);
    }

    #[test]
    fn sqrt_GivenNormalInput_NewSeedingProducesSameResult() {
        assert!(r("2").sqrt(1000).to_string().starts_with("1.41421356237"));
    }

    // RealSqrtAsyncTests.cs -----------------------------------------------

    #[test]
    fn sqrt_async_GivenPerfectSquareFour_ReturnsExactlyTwo() {
        assert_eq!(r("4").sqrt(1000), r("2"));
    }

    #[test]
    fn sqrt_async_GivenConcurrentInputs_AllReturnCorrectValues() {
        let sqrt_two = r("2").sqrt(20).to_string();
        let sqrt_three = r("3").sqrt(20).to_string();
        assert!(sqrt_two.starts_with("1.41421356237"));
        assert!(sqrt_three.starts_with("1.73205080756"));
    }

    // RealPiAsyncTests.cs -------------------------------------------------

    #[test]
    fn pi_async_GivenOneDigit_ReturnsCorrectValue() {
        assert_eq!(Real::pi(1).to_string(), "3.1");
    }

    #[test]
    fn pi_async_GivenConcurrentCalls_AllReturnCorrectValues() {
        let first = Real::pi(10).to_string();
        let second = Real::pi(10).to_string();
        assert_eq!(first, "3.1415926535");
        assert_eq!(second, "3.1415926535");
    }

    // ---------------------------------------------------------------------
    // Corpus properties (RealAddPeriodicExactTests.cs:91,126;
    // RealMultiplyPeriodicExactTests.cs:86)
    // ---------------------------------------------------------------------
    // RealMultiplyPeriodicExactTests.cs audit shapes and facts --------

    fn assert_mul_audit(p: i64, divisor: &str) {
        let q = r(divisor);
        let quotient = Real::from_i64(p).div(&q, 1000).unwrap();
        let product = quotient.mul(&q);
        assert!(product == Real::from_i64(p));
        let (n, d) = product.to_rational();
        assert!(n == Int::from_i64(p) && d == Nat::from_u64(1));
    }

    #[test]
    fn mul_audit_GivenOneOverSeventeen_IsExactlyOne() {
        assert_mul_audit(1, "17");
    }

    #[test]
    fn mul_audit_GivenTwoOverSeventeen_IsExactlyTwo() {
        assert_mul_audit(2, "17");
    }

    #[test]
    fn mul_audit_GivenOneOverZeroPointSevenFive_IsExactlyOne() {
        assert_mul_audit(1, "0.75");
    }

    #[test]
    fn mul_audit_GivenSevenOverZeroPointNineFive_IsExactlySeven() {
        assert_mul_audit(7, "0.95");
    }

    #[test]
    fn mul_oneOverSeventeenTimesSeventeen_IsExactlyOne_AndStillRendersItsPeriod() {
        let seventeenth = r("1").div(&r("17"), 1000).unwrap();
        assert_eq!(seventeenth.to_string(), "0.(0588235294117647)");
        let product = seventeenth.mul(&r("17"));
        assert!(product == Real::from_i64(1));
        assert_eq!(product.to_string(), "1");
        assert_eq!(product.cmp(&Real::from_i64(1)), Ordering::Equal);
    }

    #[test]
    fn mul_fourThirdsTimesThreeQuarters_IsExactlyOne() {
        let four_thirds = r("4").div(&r("3"), 1000).unwrap();
        let three_quarters = r("3").div(&r("4"), 1000).unwrap();
        assert!(is_periodic(&four_thirds));
        assert!(!is_periodic(&three_quarters));
        let product = four_thirds.mul(&three_quarters);
        assert!(product == Real::from_i64(1));
        assert_eq!(product.to_string(), "1");
    }

    #[test]
    fn mul_thirdTimesTwoThirds_IsExactlyZeroPointTwoPeriod() {
        let third = r("1").div(&r("3"), 1000).unwrap();
        let two_thirds = r("2").div(&r("3"), 1000).unwrap();
        let product = third.mul(&two_thirds);
        assert_eq!(product.to_string(), "0.(2)");
        assert_eq!(product.to_rational(), (Int::from_i64(2), Nat::from_u64(9)));
        let one_over_twenty_one = third.mul(&r("1").div(&r("7"), 1000).unwrap());
        assert_eq!(one_over_twenty_one.to_string(), "0.(047619)");
        assert_eq!(
            one_over_twenty_one.to_rational(),
            (Int::from_i64(1), Nat::from_u64(21))
        );
    }

    // RealAddPeriodicExactTests.cs:91,126 — the nine add and nine subtract corpus rows at
    // q = 19, s = 17 (census #582-#584, #606-#608, #630-#632). The exact-fraction fix makes each
    // row denote the exact rational AND compare equal to the Real built from that fraction; the
    // same two assertions the generated corpus rows already carry.

    #[test]
    fn add_p1_q19_r1_s17() {
        check_add_row(1, 19, 1, 17);
    }

    #[test]
    fn add_p1_q19_rm1_s17() {
        check_add_row(1, 19, -1, 17);
    }

    #[test]
    fn add_p1_q19_r5_s17() {
        check_add_row(1, 19, 5, 17);
    }

    #[test]
    fn add_p2_q19_r1_s17() {
        check_add_row(2, 19, 1, 17);
    }

    #[test]
    fn add_p2_q19_rm1_s17() {
        check_add_row(2, 19, -1, 17);
    }

    #[test]
    fn add_p2_q19_r5_s17() {
        check_add_row(2, 19, 5, 17);
    }

    #[test]
    fn add_pm5_q19_r1_s17() {
        check_add_row(-5, 19, 1, 17);
    }

    #[test]
    fn add_pm5_q19_rm1_s17() {
        check_add_row(-5, 19, -1, 17);
    }

    #[test]
    fn add_pm5_q19_r5_s17() {
        check_add_row(-5, 19, 5, 17);
    }

    #[test]
    fn sub_p1_q19_r1_s17() {
        check_sub_row(1, 19, 1, 17);
    }

    #[test]
    fn sub_p1_q19_rm1_s17() {
        check_sub_row(1, 19, -1, 17);
    }

    #[test]
    fn sub_p1_q19_r5_s17() {
        check_sub_row(1, 19, 5, 17);
    }

    #[test]
    fn sub_p2_q19_r1_s17() {
        check_sub_row(2, 19, 1, 17);
    }

    #[test]
    fn sub_p2_q19_rm1_s17() {
        check_sub_row(2, 19, -1, 17);
    }

    #[test]
    fn sub_p2_q19_r5_s17() {
        check_sub_row(2, 19, 5, 17);
    }

    #[test]
    fn sub_pm5_q19_r1_s17() {
        check_sub_row(-5, 19, 1, 17);
    }

    #[test]
    fn sub_pm5_q19_rm1_s17() {
        check_sub_row(-5, 19, -1, 17);
    }

    #[test]
    fn sub_pm5_q19_r5_s17() {
        check_sub_row(-5, 19, 5, 17);
    }

    #[test]
    fn pow_zeroBase_GivenNegativeExponent_DoesNotReturnZero() {
        assert!(Real::zero().pow_int(-1).is_none());
    }
}
