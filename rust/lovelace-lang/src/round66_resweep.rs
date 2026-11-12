//! Round 66: the deferred census rows this tree now answers across the language and symbolics
//! surfaces.
//!
//! The C# assertion (cited per test) is the contract. Each test states the translation it uses.

use crate::numeric::{evaluate, Num};
use lovelace_symbolics::symbolic_matrix::SymbolicMatrix;
use lovelace_symbolics::{substitute, Expr, Rat};

/// SmokeTests.cs:158 Inverse_SymbolicMatrix_TimesOriginalIsIdentity - A times A-inverse is the
/// identity, verified numerically at x = 3, y = 2. The Rust surface publishes the inverse with its
/// NonZero condition; every entry of the product is reduced to a symbol-free expression at the
/// C# row's point and evaluated exactly, so the assertion is the same numeric substitution the
/// reference performs.
#[test]
fn acceptance_inverse_times_original_is_identity() {
    let x = Expr::sym("x");
    let y = Expr::sym("y");
    let a = SymbolicMatrix::from_rows(vec![
        vec![x.clone(), Expr::num(1)],
        vec![y.clone(), x.clone()],
    ])
    .expect("2x2 matrix");
    let inverse = a
        .inverse_with_conditions()
        .matrix
        .expect("the matrix is invertible");
    let product = a.multiply(&inverse).expect("square product");

    let three = Expr::num(3);
    let two = Expr::num(2);
    for (row, column, expected) in [(0usize, 0usize, 1i64), (0, 1, 0), (1, 0, 0), (1, 1, 1)] {
        let entry = product.get(row, column).expect("entry");
        let at_point = substitute(&substitute(entry, "x", &three), "y", &two);
        let value = evaluate(&at_point, 40)
            .unwrap_or_else(|error| panic!("entry ({row},{column}) did not evaluate: {}", error.message));
        match value {
            Num::Exact(rat) => assert_eq!(
                rat,
                Rat::from_i64(expected),
                "A A-inverse at (x, y) = (3, 2): entry ({row},{column}) is {rat}, expected {expected}"
            ),
            other => panic!("entry ({row},{column}) is not exact at the point: {other:?}"),
        }
    }
}
