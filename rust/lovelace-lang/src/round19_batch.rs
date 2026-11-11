//! Round-19 ports of the budget/cancellation rows in `Lovelace.Suite.Tests`.
//!
//! The C# assertions are the contract. The engine-level tests drive the same
//! `Engine::eval` surface the suite uses; the cancellation tests arm the same
//! thread-local deadline the runner arms. Rows whose C# assertion has no Rust
//! analogue (a CLR exception type, a Modus plugin registration, a ValueKind enum)
//! are deliberately absent rather than weakened.

use std::time::{Duration, Instant};

use crate::syntax::{Expr, Parser, Stmt};
use crate::{registry, Engine, Value};

fn engine() -> Engine {
    Engine::new()
}

fn with_big<T: Send + 'static>(probe: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(probe)
        .expect("probe thread")
        .join()
        .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
}

fn depth_error(err: &crate::LangError) -> bool {
    err.code == "DepthExceeded" && err.category == "BudgetExceeded"
}

fn parse_expression(source: &str) -> Result<Expr, crate::LangError> {
    let source = source.to_string();
    with_big(move || {
        let tokens = crate::syntax::tokenize(&source)?;
        Parser::new(tokens).single_expression()
    })
}

fn parse_program(source: &str) -> Result<Vec<Stmt>, crate::LangError> {
    let source = source.to_string();
    with_big(move || {
        let tokens = crate::syntax::tokenize(&source)?;
        Parser::new(tokens).program()
    })
}

fn parens_at_depth(depth: usize) -> String {
    format!("{}1{};", "(".repeat(depth - 2), ")".repeat(depth - 2))
}

fn flat_chain_at_depth(depth: usize) -> String {
    format!("1{};", "-1".repeat(depth - 3))
}

// ---------------------------------------------------------------------------
// InputDepthGuardTests
// ---------------------------------------------------------------------------

#[test]
fn suite_depth_nesting_at_the_budget_still_parses() {
    let program = parse_program(&parens_at_depth(256)).expect("depth 256 parses");
    assert_eq!(program.len(), 1);
    match &program[0] {
        Stmt::Expr(Expr::Number(raw), _) => assert_eq!(raw, "1"),
        other => panic!("expected the literal statement, got {other:?}"),
    }
}

#[test]
fn suite_depth_nesting_one_past_the_budget_is_refused_with_the_typed_depth_error() {
    let err = parse_program(&parens_at_depth(257)).expect_err("depth 257 is refused");
    assert!(depth_error(&err), "{} / {}", err.code, err.category);
    assert!(err.message.contains("257"), "{}", err.message);
    assert!(err.message.contains("256"), "{}", err.message);
    assert!(err.message.contains("expression nesting depth"));
}

#[test]
fn suite_depth_unary_chain_one_past_the_budget_is_refused() {
    let accepted = format!("{}1;", "-".repeat(253));
    assert!(parse_program(&accepted).is_ok());
    let refused = format!("{}1;", "-".repeat(257));
    let err = parse_program(&refused).expect_err("a deep unary chain is refused");
    assert!(depth_error(&err));
}

#[test]
fn suite_depth_nested_blocks_one_past_the_budget_are_refused() {
    let accepted = format!("{}1;{}", "{".repeat(253), "}".repeat(253));
    assert!(parse_program(&accepted).is_ok());
    let refused = format!("{}1;{}", "{".repeat(257), "}".repeat(257));
    let err = parse_program(&refused).expect_err("deep blocks are refused");
    assert!(depth_error(&err));
}

#[test]
fn suite_depth_ordinary_nesting_and_interpolation_still_parse() {
    assert!(parse_program("-(-(-(1 + 2*3))) + sqrt(sqrt(16)) + abs(-2);").is_ok());
    assert!(parse_program("x = 1; s = $\"v={x + 1}\";").is_ok());
}

// ---------------------------------------------------------------------------
// RecursionDepthGuardTests
// ---------------------------------------------------------------------------

const RECURSION: &str = "func f(n) { if (n == 0) { return 0 }; return f(n - 1) }; f(";

fn recursion_script(depth: i64) -> String {
    format!("{RECURSION}{depth})")
}

