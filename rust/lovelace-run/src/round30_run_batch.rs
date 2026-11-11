//! Round-30 ports of the two SolveEmptySetTests rows the round-29 folded-relation fix made
//! answerable: the earlier port checked the status fields but not the record's own kind/type or
//! the diagnostic's category, which the C# rows assert. These tests repeat the FULL C# assertion.
//!
//! Every test drives the runner's own execute/envelope_for pair through parity_batch::run_args,
//! so an assertion cannot pass on a flow the binary does not run.

use super::parity_batch::*;
use lovelace_json::Json;

fn with_large_stack<T: Send + 'static>(probe: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(probe)
        .expect("probe thread")
        .join()
        .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
}

fn run_script(script: &str) -> RunResult {
    let script = script.to_string();
    with_large_stack(move || {
        run_args(&[
            "--eval",
            &script,
            "--json",
            "--omit-functions",
            "--omit-variables",
        ])
    })
}

fn field<'a>(st: &'a Json, name: &str) -> &'a Json {
    for f in array(get(st, "fields")) {
        if text(get(f, "name")) == name {
            return get(f, "value");
        }
    }
    panic!("structured record has no field '{name}': {}", st)
}

/// SolveEmptySetTests.Solve_OfAProvablyFalseEquation_AnswersTheEmptySet_NotADomainError.
#[test]
fn solve_of_a_provably_false_equation_publishes_the_solve_result_record() {
    let r = run_script("x = symbol(\"x\"); solve(0 == 1, x)");
    assert_eq!(
        r.exit, 0,
        "solve(0 == 1, x) must answer, not fail: {}",
        r.env
    );
    assert!(boolean(get(&r.env, "ok")));

    let record = get(get(&r.env, "result"), "structured");
    assert_eq!(text(get(record, "kind")), "Record");
    assert_eq!(text(get(record, "type")), "SolveResult");

    assert_eq!(text(get(field(record, "status"), "value")), "NoSolutions");
    assert_eq!(text(get(field(record, "complete"), "value")), "true");

    let diagnostics = array(get(field(record, "diagnostics"), "elements"));
    assert!(
        !diagnostics.is_empty(),
        "the empty set reports why it is empty"
    );
    let diagnostic = &diagnostics[0];
    assert_eq!(
        text(get(field(diagnostic, "code"), "value")),
        "solve.no-solutions"
    );
    assert_eq!(
        text(get(field(diagnostic, "message"), "value")),
        "the equation reduces to a nonzero constant."
    );
}

/// SolveEmptySetTests.SolveFull_OfAProvablyFalseEquation_ReportsNoSolutionsComplete.
#[test]
fn solve_full_of_a_provably_false_equation_publishes_no_solutions_complete() {
    let r = run_script("x = symbol(\"x\"); solve_full(2 == 3, x)");
    assert_eq!(
        r.exit, 0,
        "solve_full(2 == 3, x) must answer, not fail: {}",
        r.env
    );

    let record = get(get(&r.env, "result"), "structured");
    assert_eq!(text(get(record, "kind")), "Record");
    assert_eq!(text(get(record, "type")), "SolveResult");

    let status = field(record, "status");
    assert_eq!(text(get(status, "kind")), "Enum");
    assert_eq!(text(get(status, "type")), "SolveStatus");
    assert_eq!(text(get(status, "value")), "NoSolutions");

    assert_eq!(text(get(field(record, "complete"), "value")), "true");

    let completeness = field(record, "completeness");
    assert_eq!(text(get(completeness, "type")), "Completeness");
    assert_eq!(text(get(completeness, "value")), "Complete");

    let solutions = field(record, "solutions");
    assert_eq!(
        number(&array(get(solutions, "shape"))[0]),
        0,
        "solutions shape[0]"
    );

    let diagnostics = array(get(field(record, "diagnostics"), "elements"));
    assert!(
        !diagnostics.is_empty(),
        "a NoSolutions answer reports why the set is empty"
    );
    let diagnostic = &diagnostics[0];
    let category = field(diagnostic, "category");
    assert_eq!(text(get(category, "kind")), "Enum");
    assert_eq!(text(get(category, "type")), "ErrorCategory");
    assert_eq!(text(get(category, "value")), "NoSolution");
    assert_eq!(
        text(get(field(diagnostic, "code"), "value")),
        "solve.no-solutions"
    );
}

