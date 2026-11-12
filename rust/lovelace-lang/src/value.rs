//! Runtime values, records, complex numbers, and scalar/array arithmetic.

use std::cmp::Ordering;

use lovelace_array::NdArray;
use lovelace_int::Int;
use lovelace_nat::Nat;
use lovelace_real::Real;

use crate::LangError;
use crate::ast::BinOp;

/// A first-class complex number with exact real and imaginary parts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Complex {
    /// The real part.
    pub re: Real,
    /// The imaginary part.
    pub im: Real,
}

impl Complex {
    /// Builds a complex number from its parts.
    pub fn new(re: Real, im: Real) -> Complex {
        Complex { re, im }
    }

    /// The complex zero.
    pub fn zero() -> Complex {
        Complex {
            re: Real::zero(),
            im: Real::zero(),
        }
    }

    /// The magnitude `|z| = sqrt(re^2 + im^2)`.
    pub fn magnitude(&self, digits: usize) -> Real {
        let re2 = self.re.mul(&self.re);
        let im2 = self.im.mul(&self.im);
        re2.add(&im2).sqrt(digits)
    }

    /// Complex addition.
    pub fn add(&self, other: &Complex) -> Complex {
        Complex {
            re: self.re.add(&other.re),
            im: self.im.add(&other.im),
        }
    }

    /// Complex multiplication.
    pub fn mul(&self, other: &Complex) -> Complex {
        Complex {
            re: self.re.mul(&other.re).sub(&self.im.mul(&other.im)),
            im: self.re.mul(&other.im).add(&self.im.mul(&other.re)),
        }
    }

    /// Multiplies by an exact real scalar.
    pub fn scale(&self, factor: &Real) -> Complex {
        Complex {
            re: self.re.mul(factor),
            im: self.im.mul(factor),
        }
    }
}

/// A structured record value, e.g. `DivRemResult(quotient: 3, remainder: 2)`.
#[derive(Clone, Debug)]
pub struct RecordValue {
    /// The declared record type name.
    pub type_name: &'static str,
    /// Fields in declaration order.
    pub fields: Vec<(String, Value)>,
}

impl RecordValue {
    /// Looks up a field by name.
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.fields
            .iter()
            .find(|(field, _)| field == name)
            .map(|(_, value)| value)
    }
}

/// A runtime value.
#[derive(Clone, Debug)]
pub enum Value {
    /// An arbitrary-precision natural number.
    Natural(Nat),
    /// An arbitrary-precision signed integer.
    Integer(Int),
    /// An exact real number.
    Real(Real),
    /// A Boolean.
    Boolean(bool),
    /// Text.
    Text(String),
    /// A vector (rank 1) or N-dimensional array (rank >= 2).
    Array(NdArray<Value>),
    /// A named function reference.
    Function(String),
    /// The absence of a value.
    Void,
    /// A structured record.
    Record(RecordValue),
    /// A complex number.
    Complex(Complex),
}

impl Value {
    /// The value's kind name, as reported by `type()` and typed formatting.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Value::Natural(_) => "Natural",
            Value::Integer(_) => "Integer",
            Value::Real(_) => "Real",
            Value::Boolean(_) => "Boolean",
            Value::Text(_) => "Text",
            Value::Array(array) => {
                if array.rank() == 1 {
                    "Vector"
                } else {
                    "Array"
                }
            }
            Value::Function(_) => "Function",
            Value::Void => "Void",
            Value::Record(record) => record.type_name,
            Value::Complex(_) => "Complex",
        }
    }

    /// The numeric widening rank (Natural 0, Integer 1, Real 2), if numeric.
    pub fn numeric_rank(&self) -> Option<u8> {
        match self {
            Value::Natural(_) => Some(0),
            Value::Integer(_) => Some(1),
            Value::Real(_) => Some(2),
            _ => None,
        }
    }

    /// True for vector and array values.
    pub fn is_array_like(&self) -> bool {
        matches!(self, Value::Array(_))
    }

    /// The underlying array, when this is a vector or array.
    pub fn as_array(&self) -> Option<&NdArray<Value>> {
        match self {
            Value::Array(array) => Some(array),
            _ => None,
        }
    }

    /// Builds a rank-1 vector from elements.
    pub fn vector(elements: Vec<Value>) -> Value {
        let shape = vec![elements.len()];
        Value::Array(NdArray::from_vec(shape, elements).expect("vector length matches its shape"))
    }

    /// Builds an array from a shape and row-major data.
    ///
    /// # Errors
    ///
    /// Returns an error when the shape and data length disagree.
    pub fn array(shape: Vec<usize>, data: Vec<Value>) -> Result<Value, LangError> {
        NdArray::from_vec(shape, data)
            .map(Value::Array)
            .map_err(|_| LangError::new("Array shape does not match its data."))
    }
}