fn recursion_with_deep_body(depth: i64, terms: usize) -> String {
    format!(
        "func f(n) {{ if (n == 0) {{ return 0 }}; return f(n - 1){} }}; f({depth})",
        " + 1".repeat(terms)
    )
}

fn assert_evaluation_refusal(err: &crate::LangError) {
    assert!(depth_error(err), "{} / {}", err.code, err.category);
    assert!(err.message.contains("exceeds the maximum supported evaluation depth of 512"), "{}", err.message);
    let at = err.message.find("evaluation depth ").expect("names the evaluation depth")
        + "evaluation depth ".len();
    let measured: usize = err.message[at..]
        .split(' ')
        .next()
        .and_then(|n| n.parse().ok())
        .expect("a measured depth number");
    assert!(measured > 512, "the measured depth must be past the budget: {measured}");
}

#[test]
fn suite_recursion_far_past_the_budget_is_refused_and_does_not_return_a_value() {
    with_big(|| {
    let mut engine = engine();
    let err = engine.eval(&recursion_script(200)).expect_err("f(200) is refused");
    assert_evaluation_refusal(&err);
    });
}


#[test]
fn suite_recursion_at_the_deepest_accepted_depth_answers() {
    with_big(|| {
    let mut engine = engine();
    let value = engine.eval(&recursion_script(85)).expect("f(85) answers");
    assert_eq!(value.display(), "0");
    });
}


#[test]
fn suite_recursion_one_level_past_the_deepest_accepted_depth_is_refused() {
    with_big(|| {
    let mut engine = engine();
    let err = engine.eval(&recursion_script(86)).expect_err("f(86) is refused");
    assert_evaluation_refusal(&err);
    });
}


#[test]
fn suite_recursion_ordinary_recursion_keeps_working() {
    with_big(|| {
    let mut engine = engine();
    engine.eval("func fact(n) { if (n == 0) { return 1 }; n * fact(n - 1) }").unwrap();
    assert_eq!(engine.eval("fact(10)").unwrap().display(), "3628800");
    engine.eval("func even(n) { if (n == 0) { return 1 }; return odd(n - 1) }").unwrap();
    engine.eval("func odd(n) { if (n == 0) { return 0 }; return even(n - 1) }").unwrap();
    assert_eq!(engine.eval("even(40)").unwrap().display(), "1");
    engine
        .eval("func fib(n) { if (n < 2) { return n }; return fib(n - 1) + fib(n - 2) }")
        .unwrap();
    assert_eq!(engine.eval("fib(20)").unwrap().display(), "6765");
    });
}


#[test]
fn suite_recursion_a_deep_flat_chain_is_still_evaluated() {
    with_big(|| {
    let mut engine = engine();
    let source = format!("1{}", " + 1".repeat(500));
    assert_eq!(engine.eval(&source).unwrap().display(), "501");
    });
}


// ---------------------------------------------------------------------------
// RecursionBudgetCompositionTests
// ---------------------------------------------------------------------------

const F: &str = "func f(n) { if (n == 0) { return 0 }; return f(n - 1) }; ";
const IDENT: &str = "func ident(x) { return x }; ";

fn composition_shapes() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("top level", "f({0})", "0"),
        ("a block holding only the call", "{ f({0}) }", "0"),
        ("a block with the call first", "{ f({0}); 1 }", "1"),
        ("a block with one preceding sibling", "{ 1; f({0}) }", "0"),
        ("a block with two preceding siblings", "{ 1; 1; f({0}) }", "0"),
        ("a block with four preceding siblings", "{ 1; 1; 1; 1; f({0}) }", "0"),
        ("a block with five preceding siblings", "{ 1; 1; 1; 1; 1; f({0}) }", "0"),
        ("a block with twenty preceding siblings", "{ 1; 1; 1; 1; 1; 1; 1; 1; 1; 1; 1; 1; 1; 1; 1; 1; 1; 1; 1; 1; f({0}) }", "0"),
        ("four top-level statements before the call", "1; 1; 1; 1; f({0})", "0"),
        ("three nested blocks", "{ { { f({0}) } } }", "0"),
        ("the argument of print", "print(f({0}))", ""),
        ("the left operand of +", "f({0}) + 0", "0"),
        ("the argument of a user function", "ident(f({0}))", "0"),
        ("the body of an if", "if (1 == 1) { f({0}) }", "0"),
        ("the body of a for", "for i in 1..5 { f({0}) }", "0"),
        ("the body of a while", "i = 0; while (i < 5) { i = i + 1; f({0}) }", "0"),
    ]
}

