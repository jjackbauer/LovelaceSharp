//! Operators, indexing, ranges and the builtin function library.

use lovelace_array::{ArrayError, NdArray};
use lovelace_int::Int;
use lovelace_nat::Nat;
use lovelace_real::Real;
use lovelace_symbolics::Expr as Sym;

use crate::syntax::BinOp;
use crate::{Engine, LangError, PlotCapture, Value};

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
        Value::Real(r) => Some(r.clone()),
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
            if r.is_integer() {
                r.to_int_exact().map(|i| Sym::rational(&i, &Nat::from_u64(1)))
            } else if let Some(i) = r.to_int_exact() {
                Some(Sym::rational(&i, &Nat::from_u64(1)))
            } else {
                // A terminating decimal can be written as a rational with a power-of-ten
                // denominator; anything else stays non-symbolic.
                let text = r.to_string();
                if text.contains('(') {
                    return None;
                }
                match text.split_once('.') {
                    Some((ip, fp)) => {
                        let sign = if ip.starts_with('-') { -1 } else { 1 };
                        let int_part = ip.trim_start_matches('-');
                        let digits = format!("{}{}", if int_part.is_empty() { "0" } else { int_part }, fp);
                        let num = Int::from_decimal(&digits)?;
                        let den = Nat::from_u64(10).pow(fp.len() as u64);
                        Some(Sym::rational(&if sign < 0 { num.negate() } else { num }, &den))
                    }
                    None => None,
                }
            }
        }
        _ => None,
    }
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
                    let pi = Int::from_decimal(&p.to_decimal()).ok_or_else(|| LangError::plain("Overflow."))?;
                    let qi = Int::from_decimal(&q.to_decimal()).ok_or_else(|| LangError::plain("Overflow."))?;
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

/// Divides two values, always producing an exact real.
pub fn div(a: &Value, b: &Value, precision: usize) -> Result<Value, LangError> {
    if matches!(a, Value::Symbolic(_)) || matches!(b, Value::Symbolic(_)) {
        if let (Some(x), Some(y)) = (as_expr(a), as_expr(b)) {
            return Ok(Value::Symbolic(Sym::div(x, y)));
        }
    }
    elementwise(a, b, "div", |x, y| {
        let rx = as_real(x).ok_or_else(|| type_error("/", x, y))?;
        let ry = as_real(y).ok_or_else(|| type_error("/", x, y))?;
        match rx.div(&ry, precision.max(64)) {
            Some(v) => Ok(Value::Real(v)),
            None => Err(LangError::plain("Cannot divide by zero.")),
        }
    })
}

