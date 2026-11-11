//! Fixed-width exact-decimal reals: LReal64 (64-bit significand, up to 19 significant digits)
//! and LReal128 (128-bit significand, up to 38), the Rust port of Lovelace.Real.LReal64 and
//! LReal128.
//!
//! The value is significand * 10^exponent plus optional period metadata, exactly like the
//! arbitrary-precision [Real]; the difference is CAPACITY. An operation whose result needs more
//! significand digits than the width carries (or, for a quotient, a period longer than the width's
//! significant-digit count) is REFUSED with [LRealError::Promote] rather than rounded: the caller
//! promotes to [Real]. The algorithm mirrors the C# reference statement for statement, including
//! the fixed working expansion of a periodic operand and the period re-detection after an
//! operation, so the observable value (and its rendering) agrees with Real.

use std::cmp::Ordering;
use std::fmt;
use std::marker::PhantomData;

use lovelace_nat::Nat;

use crate::Real;

/// The capacity of a fixed-width real: the largest significand the width carries, the working
/// fractional digits used when a periodic operand must be written out, and the significant-digit
/// count the quotient period bound is stated in.
pub trait LRealWidth: Copy {
    /// Largest significand magnitude the width carries.
    const MAX: u128;
    /// Fractional places a periodic operand is expanded to before an operation.
    const WORKING_FRACTIONAL_DIGITS: usize;
    /// The width's significant-digit count (19 for LReal64, 38 for LReal128).
    const MAX_SIGNIFICANT_DIGITS: usize;
    /// Long-division iteration bound.
    const MAX_LOOP: usize;
    /// The type's own name, used in refusal text.
    const NAME: &'static str;
}

/// 64-bit significand width.
#[derive(Clone, Copy, Debug)]
pub struct W64;
/// 128-bit significand width.
#[derive(Clone, Copy, Debug)]
pub struct W128;

impl LRealWidth for W64 {
    const MAX: u128 = u64::MAX as u128;
    const WORKING_FRACTIONAL_DIGITS: usize = 18;
    const MAX_SIGNIFICANT_DIGITS: usize = 19;
    const MAX_LOOP: usize = 256;
    const NAME: &'static str = "LReal64";
}

impl LRealWidth for W128 {
    const MAX: u128 = u128::MAX;
    const WORKING_FRACTIONAL_DIGITS: usize = 37;
    const MAX_SIGNIFICANT_DIGITS: usize = 38;
    const MAX_LOOP: usize = 512;
    const NAME: &'static str = "LReal128";
}

/// A fixed-width exact decimal real with a 64-bit significand (up to 19 significant digits).
pub type LReal64 = Fixed<W64>;
/// A fixed-width exact decimal real with a 128-bit significand (up to 38 significant digits).
pub type LReal128 = Fixed<W128>;

/// The display precision the reference uses for non-periodic renderings. The C# tests set the
/// fixed-width type's display precision to Real.DisplayDecimalPlaces, which defaults to 100.
pub const DEFAULT_DISPLAY_DECIMAL_PLACES: usize = 100;

/// Why a fixed-width operation could not produce a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LRealError {
    /// The result needs more significand digits (or a longer period) than the width carries:
    /// promote to [Real] instead of rounding.
    Promote,
    /// The text is not a decimal representation this type can hold.
    Parse,
    /// The divisor was zero.
    DivideByZero,
}

impl fmt::Display for LRealError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LRealError::Promote => write!(
                f,
                "the result requires more significant digits than the fixed width carries; promote to Real."
            ),
            LRealError::Parse => write!(f, "not a valid decimal representation of a Real number."),
            LRealError::DivideByZero => write!(f, "cannot divide by zero."),
        }
    }
}

impl std::error::Error for LRealError {}

/// A fixed-width exact-decimal real.
#[derive(Clone, Copy, Debug)]
pub struct Fixed<W: LRealWidth> {
    /// Stored digits (the integer part followed by the non-repeating digits and one period block).
    sig: u128,
    /// Decimal exponent: the value is sig * 10^exp.
    exp: i64,
    /// Index of the first repeating fractional digit, meaningful when p_len > 0.
    p_start: i64,
    /// Length of the repeating block; 0 means the expansion terminates.
    p_len: i64,
    /// Sign; a zero is never negative.
    neg: bool,
    /// Provenance: true when a truncation produced these digits.
    inexact: bool,
    _width: PhantomData<W>,
}