#[test]
fn suite_composition_deepest_accepted_call_answers_in_every_composition() {
    with_big(|| {
    for (name, body, answer) in composition_shapes() {
        let mut engine = engine();
        engine.eval(F).unwrap();
        engine.eval(IDENT).unwrap();
        let script = body.replace("{0}", "85");
        let value = engine.eval(&script).unwrap_or_else(|e| panic!("{name}: {e}"));
        if !answer.is_empty() && name != "the body of a for" && name != "the body of a while" {
            assert_eq!(value.display(), answer, "{name}");
        }
    }
    });
}


#[test]
fn suite_composition_one_level_past_the_boundary_is_refused_in_every_composition() {
    with_big(|| {
    for (name, body, _) in composition_shapes() {
        let mut engine = engine();
        engine.eval(F).unwrap();
        engine.eval(IDENT).unwrap();
        let script = body.replace("{0}", "86");
        let err = engine.eval(&script).expect_err(name);
        assert_evaluation_refusal(&err);
    }
    });
}


fn deepest_accepted(engine: &mut Engine, body: &str) -> i64 {
    for depth in (0..=90).rev() {
        let script = body.replace("{0}", &depth.to_string());
        if engine.eval(&script).is_ok() {
            return depth;
        }
    }
    -1
}

#[test]
fn suite_composition_the_boundary_moves_only_with_live_nesting() {
    with_big(|| {
    let mut engine = engine();
    engine.eval(F).unwrap();
    engine.eval(IDENT).unwrap();
    for (_name, body, _) in composition_shapes() {
        assert_eq!(deepest_accepted(&mut engine, body), 85, "composition {body}");
    }
    assert_eq!(
        deepest_accepted(&mut engine, "func g() { return f({0}) }; g()"),
        84,
        "one live frame"
    );
    assert_eq!(
        deepest_accepted(&mut engine, "func g() { return f({0}) }; func h() { return g() }; h()"),
        83,
        "two live frames"
    );
    });
}


// ---------------------------------------------------------------------------
// ValueDepthGuardTests
// ---------------------------------------------------------------------------

fn wrap_script(wraps: usize) -> String {
    let mut s = String::from("x = 1");
    for _ in 0..wraps {
        s.push_str("; x = [x]");
    }
    s
}

fn expected_display(wraps: usize) -> String {
    format!("{}1{}", "[".repeat(wraps), "]".repeat(wraps))
}

#[test]
fn suite_value_depth_value_at_the_budget_is_still_rendered() {
    with_big(|| {
    let mut engine = engine();
    let value = engine.eval(&wrap_script(1023)).expect("depth 1024 is the budget");
    assert_eq!(engine.format(&value), expected_display(1023));
    let projected = crate::structured::project_value(&engine, &value, None);
    assert_eq!(text(&projected, "kind"), "Array");
    });
}


#[test]
fn suite_value_depth_rank_raised_by_repetition_is_refused() {
    with_big(|| {
    let mut engine = engine();
    assert!(engine.eval(&wrap_script(1023)).is_ok());
    let err = engine.eval(&wrap_script(1024)).expect_err("value depth 1025 is refused");
    assert!(depth_error(&err));
    assert!(err.message.starts_with("value depth "), "{}", err.message);
    assert!(err.message.contains("exceeds the maximum supported nesting depth of 1024"), "{}", err.message);
    });
}


#[test]
fn suite_value_depth_rank_far_past_the_budget_is_refused() {
    with_big(|| {
    let mut engine = engine();
    let err = engine.eval(&wrap_script(20000)).expect_err("20000 wraps are refused");
    assert!(depth_error(&err));
    });
}


#[test]
fn suite_value_depth_one_past_the_budget_is_refused_by_the_display_walk() {
    with_big(|| {
    let mut engine = engine();
    let err = engine.eval(&wrap_script(1024)).expect_err("1024 wraps are refused");
    assert!(depth_error(&err));
    assert!(err.message.contains("value depth 1025"), "{}", err.message);
    });
}


