//! Round-35 ports of the assumption/tri-state family: the language surface
//! (assume / assume_* / assume_clear / assumptions / simplify / simplify_full / inspect) and the
//! kernel's tri-state answers, contradiction detection and published conditions.
//!
//! Every test repeats the C# assertion it is mapped to and the oracle-measured rendering; the row
//! ids are listed in docs/goal-parity-contract/round-35/assumptions-realliteral.md.

use crate::{Engine, LangError, Value};

fn eval(script: &str) -> Result<Value, LangError> {
    Engine::new().eval(script)
}

fn display(script: &str) -> String {
    eval(script)
        .unwrap_or_else(|error| panic!("'{script}' failed: {}", error.message))
        .display()
}

fn refusal(script: &str) -> LangError {
    match eval(script) {
        Err(error) => error,
        Ok(value) => panic!("'{script}' answered {}", value.display()),
    }
}

fn record_field<'a>(value: &'a Value, name: &str) -> &'a Value {
    match value {
        Value::Record(type_name, fields) => fields
            .iter()
            .find(|(field, _)| field.as_str() == name)
            .map(|(_, value)| value)
            .unwrap_or_else(|| panic!("record {type_name} has no field '{name}'")),
        other => panic!("expected a record, got {}", other.kind_name()),
    }
}

fn array_items(value: &Value) -> &[Value] {
    match value {
        Value::Vector(items) => items,
        other => panic!("expected a vector, got {}", other.kind_name()),
    }
}

// ---------------------------------------------------------------------------
// The published set: display and structured atoms
// ---------------------------------------------------------------------------

/// C# AssumptionSoundnessTests.DomainAtoms_MergeToTheNarrowest + the assumptions() record: a
/// domain atom renders as membership and projects to a DomainCondition leaf.
#[test]
fn assumptions_publish_domain_property_and_relation_atoms() {
    let value = eval(
        "x = symbol(\"x\"); assume_real(x); assume_positive(x); assume(x > 5); assumptions()",
    )
    .expect("assumptions");
    assert_eq!(
        record_field(&value, "display").display(),
        "x in Real; x is Positive; x > 5"
    );
    let atoms = array_items(record_field(&value, "assumptions"));
    assert_eq!(atoms.len(), 3);
    assert_eq!(
        atoms[0].display(),
        "DomainCondition(variable: x, domain: real)"
    );
    assert_eq!(atoms[1].display(), "x > 0");
    assert_eq!(atoms[2].display(), "x > 5");
}

/// The reference's SymbolWithDomain: symbol(name, domain) ASSUMES the domain for the session.
#[test]
fn symbol_with_a_domain_records_a_domain_assumption() {
    let value = eval("x = symbol(\"x\", real); assumptions()").expect("assumptions");
    assert_eq!(record_field(&value, "display").display(), "x in Real");
    let atoms = array_items(record_field(&value, "assumptions"));
    assert_eq!(atoms.len(), 1);
    assert_eq!(
        atoms[0].display(),
        "DomainCondition(variable: x, domain: real)"
    );
}

/// C# AssumptionAddScalingTests.Add_OrderEqualsHistoricalAppendAndResort_ForEveryInsertionOrder:
/// the stored order is the canonical rendering order, not the insertion order.
#[test]
fn assumption_atoms_are_published_in_canonical_order_regardless_of_insertion() {
    // The two Gt relations tie on the reference's ordering key (an expression contributes its
    // node KIND, not its value), so their insertion order decides between them; the operator and
    // atom kind decide everything else.
    assert_eq!(
        display("x = symbol(\"x\"); assume(x != 2); assume(x > 5); assume_real(x); assume(x > 0); assumptions()"),
        "AssumptionSet(display: x in Real; x > 5; x > 0; x != 2, assumptions: [DomainCondition(variable: x, domain: real), x > 5, x > 0, x != 2])"
    );
}

/// C# AssumptionAddScalingTests.Add_SameAtomTwiceFromDifferentContexts_IsStoredOnce.
#[test]
fn the_same_atom_added_twice_is_stored_once() {
    assert_eq!(
        display("x = symbol(\"x\"); assume(x > 5); assume(x > 5); assumptions()"),
        "AssumptionSet(display: x > 5, assumptions: [x > 5])"
    );
}

