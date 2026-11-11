//! Arbitrary-precision complex numbers and their fixed-width exact companions.
//!
//! This is the Rust port of the reference 'Lovelace.Complex' library: two value families over
//! [Real](lovelace_real::Real), with no floating point anywhere.
//!
//! * [Complex] - an arbitrary-precision pair of Real components. Component-wise arithmetic,
//!   a decimal text form (2, 4i, 1.5 + 0.5i, 1.5 - 0.5i), equality, magnitude and the
//!   fixed-width fast path the reference uses when the ambient computation precision is at most
//!   37 places.
//! * [LComplex64] / [LComplex128] - fixed-width exact-decimal forms (19 and 38 significant
//!   digits). A result that needs more digits promotes (LRealPromoteException) instead of
//!   rounding, so the exact arbitrary-precision form can take over.
//!
//! ## Stated boundary
//!
//! The fixed-width forms here do not model the reference's periodic decimal expansions. A
//! periodic component does not convert (TryFromComplex answers None), and a fixed-width
//! operation whose exact quotient would need a period or more significant digits than the width
//! promotes rather than rounding. The arbitrary-precision [Complex] type then computes the
//! exact result, which is the reference's own fallback and never loses a digit (CPLX-004).

use lovelace_real::Real;

/// The reference's default computation budget (Real.MaxComputationDecimalPlaces).
pub const AMBIENT_COMPUTATION_DIGITS: usize = 1000;

/// The precision at or below which the reference takes its fixed-width fast path.
pub const FAST_PATH_MAX_DIGITS: usize = 37;

/// The promotion signal of the fixed-width forms: the exact result needs more significant digits
/// than the fixed width holds, so the caller must use the arbitrary-precision [Complex].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LRealPromoteException {
    message: String,
}

impl LRealPromoteException {
    fn new(width: usize) -> LRealPromoteException {
        LRealPromoteException {
            message: format!(
                "result requires more than {width} significant digits; promote to Complex."
            ),
        }
    }

    /// The refusal sentence.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for LRealPromoteException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for LRealPromoteException {}

// ---------------------------------------------------------------------------
// Arbitrary-precision Complex
// ---------------------------------------------------------------------------

/// An arbitrary-precision complex number: re + im*i, both parts Real.
#[derive(Clone, Debug)]
pub struct Complex {
    re: Real,
    im: Real,
}

impl Complex {
    /// Builds re + im*i.
    pub fn new(re: Real, im: Real) -> Complex {
        Complex { re, im }
    }

    /// Builds the purely real re + 0i.
    pub fn from_real(re: Real) -> Complex {
        Complex {
            im: Real::zero(),
            re,
        }
    }

    /// Builds a complex number from an integer (Integer/Natural promotion).
    pub fn from_i64(value: i64) -> Complex {
        Complex::from_real(Real::from_i64(value))
    }

    /// 0 + 0i.
    pub fn zero() -> Complex {
        Complex::new(Real::zero(), Real::zero())
    }

    /// 1 + 0i.
    pub fn one() -> Complex {
        Complex::new(Real::from_i64(1), Real::zero())
    }

    /// 0 + 1i, the imaginary unit.
    pub fn i() -> Complex {
        Complex::new(Real::zero(), Real::from_i64(1))
    }

    /// pi + 0i truncated to digits fractional places.
    pub fn pi(digits: usize) -> Complex {
        Complex::from_real(Real::pi(digits))
    }

    /// e + 0i truncated to digits fractional places (the reference truncation of e).
    pub fn e(digits: usize) -> Complex {
        Complex::from_real(e_digits(digits))
    }

    /// The real component.
    pub fn re(&self) -> &Real {
        &self.re
    }

    /// The imaginary component.
    pub fn im(&self) -> &Real {
        &self.im
    }

    /// Both components, real first.
    pub fn parts(&self) -> (&Real, &Real) {
        (&self.re, &self.im)
    }

