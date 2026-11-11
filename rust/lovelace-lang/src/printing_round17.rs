//! Round 17: counted parity coverage for the Symbolics printing cluster.
//!
//! The three theories the round-16 printing surface unblocked are ported here row by row, with the
//! census case index pinned to the C# theory's own corpus order:
//!
//!   * `PrettyParenthesesTests.CanonicalMode_StillRoundTripsThroughCanonicalParse` — 48 rows
//!     (`Printing.CanonicalParse(CanonicalPrint(e)).CanonicalPrint() == CanonicalPrint(e)` and the
//!     Canonical print mode is `CanonicalPrint`);
//!   * `PrettyParenthesesTests.DebugMode_IsStructuralAndContainsNoPrecedenceParentheses` — 48 rows
//!     (the Debug print mode is `DebugPrint` and the rendering contains no `((`);
//!   * `LatexPrinterTests.CanonicalAndPretty_AreUntouchedByTheLatexArm` — 49 rows (Canonical and
//!     Debug are unchanged, `Print(options).Text` is the LaTeX rendering, and the Canonical text
//!     starts with `(`).
//!
//! The module also carries the adjacent printing/structural rows whose C# assertions the same
//! surface reproduces: the print-budget rows of `Lovelace.Symbolics.Tests` and
//! `Lovelace.Run.Tests` (through the structured projection, this crate's only budgeted rendering),
//! the pretty fraction-coefficient row, the canonical order row, the power-of-a-symbolic-exponent
//! row and one usage example. Every expected string in these tests is the C# assertion; none was
//! read off this crate's output. The mapping is recorded in parity/manifest-map.json with the C#
//! file, line and case index.

use std::cmp::Ordering;

use crate::{Engine, Value, structured};
use lovelace_int::Int;
use lovelace_json::Json;
use lovelace_nat::Nat;
use lovelace_symbolics::{Expr, PrintMode, canonical_order, canonical_parse, expand};

const PRELUDE: &str = "x = symbol(\"x\"); y = symbol(\"y\"); a = symbol(\"a\"); b = symbol(\"b\"); z = symbol(\"z\"); w = symbol(\"w\");\n";

/// `PrettyParenthesesTests.Corpus` in the C# theory-data order: the census case index is the
/// first element, so `CanonicalMode...#N` is the row with index N.
const PRETTY_CORPUS: &[(usize, &str)] = &[
    (0, "(x + 1)^12"),
    (1, "exp(-x^2)"),
    (2, "integrate_full(exp(-x^2), x)"),
    (3, "factor(-x^2 - 2*x - 1)"),
    (4, "x - (y - a)"),
    (5, "x - y - a"),
    (6, "a - (x + y)"),
    (7, "-x^2"),
    (8, "(-x)^2"),
    (9, "(-x)^3"),
    (10, "-(x + y)"),
    (11, "(-x - 1)^2"),
    (12, "(-x)^(1/2)"),
    (13, "(-1)^x"),
    (14, "(-2)^x"),
    (15, "(-1/2)^x"),
    (16, "(-1.5)^x"),
    (17, "(-3)^(-x)"),
    (18, "(1/2)^x"),
    (19, "(2/3)^x"),
    (20, "(-3/2)^x"),
    (21, "(x/y)/z"),
    (22, "x/y/z"),
    (23, "x/y/z/w"),
    (24, "(x/y)/(z/w)"),
    (25, "x/(y*z)"),
    (26, "(x*y)/(z*w)"),
    (27, "(x/y)*z"),
    (28, "apart(1/(x^2 - 1), x)"),
    (29, "(x^y)^2"),
    (30, "x^(y^2)"),
    (31, "(x + y)*(x - y)"),
    (32, "x*(y + a)"),
    (33, "-(x + y)*(x - y)"),
    (34, "x/(y + 1)"),
    (35, "(x + 1)/y"),
    (36, "x^(-2)"),
    (37, "(x + 1)^(-1)"),
    (38, "1/(2*(x + 1))"),
    (39, "-1/(2*(x + 1))"),
    (40, "(1/2)*x + 1/3"),
    (41, "x/2"),
    (42, "exp(-(x + 1)^2)"),
    (43, "sin(x + y) + cos(x^2)"),
    (44, "sqrt(x + y)"),
    (45, "log(x + 1)/(x - 1)"),
    (46, "diff(exp(x)*x^2, x)"),
    (47, "sin((x + y)/(x - y))"),
];

