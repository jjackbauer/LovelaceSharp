//! Ports from Lovelace.Symbolics.Tests: DxStructuredResults, SolveCompleteness*,
//! SystemSolveRecord, EnumValueEmission and ExactnessFlag language-surface rows.

use super::parity_batch::*;
use lovelace_json::Json;

fn eval(script: &str) -> RunResult {
    envelope(script, &[])
}

fn structured(r: &RunResult) -> &Json {
    get(get(&r.env, "result"), "structured")
}

fn enum_field<'a>(field: &'a Json, type_name: &str) -> &'a str {
    assert_eq!(text(get(field, "kind")), "Enum");
    assert_eq!(text(get(field, "type")), type_name);
    text(get(field, "value"))
}

// ---------------------------------------------------------------------------
// DxStructuredResultsTests — the type vocabulary
// ---------------------------------------------------------------------------

#[test]
fn dx_type_uses_one_vocabulary_for_every_kind() {
    let rows = [
        ("1", "Natural"),
        ("1/2", "Real"),
        ("1.5", "Real"),
        ("[1, 2]", "Vector"),
        ("[[1, 2], [3, 4]]", "Array"),
        ("symbol(\"x\")", "Symbolic"),
        ("1 > 0", "Boolean"),
        ("\"text\"", "Text"),
        ("complex", "Domain"),
        (
            "solve_full(symbol(\"z\")^2 - 4 == 0, symbol(\"z\"))",
            "SolveResult",
        ),
        (
            "solve_full(symbol(\"z\")^2 - 4 == 0, symbol(\"z\")).status",
            "SolveStatus",
        ),
    ];
    for (source, expected) in rows {
        let r = eval(&format!("type({source})"));
        assert_eq!(r.exit, 0, "type({source})");
        assert_eq!(
            text(get(get(&r.env, "result"), "display")),
            expected,
            "type({source})"
        );
    }
}

// ---------------------------------------------------------------------------
// DxStructuredResultsTests — record facts
// ---------------------------------------------------------------------------

#[test]
fn dx_integration_result_reports_status_and_verification() {
    let r = eval("x = symbol(\"x\"); integrate_full(x^2, x)");
    assert_eq!(r.exit, 0);
    let value = structured(&r);
    assert_eq!(text(get(value, "type")), "IntegrationResult");
    assert_eq!(
        enum_field(record_field(value, "status"), "IntegrationStatus"),
        "SolvedExact"
    );
    assert_eq!(text(get(record_field(value, "verified"), "value")), "true");
    assert_eq!(
        text(get(record_field(value, "expression"), "pretty")),
        "1/3*x^3"
    );
    assert_eq!(text(get(record_field(value, "method"), "value")), "table");
    assert!(
        text(get(record_field(value, "verification_method"), "value"))
            .starts_with("differentiation+")
    );
}

#[test]
fn dx_optimization_result_reports_costs() {
    let r = eval("x = symbol(\"x\"); optimize_full(x^2 + 2*x + 1, [x])");
    assert_eq!(r.exit, 0);
    let value = structured(&r);
    assert_eq!(text(get(value, "type")), "OptimizationResult");
    assert_eq!(
        text(get(record_field(value, "policy"), "value")),
        "cse+horner+precision-aware"
    );
    assert_eq!(
        text(get(record_field(value, "transformations"), "kind")),
        "Array"
    );
    assert_eq!(
        text(get(record_field(value, "estimated_cost_before"), "kind")),
        "Integer"
    );
}

