//! The compact real literal: a significand and a decimal exponent, carried WITHOUT expanding the
//! power of ten.
//!
//! The reference's `RealLiteral(new Int(15), 1_000_000_000)` prints as `15e1000000000` and parses
//! back to the same pair; the value has a billion digits, so a carrier that materialized them
//! would be unusable. This is that carrier: identity is the (significand, exponent) pair, the
//! rendering is compact, and `to_rational` refuses rather than materializing a huge power.

use lovelace_int::Int;
use lovelace_nat::Nat;

use crate::Rat;

/// A real literal in compact `significand e exponent` form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RealLiteral {
    significand: Int,
    exponent: i64,
}

impl RealLiteral {
    /// The literal `significand * 10^exponent`.
    pub fn new(significand: Int, exponent: i64) -> RealLiteral {
        RealLiteral {
            significand,
            exponent,
        }
    }

    /// The digits that carry the value.
    pub fn significand(&self) -> &Int {
        &self.significand
    }

    /// The decimal exponent.
    pub fn exponent(&self) -> i64 {
        self.exponent
    }

    /// Parses the compact form this type prints (`15e1000000000`, `-15e-3`, `7`).
    pub fn parse(text: &str) -> Option<RealLiteral> {
        let text = text.trim();
        if text.is_empty() {
            return None;
        }
        match text.find(['e', 'E']) {
            None => {
                let significand = Int::from_decimal(text)?;
                Some(RealLiteral::new(significand, 0))
            }
            Some(at) => {
                let significand = Int::from_decimal(&text[..at])?;
                let exponent = text[at + 1..].parse::<i64>().ok()?;
                Some(RealLiteral::new(significand, exponent))
            }
        }
    }

    /// The exact rational value, or None when the power of ten is wider than the materializer
    /// bound: a billion-digit denominator is refused, never silently truncated.
    pub fn to_rational(&self) -> Option<Rat> {
        let magnitude = self.exponent.unsigned_abs();
        if magnitude > 100_000 {
            return None;
        }
        let mut digits = String::from("1");
        for _ in 0..magnitude {
            digits.push('0');
        }
        if self.exponent >= 0 {
            Some(Rat::new(
                self.significand.mul(&Int::from_decimal(&digits)?),
                Nat::from_u64(1),
            ))
        } else {
            Some(Rat::new(self.significand.clone(), Nat::from_decimal(&digits)?))
        }
    }
}

impl std::fmt::Display for RealLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.exponent == 0 {
            f.write_str(&self.significand.to_decimal())
        } else {
            write!(f, "{}e{}", self.significand.to_decimal(), self.exponent)
        }
    }
}