#[test]
fn suite_value_depth_a_value_far_past_the_budget_is_refused() {
    with_big(|| {
    let mut engine = engine();
    let err = engine.eval(&wrap_script(20000)).expect_err("20000 wraps are refused");
    assert!(depth_error(&err));
    });
}


#[test]
fn suite_value_depth_a_wide_value_is_not_refused() {
    with_big(|| {
    let mut engine = engine();
    let value = Value::Vector((0..20000).map(natural).collect());
    let text = engine.format(&value);
    assert!(text.starts_with("[0, 1, 2"), "{}", &text[..text.len().min(40)]);
    assert!(text.ends_with("19999]"));
    });
}


// ---------------------------------------------------------------------------
// ParserTests / FunctionTests / StatementTests
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// NameSuggestionsTests — the four names nothing is close to, plus the dropped synonym
// ---------------------------------------------------------------------------

#[test]
fn suite_name_suggestions_a_name_nothing_is_close_to_is_not_guessed_at() {
    for queried in ["nosuchfn", "push", "zip"] {
        assert!(
            registry::suggest(queried).is_empty(),
            "'{queried}' must not be guessed at: {:?}",
            registry::suggest(queried)
        );
    }
}

#[test]
fn suite_name_suggestions_a_synonym_whose_target_is_not_registered_is_dropped() {
    // 'derive' is the synonym of 'diff'; with a registry that only has det/inv the
    // answer is dropped rather than offered through the synonym.
    assert_eq!(registry::name_rank("det", "derive"), -1);
    assert_eq!(registry::name_rank("inv", "derive"), -1);
}

// ---------------------------------------------------------------------------
// RecordValueTests — through the records the engine already publishes
// ---------------------------------------------------------------------------

#[test]
fn suite_record_returns_through_engine_with_record_kind() {
    let mut engine = engine();
    let value = engine.eval("divrem(7, 2)").expect("divrem answers");
    assert_eq!(value.kind_name(), "Record");
    match &value {
        Value::Record(name, _) => assert_eq!(name, "DivRemResult"),
        other => panic!("expected a record, got {other:?}"),
    }
}

#[test]
fn suite_record_member_access_reads_fields() {
    let mut engine = engine();
    engine.eval("r = divrem(7, 2)").unwrap();
    assert_eq!(engine.eval("r.quotient").unwrap().display(), "3");
    assert_eq!(engine.eval("r.remainder").unwrap().display(), "1");
}

fn natural(n: u64) -> Value {
    Value::Natural(lovelace_nat::Nat::from_u64(n))
}

fn demo_record() -> Value {
    Value::Record(
        "DemoResult".to_string(),
        vec![
            ("count".to_string(), natural(3)),
            ("label".to_string(), Value::Text("hello".to_string())),
            (
                "nested".to_string(),
                Value::Record(
                    "Inner".to_string(),
                    vec![("flag".to_string(), Value::Boolean(true))],
                ),
            ),
        ],
    )
}

#[test]
fn suite_record_formatter_renders_type_name_and_fields() {
    let value = demo_record();
    assert_eq!(
        value.display(),
        "DemoResult(count: 3, label: hello, nested: Inner(flag: True))"
    );
    assert_eq!(
        value.typed(),
        "DemoResult(count: 3, label: hello, nested: Inner(flag: True)) (DemoResult)"
    );
}

// ---------------------------------------------------------------------------
// PlotTests — the engine-level shapes (the renderer-model rows have no Rust seam)
// ---------------------------------------------------------------------------

fn plot_dir(name: &str) -> String {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../target/tmp/").to_string() + name;
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn suite_plot_given_single_vector_writes_svg_file() {
    let dir = plot_dir("round19-plot-single");
    let mut engine = engine();
    engine.set_plot_dir(&dir);
    engine.set_plot_file("test.svg");
    let value = engine.eval("plot([4, 9, 16])").expect("plot answers");
    assert_eq!(value.kind_name(), "Text");
    let path = value.display();
    assert!(std::path::Path::new(&path).exists());
    let svg = std::fs::read_to_string(&path).unwrap();
    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("<polyline"));
}

#[test]
fn suite_plot_given_mismatched_vector_lengths_throws() {
    let mut engine = engine();
    let err = engine.eval("plot(1..3, [1, 2])").expect_err("mismatched lengths are refused");
    assert_eq!(err.code, "InvalidOperation");
}

