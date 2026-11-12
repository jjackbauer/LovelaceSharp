//! Additive symbolic-matrix surface: the determinant and the adjugate-over-determinant inverse
//! with the NonZero condition the reference's `SymbolicMatrix.InverseWithConditions` publishes.
//!
//! This is the smallest surface the three `MetamorphicSetClosureTests` inverse rows need. It is
//! deliberately additive: no existing symbolics behaviour changes, and the matrix is built from
//! the crate's own [`crate::rational_functions`] (for the denominator split) and
//! [`crate::groebner::Polynomial`] (for the exact polynomial division Bareiss performs).
//!
//! The reference's inverse is the adjugate over the determinant: entry (i,j) is
//! `cofactor(j,i)/det`, and the record carries exactly one condition, det != 0. At a zero of the
//! determinant the entries are undefined, which is why the condition is part of the result rather
//! than an aside. A structurally zero adjugate entry is published as the plain polynomial zero,
//! never as `0/det`.

use std::collections::BTreeSet;

use crate::assumptions::{Assumption, AssumptionSet, SymbolPredicate};
use crate::groebner::{MonomialOrder, Polynomial};
use crate::{Expr, expand};

/// A rectangular matrix of expressions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymbolicMatrix {
    rows: usize,
    columns: usize,
    data: Vec<Expr>,
}

/// The outcome of [`SymbolicMatrix::inverse_with_conditions`]: the inverse (None when the matrix
/// is singular or not square), the conditions the inverse requires, and the reference's note.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixInverseResult {
    /// The inverse, or None when the matrix is singular or not square.
    pub matrix: Option<SymbolicMatrix>,
    /// The conditions under which the inverse is the inverse; exactly det != 0 when it exists.
    pub conditions: AssumptionSet,
    /// The reference's note for a refused inverse (`matrix is singular`), None when solved.
    pub note: Option<String>,
}

impl MatrixInverseResult {
    /// True when an inverse was produced.
    pub fn is_solved(&self) -> bool {
        self.matrix.is_some()
    }
}

/// The outcome of [`SymbolicMatrix::solve_with_conditions`]: the particular solution (None when
/// the system is refused), the conditions under which it solves A*x = b, and the reference's note
/// for a refused solve.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixSolveResult {
    /// The solution vector, or None when the system is singular, underdetermined or not square.
    pub vector: Option<Vec<Expr>>,
    /// The conditions under which the vector solves the system; exactly det != 0 when it exists.
    pub conditions: AssumptionSet,
    /// The reference's note for a refused solve, None when solved.
    pub note: Option<String>,
}

impl MatrixSolveResult {
    /// True when a solution vector was produced.
    pub fn is_solved(&self) -> bool {
        self.vector.is_some()
    }
}

impl SymbolicMatrix {
    /// Builds a matrix from rows. None when the rows are not all the same width.
    pub fn from_rows(rows: Vec<Vec<Expr>>) -> Option<SymbolicMatrix> {
        let row_count = rows.len();
        let column_count = rows.first().map_or(0, Vec::len);
        if rows.iter().any(|row| row.len() != column_count) {
            return None;
        }
        let mut data = Vec::with_capacity(row_count * column_count);
        for row in rows {
            data.extend(row);
        }
        Some(SymbolicMatrix {
            rows: row_count,
            columns: column_count,
            data,
        })
    }

    /// The row count.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// The column count.
    pub fn columns(&self) -> usize {
        self.columns
    }

    /// The entry at (row, column), or None when the position is outside the matrix.
    pub fn get(&self, row: usize, column: usize) -> Option<&Expr> {
        if row >= self.rows || column >= self.columns {
            return None;
        }
        self.data.get(row * self.columns + column)
    }

    /// The columns of a transpose (the reference's `Transpose`).
    pub fn transpose(&self) -> SymbolicMatrix {
        let mut data = vec![Expr::num(0); self.rows * self.columns];
        for row in 0..self.rows {
            for column in 0..self.columns {
                data[column * self.rows + row] = self.data[row * self.columns + column].clone();
            }
        }
        SymbolicMatrix {
            rows: self.columns,
            columns: self.rows,
            data,
        }
    }

