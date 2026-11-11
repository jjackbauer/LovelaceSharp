//! Ports of `Lovelace.Suite.Tests` engine-surface cases: vectors, engine state, script
//! normalization, plotting, exactness preservation, and the interpreter variable store.
//!
//! Every test uses the same concrete source or value the C# test uses and asserts the same
//! observable result (the typed rendering where the C# asserts kind/element, the structured
//! `exact` flag where the C# asserts exactness, or the same failure taxonomy).

use crate::syntax::{Expr, Parser};
use crate::{Engine, Value};

fn engine() -> Engine {
    Engine::new()
}

fn typed(script: &str) -> String {
    let mut e = engine();
    let v = e
        .eval(script)
        .unwrap_or_else(|err| panic!("script {script}: {}", err.message));
    e.format_typed(&v)
}

fn plot_dir(name: &str) -> String {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../target/tmp/").to_string() + name;
    let _ = std::fs::create_dir_all(&dir);
    dir
}

// ---------------------------------------------------------------------------
// VectorTests
// ---------------------------------------------------------------------------

#[test]
fn evaluate_given_list_literal_produces_vector() {
    let mut e = engine();
    let v = e.eval("[10, 20, 30]").expect("list literal evaluates");
    match &v {
        Value::Vector(items) => {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0].display(), "10");
        }
        other => panic!("wrong kind: {}", other.kind_name()),
    }
    assert_eq!(e.format_typed(&v), "[10, 20, 30] (Vector)");
}

#[test]
fn evaluate_given_index_uses_zero_based_indexing() {
    assert_eq!(typed("[10, 20, 30][0]"), "10 (Natural)");
}

#[test]
fn evaluate_given_index_out_of_range_throws() {
    let mut e = engine();
    let err = e
        .eval("[10, 20, 30][3]")
        .expect_err("index 3 is out of range");
    assert_eq!(err.code, "InvalidOperation");
}

#[test]
fn evaluate_given_vector_addition_applies_element_wise() {
    assert_eq!(typed("[1, 2] + [10, 20]"), "[11, 22] (Vector)");
}

#[test]
fn evaluate_given_vector_scalar_multiply_broadcasts() {
    assert_eq!(typed("[1, 2, 3] * 10"), "[10, 20, 30] (Vector)");
}

#[test]
fn evaluate_given_mismatched_vector_lengths_throws() {
    let mut e = engine();
    let err = e
        .eval("[1, 2] + [1, 2, 3]")
        .expect_err("mismatched lengths are refused");
    assert_eq!(err.code, "InvalidOperation");
}

#[test]
fn evaluate_given_len_returns_vector_length() {
    assert_eq!(typed("len([5, 6, 7, 8])"), "4 (Natural)");
}

#[test]
fn evaluate_given_indexed_range_element_returns_expected_value() {
    assert_eq!(typed("(1..5)[2]"), "3 (Natural)");
}

// ---------------------------------------------------------------------------
// SuiteEngineTests
// ---------------------------------------------------------------------------

#[test]
fn evaluate_given_assignment_exposes_variable_in_variables_dictionary() {
    let mut e = engine();
    e.eval("x = 42").expect("assignment evaluates");
    let vars = e.variable_values();
    let value = vars
        .iter()
        .find(|(name, _)| name == "x")
        .map(|(_, v)| v)
        .expect("x is stored");
    assert!(
        matches!(value, Value::Natural(_)),
        "x is not Natural: {}",
        value.kind_name()
    );
    assert_eq!(value.display(), "42");
}

#[test]
fn evaluate_given_result_stores_in_underscore_variable() {
    let mut e = engine();
    e.eval("1 + 1").expect("addition evaluates");
    let vars = e.variable_values();
    let value = vars
        .iter()
        .find(|(name, _)| name == "_")
        .map(|(_, v)| v)
        .expect("_ is stored");
    assert_eq!(value.display(), "2");
}

#[test]
fn parse_given_statements_returns_program() {
    let toks = crate::syntax::tokenize("x = 1; y = 2; x + y").expect("source tokenises");
    let program = Parser::new(toks).program().expect("source parses");
    assert_eq!(program.len(), 3);
}