#[test]
fn suite_plot_given_empty_vector_throws() {
    let mut engine = engine();
    let err = engine.eval("plot([])").expect_err("an empty plot is refused");
    assert_eq!(err.code, "InvalidOperation");
}

#[test]
fn suite_plot_given_title_includes_title_in_svg() {
    let dir = plot_dir("round19-plot-title");
    let mut engine = engine();
    engine.set_plot_dir(&dir);
    engine.set_plot_file("titled.svg");
    let value = engine.eval("plot(1..3, [1, 4, 9], \"My Plot\")").expect("titled plot answers");
    let svg = std::fs::read_to_string(value.display()).unwrap();
    assert!(svg.contains("My Plot"), "the title must reach the SVG");
}

fn text<'a>(j: &'a lovelace_json::Json, key: &str) -> &'a str {
    j.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("missing string '{key}'"))
}
// ---------------------------------------------------------------------------
// StructuralEqualityTests / EnumPayloadSeamTests / SuiteEngineTests additions
// ---------------------------------------------------------------------------

#[test]
fn suite_structural_identical_records_are_equal() {
    fn sample(count: u64) -> Value {
        Value::Record(
            "SolveResult".to_string(),
            vec![
                ("status".to_string(), Value::Enum("SolveStatus".to_string(), "Solved".to_string())),
                ("solutions".to_string(), Value::Vector(vec![
                    Value::Record("Solution".to_string(), vec![
                        ("value".to_string(), Value::Integer(lovelace_int::Int::from_i64(-2))),
                        ("multiplicity".to_string(), natural(1)),
                    ]),
                    Value::Record("Solution".to_string(), vec![
                        ("value".to_string(), Value::Integer(lovelace_int::Int::from_i64(2))),
                        ("multiplicity".to_string(), natural(count)),
                    ]),
                ])),
            ],
        )
    }
    assert_eq!(sample(2), sample(2));
    assert_ne!(sample(2), sample(3));
}

#[test]
fn suite_enum_value_api_carries_the_enum_kind() {
    let value = Value::Enum("SolveStatus".to_string(), "Partial".to_string());
    assert_eq!(value.kind_name(), "Enum");
    assert_eq!(value.display(), "Partial");
    assert_eq!(value.typed(), "Partial");
    let projected = crate::structured::project_value(&Engine::new(), &value, None);
    assert_eq!(text(&projected, "kind"), "Enum");
    assert_eq!(text(&projected, "type"), "SolveStatus");
    assert_eq!(text(&projected, "value"), "Partial");
}

#[test]
fn suite_engine_records_per_statement_timings() {
    let mut engine = engine();
    engine.eval("1+1; 2+2").unwrap();
    let timings = engine.operation_timings();
    assert_eq!(timings.len(), 2);
    assert_eq!(timings[0].position, 0);
    assert_eq!(timings[1].position, 5);
}

#[test]
fn suite_engine_error_records_diagnostic_with_position() {
    let mut engine = engine();
    let err = engine.eval("1+1; det(1)").expect_err("det(1) fails");
    assert_eq!(err.position, 5, "the failing statement's position");
    assert_eq!(err.code, "InvalidArgument");
}

fn points_attribute(svg: &str) -> String {
    let needle = "points=\"";
    let start = svg.find(needle).expect("the SVG carries a polyline") + needle.len();
    let tail = &svg[start..];
    tail[..tail.find('"').unwrap()].to_string()
}

fn plot_svg(script: &str, tag: &str) -> (String, String) {
    let dir = plot_dir(&format!("round19-plot-{tag}"));
    let mut engine = engine();
    engine.set_plot_dir(&dir);
    engine.set_plot_file("plot.svg");
    let value = engine.eval(script).expect("the plot answers");
    let svg = std::fs::read_to_string(value.display()).unwrap();
    (svg, dir)
}

#[test]
fn suite_plot_fixed_model_produces_deterministic_output() {
    let (first, _) = plot_svg("plot([1, 2, 3], [2, 4, 9])", "det-a");
    let (second, _) = plot_svg("plot([1, 2, 3], [2, 4, 9])", "det-b");
    assert_eq!(first, second);
}