    /// The product of two compatible matrices, or None when the shapes do not line up.
    pub fn multiply(&self, other: &SymbolicMatrix) -> Option<SymbolicMatrix> {
        if self.columns != other.rows {
            return None;
        }
        let mut data = Vec::with_capacity(self.rows * other.columns);
        for row in 0..self.rows {
            for column in 0..other.columns {
                let mut total = Expr::num(0);
                for k in 0..self.columns {
                    total = Expr::add(
                        total,
                        Expr::mul(
                            self.data[row * self.columns + k].clone(),
                            other.data[k * other.columns + column].clone(),
                        ),
                    );
                }
                data.push(total);
            }
        }
        Some(SymbolicMatrix {
            rows: self.rows,
            columns: other.columns,
            data,
        })
    }

    /// The Bareiss fraction-free determinant, or None for a non-square matrix.
    ///
    /// The algorithm is the reference's: choose the first row at or below the current one whose
    /// entry is not a provably-zero constant, swap it into place, and update the trailing block
    /// with exact polynomial division by the previous pivot. When no pivot exists the determinant
    /// is zero. The condition expression the inverse publishes is this same expression.
    pub fn det(&self) -> Option<Expr> {
        if self.rows != self.columns {
            return None;
        }
        let n = self.rows;
        if n == 0 {
            return Some(Expr::num(1));
        }
        if n == 1 {
            return Some(self.data[0].clone());
        }
        let variables = self.symbols();
        let mut entries = self.data.clone();
        let mut previous = Expr::num(1);
        let mut sign = 1i64;
        for k in 0..n - 1 {
            let mut pivot = None;
            for row in k..n {
                if !is_provably_zero(&entries[row * n + k]) {
                    pivot = Some(row);
                    break;
                }
            }
            let Some(pivot) = pivot else {
                return Some(Expr::num(0));
            };
            if pivot != k {
                for column in k..n {
                    entries.swap(k * n + column, pivot * n + column);
                }
                sign = -sign;
            }
            for row in k + 1..n {
                for column in k + 1..n {
                    let numerator = Expr::sub(
                        Expr::mul(
                            entries[row * n + column].clone(),
                            entries[k * n + k].clone(),
                        ),
                        Expr::mul(
                            entries[row * n + k].clone(),
                            entries[k * n + column].clone(),
                        ),
                    );
                    entries[row * n + column] = exact_divide(&numerator, &previous, &variables);
                }
            }
            previous = entries[k * n + k].clone();
        }
        let determinant = entries[(n - 1) * n + (n - 1)].clone();
        Some(if sign < 0 {
            Expr::neg(determinant)
        } else {
            determinant
        })
    }

    /// The generic rank over the rational-function field, by fraction-free (Bareiss) elimination
    /// with full pivoting.
    ///
    /// Every pivot that is not a provably-zero constant contributes one to the rank, so a symbolic
    /// pivot counts (the generic rank over Q(x)) and a numeric zero does not. The pivot search is
    /// the reference's: the first row at or below the current one that holds a non-zero entry at
    /// or right of the current column, and the first such column in that row. A zero row or column
    /// is skipped by the search; when no pivot remains the rank is final.
    pub fn rank(&self) -> usize {
        let n = self.rows;
        let m = self.columns;
        if n == 0 || m == 0 {
            return 0;
        }
        let variables = self.symbols();
        let mut entries = self.data.clone();
        let mut previous = Expr::num(1);
        let mut rank = 0;
        let (mut row_index, mut column_index) = (0usize, 0usize);
        while row_index < n && column_index < m {
            let mut pivot: Option<(usize, usize)> = None;
            for i in row_index..n {
                for j in column_index..m {
                    if !is_provably_zero(&entries[i * m + j]) {
                        pivot = Some((i, j));
                        break;
                    }
                }
                if pivot.is_some() {
                    break;
                }
            }
            let Some((pivot_row, pivot_column)) = pivot else {
                break;
            };
            if pivot_row != row_index {
                for j in 0..m {
                    entries.swap(row_index * m + j, pivot_row * m + j);
                }
            }
            if pivot_column != column_index {
                for i in 0..n {
                    entries.swap(i * m + column_index, i * m + pivot_column);
                }
            }
            rank += 1;
            let pivot_entry = entries[row_index * m + column_index].clone();
            for i in row_index + 1..n {
                for j in column_index + 1..m {
                    let numerator = Expr::sub(
                        Expr::mul(entries[i * m + j].clone(), pivot_entry.clone()),
                        Expr::mul(
                            entries[i * m + column_index].clone(),
                            entries[row_index * m + j].clone(),
                        ),
                    );
                    entries[i * m + j] = exact_divide(&numerator, &previous, &variables);
                }
                entries[i * m + column_index] = Expr::num(0);
            }
            previous = pivot_entry;
            row_index += 1;
            column_index += 1;
        }
        rank
    }

