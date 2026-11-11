//! Ports of Lovelace.Suite.Tests.RecordSchemaTests: the record-schema registry must describe the
//! records the kernel ACTUALLY produces. Every assertion is the C# case's own contract, and each
//! test drives the runner's own execute/envelope pair through parity_batch::run_args, so the
//! records validated here are the records the binary publishes.

use std::collections::BTreeSet;

use super::parity_batch::*;
use crate::schemas::RecordSchemas;
use lovelace_json::Json;

/// The structured value of the final result, the object a record schema describes.
fn structured(env: &Json) -> &Json {
    get(get(env, "result"), "structured")
}

/// Collects every record reachable from a structured value, to the reference's depth limit.
fn collect<'a>(value: &'a Json, into: &mut Vec<&'a Json>, depth: usize) {
    if depth > 6 {
        return;
    }
    match value.get("kind").and_then(Json::as_str) {
        Some("Record") => {
            into.push(value);
            for field in array(get(value, "fields")) {
                collect(get(field, "value"), into, depth + 1);
            }
        }
        Some("Array") => {
            for element in array(get(value, "elements")) {
                collect(element, into, depth + 1);
            }
        }
        _ => {}
    }
}

fn assert_schema(type_name: &str, expected: &[(&str, &str)]) {
    let schema = RecordSchemas::for_type(type_name)
        .unwrap_or_else(|| panic!("no schema registered for '{type_name}'"));
    let names: Vec<&str> = schema.fields.iter().map(|f| f.name).collect();
    let kinds: Vec<&str> = schema.fields.iter().map(|f| f.kind).collect();
    let expected_names: Vec<&str> = expected.iter().map(|(name, _)| *name).collect();
    let expected_kinds: Vec<&str> = expected.iter().map(|(_, kind)| *kind).collect();
    assert_eq!(names, expected_names, "{type_name}: field names/order");
    assert_eq!(kinds, expected_kinds, "{type_name}: declared kinds");
    assert_eq!(
        schema.field_names(),
        expected_names,
        "{type_name}: field_names()"
    );
}

fn text_value(value: &str) -> Json {
    Json::obj([
        ("kind", Json::Str("Text".to_string())),
        ("value", Json::Str(value.to_string())),
    ])
}

fn integer_value(value: i64) -> Json {
    Json::obj([
        ("kind", Json::Str("Integer".to_string())),
        ("exact", Json::Bool(true)),
        ("value", Json::Num(value.to_string())),
    ])
}

/// A structured record value in the runner's published shape.
fn record(type_name: &str, fields: &[(&str, Json)]) -> Json {
    Json::obj([
        ("kind", Json::Str("Record".to_string())),
        ("type", Json::Str(type_name.to_string())),
        (
            "fields",
            Json::Arr(
                fields
                    .iter()
                    .map(|(name, value)| {
                        Json::obj([
                            ("name", Json::Str((*name).to_string())),
                            ("value", value.clone()),
                        ])
                    })
                    .collect(),
            ),
        ),
    ])
}

