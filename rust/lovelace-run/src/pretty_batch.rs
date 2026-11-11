//! Pretty-printer ports from Lovelace.Symbolics.Tests.PrettyParenthesesTests.
//!
//! The C# acceptance property is round-trip: parse(pretty(e)) has the same canonical form as e.
//! The Rust port evaluates the same source through the engine, re-evaluates the published display,
//! and compares the structured canonical form. Minimality is the second property: the display has
//! no doubled parenthesis pair.

use super::parity_batch::*;
use super::*;
use lovelace_json::Json;

const PRELUDE: &str = "x = symbol(\"x\"); y = symbol(\"y\"); a = symbol(\"a\"); b = symbol(\"b\"); \
z = symbol(\"z\"); w = symbol(\"w\"); ";

const CORPUS: &[&str] = &[
    "(x + 1)^12", "exp(-x^2)", "integrate_full(exp(-x^2), x)", "factor(-x^2 - 2*x - 1)",
    "x - (y - a)", "x - y - a", "a - (x + y)", "-x^2", "(-x)^2", "(-x)^3", "-(x + y)",
    "(-x - 1)^2", "(-x)^(1/2)", "(-1)^x", "(-2)^x", "(-1/2)^x", "(-1.5)^x", "(-3)^(-x)",
    "(1/2)^x", "(2/3)^x", "(-3/2)^x", "(x/y)/z", "x/y/z", "x/y/z/w", "(x/y)/(z/w)",
    "x/(y*z)", "(x*y)/(z*w)", "(x/y)*z", "apart(1/(x^2 - 1), x)", "(x^y)^2", "x^(y^2)",
    "(x + y)*(x - y)", "x*(y + a)", "-(x + y)*(x - y)", "x/(y + 1)", "(x + 1)/y", "x^(-2)",
    "(x + 1)^(-1)", "1/(2*(x + 1))", "-1/(2*(x + 1))", "(1/2)*x + 1/3", "x/2",
    "exp(-(x + 1)^2)", "sin(x + y) + cos(x^2)", "sqrt(x + y)", "log(x + 1)/(x - 1)",
    "diff(exp(x)*x^2, x)", "sin((x + y)/(x - y))",
];

/// The sources whose round-trip the reference accepts and Rust reproduces.
const ROUND_TRIP: &[&str] = &[
    "(x + 1)^12", "exp(-x^2)", "integrate_full(exp(-x^2), x)", "factor(-x^2 - 2*x - 1)",
    "x - y - a", "-x^2", "(-x)^2", "(-x)^3",
    "-(x + y)", "(-x - 1)^2", "x/(y*z)", "(x*y)/(z*w)",
    "(x/y)*z", "apart(1/(x^2 - 1), x)", "(x^y)^2", "x^(y^2)",
    "(x + y)*(x - y)", "x*(y + a)", "-(x + y)*(x - y)", "x/(y + 1)",
    "(x + 1)/y", "x^(-2)", "(x + 1)^(-1)", "1/(2*(x + 1))",
    "-1/(2*(x + 1))", "x/2", "exp(-(x + 1)^2)", "sin(x + y) + cos(x^2)",
    "sqrt(x + y)", "log(x + 1)/(x - 1)", "diff(exp(x)*x^2, x)", "sin((x + y)/(x - y))",
    // the negative/rational power bases, whose display now re-parses to the same canonical form
    "(-x)^(1/2)", "(-1)^x", "(-2)^x", "(-1/2)^x", "(-1.5)^x", "(-3)^(-x)",
    "(1/2)^x", "(2/3)^x", "(-3/2)^x", "(1/2)*x + 1/3",
    // nested subtraction and flat denominators: the display divided once per denominator factor
    // and kept the subtrahend's sum delimited, so these now re-parse to the same value
    "x - (y - a)", "a - (x + y)",
    "(x/y)/z", "x/y/z", "x/y/z/w", "(x/y)/(z/w)",
];

