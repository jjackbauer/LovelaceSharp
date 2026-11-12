//! Round-70 ports of the four \`plot-wire\` census rows.
//!
//! Each test drives the runner's own \`run_args\` (the same \`execute\`/\`envelope_for\` pair \`main\`
//! runs), so an assertion cannot pass on a flow the binary does not execute. The C# fact body is
//! the contract: exit code, code/category, recoverability, the caller path in the message, and
//! the duration/diagnostics shape on the error path.

use super::parity_batch::*;
use super::*;
use lovelace_lang::Value;

fn field<'a>(record: &'a [(String, Value)], name: &str) -> Option<&'a Value> {
    record
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
}

fn text_value(value: &Value) -> Option<String> {
    match value {
        Value::Text(text) => Some(text.clone()),
        // The capability record publishes the category as the documented ErrorCategory enum.
        Value::Enum(_, name) => Some(name.clone()),
        _ => None,
    }
}

/// The live \`capabilities()\` entry for \`operation_class\` (code, category, message).
fn advertised(operation_class: &str) -> (String, String, String) {
    let mut engine = Engine::new();
    let value = engine
        .eval("capabilities()")
        .expect("capabilities() answers");
    let Value::Record(_, record) = value else {
        panic!("capabilities() must be a record");
    };
    let operations = field(&record, "unsupported_operations").expect("the operations field");
    let Value::Vector(items) = operations else {
        panic!("unsupported_operations must be a vector: {operations:?}");
    };
    for item in items {
        let Value::Record(_, entry) = item else {
            panic!("an UnsupportedCapability must be a record");
        };
        if field(entry, "operation_class")
            .and_then(text_value)
            .as_deref()
            == Some(operation_class)
        {
            return (
                field(entry, "code")
                    .and_then(text_value)
                    .expect("an entry carries code"),
                field(entry, "category")
                    .and_then(text_value)
                    .expect("an entry carries category"),
                field(entry, "message")
                    .and_then(text_value)
                    .expect("an entry carries message"),
            );
        }
    }
    panic!("capabilities() does not advertise '{operation_class}'");
}