#[test]
fn dx_type_and_inspect_are_structural() {
    for (script, expected) in [
        ("x = symbol(\"x\"); type(x)", "Symbolic"),
        (
            "x = symbol(\"x\"); type(solve_full(x^2 - 4 == 0, x))",
            "SolveResult",
        ),
    ] {
        let r = eval(script);
        assert_eq!(text(get(get(&r.env, "result"), "display")), expected);
    }
    let r = eval("x = symbol(\"x\"); inspect(x^2 + 1)");
    let value = structured(&r);
    assert_eq!(text(get(value, "type")), "Inspection");
    assert_eq!(text(get(record_field(value, "type"), "value")), "Symbolic");
    assert_eq!(text(get(record_field(value, "domain"), "kind")), "Domain");
    assert_eq!(
        text(get(record_field(value, "domain"), "domain")),
        "complex"
    );
    let free = record_field(value, "free_symbols");
    assert_eq!(text(get(free, "kind")), "Array");
    assert_eq!(array(get(free, "elements")).len(), 1);
    assert_eq!(text(get(record_field(value, "pretty"), "value")), "x^2 + 1");
    assert_eq!(
        text(get(record_field(value, "canonical"), "value")),
        "(add (rat 1 1) (pow (sym x) (rat 2 1)))"
    );
    assert_eq!(text(get(record_field(value, "shape"), "kind")), "Null");
    assert_eq!(text(get(record_field(value, "rank"), "kind")), "Null");
    assert_eq!(
        text(get(record_field(value, "element_domain"), "kind")),
        "Domain"
    );
    assert_eq!(
        text(get(record_field(value, "assumptions"), "kind")),
        "Array"
    );
}

#[test]
fn dx_jacobian_and_hessian_preserve_matrix_shape() {
    let r = eval("x = symbol(\"x\"); y = symbol(\"y\"); jacobian([x*y, x+y], [x, y])");
    assert_eq!(r.exit, 0);
    assert_eq!(text(get(structured(&r), "kind")), "Array");
    assert_eq!(
        array(get(structured(&r), "shape"))
            .iter()
            .map(number)
            .collect::<Vec<_>>(),
        vec![2, 2]
    );
    assert_eq!(
        text(get(get(&r.env, "result"), "typed")),
        "[[y, x], [1, 1]] (Array)"
    );

    let h = eval("x = symbol(\"x\"); y = symbol(\"y\"); hessian(x*y, [x, y])");
    assert_eq!(h.exit, 0);
    assert_eq!(text(get(structured(&h), "kind")), "Array");
    assert_eq!(
        array(get(structured(&h), "shape"))
            .iter()
            .map(number)
            .collect::<Vec<_>>(),
        vec![2, 2]
    );
    assert_eq!(
        text(get(get(&h.env, "result"), "typed")),
        "[[0, 1], [1, 0]] (Array)"
    );
}

#[test]
fn dx_linsolve_full_exposes_the_det_condition() {
    let r = eval("x = symbol(\"x\"); y = symbol(\"y\"); linsolve_full([[x, 1], [0, y]], [0, 1])");
    assert_eq!(r.exit, 0);
    let value = structured(&r);
    assert_eq!(text(get(value, "type")), "MatrixSolveResult");
    assert_eq!(
        enum_field(record_field(value, "status"), "SolveStatus"),
        "Solved"
    );
    let conditions = record_field(value, "conditions");
    assert_eq!(text(get(conditions, "kind")), "Array");
    let elements = array(get(conditions, "elements"));
    assert_eq!(elements.len(), 1);
    assert_eq!(text(get(&elements[0], "pretty")), "x*y != 0");
}

#[test]
fn dx_solve_system_full_emits_binding_records() {
    let r = eval(
        "x = symbol(\"x\"); y = symbol(\"y\"); solve_system_full([x + y == 1, x - y == 3], [x, y])",
    );
    assert_eq!(r.exit, 0);
    let value = structured(&r);
    assert_eq!(text(get(value, "type")), "SystemSolveResult");
    let solutions = array(get(record_field(value, "solutions"), "elements"));
    assert_eq!(solutions.len(), 1);
    assert_eq!(text(get(&solutions[0], "type")), "SystemSolution");
    let bindings = array(get(record_field(&solutions[0], "bindings"), "elements"));
    assert_eq!(bindings.len(), 2);
    let names: Vec<&str> = bindings
        .iter()
        .map(|b| text(get(record_field(b, "name"), "canonical")))
        .collect();
    let values: Vec<&str> = bindings
        .iter()
        .map(|b| text(get(record_field(b, "value"), "pretty")))
        .collect();
    assert_eq!(names, vec!["(sym x)", "(sym y)"]);
    assert_eq!(values, vec!["2", "-1"]);
}

