//! LaTeX and negative-square-root parity: the printer arm, the language builtin and the
//! principal complex square root, each pinned to the C# oracle's exact output.
//!
//! The C# contract is `Lovelace.Symbolics.Tests.LatexPrinterTests` /
//! `LatexSymbolNameEscapingTests` (the 213 census cases that call the LaTeX arm) and
//! `HyperbolicBuiltinsAndNegativeIntegerPowersTests` / `ComplexClosedFormTests` for sqrt.
//! Every expected string below was produced by replaying the same source through the pinned
//! `Lovelace.Run` oracle, so the assertion is the reference's behaviour, not this crate's.

use crate::Engine;
use crate::Value;

const PRELUDE: &str = "x = symbol(\"x\"); y = symbol(\"y\"); a = symbol(\"a\"); b = symbol(\"b\"); z = symbol(\"z\"); w = symbol(\"w\");\n";

fn engine() -> Engine {
    Engine::new()
}

fn latex(source: &str) -> String {
    let mut e = engine();
    match e.eval(&format!("{PRELUDE}{source}")) {
        Ok(Value::Text(text)) => text,
        Ok(v) => panic!("{source} produced {} instead of Text", e.format_typed(&v)),
        Err(err) => panic!("{source} unexpectedly failed: {}", err.message),
    }
}

fn display(source: &str) -> String {
    let mut e = engine();
    match e.eval(&format!("{PRELUDE}{source}")) {
        Ok(v) => e.format_typed(&v),
        Err(err) => panic!("{source} unexpectedly failed: {}", err.message),
    }
}

fn canonical(source: &str) -> String {
    let mut e = engine();
    match e.eval(&format!("{PRELUDE}{source}")) {
        Ok(Value::Symbolic(s)) => s.canonical(),
        Ok(v) => panic!(
            "{source} produced {} instead of Symbolic",
            e.format_typed(&v)
        ),
        Err(err) => panic!("{source} unexpectedly failed: {}", err.message),
    }
}

#[test]
fn latex_precedence_corpus_matches_the_oracle_exactly() {
    let corpus: &[(&str, &str)] = &[
        ("x - (y - a)", "x - \\left(y - a\\right)"),
        ("x - y - a", "x - a - y"),
        ("a - (x + y)", "a - \\left(x + y\\right)"),
        ("-x^2", "-x^{2}"),
        ("(-x)^2", "\\left(-x\\right)^{2}"),
        ("(-x)^3", "\\left(-x\\right)^{3}"),
        ("-(x + y)", "-\\left(x + y\\right)"),
        ("(-x - 1)^2", "\\left(-x - 1\\right)^{2}"),
        ("(-1)^x", "\\left(-1\\right)^{x}"),
        ("(-2)^x", "\\left(-2\\right)^{x}"),
        ("(-1/2)^x", "\\left(-\\frac{1}{2}\\right)^{x}"),
        ("(-1.5)^x", "\\left(-\\frac{3}{2}\\right)^{x}"),
        ("(1/2)^x", "\\frac{1}{2}^{x}"),
        ("(2/3)^x", "\\frac{2}{3}^{x}"),
        ("(-3/2)^x", "\\left(-\\frac{3}{2}\\right)^{x}"),
        ("(x/y)/z", "\\frac{\\frac{x}{y}}{z}"),
        ("x/y/z", "\\frac{\\frac{x}{y}}{z}"),
        ("x/y/z/w", "\\frac{\\frac{\\frac{x}{w}}{y}}{z}"),
        ("(x/y)/(z/w)", "\\frac{\\frac{x}{y}}{\\frac{z}{w}}"),
        ("x/(y*z)", "\\frac{x}{y \\cdot z}"),
        ("(x*y)/(z*w)", "\\frac{x \\cdot y}{w \\cdot z}"),
        ("(x/y)*z", "\\frac{x \\cdot z}{y}"),
        (
            "apart(1/(x^2 - 1), x)",
            "-\\frac{\\frac{1}{2}}{x + 1} + \\frac{\\frac{1}{2}}{x - 1}",
        ),
        ("(x + 1)^12", "\\left(x + 1\\right)^{12}"),
        ("(x + y)^2", "\\left(x + y\\right)^{2}"),
        ("(x^y)^2", "\\left(x^{y}\\right)^{2}"),
        ("x^(y^2)", "x^{y^{2}}"),
        (
            "(x + y)*(x - y)",
            "\\left(x + y\\right) \\cdot \\left(x - y\\right)",
        ),
        ("x*(y + a)", "x \\cdot \\left(a + y\\right)"),
        (
            "-(x + y)*(x - y)",
            "-\\left(x + y\\right) \\cdot \\left(x - y\\right)",
        ),
        ("x/(y + 1)", "\\frac{x}{y + 1}"),
        ("(x + 1)/y", "\\frac{x + 1}{y}"),
        ("x^(-2)", "x^{-2}"),
        ("(x + 1)^(-1)", "\\left(x + 1\\right)^{-1}"),
        (
            "1/(2*(x + 1))",
            "\\left(2 \\cdot \\left(x + 1\\right)\\right)^{-1}",
        ),
        (
            "-1/(2*(x + 1))",
            "-\\frac{1}{2 \\cdot \\left(x + 1\\right)}",
        ),
        ("(1/2)*x + 1/3", "\\frac{1}{2} \\cdot x + \\frac{1}{3}"),
        ("x/2", "\\frac{1}{2} \\cdot x"),
        ("x/(2*y)", "\\frac{x}{2 \\cdot y}"),
        ("exp(-x^2)", "\\exp(-x^{2})"),
        ("sin(x + y) + cos(x^2)", "\\cos(x^{2}) + \\sin(x + y)"),
        ("sqrt(x + y)", "\\sqrt{x + y}"),
        ("(-x)^(1/2)", "\\sqrt{-x}"),
        ("x^(-1/2)", "\\frac{1}{\\sqrt{x}}"),
        ("log(x + 1)/(x - 1)", "\\frac{\\log(x + 1)}{x - 1}"),
        ("sin((x + y)/(x - y))", "\\sin(\\frac{x + y}{x - y})"),
        ("abs(x)", "\\left|x\\right|"),
        (
            "abs(x + y)*(x - 1)",
            "\\left|x + y\\right| \\cdot \\left(x - 1\\right)",
        ),
        ("abs(x - y)^2", "\\left|x - y\\right|^{2}"),
    ];
    for (source, expected) in corpus {
        assert_eq!(
            latex(&format!("latex({source})")),
            *expected,
            "latex({source})"
        );
    }
}

