//! The observable recursion contract of the runner.
//!
//! The reference accepts the documented deepest recursion of the audit's shape
//! (`func f(n) { if (n == 0) { return 0 }; return f(n - 1) }` called as `f(85)`; `f(86)` is
//! refused) and the Rust runner must accept the SAME script and publish the SAME result. This test
//! drives the built binary rather than the library because the defect was in the runner process:
//! on the default native stack the debug runner died with 0xC00000FD before answering.

use std::process::Command;

use lovelace_json::Json;

/// The audit's exact shape at the documented deepest accepted depth.
const SCRIPT: &str = "func f(n) { if (n == 0) { return 0 }; return f(n - 1) }; f(85)";

#[test]
fn the_documented_deepest_recursion_is_accepted() {
    let output = Command::new(env!("CARGO_BIN_EXE_lovelace-run"))
        .args(["--eval", SCRIPT, "--omit-functions", "--omit-variables"])
        .output()
        .expect("cannot start lovelace-run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(0),
        "f(85) must exit 0; status {:?}, stderr: {stderr}",
        output.status.code()
    );
    assert!(stderr.is_empty(), "f(85) wrote to stderr: {stderr}");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let doc = Json::parse(&stdout).expect("stdout is not one JSON document");
    assert!(
        matches!(doc.get("ok"), Some(Json::Bool(true))),
        "f(85) was not reported ok: {stdout}"
    );
    let result = doc.get("result").expect("the envelope has no result");
    assert_eq!(result.get("display").and_then(|v| v.as_str()), Some("0"));
    assert_eq!(result.get("typed").and_then(|v| v.as_str()), Some("0 (Natural)"));
}
