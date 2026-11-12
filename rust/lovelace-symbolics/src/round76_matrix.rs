//! Round 76: the four deferred MatrixAdvanceTests census rows, ported to the crate's
//! SymbolicMatrix surface (rank, inverse, solve, solve_with_conditions).
//!
//! Each test cites its C# case and asserts the same facts on the same concrete matrices. The
//! observed behaviour, the member map and the four census ids are in
//! docs/goal-parity-contract/round-76/matrix-rank-solve.md.

use crate::assumptions::{Assumption, SymbolPredicate};
use crate::symbolic_matrix::SymbolicMatrix;
use crate::{Expr, substitute};

fn sym(name: &str) -> Expr {
    Expr::sym(name)
}

fn matrix(rows: Vec<Vec<Expr>>) -> SymbolicMatrix {
    SymbolicMatrix::from_rows(rows).expect("rectangular matrix")
}

fn third() -> Expr {
    Expr::div(Expr::num(1), Expr::num(3))
}

/// C# MatrixAdvanceTests.cs:16 Rank_GenericOverSymbols_And_Numeric -- census id
/// Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.MatrixAdvanceTests/Rank_GenericOverSymbols_And_Numeric#0.
///
/// [[x,1],[y,x]] has generic rank 2, the doubled symbolic row has rank 1, [[1,2],[2,4]] has rank
/// 1, and the 3x3 identity has rank 3.
#[test]
fn rank_generic_over_symbols_and_numeric() {
    let x = sym("x");
    let y = sym("y");

    let symbolic = matrix(vec![
        vec![x.clone(), Expr::num(1)],
        vec![y.clone(), x.clone()],
    ]);
    assert_eq!(
        symbolic.rank(),
        2,
        "[[x,1],[y,x]] is generically non-singular"
    );

    let dependent = matrix(vec![vec![x.clone(), x.clone()], vec![x.clone(), x.clone()]]);
    assert_eq!(dependent.rank(), 1, "the two symbolic rows coincide");

    let numeric = matrix(vec![
        vec![Expr::num(1), Expr::num(2)],
        vec![Expr::num(2), Expr::num(4)],
    ]);
    assert_eq!(
        numeric.rank(),
        1,
        "the second numeric row is twice the first"
    );

    let identity = matrix(vec![
        vec![Expr::num(1), Expr::num(0), Expr::num(0)],
        vec![Expr::num(0), Expr::num(1), Expr::num(0)],
        vec![Expr::num(0), Expr::num(0), Expr::num(1)],
    ]);
    assert_eq!(identity.rank(), 3, "the identity has full rank");
}

/// C# MatrixAdvanceTests.cs:40 InverseWithConditions_NonSingular_MatchesInverse_And_CarriesCondition
/// -- census id Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.MatrixAdvanceTests/InverseWithConditions_NonSingular_MatchesInverse_And_CarriesCondition#0.
///
/// For [[x,1],[0,y]] the structured inverse exists, carries exactly the determinant NonZero
/// condition, has no note, and is the same matrix the convenience Inverse publishes.
#[test]
fn inverse_with_conditions_non_singular_matches_inverse_and_carries_condition() {
    let x = sym("x");
    let y = sym("y");
    let a = matrix(vec![
        vec![x.clone(), Expr::num(1)],
        vec![Expr::num(0), y.clone()],
    ]);

    let result = a.inverse_with_conditions();
    let inverse = result.matrix.clone().expect("the matrix is invertible");
    assert_eq!(
        result.note, None,
        "an invertible matrix has no refusal note"
    );

    let determinant = a.det().expect("square matrix");
    assert_eq!(
        determinant,
        Expr::mul(x.clone(), y.clone()),
        "the determinant is x*y"
    );
    let atoms = result.conditions.atoms();
    assert_eq!(atoms.len(), 1, "an invertible matrix carries one condition");
    assert_eq!(
        atoms[0],
        Assumption::ExpressionProperty(determinant, SymbolPredicate::NonZero),
        "the carried condition is exactly det != 0"
    );

    assert_eq!(
        a.inverse(),
        Some(inverse.clone()),
        "Inverse and InverseWithConditions agree"
    );

    // A * A^-1 is the identity under the condition. At (x, y) = (2, 3) every entry reduces
    // exactly to 0 or 1, which is the reference's substitution check.
    let product = a.multiply(&inverse).expect("square product");
    let two = Expr::num(2);
    let three = Expr::num(3);
    for (row, column, expected) in [(0usize, 0usize, 1i64), (0, 1, 0), (1, 0, 0), (1, 1, 1)] {
        let entry = product.get(row, column).expect("entry in range");
        let at_point = substitute(&substitute(entry, "x", &two), "y", &three);
        assert_eq!(
            at_point,
            Expr::num(expected),
            "A A^-1 at (x, y) = (2, 3), entry ({row}, {column})"
        );
    }
}

