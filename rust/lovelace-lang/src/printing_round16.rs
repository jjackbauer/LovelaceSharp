//! Round 16: the language surface of the Symbolics printing cluster.
//!
//! This module ports the assertions of three C# theories that were refused because the printing
//! surface was absent:
//!   * PrettyParenthesesTests.CanonicalMode_StillRoundTripsThroughCanonicalParse (48 rows);
//!   * PrettyParenthesesTests.DebugMode_IsStructuralAndContainsNoPrecedenceParentheses (48 rows);
//!   * LatexPrinterTests.CanonicalAndPretty_AreUntouchedByTheLatexArm (49 rows).
//!
//! The C# assertions are properties of the SAME language corpus: parse the canonical text back,
//! compare canonical text; compare DebugPrint with the Debug print mode and forbid "(("; and
//! check that Canonical and Debug are unaffected by the LaTeX arm. The oracle debug/canonical
//! strings in the last test were captured from the pinned C# Printing class for the same
//! expressions (see docs/goal-parity-contract/round-16/symbolics-printing.md).

use crate::{Engine, Value};
use lovelace_symbolics::{canonical_parse, Expr as Sym, PrintMode};

const PRELUDE: &str = "x = symbol(\"x\"); y = symbol(\"y\"); a = symbol(\"a\"); b = symbol(\"b\"); z = symbol(\"z\"); w = symbol(\"w\");\n";

const PRETTY_CORPUS: &[&str] = &[
    "(x + 1)^12",
    "exp(-x^2)",
    "integrate_full(exp(-x^2), x)",
    "factor(-x^2 - 2*x - 1)",
    "x - (y - a)",
    "x - y - a",
    "a - (x + y)",
    "-x^2",
    "(-x)^2",
    "(-x)^3",
    "-(x + y)",
    "(-x - 1)^2",
    "(-x)^(1/2)",
    "(-1)^x",
    "(-2)^x",
    "(-1/2)^x",
    "(-1.5)^x",
    "(-3)^(-x)",
    "(1/2)^x",
    "(2/3)^x",
    "(-3/2)^x",
    "(x/y)/z",
    "x/y/z",
    "x/y/z/w",
    "(x/y)/(z/w)",
    "x/(y*z)",
    "(x*y)/(z*w)",
    "(x/y)*z",
    "apart(1/(x^2 - 1), x)",
    "(x^y)^2",
    "x^(y^2)",
    "(x + y)*(x - y)",
    "x*(y + a)",
    "-(x + y)*(x - y)",
    "x/(y + 1)",
    "(x + 1)/y",
    "x^(-2)",
    "(x + 1)^(-1)",
    "1/(2*(x + 1))",
    "-1/(2*(x + 1))",
    "(1/2)*x + 1/3",
    "x/2",
    "exp(-(x + 1)^2)",
    "sin(x + y) + cos(x^2)",
    "sqrt(x + y)",
    "log(x + 1)/(x - 1)",
    "diff(exp(x)*x^2, x)",
    "sin((x + y)/(x - y))",
];

const LATEX_CORPUS: &[&str] = &[
    "x - (y - a)",
    "x - y - a",
    "a - (x + y)",
    "-x^2",
    "(-x)^2",
    "(-x)^3",
    "-(x + y)",
    "(-x - 1)^2",
    "(-1)^x",
    "(-2)^x",
    "(-1/2)^x",
    "(-1.5)^x",
    "(1/2)^x",
    "(2/3)^x",
    "(-3/2)^x",
    "(x/y)/z",
    "x/y/z",
    "x/y/z/w",
    "(x/y)/(z/w)",
    "x/(y*z)",
    "(x*y)/(z*w)",
    "(x/y)*z",
    "apart(1/(x^2 - 1), x)",
    "(x + 1)^12",
    "(x + y)^2",
    "(x^y)^2",
    "x^(y^2)",
    "(x + y)*(x - y)",
    "x*(y + a)",
    "-(x + y)*(x - y)",
    "x/(y + 1)",
    "(x + 1)/y",
    "x^(-2)",
    "(x + 1)^(-1)",
    "1/(2*(x + 1))",
    "-1/(2*(x + 1))",
    "(1/2)*x + 1/3",
    "x/2",
    "x/(2*y)",
    "exp(-x^2)",
    "sin(x + y) + cos(x^2)",
    "sqrt(x + y)",
    "(-x)^(1/2)",
    "x^(-1/2)",
    "log(x + 1)/(x - 1)",
    "sin((x + y)/(x - y))",
    "abs(x)",
    "abs(x + y)*(x - 1)",
    "abs(x - y)^2",
];

fn engine() -> Engine {
    Engine::new()
}