/// `LatexPrinterTests.Corpus` in the C# theory-data order.
const LATEX_CORPUS: &[(usize, &str)] = &[
    (0, "x - (y - a)"),
    (1, "x - y - a"),
    (2, "a - (x + y)"),
    (3, "-x^2"),
    (4, "(-x)^2"),
    (5, "(-x)^3"),
    (6, "-(x + y)"),
    (7, "(-x - 1)^2"),
    (8, "(-1)^x"),
    (9, "(-2)^x"),
    (10, "(-1/2)^x"),
    (11, "(-1.5)^x"),
    (12, "(1/2)^x"),
    (13, "(2/3)^x"),
    (14, "(-3/2)^x"),
    (15, "(x/y)/z"),
    (16, "x/y/z"),
    (17, "x/y/z/w"),
    (18, "(x/y)/(z/w)"),
    (19, "x/(y*z)"),
    (20, "(x*y)/(z*w)"),
    (21, "(x/y)*z"),
    (22, "apart(1/(x^2 - 1), x)"),
    (23, "(x + 1)^12"),
    (24, "(x + y)^2"),
    (25, "(x^y)^2"),
    (26, "x^(y^2)"),
    (27, "(x + y)*(x - y)"),
    (28, "x*(y + a)"),
    (29, "-(x + y)*(x - y)"),
    (30, "x/(y + 1)"),
    (31, "(x + 1)/y"),
    (32, "x^(-2)"),
    (33, "(x + 1)^(-1)"),
    (34, "1/(2*(x + 1))"),
    (35, "-1/(2*(x + 1))"),
    (36, "(1/2)*x + 1/3"),
    (37, "x/2"),
    (38, "x/(2*y)"),
    (39, "exp(-x^2)"),
    (40, "sin(x + y) + cos(x^2)"),
    (41, "sqrt(x + y)"),
    (42, "(-x)^(1/2)"),
    (43, "x^(-1/2)"),
    (44, "log(x + 1)/(x - 1)"),
    (45, "sin((x + y)/(x - y))"),
    (46, "abs(x)"),
    (47, "abs(x + y)*(x - 1)"),
    (48, "abs(x - y)^2"),
];

/// `PrintBudgetTotalityTests.NodeCounts`: the census case index and the node count the script
/// builder emits.
const NODE_COUNTS: &[(usize, usize)] = &[(0, 3), (1, 5), (2, 7), (3, 9), (4, 11), (5, 13)];

fn rational(num: i64, den: u64) -> Expr {
    Expr::rational(&Int::from_i64(num), &Nat::from_u64(den))
}

/// The expression a corpus row denotes, built through the language. `integrate_full` publishes a
/// record; the C# `BuildAsync` renders the record's `expression` field, and so does this helper.
fn symbolic(source: &str) -> Expr {
    let mut engine = Engine::new();
    let value = engine
        .eval(&format!("{PRELUDE}{source}"))
        .unwrap_or_else(|err| panic!("{source} unexpectedly failed: {}", err.message));
    match value {
        Value::Symbolic(expr) => expr,
        Value::Record(_, fields) => match fields.iter().find(|(name, _)| name == "expression") {
            Some((_, Value::Symbolic(expr))) => expr.clone(),
            other => panic!("{source} record expression is {other:?}"),
        },
        other => panic!(
            "{source} produced {} instead of Symbolic",
            other.kind_name()
        ),
    }
}

/// Runs one script and returns its value.
fn script_value(source: &str) -> Value {
    let mut engine = Engine::new();
    engine
        .eval(source)
        .unwrap_or_else(|err| panic!("{source} unexpectedly failed: {}", err.message))
}

