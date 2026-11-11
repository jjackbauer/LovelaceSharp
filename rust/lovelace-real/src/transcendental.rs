//! Arbitrary-precision transcendental kernel.
//!
//! The algorithms are the reference kernel's (\`Lovelace.Complex/ComplexMath.cs\`): argument
//! reduction first (x mod 2*pi for the trigonometric pair, a decimal decomposition for ln, an
//! integer/fractional split for exp), then a Taylor or reduced-atanh series evaluated at the
//! requested fractional budget plus 20 guard places, and finally truncation back to the requested
//! budget. Arithmetic follows the reference's two division shapes: the truncating fixed-point
//! division the series use, and the ambient-precision division the composed identities use.
//!
//! Everything here is deterministic fixed-point decimal arithmetic over \`Nat\`/\`Int\`; no IEEE
//! floating point is used. Every result that dropped a digit is marked inexact.

use std::cmp::Ordering;

use lovelace_int::Int;
use lovelace_nat::Nat;

use crate::Real;

/// Extra working digits carried internally so the final truncated result keeps the requested
/// precision (the reference's \`ComplexMath.GuardDigits\`).
pub const GUARD_DIGITS: usize = 20;

fn int_from_mag(mag: &Nat) -> Int {
    Int::from_decimal(&mag.to_decimal()).unwrap_or_else(Int::zero)
}

/// Builds a value from sign/magnitude/exponent without normalising.
fn raw(neg: bool, mag: Nat, exp: i64, inexact: bool) -> Real {
    let mut int = int_from_mag(&mag);
    if neg {
        int = int.negate();
    }
    Real { int, exp, period_start: -1, period_len: 0, inexact }
}

/// Builds a normalised value, preserving the provenance flag.
fn norm(neg: bool, mag: Nat, exp: i64, inexact: bool) -> Real {
    raw(neg, mag, exp, inexact).normalized()
}

fn ten_pow(n: usize) -> Nat {
    Nat::from_u64(10).pow(n as u64)
}

fn one() -> Real {
    Real::from_i64(1)
}

fn two() -> Real {
    Real::from_i64(2)
}

fn ten_to_the_neg(n: usize) -> Real {
    raw(false, Nat::from_u64(1), -(n as i64), false)
}

/// The reference \`TruncateFrac\`: drop fractional digits beyond \`max_frac\`, toward zero,
/// carrying the truncation in the result's provenance. A value smaller than the window keeps
/// \`max_frac\` SIGNIFICANT digits instead of being flushed to zero.
pub fn truncate_frac(x: &Real, max_frac: usize) -> Real {
    if x.is_zero() || x.period_len > 0 {
        return x.clone();
    }
    let stored_frac = if x.exp < 0 { (-x.exp) as usize } else { 0 };
    if stored_frac <= max_frac {
        return x.clone();
    }
    let digits = x.int.magnitude().to_decimal();
    let to_drop = stored_frac - max_frac;
    let (keep, drop, sig_only) = if digits.len() > to_drop {
        (&digits[..digits.len() - to_drop], to_drop, false)
    } else {
        // The window sits above the value: keep max_frac significant digits rather than nothing.
        let keep_len = max_frac.min(digits.len());
        if keep_len == 0 {
            return Real::zero().mark_inexact();
        }
        (&digits[..keep_len], digits.len() - keep_len, true)
    };
    let mag = Nat::from_decimal(keep).unwrap_or_else(Nat::zero);
    let exp = x.exp + drop as i64;
    let mut value = norm(x.int.is_negative(), mag, exp, true);
    if sig_only && value.is_zero() {
        return Real::zero().mark_inexact();
    }
    value.inexact = true;
    value
}

fn with_argument_provenance(result: Real, argument: &Real) -> Real {
    if argument.is_exact() {
        result
    } else {
        result.mark_inexact()
    }
}

fn with_two_argument_provenance(result: Real, first: &Real, second: &Real) -> Real {
    if first.is_exact() && second.is_exact() {
        result
    } else {
        result.mark_inexact()
    }
}

