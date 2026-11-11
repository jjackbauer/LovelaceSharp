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
    Real {
        int,
        exp,
        period_start: -1,
        period_len: 0,
        inexact,
    }
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
    raw(neg, q, -n + adjust, !r.is_zero())
}

/// The reference ambient \`Real.Divide\` for the operand shapes the kernel feeds it. Periodic
/// operands are refused by the construction of every transcendental (their digits are a
/// truncation), so this is the non-periodic path: exact when the quotient terminates, otherwise
/// truncated to \`max_digits\` fractional places and marked inexact.
pub fn div_at(left: &Real, right: &Real, max_digits: usize) -> Real {
    left.div(right, max_digits).unwrap_or_else(Real::zero)
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
        real_sin(self, digits)
    }

    /// \`cos(x)\` truncated to \`digits\` fractional places.
    pub fn cos(&self, digits: usize) -> Real {
        let digits = digits.max(1);
        real_cos(self, digits)
    }

    /// \`tan(x) = sin(x) / cos(x)\` with both halves taken at the ambient precision, matching the
    /// reference's numeric evaluator (\`NumOps.Tan\`). \`None\` at a pole.
    pub fn tan(&self, digits: usize) -> Option<Real> {
        let digits = digits.max(1);
        let sin = complex_math_sin(self, digits);
        let cos = complex_math_cos(self, digits);
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
        let arg = if big {
            div_truncate(&one(), &ax, work)
        } else {
            ax
        };
        let mut y = arg;
        for _ in 0..3 {
            let denom = one().add(&y.mul(&y)).sqrt(work);
            y = div_truncate(&y, &one().add(&denom), work);
        }
        let atan_arg = atan_series(&y, work).mul(&Real::from_i64(8));
        let mut result = if big {
            Real::pi(work)
                .div(&two(), work)
                .unwrap_or_else(Real::zero)
                .sub(&atan_arg)
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
        if self.cmp(&one()) == Ordering::Greater || self.cmp(&Real::from_i64(-1)) == Ordering::Less
        {
            return None;
        }
        let half_pi = Real::pi(digits)
            .div(&two(), digits)
            .unwrap_or_else(Real::zero);
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
        let half_pi = Real::pi(digits)
            .div(&two(), digits)
            .unwrap_or_else(Real::zero);
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
            let branch = if y.int.is_negative() {
                half_pi.neg()
            } else {
                half_pi
            };
            return with_two_argument_provenance(branch, y, x);
        }
        let work = digits + GUARD_DIGITS;
        // The reference's Atan2 divides with DivideTruncate (a fixed-point truncation), not
        // with the ambient period-preserving division: a periodic quotient such as -0.(3) is
        // not a value the atan series may be handed, and the reference never hands it one.
        let at = div_truncate(y, x, work).atan(digits);
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
        if self.cmp(&one()) != Ordering::Less || self.cmp(&Real::from_i64(-1)) != Ordering::Greater
        {
            return None;
        }
        let left = one().add(self).ln(digits)?;
        let right = one().sub(self).ln(digits)?;
        Some(
            left.sub(&right)
                .div(&two(), digits)
                .unwrap_or_else(Real::zero),
        )
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
        self.exp(digits)
            .sub(&self.neg().exp(digits))
            .div(&two(), digits)
            .unwrap_or_else(Real::zero)
    }

    /// The reference \`RealCosh\`.
    pub fn cosh_hyperbolic(&self, digits: usize) -> Real {
        let digits = digits.max(1);
        self.exp(digits)
            .add(&self.neg().exp(digits))
            .div(&two(), digits)
            .unwrap_or_else(Real::zero)
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
    if x.exp < 0 { -x.exp } else { 0 }
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
        sum = if k & 1 == 1 {
            sum.sub(&contrib)
        } else {
            sum.add(&contrib)
        };
        if contrib.abs().cmp(&threshold) == Ordering::Less {
            break;
        }
    }
    sum
}

