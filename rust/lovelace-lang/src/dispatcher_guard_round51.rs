//! Round-51 dispatcher guard: the registry and the dispatcher must agree.
//!
//! The gap this module pins closed: filter, scale, ndims and cancel_full were REGISTERED (so
//! --functions and the name-suggestion machinery published them) but the dispatcher's match had
//! no arm for them, so a call answered "Unknown function ... Did you mean '<itself>'?". No test
//! compared the two lists, and every registry/help test passed while four builtins could not run.
//!
//! The guard below is structural rather than example-driven: it enumerates EVERY registry entry
//! and sends a call through the real dispatcher. A registered name that has no arm answers
//! UnknownFunction and fails the test, so the next entry added to the registry without a
//! dispatcher arm is caught the moment it lands.

use crate::registry;
use crate::{Engine, LangError, Value};

fn eval(source: &str) -> Result<Value, LangError> {
    Engine::new().eval(source)
}

fn typed(source: &str) -> String {
    eval(source)
        .unwrap_or_else(|error| panic!("{source}: {}", error.message))
        .typed()
}

fn record_field<'a>(fields: &'a [(String, Value)], name: &str) -> &'a Value {
    fields
        .iter()
        .find(|(field, _)| field == name)
        .map(|(_, value)| value)
        .unwrap_or_else(|| panic!("the record has no {name} field"))
}

#[test]
fn every_registered_builtin_dispatches_instead_of_answering_unknown_function() {
    let mut unregistered_arms: Vec<&str> = Vec::new();
    for info in registry::BUILTINS {
        let mut engine = Engine::new();
        // One call through the real dispatcher is enough to decide recognition. The zero-argument
        // call is allowed to succeed (the domain constants) or to publish its own arity/shape
        // refusal; ONLY UnknownFunction means the registry has no matching arm.
        match crate::builtins::call(&mut engine, info.name, Vec::new(), 0) {
            Ok(_) => {}
            Err(error) if error.code == "UnknownFunction" => unregistered_arms.push(info.name),
            Err(_) => {}
        }
    }
    assert!(
        unregistered_arms.is_empty(),
        "registered builtins with no dispatcher arm: {unregistered_arms:?}"
    );
}

#[test]
fn the_four_gap_builtins_run_their_published_examples() {
    // The six descriptor examples that were unrunnable before the arms landed.
    assert_eq!(
        typed("x = symbol(\"x\"); cancel_full(x/x)"),
        "CancelResult(status: Conditional, original: x/x, expression: 1, changed: True, conditions: [x != 0]) (CancelResult)"
    );
    assert_eq!(
        typed("x = symbol(\"x\"); cancel_full((x^2 - 1)/(x - 1))"),
        "CancelResult(status: Conditional, original: (x^2 - 1)/(x - 1), expression: x + 1, changed: True, conditions: [x - 1 != 0]) (CancelResult)"
    );
    assert_eq!(typed("filter([1], [1], [1, 2, 3])"), "[1, 2, 3] (Vector)");
    assert_eq!(typed("ndims([[1, 2], [3, 4]])"), "2 (Natural)");
    assert_eq!(typed("scale([1, 2, 3], 2)"), "[2, 4, 6] (Vector)");
    assert_eq!(typed("scale([1, 2, 3], 0.5)"), "[0.5, 1, 1.5] (Vector)");
}

#[test]
fn filter_signal_and_impulse_forms_match_the_reference_arithmetic() {
    // FIR: filter(a=[1], b=[1,2,3], x=[1,1,1,1]) is the convolution prefix [1,3,6,6].
    assert_eq!(
        typed("filter([1], [1, 2, 3], [1, 1, 1, 1])"),
        "[1, 3, 6, 6] (Vector)"
    );
    // IIR: y[n] = x[n] + 0.5*y[n-1] with zero initial state.
    assert_eq!(
        typed("filter([1, -0.5], [1], [1, 1, 1, 1])"),
        "[1, 1.5, 1.75, 1.875] (Vector)"
    );
    // Scalar third argument: the same IIR's impulse response of length 4.
    assert_eq!(
        typed("filter([1, -0.5], [1], 4)"),
        "[1, 0.5, 0.25, 0.125] (Vector)"
    );
}

#[test]
fn cancel_full_publishes_the_conditions_the_bare_value_cannot_carry() {
    let conditional = eval("x = symbol(\"x\"); cancel_full(x/x)").expect("cancel_full(x/x)");
    let Value::Record(record_type, fields) = &conditional else {
        panic!("cancel_full must answer a CancelResult record, got {conditional:?}");
    };
    assert_eq!(record_type, "CancelResult");
    assert_eq!(
        record_field(fields, "status"),
        &Value::Enum("CancelStatus".to_string(), "Conditional".to_string())
    );
    assert_eq!(record_field(fields, "original").display(), "x/x");
    assert_eq!(record_field(fields, "expression").display(), "1");
    assert_eq!(record_field(fields, "changed"), &Value::Boolean(true));
    let Value::Vector(conditions) = record_field(fields, "conditions") else {
        panic!("conditions must be a vector");
    };
    assert_eq!(conditions.len(), 1);
    assert_eq!(conditions[0].display(), "x != 0");

    // An exact reduction reports no conditions at all and leaves the expression unchanged.
    let exact =
        eval("x = symbol(\"x\"); cancel_full((x^2 + 1)/(x - 1))").expect("cancel_full exact");
    let Value::Record(_, exact_fields) = &exact else {
        panic!("cancel_full must answer a CancelResult record");
    };
    assert_eq!(
        record_field(exact_fields, "status"),
        &Value::Enum("CancelStatus".to_string(), "Exact".to_string())
    );
    assert_eq!(
        record_field(exact_fields, "original").display(),
        record_field(exact_fields, "expression").display()
    );
    assert_eq!(
        record_field(exact_fields, "changed"),
        &Value::Boolean(false)
    );
    let Value::Vector(exact_conditions) = record_field(exact_fields, "conditions") else {
        panic!("conditions must be a vector");
    };
    assert!(exact_conditions.is_empty());

    // A power of a sum is expanded before the polynomial read; the removed pole is the condition.
    let powered = eval("x = symbol(\"x\"); cancel_full((x - 1)^2/(x - 1))")
        .expect("cancel_full((x - 1)^2/(x - 1))");
    let Value::Record(_, powered_fields) = &powered else {
        panic!("cancel_full must answer a CancelResult record");
    };
    assert_eq!(
        record_field(powered_fields, "expression").display(),
        "x - 1"
    );
    let Value::Vector(powered_conditions) = record_field(powered_fields, "conditions") else {
        panic!("conditions must be a vector");
    };
    assert_eq!(powered_conditions.len(), 1);
    assert_eq!(powered_conditions[0].display(), "x - 1 != 0");
}
