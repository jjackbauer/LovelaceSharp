//! Round 16: the Symbolics printing cluster — canonical parse, Debug print and print-mode
//! dispatch.
//!
//! The reference contract is Lovelace.Symbolics.Tests/PrettyParenthesesTests
//! (CanonicalMode_StillRoundTripsThroughCanonicalParse and
//! DebugMode_IsStructuralAndContainsNoPrecedenceParentheses) plus
//! LatexPrinterTests/CanonicalAndPretty_AreUntouchedByTheLatexArm. The C# assertions are:
//!   * Printing.CanonicalParse(Printing.CanonicalPrint(e), ctx) reprints the SAME canonical text;
//!   * Printing.PrettyPrint(e, PrintOptions(Debug)) == Printing.DebugPrint(e), and the debug text
//!     never contains "((";
//!   * Printing.PrettyPrint(e, PrintOptions(Canonical)) == Printing.CanonicalPrint(e) and
//!     Printing.Print(e, options) routes the same way.
//!
//! Every expected STRING in this module was produced by the pinned C# oracle (a small probe
//! program that calls the reference Printing class on the same expressions), not read off this
//! crate's output.

use lovelace_int::Int;
use lovelace_nat::Nat;

use crate::{
    canonical_parse, canonical_order, Expr, PrintMode, RelOp, UnknownPrintMode,
};

fn rational(num: i64, den: u64) -> Expr {
    Expr::rational(&Int::from_i64(num), &Nat::from_u64(den))
}

fn x() -> Expr {
    Expr::sym("x")
}

fn y() -> Expr {
    Expr::sym("y")
}

fn a() -> Expr {
    Expr::sym("a")
}

fn w() -> Expr {
    Expr::sym("w")
}

fn z() -> Expr {
    Expr::sym("z")
}

fn named(name: &str) -> Expr {
    Expr::Fun(name.to_string(), Vec::new())
}

fn call(name: &str, args: Vec<Expr>) -> Expr {
    Expr::Fun(name.to_string(), args)
}

/// The expression shapes the C# corpus covers: constants, sums, products, powers, rationals,
/// functions (sin/exp/log/sqrt), nested forms, relations, named constants and a root.
fn corpus() -> Vec<Expr> {
    let exp = |e: Expr| call("exp", vec![e]);
    let sin = |e: Expr| call("sin", vec![e]);
    let log = |e: Expr| call("log", vec![e]);
    vec![
        Expr::num(0),
        Expr::num(42),
        rational(-1, 2),
        rational(2, 3),
        x(),
        Expr::sym("alpha_1"),
        Expr::add(x(), Expr::num(1)),
        Expr::sub(x(), Expr::num(1)),
        Expr::add(Expr::add(x(), y()), a()),
        Expr::mul(x(), y()),
        Expr::mul(Expr::mul(Expr::num(2), x()), exp(x())),
        Expr::pow(x(), Expr::num(-2)),
        Expr::pow(Expr::add(x(), Expr::num(1)), Expr::num(12)),
        Expr::pow(Expr::mul(Expr::num(-1), x()), rational(1, 2)),
        Expr::sqrt(Expr::add(x(), y())),
        sin(Expr::add(x(), y())),
        exp(Expr::mul(Expr::num(-1), Expr::pow(x(), Expr::num(2)))),
        Expr::div(log(Expr::add(x(), Expr::num(1))), Expr::sub(x(), Expr::num(1))),
        sin(Expr::div(Expr::add(x(), y()), Expr::sub(x(), y()))),
        named("pi"),
        named("e"),
        named("i"),
        named("inf"),
        Expr::relation(RelOp::Lt, x(), Expr::num(1)),
        Expr::relation(RelOp::Eq, x(), Expr::add(y(), Expr::num(1))),
        call(
            "rootof",
            vec![
                Expr::sub(Expr::pow(x(), Expr::num(2)), Expr::num(2)),
                Expr::num(0),
            ],
        ),
        call("f", Vec::new()),
        call("f", vec![x(), y()]),
        call("and", vec![x(), y()]),
        call("or", vec![x(), y()]),
        call("not", vec![x()]),
        Expr::Div(Box::new(Expr::pow(y(), Expr::num(2))), Box::new(x())),
        Expr::Div(Box::new(Expr::num(0)), Box::new(Expr::mul(x(), y()))),
    ]
}

#[test]
fn canonical_parse_reprints_every_expression_shape_identically() {
    for expr in corpus() {
        let text = expr.canonical();
        let reparsed = canonical_parse(&text)
            .unwrap_or_else(|e| panic!("canonical parse failed for {text}: {e}"));
        assert_eq!(
            reparsed.canonical(),
            text,
            "canonical text did not round-trip for {text}"
        );
    }
}

