//! Round-21 DSP parity batch, runner side: the published-envelope rows the round-20 builtins
//! unblock -- the integer-argument boundary matrix, the plugin integer-argument wire contract and
//! the DspPlugin argument refusals. The C# assertion is the contract; these tests drive the same
//! in-process runner the binary publishes.

use super::parity_batch::*;
use super::*;

fn run(source: &str) -> RunResult {
    run_args(&[
        "--eval",
        source,
        "--omit-functions",
        "--omit-variables",
        "--plot-dir",
        &plot_dir(),
    ])
}

fn accept(source: &str) -> RunResult {
    let result = run(source);
    assert_eq!(result.exit, 0, "{source}: {}", result.stdout);
    assert!(
        boolean(get(&result.env, "ok")),
        "{source}: {}",
        result.stdout
    );
    result
}

fn typed(source: &str) -> String {
    text(get(get(&accept(source).env, "result"), "typed")).to_string()
}

fn output(source: &str) -> Vec<String> {
    array(get(&accept(source).env, "output"))
        .iter()
        .map(|value| text(value).to_string())
        .collect()
}

fn refusal(source: &str) -> RunResult {
    let result = run(source);
    assert_eq!(result.exit, 1, "{source}: {}", result.stdout);
    assert!(
        !boolean(get(&result.env, "ok")),
        "{source}: {}",
        result.stdout
    );
    result
}

fn field<'a>(result: &'a RunResult, key: &str) -> &'a str {
    get(&result.env, key).as_str().unwrap_or("")
}

/// The boundary matrix row's liveness check: a text sentinel must cross as the argument refusal
/// for that parameter, naming the builtin and the parameter's anchor text.
fn assert_template_reaches_argument(builtin: &str, source: &str, anchor: &str) {
    let result = refusal(source);
    let code = field(&result, "code");
    let category = field(&result, "category");
    let message = field(&result, "message");
    assert!(
        code == "InvalidArgument" || code == "InvalidOperation",
        "{source}: expected a typed argument refusal, got {code}/{category} with '{message}'"
    );
    assert!(
        category == "TypeMismatch" || category == "DomainError",
        "{source}: expected a caller-side refusal category, got {category} with '{message}'"
    );
    assert!(
        message.starts_with(&format!("{builtin}()")),
        "{source}: the refusal must name the builtin as '{builtin}()' ({message})"
    );
    assert!(
        message.contains(anchor),
        "{source}: the refusal must contain the anchor '{anchor}' ({message})"
    );
}

// ---------------------------------------------------------------------------
// Lovelace.Run.Tests/IntegerArgumentBoundaryMatrixTests.cs -- the nine DSP inventory rows.
// Every accepted integer is driven, and the template sentinel proves the row reaches its argument.
// ---------------------------------------------------------------------------

/// Inventory rows movingavg:w (EveryBoundaryCell rows 195-197; template row 19).
#[test]
fn boundary_movingavg_w_accepts_one_two_three() {
    for value in ["1", "2", "3"] {
        let source = format!("movingavg([1, 2, 3, 4, 5], {value})");
        let typed_value = typed(&source);
        assert!(typed_value.ends_with("(Vector)"), "{source}: {typed_value}");
    }
    assert_eq!(
        typed("movingavg([1, 2, 3, 4, 5], 1)"),
        "[1, 2, 3, 4, 5] (Vector)"
    );
    assert_eq!(
        typed("movingavg([1, 2, 3, 4, 5], 3)"),
        "[0.(3), 1, 2, 3, 4] (Vector)"
    );
    assert_template_reaches_argument(
        "movingavg",
        "movingavg([1, 2, 3, 4, 5], \"sentinel\")",
        "(w)",
    );
}

/// Inventory rows delay:k (cells 205-207; template 20).
#[test]
fn boundary_delay_k_accepts_one_two_three() {
    assert_eq!(typed("delay([1, 2, 3, 4], 1)"), "[0, 1, 2, 3] (Vector)");
    assert_eq!(typed("delay([1, 2, 3, 4], 2)"), "[0, 0, 1, 2] (Vector)");
    assert_eq!(typed("delay([1, 2, 3, 4], 3)"), "[0, 0, 0, 1] (Vector)");
    assert_template_reaches_argument("delay", "delay([1, 2, 3, 4], \"sentinel\")", "(k)");
}