impl<W: LRealWidth> Default for Fixed<W> {
    fn default() -> Self {
        Self::zero()
    }
}

impl<W: LRealWidth> Fixed<W> {
    fn parts(sig: u128, exp: i64, p_start: i64, p_len: i64, neg: bool, inexact: bool) -> Self {
        Fixed { sig, exp, p_start, p_len, neg, inexact, _width: PhantomData }
    }

    /// The value zero.
    pub fn zero() -> Self {
        Self::parts(0, 0, 0, 0, false, false)
    }

    /// The value one.
    pub fn one() -> Self {
        Self::parts(1, 0, 0, 0, false, false)
    }

    /// The value negative one.
    pub fn negative_one() -> Self {
        Self::parts(1, 0, 0, 0, true, false)
    }

    /// True when the value is zero.
    pub fn is_zero(&self) -> bool {
        self.sig == 0
    }

    /// True when the value is negative (a zero is never negative).
    pub fn is_negative(&self) -> bool {
        self.neg && self.sig != 0
    }

    /// True when the expansion repeats.
    pub fn is_periodic(&self) -> bool {
        self.p_len > 0
    }

    /// Provenance: false when a truncation produced these digits.
    pub fn is_exact(&self) -> bool {
        !self.inexact
    }

    /// True when the value is an integer with no fractional part.
    pub fn is_integer(&self) -> bool {
        !self.is_periodic() && self.exp >= 0
    }

    /// The decimal exponent (value = significand * 10^exponent).
    pub fn exponent(&self) -> i64 {
        self.exp
    }

    /// The stored significand.
    pub fn significand(&self) -> u128 {
        self.sig
    }

    /// The index of the first repeating fractional digit.
    pub fn period_start(&self) -> i64 {
        self.p_start
    }

    /// The length of the repeating block.
    pub fn period_length(&self) -> i64 {
        self.p_len
    }

    fn as_inexact(self) -> Self {
        if self.inexact {
            self
        } else {
            Self::parts(self.sig, self.exp, self.p_start, self.p_len, self.neg, true)
        }
    }

    // ------------------------------------------------------------------
    // Parsing
    // ------------------------------------------------------------------

    /// Parses 3.14, -0.5, 0.(3), 0.1(6), 2.
    pub fn parse(text: &str) -> Result<Self, LRealError> {
        Self::try_parse(text).ok_or(LRealError::Parse)
    }

    /// Parses a decimal, returning None when the text is not one this width can hold.
    pub fn try_parse(text: &str) -> Option<Self> {
        let text = text.trim();
        if text.is_empty() {
            return None;
        }
        let (neg, body) = match text.as_bytes()[0] {
            b'-' => (true, &text[1..]),
            b'+' => (false, &text[1..]),
            _ => (false, text),
        };
        if body.is_empty() {
            return None;
        }
        let dot = body.find('.');
        let open = body.find('(');
        let (int_part, non_repeating, periodic, has_period) = match dot {
            None => {
                if open.is_some() {
                    return None;
                }
                (body, "", "", false)
            }
            Some(d) => {
                if body[..d].is_empty() {
                    return None;
                }
                match open {
                    Some(o) => {
                        let c = body.rfind(')')?;
                        if o < d + 1 || c != body.len() - 1 || c <= o + 1 {
                            return None;
                        }
                        (&body[..d], &body[d + 1..o], &body[o + 1..c], true)
                    }
                    None => (&body[..d], &body[d + 1..], "", false),
                }
            }
        };
        if !int_part.bytes().all(|b| b.is_ascii_digit())
            || !non_repeating.bytes().all(|b| b.is_ascii_digit())
            || !periodic.bytes().all(|b| b.is_ascii_digit())
        {
            return None;
        }
        let all_digits = format!("{int_part}{non_repeating}{periodic}");
        if all_digits.is_empty() {
            return None;
        }
        let (exp, p_start, p_len) = if has_period {
            let ps = non_repeating.len();
            let pl = periodic.len();
            (-((ps + pl) as i64), ps as i64, pl as i64)
        } else {
            (-(non_repeating.len() as i64), 0, 0)
        };
        let mag: u128 = all_digits.parse().ok()?;
        if mag > W::MAX {
            return None;
        }
        let neg = neg && mag != 0;
        let value = Self::parts(mag, exp, p_start, p_len, neg, false);
        Some(if p_len == 0 { Self::normalize(value) } else { value })
    }