/// Projects one value through the structured wire projection, the crate's budgeted rendering.
fn projection(value: &Value, budget: Option<usize>) -> Json {
    let engine = Engine::new();
    structured::project_value(&engine, value, budget)
}

fn projected(expr: &Expr, budget: Option<usize>) -> Json {
    projection(&Value::Symbolic(expr.clone()), budget)
}

fn json_text(json: &Json, key: &str) -> String {
    json.get(key)
        .and_then(Json::as_str)
        .unwrap_or_else(|| panic!("the projection has no string '{key}': {}", json.to_string()))
        .to_string()
}

fn json_number(json: &Json, key: &str) -> usize {
    match json.get(key) {
        Some(Json::Num(text)) => text
            .parse()
            .unwrap_or_else(|_| panic!("'{key}' is not a count: {text}")),
        other => panic!("the projection has no numeric '{key}': {other:?}"),
    }
}

fn json_flag(json: &Json, key: &str) -> bool {
    matches!(json.get(key), Some(Json::Bool(true)))
}

/// The C# `AssertAbbreviationOf`: a strict prefix of the real rendering, terminated by " …".
fn assert_abbreviation_of(full: &str, abridged: &str, what: &str) {
    let prefix = abridged
        .strip_suffix(" …")
        .unwrap_or_else(|| panic!("{what}: '{abridged}' does not end with the ellipsis"));
    assert!(
        full.starts_with(prefix),
        "{what}: '{abridged}' is not a prefix of '{full}'"
    );
    assert!(
        prefix.len() < full.len(),
        "{what}: '{abridged}' abbreviates nothing ('{full}' is {} chars, the prefix is {})",
        full.len(),
        prefix.len()
    );
}

fn is_token_char(c: char) -> bool {
    c.is_alphanumeric() || c == '.'
}

/// The C# `AssertEndsAtATokenBoundary`: the retained body is a strict prefix whose first dropped
/// character is not a token character, so the cut never lands inside a number or identifier.
fn assert_ends_at_a_token_boundary(full: &str, abridged: &str, what: &str) {
    let body = abridged
        .strip_suffix(" …")
        .unwrap_or_else(|| panic!("{what}: '{abridged}' does not end with the ellipsis"));
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
        "{what}: the cut falls between two token characters, so '{abridged}' ends inside the token"
    );
}

// ------------------------------------------------------------------
// PrettyParenthesesTests (48 rows each)
// ------------------------------------------------------------------

/// `CanonicalMode_StillRoundTripsThroughCanonicalParse`
/// (Lovelace.Symbolics.Tests/PrettyParenthesesTests.cs:218).
#[test]
fn prettyparenthesestests_canonicalmode_48_rows() {
    assert_eq!(PRETTY_CORPUS.len(), 48, "the C# theory has 48 corpus rows");
    for &(case, source) in PRETTY_CORPUS {
        let expr = symbolic(source);
        let canonical = expr.canonical();
        let reparsed = canonical_parse(&canonical).unwrap_or_else(|err| {
            panic!("case #{case} {source}: canonical parse of {canonical} failed: {err}")
        });
        assert_eq!(
            reparsed.canonical(),
            canonical,
            "case #{case} {source}: CanonicalPrint(CanonicalParse(text)) is not the same canonical text"
        );
        assert_eq!(
            expr.print(PrintMode::Canonical),
            canonical,
            "case #{case} {source}: the Canonical mode is not CanonicalPrint"
        );
    }
}

/// `DebugMode_IsStructuralAndContainsNoPrecedenceParentheses`
/// (Lovelace.Symbolics.Tests/PrettyParenthesesTests.cs:229).
#[test]
fn prettyparenthesestests_debugmode_48_rows() {
    assert_eq!(PRETTY_CORPUS.len(), 48, "the C# theory has 48 corpus rows");
    for &(case, source) in PRETTY_CORPUS {
        let expr = symbolic(source);
        let debug = expr.print(PrintMode::Debug);
        assert_eq!(
            debug,
            expr.debug_print(),
            "case #{case} {source}: the Debug mode is not DebugPrint"
        );
        assert!(
            !debug.contains("(("),
            "case #{case} {source}: the debug rendering contains a doubled parenthesis pair: {debug}"
        );
    }
}

