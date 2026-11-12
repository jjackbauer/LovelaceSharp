//! Round 90: the five small deferred families in the runner/DSP layer.
//!
//! Each test names the census id it closes and asserts the C# case's own observable, not a
//! Rust-shaped approximation:
//!
//! * Lovelace.Suite.Tests/PlotFileWriteRefusalTests (3 rows): the typed PlotFileWriteException
//!   and the two caller-path refusals. The two runner-facing rows are also driven at the wire
//!   level in lovelace-run::round90_runner_families.
//! * Lovelace.Dsp.Tests/DspTests (3 rows): the signal Sum/Product combinators and the Sequence
//!   length-mismatch ArgumentException.
//! * Lovelace.Complex.Tests/DenseArrayComplexTests (3 rows): DenseArray<Complex> construction,
//!   metadata, GetElement and AsSpan.
//! * Lovelace.Integer.Tests/IntegerToStringTests (2 rows): Integer.TryFormat.
//! * Lovelace.Suite.Tests/ParserTests (2 rows): chained assignment and the unary-plus AST node.

use std::any::TypeId;
use std::path::Path;

use lovelace_array::{DType, DenseArray, Precision};
use lovelace_int::Int;

use crate::Engine;
use crate::complex::Complex;
use crate::display_format::TryFormat;
use crate::plot_file::{PlotFileWriteException, PlotFileWriteFailure, write_plot_file};
use crate::signals::{Impulse, Product, Sequence, Signal, Step, Sum};
use crate::syntax::{Expr, Parser, tokenize};

fn plot_dir(name: &str) -> String {
    let directory = format!("{}/../target/tmp/{}", env!("CARGO_MANIFEST_DIR"), name);
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("a fresh probe directory");
    directory
}

fn complex(re: i64, im: i64) -> Complex {
    Complex::new(
        lovelace_real::Real::from_i64(re),
        lovelace_real::Real::from_i64(im),
    )
}

// ---------------------------------------------------------------------------
// Lovelace.Suite.Tests/PlotFileWriteRefusalTests
// ---------------------------------------------------------------------------

/// TheSuiteDeclaresThePlotFileWriteRefusalTheRunnerClassifies: the suite declares the typed
/// refusal and the runner's classification is PlotFileError/TypeMismatch.
#[test]
fn the_suite_declares_the_plot_file_write_refusal_the_runner_classifies() {
    fn requires_plot_file_write_exception(_: &PlotFileWriteException) {}
    let directory = plot_dir("round90-plot-type");
    let failure = write_plot_file(&directory, "nope/x.svg", "<svg/>")
        .expect_err("the missing parent is refused");
    requires_plot_file_write_exception(&failure);
    assert_eq!(
        std::any::type_name::<PlotFileWriteException>(),
        "lovelace_lang::plot_file::PlotFileWriteException"
    );
    let classified = failure.to_lang_error();
    assert_eq!(classified.code, "PlotFileError");
    assert_eq!(classified.category, "TypeMismatch");
    assert!(classified.message.contains(failure.requested()));
}

/// PlotFileUnderAMissingDirectory_IsRefusedByType: the write refuses with the typed exception,
/// names Path.Combine(root, plotFileName), and the engine publishes the same classification.
#[test]
fn plot_file_under_a_missing_directory_is_refused_by_type() {
    let directory = plot_dir("round90-plot-missing");
    let expected = write_plot_file(&directory, "nope/x.svg", "<svg/>")
        .expect_err("the missing parent is refused");
    assert_eq!(expected.failure(), PlotFileWriteFailure::PathNotFound);
    let requested = Path::new(&directory)
        .join("nope/x.svg")
        .to_string_lossy()
        .into_owned();
    assert_eq!(expected.requested(), requested);

    let mut engine = Engine::new();
    engine.set_plot_dir(&directory);
    engine.set_plot_file("nope/x.svg");
    let error = engine
        .eval("plot([1, 2, 3])")
        .expect_err("the plot write is refused");
    assert_eq!(error.code, "PlotFileError");
    assert_eq!(error.category, "TypeMismatch");
    assert_eq!(error.message, expected.to_string());
    assert!(error.message.contains(&requested), "{}", error.message);
}

