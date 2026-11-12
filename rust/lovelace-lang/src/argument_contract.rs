//! The exact-integer argument contract's typed refusals.
//!
//! A call site that needs an integer has to tell a CALLER mistake apart from a DOMAIN failure.
//! The reference freezes the distinction in `ArgumentContractException`: `NotAnInteger` is a
//! payload that denotes no integer, and `NotAnIntegerKind` is a payload that DOES denote one but
//! arrived as a Real. Both cross as the recoverable `InvalidArgument`/`TypeMismatch` pair, so a
//! consumer can branch on "you gave me an argument I cannot honour" rather than on "your
//! mathematics is undefined".
//!
//! The constructors are public because the contract is a LIBRARY surface: a host embedding the
//! engine, or a plugin's own call site, must be able to build the exact refusal without going
//! through a builtin that happens to reach it.

use crate::LangError;

/// The quoted-name head every refusal starts with (`movingavg(): argument 1 (w)`). Position 0 is
/// the quoted-name form a range or index call site uses.
fn head(builtin: &str, parameter: &str, position: usize) -> String {
    if position == 0 {
        format!("{builtin}(): argument '{parameter}'")
    } else {
        format!("{builtin}(): argument {position} ({parameter})")
    }
}

fn refuse(message: String) -> LangError {
    LangError::coded(message, 0, "InvalidArgument", "TypeMismatch")
}

/// The wrong-KIND refusal for a payload that IS integral but is not a Natural/Integer: the caller
/// is told to write it as the integer it denotes (the reference's
/// `ArgumentContractException.NotAnIntegerKind`).
pub fn not_an_integer_kind(
    builtin: &str,
    parameter: &str,
    position: usize,
    text: &str,
) -> LangError {
    refuse(format!(
        "{} must be a Natural or Integer; got the Real {text}, which is integral — write it as {text}.",
        head(builtin, parameter, position)
    ))
}

/// The not-an-integer refusal for a payload that denotes no integer (the reference's
/// `ArgumentContractException.NotAnInteger`). `arrived` is the payload's own description, so a
/// Real reads `the Real 3.7` and a Text simply `Text`.
pub fn not_an_integer(builtin: &str, parameter: &str, position: usize, arrived: &str) -> LangError {
    refuse(format!(
        "{} must be a Natural or Integer that denotes an exact integer; got {arrived}.",
        head(builtin, parameter, position)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Lovelace.Suite.Tests.DspIntegerArgumentContractTests.TheExactlyIntegralRealRefusal_IsStillTheContractsContract`:
    /// the contract's wrong-KIND refusal is reachable without a builtin, names the integer to
    /// write, and crosses as the recoverable argument pair.
    #[test]
    fn an_integral_real_is_refused_by_kind_and_named() {
        let refusal = not_an_integer_kind("movingavg", "w", 1, "3");

        assert_eq!(refusal.code, "InvalidArgument");
        assert_eq!(refusal.category, "TypeMismatch");
        assert!(
            refusal.message.contains("write it as 3"),
            "{}",
            refusal.message
        );
        assert!(
            refusal
                .message
                .starts_with("movingavg(): argument 1 (w) must be a Natural or Integer;"),
            "{}",
            refusal.message
        );
        // The same shape the reference's ArgumentException carries: a caller mistake, not a
        // domain failure.
        assert_ne!(refusal.category, "DomainError");
    }

    /// The sibling arm: a payload that denotes no integer names what arrived.
    #[test]
    fn a_non_integral_real_is_refused_and_names_the_payload() {
        let refusal = not_an_integer("movingavg", "w", 1, "the Real 3.7");

        assert_eq!(refusal.code, "InvalidArgument");
        assert_eq!(refusal.category, "TypeMismatch");
        assert!(
            refusal.message.ends_with("got the Real 3.7."),
            "{}",
            refusal.message
        );
    }
}