/// C# MatrixAdvanceTests.cs:67 SolveWithConditions_MatchesSolve_And_CarriesCondition -- census id
/// Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.MatrixAdvanceTests/SolveWithConditions_MatchesSolve_And_CarriesCondition#0.
///
/// The condition-carrying solve returns the same vector as the convenience Solve, carries exactly
/// the determinant NonZero condition and has no note. The particular solution at (x, y) = (2, 3)
/// is (1/3, 1/3) and satisfies both rows of the system.
#[test]
fn solve_with_conditions_matches_solve_and_carries_condition() {
    let x = sym("x");
    let y = sym("y");
    let a = matrix(vec![
        vec![x.clone(), Expr::num(1)],
        vec![Expr::num(0), y.clone()],
    ]);
    let b = vec![Expr::num(1), Expr::num(1)];

    let result = a.solve_with_conditions(&b);
    let vector = result.vector.clone().expect("nonsingular system");
    assert_eq!(result.note, None, "a solved system has no refusal note");
    assert_eq!(vector.len(), 2, "one entry per unknown");

    let determinant = a.det().expect("square matrix");
    let atoms = result.conditions.atoms();
    assert_eq!(atoms.len(), 1, "a solved system carries one condition");
    assert_eq!(
        atoms[0],
        Assumption::ExpressionProperty(determinant, SymbolPredicate::NonZero),
        "the carried condition is exactly det != 0"
    );

    let expected = a.solve(&b).expect("Solve publishes the same vector");
    assert_eq!(expected.len(), vector.len());
    for (index, (left, right)) in expected.iter().zip(vector.iter()).enumerate() {
        assert_eq!(
            left, right,
            "Solve and SolveWithConditions agree at index {index}"
        );
    }

    let two = Expr::num(2);
    let three = Expr::num(3);
    let at_point: Vec<Expr> = vector
        .iter()
        .map(|entry| substitute(&substitute(entry, "x", &two), "y", &three))
        .collect();
    assert_eq!(
        at_point,
        vec![third(), third()],
        "at (x, y) = (2, 3) the solution is (1/3, 1/3)"
    );
    let first_row = Expr::sub(
        Expr::add(Expr::mul(two, at_point[0].clone()), at_point[1].clone()),
        Expr::num(1),
    );
    let second_row = Expr::sub(Expr::mul(three, at_point[1].clone()), Expr::num(1));
    assert_eq!(first_row, Expr::num(0), "x*v0 + v1 = 1 at the point");
    assert_eq!(second_row, Expr::num(0), "y*v1 = 1 at the point");
}

/// C# MatrixAdvanceTests.cs:86 Inverse_Invertible_BehaviorPreserved -- census id
/// Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.MatrixAdvanceTests/Inverse_Invertible_BehaviorPreserved#0.
///
/// The convenience Inverse returns a matrix for [[x,1],[0,y]] and that matrix has the same
/// entries as InverseWithConditions, entry by entry. At (x, y) = (2, 3) those entries are the
/// adjugate-over-determinant values (1/2, -1/6, 0, 1/3).
#[test]
fn inverse_invertible_behavior_preserved() {
    let x = sym("x");
    let y = sym("y");
    let a = matrix(vec![
        vec![x.clone(), Expr::num(1)],
        vec![Expr::num(0), y.clone()],
    ]);

    let inverse = a.inverse().expect("the matrix is invertible");
    let with_conditions = a
        .inverse_with_conditions()
        .matrix
        .expect("the matrix is invertible");
    assert_eq!(inverse.rows(), with_conditions.rows());
    assert_eq!(inverse.columns(), with_conditions.columns());
    for row in 0..inverse.rows() {
        for column in 0..inverse.columns() {
            assert_eq!(
                inverse.get(row, column),
                with_conditions.get(row, column),
                "Inverse and InverseWithConditions differ at ({row}, {column})"
            );
        }
    }

    let two = Expr::num(2);
    let three = Expr::num(3);
    let expected = [
        Expr::div(Expr::num(1), Expr::num(2)),
        Expr::div(Expr::num(-1), Expr::num(6)),
        Expr::num(0),
        Expr::div(Expr::num(1), Expr::num(3)),
    ];
    for (index, expected) in expected.iter().enumerate() {
        let row = index / 2;
        let column = index % 2;
        let entry = inverse.get(row, column).expect("entry in range");
        let at_point = substitute(&substitute(entry, "x", &two), "y", &three);
        assert_eq!(&at_point, expected, "entry ({row}, {column}) at (2, 3)");
    }
}