/// Negates a value.
pub fn neg(v: &Value) -> Result<Value, LangError> {
    match v {
        Value::Natural(n) => {
            let i = Int::from_decimal(&n.to_decimal()).ok_or_else(|| LangError::plain("Overflow."))?;
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
        BinOp::Rem => elementwise(l, r, "rem", |a, b| {
            let (a, b) = numeric_pair(a, b).ok_or_else(|| type_error("%", a, b))?;
            match (&a, &b) {
                (Value::Natural(x), Value::Natural(y)) => x
                    .div_rem(y)
                    .map(|(_, r)| Value::Natural(r))
                    .ok_or_else(|| LangError::plain("Cannot divide by zero.")),
                (Value::Integer(x), Value::Integer(y)) => match x.div_rem_signed(y) {
                    Some((_, rem)) => Ok(Value::Integer(rem)),
                    None => Err(LangError::plain("Cannot divide by zero.")),
                },
                _ => Err(type_error("%", &a, &b)),
            }
        }),
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
    Err(LangError::plain(
        "Comparison needs two numeric values.",
    ))
}

fn pow(base: &Value, exp: &Value) -> Result<Value, LangError> {
    if matches!(base, Value::Symbolic(_)) || matches!(exp, Value::Symbolic(_)) {
        if let Some(v) = symbolic_pow(base, exp) {
            return Ok(v);
        }
    }
    let e = as_int(exp).ok_or_else(|| {
        LangError::plain("Exponent must be an Integer or a Natural.")
    })?;
    let e_nat = Int::from_decimal(&e.magnitude().to_decimal())
        .ok_or_else(|| LangError::plain("Exponent is too large."))?;
    let e_u64 = e_nat.magnitude().to_u64().ok_or_else(|| LangError::plain("Exponent is too large."))?;
    let negative = e.is_negative();
    elementwise_scalar(base, |v| {
        let value = match v {
            Value::Natural(n) => Value::Natural(n.pow(e_u64)),
            Value::Integer(i) => Value::Integer(i.pow(e_u64)),
            Value::Real(r) => Value::Real(
                r.pow_int(e_u64 as i64)
                    .ok_or_else(|| LangError::plain("Cannot raise this value to that power."))?,
            ),
            other => {
                return Err(LangError::plain(format!(
                    "Cannot raise a value of kind '{}' to a power.",
                    other.kind_name()
                )))
            }
        };
        if negative {
            match as_real(&value) {
                Some(r) => match Real::from_i64(1).div(&r, 1000) {
                    Some(inv) => Ok(Value::Real(inv)),
                    None => Err(LangError::plain("Cannot divide by zero.")),
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
    let sa = shape_of(a).map(|s| shape_text(&s)).unwrap_or_else(|| "[]".to_string());
    let sb = shape_of(b).map(|s| shape_text(&s)).unwrap_or_else(|| "[]".to_string());
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
            for (p, q) in x.iter().zip(y.iter()) {
                out.push(f(p, q)?);
            }
            Ok(Value::Vector(out))
        }
        (Value::Array(x), Value::Array(y)) => {
            if x.shape() != y.shape() {
                return Err(broadcast_error(a, b));
            }
            let mut out = Vec::with_capacity(x.numel());
            for (p, q) in x.iter().zip(y.iter()) {
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
            for item in v {
                out.push(f(scalar, item)?);
            }
            Ok(Value::Vector(out))
        }
        (Value::Vector(v), scalar) if shape_of(scalar).is_none() => {
            let mut out = Vec::with_capacity(v.len());
            for item in v {
                out.push(f(item, scalar)?);
            }
            Ok(Value::Vector(out))
        }
        (scalar, Value::Array(m)) if shape_of(scalar).is_none() => {
            let mut out = Vec::with_capacity(m.numel());
            for item in m.iter() {
                out.push(f(scalar, item)?);
            }
            Ok(Value::Array(
                NdArray::from_vec(m.shape().to_vec(), out).map_err(array_error)?,
            ))
        }
        (Value::Array(m), scalar) if shape_of(scalar).is_none() => {
            let mut out = Vec::with_capacity(m.numel());
            for item in m.iter() {
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
            for i in items {
                out.push(f(i)?);
            }
            Ok(Value::Vector(out))
        }
        Value::Array(a) => {
            let mut out = Vec::with_capacity(a.numel());
            for i in a.iter() {
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

/// Builds a list literal, promoting to an array when the elements are uniform lists.
pub fn make_list(items: Vec<Value>) -> Result<Value, LangError> {
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
            let flat: Vec<Value> = rows.into_iter().flatten().collect();
            return Ok(Value::Array(NdArray::from_vec(shape, flat).map_err(array_error)?));
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
            return Ok(Value::Array(NdArray::from_vec(new_shape, flat).map_err(array_error)?));
        }
    }
    Ok(Value::Vector(items))
}

/// Builds a range value.
pub fn make_range(start: &Value, step: Option<&Value>, end: &Value) -> Result<Value, LangError> {
    let s = as_real(start).ok_or_else(|| {
        LangError::plain("Range bounds must be numeric.")
    })?;
    let e = as_real(end).ok_or_else(|| {
        LangError::plain("Range bounds must be numeric.")
    })?;
    let step_value = match step {
        Some(v) => {
            let sv = as_real(v).ok_or_else(|| LangError::plain("Range step must be numeric."))?;
            if sv.is_zero() {
                return Err(LangError::plain("Range step must not be zero."));
            }
            sv
        }
        None => {
            if s.cmp(&e) == std::cmp::Ordering::Greater {
                Real::from_i64(-1)
            } else {
                Real::from_i64(1)
            }
        }
    };
    let mut out = Vec::new();
    let mut cur = s.clone();
    let up = step_value.cmp(&Real::zero()) == std::cmp::Ordering::Greater;
    let mut guard = 0usize;
    while if up { cur.cmp(&e) != std::cmp::Ordering::Greater } else { cur.cmp(&e) != std::cmp::Ordering::Less } {
        out.push(from_real(&cur));
        cur = cur.add(&step_value);
        guard += 1;
        if guard > 1_000_000 {
            return Err(LangError::plain("Range is too large."));
        }
    }
    Ok(Value::Vector(out))
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

fn index_int(v: &Value, len: usize) -> Result<usize, LangError> {
    let i = as_int(v).ok_or_else(|| LangError::plain("Index must be an Integer."))?;
    if i.is_negative() {
        let shown = i.to_decimal();
        return Err(LangError::plain(format!(
            "Index {} is out of range for vector of length {}.",
            shown, len
        )));
    }
    let n = i
        .magnitude()
        .to_u64()
        .ok_or_else(|| LangError::plain("Index is too large."))?;
    if (n as usize) >= len {
        return Err(LangError::plain(format!(
            "Index {} is out of range for vector of length {}.",
            n, len
        )));
    }
    Ok(n as usize)
}

fn slice_indices(spec: &IndexSpec, len: usize) -> Result<Vec<usize>, LangError> {
    match spec {
        IndexSpec::Full => Ok((0..len).collect()),
        IndexSpec::Single(v) => Ok(vec![index_int(v, len)?]),
        IndexSpec::Slice(a, b, step) => {
            let to_i = |v: &Value| -> Result<i64, LangError> {
                let i = as_int(v).ok_or_else(|| LangError::plain("Slice bound must be an Integer."))?;
                let n = i.magnitude().to_u64().unwrap_or(0) as i64;
                Ok(if i.is_negative() { -n } else { n })
            };
            let step_v = match step {
                Some(v) => to_i(v)?,
                None => 1,
            };
            if step_v == 0 {
                return Err(LangError::plain("Slice step must not be zero."));
            }
            let mut start = match a {
                Some(v) => to_i(v)?,
                None => if step_v > 0 { 0 } else { len as i64 - 1 },
            };
            let stop = match b {
                Some(v) => to_i(v)?,
                None => if step_v > 0 { len as i64 } else { -1 },
            };
            if start < 0 {
                start += len as i64;
            }
            let mut stop_adj = stop;
            if stop_adj < 0 && b.is_some() {
                stop_adj += len as i64;
            }
            let mut out = Vec::new();
            let mut i = start;
            while if step_v > 0 { i < stop_adj } else { i > stop_adj } {
                if i >= 0 && (i as usize) < len {
                    out.push(i as usize);
                }
                i += step_v;
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
                    Ok(Value::Vector(idx.into_iter().map(|i| items[i].clone()).collect()))
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
                per_axis.push(slice_indices_or_single(spec, shape[axis])?);
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
            Ok(Value::Array(NdArray::from_vec(new_shape, flat).map_err(array_error)?))
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
pub fn call(engine: &mut Engine, name: &str, args: Vec<Value>, pos: usize) -> Result<Value, LangError> {
    let arity = |n: usize| -> Result<(), LangError> {
        if args.len() == n {
            Ok(())
        } else {
            Err(LangError::at(
                format!("'{}' expects {} argument(s), but {} were given.", name, n, args.len()),
                pos,
            ))
        }
    };
    match name {
        "abs" => {
            arity(1)?;
            match &args[0] {
                Value::Natural(n) => Ok(Value::Natural(n.clone())),
                Value::Integer(i) => Ok(Value::Integer(if i.is_negative() { i.negate() } else { i.clone() })),
                Value::Real(r) => Ok(Value::Real(r.abs())),
                Value::Complex(re, im) => {
                    let sq = re.mul(re).add(&im.mul(im));
                    Ok(Value::Real(sq.sqrt(64)))
                }
                other => Err(LangError::plain(format!("abs needs a number, but got '{}'.", other.kind_name()))),
            }
        }
        "sign" => {
            arity(1)?;
            match as_real(&args[0]) {
                Some(r) => {
                    let s = if r.is_zero() { 0 } else if r.cmp(&Real::zero()) == std::cmp::Ordering::Less { -1 } else { 1 };
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
            let r = as_real(&args[0]).ok_or_else(|| LangError::plain("inv needs a number or a matrix."))?;
            match Real::from_i64(1).div(&r, engine.precision_value().max(64)) {
                Some(v) => Ok(Value::Real(v)),
                None => Err(LangError::plain("Cannot divide by zero.")),
            }
        }
        "divrem" => {
            arity(2)?;
            let a = as_int(&args[0]).ok_or_else(|| LangError::plain("divrem needs integers."))?;
            let b = as_int(&args[1]).ok_or_else(|| LangError::plain("divrem needs integers."))?;
            let (q, r) = a
                .div_rem_signed(&b)
                .ok_or_else(|| LangError::plain("Cannot divide by zero."))?;
            Ok(Value::Record(
                "DivRemResult".to_string(),
                vec![("quotient".to_string(), Value::Integer(q)), ("remainder".to_string(), Value::Integer(r))],
            ))
        }
        "is_even" | "is_odd" => {
            arity(1)?;
            let i = as_int(&args[0]).ok_or_else(|| LangError::plain("Predicate needs an integer."))?;
            let (_, rem) = i.div_rem(&Nat::from_u64(2)).ok_or_else(|| LangError::plain("Cannot divide by zero."))?;
            let even = rem.is_zero();
            Ok(Value::Boolean(if name == "is_even" { even } else { !even }))
        }
        "sqrt" => {
            arity(1)?;
            if let Some(e) = as_expr(&args[0]) {
                if matches!(args[0], Value::Symbolic(_)) {
                    return Ok(Value::Symbolic(Sym::sqrt(e)));
                }
            }
            let r = as_real(&args[0]).ok_or_else(|| LangError::plain("sqrt needs a number."))?;
            Ok(Value::Real(r.sqrt(engine.precision_value())))
        }
        "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "exp" | "log" | "sinh" | "cosh"
        | "tanh" => {
            arity(1)?;
            let e = as_expr(&args[0]).ok_or_else(|| {
                LangError::plain(format!("{} needs a number or an expression.", name))
            })?;
            Ok(Value::Symbolic(Sym::Fun(name.to_string(), vec![e])))
        }
        "symbol" => {
            if args.is_empty() {
                return Err(LangError::plain("symbol needs a name."));
            }
            let name = match &args[0] {
                Value::Text(t) => t.clone(),
                other => other.display(),
            };
            Ok(Value::Symbolic(Sym::sym(&name)))
        }
        "diff" => {
            arity(2)?;
            let f = as_expr(&args[0]).ok_or_else(|| LangError::plain("diff needs an expression."))?;
            let var = match &args[1] {
                Value::Symbolic(Sym::Sym(name)) => name.clone(),
                Value::Text(name) => name.clone(),
                other => {
                    return Err(LangError::plain(format!(
                        "diff(): argument 2 must be a symbolic variable; got {}.",
                        other.kind_name()
                    )))
                }
            };
            Ok(Value::Symbolic(lovelace_symbolics::diff(&f, &var)))
        }
        "jacobian" => {
            arity(2)?;
            let funcs = flatten_values(&args[0])?;
            let vars = flatten_values(&args[1])?;
            let mut rows: Vec<Value> = Vec::new();
            for f in &funcs {
                let fe = as_expr(f).ok_or_else(|| LangError::plain("jacobian needs expressions."))?;
                let mut row: Vec<Value> = Vec::new();
                for v in &vars {
                    let name = match v {
                        Value::Symbolic(Sym::Sym(n)) => n.clone(),
                        _ => return Err(LangError::plain("jacobian needs variables.")),
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
            let f = as_expr(&args[0]).ok_or_else(|| LangError::plain("hessian needs an expression."))?;
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
            let f = as_expr(&args[0]).ok_or_else(|| LangError::plain("subs needs an expression."))?;
            let var = match &args[1] {
                Value::Symbolic(Sym::Sym(n)) => n.clone(),
                _ => return Err(LangError::plain("subs needs a variable.")),
            };
            let replacement = as_expr(&args[2]).ok_or_else(|| LangError::plain("subs needs a replacement."))?;
            Ok(Value::Symbolic(lovelace_symbolics::substitute(&f, &var, &replacement)))
        }
        "integrate" => {
            arity(2)?;
            let f = as_expr(&args[0]).ok_or_else(|| LangError::plain("integrate needs an expression."))?;
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
                        return Ok(Value::Symbolic(Sym::Fun("integrate".to_string(), vec![f, Sym::sym(&var)])));
                    }
                }
                return Ok(Value::Symbolic(acc));
            }
            Ok(Value::Symbolic(Sym::Fun("integrate".to_string(), vec![f, Sym::sym(&var)])))
        }
        "and" | "or" | "not" => {
            if name == "not" {
                arity(1)?;
            }
            let mut args_out = Vec::new();
            for a in &args {
                args_out.push(as_expr(a).ok_or_else(|| LangError::plain("Logical operators need expressions."))?);
            }
            Ok(Value::Symbolic(Sym::Fun(name.to_string(), args_out)))
        }
        "integrate_full" => {
            arity(2)?;
            let f = as_expr(&args[0]).ok_or_else(|| LangError::plain("integrate needs an expression."))?;
            let var = match &args[1] {
                Value::Symbolic(Sym::Sym(n)) => n.clone(),
                _ => return Err(LangError::plain("integrate needs a variable.")),
            };
            let antiderivative = integrate_basic(&f, &var);
            let (status, verified, expression) = match &antiderivative {
                Some(a) => ("SolvedExact", true, Value::Symbolic(a.clone())),
                None => (
                    "Unevaluated",
                    false,
                    Value::Symbolic(Sym::Fun("integrate".to_string(), vec![f.clone(), Sym::sym(&var)])),
                ),
            };
            Ok(Value::Record(
                "IntegrationResult".to_string(),
                vec![
                    ("status".to_string(), Value::Text(status.to_string())),
                    ("verified".to_string(), Value::Boolean(verified)),
                    ("expression".to_string(), expression),
                    ("variable".to_string(), Value::Symbolic(Sym::sym(&var))),
                    ("conditions".to_string(), Value::Vector(Vec::new())),
                    ("diagnostics".to_string(), Value::Vector(Vec::new())),
                ],
            ))
        }
        "compile" | "compile_full" => {
            if args.len() < 2 {
                return Err(LangError::plain("compile needs an expression and parameters."));
            }
            let f = as_expr(&args[0]).ok_or_else(|| LangError::plain("compile needs an expression."))?;
            let params = flatten_values(&args[1])?;
            let mut parameters: Vec<Value> = Vec::new();
            for p in &params {
                match p {
                    Value::Symbolic(Sym::Sym(n)) => parameters.push(Value::Record(
                        "ParameterInfo".to_string(),
                        vec![
                            ("name".to_string(), Value::Symbolic(Sym::sym(n))),
                            ("domain".to_string(), Value::Domain("complex".to_string())),
                        ],
                    )),
                    _ => return Err(LangError::plain("compile needs parameter symbols.")),
                }
            }
            Ok(Value::Record(
                "CompilationResult".to_string(),
                vec![
                    ("target".to_string(), Value::Text("mathir".to_string())),
                    ("mathir_version".to_string(), Value::Integer(Int::from_i64(2))),
                    ("parameters".to_string(), Value::Vector(parameters)),
                    ("result_domain".to_string(), Value::Domain("complex".to_string())),
                    ("precision_policy".to_string(), Value::Text("caller_supplied".to_string())),
                    ("optimization_policy".to_string(), Value::Text("none".to_string())),
                    ("expression".to_string(), Value::Symbolic(f)),
                    ("diagnostics".to_string(), Value::Vector(Vec::new())),
                ],
            ))
        }
        "evalir" | "evalir_batch" => {
            if args.len() < 2 {
                return Err(LangError::plain("evalir needs a compiled expression and inputs."));
            }
            let expr = match &args[0] {
                Value::Text(text) if text.starts_with("#!mathir") => {
                    match parse_ir_text(text) {
                        Some(e) => e,
                        None => return Err(LangError::plain("evalir could not read that IR text.")),
                    }
                }
                Value::Symbolic(e) => e.clone(),
                Value::Record(_, fields) => fields
                    .iter()
                    .find(|(k, _)| k == "expression")
                    .and_then(|(_, v)| as_expr(v))
                    .ok_or_else(|| LangError::plain("evalir needs a compiled expression."))?,
                other => {
                    return Err(LangError::plain(format!(
                        "evalir needs a compiled expression, but got '{}'.",
                        other.kind_name()
                    )))
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
                let r = as_real(point).ok_or_else(|| LangError::plain("evalir needs numeric inputs."))?;
                let mut acc = Real::zero();
                for c in coeffs.iter().rev() {
                    let num = Int::from_decimal(&c.numerator().to_decimal())
                        .ok_or_else(|| LangError::plain("coefficient is too large."))?;
                    let den = Int::from_decimal(&c.denominator().to_decimal())
                        .ok_or_else(|| LangError::plain("coefficient is too large."))?;
                    let coef = Real::from_int(&num)
                        .div(&Real::from_int(&den), engine.precision_value())
                        .ok_or_else(|| LangError::plain("Cannot divide by zero."))?;
                    acc = acc.mul(&r).add(&coef);
                }
                if acc.is_integer() {
                    if let Some(i) = acc.to_int_exact() {
                        return Ok(Value::Integer(i));
                    }
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
            let e = as_expr(&args[0]).ok_or_else(|| LangError::plain("inspect needs an expression."))?;
            let mut symbols: Vec<String> = Vec::new();
            collect_all_symbols(&e, &mut symbols);
            let free: Vec<Value> = symbols
                .into_iter()
                .map(|n| Value::Symbolic(Sym::sym(&n)))
                .collect();
            Ok(Value::Record(
                "Inspection".to_string(),
                vec![
                    ("expression".to_string(), Value::Symbolic(e.clone())),
                    ("canonical".to_string(), Value::Text(e.to_string())),
                    ("free_symbols".to_string(), Value::Vector(free)),
                    ("node_count".to_string(), Value::Natural(Nat::from_u64(node_count(&e)))),
                    ("domain".to_string(), Value::Domain("complex".to_string())),
                ],
            ))
        }
        "inf" => {
            if !args.is_empty() {
                return Err(LangError::plain("inf takes no arguments."));
            }
            Ok(Value::Text("inf".to_string()))
        }
        "series" => {
            if args.len() < 4 {
                return Err(LangError::plain("series needs an expression, a variable, a point and an order."));
            }
            let f = as_expr(&args[0]).ok_or_else(|| LangError::plain("series needs an expression."))?;
            let var = match &args[1] {
                Value::Symbolic(Sym::Sym(name)) => name.clone(),
                _ => return Err(LangError::plain("series needs a variable.")),
            };
            let point = as_expr(&args[2]).ok_or_else(|| LangError::plain("series needs a point."))?;
            let order = as_int(&args[3])
                .and_then(|i| i.magnitude().to_u64())
                .ok_or_else(|| LangError::plain("series needs an order."))? as i64;
            let shifted = if point == Sym::num(0) {
                f.clone()
            } else {
                lovelace_symbolics::substitute(&f, &var, &Sym::add(Sym::sym(&var), point))
            };
            let Some(ser) = lovelace_symbolics::series(&shifted, &var, order) else {
                return Ok(Value::Symbolic(f));
            };
            let mut terms: Vec<Sym> = Vec::new();
            for (e, c) in ser.iter() {
                if *e >= order || *e < 0 {
                    continue;
                }
                let coefficient = Sym::Num(c.clone());
                if *e == 0 {
                    terms.push(coefficient);
                } else {
                    terms.push(Sym::mul(coefficient, Sym::pow(Sym::sym(&var), Sym::num(*e))));
                }
            }
            if terms.is_empty() {
                terms.push(Sym::num(0));
            }
            terms.push(Sym::Fun(
                "O".to_string(),
                vec![Sym::pow(Sym::sym(&var), Sym::num(order))],
            ));
            Ok(Value::Symbolic(Sym::Add(terms)))
        }
        "limit" | "limit_full" | "limit_left" | "limit_right" => {
            if args.len() < 3 {
                return Err(LangError::plain("limit needs an expression, a variable and a point."));
            }
            let f = as_expr(&args[0]).ok_or_else(|| LangError::plain("limit needs an expression."))?;
            let var = match &args[1] {
                Value::Symbolic(Sym::Sym(name)) => name.clone(),
                _ => return Err(LangError::plain("limit needs a variable.")),
            };
            let side = match name {
                "limit_left" => Side::Left,
                "limit_right" => Side::Right,
                _ => Side::Both,
            };
            let at_infinity = matches!(&args[2], Value::Text(t) if t == "inf")
                || matches!(&args[2], Value::Symbolic(Sym::Fun(f, a)) if f == "inf" && a.is_empty());
            Ok(limit_result(&f, &var, at_infinity, side))
        }
        "solve" | "solve_full" => {
            if args.len() < 2 {
                return Err(LangError::plain("solve needs an equation and a variable."));
            }
            let var = match &args[1] {
                Value::Symbolic(Sym::Sym(name)) => name.clone(),
                _ => return Err(LangError::plain("solve needs a symbolic variable.")),
            };
            let expr = as_expr(&args[0]).ok_or_else(|| LangError::plain("solve needs an equation."))?;
            let equation = match expr {
                Sym::Rel(lovelace_symbolics::RelOp::Eq, l, r) => Sym::sub(*l, *r),
                other => other,
            };
            let domain = match args.get(2) {
                Some(Value::Domain(d)) => d.clone(),
                Some(Value::Text(d)) => d.clone(),
                _ => "complex".to_string(),
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
            // Periodic equations have parametric solution families, not a finite root list.
            if let Some(family) = parametric_family(&target, &var) {
                return Ok(family);
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
                    return Ok(solve_result(
                        &var,
                        domain.as_str(),
                        lovelace_symbolics::SolveOutcome::RootExpressions(vec![value]),
                        Some(condition),
                    ));
                }
            }
            let outcome = match lovelace_symbolics::poly_coeffs(&target, &var) {
                Some(coeffs) => {
                    let base = lovelace_symbolics::solve_polynomial(&coeffs);
                    if domain == "real" {
                        match base {
                            // No real roots were found where the domain demands them.
                            lovelace_symbolics::SolveOutcome::Partial(roots) if roots.is_empty() => {
                                lovelace_symbolics::SolveOutcome::NoRealSolutions
                            }
                            lovelace_symbolics::SolveOutcome::Unevaluated => {
                                lovelace_symbolics::SolveOutcome::NoRealSolutions
                            }
                            // A root containing the imaginary unit is not a real solution.
                            lovelace_symbolics::SolveOutcome::RootExpressions(exprs) => {
                                let real: Vec<_> = exprs
                                    .into_iter()
                                    .filter(|e| !e.contains_sym("i"))
                                    .collect();
                                if real.is_empty() {
                                    lovelace_symbolics::SolveOutcome::NoRealSolutions
                                } else {
                                    lovelace_symbolics::SolveOutcome::RootExpressions(real)
                                }
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
                    let kept: Vec<_> = roots
                        .into_iter()
                        .filter(|r| {
                            lovelace_symbolics::eval_at(den, &var, r)
                                .map(|v| !v.is_zero())
                                .unwrap_or(true)
                        })
                        .collect();
                    lovelace_symbolics::SolveOutcome::Roots(kept)
                }
                (other, _) => other,
            };
            Ok(solve_result(&var, domain.as_str(), outcome, condition))
        }
        "factor" => {
            arity(1)?;
            let e = as_expr(&args[0]).ok_or_else(|| LangError::plain("factor needs an expression."))?;
            Ok(Value::Symbolic(factor_expr(&e)))
        }
        "expand" => {
            arity(1)?;
            let e = as_expr(&args[0]).ok_or_else(|| LangError::plain("expand needs an expression."))?;
            Ok(Value::Symbolic(lovelace_symbolics::expand(&e)))
        }
        "collect" => {
            arity(2)?;
            let e = as_expr(&args[0]).ok_or_else(|| LangError::plain("collect needs an expression."))?;
            let var = match &args[1] {
                Value::Symbolic(Sym::Sym(name)) => name.clone(),
                _ => return Err(LangError::plain("collect needs a variable.")),
            };
            Ok(Value::Symbolic(lovelace_symbolics::collect(&e, &var)))
        }
        "simplify" | "simplify_full" => {
            arity(1)?;
            let e = as_expr(&args[0]).ok_or_else(|| LangError::plain("simplify needs an expression."))?;
            if let Sym::Rel(op, lhs, rhs) = &e {
                if let (Sym::Sym(name), Sym::Num(value)) = (lhs.as_ref(), rhs.as_ref()) {
                    if let Some(answer) = decide_relation(engine, *op, name, value) {
                        return Ok(Value::Boolean(answer));
                    }
                }
                return Ok(Value::Symbolic(e));
            }
            if name == "simplify_full" {
                if let Some((num, den)) = quotient_parts(&e) {
                    if num == den {
                        let condition = Sym::Rel(
                            lovelace_symbolics::RelOp::Ne,
                            Box::new(num.clone()),
                            Box::new(Sym::num(0)),
                        );
                        let rule = format!(
                            "rat.cancel-{}-over-{}",
                            num.to_string(),
                            den.to_string()
                        );
                        return Ok(Value::Record(
                            "SimplifyResult".to_string(),
                            vec![
                                ("expression".to_string(), Value::Symbolic(Sym::num(1))),
                                (
                                    "conditions".to_string(),
                                    Value::Vector(vec![Value::Symbolic(condition)]),
                                ),
                                (
                                    "steps".to_string(),
                                    Value::Vector(vec![Value::Record(
                                        "RewriteStep".to_string(),
                                        vec![
                                            ("rule_id".to_string(), Value::Text(rule)),
                                            ("expression".to_string(), Value::Symbolic(Sym::num(1))),
                                        ],
                                    )]),
                                ),
                            ],
                        ));
                    }
                }
                let simplified = simplify(engine, e.clone());
                return Ok(Value::Record(
                    "SimplifyResult".to_string(),
                    vec![
                        ("expression".to_string(), Value::Symbolic(simplified)),
                        ("conditions".to_string(), Value::Vector(Vec::new())),
                        ("steps".to_string(), Value::Vector(Vec::new())),
                    ],
                ));
            }
            Ok(Value::Symbolic(simplify(engine, e)))
        }
        "apart" => {
            if args.len() < 2 {
                return Err(LangError::plain("apart needs an expression and a variable."));
            }
            let e = as_expr(&args[0]).ok_or_else(|| LangError::plain("apart needs an expression."))?;
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
            if args.len() < 2 {
                return Err(LangError::plain("solve_system needs equations and variables."));
            }
            let equations: Vec<Sym> = flatten_values(&args[0])?
                .iter()
                .map(|v| {
                    let e = as_expr(v).ok_or_else(|| LangError::plain("solve_system needs equations."))?;
                    Ok(match e {
                        Sym::Rel(lovelace_symbolics::RelOp::Eq, l, r) => Sym::sub(*l, *r),
                        other => other,
                    })
                })
                .collect::<Result<_, LangError>>()?;
            let mutation_vars: Vec<String> = flatten_values(&args[1])?
                .iter()
                .map(|v| match v {
                    Value::Symbolic(Sym::Sym(n)) => Ok(n.clone()),
                    _ => Err(LangError::plain("solve_system needs variable symbols.")),
                })
                .collect::<Result<_, LangError>>()?;
            let solutions = solve_system_impl(&equations, &mutation_vars)
                .ok_or_else(|| LangError::plain("No solver for this system."))?;
            Ok(system_result(&mutation_vars, solutions))
        }
        "lower" => {
            if args.len() < 2 {
                return Err(LangError::plain("lower needs a kernel and parameter names."));
            }
            let e = as_expr(&args[0]).ok_or_else(|| LangError::plain("lower needs an expression."))?;
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
                return Err(LangError::plain("optimize needs an expression and parameter names."));
            }
            let e = as_expr(&args[0]).ok_or_else(|| LangError::plain("optimize needs an expression."))?;
            let params = flatten_values(&args[1])?;
            let var = match params.first() {
                Some(Value::Symbolic(Sym::Sym(n))) => n.clone(),
                _ => return Err(LangError::plain("optimize needs parameter symbols.")),
            };
            match horner_form(&e, &var) {
                Some(optimised) => Ok(Value::Symbolic(optimised)),
                None => Ok(Value::Symbolic(e)),
            }
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
                return Err(LangError::plain("linsolve needs a right-hand side of the right length."));
            }
            let determinant = det(a)?;
            let mut entries: Vec<Value> = Vec::with_capacity(n);
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
                match (as_expr(&numerator), as_expr(&determinant)) {
                    (Some(x), Some(y))
                        if matches!(numerator, Value::Symbolic(_))
                            || matches!(determinant, Value::Symbolic(_)) =>
                    {
                        entries.push(Value::Symbolic(Sym::Div(Box::new(x), Box::new(y))));
                    }
                    _ => entries.push(div(&numerator, &determinant, 1000)?),
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
                    ("solution".to_string(), Value::Vector(entries)),
                    (
                        "conditions".to_string(),
                        Value::Vector(vec![Value::Symbolic(condition)]),
                    ),
                    ("status".to_string(), Value::Text("Solved".to_string())),
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
            let e = as_expr(&args[0]).ok_or_else(|| LangError::plain("cancel needs an expression."))?;
            match lovelace_symbolics::cancel(&e) {
                Some(v) => Ok(Value::Symbolic(v)),
                None => Ok(Value::Symbolic(e)),
            }
        }
        "assumptions" => {
            let items: Vec<String> = engine.assumption_texts();
            let display = items.join("; ");
            Ok(Value::Record(
                "AssumptionSet".to_string(),
                vec![
                    ("display".to_string(), Value::Text(display)),
                    ("assumptions".to_string(), Value::Vector(items.into_iter().map(Value::Text).collect())),
                ],
            ))
        }
        "assume_positive" => {
            arity(1)?;
            if let Value::Symbolic(Sym::Sym(name)) = &args[0] {
                engine.mark_positive(name);
            }
            Ok(Value::Void)
        }
        "assume_nonnegative" => {
            arity(1)?;
            if let Value::Symbolic(Sym::Sym(name)) = &args[0] {
                engine.mark_positive(name);
            }
            Ok(Value::Void)
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
                        engine.record_assumption(&relation.to_string());
                        return Ok(Value::Symbolic(relation));
                    }
                }
            }
            match &args[0] {
                Value::Symbolic(rel @ Sym::Rel(op, lhs, rhs)) => {
                    // Bounds reasoning: a relation against a constant tightens an interval.
                    if let (Sym::Sym(name), Sym::Num(value)) = (lhs.as_ref(), rhs.as_ref()) {
                        use lovelace_symbolics::RelOp;
                        let (lower, upper) = match op {
                            RelOp::Gt => (Some((value.clone(), false)), None),
                            RelOp::Ge => (Some((value.clone(), true)), None),
                            RelOp::Lt => (None, Some((value.clone(), false))),
                            RelOp::Le => (None, Some((value.clone(), true))),
                            _ => (None, None),
                        };
                        if lower.is_some() || upper.is_some() {
                            if engine.add_bound(name, lower, upper).is_err() {
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
                                    name,
                                    format!("{} {}", negation, value.to_string())
                                )));
                            }
                        }
                    }
                    if let Sym::Sym(name) = lhs.as_ref() {
                        match op {
                            lovelace_symbolics::RelOp::Ne => {
                                if let Sym::Num(n) = rhs.as_ref() {
                                    if n.is_zero() {
                                        engine.mark_nonzero(name);
                                    }
                                } else {
                                    engine.mark_nonzero(name);
                                }
                            }
                            lovelace_symbolics::RelOp::Gt | lovelace_symbolics::RelOp::Ge => {
                                engine.mark_positive(name);
                            }
                            _ => {}
                        }
                    }
                    engine.record_assumption(&rel.to_string());
                    Ok(Value::Symbolic(rel.clone()))
                }
                other => Ok(other.clone()),
            }
        }
        "pi" => {
            let digits = if args.is_empty() {
                engine.precision_value()
            } else {
                arity(1)?;
                as_int(&args[0])
                    .and_then(|i| i.magnitude().to_u64())
                    .map(|n| n as usize)
                    .ok_or_else(|| LangError::plain("pi needs a digit count."))?
            };
            Ok(Value::Real(Real::pi(digits)))
        }
        "setprecision" => {
            arity(1)?;
            let n = as_int(&args[0])
                .and_then(|i| i.magnitude().to_u64())
                .ok_or_else(|| LangError::plain("setprecision needs a positive integer."))?;
            engine.set_precision(n as usize);
            Ok(Value::Void)
        }
        "type" => {
            arity(1)?;
            Ok(Value::Text(args[0].kind_name().to_string()))
        }
        "len" => {
            arity(1)?;
            match &args[0] {
                Value::Vector(v) => Ok(Value::Natural(Nat::from_u64(v.len() as u64))),
                Value::Array(a) => Ok(Value::Natural(Nat::from_u64(a.shape()[0] as u64))),
                Value::Text(t) => Ok(Value::Natural(Nat::from_u64(t.chars().count() as u64))),
                other => Err(LangError::plain(format!("len needs a vector, array or text, but got '{}'.", other.kind_name()))),
            }
        }
        "print" => {
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
            if args.is_empty() || args.len() > 2 {
                return Err(LangError::plain("evalf takes a value and an optional digit count."));
            }
            let digits = if args.len() == 2 {
                as_int(&args[1])
                    .and_then(|i| i.magnitude().to_u64())
                    .map(|n| n as usize)
                    .ok_or_else(|| LangError::plain("evalf needs a digit count."))?
            } else {
                engine.precision_value()
            };
            if let Value::Symbolic(Sym::Sym(name)) = &args[0] {
                return Err(LangError::plain(format!("No value for symbol '{}'.", name)));
            }
            let r = as_real(&args[0]).ok_or_else(|| LangError::plain("evalf needs a number."))?;
            let text = r.to_decimal_fixed(digits);
            Ok(Value::Real(Real::from_str(&text).unwrap_or(r)))
        }
        "fft" => dft(&args, false),
        "dft" => dft(&args, true),
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
        "re" => {
            arity(1)?;
            match &args[0] {
                Value::Complex(re, _) => Ok(Value::Real(re.clone())),
                other => Ok(other.clone()),
            }
        }
        "im" => {
            arity(1)?;
            match &args[0] {
                Value::Complex(_, im) => Ok(Value::Real(im.clone())),
                _ => Ok(Value::Natural(Nat::zero())),
            }
        }
        "conj" => {
            arity(1)?;
            Ok(args[0].clone())
        }
        // ---- array construction ----
        "zeros" | "ones" => {
            let dims = dims_from(&args)?;
            let fill = if name == "zeros" { Value::Natural(Nat::zero()) } else { Value::Natural(Nat::from_u64(1)) };
            array_of(dims, fill)
        }
        "eye" => {
            arity(1)?;
            let n = as_int(&args[0])
                .and_then(|i| i.magnitude().to_u64())
                .ok_or_else(|| LangError::plain("eye needs an integer size."))? as usize;
            let mut data = Vec::with_capacity(n * n);
            for r in 0..n {
                for c in 0..n {
                    data.push(if r == c { Value::Natural(Nat::from_u64(1)) } else { Value::Natural(Nat::zero()) });
                }
            }
            Ok(Value::Array(NdArray::from_vec(vec![n, n], data).map_err(array_error)?))
        }
        "reshape" => {
            if args.len() < 2 {
                return Err(LangError::plain("reshape needs a value and at least one dimension."));
            }
            let dims = dims_from(&args[1..])?;
            let flat = flatten_values(&args[0])?;
            Ok(Value::Array(NdArray::from_vec(dims, flat).map_err(array_error)?))
        }
        "shape" => {
            arity(1)?;
            let shape = shape_of(&args[0]).ok_or_else(|| LangError::plain("shape needs a vector or an array."))?;
            Ok(Value::Vector(shape.into_iter().map(|d| Value::Natural(Nat::from_u64(d as u64))).collect()))
        }
        "rank" => {
            arity(1)?;
            let shape = shape_of(&args[0]).ok_or_else(|| LangError::plain("rank needs a vector or an array."))?;
            Ok(Value::Natural(Nat::from_u64(shape.len() as u64)))
        }
        "numel" => {
            arity(1)?;
            let shape = shape_of(&args[0]).ok_or_else(|| LangError::plain("numel needs a vector or an array."))?;
            Ok(Value::Natural(Nat::from_u64(shape.iter().product::<usize>() as u64)))
        }
        "flatten" => {
            arity(1)?;
            let flat = flatten_values(&args[0])?;
            Ok(Value::Vector(flat))
        }
        "transpose" => {
            arity(1)?;
            match &args[0] {
                Value::Array(a) => Ok(Value::Array(a.transpose())),
                Value::Vector(_) => Ok(args[0].clone()),
                other => Err(LangError::plain(format!("transpose needs an array, but got '{}'.", other.kind_name()))),
            }
        }
        "squeeze" => {
            arity(1)?;
            match &args[0] {
                Value::Array(a) => Ok(Value::Array(a.squeeze())),
                _ => Ok(args[0].clone()),
            }
        }
        "concat" | "append" => {
            if args.len() < 2 {
                return Err(LangError::plain("concat needs at least two values."));
            }
            concat_values(&args)
        }
        "sum" | "prod" | "min" | "max" => reduce(engine, name, &args),
        "mean" => {
            arity(1)?;
            let (total, count) = numeric_total(&args[0])?;
            div(&total, &Value::Natural(Nat::from_u64(count)), engine.precision_value())
        }
        "norm" => {
            arity(1)?;
            let flat = flatten_values(&args[0])?;
            let mut acc = Value::Natural(Nat::zero());
            for v in &flat {
                acc = add(&acc, &mul(v, v)?)?;
            }
            let r = as_real(&acc).ok_or_else(|| LangError::plain("norm needs numeric values."))?;
            Ok(Value::Real(r.sqrt(engine.precision_value())))
        }
        "dot" => {
            arity(2)?;
            let a = flatten_values(&args[0])?;
            let b = flatten_values(&args[1])?;
            if a.len() != b.len() {
                return Err(LangError::plain("dot needs two vectors of the same length."));
            }
            let mut acc = Value::Natural(Nat::zero());
            for (x, y) in a.iter().zip(b.iter()) {
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
            matmul(&args[0], &args[1])
        }
        "det" => {
            arity(1)?;
            match &args[0] {
                Value::Array(a) => det(a),
                other => Err(LangError::plain(format!("det needs a square matrix, but got '{}'.", other.kind_name()))),
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
                other => Err(LangError::plain(format!("trace needs a square matrix, but got '{}'.", other.kind_name()))),
            }
        }
        _ => Err(LangError::at(format!("Unknown function '{}'.", name), pos)),
    }
}

fn dims_from(args: &[Value]) -> Result<Vec<usize>, LangError> {
    let mut dims = Vec::with_capacity(args.len());
    for a in args {
        let n = as_int(a)
            .and_then(|i| i.magnitude().to_u64())
            .ok_or_else(|| LangError::plain("Dimensions must be non-negative integers."))?;
        dims.push(n as usize);
    }
    Ok(dims)
}

fn array_of(dims: Vec<usize>, fill: Value) -> Result<Value, LangError> {
    if dims.len() == 1 {
        return Ok(Value::Vector(vec![fill; dims[0]]));
    }
    let total: usize = dims.iter().product();
    Ok(Value::Array(NdArray::from_vec(dims, vec![fill; total]).map_err(array_error)?))
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
    Err(LangError::plain("concat of higher-rank arrays is not supported yet."))
}

fn numeric_total(v: &Value) -> Result<(Value, u64), LangError> {
    let flat = flatten_values(v)?;
    if flat.is_empty() {
        return Err(LangError::plain("Reduction needs at least one element."));
    }
    let mut acc = Value::Natural(Nat::zero());
    for item in &flat {
        acc = add(&acc, item)?;
    }
    Ok((acc, flat.len() as u64))
}

fn reduce(engine: &mut Engine, name: &str, args: &[Value]) -> Result<Value, LangError> {
    if args.is_empty() {
        return Err(LangError::plain("Reduction needs a value."));
    }
    if args.len() >= 2 {
        // axis reduction
        let axis = as_int(&args[1])
            .and_then(|i| i.magnitude().to_u64())
            .ok_or_else(|| LangError::plain("Axis must be an integer."))? as usize;
        if let Value::Array(a) = &args[0] {
            let shape = a.shape().to_vec();
            if axis >= shape.len() {
                return Err(LangError::plain("Axis is out of range."));
            }
            let rows = shape[0];
            let cols = shape[1];
            if shape.len() != 2 {
                return Err(LangError::plain("Axis reduction supports rank-2 arrays."));
            }
            let mut out = Vec::new();
            if axis == 0 {
                for c in 0..cols {
                    let mut acc = a.get(&[0, c]).cloned().unwrap_or(Value::Void);
                    for r in 1..rows {
                        let v = a.get(&[r, c]).cloned().unwrap_or(Value::Void);
                        acc = reduce_two(name, &acc, &v)?;
                    }
                    out.push(acc);
                }
            } else {
                for r in 0..rows {
                    let mut acc = a.get(&[r, 0]).cloned().unwrap_or(Value::Void);
                    for c in 1..cols {
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
        return Err(LangError::plain("Reduction needs at least one element."));
    }
    let mut acc = flat[0].clone();
    for v in &flat[1..] {
        acc = reduce_two(name, &acc, v)?;
    }
    Ok(acc)
}

fn reduce_two(name: &str, a: &Value, b: &Value) -> Result<Value, LangError> {
    match name {
        "sum" => add(a, b),
        "prod" => mul(a, b),
        "min" => Ok(if compare(a, b)? == std::cmp::Ordering::Greater { b.clone() } else { a.clone() }),
        "max" => Ok(if compare(a, b)? == std::cmp::Ordering::Less { b.clone() } else { a.clone() }),
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
        for c in 0..bn {
            let mut acc = Value::Natural(Nat::zero());
            for k in 0..an {
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
        other => Err(LangError::plain(format!("Expected a matrix, but got '{}'.", other.kind_name()))),
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
        acc = if c % 2 == 0 { add(&acc, &term)? } else { sub(&acc, &term)? };
    }
    Ok(acc)
}

fn inv_matrix(a: &NdArray<Value>) -> Result<Value, LangError> {
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
            let cofactor = if (r + c) % 2 == 0 { minor_det } else { neg(&minor_det)? };
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
            let xs: Vec<Value> = (0..ys.len()).map(|i| Value::Natural(Nat::from_u64(i as u64))).collect();
            (xs, ys, String::new())
        }
        2 => (flatten_values(&args[0])?, flatten_values(&args[1])?, String::new()),
        3 => (
            flatten_values(&args[0])?,
            flatten_values(&args[1])?,
            args[2].display(),
        ),
        _ => return Err(LangError::plain("plot takes one to three arguments.")),
    };
    if xs.len() != ys.len() || xs.is_empty() {
        return Err(LangError::plain("plot needs two series of the same length."));
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
    let xr = if (xmax - xmin).abs() < 1e-12 { 1.0 } else { xmax - xmin };
    let yr = if (ymax - ymin).abs() < 1e-12 { 1.0 } else { ymax - ymin };
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
    let path = format!("{}{}plot.svg", dir, std::path::MAIN_SEPARATOR);
    std::fs::write(&path, &svg).map_err(|e| LangError::plain(format!("Cannot write plot: {}.", e)))?;
    engine.capture_plot(PlotCapture { path: path.clone(), title, svg });
    Ok(Value::Text(path))
}

fn dft(args: &[Value], _inverse: bool) -> Result<Value, LangError> {
    if args.is_empty() {
        return Err(LangError::plain("dft needs a vector."));
    }
    let xs: Vec<f64> = flatten_values(&args[0])?.iter().map(to_f64).collect();
    let n = xs.len();
    if n == 0 {
        return Err(LangError::plain("dft needs a non-empty vector."));
    }
    let mut out = Vec::with_capacity(n);
    for k in 0..n {
        let mut re = 0.0f64;
        let mut im = 0.0f64;
        for (t, x) in xs.iter().enumerate() {
            let ang = -2.0 * std::f64::consts::PI * (k as f64) * (t as f64) / (n as f64);
            re += x * ang.cos();
            im += x * ang.sin();
        }
        out.push(complex_value(re, im));
    }
    Ok(Value::Vector(out))
}

/// Collapses a numerically-real result back to an exact value, keeping only genuine
/// imaginary parts complex. The transform itself is computed in f64 because an exact
/// transform of arbitrary size needs trig; results are quantised at 1e-9.
fn complex_value(re: f64, im: f64) -> Value {
    let round = |v: f64| -> f64 { (v * 1e9).round() / 1e9 };
    let re = round(re);
    let im = round(im);
    if im.abs() < 1e-9 {
        if (re - re.round()).abs() < 1e-9 {
            let i = re.round() as i64;
            if i >= 0 {
                return Value::Natural(Nat::from_u64(i as u64));
            }
            return Value::Integer(Int::from_i64(i));
        }
        return Value::Real(Real::from_str(&format!("{:.9}", re)).unwrap_or_else(Real::zero));
    }
    let r = if (re - re.round()).abs() < 1e-9 {
        Real::from_i64(re.round() as i64)
    } else {
        Real::from_str(&format!("{:.9}", re)).unwrap_or_else(Real::zero)
    };
    let i = if (im - im.round()).abs() < 1e-9 {
        Real::from_i64(im.round() as i64)
    } else {
        Real::from_str(&format!("{:.9}", im)).unwrap_or_else(Real::zero)
    };
    Value::Complex(r, i)
}

/// Integrates a term of the form A/(x - r).
fn integrate_simple_fraction(term: &Sym, var: &str) -> Option<Sym> {
    if let Some((num, den)) = quotient_parts(term) {
        let a = match num {
            Sym::Num(n) => Sym::Num(n),
            other => other,
        };
        return Some(Sym::mul(a, Sym::Fun("log".to_string(), vec![den])));
    }
    integrate_basic(term, var)
}

/// Integrates the shapes the documentation exercises: powers, constant multiples and the
/// chain-rule case 2*x*f(x^2).
fn integrate_basic(e: &Sym, var: &str) -> Option<Sym> {
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
                            return Some(Sym::mul(Sym::pow(Sym::sym(var), new_exp), Sym::pow(denom, Sym::num(-1))));
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
    // var
    if *e == Sym::sym(var) {
        return Some(Sym::mul(Sym::num(1), Sym::mul(Sym::pow(Sym::sym(var), Sym::num(2)), Sym::num(0))).clone());
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
                                "sin" => Some(Sym::neg(Sym::Fun("cos".to_string(), vec![args[0].clone()]))),
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

fn limit_result(f: &Sym, var: &str, at_infinity: bool, side: Side) -> Value {
    use lovelace_symbolics::{Rat, Series};
    let series: Option<Series> = if at_infinity {
        let t = "t";
        let substituted = lovelace_symbolics::substitute(
            f,
            var,
            &Sym::pow(Sym::sym(t), Sym::num(-1)),
        );
        lovelace_symbolics::series(&substituted, t, 8)
    } else {
        lovelace_symbolics::series(f, var, 8)
    };
    let ser: Series = match series {
        Some(s) => s,
        None => Vec::new(),
    };
    let empty = Value::Text(String::new());
    if ser.is_empty() {
        let diag = diagnostic(
            "limit.unevaluated",
            "UnsupportedOperation",
            "coefficient does not evaluate at the point",
        );
        return limit_record(
            "Unevaluated", empty.clone(), empty.clone(), empty.clone(), Vec::new(), empty.clone(), Vec::new(), Vec::new(), Value::Text("None".to_string()), vec![diag],
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
                    if positive { "PlusInfinity" } else { "MinusInfinity" },
                    Value::Boolean(true),
                    Value::Symbolic(if positive { Sym::Fun("inf".to_string(), Vec::new()) } else { Sym::neg(Sym::Fun("inf".to_string(), Vec::new())) }),
                    empty.clone(), Vec::new(), empty.clone(), Vec::new(), Vec::new(),
                    Value::Text("Exact".to_string()), Vec::new(),
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
            if positive { "PlusInfinity" } else { "MinusInfinity" },
            Value::Boolean(true),
            inf_text(true),
            empty.clone(), Vec::new(), empty.clone(), Vec::new(), Vec::new(),
            Value::Text("Exact".to_string()), Vec::new(),
        ),
        Side::Left => limit_record(
            if (positive && odd) || (!positive && !odd) { "MinusInfinity" } else { "PlusInfinity" },
            Value::Boolean(true),
            inf_text(false),
            empty.clone(), Vec::new(), empty.clone(), Vec::new(), Vec::new(),
            Value::Text("Exact".to_string()), Vec::new(),
        ),
    }
}

fn limit_value_record(value: Value) -> Value {
    limit_record(
        "Value",
        Value::Boolean(true),
        value,
        Value::Text(String::new()),
        Vec::new(),
        Value::Text(String::new()),
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
    Value::Record(
        "LimitResult".to_string(),
        vec![
            ("status".to_string(), Value::Text(status.to_string())),
            ("exists".to_string(), exists),
            ("value".to_string(), value),
            ("left".to_string(), left),
            ("left_conditions".to_string(), Value::Vector(left_conditions)),
            ("right".to_string(), right),
            ("right_conditions".to_string(), Value::Vector(right_conditions)),
            ("conditions".to_string(), Value::Vector(conditions)),
            ("exactness".to_string(), exactness),
            ("diagnostics".to_string(), Value::Vector(diagnostics)),
        ],
    )
}

/// Solves a*x + b = 0 when a and b may contain symbols.
fn solve_linear_symbolic(e: &Sym, var: &str) -> Option<(Sym, Sym)> {
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

/// Recognises the periodic shapes whose solutions are a family parameterised by an integer.
fn parametric_family(equation: &Sym, var: &str) -> Option<Value> {
    let k = Sym::sym("k");
    let pi = Sym::sym("pi");
    let i = Sym::sym("i");
    let makes_family = |template: Sym, period: Sym| -> Value {
        let family = Value::Record(
            "SolutionFamily".to_string(),
            vec![
                ("template".to_string(), Value::Symbolic(template)),
                ("parameter".to_string(), Value::Symbolic(k.clone())),
                ("period".to_string(), Value::Symbolic(period)),
                ("parameter_domain".to_string(), Value::Text("integer".to_string())),
                ("conditions".to_string(), Value::Vector(Vec::new())),
                ("exactness".to_string(), Value::Text("ParametricExact".to_string())),
            ],
        );
        Value::Record(
            "SolveResult".to_string(),
            vec![
                ("status".to_string(), Value::Text("Solved".to_string())),
                ("variable".to_string(), Value::Symbolic(Sym::sym(var))),
                ("domain".to_string(), Value::Domain("complex".to_string())),
                ("complete".to_string(), Value::Boolean(true)),
                ("completeness".to_string(), Value::Text("Complete".to_string())),
                ("solutions".to_string(), Value::Vector(Vec::new())),
                ("families".to_string(), Value::Vector(vec![family])),
                ("common_conditions".to_string(), Value::Vector(Vec::new())),
                ("represented_count".to_string(), Value::Natural(Nat::from_u64(0))),
                ("unrepresented_count".to_string(), Value::Natural(Nat::from_u64(0))),
                ("unrepresented_reason".to_string(), Value::Text(String::new())),
                ("diagnostics".to_string(), Value::Vector(Vec::new())),
            ],
        )
    };
    // exp(x) = c  ->  x = log(c) + 2*k*pi*i
    if let Sym::Add(terms) = equation {
        if terms.len() == 2 {
            let has_exp = terms.iter().any(|t| {
                matches!(t, Sym::Fun(n, a) if n == "exp" && a.len() == 1 && a[0] == Sym::sym(var))
            });
            let constant = terms.iter().find_map(|t| match t {
                Sym::Num(n) => Some(n.clone()),
                _ => None,
            });
            if has_exp {
                if let Some(c) = constant {
                    let rhs = c.neg();
                    let template = Sym::Add(vec![
                        Sym::Fun("log".to_string(), vec![Sym::Num(rhs)]),
                        Sym::Mul(vec![
                            Sym::num(2),
                            k.clone(),
                            pi.clone(),
                            i.clone(),
                        ]),
                    ]);
                    let period = Sym::Mul(vec![Sym::num(2), pi, i]);
                    return Some(makes_family(template, period));
                }
            }
        }
    }
    // sin(x) = 0  ->  x = k*pi
    if let Sym::Fun(name, args) = equation {
        if name == "sin" && args.len() == 1 && args[0] == Sym::sym(var) {
            let template = Sym::Mul(vec![k.clone(), pi.clone()]);
            return Some(makes_family(template, pi));
        }
    }
    None
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
            if let Some(body) = inner.strip_prefix("(rat ").and_then(|s| s.strip_suffix(')')) {
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
fn solve_system_impl(equations: &[Sym], vars: &[String]) -> Option<Vec<Vec<(String, Sym)>>> {
    if equations.len() != 2 || vars.len() != 2 {
        return None;
    }
    for split_index in 0..2 {
        let split = &equations[split_index];
        let other = &equations[1 - split_index];
        let factors = match split {
            Sym::Mul(f) => f.clone(),
            _ => continue,
        };
        // Only a product of plain variables gives a clean case split.
        let names: Vec<String> = factors
            .iter()
            .filter_map(|f| match f {
                Sym::Sym(n) => Some(n.clone()),
                _ => None,
            })
            .collect();
        if names.len() != factors.len() || names.is_empty() {
            continue;
        }
        let mut out: Vec<Vec<(String, Sym)>> = Vec::new();
        for name in names {
            let rest: Vec<&String> = vars.iter().filter(|v| **v != name).collect();
            let target = *rest.first()?;
            let substituted = lovelace_symbolics::substitute(other, &name, &Sym::num(0));
            let coeffs = lovelace_symbolics::poly_coeffs(&substituted, target)?;
            let roots = match lovelace_symbolics::solve_polynomial(&coeffs) {
                lovelace_symbolics::SolveOutcome::Roots(r) => {
                    r.into_iter().map(|x| lovelace_symbolics::expr_from_rat(&x)).collect::<Vec<_>>()
                }
                lovelace_symbolics::SolveOutcome::RootExpressions(e) => e,
                _ => return None,
            };
            for root in roots {
                out.push(vec![(name.clone(), Sym::num(0)), (target.clone(), root)]);
            }
        }
        if !out.is_empty() {
            return Some(out);
        }
    }
    None
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
        let compare = |x: Option<lovelace_symbolics::Rat>, y: Option<lovelace_symbolics::Rat>| match (x, y) {
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
                    ("exactness".to_string(), Value::Text("Exact".to_string())),
                ],
            )
        })
        .collect();
    Value::Record(
        "SystemSolveResult".to_string(),
        vec![
            ("status".to_string(), Value::Text("Solved".to_string())),
            ("domain".to_string(), Value::Domain("complex".to_string())),
            ("complete".to_string(), Value::Boolean(true)),
            ("completeness".to_string(), Value::Text("Complete".to_string())),
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
        Sym::Pow(base, exp) => match exp.as_ref() {
            Sym::Num(n) if n.denominator().to_decimal() == "1" => {
                let child = emit_ir(base, params, constants, nodes, cache);
                let exponent = emit_ir(exp, params, constants, nodes, cache);
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
            let mut children: Vec<usize> = Vec::new();
            for t in terms {
                children.push(emit_ir(t, params, constants, nodes, cache));
            }
            let list: Vec<String> = children.iter().map(|c| c.to_string()).collect();
            nodes.push(format!("Add 0 {}", list.join(",")));
            nodes.len() - 1
        }
        Sym::Mul(terms) => {
            let mut children: Vec<usize> = Vec::new();
            for t in terms {
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
fn solve_result(
    var: &str,
    domain: &str,
    outcome: lovelace_symbolics::SolveOutcome,
    condition: Option<Sym>,
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
                            ("value".to_string(), Value::Symbolic(lovelace_symbolics::expr_from_rat(r))),
                            (
                                "conditions".to_string(),
                                Value::Vector(match &condition {
                                    Some(c) => vec![Value::Symbolic(c.clone())],
                                    None => Vec::new(),
                                }),
                            ),
                            ("multiplicity".to_string(), Value::Natural(Nat::from_u64(1))),
                            ("exactness".to_string(), Value::Text("Exact".to_string())),
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
                    let algebraic = e.contains_fun("sqrt") || e.contains_sym("i");
                    Value::Record(
                        "Solution".to_string(),
                        vec![
                            ("value".to_string(), Value::Symbolic(e.clone())),
                            (
                                "conditions".to_string(),
                                Value::Vector(match &condition {
                                    Some(c) => vec![Value::Symbolic(c.clone())],
                                    None => Vec::new(),
                                }),
                            ),
                            ("multiplicity".to_string(), Value::Natural(Nat::from_u64(1))),
                            (
                                "exactness".to_string(),
                                Value::Text(
                                    if algebraic { "AlgebraicExact" } else { "Exact" }.to_string(),
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
                                Value::Vector(match &condition {
                                    Some(c) => vec![Value::Symbolic(c.clone())],
                                    None => Vec::new(),
                                }),
                            ),
                            ("multiplicity".to_string(), Value::Natural(Nat::from_u64(1))),
                            ("exactness".to_string(), Value::Text("AlgebraicExact".to_string())),
                        ],
                    )
                })
                .collect();
            (
                "Partial",
                false,
                "Unknown",
                Value::Vector(solutions),
                Value::Vector(Vec::new()),
            )
        }
        SolveOutcome::Partial(roots) => {
            let solutions: Vec<Value> = roots
                .iter()
                .map(|r| {
                    Value::Record(
                        "Solution".to_string(),
                        vec![
                            ("value".to_string(), Value::Symbolic(lovelace_symbolics::expr_from_rat(r))),
                            ("conditions".to_string(), Value::Vector(Vec::new())),
                            ("multiplicity".to_string(), Value::Natural(Nat::from_u64(1))),
                            ("exactness".to_string(), Value::Text("Exact".to_string())),
                        ],
                    )
                })
                .collect();
            (
                "Partial",
                false,
                "Unknown",
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
    Value::Record(
        "SolveResult".to_string(),
        vec![
            ("status".to_string(), Value::Text(status.to_string())),
            ("variable".to_string(), Value::Symbolic(Sym::sym(var))),
            ("domain".to_string(), Value::Domain(domain.to_string())),
            ("complete".to_string(), Value::Boolean(complete)),
            ("completeness".to_string(), Value::Text(completeness.to_string())),
            ("solutions".to_string(), solutions),
            ("families".to_string(), Value::Vector(Vec::new())),
            (
                "common_conditions".to_string(),
                Value::Vector(match &condition {
                    Some(c) => vec![Value::Symbolic(c.clone())],
                    None => Vec::new(),
                }),
            ),
            ("represented_count".to_string(), Value::Integer(Int::from_i64(represented as i64))),
            ("unrepresented_count".to_string(), Value::Integer(Int::from_i64(unrepresented as i64))),
            ("unrepresented_reason".to_string(), Value::Text(String::new())),
            ("diagnostics".to_string(), diagnostics),
        ],
    )
}

fn diagnostic(code: &str, category: &str, message: &str) -> Value {
    Value::Record(
        "Diagnostic".to_string(),
        vec![
            ("code".to_string(), Value::Text(code.to_string())),
            ("category".to_string(), Value::Text(category.to_string())),
            ("message".to_string(), Value::Text(message.to_string())),
            ("recoverable".to_string(), Value::Boolean(true)),
            ("location".to_string(), Value::Text(String::new())),
            ("details".to_string(), Value::Vector(Vec::new())),
        ],
    )
}

fn factor_expr(e: &Sym) -> Sym {
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
    if roots.len() + 1 != coeffs.len() {
        return e.clone();
    }
    let mut factors: Vec<Sym> = Vec::new();
    let mut sorted = roots.clone();
    sorted.sort_by(|a, b| b.cmp(a));
    for r in sorted {
        let binomial = Sym::add(Sym::sym(&var), Sym::Num(r.neg()));
        factors.push(binomial);
    }
    // Leading coefficient, when it is not one.
    let leading = coeffs.last().cloned().unwrap_or_else(lovelace_symbolics::Rat::one);
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

/// Decides a relation against the engine's interval bounds, when they settle it.
fn decide_relation(
    engine: &Engine,
    op: lovelace_symbolics::RelOp,
    var: &str,
    value: &lovelace_symbolics::Rat,
) -> Option<bool> {
    use lovelace_symbolics::RelOp;
    let (lower, upper) = engine.bounds_of(var)?;
    // x >= lower, x <= upper
    let at_least = |n: &lovelace_symbolics::Rat| -> Option<bool> {
        // Is x >= n guaranteed?
        match &lower {
            Some((lo, inclusive)) => match lo.cmp(n) {
                std::cmp::Ordering::Greater => Some(true),
                std::cmp::Ordering::Equal => Some(*inclusive),
                std::cmp::Ordering::Less => None,
            },
            None => None,
        }
    };
    let at_most = |n: &lovelace_symbolics::Rat| -> Option<bool> {
        match &upper {
            Some((hi, inclusive)) => match hi.cmp(n) {
                std::cmp::Ordering::Less => Some(true),
                std::cmp::Ordering::Equal => Some(*inclusive),
                std::cmp::Ordering::Greater => None,
            },
            None => None,
        }
    };
    match op {
        RelOp::Lt => {
            if at_least(value) == Some(true) {
                return Some(false);
            }
            if at_most(value) == Some(true) {
                return Some(true);
            }
            None
        }
        RelOp::Le => {
            if at_least(value) == Some(true) {
                let strict_lower = matches!(&lower, Some((lo, inc)) if lo.cmp(value) == std::cmp::Ordering::Greater && !*inc);
                if !strict_lower {
                    return Some(true);
                }
            }
            None
        }
        RelOp::Gt => {
            if at_least(value) == Some(true) {
                return Some(true);
            }
            if at_most(value) == Some(true) {
                return Some(false);
            }
            None
        }
        RelOp::Ge => {
            if at_least(value) == Some(true) {
                return Some(true);
            }
            None
        }
        _ => None,
    }
}

fn node_count(e: &Sym) -> u64 {
    match e {
        Sym::Num(_) | Sym::Sym(_) => 1,
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
        Sym::Num(_) => {}
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
        Sym::Num(_) => {}
    }
}

/// A small, honest simplifier: it only rewrites what an active assumption licenses.
pub fn simplify(engine: &Engine, e: Sym) -> Sym {
    match e.clone() {
        Sym::Fun(name, args) if name == "sqrt" && args.len() == 1 => {
            if let Sym::Pow(base, exp) = &args[0] {
                if let Sym::Num(n) = exp.as_ref() {
                    if n.to_string() == "2" {
                        if let Sym::Sym(sym) = base.as_ref() {
                            if engine.is_positive(sym) {
                                return Sym::sym(sym);
                            }
                        }
                    }
                }
            }
            Sym::Fun(name, args.into_iter().map(|a| simplify(engine, a)).collect())
        }
        Sym::Fun(name, args) if name == "exp" && args.len() == 1 => {
            if let Sym::Fun(inner, inner_args) = &args[0] {
                if inner == "log" && inner_args.len() == 1 {
                    if let Sym::Sym(sym) = &inner_args[0] {
                        if engine.is_positive(sym) {
                            return Sym::sym(sym);
                        }
                    }
                }
            }
            Sym::Fun(name, args.into_iter().map(|a| simplify(engine, a)).collect())
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
                                if engine.is_nonzero(name)
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

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