// ===========================================================================
// Round-30 (continued): complex closed forms, inspect #4, argument shapes,
// and the limit short-form message.
// ===========================================================================

fn code_of(r: &RunResult) -> &str {
    text(get(&r.env, "code"))
}

fn category_of(r: &RunResult) -> &str {
    text(get(&r.env, "category"))
}

fn message_of(r: &RunResult) -> &str {
    text(get(&r.env, "message"))
}

fn structured_of(r: &RunResult) -> &Json {
    get(get(&r.env, "result"), "structured")
}

fn canonical_of(value: &Json) -> String {
    text(get(value, "canonical")).to_string()
}

/// The `value` field of a Text/Enum/Boolean/Integer structured value.
fn value_of(value: &Json) -> &str {
    text(get(value, "value"))
}

// ---------------------------------------------------------------------------
// ComplexClosedFormTests: the two solution-set rows
// ---------------------------------------------------------------------------

/// QuadraticWithNegativeDiscriminant_YieldsTheExactComplexSolutionSet — x^2 + 1 == 0 has the two
/// exact roots i and -i (SymPy: solve(x**2 + 1, x) = [-I, I]), and substituting either expands to
/// exactly zero.
#[test]
fn complex_quadratic_with_negative_discriminant_yields_the_exact_complex_solution_set() {
    let r = run_script("x = symbol(\"x\"); solve_full(x^2 + 1 == 0, x)");
    assert!(boolean(get(&r.env, "ok")), "{}", r.env.to_string());
    let st = structured_of(&r);
    assert_eq!(value_of(field(st, "status")), "Solved");
    assert_eq!(value_of(field(st, "completeness")), "Complete");
    assert_eq!(value_of(field(st, "unrepresented_count")), "0");

    let mut values: Vec<String> = array(get(field(st, "solutions"), "elements"))
        .iter()
        .map(|s| canonical_of(field(s, "value")))
        .collect();
    values.sort();
    assert_eq!(
        values,
        vec!["(i)".to_string(), "(mul (rat -1 1) (i))".to_string()]
    );

    for root in ["(-1)^(1/2)", "-(-1)^(1/2)"] {
        let check = run_script(&format!(
            "x = symbol(\"x\"); expand(subs(x^2 + 1, x, {root}))"
        ));
        assert!(boolean(get(&check.env, "ok")), "{}", check.env.to_string());
        assert_eq!(
            text(get(get(&check.env, "result"), "display")),
            "0",
            "x^2 + 1 at {root}"
        );
    }
}

