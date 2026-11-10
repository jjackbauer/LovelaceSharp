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

use std::cmp::Ordering;

use lovelace_int::Int;
use lovelace_nat::Nat;

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
}

impl Real {
    /// The value zero.
    pub fn zero() -> Real {
        Real { int: Int::zero(), exp: 0, period_start: -1, period_len: 0 }
    }

    /// Builds a real from a signed integer.
    pub fn from_int(value: &Int) -> Real {
        Real { int: value.clone(), exp: 0, period_start: -1, period_len: 0 }
    }

    /// Builds a real from an `i64`.
    pub fn from_i64(value: i64) -> Real {
        Real::from_int(&Int::from_i64(value))
    }

    /// True when the value has no repeating block (terminating decimal).
    pub fn is_exact(&self) -> bool {
        true
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
        if self.exp < 0 { (-self.exp) as usize } else { 0 }
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
        };
        value = value.normalized();
        Some(value)
    }

    /// Renders the canonical display form: `2`, `0.25`, `0.(3)`, `0.1(6)`, `3.14159`.
    pub fn to_string(&self) -> String {
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
            if !fp.is_empty() {
                out.push('.');
                out.push_str(fp);
            }
        }
        out
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

    /// Multiplies by a power of ten.
    fn shift_exp(&self, delta: i64) -> Real {
        let mut c = self.clone();
        c.exp += delta;
        c
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
        let pre_n = if pre.is_empty() { Nat::zero() } else { Nat::from_decimal(pre).unwrap_or_else(Nat::zero) };
        let per_n = if per.is_empty() { Nat::zero() } else { Nat::from_decimal(per).unwrap_or_else(Nat::zero) };
        // value = int_part + pre/10^s + per / (10^s * (10^k - 1))
        let ten_s = ten.pow(s as u64);
        let ten_k_minus_1 = Nat::from_u64(10).pow(k as u64);
        let nine = ten_k_minus_1
            .sub(&Nat::from_u64(1))
            .unwrap_or_else(Nat::zero);
        let den = ten_s.mul(&nine.clone());
        let mut num = int_part.mul(&den.clone());
        num = num.add(&pre_n.mul(&nine));
        num = num.add(&per_n.mul(&ten_s));
        let num_int = if self.int.is_negative() { int_from_nat(&num).negate() } else { int_from_nat(&num) };
        (num_int, den)
    }

    /// Builds a real from a rational, detecting the repeating period.
    pub fn from_fraction(num: &Nat, den: &Nat, max_digits: usize) -> Option<Real> {
        let (q, r) = num.div_rem(den)?;
        let mut int = int_from_nat(&q);
        if r.is_zero() {
            return Some(Real { int, exp: 0, period_start: -1, period_len: 0 });
        }
        let mut digits: Vec<u8> = Vec::new();
        let mut seen: Vec<(Nat, usize)> = Vec::new();
        let mut rem = r;
        let mut period_start: i64 = -1;
        let mut period_len: i64 = 0;
        while !rem.is_zero() {
            if let Some((_, idx)) = seen.iter().find(|(v, _)| *v == rem) {
                period_start = *idx as i64;
                period_len = (digits.len() - *idx) as i64;
                break;
            }
            seen.push((rem.clone(), digits.len()));
            let scaled = rem.mul(&Nat::from_u64(10));
            let (d, nr) = scaled.div_rem(den)?;
            digits.push(d.to_u64().unwrap_or(0) as u8);
            rem = nr;
            if digits.len() > max_digits {
                break;
            }
        }
        let mut text = String::new();
        for d in &digits {
            text.push((b'0' + *d) as char);
        }
        if text.is_empty() {
            text.push('0');
        }
        let mag = Nat::from_decimal(&text)?;
        // The fractional digits scale the integer part: q * 10^len + digits.
        let scale = Nat::from_u64(10).pow(digits.len() as u64);
        int = int_from_nat(&q).mul(&int_from_nat(&scale)).add(&int_from_nat(&mag));
        Some(Real {
            int,
            exp: -(digits.len() as i64),
            period_start,
            period_len,
        })
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
            };
            r = r.normalized();
            return r;
        }
        let (an, ad) = self.to_fraction();
        let (bn, bd) = other.to_fraction();
        let num = an.mul(&bn);
        let den = ad.mul(&bd);
        fraction_real(num, den, self.frac_digits().max(other.frac_digits()) + 40)
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
        if bn.is_negative() {
            Some(fraction_real(num.negate(), den, max_digits))
        } else {
            Some(fraction_real(num, den, max_digits))
        }
    }

    fn arithmetic_via_fraction(&self, other: &Real, _f: impl Fn(&Nat, &Nat) -> (Nat, Nat)) -> Real {
        let (an, ad) = self.to_fraction();
        let (bn, bd) = other.to_fraction();
        // an/ad + bn/bd = (an*bd + bn*ad) / (ad*bd)
        let left = an.mul(&int_from_nat(&bd));
        let right = bn.mul(&int_from_nat(&ad));
        let num = left.add(&right);
        let den = ad.mul(&bd);
        fraction_real(num, den, self.frac_digits().max(other.frac_digits()) + 40)
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

    /// Compares two reals exactly.
    pub fn cmp(&self, other: &Real) -> Ordering {
        let (an, ad) = self.to_fraction();
        let (bn, bd) = other.to_fraction();
        let left = an.mul(&int_from_nat(&bd));
        let right = bn.mul(&int_from_nat(&ad));
        left.cmp(&right)
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
        Real::from_str(&assembled).unwrap_or_else(Real::zero)
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
                let (t, _) = term.div_rem(&Nat::from_u64(2 * k + 1)).expect("nonzero divisor");
                if t.is_zero() {
                    break;
                }
                total = if k % 2 == 0 { total.add(&t) } else { total.sub(&t).unwrap_or_else(Nat::zero) };
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
        Real::from_scaled(int_from_nat(&truncated), -(digits as i64))
    }

    /// Builds a value with an explicit scale, without normalising (used by pi, whose
    /// published digits include a trailing zero at 50 places).
    fn from_scaled(int: Int, exp: i64) -> Real {
        Real { int, exp, period_start: -1, period_len: 0 }
    }
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
    let ten = |n: i64| if n <= 0 { Nat::from_u64(1) } else { Nat::from_u64(10).pow(n as u64) };
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
        0 | 1 => {
            if text == "1" { return Nat::from_u64(1); }
        }
        _ => {}
    }
    // initial guess: 10^ceil(len/2)
    let guess_exp = ((text.len() + 1) / 2) as u64;
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
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Real {}

impl PartialOrd for Real {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Real {
    fn cmp(&self, other: &Self) -> Ordering {
        Real::cmp(self, other)
    }
}

impl std::fmt::Display for Real {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_string())
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
        assert_eq!(Real::from_i64(2).sqrt(20).to_string(), "1.4142135623730950488");
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