#[test]
fn parse_expression_given_expression_returns_binary_expr() {
    let toks = crate::syntax::tokenize("1 + 2").expect("source tokenises");
    let expr = Parser::new(toks)
        .single_expression()
        .expect("source parses");
    assert!(
        matches!(expr, Expr::Binary(crate::syntax::BinOp::Add, _, _)),
        "wrong node: {expr:?}"
    );
}

// ---------------------------------------------------------------------------
// ScriptSourceTests (only the evaluation case; Rust has no ScriptSource preprocessing API)
// ---------------------------------------------------------------------------

#[test]
fn evaluate_async_given_multiline_script_plots_one_over_x_squared() {
    let dir = plot_dir("suite-multiline");
    let mut e = engine();
    e.set_plot_dir(&dir);
    e.set_plot_file("invx2.svg");
    let source = "x = 1..10\ny = 1 / x^2\nplot(x, y, \"1/x^2\")";
    let value = e.eval(source).expect("multiline script evaluates");
    assert!(
        matches!(value, Value::Text(_)),
        "plot does not return Text: {}",
        value.kind_name()
    );
    let plot = e.last_plot().expect("plot captured");
    assert_eq!(plot.title, "1/x^2");
    let vars = e.variable_values();
    let y = vars
        .iter()
        .find(|(name, _)| name == "y")
        .map(|(_, v)| v)
        .expect("y is stored");
    match y {
        Value::Vector(items) => {
            assert_eq!(items.len(), 10);
            assert_eq!(items[0].display(), "1");
            assert_eq!(items[1].display(), "0.25");
            assert_eq!(items[9].display(), "0.01");
        }
        other => panic!("y is not a vector: {}", other.kind_name()),
    }
}

// ---------------------------------------------------------------------------
// PlotTests / SuiteEngineOutputPlotTests
// ---------------------------------------------------------------------------

#[test]
fn plot_given_single_vector_writes_svg_file() {
    let dir = plot_dir("suite-plot-single");
    let mut e = engine();
    e.set_plot_dir(&dir);
    e.set_plot_file("test.svg");
    let value = e.eval("plot([4, 9, 16])").expect("plot evaluates");
    let path = match value {
        Value::Text(path) => path,
        other => panic!("plot does not return Text: {}", other.kind_name()),
    };
    assert!(
        std::path::Path::new(&path).exists(),
        "missing plot file {path}"
    );
    let svg = std::fs::read_to_string(&path).expect("plot file readable");
    assert!(svg.starts_with("<svg"), "svg does not start with <svg");
    assert!(svg.contains("<polyline"), "svg has no polyline");
}

#[test]
fn plot_given_mismatched_vector_lengths_throws() {
    let mut e = engine();
    let err = e
        .eval("plot(1..3, [1, 2])")
        .expect_err("mismatched plot is refused");
    assert_eq!(err.code, "InvalidOperation");
}

#[test]
fn plot_given_empty_vector_throws() {
    let mut e = engine();
    let err = e.eval("plot([])").expect_err("empty plot is refused");
    assert_eq!(err.code, "InvalidOperation");
}

#[test]
fn plot_given_title_includes_title_in_svg() {
    let dir = plot_dir("suite-plot-title");
    let mut e = engine();
    e.set_plot_dir(&dir);
    e.set_plot_file("titled.svg");
    let value = e
        .eval("plot(1..3, [1, 4, 9], \"My Plot\")")
        .expect("titled plot evaluates");
    let path = match value {
        Value::Text(path) => path,
        other => panic!("plot does not return Text: {}", other.kind_name()),
    };
    let svg = std::fs::read_to_string(&path).expect("plot file readable");
    assert!(svg.contains("My Plot"), "title missing from svg");
}

#[test]
fn evaluate_async_given_plot_call_sets_last_plot_svg() {
    let dir = plot_dir("suite-plot-last");
    let mut e = engine();
    e.set_plot_dir(&dir);
    e.set_plot_file("test.svg");
    e.eval("plot(1..3, [1, 4, 9])").expect("plot evaluates");
    let plot = e.last_plot().expect("last plot captured");
    assert!(plot.svg.starts_with("<svg"), "svg does not start with <svg");
}

// ---------------------------------------------------------------------------
// InterpreterVariableStoreTests
// ---------------------------------------------------------------------------

#[test]
fn evaluate_given_assignment_stores_value_and_returns_it() {
    assert_eq!(typed("x = 5"), "5 (Natural)");
}