    /// Exact only when BOTH components are exact (the reference IsExact rule).
    pub fn is_exact(&self) -> bool {
        self.re.is_exact() && self.im.is_exact()
    }

    /// Component-wise addition under the reference binary dispatch (exact either way).
    pub fn add(&self, other: &Complex) -> Complex {
        self.add_at(other, AMBIENT_COMPUTATION_DIGITS)
    }

    /// Component-wise subtraction under the reference binary dispatch.
    pub fn sub(&self, other: &Complex) -> Complex {
        self.sub_at(other, AMBIENT_COMPUTATION_DIGITS)
    }

    /// FOIL multiplication under the reference binary dispatch.
    pub fn mul(&self, other: &Complex) -> Complex {
        self.mul_at(other, AMBIENT_COMPUTATION_DIGITS)
    }

    /// Complex division a * (1/b); None when b is zero.
    pub fn div(&self, other: &Complex) -> Option<Complex> {
        self.div_at(other, AMBIENT_COMPUTATION_DIGITS)
    }

    /// Negation.
    pub fn neg(&self) -> Complex {
        Complex::new(self.re.neg(), self.im.neg())
    }

    /// Scales both components by a real scalar.
    pub fn scale(&self, factor: &Real) -> Complex {
        Complex::new(self.re.mul(factor), self.im.mul(factor))
    }

    /// Complex conjugate re - im*i.
    pub fn conjugate(&self) -> Complex {
        Complex::new(self.re.clone(), self.im.neg())
    }

    /// |z|^2 = re^2 + im^2, exact.
    pub fn magnitude_squared(&self) -> Real {
        self.re.mul(&self.re).add(&self.im.mul(&self.im))
    }

    /// |z| = sqrt(re^2 + im^2) truncated to digits fractional places.
    pub fn magnitude(&self, digits: usize) -> Real {
        self.magnitude_squared().sqrt(digits)
    }

    /// 1/z = conj(z) / |z|^2; None for zero.
    pub fn reciprocal(&self, digits: usize) -> Option<Complex> {
        let m2 = self.magnitude_squared();
        if m2.is_zero() {
            return None;
        }
        Some(Complex::new(
            self.re.div(&m2, digits)?,
            self.im.neg().div(&m2, digits)?,
        ))
    }

    /// Integer power by binary exponentiation; a negative exponent takes the reciprocal.
    pub fn pow_int(&self, exponent: i64, digits: usize) -> Option<Complex> {
        if exponent == 0 {
            return Some(Complex::one());
        }
        if exponent < 0 {
            let positive = self.pow_int(exponent.checked_neg()?, digits)?;
            return positive.reciprocal(digits);
        }
        let mut result = Complex::one();
        let mut factor = self.clone();
        let mut n = exponent as u64;
        while n > 0 {
            if n & 1 == 1 {
                result = result.mul(&factor);
            }
            factor = factor.mul(&factor);
            n >>= 1;
        }
        Some(result)
    }

    /// Addition under the reference Complex.Binary dispatch: at a budget of at most 37 places
    /// the fixed-width forms are tried first (19 digits, then 38) and the exact
    /// arbitrary-precision addition answers when neither fits. Both paths are exact.
    pub fn add_at(&self, other: &Complex, max_digits: usize) -> Complex {
        if let Some(fixed) = fast_fixed(self, other, max_digits, FixedOp::Add) {
            return fixed;
        }
        Complex::new(self.re.add(&other.re), self.im.add(&other.im))
    }

    /// Subtraction under the reference Complex.Binary dispatch.
    pub fn sub_at(&self, other: &Complex, max_digits: usize) -> Complex {
        if let Some(fixed) = fast_fixed(self, other, max_digits, FixedOp::Sub) {
            return fixed;
        }
        Complex::new(self.re.sub(&other.re), self.im.sub(&other.im))
    }