fn plot_dir(name: &str) -> String {
    let dir = format!("{}/{}", super::parity_batch::plot_dir(), name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a fresh probe directory");
    dir
}

fn run(script: &str, dir: &str, file: &str) -> RunResult {
    run_args(&[
        "--eval",
        script,
        "--json",
        "--omit-functions",
        "--omit-variables",
        "--plot-dir",
        dir,
        "--plot-file",
        file,
    ])
}

#[test]
fn empty_plot_directory_is_an_error_envelope_with_exit_one() {
    // PlotDirectoryErrorTests.EmptyPlotDirectory_IsAnErrorEnvelopeWithExitOne: an empty
    // --plot-dir is a caller mistake that crosses as an envelope, never a silent success.
    let r = run_args(&[
        "--eval",
        "1",
        "--json",
        "--omit-functions",
        "--omit-variables",
        "--plot-dir",
        "",
    ]);
    assert_eq!(r.exit, 1, "{}", r.env);
    assert!(!r.stdout.is_empty(), "stdout must never be empty");
    assert_eq!(number(get(&r.env, "protocolVersion")), 1);
    assert!(!boolean(get(&r.env, "ok")));
    assert_eq!(text(get(&r.env, "code")), "PlotDirectoryError");
    assert_eq!(text(get(&r.env, "category")), "TypeMismatch");
    assert!(boolean(get(&r.env, "recoverable")));
    assert!(text(get(&r.env, "message")).starts_with("Cannot use plot directory ''"));
    assert!(opt(&r.env, "diagnostics").is_some());
    assert!(opt(&r.env, "elapsed").is_some());
    let elapsed = get(&r.env, "elapsedTime");
    assert!(opt(elapsed, "value").is_some());
    assert!(opt(elapsed, "unit").is_some());
    assert!(opt(&r.env, "timings").is_some());
}

#[test]
fn empty_plot_file_is_an_error_envelope_with_exit_one() {
    // PlotFileErrorTests.EmptyPlotFile_IsAnErrorEnvelopeWithExitOne: the empty name resolves to
    // the plot directory itself, and the refusal names the path the caller asked for.
    let dir = plot_dir("plot-surface-empty-file");
    let r = run("plot([1, 2, 3])", &dir, "");
    assert_eq!(r.exit, 1, "{}", r.env);
    assert!(!boolean(get(&r.env, "ok")));
    assert_eq!(text(get(&r.env, "code")), "PlotFileError");
    assert_eq!(text(get(&r.env, "category")), "TypeMismatch");
    assert!(boolean(get(&r.env, "recoverable")));
    assert!(
        text(get(&r.env, "message")).contains(&dir),
        "the refusal names the requested path: {}",
        r.env
    );
    let diagnostics = array(get(&r.env, "diagnostics"));
    assert_eq!(diagnostics.len(), 1, "{}", r.env);
    assert!(
        text(get(&diagnostics[0], "message")).contains(&dir),
        "the diagnostic names the requested path: {}",
        r.env
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn plot_file_under_a_missing_directory_is_an_error_envelope_with_exit_one() {
    // PlotFileErrorTests.PlotFileUnderAMissingDirectory_IsAnErrorEnvelopeWithExitOne: the write
    // does not create "nope", so the caller sees exit 1 and a refusal naming the path.
    let dir = plot_dir("plot-surface-missing-parent");
    let r = run("plot([1, 2, 3])", &dir, "nope/x.svg");
    // The C# row asserts the message contains Path.Combine(plotDirectory, plotFileName); the
    // Rust equivalent is the platform join, which keeps the caller's forward slash inside the
    // file-name argument and inserts the platform separator before it.
    let requested = std::path::Path::new(&dir)
        .join("nope/x.svg")
        .display()
        .to_string();
    assert_eq!(r.exit, 1, "{}", r.env);
    assert!(!boolean(get(&r.env, "ok")));
    assert_eq!(text(get(&r.env, "code")), "PlotFileError");
    assert_eq!(text(get(&r.env, "category")), "TypeMismatch");
    assert!(boolean(get(&r.env, "recoverable")));
    assert!(
        text(get(&r.env, "message")).contains(&requested),
        "the refusal names the requested path: {}",
        r.env
    );
    let diagnostics = array(get(&r.env, "diagnostics"));
    assert_eq!(diagnostics.len(), 1, "{}", r.env);
    assert!(
        text(get(&diagnostics[0], "message")).contains(&requested),
        "the diagnostic names the requested path: {}",
        r.env
    );
    assert!(
        !std::path::Path::new(&dir).join("nope").exists(),
        "the refused write must not create the missing parent"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn symbolic_plot_is_a_typed_refusal() {
    // CapabilitiesBuiltinTests.SymbolicPlot_IsATypedRefusal_NeverAnInternalInvariantFailure: a
    // non-vector argument and a symbolic element are both typed InvalidOperation/DomainError
    // refusals, never InternalInvariantFailure, and the live envelope equals the advertisement.
    let dir = plot_dir("plot-surface-symbolic");
    let cases = [
        (
            "x = symbol(\"x\"); plot(sin(x));",
            "plot() argument 1 must be a vector, but got 'Symbolic'.",
        ),
        (
            "plot(1);",
            "plot() argument 1 must be a vector, but got 'Natural'.",
        ),
        (
            "x = symbol(\"x\"); plot([x, 1, 2]);",
            "Cannot convert value of kind 'Symbolic' to a number for plotting.",
        ),
    ];
    for (script, expected) in cases {
        let r = run(script, &dir, "plot.svg");
        assert_eq!(r.exit, 1, "{script}: {}", r.env);
        assert!(!boolean(get(&r.env, "ok")), "{script}: {}", r.env);
        assert_eq!(text(get(&r.env, "code")), "InvalidOperation", "{script}");
        assert_eq!(text(get(&r.env, "category")), "DomainError", "{script}");
        assert_ne!(
            text(get(&r.env, "category")),
            "InternalInvariantFailure",
            "{script}"
        );
        assert!(boolean(get(&r.env, "recoverable")), "{script}");
        assert_eq!(text(get(&r.env, "message")), expected, "{script}");
    }

    let (code, category, message) = advertised("plot.symbolic-expression");
    let r = run("x = symbol(\"x\"); plot(sin(x));", &dir, "plot.svg");
    assert_eq!(code, text(get(&r.env, "code")));
    assert_eq!(category, text(get(&r.env, "category")));
    assert_eq!(message, text(get(&r.env, "message")));

    let (code, category, message) = advertised("plot.symbolic-element");
    let r = run("x = symbol(\"x\"); plot([x, 1, 2]);", &dir, "plot.svg");
    assert_eq!(code, text(get(&r.env, "code")));
    assert_eq!(category, text(get(&r.env, "category")));
    assert_eq!(message, text(get(&r.env, "message")));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_writable_plot_file_is_still_written_after_the_refusals() {
    // The success path is not allowed to regress: a writable --plot-file is still written, still
    // named by the envelope, and still carries the same polyline document.
    let dir = plot_dir("plot-surface-success");
    let r = run("plot([1, 2, 3])", &dir, "ok.svg");
    assert_eq!(r.exit, 0, "{}", r.env);
    assert!(boolean(get(&r.env, "ok")));
    let plot = get(&r.env, "plot");
    let path = text(get(plot, "path"));
    assert!(path.ends_with("ok.svg"), "{path}");
    let svg = text(get(plot, "svg"));
    assert!(svg.starts_with("<svg"), "{svg}");
    assert!(svg.contains("<polyline"), "{svg}");
    assert!(std::path::Path::new(&path).exists(), "{path}");
    let _ = std::fs::remove_dir_all(&dir);
}
