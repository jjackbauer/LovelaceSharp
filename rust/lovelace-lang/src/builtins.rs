//! Operators, indexing, ranges and the builtin function library.

use lovelace_array::{ArrayError, NdArray};
use lovelace_int::Int;
use lovelace_nat::Nat;
use lovelace_real::Real;
use lovelace_symbolics::Expr as Sym;
use lovelace_symbolics::assumptions::{
    Assumption, Domain as AssumptionDomain, SymbolPredicate, Tristate,
};
use lovelace_symbolics::groebner::{MonomialOrder, Polynomial, groebner_basis};

use crate::registry;
use crate::syntax::BinOp;
use crate::{Engine, ErrorSignature, LangError, PlotCapture, Value, cancelled_error};

/// One strided deadline check for a kernel loop. `Ok(())` means "keep going"; a cancelled run
/// returns the structured Cancelled refusal, which the evaluation positions at the statement that
/// was running. When no deadline is armed this is a single thread-local read.
#[inline]
fn poll(iteration: u64) -> Result<(), LangError> {
    if lovelace_symbolics::cancel::poll(iteration) {
        Err(cancelled_error())
    } else {
        Ok(())
    }
}

/// An unstrided check, for a recursion boundary or an outer loop whose body is a full sweep.
#[inline]
fn checkpoint() -> Result<(), LangError> {
    if lovelace_symbolics::cancel::check() {
        Err(cancelled_error())
    } else {
        Ok(())
    }
}

/// One index expression inside brackets.
#[derive(Clone, Debug)]
pub enum IndexSpec {
    /// A single index.
    Single(Value),
    /// A slice.
    Slice(Option<Value>, Option<Value>, Option<Value>),
    /// The whole axis.
    Full,
}

// ---------------------------------------------------------------------------
// Numeric coercion
// ---------------------------------------------------------------------------

/// Converts a numeric value to a real, if it is numeric.
pub fn as_real(v: &Value) -> Option<Real> {
    match v {
        Value::Natural(n) => Some(Real::from_int(&Int::from_decimal(&n.to_decimal())?)),
        Value::Integer(i) => Some(Real::from_int(i)),
        Value::Real(r) | Value::RealClamped(r, _, _) => Some(r.clone()),
        _ => None,
    }
}

fn as_int(v: &Value) -> Option<Int> {
    match v {
        Value::Natural(n) => Int::from_decimal(&n.to_decimal()),
        Value::Integer(i) => Some(i.clone()),
        _ => None,
    }
}

fn kind_rank(v: &Value) -> u8 {
    match v {
        Value::Natural(_) => 0,
        Value::Integer(_) => 1,
        Value::Real(_) => 2,
        _ => 9,
    }
}

fn numeric_pair(a: &Value, b: &Value) -> Option<(Value, Value)> {
    let ra = kind_rank(a);
    let rb = kind_rank(b);
    if ra == 9 || rb == 9 {
        return None;
    }
    let target = ra.max(rb);
    Some((widen(a, target)?, widen(b, target)?))
}

fn widen(v: &Value, target: u8) -> Option<Value> {
    match (v, target) {
        (Value::Natural(n), 0) => Some(Value::Natural(n.clone())),
        (Value::Natural(n), 1) => Some(Value::Integer(Int::from_decimal(&n.to_decimal())?)),
        (Value::Natural(n), 2) => as_real(&Value::Natural(n.clone())).map(Value::Real),
        (Value::Integer(i), 1) => Some(Value::Integer(i.clone())),
        (Value::Integer(i), 2) => Some(Value::Real(Real::from_int(i))),
        (Value::Real(r), 2) => Some(Value::Real(r.clone())),
        (Value::RealClamped(r, _, _), 2) => Some(Value::Real(r.clone())),
        _ => None,
    }
}

fn shape_of(v: &Value) -> Option<Vec<usize>> {
    match v {
        Value::Vector(items) => Some(vec![items.len()]),
        Value::Array(a) => Some(a.shape().to_vec()),
        _ => None,
    }
}

fn shape_text(shape: &[usize]) -> String {
    let parts: Vec<String> = shape.iter().map(|d| d.to_string()).collect();
    format!("[{}]", parts.join(", "))
}

// ---------------------------------------------------------------------------
// Operators
// ---------------------------------------------------------------------------

/// Converts a value to a symbolic expression when it can be represented exactly.
pub fn as_expr(v: &Value) -> Option<Sym> {
    match v {
        Value::Symbolic(e) => Some(e.clone()),
        Value::Natural(n) => {
            let i = Int::from_decimal(&n.to_decimal())?;
            Some(Sym::rational(&i, &Nat::from_u64(1)))
        }
        Value::Integer(i) => Some(Sym::rational(i, &Nat::from_u64(1))),
        Value::Real(r) => {
            // An exact Real is a rational at heart, including a periodic one (1/3 is exact and
            // publishes as 1/3, not as the decimal 0.(3)). An INEXACT Real keeps its decimal
            // identity: the value crossed into the expression from the numeric tier, so it is a
            // RealConstant leaf (outside the exact basis by its kind), never a rational silently
            // promoted to exact.
            if r.is_exact() {
                let (num, den) = r.to_rational();
                return Some(Sym::rational(&num, &den));
            }
            // The reference carries a real VALUE into an expression as a rational while its
            // decimal is short enough to read back exactly, and as a RealConstant leaf beyond
            // that. The boundary is measured, not assumed: subs(x + 1, x, pi(18)) answers the
            // rational 2070796326794896619/500000000000000000 while subs(x + 1, x, pi(19))
            // answers 1 + 3.1415926535897932384 (a Real leaf).
            let text = r.to_string();
            if significant_digits(&text) <= 19 {
                if let Some(value) = lovelace_symbolics::assumptions::decimal_to_rational(&text) {
                    return Some(Sym::Num(value));
                }
            }
            Some(Sym::real(&text))
        }
        _ => None,
    }
}

/// The count of significant decimal digits in a rendering (leading and trailing zeros are not
/// significant), used for the real-value-to-expression boundary.
fn significant_digits(text: &str) -> usize {
    let body = text.trim_start_matches('-');
    let digits: String = body.chars().filter(|c| c.is_ascii_digit()).collect();
    let trimmed = digits.trim_start_matches('0').trim_end_matches('0');
    if trimmed.is_empty() {
        digits.trim_start_matches('0').len()
    } else {
        trimmed.len()
    }
}

fn is_symbolic_zero(e: &Sym) -> bool {
    matches!(e, Sym::Num(r) if r.is_zero())
}

fn symbolic_pow(a: &Value, b: &Value) -> Option<Value> {
    let ea = as_expr(a)?;
    let eb = as_expr(b)?;
    Some(Value::Symbolic(Sym::pow(ea, eb)))
}

/// Adds two values.
pub fn add(a: &Value, b: &Value) -> Result<Value, LangError> {
    if matches!(a, Value::Symbolic(_)) || matches!(b, Value::Symbolic(_)) {
        if let (Some(x), Some(y)) = (as_expr(a), as_expr(b)) {
            return Ok(Value::Symbolic(Sym::add(x, y)));
        }
    }
    elementwise(a, b, "add", |x, y| {
        let (x, y) = numeric_pair(x, y).ok_or_else(|| type_error("+", x, y))?;
        match (&x, &y) {
            (Value::Natural(p), Value::Natural(q)) => Ok(Value::Natural(p.add(q))),
            (Value::Integer(p), Value::Integer(q)) => Ok(Value::Integer(p.add(q))),
            (Value::Real(p), Value::Real(q)) => Ok(Value::Real(p.add(q))),
            _ => Err(type_error("+", &x, &y)),
        }
    })
}

/// Subtracts two values, widening a natural difference to an integer when it goes negative.
pub fn sub(a: &Value, b: &Value) -> Result<Value, LangError> {
    if matches!(a, Value::Symbolic(_)) || matches!(b, Value::Symbolic(_)) {
        if let (Some(x), Some(y)) = (as_expr(a), as_expr(b)) {
            return Ok(Value::Symbolic(Sym::sub(x, y)));
        }
    }
    elementwise(a, b, "sub", |x, y| {
        let (x, y) = numeric_pair(x, y).ok_or_else(|| type_error("-", x, y))?;
        match (&x, &y) {
            (Value::Natural(p), Value::Natural(q)) => match p.sub(q) {
                Some(d) => Ok(Value::Natural(d)),
                None => {
                    let pi = Int::from_decimal(&p.to_decimal())
                        .ok_or_else(|| LangError::plain("Overflow."))?;
                    let qi = Int::from_decimal(&q.to_decimal())
                        .ok_or_else(|| LangError::plain("Overflow."))?;
                    Ok(Value::Integer(pi.sub(&qi)))
                }
            },
            (Value::Integer(p), Value::Integer(q)) => Ok(Value::Integer(p.sub(q))),
            (Value::Real(p), Value::Real(q)) => Ok(Value::Real(p.sub(q))),
            _ => Err(type_error("-", &x, &y)),
        }
    })
}

/// Multiplies two values.
pub fn mul(a: &Value, b: &Value) -> Result<Value, LangError> {
    if matches!(a, Value::Symbolic(_)) || matches!(b, Value::Symbolic(_)) {
        if let (Some(x), Some(y)) = (as_expr(a), as_expr(b)) {
            return Ok(Value::Symbolic(Sym::mul(x, y)));
        }
    }
    elementwise(a, b, "mul", |x, y| {
        let (x, y) = numeric_pair(x, y).ok_or_else(|| type_error("*", x, y))?;
        match (&x, &y) {
            (Value::Natural(p), Value::Natural(q)) => Ok(Value::Natural(p.mul(q))),
            (Value::Integer(p), Value::Integer(q)) => Ok(Value::Integer(p.mul(q))),
            (Value::Real(p), Value::Real(q)) => Ok(Value::Real(p.mul(q))),
            _ => Err(type_error("*", &x, &y)),
        }
    })
}

/// The published refusal for a zero divisor: the reference's DivisionByZero / DomainError.
fn divide_by_zero() -> LangError {
    LangError::coded("Cannot divide by zero.", 0, "DivisionByZero", "DomainError")
}

/// Divides two values. Two integer scalars whose quotient is exact narrow back to their integer
/// kind (`100 / 5` is Natural 20, `-6 / 2` is Integer -3); an inexact quotient is Real. A
/// container result follows D2: when EVERY element is an exact integer the whole result narrows
/// (Natural for non-negative elements, Integer for negative ones), otherwise it stays Real.
pub fn div(a: &Value, b: &Value, precision: usize) -> Result<Value, LangError> {
    if matches!(a, Value::Symbolic(_)) || matches!(b, Value::Symbolic(_)) {
        if let (Some(x), Some(y)) = (as_expr(a), as_expr(b)) {
            return Ok(Value::Symbolic(Sym::div(x, y)));
        }
    }
    let container = shape_of(a).is_some() || shape_of(b).is_some();
    let result = elementwise(a, b, "div", |x, y| {
        let rx = as_real(x).ok_or_else(|| type_error("/", x, y))?;
        let ry = as_real(y).ok_or_else(|| type_error("/", x, y))?;
        // The reference's `Real.Divide` runs at the ambient computation cap, which
        // `setprecision` moves together with the display cap (`WithPrecision`). A 64-place floor
        // here made `setprecision(18); 1/19` return the exact period while the reference (and the
        // class-Real side of LRealPeriodicFastPathTests) truncates it at 18 places.
        let places = precision.max(1);
        match rx.div(&ry, places) {
            Some(v) => {
                // The reference's digit loop stops at the computation cap, but its freed
                // leading-zero walk can still discover a period whose block ends past the cap.
                // The round-13 language contract publishes that quotient as the truncation it
                // is (1/(3*10^n) at n == places): keep its value and period metadata, and say
                // the expansion did not fit. A period that ends before the cap stays exact.
                let beyond =
                    v.period_length() > 0 && v.period_start() + v.period_length() > places as i64;
                Ok(Value::Real(if beyond { v.mark_inexact() } else { v }))
            }
            None => Err(divide_by_zero()),
        }
    })?;
    if container {
        return narrow_division_result(result);
    }
    // Scalar integer division: the quotient narrows only when the division is EXACT.
    if let (Some(left), Some(right)) = (as_int(a), as_int(b)) {
        if right.is_zero() {
            return Err(divide_by_zero());
        }
        let (quotient, remainder) = left.div_rem_signed(&right).expect("nonzero divisor");
        if remainder.is_zero() {
            if matches!(a, Value::Natural(_)) && matches!(b, Value::Natural(_)) {
                return Ok(Value::Natural(quotient.magnitude().clone()));
            }
            return Ok(Value::Integer(quotient));
        }
    }
    Ok(result)
}

/// D2: an elementwise quotient whose every element is an exact integer is published as one
/// homogeneous integer vector; otherwise every element is Real (a mixed vector would make the
/// element kind depend on the data).
fn narrow_division_result(value: Value) -> Result<Value, LangError> {
    fn narrow(items: Vec<Value>) -> Vec<Value> {
        let all_integer = items
            .iter()
            .all(|v| matches!(v, Value::Real(r) if r.is_exact() && r.is_integer()));
        items
            .into_iter()
            .map(|v| {
                if all_integer {
                    match &v {
                        Value::Real(r) => from_real(r),
                        _ => v,
                    }
                } else {
                    let converted = match &v {
                        Value::Integer(i) => Some(Value::Real(Real::from_int(i))),
                        Value::Natural(n) => as_real(&Value::Natural(n.clone())).map(Value::Real),
                        _ => None,
                    };
                    converted.unwrap_or(v)
                }
            })
            .collect()
    }
    match value {
        Value::Vector(items) => Ok(Value::Vector(narrow(items))),
        Value::Array(a) => {
            let shape = a.shape().to_vec();
            let items = narrow(a.iter().cloned().collect());
            Ok(Value::Array(
                NdArray::from_vec(shape, items).map_err(array_error)?,
            ))
        }
        other => Ok(other),
    }
}

/// Remainder with the reference's per-kind rule: the INTEGER remainder takes the sign of the
/// quotient (-7 % -3 is 1, 17 % -5 is -2), while the REAL remainder is the truncated
/// left - trunc(left / right) * right (-5.5 % 2 is -1.5).
fn rem(a: &Value, b: &Value, precision: usize) -> Result<Value, LangError> {
    elementwise(a, b, "rem", |x, y| {
        let (x, y) = numeric_pair(x, y).ok_or_else(|| type_error("%", x, y))?;
        match (&x, &y) {
            (Value::Natural(p), Value::Natural(q)) => p
                .div_rem(q)
                .map(|(_, r)| Value::Natural(r))
                .ok_or_else(divide_by_zero),
            (Value::Integer(p), Value::Integer(q)) => integer_remainder(p, q)
                .map(Value::Integer)
                .ok_or_else(divide_by_zero),
            (Value::Real(p), Value::Real(q)) => real_remainder(p, q, precision).map(Value::Real),
            _ => Err(type_error("%", &x, &y)),
        }
    })
}

/// The integer remainder whose SIGN follows the quotient (not the dividend): the magnitude is
/// |a| mod |b|, and a negative quotient negates it.
fn integer_remainder(a: &Int, b: &Int) -> Option<Int> {
    if b.is_zero() {
        return None;
    }
    let (quotient, _) = a.div_rem_signed(b)?;
    let (_, magnitude) = a.magnitude().div_rem(b.magnitude())?;
    let text = if quotient.is_negative() && !magnitude.is_zero() {
        format!("-{}", magnitude.to_decimal())
    } else {
        magnitude.to_decimal()
    };
    Int::from_decimal(&text)
}

/// Truncated real remainder: left - trunc(left / right) * right.
fn real_remainder(left: &Real, right: &Real, precision: usize) -> Result<Real, LangError> {
    let quotient = left
        .div(right, precision.max(64))
        .ok_or_else(divide_by_zero)?;
    let (numerator, denominator) = quotient.to_rational();
    let (whole, _) = numerator.div_rem(&denominator).ok_or_else(divide_by_zero)?;
    let truncated = Real::from_int(&whole);
    Ok(left.sub(&truncated.mul(right)))
}

/// Negates a value.
pub fn neg(v: &Value) -> Result<Value, LangError> {
    match v {
        Value::Natural(n) => {
            let i =
                Int::from_decimal(&n.to_decimal()).ok_or_else(|| LangError::plain("Overflow."))?;
            Ok(Value::Integer(i.negate()))
        }
        Value::Integer(i) => Ok(Value::Integer(i.negate())),
        Value::Real(r) => Ok(Value::Real(r.neg())),
        Value::Symbolic(e) => Ok(Value::Symbolic(Sym::neg(e.clone()))),
        Value::Vector(items) => {
            let mut out = Vec::with_capacity(items.len());
            for it in items {
                out.push(neg(it)?);
            }
            Ok(Value::Vector(out))
        }
        Value::Array(a) => {
            let mut out = Vec::with_capacity(a.numel());
            for it in a.iter() {
                out.push(neg(it)?);
            }
            Ok(Value::Array(
                NdArray::from_vec(a.shape().to_vec(), out).map_err(array_error)?,
            ))
        }
        other => Err(LangError::plain(format!(
            "Cannot negate a value of kind '{}'.",
            other.kind_name()
        ))),
    }
}

/// Applies a binary operator.
pub fn binary(op: BinOp, l: &Value, r: &Value, precision: usize) -> Result<Value, LangError> {
    match op {
        BinOp::Add => add(l, r),
        BinOp::Sub => sub(l, r),
        BinOp::Mul => mul(l, r),
        BinOp::Div => div(l, r, precision),
        BinOp::Rem => rem(l, r, precision),
        BinOp::Pow => pow(l, r),
        BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
            if matches!(l, Value::Symbolic(_)) || matches!(r, Value::Symbolic(_)) {
                if let (Some(x), Some(y)) = (as_expr(l), as_expr(r)) {
                    use lovelace_symbolics::RelOp;
                    let op = match op {
                        BinOp::Eq => Some(RelOp::Eq),
                        BinOp::Ne => Some(RelOp::Ne),
                        BinOp::Lt => Some(RelOp::Lt),
                        BinOp::Le => Some(RelOp::Le),
                        BinOp::Gt => Some(RelOp::Gt),
                        BinOp::Ge => Some(RelOp::Ge),
                        _ => None,
                    };
                    if let Some(rel) = op {
                        return Ok(Value::Symbolic(Sym::relation(rel, x, y)));
                    }
                }
            }
            let ord = compare(l, r)?;
            let result = match op {
                BinOp::Eq => ord == std::cmp::Ordering::Equal,
                BinOp::Ne => ord != std::cmp::Ordering::Equal,
                BinOp::Lt => ord == std::cmp::Ordering::Less,
                BinOp::Le => ord != std::cmp::Ordering::Greater,
                BinOp::Gt => ord == std::cmp::Ordering::Greater,
                BinOp::Ge => ord != std::cmp::Ordering::Less,
                _ => unreachable!(),
            };
            Ok(Value::Boolean(result))
        }
    }
}

fn compare(a: &Value, b: &Value) -> Result<std::cmp::Ordering, LangError> {
    if let (Some(x), Some(y)) = (as_real(a), as_real(b)) {
        return Ok(x.cmp(&y));
    }
    if a == b {
        return Ok(std::cmp::Ordering::Equal);
    }
    Err(LangError::plain("Comparison needs two numeric values."))
}

fn pow(base: &Value, exp: &Value) -> Result<Value, LangError> {
    if matches!(base, Value::Symbolic(_)) || matches!(exp, Value::Symbolic(_)) {
        if let Some(v) = symbolic_pow(base, exp) {
            return Ok(v);
        }
    }
    // The reference's ONE exact complex fallback in the numeric power path (NumericOps.PowerReal):
    // an exact negative base under the PRINCIPAL square root has the closed form
    // (-a)^(1/2) = i*sqrt(a), so it is routed to the kernel's Power constructor. Every other
    // non-integer exponent keeps the typed refusal below.
    if let (Some(b), Some(e)) = (as_expr(base), as_expr(exp)) {
        if matches!(&b, Sym::Num(n) if n.is_negative())
            && matches!(&e, Sym::Num(n)
                if n.numerator().to_decimal() == "1" && n.denominator().to_decimal() == "2")
        {
            return Ok(Value::Symbolic(Sym::pow(b, e)));
        }
    }
    let e = as_int(exp).ok_or_else(|| {
        // A non-integer exponent is not a domain failure: the runtime does not offer the
        // operation, and the caller can rewrite it with an exact exponent.
        LangError::coded(
            "Non-integer exponents are not yet supported.",
            0,
            "UnsupportedOperation",
            "UnsupportedOperation",
        )
    })?;
    let e_nat = Int::from_decimal(&e.magnitude().to_decimal())
        .ok_or_else(|| LangError::plain("Exponent is too large."))?;
    let e_u64 = e_nat
        .magnitude()
        .to_u64()
        .ok_or_else(|| LangError::plain("Exponent is too large."))?;
    let negative = e.is_negative();
    elementwise_scalar(base, |v| {
        let value =
            match v {
                Value::Natural(n) => Value::Natural(n.pow(e_u64)),
                Value::Integer(i) => Value::Integer(i.pow(e_u64)),
                Value::Real(r) => {
                    // A u64 exponent wider than i64 used to wrap negative here; refuse the count
                    // exactly as the reference's Int64 conversion does, instead of silently
                    // inverting the power.
                    let exponent = i64::try_from(e_u64).map_err(|_| {
                        LangError::coded(
                            "Value was either too large or too small for an Int64.",
                            0,
                            "ArithmeticError",
                            "DomainError",
                        )
                    })?;
                    Value::Real(r.pow_int(exponent).ok_or_else(|| {
                        LangError::plain("Cannot raise this value to that power.")
                    })?)
                }
                other => {
                    return Err(LangError::plain(format!(
                        "Cannot raise a value of kind '{}' to a power.",
                        other.kind_name()
                    )));
                }
            };
        if negative {
            match as_real(&value) {
                Some(r) => match Real::from_i64(1).div(&r, 1000) {
                    Some(inv) => Ok(Value::Real(inv)),
                    None => Err(divide_by_zero()),
                },
                None => Err(LangError::plain("Cannot raise this value to that power.")),
            }
        } else {
            Ok(value)
        }
    })
}

// ---------------------------------------------------------------------------
// Element-wise machinery
// ---------------------------------------------------------------------------

fn type_error(op: &str, a: &Value, b: &Value) -> LangError {
    if op == "+" && (a.is_numeric() || b.is_numeric()) && (!a.is_numeric() || !b.is_numeric()) {
        let numeric = if a.is_numeric() { a } else { b };
        let other = if a.is_numeric() { b } else { a };
        return LangError::plain(format!(
            "Cannot widen from {} to {}: only numeric kinds (Natural, Integer, Real) support widening.",
            numeric.kind_name(),
            other.kind_name()
        ));
    }
    LangError::plain(format!(
        "Cannot apply '{}' to '{}' and '{}'.",
        op,
        a.kind_name(),
        b.kind_name()
    ))
}

fn broadcast_error(a: &Value, b: &Value) -> LangError {
    let sa = shape_of(a)
        .map(|s| shape_text(&s))
        .unwrap_or_else(|| "[]".to_string());
    let sb = shape_of(b)
        .map(|s| shape_text(&s))
        .unwrap_or_else(|| "[]".to_string());
    LangError::plain(format!(
        "Operands could not be broadcast together with shapes {} and {}.",
        sa, sb
    ))
}

fn elementwise<F>(a: &Value, b: &Value, _name: &str, f: F) -> Result<Value, LangError>
where
    F: Fn(&Value, &Value) -> Result<Value, LangError> + Copy,
{
    match (a, b) {
        (Value::Vector(x), Value::Vector(y)) => {
            if x.len() != y.len() {
                return Err(broadcast_error(a, b));
            }
            let mut out = Vec::with_capacity(x.len());
            for (i, (p, q)) in x.iter().zip(y.iter()).enumerate() {
                poll(i as u64)?;
                out.push(f(p, q)?);
            }
            Ok(Value::Vector(out))
        }
        (Value::Array(x), Value::Array(y)) => {
            if x.shape() != y.shape() {
                return Err(broadcast_error(a, b));
            }
            let mut out = Vec::with_capacity(x.numel());
            for (i, (p, q)) in x.iter().zip(y.iter()).enumerate() {
                poll(i as u64)?;
                out.push(f(p, q)?);
            }
            Ok(Value::Array(
                NdArray::from_vec(x.shape().to_vec(), out).map_err(array_error)?,
            ))
        }
        (Value::Vector(v), Value::Array(m)) => broadcast_vector_over_rows(v, m, f, true),
        (Value::Array(m), Value::Vector(v)) => broadcast_vector_over_rows(v, m, f, false),
        (scalar, Value::Vector(v)) if !shape_of(scalar).is_some() => {
            let mut out = Vec::with_capacity(v.len());
            for (i, item) in v.iter().enumerate() {
                poll(i as u64)?;
                out.push(f(scalar, item)?);
            }
            Ok(Value::Vector(out))
        }
        (Value::Vector(v), scalar) if shape_of(scalar).is_none() => {
            let mut out = Vec::with_capacity(v.len());
            for (i, item) in v.iter().enumerate() {
                poll(i as u64)?;
                out.push(f(item, scalar)?);
            }
            Ok(Value::Vector(out))
        }
        (scalar, Value::Array(m)) if shape_of(scalar).is_none() => {
            let mut out = Vec::with_capacity(m.numel());
            for (i, item) in m.iter().enumerate() {
                poll(i as u64)?;
                out.push(f(scalar, item)?);
            }
            Ok(Value::Array(
                NdArray::from_vec(m.shape().to_vec(), out).map_err(array_error)?,
            ))
        }
        (Value::Array(m), scalar) if shape_of(scalar).is_none() => {
            let mut out = Vec::with_capacity(m.numel());
            for (i, item) in m.iter().enumerate() {
                poll(i as u64)?;
                out.push(f(item, scalar)?);
            }
            Ok(Value::Array(
                NdArray::from_vec(m.shape().to_vec(), out).map_err(array_error)?,
            ))
        }
        _ => f(a, b),
    }
}

fn broadcast_vector_over_rows<F>(
    v: &[Value],
    m: &NdArray<Value>,
    f: F,
    vector_first: bool,
) -> Result<Value, LangError>
where
    F: Fn(&Value, &Value) -> Result<Value, LangError> + Copy,
{
    let shape = m.shape().to_vec();
    if shape.len() < 2 || shape[shape.len() - 1] != v.len() {
        let a = Value::Vector(v.to_vec());
        let b = Value::Array(m.clone());
        return Err(broadcast_error(&a, &b));
    }
    let rows = shape[0];
    let cols = shape[1];
    let mut out = Vec::with_capacity(m.numel());
    for r in 0..rows {
        // One row is a full sweep of the vector, so the outer loop checks on every pass.
        checkpoint()?;
        for c in 0..cols {
            let cell = m.get(&[r, c]).cloned().unwrap_or(Value::Void);
            out.push(if vector_first {
                f(&v[c], &cell)?
            } else {
                f(&cell, &v[c])?
            });
        }
    }
    Ok(Value::Array(
        NdArray::from_vec(shape, out).map_err(array_error)?,
    ))
}

fn elementwise_scalar<F>(v: &Value, f: F) -> Result<Value, LangError>
where
    F: Fn(&Value) -> Result<Value, LangError> + Copy,
{
    match v {
        Value::Vector(items) => {
            let mut out = Vec::with_capacity(items.len());
            for (index, i) in items.iter().enumerate() {
                poll(index as u64)?;
                out.push(f(i)?);
            }
            Ok(Value::Vector(out))
        }
        Value::Array(a) => {
            let mut out = Vec::with_capacity(a.numel());
            for (index, i) in a.iter().enumerate() {
                poll(index as u64)?;
                out.push(f(i)?);
            }
            Ok(Value::Array(
                NdArray::from_vec(a.shape().to_vec(), out).map_err(array_error)?,
            ))
        }
        other => f(other),
    }
}

fn array_error(e: ArrayError) -> LangError {
    LangError::plain(format!("{}", e))
}

// ---------------------------------------------------------------------------
// Literals and ranges
// ---------------------------------------------------------------------------

/// D3: widens every numeric element in one literal/array to the widest kind present.
fn promote_numeric_kinds(items: &mut [Value]) {
    let mut target = 0u8;
    for item in items.iter() {
        if item.is_numeric() {
            target = target.max(kind_rank(item));
        }
    }
    if target == 0 {
        return;
    }
    for item in items.iter_mut() {
        if item.is_numeric() {
            if let Some(widened) = widen(item, target) {
                *item = widened;
            }
        }
    }
}

/// Builds a list literal, promoting to an array when the elements are uniform lists.
///
/// D3: the numeric elements of ONE literal share the widest kind among them, so
/// `[1, 2.5, 3]` is three Reals and `[1, -2]` is two Integers, never a mixed vector.
pub fn make_list(mut items: Vec<Value>) -> Result<Value, LangError> {
    promote_numeric_kinds(&mut items);
    if items.iter().all(|v| matches!(v, Value::Vector(_))) {
        let mut rows: Vec<Vec<Value>> = Vec::with_capacity(items.len());
        let mut width: Option<usize> = None;
        for v in &items {
            if let Value::Vector(row) = v {
                if let Some(w) = width {
                    if w != row.len() {
                        return Err(LangError::plain(
                            "Ragged nested list literal: every row must have the same shape.",
                        ));
                    }
                } else {
                    width = Some(row.len());
                }
                rows.push(row.clone());
            }
        }
        if rows.len() > 1 || width.unwrap_or(0) > 0 {
            let shape = vec![rows.len(), width.unwrap_or(0)];
            let mut flat: Vec<Value> = rows.into_iter().flatten().collect();
            promote_numeric_kinds(&mut flat);
            return Ok(Value::Array(
                NdArray::from_vec(shape, flat).map_err(array_error)?,
            ));
        }
    }
    if !items.is_empty() && items.iter().all(|v| matches!(v, Value::Array(_))) {
        let mut shape: Option<Vec<usize>> = None;
        let mut total = 0usize;
        for v in &items {
            if let Value::Array(a) = v {
                if let Some(s) = &shape {
                    if s != a.shape() {
                        return Err(LangError::plain(
                            "Ragged nested list literal: every row must have the same shape.",
                        ));
                    }
                } else {
                    shape = Some(a.shape().to_vec());
                }
                total += a.numel();
            }
        }
        if let Some(mut s) = shape {
            let mut new_shape = vec![items.len()];
            new_shape.append(&mut s);
            let mut flat = Vec::with_capacity(total);
            for v in &items {
                if let Value::Array(a) = v {
                    flat.extend(a.iter().cloned());
                }
            }
            promote_numeric_kinds(&mut flat);
            return Ok(Value::Array(
                NdArray::from_vec(new_shape, flat).map_err(array_error)?,
            ));
        }
    }
    Ok(Value::Vector(items))
}

/// The reference's refusal for a range bound or step that is an exact integer too wide for a
/// signed 64-bit count. The wording is Int64Range.Describe plus the parameter the call site
/// names - a bound has no numeric position, so the message uses the quoted-name form.
fn range_component_refusal(parameter: &str, got: &str) -> LangError {
    let bound = if parameter == "step" {
        "that is a positive count (1 or more, at most 9223372036854775807)"
    } else {
        "that fits a signed 64-bit integer (at most 9223372036854775807 in absolute value)"
    };
    LangError::coded(
        format!("range(): argument '{parameter}' must be a Natural or Integer {bound}; got {got}."),
        0,
        "InvalidArgument",
        "TypeMismatch",
    )
}

/// The decimal text of an exact integer value; None for every other kind.
fn integer_text(value: &Value) -> Option<String> {
    match value {
        Value::Natural(n) => Some(n.to_decimal()),
        Value::Integer(i) => Some(i.to_decimal()),
        _ => None,
    }
}

/// Builds a range value.
pub fn make_range(start: &Value, step: Option<&Value>, end: &Value) -> Result<Value, LangError> {
    // A bound or step wider than a signed 64-bit count is refused BY NAME before the progression
    // starts. Without this guard the bound crosses as a Real, the loop below runs to its
    // one-million-element cap and then reports Range is too large: the reference refuses at once.
    for value in [start, end] {
        if let Some(text) = integer_text(value) {
            if text.parse::<i64>().is_err() {
                return Err(range_component_refusal("bound", &text));
            }
        }
    }
    if let Some(value) = step {
        if let Some(text) = integer_text(value) {
            if text.parse::<i64>().is_err() {
                return Err(range_component_refusal("step", &text));
            }
        }
    }
    // A bound is an integer-semantic argument: an exactly integral Real is refused by KIND (write
    // it as an integer) and a fractional Real by value, never floored into the nearest count.
    let start = exact_integer_argument("range", "bound", 0, start, CountBounds::Unbounded)?;
    let end = exact_integer_argument("range", "bound", 0, end, CountBounds::Unbounded)?;
    let s = Real::from_i64(start);
    let e = Real::from_i64(end);
    let explicit_step = step.is_some();
    let step_value = match step {
        Some(v) => {
            let sv = as_real(v).ok_or_else(|| LangError::plain("Range step must be numeric."))?;
            if sv.is_zero() {
                return Err(LangError::plain("Range step must not be zero."));
            }
            sv
        }
        // The default step is ALWAYS +1: a descending bound pair is an empty range, never a
        // silently reversed one (`1..0` has length 0).
        None => Real::from_i64(1),
    };
    let mut out = Vec::new();
    let mut cur = s.clone();
    let up = step_value.cmp(&Real::zero()) == std::cmp::Ordering::Greater;
    let mut guard = 0usize;
    while if up {
        cur.cmp(&e) != std::cmp::Ordering::Greater
    } else {
        cur.cmp(&e) != std::cmp::Ordering::Less
    } {
        // The range build is tens of millions of boxed values on the documented workloads, so it
        // polls like every other kernel instead of waiting for the statement boundary.
        poll(guard as u64)?;
        // A stepped range is an INTEGER progression: every element carries the Integer kind even
        // when the value itself is a positive whole number.
        out.push(if explicit_step {
            integer_value(&cur)
        } else {
            from_real(&cur)
        });
        cur = cur.add(&step_value);
        guard += 1;
        if guard > 1_000_000 {
            return Err(LangError::plain("Range is too large."));
        }
    }
    Ok(Value::Vector(out))
}

fn integer_value(r: &Real) -> Value {
    match r.to_int_exact() {
        Some(i) => Value::Integer(i),
        None => Value::Real(r.clone()),
    }
}

fn from_real(r: &Real) -> Value {
    if r.is_integer() {
        if let Some(i) = r.to_int_exact() {
            if !i.is_negative() {
                if let Some(n) = Nat::from_decimal(&i.to_decimal()) {
                    return Value::Natural(n);
                }
            }
            return Value::Integer(i);
        }
    }
    Value::Real(r.clone())
}

// ---------------------------------------------------------------------------
// Indexing
// ---------------------------------------------------------------------------

/// Reads one index through the reference's exact-integer contract: a Natural or Integer that fits
/// a signed 64-bit integer, or the typed argument refusal that names the value that arrived — a
/// Real is refused by KIND (never floored), an integer too wide for Int64 is refused by NAME, and
/// neither can cross as a raw conversion failure.
pub(crate) fn index_argument(v: &Value) -> Result<i64, LangError> {
    exact_integer_argument("index", "index", 0, v, CountBounds::Unbounded)
}

fn index_int(v: &Value, len: usize) -> Result<usize, LangError> {
    let i = index_argument(v)?;
    if i < 0 || i as usize >= len {
        return Err(LangError::plain(format!(
            "Index {i} is out of range for vector of length {len}."
        )));
    }
    Ok(i as usize)
}

/// One index naming one axis of a rank >= 2 array: the refusal names the DIMENSION and its size,
/// as the reference does, so a caller can tell which subscript was wrong.
fn index_int_for_dimension(v: &Value, axis: usize, dimension: usize) -> Result<usize, LangError> {
    let i = index_argument(v)?;
    if i < 0 || i as usize >= dimension {
        return Err(LangError::plain(format!(
            "Index {i} is out of range for dimension {axis} of size {dimension}."
        )));
    }
    Ok(i as usize)
}

/// A slice bound or step through the reference's range-bound contract: an exact Natural or
/// Integer that fits a signed 64-bit count. A wider value is refused by name before any index
/// arithmetic, so it can neither wrap nor walk the signed range.
fn slice_integer(value: &Value, parameter: &str) -> Result<i64, LangError> {
    let integer =
        as_int(value).ok_or_else(|| LangError::plain("Slice bound must be an Integer."))?;
    let text = integer.to_decimal();
    text.parse::<i64>()
        .map_err(|_| range_component_refusal(parameter, &text))
}

fn slice_indices(spec: &IndexSpec, len: usize) -> Result<Vec<usize>, LangError> {
    match spec {
        IndexSpec::Full => Ok((0..len).collect()),
        IndexSpec::Single(v) => Ok(vec![index_int(v, len)?]),
        IndexSpec::Slice(a, b, step) => {
            let step_v = match step {
                Some(v) => slice_integer(v, "step")?,
                None => 1,
            };
            if step_v <= 0 {
                // The reference routes a slice step through range(): a step must be a POSITIVE
                // count, so 0 and negative steps share its argument refusal (and reversing is
                // not spelled with a slice at all).
                return Err(LangError::coded(
                    format!(
                        "range(): argument 'step' must be a Natural or Integer that is a positive count (1 or more, at most 9223372036854775807); got {}.",
                        step_v
                    ),
                    0,
                    "InvalidArgument",
                    "TypeMismatch",
                ));
            }
            // Bounds are clipped to [0, len] BEFORE any arithmetic, exactly like the reference's
            // slice path: an i64::MIN bound would otherwise walk (or, on the old negate-the-
            // magnitude conversion, panic on) the whole signed range.
            let len_i = len as i64;
            let mut start = match a {
                Some(v) => slice_integer(v, "bound")?,
                None => 0,
            };
            let stop = match b {
                Some(v) => slice_integer(v, "bound")?,
                None => len_i,
            };
            if start < 0 {
                start = start.saturating_add(len_i);
            }
            start = start.clamp(0, len_i);
            let mut stop_adj = stop;
            if stop_adj < 0 && b.is_some() {
                stop_adj = stop_adj.saturating_add(len_i);
            }
            stop_adj = stop_adj.clamp(0, len_i);
            let mut out = Vec::new();
            let mut i = start;
            while i < stop_adj {
                out.push(i as usize);
                // A step that would overflow i64 ends the slice at the last index emitted; it
                // never wraps and never panics.
                match i.checked_add(step_v) {
                    Some(next) => i = next,
                    None => break,
                }
            }
            Ok(out)
        }
    }
}

/// Applies indexing to a value.
pub fn index(target: &Value, specs: &[IndexSpec]) -> Result<Value, LangError> {
    match target {
        Value::Vector(items) => {
            let spec = specs
                .first()
                .ok_or_else(|| LangError::plain("Index requires at least one argument."))?;
            match spec {
                IndexSpec::Single(v) => {
                    let idx = index_int(v, items.len())?;
                    Ok(items[idx].clone())
                }
                other => {
                    let idx = slice_indices(other, items.len())?;
                    Ok(Value::Vector(
                        idx.into_iter().map(|i| items[i].clone()).collect(),
                    ))
                }
            }
        }
        Value::Array(a) => {
            let shape = a.shape().to_vec();
            if specs.len() > shape.len() {
                return Err(LangError::plain("Too many indices for this array."));
            }
            let mut per_axis: Vec<Vec<usize>> = Vec::new();
            for (axis, spec) in specs.iter().enumerate() {
                per_axis.push(match spec {
                    IndexSpec::Single(value) => {
                        vec![index_int_for_dimension(value, axis, shape[axis])?]
                    }
                    other => slice_indices(other, shape[axis])?,
                });
            }
            let all_single = specs.iter().all(|s| matches!(s, IndexSpec::Single(_)));
            // A single index removes its axis; a slice or full axis keeps it.
            let mut new_shape: Vec<usize> = specs
                .iter()
                .zip(per_axis.iter())
                .filter(|(spec, _)| !matches!(spec, IndexSpec::Single(_)))
                .map(|(_, v)| v.len())
                .collect();
            new_shape.extend_from_slice(&shape[specs.len()..]);
            let total: usize = new_shape.iter().product();
            let mut flat = Vec::with_capacity(total);
            let mut idx = vec![0usize; new_shape.len()];
            for _ in 0..total {
                let mut full: Vec<usize> = Vec::with_capacity(shape.len());
                let mut coord_axis = 0usize;
                for (axis, spec) in specs.iter().enumerate() {
                    if matches!(spec, IndexSpec::Single(_)) {
                        // Fixed coordinate taken from the single index.
                        if let IndexSpec::Single(val) = spec {
                            if let Some(n) = as_int(val).and_then(|i| i.magnitude().to_u64()) {
                                full.push(n as usize);
                            }
                        }
                    } else {
                        full.push(per_axis[axis][idx[coord_axis]]);
                        coord_axis += 1;
                    }
                }
                for extra in specs.len()..shape.len() {
                    full.push(idx[coord_axis]);
                    let _ = extra;
                    coord_axis += 1;
                }
                if let Some(v) = a.get(&full) {
                    flat.push(v.clone());
                }
                let mut carry = true;
                for d in (0..new_shape.len()).rev() {
                    if !carry {
                        break;
                    }
                    idx[d] += 1;
                    if idx[d] == new_shape[d] {
                        idx[d] = 0;
                    } else {
                        carry = false;
                    }
                }
            }
            if all_single && specs.len() == shape.len() {
                return Ok(flat.into_iter().next().unwrap_or(Value::Void));
            }
            if new_shape.len() == 1 {
                return Ok(Value::Vector(flat));
            }
            if new_shape.is_empty() {
                return Ok(flat.into_iter().next().unwrap_or(Value::Void));
            }
            Ok(Value::Array(
                NdArray::from_vec(new_shape, flat).map_err(array_error)?,
            ))
        }
        other => Err(LangError::plain(format!(
            "Cannot index a value of kind '{}'.",
            other.kind_name()
        ))),
    }
}

fn slice_indices_or_single(spec: &IndexSpec, len: usize) -> Result<Vec<usize>, LangError> {
    match spec {
        IndexSpec::Single(v) => Ok(vec![index_int(v, len)?]),
        other => slice_indices(other, len),
    }
}

// ---------------------------------------------------------------------------
// Builtins
// ---------------------------------------------------------------------------

/// Dispatches a builtin call.
pub fn call(
    engine: &mut Engine,
    name: &str,
    args: Vec<Value>,
    pos: usize,
) -> Result<Value, LangError> {
    // The call boundary is the coarse cancellation seam: a deadline that fired while the engine
    // was outside an inner loop stops the run before another builtin starts.
    checkpoint()?;
    let arity = |n: usize| -> Result<(), LangError> {
        if args.len() == n {
            Ok(())
        } else {
            // The call-site validator publishes the DECLARED signature the call was measured
            // against, so a repair is chosen from structure rather than from the sentence.
            let (message, signature) = arity_failure(name, args.len());
            let mut error = LangError::coded(message, pos, "InvalidArgument", "TypeMismatch");
            if let Some(signature) = signature {
                error = error.with_signature(signature);
            }
            Err(error)
        }
    };
    match name {
        // The four domain constants are zero-arity builtins in the reference: a bare name reads
        // the value, and a CALL of the same name is the declared-arity refusal (complex(1, 2)
        // => "complex(): expected 0 arguments; got 2."), never an unknown function.
        "real" | "integer" | "rational" | "complex" => {
            arity(0)?;
            Ok(Value::Domain(name.to_string()))
        }
        "abs" => {
            arity(1)?;
            match &args[0] {
                Value::Natural(n) => Ok(Value::Natural(n.clone())),
                Value::Integer(i) => Ok(Value::Integer(if i.is_negative() {
                    i.negate()
                } else {
                    i.clone()
                })),
                Value::Real(r) => Ok(Value::Real(r.abs())),
                Value::Symbolic(e) => {
                    // The reference refuses a canonical node deeper than the published symbolic
                    // budget while the node is being built.
                    let expr = Sym::Fun("abs".to_string(), vec![e.clone()]);
                    crate::ensure_symbolic_budget(&expr)?;
                    Ok(Value::Symbolic(expr))
                }
                Value::Complex(re, im) => Ok(Value::Real(complex_magnitude(
                    re,
                    im,
                    engine.precision_value(),
                ))),
                // Elementwise magnitude for a complex array (the magnitude-spectrum idiom). The
                // reference accepts only an array whose inferred element kind is Complex and
                // flattens it to a rank-1 vector of Real magnitudes; every other array kind is
                // refused with the inferred DType in the sentence.
                Value::Vector(items) => abs_complex_array(items.iter(), engine.precision_value()),
                Value::Array(array) => abs_complex_array(array.iter(), engine.precision_value()),
                other => Err(LangError::plain(format!(
                    "abs needs a number, but got '{}'.",
                    other.kind_name()
                ))),
            }
        }
        "sign" => {
            arity(1)?;
            match as_real(&args[0]) {
                Some(r) => {
                    let s = if r.is_zero() {
                        0
                    } else if r.cmp(&Real::zero()) == std::cmp::Ordering::Less {
                        -1
                    } else {
                        1
                    };
                    Ok(Value::Integer(Int::from_i64(s)))
                }
                None => Err(LangError::plain("sign needs a number.")),
            }
        }
        "inv" => {
            arity(1)?;
            if let Value::Array(a) = &args[0] {
                return inv_matrix(a);
            }
            let r = as_real(&args[0])
                .ok_or_else(|| LangError::plain("inv needs a number or a matrix."))?;
            match Real::from_i64(1).div(&r, engine.precision_value().max(64)) {
                Some(v) => Ok(Value::Real(v)),
                None => Err(divide_by_zero()),
            }
        }
        "divrem" => {
            arity(2)?;
            let real_kind = |v: &Value| matches!(v, Value::Real(_));
            if real_kind(&args[0]) || real_kind(&args[1]) {
                return Err(LangError::plain(
                    "divrem() is not supported for values of kind 'Real'. Use Natural or Integer operands.",
                ));
            }
            let a = as_int(&args[0]).ok_or_else(|| {
                LangError::plain(format!(
                    "divrem() is not supported for values of kind '{}'. Use Natural or Integer operands.",
                    args[0].kind_name()
                ))
            })?;
            let b = as_int(&args[1]).ok_or_else(|| {
                LangError::plain(format!(
                    "divrem() is not supported for values of kind '{}'. Use Natural or Integer operands.",
                    args[1].kind_name()
                ))
            })?;
            let (quotient, _) = a.div_rem_signed(&b).ok_or_else(divide_by_zero)?;
            let remainder = integer_remainder(&a, &b).ok_or_else(divide_by_zero)?;
            let both_natural =
                matches!(args[0], Value::Natural(_)) && matches!(args[1], Value::Natural(_));
            let q = if both_natural {
                Value::Natural(quotient.magnitude().clone())
            } else {
                Value::Integer(quotient)
            };
            let r = if both_natural {
                Value::Natural(remainder.magnitude().clone())
            } else {
                Value::Integer(remainder)
            };
            Ok(Value::Record(
                "DivRemResult".to_string(),
                vec![("quotient".to_string(), q), ("remainder".to_string(), r)],
            ))
        }
        "is_even" | "is_odd" => {
            arity(1)?;
            let i =
                as_int(&args[0]).ok_or_else(|| LangError::plain("Predicate needs an integer."))?;
            let (_, rem) = i.div_rem(&Nat::from_u64(2)).ok_or_else(divide_by_zero)?;
            let even = rem.is_zero();
            Ok(Value::Boolean(if name == "is_even" { even } else { !even }))
        }
        "sqrt" => {
            arity(1)?;
            if let Value::Symbolic(e) = &args[0] {
                return Ok(Value::Symbolic(Sym::sqrt(e.clone())));
            }
            let r = as_real(&args[0]).ok_or_else(|| LangError::plain("sqrt needs a number."))?;
            if r.cmp(&Real::zero()) == std::cmp::Ordering::Less {
                // The real square root is the REAL-DOMAIN contract and keeps refusing negative
                // values. The refused value has an exact closed form though, so route it to the
                // symbolic complex path: sqrt(-a) = i*sqrt(a) exactly, the principal branch.
                let (num, den) = r.to_rational();
                let base = Sym::rational(&num, &den);
                let half = Sym::rational(&Int::from_i64(1), &Nat::from_u64(2));
                return Ok(Value::Symbolic(Sym::pow(base, half)));
            }
            Ok(Value::Real(r.sqrt(engine.precision_value())))
        }
        "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "exp" | "log" | "sinh" | "cosh"
        | "tanh" | "asinh" | "acosh" | "atanh" => {
            arity(1)?;
            let e = as_expr(&args[0]).ok_or_else(|| {
                LangError::plain(format!("{} needs a number or an expression.", name))
            })?;
            // The inverse hyperbolics fold at their defining point, exactly as sin(0) does:
            // asinh(0), acosh(1) and atanh(0) are the Symbolic 0, not an unevaluated node.
            let folds = match name {
                "asinh" => is_symbolic_zero(&e),
                "acosh" => matches!(&e, Sym::Num(r) if r.is_one()),
                "atanh" => is_symbolic_zero(&e),
                _ => false,
            };
            if folds {
                return Ok(Value::Symbolic(Sym::num(0)));
            }
            Ok(Value::Symbolic(Sym::Fun(name.to_string(), vec![e])))
        }
        "symbol" => {
            if args.is_empty() {
                return Err(LangError::plain("symbol needs a name."));
            }
            // The name is a Text value; coercing a number to its display spelling invented a
            // symbol out of a caller's shape mistake (symbol(1) answered 1), so a non-Text
            // argument is the documented argument error.
            let name = match &args[0] {
                Value::Text(t) => t.clone(),
                other => {
                    return Err(shape_failure(
                        "symbol",
                        1,
                        "a text name such as \"x\"",
                        other,
                    ));
                }
            };
            // An optional domain is ASSUMED for the symbol by the session that created it (the
            // reference's SymbolWithDomain), which is what lets simplify(sqrt(x^2)) answer abs(x)
            // under symbol("x", real) and inspect report the DomainCondition.
            if let Some(Value::Domain(domain)) = args.get(1) {
                let narrowed = match domain.as_str() {
                    "real" => AssumptionDomain::Real,
                    "integer" => AssumptionDomain::Integer,
                    "rational" => AssumptionDomain::Rational,
                    _ => AssumptionDomain::Complex,
                };
                engine
                    .add_assumption(Assumption::SymbolDomain(name.clone(), narrowed))
                    .map_err(unsatisfiable_assumptions)?;
                engine.record_domain_assumption(&name, narrowed.name());
            }
            Ok(Value::Symbolic(Sym::sym(&name)))
        }
        "diff" => {
            arity(2)?;
            let f =
                as_expr(&args[0]).ok_or_else(|| LangError::plain("diff needs an expression."))?;
            let var = match &args[1] {
                Value::Symbolic(Sym::Sym(name)) => name.clone(),
                Value::Text(name) => name.clone(),
                other => {
                    return Err(LangError::plain(format!(
                        "diff(): argument 2 must be a symbolic variable; got {}.",
                        other.kind_name()
                    )));
                }
            };
            Ok(Value::Symbolic(lovelace_symbolics::diff(&f, &var)))
        }
        "jacobian" => {
            arity(2)?;
            const FUNCS: &str = "a list of expressions, e.g. [x*y, x + y]";
            const VARS: &str = "a list of parameter symbols, e.g. [x, y]";
            let funcs = argument_list(name, 1, FUNCS, &args[0])?;
            let vars = argument_list(name, 2, VARS, &args[1])?;
            let mut rows: Vec<Value> = Vec::new();
            for f in &funcs {
                let fe = as_expr(f).ok_or_else(|| shape_failure(name, 1, FUNCS, &args[0]))?;
                let mut row: Vec<Value> = Vec::new();
                for v in &vars {
                    let name = match v {
                        Value::Symbolic(Sym::Sym(n)) => n.clone(),
                        _ => return Err(shape_failure("jacobian", 2, VARS, &args[1])),
                    };
                    row.push(Value::Symbolic(lovelace_symbolics::diff(&fe, &name)));
                }
                rows.push(Value::Vector(row));
            }
            let cols = vars.len();
            let mut flat: Vec<Value> = Vec::new();
            for r in &rows {
                if let Value::Vector(cells) = r {
                    flat.extend(cells.clone());
                }
            }
            Ok(Value::Array(
                NdArray::from_vec(vec![funcs.len(), cols], flat).map_err(array_error)?,
            ))
        }
        "hessian" => {
            arity(2)?;
            let f = as_expr(&args[0])
                .ok_or_else(|| LangError::plain("hessian needs an expression."))?;
            let vars = flatten_values(&args[1])?;
            let names: Vec<String> = vars
                .iter()
                .map(|v| match v {
                    Value::Symbolic(Sym::Sym(n)) => Ok(n.clone()),
                    _ => Err(LangError::plain("hessian needs variables.")),
                })
                .collect::<Result<_, _>>()?;
            let n = names.len();
            let mut flat: Vec<Value> = Vec::with_capacity(n * n);
            for i in 0..n {
                for j in 0..n {
                    let first = lovelace_symbolics::diff(&f, &names[i]);
                    let second = lovelace_symbolics::diff(&first, &names[j]);
                    flat.push(Value::Symbolic(second));
                }
            }
            Ok(Value::Array(
                NdArray::from_vec(vec![n, n], flat).map_err(array_error)?,
            ))
        }
        "subs" => {
            arity(3)?;
            let f =
                as_expr(&args[0]).ok_or_else(|| LangError::plain("subs needs an expression."))?;
            let var = match &args[1] {
                Value::Symbolic(Sym::Sym(n)) => n.clone(),
                _ => return Err(LangError::plain("subs needs a variable.")),
            };
            let replacement =
                as_expr(&args[2]).ok_or_else(|| LangError::plain("subs needs a replacement."))?;
            Ok(Value::Symbolic(lovelace_symbolics::substitute(
                &f,
                &var,
                &replacement,
            )))
        }
        "integrate" => {
            arity(2)?;
            let f = as_expr(&args[0])
                .ok_or_else(|| LangError::plain("integrate needs an expression."))?;
            checkpoint()?;
            let var = match &args[1] {
                Value::Symbolic(Sym::Sym(name)) => name.clone(),
                _ => return Err(LangError::plain("integrate needs a variable.")),
            };
            if let Some(result) = integrate_basic(&f, &var) {
                return Ok(Value::Symbolic(result));
            }
            // Integrate a partial-fraction decomposition term by term: A/(x - r) -> A*log(x - r).
            if let Some(decomposition) = lovelace_symbolics::apart(&f, &var) {
                let terms = match &decomposition {
                    Sym::Add(ts) => ts.clone(),
                    single => vec![single.clone()],
                };
                let mut acc = Sym::num(0);
                for t in terms {
                    if let Some(integrated) = integrate_simple_fraction(&t, &var) {
                        acc = Sym::add(acc, integrated);
                    } else {
                        return Ok(Value::Symbolic(Sym::Fun(
                            "integrate".to_string(),
                            vec![f, Sym::sym(&var)],
                        )));
                    }
                }
                return Ok(Value::Symbolic(acc));
            }
            Ok(Value::Symbolic(Sym::Fun(
                "integrate".to_string(),
                vec![f, Sym::sym(&var)],
            )))
        }
        "and" | "or" | "not" => {
            if name == "not" {
                arity(1)?;
            }
            let mut args_out = Vec::new();
            for a in &args {
                args_out.push(
                    as_expr(a)
                        .ok_or_else(|| LangError::plain("Logical operators need expressions."))?,
                );
            }
            Ok(Value::Symbolic(Sym::Fun(name.to_string(), args_out)))
        }
        "integrate_full" => {
            arity(2)?;
            let f = as_expr(&args[0])
                .ok_or_else(|| LangError::plain("integrate needs an expression."))?;
            checkpoint()?;
            let var = match &args[1] {
                Value::Symbolic(Sym::Sym(n)) => n.clone(),
                _ => return Err(LangError::plain("integrate needs a variable.")),
            };
            let antiderivative = integrate_basic(&f, &var);
            let (status, verified, expression, method, verification, exactness) =
                match &antiderivative {
                    Some(a) => {
                        // Verification is the same rule the reference applies: differentiate the
                        // antiderivative and expand, then compare with the integrand.
                        let derivative =
                            lovelace_symbolics::expand(&lovelace_symbolics::diff(a, &var));
                        let verified = derivative == lovelace_symbolics::expand(&f);
                        (
                            "SolvedExact",
                            verified,
                            Value::Symbolic(a.clone()),
                            "table",
                            if verified {
                                "differentiation+expansion"
                            } else {
                                ""
                            },
                            "Exact",
                        )
                    }
                    None => (
                        "Unevaluated",
                        false,
                        Value::Symbolic(Sym::Fun(
                            "integrate".to_string(),
                            vec![f.clone(), Sym::sym(&var)],
                        )),
                        "",
                        "",
                        "None",
                    ),
                };
            Ok(Value::Record(
                "IntegrationResult".to_string(),
                vec![
                    (
                        "status".to_string(),
                        enum_value("IntegrationStatus", status),
                    ),
                    ("expression".to_string(), expression),
                    ("conditions".to_string(), Value::Vector(Vec::new())),
                    ("verified".to_string(), Value::Boolean(verified)),
                    ("method".to_string(), Value::Text(method.to_string())),
                    (
                        "verification_method".to_string(),
                        Value::Text(verification.to_string()),
                    ),
                    (
                        "exactness".to_string(),
                        enum_value("SolutionExactness", exactness),
                    ),
                    ("diagnostics".to_string(), Value::Vector(Vec::new())),
                ],
            ))
        }
        "compile" | "compile_full" => {
            if args.len() < 2 {
                return Err(LangError::plain(
                    "compile needs an expression and parameters.",
                ));
            }
            let f = as_expr(&args[0])
                .ok_or_else(|| LangError::plain("compile needs an expression."))?;
            let params = flatten_values(&args[1])?;
            let mut names: Vec<String> = Vec::new();
            let mut parameters: Vec<Value> = Vec::new();
            for p in &params {
                match p {
                    Value::Symbolic(Sym::Sym(n)) => {
                        names.push(n.clone());
                        parameters.push(Value::Record(
                            "ParameterInfo".to_string(),
                            vec![
                                ("name".to_string(), Value::Text(n.clone())),
                                ("domain".to_string(), Value::Domain("complex".to_string())),
                            ],
                        ));
                    }
                    _ => return Err(LangError::plain("compile needs parameter symbols.")),
                }
            }
            Ok(Value::Record(
                "CompilationResult".to_string(),
                vec![
                    ("ir".to_string(), Value::Text(ir_text(&f, &names))),
                    ("parameters".to_string(), Value::Vector(parameters)),
                    (
                        "result_domain".to_string(),
                        Value::Domain("complex".to_string()),
                    ),
                    (
                        "result_type".to_string(),
                        Value::Text(result_type(&f, &names)),
                    ),
                    (
                        "precision_policy".to_string(),
                        Value::Text("caller_supplied".to_string()),
                    ),
                    (
                        "optimization_policy".to_string(),
                        Value::Text("none".to_string()),
                    ),
                    ("target".to_string(), Value::Text("mathir".to_string())),
                    (
                        "mathir_version".to_string(),
                        Value::Integer(Int::from_i64(2)),
                    ),
                    ("exact".to_string(), Value::Boolean(f.is_exact())),
                    ("diagnostics".to_string(), Value::Vector(Vec::new())),
                ],
            ))
        }
        "evalir" | "evalir_batch" => {
            // The reference declares three parameters (ir, values, digits), so a call with any
            // other count is the declared-arity refusal, never a silent default precision.
            if args.len() != 3 {
                let (message, signature) = arity_failure(name, args.len());
                let mut error = LangError::coded(message, pos, "InvalidArgument", "TypeMismatch");
                if let Some(signature) = signature {
                    error = error.with_signature(signature);
                }
                return Err(error);
            }
            let digits = evalir_digits(name, &args[2])?;
            let expr = match &args[0] {
                Value::Text(text) if text.starts_with("#!mathir") => {
                    // The reference gates the VERSION before reading any node: everything after
                    // "#!mathir " must start with the supported version, and the refusal quotes
                    // that whole tail verbatim (Unsupported MathIR format version '1\nparam x\n'
                    // (expected 2).).
                    if let Some(error) = ir_version_error(text, pos) {
                        return Err(error);
                    }
                    match parse_ir_text(text) {
                        Some(e) => e,
                        None => {
                            return Err(LangError::plain("evalir could not read that IR text."));
                        }
                    }
                }
                Value::Symbolic(e) => e.clone(),
                // compile() answers a CompilationResult record whose payload is the IR TEXT;
                // a record carrying a symbolic "expression" is accepted too. Both are compiled
                // expressions — evalir must read either, as the reference does.
                Value::Record(_, fields) => {
                    if let Some(e) = fields
                        .iter()
                        .find(|(k, _)| k == "expression")
                        .and_then(|(_, v)| as_expr(v))
                    {
                        e
                    } else if let Some(text) =
                        fields
                            .iter()
                            .find(|(k, _)| k == "ir")
                            .and_then(|(_, v)| match v {
                                Value::Text(t) => Some(t),
                                _ => None,
                            })
                    {
                        parse_ir_text(text).ok_or_else(|| {
                            LangError::plain("evalir could not read that IR text.")
                        })?
                    } else {
                        return Err(LangError::plain("evalir needs a compiled expression."));
                    }
                }
                other => {
                    return Err(LangError::plain(format!(
                        "evalir needs a compiled expression, but got '{}'.",
                        other.kind_name()
                    )));
                }
            };
            // One free symbol, in order of first appearance.
            let mut symbols: Vec<String> = Vec::new();
            collect_all_symbols(&expr, &mut symbols);
            if symbols.len() > 1 {
                return Err(LangError::plain("evalir supports one free symbol."));
            }
            let var = symbols.first().cloned().unwrap_or_else(|| "x".to_string());
            let coeffs = lovelace_symbolics::poly_coeffs(&expr, &var)
                .ok_or_else(|| LangError::plain("evalir needs a polynomial expression."))?;
            let evaluate = |point: &Value| -> Result<Value, LangError> {
                let r = as_real(point)
                    .ok_or_else(|| LangError::plain("evalir needs numeric inputs."))?;
                let mut acc = Real::zero();
                for c in coeffs.iter().rev() {
                    let num = Int::from_decimal(&c.numerator().to_decimal())
                        .ok_or_else(|| LangError::plain("coefficient is too large."))?;
                    let den = Int::from_decimal(&c.denominator().to_decimal())
                        .ok_or_else(|| LangError::plain("coefficient is too large."))?;
                    // The caller's digit count is a PRECISION: it is the count the answer is
                    // computed at, so the coefficient division runs at that count rather than
                    // the ambient one (the reference's Real.WithPrecision(digits, ...) scope).
                    let coef = Real::from_int(&num)
                        .div(&Real::from_int(&den), digits)
                        .ok_or_else(divide_by_zero)?;
                    acc = acc.mul(&r).add(&coef);
                }
                if acc.is_integer() {
                    if let Some(i) = acc.to_int_exact() {
                        return Ok(Value::Integer(i));
                    }
                }
                // The digits argument bounds the ANSWER, not just the working precision: a
                // quotient the precision scope could not terminate inside the count is
                // published truncated to it, and the cut is carried in the value's provenance
                // (the reference publishes a precision-scoped Real, never a periodic one).
                if stored_fraction_digits(&acc) > digits {
                    let text = acc.to_decimal_fixed(digits);
                    let truncated = Real::from_str(&text).unwrap_or_else(|| acc.clone());
                    return Ok(Value::Real(truncated.mark_inexact()));
                }
                Ok(Value::Real(acc))
            };
            if name == "evalir" {
                // A single-point vector is the documented way to pass one input.
                if let Value::Vector(points) = &args[1] {
                    if points.len() == 1 {
                        return evaluate(&points[0]);
                    }
                }
                return evaluate(&args[1]);
            }
            let points = flatten_values(&args[1])?;
            let mut out = Vec::with_capacity(points.len());
            for p in &points {
                out.push(evaluate(p)?);
            }
            Ok(Value::Vector(out))
        }
        "inspect" => {
            arity(1)?;
            // The record is UNIFORM across kinds: every field is present and carries Null where it
            // does not apply, so a consumer never probes which fields a kind happens to have.
            // Exactness is a property of the NUMERIC/SYMBOLIC kinds (the same provenance the
            // value's own wire form publishes), and a non-symbolic value is inspected, not
            // refused: inspect("hi") answers type Text with Null exactness.
            let value = &args[0];
            let (type_name, domain, exact, free_symbols, node_count, canonical, pretty) =
                match value {
                    Value::Symbolic(e) => (
                        "Symbolic".to_string(),
                        Value::Domain(e.domain().name().to_string()),
                        Value::Boolean(e.is_exact()),
                        Value::Vector(e.free_symbols().into_iter().map(Value::Text).collect()),
                        Value::Natural(Nat::from_u64(e.node_count() as u64)),
                        Value::Text(e.canonical()),
                        Value::Text(e.to_string()),
                    ),
                    Value::Domain(name) => (
                        "Domain".to_string(),
                        Value::Domain(name.clone()),
                        Value::Void,
                        Value::Vector(Vec::new()),
                        Value::Void,
                        Value::Void,
                        Value::Void,
                    ),
                    Value::Natural(_) | Value::Integer(_) => (
                        value.kind_name().to_string(),
                        Value::Void,
                        Value::Boolean(true),
                        Value::Vector(Vec::new()),
                        Value::Void,
                        Value::Void,
                        Value::Void,
                    ),
                    Value::Real(r) | Value::RealClamped(r, _, _) => (
                        "Real".to_string(),
                        Value::Void,
                        Value::Boolean(r.is_exact()),
                        Value::Vector(Vec::new()),
                        Value::Void,
                        Value::Void,
                        Value::Void,
                    ),
                    Value::Complex(re, im) => (
                        "Complex".to_string(),
                        Value::Void,
                        Value::Boolean(re.is_exact() && im.is_exact()),
                        Value::Vector(Vec::new()),
                        Value::Void,
                        Value::Void,
                        Value::Void,
                    ),
                    Value::Record(name, _) => (
                        name.clone(),
                        Value::Void,
                        Value::Void,
                        Value::Vector(Vec::new()),
                        Value::Void,
                        Value::Void,
                        Value::Void,
                    ),
                    Value::Enum(name, _) => (
                        name.clone(),
                        Value::Void,
                        Value::Void,
                        Value::Vector(Vec::new()),
                        Value::Void,
                        Value::Void,
                        Value::Void,
                    ),
                    other => (
                        other.kind_name().to_string(),
                        Value::Void,
                        Value::Void,
                        Value::Vector(Vec::new()),
                        Value::Void,
                        Value::Void,
                        Value::Void,
                    ),
                };
            let domain_of = |v: &Value| -> Value {
                match v {
                    Value::Symbolic(e) => Value::Domain(e.domain().name().to_string()),
                    Value::Domain(name) => Value::Domain(name.clone()),
                    Value::Natural(_) | Value::Integer(_) => Value::Domain("integer".to_string()),
                    Value::Real(_) | Value::RealClamped(_, _, _) => {
                        Value::Domain("real".to_string())
                    }
                    Value::Complex(_, _) => Value::Domain("complex".to_string()),
                    _ => Value::Void,
                }
            };
            let (shape, rank, element_domain) = match value {
                Value::Vector(items) => (
                    Value::Vector(vec![Value::Natural(Nat::from_u64(items.len() as u64))]),
                    Value::Natural(Nat::from_u64(1)),
                    items.first().map(domain_of).unwrap_or(Value::Void),
                ),
                Value::Array(array) => {
                    let shape = array
                        .shape()
                        .iter()
                        .map(|dimension| Value::Natural(Nat::from_u64(*dimension as u64)))
                        .collect();
                    let first = array.iter().next().map(domain_of).unwrap_or(Value::Void);
                    (
                        Value::Vector(shape),
                        Value::Natural(Nat::from_u64(array.shape().len() as u64)),
                        first,
                    )
                }
                // A scalar's element domain is the value's OWN math domain, and only a
                // symbolic or domain value has one: inspect(1/3).element_domain is Null in the
                // reference, not "real".
                _ => (
                    Value::Void,
                    Value::Void,
                    match value {
                        Value::Symbolic(e) => Value::Domain(e.domain().name().to_string()),
                        Value::Domain(name) => Value::Domain(name.clone()),
                        _ => Value::Void,
                    },
                ),
            };
            let members = match value {
                Value::Record(_, fields) => Value::Vector(
                    fields
                        .iter()
                        .map(|(name, _)| Value::Text(name.clone()))
                        .collect(),
                ),
                _ => Value::Void,
            };
            Ok(Value::Record(
                "Inspection".to_string(),
                vec![
                    ("type".to_string(), Value::Text(type_name)),
                    ("domain".to_string(), domain),
                    ("exact".to_string(), exact),
                    ("free_symbols".to_string(), free_symbols),
                    ("node_count".to_string(), node_count),
                    ("canonical".to_string(), canonical),
                    ("pretty".to_string(), pretty),
                    ("shape".to_string(), shape),
                    ("rank".to_string(), rank),
                    ("element_domain".to_string(), element_domain),
                    (
                        "assumptions".to_string(),
                        Value::Vector(match value {
                            Value::Symbolic(e) => relevant_assumption_leaves(engine, e),
                            _ => Vec::new(),
                        }),
                    ),
                    ("members".to_string(), members),
                ],
            ))
        }
        "latex" => {
            arity(1)?;
            let text = match &args[0] {
                Value::Symbolic(e) => e.latex(),
                // An exact Real is a rational at heart; an inexact one is published as the
                // decimal the caller computed, exactly as the reference printer does.
                Value::Real(r) if r.is_exact() => {
                    let (num, den) = r.to_rational();
                    Sym::rational(&num, &den).latex()
                }
                Value::Real(r) => r.to_string(),
                Value::Natural(_) | Value::Integer(_) => {
                    as_expr(&args[0]).map(|e| e.latex()).unwrap_or_default()
                }
                Value::Complex(re, im) => {
                    if re.is_exact() && im.is_exact() {
                        let (rn, rd) = re.to_rational();
                        let (inn, ind) = im.to_rational();
                        format!(
                            "({} + {} i)",
                            Sym::rational(&rn, &rd).to_string(),
                            Sym::rational(&inn, &ind).to_string()
                        )
                    } else {
                        return Err(LangError::plain("latex needs an expression."));
                    }
                }
                other => {
                    return Err(LangError::plain(format!(
                        "latex needs an expression, but got '{}'.",
                        other.kind_name()
                    )));
                }
            };
            Ok(Value::Text(text))
        }
        "inf" => {
            if !args.is_empty() {
                return Err(LangError::plain("inf takes no arguments."));
            }
            Ok(Value::Text("inf".to_string()))
        }
        "series" => {
            if args.len() < 4 {
                return Err(LangError::plain(
                    "series needs an expression, a variable, a point and an order.",
                ));
            }
            let f =
                as_expr(&args[0]).ok_or_else(|| LangError::plain("series needs an expression."))?;
            let var = match &args[1] {
                Value::Symbolic(Sym::Sym(name)) => name.clone(),
                _ => return Err(LangError::plain("series needs a variable.")),
            };
            let point =
                as_expr(&args[2]).ok_or_else(|| LangError::plain("series needs a point."))?;
            let order = series_order_argument(&args[3])?;
            // The published series is the RIGHT-HAND expansion: a kink (abs(g) at a point where g
            // vanishes) is resolved by the sign of the argument's leading coefficient before
            // anything is differentiated, the coefficients are carried as expressions (so a
            // symbolic leading coefficient publishes sign(a)*a*x), and the tail is the reference's
            // order node — (order var point degree), displayed O((x - 1)^3).
            Ok(Value::Symbolic(
                lovelace_symbolics::series_kink::publish_series(&f, &var, &point, order),
            ))
        }
        "limit" | "limit_full" | "limit_left" | "limit_right" => {
            if args.len() < 3 {
                return Err(LangError::plain(
                    "limit needs an expression, a variable and a point.",
                ));
            }
            let f =
                as_expr(&args[0]).ok_or_else(|| LangError::plain("limit needs an expression."))?;
            let var = match &args[1] {
                Value::Symbolic(Sym::Sym(name)) => name.clone(),
                _ => return Err(LangError::plain("limit needs a variable.")),
            };
            let side = match name {
                "limit_left" => Side::Left,
                "limit_right" => Side::Right,
                _ => Side::Both,
            };
            // +infinity and -infinity are the same POINT for the series substitution x = 1/t
            // (the reference takes the two-sided limit at the point); -inf arrives here as the
            // negated infinity node, never as a finite point.
            let at_infinity = matches!(&args[2], Value::Text(t) if t == "inf")
                || matches!(&args[2], Value::Symbolic(Sym::Fun(f, a)) if f == "inf" && a.is_empty())
                || matches!(&args[2], Value::Symbolic(s) if is_negated_infinity(s));
            let point = as_expr(&args[2]).unwrap_or_else(|| Sym::num(0));
            Ok(limit_result(&f, &var, &point, at_infinity, side))
        }
        "solve" | "solve_full" => {
            if args.len() < 2 {
                return Err(LangError::plain("solve needs an equation and a variable."));
            }
            let var = match &args[1] {
                Value::Symbolic(Sym::Sym(name)) => name.clone(),
                _ => return Err(LangError::plain("solve needs a symbolic variable.")),
            };
            let domain = match args.get(2) {
                Some(Value::Domain(d)) => d.clone(),
                Some(Value::Text(d)) => d.clone(),
                _ => "complex".to_string(),
            };
            // The solve domain argument accepts real and complex only. A refused domain is the
            // documented InvalidOperation/DomainError refusal (the reference's own message), not a
            // silent fallback to complex: solve(x, x, integer) is a caller mistake.
            if domain != "real" && domain != "complex" {
                return Err(LangError::coded(
                    format!("solve(): currently supports domains real and complex; got {domain}."),
                    0,
                    "InvalidOperation",
                    "DomainError",
                ));
            }
            // A relation the interpreter already folded to a Boolean is still an equation: an
            // unsatisfiable one is the complete empty set, and an identity is an enumeration the
            // solver does not enumerate (0 = 0 holds for every value). Neither is a "needs an
            // equation" refusal.
            if let Value::Boolean(truth) = &args[0] {
                return Ok(if *truth {
                    unevaluated_solve_result(
                        &var,
                        &domain,
                        "solve.unevaluated",
                        "0 = 0: every value is a solution.",
                    )
                } else {
                    no_solutions_result(
                        &var,
                        &domain,
                        "the equation reduces to a nonzero constant.",
                    )
                });
            }
            let expr =
                as_expr(&args[0]).ok_or_else(|| LangError::plain("solve needs an equation."))?;
            checkpoint()?;
            let equation = match expr {
                Sym::Rel(lovelace_symbolics::RelOp::Eq, l, r) => Sym::sub(*l, *r),
                other => other,
            };
            // A quotient is solved through its numerator, carrying the denominator's
            // non-vanishing condition into the result.
            let (target, condition, denominator) = match quotient_parts(&equation) {
                Some((num, den)) => (
                    num,
                    Some(Sym::Rel(
                        lovelace_symbolics::RelOp::Ne,
                        Box::new(den.clone()),
                        Box::new(Sym::num(0)),
                    )),
                    Some(den),
                ),
                None => (equation.clone(), None, None),
            };
            // A pure power of the solved variable is a monomial. The reference's polynomial
            // conversion stores the exponent as an int: for 1..=i32::MAX that is a real
            // polynomial with one root, 0, carrying the exponent as its multiplicity, and the
            // answer is published directly (x^1000000 is one Solution(value: 0, multiplicity:
            // 1000000)). An exponent above i32::MAX narrows to a negative int in the reference's
            // own conversion and the polynomial is then classified as a nonzero constant, so the
            // reference answers NoSolutions; reproduce that classification. Without this branch
            // a large monomial would either resize a coefficient vector by the exponent or reach
            // the materialization bound and report the wrong (Unevaluated) outcome.
            if let Sym::Pow(base, exp) = &target {
                if base.as_ref() == &Sym::sym(&var) {
                    if let Sym::Num(n) = exp.as_ref() {
                        if n.is_integer() && !n.is_negative() {
                            match n.to_i64() {
                                Some(copies) if (1..=2147483647).contains(&copies) => {
                                    return Ok(solve_with_assumptions(
                                        engine,
                                        &var,
                                        domain.as_str(),
                                        lovelace_symbolics::SolveOutcome::Roots(vec![
                                            lovelace_symbolics::Root {
                                                value: lovelace_symbolics::Rat::zero(),
                                                multiplicity: copies as u64,
                                            },
                                        ]),
                                        condition,
                                    ));
                                }
                                Some(copies) if copies > 2147483647 => {
                                    return Ok(no_solutions_result(
                                        &var,
                                        domain.as_str(),
                                        "the equation reduces to a nonzero constant.",
                                    ));
                                }
                                // A positive integer too wide for i64 narrows the same way in the
                                // reference's conversion, so it is the same NoSolutions branch.
                                None => {
                                    return Ok(no_solutions_result(
                                        &var,
                                        domain.as_str(),
                                        "the equation reduces to a nonzero constant.",
                                    ));
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            // The reference's structural dispatch tries the POLYNOMIAL form first, and its
            // conversion does not expand a power of a sum. So `(x - 1)^2 == 0` is answered by the
            // inverse branch: solve `x - 1 = 0` once per nth root of zero and append each root n
            // times, which is why that spelling publishes two multiplicity-1 solutions while the
            // expanded spelling publishes one multiplicity-2 solution. Reproduce the branch so
            // both spellings agree with the oracle.
            //
            // The same branch is the reference's answer for EVERY positive integer exponent a sum
            // is raised to, not only 2..=16. Above 16 the reference cannot enumerate the n roots
            // of zero (its count parse asks for an int in 1..=16) and falls back to the PRINCIPAL
            // root, which for c = 0 is 0: `(x^2 - 1)^9223372036854775807 == 0` is therefore solved
            // as `x^2 - 1 == 0` with one multiplicity-1 entry per root. The exponent is read as
            // an i64 when it fits and as "wider than i64" otherwise; both take the principal
            // branch, so a user-sized exponent never reaches an `as usize` count, a coefficient
            // vector sized by the exponent, or the degree multiplication in poly_coeffs.
            if let Sym::Pow(base, exp) = &target {
                if let Sym::Num(n) = exp.as_ref() {
                    if n.is_integer() && !n.is_negative() && matches!(base.as_ref(), Sym::Add(_)) {
                        match n.to_i64() {
                            Some(copies) if (2..=16).contains(&copies) => {
                                if let Some(outcome) =
                                    power_branch_outcome(base, &var, domain.as_str(), copies)
                                {
                                    return Ok(solve_with_assumptions(
                                        engine,
                                        &var,
                                        domain.as_str(),
                                        outcome,
                                        condition,
                                    ));
                                }
                            }
                            // 17..=i64::MAX, and a positive integer too wide for i64: the
                            // reference's count parse fails and only the principal inverse is
                            // generated. One copy of the base's roots is the c = 0 instance of
                            // that branch.
                            Some(copies) if copies > 16 => {
                                if let Some(outcome) =
                                    power_branch_outcome(base, &var, domain.as_str(), 1)
                                {
                                    return Ok(solve_with_assumptions(
                                        engine,
                                        &var,
                                        domain.as_str(),
                                        outcome,
                                        condition,
                                    ));
                                }
                            }
                            None => {
                                if let Some(outcome) =
                                    power_branch_outcome(base, &var, domain.as_str(), 1)
                                {
                                    return Ok(solve_with_assumptions(
                                        engine,
                                        &var,
                                        domain.as_str(),
                                        outcome,
                                        condition,
                                    ));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            // A linear equation with a symbolic coefficient still has an exact solution.
            if lovelace_symbolics::poly_coeffs(&target, &var).is_none() {
                if let Some(solution) = solve_linear_symbolic(&target, &var) {
                    let (value, coefficient) = solution;
                    let condition = Sym::Rel(
                        lovelace_symbolics::RelOp::Ne,
                        Box::new(coefficient),
                        Box::new(Sym::num(0)),
                    );
                    return Ok(solve_with_assumptions(
                        engine,
                        &var,
                        domain.as_str(),
                        lovelace_symbolics::SolveOutcome::RootExpressions(vec![value]),
                        Some(condition),
                    ));
                }
            }
            // Elementary compositions: h(u) = c with an invertible head h (exp, log, a power,
            // sin/cos/tan). The reference reaches this branch only after the polynomial and
            // linear forms have declined, and every candidate is verified against the original
            // equation before it can enter a complete set.
            if lovelace_symbolics::poly_coeffs(&target, &var).is_none() {
                if let Some(branch) = solve_elementary(&target, &var, &domain, 0) {
                    return Ok(enforce_solution_values(
                        solve_branch_to_value(engine, &var, &domain, branch),
                        &var,
                    ));
                }
            }
            let outcome = match lovelace_symbolics::poly_coeffs(&target, &var) {
                Some(coeffs) => {
                    let base = lovelace_symbolics::solve_polynomial_named(&coeffs, &var);
                    // A quartic-or-higher factor with NO real root: every root is complex
                    // algebraic and RootOf is real-only, so the set is UNEVALUATED with every root
                    // counted as unrepresented — x^4 + 1 is the documented shape, never "no
                    // solutions" and never a partial set that silently drops all four roots.
                    if domain != "real" && coeffs.len() > 4 {
                        if let lovelace_symbolics::SolveOutcome::Partial(roots) = &base {
                            if roots.is_empty() {
                                return Ok(unevaluated_unrepresented_result(
                                    &var,
                                    domain.as_str(),
                                    (coeffs.len() - 1) as i64,
                                ));
                            }
                        }
                    }
                    if domain == "real" {
                        match base {
                            // No real roots were found where the domain demands them.
                            lovelace_symbolics::SolveOutcome::Partial(roots)
                                if roots.is_empty() =>
                            {
                                lovelace_symbolics::SolveOutcome::NoRealSolutions
                            }
                            lovelace_symbolics::SolveOutcome::Unevaluated => {
                                lovelace_symbolics::SolveOutcome::NoRealSolutions
                            }
                            // A root containing the imaginary unit is not a real solution.
                            lovelace_symbolics::SolveOutcome::RootExpressions(exprs) => {
                                let real: Vec<_> = exprs
                                    .into_iter()
                                    .filter(|e| !e.has_imaginary_unit())
                                    .collect();
                                if real.is_empty() {
                                    lovelace_symbolics::SolveOutcome::NoRealSolutions
                                } else {
                                    lovelace_symbolics::SolveOutcome::RootExpressions(real)
                                }
                            }
                            // Over the reals the representable (RootOf) roots are the complete
                            // answer, so the complex-domain remainder is no longer unrepresented.
                            lovelace_symbolics::SolveOutcome::RootExpressionsCount(exprs, _) => {
                                lovelace_symbolics::SolveOutcome::RootExpressions(exprs)
                            }
                            other => other,
                        }
                    } else {
                        base
                    }
                }
                None => lovelace_symbolics::SolveOutcome::Unevaluated,
            };
            let outcome = match (outcome, &denominator) {
                (lovelace_symbolics::SolveOutcome::Roots(roots), Some(den)) => {
                    let had_roots = !roots.is_empty();
                    let kept: Vec<lovelace_symbolics::Root> = roots
                        .into_iter()
                        .filter(|r| {
                            lovelace_symbolics::eval_at(den, &var, &r.value)
                                .map(|v| !v.is_zero())
                                .unwrap_or(true)
                        })
                        .collect();
                    // A numerator root that zeroes the denominator is NOT a solution: the
                    // condition is violated by the value itself. When every root is excluded the
                    // answer is NoSolutions (the reference's own note), never a Solved empty set.
                    if kept.is_empty() {
                        let message = if had_roots {
                            "every root violates the denominator condition."
                        } else {
                            "no solutions"
                        };
                        return Ok(no_solutions_result(&var, domain.as_str(), message));
                    }
                    lovelace_symbolics::SolveOutcome::Roots(kept)
                }
                (other, _) => other,
            };
            Ok(solve_with_assumptions(
                engine,
                &var,
                domain.as_str(),
                outcome,
                condition,
            ))
        }
        "factor" => {
            arity(1)?;
            let e =
                as_expr(&args[0]).ok_or_else(|| LangError::plain("factor needs an expression."))?;
            checkpoint()?;
            Ok(Value::Symbolic(factor_expr(&e)))
        }
        "expand" => {
            arity(1)?;
            let e =
                as_expr(&args[0]).ok_or_else(|| LangError::plain("expand needs an expression."))?;
            checkpoint()?;
            Ok(Value::Symbolic(lovelace_symbolics::expand(&e)))
        }
        "collect" => {
            arity(2)?;
            let e = as_expr(&args[0])
                .ok_or_else(|| LangError::plain("collect needs an expression."))?;
            let var = match &args[1] {
                Value::Symbolic(Sym::Sym(name)) => name.clone(),
                _ => return Err(LangError::plain("collect needs a variable.")),
            };
            Ok(Value::Symbolic(lovelace_symbolics::collect(&e, &var)))
        }
        "simplify" | "simplify_full" => {
            arity(1)?;
            let e = as_expr(&args[0])
                .ok_or_else(|| LangError::plain("simplify needs an expression."))?;
            checkpoint()?;
            if let Sym::Rel(op, lhs, rhs) = &e {
                if let (Sym::Sym(name), Sym::Num(value)) = (lhs.as_ref(), rhs.as_ref()) {
                    if let Some(answer) = decide_relation(engine, *op, name, value) {
                        return Ok(Value::Boolean(answer));
                    }
                }
                return Ok(Value::Symbolic(e));
            }
            if name == "simplify_full" {
                // TransformResult: the rewritten expression, its side conditions, and the
                // provenance trace (stable rule ids and classifications), never a bare value.
                if matches!(e, Sym::Rel(_, _, _)) {
                    return Ok(transform_result(
                        engine,
                        e.clone(),
                        e,
                        Vec::new(),
                        Vec::new(),
                        false,
                    ));
                }
                // A ZERO numerator over a denominator that may vanish: 0/x -> 0 with the
                // pole condition. This is the reference's rat.cancel-zero-over-x rule; the safe
                // simplify() keeps 0/x, only the full transform applies it (0/x is undefined at
                // 0 while 0 is defined).
                if let Some((num, den)) = quotient_parts(&e) {
                    if num == Sym::num(0) && !matches!(&den, Sym::Num(n) if !n.is_zero()) {
                        let condition = Sym::Rel(
                            lovelace_symbolics::RelOp::Ne,
                            Box::new(den.clone()),
                            Box::new(Sym::num(0)),
                        );
                        let step = RewriteStepData {
                            rule_id: "rat.cancel-zero-over-x",
                            classification: "Conditional",
                            before: e.clone(),
                            after: Sym::num(0),
                            required_conditions: vec![Value::Symbolic(condition.clone())],
                        };
                        return Ok(transform_result(
                            engine,
                            e.clone(),
                            Sym::num(0),
                            vec![Value::Symbolic(condition)],
                            vec![step],
                            false,
                        ));
                    }
                    if num == den {
                        let condition = Sym::Rel(
                            lovelace_symbolics::RelOp::Ne,
                            Box::new(num.clone()),
                            Box::new(Sym::num(0)),
                        );
                        let step = RewriteStepData {
                            rule_id: "rat.cancel-x-over-x",
                            classification: "Conditional",
                            before: e.clone(),
                            after: Sym::num(1),
                            required_conditions: vec![Value::Symbolic(condition.clone())],
                        };
                        return Ok(transform_result(
                            engine,
                            e.clone(),
                            Sym::num(1),
                            vec![Value::Symbolic(condition)],
                            vec![step],
                            false,
                        ));
                    }
                }
                let mut budget = RewriteBudget::new();
                let (simplified, steps) = simplify_traced(engine, e.clone(), &mut budget);
                let conditions = step_conditions(&steps);
                return Ok(transform_result(
                    engine,
                    e,
                    simplified,
                    conditions,
                    steps,
                    budget.exhausted(),
                ));
            }
            Ok(Value::Symbolic(simplify(engine, e)))
        }
        "apart" => {
            if args.len() < 2 {
                return Err(LangError::plain(
                    "apart needs an expression and a variable.",
                ));
            }
            let e =
                as_expr(&args[0]).ok_or_else(|| LangError::plain("apart needs an expression."))?;
            let var = match &args[1] {
                Value::Symbolic(Sym::Sym(n)) => n.clone(),
                _ => return Err(LangError::plain("apart needs a variable.")),
            };
            match lovelace_symbolics::apart(&e, &var) {
                Some(v) => Ok(Value::Symbolic(v)),
                None => Ok(Value::Symbolic(e)),
            }
        }
        "solve_system" | "solve_system_full" => {
            checkpoint()?;
            if args.len() < 2 {
                return Err(LangError::plain(
                    "solve_system needs equations and variables.",
                ));
            }
            // A relation folded to a Boolean is still an equation: false is a contradiction
            // (the complete empty set), true imposes nothing and is carried as 0.
            const EQUATIONS: &str = "a list of equations, e.g. [x + y == 2, x - y == 0]";
            const VARS: &str = "a list of parameter symbols, e.g. [x, y]";
            let raw_equations = argument_list(name, 1, EQUATIONS, &args[0])?;
            let mut unsatisfiable = false;
            let mut identity_only = true;
            let equations: Vec<Sym> = raw_equations
                .iter()
                .map(|v| match v {
                    Value::Boolean(false) => {
                        unsatisfiable = true;
                        identity_only = false;
                        Ok(Sym::num(1))
                    }
                    Value::Boolean(true) => Ok(Sym::num(0)),
                    other => {
                        identity_only = false;
                        let e = as_expr(other)
                            .ok_or_else(|| shape_failure(name, 1, EQUATIONS, &args[0]))?;
                        Ok(match e {
                            Sym::Rel(lovelace_symbolics::RelOp::Eq, l, r) => Sym::sub(*l, *r),
                            other => other,
                        })
                    }
                })
                .collect::<Result<_, LangError>>()?;
            let mutation_vars: Vec<String> = argument_list(name, 2, VARS, &args[1])?
                .iter()
                .map(|v| match v {
                    Value::Symbolic(Sym::Sym(n)) => Ok(n.clone()),
                    _ => Err(shape_failure(name, 2, VARS, &args[1])),
                })
                .collect::<Result<_, LangError>>()?;
            if unsatisfiable {
                return Ok(no_solutions_system());
            }
            if equations.is_empty() {
                return Ok(unevaluated_system_result(NO_EQUATIONS_NOTE));
            }
            if identity_only {
                let var = mutation_vars
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "x".to_string());
                return Ok(unevaluated_system_result(&format!(
                    "system-solve.elimination-incomplete: no equation of this branch determines {var}, so the values of {var} are not enumerated here."
                )));
            }
            match solve_system_impl(&equations, &mutation_vars) {
                SystemOutcome::Solutions(solutions) => Ok(system_result_with_assumptions(
                    engine,
                    &mutation_vars,
                    solutions,
                )),
                SystemOutcome::NoSolutions => Ok(no_solutions_system()),
                // A well-shaped system this solver cannot decide is the reference's honest
                // Unevaluated record, not an internal-sounding "no solver" error.
                SystemOutcome::Unevaluated(message) => Ok(unevaluated_system_result(&message)),
            }
        }
        "lower" => {
            if args.len() < 2 {
                return Err(LangError::plain(
                    "lower needs a kernel and parameter names.",
                ));
            }
            let e =
                as_expr(&args[0]).ok_or_else(|| LangError::plain("lower needs an expression."))?;
            let params = flatten_values(&args[1])?;
            let mut names: Vec<String> = Vec::new();
            for p in &params {
                if let Value::Symbolic(Sym::Sym(n)) = p {
                    names.push(n.clone());
                }
            }
            // The kernel carries both its textual IR and the expression it denotes, so a
            // consumer can either read the text or evaluate it.
            Ok(Value::Text(ir_text(&e, &names)))
        }
        "optimize" | "optimize_full" => {
            if args.len() < 2 {
                return Err(LangError::plain(
                    "optimize needs an expression and parameter names.",
                ));
            }
            let e = as_expr(&args[0])
                .ok_or_else(|| LangError::plain("optimize needs an expression."))?;
            let params = flatten_values(&args[1])?;
            let var = match params.first() {
                Some(Value::Symbolic(Sym::Sym(n))) => n.clone(),
                _ => return Err(LangError::plain("optimize needs parameter symbols.")),
            };
            let optimized = horner_form(&e, &var).unwrap_or_else(|| e.clone());
            if name == "optimize" {
                return Ok(Value::Symbolic(optimized));
            }
            let shared = shared_subtree_count(&e);
            let rewrites = if optimized == e { 0 } else { 1 };
            let mut transformations: Vec<Value> = Vec::new();
            if shared > 0 {
                transformations.push(Value::Text("cse".to_string()));
            }
            if rewrites > 0 {
                transformations.push(Value::Text("horner".to_string()));
            }
            Ok(Value::Record(
                "OptimizationResult".to_string(),
                vec![
                    ("original".to_string(), Value::Symbolic(e.clone())),
                    ("optimized".to_string(), Value::Symbolic(optimized.clone())),
                    (
                        "estimated_cost_before".to_string(),
                        Value::Integer(Int::from_i64(e.node_count() as i64)),
                    ),
                    (
                        "estimated_cost_after".to_string(),
                        Value::Integer(Int::from_i64(optimized.node_count() as i64)),
                    ),
                    (
                        "shared_subtrees".to_string(),
                        Value::Integer(Int::from_i64(shared as i64)),
                    ),
                    (
                        "horner_rewrites".to_string(),
                        Value::Integer(Int::from_i64(rewrites)),
                    ),
                    (
                        "transformations".to_string(),
                        Value::Vector(transformations),
                    ),
                    ("target".to_string(), Value::Text("mathir".to_string())),
                    (
                        "policy".to_string(),
                        Value::Text("cse+horner+precision-aware".to_string()),
                    ),
                    ("diagnostics".to_string(), Value::Vector(Vec::new())),
                ],
            ))
        }
        "linsolve" | "linsolve_full" => {
            arity(2)?;
            let Value::Array(a) = &args[0] else {
                return Err(LangError::plain("linsolve needs a matrix."));
            };
            let shape = a.shape().to_vec();
            if shape.len() != 2 || shape[0] != shape[1] {
                return Err(LangError::plain("linsolve needs a square matrix."));
            }
            let n = shape[0];
            let rhs = flatten_values(&args[1])?;
            if rhs.len() != n {
                return Err(LangError::plain(
                    "linsolve needs a right-hand side of the right length.",
                ));
            }
            let determinant = det(a)?;
            // A singular coefficient matrix has no solution vector at all: the record says so
            // structurally, with the diagnostic the reference publishes.
            let singular = as_expr(&determinant)
                .map(|e| e == Sym::num(0))
                .unwrap_or(false);
            if singular {
                if name == "linsolve" {
                    return Ok(Value::Vector(Vec::new()));
                }
                return Ok(matrix_no_solutions("MatrixSolveResult", "solutions"));
            }
            // The reference solves a linear system by fraction-free (Bareiss) elimination and
            // back-substitution, NOT by Cramer's rule. The difference is observable in the
            // symbolic shape: Cramer simplifies -x/x/(x*y) to -1/(x*y), while the reference's
            // back-substitution keeps the intermediate x/x (the corpus sym/076 rendering).
            let symbolic = a.iter().any(|v| !matches!(as_expr(v), Some(Sym::Num(_))))
                || rhs.iter().any(|v| !matches!(as_expr(v), Some(Sym::Num(_))));
            let mut entries: Vec<Value> = Vec::with_capacity(n);
            if symbolic {
                let mut matrix: Vec<Vec<Sym>> = Vec::with_capacity(n);
                for r in 0..n {
                    let mut row = Vec::with_capacity(n);
                    for c in 0..n {
                        row.push(a.get(&[r, c]).and_then(as_expr).ok_or_else(|| {
                            LangError::plain("linsolve needs a symbolic matrix.")
                        })?);
                    }
                    matrix.push(row);
                }
                let right: Vec<Sym> = rhs
                    .iter()
                    .map(|v| {
                        as_expr(v).ok_or_else(|| {
                            LangError::plain("linsolve needs a symbolic right-hand side.")
                        })
                    })
                    .collect::<Result<_, _>>()?;
                let solved = solve_bareiss(&matrix, &right)
                    .ok_or_else(|| LangError::plain("System is singular or underdetermined."))?;
                entries.extend(solved.into_iter().map(Value::Symbolic));
            } else {
                for col in 0..n {
                    // Cramer rule: replace the column with the right-hand side and take its determinant.
                    let mut replaced: Vec<Value> = Vec::with_capacity(n * n);
                    for r in 0..n {
                        for c in 0..n {
                            if c == col {
                                replaced.push(rhs[r].clone());
                            } else if let Some(v) = a.get(&[r, c]) {
                                replaced.push(v.clone());
                            }
                        }
                    }
                    let minor = NdArray::from_vec(vec![n, n], replaced).map_err(array_error)?;
                    let numerator = det(&minor)?;
                    entries.push(div(&numerator, &determinant, 1000)?);
                }
            }
            let condition = Sym::Rel(
                lovelace_symbolics::RelOp::Ne,
                Box::new(as_expr(&determinant).unwrap_or_else(|| Sym::num(0))),
                Box::new(Sym::num(0)),
            );
            if name == "linsolve" {
                return Ok(Value::Vector(entries));
            }
            Ok(Value::Record(
                "MatrixSolveResult".to_string(),
                vec![
                    ("status".to_string(), enum_value("SolveStatus", "Solved")),
                    ("complete".to_string(), Value::Boolean(true)),
                    (
                        "completeness".to_string(),
                        enum_value("Completeness", "Complete"),
                    ),
                    (
                        "solutions".to_string(),
                        Value::Vector(vec![Value::Vector(entries)]),
                    ),
                    (
                        "conditions".to_string(),
                        Value::Vector(vec![Value::Symbolic(condition)]),
                    ),
                    ("diagnostics".to_string(), Value::Vector(Vec::new())),
                ],
            ))
        }
        "inv_full" => {
            arity(1)?;
            let Value::Array(a) = &args[0] else {
                return Err(LangError::plain("inv_full needs a matrix."));
            };
            let d = det(a)?;
            if as_expr(&d).map(|e| e == Sym::num(0)).unwrap_or(false) {
                return Ok(matrix_no_solutions("MatrixInverseResult", "inverse"));
            }
            let inverse = inv_matrix(a)?;
            Ok(Value::Record(
                "MatrixInverseResult".to_string(),
                vec![
                    ("status".to_string(), enum_value("SolveStatus", "Solved")),
                    ("complete".to_string(), Value::Boolean(true)),
                    (
                        "completeness".to_string(),
                        enum_value("Completeness", "Complete"),
                    ),
                    ("inverse".to_string(), inverse),
                    ("conditions".to_string(), Value::Vector(Vec::new())),
                    ("diagnostics".to_string(), Value::Vector(Vec::new())),
                ],
            ))
        }
        "matrix_rank" => {
            arity(1)?;
            match &args[0] {
                Value::Array(a) => {
                    let shape = a.shape().to_vec();
                    if shape.len() != 2 {
                        return Err(LangError::plain("matrix_rank needs a rank-2 matrix."));
                    }
                    let (rows, cols) = (shape[0], shape[1]);
                    let mut rank = rows.min(cols);
                    if rows == cols {
                        let d = det(a)?;
                        let zero = as_expr(&d).map(|e| e == Sym::num(0)).unwrap_or(false);
                        if zero {
                            rank = rows.saturating_sub(1);
                        }
                    }
                    Ok(Value::Integer(Int::from_i64(rank as i64)))
                }
                other => Err(LangError::plain(format!(
                    "matrix_rank needs a matrix, but got '{}'.",
                    other.kind_name()
                ))),
            }
        }
        "cancel" => {
            arity(1)?;
            let e =
                as_expr(&args[0]).ok_or_else(|| LangError::plain("cancel needs an expression."))?;
            match lovelace_symbolics::cancel(&e) {
                Some(v) => Ok(Value::Symbolic(v)),
                None => Ok(Value::Symbolic(e)),
            }
        }
        "assumptions" => {
            // The SAME atom projection every other condition array uses, next to the human
            // rendering: the set is a structured value, never a string a caller has to parse.
            let set = engine.assumptions();
            let display = set
                .atoms()
                .iter()
                .map(|atom| assumption_text(atom))
                .collect::<Vec<String>>()
                .join("; ");
            let items: Vec<Value> = set.atoms().iter().map(assumption_condition_leaf).collect();
            Ok(Value::Record(
                "AssumptionSet".to_string(),
                vec![
                    ("display".to_string(), Value::Text(display)),
                    ("assumptions".to_string(), Value::Vector(items)),
                ],
            ))
        }
        "assume_positive" | "assume_nonnegative" | "assume_negative" => {
            arity(1)?;
            if let Value::Symbolic(Sym::Sym(symbol)) = &args[0] {
                let predicate = match name {
                    "assume_positive" => SymbolPredicate::Positive,
                    "assume_nonnegative" => SymbolPredicate::NonNegative,
                    _ => SymbolPredicate::Negative,
                };
                // The property atom is what the set stores (the reference's
                // SymbolPropertyAssumption), so assumptions() renders "x is Positive" and the
                // implication lattice can derive x > 0 from it.
                engine
                    .add_assumption(Assumption::SymbolProperty(symbol.clone(), predicate))
                    .map_err(unsatisfiable_assumptions)?;
                if matches!(
                    predicate,
                    SymbolPredicate::Positive | SymbolPredicate::NonNegative
                ) {
                    engine.mark_positive(symbol);
                }
                // solve still decides candidate roots against the relation form of the same atom.
                if let Some(op) = Assumption::relation_op(predicate) {
                    engine.record_assumption_atom(&Sym::Rel(
                        op,
                        Box::new(Sym::sym(symbol)),
                        Box::new(Sym::num(0)),
                    ));
                }
                // The reference answers the SYMBOL, not Void: the assume* builtins publish the
                // expression whose assumption they recorded.
                return Ok(Value::Symbolic(Sym::sym(symbol)));
            }
            Ok(Value::Void)
        }
        "assume_real" | "assume_integer" => {
            arity(1)?;
            // A Text name resolves to the symbol of that name, exactly as the reference's
            // assume_real("y") does; the variable itself is the argument's value.
            let symbolic = match &args[0] {
                Value::Text(name) => Sym::sym(name),
                other => as_expr(other).ok_or_else(|| {
                    LangError::plain(format!("{name} needs a symbolic variable."))
                })?,
            };
            if let Sym::Sym(variable) = &symbolic {
                let domain = if name == "assume_real" {
                    AssumptionDomain::Real
                } else {
                    AssumptionDomain::Integer
                };
                engine
                    .add_assumption(Assumption::SymbolDomain(variable.clone(), domain))
                    .map_err(unsatisfiable_assumptions)?;
                engine.record_domain_assumption(variable, domain.name());
            } else {
                return Err(LangError::plain(format!(
                    "{name} needs a symbolic variable."
                )));
            }
            Ok(Value::Symbolic(symbolic))
        }
        "assume_clear" => {
            // One engine's assumptions, never another's and never a host-wide slot.
            engine.clear_assumptions();
            Ok(Value::Text("assumptions cleared".to_string()))
        }
        "assume" => {
            arity(1)?;
            if let Value::Symbolic(Sym::Fun(fname, parts)) = &args[0] {
                if fname == "and" {
                    for part in parts.clone() {
                        let sub = Value::Symbolic(part);
                        let _ = call(engine, "assume", vec![sub], pos)?;
                    }
                    return Ok(Value::Void);
                }
                if fname == "not" && parts.len() == 1 {
                    if let Sym::Rel(op, a, b) = &parts[0] {
                        use lovelace_symbolics::RelOp;
                        let negated = match op {
                            RelOp::Eq => RelOp::Ne,
                            RelOp::Ne => RelOp::Eq,
                            RelOp::Gt => RelOp::Le,
                            RelOp::Le => RelOp::Gt,
                            RelOp::Lt => RelOp::Ge,
                            RelOp::Ge => RelOp::Lt,
                        };
                        let relation = Sym::Rel(negated, a.clone(), b.clone());
                        if let (Sym::Sym(symbol), Sym::Num(bound)) = (a.as_ref(), b.as_ref()) {
                            engine
                                .add_assumption(Assumption::SymbolRelation(
                                    symbol.clone(),
                                    negated,
                                    Sym::Num(bound.clone()),
                                ))
                                .map_err(unsatisfiable_assumptions)?;
                        }
                        engine.record_assumption(&relation.to_string());
                        engine.record_assumption_atom(&relation);
                        return Ok(Value::Symbolic(relation));
                    }
                }
            }
            match &args[0] {
                Value::Symbolic(rel @ Sym::Rel(op, lhs, rhs)) => {
                    // The set is the authority on contradictions: it knows the property lattice
                    // and the integer-domain emptiness a bound comparison cannot see.
                    if let Sym::Sym(symbol) = lhs.as_ref() {
                        engine
                            .add_assumption(Assumption::SymbolRelation(
                                symbol.clone(),
                                *op,
                                (**rhs).clone(),
                            ))
                            .map_err(unsatisfiable_assumptions)?;
                    }
                    // Bounds reasoning: a relation against a constant tightens an interval.
                    if let (Sym::Sym(name_of_symbol), Sym::Num(value)) =
                        (lhs.as_ref(), rhs.as_ref())
                    {
                        use lovelace_symbolics::RelOp;
                        let (lower, upper) = match op {
                            RelOp::Gt => (Some((value.clone(), false)), None),
                            RelOp::Ge => (Some((value.clone(), true)), None),
                            RelOp::Lt => (None, Some((value.clone(), false))),
                            RelOp::Le => (None, Some((value.clone(), true))),
                            _ => (None, None),
                        };
                        if lower.is_some() || upper.is_some() {
                            if engine.add_bound(name_of_symbol, lower, upper).is_err() {
                                let negation = match op {
                                    RelOp::Le => ">",
                                    RelOp::Lt => ">=",
                                    RelOp::Ge => "<",
                                    RelOp::Gt => "<=",
                                    RelOp::Eq => "!=",
                                    RelOp::Ne => "==",
                                };
                                return Err(LangError::plain(format!(
                                    "Assumption {} contradicts the existing assumptions (its negation {} {} is provable).",
                                    rel.to_string(),
                                    name_of_symbol,
                                    format!("{} {}", negation, value.to_string())
                                )));
                            }
                        }
                    }
                    if let Sym::Sym(name_of_symbol) = lhs.as_ref() {
                        match op {
                            lovelace_symbolics::RelOp::Ne => {
                                if let Sym::Num(n) = rhs.as_ref() {
                                    if n.is_zero() {
                                        engine.mark_nonzero(name_of_symbol);
                                    }
                                } else {
                                    engine.mark_nonzero(name_of_symbol);
                                }
                            }
                            lovelace_symbolics::RelOp::Gt | lovelace_symbolics::RelOp::Ge => {
                                engine.mark_positive(name_of_symbol);
                            }
                            _ => {}
                        }
                    }
                    engine.record_assumption(&rel.to_string());
                    engine.record_assumption_atom(rel);
                    Ok(Value::Symbolic(rel.clone()))
                }
                // A wrong-SHAPED argument is the documented argument error, never a silent
                // accept: assume(1) asserts nothing and used to answer 1 as if it had.
                other => Err(shape_failure(
                    "assume",
                    1,
                    "a relation such as x > 5",
                    other,
                )),
            }
        }
        "pi" | "e" => {
            let digits = if args.is_empty() {
                // The engine default is the DISPLAY precision (100 until setprecision moves it),
                // exactly as pi() with no argument uses Real.DisplayDecimalPlaces.
                engine.display_precision()
            } else {
                arity(1)?;
                let max = engine.precision_value().max(1);
                digit_count_bounded(name, &args[0], max, 1)?
            };
            Ok(Value::Real(if name == "pi" {
                Real::pi(digits)
            } else {
                e_constant(digits)
            }))
        }
        "setprecision" => {
            arity(1)?;
            let n = precision_digit_count(name, &args[0])?;
            engine.set_precision(n);
            Ok(Value::Void)
        }
        "capabilities" => {
            arity(0)?;
            Ok(capabilities_record())
        }
        "type" => {
            arity(1)?;
            // A record (and an enum) answers its DECLARED type name — SolveResult, SolveStatus —
            // never the wire kind "Record"/"Enum"; type() reports the value's type.
            let name = match &args[0] {
                Value::Record(type_name, _) => type_name.clone(),
                Value::Enum(type_name, _) => type_name.clone(),
                other => other.kind_name().to_string(),
            };
            Ok(Value::Text(name))
        }
        "len" => {
            arity(1)?;
            match &args[0] {
                Value::Vector(v) => Ok(Value::Natural(Nat::from_u64(v.len() as u64))),
                // A rank-0 array (a degenerate squeeze) still holds one element; never index
                // shape[0] without a fallback.
                Value::Array(a) => Ok(Value::Natural(Nat::from_u64(
                    a.shape().first().copied().unwrap_or(1) as u64,
                ))),
                Value::Text(t) => Ok(Value::Natural(Nat::from_u64(t.chars().count() as u64))),
                other => Err(LangError::plain(format!(
                    "len needs a vector, array or text, but got '{}'.",
                    other.kind_name()
                ))),
            }
        }
        "print" => {
            // Print-mode dispatch. The reference printer has four modes (Pretty, Canonical,
            // Debug, Latex) selected by enum, so the language surface names them with a second
            // argument: print(expr, "debug"). A mode name outside the documented four is a
            // CALLER-SIDE ARGUMENT problem (InvalidArgument / TypeMismatch), never a silent
            // fallback to Pretty. The one-argument form and every non-(Symbolic, Text) call keep
            // the reference's ordinary print(value, ...) behaviour.
            if let [Value::Symbolic(e), Value::Text(mode)] = args.as_slice() {
                let mode = lovelace_symbolics::PrintMode::from_name(mode).map_err(|err| {
                    LangError::coded(
                        format!("print(): {err}."),
                        0,
                        "InvalidArgument",
                        "TypeMismatch",
                    )
                })?;
                engine.push_output(e.print(mode));
                return Ok(Value::Void);
            }
            let mut line = String::new();
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    line.push(' ');
                }
                line.push_str(&a.display());
            }
            engine.push_output(line);
            Ok(Value::Void)
        }
        "plot" => builtins_plot(engine, args),
        "evalf" => {
            arity(2)?;
            let digits = evalf_digit_count(&args[1])?;
            let value = evalf_value(engine, &args[0], digits)?;
            // THE CAP IS NOT SILENT (round-20 audit I, I-2): a request above the builtin's
            // 1000-place computation cap asks for places it cannot deliver, so an INEXACT
            // result — a truncation — carries the same three wire fields a bounded rendering
            // reports. An exact answer (0.5, 2) dropped nothing and is never marked.
            Ok(clamp_evalf_result(digits, value))
        }
        "fft" => dft(engine, &args, false, "fft"),
        "dft" => dft(engine, &args, true, "dft"),
        "conv" => {
            arity(2)?;
            let a = flatten_values(&args[0])?;
            let b = flatten_values(&args[1])?;
            if a.is_empty() || b.is_empty() {
                return Err(LangError::plain("conv needs two non-empty vectors."));
            }
            let mut out = vec![Value::Natural(Nat::zero()); a.len() + b.len() - 1];
            for (i, x) in a.iter().enumerate() {
                for (j, y) in b.iter().enumerate() {
                    out[i + j] = add(&out[i + j], &mul(x, y)?)?;
                }
            }
            Ok(Value::Vector(out))
        }
        "movingavg" => dsp_movingavg(engine, &args),
        "delay" => dsp_delay(&args),
        "impulse" => dsp_impulse(&args),
        "step" => dsp_step(&args),
        "cosine" => dsp_cosine(engine, &args),
        "exponential" => dsp_exponential(engine, &args),
        "powerseries" => dsp_powerseries(engine, &args),
        "noise" => dsp_noise(engine, &args),
        // re/im/conj are the DspPlugin surface: a scalar goes through ToComplex (so a Real,
        // Integer or Natural reads as re + 0i), and an array is mapped elementwise through the
        // same conversion and flattened to a rank-1 vector, exactly as MapElements does.
        "re" => {
            arity(1)?;
            dsp_map_complex(&args[0], |re, _im| Value::Real(re))
        }
        "im" => {
            arity(1)?;
            dsp_map_complex(&args[0], |_re, im| Value::Real(im))
        }
        "conj" => {
            arity(1)?;
            dsp_map_complex(&args[0], |re, im| Value::Complex(re, im.neg()))
        }
        // ---- array construction ----
        "zeros" | "ones" => {
            let dims = dims_from(name, &args)?;
            let fill = if name == "zeros" {
                Value::Natural(Nat::zero())
            } else {
                Value::Natural(Nat::from_u64(1))
            };
            array_of(name, dims, fill)
        }
        "eye" => {
            // eye(n) / eye(rows, cols): the column count is optional, so the declared arity is
            // 1..=2 (the reference's minArity 1 over two declared parameters).
            if args.is_empty() || args.len() > 2 {
                let (message, signature) = arity_failure(name, args.len());
                let mut error = LangError::coded(message, pos, "InvalidArgument", "TypeMismatch");
                if let Some(signature) = signature {
                    error = error.with_signature(signature);
                }
                return Err(error);
            }
            // The reference reads both dimensions through the positive-count contract and then
            // measures the SHAPE against the allocation budget before any buffer exists. The old
            // n * n capacity expression overflowed on eye(2^63) and aborted the process.
            let rows =
                exact_integer_argument("eye", "rows", 1, &args[0], CountBounds::Positive)? as u64;
            let cols = if args.len() == 2 {
                exact_integer_argument("eye", "cols", 2, &args[1], CountBounds::Positive)? as u64
            } else {
                rows
            };
            let shape = [rows, cols];
            let total = ensure_allocation("eye", &shape)?;
            let mut data = Vec::with_capacity(total);
            for r in 0..rows {
                poll(r)?;
                for c in 0..cols {
                    data.push(if r == c {
                        Value::Natural(Nat::from_u64(1))
                    } else {
                        Value::Natural(Nat::zero())
                    });
                }
            }
            Ok(Value::Array(
                NdArray::from_vec(vec![rows as usize, cols as usize], data).map_err(array_error)?,
            ))
        }
        "reshape" => {
            if args.len() < 2 {
                return Err(LangError::plain(
                    "reshape needs a value and at least one dimension.",
                ));
            }
            let dims = dims_from(name, &args[1..])?;
            let shape: Vec<usize> = dims.into_iter().map(|d| d as usize).collect();
            let flat = flatten_values(&args[0])?;
            Ok(Value::Array(
                NdArray::from_vec(shape, flat).map_err(array_error)?,
            ))
        }
        "shape" => {
            arity(1)?;
            let shape = shape_of(&args[0])
                .ok_or_else(|| LangError::plain("shape needs a vector or an array."))?;
            Ok(Value::Vector(
                shape
                    .into_iter()
                    .map(|d| Value::Natural(Nat::from_u64(d as u64)))
                    .collect(),
            ))
        }
        "rank" => {
            arity(1)?;
            let shape = shape_of(&args[0])
                .ok_or_else(|| LangError::plain("rank needs a vector or an array."))?;
            Ok(Value::Natural(Nat::from_u64(shape.len() as u64)))
        }
        "numel" => {
            arity(1)?;
            let shape = shape_of(&args[0])
                .ok_or_else(|| LangError::plain("numel needs a vector or an array."))?;
            Ok(Value::Natural(Nat::from_u64(
                shape.iter().product::<usize>() as u64,
            )))
        }
        "flatten" => {
            arity(1)?;
            let flat = flatten_values(&args[0])?;
            Ok(Value::Vector(flat))
        }
        "transpose" => {
            // The descriptor declares the optional perm, so the accepted count is 1..=2; the first
            // position is guarded for SHAPE before the perm is read (transpose(1, 1) names
            // argument 1 rather than an arity or a perm failure).
            if args.is_empty() || args.len() > 2 {
                let (message, signature) = arity_failure(name, args.len());
                let mut error = LangError::coded(message, pos, "InvalidArgument", "TypeMismatch");
                if let Some(signature) = signature {
                    error = error.with_signature(signature);
                }
                return Err(error);
            }
            match &args[0] {
                Value::Array(a) => {
                    if args.len() == 2 {
                        const PERM: &str = "a list of axis positions";
                        let positions = argument_list(name, 2, PERM, &args[1])?;
                        let mut axes: Vec<usize> = Vec::with_capacity(positions.len());
                        for position in &positions {
                            let text = as_int(position)
                                .map(|i| i.to_decimal())
                                .ok_or_else(|| shape_failure(name, 2, PERM, &args[1]))?;
                            let axis = text
                                .parse::<usize>()
                                .map_err(|_| shape_failure(name, 2, PERM, &args[1]))?;
                            axes.push(axis);
                        }
                        return a
                            .transpose_axes(&axes)
                            .map(Value::Array)
                            .map_err(|err| LangError::plain(err.to_string()));
                    }
                    Ok(Value::Array(a.transpose()))
                }
                Value::Vector(_) => Ok(args[0].clone()),
                other => Err(shape_failure(name, 1, "an array or vector", other)),
            }
        }
        "squeeze" => {
            arity(1)?;
            match &args[0] {
                Value::Array(a) => {
                    let squeezed = a.squeeze();
                    // The reference collapses a shape made only of singletons to a rank-1
                    // VECTOR, never rank 0: squeeze([[[1]]]) is a one-element vector, so
                    // shape/rank/len/type all agree with it.
                    if squeezed.rank() <= 1 {
                        Ok(Value::Vector(squeezed.iter().cloned().collect()))
                    } else {
                        Ok(Value::Array(squeezed))
                    }
                }
                _ => Ok(args[0].clone()),
            }
        }
        "append" => {
            arity(2)?;
            concat_values(&args)
        }
        "concat" => {
            if args.len() < 2 || args.len() > 3 {
                let (message, signature) = arity_failure(name, args.len());
                let mut error = LangError::coded(message, pos, "InvalidArgument", "TypeMismatch");
                if let Some(signature) = signature {
                    error = error.with_signature(signature);
                }
                return Err(error);
            }
            // The optional third argument is an AXIS, not another value to concatenate: it is read
            // through the integer-argument contract (a Real is refused, never floored), and a
            // vector has exactly one axis, so any other number is the axis-range refusal.
            let inputs = if args.len() == 3 {
                let axis =
                    exact_integer_argument(name, "axis", 3, &args[2], CountBounds::Unbounded)?;
                if axis != 0 {
                    return Err(LangError::coded(
                        format!(
                            "concat(): argument 3 (axis) must be the only axis (0); got {axis}."
                        ),
                        0,
                        "InvalidArgument",
                        "TypeMismatch",
                    ));
                }
                &args[..2]
            } else {
                &args[..]
            };
            concat_values(inputs)
        }
        "sum" | "prod" | "min" | "max" => reduce(engine, name, &args),
        "mean" => reduction_builtin(engine, name, &args, Reduction::Mean),
        "norm" => reduction_builtin(engine, name, &args, Reduction::Norm),
        "dot" => {
            arity(2)?;
            let a = flatten_values(&args[0])?;
            let b = flatten_values(&args[1])?;
            if a.len() != b.len() {
                return Err(LangError::plain(
                    "dot needs two vectors of the same length.",
                ));
            }
            let mut acc = Value::Natural(Nat::zero());
            for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
                poll(i as u64)?;
                acc = add(&acc, &mul(x, y)?)?;
            }
            Ok(acc)
        }
        "cross" => {
            arity(2)?;
            let a = flatten_values(&args[0])?;
            let b = flatten_values(&args[1])?;
            if a.len() != 3 || b.len() != 3 {
                return Err(LangError::plain("cross needs two vectors of length 3."));
            }
            let c0 = sub(&mul(&a[1], &b[2])?, &mul(&a[2], &b[1])?)?;
            let c1 = sub(&mul(&a[2], &b[0])?, &mul(&a[0], &b[2])?)?;
            let c2 = sub(&mul(&a[0], &b[1])?, &mul(&a[1], &b[0])?)?;
            Ok(Value::Vector(vec![c0, c1, c2]))
        }
        "matmul" => {
            arity(2)?;
            // Both positions are array/vector operands; a scalar is the documented argument-shape
            // refusal naming the position, not the internal "Expected a matrix" sentence.
            if !matches!(&args[0], Value::Vector(_) | Value::Array(_)) {
                return Err(shape_failure(name, 1, "an array or vector", &args[0]));
            }
            if !matches!(&args[1], Value::Vector(_) | Value::Array(_)) {
                return Err(shape_failure(name, 2, "an array or vector", &args[1]));
            }
            matmul(&args[0], &args[1])
        }
        "det" => {
            arity(1)?;
            match &args[0] {
                Value::Array(a) => det(a),
                other => Err(shape_failure("det", 1, "a square matrix", other)),
            }
        }
        "trace" => {
            arity(1)?;
            match &args[0] {
                Value::Array(a) => {
                    let shape = a.shape().to_vec();
                    if shape.len() != 2 || shape[0] != shape[1] {
                        return Err(LangError::plain("trace needs a square matrix."));
                    }
                    let mut acc = Value::Natural(Nat::zero());
                    for i in 0..shape[0] {
                        if let Some(v) = a.get(&[i, i]) {
                            acc = add(&acc, v)?;
                        }
                    }
                    Ok(acc)
                }
                other => Err(shape_failure("trace", 1, "a square matrix", other)),
            }
        }
        _ => Err(unknown_function(name, pos)),
    }
}

/// The declared signature a call was measured against, when the name is in the registry.
fn declared_signature(name: &str) -> Option<ErrorSignature> {
    registry::find(name).map(|info| ErrorSignature {
        function: name.to_string(),
        parameters: Some(info.parameters.iter().map(|p| (*p).to_string()).collect()),
        expected_min: Some(info.min_arity),
        expected_max: if info.variadic {
            None
        } else {
            Some(info.parameter_count)
        },
        variadic: Some(info.variadic),
        ..ErrorSignature::default()
    })
}

/// A digit-count argument that must be a Natural or Integer: the reference's KIND refusal
/// (DomainError) or its out-of-range refusal (InvalidArgument/TypeMismatch).
fn digit_count_bounded(
    name: &str,
    arg: &Value,
    max: usize,
    min: usize,
) -> Result<usize, LangError> {
    let value = match arg {
        Value::Natural(_) | Value::Integer(_) => {
            as_int(arg).expect("a Natural or Integer converts to Int")
        }
        other => return Err(non_integer_digit_argument(name, other)),
    };
    let magnitude = value.magnitude().to_u64();
    let in_range =
        !value.is_negative() && magnitude.is_some_and(|n| n >= min as u64 && n <= max as u64);
    if !in_range {
        return Err(LangError::coded(
            format!(
                "{name}(): argument 1 (digits) must be a Natural or Integer between {min} and {max}; got {}.",
                value.to_decimal()
            ),
            0,
            "InvalidArgument",
            "TypeMismatch",
        ));
    }
    Ok(magnitude.unwrap_or(0) as usize)
}

/// The setprecision count has its own wording: any positive integer up to i64::MAX.
fn precision_digit_count(name: &str, arg: &Value) -> Result<usize, LangError> {
    let value = match arg {
        Value::Natural(_) | Value::Integer(_) => {
            as_int(arg).expect("a Natural or Integer converts to Int")
        }
        other => return Err(non_integer_digit_argument(name, other)),
    };
    let magnitude = value.magnitude().to_u64();
    let max = i64::MAX as u64;
    let in_range = !value.is_negative() && magnitude.is_some_and(|n| n >= 1 && n <= max);
    if !in_range {
        return Err(LangError::coded(
            format!(
                "{name}(): argument 1 (digits) must be a Natural or Integer that is a positive count (1 or more, at most 9223372036854775807); got {}.",
                value.to_decimal()
            ),
            0,
            "InvalidArgument",
            "TypeMismatch",
        ));
    }
    Ok(magnitude.unwrap_or(1) as usize)
}

fn non_integer_digit_argument(name: &str, arg: &Value) -> LangError {
    // An integral Real gets the write-it-as-an-integer hint; every other shape names what
    // arrived and says the parameter needs an EXACT integer.
    if let Value::Real(r) = arg {
        if let Some(i) = r.to_int_exact().filter(|_| r.is_exact()) {
            return LangError::plain(format!(
                "{name}(): argument 1 (digits) must be a Natural or Integer; got the Real {}, which is integral — write it as {}.",
                r.to_string(),
                i.to_decimal()
            ));
        }
    }
    let arrived = match arg {
        Value::Real(r) => format!("the Real {}", r.to_string()),
        other => format!("the {}", other.kind_name()),
    };
    LangError::plain(format!(
        "{name}(): argument 1 (digits) must be a Natural or Integer that denotes an exact integer; got {arrived}."
    ))
}

/// The `digits` argument of evalir/evalir_batch — a PRECISION, so a silent alteration of it is
/// a wrong answer that looks like a right one. Bounds are the sibling evalf's [1, int.MaxValue];
/// a non-integral Real is refused (never floored), an integral Real names the integer to write
/// instead, and an integer too wide for Int64 is refused by name.
fn evalir_digits(name: &str, arg: &Value) -> Result<usize, LangError> {
    let head = format!("{name}(): argument 3 (digits)");
    let bounds = "between 1 and 2147483647";
    let refuse = |message: String| LangError::coded(message, 0, "InvalidArgument", "TypeMismatch");
    match arg {
        Value::Natural(n) => {
            let text = n.to_decimal();
            match text.parse::<i64>() {
                Ok(v) if (1..=2147483647).contains(&v) => Ok(v as usize),
                Ok(v) => Err(refuse(format!(
                    "{head} must be a Natural or Integer {bounds}; got {v}."
                ))),
                Err(_) => Err(refuse(format!(
                    "{head} must be a Natural or Integer {bounds}; got {text}."
                ))),
            }
        }
        Value::Integer(i) => {
            let text = i.to_decimal();
            match text.parse::<i64>() {
                Ok(v) if (1..=2147483647).contains(&v) => Ok(v as usize),
                Ok(v) => Err(refuse(format!(
                    "{head} must be a Natural or Integer {bounds}; got {v}."
                ))),
                Err(_) => Err(refuse(format!(
                    "{head} must be a Natural or Integer {bounds}; got {text}."
                ))),
            }
        }
        Value::Real(r) => match r.to_int_exact() {
            Some(i) => {
                let text = i.to_decimal();
                Err(refuse(format!(
                    "{head} must be a Natural or Integer; got the Real {text}, which is integral — write it as {text}."
                )))
            }
            None => Err(refuse(format!(
                "{head} must be a Natural or Integer that denotes an exact integer; got {r}."
            ))),
        },
        other => Err(refuse(format!(
            "{head} must be a Natural or Integer that denotes an exact integer; got {}.",
            other.kind_name()
        ))),
    }
}

/// The evalf computation itself: the argument bounded to the REQUESTED decimal count, never to
/// the working precision (the reference's EvalfPayload / EvalfWorkingPrecision pair).
fn evalf_value(engine: &Engine, arg: &Value, digits: usize) -> Result<Value, LangError> {
    // The builtin's own computation cap: a request above 1000 publishes at most 1000 decimal
    // places (see clamp_evalf_result for the wire notice).
    let places = digits.min(1000);
    match arg {
        // An integer payload stays an integer (the reference's NumInt arm); the Integer kind is
        // what the reference publishes even for a Natural argument.
        Value::Natural(_) | Value::Integer(_) => Ok(Value::Integer(
            as_int(arg).expect("a Natural or Integer converts to Int"),
        )),
        Value::Real(r) | Value::RealClamped(r, _, _) => {
            if r.is_exact() && r.is_integer() {
                if let Some(i) = r.to_int_exact() {
                    return Ok(Value::Integer(i));
                }
            }
            // An inexact value that already fits the request is returned untouched
            // (evalf(pi(30), 40) keeps its 30 digits and its inexact flag).
            if !r.is_exact() && stored_fraction_digits(r) <= places {
                return Ok(Value::Real(r.clone()));
            }
            let text = r.to_decimal_fixed(places);
            let parsed = Real::from_str(&text).unwrap_or_else(|| r.clone());
            // A truncated decimal string of at least the plugin's computation budget is the
            // residue of an approximation, not an exact rational (Real.Parse rule).
            let boundary = engine.dsp_precision().min(1000).max(1);
            let cut = stored_fraction_digits(&parsed) >= boundary;
            Ok(Value::Real(if !r.is_exact() || cut {
                parsed.mark_inexact()
            } else {
                parsed
            }))
        }
        Value::Symbolic(expression) => {
            // The evaluated expression is a numeric tree, not a value: the reference evaluates it
            // with room for the integer part and the internal guard, then bounds the published
            // digits back to the request (EvalfWorkingPrecision / EvalfPayload).
            let mut working = digits + 20;
            let mut num = crate::numeric::evaluate(expression, working)?;
            let integer_digits = num.integer_digits();
            if integer_digits >= 20 {
                working = digits + integer_digits + 20;
                num = crate::numeric::evaluate(expression, working)?;
            }
            num.into_value(digits)
        }
        _ => Err(LangError::plain("evalf needs a number.")),
    }
}

/// The builtin's own decimal-place computation cap (EvalfDecimalPlaceCap).
const EVALF_DECIMAL_PLACE_CAP: u64 = 1000;

/// Attaches the cap's notice to an INEXACT result of a request above the cap: the places past the
/// cap are not there, so the value says so instead of crossing as if the count had been honoured.
/// An exact answer dropped nothing and is returned untouched.
fn clamp_evalf_result(digits: usize, value: Value) -> Value {
    if digits as u64 > EVALF_DECIMAL_PLACE_CAP {
        if let Value::Real(r) = &value {
            if !r.is_exact() {
                return Value::RealClamped(r.clone(), "digit-cap", EVALF_DECIMAL_PLACE_CAP);
            }
        }
    }
    value
}

/// The evalf digit count: 1..=i32::MAX, with the builtin's own word for the parameter.
fn evalf_digit_count(arg: &Value) -> Result<usize, LangError> {
    let message = |detail: &str| {
        LangError::coded(
            format!(
                "evalf(): argument 2 (digits) must be a Natural or Integer digit count between 1 and 2147483647; {detail}"
            ),
            0,
            "InvalidArgument",
            "TypeMismatch",
        )
    };
    let value = match arg {
        Value::Natural(_) | Value::Integer(_) => as_int(arg).expect("integer"),
        other => return Err(message(&format!("got {}.", other.kind_name()))),
    };
    match value.magnitude().to_u64() {
        Some(n) if !value.is_negative() && n >= 1 && n <= 2_147_483_647 => Ok(n as usize),
        _ => Err(message(&format!("got {}.", value.to_decimal()))),
    }
}

/// The fractional digits a Real actually stores: a periodic rendering has no last place, so it
/// reports usize::MAX (any requested count cuts it).
fn stored_fraction_digits(value: &Real) -> usize {
    let text = value.to_string();
    if text.contains('(') {
        return usize::MAX;
    }
    match text.find('.') {
        Some(dot) => text[dot + 1..]
            .chars()
            .filter(|c| c.is_ascii_digit())
            .count(),
        None => 0,
    }
}

/// e truncated to the requested fractional digits (the constant itself is irrational, so the
/// value is published inexact). Computed with the fixed-point factorial series.
pub(crate) fn e_constant(digits: usize) -> Real {
    let guard = 12usize;
    let scale = Nat::from_u64(10).pow(digits.saturating_add(guard) as u64);
    let mut sum = scale.clone();
    let mut term = scale;
    let mut k = 1u64;
    loop {
        let divisor = Nat::from_u64(k);
        let (next, _) = term.div_rem(&divisor).expect("nonzero divisor");
        if next.is_zero() {
            break;
        }
        sum = sum.add(&next);
        term = next;
        k += 1;
        if k > 1_000_000 {
            break;
        }
    }
    let (truncated, _) = sum
        .div_rem(&Nat::from_u64(10).pow(guard as u64))
        .expect("nonzero divisor");
    let text = truncated.to_decimal();
    let assembled = if digits == 0 {
        text
    } else if text.len() <= digits {
        let mut s = String::from("0.");
        for _ in 0..(digits - text.len()) {
            s.push('0');
        }
        s.push_str(&text);
        s
    } else {
        let split = text.len() - digits;
        format!("{}.{}", &text[..split], &text[split..])
    };
    Real::from_str(&assembled)
        .unwrap_or_else(Real::zero)
        .mark_inexact()
}

/// The arity refusal's message and signature, in the reference's grammar:
/// `dot(): expected 2 arguments; got 1.`
fn arity_failure(name: &str, actual: usize) -> (String, Option<ErrorSignature>) {
    let Some(info) = registry::find(name) else {
        return (format!("{name}(): expected arguments; got {actual}."), None);
    };
    let plural = |count: usize| if count == 1 { "argument" } else { "arguments" };
    let expected = if info.variadic {
        format!("at least {} {}", info.min_arity, plural(info.min_arity))
    } else if info.min_arity == info.parameter_count {
        format!("{} {}", info.min_arity, plural(info.min_arity))
    } else {
        format!("{} to {} arguments", info.min_arity, info.parameter_count)
    };
    let mut signature = declared_signature(name).unwrap_or_default();
    signature.actual = Some(actual);
    (
        format!("{name}(): expected {expected}; got {actual}."),
        Some(signature),
    )
}

/// Reads a position that must be a LIST (a vector or an array). A scalar where a list is required
/// is the documented argument-shape refusal, not a fallback that treats the scalar as a one-element
/// list: solve_system(x + y == 2, x - y == 0) is the call form a caller reaches for, and the answer
/// names the position and the shape it expected.
fn argument_list(
    name: &str,
    position: usize,
    expected: &str,
    value: &Value,
) -> Result<Vec<Value>, LangError> {
    match value {
        Value::Vector(items) => Ok(items.clone()),
        Value::Array(array) => Ok(array.iter().cloned().collect()),
        other => Err(shape_failure(name, position, expected, other)),
    }
}

/// A call-site SHAPE refusal: the 1-based argument position, what it requires and what arrived,
/// with the declared signature alongside it.
fn shape_failure(name: &str, position: usize, expected: &str, arrived: &Value) -> LangError {
    let mut signature = declared_signature(name).unwrap_or_else(|| ErrorSignature {
        function: name.to_string(),
        ..ErrorSignature::default()
    });
    signature.position = Some(position);
    signature.parameter = registry::find(name)
        .and_then(|info| info.parameters.get(position.saturating_sub(1)))
        .map(|p| (*p).to_string());
    signature.expected_shape = Some(expected.to_string());
    signature.arrived_kind = Some(arrived.kind_name().to_string());
    LangError::coded(
        format!(
            "{name}(): argument {position} must be {expected}; got {}.",
            arrived.kind_name()
        ),
        0,
        "InvalidArgument",
        "TypeMismatch",
    )
    .with_signature(signature)
}

/// An unresolved NAME is a vocabulary failure with its own code and category, and the registry's
/// answer — the closest names, any syntax note, and how many names exist — travels with it.
fn unknown_function(name: &str, pos: usize) -> LangError {
    let suggestions = registry::suggest(name);
    let hints = registry::hints(name);
    let mut message = format!("Unknown function '{name}'.");
    if suggestions.len() == 1 {
        message.push_str(&format!(" Did you mean '{}'?", suggestions[0]));
    } else if suggestions.len() > 1 {
        let quoted: Vec<String> = suggestions.iter().map(|s| format!("'{s}'")).collect();
        message.push_str(&format!(" Did you mean one of: {}?", quoted.join(", ")));
    }
    for hint in &hints {
        message.push(' ');
        message.push_str(hint);
    }
    let mut error = LangError::coded(message, pos, "UnknownFunction", "UnsupportedOperation");
    error.suggestions = suggestions;
    error.hints = hints;
    error.registry_size = Some(registry::BUILTINS.len());
    error
}

/// The reference's single-allocation budget for ONE array (ArrayAllocationBudget.MaxElements:
/// 2^28 elements). A constructor measures its shape against it BEFORE the buffer exists, so an
/// over-budget request cannot reach the allocator or overflow the capacity arithmetic on the way.
const MAX_SINGLE_ALLOCATION_ELEMENTS: u64 = 1 << 28;

/// Which count bound an exact-integer argument declares. The prose is the reference's own
/// Int64Range.Describe() text, which every refusal quotes verbatim.
#[derive(Clone, Copy)]
enum CountBounds {
    NonNegative,
    Positive,
    Unbounded,
}

impl CountBounds {
    fn describe(self) -> &'static str {
        match self {
            CountBounds::NonNegative => {
                "that is a non-negative count (0 or more, at most 9223372036854775807)"
            }
            CountBounds::Positive => {
                "that is a positive count (1 or more, at most 9223372036854775807)"
            }
            CountBounds::Unbounded => {
                "that fits a signed 64-bit integer (at most 9223372036854775807 in absolute value)"
            }
        }
    }

    fn contains(self, value: i64) -> bool {
        match self {
            CountBounds::NonNegative => value >= 0,
            CountBounds::Positive => value >= 1,
            CountBounds::Unbounded => true,
        }
    }
}

/// Reads an exact integer argument through the reference's ExactIntegerArgument contract: a
/// Natural or Integer in the declared bound, or the typed refusal that names the builtin, the
/// 1-based position, the parameter and the value that arrived. Position 0 is the quoted-name
/// form the range call sites use.
fn exact_integer_argument(
    name: &str,
    parameter: &str,
    position: usize,
    value: &Value,
    bounds: CountBounds,
) -> Result<i64, LangError> {
    let head = if position == 0 {
        format!("{name}(): argument '{parameter}'")
    } else {
        format!("{name}(): argument {position} ({parameter})")
    };
    let refuse = |message: String| LangError::coded(message, 0, "InvalidArgument", "TypeMismatch");
    match value {
        Value::Natural(n) => {
            let text = n.to_decimal();
            match text.parse::<i64>() {
                Ok(v) if bounds.contains(v) => Ok(v),
                Ok(v) => Err(refuse(format!(
                    "{head} must be a Natural or Integer {}; got {v}.",
                    bounds.describe()
                ))),
                Err(_) => Err(refuse(format!(
                    "{head} must be a Natural or Integer {}; got {text}.",
                    bounds.describe()
                ))),
            }
        }
        Value::Integer(i) => {
            let text = i.to_decimal();
            match text.parse::<i64>() {
                Ok(v) if bounds.contains(v) => Ok(v),
                Ok(v) => Err(refuse(format!(
                    "{head} must be a Natural or Integer {}; got {v}.",
                    bounds.describe()
                ))),
                Err(_) => Err(refuse(format!(
                    "{head} must be a Natural or Integer {}; got {text}.",
                    bounds.describe()
                ))),
            }
        }
        Value::Real(r) => match r.to_int_exact() {
            Some(i) => {
                let text = i.to_decimal();
                Err(refuse(format!(
                    "{head} must be a Natural or Integer; got the Real {text}, which is integral — write it as {text}."
                )))
            }
            None => Err(refuse(format!(
                "{head} must be a Natural or Integer that denotes an exact integer; got the Real {r}."
            ))),
        },
        other => Err(refuse(format!(
            "{head} must be a Natural or Integer that denotes an exact integer; got the {}.",
            other.kind_name()
        ))),
    }
}

/// The reference's pre-allocation budget refusal. The count is saturated to i64::MAX when the
/// true product does not fit a signed 64-bit count; the message says so rather than printing a
/// wrapped number.
fn allocation_refusal(
    name: &str,
    shape: &[u64],
    total: u64,
    product_overflowed: bool,
) -> LangError {
    let shape_text = shape
        .iter()
        .map(|dimension| dimension.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let holds = if product_overflowed {
        format!("more than {}", i64::MAX)
    } else {
        total.to_string()
    };
    LangError::coded(
        format!(
            "{name}() requested one array of shape [{shape_text}], which holds {holds} element(s) — more than the maximum single-allocation budget of {MAX_SINGLE_ALLOCATION_ELEMENTS} element(s). The request is refused BEFORE the allocation is attempted. Ask for a smaller shape, or process the data in pieces."
        ),
        0,
        "AllocationRefused",
        "BudgetExceeded",
    )
}

/// Measures a shape with wide arithmetic (never overflowing) and refuses it before a buffer is
/// created. Dimensions are already bounded to i64::MAX by the exact-integer contract.
fn ensure_allocation(name: &str, shape: &[u64]) -> Result<usize, LangError> {
    let mut total: u128 = 1;
    for &dimension in shape {
        if dimension == 0 {
            total = 0;
            continue;
        }
        total = total.saturating_mul(dimension as u128);
    }
    let overflowed = total > i64::MAX as u128;
    let requested = if overflowed {
        i64::MAX as u64
    } else {
        total as u64
    };
    if requested > MAX_SINGLE_ALLOCATION_ELEMENTS {
        return Err(allocation_refusal(name, shape, requested, overflowed));
    }
    Ok(requested as usize)
}

/// The reference's reader for series' order (the Symbolics plugin's own exact-integer contract):
/// a Natural or Integer that fits i64 and lies in 1..=int.MaxValue. A wider Natural/Integer is the
/// plugin's "denotes an exact integer" refusal (not the bound form), and an integral Real names
/// the integer to write instead. The old code cast the magnitude with as i64, which wrapped a
/// 2^63 order negative and produced a nonsense O(x^-9223372036854775808).
fn series_order_argument(value: &Value) -> Result<i64, LangError> {
    let head = "series(): argument 4 (order)";
    let bounds = "between 1 and 2147483647";
    let refuse = |message: String| LangError::coded(message, 0, "InvalidArgument", "TypeMismatch");
    match value {
        Value::Natural(n) => {
            let text = n.to_decimal();
            match text.parse::<i64>() {
                Ok(v) if (1..=2147483647).contains(&v) => Ok(v),
                Ok(v) => Err(refuse(format!(
                    "{head} must be a Natural or Integer {bounds}; got {v}."
                ))),
                Err(_) => Err(refuse(format!(
                    "{head} must be a Natural or Integer that denotes an exact integer; got the Natural {text}."
                ))),
            }
        }
        Value::Integer(i) => {
            let text = i.to_decimal();
            match text.parse::<i64>() {
                Ok(v) if (1..=2147483647).contains(&v) => Ok(v),
                Ok(v) => Err(refuse(format!(
                    "{head} must be a Natural or Integer {bounds}; got {v}."
                ))),
                Err(_) => Err(refuse(format!(
                    "{head} must be a Natural or Integer that denotes an exact integer; got the Integer {text}."
                ))),
            }
        }
        Value::Real(r) => match r.to_int_exact() {
            Some(i) => {
                let text = i.to_decimal();
                Err(refuse(format!(
                    "{head} must be a Natural or Integer; got the Real {text}, which is integral — write it as {text}."
                )))
            }
            None => Err(refuse(format!(
                "{head} must be a Natural or Integer that denotes an exact integer; got the Real {r}."
            ))),
        },
        other => Err(refuse(format!(
            "{head} must be a Natural or Integer that denotes an exact integer; got the {}.",
            other.kind_name()
        ))),
    }
}

fn dims_from(name: &str, args: &[Value]) -> Result<Vec<u64>, LangError> {
    let mut dims = Vec::with_capacity(args.len());
    for (index, argument) in args.iter().enumerate() {
        let dimension = exact_integer_argument(
            name,
            "a dimension",
            index + 1,
            argument,
            CountBounds::NonNegative,
        )?;
        dims.push(dimension as u64);
    }
    Ok(dims)
}

fn array_of(name: &str, dims: Vec<u64>, fill: Value) -> Result<Value, LangError> {
    // A five-million-element fill is itself a kernel: the elements are boxed values, so a fill can
    // cost hundreds of milliseconds and must observe the deadline like any other loop.
    checkpoint()?;
    let total = ensure_allocation(name, &dims)?;
    let shape: Vec<usize> = dims
        .into_iter()
        .map(|dimension| dimension as usize)
        .collect();
    if shape.len() == 1 {
        return Ok(Value::Vector(filled(total, &fill)?));
    }
    Ok(Value::Array(
        NdArray::from_vec(shape, filled(total, &fill)?).map_err(array_error)?,
    ))
}

/// A polled fill: builds in chunks so a deadline stops the allocation instead of waiting for the
/// whole array (and never changes the value when no deadline is armed).
fn filled(total: usize, fill: &Value) -> Result<Vec<Value>, LangError> {
    const CHUNK: usize = 65_536;
    let mut out = Vec::with_capacity(total);
    let mut done = 0usize;
    while done < total {
        checkpoint()?;
        let end = (done + CHUNK).min(total);
        out.resize(end, fill.clone());
        done = end;
    }
    Ok(out)
}

fn flatten_values(v: &Value) -> Result<Vec<Value>, LangError> {
    Ok(match v {
        Value::Vector(items) => items.clone(),
        Value::Array(a) => a.iter().cloned().collect(),
        other => vec![other.clone()],
    })
}

fn concat_values(args: &[Value]) -> Result<Value, LangError> {
    let mut any_array = false;
    for a in args {
        if matches!(a, Value::Array(_)) {
            any_array = true;
        }
    }
    if !any_array {
        let mut out = Vec::new();
        for a in args {
            out.extend(flatten_values(a)?);
        }
        return Ok(Value::Vector(out));
    }
    Err(LangError::plain(
        "concat of higher-rank arrays is not supported yet.",
    ))
}

fn numeric_total(v: &Value) -> Result<(Value, u64), LangError> {
    let flat = flatten_values(v)?;
    if flat.is_empty() {
        return Err(LangError::plain("mean() cannot reduce an empty array."));
    }
    let mut acc = Value::Natural(Nat::zero());
    for (i, item) in flat.iter().enumerate() {
        poll(i as u64)?;
        acc = add(&acc, item)?;
    }
    Ok((acc, flat.len() as u64))
}

/// Which reduction a 1-or-2-argument builtin performs over its input.
#[derive(Clone, Copy)]
enum Reduction {
    Mean,
    Norm,
}

/// mean(a) / mean(a, axis) and norm(a) / norm(a, axis): the argument COUNT is the declared
/// optional tail (minArity 1, two parameters), the argument SHAPE is checked here so a scalar
/// handed to a reduction crosses as a recoverable argument error, and the axis is checked against
/// the input's own rank (the reference's ReduceBuiltin / RequireAxisInRange pair).
fn reduction_builtin(
    engine: &Engine,
    name: &str,
    args: &[Value],
    kind: Reduction,
) -> Result<Value, LangError> {
    if args.is_empty() || args.len() > 2 {
        let (message, signature) = arity_failure(name, args.len());
        let mut error = LangError::coded(message, 0, "InvalidArgument", "TypeMismatch");
        if let Some(signature) = signature {
            error = error.with_signature(signature);
        }
        return Err(error);
    }
    let Some((shape, data)) = array_like_parts(&args[0]) else {
        return Err(shape_failure(name, 1, "an array or vector", &args[0]));
    };
    if data.is_empty() {
        return Err(LangError::plain(format!(
            "{name}() cannot reduce an empty array."
        )));
    }
    if args.len() == 1 {
        return reduce_slice(name, &data, kind, engine.precision_value());
    }
    let axis = reduction_axis(name, &args[1], shape.len())?;
    reduce_along_axis(name, &shape, &data, axis, kind, engine.precision_value())
}

/// A value's shape and row-major data, for the two array-like kinds; None for a scalar.
fn array_like_parts(value: &Value) -> Option<(Vec<usize>, Vec<Value>)> {
    match value {
        Value::Vector(items) => Some((vec![items.len()], items.clone())),
        Value::Array(array) => Some((array.shape().to_vec(), array.iter().cloned().collect())),
        _ => None,
    }
}

/// The reference's axis guard: `rank` axes numbered 0..rank-1, a rank-1 value has ONE axis, and
/// the refusal names the builtin, the position, the parameter, the rank and the number that
/// arrived (ArgumentContractException.AxisOutOfRange).
fn reduction_axis(name: &str, value: &Value, rank: usize) -> Result<usize, LangError> {
    let axis = exact_integer_argument(name, "axis", 2, value, CountBounds::Unbounded)?;
    if axis >= 0 && (axis as usize) < rank {
        return Ok(axis as usize);
    }
    let axes = if rank == 1 {
        "the only axis (0)".to_string()
    } else {
        format!("one of the value's {rank} axes, 0 to {}", rank - 1)
    };
    Err(LangError::coded(
        format!("{name}(): argument 2 (axis) must be {axes}; got {axis}."),
        0,
        "InvalidArgument",
        "TypeMismatch",
    ))
}

fn reduce_slice(
    name: &str,
    values: &[Value],
    kind: Reduction,
    precision: usize,
) -> Result<Value, LangError> {
    if values.is_empty() {
        return Err(LangError::plain(format!(
            "{name}() cannot reduce an empty array."
        )));
    }
    let mut acc = Value::Natural(Nat::zero());
    for (i, value) in values.iter().enumerate() {
        poll(i as u64)?;
        let term = match kind {
            Reduction::Mean => value.clone(),
            Reduction::Norm => mul(value, value)?,
        };
        acc = add(&acc, &term)?;
    }
    match kind {
        Reduction::Mean => div(
            &acc,
            &Value::Natural(Nat::from_u64(values.len() as u64)),
            precision,
        ),
        Reduction::Norm => {
            let real =
                as_real(&acc).ok_or_else(|| LangError::plain("norm needs numeric values."))?;
            Ok(Value::Real(real.sqrt(precision)))
        }
    }
}

/// Reduces every slice along `axis`, publishing a value of rank one lower (a scalar for a rank-1
/// input, a vector for a rank-2 one) — the reference's ReduceAxisResult.
fn reduce_along_axis(
    name: &str,
    shape: &[usize],
    data: &[Value],
    axis: usize,
    kind: Reduction,
    precision: usize,
) -> Result<Value, LangError> {
    let mut strides = vec![1usize; shape.len()];
    for d in (0..shape.len().saturating_sub(1)).rev() {
        strides[d] = strides[d + 1] * shape[d + 1];
    }
    let out_shape: Vec<usize> = shape
        .iter()
        .enumerate()
        .filter(|(d, _)| *d != axis)
        .map(|(_, dimension)| *dimension)
        .collect();
    let out_len: usize = out_shape.iter().product();
    let axis_len = shape[axis];
    let mut out = Vec::with_capacity(out_len);
    for flat in 0..out_len {
        checkpoint()?;
        let mut coords = vec![0usize; out_shape.len()];
        let mut rest = flat;
        for i in (0..out_shape.len()).rev() {
            coords[i] = rest % out_shape[i];
            rest /= out_shape[i];
        }
        let mut input_coords = Vec::with_capacity(shape.len());
        let mut out_axis = 0usize;
        for d in 0..shape.len() {
            if d == axis {
                input_coords.push(0usize);
            } else {
                input_coords.push(coords[out_axis]);
                out_axis += 1;
            }
        }
        let base: usize = input_coords
            .iter()
            .zip(strides.iter())
            .map(|(coord, stride)| coord * stride)
            .sum();
        let mut slice = Vec::with_capacity(axis_len);
        for k in 0..axis_len {
            slice.push(data[base + k * strides[axis]].clone());
        }
        out.push(reduce_slice(name, &slice, kind, precision)?);
    }
    if out_shape.is_empty() {
        return Ok(out.pop().unwrap_or(Value::Void));
    }
    if out_shape.len() == 1 {
        return Ok(Value::Vector(out));
    }
    Ok(Value::Array(
        NdArray::from_vec(out_shape, out).map_err(array_error)?,
    ))
}

fn reduce(engine: &mut Engine, name: &str, args: &[Value]) -> Result<Value, LangError> {
    if args.is_empty() {
        return Err(LangError::plain("Reduction needs a value."));
    }
    if args.len() >= 2 {
        // axis reduction: the axis is an integer-semantic argument, so a Text or fractional Real
        // is refused by the shared contract (name + position + parameter) rather than by prose.
        let axis = exact_integer_argument(name, "axis", 2, &args[1], CountBounds::Unbounded)?;
        if axis < 0 {
            return Err(LangError::coded(
                format!("{name}(): argument 2 (axis) must be the only axis (0); got {axis}."),
                0,
                "InvalidArgument",
                "TypeMismatch",
            ));
        }
        let axis = axis as usize;
        if let Value::Array(a) = &args[0] {
            let shape = a.shape().to_vec();
            // The rank check must precede the indexing: a rank-1 array has shape[0] but no
            // shape[1], and the old order read both before asking whether the rank was 2.
            if shape.len() != 2 {
                return Err(LangError::plain("Axis reduction supports rank-2 arrays."));
            }
            if axis >= shape.len() {
                return Err(LangError::plain("Axis is out of range."));
            }
            let rows = shape[0];
            let cols = shape[1];
            let mut out = Vec::new();
            if axis == 0 {
                for c in 0..cols {
                    checkpoint()?;
                    let mut acc = a.get(&[0, c]).cloned().unwrap_or(Value::Void);
                    for r in 1..rows {
                        let v = a.get(&[r, c]).cloned().unwrap_or(Value::Void);
                        acc = reduce_two(name, &acc, &v)?;
                    }
                    out.push(acc);
                }
            } else {
                for r in 0..rows {
                    checkpoint()?;
                    let mut acc = a.get(&[r, 0]).cloned().unwrap_or(Value::Void);
                    for c in 1..cols {
                        poll(c as u64)?;
                        let v = a.get(&[r, c]).cloned().unwrap_or(Value::Void);
                        acc = reduce_two(name, &acc, &v)?;
                    }
                    out.push(acc);
                }
            }
            return Ok(Value::Vector(out));
        }
    }
    let _ = engine;
    let flat = flatten_values(&args[0])?;
    if flat.is_empty() {
        // D6: the two monoids answer their identity; min/max have no identity and refuse.
        return match name {
            "sum" => Ok(Value::Natural(Nat::zero())),
            "prod" => Ok(Value::Natural(Nat::from_u64(1))),
            "min" => Err(LangError::plain("min() cannot reduce an empty array.")),
            "max" => Err(LangError::plain("max() cannot reduce an empty array.")),
            _ => Err(LangError::plain("Reduction needs at least one element.")),
        };
    }
    let mut acc = flat[0].clone();
    for (i, v) in flat[1..].iter().enumerate() {
        // A product's operands grow to six figures of digits, so the late iterations cost
        // milliseconds each: the stride is the only thing between the budget and a long overrun.
        poll(i as u64)?;
        acc = reduce_two(name, &acc, v)?;
    }
    Ok(acc)
}

fn reduce_two(name: &str, a: &Value, b: &Value) -> Result<Value, LangError> {
    match name {
        "sum" => add(a, b),
        "prod" => mul(a, b),
        "min" => Ok(if compare(a, b)? == std::cmp::Ordering::Greater {
            b.clone()
        } else {
            a.clone()
        }),
        "max" => Ok(if compare(a, b)? == std::cmp::Ordering::Less {
            b.clone()
        } else {
            a.clone()
        }),
        _ => Err(LangError::plain("Unknown reduction.")),
    }
}

fn matmul(a: &Value, b: &Value) -> Result<Value, LangError> {
    let (am, an) = matrix_dims(a)?;
    let (bm, bn) = matrix_dims(b)?;
    if an != bm {
        return Err(LangError::plain("matmul shapes do not agree."));
    }
    let a_mat = to_matrix(a)?;
    let b_mat = to_matrix(b)?;
    let mut out = Vec::with_capacity(am * bn);
    for r in 0..am {
        // One row of the result is a full sweep of the inner triple loop.
        checkpoint()?;
        for c in 0..bn {
            let mut acc = Value::Natural(Nat::zero());
            for k in 0..an {
                poll(k as u64)?;
                acc = add(&acc, &mul(&a_mat[r][k], &b_mat[k][c])?)?;
            }
            out.push(constant_first(acc));
        }
    }
    Ok(Value::Array(
        NdArray::from_vec(vec![am, bn], out).map_err(array_error)?,
    ))
}

/// Matrix accumulation publishes a constant term first; the canonical adder sorts by degree.
fn constant_first(v: Value) -> Value {
    if let Value::Symbolic(Sym::Add(terms)) = &v {
        if terms.len() == 2 {
            let constant = terms.iter().position(|t| matches!(t, Sym::Num(_)));
            if let Some(index) = constant {
                if index != 0 {
                    let mut reordered = terms.clone();
                    let term = reordered.remove(index);
                    reordered.insert(0, term);
                    return Value::Symbolic(Sym::Add(reordered));
                }
            }
        }
    }
    v
}

fn matrix_dims(v: &Value) -> Result<(usize, usize), LangError> {
    match v {
        Value::Array(a) => {
            let s = a.shape().to_vec();
            if s.len() != 2 {
                return Err(LangError::plain("Expected a rank-2 matrix."));
            }
            Ok((s[0], s[1]))
        }
        Value::Vector(_) => {
            let f = flatten_values(v)?;
            Ok((1, f.len()))
        }
        other => Err(LangError::plain(format!(
            "Expected a matrix, but got '{}'.",
            other.kind_name()
        ))),
    }
}

fn to_matrix(v: &Value) -> Result<Vec<Vec<Value>>, LangError> {
    let (r, c) = matrix_dims(v)?;
    let flat = flatten_values(v)?;
    let mut out = Vec::with_capacity(r);
    for i in 0..r {
        out.push(flat[i * c..(i + 1) * c].to_vec());
    }
    Ok(out)
}

fn det(a: &NdArray<Value>) -> Result<Value, LangError> {
    checkpoint()?;
    let shape = a.shape().to_vec();
    if shape.len() != 2 || shape[0] != shape[1] {
        return Err(LangError::plain("det needs a square matrix."));
    }
    let n = shape[0];
    let m = to_matrix(&Value::Array(a.clone()))?;
    if n == 1 {
        return Ok(m[0][0].clone());
    }
    if n == 2 {
        return sub(&mul(&m[0][0], &m[1][1])?, &mul(&m[0][1], &m[1][0])?);
    }
    // Laplace expansion along the first row.
    let mut acc = Value::Natural(Nat::zero());
    for c in 0..n {
        let mut minor = Vec::with_capacity((n - 1) * (n - 1));
        for r in 1..n {
            for cc in 0..n {
                if cc != c {
                    minor.push(m[r][cc].clone());
                }
            }
        }
        let sub_det = det(&NdArray::from_vec(vec![n - 1, n - 1], minor).map_err(array_error)?)?;
        let term = mul(&m[0][c], &sub_det)?;
        acc = if c % 2 == 0 {
            add(&acc, &term)?
        } else {
            sub(&acc, &term)?
        };
    }
    Ok(acc)
}

fn inv_matrix(a: &NdArray<Value>) -> Result<Value, LangError> {
    checkpoint()?;
    let shape = a.shape().to_vec();
    if shape.len() != 2 || shape[0] != shape[1] {
        return Err(LangError::plain("inv needs a square matrix."));
    }
    let n = shape[0];
    let m = to_matrix(&Value::Array(a.clone()))?;
    let d = det(a)?;
    if let Some(dr) = as_real(&d) {
        if dr.is_zero() {
            return Err(LangError::plain("Matrix is singular."));
        }
    }
    let mut out = Vec::with_capacity(n * n);
    for r in 0..n {
        for c in 0..n {
            let mut minor = Vec::with_capacity((n - 1) * (n - 1));
            for rr in 0..n {
                if rr != c {
                    for cc in 0..n {
                        if cc != r {
                            minor.push(m[rr][cc].clone());
                        }
                    }
                }
            }
            let minor_det = if n > 1 {
                det(&NdArray::from_vec(vec![n - 1, n - 1], minor).map_err(array_error)?)?
            } else {
                Value::Natural(Nat::from_u64(1))
            };
            let cofactor = if (r + c) % 2 == 0 {
                minor_det
            } else {
                neg(&minor_det)?
            };
            // Symbolic entries keep an explicit quotient: the reference publishes 0/(x*y)
            // rather than collapsing it to zero.
            if matches!(cofactor, Value::Symbolic(_)) || matches!(d, Value::Symbolic(_)) {
                if let (Some(cx), Some(dx)) = (as_expr(&cofactor), as_expr(&d)) {
                    out.push(Value::Symbolic(Sym::Div(Box::new(cx), Box::new(dx))));
                    continue;
                }
            }
            out.push(div(&cofactor, &d, 1000)?);
        }
    }
    Ok(Value::Array(
        NdArray::from_vec(vec![n, n], out).map_err(array_error)?,
    ))
}

// ---------------------------------------------------------------------------
// Plotting
// ---------------------------------------------------------------------------

fn to_f64(v: &Value) -> f64 {
    let text = v.display();
    let plain = match text.find('(') {
        Some(i) => text[..i].to_string(),
        None => text,
    };
    plain.parse::<f64>().unwrap_or(0.0)
}

fn builtins_plot(engine: &mut Engine, args: Vec<Value>) -> Result<Value, LangError> {
    let (xs, ys, title) = match args.len() {
        1 => {
            let ys = flatten_values(&args[0])?;
            let xs: Vec<Value> = (0..ys.len())
                .map(|i| Value::Natural(Nat::from_u64(i as u64)))
                .collect();
            (xs, ys, String::new())
        }
        2 => (
            flatten_values(&args[0])?,
            flatten_values(&args[1])?,
            String::new(),
        ),
        3 => (
            flatten_values(&args[0])?,
            flatten_values(&args[1])?,
            args[2].display(),
        ),
        _ => return Err(LangError::plain("plot takes one to three arguments.")),
    };
    if xs.len() != ys.len() || xs.is_empty() {
        return Err(LangError::plain(
            "plot needs two series of the same length.",
        ));
    }
    let width = 640.0f64;
    let height = 400.0f64;
    let margin = 40.0f64;
    let xv: Vec<f64> = xs.iter().map(to_f64).collect();
    let yv: Vec<f64> = ys.iter().map(to_f64).collect();
    let xmin = xv.iter().cloned().fold(f64::INFINITY, f64::min);
    let xmax = xv.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let ymin = yv.iter().cloned().fold(f64::INFINITY, f64::min);
    let ymax = yv.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let xr = if (xmax - xmin).abs() < 1e-12 {
        1.0
    } else {
        xmax - xmin
    };
    let yr = if (ymax - ymin).abs() < 1e-12 {
        1.0
    } else {
        ymax - ymin
    };
    let px = |x: f64| margin + (x - xmin) / xr * (width - 2.0 * margin);
    let py = |y: f64| height - margin - (y - ymin) / yr * (height - 2.0 * margin);
    let mut points = String::new();
    for (i, (x, y)) in xv.iter().zip(yv.iter()).enumerate() {
        if i > 0 {
            points.push(' ');
        }
        points.push_str(&format!("{:.3},{:.3}", px(*x), py(*y)));
    }
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">\n<rect width=\"{w}\" height=\"{h}\" fill=\"#ffffff\"/>\n<text x=\"{mx}\" y=\"24\" font-family=\"sans-serif\" font-size=\"14\">{title}</text>\n<polyline fill=\"none\" stroke=\"#1f6feb\" stroke-width=\"1.5\" points=\"{points}\"/>\n</svg>\n",
        w = width as i64,
        h = height as i64,
        mx = margin as i64,
        title = escape_xml(&title),
        points = points
    );
    let dir = engine.plot_dir();
    // The published path is ABSOLUTE and separator-normalised, like the reference's
    // Path.GetFullPath(Path.Combine(dir, file)): a caller that passed "tmp/x/plots" gets the
    // same spelling in result, plot and artifacts.
    let joined = std::path::Path::new(&dir).join(engine.plot_file());
    let mut path = std::path::absolute(&joined)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| joined.to_string_lossy().into_owned());
    #[cfg(windows)]
    {
        path = path.replace('/', "\\");
    }
    if let Some(parent) = std::path::Path::new(&path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            LangError::coded(
                format!("Cannot write plot: {e}."),
                0,
                "PlotFileError",
                "TypeMismatch",
            )
        })?;
    }
    std::fs::write(&path, &svg).map_err(|e| {
        LangError::coded(
            format!("Cannot write plot: {e}."),
            0,
            "PlotFileError",
            "TypeMismatch",
        )
    })?;
    engine.capture_plot(PlotCapture {
        path: path.clone(),
        title,
        svg,
    });
    Ok(Value::Text(path))
}

// ---------------------------------------------------------------------------
// DSP signal constructors and sample-domain operations
// ---------------------------------------------------------------------------

/// The integer bounds a DSP builtin's count/window/seed argument declares. The tail of every
/// refusal message is the reference's own Int64Range.Describe() text.
#[derive(Clone, Copy)]
enum DspIntBounds {
    Unbounded,
    NonNegative,
    /// A non-negative count the reference carries in a signed 32-bit integer (0..=2147483647).
    NonNegative32,
    Positive,
    Seed,
}

impl DspIntBounds {
    fn describe(self) -> &'static str {
        match self {
            DspIntBounds::Unbounded => {
                "that fits a signed 64-bit integer (at most 9223372036854775807 in absolute value)"
            }
            DspIntBounds::NonNegative => {
                "that is a non-negative count (0 or more, at most 9223372036854775807)"
            }
            DspIntBounds::NonNegative32 => "between 0 and 2147483647",
            DspIntBounds::Positive => {
                "that is a positive count (1 or more, at most 9223372036854775807)"
            }
            DspIntBounds::Seed => "between -2147483648 and 2147483647",
        }
    }

    fn contains(self, value: i64) -> bool {
        match self {
            DspIntBounds::Unbounded => true,
            DspIntBounds::NonNegative => value >= 0,
            DspIntBounds::NonNegative32 => (0..=2147483647).contains(&value),
            DspIntBounds::Positive => value >= 1,
            DspIntBounds::Seed => (-2147483648..=2147483647).contains(&value),
        }
    }
}

/// The payload type name the reference's TypeName reports for a language value. The plugin
/// channel unboxes a vector/array as a PayloadArray, a text as System.String, and a symbolic
/// expression as Expr; the refusals name what arrived.
fn dsp_payload_type_name(value: &Value) -> &'static str {
    match value {
        Value::Natural(_) => "Natural",
        Value::Integer(_) => "Integer",
        Value::Real(_) | Value::RealClamped(_, _, _) => "Real",
        Value::Complex(_, _) => "Complex",
        Value::Text(_) => "String",
        Value::Boolean(_) => "Boolean",
        Value::Symbolic(_) => "Expr",
        Value::Vector(_) | Value::Array(_) => "PayloadArray",
        Value::Record(_, _) => "RecordValue",
        Value::Enum(_, _) => "EnumValue",
        Value::Function(_) => "FunctionValue",
        // The reference's TypeName reports the CLR class of a domain payload.
        Value::Domain(_) => "MathDomain",
        Value::Void => "nothing",
    }
}

/// Reads an exact integer argument through the reference's ExactIntegerArgument contract.
/// A Real is accepted only when it IS an exact integer (2.0 is, 2.5 is not), exactly as the
/// C# reader's TryGetExactInt64 arm decides.
fn dsp_integer(
    value: &Value,
    name: &str,
    parameter: &str,
    position: usize,
    bounds: DspIntBounds,
) -> Result<i64, LangError> {
    let parsed: Option<i64> = match value {
        Value::Natural(n) => n.to_decimal().parse::<i64>().ok(),
        Value::Integer(i) => i.to_decimal().parse::<i64>().ok(),
        Value::Real(r) => r
            .to_int_exact()
            .and_then(|i| i.to_decimal().parse::<i64>().ok()),
        _ => None,
    };
    if let Some(v) = parsed {
        if bounds.contains(v) {
            return Ok(v);
        }
        return Err(LangError::coded(
            format!(
                "{name}(): argument {position} ({parameter}) must be a Natural or Integer {}; got {v}.",
                bounds.describe()
            ),
            0,
            "InvalidArgument",
            "TypeMismatch",
        ));
    }
    // A Natural/Integer wider than i64 is reported with its own digits under the declared
    // bound; every other payload (including any Real the reader rejected) is the
    // NotAnInteger refusal, which names what arrived rather than the bound.
    let received = match value {
        Value::Natural(n) => Some(n.to_decimal()),
        Value::Integer(i) => Some(i.to_decimal()),
        _ => None,
    };
    if let Some(received) = received {
        return Err(LangError::coded(
            format!(
                "{name}(): argument {position} ({parameter}) must be a Natural or Integer {}; got {received}.",
                bounds.describe()
            ),
            0,
            "InvalidArgument",
            "TypeMismatch",
        ));
    }
    let received = match value {
        Value::Real(r) => format!("Real {}", r.to_string()),
        other => dsp_payload_type_name(other).to_string(),
    };
    Err(LangError::coded(
        format!(
            "{name}(): argument {position} ({parameter}) must be a Natural or Integer that denotes an exact integer; got {received}."
        ),
        0,
        "InvalidArgument",
        "TypeMismatch",
    ))
}

/// The arity refusal for a DSP builtin, carrying the registry's declared signature exactly like
/// the shared dispatch helper.
fn dsp_arity(name: &str, actual: usize, min: usize, max: usize) -> Result<(), LangError> {
    if actual >= min && actual <= max {
        return Ok(());
    }
    let (message, signature) = arity_failure(name, actual);
    let mut error = LangError::coded(message, 0, "InvalidArgument", "TypeMismatch");
    if let Some(signature) = signature {
        error = error.with_signature(signature);
    }
    Err(error)
}

/// Signal.Sample's own range guard: a sequence of n samples is sampled over [0, n-1], and a
/// span wider than int.MaxValue is refused before any value is materialised.
fn dsp_sample_count(n: i64) -> Result<usize, LangError> {
    if n > 2147483647 {
        return Err(LangError::coded(
            format!(
                "The sample range [0, {}] spans more than 2147483647 samples. (Parameter 'end')",
                n - 1
            ),
            0,
            "InvalidArgument",
            "TypeMismatch",
        ));
    }
    Ok(n as usize)
}

/// A real argument through DspPlugin.ToReal (Natural, Integer or Real; no Complex).
fn dsp_to_real(value: &Value) -> Result<Real, LangError> {
    match value {
        Value::Natural(_) | Value::Integer(_) | Value::Real(_) => {
            Ok(as_real(value).expect("a numeric value converts to Real"))
        }
        other => Err(LangError::plain(format!(
            "Expected a numeric argument, but got '{}'.",
            dsp_payload_type_name(other)
        ))),
    }
}

/// One complex sample through DspPlugin.ToComplex (Complex, Real, Integer or Natural).
fn dsp_to_complex(value: &Value) -> Result<(Real, Real), LangError> {
    match value {
        Value::Complex(re, im) => Ok((re.clone(), im.clone())),
        Value::Natural(_) | Value::Integer(_) | Value::Real(_) => Ok((
            as_real(value).expect("a numeric value converts to Real"),
            Real::zero(),
        )),
        other => Err(LangError::plain(format!(
            "DSP builtins expect numeric/complex elements, but got '{}'.",
            dsp_payload_type_name(other)
        ))),
    }
}

/// The whole signal through DspPlugin.ToComplexArray: a scalar is refused (the plugin never
/// treats one as a one-sample signal), an array is flattened, and every element is converted.
fn dsp_to_complex_array(value: &Value) -> Result<Vec<(Real, Real)>, LangError> {
    let items: Vec<&Value> = match value {
        Value::Vector(items) => items.iter().collect(),
        Value::Array(array) => array.iter().collect(),
        _ => {
            return Err(LangError::plain(
                "DSP builtins expect an array argument, but got a scalar.",
            ));
        }
    };
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(dsp_to_complex(item)?);
    }
    Ok(out)
}

/// The magnitude |re + im·i| = sqrt(re² + im²) at `digits` fractional places. The squares and
/// their sum are exact (a product/sum of exact Reals is exact); the square root is exact only
/// when it squares back, so an irrational magnitude is published inexact, as the reference does.
fn complex_magnitude(re: &Real, im: &Real, digits: usize) -> Real {
    re.mul(re).add(&im.mul(im)).sqrt(digits)
}

/// The element kind a homogeneous array reports, in the reference's inference order: any Complex
/// element wins, then any Real, then any Integer, and everything else is Natural.
fn inferred_array_dtype(items: &[&Value]) -> &'static str {
    if items.iter().any(|v| matches!(v, Value::Complex(_, _))) {
        "Complex"
    } else if items.iter().any(|v| matches!(v, Value::Real(_))) {
        "Real"
    } else if items.iter().any(|v| matches!(v, Value::Integer(_))) {
        "Integer"
    } else {
        "Natural"
    }
}

/// abs() over an array: only a Complex array is accepted, and the result is the rank-1 vector of
/// magnitudes. The refusal names the inferred DType, as the reference does.
fn abs_complex_array<'a>(
    items: impl Iterator<Item = &'a Value>,
    digits: usize,
) -> Result<Value, LangError> {
    let items: Vec<&Value> = items.collect();
    let dtype = inferred_array_dtype(&items);
    if dtype != "Complex" {
        return Err(LangError::plain(format!(
            "abs() on arrays requires a Complex array (got DType.{dtype}); use abs() on individual scalars for Natural/Integer/Real."
        )));
    }
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let (re, im) = dsp_to_complex(item)?;
        out.push(Value::Real(complex_magnitude(&re, &im, digits)));
    }
    Ok(Value::Vector(out))
}

/// The DspPlugin MapElements shape: a scalar converts through ToComplex, and an array maps every
/// element through it and comes back as a rank-1 vector.
fn dsp_map_complex(
    value: &Value,
    select: impl Fn(Real, Real) -> Value,
) -> Result<Value, LangError> {
    match value {
        Value::Vector(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                let (re, im) = dsp_to_complex(item)?;
                out.push(select(re, im));
            }
            Ok(Value::Vector(out))
        }
        Value::Array(array) => {
            let mut out = Vec::with_capacity(array.numel());
            for item in array.iter() {
                let (re, im) = dsp_to_complex(item)?;
                out.push(select(re, im));
            }
            Ok(Value::Vector(out))
        }
        other => {
            let (re, im) = dsp_to_complex(other)?;
            Ok(select(re, im))
        }
    }
}

fn dsp_real(value: i64) -> Real {
    Real::from_i64(value)
}

fn dsp_complex(re: Real, im: Real) -> Value {
    Value::Complex(re, im)
}

/// True when the value stores a repeating decimal. The fixed-width complex fast path the
/// reference uses at DSP precision expands a periodic component and re-detects its period,
/// which marks the product inexact even when the rational value is exactly representable.
fn dsp_is_periodic(value: &Real) -> bool {
    value.to_string().contains('(')
}

/// The length of the repeating block a periodic value stores (0 for a finite decimal). The
/// fixed-width path can only re-detect a period that fits its significant-digit budget.
fn dsp_period_block_len(value: &Real) -> usize {
    let text = value.to_string();
    match (text.find('('), text.rfind(')')) {
        (Some(open), Some(close)) if close > open + 1 => close - open - 1,
        _ => 0,
    }
}

fn dsp_complex_mul(a: &(Real, Real), b: &(Real, Real)) -> (Real, Real) {
    let mut re = a.0.mul(&b.0).sub(&a.1.mul(&b.1));
    let mut im = a.0.mul(&b.1).add(&a.1.mul(&b.0));
    if dsp_is_periodic(&a.0)
        || dsp_is_periodic(&a.1)
        || dsp_is_periodic(&b.0)
        || dsp_is_periodic(&b.1)
    {
        re = re.mark_inexact();
        im = im.mark_inexact();
    }
    (re, im)
}

/// DspMath.PowInt: binary exponentiation over a NON-negative exponent (samples index n >= 0).
fn dsp_complex_pow_int(base: &(Real, Real), exponent: i64) -> (Real, Real) {
    let mut result = (Real::from_i64(1), Real::zero());
    let mut b = base.clone();
    let mut e = exponent.unsigned_abs();
    while e > 0 {
        if e & 1 == 1 {
            result = dsp_complex_mul(&result, &b);
        }
        b = dsp_complex_mul(&b, &b);
        e >>= 1;
    }
    result
}

/// The magnitude digits and decimal exponent of a finite decimal value, in the shape the
/// fixed-width LReal128 conversion reads them. None for a periodic or unwieldy value.
fn dsp_finite_decimal_parts(value: &Real) -> Option<(Nat, i64)> {
    let text = value.to_string();
    if text.contains('(') {
        return None;
    }
    let body = text.strip_prefix('-').unwrap_or(&text);
    let (integer, fraction) = match body.split_once('.') {
        Some((i, f)) => (i, f),
        None => (body, ""),
    };
    let mut digits = String::new();
    digits.push_str(integer.trim_start_matches('0'));
    digits.push_str(fraction);
    if digits.is_empty() {
        return Some((Nat::zero(), 0));
    }
    let magnitude = Nat::from_decimal(&digits)?;
    Some((magnitude, -(fraction.len() as i64)))
}

/// Whether LReal128.TryFromReal accepts the value: its magnitude fits a UInt128 and its
/// exponent fits an int.
fn dsp_lreal128_convertible(value: &Real) -> bool {
    let text = value.to_string();
    let body = text.strip_prefix('-').unwrap_or(&text);
    let (magnitude, exponent) = if let Some(open_paren) = body.find('(') {
        let Some(close_paren) = body.rfind(')') else {
            return false;
        };
        if close_paren <= open_paren + 1 {
            return false;
        }
        let Some((integer, fraction)) = body.split_once('.') else {
            return false;
        };
        let rest = &fraction[..close_paren - (body.find('.').map(|d| d + 1).unwrap_or(0))];
        let open = rest.find('(').unwrap_or(0);
        let close = rest.rfind(')').unwrap_or(rest.len());
        let prefix = &rest[..open];
        let period = &rest[open + 1..close];
        let mut digits = String::new();
        digits.push_str(integer.trim_start_matches('0'));
        digits.push_str(prefix);
        digits.push_str(period);
        let magnitude = if digits.is_empty() {
            Nat::zero()
        } else {
            Nat::from_decimal(&digits).unwrap_or_else(Nat::zero)
        };
        (
            magnitude,
            -((prefix.chars().count() + period.chars().count()) as i64),
        )
    } else {
        match dsp_finite_decimal_parts(value) {
            Some(parts) => parts,
            None => return false,
        }
    };
    let digits = magnitude.to_decimal();
    digits.len() <= 38 && exponent >= i32::MIN as i64 && exponent <= i32::MAX as i64
}

/// The malformed periodic rendering C#'s LReal128.Divide produces when a remainder repeats at
/// the first fractional step and the integer quotient is non-zero: the period metadata names
/// fractional positions while the stored magnitude still begins with the integer digits.
/// Returns the rendering only when that state occurs; None means the ordinary exact quotient is
/// the same observable (terminating result, or a well-formed period).
fn dsp_lreal128_division_render(numerator: &Real, divisor: i64) -> Option<String> {
    if divisor <= 0 {
        return None;
    }
    let (magnitude, exponent) = dsp_finite_decimal_parts(numerator)?;
    if magnitude.to_decimal().len() > 38 {
        return None;
    }
    let divisor_nat = Nat::from_decimal(&divisor.to_string())?;
    let (mut signature, mut remainder) = magnitude.div_rem(&divisor_nat)?;
    let mut history: Vec<(Nat, usize)> = Vec::new();
    let mut frac_count = 0usize;
    let mut period_start = 0usize;
    let mut period_length = 0usize;
    let mut found = false;
    while !remainder.is_zero() && frac_count < 512 {
        if let Some(position) = history.iter().position(|(value, _)| *value == remainder) {
            period_start = history[position].1;
            period_length = frac_count - period_start;
            found = true;
            break;
        }
        history.push((remainder.clone(), frac_count));
        remainder = remainder.mul(&Nat::from_u64(10));
        let (digit, next) = remainder.div_rem(&divisor_nat)?;
        remainder = next;
        signature = signature.mul(&Nat::from_u64(10)).add(&digit);
        if signature.to_decimal().len() > 38 {
            return None;
        }
        frac_count += 1;
    }
    if !found || period_length > 38 {
        return None;
    }
    let result_exponent = exponent - frac_count as i64;
    if result_exponent < i32::MIN as i64 || result_exponent > i32::MAX as i64 {
        return None;
    }
    let digits = signature.to_decimal();
    let frac_len = (-result_exponent).max(0) as usize;
    let padded = if digits.len() < frac_len {
        format!("{}{}", "0".repeat(frac_len - digits.len()), digits)
    } else {
        digits.clone()
    };
    let split_at = padded.len().saturating_sub(frac_len);
    let int_part = if split_at == 0 {
        "0"
    } else {
        &padded[..split_at]
    };
    let digit_at = |position: usize| -> char {
        let digits_len = digits.len() as i64;
        let frac_len = frac_len as i64;
        let idx = if position < period_start {
            digits_len - frac_len + position as i64
        } else {
            let offset = (position - period_start) % period_length;
            digits_len - frac_len + period_start as i64 + offset as i64
        };
        if idx < 0 || idx >= digits_len {
            '0'
        } else {
            digits.as_bytes()[idx as usize] as char
        }
    };
    let non_repeating: String = (0..period_start).map(digit_at).collect();
    let period: String = (0..period_length)
        .map(|offset| digit_at(period_start + offset))
        .collect();
    let sign = if numerator.cmp(&Real::zero()) == std::cmp::Ordering::Less {
        "-"
    } else {
        ""
    };
    Some(format!("{sign}{int_part}.{non_repeating}({period})"))
}

fn dsp_movingavg(engine: &Engine, args: &[Value]) -> Result<Value, LangError> {
    dsp_arity("movingavg", args.len(), 2, 2)?;
    let window = dsp_integer(&args[1], "movingavg", "w", 2, DspIntBounds::Positive)?;
    let values = dsp_to_complex_array(&args[0])?;
    if values.is_empty() {
        return Ok(Value::Vector(Vec::new()));
    }
    let precision = engine.dsp_precision();
    let divisor = dsp_real(window);
    // The reference takes its fixed-width path while the DSP precision fits 37 and every
    // sample converts into LReal128. That path's division has a periodic-state quirk the
    // arbitrary-precision path does not; the observable repair below is applied only in the
    // exact circumstances the fixed path would run.
    let fixed_path = precision <= 37
        && values
            .iter()
            .all(|(re, im)| dsp_lreal128_convertible(re) && dsp_lreal128_convertible(im));
    // A periodic operand in the fixed-width path is expanded and re-detected on every
    // operation, and the reference reports the quotient inexact even where the rational is
    // exactly representable. Long periods promote back to the exact path; only the short
    // periods the fixed width can actually re-detect keep that provenance.
    let fixed_periodic = fixed_path
        && values.iter().any(|(re, im)| {
            (dsp_is_periodic(re) && dsp_period_block_len(re) <= 38)
                || (dsp_is_periodic(im) && dsp_period_block_len(im) <= 38)
        });
    let mut out = Vec::with_capacity(values.len());
    for n in 0..values.len() {
        let start = n as i128 - window as i128 + 1;
        let mut sum_re = Real::zero();
        let mut sum_im = Real::zero();
        for k in 0..=n {
            if (k as i128) < start {
                continue;
            }
            sum_re = sum_re.add(&values[k].0);
            sum_im = sum_im.add(&values[k].1);
        }
        let re = sum_re.div(&divisor, precision).unwrap_or_else(Real::zero);
        let im = sum_im.div(&divisor, precision).unwrap_or_else(Real::zero);
        let (re, im) = if fixed_path {
            let fix = |value: &Real, quotient: Real| {
                if !dsp_lreal128_convertible(value) {
                    return quotient;
                }
                match dsp_lreal128_division_render(value, window) {
                    Some(rendered) if rendered != quotient.to_string() => {
                        Real::from_str(&rendered).unwrap_or(quotient)
                    }
                    _ => quotient,
                }
            };
            (fix(&sum_re, re), fix(&sum_im, im))
        } else {
            (re, im)
        };
        let (re, im) = if fixed_periodic {
            (re.mark_inexact(), im.mark_inexact())
        } else {
            (re, im)
        };
        out.push(dsp_complex(re, im));
    }
    Ok(Value::Vector(out))
}

fn dsp_delay(args: &[Value]) -> Result<Value, LangError> {
    dsp_arity("delay", args.len(), 2, 2)?;
    let offset = dsp_integer(&args[1], "delay", "k", 2, DspIntBounds::Unbounded)?;
    let values = dsp_to_complex_array(&args[0])?;
    if values.is_empty() {
        // Signal.Sample(signal, 0, -1) refuses before returning the empty sequence.
        return Err(LangError::coded(
            "end must be ≥ begin. (Parameter 'end')",
            0,
            "InvalidArgument",
            "TypeMismatch",
        ));
    }
    let mut out = Vec::with_capacity(values.len());
    for n in 0..values.len() {
        let index = n as i128 - offset as i128;
        if index >= 0 && (index as usize) < values.len() {
            let value = values[index as usize].clone();
            out.push(dsp_complex(value.0, value.1));
        } else {
            out.push(dsp_complex(Real::zero(), Real::zero()));
        }
    }
    Ok(Value::Vector(out))
}

fn dsp_impulse(args: &[Value]) -> Result<Value, LangError> {
    dsp_arity("impulse", args.len(), 1, 1)?;
    let n = dsp_integer(&args[0], "impulse", "n", 1, DspIntBounds::NonNegative)?;
    let n = dsp_sample_count(n)?;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        out.push(dsp_complex(
            if i == 0 {
                Real::from_i64(1)
            } else {
                Real::zero()
            },
            Real::zero(),
        ));
    }
    Ok(Value::Vector(out))
}

fn dsp_step(args: &[Value]) -> Result<Value, LangError> {
    dsp_arity("step", args.len(), 1, 1)?;
    let n = dsp_integer(&args[0], "step", "n", 1, DspIntBounds::NonNegative)?;
    let n = dsp_sample_count(n)?;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        out.push(dsp_complex(Real::from_i64(1), Real::zero()));
    }
    Ok(Value::Vector(out))
}

fn dsp_exponential(engine: &Engine, args: &[Value]) -> Result<Value, LangError> {
    dsp_arity("exponential", args.len(), 2, 2)?;
    let n = dsp_integer(&args[1], "exponential", "n", 2, DspIntBounds::NonNegative)?;
    let base = dsp_to_complex(&args[0])?;
    if n == 0 {
        return Ok(Value::Vector(Vec::new()));
    }
    let n = dsp_sample_count(n)?;
    let digits = engine.dsp_precision();
    let mut out = Vec::with_capacity(n);
    for k in 0..n {
        let z = (
            base.0.mul(&dsp_real(k as i64)),
            base.1.mul(&dsp_real(k as i64)),
        );
        let sample = dsp_complex_exp(&z, digits, engine.display_precision());
        out.push(dsp_complex(sample.0, sample.1));
    }
    Ok(Value::Vector(out))
}

fn dsp_cosine(engine: &Engine, args: &[Value]) -> Result<Value, LangError> {
    dsp_arity("cosine", args.len(), 3, 3)?;
    let n = dsp_integer(&args[2], "cosine", "n", 3, DspIntBounds::NonNegative)?;
    let frequency = dsp_to_real(&args[0])?;
    let phase = dsp_to_real(&args[1])?;
    if n == 0 {
        return Ok(Value::Vector(Vec::new()));
    }
    let n = dsp_sample_count(n)?;
    let digits = engine.dsp_precision();
    let pi = Real::pi(digits);
    let two = dsp_real(2);
    let mut out = Vec::with_capacity(n);
    for k in 0..n {
        let angle = pi
            .mul(&two)
            .mul(&frequency)
            .mul(&dsp_real(k as i64))
            .add(&phase);
        let cos = dsp_real_cos(&angle, digits, digits, engine.display_precision());
        out.push(dsp_complex(cos, Real::zero()));
    }
    Ok(Value::Vector(out))
}

fn dsp_powerseries(engine: &Engine, args: &[Value]) -> Result<Value, LangError> {
    let _ = engine;
    dsp_arity("powerseries", args.len(), 3, 3)?;
    let n = dsp_integer(&args[2], "powerseries", "n", 3, DspIntBounds::NonNegative)?;
    let k = dsp_to_complex(&args[0])?;
    let a = dsp_to_complex(&args[1])?;
    if n == 0 {
        return Ok(Value::Vector(Vec::new()));
    }
    let n = dsp_sample_count(n)?;
    let mut out = Vec::with_capacity(n);
    for index in 0..n {
        let power = dsp_complex_pow_int(&a, index as i64);
        let scaled = dsp_complex_mul(&k, &power);
        let sample = dsp_complex_mul(&scaled, &(dsp_real(index as i64), Real::zero()));
        out.push(dsp_complex(sample.0, sample.1));
    }
    Ok(Value::Vector(out))
}

fn dsp_noise(engine: &Engine, args: &[Value]) -> Result<Value, LangError> {
    dsp_arity("noise", args.len(), 3, 4)?;
    let (seed, n) = if args.len() == 3 {
        (
            0i32,
            dsp_integer(&args[2], "noise", "n", 3, DspIntBounds::NonNegative)?,
        )
    } else {
        (
            dsp_integer(&args[2], "noise", "seed", 3, DspIntBounds::Seed)? as i32,
            dsp_integer(&args[3], "noise", "n", 4, DspIntBounds::NonNegative)?,
        )
    };
    let scale = dsp_to_real(&args[0])?;
    let displacement = dsp_to_real(&args[1])?;
    if n == 0 {
        return Ok(Value::Vector(Vec::new()));
    }
    let n = dsp_sample_count(n)?;
    let digits = engine.dsp_precision();
    let mut random = DspRandom::new(seed);
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let u_re = dsp_next_unit(&mut random, digits);
        let u_im = dsp_next_unit(&mut random, digits);
        let re = scale.mul(&u_re).add(&displacement);
        let im = scale.mul(&u_im).add(&displacement);
        out.push(dsp_complex(re, im));
    }
    Ok(Value::Vector(out))
}

// ---------------------------------------------------------------------------
// Exact randomized sequence — the .NET seeded Random (Knuth subtractive generator)
// ---------------------------------------------------------------------------

struct DspRandom {
    seed_array: [i32; 56],
    inext: i32,
    inextp: i32,
}

impl DspRandom {
    const MBIG: i32 = 2147483647;
    const MSEED: i32 = 161803398;

    fn new(seed: i32) -> DspRandom {
        let mut seed_array = [0i32; 56];
        let subtraction = if seed == i32::MIN {
            Self::MBIG
        } else {
            seed.abs()
        };
        let mut mj = Self::MSEED.wrapping_sub(subtraction);
        seed_array[55] = mj;
        let mut mk = 1i32;
        for i in 1..55 {
            let ii = (21 * i) % 55;
            seed_array[ii] = mk;
            mk = mj.wrapping_sub(mk);
            if mk < 0 {
                mk = mk.wrapping_add(Self::MBIG);
            }
            mj = seed_array[ii];
        }
        for _ in 1..5 {
            for i in 1..56 {
                seed_array[i] = seed_array[i].wrapping_sub(seed_array[1 + (i + 30) % 55]);
                if seed_array[i] < 0 {
                    seed_array[i] = seed_array[i].wrapping_add(Self::MBIG);
                }
            }
        }
        DspRandom {
            seed_array,
            inext: 0,
            inextp: 21,
        }
    }

    fn internal_sample(&mut self) -> i32 {
        let mut loc_inext = self.inext;
        let mut loc_inextp = self.inextp;
        loc_inext += 1;
        if loc_inext >= 56 {
            loc_inext = 1;
        }
        loc_inextp += 1;
        if loc_inextp >= 56 {
            loc_inextp = 1;
        }
        let mut ret =
            self.seed_array[loc_inext as usize].wrapping_sub(self.seed_array[loc_inextp as usize]);
        if ret == Self::MBIG {
            ret -= 1;
        }
        if ret < 0 {
            ret = ret.wrapping_add(Self::MBIG);
        }
        self.seed_array[loc_inext as usize] = ret;
        self.inext = loc_inext;
        self.inextp = loc_inextp;
        ret
    }

    /// Random.Next(0, 10): the reference draws each decimal digit this way.
    fn next_digit(&mut self) -> u8 {
        let sample = (self.internal_sample() as f64) * (1.0 / (Self::MBIG as f64));
        (sample * 10.0) as u8
    }
}

/// Noise.NextUnit: a "0." + digits exact decimal drawn from the seeded generator. Strings as
/// wide as the computation cap are truncations, so the reference marks them inexact.
fn dsp_next_unit(random: &mut DspRandom, digits: usize) -> Real {
    let mut text = String::with_capacity(digits + 2);
    text.push_str("0.");
    for _ in 0..digits {
        text.push((b'0' + random.next_digit()) as char);
    }
    Real::from_str(&text)
        .unwrap_or_else(Real::zero)
        .mark_inexact()
}

// ---------------------------------------------------------------------------
// Arbitrary-precision exp / sin / cos at the DSP precision
// ---------------------------------------------------------------------------

/// The decimal exponent a non-periodic value is stored at: 0 for an integer (the reference
/// never normalises a positive exponent into the magnitude) and -fractional-digits otherwise.
fn dsp_decimal_exponent(value: &Real) -> i64 {
    let text = value.to_string();
    if text.contains('(') {
        return 0;
    }
    match text.split_once('.') {
        Some((_, fraction)) => -(fraction.len() as i64),
        None => 0,
    }
}

/// Real.DivideNonPeriodic: one fixed-point integer division truncated to exactly frac_digits
/// fractional places. Unlike Real::div, no repeating period is detected; a non-zero remainder
/// is a truncation and marks the result inexact.
fn dsp_divide_non_periodic(left: &Real, right: &Real, frac_digits: usize) -> Option<Real> {
    if right.is_zero() {
        return None;
    }
    if left.is_zero() {
        return Some(if left.is_exact() && right.is_exact() {
            Real::zero()
        } else {
            Real::zero().mark_inexact()
        });
    }
    let (left_num, left_den) = left.to_rational();
    let (right_num, right_den) = right.to_rational();
    let numerator = left_num
        .mul(&Int::from_decimal(&right_den.to_decimal()).expect("a natural decimal is an Int"));
    let denominator = left_den.mul(right_num.magnitude());
    let negative = numerator.is_negative();
    // The reference divides the two INTEGER magnitudes and then shifts by the operands' own
    // decimal exponents, so the result keeps frac_digits places BELOW the quotient's natural
    // scale (exponent = -frac_digits + left.Exponent - right.Exponent). Expressing that as a
    // value-relative scale gives K = frac_digits + right.Exponent - left.Exponent.
    let exponent = frac_digits as i64 + dsp_decimal_exponent(right) - dsp_decimal_exponent(left);
    let places = exponent.max(0) as usize;
    let scale = Nat::from_u64(10).pow(places as u64);
    let scaled = numerator.magnitude().mul(&scale);
    let (quotient, remainder) = scaled.div_rem(&denominator)?;
    let value = Real::from_fraction(&quotient, &scale, places)?;
    let value = if negative { value.neg() } else { value };
    if !remainder.is_zero() || !left.is_exact() || !right.is_exact() {
        Some(value.mark_inexact())
    } else {
        Some(value)
    }
}

/// 10^-(guard+1), the Taylor loops' termination threshold.
fn dsp_decimal_guard(guard: usize) -> Real {
    Real::from_str(&format!("0.{}1", "0".repeat(guard))).unwrap_or_else(Real::zero)
}

fn dsp_is_below_guard(term: &Real, guard: usize) -> bool {
    if term.is_zero() {
        return true;
    }
    term.abs().cmp(&dsp_decimal_guard(guard)) == std::cmp::Ordering::Less
}

/// Real.Exp: halve the argument into |y| < 1, sum the Taylor series at guard, then raise the
/// result back by the halving count. exp(0) is the exact one.
fn dsp_real_exp(value: &Real, digits: usize) -> Real {
    let one = Real::from_i64(1);
    if value.is_zero() {
        return one;
    }
    let guard = digits + 10;
    let mut n: i64 = 1;
    let mut y = value.clone();
    while y.abs().cmp(&one) != std::cmp::Ordering::Less {
        y = y.div(&dsp_real(2), digits).unwrap_or_else(Real::zero);
        n = n.saturating_mul(2);
    }
    let exp_y = dsp_exp_taylor(&y, guard);
    dsp_real_pow_int(&exp_y, n).mark_inexact()
}

fn dsp_exp_taylor(y: &Real, guard: usize) -> Real {
    let mut term = Real::from_i64(1);
    let mut sum = Real::from_i64(1);
    let mut k: i64 = 1;
    loop {
        let numerator = term.mul(y);
        term = dsp_divide_non_periodic(&numerator, &dsp_real(k), guard).unwrap_or_else(Real::zero);
        sum = sum.add(&term);
        if dsp_is_below_guard(&term, guard) {
            break;
        }
        k += 1;
        if k > 1_000_000 {
            break;
        }
    }
    sum
}

/// Real.Pow for a non-negative integer exponent: binary exponentiation.
fn dsp_real_pow_int(base: &Real, exponent: i64) -> Real {
    let mut result = Real::from_i64(1);
    let mut b = base.clone();
    let mut e = exponent.unsigned_abs();
    while e > 0 {
        if e & 1 == 1 {
            result = result.mul(&b);
        }
        b = b.mul(&b);
        e >>= 1;
    }
    result
}

/// The fractional places a non-periodic value stores (its -Exponent); 0 for an integer or a
/// periodic value.
fn dsp_argument_places(value: &Real) -> usize {
    let text = value.to_string();
    if text.contains('(') {
        return 0;
    }
    match text.split_once('.') {
        Some((_, fraction)) => fraction.len(),
        None => 0,
    }
}

/// The value's integer digit count (ToNatural().DecimalDigitCount() + Exponent): 0 below 1 and
/// 0 for a periodic value.
fn dsp_trig_integer_digits(value: &Real) -> usize {
    if value.to_string().contains('(') {
        return 0;
    }
    let integer = value.abs().to_decimal_fixed(0);
    let digits = integer.trim_start_matches('-').trim_start_matches('0');
    if digits.is_empty() { 0 } else { digits.len() }
}

/// Real.ReduceToTwoPi: subtract the whole number of turns without truncating small angles.
fn dsp_reduce_to_two_pi(value: &Real, two_pi: &Real) -> Real {
    if value.cmp(&Real::zero()) != std::cmp::Ordering::Less
        && value.cmp(two_pi) == std::cmp::Ordering::Less
    {
        return value.clone();
    }
    let turns = dsp_whole_quotient(value, two_pi);
    let mut reduced = value.sub(&two_pi.mul(&turns));
    if reduced.cmp(&Real::zero()) == std::cmp::Ordering::Less {
        reduced = reduced.add(two_pi);
    }
    reduced
}

/// The truncated integer quotient left / right, computed from the exact rationals.
fn dsp_whole_quotient(left: &Real, right: &Real) -> Real {
    let (left_num, left_den) = left.to_rational();
    let (right_num, right_den) = right.to_rational();
    let numerator = left_num
        .mul(&Int::from_decimal(&right_den.to_decimal()).expect("a natural decimal is an Int"));
    let denominator = left_den.mul(right_num.magnitude());
    let (quotient, _) = numerator
        .magnitude()
        .div_rem(&denominator)
        .unwrap_or((Nat::zero(), Nat::zero()));
    let quotient = Int::from_decimal(&quotient.to_decimal()).expect("a natural decimal is an Int");
    Real::from_int(&if numerator.is_negative() {
        quotient.negate()
    } else {
        quotient
    })
}

/// The angle pi * num / den built the way Real.SpecialAngleCandidate builds it: an exact
/// terminating quotient, otherwise the integer quotient truncated at pi's own decimal scale.
fn dsp_special_angle_candidate(pi: &Real, num: i64, den: i64) -> Real {
    if num == 0 {
        return Real::zero();
    }
    let places = dsp_argument_places(pi).max(1);
    pi.mul(&dsp_real(num))
        .div(&dsp_real(den), places)
        .unwrap_or_else(Real::zero)
}

/// Real.TrySpecialAngle: the 16 multiples of pi/6 and pi/4 with exact sin/cos.
fn dsp_try_special_angle(x: &Real, pi: &Real, places: usize) -> Option<(Real, Real)> {
    let half = Real::from_str("0.5").expect("0.5 is a valid decimal");
    let neg_half = half.neg();
    let one = Real::from_i64(1);
    let neg_one = one.neg();
    let sqrt2_half = Real::from_i64(2)
        .sqrt(places)
        .div(&dsp_real(2), places)
        .unwrap_or_else(Real::zero);
    let sqrt3_half = Real::from_i64(3)
        .sqrt(places)
        .div(&dsp_real(2), places)
        .unwrap_or_else(Real::zero);
    let entries: [(i64, i64, Real, Real); 16] = [
        (0, 1, Real::zero(), one.clone()),
        (1, 6, half.clone(), sqrt3_half.clone()),
        (1, 4, sqrt2_half.clone(), sqrt2_half.clone()),
        (1, 3, sqrt3_half.clone(), half.clone()),
        (1, 2, one.clone(), Real::zero()),
        (2, 3, sqrt3_half.clone(), neg_half.clone()),
        (3, 4, sqrt2_half.clone(), sqrt2_half.neg()),
        (5, 6, half.clone(), sqrt3_half.neg()),
        (1, 1, Real::zero(), neg_one.clone()),
        (7, 6, neg_half.clone(), sqrt3_half.neg()),
        (5, 4, sqrt2_half.neg(), sqrt2_half.neg()),
        (4, 3, sqrt3_half.neg(), neg_half.clone()),
        (3, 2, neg_one, Real::zero()),
        (5, 3, sqrt3_half.neg(), half.clone()),
        (7, 4, sqrt2_half.neg(), sqrt2_half.clone()),
        (11, 6, neg_half, sqrt3_half),
    ];
    for (num, den, sin, cos) in entries {
        let angle = dsp_special_angle_candidate(pi, num, den);
        if x.cmp(&angle) == std::cmp::Ordering::Equal {
            return Some((sin, cos));
        }
    }
    None
}

/// pi at the width the reduction needs: never narrower than the working scope.
fn dsp_fetch_pi(places: usize, working: usize) -> Real {
    Real::pi(places.max(working))
}

fn dsp_sin_taylor(x: &Real, guard: usize) -> Real {
    let x2 = x.mul(x);
    let mut term = x.clone();
    let mut sum = x.clone();
    let mut k: i64 = 1;
    loop {
        let numerator = term.mul(&x2).neg();
        term = dsp_divide_non_periodic(&numerator, &dsp_real((2 * k) * (2 * k + 1)), guard)
            .unwrap_or_else(Real::zero);
        sum = sum.add(&term);
        if dsp_is_below_guard(&term, guard) {
            break;
        }
        k += 1;
        if k > 1_000_000 {
            break;
        }
    }
    sum
}

fn dsp_cos_taylor(x: &Real, guard: usize) -> Real {
    let x2 = x.mul(x);
    let mut term = Real::from_i64(1);
    let mut sum = Real::from_i64(1);
    let mut k: i64 = 1;
    loop {
        let numerator = term.mul(&x2).neg();
        term = dsp_divide_non_periodic(&numerator, &dsp_real((2 * k - 1) * (2 * k)), guard)
            .unwrap_or_else(Real::zero);
        sum = sum.add(&term);
        if dsp_is_below_guard(&term, guard) {
            break;
        }
        k += 1;
        if k > 1_000_000 {
            break;
        }
    }
    sum
}

fn dsp_real_cos(value: &Real, digits: usize, ambient: usize, publish: usize) -> Real {
    if value.cmp(&Real::zero()) == std::cmp::Ordering::Less {
        return dsp_real_cos(&value.neg(), digits, ambient, publish);
    }
    let ambient_pi = Real::pi(ambient);
    let ambient_two_pi = ambient_pi.mul(&dsp_real(2));
    let mut x = dsp_reduce_to_two_pi(value, &ambient_two_pi);
    if let Some((_, cos)) = dsp_try_special_angle(&x, &ambient_pi, ambient) {
        return cos;
    }
    let guard = digits + 10;
    let integer_digits = dsp_trig_integer_digits(value);
    let working = (digits + 10 + integer_digits).max(ambient);
    let pi_places = integer_digits + dsp_argument_places(value) + digits + 12;
    let pi = dsp_fetch_pi(pi_places, working);
    let two_pi = pi.mul(&dsp_real(2));
    let half_pi = pi.div(&dsp_real(2), working).unwrap_or_else(Real::zero);
    x = dsp_reduce_to_two_pi(value, &two_pi);
    if let Some((_, cos)) = dsp_try_special_angle(&x, &pi, working) {
        return cos;
    }
    let mut negate = false;
    if x.cmp(&pi) != std::cmp::Ordering::Greater {
        if x.cmp(&half_pi) == std::cmp::Ordering::Greater {
            x = pi.sub(&x);
            negate = true;
        }
    } else if x.cmp(&pi.add(&half_pi)) != std::cmp::Ordering::Greater {
        x = x.sub(&pi);
        negate = true;
    } else {
        x = two_pi.sub(&x);
    }
    let distance = half_pi.sub(&x);
    let cos = if dsp_is_below_guard(&distance, guard) {
        dsp_sin_taylor(&distance, guard)
    } else {
        dsp_cos_taylor(&x, guard)
    };
    // The reference's Real.ToString publishes a non-periodic value at the display cap
    // (Real.DisplayDecimalPlaces); the language's typed rendering has no separate display step,
    // so the transcendental result is published here at the same width.
    let cos = cos.mark_inexact().truncated_frac(publish);
    if negate { cos.neg() } else { cos }
}

fn dsp_real_sin(value: &Real, digits: usize, publish: usize) -> Real {
    if value.cmp(&Real::zero()) == std::cmp::Ordering::Less {
        return dsp_real_sin(&value.neg(), digits, publish).neg();
    }
    let ambient = digits;
    let ambient_pi = Real::pi(ambient);
    let ambient_two_pi = ambient_pi.mul(&dsp_real(2));
    let mut x = dsp_reduce_to_two_pi(value, &ambient_two_pi);
    if let Some((sin, _)) = dsp_try_special_angle(&x, &ambient_pi, ambient) {
        return sin;
    }
    let guard = digits + 10;
    let integer_digits = dsp_trig_integer_digits(value);
    let working = (digits + 10 + integer_digits).max(ambient);
    let pi_places = integer_digits + dsp_argument_places(value) + digits + 12;
    let pi = dsp_fetch_pi(pi_places, working);
    let two_pi = pi.mul(&dsp_real(2));
    let half_pi = pi.div(&dsp_real(2), working).unwrap_or_else(Real::zero);
    x = dsp_reduce_to_two_pi(value, &two_pi);
    if let Some((sin, _)) = dsp_try_special_angle(&x, &pi, working) {
        return sin;
    }
    let mut negative = false;
    if x.cmp(&pi) != std::cmp::Ordering::Greater {
        if x.cmp(&half_pi) == std::cmp::Ordering::Greater {
            x = pi.sub(&x);
        }
    } else if x.cmp(&pi.add(&half_pi)) != std::cmp::Ordering::Greater {
        x = x.sub(&pi);
        negative = true;
    } else {
        x = two_pi.sub(&x);
        negative = true;
    }
    let sin = dsp_sin_taylor(&x, guard)
        .mark_inexact()
        .truncated_frac(publish);
    if negative { sin.neg() } else { sin }
}

/// Complex.Exp: e^z = e^re * (cos(im) + i*sin(im)).
fn dsp_complex_exp(z: &(Real, Real), digits: usize, publish: usize) -> (Real, Real) {
    let exponential = dsp_real_exp(&z.0, digits);
    if z.1.is_zero() {
        return (exponential, Real::zero());
    }
    let cos = dsp_real_cos(&z.1, digits, digits, publish);
    let sin = dsp_real_sin(&z.1, digits, publish);
    (exponential.mul(&cos), exponential.mul(&sin))
}

fn dft(engine: &Engine, args: &[Value], _inverse: bool, name: &str) -> Result<Value, LangError> {
    if args.is_empty() {
        return Err(LangError::plain("dft needs a vector."));
    }
    let values = flatten_values(&args[0])?;
    if values.is_empty() {
        return Err(LangError::plain("dft needs a non-empty vector."));
    }
    // The optional length is an integer-semantic argument, not a value to transform: the
    // reference zero-pads (or truncates) the input to exactly n samples and n = 0 is the empty
    // transform. Ignoring it answered a DIFFERENT transform than the caller asked for.
    let n = if args.len() >= 2 {
        dsp_integer(&args[1], name, "n", 2, DspIntBounds::NonNegative32)? as usize
    } else {
        values.len()
    };
    if n == 0 {
        return Ok(Value::Vector(Vec::new()));
    }
    let digits = engine.dsp_precision();
    let mut samples: Vec<(Real, Real)> = Vec::with_capacity(n);
    for value in &values {
        samples.push(complex_sample(value)?);
    }
    samples.resize(n, (Real::zero(), Real::zero()));
    let roots = roots_of_unity(n, digits);
    let mut out = Vec::with_capacity(n);
    for k in 0..n {
        let mut sum_re = Real::zero();
        let mut sum_im = Real::zero();
        for (t, sample) in samples.iter().enumerate() {
            let (root_re, root_im) = &roots[(k * t) % n];
            let re = sample.0.mul(root_re).sub(&sample.1.mul(root_im));
            let im = sample.0.mul(root_im).add(&sample.1.mul(root_re));
            sum_re = sum_re.add(&re);
            sum_im = sum_im.add(&im);
        }
        out.push(Value::Complex(sum_re, sum_im));
    }
    Ok(Value::Vector(out))
}

/// One DFT sample as (real, imaginary) decimal parts.
fn complex_sample(value: &Value) -> Result<(Real, Real), LangError> {
    match value {
        Value::Natural(n) => {
            let int = Int::from_decimal(&n.to_decimal())
                .ok_or_else(|| LangError::plain("dft could not read a natural sample."))?;
            Ok((Real::from_int(&int), Real::zero()))
        }
        Value::Integer(i) => Ok((Real::from_int(i), Real::zero())),
        Value::Real(r) => Ok((r.clone(), Real::zero())),
        Value::Complex(re, im) => Ok((re.clone(), im.clone())),
        other => Err(LangError::plain(format!(
            "DSP builtins expect numeric/complex elements, but got '{}'.",
            other.kind_name()
        ))),
    }
}

/// The N distinct roots of unity e^(-2*pi*i*k/N), in order, at `digits` decimal places.
///
/// Each angle is reduced to lowest terms (gcd), so the rational roots the reference's
/// special-angle table covers are answered exactly; every other root goes through the
/// decimal sine/cosine series. The transform therefore keeps the precision it computes at
/// instead of a fixed number of digits.
fn roots_of_unity(n: usize, digits: usize) -> Vec<(Real, Real)> {
    let mut roots = Vec::with_capacity(n);
    for k in 0..n {
        let twice = 2 * k;
        let g = gcd_usize(twice, n);
        let num = (twice / g) as i64;
        let den = (n / g) as i64;
        let (sin, cos) = match special_angle(num, den, digits) {
            Some(pair) => pair,
            None => {
                let pi = Real::pi(digits);
                let angle = pi
                    .mul(&Real::from_i64(num))
                    .div(&Real::from_i64(den), digits)
                    .unwrap_or_else(Real::zero);
                decimal_sin_cos(&angle, digits)
            }
        };
        roots.push((cos, sin.neg()));
    }
    roots
}

fn gcd_usize(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

/// The exact sine/cosine of a rational multiple of pi in [0, 2*pi), for the 16 angles the
/// reference DSP kernel tabulates as rationals or square roots. Returns `None` for the rest.
fn special_angle(num: i64, den: i64, digits: usize) -> Option<(Real, Real)> {
    let half = || Real::from_str("0.5").expect("0.5 is a valid decimal");
    let sqrt2_half = || {
        Real::from_i64(2)
            .sqrt(digits)
            .div(&Real::from_i64(2), digits)
            .unwrap_or_else(Real::zero)
    };
    let sqrt3_half = || {
        Real::from_i64(3)
            .sqrt(digits)
            .div(&Real::from_i64(2), digits)
            .unwrap_or_else(Real::zero)
    };
    let pair = match (num, den) {
        (0, 1) => (Real::zero(), Real::from_i64(1)),
        (1, 6) => (half(), sqrt3_half()),
        (1, 4) => (sqrt2_half(), sqrt2_half()),
        (1, 3) => (sqrt3_half(), half()),
        (1, 2) => (Real::from_i64(1), Real::zero()),
        (2, 3) => (sqrt3_half(), half().neg()),
        (3, 4) => (sqrt2_half(), sqrt2_half().neg()),
        (5, 6) => (half(), sqrt3_half().neg()),
        (1, 1) => (Real::zero(), Real::from_i64(-1)),
        (7, 6) => (half().neg(), sqrt3_half().neg()),
        (5, 4) => (sqrt2_half().neg(), sqrt2_half().neg()),
        (4, 3) => (sqrt3_half().neg(), half().neg()),
        (3, 2) => (Real::from_i64(-1), Real::zero()),
        (5, 3) => (sqrt3_half().neg(), half()),
        (7, 4) => (sqrt2_half().neg(), sqrt2_half()),
        (11, 6) => (half().neg(), sqrt3_half()),
        _ => return None,
    };
    Some(pair)
}

/// Sine and cosine of an angle in [0, 2*pi) by argument reduction to [0, pi/2] and the
/// decimal Taylor series, at `digits` published places with a ten-place guard.
fn decimal_sin_cos(angle: &Real, digits: usize) -> (Real, Real) {
    let guard = digits + 10;
    let pi = Real::pi(digits);
    let two_pi = pi.mul(&Real::from_i64(2));
    let half_pi = pi.div(&Real::from_i64(2), guard).unwrap_or_else(Real::zero);
    let three_half_pi = pi.add(&half_pi);
    let mut x = angle.clone();
    if x.cmp(&Real::zero()) == std::cmp::Ordering::Less {
        x = x.add(&two_pi);
    }
    if x.cmp(&two_pi) != std::cmp::Ordering::Less {
        x = x.sub(&two_pi);
    }
    let (reduced, sin_negative, cos_negative) = if x.cmp(&half_pi) != std::cmp::Ordering::Greater {
        (x.clone(), false, false)
    } else if x.cmp(&pi) != std::cmp::Ordering::Greater {
        (pi.sub(&x), false, true)
    } else if x.cmp(&three_half_pi) != std::cmp::Ordering::Greater {
        (x.sub(&pi), true, true)
    } else {
        (two_pi.sub(&x), true, false)
    };
    let sin = sin_taylor(&reduced, guard);
    let cos = cos_taylor(&reduced, guard);
    let sin = if sin_negative { sin.neg() } else { sin };
    let cos = if cos_negative { cos.neg() } else { cos };
    (sin.mark_inexact(), cos.mark_inexact())
}

fn sin_taylor(x: &Real, guard: usize) -> Real {
    let x2 = x.mul(x);
    let mut term = x.clone();
    let mut sum = x.clone();
    let threshold = decimal_guard(guard);
    let mut k: i64 = 1;
    loop {
        let denom = (2 * k) * (2 * k + 1);
        term = term
            .mul(&x2)
            .neg()
            .div(&Real::from_i64(denom), guard)
            .unwrap_or_else(Real::zero);
        sum = sum.add(&term);
        if term.abs().cmp(&threshold) == std::cmp::Ordering::Less || k >= 100_000 {
            break;
        }
        k += 1;
    }
    sum
}

fn cos_taylor(x: &Real, guard: usize) -> Real {
    let x2 = x.mul(x);
    let mut term = Real::from_i64(1);
    let mut sum = Real::from_i64(1);
    let threshold = decimal_guard(guard);
    let mut k: i64 = 1;
    loop {
        let denom = (2 * k - 1) * (2 * k);
        term = term
            .mul(&x2)
            .neg()
            .div(&Real::from_i64(denom), guard)
            .unwrap_or_else(Real::zero);
        sum = sum.add(&term);
        if term.abs().cmp(&threshold) == std::cmp::Ordering::Less || k >= 100_000 {
            break;
        }
        k += 1;
    }
    sum
}

/// `10^-(guard+1)`, the series' termination threshold.
fn decimal_guard(guard: usize) -> Real {
    let text = format!("0.{}1", "0".repeat(guard));
    Real::from_str(&text).unwrap_or_else(Real::zero)
}

/// Integrates a term of the form A/(x - r).
fn integrate_simple_fraction(term: &Sym, var: &str) -> Option<Sym> {
    if lovelace_symbolics::cancel::check() {
        return None;
    }
    if let Some((num, den)) = quotient_parts(term) {
        let a = match num {
            Sym::Num(n) => Sym::Num(n),
            other => other,
        };
        return Some(Sym::mul(a, Sym::Fun("log".to_string(), vec![den])));
    }
    integrate_basic(term, var)
}

/// The linear decomposition of `u` in `var`: Some((a, b)) when u = a*var + b with `a` free of
/// the variable. None for any higher degree (exp(x^2) is not a table entry) and for a coefficient
/// that still mentions the variable.
fn linear_parts(u: &Sym, var: &str) -> Option<(Sym, Sym)> {
    let terms = match u {
        Sym::Add(ts) => ts.clone(),
        single => vec![single.clone()],
    };
    let mut coefficient: Option<Sym> = None;
    let mut constant = Sym::num(0);
    for t in terms {
        match t.degree_in(var) {
            0 => constant = Sym::add(constant, t),
            1 => {
                let part = divide_var(&t, var)?;
                coefficient = Some(match coefficient {
                    Some(existing) => Sym::add(existing, part),
                    None => part,
                });
            }
            _ => return None,
        }
    }
    let a = coefficient?;
    if a.contains(var) || a == Sym::num(0) {
        return None;
    }
    Some((a, constant))
}

/// Integrates the shapes the documentation exercises: powers, constant multiples and the
/// chain-rule case 2*x*f(x^2).
fn integrate_basic(e: &Sym, var: &str) -> Option<Sym> {
    // A cancelled antiderivative is not an answer; the evaluation discards the partial and reports
    // the structured Cancelled refusal (Lovelace.Symbolics/Calculus/Integrate.cs polls its walk).
    if lovelace_symbolics::cancel::check() {
        return None;
    }
    // A sum integrates term by term.
    if let Sym::Add(terms) = e {
        let mut acc = Sym::num(0);
        for t in terms {
            acc = Sym::add(acc, integrate_basic(t, var)?);
        }
        return Some(acc);
    }
    // Constants.
    if !e.contains(var) {
        return Some(Sym::mul(e.clone(), Sym::sym(var)));
    }
    // var^n
    if let Sym::Pow(base, exp) = e {
        if **base == Sym::sym(var) {
            if let Sym::Num(n) = exp.as_ref() {
                if n.denominator().to_decimal() == "1" {
                    if let Some(k) = n.to_i64() {
                        if k >= 0 {
                            let new_exp = Sym::num(k + 1);
                            let denom = Sym::num(k + 1);
                            return Some(Sym::mul(
                                Sym::pow(Sym::sym(var), new_exp),
                                Sym::pow(denom, Sym::num(-1)),
                            ));
                        }
                    }
                }
            }
        }
    }
    // 1/x integrates to log(x).
    if let Sym::Pow(base, exp) = e {
        if **base == Sym::sym(var) {
            if let Sym::Num(n) = exp.as_ref() {
                if n.to_string() == "-1" {
                    return Some(Sym::Fun("log".to_string(), vec![Sym::sym(var)]));
                }
            }
        }
    }
    // x integrates to x^2/2: the power rule at n = 1. The previous form multiplied the power by
    // the coefficient ZERO, so every term linear in the variable vanished from the antiderivative
    // (integrate(3*x^2 + 2*x + 1) answered x^3 + x instead of x^3 + x^2 + x).
    if *e == Sym::sym(var) {
        let half = Sym::pow(Sym::num(2), Sym::num(-1));
        return Some(Sym::mul(Sym::pow(Sym::sym(var), Sym::num(2)), half));
    }
    // sin, cos and exp of a LINEAR argument: the antiderivative is the same head over the same
    // argument, scaled by 1/a. This is the table's exp(3x) -> exp(3x)/3, sin(x) -> -cos(x) and
    // cos(x) -> sin(x) rows; a non-linear argument (exp(x^2)) still reaches the refusal.
    if let Sym::Fun(name, args) = e {
        if args.len() == 1 && matches!(name.as_str(), "sin" | "cos" | "exp") {
            if let Some((a, _b)) = linear_parts(&args[0], var) {
                let antiderivative = match name.as_str() {
                    "sin" => Sym::neg(Sym::Fun("cos".to_string(), vec![args[0].clone()])),
                    "cos" => Sym::Fun("sin".to_string(), vec![args[0].clone()]),
                    _ => Sym::Fun("exp".to_string(), vec![args[0].clone()]),
                };
                return Some(Sym::mul(antiderivative, Sym::pow(a, Sym::num(-1))));
            }
        }
    }
    // coefficient * f
    if let Sym::Mul(factors) = e {
        let mut constant = Sym::num(1);
        let mut rest: Vec<Sym> = Vec::new();
        for f in factors {
            if f.contains(var) {
                rest.push(f.clone());
            } else {
                constant = Sym::mul(constant, f.clone());
            }
        }
        if rest.len() == 1 {
            let inner = integrate_basic(&rest[0], var)?;
            return Some(Sym::mul(constant, inner));
        }
    }
    // x * exp(a*x + b): integration by parts, exp(u) * (x/a - 1/a^2). This is the table's
    // x*exp(x) -> exp(x)*(x - 1) row.
    if let Sym::Mul(factors) = e {
        if factors.len() == 2 {
            for (first, second) in [
                (factors[0].clone(), factors[1].clone()),
                (factors[1].clone(), factors[0].clone()),
            ] {
                if first != Sym::sym(var) {
                    continue;
                }
                if let Sym::Fun(name, args) = &second {
                    if name == "exp" && args.len() == 1 {
                        if let Some((a, _b)) = linear_parts(&args[0], var) {
                            let inverse = Sym::pow(a.clone(), Sym::num(-1));
                            let square = Sym::pow(a, Sym::num(-2));
                            let body = Sym::sub(Sym::mul(Sym::sym(var), inverse), square);
                            return Some(Sym::mul(second.clone(), body));
                        }
                    }
                }
            }
        }
    }
    // Chain-rule case: c * x * f(x^2) with c = 2 integrates to F(x^2).
    if let Sym::Mul(factors) = e {
        let mut constant_is_two = false;
        let mut has_var = false;
        let mut outer: Option<Sym> = None;
        let mut consumed = 0usize;
        for f in factors {
            match f {
                Sym::Num(n) if n.to_string() == "2" => {
                    constant_is_two = true;
                    consumed += 1;
                }
                Sym::Sym(s) if s == var => {
                    has_var = true;
                    consumed += 1;
                }
                Sym::Fun(name, args) if args.len() == 1 => {
                    if let Sym::Pow(base, exp) = &args[0] {
                        if base.as_ref() == &Sym::sym(var) && exp.as_ref() == &Sym::num(2) {
                            outer = match name.as_str() {
                                "cos" => Some(Sym::Fun("sin".to_string(), vec![args[0].clone()])),
                                "sin" => Some(Sym::neg(Sym::Fun(
                                    "cos".to_string(),
                                    vec![args[0].clone()],
                                ))),
                                "exp" => Some(Sym::Fun("exp".to_string(), vec![args[0].clone()])),
                                _ => None,
                            };
                            consumed += 1;
                        }
                    }
                }
                _ => {}
            }
        }
        if constant_is_two && has_var && consumed == factors.len() {
            if let Some(o) = outer {
                return Some(o);
            }
        }
        let _ = &mut outer;
    }
    None
}

#[derive(Clone, Copy, PartialEq)]
enum Side {
    Left,
    Right,
    Both,
}

/// True when every subterm of `e` has a value at the point: no denominator and no negative
/// power vanishes there. (2^(1/x))^x is the case that matters - the numeric tier would fold the
/// substituted (2^(1/0))^0 to 1, and 1 is not a value the expression has near the point.
fn substitution_is_defined(e: &Sym, var: &str, point: &Sym) -> bool {
    match e {
        Sym::Div(top, bottom) => {
            substitution_is_defined(top, var, point)
                && substitution_is_defined(bottom, var, point)
                && !vanishes_at(bottom, var, point)
        }
        Sym::Pow(base, exp) => {
            let negative_exponent = matches!(exp.as_ref(), Sym::Num(n) if n.is_negative());
            substitution_is_defined(base, var, point)
                && substitution_is_defined(exp, var, point)
                && !(negative_exponent && vanishes_at(base, var, point))
        }
        Sym::Add(items) | Sym::Mul(items) => {
            items.iter().all(|t| substitution_is_defined(t, var, point))
        }
        Sym::Fun(_, args) => args.iter().all(|t| substitution_is_defined(t, var, point)),
        Sym::Rel(_, a, b) => {
            substitution_is_defined(a, var, point) && substitution_is_defined(b, var, point)
        }
        _ => true,
    }
}

/// True when the expression is EXACTLY zero at the point.
fn vanishes_at(e: &Sym, var: &str, point: &Sym) -> bool {
    let substituted = lovelace_symbolics::substitute(e, var, point);
    if substituted.contains(var) || has_undefined_subterm(&substituted) {
        return false;
    }
    matches!(
        crate::numeric::evaluate(&substituted, 30),
        Ok(crate::numeric::Num::Exact(value)) if value.is_zero()
    )
}

/// True when a substituted expression carries a term with NO value at the point: an infinity
/// node, or a negative power of a zero base (1/0). The direct-substitution step must not read a
/// value off such a tree.
fn has_undefined_subterm(e: &Sym) -> bool {
    match e {
        Sym::Fun(name, args) => name == "inf" || args.iter().any(has_undefined_subterm),
        Sym::Pow(base, exp) => {
            let zero_base = matches!(base.as_ref(), Sym::Num(n) if n.is_zero());
            let negative_exponent = matches!(exp.as_ref(), Sym::Num(n) if n.is_negative());
            (zero_base && negative_exponent)
                || has_undefined_subterm(base)
                || has_undefined_subterm(exp)
        }
        Sym::Add(items) | Sym::Mul(items) => items.iter().any(has_undefined_subterm),
        Sym::Div(a, b) => has_undefined_subterm(a) || has_undefined_subterm(b),
        Sym::Rel(_, a, b) => has_undefined_subterm(a) || has_undefined_subterm(b),
        _ => false,
    }
}

/// True when the expression is the negated infinity node (-inf), the point `limit(f, x, -inf)`
/// names.
fn is_negated_infinity(e: &Sym) -> bool {
    let infinity = Sym::Fun("inf".to_string(), Vec::new());
    match e {
        // -inf is the product -1*inf; a sum of the infinity node is not this point.
        Sym::Mul(factors) => factors.iter().any(|f| *f == infinity),
        Sym::Fun(name, args) => name == "inf" && args.is_empty(),
        _ => *e == infinity,
    }
}

/// The exact FINITE limit of an expression at the point, or None when there is no finite exact
/// limit (a pole, an oscillation, or a shape the series cannot decide). Used by the
/// indeterminate-exponential rule, never published on its own.
fn limit_at(f: &Sym, var: &str, at_infinity: bool) -> Option<Sym> {
    // A constant is its own limit; this is what keeps a parameter exact in (1 + x)^(a/x) -> e^a.
    if !f.contains(var) {
        return Some(f.clone());
    }
    if !at_infinity {
        let substituted = lovelace_symbolics::substitute(f, var, &Sym::num(0));
        if !substituted.contains(var) {
            if let Ok(crate::numeric::Num::Exact(value)) =
                crate::numeric::evaluate(&substituted, 30)
            {
                return Some(Sym::Num(value));
            }
        }
    }
    let series = if at_infinity {
        let substituted =
            lovelace_symbolics::substitute(f, var, &Sym::pow(Sym::sym("t"), Sym::num(-1)));
        lovelace_symbolics::series(&substituted, "t", 6)
    } else {
        lovelace_symbolics::series(f, var, 6)
    };
    let ser = series?;
    let (min_exp, coeff) = ser.first()?.clone();
    if min_exp > 0 {
        return Some(Sym::num(0));
    }
    if min_exp == 0 {
        return Some(Sym::Num(coeff));
    }
    None
}

/// Cancels one factor of `var` against one reciprocal `var^-1` in a product, and returns the
/// remaining product. Only used for a limit cofactor, where the cancellation is what the limit
/// needs (never a rewrite the kernel would publish as an identity).
fn cancel_var_reciprocal(e: &Sym, var: &str) -> Option<Sym> {
    let factors = match e {
        Sym::Mul(f) => f.clone(),
        _ => return None,
    };
    let symbol = Sym::sym(var);
    let reciprocal = Sym::pow(symbol.clone(), Sym::num(-1));
    let mut rest: Vec<Sym> = Vec::new();
    let mut has_reciprocal = false;
    let mut has_symbol = false;
    for f in factors {
        if f == reciprocal {
            has_reciprocal = true;
            continue;
        }
        if f == symbol && !has_symbol {
            has_symbol = true;
            continue;
        }
        rest.push(f);
    }
    if !has_reciprocal || !has_symbol {
        return None;
    }
    let mut acc = Sym::num(1);
    for f in rest {
        acc = Sym::mul(acc, f);
    }
    Some(acc)
}

/// True when the expression is undecided at the point in EVERY sense: no finite value and no
/// pole. A base that grows without bound is not undecided (x + 1 at infinity), while 2^(1/x)
/// oscillates between 0 and +infinity next to zero and has no limit at all.
fn limit_is_undecided(f: &Sym, var: &str, at_infinity: bool) -> bool {
    if limit_at(f, var, at_infinity).is_some() {
        return false;
    }
    let series = if at_infinity {
        let substituted =
            lovelace_symbolics::substitute(f, var, &Sym::pow(Sym::sym("t"), Sym::num(-1)));
        lovelace_symbolics::series(&substituted, "t", 6)
    } else {
        lovelace_symbolics::series(f, var, 6)
    };
    match series {
        Some(entries) => entries.is_empty(),
        None => true,
    }
}

/// The limit of the indeterminate exponential shape `(1 + u)^v`, or None when the expression is
/// not that shape. The answer is exp(lim v*u); a cofactor with no finite limit is the typed
/// Unevaluated the reference publishes for `limit((1 + 1/x)^(x^2), x, inf)`.
fn exponential_family_limit(
    base: &Sym,
    exponent: &Sym,
    var: &str,
    at_infinity: bool,
) -> Option<Value> {
    // The exponent must tend to +-infinity; a finite exponent is not this family (and a finite
    // power is decided by substitution or by the series).
    if limit_at(exponent, var, at_infinity).is_some() {
        return None;
    }
    // The base must tend to ONE: u = base - 1 tends to zero. A power whose base tends somewhere
    // else has NO value at the point either, and the reference names that shape explicitly:
    // limit((2 + x)^(1/x), x, 0) is not e^(lim u*v).
    let offset = Sym::sub(base.clone(), Sym::num(1));
    let offset_limit = match limit_at(&offset, var, at_infinity) {
        Some(value) => value,
        None => return None,
    };
    if offset_limit != Sym::num(0) {
        let diag = diagnostic(
            "limit.unevaluated",
            "UnsupportedOperation",
            "an exponent has no value at the point and the base does not substitute to 1, so the limit is not e^(lim u*v)",
        );
        return Some(limit_record(
            "Unevaluated",
            Value::Void,
            Value::Void,
            Value::Void,
            Vec::new(),
            Value::Void,
            Vec::new(),
            Vec::new(),
            Value::Text("None".to_string()),
            vec![diag],
        ));
    }
    let product = Sym::mul(exponent.clone(), offset.clone());
    // The cofactor is a RATIO (x * a/x -> a) and the product algebra deliberately keeps x*x^-1
    // (cancelling needs a nonzero assumption). The limit only needs the cancelled form, so try
    // the exact quotient cancellation first and the reciprocal-pair cancellation after it.
    let cancelled =
        lovelace_symbolics::cancel(&product).or_else(|| cancel_var_reciprocal(&product, var));
    let cofactor = match &cancelled {
        Some(simplified) => limit_at(simplified, var, at_infinity),
        None => limit_at(&product, var, at_infinity),
    };
    match cofactor {
        Some(value) => {
            let answer = Sym::pow(Sym::Fun("e".to_string(), Vec::new()), value);
            // A closed form that is not a rational constant is ALGEBRAICALLY exact, not plain
            // exact: the reference publishes AlgebraicExact for e, e^2 and e^(3/2).
            Some(limit_record(
                "Value",
                Value::Boolean(true),
                Value::Symbolic(answer),
                Value::Void,
                Vec::new(),
                Value::Void,
                Vec::new(),
                Vec::new(),
                Value::Text("AlgebraicExact".to_string()),
                Vec::new(),
            ))
        }
        None => {
            let diag = diagnostic(
                "limit.unevaluated",
                "UnsupportedOperation",
                "the indeterminate factor's cofactor has no limit at the point",
            );
            Some(limit_record(
                "Unevaluated",
                Value::Void,
                Value::Void,
                Value::Void,
                Vec::new(),
                Value::Void,
                Vec::new(),
                Vec::new(),
                Value::Text("None".to_string()),
                vec![diag],
            ))
        }
    }
}

fn limit_result(f: &Sym, var: &str, point: &Sym, at_infinity: bool, side: Side) -> Value {
    use lovelace_symbolics::Series;
    // Step 1 of the reference's LimitCore: DIRECT SUBSTITUTION. A value at the point IS the limit
    // when the function is defined there — limit(abs(x), x, 0) = 0 and limit(sqrt(x), x, 0) = 0 are
    // the value at 0, not a series coefficient. Only an EXACT value is taken here: 0/0 must not fold
    // (the numeric evaluator refuses the division, so the series path still decides the
    // indeterminate shapes) and a transcendental substitution stays on the series path, which is
    // where the documented answers are computed.
    if !at_infinity {
        let substituted = lovelace_symbolics::substitute(f, var, point);
        // A substitution that leaves an undefined subterm (1/0, an infinity node) is not a
        // VALUE at the point: (2^(1/x))^x substitutes to (2^(1/0))^0, which the numeric tier
        // would fold to 1, and 1 is not the limit of an expression that has none.
        if !substituted.contains(var)
            && substitution_is_defined(f, var, point)
            && !has_undefined_subterm(&substituted)
        {
            if let Ok(number) = crate::numeric::evaluate(&substituted, 30) {
                let exact = match &number {
                    crate::numeric::Num::Exact(value) => Some(value.clone()),
                    // sqrt(0) crosses the numeric tier as a Real that is EXACTLY zero; the value
                    // at the point is still 0, and the reference publishes it as one.
                    crate::numeric::Num::Real(value) if value.is_zero() => {
                        Some(lovelace_symbolics::Rat::zero())
                    }
                    crate::numeric::Num::Complex(value)
                        if value.re().is_zero() && value.im().is_zero() =>
                    {
                        Some(lovelace_symbolics::Rat::zero())
                    }
                    _ => None,
                };
                if let Some(value) = exact {
                    return limit_value_record(Value::Symbolic(Sym::Num(value)));
                }
            }
        }
    }
    // Everything below expands AROUND the point: shifting the variable by the point makes the
    // expansion point zero again (x -> point + x), which is what the series engine takes. For the
    // origin the shift is the identity, so the documented answers are unchanged, and a limit at a
    // finite point is the limit of the shifted expression at zero (limit(x^2 + 1, x, 2) = 5).
    let shifted = if at_infinity {
        f.clone()
    } else {
        lovelace_symbolics::substitute(f, var, &Sym::add(point.clone(), Sym::sym(var)))
    };
    // THE INDETERMINATE EXPONENTIAL FAMILY: (1 + u)^v with u -> 0 and v -> +-infinity. The value
    // at the point does not exist, so substitution is not a proof here - but the limit IS
    // exp(lim v*u): (1 + 1/x)^x -> e, (1 + 2/x)^x -> e^2, (1 + 3/x)^(x/2) -> e^(3/2), and a
    // parameter stays exact ((1 + x)^(a/x) -> e^a). The shape is refused when the cofactor v*u has
    // no finite limit ((1 + 1/x)^(x^2) -> +oo).
    if let Sym::Pow(base, exponent) = &shifted {
        if let Some(value) = exponential_family_limit(base, exponent, var, at_infinity) {
            return value;
        }
        // A POWER whose BASE has no limit at the point has no limit either: 2^(1/x) oscillates
        // between 0 and +infinity next to 0, so (2^(1/x))^x has no value there. The reference
        // refuses; the series must not decide a shape whose base does not converge.
        if limit_is_undecided(base, var, at_infinity) {
            let diag = diagnostic(
                "limit.unevaluated",
                "UnsupportedOperation",
                "coefficient does not evaluate at the point",
            );
            return limit_record(
                "Unevaluated",
                Value::Void,
                Value::Void,
                Value::Void,
                Vec::new(),
                Value::Void,
                Vec::new(),
                Vec::new(),
                Value::Text("None".to_string()),
                vec![diag],
            );
        }
    }
    let series: Option<Series> = if at_infinity {
        let t = "t";
        let substituted =
            lovelace_symbolics::substitute(f, var, &Sym::pow(Sym::sym(t), Sym::num(-1)));
        lovelace_symbolics::series(&substituted, t, 8)
    } else {
        lovelace_symbolics::series(&shifted, var, 8)
    };
    let ser: Series = match series {
        Some(s) => s,
        None => Vec::new(),
    };
    let empty = Value::Void;
    if ser.is_empty() {
        // The reference distinguishes the two reasons a series cannot decide the limit: a leading
        // coefficient that is still symbolic (a/x at 0), and a coefficient that exists but does not
        // evaluate at the point (sin(x) at infinity).
        let mut symbols = Vec::new();
        collect_all_symbols(f, &mut symbols);
        let message = if symbols.iter().any(|symbol| symbol != var) {
            "leading coefficient is symbolic"
        } else {
            "coefficient does not evaluate at the point"
        };
        let diag = diagnostic("limit.unevaluated", "UnsupportedOperation", message);
        return limit_record(
            "Unevaluated",
            empty.clone(),
            empty.clone(),
            empty.clone(),
            Vec::new(),
            empty.clone(),
            Vec::new(),
            Vec::new(),
            Value::Text("None".to_string()),
            vec![diag],
        );
    }
    let (min_exp, coeff) = ser[0].clone();
    let coeff_value = Value::Symbolic(Sym::Num(coeff.clone()));
    if min_exp > 0 {
        return limit_value_record(Value::Symbolic(Sym::num(0)));
    }
    if min_exp == 0 {
        return limit_value_record(coeff_value);
    }
    // A pole: the sign depends on the parity of the pole order.
    let odd = min_exp % 2 != 0;
    let positive = !coeff.is_negative();
    let inf_text = |positive_side: bool| -> Value {
        let up = positive == positive_side;
        let inf = Sym::Fun("inf".to_string(), Vec::new());
        Value::Symbolic(if up { inf } else { Sym::neg(inf) })
    };
    match side {
        Side::Both => {
            if !odd {
                return limit_record(
                    if positive {
                        "PlusInfinity"
                    } else {
                        "MinusInfinity"
                    },
                    Value::Boolean(true),
                    Value::Symbolic(if positive {
                        Sym::Fun("inf".to_string(), Vec::new())
                    } else {
                        Sym::neg(Sym::Fun("inf".to_string(), Vec::new()))
                    }),
                    empty.clone(),
                    Vec::new(),
                    empty.clone(),
                    Vec::new(),
                    Vec::new(),
                    Value::Text("Exact".to_string()),
                    Vec::new(),
                );
            }
            limit_record(
                "DoesNotExist",
                Value::Boolean(false),
                empty.clone(),
                inf_text(false),
                vec![Value::Text(format!("{} < 0", var))],
                inf_text(true),
                vec![Value::Text(format!("{} > 0", var))],
                Vec::new(),
                Value::Text("Exact".to_string()),
                Vec::new(),
            )
        }
        Side::Right => limit_record(
            if positive {
                "PlusInfinity"
            } else {
                "MinusInfinity"
            },
            Value::Boolean(true),
            inf_text(true),
            empty.clone(),
            Vec::new(),
            empty.clone(),
            Vec::new(),
            Vec::new(),
            Value::Text("Exact".to_string()),
            Vec::new(),
        ),
        Side::Left => limit_record(
            if (positive && odd) || (!positive && !odd) {
                "MinusInfinity"
            } else {
                "PlusInfinity"
            },
            Value::Boolean(true),
            inf_text(false),
            empty.clone(),
            Vec::new(),
            empty.clone(),
            Vec::new(),
            Vec::new(),
            Value::Text("Exact".to_string()),
            Vec::new(),
        ),
    }
}

fn limit_value_record(value: Value) -> Value {
    limit_record(
        "Value",
        Value::Boolean(true),
        value,
        Value::Void,
        Vec::new(),
        Value::Void,
        Vec::new(),
        Vec::new(),
        Value::Text("Exact".to_string()),
        Vec::new(),
    )
}

fn limit_record(
    status: &str,
    exists: Value,
    value: Value,
    left: Value,
    left_conditions: Vec<Value>,
    right: Value,
    right_conditions: Vec<Value>,
    conditions: Vec<Value>,
    exactness: Value,
    diagnostics: Vec<Value>,
) -> Value {
    let exactness_text = match exactness {
        Value::Text(t) => t,
        other => other.display(),
    };
    Value::Record(
        "LimitResult".to_string(),
        vec![
            ("status".to_string(), enum_value("LimitStatus", status)),
            ("exists".to_string(), exists),
            ("value".to_string(), value),
            ("left".to_string(), left),
            (
                "left_conditions".to_string(),
                Value::Vector(left_conditions),
            ),
            ("right".to_string(), right),
            (
                "right_conditions".to_string(),
                Value::Vector(right_conditions),
            ),
            ("conditions".to_string(), Value::Vector(conditions)),
            (
                "exactness".to_string(),
                enum_value("SolutionExactness", &exactness_text),
            ),
            ("diagnostics".to_string(), Value::Vector(diagnostics)),
        ],
    )
}

/// Solves a*x + b = 0 when a and b may contain symbols.
fn solve_linear_symbolic(e: &Sym, var: &str) -> Option<(Sym, Sym)> {
    if lovelace_symbolics::cancel::check() {
        return None;
    }
    let terms = match e {
        Sym::Add(ts) => ts.clone(),
        single => vec![single.clone()],
    };
    let mut coefficient: Option<Sym> = None;
    let mut constant = Sym::num(0);
    for t in terms {
        let degree = t.degree_in(var);
        match degree {
            0 => constant = Sym::add(constant, t),
            1 => {
                let part = divide_var(&t, var)?;
                coefficient = Some(match coefficient {
                    Some(existing) => Sym::add(existing, part),
                    None => part,
                });
            }
            _ => return None,
        }
    }
    let a = coefficient?;
    // The reference's TryCoefficients guard: a coefficient of a linear equation in x is free of x
    // by definition. An x that leaks into c0 or c1 (x*log(x), -sin(x)) is not a coefficient at
    // all — root = -c0/c1 would be a circular answer, not a value.
    if constant.contains(var) || a.contains(var) {
        return None;
    }
    // x = -b/a
    let value = Sym::mul(Sym::neg(constant), Sym::pow(a.clone(), Sym::num(-1)));
    Some((value, a))
}

/// Removes one factor of the variable from a degree-one term.
fn divide_var(t: &Sym, var: &str) -> Option<Sym> {
    match t {
        Sym::Sym(s) if s == var => Some(Sym::num(1)),
        Sym::Mul(factors) => {
            let mut rest = Vec::new();
            let mut removed = false;
            for f in factors {
                if !removed && matches!(f, Sym::Sym(s) if s == var) {
                    removed = true;
                    continue;
                }
                rest.push(f.clone());
            }
            if !removed {
                return None;
            }
            Some(match rest.len() {
                0 => Sym::num(1),
                1 => rest.pop().unwrap(),
                _ => Sym::Mul(rest),
            })
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Inverse-branch solving (round 31)
//
// A port of the reference's Solvers.SolveElementary / SolveInverse / SolvePeriodic /
// SolveExponentialFamily: an equation whose solved variable appears only through an invertible
// head h(u) is answered by inverting h, solving u == h^-1(c) recursively, and VERIFYING every
// candidate against the original equation. Periodic heads publish parametric families instead of a
// finite list, and a value that still mentions the solved variable is a circular answer (not a
// root): it is dropped, and a set that lost one is never claimed complete.
// ---------------------------------------------------------------------------

/// One branch's outcome before it is projected onto the SolveResult record.
enum SolveBranch {
    /// Values (each with its conditions) and/or parametric families.
    Set {
        solutions: Vec<(Sym, Vec<Sym>)>,
        families: Vec<Value>,
    },
    /// No solutions, carrying the reference's own reason.
    Empty(String),
    /// Unevaluated, carrying the reference's own reason.
    Unevaluated(String),
}

fn pi_constant() -> Sym {
    Sym::Fun("pi".to_string(), Vec::new())
}

/// The imaginary unit as the reference represents it: a named constant, never a user symbol, so a
/// family template's canonical form is (i) and its free_symbols do not include i.
fn imaginary_constant() -> Sym {
    Sym::Fun("i".to_string(), Vec::new())
}

/// The numeric value of a constant expression at the solver's 40-digit working precision, or None
/// when the expression is symbolic. The reference's Evaluation.ConstantToNum over the real tier.
fn constant_real(c: &Sym) -> Option<Real> {
    crate::numeric::evaluate(c, 40)
        .ok()
        .and_then(|num| num.to_real(40))
}

/// True when the expression mentions the solved variable as a free symbol. Structural on purpose:
/// the defining polynomial of a RootOf names x as that value's BOUND variable, so
/// rootof(x^4 - x^2 - 1, 0) is a value like any other, while log(x) offered as a solution of
/// exp(x) = x is a circular answer.
fn mentions_solved_variable(e: &Sym, var: &str) -> bool {
    match e {
        Sym::Fun(name, _) if name == "rootof" => false,
        Sym::Sym(s) => s == var,
        Sym::Num(_) | Sym::Real(_) => false,
        Sym::Add(v) | Sym::Mul(v) => v.iter().any(|x| mentions_solved_variable(x, var)),
        Sym::Pow(a, b) => mentions_solved_variable(a, var) || mentions_solved_variable(b, var),
        Sym::Fun(_, args) => args.iter().any(|a| mentions_solved_variable(a, var)),
        Sym::Div(a, b) => mentions_solved_variable(a, var) || mentions_solved_variable(b, var),
        Sym::Rel(_, a, b) => mentions_solved_variable(a, var) || mentions_solved_variable(b, var),
    }
}

/// Chooses a family parameter symbol not colliding with the equation's symbols (the reference's
/// FreshParameter: k, then k1, k2, ...).
fn fresh_parameter(c: &Sym, u: &Sym) -> String {
    let mut name = "k".to_string();
    let mut index = 1;
    while c.contains(&name) || u.contains(&name) {
        name = format!("k{index}");
        index += 1;
    }
    name
}

/// The total-function folds the reference applies when it CONSTRUCTS a function application
/// (Exprs.Function -> Evaluation.FoldFunction): the exact special points the family builders rely
/// on, so atan(0) in tan(x) = 0 is the constant 0 and the template is k*pi, not atan(0) + k*pi.
fn fold_unary(name: &str, arg: &Sym) -> Sym {
    if let Sym::Num(r) = arg {
        let fold = match name {
            "exp" if r.is_zero() => Some(Sym::num(1)),
            "sin" if r.is_zero() => Some(Sym::num(0)),
            "cos" if r.is_zero() => Some(Sym::num(1)),
            "tan" if r.is_zero() => Some(Sym::num(0)),
            "log" if r.is_one() => Some(Sym::num(0)),
            "atan" if r.is_zero() => Some(Sym::num(0)),
            "asin" if r.is_zero() => Some(Sym::num(0)),
            "acos" if r.is_one() => Some(Sym::num(0)),
            _ => None,
        };
        if let Some(folded) = fold {
            return folded;
        }
    }
    Sym::Fun(name.to_string(), vec![arg.clone()])
}

/// A sum of family-template terms in the published order, dropping the folded zero terms the
/// reference's Add would collect (a template is Add(0, k*pi) as k*pi, not 0 + k*pi).
fn add_terms(terms: Vec<Sym>) -> Sym {
    let kept: Vec<Sym> = terms.into_iter().filter(|t| *t != Sym::num(0)).collect();
    match kept.len() {
        0 => Sym::num(0),
        1 => kept.into_iter().next().unwrap(),
        _ => Sym::Add(kept),
    }
}

/// A SolutionFamily record. The template is built with the raw Add/Mul constructors in the
/// reference's published order: the family builder is the ONE place the term order is chosen, so a
/// canonicalizing rebuild would reorder a template the contract fixes.
fn solution_family(template: Sym, parameter: &str, period: Sym, conditions: Vec<Sym>) -> Value {
    Value::Record(
        "SolutionFamily".to_string(),
        vec![
            ("template".to_string(), Value::Symbolic(template)),
            (
                "parameter".to_string(),
                Value::Symbolic(Sym::sym(parameter)),
            ),
            ("period".to_string(), Value::Symbolic(period)),
            (
                "parameter_domain".to_string(),
                Value::Domain("integer".to_string()),
            ),
            (
                "conditions".to_string(),
                Value::Vector(conditions.into_iter().map(Value::Symbolic).collect()),
            ),
            (
                "exactness".to_string(),
                enum_value("SolutionExactness", "ParametricExact"),
            ),
        ],
    )
}

/// Solves h(u) = c for the periodic heads sin/cos/tan: the solutions are parametric families (sin
/// and cos have period 2*pi, tan pi), not a finite list.
fn solve_periodic(name: &str, u: &Sym, c: &Sym, var: &str, domain: &str) -> SolveBranch {
    if *u != Sym::sym(var) {
        return SolveBranch::Unevaluated(format!(
            "{name}(u) = c with composite u is not supported in v1."
        ));
    }
    let Some(cv) = constant_real(c) else {
        return SolveBranch::Unevaluated(format!(
            "{name}(x) = c with symbolic c is not supported in v1."
        ));
    };
    let k = fresh_parameter(c, u);
    let kexpr = Sym::sym(&k);
    let pi = pi_constant();
    let principal = fold_unary(
        match name {
            "sin" => "asin",
            "cos" => "acos",
            _ => "atan",
        },
        c,
    );
    let two_pi = Sym::mul(Sym::num(2), pi.clone());
    if name == "tan" {
        let template = add_terms(vec![principal, Sym::mul(kexpr, pi.clone())]);
        return SolveBranch::Set {
            solutions: Vec::new(),
            families: vec![solution_family(template, &k, pi, Vec::new())],
        };
    }
    // sin/cos: classify |c| against 1.
    let cmp_one = cv.abs().cmp(&Real::from_i64(1));
    if cmp_one == std::cmp::Ordering::Greater {
        return if domain == "real" {
            SolveBranch::Empty(format!("{name}(x) = c has no real solution for |c| > 1."))
        } else {
            SolveBranch::Unevaluated(format!(
                "{name}(x) = c has complex solutions for |c| > 1 that are not representable in v1."
            ))
        };
    }
    if cv.is_zero() {
        // sin(x) = 0 -> x = k*pi; cos(x) = 0 -> x = pi/2 + k*pi.
        let template = if name == "sin" {
            Sym::mul(kexpr, pi.clone())
        } else {
            Sym::Add(vec![
                Sym::mul(
                    Sym::rational(&Int::from_i64(1), &Nat::from_u64(2)),
                    pi.clone(),
                ),
                Sym::mul(kexpr, pi.clone()),
            ])
        };
        return SolveBranch::Set {
            solutions: Vec::new(),
            families: vec![solution_family(template, &k, pi, Vec::new())],
        };
    }
    if cmp_one == std::cmp::Ordering::Equal {
        // |c| = 1: the two branches merge into one family.
        let template = add_terms(vec![principal, Sym::mul(kexpr, two_pi.clone())]);
        return SolveBranch::Set {
            solutions: Vec::new(),
            families: vec![solution_family(template, &k, two_pi, Vec::new())],
        };
    }
    // |c| < 1: two families, one per branch.
    let first = add_terms(vec![
        principal.clone(),
        Sym::mul(kexpr.clone(), two_pi.clone()),
    ]);
    let second = if name == "sin" {
        add_terms(vec![
            pi.clone(),
            Sym::mul(Sym::num(-1), principal),
            Sym::mul(kexpr, two_pi.clone()),
        ])
    } else {
        add_terms(vec![
            Sym::mul(Sym::num(-1), principal),
            Sym::mul(kexpr, two_pi.clone()),
        ])
    };
    SolveBranch::Set {
        solutions: Vec::new(),
        families: vec![
            solution_family(first, &k, two_pi.clone(), Vec::new()),
            solution_family(second, &k, two_pi, Vec::new()),
        ],
    }
}

/// exp(u) = c over the complex field: the inverse is not one value but the family
/// log(c) + 2*pi*k*i (exp is periodic with period 2*pi*i), the same shape the periodic
/// trigonometric inverses publish. c != 0 is the hypothesis the family is valid under when c is
/// not numerically decidable.
fn solve_exponential_family(h: &Sym, u: &Sym, c: &Sym, var: &str) -> SolveBranch {
    if *u != Sym::sym(var) {
        return SolveBranch::Unevaluated(
            "exp(u) = c with composite u is not supported in v1: the inverse of exp over the complex \
             field is the family log(c) + 2*pi*k*i, which cannot be propagated through a composite \
             argument."
                .to_string(),
        );
    }
    let k = fresh_parameter(c, u);
    let i = imaginary_constant();
    let pi = pi_constant();
    let period = Sym::Mul(vec![Sym::num(2), pi.clone(), i.clone()]);
    // log(1) folds to 0 (the reference's FoldFunction), so exp(x) = 1 is 2*pi*k*i, not log(1) + ...
    let principal = fold_unary("log", c);
    let template = add_terms(vec![
        principal.clone(),
        Sym::Mul(vec![Sym::num(2), Sym::sym(&k), pi, i]),
    ]);
    let conditions = if constant_real(c).is_none() {
        vec![Sym::Rel(
            lovelace_symbolics::RelOp::Ne,
            Box::new(c.clone()),
            Box::new(Sym::num(0)),
        )]
    } else {
        Vec::new()
    };
    // The family is a claim about EVERY member: spot-check the principal member (k = 0) against
    // the ORIGINAL equation. A decidable non-zero residual means the template is not a solution at
    // all; an undecidable one (symbolic c) is kept.
    let principal_member = lovelace_symbolics::substitute(&template, &k, &Sym::num(0));
    if residual_is_zero(&Sym::sub(h.clone(), c.clone()), var, &principal_member) == Some(false) {
        return SolveBranch::Unevaluated(
            "the principal member log(c) of the inverse family does not satisfy the original \
             equation, so no solution set is claimed."
                .to_string(),
        );
    }
    SolveBranch::Set {
        solutions: Vec::new(),
        families: vec![solution_family(template, &k, period, conditions)],
    }
}

/// The n-th roots of unity as exact expressions in ascending k order (z_k = e^(2*pi*i*k/n)).
fn unity_roots(n: i64) -> Vec<Sym> {
    let half = Sym::rational(&Int::from_i64(1), &Nat::from_u64(2));
    match n {
        1 => vec![Sym::num(1)],
        2 => vec![Sym::num(1), Sym::num(-1)],
        4 => vec![
            Sym::num(1),
            imaginary_constant(),
            Sym::num(-1),
            Sym::neg(imaginary_constant()),
        ],
        3 => {
            let sqrt3_half = Sym::mul(Sym::pow(Sym::num(3), half.clone()), half);
            let minus_half = Sym::rational(&Int::from_i64(-1), &Nat::from_u64(2));
            let omega = Sym::add(
                minus_half.clone(),
                Sym::mul(sqrt3_half.clone(), imaginary_constant()),
            );
            let omega2 = Sym::sub(minus_half, Sym::mul(sqrt3_half, imaginary_constant()));
            vec![Sym::num(1), omega, omega2]
        }
        _ => {
            let two_pi = Sym::mul(Sym::num(2), pi_constant());
            let mut list = Vec::new();
            for step in 0..n.max(0) {
                if step == 0 {
                    list.push(Sym::num(1));
                    continue;
                }
                if n % 2 == 0 && step == n / 2 {
                    list.push(Sym::num(-1));
                    continue;
                }
                let angle = Sym::mul(
                    Sym::rational(&Int::from_i64(step), &Nat::from_u64(n as u64)),
                    two_pi.clone(),
                );
                list.push(Sym::Add(vec![
                    Sym::Fun("cos".to_string(), vec![angle.clone()]),
                    Sym::mul(
                        imaginary_constant(),
                        Sym::Fun("sin".to_string(), vec![angle]),
                    ),
                ]));
            }
            list
        }
    }
}

/// Inverts one head h and solves the branches it produces (the reference's SolveInverse).
fn solve_inverse(h: &Sym, c: &Sym, var: &str, domain: &str, depth: usize) -> Option<SolveBranch> {
    if depth > 8 {
        return Some(SolveBranch::Unevaluated(
            "inverse nesting budget exceeded.".to_string(),
        ));
    }
    let mut u: Option<Sym> = None;
    let mut inverses: Option<Vec<Sym>> = None;
    match h {
        Sym::Fun(name, args) if args.len() == 1 => {
            u = Some(args[0].clone());
            match name.as_str() {
                "sin" | "cos" | "tan" => {
                    return Some(solve_periodic(name, &args[0], c, var, domain));
                }
                "exp" => {
                    // exp(u) = 0 has no solution, and over the reals it needs c > 0.
                    if let Some(cv) = constant_real(c) {
                        if cv.is_zero() {
                            return Some(SolveBranch::Empty(
                                "exp(u) = 0 has no solution.".to_string(),
                            ));
                        }
                        if domain == "real" && cv.cmp(&Real::zero()) == std::cmp::Ordering::Less {
                            return Some(SolveBranch::Empty(
                                "exp(u) = c has no real solution for c < 0.".to_string(),
                            ));
                        }
                    }
                    // Over the complex field exp's inverse is the infinite family, not one value.
                    if domain == "complex" {
                        return Some(solve_exponential_family(h, &args[0], c, var));
                    }
                    inverses = Some(vec![Sym::Fun("log".to_string(), vec![c.clone()])]);
                }
                "log" => inverses = Some(vec![Sym::Fun("exp".to_string(), vec![c.clone()])]),
                // The reference represents sqrt(u) as the half power u^(1/2), so its inverse is
                // u = c^2 (the principal branch only: the exponent is not an integer).
                "sqrt" => {
                    inverses = Some(vec![Sym::pow(c.clone(), Sym::num(2))]);
                }
                _ => return None,
            }
        }
        Sym::Pow(base, exp) => {
            let n = match exp.as_ref() {
                Sym::Num(n) if !n.is_one() => n.clone(),
                _ => return None,
            };
            if n.is_zero() {
                return None;
            }
            u = Some((**base).clone());
            let principal = Sym::pow(
                c.clone(),
                Sym::Num(
                    lovelace_symbolics::Rat::one()
                        .div(&n)
                        .unwrap_or_else(lovelace_symbolics::Rat::one),
                ),
            );
            // u^n = c has exactly n solutions over the complex field; every root of unity is
            // generated whenever the exponent is a small positive integer.
            if n.is_integer() {
                if let Some(count) = n.to_i64() {
                    if (1..=16).contains(&count) {
                        inverses = Some(
                            unity_roots(count)
                                .into_iter()
                                .map(|z| Sym::mul(principal.clone(), z))
                                .collect(),
                        );
                    }
                }
            }
            if inverses.is_none() {
                inverses = Some(vec![principal]);
            }
        }
        _ => return None,
    }
    let u = u?;
    let inverses = inverses?;

    let mut solutions: Vec<(Sym, Vec<Sym>)> = Vec::new();
    let mut families: Vec<Value> = Vec::new();
    for inverse in inverses {
        let inner = Sym::sub(u.clone(), inverse);
        match solve_inner(&inner, var, domain, depth + 1) {
            SolveBranch::Empty(_) => continue,
            SolveBranch::Set {
                solutions: inner_solutions,
                families: inner_families,
            } => {
                solutions.extend(inner_solutions);
                families.extend(inner_families);
            }
            SolveBranch::Unevaluated(message) => return Some(SolveBranch::Unevaluated(message)),
        }
    }
    if solutions.is_empty() && families.is_empty() {
        return Some(SolveBranch::Empty(
            "no branch produced a solution.".to_string(),
        ));
    }
    // The branches above are NECESSARY-condition generators, not equivalences: sqrt(x) = -2
    // inverts to x = 4 and sqrt(4) = +2. Every candidate is verified against the ORIGINAL h - c,
    // not the transformed equation the recursion solved.
    let original = Sym::sub(h.clone(), c.clone());
    let mut rejected = 0usize;
    let mut verified: Vec<(Sym, Vec<Sym>)> = Vec::new();
    for (value, conditions) in solutions {
        match residual_is_zero(&original, var, &value) {
            Some(false) => rejected += 1,
            _ => verified.push((value, conditions)),
        }
    }
    if verified.is_empty() && families.is_empty() {
        return Some(SolveBranch::Unevaluated(format!(
            "every candidate from the inverse branch(es) fails substitution into the original \
             equation ({rejected} rejected): the inverse step is not invertible on this branch, so \
             no solution set is claimed."
        )));
    }
    Some(SolveBranch::Set {
        solutions: verified,
        families,
    })
}

/// The reference's SolveElementary: a bare invertible head h(u) = 0, and h(u) + c = 0 with c a
/// folded constant.
fn solve_elementary(f: &Sym, var: &str, domain: &str, depth: usize) -> Option<SolveBranch> {
    if matches!(f, Sym::Fun(..) | Sym::Pow(..)) {
        if let Some(branch) = solve_inverse(f, &Sym::num(0), var, domain, depth) {
            return Some(branch);
        }
    }
    if let Sym::Add(terms) = f {
        for term in terms {
            let rhs = match term {
                Sym::Num(n) => Sym::Num(n.neg()),
                // A term written as -h(u) is the reference's second accepted shape.
                Sym::Mul(factors) if factors.len() == 2 && factors[0] == Sym::num(-1) => {
                    factors[1].clone()
                }
                _ => continue,
            };
            let u = Sym::sub(f.clone(), term.clone());
            if !matches!(u, Sym::Fun(..) | Sym::Pow(..)) {
                continue;
            }
            if let Some(branch) = solve_inverse(&u, &rhs, var, domain, depth) {
                return Some(branch);
            }
        }
    }
    None
}

/// The ONE residual predicate: true when the equation is zero at the candidate, false when it is
/// provably not zero, None when it is not numerically decidable. The inverse-branch gate drops a
/// candidate only on a decidable false and keeps it otherwise.
pub(crate) fn residual_is_zero(equation: &Sym, var: &str, value: &Sym) -> Option<bool> {
    let at = lovelace_symbolics::substitute(equation, var, value);
    if let Sym::Num(n) = &at {
        return Some(n.is_zero());
    }
    let tolerance = Real::from_str(&format!("0.{}1", "0".repeat(24))).unwrap_or_else(Real::zero);
    match crate::numeric::evaluate(&at, 40) {
        Ok(num) => match num {
            crate::numeric::Num::Exact(r) => Some(r.is_zero()),
            crate::numeric::Num::Real(r) => {
                Some(r.abs().cmp(&tolerance) == std::cmp::Ordering::Less)
            }
            crate::numeric::Num::Complex(z) => {
                Some(z.magnitude(40).cmp(&tolerance) == std::cmp::Ordering::Less)
            }
        },
        Err(_) => None,
    }
}

/// One recursive solve of the inner equation u == h^-1(c): polynomial first, then a linear
/// equation with variable-free coefficients, then the elementary inverses. Anything else is the
/// reference's structured Unevaluated.
fn solve_inner(equation: &Sym, var: &str, domain: &str, depth: usize) -> SolveBranch {
    if depth > 8 {
        return SolveBranch::Unevaluated("inverse nesting budget exceeded.".to_string());
    }
    if let Some(coeffs) = lovelace_symbolics::poly_coeffs(equation, var) {
        let outcome = filter_real_domain(
            lovelace_symbolics::solve_polynomial_named(&coeffs, var),
            domain,
        );
        use lovelace_symbolics::SolveOutcome;
        return match outcome {
            SolveOutcome::Roots(roots) => SolveBranch::Set {
                solutions: roots
                    .into_iter()
                    .map(|r| (lovelace_symbolics::expr_from_rat(&r.value), Vec::new()))
                    .collect(),
                families: Vec::new(),
            },
            SolveOutcome::RootExpressions(expressions) => SolveBranch::Set {
                solutions: expressions.into_iter().map(|e| (e, Vec::new())).collect(),
                families: Vec::new(),
            },
            SolveOutcome::RootExpressionsCount(expressions, _) => SolveBranch::Set {
                solutions: expressions.into_iter().map(|e| (e, Vec::new())).collect(),
                families: Vec::new(),
            },
            SolveOutcome::NoSolutions | SolveOutcome::NoRealSolutions => {
                SolveBranch::Empty("no solutions".to_string())
            }
            _ => SolveBranch::Unevaluated("No solver for this structure.".to_string()),
        };
    }
    if let Some((value, coefficient)) = solve_linear_symbolic(equation, var) {
        if !mentions_solved_variable(&value, var) {
            let condition = Sym::Rel(
                lovelace_symbolics::RelOp::Ne,
                Box::new(coefficient),
                Box::new(Sym::num(0)),
            );
            return SolveBranch::Set {
                solutions: vec![(value, vec![condition])],
                families: Vec::new(),
            };
        }
    }
    if let Some(branch) = solve_elementary(equation, var, domain, depth + 1) {
        return branch;
    }
    SolveBranch::Unevaluated("No solver for this structure.".to_string())
}

/// The real-domain filter the top-level polynomial path applies, restated for the inner branches.
fn filter_real_domain(
    outcome: lovelace_symbolics::SolveOutcome,
    domain: &str,
) -> lovelace_symbolics::SolveOutcome {
    use lovelace_symbolics::SolveOutcome;
    if domain != "real" {
        return outcome;
    }
    match outcome {
        SolveOutcome::Partial(roots) if roots.is_empty() => SolveOutcome::NoRealSolutions,
        SolveOutcome::Unevaluated => SolveOutcome::NoRealSolutions,
        SolveOutcome::RootExpressions(expressions) => {
            let real: Vec<_> = expressions
                .into_iter()
                .filter(|e| !e.has_imaginary_unit())
                .collect();
            if real.is_empty() {
                SolveOutcome::NoRealSolutions
            } else {
                SolveOutcome::RootExpressions(real)
            }
        }
        SolveOutcome::RootExpressionsCount(expressions, _) => {
            SolveOutcome::RootExpressions(expressions)
        }
        other => other,
    }
}

/// The solution exactness classifier the SolveResult record uses: a radical or algebraic value is
/// AlgebraicExact, a plain elementary inverse such as log(5) is Exact.
fn solution_exactness(e: &Sym) -> &'static str {
    let algebraic = e.contains_fun("sqrt")
        || e.contains_fun("rootof")
        || e.has_imaginary_unit()
        || e.has_algebraic_power()
        || e.contains_fun("acos");
    if algebraic { "AlgebraicExact" } else { "Exact" }
}

fn solution_record(value: &Sym, conditions: &[Sym]) -> Value {
    Value::Record(
        "Solution".to_string(),
        vec![
            ("value".to_string(), Value::Symbolic(value.clone())),
            (
                "conditions".to_string(),
                Value::Vector(conditions.iter().cloned().map(Value::Symbolic).collect()),
            ),
            ("multiplicity".to_string(), Value::Integer(Int::from_i64(1))),
            (
                "exactness".to_string(),
                enum_value("SolutionExactness", solution_exactness(value)),
            ),
        ],
    )
}

/// The conditions every solution shares (an intersection, never a union).
fn common_conditions(solutions: &[(Sym, Vec<Sym>)]) -> Vec<Sym> {
    let Some((_, first)) = solutions.first() else {
        return Vec::new();
    };
    let mut common = first.clone();
    for (_, conditions) in solutions.iter().skip(1) {
        common.retain(|atom| conditions.contains(atom));
    }
    common
}

/// Projects one elementary branch onto the SolveResult record.
fn solve_branch_to_value(engine: &Engine, var: &str, domain: &str, branch: SolveBranch) -> Value {
    match branch {
        SolveBranch::Empty(message) => no_solutions_result(var, domain, &message),
        SolveBranch::Unevaluated(message) => {
            unevaluated_solve_result(var, domain, "solve.unevaluated", &message)
        }
        SolveBranch::Set {
            solutions,
            families,
        } => {
            let common = common_conditions(&solutions);
            let records: Vec<Value> = solutions
                .iter()
                .map(|(value, conditions)| solution_record(value, conditions))
                .collect();
            let represented = records.len();
            let has_families = !families.is_empty();
            let result = Value::Record(
                "SolveResult".to_string(),
                vec![
                    ("status".to_string(), enum_value("SolveStatus", "Solved")),
                    ("variable".to_string(), Value::Symbolic(Sym::sym(var))),
                    ("domain".to_string(), Value::Domain(domain.to_string())),
                    ("complete".to_string(), Value::Boolean(true)),
                    (
                        "completeness".to_string(),
                        enum_value("Completeness", "Complete"),
                    ),
                    ("solutions".to_string(), Value::Vector(records)),
                    ("families".to_string(), Value::Vector(families)),
                    (
                        "common_conditions".to_string(),
                        Value::Vector(common.iter().cloned().map(Value::Symbolic).collect()),
                    ),
                    (
                        "represented_count".to_string(),
                        Value::Integer(Int::from_i64(represented as i64)),
                    ),
                    (
                        "unrepresented_count".to_string(),
                        Value::Integer(Int::from_i64(0)),
                    ),
                    (
                        "unrepresented_reason".to_string(),
                        Value::Text(String::new()),
                    ),
                    ("diagnostics".to_string(), Value::Vector(Vec::new())),
                ],
            );
            if has_families {
                family_with_assumptions(engine, var, result)
            } else {
                result
            }
        }
    }
}

/// The solver's publication boundary: a solution VALUE and equally a family TEMPLATE must not
/// contain the symbol being solved for. A circular entry is dropped; a set that lost one is no
/// longer complete (Unevaluated when nothing survived, Partial when something did).
fn enforce_solution_values(result: Value, var: &str) -> Value {
    let Value::Record(name, mut fields) = result else {
        return result;
    };
    let mut circular: u64 = 0;
    if let Some((_, Value::Vector(solutions))) =
        fields.iter_mut().find(|(key, _)| key == "solutions")
    {
        let before = solutions.len();
        solutions.retain(|entry| match entry {
            Value::Record(_, solution_fields) => {
                match solution_fields.iter().find(|(key, _)| key == "value") {
                    Some((_, Value::Symbolic(value))) => !mentions_solved_variable(value, var),
                    _ => true,
                }
            }
            _ => true,
        });
        circular += (before - solutions.len()) as u64;
    }
    if let Some((_, Value::Vector(families))) = fields.iter_mut().find(|(key, _)| key == "families")
    {
        let before = families.len();
        families.retain(|entry| match entry {
            Value::Record(_, family_fields) => {
                match family_fields.iter().find(|(key, _)| key == "template") {
                    Some((_, Value::Symbolic(template))) => {
                        !mentions_solved_variable(template, var)
                    }
                    _ => true,
                }
            }
            _ => true,
        });
        circular += (before - families.len()) as u64;
    }
    if circular == 0 {
        return Value::Record(name, fields);
    }
    let reason = format!(
        "an inverse branch produced {circular} candidate(s) that still contain the solved variable \
         {var} (a circular answer, not a value), so they are not represented and the set is not \
         claimed complete."
    );
    let solutions_left = matches!(
        fields.iter().find(|(key, _)| key == "solutions"),
        Some((_, Value::Vector(v))) if !v.is_empty()
    );
    let families_left = matches!(
        fields.iter().find(|(key, _)| key == "families"),
        Some((_, Value::Vector(v))) if !v.is_empty()
    );
    fn set_field(fields: &mut Vec<(String, Value)>, key: &str, value: Value) {
        if let Some(slot) = fields.iter_mut().find(|(k, _)| k == key) {
            slot.1 = value;
        }
    }
    if !solutions_left && !families_left {
        set_field(
            &mut fields,
            "status",
            enum_value("SolveStatus", "Unevaluated"),
        );
        set_field(&mut fields, "complete", Value::Boolean(false));
        set_field(
            &mut fields,
            "completeness",
            enum_value("Completeness", "Unknown"),
        );
        set_field(
            &mut fields,
            "represented_count",
            Value::Integer(Int::from_i64(0)),
        );
        set_field(
            &mut fields,
            "unrepresented_count",
            Value::Integer(Int::from_i64(0)),
        );
        set_field(
            &mut fields,
            "unrepresented_reason",
            Value::Text(String::new()),
        );
        set_field(
            &mut fields,
            "diagnostics",
            Value::Vector(vec![diagnostic(
                "solve.unevaluated",
                "UnsupportedOperation",
                &reason,
            )]),
        );
        return Value::Record(name, fields);
    }
    // Partial: something survived, so the set is a real (incomplete) answer.
    let previous: u64 = match fields.iter().find(|(key, _)| key == "unrepresented_count") {
        Some((_, Value::Integer(count))) => count.to_decimal().parse::<u64>().unwrap_or(0),
        _ => 0,
    };
    set_field(&mut fields, "status", enum_value("SolveStatus", "Partial"));
    set_field(&mut fields, "complete", Value::Boolean(false));
    set_field(
        &mut fields,
        "completeness",
        enum_value("Completeness", "Partial"),
    );
    set_field(
        &mut fields,
        "unrepresented_count",
        Value::Integer(Int::from_i64((previous + circular) as i64)),
    );
    set_field(&mut fields, "unrepresented_reason", Value::Text(reason));
    Value::Record(name, fields)
}
/// The MathIR format-version gate. The supported version is 2; the refusal quotes everything
/// after the "#!mathir " prefix, which is exactly what the reference's IrProgram.Deserialize
/// reports (`Unsupported MathIR format version '<tail>' (expected 2).`) as a ParseError.
fn ir_version_error(text: &str, pos: usize) -> Option<LangError> {
    let rest = text.strip_prefix("#!mathir")?;
    let tail = rest.strip_prefix(' ').unwrap_or(rest);
    if tail.starts_with('2') {
        return None;
    }
    Some(LangError::coded(
        format!("Unsupported MathIR format version '{tail}' (expected 2)."),
        pos,
        "InvalidInput",
        "ParseError",
    ))
}

/// Reads back the MathIR text emitted by lower: constants, parameters and one node per line.
fn parse_ir_text(text: &str) -> Option<Sym> {
    let mut constants: Vec<Sym> = Vec::new();
    let mut params: Vec<String> = Vec::new();
    let mut nodes: Vec<Sym> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("#!") {
            continue;
        }
        if let Some(rest) = line.strip_prefix("param ") {
            params.push(rest.trim().to_string());
            continue;
        }
        if let Some(rest) = line.strip_prefix("const ") {
            let inner = rest.trim();
            if let Some(body) = inner
                .strip_prefix("(rat ")
                .and_then(|s| s.strip_suffix(')'))
            {
                let mut parts = body.split_whitespace();
                let num = parts.next()?;
                let den = parts.next()?;
                let n = Int::from_decimal(num)?;
                let d = Nat::from_decimal(den)?;
                constants.push(Sym::rational(&n, &d));
                continue;
            }
            return None;
        }
        let mut parts = line.splitn(3, ' ');
        let op = parts.next()?;
        let _aux = parts.next()?;
        let children = parts.next().unwrap_or("");
        let mut indices: Vec<usize> = Vec::new();
        for piece in children.split(',') {
            if piece.is_empty() {
                continue;
            }
            indices.push(piece.parse::<usize>().ok()?);
        }
        let node = match op {
            "Parameter" => Sym::sym(&params.get(indices.first().copied()?)?.clone()),
            "Constant" => constants.get(indices.first().copied()?)?.clone(),
            "Add" => {
                let mut acc = Sym::num(0);
                for i in &indices {
                    acc = Sym::add(acc, nodes.get(*i)?.clone());
                }
                acc
            }
            "Mul" => {
                let mut acc = Sym::num(1);
                for i in &indices {
                    acc = Sym::mul(acc, nodes.get(*i)?.clone());
                }
                acc
            }
            "PowInt" => {
                // The exponent is its own constant node in the published format.
                let base = nodes.get(indices.first().copied()?)?.clone();
                let exponent = nodes.get(indices.get(1).copied()?)?.clone();
                Sym::pow(base, exponent)
            }
            "Div" => {
                let a = nodes.get(indices.first().copied()?)?.clone();
                let b = nodes.get(indices.get(1).copied()?)?.clone();
                Sym::Div(Box::new(a), Box::new(b))
            }
            other => {
                let mut args: Vec<Sym> = Vec::new();
                for i in &indices {
                    args.push(nodes.get(*i)?.clone());
                }
                Sym::Fun(other.to_string(), args)
            }
        };
        nodes.push(node);
    }
    nodes.pop()
}

/// Rewrites a univariate polynomial in Horner form, factoring the lowest power of the variable
/// first. Returns None when the expression is not a polynomial in that variable.
fn horner_form(e: &Sym, var: &str) -> Option<Sym> {
    let coeffs = lovelace_symbolics::poly_coeffs(e, var)?;
    if coeffs.len() < 2 {
        return None;
    }
    let lowest = coeffs.iter().position(|c| !c.is_zero())?;
    let trimmed: Vec<_> = coeffs[lowest..].to_vec();
    let mut acc = Sym::Num(trimmed[trimmed.len() - 1].clone());
    for c in trimmed.iter().rev().skip(1) {
        // A level whose multiplicand is the bare variable reads as x + c; otherwise the
        // constant leads, as in 1 + x*(...).
        let product = Sym::mul(Sym::sym(var), acc.clone());
        let constant = Sym::Num(c.clone());
        acc = if matches!(product, Sym::Sym(_)) {
            Sym::Add(vec![product, constant])
        } else {
            Sym::Add(vec![constant, product])
        };
    }
    if lowest == 0 {
        Some(acc)
    } else {
        Some(Sym::Mul(vec![
            Sym::pow(Sym::sym(var), Sym::num(lowest as i64)),
            acc,
        ]))
    }
}

/// Solves a small system by splitting on a product equation and substituting each factor.

/// Fraction-free (Bareiss) linear solve over exact symbolic expressions: forward elimination
/// with the previous pivot as divisor, then back-substitution. This is the reference's
/// `SymbolicMatrix.SolveCore`; its intermediate quotients are part of the published shape
/// (sym/076), so it is not interchangeable with Cramer's rule even though both are exact.
fn solve_bareiss(matrix: &[Vec<Sym>], rhs: &[Sym]) -> Option<Vec<Sym>> {
    let n = matrix.len();
    if rhs.len() != n || matrix.iter().any(|row| row.len() != n) {
        return None;
    }
    let mut m: Vec<Vec<Sym>> = Vec::with_capacity(n);
    for r in 0..n {
        let mut row = matrix[r].clone();
        row.push(rhs[r].clone());
        m.push(row);
    }
    let mut prev = Sym::num(1);
    for k in 0..n {
        // The first row whose pivot entry is not a provable zero, exactly as the reference
        // selects it: no magnitude heuristic, no partial pivoting.
        let pivot = (k..n).find(|i| !matches!(m[*i][k], Sym::Num(ref r) if r.is_zero()))?;
        if pivot != k {
            m.swap(k, pivot);
        }
        for i in (k + 1)..n {
            for j in (k + 1)..=n {
                let left = Sym::mul(m[i][j].clone(), m[k][k].clone());
                let right = Sym::mul(m[i][k].clone(), m[k][j].clone());
                let numerator = Sym::sub(left, right);
                m[i][j] = exact_divide(&numerator, &prev);
            }
            m[i][k] = Sym::num(0);
        }
        prev = m[k][k].clone();
    }
    let mut solution = vec![Sym::num(0); n];
    for i in (0..n).rev() {
        let mut acc = m[i][n].clone();
        for j in (i + 1)..n {
            let term = Sym::mul(m[i][j].clone(), solution[j].clone());
            // Adding to a literal zero would wrap the term in an extra product and change the
            // published division shape; the reference's Add(0, t) is t.
            acc = if acc == Sym::num(0) {
                Sym::neg(term)
            } else {
                Sym::sub(acc, term)
            };
        }
        solution[i] = exact_divide(&acc, &m[i][i]);
    }
    Some(solution)
}

/// Exact division when the divisor is a unit; otherwise the rational quotient is the
/// value-correct fallback the reference uses too (its polynomial division is an optimization).
fn exact_divide(a: &Sym, b: &Sym) -> Sym {
    if *b == Sym::num(1) {
        return a.clone();
    }
    if let (Sym::Num(x), Sym::Num(y)) = (a, b) {
        if let Some(q) = x.div(y) {
            return Sym::Num(q);
        }
    }
    Sym::div(a.clone(), b.clone())
}

/// Solves a square LINEAR system by exact Gaussian elimination over the rationals.
///
/// The coefficient extraction is structural, not a guess: substituting every variable with zero
/// gives the constant term, substituting all but one with zero and that one with one gives the
/// constant plus that coefficient, and the claimed linear form is checked against the equation
/// before it is trusted — a nonlinear equation simply does not take this path.
fn solve_linear_system(equations: &[Sym], vars: &[String]) -> Option<Vec<Vec<(String, Sym)>>> {
    let n = vars.len();
    if n == 0 || equations.len() != n {
        return None;
    }
    let evaluate = |e: &Sym, assign: &[(String, Sym)]| -> Sym {
        let mut cur = e.clone();
        for (name, value) in assign {
            cur = lovelace_symbolics::substitute(&cur, name, value);
        }
        cur
    };
    let zero_assign: Vec<(String, Sym)> = vars.iter().map(|v| (v.clone(), Sym::num(0))).collect();
    let mut rows: Vec<Vec<lovelace_symbolics::Rat>> = Vec::with_capacity(n);
    for equation in equations {
        let constant = match evaluate(equation, &zero_assign) {
            Sym::Num(r) => r,
            _ => return None,
        };
        let mut row = Vec::with_capacity(n + 1);
        for i in 0..n {
            let mut assign = zero_assign.clone();
            assign[i] = (vars[i].clone(), Sym::num(1));
            let at_one = evaluate(equation, &assign);
            let coefficient = match at_one {
                Sym::Num(r) => r,
                _ => return None,
            };
            row.push(coefficient.add(&constant.neg()));
        }
        row.push(constant);
        rows.push(row);
    }
    // A structurally linear system is checked by re-reading each equation under its own
    // coefficients: f == c0 + sum c_i*x_i.
    for (index, equation) in equations.iter().enumerate() {
        let mut claimed = Sym::Num(rows[index][n].clone());
        for (i, name) in vars.iter().enumerate() {
            claimed = Sym::add(
                claimed,
                Sym::mul(Sym::Num(rows[index][i].clone()), Sym::sym(name)),
            );
        }
        if claimed != *equation {
            return None;
        }
    }
    // f = c0 + sum(c_i*x_i) = 0, so the elimination's right-hand side is -c0.
    for row in rows.iter_mut() {
        row[n] = row[n].neg();
    }
    // Gaussian elimination on the augmented matrix.
    let mut a = rows;
    for col in 0..n {
        let pivot = (col..n).find(|r| !a[*r][col].is_zero())?;
        a.swap(col, pivot);
        let p = a[col][col].clone();
        for k in col..=n {
            a[col][k] = a[col][k].div(&p)?;
        }
        for r in 0..n {
            if r == col {
                continue;
            }
            let factor = a[r][col].clone();
            if factor.is_zero() {
                continue;
            }
            for k in col..=n {
                let subtracted = a[col][k].mul(&factor);
                a[r][k] = a[r][k].add(&subtracted.neg());
            }
        }
    }
    Some(vec![
        vars.iter()
            .enumerate()
            .map(|(i, name)| (name.clone(), Sym::Num(a[i][n].clone())))
            .collect(),
    ])
}

/// The outcome of a polynomial-system solve, before the wire record is built.
enum SystemOutcome {
    /// At least one verified solution assignment.
    Solutions(Vec<Vec<(String, Sym)>>),
    /// The elimination DERIVED a contradiction: an empty set proved, no note needed.
    NoSolutions,
    /// The branch search stopped without a proof; the string starts with `system-solve.`.
    Unevaluated(String),
}

struct SystemSolveState {
    solutions: Vec<Vec<(String, Sym)>>,
    contradictions: usize,
    unproved: usize,
    first_reason: Option<String>,
    truncated: bool,
}

impl SystemSolveState {
    fn new() -> SystemSolveState {
        SystemSolveState {
            solutions: Vec::new(),
            contradictions: 0,
            unproved: 0,
            first_reason: None,
            truncated: false,
        }
    }

    /// Records a branch abandoned without a proof, keeping the first stable reason.
    fn abandon(&mut self, reason: String) {
        self.unproved += 1;
        if self.first_reason.is_none() {
            self.first_reason = Some(reason);
        }
    }
}

const MAX_SYSTEM_SOLUTIONS: usize = 128;
const NO_EQUATIONS_NOTE: &str = "system-solve.no-equations: the system has no equations, so every assignment of the variables is a solution; the solution set is not enumerable here.";
const NON_POLYNOMIAL_NOTE: &str =
    "system-solve.non-polynomial: only polynomial systems are supported in v1.";

/// Solves a polynomial system by Lex Gröbner elimination into a triangular system, exactly as
/// the reference's `SystemSolvers`: only a DERIVED contradiction makes the result NoSolutions;
/// every abandoned branch keeps the answer Unevaluated with its stable note.
fn solve_system_impl(equations: &[Sym], vars: &[String]) -> SystemOutcome {
    if equations.is_empty() {
        return SystemOutcome::Unevaluated(NO_EQUATIONS_NOTE.to_string());
    }
    // The polynomiality gate runs on the whole system before any elimination, as the reference
    // does: a non-polynomial equation names its own refusal instead of a branch reason.
    if equations
        .iter()
        .any(|equation| Polynomial::from_expr(equation, vars).is_none())
    {
        return SystemOutcome::Unevaluated(NON_POLYNOMIAL_NOTE.to_string());
    }
    if vars.is_empty() {
        for equation in equations {
            match Polynomial::from_expr(equation, vars) {
                Some(p) if p.is_zero() => {}
                Some(_) => return SystemOutcome::NoSolutions,
                None => {
                    return SystemOutcome::Unevaluated(
                        "system-solve.elimination-incomplete: a remaining equation is not a numeric constant, so this branch is not decided.".to_string(),
                    )
                }
            }
        }
        return SystemOutcome::Solutions(vec![Vec::new()]);
    }
    let mut state = SystemSolveState::new();
    solve_system_recursive(equations, vars, &mut Vec::new(), &mut state);
    if !state.solutions.is_empty() {
        return SystemOutcome::Solutions(state.solutions);
    }
    if state.contradictions > 0 && state.unproved == 0 && !state.truncated {
        return SystemOutcome::NoSolutions;
    }
    SystemOutcome::Unevaluated(state.first_reason.unwrap_or_else(|| {
        "system-solve.elimination-incomplete: the system was not decided.".to_string()
    }))
}

fn solve_system_recursive(
    equations: &[Sym],
    vars: &[String],
    partial: &mut Vec<(String, Sym)>,
    state: &mut SystemSolveState,
) {
    if state.solutions.len() >= MAX_SYSTEM_SOLUTIONS {
        state.truncated = true;
        return;
    }
    if vars.is_empty() {
        for equation in equations {
            match Polynomial::from_expr(equation, vars) {
                Some(p) if p.is_zero() => {}
                Some(_) => {
                    state.contradictions += 1;
                    return;
                }
                None => {
                    state.abandon(
                        "system-solve.elimination-incomplete: a remaining equation is not a numeric constant, so this branch is not decided.".to_string(),
                    );
                    return;
                }
            }
        }
        state.solutions.push(partial.clone());
        return;
    }
    if vars.len() == 1 {
        solve_last_variable(equations, &vars[0], partial, state);
        return;
    }
    let Some(polys) = polynomial_system(equations, vars) else {
        state.abandon(
            "system-solve.elimination-incomplete: the system stopped being a polynomial system after substitution, so this branch is not decided.".to_string(),
        );
        return;
    };
    let basis = groebner_basis(&polys, MonomialOrder::Lex);
    let eliminated_variables = &vars[..vars.len() - 1];
    let mut eliminated: Vec<&Polynomial> = basis
        .iter()
        .filter(|p| !p.is_zero() && p.is_free_of(eliminated_variables))
        .collect();
    eliminated.sort_by_key(|p| p.total_degree());
    let Some(univariate) = eliminated.first() else {
        let last = &vars[vars.len() - 1];
        state.abandon(format!(
            "system-solve.elimination-incomplete: the Gröbner basis has no polynomial in {last} alone, so the values of {last} are not enumerated here."
        ));
        return;
    };
    let last = &vars[vars.len() - 1];
    if univariate.total_degree() <= 0 {
        state.contradictions += 1;
        return;
    }
    let Some(coefficients) = lovelace_symbolics::poly_coeffs(&univariate.to_expr(), last) else {
        state.abandon(format!(
            "system-solve.elimination-incomplete: the elimination polynomial in {last} is not convertible, so this branch is not decided."
        ));
        return;
    };
    let outcome = lovelace_symbolics::solve_polynomial_named(&coefficients, last);
    let (roots, partial_roots) = roots_of(outcome);
    if roots.is_empty() {
        state.abandon(format!(
            "system-solve.univariate-incomplete: the elimination polynomial in {last} has no representable root, so this branch is not decided."
        ));
        return;
    }
    if partial_roots {
        state.abandon(format!(
            "system-solve.univariate-incomplete: the roots of the elimination polynomial in {last} are only partially representable."
        ));
    }
    for root in roots {
        if state.solutions.len() >= MAX_SYSTEM_SOLUTIONS {
            state.truncated = true;
            return;
        }
        let substituted: Vec<Sym> = equations
            .iter()
            .map(|equation| lovelace_symbolics::substitute(equation, last, &root))
            .collect();
        partial.push((last.clone(), root));
        solve_system_recursive(&substituted, &vars[..vars.len() - 1], partial, state);
        partial.pop();
    }
}

/// The roots an outcome carries, plus whether the outcome admitted missing roots.
fn roots_of(outcome: lovelace_symbolics::SolveOutcome) -> (Vec<Sym>, bool) {
    use lovelace_symbolics::SolveOutcome;
    match outcome {
        SolveOutcome::Roots(roots) => (
            roots
                .into_iter()
                .map(|r| lovelace_symbolics::expr_from_rat(&r.value))
                .collect(),
            false,
        ),
        SolveOutcome::RootExpressions(expressions) => (expressions, false),
        SolveOutcome::RootExpressionsCount(expressions, missing) => (expressions, missing > 0),
        SolveOutcome::Partial(roots) => (
            roots
                .into_iter()
                .map(|r| lovelace_symbolics::expr_from_rat(&r))
                .collect(),
            true,
        ),
        SolveOutcome::NoSolutions | SolveOutcome::NoRealSolutions | SolveOutcome::Unevaluated => {
            (Vec::new(), true)
        }
    }
}

fn polynomial_system(equations: &[Sym], vars: &[String]) -> Option<Vec<Polynomial>> {
    equations
        .iter()
        .map(|equation| Polynomial::from_expr(equation, vars))
        .collect()
}

/// How a candidate assignment fared against the equations it must satisfy.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CandidateVerdict {
    Accepted,
    Refuted,
    Unchecked,
}

/// One variable left: an equation that restricts it must have a COMPLETE representable root set;
/// an equation whose coefficients are not rational (a radical introduced by back-substitution)
/// still determines the value when its leading coefficient is provably nonzero.
fn solve_last_variable(
    equations: &[Sym],
    x: &str,
    partial: &mut Vec<(String, Sym)>,
    state: &mut SystemSolveState,
) {
    let mut isolated: Option<(usize, Polynomial)> = None;
    for (index, equation) in equations.iter().enumerate() {
        let Some(candidate) = Polynomial::from_expr(equation, &[x.to_string()]) else {
            continue;
        };
        if candidate.is_zero() {
            continue;
        }
        isolated = Some((index, candidate));
        break;
    }
    if let Some((index, polynomial)) = isolated {
        if polynomial.total_degree() <= 0 {
            state.contradictions += 1;
            return;
        }
        let Some(coefficients) = lovelace_symbolics::poly_coeffs(&polynomial.to_expr(), x) else {
            state.abandon(format!(
                "system-solve.univariate-incomplete: the equation that determines {x} has no representable root."
            ));
            return;
        };
        let outcome = lovelace_symbolics::solve_polynomial_named(&coefficients, x);
        let (roots, partial_roots) = roots_of(outcome);
        if roots.is_empty() {
            state.abandon(format!(
                "system-solve.univariate-incomplete: the equation that determines {x} has no representable root."
            ));
            return;
        }
        let others: Vec<Sym> = equations
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != index)
            .map(|(_, equation)| equation.clone())
            .collect();
        let mut accepted = 0usize;
        let mut undecided = 0usize;
        for root in roots {
            match judge_candidate(&root, &others, x) {
                CandidateVerdict::Accepted => {
                    let mut assignment = partial.clone();
                    assignment.push((x.to_string(), root));
                    state.solutions.push(assignment);
                    accepted += 1;
                }
                CandidateVerdict::Refuted => {}
                CandidateVerdict::Unchecked => undecided += 1,
            }
            if state.solutions.len() >= MAX_SYSTEM_SOLUTIONS {
                state.truncated = true;
                return;
            }
        }
        if accepted > 0 {
            if partial_roots {
                state.abandon(format!(
                    "system-solve.univariate-incomplete: the equation that determines {x} has roots this kernel cannot represent."
                ));
            }
            return;
        }
        if !partial_roots && undecided == 0 {
            // Every root of the restricting equation is refuted by the others: a proof.
            state.contradictions += 1;
            return;
        }
        state.abandon(if undecided > 0 {
            format!(
                "system-solve.candidate-undecided: no candidate for {x} could be checked against the other equations."
            )
        } else {
            format!(
                "system-solve.univariate-incomplete: the roots of {x} are only partially representable, so an empty result here is not a proof."
            )
        });
        return;
    }

    // An equation c1*x + c0 = 0 whose coefficients are not rational constants. The radicals a
    // triangular elimination introduces live here; a provably nonzero c1 pins exactly one value.
    for (index, equation) in equations.iter().enumerate() {
        let Some(coefficients) = expression_coefficients(equation, x) else {
            continue;
        };
        if coefficients.len() > 2 {
            continue;
        }
        let c0 = coefficients[0].clone();
        if coefficients.len() == 1 {
            if provably_nonzero_constant(&c0) {
                state.contradictions += 1;
                return;
            }
            continue;
        }
        let c1 = coefficients[1].clone();
        if !provably_nonzero_constant(&c1) {
            continue;
        }
        let root = Sym::div(Sym::neg(c0), c1);
        let others: Vec<Sym> = equations
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != index)
            .map(|(_, equation)| equation.clone())
            .collect();
        match judge_candidate(&root, &others, x) {
            CandidateVerdict::Accepted => {
                let mut assignment = partial.clone();
                assignment.push((x.to_string(), root));
                state.solutions.push(assignment);
            }
            CandidateVerdict::Refuted => state.contradictions += 1,
            CandidateVerdict::Unchecked => state.abandon(format!(
                "system-solve.candidate-undecided: the only candidate for {x} could not be checked against the other equations."
            )),
        }
        return;
    }

    state.abandon(format!(
        "system-solve.elimination-incomplete: no equation of this branch determines {x}, so the values of {x} are not enumerated here."
    ));
}

/// Splits an expression into coefficients of `var` as expressions (the reference's
/// `Algebra.TryCoefficients`): terms that mention the variable must be non-negative integer
/// powers of it; every other factor becomes part of the coefficient.
fn expression_coefficients(expression: &Sym, var: &str) -> Option<Vec<Sym>> {
    fn add(a: &[Sym], b: &[Sym]) -> Vec<Sym> {
        let mut out = vec![Sym::num(0); a.len().max(b.len())];
        for (i, slot) in out.iter_mut().enumerate() {
            *slot = Sym::add(
                a.get(i).cloned().unwrap_or_else(|| Sym::num(0)),
                b.get(i).cloned().unwrap_or_else(|| Sym::num(0)),
            );
        }
        trim_coefficients(out)
    }
    fn multiply(a: &[Sym], b: &[Sym]) -> Vec<Sym> {
        let mut out = vec![Sym::num(0); a.len() + b.len().saturating_sub(1)];
        for (i, left) in a.iter().enumerate() {
            for (j, right) in b.iter().enumerate() {
                out[i + j] = Sym::add(out[i + j].clone(), Sym::mul(left.clone(), right.clone()));
            }
        }
        trim_coefficients(out)
    }
    fn trim_coefficients(mut coefficients: Vec<Sym>) -> Vec<Sym> {
        while coefficients.len() > 1
            && matches!(coefficients.last(), Some(Sym::Num(n)) if n.is_zero())
        {
            coefficients.pop();
        }
        coefficients
    }
    match expression {
        Sym::Num(_) | Sym::Real(_) => Some(vec![expression.clone()]),
        Sym::Sym(name) if name == var => Some(vec![Sym::num(0), Sym::num(1)]),
        Sym::Sym(_) => Some(vec![expression.clone()]),
        Sym::Add(items) => {
            let mut coefficients = vec![Sym::num(0)];
            for item in items {
                let term = expression_coefficients(item, var)?;
                coefficients = add(&coefficients, &term);
            }
            Some(trim_coefficients(coefficients))
        }
        Sym::Mul(items) => {
            let mut coefficients = vec![Sym::num(1)];
            for item in items {
                let factor = expression_coefficients(item, var)?;
                coefficients = multiply(&coefficients, &factor);
            }
            Some(trim_coefficients(coefficients))
        }
        Sym::Pow(base, exponent) => {
            if !expression.contains(var) {
                return Some(vec![expression.clone()]);
            }
            let Sym::Num(power) = exponent.as_ref() else {
                return None;
            };
            if !power.is_integer() || power.is_negative() {
                return None;
            }
            let degree = power.to_i64()?;
            if !(0..=8).contains(&degree) {
                return None;
            }
            let Sym::Sym(name) = base.as_ref() else {
                return None;
            };
            if name != var {
                return None;
            }
            let mut coefficients = vec![Sym::num(0); degree as usize + 1];
            coefficients[degree as usize] = Sym::num(1);
            Some(coefficients)
        }
        Sym::Fun(..) | Sym::Div(..) | Sym::Rel(..) => {
            if expression.contains(var) {
                None
            } else {
                Some(vec![expression.clone()])
            }
        }
    }
}

/// True only for a PROVABLY nonzero rational constant (a literal, or a rational after
/// expansion). A radical is deliberately not decided by rounding.
fn provably_nonzero_constant(expression: &Sym) -> bool {
    match expression {
        Sym::Num(n) => !n.is_zero(),
        _ => match lovelace_symbolics::expand(expression) {
            Sym::Num(n) => !n.is_zero(),
            _ => false,
        },
    }
}

/// The system path's residual gate: Accepted only when every equation is provably zero at the
/// candidate, Refuted when an equation is provably nonzero, Unchecked when the evidence is
/// missing. That distinction separates a proof of emptiness from a search that merely stopped.
fn judge_candidate(value: &Sym, equations: &[Sym], x: &str) -> CandidateVerdict {
    let mut verdict = CandidateVerdict::Accepted;
    for equation in equations {
        let residual = lovelace_symbolics::substitute(equation, x, value);
        if matches!(&residual, Sym::Num(n) if n.is_zero()) {
            continue;
        }
        if matches!(&residual, Sym::Num(n) if !n.is_zero()) {
            return CandidateVerdict::Refuted;
        }
        let expanded = lovelace_symbolics::expand(&residual);
        if matches!(&expanded, Sym::Num(n) if n.is_zero()) {
            continue;
        }
        if matches!(&expanded, Sym::Num(n) if !n.is_zero()) {
            return CandidateVerdict::Refuted;
        }
        match numeric_residual_is_zero(&expanded) {
            Some(true) => continue,
            Some(false) => return CandidateVerdict::Refuted,
            None => verdict = CandidateVerdict::Unchecked,
        }
    }
    verdict
}

/// Evaluates a variable-free residual at 40 places and compares its magnitude with the
/// reference's 1e-24 tolerance. None means the expression is not numerically decidable.
fn numeric_residual_is_zero(expression: &Sym) -> Option<bool> {
    let value = crate::numeric::evaluate(expression, 40).ok()?;
    let real = value.to_real(40)?;
    let tolerance = Real::from_str("0.000000000000000000000001")?;
    Some(real.abs().cmp(&tolerance) == std::cmp::Ordering::Less)
}

/// NoSolutions over the session's domain: every assignment the elimination verified is refuted by
/// the assumption store, which is a COMPLETE answer and carries no note.
fn no_solutions_system() -> Value {
    Value::Record(
        "SystemSolveResult".to_string(),
        vec![
            (
                "status".to_string(),
                enum_value("SolveStatus", "NoSolutions"),
            ),
            ("domain".to_string(), Value::Domain("complex".to_string())),
            ("complete".to_string(), Value::Boolean(true)),
            (
                "completeness".to_string(),
                enum_value("Completeness", "Complete"),
            ),
            ("solutions".to_string(), Value::Vector(Vec::new())),
            ("diagnostics".to_string(), Value::Vector(Vec::new())),
        ],
    )
}

/// Filters the system's assignments through the SAME session store: an assignment is dropped as a
/// whole when one of its bindings is refuted, and a set every one of whose assignments is refuted
/// becomes NoSolutions.
fn system_result_with_assumptions(
    engine: &Engine,
    vars: &[String],
    solutions: Vec<Vec<(String, Sym)>>,
) -> Value {
    let had = !solutions.is_empty();
    let kept: Vec<Vec<(String, Sym)>> = solutions
        .into_iter()
        .filter(|assignment| {
            assignment.iter().all(|(name, value)| {
                let atoms = engine.assumption_atoms_on(name);
                let domains = engine.domain_assumptions_on(name);
                let mut undecided = Vec::new();
                root_admitted(value, name, &atoms, &domains, &mut undecided)
            })
        })
        .collect();
    if kept.is_empty() && had {
        return no_solutions_system();
    }
    system_result(vars, kept)
}

fn system_result(vars: &[String], solutions: Vec<Vec<(String, Sym)>>) -> Value {
    // Publish in a stable order: by the last variable's value, then the first.
    let mut sorted = solutions;
    // Order by the last variable's value, then the first: the reference publishes the
    // solutions of a circle-and-axes system in that sweep.
    let value_of = |s: &Vec<(String, Sym)>, name: &str| -> Option<lovelace_symbolics::Rat> {
        match s.iter().find(|(n, _)| n == name).map(|(_, v)| v) {
            Some(Sym::Num(r)) => Some(r.clone()),
            _ => None,
        }
    };
    let last = vars.last().cloned().unwrap_or_default();
    let first = vars.first().cloned().unwrap_or_default();
    sorted.sort_by(|a, b| {
        let compare =
            |x: Option<lovelace_symbolics::Rat>, y: Option<lovelace_symbolics::Rat>| match (x, y) {
                (Some(p), Some(q)) => p.cmp(&q),
                _ => std::cmp::Ordering::Equal,
            };
        let by_last = compare(value_of(a, &last), value_of(b, &last));
        if by_last != std::cmp::Ordering::Equal {
            return by_last;
        }
        compare(value_of(a, &first), value_of(b, &first))
    });
    let records: Vec<Value> = sorted
        .into_iter()
        .map(|bindings| {
            let bindings: Vec<Value> = vars
                .iter()
                .map(|name| {
                    let value = bindings
                        .iter()
                        .find(|(n, _)| n == name)
                        .map(|(_, v)| v.clone())
                        .unwrap_or_else(|| Sym::num(0));
                    Value::Record(
                        "Binding".to_string(),
                        vec![
                            ("name".to_string(), Value::Symbolic(Sym::sym(name))),
                            ("value".to_string(), Value::Symbolic(value)),
                        ],
                    )
                })
                .collect();
            Value::Record(
                "SystemSolution".to_string(),
                vec![
                    ("bindings".to_string(), Value::Vector(bindings)),
                    ("conditions".to_string(), Value::Vector(Vec::new())),
                    (
                        "exactness".to_string(),
                        enum_value("SolutionExactness", "Exact"),
                    ),
                ],
            )
        })
        .collect();
    Value::Record(
        "SystemSolveResult".to_string(),
        vec![
            ("status".to_string(), enum_value("SolveStatus", "Solved")),
            ("domain".to_string(), Value::Domain("complex".to_string())),
            ("complete".to_string(), Value::Boolean(true)),
            (
                "completeness".to_string(),
                enum_value("Completeness", "Complete"),
            ),
            ("solutions".to_string(), Value::Vector(records)),
            ("diagnostics".to_string(), Value::Vector(Vec::new())),
        ],
    )
}

/// Emits the MathIR text for a lowered expression.
///
/// The order is the published format: a power emits its base, then its exponent as a
/// constant node, then the power node; sums and products emit children in order; repeated
/// subexpressions reuse their index.
fn ir_text(e: &Sym, params: &[String]) -> String {
    let mut constants: Vec<String> = Vec::new();
    let mut nodes: Vec<String> = Vec::new();
    let mut cache: Vec<(String, usize)> = Vec::new();
    emit_ir(e, params, &mut constants, &mut nodes, &mut cache);
    let mut lines: Vec<String> = Vec::new();
    lines.push("#!mathir 2".to_string());
    for p in params {
        lines.push(format!("param {}", p));
    }
    for c in &constants {
        lines.push(format!("const {}", c));
    }
    lines.extend(nodes);
    lines.join("\n")
}

/// The CSE accounting the optimizer publishes: every subexpression that occurs more than once
/// contributes its extra occurrences, so a shared subtree is counted once per reuse.
fn shared_subtree_count(e: &Sym) -> usize {
    fn walk(e: &Sym, counts: &mut Vec<(String, usize)>) {
        let key = e.to_string();
        if let Some(slot) = counts.iter_mut().find(|(k, _)| *k == key) {
            slot.1 += 1;
            return;
        }
        counts.push((key, 1));
        match e {
            Sym::Add(items) | Sym::Mul(items) => items.iter().for_each(|i| walk(i, counts)),
            Sym::Pow(a, b) | Sym::Div(a, b) | Sym::Rel(_, a, b) => {
                walk(a, counts);
                walk(b, counts);
            }
            Sym::Fun(_, args) => args.iter().for_each(|a| walk(a, counts)),
            Sym::Num(_) | Sym::Real(_) | Sym::Sym(_) => {}
        }
    }
    let mut counts: Vec<(String, usize)> = Vec::new();
    walk(e, &mut counts);
    counts.iter().map(|(_, c)| c.saturating_sub(1)).sum()
}

/// The MathIR payload type of an expression over the given parameters: the narrowest type that
/// holds every value the expression can take. A polynomial with integer coefficients over the
/// parameters is integer-valued; rational coefficients keep it rational; a non-rational leaf (a
/// transcendental constant, a radical or a transcendental function) widens it to real, and an
/// imaginary leaf to complex.
fn result_type(e: &Sym, params: &[String]) -> String {
    use lovelace_symbolics::Dom;
    let _ = params;
    let domain = e.domain();
    match domain {
        Dom::Integer | Dom::Rational => {
            // A rational whose denominators are all one is integer-valued.
            if polynomial_is_integral(e) {
                "Integer".to_string()
            } else {
                "Rational".to_string()
            }
        }
        Dom::Real => "Real".to_string(),
        Dom::Complex => {
            if polynomial_is_integral(e) {
                "Integer".to_string()
            } else {
                "Complex".to_string()
            }
        }
    }
}

/// True when the expression is a polynomial with integer coefficients in its symbols.
fn polynomial_is_integral(e: &Sym) -> bool {
    match e {
        Sym::Num(n) => n.denominator().to_decimal() == "1",
        // a real constant is a decimal value: never an integer polynomial coefficient
        Sym::Real(_) => false,
        Sym::Sym(_) => true,
        Sym::Add(items) | Sym::Mul(items) => items.iter().all(polynomial_is_integral),
        Sym::Pow(base, exp) => {
            polynomial_is_integral(base)
                && matches!(exp.as_ref(), Sym::Num(n)
                    if n.denominator().to_decimal() == "1" && !n.is_negative())
        }
        Sym::Div(a, b) => polynomial_is_integral(a) && polynomial_is_integral(b),
        Sym::Fun(..) | Sym::Rel(..) => false,
    }
}

/// Emits a fresh Constant node over the deduplicated constant table. Literal nodes are never
/// cached: the reference's lowering adds one node per literal occurrence, so two occurrences of
/// the same value share a const entry but not a node (see the PowInt exponent path).
fn emit_constant_node(
    value: &lovelace_symbolics::Rat,
    constants: &mut Vec<String>,
    nodes: &mut Vec<String>,
) -> usize {
    let ci = constant_index(value, constants);
    nodes.push(format!("Constant 0 {}", ci));
    nodes.len() - 1
}

fn constant_index(value: &lovelace_symbolics::Rat, constants: &mut Vec<String>) -> usize {
    let text = format!(
        "(rat {} {})",
        value.numerator().to_decimal(),
        value.denominator().to_decimal()
    );
    match constants.iter().position(|c| *c == text) {
        Some(i) => i,
        None => {
            constants.push(text);
            constants.len() - 1
        }
    }
}

fn emit_ir(
    e: &Sym,
    params: &[String],
    constants: &mut Vec<String>,
    nodes: &mut Vec<String>,
    cache: &mut Vec<(String, usize)>,
) -> usize {
    let key = e.to_string();
    if let Some((_, index)) = cache.iter().find(|(k, _)| *k == key) {
        return *index;
    }
    let index = match e {
        Sym::Sym(name) => {
            let position = params.iter().position(|p| p == name).unwrap_or(0);
            nodes.push(format!("Parameter 0 {}", position));
            nodes.len() - 1
        }
        Sym::Num(r) => {
            let ci = constant_index(r, constants);
            nodes.push(format!("Constant 0 {}", ci));
            nodes.len() - 1
        }
        Sym::Real(text) => {
            let entry = format!("(real {text})");
            let ci = match constants.iter().position(|c| *c == entry) {
                Some(i) => i,
                None => {
                    constants.push(entry);
                    constants.len() - 1
                }
            };
            nodes.push(format!("Constant 0 {}", ci));
            nodes.len() - 1
        }
        Sym::Pow(base, exp) => match exp.as_ref() {
            Sym::Num(n) if n.denominator().to_decimal() == "1" => {
                let child = emit_ir(base, params, constants, nodes, cache);
                // The reference's PowInt exponent is emitted as a FRESH Constant node on every
                // occurrence (Lowering.Emit calls AddConstant, which never consults the node
                // cache): the constant TABLE dedupes by value, but the node does not. A literal
                // that appears both as an exponent and as a coefficient therefore names two
                // distinct nodes over one const entry, which is the published sym/071 listing.
                let exponent = emit_constant_node(n, constants, nodes);
                nodes.push(format!("PowInt 0 {},{}", child, exponent));
                nodes.len() - 1
            }
            _ => {
                let a = emit_ir(base, params, constants, nodes, cache);
                let b2 = emit_ir(exp, params, constants, nodes, cache);
                nodes.push(format!("Pow 0 {},{}", a, b2));
                nodes.len() - 1
            }
        },
        Sym::Add(terms) => {
            let mut ordered = terms.clone();
            ordered.sort_by(lovelace_symbolics::canonical_order);
            let mut children: Vec<usize> = Vec::new();
            for t in &ordered {
                children.push(emit_ir(t, params, constants, nodes, cache));
            }
            let list: Vec<String> = children.iter().map(|c| c.to_string()).collect();
            nodes.push(format!("Add 0 {}", list.join(",")));
            nodes.len() - 1
        }
        Sym::Mul(terms) => {
            let mut ordered = terms.clone();
            ordered.sort_by(lovelace_symbolics::canonical_order);
            let mut children: Vec<usize> = Vec::new();
            for t in &ordered {
                children.push(emit_ir(t, params, constants, nodes, cache));
            }
            let list: Vec<String> = children.iter().map(|c| c.to_string()).collect();
            nodes.push(format!("Mul 0 {}", list.join(",")));
            nodes.len() - 1
        }
        Sym::Div(a, b2) => {
            let x = emit_ir(a, params, constants, nodes, cache);
            let y = emit_ir(b2, params, constants, nodes, cache);
            nodes.push(format!("Div 0 {},{}", x, y));
            nodes.len() - 1
        }
        Sym::Fun(name, args) => {
            let mut children: Vec<usize> = Vec::new();
            for a in args {
                children.push(emit_ir(a, params, constants, nodes, cache));
            }
            let list: Vec<String> = children.iter().map(|c| c.to_string()).collect();
            nodes.push(format!("{} 0 {}", name, list.join(",")));
            nodes.len() - 1
        }
        Sym::Rel(_, _, _) => {
            nodes.push("Unsupported 0 ".to_string());
            nodes.len() - 1
        }
    };
    cache.push((key, index));
    index
}
/// Why a solution set is Partial although no root is missing from the wire: the roots exist in
/// the requested domain but the exact kernel cannot represent them (RootOf is real-only in v1).
/// This is the reference's own reason string, carried into `unrepresented_reason` and into the
/// `solve.unrepresented-roots` diagnostic message (C# SolvePolynomial / HighDegreeRoots).
const UNREPRESENTED_ROOTS_REASON: &str =
    "complex algebraic roots not supported (RootOf is real-only in v1).";

/// A NoSolutions SolveResult carrying the reference's own diagnostic note: the proof that the
/// set is empty (every numerator root violates the denominator condition, or there are none) with
/// no common conditions, because a condition that excludes every root is not a live branch.
fn no_solutions_result(var: &str, domain: &str, message: &str) -> Value {
    Value::Record(
        "SolveResult".to_string(),
        vec![
            (
                "status".to_string(),
                enum_value("SolveStatus", "NoSolutions"),
            ),
            ("variable".to_string(), Value::Symbolic(Sym::sym(var))),
            ("domain".to_string(), Value::Domain(domain.to_string())),
            ("complete".to_string(), Value::Boolean(true)),
            (
                "completeness".to_string(),
                enum_value("Completeness", "Complete"),
            ),
            ("solutions".to_string(), Value::Vector(Vec::new())),
            ("families".to_string(), Value::Vector(Vec::new())),
            ("common_conditions".to_string(), Value::Vector(Vec::new())),
            (
                "represented_count".to_string(),
                Value::Integer(Int::from_i64(0)),
            ),
            (
                "unrepresented_count".to_string(),
                Value::Integer(Int::from_i64(0)),
            ),
            (
                "unrepresented_reason".to_string(),
                Value::Text(String::new()),
            ),
            (
                "diagnostics".to_string(),
                Value::Vector(vec![diagnostic(
                    "solve.no-solutions",
                    "NoSolution",
                    message,
                )]),
            ),
        ],
    )
}

/// An Unevaluated SolveResult: the solver could not enumerate the set, and says why with the
/// reference's own diagnostic.
fn unevaluated_solve_result(var: &str, domain: &str, code: &str, message: &str) -> Value {
    Value::Record(
        "SolveResult".to_string(),
        vec![
            (
                "status".to_string(),
                enum_value("SolveStatus", "Unevaluated"),
            ),
            ("variable".to_string(), Value::Symbolic(Sym::sym(var))),
            ("domain".to_string(), Value::Domain(domain.to_string())),
            ("complete".to_string(), Value::Boolean(false)),
            (
                "completeness".to_string(),
                enum_value("Completeness", "Unknown"),
            ),
            ("solutions".to_string(), Value::Vector(Vec::new())),
            ("families".to_string(), Value::Vector(Vec::new())),
            ("common_conditions".to_string(), Value::Vector(Vec::new())),
            (
                "represented_count".to_string(),
                Value::Integer(Int::from_i64(0)),
            ),
            (
                "unrepresented_count".to_string(),
                Value::Integer(Int::from_i64(0)),
            ),
            (
                "unrepresented_reason".to_string(),
                Value::Text(String::new()),
            ),
            (
                "diagnostics".to_string(),
                Value::Vector(vec![diagnostic(code, "UnsupportedOperation", message)]),
            ),
        ],
    )
}

/// An Unevaluated SolveResult whose roots are ALL unrepresented: a polynomial of degree four or
/// more with no real root has only complex algebraic roots, and the kernel's RootOf is real-only.
/// The reference reports the SET as unevaluated (never "no solutions") and counts every root as
/// unrepresented, with the reason string the kernel publishes for this class.
fn unevaluated_unrepresented_result(var: &str, domain: &str, unrepresented: i64) -> Value {
    Value::Record(
        "SolveResult".to_string(),
        vec![
            (
                "status".to_string(),
                enum_value("SolveStatus", "Unevaluated"),
            ),
            ("variable".to_string(), Value::Symbolic(Sym::sym(var))),
            ("domain".to_string(), Value::Domain(domain.to_string())),
            ("complete".to_string(), Value::Boolean(false)),
            (
                "completeness".to_string(),
                enum_value("Completeness", "Unknown"),
            ),
            ("solutions".to_string(), Value::Vector(Vec::new())),
            ("families".to_string(), Value::Vector(Vec::new())),
            ("common_conditions".to_string(), Value::Vector(Vec::new())),
            (
                "represented_count".to_string(),
                Value::Integer(Int::from_i64(0)),
            ),
            (
                "unrepresented_count".to_string(),
                Value::Integer(Int::from_i64(unrepresented)),
            ),
            (
                "unrepresented_reason".to_string(),
                Value::Text(UNREPRESENTED_ROOTS_REASON.to_string()),
            ),
            (
                "diagnostics".to_string(),
                Value::Vector(vec![diagnostic(
                    "solve.unevaluated",
                    "UnsupportedOperation",
                    UNREPRESENTED_ROOTS_REASON,
                )]),
            ),
        ],
    )
}

/// An Unevaluated SystemSolveResult: well shaped, not enumerated here.
fn unevaluated_system_result(message: &str) -> Value {
    Value::Record(
        "SystemSolveResult".to_string(),
        vec![
            (
                "status".to_string(),
                enum_value("SolveStatus", "Unevaluated"),
            ),
            ("domain".to_string(), Value::Domain("complex".to_string())),
            ("complete".to_string(), Value::Boolean(false)),
            (
                "completeness".to_string(),
                enum_value("Completeness", "Unknown"),
            ),
            ("solutions".to_string(), Value::Vector(Vec::new())),
            (
                "diagnostics".to_string(),
                Value::Vector(vec![diagnostic(
                    "system-solve.unevaluated",
                    "UnsupportedOperation",
                    message,
                )]),
            ),
        ],
    )
}

/// The reference's structural inverse branch for `u^n = 0` when `u` is a SUM: solve `u = 0`
/// once per nth root of zero and append every inner solution `n` times (preserving the inner
/// multiplicity). `None` means "this shape is not ours; use the polynomial path".
fn power_branch_outcome(
    base: &Sym,
    var: &str,
    domain: &str,
    copies: i64,
) -> Option<lovelace_symbolics::SolveOutcome> {
    use lovelace_symbolics::{Root, SolveOutcome};
    let coeffs = lovelace_symbolics::poly_coeffs(base, var)?;
    let mut inner = lovelace_symbolics::solve_polynomial_named(&coeffs, var);
    if domain == "real" {
        inner = match inner {
            SolveOutcome::Partial(roots) if roots.is_empty() => SolveOutcome::NoRealSolutions,
            SolveOutcome::Unevaluated => SolveOutcome::NoRealSolutions,
            SolveOutcome::RootExpressions(exprs) => {
                let real: Vec<_> = exprs
                    .into_iter()
                    .filter(|e| !e.has_imaginary_unit())
                    .collect();
                if real.is_empty() {
                    SolveOutcome::NoRealSolutions
                } else {
                    SolveOutcome::RootExpressions(real)
                }
            }
            SolveOutcome::RootExpressionsCount(exprs, _) => SolveOutcome::RootExpressions(exprs),
            other => other,
        };
    }
    // A negative exponent reaches this branch with the sign still on the count; casting it to
    // usize would wrap to ~2^64 and the capacity multiply would overflow (or the allocation
    // would abort). This branch only models u^n = 0 for a positive n.
    if copies <= 0 {
        return None;
    }
    let copies = copies as usize;
    match inner {
        SolveOutcome::Roots(roots) => {
            if roots.is_empty() {
                return Some(SolveOutcome::NoSolutions);
            }
            let capacity = roots.len().checked_mul(copies)?;
            if capacity > MAX_SINGLE_ALLOCATION_ELEMENTS as usize {
                return None;
            }
            let mut expanded: Vec<Root> = Vec::with_capacity(capacity);
            for _ in 0..copies {
                expanded.extend(roots.iter().cloned());
            }
            // The reference sorts the final solution set numerically before publishing it.
            expanded.sort_by(|a, b| a.value.cmp(&b.value));
            Some(SolveOutcome::Roots(expanded))
        }
        SolveOutcome::RootExpressions(exprs) => {
            if exprs.is_empty() {
                return None;
            }
            let capacity = exprs.len().checked_mul(copies)?;
            if capacity > MAX_SINGLE_ALLOCATION_ELEMENTS as usize {
                return None;
            }
            let mut expanded: Vec<Sym> = Vec::with_capacity(capacity);
            for _ in 0..copies {
                expanded.extend(exprs.iter().cloned());
            }
            Some(SolveOutcome::RootExpressions(expanded))
        }
        SolveOutcome::NoRealSolutions | SolveOutcome::NoSolutions => {
            Some(SolveOutcome::NoSolutions)
        }
        _ => None,
    }
}

/// Why a solution set can be empty although the kernel found roots: every root is refuted by the
/// session's assumption store. That is a COMPLETE answer over the session's domain (the reference's
/// SolveRecord derives NoSolutions from the empty accepted list) and carries no note.
fn no_solutions_under_assumptions(var: &str, domain: &str) -> Value {
    Value::Record(
        "SolveResult".to_string(),
        vec![
            (
                "status".to_string(),
                enum_value("SolveStatus", "NoSolutions"),
            ),
            ("variable".to_string(), Value::Symbolic(Sym::sym(var))),
            ("domain".to_string(), Value::Domain(domain.to_string())),
            ("complete".to_string(), Value::Boolean(true)),
            (
                "completeness".to_string(),
                enum_value("Completeness", "Complete"),
            ),
            ("solutions".to_string(), Value::Vector(Vec::new())),
            ("families".to_string(), Value::Vector(Vec::new())),
            ("common_conditions".to_string(), Value::Vector(Vec::new())),
            (
                "represented_count".to_string(),
                Value::Integer(Int::from_i64(0)),
            ),
            (
                "unrepresented_count".to_string(),
                Value::Integer(Int::from_i64(0)),
            ),
            (
                "unrepresented_reason".to_string(),
                Value::Text(String::new()),
            ),
            ("diagnostics".to_string(), Value::Vector(Vec::new())),
        ],
    )
}

/// True when the value PROVABLY lies off the real line. Deliberately narrow, like the reference's
/// ProvablyNonReal: the imaginary unit and a product with exactly one imaginary factor whose other
/// factors are not complex (the shapes `i`, `-i` and `i*sqrt(2)` take).
fn provably_non_real(value: &Sym) -> bool {
    fn is_i(e: &Sym) -> bool {
        matches!(e, Sym::Sym(name) if name == "i")
            || matches!(e, Sym::Fun(name, args) if name == "i" && args.is_empty())
    }
    match value {
        Sym::Sym(name) => name == "i",
        Sym::Fun(name, args) => name == "i" && args.is_empty(),
        Sym::Mul(factors) => {
            factors.iter().filter(|factor| is_i(factor)).count() == 1
                && factors
                    .iter()
                    .filter(|factor| factor.has_imaginary_unit())
                    .count()
                    == 1
        }
        _ => false,
    }
}

/// A constant expression as an exact-or-computed Real, for deciding an assumption at a root.
fn const_real(expression: &Sym) -> Option<Real> {
    let value = crate::numeric::evaluate(expression, 64).ok()?;
    value.to_real(64)
}

/// Decides one assumption atom at `var = value`. `None` means the atom cannot be decided there
/// (it then rides on the solution as a condition instead of being guessed).
fn judge_atom(atom: &Sym, var: &str, value: &Sym) -> Option<bool> {
    let Sym::Rel(op, left, right) = atom else {
        return None;
    };
    use lovelace_symbolics::RelOp;
    // An ordering relation is satisfied by reals only, so a value that provably lies off the real
    // line is refuted here (the numeric tier cannot compare a complex value with a real bound).
    if matches!(op, RelOp::Gt | RelOp::Ge | RelOp::Lt | RelOp::Le) && provably_non_real(value) {
        return Some(false);
    }
    let left = lovelace_symbolics::substitute(left, var, value);
    let right = lovelace_symbolics::substitute(right, var, value);
    let left = const_real(&left)?;
    let right = const_real(&right)?;
    Some(match op {
        RelOp::Eq => left == right,
        RelOp::Ne => left != right,
        RelOp::Gt => left > right,
        RelOp::Ge => left >= right,
        RelOp::Lt => left < right,
        RelOp::Le => left <= right,
    })
}

/// Whether a value is compatible with an assumed DOMAIN of the solved variable. The lattice is
/// Integer < Rational < Real < Complex: a value at or inside the assumed domain is admitted, and a
/// value that PROVABLY lies outside it is refuted; anything else is undecided.
fn domain_admits(domain: &str, value: &Sym) -> Option<bool> {
    use lovelace_symbolics::Dom;
    let assumed = match domain {
        "integer" => 0u8,
        "rational" => 1u8,
        "real" => 2u8,
        _ => 3u8,
    };
    let actual = match value.domain() {
        Dom::Integer => 0u8,
        Dom::Rational => 1u8,
        Dom::Real => 2u8,
        Dom::Complex => 3u8,
    };
    if actual <= assumed {
        return Some(true);
    }
    if assumed < 3 && provably_non_real(value) {
        return Some(false);
    }
    if assumed == 0 {
        // An exact rational that is not a whole number is provably outside the integers.
        if let Sym::Num(r) = value {
            if !r.is_integer() {
                return Some(false);
            }
        }
    }
    None
}

/// The roots the session's assumptions admit, plus the atoms that could not be decided at any of
/// them (carried as conditions). `None` means every represented root was refuted.
fn filter_roots(
    roots: Vec<lovelace_symbolics::Root>,
    var: &str,
    atoms: &[Sym],
    domains: &[String],
    undecided: &mut Vec<Sym>,
) -> (Vec<lovelace_symbolics::Root>, bool) {
    let had = !roots.is_empty();
    let mut kept = Vec::with_capacity(roots.len());
    for root in roots {
        let value = lovelace_symbolics::expr_from_rat(&root.value);
        if root_admitted(&value, var, atoms, domains, undecided) {
            kept.push(root);
        }
    }
    let emptied = had && kept.is_empty();
    (kept, emptied)
}

fn root_admitted(
    value: &Sym,
    var: &str,
    atoms: &[Sym],
    domains: &[String],
    undecided: &mut Vec<Sym>,
) -> bool {
    for atom in atoms {
        match judge_atom(atom, var, value) {
            Some(true) => {}
            Some(false) => return false,
            None => {
                if !undecided
                    .iter()
                    .any(|existing| existing.to_string() == atom.to_string())
                {
                    undecided.push(atom.clone());
                }
            }
        }
    }
    for domain in domains {
        if domain_admits(domain, value) == Some(false) {
            return false;
        }
    }
    true
}

/// One solve result with the session's assumptions applied: refuted roots are dropped, a set every
/// one of whose roots is refuted becomes NoSolutions, and undecided atoms ride on the kept
/// solutions as conditions.
fn solve_with_assumptions(
    engine: &Engine,
    var: &str,
    domain: &str,
    outcome: lovelace_symbolics::SolveOutcome,
    condition: Option<Sym>,
) -> Value {
    use lovelace_symbolics::SolveOutcome;
    let atoms = engine.assumption_atoms_on(var);
    let domains = engine.domain_assumptions_on(var);
    if atoms.is_empty() && domains.is_empty() {
        return solve_result(var, domain, outcome, condition, &[]);
    }
    let mut undecided: Vec<Sym> = Vec::new();
    match outcome {
        SolveOutcome::Roots(roots) => {
            let (kept, emptied) = filter_roots(roots, var, &atoms, &domains, &mut undecided);
            if emptied {
                return no_solutions_under_assumptions(var, domain);
            }
            solve_result(
                var,
                domain,
                SolveOutcome::Roots(kept),
                condition,
                &undecided,
            )
        }
        SolveOutcome::RootExpressions(expressions) => {
            let (kept, emptied) =
                filter_expressions(expressions, var, &atoms, &domains, &mut undecided);
            if emptied {
                return no_solutions_under_assumptions(var, domain);
            }
            solve_result(
                var,
                domain,
                SolveOutcome::RootExpressions(kept),
                condition,
                &undecided,
            )
        }
        SolveOutcome::RootExpressionsCount(expressions, count) => {
            let before = expressions.len() as u64;
            let (kept, emptied) =
                filter_expressions(expressions, var, &atoms, &domains, &mut undecided);
            if emptied {
                return no_solutions_under_assumptions(var, domain);
            }
            let removed = before - kept.len() as u64;
            solve_result(
                var,
                domain,
                SolveOutcome::RootExpressionsCount(kept, count.saturating_sub(removed)),
                condition,
                &undecided,
            )
        }
        other => solve_result(var, domain, other, condition, &[]),
    }
}

fn filter_expressions(
    expressions: Vec<Sym>,
    var: &str,
    atoms: &[Sym],
    domains: &[String],
    undecided: &mut Vec<Sym>,
) -> (Vec<Sym>, bool) {
    let had = !expressions.is_empty();
    let kept: Vec<Sym> = expressions
        .into_iter()
        .filter(|value| root_admitted(value, var, atoms, domains, undecided))
        .collect();
    let emptied = had && kept.is_empty();
    (kept, emptied)
}

/// A family's conditions carry the atoms the store cannot decide one value at a time (a template
/// like `k*pi` is not a value, so `x > 5` rides on the family rather than being guessed).
fn family_with_assumptions(engine: &Engine, var: &str, family: Value) -> Value {
    let atoms = engine.assumption_atoms_on(var);
    if atoms.is_empty() {
        return family;
    }
    let Value::Record(name, mut fields) = family else {
        return family;
    };
    if let Some((_, Value::Vector(families))) = fields.iter_mut().find(|(key, _)| key == "families")
    {
        for entry in families.iter_mut() {
            if let Value::Record(_, family_fields) = entry {
                if let Some((_, Value::Vector(conditions))) = family_fields
                    .iter_mut()
                    .find(|(key, _)| key == "conditions")
                {
                    for atom in &atoms {
                        conditions.push(Value::Symbolic(atom.clone()));
                    }
                }
            }
        }
    }
    Value::Record(name, fields)
}

/// The solution conditions a solve publishes: its own excluded-domain condition plus every
/// ambient atom that could not be decided at the kept roots.
fn solution_conditions(condition: &Option<Sym>, extra: &[Sym]) -> Value {
    let mut items: Vec<Value> = Vec::new();
    if let Some(condition) = condition {
        items.push(Value::Symbolic(condition.clone()));
    }
    for atom in extra {
        items.push(Value::Symbolic(atom.clone()));
    }
    Value::Vector(items)
}

fn solve_result(
    var: &str,
    domain: &str,
    outcome: lovelace_symbolics::SolveOutcome,
    condition: Option<Sym>,
    extra_conditions: &[Sym],
) -> Value {
    use lovelace_symbolics::SolveOutcome;
    let mut unrepresented: u64 = 0;
    let (status, complete, completeness, solutions, diagnostics) = match outcome {
        SolveOutcome::Roots(roots) => {
            let solutions: Vec<Value> = roots
                .iter()
                .map(|r| {
                    Value::Record(
                        "Solution".to_string(),
                        vec![
                            (
                                "value".to_string(),
                                Value::Symbolic(lovelace_symbolics::expr_from_rat(&r.value)),
                            ),
                            (
                                "conditions".to_string(),
                                solution_conditions(&condition, extra_conditions),
                            ),
                            // The factorization's multiplicity, not a hard-coded 1: a repeated
                            // root is ONE solution carrying how many times it divides the
                            // polynomial (the oracle's x^2 - 2x + 1 is multiplicity 2).
                            (
                                "multiplicity".to_string(),
                                Value::Integer(Int::from_i64(r.multiplicity as i64)),
                            ),
                            (
                                "exactness".to_string(),
                                enum_value("SolutionExactness", "Exact"),
                            ),
                        ],
                    )
                })
                .collect();
            (
                "Solved",
                true,
                "Complete",
                Value::Vector(solutions),
                Value::Vector(Vec::new()),
            )
        }
        SolveOutcome::NoRealSolutions => (
            "NoSolutions",
            true,
            "Complete",
            Value::Vector(Vec::new()),
            Value::Vector(vec![diagnostic(
                "solve.no-solutions",
                "NoSolution",
                "no real solutions",
            )]),
        ),
        SolveOutcome::NoSolutions => (
            "NoSolutions",
            true,
            "Complete",
            Value::Vector(Vec::new()),
            Value::Vector(vec![diagnostic(
                "solve.no-solutions",
                "NoSolution",
                "the equation reduces to a nonzero constant.",
            )]),
        ),
        SolveOutcome::RootExpressions(exprs) => {
            let solutions: Vec<Value> = exprs
                .iter()
                .map(|e| {
                    // A radical or an algebraic number (RootOf) is AlgebraicExact; a value built
                    // only from rationals and symbols is Exact.
                    let algebraic = e.contains_fun("sqrt")
                        || e.contains_fun("rootof")
                        || e.has_imaginary_unit()
                        // A radical that is NOT a sqrt is still algebraic: Cardano's cube roots
                        // (2^(1/3)) carry a non-integer power, not a sqrt call. The trigonometric
                        // cubic branch (2*cos(...acos...)) is algebraic too; a plain elementary
                        // inverse such as log(5) stays Exact, as the oracle reports it.
                        || e.has_algebraic_power()
                        || e.contains_fun("acos");
                    Value::Record(
                        "Solution".to_string(),
                        vec![
                            ("value".to_string(), Value::Symbolic(e.clone())),
                            (
                                "conditions".to_string(),
                                solution_conditions(&condition, extra_conditions),
                            ),
                            ("multiplicity".to_string(), Value::Integer(Int::from_i64(1))),
                            (
                                "exactness".to_string(),
                                enum_value(
                                    "SolutionExactness",
                                    if algebraic { "AlgebraicExact" } else { "Exact" },
                                ),
                            ),
                        ],
                    )
                })
                .collect();
            (
                "Solved",
                true,
                "Complete",
                Value::Vector(solutions),
                Value::Vector(Vec::new()),
            )
        }
        SolveOutcome::RootExpressionsCount(exprs, count) => {
            unrepresented = count;
            let solutions: Vec<Value> = exprs
                .iter()
                .map(|e| {
                    Value::Record(
                        "Solution".to_string(),
                        vec![
                            ("value".to_string(), Value::Symbolic(e.clone())),
                            (
                                "conditions".to_string(),
                                solution_conditions(&condition, extra_conditions),
                            ),
                            ("multiplicity".to_string(), Value::Integer(Int::from_i64(1))),
                            (
                                "exactness".to_string(),
                                enum_value("SolutionExactness", "AlgebraicExact"),
                            ),
                        ],
                    )
                })
                .collect();
            (
                "Partial",
                false,
                "Partial",
                Value::Vector(solutions),
                Value::Vector(vec![diagnostic(
                    "solve.unrepresented-roots",
                    "UnsupportedOperation",
                    UNREPRESENTED_ROOTS_REASON,
                )]),
            )
        }
        SolveOutcome::Partial(roots) => {
            let solutions: Vec<Value> = roots
                .iter()
                .map(|r| {
                    Value::Record(
                        "Solution".to_string(),
                        vec![
                            (
                                "value".to_string(),
                                Value::Symbolic(lovelace_symbolics::expr_from_rat(r)),
                            ),
                            ("conditions".to_string(), Value::Vector(Vec::new())),
                            ("multiplicity".to_string(), Value::Integer(Int::from_i64(1))),
                            (
                                "exactness".to_string(),
                                enum_value("SolutionExactness", "Exact"),
                            ),
                        ],
                    )
                })
                .collect();
            (
                "Partial",
                false,
                "Partial",
                Value::Vector(solutions),
                Value::Vector(Vec::new()),
            )
        }
        SolveOutcome::Unevaluated => (
            "Unevaluated",
            false,
            "Unknown",
            Value::Vector(Vec::new()),
            Value::Vector(vec![diagnostic(
                "solve.unevaluated",
                "UnsupportedOperation",
                "No solver for this structure.",
            )]),
        ),
    };
    let represented = match &solutions {
        Value::Vector(v) => v.len(),
        _ => 0,
    };
    let unrepresented_reason = if unrepresented > 0 {
        UNREPRESENTED_ROOTS_REASON
    } else {
        ""
    };
    Value::Record(
        "SolveResult".to_string(),
        vec![
            ("status".to_string(), enum_value("SolveStatus", status)),
            ("variable".to_string(), Value::Symbolic(Sym::sym(var))),
            ("domain".to_string(), Value::Domain(domain.to_string())),
            ("complete".to_string(), Value::Boolean(complete)),
            (
                "completeness".to_string(),
                enum_value("Completeness", completeness),
            ),
            ("solutions".to_string(), solutions),
            ("families".to_string(), Value::Vector(Vec::new())),
            (
                "common_conditions".to_string(),
                Value::Vector(match &condition {
                    Some(c) => vec![Value::Symbolic(c.clone())],
                    None => Vec::new(),
                }),
            ),
            (
                "represented_count".to_string(),
                Value::Integer(Int::from_i64(represented as i64)),
            ),
            (
                "unrepresented_count".to_string(),
                Value::Integer(Int::from_i64(unrepresented as i64)),
            ),
            (
                "unrepresented_reason".to_string(),
                Value::Text(unrepresented_reason.to_string()),
            ),
            ("diagnostics".to_string(), diagnostics),
        ],
    )
}

/// The live capability statement behind capabilities(): the domains the solver's domain argument
/// accepts and refuses, every refusal class the kernel publishes, and how exact the statement is.
/// The table is DATA transcribed from the reference's registry (C# SymbolicsPlugin.CapabilitiesRecord).
fn capabilities_record() -> Value {
    fn capability(
        operation_class: &str,
        code: &str,
        category: &str,
        message: &str,
        trigger: &str,
        scope: Option<&str>,
    ) -> Value {
        Value::Record(
            "UnsupportedCapability".to_string(),
            vec![
                (
                    "operation_class".to_string(),
                    Value::Text(operation_class.to_string()),
                ),
                ("code".to_string(), Value::Text(code.to_string())),
                (
                    "category".to_string(),
                    enum_value("ErrorCategory", category),
                ),
                ("message".to_string(), Value::Text(message.to_string())),
                ("trigger".to_string(), Value::Text(trigger.to_string())),
                (
                    "scope".to_string(),
                    match scope {
                        Some(text) => Value::Text(text.to_string()),
                        None => Value::Void,
                    },
                ),
            ],
        )
    }
    Value::Record(
        "CapabilitiesResult".to_string(),
        vec![
            (
                "solve_domains_accepted".to_string(),
                Value::Vector(vec![
                    Value::Domain("real".to_string()),
                    Value::Domain("complex".to_string()),
                ]),
            ),
            (
                "solve_domains_refused".to_string(),
                Value::Vector(vec![
                    Value::Domain("integer".to_string()),
                    Value::Domain("rational".to_string()),
                ]),
            ),
            (
                "unsupported_operations".to_string(),
                Value::Vector(vec![
                    capability(
                        "pow.non-integer-exponent-of-a-positive-base",
                        "UnsupportedOperation",
                        "UnsupportedOperation",
                        "Non-integer exponents are not yet supported.",
                        "2^(1/2)",
                        Some(
                            "Refused for a POSITIVE base: 2^(1/2), 4^(1/2), 9^(1/2), 0.5^(1/2) and 2^(-1/2) all fail. The message is the kernel's own and is wider than the refusal: an exact NEGATIVE base under the principal square root IS supported — (-4)^(1/2) = 2*i, (-9)^(1/2) = 3*i, (-2)^(1/2) = i*sqrt(2) — and a ZERO base is supported too (0^(1/2) = 0).",
                        ),
                    ),
                    capability(
                        "pow.negative-base-unrepresentable-exponent",
                        "UnsupportedOperation",
                        "UnsupportedOperation",
                        "Non-integer exponents are not yet supported.",
                        "(-8)^(1/3)",
                        Some(
                            "Refused for a NEGATIVE base with any non-integer exponent OTHER than the principal square root: (-8)^(1/3), (-4)^(1/4), (-4)^(2/3), (-4)^(3/2) and (-4)^(-1/2) all fail, while (-4)^(1/2) is supported (the entry above).",
                        ),
                    ),
                    capability(
                        "solve.unsupported-domain",
                        "InvalidOperation",
                        "DomainError",
                        "solve(): currently supports domains real and complex; got integer.",
                        "x = symbol(\"x\"); solve(x^2 - 2 == 0, x, integer)",
                        None,
                    ),
                    capability(
                        "rootof.complex-algebraic",
                        "solve.unrepresented-roots",
                        "UnsupportedOperation",
                        "complex algebraic roots not supported (RootOf is real-only in v1).",
                        "x = symbol(\"x\"); solve_full(x^4 - x^2 - 1 == 0, x)",
                        None,
                    ),
                    capability(
                        "limit.unevaluated-in-record-diagnostics",
                        "limit.unevaluated",
                        "UnsupportedOperation",
                        "coefficient does not evaluate at the point",
                        "x = symbol(\"x\"); limit_full(sin(x), x, inf)",
                        None,
                    ),
                    capability(
                        "integration.unevaluated-in-record-diagnostics",
                        "integration.unevaluated",
                        "UnsupportedOperation",
                        "No closed form was found, so the integral is returned unevaluated; the reason for the refusal is carried in this diagnostic's details.",
                        "x = symbol(\"x\"); integrate_full(exp(x^2), x)",
                        None,
                    ),
                    capability(
                        "plot.symbolic-expression",
                        "InvalidOperation",
                        "DomainError",
                        "plot() argument 1 must be a vector, but got 'Symbolic'.",
                        "x = symbol(\"x\"); plot(sin(x))",
                        None,
                    ),
                    capability(
                        "plot.symbolic-element",
                        "InvalidOperation",
                        "DomainError",
                        "Cannot convert value of kind 'Symbolic' to a number for plotting.",
                        "x = symbol(\"x\"); plot([x, 1, 2])",
                        None,
                    ),
                    capability(
                        "solve.non-symbolic-variable",
                        "InvalidOperation",
                        "DomainError",
                        "solve(): argument 2 must be a symbolic variable; got Natural.",
                        "x = symbol(\"x\"); solve(x^2 - 2 == 0, 1)",
                        None,
                    ),
                    capability(
                        "solve.non-symbolic-expression",
                        "InvalidOperation",
                        "DomainError",
                        "solve(): argument 1 must be a symbolic expression; got Vector.",
                        "x = symbol(\"x\"); solve([x == 1], x)",
                        None,
                    ),
                    capability(
                        "diff.non-symbolic-variable",
                        "InvalidOperation",
                        "DomainError",
                        "diff(): argument 2 must be a symbolic variable; got Natural.",
                        "x = symbol(\"x\"); diff(x^2, 1)",
                        None,
                    ),
                    capability(
                        "integrate.non-symbolic-variable",
                        "InvalidOperation",
                        "DomainError",
                        "integrate(): argument 2 must be a symbolic variable; got Natural.",
                        "x = symbol(\"x\"); integrate(x^2, 1)",
                        None,
                    ),
                    capability(
                        "limit.non-symbolic-variable",
                        "InvalidOperation",
                        "DomainError",
                        "limit(): argument 2 must be a symbolic variable; got Natural.",
                        "x = symbol(\"x\"); limit(sin(x), 1, 0)",
                        None,
                    ),
                    capability(
                        "dsp.symbolic-element",
                        "InvalidOperation",
                        "DomainError",
                        "DSP builtins expect numeric/complex elements, but got 'SymbolExpr'.",
                        "x = symbol(\"x\"); dft([x, 1, 2, 3])",
                        None,
                    ),
                    capability(
                        "linsolve.non-symbolic-matrix",
                        "InvalidOperation",
                        "DomainError",
                        "linsolve() requires a symbolic matrix A.",
                        "A = [[1,2],[2,4]]; b = [[1],[3]]; linsolve(A, b)",
                        None,
                    ),
                    capability(
                        "fft.non-power-of-two-length",
                        "InvalidArgument",
                        "TypeMismatch",
                        "FFT length must be a power of two, but got 3. (Parameter 'x')",
                        "x = symbol(\"x\"); fft([1,2,3])",
                        None,
                    ),
                    capability(
                        "solve.unevaluated-in-record-diagnostics",
                        "solve.unevaluated",
                        "UnsupportedOperation",
                        "0 = 0: every value is a solution.",
                        "x = symbol(\"x\"); solve_full(x == x, x)",
                        None,
                    ),
                    capability(
                        "system-solve.unevaluated-in-record-diagnostics",
                        "system-solve.unevaluated",
                        "UnsupportedOperation",
                        "system-solve.elimination-incomplete: the Gröbner basis has no polynomial in y alone, so the values of y are not enumerated here.",
                        "x = symbol(\"x\"); y = symbol(\"y\"); solve_system_full([x + y == 1], [x, y])",
                        None,
                    ),
                    capability(
                        "integration.no-closed-form-in-record-diagnostics",
                        "integration.no-closed-form",
                        "UnsupportedOperation",
                        "no integration tier produced a candidate closed form for integrate(exp(x^2), x)",
                        "x = symbol(\"x\"); integrate_full(exp(x^2), x)",
                        None,
                    ),
                ]),
            ),
            (
                "exactness".to_string(),
                enum_value("CapabilitiesExactness", "BestEffort"),
            ),
        ],
    )
}

/// A declared enumeration variant: the type name and the variant name are both on the wire.
fn enum_value(type_name: &str, variant: &str) -> Value {
    Value::Enum(type_name.to_string(), variant.to_string())
}

/// The matrix result a singular coefficient matrix produces: no solution at all, which is a
/// COMPLETE answer rather than a partial one.
fn matrix_no_solutions(record: &str, payload_field: &str) -> Value {
    Value::Record(
        record.to_string(),
        vec![
            (
                "status".to_string(),
                enum_value("SolveStatus", "NoSolutions"),
            ),
            ("complete".to_string(), Value::Boolean(true)),
            (
                "completeness".to_string(),
                enum_value("Completeness", "Complete"),
            ),
            (payload_field.to_string(), Value::Vector(Vec::new())),
            ("conditions".to_string(), Value::Vector(Vec::new())),
            (
                "diagnostics".to_string(),
                Value::Vector(vec![diagnostic(
                    "matrix.singular",
                    "NoSolution",
                    "matrix is singular",
                )]),
            ),
        ],
    )
}

/// A Diagnostic record: the vocabulary every rich record's diagnostics array carries.
/// The reference's refusal for an assumption that contradicts the session: the sentence is the
/// diagnostic consumers read, and the code/category say the branch has no model.
fn unsatisfiable_assumptions(message: String) -> LangError {
    LangError::coded(message, 0, "UnsatisfiableAssumptions", "DomainError")
}

/// The human rendering one assumption atom publishes in assumptions().display.
fn assumption_text(atom: &Assumption) -> String {
    atom.describe()
}

/// One assumption atom as its structured condition leaf: a relation stays a symbolic relation, a
/// predicate with no relation form becomes an explicit FiniteCondition/PredicateCondition record, a domain
/// restriction becomes DomainCondition, an interval becomes IntervalCondition.
fn assumption_condition_leaf(atom: &Assumption) -> Value {
    match atom {
        Assumption::SymbolRelation(symbol, op, bound) => Value::Symbolic(Sym::Rel(
            *op,
            Box::new(Sym::sym(symbol)),
            Box::new(bound.clone()),
        )),
        Assumption::SymbolProperty(symbol, predicate) => {
            predicate_leaf(Some(Sym::sym(symbol)), *predicate)
        }
        Assumption::ExpressionProperty(expression, predicate) => {
            predicate_leaf(Some(expression.clone()), *predicate)
        }
        Assumption::SymbolDomain(symbol, domain) => Value::Record(
            "DomainCondition".to_string(),
            vec![
                ("variable".to_string(), Value::Symbolic(Sym::sym(symbol))),
                (
                    "domain".to_string(),
                    Value::Domain(domain.name().to_string()),
                ),
            ],
        ),
        Assumption::Interval(expression, lower, lower_open, upper, upper_open) => Value::Record(
            "IntervalCondition".to_string(),
            vec![
                (
                    "expression".to_string(),
                    Value::Symbolic(expression.clone()),
                ),
                (
                    "lower".to_string(),
                    lower.clone().map(Value::Symbolic).unwrap_or(Value::Void),
                ),
                ("lower_open".to_string(), Value::Boolean(*lower_open)),
                (
                    "upper".to_string(),
                    upper.clone().map(Value::Symbolic).unwrap_or(Value::Void),
                ),
                ("upper_open".to_string(), Value::Boolean(*upper_open)),
            ],
        ),
    }
}

/// A predicate leaf: the equivalent relation when one exists (NonZero => x != 0), otherwise an
/// explicit predicate record, so the payload never degrades to text.
fn predicate_leaf(subject: Option<Sym>, predicate: SymbolPredicate) -> Value {
    let Some(subject) = subject else {
        return Value::Void;
    };
    if let Some(op) = Assumption::relation_op(predicate) {
        return Value::Symbolic(Sym::Rel(op, Box::new(subject), Box::new(Sym::num(0))));
    }
    if predicate == SymbolPredicate::Finite {
        return Value::Record(
            "FiniteCondition".to_string(),
            vec![("expression".to_string(), Value::Symbolic(subject))],
        );
    }
    Value::Record(
        "PredicateCondition".to_string(),
        vec![
            (
                "predicate".to_string(),
                Value::Text(predicate.name().to_string()),
            ),
            ("expression".to_string(), Value::Symbolic(subject)),
        ],
    )
}

/// Projects the engine's active assumptions that constrain the symbols free in an expression, as
/// the same structured condition leaves assumptions() publishes.
fn relevant_assumption_leaves(engine: &Engine, expression: &Sym) -> Vec<Value> {
    let names = expression.free_symbols();
    if names.is_empty() {
        return Vec::new();
    }
    let relevant: Vec<Assumption> = engine
        .assumptions()
        .atoms()
        .iter()
        .filter(|atom| names.iter().any(|name| atom.mentions(name)))
        .cloned()
        .collect();
    if relevant.is_empty() {
        return Vec::new();
    }
    lovelace_symbolics::assumptions::AssumptionSet::from_atoms(relevant)
        .atoms()
        .iter()
        .map(assumption_condition_leaf)
        .collect()
}

fn diagnostic(code: &str, category: &str, message: &str) -> Value {
    Value::Record(
        "Diagnostic".to_string(),
        vec![
            ("code".to_string(), Value::Text(code.to_string())),
            (
                "category".to_string(),
                enum_value("ErrorCategory", category),
            ),
            ("message".to_string(), Value::Text(message.to_string())),
            ("recoverable".to_string(), Value::Boolean(true)),
            ("location".to_string(), Value::Void),
            ("details".to_string(), Value::Vector(Vec::new())),
        ],
    )
}

fn factor_expr(e: &Sym) -> Sym {
    if lovelace_symbolics::cancel::check() {
        return e.clone();
    }
    // Single-variable polynomial factorisation over the rationals, when it applies.
    let mut var_name: Option<String> = None;
    collect_symbols(e, &mut var_name);
    let Some(var) = var_name else {
        return e.clone();
    };
    let Some(coeffs) = lovelace_symbolics::poly_coeffs(e, &var) else {
        return e.clone();
    };
    if coeffs.len() < 3 {
        return e.clone();
    }
    let outcome = lovelace_symbolics::solve_polynomial(&coeffs);
    let roots = match outcome {
        lovelace_symbolics::SolveOutcome::Roots(r) => r,
        _ => return e.clone(),
    };
    // A repeated root is one FACTOR with a multiplicity: x^2 - 2x + 1 = (x - 1)^2, never a list
    // of duplicate binomials.
    let total: u64 = roots.iter().map(|root| root.multiplicity).sum();
    if total + 1 != coeffs.len() as u64 {
        return e.clone();
    }
    let mut factors: Vec<Sym> = Vec::new();
    let mut sorted = roots.clone();
    sorted.sort_by(|a, b| b.value.cmp(&a.value));
    for r in sorted {
        let binomial = Sym::add(Sym::sym(&var), Sym::Num(r.value.neg()));
        if r.multiplicity == 1 {
            factors.push(binomial);
        } else {
            factors.push(Sym::pow(binomial, Sym::num(r.multiplicity as i64)));
        }
    }
    // Leading coefficient, when it is not one.
    let leading = coeffs
        .last()
        .cloned()
        .unwrap_or_else(lovelace_symbolics::Rat::one);
    let mut out: Vec<Sym> = Vec::new();
    if !leading.is_one() {
        out.push(Sym::Num(leading));
    }
    out.extend(factors);
    Sym::Mul(out)
}

/// Splits `num * den^-1` into its two parts.
fn quotient_parts(e: &Sym) -> Option<(Sym, Sym)> {
    if let Sym::Pow(base, exp) = e {
        if let Sym::Num(n) = exp.as_ref() {
            if n.to_string() == "-1" {
                return Some((Sym::num(1), (**base).clone()));
            }
        }
    }
    let factors = match e {
        Sym::Mul(f) => f.clone(),
        _ => return None,
    };
    let mut numerator: Vec<Sym> = Vec::new();
    let mut denominator: Option<Sym> = None;
    for f in factors {
        if let Sym::Pow(base, exp) = &f {
            if let Sym::Num(n) = exp.as_ref() {
                if n.to_string() == "-1" {
                    denominator = Some((**base).clone());
                    continue;
                }
            }
        }
        numerator.push(f);
    }
    let den = denominator?;
    let num = match numerator.len() {
        0 => Sym::num(1),
        1 => numerator.pop().unwrap(),
        _ => Sym::Mul(numerator),
    };
    Some((num, den))
}

/// Decides a relation against the engine's assumption set: True only when the set proves it,
/// False only when the set proves its negation, and undecided otherwise (never a guess).
fn decide_relation(
    engine: &Engine,
    op: lovelace_symbolics::RelOp,
    var: &str,
    value: &lovelace_symbolics::Rat,
) -> Option<bool> {
    let query = Assumption::SymbolRelation(var.to_string(), op, Sym::Num(value.clone()));
    match engine.assumptions().ask(&query) {
        Tristate::True => Some(true),
        Tristate::False => Some(false),
        Tristate::Unknown | Tristate::Unsatisfiable => None,
    }
}

fn node_count(e: &Sym) -> u64 {
    match e {
        Sym::Num(_) | Sym::Real(_) | Sym::Sym(_) => 1,
        Sym::Add(v) | Sym::Mul(v) => 1 + v.iter().map(node_count).sum::<u64>(),
        Sym::Pow(a, b) => 1 + node_count(a) + node_count(b),
        Sym::Fun(_, args) => 1 + args.iter().map(node_count).sum::<u64>(),
        Sym::Div(a, b) => 1 + node_count(a) + node_count(b),
        Sym::Rel(_, a, b) => 1 + node_count(a) + node_count(b),
    }
}

fn collect_all_symbols(e: &Sym, out: &mut Vec<String>) {
    match e {
        Sym::Sym(s) => {
            if !out.iter().any(|x| x == s) {
                out.push(s.clone());
            }
        }
        Sym::Add(v) | Sym::Mul(v) => {
            for x in v {
                collect_all_symbols(x, out);
            }
        }
        Sym::Pow(a, b) => {
            collect_all_symbols(a, out);
            collect_all_symbols(b, out);
        }
        Sym::Fun(_, args) => {
            for a in args {
                collect_all_symbols(a, out);
            }
        }
        Sym::Div(a, b) => {
            collect_all_symbols(a, out);
            collect_all_symbols(b, out);
        }
        Sym::Rel(_, a, b) => {
            collect_all_symbols(a, out);
            collect_all_symbols(b, out);
        }
        Sym::Num(_) | Sym::Real(_) => {}
    }
}

fn collect_symbols(e: &Sym, found: &mut Option<String>) {
    match e {
        Sym::Sym(s) => {
            if found.is_none() {
                *found = Some(s.clone());
            }
        }
        Sym::Add(v) | Sym::Mul(v) => {
            for x in v {
                collect_symbols(x, found);
            }
        }
        Sym::Pow(a, b) => {
            collect_symbols(a, found);
            collect_symbols(b, found);
        }
        Sym::Div(a, b) => {
            collect_symbols(a, found);
            collect_symbols(b, found);
        }
        Sym::Fun(_, args) => {
            for a in args {
                collect_symbols(a, found);
            }
        }
        Sym::Rel(_, a, b) => {
            collect_symbols(a, found);
            collect_symbols(b, found);
        }
        Sym::Num(_) | Sym::Real(_) => {}
    }
}

/// A small, honest simplifier: it only rewrites what an active assumption licenses.
pub fn simplify(engine: &Engine, e: Sym) -> Sym {
    // The safe simplifier walks the whole tree bottom-up, rebuilding each sum and product through
    // the canonical adder; a cancelled walk returns the node it was given, which the evaluation
    // discards in favour of the structured Cancelled refusal.
    if lovelace_symbolics::cancel::check() {
        return e;
    }
    match e.clone() {
        Sym::Fun(name, args) if name == "sqrt" && args.len() == 1 => {
            if let Sym::Pow(base, exp) = &args[0] {
                if let Sym::Num(n) = exp.as_ref() {
                    if n.to_string() == "2" {
                        // sqrt(x^2) = x when x is provably nonnegative, abs(x) when x is
                        // provably real, and stays unevaluated otherwise: the identity without a
                        // sign premise is the domain-conditional rule, and firing it unguarded
                        // would define sqrt((-1)^2) as -1.
                        if engine.assumptions().ask(&Assumption::ExpressionProperty(
                            (**base).clone(),
                            SymbolPredicate::NonNegative,
                        )) == Tristate::True
                        {
                            return (**base).clone();
                        }
                        if engine.assumptions().provably_real(base) {
                            return Sym::Fun("abs".to_string(), vec![(**base).clone()]);
                        }
                    }
                }
            }
            Sym::Fun(
                name,
                args.into_iter().map(|a| simplify(engine, a)).collect(),
            )
        }
        Sym::Fun(name, args) if name == "exp" && args.len() == 1 => {
            if let Sym::Fun(inner, inner_args) = &args[0] {
                if inner == "log" && inner_args.len() == 1 {
                    if let Sym::Sym(sym) = &inner_args[0] {
                        if engine.assumptions().ask(&Assumption::SymbolProperty(
                            sym.clone(),
                            SymbolPredicate::Positive,
                        )) == Tristate::True
                        {
                            return Sym::sym(sym);
                        }
                    }
                }
            }
            Sym::Fun(
                name,
                args.into_iter().map(|a| simplify(engine, a)).collect(),
            )
        }
        Sym::Fun(name, args) if name == "log" && args.len() == 1 => {
            if let Sym::Fun(inner, inner_args) = &args[0] {
                if inner == "exp"
                    && inner_args.len() == 1
                    && engine.assumptions().provably_real(&inner_args[0])
                {
                    // log(exp(x)) = x only for a real x: over the complexes the identity fails by
                    // the imaginary period, so the unconditioned form stays unevaluated.
                    return inner_args[0].clone();
                }
            }
            Sym::Fun(
                name,
                args.into_iter().map(|a| simplify(engine, a)).collect(),
            )
        }
        Sym::Pow(base, exp) if is_square_exp(&exp) => {
            if let Sym::Fun(name, args) = base.as_ref() {
                if name == "abs" && args.len() == 1 && engine.assumptions().provably_real(&args[0])
                {
                    // abs(x)^2 = x^2 only for a real x: over C, |z|^2 is not z^2.
                    return Sym::Pow(Box::new(args[0].clone()), exp);
                }
            }
            Sym::Pow(base, exp)
        }
        Sym::Add(terms) => {
            let simplified: Vec<Sym> = terms.into_iter().map(|t| simplify(engine, t)).collect();
            // sin(x)^2 + cos(x)^2 = 1
            let mut sin_sq = None;
            let mut cos_sq = None;
            for t in &simplified {
                if let Sym::Pow(base, exp) = t {
                    if let Sym::Num(n) = exp.as_ref() {
                        if n.to_string() == "2" {
                            if let Sym::Fun(fname, fargs) = base.as_ref() {
                                if fargs.len() == 1 {
                                    if fname == "sin" {
                                        sin_sq = Some(fargs[0].clone());
                                    }
                                    if fname == "cos" {
                                        cos_sq = Some(fargs[0].clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if let (Some(a), Some(b)) = (sin_sq, cos_sq) {
                if a == b && simplified.len() == 2 {
                    return Sym::num(1);
                }
            }
            let mut acc = Sym::num(0);
            for t in simplified {
                acc = Sym::add(acc, t);
            }
            acc
        }
        Sym::Mul(factors) => {
            let simplified: Vec<Sym> = factors.into_iter().map(|t| simplify(engine, t)).collect();
            // x * x^-1 cancels to 1 only when x is known nonzero.
            let mut kept: Vec<Sym> = Vec::new();
            for f in simplified {
                let mut consumed = false;
                if let Sym::Pow(base, exp) = &f {
                    if let Sym::Num(n) = exp.as_ref() {
                        if n.to_string() == "-1" {
                            if let Sym::Sym(name) = base.as_ref() {
                                if engine.assumptions().ask(&Assumption::SymbolProperty(
                                    name.clone(),
                                    SymbolPredicate::NonZero,
                                )) == Tristate::True
                                    && kept.iter().any(|k| *k == Sym::sym(name))
                                {
                                    kept.retain(|k| *k != Sym::sym(name));
                                    consumed = true;
                                }
                            }
                        }
                    }
                }
                if !consumed {
                    kept.push(f);
                }
            }
            let mut acc = Sym::num(1);
            for f in kept {
                acc = Sym::mul(acc, f);
            }
            acc
        }
        other => other,
    }
}

/// One applied rewrite: the stable rule id, the rule's declared classification, the node it
/// rewrote, and the conditions the rewrite requires. Provenance, never a second policy.
struct RewriteStepData {
    /// Stable rewrite-rule id (the reference's registry id).
    rule_id: &'static str,
    /// The rule's declared classification: Universal, Conditional or DomainSpecific.
    classification: &'static str,
    /// The sub-expression the rule rewrote.
    before: Sym,
    /// The sub-expression it produced.
    after: Sym,
    /// Conditions the rewrite requires (empty when none). A condition is a Value because the
    /// reference publishes `FiniteCondition` RECORDS alongside plain relational expressions.
    required_conditions: Vec<Value>,
}

/// One RewriteStep wire record.
fn rewrite_step_value(step: RewriteStepData) -> Value {
    Value::Record(
        "RewriteStep".to_string(),
        vec![
            ("rule_id".to_string(), Value::Text(step.rule_id.to_string())),
            (
                "classification".to_string(),
                enum_value("RuleClassification", step.classification),
            ),
            ("before".to_string(), Value::Symbolic(step.before)),
            ("after".to_string(), Value::Symbolic(step.after)),
            (
                "required_conditions".to_string(),
                Value::Vector(step.required_conditions),
            ),
        ],
    )
}

/// The TransformResult record simplify_full publishes: status, original, expression, changed,
/// the conditions the result requires, the rule steps that produced it, budget state, diagnostics.
fn transform_result(
    engine: &Engine,
    original: Sym,
    expression: Sym,
    conditions: Vec<Value>,
    steps: Vec<RewriteStepData>,
    budget_exceeded: bool,
) -> Value {
    let changed = original != expression;
    // A rewrite that fired is only valid where its conditions can hold. When the session's
    // assumptions already deny them, the branch has no model and the result says so with the
    // sticky Unsatisfiable marker instead of a Satisfied answer over a contradiction.
    let unsatisfiable = !conditions_are_satisfiable(engine, &conditions);
    let (status, conditions, diagnostics) = if unsatisfiable {
        (
            "Unsatisfiable",
            vec![Value::Record(
                "UnsatisfiableConditions".to_string(),
                Vec::new(),
            )],
            vec![unsatisfiable_conditions_diagnostic()],
        )
    } else if budget_exceeded {
        (
            "BudgetExceeded",
            conditions,
            vec![diagnostic(
                "transform.budget-exceeded",
                "BudgetExceeded",
                "the rewrite budget 'rewrite_steps' was exhausted; the returned expression is a partial result.",
            )],
        )
    } else {
        ("Satisfied", conditions, Vec::new())
    };
    Value::Record(
        "TransformResult".to_string(),
        vec![
            ("status".to_string(), enum_value("TransformStatus", status)),
            ("original".to_string(), Value::Symbolic(original)),
            ("expression".to_string(), Value::Symbolic(expression)),
            ("changed".to_string(), Value::Boolean(changed)),
            ("conditions".to_string(), Value::Vector(conditions)),
            (
                "steps".to_string(),
                Value::Vector(steps.into_iter().map(rewrite_step_value).collect()),
            ),
            (
                "budget_exceeded".to_string(),
                Value::Boolean(budget_exceeded),
            ),
            (
                "budget_kind".to_string(),
                Value::Text(if budget_exceeded {
                    "rewrite_steps".to_string()
                } else {
                    String::new()
                }),
            ),
            ("diagnostics".to_string(), Value::Vector(diagnostics)),
        ],
    )
}

/// True when the engine's assumptions plus the rewrite's conditions have a model. Conditions are
/// only decided when they are a symbol-relation atom; anything else adds nothing, so an
/// undecidable condition is never reported as a contradiction.
fn conditions_are_satisfiable(engine: &Engine, conditions: &[Value]) -> bool {
    let mut set = engine.assumptions().clone();
    if set.is_unsatisfiable() {
        return false;
    }
    for condition in conditions {
        let Value::Symbolic(Sym::Rel(op, lhs, rhs)) = condition else {
            continue;
        };
        let Sym::Sym(symbol) = lhs.as_ref() else {
            continue;
        };
        match set.add(Assumption::SymbolRelation(
            symbol.clone(),
            *op,
            (**rhs).clone(),
        )) {
            Ok(next) => set = next,
            Err(_) => return false,
        }
    }
    true
}

/// The DomainError the Unsatisfiable status carries: the branch no model, so the rewrite is not a
/// value of this session.
fn unsatisfiable_conditions_diagnostic() -> Value {
    Value::Record(
        "Diagnostic".to_string(),
        vec![
            (
                "code".to_string(),
                Value::Text("transform.unsatisfiable-conditions".to_string()),
            ),
            (
                "category".to_string(),
                enum_value("ErrorCategory", "DomainError"),
            ),
            (
                "message".to_string(),
                Value::Text(
                    "the applied rules require contradictory conditions, so this branch has no model."
                        .to_string(),
                ),
            ),
            ("recoverable".to_string(), Value::Boolean(false)),
            ("location".to_string(), Value::Void),
            ("details".to_string(), Value::Vector(Vec::new())),
        ],
    )
}

/// The result's `conditions` array: every condition a fired rule required, in first-application
/// order and without duplicates (the reference unions the applied rules' assumption sets).
fn step_conditions(steps: &[RewriteStepData]) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::new();
    for step in steps {
        for condition in &step.required_conditions {
            let text = condition.display();
            if !out.iter().any(|existing| existing.display() == text) {
                out.push(condition.clone());
            }
        }
    }
    out
}

/// True when an exponent node is the integer two.
fn is_square_exp(exp: &Sym) -> bool {
    matches!(exp, Sym::Num(n) if n.to_string() == "2")
}

/// simplify_full's traced simplifier: the same rewrites as the safe simplifier, each recorded as
/// a RewriteStep so the TransformResult publishes provenance instead of only the answer.
fn simplify_traced(
    engine: &Engine,
    e: Sym,
    budget: &mut RewriteBudget,
) -> (Sym, Vec<RewriteStepData>) {
    use lovelace_symbolics::RelOp;
    if lovelace_symbolics::cancel::check() {
        return (e, Vec::new());
    }
    // Rewrite the children bottom-up: a step's before/after are the node the rule rewrote.
    let (rebuilt, mut steps) = match e.clone() {
        Sym::Fun(name, args) => {
            let mut steps = Vec::new();
            let mut new_args = Vec::with_capacity(args.len());
            for a in args {
                let (s, mut st) = simplify_traced(engine, a, budget);
                new_args.push(s);
                steps.append(&mut st);
            }
            (Sym::Fun(name, new_args), steps)
        }
        Sym::Add(terms) => {
            let mut steps = Vec::new();
            let mut acc = Sym::num(0);
            for t in terms {
                let (s, mut st) = simplify_traced(engine, t, budget);
                acc = Sym::add(acc, s);
                steps.append(&mut st);
            }
            (acc, steps)
        }
        Sym::Mul(factors) => {
            let mut steps = Vec::new();
            let mut acc = Sym::num(1);
            for f in factors {
                let (s, mut st) = simplify_traced(engine, f, budget);
                acc = Sym::mul(acc, s);
                steps.append(&mut st);
            }
            (acc, steps)
        }
        Sym::Pow(a, b) => {
            let (sa, mut st1) = simplify_traced(engine, *a, budget);
            let (sb, mut st2) = simplify_traced(engine, *b, budget);
            st1.append(&mut st2);
            (Sym::pow(sa, sb), st1)
        }
        Sym::Div(a, b) => {
            let (sa, mut st1) = simplify_traced(engine, *a, budget);
            let (sb, mut st2) = simplify_traced(engine, *b, budget);
            st1.append(&mut st2);
            (Sym::Div(Box::new(sa), Box::new(sb)), st1)
        }
        Sym::Rel(op, a, b) => {
            let (sa, mut st1) = simplify_traced(engine, *a, budget);
            let (sb, mut st2) = simplify_traced(engine, *b, budget);
            st1.append(&mut st2);
            (Sym::Rel(op, Box::new(sa), Box::new(sb)), st1)
        }
        other => (other, Vec::new()),
    };

    // trig: sin(x)^2 + cos(x)^2 -> 1 (universal).
    if let Sym::Add(terms) = &rebuilt {
        let mut sin_sq = None;
        let mut cos_sq = None;
        for t in terms {
            if let Sym::Pow(base, exp) = t {
                if is_square_exp(exp) {
                    if let Sym::Fun(fname, fargs) = base.as_ref() {
                        if fargs.len() == 1 {
                            if fname == "sin" {
                                sin_sq = Some(fargs[0].clone());
                            }
                            if fname == "cos" {
                                cos_sq = Some(fargs[0].clone());
                            }
                        }
                    }
                }
            }
        }
        if let (Some(a), Some(b)) = (sin_sq, cos_sq) {
            if a == b && terms.len() == 2 && budget.step() {
                let after = Sym::num(1);
                steps.push(RewriteStepData {
                    rule_id: "trig.pythagorean-sin2-cos2",
                    classification: "Universal",
                    before: rebuilt.clone(),
                    after: after.clone(),
                    required_conditions: Vec::new(),
                });
                return (after, steps);
            }
        }
    }

    if let Sym::Fun(fname, fargs) = &rebuilt {
        // power: sqrt(w^2) -> w when w is known positive.
        if fname == "sqrt" && fargs.len() == 1 {
            if let Sym::Pow(base, exp) = &fargs[0] {
                if is_square_exp(exp) {
                    if let Sym::Sym(s) = base.as_ref() {
                        if engine.is_positive(s) && budget.step() {
                            let after = Sym::sym(s);
                            let condition =
                                Sym::Rel(RelOp::Ge, Box::new(after.clone()), Box::new(Sym::num(0)));
                            steps.push(RewriteStepData {
                                rule_id: "pow.sqrt-square-nonnegative",
                                classification: "Conditional",
                                before: rebuilt.clone(),
                                after: after.clone(),
                                required_conditions: vec![Value::Symbolic(condition)],
                            });
                            return (after, steps);
                        }
                    }
                }
            }
        }
        // logexp: exp(log(u)) -> u, carrying the FINITE condition on log(u). The reference's
        // full transform applies this conditionally (the safe simplifier requires u > 0 first);
        // the condition is semantics, not provenance, so it is recorded on the step and lifted
        // into the result's conditions array.
        if fname == "exp" && fargs.len() == 1 {
            if let Sym::Fun(inner, inner_args) = &fargs[0] {
                if inner == "log" && inner_args.len() == 1 && budget.step() {
                    let log_expr = fargs[0].clone();
                    let after = inner_args[0].clone();
                    steps.push(RewriteStepData {
                        rule_id: "logexp.exp-log",
                        classification: "Conditional",
                        before: rebuilt.clone(),
                        after: after.clone(),
                        required_conditions: vec![finite_condition_value(log_expr)],
                    });
                    return (after, steps);
                }
            }
        }
    }

    // rat: x * x^-1 -> 1, carrying the non-vanishing condition x != 0.
    if let Sym::Mul(factors) = &rebuilt {
        let mut pair: Option<(usize, usize, String)> = None;
        for i in 0..factors.len() {
            if let Sym::Pow(base, exp) = &factors[i] {
                if let Sym::Num(n) = exp.as_ref() {
                    if n.to_string() == "-1" {
                        if let Sym::Sym(s) = base.as_ref() {
                            if let Some(j) = factors
                                .iter()
                                .position(|f| matches!(f, Sym::Sym(name) if name == s))
                            {
                                if j != i {
                                    pair = Some((i, j, s.clone()));
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
        if let Some((i, j, s)) = pair {
            if budget.step() {
                let mut after = Sym::num(1);
                for (k, f) in factors.iter().enumerate() {
                    if k != i && k != j {
                        after = Sym::mul(after, f.clone());
                    }
                }
                let condition = Sym::Rel(RelOp::Ne, Box::new(Sym::sym(&s)), Box::new(Sym::num(0)));
                steps.push(RewriteStepData {
                    rule_id: "rat.cancel-x-over-x",
                    classification: "Conditional",
                    before: rebuilt.clone(),
                    after: after.clone(),
                    required_conditions: vec![Value::Symbolic(condition)],
                });
                return (after, steps);
            }
        }
    }

    (rebuilt, steps)
}

/// A `FiniteCondition` wire record: the condition that the expression is finite. The reference
/// publishes it as its own record type inside TransformResult conditions and RewriteStep
/// required_conditions.
fn finite_condition_value(expression: Sym) -> Value {
    Value::Record(
        "FiniteCondition".to_string(),
        vec![("expression".to_string(), Value::Symbolic(expression))],
    )
}

/// The reference's rewrite budget: 400 rule applications across the whole transformation.
/// Exhausting it is a BudgetExceeded TransformResult (with a diagnostic), never a truncated
/// traversal or an unbounded recursion.
const REWRITE_MAX_STEPS: usize = 400;

struct RewriteBudget {
    steps: usize,
    exhausted: bool,
}

impl RewriteBudget {
    fn new() -> RewriteBudget {
        RewriteBudget {
            steps: 0,
            exhausted: false,
        }
    }

    /// Consumes one rewrite step. False once the documented budget is spent: no further rule may
    /// fire, and the caller must report BudgetExceeded with the partial expression.
    fn step(&mut self) -> bool {
        if self.steps >= REWRITE_MAX_STEPS {
            self.exhausted = true;
            return false;
        }
        self.steps += 1;
        true
    }

    fn exhausted(&self) -> bool {
        self.exhausted
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