    /// Multiplication under the reference Complex.Binary dispatch.
    pub fn mul_at(&self, other: &Complex, max_digits: usize) -> Complex {
        if let Some(fixed) = fast_fixed(self, other, max_digits, FixedOp::Mul) {
            return fixed;
        }
        Complex::new(
            self.re.mul(&other.re).sub(&self.im.mul(&other.im)),
            self.re.mul(&other.im).add(&self.im.mul(&other.re)),
        )
    }

    /// Division under the reference Complex.Binary dispatch.
    pub fn div_at(&self, other: &Complex, max_digits: usize) -> Option<Complex> {
        if let Some(fixed) = fast_fixed(self, other, max_digits, FixedOp::Div) {
            return Some(fixed);
        }
        Some(self.mul(&other.reciprocal(max_digits)?))
    }

    /// Parses the reference forms: 2, 4i, -3i, i, -i, 1.5+0.5i, 1.5 - 0.5i, -2+3i.
    pub fn parse(text: &str) -> Result<Complex, String> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err("Input string was empty.".to_string());
        }
        if !trimmed.ends_with('i') {
            return Ok(Complex::from_real(parse_real(trimmed)?));
        }
        let body = trimmed[..trimmed.len() - 1].trim_end();
        if body.is_empty() || body == "+" || body == "-" {
            let im = if body == "-" { "-1" } else { "1" };
            return Ok(Complex::new(Real::zero(), parse_real(im)?));
        }
        // The last +/- that is not a leading sign splits the two components.
        let mut split = None;
        for (index, ch) in body.char_indices().skip(1) {
            if ch == '+' || ch == '-' {
                split = Some(index);
            }
        }
        let Some(split) = split else {
            return Ok(Complex::new(Real::zero(), parse_real(body)?));
        };
        let re_part = body[..split].trim();
        let im_part = body[split..].trim();
        let mut chars = im_part.chars();
        let sign = chars.next().unwrap_or('+');
        let mut im_digits = chars.as_str().trim().to_string();
        if im_digits.is_empty() {
            im_digits = "1".to_string();
        }
        let im_text = if sign == '-' {
            format!("-{im_digits}")
        } else {
            im_digits
        };
        Ok(Complex::new(parse_real(re_part)?, parse_real(&im_text)?))
    }
}

impl PartialEq for Complex {
    fn eq(&self, other: &Complex) -> bool {
        self.re == other.re && self.im == other.im
    }
}

impl Eq for Complex {}

impl std::fmt::Display for Complex {
    /// The reference Complex.ToString form: a zero part is omitted entirely.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.im.is_zero() {
            return f.write_str(&self.re.to_string());
        }
        if self.re.is_zero() {
            return f.write_str(&format!("{}i", self.im));
        }
        let sign = if self.im.cmp(&Real::zero()) == std::cmp::Ordering::Less {
            "-"
        } else {
            "+"
        };
        f.write_str(&format!("{} {} {}i", self.re, sign, self.im.abs()))
    }
}

fn parse_real(text: &str) -> Result<Real, String> {
    Real::from_str(text).ok_or_else(|| {
        format!("The string '{text}' is not a valid decimal representation of a Real number.")
    })
}

fn e_digits(digits: usize) -> Real {
    // e = sum(1/k!) to the requested places, in exact Real arithmetic.
    let mut series = Real::from_i64(1);
    let mut term = Real::from_i64(1);
    let mut k: i64 = 1;
    let limit = digits as i64 + 40;
    while k <= limit {
        term = term
            .div(&Real::from_i64(k), digits + 20)
            .unwrap_or_else(Real::zero);
        if term.is_zero() {
            break;
        }
        series = series.add(&term);
        k += 1;
    }
    series
}

// ---------------------------------------------------------------------------
// Fixed-width exact decimal forms
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct Fixed {
    sig: u128,
    exp: i32,
    neg: bool,
    inexact: bool,
}

impl Fixed {
    fn zero() -> Fixed {
        Fixed {
            sig: 0,
            exp: 0,
            neg: false,
            inexact: false,
        }
    }

