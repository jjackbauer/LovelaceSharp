//! Round 66: the deferred census rows this tree now answers through its own symbolics surface.
//!
//! Every test is one census case. The C# assertion (cited per test) is the contract; the comment
//! records the translation rule where the Rust surface has no reference identity (Assert.Same
//! becomes equality of canonical forms because Expr is a value, not an interned object). Nothing
//! here weakens an assertion: where the C# row asserts a reference, the Rust test asserts the
//! canonical-form identity that the reference's interning is an implementation of.

use crate::{
    canonical_parse, square_free_univariate, Expr, Rat,
};
use crate::symbolic_matrix::SymbolicMatrix;

/// SmokeTests.cs:245 Interning_EqualStructureSharesReference - Add(Power(x,2), x) and
/// Add(x, Power(x,2)) are the same expression. The C# row asserts Assert.Same because its
/// Exprs.Add canonicalises into one interned node; Rust Expr is a value, so the same claim is
/// a == b: canonicalisation is observable as structural equality.
#[test]
fn acceptance_interned_equal_structure_is_the_same_value() {
    let x = Expr::sym("x");
    let a = Expr::add(Expr::pow(x.clone(), Expr::num(2)), x.clone());
    let b = Expr::add(x.clone(), Expr::pow(x, Expr::num(2)));
    assert_eq!(a, b, "equal structure must be one canonical expression");
}

/// SmokeTests.cs:224 PrintParse_RoundTrips - canonical print then canonical parse returns the
/// expression. The C# row asserts Assert.Equal and Assert.Same; Rust has no interned node, so the
/// claim is the canonical-form identity the interning implements: parsing the canonical text
/// yields an expression whose canonical text is the one that was parsed.
#[test]
fn acceptance_canonical_print_parse_round_trips() {
    let x = Expr::sym("x");
    let e = Expr::div(
        Expr::add(Expr::pow(x.clone(), Expr::num(2)), Expr::num(1)),
        Expr::sub(x, Expr::num(1)),
    );
    let text = e.canonical();
    let back = canonical_parse(&text).unwrap_or_else(|error| panic!("canonical parse failed: {error:?}"));
    assert_eq!(back.canonical(), text, "canonical text must round-trip");
    let again = canonical_parse(&back.canonical()).expect("idempotent canonical parse");
    assert_eq!(again.canonical(), text, "canonical parse must be idempotent");
}

/// UsageExamples.cs:27 Snippet_16_2_EqualityAndHashConsing - the published snippet: the two
/// construction orders are the same expression.
#[test]
fn usage_snippet_16_2_equality_and_hash_consing() {
    let x = Expr::sym("x");
    let a = Expr::add(Expr::pow(x.clone(), Expr::num(2)), x.clone());
    let b = Expr::add(x.clone(), Expr::pow(x, Expr::num(2)));
    assert_eq!(a, b, "canonical construction: equal structure is equal");
}

/// UsageExamples.cs:50 Snippet_16_4_CanonicalTextRoundTrip - x^2 + 2*x + 1 prints canonically and
/// parses back to the same canonical form (the C# row's Assert.Same, translated the same way).
#[test]
fn usage_snippet_16_4_canonical_text_round_trip() {
    let x = Expr::sym("x");
    let f = Expr::add(
        Expr::add(
            Expr::pow(x.clone(), Expr::num(2)),
            Expr::mul(Expr::num(2), x.clone()),
        ),
        Expr::num(1),
    );
    let text = f.canonical();
    let back = canonical_parse(&text).unwrap_or_else(|error| panic!("canonical parse failed: {error:?}"));
    assert_eq!(back.canonical(), text, "canonical text must round-trip");
}

/// UsageExamples.cs:96 Snippet_16_8_ExactnessDiscipline - the published four claims: an exact
/// rational, a radical of an exact base, a closed form, and the Real leaf that is the carrier of
/// approximation. Expr::is_exact is the flag the snippet reads.
#[test]
fn usage_snippet_16_8_exactness_discipline() {
    let third = Rat::new(lovelace_int::Int::from_i64(1), lovelace_nat::Nat::from_u64(3));
    let half = Rat::new(lovelace_int::Int::from_i64(1), lovelace_nat::Nat::from_u64(2));
    assert!(Expr::Num(third).is_exact(), "an exact rational is exact");
    assert!(
        Expr::pow(Expr::num(2), Expr::Num(half)).is_exact(),
        "sqrt(2) is a radical of an exact base, so it is exact"
    );
    assert!(
        Expr::Fun("sin".to_string(), vec![Expr::sym("x")]).is_exact(),
        "an exact closed form is exact"
    );
    assert!(
        !Expr::real("1.5").is_exact(),
        "the Real leaf is the carrier of approximation"
    );
}

/// MatrixAdvanceTests.cs:57 InverseWithConditions_Singular_ReturnsNullMatrix_And_Note: the
/// singular matrix publishes no inverse, the reference's note, and no condition to carry.
#[test]
fn matrix_inverse_with_conditions_singular_reports_no_matrix_and_the_note() {
    let a = SymbolicMatrix::from_rows(vec![
        vec![Expr::num(1), Expr::num(2)],
        vec![Expr::num(2), Expr::num(4)],
    ])
    .expect("2x2 matrix");
    let result = a.inverse_with_conditions();
    assert!(result.matrix.is_none(), "a singular matrix has no inverse");
    assert_eq!(result.note.as_deref(), Some("matrix is singular"));
    assert!(
        result.conditions.atoms().is_empty(),
        "a refused inverse carries no condition"
    );
}

