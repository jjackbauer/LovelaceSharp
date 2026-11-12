//! Round 93 — the six deferred eval/print families (12 rows), one test per census id.
//!
//! * `EvalirDigitArgumentContractTests/AnExactlyIntegralRealLiteral_IsTheIntegerItDenotes#0..#1`
//!   — the runner's `2.0` digit argument is the integer 2, for `evalir` and `evalir_batch`;
//! * `PrintBudgetTests/DepthBudget_ReportsDepthLimit#0` and
//!   `PrintBudgetTests/NodeBudget_OnACompactRendering_DropsNothingAndSaysSo#0` — the kernel
//!   budget's depth reason and its "nothing dropped, nothing reported" rule;
//! * `PrintBudgetTokenBoundaryTests/AnIdentifierLongerThanTheAllowance_IsKeptWhole#0` and
//!   `.../ARenderingWithinTheAllowance_IsUntouched#0` — the token-boundary cut and the compact
//!   rendering the allowance holds whole;
//! * `TruncatingEvaluationStaysInexactTests/FunctionOfATruncatedArgument_IsInexact#0` and
//!   `.../LiteralRoundTrip_KeepsGenuinelyExactValuesExact#0` — the route crossing a real literal;
//! * `ValueDepthGuardTests/ADeepRecordChain_IsRefusedTheSameWay#0` and
//!   `.../ValueOnePastTheBudget_IsRefusedByTheStructuredProjection#0` — the value budget at the
//!   two walks;
//! * `DocsSyncTests/Every_CSharpSnippet_AppearsInUsageExamples#0` and
//!   `.../Every_LovelaceExample_HasAnExpectedResult#0` — the usage guide's sync contract.
//!
//! Every expected value is the C# assertion, not this crate's output.

use std::path::PathBuf;

use lovelace_array::NdArray;
use lovelace_json::Json;
use lovelace_nat::Nat;
use lovelace_real::Real;
use lovelace_symbolics::{Expr, PrintMode, expand};

use crate::docs_sync::{check_csharp_snippets_appear, parse_examples};
use crate::evaluation_boundary::{builtin_numeric_function, evaluate_to_num};
use crate::numeric::Num;
use crate::print_budget::{PrintBudget, expr_depth, print};
use crate::real_literal_route::RealLiteral;
use crate::{Engine, Value};

// ---------------------------------------------------------------------------
// EvalirDigitArgumentContractTests
// ---------------------------------------------------------------------------

fn evalir_call(builtin: &str, digits: &str) -> String {
    let values = if builtin == "evalir" { "[1]" } else { "[1, 2]" };
    format!("x = symbol(\"x\"); {builtin}(compile(x*1/3, [x]), {values}, {digits})")
}

/// The display plus the published exactness of one evalir answer. A 2-place answer to
/// 1/3 truncates a non-terminating expansion, so the oracle publishes `exact:false` too.
fn evalir_answer(builtin: &str, digits: &str) -> (String, Vec<bool>) {
    let script = evalir_call(builtin, digits);
    let mut engine = Engine::new();
    let value = engine
        .eval(&script)
        .unwrap_or_else(|error| panic!("{script}: {}", error.message));
    let structured = crate::structured::project_value(&engine, &value, None);
    let exactness = match structured.get("elements") {
        Some(Json::Arr(items)) => items
            .iter()
            .map(|item| matches!(item.get("exact"), Some(Json::Bool(true))))
            .collect(),
        _ => vec![matches!(structured.get("exact"), Some(Json::Bool(true)))],
    };
    (value.display(), exactness)
}

