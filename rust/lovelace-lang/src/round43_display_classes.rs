//! Round 43: the display classes - the kernel's display term order and the Real display cap.
//! Expected strings are the oracle's own output for the same script (Lovelace.Run --eval ...).

use lovelace_json::Json;

use crate::{Engine, Value};

fn eval(script: &str) -> Value {
    Engine::new()
        .eval(script)
        .unwrap_or_else(|error| panic!("script failed: {}", error.message))
}

/// The WIRE display form: Engine::format scopes the Real display cap exactly as the runner does,
/// so this is the rendering the differential harness compares.
fn display(script: &str) -> String {
    let mut engine = Engine::new();
    let value = engine
        .eval(script)
        .unwrap_or_else(|error| panic!("script failed: {}", error.message));
    engine.format(&value)
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

/// Printing.OrderTermsForDisplay: a quotient sum is NOT a single-symbol polynomial, so it renders
/// in canonical order (the constant first), not in the stored degree order (the constant last).
#[test]
fn round43_linsolve_sums_render_in_canonical_order() {
    assert_eq!(
        display("x = symbol(\"x\"); linsolve([[x, 1], [1, x]], [5, 4])"),
        "[(5 - (4*x - 5)/(x^2 - 1))/x, (4*x - 5)/(x^2 - 1)]"
    );
    let full = eval("x = symbol(\"x\"); linsolve_full([[x, 1], [1, x]], [5, 4])");
    let Value::Vector(solutions) = field(&full, "solutions") else {
        panic!("solutions must be a vector");
    };
    assert_eq!(solutions[0].display(), "(5 - (4*x - 5)/(x^2 - 1))/x");
    // A single-symbol polynomial keeps the human descending-degree order (x - 1, never -1 + x).
    assert_eq!(display("x = symbol(\"x\"); x^2 - 1"), "x^2 - 1");
    assert_eq!(
        display("x = symbol(\"x\"); 1 + (x^2 - 1)^100"),
        "1 + (x^2 - 1)^100"
    );
}

/// Real.DisplayDecimalPlaces: a non-periodic value publishes 100 fractional digits by default,
/// while the structured value keeps every computed digit (the reference publishes both).
#[test]
fn round43_default_real_display_is_capped_at_one_hundred_places() {
    let rendered = display("norm([7, 1, 1])");
    assert_eq!(
        rendered,
        "7.1414284285428499979993998113672652787661711599027338332084308827658204064400218862589882135328204182"
    );
    assert_eq!(rendered.chars().count(), 102);
    let value = eval("norm([7, 1, 1])");
    let structured = crate::structured::project_value(&Engine::new(), &value, None);
    let full = structured
        .get("value")
        .and_then(Json::as_str)
        .expect("the structured Real publishes its full value");
    assert!(
        full.len() > rendered.len() && full.starts_with(&rendered),
        "the structured value keeps the computed digits: {} vs {}",
        full.len(),
        rendered.len()
    );
}

/// setprecision moves the display cap with the computation precision (the reference's
/// WithPrecision scope sets BOTH knobs).
#[test]
fn round43_setprecision_moves_the_display_cap() {
    let rendered = display("setprecision(1000); sqrt(2)");
    assert_eq!(rendered.chars().count(), 1002);
    assert_eq!(
        &rendered[..102],
        display("sqrt(2)").as_str(),
        "the 1000-place rendering starts with the 100-place one"
    );
    // A periodic expansion is rendered in full regardless of the cap.
    assert_eq!(display("1/3"), "0.(3)");
}

/// The reference prints a product's factors in the kernel's canonical order even when a builder
/// appended them in another order (factor() did), and a divided power is not cancelled.
#[test]
fn round43_products_render_in_canonical_order_and_keep_divided_powers() {
    assert_eq!(
        display("x = symbol(\"x\"); factor(x^3 - x)"),
        "x*(x - 1)*(x + 1)"
    );
    assert_eq!(
        display("x = symbol(\"x\"); factor(x^2 - 1)"),
        "(x - 1)*(x + 1)"
    );
    // x^2 * x^-1 is NOT x: the product is undefined at 0 where x^-1 is, so the reference keeps
    // the two powers separate. Only a combined exponent that STAYS negative may merge.
    assert_eq!(
        display("x = symbol(\"x\"); diff(x^2*log(x), x)"),
        "2*x*log(x) + x^2/x"
    );
    assert_eq!(display("x = symbol(\"x\"); x^2/x"), "x^2/x");
    assert_eq!(display("x = symbol(\"x\"); simplify(x/x)"), "x/x");
    assert_eq!(display("x = symbol(\"x\"); x*x^-1"), "x/x");
    assert_eq!(display("x = symbol(\"x\"); x^2*x^-3"), "x^-1");
}

/// An unevaluated integral is the reference's BINDING node (integ (x) operand), exact, with the
/// class-level refusal diagnostic carrying the input-specific reason in its details AND at the top
/// level of the diagnostics array.
#[test]
fn round43_unevaluated_integral_is_the_integral_node_with_its_diagnostics() {
    let value = eval("x = symbol(\"x\"); integrate_full(exp(-x^2), x)");
    let expression = match field(&value, "expression") {
        Value::Symbolic(expression) => expression,
        other => panic!("expression is {}", other.kind_name()),
    };
    assert_eq!(
        expression.canonical(),
        "(integ (x) (fn exp (mul (rat -1 1) (pow (sym x) (rat 2 1)))))"
    );
    assert_eq!(expression.node_count(), 7);
    assert_eq!(field(&value, "exactness").display(), "Exact");
    let Value::Vector(diagnostics) = field(&value, "diagnostics") else {
        panic!("diagnostics must be a vector");
    };
    assert_eq!(
        diagnostics.len(),
        2,
        "the class-level and the input-specific diagnostic"
    );
    assert_eq!(
        field(&diagnostics[0], "code").display(),
        "integration.unevaluated"
    );
    assert_eq!(
        field(&diagnostics[1], "code").display(),
        "integration.no-closed-form"
    );
    let Value::Vector(details) = field(&diagnostics[0], "details") else {
        panic!("details must be a vector");
    };
    assert_eq!(details.len(), 1);
    assert_eq!(
        field(&details[0], "code").display(),
        "integration.no-closed-form"
    );
}

/// The verification token names the equivalence test the oracle's ordered checks closed with:
/// canonical for an atom integrand, expansion for a composite one.
#[test]
fn round43_integration_verification_reports_the_equivalence_test() {
    assert_eq!(
        field(
            &eval("x = symbol(\"x\"); integrate_full(sin(x), x)"),
            "verification_method"
        )
        .display(),
        "differentiation+canonical"
    );
    assert_eq!(
        field(
            &eval("x = symbol(\"x\"); integrate_full(x^2, x)"),
            "verification_method"
        )
        .display(),
        "differentiation+expansion"
    );
}
