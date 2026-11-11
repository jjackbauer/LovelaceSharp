//! Sparse multivariate polynomials over the rationals and Buchberger Gröbner bases.
//!
//! This is the port of `Lovelace.Symbolics.Algebra.Groebner` / `Polynomial` that the
//! system solver needs: the same variable order, monomial orders, monic normalization,
//! Buchberger pair loop, reduction and interreduction. Correctness first: the algorithm is the
//! reference's own, not a shortcut that happens to answer the documented examples.

use std::cmp::Ordering;

use crate::{Expr, Rat};

/// The monomial order a Gröbner computation uses (the reference's `MonomialOrder`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MonomialOrder {
    /// Pure lexicographic order over the variable sequence.
    Lex,
    /// Graded lexicographic: total degree first, then lex.
    GrLex,
    /// Graded reverse lexicographic: total degree first, then reverse lex.
    GrRevLex,
}

/// A dense exponent vector over a polynomial's variable order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Monomial {
    exps: Vec<u32>,
}

impl Monomial {
    /// The exponent of the variable at `index` (zero past the stored width).
    pub fn exponent(&self, index: usize) -> u32 {
        self.exps.get(index).copied().unwrap_or(0)
    }

    /// The total degree (sum of exponents).
    pub fn total_degree(&self) -> u32 {
        self.exps.iter().sum()
    }

    fn unit(width: usize) -> Monomial {
        Monomial { exps: vec![0; width] }
    }

    fn with(width: usize, index: usize, exponent: u32) -> Monomial {
        let mut exps = vec![0; width];
        if index < width {
            exps[index] = exponent;
        }
        Monomial { exps }
    }

    fn multiply(&self, other: &Monomial) -> Monomial {
        let width = self.exps.len().max(other.exps.len());
        let mut exps = vec![0; width];
        for i in 0..width {
            exps[i] = self.exponent(i) + other.exponent(i);
        }
        Monomial { exps }
    }

    fn divide(&self, other: &Monomial) -> Option<Monomial> {
        let width = self.exps.len().max(other.exps.len());
        let mut exps = vec![0; width];
        for i in 0..width {
            let a = self.exponent(i);
            let b = other.exponent(i);
            if a < b {
                return None;
            }
            exps[i] = a - b;
        }
        Some(Monomial { exps })
    }

    fn lcm(&self, other: &Monomial) -> Monomial {
        let width = self.exps.len().max(other.exps.len());
        let mut exps = vec![0; width];
        for i in 0..width {
            exps[i] = self.exponent(i).max(other.exponent(i));
        }
        Monomial { exps }
    }

    fn divisible_by(&self, other: &Monomial) -> bool {
        let width = self.exps.len().max(other.exps.len());
        (0..width).all(|i| self.exponent(i) >= other.exponent(i))
    }
}

/// The reference's `MonomialComparer`: positive when `a` is the greater monomial.
fn compare_monomials(a: &Monomial, b: &Monomial, order: MonomialOrder) -> Ordering {
    if order != MonomialOrder::Lex {
        let degree = a.total_degree().cmp(&b.total_degree());
        if degree != Ordering::Equal {
            return degree;
        }
    }
    let n = a.exps.len().max(b.exps.len());
    for i in 0..n {
        if order == MonomialOrder::GrRevLex {
            let index = n - 1 - i;
            let cmp = a.exponent(index).cmp(&b.exponent(index));
            if cmp != Ordering::Equal {
                return cmp.reverse();
            }
        } else {
            let cmp = a.exponent(i).cmp(&b.exponent(i));
            if cmp != Ordering::Equal {
                return cmp;
            }
        }
    }
    Ordering::Equal
}

/// A sparse multivariate polynomial with rational coefficients over a fixed variable order.
#[derive(Clone, Debug)]
pub struct Polynomial {
    variables: Vec<String>,
    terms: Vec<(Monomial, Rat)>,
}

impl Polynomial {
    /// The zero polynomial over the given variable order.
    pub fn new(variables: &[String]) -> Polynomial {
        Polynomial { variables: variables.to_vec(), terms: Vec::new() }
    }

    /// The variable order this polynomial lives in.
    pub fn variables(&self) -> &[String] {
        &self.variables
    }

