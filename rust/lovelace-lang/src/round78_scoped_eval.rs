//! Round 78 — assumption-scoped numeric evaluation.
//!
//! The four deferred `Lovelace.Symbolics.Tests.PiecewiseAssumptionTests` rows in the
//! `assumption-scoped numeric evaluation (EvaluateToNum with caller assumptions)` block:
//!
//! * `MirroredSpelling_IsAlsoDecided#0`
//! * `ProvenGuard_TakesTheBranch#0`
//! * `RefutedGuard_TakesTheOtherwiseBranch#0`
//! * `WithoutAssumptions_TheGuardIsStillUndecidable#0`
//!
//! Each test drives the reference's `Evaluation.EvaluateToNum` over the same `piecewise` shape
//! with the same assumption scope, through the additive `numeric::evaluate_scoped` entry point.
//! The value and refusal assertions are the C# assertions; the condition assertions pin the new
//! relied-on subset, and `a_guard_decided_without_assumptions_reports_none` is the "no assumptions
//! needed" branch the API must also get right.

use lovelace_symbolics::assumptions::{Assumption, AssumptionSet};
use lovelace_symbolics::{Expr, RelOp};

use crate::numeric::{Num, ScopedEvaluation, evaluate, evaluate_scoped};

/// `PiecewiseAssumptionTests.cs:21 PiecewiseAbs`: `piecewise(1 if x >= 0, 2)`.
fn piecewise_abs() -> Expr {
    Expr::piecewise(
        vec![(
            Expr::relation(RelOp::Ge, Expr::sym("x"), Expr::num(0)),
            Expr::num(1),
        )],
        Expr::num(2),
    )
}

fn scope(atom: Assumption) -> AssumptionSet {
    AssumptionSet::empty()
        .add(atom)
        .expect("a single atom is consistent")
}

fn relation(symbol: &str, op: RelOp, bound: i64) -> Assumption {
    Assumption::SymbolRelation(symbol.to_string(), op, Expr::num(bound))
}

fn integer(value: &Num) -> i64 {
    match value {
        Num::Exact(value) => value.to_i64().expect("an exact integer"),
        other => panic!("expected an exact integer, got {other:?}"),
    }
}

fn assert_conditions(result: &ScopedEvaluation, expected: &AssumptionSet) {
    assert_eq!(
        &result.conditions,
        expected,
        "conditions were {:?}",
        result.conditions.atoms()
    );
}

/// `PiecewiseAssumptionTests.cs:26 RefutedGuard_TakesTheOtherwiseBranch`: under `x < 0` the
/// guard `x >= 0` is refuted and the otherwise value 2 is the numeric answer.
#[test]
fn refuted_guard_takes_the_otherwise_branch() {
    let pw = piecewise_abs();
    let assumptions = scope(relation("x", RelOp::Lt, 0));

    let result = evaluate_scoped(&pw, 20, &assumptions).expect("the refuted guard takes otherwise");

    assert_eq!(integer(&result.value), 2);
    assert_conditions(&result, &assumptions);
}

/// `PiecewiseAssumptionTests.cs:41 ProvenGuard_TakesTheBranch`: under `x > 0` the guard
/// `x >= 0` is proven and the branch value 1 is the numeric answer. The reported condition is
/// the assumed atom `x > 0`, not the weaker guard `x >= 0`: it is the fact the decision used.
#[test]
fn proven_guard_takes_the_branch() {
    let pw = piecewise_abs();
    let assumptions = scope(relation("x", RelOp::Gt, 0));

    let result = evaluate_scoped(&pw, 20, &assumptions).expect("the proven guard takes its branch");

    assert_eq!(integer(&result.value), 1);
    assert_conditions(&result, &assumptions);
}

/// `PiecewiseAssumptionTests.cs:56 MirroredSpelling_IsAlsoDecided`: the guard is written
/// constant-first, `0 < x`, and the same `x > 0` assumption decides it (the lattice stores
/// symbol-op-constant only, so the mirrored relation is flipped into it).
#[test]
fn mirrored_spelling_is_also_decided() {
    let pw = Expr::piecewise(
        vec![(
            Expr::relation(RelOp::Lt, Expr::num(0), Expr::sym("x")),
            Expr::num(1),
        )],
        Expr::num(2),
    );
    let assumptions = scope(relation("x", RelOp::Gt, 0));

    let result = evaluate_scoped(&pw, 20, &assumptions).expect("the mirrored guard is decided");

    assert_eq!(integer(&result.value), 1);
    assert_conditions(&result, &assumptions);
}

/// `PiecewiseAssumptionTests.cs:74 WithoutAssumptions_TheGuardIsStillUndecidable`: with nothing
/// known about `x` the guard is genuinely undecidable, so the evaluation refuses instead of
/// silently picking a branch. The value-only entry point keeps its pre-existing refusal, and the
/// scoped entry point with an empty scope refuses identically.
#[test]
fn without_assumptions_the_guard_is_still_undecidable() {
    let pw = piecewise_abs();

    let refused = evaluate(&pw, 20).expect_err("no binding and no assumption must refuse");
    assert_eq!(
        refused.message,
        "Piecewise guard cannot be decided numerically."
    );

    let refused = evaluate_scoped(&pw, 20, &AssumptionSet::empty())
        .expect_err("an empty scope must not guess a branch");
    assert_eq!(
        refused.message,
        "Piecewise guard cannot be decided numerically."
    );
}

/// The "no assumptions needed" branch of the same API: a guard decided numerically succeeds
/// under an empty scope and reports no conditions, and an atom unrelated to the guard is not
/// reported as relied on.
#[test]
fn a_guard_decided_without_assumptions_reports_none() {
    let pw = Expr::piecewise(
        vec![(
            Expr::relation(RelOp::Ge, Expr::num(1), Expr::num(0)),
            Expr::num(3),
        )],
        Expr::num(4),
    );
    let assumptions = scope(relation("unrelated", RelOp::Gt, 0));

    let result = evaluate_scoped(&pw, 20, &assumptions).expect("a numeric guard is decided");

    assert_eq!(integer(&result.value), 3);
    assert!(result.conditions.atoms().is_empty());
}