    /// The inverse via the adjugate over the determinant, carrying the det != 0 condition.
    ///
    /// A singular or non-square matrix returns no matrix, an empty condition set, and the
    /// reference's note. The reference publishes the determinant as an
    /// [`Assumption::ExpressionProperty`] NonZero atom even when the determinant is a bare symbol.
    pub fn inverse_with_conditions(&self) -> MatrixInverseResult {
        let Some(determinant) = self.det() else {
            return MatrixInverseResult {
                matrix: None,
                conditions: AssumptionSet::empty(),
                note: Some("determinant requires a square matrix".to_string()),
            };
        };
        if is_provably_zero(&determinant) {
            return MatrixInverseResult {
                matrix: None,
                conditions: AssumptionSet::empty(),
                note: Some("matrix is singular".to_string()),
            };
        }
        let conditions = AssumptionSet::from_atoms([Assumption::ExpressionProperty(
            determinant.clone(),
            SymbolPredicate::NonZero,
        )]);
        let mut data = vec![Expr::num(0); self.rows * self.columns];
        for row in 0..self.rows {
            for column in 0..self.columns {
                let minor = self.minor(row, column);
                let cofactor = minor.det().expect("a minor of a square matrix is square");
                let cofactor = if (row + column) % 2 == 0 {
                    cofactor
                } else {
                    Expr::neg(cofactor)
                };
                data[column * self.columns + row] = if is_provably_zero(&cofactor) {
                    Expr::num(0)
                } else {
                    Expr::div(cofactor, determinant.clone())
                };
            }
        }
        MatrixInverseResult {
            matrix: Some(SymbolicMatrix {
                rows: self.rows,
                columns: self.columns,
                data,
            }),
            conditions,
            note: None,
        }
    }

    /// The inverse the reference's Inverse publishes: the adjugate over the determinant.
    ///
    /// None is the reference's InvalidOperationException. A singular or non-square matrix has no
    /// inverse; [`Self::inverse_with_conditions`] publishes the same matrix and the refusal note.
    pub fn inverse(&self) -> Option<SymbolicMatrix> {
        self.inverse_with_conditions().matrix
    }

    /// The particular solution of A*x = b the reference's static Solve publishes.
    ///
    /// None is the reference's InvalidOperationException. A singular, non-square or
    /// underdetermined system has no vector; [`Self::solve_with_conditions`] publishes the same
    /// vector and the refusal note. The reference has no free-parameter representation: a
    /// singular system is refused, not parametrized.
    pub fn solve(&self, b: &[Expr]) -> Option<Vec<Expr>> {
        self.solve_with_conditions(b).vector
    }