// ---------------------------------------------------------------------------
// The reference's Real.Sin / Real.Cos kernel (Lovelace.Real/Real.cs)
//
// TWO trigonometric kernels exist in the reference, and they are not the same
// function: ComplexMath.Sin/Cos reduce modulo 2*pi with an i64 turn quotient
// (the numeric evaluator's path), while Real.Sin/Cos ask the special-angle
// table first, carry the argument-aware precision, and reduce with an INTEGER
// quotient, so 10^20 and 10^100 are measured instead of overflowing. The
// sin_cos_raw pair below is the ComplexMath port; the functions that follow
// port the Real.Sin/Cos contract.
// ---------------------------------------------------------------------------

/// The reference's ComplexMath.Sin: the numeric evaluator's path, kept explicit so the wire's
/// evalf keeps comparing against the same function it always did.
pub fn complex_math_sin(value: &Real, digits: usize) -> Real {
    let digits = digits.max(1);
    truncate_frac(&sin_cos_at_precision(value, digits).0, digits)
}

/// The reference's ComplexMath.Cos.
pub fn complex_math_cos(value: &Real, digits: usize) -> Real {
    let digits = digits.max(1);
    truncate_frac(&sin_cos_at_precision(value, digits).1, digits)
}

/// 10^-places as an exact literal.
fn ten_to_the_neg_places(places: usize) -> Real {
    ten_to_the_neg(places)
}

/// The number of INTEGER digits of a value (0 for anything below 1): the magnitude's decimal width
/// plus the exponent. Reading the exponent alone is wrong for a decimal literal, which reports
/// exponent 0 however long it is (10^100).
fn trig_integer_digits(value: &Real) -> i64 {
    const MAX_EXTRA: i64 = i32::MAX as i64 - 16;
    if value.period_len > 0 {
        return 0;
    }
    let digits = value.int.magnitude().to_decimal().len() as i64 + value.exp;
    digits.clamp(0, MAX_EXTRA)
}

/// The decimal place count the kernel must compute at for THIS argument: the request plus the
/// integer digits the reduction's cancellation spends (never below the ambient request).
pub(crate) fn trig_reduction_places(value: &Real, digits: usize) -> usize {
    let required = digits as i64 + 10 + trig_integer_digits(value);
    if required > digits as i64 {
        required as usize
    } else {
        digits
    }
}

/// The places pi must carry for a reduction that measures the angle: the argument's own fractional
/// places plus the request plus the guard, or the working width.
pub(crate) fn trig_pi_places(value: &Real, digits: usize, working: usize) -> usize {
    let argument_places = if value.period_len > 0 {
        0
    } else if value.exp < 0 {
        -value.exp
    } else {
        0
    };
    let required = trig_integer_digits(value) + argument_places + digits as i64 + 12;
    if required > working as i64 {
        required as usize
    } else {
        working
    }
}

pub(crate) fn truncate_toward_zero(value: &Real) -> Real {
    Real::from_str(&value.to_decimal_fixed(0)).unwrap_or_else(Real::zero)
}

/// The integer part of left / right (truncated toward zero) with one integer division: the
/// reference's WholeQuotient, the reason a ~1.6e99 turn count never narrows through an i64.
pub(crate) fn whole_quotient(left: &Real, right: &Real) -> Real {
    if left.period_len > 0 || right.period_len > 0 {
        let quotient = left
            .div(right, Real::DEFAULT_COMPUTATION_DECIMAL_PLACES)
            .unwrap_or_else(Real::zero);
        return truncate_toward_zero(&quotient);
    }
    let mut numerator = left.int.magnitude().clone();
    let mut denominator = right.int.magnitude().clone();
    let adjust = left.exp - right.exp;
    if adjust > 0 {
        numerator = numerator.mul_pow10(adjust as usize);
    } else if adjust < 0 {
        denominator = denominator.mul_pow10((-adjust) as usize);
    }
    let Some((whole, _)) = numerator.div_rem(&denominator) else {
        return Real::zero();
    };
    let negative = left.int.is_negative() != right.int.is_negative() && !whole.is_zero();
    let mut integer = int_from_mag(&whole);
    if negative {
        integer = integer.negate();
    }
    let value = Real {
        int: integer,
        exp: 0,
        period_start: -1,
        period_len: 0,
        inexact: left.inexact || right.inexact,
    };
    value.normalized()
}

