//! Round 50: the two deferred census rows the post-round-48 tree now answers through the wire.
//!
//! Both rows are SuiteEngine state observations whose whole contract is published by the runner's
//! own envelope (the variables list, the functions list and the structural duration pair), so the
//! C# assertion can be repeated here without a library-only API. Each test drives
//! `parity_batch::run_args` — the runner's own parse/evaluate/envelope path — with the C#
//! case's exact script; the oracle (pinned C# runner) publishes the same normalised envelope for
//! the same argv (measured in this round's sweep).

use super::parity_batch::*;
use lovelace_json::Json;

/// SuiteEngineTests.cs:29 `CaptureState_GivenVariablesAndFunctions_ReturnsMatchingSnapshot`:
/// after `x = 3.14` and `func f(y) = y * 2` the captured state holds x as a Real and
/// f as a user (non-builtin) function. The runner publishes exactly that snapshot as its
/// `variables` and `functions` arrays; the C# alias `_` for the last value is ignored.
#[test]
fn capture_state_publishes_variables_and_user_functions() {
    let r = run_args(&["--eval", "x = 3.14; func f(y) = y * 2"]);
    assert_eq!(r.exit, 0, "{}", r.env);
    assert!(boolean(get(&r.env, "ok")), "{}", r.env);

    let variables = array(get(&r.env, "variables"));
    let x = variables
        .iter()
        .find(|v| text(get(v, "name")) == "x")
        .unwrap_or_else(|| panic!("the captured state lost the variable x: {}", r.env));
    assert_eq!(text(get(x, "kind")), "Real", "x must be captured as a Real");

    let functions = array(get(&r.env, "functions"));
    let f = functions
        .iter()
        .find(|v| text(get(v, "name")) == "f")
        .unwrap_or_else(|| panic!("the captured state lost the function f: {}", r.env));
    assert!(
        !boolean(get(f, "builtin")),
        "f is a user function, so it must not be published as a builtin"
    );
}

/// SuiteEngineTests.cs:212 `EvaluateAsync_GivenScript_RecordsElapsedTime`: a completed
/// evaluation records a non-negative elapsed duration and a non-empty display form. On the wire
/// those are the structural `elapsedTime` pair (invariant 4) and the `elapsed` display string.
#[test]
fn a_completed_script_records_its_elapsed_time() {
    let r = run_args(&["--eval", "1 + 1"]);
    assert_eq!(r.exit, 0, "{}", r.env);
    assert!(boolean(get(&r.env, "ok")), "{}", r.env);

    let elapsed_time = get(&r.env, "elapsedTime");
    let value = match get(elapsed_time, "value") {
        Json::Num(raw) => raw
            .parse::<f64>()
            .unwrap_or_else(|_| panic!("elapsedTime.value is not a number: {raw}")),
        other => panic!("elapsedTime.value is not numeric: {other}"),
    };
    assert!(value >= 0.0, "elapsed must not be negative: {value}");
    let unit = text(get(elapsed_time, "unit"));
    assert!(
        !unit.trim().is_empty(),
        "elapsedTime.unit must not be empty"
    );

    let display = text(get(&r.env, "elapsed"));
    assert!(
        !display.trim().is_empty(),
        "the human elapsed display must not be empty"
    );
}
/// The runner's descriptor query for one builtin: `--function <name>` publishes the same
/// BuiltinDescriptor the C# `HelpService.Function` text is rendered from.
fn describe(name: &str) -> RunResult {
    let r = run_args(&["--eval", "1", "--json", "--function", name]);
    assert_eq!(r.exit, 0, "--function {name}: {}", r.env);
    assert!(boolean(get(&r.env, "ok")), "--function {name}: {}", r.env);
    r
}