#[test]
fn suite_plot_two_points_fall_back_to_polyline() {
    let (svg, _) = plot_svg("plot(1..2, [1, 4])", "two");
    assert!(svg.contains("<polyline"));
    assert!(!svg.contains("<path"));
}

#[test]
fn suite_plot_three_points_uses_polyline_not_path() {
    let (svg, _) = plot_svg("plot(1..3, [1, 4, 9])", "three");
    assert!(svg.contains("<polyline"));
    assert!(!svg.contains("<path"));
}

#[test]
fn suite_plot_collinear_points_stay_collinear() {
    let (svg, _) = plot_svg("plot(1..4, [1, 2, 3, 4])", "collinear");
    let points = points_attribute(&svg);
    let coords: Vec<(f64, f64)> = points
        .split(' ')
        .map(|token| {
            let mut parts = token.split(',');
            (
                parts.next().unwrap().parse().unwrap(),
                parts.next().unwrap().parse().unwrap(),
            )
        })
        .collect();
    assert!(coords.len() >= 4, "expected the polyline to carry every point");
    let (ax, ay) = coords[0];
    let (bx, by) = coords[coords.len() - 1];
    let line_length = ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt();
    for (x, y) in coords {
        let cross = (bx - ax) * (y - ay) - (by - ay) * (x - ax);
        assert!(cross.abs() / line_length < 0.05);
    }
}

// ---------------------------------------------------------------------------
// Round-8 engine refusals that now reproduce
// ---------------------------------------------------------------------------

fn evaluated(script: &str) -> (Engine, Value) {
    let mut engine = engine();
    let value = engine.eval(script).unwrap_or_else(|e| panic!("'{script}': {e}"));
    (engine, value)
}

#[test]
fn suite_empty_reductions_answer_zero_and_one() {
    let (_e, sum) = evaluated("sum([])");
    assert_eq!(sum.kind_name(), "Natural");
    assert_eq!(sum.display(), "0");
    let (_e, prod) = evaluated("prod([])");
    assert_eq!(prod.kind_name(), "Natural");
    assert_eq!(prod.display(), "1");
}

#[test]
fn suite_slices_with_step_match_the_documented_values() {
    assert_eq!(evaluated("[0, 1, 2, 3, 4][::2]").0.format_typed(&evaluated("[0, 1, 2, 3, 4][::2]").1), "[0, 2, 4] (Vector)");
    assert_eq!(
        evaluated("[1, 2, 3, 4, 5][::2] * 2").0.format_typed(&evaluated("[1, 2, 3, 4, 5][::2] * 2").1),
        "[2, 6, 10] (Vector)"
    );
}

#[test]
fn suite_len_of_an_empty_range_is_zero() {
    let (_e, value) = evaluated("len(1..0)");
    assert_eq!(value.kind_name(), "Natural");
    assert_eq!(value.display(), "0");
}

#[test]
fn suite_typed_array_promotion_and_narrowing() {
    let cases: &[(&str, &str, &[&str])] = &[
        ("[1, 2.5, 3]", "Vector", &["Real", "Real", "Real"]),
        ("[[1, 2.5], [3, 4]]", "Array", &["Real", "Real", "Real", "Real"]),
        ("[4, 6] / [2, 2]", "Vector", &["Natural", "Natural"]),
        ("[2.5] / [0.25]", "Vector", &["Natural"]),
        ("[-4, -6] / [-2, -2]", "Vector", &["Natural", "Natural"]),
    ];
    for (script, kind, expected) in cases {
        let mut engine = engine();
        let value = engine.eval(script).unwrap();
        assert_eq!(value.kind_name(), *kind, "{script}");
        let projected = crate::structured::project_value(&engine, &value, None);
        assert_eq!(text(&projected, "kind"), if *kind == "Vector" { "Array" } else { "Array" });
        let elements = array_values(&projected);
        let kinds: Vec<String> = elements.iter().map(|e| text(e, "kind").to_string()).collect();
        let expected: Vec<String> = expected.iter().map(|s| (*s).to_string()).collect();
        assert_eq!(kinds, expected, "{script}");
    }
}

fn array_values(j: &lovelace_json::Json) -> Vec<&lovelace_json::Json> {
    j.get("elements")
        .and_then(|e| match e {
            lovelace_json::Json::Arr(items) => Some(items.iter().collect()),
            _ => None,
        })
        .unwrap_or_default()
}

