//! The route-carrying real literal: a decimal rendering plus the exactness provenance of the
//! value it came from.
//!
//! The reference's `RealLiteral` stores digits and PROVENANCE, and `ToReal` restores both: a
//! genuinely exact value (`1/2`, `sqrt(4)`, a terminating quotient) comes back exact, while a
//! value produced by a truncating evaluation (`pi` cut to 30 places, a transcendental's series
//! result) comes back inexact even when its stored digits happen to be a finite decimal. This
//! module is that pair of operations at the library level; `lovelace_symbolics::RealLiteral` is
//! the compact `significand e exponent` carrier and deliberately has no route field.
//!
//! The C# row this exists for is
//! `TruncatingEvaluationStaysInexactTests.LiteralRoundTrip_KeepsGenuinelyExactValuesExact`: a
//! fix that cleared the flag everywhere would pass the truncated family and fail this control.

use lovelace_real::Real;

/// A real literal that carries both the digits and the exactness route of the value it was built
/// from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RealLiteral {
    digits: String,
    exact: bool,
}

impl RealLiteral {
    /// The reference's `RealLiteral.FromRealExact(value)`: the value's digits plus its route.
    pub fn from_real_exact(value: &Real) -> RealLiteral {
        RealLiteral {
            digits: value.to_string(),
            exact: value.is_exact(),
        }
    }

    /// The reference's `RealLiteral.ToReal()`: the same digits, with the route restored.
    ///
    /// The digits are a decimal rendering this type produced, so a parse failure is a defect in
    /// the literal, not a caller error.
    pub fn to_real(&self) -> Real {
        let value = Real::from_str(&self.digits).unwrap_or_else(|| {
            panic!(
                "a RealLiteral's digits are a decimal rendering and must parse: '{}'",
                self.digits
            )
        });
        if self.exact {
            value
        } else {
            value.mark_inexact()
        }
    }

    /// The stored digits, exactly as they crossed into the literal.
    pub fn digits(&self) -> &str {
        &self.digits
    }

    /// Whether this literal came from a value that dropped no digit.
    pub fn is_exact(&self) -> bool {
        self.exact
    }
}
