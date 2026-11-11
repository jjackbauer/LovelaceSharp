//! Round 45: the deferred census rows the closed differential now answers. Every expected value is
//! the C# case's own contract (Lovelace.Run.Tests / Lovelace.Symbolics.Tests) and, where the case
//! asserts a wire value, the oracle's own output for the same script. Each test drives the runner's
//! OWN execute/envelope_for pair through parity_batch::run_args, so an assertion cannot pass on a
//! flow the binary does not run.

use super::parity_batch::*;
use lovelace_json::Json;

fn result(env: &Json) -> &Json {
    get(env, "result")
}

fn structured(env: &Json) -> &Json {
    get(result(env), "structured")
}

fn diagnostic_fields<'a>(diagnostic: &'a Json, name: &str) -> &'a Json {
    record_field(diagnostic, name)
}

/// AbsSymbolicRoundTripTests.cs:56 `Diff_Of_Abs_Symbolic_DoesNotThrow`: d|x|/dx is the guarded sign
/// branch plus the unevaluated derivative, and the canonical form is the reference's own.
#[test]
fn diff_of_abs_symbolic_carries_the_nonzero_guard() {
    let r = envelope("x = symbol(\"x\"); diff(abs(x), x)", &[]);
    assert_eq!(r.exit, 0, "{}", r.env);
    assert!(boolean(get(&r.env, "ok")));
    let value = result(&r.env);
    assert_eq!(text(get(value, "kind")), "Symbolic");
    assert_eq!(
        text(get(value, "display")),
        "piecewise(sign(x) if x != 0, diff(abs(x), x))"
    );
    let st = structured(&r.env);
    assert_eq!(
        text(get(st, "canonical")),
        "(pw ((ne (sym x) (rat 0 1)) (fn sign (sym x))) (der (x) (fn abs (sym x))))"
    );
}

/// DxSemanticClosureTests.cs:303 `Language_SolveQuartic_ReportsUnevaluatedNote`: the record's note
/// is a structured Diagnostic with the frozen code and the reference's exact message.
#[test]
fn solve_quartic_reports_the_unevaluated_diagnostic() {
    let r = envelope("x = symbol(\"x\"); solve(x^4 + 1 == 0, x)", &[]);
    assert_eq!(r.exit, 0, "{}", r.env);
    assert!(boolean(get(&r.env, "ok")));
    let record = structured(&r.env);
    assert_eq!(text(get(record, "kind")), "Record");
    assert_eq!(text(get(record, "type")), "SolveResult");
    assert_eq!(
        text(get(record_field(record, "status"), "value")),
        "Unevaluated"
    );
    assert_eq!(
        text(get(record_field(record, "complete"), "value")),
        "false"
    );
    let diagnostics = array(get(record_field(record, "diagnostics"), "elements"));
    let diagnostic = diagnostics.first().expect("one diagnostic");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        text(get(diagnostic_fields(diagnostic, "code"), "value")),
        "solve.unevaluated"
    );
    assert_eq!(
        text(get(diagnostic_fields(diagnostic, "message"), "value")),
        "complex algebraic roots not supported (RootOf is real-only in v1)."
    );
}

/// DxStructuredResultsTests.cs:413 `InvFull_ExposesDetCondition`: the matrix inverse record is
/// Solved and carries the det(a) != 0 condition — a one-element Symbolic relation, typed Vector.
#[test]
fn inv_full_exposes_the_determinant_condition() {
    let r = envelope(
        "x = symbol(\"x\"); y = symbol(\"y\"); inv_full([[x, 0], [0, y]])",
        &[],
    );
    assert_eq!(r.exit, 0, "{}", r.env);
    assert!(boolean(get(&r.env, "ok")));
    let record = structured(&r.env);
    assert_eq!(text(get(record, "type")), "MatrixInverseResult");
    assert_eq!(text(get(record_field(record, "status"), "value")), "Solved");
    let conditions = record_field(record, "conditions");
    assert_eq!(text(get(conditions, "kind")), "Array");
    assert_eq!(text(get(conditions, "type")), "Vector");
    let elements = array(get(conditions, "elements"));
    assert_eq!(elements.len(), 1);
    assert_eq!(text(get(&elements[0], "kind")), "Symbolic");
    assert_eq!(text(get(&elements[0], "pretty")), "x*y != 0");
    let rendered = text(get(result(&r.env), "display"));
    assert!(
        rendered.contains("conditions: [x*y != 0]"),
        "the record rendering carries the condition: {rendered}"
    );
}

