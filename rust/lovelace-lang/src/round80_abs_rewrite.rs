//! Round 80 — the abs-rewrite behavioural family (four deferred rows).
//!
//! The four census rows behind these tests are:
//!
//! * `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.AbsSymbolicRoundTripTests/Abs_Of_Symbol_Matches_The_Rewriters_Own_Abs_Node#0`
//! * `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.AbsSymbolicRoundTripTests/Simplify_Sqrt_Square_Under_Real_Is_Still_Abs_X#0`
//! * `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.AbsSymbolicRoundTripTests/Subs_Abs_Symbolic_At_Negative_Value_Gives_Three#0`
//! * `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.UnsatisfiablePropagationTests/SimplifyFull_DomainCondition_CarriesADomainValueNotText#0`
//!
//! Each test drives the same language script the C# body drives and asserts the oracle-observed
//! values measured against `Lovelace.Run` (round 80, 2026-11-12): the real-domain
//! `pow.sqrt-square-real` rewrite produces `abs(x)` with the `DomainCondition` payload, and
//! `subs(abs(x), x, -3)` folds at construction to the exact rational 3.

use lovelace_symbolics::Expr;

use crate::{Engine, Value};

fn eval(engine: &mut Engine, script: &str) -> Value {
    engine
        .eval(script)
        .unwrap_or_else(|error| panic!("'{script}' failed: {}", error.message))
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

fn symbolic(value: &Value) -> &Expr {
    match value {
        Value::Symbolic(expression) => expression,
        other => panic!("expected a symbolic value, got {}", other.kind_name()),
    }
}

fn vector_items(value: &Value) -> &[Value] {
    match value {
        Value::Vector(items) => items,
        other => panic!("expected a vector, got {}", other.kind_name()),
    }
}

/// `AbsSymbolicRoundTripTests.cs:24`: `abs(x)` builds the same node the rewriter builds for
/// `sqrt(x^2)` under a real `x`, and the rewriter's `expression` field is that node.
#[test]
fn abs_of_symbol_matches_the_rewriters_own_abs_node() {
    let mut engine = Engine::new();
    eval(&mut engine, "x = symbol(\"x\", real)");

    let applied = eval(&mut engine, "abs(x)");
    let applied = symbolic(&applied);
    assert_eq!(applied.to_string(), "abs(x)");
    assert_eq!(applied.canonical(), "(fn abs (sym x))");

    let rewritten = eval(&mut engine, "simplify_full(sqrt(x^2)).expression");
    let rewritten = symbolic(&rewritten);
    assert_eq!(rewritten.canonical(), applied.canonical());
    // Structural identity with the node the rewrites themselves construct.
    assert_eq!(
        *rewritten,
        Expr::Fun("abs".to_string(), vec![Expr::sym("x")])
    );
}

/// `AbsSymbolicRoundTripTests.cs:88`: under `x` real, `simplify_full(sqrt(x^2))` is a
/// `TransformResult` whose expression is `abs(x)`, with the domain-specific provenance the
/// reference publishes.
#[test]
fn simplify_sqrt_square_under_real_is_still_abs_x() {
    let mut engine = Engine::new();
    eval(&mut engine, "x = symbol(\"x\", real)");

    assert_eq!(
        eval(&mut engine, "type(simplify_full(sqrt(x^2)))").display(),
        "TransformResult"
    );
    let result = eval(&mut engine, "simplify_full(sqrt(x^2))");
    assert_eq!(
        symbolic(record_field(&result, "expression")).to_string(),
        "abs(x)"
    );
    assert_eq!(
        symbolic(record_field(&result, "expression")).canonical(),
        "(fn abs (sym x))"
    );
    assert_eq!(record_field(&result, "changed").display(), "True");
    assert_eq!(
        record_field(&result, "conditions").display(),
        "[DomainCondition(variable: x, domain: real)]"
    );
    let steps = vector_items(record_field(&result, "steps"));
    assert_eq!(steps.len(), 1);
    assert_eq!(
        record_field(&steps[0], "rule_id").display(),
        "pow.sqrt-square-real"
    );
    assert_eq!(
        record_field(&steps[0], "classification").display(),
        "DomainSpecific"
    );
}

/// `AbsSymbolicRoundTripTests.cs:73`: `subs(abs(x), x, -3)` is the exact rational 3 — a
/// `Symbolic` `(rat 3 1)` with no free symbols, not `abs(-3)`.
#[test]
fn subs_abs_symbolic_at_negative_value_gives_three() {
    let mut engine = Engine::new();
    eval(&mut engine, "x = symbol(\"x\")");

    let value = eval(&mut engine, "subs(abs(x), x, -3)");
    let expression = symbolic(&value);
    assert_eq!(expression.to_string(), "3");
    assert_eq!(expression.canonical(), "(rat 3 1)");
    assert!(expression.free_symbols().is_empty());
}

/// `UnsatisfiablePropagationTests.cs:80`: the `simplify_full(sqrt(x^2))` condition is a
/// `DomainCondition` record carrying a `Symbolic` variable and a `Domain` value (not text),
/// and no raw C# record rendering leaks into the published display.
#[test]
fn simplify_full_domain_condition_carries_a_domain_value_not_text() {
    let mut engine = Engine::new();
    eval(&mut engine, "x = symbol(\"x\", real)");

    let result = eval(&mut engine, "simplify_full(sqrt(x^2))");
    let conditions = record_field(&result, "conditions");
    let items = vector_items(conditions);
    assert_eq!(items.len(), 1);
    let Value::Record(type_name, fields) = &items[0] else {
        panic!(
            "expected a DomainCondition record, got {}",
            items[0].kind_name()
        );
    };
    assert_eq!(*type_name, "DomainCondition");
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].0, "variable");
    assert_eq!(fields[0].1.kind_name(), "Symbolic");
    assert_eq!(symbolic(&fields[0].1).canonical(), "(sym x)");
    assert_eq!(fields[1].0, "domain");
    assert_eq!(fields[1].1.kind_name(), "Domain");
    assert_eq!(fields[1].1.display(), "real");
    assert!(!conditions.display().contains("SymbolDomainAssumption"));
}
