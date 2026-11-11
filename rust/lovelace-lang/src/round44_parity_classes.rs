//! Round 44: the last four differential classes - det kinds, limit conditions and the two
//! error-taxonomy/prose families. Every expected value and message is the oracle's own output for
//! the same script (Lovelace.Run --eval ... --json --omit-functions).

use crate::{Engine, LangError, Value};

fn eval(script: &str) -> Result<Value, LangError> {
    Engine::new().eval(script)
}

fn value(script: &str) -> Value {
    eval(script).unwrap_or_else(|error| panic!("script failed: {}", error.message))
}

fn display(script: &str) -> String {
    value(script).display()
}

fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    match value {
        Value::Record(_, fields) => fields
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
            .unwrap_or_else(|| panic!("the record has no '{name}' field")),
        other => panic!("expected a record, got {}", other.kind_name()),
    }
}

/// det of a NUMERIC matrix is TypedArrayOps.Det: Gaussian elimination over the numeric tier, so
/// the published KIND follows the operations (Real 46 for [[8,6],[3,8]], Integer -2 for
/// [[1,2],[3,4]], Natural 0 for a singular matrix). A symbolic matrix keeps the exact determinant.
#[test]
fn round44_numeric_determinants_publish_the_elimination_kind() {
    for (script, text, kind) in [
        ("det([[8, 6], [3, 8]])", "46", "Real"),
        ("det([[8, 1], [1, 7]])", "55", "Real"),
        ("det([[1, 2], [3, 4]])", "-2", "Integer"),
        ("det([[2, 0], [0, 3]])", "6", "Natural"),
        ("det([[1, 2, 3], [4, 5, 6], [7, 8, 9]])", "0", "Natural"),
        ("det([[0, 1], [1, 0]])", "-1", "Integer"),
        ("det([[5]])", "5", "Natural"),
    ] {
        assert_eq!(display(script), text, "{script}");
        assert_eq!(value(script).kind_name(), kind, "{script}");
    }
    assert_eq!(
        display("x = symbol(\"x\"); y = symbol(\"y\"); det([[x, 1], [y, x]])"),
        "x^2 - y"
    );
}

/// A one-sided limit's conditions are Symbolic RELATION leaves, not Text; measured oracle shape:
/// {"kind":"Symbolic","pretty":"x < 0","canonical":"(lt (sym x) (rat 0 1))", ...}.
#[test]
fn round44_limit_side_conditions_are_symbolic_relations() {
    let result = value("x = symbol(\"x\"); limit_full(1/x, x, 0)");
    let Value::Vector(left) = field(&result, "left_conditions") else {
        panic!("left_conditions must be a vector");
    };
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].kind_name(), "Symbolic");
    assert_eq!(left[0].display(), "x < 0");
    match &left[0] {
        Value::Symbolic(expression) => {
            assert_eq!(expression.canonical(), "(lt (sym x) (rat 0 1))")
        }
        other => panic!("expected Symbolic, got {}", other.kind_name()),
    }
    let Value::Vector(right) = field(&result, "right_conditions") else {
        panic!("right_conditions must be a vector");
    };
    assert_eq!(right[0].display(), "x > 0");
    match &right[0] {
        Value::Symbolic(expression) => {
            assert_eq!(expression.canonical(), "(gt (sym x) (rat 0 1))")
        }
        other => panic!("expected Symbolic, got {}", other.kind_name()),
    }
}

/// The reshape element-count refusal is the language builtin's InvalidArgument/TypeMismatch with
/// both counts; it used to escape as the array kernel's DomainError/"shape mismatch".
#[test]
fn round44_reshape_element_count_refusal_names_both_counts() {
    let error = eval("reshape([1, 2, 3, 4, 5, 6], 3, 3)").unwrap_err();
    assert_eq!(error.code, "InvalidArgument");
    assert_eq!(error.category, "TypeMismatch");
    assert_eq!(
        error.message,
        "reshape(): argument 2 (dims) describes 9 element(s), but the value it reshapes holds 6; the element count must agree."
    );
    let positioned = eval("x = symbol(\"x\"); reshape([1, 2, 3, 4, 5, 6], 2, 2)").unwrap_err();
    assert_eq!(positioned.position, 17);
    assert_eq!(
        positioned.message,
        "reshape(): argument 2 (dims) describes 4 element(s), but the value it reshapes holds 6; the element count must agree."
    );
}

/// A zero base under a negative exponent is the reference's argument refusal, not a
/// division-by-zero domain error.
#[test]
fn round44_zero_base_negative_exponent_is_an_argument_refusal() {
    let error = eval("0^(-1)").unwrap_err();
    assert_eq!(error.code, "InvalidArgument");
    assert_eq!(error.category, "TypeMismatch");
    assert_eq!(error.message, "Base cannot be zero. (Parameter 'base')");
    let positioned = eval("a = 1; 0^(-1)").unwrap_err();
    assert_eq!(positioned.position, 7);
}

/// An undefined name is reported at the statement the evaluator is running, never at the symbol.
#[test]
fn round44_undefined_variable_reports_the_statement_position() {
    let error = eval("diff(x)").unwrap_err();
    assert_eq!(error.message, "Undefined variable 'x'.");
    assert_eq!(error.position, 0);
    let positioned = eval("a = 1; diff(x)").unwrap_err();
    assert_eq!(positioned.message, "Undefined variable 'x'.");
    assert_eq!(positioned.position, 7);
}
