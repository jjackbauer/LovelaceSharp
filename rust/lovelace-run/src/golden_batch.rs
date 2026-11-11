//! Golden-envelope ports from Lovelace.Run.Tests.GoldenEnvelopeTests.
//!
//! The fixtures live in the C# project that owns the wire contract; the Rust runner is compared to
//! the same golden after the same volatile-key normalisation the C# test applies.

use super::parity_batch::*;
use super::*;
use lovelace_json::Json;

const FIXTURES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../Lovelace.Run.Tests/fixtures"
);
const VOLATILE: [&str; 4] = ["revision", "elapsed", "elapsedTime", "timings"];

fn normalise(j: &Json) -> Json {
    match j {
        Json::Obj(map) => {
            let mut out = std::collections::BTreeMap::new();
            for (key, value) in map {
                let v = if VOLATILE.contains(&key.as_str()) {
                    Json::Str("<volatile>".to_string())
                } else {
                    normalise(value)
                };
                out.insert(key.clone(), v);
            }
            Json::Obj(out)
        }
        Json::Arr(items) => Json::Arr(items.iter().map(normalise).collect()),
        other => other.clone(),
    }
}

fn run_fixture(name: &str) -> RunResult {
    let script = format!("{FIXTURES}/{name}.ls");
    run_args(&["--file", &script, "--omit-functions"])
}

fn fixture(name: &str) -> Json {
    let path = format!("{FIXTURES}/{name}.json");
    Json::parse(&std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}")))
        .expect("golden parses")
}

#[test]
fn every_fixture_matches_its_golden_envelope() {
    let corpus = [
        "symbolic",
        "vector",
        "array",
        "record",
        "nested_record",
        "solve_result",
        "solve_nosolutions",
        "transform_result",
        "limit_result",
        "system_solve",
        "integration_result",
        "optimization_result",
        "compilation",
        "complex",
        "matrix_inverse_result",
        "matrix_solve_result",
        "absent_field",
        "capabilities",
        "error_envelope",
        "print_purity",
    ];
    for name in corpus {
        let r = run_fixture(name);
        let expected_exit = if name == "error_envelope" { 1 } else { 0 };
        assert_eq!(
            r.exit, expected_exit,
            "'{name}' exited {}: {}",
            r.exit, r.stdout
        );
        let actual = normalise(&r.env);
        let expected = normalise(&fixture(name));
        assert_eq!(
            actual, expected,
            "{name}: envelope does not match the golden fixture"
        );
    }
}

#[test]
fn solve_record_carries_the_structural_solve_contract() {
    let r = run_fixture("record");
    assert_eq!(r.exit, 0);
    assert!(boolean(get(&r.env, "ok")));
    let structured = get(get(&r.env, "result"), "structured");
    assert_eq!(text(get(structured, "kind")), "Record");
    assert_eq!(text(get(structured, "type")), "SolveResult");
    let status = record_field(structured, "status");
    assert_eq!(text(get(status, "value")), "Solved");
    assert_eq!(text(get(status, "kind")), "Enum");
    assert_eq!(text(get(status, "type")), "SolveStatus");
    let completeness = record_field(structured, "completeness");
    assert_eq!(text(get(completeness, "kind")), "Enum");
    assert_eq!(text(get(completeness, "type")), "Completeness");
    assert_eq!(text(get(completeness, "value")), "Complete");
    assert_eq!(
        text(get(record_field(structured, "domain"), "domain")),
        "complex"
    );
    let solutions = record_field(structured, "solutions");
    assert_eq!(text(get(solutions, "kind")), "Array");
    assert_eq!(
        array(get(solutions, "shape"))
            .iter()
            .map(number)
            .collect::<Vec<_>>(),
        vec![2]
    );
    let elements = array(get(solutions, "elements"));
    assert_eq!(elements.len(), 2);
    for solution in elements {
        assert_eq!(text(get(solution, "kind")), "Record");
        assert_eq!(text(get(solution, "type")), "Solution");
        let exactness = record_field(solution, "exactness");
        assert_eq!(text(get(exactness, "kind")), "Enum");
        assert_eq!(text(get(exactness, "type")), "SolutionExactness");
    }
}