// ---------------------------------------------------------------------------
// Contradictions
// ---------------------------------------------------------------------------

/// C# AssumptionSoundnessTests.PropertyAndRelationAtoms_Contradict and the DxSemanticClosure
/// refutation rows: the refusal is the reference sentence and the UnsatisfiableAssumptions code.
#[test]
fn contradictory_relations_are_refused_with_the_documented_message() {
    let error = refusal("x = symbol(\"x\"); assume(x > 0); assume(x < 0)");
    assert_eq!(
        error.message,
        "Assumption x < 0 contradicts the existing assumptions (its negation x >= 0 is provable)."
    );
    assert_eq!(error.code, "UnsatisfiableAssumptions");
    assert_eq!(error.category, "DomainError");
}

/// C# AssumptionSoundnessTests.ContradictionDetection_IsNeitherMissedNorInvented, engine level:
/// compatible relations are accepted.
#[test]
fn compatible_relations_do_not_throw() {
    assert_eq!(
        display("x = symbol(\"x\"); assume(x > 0); assume(x < 10); assume(x != 5); assumptions()"),
        "AssumptionSet(display: x > 0; x < 10; x != 5, assumptions: [x > 0, x < 10, x != 5])"
    );
}

/// The property/relation collision is caught in BOTH insertion orders.
#[test]
fn property_and_relation_atoms_contradict_in_both_orders() {
    let forward = refusal("x = symbol(\"x\"); assume_positive(x); assume(x <= 0)");
    assert_eq!(
        forward.message,
        "Assumption x <= 0 contradicts the existing assumptions (its negation x > 0 is provable)."
    );
    let reverse = refusal("x = symbol(\"x\"); assume(x <= 0); assume_positive(x)");
    assert_eq!(
        reverse.message,
        "Assumption x is Positive contradicts the existing assumptions (its negation x is NonPositive is provable)."
    );
}

/// C# IntegerIntervalContradictionTests: an integer domain with fractional bounds is empty even
/// though no pair of atoms contradicts pairwise.
#[test]
fn integer_interval_emptiness_is_refused() {
    let error = refusal("x = symbol(\"x\", integer); assume(x > 1/2); assume(x < 1)");
    assert_eq!(
        error.message,
        "Assumption x < 1 leaves no integer satisfying the existing assumptions."
    );
    assert_eq!(error.code, "UnsatisfiableAssumptions");
    // over the reals the same pair is satisfiable
    assert_eq!(
        display("x = symbol(\"x\", real); assume(x > 1/2); assume(x < 1); assumptions().display"),
        "x in Real; x > 1/2; x < 1"
    );
}

// ---------------------------------------------------------------------------
// assume_clear
// ---------------------------------------------------------------------------

/// C# ContextWithAssumptions / assume_clear: the session's set is emptied, and simplification
/// stops being licensed by it.
#[test]
fn assume_clear_empties_the_session() {
    assert_eq!(
        display("x = symbol(\"x\"); assume(x > 0); assume_clear(); assumptions()"),
        "AssumptionSet(display: , assumptions: [])"
    );
    assert_eq!(
        display("x = symbol(\"x\"); assume(x > 0); assume_clear(); simplify(sqrt(x^2))"),
        "sqrt(x^2)"
    );
    assert_eq!(display("x = symbol(\"x\"); assume_clear()"), "assumptions cleared");
}

// ---------------------------------------------------------------------------
// Domain-conditional rewrites
// ---------------------------------------------------------------------------

/// C# AcceptanceTests.Simplify_SqrtSquare_UnderAssumptions and AbsSymbolicRoundTripTests.
#[test]
fn simplify_sqrt_square_under_real_is_abs_x() {
    assert_eq!(display("x = symbol(\"x\", real); simplify(sqrt(x^2))"), "abs(x)");
    // a provably nonnegative x keeps the unconditional form
    assert_eq!(display("x = symbol(\"x\"); assume(x > 5); simplify(sqrt(x^2))"), "x");
    assert_eq!(display("x = symbol(\"x\"); assume(x >= 0); simplify(sqrt(x^2))"), "x");
    // without a premise the identity is not licensed
    assert_eq!(display("x = symbol(\"x\"); simplify(sqrt(x^2))"), "sqrt(x^2)");
}