fn assert_integral_real_digits_answer_like_the_integer(builtin: &str) {
    let (from_integral_real, exactness) = evalir_answer(builtin, "2.0");
    let (from_integer, _) = evalir_answer(builtin, "2");

    assert_eq!(
        from_integer, from_integral_real,
        "{builtin}: 2.0 is the integer 2 and must answer the same count"
    );
    assert!(
        from_integral_real.contains("0.33"),
        "{builtin}: 2.0 answered {from_integral_real}, not two places"
    );
    assert!(
        !from_integral_real.contains("0.333"),
        "{builtin}: 2.0 answered more than two places: {from_integral_real}"
    );
    assert!(
        exactness.iter().all(|exact| !exact),
        "{builtin}: a 2-place answer to 1/3 was published exact: {exactness:?}"
    );
}

/// `EvalirDigitArgumentContractTests.cs:125` row `#0` (evalir).
#[test]
fn evalirdigitargumentcontracttests_an_exactly_integral_real_literal_is_the_integer_it_denotes_0() {
    assert_integral_real_digits_answer_like_the_integer("evalir");
}

/// `EvalirDigitArgumentContractTests.cs:125` row `#1` (evalir_batch).
#[test]
fn evalirdigitargumentcontracttests_an_exactly_integral_real_literal_is_the_integer_it_denotes_1() {
    assert_integral_real_digits_answer_like_the_integer("evalir_batch");
}

// ---------------------------------------------------------------------------
// PrintBudgetTests
// ---------------------------------------------------------------------------

/// `(x + 1)^power` expanded, the C# `Algebra.Expand` the two budget rows are stated over.
fn big_expand(power: i64) -> Expr {
    expand(&Expr::pow(
        Expr::add(Expr::sym("x"), Expr::num(1)),
        Expr::num(power),
    ))
}

/// `PrintBudgetTests.cs:73`: `MaxDepth 2` on `(x + 1)^12` is a `depth-limit` truncation, and
/// the outcome names the depth cap that stopped it.
#[test]
fn printbudgettests_depth_budget_reports_depth_limit() {
    let big = big_expand(12);
    assert!(
        expr_depth(&big) > 2,
        "the corpus value must be deeper than the depth cap"
    );

    let outcome = print(
        &big,
        PrintMode::Pretty,
        Some(PrintBudget {
            max_nodes: None,
            max_depth: Some(2),
        }),
    );

    assert!(
        outcome.truncated,
        "a depth-capped rendering was not abbreviated"
    );
    let truncation = outcome
        .truncation
        .expect("a truncated outcome carries its reason");
    assert_eq!(truncation.reason, "depth-limit");
    assert_eq!(truncation.budget, 2);
    assert_eq!(truncation.node_count, big.node_count());
    assert!(outcome.text.ends_with(" …"), "{}", outcome.text);
}

/// `PrintBudgetTests.cs:61`: `x + 1` has 3 nodes but a 5-character rendering. At `MaxNodes 1`
/// the node count is over the budget, yet nothing was dropped, so nothing is reported and the
/// text is the whole rendering.
#[test]
fn printbudgettests_node_budget_on_a_compact_rendering_drops_nothing_and_says_so() {
    let small = Expr::add(Expr::sym("x"), Expr::num(1));
    assert_eq!(small.node_count(), 3, "x + 1 is three nodes");

    let outcome = print(
        &small,
        PrintMode::Pretty,
        Some(PrintBudget {
            max_nodes: Some(1),
            max_depth: None,
        }),
    );

    assert!(
        !outcome.truncated,
        "nothing was dropped, so the outcome must not say truncated"
    );
    assert!(
        outcome.truncation.is_none(),
        "a whole rendering has no truncation"
    );
    assert_eq!(outcome.text, small.to_string());
}

// ---------------------------------------------------------------------------
// PrintBudgetTokenBoundaryTests
// ---------------------------------------------------------------------------

fn is_token_char(c: char) -> bool {
    c.is_alphanumeric() || c == '.'
}

