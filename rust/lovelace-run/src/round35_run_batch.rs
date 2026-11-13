//! Round 35: the adjudicated Lovelace.Run.Tests tail and the envelope rows the recent
//! production unblocked. The C# tests drive the published runner; these tests either build the
//! SAME envelope in-process through envelope_for (the function the binary's run() calls) or,
//! where the assertion is about the real process (stdout purity, the --text surface), spawn the
//! built runner binary.

use std::io::Write;

use super::*;

fn run_envelope(script: &str) -> (Json, i32) {
    let opts = Options {
        source: Some(script.to_string()),
        origin: SourceOrigin::Eval,
        source_file: None,
        plot_dir: None,
        plot_file: None,
        mode: Mode::Json,
        omit_functions: true,
        omit_variables: true,
        print_budget: None,
        cancel_after: None,
        list_functions: false,
        function_pattern: None,
        describe_function: None,
        function_limit: 50,
        repl: false,
    };
    let script = ScriptSource::new(SourceIdentity::eval(), script);
    let mut engine = Engine::new();
    let execution = execute(&mut engine, &opts, script.text());
    envelope_for(&mut engine, &opts, &script, execution)
}

fn field<'a>(value: &'a Json, name: &str) -> &'a Json {
    value
        .get("fields")
        .and_then(Json::as_arr)
        .and_then(|fields| {
            fields
                .iter()
                .find(|f| f.get("name").and_then(Json::as_str) == Some(name))
                .and_then(|f| f.get("value"))
        })
        .unwrap_or_else(|| panic!("record has no field '{name}': {}", value))
}

fn bool_of(value: &Json) -> Option<bool> {
    match value {
        Json::Bool(flag) => Some(*flag),
        _ => None,
    }
}

fn text(value: &Json) -> &str {
    value
        .get("value")
        .and_then(Json::as_str)
        .or_else(|| value.as_str())
        .unwrap_or_else(|| panic!("not a text value: {}", value))
}

fn runner_binary() -> std::path::PathBuf {
    let test_exe = std::env::current_exe().expect("test binary path");
    let profile_dir = test_exe
        .parent()
        .and_then(std::path::Path::parent)
        .expect("target profile directory");
    profile_dir.join(format!("lovelace-run{}", std::env::consts::EXE_SUFFIX))
}