/// Reduces value into [0, 2*pi) with the integer quotient.
pub(crate) fn reduce_to_two_pi_whole(value: &Real, two_pi: &Real) -> Real {
    if value.cmp(&Real::zero()) != Ordering::Less && value.cmp(two_pi) == Ordering::Less {
        return value.clone();
    }
    let turns = whole_quotient(value, two_pi);
    let mut reduced = value.sub(&two_pi.mul(&turns));
    if reduced.cmp(&Real::zero()) == Ordering::Less {
        reduced = reduced.add(two_pi);
    }
    reduced
}

/// Exactly value / 2 (the reference's HalfOf): halving a decimal only moves its point one place.
fn half_of(value: &Real, places: usize) -> Real {
    value
        .div(&Real::from_i64(2), places + 2)
        .unwrap_or_else(Real::zero)
}

/// The reference's TryTerminateExactly: a quotient whose reduced denominator is 2^a*5^b is answered
/// exactly, everything else is not (the caller then truncates at pi's own scale).
fn try_terminate_exactly(numerator: &Nat, denominator: &Nat) -> Option<(Nat, i64)> {
    let mut probe = denominator.clone();
    let mut twos = 0i64;
    let mut fives = 0i64;
    while let Some((quotient, remainder)) = probe.div_rem(&Nat::from_u64(2)) {
        if !remainder.is_zero() {
            break;
        }
        probe = quotient;
        twos += 1;
    }
    while let Some((quotient, remainder)) = probe.div_rem(&Nat::from_u64(5)) {
        if !remainder.is_zero() {
            break;
        }
        probe = quotient;
        fives += 1;
    }
    if probe.to_decimal() != "1" {
        return None;
    }
    if twos >= fives {
        let magnitude = if twos == fives {
            numerator.clone()
        } else {
            numerator.mul(&Nat::from_u64(5).pow((twos - fives) as u64))
        };
        Some((magnitude, twos))
    } else {
        Some((
            numerator.mul(&Nat::from_u64(2).pow((fives - twos) as u64)),
            fives,
        ))
    }
}

/// The value pi*num/den produces at pi's own scale, in integer arithmetic: the exact terminating
/// quotient when den is 2^a*5^b, otherwise the truncating division, exactly as the reference builds
/// the candidate angle it compares against.
pub(crate) fn special_angle_candidate(pi: &Real, num: u64, den: u64) -> Real {
    let numerator = pi.int.magnitude().mul(&Nat::from_u64(num));
    let denominator = Nat::from_u64(den);
    if let Some((magnitude, scale)) = try_terminate_exactly(&numerator, &denominator) {
        return Real {
            int: int_from_mag(&magnitude),
            exp: pi.exp - scale,
            period_start: -1,
            period_len: 0,
            inexact: false,
        }
        .normalized();
    }
    let (magnitude, _) = numerator
        .div_rem(&denominator)
        .unwrap_or((Nat::zero(), Nat::zero()));
    Real {
        int: int_from_mag(&magnitude),
        exp: pi.exp,
        period_start: -1,
        period_len: 0,
        inexact: false,
    }
    .normalized()
}

