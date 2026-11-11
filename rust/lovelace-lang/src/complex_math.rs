//! The reference \`Lovelace.Complex/ComplexMath.cs\` complex surface.
//!
//! Every entry point computes at the ambient decimal budget, truncates both components back to
//! that budget, and re-attaches the argument's provenance to both components: a component computed
//! from an approximation is itself an approximation however exact its digits look.

use lovelace_real::transcendental::truncate_frac;
use lovelace_real::Real;

use crate::complex::Complex;

/// The reference ComplexMath.Sin/Cos of a REAL argument (the SinCosAtPrecision pair). The complex
/// surface composes these, not Real.Sin: the two kernels differ for a huge argument and for an
/// exact table angle, and this is the numeric evaluator's path.
fn real_sin(value: &Real, digits: usize) -> Real {
    truncate_frac(&value.sin_cos_raw(digits).0, digits)
}

fn real_cos(value: &Real, digits: usize) -> Real {
    truncate_frac(&value.sin_cos_raw(digits).1, digits)
}
fn truncate_pair(z: &Complex, digits: usize) -> Complex {
    Complex::new(truncate_frac(z.re(), digits), truncate_frac(z.im(), digits))
}

fn with_provenance(result: Complex, argument: &Complex) -> Complex {
    if argument.is_exact() {
        result
    } else {
        Complex::new(result.re().clone().mark_inexact(), result.im().clone().mark_inexact())
    }
}

/// \`e^(a+bi) = e^a (cos b + i sin b)\`.
pub fn exp(z: &Complex, digits: usize) -> Complex {
    let digits = digits.max(1);
    let e = z.re().exp(digits);
    let cos = real_cos(z.im(), digits);
    let sin = real_sin(z.im(), digits);
    with_provenance(truncate_pair(&Complex::new(e.mul(&cos), e.mul(&sin)), digits), z)
}

/// \`sin(a+bi) = sin a cosh b + i cos a sinh b\`.
pub fn sin(z: &Complex, digits: usize) -> Complex {
    let digits = digits.max(1);
    let a = z.re();
    let b = z.im();
    let value = Complex::new(
        real_sin(a, digits).mul(&b.cosh_hyperbolic(digits)),
        real_cos(a, digits).mul(&b.sinh_hyperbolic(digits)),
    );
    with_provenance(truncate_pair(&value, digits), z)
}

/// \`cos(a+bi) = cos a cosh b - i sin a sinh b\`.
pub fn cos(z: &Complex, digits: usize) -> Complex {
    let digits = digits.max(1);
    let a = z.re();
    let b = z.im();
    let value = Complex::new(
        real_cos(a, digits).mul(&b.cosh_hyperbolic(digits)),
        real_sin(a, digits).mul(&b.sinh_hyperbolic(digits)).neg(),
    );
    with_provenance(truncate_pair(&value, digits), z)
}

/// \`tan(z) = sin(z) / cos(z)\` at the ambient budget.
pub fn tan(z: &Complex, digits: usize) -> Option<Complex> {
    let digits = digits.max(1);
    let value = sin(z, digits).div_at(&cos(z, digits), digits)?;
    Some(with_provenance(truncate_pair(&value, digits), z))
}

/// \`sinh(a+bi) = sinh a cos b + i cosh a sin b\`.
pub fn sinh(z: &Complex, digits: usize) -> Complex {
    let digits = digits.max(1);
    let a = z.re();
    let b = z.im();
    let value = Complex::new(
        a.sinh_hyperbolic(digits).mul(&real_cos(b, digits)),
        a.cosh_hyperbolic(digits).mul(&real_sin(b, digits)),
    );
    with_provenance(truncate_pair(&value, digits), z)
}

/// \`cosh(a+bi) = cosh a cos b + i sinh a sin b\`.
pub fn cosh(z: &Complex, digits: usize) -> Complex {
    let digits = digits.max(1);
    let a = z.re();
    let b = z.im();
    let value = Complex::new(
        a.cosh_hyperbolic(digits).mul(&real_cos(b, digits)),
        a.sinh_hyperbolic(digits).mul(&real_sin(b, digits)),
    );
    with_provenance(truncate_pair(&value, digits), z)
}

/// Principal square root: \`s = sqrt((|z|+a)/2)\`, \`t = sqrt((|z|-a)/2)\`, sign of t from Im.
pub fn sqrt(z: &Complex, digits: usize) -> Complex {
    let digits = digits.max(1);
    let two = Real::from_i64(2);
    let mag = z.magnitude(digits);
    let a = z.re();
    let s = mag.add(a).div(&two, digits).unwrap_or_else(Real::zero).sqrt(digits);
    let diff = mag.sub(a);
    let diff = if diff.cmp(&Real::zero()) == std::cmp::Ordering::Less { Real::zero() } else { diff };
    let t = diff.div(&two, digits).unwrap_or_else(Real::zero).sqrt(digits);
    let value = if z.im().cmp(&Real::zero()) == std::cmp::Ordering::Less {
        Complex::new(s, t.neg())
    } else {
        Complex::new(s, t)
    };
    with_provenance(truncate_pair(&value, digits), z)
}

/// Principal logarithm \`ln|z| + i atan2(Im, Re)\`; \`None\` for zero.
pub fn log(z: &Complex, digits: usize) -> Option<Complex> {
    let digits = digits.max(1);
    if z.re().is_zero() && z.im().is_zero() {
        return None;
    }
    let magnitude = z.magnitude(digits);
    let angle = Real::atan2(z.im(), z.re(), digits);
    Some(truncate_pair(
        &Complex::new(magnitude.ln(digits)?, angle),
        digits,
    ))
}

/// Principal complex power \`b^e = exp(e log b)\`.
pub fn pow(base: &Complex, exponent: &Complex, digits: usize) -> Option<Complex> {
    let digits = digits.max(1);
    let log = log(base, digits)?;
    let product = exponent.mul_at(&log, digits);
    Some(exp(&product, digits))
}

