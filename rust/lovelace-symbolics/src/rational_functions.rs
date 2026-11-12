//! Rational-function normalization: the additive port of the reference's
//! `Lovelace.Symbolics.RationalFunctions` cancel surface.
//!
//! The reference exposes four related observations:
//!
//! * [`try_rationalize`] returns the (numerator, denominator) pair of an expression viewed as a
//!   rational function, or None when a part is not a polynomial over the expression's symbols.
//!   A single quotient term is split exactly; an additive expression is NOT put over a common
//!   denominator (the reference returns the input over one for that shape).
//! * [`cancel`] is the bare value form: the reduced expression alone, with no condition. It is a
//!   definedness EXTENSION (the removed pole is gone), so a caller that must not lose where the
//!   input is defined reads [`cancel_with_conditions`] instead.
//! * [`cancel_with_conditions`] reports the reduced expression, whether it changed, and one
//!   NonZero condition per removed factor. Cancelling `g` out of `n/d` agrees with `n/d`
//!   exactly where `g != 0`; at a zero of `g` the input is undefined while the reduced form
//!   may be defined. A factor that is a symbol carries a symbol-property atom, anything else an
//!   expression-property atom, which is the atom the rewrite path uses.
//! * [`non_zero_condition`] builds that same atom for a caller that needs it in isolation.
//!
//! The reduction walks the canonical sum one additive term at a time, exactly as the reference
//! does: a term that is not a quotient of polynomials (or whose common factor is one/zero) is kept
//! unchanged. A power of a sum is not a polynomial to the parser, so a numerator or denominator the
//! parser refuses is expanded once and parsed again; expansion is value-preserving and adds no
//! condition. The polynomial engine itself is the crate's existing [`crate::groebner::Polynomial`]
//! for parsing, with the reference's multivariate-distance sparse division, Euclidean gcd and
//! canonical rendering kept here because the Gröbner module exposes neither division nor gcd.
//!
//! The public surface is additive: the existing crate-root `cancel` function and every other
//! symbolics behaviour are untouched.

use crate::assumptions::{Assumption, AssumptionSet, SymbolPredicate};
use crate::groebner::Polynomial;
use crate::{Expr, Rat, expand};

/// How a cancellation changed its input, as a type rather than as spelling.
///
/// `Exact`: nothing was removed, so the result equals the input pointwise. `Conditional`: a
/// common factor was removed, so the result equals the input only where that factor is nonzero
/// (see [`CancelResult::conditions`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CancelStatus {
    /// No factor was removed; the result is pointwise equal to the input.
    Exact,
    /// A common factor was removed; the result holds under the reported NonZero conditions.
    Conditional,
}

impl CancelStatus {
    /// The kernel spelling the reference record publishes (`Exact` / `Conditional`).
    pub fn name(self) -> &'static str {
        match self {
            CancelStatus::Exact => "Exact",
            CancelStatus::Conditional => "Conditional",
        }
    }
}

/// The outcome of [`cancel_with_conditions`]: the input, the reduced expression, and the
/// conditions the reduction requires. The bare [`cancel`] drops the last two.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CancelResult {
    /// The expression the reduction started from.
    pub original: Expr,
    /// The reduced expression.
    pub expression: Expr,
    /// The NonZero conditions under which `expression` equals `original`.
    pub conditions: AssumptionSet,
}

impl CancelResult {
    /// Builds a result from its three published fields.
    pub fn new(original: Expr, expression: Expr, conditions: AssumptionSet) -> CancelResult {
        CancelResult {
            original,
            expression,
            conditions,
        }
    }

    /// True when the reduction changed the expression.
    pub fn changed(&self) -> bool {
        self.expression != self.original
    }

    /// `Exact` when no factor was removed (no conditions), `Conditional` otherwise.
    pub fn status(&self) -> CancelStatus {
        if self.conditions.atoms().is_empty() {
            CancelStatus::Exact
        } else {
            CancelStatus::Conditional
        }
    }
}

/// The NonZero condition on an expression, in the exact shape the rewrite path uses: a symbol
/// carries a [`Assumption::SymbolProperty`], everything else an
/// [`Assumption::ExpressionProperty`].
pub fn non_zero_condition(e: &Expr) -> AssumptionSet {
    AssumptionSet::from_atoms([non_zero_atom(e)])
}

fn non_zero_atom(e: &Expr) -> Assumption {
    match e {
        Expr::Sym(s) => Assumption::SymbolProperty(s.clone(), SymbolPredicate::NonZero),
        other => Assumption::ExpressionProperty(other.clone(), SymbolPredicate::NonZero),
    }
}

