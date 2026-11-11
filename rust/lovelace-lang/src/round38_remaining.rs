//! Round 38: deferred census rows the current production answers.
//!
//! Every test is one deferred census row (or a group of rows that assert the same case). The C#
//! assertion is the contract; where the Rust surface has a different shape the test asserts the
//! SAME value or refusal through the Rust surface, and
//! docs/goal-parity-contract/round-38/remaining-porting2.md records the translation and the
//! measured refusals.

use crate::{Engine, Value};
use lovelace_symbolics::assumptions::{Assumption, AssumptionSet, Domain, SymbolPredicate};
use lovelace_symbolics::{Expr, Rat, RelOp};

fn eval(engine: &mut Engine, script: &str) -> Value {
    engine
        .eval(script)
        .unwrap_or_else(|error| panic!("script: {script} failed: {}", error.message))
}

fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    match value {
        Value::Record(type_name, fields) => fields
            .iter()
            .find(|(candidate, _)| candidate == name)
            .map(|(_, value)| value)
            .unwrap_or_else(|| panic!("record {type_name} has no field '{name}'")),
        other => panic!("not a record: {}", other.kind_name()),
    }
}

fn record_type(value: &Value) -> &str {
    match value {
        Value::Record(type_name, _) => type_name,
        other => panic!("not a record: {}", other.kind_name()),
    }
}

fn enum_member(value: &Value) -> (&str, &str) {
    match value {
        Value::Enum(type_name, member) => (type_name, member),
        other => panic!("not an enum: {} = {}", other.kind_name(), other.display()),
    }
}

fn boolean(value: &Value) -> bool {
    match value {
        Value::Boolean(flag) => *flag,
        other => panic!("not a boolean: {}", other.kind_name()),
    }
}

fn elements(value: &Value) -> &[Value] {
    match value {
        Value::Vector(items) => items,
        other => panic!("not a vector: {}", other.kind_name()),
    }
}

fn text(value: &Value) -> &str {
    match value {
        Value::Text(payload) => payload,
        other => panic!("not text: {}", other.kind_name()),
    }
}

// ===========================================================================
// Lovelace.Symbolics.Tests.AssumptionAddScalingTests (2 of 3 rows) and
// AssumptionSoundnessTests.PropertyAndRelationAtoms_Contradict
// ===========================================================================

/// The reference Mixed() pool: distinct non-contradicting atoms of every shape, with names that
/// do not sort in insertion order so an insertion never lands at the end.
fn mixed_atoms() -> Vec<Assumption> {
    vec![
        Assumption::SymbolDomain("z".to_string(), Domain::Real),
        Assumption::SymbolProperty("y".to_string(), SymbolPredicate::Positive),
        Assumption::SymbolRelation("z".to_string(), RelOp::Gt, Expr::num(2)),
        Assumption::SymbolDomain("x".to_string(), Domain::Integer),
        Assumption::SymbolProperty("x".to_string(), SymbolPredicate::Even),
        Assumption::SymbolRelation("x".to_string(), RelOp::Ge, Expr::num(3)),
        Assumption::ExpressionProperty(Expr::sym("y"), SymbolPredicate::NonZero),
        Assumption::Interval(
            Expr::sym("z"),
            Some(Expr::num(0)),
            false,
            Some(Expr::num(9)),
            true,
        ),
    ]
}

fn build_set(atoms: &[Assumption]) -> AssumptionSet {
    let mut set = AssumptionSet::empty();
    for atom in atoms {
        set = set.add(atom.clone()).unwrap_or_else(|contradiction| {
            panic!("atom {atom:?} contradicted: {}", contradiction.message)
        });
    }
    set
}

fn keys(set: &AssumptionSet) -> Vec<String> {
    set.atoms().iter().map(|atom| atom.order_key()).collect()
}

/// Add_MixedAtomsInAnyInsertionOrder_YieldsTheSameCanonicalOrder.
///
/// The stored order is ordinal by each atom's own record rendering (Rust publishes that rendering
/// as order_key, the same string the reference's generated ToString produces), for both the
/// forward and the reversed insertion order.
#[test]
fn assumption_add_mixed_atoms_in_any_insertion_order_yields_the_same_canonical_order() {
    let atoms = mixed_atoms();
    let mut expected: Vec<String> = atoms.iter().map(|atom| atom.order_key()).collect();
    expected.sort();

    assert_eq!(
        expected,
        keys(&build_set(&atoms)),
        "forward insertion order"
    );

    let mut reversed = atoms.clone();
    reversed.reverse();
    assert_eq!(
        expected,
        keys(&build_set(&reversed)),
        "reversed insertion order"
    );
}