#[test]
fn latex_known_renderings_match_the_oracle_exactly() {
    let cases: &[(&str, &str)] = &[
        ("latex(x^2)", "x^{2}"),
        ("latex(sqrt(x))", "\\sqrt{x}"),
        ("latex((x + 1)^2)", "\\left(x + 1\\right)^{2}"),
        ("latex(x - (y - a))", "x - \\left(y - a\\right)"),
        (
            "latex((x + y)*(x - y))",
            "\\left(x + y\\right) \\cdot \\left(x - y\\right)",
        ),
        ("latex(x^(-2))", "x^{-2}"),
        ("latex(1/(x + 1))", "\\left(x + 1\\right)^{-1}"),
        ("latex(-x^2)", "-x^{2}"),
        ("latex(abs(x))", "\\left|x\\right|"),
        ("latex(x^(y^2))", "x^{y^{2}}"),
        ("latex(sin(x))", "\\sin(x)"),
        ("latex(1/2)", "\\frac{1}{2}"),
        ("latex(3/4)", "\\frac{3}{4}"),
        ("latex(-1/2)", "-\\frac{1}{2}"),
        ("latex(7)", "7"),
        ("latex(2.5)", "\\frac{5}{2}"),
        ("latex(-0.25)", "-\\frac{1}{4}"),
        ("latex(0.1)", "\\frac{1}{10}"),
    ];
    for (source, expected) in cases {
        assert_eq!(latex(source), *expected, "latex({source})");
    }
}

#[test]
fn latex_symbol_name_escaping_matches_the_oracle_exactly() {
    let cases: &[(&str, &str)] = &[
        ("latex(symbol(\"x_1\"))", "x\\_1"),
        ("latex(symbol(\"p%q\"))", "p\\%q"),
        ("latex(symbol(\"a&b\"))", "a\\&b"),
        ("latex(symbol(\"h#1\"))", "h\\#1"),
        ("latex(symbol(\"c{d}\"))", "c\\{d\\}"),
        ("latex(symbol(\"o}e\"))", "o\\}e"),
        ("latex(symbol(\"b\\\\s\"))", "b\\textbackslash{}s"),
        ("latex(symbol(\"e^f\"))", "e\\textasciicircum{}f"),
        ("latex(symbol(\"t~u\"))", "t\\textasciitilde{}u"),
        ("latex(symbol(\"m$n\"))", "m\\$n"),
        ("latex(symbol(\"__\"))", "\\_\\_"),
        ("latex(symbol(\"x_1\")^2)", "x\\_1^{2}"),
        (
            "latex(symbol(\"a%b\") * symbol(\"c&d\"))",
            "a\\%b \\cdot c\\&d",
        ),
        ("latex(symbol(\"x_1\") + symbol(\"x_1\"))", "2 \\cdot x\\_1"),
        ("latex(abs(symbol(\"x_1\")))", "\\left|x\\_1\\right|"),
        ("latex(1/symbol(\"x_1\"))", "x\\_1^{-1}"),
        ("latex(sin(symbol(\"x_1\")))", "\\sin(x\\_1)"),
    ];
    for (source, expected) in cases {
        assert_eq!(latex(source), *expected, "latex({source})");
    }
}

