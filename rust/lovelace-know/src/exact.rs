//! Exact rational arithmetic for Lovelace numeric literals.
//!
//! The C# pipeline models every operand, axis position and guard threshold as an exact
//! rational (Lovelace.Knowledge ExactNumber) so that boundary detection never sees a
//! floating-point artefact. This module preserves that representation, including the
//! round trip with the Lovelace literal syntax: whole numbers, terminating decimals, and
//! periodic fractions such as 0.(3) and 0.1(6).

use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;

use lovelace_int::{Int, Nat};

/// A failure while parsing or normalizing a number literal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NumberParseError {
    message: String,
}

impl NumberParseError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Returns the human-readable reason.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for NumberParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for NumberParseError {}

/// An exact rational number in canonical form: lowest terms, positive denominator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactNumber {
    num: Int,
    den: Nat,
}

impl ExactNumber {
    /// Builds a normalized rational. Returns None when the denominator is zero.
    #[must_use]
    pub fn new(num: Int, den: Nat) -> Option<Self> {
        if den.is_zero() {
            return None;
        }
        let divisor = num.magnitude().gcd(&den);
        if divisor.is_one() {
            return Some(Self { num, den });
        }
        let (num, num_remainder) = num.div_rem(&divisor)?;
        let (den, den_remainder) = den.div_rem(&divisor)?;
        if !num_remainder.is_zero() || !den_remainder.is_zero() {
            return None;
        }
        Some(Self { num, den })
    }

    /// The additive identity.
    #[must_use]
    pub fn zero() -> Self {
        Self {
            num: Int::zero(),
            den: Nat::from_u64(1),
        }
    }

    /// The multiplicative identity.
    #[must_use]
    pub fn one() -> Self {
        Self {
            num: Int::from_i64(1),
            den: Nat::from_u64(1),
        }
    }

    /// Builds an exact integer.
    #[must_use]
    pub fn from_i64(value: i64) -> Self {
        Self {
            num: Int::from_i64(value),
            den: Nat::from_u64(1),
        }
    }

    /// The signed numerator.
    #[must_use]
    pub fn num(&self) -> &Int {
        &self.num
    }

    /// The non-negative denominator.
    #[must_use]
    pub fn den(&self) -> &Nat {
        &self.den
    }