/// The C# `AssertEndsAtATokenBoundary`: the retained body is a strict prefix whose first dropped
/// character is not a token character.
fn assert_ends_at_a_token_boundary(full: &str, abridged: &str, what: &str) {
    assert!(
        abridged.ends_with(" …"),
        "{what}: '{abridged}' has no ellipsis"
    );
    let body = &abridged[..abridged.len() - " …".len()];
    assert!(
        body.len() < full.len(),
        "{what}: '{abridged}' abbreviates nothing ('{full}' is {} chars)",
        full.len()
    );
    assert!(
        full.starts_with(body),
        "{what}: '{abridged}' is not a prefix of '{full}'"
    );
    let retains = body.chars().last().map(is_token_char).unwrap_or(false);
    let drops = full[body.len()..]
        .chars()
        .next()
        .map(is_token_char)
        .unwrap_or(false);
    assert!(
        !(retains && drops),
        "{what}: the cut falls inside a token, so '{abridged}' ends inside it"
    );
}

/// `PrintBudgetTokenBoundaryTests.cs:69`: a 54-character identifier at `MaxNodes 1` is kept
/// WHOLE (the allowance falls inside its first token); the abbreviation is the identifier plus
/// the ellipsis, never 48 of its characters.
#[test]
fn printbudgettokenboundarytests_an_identifier_longer_than_the_allowance_is_kept_whole() {
    let name = "a".repeat(54);
    let expression = Expr::add(Expr::sym(&name), Expr::num(1));
    let full = expression.to_string();
    assert_eq!(full, format!("{name} + 1"));

    let outcome = print(
        &expression,
        PrintMode::Pretty,
        Some(PrintBudget {
            max_nodes: Some(1),
            max_depth: None,
        }),
    );

    let body = outcome
        .text
        .strip_suffix(" …")
        .unwrap_or_else(|| panic!("'{}' does not end with the ellipsis", outcome.text));
    assert_eq!(
        body,
        name,
        "the pretty body is {} characters of a {}-character identifier",
        body.len(),
        name.len()
    );
    assert!(outcome.truncated, "the abbreviation must say so");
    let truncation = outcome.truncation.expect("the reason");
    assert_eq!(truncation.reason, "node-budget");
    assert_eq!(truncation.budget, 1);
    assert_ends_at_a_token_boundary(&full, &outcome.text, "the 54-character symbol");
}

/// `PrintBudgetTokenBoundaryTests.cs:105`: a 44-character name plus " + 1" is exactly the
/// 48-character allowance, so `MaxNodes 1` returns it whole and reports nothing.
#[test]
fn printbudgettokenboundarytests_a_rendering_within_the_allowance_is_untouched() {
    let name = "a".repeat(44);
    let expression = Expr::add(Expr::sym(&name), Expr::num(1));
    let full = expression.to_string();
    assert_eq!(full.len(), 48, "44 name characters plus ' + 1'");

    let outcome = print(
        &expression,
        PrintMode::Pretty,
        Some(PrintBudget {
            max_nodes: Some(1),
            max_depth: None,
        }),
    );

    assert!(
        !outcome.truncated,
        "a rendering the allowance holds whole must not be abbreviated"
    );
    assert!(outcome.truncation.is_none());
    assert_eq!(outcome.text, full);
}

// ---------------------------------------------------------------------------
// TruncatingEvaluationStaysInexactTests
// ---------------------------------------------------------------------------

/// The C# `TruncatedArguments()` family: the three routes that actually lose digits (a truncated
/// constant, a truncated function value, and a truncation moved into the unit interval). Every
/// member is built by the operations, and the tests assert the premise before using it.
fn truncated_arguments() -> Vec<(&'static str, Real)> {
    let pi30 = Real::pi(30);
    let e30 = Real::e(30);
    let ten = Real::from_i64(10);
    let six = Real::from_i64(6);
    let four = Real::from_i64(4);
    let one = Real::from_i64(1);
    vec![
        ("pi to 30 places", pi30.clone()),
        ("e to 30 places", e30.clone()),
        ("sqrt(2)", Real::from_i64(2).sqrt(40)),
        ("sqrt(3)", Real::from_i64(3).sqrt(40)),
        ("sin(1)", one.sin(40)),
        ("cos(1)", one.cos(40)),
        ("exp(1)", one.exp(40)),
        ("pi/10 to 30 places", pi30.div(&ten, 40).expect("pi / 10")),
        ("pi/6 to 30 places", pi30.div(&six, 40).expect("pi / 6")),
        ("pi/4 to 30 places", pi30.div(&four, 40).expect("pi / 4")),
        ("e/4 to 30 places", e30.div(&four, 40).expect("e / 4")),
    ]
}

