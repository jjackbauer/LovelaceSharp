//! Round 90: the two runner-facing PlotFileWriteRefusal rows.
//!
//! Each test drives the runner's own run_args pair and asserts the envelope the reference
//! publishes for the same caller path. The library half (the typed PlotFileWriteException and
//! the empty/missing write sites) lives in lovelace-lang::round90_runner_families; these tests
//! pin what the runner classifies on the wire, including the exact CLR inner sentence.

use std::path::Path;

use super::parity_batch::*;

fn plot_dir(name: &str) -> String {
    let dir = format!("{}/{}", super::parity_batch::plot_dir(), name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a fresh probe directory");
    dir
}

fn requested_path(directory: &str, file_name: &str) -> String {
    if file_name.is_empty() {
        directory.to_string()
    } else {
        Path::new(directory)
            .join(file_name)
            .to_string_lossy()
            .into_owned()
    }
}

fn absolute_path(requested: &str) -> String {
    let full = std::path::absolute(requested)
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|_| requested.to_string());
    #[cfg(windows)]
    let full = full.replace('/', "\\");
    full
}

fn run(script: &str, directory: &str, file_name: &str) -> RunResult {
    run_args(&[
        "--eval",
        script,
        "--json",
        "--omit-functions",
        "--omit-variables",
        "--plot-dir",
        directory,
        "--plot-file",
        file_name,
    ])
}

fn assert_refusal(r: &RunResult, requested: &str, detail: &str) {
    assert_eq!(r.exit, 1, "{}", r.env);
    assert!(!boolean(get(&r.env, "ok")), "{}", r.env);
    assert_eq!(text(get(&r.env, "code")), "PlotFileError");
    assert_eq!(text(get(&r.env, "category")), "TypeMismatch");
    assert!(boolean(get(&r.env, "recoverable")), "{}", r.env);
    let expected = format!("Cannot write the plot file '{requested}': {detail}");
    assert_eq!(text(get(&r.env, "message")), expected);
    let diagnostics = array(get(&r.env, "diagnostics"));
    assert_eq!(diagnostics.len(), 1, "{}", r.env);
    assert_eq!(text(get(&diagnostics[0], "message")), expected);
}

/// PlotFileWriteRefusalTests.PlotFileUnderAMissingDirectory_IsRefusedByType, at the wire.
#[test]
fn plot_file_under_a_missing_directory_is_refused_by_type() {
    let directory = plot_dir("round90-wire-missing");
    let requested = requested_path(&directory, "nope/x.svg");
    let full = absolute_path(&requested);
    let r = run("plot([1, 2, 3])", &directory, "nope/x.svg");
    assert_refusal(
        &r,
        &requested,
        &format!("Could not find a part of the path '{full}'."),
    );
    assert!(
        !Path::new(&directory).join("nope").exists(),
        "the refused write must not create the missing parent"
    );
    let _ = std::fs::remove_dir_all(&directory);
}

/// PlotFileWriteRefusalTests.EmptyPlotFile_IsRefusedByType, at the wire.
#[test]
fn empty_plot_file_is_refused_by_type() {
    let directory = plot_dir("round90-wire-empty");
    let requested = requested_path(&directory, "");
    let full = absolute_path(&requested);
    let r = run("plot([1, 2, 3])", &directory, "");
    assert_refusal(
        &r,
        &requested,
        &format!("Access to the path '{full}' is denied."),
    );
    let _ = std::fs::remove_dir_all(&directory);
}
