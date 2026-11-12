//! Round 84: the evalir-multi family.
//!
//! The four deferred census rows of
//! `Lovelace.Symbolics.Tests/DxStructuredResultsTests` whose contract needs a MULTI-PARAMETER
//! kernel. The script is the row's own input; every expected value is the oracle's measured
//! answer, probed with
//! `dotnet exec Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll --eval <row> --omit-functions`
//! against `rust/target/release/lovelace-run.exe`:
//!
//! * `compile_full(x + y, [x, y]).result_type` was `unknown` in the oracle and `Integer` here;
//! * `evalir_batch(compile(x*y + 1, [x, y]), [[1, 2], [3, 4]], 40)` was `[3, 13]` and refused
//!   `evalir supports one free symbol.`;
//! * a row of the wrong width was the documented row-width refusal and was refused above;
//! * a flat list that is not a whole number of rows was the documented whole-rows refusal.
//!
//! The boundary cases below pin the same path at zero parameters (the compilation contract's own
//! refusal, and a constant zero-parameter kernel read from `lower`), one parameter (the single-
//! lane contract the multi-parameter path must not disturb) and duplicate parameter names (the
//! reference's LAST-position binding, whose out-of-range operand validation refuses the compile).

use crate::{Engine, LangError, Value};

fn eval(script: &str) -> Value {
    let mut engine = Engine::new();
    eval_in(&mut engine, script)
}

fn eval_in(engine: &mut Engine, script: &str) -> Value {
    engine
        .eval(script)
        .unwrap_or_else(|error| panic!("script: {script} failed: {}", error.message))
}

fn refusal(script: &str) -> LangError {
    match Engine::new().eval(script) {
        Ok(value) => panic!(
            "script: {script} answered {} instead of refusing",
            value.typed()
        ),
        Err(error) => error,
    }
}

fn display(script: &str) -> String {
    eval(script).display()
}

fn typed(script: &str) -> String {
    eval(script).typed()
}

fn record_fields<'a>(value: &'a Value, type_name: &str) -> &'a [(String, Value)] {
    match value {
        Value::Record(name, fields) if name == type_name => fields,
        Value::Record(name, _) => panic!("expected record {type_name}, got {name}"),
        other => panic!("expected record {type_name}, got {}", other.kind_name()),
    }
}

fn field<'a>(fields: &'a [(String, Value)], name: &str) -> &'a Value {
    fields
        .iter()
        .find(|(field, _)| field == name)
        .map(|(_, value)| value)
        .unwrap_or_else(|| panic!("record has no field {name}"))
}

fn text(value: &Value) -> &str {
    match value {
        Value::Text(text) => text,
        other => panic!("expected Text, got {}", other.kind_name()),
    }
}

// ---------------------------------------------------------------------------
// DxStructuredResultsTests/EvalirBatch_AcceptsNestedRowValues#0
// ---------------------------------------------------------------------------

/// Census row `EvalirBatch_AcceptsNestedRowValues#0` (DxStructuredResultsTests.cs:424).
///
/// A two-parameter kernel accepts row-shaped array literals (row-major) and evaluates row by row.
/// The kernel record's `result_type` is the FORWARD inference's answer for an unconstrained
/// parameter, `unknown` -- never the symbolic domain's `Integer`.
#[test]
fn evalir_batch_accepts_nested_row_values() {
    let mut engine = Engine::new();
    eval_in(&mut engine, "x = symbol(\"x\")");
    eval_in(&mut engine, "y = symbol(\"y\")");
    let kernel = eval_in(&mut engine, "compile_full(x + y, [x, y])");
    let fields = record_fields(&kernel, "CompilationResult");
    assert_eq!(text(field(fields, "result_type")), "unknown");
    // ...while a constant-bearing kernel still infers its narrowest type.
    let integer_kernel = eval_in(&mut engine, "compile_full(x^2 + 1, [x])");
    let integer_fields = record_fields(&integer_kernel, "CompilationResult");
    assert_eq!(text(field(integer_fields, "result_type")), "Integer");

    let batch = eval_in(
        &mut engine,
        "evalir_batch(compile_full(x + y, [x, y]), [[1, 2], [3, 4]], 40)",
    );
    assert_eq!(batch.typed(), "[3, 7] (Vector)");
    assert!(matches!(kernel, Value::Record(ref name, _) if name == "CompilationResult"));
}

// ---------------------------------------------------------------------------
// DxStructuredResultsTests/EvalirBatch_RowOrientedInput_EvaluatesRowByRow#0
// ---------------------------------------------------------------------------