#[test]
fn latex_documented_escapes_are_the_single_characters() {
    let cases: &[(&str, &str)] = &[
        ("latex(symbol(\"#\"))", "\\#"),
        ("latex(symbol(\"$\"))", "\\$"),
        ("latex(symbol(\"%\"))", "\\%"),
        ("latex(symbol(\"&\"))", "\\&"),
        ("latex(symbol(\"_\"))", "\\_"),
        ("latex(symbol(\"{\"))", "\\{"),
        ("latex(symbol(\"}\"))", "\\}"),
        ("latex(symbol(\"~\"))", "\\textasciitilde{}"),
        ("latex(symbol(\"^\"))", "\\textasciicircum{}"),
        ("latex(symbol(\"\\\\\"))", "\\textbackslash{}"),
    ];
    for (source, expected) in cases {
        assert_eq!(latex(source), *expected, "latex({source})");
    }
}

#[test]
fn sqrt_of_negative_values_is_the_principal_complex_root() {
    let cases: &[(&str, &str)] = &[
        ("sqrt(-1)", "i (Symbolic)"),
        ("sqrt(-4)", "2*i (Symbolic)"),
        ("sqrt(-2)", "i*sqrt(2) (Symbolic)"),
        ("sqrt(-3)", "i*sqrt(3) (Symbolic)"),
        ("sqrt(-9)", "3*i (Symbolic)"),
        ("sqrt(-16)", "4*i (Symbolic)"),
        ("sqrt(-1/4)", "1/2*i (Symbolic)"),
        ("sqrt(-4/9)", "2/3*i (Symbolic)"),
        ("sqrt(-0.25)", "1/2*i (Symbolic)"),
        ("sqrt(-2.5)", "i*sqrt(5/2) (Symbolic)"),
        ("sqrt(-1/2)", "i*sqrt(1/2) (Symbolic)"),
        ("sqrt(-1)^2", "-1 (Symbolic)"),
        ("sqrt(-4) + sqrt(-1)", "3*i (Symbolic)"),
        ("sqrt(-4)*sqrt(-9)", "-6 (Symbolic)"),
    ];
    for (source, expected) in cases {
        assert_eq!(display(source), *expected, "{source}");
    }
}

#[test]
fn sqrt_of_non_negative_and_symbolic_values_is_unchanged() {
    assert_eq!(display("sqrt(4)"), "2 (Real)");
    assert_eq!(display("sqrt(0)"), "0 (Real)");
    assert_eq!(display("sqrt(9/4)"), "1.5 (Real)");
    assert_eq!(display("sqrt(x)"), "sqrt(x) (Symbolic)");
    assert_eq!(display("sqrt(-x)"), "sqrt(-x) (Symbolic)");
    assert_eq!(display("sqrt(x^2)"), "sqrt(x^2) (Symbolic)");
    // A user symbol literally named i is NOT the imaginary unit and must not fold.
    assert_eq!(display("i = symbol(\"i\"); i^2"), "i^2 (Symbolic)");
    assert_eq!(
        display("i = symbol(\"i\"); sqrt(2*i)"),
        "sqrt(2*i) (Symbolic)"
    );
}

#[test]
fn nested_power_renderings_round_trip_through_the_parser() {
    let cases: &[(&str, &str)] = &[
        ("(x^y)^2", "(x^y)^2 (Symbolic)"),
        ("x^(y^2)", "x^(y^2) (Symbolic)"),
        ("(x + y)^2", "(x + y)^2 (Symbolic)"),
        ("(x*y)^2", "(x*y)^2 (Symbolic)"),
        ("(-x)^2", "(-x)^2 (Symbolic)"),
        ("(-2)^x", "(-2)^x (Symbolic)"),
        ("(1/2)^x", "(1/2)^x (Symbolic)"),
        // The reference's pretty arm renders a 1/2 exponent as the radical: oracle
        // "x^(1/2)" -> "sqrt(x) (Symbolic)". The canonical form is unchanged, so the
        // re-parse assertion below still holds.
        ("x^(1/2)", "sqrt(x) (Symbolic)"),
    ];
    for (source, expected) in cases {
        assert_eq!(display(source), *expected, "{source}");
        let rendered = expected.trim_end_matches(" (Symbolic)");
        assert_eq!(
            canonical(source),
            canonical(rendered),
            "{source} rendered as {rendered} and re-parsed to a different canonical form"
        );
    }
    assert_eq!(display("2^(3^2)"), "512 (Natural)");
}