    /// The terms, each as (monomial, coefficient); zero terms are never stored.
    pub fn terms(&self) -> &[(Monomial, Rat)] {
        &self.terms
    }

    /// True when the polynomial is zero.
    pub fn is_zero(&self) -> bool {
        self.terms.is_empty()
    }

    /// True when the polynomial is the constant one.
    pub fn is_one(&self) -> bool {
        self.terms.len() == 1
            && self.terms[0].0.total_degree() == 0
            && self.terms[0].1.is_one()
    }

    /// The total degree, or -1 for the zero polynomial.
    pub fn total_degree(&self) -> i64 {
        if self.terms.is_empty() {
            return -1;
        }
        self.terms.iter().map(|(m, _)| m.total_degree() as i64).max().unwrap_or(-1)
    }

    /// The leading term under `order`.
    pub fn leading_term(&self, order: MonomialOrder) -> Option<(&Monomial, &Rat)> {
        let mut best: Option<&(Monomial, Rat)> = None;
        for term in &self.terms {
            match best {
                None => best = Some(term),
                Some(current) => {
                    if compare_monomials(&term.0, &current.0, order) == Ordering::Greater {
                        best = Some(term);
                    }
                }
            }
        }
        best.map(|(m, c)| (m, c))
    }

    /// The leading coefficient under `order`.
    pub fn leading_coefficient(&self, order: MonomialOrder) -> Rat {
        self.leading_term(order).map(|(_, c)| c.clone()).unwrap_or_else(Rat::zero)
    }

    /// The constant term.
    pub fn constant_term(&self) -> Rat {
        let unit = Monomial::unit(self.variables.len());
        self.terms
            .iter()
            .find(|(m, _)| *m == unit)
            .map(|(_, c)| c.clone())
            .unwrap_or_else(Rat::zero)
    }

    /// Adds a term, combining an existing monomial and dropping a zero coefficient.
    fn add_term(&mut self, monomial: Monomial, coefficient: Rat) {
        if coefficient.is_zero() {
            return;
        }
        if let Some(index) = self.terms.iter().position(|(m, _)| *m == monomial) {
            let sum = self.terms[index].1.add(&coefficient);
            if sum.is_zero() {
                self.terms.remove(index);
            } else {
                self.terms[index].1 = sum;
            }
            return;
        }
        self.terms.push((monomial, coefficient));
    }

    /// A polynomial holding one term.
    pub fn from_term(variables: &[String], monomial: Monomial, coefficient: Rat) -> Polynomial {
        let mut p = Polynomial::new(variables);
        p.add_term(monomial, coefficient);
        p
    }

    /// The constant polynomial `c`.
    pub fn from_constant(variables: &[String], c: Rat) -> Polynomial {
        Polynomial::from_term(variables, Monomial::unit(variables.len()), c)
    }

    /// The variable at `index`.
    pub fn from_variable(variables: &[String], index: usize) -> Polynomial {
        Polynomial::from_term(
            variables,
            Monomial::with(variables.len(), index, 1),
            Rat::one(),
        )
    }

    /// Converts an expression to a polynomial over `variables`, or None when it is not a
    /// polynomial (the reference's `TryFromExpr` refusal).
    pub fn from_expr(e: &Expr, variables: &[String]) -> Option<Polynomial> {
        match e {
            Expr::Num(c) => Some(Polynomial::from_constant(variables, c.clone())),
            Expr::Sym(name) => {
                let index = variables.iter().position(|v| v == name)?;
                Some(Polynomial::from_variable(variables, index))
            }
            Expr::Add(items) => {
                let mut p = Polynomial::new(variables);
                for item in items {
                    let term = Polynomial::from_expr(item, variables)?;
                    p = Polynomial::add(&p, &term);
                }
                Some(p)
            }
            Expr::Mul(items) => {
                let mut p = Polynomial::from_constant(variables, Rat::one());
                for item in items {
                    let factor = Polynomial::from_expr(item, variables)?;
                    p = Polynomial::multiply(&p, &factor);
                }
                Some(p)
            }
            Expr::Pow(base, exponent) => {
                let Expr::Sym(name) = base.as_ref() else {
                    return None;
                };
                let Expr::Num(power) = exponent.as_ref() else {
                    return None;
                };
                if !power.is_integer() || power.is_negative() {
                    return None;
                }
                let index = variables.iter().position(|v| v == name)?;
                let exponent = power.to_i64()?;
                if exponent < 0 || exponent > u32::MAX as i64 {
                    return None;
                }
                Some(Polynomial::from_term(
                    variables,
                    Monomial::with(variables.len(), index, exponent as u32),
                    Rat::one(),
                ))
            }
            _ => None,
        }
    }