    /// Fraction-free solve A*x = b (augmented Bareiss elimination), carrying the det != 0
    /// condition the reference publishes.
    ///
    /// The reference computes the determinant first, so a non-square matrix is refused with the
    /// determinant note, and a provably-zero determinant is refused with the singular note and no
    /// condition. Otherwise the result carries exactly the NonZero atom for the determinant. The
    /// reference's throw for a failed pivot search is published as a note: since the determinant is
    /// not provably zero, that branch is not reachable through this surface.
    pub fn solve_with_conditions(&self, b: &[Expr]) -> MatrixSolveResult {
        let Some(determinant) = self.det() else {
            return MatrixSolveResult {
                vector: None,
                conditions: AssumptionSet::empty(),
                note: Some("determinant requires a square matrix".to_string()),
            };
        };
        if is_provably_zero(&determinant) {
            return MatrixSolveResult {
                vector: None,
                conditions: AssumptionSet::empty(),
                note: Some("matrix is singular".to_string()),
            };
        }
        if b.len() != self.rows {
            return MatrixSolveResult {
                vector: None,
                conditions: AssumptionSet::empty(),
                note: Some("right-hand side length does not match the matrix".to_string()),
            };
        }
        let conditions = AssumptionSet::from_atoms([Assumption::ExpressionProperty(
            determinant,
            SymbolPredicate::NonZero,
        )]);
        let Some(vector) = self.solve_core(b) else {
            return MatrixSolveResult {
                vector: None,
                conditions: AssumptionSet::empty(),
                note: Some("system is singular or underdetermined".to_string()),
            };
        };
        MatrixSolveResult {
            vector: Some(vector),
            conditions,
            note: None,
        }
    }

    /// The reference's SolveCore: forward elimination on the augmented matrix over rows BELOW the
    /// pivot, then back-substitution. Eliminating the rows above the pivot with the previous-pivot
    /// division is not exact, which is why only the trailing submatrix is updated.
    fn solve_core(&self, b: &[Expr]) -> Option<Vec<Expr>> {
        let n = self.rows;
        let stride = n + 1;
        let variables = self.symbols();
        let mut augmented = vec![Expr::num(0); n * stride];
        for row in 0..n {
            for column in 0..n {
                augmented[row * stride + column] = self.data[row * n + column].clone();
            }
            augmented[row * stride + n] = b[row].clone();
        }
        let mut previous = Expr::num(1);
        for k in 0..n {
            let pivot = (k..n).find(|&row| !is_provably_zero(&augmented[row * stride + k]))?;
            if pivot != k {
                for column in k..=n {
                    augmented.swap(k * stride + column, pivot * stride + column);
                }
            }
            for i in k + 1..n {
                for j in k + 1..=n {
                    let numerator = Expr::sub(
                        Expr::mul(
                            augmented[i * stride + j].clone(),
                            augmented[k * stride + k].clone(),
                        ),
                        Expr::mul(
                            augmented[i * stride + k].clone(),
                            augmented[k * stride + j].clone(),
                        ),
                    );
                    augmented[i * stride + j] = exact_divide(&numerator, &previous, &variables);
                }
                augmented[i * stride + k] = Expr::num(0);
            }
            previous = augmented[k * stride + k].clone();
        }
        let mut solution = vec![Expr::num(0); n];
        for i in (0..n).rev() {
            let mut accumulator = augmented[i * stride + n].clone();
            for j in i + 1..n {
                accumulator = Expr::sub(
                    accumulator,
                    Expr::mul(augmented[i * stride + j].clone(), solution[j].clone()),
                );
            }
            solution[i] = Expr::div(accumulator, augmented[i * stride + i].clone());
        }
        Some(solution)
    }

    fn minor(&self, skip_row: usize, skip_column: usize) -> SymbolicMatrix {
        let mut rows: Vec<Vec<Expr>> = Vec::with_capacity(self.rows.saturating_sub(1));
        for row in 0..self.rows {
            if row == skip_row {
                continue;
            }
            let mut entries: Vec<Expr> = Vec::with_capacity(self.columns.saturating_sub(1));
            for column in 0..self.columns {
                if column == skip_column {
                    continue;
                }
                entries.push(self.data[row * self.columns + column].clone());
            }
            rows.push(entries);
        }
        SymbolicMatrix::from_rows(rows).expect("a minor of a rectangular matrix is rectangular")
    }

    /// Every free symbol of the matrix, sorted by name (the reference's `VariablesOf`).
    pub fn symbols(&self) -> Vec<String> {
        let mut names: BTreeSet<String> = BTreeSet::new();
        for entry in &self.data {
            names.extend(entry.free_symbols());
        }
        names.into_iter().collect()
    }
}

