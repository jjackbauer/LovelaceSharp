//! Round 42: the differential-class regressions this round fixed. Every expected string is the
//! oracle's own answer for the same script (Lovelace.Run --eval ... --json --omit-functions), so a
//! future drift is caught at the language boundary rather than only in a diff run.

use crate::{Engine, LangError, Value};

fn eval(script: &str) -> Result<Value, LangError> {
    Engine::new().eval(script)
}

fn display(script: &str) -> String {
    eval(script)
        .unwrap_or_else(|error| panic!("script failed: {}", error.message))
        .display()
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

// ---------------------------------------------------------------------------
// Exit-code class: refusals the oracle makes and the runner used to accept.
// ---------------------------------------------------------------------------

#[test]
fn round42_hash_comment_is_a_parse_error_at_the_hash() {
    let error = eval("# leading comment\n2 + 3").unwrap_err();
    assert_eq!(error.message, "Unexpected character '#' at position 0.");
    assert_eq!(error.code, "ParseError");
    assert_eq!(error.category, "ParseError");
    assert_eq!(error.position, 0);
}

#[test]
fn round42_non_ascii_identifier_is_refused_in_code_units() {
    let error = eval("résultat = 2; résultat").unwrap_err();
    assert_eq!(error.message, "Unexpected character 'é' at position 1.");
    assert_eq!(error.code, "ParseError");
    assert_eq!(error.category, "ParseError");
    assert_eq!(error.position, 1);
}

#[test]
fn round42_numeric_linsolve_is_refused_as_a_symbolic_matrix_builtin() {
    for (script, message) in [
        (
            "linsolve([[1, 2], [3, 4]], [1, 2])",
            "linsolve() requires a symbolic matrix A.",
        ),
        (
            "linsolve_full([[1, 2], [3, 4]], [x, 1])",
            "linsolve_full() requires a symbolic matrix A.",
        ),
    ] {
        let prepared = if script.contains("x, 1") {
            format!("x = symbol(\"x\"); {script}")
        } else {
            script.to_string()
        };
        let error = eval(&prepared).unwrap_err();
        assert_eq!(error.message, message);
        assert_eq!(error.code, "InvalidOperation");
        assert_eq!(error.category, "DomainError");
    }
    // one symbolic entry makes the matrix symbolic; the numeric solve still works
    assert_eq!(
        display("x = symbol(\"x\"); linsolve([[1, 2], [3, x]], [1, 2])"),
        "[1 + 2/(x - 6), -1/(x - 6)]"
    );
}

// ---------------------------------------------------------------------------
// Matrix record shape class.
// ---------------------------------------------------------------------------

#[test]
fn round42_numeric_inverse_kinds_follow_the_numeric_operations() {
    let value = eval("inv([[1, 2], [3, 7]])").unwrap();
    let Value::Array(matrix) = value else {
        panic!("inv must answer an array");
    };
    let expected = [
        ("7", "Integer"),
        ("-2", "Integer"),
        ("-3", "Integer"),
        ("1", "Natural"),
    ];
    for (i, (text, kind)) in expected.iter().enumerate() {
        let entry = matrix
            .get(&[i / 2, i % 2])
            .unwrap_or_else(|| panic!("element {i} missing"));
        assert_eq!(entry.display(), *text, "element {i} value");
        assert_eq!(entry.kind_name(), *kind, "element {i} kind");
    }
    // A pivot of 3 promotes every entry it touches to Real, even an integral one.
    let value = eval("inv([[3, 5], [4, 6]])").unwrap();
    let Value::Array(matrix) = value else {
        panic!("inv must answer an array");
    };
    for i in 0..4 {
        let entry = matrix.get(&[i / 2, i % 2]).expect("element");
        assert_eq!(entry.kind_name(), "Real", "element {i}");
    }
}

#[test]
fn round42_singular_numeric_inverse_names_the_full_refusal() {
    let error = eval("inv([[1, 4], [2, 8]])").unwrap_err();
    assert_eq!(error.message, "Matrix is singular and cannot be inverted.");
}

#[test]
fn round42_symbolic_inverse_elements_are_products_over_the_determinant() {
    let value = eval("x = symbol(\"x\"); inv([[x, 1], [1, x]])").unwrap();
    let Value::Array(matrix) = value else {
        panic!("inv must answer an array");
    };
    let expected = "(mul (sym x) (pow (add (rat -1 1) (pow (sym x) (rat 2 1))) (rat -1 1)))";
    match matrix.get(&[0, 0]).expect("element") {
        Value::Symbolic(expr) => {
            assert_eq!(expr.canonical(), expected);
            // Divide(a, b) is Multiply(a, Power(b, -1)) in the reference: 9 nodes, not a Div's 7.
            assert_eq!(expr.node_count(), 9);
        }
        other => panic!("expected a symbolic element, got {}", other.kind_name()),
    }
    // Zero over the determinant is NOT folded away (0/(x*y) stays visible).
    let value = eval("x = symbol(\"x\"); y = symbol(\"y\"); inv([[x, 1], [0, y]])").unwrap();
    let Value::Array(matrix) = value else {
        panic!("inv must answer an array");
    };
    assert_eq!(matrix.get(&[1, 0]).expect("element").display(), "0/(x*y)");
}

#[test]
fn round42_inv_full_carries_the_nonzero_determinant_condition() {
    let value = eval("x = symbol(\"x\"); inv_full([[x, 1], [1, x]])").unwrap();
    let Value::Vector(conditions) = field(&value, "conditions") else {
        panic!("conditions must be a vector");
    };
    assert_eq!(conditions.len(), 1);
    assert_eq!(conditions[0].display(), "x^2 - 1 != 0");
    let Value::Array(inverse) = field(&value, "inverse") else {
        panic!("inverse must be an array");
    };
    assert_eq!(
        inverse.get(&[0, 0]).expect("element").display(),
        "x/(x^2 - 1)"
    );
}

#[test]
fn round42_linsolve_full_solutions_are_the_flat_solution_vector() {
    let value = eval("x = symbol(\"x\"); linsolve_full([[x, 1], [1, x]], [5, 4])").unwrap();
    let Value::Vector(solutions) = field(&value, "solutions") else {
        panic!("solutions must be a vector");
    };
    assert_eq!(
        solutions.len(),
        2,
        "the solution vector has one entry per unknown"
    );
    assert_eq!(solutions[0].display(), "(5 - (4*x - 5)/(x^2 - 1))/x");
    let Value::Vector(conditions) = field(&value, "conditions") else {
        panic!("conditions must be a vector");
    };
    assert_eq!(conditions[0].display(), "x^2 - 1 != 0");
}