/// HyperbolicBuiltinsAndNegativeIntegerPowersTests.cs:77 `HyperbolicBuiltins_HaveCompleteDescriptors`:
/// the descriptor carries a summary, a real example, a declared return kind, and a cross-reference
/// that resolves to a real builtin which references it back. The C# assertions are on the help TEXT
/// rendered from these fields; the runner publishes the fields themselves, so each textual
/// assertion becomes the field it is rendered from.
fn assert_hyperbolic_descriptor(name: &str, inverse: &str, example: &str) {
    let r = describe(name);
    let descriptor = get(&r.env, "function");
    assert_eq!(text(get(descriptor, "name")), name);

    let summary = text(get(descriptor, "summary"));
    assert!(
        !summary.trim().is_empty() && !summary.contains("(no summary registered)"),
        "{name}: the descriptor registered no summary"
    );

    // "help contains <name>(x)" is the declared parameter list; it must be exactly the one the
    // rendered help shows.
    let parameters: Vec<&str> = array(get(descriptor, "parameters"))
        .iter()
        .map(text)
        .collect();
    assert_eq!(
        parameters,
        vec!["x"],
        "{name}: help would not render {name}(x)"
    );

    let examples: Vec<&str> = array(get(descriptor, "examples"))
        .iter()
        .map(text)
        .collect();
    assert!(
        examples.contains(&example),
        "{name}: the help text must carry the example '{example}', got {examples:?}"
    );

    // "must not contain 'Returns: Value'" — the declared return kind is the Symbolic one.
    assert_eq!(
        text(get(descriptor, "returnKind")),
        "Symbolic",
        "{name}: the help would render a Value return kind"
    );

    // The cross-reference must be a REAL function, and the counterpart must reference it back.
    let inverse_run = describe(inverse);
    let inverse_descriptor = get(&inverse_run.env, "function");
    assert!(
        boolean(get(inverse_descriptor, "builtin")),
        "{inverse} must exist as a builtin for the cross-reference to resolve"
    );
    let related: Vec<&str> = array(get(descriptor, "related")).iter().map(text).collect();
    assert!(
        related.contains(&inverse),
        "{name}: the help must name {inverse}, got {related:?}"
    );
    let inverse_related: Vec<&str> = array(get(inverse_descriptor, "related"))
        .iter()
        .map(text)
        .collect();
    assert!(
        inverse_related.contains(&name),
        "{inverse}: the help must name {name} back, got {inverse_related:?}"
    );
}

#[test]
fn asinh_descriptor_is_complete_and_cross_references_sinh() {
    assert_hyperbolic_descriptor("asinh", "sinh", "asinh(0)");
}

#[test]
fn acosh_descriptor_is_complete_and_cross_references_cosh() {
    assert_hyperbolic_descriptor("acosh", "cosh", "acosh(1)");
}

#[test]
fn atanh_descriptor_is_complete_and_cross_references_tanh() {
    assert_hyperbolic_descriptor("atanh", "tanh", "atanh(0)");
}

/// CapabilitiesBuiltinTests.cs:652 `Capabilities_IsOneRegisteredBuiltin_WithCompleteMetadata`:
/// exactly one builtin named capabilities, registered by the Symbolics plugin, with a complete
/// descriptor (category Introspection, no parameters, a summary, examples, related entries, and
/// the CapabilitiesResult return kind) whose rendered help is not the placeholder.
#[test]
fn capabilities_is_one_registered_builtin_with_complete_metadata() {
    let registry = run_args(&["--eval", "1", "--json"]);
    assert_eq!(registry.exit, 0, "{}", registry.env);
    let named: Vec<&Json> = array(get(&registry.env, "functions"))
        .iter()
        .filter(|f| text(get(f, "name")) == "capabilities")
        .collect();
    assert_eq!(
        named.len(),
        1,
        "capabilities() must be registered exactly once: {}",
        registry.env
    );

    let r = describe("capabilities");
    let descriptor = get(&r.env, "function");
    assert_eq!(text(get(descriptor, "plugin")), "Lovelace.Symbolics");
    assert_eq!(text(get(descriptor, "category")), "Introspection");
    assert!(
        array(get(descriptor, "parameters")).is_empty(),
        "capabilities() takes no parameters"
    );
    assert!(!text(get(descriptor, "summary")).trim().is_empty());
    assert!(!array(get(descriptor, "examples")).is_empty());
    assert!(!array(get(descriptor, "related")).is_empty());
    assert_eq!(text(get(descriptor, "returnKind")), "CapabilitiesResult");

    // the help contract: a real summary, and never a placeholder return kind
    let summary = text(get(descriptor, "summary"));
    assert!(!summary.contains("(no summary registered)"));
    assert_ne!(text(get(descriptor, "returnKind")), "Value");
}