    /// Renders the polynomial back to a canonical expression.
    pub fn to_expr(&self) -> Expr {
        let mut terms: Vec<Expr> = Vec::with_capacity(self.terms.len());
        for (monomial, coefficient) in &self.terms {
            let mut factors: Vec<Expr> = Vec::new();
            if !coefficient.is_one() {
                factors.push(Expr::Num(coefficient.clone()));
            }
            for (i, variable) in self.variables.iter().enumerate() {
                let exponent = monomial.exponent(i);
                if exponent == 0 {
                    continue;
                }
                let symbol = Expr::sym(variable);
                factors.push(if exponent == 1 {
                    symbol
                } else {
                    Expr::pow(symbol, Expr::num(exponent as i64))
                });
            }
            terms.push(match factors.len() {
                0 => Expr::num(1),
                1 => factors.pop().unwrap(),
                _ => Expr::Mul(factors),
            });
        }
        match terms.len() {
            0 => Expr::num(0),
            1 => terms.pop().unwrap(),
            _ => Expr::Add(terms),
        }
    }

    /// Sum of two polynomials in the same order.
    pub fn add(a: &Polynomial, b: &Polynomial) -> Polynomial {
        let mut result = a.clone();
        for (m, c) in &b.terms {
            result.add_term(m.clone(), c.clone());
        }
        result
    }

    /// Difference of two polynomials.
    pub fn subtract(a: &Polynomial, b: &Polynomial) -> Polynomial {
        Polynomial::add(a, &Polynomial::negate(b))
    }

    /// Negation.
    pub fn negate(a: &Polynomial) -> Polynomial {
        Polynomial {
            variables: a.variables.clone(),
            terms: a.terms.iter().map(|(m, c)| (m.clone(), c.neg())).collect(),
        }
    }

    /// Product of two polynomials.
    pub fn multiply(a: &Polynomial, b: &Polynomial) -> Polynomial {
        let mut result = Polynomial::new(&a.variables);
        for (ma, ca) in &a.terms {
            for (mb, cb) in &b.terms {
                result.add_term(ma.multiply(mb), ca.mul(cb));
            }
        }
        result
    }

    /// Scales every coefficient so the Lex-leading coefficient is one.
    pub fn monic(&self) -> Polynomial {
        if self.is_zero() {
            return self.clone();
        }
        let leading = self.leading_coefficient(MonomialOrder::Lex);
        if leading.is_one() {
            return self.clone();
        }
        let mut result = Polynomial::new(&self.variables);
        for (m, c) in &self.terms {
            if let Some(quotient) = c.div(&leading) {
                result.add_term(m.clone(), quotient);
            }
        }
        result
    }

    /// True when no term mentions any of the named variables (by exponent, not by position).
    pub fn is_free_of(&self, names: &[String]) -> bool {
        let indices: Vec<usize> = names
            .iter()
            .filter_map(|name| self.variables.iter().position(|v| v == name))
            .collect();
        self.terms.iter().all(|(m, _)| indices.iter().all(|i| m.exponent(*i) == 0))
    }
}