#[test]
fn canonical_parse_rebuilds_leaf_values_exactly() {
    // Leaf values are rebuilt exactly. Sum and product nodes are stored in the constructor's
    // display order, while the canonical text is sorted; those two orders differ, which is why
    // the C# contract compares canonical TEXT (the identity) rather than object identity.
    for expr in [Expr::num(42), rational(-1, 2), x(), named("pi"), named("inf")] {
        let text = expr.canonical();
        let reparsed = canonical_parse(&text).expect("canonical parse");
        assert_eq!(reparsed, expr, "parse changed the leaf built for {text}");
    }
    // A quotient's canonical form is the reference's multiply-by-reciprocal shape, so that shape
    // is the value a parse reconstructs.
    let quotient = Expr::Div(Box::new(x()), Box::new(y()));
    let reparsed = canonical_parse(&quotient.canonical()).expect("quotient parse");
    assert_eq!(
        reparsed,
        Expr::mul(x(), Expr::pow(y(), Expr::num(-1))),
        "a quotient parses to its canonical multiply-by-reciprocal shape"
    );
}

#[test]
fn canonical_parse_accepts_the_oracle_int_alias_and_normalises_it() {
    assert_eq!(canonical_parse("(int 5)").unwrap().canonical(), "(rat 5 1)");
    assert_eq!(canonical_parse("(int -3)").unwrap().canonical(), "(rat -3 1)");
    assert_eq!(
        canonical_parse("(add (int 1) (sym x))").unwrap().canonical(),
        "(add (rat 1 1) (sym x))"
    );
}

#[test]
fn canonical_parse_refuses_malformed_text_with_a_position() {
    let cases: &[(&str, &str)] = &[
        ("", "unexpected end of input"),
        ("x", "expected '('"),
        ("(sym x", "unexpected end of input"),
        ("(sym x) trailing", "trailing tokens"),
        ("(frob x)", "unknown tag 'frob'"),
        ("(rat 1 0)", "denominator must not be zero"),
        ("(rat x 1)", "integer numerator"),
        ("(rat 1 -2)", "non-negative integer denominator"),
        ("(pow (sym x))", "expected '('"),
        ("(real 1.5)", "numeric literal"),
        ("(integ (x) (sym x))", "not part of this crate's canonical vocabulary"),
        ("(order (sym x) (rat 0 1) (rat 1 1))", "not part of this crate's canonical vocabulary"),
    ];
    for (text, needle) in cases {
        let err = canonical_parse(text).expect_err(text);
        assert!(
            err.message.contains(needle),
            "refusal for {text:?} was {:?}, expected it to contain {needle:?}",
            err.message
        );
    }
    let err = canonical_parse("(sym x").unwrap_err();
    assert_eq!(err.position, 6, "position of the end of input");
}

#[test]
fn debug_print_matches_the_reference_shapes() {
    // The expected strings are the reference Printing.DebugPrint output captured by the oracle
    // probe for the same expressions.
    let cases: &[(Expr, &str)] = &[
        (Expr::num(1), "rat(1)"),
        (rational(-1, 2), "rat(-1/2)"),
        (x(), "sym(x)"),
        (Expr::add(x(), Expr::num(1)), "add[rat(1), sym(x)]"),
        (Expr::mul(Expr::num(2), x()), "mul[rat(2), sym(x)]"),
        (Expr::pow(x(), Expr::num(2)), "pow[sym(x), rat(2)]"),
        (call("sin", vec![x()]), "fn:sin(sym(x))"),
        (named("pi"), "named(Pi)"),
        (named("e"), "named(E)"),
        (named("i"), "named(I)"),
        (named("inf"), "named(Infinity)"),
        (
            Expr::relation(RelOp::Lt, x(), Expr::num(1)),
            "rel:Lt(sym(x), rat(1))",
        ),
        (
            Expr::Div(Box::new(x()), Box::new(y())),
            "mul[sym(x), pow[sym(y), rat(-1)]]",
        ),
        (
            call("and", vec![x(), y()]),
            "and[sym(x), sym(y)]",
        ),
        (call("not", vec![x()]), "not[sym(x)]"),
        (
            Expr::pow(Expr::add(x(), Expr::num(1)), Expr::num(12)),
            "pow[add[rat(1), sym(x)], rat(12)]",
        ),
        (
            Expr::sub(x(), Expr::sub(y(), a())),
            "add[sym(x), mul[rat(-1), add[sym(y), mul[rat(-1), sym(a)]]]]",
        ),
        (
            Expr::div(Expr::div(x(), y()), z()),
            "mul[sym(x), pow[sym(y), rat(-1)], pow[sym(z), rat(-1)]]",
        ),
        (
            Expr::div(x(), Expr::mul(y(), z())),
            "mul[sym(x), pow[mul[sym(y), sym(z)], rat(-1)]]",
        ),
        (
            Expr::div(Expr::num(1), Expr::mul(Expr::num(2), Expr::add(x(), Expr::num(1)))),
            "pow[mul[rat(2), add[rat(1), sym(x)]], rat(-1)]",
        ),
        (
            Expr::add(
                Expr::mul(rational(1, 2), x()),
                rational(1, 3),
            ),
            "add[rat(1/3), mul[rat(1/2), sym(x)]]",
        ),
        (
            Expr::add(call("sin", vec![Expr::add(x(), y())]), call("cos", vec![Expr::pow(x(), Expr::num(2))])),
            "add[fn:cos(pow[sym(x), rat(2)]), fn:sin(add[sym(x), sym(y)])]",
        ),
        (
            Expr::sqrt(Expr::add(x(), y())),
            "pow[add[sym(x), sym(y)], rat(1/2)]",
        ),
        (
            call("rootof", vec![Expr::sub(Expr::pow(x(), Expr::num(2)), Expr::num(2)), Expr::num(0)]),
            "rootof[add[rat(-2), pow[sym(x), rat(2)]], 0]",
        ),
    ];
    for (expr, expected) in cases {
        assert_eq!(expr.debug_print(), *expected, "debug shape of {}", expr.canonical());
        assert_eq!(expr.print(PrintMode::Debug), *expected);
    }
}

