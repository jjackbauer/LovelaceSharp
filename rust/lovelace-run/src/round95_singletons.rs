//! Round 95 — the runner-side halves of the language/engine singleton rows.
//!
//! Each test drives the runner's own `run_args` (the same `execute`/`envelope_for` pair `main`
//! runs), so a wire assertion cannot pass on a flow the binary does not execute. The language-tier
//! halves (scoping, projection, the contract constructor) live in
//! `lovelace-lang::round95_engine_singletons`; this module pins what crosses the wire.

use super::parity_batch::*;
use super::*;

/// The interpreter's walk is native recursion, so the 120-deep nest the oracle measures needs a
/// thread with a realistic stack; the refusal is about the evaluation budget, not about Rust's
/// default test stack.
fn with_large_stack<T: Send + 'static>(probe: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(probe)
        .expect("probe thread")
        .join()
        .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
}

fn run_script(script: &str) -> RunResult {
    let script = script.to_string();
    with_large_stack(move || run_args(&["--eval", &script, "--json", "--omit-functions"]))
}

/// The names the envelope publishes in its `variables` array.
fn variable_names(env: &lovelace_json::Json) -> Vec<String> {
    array(get(env, "variables"))
        .iter()
        .map(|variable| text(get(variable, "name")).to_string())
        .collect()
}

// ---------------------------------------------------------------------------
// Recursion depth on the wire
// ---------------------------------------------------------------------------

/// `RecursionDepthEnvelopeTests.NestedBlocksInsideARecursiveBody_AreRefused_InsteadOfDying`: the
/// audit's 120 nested `if`s at five call levels. The pre-fix Rust run ANSWERED here (the oracle
/// refuses with `DepthExceeded`); the refused row is the exact shape this asserts.
#[test]
fn recursiondepthenvelope_nested_blocks_inside_a_recursive_body_are_refused() {
    let script = format!(
        "func f(n) {{ if (n == 0) {{ return 0 }}; {}return f(n - 1){} }}\nf(5)",
        "if (1 == 1) { ".repeat(120),
        " }".repeat(120)
    );
    let r = run_script(&script);

    assert_eq!(r.exit, 1, "{}", r.stdout);
    assert!(!boolean(get(&r.env, "ok")));
    assert_eq!(text(get(&r.env, "code")), "DepthExceeded");
    assert_eq!(text(get(&r.env, "category")), "BudgetExceeded");
    assert!(boolean(get(&r.env, "recoverable")));
    assert!(
        text(get(&r.env, "message"))
            .contains("exceeds the maximum supported evaluation depth of 512"),
        "{}",
        r.stdout
    );
}

/// `RecursionDepthGuardTests.ADeepExpressionInsideTheRecursiveBody_IsRefused` on the wire: ten
/// call levels with a 100-term body chain used to ANSWER in Rust and is a typed refusal now.
#[test]
fn recursiondepthguard_a_deep_expression_inside_the_recursive_body_is_refused() {
    let script = format!(
        "func f(n) {{ if (n == 0) {{ return 0 }}; return f(n - 1){} }}\nf(10)",
        " + 1".repeat(100)
    );
    let r = run_script(&script);

    assert_eq!(r.exit, 1, "{}", r.stdout);
    assert_eq!(text(get(&r.env, "code")), "DepthExceeded");
    assert_eq!(text(get(&r.env, "category")), "BudgetExceeded");
    assert!(boolean(get(&r.env, "recoverable")));
}

/// The boundary the sibling recursion rows publish still holds: f(85) answers and f(86) refuses.
#[test]
fn recursiondepthguard_the_published_boundary_still_holds_on_the_wire() {
    with_large_stack(|| {
        let accepted = run_args(&[
            "--eval",
            "func f(n) { if (n == 0) { return 0 }; return f(n - 1) }; f(85)",
            "--json",
            "--omit-functions",
            "--omit-variables",
        ]);
        assert_eq!(accepted.exit, 0, "{}", accepted.stdout);
        assert!(boolean(get(&accepted.env, "ok")));
        assert_eq!(text(get(get(&accepted.env, "result"), "display")), "0");

        let refused = run_args(&[
            "--eval",
            "func f(n) { if (n == 0) { return 0 }; return f(n - 1) }; f(86)",
            "--json",
            "--omit-functions",
            "--omit-variables",
        ]);
        assert_eq!(refused.exit, 1, "{}", refused.stdout);
        assert_eq!(text(get(&refused.env, "code")), "DepthExceeded");
    });
}

// ---------------------------------------------------------------------------
// Broadcast on the wire
// ---------------------------------------------------------------------------

