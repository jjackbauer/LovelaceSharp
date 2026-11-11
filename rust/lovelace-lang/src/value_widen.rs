//! The additive Value widening and kind-metadata surface, mirroring Lovelace.Suite.Value.
//!
//! The C# Value carries a ValueKind tag with fixed ordinals and exposes Widen, WidenPair,
//! the As... payload accessors and a kind-prefixed ToString. The Rust Value enum already
//! identifies the payload, so this module adds the named metadata surface next to the existing
//! kind_name/display/typed methods without changing any of them.
//!
//! The widening ladder is the reference's: Natural -> Integer -> Real. The ValueKind ordinals are
//! load-bearing, because Widen compares target < kind and WidenPair chooses max(a, b) over them.

use lovelace_int::Int;
use lovelace_nat::Nat;
use lovelace_real::Real;

use crate::{LangError, Value};

/// The C# ValueKind tag, with the reference's ordinals.
///
/// The numeric values are the ones Lovelace.Suite.ValueKind declares: the three numeric kinds are
/// ordered from narrowest to widest (Natural = 0, Integer = 1, Real = 2) and every other kind
/// keeps its reference ordinal. Record is 11, Domain 12 and Enum 13, exactly as in the C# enum;
/// the values are never renumbered because the widening lattice and WidenPair's max depend on them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ValueKind {
    /// An arbitrary-precision natural number.
    Natural = 0,
    /// An arbitrary-precision signed integer.
    Integer = 1,
    /// An exact real number.
    Real = 2,
    /// A Boolean.
    Boolean = 3,
    /// Text.
    Text = 4,
    /// A rank-1 vector.
    Vector = 5,
    /// A function reference.
    Function = 6,
    /// The absence of a value.
    Void = 7,
    /// A rank >= 2 array.
    Array = 8,
    /// A complex number.
    Complex = 9,
    /// A symbolic expression.
    Symbolic = 10,
    /// A structured record.
    Record = 11,
    /// A first-class mathematical domain such as real or complex.
    Domain = 12,
    /// A declared enumeration variant.
    Enum = 13,
}

impl ValueKind {
    /// The reference ordinal of this kind (the C# ValueKind numeric value).
    pub const fn rank(self) -> u8 {
        self as u8
    }

    /// True for the three numeric kinds that participate in widening.
    pub const fn is_numeric(self) -> bool {
        matches!(
            self,
            ValueKind::Natural | ValueKind::Integer | ValueKind::Real
        )
    }

    /// The kind's name, matching the C# enum member name.
    pub const fn name(self) -> &'static str {
        match self {
            ValueKind::Natural => "Natural",
            ValueKind::Integer => "Integer",
            ValueKind::Real => "Real",
            ValueKind::Boolean => "Boolean",
            ValueKind::Text => "Text",
            ValueKind::Vector => "Vector",
            ValueKind::Function => "Function",
            ValueKind::Void => "Void",
            ValueKind::Array => "Array",
            ValueKind::Complex => "Complex",
            ValueKind::Symbolic => "Symbolic",
            ValueKind::Record => "Record",
            ValueKind::Domain => "Domain",
            ValueKind::Enum => "Enum",
        }
    }
}

impl Value {
    /// The reference Value.Kind tag.
    ///
    /// RealClamped is a Real in every other respect, so it reports Real; a rank-1 Value::Vector
    /// and a rank >= 2 Value::Array keep their distinct reference kinds.
    pub fn kind(&self) -> ValueKind {
        match self {
            Value::Natural(_) => ValueKind::Natural,
            Value::Integer(_) => ValueKind::Integer,
            Value::Real(_) | Value::RealClamped(_, _, _) => ValueKind::Real,
            Value::Boolean(_) => ValueKind::Boolean,
            Value::Text(_) => ValueKind::Text,
            Value::Vector(_) => ValueKind::Vector,
            Value::Array(_) => ValueKind::Array,
            Value::Function(_) => ValueKind::Function,
            Value::Void => ValueKind::Void,
            Value::Complex(_, _) => ValueKind::Complex,
            Value::Symbolic(_) => ValueKind::Symbolic,
            Value::Record(_, _) => ValueKind::Record,
            Value::Domain(_) => ValueKind::Domain,
            Value::Enum(_, _) => ValueKind::Enum,
        }
    }

    /// The stored natural, when this is a Natural.
    pub fn as_natural(&self) -> Option<&Nat> {
        match self {
            Value::Natural(nat) => Some(nat),
            _ => None,
        }
    }

    /// The stored integer, when this is an Integer.
    pub fn as_integer(&self) -> Option<&Int> {
        match self {
            Value::Integer(int) => Some(int),
            _ => None,
        }
    }

    /// The stored real, when this is a Real (including a digit-capped RealClamped).
    pub fn as_real(&self) -> Option<&Real> {
        match self {
            Value::Real(real) | Value::RealClamped(real, _, _) => Some(real),
            _ => None,
        }
    }