#[test]
fn evaluate_given_variable_reference_returns_stored_value() {
    let mut e = engine();
    e.eval("x = 5").expect("assignment evaluates");
    let value = e.eval("x").expect("reference evaluates");
    assert_eq!(e.format_typed(&value), "5 (Natural)");
}

#[test]
fn evaluate_given_undefined_variable_throws_error() {
    let mut e = engine();
    let err = e.eval("y").expect_err("undefined variable is refused");
    assert!(
        err.message.contains('y'),
        "message does not name y: {}",
        err.message
    );
}

// ---------------------------------------------------------------------------
// InterpreterBuiltinInvTests
// ---------------------------------------------------------------------------

#[test]
fn evaluate_given_inv_of_zero_throws_divide_by_zero_exception() {
    let mut e = engine();
    let err = e.eval("inv(0)").expect_err("inv(0) is refused");
    // C# DivideByZeroException crosses the wire as the DivisionByZero code.
    assert_eq!(err.code, "DivisionByZero");
}

// ---------------------------------------------------------------------------
// ExactnessPreservationTests2.Arithmetic_DoesNotDegradeExactness
// ---------------------------------------------------------------------------

#[test]
fn arithmetic_does_not_degrade_exactness() {
    for source in ["1/3 * 3", "1/3 - 1/3", "1/6 + 1/3", "1/3 + 0"] {
        let mut e = engine();
        let value = e
            .eval(source)
            .unwrap_or_else(|err| panic!("{source}: {}", err.message));
        match value {
            Value::Real(real) => assert!(real.is_exact(), "{source} lost exactness"),
            other => panic!("{source} is {}, not Real", other.kind_name()),
        }
    }
}

// ---------------------------------------------------------------------------
// DescriptorExampleTests.Example_Executes — every remaining example the Rust
// registry advertises and the reference engine also runs. The C# assertion is
// exactly "the snippet executes on a fresh engine primed with the documented
// prelude"; each row below is that same prelude + snippet.
// ---------------------------------------------------------------------------

const DESCRIPTOR_PRELUDE: &str = "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); ";

#[test]
fn descriptor_examples_execute() {
    let examples: &[(&str, &str)] = &[
        ("assume", "assume(and(x > 0, x < 10))"),
        ("print", "print(\"hi\", 42)"),
        ("setprecision", "setprecision(100)"),
        ("simplify_full", "simplify_full(exp(log(x)))"),
        ("squeeze", "squeeze([[[1]]])"),
        ("assume_nonnegative", "assume_nonnegative(x)"),
        ("assume_positive", "assume_positive(x)"),
        ("collect", "collect(x*y + x + y + 1, x)"),
        ("compile", "compile(x^2 + 1, [x])"),
        ("conj", "conj(fft([0, 1, 0, 0])[1])"),
        ("inv_full", "inv_full([[x, 1], [1, x]])"),
        ("linsolve", "linsolve([[x, 1], [1, x]], [5, 4])"),
        ("linsolve_full", "linsolve_full([[x, 1], [1, x]], [5, 4])"),
        ("mean", "mean([1, 2, 3])"),
        ("or", "or(x > 0, x < -1)"),
        ("pi", "pi()"),
        ("plot", "plot([1, 2, 3])"),
        ("plot", "plot([1, 2, 3], [1, 4, 9], \"title\")"),
    ];
    let dir = plot_dir("suite-descriptor-doctest");
    for (function, example) in examples {
        let mut e = engine();
        e.set_plot_dir(&dir);
        e.set_plot_file("doctest.svg");
        e.eval(DESCRIPTOR_PRELUDE)
            .unwrap_or_else(|err| panic!("descriptor prelude failed: {}", err.message));
        e.eval(example).unwrap_or_else(|err| {
            panic!(
                "the descriptor example for '{function}' does not execute: {example}: {}",
                err.message
            )
        });
    }
}

// ---------------------------------------------------------------------------
// DspPluginTests (the rows whose Rust core behaviour matches the plugin story)
// ---------------------------------------------------------------------------

#[test]
fn fft_given_impulse_returns_all_ones() {
    assert_eq!(typed("fft([1, 0, 0, 0])"), "[1, 1, 1, 1] (Vector)");
}

