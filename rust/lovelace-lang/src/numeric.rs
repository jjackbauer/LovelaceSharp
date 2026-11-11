//! Numeric evaluation for \`evalf\`: the reference's \`EvaluateToNum\` over the canonical
//! symbolic tree.
//!
//! The tiers mirror the reference exactly: exact rationals stay rational until a transcendental
//! forces a Real, the elementary functions run at the ambient decimal budget, and the published
//! value is truncated back to the requested count afterwards. A transcendental result is a
//! truncation, so it is inexact; an exact rational that fits the window stays exact.

use std::cmp::Ordering;

use lovelace_int::Int;
use lovelace_nat::Nat;
use lovelace_real::Real;
use lovelace_real::transcendental::truncate_frac;
use lovelace_symbolics::{Expr as Sym, Rat};

use crate::LangError;
use crate::complex::Complex;
use crate::complex_math;

/// A numeric value at the reference's three tiers.
#[derive(Clone, Debug)]
pub enum Num {
    /// An exact rational (the reference NumInt / NumRat pair).
    Exact(Rat),
    /// A fixed-point real computed at the ambient budget.
    Real(Real),
    /// A complex value.
    Complex(Complex),
}

/// Evaluates a symbolic expression numerically at \`places\` fractional digits.
pub fn evaluate(expression: &Sym, places: usize) -> Result<Num, LangError> {
    let places = places.max(1);
    match expression {
        Sym::Num(r) => Ok(Num::Exact(r.clone())),
        // A real constant is the decimal the numeric tier handed over; reading it back marks the
        // value inexact, so an approximation is never promoted to an exact rational.
        Sym::Real(text) => match Real::from_str(text) {
            Some(value) => Ok(Num::Real(value.mark_inexact())),
            None => Err(LangError::plain(format!(
                "The real constant '{text}' is not a decimal value."
            ))),
        },
        Sym::Sym(name) => Err(LangError::plain(format!("No value for symbol '{}'.", name))),
        Sym::Add(terms) => {
            let mut acc = Num::Exact(Rat::zero());
            for term in terms {
                acc = acc.add(&evaluate(term, places)?, places)?;
            }
            Ok(acc)
        }
        Sym::Mul(factors) => {
            let mut acc = Num::Exact(Rat::one());
            for factor in factors {
                acc = acc.mul(&evaluate(factor, places)?, places)?;
            }
            Ok(acc)
        }
        Sym::Div(left, right) => {
            let left = evaluate(left, places)?;
            let right = evaluate(right, places)?;
            left.div(&right, places)
        }
        Sym::Pow(base, exponent) => {
            let base = evaluate(base, places)?;
            let exponent = evaluate(exponent, places)?;
            base.pow(&exponent, places)
        }
        Sym::Rel(..) => Err(LangError::plain(
            "A relation has no numeric value; evaluate a number or an expression.",
        )),
        Sym::Fun(name, args) => {
            let mut values = Vec::with_capacity(args.len());
            for arg in args {
                values.push(evaluate(arg, places)?);
            }
            evaluate_function(name, &values, places)
        }
    }
}

impl Num {
    fn is_zero(&self) -> bool {
        match self {
            Num::Exact(r) => r.is_zero(),
            Num::Real(r) => r.is_zero(),
            Num::Complex(z) => z.re().is_zero() && z.im().is_zero(),
        }
    }

    /// The value as a Real at the ambient budget. C# \`NumOps.ToReal\` refuses a complex value.
    pub fn to_real(&self, places: usize) -> Option<Real> {
        match self {
            Num::Exact(r) => Some(rat_to_real(r, places)),
            Num::Real(r) => Some(r.clone()),
            Num::Complex(_) => None,
        }
    }

    /// The value as a complex number at the ambient budget.
    pub fn to_complex(&self, places: usize) -> Option<Complex> {
        match self {
            Num::Complex(z) => Some(z.clone()),
            _ => self.to_real(places).map(Complex::from_real),
        }
    }

    /// The reference \`NumOps.Add\`.
    pub fn add(&self, other: &Num, places: usize) -> Result<Num, LangError> {
        match (self, other) {
            (Num::Exact(a), Num::Exact(b)) => Ok(Num::Exact(a.add(b))),
            (Num::Complex(a), _) => Ok(Num::Complex(a.add_at(
                &other.to_complex(places).ok_or_else(complex_to_real_error)?,
                places,
            ))),
            (_, Num::Complex(b)) => Ok(Num::Complex(
                self.to_complex(places)
                    .ok_or_else(complex_to_real_error)?
                    .add_at(b, places),
            )),
            _ => Ok(Num::Real(
                self.to_real(places)
                    .ok_or_else(complex_to_real_error)?
                    .add(&other.to_real(places).ok_or_else(complex_to_real_error)?),
            )),
        }
    }