/// QuadraticWithNegativeDiscriminant_CoversANonZeroRealPart — x^2 + 4 == 0 and x^2 + 2x + 5 == 0
/// publish the exact unexpanded quotient (-b +/- sqrt(disc)) / 2a, and every root expands to zero.
#[test]
fn complex_quadratic_covers_a_non_zero_real_part() {
    let four = run_script("x = symbol(\"x\"); solve_full(x^2 + 4 == 0, x)");
    let st = structured_of(&four);
    assert_eq!(value_of(field(st, "status")), "Solved");
    let mut values: Vec<String> = array(get(field(st, "solutions"), "elements"))
        .iter()
        .map(|s| canonical_of(field(s, "value")))
        .collect();
    values.sort();
    assert_eq!(
        values,
        vec![
            "(mul (rat -2 1) (i))".to_string(),
            "(mul (rat 2 1) (i))".to_string()
        ]
    );

    let shifted = run_script("x = symbol(\"x\"); solve_full(x^2 + 2*x + 5 == 0, x)");
    let st = structured_of(&shifted);
    assert_eq!(value_of(field(st, "status")), "Solved");
    assert_eq!(value_of(field(st, "unrepresented_count")), "0");
    let mut values: Vec<String> = array(get(field(st, "solutions"), "elements"))
        .iter()
        .map(|s| canonical_of(field(s, "value")))
        .collect();
    values.sort();
    assert_eq!(
        values,
        vec![
            "(mul (rat 1 2) (add (rat -2 1) (mul (rat -4 1) (i))))".to_string(),
            "(mul (rat 1 2) (add (rat -2 1) (mul (rat 4 1) (i))))".to_string(),
        ]
    );

    for (root, polynomial) in [
        ("(4*(-1)^(1/2) - 2)/2", "x^2 + 2*x + 5"),
        ("(-4*(-1)^(1/2) - 2)/2", "x^2 + 2*x + 5"),
        ("2*(-1)^(1/2)", "x^2 + 4"),
    ] {
        let check = run_script(&format!(
            "x = symbol(\"x\"); expand(subs({polynomial}, x, {root}))"
        ));
        assert!(boolean(get(&check.env, "ok")), "{}", check.env.to_string());
        assert_eq!(
            text(get(get(&check.env, "result"), "display")),
            "0",
            "{polynomial} at {root}"
        );
    }
}

// ---------------------------------------------------------------------------
// InspectionExactnessTests #4: "(-4)^(1/2)"
// ---------------------------------------------------------------------------

/// Inspect_AnswersExactnessAsABoolean("(-4)^(1/2)", true) — the evaluation itself is the fix: the
/// negative base under the principal square root now folds to the exact 2*i, and inspect publishes
/// the Symbolic record over it.
#[test]
fn inspection_answers_exactness_as_a_boolean_for_a_negative_base_half_power() {
    let r = run_script("inspect((-4)^(1/2))");
    assert!(boolean(get(&r.env, "ok")), "{}", r.env.to_string());
    let st = structured_of(&r);
    assert_eq!(text(get(st, "type")), "Inspection");
    assert_eq!(value_of(field(st, "type")), "Symbolic");
    assert_eq!(text(get(field(st, "domain"), "domain")), "complex");
    assert_eq!(value_of(field(st, "exact")), "true");
    assert_eq!(value_of(field(st, "canonical")), "(mul (rat 2 1) (i))");
    assert_eq!(value_of(field(st, "pretty")), "2*i");
    assert!(array(get(field(st, "free_symbols"), "elements")).is_empty());

    let value = run_script("(-4)^(1/2)");
    assert!(boolean(get(&value.env, "ok")), "{}", value.env.to_string());
    assert_eq!(text(get(get(&value.env, "result"), "display")), "2*i");
}

// ---------------------------------------------------------------------------
// SystemBuiltinArgumentShapeTests
// ---------------------------------------------------------------------------

fn assert_documented_shape_refusal(script: &str, builtin: &str, position: &str, expected: &str) {
    let r = run_script(script);
    assert_eq!(r.exit, 1, "{script}: a refusal exits 1");
    assert!(!boolean(get(&r.env, "ok")), "{script}");
    assert_eq!(
        code_of(&r),
        "InvalidArgument",
        "{script}: {}",
        message_of(&r)
    );
    assert_eq!(
        category_of(&r),
        "TypeMismatch",
        "{script}: {}",
        message_of(&r)
    );
    assert!(boolean(get(&r.env, "recoverable")), "{script}: recoverable");
    let prefix = format!("{builtin}(): argument {position} must be {expected}");
    assert!(
        message_of(&r).starts_with(&prefix),
        "{script}: '{}' does not start with '{prefix}'",
        message_of(&r)
    );
    assert!(
        message_of(&r).contains("; got "),
        "{script}: '{}'",
        message_of(&r)
    );
    assert!(
        !message_of(&r).contains("cast"),
        "{script}: '{}'",
        message_of(&r)
    );
}

