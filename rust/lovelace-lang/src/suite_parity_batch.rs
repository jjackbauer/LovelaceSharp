//! Parity batch for `Lovelace.Suite.Tests` and `Lovelace.Symbolics.Tests` (census rows
//! whose behaviour this crate reproduces exactly).
//!
//! Every test is one census case. The C# assertion is the contract: the script is the C# test's
//! own input and the expected text is what the C# assertion demands (for the documentation suites, the
//! `result` block the C# test parses out of the guide; for descriptor examples, the reference
//! rendering the C# engine produced for the same snippet). A Rust test asserts that same concrete
//! text; a row whose Rust behaviour differs is not here, it is refused in the round report with the
//! C# file:line, the Rust behaviour and the difference.
//!
//! Helpers are shared per mode so a failure names the script and both texts.

use crate::Engine;
use lovelace_json::Json;

fn engine() -> Engine {
    Engine::new()
}

fn check_typed(script: &str, expected: &str) {
    let mut e = engine();
    match e.eval(script) {
        Ok(v) => assert_eq!(e.format_typed(&v), expected, "script: {script}"),
        Err(err) => panic!("script: {script} unexpectedly failed: {}", err.message),
    }
}

fn check_error(script: &str, expected: &str) {
    let mut e = engine();
    match e.eval(script) {
        Ok(v) => panic!(
            "script: {script} was expected to fail but produced {}",
            e.format_typed(&v)
        ),
        Err(err) => assert_eq!(err.message, expected, "script: {script}"),
    }
}

fn check_text(script: &str, expected: &str) {
    let mut e = engine();
    match e.eval(script) {
        Ok(crate::Value::Text(text)) => assert_eq!(text, expected, "script: {script}"),
        Ok(v) => panic!(
            "script: {script} produced {} instead of Text",
            e.format_typed(&v)
        ),
        Err(err) => panic!("script: {script} unexpectedly failed: {}", err.message),
    }
}

fn check_prints(script: &str, expected: &str) {
    let mut e = engine();
    match e.eval(script) {
        Ok(_) => {
            let out = e.take_output();
            assert_eq!(out.join("\n"), expected, "script: {script}");
        }
        Err(err) => panic!("script: {script} unexpectedly failed: {}", err.message),
    }
}

fn check_plot_title(script: &str, expected: &str) {
    let mut e = engine();
    match e.eval(script) {
        Ok(_) => {
            let plot = e
                .last_plot()
                .unwrap_or_else(|| panic!("script: {script} produced no plot"));
            assert_eq!(plot.title, expected, "script: {script}");
        }
        Err(err) => panic!("script: {script} unexpectedly failed: {}", err.message),
    }
}

fn structured_of(script: &str) -> (String, Json) {
    let mut e = engine();
    let v = match e.eval(script) {
        Ok(v) => v,
        Err(err) => panic!("script: {script} unexpectedly failed: {}", err.message),
    };
    let j = crate::structured::project_value(&e, &v, None);
    (e.format_typed(&v), j)
}

fn json_bool(j: &Json) -> Option<bool> {
    match j {
        Json::Bool(b) => Some(*b),
        _ => None,
    }
}

fn num_str(j: &Json) -> String {
    match j {
        Json::Num(s) => s.clone(),
        other => other.to_string(),
    }
}

fn field<'a>(j: &'a Json, key: &str) -> Option<&'a str> {
    j.get(key).and_then(|v| v.as_str())
}

