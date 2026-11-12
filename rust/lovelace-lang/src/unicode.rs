//! Unicode symbolic rendering for the engine's display-precision formatter.
//!
//! The reference's `ValueFormatter.Format(value, unicode)` calls
//! `Printing.PrettyPrint(expr, PrintOptions(Unicode: true))`; that printer differs from the ASCII
//! form in exactly two places (Lovelace.Symbolics/Printing.cs:646-666): the named constants
//! `pi`/`inf` become π/∞, and the relations `!=`/`<=`/>=` become ≠/≤/≥. Everything else —
//! including `sqrt(...)`, products, powers and precedence — is byte-identical.
//!
//! This crate owns the engine formatter, while the expression printer lives in the symbolics
//! crate (which another lane is editing). The implementation therefore delegates the structural
//! rendering to the existing ASCII printer after marking the two named constants with sentinels
//! that cannot be ordinary symbols, and substitutes the relation tokens afterwards. A user symbol
//! that happens to be spelled `inf`/`pi` is a distinct `Expr::Sym` node and is left untouched.

use lovelace_symbolics::Expr;

const INFINITY_MARKER: &str = "__lovelace_unicode_infinity__";
const PI_MARKER: &str = "__lovelace_unicode_pi__";

fn mark_named_constants(expr: &Expr) -> Expr {
    match expr {
        Expr::Fun(name, arguments) if arguments.is_empty() && name == "inf" => {
            Expr::sym(INFINITY_MARKER)
        }
        Expr::Fun(name, arguments) if arguments.is_empty() && name == "pi" => Expr::sym(PI_MARKER),
        Expr::Add(terms) => Expr::Add(terms.iter().map(mark_named_constants).collect()),
        Expr::Mul(factors) => Expr::Mul(factors.iter().map(mark_named_constants).collect()),
        Expr::Pow(base, exponent) => Expr::Pow(
            Box::new(mark_named_constants(base)),
            Box::new(mark_named_constants(exponent)),
        ),
        Expr::Fun(name, arguments) => Expr::Fun(
            name.clone(),
            arguments.iter().map(mark_named_constants).collect(),
        ),
        Expr::Div(numerator, denominator) => Expr::Div(
            Box::new(mark_named_constants(numerator)),
            Box::new(mark_named_constants(denominator)),
        ),
        Expr::Rel(operator, left, right) => Expr::Rel(
            *operator,
            Box::new(mark_named_constants(left)),
            Box::new(mark_named_constants(right)),
        ),
        other => other.clone(),
    }
}

/// The Unicode form of one symbolic expression, mirroring the reference's Pretty printer with
/// `Unicode: true`.
pub fn symbolic(expr: &Expr) -> String {
    mark_named_constants(expr)
        .to_string()
        .replace(INFINITY_MARKER, "∞")
        .replace(PI_MARKER, "π")
        .replace(" != ", " ≠ ")
        .replace(" <= ", " ≤ ")
        .replace(" >= ", " ≥ ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_constants_and_relations_use_the_unicode_glyphs() {
        assert_eq!(symbolic(&Expr::Fun("inf".to_string(), Vec::new())), "∞");
        assert_eq!(symbolic(&Expr::Fun("pi".to_string(), Vec::new())), "π");
        assert_eq!(
            symbolic(&Expr::Rel(
                lovelace_symbolics::RelOp::Ne,
                Box::new(Expr::sym("x")),
                Box::new(Expr::num(0))
            )),
            "x ≠ 0"
        );
        assert_eq!(
            symbolic(&Expr::sym("inf")),
            "inf",
            "a user symbol spelled inf is not the named constant"
        );
    }

    #[test]
    fn the_ascii_structure_is_preserved() {
        assert_eq!(
            symbolic(&Expr::add(
                Expr::num(1),
                Expr::Fun("inf".to_string(), Vec::new())
            )),
            "∞ + 1"
        );
    }
}