/// C# KernelHardeningTests.AbsSquare_Simplified_UnderReal.
#[test]
fn simplify_abs_square_under_real_is_x_squared() {
    assert_eq!(display("x = symbol(\"x\", real); simplify(abs(x)^2)"), "x^2");
    assert_eq!(display("x = symbol(\"x\"); simplify(abs(x)^2)"), "abs(x)^2");
}

/// C# KernelHardeningTests.LogExp_Simplified_UnderReal.
#[test]
fn simplify_log_exp_under_real_is_the_identity() {
    assert_eq!(display("x = symbol(\"x\", real); simplify(log(exp(x)))"), "x");
    assert_eq!(display("x = symbol(\"x\"); simplify(log(exp(x)))"), "log(exp(x))");
    // an integer is a real (the domain lattice only ever narrows), so the rule still fires
    assert_eq!(display("x = symbol(\"x\"); assume_integer(x); simplify(log(exp(x)))"), "x");
    // a merely rational-bound symbol with no real domain stays unresolved
    assert_eq!(
        display("x = symbol(\"x\"); assume(x > 0); simplify(log(exp(x)))"),
        "log(exp(x))"
    );
}

// ---------------------------------------------------------------------------
// Tri-state relation answers
// ---------------------------------------------------------------------------

/// C# DxSemanticClosureTests.AskRelation_ProvesWeakerBounds / AskRelation_RefutesContradictoryBounds
/// and KernelHardeningTests.Or_ReturnsFalse_WhenAllDisjunctsFalse: simplify() answers True only
/// when the set proves the relation and False only when it proves the negation.
#[test]
fn simplify_relation_answers_true_false_or_stays_unevaluated() {
    assert_eq!(display("x = symbol(\"x\"); assume(x > 5); simplify(x > 0)"), "True");
    assert_eq!(display("x = symbol(\"x\"); assume(x > 5); simplify(x < 5)"), "False");
    // x > 5 says nothing about x < 7
    assert_eq!(display("x = symbol(\"x\"); assume(x > 5); simplify(x < 7)"), "x < 7");
    // a property atom answers through the same lattice
    assert_eq!(display("x = symbol(\"x\"); assume_positive(x); simplify(x > 0)"), "True");
    // and an empty set answers nothing
    assert_eq!(display("x = symbol(\"x\"); simplify(x > 0)"), "x > 0");
}

// ---------------------------------------------------------------------------
// Unsatisfiable transform
// ---------------------------------------------------------------------------

/// C# DxSemanticClosureTests.Unsatisfiable_Sentinel_IsSticky and
/// DiagnosticContractTests.UnsatisfiableTransformation_CarriesADomainErrorDiagnostic.
#[test]
fn simplify_full_reports_unsatisfiable_conditions_as_a_marker() {
    let value = eval("x = symbol(\"x\"); assume(x == 0); simplify_full(x/x)").expect("transform");
    assert_eq!(record_field(&value, "status").display(), "Unsatisfiable");
    assert_eq!(record_field(&value, "expression").display(), "1");
    let conditions = array_items(record_field(&value, "conditions"));
    assert_eq!(conditions.len(), 1);
    assert_eq!(conditions[0].display(), "UnsatisfiableConditions()");
    let diagnostics = array_items(record_field(&value, "diagnostics"));
    assert_eq!(diagnostics.len(), 1);
    let diagnostic = &diagnostics[0];
    assert_eq!(record_field(diagnostic, "code").display(), "transform.unsatisfiable-conditions");
    assert_eq!(record_field(diagnostic, "category").display(), "DomainError");
    assert_eq!(record_field(diagnostic, "recoverable").display(), "False");
    assert_eq!(
        record_field(diagnostic, "message").display(),
        "the applied rules require contradictory conditions, so this branch has no model."
    );
    // the same rewrite without the contradictory assumption stays Satisfied
    assert_eq!(
        record_field(
            &eval("x = symbol(\"x\"); simplify_full(x/x)").expect("transform"),
            "status"
        )
        .display(),
        "Satisfied"
    );
}

// ---------------------------------------------------------------------------
// inspect
// ---------------------------------------------------------------------------