    // ------------------------------------------------------------------
    // Normalization
    // ------------------------------------------------------------------

    fn normalize(mut value: Self) -> Self {
        if value.p_len > 0 || value.exp >= 0 || value.sig == 0 {
            return value;
        }
        while value.exp < 0 && value.sig % 10 == 0 {
            value.sig /= 10;
            value.exp += 1;
        }
        value
    }

    fn from_normalized_nat(mag: Nat, exp: i64, neg: bool, inexact: bool) -> Result<Self, LRealError> {
        if mag.is_zero() {
            return Ok(if inexact { Self::zero().as_inexact() } else { Self::zero() });
        }
        let mut s = mag;
        let mut e = exp;
        while e < 0 {
            let (q, rem) = s.div_rem(&Nat::from_u64(10)).ok_or(LRealError::Promote)?;
            if !rem.is_zero() {
                break;
            }
            s = q;
            e += 1;
        }
        let text = s.to_decimal();
        let value: u128 = text.parse().map_err(|_| LRealError::Promote)?;
        if value > W::MAX {
            return Err(LRealError::Promote);
        }
        Ok(Self::parts(value, e, 0, 0, neg, inexact))
    }

    // ------------------------------------------------------------------
    // Arithmetic
    // ------------------------------------------------------------------

    /// Addition: the exact sum when it fits, Promote otherwise.
    pub fn add(&self, other: &Self) -> Result<Self, LRealError> {
        let a = *self;
        let b = *other;
        if a.sig == 0 {
            return Ok(if a.inexact { b.as_inexact() } else { b });
        }
        if b.sig == 0 {
            return Ok(if b.inexact { a.as_inexact() } else { a });
        }
        if a.p_len > 0 || b.p_len > 0 {
            let expanded_a = a.expand_to_non_periodic(W::WORKING_FRACTIONAL_DIGITS)?;
            let expanded_b = b.expand_to_non_periodic(W::WORKING_FRACTIONAL_DIGITS)?;
            let raw = expanded_a.add(&expanded_b)?;
            return Self::detect_and_normalize_period(raw);
        }
        let result_exp = a.exp.min(b.exp);
        let mag_a = nat_shift(a.sig, a.exp - result_exp);
        let mag_b = nat_shift(b.sig, b.exp - result_exp);
        let (sum, neg) = if a.neg == b.neg {
            (mag_a.add(&mag_b), a.neg)
        } else if nat_cmp(&mag_a, &mag_b) != Ordering::Less {
            (mag_a.sub(&mag_b).ok_or(LRealError::Promote)?, a.neg)
        } else {
            (mag_b.sub(&mag_a).ok_or(LRealError::Promote)?, b.neg)
        };
        Self::from_normalized_nat(sum, result_exp, neg, a.inexact || b.inexact)
    }

    /// Subtraction: add(a, -b).
    pub fn sub(&self, other: &Self) -> Result<Self, LRealError> {
        self.add(&other.neg())
    }

    /// Multiplication: the exact product when it fits, Promote otherwise.
    pub fn mul(&self, other: &Self) -> Result<Self, LRealError> {
        let a = *self;
        let b = *other;
        if a.sig == 0 || b.sig == 0 {
            return Ok(if a.inexact || b.inexact {
                Self::zero().as_inexact()
            } else {
                Self::zero()
            });
        }
        if a.p_len > 0 || b.p_len > 0 {
            let expanded_a = a.expand_to_non_periodic(W::WORKING_FRACTIONAL_DIGITS)?;
            let expanded_b = b.expand_to_non_periodic(W::WORKING_FRACTIONAL_DIGITS)?;
            let raw = expanded_a.mul(&expanded_b)?;
            return Self::detect_and_normalize_period(raw);
        }
        let exp = a.exp + b.exp;
        let product = nat_of(a.sig).mul(&nat_of(b.sig));
        Self::from_normalized_nat(product, exp, a.neg != b.neg, a.inexact || b.inexact)
    }

