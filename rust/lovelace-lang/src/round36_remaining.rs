//! Round 36: the deferred rows the current production answers.
//!
//! Every test is one deferred census row (or a group of rows that assert the same case). The C#
//! assertion is the contract; where the Rust surface has a different shape the test asserts the
//! SAME value or refusal through the Rust surface, and
//! docs/goal-parity-contract/round-36/remaining-porting.md records the translation.

use std::cmp::Ordering;

use crate::{Engine, Value};
use lovelace_int::Int;
use lovelace_real::Real;

fn eval(script: &str) -> Value {
    Engine::new()
        .eval(script)
        .unwrap_or_else(|error| panic!("script: {script} failed: {}", error.message))
}

fn display(script: &str) -> String {
    eval(script).display()
}

fn real(text: &str) -> Real {
    Real::from_str(text).unwrap_or_else(|| panic!("bad real {text}"))
}

/// |actual - expected| < tolerance, on the exact-real tier.
fn close(actual: &Real, expected: &str, tolerance: &str) -> bool {
    let difference = actual.sub(&real(expected)).abs();
    difference.cmp(&real(tolerance)) == Ordering::Less
}

// ===========================================================================
// Lovelace.Integer.Tests.IntegerToStringTests.ToString_WithFormatProvider
// ===========================================================================

/// IntegerToStringTests.ToString_WithFormatProvider_ReturnsSameAsToString.
///
/// The reference has one rendering implementation and the format-provider overload delegates to it
/// (ToString(null, null) is ToString()). In Rust the same two entry points are Display and the
/// inherent to_decimal.
#[test]
fn integer_to_string_with_format_provider_returns_same_as_to_string() {
    for text in [
        "7",
        "-7",
        "0",
        "18446744073709551616",
        "-340282366920938463463374607431768211455",
    ] {
        let value = Int::from_decimal(text).unwrap_or_else(|| panic!("bad integer {text}"));
        assert_eq!(
            value.to_decimal(),
            format!("{value}"),
            "{text}: the two renderings differ"
        );
    }
}

// ===========================================================================
// Lovelace.Suite.Tests.DescriptorExampleTests.Example_Executes#11 (assume_clear)
// ===========================================================================

/// The reference's descriptor doctest evaluates the documented prelude, then the example. The
/// example for assume_clear() is "assume_clear()"; the row asserts it executes, and that is what
/// is asserted here: the snippet the descriptor publishes evaluates without refusing.
#[test]
fn descriptor_example_for_assume_clear_executes() {
    let prelude = "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); ";
    let value = Engine::new()
        .eval(prelude)
        .and_then(|_| Engine::new().eval(&format!("{prelude}assume_clear()")))
        .unwrap_or_else(|error| {
            panic!(
                "the assume_clear() descriptor example must execute: {}",
                error.message
            )
        });
    assert_eq!("assumptions cleared", value.display());
}

// ===========================================================================
// Lovelace.Symbolics.Tests.CapabilitiesBuiltinTests
//   SolveDomainStatement_NamesItsOperation_AndEveryDomainStaysUsableElsewhere
// ===========================================================================

fn record_field<'a>(value: &'a Value, name: &str) -> &'a Value {
    match value {
        Value::Record(type_name, fields) => fields
            .iter()
            .find(|(field, _)| field == name)
            .map(|(_, value)| value)
            .unwrap_or_else(|| panic!("record {type_name} has no field '{name}'")),
        other => panic!("not a record: {}", other.kind_name()),
    }
}

fn domain_names(value: &Value) -> Vec<String> {
    match value {
        Value::Vector(items) => items
            .iter()
            .map(|item| match item {
                Value::Domain(name) => name.clone(),
                other => panic!("not a domain: {}", other.kind_name()),
            })
            .collect(),
        other => panic!("not a vector: {}", other.kind_name()),
    }
}

