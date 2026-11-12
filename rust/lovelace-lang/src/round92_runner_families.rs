//! Round 92: the twelve deferred runner/Suite rows across CancellationObservation,
//! SuiteEngineOutputPlot, HelpService, NameSuggestions, CapabilitiesBuiltin and
//! DiagnosticContract.
//!
//! Each test names the census id it closes and asserts the C# case's own observable. The oracle
//! was measured with `Lovelace.Run --eval/--file --json --omit-functions`; the two capability rows
//! and the solve/limit note row are driven through the same engine envelope, and the library-only
//! rows (cancellation token, output writer, help view, name metric, category vocabulary) through
//! the public Rust surface that now carries the same contract.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::help;
use crate::name_suggestions;
use crate::{Engine, ErrorCategory, Value};

fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    match value {
        Value::Record(_, fields) => fields
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
            .unwrap_or_else(|| panic!("record has no field '{name}'")),
        other => panic!("not a record: {}", other.kind_name()),
    }
}

fn record_text(value: &Value, name: &str) -> String {
    match field(value, name) {
        Value::Text(text) => text.clone(),
        other => panic!("field '{name}' is not Text: {}", other.kind_name()),
    }
}

fn record_enum(value: &Value, name: &str) -> String {
    match field(value, name) {
        Value::Enum(_, member) => member.clone(),
        other => panic!("field '{name}' is not an Enum: {}", other.kind_name()),
    }
}

fn elements(value: &Value) -> Vec<Value> {
    match value {
        Value::Vector(items) => items.clone(),
        Value::Array(array) => array.iter().cloned().collect(),
        other => panic!("not a vector/array: {}", other.kind_name()),
    }
}

