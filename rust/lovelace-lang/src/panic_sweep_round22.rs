//! Round 22 panic-sweep regression tests: every input below reached a panic (or an abort) in the
//! builtins before the fix. Each expected code/category/message is the pinned oracle's record for
//! the same script, captured in docs/goal-parity-contract/round-22/panic-sweep.md.

use crate::{Engine, LangError, Value};

fn eval(script: &str) -> Result<Value, LangError> {
    Engine::new().eval(script)
}

/// Asserts that the script is refused with the oracle's exact wire record, and that it is a
/// RECOVERABLE typed refusal rather than a panic.
fn assert_refusal(script: &str, code: &str, category: &str, message: &str) {
    match eval(script) {
        Ok(value) => panic!(
            "{script}: expected a refusal, evaluated to {}",
            value.display()
        ),
        Err(error) => {
            assert_eq!(
                error.code, code,
                "{script}: code (message: {})",
                error.message
            );
            assert_eq!(
                error.category, category,
                "{script}: category (message: {})",
                error.message
            );
            assert_eq!(error.message, message, "{script}: message");
        }
    }
}

fn display(script: &str) -> String {
    eval(script)
        .unwrap_or_else(|error| panic!("{script}: {}", error.message))
        .display()
}

// ---------------------------------------------------------------------------
// Site 1: eye() multiplied the dimension by itself before any bound check
// (builtins.rs line 2309: attempt to multiply with overflow, exit 101).
// ---------------------------------------------------------------------------

#[test]
fn eye_rows_beyond_i64_is_the_oracles_argument_refusal() {
    assert_refusal(
        "eye(9223372036854775808)",
        "InvalidArgument",
        "TypeMismatch",
        "eye(): argument 1 (rows) must be a Natural or Integer that is a positive count (1 or more, at most 9223372036854775807); got 9223372036854775808.",
    );
}

#[test]
fn eye_shape_over_the_allocation_budget_is_refused_before_the_allocator() {
    assert_refusal(
        "eye(4294967296)",
        "AllocationRefused",
        "BudgetExceeded",
        "eye() requested one array of shape [4294967296, 4294967296], which holds more than 9223372036854775807 element(s) — more than the maximum single-allocation budget of 268435456 element(s). The request is refused BEFORE the allocation is attempted. Ask for a smaller shape, or process the data in pieces.",
    );
}

#[test]
fn eye_zero_and_negative_rows_are_positive_count_refusals_not_magnitudes() {
    let positive = "eye(): argument 1 (rows) must be a Natural or Integer that is a positive count (1 or more, at most 9223372036854775807); got ";
    assert_refusal(
        "eye(0)",
        "InvalidArgument",
        "TypeMismatch",
        &format!("{positive}0."),
    );
    assert_refusal(
        "eye(-1)",
        "InvalidArgument",
        "TypeMismatch",
        &format!("{positive}-1."),
    );
}

// ---------------------------------------------------------------------------
// Site 2: zeros()/ones() pushed a u64 dimension into usize and multiplied the
// shape with unchecked product (capacity overflow / attempt to multiply with
// overflow, exit 101).
// ---------------------------------------------------------------------------

#[test]
fn array_dimensions_beyond_i64_are_argument_refusals() {
    let dim = "must be a Natural or Integer that is a non-negative count (0 or more, at most 9223372036854775807); got ";
    assert_refusal(
        "zeros(9223372036854775808)",
        "InvalidArgument",
        "TypeMismatch",
        &format!("zeros(): argument 1 (a dimension) {dim}9223372036854775808."),
    );
    assert_refusal(
        "zeros(2, 9223372036854775808)",
        "InvalidArgument",
        "TypeMismatch",
        &format!("zeros(): argument 2 (a dimension) {dim}9223372036854775808."),
    );
    assert_refusal(
        "ones(18446744073709551615)",
        "InvalidArgument",
        "TypeMismatch",
        &format!("ones(): argument 1 (a dimension) {dim}18446744073709551615."),
    );
}

#[test]
fn array_shapes_over_the_allocation_budget_are_refused_before_the_allocator() {
    assert_refusal(
        "zeros(9223372036854775807)",
        "AllocationRefused",
        "BudgetExceeded",
        "zeros() requested one array of shape [9223372036854775807], which holds 9223372036854775807 element(s) — more than the maximum single-allocation budget of 268435456 element(s). The request is refused BEFORE the allocation is attempted. Ask for a smaller shape, or process the data in pieces.",
    );
    assert_refusal(
        "zeros(4294967296, 4294967296)",
        "AllocationRefused",
        "BudgetExceeded",
        "zeros() requested one array of shape [4294967296, 4294967296], which holds more than 9223372036854775807 element(s) — more than the maximum single-allocation budget of 268435456 element(s). The request is refused BEFORE the allocation is attempted. Ask for a smaller shape, or process the data in pieces.",
    );
}

#[test]
fn a_negative_dimension_is_refused_instead_of_read_as_its_magnitude() {
    // Before the fix zeros(-1) silently built a one-element zero vector.
    assert_refusal(
        "zeros(-1)",
        "InvalidArgument",
        "TypeMismatch",
        "zeros(): argument 1 (a dimension) must be a Natural or Integer that is a non-negative count (0 or more, at most 9223372036854775807); got -1.",
    );
}