/// Census row `EvalirBatch_RowOrientedInput_EvaluatesRowByRow#0` (DxStructuredResultsTests.cs:252).
///
/// Row-oriented input evaluates row by row: `[[1, 2], [3, 4]]` pairs x=1/y=2 and x=3/y=4, so
/// `x*y + 1` answers `[3, 13]`. A flat row-major list of the same four values is the same two
/// rows, and neither form is re-flowed into different parameter pairings.
#[test]
fn evalir_batch_row_oriented_input_evaluates_row_by_row() {
    let mut engine = Engine::new();
    eval_in(&mut engine, "x = symbol(\"x\")");
    eval_in(&mut engine, "y = symbol(\"y\")");
    let rows = eval_in(
        &mut engine,
        "evalir_batch(compile(x*y + 1, [x, y]), [[1, 2], [3, 4]], 40)",
    );
    assert_eq!(rows.typed(), "[3, 13] (Vector)");
    let flat = eval_in(
        &mut engine,
        "evalir_batch(compile(x*y + 1, [x, y]), [1, 2, 3, 4], 40)",
    );
    assert_eq!(flat.typed(), "[3, 13] (Vector)");
}

// ---------------------------------------------------------------------------
// DxStructuredResultsTests/EvalirBatch_RowWidthMismatch_IsRejectedWithTheDocumentedShape#0
// ---------------------------------------------------------------------------

/// Census row `EvalirBatch_RowWidthMismatch_IsRejectedWithTheDocumentedShape#0`
/// (DxStructuredResultsTests.cs:261). Three values per row for a two-parameter kernel is refused
/// with the documented row-width sentence naming the kernel width and the offending row, instead
/// of being re-flowed into different rows (which would silently pair the wrong parameters).
#[test]
fn evalir_batch_row_width_mismatch_is_rejected_with_the_documented_shape() {
    let error = refusal(
        "x = symbol(\"x\"); y = symbol(\"y\"); evalir_batch(compile(x*y + 1, [x, y]), [[1, 2, 5], [3, 4, 6]], 40)",
    );
    assert_eq!(
        error.message,
        "evalir_batch(): kernel expects 2 parameters per row; row 0 contains 3."
    );
    assert_eq!(error.code, "InvalidOperation");
    assert_eq!(error.category, "DomainError");
}

// ---------------------------------------------------------------------------
// DxStructuredResultsTests/EvalirBatch_FlatInputNotAWholeNumberOfRows_IsRejected#0
// ---------------------------------------------------------------------------

/// Census row `EvalirBatch_FlatInputNotAWholeNumberOfRows_IsRejected#0`
/// (DxStructuredResultsTests.cs:273). A flat list whose length is not a multiple of the kernel
/// width is refused with the documented whole-rows sentence, naming both counts.
#[test]
fn evalir_batch_flat_input_not_a_whole_number_of_rows_is_rejected() {
    let error = refusal(
        "x = symbol(\"x\"); y = symbol(\"y\"); evalir_batch(compile(x*y + 1, [x, y]), [1, 2, 3], 40)",
    );
    assert_eq!(
        error.message,
        "evalir_batch(): kernel expects 2 parameters per row; got 3 values, which is not a whole number of rows."
    );
    assert_eq!(error.code, "InvalidOperation");
    assert_eq!(error.category, "DomainError");
}

// ---------------------------------------------------------------------------
// The family's argument mapping and arity contract
// ---------------------------------------------------------------------------

/// The values are bound to the kernel's parameter list BY POSITION, in declaration order:
/// `compile(x - y, [x, y])` at `[10, 3]` is 7, while the same expression compiled over
/// `[y, x]` is -7. `evalir` takes exactly one value per parameter (the scalar arity contract),
/// and a single-row nested list is the same point, never a one-element list.
#[test]
fn evalir_multi_parameter_binds_values_in_declaration_order() {
    assert_eq!(
        typed("x = symbol(\"x\"); y = symbol(\"y\"); evalir(compile(x - y, [x, y]), [10, 3], 40)"),
        "7 (Integer)"
    );
    assert_eq!(
        typed("x = symbol(\"x\"); y = symbol(\"y\"); evalir(compile(x - y, [y, x]), [10, 3], 40)"),
        "-7 (Integer)"
    );
    assert_eq!(
        typed(
            "x = symbol(\"x\"); y = symbol(\"y\"); evalir(compile(x - y, [x, y]), [[10, 3]], 40)"
        ),
        "7 (Integer)"
    );
    let error =
        refusal("x = symbol(\"x\"); y = symbol(\"y\"); evalir(compile(x - y, [x, y]), [10], 40)");
    assert_eq!(
        error.message,
        "evalir(): kernel expects 2 parameters; got 1 values."
    );
    assert_eq!(error.code, "InvalidOperation");
    assert_eq!(error.category, "DomainError");
}