    /// The reference \`NumOps.Subtract\`.
    pub fn sub(&self, other: &Num, places: usize) -> Result<Num, LangError> {
        match (self, other) {
            (Num::Exact(a), Num::Exact(b)) => Ok(Num::Exact(a.add(&b.neg()))),
            (Num::Complex(a), _) => Ok(Num::Complex(a.sub_at(
                &other.to_complex(places).ok_or_else(complex_to_real_error)?,
                places,
            ))),
            (_, Num::Complex(b)) => Ok(Num::Complex(
                self.to_complex(places)
                    .ok_or_else(complex_to_real_error)?
                    .sub_at(b, places),
            )),
            _ => Ok(Num::Real(
                self.to_real(places)
                    .ok_or_else(complex_to_real_error)?
                    .sub(&other.to_real(places).ok_or_else(complex_to_real_error)?),
            )),
        }
    }

    /// The reference \`NumOps.Multiply\`.
    pub fn mul(&self, other: &Num, places: usize) -> Result<Num, LangError> {
        match (self, other) {
            (Num::Exact(a), Num::Exact(b)) => Ok(Num::Exact(a.mul(b))),
            (Num::Complex(a), _) => Ok(Num::Complex(a.mul_at(
                &other.to_complex(places).ok_or_else(complex_to_real_error)?,
                places,
            ))),
            (_, Num::Complex(b)) => Ok(Num::Complex(
                self.to_complex(places)
                    .ok_or_else(complex_to_real_error)?
                    .mul_at(b, places),
            )),
            _ => Ok(Num::Real(
                self.to_real(places)
                    .ok_or_else(complex_to_real_error)?
                    .mul(&other.to_real(places).ok_or_else(complex_to_real_error)?),
            )),
        }
    }

    /// The reference \`NumOps.Divide\`.
    pub fn div(&self, other: &Num, places: usize) -> Result<Num, LangError> {
        if other.is_zero() {
            return Err(LangError::coded(
                "Division by zero during numeric evaluation.",
                0,
                "DivisionByZero",
                "DomainError",
            ));
        }
        match (self, other) {
            (Num::Exact(a), Num::Exact(b)) => a
                .div(b)
                .map(Num::Exact)
                .ok_or_else(|| LangError::plain("Division by zero during numeric evaluation.")),
            (Num::Complex(a), _) => Ok(Num::Complex(
                a.div_at(
                    &other.to_complex(places).ok_or_else(complex_to_real_error)?,
                    places,
                )
                .ok_or_else(|| LangError::plain("Division by zero during numeric evaluation."))?,
            )),
            (_, Num::Complex(b)) => Ok(Num::Complex(
                self.to_complex(places)
                    .ok_or_else(complex_to_real_error)?
                    .div_at(b, places)
                    .ok_or_else(|| {
                        LangError::plain("Division by zero during numeric evaluation.")
                    })?,
            )),
            _ => Ok(Num::Real(
                self.to_real(places)
                    .ok_or_else(complex_to_real_error)?
                    .div(
                        &other.to_real(places).ok_or_else(complex_to_real_error)?,
                        places,
                    )
                    .ok_or_else(|| {
                        LangError::plain("Division by zero during numeric evaluation.")
                    })?,
            )),
        }
    }

    /// Negation.
    pub fn neg(&self) -> Num {
        match self {
            Num::Exact(r) => Num::Exact(r.neg()),
            Num::Real(r) => Num::Real(r.neg()),
            Num::Complex(z) => Num::Complex(z.neg()),
        }
    }

    fn is_integral(&self) -> Option<&Int> {
        match self {
            Num::Exact(r) if r.is_integer() => Some(r.numerator()),
            _ => None,
        }
    }