/// Builds an `Int` from an arbitrary-precision natural magnitude.
pub(crate) fn int_from_nat(value: &Nat) -> Int {
    Int::from_decimal(&value.to_decimal()).unwrap_or_else(Int::zero)
}

/// Builds a `Real` from a natural number.
pub(crate) fn real_from_nat(value: &Nat) -> Real {
    Real::from_int(&int_from_nat(value))
}

/// Reports a widening failure with the reference wording.
fn widen_error(from: &str, to: &str) -> LangError {
    LangError::new(format!(
        "Cannot widen from {from} to {to}: only numeric kinds (Natural, Integer, Real) support widening."
    ))
}

fn unsupported(op: BinOp, kind: &str) -> LangError {
    LangError::new(format!(
        "Operator '{}' is not supported for type '{kind}'.",
        op.symbol()
    ))
}

/// The full kind order used when widening a numeric value against a non-numeric one.
fn kind_order(value: &Value) -> u8 {
    match value {
        Value::Natural(_) => 0,
        Value::Integer(_) => 1,
        Value::Real(_) => 2,
        Value::Boolean(_) => 3,
        Value::Text(_) => 4,
        Value::Array(array) if array.rank() == 1 => 5,
        Value::Function(_) => 6,
        Value::Void => 7,
        Value::Array(_) => 8,
        Value::Complex(_) => 9,
        Value::Record(_) => 11,
    }
}

/// Widens a numeric value to the requested rank.
pub(crate) fn widen(value: &Value, target: u8) -> Result<Value, LangError> {
    let rank = value
        .numeric_rank()
        .ok_or_else(|| widen_error(value.kind_name(), kind_name_for_rank(target)))?;
    if rank == target {
        return Ok(value.clone());
    }
    if rank > target {
        return Err(LangError::new(format!(
            "Cannot narrow from {} to {}.",
            value.kind_name(),
            kind_name_for_rank(target)
        )));
    }
    match target {
        1 => match value {
            Value::Natural(nat) => Ok(Value::Integer(int_from_nat(nat))),
            _ => unreachable!(),
        },
        2 => match value {
            Value::Natural(nat) => Ok(Value::Real(real_from_nat(nat))),
            Value::Integer(int) => Ok(Value::Real(Real::from_int(int))),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }
}

fn kind_name_for_rank(rank: u8) -> &'static str {
    match rank {
        0 => "Natural",
        1 => "Integer",
        _ => "Real",
    }
}

/// Widens both operands to the wider numeric kind.
pub(crate) fn widen_pair(left: &Value, right: &Value) -> Result<(Value, Value), LangError> {
    match (left.numeric_rank(), right.numeric_rank()) {
        (Some(a), Some(b)) => {
            let target = a.max(b);
            Ok((widen(left, target)?, widen(right, target)?))
        }
        (Some(_), None) => Err(widen_error(left.kind_name(), right.kind_name())),
        (None, Some(_)) => Err(widen_error(right.kind_name(), left.kind_name())),
        (None, None) => {
            let a = kind_order(left);
            let b = kind_order(right);
            if a == b {
                Ok((left.clone(), right.clone()))
            } else if a < b {
                Err(widen_error(left.kind_name(), right.kind_name()))
            } else {
                Err(widen_error(right.kind_name(), left.kind_name()))
            }
        }
    }
}

/// Raises a numeric value to the requested power of ten.
fn nat_pow10(power: usize) -> Nat {
    Nat::from_u64(10).pow(power as u64)
}

/// Renders a non-negative integer/natural ratio as an exact Real.
fn real_ratio_nat(
    num: &Nat,
    den: &Nat,
    precision: usize,
    negative: bool,
) -> Result<Real, LangError> {
    let value = Real::from_fraction(num, den, precision)
        .ok_or_else(|| LangError::new("Cannot divide by zero."))?;
    Ok(if negative { value.neg() } else { value })
}

/// Converts a natural to a `u64` or reports an overflow.
fn nat_to_u64(value: &Nat, what: &str) -> Result<u64, LangError> {
    value
        .to_u64()
        .ok_or_else(|| LangError::new(format!("{what} is too large.")))
}

/// Converts a natural to a non-negative `usize`.
pub(crate) fn nat_to_usize(value: &Nat) -> Option<usize> {
    let n = value.to_u64()?;
    usize::try_from(n).ok()
}