/// RecordSchemaTests.cs:44 EveryProducedRecord_ConformsToItsDeclaredSchema: every record the
/// representative corpus produces validates, and the corpus actually drives every structural kind
/// the C# case names.
#[test]
fn every_produced_record_conforms_to_its_declared_schema() {
    const SCRIPTS: &[&str] = &[
        r#"x = symbol("x"); solve_full(x^2 - 4 == 0, x)"#,
        r#"x = symbol("x"); solve_full(sin(x) == 0, x)"#,
        r#"x = symbol("x"); y = symbol("y"); solve_system_full([x + y == 1, x - y == 3], [x, y])"#,
        r#"x = symbol("x"); y = symbol("y"); solve_system([x + y == 1, x - y == 3], [x, y])"#,
        r#"x = symbol("x"); simplify_full(x/x)"#,
        r#"x = symbol("x", real); simplify_full(sqrt(x^2))"#,
        r#"x = symbol("x"); solve_full(x^4 - x^2 - 1 == 0, x)"#,
        r#"x = symbol("x"); assume(x == 0); simplify_full(x/x)"#,
        r#"x = symbol("x"); limit_full(1/x, x, 0)"#,
        r#"x = symbol("x"); integrate_full(x^2, x)"#,
        r#"x = symbol("x"); optimize_full(x^2 + 2*x + 1, [x])"#,
        r#"x = symbol("x"); compile_full(x^2 + 1, [x])"#,
        r#"x = symbol("x"); inspect(x^2 + 1)"#,
        r#"x = symbol("x"); y = symbol("y"); inv_full([[x, 1], [0, y]])"#,
        r#"x = symbol("x"); inv_full([[x, x], [x, x]])"#,
        r#"x = symbol("x"); y = symbol("y"); linsolve_full([[x, 1], [0, y]], [0, 1])"#,
        r#"x = symbol("x"); linsolve_full([[x, x], [x, x]], [1, 1])"#,
    ];

    let envelopes: Vec<RunResult> = SCRIPTS.iter().map(|script| envelope(script, &[])).collect();
    let mut records: Vec<&Json> = Vec::new();
    for r in &envelopes {
        assert_eq!(r.exit, 0, "{}", r.env);
        assert!(boolean(get(&r.env, "ok")), "{}", r.env);
        collect(structured(&r.env), &mut records, 0);
    }

    assert!(!records.is_empty(), "the corpus produced no record");
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for record in &records {
        assert_eq!(text(get(record, "kind")), "Record");
        seen.insert(text(get(record, "type")));
        let violations = RecordSchemas::validate(record);
        assert!(
            violations.is_empty(),
            "schema drift:\n  {}\nrecord types seen: {seen:?}",
            violations.join("\n  ")
        );
    }

    // The corpus must exercise the schemas, not pass because nothing was compared.
    for required in [
        "SolveResult",
        "Solution",
        "SolutionFamily",
        "SystemSolveResult",
        "SystemSolution",
        "Binding",
        "TransformResult",
        "RewriteStep",
        "LimitResult",
        "IntegrationResult",
        "OptimizationResult",
        "CompilationResult",
        "ParameterInfo",
        "Inspection",
        "MatrixInverseResult",
        "MatrixSolveResult",
        "Diagnostic",
    ] {
        assert!(
            seen.contains(required),
            "the corpus did not produce record type '{required}'; seen: {seen:?}"
        );
    }
}

/// RecordSchemaTests.cs:117 UnknownRecordType_IsReportedAsUnknown_NotAsConforming.
#[test]
fn unknown_record_type_is_reported_as_unknown_not_as_conforming() {
    let value = record("NotARealResult", &[("field", text_value("value"))]);
    assert!(RecordSchemas::for_type("NotARealResult").is_none());
    assert!(RecordSchemas::validate(&value).is_empty());
}

/// RecordSchemaTests.cs:125 Validate_ReportsMissingAndUndeclaredFields.
#[test]
fn validate_reports_missing_and_undeclared_fields() {
    let schema = RecordSchemas::for_type("ParameterInfo").expect("ParameterInfo is registered");
    let wrong = record(
        "ParameterInfo",
        &[("name", text_value("x")), ("extra", integer_value(1))],
    );
    let violations = schema.validate(&wrong);
    assert!(
        violations
            .iter()
            .any(|v| v.contains("missing declared field 'domain'"))
    );
    assert!(
        violations
            .iter()
            .any(|v| v.contains("undeclared field 'extra'"))
    );
    assert_eq!(
        violations,
        vec![
            "ParameterInfo: missing declared field 'domain'".to_string(),
            "ParameterInfo: undeclared field 'extra'".to_string(),
        ]
    );
}