/// The reference \`ComplexMath.DivideTruncate\`: one fixed-point division to \`frac_digits\`
/// fractional places (after folding a positive exponent difference into the numerator).
/// The result is built without normalising, exactly as the reference builds it.
pub fn div_truncate(left: &Real, right: &Real, frac_digits: usize) -> Real {
    assert!(!right.is_zero(), "Cannot divide a Real by zero.");
    if left.is_zero() {
        return Real::zero();
    }
    let neg = left.int.is_negative() != right.int.is_negative();
    let a = left.int.magnitude();
    let b = right.int.magnitude();
    let adjust = left.exp - right.exp;
    let n = frac_digits as i64 + adjust.max(0);
    let scaled = a.mul_pow10(n as usize);
    let (q, r) = scaled.div_rem(b).expect("nonzero divisor");
    let value = raw(neg, q, -n + adjust, !r.is_zero());
    value
}

/// The reference ambient \`Real.Divide\` for the operand shapes the kernel feeds it. Periodic
/// operands are refused by the construction of every transcendental (their digits are a
/// truncation), so this is the non-periodic path: exact when the quotient terminates, otherwise
/// truncated to \`max_digits\` fractional places and marked inexact.
pub fn div_at(left: &Real, right: &Real, max_digits: usize) -> Real {
    let q = left.div(right, max_digits).unwrap_or_else(Real::zero);
    q
}

/// \`e\` truncated to \`digits\` fractional places, computed exactly then divided once (the
/// reference \`Real.ETo\`).
pub fn e_constant(digits: usize) -> Real {
    let guard = digits + 10;
    let limit = ten_pow(guard + 1);
    let mut nfact = Nat::from_u64(1);
    let mut n: u64 = 1;
    while nfact.cmp(&limit) == Ordering::Less {
        n += 1;
        nfact = nfact.mul(&Nat::from_u64(n));
    }
    let mut sum_num = nfact.clone();
    let mut term = nfact.clone();
    for k in 1..=n {
        let (next, _) = term.div_rem(&Nat::from_u64(k)).expect("nonzero divisor");
        term = next;
        sum_num = sum_num.add(&term);
    }
    let scaled = sum_num.mul_pow10(guard);
    let (q, r) = scaled.div_rem(&nfact).expect("nonzero divisor");
    let mut value = norm(false, q, -(guard as i64), !r.is_zero());
    value.inexact = true;
    truncate_frac(&value, digits)
}

/// Splits \`x\` into \`n = trunc(x)\` and \`f = x - n\` with \`|f| < 1\`.
fn split_integer_fractional(x: &Real) -> (i64, Real) {
    if x.exp >= 0 {
        // Integral: the magnitude already holds every digit.
        let text = x.int.to_decimal();
        let n = text.parse::<i64>().unwrap_or(0);
        return (n, Real::zero());
    }
    let frac = (-x.exp) as usize;
    let mag = x.int.magnitude().to_decimal();
    let n = if mag.len() > frac {
        let text = &mag[..mag.len() - frac];
        let parsed = text.parse::<i64>().unwrap_or(0);
        if x.int.is_negative() { -parsed } else { parsed }
    } else {
        0
    };
    (n, x.sub(&Real::from_i64(n)))
}

fn int_pow(base: &Real, n: i64, work: usize) -> Real {
    if n < 0 {
        let positive = int_pow(base, -n, work);
        return div_truncate(&one(), &positive, work + GUARD_DIGITS);
    }
    let mut result = one();
    let mut factor = base.clone();
    let mut e = n;
    while e > 0 {
        if e & 1 == 1 {
            result = result.mul(&factor);
        }
        factor = factor.mul(&factor);
        e >>= 1;
    }
    result
}

fn exp_series(g: &Real, work: usize) -> Real {
    let mut term = one();
    let mut sum = one();
    let threshold = ten_to_the_neg(work);
    let max_k = 2 * work + 60;
    for k in 1..=max_k {
        term = div_truncate(&term.mul(g), &Real::from_i64(k as i64), work);
        sum = sum.add(&term);
        if term.abs().cmp(&threshold) == Ordering::Less {
            break;
        }
    }
    sum
}