/// Inventory rows impulse:n (cells 223-225; template 22).
#[test]
fn boundary_impulse_n_accepts_one_two_three() {
    assert_eq!(typed("impulse(1)"), "[1] (Vector)");
    assert_eq!(typed("impulse(2)"), "[1, 0] (Vector)");
    assert_eq!(typed("impulse(3)"), "[1, 0, 0] (Vector)");
    assert_template_reaches_argument("impulse", "impulse(\"sentinel\")", "(n)");
}

/// Inventory rows step:n (cells 233-235; template 23).
#[test]
fn boundary_step_n_accepts_one_two_three() {
    assert_eq!(typed("step(1)"), "[1] (Vector)");
    assert_eq!(typed("step(2)"), "[1, 1] (Vector)");
    assert_eq!(typed("step(3)"), "[1, 1, 1] (Vector)");
    assert_template_reaches_argument("step", "step(\"sentinel\")", "(n)");
}

/// Inventory rows exponential:n (cells 243-245; template 24).
#[test]
fn boundary_exponential_n_accepts_one_two_three() {
    assert_eq!(typed("exponential(0.5, 1)"), "[1] (Vector)");
    for value in ["2", "3"] {
        let source = format!("exponential(0.5, {value})");
        assert!(typed(&source).ends_with("(Vector)"), "{source}");
    }
    assert_template_reaches_argument("exponential", "exponential(0.5, \"sentinel\")", "(n)");
}

/// Inventory rows cosine:n (cells 253-255; template 25).
#[test]
fn boundary_cosine_n_accepts_one_two_three() {
    assert_eq!(typed("cosine(0.25, 0, 1)"), "[1] (Vector)");
    assert_eq!(typed("cosine(0.25, 0, 2)"), "[1, 0] (Vector)");
    assert_eq!(typed("cosine(0.25, 0, 3)"), "[1, 0, -1] (Vector)");
    assert_template_reaches_argument("cosine", "cosine(0.25, 0, \"sentinel\")", "(n)");
}

/// Inventory rows powerseries:n (cells 263-265; template 26).
#[test]
fn boundary_powerseries_n_accepts_one_two_three() {
    assert_eq!(typed("powerseries(2, 0.5, 1)"), "[0] (Vector)");
    assert_eq!(typed("powerseries(2, 0.5, 2)"), "[0, 1] (Vector)");
    assert_eq!(typed("powerseries(2, 0.5, 3)"), "[0, 1, 1] (Vector)");
    assert_template_reaches_argument("powerseries", "powerseries(2, 0.5, \"sentinel\")", "(n)");
}

/// Inventory rows noise:n (cells 273-275; template 27).
#[test]
fn boundary_noise_n_accepts_one_two_three() {
    for value in 1..=3 {
        let source = format!("noise(1, 0, {value})");
        let result = accept(&source);
        let typed_value = text(get(get(&result.env, "result"), "typed"));
        assert!(typed_value.ends_with("(Vector)"), "{source}: {typed_value}");
    }
    assert_template_reaches_argument("noise", "noise(1, 0, \"sentinel\")", "(n)");
}

/// Inventory rows noise:seed (cells 283-285; template 28): the fourth position is the seed.
#[test]
fn boundary_noise_seed_accepts_zero_one_two() {
    let zero = typed("noise(1, 0, 0, 2)");
    let one = typed("noise(1, 0, 1, 2)");
    let two = typed("noise(1, 0, 2, 2)");
    assert!(zero.ends_with("(Vector)"));
    assert!(one.ends_with("(Vector)"));
    assert!(two.ends_with("(Vector)"));
    assert_ne!(zero, one);
    assert_ne!(one, two);
    assert_template_reaches_argument("noise", "noise(1, 0, \"sentinel\", 2)", "(seed)");
}

// ---------------------------------------------------------------------------
// Lovelace.Run.Tests/PluginIntegerArgumentWireTests.cs -- the seven DSP rows.
// ---------------------------------------------------------------------------

/// PluginIntegerArgumentWireTests.cs:130 ANonIntegralWindow_IsAnArgumentError_NotADomainFailure.
#[test]
fn nonintegral_window_is_an_argument_error_not_a_domain_failure() {
    let result = refusal("print(movingavg([1, 2, 3, 4, 5], 3.7))");
    assert_eq!(field(&result, "code"), "InvalidArgument");
    assert_eq!(field(&result, "category"), "TypeMismatch");
    assert!(boolean(get(&result.env, "recoverable")));
    assert!(field(&result, "message").contains("movingavg"));
    assert!(field(&result, "message").contains("3.7"));
}