    /// The reference \`NumOps.Pow\`.
    pub fn pow(&self, exponent: &Num, places: usize) -> Result<Num, LangError> {
        if self.is_zero()
            && matches!(exponent, Num::Exact(r) if r.is_integer() && r.numerator().sign() <= 0)
        {
            return Err(LangError::plain("0 raised to a non-positive power."));
        }
        // Zero raised to a POSITIVE power is exactly zero at every tier: 0^(1/2) = 0, not a
        // complex principal branch and not an approximation. The non-positive exponents were
        // refused above, so what reaches here is positive.
        if self.is_zero() {
            return Ok(Num::Exact(Rat::zero()));
        }
        if let Some(n) = exponent
            .is_integral()
            .and_then(|i| i.to_decimal().parse::<i64>().ok())
        {
            return self.pow_int(n, places);
        }
        // b^e = exp(e ln b): the real path for a positive real base, the principal branch otherwise.
        if let Some(base) = self.to_real(places) {
            if base.cmp(&Real::zero()) == Ordering::Greater {
                let e = exponent.to_real(places).ok_or_else(complex_to_real_error)?;
                return base
                    .pow_real(&e, places)
                    .map(Num::Real)
                    .ok_or_else(|| LangError::plain("Pow(b, e) requires b > 0."));
            }
        }
        let base = self.to_complex(places).ok_or_else(complex_to_real_error)?;
        let e = exponent
            .to_complex(places)
            .ok_or_else(complex_to_real_error)?;
        complex_math::pow(&base, &e, places)
            .map(Num::Complex)
            .ok_or_else(|| LangError::plain("Pow(b, e) requires b > 0."))
    }

    fn pow_int(&self, n: i64, places: usize) -> Result<Num, LangError> {
        if n == 0 {
            return Ok(Num::Exact(Rat::one()));
        }
        if n < 0 {
            let positive = self.pow_int(-n, places)?;
            return Num::Exact(Rat::one()).div(&positive, places);
        }
        let mut result = Num::Exact(Rat::one());
        let mut factor = self.clone();
        let mut e = n;
        while e > 0 {
            if e & 1 == 1 {
                result = result.mul(&factor, places)?;
            }
            factor = factor.mul(&factor, places)?;
            e >>= 1;
        }
        Ok(result)
    }

    /// An upper bound on the value's integer digits, used to size the working precision exactly
    /// as the reference does.
    pub fn integer_digits(&self) -> usize {
        match self {
            Num::Exact(r) => {
                if r.is_zero() {
                    return 0;
                }
                let num = r.numerator().magnitude().to_decimal().len() as i64;
                let den = r.denominator().to_decimal().len() as i64;
                (num - den + 1).max(0) as usize
            }
            Num::Real(value) => integer_digits_of_real(value),
            Num::Complex(z) => integer_digits_of_real(z.re()).max(integer_digits_of_real(z.im())),
        }
    }

    /// Bounds the evaluated value to the requested decimal count (the reference \`EvalfPayload\`).
    pub fn into_value(self, digits: usize) -> Result<Value, LangError> {
        let places = digits.min(1000);
        match self {
            Num::Exact(r) => {
                if r.is_integer() {
                    Ok(Value::Integer(r.numerator().clone()))
                } else {
                    Ok(Value::Real(rational_to_published(&r, places)))
                }
            }
            Num::Real(r) => Ok(Value::Real(truncate_frac(&r, places))),
            Num::Complex(z) => Ok(Value::Complex(
                truncate_frac(z.re(), places),
                truncate_frac(z.im(), places),
            )),
        }
    }
}

use crate::Value;

fn integer_digits_of_real(value: &Real) -> usize {
    if value.is_zero() {
        return 0;
    }
    // The reference reads the stored magnitude width plus the exponent; the rendered integer
    // part is the same bound. A value below one has zero integer digits.
    let text = value.to_string();
    let body = text.trim_start_matches('-');
    let int_part = body.split('.').next().unwrap_or(body);
    if int_part == "0" { 0 } else { int_part.len() }
}

fn domain_argument_error(message: &str) -> LangError {
    LangError::coded(
        format!("{message} (Parameter 'x')"),
        0,
        "InvalidArgument",
        "TypeMismatch",
    )
}

fn ln_domain_error() -> LangError {
    LangError::coded(
        "Ln(x) requires x > 0. (Parameter 'x')",
        0,
        "EvaluationError",
        "DomainError",
    )
}

fn complex_to_real_error() -> LangError {
    LangError::plain("Complex value cannot convert to Real (use re()/im()).")
}