/// `TruncatingEvaluationStaysInexactTests.cs:273`: every elementary function of every truncated
/// argument crosses through a real literal and still answers an INEXACT Real. The control that
/// keeps this from being satisfied by a blanket "inexact" is the next test.
#[test]
fn truncatingevaluationstaysinexacttests_function_of_a_truncated_argument_is_inexact() {
    let functions = [
        "sin", "cos", "tan", "exp", "atan", "sinh", "cosh", "tanh", "sqrt",
    ];
    let arguments = truncated_arguments();
    let mut pairs = 0usize;

    for (label, value) in &arguments {
        assert!(
            !value.is_exact(),
            "{label}: the family must consist of truncations"
        );
        for name in functions {
            let literal = RealLiteral::from_real_exact(value);
            let node = Expr::Fun(name.to_string(), vec![Expr::real(literal.digits())]);
            let result = evaluate_to_num(&node, 40, &[], &builtin_numeric_function)
                .unwrap_or_else(|error| panic!("{name}({label}): {}", error.message()));
            let Num::Real(real) = result else {
                panic!("{name}({label}) must answer a Real, got {result:?}");
            };
            assert!(
                !real.is_exact(),
                "{name}({label}) = {real} came from a truncated argument and must not claim exactness"
            );
            pairs += 1;
        }
    }

    assert_eq!(pairs, arguments.len() * functions.len());
}

/// `TruncatingEvaluationStaysInexactTests.cs:239`: a genuinely exact value's literal round trip
/// stays exact. This is the control direction of the property above.
#[test]
fn truncatingevaluationstaysinexacttests_literal_round_trip_keeps_genuinely_exact_values_exact() {
    let exact_values = [
        ("0", "0"),
        ("1", "1"),
        ("1/2", "0.5"),
        ("-3/2", "-1.5"),
        ("1/8", "0.125"),
        ("1000000", "1000000"),
        ("1/(10^19)", "0.0000000000000000001"),
    ];

    for (label, text) in exact_values {
        let value = Real::from_str(text).unwrap_or_else(|| panic!("{label}: not a decimal"));
        assert!(
            value.is_exact(),
            "{label}: the control family must consist of exact values"
        );

        let literal = RealLiteral::from_real_exact(&value);
        assert!(
            literal.is_exact(),
            "{label}: the literal lost the exact route"
        );
        let back = literal.to_real();
        assert_eq!(value, back, "{label}: the literal changed the digits");
        assert!(
            back.is_exact(),
            "{label} is exact and must stay exact through the literal"
        );
    }
}

// ---------------------------------------------------------------------------
// ValueDepthGuardTests
// ---------------------------------------------------------------------------

fn nested_arrays(wraps: usize) -> Value {
    let mut value = Value::Natural(Nat::from_u64(1));
    for _ in 0..wraps {
        value = Value::Array(
            NdArray::from_vec(vec![1], vec![value]).expect("a rank-1 single-element array"),
        );
    }
    value
}

fn nested_records(levels: usize) -> Value {
    let mut value = Value::Natural(Nat::from_u64(1));
    for _ in 0..levels {
        value = Value::Record("Level".to_string(), vec![("inner".to_string(), value)]);
    }
    value
}

fn expected_display(wraps: usize) -> String {
    format!("{}1{}", "[".repeat(wraps), "]".repeat(wraps))
}

fn value_depth_error(error: &crate::LangError) -> bool {
    error.code == "DepthExceeded" && error.category == "BudgetExceeded"
}