#[test]
fn fft_given_impulse_returns_complex_typed_vector() {
    let mut e = engine();
    let value = e.eval("fft([0, 1, 0, 0])").expect("fft evaluates");
    match value {
        Value::Vector(items) => {
            assert_eq!(items.len(), 4);
            for item in &items {
                assert!(
                    matches!(item, Value::Complex(_, _)),
                    "element is {}",
                    item.kind_name()
                );
            }
        }
        other => panic!("fft is not a vector: {}", other.kind_name()),
    }
}

#[test]
fn abs_given_complex_returns_magnitude() {
    assert_eq!(typed("abs(fft([0, 1, 0, 0])[1])"), "1 (Real)");
}

// ---------------------------------------------------------------------------
// EnumPayloadSeamTests.EnumValues_AreEqualExactlyWhenTypeNameAndNameMatch
// ---------------------------------------------------------------------------

#[test]
fn enum_values_are_equal_exactly_when_type_name_and_name_match() {
    let mut e = engine();
    e.eval("x = symbol(\"x\")").expect("symbol evaluates");
    let solved = e
        .eval("solve_full(x^2 - 4 == 0, x).status")
        .expect("status evaluates");
    let solved_again = e
        .eval("solve_full(x^2 - 4 == 0, x).status")
        .expect("status evaluates");
    assert_eq!(solved, solved_again);
    let status = e
        .eval("solve_full(x^4 - x^2 - 1 == 0, x).status")
        .expect("status evaluates");
    let completeness = e
        .eval("solve_full(x^4 - x^2 - 1 == 0, x).completeness")
        .expect("completeness evaluates");
    assert_ne!(status, completeness);
    let status_debug = format!("{status:?}");
    let completeness_debug = format!("{completeness:?}");
    assert!(
        status_debug.contains("SolveStatus"),
        "wrong type: {status_debug}"
    );
    assert!(
        completeness_debug.contains("Completeness"),
        "wrong type: {completeness_debug}"
    );
}

// ---------------------------------------------------------------------------
// ValueTests constructor kinds
// ---------------------------------------------------------------------------

#[test]
fn value_given_natural_stores_natural_kind() {
    use lovelace_nat::Nat;
    assert_eq!(Value::Natural(Nat::from_u64(42)).kind_name(), "Natural");
}

#[test]
fn value_given_integer_stores_integer_kind() {
    use lovelace_int::Int;
    assert_eq!(Value::Integer(Int::from_i64(42)).kind_name(), "Integer");
}

#[test]
fn value_given_real_stores_real_kind() {
    use lovelace_real::Real;
    assert_eq!(
        Value::Real(Real::from_str("3.14").unwrap()).kind_name(),
        "Real"
    );
}

// ---------------------------------------------------------------------------
// CrossEngineIsolationTests
// ---------------------------------------------------------------------------

#[test]
fn concurrent_engines_capture_their_own_print_output() {
    let mut a = engine();
    let mut b = engine();
    a.eval("print(\"from-a\")").expect("A prints");
    b.eval("print(\"from-b\")").expect("B prints");
    assert_eq!(a.take_output(), vec!["from-a".to_string()]);
    assert_eq!(b.take_output(), vec!["from-b".to_string()]);
}

#[test]
fn concurrent_engines_keep_their_own_variables() {
    let mut a = engine();
    let mut b = engine();
    a.eval("shared = 1; print(\"a\")").expect("A runs");
    b.eval("shared = 2; print(\"b\")").expect("B runs");
    let value_of = |e: &Engine, name: &str| {
        e.variable_values()
            .into_iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.display())
            .expect("variable stored")
    };
    assert_eq!(value_of(&a, "shared"), "1");
    assert_eq!(value_of(&b, "shared"), "2");
}

// ---------------------------------------------------------------------------
// LRealDispatchTests (all six rows: the engine's own precision knob)
// ---------------------------------------------------------------------------

#[test]
fn lreal_dispatch_arithmetic_at_low_precision_matches_class_real() {
    let cases: &[(&str, usize, &str)] = &[
        ("0.1 + 0.2", 18, "0.3"),
        ("1 / 3", 18, "0.(3)"),
        ("3.14 * 2.71", 18, "8.5094"),
        ("10 / 7", 18, "1.(428571)"),
        (
            "2.345678901234567 * 1.234567890123456",
            37,
            "2.895899852004265910684444703552",
        ),
    ];
    for (expr, precision, expected) in cases {
        let mut e = engine();
        e.set_precision(*precision);
        let value = e
            .eval(expr)
            .unwrap_or_else(|err| panic!("{expr}: {}", err.message));
        assert_eq!(e.format(&value), *expected, "{expr} at {precision} places");
    }
}