/// RecordSchemaTests.cs:147 SolverSchemas_DeclareTheStructuralKinds: field names, order and
/// declared kinds are the machine contract a consumer reads.
#[test]
fn solver_schemas_declare_the_structural_kinds() {
    assert_schema(
        "SystemSolveResult",
        &[
            ("status", "Enum"),
            ("domain", "Domain"),
            ("complete", "Boolean"),
            ("completeness", "Enum"),
            ("solutions", "Array"),
            ("diagnostics", "Array"),
        ],
    );
    assert_schema(
        "SystemSolution",
        &[
            ("bindings", "Array"),
            ("conditions", "Array"),
            ("exactness", "Enum"),
        ],
    );
    assert_schema("Binding", &[("name", "Symbolic"), ("value", "Symbolic")]);
    assert_schema(
        "SolveResult",
        &[
            ("status", "Enum"),
            ("variable", "Symbolic"),
            ("domain", "Domain"),
            ("complete", "Boolean"),
            ("completeness", "Enum"),
            ("solutions", "Array"),
            ("families", "Array"),
            ("common_conditions", "Array"),
            ("represented_count", "Integer"),
            ("unrepresented_count", "Integer"),
            ("unrepresented_reason", "Text"),
            ("diagnostics", "Array"),
        ],
    );
    assert_schema(
        "SolutionFamily",
        &[
            ("template", "Symbolic"),
            ("parameter", "Symbolic"),
            ("period", "Symbolic"),
            ("parameter_domain", "Domain"),
            ("conditions", "Array"),
            ("exactness", "Enum"),
        ],
    );
    assert_schema(
        "Solution",
        &[
            ("value", "Symbolic"),
            ("conditions", "Array"),
            ("multiplicity", "Integer"),
            ("exactness", "Enum"),
        ],
    );
    assert_schema(
        "TransformResult",
        &[
            ("status", "Enum"),
            ("original", "Symbolic"),
            ("expression", "Symbolic"),
            ("changed", "Boolean"),
            ("conditions", "Array"),
            ("steps", "Array"),
            ("budget_exceeded", "Boolean"),
            ("budget_kind", "Text"),
            ("diagnostics", "Array"),
        ],
    );
    assert_schema(
        "RewriteStep",
        &[
            ("rule_id", "Text"),
            ("classification", "Enum"),
            ("before", "Symbolic"),
            ("after", "Symbolic"),
            ("required_conditions", "Array"),
        ],
    );
    assert_schema(
        "LimitResult",
        &[
            ("status", "Enum"),
            ("exists", "Boolean|Null"),
            ("value", "Symbolic|Null"),
            ("left", "Symbolic|Null"),
            ("left_conditions", "Array"),
            ("right", "Symbolic|Null"),
            ("right_conditions", "Array"),
            ("conditions", "Array"),
            ("exactness", "Enum"),
            ("diagnostics", "Array"),
        ],
    );
    assert_schema(
        "IntegrationResult",
        &[
            ("status", "Enum"),
            ("expression", "Symbolic"),
            ("conditions", "Array"),
            ("verified", "Boolean"),
            ("method", "Text"),
            ("verification_method", "Text"),
            ("exactness", "Enum"),
            ("diagnostics", "Array"),
        ],
    );
    assert_schema(
        "OptimizationResult",
        &[
            ("original", "Symbolic"),
            ("optimized", "Symbolic"),
            ("estimated_cost_before", "Integer"),
            ("estimated_cost_after", "Integer"),
            ("shared_subtrees", "Integer"),
            ("horner_rewrites", "Integer"),
            ("transformations", "Array"),
            ("target", "Text"),
            ("policy", "Text"),
            ("diagnostics", "Array"),
        ],
    );
    assert_schema(
        "CompilationResult",
        &[
            ("ir", "Text"),
            ("parameters", "Array"),
            ("result_domain", "Domain"),
            ("result_type", "Text"),
            ("precision_policy", "Text"),
            ("optimization_policy", "Text"),
            ("target", "Text"),
            ("mathir_version", "Integer"),
            ("exact", "Boolean"),
            ("diagnostics", "Array"),
        ],
    );
    assert_schema(
        "MatrixInverseResult",
        &[
            ("status", "Enum"),
            ("complete", "Boolean"),
            ("completeness", "Enum"),
            ("inverse", "Array"),
            ("conditions", "Array"),
            ("diagnostics", "Array"),
        ],
    );
    assert_schema(
        "MatrixSolveResult",
        &[
            ("status", "Enum"),
            ("complete", "Boolean"),
            ("completeness", "Enum"),
            ("solutions", "Array"),
            ("conditions", "Array"),
            ("diagnostics", "Array"),
        ],
    );
}