    /// Division: the quotient when its period fits the width, Promote otherwise.
    pub fn div(&self, other: &Self) -> Result<Self, LRealError> {
        let a = *self;
        let b = *other;
        if b.sig == 0 {
            return Err(LRealError::DivideByZero);
        }
        if a.sig == 0 {
            return Ok(if a.inexact || b.inexact {
                Self::zero().as_inexact()
            } else {
                Self::zero()
            });
        }
        if a.p_len > 0 || b.p_len > 0 {
            let expanded_a = a.expand_to_non_periodic(W::WORKING_FRACTIONAL_DIGITS)?;
            let expanded_b = b.expand_to_non_periodic(W::WORKING_FRACTIONAL_DIGITS)?;
            let mut raw = expanded_a.div(&expanded_b)?;
            if raw.p_len >= W::WORKING_FRACTIONAL_DIGITS as i64 {
                // A period as long as the expansion width is the boundary repeating, not the
                // quotient's own period: stripping it leaves a prefix, so the result is not exact.
                raw.p_start = 0;
                raw.p_len = 0;
                raw.inexact = true;
            }
            return Self::detect_and_normalize_period(raw);
        }
        let neg = a.neg != b.neg;
        let mut sig = a.sig / b.sig;
        let mut rem = a.sig % b.sig;
        let exp_adj = a.exp - b.exp;
        let mut frac_count: i64 = 0;
        let mut history: Vec<(u128, i64)> = Vec::new();
        let mut p_start: i64 = 0;
        let mut p_len: i64 = 0;
        let mut found = false;
        while rem != 0 && frac_count < W::MAX_LOOP as i64 {
            if let Some(position) = history.iter().position(|(value, _)| *value == rem) {
                p_start = history[position].1;
                p_len = frac_count - p_start;
                found = true;
                break;
            }
            history.push((rem, frac_count));
            rem = rem.checked_mul(10).ok_or(LRealError::Promote)?;
            let digit = rem / b.sig;
            rem %= b.sig;
            if sig > (W::MAX - digit) / 10 {
                return Err(LRealError::Promote);
            }
            sig = sig * 10 + digit;
            frac_count += 1;
        }
        let result_exp = -frac_count + exp_adj;
        if found {
            if p_len > W::MAX_SIGNIFICANT_DIGITS as i64 {
                return Err(LRealError::Promote);
            }
            return Ok(Self::parts(sig, result_exp, p_start, p_len, neg, a.inexact || b.inexact));
        }
        if rem == 0 {
            return Self::from_normalized_nat(nat_of(sig), result_exp, neg, a.inexact || b.inexact);
        }
        Err(LRealError::Promote)
    }

    /// Negation (a zero stays unsigned).
    pub fn neg(&self) -> Self {
        let neg = if self.is_zero() { false } else { !self.neg };
        Self::parts(self.sig, self.exp, self.p_start, self.p_len, neg, self.inexact)
    }

    /// Absolute value.
    pub fn abs(&self) -> Self {
        if self.is_negative() {
            self.neg()
        } else {
            *self
        }
    }

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    fn get_decimal_digit(&self, position: i64) -> u8 {
        if self.exp >= 0 {
            return 0;
        }
        let digits = self.sig.to_string();
        let frac_len = -self.exp;
        if self.p_len == 0 || position < self.p_start {
            let index = digits.len() as i64 - frac_len + position;
            if index < 0 || index >= digits.len() as i64 {
                return 0;
            }
            return digits.as_bytes()[index as usize] - b'0';
        }
        let periodic_offset = (position - self.p_start) % self.p_len;
        let index = digits.len() as i64 - frac_len + self.p_start + periodic_offset;
        if index < 0 || index >= digits.len() as i64 {
            return 0;
        }
        digits.as_bytes()[index as usize] - b'0'
    }

    fn expand_to_non_periodic(&self, frac_digits: usize) -> Result<Self, LRealError> {
        let digits = self.sig.to_string();
        let frac_len = if self.exp < 0 { (-self.exp) as usize } else { 0 };
        let int_part_len = digits.len().saturating_sub(frac_len);
        let int_part = if int_part_len > 0 { &digits[..int_part_len] } else { "0" };
        let mut text = String::with_capacity(int_part.len() + frac_digits);
        text.push_str(int_part);
        for position in 0..frac_digits {
            text.push((b'0' + self.get_decimal_digit(position as i64)) as char);
        }
        let mag: u128 = text.parse().map_err(|_| LRealError::Promote)?;
        if mag > W::MAX {
            return Err(LRealError::Promote);
        }
        let neg = self.neg && mag != 0;
        // Writing a periodic value out to a fixed number of places truncates it.
        Ok(Self::parts(mag, -(frac_digits as i64), 0, 0, neg, true))
    }