/// Splits an expression into (numerator, denominator) as polynomials over its symbols.
///
/// This is the reference's `TryRationalize`: the canonical sum is walked term by term; a
/// negative-power factor is a denominator factor, every other factor is a numerator factor. A
/// single term is split exactly. A sum is returned as the input over one, because the reference
/// deliberately does not put several terms over a common denominator. None is the refusal: some
/// numerator or denominator factor is not a polynomial in the expression's symbols.
pub fn try_rationalize(e: &Expr) -> Option<(Expr, Expr)> {
    let variables = e.free_symbols();
    if variables.is_empty() {
        return Some((e.clone(), Expr::num(1)));
    }
    let terms: Vec<Expr> = match e {
        Expr::Add(terms) => terms.clone(),
        single => vec![single.clone()],
    };
    let mut term_polys: Vec<Expr> = Vec::with_capacity(terms.len());
    for term in &terms {
        let mut numerator_factors: Vec<Expr> = Vec::new();
        let mut denominator_factors: Vec<Expr> = Vec::new();
        for factor in factor_list(term) {
            if let Some((base, exponent)) = negative_power(&factor) {
                // The reference consumes a negative INTEGER exponent by repeating the base; a
                // negative non-integer exponent is left in the numerator and makes the parse
                // refuse (the reference's cast truncates it to zero and silently drops the
                // factor, which is not a behaviour this port repeats).
                if let Some(count) = exponent.to_i64() {
                    if count > 0 {
                        for _ in 0..count {
                            denominator_factors.push(base.clone());
                        }
                        continue;
                    }
                }
            }
            numerator_factors.push(factor);
        }
        let numerator = product_or_one(numerator_factors);
        let denominator = product_or_one(denominator_factors);
        if parse_polynomial(&numerator, &variables).is_none()
            || parse_polynomial(&denominator, &variables).is_none()
        {
            return None;
        }
        term_polys.push(Expr::div(numerator, denominator));
    }
    if term_polys.len() == 1 {
        return Some(split_single(&term_polys[0]));
    }
    Some((e.clone(), Expr::num(1)))
}

/// The bare value form of the reduction: the reduced expression alone.
///
/// An [`Expr`] has no place to carry a side condition, and the reduction is not an everywhere
/// identity: it removes the poles of the cancelled factor. Use [`cancel_with_conditions`] when
/// the definedness delta matters.
pub fn cancel(e: &Expr) -> Expr {
    cancel_with_conditions(e).expression
}

/// The structured form of [`cancel`]: the reduced expression plus the conditions under which it
/// equals the input.
///
/// The canonical sum is reduced one additive term at a time. A term whose numerator or
/// denominator is not a polynomial, or whose common factor is one/zero, is kept unchanged. Every
/// removed factor contributes exactly one NonZero condition, built by [`non_zero_condition`].
pub fn cancel_with_conditions(e: &Expr) -> CancelResult {
    let variables = e.free_symbols();
    if variables.is_empty() {
        return CancelResult::new(e.clone(), e.clone(), AssumptionSet::empty());
    }
    let terms: Vec<Expr> = match e {
        Expr::Add(terms) => terms.clone(),
        single => vec![single.clone()],
    };
    let mut results: Vec<Expr> = Vec::with_capacity(terms.len());
    let mut conditions = AssumptionSet::empty();
    for term in terms {
        let unchanged = term.clone();
        let (numerator, denominator) = split_single(&term);
        let (Some(numerator_polynomial), Some(denominator_polynomial)) = (
            parse_polynomial(&numerator, &variables),
            parse_polynomial(&denominator, &variables),
        ) else {
            results.push(unchanged);
            continue;
        };
        let common = sparse_gcd(&numerator_polynomial, &denominator_polynomial);
        if common.is_empty() || sparse_is_one(&common) {
            results.push(unchanged);
            continue;
        }
        let (Some(reduced_numerator), Some(reduced_denominator)) = (
            sparse_exact_quotient(&numerator_polynomial, &common),
            sparse_exact_quotient(&denominator_polynomial, &common),
        ) else {
            // An exact common divisor always divides, but an internal refusal must keep the
            // original term rather than publish a silently wrong reduction.
            results.push(unchanged);
            continue;
        };
        let numerator_expression = sparse_to_expr(&reduced_numerator, &variables);
        let reduced = if sparse_is_one(&reduced_denominator) {
            numerator_expression
        } else {
            Expr::div(
                numerator_expression,
                sparse_to_expr(&reduced_denominator, &variables),
            )
        };
        results.push(reduced);
        let factor = sparse_to_expr(&common, &variables);
        conditions = conditions
            .add_owned(non_zero_atom(&factor))
            .unwrap_or_else(|(_, set)| set);
    }
    let expression = match results.len() {
        1 => results.pop().expect("one result"),
        _ => results.into_iter().fold(Expr::num(0), Expr::add),
    };
    CancelResult::new(e.clone(), expression, conditions)
}

