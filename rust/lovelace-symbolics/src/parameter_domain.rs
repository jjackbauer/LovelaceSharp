//! The solver's parameter-domain enum and its total projection onto the domain lattice.
//!
//! The C# reference declares `ParameterDomain { Integers, NonNegativeIntegers }` beside
//! `SolutionFamily` and projects it to the first-class `MathDomain` payload with a total switch:
//! both declared arms answer `Integer`, and an undeclared ordinal — `(ParameterDomain)99` — is
//! refused instead of silently inheriting `Integer`. This module is that enum and that projection
//! in Rust, where `Dom` is the kernel lattice the reference's `MathDomain` crosses the wire as.
//!
//! A C# enum is an integer and can therefore hold a value no arm declares; a Rust enum cannot.
//! The raw representation is modelled by the ordinal entry points: [`ParameterDomain::from_ordinal`]
//! resolves a reference ordinal (refusing an undeclared one) and
//! [`parameter_domain_of_ordinal`] is the total mapping over that representation, mirroring
//! `SymbolicsPlugin.ParameterDomainOf((ParameterDomain)99)`.

use std::fmt;

use crate::Dom;

/// The domain of a solution-family parameter (the reference's `ParameterDomain`).
///
/// The discriminants are the reference ordinals: `Integers` is 0 and `NonNegativeIntegers` is 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum ParameterDomain {
    /// The parameter ranges over every integer, k in Z.
    Integers = 0,
    /// The parameter ranges over the non-negative integers. The non-negativity constraint is
    /// carried by the family's conditions, never by the wire domain.
    NonNegativeIntegers = 1,
}

impl ParameterDomain {
    /// Every declared arm, in ordinal order — the Rust analogue of
    /// `Enum.GetValues<ParameterDomain>()`.
    pub const ALL: [ParameterDomain; 2] = [
        ParameterDomain::Integers,
        ParameterDomain::NonNegativeIntegers,
    ];

    /// The reference ordinal of this arm.
    pub const fn ordinal(self) -> i32 {
        self as i32
    }

    /// Resolves a reference ordinal, refusing a value the C# enum does not declare.
    ///
    /// This is the Rust form of the cast that produces `(ParameterDomain)99`: the value exists as
    /// an ordinal, and it has no declared arm.
    pub fn from_ordinal(ordinal: i32) -> Result<ParameterDomain, UndeclaredParameterDomain> {
        match ordinal {
            0 => Ok(ParameterDomain::Integers),
            1 => Ok(ParameterDomain::NonNegativeIntegers),
            _ => Err(UndeclaredParameterDomain { ordinal }),
        }
    }

    /// The kernel domain this parameter domain projects onto (the reference's
    /// `ParameterDomainOf` for one declared arm).
    ///
    /// Both declared arms project to `Dom::Integer`: a non-negative parameter's constraint rides
    /// on the family's conditions, not on the domain.
    pub const fn math_domain(self) -> Dom {
        match self {
            ParameterDomain::Integers | ParameterDomain::NonNegativeIntegers => Dom::Integer,
        }
    }
}

/// The refusal for a parameter-domain ordinal the reference enum does not declare.
///
/// `Display` reproduces the reference exception's message body, parameter name and actual value:
/// `No wire Domain is declared for this parameter domain. (Parameter 'domain') Actual value was 99.`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UndeclaredParameterDomain {
    /// The ordinal that has no declared arm.
    pub ordinal: i32,
}

impl UndeclaredParameterDomain {
    /// The reference's parameter name for the refused argument.
    pub const PARAMETER_NAME: &'static str = "domain";
}

impl fmt::Display for UndeclaredParameterDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "No wire Domain is declared for this parameter domain. (Parameter '{}') Actual value was {}.",
            UndeclaredParameterDomain::PARAMETER_NAME,
            self.ordinal
        )
    }
}

impl std::error::Error for UndeclaredParameterDomain {}

/// The total projection a declared parameter domain makes onto the kernel domain lattice.
///
/// The switch is exhaustive over the declared arms, exactly as the reference's is. An undeclared
/// ordinal never reaches this function: [`ParameterDomain::from_ordinal`] refuses it first, and
/// [`parameter_domain_of_ordinal`] is the entry point that combines the two.
pub const fn parameter_domain_of(domain: ParameterDomain) -> Dom {
    domain.math_domain()
}

/// The total mapping over the reference's raw enum representation.
///
/// Every ordinal either projects to its declared kernel domain or is refused with the reference's
/// message. In particular `99` is a decision that has not been made, not a silent `Integer`.
pub fn parameter_domain_of_ordinal(ordinal: i32) -> Result<Dom, UndeclaredParameterDomain> {
    Ok(parameter_domain_of(ParameterDomain::from_ordinal(ordinal)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_declared_arm_projects_to_the_integer_domain() {
        for arm in ParameterDomain::ALL {
            assert_eq!(parameter_domain_of(arm), Dom::Integer);
            assert_eq!(parameter_domain_of_ordinal(arm.ordinal()), Ok(Dom::Integer));
        }
    }

    #[test]
    fn undeclared_ordinal_is_refused_with_the_reference_message() {
        let error = parameter_domain_of_ordinal(99).expect_err("99 has no declared arm");
        assert_eq!(error.ordinal, 99);
        assert_eq!(ParameterDomain::from_ordinal(99), Err(error));
        assert_eq!(
            error.to_string(),
            "No wire Domain is declared for this parameter domain. (Parameter 'domain') Actual value was 99."
        );
    }
}