/// EmptyPlotFile_IsRefusedByType: an empty file name is the plot directory itself, refused as
/// the typed access denial that names the requested path.
#[test]
fn empty_plot_file_is_refused_by_type() {
    let directory = plot_dir("round90-plot-empty");
    let expected = write_plot_file(&directory, "", "<svg/>").expect_err("a directory is refused");
    assert_eq!(expected.failure(), PlotFileWriteFailure::AccessDenied);
    assert_eq!(expected.requested(), directory);
    assert!(
        expected.detail().ends_with("' is denied."),
        "{}",
        expected.detail()
    );

    let mut engine = Engine::new();
    engine.set_plot_dir(&directory);
    engine.set_plot_file("");
    let error = engine
        .eval("plot([1, 2, 3])")
        .expect_err("the plot write is refused");
    assert_eq!(error.code, "PlotFileError");
    assert_eq!(error.category, "TypeMismatch");
    assert_eq!(error.message, expected.to_string());
    assert!(error.message.contains(&directory), "{}", error.message);
}

// ---------------------------------------------------------------------------
// Lovelace.Dsp.Tests/DspTests
// ---------------------------------------------------------------------------

/// Sum_GivenImpulseAndStep_AddsPointwise: Sum(Impulse, Step).Get(0) == 2, Get(1) == 1.
#[test]
fn sum_given_impulse_and_step_adds_pointwise() {
    let sum = Sum::new(Impulse, Step);
    assert_eq!(sum.get(0), complex(2, 0));
    assert_eq!(sum.get(1), complex(1, 0));
    // Outside the reference's two asserted samples both signals are zero.
    assert_eq!(sum.get(-1), complex(0, 0));
}

/// Product_GivenImpulseAndStep_MultipliesPointwise: Product(Impulse, Step).Get(0) == 1,
/// Get(1) == 0.
#[test]
fn product_given_impulse_and_step_multiplies_pointwise() {
    let product = Product::new(Impulse, Step);
    assert_eq!(product.get(0), complex(1, 0));
    assert_eq!(product.get(1), complex(0, 0));
    assert_eq!(product.get(-1), complex(0, 0));
}

/// Sequence_GivenLengthMismatch_ThrowsArgumentException: Sequence(0, 2, [One, One]) requires
/// three values, so the constructor refuses with the typed length-mismatch error.
#[test]
fn sequence_given_length_mismatch_throws_argument_exception() {
    let failure = Sequence::new(0, 2, vec![Complex::one(), Complex::one()])
        .expect_err("two values cannot cover [0, 2]");
    assert_eq!(failure.expected(), 3);
    assert_eq!(failure.got(), 2);
    assert_eq!(
        failure.to_string(),
        "Sequence over [0,2] requires 3 value(s), but got 2. (Parameter 'values')"
    );
    // The matching sequence is accepted, so the refusal is the length check, not the shape.
    assert!(Sequence::new(0, 1, vec![Complex::one(), Complex::one()]).is_ok());
}

// ---------------------------------------------------------------------------
// Lovelace.Complex.Tests/DenseArrayComplexTests
// ---------------------------------------------------------------------------

fn complex_array() -> DenseArray<Complex> {
    DenseArray::new(
        vec![2],
        vec![complex(1, 2), complex(3, -4)],
        DType::Complex,
        Precision::new(16),
    )
    .expect("a packed complex array")
}

/// DenseArrayOfComplex_GivenConstruction_ReportsComplexDTypeAndElementType.
#[test]
fn dense_array_of_complex_given_construction_reports_complex_dtype_and_element_type() {
    let array = complex_array();
    assert_eq!(array.dtype(), DType::Complex);
    assert_eq!(array.element_type(), TypeId::of::<Complex>());
    assert_eq!(array.numel(), 2);
    assert!(array.is_contiguous());
    assert_eq!(array.shape(), &[2]);
    assert_eq!(array.rank(), 1);
    assert_eq!(array.precision().significant_digits(), 16);
}