impl Real {
    /// \`e\` truncated to \`digits\` fractional places (irrational, so always inexact).
    pub fn e(digits: usize) -> Real {
        e_constant(digits)
    }

    /// \`e^x\` truncated to \`digits\` fractional places (the reference \`ComplexMath.Exp\`).
    pub fn exp(&self, digits: usize) -> Real {
        let digits = digits.max(1);
        if self.is_zero() {
            return with_argument_provenance(one(), self);
        }
        let work = digits + GUARD_DIGITS;
        let (n, f) = split_integer_fractional(self);
        let exp_f = exp_series(&f, work);
        let exp_n = int_pow(&e_constant(work), n, work);
        truncate_frac(&exp_f.mul(&exp_n), digits)
    }

    /// Natural logarithm truncated to \`digits\` fractional places. \`None\` for x <= 0.
    pub fn ln(&self, digits: usize) -> Option<Real> {
        let digits = digits.max(1);
        if self.is_zero() || self.int.is_negative() {
            return None;
        }
        let work = digits + GUARD_DIGITS;
        let (exp10, m) = decompose(self);
        let ln10 = Real::from_i64(3)
            .mul(&atanh_ln(&two(), work))
            .add(&atanh_ln(&Real::from_str("1.25").expect("1.25"), work));
        let value = atanh_ln(&m, work).add(&Real::from_i64(exp10).mul(&ln10));
        Some(truncate_frac(&value, digits))
    }

    /// \`sin(x)\` truncated to \`digits\` fractional places.
    pub fn sin(&self, digits: usize) -> Real {
        let digits = digits.max(1);
        truncate_frac(&sin_cos_at_precision(self, digits).0, digits)
    }

    /// \`cos(x)\` truncated to \`digits\` fractional places.
    pub fn cos(&self, digits: usize) -> Real {
        let digits = digits.max(1);
        truncate_frac(&sin_cos_at_precision(self, digits).1, digits)
    }

    /// \`tan(x) = sin(x) / cos(x)\` with both halves taken at the ambient precision, matching the
    /// reference's numeric evaluator (\`NumOps.Tan\`). \`None\` at a pole.
    pub fn tan(&self, digits: usize) -> Option<Real> {
        let digits = digits.max(1);
        let sin = self.sin(digits);
        let cos = self.cos(digits);
        if cos.is_zero() {
            return None;
        }
        Some(div_at(&sin, &cos, digits))
    }

    /// \`atan(x)\` truncated to \`digits\` fractional places.
    pub fn atan(&self, digits: usize) -> Real {
        let digits = digits.max(1);
        let work = digits + GUARD_DIGITS;
        let negate = self.int.is_negative();
        let ax = self.abs();
        let big = ax.cmp(&one()) == Ordering::Greater;
        let arg = if big { div_truncate(&one(), &ax, work) } else { ax };
        let mut y = arg;
        for _ in 0..3 {
            let denom = one().add(&y.mul(&y)).sqrt(work);
            y = div_truncate(&y, &one().add(&denom), work);
        }
        let atan_arg = atan_series(&y, work).mul(&Real::from_i64(8));
        let mut result = if big {
            Real::pi(work).div(&two(), work).unwrap_or_else(Real::zero).sub(&atan_arg)
        } else {
            atan_arg
        };
        if negate {
            result = result.neg();
        }
        with_argument_provenance(truncate_frac(&result, digits), self)
    }

    /// \`asin(x)\` for |x| <= 1, truncated to \`digits\` fractional places.
    pub fn asin(&self, digits: usize) -> Option<Real> {
        let digits = digits.max(1);
        if self.cmp(&one()) == Ordering::Greater || self.cmp(&Real::from_i64(-1)) == Ordering::Less {
            return None;
        }
        let half_pi = Real::pi(digits).div(&two(), digits).unwrap_or_else(Real::zero);
        if self.cmp(&one()) == Ordering::Equal {
            return Some(half_pi);
        }
        if self.cmp(&Real::from_i64(-1)) == Ordering::Equal {
            return Some(half_pi.neg());
        }
        let work = digits + GUARD_DIGITS;
        let arg = div_truncate(self, &one().sub(&self.mul(self)).sqrt(digits), work);
        Some(with_argument_provenance(arg.atan(digits), self))
    }

