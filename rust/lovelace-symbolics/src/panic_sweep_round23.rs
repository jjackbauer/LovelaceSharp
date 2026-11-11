//! Round 23 symbolics panic-sweep regression tests.
//!
//! Every case here reached a panic (debug overflow or an out-of-range capacity) in the exact
//! kernel before the fix. The tests call the kernel directly so the guard is pinned where it
//! lives, independent of the language surface.

use super::*;

fn sum_of_squares_minus_one() -> Expr {
    let x = Expr::sym("x");
    Expr::add(Expr::pow(x, Expr::num(2)), Expr::num(-1))
}

#[test]
fn huge_exponent_degree_saturates_instead_of_multiplying_with_overflow() {
    let expression = Expr::pow(sum_of_squares_minus_one(), Expr::num(i64::MAX));
    assert_eq!(expression.degree_in("x"), i64::MAX);
    // Term ordering is the other caller of the same degree walk; it must not panic either.
    assert_eq!(expression.degree_in_total(), i64::MAX);
}

#[test]
fn collect_leaves_the_huge_power_alone() {
    let expression = Expr::pow(sum_of_squares_minus_one(), Expr::num(i64::MAX));
    let collected = collect(&expression, "x");
    assert_eq!(collected.to_string(), "(x^2 - 1)^9223372036854775807");
}

#[test]
fn poly_coeffs_refuses_a_degree_it_cannot_materialize() {
    let x = Expr::sym("x");
    let huge = Expr::pow(x.clone(), Expr::num(i64::MAX));
    assert!(poly_coeffs(&huge, "x").is_none());

    // The bound is a bound, not a ban: an ordinary degree still materializes.
    let ordinary = Expr::pow(x, Expr::num(4));
    let coeffs = poly_coeffs(&ordinary, "x").expect("x^4 is a polynomial");
    assert_eq!(coeffs.len(), 5);
    assert!(coeffs[4].is_one());
}

#[test]
fn a_polynomial_with_the_i64_min_coefficient_does_not_panic_in_root_search() {
    // find_rational_root used i64::MIN.abs(), which panics; the coefficient is reachable from
    // the language as -9223372036854775808.
    let coeffs = vec![Rat::from_i64(i64::MIN), Rat::from_i64(1), Rat::from_i64(1)];
    let outcome = solve_polynomial(&coeffs);
    // The constant is far below i64, but the polynomial has a positive discriminant, so the
    // exact-radical path answers two algebraic roots instead of failing. What this row pins is
    // that the root SEARCH returns rather than panicking in i64::MIN.abs().
    assert!(
        matches!(&outcome, SolveOutcome::RootExpressions(roots) if roots.len() == 2),
        "a degree-2 polynomial with this constant must return its two exact roots, got {outcome:?}"
    );
}

#[test]
fn a_negative_i64_min_exponent_is_refused_not_negated() {
    assert!(Rat::from_i64(2).pow_i64(i64::MIN).is_none());
}