/// PluginIntegerArgumentWireTests.cs:146 ADelayWiderThanInt64_IsTheSameTypedRefusal.
#[test]
fn delay_wider_than_int64_is_the_same_typed_refusal() {
    let too_wide = "1000000000000000000000000000000";
    let result = refusal(&format!("print(delay([1, 2, 3, 4], {too_wide}))"));
    assert_eq!(field(&result, "code"), "InvalidArgument");
    assert_eq!(field(&result, "category"), "TypeMismatch");
    assert!(field(&result, "message").contains(too_wide));
}

/// PluginIntegerArgumentWireTests.cs:159 ANegativeSampleCount_IsAnArgumentError.
#[test]
fn negative_sample_count_is_an_argument_error() {
    let result = refusal("print(impulse(-1))");
    assert_eq!(field(&result, "code"), "InvalidArgument");
    assert_eq!(field(&result, "category"), "TypeMismatch");
    assert!(field(&result, "message").contains("-1"));
}

/// PluginIntegerArgumentWireTests.cs:174 TheAcceptedShapes_StillAnswer, four theory rows.
#[test]
fn accepted_dsp_shapes_still_answer() {
    assert_eq!(
        output("print(movingavg([1, 2, 3, 4, 5], 3))"),
        vec!["[0.(3), 1, 2, 3, 4]"]
    );
    assert_eq!(
        output("print(delay([1, 2, 3, 4], 1))"),
        vec!["[0, 1, 2, 3]"]
    );
    assert_eq!(output("print(impulse(3))"), vec!["[1, 0, 0]"]);
    assert_eq!(output("print(len(step(4)))"), vec!["4"]);
}

// ---------------------------------------------------------------------------
// Lovelace.Suite.Tests/DspPluginTests.cs -- the five rows whose observable Rust behaviour matches.
// ---------------------------------------------------------------------------

/// DspPluginTests.cs:155 Noise_GivenDefaultSeed_IsReproducible.
#[test]
fn noise_given_default_seed_is_reproducible() {
    let first = typed("noise(1, 0, 3)");
    let second = typed("noise(1, 0, 3)");
    let explicit_seed = typed("noise(1, 0, 0, 3)");
    assert_eq!(first, second);
    assert_eq!(first, explicit_seed);
}

/// DspPluginTests.cs:181 Impulse_GivenNegativeN_ThrowsActionableError.
#[test]
fn impulse_given_negative_n_throws_actionable_error() {
    let result = refusal("impulse(-1)");
    assert_eq!(field(&result, "code"), "InvalidArgument");
    let message = field(&result, "message");
    assert!(message.contains("non-negative"), "{message}");
    assert!(message.contains("impulse"), "{message}");
    assert!(message.contains("(n)"), "{message}");
    assert!(message.contains("-1"), "{message}");
}

/// DspPluginTests.cs:204 Delay_GivenOutOfRangeK_ThrowsActionableError.
#[test]
fn delay_given_out_of_range_k_throws_actionable_error() {
    let result = refusal("delay([1, 2, 3], 99999999999999999999)");
    assert_eq!(field(&result, "code"), "InvalidArgument");
    let message = field(&result, "message");
    assert!(message.contains("9223372036854775807"), "{message}");
    assert!(message.contains("99999999999999999999"), "{message}");
}

/// DspPluginTests.cs:227 MovingAverage_GivenNonPositiveWindow_ThrowsActionableError.
#[test]
fn movingaverage_given_nonpositive_window_throws_actionable_error() {
    let result = refusal("movingavg([1, 2, 3], 0)");
    assert_eq!(field(&result, "code"), "InvalidArgument");
    let message = field(&result, "message");
    assert!(message.contains("positive"), "{message}");
    assert!(message.contains("movingavg"), "{message}");
    assert!(message.contains("(w)"), "{message}");
    assert!(message.contains("0"), "{message}");
}

