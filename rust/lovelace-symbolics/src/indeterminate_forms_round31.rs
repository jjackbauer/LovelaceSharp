//! Round 31: the indeterminate forms of the arithmetic identities (the N18 family).
//!
//! Transcribed from `Lovelace.Symbolics.Tests/IndeterminateFormTests.cs`. The identities
//! `a - a = 0` and `0 * a = 0` are valid over DEFINED, FINITE values only; with `a` bound to the
//! symbolic constant infinity they are indeterminate, so the zero-coefficient / zero-factor folds
//! must keep the form visible instead of erasing it. The positive controls pin the identities for
//! symbols and for finite named constants (Pi, E), which keep folding.

use crate::Expr;

fn infinity() -> Expr {
    Expr::Fun("inf".to_string(), Vec::new())
}

fn pi() -> Expr {
    Expr::Fun("pi".to_string(), Vec::new())
}

fn e_constant() -> Expr {
    Expr::Fun("e".to_string(), Vec::new())
}

/// A definite zero: the folded rational constant 0.
fn is_definite_zero(e: &Expr) -> bool {
    matches!(e, Expr::Num(n) if n.is_zero())
}

fn mentions_infinity(e: &Expr) -> bool {
    match e {
        Expr::Fun(name, args) => {
            (name == "inf" && args.is_empty()) || args.iter().any(mentions_infinity)
        }
        Expr::Add(v) | Expr::Mul(v) => v.iter().any(mentions_infinity),
        Expr::Pow(b, x) => mentions_infinity(b) || mentions_infinity(x),
        Expr::Div(a, b) => mentions_infinity(a) || mentions_infinity(b),
        Expr::Rel(_, a, b) => mentions_infinity(a) || mentions_infinity(b),
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// N18: the defect — an indeterminate form must not return a definite 0
// ---------------------------------------------------------------------------

#[test]
fn infinity_minus_infinity_is_not_definite_zero() {
    let e = Expr::sub(infinity(), infinity());
    assert!(!is_definite_zero(&e), "inf - inf must not fold to a definite 0; got: {e}");
    assert!(mentions_infinity(&e), "the infinity operand must survive the fold; got: {e}");
    // The chosen outcome: the canonical indeterminate product 0*inf, exactly the treatment
    // 0/x gets (the zero coefficient stays visible over a hazardous remainder).
    assert_eq!(e.to_string(), "0*inf");
}

#[test]
fn zero_times_infinity_is_not_definite_zero() {
    let e = Expr::mul(Expr::num(0), infinity());
    assert!(!is_definite_zero(&e), "0 * inf must not fold to a definite 0; got: {e}");
    assert!(mentions_infinity(&e), "the infinity factor must survive the fold; got: {e}");
    assert_eq!(e.to_string(), "0*inf");
}

#[test]
fn infinity_plus_one_stays_unevaluated() {
    // Control: the honest treatment that inf - inf and 0 * inf must match.
    let e = Expr::add(infinity(), Expr::num(1));
    assert!(!is_definite_zero(&e), "inf + 1 has no numeric value; got: {e}");
    assert!(mentions_infinity(&e), "inf must survive; got: {e}");
}

#[test]
fn nested_infinity_difference_is_not_definite_zero() {
    // The same hole one level down: the guard must look through the operand structure.
    let x = Expr::sym("x");
    let sum = Expr::add(infinity(), x);
    let e = Expr::sub(sum.clone(), sum);
    assert!(
        !is_definite_zero(&e),
        "(inf + x) - (inf + x) is indeterminate and must not fold to a definite 0; got: {e}"
    );
    assert!(mentions_infinity(&e), "inf must survive; got: {e}");
}

#[test]
fn indeterminate_difference_embedded_in_sum_does_not_resurrect_zero() {
    // The Add path re-applies the same guard: (inf - inf) + 1 must not become 1.
    let e = Expr::add(Expr::sub(infinity(), infinity()), Expr::num(1));
    assert!(!is_definite_zero(&e), "(inf - inf) + 1 must not fold to 1; got: {e}");
    assert!(mentions_infinity(&e), "inf must survive; got: {e}");
}

// ---------------------------------------------------------------------------
// Forbidden-regression guards: the identities stay valid where they are valid
// ---------------------------------------------------------------------------

#[test]
fn symbol_minus_itself_still_folds_to_zero() {
    let x = Expr::sym("x");
    assert_eq!(Expr::sub(x.clone(), x), Expr::num(0));
}

#[test]
fn zero_times_symbol_still_folds_to_zero() {
    let x = Expr::sym("x");
    assert_eq!(Expr::mul(Expr::num(0), x), Expr::num(0));
}

#[test]
fn finite_named_constant_identities_still_fold() {
    // Pi and E are named but finite: pi - pi = 0 and 0 * pi = 0 remain correct.
    assert_eq!(Expr::sub(pi(), pi()), Expr::num(0));
    assert_eq!(Expr::mul(Expr::num(0), pi()), Expr::num(0));
    assert_eq!(Expr::sub(e_constant(), e_constant()), Expr::num(0));
}

// ---------------------------------------------------------------------------
// Sibling identities in the same fold code (the N18 sweep)
// ---------------------------------------------------------------------------

#[test]
fn sibling_infinity_over_infinity_stays_unevaluated() {
    let e = Expr::div(infinity(), infinity());
    assert!(!is_definite_zero(&e), "inf/inf must not be 0; got: {e}");
    assert!(!matches!(&e, Expr::Num(n) if n.is_one()), "inf/inf must not be 1; got: {e}");
}

#[test]
fn sibling_zero_over_zero_is_not_definite_zero() {
    let e = Expr::div(Expr::num(0), Expr::num(0));
    assert!(!is_definite_zero(&e), "0/0 must not fold to 0; got: {e}");
}

#[test]
fn sibling_infinity_to_the_zero() {
    // inf^0 = 1 is preserved by convention (matching 0^0 = 1 in this kernel) and is NOT the N18
    // root cause; this test documents the decision rather than changing it.
    assert_eq!(Expr::pow(infinity(), Expr::num(0)), Expr::num(1));
}