/// WrongShapedArgument_IsARecoverableArgumentError_NeverAnInternalFailure (the six theory rows).
#[test]
fn system_builtins_refuse_a_wrong_shaped_argument() {
    let declare = "x = symbol(\"x\"); y = symbol(\"y\"); ";
    let equations = "a list of equations, e.g. [x + y == 2, x - y == 0]";
    let expressions = "a list of expressions, e.g. [x*y, x + y]";
    let symbols = "a list of parameter symbols, e.g. [x, y]";
    for (call, builtin, position, expected) in [
        (
            "solve_system(x + y == 2, x - y == 0)",
            "solve_system",
            "1",
            equations,
        ),
        (
            "solve_system_full(x + y == 2, x - y == 0)",
            "solve_system_full",
            "1",
            equations,
        ),
        ("jacobian(x*y, [x, y])", "jacobian", "1", expressions),
        (
            "solve_system([x + y == 2], x)",
            "solve_system",
            "2",
            symbols,
        ),
        (
            "solve_system_full([x + y == 2], x)",
            "solve_system_full",
            "2",
            symbols,
        ),
        ("jacobian([x*y], x)", "jacobian", "2", symbols),
    ] {
        let script = format!("{declare}{call}");
        assert_documented_shape_refusal(&script, builtin, position, expected);
        assert!(
            message_of(&run_script(&script)).ends_with("got Symbolic."),
            "{script}: '{}'",
            message_of(&run_script(&script))
        );
    }
}

/// ElementThatIsNotAnExpression_IsAlsoAReportedArgumentShapeViolation — the vector IS the right
/// kind, so the refusal names the position and reports the arrived kind Vector.
#[test]
fn system_builtins_refuse_an_element_that_is_not_an_expression() {
    let script = "x = symbol(\"x\"); y = symbol(\"y\"); solve_system([inspect(x^2)], [x, y])";
    assert_documented_shape_refusal(
        script,
        "solve_system",
        "1",
        "a list of equations, e.g. [x + y == 2, x - y == 0]",
    );
    assert!(
        message_of(&run_script(script)).ends_with("got Vector."),
        "{}",
        message_of(&run_script(script))
    );
}

/// TheDeclaredListForm_StillSucceeds — the positive control on the same three builtins.
#[test]
fn the_declared_system_list_forms_still_succeed() {
    for script in [
        "x = symbol(\"x\"); y = symbol(\"y\"); solve_system([x + y == 2, x - y == 0], [x, y])",
        "x = symbol(\"x\"); y = symbol(\"y\"); solve_system_full([x + y == 2, x - y == 0], [x, y])",
        "x = symbol(\"x\"); y = symbol(\"y\"); jacobian([x*y, x + y], [x, y])",
    ] {
        let r = run_script(script);
        assert!(boolean(get(&r.env, "ok")), "{script}: {}", r.env);
    }
}

// ---------------------------------------------------------------------------
// BuiltinArgumentShapeSweepTests: the audit's five reproductions
// ---------------------------------------------------------------------------

/// TheAuditsFiveReproductions_CrossAsTheDocumentedArgumentError.
#[test]
fn audit_reproductions_cross_as_the_documented_argument_error() {
    for (call, builtin, expected) in [
        ("det(1)", "det", "a square matrix"),
        ("matmul(1, 1)", "matmul", "an array or vector"),
        ("symbol(1)", "symbol", "a text name such as \"x\""),
        (
            "x = symbol(\"x\"); assume(1)",
            "assume",
            "a relation such as x > 5",
        ),
        ("transpose(1, 1)", "transpose", "an array or vector"),
    ] {
        assert_documented_shape_refusal(call, builtin, "1", expected);
        assert!(
            message_of(&run_script(call)).ends_with("got Natural."),
            "{call}: '{}'",
            message_of(&run_script(call))
        );
    }
}