/// The reference's `SplitSingle`: a single term into numerator and positive-power denominator.
/// The explicit [`Expr::Div`] node is a Rust representation the reference does not have; it is
/// split the same way a product of a numerator and a negative power is.
fn split_single(e: &Expr) -> (Expr, Expr) {
    if let Expr::Div(numerator, denominator) = e {
        return ((**numerator).clone(), (**denominator).clone());
    }
    if let Some((base, exponent)) = negative_power(e) {
        return (Expr::num(1), Expr::pow(base, Expr::Num(exponent)));
    }
    if let Expr::Mul(factors) = e {
        let mut numerator: Vec<Expr> = Vec::new();
        let mut denominator: Vec<Expr> = Vec::new();
        for factor in factors {
            match negative_power(factor) {
                Some((base, exponent)) => {
                    denominator.push(Expr::pow(base, Expr::Num(exponent)));
                }
                None => numerator.push(factor.clone()),
            }
        }
        return (product_or_one(numerator), product_or_one(denominator));
    }
    (e.clone(), Expr::num(1))
}

/// The base and the NEGATED exponent when the factor is a power with a negative numeric exponent.
fn negative_power(factor: &Expr) -> Option<(Expr, Rat)> {
    let Expr::Pow(base, exponent) = factor else {
        return None;
    };
    let Expr::Num(value) = exponent.as_ref() else {
        return None;
    };
    if value.is_negative() {
        Some(((**base).clone(), value.neg()))
    } else {
        None
    }
}

fn factor_list(term: &Expr) -> Vec<Expr> {
    match term {
        Expr::Mul(factors) => factors.clone(),
        other => vec![other.clone()],
    }
}

fn product_or_one(factors: Vec<Expr>) -> Expr {
    factors.into_iter().fold(Expr::num(1), Expr::mul)
}

/// Parses one numerator or denominator as a polynomial over the expression's symbols. The
/// reference expands once when the direct parse refuses (a power of a sum is value-preserving to
/// expand), which is what makes `(x-1)^2/(x-1)` reducible without changing the parser contract.
fn parse_polynomial(expression: &Expr, variables: &[String]) -> Option<SparsePolynomial> {
    let parsed = Polynomial::from_expr(expression, variables).or_else(|| {
        let expanded = expand(expression);
        if expanded == *expression {
            None
        } else {
            Polynomial::from_expr(&expanded, variables)
        }
    })?;
    Some(
        parsed
            .terms()
            .iter()
            .map(|(monomial, coefficient)| {
                (
                    (0..variables.len())
                        .map(|index| monomial.exponent(index))
                        .collect(),
                    coefficient.clone(),
                )
            })
            .collect(),
    )
}

/// One term of a sparse multivariate polynomial: exponents by variable position, coefficient.
type SparseTerm = (Vec<u32>, Rat);
/// A sparse multivariate polynomial over Q, as the reference's Polynomial stores it.
type SparsePolynomial = Vec<SparseTerm>;

/// Adds one term, combining equal monomials and dropping a zero coefficient.
fn sparse_add_term(polynomial: &mut SparsePolynomial, exponents: Vec<u32>, coefficient: Rat) {
    if coefficient.is_zero() {
        return;
    }
    if let Some(index) = polynomial
        .iter()
        .position(|(existing, _)| *existing == exponents)
    {
        let sum = polynomial[index].1.add(&coefficient);
        if sum.is_zero() {
            polynomial.remove(index);
        } else {
            polynomial[index].1 = sum;
        }
        return;
    }
    polynomial.push((exponents, coefficient));
}

/// Lexicographic monomial comparison (the reference's MonomialOrder.Lex).
fn sparse_compare_exponents(left: &[u32], right: &[u32]) -> std::cmp::Ordering {
    let width = left.len().max(right.len());
    for index in 0..width {
        let comparison = left
            .get(index)
            .copied()
            .unwrap_or(0)
            .cmp(&right.get(index).copied().unwrap_or(0));
        if comparison != std::cmp::Ordering::Equal {
            return comparison;
        }
    }
    std::cmp::Ordering::Equal
}

