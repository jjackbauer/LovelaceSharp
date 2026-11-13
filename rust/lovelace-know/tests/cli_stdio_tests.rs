//! End-to-end JSON-over-stdio tests: the protocol, exit codes, and the real runner.

mod common;

use lovelace_know::graph;
use lovelace_know::model::Graph;

fn response(stdout: &str) -> lovelace_json::Json {
    lovelace_json::Json::parse(stdout).expect("the response is JSON")
}

fn member<'a>(document: &'a lovelace_json::Json, key: &str) -> &'a lovelace_json::Json {
    document
        .get(key)
        .unwrap_or_else(|| panic!("{key} is present"))
}

#[test]
fn config_command_resolves_the_defaults() {
    let (stdout, stderr, code) = common::run_cli(&["--stdin"], r#"{"command":"config"}"#);
    assert_eq!(code, 0, "stderr: {stderr}");
    let document = response(&stdout);
    assert_eq!(member(&document, "ok"), &lovelace_json::Json::Bool(true));
    assert_eq!(
        document
            .get("command")
            .and_then(lovelace_json::Json::as_str),
        Some("config")
    );
    assert_eq!(
        document
            .get("summary")
            .and_then(lovelace_json::Json::as_str),
        Some(
            "config resolved: seed=20240617, batch=64, maxSamples=700, ops=12, naturals=13, integers=13, reals=8"
        )
    );
    let config = member(&document, "config");
    assert_eq!(
        config.get("seed"),
        Some(&lovelace_json::Json::Num("20240617".to_string()))
    );
    // Unused members are present and null.
    assert_eq!(member(&document, "graph"), &lovelace_json::Json::Null);
    assert_eq!(member(&document, "error"), &lovelace_json::Json::Null);
}

#[test]
fn help_command_returns_the_command_list() {
    let (stdout, stderr, code) = common::run_cli(&["--stdin"], r#"{"command":"help"}"#);
    assert_eq!(code, 0, "stderr: {stderr}");
    let document = response(&stdout);
    let summary = document
        .get("summary")
        .and_then(lovelace_json::Json::as_str)
        .expect("summary");
    assert!(summary.contains("config"));
    assert!(summary.contains("converge"));
    assert!(summary.contains("query"));
}

#[test]
fn unknown_command_exits_one() {
    let (stdout, _, code) = common::run_cli(&["--stdin"], r#"{"command":"bogus"}"#);
    assert_eq!(code, 1);
    let document = response(&stdout);
    assert_eq!(member(&document, "ok"), &lovelace_json::Json::Bool(false));
    assert_eq!(
        document.get("error").and_then(lovelace_json::Json::as_str),
        Some("unknown command 'bogus'")
    );
}

#[test]
fn invalid_request_json_exits_one() {
    let (stdout, _, code) = common::run_cli(&["--stdin"], "{not json");
    assert_eq!(code, 1);
    let document = response(&stdout);
    assert_eq!(member(&document, "ok"), &lovelace_json::Json::Bool(false));
    assert!(
        document
            .get("error")
            .and_then(lovelace_json::Json::as_str)
            .expect("an error message")
            .starts_with("invalid request JSON:")
    );
}

#[test]
fn unknown_argument_is_a_usage_error() {
    let (stdout, stderr, code) = common::run_cli(&["--bogus"], "");
    assert_eq!(code, 2);
    assert!(stdout.is_empty(), "usage errors go to stderr");
    assert!(stderr.contains("Unknown argument '--bogus'"));
    assert!(stderr.contains("Usage:"));
}

#[test]
fn missing_request_is_a_usage_error() {
    let (_, stderr, code) = common::run_cli(&[], "");
    assert_eq!(code, 2);
    assert!(stderr.contains("No request provided"));
}

#[test]
fn eval_and_file_forms_are_equivalent() {
    let path = common::temp_graph_path("request");
    std::fs::write(&path, r#"{"command":"config"}"#).expect("the request file is written");
    let (from_file, _, code) = common::run_cli(&["--file", &path.display().to_string()], "");
    assert_eq!(code, 0);
    let (from_stdin, _, code) = common::run_cli(&["--stdin"], r#"{"command":"config"}"#);
    assert_eq!(code, 0);
    assert_eq!(response(&from_file), response(&from_stdin));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sample_command_executes_the_real_runner() {
    let request = format!(
        r#"{{"command":"sample","runner":"{}","batchSize":3}}"#,
        common::json_path(&common::runner_path())
    );
    let (stdout, stderr, code) = common::run_cli(&["--stdin"], &request);
    assert_eq!(code, 0, "stderr: {stderr}");
    let document = response(&stdout);
    let samples = member(&document, "samples")
        .as_arr()
        .expect("samples is an array");
    assert_eq!(samples.len(), 3);
    let summary = document
        .get("summary")
        .and_then(lovelace_json::Json::as_str)
        .expect("summary");
    assert!(summary.starts_with("executed 3 samples: "), "{summary}");
    for sample in samples {
        let script = sample
            .get("script")
            .and_then(lovelace_json::Json::as_str)
            .expect("script");
        assert!(
            script.contains(' '),
            "the script is an expression: {script}"
        );
        assert!(
            sample
                .get("sigma")
                .and_then(lovelace_json::Json::as_str)
                .is_some()
        );
    }
}

#[test]
fn converge_then_query_round_trip() {
    let graph_path = common::temp_graph_path("cli-converge");
    let _ = std::fs::remove_file(&graph_path);
    let request = format!(
        r#"{{"command":"converge","graphPath":"{}","runner":"{}","maxSamples":8,"batchSize":8}}"#,
        common::json_path(&graph_path),
        common::json_path(&common::runner_path())
    );
    let (stdout, stderr, code) = common::run_cli(&["--stdin"], &request);
    assert_eq!(code, 0, "stderr: {stderr}");
    let document = response(&stdout);
    assert_eq!(member(&document, "ok"), &lovelace_json::Json::Bool(true));
    assert!(
        graph_path.is_file(),
        "the converge command persists the graph"
    );

    let persisted = graph::load(&graph_path).expect("the graph is readable");
    assert_eq!(persisted.samples.len(), 8);
    assert!(persisted.metrics.is_some());
    assert_eq!(persisted.version, Graph::VERSION);

    let query = format!(
        r#"{{"command":"query","graphPath":"{}","query":"summary"}}"#,
        common::json_path(&graph_path)
    );
    let (stdout, _, code) = common::run_cli(&["--stdin"], &query);
    assert_eq!(code, 0);
    let summary = response(&stdout)
        .get("summary")
        .and_then(lovelace_json::Json::as_str)
        .expect("summary")
        .to_string();
    assert!(summary.starts_with("planes="), "{summary}");

    let planes_query = format!(
        r#"{{"command":"query","graphPath":"{}","query":"planes"}}"#,
        common::json_path(&graph_path)
    );
    let (stdout, _, code) = common::run_cli(&["--stdin"], &planes_query);
    assert_eq!(code, 0);
    let document = response(&stdout);
    let planes = member(&document, "planes")
        .as_arr()
        .expect("planes is an array");
    assert_eq!(planes.len(), persisted.planes.len());

    let unknown_query = format!(
        r#"{{"command":"query","graphPath":"{}","query":"nope"}}"#,
        common::json_path(&graph_path)
    );
    let (stdout, _, code) = common::run_cli(&["--stdin"], &unknown_query);
    assert_eq!(code, 1);
    assert!(
        response(&stdout)
            .get("error")
            .and_then(lovelace_json::Json::as_str)
            .expect("an error")
            .contains("unknown query 'nope'")
    );

    let _ = std::fs::remove_file(&graph_path);
}

#[test]
fn missing_graph_exits_one() {
    let missing = common::temp_graph_path("missing");
    let _ = std::fs::remove_file(&missing);
    let request = format!(
        r#"{{"command":"query","graphPath":"{}"}}"#,
        common::json_path(&missing)
    );
    let (stdout, _, code) = common::run_cli(&["--stdin"], &request);
    assert_eq!(code, 1);
    assert!(
        response(&stdout)
            .get("error")
            .and_then(lovelace_json::Json::as_str)
            .expect("an error")
            .starts_with("no graph at '")
    );
}
