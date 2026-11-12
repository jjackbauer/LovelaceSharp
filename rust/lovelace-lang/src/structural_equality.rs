//! Deep structural equality over [`Value`], the Rust counterpart of the reference
//! `Lovelace.Suite.StructuralEquality` (`Lovelace.Suite/StructuralEquality.cs`).
//!
//! Equality is STRUCTURAL, never mathematical. Two values are equal only when they have the
//! same kind and, recursively, the same payload:
//!
//! * records compare by type name, field count, field order and field values (the exact
//!   `$.field` location is named when one differs);
//! * vectors and arrays compare by shape and then elements in row-major order;
//! * symbolic values compare by their canonical S-expression form
//!   ([`Expr::canonical`](lovelace_symbolics::Expr::canonical)), never by their pretty text;
//! * scalars compare by numeric value, Boolean, ordinal text, enum type and member, domain name
//!   or function name.
//!
//! The comparison is assumption-free by construction: it takes no assumption set and never
//! simplifies. `x` and `abs(x)` remain different even under the assumption `x > 0`, and two
//! mathematically equal but structurally different expressions (`(x + 1)^2` and
//! `x^2 + 2*x + 1`) remain different. Commutative canonicalisation is honoured because the
//! canonical form of a sum/product is operand-order independent.
//!
//! The first difference is reported as `path: reason`, where the root path is `$` and nested
//! positions extend it with `.field`, `[index]` and `.re`/`.im`, matching the reference
//! diagnostic exactly.

use std::cmp::Ordering;

use crate::Value;

/// Deep structural equality over [`Value`].
///
/// The reference declares this as a static class; Rust has no static classes, so the same
/// functions are associated functions of this zero-sized type.
pub struct StructuralEquality;

impl StructuralEquality {
    /// True when the two values are structurally identical.
    pub fn values_equal(a: &Value, b: &Value) -> bool {
        Self::first_difference(a, b).is_none()
    }

    /// True when the two values are structurally identical records.
    ///
    /// The reference spells this overload `RecordsEqual(RecordValue, RecordValue)`; Rust has no
    /// separate record wrapper (`Value::Record` is the record), so the two values are checked to
    /// be records first and then compared by [`values_equal`](Self::values_equal).
    pub fn records_equal(a: &Value, b: &Value) -> bool {
        matches!((a, b), (Value::Record(..), Value::Record(..))) && Self::values_equal(a, b)
    }

    /// The first structural difference, as `path: reason`, or `None` when the values are equal.
    ///
    /// The root path is `$`, the reference's default.
    pub fn first_difference(a: &Value, b: &Value) -> Option<String> {
        Self::first_difference_from(a, b, "$")
    }