/// C# DxStructuredResultsTests.Inspect_ReportsTheAssumptionsThatConstrainTheFreeSymbols.
#[test]
fn inspect_reports_only_the_assumptions_that_constrain_the_free_symbols() {
    let value = eval(
        "x = symbol(\"x\"); y = symbol(\"y\"); assume(x > 0); assume(y > 5); assume_real(x); inspect(x + 1)",
    )
    .expect("inspection");
    let assumptions = array_items(record_field(&value, "assumptions"));
    assert_eq!(assumptions.len(), 2);
    assert_eq!(assumptions[0].display(), "DomainCondition(variable: x, domain: real)");
    assert_eq!(assumptions[1].display(), "x > 0");
    // an expression with no free symbols carries no conditions
    let constant = eval("x = symbol(\"x\"); assume(x > 0); inspect(2 + 2)").expect("inspection");
    assert!(array_items(record_field(&constant, "assumptions")).is_empty());
}

// ---------------------------------------------------------------------------
// Isolation between engines
// ---------------------------------------------------------------------------

/// C# AssumptionScopeIsolationTests.TwoConcurrentScopes_AreIsolated, engine level: two engines
/// that load the same plugin each keep their own set, and neither answers the other's query.
#[test]
fn engines_keep_their_own_assumption_sets() {
    let mut first = Engine::new();
    let mut second = Engine::new();
    first.eval("x = symbol(\"x\", real)").expect("symbol");

    assert_eq!(
        record_field(&first.eval("assumptions()").expect("set"), "display").display(),
        "x in Real"
    );
    assert_eq!(
        record_field(&second.eval("assumptions()").expect("set"), "display").display(),
        ""
    );
    // the second engine has no real domain, so the domain-conditional rewrite does not fire
    assert_eq!(
        second
            .eval("x = symbol(\"x\"); simplify(sqrt(x^2))")
            .expect("simplify")
            .display(),
        "sqrt(x^2)"
    );
    assert_eq!(
        first.eval("simplify(sqrt(x^2))").expect("simplify").display(),
        "abs(x)"
    );
    // and the first engine's set is untouched by the second engine's work
    assert_eq!(
        record_field(&first.eval("assumptions()").expect("set"), "display").display(),
        "x in Real"
    );
}

// ---------------------------------------------------------------------------
// Scaling
// ---------------------------------------------------------------------------

/// C# AssumptionAddScalingTests.Add_Scaling_800AtomsStaysUnder250ms_AndKeepsEveryAtomInOrder:
/// nothing is dropped, the stored sequence is non-decreasing in the atom's ordering key, and 800
/// insertions stay inside the reference's 250 ms tripwire.
#[test]
fn eight_hundred_atoms_stay_ordered_and_fast() {
    use lovelace_symbolics::assumptions::{Assumption, AssumptionSet};
    use lovelace_symbolics::Expr as Sym;
    use lovelace_symbolics::RelOp;

    let start = std::time::Instant::now();
    let mut set = AssumptionSet::empty();
    for value in (0..800i64).rev() {
        set = set
            .add_owned(Assumption::SymbolRelation(
                "x".to_string(),
                RelOp::Gt,
                Sym::num(value),
            ))
            .expect("consistent");
    }
    let elapsed = start.elapsed();
    assert_eq!(set.atoms().len(), 800);
    let keys: Vec<String> = set.atoms().iter().map(Assumption::order_key).collect();
    for pair in keys.windows(2) {
        assert!(pair[0] <= pair[1], "order broken: {} > {}", pair[0], pair[1]);
    }
    // every atom is present, and the order is the insertion order among the tied keys
    let rendered: Vec<String> = set.atoms().iter().map(Assumption::describe).collect();
    assert_eq!(rendered.first().map(String::as_str), Some("x > 799"));
    assert_eq!(rendered.last().map(String::as_str), Some("x > 0"));
    // The reference asserts 250 ms for this workload in a Release build and marks the case
    // Category=Timing so an instrumented CI run excludes it. A debug build is 20-40x slower
    // (unoptimized arithmetic and allocation), so the tripwire is only meaningful there; the
    // structural assertions above are the machine-independent half and run in both.
    println!("800 atoms: {elapsed:?} (ceiling {} ms)", if cfg!(debug_assertions) { 2000 } else { 250 });
    let ceiling = if cfg!(debug_assertions) { 2_000 } else { 250 };
    assert!(elapsed.as_millis() < ceiling, "800 atoms took {elapsed:?}");
}