    fn is_zero(&self) -> bool {
        self.sig == 0
    }

    fn digits_len(&self) -> usize {
        if self.sig == 0 {
            0
        } else {
            self.sig.to_string().len()
        }
    }

    fn fits(&self, max: usize) -> bool {
        self.digits_len() <= max
    }

    fn normalized(mut self) -> Fixed {
        if self.sig == 0 {
            self.exp = 0;
            self.neg = false;
            return self;
        }
        while self.exp < 0 && self.sig % 10 == 0 {
            self.sig /= 10;
            self.exp += 1;
        }
        self
    }

    fn from_real(value: &Real, max: usize) -> Option<Fixed> {
        let text = value.to_string();
        if text.contains('(') {
            // A periodic expansion is not modelled by the fixed width; the
            // arbitrary-precision path answers it exactly.
            return None;
        }
        let (neg, body) = split_sign(&text);
        let (int_part, frac) = match body.split_once('.') {
            Some((i, f)) => (i, f),
            None => (body, ""),
        };
        if !int_part.chars().all(|c| c.is_ascii_digit())
            || !frac.chars().all(|c| c.is_ascii_digit())
        {
            return None;
        }
        let mut digits = String::new();
        digits.push_str(int_part.trim_start_matches('0'));
        digits.push_str(frac);
        let digits = digits.trim_start_matches('0').to_string();
        if digits.is_empty() {
            return Some(Fixed {
                sig: 0,
                exp: 0,
                neg: false,
                inexact: !value.is_exact(),
            });
        }
        if digits.len() > max {
            return None;
        }
        let sig = digits.parse::<u128>().ok()?;
        let exp = i32::try_from(-(frac.len() as i64)).ok()?;
        Some(
            Fixed {
                sig,
                exp,
                neg,
                inexact: !value.is_exact(),
            }
            .normalized(),
        )
    }

    fn to_real(self) -> Real {
        Real::from_str(&self.value_text()).unwrap_or_else(Real::zero)
    }

    fn value_text(&self) -> String {
        if self.sig == 0 {
            return "0".to_string();
        }
        let digits = self.sig.to_string();
        let mut body = if self.exp >= 0 {
            let mut text = digits;
            for _ in 0..self.exp {
                text.push('0');
            }
            text
        } else {
            let frac = (-self.exp) as usize;
            if digits.len() <= frac {
                let mut text = String::from("0.");
                for _ in 0..(frac - digits.len()) {
                    text.push('0');
                }
                text.push_str(&digits);
                text
            } else {
                let split = digits.len() - frac;
                format!("{}.{}", &digits[..split], &digits[split..])
            }
        };
        if self.neg {
            body.insert(0, '-');
        }
        body
    }

    fn add(&self, other: &Fixed, max: usize) -> Result<Fixed, LRealPromoteException> {
        let (a, b, exp) = if self.exp == other.exp {
            (self.sig, other.sig, self.exp)
        } else if self.exp < other.exp {
            let diff = (other.exp - self.exp) as u32;
            (self.sig, scale(other.sig, diff, max)?, self.exp)
        } else {
            let diff = (self.exp - other.exp) as u32;
            (scale(self.sig, diff, max)?, other.sig, other.exp)
        };
        let (sig, neg) = if self.neg == other.neg {
            (
                a.checked_add(b)
                    .ok_or_else(|| LRealPromoteException::new(max))?,
                self.neg,
            )
        } else if a >= b {
            (a - b, self.neg)
        } else {
            (b - a, other.neg)
        };
        let result = Fixed {
            sig,
            exp,
            neg,
            inexact: self.inexact || other.inexact,
        }
        .normalized();
        if result.fits(max) {
            Ok(result)
        } else {
            Err(LRealPromoteException::new(max))
        }
    }

    fn negate(&self) -> Fixed {
        Fixed {
            neg: !self.neg && self.sig != 0,
            ..*self
        }
    }