/// Add_OrderEqualsHistoricalAppendAndResort_ForEveryInsertionOrder.
///
/// The reference replicates its pre-fix algorithm (append, then ordinal re-sort of the whole array)
/// and compares it with the incremental set for fifty insertion orders drawn from System.Random.
/// System.Random is not reproducible here, so this sweeps fifty deterministic orders with the same
/// per-order assertion: the incremental set must store exactly the sequence append-and-resort
/// produces. The quantified claim is universal, so the different sample is not a weaker statement.
#[test]
fn assumption_add_order_equals_historical_append_and_resort_for_every_insertion_order() {
    let pool = mixed_atoms();
    for trial in 0..50u64 {
        let order = shuffle(&pool, trial);
        let mut legacy: Vec<Assumption> = Vec::new();
        for atom in &order {
            if legacy.contains(atom) {
                continue;
            }
            legacy.push(atom.clone());
            legacy.sort_by_key(|left| left.order_key());
        }
        let legacy_keys: Vec<String> = legacy.iter().map(|atom| atom.order_key()).collect();
        assert_eq!(
            legacy_keys,
            keys(&build_set(&order)),
            "trial {trial}: incremental order differs from append-and-resort"
        );
    }
}

/// A deterministic Fisher-Yates shuffle: the reference's System.Random(20250825) sequence is not
/// reproducible in this crate, and the order is a sample, not part of the assertion.
fn shuffle(atoms: &[Assumption], trial: u64) -> Vec<Assumption> {
    let mut out = atoms.to_vec();
    let mut state = 20_250_825u64 ^ trial.wrapping_mul(2_654_435_761);
    for index in (1..out.len()).rev() {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let pick = ((state >> 33) as usize) % (index + 1);
        out.swap(index, pick);
    }
    out
}

// AssumptionSoundnessTests.PropertyAndRelationAtoms_Contradict is REFUSED, not mapped: measured
// in this tree, AssumptionSet::add accepts SymbolProperty(x, NonZero) after SymbolRelation(x, Eq, 0)
// (the reverse order is caught), so the row's "both insertion orders" assertion does not hold.
// docs/goal-parity-contract/round-38/remaining-porting2.md records the measurement.

// ===========================================================================
// Lovelace.Symbolics.Tests.SolveCompletenessPairingTests
//   .SystemSolve_NoSolutions_IsACompleteAnswer
// ===========================================================================

/// An inconsistent polynomial system is a PROVABLY empty solution set over the requested domain,
/// so SystemSolveResult publishes NoSolutions with complete: true.
#[test]
fn system_solve_no_solutions_is_a_complete_answer() {
    let mut engine = Engine::new();
    let record = eval(
        &mut engine,
        "x = symbol(\"x\"); y = symbol(\"y\"); solve_system_full([x + y == 1, x + y == 2], [x, y])",
    );
    assert_eq!("SystemSolveResult", record_type(&record));
    assert_eq!(
        ("SolveStatus", "NoSolutions"),
        enum_member(field(&record, "status"))
    );
    assert!(
        boolean(field(&record, "complete")),
        "an inconsistent polynomial system has a provably empty solution set"
    );
}

// ===========================================================================
// Lovelace.Symbolics.Tests.DiagnosticContractTests
//   .UnsatisfiableTransformation_CarriesADomainErrorDiagnostic
// ===========================================================================

/// A transformation whose collected conditions are refuted by the active assumptions reports a
/// DomainError diagnostic with the stable code, never prose and never an empty array.
#[test]
fn unsatisfiable_transformation_carries_a_domain_error_diagnostic() {
    let mut engine = Engine::new();
    eval(&mut engine, "x = symbol(\"x\")");
    eval(&mut engine, "assume(x == 0)");
    let record = eval(&mut engine, "simplify_full(x/x)");

    assert_eq!("TransformResult", record_type(&record));
    assert_eq!(
        ("TransformStatus", "Unsatisfiable"),
        enum_member(field(&record, "status"))
    );

    let diagnostics = elements(field(&record, "diagnostics"));
    assert!(
        !diagnostics.is_empty(),
        "an unsatisfiable transformation carries a Diagnostic, not an empty array"
    );
    assert!(
        diagnostics.iter().any(|diagnostic| {
            text(field(diagnostic, "code")) == "transform.unsatisfiable-conditions"
                && enum_member(field(diagnostic, "category")).1 == "DomainError"
        }),
        "no DomainError diagnostic with code transform.unsatisfiable-conditions"
    );
}

// ===========================================================================
// Lovelace.Symbolics.Tests.UnsatisfiablePropagationTests
//   .SimplifyFull_UnsatisfiableConditions_SerializesAsAnExplicitMarker
//   .ConditionPayload_IsStructuredForEveryReachableAtomKind
// ===========================================================================

/// SimplifyFull_UnsatisfiableConditions_SerializesAsAnExplicitMarker.
///
/// The conditions field (index 4 of TransformResult) publishes one structured
/// UnsatisfiableConditions record with no fields -- not the string "unsatisfiable".
#[test]
fn simplify_full_unsatisfiable_conditions_serialize_as_an_explicit_marker() {
    let mut engine = Engine::new();
    eval(&mut engine, "x = symbol(\"x\")");
    eval(&mut engine, "assume(x == 0)");
    let record = eval(&mut engine, "simplify_full(x/x)");

    let conditions = match &record {
        Value::Record(_, fields) => &fields[4].1,
        other => panic!("not a record: {}", other.kind_name()),
    };
    let leaves = elements(conditions);
    assert_eq!(1, leaves.len(), "one marker for one refuted branch");
    match &leaves[0] {
        Value::Record(type_name, fields) => {
            assert_eq!("UnsatisfiableConditions", type_name);
            assert!(fields.is_empty(), "the marker carries no fields");
        }
        other => panic!("the marker is a record, got {}", other.kind_name()),
    }
}