    /// \`acos(x)\` for |x| <= 1, truncated to \`digits\` fractional places.
    pub fn acos(&self, digits: usize) -> Option<Real> {
        let digits = digits.max(1);
        let asin = self.asin(digits)?;
        let half_pi = Real::pi(digits).div(&two(), digits).unwrap_or_else(Real::zero);
        Some(half_pi.sub(&asin))
    }

    /// \`atan2(y, x)\` in (-pi, pi], truncated to \`digits\` fractional places.
    pub fn atan2(y: &Real, x: &Real, digits: usize) -> Real {
        let digits = digits.max(1);
        let pi = Real::pi(digits);
        let half_pi = pi.div(&two(), digits).unwrap_or_else(Real::zero);
        let x_zero = x.is_zero();
        let y_zero = y.is_zero();
        if x_zero {
            if y_zero {
                return with_two_argument_provenance(Real::zero(), y, x);
            }
            let branch = if y.int.is_negative() { half_pi.neg() } else { half_pi };
            return with_two_argument_provenance(branch, y, x);
        }
        let work = digits + GUARD_DIGITS;
        let at = y.div(x, work).unwrap_or_else(Real::zero).atan(digits);
        let result = if !x.int.is_negative() {
            at
        } else if y.int.is_negative() {
            at.sub(&pi)
        } else {
            at.add(&pi)
        };
        with_two_argument_provenance(result, y, x)
    }

    /// \`sinh(x) = (e^x - e^-x)/2\`, the reference numeric evaluator's shape.
    pub fn sinh(&self, digits: usize) -> Real {
        let digits = digits.max(1);
        let e = self.exp(digits);
        let inv = div_at(&one(), &e, digits);
        e.sub(&inv).div(&two(), digits).unwrap_or_else(Real::zero)
    }

    /// \`cosh(x) = (e^x + e^-x)/2\`, the reference numeric evaluator's shape.
    pub fn cosh(&self, digits: usize) -> Real {
        let digits = digits.max(1);
        let e = self.exp(digits);
        let inv = div_at(&one(), &e, digits);
        e.add(&inv).div(&two(), digits).unwrap_or_else(Real::zero)
    }

    /// \`tanh(x) = sinh(x)/cosh(x)\`.
    pub fn tanh(&self, digits: usize) -> Real {
        let digits = digits.max(1);
        let sinh = self.sinh(digits);
        let cosh = self.cosh(digits);
        div_at(&sinh, &cosh, digits)
    }

    /// \`asinh(x) = ln(x + sqrt(x^2 + 1))\`.
    pub fn asinh(&self, digits: usize) -> Real {
        let digits = digits.max(1);
        let root = self.mul(self).add(&one()).sqrt(digits);
        self.add(&root).ln(digits).unwrap_or_else(Real::zero)
    }

    /// \`acosh(x) = ln(x + sqrt(x^2 - 1))\` for x >= 1. \`None\` below 1.
    pub fn acosh(&self, digits: usize) -> Option<Real> {
        let digits = digits.max(1);
        if self.cmp(&one()) == Ordering::Less {
            return None;
        }
        let root = self.mul(self).sub(&one()).sqrt(digits);
        self.add(&root).ln(digits)
    }

    /// \`atanh(x) = (ln(1+x) - ln(1-x))/2\` for |x| < 1. \`None\` outside the open interval.
    pub fn atanh(&self, digits: usize) -> Option<Real> {
        let digits = digits.max(1);
        if self.cmp(&one()) != Ordering::Less || self.cmp(&Real::from_i64(-1)) != Ordering::Greater {
            return None;
        }
        let left = one().add(self).ln(digits)?;
        let right = one().sub(self).ln(digits)?;
        Some(left.sub(&right).div(&two(), digits).unwrap_or_else(Real::zero))
    }

