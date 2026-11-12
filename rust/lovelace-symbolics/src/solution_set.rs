//! The library-level solution set and the one deriving function for the published
//! `(complete, completeness)` pair.
//!
//! The reference's `Solvers.Solve` returns a `SolutionSet` whose `Status` drives both the
//! legacy `Kind` and the `Complete` field; `SolveCompletenessMapping.Of` is the single
//! derivation the records and tests share. The language layer of this crate already publishes
//! the same pairs on its `SolveResult`, but the library surface was missing: a caller could
//! observe `SolveOutcome` (a root payload) and had no honest way to ask whether the payload is
//! the complete set, or which pair a status implies.
//!
//! This module is additive. It wraps `solve_polynomial_named` with the reference's default
//! domain policy and adds the mapping table, so the two deferred census rows
//! (`Solve_QuarticNoRealRoots_DefaultDomain_ReportsUnevaluated` and
//! `EveryStatusCarriesTheDocumentedPair#3`) can be driven at the reference's own surface.

use crate::{Expr, Rat, RelOp, SolveOutcome, poly_coeffs, solve_polynomial_named};

/// The field a solution set ranges over (the reference's `SolveDomain`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SolveDomain {
    /// Only real values.
    Real,
    /// The default; an unconstrained symbol ranges over the complex field.
    Complex,
}

/// The solver's disposition (the reference's `SolveStatus`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SolveStatus {
    /// The represented set is the COMPLETE set over the requested domain.
    Solved,
    /// Some roots were found and some were not representable.
    Partial,
    /// The set over the requested domain is provably empty.
    NoSolutions,
    /// The solver has no answer for this structure.
    Unevaluated,
    /// A search budget stopped the solve.
    BudgetExceeded,
}

/// How much of the solution set a result represents (the reference's `Completeness`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Completeness {
    /// Every solution over the requested domain is represented.
    Complete,
    /// Some solutions are missing.
    Partial,
    /// The solver cannot say whether anything is missing.
    Unknown,
}

impl Completeness {
    /// The kernel spelling (`Complete`, ...), as the reference enum prints it.
    pub fn name(self) -> &'static str {
        match self {
            Completeness::Complete => "Complete",
            Completeness::Partial => "Partial",
            Completeness::Unknown => "Unknown",
        }
    }
}

/// The reference's `SolveCompletenessMapping`: one derivation of `(complete, completeness)`
/// from the effective status, so the two published fields cannot disagree.
pub struct SolveCompletenessMapping;

impl SolveCompletenessMapping {
    /// The published pair for `status`; a budget stop defaults to an unknown subset.
    pub fn of(status: SolveStatus) -> (bool, Completeness) {
        Self::of_with_partial_subset(status, Completeness::Unknown)
    }

    /// The published pair for `status`. `partial_subset` is the answer only for
    /// `BudgetExceeded`; every other status ignores it, exactly as the reference's optional
    /// parameter does.
    pub fn of_with_partial_subset(
        status: SolveStatus,
        partial_subset: Completeness,
    ) -> (bool, Completeness) {
        match status {
            SolveStatus::Solved | SolveStatus::NoSolutions => (true, Completeness::Complete),
            SolveStatus::Partial => (false, Completeness::Partial),
            SolveStatus::Unevaluated => (false, Completeness::Unknown),
            SolveStatus::BudgetExceeded => (false, partial_subset),
        }
    }
}

/// A solver outcome at the reference's library surface (the reference's `SolutionSet`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SolutionSet {
    /// The disposition.
    pub status: SolveStatus,
    /// The requested domain.
    pub domain: SolveDomain,
    /// The represented solutions, in publication order.
    pub solutions: Vec<Expr>,
    /// The human-readable reason for a non-complete answer.
    pub note: Option<String>,
    /// How many roots of the requested domain are known but not represented.
    pub unrepresented_count: u64,
    /// The machine-readable reason the roots are unrepresented.
    pub unrepresented_reason: Option<String>,
}

impl SolutionSet {
    /// The published `(complete, completeness)` pair for this set.
    pub fn completeness_pair(&self) -> (bool, Completeness) {
        SolveCompletenessMapping::of(self.status)
    }

    fn new(status: SolveStatus, domain: SolveDomain, solutions: Vec<Expr>) -> SolutionSet {
        SolutionSet {
            status,
            domain,
            solutions,
            note: None,
            unrepresented_count: 0,
            unrepresented_reason: None,
        }
    }

    fn with_note(mut self, note: impl Into<String>) -> SolutionSet {
        self.note = Some(note.into());
        self
    }

    fn unevaluated_unrepresented(domain: SolveDomain, count: u64, reason: &str) -> SolutionSet {
        SolutionSet {
            status: SolveStatus::Unevaluated,
            domain,
            solutions: Vec::new(),
            note: Some(reason.to_string()),
            unrepresented_count: count,
            unrepresented_reason: Some(reason.to_string()),
        }
    }
}

/// The reason the reference's solver publishes when a degree >= 4 factor has no real root.
pub const COMPLEX_ALGEBRAIC_ROOTS_REASON: &str =
    "complex algebraic roots not supported (RootOf is real-only in v1).";