/// The expression a corpus row denotes. integrate_full publishes a record; the C# BuildAsync
/// renders the record's "expression" field, and so does this helper.
fn symbolic(source: &str) -> Sym {
    let mut e = engine();
    let value = e
        .eval(&format!("{PRELUDE}{source}"))
        .unwrap_or_else(|err| panic!("{source} unexpectedly failed: {}", err.message));
    match value {
        Value::Symbolic(expr) => expr,
        Value::Record(_, fields) => match fields.iter().find(|(name, _)| name == "expression") {
            Some((_, Value::Symbolic(expr))) => expr.clone(),
            other => panic!("{source} record expression is {other:?}"),
        },
        other => panic!("{source} produced {} instead of Symbolic", other.kind_name()),
    }
}

fn printed(source: &str, mode: &str) -> String {
    let mut e = engine();
    let expression = if source.starts_with("integrate_full") {
        format!("({source}).expression")
    } else {
        source.to_string()
    };
    let script = format!("{PRELUDE}e = {expression}; print(e, \"{mode}\")");
    e.eval(&script)
        .unwrap_or_else(|err| panic!("print({source}, {mode}) failed: {}", err.message));
    e.take_output().join("\n")
}

#[test]
fn canonical_mode_round_trips_through_canonical_parse_on_the_pretty_corpus() {
    for source in PRETTY_CORPUS {
        let expr = symbolic(source);
        let canonical = expr.canonical();
        let reparsed = canonical_parse(&canonical)
            .unwrap_or_else(|err| panic!("{source}: canonical parse of {canonical} failed: {err}"));
        assert_eq!(
            reparsed.canonical(),
            canonical,
            "{source}: canonical text did not round-trip"
        );
        assert_eq!(
            expr.print(PrintMode::Canonical),
            canonical,
            "{source}: Canonical mode is not CanonicalPrint"
        );
        assert_eq!(
            printed(source, "canonical"),
            canonical,
            "{source}: print(expr, \"canonical\") is not CanonicalPrint"
        );
    }
}

#[test]
fn debug_mode_is_structural_and_contains_no_precedence_parentheses_on_the_pretty_corpus() {
    for source in PRETTY_CORPUS {
        let expr = symbolic(source);
        let debug = expr.debug_print();
        assert_eq!(
            expr.print(PrintMode::Debug),
            debug,
            "{source}: Debug mode is not DebugPrint"
        );
        assert!(
            !debug.contains("(("),
            "{source}: debug rendering contains '((': {debug}"
        );
        assert_eq!(
            printed(source, "debug"),
            debug,
            "{source}: print(expr, \"debug\") is not DebugPrint"
        );
    }
}

#[test]
fn canonical_and_debug_are_untouched_by_the_latex_arm_on_the_latex_corpus() {
    for source in LATEX_CORPUS {
        let expr = symbolic(source);
        let canonical = expr.canonical();
        let debug = expr.debug_print();
        // The oracle's canonical text for this row parses back to the same text, which is the
        // C# CanonicalMode assertion applied to the LaTeX corpus as well.
        let reparsed = canonical_parse(&canonical)
            .unwrap_or_else(|err| panic!("{source}: canonical parse failed: {err}"));
        assert_eq!(
            reparsed.canonical(),
            canonical,
            "{source}: canonical text did not round-trip"
        );
        assert_eq!(
            expr.print(PrintMode::Canonical),
            canonical,
            "{source}: the LaTeX arm disturbed Canonical mode"
        );
        assert_eq!(
            expr.print(PrintMode::Debug),
            debug,
            "{source}: the LaTeX arm disturbed Debug mode"
        );
        assert_eq!(
            printed(source, "latex"),
            expr.latex(),
            "{source}: print(expr, \"latex\") is not the LaTeX rendering"
        );
        assert!(
            printed(source, "canonical").starts_with('('),
            "{source}: Print under Canonical does not start with '('"
        );
    }
}

