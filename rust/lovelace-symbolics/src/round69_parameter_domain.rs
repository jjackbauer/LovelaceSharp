//! Round-69: the deferred `ParameterDomainOf_MapsEveryDeclaredArm#0` census row.
//!
//! `Lovelace.Symbolics.Tests/DxStructuredResultsTests.cs:562` pins a dictionary of the WHOLE
//! `ParameterDomain` enum onto `MathDomain.Integer`, requires the pinned keys to equal
//! `Enum.GetValues<ParameterDomain>()`, maps every declared arm, and requires
//! `(ParameterDomain)99` to throw `ArgumentOutOfRangeException`. Every claim is reproduced here
//! against the Rust surface; `Dom` is the kernel lattice the reference's `MathDomain` is.

use crate::Dom;
use crate::parameter_domain::{
    ParameterDomain, UndeclaredParameterDomain, parameter_domain_of, parameter_domain_of_ordinal,
};

/// The C# test's `pinned` dictionary, in the same order.
fn pinned_table() -> [(ParameterDomain, Dom); 2] {
    [
        (ParameterDomain::Integers, Dom::Integer),
        (ParameterDomain::NonNegativeIntegers, Dom::Integer),
    ]
}

/// C# DxStructuredResultsTests.cs:562 `ParameterDomainOf_MapsEveryDeclaredArm`: both declared
/// arms map to the integer domain, the table is exactly the declared enumeration, and an
/// undeclared value is refused rather than silently inheriting `Integer`.
#[test]
fn parameter_domain_of_maps_every_declared_arm() {
    let pinned = pinned_table();

    // the table must be the WHOLE enum: a third arm cannot be added without a decision
    let mut table: Vec<ParameterDomain> = pinned.iter().map(|(domain, _)| *domain).collect();
    table.sort();
    let mut declared = ParameterDomain::ALL.to_vec();
    declared.sort();
    assert_eq!(table, declared, "the pinned table must be the whole enum");

    for (domain, expected) in pinned {
        assert_eq!(
            parameter_domain_of(domain),
            expected,
            "{domain:?} must map to the integer domain"
        );
        assert_eq!(parameter_domain_of_ordinal(domain.ordinal()), Ok(expected));
    }

    // an arm the switch does not declare must not silently inherit an answer: the mapping is
    // total, so every undeclared value is a decision that has not been made
    let error = parameter_domain_of_ordinal(99)
        .expect_err("(ParameterDomain)99 must be refused, not mapped to Integer");
    assert_eq!(error, UndeclaredParameterDomain { ordinal: 99 });
    assert_eq!(
        error.to_string(),
        "No wire Domain is declared for this parameter domain. (Parameter 'domain') Actual value was 99."
    );
}

/// The C# test's `Enum.GetValues<ParameterDomain>().OrderBy(v => v)`: the declared arms are the
/// reference enumeration, with the reference ordinals 0 and 1 and no others.
#[test]
fn declared_arms_are_the_reference_enumeration() {
    assert_eq!(
        ParameterDomain::ALL,
        [
            ParameterDomain::Integers,
            ParameterDomain::NonNegativeIntegers,
        ]
    );
    for (index, arm) in ParameterDomain::ALL.into_iter().enumerate() {
        assert_eq!(arm.ordinal(), index as i32);
        assert_eq!(ParameterDomain::from_ordinal(arm.ordinal()), Ok(arm));
    }
}

/// One arm per test, so a future arm that maps elsewhere cannot hide behind a loop: the
/// `Integers` arm is the integer domain.
#[test]
fn integers_arm_maps_to_the_integer_domain() {
    assert_eq!(ParameterDomain::Integers.ordinal(), 0);
    assert_eq!(parameter_domain_of(ParameterDomain::Integers), Dom::Integer);
    assert_eq!(
        parameter_domain_of_ordinal(ParameterDomain::Integers.ordinal()),
        Ok(Dom::Integer)
    );
}

/// The second declared arm: `NonNegativeIntegers` is also the integer domain, because its
/// non-negativity constraint rides on the family's conditions, not on the wire domain.
#[test]
fn non_negative_integers_arm_maps_to_the_integer_domain() {
    assert_eq!(ParameterDomain::NonNegativeIntegers.ordinal(), 1);
    assert_eq!(
        parameter_domain_of(ParameterDomain::NonNegativeIntegers),
        Dom::Integer
    );
    assert_eq!(
        parameter_domain_of_ordinal(ParameterDomain::NonNegativeIntegers.ordinal()),
        Ok(Dom::Integer)
    );
}

/// The reference's refusal is total: every ordinal outside the declared set is refused with the
/// reference exception's message body, parameter name and actual value.
#[test]
fn undeclared_values_are_refused() {
    for ordinal in [-1, 2, 99, i32::MAX] {
        let error = parameter_domain_of_ordinal(ordinal).expect_err("undeclared ordinal");
        assert_eq!(error.ordinal, ordinal);
        assert_eq!(ParameterDomain::from_ordinal(ordinal), Err(error));
        assert_eq!(
            error.to_string(),
            format!(
                "No wire Domain is declared for this parameter domain. (Parameter 'domain') Actual value was {ordinal}."
            )
        );
    }
}