fn sparse_leading_term(polynomial: &SparsePolynomial) -> Option<&SparseTerm> {
    polynomial
        .iter()
        .max_by(|left, right| sparse_compare_exponents(&left.0, &right.0))
}

/// True when every exponent of the divisor monomial is at most the dividend's.
fn sparse_monomial_divides(dividend: &[u32], divisor: &[u32]) -> bool {
    let width = dividend.len().max(divisor.len());
    (0..width).all(|index| {
        dividend.get(index).copied().unwrap_or(0) >= divisor.get(index).copied().unwrap_or(0)
    })
}

/// The monomial quotient (the caller has already checked divisibility).
fn sparse_monomial_quotient(dividend: &[u32], divisor: &[u32]) -> Vec<u32> {
    let width = dividend.len().max(divisor.len());
    (0..width)
        .map(|index| {
            dividend.get(index).copied().unwrap_or(0) - divisor.get(index).copied().unwrap_or(0)
        })
        .collect()
}

/// Multiplies every term of a polynomial by one monomial coefficient.
fn sparse_multiply_term(
    polynomial: &SparsePolynomial,
    exponents: &[u32],
    coefficient: &Rat,
) -> SparsePolynomial {
    let mut result = SparsePolynomial::new();
    for (term_exponents, term_coefficient) in polynomial {
        let width = term_exponents.len().max(exponents.len());
        let mut product = vec![0u32; width];
        for (index, slot) in product.iter_mut().enumerate() {
            *slot = term_exponents.get(index).copied().unwrap_or(0)
                + exponents.get(index).copied().unwrap_or(0);
        }
        sparse_add_term(&mut result, product, term_coefficient.mul(coefficient));
    }
    result
}

fn sparse_subtract(left: &SparsePolynomial, right: &SparsePolynomial) -> SparsePolynomial {
    let mut result = left.clone();
    for (exponents, coefficient) in right {
        sparse_add_term(&mut result, exponents.clone(), coefficient.neg());
    }
    result
}

fn sparse_is_one(polynomial: &SparsePolynomial) -> bool {
    polynomial.len() == 1
        && polynomial[0].0.iter().all(|exponent| *exponent == 0)
        && polynomial[0].1.is_one()
}

/// Makes a polynomial monic (leading coefficient 1), the normalization the reference's Euclidean
/// gcd applies to every remainder so the reported common factor is canonical.
fn sparse_monic(polynomial: &SparsePolynomial) -> SparsePolynomial {
    let mut polynomial = polynomial.clone();
    let Some((_, leading)) = sparse_leading_term(&polynomial).cloned() else {
        return polynomial;
    };
    if leading.is_one() {
        return polynomial;
    }
    for (_, coefficient) in polynomial.iter_mut() {
        *coefficient = coefficient
            .div(&leading)
            .expect("a monic normalization divides by its nonzero leading coefficient");
    }
    polynomial
}

/// The reference's multivariate long division (Polynomial.DivRem under Lex). A leading term the
/// divisor's leading monomial does not divide moves to the remainder; everything else is reduced.
fn sparse_div_rem(
    dividend: &SparsePolynomial,
    divisor: &SparsePolynomial,
) -> Option<(SparsePolynomial, SparsePolynomial)> {
    if divisor.is_empty() {
        return None;
    }
    let mut quotient = SparsePolynomial::new();
    let mut remainder = SparsePolynomial::new();
    let mut working = dividend.clone();
    while !working.is_empty() {
        let (leading_exponents, leading_coefficient) = sparse_leading_term(&working)
            .expect("a non-empty polynomial has a leading term")
            .clone();
        let (divisor_exponents, divisor_coefficient) = sparse_leading_term(divisor)
            .expect("a non-empty divisor has a leading term")
            .clone();
        if sparse_monomial_divides(&leading_exponents, &divisor_exponents) {
            let quotient_exponents =
                sparse_monomial_quotient(&leading_exponents, &divisor_exponents);
            let quotient_coefficient = leading_coefficient.div(&divisor_coefficient)?;
            sparse_add_term(
                &mut quotient,
                quotient_exponents.clone(),
                quotient_coefficient.clone(),
            );
            let subtrahend =
                sparse_multiply_term(divisor, &quotient_exponents, &quotient_coefficient);
            working = sparse_subtract(&working, &subtrahend);
        } else {
            sparse_add_term(
                &mut remainder,
                leading_exponents.clone(),
                leading_coefficient.clone(),
            );
            sparse_add_term(&mut working, leading_exponents, leading_coefficient.neg());
        }
    }
    Some((quotient, remainder))
}

