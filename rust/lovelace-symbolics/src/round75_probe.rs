//! Round 75: assumption-pair probing for the deferred
//! AssumptionSoundnessTests.PropertyAndRelationAtoms_Contradict row.
//!
//! The C# body asserts that four property/relation pairs are contradictory in the
//! property-first order and that the positive/Le pair is also caught in the reverse
//! order. The named Rust test `property_and_relation_atoms_contradict_for_the_five_pairs`
//! repeats exactly those five probes against the public AssumptionSet::add path.

use crate::assumptions::{Assumption, AssumptionSet, SymbolPredicate};
use crate::{Expr, RelOp};

fn symbol() -> String {
    "x".to_string()
}

fn relation(op: RelOp, bound: i64) -> Assumption {
    Assumption::SymbolRelation(symbol(), op, Expr::num(bound))
}

fn property(predicate: SymbolPredicate) -> Assumption {
    Assumption::SymbolProperty(symbol(), predicate)
}

/// AssumptionSoundnessTests.PropertyAndRelationAtoms_Contradict: the four
/// property-then-relation contradictions the C# body drives, plus the reverse
/// relation-first order it also requires.
#[test]
fn property_and_relation_atoms_contradict_for_the_five_pairs() {
    let property_first = [
        (SymbolPredicate::Positive, RelOp::Le),
        (SymbolPredicate::Negative, RelOp::Ge),
        (SymbolPredicate::NonNegative, RelOp::Lt),
        (SymbolPredicate::NonZero, RelOp::Eq),
    ];
    for (predicate, op) in property_first {
        let result = AssumptionSet::empty()
            .add(property(predicate))
            .expect("the single property is consistent")
            .add(relation(op, 0));
        assert!(
            result.is_err(),
            "{} together with x {} 0 must contradict",
            predicate.name(),
            match op {
                RelOp::Le => "<=",
                RelOp::Ge => ">=",
                RelOp::Lt => "<",
                RelOp::Eq => "==",
                _ => "?",
            }
        );
    }

    // The reverse order: the relation is added first, then the property.
    let reverse = AssumptionSet::empty()
        .add(relation(RelOp::Le, 0))
        .expect("x <= 0 is consistent")
        .add(property(SymbolPredicate::Positive));
    assert!(
        reverse.is_err(),
        "x <= 0 followed by Positive must be caught as well"
    );
}