#[test]
fn lreal_dispatch_default_precision_is_unchanged() {
    let mut e = engine();
    let value = e.eval("0.1 + 0.2").expect("addition evaluates");
    assert_eq!(e.format(&value), "0.3");
}

// ---------------------------------------------------------------------------
// PlotFileWriteRefusalTests.WritablePlotFile_IsStillWritten
// ---------------------------------------------------------------------------

#[test]
fn writable_plot_file_is_still_written() {
    let dir = plot_dir("suite-plot-writable");
    let dir = std::fs::canonicalize(&dir)
        .expect("plot dir exists")
        .to_string_lossy()
        .to_string();
    let mut e = engine();
    e.set_plot_dir(&dir);
    e.set_plot_file("ok.svg");
    let value = e.eval("plot([1, 2, 3])").expect("plot evaluates");
    let path = match value {
        Value::Text(path) => path,
        other => panic!("plot does not return Text: {}", other.kind_name()),
    };
    let expected = std::path::Path::new(&dir)
        .join("ok.svg")
        .to_string_lossy()
        .to_string();
    assert_eq!(path, expected);
    assert!(std::path::Path::new(&path).exists(), "plot file missing");
    let svg = std::fs::read_to_string(&path).expect("plot readable");
    assert!(svg.to_lowercase().contains("<svg"), "plot is not SVG");
}

// ---------------------------------------------------------------------------
// InterpreterBuiltinSetprecisionTests (the rows whose observable is the one
// engine precision knob Rust keeps, plus the kind refusal and the pi cap lift)
// ---------------------------------------------------------------------------

#[test]
fn setprecision_given_natural_argument_sets_engine_precision() {
    let mut e = engine();
    let value = e
        .eval("setprecision(2500)")
        .expect("setprecision evaluates");
    assert_eq!(value, Value::Void);
    assert_eq!(e.precision(), 2500);
}

#[test]
fn setprecision_given_integer_argument_sets_engine_precision() {
    let mut e = engine();
    let value = e
        .eval("setprecision(-(-2500))")
        .expect("setprecision evaluates");
    assert_eq!(value, Value::Void);
    assert_eq!(e.precision(), 2500);
}

#[test]
fn setprecision_given_precision_above_default_allows_pi_to_compute_that_many_digits() {
    let mut e = engine();
    e.eval("setprecision(1100)")
        .expect("setprecision evaluates");
    let value = e.eval("pi(1100)").expect("pi(1100) evaluates");
    match value {
        Value::Real(real) => {
            let text = real.to_string();
            let fractional = text.split_once('.').map(|(_, f)| f).unwrap_or("");
            assert_eq!(
                fractional.len(),
                1100,
                "pi(1100) does not carry 1100 fractional digits"
            );
        }
        other => panic!("pi is not Real: {}", other.kind_name()),
    }
}

#[test]
fn setprecision_given_integral_real_keeps_the_documented_type_refusal() {
    let mut e = engine();
    let err = e
        .eval("setprecision(3.0)")
        .expect_err("a Real count is refused");
    assert_eq!(err.code, "InvalidOperation");
}

// ---------------------------------------------------------------------------
// InterpreterFactorialTests.Evaluate_GivenRealFactorial_ThrowsDescriptiveError
// ---------------------------------------------------------------------------

#[test]
fn factorial_given_real_factorial_throws_descriptive_error() {
    let mut e = engine();
    let err = e.eval("3.14!").expect_err("a Real factorial is refused");
    assert_eq!(err.code, "InvalidOperation");
    assert!(
        err.message.contains("Real"),
        "message does not name Real: {}",
        err.message
    );
}

// ---------------------------------------------------------------------------
// DspPluginTests.Dft_GivenLength_MatchesPaddedSignal
// ---------------------------------------------------------------------------

/// DspPluginTests.cs:133 asserts dft([1, 2, 3], 4) renders exactly like dft([1, 2, 3, 0]).
/// The round-29 integer-argument fix made the optional length pad the signal, so the two calls
/// are the same transform now.
#[test]
fn dft_given_length_matches_padded_signal() {
    assert_eq!(typed("dft([1, 2, 3], 4)"), typed("dft([1, 2, 3, 0])"));
}