#[test]
fn a_zero_dimension_keeps_the_shape_constructible() {
    // The budget contract (D5) never refuses a shape that holds no elements.
    assert_eq!(display("numel(zeros(9223372036854775807, 0))"), "0");
    assert_eq!(display("zeros(2, 3)"), "[[0, 0, 0], [0, 0, 0]]");
    assert_eq!(display("eye(3)"), "[[1, 0, 0], [0, 1, 0], [0, 0, 1]]");
}

// ---------------------------------------------------------------------------
// Site 3: make_range() accepted a bound wider than i64 as a Real and looped to
// its one-million-element cap (multi-second wait, wrong refusal).
// ---------------------------------------------------------------------------

#[test]
fn a_range_bound_beyond_i64_is_refused_by_name() {
    assert_refusal(
        "1..9223372036854775808",
        "InvalidArgument",
        "TypeMismatch",
        "range(): argument 'bound' must be a Natural or Integer that fits a signed 64-bit integer (at most 9223372036854775807 in absolute value); got 9223372036854775808.",
    );
}

#[test]
fn a_range_step_beyond_i64_is_refused_by_name() {
    assert_refusal(
        "1..9223372036854775808..3",
        "InvalidArgument",
        "TypeMismatch",
        "range(): argument 'step' must be a Natural or Integer that is a positive count (1 or more, at most 9223372036854775807); got 9223372036854775808.",
    );
    assert_eq!(display("1..5"), "[1, 2, 3, 4, 5]");
}

// ---------------------------------------------------------------------------
// Site 4: slice_indices() negated a magnitude and then added the step without
// overflow checks (attempt to negate / add with overflow, exit 101).
// ---------------------------------------------------------------------------

#[test]
fn slice_bounds_at_i64_min_are_clipped_instead_of_overflowing() {
    assert_eq!(display("[1, 2, 3][-9223372036854775808:1]"), "[1]");
    assert_eq!(display("[1, 2, 3][0:-9223372036854775808]"), "[]");
}

#[test]
fn a_slice_step_that_would_overflow_ends_the_slice_at_the_last_index() {
    assert_eq!(
        display("[1, 2, 3][9223372036854775806:9223372036854775807:2]"),
        "[]"
    );
    assert_eq!(display("[1, 2, 3][0:1:9223372036854775807]"), "[1]");
}

#[test]
fn slice_bounds_beyond_i64_are_refused_by_name() {
    assert_refusal(
        "[1, 2, 3][0:9223372036854775808]",
        "InvalidArgument",
        "TypeMismatch",
        "range(): argument 'bound' must be a Natural or Integer that fits a signed 64-bit integer (at most 9223372036854775807 in absolute value); got 9223372036854775808.",
    );
    assert_refusal(
        "[1, 2, 3][0:1:9223372036854775808]",
        "InvalidArgument",
        "TypeMismatch",
        "range(): argument 'step' must be a Natural or Integer that is a positive count (1 or more, at most 9223372036854775807); got 9223372036854775808.",
    );
}

#[test]
fn reshape_dimensions_use_the_same_exact_integer_contract() {
    assert_refusal(
        "reshape([1, 2], 2.5)",
        "InvalidArgument",
        "TypeMismatch",
        "reshape(): argument 1 (a dimension) must be a Natural or Integer that denotes an exact integer; got the Real 2.5.",
    );
    assert_eq!(display("reshape([1, 2, 3, 4], 2, 2)"), "[[1, 2], [3, 4]]");
}

// ---------------------------------------------------------------------------
// Site 5: a Real exponent wider than i64 was cast with as, wrapping to a
// negative exponent and silently inverting the power.
// ---------------------------------------------------------------------------

#[test]
fn squeeze_of_singletons_is_a_rank_one_vector_and_len_does_not_panic() {
    // len(squeeze([[[1]]])) panicked on the rank-0 shape before the fix.
    assert_eq!(display("len(squeeze([[[1]]]))"), "1");
    assert_eq!(display("type(squeeze([[[1]]]))"), "Vector");
    assert_eq!(display("shape(squeeze([[[1]]]))"), "[1]");
    assert_eq!(display("rank(squeeze([[[1]]]))"), "1");
    assert_eq!(display("squeeze([[[1, 2]]])"), "[1, 2]");
}

#[test]
fn a_series_order_beyond_i64_is_the_oracles_argument_refusal() {
    assert_refusal(
        "x = symbol(\"x\"); series(sin(x), x, 0, 9223372036854775808)",
        "InvalidArgument",
        "TypeMismatch",
        "series(): argument 4 (order) must be a Natural or Integer that denotes an exact integer; got the Natural 9223372036854775808.",
    );
    assert_refusal(
        "x = symbol(\"x\"); series(sin(x), x, 0, 2147483648)",
        "InvalidArgument",
        "TypeMismatch",
        "series(): argument 4 (order) must be a Natural or Integer between 1 and 2147483647; got 2147483648.",
    );
    assert_refusal(
        "x = symbol(\"x\"); series(sin(x), x, 0, 2.5)",
        "InvalidArgument",
        "TypeMismatch",
        "series(): argument 4 (order) must be a Natural or Integer that denotes an exact integer; got the Real 2.5.",
    );
}

#[test]
fn a_real_exponent_beyond_i64_is_the_oracles_arithmetic_refusal() {
    assert_refusal(
        "2.0^9223372036854775808",
        "ArithmeticError",
        "DomainError",
        "Value was either too large or too small for an Int64.",
    );
    assert_eq!(display("2.0^5"), "32");
}
