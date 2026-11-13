//! Shared helpers for the lovelace-know integration tests.
#![allow(dead_code)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use lovelace_know::json::Out;
use lovelace_know::model::{Operand, Operation, SampleRecord, SamplingKind};

/// Resolves the real lovelace-run executable the tests must drive.
pub fn runner_path() -> PathBuf {
    if let Ok(path) = std::env::var("LOVELACE_RUN") {
        if !path.is_empty() {
            return PathBuf::from(path);
        }
    }
    let name = if cfg!(windows) {
        "lovelace-run.exe"
    } else {
        "lovelace-run"
    };
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(executable) = std::env::current_exe() {
        if let Some(directory) = executable.parent() {
            // target/debug/deps -> target/debug
            if let Some(profile) = directory.parent() {
                candidates.push(profile.join(name));
                if let Some(target) = profile.parent() {
                    candidates.push(target.join("release").join(name));
                }
            }
        }
    }
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    candidates.push(manifest.join("../../target/debug").join(name));
    candidates.push(manifest.join("../../target/release").join(name));
    for candidate in candidates {
        if candidate.is_file() {
            return candidate;
        }
    }
    panic!(
        "lovelace-run not found; build it first (cargo build -p lovelace-run) or set LOVELACE_RUN"
    );
}

/// Resolves the lovelace-know CLI executable under test.
pub fn cli_binary() -> PathBuf {
    let name = if cfg!(windows) {
        "lovelace-know.exe"
    } else {
        "lovelace-know"
    };
    let executable = std::env::current_exe().expect("the test binary has a path");
    let profile = executable
        .parent()
        .and_then(Path::parent)
        .expect("target/<profile>/deps");
    let candidate = profile.join(name);
    assert!(
        candidate.is_file(),
        "the lovelace-know binary is missing at {}",
        candidate.display()
    );
    candidate
}

/// The frozen C# reference graph (seed 20240617), checked into the repository root.
pub fn reference_graph_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../knowledge-graph-demo.json")
}

/// A unique temporary graph path for one test.
pub fn temp_graph_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("lovelace-know-{}-{name}.json", std::process::id()));
    path
}

/// A path rendered for embedding in JSON (forward slashes, which Windows accepts).
pub fn json_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

/// Runs the CLI with the given arguments and stdin, returning stdout, stderr, and exit code.
pub fn run_cli(arguments: &[&str], stdin: &str) -> (String, String, i32) {
    let mut child = Command::new(cli_binary())
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the CLI starts");
    if let Some(mut pipe) = child.stdin.take() {
        pipe.write_all(stdin.as_bytes())
            .expect("the request is written");
    }
    let output = child.wait_with_output().expect("the CLI exits");
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        output.status.code().unwrap_or(-1),
    )
}

/// Re-parses a rendered value, so a test can compare against a parsed reference document.
pub fn round_trip(value: &Out) -> lovelace_json::Json {
    lovelace_json::Json::parse(&value.compact()).expect("the crate emits valid JSON")
}

/// Builds a sweep sample for reducer tests.
pub fn sweep_sample(
    index: i32,
    op: Operation,
    anchor: &str,
    position: &str,
    swept_side: &str,
    sweep_id: &str,
    sigma: &str,
) -> SampleRecord {
    let success = !sigma.starts_with("err|");
    SampleRecord {
        index,
        script: format!("{anchor} {} {position}", op.symbol()),
        op,
        left: Operand::natural(anchor),
        right: Operand::natural(position),
        sweep_id: Some(sweep_id.to_string()),
        swept_side: Some(swept_side.to_string()),
        axis_pos: Some(position.to_string()),
        sigma: sigma.to_string(),
        success,
        kind: if success {
            Some(sigma.trim_start_matches("Boolean:").to_string())
        } else {
            None
        },
        typed: None,
        error_message: sigma.strip_prefix("err|").map(str::to_string),
        sampling_kind: SamplingKind::Sweep,
        weight: 1.0,
    }
}

/// Builds a validation (held-out) sample for C3 tests.
pub fn validation_sample(
    index: i32,
    op: Operation,
    anchor: &str,
    position: &str,
    swept_side: &str,
    sigma: &str,
) -> SampleRecord {
    let mut sample = sweep_sample(index, op, anchor, position, swept_side, "validate", sigma);
    sample.sampling_kind = SamplingKind::Validate;
    sample
}