/// The 16 exact sin/cos pairs of the special-angle table (multiples of pi/6 and pi/4 in [0, 2*pi)).
pub(crate) fn special_angle_table(places: usize) -> [(u64, u64, Real, Real); 16] {
    let half = Real::from_str("0.5").unwrap_or_else(Real::zero);
    let neg_half = Real::from_str("-0.5").unwrap_or_else(Real::zero);
    let neg_one = Real::from_i64(-1);
    let sqrt2_half = Real::from_i64(2)
        .sqrt(places)
        .div(&Real::from_i64(2), places + 5)
        .unwrap_or_else(Real::zero);
    let sqrt3_half = Real::from_i64(3)
        .sqrt(places)
        .div(&Real::from_i64(2), places + 5)
        .unwrap_or_else(Real::zero);
    [
        (0, 1, Real::zero(), Real::from_i64(1)),
        (1, 6, half.clone(), sqrt3_half.clone()),
        (1, 4, sqrt2_half.clone(), sqrt2_half.clone()),
        (1, 3, sqrt3_half.clone(), half.clone()),
        (1, 2, Real::from_i64(1), Real::zero()),
        (2, 3, sqrt3_half.clone(), neg_half.clone()),
        (3, 4, sqrt2_half.clone(), sqrt2_half.neg()),
        (5, 6, half.clone(), sqrt3_half.neg()),
        (1, 1, Real::zero(), neg_one.clone()),
        (7, 6, neg_half.clone(), sqrt3_half.neg()),
        (5, 4, sqrt2_half.neg(), sqrt2_half.neg()),
        (4, 3, sqrt3_half.neg(), neg_half.clone()),
        (3, 2, neg_one.clone(), Real::zero()),
        (5, 3, sqrt3_half.neg(), half.clone()),
        (7, 4, sqrt2_half.neg(), sqrt2_half.clone()),
        (11, 6, neg_half, sqrt3_half),
    ]
}

/// The reference's TrySpecialAngle: the exact sin/cos when x is one of the 16 table angles, else
/// None. The comparison is against pi at the CALLER's scale, which is what makes sin(pi) exactly 0.
pub(crate) fn try_special_angle(x: &Real, pi: &Real, places: usize) -> Option<(Real, Real)> {
    let table = special_angle_table(places);
    for (num, den, sin, cos) in table {
        let angle = if num == 0 {
            Real::zero()
        } else {
            special_angle_candidate(pi, num, den)
        };
        // A non-periodic x whose own last stored place sits above the angle's cannot equal it.
        if x.period_len == 0 && x.exp > angle.exp {
            continue;
        }
        if x.cmp(&angle) == Ordering::Equal {
            return Some((sin, cos));
        }
    }
    None
}

/// |value| < 10^-(guard+1), the series' termination threshold and the cosine's cancellation test.
pub(crate) fn below_decimal_guard(value: &Real, guard: usize) -> bool {
    value.abs().cmp(&ten_to_the_neg_places(guard + 1)) == Ordering::Less
}

/// sin(r) at guard places: the reference's SinTaylor (threshold 10^-(guard+1)).
pub(crate) fn sin_taylor_guarded(r: &Real, guard: usize) -> Real {
    let x2 = r.mul(r);
    let mut term = r.clone();
    let mut sum = r.clone();
    for k in 1..=(5 * guard + 60) {
        let denom = (2 * k) * (2 * k + 1);
        term = div_truncate(&term.neg().mul(&x2), &Real::from_i64(denom as i64), guard);
        sum = sum.add(&term);
        if below_decimal_guard(&term, guard) {
            break;
        }
    }
    sum
}

/// cos(r) at guard places: the reference's CosTaylor (threshold 10^-(guard+1)).
pub(crate) fn cos_taylor_guarded(r: &Real, guard: usize) -> Real {
    let x2 = r.mul(r);
    let mut term = Real::from_i64(1);
    let mut sum = Real::from_i64(1);
    for k in 1..=(5 * guard + 60) {
        let denom = (2 * k - 1) * (2 * k);
        term = div_truncate(&term.neg().mul(&x2), &Real::from_i64(denom as i64), guard);
        sum = sum.add(&term);
        if below_decimal_guard(&term, guard) {
            break;
        }
    }
    sum
}