// ------------------------------------------------------------------
// LatexPrinterTests (49 rows)
// ------------------------------------------------------------------

/// `CanonicalAndPretty_AreUntouchedByTheLatexArm`
/// (Lovelace.Symbolics.Tests/LatexPrinterTests.cs:312).
#[test]
fn latexprintertests_canonicalandpretty_49_rows() {
    assert_eq!(LATEX_CORPUS.len(), 49, "the C# theory has 49 corpus rows");
    for &(case, source) in LATEX_CORPUS {
        let expr = symbolic(source);
        let canonical = expr.canonical();
        assert_eq!(
            expr.print(PrintMode::Canonical),
            canonical,
            "case #{case} {source}: the Canonical mode is not CanonicalPrint"
        );
        assert_eq!(
            expr.print(PrintMode::Debug),
            expr.debug_print(),
            "case #{case} {source}: the Debug mode is not DebugPrint"
        );
        assert_eq!(
            expr.print(PrintMode::Latex),
            expr.latex(),
            "case #{case} {source}: the unbudgeted Print under Latex is not the LaTeX rendering"
        );
        assert!(
            expr.print(PrintMode::Canonical).starts_with('('),
            "case #{case} {source}: the Canonical rendering does not start with '('"
        );
    }
}

// ------------------------------------------------------------------
// Lovelace.Run.Tests print-budget rows (wire contract)
// ------------------------------------------------------------------

/// `PrintBudgetTotalityTests.EveryNodeCountOverBudget_IsAbbreviatedAndReportsIt`
/// (Lovelace.Run.Tests/PrintBudgetTotalityTests.cs:54). The six theory rows are the six node
/// counts in `NodeCounts`.
#[test]
fn run_printbudgettotalitytests_every_node_count_over_budget_6_rows() {
    for &(case, nodes) in NODE_COUNTS {
        let script = sum_script(nodes);
        let full = projection(&script_value(&script), None);
        assert_eq!(
            json_number(&full, "nodeCount"),
            nodes,
            "case #{case}: the script builds exactly {nodes} nodes"
        );
        let full_pretty = json_text(&full, "pretty");
        let full_canonical = json_text(&full, "canonical");

        for budget in 1..nodes {
            let structured = projection(&script_value(&script), Some(budget));
            assert!(
                json_flag(&structured, "truncated"),
                "case #{case} nodeCount {nodes} budget {budget}: an over-budget value crossed as if it were whole"
            );
            assert_eq!(
                json_text(&structured, "truncationReason"),
                "node-budget",
                "case #{case} nodeCount {nodes} budget {budget}: the reason"
            );
            assert_eq!(
                json_number(&structured, "budget"),
                budget,
                "case #{case} nodeCount {nodes}: the budget that stopped the rendering"
            );
            assert_abbreviation_of(
                &full_pretty,
                &json_text(&structured, "pretty"),
                &format!("case #{case} nodeCount {nodes} budget {budget} pretty"),
            );
            assert_abbreviation_of(
                &full_canonical,
                &json_text(&structured, "canonical"),
                &format!("case #{case} nodeCount {nodes} budget {budget} canonical"),
            );
        }

        for budget in [nodes, nodes + 2] {
            let structured = projection(&script_value(&script), Some(budget));
            assert!(
                structured.get("truncated").is_none(),
                "case #{case} nodeCount {nodes} budget {budget}: a whole rendering carries a truncation flag"
            );
            assert!(
                structured.get("truncationReason").is_none(),
                "case #{case} nodeCount {nodes} budget {budget}: a whole rendering carries a reason"
            );
            assert!(
                structured.get("budget").is_none(),
                "case #{case} nodeCount {nodes} budget {budget}: a whole rendering carries a budget"
            );
            assert_eq!(json_text(&structured, "pretty"), full_pretty);
            assert_eq!(json_text(&structured, "canonical"), full_canonical);
        }
    }
}