// ---------------------------------------------------------------------------
// Boundary: zero parameters
// ---------------------------------------------------------------------------

/// Zero parameters: the compilation contract refuses an empty parameter list outright, while the
/// UNVALIDATED `lower` still publishes a constant zero-parameter kernel. Reading that kernel
/// back covers the zero-width batch contract: one value per row is not a whole number of rows
/// (the reference's DivisionByZero), and an empty list is refused before the width is read.
#[test]
fn evalir_zero_parameter_boundary_is_the_compilation_refusal_and_a_constant_kernel() {
    for script in [
        "x = symbol(\"x\"); compile(x, [])",
        "x = symbol(\"x\"); compile_full(x, [])",
    ] {
        let error = refusal(script);
        assert_eq!(
            error.message,
            "Compilation requires at least one parameter. (Parameter 'parameters')"
        );
        assert_eq!(error.code, "InvalidArgument");
        assert_eq!(error.category, "TypeMismatch");
    }

    // A constant expression over no parameters lowers (and only lowers) to a zero-parameter
    // kernel; evalir evaluates it once, with no values to bind.
    assert_eq!(
        display("x = symbol(\"x\"); lower(x-x, [])"),
        "#!mathir 2\nconst (rat 0 1)\nConstant 0 0"
    );
    assert_eq!(
        typed("x = symbol(\"x\"); evalir(lower(x-x, []), [], 40)"),
        "0 (Integer)"
    );
    let error = refusal("x = symbol(\"x\"); evalir(lower(x-x, []), [7], 40)");
    assert_eq!(
        error.message,
        "evalir(): kernel expects 0 parameters; got 1 values."
    );
    let error = refusal("x = symbol(\"x\"); evalir_batch(lower(x-x, []), [], 40)");
    assert_eq!(error.message, "evalir_batch(): values must not be empty.");
    let error = refusal("x = symbol(\"x\"); evalir_batch(lower(x-x, []), [7], 40)");
    assert_eq!(error.message, "Attempted to divide by zero.");
    assert_eq!(error.code, "DivisionByZero");
    assert_eq!(error.category, "DomainError");
}

// ---------------------------------------------------------------------------
// Boundary: one parameter
// ---------------------------------------------------------------------------

/// One parameter: the single-lane contract the multi-parameter path must not disturb. A scalar
/// call, a flat batch, a one-column row list and `compile_full`'s inferred type all stay on the
/// one-parameter answers.
#[test]
fn evalir_one_parameter_boundary_keeps_the_single_lane_contract() {
    assert_eq!(
        typed("x = symbol(\"x\"); evalir(compile(x^2 + 1, [x]), [2], 40)"),
        "5 (Integer)"
    );
    assert_eq!(
        typed("x = symbol(\"x\"); evalir_batch(compile(x^2 + 1, [x]), [1, 2, 3], 40)"),
        "[2, 5, 10] (Vector)"
    );
    assert_eq!(
        typed("x = symbol(\"x\"); evalir_batch(compile(x^2 + 1, [x]), [[1], [2]], 40)"),
        "[2, 5] (Vector)"
    );
    assert_eq!(
        text(field(
            record_fields(
                &eval("x = symbol(\"x\"); compile_full(x^2 + 1, [x])"),
                "CompilationResult"
            ),
            "result_type"
        )),
        "Integer"
    );
}

// ---------------------------------------------------------------------------
// Boundary: duplicate parameter names
// ---------------------------------------------------------------------------

/// Duplicate parameter names: the reference's name -> index dictionary is filled in declaration
/// order, so the LAST occurrence wins. `lower` is unvalidated and publishes that out-of-range
/// operand; `compile`/`compile_full`/`evalir` validate the program and refuse it as the
/// reference's IR validation does.
#[test]
fn evalir_duplicate_parameter_names_bind_last_and_compilation_refuses() {
    assert_eq!(
        display("x = symbol(\"x\"); lower(x, [x, x])"),
        "#!mathir 2\nparam x\nparam x\nParameter 0 1"
    );
    for script in [
        "x = symbol(\"x\"); compile(x, [x, x])",
        "x = symbol(\"x\"); compile_full(x, [x, x])",
        "x = symbol(\"x\"); evalir(lower(x, [x, x]), [2, 3], 40)",
    ] {
        let error = refusal(script);
        assert_eq!(
            error.message,
            "IR validation: node 0 (Parameter) references out-of-range operand 1."
        );
        assert_eq!(error.code, "InvalidOperation");
        assert_eq!(error.category, "DomainError");
    }
}