    /// \`b^e = exp(e * ln b)\` for b > 0. \`None\` otherwise.
    pub fn pow_real(&self, exponent: &Real, digits: usize) -> Option<Real> {
        let digits = digits.max(1);
        if self.is_zero() || self.int.is_negative() {
            return None;
        }
        let product = exponent.mul(&self.ln(digits)?);
        Some(product.exp(digits))
    }

    /// The raw \`sin\`/\`cos\` pair before the final ambient truncation (see
    /// \`sin_cos_at_precision\` in the reference).
    pub fn sin_cos_raw(&self, digits: usize) -> (Real, Real) {
        sin_cos_at_precision(self, digits.max(1))
    }

    /// The reference \`RealSinh\`: (e^x - e^-x)/2 with each exp at the ambient precision.
    pub fn sinh_hyperbolic(&self, digits: usize) -> Real {
        let digits = digits.max(1);
        self.exp(digits).sub(&self.neg().exp(digits)).div(&two(), digits).unwrap_or_else(Real::zero)
    }

    /// The reference \`RealCosh\`.
    pub fn cosh_hyperbolic(&self, digits: usize) -> Real {
        let digits = digits.max(1);
        self.exp(digits).add(&self.neg().exp(digits)).div(&two(), digits).unwrap_or_else(Real::zero)
    }

}

/// The reference \`ComplexMath.Decompose\`: positive x = m * 10^L with m in [1, 10).
fn decompose(x: &Real) -> (i64, Real) {
    let mag = x.int.magnitude().to_decimal();
    let len = mag.len() as i64;
    let l = len - 1 + x.exp;
    let m = x.mul(&pow10(-l));
    (l, m)
}

fn pow10(e: i64) -> Real {
    if e == 0 {
        return one();
    }
    if e > 0 {
        Real::from_str(&format!("1{}", "0".repeat(e as usize))).unwrap_or_else(one)
    } else {
        Real::from_str(&format!("0.{}1", "0".repeat((-e - 1) as usize))).unwrap_or_else(one)
    }
}

/// The reference \`ComplexMath.AtanhLn\`: 16 square roots, then a reduced atanh series.
fn atanh_ln(m: &Real, work: usize) -> Real {
    let mut x = m.clone();
    for _ in 0..16 {
        x = x.sqrt(work);
    }
    let t = div_truncate(&x.sub(&one()), &x.add(&one()), work);
    let t2 = t.mul(&t);
    let mut pow = t.clone();
    let mut sum = t;
    let threshold = ten_to_the_neg(work);
    let max_k = ((work + 20) * 2) / 19 + 5;
    for k in 1..=max_k {
        pow = pow.mul(&t2);
        let contrib = div_truncate(&pow, &Real::from_i64((2 * k + 1) as i64), work);
        sum = sum.add(&contrib);
        if contrib.abs().cmp(&threshold) == Ordering::Less {
            break;
        }
    }
    sum.mul(&two()).mul(&Real::from_i64(65536))
}

fn argument_places(x: &Real) -> i64 {
    if x.period_len > 0 {
        return 0;
    }
    if x.exp < 0 {
        -x.exp
    } else {
        0
    }
}

fn pi_digits_for(x: &Real, digits: usize) -> usize {
    let resolved = argument_places(x) + digits as i64 + GUARD_DIGITS as i64;
    let ambient = digits as i64;
    resolved.max(ambient) as usize
}

fn trunc_to_long(q: &Real) -> i64 {
    let text = q.to_decimal_fixed(0);
    text.parse::<i64>().unwrap_or(0)
}

fn reduce_to_two_pi(value: &Real, _pi: &Real, two_pi: &Real, work: usize) -> Real {
    if value.cmp(&Real::zero()) != Ordering::Less && value.cmp(two_pi) == Ordering::Less {
        return value.clone();
    }
    let k = trunc_to_long(&div_at(value, two_pi, work));
    let mut reduced = value.sub(&two_pi.mul(&Real::from_i64(k)));
    if reduced.cmp(&Real::zero()) == Ordering::Less {
        reduced = reduced.add(two_pi);
    }
    reduced
}