/// `PrintBudgetTotalityTests.SmallValueOverASmallBudget_IsAbbreviated`
/// (Lovelace.Run.Tests/PrintBudgetTotalityTests.cs:108).
#[test]
fn run_printbudgettotalitytests_small_value_over_a_small_budget() {
    let structured = projection(&script_value("x = symbol(\"x\"); x^2"), Some(1));
    assert_eq!(json_number(&structured, "nodeCount"), 3);
    assert!(
        json_flag(&structured, "truncated"),
        "x^2 (3 nodes) at budget 1 crossed with no truncation flag"
    );
    assert_eq!(json_text(&structured, "truncationReason"), "node-budget");
    assert_eq!(json_number(&structured, "budget"), 1);
    let canonical = json_text(&structured, "canonical");
    assert!(canonical.starts_with("(pow "), "canonical: {canonical}");
    assert!(canonical.ends_with(" …"), "canonical: {canonical}");
}

/// `PrintBudgetTokenBoundaryTests.AnIdentifierLongerThanTheProjectionsAllowance_IsNeverSplit`
/// (Lovelace.Run.Tests/PrintBudgetTokenBoundaryTests.cs:85).
#[test]
fn run_printbudgettokenboundarytests_identifier_longer_than_the_projections_allowance() {
    let name = "b".repeat(40);
    let script = format!("x = symbol(\"x\"); {name} = symbol(\"{name}\"); {name} + x");
    let full = projection(&script_value(&script), None);
    let abridged = projection(&script_value(&script), Some(1));
    assert!(
        json_flag(&abridged, "truncated"),
        "the abbreviation did not say so"
    );
    assert_eq!(json_text(&abridged, "truncationReason"), "node-budget");
    assert_ends_at_a_token_boundary(
        &json_text(&full, "pretty"),
        &json_text(&abridged, "pretty"),
        "pretty",
    );
    assert_ends_at_a_token_boundary(
        &json_text(&full, "canonical"),
        &json_text(&abridged, "canonical"),
        "canonical",
    );
}

/// The script `SumScript(nodes)`: `nodes − 1` distinct symbols and their sum, so exactly
/// `nodes` nodes.
fn sum_script(nodes: usize) -> String {
    let terms = nodes - 1;
    let mut script = String::new();
    for i in 0..terms {
        script.push_str(&format!("s{i} = symbol(\"s{i}\"); "));
    }
    script.push_str(
        &(0..terms)
            .map(|i| format!("s{i}"))
            .collect::<Vec<String>>()
            .join(" + "),
    );
    script
}

// ------------------------------------------------------------------
// Lovelace.Symbolics.Tests print budgets, through the structured projection
// ------------------------------------------------------------------

/// `(x + 1)^power` expanded, the C# `Algebra.Expand` the budget tests are stated over.
fn big_expand(power: i64) -> Expr {
    expand(&Expr::pow(
        Expr::add(Expr::sym("x"), Expr::num(1)),
        Expr::num(power),
    ))
}

/// `PrintBudgetTests.NoBudget_IsExactlyTheUnboundedRendering`
/// (Lovelace.Symbolics.Tests/PrintBudgetTests.cs:23).
#[test]
fn printbudgettests_no_budget_is_exactly_the_unbounded_rendering() {
    let big = big_expand(12);
    let outcome = projected(&big, None);
    assert!(
        !json_flag(&outcome, "truncated"),
        "an unbudgeted rendering was abbreviated"
    );
    assert!(
        outcome.get("truncationReason").is_none(),
        "an unbudgeted rendering carries a truncation"
    );
    assert_eq!(json_text(&outcome, "pretty"), big.to_string());
}

/// `PrintBudgetTests.BudgetLargerThanTheExpression_ChangesNothing`
/// (Lovelace.Symbolics.Tests/PrintBudgetTests.cs:33).
#[test]
fn printbudgettests_budget_larger_than_the_expression_changes_nothing() {
    let big = big_expand(12);
    let outcome = projected(&big, Some(100_000));
    assert!(!json_flag(&outcome, "truncated"));
    assert_eq!(json_text(&outcome, "pretty"), big.to_string());
}

