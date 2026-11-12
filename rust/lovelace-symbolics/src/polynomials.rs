//! The reference's `Polynomials` facade: content, primitive part, gcd/lcm, resultant,
//! discriminant and factorization over Q.
//!
//! Every routine takes coefficients in ASCENDING degree order, the same carrier the crate's
//! `poly_coeffs`/`poly_div`/`square_free_univariate` already use, and returns the reference's
//! exact values: a zero polynomial is `[0]`, a constant is `[c]`, and every emitted factor is
//! monic (the leading coefficient lives in the content, as the reference's `Factoring.PolyFactors`
//! does it).
//!
//! The algorithms are ported from `Lovelace.Symbolics/Algebra/Polynomials.cs` and
//! `Algebra/Factor.cs`; the comments record the reference branch each one mirrors.

use std::cmp::Ordering;

use lovelace_int::Int;
use lovelace_nat::Nat;

use crate::{
    Rat, integer_coefficients, poly_derivative, poly_div, poly_gcd, poly_remainder, rat_reduce,
    square_free_univariate, trim_poly,
};

/// True when every coefficient is zero (the zero polynomial, including an empty slice).
fn is_zero_poly(p: &[Rat]) -> bool {
    p.iter().all(|c| c.is_zero())
}

/// The gcd of two non-negative rationals: gcd(numerators) / lcm(denominators), the reference's
/// `Factoring.GcdRational`.
fn gcd_rational(a: &Rat, b: &Rat) -> Rat {
    let numerator = a.numerator().magnitude().gcd(b.numerator().magnitude());
    let denominator = a.denominator().lcm(b.denominator());
    let signed = Int::from_decimal(&numerator.to_decimal()).unwrap_or_else(Int::zero);
    rat_reduce(&Rat::from_parts(signed, denominator))
}

/// The rational content of a polynomial: the gcd of its nonzero coefficient magnitudes. The
/// reference answers one for the zero polynomial (its `PrimitivePart` splits zero as `(zero, 1)`).
pub fn content(coeffs: &[Rat]) -> Rat {
    let p = trim_poly(coeffs.to_vec());
    if is_zero_poly(&p) {
        return Rat::one();
    }
    let mut value = Rat::zero();
    for coefficient in &p {
        if coefficient.is_zero() {
            continue;
        }
        let magnitude = coefficient.abs();
        value = if value.is_zero() {
            magnitude
        } else {
            gcd_rational(&value, &magnitude)
        };
    }
    if value.is_zero() { Rat::one() } else { value }
}

/// The primitive part: the polynomial divided by its content. The zero polynomial is unchanged.
pub fn primitive_part(coeffs: &[Rat]) -> Vec<Rat> {
    let p = trim_poly(coeffs.to_vec());
    if is_zero_poly(&p) {
        return vec![Rat::zero()];
    }
    let divisor = content(&p);
    let mut out = Vec::with_capacity(p.len());
    for coefficient in &p {
        match coefficient.div(&divisor) {
            Some(quotient) => out.push(rat_reduce(&quotient)),
            None => out.push(coefficient.clone()),
        }
    }
    trim_poly(out)
}

/// The product of two coefficient vectors.
fn multiply(a: &[Rat], b: &[Rat]) -> Vec<Rat> {
    let a = trim_poly(a.to_vec());
    let b = trim_poly(b.to_vec());
    if is_zero_poly(&a) || is_zero_poly(&b) {
        return vec![Rat::zero()];
    }
    let mut out = vec![Rat::zero(); a.len() + b.len() - 1];
    for (i, left) in a.iter().enumerate() {
        if left.is_zero() {
            continue;
        }
        for (j, right) in b.iter().enumerate() {
            out[i + j] = rat_reduce(&out[i + j].add(&left.mul(right)));
        }
    }
    trim_poly(out)
}

/// The monic Euclidean gcd over Q (the reference's `Polynomial.GcdUnivariate`).
pub fn gcd(a: &[Rat], b: &[Rat]) -> Vec<Rat> {
    poly_gcd(a, b)
}

/// The least common multiple: `(a / gcd(a, b)) * b`, or zero when either operand is zero. This
/// is the reference's `Polynomials.Lcm`; note that for constants it is the product divided by the
/// gcd, so `lcm(6, 4) = 24`, not the integer least common multiple.
pub fn lcm(a: &[Rat], b: &[Rat]) -> Vec<Rat> {
    let a = trim_poly(a.to_vec());
    let b = trim_poly(b.to_vec());
    if is_zero_poly(&a) || is_zero_poly(&b) {
        return vec![Rat::zero()];
    }
    let divisor = poly_gcd(&a, &b);
    let quotient = match poly_div(&a, &divisor) {
        Some(value) => value,
        None => return vec![Rat::zero()],
    };
    multiply(&quotient, &b)
}