/// SolveDomainStatement_NamesItsOperation_AndEveryDomainStaysUsableElsewhere.
#[test]
fn solve_domain_statement_names_its_operation_and_every_domain_stays_usable_elsewhere() {
    let capabilities = eval("capabilities()");
    match &capabilities {
        Value::Record(type_name, _) => assert_eq!("CapabilitiesResult", type_name),
        other => panic!("capabilities() answered {}", other.kind_name()),
    }

    assert_eq!(
        vec!["real".to_string(), "complex".to_string()],
        domain_names(record_field(&capabilities, "solve_domains_accepted"))
    );
    assert_eq!(
        vec!["integer".to_string(), "rational".to_string()],
        domain_names(record_field(&capabilities, "solve_domains_refused"))
    );
    // the ambiguous runtime-wide claim is GONE, not merely re-valued
    assert!(
        matches!(&capabilities, Value::Record(_, fields) if !fields.iter().any(|(n, _)| n == "supported_domains" || n == "unsupported_domains")),
        "the record still carries the runtime-wide domain pair"
    );

    // every REFUSED domain must refuse, with the advertised code/category
    for domain in ["integer", "rational"] {
        let refusal = Engine::new()
            .eval(&format!(
                "x = symbol(\"x\"); solve(x^2 - 2 == 0, x, {domain})"
            ))
            .err()
            .unwrap_or_else(|| {
                panic!("the statement refuses {domain}, so the live call must refuse it")
            });
        assert_eq!("InvalidOperation", refusal.code, "{domain}");
        assert_eq!("DomainError", refusal.category, "{domain}");
        assert!(
            refusal.message.contains(&format!("got {domain}")),
            "{domain}: {}",
            refusal.message
        );
    }

    // every ACCEPTED domain must answer with the requested domain on the record
    for domain in ["real", "complex"] {
        let value = eval(&format!(
            "x = symbol(\"x\"); solve(x^2 - 2 == 0, x, {domain})"
        ));
        match &value {
            Value::Record(type_name, _) => assert_eq!("SolveResult", type_name, "{domain}"),
            other => panic!("{domain}: solve answered {}", other.kind_name()),
        }
        match record_field(&value, "domain") {
            Value::Domain(published) => assert_eq!(domain, published, "{domain}"),
            other => panic!("{domain}: the domain field is {}", other.kind_name()),
        }
    }

    // the surface the renamed fields no longer speak for: every domain value works
    for domain in ["real", "complex", "integer", "rational"] {
        let value = eval(&format!("symbol(\"s\", {domain})"));
        assert_eq!("s", value.display(), "symbol(name, {domain}) is supported");
    }
}

// ===========================================================================
// Lovelace.Symbolics.Tests.NegativePowerEvaluationTests
// ===========================================================================

const TOLERANCE: &str = "0.00000000000000000001";

/// NegativeIntegerPowerOfAReal_IsTheReciprocal — the C# assertion evaluates the canonical power at
/// x = 1/3 through the kernel; the Rust surface evaluates the same expression through the engine,
/// and the value is the assertion.
#[test]
fn negative_integer_power_of_a_real_is_the_reciprocal() {
    let positive = eval("x = symbol(\"x\"); evalf(subs(cos(x^3)^2, x, 1/3), 40)");
    let Value::Real(positive) = positive else {
        panic!("cos(1/27)^2 is not a Real")
    };
    assert!(
        close(
            &positive,
            "0.998628884998283893103390698137479406248279",
            TOLERANCE
        ),
        "cos(1/27)^2 = {positive}"
    );

    let negative = eval("x = symbol(\"x\"); evalf(subs(cos(x^3)^(-2), x, 1/3), 40)");
    let Value::Real(negative) = negative else {
        panic!("cos(1/27)^-2 is not a Real")
    };
    assert!(
        close(
            &negative,
            "1.00137299753923947744327004754154925203741",
            TOLERANCE
        ),
        "cos(1/27)^-2 = {negative}"
    );
}

