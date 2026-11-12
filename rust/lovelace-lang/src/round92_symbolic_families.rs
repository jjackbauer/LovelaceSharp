//! Round 92 — the language half of the six small deferred symbolic families.
//!
//! The rows driven here are the ones whose observable crosses the numeric tier or the
//! interpreter:
//!
//! * the two `ComplexMathInexactArgumentProvenanceTests` symbolic/numeric agreement rows, whose
//!   library surface is `Expr::real` + `Evaluation.EvaluateToNum`;
//! * the two `LinsolveCheckTests` rows, which substitute numeric bindings into the solution
//!   vector `SymbolicMatrix::solve` publishes;
//! * `RecursionBudgetCompositionTests` shapes #14/#15, the deepest accepted call answering from
//!   a `for` body and a `while` body (the loop's value is the last completed body value).
//!
//! Every assertion is the oracle's own observable; no test states a weaker approximation.

use std::cmp::Ordering;

use lovelace_real::Real;
use lovelace_symbolics::symbolic_matrix::SymbolicMatrix;
use lovelace_symbolics::{Expr, Rat};

use crate::evaluation_boundary::{builtin_numeric_function, compare, evaluate_to_num};
use crate::numeric::Num;
use crate::{Engine, Value};

fn with_big<T: Send + 'static>(probe: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(probe)
        .expect("probe thread")
        .join()
        .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
}

// ---------------------------------------------------------------------------
// ComplexMathInexactArgumentProvenanceTests — symbolic node and numeric value agree
// ---------------------------------------------------------------------------

/// The decimal literal for a Real, copying every stored digit (the reference's
/// `RealLiteral.FromRealExact`).
fn pi_literal(places: usize) -> Expr {
    Expr::real(&Real::pi(places).to_decimal_fixed(places))
}

/// C# ComplexMathInexactArgumentProvenanceTests.cs:161
/// `SymbolicNodeAndNumericResult_AgreeForTheSameExpression`: `cos(pi(30))`'s symbolic node is
/// inexact, and the numeric value EvaluateToNum reads back for that same node is inexact too —
/// the value may not claim an exactness the node denies. The value is the reference's
/// `ComplexMath.Cos(pi30)`.
#[test]
fn complexmathinexactargumentprovenancetests_symbolicnodeandnumericresult_agree_for_the_same_expression()
 {
    let places = 30;
    let pi = Real::pi(places);
    let literal = pi_literal(places);
    assert!(
        !literal.is_exact(),
        "the literal for pi(30) must be inexact"
    );

    let node = Expr::Fun("cos".to_string(), vec![literal]);
    assert!(
        !node.is_exact(),
        "the symbolic node cos(pi(30)) must be inexact"
    );

    let numeric = evaluate_to_num(&node, places, &[], &builtin_numeric_function)
        .expect("cos(pi(30)) evaluates numerically");
    let Num::Real(value) = numeric else {
        panic!("cos(pi(30)) must be a Real, got {numeric:?}");
    };
    assert_eq!(
        value,
        lovelace_real::transcendental::complex_math_cos(&pi, places)
    );
    assert_eq!(node.is_exact(), value.is_exact());
    assert!(
        !value.is_exact(),
        "the numeric value of cos(pi(30)) claims exactness the symbolic node denies"
    );
}

/// C# ComplexMathInexactArgumentProvenanceTests.cs:188
/// `SymbolicNodeAndNumericResult_AgreeForSine`: the same pairing for `sin(pi(30))`, whose
/// value is zero. The flag may not be read off the value's shape: an exact-looking 0 out of an
/// inexact argument is still an approximation's zero.
#[test]
fn complexmathinexactargumentprovenancetests_symbolicnodeandnumericresult_agree_for_sine() {
    let places = 30;
    let pi = Real::pi(places);
    let literal = pi_literal(places);
    let node = Expr::Fun("sin".to_string(), vec![literal]);
    assert!(
        !node.is_exact(),
        "the symbolic node sin(pi(30)) must be inexact"
    );

    let numeric = evaluate_to_num(&node, places, &[], &builtin_numeric_function)
        .expect("sin(pi(30)) evaluates numerically");
    let Num::Real(value) = numeric else {
        panic!("sin(pi(30)) must be a Real, got {numeric:?}");
    };
    assert_eq!(
        value,
        lovelace_real::transcendental::complex_math_sin(&pi, places)
    );
    assert_eq!(node.is_exact(), value.is_exact());
    assert!(
        !value.is_exact(),
        "the numeric value of sin(pi(30)) = {value} claims exactness"
    );
}

// ---------------------------------------------------------------------------
// LinsolveCheckTests — solve, then verify by numeric substitution
// ---------------------------------------------------------------------------

fn rat(value: i64) -> Rat {
    Rat::from_i64(value)
}

fn exact(value: i64) -> Num {
    Num::Exact(rat(value))
}

fn evaluated(value: &Expr, places: usize, bindings: &[(&str, Num)]) -> Num {
    evaluate_to_num(value, places, bindings, &builtin_numeric_function)
        .unwrap_or_else(|error| panic!("solution value {value:?} must evaluate: {error}"))
}

fn assert_zero(value: &Num, places: usize, label: &str) {
    assert_eq!(
        compare(value, &exact(0), places).expect("real residuals compare"),
        Ordering::Equal,
        "{label}: residual {value:?} must be zero"
    );
}

fn assert_one(value: &Num, places: usize, label: &str) {
    assert_eq!(
        compare(value, &exact(1), places).expect("real residuals compare"),
        Ordering::Equal,
        "{label}: residual {value:?} must be one"
    );
}