/// RecordSchemaTests.cs:204 DiagnosticSchema_DeclaresTheSixFrozenFields.
#[test]
fn diagnostic_schema_declares_the_six_frozen_fields() {
    assert_schema(
        "Diagnostic",
        &[
            ("code", "Text"),
            ("category", "Enum"),
            ("message", "Text"),
            ("recoverable", "Boolean"),
            ("location", "Record|Null"),
            ("details", "Array"),
        ],
    );
}

/// The shared body of the two SingularMatrix rows: a provably empty solution set is a COMPLETE
/// answer whose reason is ONE structured Diagnostic, not a prose string.
fn assert_singular_matrix(script: &str, type_name: &str, payload_field: &str) {
    let r = envelope(script, &[]);
    assert_eq!(r.exit, 0, "{}", r.env);
    assert!(boolean(get(&r.env, "ok")), "{}", r.env);

    let record = structured(&r.env);
    assert_eq!(text(get(record, "kind")), "Record");
    assert_eq!(text(get(record, "type")), type_name);
    assert!(RecordSchemas::for_type(type_name).is_some());
    assert!(RecordSchemas::validate(record).is_empty(), "{}", r.env);

    let status = record_field(record, "status");
    assert_eq!(text(get(status, "kind")), "Enum");
    assert_eq!(text(get(status, "type")), "SolveStatus");
    assert_eq!(text(get(status, "value")), "NoSolutions");

    let complete = record_field(record, "complete");
    assert_eq!(text(get(complete, "kind")), "Boolean");
    assert_eq!(text(get(complete, "value")), "true");

    let completeness = record_field(record, "completeness");
    assert_eq!(text(get(completeness, "kind")), "Enum");
    assert_eq!(text(get(completeness, "type")), "Completeness");
    assert_eq!(text(get(completeness, "value")), "Complete");

    // The payload stays an Array in the middle of the record, conditions after it.
    assert_eq!(
        text(get(record_field(record, payload_field), "kind")),
        "Array"
    );
    assert_eq!(
        text(get(record_field(record, "conditions"), "kind")),
        "Array"
    );

    // Diagnostics: an Array with exactly one Diagnostic - never the prose "matrix is singular".
    let diagnostics = record_field(record, "diagnostics");
    assert_eq!(text(get(diagnostics, "kind")), "Array");
    let elements = array(get(diagnostics, "elements"));
    assert_eq!(elements.len(), 1, "{script}: exactly one diagnostic");
    let diagnostic = &elements[0];
    assert_eq!(text(get(diagnostic, "kind")), "Record");
    assert_eq!(text(get(diagnostic, "type")), "Diagnostic");
    assert!(RecordSchemas::for_type("Diagnostic").is_some());
    assert!(RecordSchemas::validate(diagnostic).is_empty());

    assert_eq!(
        text(get(record_field(diagnostic, "code"), "value")),
        "matrix.singular"
    );
    let category = record_field(diagnostic, "category");
    assert_eq!(text(get(category, "kind")), "Enum");
    assert_eq!(text(get(category, "type")), "ErrorCategory");
    assert_eq!(text(get(category, "value")), "NoSolution");
    let message = text(get(record_field(diagnostic, "message"), "value"));
    assert!(
        !message.trim().is_empty(),
        "the human sentence moves INTO the diagnostic message"
    );
    let recoverable = record_field(diagnostic, "recoverable");
    assert_eq!(text(get(recoverable, "kind")), "Boolean");
    assert_eq!(text(get(recoverable, "value")), "true");
    // Null, never "".
    assert_eq!(
        text(get(record_field(diagnostic, "location"), "kind")),
        "Null"
    );
    assert!(array(get(record_field(diagnostic, "details"), "elements")).is_empty());
}