// ===========================================================================
// Lovelace.Symbolics.Tests.DebugEvalir.Lower_Evaluate_FiveTerm
// ===========================================================================

/// The five-term polynomial is lowered, optimized, serialized, deserialized and evaluated at x=2
/// to 92 -- the reference asserts the same number after the same round trip.
#[test]
fn evalir_lowers_optimizes_and_evaluates_a_five_term_polynomial() {
    let mut engine = Engine::new();
    eval(&mut engine, "x = symbol(\"x\")");
    let value = eval(
        &mut engine,
        "evalir(compile(x^5 + 2*x^4 + 3*x^3 + x^2, [x]), [2], 40)",
    );
    assert_eq!("Integer", value.kind_name());
    assert_eq!("92", engine.format(&value));
}

// ===========================================================================
// Lovelace.Symbolics.Tests.PolynomialsApiTests
//   .FromExpr_RoundTrip_DegreeCoefficientTerms
//   .Divide_X3Minus1_By_XMinus1
// ===========================================================================

fn polynomial_in_x(expression: Expr) -> Vec<Rat> {
    lovelace_symbolics::poly_coeffs(&expression, "x")
        .unwrap_or_else(|| panic!("not a polynomial in x: {}", expression))
}

fn coefficient(coefficients: &[Rat], degree: usize) -> Rat {
    coefficients.get(degree).cloned().unwrap_or_else(Rat::zero)
}

/// FromExpr_RoundTrip_DegreeCoefficientTerms: degree 2, coefficients 1/2/1 at degrees 2/1/0, zero
/// above the degree, three terms.
#[test]
fn polynomial_coefficients_round_trip_degree_terms_and_coefficients() {
    let x = Expr::sym("x");
    let p = Expr::add(
        Expr::add(
            Expr::pow(x.clone(), Expr::num(2)),
            Expr::mul(Expr::num(2), x),
        ),
        Expr::num(1),
    );
    let coefficients = polynomial_in_x(p);

    assert_eq!(2, coefficients.len() - 1, "degree");
    assert_eq!(Rat::one(), coefficient(&coefficients, 2));
    assert_eq!(Rat::from_i64(2), coefficient(&coefficients, 1));
    assert_eq!(Rat::one(), coefficient(&coefficients, 0));
    assert_eq!(Rat::zero(), coefficient(&coefficients, 3));
    assert_eq!(
        3,
        coefficients.iter().filter(|value| !value.is_zero()).count(),
        "term count"
    );
}

/// Divide_X3Minus1_By_XMinus1: the quotient is x^2 + x + 1 and the remainder is zero. The Rust
/// polynomial division publishes the quotient; the zero remainder is asserted by reconstructing
/// num = quotient * divisor exactly, which is the same statement as r == 0.
#[test]
fn polynomial_division_of_x_cubed_minus_one_by_x_minus_one() {
    let x = Expr::sym("x");
    let numerator = polynomial_in_x(Expr::add(Expr::pow(x.clone(), Expr::num(3)), Expr::num(-1)));
    let denominator = polynomial_in_x(Expr::add(x, Expr::num(-1)));

    let quotient =
        lovelace_symbolics::poly_div(&numerator, &denominator).expect("x - 1 divides x^3 - 1");
    assert_eq!(
        vec![Rat::one(), Rat::one(), Rat::one()],
        quotient,
        "x^2 + x + 1"
    );

    // remainder: num - quotient*denominator is the zero polynomial
    let mut product = vec![Rat::zero(); quotient.len() + denominator.len() - 1];
    for (left, a) in quotient.iter().enumerate() {
        for (right, b) in denominator.iter().enumerate() {
            product[left + right] = product[left + right].add(&a.mul(b));
        }
    }
    assert_eq!(numerator, product, "the division must leave no remainder");
}

// ===========================================================================
// Lovelace.Symbolics.Tests.UsageDocumentationTests
//   .DocumentedExample_MatchesEngine#75
// ===========================================================================

/// The usage guide documents linsolve([[x, 1], [0, y]], [0, 1]) rendering exactly
/// [-x/x/(x*y), x/(x*y)]; the reference row asserts the documented typed rendering, and the Rust
/// engine now publishes the same bytes (the round-26 canonical-shape refusal no longer measures).
#[test]
fn documented_linsolve_example_matches_the_engine() {
    let mut engine = Engine::new();
    eval(&mut engine, "x = symbol(\"x\"); y = symbol(\"y\")");
    let value = eval(&mut engine, "linsolve([[x, 1], [0, y]], [0, 1])");
    assert_eq!(
        "[-x/x/(x*y), x/(x*y)] (Vector)",
        engine.format_typed(&value)
    );
}