/// SolveCompletenessPairingTests.cs:84 `Unevaluated_IsNotComplete`: an unevaluated solve publishes
/// complete=false / completeness=Unknown, never Complete.
#[test]
fn solve_full_unevaluated_is_not_complete() {
    let r = envelope("x = symbol(\"x\"); solve_full(x^4 + 1 == 0, x)", &[]);
    assert_eq!(r.exit, 0, "{}", r.env);
    let record = structured(&r.env);
    assert_eq!(
        text(get(record_field(record, "status"), "value")),
        "Unevaluated"
    );
    assert_eq!(
        text(get(record_field(record, "complete"), "value")),
        "false"
    );
    assert_eq!(
        text(get(record_field(record, "completeness"), "value")),
        "Unknown"
    );
}

/// SeriesLaurentAndBranchPointTests.cs:89 `Series_OfARemovableDenominator_IsUnchanged`: the control
/// expansion keeps exactly its previous text, and no branch-point artefact crosses the wire.
#[test]
fn series_of_a_removable_denominator_is_unchanged() {
    let r = envelope("x = symbol(\"x\"); series(1/(x - 1), x, 0, 2)", &[]);
    assert_eq!(r.exit, 0, "{}", r.env);
    assert_eq!(text(get(result(&r.env), "display")), "-x + O(x^2) - 1");
    let wire = r.env.to_string();
    assert!(!wire.contains("sqrt(0)"), "{wire}");
    assert!(!wire.contains("/0"), "{wire}");
}

/// CapabilitiesBuiltinTests.cs:554 `IntegrationRefusal_IsTypedStructuredAndClassLevel`: every
/// unevaluated integral is a record whose TOP-LEVEL diagnostics carry the class code first and the
/// input-specific code second, with the same reason on both and distinct reasons per integrand.
#[test]
fn integration_refusal_is_typed_structured_and_class_level() {
    let integrands = ["exp(x^2)", "sin(x^2)", "1/(x^5+1)"];
    let mut messages = Vec::new();
    let mut reasons = Vec::new();
    for integrand in integrands {
        let script = format!("x = symbol(\"x\"); integrate_full({integrand}, x);");
        let r = envelope(&script, &[]);
        assert_eq!(r.exit, 0, "{script}: {}", r.env);
        assert!(boolean(get(&r.env, "ok")), "{script}");
        let record = structured(&r.env);
        assert_eq!(text(get(record, "kind")), "Record");
        assert_eq!(text(get(record, "type")), "IntegrationResult");
        assert_eq!(
            text(get(record_field(record, "status"), "value")),
            "Unevaluated"
        );
        let top = array(get(record_field(record, "diagnostics"), "elements"));
        assert_eq!(top.len(), 2, "{script}: {record}");
        let class = &top[0];
        assert_eq!(
            text(get(diagnostic_fields(class, "code"), "value")),
            "integration.unevaluated"
        );
        assert_eq!(
            text(get(diagnostic_fields(class, "category"), "value")),
            "UnsupportedOperation"
        );
        let class_message = text(get(diagnostic_fields(class, "message"), "value"));
        assert!(!class_message.is_empty());
        let details = array(get(diagnostic_fields(class, "details"), "elements"));
        assert_eq!(details.len(), 1);
        let detail = &details[0];
        assert_eq!(
            text(get(diagnostic_fields(detail, "code"), "value")),
            "integration.no-closed-form"
        );
        assert_eq!(
            text(get(diagnostic_fields(detail, "category"), "value")),
            "UnsupportedOperation"
        );
        let reason = text(get(diagnostic_fields(detail, "message"), "value"));
        assert!(!reason.is_empty());
        assert!(reason.contains("integrate("), "{reason}");
        let specific = &top[1];
        assert_eq!(
            text(get(diagnostic_fields(specific, "code"), "value")),
            "integration.no-closed-form"
        );
        assert_eq!(
            text(get(diagnostic_fields(specific, "category"), "value")),
            "UnsupportedOperation"
        );
        assert_eq!(
            text(get(diagnostic_fields(specific, "message"), "value")),
            reason
        );
        assert!(array(get(diagnostic_fields(specific, "details"), "elements")).is_empty());
        messages.push(class_message.to_string());
        reasons.push(reason.to_string());
    }
    messages.sort();
    messages.dedup();
    assert_eq!(messages.len(), 1);
    reasons.sort();
    reasons.dedup();
    assert_eq!(reasons.len(), integrands.len());
}