#[test]
fn debug_print_visits_operands_in_canonical_order_not_display_order() {
    // Expr::add stores x before 1 (the display order is degree first); the reference expression
    // stores the constant first (its canonical order). Debug must print the reference order.
    let sum = Expr::add(x(), Expr::num(1));
    assert_eq!(sum.to_string(), "x + 1");
    assert_eq!(sum.canonical(), "(add (rat 1 1) (sym x))");
    assert_eq!(sum.debug_print(), "add[rat(1), sym(x)]");

    let product = Expr::mul(x(), Expr::num(2));
    assert_eq!(product.to_string(), "2*x");
    assert_eq!(product.canonical(), "(mul (rat 2 1) (sym x))");
    assert_eq!(product.debug_print(), "mul[rat(2), sym(x)]");

    let mut left = Expr::add(x(), Expr::num(1));
    left = Expr::mul(left, y());
    left = Expr::pow(left, Expr::num(2));
    assert_eq!(left.debug_print(), "pow[mul[sym(y), add[rat(1), sym(x)]], rat(2)]");
}

#[test]
fn debug_print_contains_no_doubled_parenthesis_over_the_corpus() {
    for expr in corpus() {
        let debug = expr.debug_print();
        assert!(
            !debug.contains("(("),
            "debug rendering contains '((', which the C# test forbids: {debug}"
        );
    }
}

#[test]
fn print_dispatches_every_mode_and_the_dispatch_is_case_insensitive() {
    let expr = Expr::add(Expr::pow(x(), Expr::num(2)), Expr::num(1));
    assert_eq!(expr.print(PrintMode::Pretty), expr.to_string());
    assert_eq!(expr.print(PrintMode::Canonical), expr.canonical());
    assert_eq!(expr.print(PrintMode::Latex), expr.latex());
    assert_eq!(expr.print(PrintMode::Debug), expr.debug_print());

    for name in ["pretty", "canonical", "latex", "debug"] {
        let mode = PrintMode::from_name(name).expect(name);
        assert_eq!(mode.name(), name);
        assert_eq!(expr.print_named(name).unwrap(), expr.print(mode));
        let upper = name.to_ascii_uppercase();
        assert_eq!(PrintMode::from_name(&upper).unwrap(), mode, "case-insensitive {upper}");
    }
}

#[test]
fn unknown_print_mode_is_refused_and_names_every_accepted_mode() {
    let err = PrintMode::from_name("wibble").unwrap_err();
    assert_eq!(
        err,
        UnknownPrintMode { name: "wibble".to_string() }
    );
    assert_eq!(
        err.to_string(),
        "unknown print mode 'wibble'; expected one of: pretty, canonical, latex, debug"
    );
    assert!(PrintMode::from_name("").is_err());
    assert!(PrintMode::from_name("Debug ").is_err(), "no trimming: the name is exact");
}

#[test]
fn canonical_order_is_the_same_order_debug_and_canonical_use() {
    // The two renderings choose the same operand order; a divergence here would let Debug and
    // Canonical describe different trees.
    let sum = Expr::add(Expr::add(x(), y()), Expr::num(1));
    let mut terms = vec![y(), Expr::num(1), x()];
    terms.sort_by(canonical_order);
    let ordered = Expr::Add(terms.clone());
    assert_eq!(
        sum.canonical(),
        ordered.canonical(),
        "canonical text is ordered by canonical_order"
    );
    assert_eq!(sum.debug_print(), ordered.debug_print());
    let _ = w();
}
