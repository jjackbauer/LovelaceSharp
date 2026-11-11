//! Round 39: the language-side symbolics residue - the limit family the oracle evaluates, the
//! integration table and the MathIR version refusal.
//!
//! Expected strings are the oracle's own output for the same script.

use crate::{Engine, LangError, Value};

fn display(script: &str) -> String {
    Engine::new()
        .eval(script)
        .unwrap_or_else(|error| panic!("script failed: {}", error.message))
        .display()
}

fn eval(script: &str) -> Result<Value, LangError> {
    Engine::new().eval(script)
}

/// The `x = symbol("x")` prefix every script below shares.
fn with_x(body: &str) -> String {
    format!("x = symbol(\"x\"); {body}")
}

/// The `value` field of the LimitResult record a `limit(...)` call publishes.
fn limit_value(body: &str) -> Value {
    match eval(&with_x(body)).unwrap_or_else(|e| panic!("{body}: {}", e.message)) {
        Value::Record(_, fields) => fields
            .into_iter()
            .find(|(name, _)| name == "value")
            .map(|(_, v)| v)
            .expect("the record has a value field"),
        other => panic!("expected a LimitResult record, got {}", other.kind_name()),
    }
}

/// IndeterminateExponentialLimitTests: the three probes and the exact-exponent family the
/// oracle evaluates to a closed form.
#[test]
fn indeterminate_exponential_limits_answer_the_exact_closed_form() {
    assert_eq!(limit_value("limit((1 + 1/x)^x, x, inf)").display(), "e");
    assert_eq!(limit_value("limit((1 + x)^(1/x), x, 0)").display(), "e");
    assert_eq!(limit_value("limit((1 + 2/x)^x, x, inf)").display(), "e^2");
    assert_eq!(limit_value("limit((1 + 1/x)^(3*x), x, inf)").display(), "e^3");
    assert_eq!(limit_value("limit((1 + 1/x)^x, x, -inf)").display(), "e");
    assert_eq!(limit_value("limit((1 + 3/x)^(x/2), x, inf)").display(), "e^(3/2)");
    assert_eq!(
        display("x = symbol(\"x\"); a = symbol(\"a\"); limit((1 + x)^(a/x), x, 0).value"),
        "e^a"
    );
    let record = display("x = symbol(\"x\"); limit_full((1 + 1/x)^x, x, inf)");
    assert!(
        record.contains("status: Value") && record.contains("exactness: AlgebraicExact"),
        "the closed form crosses the wire as AlgebraicExact: {record}"
    );
}

/// The indeterminate shapes are still REFUSED with a reason: answering the value at the point
/// would publish a wrong number with exists: true.
#[test]
fn indeterminate_exponential_shapes_stay_unevaluated_with_a_reason() {
    for body in [
        "limit((2 + x)^(1/x), x, 0)",
        "limit((1 + 1/x)^(x^2), x, inf)",
        "limit((2^(1/x))^x, x, 0)",
        "limit((1 + x)^(1/x) - 1, x, 0)",
    ] {
        let record = eval(&with_x(body)).unwrap_or_else(|e| panic!("{body}: {}", e.message));
        let text = record.display();
        assert!(
            text.starts_with("LimitResult(status: Unevaluated"),
            "{body} was answered: {text}"
        );
        assert!(text.contains("limit.unevaluated"), "{body} carries no diagnostic: {text}");
    }
    let text = eval(&with_x("limit((1 + 1/x)^(x^2), x, inf)")).expect("record").display();
    assert!(
        text.contains("the indeterminate factor"),
        "the cofactor refusal carries the reference reason: {text}"
    );
}

/// The determined limits keep exactly their previous answers, and the substitution-shaped ones
/// still answer immediately.
#[test]
fn determined_limits_keep_their_answers() {
    assert_eq!(limit_value("limit(sin(x)/x, x, 0)").display(), "1");
    assert_eq!(limit_value("limit(x^2 + 1, x, 2)").display(), "5");
    assert_eq!(limit_value("limit((1 + x)^2, x, 0)").display(), "1");
    assert_eq!(limit_value("limit(2^x, x, 3)").display(), "8");
    assert_eq!(limit_value("limit(x^x, x, 0)").display(), "1");
    assert_eq!(limit_value("limit((x^2 + 1)/(x^2 - 1), x, inf)").display(), "1");
    assert_eq!(
        display("x = symbol(\"x\"); limit(1/x, x, 0)"),
        "LimitResult(status: DoesNotExist, exists: False, value: , left: -inf, left_conditions: [x < 0], right: inf, right_conditions: [x > 0], conditions: [], exactness: Exact, diagnostics: [])"
    );
}