    /// Promotes this value to target along Natural -> Integer -> Real.
    ///
    /// Passing the current kind returns an equal value (the C# implementation returns this; an
    /// immutable Rust enum has no reference identity to preserve, so the mirror observes the
    /// unchanged payload instead). Non-numeric kinds are refused, and so is a target narrower
    /// than the current kind, with the reference's messages.
    pub fn widen(&self, target: ValueKind) -> Result<Value, LangError> {
        let from = self.kind();
        if target == from {
            return Ok(self.clone());
        }
        if !from.is_numeric() || !target.is_numeric() {
            return Err(non_numeric_widen_error(from, target));
        }
        if target.rank() < from.rank() {
            return Err(LangError::plain(format!(
                "Cannot narrow from {} to {}.",
                from.name(),
                target.name()
            )));
        }

        match (from, target) {
            (ValueKind::Natural, ValueKind::Integer) => {
                let Value::Natural(nat) = self else {
                    unreachable!("kind() reported Natural for a non-Natural value");
                };
                Ok(Value::Integer(natural_to_integer(nat)))
            }
            (ValueKind::Natural, ValueKind::Real) => {
                let Value::Natural(nat) = self else {
                    unreachable!("kind() reported Natural for a non-Natural value");
                };
                Ok(Value::Real(Real::from_int(&natural_to_integer(nat))))
            }
            (ValueKind::Integer, ValueKind::Real) => {
                let Value::Integer(int) = self else {
                    unreachable!("kind() reported Integer for a non-Integer value");
                };
                Ok(Value::Real(Real::from_int(int)))
            }
            _ => Err(LangError::plain(format!(
                "Unsupported widening: {} → {}.",
                from.name(),
                target.name()
            ))),
        }
    }

    /// Widens both operands to max(a.Kind, b.Kind) and returns the pair.
    ///
    /// Both results have the same kind on success. Only numeric kinds can be widened; two
    /// operands already of the same non-numeric kind are returned unchanged, exactly as the
    /// reference's two Widen calls behave.
    pub fn widen_pair(left: &Value, right: &Value) -> Result<(Value, Value), LangError> {
        let target = left.kind().max(right.kind());
        Ok((left.widen(target)?, right.widen(target)?))
    }

    /// The reference Value.ToString form: "Kind: value", except Text (raw) and Void ("Void").
    pub fn kind_prefixed(&self) -> String {
        match self {
            Value::Natural(_)
            | Value::Integer(_)
            | Value::Real(_)
            | Value::RealClamped(_, _, _)
            | Value::Boolean(_)
            | Value::Complex(_, _)
            | Value::Symbolic(_)
            | Value::Vector(_)
            | Value::Array(_) => format!("{}: {}", self.kind_name(), self.display()),
            Value::Text(_) => self.display(),
            Value::Function(name) => format!("Function: {name}"),
            Value::Void => "Void".to_string(),
            Value::Record(name, _) => format!("Record: {name}"),
            Value::Domain(name) => format!("Domain: {}", name.to_lowercase()),
            Value::Enum(type_name, variant) => format!("Enum: {type_name}.{variant}"),
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.kind_prefixed())
    }
}

/// Builds the Integer that carries a natural's magnitude.
fn natural_to_integer(value: &Nat) -> Int {
    Int::from_decimal(&value.to_decimal()).expect("a natural's decimal digits form an integer")
}

/// The reference's non-numeric widening refusal, including its Complex/Symbolic hints.
fn non_numeric_widen_error(from: ValueKind, target: ValueKind) -> LangError {
    LangError::plain(format!(
        "Cannot widen from {} to {}: only numeric kinds (Natural, Integer, Real) support widening.{}",
        from.name(),
        target.name(),
        widen_hint(from, target)
    ))
}