    fn mul(&self, other: &Fixed, max: usize) -> Result<Fixed, LRealPromoteException> {
        if self.sig == 0 || other.sig == 0 {
            return Ok(Fixed {
                sig: 0,
                exp: 0,
                neg: false,
                inexact: self.inexact || other.inexact,
            });
        }
        let sig = self
            .sig
            .checked_mul(other.sig)
            .ok_or_else(|| LRealPromoteException::new(max))?;
        let exp = self
            .exp
            .checked_add(other.exp)
            .ok_or_else(|| LRealPromoteException::new(max))?;
        let result = Fixed {
            sig,
            exp,
            neg: self.neg != other.neg,
            inexact: self.inexact || other.inexact,
        }
        .normalized();
        if result.fits(max) {
            Ok(result)
        } else {
            Err(LRealPromoteException::new(max))
        }
    }

    /// Exact division: the quotient must terminate within max fractional digits; otherwise the
    /// fixed width cannot hold it and the caller promotes. Never rounds.
    fn div(&self, other: &Fixed, max: usize) -> Result<Fixed, LRealPromoteException> {
        if other.sig == 0 {
            return Err(LRealPromoteException::new(max));
        }
        if self.sig == 0 {
            return Ok(Fixed {
                sig: 0,
                exp: 0,
                neg: false,
                inexact: self.inexact || other.inexact,
            });
        }
        let mut numerator = self.sig;
        let mut extra = 0u32;
        loop {
            if numerator % other.sig == 0 {
                break;
            }
            if extra as usize >= max {
                return Err(LRealPromoteException::new(max));
            }
            numerator = numerator
                .checked_mul(10)
                .ok_or_else(|| LRealPromoteException::new(max))?;
            extra += 1;
        }
        let sig = numerator / other.sig;
        let exp = self
            .exp
            .checked_sub(other.exp)
            .and_then(|e| e.checked_sub(i32::try_from(extra).ok()?))
            .ok_or_else(|| LRealPromoteException::new(max))?;
        let result = Fixed {
            sig,
            exp,
            neg: self.neg != other.neg,
            inexact: self.inexact || other.inexact,
        }
        .normalized();
        if result.fits(max) {
            Ok(result)
        } else {
            Err(LRealPromoteException::new(max))
        }
    }
}

fn scale(value: u128, places: u32, max: usize) -> Result<u128, LRealPromoteException> {
    let mut result = value;
    for _ in 0..places {
        result = result
            .checked_mul(10)
            .ok_or_else(|| LRealPromoteException::new(max))?;
    }
    Ok(result)
}

fn split_sign(text: &str) -> (bool, &str) {
    if let Some(rest) = text.strip_prefix('-') {
        (true, rest)
    } else if let Some(rest) = text.strip_prefix('+') {
        (false, rest)
    } else {
        (false, text)
    }
}

#[derive(Clone, Copy, Debug)]
struct FixedComplex {
    re: Fixed,
    im: Fixed,
    width: usize,
}

#[derive(Clone, Copy)]
enum FixedOp {
    Add,
    Sub,
    Mul,
    Div,
}

impl FixedComplex {
    fn from_complex(value: &Complex, width: usize) -> Option<FixedComplex> {
        Some(FixedComplex {
            re: Fixed::from_real(&value.re, width)?,
            im: Fixed::from_real(&value.im, width)?,
            width,
        })
    }

    fn to_complex(self) -> Complex {
        Complex::new(self.re.to_real(), self.im.to_real())
    }