/// \`RationalReal.ToReal\` at the ambient precision: exactly \`places\` fractional digits, truncated.
/// The result is exact when the expansion terminates, inexact otherwise (the reference \`Real.Parse\`
/// boundary at the same width).
fn rat_to_real(value: &Rat, places: usize) -> Real {
    let num = value.numerator();
    let den = value.denominator();
    let negative = num.is_negative();
    let mag = num.magnitude();
    let (whole, mut rem) = mag.div_rem(den).expect("nonzero divisor");
    let mut digits = String::new();
    for _ in 0..places {
        rem = rem.mul(&Nat::from_u64(10));
        let (d, next) = rem.div_rem(den).expect("nonzero divisor");
        digits.push_str(&d.to_decimal());
        rem = next;
    }
    let mut all = whole.to_decimal();
    all.push_str(&digits);
    let magnitude = Nat::from_decimal(&all).unwrap_or_else(Nat::zero);
    let mut result = Real::from_str(&format!(
        "{}{}.{}",
        if negative { "-" } else { "" },
        whole.to_decimal(),
        digits
    ))
    .unwrap_or_else(Real::zero);
    if !rem.is_zero() {
        result = result.mark_inexact();
    }
    let _ = magnitude;
    result
}

/// The published form of an exact rational: bounded to the requested digit count. The payload
/// path is exact unless the reference 1000-place literal boundary marks the rendered digits inexact.
fn rational_to_published(value: &Rat, places: usize) -> Real {
    let num = value.numerator();
    let den = value.denominator();
    let negative = num.is_negative();
    let mag = num.magnitude();
    let (whole, mut rem) = mag.div_rem(den).expect("nonzero divisor");
    let mut digits = String::new();
    for _ in 0..places {
        rem = rem.mul(&Nat::from_u64(10));
        let (d, next) = rem.div_rem(den).expect("nonzero divisor");
        digits.push_str(&d.to_decimal());
        rem = next;
    }
    let mut text = String::new();
    if negative {
        text.push('-');
    }
    text.push_str(&whole.to_decimal());
    if places > 0 {
        text.push('.');
        text.push_str(&digits);
    }
    let result = Real::from_str(&text).unwrap_or_else(Real::zero);
    if places >= 1000 && !rem.is_zero() {
        result.mark_inexact()
    } else {
        result
    }
}

fn unary<'a>(name: &str, values: &'a [Num]) -> Result<&'a Num, LangError> {
    if values.len() != 1 {
        return Err(LangError::plain(format!(
            "{name}(): expected 1 argument; got {}.",
            values.len()
        )));
    }
    Ok(&values[0])
}