/// The sources whose Rust display is minimal: no doubled parenthesis pair.
const MINIMALITY: &[&str] = &[
    "(x + 1)^12", "exp(-x^2)", "integrate_full(exp(-x^2), x)", "factor(-x^2 - 2*x - 1)",
    "x - (y - a)", "x - y - a", "a - (x + y)", "-x^2", "(-x)^2", "(-x)^3", "-(x + y)",
    "(-x - 1)^2", "(-x)^(1/2)", "(-1)^x", "(-2)^x", "(-1/2)^x", "(-1.5)^x", "(-3)^(-x)",
    "(1/2)^x", "(-3/2)^x", "(x/y)/z", "x/y/z", "x/y/z/w", "(x/y)/(z/w)", "x/(y*z)",
    "(x*y)/(z*w)", "(x/y)*z", "apart(1/(x^2 - 1), x)", "(x^y)^2", "x^(y^2)",
    "(x + y)*(x - y)", "x*(y + a)", "-(x + y)*(x - y)", "x/(y + 1)", "(x + 1)/y", "x^(-2)",
    "(x + 1)^(-1)", "1/(2*(x + 1))", "-1/(2*(x + 1))", "x/2", "exp(-(x + 1)^2)",
    "sin(x + y) + cos(x^2)", "sqrt(x + y)", "log(x + 1)/(x - 1)", "diff(exp(x)*x^2, x)",
    "sin((x + y)/(x - y))", "(2/3)^x", "(1/2)*x + 1/3",
];

struct Built {
    pretty: String,
    canonical: String,
}

fn evaluate(source: &str) -> RunResult {
    let script = format!("{PRELUDE}{source}");
    run_args(&["--eval", &script, "--omit-functions", "--omit-variables"])
}

fn build(source: &str) -> Built {
    let r = evaluate(source);
    assert_eq!(r.exit, 0, "'{source}' must evaluate: {}", r.stdout);
    let mut structured = get(get(&r.env, "result"), "structured");
    let mut pretty = text(get(get(&r.env, "result"), "display")).to_string();
    if text(get(structured, "kind")) == "Record" {
        let expression = record_field(structured, "expression");
        structured = expression;
        pretty = text(get(expression, "pretty")).to_string();
    }
    assert_eq!(text(get(structured, "kind")), "Symbolic", "'{source}' is not symbolic");
    Built { pretty, canonical: text(get(structured, "canonical")).to_string() }
}

fn build_display(source: &str) -> String {
    let r = evaluate(source);
    assert_eq!(r.exit, 0, "'{source}' must evaluate");
    text(get(get(&r.env, "result"), "display")).to_string()
}

fn doubled_pairs(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut matches = vec![usize::MAX; chars.len()];
    let mut stack: Vec<usize> = Vec::new();
    for (i, ch) in chars.iter().enumerate() {
        if *ch == '(' {
            stack.push(i);
        } else if *ch == ')' {
            if let Some(open) = stack.pop() {
                matches[open] = i;
                matches[i] = open;
            }
        }
    }
    let mut out = Vec::new();
    for i in 0..chars.len().saturating_sub(1) {
        if chars[i] == '(' && chars[i + 1] == '(' && matches[i] == matches[i + 1] + 1 {
            out.push(chars[i..=matches[i]].iter().collect());
        }
    }
    out
}

#[test]
fn pretty_round_trips_to_the_same_canonical_form() {
    for source in ROUND_TRIP {
        let built = build(source);
        let reparsed = evaluate(&built.pretty);
        assert_eq!(reparsed.exit, 0, "'{source}' display '{}' must re-parse", built.pretty);
        let mut structured = get(get(&reparsed.env, "result"), "structured");
        if text(get(structured, "kind")) == "Record" {
            structured = record_field(structured, "expression");
        }
        assert_eq!(
            text(get(structured, "canonical")),
            built.canonical,
            "'{source}' rendered as '{}' and re-parsed to a different canonical form",
            built.pretty
        );
    }
}

#[test]
fn pretty_is_minimal_no_doubled_parenthesis_pair() {
    for source in MINIMALITY {
        let pretty = build_display(source);
        let doubled = doubled_pairs(&pretty);
        assert!(
            doubled.is_empty(),
            "redundant parentheses in \"{source}\" -> {pretty}: {}",
            doubled.join(" ")
        );
    }
}