/// Reduces `p` to a normal form against `basis` (multivariate division remainder).
pub fn reduce(p: &Polynomial, basis: &[Polynomial], order: MonomialOrder) -> Polynomial {
    let mut remainder = Polynomial::new(p.variables());
    let mut current = p.clone();
    while !current.is_zero() {
        let (leading, coefficient) = match current.leading_term(order) {
            Some((m, c)) => (m.clone(), c.clone()),
            None => break,
        };
        let mut reduced = false;
        for divisor in basis {
            if divisor.is_zero() {
                continue;
            }
            let Some((divisor_leading, divisor_coefficient)) = divisor.leading_term(order) else {
                continue;
            };
            if !leading.divisible_by(divisor_leading) {
                continue;
            }
            let quotient_monomial = leading.divide(divisor_leading).expect("divisibility checked");
            let Some(quotient_coefficient) = coefficient.div(divisor_coefficient) else {
                continue;
            };
            let quotient = Polynomial::from_term(
                p.variables(),
                quotient_monomial,
                quotient_coefficient,
            );
            current = Polynomial::subtract(&current, &Polynomial::multiply(&quotient, divisor));
            reduced = true;
            break;
        }
        if !reduced {
            let term = Polynomial::from_term(p.variables(), leading, coefficient);
            remainder = Polynomial::add(&remainder, &term);
            current = Polynomial::subtract(&current, &term);
        }
    }
    remainder
}

fn s_polynomial(f: &Polynomial, g: &Polynomial, order: MonomialOrder) -> Polynomial {
    let (lf, cf) = f.leading_term(order).expect("nonzero S-polynomial operand");
    let (lg, cg) = g.leading_term(order).expect("nonzero S-polynomial operand");
    let lcm = lf.lcm(lg);
    let mf = lcm.divide(lf).expect("lcm divisible by leading monomial");
    let mg = lcm.divide(lg).expect("lcm divisible by leading monomial");
    let left_coefficient = Rat::one().div(cf).expect("nonzero leading coefficient");
    let right_coefficient = Rat::one().div(cg).expect("nonzero leading coefficient");
    let left = Polynomial::multiply(
        &Polynomial::from_term(f.variables(), mf, left_coefficient),
        f,
    );
    let right = Polynomial::multiply(
        &Polynomial::from_term(g.variables(), mg, right_coefficient),
        g,
    );
    Polynomial::subtract(&left, &right)
}

fn interreduce(basis: Vec<Polynomial>, order: MonomialOrder) -> Vec<Polynomial> {
    let mut result: Vec<Polynomial> = Vec::new();
    for (i, polynomial) in basis.iter().enumerate() {
        let others: Vec<Polynomial> = basis
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != i)
            .map(|(_, p)| p.clone())
            .collect();
        let reduced = reduce(polynomial, &others, order);
        if !reduced.is_zero() {
            result.push(reduced.monic());
        }
    }
    result.sort_by(|a, b| {
        let (ma, _) = a.leading_term(order).expect("nonempty basis element");
        let (mb, _) = b.leading_term(order).expect("nonempty basis element");
        compare_monomials(ma, mb, order)
    });
    result
}

/// Computes a reduced Gröbner basis of the ideal generated by `generators`.
pub fn groebner_basis(generators: &[Polynomial], order: MonomialOrder) -> Vec<Polynomial> {
    let mut basis: Vec<Polynomial> = generators
        .iter()
        .filter(|g| !g.is_zero())
        .map(|g| g.monic())
        .collect();
    if basis.is_empty() {
        return Vec::new();
    }
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    for i in 0..basis.len() {
        for j in (i + 1)..basis.len() {
            pairs.push((i, j));
        }
    }
    let mut seen = pairs.clone();
    while let Some((i, j)) = if pairs.is_empty() { None } else { Some(pairs.remove(0)) } {
        let s = s_polynomial(&basis[i], &basis[j], order);
        if s.is_zero() {
            continue;
        }
        let remainder = reduce(&s, &basis, order);
        if remainder.is_zero() {
            continue;
        }
        let k = basis.len();
        basis.push(remainder.monic());
        for t in 0..k {
            if !seen.contains(&(t, k)) {
                seen.push((t, k));
                pairs.push((t, k));
            }
        }
    }
    interreduce(basis, order)
}