/// `PrintBudgetTests.NodeBudget_ReportsReasonCountBudgetAndPartialResult`
/// (Lovelace.Symbolics.Tests/PrintBudgetTests.cs:42).
#[test]
fn printbudgettests_node_budget_reports_reason_count_budget_and_partial_result() {
    let big = big_expand(20);
    let full = big.to_string();
    let outcome = projected(&big, Some(20));
    assert!(
        json_flag(&outcome, "truncated"),
        "the value was not abbreviated"
    );
    assert_eq!(json_text(&outcome, "truncationReason"), "node-budget");
    assert_eq!(json_number(&outcome, "nodeCount"), big.node_count());
    assert_eq!(json_number(&outcome, "budget"), 20);
    assert!(big.node_count() > 20, "the corpus value is over the budget");
    assert_abbreviation_of(
        &full,
        &json_text(&outcome, "pretty"),
        "the pretty form of (x + 1)^20",
    );
}

/// `PrintBudgetTests.CanonicalMode_HonoursTheSameBudget`
/// (Lovelace.Symbolics.Tests/PrintBudgetTests.cs:83).
#[test]
fn printbudgettests_canonical_mode_honours_the_same_budget() {
    let big = big_expand(20);
    let outcome = projected(&big, Some(20));
    assert!(
        json_flag(&outcome, "truncated"),
        "the canonical value was not abbreviated"
    );
    assert!(
        json_text(&outcome, "canonical").starts_with("(add "),
        "canonical: {}",
        json_text(&outcome, "canonical")
    );
    assert_eq!(json_text(&outcome, "truncationReason"), "node-budget");
}

/// `PrintBudgetTests.Abbreviation_NeverSplitsAToken`
/// (Lovelace.Symbolics.Tests/PrintBudgetTests.cs:94).
#[test]
fn printbudgettests_abbreviation_never_splits_a_token() {
    let big = big_expand(20);
    let outcome = projected(&big, Some(20));
    let text = json_text(&outcome, "pretty");
    let body = text
        .strip_suffix(" …")
        .unwrap_or_else(|| panic!("'{text}' does not end with the ellipsis"));
    let last = body.chars().last().expect("a retained body");
    assert!(
        !is_token_char(last),
        "the retained text ends inside a token: …'{}'",
        &body[body.len().saturating_sub(12)..]
    );
    assert!(
        big.to_string().starts_with(body),
        "'{body}' is not a prefix of the pretty rendering"
    );
}

/// `PrintBudgetTokenBoundaryTests.TheCanonicalFormOfTheSameValue_AlsoEndsAtABoundary`
/// (Lovelace.Symbolics.Tests/PrintBudgetTokenBoundaryTests.cs:89).
#[test]
fn printbudgettokenboundarytests_canonical_form_of_the_same_value_ends_at_a_boundary() {
    let name = "a".repeat(54);
    let expr = Expr::add(Expr::sym(&name), Expr::num(1));
    let full = projected(&expr, None);
    let abridged = projected(&expr, Some(1));
    assert_ends_at_a_token_boundary(
        &json_text(&full, "canonical"),
        &json_text(&abridged, "canonical"),
        "the canonical form of a 54-character symbol",
    );
    assert!(
        json_text(&abridged, "canonical").starts_with("(add (rat 1 1) (sym "),
        "canonical: {}",
        json_text(&abridged, "canonical")
    );
}

/// `PrintBudgetTokenBoundaryTests.ARenderingThatIsOneLongToken_RetainsNothing`
/// (Lovelace.Symbolics.Tests/PrintBudgetTokenBoundaryTests.cs:123).
#[test]
fn printbudgettokenboundarytests_a_rendering_that_is_one_long_token_retains_nothing() {
    let name = "a".repeat(54);
    let expr = Expr::sym(&name);
    let full = projected(&expr, None);
    assert_eq!(json_text(&full, "pretty"), name);
    let outcome = projected(&expr, Some(0));
    assert!(
        json_flag(&outcome, "truncated"),
        "a value the budget stopped must say so"
    );
    assert_eq!(json_text(&outcome, "pretty"), "…");
    assert_eq!(json_text(&outcome, "truncationReason"), "node-budget");
}