/// DenseArrayOfComplex_GivenElements_GetElementRoundTrips.
#[test]
fn dense_array_of_complex_given_elements_get_element_round_trips() {
    let array = complex_array();
    assert_eq!(array.get_element(0), Some(&complex(1, 2)));
    assert_eq!(array.get_element(1), Some(&complex(3, -4)));
}

/// DenseArrayOfComplex_GivenContiguous_AsSpanIterates.
#[test]
fn dense_array_of_complex_given_contiguous_as_span_iterates() {
    let array = complex_array();
    let span = array.as_span();
    assert_eq!(span[0], complex(1, 2));
    assert_eq!(span[1], complex(3, -4));
    assert_eq!(span.len(), 2);
}

// ---------------------------------------------------------------------------
// Lovelace.Integer.Tests/IntegerToStringTests
// ---------------------------------------------------------------------------

/// TryFormat_GivenSufficientBuffer_WritesRepresentationAndReturnsTrue.
#[test]
fn try_format_given_sufficient_buffer_writes_representation_and_returns_true() {
    let value = Int::from_i64(-42);
    let mut destination = ['\0'; 10];
    let written = value
        .try_format(&mut destination)
        .expect("ten chars hold -42");
    assert_eq!(written, 3);
    assert_eq!(destination[..written].iter().collect::<String>(), "-42");
}

/// TryFormat_GivenInsufficientBuffer_ReturnsFalse: the false branch reports zero chars written
/// and leaves the destination untouched.
#[test]
fn try_format_given_insufficient_buffer_returns_false() {
    let value = Int::from_i64(-12345);
    let mut destination = ['\0'; 3];
    assert_eq!(value.try_format(&mut destination), None);
    assert!(destination.iter().all(|ch| *ch == '\0'));
}

// ---------------------------------------------------------------------------
// Lovelace.Suite.Tests/ParserTests
// ---------------------------------------------------------------------------

/// Parse_GivenChainedAssignment_RightAssociative: x = y = 5 is Assign(x, Assign(y, 5)).
#[test]
fn parse_given_chained_assignment_right_associative() {
    let tokens = tokenize("x = y = 5").expect("the source tokenises");
    let expression = Parser::new(tokens)
        .single_expression()
        .expect("chained assignment parses");
    match expression {
        Expr::Assign(name, value) => {
            assert_eq!(name, "x");
            match *value {
                Expr::Assign(inner_name, inner_value) => {
                    assert_eq!(inner_name, "y");
                    assert!(
                        matches!(*inner_value, Expr::Number(ref text) if text == "5"),
                        "the innermost value is not 5: {inner_value:?}"
                    );
                }
                other => panic!("the value is not an assignment: {other:?}"),
            }
        }
        other => panic!("not an assignment: {other:?}"),
    }

    let mut engine = Engine::new();
    let value = engine
        .eval("x = y = 5")
        .expect("chained assignment evaluates");
    assert_eq!(engine.format_typed(&value), "5 (Natural)");
    let variables = engine.variables();
    assert!(
        variables
            .iter()
            .any(|(name, text)| name == "x" && text == "5 (Natural)")
    );
    assert!(
        variables
            .iter()
            .any(|(name, text)| name == "y" && text == "5 (Natural)")
    );
}

/// Parse_GivenUnaryPlus_ReturnsUnaryExprPlus: +5 is UnaryExpr(Plus, 5).
#[test]
fn parse_given_unary_plus_returns_unary_expr_plus() {
    let tokens = tokenize("+5").expect("the source tokenises");
    let expression = Parser::new(tokens)
        .single_expression()
        .expect("unary plus parses");
    match expression {
        Expr::Plus(inner) => assert!(
            matches!(*inner, Expr::Number(ref text) if text == "5"),
            "the operand is not 5: {inner:?}"
        ),
        other => panic!("not a unary plus: {other:?}"),
    }

    let mut engine = Engine::new();
    let value = engine.eval("+5").expect("unary plus evaluates");
    assert_eq!(engine.format_typed(&value), "5 (Natural)");
}