/// Eliminates the named variables from the ideal: the Lex-basis generators that no longer
/// mention them (the elimination theorem).
pub fn eliminate(
    generators: &[Polynomial],
    eliminated: &[String],
    order: MonomialOrder,
) -> Vec<Polynomial> {
    groebner_basis(generators, order)
        .into_iter()
        .filter(|p| p.is_free_of(eliminated))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars() -> Vec<String> {
        vec!["x".to_string(), "y".to_string()]
    }

    fn x() -> Expr {
        Expr::sym("x")
    }

    fn y() -> Expr {
        Expr::sym("y")
    }

    fn poly(expression: Expr) -> Polynomial {
        Polynomial::from_expr(&expression, &vars()).expect("expression is a polynomial")
    }

    fn circle() -> Polynomial {
        poly(Expr::sub(
            Expr::add(Expr::pow(x(), Expr::num(2)), Expr::pow(y(), Expr::num(2))),
            Expr::num(1),
        ))
    }

    fn hyperbola() -> Polynomial {
        poly(Expr::mul(x(), y()))
    }

    fn assert_is_basis(basis: &[Polynomial], order: MonomialOrder) {
        for element in basis {
            let remainder = reduce(element, basis, order);
            assert!(
                remainder.is_zero(),
                "basis element {} does not reduce to zero",
                element.to_expr()
            );
        }
    }

    #[test]
    fn circle_hyperbola_grlex_basis_has_the_documented_three_elements() {
        let basis = groebner_basis(&[circle(), hyperbola()], MonomialOrder::GrLex);
        assert_eq!(basis.len(), 3);
        assert_is_basis(&basis, MonomialOrder::GrLex);
        let eliminated = poly(Expr::sub(
            Expr::pow(y(), Expr::num(3)),
            y(),
        ));
        assert!(reduce(&eliminated, &basis, MonomialOrder::GrLex).is_zero());
    }

    #[test]
    fn eliminate_x_from_circle_hyperbola_is_y_cubed_minus_y() {
        let eliminated = eliminate(
            &[circle(), hyperbola()],
            &["x".to_string()],
            MonomialOrder::Lex,
        );
        assert_eq!(eliminated.len(), 1);
        let text = eliminated[0].to_expr().to_string().replace(' ', "");
        assert_eq!(text, "y^3-y");
    }

    #[test]
    fn trivial_ideal_from_x_and_x_plus_one_is_one() {
        let basis = groebner_basis(
            &[poly(x()), poly(Expr::add(x(), Expr::num(1)))],
            MonomialOrder::GrLex,
        );
        assert_eq!(basis.len(), 1);
        assert_eq!(basis[0].to_expr().to_string(), "1");
    }

    #[test]
    fn reduce_normal_form_is_idempotent() {
        let basis = groebner_basis(&[circle(), hyperbola()], MonomialOrder::GrLex);
        let p = poly(Expr::add(
            Expr::pow(x(), Expr::num(3)),
            Expr::mul(Expr::pow(x(), Expr::num(2)), y()),
        ));
        let once = reduce(&p, &basis, MonomialOrder::GrLex);
        let twice = reduce(&once, &basis, MonomialOrder::GrLex);
        let difference = Polynomial::subtract(&once, &twice);
        assert!(difference.is_zero());
    }

    #[test]
    fn lex_basis_of_ordered_monomials_keeps_three_elements() {
        let basis = groebner_basis(
            &[
                poly(Expr::pow(x(), Expr::num(2))),
                poly(Expr::mul(x(), y())),
                poly(Expr::pow(y(), Expr::num(2))),
            ],
            MonomialOrder::Lex,
        );
        assert_eq!(basis.len(), 3);
        assert_is_basis(&basis, MonomialOrder::Lex);
    }

    #[test]
    fn quadratic_with_a_positive_nonsquare_discriminant_has_exact_radical_roots() {
        let outcome = crate::solve_polynomial_named(
            &[Rat::from_i64(-2), Rat::zero(), Rat::one()],
            "x",
        );
        match outcome {
            crate::SolveOutcome::RootExpressions(expressions) => {
                let texts: Vec<String> = expressions.iter().map(|e| e.to_string()).collect();
                assert_eq!(texts, vec!["-1/2*sqrt(8)", "1/2*sqrt(8)"]);
            }
            other => panic!("expected exact radical roots, got {other:?}"),
        }
    }

    #[test]
    fn a_single_polynomial_is_its_own_basis() {
        let basis = groebner_basis(
            &[poly(Expr::add(
                Expr::sub(
                    Expr::pow(x(), Expr::num(3)),
                    Expr::mul(Expr::num(2), x()),
                ),
                Expr::num(1),
            ))],
            MonomialOrder::Lex,
        );
        assert_eq!(basis.len(), 1);
        assert_is_basis(&basis, MonomialOrder::Lex);
    }
}