#[test]
fn error_envelope_carries_a_structural_code_and_category() {
    let r = run_fixture("error_envelope");
    assert_eq!(r.exit, 1);
    assert!(!boolean(get(&r.env, "ok")));
    assert!(!text(get(&r.env, "code")).is_empty());
    assert!(!text(get(&r.env, "category")).is_empty());
}

#[test]
fn rich_result_diagnostics_cross_the_wire_as_an_array() {
    let fixtures = [
        ("record", "SolveResult"),
        ("system_solve", "SystemSolveResult"),
        ("matrix_inverse_result", "MatrixInverseResult"),
        ("matrix_solve_result", "MatrixSolveResult"),
        ("limit_result", "LimitResult"),
        ("integration_result", "IntegrationResult"),
        ("transform_result", "TransformResult"),
        ("optimization_result", "OptimizationResult"),
        ("compilation", "CompilationResult"),
    ];
    for (name, record_type) in fixtures {
        let r = run_fixture(name);
        assert_eq!(r.exit, 0, "'{name}' exited {}", r.exit);
        let structured = get(get(&r.env, "result"), "structured");
        assert_eq!(text(get(structured, "kind")), "Record");
        assert_eq!(text(get(structured, "type")), record_type);
        let ordered = array(get(structured, "fields"));
        assert_eq!(
            text(get(&ordered[ordered.len() - 1], "name")),
            "diagnostics"
        );
        let diagnostics = record_field(structured, "diagnostics");
        assert_eq!(text(get(diagnostics, "kind")), "Array");
        assert!(opt(diagnostics, "value").is_none());
        assert!(opt(diagnostics, "elements").is_some());
    }
}

#[test]
fn wire_diagnostic_carries_the_six_frozen_fields_in_order() {
    let r = run_fixture("solve_result");
    assert_eq!(r.exit, 0);
    let structured = get(get(&r.env, "result"), "structured");
    assert_eq!(
        text(get(record_field(structured, "status"), "value")),
        "Partial"
    );
    let diagnostics = record_field(structured, "diagnostics");
    assert_eq!(text(get(diagnostics, "kind")), "Array");
    let elements = array(get(diagnostics, "elements"));
    assert!(!elements.is_empty());
    let diagnostic = &elements[0];
    assert_eq!(text(get(diagnostic, "kind")), "Record");
    assert_eq!(text(get(diagnostic, "type")), "Diagnostic");
    let names: Vec<&str> = array(get(diagnostic, "fields"))
        .iter()
        .map(|f| text(get(f, "name")))
        .collect();
    assert_eq!(
        names,
        vec![
            "code",
            "category",
            "message",
            "recoverable",
            "location",
            "details"
        ]
    );
    assert!(!text(get(record_field(diagnostic, "code"), "value")).is_empty());
    assert!(!text(get(record_field(diagnostic, "message"), "value")).is_empty());
    let category = record_field(diagnostic, "category");
    assert_eq!(text(get(category, "kind")), "Enum");
    assert_eq!(text(get(category, "type")), "ErrorCategory");
    assert_eq!(
        text(get(record_field(diagnostic, "recoverable"), "kind")),
        "Boolean"
    );
    assert_eq!(
        text(get(record_field(diagnostic, "location"), "kind")),
        "Null"
    );
    let details = record_field(diagnostic, "details");
    assert_eq!(text(get(details, "kind")), "Array");
    assert!(array(get(details, "elements")).is_empty());
}

#[test]
fn every_fixture_script_has_a_corpus_row() {
    let mut on_disk: Vec<String> = std::fs::read_dir(FIXTURES)
        .expect("fixture directory")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".ls"))
        .map(|n| n.trim_end_matches(".ls").to_string())
        .collect();
    on_disk.sort();
    let mut corpus = vec![
        "symbolic",
        "vector",
        "array",
        "record",
        "nested_record",
        "solve_result",
        "solve_nosolutions",
        "transform_result",
        "limit_result",
        "system_solve",
        "integration_result",
        "optimization_result",
        "compilation",
        "complex",
        "matrix_inverse_result",
        "matrix_solve_result",
        "absent_field",
        "capabilities",
        "error_envelope",
        "print_purity",
    ];
    corpus.sort();
    assert_eq!(on_disk, corpus);
}