/// C# LinsolveCheckTests.cs:11 `Solve_VerifiesByNumericSubstitution`: the 2x2 symbolic system
/// with `A = [[x, 1], [0, y]]`, `b = [0, 1]`; substituting two parameter pairs into the
/// solution vector reproduces `x*v0 + v1 = 0` and `y*v1 = 1`.
#[test]
fn linsolvechecktests_solve_verifiesbynumericsubstitution() {
    let x = Expr::sym("x");
    let y = Expr::sym("y");
    let matrix = SymbolicMatrix::from_rows(vec![vec![x, Expr::num(1)], vec![Expr::num(0), y]])
        .expect("2x2 matrix");
    let b = vec![Expr::num(0), Expr::num(1)];
    let solution = matrix
        .solve(&b)
        .expect("the determinant x*y is nonzero generically");
    assert_eq!(solution.len(), 2);

    let places = 50;
    for (px, py) in [
        (rat(2), rat(3)),
        (rat(-1).div(&rat(2)).unwrap(), rat(5).div(&rat(2)).unwrap()),
    ] {
        let bindings = [("x", Num::Exact(px.clone())), ("y", Num::Exact(py.clone()))];
        let v0 = evaluated(&solution[0], places, &bindings);
        let v1 = evaluated(&solution[1], places, &bindings);
        let row0 = Num::Exact(px)
            .mul(&v0, places)
            .expect("x*v0")
            .add(&v1, places)
            .expect("x*v0 + v1");
        let row1 = Num::Exact(py).mul(&v1, places).expect("y*v1");
        assert_zero(&row0, places, "x*v0 + v1 = 0");
        assert_one(&row1, places, "y*v1 = 1");
    }
}

/// C# LinsolveCheckTests.cs:40 `Solve_3x3_VerifiesByNumericSubstitution`: the 3x3 symbolic
/// system substituted at `x = 2, y = 3` reproduces the three published rows
/// `2*v0 + v1 = 1`, `v0 + 2*v1 + v2 = 0` and `v1 + 3*v2 = 1`.
#[test]
fn linsolvechecktests_solve3x3_verifiesbynumericsubstitution() {
    let x = Expr::sym("x");
    let y = Expr::sym("y");
    let matrix = SymbolicMatrix::from_rows(vec![
        vec![x, Expr::num(1), Expr::num(0)],
        vec![Expr::num(1), Expr::sym("x"), Expr::num(1)],
        vec![Expr::num(0), Expr::num(1), y],
    ])
    .expect("3x3 matrix");
    let b = vec![Expr::num(1), Expr::num(0), Expr::num(1)];
    let solution = matrix
        .solve(&b)
        .expect("the determinant is nonzero at x=2, y=3");
    assert_eq!(solution.len(), 3);

    let places = 50;
    let bindings = [("x", exact(2)), ("y", exact(3))];
    let v0 = evaluated(&solution[0], places, &bindings);
    let v1 = evaluated(&solution[1], places, &bindings);
    let v2 = evaluated(&solution[2], places, &bindings);

    let row0 = exact(2)
        .mul(&v0, places)
        .expect("2*v0")
        .add(&v1, places)
        .expect("2*v0 + v1");
    let row1 = v0
        .add(&exact(2).mul(&v1, places).expect("2*v1"), places)
        .expect("v0 + 2*v1")
        .add(&v2, places)
        .expect("v0 + 2*v1 + v2");
    let row2 = v1
        .add(&exact(3).mul(&v2, places).expect("3*v2"), places)
        .expect("v1 + 3*v2");

    assert_one(&row0, places, "2*v0 + v1 = 1");
    assert_zero(&row1, places, "v0 + 2*v1 + v2 = 0");
    assert_one(&row2, places, "v1 + 3*v2 = 1");
}

// ---------------------------------------------------------------------------
// RecursionBudgetCompositionTests — shapes #14 and #15
// ---------------------------------------------------------------------------

const F: &str = "func f(n) { if (n == 0) { return 0 }; return f(n - 1) }; ";

fn engine() -> Engine {
    let mut engine = Engine::new();
    engine.eval(F).expect("define f");
    engine
}

/// C# RecursionBudgetCompositionTests.cs:99 `TheDeepestAcceptedCall_Answers_InEveryComposition`
/// row 14 (`the body of a for`): `for i in 1..5 { f(85) }` answers the deepest accepted call's
/// value, 0, because the loop's own value is the last completed body value.
#[test]
fn recursionbudgetcompositiontests_thedeepestacceptedcall_answers_in_a_for_body() {
    with_big(|| {
        let mut engine = engine();
        let value = engine
            .eval("for i in 1..5 { f(85) }")
            .expect("the deepest accepted call answers from a for body");
        assert_eq!(value.display(), "0");
        assert!(matches!(value, Value::Natural(_)));
    });
}

/// C# RecursionBudgetCompositionTests.cs:99 row 15 (`the body of a while`):
/// `i = 0; while (i < 5) { i = i + 1; f(85) }` answers 0 for the same reason.
#[test]
fn recursionbudgetcompositiontests_thedeepestacceptedcall_answers_in_a_while_body() {
    with_big(|| {
        let mut engine = engine();
        let value = engine
            .eval("i = 0; while (i < 5) { i = i + 1; f(85) }")
            .expect("the deepest accepted call answers from a while body");
        assert_eq!(value.display(), "0");
        assert!(matches!(value, Value::Natural(_)));
    });
}
