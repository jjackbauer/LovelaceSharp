//! Ports from Lovelace.Symbolics.Tests.EnumValueEmissionTests: every enum-valued field of a rich
//! result record crosses as a first-class Enum carrying the DECLARED type name.

use super::parity_batch::*;
use super::*;

fn eval_structured(script: &str) -> Json {
    let r = envelope(script, &[]);
    assert_eq!(r.exit, 0, "'{script}' exited {}: {}", r.exit, r.stdout);
    get(get(&r.env, "result"), "structured").clone()
}

fn assert_enum(field: &Json, type_name: &str, member: &str) {
    assert_eq!(text(get(field, "kind")), "Enum");
    assert_eq!(text(get(field, "type")), type_name);
    assert_eq!(text(get(field, "value")), member);
}

#[test]
fn enum_fields_cross_as_declared_enum_values() {
    let value = eval_structured("x = symbol(\"x\"); solve_full(x^2 - 4 == 0, x)");
    assert_enum(record_field(&value, "status"), "SolveStatus", "Solved");
    assert_enum(
        record_field(&value, "completeness"),
        "Completeness",
        "Complete",
    );
    let solution = &array(get(record_field(&value, "solutions"), "elements"))[0];
    assert_enum(
        record_field(solution, "exactness"),
        "SolutionExactness",
        "Exact",
    );

    let partial = eval_structured("x = symbol(\"x\"); solve_full(x^4 - x^2 - 1 == 0, x)");
    assert_enum(record_field(&partial, "status"), "SolveStatus", "Partial");
    assert_enum(
        record_field(&partial, "completeness"),
        "Completeness",
        "Partial",
    );
    assert_eq!(
        text(get(record_field(&partial, "complete"), "kind")),
        "Boolean"
    );

    let family = eval_structured("x = symbol(\"x\"); solve_full(sin(x) == 0, x)");
    let family = &array(get(record_field(&family, "families"), "elements"))[0];
    assert_enum(
        record_field(family, "exactness"),
        "SolutionExactness",
        "ParametricExact",
    );

    let system = eval_structured(
        "x = symbol(\"x\"); y = symbol(\"y\"); solve_system_full([x + y == 1, x - y == 3], [x, y])",
    );
    assert_enum(record_field(&system, "status"), "SolveStatus", "Solved");
    let system_solution = &array(get(record_field(&system, "solutions"), "elements"))[0];
    assert_enum(
        record_field(system_solution, "exactness"),
        "SolutionExactness",
        "Exact",
    );

    let limit = eval_structured("x = symbol(\"x\"); limit_full(1/x, x, 0)");
    assert_enum(
        record_field(&limit, "status"),
        "LimitStatus",
        "DoesNotExist",
    );
    assert_enum(
        record_field(&limit, "exactness"),
        "SolutionExactness",
        "Exact",
    );
    let limit_value = eval_structured("x = symbol(\"x\"); limit_full(sin(x)/x, x, 0)");
    assert_enum(record_field(&limit_value, "status"), "LimitStatus", "Value");

    let integral = eval_structured("x = symbol(\"x\"); integrate_full(x^2, x)");
    assert_enum(
        record_field(&integral, "status"),
        "IntegrationStatus",
        "SolvedExact",
    );
    assert_enum(
        record_field(&integral, "exactness"),
        "SolutionExactness",
        "Exact",
    );

    let transform = eval_structured("x = symbol(\"x\"); simplify_full(sin(x)^2 + cos(x)^2)");
    assert_enum(
        record_field(&transform, "status"),
        "TransformStatus",
        "Satisfied",
    );
    let step = &array(get(record_field(&transform, "steps"), "elements"))[0];
    assert_enum(
        record_field(step, "classification"),
        "RuleClassification",
        "Universal",
    );
    assert_eq!(text(get(record_field(step, "rule_id"), "kind")), "Text");

    for (script, expected) in [
        (
            "x = symbol(\"x\"); type(solve_full(x^2 - 4 == 0, x).status)",
            "SolveStatus",
        ),
        (
            "x = symbol(\"x\"); type(solve_full(x^2 - 4 == 0, x).completeness)",
            "Completeness",
        ),
        (
            "x = symbol(\"x\"); type(simplify_full(x/x).status)",
            "TransformStatus",
        ),
        (
            "x = symbol(\"x\"); type(simplify_full(x/x).steps[0].classification)",
            "RuleClassification",
        ),
    ] {
        let r = envelope(script, &[]);
        assert_eq!(r.exit, 0);
        assert_eq!(text(get(get(&r.env, "result"), "display")), expected);
    }
}