/// Euclidean gcd over Q, normalized to a monic polynomial.
///
/// A multivariate remainder sequence can cycle when the polynomials are relatively prime (the
/// reference's own loop can: `cancel(y/x)`). A repeated state means the gcd is a unit, and the
/// step cap is the same conservative answer for a sequence that neither converges nor repeats
/// within a bounded number of reductions. The cap keeps the additive API total; it never turns a
/// converged gcd into one.
fn sparse_gcd(left: &SparsePolynomial, right: &SparsePolynomial) -> SparsePolynomial {
    let width = left
        .iter()
        .chain(right.iter())
        .map(|(exponents, _)| exponents.len())
        .max()
        .unwrap_or(0);
    let unit = || vec![(vec![0u32; width], Rat::one())];
    let mut left = sparse_monic(left);
    let mut right = sparse_monic(right);
    let mut history: Vec<(SparsePolynomial, SparsePolynomial)> = Vec::new();
    let mut steps = 0usize;
    while !right.is_empty() {
        if steps >= 10_000
            || history
                .iter()
                .any(|(seen_left, seen_right)| *seen_left == left && *seen_right == right)
        {
            return unit();
        }
        history.push((left.clone(), right.clone()));
        let (_, remainder) =
            sparse_div_rem(&left, &right).expect("a nonzero divisor always divides");
        left = right;
        right = sparse_monic(&remainder);
        steps += 1;
    }
    left
}

/// Exact polynomial division: the quotient when the remainder is zero (a common factor always
/// divides), None otherwise.
fn sparse_exact_quotient(
    dividend: &SparsePolynomial,
    divisor: &SparsePolynomial,
) -> Option<SparsePolynomial> {
    let (quotient, remainder) = sparse_div_rem(dividend, divisor)?;
    if remainder.is_empty() {
        Some(quotient)
    } else {
        None
    }
}