/// True for a structurally zero numeric constant (the reference's `IsProvablyZero` structural
/// half; the crate has no numeric evaluator to consult, and a structural zero is the conservative
/// test the pivot selection needs).
fn is_provably_zero(e: &Expr) -> bool {
    matches!(e, Expr::Num(value) if value.is_zero())
}

/// Exact division when provable (polynomial division over the matrix's symbols); the fallback is
/// rational-function division, value-correct even when it cannot stay fraction-free.
fn exact_divide(numerator: &Expr, denominator: &Expr, variables: &[String]) -> Expr {
    if let (Expr::Num(top), Expr::Num(bottom)) = (numerator, denominator) {
        if !bottom.is_zero() {
            if let Some(quotient) = top.div(bottom) {
                return Expr::Num(quotient);
            }
        }
    }
    if let (Some(top), Some(bottom)) = (
        parse_polynomial(numerator, variables),
        parse_polynomial(denominator, variables),
    ) {
        if !bottom.is_zero() {
            let (quotient, remainder) = top.div_rem(&bottom, MonomialOrder::Lex);
            if remainder.is_zero() {
                return polynomial_to_expr(&quotient, variables);
            }
        }
    }
    Expr::div(numerator.clone(), denominator.clone())
}

fn parse_polynomial(expression: &Expr, variables: &[String]) -> Option<Polynomial> {
    Polynomial::from_expr(expression, variables).or_else(|| {
        let expanded = expand(expression);
        if expanded == *expression {
            None
        } else {
            Polynomial::from_expr(&expanded, variables)
        }
    })
}