    /// Whether the value is exactly zero.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.num.is_zero()
    }

    /// Whether the value is strictly negative.
    #[must_use]
    pub fn is_negative(&self) -> bool {
        self.num.is_negative()
    }

    /// Whether the value is a whole number.
    #[must_use]
    pub fn is_integer(&self) -> bool {
        self.den.is_one()
    }

    /// Exact addition.
    #[must_use]
    pub fn add(&self, other: &Self) -> Self {
        let num = self
            .num
            .mul(&Int::from_nat(other.den.clone()))
            .add(&other.num.mul(&Int::from_nat(self.den.clone())));
        let den = self.den.mul(&other.den);
        Self::new(num, den).expect("a product of non-zero denominators is non-zero")
    }

    /// Exact subtraction.
    #[must_use]
    pub fn subtract(&self, other: &Self) -> Self {
        let num = self
            .num
            .mul(&Int::from_nat(other.den.clone()))
            .sub(&other.num.mul(&Int::from_nat(self.den.clone())));
        let den = self.den.mul(&other.den);
        Self::new(num, den).expect("a product of non-zero denominators is non-zero")
    }

    /// Parses a Lovelace numeric literal: 42, -7, 3.14, .5, 0.(3), 0.1(6), -1.2(34).
    pub fn parse(literal: &str) -> Result<Self, NumberParseError> {
        let trimmed = literal.trim();
        if trimmed.is_empty() {
            return Err(NumberParseError::new(
                "cannot parse an empty number literal",
            ));
        }
        let (negative, body) = match trimmed.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, trimmed.strip_prefix('+').unwrap_or(trimmed)),
        };
        let dot = body.find('.');
        let open = body.find('(');

        let (integer_part, fraction_part, period_part) = if let Some(open) = open {
            if let Some(dot) = dot {
                if open < dot {
                    return Err(NumberParseError::new(format!(
                        "malformed periodic literal '{literal}'"
                    )));
                }
            }
            if !body.ends_with(')') {
                return Err(NumberParseError::new(format!(
                    "unterminated period in '{literal}'"
                )));
            }
            match dot {
                Some(dot) => (
                    &body[..dot],
                    &body[dot + 1..open],
                    Some(&body[open + 1..body.len() - 1]),
                ),
                None => (&body[..open], "", Some(&body[open + 1..body.len() - 1])),
            }
        } else if let Some(dot) = dot {
            (&body[..dot], &body[dot + 1..], None)
        } else {
            (body, "", None)
        };

        if let Some(period) = period_part {
            if period.is_empty() {
                return Err(NumberParseError::new(format!(
                    "empty period in '{literal}'"
                )));
            }
        }

        let integer_value = parse_decimal(integer_part, literal)?;
        let fraction_value = parse_decimal(fraction_part, literal)?;
        let fraction_scale = pow10(fraction_part.len());

        let (num, den) = match period_part {
            None => {
                let num = Int::from_nat(integer_value)
                    .mul(&Int::from_nat(fraction_scale.clone()))
                    .add(&Int::from_nat(fraction_value));
                (num, fraction_scale)
            }
            Some(period) => {
                let period_value = parse_decimal(period, literal)?;
                let period_scale = pow10(period.len());
                let rep_unit = period_scale
                    .sub(&Nat::from_u64(1))
                    .ok_or_else(|| NumberParseError::new("empty period"))?;
                let num = Int::from_nat(integer_value)
                    .mul(&Int::from_nat(fraction_scale.clone()))
                    .mul(&Int::from_nat(rep_unit.clone()))
                    .add(&Int::from_nat(fraction_value).mul(&Int::from_nat(rep_unit.clone())))
                    .add(&Int::from_nat(period_value));
                let den = fraction_scale.mul(&rep_unit);
                (num, den)
            }
        };

        let num = if negative { num.negate() } else { num };
        Self::new(num, den)
            .ok_or_else(|| NumberParseError::new(format!("cannot normalize '{literal}'")))
    }

    /// Emits a Lovelace literal that parses back to this exact value.
    #[must_use]
    pub fn to_lovelace_literal(&self) -> String {
        if self.is_integer() {
            return self.num.to_decimal();
        }
        let negative = self.num.is_negative();
        let magnitude = self.num.magnitude();
        let (integer_part, mut remainder) = magnitude
            .div_rem(&self.den)
            .expect("the denominator is never zero");
        let ten = Nat::from_u64(10);
        // Nat's interior mutability is a memoized limb/decimal cache; Eq and Hash are
        // value-based, so a Nat is a sound key and the lint is a false positive here.
        #[allow(clippy::mutable_key_type)]
        let mut seen: HashMap<Nat, usize> = HashMap::new();
        let mut digits: Vec<u8> = Vec::new();
        let mut period_start: Option<usize> = None;
        while !remainder.is_zero() {
            if let Some(start) = seen.get(&remainder) {
                period_start = Some(*start);
                break;
            }
            seen.insert(remainder.clone(), digits.len());
            let shifted = remainder.mul(&ten);
            let (digit, next) = shifted
                .div_rem(&self.den)
                .expect("the denominator is never zero");
            digits.push(
                u8::try_from(digit.to_u64().expect("a decimal digit fits in u64"))
                    .expect("a decimal digit fits in u8"),
            );
            remainder = next;
        }
        let mut out = String::new();
        if negative {
            out.push('-');
        }
        out.push_str(&integer_part.to_decimal());
        out.push('.');
        for (index, digit) in digits.iter().enumerate() {
            if period_start == Some(index) {
                out.push('(');
            }
            out.push(char::from(b'0' + *digit));
        }
        if period_start.is_some() {
            out.push(')');
        }
        out
    }
}

impl PartialOrd for ExactNumber {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ExactNumber {
    fn cmp(&self, other: &Self) -> Ordering {
        let left = self.num.mul(&Int::from_nat(other.den.clone()));
        let right = other.num.mul(&Int::from_nat(self.den.clone()));
        left.cmp(&right)
    }
}

impl fmt::Display for ExactNumber {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_lovelace_literal())
    }
}

fn parse_decimal(text: &str, literal: &str) -> Result<Nat, NumberParseError> {
    let trimmed = text.trim_start_matches('0');
    if trimmed.is_empty() {
        return Ok(Nat::zero());
    }
    if !trimmed.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(NumberParseError::new(format!(
            "malformed number literal '{literal}'"
        )));
    }
    Nat::from_decimal(trimmed)
        .ok_or_else(|| NumberParseError::new(format!("malformed number literal '{literal}'")))
}

fn pow10(exponent: usize) -> Nat {
    let mut value = Nat::from_u64(1);
    let ten = Nat::from_u64(10);
    for _ in 0..exponent {
        value = value.mul(&ten);
    }
    value
}