/// PolynomialsApiTests.cs:70 SquareFree_X2Times_XMinus1: x^3 - x^2 decomposes into (x - 1) once
/// and x twice, ascending coefficient order.
#[test]
fn polynomials_square_free_of_x_squared_times_x_minus_one() {
    let p = vec![
        Rat::from_i64(0),
        Rat::from_i64(0),
        Rat::from_i64(-1),
        Rat::from_i64(1),
    ];
    let factors = square_free_univariate(&p);
    assert_eq!(factors.len(), 2, "two square-free factors: {factors:?}");
    assert_eq!(
        factors[0],
        (vec![Rat::from_i64(-1), Rat::from_i64(1)], 1),
        "(x - 1) once"
    );
    assert_eq!(
        factors[1],
        (vec![Rat::from_i64(0), Rat::from_i64(1)], 2),
        "x twice"
    );
}

/// UnsatisfiablePropagationTests.cs:46 Transform_RequirementConsistentWithAssumptions_KeepsItsConditions
/// - x/x may only be cancelled when x != 0; a caller assumption set that does not refute it keeps
/// the requirement, and the required atom is the NonZero property of x.
#[test]
fn transform_requirement_consistent_with_assumptions_keeps_its_conditions() {
    use crate::assumptions::{Assumption, AssumptionSet, SymbolPredicate};
    use crate::rewrite_registry::{transform, TransformOptions};
    use crate::RelOp;

    let x = Expr::sym("x");
    let assumptions = AssumptionSet::empty()
        .add(Assumption::SymbolRelation(
            "x".to_string(),
            RelOp::Gt,
            Expr::num(5),
        ))
        .expect("consistent assumption set");
    let result = transform(
        &Expr::div(x.clone(), x.clone()),
        &assumptions,
        TransformOptions::default(),
    );
    assert!(!result.conditions.is_unsatisfiable());
    assert!(
        result
            .conditions
            .atoms()
            .contains(&Assumption::SymbolProperty("x".to_string(), SymbolPredicate::NonZero)),
        "the kept requirement must be x != 0, got {:?}",
        result.conditions.atoms()
    );
}

/// UnsatisfiablePropagationTests.cs:31 Transform_RequirementRefutedByAssumptions_IsUnsatisfiable -
/// the same rewrite under x = 0: the required x != 0 is refuted, so the branch is unsatisfiable.
#[test]
fn transform_requirement_refuted_by_assumptions_is_unsatisfiable() {
    use crate::assumptions::{Assumption, AssumptionSet};
    use crate::rewrite_registry::{transform, TransformOptions};
    use crate::RelOp;

    let x = Expr::sym("x");
    let assumptions = AssumptionSet::empty()
        .add(Assumption::SymbolRelation(
            "x".to_string(),
            RelOp::Eq,
            Expr::num(0),
        ))
        .expect("consistent assumption set");
    let result = transform(
        &Expr::div(x.clone(), x.clone()),
        &assumptions,
        TransformOptions::default(),
    );
    assert!(
        result.conditions.is_unsatisfiable(),
        "x/x under x == 0 must be an unsatisfiable branch, got {:?}",
        result.conditions.atoms()
    );
}

/// DxSemanticClosureTests.cs:146 Transform_BudgetExceeded_IsReported: a zero step budget is
/// reported as an exceeded budget rather than silently producing an unchanged result.
#[test]
fn dx_semantic_transform_budget_exceeded_is_reported() {
    use crate::assumptions::AssumptionSet;
    use crate::rewrite_registry::{transform, TransformOptions};

    let x = Expr::sym("x");
    let options = TransformOptions {
        trace: false,
        max_steps: 0,
        diagnose: false,
    };
    let result = transform(
        &Expr::div(x.clone(), x.clone()),
        &AssumptionSet::empty(),
        options,
    );
    assert!(result.budget_exceeded, "a zero budget must be reported");
}

/// DxSemanticClosureTests.cs:134 Transform_TraceIsOptIn_ButConditionsAlwaysSurvive: with tracing
/// off the steps are empty, the semantic condition (x != 0) still comes through, and the budget,
/// which was never exhausted, is not reported.
#[test]
fn dx_semantic_transform_trace_is_opt_in_but_conditions_always_survive() {
    use crate::assumptions::{Assumption, AssumptionSet, SymbolPredicate, Tristate};
    use crate::rewrite_registry::{transform, TransformOptions};

    let x = Expr::sym("x");
    let result = transform(
        &Expr::div(x.clone(), x.clone()),
        &AssumptionSet::empty(),
        TransformOptions::default(),
    );
    assert!(result.steps.is_empty(), "the trace is opt-in");
    assert_eq!(
        result.conditions.ask(&Assumption::SymbolProperty(
            "x".to_string(),
            SymbolPredicate::NonZero
        )),
        Tristate::True,
        "the NonZero requirement survives without a trace"
    );
    assert!(!result.budget_exceeded);
}
