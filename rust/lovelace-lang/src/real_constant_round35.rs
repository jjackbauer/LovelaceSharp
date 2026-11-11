//! Round-35 ports of the RealLiteral / RealConstant family: the symbolic layer carries a real
//! constant leaf when a value crosses in from the numeric tier, and that leaf is outside the
//! exact basis by its kind.
//!
//! The row ids are listed in docs/goal-parity-contract/round-35/assumptions-realliteral.md.

use crate::{Engine, LangError, Value};

fn display(script: &str) -> String {
    Engine::new()
        .eval(script)
        .unwrap_or_else(|error| panic!("'{script}' failed: {}", error.message))
        .display()
}

fn eval(script: &str) -> Result<Value, LangError> {
    Engine::new().eval(script)
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

/// C# ExactnessFlagTests.InspectExact_IsFalse_WhenARealApproximationIsInTheTree and
/// KernelHardeningTests.NumericToSymbolicToNumeric_500Digits_RoundTrips: substituting a truncated
/// constant keeps a Real constant leaf, printed as its decimal, outside the exact basis.
#[test]
fn substituting_a_truncated_constant_keeps_a_real_leaf() {
    assert_eq!(
        display("x = symbol(\"x\"); subs(x + 1, x, pi(30))"),
        "1 + 3.141592653589793238462643383279"
    );
    let inspection = eval("x = symbol(\"x\"); inspect(subs(x + 1, x, pi(30)))").expect("inspect");
    assert_eq!(record_field(&inspection, "exact").display(), "False");
    assert_eq!(record_field(&inspection, "domain").display(), "real");
    assert_eq!(
        record_field(&inspection, "canonical").display(),
        "(add (rat 1 1) (real 3.141592653589793238462643383279))"
    );
    // e crosses the same way
    assert_eq!(
        display("x = symbol(\"x\"); subs(x + 1, x, e(30))"),
        "1 + 2.718281828459045235360287471352"
    );
}

/// C# TruncatingEvaluationStaysInexactTests.LiteralRoundTrip_PreservesBothTheDigitsAndTheRoute:
/// the leaf keeps the value's digits and its inexact route.
#[test]
fn the_real_leaf_preserves_the_digits_and_the_route() {
    let exact = eval("x = symbol(\"x\"); inspect(subs(x + 1, x, pi(30))).exact").expect("exact");
    assert_eq!(exact.display(), "False");
    // a decimal that fits the reference's read-back boundary is a rational instead
    assert_eq!(
        display("x = symbol(\"x\"); inspect(subs(x + 1, x, pi(18))).exact"),
        "True"
    );
    assert_eq!(
        display("x = symbol(\"x\"); subs(x + 1, x, pi(18))"),
        "2070796326794896619/500000000000000000"
    );
}

/// C# ExactnessFlagTests.ElementaryFunctionOfAnApproximateArgument_IsInexact: the approximate
/// argument is a Real leaf, so the function application is inexact too - and evalf still
/// evaluates it.
#[test]
fn an_elementary_function_of_an_approximate_argument_is_inexact_and_evaluable() {
    assert_eq!(
        display("evalf(sin(pi(1)*1/6), 40)"),
        "0.4939846521590624789945640658702374513097"
    );
    let inspection = eval("x = symbol(\"x\"); inspect(sin(pi(30)))").expect("inspect");
    assert_eq!(record_field(&inspection, "exact").display(), "False");
}

/// C# TruncatingEvaluationStaysInexactTests.FunctionOfATruncatedValue_PublishesExactFalse_AndNoRationalForm:
/// a short truncated constant is read back as the exact rational it denotes, so the value it
/// produces is exact again.
#[test]
fn short_truncated_constants_read_back_as_exact_rationals() {
    assert_eq!(
        display("x = symbol(\"x\"); subs(x + 1, x, pi(2))"),
        "207/50"
    );
    assert_eq!(
        display("x = symbol(\"x\"); subs(x^2, x, pi(2))"),
        "24649/2500"
    );
    // beyond the boundary the leaf stays a real power
    assert_eq!(
        display("x = symbol(\"x\"); subs(x^2, x, pi(30))"),
        "3.141592653589793238462643383279^2"
    );
}

/// The real leaf is a constant: one node, no free symbols, real-valued, never exact.
#[test]
fn the_real_leaf_is_a_real_constant_leaf() {
    let inspection = eval("x = symbol(\"x\"); inspect(subs(x, x, pi(30)))").expect("inspect");
    assert_eq!(record_field(&inspection, "node_count").display(), "1");
    assert_eq!(record_field(&inspection, "free_symbols").display(), "[]");
    assert_eq!(record_field(&inspection, "domain").display(), "real");
    assert_eq!(record_field(&inspection, "exact").display(), "False");
}