/// DerivativeOfTanOfXCubed_EvaluatesToTheSymPyValue — 3*x^2*cos(x^3)^-2 at x = 1/3.
#[test]
fn derivative_of_tan_of_x_cubed_evaluates_to_the_sympy_value() {
    let value = eval("x = symbol(\"x\"); evalf(subs(3*x^2*cos(x^3)^(-2), x, 1/3), 40)");
    let Value::Real(value) = value else {
        panic!("3*x^2*cos(x^3)^-2 is not a Real")
    };
    assert!(
        close(
            &value,
            "0.333790999179746492481090015847183084012471",
            TOLERANCE
        ),
        "3*x^2*cos(x^3)^-2 at 1/3 = {value}"
    );
}

// ===========================================================================
// Lovelace.Symbolics.Tests.TruncatingEvaluationStaysInexactTests
//   SymbolicTreeFlag_AndNumericReadBack_Agree (#0..#3)
// ===========================================================================

/// The symbolic tree's flag and the numeric read-back's flag must agree for the same expression.
#[test]
fn symbolic_tree_flag_and_numeric_read_back_agree() {
    for expression in [
        "subs(x + 1, x, pi(30))",
        "subs(x + 1, x, sqrt(2))",
        "subs(x*2, x, pi(30))",
        "subs(x + x, x, pi(30))",
    ] {
        let tree = eval(&format!("x = symbol(\"x\"); inspect({expression}).exact"));
        assert_eq!(
            Value::Boolean(false).display(),
            tree.display(),
            "{expression}: the tree claims exactness"
        );
        let numeric = eval(&format!("x = symbol(\"x\"); evalf({expression}, 40)"));
        let Value::Real(numeric) = numeric else {
            panic!("{expression}: evalf is not a Real")
        };
        assert!(
            !numeric.is_exact(),
            "{expression}: the numeric read-back claims exactness ({numeric})"
        );
    }
}

// ===========================================================================
// Lovelace.Symbolics.Tests.UsageExamples (the README snippets that assert values)
// ===========================================================================

/// Snippet_16_1_ContextSymbolsAndConstruction.
#[test]
fn snippet_16_1_context_symbols_and_construction() {
    assert_eq!("x^2 + 2*x + 1", display("x = symbol(\"x\"); x^2 + 2*x + 1"));
}

/// Snippet_16_3_AssumptionsDriveSimplification: unchanged unconstrained, x under x >= 0.
#[test]
fn snippet_16_3_assumptions_drive_simplification() {
    assert_eq!(
        "sqrt(x^2)",
        display("x = symbol(\"x\"); simplify(sqrt(x^2))")
    );
    assert_eq!(
        "x",
        display("x = symbol(\"x\"); assume(x >= 0); simplify(sqrt(x^2))")
    );
}

/// Snippet_16_6_MathIrLowerAndEvaluate: lower x^2 + 1, evaluate at 4, answer 17.
#[test]
fn snippet_16_6_math_ir_lower_and_evaluate() {
    assert_eq!(
        "17",
        display("x = symbol(\"x\"); evalir(lower(x^2 + 1, [x]), [4], 40)")
    );
}

/// Snippet_16_7_SymbolicMatrices: det([[x, 1], [y, x]]) is x^2 - y.
#[test]
fn snippet_16_7_symbolic_matrices() {
    assert_eq!(
        "x^2 - y",
        display("x = symbol(\"x\"); y = symbol(\"y\"); det([[x, 1], [y, x]])")
    );
}

/// Snippet_16_5_NumericEvaluationAtPrecision: x^2 + 1 at x = 3 evaluates to 10.
#[test]
fn snippet_16_5_numeric_evaluation_at_precision() {
    let value = eval("x = symbol(\"x\"); evalf(subs(x^2 + 1, x, 3), 40)");
    assert_eq!("10", value.display());
    match value {
        crate::Value::Integer(n) => assert_eq!("10", n.to_decimal()),
        other => panic!("the numeric evaluation answered {}", other.kind_name()),
    }
}