    /// [`first_difference`](Self::first_difference) with an explicit root path.
    pub fn first_difference_from(a: &Value, b: &Value, path: &str) -> Option<String> {
        let kind = kind_tag(a);
        let other_kind = kind_tag(b);
        if kind != other_kind {
            return Some(format!("{path}: kind {kind} != {other_kind}"));
        }

        match (a, b) {
            (Value::Natural(a), Value::Natural(b)) => numeric_difference(path, kind, a.cmp(b)),
            (Value::Integer(a), Value::Integer(b)) => numeric_difference(path, kind, a.cmp(b)),
            // Both spellings of a numeric Real are the same `ValueKind.Real` in the reference,
            // so a clamped value compares by its numeric payload (the clamp metadata is not part
            // of the number's structural identity).
            (
                Value::Real(a) | Value::RealClamped(a, ..),
                Value::Real(b) | Value::RealClamped(b, ..),
            ) => numeric_difference(path, kind, a.cmp(b)),
            (Value::Complex(ar, ai), Value::Complex(br, bi)) => {
                if ar.cmp(br) != Ordering::Equal {
                    return Some(format!("{path}.re: differs"));
                }
                if ai.cmp(bi) != Ordering::Equal {
                    return Some(format!("{path}.im: differs"));
                }
                None
            }
            (Value::Boolean(a), Value::Boolean(b)) => {
                (a != b).then(|| format!("{path}: boolean differs"))
            }
            (Value::Text(a), Value::Text(b)) => (a != b).then(|| format!("{path}: text differs")),
            (Value::Enum(a_type, a_member), Value::Enum(b_type, b_member)) => {
                // Equal iff BOTH the declared enum type and the member match: SolveStatus.Partial
                // is not Completeness.Partial, which is the point of carrying the type.
                if a_type != b_type {
                    return Some(format!("{path}: enum type {a_type} != {b_type}"));
                }
                (a_member != b_member)
                    .then(|| format!("{path}: enum {a_type} value {a_member} != {b_member}"))
            }
            (Value::Domain(a), Value::Domain(b)) => {
                (a != b).then(|| format!("{path}: domain differs"))
            }
            (Value::Function(a), Value::Function(b)) => {
                (a != b).then(|| format!("{path}: function differs"))
            }
            (Value::Symbolic(a), Value::Symbolic(b)) => (a.canonical() != b.canonical())
                .then(|| format!("{path}: symbolic canonical differs")),
            (Value::Vector(a), Value::Vector(b)) => {
                if a.len() != b.len() {
                    return Some(format!("{path}: shape [{}] != [{}]", a.len(), b.len()));
                }
                Self::elements_difference(a, b, path)
            }
            (Value::Array(a), Value::Array(b)) => {
                if a.shape() != b.shape() {
                    return Some(format!(
                        "{path}: shape [{}] != [{}]",
                        join_shape(a.shape()),
                        join_shape(b.shape())
                    ));
                }
                Self::elements_difference(a.as_slice(), b.as_slice(), path)
            }
            (Value::Record(a_type, a_fields), Value::Record(b_type, b_fields)) => {
                if a_type != b_type {
                    return Some(format!("{path}: record type {a_type} != {b_type}"));
                }
                if a_fields.len() != b_fields.len() {
                    return Some(format!(
                        "{path}: {a_type} has {} fields, other has {}",
                        a_fields.len(),
                        b_fields.len()
                    ));
                }
                for (index, ((a_name, a_value), (b_name, b_value))) in
                    a_fields.iter().zip(b_fields.iter()).enumerate()
                {
                    // Field ORDER is part of the record's structure: the positional check names
                    // the first slot whose declared name differs.
                    if a_name != b_name {
                        return Some(format!(
                            "{path}.{a_type}[{index}]: field name {a_name} != {b_name}"
                        ));
                    }
                    if let Some(difference) =
                        Self::first_difference_from(a_value, b_value, &format!("{path}.{a_name}"))
                    {
                        return Some(difference);
                    }
                }
                None
            }
            (Value::Void, Value::Void) => None,
            // `kind_tag` recognises every variant, so equal tags mean the same variant (or the
            // two Real spellings, folded above). Reaching this arm would mean the two helpers
            // disagree about a variant.
            _ => unreachable!("kind_tag returned one tag for two different Value variants"),
        }
    }

    fn elements_difference(a: &[Value], b: &[Value], path: &str) -> Option<String> {
        for (index, (a, b)) in a.iter().zip(b.iter()).enumerate() {
            if let Some(difference) = Self::first_difference_from(a, b, &format!("{path}[{index}]"))
            {
                return Some(difference);
            }
        }
        None
    }
}

/// The reference's `ValueKind` name for a value.
///
/// This is deliberately not [`Value::kind_name`]: that method publishes the WIRE kind, where a
/// rank-1 `Value::Array` is a `Vector`. The structural kind is the stored variant, and the two
/// storage spellings are distinct structural kinds exactly as the reference's `ValueKind.Vector`
/// and `ValueKind.Array` are. The two Real spellings are one kind because the reference has one.
fn kind_tag(value: &Value) -> &'static str {
    match value {
        Value::Natural(_) => "Natural",
        Value::Integer(_) => "Integer",
        Value::Real(_) | Value::RealClamped(..) => "Real",
        Value::Boolean(_) => "Boolean",
        Value::Text(_) => "Text",
        Value::Vector(_) => "Vector",
        Value::Array(_) => "Array",
        Value::Function(_) => "Function",
        Value::Void => "Void",
        Value::Complex(..) => "Complex",
        Value::Symbolic(_) => "Symbolic",
        Value::Domain(_) => "Domain",
        Value::Enum(..) => "Enum",
        Value::Record(..) => "Record",
    }
}

fn numeric_difference(path: &str, kind: &str, ordering: Ordering) -> Option<String> {
    (ordering != Ordering::Equal).then(|| format!("{path}: {kind} value differs"))
}

fn join_shape(shape: &[usize]) -> String {
    shape
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(",")
}