/// The exact determinant over Q by fraction-free Bareiss elimination, as the reference's
/// `Polynomials.Determinant`.
fn determinant(matrix: &[Vec<Rat>]) -> Rat {
    let n = matrix.len();
    if n == 0 {
        return Rat::one();
    }
    if n == 1 {
        return matrix[0][0].clone();
    }
    let mut m = matrix.to_vec();
    let mut previous = Rat::one();
    let mut sign: i64 = 1;
    for k in 0..n - 1 {
        let mut pivot = k;
        while pivot < n && m[pivot][k].is_zero() {
            pivot += 1;
        }
        if pivot == n {
            return Rat::zero();
        }
        if pivot != k {
            m.swap(pivot, k);
            sign = -sign;
        }
        for i in k + 1..n {
            for j in k + 1..n {
                let left = rat_reduce(&m[i][j].mul(&m[k][k]));
                let right = rat_reduce(&m[i][k].mul(&m[k][j]));
                let numerator = left.add(&right.neg());
                m[i][j] = match numerator.div(&previous) {
                    Some(value) => rat_reduce(&value),
                    None => Rat::zero(),
                };
            }
            m[i][k] = Rat::zero();
        }
        previous = m[k][k].clone();
    }
    if sign < 0 {
        m[n - 1][n - 1].neg()
    } else {
        m[n - 1][n - 1].clone()
    }
}

/// The resultant of two univariate polynomials as the determinant of the Sylvester matrix, the
/// reference's `Polynomials.Resultant`. Both zero or either zero answers zero; two constants
/// answer one.
pub fn resultant(f: &[Rat], g: &[Rat]) -> Rat {
    let f = trim_poly(f.to_vec());
    let g = trim_poly(g.to_vec());
    if is_zero_poly(&f) || is_zero_poly(&g) {
        return Rat::zero();
    }
    let m = f.len() - 1;
    let n = g.len() - 1;
    let size = m + n;
    if size == 0 {
        return Rat::one();
    }
    let mut matrix = vec![vec![Rat::zero(); size]; size];
    // n rows of f (ascending coefficients, shifted).
    for i in 0..n {
        for (j, coefficient) in f.iter().enumerate() {
            matrix[i][i + j] = coefficient.clone();
        }
    }
    // m rows of g (ascending coefficients, shifted).
    for i in 0..m {
        for (j, coefficient) in g.iter().enumerate() {
            matrix[n + i][i + j] = coefficient.clone();
        }
    }
    determinant(&matrix)
}

/// The discriminant: `(-1)^(n(n-1)/2) * res(f, f') / lc(f)`. Zero answers zero, a constant
/// answers one, and a linear polynomial answers one.
pub fn discriminant(f: &[Rat]) -> Rat {
    let f = trim_poly(f.to_vec());
    if is_zero_poly(&f) {
        return Rat::zero();
    }
    let n = f.len() - 1;
    if n == 0 {
        return Rat::one();
    }
    let leading = f[n].clone();
    let derivative = poly_derivative(&f);
    let resultant = resultant(&f, &derivative);
    let flip = ((n * (n - 1)) / 2) % 2 != 0;
    let signed = if flip { resultant.neg() } else { resultant };
    match signed.div(&leading) {
        Some(value) => rat_reduce(&value),
        None => Rat::zero(),
    }
}

/// The positive divisors of a magnitude, ascending. The trial division is capped like the crate's
/// existing rational-root enumeration so a pathological coefficient is a bounded answer rather
/// than a hang.
fn positive_divisors(n: &Nat) -> Vec<Nat> {
    if n.is_zero() {
        return vec![Nat::from_u64(1)];
    }
    let mut out = Vec::new();
    let mut divisor = Nat::from_u64(1);
    loop {
        let square = divisor.mul(&divisor);
        if square.cmp(n) == Ordering::Greater {
            break;
        }
        let Some((quotient, remainder)) = n.div_rem(&divisor) else {
            break;
        };
        if remainder.is_zero() {
            out.push(divisor.clone());
            if quotient.cmp(&divisor) != Ordering::Equal {
                out.push(quotient);
            }
        }
        if divisor
            .to_u64()
            .map(|value| value >= 100_000)
            .unwrap_or(true)
        {
            break;
        }
        divisor.increment();
    }
    out.sort();
    out.dedup();
    out
}