#[test]
fn dx_solve_system_full_carries_domain_complete_and_exactness() {
    let r = eval(
        "x = symbol(\"x\"); y = symbol(\"y\"); solve_system_full([x + y == 1, x - y == 3], [x, y])",
    );
    let value = structured(&r);
    let names: Vec<&str> = array(get(value, "fields"))
        .iter()
        .map(|f| text(get(f, "name")))
        .collect();
    assert_eq!(
        names,
        vec![
            "status",
            "domain",
            "complete",
            "completeness",
            "solutions",
            "diagnostics"
        ]
    );
    assert_eq!(text(get(record_field(value, "domain"), "kind")), "Domain");
    assert_eq!(
        text(get(record_field(value, "domain"), "domain")),
        "complex"
    );
    assert_eq!(
        text(get(record_field(value, "complete"), "kind")),
        "Boolean"
    );
    assert_eq!(text(get(record_field(value, "complete"), "value")), "true");
    assert_eq!(
        enum_field(record_field(value, "completeness"), "Completeness"),
        "Complete"
    );
    let solution = &array(get(record_field(value, "solutions"), "elements"))[0];
    assert_eq!(text(get(solution, "type")), "SystemSolution");
    assert_eq!(
        enum_field(record_field(solution, "exactness"), "SolutionExactness"),
        "Exact"
    );
    assert_eq!(
        text(get(record_field(solution, "conditions"), "kind")),
        "Array"
    );
}

#[test]
fn dx_solve_system_full_every_solution_carries_exactness_and_bindings() {
    let r = eval(
        "x = symbol(\"x\"); y = symbol(\"y\"); solve_system_full([x^2 + y^2 - 1 == 0, x*y == 0], [x, y])",
    );
    assert_eq!(r.exit, 0);
    let value = structured(&r);
    let solutions = array(get(record_field(value, "solutions"), "elements"));
    assert_eq!(solutions.len(), 4);
    for solution in solutions {
        assert_eq!(text(get(solution, "type")), "SystemSolution");
        assert_eq!(
            enum_field(record_field(solution, "exactness"), "SolutionExactness"),
            "Exact"
        );
        let first = &array(get(record_field(solution, "bindings"), "elements"))[0];
        assert_eq!(text(get(first, "type")), "Binding");
    }
}

#[test]
fn dx_solve_full_family_parameter_is_symbolic_and_domain_is_a_domain() {
    let r = eval("x = symbol(\"x\"); solve_full(sin(x) == 0, x)");
    assert_eq!(r.exit, 0);
    let family = &array(get(record_field(structured(&r), "families"), "elements"))[0];
    assert_eq!(text(get(family, "type")), "SolutionFamily");
    let parameter = record_field(family, "parameter");
    assert_eq!(text(get(parameter, "kind")), "Symbolic");
    assert_eq!(text(get(parameter, "pretty")), "k");
    assert_eq!(text(get(parameter, "canonical")), "(sym k)");
    let domain = record_field(family, "parameter_domain");
    assert_eq!(text(get(domain, "kind")), "Domain");
    assert_eq!(text(get(domain, "domain")), "integer");
}

#[test]
fn dx_solve_full_variable_is_symbolic_not_the_name_string() {
    let r = eval("x = symbol(\"x\"); solve_full(x^2 - 4 == 0, x)");
    let variable = record_field(structured(&r), "variable");
    assert_eq!(text(get(variable, "kind")), "Symbolic");
    assert_eq!(text(get(variable, "pretty")), "x");
    assert_eq!(text(get(variable, "canonical")), "(sym x)");
}

// ---------------------------------------------------------------------------
// SolveCompletenessPairingTests / SolveCompletenessMappingTests
// ---------------------------------------------------------------------------