fn polynomial_to_expr(polynomial: &Polynomial, variables: &[String]) -> Expr {
    let mut accumulator = Expr::num(0);
    for (monomial, coefficient) in polynomial.terms() {
        let mut term = Expr::Num(coefficient.clone());
        for (index, variable) in variables.iter().enumerate() {
            let exponent = monomial.exponent(index);
            if exponent == 0 {
                continue;
            }
            let power = if exponent == 1 {
                Expr::sym(variable)
            } else {
                Expr::pow(Expr::sym(variable), Expr::num(exponent as i64))
            };
            term = Expr::mul(term, power);
        }
        accumulator = Expr::add(accumulator, term);
    }
    accumulator
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rational_functions::try_rationalize;
    use lovelace_int::Int;
    use lovelace_nat::Nat;

    fn sym(name: &str) -> Expr {
        Expr::sym(name)
    }

    fn matrix(rows: Vec<Vec<Expr>>) -> SymbolicMatrix {
        SymbolicMatrix::from_rows(rows).expect("rectangular matrix")
    }

    fn rational(numerator: i64, denominator: i64) -> crate::Rat {
        crate::Rat::from_parts(Int::from_i64(numerator), Nat::from_u64(denominator as u64))
    }

    /// Substitutes rational bindings and evaluates exactly; None is the reference's
    /// `EvaluationException` (the expression is undefined at that point).
    fn evaluate(expression: &Expr, bindings: &[(&str, crate::Rat)]) -> Option<crate::Rat> {
        match expression {
            Expr::Num(value) => Some(value.clone()),
            Expr::Sym(name) => bindings
                .iter()
                .find(|(bound, _)| *bound == name)
                .map(|(_, value)| value.clone()),
            Expr::Add(terms) => {
                let mut total = crate::Rat::zero();
                for term in terms {
                    total = total.add(&evaluate(term, bindings)?);
                }
                Some(total)
            }
            Expr::Mul(factors) => {
                let mut total = crate::Rat::one();
                for factor in factors {
                    total = total.mul(&evaluate(factor, bindings)?);
                }
                Some(total)
            }
            Expr::Pow(base, exponent) => {
                let base = evaluate(base, bindings)?;
                let exponent = evaluate(exponent, bindings)?;
                base.pow_i64(exponent.to_i64()?)
            }
            Expr::Div(numerator, denominator) => {
                let numerator = evaluate(numerator, bindings)?;
                let denominator = evaluate(denominator, bindings)?;
                numerator.div(&denominator)
            }
            Expr::Rel(_, _, _) | Expr::Fun(_, _) | Expr::Real(_) | Expr::Complex(_, _) => None,
        }
    }

    /// Clears the denominators the candidate reports and checks the polynomial identity
    /// A·candidate = I. Every nonzero candidate numerator must carry the determinant as its
    /// denominator (the reference check); the identity is then exact polynomial equality, not a
    /// sample. The returned Err is the first counterexample, like the C# helper's detail string.
    fn prove_inverse_identity(
        a: &SymbolicMatrix,
        candidate: &SymbolicMatrix,
        determinant: &Expr,
    ) -> Result<(), String> {
        let n = a.rows();
        if candidate.rows() != n || candidate.columns() != n {
            return Err("candidate shape does not match the matrix".to_string());
        }
        let mut numerators: Vec<Vec<Expr>> = vec![vec![Expr::num(0); n]; n];
        let mut denominators: Vec<Vec<Expr>> = vec![vec![Expr::num(1); n]; n];
        for k in 0..n {
            for j in 0..n {
                let Some((num, den)) = try_rationalize(candidate.get(k, j).expect("in range"))
                else {
                    return Err(format!("candidate[{k},{j}] did not rationalize"));
                };
                if !is_provably_zero(&num) && den != *determinant {
                    return Err(format!(
                        "candidate[{k},{j}] denominator is not the determinant condition"
                    ));
                }
                numerators[k][j] = num;
                denominators[k][j] = den;
            }
        }
        let total = Expr::pow(determinant.clone(), Expr::num(n as i64));
        let share = Expr::pow(determinant.clone(), Expr::num(n as i64 - 1));
        for i in 0..n {
            for j in 0..n {
                let mut sum = Expr::num(0);
                for k in 0..n {
                    let num = numerators[k][j].clone();
                    let cleared = if denominators[k][j] == *determinant {
                        Expr::mul(num, share.clone())
                    } else if is_provably_zero(&num) {
                        Expr::num(0)
                    } else {
                        return Err(format!("candidate[{k},{j}] lost the determinant"));
                    };
                    sum = Expr::add(
                        sum,
                        Expr::mul(a.get(i, k).expect("in range").clone(), cleared),
                    );
                }
                let rhs = if i == j { total.clone() } else { Expr::num(0) };
                if expand(&Expr::sub(sum, rhs)) != Expr::num(0) {
                    return Err(format!(
                        "(A*A^-1)[{i},{j}] is not the identity: the cleared numerator difference is non-zero"
                    ));
                }
            }
        }
        Ok(())
    }

    fn with_entry(
        m: &SymbolicMatrix,
        row: usize,
        column: usize,
        mutate: impl Fn(Expr) -> Expr,
    ) -> SymbolicMatrix {
        let (numerator, denominator) =
            try_rationalize(m.get(row, column).expect("in range")).expect("entry rationalizes");
        let mut rows: Vec<Vec<Expr>> = (0..m.rows())
            .map(|i| {
                (0..m.columns())
                    .map(|j| m.get(i, j).expect("in range").clone())
                    .collect()
            })
            .collect();
        rows[row][column] = Expr::div(mutate(numerator), denominator);
        SymbolicMatrix::from_rows(rows).expect("rectangular matrix")
    }

    #[test]
    fn inverse_times_original_is_identity_symbolically_under_det_non_zero_condition() {
        let a = sym("a");
        let b = sym("b");
        let c = sym("c");
        let d = sym("d");
        let x = sym("x");
        let y = sym("y");
        let z = sym("z");
        let cases: Vec<(String, SymbolicMatrix, Expr)> = vec![
            (
                "2x2 general [[a,b],[c,d]]".to_string(),
                matrix(vec![vec![a.clone(), b.clone()], vec![c.clone(), d.clone()]]),
                Expr::sub(
                    Expr::mul(a.clone(), d.clone()),
                    Expr::mul(b.clone(), c.clone()),
                ),
            ),
            (
                "2x2 [[x,1],[y,x]]".to_string(),
                matrix(vec![
                    vec![x.clone(), Expr::num(1)],
                    vec![y.clone(), x.clone()],
                ]),
                Expr::sub(Expr::pow(x.clone(), Expr::num(2)), y.clone()),
            ),
            (
                "3x3 tridiagonal".to_string(),
                matrix(vec![
                    vec![x.clone(), Expr::num(1), Expr::num(0)],
                    vec![Expr::num(1), y.clone(), Expr::num(1)],
                    vec![Expr::num(0), Expr::num(1), z.clone()],
                ]),
                Expr::sub(
                    Expr::sub(
                        Expr::mul(Expr::mul(x.clone(), y.clone()), z.clone()),
                        x.clone(),
                    ),
                    z.clone(),
                ),
            ),
            (
                "3x3 cyclic [[x,y,1],[0,x,y],[y,0,x]]".to_string(),
                matrix(vec![
                    vec![x.clone(), y.clone(), Expr::num(1)],
                    vec![Expr::num(0), x.clone(), y.clone()],
                    vec![y.clone(), Expr::num(0), x.clone()],
                ]),
                Expr::sub(
                    Expr::add(
                        Expr::pow(x.clone(), Expr::num(3)),
                        Expr::pow(y.clone(), Expr::num(3)),
                    ),
                    Expr::mul(x.clone(), y.clone()),
                ),
            ),
            (
                "3x3 Vandermonde".to_string(),
                matrix(vec![
                    vec![Expr::num(1), x.clone(), Expr::pow(x.clone(), Expr::num(2))],
                    vec![Expr::num(1), y.clone(), Expr::pow(y.clone(), Expr::num(2))],
                    vec![Expr::num(1), z.clone(), Expr::pow(z.clone(), Expr::num(2))],
                ]),
                expand(&Expr::mul(
                    Expr::mul(
                        Expr::sub(y.clone(), x.clone()),
                        Expr::sub(z.clone(), x.clone()),
                    ),
                    Expr::sub(z.clone(), y.clone()),
                )),
            ),
        ];
        for (name, a, expected_determinant) in cases {
            let determinant = a.det().unwrap_or_else(|| panic!("{name}: no determinant"));
            let result = a.inverse_with_conditions();
            let inverse = result
                .matrix
                .clone()
                .unwrap_or_else(|| panic!("{name}: no inverse (note: {:?})", result.note));
            assert_eq!(result.note, None, "{name}");
            let atoms = result.conditions.atoms();
            assert_eq!(atoms.len(), 1, "{name}: expected one condition");
            match &atoms[0] {
                Assumption::ExpressionProperty(condition, SymbolPredicate::NonZero) => {
                    assert_eq!(
                        *condition, determinant,
                        "{name}: condition is not the determinant"
                    );
                    assert_eq!(
                        expand(condition),
                        expand(&expected_determinant),
                        "{name}: emitted determinant differs from the independent ground truth"
                    );
                }
                other => panic!("{name}: expected an expression NonZero atom, got {other:?}"),
            }
            assert!(
                !is_provably_zero(&determinant),
                "{name}: the condition is 0 != 0"
            );
            prove_inverse_identity(&a, &inverse, &determinant)
                .unwrap_or_else(|detail| panic!("{name}: {detail}"));
        }
    }

    #[test]
    fn inverse_identity_check_rejects_inverses_that_pass_the_numeric_spot_check() {
        let x = sym("x");
        let y = sym("y");
        let a = matrix(vec![
            vec![x.clone(), Expr::num(1)],
            vec![y.clone(), x.clone()],
        ]);
        let determinant = a.det().expect("square matrix");
        let inverse = a
            .inverse_with_conditions()
            .matrix
            .expect("symbolically nonsingular");
        prove_inverse_identity(&a, &inverse, &determinant)
            .expect("the kernel's own inverse passes");

        let transposed = matrix(vec![
            vec![
                inverse.get(0, 0).expect("in range").clone(),
                inverse.get(1, 0).expect("in range").clone(),
            ],
            vec![
                inverse.get(0, 1).expect("in range").clone(),
                inverse.get(1, 1).expect("in range").clone(),
            ],
        ]);
        let detail = prove_inverse_identity(&a, &transposed, &determinant)
            .expect_err("transposed adjugate accepted");
        assert!(!detail.is_empty());

        let sign_flipped = with_entry(&inverse, 0, 1, Expr::neg);
        let detail = prove_inverse_identity(&a, &sign_flipped, &determinant)
            .expect_err("sign-flipped cofactor accepted");
        assert!(!detail.is_empty());

        let perturbed = with_entry(&inverse, 0, 0, |numerator| {
            Expr::add(numerator, Expr::num(1))
        });
        let detail = prove_inverse_identity(&a, &perturbed, &determinant)
            .expect_err("numerator perturbation accepted");
        assert!(!detail.is_empty());

        // This corruption vanishes at (x,y) = (3,2): the old numeric spot check accepts it.
        let blind_spot = with_entry(&inverse, 0, 1, |numerator| {
            Expr::sub(
                numerator,
                Expr::pow(Expr::sub(x.clone(), Expr::num(3)), Expr::num(2)),
            )
        });
        let product = a.multiply(&blind_spot).expect("compatible shapes");
        let bindings = [("x", rational(3, 1)), ("y", rational(2, 1))];
        assert_eq!(
            evaluate(product.get(0, 0).expect("in range"), &bindings),
            Some(rational(1, 1))
        );
        assert_eq!(
            evaluate(product.get(0, 1).expect("in range"), &bindings),
            Some(crate::Rat::zero())
        );
        assert_eq!(
            evaluate(product.get(1, 0).expect("in range"), &bindings),
            Some(crate::Rat::zero())
        );
        assert_eq!(
            evaluate(product.get(1, 1).expect("in range"), &bindings),
            Some(rational(1, 1))
        );
        let detail = prove_inverse_identity(&a, &blind_spot, &determinant)
            .expect_err("an inverse that is only correct at (3,2) was accepted symbolically");
        assert!(!detail.is_empty());
    }

    #[test]
    fn inverse_identity_holds_only_where_det_is_non_zero() {
        let x = sym("x");
        let y = sym("y");
        let a = matrix(vec![
            vec![x.clone(), Expr::num(1)],
            vec![y.clone(), x.clone()],
        ]);
        let inverse = a
            .inverse_with_conditions()
            .matrix
            .expect("symbolically nonsingular");

        // det = x^2 - y = 0 at (2,4): the entries are undefined there.
        let on_condition = [("x", rational(2, 1)), ("y", rational(4, 1))];
        assert_eq!(
            evaluate(inverse.get(0, 0).expect("in range"), &on_condition),
            None
        );
        assert_eq!(
            evaluate(inverse.get(1, 1).expect("in range"), &on_condition),
            None
        );

        // det = 1 at (2,3): defined, and the product is the identity there.
        let off_condition = [("x", rational(2, 1)), ("y", rational(3, 1))];
        let product = a.multiply(&inverse).expect("compatible shapes");
        assert_eq!(
            evaluate(product.get(0, 0).expect("in range"), &off_condition),
            Some(rational(1, 1))
        );
        assert_eq!(
            evaluate(product.get(0, 1).expect("in range"), &off_condition),
            Some(crate::Rat::zero())
        );
        assert_eq!(
            evaluate(product.get(1, 0).expect("in range"), &off_condition),
            Some(crate::Rat::zero())
        );
        assert_eq!(
            evaluate(product.get(1, 1).expect("in range"), &off_condition),
            Some(rational(1, 1))
        );
    }

    #[test]
    fn a_singular_matrix_publishes_no_inverse_and_the_reference_note() {
        let x = sym("x");
        let singular = matrix(vec![vec![x.clone(), x.clone()], vec![x.clone(), x.clone()]]);
        let result = singular.inverse_with_conditions();
        assert_eq!(result.matrix, None);
        assert!(result.conditions.atoms().is_empty());
        assert_eq!(result.note.as_deref(), Some("matrix is singular"));
    }

    #[test]
    fn determinant_of_a_non_square_matrix_is_refused() {
        let nonsquare = matrix(vec![vec![Expr::num(1), Expr::num(2), Expr::num(3)]]);
        assert_eq!(nonsquare.det(), None);
    }
}