/// Every rational root of a polynomial, ascending, the reference's `Polynomial.RationalRoots`:
/// a zero root is extracted first when x divides the polynomial, then the rational root theorem
/// enumerates `±p/q` over the divisors of the constant and leading integer coefficients.
fn rational_roots(coeffs: &[Rat]) -> Vec<Rat> {
    let p = trim_poly(coeffs.to_vec());
    if is_zero_poly(&p) {
        return Vec::new();
    }
    let mut roots: Vec<Rat> = Vec::new();
    let first_nonzero = p.iter().position(|c| !c.is_zero()).unwrap_or(0);
    let reduced: Vec<Rat> = if first_nonzero > 0 {
        roots.push(Rat::zero());
        p[first_nonzero..].to_vec()
    } else {
        p.clone()
    };
    let Some((integers, _multiple)) = integer_coefficients(&reduced) else {
        return roots;
    };
    if integers.len() < 2 {
        return roots;
    }
    let constant = integers[0].magnitude();
    let leading = integers[integers.len() - 1].magnitude();
    if constant.is_zero() || leading.is_zero() {
        return roots;
    }
    let numerators = positive_divisors(constant);
    let denominators = positive_divisors(leading);
    for numerator in &numerators {
        for denominator in &denominators {
            for sign in [1i8, -1i8] {
                let magnitude =
                    Int::from_decimal(&numerator.to_decimal()).unwrap_or_else(Int::zero);
                let signed = if sign < 0 {
                    magnitude.negate()
                } else {
                    magnitude
                };
                let candidate = Rat::new(signed, denominator.clone());
                if roots
                    .iter()
                    .any(|root| root.cmp(&candidate) == Ordering::Equal)
                {
                    continue;
                }
                if crate::eval_poly(&reduced, &candidate)
                    .map(|value| value.is_zero())
                    .unwrap_or(false)
                {
                    roots.push(candidate);
                }
            }
        }
    }
    roots.sort();
    roots
}

/// The quotient and remainder of a univariate division, ascending coefficients.
fn div_rem(numerator: &[Rat], denominator: &[Rat]) -> (Vec<Rat>, Vec<Rat>) {
    let quotient = poly_div(numerator, denominator).unwrap_or_else(|| vec![Rat::zero()]);
    (
        trim_poly(quotient),
        trim_poly(poly_remainder(numerator, denominator)),
    )
}

/// The reference's `Factoring.PolyFactors`: a rational content and monic factors with
/// multiplicities.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolynomialFactors {
    /// The rational content factored out of the input.
    pub content: Rat,
    /// The factors, each an ascending coefficient vector with its multiplicity.
    pub factors: Vec<(Vec<Rat>, u64)>,
}

/// Factors a univariate polynomial over Q the way `Factoring.FactorPoly` does: content, Yun's
/// square-free decomposition, and one monic linear factor per rational root. Residual factors are
/// returned whole, never claimed irreducible.
///
/// The content carries the primitive part's leading coefficient when any factor was extracted, so
/// the content times the factors reconstructs the input exactly (including the sign of a negative
/// leading coefficient). The zero polynomial answers content zero and no factors; a nonzero
/// constant answers its absolute value as content and the signed unit as its one factor.
pub fn factor(coeffs: &[Rat]) -> PolynomialFactors {
    let p = trim_poly(coeffs.to_vec());
    if is_zero_poly(&p) {
        return PolynomialFactors {
            content: Rat::zero(),
            factors: Vec::new(),
        };
    }
    let primitive = primitive_part(&p);
    let primitive_content = content(&p);
    let leading = primitive.last().cloned().unwrap_or_else(Rat::one);
    let mut factors: Vec<(Vec<Rat>, u64)> = Vec::new();
    for (square_free, multiplicity) in square_free_univariate(&primitive) {
        let mut remaining = square_free;
        for root in rational_roots(&remaining) {
            let linear = vec![root.neg(), Rat::one()];
            factors.push((linear.clone(), multiplicity));
            let (quotient, remainder) = div_rem(&remaining, &linear);
            if is_zero_poly(&remainder) {
                remaining = quotient;
            }
        }
        if !(remaining.len() == 1 && remaining[0].is_one()) && !is_zero_poly(&remaining) {
            factors.push((remaining, multiplicity));
        }
    }
    let emitted = !factors.is_empty();
    if !emitted {
        // Nothing was emitted: the primitive part itself is a unit, and it already carries its
        // own sign, so the content is not scaled again (the reference's FactorPoly branch).
        factors.push((primitive, 1));
    }
    let content = if emitted {
        rat_reduce(&primitive_content.mul(&leading))
    } else {
        primitive_content
    };
    PolynomialFactors { content, factors }
}