/// RecordSchemaTests.cs:227 SingularMatrix InlineData inv_full (#0).
#[test]
fn singular_matrix_inv_full_is_a_complete_no_solutions_carrying_a_structured_diagnostic() {
    assert_singular_matrix(
        r#"x = symbol("x"); inv_full([[x, x], [x, x]])"#,
        "MatrixInverseResult",
        "inverse",
    );
}

/// RecordSchemaTests.cs:227 SingularMatrix InlineData linsolve_full (#1).
#[test]
fn singular_matrix_linsolve_full_is_a_complete_no_solutions_carrying_a_structured_diagnostic() {
    assert_singular_matrix(
        r#"x = symbol("x"); linsolve_full([[x, x], [x, x]], [1, 1])"#,
        "MatrixSolveResult",
        "solutions",
    );
}

/// The shared body of the two SolvedMatrix rows: Solved / complete true / Completeness.Complete
/// and an EMPTY diagnostics array - an Array with zero elements.
fn assert_solved_matrix(script: &str, type_name: &str, payload_field: &str) {
    let r = envelope(script, &[]);
    assert_eq!(r.exit, 0, "{}", r.env);
    assert!(boolean(get(&r.env, "ok")), "{}", r.env);

    let record = structured(&r.env);
    assert_eq!(text(get(record, "kind")), "Record");
    assert_eq!(text(get(record, "type")), type_name);
    assert!(RecordSchemas::for_type(type_name).is_some());
    assert!(RecordSchemas::validate(record).is_empty(), "{}", r.env);

    let status = record_field(record, "status");
    assert_eq!(text(get(status, "kind")), "Enum");
    assert_eq!(text(get(status, "type")), "SolveStatus");
    assert_eq!(text(get(status, "value")), "Solved");

    let complete = record_field(record, "complete");
    assert_eq!(text(get(complete, "kind")), "Boolean");
    assert_eq!(text(get(complete, "value")), "true");

    let completeness = record_field(record, "completeness");
    assert_eq!(text(get(completeness, "kind")), "Enum");
    assert_eq!(text(get(completeness, "type")), "Completeness");
    assert_eq!(text(get(completeness, "value")), "Complete");

    assert_eq!(
        text(get(record_field(record, payload_field), "kind")),
        "Array"
    );
    assert_eq!(
        text(get(record_field(record, "conditions"), "kind")),
        "Array"
    );

    // Zero elements: an Array, NOT Null, NOT "".
    let diagnostics = record_field(record, "diagnostics");
    assert_eq!(text(get(diagnostics, "kind")), "Array");
    assert!(
        array(get(diagnostics, "elements")).is_empty(),
        "{script}: a solved matrix carries no diagnostics"
    );
}

/// RecordSchemaTests.cs:295 SolvedMatrix InlineData inv_full (#0).
#[test]
fn solved_matrix_inv_full_is_complete_with_an_empty_diagnostics_array() {
    assert_solved_matrix(
        r#"x = symbol("x"); y = symbol("y"); inv_full([[x, 1], [0, y]])"#,
        "MatrixInverseResult",
        "inverse",
    );
}

/// RecordSchemaTests.cs:295 SolvedMatrix InlineData linsolve_full (#1).
#[test]
fn solved_matrix_linsolve_full_is_complete_with_an_empty_diagnostics_array() {
    assert_solved_matrix(
        r#"x = symbol("x"); y = symbol("y"); linsolve_full([[x, 1], [0, y]], [0, 1])"#,
        "MatrixSolveResult",
        "solutions",
    );
}