    fn apply(
        &self,
        other: &FixedComplex,
        op: FixedOp,
    ) -> Result<FixedComplex, LRealPromoteException> {
        let width = self.width;
        let (re, im) = match op {
            FixedOp::Add => (
                self.re.add(&other.re, width)?,
                self.im.add(&other.im, width)?,
            ),
            FixedOp::Sub => (
                self.re.add(&other.re.negate(), width)?,
                self.im.add(&other.im.negate(), width)?,
            ),
            FixedOp::Mul => (
                self.re
                    .mul(&other.re, width)?
                    .add(&self.im.mul(&other.im, width)?.negate(), width)?,
                self.re
                    .mul(&other.im, width)?
                    .add(&self.im.mul(&other.re, width)?, width)?,
            ),
            FixedOp::Div => {
                let reciprocal = other.reciprocal()?;
                return self.apply(&reciprocal, FixedOp::Mul);
            }
        };
        Ok(FixedComplex { re, im, width })
    }

    fn conjugate(&self) -> FixedComplex {
        FixedComplex {
            re: self.re,
            im: self.im.negate(),
            width: self.width,
        }
    }

    fn magnitude_squared(&self) -> Result<Fixed, LRealPromoteException> {
        let width = self.width;
        self.re
            .mul(&self.re, width)?
            .add(&self.im.mul(&self.im, width)?, width)
    }

    fn reciprocal(&self) -> Result<FixedComplex, LRealPromoteException> {
        let width = self.width;
        let m2 = self.magnitude_squared()?;
        if m2.is_zero() {
            return Err(LRealPromoteException::new(width));
        }
        Ok(FixedComplex {
            re: self.re.div(&m2, width)?,
            im: self.im.negate().div(&m2, width)?,
            width,
        })
    }

    fn negate(&self) -> FixedComplex {
        FixedComplex {
            re: self.re.negate(),
            im: self.im.negate(),
            width: self.width,
        }
    }

    fn eq_component(left: Fixed, right: Fixed) -> bool {
        left.sig == right.sig && (left.sig == 0 || (left.exp == right.exp && left.neg == right.neg))
    }
}

fn fast_fixed(left: &Complex, right: &Complex, max_digits: usize, op: FixedOp) -> Option<Complex> {
    if max_digits > FAST_PATH_MAX_DIGITS {
        return None;
    }
    if let (Some(a), Some(b)) = (
        FixedComplex::from_complex(left, 19),
        FixedComplex::from_complex(right, 19),
    ) {
        if let Ok(result) = a.apply(&b, op) {
            return Some(result.to_complex());
        }
    }
    if let (Some(a), Some(b)) = (
        FixedComplex::from_complex(left, 38),
        FixedComplex::from_complex(right, 38),
    ) {
        if let Ok(result) = a.apply(&b, op) {
            return Some(result.to_complex());
        }
    }
    None
}