    fn detect_and_normalize_period(value: Self) -> Result<Self, LRealError> {
        if value.exp >= 0 || value.sig == 0 {
            return Ok(value);
        }
        let digits = value.sig.to_string();
        let frac_len = (-value.exp) as usize;
        let int_part_len = digits.len().saturating_sub(frac_len);
        let int_part = if int_part_len > 0 { &digits[..int_part_len] } else { "0" };
        let mut frac_part = if digits.len() > int_part_len {
            digits[int_part_len..].to_string()
        } else {
            "0".repeat(frac_len)
        };
        if frac_part.len() < frac_len {
            frac_part = format!("{:0>width$}", frac_part, width = frac_len);
        }
        let (mut p_start, mut p_len) = find_smallest_period(&frac_part, 0);
        if p_len == 0 && frac_part.len() > 2 {
            let (start, len) = find_smallest_period(&frac_part, 1);
            p_start = start;
            p_len = len;
        }
        if p_len == 0 {
            return Ok(value);
        }
        let period = &frac_part[p_start..p_start + p_len];
        let all_nines = period.bytes().all(|b| b == b'9');
        if all_nines {
            let non_repeating = &frac_part[..p_start];
            let combined = format!("{int_part}{non_repeating}");
            let mut mag: u128 = if combined.is_empty() {
                0
            } else {
                combined.parse().map_err(|_| LRealError::Promote)?
            };
            mag = mag.checked_add(1).ok_or(LRealError::Promote)?;
            if mag > W::MAX {
                return Err(LRealError::Promote);
            }
            let new_exp = -(non_repeating.len() as i64);
            let neg = value.neg && mag != 0;
            return Ok(Self::parts(mag, new_exp, 0, 0, neg, value.inexact));
        }
        let stored = format!("{int_part}{}", &frac_part[..p_start + p_len]);
        let mag: u128 = stored.parse().map_err(|_| LRealError::Promote)?;
        if mag > W::MAX {
            return Err(LRealError::Promote);
        }
        let negative = value.neg && mag != 0;
        Ok(Self::parts(
            mag,
            -((p_start + p_len) as i64),
            p_start as i64,
            p_len as i64,
            negative,
            value.inexact,
        ))
    }

    // ------------------------------------------------------------------
    // Comparison / formatting
    // ------------------------------------------------------------------

    /// Compares two values exactly.
    pub fn compare(&self, other: &Self) -> Ordering {
        self.cmp_with_display(other, DEFAULT_DISPLAY_DECIMAL_PLACES)
    }

    /// Compares two values, reading a periodic expansion out to display places.
    pub fn cmp_with_display(&self, other: &Self, display: usize) -> Ordering {
        let this_neg = self.is_negative();
        let other_neg = other.is_negative();
        if !this_neg && other_neg {
            return Ordering::Greater;
        }
        if this_neg && !other_neg {
            return Ordering::Less;
        }
        let digits_a = self.sig.to_string();
        let digits_b = other.sig.to_string();
        let int_len_a = digits_a.len() as i64 + self.exp;
        let int_len_b = digits_b.len() as i64 + other.exp;
        let sign_mul = if this_neg { -1 } else { 1 };
        if int_len_a != int_len_b {
            let order = if int_len_a > int_len_b {
                Ordering::Greater
            } else {
                Ordering::Less
            };
            return if sign_mul < 0 { order.reverse() } else { order };
        }
        let stored_max = digits_a.len().max(digits_b.len()) as i64;
        let max_positions = if self.is_periodic() || other.is_periodic() {
            stored_max + display as i64
        } else {
            stored_max
        };
        for position in 0..max_positions {
            let digit_a = digit_at_position(self, position, int_len_a, &digits_a);
            let digit_b = digit_at_position(other, position, int_len_b, &digits_b);
            if digit_a != digit_b {
                let order = if digit_a > digit_b {
                    Ordering::Greater
                } else {
                    Ordering::Less
                };
                return if sign_mul < 0 { order.reverse() } else { order };
            }
        }
        Ordering::Equal
    }

    /// Renders the value at the reference's display precision (100 fractional places).
    pub fn render(&self) -> String {
        self.to_string_with_display(DEFAULT_DISPLAY_DECIMAL_PLACES)
    }