fn with_big_stack<T: Send + 'static>(probe: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(probe)
        .expect("probe thread")
        .join()
        .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
}

/// `ValueDepthGuardTests.cs:80`: the STRUCTURED projection refuses the value one level past the
/// budget with the typed boundary message, at exactly 1025.
#[test]
fn valuedepthguardtests_value_one_past_the_budget_is_refused_by_the_structured_projection() {
    with_big_stack(|| {
        let engine = Engine::new();
        let at_budget = nested_arrays(1023);
        assert_eq!(
            at_budget.try_display().expect("depth 1024 is the budget"),
            expected_display(1023)
        );
        let projected = crate::structured::try_project_value(&engine, &at_budget, None)
            .expect("depth 1024 is still projected");
        assert_eq!(projected.get("kind").and_then(Json::as_str), Some("Array"));

        let past = nested_arrays(1024);
        let error = crate::structured::try_project_value(&engine, &past, None)
            .expect_err("value depth 1025 is refused by the structured projection");
        assert!(
            value_depth_error(&error),
            "{} / {}",
            error.code,
            error.category
        );
        assert!(
            error
                .message
                .contains("value depth 1025 exceeds the maximum supported nesting depth of 1024"),
            "{}",
            error.message
        );
    });
}

/// `ValueDepthGuardTests.cs:154`: a record chain is refused the same way as an array chain —
/// depth 1024 renders, 1025 is refused by the display walk, and 20000 is refused by the
/// structured projection instead of exhausting the stack.
#[test]
fn valuedepthguardtests_a_deep_record_chain_is_refused_the_same_way() {
    with_big_stack(|| {
        let at_budget = nested_records(1023);
        assert!(
            !at_budget
                .try_display()
                .expect("depth 1024 is the budget")
                .is_empty()
        );

        let past = nested_records(1024);
        let error = past
            .try_display()
            .expect_err("value depth 1025 is refused by the display walk");
        assert!(
            value_depth_error(&error),
            "{} / {}",
            error.code,
            error.category
        );
        assert!(
            error
                .message
                .contains("value depth 1025 exceeds the maximum supported nesting depth of 1024"),
            "{}",
            error.message
        );

        let deep = nested_records(20000);
        let error = crate::structured::try_project_value(&Engine::new(), &deep, None)
            .expect_err("20000 record levels are refused, not walked");
        assert!(
            value_depth_error(&error),
            "{} / {}",
            error.code,
            error.category
        );
        assert!(
            error
                .message
                .contains("maximum supported nesting depth of 1024"),
            "{}",
            error.message
        );
    });
}

// ---------------------------------------------------------------------------
// DocsSyncTests
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_repo_file(relative: &str) -> String {
    let path = repo_root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// `DocsSyncTests.cs:24`: every C# snippet in the usage guide appears verbatim
/// (whitespace-normalized) in the compiled examples file.
#[test]
fn docssynctests_every_c_sharp_snippet_appears_in_usage_examples() {
    let readme = read_repo_file("Lovelace.Symbolics/README.md");
    let examples = read_repo_file("Lovelace.Symbolics.Tests/UsageExamples.cs");

    let checked = check_csharp_snippets_appear(&readme, &examples)
        .unwrap_or_else(|missing| panic!("{}", missing.join("\n")));
    assert!(checked > 0, "the usage guide has no C# snippets");
}

/// `DocsSyncTests.cs:38`: every lovelace block in the usage guide is followed by a result block
/// whose expected result is non-blank.
#[test]
fn docssynctests_every_lovelace_example_has_an_expected_result() {
    let readme = read_repo_file("Lovelace.Symbolics/README.md");
    let examples = parse_examples(&readme);

    assert!(
        !examples.is_empty(),
        "the usage guide has no lovelace examples"
    );
    for (index, example) in examples.iter().enumerate() {
        assert!(
            !example.expected.trim().is_empty(),
            "example #{index} has no expected result:\n{}",
            example.script
        );
    }
}