/// The reference \`ComplexMath.Fold\`: reduce modulo pi/2 and fold into [0, pi/4].
fn fold(x: &Real, pi: &Real, work: usize) -> (i64, Real, bool) {
    let two_pi = pi.mul(&two());
    let half_pi = pi.div(&two(), work).unwrap_or_else(Real::zero);
    let quarter_pi = pi.div(&Real::from_i64(4), work).unwrap_or_else(Real::zero);
    let xr = reduce_to_two_pi(x, pi, &two_pi, work);
    let k = trunc_to_long(&div_at(&xr, &half_pi, work));
    let mut r = xr.sub(&half_pi.mul(&Real::from_i64(k)));
    let mut swap = false;
    if r.cmp(&quarter_pi) == Ordering::Greater {
        r = half_pi.sub(&r);
        swap = true;
    }
    (k, r, swap)
}

fn reduces_to_zero_against_ambient_pi(x: &Real, work: usize) -> bool {
    let ambient = work.saturating_sub(GUARD_DIGITS).max(1);
    let pi = Real::pi(ambient);
    let (_, r, _) = fold(x, &pi, work);
    r.is_zero()
}

/// The reference \`ComplexMath.SinCosAtPrecision\`: the untruncated pair.
fn sin_cos_at_precision(x: &Real, digits: usize) -> (Real, Real) {
    if x.is_zero() {
        if x.is_exact() {
            return (Real::zero(), one());
        }
        return (Real::zero().mark_inexact(), one().mark_inexact());
    }
    let work = digits + GUARD_DIGITS;
    let ambient = digits;
    let mut pi_digits = pi_digits_for(x, digits);
    if argument_places(x) <= ambient as i64 && reduces_to_zero_against_ambient_pi(x, work) {
        pi_digits = ambient;
    }
    let pi = Real::pi(pi_digits);
    let (k, r, swap) = fold(x, &pi, work);
    let mut s = sin_taylor(&r, work);
    let mut c = cos_taylor(&r, work);
    if swap {
        std::mem::swap(&mut s, &mut c);
    }
    let (sin, cos) = match k.rem_euclid(4) {
        1 => (c, s.neg()),
        2 => (s.neg(), c.neg()),
        3 => (c.neg(), s),
        _ => (s, c),
    };
    if x.is_exact() {
        (sin, cos)
    } else {
        (sin.mark_inexact(), cos.mark_inexact())
    }
}

fn sin_taylor(r: &Real, work: usize) -> Real {
    let x2 = r.mul(r);
    let mut term = r.clone();
    let mut sum = r.clone();
    let threshold = ten_to_the_neg(work);
    let max_k = 5 * work + 60;
    for k in 1..=max_k {
        let denom = (2 * k) * (2 * k + 1);
        term = div_truncate(&term.neg().mul(&x2), &Real::from_i64(denom as i64), work);
        sum = sum.add(&term);
        if term.abs().cmp(&threshold) == Ordering::Less {
            break;
        }
    }
    sum
}

fn cos_taylor(r: &Real, work: usize) -> Real {
    let x2 = r.mul(r);
    let mut term = one();
    let mut sum = one();
    let threshold = ten_to_the_neg(work);
    let max_k = 5 * work + 60;
    for k in 1..=max_k {
        let denom = (2 * k - 1) * (2 * k);
        term = div_truncate(&term.neg().mul(&x2), &Real::from_i64(denom as i64), work);
        sum = sum.add(&term);
        if term.abs().cmp(&threshold) == Ordering::Less {
            break;
        }
    }
    sum
}

fn atan_series(y: &Real, work: usize) -> Real {
    let y2 = y.mul(y);
    let mut pow = y.clone();
    let mut sum = y.clone();
    let threshold = ten_to_the_neg(work);
    let max_k = 3 * work + 60;
    for k in 1..=max_k {
        pow = pow.mul(&y2);
        let contrib = div_truncate(&pow, &Real::from_i64((2 * k + 1) as i64), work);
        sum = if k & 1 == 1 { sum.sub(&contrib) } else { sum.add(&contrib) };
        if contrib.abs().cmp(&threshold) == Ordering::Less {
            break;
        }
    }
    sum
}