fn solve_pair(script: &str) -> (String, String, String) {
    let r = envelope(script, &[]);
    assert_eq!(r.exit, 0, "{script}");
    let value = structured(&r);
    (
        text(get(record_field(value, "status"), "value")).to_string(),
        text(get(record_field(value, "complete"), "value")).to_string(),
        text(get(record_field(value, "completeness"), "value")).to_string(),
    )
}

#[test]
fn solve_completeness_no_solutions_is_a_complete_answer() {
    for script in [
        "x = symbol(\"x\"); solve_full((x^2-1)/(x^2-1) == 0, x)",
        "x = symbol(\"x\"); solve_full(x/x == 0, x)",
        "x = symbol(\"x\"); solve_full(x^2 + 1 == 0, x, real)",
    ] {
        let (status, complete, completeness) = solve_pair(script);
        assert_eq!(status, "NoSolutions", "{script}");
        assert_eq!(complete, "true", "{script}");
        assert_eq!(completeness, "Complete", "{script}");
    }
}

#[test]
fn solve_completeness_partial_and_solved_pairs() {
    let (status, complete, completeness) =
        solve_pair("x = symbol(\"x\"); solve_full(x^4 - x^2 - 1 == 0, x)");
    assert_eq!(
        (status.as_str(), complete.as_str(), completeness.as_str()),
        ("Partial", "false", "Partial")
    );
    let unrepresented = envelope("x = symbol(\"x\"); solve_full(x^4 - x^2 - 1 == 0, x)", &[]);
    let value = structured(&unrepresented);
    assert_eq!(
        text(get(record_field(value, "unrepresented_count"), "value")),
        "2"
    );

    let (status, complete, completeness) =
        solve_pair("x = symbol(\"x\"); solve_full(x^2 - 4 == 0, x)");
    assert_eq!(
        (status.as_str(), complete.as_str(), completeness.as_str()),
        ("Solved", "true", "Complete")
    );
}

#[test]
fn solve_completeness_language_statuses_report_the_mapping_pair() {
    let rows = [
        ("solve_full(x^2 - 4 == 0, x)", "Solved", "true", "Complete"),
        (
            "solve_full(x^4 - x^2 - 1 == 0, x)",
            "Partial",
            "false",
            "Partial",
        ),
        (
            "solve_full((x^2-1)/(x^2-1) == 0, x)",
            "NoSolutions",
            "true",
            "Complete",
        ),
        ("solve_full(x/x == 0, x)", "NoSolutions", "true", "Complete"),
        (
            "solve_full(x^2 + 1 == 0, x, real)",
            "NoSolutions",
            "true",
            "Complete",
        ),
    ];
    for (expression, status, complete, completeness) in rows {
        let (s, c, k) = solve_pair(&format!("x = symbol(\"x\"); {expression}"));
        assert_eq!(
            (s.as_str(), c.as_str(), k.as_str()),
            (status, complete, completeness),
            "{expression}"
        );
    }
}

// ---------------------------------------------------------------------------
// SystemSolveRecordTests
// ---------------------------------------------------------------------------

#[test]
fn system_solve_record_has_the_declared_schema() {
    let r = envelope(
        "x = symbol(\"x\"); y = symbol(\"y\"); solve_system([x + y == 2, x - y == 0], [x, y])",
        &[],
    );
    assert_eq!(r.exit, 0);
    let value = structured(&r);
    assert_eq!(text(get(value, "type")), "SystemSolveResult");
    let names: Vec<&str> = array(get(value, "fields"))
        .iter()
        .map(|f| text(get(f, "name")))
        .collect();
    assert_eq!(
        names,
        vec![
            "status",
            "domain",
            "complete",
            "completeness",
            "solutions",
            "diagnostics"
        ]
    );
    assert_eq!(
        enum_field(record_field(value, "status"), "SolveStatus"),
        "Solved"
    );
    assert_eq!(
        text(get(record_field(value, "status"), "type")),
        "SolveStatus"
    );
    assert_eq!(text(get(record_field(value, "domain"), "kind")), "Domain");
    assert_eq!(
        text(get(record_field(value, "complete"), "kind")),
        "Boolean"
    );
    assert_eq!(text(get(record_field(value, "complete"), "value")), "true");
    assert_eq!(
        enum_field(record_field(value, "completeness"), "Completeness"),
        "Complete"
    );
    let solutions = array(get(record_field(value, "solutions"), "elements"));
    assert_eq!(solutions.len(), 1);
    assert_eq!(text(get(&solutions[0], "type")), "SystemSolution");
    let solution = &solutions[0];
    assert_eq!(
        text(get(record_field(solution, "bindings"), "kind")),
        "Array"
    );
    assert_eq!(
        array(get(record_field(solution, "bindings"), "elements")).len(),
        2
    );
    assert_eq!(
        text(get(record_field(solution, "conditions"), "kind")),
        "Array"
    );
    assert_eq!(
        text(get(record_field(solution, "exactness"), "kind")),
        "Enum"
    );
    let diagnostics = record_field(value, "diagnostics");
    assert_eq!(text(get(diagnostics, "kind")), "Array");
    assert!(opt(diagnostics, "value").is_none());
    assert!(array(get(diagnostics, "elements")).is_empty());
}