/// Real.Sin(value, digits): the special-angle table first (against the caller's own pi), then an
/// argument-aware reduction with an INTEGER turn quotient and the series at digits+10 guard places.
fn real_sin(value: &Real, digits: usize) -> Real {
    let digits = digits.max(1);
    if value.cmp(&Real::zero()) == Ordering::Less {
        return real_sin(&value.neg(), digits).neg();
    }
    // (1) the table, against the caller's pi and in the caller's scope.
    let ambient_pi = Real::pi(digits);
    let ambient_two_pi = ambient_pi.mul(&Real::from_i64(2));
    let ambient_x = reduce_to_two_pi_whole(value, &ambient_two_pi);
    if let Some((sin, _)) = try_special_angle(&ambient_x, &ambient_pi, digits) {
        return sin;
    }
    // (2) a general angle runs at the width THIS argument needs.
    let working = trig_reduction_places(value, digits);
    let guard = digits + 10;
    let pi = Real::pi(trig_pi_places(value, digits, working));
    let two_pi = pi.mul(&Real::from_i64(2));
    let half_pi = half_of(&pi, working);
    let mut x = reduce_to_two_pi_whole(value, &two_pi);
    if let Some((sin, _)) = try_special_angle(&x, &pi, working) {
        return sin;
    }
    let mut negative = false;
    if x.cmp(&pi) != Ordering::Greater {
        if x.cmp(&half_pi) == Ordering::Greater {
            x = pi.sub(&x);
        }
    } else if x.cmp(&pi.add(&half_pi)) != Ordering::Greater {
        x = x.sub(&pi);
        negative = true;
    } else {
        x = two_pi.sub(&x);
        negative = true;
    }
    let sin = sin_taylor_guarded(&x, guard).mark_inexact();
    if negative { sin.neg() } else { sin }
}

/// Real.Cos(value, digits), the reference's even-symmetry/table/reduction order.
fn real_cos(value: &Real, digits: usize) -> Real {
    let digits = digits.max(1);
    if value.cmp(&Real::zero()) == Ordering::Less {
        return real_cos(&value.neg(), digits);
    }
    let ambient_pi = Real::pi(digits);
    let ambient_two_pi = ambient_pi.mul(&Real::from_i64(2));
    let ambient_x = reduce_to_two_pi_whole(value, &ambient_two_pi);
    if let Some((_, cos)) = try_special_angle(&ambient_x, &ambient_pi, digits) {
        return cos;
    }
    let working = trig_reduction_places(value, digits);
    let guard = digits + 10;
    let pi = Real::pi(trig_pi_places(value, digits, working));
    let two_pi = pi.mul(&Real::from_i64(2));
    let half_pi = half_of(&pi, working);
    let mut x = reduce_to_two_pi_whole(value, &two_pi);
    if let Some((_, cos)) = try_special_angle(&x, &pi, working) {
        return cos;
    }
    let mut negate = false;
    if x.cmp(&pi) != Ordering::Greater {
        if x.cmp(&half_pi) == Ordering::Greater {
            x = pi.sub(&x);
            negate = true;
        }
    } else if x.cmp(&pi.add(&half_pi)) != Ordering::Greater {
        x = x.sub(&pi);
        negate = true;
    } else {
        x = two_pi.sub(&x);
    }
    // Within 10^-(guard+1) of pi/2 the cosine series is pure cancellation; there cos(x) =
    // sin(pi/2 - x) and the sine of a small argument has no cancellation to lose it in.
    let distance = half_pi.sub(&x);
    let cos = if below_decimal_guard(&distance, guard) {
        sin_taylor_guarded(&distance, guard)
    } else {
        cos_taylor_guarded(&x, guard)
    }
    .mark_inexact();
    if negate { cos.neg() } else { cos }
}