/// Reads an exact i64 from a Natural or Integer value.
pub(crate) fn exact_i64(value: &Value) -> Option<i64> {
    match value {
        Value::Natural(nat) => {
            let text = nat.to_decimal();
            text.parse::<i64>().ok()
        }
        Value::Integer(int) => int.to_decimal().parse::<i64>().ok(),
        _ => None,
    }
}

/// Applies a scalar binary operator after widening both operands.
pub(crate) fn apply_scalar(
    op: BinOp,
    left: &Value,
    right: &Value,
    precision: usize,
) -> Result<Value, LangError> {
    let (left, right) = widen_pair(left, right)?;
    match (&left, &right) {
        (Value::Natural(a), Value::Natural(b)) => match op {
            BinOp::Add => Ok(Value::Natural(a.add(b))),
            BinOp::Subtract => match a.sub(b) {
                Some(value) => Ok(Value::Natural(value)),
                None => {
                    let a = int_from_nat(a);
                    let b = int_from_nat(b);
                    Ok(Value::Integer(a.sub(&b)))
                }
            },
            BinOp::Multiply => Ok(Value::Natural(a.mul(b))),
            BinOp::Divide => match a.div_rem(b) {
                None => Err(LangError::new("Cannot divide by zero.")),
                Some((quotient, remainder)) if remainder.is_zero() => Ok(Value::Natural(quotient)),
                Some(_) => Ok(Value::Real(real_ratio_nat(a, b, precision, false)?)),
            },
            BinOp::Modulo => match a.div_rem(b) {
                None => Err(LangError::new("Cannot divide by zero.")),
                Some((_, remainder)) => Ok(Value::Natural(remainder)),
            },
            BinOp::Power => {
                let exponent = nat_to_u64(b, "Exponent")?;
                Ok(Value::Natural(a.pow(exponent)))
            }
            _ => Err(unsupported(op, "Natural")),
        },
        (Value::Integer(a), Value::Integer(b)) => match op {
            BinOp::Add => Ok(Value::Integer(a.add(b))),
            BinOp::Subtract => Ok(Value::Integer(a.sub(b))),
            BinOp::Multiply => Ok(Value::Integer(a.mul(b))),
            BinOp::Divide => match a.div_rem_signed(b) {
                None => Err(LangError::new("Cannot divide by zero.")),
                Some((quotient, remainder)) if remainder.is_zero() => Ok(Value::Integer(quotient)),
                Some(_) => {
                    let negative = a.is_negative() ^ b.is_negative();
                    let value = real_ratio_nat(a.magnitude(), b.magnitude(), precision, negative)?;
                    Ok(Value::Real(value))
                }
            },
            BinOp::Modulo => match a.div_rem_signed(b) {
                None => Err(LangError::new("Cannot divide by zero.")),
                Some((_, remainder)) => Ok(Value::Integer(remainder)),
            },
            BinOp::Power => {
                let exponent = a_exponent(b)?;
                if exponent >= 0 {
                    Ok(Value::Integer(a.pow(exponent as u64)))
                } else {
                    let base = a.pow((-exponent) as u64);
                    let base = Real::from_int(&base);
                    let one = Real::from_i64(1);
                    one.div(&base, precision)
                        .map(Value::Real)
                        .ok_or_else(|| LangError::new("Cannot divide by zero."))
                }
            }
            _ => Err(unsupported(op, "Integer")),
        },
        (Value::Real(a), Value::Real(b)) => match op {
            BinOp::Add => Ok(Value::Real(a.add(b))),
            BinOp::Subtract => Ok(Value::Real(a.sub(b))),
            BinOp::Multiply => Ok(Value::Real(a.mul(b))),
            // The Real divide is the reference's Real.Divide, whose zero refusal names the Real
            // path: "Cannot divide a Real by zero." (Real.cs:915). The exact Rational/Natural
            // divide keeps the generic sentence.
            BinOp::Divide => a.div(b, precision).map(Value::Real).ok_or_else(|| {
                LangError::coded(
                    "Cannot divide a Real by zero.",
                    0,
                    "DivisionByZero",
                    "DomainError",
                )
            }),
            BinOp::Power => {
                let exponent = b
                    .to_int_exact()
                    .and_then(|value| value.to_decimal().parse::<i64>().ok())
                    .ok_or_else(|| LangError::new("Real exponents are not supported."))?;
                a.pow_int(exponent)
                    .map(Value::Real)
                    .ok_or_else(|| LangError::new("Cannot divide by zero."))
            }
            _ => Err(unsupported(op, "Real")),
        },
        (Value::Complex(_), _) | (_, Value::Complex(_)) => Err(LangError::new(
            "Operator is not supported for Complex; use re()/im()/conj()/abs() to bridge back to Real.",
        )),
        _ => Err(unsupported(op, left.kind_name())),
    }
}

