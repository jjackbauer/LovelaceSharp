//! The structured diagnostic vocabulary's category type.
//!
//! The reference freezes the seven `Lovelace.Abstractions.ErrorCategory` members and publishes
//! their ORDER as part of the contract; the wire carries the member's name, and
//! `DiagnosticContractTests.ErrorCategory_HasExactlyTheSevenFrozenMembers` asserts that no member
//! is added, removed or reordered. The engine's `LangError.category` is the wire name, so this
//! type is the typed view of the same vocabulary: `from_name` and `name` are lossless, and
//! keeping the order here is what makes the contract checkable in Rust.

/// The seven frozen error categories, in the reference's declared order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ErrorCategory {
    /// The source could not be parsed.
    ParseError,
    /// The value or operation is outside the domain.
    DomainError,
    /// The runtime does not offer the operation.
    UnsupportedOperation,
    /// A declared work budget was exhausted (including cancellation).
    BudgetExceeded,
    /// The equation or system has no solution.
    NoSolution,
    /// An argument's kind does not match the declared shape.
    TypeMismatch,
    /// An internal invariant was violated.
    InternalInvariantFailure,
}

impl ErrorCategory {
    /// Every member, in the frozen order.
    pub const ALL: [ErrorCategory; 7] = [
        ErrorCategory::ParseError,
        ErrorCategory::DomainError,
        ErrorCategory::UnsupportedOperation,
        ErrorCategory::BudgetExceeded,
        ErrorCategory::NoSolution,
        ErrorCategory::TypeMismatch,
        ErrorCategory::InternalInvariantFailure,
    ];

    /// The member's wire name.
    pub fn name(self) -> &'static str {
        match self {
            ErrorCategory::ParseError => "ParseError",
            ErrorCategory::DomainError => "DomainError",
            ErrorCategory::UnsupportedOperation => "UnsupportedOperation",
            ErrorCategory::BudgetExceeded => "BudgetExceeded",
            ErrorCategory::NoSolution => "NoSolution",
            ErrorCategory::TypeMismatch => "TypeMismatch",
            ErrorCategory::InternalInvariantFailure => "InternalInvariantFailure",
        }
    }

    /// Resolves a wire name. Unknown names are `None`: the vocabulary is closed.
    pub fn from_name(name: &str) -> Option<ErrorCategory> {
        ErrorCategory::ALL
            .into_iter()
            .find(|category| category.name() == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_names_are_frozen_and_round_trip() {
        let names: Vec<&str> = ErrorCategory::ALL.iter().map(|c| c.name()).collect();
        assert_eq!(
            names,
            vec![
                "ParseError",
                "DomainError",
                "UnsupportedOperation",
                "BudgetExceeded",
                "NoSolution",
                "TypeMismatch",
                "InternalInvariantFailure",
            ]
        );
        for category in ErrorCategory::ALL {
            assert_eq!(ErrorCategory::from_name(category.name()), Some(category));
        }
        assert_eq!(ErrorCategory::from_name("InternalError"), None);
    }
}