#[test]
fn pretty_reported_case_power_of_a_sum_has_one_pair() {
    assert_eq!(build_display("(x + 1)^12"), "(x + 1)^12");
}

#[test]
fn pretty_reported_case_exp_of_negative_square_has_no_argument_parens() {
    assert_eq!(build_display("exp(-x^2)"), "exp(-x^2)");
}
/// The offset of the parenthesis that closes the one at `open`, or None when the text has no
/// such pair.
fn matching_close(text: &str, open: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    for (index, byte) in bytes.iter().enumerate().skip(open) {
        match byte {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

/// Asserts that `rendering` raises a power whose base text is exactly the one delimited group
/// `expected_base`: the base cannot leak an operator of its own to the power's level.
fn assert_delimited_base(rendering: &str, expected_base: &str) {
    let caret = rendering.find('^').unwrap_or_else(|| panic!("no power in \"{rendering}\""));
    let basis = &rendering[..caret];
    assert_eq!(basis, expected_base);
    assert!(basis.starts_with('('), "the base {basis} is not a delimited group");
    assert_eq!(matching_close(basis, 0), Some(basis.len() - 1), "the group IS the whole base");
}

/// ReportedCase_FactoredSquare_HasOnePair — the factoriser emits -(x + 1)^2, one pair around the
/// sum and none around the negative coefficient.
#[test]
fn pretty_reported_case_factored_square_has_one_pair() {
    assert_eq!(build_display("factor(-x^2 - 2*x - 1)"), "-(x + 1)^2");
}

/// ReportedCase_FlatDenominators_DivideOncePerFactor — the number of divisions in the rendering
/// equals the number of denominator factors in the value: x/(y*z) is ONE divisor (the product), so
/// it keeps its single slash and its pair; two separate denominator factors divide twice.
#[test]
fn pretty_reported_case_flat_denominators_divide_once_per_factor() {
    assert_eq!(build_display("(x/y)/z"), "x/y/z");
    assert_eq!(build_display("x/y/z"), "x/y/z");
    assert_eq!(build_display("x/y/z/w"), "x/w/y/z");
    assert_eq!(build_display("x/(y*z)"), "x/(y*z)");
    assert_eq!(build_display("(x/y)/(z/w)"), "x/y/(z/w)");
}

/// LatexPrinterTests.GroupingParity_LatexAndPrettyHaveTheSameStructuralNesting for the eight rows
/// the pretty grouping fix unblocked: both renderings carry the same structural nesting, which for
/// these precedence-sensitive shapes is pinned by the exact texts the reference emits.
#[test]
fn latex_grouping_rows_carry_the_same_nesting_as_pretty() {
    for (source, pretty, latex) in [
        ("x - (y - a)", "x - (y - a)", "x - \\left(y - a\\right)"),
        ("a - (x + y)", "a - (x + y)", "a - \\left(x + y\\right)"),
        ("(x/y)/z", "x/y/z", "\\frac{\\frac{x}{y}}{z}"),
        ("x/y/z", "x/y/z", "\\frac{\\frac{x}{y}}{z}"),
        ("x/y/z/w", "x/w/y/z", "\\frac{\\frac{\\frac{x}{w}}{y}}{z}"),
        ("(x/y)/(z/w)", "x/y/(z/w)", "\\frac{\\frac{x}{y}}{\\frac{z}{w}}"),
        ("(-x)^(1/2)", "sqrt(-x)", "\\sqrt{-x}"),
        ("x^(-1/2)", "1/sqrt(x)", "\\frac{1}{\\sqrt{x}}"),
    ] {
        assert_eq!(build_display(source), pretty, "{source}: pretty");
        assert_eq!(build_display(&format!("latex({source})")), latex, "{source}: latex");
    }
}

#[test]
fn pretty_reported_case_power_of_a_rational_constant_has_a_delimited_base() {
    let built = build("(1/2)^x");
    assert_eq!(built.pretty, "(1/2)^x");
    assert_eq!(built.canonical, "(pow (rat 1 2) (sym x))");
    assert_delimited_base(&built.pretty, "(1/2)");

    let third = build("(2/3)^x");
    assert_eq!(third.pretty, "(2/3)^x");
}
