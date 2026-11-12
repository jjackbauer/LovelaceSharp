//! Round-78 StructuralEquality rows.
//!
//! The four census rows are `Lovelace.Suite.Tests.StructuralEqualityTests`:
//! `DifferenceInANestedField_IsNamedByPath#0`, `DifferentKinds_AreReportedByKind#0`,
//! `DifferentShape_IsReportedAsShape#0` and
//! `SymbolicEquality_IsByCanonicalForm_NotPrettyText#0`. They exercise the reference
//! `Lovelace.Suite.StructuralEquality` surface, whose Rust counterpart is
//! [`crate::structural_equality::StructuralEquality`].
//!
//! The row tests mirror the C# bodies. The supporting tests pin the semantics the C# source
//! documents: kind-first comparison, record/array path reporting, and symbolic equality by
//! canonical form (commutative and unordered operands are equal; mathematically equal but
//! structurally different forms are not; assumptions never enter the comparison).

use lovelace_array::NdArray;
use lovelace_int::Int;
use lovelace_nat::Nat;
use lovelace_real::Real;
use lovelace_symbolics::Expr;
use lovelace_symbolics::assumptions::{Assumption, AssumptionSet, SymbolPredicate, Tristate};

use crate::Value;
use crate::structural_equality::StructuralEquality;

fn natural(value: u64) -> Value {
    Value::Natural(Nat::from_u64(value))
}

fn integer(value: i64) -> Value {
    Value::Integer(Int::from_i64(value))
}

fn symbolic(expression: Expr) -> Value {
    Value::Symbolic(expression)
}

/// The reference `StructuralEqualityTests.Sample`: a `SolveResult` record whose second
/// solution carries the varying multiplicity. `status` is Text (the payload was a string),
/// `domain` is a Domain value and the `value` fields are symbolic integer expressions, exactly
/// as `PayloadMap.Wrap` maps the C# payload.
fn sample(count: i64) -> Value {
    Value::Record(
        "SolveResult".to_string(),
        vec![
            ("status".to_string(), Value::Text("Solved".to_string())),
            ("domain".to_string(), Value::Domain("complex".to_string())),
            (
                "solutions".to_string(),
                Value::Vector(vec![
                    Value::Record(
                        "Solution".to_string(),
                        vec![
                            ("value".to_string(), symbolic(Expr::num(-2))),
                            ("multiplicity".to_string(), integer(1)),
                        ],
                    ),
                    Value::Record(
                        "Solution".to_string(),
                        vec![
                            ("value".to_string(), symbolic(Expr::num(2))),
                            ("multiplicity".to_string(), integer(count)),
                        ],
                    ),
                ]),
            ),
        ],
    )
}

#[test]
fn difference_in_a_nested_field_is_named_by_path() {
    let difference = StructuralEquality::first_difference(&sample(2), &sample(3))
        .expect("the nested multiplicity differs");
    assert_eq!(
        difference, "$.solutions[1].multiplicity: Integer value differs",
        "the diagnostic must name the exact nested location"
    );
    assert!(difference.contains("solutions[1].multiplicity"));
    assert!(!StructuralEquality::values_equal(&sample(2), &sample(3)));
}

#[test]
fn different_kinds_are_reported_by_kind() {
    let difference =
        StructuralEquality::first_difference(&integer(1), &Value::Text("1".to_string()))
            .expect("an Integer and a Text differ");
    assert_eq!(difference, "$: kind Integer != Text");
    assert!(!StructuralEquality::values_equal(
        &integer(1),
        &Value::Text("1".to_string())
    ));
}

#[test]
fn different_shape_is_reported_as_shape() {
    let left = Value::Vector(vec![natural(1), natural(2)]);
    let right = Value::Vector(vec![natural(1)]);
    let difference =
        StructuralEquality::first_difference(&left, &right).expect("the shapes differ");
    assert_eq!(difference, "$: shape [2] != [1]");
    assert!(difference.contains("shape"));
}