/// The hint the reference appends to a non-numeric widening refusal.
fn widen_hint(from: ValueKind, target: ValueKind) -> &'static str {
    if target == ValueKind::Complex {
        " Reductions over Complex arrays (sum/mean/dot/norm/matmul) are not supported; use re()/im()/conj()/abs() to bridge back to Real."
    } else if from == ValueKind::Complex {
        " Complex is a domain type; use re()/im()/conj()/abs() to bridge back to Real."
    } else if from == ValueKind::Symbolic || target == ValueKind::Symbolic {
        " Symbolic is a domain type; use subs()/evalf() to bridge back to numeric values."
    } else {
        ""
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use lovelace_int::Int;
    use lovelace_nat::Nat;
    use lovelace_real::Real;

    fn nat(text: &str) -> Nat {
        Nat::from_decimal(text).expect("the test literal is a natural")
    }

    fn int(text: &str) -> Int {
        Int::from_decimal(text).expect("the test literal is an integer")
    }

    fn real(text: &str) -> Real {
        Real::from_str(text).expect("the test literal is a real")
    }

    /// ValueTests.cs:46 Value_ToString_GivenNatural_PrefixesKindLabel.
    #[test]
    fn value_to_string_given_natural_prefixes_kind_label() {
        let value = Value::Natural(nat("42"));
        let rendered = value.to_string();
        assert!(rendered.starts_with("Natural:"), "{rendered}");
        assert!(rendered.contains("42"), "{rendered}");
        assert_eq!(rendered, "Natural: 42");
    }

    /// ValueTests.cs:60 Widen_GivenNaturalToInteger_ReturnsIntegerValue.
    #[test]
    fn widen_given_natural_to_integer_returns_integer_value() {
        let value = Value::Natural(nat("5"));
        let widened = value
            .widen(ValueKind::Integer)
            .expect("a Natural widens to an Integer");
        assert_eq!(widened.kind(), ValueKind::Integer);
        assert_eq!(
            widened
                .as_integer()
                .expect("an Integer payload")
                .to_decimal(),
            "5"
        );
    }

    /// ValueTests.cs:70 Widen_GivenNaturalToReal_ReturnsRealValue.
    #[test]
    fn widen_given_natural_to_real_returns_real_value() {
        let value = Value::Natural(nat("5"));
        let widened = value
            .widen(ValueKind::Real)
            .expect("a Natural widens to a Real");
        assert_eq!(widened.kind(), ValueKind::Real);
        assert_eq!(widened.as_real(), Some(&real("5")));
    }

    /// ValueTests.cs:80 Widen_GivenIntegerToReal_ReturnsRealValue.
    #[test]
    fn widen_given_integer_to_real_returns_real_value() {
        let value = Value::Integer(int("-3"));
        let widened = value
            .widen(ValueKind::Real)
            .expect("an Integer widens to a Real");
        assert_eq!(widened.kind(), ValueKind::Real);
        assert_eq!(widened.as_real(), Some(&real("-3")));
    }

    /// ValueTests.cs:90 Widen_GivenSameKind_ReturnsSameValue.
    #[test]
    fn widen_given_same_kind_returns_same_value() {
        let value = Value::Natural(nat("7"));
        let widened = value
            .widen(ValueKind::Natural)
            .expect("the same kind is not a narrowing");
        assert_eq!(widened.kind(), ValueKind::Natural);
        assert_eq!(widened, value);
        assert_eq!(widened.as_natural(), Some(&nat("7")));
    }

    /// ValueTests.cs:99 Widen_GivenWiderToNarrower_ThrowsInvalidOperationException.
    #[test]
    fn widen_given_wider_to_narrower_throws_invalid_operation_exception() {
        let value = Value::Integer(int("3"));
        let error = value
            .widen(ValueKind::Natural)
            .expect_err("an Integer cannot narrow to a Natural");
        assert_eq!(error.message, "Cannot narrow from Integer to Natural.");
        assert_eq!(error.code, "InvalidOperation");
    }

    /// ValueTests.cs:111 WidenPair_GivenNaturalAndInteger_ReturnsBothAsInteger.
    #[test]
    fn widen_pair_given_natural_and_integer_returns_both_as_integer() {
        let a = Value::Natural(nat("5"));
        let b = Value::Integer(int("3"));
        let (wa, wb) = Value::widen_pair(&a, &b).expect("both operands are numeric");
        assert_eq!(wa.kind(), ValueKind::Integer);
        assert_eq!(wb.kind(), ValueKind::Integer);
        assert_eq!(
            wa.as_integer().expect("an Integer payload").to_decimal(),
            "5"
        );
        assert_eq!(
            wb.as_integer().expect("an Integer payload").to_decimal(),
            "3"
        );
    }

    /// ValueTests.cs:123 WidenPair_GivenNaturalAndReal_ReturnsBothAsReal.
    #[test]
    fn widen_pair_given_natural_and_real_returns_both_as_real() {
        let a = Value::Natural(nat("5"));
        let b = Value::Real(real("3.14"));
        let (wa, wb) = Value::widen_pair(&a, &b).expect("both operands are numeric");
        assert_eq!(wa.kind(), ValueKind::Real);
        assert_eq!(wb.kind(), ValueKind::Real);
        assert_eq!(wa.as_real(), Some(&real("5")));
        assert_eq!(wb.as_real(), Some(&real("3.14")));
    }

    /// ValueTests.cs:135 WidenPair_GivenSameKind_ReturnsUnchanged.
    #[test]
    fn widen_pair_given_same_kind_returns_unchanged() {
        let a = Value::Natural(nat("5"));
        let b = Value::Natural(nat("3"));
        let (wa, wb) = Value::widen_pair(&a, &b).expect("the same kind needs no widening");
        assert_eq!(wa.kind(), ValueKind::Natural);
        assert_eq!(wb.kind(), ValueKind::Natural);
        assert_eq!(wa.as_natural(), Some(&nat("5")));
        assert_eq!(wb.as_natural(), Some(&nat("3")));
    }
}