fn unique_plot_dir(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../target/tmp")
        .join(format!("round92-{label}-{}-{nanos}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a fresh probe directory");
    dir
}

// ---------------------------------------------------------------------------
// Lovelace.Suite.Tests/CancellationObservationTests
// ---------------------------------------------------------------------------

/// The test-side fence the reference uses: a non-observing implementation fails rather than hangs.
const CANCELLATION_FENCE: Duration = Duration::from_secs(20);

fn assert_cancelled_promptly(script: &str) {
    let mut engine = Engine::new();
    let started = Instant::now();
    let error = match engine.eval_with_budget(script, Duration::from_millis(1500)) {
        Ok(value) => panic!("'{script}' must be refused, got {}", value.kind_name()),
        Err(error) => error,
    };
    let elapsed = started.elapsed();
    assert!(
        elapsed < CANCELLATION_FENCE,
        "'{script}' observed cancellation only after {elapsed:?}; the budget was 1500 ms"
    );
    assert_eq!(error.code, "Cancelled", "'{script}': {error:?}");
    assert_eq!(error.category, "BudgetExceeded");
    assert_eq!(error.message, crate::CANCELLED_MESSAGE);
    assert!(
        engine.was_cancelled(),
        "'{script}': the token was not observed"
    );
}

/// CancellationObservationTests.Evaluate_GivenLongNaturalPowerAndShortBudget_CancelsPromptly.
#[test]
fn evaluate_given_long_natural_power_and_short_budget_cancels_promptly() {
    assert_cancelled_promptly("2^1000000000");
}

/// CancellationObservationTests.Evaluate_GivenLongFactorialAndShortBudget_CancelsPromptly.
#[test]
fn evaluate_given_long_factorial_and_short_budget_cancels_promptly() {
    assert_cancelled_promptly("2000000!");
}

// ---------------------------------------------------------------------------
// Lovelace.Suite.Tests/SuiteEngineOutputPlotTests
// ---------------------------------------------------------------------------

/// EvaluateAsync_GivenOutputWriter_CapturesPrintLinesToThatWriter: the print lines go to the
/// caller's writer, and the engine's previous output is restored (the reference's finally).
#[test]
fn evaluate_async_given_output_writer_captures_print_lines_to_that_writer() {
    let mut engine = Engine::new();
    engine
        .eval("print(\"before\")")
        .expect("the prelude prints");
    let mut capture: Vec<u8> = Vec::new();
    let _ = engine
        .eval_with_output("print(\"hi\")", &mut capture)
        .expect("the print call answers");
    assert_eq!(String::from_utf8(capture).expect("utf8").trim(), "hi");
    assert_eq!(engine.take_output(), vec!["before".to_string()]);
}

/// ResetPlotCapture_GivenPreviousPlot_ClearsLastPlot.
#[test]
fn reset_plot_capture_given_previous_plot_clears_last_plot() {
    let mut engine = Engine::new();
    let directory = unique_plot_dir("reset");
    engine.set_plot_dir(&directory.to_string_lossy());
    engine.set_plot_file("test.svg");
    engine
        .eval("plot([1, 2, 3])")
        .expect("the plot is captured");
    assert!(engine.last_plot().is_some(), "the plot must be captured");
    engine.reset_plot_capture();
    assert!(engine.last_plot().is_none(), "the capture must be cleared");
    let _ = std::fs::remove_dir_all(&directory);
}

// ---------------------------------------------------------------------------
// Lovelace.Suite.Tests/HelpServiceTests
// ---------------------------------------------------------------------------

/// Help_Overview_ListsAllCategories: every category with functions in `funcs(null)` is listed by
/// the overview, together with the Symbolics/Solving headings and the `help <function>` line.
#[test]
fn help_overview_lists_all_categories() {
    let overview = help::overview();
    let listed = help::funcs(None);
    for category in help::CATEGORY_ORDER {
        if listed.contains(category) {
            assert!(overview.contains(category), "overview lacks {category}");
        }
    }
    assert!(overview.contains("Symbolics"));
    assert!(overview.contains("Solving"));
    assert!(overview.contains("help <function>"));
}

/// UnicodeOutput_ThreadsThroughEngineFormatting: ASCII is the default, the knob turns inf into ∞,
/// and clearing it restores the ASCII form.
#[test]
fn unicode_output_threads_through_engine_formatting() {
    let mut engine = Engine::new();
    engine.eval("x = symbol(\"x\")").expect("symbol");
    let infinity = engine.eval("inf").expect("inf");
    assert_eq!(engine.format_typed(&infinity), "inf (Symbolic)");
    engine.set_unicode_output(true);
    assert!(engine.unicode_output());
    assert_eq!(engine.format_typed(&infinity), "∞ (Symbolic)");
    engine.set_unicode_output(false);
    assert_eq!(engine.format_typed(&infinity), "inf (Symbolic)");
}

// ---------------------------------------------------------------------------
// Lovelace.Suite.Tests/NameSuggestionsTests
// ---------------------------------------------------------------------------

/// The registry shaped like the live one for the names these tests exercise.
const REGISTRY: &[&str] = &[
    "det",
    "diff",
    "inv",
    "mean",
    "len",
    "factor",
    "shape",
    "trace",
    "eye",
    "numel",
    "matmul",
    "transpose",
    "sum",
    "dot",
    "solve",
    "solve_full",
    "solve_system",
];

/// ANameNothingIsCloseTo_IsNotGuessedAt.
#[test]
fn a_name_nothing_is_close_to_is_not_guessed_at() {
    for queried in ["fft", "nosuchfn", "push", "zip"] {
        assert!(
            name_suggestions::suggest(REGISTRY, queried).is_empty(),
            "'{queried}' must not be guessed at"
        );
    }
}

/// EditDistance_IsTheStructuralMetric (#3, the `int.MaxValue` sentinel row).
#[test]
fn edit_distance_is_the_structural_metric() {
    assert_eq!(name_suggestions::edit_distance("transpose", "tranpose"), 1);
    assert_eq!(name_suggestions::edit_distance("det", "dot"), 1);
    assert_eq!(name_suggestions::edit_distance("transpose", "tranpsose"), 2);
    assert_eq!(
        name_suggestions::edit_distance("solve_full", "solve_x"),
        i32::MAX
    );
}

// ---------------------------------------------------------------------------
// Lovelace.Symbolics.Tests/CapabilitiesBuiltinTests
// ---------------------------------------------------------------------------

struct Advertised {
    code: String,
    category: String,
    message: String,
    trigger: String,
    scope: String,
}

fn advertised_capabilities(engine: &mut Engine) -> Vec<(String, Advertised)> {
    let capabilities = engine.eval("capabilities()").expect("capabilities");
    let operations = elements(field(&capabilities, "unsupported_operations"));
    operations
        .iter()
        .map(|entry| {
            let category = record_enum(entry, "category");
            (
                record_text(entry, "operation_class"),
                Advertised {
                    code: record_text(entry, "code"),
                    category,
                    message: record_text(entry, "message"),
                    trigger: record_text(entry, "trigger"),
                    scope: match field(entry, "scope") {
                        Value::Text(text) => text.clone(),
                        Value::Void => String::new(),
                        other => panic!("scope is {}", other.kind_name()),
                    },
                },
            )
        })
        .collect()
}

/// AdvertisedCodesAndCategories_MatchTheLiveEnvelope: every advertised entry is RUN through the
/// live call and its carrier (error envelope or record diagnostic) is checked field by field.
#[test]
fn advertised_codes_and_categories_match_the_live_envelope() {
    let mut engine = Engine::new();
    let advertised = advertised_capabilities(&mut engine);
    assert_eq!(advertised.len(), 19);
    let mut failures: Vec<String> = Vec::new();
    for (operation_class, entry) in &advertised {
        let live = Engine::new().eval(&entry.trigger);
        match live {
            Err(error) => {
                if error.code != entry.code
                    || error.category != entry.category
                    || error.message != entry.message
                {
                    failures.push(format!(
                        "'{operation_class}' trigger '{}': advertised {}/{} \"{}\" but live produced {}/{} \"{}\"",
                        entry.trigger, entry.code, entry.category, entry.message, error.code, error.category, error.message
                    ));
                }
            }
            Ok(value) => {
                match &value {
                    Value::Record(_, _) => {}
                    other => {
                        failures.push(format!(
                            "'{operation_class}': live call succeeded with {} instead of a record",
                            other.kind_name()
                        ));
                        continue;
                    }
                }
                let status = record_enum(&value, "status");
                assert!(
                    [
                        "Unevaluated",
                        "Partial",
                        "Failed",
                        "BudgetExceeded",
                        "Unsatisfiable"
                    ]
                    .contains(&status.as_str()),
                    "'{operation_class}' marks a record-carried refusal but status is {status}"
                );
                let diagnostics = elements(field(&value, "diagnostics"));
                let diagnostic = diagnostics
                    .iter()
                    .find(|diagnostic| record_text(diagnostic, "code") == entry.code)
                    .unwrap_or_else(|| {
                        panic!(
                            "'{operation_class}' carried no diagnostic with code '{}'",
                            entry.code
                        )
                    });
                let category = record_enum(diagnostic, "category");
                let message = record_text(diagnostic, "message");
                if category != entry.category || message != entry.message {
                    failures.push(format!(
                        "'{operation_class}' trigger '{}': advertised {}/\"{}\" but record carried {}/\"{}\"",
                        entry.trigger, entry.category, entry.message, category, message
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// PowerExponentClass_RefusesWhatItSays_AndNamesWhatIsSupported: both directions of the power
/// boundary, plus the scope text that names the supported counterexamples.
#[test]
fn power_exponent_class_refuses_what_it_says_and_names_what_is_supported() {
    let mut engine = Engine::new();
    let advertised = advertised_capabilities(&mut engine);
    let find = |class: &str| {
        advertised
            .iter()
            .find(|(name, _)| name == class)
            .map(|(_, entry)| entry)
            .unwrap_or_else(|| panic!("missing class {class}"))
    };
    let positive = find("pow.non-integer-exponent-of-a-positive-base");
    let negative = find("pow.negative-base-unrepresentable-exponent");
    assert!(
        !advertised
            .iter()
            .any(|(name, _)| name == "pow.non-integer-exponent")
    );

    for script in [
        "2^(1/2)",
        "4^(1/2)",
        "9^(1/2)",
        "0.5^(1/2)",
        "2^(1/3)",
        "2^(2/3)",
        "2^(-1/2)",
        "(-8)^(1/3)",
        "(-4)^(1/4)",
        "(-4)^(2/3)",
        "(-4)^(3/2)",
        "(-4)^(-1/2)",
    ] {
        let error = Engine::new()
            .eval(script)
            .expect_err("the advertised refusal must be live");
        assert_eq!(error.code, "UnsupportedOperation", "{script}");
        assert_eq!(error.category, "UnsupportedOperation", "{script}");
        assert_eq!(error.message, positive.message, "{script}");
    }

    for (script, expected) in [
        ("(-4)^(1/2)", "2*i"),
        ("(-1)^(1/2)", "i"),
        ("(-9)^(1/2)", "3*i"),
        ("(-2)^(1/2)", "i*sqrt(2)"),
        ("(-4)^0.5", "2*i"),
        ("(-4)^(2/4)", "2*i"),
        ("0^(1/2)", "0"),
    ] {
        let value = Engine::new()
            .eval(script)
            .unwrap_or_else(|error| panic!("'{script}' must answer: {error:?}"));
        let text = Engine::new().format(&value);
        assert_eq!(text, expected, "{script}");
    }

    assert!(positive.scope.contains("(-4)^(1/2)"));
    assert!(negative.scope.contains("(-4)^(1/2)"));
    assert!(positive.scope.to_lowercase().contains("positive base"));
    assert!(negative.scope.to_lowercase().contains("negative base"));
}

// ---------------------------------------------------------------------------
// Lovelace.Symbolics.Tests/DiagnosticContractTests
// ---------------------------------------------------------------------------

/// ErrorCategory_HasExactlyTheSevenFrozenMembers.
#[test]
fn error_category_has_exactly_the_seven_frozen_members() {
    let names: Vec<&str> = ErrorCategory::ALL.iter().map(|c| c.name()).collect();
    assert_eq!(
        names,
        vec![
            "ParseError",
            "DomainError",
            "UnsupportedOperation",
            "BudgetExceeded",
            "NoSolution",
            "TypeMismatch",
            "InternalInvariantFailure",
        ]
    );
    assert_eq!(ErrorCategory::ALL.len(), 7);
    for category in ErrorCategory::ALL {
        assert_eq!(ErrorCategory::from_name(category.name()), Some(category));
    }
}

/// KernelNotes_CrossAsDiagnosticMessages: the kernel's free-text notes survive as diagnostics.
#[test]
fn kernel_notes_cross_as_diagnostic_messages() {
    let mut engine = Engine::new();
    engine.eval("x = symbol(\"x\")").expect("symbol");
    let solve = engine
        .eval("solve_full(x == x, x)")
        .expect("identity solve answers");
    assert_eq!(record_enum(&solve, "status"), "Unevaluated");
    let solve_diagnostic = elements(field(&solve, "diagnostics"))[0].clone();
    assert_eq!(record_text(&solve_diagnostic, "code"), "solve.unevaluated");
    assert_eq!(
        record_enum(&solve_diagnostic, "category"),
        "UnsupportedOperation"
    );
    assert_eq!(
        record_text(&solve_diagnostic, "message"),
        "0 = 0: every value is a solution."
    );

    let mut limit_engine = Engine::new();
    limit_engine
        .eval("a = symbol(\"a\"); x = symbol(\"x\")")
        .expect("symbols");
    let limit = limit_engine
        .eval("limit_full(a/x, x, 0)")
        .expect("symbolic limit answers");
    assert_eq!(record_enum(&limit, "status"), "Unevaluated");
    let limit_diagnostic = elements(field(&limit, "diagnostics"))[0].clone();
    assert_eq!(record_text(&limit_diagnostic, "code"), "limit.unevaluated");
    assert_eq!(
        record_enum(&limit_diagnostic, "category"),
        "UnsupportedOperation"
    );
    assert_eq!(
        record_text(&limit_diagnostic, "message"),
        "leading coefficient is symbolic"
    );
}
