//! Round 92 — the Symbolics half of the six small deferred symbolic families.
//!
//! Three families are driven here at the reference's own library surface:
//!
//! * `DxSemanticClosureTests/DomainOf_NegativeIntegerPower_IsRationalNotInteger#0`
//!   (`Domains.DomainOf` under an assumption scope);
//! * `DxSemanticClosureTests/Solve_QuarticNoRealRoots_DefaultDomain_ReportsUnevaluated#0`
//!   (library `Solvers.Solve` default domain);
//! * `SolveCompletenessMappingTests` rows `BudgetExceeded_...` and
//!   `EveryStatusCarriesTheDocumentedPair#3` (the mapping table);
//! * `RewriteProofBridgeTests` both rows (the rule to obligation join point).
//!
//! Every assertion is the oracle's own observable; no test states a weaker approximation.

use crate::assumptions::{Assumption, AssumptionSet, Domain};
use crate::proof_bridge::{ProofStatus, bridge_is_in_step, proof_obligations, proof_rule_ids};
use crate::rewrite_registry::shipped_rule_ids;
use crate::solution_set::{SolveCompletenessMapping, SolveDomain, SolveStatus, solve_expression};
use crate::{Dom, Expr, domain_of};

// ---------------------------------------------------------------------------
// DxSemanticClosureTests — Domains.DomainOf
// ---------------------------------------------------------------------------

/// C# DxSemanticClosureTests.cs:286 `DomainOf_NegativeIntegerPower_IsRationalNotInteger`:
/// under `x : Integer`, `x^-1` is Rational (never Integer), while `x^2` keeps Integer.
#[test]
fn dxsemanticclosuretests_domainof_negativeintegerpower_isrationalnotinteger() {
    let x = Expr::sym("x");
    let assumptions =
        AssumptionSet::from_atoms([Assumption::SymbolDomain("x".to_string(), Domain::Integer)]);

    let inverse = Expr::pow(x.clone(), Expr::num(-1));
    assert_eq!(
        domain_of(&inverse, &assumptions),
        Dom::Rational,
        "x^-1 under x : Integer is rational-valued, not integer"
    );
    let square = Expr::pow(x, Expr::num(2));
    assert_eq!(
        domain_of(&square, &assumptions),
        Dom::Integer,
        "x^2 under x : Integer keeps the integer domain"
    );
}

// ---------------------------------------------------------------------------
// DxSemanticClosureTests — Solvers.Solve, default domain
// ---------------------------------------------------------------------------

/// C# DxSemanticClosureTests.cs:179 `Solve_QuarticNoRealRoots_DefaultDomain_ReportsUnevaluated`:
/// `x^4 + 1` has four complex algebraic roots, none representable because RootOf is real-only.
/// The default (complex) domain answers Unevaluated with the note, never a partial set that
/// silently drops all four roots. The real domain is the complete empty set.
#[test]
fn dxsemanticclosuretests_solve_quartic_no_real_roots_default_domain_reports_unevaluated() {
    let x = Expr::sym("x");
    let equation = Expr::add(Expr::pow(x, Expr::num(4)), Expr::num(1));

    let set = solve_expression(&equation, "x", SolveDomain::Complex);
    assert_eq!(set.status, SolveStatus::Unevaluated);
    assert_eq!(set.solutions.len(), 0);
    assert_eq!(set.unrepresented_count, 4);
    let note = set.note.as_deref().expect("the reference publishes a note");
    assert!(
        note.contains("complex algebraic roots"),
        "the note must name the unrepresentable root class, got {note:?}"
    );

    let real = solve_expression(&equation, "x", SolveDomain::Real);
    assert_eq!(real.status, SolveStatus::NoSolutions);
    assert_eq!(real.note.as_deref(), Some("no real solutions"));
}

// ---------------------------------------------------------------------------
// SolveCompletenessMappingTests
// ---------------------------------------------------------------------------

/// C# SolveCompletenessMappingTests.cs:47
/// `BudgetExceeded_IsIncomplete_WithTheKernelCompletenessOfThePartialSubset`: a budget stop is
/// never complete; with no subset it is Unknown, and with an explicit Partial subset it carries
/// that subset's completeness.
#[test]
fn solvecompletenessmappingtests_budgetexceeded_is_incomplete_with_the_kernel_completeness() {
    assert_eq!(
        SolveCompletenessMapping::of(SolveStatus::BudgetExceeded),
        (false, crate::solution_set::Completeness::Unknown)
    );
    assert_eq!(
        SolveCompletenessMapping::of_with_partial_subset(
            SolveStatus::BudgetExceeded,
            crate::solution_set::Completeness::Partial,
        ),
        (false, crate::solution_set::Completeness::Partial)
    );
}

/// C# SolveCompletenessMappingTests.cs:39 rows 0-3, with row 3
/// (`EveryStatusCarriesTheDocumentedPair#3`) being the deferred one: Unevaluated pairs with
/// `complete == false` and `Unknown`, never with a claimed-complete status.
#[test]
fn solvecompletenessmappingtests_everystatuscarriesthedocumentedpair() {
    use crate::solution_set::Completeness;
    assert_eq!(
        SolveCompletenessMapping::of(SolveStatus::Solved),
        (true, Completeness::Complete)
    );
    assert_eq!(
        SolveCompletenessMapping::of(SolveStatus::NoSolutions),
        (true, Completeness::Complete)
    );
    assert_eq!(
        SolveCompletenessMapping::of(SolveStatus::Partial),
        (false, Completeness::Partial)
    );
    assert_eq!(
        SolveCompletenessMapping::of(SolveStatus::Unevaluated),
        (false, Completeness::Unknown),
        "row 3: Unevaluated is incomplete and its completeness is Unknown"
    );
}

// ---------------------------------------------------------------------------
// RewriteProofBridgeTests
// ---------------------------------------------------------------------------

/// C# RewriteProofBridgeTests.cs:14 `EveryShippedRule_HasExactlyOneProofObligation`: the
/// sorted shipped-rule ids and the sorted obligation keys are the same list. The registry already
/// shipped the nine ids; this row is the missing join point, not a rule change.
#[test]
fn rewriteproofbridgetests_everyshippedrule_hasexactlyoneproofobligation() {
    let mut shipped = shipped_rule_ids();
    let mut stated = proof_rule_ids();
    shipped.sort();
    stated.sort();
    assert_eq!(shipped, stated);
    assert!(bridge_is_in_step());
    assert_eq!(
        proof_obligations().len(),
        9,
        "the reference states exactly nine obligations"
    );
    let mut unique = stated.clone();
    unique.dedup();
    assert_eq!(unique, stated, "an obligation must not be stated twice");
}

/// C# RewriteProofBridgeTests.cs:26 `EveryObligation_IsStatedAndHonestAboutItsStatus`: each
/// obligation's id is its key, every statement is non-empty, and no theorem name is attached
/// unless the status is MachineChecked. Every shipped rule is Unproven today.
#[test]
fn rewriteproofbridgetests_everyobligation_isstatedandhonestaboutitsstatus() {
    for obligation in proof_obligations() {
        assert!(
            !obligation.statement.trim().is_empty(),
            "{} has an empty proof statement",
            obligation.rule_id
        );
        if obligation.status != ProofStatus::MachineChecked {
            assert!(
                obligation.theorem.is_none(),
                "{} names a theorem without a checked proof",
                obligation.rule_id
            );
        }
        assert_eq!(
            obligation.status,
            ProofStatus::Unproven,
            "{} must be honestly Unproven in v1",
            obligation.rule_id
        );
    }
}
