//! FalsificationTests.cs: the ten probe-API rows.

use super::*;

#[test]
fn trig_pythagorean_universal() {
    let lhs = Expr::add(
        Expr::pow(fun("sin", vec![x()]), Expr::num(2)),
        Expr::pow(fun("cos", vec![x()]), Expr::num(2)),
    );
    fuzz_identity(
        "trig.pythagorean-sin2-cos2",
        &lhs,
        &Expr::num(1),
        &all_points(),
    );
}

#[test]
fn sqrt_square_non_negative() {
    let lhs = Expr::pow(Expr::pow(x(), Expr::num(2)), Expr::Num(rational(1, 2)));
    let points: Vec<Rat> = all_points()
        .into_iter()
        .filter(|point| point >= &Rat::zero())
        .collect();
    fuzz_identity("pow.sqrt-square-nonnegative", &lhs, &x(), &points);
}

#[test]
fn sqrt_square_real() {
    let lhs = Expr::pow(Expr::pow(x(), Expr::num(2)), Expr::Num(rational(1, 2)));
    let rhs = fun("abs", vec![x()]);
    fuzz_identity("pow.sqrt-square-real", &lhs, &rhs, &all_points());
}

#[test]
fn exp_log_universal_on_positive_points() {
    let lhs = fun("exp", vec![fun("log", vec![x()])]);
    let points: Vec<Rat> = all_points()
        .into_iter()
        .filter(|point| point > &Rat::zero())
        .collect();
    fuzz_identity("logexp.exp-log", &lhs, &x(), &points);
}

#[test]
fn log_exp_real() {
    let lhs = fun("log", vec![fun("exp", vec![x()])]);
    fuzz_identity("logexp.log-exp", &lhs, &x(), &all_points());
}

#[test]
fn abs_square_real() {
    let lhs = Expr::pow(fun("abs", vec![x()]), Expr::num(2));
    let rhs = Expr::pow(x(), Expr::num(2));
    fuzz_identity("abs.abs-square", &lhs, &rhs, &all_points());
}

#[test]
fn abs_square_is_not_universal_over_complex() {
    let lhs = Expr::pow(fun("abs", vec![x()]), Expr::num(2));
    let rhs = Expr::pow(x(), Expr::num(2));
    let value = complex_value("1.5", "2");
    let left = default_eval(&lhs, "x", &value).expect("the left side evaluates");
    let right = default_eval(&rhs, "x", &value).expect("the right side evaluates");
    let delta = left
        .sub(&right, PRECISION)
        .expect("the difference evaluates");
    let magnitude = num_abs(&delta);
    assert_eq!(
        num_cmp(&magnitude, &Num::Exact(Rat::zero())),
        Some(Ordering::Greater),
        "|z|^2 and z^2 must differ at 1.5+2i"
    );
}

#[test]
fn abs_neg_universal() {
    let lhs = fun("abs", vec![Expr::neg(x())]);
    let rhs = fun("abs", vec![x()]);
    fuzz_identity("abs.abs-neg", &lhs, &rhs, &all_points());
}

#[test]
fn cancel_x_over_x_non_zero_points() {
    let lhs = Expr::div(x(), x());
    let points: Vec<Rat> = all_points()
        .into_iter()
        .filter(|point| !point.is_zero())
        .collect();
    fuzz_identity("rat.cancel-x-over-x", &lhs, &Expr::num(1), &points);
}

#[test]
fn cancel_zero_over_x_non_zero_points() {
    let lhs = Expr::div(Expr::num(0), x());
    let points: Vec<Rat> = all_points()
        .into_iter()
        .filter(|point| !point.is_zero())
        .collect();
    fuzz_identity("rat.cancel-zero-over-x", &lhs, &Expr::num(0), &points);
}