/// The oracle-pinned table: source, canonical text, DebugPrint text, LaTeX text. Every expected
/// value was captured from the C# Printing class on the expression the same language source
/// builds, so this is an oracle comparison, not a Rust self-comparison.
#[test]
fn debug_and_canonical_modes_match_the_oracle_on_twenty_eight_expressions() {
    let cases: &[(&str, &str, &str, &str)] = &[
    ("(x + 1)^12", "(pow (add (rat 1 1) (sym x)) (rat 12 1))", "pow[add[rat(1), sym(x)], rat(12)]", "\\left(x + 1\\right)^{12}"),
    ("exp(-x^2)", "(fn exp (mul (rat -1 1) (pow (sym x) (rat 2 1))))", "fn:exp(mul[rat(-1), pow[sym(x), rat(2)]])", "\\exp(-x^{2})"),
    ("factor(-x^2 - 2*x - 1)", "(mul (rat -1 1) (pow (add (rat 1 1) (sym x)) (rat 2 1)))", "mul[rat(-1), pow[add[rat(1), sym(x)], rat(2)]]", "-\\left(x + 1\\right)^{2}"),
    ("x - (y - a)", "(add (sym x) (mul (rat -1 1) (add (sym y) (mul (rat -1 1) (sym a)))))", "add[sym(x), mul[rat(-1), add[sym(y), mul[rat(-1), sym(a)]]]]", "x - \\left(y - a\\right)"),
    ("x - y - a", "(add (sym x) (mul (rat -1 1) (sym a)) (mul (rat -1 1) (sym y)))", "add[sym(x), mul[rat(-1), sym(a)], mul[rat(-1), sym(y)]]", "x - a - y"),
    ("a - (x + y)", "(add (sym a) (mul (rat -1 1) (add (sym x) (sym y))))", "add[sym(a), mul[rat(-1), add[sym(x), sym(y)]]]", "a - \\left(x + y\\right)"),
    ("-x^2", "(mul (rat -1 1) (pow (sym x) (rat 2 1)))", "mul[rat(-1), pow[sym(x), rat(2)]]", "-x^{2}"),
    ("(-x)^2", "(pow (mul (rat -1 1) (sym x)) (rat 2 1))", "pow[mul[rat(-1), sym(x)], rat(2)]", "\\left(-x\\right)^{2}"),
    ("(-x)^3", "(pow (mul (rat -1 1) (sym x)) (rat 3 1))", "pow[mul[rat(-1), sym(x)], rat(3)]", "\\left(-x\\right)^{3}"),
    ("-(x + y)", "(mul (rat -1 1) (add (sym x) (sym y)))", "mul[rat(-1), add[sym(x), sym(y)]]", "-\\left(x + y\\right)"),
    ("(-x - 1)^2", "(pow (add (rat -1 1) (mul (rat -1 1) (sym x))) (rat 2 1))", "pow[add[rat(-1), mul[rat(-1), sym(x)]], rat(2)]", "\\left(-x - 1\\right)^{2}"),
    ("(-x)^(1/2)", "(pow (mul (rat -1 1) (sym x)) (rat 1 2))", "pow[mul[rat(-1), sym(x)], rat(1/2)]", "\\sqrt{-x}"),
    ("(-1)^x", "(pow (rat -1 1) (sym x))", "pow[rat(-1), sym(x)]", "\\left(-1\\right)^{x}"),
    ("(-2)^x", "(pow (rat -2 1) (sym x))", "pow[rat(-2), sym(x)]", "\\left(-2\\right)^{x}"),
    ("(-1/2)^x", "(pow (rat -1 2) (sym x))", "pow[rat(-1/2), sym(x)]", "\\left(-\\frac{1}{2}\\right)^{x}"),
    ("(-1.5)^x", "(pow (rat -3 2) (sym x))", "pow[rat(-3/2), sym(x)]", "\\left(-\\frac{3}{2}\\right)^{x}"),
    ("(-3)^(-x)", "(pow (rat -3 1) (mul (rat -1 1) (sym x)))", "pow[rat(-3), mul[rat(-1), sym(x)]]", "\\left(-3\\right)^{-x}"),
    ("(1/2)^x", "(pow (rat 1 2) (sym x))", "pow[rat(1/2), sym(x)]", "\\frac{1}{2}^{x}"),
    ("(2/3)^x", "(pow (rat 2 3) (sym x))", "pow[rat(2/3), sym(x)]", "\\frac{2}{3}^{x}"),
    ("(-3/2)^x", "(pow (rat -3 2) (sym x))", "pow[rat(-3/2), sym(x)]", "\\left(-\\frac{3}{2}\\right)^{x}"),
    ("(x/y)/z", "(mul (sym x) (pow (sym y) (rat -1 1)) (pow (sym z) (rat -1 1)))", "mul[sym(x), pow[sym(y), rat(-1)], pow[sym(z), rat(-1)]]", "\\frac{\\frac{x}{y}}{z}"),
    ("x/y/z", "(mul (sym x) (pow (sym y) (rat -1 1)) (pow (sym z) (rat -1 1)))", "mul[sym(x), pow[sym(y), rat(-1)], pow[sym(z), rat(-1)]]", "\\frac{\\frac{x}{y}}{z}"),
    ("x/y/z/w", "(mul (sym x) (pow (sym w) (rat -1 1)) (pow (sym y) (rat -1 1)) (pow (sym z) (rat -1 1)))", "mul[sym(x), pow[sym(w), rat(-1)], pow[sym(y), rat(-1)], pow[sym(z), rat(-1)]]", "\\frac{\\frac{\\frac{x}{w}}{y}}{z}"),
    ("(x/y)/(z/w)", "(mul (sym x) (pow (sym y) (rat -1 1)) (pow (mul (sym z) (pow (sym w) (rat -1 1))) (rat -1 1)))", "mul[sym(x), pow[sym(y), rat(-1)], pow[mul[sym(z), pow[sym(w), rat(-1)]], rat(-1)]]", "\\frac{\\frac{x}{y}}{\\frac{z}{w}}"),
    ("x/(y*z)", "(mul (sym x) (pow (mul (sym y) (sym z)) (rat -1 1)))", "mul[sym(x), pow[mul[sym(y), sym(z)], rat(-1)]]", "\\frac{x}{y \\cdot z}"),
    ("(x*y)/(z*w)", "(mul (sym x) (sym y) (pow (mul (sym w) (sym z)) (rat -1 1)))", "mul[sym(x), sym(y), pow[mul[sym(w), sym(z)], rat(-1)]]", "\\frac{x \\cdot y}{w \\cdot z}"),
    ("(x/y)*z", "(mul (sym x) (sym z) (pow (sym y) (rat -1 1)))", "mul[sym(x), sym(z), pow[sym(y), rat(-1)]]", "\\frac{x \\cdot z}{y}"),
    ("apart(1/(x^2 - 1), x)", "(add (mul (rat -1 2) (pow (add (rat 1 1) (sym x)) (rat -1 1))) (mul (rat 1 2) (pow (add (rat -1 1) (sym x)) (rat -1 1))))", "add[mul[rat(-1/2), pow[add[rat(1), sym(x)], rat(-1)]], mul[rat(1/2), pow[add[rat(-1), sym(x)], rat(-1)]]]", "-\\frac{\\frac{1}{2}}{x + 1} + \\frac{\\frac{1}{2}}{x - 1}"),
    ];
    assert!(cases.len() >= 20);
    for (source, canonical, debug, latex) in cases {
        let expr = symbolic(source);
        assert_eq!(expr.canonical(), *canonical, "{source}: canonical differs from the oracle");
        assert_eq!(expr.print(PrintMode::Canonical), *canonical, "{source}: Canonical mode");
        assert_eq!(expr.debug_print(), *debug, "{source}: DebugPrint differs from the oracle");
        assert_eq!(expr.print(PrintMode::Debug), *debug, "{source}: Debug mode");
        assert_eq!(expr.latex(), *latex, "{source}: LaTeX differs from the oracle");
        assert_eq!(printed(source, "debug"), *debug, "{source}: print(expr, \"debug\")");
    }
}