#[test]
fn system_solve_two_call_forms_publish_the_same_record_shape() {
    let system = "[x + y == 2, x - y == 0], [x, y]";
    let first = envelope(
        &format!("x = symbol(\"x\"); y = symbol(\"y\"); solve_system({system})"),
        &[],
    );
    let second = envelope(
        &format!("x = symbol(\"x\"); y = symbol(\"y\"); solve_system_full({system})"),
        &[],
    );
    assert_eq!(first.exit, 0);
    assert_eq!(second.exit, 0);
    assert_eq!(structured(&first), structured(&second));
}

// ---------------------------------------------------------------------------
// ExactnessFlagTests — elementary functions of exact arguments are exact
// ---------------------------------------------------------------------------

#[test]
fn exactness_elementary_function_of_an_exact_argument_is_exact() {
    let names = [
        "sin", "cos", "tan", "asin", "acos", "atan", "exp", "log", "sinh", "cosh", "tanh",
    ];
    for name in names {
        let r = envelope(&format!("x = symbol(\"x\"); inspect({name}(x)).exact"), &[]);
        assert_eq!(r.exit, 0, "{name}(x)");
        let value = get(get(&r.env, "result"), "structured");
        assert_eq!(text(get(value, "kind")), "Boolean", "{name}(x)");
        assert_eq!(text(get(value, "value")), "true", "{name}(x) must be exact");
    }
}

/// DxStructuredResultsTests.cs:349 — inspect([[x, 1], [2, 3]]) publishes the ARRAY's own
/// structural metadata: type Array, shape [2, 2] as a Vector, rank 2, element_domain complex.
/// (The record rebuild that made scalars report their own kind also made this row reachable.)
#[test]
fn dx_inspect_array_shape_rank_and_element_domain_are_structural() {
    let r = eval("x = symbol(\"x\"); inspect([[x, 1], [2, 3]])");
    assert_eq!(r.exit, 0);
    let value = structured(&r);
    assert_eq!(text(get(record_field(value, "type"), "value")), "Array");

    let shape = record_field(value, "shape");
    assert_eq!(text(get(shape, "kind")), "Array");
    assert_eq!(text(get(shape, "type")), "Vector");
    assert_eq!(
        array(get(shape, "shape"))
            .iter()
            .map(number)
            .collect::<Vec<_>>(),
        vec![2]
    );
    let elements = array(get(shape, "elements"));
    assert_eq!(elements.len(), 2);
    for element in elements {
        assert_eq!(text(get(element, "value")), "2", "shape element");
    }
    assert!(
        text(get(get(&r.env, "result"), "display")).contains("shape: [2, 2]"),
        "the record renders the shape as [2, 2]"
    );

    assert_eq!(
        text(get(record_field(value, "rank"), "value")),
        "2",
        "array rank"
    );

    let domain = record_field(value, "element_domain");
    assert_eq!(text(get(domain, "kind")), "Domain");
    assert_eq!(text(get(domain, "domain")), "complex");
}