/// DspPluginTests.cs:275 DspPlugin_GivenUntouchedKnob_ComputesAtFastBudget_AndPromotesWhenRaised.
/// The C# test also reads engine.PrecisionExplicitlySet, an engine-internal flag with no wire
/// counterpart; the observable promotion (a longer answer once the knob is raised) is asserted.
#[test]
fn dsp_plugin_untouched_knob_computes_at_fast_budget_and_promotes() {
    let fast = typed("im(dft([1, 2, 3])[1])");
    let fast_digits = decimals(&fast);
    assert!(
        (15..=45).contains(&fast_digits),
        "fast digits {fast_digits}: {fast}"
    );

    let promoted = typed("setprecision(60); im(dft([1, 2, 3])[1])");
    let promoted_digits = decimals(&promoted);
    assert!(
        promoted_digits >= fast_digits + 15,
        "expected silent promotion when the knob is raised (fast={fast_digits}, promoted={promoted_digits})"
    );
}
// ---------------------------------------------------------------------------
// Lovelace.Run.Tests/IntegerArgumentBoundaryMatrixTests.cs -- the two registry census facts
// ---------------------------------------------------------------------------

const INTEGER_SEMANTIC_NAMES: &[&str] = &[
    "dims", "rows", "cols", "axis", "axes", "order", "digits", "w", "k", "n", "index", "bound",
    "seed",
];

const INVENTORY: &[(&str, &str)] = &[
    ("zeros", "dims"),
    ("ones", "dims"),
    ("eye", "rows"),
    ("eye", "cols"),
    ("reshape", "dims"),
    ("sum", "axis"),
    ("prod", "axis"),
    ("min", "axis"),
    ("max", "axis"),
    ("mean", "axis"),
    ("norm", "axis"),
    ("concat", "axis"),
    ("setprecision", "digits"),
    ("pi", "digits"),
    ("e", "digits"),
    ("evalf", "digits"),
    ("series", "order"),
    ("evalir", "digits"),
    ("evalir_batch", "digits"),
    ("movingavg", "w"),
    ("delay", "k"),
    ("dft", "n"),
    ("impulse", "n"),
    ("step", "n"),
    ("exponential", "n"),
    ("cosine", "n"),
    ("powerseries", "n"),
    ("noise", "n"),
    ("noise", "seed"),
];

const NON_REGISTRY_INVENTORY: &[(&str, &str)] = &[("index", "index"), ("range", "bound")];

const EXCLUDED: &[(&str, &str)] = &[
    ("plot", "title"),
    ("powerseries", "k"),
    ("transpose", "perm"),
];

fn registry_has(name: &str, parameter: &str) -> bool {
    registry::BUILTINS
        .iter()
        .any(|info| info.name == name && info.parameters.contains(&parameter))
}

/// IntegerArgumentBoundaryMatrixTests.cs:206 TheInventoryCoversEveryIntegerSemanticParameterInTheRegistry.
#[test]
fn the_inventory_covers_every_integer_semantic_parameter_in_the_registry() {
    let mut required: Vec<(String, String)> = Vec::new();
    for info in registry::BUILTINS {
        for parameter in info.parameters {
            if !INTEGER_SEMANTIC_NAMES.contains(parameter) {
                continue;
            }
            if EXCLUDED
                .iter()
                .any(|(builtin, excluded)| *builtin == info.name && excluded == parameter)
            {
                continue;
            }
            required.push((info.name.to_string(), (*parameter).to_string()));
        }
    }

    for (builtin, parameter) in &required {
        assert!(
            INVENTORY
                .iter()
                .any(|(b, p)| b == builtin && p == parameter),
            "an integer-semantic argument is registered but not in the boundary matrix: {builtin}:{parameter}"
        );
    }
    for (builtin, parameter) in INVENTORY {
        assert!(
            required.iter().any(|(b, p)| b == builtin && p == parameter),
            "the boundary matrix carries an entry the registry no longer declares: {builtin}:{parameter}"
        );
    }
    for (builtin, parameter) in NON_REGISTRY_INVENTORY {
        assert!(
            !registry_has(builtin, parameter),
            "a non-registry entry now names a REGISTERED integer-semantic argument: {builtin}:{parameter}"
        );
    }

    assert!(
        registry::BUILTINS.len() >= 100,
        "the registry exposed only {} builtins",
        registry::BUILTINS.len()
    );
    assert!(
        INVENTORY.len() >= 15,
        "the inventory carries only {} rows",
        INVENTORY.len()
    );
    assert!(
        NON_REGISTRY_INVENTORY.len() >= 2,
        "the non-registry inventory lost its call sites"
    );
}

/// IntegerArgumentBoundaryMatrixTests.cs:257 EveryExclusionIsStillNeeded.
#[test]
fn every_exclusion_is_still_needed() {
    for (builtin, parameter) in EXCLUDED {
        assert!(
            registry_has(builtin, parameter),
            "'{builtin}:{parameter}' is excluded but '{builtin}' does not declare '{parameter}'"
        );
    }
}