#[test]
fn symbolic_equality_is_by_canonical_form_not_pretty_text() {
    // The C# body: x + x and 2*x are the same canonical expression, so the comparison is
    // canonical-driven (the C# assertion itself is a tautology; this pins the intended fact).
    let doubled = symbolic(Expr::add(Expr::sym("x"), Expr::sym("x")));
    let scaled = symbolic(Expr::mul(Expr::num(2), Expr::sym("x")));
    assert_eq!(doubled.display(), scaled.display());
    assert!(StructuralEquality::values_equal(&doubled, &scaled));
    assert!(StructuralEquality::first_difference(&doubled, &scaled).is_none());
    assert!(StructuralEquality::first_difference(&doubled, &doubled).is_none());

    // The discriminating direction: the same pretty text is not enough. A rational leaf and a
    // quotient node both render "-1/2", but their canonical forms differ, so they are NOT equal.
    let rational = symbolic(Expr::rational(&Int::from_i64(-1), &Nat::from_u64(2)));
    let quotient = symbolic(Expr::Div(Box::new(Expr::num(-1)), Box::new(Expr::num(2))));
    assert_eq!(rational.display(), quotient.display());
    assert_eq!(
        StructuralEquality::first_difference(&rational, &quotient),
        Some("$: symbolic canonical differs".to_string())
    );
}

#[test]
fn commutative_operands_canonicalise_to_one_form() {
    let x = Expr::sym("x");
    let y = Expr::sym("y");
    assert!(StructuralEquality::values_equal(
        &symbolic(Expr::add(x.clone(), y.clone())),
        &symbolic(Expr::add(y.clone(), x.clone()))
    ));
    assert!(StructuralEquality::values_equal(
        &symbolic(Expr::mul(x, y.clone())),
        &symbolic(Expr::mul(y, Expr::sym("x")))
    ));
}

#[test]
fn mathematically_equal_expressions_with_different_structure_are_unequal() {
    let x = Expr::sym("x");

    // (x + 1)^2 and x^2 + 2*x + 1 denote the same polynomial but are different trees.
    let square_of_sum = symbolic(Expr::pow(Expr::add(x.clone(), Expr::num(1)), Expr::num(2)));
    let expanded = symbolic(Expr::add(
        Expr::add(
            Expr::pow(x.clone(), Expr::num(2)),
            Expr::mul(Expr::num(2), x.clone()),
        ),
        Expr::num(1),
    ));
    assert_eq!(
        StructuralEquality::first_difference(&square_of_sum, &expanded),
        Some("$: symbolic canonical differs".to_string())
    );

    // The Pythagorean identity is not rewritten by a structural comparison either.
    let identity = symbolic(Expr::add(
        Expr::pow(Expr::Fun("sin".to_string(), vec![x.clone()]), Expr::num(2)),
        Expr::pow(Expr::Fun("cos".to_string(), vec![x]), Expr::num(2)),
    ));
    assert!(!StructuralEquality::values_equal(
        &identity,
        &symbolic(Expr::num(1))
    ));
}

#[test]
fn rational_and_real_leaves_are_different_node_kinds() {
    let rational = symbolic(Expr::rational(&Int::from_i64(1), &Nat::from_u64(2)));
    let decimal = symbolic(Expr::real("0.5"));
    assert_eq!(
        StructuralEquality::first_difference(&rational, &decimal),
        Some("$: symbolic canonical differs".to_string())
    );
}

#[test]
fn assumptions_do_not_change_structural_equality() {
    let positive_x = Assumption::SymbolProperty("x".to_string(), SymbolPredicate::Positive);
    let assumptions = AssumptionSet::empty()
        .add(positive_x.clone())
        .expect("x > 0 is a consistent assumption");
    assert_eq!(assumptions.ask(&positive_x), Tristate::True);

    let x = symbolic(Expr::sym("x"));
    let abs_x = symbolic(Expr::Fun("abs".to_string(), vec![Expr::sym("x")]));
    // abs(x) equals x mathematically under x > 0, but StructuralEquality takes no assumption set
    // and never simplifies: the pair stays structurally different.
    assert!(!StructuralEquality::values_equal(&abs_x, &x));
    assert_eq!(
        StructuralEquality::first_difference(&abs_x, &x),
        Some("$: symbolic canonical differs".to_string())
    );
    assert!(StructuralEquality::values_equal(&x, &x));
}

#[test]
fn same_display_different_kinds_are_unequal() {
    let natural_two = natural(2);
    let integer_two = integer(2);
    let symbolic_two = symbolic(Expr::num(2));
    assert_eq!(natural_two.display(), "2");
    assert_eq!(integer_two.display(), "2");
    assert_eq!(symbolic_two.display(), "2");
    assert_eq!(
        StructuralEquality::first_difference(&natural_two, &integer_two),
        Some("$: kind Natural != Integer".to_string())
    );
    assert_eq!(
        StructuralEquality::first_difference(&integer_two, &symbolic_two),
        Some("$: kind Integer != Symbolic".to_string())
    );
}