fn evaluate_function(name: &str, values: &[Num], places: usize) -> Result<Num, LangError> {
    // The reference folds total functions at their exact special points BEFORE evaluation can
    // see them (Exp 0 = 1, Sin 0 = 0, ...); the folded value is exact.
    if let [Num::Exact(r)] = values {
        let one = Rat::one();
        let zero = Rat::zero();
        let folded = match name {
            "exp" if r.is_zero() => Some(one.clone()),
            "sin" if r.is_zero() => Some(zero.clone()),
            "cos" if r.is_zero() => Some(one.clone()),
            "tan" if r.is_zero() => Some(zero.clone()),
            "log" if *r == one => Some(zero.clone()),
            "atan" if r.is_zero() => Some(zero.clone()),
            "asin" if r.is_zero() => Some(zero.clone()),
            "acos" if *r == one => Some(zero.clone()),
            "sinh" if r.is_zero() => Some(zero.clone()),
            "cosh" if r.is_zero() => Some(one.clone()),
            "tanh" if r.is_zero() => Some(zero.clone()),
            "asinh" if r.is_zero() => Some(zero.clone()),
            "acosh" if *r == one => Some(zero.clone()),
            "atanh" if r.is_zero() => Some(zero.clone()),
            _ => None,
        };
        if let Some(value) = folded {
            return Ok(Num::Exact(value));
        }
    }
    match name {
        "exp" => {
            let x = unary(name, values)?;
            match x {
                Num::Complex(z) => Ok(Num::Complex(complex_math::exp(z, places))),
                _ => Ok(Num::Real(
                    x.to_real(places)
                        .ok_or_else(complex_to_real_error)?
                        .exp(places),
                )),
            }
        }
        "log" => {
            let x = unary(name, values)?;
            match x {
                Num::Complex(z) => complex_math::log(z, places)
                    .map(Num::Complex)
                    .ok_or_else(|| LangError::plain("log(0) is undefined.")),
                _ => {
                    let real = x.to_real(places).ok_or_else(complex_to_real_error)?;
                    if real.cmp(&Real::zero()) == Ordering::Less {
                        complex_math::log(&Complex::from_real(real), places)
                            .map(Num::Complex)
                            .ok_or_else(|| LangError::plain("log(0) is undefined."))
                    } else if real.is_zero() {
                        Err(ln_domain_error())
                    } else {
                        real.ln(places).map(Num::Real).ok_or_else(ln_domain_error)
                    }
                }
            }
        }
        "sin" | "cos" | "tan" => {
            let x = unary(name, values)?;
            match x {
                Num::Complex(z) => match name {
                    "sin" => Ok(Num::Complex(complex_math::sin(z, places))),
                    "cos" => Ok(Num::Complex(complex_math::cos(z, places))),
                    _ => complex_math::tan(z, places)
                        .map(Num::Complex)
                        .ok_or_else(|| {
                            LangError::plain("Division by zero during numeric evaluation.")
                        }),
                },
                _ => {
                    let real = x.to_real(places).ok_or_else(complex_to_real_error)?;
                    match name {
                        "sin" => Ok(Num::Real(truncate_frac(
                            &real.sin_cos_raw(places).0,
                            places,
                        ))),
                        "cos" => Ok(Num::Real(truncate_frac(
                            &real.sin_cos_raw(places).1,
                            places,
                        ))),
                        _ => real.tan(places).map(Num::Real).ok_or_else(|| {
                            LangError::plain("Division by zero during numeric evaluation.")
                        }),
                    }
                }
            }
        }
        "atan" => {
            let x = unary(name, values)?;
            let real = x
                .to_real(places)
                .ok_or_else(|| LangError::plain("atan is real-only in v1."))?;
            Ok(Num::Real(real.atan(places)))
        }
        "asin" | "acos" => {
            let x = unary(name, values)?;
            let real = x.to_real(places).ok_or_else(complex_to_real_error)?;
            let value = if name == "asin" {
                real.asin(places)
            } else {
                real.acos(places)
            };
            value
                .map(Num::Real)
                .ok_or_else(|| domain_argument_error("AsinReal(x) requires |x| <= 1."))
        }
        "sinh" | "cosh" | "tanh" => {
            let x = unary(name, values)?;
            let real = x.to_real(places).ok_or_else(complex_to_real_error)?;
            match name {
                "sinh" => Ok(Num::Real(real.sinh(places))),
                "cosh" => Ok(Num::Real(real.cosh(places))),
                _ => Ok(Num::Real(real.tanh(places))),
            }
        }
        "asinh" => {
            let x = unary(name, values)?;
            let real = x.to_real(places).ok_or_else(complex_to_real_error)?;
            Ok(Num::Real(real.asinh(places)))
        }
        "acosh" => {
            let x = unary(name, values)?;
            let real = x.to_real(places).ok_or_else(complex_to_real_error)?;
            real.acosh(places)
                .map(Num::Real)
                .ok_or_else(|| domain_argument_error("AcoshReal(x) requires x >= 1."))
        }
        "atanh" => {
            let x = unary(name, values)?;
            let real = x.to_real(places).ok_or_else(complex_to_real_error)?;
            real.atanh(places)
                .map(Num::Real)
                .ok_or_else(|| domain_argument_error("AtanhReal(x) requires |x| < 1."))
        }
        // The named mathematical constants of the reference's numeric tier: pi and e are real
        // approximations at the ambient budget, i is the exact imaginary unit. The solver's
        // residual check needs them (a candidate such as 2*i must be numerically checkable).
        "pi" if values.is_empty() => Ok(Num::Real(Real::pi(places))),
        "e" if values.is_empty() => Ok(Num::Real(Real::e(places))),
        "i" if values.is_empty() => Ok(Num::Complex(Complex::i())),
        "sqrt" => {
            let x = unary(name, values)?;
            x.pow(
                &Num::Exact(Rat::from_i64(1).div(&Rat::from_i64(2)).unwrap()),
                places,
            )
        }
        "abs" => {
            let x = unary(name, values)?;
            match x {
                Num::Exact(r) => Ok(Num::Exact(r.abs())),
                Num::Real(r) => Ok(Num::Real(r.abs())),
                Num::Complex(z) => Ok(Num::Real(z.magnitude(places))),
            }
        }
        _ => Err(LangError::plain(format!(
            "Function '{}' has no numeric evaluator.",
            name
        ))),
    }
}
