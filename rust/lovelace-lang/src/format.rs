//! Canonical display and typed formatting for runtime values.
//!
//! Display rules (from the documentation corpus): numbers render exactly, Booleans as
//! `True`/`False`, Text raw, vectors and arrays as nested bracketed lists, and records
//! as `TypeName(field: value, ...)`. The typed form appends ` (Kind)` except for Text
//! (raw) and Void (`(void)`).

use lovelace_array::NdArray;

use crate::value::{Complex, Value};

/// Returns the plain display text of a value.
pub fn format(value: &Value) -> String {
    match value {
        Value::Natural(nat) => nat.to_decimal(),
        Value::Integer(int) => int.to_decimal(),
        Value::Real(real) => real.to_string(),
        Value::Boolean(flag) => if *flag { "True".to_string() } else { "False".to_string() },
        Value::Text(text) => text.clone(),
        Value::Array(array) => format_array(array),
        Value::Function(name) => format!("Function: {name}"),
        Value::Void => String::new(),
        Value::Record(record) => {
            let fields: Vec<String> = record
                .fields
                .iter()
                .map(|(name, value)| format!("{name}: {}", format(value)))
                .collect();
            format!("{}({})", record.type_name, fields.join(", "))
        }
        Value::Complex(complex) => format_complex(complex),
    }
}

/// Returns the value with its type suffix, e.g. `42 (Natural)`.
pub fn format_typed(value: &Value) -> String {
    match value {
        Value::Text(text) => text.clone(),
        Value::Void => "(void)".to_string(),
        Value::Record(record) => format!("{} ({})", format(value), record.type_name),
        _ => format!("{} ({})", format(value), value.kind_name()),
    }
}

fn format_complex(value: &Complex) -> String {
    let im_zero = value.im.is_zero();
    let re_zero = value.re.is_zero();
    if im_zero {
        return value.re.to_string();
    }
    if re_zero {
        return format!("{}i", value.im.to_string());
    }
    let negative = value.im.cmp(&lovelace_real::Real::zero()) == std::cmp::Ordering::Less;
    let im = if negative { value.im.neg() } else { value.im.clone() };
    format!(
        "{} {} {}i",
        value.re.to_string(),
        if negative { "-" } else { "+" },
        im.to_string()
    )
}

fn format_array(array: &NdArray<Value>) -> String {
    let shape = array.shape();
    if shape.is_empty() {
        return array
            .as_slice()
            .first()
            .map(format)
            .unwrap_or_else(|| "[]".to_string());
    }
    format_level(shape, array.as_slice(), 0, 0)
}

fn format_level(shape: &[usize], data: &[Value], dim: usize, offset: usize) -> String {
    if dim + 1 == shape.len() {
        let mut parts = Vec::with_capacity(shape[dim]);
        for index in 0..shape[dim] {
            if let Some(value) = data.get(offset + index) {
                parts.push(format(value));
            }
        }
        return format!("[{}]", parts.join(", "));
    }
    let stride: usize = shape[dim + 1..].iter().product();
    let mut rows = Vec::with_capacity(shape[dim]);
    for index in 0..shape[dim] {
        rows.push(format_level(shape, data, dim + 1, offset + index * stride));
    }
    format!("[{}]", rows.join(", "))
}