#[test]
fn canonical_parse_reads_the_oracle_example_from_the_task() {
    let text = "(add (rat 1 1) (pow (sym x) (rat 2 1)))";
    let expr = canonical_parse(text).expect("the documented canonical form parses");
    // A codec, not a re-canonicaliser: the operand order is the text's, so the CANONICAL text is
    // what round-trips byte-identically. The debug rendering reads the same order.
    assert_eq!(expr.canonical(), text);
    assert_eq!(expr.debug_print(), "add[rat(1), pow[sym(x), rat(2)]]");
}

#[test]
fn print_builtin_refuses_an_unknown_mode_and_names_the_accepted_set() {
    let mut e = engine();
    let err = e
        .eval("x = symbol(\"x\"); print(x, \"wibble\")")
        .expect_err("an unknown print mode must be refused");
    assert_eq!(err.code, "InvalidArgument");
    assert_eq!(err.category, "TypeMismatch");
    assert_eq!(
        err.message,
        "print(): unknown print mode 'wibble'; expected one of: pretty, canonical, latex, debug."
    );
    assert!(
        e.take_output().is_empty(),
        "a refused print mode must not write a partial rendering"
    );
}

#[test]
fn print_builtin_keeps_the_reference_behaviour_for_non_mode_calls() {
    let cases: &[(&str, &str)] = &[
        ("print(\"hi\", 42)", "hi 42"),
        // The first argument is not symbolic, so this is the reference's two-value print.
        ("print(\"hi\", \"debug\")", "hi debug"),
        ("x = symbol(\"x\"); print(x)", "x"),
        ("x = symbol(\"x\"); print(x, 1)", "x 1"),
        ("x = symbol(\"x\"); print(x, \"DEBUG\")", "sym(x)"),
        ("x = symbol(\"x\"); print(x, \"pretty\")", "x"),
        ("x = symbol(\"x\"); print(x, \"canonical\")", "(sym x)"),
        ("x = symbol(\"x\"); print(x, \"latex\")", "x"),
    ];
    for (script, expected) in cases {
        let mut e = engine();
        e.eval(script)
            .unwrap_or_else(|err| panic!("{script} failed: {}", err.message));
        assert_eq!(e.take_output().join("\n"), *expected, "script: {script}");
    }
}