fn a_exponent(value: &Int) -> Result<i64, LangError> {
    value
        .to_decimal()
        .parse::<i64>()
        .map_err(|_| LangError::new("Exponent is too large."))
}

/// Compares two numeric values exactly.
pub(crate) fn compare_values(left: &Value, right: &Value) -> Result<Ordering, LangError> {
    let (left, right) = widen_pair(left, right)?;
    match (&left, &right) {
        (Value::Natural(a), Value::Natural(b)) => Ok(a.cmp(b)),
        (Value::Integer(a), Value::Integer(b)) => Ok(a.cmp(b)),
        (Value::Real(a), Value::Real(b)) => Ok(a.cmp(b)),
        (Value::Complex(_), _) | (_, Value::Complex(_)) => Err(LangError::new(
            "Cannot compare Complex values; use abs()/re()/im() to compare their Real parts.",
        )),
        _ => Err(LangError::new(format!(
            "Cannot compare values of kind '{}'.",
            left.kind_name()
        ))),
    }
}

/// Applies a comparison operator.
pub(crate) fn apply_comparison(op: BinOp, left: &Value, right: &Value) -> Result<Value, LangError> {
    let ordering = compare_values(left, right)?;
    let result = match op {
        BinOp::Equal => ordering == Ordering::Equal,
        BinOp::NotEqual => ordering != Ordering::Equal,
        BinOp::Greater => ordering == Ordering::Greater,
        BinOp::Less => ordering == Ordering::Less,
        BinOp::GreaterEqual => ordering != Ordering::Less,
        BinOp::LessEqual => ordering != Ordering::Greater,
        _ => false,
    };
    Ok(Value::Boolean(result))
}

/// Arithmetic negation.
pub(crate) fn negate_value(value: &Value) -> Result<Value, LangError> {
    match value {
        Value::Natural(nat) => Ok(Value::Integer(int_from_nat(nat).negate())),
        Value::Integer(int) => Ok(Value::Integer(int.negate())),
        Value::Real(real) => Ok(Value::Real(real.neg())),
        _ => Err(LangError::new(format!(
            "Unary negation is not supported for type '{}'.",
            value.kind_name()
        ))),
    }
}

/// Postfix factorial.
pub(crate) fn factorial_value(value: &Value) -> Result<Value, LangError> {
    match value {
        Value::Natural(nat) => {
            let n = nat_to_u64(nat, "Factorial argument")?;
            Ok(Value::Natural(Nat::factorial(n)))
        }
        Value::Integer(int) => {
            let n = int
                .to_decimal()
                .parse::<u64>()
                .map_err(|_| LangError::new("Factorial is not supported for negative numbers."))?;
            Ok(Value::Integer(Int::factorial(n)))
        }
        Value::Real(_) => Err(LangError::new(
            "Factorial is not supported for Real numbers.",
        )),
        _ => Err(LangError::new(format!(
            "Factorial is not supported for type '{}'.",
            value.kind_name()
        ))),
    }
}

/// Widens a numeric element list to its widest kind, leaving non-numeric lists alone.
pub(crate) fn promote_numeric(elements: &mut [Value]) {
    let mut target: Option<u8> = None;
    for element in elements.iter() {
        match element.numeric_rank() {
            Some(rank) => target = Some(target.map_or(rank, |current| current.max(rank))),
            None => return,
        }
    }
    if let Some(target) = target {
        for element in elements.iter_mut() {
            if let Ok(widened) = widen(element, target) {
                *element = widened;
            }
        }
    }
}

/// Applies the division narrow-back rule: exact integer quotients become Natural/Integer.
pub(crate) fn promote_division(elements: &mut [Value]) {
    let mut all_exact = true;
    let mut all_non_negative = true;
    for element in elements.iter() {
        match element {
            Value::Natural(_) => {}
            Value::Integer(int) => {
                if int.is_negative() {
                    all_non_negative = false;
                }
            }
            Value::Real(real) => {
                if real.is_integer() {
                    if real.cmp(&Real::zero()) == Ordering::Less {
                        all_non_negative = false;
                    }
                } else {
                    all_exact = false;
                    break;
                }
            }
            _ => return,
        }
    }
    if !all_exact {
        promote_numeric(elements);
        return;
    }
    for element in elements.iter_mut() {
        match element {
            Value::Integer(int) if all_non_negative => {
                *element = Value::Natural(int.magnitude().clone());
            }
            Value::Real(real) => {
                if let Some(int) = real.to_int_exact() {
                    *element = if all_non_negative {
                        Value::Natural(int.magnitude().clone())
                    } else {
                        Value::Integer(int)
                    };
                }
            }
            _ => {}
        }
    }
}