/// HyperbolicBuiltinsAndNegativeIntegerPowersTests.cs:384 `ZeroToTheMinusOne...`: the zero base is
/// a typed, recoverable refusal that names the base, the diagnostic repeats it, and the session
/// survives the failure.
#[test]
fn zero_to_the_minus_one_is_a_typed_recoverable_error_that_names_the_base() {
    let r = envelope("0^-1", &[]);
    assert_eq!(r.exit, 1, "{}", r.env);
    assert!(!boolean(get(&r.env, "ok")));
    assert_eq!(text(get(&r.env, "code")), "InvalidArgument");
    assert_eq!(text(get(&r.env, "category")), "TypeMismatch");
    assert_eq!(
        text(get(&r.env, "message")),
        "Base cannot be zero. (Parameter 'base')"
    );
    assert!(!text(get(&r.env, "message")).contains("exponent"));
    assert!(boolean(get(&r.env, "recoverable")));
    let diagnostics = array(get(&r.env, "diagnostics"));
    assert_eq!(
        text(get(&diagnostics[0], "message")),
        "Base cannot be zero. (Parameter 'base')"
    );
    let recovered = envelope("1 + 1", &[]);
    assert_eq!(recovered.exit, 0, "{}", recovered.env);
    assert_eq!(text(get(result(&recovered.env), "display")), "2");
    assert_eq!(text(get(result(&recovered.env), "typed")), "2 (Natural)");
}

/// AcceptanceTests.cs:Hessian_Computes — the Hessian of `(x^2+y^2)*exp(-(x+y))` is a 2x2 matrix
/// whose mixed partials are the SAME symbolic node (canonical form included).
#[test]
fn hessian_computes_a_symmetric_matrix() {
    let r = envelope(
        "x = symbol(\"x\"); y = symbol(\"y\"); hessian((x^2 + y^2)*exp(-(x + y)), [x, y])",
        &[],
    );
    assert_eq!(r.exit, 0, "{}", r.env);
    assert!(boolean(get(&r.env, "ok")));
    let st = structured(&r.env);
    assert_eq!(text(get(st, "kind")), "Array");
    let shape = array(get(st, "shape"));
    assert_eq!(shape.len(), 2);
    assert_eq!(number(&shape[0]), 2);
    assert_eq!(number(&shape[1]), 2);
    let elements = array(get(st, "elements"));
    assert_eq!(elements.len(), 4);
    for element in elements {
        assert_eq!(text(get(element, "kind")), "Symbolic");
    }
    let lower = &elements[1];
    let upper = &elements[2];
    assert_eq!(
        text(get(lower, "canonical")),
        text(get(upper, "canonical")),
        "mixed partials must be the same node"
    );
    assert_eq!(text(get(lower, "pretty")), text(get(upper, "pretty")));
}

/// ArrayTests.cs:195 `CaptureState_ReportsArrayKind`: the captured variable's kind is Array, and
/// the published structured form is the 2x2 array it denotes.
#[test]
fn capture_state_reports_the_array_kind() {
    let r = run_args(&[
        "--eval",
        "m = [[1, 2], [3, 4]]",
        "--json",
        "--omit-functions",
    ]);
    assert_eq!(r.exit, 0, "{}", r.env);
    let variables = array(get(&r.env, "variables"));
    let m = variables
        .iter()
        .find(|v| text(get(v, "name")) == "m")
        .expect("the variable m is captured");
    assert_eq!(text(get(m, "kind")), "Array");
    let st = get(m, "structured");
    assert_eq!(text(get(st, "kind")), "Array");
    let shape = array(get(st, "shape"));
    assert_eq!(shape.len(), 2);
    assert_eq!(number(&shape[0]), 2);
    assert_eq!(number(&shape[1]), 2);
}

/// StructuredRealExactnessTests.cs:103 `TruncatedQuotientAtTheBudget_IsNotCertifiedExact`: the
/// 18-digit truncation is exactly that text and carries no exactness claim or fraction.
#[test]
fn truncated_quotient_at_the_budget_is_not_certified_exact() {
    let r = envelope("setprecision(18); 1/1009", &[]);
    assert_eq!(r.exit, 0, "{}", r.env);
    let st = structured(&r.env);
    assert_eq!(text(get(st, "value")), "0.000991080277502477");
    assert!(
        !boolean(get(st, "exact")),
        "the truncation is not exact: {st}"
    );
    assert!(opt(st, "numerator").is_none(), "{st}");
    assert!(opt(st, "denominator").is_none(), "{st}");
}