struct Run {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn spawn(args: &[&str]) -> Run {
    let binary = runner_binary();
    let output = std::process::Command::new(&binary)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("cannot run {}: {e}", binary.display()));
    Run {
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

// ===========================================================================
// Lovelace.Run.Tests.UnknownFunctionEnvelopeTests.TheTextSurface_CarriesTheSameAnswer
// ===========================================================================

#[test]
fn unknown_function_text_surface_carries_the_same_answer() {
    let run = spawn(&[
        "--eval",
        "determinant([[1,2],[3,4]])",
        "--omit-functions",
        "--omit-variables",
        "--text",
    ]);
    assert_eq!(run.code, Some(1));
    assert!(
        run.stderr
            .contains("Error [UnknownFunction/UnsupportedOperation]"),
        "stderr: {}",
        run.stderr
    );
    assert!(
        run.stderr.contains("Did you mean 'det'?"),
        "stderr: {}",
        run.stderr
    );
}

// ===========================================================================
// Lovelace.Run.Tests.StdoutPurityTests
// ===========================================================================

#[test]
fn real_process_stdout_is_one_json_document_and_print_stays_in_output() {
    let dir = std::env::temp_dir().join(format!("lovelace-round35-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let script = dir.join("print_purity.lov");
    let mut file = std::fs::File::create(&script).expect("script file");
    writeln!(file, "print(\"hello from the script\"); 1 + 1").expect("write script");
    drop(file);

    let run = spawn(&[
        "--file",
        script.to_str().expect("utf-8 path"),
        "--omit-functions",
    ]);
    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    let envelope = Json::parse(run.stdout.trim()).expect("stdout is one JSON document");
    let output: Vec<&str> = envelope
        .get("output")
        .and_then(Json::as_arr)
        .expect("output array")
        .iter()
        .filter_map(Json::as_str)
        .collect();
    assert_eq!(output, vec!["hello from the script"]);
    let result = envelope.get("result").expect("result");
    let structured = result.get("structured").expect("structured result");
    assert_eq!(structured.get("value").and_then(Json::as_str), Some("2"));
    assert_eq!(run.stdout.matches("hello from the script").count(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

// ===========================================================================
// Lovelace.Run.Tests.LimitShortFormRecordTests.Refusal_CarriesTheCodeAndCategory...
// ===========================================================================

#[test]
fn limit_short_form_refusals_carry_the_diagnostic() {
    for call in ["limit", "limit_left", "limit_right"] {
        let script = format!("a = symbol(\"a\"); x = symbol(\"x\"); {call}(a/x, x, 0)");
        let (envelope, exit) = run_envelope(&script);
        assert_eq!(exit, 0, "{script}: {}", envelope);
        assert_eq!(envelope.get("ok").and_then(bool_of), Some(true));

        let structured = envelope
            .get("result")
            .expect("result")
            .get("structured")
            .expect("structured");
        assert_eq!(
            structured.get("kind").and_then(Json::as_str),
            Some("Record")
        );
        assert_eq!(
            structured.get("type").and_then(Json::as_str),
            Some("LimitResult")
        );

        let status = field(structured, "status");
        assert_eq!(status.get("kind").and_then(Json::as_str), Some("Enum"));
        assert_eq!(
            status.get("type").and_then(Json::as_str),
            Some("LimitStatus")
        );
        assert_eq!(
            status.get("value").and_then(Json::as_str),
            Some("Unevaluated")
        );
        assert_eq!(
            field(structured, "exists")
                .get("kind")
                .and_then(Json::as_str),
            Some("Null")
        );

        let diagnostics = field(structured, "diagnostics");
        let elements = diagnostics
            .get("elements")
            .and_then(Json::as_arr)
            .expect("elements");
        assert_eq!(elements.len(), 1, "{script}: {}", diagnostics);
        let diagnostic = &elements[0];
        assert_eq!(
            diagnostic.get("type").and_then(Json::as_str),
            Some("Diagnostic")
        );
        assert_eq!(text(field(diagnostic, "code")), "limit.unevaluated");
        let category = field(diagnostic, "category");
        assert_eq!(
            category.get("type").and_then(Json::as_str),
            Some("ErrorCategory")
        );
        assert_eq!(
            category.get("value").and_then(Json::as_str),
            Some("UnsupportedOperation")
        );
        assert_eq!(
            text(field(diagnostic, "message")),
            "leading coefficient is symbolic"
        );
        assert_eq!(text(field(diagnostic, "recoverable")), "true");
    }
}

// ===========================================================================
// Lovelace.Run.Tests.BuiltinSurfaceContractTests.Assumptions_ReturnsStructuredAtoms...
// ===========================================================================

#[test]
fn assumptions_returns_structured_atoms_with_display_and_domain_condition() {
    let (envelope, exit) = run_envelope("x = symbol(\"x\"); assume(x > 5); assumptions()");
    assert_eq!(exit, 0);
    let structured = envelope
        .get("result")
        .expect("result")
        .get("structured")
        .expect("structured");
    assert_eq!(
        structured.get("type").and_then(Json::as_str),
        Some("AssumptionSet")
    );
    let display = field(structured, "display");
    assert_eq!(text(display), "x > 5");
    let atoms = field(structured, "assumptions");
    let elements = atoms
        .get("elements")
        .and_then(Json::as_arr)
        .expect("elements");
    assert_eq!(elements.len(), 1);
    assert_eq!(
        elements[0].get("kind").and_then(Json::as_str),
        Some("Symbolic")
    );
    assert_eq!(
        elements[0].get("canonical").and_then(Json::as_str),
        Some("(gt (sym x) (rat 5 1))")
    );

    let (envelope, exit) = run_envelope("x = symbol(\"x\"); assume_integer(x); assumptions()");
    assert_eq!(exit, 0);
    let structured = envelope
        .get("result")
        .expect("result")
        .get("structured")
        .expect("structured");
    let atoms = field(structured, "assumptions");
    let elements = atoms
        .get("elements")
        .and_then(Json::as_arr)
        .expect("elements");
    let atom = &elements[0];
    assert_eq!(
        atom.get("type").and_then(Json::as_str),
        Some("DomainCondition")
    );
    let domain = field(atom, "domain");
    assert_eq!(domain.get("kind").and_then(Json::as_str), Some("Domain"));
    assert_eq!(domain.get("domain").and_then(Json::as_str), Some("integer"));
}