    /// Renders the value, truncating a non-periodic fraction at display places.
    pub fn to_string_with_display(&self, display: usize) -> String {
        if self.sig == 0 {
            return "0".to_string();
        }
        let digits = self.sig.to_string();
        let sign = if self.neg { "-" } else { "" };
        if self.exp == 0 && !self.is_periodic() {
            return format!("{sign}{digits}");
        }
        if self.exp < 0 && !self.is_periodic() {
            let frac_len = (-self.exp) as usize;
            if frac_len >= digits.len() {
                let padded = format!("{:0>width$}", digits, width = frac_len);
                let take = frac_len.min(frac_len.saturating_add(display));
                let frac_part = &padded[..take];
                return format!("{sign}0.{frac_part}");
            }
            let split = digits.len() - frac_len;
            let int_part = &digits[..split];
            let mut frac_part = &digits[split..];
            if frac_part.len() > display {
                frac_part = &frac_part[..display];
            }
            return format!("{sign}{int_part}.{frac_part}");
        }
        if self.is_periodic() {
            let frac_len = (-self.exp) as usize;
            let padded = if digits.len() < frac_len {
                format!("{:0>width$}", digits, width = frac_len)
            } else {
                digits.clone()
            };
            let split = padded.len().saturating_sub(frac_len);
            let int_part = if split == 0 { "0" } else { &padded[..split] };
            let all_frac = &padded[split..];
            let start = self.p_start.max(0) as usize;
            let len = self.p_len.max(0) as usize;
            if start > all_frac.len() {
                return format!("{sign}{int_part}.{all_frac}(0)");
            }
            let non_repeating = &all_frac[..start];
            let period = if all_frac.len() >= start + len {
                &all_frac[start..start + len]
            } else {
                all_frac
            };
            let dot = if frac_len > 0 { "." } else { "" };
            return format!("{sign}{int_part}{dot}{non_repeating}({period})");
        }
        format!("{sign}{digits}")
    }

    /// The exact [Real] this value denotes (promotion carries the inexact provenance).
    pub fn to_real(&self) -> Real {
        let mut real =
            Real::from_str(&self.to_string_with_display(usize::MAX)).unwrap_or_else(Real::zero);
        if self.inexact {
            real = real.mark_inexact();
        }
        real
    }
}

impl<W: LRealWidth> PartialEq for Fixed<W> {
    fn eq(&self, other: &Self) -> bool {
        self.is_periodic() == other.is_periodic() && self.compare(other) == Ordering::Equal
    }
}

impl<W: LRealWidth> Eq for Fixed<W> {}

impl<W: LRealWidth> PartialOrd for Fixed<W> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.compare(other))
    }
}

impl<W: LRealWidth> Ord for Fixed<W> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.compare(other)
    }
}

impl<W: LRealWidth> fmt::Display for Fixed<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

/// Compares two decimal magnitudes.
fn nat_cmp(a: &Nat, b: &Nat) -> Ordering {
    let left = a.to_decimal();
    let right = b.to_decimal();
    match left.len().cmp(&right.len()) {
        Ordering::Equal => left.cmp(&right),
        other => other,
    }
}

fn nat_of(value: u128) -> Nat {
    Nat::from_decimal(&value.to_string()).expect("a u128 is a decimal natural")
}

fn nat_shift(sig: u128, places: i64) -> Nat {
    nat_of(sig).mul_pow10(places.max(0) as usize)
}

fn digit_at_position<W: LRealWidth>(
    value: &Fixed<W>,
    position: i64,
    int_len: i64,
    digits: &str,
) -> u8 {
    if value.is_periodic() && position >= int_len {
        return b'0' + value.get_decimal_digit(position - int_len);
    }
    if position >= 0 && position < digits.len() as i64 {
        return digits.as_bytes()[position as usize];
    }
    b'0'
}

/// The smallest repeating block of a fractional digit string, with an optional slack.
fn find_smallest_period(frac_part: &str, slack: usize) -> (usize, usize) {
    let n = frac_part.len();
    let effective_len = n.saturating_sub(slack);
    if effective_len < 2 {
        return (0, 0);
    }
    let bytes = frac_part.as_bytes();
    for period in 1..=effective_len / 2 {
        for start in 0..=(effective_len - 2 * period) {
            let mut ok = true;
            for index in (start + period)..effective_len {
                if bytes[index] != bytes[start + (index - start) % period] {
                    ok = false;
                    break;
                }
            }
            if ok {
                return (start, period);
            }
        }
    }
    (0, 0)
}