macro_rules! fixed_complex {
    ($name:ident, $width:expr, $doc:expr) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Debug)]
        pub struct $name {
            inner: FixedComplex,
        }

        impl $name {
            /// The maximum number of significant digits the components hold.
            pub const MAX_SIGNIFICANT_DIGITS: usize = $width;

            /// Parses the reference forms (2, 4i, i, -i, 1.5 + 0.5i, -2+3i).
            pub fn parse(text: &str) -> Result<$name, String> {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    return Err("Input string was empty.".to_string());
                }
                if !trimmed.ends_with('i') {
                    return Ok($name {
                        inner: FixedComplex {
                            re: parse_fixed(trimmed, $width)?,
                            im: Fixed::zero(),
                            width: $width,
                        },
                    });
                }
                let body = trimmed[..trimmed.len() - 1].trim_end();
                if body.is_empty() || body == "+" || body == "-" {
                    let im = parse_fixed(if body == "-" { "-1" } else { "1" }, $width)?;
                    return Ok($name {
                        inner: FixedComplex {
                            re: Fixed::zero(),
                            im,
                            width: $width,
                        },
                    });
                }
                let mut split = None;
                for (index, ch) in body.char_indices().skip(1) {
                    if ch == '+' || ch == '-' {
                        split = Some(index);
                    }
                }
                let Some(split) = split else {
                    return Ok($name {
                        inner: FixedComplex {
                            re: Fixed::zero(),
                            im: parse_fixed(body, $width)?,
                            width: $width,
                        },
                    });
                };
                let re_part = body[..split].trim();
                let im_part = body[split..].trim();
                let mut chars = im_part.chars();
                let sign = chars.next().unwrap_or('+');
                let mut im_digits = chars.as_str().trim().to_string();
                if im_digits.is_empty() {
                    im_digits = "1".to_string();
                }
                let im_text = if sign == '-' {
                    format!("-{im_digits}")
                } else {
                    im_digits
                };
                Ok($name {
                    inner: FixedComplex {
                        re: parse_fixed(re_part, $width)?,
                        im: parse_fixed(&im_text, $width)?,
                        width: $width,
                    },
                })
            }

            /// Converts an arbitrary-precision [Complex] when both components fit the width and
            /// are finite decimals (a periodic component does not fit).
            pub fn try_from_complex(value: &Complex) -> Option<$name> {
                FixedComplex::from_complex(value, $width).map(|inner| $name { inner })
            }

            /// Converts back to the arbitrary-precision [Complex].
            pub fn to_complex(&self) -> Complex {
                self.inner.to_complex()
            }

            /// Component-wise addition; promotes when a component outgrows the width.
            pub fn checked_add(&self, other: &$name) -> Result<$name, LRealPromoteException> {
                self.inner
                    .apply(&other.inner, FixedOp::Add)
                    .map(|inner| $name { inner })
            }

            /// Component-wise subtraction; promotes when a component outgrows the width.
            pub fn checked_sub(&self, other: &$name) -> Result<$name, LRealPromoteException> {
                self.inner
                    .apply(&other.inner, FixedOp::Sub)
                    .map(|inner| $name { inner })
            }

            /// FOIL multiplication; promotes when a component outgrows the width.
            pub fn checked_mul(&self, other: &$name) -> Result<$name, LRealPromoteException> {
                self.inner
                    .apply(&other.inner, FixedOp::Mul)
                    .map(|inner| $name { inner })
            }

            /// Complex division; promotes when the exact quotient does not fit the width.
            pub fn checked_div(&self, other: &$name) -> Result<$name, LRealPromoteException> {
                self.inner
                    .apply(&other.inner, FixedOp::Div)
                    .map(|inner| $name { inner })
            }

            /// Negation.
            pub fn neg(&self) -> $name {
                $name {
                    inner: self.inner.negate(),
                }
            }

            /// Complex conjugate.
            pub fn conjugate(&self) -> $name {
                $name {
                    inner: self.inner.conjugate(),
                }
            }

            /// |z|^2 in the fixed width (promotes when the squares do not fit).
            pub fn magnitude_squared(&self) -> Result<Real, LRealPromoteException> {
                self.inner.magnitude_squared().map(|value| value.to_real())
            }

            /// 1/z, or a promotion refusal for zero and for a quotient that does not fit.
            pub fn reciprocal(&self) -> Result<$name, LRealPromoteException> {
                self.inner.reciprocal().map(|inner| $name { inner })
            }
        }

        impl PartialEq for $name {
            fn eq(&self, other: &$name) -> bool {
                FixedComplex::eq_component(self.inner.re, other.inner.re)
                    && FixedComplex::eq_component(self.inner.im, other.inner.im)
            }
        }

        impl Eq for $name {}

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.to_complex().to_string())
            }
        }
    };
}

fixed_complex!(
    LComplex64,
    19,
    "Fixed-width complex number over 19-significant-digit exact decimals."
);
fixed_complex!(
    LComplex128,
    38,
    "Fixed-width complex number over 38-significant-digit exact decimals."
);

fn parse_fixed(text: &str, width: usize) -> Result<Fixed, String> {
    let value = parse_real(text)?;
    Fixed::from_real(&value, width).ok_or_else(|| {
        format!(
            "The string '{text}' needs more than {width} significant digits (or is a periodic decimal) and does not fit the fixed width."
        )
    })
}