/// TheDeclaredForm_StaysCallable — the guard refuses the shape, it does not disable the builtin.
#[test]
fn the_declared_audit_forms_stay_callable() {
    for (script, expected) in [
        ("det([[1, 2], [3, 4]])", "-2"),
        ("matmul([[1, 2], [3, 4]], [[5], [6]])", "[[17], [39]]"),
        ("transpose([[1, 2], [3, 4]])", "[[1, 3], [2, 4]]"),
        ("trace([[1, 2], [3, 4]])", "5"),
        ("symbol(\"x\")", "x"),
        ("x = symbol(\"x\"); assume(x > 5)", "x > 5"),
        // The reference resolves a Text name to the symbol of that name (DeclaredForm #8).
        ("x = symbol(\"x\"); assume_real(\"y\")", "y"),
    ] {
        let r = run_script(script);
        assert!(boolean(get(&r.env, "ok")), "{script}: {}", r.env);
        assert_eq!(
            text(get(get(&r.env, "result"), "display")),
            expected,
            "{script}"
        );
    }
}

// ---------------------------------------------------------------------------
// LimitShortFormRecordTests + DiagnosticContractTests: the symbolic leading coefficient
// ---------------------------------------------------------------------------

/// Refusal_CarriesTheCodeAndCategoryTheStatementAdvertises — the a/x rows report the symbolic
/// leading coefficient, the sin(x)-at-infinity controls keep the point-evaluation message.
#[test]
fn limit_short_form_reports_the_symbolic_leading_coefficient() {
    for call in [
        "limit(a/x, x, 0)",
        "limit_left(a/x, x, 0)",
        "limit_right(a/x, x, 0)",
    ] {
        let script = format!("a = symbol(\"a\"); x = symbol(\"x\"); {call}");
        let r = run_script(&script);
        assert!(boolean(get(&r.env, "ok")), "{script}: {}", r.env);
        let st = structured_of(&r);
        assert_eq!(value_of(field(st, "status")), "Unevaluated", "{call}");
        assert_eq!(text(get(field(st, "exists"), "kind")), "Null", "{call}");
        let diagnostics = array(get(field(st, "diagnostics"), "elements"));
        assert_eq!(diagnostics.len(), 1, "{call}");
        let diagnostic = &diagnostics[0];
        assert_eq!(text(get(diagnostic, "type")), "Diagnostic", "{call}");
        assert_eq!(
            value_of(field(diagnostic, "code")),
            "limit.unevaluated",
            "{call}"
        );
        assert_eq!(
            value_of(field(diagnostic, "category")),
            "UnsupportedOperation",
            "{call}"
        );
        assert_eq!(
            value_of(field(diagnostic, "message")),
            "leading coefficient is symbolic",
            "{call}"
        );
        assert_eq!(value_of(field(diagnostic, "recoverable")), "true", "{call}");
    }

    for call in ["limit(sin(x), x, inf)", "limit_left(sin(x), x, inf)"] {
        let script = format!("x = symbol(\"x\"); {call}");
        let r = run_script(&script);
        let st = structured_of(&r);
        let diagnostics = array(get(field(st, "diagnostics"), "elements"));
        assert_eq!(
            value_of(field(&diagnostics[0], "message")),
            "coefficient does not evaluate at the point",
            "{call}"
        );
    }
}

/// The record is still the SAME one limit_full publishes, field for field.
#[test]
fn limit_short_form_is_the_same_record_as_the_full_form() {
    let shared = "a = symbol(\"a\"); x = symbol(\"x\"); ";
    let short = run_script(&format!("{shared}limit(a/x, x, 0)"));
    let full = run_script(&format!("{shared}limit_full(a/x, x, 0)"));
    assert_eq!(
        structured_of(&short).to_string(),
        structured_of(&full).to_string(),
        "the short form must publish the full form's record"
    );
}
