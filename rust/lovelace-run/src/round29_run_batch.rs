//! Round-29 ports of the Run.Tests rows the integer-argument, inspection and solve fixes
//! unblock.
//!
//! Every test drives the runner's own execute/envelope_for pair through parity_batch::run_args,
//! so an assertion cannot pass on a flow the binary does not run. Each test repeats the C# row's
//! concrete assertions: exit code, code/category, message grammar, and the record fields.

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
    with_large_stack(move || run_args(&["--eval", &script, "--json", "--omit-functions"]))
}

fn code(r: &RunResult) -> &str {
    text(get(&r.env, "code"))
}

fn category(r: &RunResult) -> &str {
    text(get(&r.env, "category"))
}

fn message(r: &RunResult) -> &str {
    text(get(&r.env, "message"))
}

fn structured(r: &RunResult) -> &Json {
    get(get(&r.env, "result"), "structured")
}

fn field<'a>(st: &'a Json, name: &str) -> &'a Json {
    for f in array(get(st, "fields")) {
        if text(get(f, "name")) == name {
            return get(f, "value");
        }
    }
    panic!("structured record has no field '{name}': {}", st)
}

fn assert_argument_refusal(script: &str, builtin: &str) {
    let r = run_script(script);
    assert_eq!(r.exit, 1, "{script}: a refusal exits 1");
    assert!(!boolean(get(&r.env, "ok")), "{script}");
    assert_eq!(
        code(&r),
        "InvalidArgument",
        "{script}: code ({})",
        message(&r)
    );
    assert_eq!(
        category(&r),
        "TypeMismatch",
        "{script}: category ({})",
        message(&r)
    );
    assert!(boolean(get(&r.env, "recoverable")), "{script}: recoverable");
    assert!(
        message(&r).starts_with(&format!("{builtin}(): ")),
        "{script}: '{}'",
        message(&r)
    );
}

// ---------------------------------------------------------------------------
// IntegerArgumentBoundaryMatrixTests
// ---------------------------------------------------------------------------

/// EveryBoundaryCell_LandsOnTheExactValueOrANamedRefusal — the concat/dft/range rows (12).
#[test]
fn integer_boundary_concat_dft_and_range_refuse_a_non_integral_argument() {
    for literal in ["2.5", "0.5", "2.9999", "(5/2)"] {
        assert_argument_refusal(&format!("concat([1, 2], [3, 4], {literal})"), "concat");
        assert!(
            message(&run_script(&format!("concat([1, 2], [3, 4], {literal})")))
                .contains("argument 3 (axis)"),
            "concat {literal} must name argument 3 (axis)"
        );
        assert_argument_refusal(&format!("dft([1, 2, 3, 4], {literal})"), "dft");
        assert!(
            message(&run_script(&format!("dft([1, 2, 3, 4], {literal})")))
                .contains("argument 2 (n)"),
            "dft {literal} must name argument 2 (n)"
        );
        assert_argument_refusal(&format!("1..{literal}"), "range");
        assert!(
            message(&run_script(&format!("1..{literal}"))).contains("argument 'bound'"),
            "range {literal} must name the bound"
        );
    }
    // An exactly integral Real is refused by KIND: the repair is written in the refusal.
    let r = run_script("1..2.0");
    assert_eq!(code(&r), "InvalidArgument");
    assert!(message(&r).contains("which is integral"), "{}", message(&r));
}

/// The matrix ACCEPTS the exact integers it names, and the accepted value is the value in use.
#[test]
fn integer_boundary_accepted_cells_publish_the_value_the_caller_wrote() {
    // dft pads or truncates to exactly n samples; ignoring n answered a different transform.
    let two = run_script("dft([1, 2, 3, 4], 2)");
    assert!(boolean(get(&two.env, "ok")));
    let elements = array(get(structured(&two), "elements"));
    assert_eq!(elements.len(), 2, "dft(..., 2) is a two-point transform");
    let six = run_script("dft([1, 2, 3, 4], 6)");
    assert_eq!(array(get(structured(&six), "elements")).len(), 6);
    let zero = run_script("dft([1, 2, 3, 4], 0)");
    assert!(boolean(get(&zero.env, "ok")));
    assert!(array(get(structured(&zero), "elements")).is_empty());

    let concat = run_script("concat([1, 2], [3, 4], 0)");
    assert_eq!(text(get(structured(&concat), "type")), "Vector");
    assert_eq!(
        array(get(structured(&concat), "elements")).len(),
        4,
        "axis 0 concatenates, and the axis is not itself an element"
    );
}