fn format_shape(shape: &[usize]) -> String {
    let parts: Vec<String> = shape.iter().map(|dim| dim.to_string()).collect();
    format!("[{}]", parts.join(", "))
}

fn broadcast_shape(a: &[usize], b: &[usize]) -> Result<Vec<usize>, LangError> {
    let rank = a.len().max(b.len());
    let mut result = vec![1usize; rank];
    for dim in 0..rank {
        let sa = if dim < rank - a.len() {
            1
        } else {
            a[dim - (rank - a.len())]
        };
        let sb = if dim < rank - b.len() {
            1
        } else {
            b[dim - (rank - b.len())]
        };
        result[dim] = if sa == sb {
            sa
        } else if sa == 1 {
            sb
        } else if sb == 1 {
            sa
        } else {
            return Err(LangError::new(format!(
                "Operands could not be broadcast together with shapes {} and {}.",
                format_shape(a),
                format_shape(b)
            )));
        };
    }
    Ok(result)
}

fn offset_for(shape: &[usize], coords: &[usize], rank: usize) -> usize {
    let mut offset = 0usize;
    let mut stride = 1usize;
    for axis in (0..shape.len()).rev() {
        let coord = coords[rank - shape.len() + axis];
        let dim = shape[axis];
        let coordinate = if dim == 1 { 0 } else { coord };
        offset += coordinate * stride;
        stride *= dim;
    }
    offset
}

/// Applies arithmetic elementwise with scalar broadcast and right-aligned array broadcast.
pub(crate) fn apply_array_binary(
    op: BinOp,
    left: &Value,
    right: &Value,
    precision: usize,
) -> Result<Value, LangError> {
    match (left.as_array(), right.as_array()) {
        (Some(a), Some(b)) => {
            let a_shape = a.shape().to_vec();
            let b_shape = b.shape().to_vec();
            if a_shape == b_shape {
                let mut data = Vec::with_capacity(a.numel());
                for (x, y) in a.iter().zip(b.iter()) {
                    data.push(apply_scalar(op, x, y, precision)?);
                }
                return wrap_promoted(op, data, a_shape);
            }
            let shape = broadcast_shape(&a_shape, &b_shape)?;
            let numel: usize = shape.iter().product();
            let mut data = Vec::with_capacity(numel);
            let mut coords = vec![0usize; shape.len()];
            for flat in 0..numel {
                let mut remainder = flat;
                for axis in (0..shape.len()).rev() {
                    coords[axis] = if shape[axis] == 0 {
                        0
                    } else {
                        remainder % shape[axis]
                    };
                    if shape[axis] != 0 {
                        remainder /= shape[axis];
                    }
                }
                let a_offset = offset_for(&a_shape, &coords, shape.len());
                let b_offset = offset_for(&b_shape, &coords, shape.len());
                data.push(apply_scalar(
                    op,
                    &a.as_slice()[a_offset],
                    &b.as_slice()[b_offset],
                    precision,
                )?);
            }
            wrap_promoted(op, data, shape)
        }
        (Some(a), None) => {
            let shape = a.shape().to_vec();
            let mut data = Vec::with_capacity(a.numel());
            for element in a.iter() {
                data.push(apply_scalar(op, element, right, precision)?);
            }
            wrap_promoted(op, data, shape)
        }
        (None, Some(b)) => {
            let shape = b.shape().to_vec();
            let mut data = Vec::with_capacity(b.numel());
            for element in b.iter() {
                data.push(apply_scalar(op, left, element, precision)?);
            }
            wrap_promoted(op, data, shape)
        }
        (None, None) => apply_scalar(op, left, right, precision),
    }
}

fn wrap_promoted(op: BinOp, mut data: Vec<Value>, shape: Vec<usize>) -> Result<Value, LangError> {
    if op == BinOp::Divide {
        promote_division(&mut data);
    } else {
        promote_numeric(&mut data);
    }
    Value::array(shape, data)
}

/// Converts a natural into text, used by several builtins.
pub(crate) fn nat_display(value: &Nat) -> String {
    value.to_decimal()
}

/// Renders a power of ten as a natural.
pub(crate) fn pow10(power: usize) -> Nat {
    nat_pow10(power)
}