/// Renders a sparse polynomial back to the canonical expression the result publishes.
fn sparse_to_expr(polynomial: &SparsePolynomial, variables: &[String]) -> Expr {
    let mut accumulator = Expr::num(0);
    for (exponents, coefficient) in polynomial {
        let mut term = Expr::Num(coefficient.clone());
        for (index, variable) in variables.iter().enumerate() {
            let exponent = exponents.get(index).copied().unwrap_or(0);
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
    use lovelace_int::Int;
    use lovelace_nat::Nat;

    fn x() -> Expr {
        Expr::sym("x")
    }

    fn y() -> Expr {
        Expr::sym("y")
    }

    fn rational(numerator: i64, denominator: i64) -> Rat {
        Rat::from_parts(Int::from_i64(numerator), Nat::from_u64(denominator as u64))
    }

    fn quotient(numerator: Expr, denominator: Expr) -> Expr {
        Expr::div(numerator, denominator)
    }

    fn add_all(terms: Vec<Expr>) -> Expr {
        terms.into_iter().fold(Expr::num(0), Expr::add)
    }

    /// Substitutes rational bindings and evaluates exactly; None is the reference's
    /// `EvaluationException` (the input is undefined at that point).
    fn evaluate(expression: &Expr, bindings: &[(&str, Rat)]) -> Option<Rat> {
        match expression {
            Expr::Num(value) => Some(value.clone()),
            Expr::Sym(name) => bindings
                .iter()
                .find(|(bound, _)| *bound == name)
                .map(|(_, value)| value.clone()),
            Expr::Add(terms) => {
                let mut total = Rat::zero();
                for term in terms {
                    total = total.add(&evaluate(term, bindings)?);
                }
                Some(total)
            }
            Expr::Mul(factors) => {
                let mut total = Rat::one();
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

    #[test]
    fn try_rationalize_returns_the_documented_numerator_denominator_pairs() {
        let (numerator, denominator) =
            try_rationalize(&quotient(x(), x())).expect("x/x is rational");
        assert_eq!(numerator, x());
        assert_eq!(denominator, x());

        let top = Expr::sub(Expr::pow(x(), Expr::num(2)), Expr::num(1));
        let bottom = Expr::sub(x(), Expr::num(1));
        let (numerator, denominator) =
            try_rationalize(&quotient(top.clone(), bottom.clone())).expect("quotient is rational");
        assert_eq!(numerator, top);
        assert_eq!(denominator, bottom);

        let (numerator, denominator) =
            try_rationalize(&quotient(Expr::mul(x(), y()), Expr::sub(x(), Expr::num(1))))
                .expect("multivariate quotient is rational");
        assert_eq!(numerator, Expr::mul(x(), y()));
        assert_eq!(denominator, Expr::sub(x(), Expr::num(1)));
    }

    #[test]
    fn try_rationalize_returns_a_constant_over_one() {
        let (numerator, denominator) =
            try_rationalize(&Expr::num(5)).expect("constant is rational");
        assert_eq!(numerator, Expr::num(5));
        assert_eq!(denominator, Expr::num(1));
    }

    #[test]
    fn try_rationalize_refuses_a_non_polynomial_numerator() {
        // sin(x) is not a polynomial to the parser, so the reference's TryRationalize refuses.
        assert_eq!(
            try_rationalize(&Expr::Fun("sin".to_string(), vec![x()])),
            None
        );
        // A quotient whose denominator is a sum is still a polynomial denominator.
        let (numerator, denominator) = try_rationalize(&quotient(
            Expr::num(1),
            Expr::add(Expr::pow(x(), Expr::num(2)), Expr::num(1)),
        ))
        .expect("polynomial denominator");
        assert_eq!(numerator, Expr::num(1));
        assert_eq!(
            denominator,
            Expr::add(Expr::pow(x(), Expr::num(2)), Expr::num(1))
        );
    }

    #[test]
    fn try_rationalize_of_a_sum_returns_the_input_over_one() {
        // The reference deliberately does not put several additive terms over a common
        // denominator: the pair is the input over one.
        let sum = Expr::add(x(), quotient(Expr::num(1), x()));
        let (numerator, denominator) = try_rationalize(&sum).expect("each term is rational");
        assert_eq!(numerator, sum);
        assert_eq!(denominator, Expr::num(1));
    }

    #[test]
    fn cancel_reduces_the_documented_corpus() {
        let cases: Vec<(Expr, Expr)> = vec![
            (quotient(x(), x()), Expr::num(1)),
            (quotient(Expr::pow(x(), Expr::num(2)), x()), x()),
            (
                quotient(
                    Expr::sub(Expr::pow(x(), Expr::num(2)), Expr::num(1)),
                    Expr::sub(x(), Expr::num(1)),
                ),
                Expr::add(x(), Expr::num(1)),
            ),
            (
                quotient(
                    Expr::sub(Expr::pow(x(), Expr::num(2)), Expr::num(4)),
                    Expr::sub(x(), Expr::num(2)),
                ),
                Expr::add(x(), Expr::num(2)),
            ),
            (
                quotient(
                    Expr::pow(Expr::sub(x(), Expr::num(1)), Expr::num(2)),
                    Expr::sub(x(), Expr::num(1)),
                ),
                Expr::sub(x(), Expr::num(1)),
            ),
            (
                quotient(
                    add_all(vec![
                        Expr::num(1),
                        Expr::mul(Expr::num(-2), x()),
                        Expr::pow(x(), Expr::num(2)),
                    ]),
                    Expr::sub(x(), Expr::num(1)),
                ),
                Expr::sub(x(), Expr::num(1)),
            ),
            (
                quotient(
                    Expr::sub(Expr::pow(x(), Expr::num(3)), x()),
                    Expr::sub(Expr::pow(x(), Expr::num(2)), x()),
                ),
                Expr::add(x(), Expr::num(1)),
            ),
            (
                quotient(
                    Expr::sub(Expr::pow(x(), Expr::num(3)), Expr::num(8)),
                    Expr::sub(x(), Expr::num(2)),
                ),
                Expr::add(
                    add_all(vec![
                        Expr::pow(x(), Expr::num(2)),
                        Expr::mul(Expr::num(2), x()),
                    ]),
                    Expr::num(4),
                ),
            ),
            (
                quotient(
                    Expr::sub(Expr::mul(x(), y()), y()),
                    Expr::sub(x(), Expr::num(1)),
                ),
                y(),
            ),
        ];
        for (original, expected) in cases {
            assert_eq!(cancel(&original), expected, "cancel({original})");
        }
    }

    #[test]
    fn cancel_leaves_an_irreducible_quotient_unchanged() {
        let irreducible = quotient(
            Expr::add(Expr::pow(x(), Expr::num(2)), Expr::num(1)),
            Expr::sub(x(), Expr::num(1)),
        );
        assert_eq!(cancel(&irreducible), irreducible);
        // A non-rational term is refused the same way: the term is kept untouched.
        let sine = Expr::Fun("sin".to_string(), vec![x()]);
        assert_eq!(cancel(&sine), sine);
        // 1/x has no common factor; the pole is not removed.
        let reciprocal = quotient(Expr::num(1), x());
        assert_eq!(cancel(&reciprocal), reciprocal);
    }

    #[test]
    fn cancel_with_conditions_reports_the_removed_factor_as_a_non_zero_atom() {
        let result = cancel_with_conditions(&quotient(x(), x()));
        assert_eq!(result.expression, Expr::num(1));
        assert!(result.changed());
        assert_eq!(result.status(), CancelStatus::Conditional);
        assert_eq!(result.original, quotient(x(), x()));
        let atoms = result.conditions.atoms();
        assert_eq!(atoms.len(), 1);
        assert_eq!(
            atoms[0],
            Assumption::SymbolProperty("x".to_string(), SymbolPredicate::NonZero)
        );

        // A removed factor that is a sum carries the expression form of the same predicate.
        let shift = cancel_with_conditions(&quotient(
            Expr::pow(Expr::sub(x(), Expr::num(1)), Expr::num(2)),
            Expr::sub(x(), Expr::num(1)),
        ));
        assert_eq!(shift.expression, Expr::sub(x(), Expr::num(1)));
        let atoms = shift.conditions.atoms();
        assert_eq!(atoms.len(), 1);
        assert_eq!(
            atoms[0],
            Assumption::ExpressionProperty(Expr::sub(x(), Expr::num(1)), SymbolPredicate::NonZero)
        );
    }

    #[test]
    fn cancel_with_conditions_is_exact_when_no_factor_is_removed() {
        let irreducible = quotient(
            Expr::add(Expr::pow(x(), Expr::num(2)), Expr::num(1)),
            Expr::sub(x(), Expr::num(1)),
        );
        let result = cancel_with_conditions(&irreducible);
        assert_eq!(result.expression, irreducible);
        assert_eq!(result.original, irreducible);
        assert!(!result.changed());
        assert_eq!(result.status(), CancelStatus::Exact);
        assert!(result.conditions.atoms().is_empty());
        // The bare form agrees with the structured form.
        assert_eq!(cancel(&result.original), result.expression);
    }

    #[test]
    fn cancel_conditions_are_the_same_atoms_as_non_zero_condition() {
        let result = cancel_with_conditions(&quotient(x(), x()));
        assert_eq!(result.conditions, non_zero_condition(&x()));
        let shift = cancel_with_conditions(&quotient(
            Expr::pow(Expr::sub(x(), Expr::num(1)), Expr::num(2)),
            Expr::sub(x(), Expr::num(1)),
        ));
        assert_eq!(
            shift.conditions,
            non_zero_condition(&Expr::sub(x(), Expr::num(1)))
        );
    }

    #[test]
    fn cancel_expands_an_unexpanded_power_of_a_sum_before_reducing() {
        let unexpanded = quotient(
            Expr::pow(Expr::sub(x(), Expr::num(1)), Expr::num(2)),
            Expr::sub(x(), Expr::num(1)),
        );
        assert_eq!(cancel(&unexpanded), Expr::sub(x(), Expr::num(1)));

        // The expanded spelling agrees, and the parser boundary is unchanged: a power of a sum
        // is not a polynomial, a power of a symbol is.
        let expanded = quotient(
            expand(&Expr::pow(Expr::sub(x(), Expr::num(1)), Expr::num(2))),
            Expr::sub(x(), Expr::num(1)),
        );
        assert_eq!(cancel(&expanded), Expr::sub(x(), Expr::num(1)));
        let variables = vec!["x".to_string()];
        assert!(
            Polynomial::from_expr(
                &Expr::pow(Expr::sub(x(), Expr::num(1)), Expr::num(2)),
                &variables
            )
            .is_none()
        );
        assert!(Polynomial::from_expr(&Expr::pow(x(), Expr::num(2)), &variables).is_some());
    }

    #[test]
    fn cancel_preserves_value_off_poles_and_the_pole_is_excluded() {
        struct Case {
            original: Expr,
            poles: Vec<Rat>,
            pole_removed: bool,
        }
        let cases = vec![
            Case {
                original: quotient(x(), x()),
                poles: vec![Rat::zero()],
                pole_removed: true,
            },
            Case {
                original: quotient(Expr::pow(x(), Expr::num(2)), x()),
                poles: vec![Rat::zero()],
                pole_removed: true,
            },
            Case {
                original: quotient(
                    Expr::sub(Expr::pow(x(), Expr::num(2)), Expr::num(1)),
                    Expr::sub(x(), Expr::num(1)),
                ),
                poles: vec![Rat::from_i64(1)],
                pole_removed: true,
            },
            Case {
                original: quotient(
                    Expr::sub(Expr::pow(x(), Expr::num(2)), Expr::num(4)),
                    Expr::sub(x(), Expr::num(2)),
                ),
                poles: vec![Rat::from_i64(2)],
                pole_removed: true,
            },
            Case {
                original: quotient(
                    Expr::pow(Expr::sub(x(), Expr::num(1)), Expr::num(2)),
                    Expr::sub(x(), Expr::num(1)),
                ),
                poles: vec![Rat::from_i64(1)],
                pole_removed: true,
            },
            Case {
                original: quotient(
                    add_all(vec![
                        Expr::num(1),
                        Expr::mul(Expr::num(-2), x()),
                        Expr::pow(x(), Expr::num(2)),
                    ]),
                    Expr::sub(x(), Expr::num(1)),
                ),
                poles: vec![Rat::from_i64(1)],
                pole_removed: true,
            },
            Case {
                original: quotient(
                    Expr::sub(Expr::pow(x(), Expr::num(3)), x()),
                    Expr::sub(Expr::pow(x(), Expr::num(2)), x()),
                ),
                poles: vec![Rat::zero(), Rat::from_i64(1)],
                pole_removed: true,
            },
            Case {
                original: quotient(
                    Expr::sub(Expr::pow(x(), Expr::num(3)), Expr::num(8)),
                    Expr::sub(x(), Expr::num(2)),
                ),
                poles: vec![Rat::from_i64(2)],
                pole_removed: true,
            },
            Case {
                original: quotient(
                    Expr::sub(Expr::mul(x(), y()), y()),
                    Expr::sub(x(), Expr::num(1)),
                ),
                poles: vec![Rat::from_i64(1)],
                pole_removed: true,
            },
            Case {
                original: quotient(
                    Expr::add(Expr::pow(x(), Expr::num(2)), Expr::num(1)),
                    Expr::sub(x(), Expr::num(1)),
                ),
                poles: vec![Rat::from_i64(1)],
                pole_removed: false,
            },
        ];
        let samples = [
            rational(-4, 1),
            rational(-3, 2),
            rational(-1, 3),
            rational(2, 3),
            rational(4, 3),
            rational(5, 2),
            rational(3, 1),
            rational(7, 1),
        ];
        for case in cases {
            let reduced = cancel(&case.original);
            for point in &samples {
                assert!(
                    !case.poles.iter().any(|pole| pole == point),
                    "sample {point} is a pole of {}",
                    case.original
                );
                let bindings = [("x", point.clone()), ("y", Rat::from_i64(3))];
                let before = evaluate(&case.original, &bindings)
                    .unwrap_or_else(|| panic!("{} is undefined at {point}", case.original));
                let after = evaluate(&reduced, &bindings)
                    .unwrap_or_else(|| panic!("{reduced} is undefined at {point}"));
                assert_eq!(before, after, "{} changed value at {point}", case.original);
            }
            for pole in &case.poles {
                let bindings = [("x", pole.clone()), ("y", Rat::from_i64(3))];
                assert!(
                    evaluate(&case.original, &bindings).is_none(),
                    "{} is defined at its pole {pole}",
                    case.original
                );
                let reduced_at_pole = evaluate(&reduced, &bindings);
                if case.pole_removed {
                    assert!(
                        reduced_at_pole.is_some(),
                        "{reduced} still undefined at removed pole {pole}"
                    );
                } else {
                    assert!(
                        reduced_at_pole.is_none(),
                        "{reduced} unexpectedly defined at pole {pole}"
                    );
                }
            }
        }
    }

    #[test]
    fn cancel_reduces_each_additive_term_and_collects_every_condition() {
        let sum = Expr::add(quotient(x(), x()), quotient(y(), y()));
        let result = cancel_with_conditions(&sum);
        assert_eq!(result.expression, Expr::num(2));
        assert!(result.changed());
        assert_eq!(result.status(), CancelStatus::Conditional);
        assert_eq!(result.conditions.atoms().len(), 2);
        assert!(result.conditions.contains(&Assumption::SymbolProperty(
            "x".to_string(),
            SymbolPredicate::NonZero
        )));
        assert!(result.conditions.contains(&Assumption::SymbolProperty(
            "y".to_string(),
            SymbolPredicate::NonZero
        )));
    }

    #[test]
    fn cancel_zero_over_x_keeps_the_removed_pole_condition() {
        let result = cancel_with_conditions(&quotient(Expr::num(0), x()));
        assert_eq!(result.expression, Expr::num(0));
        assert!(result.changed());
        assert_eq!(result.conditions, non_zero_condition(&x()));
    }
}