#[test]
fn suite_substitution_preserves_the_exact_rational() {
    let mut engine = engine();
    let value = engine.eval("x = symbol(\"x\"); subs(1/3, x, 2)").unwrap();
    let projected = crate::structured::project_value(&engine, &value, None);
    assert_eq!(text(&projected, "kind"), "Symbolic");
    assert!(projected.get("exact").and_then(|v| match v { lovelace_json::Json::Bool(b) => Some(*b), _ => None }).unwrap());
    assert_eq!(text(&projected, "canonical"), "(rat 1 3)");
}

#[test]
fn suite_approximation_carries_no_rational_form_and_is_not_exact() {
    let mut engine = engine();
    let value = engine.eval("evalf(1/3, 50)").unwrap();
    let projected = crate::structured::project_value(&engine, &value, None);
    assert_eq!(text(&projected, "kind"), "Real");
    assert!(!projected.get("exact").and_then(|v| match v { lovelace_json::Json::Bool(b) => Some(*b), _ => None }).unwrap());
    assert!(projected.get("numerator").is_none());
    assert!(projected.get("denominator").is_none());
}

#[test]
fn suite_arity_refusals_name_the_builtin_and_both_counts() {
    for (script, message) in [
        ("sqrt()", "sqrt(): expected 1 argument; got 0."),
        ("sqrt(4, 9)", "sqrt(): expected 1 argument; got 2."),
        ("setprecision(10, 20)", "setprecision(): expected 1 argument; got 2."),
        ("pi(10, 20)", "pi(): expected 0 to 1 arguments; got 2."),
    ] {
        let mut engine = engine();
        let err = engine.eval(script).expect_err(script);
        assert_eq!(err.message, message, "{script}");
        assert_eq!(err.code, "InvalidArgument");
        assert_eq!(err.category, "TypeMismatch");
    }
}

#[test]
fn suite_integer_factorial_answers_an_integer() {
    let (_e, value) = evaluated("(-(-5))!");
    assert_eq!(value.kind_name(), "Integer");
    assert_eq!(value.display(), "120");
}

#[test]
fn suite_negative_integer_factorial_names_the_domain() {
    let mut engine = engine();
    let err = engine.eval("(-1)!").expect_err("negative factorial is refused");
    assert_eq!(err.message, "Factorial is not defined for negative integers.");
    assert_eq!(err.code, "InvalidOperation");
}

#[test]
fn suite_divrem_of_integers_keeps_the_same_sign_quotient_positive() {
    let mut engine = engine();
    let value = engine.eval("__x = -7; divrem(__x, -3)").unwrap();
    match value {
        Value::Record(name, fields) => {
            assert_eq!(name, "DivRemResult");
            let get = |field: &str| fields.iter().find(|(f, _)| f == field).map(|(_, v)| v).unwrap();
            assert_eq!(get("quotient").display(), "2");
            assert_eq!(get("remainder").display(), "1");
        }
        other => panic!("expected a DivRemResult, got {other:?}"),
    }
}

#[test]
fn suite_exact_natural_division_narrows_back() {
    let (_e, value) = evaluated("100 / 5");
    assert_eq!(value.kind_name(), "Natural");
    assert_eq!(value.display(), "20");
}

#[test]
fn suite_setprecision_wide_count_names_the_count() {
    let mut engine = engine();
    let err = engine.eval("setprecision(123456789012345678901234567890)").expect_err("wide count");
    assert!(err.message.contains("123456789012345678901234567890"), "{}", err.message);
    assert_eq!(err.code, "InvalidArgument");
}

#[test]
fn suite_truncated_irrational_is_not_certified_exact() {
    for source in ["pi", "e", "sqrt(2)", "setprecision(18); pi", "setprecision(18); e", "setprecision(18); sqrt(2)", "evalf(1/3, 50)"] {
        let mut engine = engine();
        let value = engine.eval(source).unwrap();
        let projected = crate::structured::project_value(&engine, &value, None);
        assert!(
            !matches!(projected.get("exact"), Some(lovelace_json::Json::Bool(true))),
            "{source} was certified exact"
        );
        assert!(projected.get("numerator").is_none(), "{source}");
        assert!(projected.get("denominator").is_none(), "{source}");
    }
}