#[test]
fn record_identity_is_type_name_field_count_and_field_order() {
    let left = Value::Record(
        "A".to_string(),
        vec![("x".to_string(), natural(1)), ("y".to_string(), natural(2))],
    );
    let other_type = Value::Record(
        "B".to_string(),
        vec![("x".to_string(), natural(1)), ("y".to_string(), natural(2))],
    );
    assert_eq!(
        StructuralEquality::first_difference(&left, &other_type),
        Some("$: record type A != B".to_string())
    );

    let fewer_fields = Value::Record("A".to_string(), vec![("x".to_string(), natural(1))]);
    assert_eq!(
        StructuralEquality::first_difference(&left, &fewer_fields),
        Some("$: A has 2 fields, other has 1".to_string())
    );

    let reordered = Value::Record(
        "A".to_string(),
        vec![("y".to_string(), natural(1)), ("x".to_string(), natural(2))],
    );
    assert_eq!(
        StructuralEquality::first_difference(&left, &reordered),
        Some("$.A[0]: field name x != y".to_string())
    );
}

#[test]
fn enum_identity_includes_the_declared_type() {
    let partial = Value::Enum("SolveStatus".to_string(), "Partial".to_string());
    let completeness = Value::Enum("Completeness".to_string(), "Partial".to_string());
    assert_eq!(
        StructuralEquality::first_difference(&partial, &completeness),
        Some("$: enum type SolveStatus != Completeness".to_string())
    );
    let solved = Value::Enum("SolveStatus".to_string(), "Solved".to_string());
    assert_eq!(
        StructuralEquality::first_difference(&partial, &solved),
        Some("$: enum SolveStatus value Partial != Solved".to_string())
    );
}

#[test]
fn complex_parts_are_named_by_path() {
    let left = Value::Complex(Real::from_i64(1), Real::from_i64(2));
    let right = Value::Complex(Real::from_i64(1), Real::from_i64(3));
    assert_eq!(
        StructuralEquality::first_difference(&left, &right),
        Some("$.im: differs".to_string())
    );
}

#[test]
fn array_shape_and_element_paths_are_named() {
    let values = vec![natural(1), natural(2), natural(3), natural(4)];
    let left = Value::Array(NdArray::from_vec(vec![2, 2], values.clone()).expect("a 2x2 array"));
    let mut changed = values.clone();
    changed[3] = natural(5);
    let right = Value::Array(NdArray::from_vec(vec![2, 2], changed).expect("a 2x2 array"));
    assert_eq!(
        StructuralEquality::first_difference(&left, &right),
        Some("$[3]: Natural value differs".to_string())
    );

    let wider = Value::Array(
        NdArray::from_vec(
            vec![2, 3],
            vec![
                natural(1),
                natural(2),
                natural(3),
                natural(4),
                natural(5),
                natural(6),
            ],
        )
        .expect("a 2x3 array"),
    );
    assert_eq!(
        StructuralEquality::first_difference(&left, &wider),
        Some("$: shape [2,2] != [2,3]".to_string())
    );
}

#[test]
fn void_and_clamped_reals_follow_the_reference_kinds() {
    assert!(StructuralEquality::values_equal(&Value::Void, &Value::Void));
    assert_eq!(
        StructuralEquality::first_difference(&Value::Void, &natural(0)),
        Some("$: kind Void != Natural".to_string())
    );

    // The reference has one Real kind; the Rust clamp metadata is not part of the number.
    let plain = Value::Real(Real::from_i64(2));
    let clamped = Value::RealClamped(Real::from_i64(2), "digit-cap", 10);
    assert!(StructuralEquality::values_equal(&plain, &clamped));
}

#[test]
fn records_equal_requires_records_and_the_root_path_is_overridable() {
    assert!(StructuralEquality::records_equal(&sample(2), &sample(2)));
    assert!(!StructuralEquality::records_equal(&sample(2), &sample(3)));
    assert!(!StructuralEquality::records_equal(&sample(2), &natural(1)));

    assert_eq!(
        StructuralEquality::first_difference_from(&natural(1), &natural(2), "solutions[0].value"),
        Some("solutions[0].value: Natural value differs".to_string())
    );
}