/// The reference's `Solvers.Solve` over a polynomial equation for one variable.
///
/// The equation is `lhs == rhs` or an expression implicitly equal to zero; a non-equality
/// relation is refused exactly as the reference refuses inequalities. The default domain is
/// `Complex`. Only the polynomial dispatch is modelled here: that is the dispatch the deferred
/// quartic row names, and it is the one whose status is observable without an `ExprContext`.
pub fn solve_expression(equation: &Expr, var: &str, domain: SolveDomain) -> SolutionSet {
    let target = match equation {
        Expr::Rel(RelOp::Eq, left, right) => Expr::sub((**left).clone(), (**right).clone()),
        Expr::Rel(..) => {
            return SolutionSet::new(SolveStatus::Unevaluated, domain, Vec::new())
                .with_note("Inequality solving is not supported in v1.");
        }
        other => other.clone(),
    };
    let Some(coeffs) = poly_coeffs(&target, var) else {
        return SolutionSet::new(SolveStatus::Unevaluated, domain, Vec::new())
            .with_note("No solver for this structure.");
    };
    solve_polynomial_set(&coeffs, var, domain)
}

/// The reference's `Solvers.SolvePolynomial` status policy over ascending coefficients.
///
/// A degree >= 4 factor with no real root under the default domain has NO represented root:
/// the reference reports `Unevaluated` with every root counted as unrepresented, never
/// `Partial` with four silently dropped roots. A real-domain request for the same factor is
/// a complete empty set (`NoSolutions`, note `no real solutions`).
pub fn solve_polynomial_set(coeffs: &[Rat], var: &str, domain: SolveDomain) -> SolutionSet {
    let degree = coeffs.len().saturating_sub(1) as u64;
    match solve_polynomial_named(coeffs, var) {
        SolveOutcome::Roots(roots) => SolutionSet::new(
            SolveStatus::Solved,
            domain,
            roots
                .iter()
                .map(|root| crate::expr_from_rat(&root.value))
                .collect(),
        ),
        SolveOutcome::NoSolutions => SolutionSet::new(SolveStatus::NoSolutions, domain, Vec::new())
            .with_note("the equation reduces to a nonzero constant."),
        SolveOutcome::NoRealSolutions => {
            SolutionSet::new(SolveStatus::NoSolutions, domain, Vec::new())
                .with_note("no real solutions")
        }
        // Over the REALS a complex radical is not a solution (x^2 + 1 has none), and the
        // RootOf values the solver represents ARE the complete real set (C# HighDegreeRoots:
        // "over the reals the count real roots ARE the complete solution set").
        SolveOutcome::RootExpressions(expressions) if domain == SolveDomain::Real => {
            let real: Vec<Expr> = expressions
                .into_iter()
                .filter(|expression| !expression.has_imaginary_unit())
                .collect();
            if real.is_empty() {
                SolutionSet::new(SolveStatus::NoSolutions, domain, Vec::new())
                    .with_note("no real solutions")
            } else {
                SolutionSet::new(SolveStatus::Solved, domain, real)
            }
        }
        SolveOutcome::RootExpressionsCount(expressions, _) if domain == SolveDomain::Real => {
            SolutionSet::new(SolveStatus::Solved, domain, expressions)
        }
        SolveOutcome::RootExpressions(expressions) => {
            SolutionSet::new(SolveStatus::Solved, domain, expressions)
        }
        SolveOutcome::RootExpressionsCount(expressions, count) => {
            let mut set = SolutionSet::new(SolveStatus::Partial, domain, expressions);
            set.note = Some(COMPLEX_ALGEBRAIC_ROOTS_REASON.to_string());
            set.unrepresented_count = count;
            set.unrepresented_reason = Some(COMPLEX_ALGEBRAIC_ROOTS_REASON.to_string());
            set
        }
        SolveOutcome::Partial(roots) if roots.is_empty() => {
            if domain == SolveDomain::Real {
                SolutionSet::new(SolveStatus::NoSolutions, domain, Vec::new())
                    .with_note("no real solutions")
            } else if degree >= 4 {
                SolutionSet::unevaluated_unrepresented(
                    domain,
                    degree,
                    COMPLEX_ALGEBRAIC_ROOTS_REASON,
                )
            } else {
                SolutionSet::new(SolveStatus::Unevaluated, domain, Vec::new())
                    .with_note("No solver for this structure.")
            }
        }
        SolveOutcome::Partial(roots) => {
            let mut set = SolutionSet::new(
                SolveStatus::Partial,
                domain,
                roots.iter().map(crate::expr_from_rat).collect(),
            );
            set.note = Some(COMPLEX_ALGEBRAIC_ROOTS_REASON.to_string());
            set.unrepresented_count = degree.saturating_sub(roots.len() as u64);
            set.unrepresented_reason = Some(COMPLEX_ALGEBRAIC_ROOTS_REASON.to_string());
            set
        }
        SolveOutcome::Unevaluated => SolutionSet::new(SolveStatus::Unevaluated, domain, Vec::new())
            .with_note("No solver for this structure."),
    }
}