/// AcceptanceTests.Integrate_Table_AllVerify: every table row integrates to the reference
/// antiderivative AND verifies by differentiation.
#[test]
fn integration_table_rows_are_exact_and_verify() {
    let cases = [
        ("integrate(x^7, x)", "1/8*x^8"),
        ("integrate(3*x^2 + 2*x + 1, x)", "x^3 + x^2 + x"),
        ("integrate(1/(x^2 - 1), x)", "-1/2*log(x + 1) + 1/2*log(x - 1)"),
        ("integrate(sin(x), x)", "-cos(x)"),
        ("integrate(exp(3*x), x)", "1/3*exp(3*x)"),
        ("integrate(2*x*cos(x^2), x)", "sin(x^2)"),
        ("integrate(x*exp(x), x)", "exp(x)*(x - 1)"),
    ];
    for (call, expected) in cases {
        assert_eq!(display(&with_x(call)), expected, "{call}");
    }
    for integrand in ["x^7", "3*x^2 + 2*x + 1", "sin(x)", "exp(3*x)", "x*exp(x)"] {
        let call = format!("integrate_full({integrand}, x).verified");
        assert_eq!(
            display(&with_x(&call)),
            "True",
            "the table entry for {integrand} does not verify"
        );
    }
    assert_eq!(
        display(&with_x("integrate(exp(-(x^2)), x)")),
        "integrate(exp(-x^2), x)"
    );
}

/// KernelHardeningTests.MathIR_Deserialize_RejectsOldVersion: the rejection names the version it
/// found and the one it expects, as the reference FormatException does.
#[test]
fn mathir_old_version_is_rejected_with_the_reference_message() {
    let error = eval("evalir(\"#!mathir 1\\nparam x\\n\", [1], 10)")
        .expect_err("the old version must be refused");
    assert_eq!(error.code, "InvalidInput");
    assert_eq!(error.category, "ParseError");
    assert_eq!(
        error.message,
        "Unsupported MathIR format version '1\nparam x\n' (expected 2)."
    );
    // a version the reader cannot see is quoted verbatim, exactly as the reference quotes it
    let error = eval("evalir(\"#!mathir 3\", [1], 10)").expect_err("version 3 must be refused");
    assert_eq!(error.message, "Unsupported MathIR format version '3' (expected 2).");
}

/// DxSemanticClosureTests.Solve_QuarticWithRealRoots_DefaultDomain_KeepsRootOfRoots: the
/// biquadratic x^4 - 5*x^2 is Solved with the exact radicals and the zero root, not three RootOf
/// values. The zero root used to hide behind the rational-root theorem, which cannot see a zero
/// constant term, so the degree-4 RootOf path took the polynomial.
#[test]
fn solve_quartic_with_real_roots_keeps_the_exact_radicals() {
    let record = display("x = symbol(\"x\"); solve(x^4 - 5*x^2 == 0, x)");
    assert!(record.starts_with("SolveResult(status: Solved"), "{record}");
    assert!(record.contains("completeness: Complete"), "{record}");
    assert!(record.contains("Solution(value: -1/2*sqrt(20)"), "{record}");
    assert!(record.contains("Solution(value: 1/2*sqrt(20)"), "{record}");
    assert!(record.contains("Solution(value: 0"), "{record}");
    assert!(record.contains("represented_count: 3"), "{record}");
    assert!(record.contains("unrepresented_count: 0"), "{record}");
    // the degree-4 case that is still not representable stays honest: Partial, two RootOf values
    let mixed = display("x = symbol(\"x\"); solve(x^4 - x^2 - 1 == 0, x)");
    assert!(mixed.starts_with("SolveResult(status: Partial"), "{mixed}");
    assert!(mixed.contains("unrepresented_count: 2"), "{mixed}");
}