/// EveryInventoryTemplate_ReachesTheArgumentItNames (the 7 deferred rows).
#[test]
fn integer_boundary_inventory_templates_reach_the_argument_they_name() {
    for (script, builtin, anchor) in [
        ("concat([1, 2], [3, 4], \"sentinel\")", "concat", "(axis)"),
        ("dft([1, 2, 3, 4], \"sentinel\")", "dft", "(n)"),
        ("1..\"sentinel\"", "range", "'bound'"),
        ("sum([[1, 2], [3, 4]], \"sentinel\")", "sum", "(axis)"),
        ("prod([1, 2, 3], \"sentinel\")", "prod", "(axis)"),
        ("min([1, 2, 3], \"sentinel\")", "min", "(axis)"),
        ("max([1, 2, 3], \"sentinel\")", "max", "(axis)"),
    ] {
        assert_argument_refusal(script, builtin);
        assert!(
            message(&run_script(script)).contains(anchor),
            "{script} must name {anchor}: {}",
            message(&run_script(script))
        );
    }
}

/// The wide-magnitude and negative rows stay typed refusals for dft.
#[test]
fn integer_boundary_dft_named_bounds_are_typed_refusals() {
    for literal in ["-1", "10^30"] {
        let r = run_script(&format!("dft([1, 2, 3, 4], {literal})"));
        assert_eq!(code(&r), "InvalidArgument", "{literal}: {}", message(&r));
        assert_eq!(category(&r), "TypeMismatch");
        assert!(
            message(&r).contains("between 0 and 2147483647"),
            "{}",
            message(&r)
        );
    }
}

// ---------------------------------------------------------------------------
// InspectionExactnessTests
// ---------------------------------------------------------------------------

fn inspect(value_source: &str) -> RunResult {
    run_script(&format!("v = {value_source}; i = inspect(v)"))
}

/// Inspect_AnswersExactnessAsABoolean — the 4 deferred rows, plus the record's own type.
#[test]
fn inspection_answers_exactness_as_a_boolean_for_every_numeric_kind() {
    for (source, expected, kind) in [
        ("sqrt(2)", "false", "Real"),
        ("(-4)^(1/2)", "true", "Symbolic"),
        ("dft([1, 2, 3])[0]", "true", "Complex"),
        ("dft([1, 2, 3])[1]", "false", "Complex"),
    ] {
        let r = inspect(source);
        assert_eq!(r.exit, 0, "inspect({source}): {}", message(&r));
        let record = structured(&r);
        assert_eq!(
            text(get(field(record, "type"), "value")),
            kind,
            "inspect({source}).type"
        );
        let exact = field(record, "exact");
        assert_eq!(
            text(get(exact, "kind")),
            "Boolean",
            "inspect({source}).exact is not Boolean"
        );
        assert_eq!(
            text(get(exact, "value")),
            expected,
            "inspect({source}).exact"
        );
    }
}

/// InspectExact_AgreesWithTheValuesOwnExactFlag — the 3 deferred rows.
#[test]
fn inspection_exact_agrees_with_the_values_own_exact_flag() {
    for source in ["sqrt(2)", "dft([1, 2, 3])[0]", "dft([1, 2, 3])[1]"] {
        let r = inspect(source);
        assert_eq!(r.exit, 0);
        let value = array(get(&r.env, "variables"))
            .iter()
            .find(|v| text(get(v, "name")) == "v")
            .map(|v| get(v, "structured"))
            .expect("the variable form of v is published");
        let record = field(structured(&r), "exact");
        assert_eq!(
            text(get(record, "value")) == "true",
            boolean(get(value, "exact")),
            "inspect({source}).exact disagrees with the value's own exact flag"
        );
    }
}

/// Inspect_KeepsNullWhereExactnessDoesNotApply — the 3 deferred rows.
#[test]
fn inspection_keeps_null_where_exactness_does_not_apply() {
    for source in ["\"hi\"", "is_even(4)", "[1, 2]"] {
        let r = inspect(source);
        assert_eq!(r.exit, 0, "inspect({source}) exited");
        assert_eq!(
            text(get(field(structured(&r), "exact"), "kind")),
            "Null",
            "inspect({source}).exact"
        );
    }
}

/// TruncatedQuotientAtTheBudget_IsNotExactInEitherSurface.
#[test]
fn inspection_truncated_quotient_at_the_budget_is_not_exact() {
    let r = run_script("setprecision(18); v = 1/1009; i = inspect(v)");
    assert_eq!(r.exit, 0);
    let value = array(get(&r.env, "variables"))
        .iter()
        .find(|v| text(get(v, "name")) == "v")
        .map(|v| get(v, "structured"))
        .expect("v is published");
    assert!(
        !boolean(get(value, "exact")),
        "the truncated quotient's own flag is false"
    );
    let exact = field(structured(&r), "exact");
    assert_eq!(text(get(exact, "kind")), "Boolean");
    assert_eq!(text(get(exact, "value")), "false");
}