/// `BroadcastTests.ColumnVector_BroadcastsToMatrix` on the wire: the deferred row's exact
/// expression now answers the reference's typed array instead of a broadcast refusal.
#[test]
fn broadcasttests_column_vector_broadcasts_to_matrix() {
    let r = run_args(&[
        "--eval",
        "[[1], [2]] + [[10, 20], [30, 40]]",
        "--json",
        "--omit-functions",
        "--omit-variables",
    ]);

    assert_eq!(r.exit, 0, "{}", r.stdout);
    assert!(boolean(get(&r.env, "ok")));
    assert_eq!(
        text(get(get(&r.env, "result"), "typed")),
        "[[11, 21], [32, 42]] (Array)"
    );
    assert_eq!(
        text(get(get(get(&r.env, "result"), "structured"), "kind")),
        "Array"
    );
}

// ---------------------------------------------------------------------------
// Scoping on the wire
// ---------------------------------------------------------------------------

/// `FunctionTests.Evaluate_GivenFunctionLocalVariable_DoesNotLeakToGlobal`: the envelope's
/// `variables` array holds only the result variable, so the function-local definition did not
/// become a global.
#[test]
fn functiontests_function_local_variable_does_not_leak_to_global() {
    let r = run_script("func f() { local = 42 }; f()");

    assert_eq!(r.exit, 0, "{}", r.stdout);
    assert_eq!(variable_names(&r.env), vec!["_".to_string()]);
}

/// `StatementTests.Evaluate_GivenVariableAssignmentInsideBlock_DoesNotLeak`: the same wire
/// observation for a braced block, whose own definition is dropped with the block.
#[test]
fn statementtests_block_assignment_does_not_leak() {
    let r = run_script("{ z = 99 }");

    assert_eq!(r.exit, 0, "{}", r.stdout);
    assert_eq!(variable_names(&r.env), vec!["_".to_string()]);
    assert_eq!(text(get(get(&r.env, "result"), "display")), "99");
}

// ---------------------------------------------------------------------------
// One projection: the façade and the CLI publish the same digits
// ---------------------------------------------------------------------------

/// `LibraryProjectionAgreementTests.TheFacade_AndTheCli_PublishTheSameDigits_ForTheFindingsScript`:
/// one computation, two public entry points, one payload. The CLI's structured value must equal the
/// library façade's, and both must carry every stored digit.
#[test]
fn libraryprojectionagreement_the_cli_and_the_facade_publish_the_same_digits() {
    let r = run_args(&[
        "--eval",
        "setprecision(1100); pi(1100)",
        "--json",
        "--omit-functions",
        "--omit-variables",
    ]);
    assert_eq!(r.exit, 0, "{}", r.stdout);
    let cli_value = text(get(get(get(&r.env, "result"), "structured"), "value")).to_string();

    let mut engine = Engine::new();
    engine.set_precision(1100);
    engine.set_display_precision(100);
    let value = engine.eval("pi(1100)").expect("pi(1100)");
    let facade = lovelace_lang::structured::project_value(&engine, &value, None);
    let facade_value = facade
        .get("value")
        .and_then(lovelace_json::Json::as_str)
        .expect("a Real publishes its value")
        .to_string();

    assert_eq!(cli_value, facade_value);
    let dot = cli_value.find('.').expect("a fractional value");
    assert_eq!(cli_value.len() - dot - 1, 1100);
}

// ---------------------------------------------------------------------------
// The error-category mirror
// ---------------------------------------------------------------------------

/// `ErrorCategoryMirrorTests.EveryMirroredTaxonomy_EqualsTheEnum`: the three literal arrays the
/// envelope families assert their categories against must equal the published enum, so a category
/// cannot be observed as a string no one declared. This is the one place that fails when the enum
/// gains a member and a mirror is not updated.
#[test]
fn errorcategorymirror_every_mirrored_taxonomy_equals_the_enum() {
    let declared: Vec<&str> = lovelace_lang::diagnostic::ErrorCategory::ALL
        .iter()
        .map(|category| category.name())
        .collect();

    assert_eq!(
        declared,
        super::round28_run_batch::MIRRORED_ERROR_CATEGORIES.to_vec(),
        "the allocation-refusal family's mirror"
    );
    assert_eq!(
        declared,
        super::plot_surface_wire::MIRRORED_ERROR_CATEGORIES.to_vec(),
        "the plot-directory family's mirror"
    );
    assert_eq!(
        declared,
        super::round90_runner_families::MIRRORED_ERROR_CATEGORIES.to_vec(),
        "the plot-file family's mirror"
    );
}