/// The block's refusal cases: rank of zero, empty and degenerate shapes; singular and non-square
/// solves; a singular inverse; and the Rust surface's typed right-hand-side length refusal. The
/// reference indexes the right-hand side out of range there (an undefined failure) instead of
/// publishing a note; the crate refuses it structurally.
#[test]
fn rank_and_solve_refusal_cases() {
    let zero = matrix(vec![
        vec![Expr::num(0), Expr::num(0)],
        vec![Expr::num(0), Expr::num(0)],
    ]);
    assert_eq!(zero.rank(), 0, "a zero matrix has rank 0");
    assert_eq!(
        SymbolicMatrix::from_rows(vec![])
            .expect("empty matrix")
            .rank(),
        0,
        "a 0x0 matrix has rank 0"
    );
    assert_eq!(
        SymbolicMatrix::from_rows(vec![vec![], vec![]])
            .expect("zero-column matrix")
            .rank(),
        0,
        "a matrix with no columns has rank 0"
    );

    let rectangular = matrix(vec![
        vec![Expr::num(1), Expr::num(2), Expr::num(3)],
        vec![Expr::num(2), Expr::num(4), Expr::num(6)],
    ]);
    assert_eq!(rectangular.rank(), 1, "the second row is twice the first");
    let staircase = matrix(vec![
        vec![Expr::num(1), Expr::num(0)],
        vec![Expr::num(0), Expr::num(1)],
        vec![Expr::num(1), Expr::num(1)],
    ]);
    assert_eq!(
        staircase.rank(),
        2,
        "three rows over two columns cap the rank"
    );

    let singular = matrix(vec![
        vec![Expr::num(1), Expr::num(2)],
        vec![Expr::num(2), Expr::num(4)],
    ]);
    let refused = singular.solve_with_conditions(&[Expr::num(1), Expr::num(1)]);
    assert_eq!(refused.vector, None);
    assert_eq!(refused.note.as_deref(), Some("matrix is singular"));
    assert!(
        refused.conditions.atoms().is_empty(),
        "a refused solve carries no condition"
    );
    assert_eq!(singular.solve(&[Expr::num(1), Expr::num(1)]), None);
    assert_eq!(singular.inverse(), None);

    let non_square = matrix(vec![vec![Expr::num(1), Expr::num(2), Expr::num(3)]]);
    assert_eq!(non_square.inverse(), None);
    assert_eq!(
        non_square.inverse_with_conditions().note.as_deref(),
        Some("determinant requires a square matrix")
    );
    let refused = non_square.solve_with_conditions(&[Expr::num(1)]);
    assert_eq!(refused.vector, None);
    assert_eq!(
        refused.note.as_deref(),
        Some("determinant requires a square matrix")
    );
    assert!(refused.conditions.atoms().is_empty());

    let x = sym("x");
    let y = sym("y");
    let invertible = matrix(vec![vec![x, Expr::num(1)], vec![Expr::num(0), y]]);
    let refused = invertible.solve_with_conditions(&[Expr::num(1)]);
    assert_eq!(refused.vector, None);
    assert_eq!(
        refused.note.as_deref(),
        Some("right-hand side length does not match the matrix")
    );
    assert!(refused.conditions.atoms().is_empty());
}