fn gcd_u64(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

/// The C# property: IF the payload is exact, its numerator/denominator are the quotient itself.
fn check_quotient_payload(script: &str, num: u64, den: u64) {
    let (typed, j) = structured_of(script);
    assert_eq!(
        field(&j, "kind"),
        Some("Real"),
        "script: {script} ({typed})"
    );
    let exact = j.get("exact").and_then(json_bool).unwrap_or(false);
    if exact {
        let n: u64 = field(&j, "numerator")
            .unwrap_or_else(|| panic!("script: {script} is exact with no numerator"))
            .parse()
            .expect("numerator is a u64");
        let d: u64 = field(&j, "denominator")
            .unwrap_or_else(|| panic!("script: {script} is exact with no denominator"))
            .parse()
            .expect("denominator is a u64");
        let g = gcd_u64(n, d).max(1);
        assert_eq!((n / g, d / g), (num, den), "script: {script}");
    }
}

fn check_rational_payload(script: &str, num: &str, den: &str) {
    let (typed, j) = structured_of(script);
    assert_eq!(
        field(&j, "kind"),
        Some("Real"),
        "script: {script} ({typed})"
    );
    assert_eq!(
        j.get("exact").and_then(json_bool),
        Some(true),
        "script: {script}"
    );
    assert_eq!(field(&j, "numerator"), Some(num), "script: {script}");
    assert_eq!(field(&j, "denominator"), Some(den), "script: {script}");
}

fn check_not_exact(script: &str) {
    let (typed, j) = structured_of(script);
    assert_eq!(
        field(&j, "kind"),
        Some("Real"),
        "script: {script} ({typed})"
    );
    assert_eq!(
        j.get("exact").and_then(json_bool),
        Some(false),
        "script: {script}"
    );
    assert!(
        j.get("numerator").is_none(),
        "script: {script} published a numerator"
    );
    assert!(
        j.get("denominator").is_none(),
        "script: {script} published a denominator"
    );
}

fn check_payload(script: &str, value: &str, num: &str, den: &str) {
    let (typed, j) = structured_of(script);
    assert_eq!(
        field(&j, "kind"),
        Some("Real"),
        "script: {script} ({typed})"
    );
    assert_eq!(field(&j, "value"), Some(value), "script: {script}");
    assert_eq!(
        j.get("exact").and_then(json_bool),
        Some(true),
        "script: {script}"
    );
    assert_eq!(field(&j, "numerator"), Some(num), "script: {script}");
    assert_eq!(field(&j, "denominator"), Some(den), "script: {script}");
}

fn element_strings(j: &Json, script: &str) -> Vec<String> {
    let elements = j
        .get("elements")
        .and_then(|v| v.as_arr())
        .unwrap_or_else(|| panic!("script: {script} published no elements"));
    elements
        .iter()
        .map(|el| {
            let kind = field(el, "kind").unwrap_or("?");
            let value = field(el, "value")
                .or_else(|| field(el, "display"))
                .unwrap_or("");
            format!("{kind}={value}")
        })
        .collect()
}

fn check_shape(script: &str, expected: &str) {
    let (typed, j) = structured_of(script);
    let shape = j
        .get("shape")
        .and_then(|v| v.as_arr())
        .unwrap_or_else(|| panic!("script: {script} published no shape ({typed})"));
    let actual: Vec<String> = shape.iter().map(num_str).collect();
    assert_eq!(actual.join(","), expected, "script: {script} ({typed})");
}

fn check_elements(script: &str, expected: &str) {
    let (typed, j) = structured_of(script);
    let actual = element_strings(&j, script);
    let want: Vec<String> = if expected.is_empty() {
        Vec::new()
    } else {
        expected.split(';').map(|s| s.to_string()).collect()
    };
    assert_eq!(actual, want, "script: {script} ({typed})");
}

fn check_count(script: &str, expected: usize) {
    let (typed, j) = structured_of(script);
    let actual = element_strings(&j, script).len();
    assert_eq!(actual, expected, "script: {script} ({typed})");
}

fn check_element(script: &str, index: usize, expected: &str) {
    let (typed, j) = structured_of(script);
    let actual = element_strings(&j, script);
    let got = actual
        .get(index)
        .unwrap_or_else(|| panic!("script: {script} has no element {index} ({typed})"));
    assert_eq!(got, expected, "script: {script}");
}

fn check_exact(script: &str, expected: bool) {
    let (typed, j) = structured_of(script);
    assert_eq!(
        j.get("exact").and_then(json_bool),
        Some(expected),
        "script: {script} ({typed})"
    );
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_001_numbers.
#[test]
fn lang_doc_001_numbers() {
    check_typed("42", "42 (Natural)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_002_numbers.
#[test]
fn lang_doc_002_numbers() {
    check_typed("3.14", "3.14 (Real)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_011_3_operators_and_preceden.
#[test]
fn lang_doc_011_3_operators_and_preceden() {
    check_typed("2 * 1..5", "[2, 4, 6, 8, 10] (Vector)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_101_symbolic_mathematics.
#[test]
fn lang_doc_101_symbolic_mathematics() {
    check_typed(
        "x = symbol(\"x\")\nfactor(x^4 - 5*x^2 + 4)",
        "(x - 2)*(x - 1)*(x + 1)*(x + 2) (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_102_symbolic_mathematics.
#[test]
fn lang_doc_102_symbolic_mathematics() {
    check_typed(
        "x = symbol(\"x\")\nsolve(x^4 - 5*x^2 + 4 == 0, x)",
        "SolveResult(status: Solved, variable: x, domain: complex, complete: True, completeness: Complete, solutions: [Solution(value: -2, conditions: [], multiplicity: 1, exactness: Exact), Solution(value: -1, conditions: [], multiplicity: 1, exactness: Exact), Solution(value: 1, conditions: [], multiplicity: 1, exactness: Exact), Solution(value: 2, conditions: [], multiplicity: 1, exactness: Exact)], families: [], common_conditions: [], represented_count: 4, unrepresented_count: 0, unrepresented_reason: , diagnostics: []) (SolveResult)",
    );
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_103_symbolic_mathematics.
#[test]
fn lang_doc_103_symbolic_mathematics() {
    check_typed(
        "x = symbol(\"x\")\nseries(sin(x)/x, x, 0, 6)",
        "1 - 1/6*x^2 + 1/120*x^4 + O(x^6) (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_104_symbolic_mathematics.
#[test]
fn lang_doc_104_symbolic_mathematics() {
    check_typed(
        "x = symbol(\"x\")\nlimit(sin(x)/x, x, 0)",
        "LimitResult(status: Value, exists: True, value: 1, left: , left_conditions: [], right: , right_conditions: [], conditions: [], exactness: Exact, diagnostics: []) (LimitResult)",
    );
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_105_symbolic_mathematics.
#[test]
fn lang_doc_105_symbolic_mathematics() {
    check_typed(
        "x = symbol(\"x\")\nintegrate(2*x*cos(x^2), x)",
        "sin(x^2) (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_106_symbolic_mathematics.
#[test]
fn lang_doc_106_symbolic_mathematics() {
    check_typed(
        "x = symbol(\"x\")\ny = symbol(\"y\")\ndet([[x, 1], [y, x]])",
        "x^2 - y (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_107_symbolic_mathematics.
#[test]
fn lang_doc_107_symbolic_mathematics() {
    check_typed(
        "x = symbol(\"x\")\nassume_positive(x)\nsimplify(sqrt(x^2))",
        "x (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_108_symbolic_mathematics.
#[test]
fn lang_doc_108_symbolic_mathematics() {
    check_typed("evalf(1/3, 25)", "0.3333333333333333333333333 (Real)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_109_symbolic_mathematics.
#[test]
fn lang_doc_109_symbolic_mathematics() {
    check_typed(
        "x = symbol(\"x\")\nsolve_full(x^2 - 4 == 0, x).status",
        "Solved",
    );
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_110_symbolic_mathematics.
#[test]
fn lang_doc_110_symbolic_mathematics() {
    check_typed(
        "x = symbol(\"x\")\ny = symbol(\"y\")\njacobian([x*y, x+y], [x, y])",
        "[[y, x], [1, 1]] (Array)",
    );
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_012_addition_and_multiplicat.
#[test]
fn lang_doc_012_addition_and_multiplicat() {
    check_typed("1 + 2", "3 (Natural)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_111_symbolic_mathematics.
#[test]
fn lang_doc_111_symbolic_mathematics() {
    check_typed(
        "x = symbol(\"x\")\ntype(solve_full(x^2 - 4 == 0, x))",
        "SolveResult",
    );
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_013_addition_and_multiplicat.
#[test]
fn lang_doc_013_addition_and_multiplicat() {
    check_typed("2 * 3", "6 (Natural)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_014_subtraction_widens_on_un.
#[test]
fn lang_doc_014_subtraction_widens_on_un() {
    check_typed("7 - 3", "4 (Natural)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_015_subtraction_widens_on_un.
#[test]
fn lang_doc_015_subtraction_widens_on_un() {
    check_typed("3 - 5", "-2 (Integer)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_016_division_is_exact_never_.
#[test]
fn lang_doc_016_division_is_exact_never_() {
    check_typed("1 / 3", "0.(3) (Real)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_017_division_is_exact_never_.
#[test]
fn lang_doc_017_division_is_exact_never_() {
    check_typed("1 / 4", "0.25 (Real)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_018_division_is_exact_never_.
#[test]
fn lang_doc_018_division_is_exact_never_() {
    check_typed("1 / 2", "0.5 (Real)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_019_division_is_exact_never_.
#[test]
fn lang_doc_019_division_is_exact_never_() {
    check_error("1 / 0", "Cannot divide by zero.");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_020_widening_to_real.
#[test]
fn lang_doc_020_widening_to_real() {
    check_typed("2 + 0.5", "2.5 (Real)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_003_numbers.
#[test]
fn lang_doc_003_numbers() {
    check_typed("-5", "-5 (Integer)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_021_widening_to_real.
#[test]
fn lang_doc_021_widening_to_real() {
    check_typed("-7 / 2", "-3.5 (Real)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_022_power.
#[test]
fn lang_doc_022_power() {
    check_typed("2 ^ 10", "1024 (Natural)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_023_factorial.
#[test]
fn lang_doc_023_factorial() {
    check_typed("5!", "120 (Natural)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_024_modulo.
#[test]
fn lang_doc_024_modulo() {
    check_typed("10 % 3", "1 (Natural)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_025_modulo.
#[test]
fn lang_doc_025_modulo() {
    check_typed("-7 % 3", "-1 (Integer)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_026_5_comparison_and_boolean.
#[test]
fn lang_doc_026_5_comparison_and_boolean() {
    check_typed("2 > 1", "True (Boolean)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_027_5_comparison_and_boolean.
#[test]
fn lang_doc_027_5_comparison_and_boolean() {
    check_typed("1 == 1", "True (Boolean)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_028_5_comparison_and_boolean.
#[test]
fn lang_doc_028_5_comparison_and_boolean() {
    check_typed("3 <= 3", "True (Boolean)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_029_5_comparison_and_boolean.
#[test]
fn lang_doc_029_5_comparison_and_boolean() {
    check_typed("2 != 2", "False (Boolean)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_030_6_variables_and_assignme.
#[test]
fn lang_doc_030_6_variables_and_assignme() {
    check_typed("x = 42", "42 (Natural)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_004_numbers.
#[test]
fn lang_doc_004_numbers() {
    check_typed("0.(3)", "0.(3) (Real)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_031_6_variables_and_assignme.
#[test]
fn lang_doc_031_6_variables_and_assignme() {
    check_typed("x = 42; x + 1", "43 (Natural)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_008_ranges.
#[test]
fn lang_doc_008_ranges() {
    check_typed("1..5", "[1, 2, 3, 4, 5] (Vector)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_033_ranges.
#[test]
fn lang_doc_033_ranges() {
    check_typed("1..2..7", "[1, 3, 5, 7] (Vector)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_034_ranges.
#[test]
fn lang_doc_034_ranges() {
    check_typed("-2..2", "[-2, -1, 0, 1, 2] (Vector)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_035_ranges.
#[test]
fn lang_doc_035_ranges() {
    check_typed("5..-1..1", "[5, 4, 3, 2, 1] (Vector)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_036_ranges.
#[test]
fn lang_doc_036_ranges() {
    check_error("1..0..5", "Range step must not be zero.");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_007_list_literals.
#[test]
fn lang_doc_007_list_literals() {
    check_typed("[1, 2, 3]", "[1, 2, 3] (Vector)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_038_indexing.
#[test]
fn lang_doc_038_indexing() {
    check_typed("[10, 20, 30][0]", "10 (Natural)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_039_indexing.
#[test]
fn lang_doc_039_indexing() {
    check_error(
        "[10, 20, 30][3]",
        "Index 3 is out of range for vector of length 3.",
    );
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_040_indexing.
#[test]
fn lang_doc_040_indexing() {
    check_error(
        "[1, 2, 3][-1]",
        "Index -1 is out of range for vector of length 3.",
    );
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_005_strings.
#[test]
fn lang_doc_005_strings() {
    check_typed("\"hello\"", "hello");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_041_length.
#[test]
fn lang_doc_041_length() {
    check_typed("len([5, 6, 7, 8])", "4 (Natural)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_042_element_wise_arithmetic_.
#[test]
fn lang_doc_042_element_wise_arithmetic_() {
    check_typed("[1, 2] + [10, 20]", "[11, 22] (Vector)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_043_element_wise_arithmetic_.
#[test]
fn lang_doc_043_element_wise_arithmetic_() {
    check_typed("[1, 2, 3] * 10", "[10, 20, 30] (Vector)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_044_element_wise_arithmetic_.
#[test]
fn lang_doc_044_element_wise_arithmetic_() {
    check_typed("[1, 2] + [[1, 2], [3, 4]]", "[[2, 4], [4, 6]] (Array)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_045_element_wise_arithmetic_.
#[test]
fn lang_doc_045_element_wise_arithmetic_() {
    check_error(
        "[1, 2] + [1, 2, 3]",
        "Operands could not be broadcast together with shapes [2] and [3].",
    );
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_046_slicing.
#[test]
fn lang_doc_046_slicing() {
    check_typed("[0, 1, 2, 3, 4][1:4]", "[1, 2, 3] (Vector)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_047_slicing.
#[test]
fn lang_doc_047_slicing() {
    check_typed("[[1, 2, 3], [4, 5, 6]][:, 1]", "[2, 5] (Vector)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_048_8_strings_and_interpolat.
#[test]
fn lang_doc_048_8_strings_and_interpolat() {
    check_typed("\"hello\"", "hello");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_006_interpolated_strings.
#[test]
fn lang_doc_006_interpolated_strings() {
    check_typed("$\"x = {3 + 4}\"", "x = 7");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_050_8_strings_and_interpolat.
#[test]
fn lang_doc_050_8_strings_and_interpolat() {
    check_typed("$\"v = {1..3}\"", "v = [1, 2, 3]");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_049_8_strings_and_interpolat.
#[test]
fn lang_doc_049_8_strings_and_interpolat() {
    check_typed("$\"x = {3 + 4}\"", "x = 7");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_051_8_strings_and_interpolat.
#[test]
fn lang_doc_051_8_strings_and_interpolat() {
    check_typed("$\"a {{ b }}\"", "a { b }");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_052_blocks.
#[test]
fn lang_doc_052_blocks() {
    check_typed("{ 1; 2; 3 }", "3 (Natural)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_053_conditionals.
#[test]
fn lang_doc_053_conditionals() {
    check_typed("if (2 > 1) { 10 } else { 20 }", "10 (Natural)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_054_conditionals.
#[test]
fn lang_doc_054_conditionals() {
    check_error(
        "if (1) { 2 }",
        "if condition must be Boolean, but got 'Natural'.",
    );
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_055_loops.
#[test]
fn lang_doc_055_loops() {
    check_typed(
        "sum = 0; for i in 1..4 { sum = sum + i }; sum",
        "10 (Natural)",
    );
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_056_loops.
#[test]
fn lang_doc_056_loops() {
    check_typed("n = 0; while (n < 3) { n = n + 1 }; n", "3 (Natural)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_057_loops.
#[test]
fn lang_doc_057_loops() {
    check_error("break", "'break' is only valid inside a loop.");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_058_10_functions.
#[test]
fn lang_doc_058_10_functions() {
    check_typed("func add(a, b) { a + b }; add(2, 3)", "5 (Natural)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_059_10_functions.
#[test]
fn lang_doc_059_10_functions() {
    check_typed("func square(x) = x ^ 2; square(5)", "25 (Natural)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_060_10_functions.
#[test]
fn lang_doc_060_10_functions() {
    check_typed("func inc(x) { return x + 1 }; inc(5)", "6 (Natural)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_037_list_literals.
#[test]
fn lang_doc_037_list_literals() {
    check_typed("[1, 2, 3]", "[1, 2, 3] (Vector)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_061_10_functions.
#[test]
fn lang_doc_061_10_functions() {
    check_typed(
        "func fact(n) { if (n == 0) { 1 } else { n * fact(n - 1) } }; fact(5)",
        "120 (Natural)",
    );
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_062_abs_x.
#[test]
fn lang_doc_062_abs_x() {
    check_typed("abs(-5)", "5 (Integer)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_063_inv_x.
#[test]
fn lang_doc_063_inv_x() {
    check_typed("inv(4)", "0.25 (Real)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_064_divrem_a_b.
#[test]
fn lang_doc_064_divrem_a_b() {
    check_typed(
        "divrem(17, 5)",
        "DivRemResult(quotient: 3, remainder: 2) (DivRemResult)",
    );
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_065_is_even_x_is_odd_x.
#[test]
fn lang_doc_065_is_even_x_is_odd_x() {
    check_typed("is_even(4)", "True (Boolean)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_066_is_even_x_is_odd_x.
#[test]
fn lang_doc_066_is_even_x_is_odd_x() {
    check_typed("is_odd(4)", "False (Boolean)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_067_sign_x.
#[test]
fn lang_doc_067_sign_x() {
    check_typed("sign(-7)", "-1 (Integer)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_068_sqrt_x.
#[test]
fn lang_doc_068_sqrt_x() {
    check_typed("sqrt(4)", "2 (Real)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_069_pi_pi_digits.
#[test]
fn lang_doc_069_pi_pi_digits() {
    check_typed("pi(5)", "3.14159 (Real)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_070_setprecision_n.
#[test]
fn lang_doc_070_setprecision_n() {
    check_typed(
        "setprecision(50)\npi(50)",
        "3.14159265358979323846264338327950288419716939937510 (Real)",
    );
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_032_ranges.
#[test]
fn lang_doc_032_ranges() {
    check_typed("1..5", "[1, 2, 3, 4, 5] (Vector)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_071_len_v.
#[test]
fn lang_doc_071_len_v() {
    check_typed("len([5, 6, 7, 8])", "4 (Natural)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_072_print_values.
#[test]
fn lang_doc_072_print_values() {
    check_prints("print(\"hi\", 1..3)", "hi [1, 2, 3]");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_073_dsp_functions_loaded_by_.
#[test]
fn lang_doc_073_dsp_functions_loaded_by_() {
    check_typed("fft([1, 0, 0, 0])", "[1, 1, 1, 1] (Vector)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_074_dsp_functions_loaded_by_.
#[test]
fn lang_doc_074_dsp_functions_loaded_by_() {
    check_typed("conv([1, 1], [1, 1])", "[1, 2, 1] (Vector)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_075_dsp_functions_loaded_by_.
#[test]
fn lang_doc_075_dsp_functions_loaded_by_() {
    check_typed(
        "z = fft([0, 1, 0, 0])[1]; [re(z), im(z), abs(z)]",
        "[0, -1, 1] (Vector)",
    );
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_076_12_plotting.
#[test]
fn lang_doc_076_12_plotting() {
    check_plot_title("plot(1..10, 1 / (1..10 ^ 2), \"1/x^2\")", "1/x^2");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_077_13_errors_and_diagnostic.
#[test]
fn lang_doc_077_13_errors_and_diagnostic() {
    check_error("nope + 1", "Undefined variable 'nope'.");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_078_13_errors_and_diagnostic.
#[test]
fn lang_doc_078_13_errors_and_diagnostic() {
    check_error(
        "2 + \"x\"",
        "Cannot widen from Natural to Text: only numeric kinds (Natural, Integer, Real) support widening.",
    );
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_079_literals.
#[test]
fn lang_doc_079_literals() {
    check_typed("[[1, 2], [3, 4]]", "[[1, 2], [3, 4]] (Array)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_080_literals.
#[test]
fn lang_doc_080_literals() {
    check_typed(
        "[[[1, 2], [3, 4]], [[5, 6], [7, 8]]]",
        "[[[1, 2], [3, 4]], [[5, 6], [7, 8]]] (Array)",
    );
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_009_3_operators_and_preceden.
#[test]
fn lang_doc_009_3_operators_and_preceden() {
    check_typed("2 ^ 3 ^ 2", "512 (Natural)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_081_literals.
#[test]
fn lang_doc_081_literals() {
    check_error(
        "[[1, 2], [3]]",
        "Ragged nested list literal: every row must have the same shape.",
    );
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_082_indexing.
#[test]
fn lang_doc_082_indexing() {
    check_typed("[[1, 2], [3, 4]][1, 0]", "3 (Natural)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_083_indexing.
#[test]
fn lang_doc_083_indexing() {
    check_typed(
        "[[[1, 2], [3, 4]], [[5, 6], [7, 8]]][0]",
        "[[1, 2], [3, 4]] (Array)",
    );
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_084_construction_and_shape.
#[test]
fn lang_doc_084_construction_and_shape() {
    check_typed("zeros(2, 3)", "[[0, 0, 0], [0, 0, 0]] (Array)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_085_construction_and_shape.
#[test]
fn lang_doc_085_construction_and_shape() {
    check_typed("reshape(1..6, 2, 3)", "[[1, 2, 3], [4, 5, 6]] (Array)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_086_construction_and_shape.
#[test]
fn lang_doc_086_construction_and_shape() {
    check_typed("shape(zeros(2, 3))", "[2, 3] (Vector)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_087_reductions.
#[test]
fn lang_doc_087_reductions() {
    check_typed("sum([[1, 2], [3, 4]])", "10 (Natural)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_088_reductions.
#[test]
fn lang_doc_088_reductions() {
    check_typed("sum([[1, 2], [3, 4]], 0)", "[4, 6] (Vector)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_089_reductions.
#[test]
fn lang_doc_089_reductions() {
    check_typed("mean([1, 2])", "1.5 (Real)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_090_reductions.
#[test]
fn lang_doc_090_reductions() {
    check_typed("norm([3, 4])", "5 (Real)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_010_3_operators_and_preceden.
#[test]
fn lang_doc_010_3_operators_and_preceden() {
    check_typed(
        "1..10 ^ 2",
        "[1, 4, 9, 16, 25, 36, 49, 64, 81, 100] (Vector)",
    );
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_091_linear_algebra.
#[test]
fn lang_doc_091_linear_algebra() {
    check_typed(
        "matmul([[1, 2], [3, 4]], [[5, 6], [7, 8]])",
        "[[19, 22], [43, 50]] (Array)",
    );
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_092_linear_algebra.
#[test]
fn lang_doc_092_linear_algebra() {
    check_typed("dot([1, 2], [3, 4])", "11 (Natural)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_093_linear_algebra.
#[test]
fn lang_doc_093_linear_algebra() {
    check_typed("cross([1, 0, 0], [0, 1, 0])", "[0, 0, 1] (Vector)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_094_linear_algebra.
#[test]
fn lang_doc_094_linear_algebra() {
    check_typed("det([[1, 2], [3, 4]])", "-2 (Integer)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_095_linear_algebra.
#[test]
fn lang_doc_095_linear_algebra() {
    check_typed("inv([[1, 2], [3, 4]])", "[[-2, 1], [1.5, -0.5]] (Array)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_096_linear_algebra.
#[test]
fn lang_doc_096_linear_algebra() {
    check_typed("trace([[1, 2], [3, 4]])", "5 (Natural)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_097_shape_manipulation_and_c.
#[test]
fn lang_doc_097_shape_manipulation_and_c() {
    check_typed("transpose([[1, 2], [3, 4]])", "[[1, 3], [2, 4]] (Array)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_098_shape_manipulation_and_c.
#[test]
fn lang_doc_098_shape_manipulation_and_c() {
    check_typed("flatten([[1, 2], [3, 4]])", "[1, 2, 3, 4] (Vector)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_099_shape_manipulation_and_c.
#[test]
fn lang_doc_099_shape_manipulation_and_c() {
    check_typed("append([1, 2], [3, 4])", "[1, 2, 3, 4] (Vector)");
}

/// C# Lovelace.Suite.Tests/LanguageDocumentationTests.cs:23 — lang_doc_100_symbolic_mathematics.
#[test]
fn lang_doc_100_symbolic_mathematics() {
    check_typed(
        "x = symbol(\"x\")\nf = x^3 + 2*x^2 + 5*x + 7\ndiff(f, x)",
        "3*x^2 + 4*x + 5 (Symbolic)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_001_1_quick_start.
#[test]
fn sym_doc_001_1_quick_start() {
    check_typed(
        "x = symbol(\"x\")\nf = x^3 + 2*x^2 + 5*x + 7\ndiff(f, x)",
        "3*x^2 + 4*x + 5 (Symbolic)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_002_2_symbols_expressions_an.
#[test]
fn sym_doc_002_2_symbols_expressions_an() {
    check_typed("x = symbol(\"x\")\nx + 0", "x (Symbolic)");
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_011_2_symbols_expressions_an.
#[test]
fn sym_doc_011_2_symbols_expressions_an() {
    check_typed(
        "x = symbol(\"x\")\nx^2 - 2*x + 1",
        "x^2 - 2*x + 1 (Symbolic)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_101_18b_structured_results_t.
#[test]
fn sym_doc_101_18b_structured_results_t() {
    check_typed(
        "x = symbol(\"x\")\ntype(solve_full(x^2 - 4 == 0, x))",
        "SolveResult",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_102_18b_structured_results_t.
#[test]
fn sym_doc_102_18b_structured_results_t() {
    check_typed(
        "x = symbol(\"x\")\ninspect(x^2 + 1).free_symbols",
        "[x] (Vector)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_103_18b_structured_results_t.
#[test]
fn sym_doc_103_18b_structured_results_t() {
    check_typed(
        "x = symbol(\"x\")\nsolve(x^2 + 1 == 0, x, real)",
        "SolveResult(status: NoSolutions, variable: x, domain: real, complete: True, completeness: Complete, solutions: [], families: [], common_conditions: [], represented_count: 0, unrepresented_count: 0, unrepresented_reason: , diagnostics: [Diagnostic(code: solve.no-solutions, category: NoSolution, message: no real solutions, recoverable: True, location: , details: [])]) (SolveResult)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_012_2_symbols_expressions_an.
#[test]
fn sym_doc_012_2_symbols_expressions_an() {
    check_typed("x = symbol(\"x\")\n2^x", "2^x (Symbolic)");
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_013_2_symbols_expressions_an.
#[test]
fn sym_doc_013_2_symbols_expressions_an() {
    check_typed("1/2 + 1/3", "0.8(3) (Real)");
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_014_2_symbols_expressions_an.
#[test]
fn sym_doc_014_2_symbols_expressions_an() {
    check_typed("x = symbol(\"x\")\nx > 0", "x > 0 (Symbolic)");
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_015_3_assumptions.
#[test]
fn sym_doc_015_3_assumptions() {
    check_typed("x = symbol(\"x\")\nassume(x > 5)", "x > 5 (Symbolic)");
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_016_3_assumptions.
#[test]
fn sym_doc_016_3_assumptions() {
    check_typed(
        "x = symbol(\"x\")\nassume(x > 5)\nassumptions()",
        "AssumptionSet(display: x > 5, assumptions: [x > 5]) (AssumptionSet)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_017_3_assumptions.
#[test]
fn sym_doc_017_3_assumptions() {
    check_typed(
        "x = symbol(\"x\")\nassume_positive(x)\nsimplify(sqrt(x^2))",
        "x (Symbolic)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_018_3_assumptions.
#[test]
fn sym_doc_018_3_assumptions() {
    check_typed(
        "y = symbol(\"y\")\nsimplify(sqrt(y^2))",
        "sqrt(y^2) (Symbolic)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_019_3_assumptions.
#[test]
fn sym_doc_019_3_assumptions() {
    check_typed("simplify(sqrt(4))", "2 (Symbolic)");
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_020_4_simplification.
#[test]
fn sym_doc_020_4_simplification() {
    check_typed(
        "x = symbol(\"x\")\nsimplify(sin(x)^2 + cos(x)^2)",
        "1 (Symbolic)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_003_2_symbols_expressions_an.
#[test]
fn sym_doc_003_2_symbols_expressions_an() {
    check_typed("x = symbol(\"x\")\nx*0", "0 (Symbolic)");
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_021_4_simplification.
#[test]
fn sym_doc_021_4_simplification() {
    check_typed(
        "x = symbol(\"x\")\nsimplify(exp(log(x)))",
        "exp(log(x)) (Symbolic)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_022_4_simplification.
#[test]
fn sym_doc_022_4_simplification() {
    check_typed(
        "x = symbol(\"x\")\nassume(x > 0)\nsimplify(exp(log(x)))",
        "x (Symbolic)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_023_5_algebra_expand_collect.
#[test]
fn sym_doc_023_5_algebra_expand_collect() {
    check_typed(
        "x = symbol(\"x\")\nexpand((x+1)^3)",
        "x^3 + 3*x^2 + 3*x + 1 (Symbolic)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_024_5_algebra_expand_collect.
#[test]
fn sym_doc_024_5_algebra_expand_collect() {
    check_typed(
        "x = symbol(\"x\")\ny = symbol(\"y\")\ncollect(x*y + x + 2, x)",
        "2 + x*(y + 1) (Symbolic)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_025_5_algebra_expand_collect.
#[test]
fn sym_doc_025_5_algebra_expand_collect() {
    check_typed(
        "x = symbol(\"x\")\nfactor(x^4 - 5*x^2 + 4)",
        "(x - 2)*(x - 1)*(x + 1)*(x + 2) (Symbolic)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_026_5_algebra_expand_collect.
#[test]
fn sym_doc_026_5_algebra_expand_collect() {
    check_typed(
        "x = symbol(\"x\")\ncancel((x^2 - 1)/(x - 1))",
        "x + 1 (Symbolic)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_027_5_algebra_expand_collect.
#[test]
fn sym_doc_027_5_algebra_expand_collect() {
    check_typed(
        "x = symbol(\"x\")\napart(1/(x^2 - 1), x)",
        "-1/2/(x + 1) + 1/2/(x - 1) (Symbolic)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_028_5_algebra_expand_collect.
#[test]
fn sym_doc_028_5_algebra_expand_collect() {
    check_typed("x = symbol(\"x\")\nfactor(sin(x))", "sin(x) (Symbolic)");
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_029_6_differentiation.
#[test]
fn sym_doc_029_6_differentiation() {
    check_typed(
        "x = symbol(\"x\")\ndiff(x*exp(x), x)",
        "exp(x) + x*exp(x) (Symbolic)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_030_6_differentiation.
#[test]
fn sym_doc_030_6_differentiation() {
    check_typed(
        "x = symbol(\"x\")\ndiff(diff(x^4, x), x)",
        "12*x^2 (Symbolic)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_004_2_symbols_expressions_an.
#[test]
fn sym_doc_004_2_symbols_expressions_an() {
    check_typed("x = symbol(\"x\")\nx*2 + 3*x", "5*x (Symbolic)");
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_031_6_differentiation.
#[test]
fn sym_doc_031_6_differentiation() {
    check_typed(
        "x = symbol(\"x\")\ndiff(cos(x^2), x)",
        "-2*x*sin(x^2) (Symbolic)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_032_6_differentiation.
#[test]
fn sym_doc_032_6_differentiation() {
    check_error(
        "x = symbol(\"x\")\ndiff(x^2, 3)",
        "diff(): argument 2 must be a symbolic variable; got Natural.",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_033_7_vector_calculus_jacobi.
#[test]
fn sym_doc_033_7_vector_calculus_jacobi() {
    check_typed(
        "x = symbol(\"x\")\ny = symbol(\"y\")\njacobian([x*y, x+y], [x, y])",
        "[[y, x], [1, 1]] (Array)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_034_7_vector_calculus_jacobi.
#[test]
fn sym_doc_034_7_vector_calculus_jacobi() {
    check_typed(
        "x = symbol(\"x\")\ny = symbol(\"y\")\nhessian(x*y, [x, y])",
        "[[0, 1], [1, 0]] (Array)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_035_8_series.
#[test]
fn sym_doc_035_8_series() {
    check_typed(
        "x = symbol(\"x\")\nseries(sin(x)/x, x, 0, 8)",
        "1 - 1/6*x^2 + 1/120*x^4 - 1/5040*x^6 + O(x^8) (Symbolic)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_036_8_series.
#[test]
fn sym_doc_036_8_series() {
    check_typed(
        "x = symbol(\"x\")\nseries(exp(x), x, 0, 5)",
        "1 + x + 1/2*x^2 + 1/6*x^3 + 1/24*x^4 + O(x^5) (Symbolic)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_037_9_limits.
#[test]
fn sym_doc_037_9_limits() {
    check_typed(
        "x = symbol(\"x\")\nlimit(sin(x)/x, x, 0)",
        "LimitResult(status: Value, exists: True, value: 1, left: , left_conditions: [], right: , right_conditions: [], conditions: [], exactness: Exact, diagnostics: []) (LimitResult)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_038_9_limits.
#[test]
fn sym_doc_038_9_limits() {
    check_typed(
        "x = symbol(\"x\")\nlimit((1 - cos(x))/x^2, x, 0)",
        "LimitResult(status: Value, exists: True, value: 1/2, left: , left_conditions: [], right: , right_conditions: [], conditions: [], exactness: Exact, diagnostics: []) (LimitResult)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_039_9_limits.
#[test]
fn sym_doc_039_9_limits() {
    check_typed(
        "x = symbol(\"x\")\nlimit(1/x, x, 0)",
        "LimitResult(status: DoesNotExist, exists: False, value: , left: -inf, left_conditions: [x < 0], right: inf, right_conditions: [x > 0], conditions: [], exactness: Exact, diagnostics: []) (LimitResult)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_040_9_limits.
#[test]
fn sym_doc_040_9_limits() {
    check_typed(
        "x = symbol(\"x\")\nlimit_left(1/x, x, 0)",
        "LimitResult(status: MinusInfinity, exists: True, value: -inf, left: , left_conditions: [], right: , right_conditions: [], conditions: [], exactness: Exact, diagnostics: []) (LimitResult)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_005_2_symbols_expressions_an.
#[test]
fn sym_doc_005_2_symbols_expressions_an() {
    check_typed("x = symbol(\"x\")\nx^2*x^3", "x^5 (Symbolic)");
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_041_9_limits.
#[test]
fn sym_doc_041_9_limits() {
    check_typed(
        "x = symbol(\"x\")\nlimit_right(1/x, x, 0)",
        "LimitResult(status: PlusInfinity, exists: True, value: inf, left: , left_conditions: [], right: , right_conditions: [], conditions: [], exactness: Exact, diagnostics: []) (LimitResult)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_042_9_limits.
#[test]
fn sym_doc_042_9_limits() {
    check_typed(
        "x = symbol(\"x\")\nlimit(1/x^2, x, 0)",
        "LimitResult(status: PlusInfinity, exists: True, value: inf, left: , left_conditions: [], right: , right_conditions: [], conditions: [], exactness: Exact, diagnostics: []) (LimitResult)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_043_9_limits.
#[test]
fn sym_doc_043_9_limits() {
    check_typed(
        "x = symbol(\"x\")\nlimit(1/(x+1), x, inf)",
        "LimitResult(status: Value, exists: True, value: 0, left: , left_conditions: [], right: , right_conditions: [], conditions: [], exactness: Exact, diagnostics: []) (LimitResult)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_044_9_limits.
#[test]
fn sym_doc_044_9_limits() {
    check_typed(
        "x = symbol(\"x\")\nlimit(x^2 + 1, x, inf)",
        "LimitResult(status: PlusInfinity, exists: True, value: inf, left: , left_conditions: [], right: , right_conditions: [], conditions: [], exactness: Exact, diagnostics: []) (LimitResult)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_045_9_limits.
#[test]
fn sym_doc_045_9_limits() {
    check_typed(
        "x = symbol(\"x\")\nlimit((2*x^2 + 1)/(x^2 - 1), x, inf)",
        "LimitResult(status: Value, exists: True, value: 2, left: , left_conditions: [], right: , right_conditions: [], conditions: [], exactness: Exact, diagnostics: []) (LimitResult)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_046_9_limits.
#[test]
fn sym_doc_046_9_limits() {
    check_typed(
        "x = symbol(\"x\")\nlimit(exp(-(x^2)), x, inf)",
        "LimitResult(status: Unevaluated, exists: , value: , left: , left_conditions: [], right: , right_conditions: [], conditions: [], exactness: None, diagnostics: [Diagnostic(code: limit.unevaluated, category: UnsupportedOperation, message: coefficient does not evaluate at the point, recoverable: True, location: , details: [])]) (LimitResult)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_047_10_integration.
#[test]
fn sym_doc_047_10_integration() {
    check_typed("x = symbol(\"x\")\nintegrate(x^5, x)", "1/6*x^6 (Symbolic)");
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_048_10_integration.
#[test]
fn sym_doc_048_10_integration() {
    check_typed("x = symbol(\"x\")\nintegrate(1/x, x)", "log(x) (Symbolic)");
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_049_10_integration.
#[test]
fn sym_doc_049_10_integration() {
    check_typed(
        "x = symbol(\"x\")\nintegrate(1/(x^2 - 1), x)",
        "-1/2*log(x + 1) + 1/2*log(x - 1) (Symbolic)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_050_10_integration.
#[test]
fn sym_doc_050_10_integration() {
    check_typed(
        "x = symbol(\"x\")\nintegrate(2*x*cos(x^2), x)",
        "sin(x^2) (Symbolic)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_006_2_symbols_expressions_an.
#[test]
fn sym_doc_006_2_symbols_expressions_an() {
    check_typed(
        "x = symbol(\"x\")\ny = symbol(\"y\")\n(x+y)+x",
        "y + 2*x (Symbolic)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_051_10_integration.
#[test]
fn sym_doc_051_10_integration() {
    check_typed(
        "x = symbol(\"x\")\nintegrate(exp(-(x^2)), x)",
        "integrate(exp(-x^2), x) (Symbolic)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_052_10b_assumed_bounds_decid.
#[test]
fn sym_doc_052_10b_assumed_bounds_decid() {
    check_typed(
        "x = symbol(\"x\")\nassume(x >= 5)\nsimplify(x < 5)",
        "False (Boolean)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_053_10b_assumed_bounds_decid.
#[test]
fn sym_doc_053_10b_assumed_bounds_decid() {
    check_typed(
        "x = symbol(\"x\")\nassume(x >= 5)\nsimplify(x > 4)",
        "True (Boolean)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_054_10b_assumed_bounds_decid.
#[test]
fn sym_doc_054_10b_assumed_bounds_decid() {
    check_error(
        "x = symbol(\"x\")\nassume(x > 0)\nassume(x <= 0)",
        "Assumption x <= 0 contradicts the existing assumptions (its negation x > 0 is provable).",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_055_11_solving_equations.
#[test]
fn sym_doc_055_11_solving_equations() {
    check_typed(
        "x = symbol(\"x\")\nsolve(2*x - 4 == 0, x)",
        "SolveResult(status: Solved, variable: x, domain: complex, complete: True, completeness: Complete, solutions: [Solution(value: 2, conditions: [], multiplicity: 1, exactness: Exact)], families: [], common_conditions: [], represented_count: 1, unrepresented_count: 0, unrepresented_reason: , diagnostics: []) (SolveResult)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_056_11_solving_equations.
#[test]
fn sym_doc_056_11_solving_equations() {
    check_typed(
        "x = symbol(\"x\")\nsolve(x^2 - 4 == 0, x)",
        "SolveResult(status: Solved, variable: x, domain: complex, complete: True, completeness: Complete, solutions: [Solution(value: -2, conditions: [], multiplicity: 1, exactness: Exact), Solution(value: 2, conditions: [], multiplicity: 1, exactness: Exact)], families: [], common_conditions: [], represented_count: 2, unrepresented_count: 0, unrepresented_reason: , diagnostics: []) (SolveResult)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_057_11_solving_equations.
#[test]
fn sym_doc_057_11_solving_equations() {
    check_typed(
        "x = symbol(\"x\")\nsolve(x^4 - 5*x^2 + 4 == 0, x)",
        "SolveResult(status: Solved, variable: x, domain: complex, complete: True, completeness: Complete, solutions: [Solution(value: -2, conditions: [], multiplicity: 1, exactness: Exact), Solution(value: -1, conditions: [], multiplicity: 1, exactness: Exact), Solution(value: 1, conditions: [], multiplicity: 1, exactness: Exact), Solution(value: 2, conditions: [], multiplicity: 1, exactness: Exact)], families: [], common_conditions: [], represented_count: 4, unrepresented_count: 0, unrepresented_reason: , diagnostics: []) (SolveResult)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_058_11_solving_equations.
#[test]
fn sym_doc_058_11_solving_equations() {
    check_typed(
        "x = symbol(\"x\")\nsolve(x^3 - 1 == 0, x)",
        "SolveResult(status: Solved, variable: x, domain: complex, complete: True, completeness: Complete, solutions: [Solution(value: 1/2*(i*sqrt(3) - 1), conditions: [], multiplicity: 1, exactness: AlgebraicExact), Solution(value: 1/2*(-i*sqrt(3) - 1), conditions: [], multiplicity: 1, exactness: AlgebraicExact), Solution(value: 1, conditions: [], multiplicity: 1, exactness: Exact)], families: [], common_conditions: [], represented_count: 3, unrepresented_count: 0, unrepresented_reason: , diagnostics: []) (SolveResult)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_059_11_solving_equations.
#[test]
fn sym_doc_059_11_solving_equations() {
    check_typed(
        "x = symbol(\"x\")\nsolve(exp(x) == 5, x)",
        "SolveResult(status: Solved, variable: x, domain: complex, complete: True, completeness: Complete, solutions: [], families: [SolutionFamily(template: log(5) + 2*k*pi*i, parameter: k, period: 2*pi*i, parameter_domain: integer, conditions: [], exactness: ParametricExact)], common_conditions: [], represented_count: 0, unrepresented_count: 0, unrepresented_reason: , diagnostics: []) (SolveResult)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_060_11_solving_equations.
#[test]
fn sym_doc_060_11_solving_equations() {
    check_typed(
        "x = symbol(\"x\")\nsolve(sin(x) == 0, x)",
        "SolveResult(status: Solved, variable: x, domain: complex, complete: True, completeness: Complete, solutions: [], families: [SolutionFamily(template: k*pi, parameter: k, period: pi, parameter_domain: integer, conditions: [], exactness: ParametricExact)], common_conditions: [], represented_count: 0, unrepresented_count: 0, unrepresented_reason: , diagnostics: []) (SolveResult)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_007_2_symbols_expressions_an.
#[test]
fn sym_doc_007_2_symbols_expressions_an() {
    check_typed("x = symbol(\"x\")\nx - x", "0 (Symbolic)");
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_061_11_solving_equations.
#[test]
fn sym_doc_061_11_solving_equations() {
    check_typed(
        "x = symbol(\"x\")\ny = symbol(\"y\")\nsolve(x + y == 0, x)",
        "SolveResult(status: Solved, variable: x, domain: complex, complete: True, completeness: Complete, solutions: [Solution(value: -y, conditions: [1 != 0], multiplicity: 1, exactness: Exact)], families: [], common_conditions: [1 != 0], represented_count: 1, unrepresented_count: 0, unrepresented_reason: , diagnostics: []) (SolveResult)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_063_11_solving_equations.
#[test]
fn sym_doc_063_11_solving_equations() {
    check_typed(
        "x = symbol(\"x\")\nsolve(x - x + 1 == 0, x)",
        "SolveResult(status: NoSolutions, variable: x, domain: complex, complete: True, completeness: Complete, solutions: [], families: [], common_conditions: [], represented_count: 0, unrepresented_count: 0, unrepresented_reason: , diagnostics: [Diagnostic(code: solve.no-solutions, category: NoSolution, message: the equation reduces to a nonzero constant., recoverable: True, location: , details: [])]) (SolveResult)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_064_11_solving_equations.
#[test]
fn sym_doc_064_11_solving_equations() {
    check_typed(
        "x = symbol(\"x\")\nsolve(exp(x) + x == 0, x)",
        "SolveResult(status: Unevaluated, variable: x, domain: complex, complete: False, completeness: Unknown, solutions: [], families: [], common_conditions: [], represented_count: 0, unrepresented_count: 0, unrepresented_reason: , diagnostics: [Diagnostic(code: solve.unevaluated, category: UnsupportedOperation, message: No solver for this structure., recoverable: True, location: , details: [])]) (SolveResult)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_065_12_symbolic_matrices.
#[test]
fn sym_doc_065_12_symbolic_matrices() {
    check_typed(
        "x = symbol(\"x\")\ny = symbol(\"y\")\ndet([[x, 1], [y, x]])",
        "x^2 - y (Symbolic)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_066_12_symbolic_matrices.
#[test]
fn sym_doc_066_12_symbolic_matrices() {
    check_typed(
        "x = symbol(\"x\")\ny = symbol(\"y\")\ninv([[x, 1], [0, y]])",
        "[[y/(x*y), -1/(x*y)], [0/(x*y), x/(x*y)]] (Array)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_067_12_symbolic_matrices.
#[test]
fn sym_doc_067_12_symbolic_matrices() {
    check_typed(
        "x = symbol(\"x\")\ny = symbol(\"y\")\nmatmul([[x, 1], [0, x]], [[y], [1]])",
        "[[1 + x*y], [x]] (Array)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_068_12_symbolic_matrices.
#[test]
fn sym_doc_068_12_symbolic_matrices() {
    check_typed(
        "x = symbol(\"x\")\ny = symbol(\"y\")\ntrace([[x, y], [y, x]])",
        "2*x (Symbolic)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_069_12_symbolic_matrices.
#[test]
fn sym_doc_069_12_symbolic_matrices() {
    check_typed(
        "x = symbol(\"x\")\ny = symbol(\"y\")\ndot([x, y], [1, 2])",
        "x + 2*y (Symbolic)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_070_13_optimization_and_math.
#[test]
fn sym_doc_070_13_optimization_and_math() {
    check_typed(
        "x = symbol(\"x\")\noptimize(x^5 + 2*x^4 + 3*x^3 + x^2, [x])",
        "x^2*(1 + x*(3 + x*(x + 2))) (Symbolic)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_008_2_symbols_expressions_an.
#[test]
fn sym_doc_008_2_symbols_expressions_an() {
    check_typed("x = symbol(\"x\")\nx/x", "x/x (Symbolic)");
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_073_14_relations_conditions_.
#[test]
fn sym_doc_073_14_relations_conditions_() {
    check_typed(
        "x = symbol(\"x\")\ncond = and(x > 0, x < 1)",
        "x < 1 and x > 0 (Symbolic)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_075_15_symbolic_matrices_ran.
#[test]
fn sym_doc_075_15_symbolic_matrices_ran() {
    check_typed(
        "x = symbol(\"x\")\ny = symbol(\"y\")\nmatrix_rank([[x, 1], [y, x]])",
        "2 (Integer)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_077_16_compilation_and_batch.
#[test]
fn sym_doc_077_16_compilation_and_batch() {
    check_typed(
        "x = symbol(\"x\")\nk = compile(x^2 + 1, [x])\nevalir_batch(k, [1, 2, 3], 40)",
        "[2, 5, 10] (Vector)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_078_17_substitution_and_eval.
#[test]
fn sym_doc_078_17_substitution_and_eval() {
    check_typed("x = symbol(\"x\")\nsubs(x^2 + 1, x, 3)", "10 (Symbolic)");
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_079_17_substitution_and_eval.
#[test]
fn sym_doc_079_17_substitution_and_eval() {
    check_typed("evalf(1/3, 25)", "0.3333333333333333333333333 (Real)");
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_080_17_substitution_and_eval.
#[test]
fn sym_doc_080_17_substitution_and_eval() {
    check_typed("evalf(sqrt(2), 5)", "1.41421 (Real)");
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_009_2_symbols_expressions_an.
#[test]
fn sym_doc_009_2_symbols_expressions_an() {
    check_typed("x = symbol(\"x\")\nsimplify(x/x)", "x/x (Symbolic)");
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_081_17_substitution_and_eval.
#[test]
fn sym_doc_081_17_substitution_and_eval() {
    check_typed(
        "setprecision(20)\nevalf(sqrt(2), 20)",
        "1.4142135623730950488 (Real)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_082_17_substitution_and_eval.
#[test]
fn sym_doc_082_17_substitution_and_eval() {
    check_error(
        "x = symbol(\"x\")\nevalf(x, 20)",
        "No value for symbol 'x'.",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_083_18b_structured_results_t.
#[test]
fn sym_doc_083_18b_structured_results_t() {
    check_typed(
        "x = symbol(\"x\")\nsolve_full(x^2 - 4 == 0, x).status",
        "Solved",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_084_18b_structured_results_t.
#[test]
fn sym_doc_084_18b_structured_results_t() {
    check_typed(
        "x = symbol(\"x\")\nsolve_full((x^2 - 1)/(x - 1) == 0, x).solutions",
        "[Solution(value: -1, conditions: [x - 1 != 0], multiplicity: 1, exactness: Exact)] (Vector)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_085_18b_structured_results_t.
#[test]
fn sym_doc_085_18b_structured_results_t() {
    check_typed(
        "x = symbol(\"x\")\nsolve_full((x^2 - 1)/(x - 1) == 0, x).common_conditions",
        "[x - 1 != 0] (Vector)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_086_18b_structured_results_t.
#[test]
fn sym_doc_086_18b_structured_results_t() {
    check_typed(
        "x = symbol(\"x\")\nsolve_full(x^4 - x^2 - 1 == 0, x).status",
        "Partial",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_087_18b_structured_results_t.
#[test]
fn sym_doc_087_18b_structured_results_t() {
    check_typed(
        "x = symbol(\"x\")\nsolve_full(x^4 - x^2 - 1 == 0, x).complete",
        "False (Boolean)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_088_18b_structured_results_t.
#[test]
fn sym_doc_088_18b_structured_results_t() {
    check_typed(
        "x = symbol(\"x\")\nsolve_full(x^4 - x^2 - 1 == 0, x).domain",
        "complex (Domain)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_089_18b_structured_results_t.
#[test]
fn sym_doc_089_18b_structured_results_t() {
    check_typed(
        "x = symbol(\"x\")\nsolve_full(x^4 - x^2 - 1 == 0, x).unrepresented_count",
        "2 (Integer)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_090_18b_structured_results_t.
#[test]
fn sym_doc_090_18b_structured_results_t() {
    check_typed(
        "x = symbol(\"x\")\nsimplify_full(x/x).expression",
        "1 (Symbolic)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_010_2_symbols_expressions_an.
#[test]
fn sym_doc_010_2_symbols_expressions_an() {
    check_typed(
        "x = symbol(\"x\")\nassume(x != 0)\nsimplify(x/x)",
        "1 (Symbolic)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_091_18b_structured_results_t.
#[test]
fn sym_doc_091_18b_structured_results_t() {
    check_typed(
        "x = symbol(\"x\")\nsimplify_full(x/x).conditions",
        "[x != 0] (Vector)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_092_18b_structured_results_t.
#[test]
fn sym_doc_092_18b_structured_results_t() {
    check_typed(
        "x = symbol(\"x\")\nsimplify_full(x/x).steps[0].rule_id",
        "rat.cancel-x-over-x",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_093_18b_structured_results_t.
#[test]
fn sym_doc_093_18b_structured_results_t() {
    check_typed(
        "x = symbol(\"x\")\nlimit_full(1/x, x, 0).exists",
        "False (Boolean)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_094_18b_structured_results_t.
#[test]
fn sym_doc_094_18b_structured_results_t() {
    check_typed(
        "x = symbol(\"x\")\nlimit_full(1/x, x, 0).left",
        "-inf (Symbolic)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_095_18b_structured_results_t.
#[test]
fn sym_doc_095_18b_structured_results_t() {
    check_typed(
        "x = symbol(\"x\")\nintegrate_full(x^2, x).status",
        "SolvedExact",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_096_18b_structured_results_t.
#[test]
fn sym_doc_096_18b_structured_results_t() {
    check_typed(
        "x = symbol(\"x\")\nintegrate_full(x^2, x).verified",
        "True (Boolean)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_097_18b_structured_results_t.
#[test]
fn sym_doc_097_18b_structured_results_t() {
    check_typed(
        "x = symbol(\"x\")\ncompile_full(x^2 + 1, [x]).mathir_version",
        "2 (Integer)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_098_18b_structured_results_t.
#[test]
fn sym_doc_098_18b_structured_results_t() {
    check_typed(
        "x = symbol(\"x\")\ncompile_full(x^2 + 1, [x]).parameters",
        "[ParameterInfo(name: x, domain: complex)] (Vector)",
    );
}

/// C# Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:23 — sym_doc_099_18b_structured_results_t.
#[test]
fn sym_doc_099_18b_structured_results_t() {
    check_typed(
        "x = symbol(\"x\")\ncompile_full(x^2 + 1, [x]).result_domain",
        "complex (Domain)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_000_abs.
#[test]
fn descriptor_example_000_abs() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); abs(-3)",
        "3 (Integer)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_101_prod.
#[test]
fn descriptor_example_101_prod() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); prod([1, 2, 3])",
        "6 (Natural)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_102_rank.
#[test]
fn descriptor_example_102_rank() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); rank([[1, 2], [3, 4]])",
        "2 (Natural)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_103_rational.
#[test]
fn descriptor_example_103_rational() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); symbol(\"q\", rational)",
        "q (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_104_re.
#[test]
fn descriptor_example_104_re() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); re(fft([0, 1, 0, 0])[1])",
        "0 (Real)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_105_real.
#[test]
fn descriptor_example_105_real() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); solve(x^2 + 1 == 0, x, real)",
        "SolveResult(status: NoSolutions, variable: x, domain: real, complete: True, completeness: Complete, solutions: [], families: [], common_conditions: [], represented_count: 0, unrepresented_count: 0, unrepresented_reason: , diagnostics: [Diagnostic(code: solve.no-solutions, category: NoSolution, message: no real solutions, recoverable: True, location: , details: [])]) (SolveResult)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_106_reshape.
#[test]
fn descriptor_example_106_reshape() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); reshape([1, 2, 3, 4], 2, 2)",
        "[[1, 2], [3, 4]] (Array)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_109_series.
#[test]
fn descriptor_example_109_series() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); series(sin(x)/x, x, 0, 8)",
        "1 - 1/6*x^2 + 1/120*x^4 - 1/5040*x^6 + O(x^8) (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_111_shape.
#[test]
fn descriptor_example_111_shape() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); shape([[1, 2], [3, 4]])",
        "[2, 2] (Vector)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_112_sign.
#[test]
fn descriptor_example_112_sign() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); sign(-7)",
        "-1 (Integer)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_113_simplify.
#[test]
fn descriptor_example_113_simplify() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); simplify(sin(x)^2 + cos(x)^2)",
        "1 (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_114_simplify.
#[test]
fn descriptor_example_114_simplify() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); simplify(x/x)",
        "x/x (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_115_simplify_full.
#[test]
fn descriptor_example_115_simplify_full() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); simplify_full(x/x)",
        "TransformResult(status: Satisfied, original: x/x, expression: 1, changed: True, conditions: [x != 0], steps: [RewriteStep(rule_id: rat.cancel-x-over-x, classification: Conditional, before: x/x, after: 1, required_conditions: [x != 0])], budget_exceeded: False, budget_kind: , diagnostics: []) (TransformResult)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_117_sin.
#[test]
fn descriptor_example_117_sin() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); sin(x)",
        "sin(x) (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_118_sinh.
#[test]
fn descriptor_example_118_sinh() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); sinh(x)",
        "sinh(x) (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_119_solve.
#[test]
fn descriptor_example_119_solve() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); solve(x^2 - 4 == 0, x)",
        "SolveResult(status: Solved, variable: x, domain: complex, complete: True, completeness: Complete, solutions: [Solution(value: -2, conditions: [], multiplicity: 1, exactness: Exact), Solution(value: 2, conditions: [], multiplicity: 1, exactness: Exact)], families: [], common_conditions: [], represented_count: 2, unrepresented_count: 0, unrepresented_reason: , diagnostics: []) (SolveResult)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_120_solve.
#[test]
fn descriptor_example_120_solve() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); solve(x^2 + 1 == 0, x, real)",
        "SolveResult(status: NoSolutions, variable: x, domain: real, complete: True, completeness: Complete, solutions: [], families: [], common_conditions: [], represented_count: 0, unrepresented_count: 0, unrepresented_reason: , diagnostics: [Diagnostic(code: solve.no-solutions, category: NoSolution, message: no real solutions, recoverable: True, location: , details: [])]) (SolveResult)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_121_solve_full.
#[test]
fn descriptor_example_121_solve_full() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); solve_full(x^2 - 4 == 0, x)",
        "SolveResult(status: Solved, variable: x, domain: complex, complete: True, completeness: Complete, solutions: [Solution(value: -2, conditions: [], multiplicity: 1, exactness: Exact), Solution(value: 2, conditions: [], multiplicity: 1, exactness: Exact)], families: [], common_conditions: [], represented_count: 2, unrepresented_count: 0, unrepresented_reason: , diagnostics: []) (SolveResult)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_122_solve_full.
#[test]
fn descriptor_example_122_solve_full() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); solve_full(x^4 - x^2 - 1 == 0, x)",
        "SolveResult(status: Partial, variable: x, domain: complex, complete: False, completeness: Partial, solutions: [Solution(value: rootof(x^4 - x^2 - 1, 0), conditions: [], multiplicity: 1, exactness: AlgebraicExact), Solution(value: rootof(x^4 - x^2 - 1, 1), conditions: [], multiplicity: 1, exactness: AlgebraicExact)], families: [], common_conditions: [], represented_count: 2, unrepresented_count: 2, unrepresented_reason: complex algebraic roots not supported (RootOf is real-only in v1)., diagnostics: [Diagnostic(code: solve.unrepresented-roots, category: UnsupportedOperation, message: complex algebraic roots not supported (RootOf is real-only in v1)., recoverable: True, location: , details: [])]) (SolveResult)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_123_solve_system.
#[test]
fn descriptor_example_123_solve_system() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); solve_system([x^2 + y^2 - 1 == 0, x*y == 0], [x, y])",
        "SystemSolveResult(status: Solved, domain: complex, complete: True, completeness: Complete, solutions: [SystemSolution(bindings: [Binding(name: x, value: 0), Binding(name: y, value: -1)], conditions: [], exactness: Exact), SystemSolution(bindings: [Binding(name: x, value: -1), Binding(name: y, value: 0)], conditions: [], exactness: Exact), SystemSolution(bindings: [Binding(name: x, value: 1), Binding(name: y, value: 0)], conditions: [], exactness: Exact), SystemSolution(bindings: [Binding(name: x, value: 0), Binding(name: y, value: 1)], conditions: [], exactness: Exact)], diagnostics: []) (SystemSolveResult)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_124_solve_system_full.
#[test]
fn descriptor_example_124_solve_system_full() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); solve_system_full([x^2 + y^2 - 1 == 0, x*y == 0], [x, y])",
        "SystemSolveResult(status: Solved, domain: complex, complete: True, completeness: Complete, solutions: [SystemSolution(bindings: [Binding(name: x, value: 0), Binding(name: y, value: -1)], conditions: [], exactness: Exact), SystemSolution(bindings: [Binding(name: x, value: -1), Binding(name: y, value: 0)], conditions: [], exactness: Exact), SystemSolution(bindings: [Binding(name: x, value: 1), Binding(name: y, value: 0)], conditions: [], exactness: Exact), SystemSolution(bindings: [Binding(name: x, value: 0), Binding(name: y, value: 1)], conditions: [], exactness: Exact)], diagnostics: []) (SystemSolveResult)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_125_sqrt.
#[test]
fn descriptor_example_125_sqrt() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); sqrt(4)",
        "2 (Real)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_126_sqrt.
#[test]
fn descriptor_example_126_sqrt() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); sqrt(x)",
        "sqrt(x) (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_129_subs.
#[test]
fn descriptor_example_129_subs() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); subs(x^2 + 1, x, 3)",
        "10 (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_130_sum.
#[test]
fn descriptor_example_130_sum() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); sum([1, 2, 3])",
        "6 (Natural)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_131_symbol.
#[test]
fn descriptor_example_131_symbol() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); symbol(\"x\")",
        "x (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_132_symbol.
#[test]
fn descriptor_example_132_symbol() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); symbol(\"x\", real)",
        "x (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_133_tan.
#[test]
fn descriptor_example_133_tan() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); tan(x)",
        "tan(x) (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_134_tanh.
#[test]
fn descriptor_example_134_tanh() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); tanh(x)",
        "tanh(x) (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_135_trace.
#[test]
fn descriptor_example_135_trace() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); trace([[1, 2], [3, 4]])",
        "5 (Natural)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_136_transpose.
#[test]
fn descriptor_example_136_transpose() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); transpose([[1, 2], [3, 4]])",
        "[[1, 3], [2, 4]] (Array)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_137_type.
#[test]
fn descriptor_example_137_type() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); type(42)",
        "Natural",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_138_type.
#[test]
fn descriptor_example_138_type() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); type(solve_full(x^2 - 4 == 0, x))",
        "SolveResult",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_139_zeros.
#[test]
fn descriptor_example_139_zeros() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); zeros(3)",
        "[0, 0, 0] (Vector)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_140_zeros.
#[test]
fn descriptor_example_140_zeros() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); zeros(2, 2)",
        "[[0, 0], [0, 0]] (Array)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_017_assumptions.
#[test]
fn descriptor_example_017_assumptions() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); assumptions()",
        "AssumptionSet(display: , assumptions: []) (AssumptionSet)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_018_atan.
#[test]
fn descriptor_example_018_atan() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); atan(x)",
        "atan(x) (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_002_acos.
#[test]
fn descriptor_example_002_acos() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); acos(x)",
        "acos(x) (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_020_cancel.
#[test]
fn descriptor_example_020_cancel() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); cancel((x^2 - 1)/(x - 1))",
        "x + 1 (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_023_capabilities.
#[test]
fn descriptor_example_023_capabilities() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); capabilities()",
        "CapabilitiesResult(solve_domains_accepted: [real, complex], solve_domains_refused: [integer, rational], unsupported_operations: [UnsupportedCapability(operation_class: pow.non-integer-exponent-of-a-positive-base, code: UnsupportedOperation, category: UnsupportedOperation, message: Non-integer exponents are not yet supported., trigger: 2^(1/2), scope: Refused for a POSITIVE base: 2^(1/2), 4^(1/2), 9^(1/2), 0.5^(1/2) and 2^(-1/2) all fail. The message is the kernel's own and is wider than the refusal: an exact NEGATIVE base under the principal square root IS supported — (-4)^(1/2) = 2*i, (-9)^(1/2) = 3*i, (-2)^(1/2) = i*sqrt(2) — and a ZERO base is supported too (0^(1/2) = 0).), UnsupportedCapability(operation_class: pow.negative-base-unrepresentable-exponent, code: UnsupportedOperation, category: UnsupportedOperation, message: Non-integer exponents are not yet supported., trigger: (-8)^(1/3), scope: Refused for a NEGATIVE base with any non-integer exponent OTHER than the principal square root: (-8)^(1/3), (-4)^(1/4), (-4)^(2/3), (-4)^(3/2) and (-4)^(-1/2) all fail, while (-4)^(1/2) is supported (the entry above).), UnsupportedCapability(operation_class: solve.unsupported-domain, code: InvalidOperation, category: DomainError, message: solve(): currently supports domains real and complex; got integer., trigger: x = symbol(\"x\"); solve(x^2 - 2 == 0, x, integer), scope: ), UnsupportedCapability(operation_class: rootof.complex-algebraic, code: solve.unrepresented-roots, category: UnsupportedOperation, message: complex algebraic roots not supported (RootOf is real-only in v1)., trigger: x = symbol(\"x\"); solve_full(x^4 - x^2 - 1 == 0, x), scope: ), UnsupportedCapability(operation_class: limit.unevaluated-in-record-diagnostics, code: limit.unevaluated, category: UnsupportedOperation, message: coefficient does not evaluate at the point, trigger: x = symbol(\"x\"); limit_full(sin(x), x, inf), scope: ), UnsupportedCapability(operation_class: integration.unevaluated-in-record-diagnostics, code: integration.unevaluated, category: UnsupportedOperation, message: No closed form was found, so the integral is returned unevaluated; the reason for the refusal is carried in this diagnostic's details., trigger: x = symbol(\"x\"); integrate_full(exp(x^2), x), scope: ), UnsupportedCapability(operation_class: plot.symbolic-expression, code: InvalidOperation, category: DomainError, message: plot() argument 1 must be a vector, but got 'Symbolic'., trigger: x = symbol(\"x\"); plot(sin(x)), scope: ), UnsupportedCapability(operation_class: plot.symbolic-element, code: InvalidOperation, category: DomainError, message: Cannot convert value of kind 'Symbolic' to a number for plotting., trigger: x = symbol(\"x\"); plot([x, 1, 2]), scope: ), UnsupportedCapability(operation_class: solve.non-symbolic-variable, code: InvalidOperation, category: DomainError, message: solve(): argument 2 must be a symbolic variable; got Natural., trigger: x = symbol(\"x\"); solve(x^2 - 2 == 0, 1), scope: ), UnsupportedCapability(operation_class: solve.non-symbolic-expression, code: InvalidOperation, category: DomainError, message: solve(): argument 1 must be a symbolic expression; got Vector., trigger: x = symbol(\"x\"); solve([x == 1], x), scope: ), UnsupportedCapability(operation_class: diff.non-symbolic-variable, code: InvalidOperation, category: DomainError, message: diff(): argument 2 must be a symbolic variable; got Natural., trigger: x = symbol(\"x\"); diff(x^2, 1), scope: ), UnsupportedCapability(operation_class: integrate.non-symbolic-variable, code: InvalidOperation, category: DomainError, message: integrate(): argument 2 must be a symbolic variable; got Natural., trigger: x = symbol(\"x\"); integrate(x^2, 1), scope: ), UnsupportedCapability(operation_class: limit.non-symbolic-variable, code: InvalidOperation, category: DomainError, message: limit(): argument 2 must be a symbolic variable; got Natural., trigger: x = symbol(\"x\"); limit(sin(x), 1, 0), scope: ), UnsupportedCapability(operation_class: dsp.symbolic-element, code: InvalidOperation, category: DomainError, message: DSP builtins expect numeric/complex elements, but got 'SymbolExpr'., trigger: x = symbol(\"x\"); dft([x, 1, 2, 3]), scope: ), UnsupportedCapability(operation_class: linsolve.non-symbolic-matrix, code: InvalidOperation, category: DomainError, message: linsolve() requires a symbolic matrix A., trigger: A = [[1,2],[2,4]]; b = [[1],[3]]; linsolve(A, b), scope: ), UnsupportedCapability(operation_class: fft.non-power-of-two-length, code: InvalidArgument, category: TypeMismatch, message: FFT length must be a power of two, but got 3. (Parameter 'x'), trigger: x = symbol(\"x\"); fft([1,2,3]), scope: ), UnsupportedCapability(operation_class: solve.unevaluated-in-record-diagnostics, code: solve.unevaluated, category: UnsupportedOperation, message: 0 = 0: every value is a solution., trigger: x = symbol(\"x\"); solve_full(x == x, x), scope: ), UnsupportedCapability(operation_class: system-solve.unevaluated-in-record-diagnostics, code: system-solve.unevaluated, category: UnsupportedOperation, message: system-solve.elimination-incomplete: the Gröbner basis has no polynomial in y alone, so the values of y are not enumerated here., trigger: x = symbol(\"x\"); y = symbol(\"y\"); solve_system_full([x + y == 1], [x, y]), scope: ), UnsupportedCapability(operation_class: integration.no-closed-form-in-record-diagnostics, code: integration.no-closed-form, category: UnsupportedOperation, message: no integration tier produced a candidate closed form for integrate(exp(x^2), x), trigger: x = symbol(\"x\"); integrate_full(exp(x^2), x), scope: )], exactness: BestEffort) (CapabilitiesResult)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_026_compile_full.
#[test]
fn descriptor_example_026_compile_full() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); compile_full(x^2 + 1, [x])",
        "CompilationResult(ir: #!mathir 2\nparam x\nconst (rat 1 1)\nconst (rat 2 1)\nConstant 0 0\nParameter 0 0\nConstant 0 1\nPowInt 0 1,2\nAdd 0 0,3, parameters: [ParameterInfo(name: x, domain: complex)], result_domain: complex, result_type: Integer, precision_policy: caller_supplied, optimization_policy: none, target: mathir, mathir_version: 2, exact: True, diagnostics: []) (CompilationResult)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_027_complex.
#[test]
fn descriptor_example_027_complex() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); solve(x^2 + 1 == 0, x, complex)",
        "SolveResult(status: Solved, variable: x, domain: complex, complete: True, completeness: Complete, solutions: [Solution(value: i, conditions: [], multiplicity: 1, exactness: AlgebraicExact), Solution(value: -i, conditions: [], multiplicity: 1, exactness: AlgebraicExact)], families: [], common_conditions: [], represented_count: 2, unrepresented_count: 0, unrepresented_reason: , diagnostics: []) (SolveResult)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_028_concat.
#[test]
fn descriptor_example_028_concat() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); concat([1, 2], [3, 4])",
        "[1, 2, 3, 4] (Vector)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_030_conv.
#[test]
fn descriptor_example_030_conv() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); conv([1, 2], [1, 1])",
        "[1, 3, 2] (Vector)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_031_cos.
#[test]
fn descriptor_example_031_cos() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); cos(x)",
        "cos(x) (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_032_cosh.
#[test]
fn descriptor_example_032_cosh() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); cosh(x)",
        "cosh(x) (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_034_cross.
#[test]
fn descriptor_example_034_cross() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); cross([1, 0, 0], [0, 1, 0])",
        "[0, 0, 1] (Vector)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_036_det.
#[test]
fn descriptor_example_036_det() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); det([[1, 2], [3, 4]])",
        "-2 (Integer)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_037_det.
#[test]
fn descriptor_example_037_det() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); det([[x, 1], [y, x]])",
        "x^2 - y (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_038_dft.
#[test]
fn descriptor_example_038_dft() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); dft([0, 1, 0, 0])",
        "[1, -1i, -1, 1i] (Vector)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_039_diff.
#[test]
fn descriptor_example_039_diff() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); diff(x^3 + 2*x^2 + 5*x + 7, x)",
        "3*x^2 + 4*x + 5 (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_004_and.
#[test]
fn descriptor_example_004_and() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); assume(x > 0); and(x > 0, x < 1)",
        "x < 1 and x > 0 (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_040_divrem.
#[test]
fn descriptor_example_040_divrem() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); divrem(17, 5)",
        "DivRemResult(quotient: 3, remainder: 2) (DivRemResult)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_041_dot.
#[test]
fn descriptor_example_041_dot() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); dot([1, 2], [3, 4])",
        "11 (Natural)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_043_evalf.
#[test]
fn descriptor_example_043_evalf() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); evalf(sqrt(2), 30)",
        "1.414213562373095048801688724209 (Real)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_044_evalf.
#[test]
fn descriptor_example_044_evalf() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); evalf(1/3, 30)",
        "0.333333333333333333333333333333 (Real)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_045_evalir.
#[test]
fn descriptor_example_045_evalir() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); evalir(compile(x^2 + 1, [x]), [3], 40)",
        "10 (Integer)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_046_evalir_batch.
#[test]
fn descriptor_example_046_evalir_batch() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); k = compile(x^2 + 1, [x]); evalir_batch(k, [1, 2, 3], 40)",
        "[2, 5, 10] (Vector)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_047_exp.
#[test]
fn descriptor_example_047_exp() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); exp(x)",
        "exp(x) (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_048_expand.
#[test]
fn descriptor_example_048_expand() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); expand((x+1)^3)",
        "x^3 + 3*x^2 + 3*x + 1 (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_005_apart.
#[test]
fn descriptor_example_005_apart() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); apart(1/(x^2 - 1), x)",
        "-1/2/(x + 1) + 1/2/(x - 1) (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_050_eye.
#[test]
fn descriptor_example_050_eye() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); eye(3)",
        "[[1, 0, 0], [0, 1, 0], [0, 0, 1]] (Array)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_051_factor.
#[test]
fn descriptor_example_051_factor() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); factor(x^4 - 5*x^2 + 4)",
        "(x - 2)*(x - 1)*(x + 1)*(x + 2) (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_052_fft.
#[test]
fn descriptor_example_052_fft() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); fft([0, 1, 0, 0])",
        "[1, -1i, -1, 1i] (Vector)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_054_flatten.
#[test]
fn descriptor_example_054_flatten() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); flatten([[1, 2], [3, 4]])",
        "[1, 2, 3, 4] (Vector)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_055_hessian.
#[test]
fn descriptor_example_055_hessian() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); hessian(x*y, [x, y])",
        "[[0, 1], [1, 0]] (Array)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_056_im.
#[test]
fn descriptor_example_056_im() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); im(fft([0, 1, 0, 0])[1])",
        "-1 (Real)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_058_inf.
#[test]
fn descriptor_example_058_inf() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); inf",
        "inf (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_059_inspect.
#[test]
fn descriptor_example_059_inspect() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); inspect(x^2 + 1)",
        "Inspection(type: Symbolic, domain: complex, exact: True, free_symbols: [x], node_count: 5, canonical: (add (rat 1 1) (pow (sym x) (rat 2 1))), pretty: x^2 + 1, shape: , rank: , element_domain: complex, assumptions: [], members: ) (Inspection)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_006_append.
#[test]
fn descriptor_example_006_append() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); append([1, 2], [3])",
        "[1, 2, 3] (Vector)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_060_integer.
#[test]
fn descriptor_example_060_integer() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); symbol(\"n\", integer)",
        "n (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_061_integrate.
#[test]
fn descriptor_example_061_integrate() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); integrate(2*x*cos(x^2), x)",
        "sin(x^2) (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_062_integrate_full.
#[test]
fn descriptor_example_062_integrate_full() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); integrate_full(x^2, x)",
        "IntegrationResult(status: SolvedExact, expression: 1/3*x^3, conditions: [], verified: True, method: table, verification_method: differentiation+expansion, exactness: Exact, diagnostics: []) (IntegrationResult)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_063_inv.
#[test]
fn descriptor_example_063_inv() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); inv(2)",
        "0.5 (Real)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_064_inv.
#[test]
fn descriptor_example_064_inv() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); inv([[1, 2], [3, 4]])",
        "[[-2, 1], [1.5, -0.5]] (Array)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_066_is_even.
#[test]
fn descriptor_example_066_is_even() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); is_even(4)",
        "True (Boolean)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_067_is_odd.
#[test]
fn descriptor_example_067_is_odd() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); is_odd(4)",
        "False (Boolean)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_068_jacobian.
#[test]
fn descriptor_example_068_jacobian() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); jacobian([x*y, x+y], [x, y])",
        "[[y, x], [1, 1]] (Array)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_007_asin.
#[test]
fn descriptor_example_007_asin() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); asin(x)",
        "asin(x) (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_070_len.
#[test]
fn descriptor_example_070_len() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); len([1, 2, 3])",
        "3 (Natural)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_071_limit.
#[test]
fn descriptor_example_071_limit() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); limit(sin(x)/x, x, 0)",
        "LimitResult(status: Value, exists: True, value: 1, left: , left_conditions: [], right: , right_conditions: [], conditions: [], exactness: Exact, diagnostics: []) (LimitResult)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_072_limit.
#[test]
fn descriptor_example_072_limit() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); limit(1/x, x, 0)",
        "LimitResult(status: DoesNotExist, exists: False, value: , left: -inf, left_conditions: [x < 0], right: inf, right_conditions: [x > 0], conditions: [], exactness: Exact, diagnostics: []) (LimitResult)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_073_limit_full.
#[test]
fn descriptor_example_073_limit_full() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); limit_full(1/x, x, 0)",
        "LimitResult(status: DoesNotExist, exists: False, value: , left: -inf, left_conditions: [x < 0], right: inf, right_conditions: [x > 0], conditions: [], exactness: Exact, diagnostics: []) (LimitResult)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_074_limit_left.
#[test]
fn descriptor_example_074_limit_left() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); limit_left(1/x, x, 0)",
        "LimitResult(status: MinusInfinity, exists: True, value: -inf, left: , left_conditions: [], right: , right_conditions: [], conditions: [], exactness: Exact, diagnostics: []) (LimitResult)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_075_limit_right.
#[test]
fn descriptor_example_075_limit_right() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); limit_right(1/x, x, 0)",
        "LimitResult(status: PlusInfinity, exists: True, value: inf, left: , left_conditions: [], right: , right_conditions: [], conditions: [], exactness: Exact, diagnostics: []) (LimitResult)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_078_log.
#[test]
fn descriptor_example_078_log() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); log(x)",
        "log(x) (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_079_lower.
#[test]
fn descriptor_example_079_lower() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); lower(x^2 + 1, [x])",
        "#!mathir 2\nparam x\nconst (rat 1 1)\nconst (rat 2 1)\nConstant 0 0\nParameter 0 0\nConstant 0 1\nPowInt 0 1,2\nAdd 0 0,3",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_080_matmul.
#[test]
fn descriptor_example_080_matmul() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); matmul([[1, 2], [3, 4]], [[5], [6]])",
        "[[17], [39]] (Array)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_081_matrix_rank.
#[test]
fn descriptor_example_081_matrix_rank() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); matrix_rank([[x, 1], [1, x]])",
        "2 (Integer)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_082_max.
#[test]
fn descriptor_example_082_max() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); max([3, 1, 2])",
        "3 (Natural)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_084_min.
#[test]
fn descriptor_example_084_min() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); min([3, 1, 2])",
        "1 (Natural)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_088_norm.
#[test]
fn descriptor_example_088_norm() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); norm([3, 4])",
        "5 (Real)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_089_not.
#[test]
fn descriptor_example_089_not() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); not(x > 0)",
        "not x > 0 (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_009_assume.
#[test]
fn descriptor_example_009_assume() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); assume(x > 5)",
        "x > 5 (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_090_numel.
#[test]
fn descriptor_example_090_numel() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); numel([[1, 2], [3, 4]])",
        "4 (Natural)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_091_ones.
#[test]
fn descriptor_example_091_ones() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); ones(3)",
        "[1, 1, 1] (Vector)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_092_optimize.
#[test]
fn descriptor_example_092_optimize() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); optimize(x^5 + 2*x^4 + 3*x^3 + x^2 + x + 1, [x])",
        "1 + x*(1 + x*(1 + x*(3 + x*(x + 2)))) (Symbolic)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_093_optimize_full.
#[test]
fn descriptor_example_093_optimize_full() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); optimize_full(x^5 + 2*x^4 + 3*x^3 + x^2 + x + 1, [x])",
        "OptimizationResult(original: x^5 + 2*x^4 + 3*x^3 + x^2 + x + 1, optimized: 1 + x*(1 + x*(1 + x*(3 + x*(x + 2)))), estimated_cost_before: 19, estimated_cost_after: 19, shared_subtrees: 6, horner_rewrites: 1, transformations: [cse, horner], target: mathir, policy: cse+horner+precision-aware, diagnostics: []) (OptimizationResult)",
    );
}

/// C# Lovelace.Suite.Tests/DescriptorExampleTests.cs:104 — descriptor_example_096_pi.
#[test]
fn descriptor_example_096_pi() {
    check_typed(
        "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); pi(50)",
        "3.14159265358979323846264338327950288419716939937510 (Real)",
    );
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_000.
#[test]
fn structured_quotient_000() {
    check_quotient_payload("setprecision(5); 1/2", 1, 2);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_001.
#[test]
fn structured_quotient_001() {
    check_quotient_payload("setprecision(5); 1/4", 1, 4);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_010.
#[test]
fn structured_quotient_010() {
    check_quotient_payload("setprecision(18); 1/2", 1, 2);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_011.
#[test]
fn structured_quotient_011() {
    check_quotient_payload("setprecision(18); 1/4", 1, 4);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_012.
#[test]
fn structured_quotient_012() {
    check_quotient_payload("setprecision(18); 1/8", 1, 8);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_013.
#[test]
fn structured_quotient_013() {
    check_quotient_payload("setprecision(18); 1/1024", 1, 1024);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_014.
#[test]
fn structured_quotient_014() {
    check_quotient_payload("setprecision(18); 1/1009", 1, 1009);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_015.
#[test]
fn structured_quotient_015() {
    check_quotient_payload("setprecision(18); 1/17", 1, 17);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_016.
#[test]
fn structured_quotient_016() {
    check_quotient_payload("setprecision(18); 22/7", 22, 7);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_017.
#[test]
fn structured_quotient_017() {
    check_quotient_payload("setprecision(18); 1/3", 1, 3);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_018.
#[test]
fn structured_quotient_018() {
    check_quotient_payload("setprecision(18); 355/113", 355, 113);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_019.
#[test]
fn structured_quotient_019() {
    check_quotient_payload("setprecision(18); 3/2", 3, 2);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_002.
#[test]
fn structured_quotient_002() {
    check_quotient_payload("setprecision(5); 1/8", 1, 8);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_020.
#[test]
fn structured_quotient_020() {
    check_quotient_payload("setprecision(30); 1/2", 1, 2);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_021.
#[test]
fn structured_quotient_021() {
    check_quotient_payload("setprecision(30); 1/4", 1, 4);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_022.
#[test]
fn structured_quotient_022() {
    check_quotient_payload("setprecision(30); 1/8", 1, 8);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_023.
#[test]
fn structured_quotient_023() {
    check_quotient_payload("setprecision(30); 1/1024", 1, 1024);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_024.
#[test]
fn structured_quotient_024() {
    check_quotient_payload("setprecision(30); 1/1009", 1, 1009);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_025.
#[test]
fn structured_quotient_025() {
    check_quotient_payload("setprecision(30); 1/17", 1, 17);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_026.
#[test]
fn structured_quotient_026() {
    check_quotient_payload("setprecision(30); 22/7", 22, 7);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_027.
#[test]
fn structured_quotient_027() {
    check_quotient_payload("setprecision(30); 1/3", 1, 3);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_028.
#[test]
fn structured_quotient_028() {
    check_quotient_payload("setprecision(30); 355/113", 355, 113);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_029.
#[test]
fn structured_quotient_029() {
    check_quotient_payload("setprecision(30); 3/2", 3, 2);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_003.
#[test]
fn structured_quotient_003() {
    check_quotient_payload("setprecision(5); 1/1024", 1, 1024);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_030.
#[test]
fn structured_quotient_030() {
    check_quotient_payload("setprecision(37); 1/2", 1, 2);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_031.
#[test]
fn structured_quotient_031() {
    check_quotient_payload("setprecision(37); 1/4", 1, 4);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_032.
#[test]
fn structured_quotient_032() {
    check_quotient_payload("setprecision(37); 1/8", 1, 8);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_033.
#[test]
fn structured_quotient_033() {
    check_quotient_payload("setprecision(37); 1/1024", 1, 1024);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_034.
#[test]
fn structured_quotient_034() {
    check_quotient_payload("setprecision(37); 1/1009", 1, 1009);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_035.
#[test]
fn structured_quotient_035() {
    check_quotient_payload("setprecision(37); 1/17", 1, 17);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_036.
#[test]
fn structured_quotient_036() {
    check_quotient_payload("setprecision(37); 22/7", 22, 7);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_037.
#[test]
fn structured_quotient_037() {
    check_quotient_payload("setprecision(37); 1/3", 1, 3);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_038.
#[test]
fn structured_quotient_038() {
    check_quotient_payload("setprecision(37); 355/113", 355, 113);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_039.
#[test]
fn structured_quotient_039() {
    check_quotient_payload("setprecision(37); 3/2", 3, 2);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_004.
#[test]
fn structured_quotient_004() {
    check_quotient_payload("setprecision(5); 1/1009", 1, 1009);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_040.
#[test]
fn structured_quotient_040() {
    check_quotient_payload("setprecision(50); 1/2", 1, 2);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_041.
#[test]
fn structured_quotient_041() {
    check_quotient_payload("setprecision(50); 1/4", 1, 4);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_042.
#[test]
fn structured_quotient_042() {
    check_quotient_payload("setprecision(50); 1/8", 1, 8);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_043.
#[test]
fn structured_quotient_043() {
    check_quotient_payload("setprecision(50); 1/1024", 1, 1024);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_044.
#[test]
fn structured_quotient_044() {
    check_quotient_payload("setprecision(50); 1/1009", 1, 1009);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_045.
#[test]
fn structured_quotient_045() {
    check_quotient_payload("setprecision(50); 1/17", 1, 17);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_046.
#[test]
fn structured_quotient_046() {
    check_quotient_payload("setprecision(50); 22/7", 22, 7);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_047.
#[test]
fn structured_quotient_047() {
    check_quotient_payload("setprecision(50); 1/3", 1, 3);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_048.
#[test]
fn structured_quotient_048() {
    check_quotient_payload("setprecision(50); 355/113", 355, 113);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_049.
#[test]
fn structured_quotient_049() {
    check_quotient_payload("setprecision(50); 3/2", 3, 2);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_050.
#[test]
fn structured_quotient_050() {
    check_quotient_payload("setprecision(1000); 1/2", 1, 2);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_051.
#[test]
fn structured_quotient_051() {
    check_quotient_payload("setprecision(1000); 1/4", 1, 4);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_052.
#[test]
fn structured_quotient_052() {
    check_quotient_payload("setprecision(1000); 1/8", 1, 8);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_053.
#[test]
fn structured_quotient_053() {
    check_quotient_payload("setprecision(1000); 1/1024", 1, 1024);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_054.
#[test]
fn structured_quotient_054() {
    check_quotient_payload("setprecision(1000); 1/1009", 1, 1009);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_055.
#[test]
fn structured_quotient_055() {
    check_quotient_payload("setprecision(1000); 1/17", 1, 17);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_056.
#[test]
fn structured_quotient_056() {
    check_quotient_payload("setprecision(1000); 22/7", 22, 7);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_057.
#[test]
fn structured_quotient_057() {
    check_quotient_payload("setprecision(1000); 1/3", 1, 3);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_058.
#[test]
fn structured_quotient_058() {
    check_quotient_payload("setprecision(1000); 355/113", 355, 113);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_059.
#[test]
fn structured_quotient_059() {
    check_quotient_payload("setprecision(1000); 3/2", 3, 2);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_007.
#[test]
fn structured_quotient_007() {
    check_quotient_payload("setprecision(5); 1/3", 1, 3);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_008.
#[test]
fn structured_quotient_008() {
    check_quotient_payload("setprecision(5); 355/113", 355, 113);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:58 — structured_quotient_009.
#[test]
fn structured_quotient_009() {
    check_quotient_payload("setprecision(5); 3/2", 3, 2);
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:115 — structured_exact_past_eighteen.
#[test]
fn structured_exact_past_eighteen() {
    check_rational_payload("1/(10^19)", "1", "10000000000000000000");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:182 — structured_sqrt_four.
#[test]
fn structured_sqrt_four() {
    check_payload("sqrt(4)", "2", "2", "1");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:136 — structured_periodic_times_divisor.
#[test]
fn structured_periodic_times_divisor() {
    check_payload("setprecision(18); (1/17)*17", "1", "1", "1");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:125 — structured_periodic_one_seventeenth.
#[test]
fn structured_periodic_one_seventeenth() {
    check_payload("1/17", "0.(0588235294117647)", "1", "17");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_000.
#[test]
fn structured_terminating_000() {
    check_rational_payload("setprecision(1); 1/2", "1", "2");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_001.
#[test]
fn structured_terminating_001() {
    check_rational_payload("setprecision(1); 1/8", "1", "8");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_010.
#[test]
fn structured_terminating_010() {
    check_rational_payload("setprecision(5); 1/2", "1", "2");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_011.
#[test]
fn structured_terminating_011() {
    check_rational_payload("setprecision(5); 1/8", "1", "8");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_012.
#[test]
fn structured_terminating_012() {
    check_rational_payload("setprecision(5); 1/125", "1", "125");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_013.
#[test]
fn structured_terminating_013() {
    check_rational_payload("setprecision(5); 1/10", "1", "10");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_014.
#[test]
fn structured_terminating_014() {
    check_rational_payload("setprecision(5); 1/1048576", "1", "1048576");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_015.
#[test]
fn structured_terminating_015() {
    check_rational_payload("setprecision(5); 1/95367431640625", "1", "95367431640625");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_016.
#[test]
fn structured_terminating_016() {
    check_rational_payload(
        "setprecision(5); 1/100000000000000000000",
        "1",
        "100000000000000000000",
    );
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_017.
#[test]
fn structured_terminating_017() {
    check_rational_payload(
        "setprecision(5); 1/18446744073709551616",
        "1",
        "18446744073709551616",
    );
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_018.
#[test]
fn structured_terminating_018() {
    check_rational_payload(
        "setprecision(5); 1/542101086242752217003726400434970855712890625",
        "1",
        "542101086242752217003726400434970855712890625",
    );
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_002.
#[test]
fn structured_terminating_002() {
    check_rational_payload("setprecision(1); 1/125", "1", "125");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_020.
#[test]
fn structured_terminating_020() {
    check_rational_payload("setprecision(18); 1/2", "1", "2");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_021.
#[test]
fn structured_terminating_021() {
    check_rational_payload("setprecision(18); 1/8", "1", "8");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_022.
#[test]
fn structured_terminating_022() {
    check_rational_payload("setprecision(18); 1/125", "1", "125");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_023.
#[test]
fn structured_terminating_023() {
    check_rational_payload("setprecision(18); 1/10", "1", "10");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_024.
#[test]
fn structured_terminating_024() {
    check_rational_payload("setprecision(18); 1/1048576", "1", "1048576");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_025.
#[test]
fn structured_terminating_025() {
    check_rational_payload("setprecision(18); 1/95367431640625", "1", "95367431640625");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_026.
#[test]
fn structured_terminating_026() {
    check_rational_payload(
        "setprecision(18); 1/100000000000000000000",
        "1",
        "100000000000000000000",
    );
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_027.
#[test]
fn structured_terminating_027() {
    check_rational_payload(
        "setprecision(18); 1/18446744073709551616",
        "1",
        "18446744073709551616",
    );
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_028.
#[test]
fn structured_terminating_028() {
    check_rational_payload(
        "setprecision(18); 1/542101086242752217003726400434970855712890625",
        "1",
        "542101086242752217003726400434970855712890625",
    );
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_003.
#[test]
fn structured_terminating_003() {
    check_rational_payload("setprecision(1); 1/10", "1", "10");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_030.
#[test]
fn structured_terminating_030() {
    check_rational_payload("setprecision(30); 1/2", "1", "2");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_031.
#[test]
fn structured_terminating_031() {
    check_rational_payload("setprecision(30); 1/8", "1", "8");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_032.
#[test]
fn structured_terminating_032() {
    check_rational_payload("setprecision(30); 1/125", "1", "125");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_033.
#[test]
fn structured_terminating_033() {
    check_rational_payload("setprecision(30); 1/10", "1", "10");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_034.
#[test]
fn structured_terminating_034() {
    check_rational_payload("setprecision(30); 1/1048576", "1", "1048576");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_035.
#[test]
fn structured_terminating_035() {
    check_rational_payload("setprecision(30); 1/95367431640625", "1", "95367431640625");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_036.
#[test]
fn structured_terminating_036() {
    check_rational_payload(
        "setprecision(30); 1/100000000000000000000",
        "1",
        "100000000000000000000",
    );
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_037.
#[test]
fn structured_terminating_037() {
    check_rational_payload(
        "setprecision(30); 1/18446744073709551616",
        "1",
        "18446744073709551616",
    );
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_038.
#[test]
fn structured_terminating_038() {
    check_rational_payload(
        "setprecision(30); 1/542101086242752217003726400434970855712890625",
        "1",
        "542101086242752217003726400434970855712890625",
    );
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_004.
#[test]
fn structured_terminating_004() {
    check_rational_payload("setprecision(1); 1/1048576", "1", "1048576");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_040.
#[test]
fn structured_terminating_040() {
    check_rational_payload("setprecision(1000); 1/2", "1", "2");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_041.
#[test]
fn structured_terminating_041() {
    check_rational_payload("setprecision(1000); 1/8", "1", "8");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_042.
#[test]
fn structured_terminating_042() {
    check_rational_payload("setprecision(1000); 1/125", "1", "125");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_043.
#[test]
fn structured_terminating_043() {
    check_rational_payload("setprecision(1000); 1/10", "1", "10");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_044.
#[test]
fn structured_terminating_044() {
    check_rational_payload("setprecision(1000); 1/1048576", "1", "1048576");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_045.
#[test]
fn structured_terminating_045() {
    check_rational_payload(
        "setprecision(1000); 1/95367431640625",
        "1",
        "95367431640625",
    );
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_046.
#[test]
fn structured_terminating_046() {
    check_rational_payload(
        "setprecision(1000); 1/100000000000000000000",
        "1",
        "100000000000000000000",
    );
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_047.
#[test]
fn structured_terminating_047() {
    check_rational_payload(
        "setprecision(1000); 1/18446744073709551616",
        "1",
        "18446744073709551616",
    );
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_048.
#[test]
fn structured_terminating_048() {
    check_rational_payload(
        "setprecision(1000); 1/542101086242752217003726400434970855712890625",
        "1",
        "542101086242752217003726400434970855712890625",
    );
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_049.
#[test]
fn structured_terminating_049() {
    check_rational_payload(
        "setprecision(1000); 1/158456325028528675187087900672000",
        "1",
        "158456325028528675187087900672000",
    );
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_005.
#[test]
fn structured_terminating_005() {
    check_rational_payload("setprecision(1); 1/95367431640625", "1", "95367431640625");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_006.
#[test]
fn structured_terminating_006() {
    check_rational_payload(
        "setprecision(1); 1/100000000000000000000",
        "1",
        "100000000000000000000",
    );
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_007.
#[test]
fn structured_terminating_007() {
    check_rational_payload(
        "setprecision(1); 1/18446744073709551616",
        "1",
        "18446744073709551616",
    );
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:86 — structured_terminating_008.
#[test]
fn structured_terminating_008() {
    check_rational_payload(
        "setprecision(1); 1/542101086242752217003726400434970855712890625",
        "1",
        "542101086242752217003726400434970855712890625",
    );
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:172 — structured_irrational_002.
#[test]
fn structured_irrational_002() {
    check_not_exact("sqrt(2)");
}

/// C# Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:172 — structured_irrational_005.
#[test]
fn structured_irrational_005() {
    check_not_exact("setprecision(18); sqrt(2)");
}

/// C# Lovelace.Suite.Tests/ArrayTests.cs:18 — GivenMatrixLiteral_ProducesRank2Array.
#[test]
fn arraytests_givenmatrixliteral_producesrank2array() {
    check_typed("[[1, 2], [3, 4]]", "[[1, 2], [3, 4]] (Array)");
    check_shape("[[1, 2], [3, 4]]", "2,2");
}

/// C# Lovelace.Suite.Tests/ArrayTests.cs:27 — GivenRank3Literal_ProducesRank3Array.
#[test]
fn arraytests_givenrank3literal_producesrank3array() {
    check_shape("[[[1, 2], [3, 4]], [[5, 6], [7, 8]]]", "2,2,2");
}

/// C# Lovelace.Suite.Tests/ArrayTests.cs:53 — GivenPartialIndex_ReturnsLowerRank.
#[test]
fn arraytests_givenpartialindex_returnslowerrank() {
    check_shape("[[[1, 2], [3, 4]], [[5, 6], [7, 8]]][0]", "2,2");
}

/// C# Lovelace.Suite.Tests/ArrayTests.cs:61 — GivenFullIndex_ReturnsElement.
#[test]
fn arraytests_givenfullindex_returnselement() {
    check_typed(
        "[[[1, 2], [3, 4]], [[5, 6], [7, 8]]][1, 0, 1]",
        "6 (Natural)",
    );
}

/// C# Lovelace.Suite.Tests/ArrayTests.cs:79 — Zeros_ProducesRequestedShape.
#[test]
fn arraytests_zeros_producesrequestedshape() {
    check_typed("zeros(2, 3)", "[[0, 0, 0], [0, 0, 0]] (Array)");
}

/// C# Lovelace.Suite.Tests/ArrayTests.cs:83 — Ones_SingleDimension_IsVector.
#[test]
fn arraytests_ones_singledimension_isvector() {
    check_typed("ones(3)", "[1, 1, 1] (Vector)");
}

/// C# Lovelace.Suite.Tests/ArrayTests.cs:87 — Eye_ProducesIdentity.
#[test]
fn arraytests_eye_producesidentity() {
    check_typed("eye(2)", "[[1, 0], [0, 1]] (Array)");
}

/// C# Lovelace.Suite.Tests/ArrayTests.cs:91 — Reshape_ProducesNewShape.
#[test]
fn arraytests_reshape_producesnewshape() {
    check_typed("reshape(1..6, 2, 3)", "[[1, 2, 3], [4, 5, 6]] (Array)");
}

/// C# Lovelace.Suite.Tests/ArrayTests.cs:95 — ShapeRankNumel_ReportMetadata.
#[test]
fn arraytests_shaperanknumel_reportmetadata() {
    check_typed("shape(zeros(2, 3))", "[2, 3] (Vector)");
    check_typed("rank(zeros(2, 3))", "2 (Natural)");
    check_typed("numel(zeros(2, 3))", "6 (Natural)");
}

/// C# Lovelace.Suite.Tests/ArrayTests.cs:103 — Flatten_ReturnsVector.
#[test]
fn arraytests_flatten_returnsvector() {
    check_typed("flatten([[1, 2], [3, 4]])", "[1, 2, 3, 4] (Vector)");
}

/// C# Lovelace.Suite.Tests/ArrayTests.cs:107 — Transpose_ReturnsTranspose.
#[test]
fn arraytests_transpose_returnstranspose() {
    check_typed("transpose([[1, 2], [3, 4]])", "[[1, 3], [2, 4]] (Array)");
}

/// C# Lovelace.Suite.Tests/ArrayTests.cs:119 — Sum_AllAndAxis.
#[test]
fn arraytests_sum_allandaxis() {
    check_typed("sum([[1, 2], [3, 4]])", "10 (Natural)");
    check_typed("sum([[1, 2], [3, 4]], 0)", "[4, 6] (Vector)");
    check_typed("sum([[1, 2], [3, 4]], 1)", "[3, 7] (Vector)");
}

/// C# Lovelace.Suite.Tests/ArrayTests.cs:134 — Norm_Euclidean.
#[test]
fn arraytests_norm_euclidean() {
    check_typed("norm([3, 4])", "5 (Real)");
}

/// C# Lovelace.Suite.Tests/ArrayTests.cs:142 — MatMul_2x2.
#[test]
fn arraytests_matmul_2x2() {
    check_typed(
        "matmul([[1, 2], [3, 4]], [[5, 6], [7, 8]])",
        "[[19, 22], [43, 50]] (Array)",
    );
}

/// C# Lovelace.Suite.Tests/ArrayTests.cs:147 — Dot_InnerProduct.
#[test]
fn arraytests_dot_innerproduct() {
    check_typed("dot([1, 2], [3, 4])", "11 (Natural)");
}

/// C# Lovelace.Suite.Tests/ArrayTests.cs:151 — Cross_3D.
#[test]
fn arraytests_cross_3d() {
    check_typed("cross([1, 0, 0], [0, 1, 0])", "[0, 0, 1] (Vector)");
}

/// C# Lovelace.Suite.Tests/ArrayTests.cs:155 — Det_ReturnsDeterminant.
#[test]
fn arraytests_det_returnsdeterminant() {
    check_typed("det([[1, 2], [3, 4]])", "-2 (Integer)");
}

/// C# Lovelace.Suite.Tests/ArrayTests.cs:159 — Inv_ReturnsInverse.
#[test]
fn arraytests_inv_returnsinverse() {
    check_typed("inv([[1, 2], [3, 4]])", "[[-2, 1], [1.5, -0.5]] (Array)");
}

/// C# Lovelace.Suite.Tests/ArrayTests.cs:167 — Trace_ReturnsDiagonalSum.
#[test]
fn arraytests_trace_returnsdiagonalsum() {
    check_typed("trace([[1, 2], [3, 4]])", "5 (Natural)");
}

/// C# Lovelace.Suite.Tests/ArrayTests.cs:175 — ConcatAndAppend_Concatenate.
#[test]
fn arraytests_concatandappend_concatenate() {
    check_typed("concat([1, 2], [3, 4])", "[1, 2, 3, 4] (Vector)");
    check_typed("append([1, 2], [3, 4])", "[1, 2, 3, 4] (Vector)");
}

/// C# Lovelace.Suite.Tests/StatementTests.cs:13 — Evaluate_GivenBlock_ReturnsLastExpressionValue.
#[test]
fn statementtests_evaluate_givenblock_returnslastexpressio() {
    check_typed("{ 1; 2; 3 }", "3 (Natural)");
}

/// C# Lovelace.Suite.Tests/StatementTests.cs:24 — Evaluate_GivenIfElseTrueBranch_SelectsTrueBranch.
#[test]
fn statementtests_evaluate_givenifelsetruebranch_selectstr() {
    check_typed("if (2 > 1) { 10 } else { 20 }", "10 (Natural)");
}

/// C# Lovelace.Suite.Tests/StatementTests.cs:34 — Evaluate_GivenIfElseFalseBranch_SelectsFalseBranch.
#[test]
fn statementtests_evaluate_givenifelsefalsebranch_selectsf() {
    check_typed("if (2 < 1) { 10 } else { 20 }", "20 (Natural)");
}

/// C# Lovelace.Suite.Tests/StatementTests.cs:44 — Evaluate_GivenWhileLoop_AccumulatesCounter.
#[test]
fn statementtests_evaluate_givenwhileloop_accumulatescount() {
    check_typed("i = 0; while (i < 5) { i = i + 1 }; i", "5 (Natural)");
}

/// C# Lovelace.Suite.Tests/StatementTests.cs:54 — Evaluate_GivenForRange_IteratesInclusiveBounds.
#[test]
fn statementtests_evaluate_givenforrange_iteratesinclusive() {
    check_typed(
        "sum = 0; for i in 1..4 { sum = sum + i }; sum",
        "10 (Natural)",
    );
}

/// C# Lovelace.Suite.Tests/StatementTests.cs:64 — Evaluate_GivenForBreak_StopsEarly.
#[test]
fn statementtests_evaluate_givenforbreak_stopsearly() {
    check_typed(
        "n = 0; for i in 1..10 { n = i; if (i == 3) { break } }; n",
        "3 (Natural)",
    );
}

/// C# Lovelace.Suite.Tests/StatementTests.cs:74 — Evaluate_GivenForContinue_SkipsElement.
#[test]
fn statementtests_evaluate_givenforcontinue_skipselement() {
    check_typed(
        "sum = 0; for i in 1..5 { if (i == 3) { continue }; sum = sum + i }; sum",
        "12 (Natural)",
    );
}

/// C# Lovelace.Suite.Tests/StatementTests.cs:85 — Evaluate_GivenReturnAtTopLevel_ReturnsValue.
#[test]
fn statementtests_evaluate_givenreturnattoplevel_returnsva() {
    check_typed("return 42", "42 (Natural)");
}

/// C# Lovelace.Suite.Tests/StatementTests.cs:113 — Evaluate_GivenRange_ProducesNaturalVector.
#[test]
fn statementtests_evaluate_givenrange_producesnaturalvecto() {
    check_count("1..5", 5);
    check_element("1..5", 0, "Natural=1");
    check_element("1..5", 4, "Natural=5");
}

/// C# Lovelace.Suite.Tests/StatementTests.cs:139 — Evaluate_GivenNegativeRange_ProducesIntegers.
#[test]
fn statementtests_evaluate_givennegativerange_producesinte() {
    check_element("-2..2", 0, "Integer=-2");
}

/// C# Lovelace.Suite.Tests/StatementTests.cs:150 — Evaluate_GivenRangeSquared_AppliesElementWise.
#[test]
fn statementtests_evaluate_givenrangesquared_applieselemen() {
    check_count("1..10 ^ 2", 10);
    check_element("1..10 ^ 2", 0, "Natural=1");
    check_element("1..10 ^ 2", 9, "Natural=100");
}

/// C# Lovelace.Suite.Tests/FunctionTests.cs:10 — Evaluate_GivenExpressionBodiedFunction_ReturnsResult.
#[test]
fn functiontests_evaluate_givenexpressionbodiedfunction_r() {
    check_typed("func square(x) = x ^ 2; square(5)", "25 (Natural)");
}

/// C# Lovelace.Suite.Tests/FunctionTests.cs:22 — Evaluate_GivenBlockBodiedFunction_ReturnsLastExpression.
#[test]
fn functiontests_evaluate_givenblockbodiedfunction_return() {
    check_typed("func f(a, b) { c = a + b; c * 2 }; f(3, 4)", "14 (Natural)");
}

/// C# Lovelace.Suite.Tests/FunctionTests.cs:33 — Evaluate_GivenExplicitReturn_ReturnsEarly.
#[test]
fn functiontests_evaluate_givenexplicitreturn_returnsearl() {
    check_typed(
        "func f(x) { if (x > 0) { return 1 }; return -1 }; f(5)",
        "1 (Natural)",
    );
    check_typed(
        "func f(x) { if (x > 0) { return 1 }; return -1 }; f(-5)",
        "-1 (Integer)",
    );
}

/// C# Lovelace.Suite.Tests/FunctionTests.cs:44 — Evaluate_GivenRecursiveFunction_ComputesFactorial.
#[test]
fn functiontests_evaluate_givenrecursivefunction_computes() {
    check_typed(
        "func fact(n) { if (n == 0) { return 1 }; n * fact(n - 1) }; fact(6)",
        "720 (Natural)",
    );
}

/// C# Lovelace.Suite.Tests/InterpolationPrintTests.cs:9 — Evaluate_GivenInterpolatedString_FormatsEmbeddedExpression.
#[test]
fn interpolationprinttests_evaluate_giveninterpolatedstring_formats() {
    check_text("$\"x = {3 + 4}\"", "x = 7");
}

/// C# Lovelace.Suite.Tests/InterpolationPrintTests.cs:20 — Evaluate_GivenInterpolatedStringWithVariable_SubstitutesValue.
#[test]
fn interpolationprinttests_evaluate_giveninterpolatedstringwithvari() {
    check_text("name = 42; $\"value is {name}\"", "value is 42");
}

/// C# Lovelace.Suite.Tests/InterpolationPrintTests.cs:31 — Evaluate_GivenEscapedBraces_ProducesLiteralBraces.
#[test]
fn interpolationprinttests_evaluate_givenescapedbraces_produceslite() {
    check_text("$\"set {{1, 2}}\"", "set {1, 2}");
}

/// C# Lovelace.Suite.Tests/InterpolationPrintTests.cs:41 — Evaluate_GivenPlainString_ReturnsText.
#[test]
fn interpolationprinttests_evaluate_givenplainstring_returnstext() {
    check_text("\"hello\"", "hello");
}

/// C# Lovelace.Suite.Tests/InterpolationPrintTests.cs:52 — Evaluate_GivenPrint_WritesRenderedValueAndReturnsVoid.
#[test]
fn interpolationprinttests_evaluate_givenprint_writesrenderedvaluea() {
    check_prints("print($\"hi {1..3}\")", "hi [1, 2, 3]");
}

/// C# Lovelace.Suite.Tests/SliceTests.cs:12 — VectorSlice_StartStop.
#[test]
fn slicetests_vectorslice_startstop() {
    check_typed("[0, 1, 2, 3, 4][1:4]", "[1, 2, 3] (Vector)");
}

/// C# Lovelace.Suite.Tests/SliceTests.cs:20 — MatrixSlice_Column.
#[test]
fn slicetests_matrixslice_column() {
    check_typed("[[1, 2, 3], [4, 5, 6]][:, 1]", "[2, 5] (Vector)");
}

/// C# Lovelace.Suite.Tests/SliceTests.cs:24 — MatrixSlice_Rows.
#[test]
fn slicetests_matrixslice_rows() {
    check_typed("[[1, 2, 3], [4, 5, 6]][1:2]", "[[4, 5, 6]] (Array)");
}

/// C# Lovelace.Suite.Tests/BroadcastTests.cs:13 — RowVector_BroadcastsToMatrix.
#[test]
fn broadcasttests_rowvector_broadcaststomatrix() {
    check_typed("[1, 2] + [[1, 2], [3, 4]]", "[[2, 4], [4, 6]] (Array)");
}

/// C# Lovelace.Suite.Tests/BroadcastTests.cs:25 — ScalarBroadcast_StillWorks.
#[test]
fn broadcasttests_scalarbroadcast_stillworks() {
    check_typed("[1, 2, 3] * 10", "[10, 20, 30] (Vector)");
}

/// C# Lovelace.Suite.Tests/EmptyReductionTests.cs:42 — Zeros_ZeroDimension_ProducesEmptyVector.
#[test]
fn emptyreductiontests_zeros_zerodimension_producesemptyvector() {
    check_elements("zeros(0)", "");
}

/// C# Lovelace.Suite.Tests/EmptyReductionTests.cs:51 — ReshapeEmpty_ToZeroDimensions_Works.
#[test]
fn emptyreductiontests_reshapeempty_tozerodimensions_works() {
    check_shape("reshape(zeros(0), 2, 0)", "2,0");
}

/// C# Lovelace.Suite.Tests/EmptyReductionTests.cs:60 — Shape_OfEmpty_Works.
#[test]
fn emptyreductiontests_shape_ofempty_works() {
    check_typed("shape(zeros(0))", "[0] (Vector)");
}

/// C# Lovelace.Suite.Tests/LenZeroDimensionTests.cs:41 — Len_IsTheFirstDimensionOfTheValue.
#[test]
fn lenzerodimensiontests_len_isthefirstdimensionofthevalue() {
    check_typed("len([[],[]])", "2 (Natural)");
}

/// C# Lovelace.Suite.Tests/LenZeroDimensionTests.cs:41 — Len_IsTheFirstDimensionOfTheValue.
#[test]
fn lenzerodimensiontests_len_isthefirstdimensionofthevalue_case1() {
    check_typed("len([[]])", "1 (Natural)");
}

/// C# Lovelace.Suite.Tests/LenZeroDimensionTests.cs:41 — Len_IsTheFirstDimensionOfTheValue.
#[test]
fn lenzerodimensiontests_len_isthefirstdimensionofthevalue_case2() {
    check_typed("len(zeros(2,0))", "2 (Natural)");
}

/// C# Lovelace.Suite.Tests/LenZeroDimensionTests.cs:41 — Len_IsTheFirstDimensionOfTheValue.
#[test]
fn lenzerodimensiontests_len_isthefirstdimensionofthevalue_case3() {
    check_typed("len(zeros(0,3))", "0 (Natural)");
}

/// C# Lovelace.Suite.Tests/LenZeroDimensionTests.cs:41 — Len_IsTheFirstDimensionOfTheValue.
#[test]
fn lenzerodimensiontests_len_isthefirstdimensionofthevalue_case4() {
    check_typed("len(zeros(0,0))", "0 (Natural)");
}

/// C# Lovelace.Suite.Tests/LenZeroDimensionTests.cs:41 — Len_IsTheFirstDimensionOfTheValue.
#[test]
fn lenzerodimensiontests_len_isthefirstdimensionofthevalue_case5() {
    check_typed("len(zeros(1,0))", "1 (Natural)");
}

/// C# Lovelace.Suite.Tests/LenZeroDimensionTests.cs:41 — Len_IsTheFirstDimensionOfTheValue.
#[test]
fn lenzerodimensiontests_len_isthefirstdimensionofthevalue_case6() {
    check_typed("len(zeros(2,3))", "2 (Natural)");
}

/// C# Lovelace.Suite.Tests/LenZeroDimensionTests.cs:41 — Len_IsTheFirstDimensionOfTheValue.
#[test]
fn lenzerodimensiontests_len_isthefirstdimensionofthevalue_case7() {
    check_typed("len(zeros(2,3,4))", "2 (Natural)");
}

/// C# Lovelace.Suite.Tests/LenZeroDimensionTests.cs:41 — Len_IsTheFirstDimensionOfTheValue.
#[test]
fn lenzerodimensiontests_len_isthefirstdimensionofthevalue_case8() {
    check_typed("len([1,2,3])", "3 (Natural)");
}

/// C# Lovelace.Suite.Tests/LenZeroDimensionTests.cs:41 — Len_IsTheFirstDimensionOfTheValue.
#[test]
fn lenzerodimensiontests_len_isthefirstdimensionofthevalue_case9() {
    check_typed("len([])", "0 (Natural)");
}

/// C# Lovelace.Suite.Tests/LenZeroDimensionTests.cs:54 — ZeroSizedArrays_StillPrintShapeAndIndexAsBefore.
#[test]
fn lenzerodimensiontests_zerosizedarrays_stillprintshapeandindexa() {
    check_typed("[[],[]]", "[[], []] (Array)");
    check_typed("zeros(2,0)", "[[], []] (Array)");
    check_typed("shape([[],[]])", "[2, 0] (Vector)");
    check_typed("zeros(2,0)[0]", "[] (Vector)");
    check_typed("numel(zeros(2,0))", "0 (Natural)");
    check_typed("rank(zeros(2,0))", "2 (Natural)");
}

/// C# Lovelace.Suite.Tests/TypedArrayPromotionTests.cs:36 — NaturalSubtraction_NoUnderflow_NarrowsBackToNatural.
#[test]
fn typedarraypromotiontests_naturalsubtraction_nounderflow_narrowsba() {
    check_elements("[5, 3] - [4, 2]", "Natural=1;Natural=1");
}

/// C# Lovelace.Suite.Tests/TypedArrayPromotionTests.cs:46 — NaturalSubtraction_Underflow_WidensToInteger.
#[test]
fn typedarraypromotiontests_naturalsubtraction_underflow_widenstoint() {
    check_elements("[5, 3] - [6, 4]", "Integer=-1;Integer=-1");
}

/// C# Lovelace.Suite.Tests/TypedArrayPromotionTests.cs:65 — NaturalDivision_Inexact_WidensToReal.
#[test]
fn typedarraypromotiontests_naturaldivision_inexact_widenstoreal() {
    check_elements("[1, 2] / [2, 2]", "Real=0.5;Real=1");
}

/// C# Lovelace.Suite.Tests/TypedArrayPromotionTests.cs:74 — ElementwiseAdd_MixedKinds_PromotesToReal.
#[test]
fn typedarraypromotiontests_elementwiseadd_mixedkinds_promotestoreal() {
    check_elements("[1, 2] + [3.5, 4.5]", "Real=4.5;Real=6.5");
}

/// C# Lovelace.Suite.Tests/TypedArrayPromotionTests.cs:103 — RealDivision_Inexact_StaysReal.
#[test]
fn typedarraypromotiontests_realdivision_inexact_staysreal() {
    check_elements("[1.0] / [3.0]", "Real=0.(3)");
}

/// C# Lovelace.Suite.Tests/ExactnessPreservationTests2.cs:30 — Arithmetic_DoesNotDegradeExactness.
#[test]
fn exactnesspreservationtests2_arithmetic_doesnotdegradeexactness() {
    check_exact("1/3 * 3", true);
    check_exact("1/3 - 1/3", true);
    check_exact("1/6 + 1/3", true);
    check_exact("1/3 + 0", true);
}

/// C# Lovelace.Suite.Tests/ExactnessPreservationTests2.cs:46 — ExactRationalTimesInteger_StaysExact.
#[test]
fn exactnesspreservationtests2_exactrationaltimesinteger_staysexact() {
    check_payload("1/3 * 3", "1", "1", "1");
}

/// C# Lovelace.Suite.Tests/ExactRationalPayloadTests2.cs:28 — ExactRational_CarriesNumeratorAndDenominator.
#[test]
fn exactrationalpayloadtests2_exactrational_carriesnumeratoranddenomin() {
    check_rational_payload("1/3", "1", "3");
}

/// C# Lovelace.Suite.Tests/ExactRationalPayloadTests2.cs:38 — ExactArithmetic_StaysExactAndRational.
#[test]
fn exactrationalpayloadtests2_exactarithmetic_staysexactandrational() {
    check_rational_payload("1/3 + 1/6", "1", "2");
}

/// C# Lovelace.Suite.Tests/InterpreterBuiltinSqrtTests.cs:27 — Evaluate_GivenSqrtOfNatural_ReturnsRealSquareRoot.
#[test]
fn interpreterbuiltinsqrttests_evaluate_givensqrtofnatural_returnsreals() {
    check_typed("sqrt(4)", "2 (Real)");
}

/// C# Lovelace.Suite.Tests/InterpreterBuiltinSqrtTests.cs:62 — Evaluate_GivenSqrtOfReal_ReturnsRealSquareRoot.
#[test]
fn interpreterbuiltinsqrttests_evaluate_givensqrtofreal_returnsrealsqua() {
    check_typed("sqrt(4.0)", "2 (Real)");
}

/// C# Lovelace.Suite.Tests/InterpreterFactorialTests.cs:21 — Evaluate_GivenNaturalFactorial_ReturnsNatural.
#[test]
fn interpreterfactorialtests_evaluate_givennaturalfactorial_returnsna() {
    check_typed("5!", "120 (Natural)");
}

/// C# Lovelace.Suite.Tests/InterpreterLiteralTests.cs:20 — Evaluate_GivenWholeNumberLiteral_ReturnsNaturalValue.
#[test]
fn interpreterliteraltests_evaluate_givenwholenumberliteral_returns() {
    check_typed("42", "42 (Natural)");
}

/// C# Lovelace.Suite.Tests/InterpreterLiteralTests.cs:35 — Evaluate_GivenDecimalLiteral_ReturnsRealValue.
#[test]
fn interpreterliteraltests_evaluate_givendecimalliteral_returnsreal() {
    check_typed("3.14", "3.14 (Real)");
}

/// C# Lovelace.Suite.Tests/InterpreterLiteralTests.cs:50 — Evaluate_GivenPeriodicLiteral_ReturnsRealValue.
#[test]
fn interpreterliteraltests_evaluate_givenperiodicliteral_returnsrea() {
    check_typed("0.(3)", "0.(3) (Real)");
}

/// C# Lovelace.Suite.Tests/InterpreterComparisonTests.cs:26 — Evaluate_GivenEqualValues_DoubleEqualsReturnsTrue.
#[test]
fn interpretercomparisontests_evaluate_givenequalvalues_doubleequalsre() {
    check_typed("5 == 5", "True (Boolean)");
}

/// C# Lovelace.Suite.Tests/InterpreterComparisonTests.cs:41 — Evaluate_GivenDifferentValues_NotEqualsReturnsTrue.
#[test]
fn interpretercomparisontests_evaluate_givendifferentvalues_notequalsr() {
    check_typed("3 != 5", "True (Boolean)");
}

/// C# Lovelace.Suite.Tests/InterpreterComparisonTests.cs:56 — Evaluate_GivenGreater_ReturnsBooleanTrue.
#[test]
fn interpretercomparisontests_evaluate_givengreater_returnsbooleantrue() {
    check_typed("5 > 3", "True (Boolean)");
}

/// C# Lovelace.Suite.Tests/InterpreterComparisonTests.cs:71 — Evaluate_GivenLess_ReturnsBooleanFalse.
#[test]
fn interpretercomparisontests_evaluate_givenless_returnsbooleanfalse() {
    check_typed("5 < 3", "False (Boolean)");
}

/// C# Lovelace.Suite.Tests/InterpreterComparisonTests.cs:86 — Evaluate_GivenCrossTypeComparison_WidensFirst.
#[test]
fn interpretercomparisontests_evaluate_givencrosstypecomparison_widens() {
    check_typed("5 == 5.0", "True (Boolean)");
}

/// C# Lovelace.Suite.Tests/InterpreterComparisonTests.cs:103 — Evaluate_GivenGreaterEqual_ReturnsTrueWhenEqual.
#[test]
fn interpretercomparisontests_evaluate_givengreaterequal_returnstruewh() {
    check_typed("5 >= 5", "True (Boolean)");
}

/// C# Lovelace.Suite.Tests/InterpreterComparisonTests.cs:114 — Evaluate_GivenLessEqual_ReturnsTrueWhenLess.
#[test]
fn interpretercomparisontests_evaluate_givenlessequal_returnstruewhenl() {
    check_typed("3 <= 5", "True (Boolean)");
}

/// C# Lovelace.Suite.Tests/InterpreterComparisonTests.cs:125 — Evaluate_GivenDoubleEquals_ReturnsFalseWhenDifferent.
#[test]
fn interpretercomparisontests_evaluate_givendoubleequals_returnsfalsew() {
    check_typed("3 == 7", "False (Boolean)");
}

/// C# Lovelace.Suite.Tests/InterpreterUnaryTests.cs:21 — Evaluate_GivenNegateNatural_WidensToIntegerAndNegates.
#[test]
fn interpreterunarytests_evaluate_givennegatenatural_widenstointe() {
    check_typed("-5", "-5 (Integer)");
}

/// C# Lovelace.Suite.Tests/InterpreterUnaryTests.cs:38 — Evaluate_GivenNegateInteger_ReturnsNegatedInteger.
#[test]
fn interpreterunarytests_evaluate_givennegateinteger_returnsnegat() {
    check_typed("-(-3)", "3 (Integer)");
}

/// C# Lovelace.Suite.Tests/InterpreterUnaryTests.cs:56 — Evaluate_GivenNegateReal_ReturnsNegatedReal.
#[test]
fn interpreterunarytests_evaluate_givennegatereal_returnsnegatedr() {
    check_typed("-3.14", "-3.14 (Real)");
}

/// C# Lovelace.Suite.Tests/InterpreterBuiltinSignTests.cs:28 — Evaluate_GivenSignOfNegative_ReturnsNegativeOne.
#[test]
fn interpreterbuiltinsigntests_evaluate_givensignofnegative_returnsnega() {
    check_typed("sign(-5)", "-1 (Integer)");
}

/// C# Lovelace.Suite.Tests/InterpreterBuiltinSignTests.cs:46 — Evaluate_GivenSignOfZero_ReturnsZero.
#[test]
fn interpreterbuiltinsigntests_evaluate_givensignofzero_returnszero() {
    check_typed("sign(0)", "0 (Integer)");
}

/// C# Lovelace.Suite.Tests/InterpreterBuiltinSignTests.cs:62 — Evaluate_GivenSignOfPositive_ReturnsOne.
#[test]
fn interpreterbuiltinsigntests_evaluate_givensignofpositive_returnsone() {
    check_typed("sign(5)", "1 (Integer)");
}

/// C# Lovelace.Suite.Tests/InterpreterBuiltinSignTests.cs:77 — Evaluate_GivenSignOfLargeNegative_ReturnsNegativeOne.
#[test]
fn interpreterbuiltinsigntests_evaluate_givensignoflargenegative_return() {
    check_typed("sign(-100)", "-1 (Integer)");
}

/// C# Lovelace.Suite.Tests/InterpreterBuiltinAbsTests.cs:28 — Evaluate_GivenAbsOfNegativeInteger_ReturnsPositiveInteger.
#[test]
fn interpreterbuiltinabstests_evaluate_givenabsofnegativeinteger_retur() {
    check_typed("abs(-5)", "5 (Integer)");
}

/// C# Lovelace.Suite.Tests/InterpreterBuiltinAbsTests.cs:46 — Evaluate_GivenAbsOfPositiveNatural_ReturnsSameNatural.
#[test]
fn interpreterbuiltinabstests_evaluate_givenabsofpositivenatural_retur() {
    check_typed("abs(5)", "5 (Natural)");
}

/// C# Lovelace.Suite.Tests/InterpreterBuiltinAbsTests.cs:61 — Evaluate_GivenAbsOfNegativeReal_ReturnsPositiveReal.
#[test]
fn interpreterbuiltinabstests_evaluate_givenabsofnegativereal_returnsp() {
    check_typed("abs(-3.14)", "3.14 (Real)");
}

/// C# Lovelace.Suite.Tests/InterpreterBuiltinInvTests.cs:27 — Evaluate_GivenInvOfNatural_WidensToRealAndInverts.
#[test]
fn interpreterbuiltininvtests_evaluate_giveninvofnatural_widenstoreala() {
    check_typed("inv(4)", "0.25 (Real)");
}

/// C# Lovelace.Suite.Tests/InterpreterBuiltinIsEvenIsOddTests.cs:31 — Evaluate_GivenIsEvenOfEvenNumber_ReturnsTrue.
#[test]
fn interpreterbuiltinisevenisoddtests_evaluate_givenisevenofevennumber_returns() {
    check_typed("is_even(4)", "True (Boolean)");
}

/// C# Lovelace.Suite.Tests/InterpreterBuiltinIsEvenIsOddTests.cs:46 — Evaluate_GivenIsEvenOfOddNumber_ReturnsFalse.
#[test]
fn interpreterbuiltinisevenisoddtests_evaluate_givenisevenofoddnumber_returnsf() {
    check_typed("is_even(3)", "False (Boolean)");
}

/// C# Lovelace.Suite.Tests/InterpreterBuiltinIsEvenIsOddTests.cs:62 — Evaluate_GivenIsEvenOfEvenNegativeInteger_ReturnsTrue.
#[test]
fn interpreterbuiltinisevenisoddtests_evaluate_givenisevenofevennegativeintege() {
    check_typed("is_even(-4)", "True (Boolean)");
}

/// C# Lovelace.Suite.Tests/InterpreterBuiltinIsEvenIsOddTests.cs:79 — Evaluate_GivenIsOddOfOddNumber_ReturnsTrue.
#[test]
fn interpreterbuiltinisevenisoddtests_evaluate_givenisoddofoddnumber_returnstr() {
    check_typed("is_odd(3)", "True (Boolean)");
}

/// C# Lovelace.Suite.Tests/InterpreterBuiltinIsEvenIsOddTests.cs:94 — Evaluate_GivenIsOddOfEvenNumber_ReturnsFalse.
#[test]
fn interpreterbuiltinisevenisoddtests_evaluate_givenisoddofevennumber_returnsf() {
    check_typed("is_odd(4)", "False (Boolean)");
}

/// C# Lovelace.Suite.Tests/InterpreterBuiltinIsEvenIsOddTests.cs:110 — Evaluate_GivenIsOddOfOddNegativeInteger_ReturnsTrue.
#[test]
fn interpreterbuiltinisevenisoddtests_evaluate_givenisoddofoddnegativeinteger_() {
    check_typed("is_odd(-3)", "True (Boolean)");
}

/// C# Lovelace.Suite.Tests/InterpreterBuiltinDivRemTests.cs:43 — Evaluate_GivenDivRemOfNaturals_ReturnsAStructuredRecord.
#[test]
fn interpreterbuiltindivremtests_evaluate_givendivremofnaturals_returnsas() {
    check_typed(
        "divrem(17, 5)",
        "DivRemResult(quotient: 3, remainder: 2) (DivRemResult)",
    );
}

/// C# Lovelace.Suite.Tests/InterpreterBuiltinDivRemTests.cs:64 — Evaluate_GivenDivRemWithANegativeDividend_KeepsTheTruncatedSigns.
#[test]
fn interpreterbuiltindivremtests_evaluate_givendivremwithanegativedividen() {
    check_typed(
        "divrem(-17, 5)",
        "DivRemResult(quotient: -3, remainder: -2) (DivRemResult)",
    );
}

/// C# Lovelace.Suite.Tests/InterpreterBinaryArithmeticTests.cs:26 — Evaluate_GivenNaturalPlusNatural_ReturnsNatural.
#[test]
fn interpreterbinaryarithmetictests_evaluate_givennaturalplusnatural_returns() {
    check_typed("2 + 3", "5 (Natural)");
}

/// C# Lovelace.Suite.Tests/InterpreterBinaryArithmeticTests.cs:44 — Evaluate_GivenNaturalPlusReal_WidensAndReturnsReal.
#[test]
fn interpreterbinaryarithmetictests_evaluate_givennaturalplusreal_widensandr() {
    check_typed("5 + 3.0", "8 (Real)");
}

/// C# Lovelace.Suite.Tests/InterpreterBinaryArithmeticTests.cs:61 — Evaluate_GivenRealMinusReal_ReturnsCorrectDifference.
#[test]
fn interpreterbinaryarithmetictests_evaluate_givenrealminusreal_returnscorre() {
    check_typed("5.0 - 3.5", "1.5 (Real)");
}

/// C# Lovelace.Suite.Tests/InterpreterBinaryArithmeticTests.cs:77 — Evaluate_GivenMultiply_ReturnsCorrectProduct.
#[test]
fn interpreterbinaryarithmetictests_evaluate_givenmultiply_returnscorrectpro() {
    check_typed("12 * 34", "408 (Natural)");
}

/// C# Lovelace.Suite.Tests/InterpreterBinaryArithmeticTests.cs:119 — Evaluate_GivenModulo_ReturnsCorrectRemainder.
#[test]
fn interpreterbinaryarithmetictests_evaluate_givenmodulo_returnscorrectremai() {
    check_typed("17 % 5", "2 (Natural)");
}

/// C# Lovelace.Suite.Tests/InterpreterBinaryArithmeticTests.cs:134 — Evaluate_GivenPower_ReturnsCorrectResult.
#[test]
fn interpreterbinaryarithmetictests_evaluate_givenpower_returnscorrectresult() {
    check_typed("2 ^ 10", "1024 (Natural)");
}

/// C# Lovelace.Suite.Tests/InterpreterBinaryArithmeticTests.cs:149 — Evaluate_GivenRealPlusReal_ReturnsCorrectSum.
#[test]
fn interpreterbinaryarithmetictests_evaluate_givenrealplusreal_returnscorrec() {
    check_typed("1.5 + 2.5", "4 (Real)");
}

/// C# Lovelace.Suite.Tests/InterpreterBinaryArithmeticTests.cs:164 — Evaluate_GivenNaturalMinusSmaller_ReturnsNaturalDifference.
#[test]
fn interpreterbinaryarithmetictests_evaluate_givennaturalminussmaller_return() {
    check_typed("10 - 3", "7 (Natural)");
}

/// C# Lovelace.Suite.Tests/InterpreterBinaryArithmeticTests.cs:179 — Evaluate_GivenNaturalMinusLarger_AutoWidensToInteger.
#[test]
fn interpreterbinaryarithmetictests_evaluate_givennaturalminuslarger_autowid() {
    check_typed("3 - 5", "-2 (Integer)");
}

/// C# Lovelace.Suite.Tests/InterpreterBinaryArithmeticTests.cs:196 — Evaluate_GivenNaturalMinusEqual_ReturnsNaturalZero.
#[test]
fn interpreterbinaryarithmetictests_evaluate_givennaturalminusequal_returnsn() {
    check_typed("5 - 5", "0 (Natural)");
}

/// C# Lovelace.Suite.Tests/InterpreterBinaryArithmeticTests.cs:212 — Evaluate_GivenNaturalDivideNonExact_WidensToReal.
#[test]
fn interpreterbinaryarithmetictests_evaluate_givennaturaldividenonexact_wide() {
    check_typed("1 / 3", "0.(3) (Real)");
}

/// C# Lovelace.Suite.Tests/InterpreterBinaryArithmeticTests.cs:223 — Evaluate_GivenIntegerDivideNonExact_WidensToReal.
#[test]
fn interpreterbinaryarithmetictests_evaluate_givenintegerdividenonexact_wide() {
    check_typed("-7 / 2", "-3.5 (Real)");
}

/// C# Lovelace.Suite.Tests/InterpreterBuiltinPiTests.cs:49 — Evaluate_GivenPiWithNaturalArgument_ReturnsRealWithRequestedDigits.
#[test]
fn interpreterbuiltinpitests_evaluate_givenpiwithnaturalargument_retu() {
    check_typed("pi(10)", "3.1415926535 (Real)");
}

/// C# Lovelace.Suite.Tests/InterpreterBuiltinPiTests.cs:24 — Evaluate_GivenPiWithNoArguments_ReturnsRealWithDisplayDecimalPlacesDigits.
#[test]
fn interpreterbuiltinpitests_evaluate_givenpiwithnoarguments_returnsr() {
    check_typed("setprecision(5); pi()", "3.14159 (Real)");
}

// ===========================================================================
// Round 12 batch — appended tests (Suite: SignPrecedence/SuiteEngine/plot;
// Symbolics: DiagnosticContract/DxSemanticClosure/Hyperbolic/abs).
// Each test names the C# case it ports; the C# assertion is the contract.
// ===========================================================================

use crate::syntax::{Expr, Parser, tokenize};

/// The engine's untyped rendering, which is what the C# `FormatValue` contract asserts.
fn r12_display(script: &str) -> String {
    let mut e = engine();
    match e.eval(script) {
        Ok(v) => e.format(&v),
        Err(err) => panic!("script: {script} unexpectedly failed: {}", err.message),
    }
}

fn r12_err(script: &str) -> crate::LangError {
    let mut e = engine();
    match e.eval(script) {
        Ok(v) => panic!(
            "script: {script} was expected to fail but produced {}",
            e.format_typed(&v)
        ),
        Err(err) => err,
    }
}

fn r12_eval_on(engine: &mut crate::Engine, script: &str) -> crate::Value {
    match engine.eval(script) {
        Ok(v) => v,
        Err(err) => panic!("script: {script} unexpectedly failed: {}", err.message),
    }
}

fn r12_type_name(v: &crate::Value) -> &str {
    match v {
        crate::Value::Record(name, _) => name,
        other => panic!("not a record: {}", other.kind_name()),
    }
}

fn r12_fields(v: &crate::Value) -> &[(String, crate::Value)] {
    match v {
        crate::Value::Record(_, fields) => fields,
        other => panic!("not a record: {}", other.kind_name()),
    }
}

fn r12_field<'a>(v: &'a crate::Value, name: &str) -> &'a crate::Value {
    for (k, value) in r12_fields(v) {
        if k == name {
            return value;
        }
    }
    panic!("record has no field '{name}'");
}

fn r12_enum(v: &crate::Value) -> (&str, &str) {
    match v {
        crate::Value::Enum(ty, name) => (ty, name),
        other => panic!("not an enum: {}", other.kind_name()),
    }
}

fn r12_vector(v: &crate::Value) -> &[crate::Value] {
    match v {
        crate::Value::Vector(items) => items,
        other => panic!("not a vector: {}", other.kind_name()),
    }
}

fn r12_domain(v: &crate::Value) -> &str {
    match v {
        crate::Value::Domain(name) => name,
        other => panic!("not a domain: {}", other.kind_name()),
    }
}

fn r12_text(v: &crate::Value) -> &str {
    match v {
        crate::Value::Text(t) => t,
        other => panic!("not text: {}", other.kind_name()),
    }
}

fn r12_boolean(v: &crate::Value) -> bool {
    match v {
        crate::Value::Boolean(b) => *b,
        other => panic!("not a boolean: {}", other.kind_name()),
    }
}

fn r12_integer(v: &crate::Value) -> String {
    match v {
        crate::Value::Integer(i) => i.to_decimal(),
        crate::Value::Natural(n) => n.to_decimal(),
        other => panic!("not an integer: {}", other.kind_name()),
    }
}

/// The structured projection of a value, which is what the wire publishes.
fn r12_structured(engine: &crate::Engine, v: &crate::Value) -> Json {
    crate::structured::project_value(engine, v, None)
}

fn r12_json_str<'a>(j: &'a Json, key: &str) -> Option<&'a str> {
    j.get(key).and_then(|v| v.as_str())
}

fn r12_json_arr<'a>(j: &'a Json, key: &str) -> Option<&'a [Json]> {
    j.get(key).and_then(|v| v.as_arr())
}

// ---------------------------------------------------------------------------
// Lovelace.Suite.Tests/SignPrecedenceTests.cs (16 cases; +2^2 is refused)
// ---------------------------------------------------------------------------

/// C# SignPrecedenceTests.cs:32 — Parse_NegatedPower_SignWrapsThePower.
#[test]
fn signprecedencetests_parse_negatedpower_signwrapsthepower() {
    let toks = tokenize("-x^2").expect("tokenizes");
    let expr = Parser::new(toks).single_expression().expect("parses");
    match expr {
        Expr::Neg(inner) => match *inner {
            Expr::Binary(crate::syntax::BinOp::Pow, left, right) => {
                assert!(
                    matches!(*left, Expr::Var(ref n, _) if n == "x"),
                    "left is not x"
                );
                assert!(
                    matches!(*right, Expr::Number(ref t) if t == "2"),
                    "right is not 2"
                );
            }
            other => panic!("operand is not a power: {other:?}"),
        },
        other => panic!("not a negation: {other:?}"),
    }
}

/// C# SignPrecedenceTests.cs:44 — Parse_ParenthesisedNegatedBase_PowerAppliesToTheNegation.
#[test]
fn signprecedencetests_parse_parenthesisednegatedbase_powerappliestothenegation() {
    let toks = tokenize("(-x)^2").expect("tokenizes");
    let expr = Parser::new(toks).single_expression().expect("parses");
    match expr {
        Expr::Binary(crate::syntax::BinOp::Pow, left, _) => {
            assert!(
                matches!(*left, Expr::Neg(_)),
                "base is not the negation: {left:?}"
            );
        }
        other => panic!("not a power: {other:?}"),
    }
}

/// C# SignPrecedenceTests.cs:54 — Parse_NegativeExponent_ParsesAsSignedPower.
#[test]
fn signprecedencetests_parse_negativeexponent_parsesassignedpower() {
    let toks = tokenize("2^-1").expect("tokenizes");
    let expr = Parser::new(toks).single_expression().expect("parses");
    match expr {
        Expr::Binary(crate::syntax::BinOp::Pow, _, right) => {
            assert!(
                matches!(*right, Expr::Neg(_)),
                "exponent is not the negation: {right:?}"
            );
        }
        other => panic!("not a power: {other:?}"),
    }
}

/// C# SignPrecedenceTests.cs:64 — Parse_SignedRangeStart_IsNotANegatedRange.
#[test]
fn signprecedencetests_parse_signedrangestart_isnotanegatedrange() {
    let toks = tokenize("-1..2").expect("tokenizes");
    let expr = Parser::new(toks).single_expression().expect("parses");
    match expr {
        Expr::Range(start, None, end) => {
            match *start {
                Expr::Neg(inner) => {
                    assert!(
                        matches!(*inner, Expr::Number(ref t) if t == "1"),
                        "start is not 1"
                    );
                }
                other => panic!("range start is not the negation: {other:?}"),
            }
            assert!(
                matches!(*end, Expr::Number(ref t) if t == "2"),
                "end is not 2"
            );
        }
        other => panic!("not a range: {other:?}"),
    }
}

/// C# SignPrecedenceTests.cs:75 — Parse_RangeStillBindsTighterThanPower.
#[test]
fn signprecedencetests_parse_rangestillbindstighterthanpower() {
    let toks = tokenize("1..10^2").expect("tokenizes");
    let expr = Parser::new(toks).single_expression().expect("parses");
    match expr {
        Expr::Binary(crate::syntax::BinOp::Pow, left, right) => {
            assert!(
                matches!(*left, Expr::Range(_, _, _)),
                "left is not the range: {left:?}"
            );
            assert!(
                matches!(*right, Expr::Number(ref t) if t == "2"),
                "right is not 2"
            );
        }
        other => panic!("not a power: {other:?}"),
    }
}

/// C# SignPrecedenceTests.cs:84 — Parse_PostfixStillBindsTighterThanSign.
#[test]
fn signprecedencetests_parse_postfixstillbindstighterthansign() {
    let toks = tokenize("-5!").expect("tokenizes");
    let expr = Parser::new(toks).single_expression().expect("parses");
    match expr {
        Expr::Neg(inner) => assert!(
            matches!(*inner, Expr::Fact(_)),
            "operand is not a factorial: {inner:?}"
        ),
        other => panic!("not a negation: {other:?}"),
    }
}

/// C# SignPrecedenceTests.cs:96 — Evaluate_SignAndPowerFollowMathConvention("-2^2", "-4").
#[test]
fn signprecedencetests_evaluate_signandpower_minustwosquared() {
    assert_eq!(r12_display("-2^2"), "-4");
}

/// C# SignPrecedenceTests.cs:97 — Evaluate_SignAndPowerFollowMathConvention("-2^3", "-8").
#[test]
fn signprecedencetests_evaluate_signandpower_minustwocubed() {
    assert_eq!(r12_display("-2^3"), "-8");
}

/// C# SignPrecedenceTests.cs:98 — Evaluate_SignAndPowerFollowMathConvention("(-2)^2", "4").
#[test]
fn signprecedencetests_evaluate_signandpower_parenthesised() {
    assert_eq!(r12_display("(-2)^2"), "4");
}

/// C# SignPrecedenceTests.cs:99 — Evaluate_SignAndPowerFollowMathConvention("0-2^2", "-4").
#[test]
fn signprecedencetests_evaluate_signandpower_zerominustwosquared() {
    assert_eq!(r12_display("0-2^2"), "-4");
}

/// C# SignPrecedenceTests.cs:100 — Evaluate_SignAndPowerFollowMathConvention("10 - 2^2", "6").
#[test]
fn signprecedencetests_evaluate_signandpower_tenminustwosquared() {
    assert_eq!(r12_display("10 - 2^2"), "6");
}

/// C# SignPrecedenceTests.cs:101 — Evaluate_SignAndPowerFollowMathConvention("2^3^2", "512").
#[test]
fn signprecedencetests_evaluate_signandpower_right_associative() {
    assert_eq!(r12_display("2^3^2"), "512");
}

/// C# SignPrecedenceTests.cs:102 — Evaluate_SignAndPowerFollowMathConvention("-5!", "-120").
#[test]
fn signprecedencetests_evaluate_signandpower_factorial() {
    assert_eq!(r12_display("-5!"), "-120");
}

/// C# SignPrecedenceTests.cs:110 — Evaluate_SymbolicNegatedPower_HasNegativeValueAtTheSamplePoint.
#[test]
fn signprecedencetests_evaluate_symbolicnegatedpower_hasnegativevalueatthesamplepoint() {
    let mut e = engine();
    r12_eval_on(&mut e, "x = symbol(\"x\")");
    let v = r12_eval_on(&mut e, "subs(-x^2, x, 3)");
    assert_eq!(e.format(&v), "-9");
}

/// C# SignPrecedenceTests.cs:121 — Evaluate_SymbolicCanonicalForm_KeepsTheSignOutsideThePower.
#[test]
fn signprecedencetests_evaluate_symboliccanonicalform_keepsthesignoutsidethepower() {
    let mut e = engine();
    r12_eval_on(&mut e, "x = symbol(\"x\")");
    let v = r12_eval_on(&mut e, "inspect(-x^2).canonical");
    let canonical = r12_text(&v);
    assert!(
        canonical.contains("(pow (sym x) (rat 2 1))"),
        "canonical: {canonical}"
    );
    assert!(!canonical.contains("(pow (mul"), "canonical: {canonical}");
}

// ---------------------------------------------------------------------------
// Lovelace.Suite.Tests/SuiteEngineTests.cs (4 of 10)
// ---------------------------------------------------------------------------

/// C# SuiteEngineTests.cs:35 — Evaluate_GivenBuiltin_IsMarkedBuiltin.
#[test]
fn suiteenginetests_evaluate_givenbuiltin_ismarkedbuiltin() {
    let mut e = engine();
    let v = r12_eval_on(&mut e, "abs(-5)");
    assert_eq!(e.format_typed(&v), "5 (Integer)");
    assert!(
        crate::registry::find("abs").is_some(),
        "abs is not a registered builtin"
    );
}

/// C# SuiteEngineTests.cs:45 — Evaluate_GivenResult_StoresInUnderscoreVariable.
#[test]
fn suiteenginetests_evaluate_givenresult_storesinunderscorevariable() {
    let mut e = engine();
    r12_eval_on(&mut e, "1 + 1");
    let vars = e.variables();
    let underscore = vars
        .iter()
        .find(|(n, _)| n == "_")
        .unwrap_or_else(|| panic!("no '_' variable: {vars:?}"));
    assert_eq!(underscore.1, "2 (Natural)");
}

/// C# SuiteEngineTests.cs:132 — Parse_GivenStatements_ReturnsProgram.
#[test]
fn suiteenginetests_parse_givenstatements_returnsprogram() {
    let toks = tokenize("x = 1; y = 2; x + y").expect("tokenizes");
    let program = Parser::new(toks).program().expect("parses");
    assert_eq!(program.len(), 3);
}

/// C# SuiteEngineTests.cs:142 — ParseExpression_GivenExpression_ReturnsExpr.
#[test]
fn suiteenginetests_parseexpression_givenexpression_returnsexpr() {
    let toks = tokenize("1 + 2").expect("tokenizes");
    let expr = Parser::new(toks).single_expression().expect("parses");
    assert!(
        matches!(expr, Expr::Binary(crate::syntax::BinOp::Add, _, _)),
        "not a binary expr: {expr:?}"
    );
}

// ---------------------------------------------------------------------------
// Lovelace.Suite.Tests/SuiteEngineOutputPlotTests.cs (:21)
// ---------------------------------------------------------------------------

/// C# SuiteEngineOutputPlotTests.cs:21 — EvaluateAsync_GivenPlotCall_SetsLastPlotSvg.
#[test]
fn suiteengineoutputplottests_evaluateasync_givenplotcall_setslastplotsvg() {
    let dir = std::env::temp_dir().join("lovelace-r12-plot");
    let _ = std::fs::create_dir_all(&dir);
    let mut e = engine();
    e.set_plot_dir(&dir.to_string_lossy());
    let _ = e;
    // the file name setter is private to the crate's host; the assertion only needs the capture
    let v = r12_eval_on(&mut e, "plot(1..3, [1, 4, 9])");
    let _ = v;
    let plot = e.last_plot().unwrap_or_else(|| panic!("no plot captured"));
    assert!(
        plot.svg.starts_with("<svg"),
        "svg does not start with <svg: {}",
        &plot.svg[..plot.svg.len().min(40)]
    );
}

// ---------------------------------------------------------------------------
// Lovelace.Symbolics.Tests/DiagnosticContractTests.cs (13 of 14; the enum
// membership fact is refused: this crate has no ErrorCategory enum type)
// ---------------------------------------------------------------------------

fn r12_rich_result(type_name: &str, script: &str) {
    let mut e = engine();
    r12_eval_on(&mut e, "x = symbol(\"x\"); y = symbol(\"y\")");
    let record = r12_eval_on(&mut e, script);
    assert_eq!(r12_type_name(&record), type_name, "script: {script}");
    let fields = r12_fields(&record);
    assert_eq!(
        fields.last().map(|(n, _)| n.as_str()),
        Some("diagnostics"),
        "diagnostics is not the last field: {fields:?}"
    );
    let diagnostics = r12_field(&record, "diagnostics");
    assert_ne!(
        diagnostics.kind_name(),
        "Text",
        "diagnostics crossed as Text"
    );
    assert_ne!(
        diagnostics.kind_name(),
        "Void",
        "diagnostics crossed as Null"
    );
    assert_eq!(diagnostics.kind_name(), "Vector");
    let structured = r12_structured(&e, diagnostics);
    assert_eq!(
        r12_json_str(&structured, "kind"),
        Some("Array"),
        "structured: {}",
        structured
    );
    assert!(
        structured.get("value").is_none(),
        "an Array carries no scalar value: {}",
        structured
    );
    let elements = r12_json_arr(&structured, "elements")
        .unwrap_or_else(|| panic!("no elements: {}", structured));
    let shape =
        r12_json_arr(&structured, "shape").unwrap_or_else(|| panic!("no shape: {}", structured));
    // the shape's first entry is the element count
    match shape.first() {
        Some(Json::Num(s)) => {
            assert_eq!(elements.len().to_string(), *s, "shape[0] != element count")
        }
        other => panic!("shape[0] is not numeric: {other:?}"),
    }
}

/// C# DiagnosticContractTests.cs:67 — EveryRichResult_EndsWithADiagnosticsArrayNeverText(SolveResult).
#[test]
fn diagnosticcontracttests_everyrichresult_solve_result_ends_with_diagnostics_array() {
    r12_rich_result("SolveResult", "solve_full(x^4 - x^2 - 1 == 0, x)");
}

/// C# DiagnosticContractTests.cs:67 — ...(SystemSolveResult).
#[test]
fn diagnosticcontracttests_everyrichresult_system_solve_result_ends_with_diagnostics_array() {
    r12_rich_result(
        "SystemSolveResult",
        "solve_system_full([x + y == 1, x - y == 3], [x, y])",
    );
}

/// C# DiagnosticContractTests.cs:67 — ...(LimitResult).
#[test]
fn diagnosticcontracttests_everyrichresult_limit_result_ends_with_diagnostics_array() {
    r12_rich_result("LimitResult", "limit_full(sin(x)/x, x, 0)");
}

/// C# DiagnosticContractTests.cs:67 — ...(IntegrationResult).
#[test]
fn diagnosticcontracttests_everyrichresult_integration_result_ends_with_diagnostics_array() {
    r12_rich_result("IntegrationResult", "integrate_full(x^2, x)");
}

/// C# DiagnosticContractTests.cs:67 — ...(TransformResult).
#[test]
fn diagnosticcontracttests_everyrichresult_transform_result_ends_with_diagnostics_array() {
    r12_rich_result("TransformResult", "simplify_full(sin(x)^2 + cos(x)^2)");
}

/// C# DiagnosticContractTests.cs:67 — ...(OptimizationResult).
#[test]
fn diagnosticcontracttests_everyrichresult_optimization_result_ends_with_diagnostics_array() {
    r12_rich_result("OptimizationResult", "optimize_full(x^2 + 2*x + 1, [x])");
}

/// C# DiagnosticContractTests.cs:67 — ...(CompilationResult).
#[test]
fn diagnosticcontracttests_everyrichresult_compilation_result_ends_with_diagnostics_array() {
    r12_rich_result("CompilationResult", "compile_full(x^2 + 1, [x])");
}

/// C# DiagnosticContractTests.cs:91 — PartialSolve_CarriesADiagnosticWithAStableCodeAndAnErrorCategoryMember.
#[test]
fn diagnosticcontracttests_partialsolve_carriesadiagnosticwithastablecodeandacategory() {
    let mut e = engine();
    r12_eval_on(&mut e, "x = symbol(\"x\")");
    let record = r12_eval_on(&mut e, "solve_full(x^4 - x^2 - 1 == 0, x)");
    let (ty, status) = r12_enum(r12_field(&record, "status"));
    assert_eq!((ty, status), ("SolveStatus", "Partial"));
    let diagnostics = r12_vector(r12_field(&record, "diagnostics"));
    assert!(
        !diagnostics.is_empty(),
        "a partial solve carries at least one Diagnostic"
    );
    let diagnostic = &diagnostics[0];
    assert_eq!(r12_type_name(diagnostic), "Diagnostic");
    let names: Vec<&str> = r12_fields(diagnostic)
        .iter()
        .map(|(n, _)| n.as_str())
        .collect();
    assert_eq!(
        names,
        vec![
            "code",
            "category",
            "message",
            "recoverable",
            "location",
            "details"
        ]
    );
    let code = r12_field(diagnostic, "code");
    assert_eq!(code.kind_name(), "Text");
    assert_eq!(r12_text(code), "solve.unrepresented-roots");
    let category = r12_field(diagnostic, "category");
    let (cat_ty, cat_name) = r12_enum(category);
    assert_eq!(cat_ty, "ErrorCategory");
    assert!(
        [
            "ParseError",
            "DomainError",
            "UnsupportedOperation",
            "BudgetExceeded",
            "NoSolution",
            "TypeMismatch",
            "InternalInvariantFailure"
        ]
        .contains(&cat_name)
    );
    let message = r12_field(diagnostic, "message");
    assert_eq!(message.kind_name(), "Text");
    assert!(!r12_text(message).is_empty());
    assert_eq!(r12_field(diagnostic, "recoverable").kind_name(), "Boolean");
    let location = r12_field(diagnostic, "location");
    assert_eq!(location.kind_name(), "Void");
    assert_eq!(
        r12_json_str(&r12_structured(&e, location), "kind"),
        Some("Null")
    );
    let details = r12_field(diagnostic, "details");
    assert_eq!(details.kind_name(), "Vector");
    assert!(r12_vector(details).is_empty());
    assert_eq!(
        r12_json_str(&r12_structured(&e, details), "kind"),
        Some("Array")
    );
}

/// C# DiagnosticContractTests.cs:137 — CompleteSolveWithoutANote_CarriesAnEmptyDiagnosticsArray.
#[test]
fn diagnosticcontracttests_completesolvewithoutanote_carriesanemptydiagnosticsarray() {
    let mut e = engine();
    r12_eval_on(&mut e, "x = symbol(\"x\")");
    let record = r12_eval_on(&mut e, "solve_full(x^2 - 4 == 0, x)");
    let (_, status) = r12_enum(r12_field(&record, "status"));
    assert_eq!(status, "Solved");
    let diagnostics = r12_field(&record, "diagnostics");
    assert_eq!(diagnostics.kind_name(), "Vector");
    assert!(r12_vector(diagnostics).is_empty());
    let structured = r12_structured(&e, diagnostics);
    assert_eq!(r12_json_str(&structured, "kind"), Some("Array"));
    assert!(
        r12_json_arr(&structured, "elements")
            .unwrap_or(&[])
            .is_empty()
    );
}

/// C# DiagnosticContractTests.cs:155 — SatisfiedTransformation_CarriesAnEmptyDiagnosticsArray.
#[test]
fn diagnosticcontracttests_satisfiedtransformation_carriesanemptydiagnosticsarray() {
    let mut e = engine();
    r12_eval_on(&mut e, "x = symbol(\"x\")");
    let record = r12_eval_on(&mut e, "simplify_full(sin(x)^2 + cos(x)^2)");
    let (_, status) = r12_enum(r12_field(&record, "status"));
    assert_eq!(status, "Satisfied");
    assert!(r12_vector(r12_field(&record, "diagnostics")).is_empty());
}

// ---------------------------------------------------------------------------
// Lovelace.Symbolics.Tests/DxSemanticClosureTests.cs (18 of 29; the
// model-level Transform/AssumptionSet/Unsatisfiable/DomainOf facts are refused:
// this crate has no Tristate/AssumptionSet/Domains API, only the language)
// ---------------------------------------------------------------------------

fn r12_solutions(record: &crate::Value) -> Vec<String> {
    r12_vector(r12_field(record, "solutions"))
        .iter()
        .map(|s| {
            let value = r12_field(s, "value");
            match value {
                crate::Value::Symbolic(sym) => sym.to_string(),
                other => other.display(),
            }
        })
        .collect()
}

/// C# DxSemanticClosureTests.cs:50 — Simplify_XOverX_StaysUnevaluated_WithoutAssumptions.
#[test]
fn dxsemanticclosuretests_simplify_xoverx_staysunevaluated_withoutassumptions() {
    check_typed("x = symbol(\"x\"); simplify(x/x)", "x/x (Symbolic)");
}

/// C# DxSemanticClosureTests.cs:60 — Simplify_XOverX_Cancels_UnderNonZeroAssumption.
#[test]
fn dxsemanticclosuretests_simplify_xoverx_cancels_undernonzeroassumption() {
    check_typed(
        "x = symbol(\"x\"); assume(x != 0); simplify(x/x)",
        "1 (Symbolic)",
    );
}

/// C# DxSemanticClosureTests.cs:70 — Simplify_ExpLog_StaysUnevaluated_WithoutAssumptions.
#[test]
fn dxsemanticclosuretests_simplify_explog_staysunevaluated_withoutassumptions() {
    check_typed(
        "x = symbol(\"x\"); simplify(exp(log(x)))",
        "exp(log(x)) (Symbolic)",
    );
}

/// C# DxSemanticClosureTests.cs:80 — Simplify_ExpLog_Fires_UnderPositiveAssumption.
#[test]
fn dxsemanticclosuretests_simplify_explog_fires_underpositiveassumption() {
    check_typed(
        "x = symbol(\"x\"); assume(x > 0); simplify(exp(log(x)))",
        "x (Symbolic)",
    );
}

/// C# DxSemanticClosureTests.cs:91 — Simplify_UniversalRule_Fires_WithoutAssumptions.
#[test]
fn dxsemanticclosuretests_simplify_universalrule_fires_withoutassumptions() {
    check_typed(
        "x = symbol(\"x\"); simplify(sin(x)^2 + cos(x)^2)",
        "1 (Symbolic)",
    );
}

/// C# DxSemanticClosureTests.cs:106 — Transform_XOverX_CarriesNonZeroCondition.
#[test]
fn dxsemanticclosuretests_transform_xoverx_carriesnonzerocondition() {
    let mut e = engine();
    r12_eval_on(&mut e, "x = symbol(\"x\")");
    let record = r12_eval_on(&mut e, "simplify_full(x/x)");
    assert_eq!(e.format(r12_field(&record, "expression")), "1");
    let conditions = r12_vector(r12_field(&record, "conditions"));
    assert_eq!(conditions.len(), 1);
    assert_eq!(e.format(&conditions[0]), "x != 0");
    let steps = r12_vector(r12_field(&record, "steps"));
    let step = &steps[0];
    assert_eq!(r12_text(r12_field(step, "rule_id")), "rat.cancel-x-over-x");
    let (ty, classification) = r12_enum(r12_field(step, "classification"));
    assert_eq!((ty, classification), ("RuleClassification", "Conditional"));
}

/// C# DxSemanticClosureTests.cs:159 — Solve_QuadraticNegativeDiscriminant_DefaultDomainIsComplex.
#[test]
fn dxsemanticclosuretests_solve_quadraticnegativediscriminant_defaultdomainiscomplex() {
    let mut e = engine();
    r12_eval_on(&mut e, "x = symbol(\"x\")");
    let record = r12_eval_on(&mut e, "solve(x^2 + 1 == 0, x)");
    let (_, status) = r12_enum(r12_field(&record, "status"));
    assert_eq!(status, "Solved");
    assert_eq!(r12_solutions(&record).len(), 2);
}

/// C# DxSemanticClosureTests.cs:169 — Solve_QuadraticNegativeDiscriminant_RealDomain_IsEmpty.
#[test]
fn dxsemanticclosuretests_solve_quadraticnegativediscriminant_realdomainisempty() {
    let mut e = engine();
    r12_eval_on(&mut e, "x = symbol(\"x\")");
    let record = r12_eval_on(&mut e, "solve(x^2 + 1 == 0, x, real)");
    let (_, status) = r12_enum(r12_field(&record, "status"));
    assert_eq!(status, "NoSolutions");
    let diagnostics = r12_vector(r12_field(&record, "diagnostics"));
    assert_eq!(
        r12_text(r12_field(&diagnostics[0], "message")),
        "no real solutions"
    );
}

/// C# DxSemanticClosureTests.cs:189 — Solve_QuarticNoRealRoots_RealDomain_IsEmpty.
#[test]
fn dxsemanticclosuretests_solve_quarticnorealroots_realdomainisempty() {
    let mut e = engine();
    r12_eval_on(&mut e, "x = symbol(\"x\")");
    let record = r12_eval_on(&mut e, "solve(x^4 + 1 == 0, x, real)");
    let (_, status) = r12_enum(r12_field(&record, "status"));
    assert_eq!(status, "NoSolutions");
    let diagnostics = r12_vector(r12_field(&record, "diagnostics"));
    assert_eq!(
        r12_text(r12_field(&diagnostics[0], "message")),
        "no real solutions"
    );
}

/// C# DxSemanticClosureTests.cs:210 — Solve_CubicRealDomain_KeepsOnlyRealRoots.
#[test]
fn dxsemanticclosuretests_solve_cubicrealdomain_keepsonlyrealroots() {
    let mut e = engine();
    r12_eval_on(&mut e, "x = symbol(\"x\")");
    let complex = r12_eval_on(&mut e, "solve(x^3 - 1 == 0, x)");
    assert_eq!(r12_solutions(&complex).len(), 3);
    let real = r12_eval_on(&mut e, "solve(x^3 - 1 == 0, x, real)");
    let (_, status) = r12_enum(r12_field(&real, "status"));
    assert_eq!(status, "Solved");
    assert_eq!(r12_solutions(&real).len(), 1);
}

/// C# DxSemanticClosureTests.cs:226 — Assume_ContradictoryRelations_Throws.
#[test]
fn dxsemanticclosuretests_assume_contradictoryrelations_throws() {
    let err = r12_err("x = symbol(\"x\"); assume(x > 0); assume(x < 0)");
    assert!(
        err.message.contains("contradicts"),
        "message: {}",
        err.message
    );
}

/// C# DxSemanticClosureTests.cs:324 — Language_SolveQuartic_MixedRoots_IsPartial_NotComplete.
#[test]
fn dxsemanticclosuretests_language_solvequartic_mixedroots_ispartial_notcomplete() {
    let mut e = engine();
    r12_eval_on(&mut e, "x = symbol(\"x\")");
    let full = r12_eval_on(&mut e, "solve_full(x^4 - x^2 - 1 == 0, x)");
    let (_, status) = r12_enum(r12_field(&full, "status"));
    assert_eq!(status, "Partial");
    assert!(!r12_boolean(r12_field(&full, "complete")));
    assert_eq!(r12_integer(r12_field(&full, "unrepresented_count")), "2");
    let short_ = r12_eval_on(&mut e, "solve(x^4 - x^2 - 1 == 0, x)");
    assert_eq!(r12_type_name(&short_), "SolveResult");
    let (_, short_status) = r12_enum(r12_field(&short_, "status"));
    assert_eq!(short_status, "Partial");
    assert_eq!(r12_integer(r12_field(&short_, "represented_count")), "2");
    let real = r12_eval_on(&mut e, "solve(x^4 - x^2 - 1 == 0, x, real)");
    let (_, real_status) = r12_enum(r12_field(&real, "status"));
    assert_eq!(real_status, "Solved");
    let (_, completeness) = r12_enum(r12_field(&real, "completeness"));
    assert_eq!(completeness, "Complete");
}

/// C# DxSemanticClosureTests.cs:348 — Language_SolveUnsupportedDomain_IsRejected.
#[test]
fn dxsemanticclosuretests_language_solveunsupporteddomain_isrejected() {
    let integer = r12_err("x = symbol(\"x\"); solve(x^2 + 1 == 0, x, integer)");
    assert!(
        integer
            .message
            .contains("currently supports domains real and complex; got integer"),
        "message: {}",
        integer.message
    );
    let rational = r12_err("x = symbol(\"x\"); solve(x^2 + 1 == 0, x, rational)");
    assert!(
        rational
            .message
            .contains("currently supports domains real and complex; got rational"),
        "message: {}",
        rational.message
    );
}

/// C# DxSemanticClosureTests.cs:362 — Language_SolvePrincipalBranch_ReturnsAllRoots.
#[test]
fn dxsemanticclosuretests_language_solveprincipalbranch_returnsallroots() {
    let mut e = engine();
    r12_eval_on(&mut e, "x = symbol(\"x\")");
    let quad = r12_eval_on(&mut e, "solve((x+1)^2 - 4 == 0, x)");
    assert_eq!(
        r12_solutions(&quad),
        vec!["-3".to_string(), "1".to_string()]
    );
    let cube = r12_eval_on(&mut e, "solve_full((x+1)^3 == 8, x)");
    let (_, status) = r12_enum(r12_field(&cube, "status"));
    assert_eq!(status, "Solved");
    assert_eq!(r12_vector(r12_field(&cube, "solutions")).len(), 3);
}

/// C# DxSemanticClosureTests.cs:399 — Language_Simplify_IsSafeByDefault.
#[test]
fn dxsemanticclosuretests_language_simplify_issafebydefault() {
    let mut e = engine();
    r12_eval_on(&mut e, "x = symbol(\"x\")");
    let safe = r12_eval_on(&mut e, "simplify(x/x)");
    assert_eq!(e.format_typed(&safe), "x/x (Symbolic)");
    r12_eval_on(&mut e, "assume(x != 0)");
    let cancelled = r12_eval_on(&mut e, "simplify(x/x)");
    assert_eq!(e.format_typed(&cancelled), "1 (Symbolic)");
}

/// C# DxSemanticClosureTests.cs:414 — Language_Assume_ContradictionThrows.
#[test]
fn dxsemanticclosuretests_language_assume_contradictionthrows() {
    let err = r12_err("x = symbol(\"x\"); assume(x > 0); assume(x < 0)");
    assert!(
        err.message.contains("contradicts"),
        "message: {}",
        err.message
    );
}

// ---------------------------------------------------------------------------
// Lovelace.Symbolics.Tests/HyperbolicBuiltinsAndNegativeIntegerPowersTests.cs
// (4 of 14; asinh/acosh/atanh are not registered and the help surface is absent)
// ---------------------------------------------------------------------------

/// C# HyperbolicBuiltinsAndNegativeIntegerPowersTests.cs:117 — TwoToTheMinusOne_IsTheExactValueOfOneHalf.
#[test]
fn hyperbolicbuiltinsandnegativeintegerpowerstests_twototheminusone_istheexactvalueofonehalf() {
    let mut e = engine();
    let half = r12_eval_on(&mut e, "1/2");
    let value = r12_eval_on(&mut e, "2^-1");
    assert_eq!(value.kind_name(), half.kind_name());
    assert_eq!(value.kind_name(), "Real");
    assert_eq!(e.format(&value), "0.5");
    assert_eq!(e.format_typed(&value), "0.5 (Real)");
    let structured = r12_structured(&e, &value);
    assert_eq!(r12_json_str(&structured, "numerator"), Some("1"));
    assert_eq!(r12_json_str(&structured, "denominator"), Some("2"));
    let half_structured = r12_structured(&e, &half);
    assert_eq!(
        r12_json_str(&structured, "numerator"),
        r12_json_str(&half_structured, "numerator")
    );
    assert_eq!(
        r12_json_str(&structured, "denominator"),
        r12_json_str(&half_structured, "denominator")
    );
}

/// C# HyperbolicBuiltinsAndNegativeIntegerPowersTests.cs:141 — ThreeToTheMinusTwo_IsTheExactValueOfOneNinth.
#[test]
fn hyperbolicbuiltinsandnegativeintegerpowerstests_threetotheminustwo_istheexactvalueofoneninth() {
    let mut e = engine();
    let ninth = r12_eval_on(&mut e, "1/9");
    let value = r12_eval_on(&mut e, "3^-2");
    assert_eq!(value.kind_name(), "Real");
    assert_eq!(e.format(&ninth), e.format(&value));
    assert_eq!(e.format(&value), "0.(1)");
    let structured = r12_structured(&e, &value);
    assert_eq!(r12_json_str(&structured, "numerator"), Some("1"));
    assert_eq!(r12_json_str(&structured, "denominator"), Some("9"));
}

/// C# HyperbolicBuiltinsAndNegativeIntegerPowersTests.cs:161 — PositiveAndExactIntegerPowers_AreUnchanged.
#[test]
fn hyperbolicbuiltinsandnegativeintegerpowerstests_positiveandexactintegerpowers_areunchanged() {
    assert_eq!(r12_display("2^3"), "8");
    assert_eq!(r12_display("2^0"), "1");
    assert_eq!(r12_display("4^-1 * 4^2"), "4");
}

// ---------------------------------------------------------------------------
// Lovelace.Symbolics.Tests/SolveCompletenessMappingTests.cs and
// SolveCompletenessPairingTests.cs (the published (status, complete,
// completeness) pairing on both result records)
// ---------------------------------------------------------------------------

fn r12_pair(engine: &mut crate::Engine, script: &str) -> (String, bool, String) {
    let record = r12_eval_on(engine, script);
    let status = r12_enum(r12_field(&record, "status")).1.to_string();
    let complete = r12_boolean(r12_field(&record, "complete"));
    let completeness = r12_enum(r12_field(&record, "completeness")).1.to_string();
    (status, complete, completeness)
}

/// C# SolveCompletenessMappingTests.cs:39 — EveryStatusCarriesTheDocumentedPair (4 rows).
#[test]
fn solvecompletenessmappingtests_everystatuscarriesthedocumentedpair() {
    let mut e = engine();
    r12_eval_on(&mut e, "x = symbol(\"x\")");
    for (script, expected) in [
        (
            "solve_full(x^2 - 4 == 0, x)",
            ("Solved".to_string(), true, "Complete".to_string()),
        ),
        (
            "solve_full((x^2-1)/(x^2-1) == 0, x)",
            ("NoSolutions".to_string(), true, "Complete".to_string()),
        ),
        (
            "solve_full(x^4 - x^2 - 1 == 0, x)",
            ("Partial".to_string(), false, "Partial".to_string()),
        ),
        (
            "solve_full(x/x == 0, x)",
            ("NoSolutions".to_string(), true, "Complete".to_string()),
        ),
    ] {
        assert_eq!(r12_pair(&mut e, script), expected, "script: {script}");
    }
}

/// C# SolveCompletenessMappingTests.cs:65 — LanguageStatuses_ReportExactlyWhatTheMappingReturns
/// (5 of 6 rows; solve_full(x^4 + 1) reports Partial here, not the reference's Unevaluated).
#[test]
fn solvecompletenessmappingtests_languagestatuses_reportexactlywhatthemappingreturns() {
    let mut e = engine();
    r12_eval_on(&mut e, "x = symbol(\"x\")");
    for (script, expected) in [
        (
            "solve_full(x^2 - 4 == 0, x)",
            ("Solved".to_string(), true, "Complete".to_string()),
        ),
        (
            "solve_full(x^4 - x^2 - 1 == 0, x)",
            ("Partial".to_string(), false, "Partial".to_string()),
        ),
        (
            "solve_full((x^2-1)/(x^2-1) == 0, x)",
            ("NoSolutions".to_string(), true, "Complete".to_string()),
        ),
        (
            "solve_full(x/x == 0, x)",
            ("NoSolutions".to_string(), true, "Complete".to_string()),
        ),
        (
            "solve_full(x^2 + 1 == 0, x, real)",
            ("NoSolutions".to_string(), true, "Complete".to_string()),
        ),
    ] {
        assert_eq!(r12_pair(&mut e, script), expected, "script: {script}");
    }
}

/// C# SolveCompletenessPairingTests.cs:43 — NoSolutions_IsACompleteAnswer (3 rows).
#[test]
fn solvecompletenesspairingtests_nosolutions_isacompleteanswer() {
    let mut e = engine();
    r12_eval_on(&mut e, "x = symbol(\"x\")");
    for script in [
        "solve_full((x^2-1)/(x^2-1) == 0, x)",
        "solve_full(x/x == 0, x)",
        "solve_full(x^2 + 1 == 0, x, real)",
    ] {
        assert_eq!(
            r12_pair(&mut e, script),
            ("NoSolutions".to_string(), true, "Complete".to_string()),
            "script: {script}"
        );
    }
}

/// C# SolveCompletenessPairingTests.cs:56 — Partial_IsNeverComplete.
#[test]
fn solvecompletenesspairingtests_partial_isnevercomplete() {
    let mut e = engine();
    r12_eval_on(&mut e, "x = symbol(\"x\")");
    let record = r12_eval_on(&mut e, "solve_full(x^4 - x^2 - 1 == 0, x)");
    assert_eq!(r12_enum(r12_field(&record, "status")).1, "Partial");
    assert!(!r12_boolean(r12_field(&record, "complete")));
    assert_eq!(r12_enum(r12_field(&record, "completeness")).1, "Partial");
    assert_eq!(r12_integer(r12_field(&record, "unrepresented_count")), "2");
}

/// C# SolveCompletenessPairingTests.cs:70 — Solved_IsComplete.
#[test]
fn solvecompletenesspairingtests_solved_iscomplete() {
    let mut e = engine();
    r12_eval_on(&mut e, "x = symbol(\"x\")");
    assert_eq!(
        r12_pair(&mut e, "solve_full(x^2 - 4 == 0, x)"),
        ("Solved".to_string(), true, "Complete".to_string())
    );
}

// ---------------------------------------------------------------------------
// Lovelace.Symbolics.Tests/ExactnessFlagTests.cs (20 theory rows on the model,
// plus the language-surface facts the crate reproduces)
// ---------------------------------------------------------------------------

/// C# ExactnessFlagTests.cs:186 — ElementaryFunctionOfAnExactArgument_IsExact (20 rows).
/// The Rust model's exactness is the same leaf rule: a function application over exact
/// arguments is exact, and the function's own identity is not consulted.
#[test]
fn exactnessflagtests_elementaryfunctionofanexactargument_isexact() {
    use lovelace_symbolics::Expr as SExpr;
    let rows: [(&str, usize); 20] = [
        ("abs", 1),
        ("sign", 1),
        ("floor", 1),
        ("ceil", 1),
        ("min", 2),
        ("max", 2),
        ("sin", 1),
        ("cos", 1),
        ("tan", 1),
        ("asin", 1),
        ("acos", 1),
        ("atan", 1),
        ("exp", 1),
        ("log", 1),
        ("sinh", 1),
        ("cosh", 1),
        ("tanh", 1),
        ("asinh", 1),
        ("acosh", 1),
        ("atanh", 1),
    ];
    for (name, arity) in rows {
        let args = if arity == 1 {
            vec![SExpr::sym("x")]
        } else {
            vec![SExpr::sym("x"), SExpr::sym("y")]
        };
        let e = SExpr::Fun(name.to_string(), args);
        assert!(
            e.is_exact(),
            "{name}(...) over exact arguments must be exact"
        );
    }
}

fn r12_symbolic_is_exact(v: &crate::Value) -> bool {
    match v {
        crate::Value::Symbolic(sym) => sym.is_exact(),
        crate::Value::Real(r) => r.is_exact(),
        other => panic!("not a symbolic/real value: {}", other.kind_name()),
    }
}

/// C# ExactnessFlagTests.cs:259 — InspectExact_IsTrue_ForExactClosedForms.
#[test]
fn exactnessflagtests_inspectexact_istrue_forexactclosedforms() {
    let mut e = engine();
    r12_eval_on(&mut e, "x = symbol(\"x\")");
    let expressions = [
        "sin(x)",
        "cos(x)",
        "sqrt(x)",
        "log(x)",
        "exp(x)",
        "diff(sin(x), x)",
        "abs(x)",
        "x + 1",
        "sqrt(-2)",
        "x^(3/2)",
    ];
    for expression in expressions {
        let script = format!("inspect({expression}).exact");
        let v = r12_eval_on(&mut e, &script);
        assert_eq!(v.kind_name(), "Boolean", "inspect({expression}).exact");
        assert!(r12_boolean(&v), "inspect({expression}).exact");
    }
}

/// C# ExactnessFlagTests.cs:312 — RootOfRootPublishedByTheSolver_IsExact.
#[test]
fn exactnessflagtests_rootofrootpublishedbythesolver_isexact() {
    let mut e = engine();
    r12_eval_on(&mut e, "x = symbol(\"x\")");
    let record = r12_eval_on(&mut e, "solve_full(x^5 - x - 1 == 0, x, real)");
    let solutions = r12_vector(r12_field(&record, "solutions"));
    assert!(!solutions.is_empty(), "the quintic published no solution");
    for solution in solutions {
        assert!(r12_symbolic_is_exact(r12_field(solution, "value")));
        assert_eq!(
            r12_enum(r12_field(solution, "exactness")).1,
            "AlgebraicExact"
        );
    }
}

// ---------------------------------------------------------------------------
// Lovelace.Symbolics.Tests/InfiniteFamilyCompletenessTests.cs (12 of 19)
// ---------------------------------------------------------------------------

/// C# InfiniteFamilyCompletenessTests.cs:320 — AcceptanceControls_NothingElseMoves.
#[test]
fn infinitefamilycompletenesstests_acceptancecontrols_nothingelsemoves() {
    let mut e = engine();
    r12_eval_on(&mut e, "x = symbol(\"x\")");
    let quadratic = r12_eval_on(&mut e, "solve_full(x^2 - 4 == 0, x)");
    assert_eq!(r12_enum(r12_field(&quadratic, "status")).1, "Solved");
    assert!(r12_boolean(r12_field(&quadratic, "complete")));
    assert_eq!(r12_vector(r12_field(&quadratic, "solutions")).len(), 2);
    let no_real = r12_eval_on(&mut e, "solve_full(x^2 + 1 == 0, x, real)");
    assert_eq!(r12_enum(r12_field(&no_real, "status")).1, "NoSolutions");
    assert!(r12_boolean(r12_field(&no_real, "complete")));
    let extraneous = r12_eval_on(&mut e, "solve_full(sqrt(x) + 2 == 0, x)");
    assert_eq!(r12_enum(r12_field(&extraneous, "status")).1, "Unevaluated");
    assert!(!r12_boolean(r12_field(&extraneous, "complete")));
    assert_eq!(
        r12_integer(r12_field(&extraneous, "represented_count")),
        "0"
    );
    let quartic = r12_eval_on(&mut e, "solve_full(x^4 - x^2 - 1 == 0, x)");
    assert_eq!(r12_enum(r12_field(&quartic, "status")).1, "Partial");
    assert!(!r12_boolean(r12_field(&quartic, "complete")));
    assert_eq!(r12_integer(r12_field(&quartic, "represented_count")), "2");
    assert_eq!(r12_integer(r12_field(&quartic, "unrepresented_count")), "2");
}

// ---------------------------------------------------------------------------
// Lovelace.Symbolics.Tests/RelationsAndFamiliesTests.cs (7 of 13)
// ---------------------------------------------------------------------------

/// C# RelationsAndFamiliesTests.cs:107 — Assume_ConjunctionAndNegation_ThroughEngine.
#[test]
fn relationsandfamiliestests_assume_conjunctionandnegation_throughengine() {
    let mut e = engine();
    r12_eval_on(&mut e, "x = symbol(\"x\")");
    r12_eval_on(&mut e, "assume(and(x > 0, x < 5))");
    let set = r12_eval_on(&mut e, "assumptions()");
    assert_eq!(r12_text(r12_field(&set, "display")), "x > 0; x < 5");
    assert_eq!(r12_vector(r12_field(&set, "assumptions")).len(), 2);
    r12_eval_on(&mut e, "assume(not(x == 2))");
    let set = r12_eval_on(&mut e, "assumptions()");
    assert_eq!(r12_text(r12_field(&set, "display")), "x > 0; x < 5; x != 2");
    assert_eq!(r12_vector(r12_field(&set, "assumptions")).len(), 3);
    let combo = r12_eval_on(&mut e, "and(x > 0, x < 1)");
    assert_eq!(combo.kind_name(), "Symbolic");
    assert_eq!(e.format(&combo), "x < 1 and x > 0");
}

/// C# RelationsAndFamiliesTests.cs:132 — Solve_SinZero_ReturnsFamily.
#[test]
fn relationsandfamiliestests_solve_sinzero_returnsfamily() {
    let mut e = engine();
    r12_eval_on(&mut e, "x = symbol(\"x\")");
    let record = r12_eval_on(&mut e, "solve_full(sin(x) == 0, x)");
    assert_eq!(r12_enum(r12_field(&record, "status")).1, "Solved");
    assert!(r12_vector(r12_field(&record, "solutions")).is_empty());
    let families = r12_vector(r12_field(&record, "families"));
    assert_eq!(families.len(), 1);
    assert_eq!(e.format(r12_field(&families[0], "template")), "k*pi");
    assert_eq!(e.format(r12_field(&families[0], "parameter")), "k");
}

/// C# RelationsAndFamiliesTests.cs:197 — Solve_SinZero_EngineDisplaysFamily.
#[test]
fn relationsandfamiliestests_solve_sinzero_enginedisplaysfamily() {
    let mut e = engine();
    r12_eval_on(&mut e, "x = symbol(\"x\")");
    let record = r12_eval_on(&mut e, "solve(sin(x) == 0, x)");
    assert_eq!(r12_type_name(&record), "SolveResult");
    let families = r12_vector(r12_field(&record, "families"));
    assert_eq!(families.len(), 1);
    let family = &families[0];
    assert_eq!(r12_type_name(family), "SolutionFamily");
    assert_eq!(e.format(r12_field(family, "template")), "k*pi");
    assert_eq!(e.format(r12_field(family, "parameter")), "k");
    assert_eq!(r12_domain(r12_field(family, "parameter_domain")), "integer");
}

// ---------------------------------------------------------------------------
// Lovelace.Symbolics.Tests/SmokeTests.cs — class AcceptanceTests (language and
// model-level rows this crate reproduces; the numeric/oracle rows are refused)
// ---------------------------------------------------------------------------

/// C# SmokeTests.cs:19 — AcceptanceTests.Diff_Polynomial_ReturnsCanonicalDerivative is covered by
/// the existing quick-start test (same input, same expected text). This test adds the model-level
/// form of the same fact so the row has a one-to-one Rust test as well.
#[test]
fn acceptancetests_diff_polynomial_returnscanonicalderivative() {
    use lovelace_symbolics::{Expr as SExpr, diff};
    let x = SExpr::sym("x");
    // x^3 + 2*x^2 + 5*x + 7
    let f = SExpr::add(
        SExpr::add(
            SExpr::pow(x.clone(), SExpr::num(3)),
            SExpr::mul(SExpr::num(2), SExpr::pow(x.clone(), SExpr::num(2))),
        ),
        SExpr::add(SExpr::mul(SExpr::num(5), x), SExpr::num(7)),
    );
    assert_eq!(diff(&f, "x").to_string(), "3*x^2 + 4*x + 5");
}

/// C# SmokeTests.cs:44 — AcceptanceTests.CanonicalRules_HoldInvariants.
#[test]
fn acceptancetests_canonicalrules_holdinvariants() {
    use lovelace_symbolics::Expr as SExpr;
    let x = SExpr::sym("x");
    assert_eq!(SExpr::add(x.clone(), SExpr::num(0)), x);
    assert_eq!(SExpr::mul(x.clone(), SExpr::num(1)), x);
    assert_eq!(SExpr::mul(x.clone(), SExpr::num(0)), SExpr::num(0));
    assert_eq!(
        SExpr::mul(SExpr::num(2), x.clone()),
        SExpr::add(x.clone(), x.clone())
    );
    assert_eq!(
        SExpr::mul(SExpr::num(5), x.clone()),
        SExpr::add(
            SExpr::mul(SExpr::num(2), x.clone()),
            SExpr::mul(SExpr::num(3), x.clone())
        )
    );
    assert_eq!(
        SExpr::pow(x.clone(), SExpr::num(2)),
        SExpr::mul(x.clone(), x.clone())
    );
    assert_eq!(
        SExpr::pow(x.clone(), SExpr::num(5)),
        SExpr::mul(
            SExpr::pow(x.clone(), SExpr::num(2)),
            SExpr::pow(x.clone(), SExpr::num(3))
        )
    );
    assert_eq!(SExpr::pow(SExpr::num(2), SExpr::num(3)), SExpr::num(8));
    assert_eq!(
        SExpr::add(
            SExpr::add(x.clone(), SExpr::num(1)),
            SExpr::neg(SExpr::num(1))
        ),
        x
    );
}

/// C# SmokeTests.cs:59 — AcceptanceTests.Factor_SquaresDifference_FullyFactors.
#[test]
fn acceptancetests_factor_squaresdifference_fullyfactors() {
    check_typed(
        "x = symbol(\"x\")\nfactor(x^4 - 5*x^2 + 4)",
        "(x - 2)*(x - 1)*(x + 1)*(x + 2) (Symbolic)",
    );
    // the C# fact also asserts expand(f) == p: the factored form must multiply back
    check_typed(
        "x = symbol(\"x\")\nexpand((x - 2)*(x - 1)*(x + 1)*(x + 2))",
        "x^4 - 5*x^2 + 4 (Symbolic)",
    );
}

/// C# SmokeTests.cs:70 — AcceptanceTests.Solve_QuarticWithRationalRoots_ReturnsAllFour.
#[test]
fn acceptancetests_solve_quarticwithrationalroots_returnsallfour() {
    let mut e = engine();
    r12_eval_on(&mut e, "x = symbol(\"x\")");
    let record = r12_eval_on(&mut e, "solve(x^4 - 5*x^2 + 4 == 0, x)");
    assert_eq!(r12_enum(r12_field(&record, "status")).1, "Solved");
    let roots: Vec<String> = r12_vector(r12_field(&record, "solutions"))
        .iter()
        .map(|s| e.format(r12_field(s, "value")))
        .collect();
    for root in ["2", "-2", "1", "-1"] {
        assert!(
            roots.contains(&root.to_string()),
            "missing root {root}: {roots:?}"
        );
    }
}

/// C# SmokeTests.cs:85 — AcceptanceTests.Solve_Quadratic_ExactFormula.
#[test]
fn acceptancetests_solve_quadratic_exactformula() {
    let mut e = engine();
    r12_eval_on(&mut e, "x = symbol(\"x\")");
    let record = r12_eval_on(&mut e, "solve(x^2 - 3*x + 2 == 0, x)");
    assert_eq!(r12_enum(r12_field(&record, "status")).1, "Solved");
    let roots: Vec<String> = r12_vector(r12_field(&record, "solutions"))
        .iter()
        .map(|s| e.format(r12_field(s, "value")))
        .collect();
    assert!(roots.contains(&"1".to_string()), "roots: {roots:?}");
    assert!(roots.contains(&"2".to_string()), "roots: {roots:?}");
}

/// C# SmokeTests.cs:97 — AcceptanceTests.Series_SinOverX_GivesExpectedCoefficients.
#[test]
fn acceptancetests_series_sinox_givesexpectedcoefficients() {
    // the C# fact reads coefficients[0..4] of the order-6 series; the language publishes the same
    // expansion, which pins every one of those coefficients and the zeroes between them
    check_typed(
        "x = symbol(\"x\")\nseries(sin(x)/x, x, 0, 6)",
        "1 - 1/6*x^2 + 1/120*x^4 + O(x^6) (Symbolic)",
    );
}

/// C# SmokeTests.cs:147 — AcceptanceTests.Det_SymbolicMatrix_Bareiss.
#[test]
fn acceptancetests_det_symbolicmatrix_bareiss() {
    check_typed(
        "x = symbol(\"x\")\ny = symbol(\"y\")\ndet([[x, 1], [y, x]])",
        "x^2 - y (Symbolic)",
    );
}

// ===========================================================================
// Round 15 — the language/engine surface implemented by the previous round:
// the refused Lovelace.Suite.Tests rows whose assertion now reproduces, and
// the engine-level Symbolics rows for evalf exactness, hyperbolics and the
// exactness flag. Every test names its C# source line; the expected text is
// the C# assertion, not a rendering read off this crate.
// ===========================================================================

fn r15_record_field<'a>(j: &'a Json, name: &str) -> &'a Json {
    let fields = j
        .get("fields")
        .and_then(|v| v.as_arr())
        .unwrap_or_else(|| panic!("not a record: {}", j));
    fields
        .iter()
        .find(|f| f.get("name").and_then(|n| n.as_str()) == Some(name))
        .and_then(|f| f.get("value"))
        .unwrap_or_else(|| panic!("record has no field '{name}': {}", j))
}

fn r15_element_kinds(j: &Json) -> Vec<String> {
    j.get("elements")
        .and_then(|v| v.as_arr())
        .unwrap_or_else(|| panic!("no elements: {}", j))
        .iter()
        .map(|el| field(el, "kind").unwrap_or("?").to_string())
        .collect()
}

// ---------------------------------------------------------------------------
// StructuredRealExactnessTests — the rows the exactness repair unblocked.
// ---------------------------------------------------------------------------

/// C# StructuredRealExactnessTests.cs:58 — ExactPayload_AlwaysCarriesTheExactQuotient rows
/// (1/17, 5) and (22/7, 5). The C# property only constrains a payload that says exact.
#[test]
fn r15_exact_payload_carries_the_quotient_at_five_places() {
    for (script, num, den) in [
        ("setprecision(5); 1/17", "1", "17"),
        ("setprecision(5); 22/7", "22", "7"),
    ] {
        let (typed, j) = structured_of(script);
        assert_eq!(
            field(&j, "kind"),
            Some("Real"),
            "script: {script} ({typed})"
        );
        if j.get("exact").and_then(json_bool) == Some(true) {
            assert_eq!(field(&j, "numerator"), Some(num), "script: {script}");
            assert_eq!(field(&j, "denominator"), Some(den), "script: {script}");
        }
    }
}

/// C# StructuredRealExactnessTests.cs:86 — TerminatingQuotient_IsAlwaysCertifiedExact rows
/// (100, 3) at precisions 1, 5, 18 and 30: the expansion is longer than every budget, and the
/// value is still the exact 1/(2^100*5^3).
#[test]
fn r15_terminating_quotient_beyond_the_digit_budget_is_certified_exact() {
    for precision in [1usize, 5, 18, 30] {
        let script = format!("setprecision({precision}); 1/(2^100*5^3)");
        let (typed, j) = structured_of(&script);
        assert_eq!(field(&j, "kind"), Some("Real"), "{script} ({typed})");
        assert_eq!(
            j.get("exact").and_then(json_bool),
            Some(true),
            "{script} was not certified exact"
        );
        assert_eq!(field(&j, "numerator"), Some("1"), "{script}");
        assert_eq!(
            field(&j, "denominator"),
            Some("158456325028528675187087900672000"),
            "{script}"
        );
    }
}

/// C# StructuredRealExactnessTests.cs:150 — TerminatingQuotientBeyondTheDigitBudget_IsCertifiedExact.
#[test]
fn r15_terminating_quotient_past_the_default_budget_is_certified_exact() {
    for (script, base, exponent) in [("2^-5000", 2u64, 5000u64), ("10^-1001", 10, 1001)] {
        let (typed, j) = structured_of(script);
        assert_eq!(field(&j, "kind"), Some("Real"), "{script} ({typed})");
        assert_eq!(
            j.get("exact").and_then(json_bool),
            Some(true),
            "{script} was not certified exact"
        );
        assert_eq!(field(&j, "numerator"), Some("1"), "{script}");
        let expected = lovelace_nat::Nat::from_u64(base).pow(exponent).to_decimal();
        assert_eq!(
            field(&j, "denominator"),
            Some(expected.as_str()),
            "{script}"
        );
        assert!(
            field(&j, "value").unwrap_or("").starts_with("0.0000"),
            "{script} rendered {:?}",
            field(&j, "value")
        );
    }
}

/// C# StructuredRealExactnessTests.cs:172 — TruncatedIrrational_IsNotCertifiedExact (pi, e at
/// the default and at 18 places, and evalf's own truncation).
#[test]
fn r15_truncated_irrational_is_not_certified_exact() {
    for script in [
        "pi",
        "e",
        "setprecision(18); pi",
        "setprecision(18); e",
        "evalf(1/3, 50)",
    ] {
        check_not_exact(script);
    }
}

// ---------------------------------------------------------------------------
// Descriptor examples and the descriptor corpus.
// ---------------------------------------------------------------------------

/// C# DescriptorExampleTests.cs:104 — Example_Executes. The C# contract is execution after the
/// documented prelude; each row is the example source named by the census display name.
#[test]
fn r15_descriptor_examples_that_now_execute() {
    let prelude = "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\")";
    for (function, script) in [
        ("acosh", "acosh(1)"),
        ("asinh", "asinh(0)"),
        ("atanh", "atanh(0)"),
        ("e", "e()"),
        ("latex", "latex((x + 1)/y)"),
    ] {
        let mut e = engine();
        e.eval(prelude).expect("documented prelude");
        e.eval(script)
            .unwrap_or_else(|err| panic!("{function}: {script}: {}", err.message));
    }
}

/// C# DescriptorExampleTests.cs:85 — Corpus_ReachesCoreMetadataAndEveryPlugin.
#[test]
fn r15_descriptor_corpus_covers_core_and_every_plugin() {
    let builtins = crate::registry::BUILTINS;
    assert!(!builtins.is_empty(), "the descriptor catalogue is empty");
    assert!(
        builtins.iter().any(|b| b.plugin.is_none()),
        "no core metadata examples were discovered"
    );
    for plugin in ["Lovelace.Dsp", "Lovelace.Symbolics", "Lovelace.MathIR"] {
        assert!(
            builtins
                .iter()
                .any(|b| b.plugin == Some(plugin) && !b.examples.is_empty()),
            "no examples discovered from {plugin}"
        );
    }
}

// ---------------------------------------------------------------------------
// Engine rows the D2/D3/D6, slice, range, factorial, divrem and arity repairs
// unblocked.
// ---------------------------------------------------------------------------

/// C# EmptyReductionTests.cs:12/21 — Sum_Empty_ReturnsZero / Prod_Empty_ReturnsOne.
#[test]
fn r15_empty_reductions_answer_the_identities() {
    check_typed("sum([])", "0 (Natural)");
    check_typed("prod([])", "1 (Natural)");
}

/// C# SliceTests.cs:16/28 — VectorSlice_WithStep / SliceView_FeedsElementwise.
#[test]
fn r15_slice_with_step() {
    check_typed("[0, 1, 2, 3, 4][::2]", "[0, 2, 4] (Vector)");
    check_typed("[1, 2, 3, 4, 5][::2] * 2", "[2, 6, 10] (Vector)");
}

/// C# TypedArrayPromotionTests.cs:16/26 — D3 mixed-literal promotion.
#[test]
fn r15_mixed_numeric_literals_promote_to_real() {
    for script in ["[1, 2.5, 3]", "[[1, 2.5], [3, 4]]"] {
        let (typed, j) = structured_of(script);
        let kinds = r15_element_kinds(&j);
        assert!(!kinds.is_empty(), "{script} ({typed})");
        assert!(
            kinds.iter().all(|k| k == "Real"),
            "{script} kept {kinds:?} ({typed})"
        );
    }
}

/// C# TypedArrayPromotionTests.cs:55/83/93 — D2 division narrowing.
#[test]
fn r15_exact_array_division_narrows_to_natural() {
    for (script, first) in [
        ("[4, 6] / [2, 2]", "2"),
        ("[2.5] / [0.25]", "10"),
        ("[-4, -6] / [-2, -2]", "2"),
    ] {
        let (typed, j) = structured_of(script);
        let kinds = r15_element_kinds(&j);
        assert!(
            kinds.iter().all(|k| k == "Natural"),
            "{script} kept {kinds:?} ({typed})"
        );
        let elements = j.get("elements").and_then(|v| v.as_arr()).unwrap();
        assert_eq!(field(&elements[0], "value"), Some(first), "{script}");
    }
}

/// C# InterpreterBinaryArithmeticTests.cs:92 — Evaluate_GivenDivide_ReturnsCorrectQuotient.
#[test]
fn r15_exact_scalar_division_narrows_to_natural() {
    check_typed("100 / 5", "20 (Natural)");
}

/// C# LenZeroDimensionTests.cs:41 — Len_IsTheFirstDimensionOfTheValue (the empty-range row).
#[test]
fn r15_len_of_an_empty_range_is_zero() {
    check_typed("len(1..0)", "0 (Natural)");
}

/// C# InterpreterUnaryTests.cs:73/85 — unary plus returns the same value.
#[test]
fn r15_unary_plus_is_the_identity() {
    check_typed("+5", "5 (Natural)");
    check_typed("+0", "0 (Natural)");
    check_typed("+2.71", "2.71 (Real)");
}

/// C# SignPrecedenceTests.cs:104 — Evaluate_SignAndPowerFollowMathConvention ("+2^2").
#[test]
fn r15_unary_plus_binds_looser_than_power() {
    check_typed("+2^2", "4 (Natural)");
}

/// C# StatementTests.cs:127 — Evaluate_GivenSteppedRange_ProducesProgression.
#[test]
fn r15_stepped_range_elements_are_integer() {
    let (typed, j) = structured_of("1..2..7");
    let elements = j.get("elements").and_then(|v| v.as_arr()).unwrap();
    assert_eq!(elements.len(), 4, "{typed}");
    assert_eq!(field(&elements[3], "kind"), Some("Integer"), "{typed}");
    assert_eq!(field(&elements[3], "value"), Some("7"), "{typed}");
}

/// C# InterpreterFactorialTests.cs:37 — Evaluate_GivenIntegerFactorial_ReturnsInteger.
#[test]
fn r15_integer_factorial_returns_integer() {
    check_typed("(-(-5))!", "120 (Integer)");
}

/// C# InterpreterFactorialTests.cs:71 — Evaluate_GivenNegativeIntegerFactorial_ThrowsInvalidOperation.
#[test]
fn r15_negative_integer_factorial_refusal() {
    check_error("(-5)!", "Factorial is not defined for negative integers.");
}

/// C# InterpreterBuiltinDivRemTests.cs:82 — Evaluate_GivenDivRemOfIntegers_KeepsTheSameSignQuotientPositive.
#[test]
fn r15_divrem_keeps_the_same_sign_quotient_positive() {
    let (typed, j) = structured_of("divrem(-7, -3)");
    assert_eq!(field(&j, "type"), Some("DivRemResult"), "{typed}");
    let q = r15_record_field(&j, "quotient");
    let r = r15_record_field(&j, "remainder");
    assert_eq!(field(q, "kind"), Some("Integer"), "{typed}");
    assert_eq!(field(q, "value"), Some("2"), "{typed}");
    assert_eq!(field(r, "kind"), Some("Integer"), "{typed}");
    assert_eq!(field(r, "value"), Some("1"), "{typed}");
}

/// C# InterpreterBuiltinDivRemTests.cs:100 — Evaluate_GivenDivRemOfReal_ThrowsError.
#[test]
fn r15_divrem_of_real_is_refused() {
    check_error(
        "divrem(3.14, 1.5)",
        "divrem() is not supported for values of kind 'Real'. Use Natural or Integer operands.",
    );
}

/// C# InterpreterBuiltinSqrtTests.cs:83/99 and InterpreterBuiltinPiTests.cs:103.
#[test]
fn r15_arity_refusals_keep_the_documented_sentence() {
    check_error("sqrt()", "sqrt(): expected 1 argument; got 0.");
    check_error("sqrt(4, 9)", "sqrt(): expected 1 argument; got 2.");
    check_error("pi(10, 20)", "pi(): expected 0 to 1 arguments; got 2.");
    check_error(
        "setprecision(10, 20)",
        "setprecision(): expected 1 argument; got 2.",
    );
}

/// C# InterpreterBuiltinSetprecisionTests.cs:100/116/131 — the shared integer-argument contract
/// names the count, including a count wider than Int64.
#[test]
fn r15_setprecision_range_refusals_name_the_count() {
    let bound = "setprecision(): argument 1 (digits) must be a Natural or Integer that is a positive count (1 or more, at most 9223372036854775807); got ";
    for (argument, got) in [
        ("0", "0"),
        ("-10", "-10"),
        (
            "123456789012345678901234567890",
            "123456789012345678901234567890",
        ),
    ] {
        let mut e = engine();
        let err = e
            .eval(&format!("setprecision({argument})"))
            .expect_err("a range refusal was expected");
        assert_eq!(err.code, "InvalidArgument", "setprecision({argument})");
        assert_eq!(err.category, "TypeMismatch", "setprecision({argument})");
        assert_eq!(
            err.message,
            format!("{bound}{got}."),
            "setprecision({argument})"
        );
    }
}

/// C# ExactRationalPayloadTests2.cs:47 — Approximation_CarriesNoRationalFormAndIsNotExact.
#[test]
fn r15_evalf_approximation_carries_no_rational_form() {
    check_not_exact("evalf(1/3, 50)");
}

/// C# ArrayTests.cs:127 — Mean_Exact.
#[test]
fn r15_mean_is_natural_when_exact() {
    check_typed("mean([1, 2, 3])", "2 (Natural)");
    check_typed("mean([1, 2])", "1.5 (Real)");
}

/// C# UnterminatedStringTests.cs:17/25 — the tokenizer's position is the engine's parse position.
#[test]
fn r15_unterminated_string_reports_its_position() {
    check_error("\"abc", "Unterminated string literal at position 0.");
    check_error("x = 1; \"abc", "Unterminated string literal at position 7.");
}

// ---------------------------------------------------------------------------
// NameSuggestionsTests — the registry's nearest-match surface.
// ---------------------------------------------------------------------------

/// C# NameSuggestionsTests.cs:28 — ARegisteredNameCloseToTheOneThatFailed_IsOfferedBack.
#[test]
fn r15_registered_name_close_to_the_failure_is_offered_back() {
    for (queried, expected) in [
        ("determinant", "det"),
        ("inverse", "inv"),
        ("factorize", "factor"),
        ("derive", "diff"),
        ("average", "mean"),
        ("rows", "shape"),
        ("numelements", "numel"),
        ("tranpose", "transpose"),
    ] {
        let mut e = engine();
        let err = e
            .eval(&format!("{queried}(1)"))
            .expect_err("an unresolved name was expected");
        assert_eq!(
            err.suggestions,
            vec![expected.to_string()],
            "query {queried}"
        );
    }
}

/// C# NameSuggestionsTests.cs:43 — ANameNothingIsCloseTo_IsNotGuessedAt (the unregistered names).
#[test]
fn r15_name_nothing_is_close_to_is_not_guessed_at() {
    for queried in ["nosuchfn", "push", "zip"] {
        let mut e = engine();
        let err = e
            .eval(&format!("{queried}(1)"))
            .expect_err("an unresolved name was expected");
        assert!(
            err.suggestions.is_empty(),
            "query {queried}: {:?}",
            err.suggestions
        );
    }
}

/// C# NameSuggestionsTests.cs:56 — TheOfferedList_IsCappedAndIndependentOfRegistryOrder.
#[test]
fn r15_offered_list_is_capped_and_ordered() {
    let mut e = engine();
    let err = e
        .eval("sol(1)")
        .expect_err("an unresolved name was expected");
    assert_eq!(
        err.suggestions,
        vec![
            "solve".to_string(),
            "solve_full".to_string(),
            "solve_system".to_string()
        ]
    );
}

/// C# NameSuggestionsTests.cs:70 — ASyntaxName_CarriesItsNoteInsteadOfAFunctionSuggestion.
#[test]
fn r15_syntax_name_carries_its_note() {
    let mut e = engine();
    let err = e
        .eval("range(1)")
        .expect_err("an unresolved name was expected");
    assert!(
        err.suggestions.is_empty(),
        "range suggestions: {:?}",
        err.suggestions
    );
    assert!(
        err.hints.first().is_some_and(|h| h.contains("1..n")),
        "range hints: {:?}",
        err.hints
    );
    assert!(crate::registry::hints("det").is_empty());
}

/// C# NameSuggestionsTests.cs:79 — Rank_OrdersTheFourDegreesOfCloseness.
#[test]
fn r15_rank_orders_the_four_degrees_of_closeness() {
    assert_eq!(crate::registry::name_rank("det", "DET"), 0);
    assert_eq!(crate::registry::name_rank("det", "determinant"), 1);
    assert_eq!(crate::registry::name_rank("transpose", "tranpose"), 2);
    assert_eq!(crate::registry::name_rank("solve_full", "solve_x"), 3);
    assert_eq!(crate::registry::name_rank("det", "fft"), -1);
}

// ---------------------------------------------------------------------------
// LRealPeriodicFastPathTests — the limited-precision fast path must not change
// a result. The reference is the class Real at the same precision, exactly as
// the C# test defines it.
// ---------------------------------------------------------------------------

fn r15_class_real(expr: &str, prec: usize) -> String {
    use lovelace_real::Real;
    let one = Real::from_i64(1);
    let r = |text: &str| Real::from_str(text).expect("class Real literal");
    let div = |a: &Real, b: &Real| a.div(b, prec).expect("class Real division");
    let value = match expr {
        "(1/17)*17" => div(&one, &r("17")).mul(&r("17")),
        "(1/6)*6" => div(&one, &r("6")).mul(&r("6")),
        "(1/13)*(1/7)" => div(&one, &r("13")).mul(&div(&one, &r("7"))),
        "(2/7)*(7/2)" => div(&r("2"), &r("7")).mul(&div(&r("7"), &r("2"))),
        "(4/3)*(3/4)" => div(&r("4"), &r("3")).mul(&div(&r("3"), &r("4"))),
        "(1/3)+(1/6)" => div(&one, &r("3")).add(&div(&one, &r("6"))),
        "(1/3)-(1/6)" => div(&one, &r("3")).sub(&div(&one, &r("6"))),
        "(1/3)-(1/3)" => div(&one, &r("3")).sub(&div(&one, &r("3"))),
        "(1/7)+(1/11)" => div(&one, &r("7")).add(&div(&one, &r("11"))),
        "(1/17)+(1/19)" => div(&one, &r("17")).add(&div(&one, &r("19"))),
        "0.1+(1/3)" => r("0.1").add(&div(&one, &r("3"))),
        "(1/3)/7" => div(&div(&one, &r("3")), &r("7")),
        other => panic!("no class-Real counterpart for '{other}'"),
    };
    value.to_string()
}

/// C# LRealPeriodicFastPathTests.cs:86 — PeriodicExpression_AtLimitedPrecision_MatchesClassReal.
#[test]
fn r15_periodic_expression_at_limited_precision_matches_class_real() {
    for expr in [
        "(1/17)*17",
        "(1/6)*6",
        "(1/13)*(1/7)",
        "(2/7)*(7/2)",
        "(4/3)*(3/4)",
        "(1/3)+(1/6)",
        "(1/3)-(1/6)",
        "(1/3)-(1/3)",
        "(1/7)+(1/11)",
        "(1/17)+(1/19)",
        "0.1+(1/3)",
        "(1/3)/7",
    ] {
        for prec in [18usize, 37] {
            let mut e = engine();
            e.set_precision(prec);
            let value = e
                .eval(expr)
                .unwrap_or_else(|err| panic!("{expr}: {}", err.message));
            assert_eq!(
                r15_class_real(expr, prec),
                e.format(&value),
                "{expr} at {prec} places"
            );
        }
    }
}

/// C# LRealPeriodicFastPathTests.cs:100 — Multiply_OneOverSeventeenTimesSeventeen_IsExactlyOne.
#[test]
fn r15_one_over_seventeen_times_seventeen_is_exactly_one() {
    for prec in [18usize, 37, 1000] {
        let mut e = engine();
        e.set_precision(prec);
        for script in ["(1/17)*17", "(1/17)*17 == 1", "(1/17)*17 - 1 == 0"] {
            let value = e
                .eval(script)
                .unwrap_or_else(|err| panic!("{script}: {}", err.message));
            let expected = if script == "(1/17)*17" { "1" } else { "True" };
            assert_eq!(e.format(&value), expected, "{script} at {prec}");
        }
    }
}

/// C# LRealPeriodicFastPathTests.cs:114 — Multiply_PeriodicQuotientTimesItsDivisor_IsExactlyTheDividend.
#[test]
fn r15_periodic_quotient_times_its_divisor_is_the_dividend() {
    for prec in [18usize, 37] {
        let mut e = engine();
        e.set_precision(prec);
        for (script, expected) in [
            ("(1/13)*(1/7)*91", "1"),
            ("(1/6)*6", "1"),
            ("(1/6)*6 == 1", "True"),
        ] {
            let value = e
                .eval(script)
                .unwrap_or_else(|err| panic!("{script}: {}", err.message));
            assert_eq!(e.format(&value), expected, "{script} at {prec}");
        }
    }
}

// ===========================================================================
// Round 15 — Symbolics rows on the same language/engine surface: evalf's digit
// contract and exactness, the hyperbolic builtins, and the exactness flag.
// ===========================================================================

/// C# EvalfDigitCountContractTests.cs:76 — NonPositiveDigits_AreARecoverableArgumentError_ForEveryShape.
#[test]
fn r15_evalf_non_positive_digits_is_the_typed_argument_error() {
    for f in [
        "sin(1)", "cos(1)", "exp(1)", "log(2)", "sqrt(2)", "1/3", "pi", "e", "2*sin(1)",
    ] {
        for digits in ["0", "-1", "-100"] {
            let mut e = engine();
            let err = e
                .eval(&format!("evalf({f}, {digits})"))
                .expect_err("a digit refusal was expected");
            assert_eq!(err.code, "InvalidArgument", "evalf({f}, {digits})");
            assert_eq!(err.category, "TypeMismatch", "evalf({f}, {digits})");
            assert_eq!(
                err.message,
                format!(
                    "evalf(): argument 2 (digits) must be a Natural or Integer digit count between 1 and 2147483647; got {digits}."
                ),
                "evalf({f}, {digits})"
            );
        }
    }
}

/// C# EvalfDigitCountContractTests.cs:136 — DigitsOutsideTheAdvertisedRange_AreTheSameTypedRefusal.
#[test]
fn r15_evalf_digits_outside_the_advertised_range_are_the_same_refusal() {
    let prefix = "evalf(): argument 2 (digits) must be a Natural or Integer digit count between 1 and 2147483647; got ";
    for digits in [
        "2147483648",
        "4294967297",
        "99999999999999999999999999999999",
        "5.0",
    ] {
        let mut e = engine();
        let err = e
            .eval(&format!("evalf(1/3, {digits})"))
            .expect_err("a digit refusal was expected");
        assert_eq!(err.code, "InvalidArgument", "digits {digits}");
        assert_eq!(err.category, "TypeMismatch", "digits {digits}");
        assert!(
            err.message.starts_with(prefix),
            "digits {digits}: {}",
            err.message
        );
    }
}

/// C# EvalfDigitCountContractTests.cs:115 — DigitsOne_IsAccepted_ForEveryShape (the four shapes
/// whose argument this crate can evaluate numerically).
#[test]
fn r15_evalf_digits_one_is_accepted() {
    for (f, expected) in [
        ("sqrt(2)", "1.4"),
        ("1/3", "0.3"),
        ("pi", "3.1"),
        ("e", "2.7"),
    ] {
        let script = format!("evalf({f}, 1)");
        let (typed, j) = structured_of(&script);
        assert_eq!(field(&j, "kind"), Some("Real"), "{script} ({typed})");
        assert_eq!(field(&j, "value"), Some(expected), "{script}");
    }
}

/// C# EvalfDigitCountContractTests.cs:154 — CountsThatWereHonoured_KeepAnsweringTheRequestedCount.
#[test]
fn r15_evalf_honoured_counts_keep_their_values() {
    for (script, expected) in [
        ("evalf(sqrt(2), 5)", "1.41421"),
        (
            "evalf(sqrt(2), 50)",
            "1.41421356237309504880168872420969807856967187537694",
        ),
        ("evalf(1/3, 5)", "0.33333"),
    ] {
        let (typed, j) = structured_of(script);
        assert_eq!(field(&j, "kind"), Some("Real"), "{script} ({typed})");
        assert_eq!(field(&j, "value"), Some(expected), "{script}");
    }
}

/// C# EvalfTruncatedConstantExactnessTests.cs:70 — TruncatedConstant_IsNeverPublishedAsAnExactRational.
#[test]
fn r15_evalf_truncated_constant_is_never_exact() {
    for (script, expected) in [
        ("evalf(pi(1), 1)", "3.1"),
        ("evalf(pi(1), 5)", "3.1"),
        ("evalf(pi(1), 40)", "3.1"),
        ("evalf(pi(30), 5)", "3.14159"),
        ("evalf(pi(30), 29)", "3.14159265358979323846264338327"),
        ("evalf(pi(30), 30)", "3.141592653589793238462643383279"),
        ("evalf(pi(30), 40)", "3.141592653589793238462643383279"),
        ("evalf(pi(30), 100)", "3.141592653589793238462643383279"),
        ("evalf(e(1), 5)", "2.7"),
        ("evalf(e(1), 40)", "2.7"),
        ("evalf(e(30), 7)", "2.7182818"),
        ("evalf(e(30), 29)", "2.71828182845904523536028747135"),
        ("evalf(e(30), 40)", "2.718281828459045235360287471352"),
        ("evalf(e(30), 100)", "2.718281828459045235360287471352"),
    ] {
        let (typed, j) = structured_of(script);
        assert_eq!(field(&j, "kind"), Some("Real"), "{script} ({typed})");
        assert_eq!(field(&j, "value"), Some(expected), "{script}");
        assert_eq!(
            j.get("exact").and_then(json_bool),
            Some(false),
            "{script} claimed exactness"
        );
        assert!(
            j.get("numerator").is_none(),
            "{script} published a numerator"
        );
        assert!(
            j.get("denominator").is_none(),
            "{script} published a denominator"
        );
    }
}

/// C# EvalfTruncatedConstantExactnessTests.cs:99 — RequestWiderThanTheConstantsOwnCount_CarriesExactlyTheConstantsDigits.
#[test]
fn r15_evalf_wider_than_the_constant_keeps_the_constants_digits() {
    for (name, own_digits, requested) in [
        ("pi", 1, 5),
        ("pi", 1, 40),
        ("pi", 30, 40),
        ("pi", 30, 100),
        ("e", 1, 40),
        ("e", 30, 40),
        ("e", 30, 100),
    ] {
        let constant_script = format!("{name}({own_digits})");
        let evaluated_script = format!("evalf({name}({own_digits}), {requested})");
        let (constant_typed, constant) = structured_of(&constant_script);
        assert_eq!(
            constant.get("exact").and_then(json_bool),
            Some(false),
            "{constant_script} ({constant_typed})"
        );
        assert!(
            constant.get("numerator").is_none() && constant.get("denominator").is_none(),
            "{constant_script} published a rational form"
        );
        let (typed, evaluated) = structured_of(&evaluated_script);
        assert_eq!(
            field(&evaluated, "value"),
            field(&constant, "value"),
            "{evaluated_script} ({typed})"
        );
        assert_eq!(
            evaluated.get("exact").and_then(json_bool),
            Some(false),
            "{evaluated_script} claimed exactness"
        );
        assert!(
            evaluated.get("numerator").is_none() && evaluated.get("denominator").is_none(),
            "{evaluated_script} published a rational form"
        );
    }
}

/// C# EvalfWorkingPrecisionHonoursRequestedDecimalsTests.cs:128 — AlreadyHonouredRequests_KeepTheirValues.
#[test]
fn r15_evalf_already_honoured_requests_keep_their_values() {
    for (script, expected) in [
        ("evalf(sqrt(2), 5)", "1.41421"),
        ("evalf(sqrt(2), 30)", "1.414213562373095048801688724209"),
        ("evalf(1/3, 5)", "0.33333"),
        ("evalf(1/3, 30)", "0.333333333333333333333333333333"),
        ("evalf(pi, 30)", "3.141592653589793238462643383279"),
        ("evalf(pi(30), 30)", "3.141592653589793238462643383279"),
        ("evalf(pi(30), 40)", "3.141592653589793238462643383279"),
        ("evalf(e(30), 29)", "2.71828182845904523536028747135"),
    ] {
        let (typed, j) = structured_of(script);
        assert_eq!(field(&j, "kind"), Some("Real"), "{script} ({typed})");
        assert_eq!(field(&j, "value"), Some(expected), "{script}");
    }
}

/// C# EvalfWorkingPrecisionHonoursRequestedDecimalsTests.cs:150 — ExactControls_KeepTheirTierAndTheirValue.
#[test]
fn r15_evalf_working_precision_exact_controls_keep_their_tier() {
    let (typed, half) = structured_of("evalf(1/2, 40)");
    assert_eq!(field(&half, "kind"), Some("Real"), "{typed}");
    assert_eq!(field(&half, "value"), Some("0.5"));
    assert_eq!(half.get("exact").and_then(json_bool), Some(true));
    assert_eq!(field(&half, "numerator"), Some("1"));
    assert_eq!(field(&half, "denominator"), Some("2"));

    let (typed, two) = structured_of("evalf(2, 30)");
    assert_eq!(field(&two, "kind"), Some("Integer"), "{typed}");
    assert_eq!(field(&two, "value"), Some("2"));
    assert_eq!(two.get("exact").and_then(json_bool), Some(true));
}

/// C# EvalfDigitCapDescriptionTests.cs:26/38 — the evalf summary names the cap and the marker,
/// and no longer denies the clamp.
#[test]
fn r15_evalf_summary_states_the_digit_cap() {
    let descriptor = crate::registry::find("evalf").expect("evalf descriptor");
    let summary = descriptor.summary.expect("evalf summary");
    assert!(summary.contains("1000"), "summary: {summary}");
    assert!(summary.contains("digit-cap"), "summary: {summary}");
    assert!(summary.contains("truncated"), "summary: {summary}");
    assert!(
        !summary.contains("bounded back to the N places that were asked for"),
        "summary still denies the clamp: {summary}"
    );
}

// ---------------------------------------------------------------------------
// HyperbolicBuiltinsAndNegativeIntegerPowersTests.
// ---------------------------------------------------------------------------

/// C# HyperbolicBuiltinsAndNegativeIntegerPowersTests.cs:46 — the three builtins are registered
/// and fold at their defining point to the exact Symbolic 0.
#[test]
fn r15_hyperbolic_builtins_fold_at_their_defining_point() {
    for (name, call) in [
        ("asinh", "asinh(0)"),
        ("acosh", "acosh(1)"),
        ("atanh", "atanh(0)"),
    ] {
        assert!(
            crate::registry::find(name).is_some(),
            "{name} is not in the registry"
        );
        let (typed, j) = structured_of(call);
        assert_eq!(field(&j, "kind"), Some("Symbolic"), "{call} ({typed})");
        assert_eq!(field(&j, "pretty"), Some("0"), "{call} ({typed})");
        assert_eq!(j.get("exact").and_then(json_bool), Some(true), "{call}");
    }
}

/// C# HyperbolicBuiltinsAndNegativeIntegerPowersTests.cs:61 — KeepTheirSymbolicApplicationForm.
#[test]
fn r15_hyperbolic_builtins_keep_their_symbolic_application_form() {
    let mut e = engine();
    e.eval("x = symbol(\"x\")").expect("symbol");
    for call in ["asinh(x)", "acosh(x)", "atanh(x)"] {
        let value = e
            .eval(call)
            .unwrap_or_else(|err| panic!("{call}: {}", err.message));
        assert_eq!(e.format(&value), call, "{call}");
    }
}

/// C# HyperbolicBuiltinsAndNegativeIntegerPowersTests.cs:117 — TwoToTheMinusOne_IsTheExactValueOfOneHalf.
#[test]
fn r15_two_to_the_minus_one_is_the_exact_half() {
    let (typed, j) = structured_of("2^-1");
    assert_eq!(field(&j, "value"), Some("0.5"), "{typed}");
    assert_eq!(j.get("exact").and_then(json_bool), Some(true), "{typed}");
    assert_eq!(field(&j, "numerator"), Some("1"), "{typed}");
    assert_eq!(field(&j, "denominator"), Some("2"), "{typed}");
}

/// C# HyperbolicBuiltinsAndNegativeIntegerPowersTests.cs:141 — ThreeToTheMinusTwo_IsTheExactValueOfOneNinth.
#[test]
fn r15_three_to_the_minus_two_is_the_exact_ninth() {
    let (typed, j) = structured_of("3^-2");
    assert_eq!(field(&j, "value"), Some("0.(1)"), "{typed}");
    assert_eq!(j.get("exact").and_then(json_bool), Some(true), "{typed}");
    assert_eq!(field(&j, "numerator"), Some("1"), "{typed}");
    assert_eq!(field(&j, "denominator"), Some("9"), "{typed}");
}

/// C# HyperbolicBuiltinsAndNegativeIntegerPowersTests.cs:161 — PositiveAndExactIntegerPowers_AreUnchanged.
#[test]
fn r15_positive_integer_powers_are_unchanged() {
    check_typed("2^3", "8 (Natural)");
    check_typed("2^0", "1 (Natural)");
    check_typed("4^-1 * 4^2", "4 (Real)");
}

// ---------------------------------------------------------------------------
// ExactnessFlagTests — the language surface the flag is read from.
// ---------------------------------------------------------------------------

/// C# ExactnessFlagTests.cs:259 — InspectExact_IsTrue_ForExactClosedForms.
#[test]
fn r15_inspect_exact_is_true_for_exact_closed_forms() {
    let mut e = engine();
    e.eval("x = symbol(\"x\")").expect("symbol");
    for expression in [
        "sin(x)",
        "cos(x)",
        "sqrt(x)",
        "log(x)",
        "exp(x)",
        "diff(sin(x), x)",
        "abs(x)",
        "x + 1",
        "sqrt(-2)",
        "x^(3/2)",
    ] {
        let script = format!("inspect({expression}).exact");
        let value = e
            .eval(&script)
            .unwrap_or_else(|err| panic!("{script}: {}", err.message));
        assert_eq!(e.format_typed(&value), "True (Boolean)", "{script}");
    }
}

/// C# ExactnessFlagTests.cs:312 — RootOfRootPublishedByTheSolver_IsExact.
#[test]
fn r15_rootof_solution_is_algebraic_exact() {
    let mut e = engine();
    e.eval("x = symbol(\"x\")").expect("symbol");
    let value = e
        .eval("solve_full(x^5 - x - 1 == 0, x, real)")
        .expect("quintic solve");
    let j = crate::structured::project_value(&e, &value, None);
    let solutions = r15_record_field(&j, "solutions");
    let elements = solutions
        .get("elements")
        .and_then(|v| v.as_arr())
        .expect("solutions array");
    assert!(!elements.is_empty(), "the quintic published no solutions");
    for solution in elements {
        assert_eq!(
            field(r15_record_field(solution, "exactness"), "value"),
            Some("AlgebraicExact")
        );
        assert_eq!(
            r15_record_field(solution, "value")
                .get("exact")
                .and_then(json_bool),
            Some(true)
        );
    }
}

/// C# ExactnessFlagTests.cs:334 — PublishedSolutionValueExact_AgreesWithItsExactnessEnum.
#[test]
fn r15_published_solution_exactness_agrees_with_its_value() {
    let mut e = engine();
    e.eval("x = symbol(\"x\")").expect("symbol");
    for script in [
        "solve_full(x^2 - 2 == 0, x, real)",
        "solve_full(x^2 - 4 == 0, x, real)",
        "solve_full(x^2 + 1 == 0, x)",
        "solve_full(x^3 - 2 == 0, x, real)",
        "solve_full(x^5 - x - 1 == 0, x, real)",
        "solve_full(exp(x) == 1, x)",
        "solve_full((x^2 - 1)/(x - 1) == 0, x)",
    ] {
        let value = e
            .eval(script)
            .unwrap_or_else(|err| panic!("{script}: {}", err.message));
        let j = crate::structured::project_value(&e, &value, None);
        let solutions = r15_record_field(&j, "solutions");
        let Some(elements) = solutions.get("elements").and_then(|v| v.as_arr()) else {
            continue;
        };
        for solution in elements {
            let exactness = field(r15_record_field(solution, "exactness"), "value")
                .unwrap_or_else(|| panic!("{script}: solution has no exactness"));
            let value_exact = r15_record_field(solution, "value")
                .get("exact")
                .and_then(json_bool)
                .unwrap_or_else(|| panic!("{script}: solution value has no exact flag"));
            assert_eq!(
                value_exact,
                exactness != "Approximate",
                "{script}: exactness {exactness} disagrees with value.exact {value_exact}"
            );
        }
    }
}

/// C# TruncatingEvaluationStaysInexactTests.cs:315 — ElementaryFunctionOfAnExactArgument_StaysExact
/// (the acosh row: this crate folds acosh(1) to the exact rational 0; the other ten rows are not
/// attempted because this crate leaves those functions as symbolic applications).
#[test]
fn r15_elementary_function_of_an_exact_argument_stays_exact() {
    let (typed, j) = structured_of("acosh(1)");
    assert_eq!(field(&j, "kind"), Some("Symbolic"), "{typed}");
    assert_eq!(j.get("exact").and_then(json_bool), Some(true), "{typed}");
    assert_eq!(field(&j, "canonical"), Some("(rat 0 1)"), "{typed}");
}