/// UsageDocumentationTests.cs:DocumentedExample_MatchesEngine#61 — the guide's own
/// `lovelace`/`result` pair, evaluated through the runner and compared to the documented typed
/// rendering byte for byte.
#[test]
fn documented_example_61_matches_the_engine() {
    let r = envelope(
        r#"x = symbol("x")
y = symbol("y")
solve_system([x^2 + y^2 - 1 == 0, x*y == 0], [x, y])"#,
        &[],
    );
    assert_eq!(r.exit, 0, "#61: {}", r.env);
    assert!(boolean(get(&r.env, "ok")));
    assert_eq!(
        text(get(get(&r.env, "result"), "typed")),
        r#"SystemSolveResult(status: Solved, domain: complex, complete: True, completeness: Complete, solutions: [SystemSolution(bindings: [Binding(name: x, value: 0), Binding(name: y, value: -1)], conditions: [], exactness: Exact), SystemSolution(bindings: [Binding(name: x, value: -1), Binding(name: y, value: 0)], conditions: [], exactness: Exact), SystemSolution(bindings: [Binding(name: x, value: 1), Binding(name: y, value: 0)], conditions: [], exactness: Exact), SystemSolution(bindings: [Binding(name: x, value: 0), Binding(name: y, value: 1)], conditions: [], exactness: Exact)], diagnostics: []) (SystemSolveResult)"#,
        "#61"
    );
}

/// UsageDocumentationTests.cs:DocumentedExample_MatchesEngine#70 — the guide's own
/// `lovelace`/`result` pair, evaluated through the runner and compared to the documented typed
/// rendering byte for byte.
#[test]
fn documented_example_70_matches_the_engine() {
    let r = envelope(
        r#"x = symbol("x")
k = optimize(x^5 + 2*x^4 + 3*x^3 + x^2, [x])
lower(k, [x])"#,
        &[],
    );
    assert_eq!(r.exit, 0, "#70: {}", r.env);
    assert!(boolean(get(&r.env, "ok")));
    assert_eq!(
        text(get(get(&r.env, "result"), "typed")),
        r#"#!mathir 2
param x
const (rat 2 1)
const (rat 1 1)
const (rat 3 1)
Parameter 0 0
Constant 0 0
PowInt 0 0,1
Constant 0 1
Constant 0 2
Constant 0 0
Add 0 5,0
Mul 0 0,6
Add 0 4,7
Mul 0 0,8
Add 0 3,9
Mul 0 2,10"#,
        "#70"
    );
}

/// UsageDocumentationTests.cs:DocumentedExample_MatchesEngine#71 — the guide's own
/// `lovelace`/`result` pair, evaluated through the runner and compared to the documented typed
/// rendering byte for byte.
#[test]
fn documented_example_71_matches_the_engine() {
    let r = envelope(
        r#"x = symbol("x")
k = optimize(x^5 + 2*x^4 + 3*x^3 + x^2, [x])
ir = lower(k, [x])
evalir(ir, [2], 40)"#,
        &[],
    );
    assert_eq!(r.exit, 0, "#71: {}", r.env);
    assert!(boolean(get(&r.env, "ok")));
    assert_eq!(
        text(get(get(&r.env, "result"), "typed")),
        r#"92 (Integer)"#,
        "#71"
    );
}

/// UsageDocumentationTests.cs:DocumentedExample_MatchesEngine#73 — the guide's own
/// `lovelace`/`result` pair, evaluated through the runner and compared to the documented typed
/// rendering byte for byte.
#[test]
fn documented_example_73_matches_the_engine() {
    let r = envelope(
        r#"x = symbol("x")
assume(and(x > 0, x < 5))
assume(not(x == 2))
assumptions()"#,
        &[],
    );
    assert_eq!(r.exit, 0, "#73: {}", r.env);
    assert!(boolean(get(&r.env, "ok")));
    assert_eq!(
        text(get(get(&r.env, "result"), "typed")),
        r#"AssumptionSet(display: x > 0; x < 5; x != 2, assumptions: [x > 0, x < 5, x != 2]) (AssumptionSet)"#,
        "#73"
    );
}

/// UsageDocumentationTests.cs:DocumentedExample_MatchesEngine#99 — the guide's own
/// `lovelace`/`result` pair, evaluated through the runner and compared to the documented typed
/// rendering byte for byte.
#[test]
fn documented_example_99_matches_the_engine() {
    let r = envelope(
        r#"x = symbol("x")
y = symbol("y")
linsolve_full([[x, 1], [0, y]], [0, 1]).conditions"#,
        &[],
    );
    assert_eq!(r.exit, 0, "#99: {}", r.env);
    assert!(boolean(get(&r.env, "ok")));
    assert_eq!(
        text(get(get(&r.env, "result"), "typed")),
        r#"[x*y != 0] (Vector)"#,
        "#99"
    );
}