// ------------------------------------------------------------------
// Adjacent printing / structural rows
// ------------------------------------------------------------------

/// `DxSemanticClosureTests.Pretty_FractionCoefficient_FoldsIntoDenominator`
/// (Lovelace.Symbolics.Tests/DxSemanticClosureTests.cs:427).
#[test]
fn dxsemanticclosuretests_pretty_fraction_coefficient_folds_into_denominator() {
    let x = Expr::sym("x");
    let term = Expr::mul(
        rational(-1, 2),
        Expr::pow(Expr::add(x.clone(), Expr::num(1)), Expr::num(-1)),
    );
    assert_eq!(term.to_string(), "-1/2/(x + 1)");
    assert_eq!(Expr::div(x.clone(), x.clone()).to_string(), "x/x");
    assert_eq!(Expr::div(Expr::num(2), x).to_string(), "2/x");
}

/// `UsageExamples.Snippet_16_1_ContextSymbolsAndConstruction`
/// (Lovelace.Symbolics.Tests/UsageExamples.cs:16).
#[test]
fn usageexamples_snippet_16_1_context_symbols_and_construction() {
    let x = Expr::sym("x");
    let f = Expr::add(
        Expr::add(
            Expr::pow(x.clone(), Expr::num(2)),
            Expr::mul(Expr::num(2), x),
        ),
        Expr::num(1),
    );
    assert_eq!(f.to_string(), "x^2 + 2*x + 1");
}

/// `KernelHardeningTests.PowerOfSymbolicExponent_PreservesExponent`
/// (Lovelace.Symbolics.Tests/KernelHardeningTests.cs:31).
#[test]
fn kernelhardeningtests_power_of_symbolic_exponent_preserves_exponent() {
    let x = Expr::sym("x");
    let y = Expr::sym("y");
    let e = Expr::pow(Expr::pow(x, y.clone()), Expr::num(2));
    match &e {
        Expr::Pow(base, exponent) => {
            assert_eq!(**exponent, Expr::num(2), "the outer exponent folded");
            match &**base {
                Expr::Pow(_, inner) => {
                    assert_eq!(**inner, y, "the symbolic exponent was lost");
                }
                other => panic!("the base is not a power: {other:?}"),
            }
        }
        other => panic!("the value is not a power: {other:?}"),
    }
    assert_eq!(e.to_string(), "(x^y)^2");
}

/// `AcceptanceTests.TermOrder_IsAntisymmetricAndNumbersFirst`
/// (Lovelace.Symbolics.Tests/SmokeTests.cs:253).
#[test]
fn acceptancetests_term_order_is_antisymmetric_and_numbers_first() {
    let x = Expr::sym("x");
    let y = Expr::sym("y");
    let items = vec![
        Expr::num(3),
        rational(1, 2),
        x.clone(),
        y.clone(),
        Expr::pow(x.clone(), Expr::num(2)),
        Expr::mul(x.clone(), y.clone()),
        Expr::add(x.clone(), y.clone()),
        Expr::Fun("sin".to_string(), vec![x]),
        Expr::Fun("pi".to_string(), vec![]),
    ];
    for i in 0..items.len() {
        for j in 0..items.len() {
            let cmp = canonical_order(&items[i], &items[j]);
            if i == j {
                assert_eq!(cmp, Ordering::Equal, "term order is not reflexive at {i}");
            } else {
                assert_eq!(
                    cmp,
                    canonical_order(&items[j], &items[i]).reverse(),
                    "term order is not antisymmetric at {i},{j}"
                );
            }
            if matches!(items[i], Expr::Num(_)) && !matches!(items[j], Expr::Num(_)) {
                assert_eq!(
                    cmp,
                    Ordering::Less,
                    "a numeric constant did not sort first at {i},{j}"
                );
            }
        }
    }
}