// ---------------------------------------------------------------------------
// SolveEmptySetTests / SystemSolveRecordTests
// ---------------------------------------------------------------------------

const DECLARE: &str = "x = symbol(\"x\"); y = symbol(\"y\"); ";

fn solve_record(source: &str) -> RunResult {
    let r = run_script(&format!("{DECLARE}{source}"));
    assert_eq!(r.exit, 0, "{source}: {}", message(&r));
    r
}

/// Solve_OfAProvablyFalseEquation_AnswersTheEmptySet_NotADomainError.
#[test]
fn solve_of_a_folded_false_equation_is_the_empty_set() {
    let r = solve_record("solve(0 == 1, x)");
    let record = structured(&r);
    assert_eq!(text(get(field(record, "status"), "value")), "NoSolutions");
    assert_eq!(
        text(get(field(record, "complete"), "value")),
        "true",
        "complete"
    );
    assert_eq!(
        text(get(field(record, "completeness"), "value")),
        "Complete"
    );
    assert!(array(get(field(record, "solutions"), "elements")).is_empty());
    let diagnostics = array(get(field(record, "diagnostics"), "elements"));
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        text(get(field(&diagnostics[0], "code"), "value")),
        "solve.no-solutions"
    );
    assert_eq!(
        text(get(field(&diagnostics[0], "message"), "value")),
        "the equation reduces to a nonzero constant."
    );
}

/// SolveFull_OfAProvablyFalseEquation_ReportsNoSolutionsComplete.
#[test]
fn solve_full_of_a_folded_false_equation_reports_no_solutions_complete() {
    let r = solve_record("solve_full(2 == 3, x)");
    let record = structured(&r);
    assert_eq!(text(get(field(record, "status"), "value")), "NoSolutions");
    assert_eq!(
        text(get(field(record, "complete"), "value")),
        "true",
        "complete"
    );
    assert_eq!(
        text(get(field(record, "completeness"), "value")),
        "Complete"
    );
    assert!(array(get(field(record, "solutions"), "elements")).is_empty());
}

/// SolveSystem_OfAProvablyFalseEquation_AnswersTheEmptySet.
#[test]
fn solve_system_of_a_folded_false_equation_answers_the_empty_set() {
    let r = solve_record("solve_system([0 == 1], [x])");
    let record = structured(&r);
    assert_eq!(text(get(field(record, "status"), "value")), "NoSolutions");
    assert_eq!(
        text(get(field(record, "complete"), "value")),
        "true",
        "complete"
    );
    assert!(array(get(field(record, "solutions"), "elements")).is_empty());
    assert!(array(get(field(record, "diagnostics"), "elements")).is_empty());
}

/// SolveSystem_StatusCompleteAndCompleteness_AreTheDocumentedRow — the sin row.
#[test]
fn solve_system_of_a_non_polynomial_system_is_an_unevaluated_record() {
    let r = solve_record("solve_system([sin(x) == 0], [x])");
    let record = structured(&r);
    assert_eq!(text(get(field(record, "status"), "value")), "Unevaluated");
    assert_eq!(
        text(get(field(record, "complete"), "value")),
        "false",
        "complete"
    );
    assert_eq!(text(get(field(record, "completeness"), "value")), "Unknown");
    assert!(array(get(field(record, "solutions"), "elements")).is_empty());
    let diagnostics = array(get(field(record, "diagnostics"), "elements"));
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        text(get(field(&diagnostics[0], "code"), "value")),
        "system-solve.unevaluated"
    );
    assert_eq!(
        text(get(field(&diagnostics[0], "message"), "value")),
        "system-solve.non-polynomial: only polynomial systems are supported in v1."
    );
}

/// An identity relation is an Unevaluated enumeration, not a refusal.
#[test]
fn solve_of_a_folded_identity_is_unevaluated() {
    let r = solve_record("solve(1 == 1, x)");
    let record = structured(&r);
    assert_eq!(text(get(field(record, "status"), "value")), "Unevaluated");
    assert_eq!(
        text(get(field(record, "complete"), "value")),
        "false",
        "complete"
    );
    let diagnostics = array(get(field(record, "diagnostics"), "elements"));
    assert_eq!(
        text(get(field(&diagnostics[0], "message"), "value")),
        "0 = 0: every value is a solution."
    );
}
