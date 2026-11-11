//! Round-27 ports of the Run/Suite production gaps the round-19 refusal table named.
//!
//! Each test repeats the C# assertion it unblocks: the evalir digit contract, the wide-argument
//! refusals (zeros/ones/series already conformant — pinned here as regression guards), the index
//! taxonomy, the solve session-assumption store, the evalf digit-cap notice, the two-argument
//! eye/mean/norm forms and the runner-neutral BOM/UTF-16 position rules.

use crate::{Engine, LangError, Value};

fn eval(script: &str) -> Result<Value, LangError> {
    let mut engine = Engine::new();
    engine.eval(script)
}

fn display(script: &str) -> String {
    eval(script)
        .unwrap_or_else(|e| panic!("'{script}' failed: {e}"))
        .display()
}

fn refusal(script: &str) -> LangError {
    match eval(script) {
        Err(error) => error,
        Ok(value) => panic!("'{script}' answered {}", value.display()),
    }
}

fn record_field<'a>(value: &'a Value, name: &str) -> &'a Value {
    match value {
        Value::Record(_, fields) => fields
            .iter()
            .find(|(field, _)| field.as_str() == name)
            .map(|(_, value)| value)
            .unwrap_or_else(|| panic!("record has no field '{name}'")),
        other => panic!("expected a record, got {}", other.kind_name()),
    }
}

fn solutions(script: &str) -> Vec<String> {
    let value = eval(script).unwrap_or_else(|e| panic!("'{script}' failed: {e}"));
    match record_field(&value, "solutions") {
        Value::Vector(items) => items
            .iter()
            .map(|solution| record_field(solution, "value").display())
            .collect(),
        other => panic!("solutions is {}", other.kind_name()),
    }
}

fn status(script: &str) -> String {
    let value = eval(script).unwrap_or_else(|e| panic!("'{script}' failed: {e}"));
    record_field(&value, "status").display()
}

fn family_conditions(script: &str) -> Vec<String> {
    let value = eval(script).unwrap_or_else(|e| panic!("'{script}' failed: {e}"));
    match record_field(&value, "families") {
        Value::Vector(families) => families
            .iter()
            .flat_map(|family| match record_field(family, "conditions") {
                Value::Vector(items) => items.iter().map(Value::display).collect::<Vec<_>>(),
                _ => Vec::new(),
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn structured_json(script: &str) -> lovelace_json::Json {
    let mut engine = Engine::new();
    let value = engine
        .eval(script)
        .unwrap_or_else(|e| panic!("'{script}' failed: {e}"));
    crate::structured::project_value(&engine, &value, None)
}

fn json_field<'a>(json: &'a lovelace_json::Json, name: &str) -> Option<&'a lovelace_json::Json> {
    match json {
        lovelace_json::Json::Obj(fields) => fields
            .iter()
            .find(|(field, _)| field.as_str() == name)
            .map(|(_, value)| value),
        _ => None,
    }
}

fn json_text(json: &lovelace_json::Json, name: &str) -> String {
    match json_field(json, name) {
        Some(lovelace_json::Json::Str(text)) => text.clone(),
        Some(other) => other.to_string(),
        None => panic!("json has no field '{name}': {}", json.to_string()),
    }
}

fn json_flag(json: &lovelace_json::Json, name: &str) -> bool {
    matches!(
        json_field(json, name),
        Some(lovelace_json::Json::Bool(true))
    )
}

// ---------------------------------------------------------------------------
// Item 1 — evalir/evalir_batch digit plumbing (EvalirDigitArgumentContractTests,
// PluginIntegerArgumentWireTests, IntegerArgumentBoundaryMatrixTests)
// ---------------------------------------------------------------------------

#[test]
fn evalir_honours_the_digit_count_it_was_given() {
    // 0.3(3) truncated to the requested count: 2 places answer 0.33, 40 answer forty 3s.
    assert_eq!(
        display("x = symbol(\"x\"); evalir(compile(x*1/3, [x]), [1], 2)"),
        "0.33"
    );
    let forty = display("x = symbol(\"x\"); evalir(compile(x*1/3, [x]), [1], 40)");
    assert_eq!(forty, format!("0.{}", "3".repeat(40)));
    let twenty = display("x = symbol(\"x\"); evalir(compile(x*1/3, [x]), [1], 20)");
    assert_eq!(twenty, format!("0.{}", "3".repeat(20)));
}

#[test]
fn evalir_the_goal_probe_is_ten_at_every_digit_count() {
    for digits in [5, 20, 40] {
        let script = format!("x = symbol(\"x\"); evalir(compile(x^2+1,[x]), [3], {digits})");
        assert_eq!(display(&script), "10", "at {digits} digits");
    }
}

#[test]
fn evalir_refuses_a_non_integral_digit_count_without_flooring() {
    let error = refusal("x = symbol(\"x\"); evalir(compile(x*1/3, [x]), [1], 2.9)");
    assert_eq!(error.code, "InvalidArgument");
    assert_eq!(error.category, "TypeMismatch");
    assert_eq!(
        error.message,
        "evalir(): argument 3 (digits) must be a Natural or Integer that denotes an exact integer; got 2.9."
    );
}

#[test]
fn evalir_refuses_counts_outside_the_declared_bound() {
    for digits in ["0", "-5", "10^30"] {
        let script = format!("x = symbol(\"x\"); evalir(compile(x*1/3, [x]), [1], {digits})");
        let error = refusal(&script);
        assert_eq!(error.code, "InvalidArgument");
        assert_eq!(error.category, "TypeMismatch");
        assert!(
            error.message.starts_with(
                "evalir(): argument 3 (digits) must be a Natural or Integer between 1 and 2147483647; got "
            ),
            "{}",
            error.message
        );
        if digits == "10^30" {
            assert!(error.message.contains("1000000000000000000000000000000"));
        }
    }
}

#[test]
fn evalir_refuses_a_text_digit_count_as_the_same_argument_error() {
    let error = refusal("x = symbol(\"x\"); evalir(compile(x*1/3, [x]), [1], \"forty\")");
    assert_eq!(error.code, "InvalidArgument");
    assert_eq!(
        error.message,
        "evalir(): argument 3 (digits) must be a Natural or Integer that denotes an exact integer; got Text."
    );
}

#[test]
fn evalir_batch_carries_the_same_digit_contract() {
    let error = refusal("x = symbol(\"x\"); evalir_batch(compile(x*1/3, [x]), [1, 2], 2.5)");
    assert_eq!(error.code, "InvalidArgument");
    assert_eq!(
        error.message,
        "evalir_batch(): argument 3 (digits) must be a Natural or Integer that denotes an exact integer; got 2.5."
    );
}

#[test]
fn evalir_without_a_digit_count_is_the_declared_arity_refusal() {
    let error = refusal("x = symbol(\"x\"); evalir(compile(x*1/3, [x]), [1])");
    assert_eq!(error.code, "InvalidArgument");
    assert!(
        error
            .message
            .starts_with("evalir(): expected 3 arguments; got 2."),
        "{}",
        error.message
    );
    assert_eq!(error.signature.as_ref().and_then(|s| s.actual), Some(2));
}

// ---------------------------------------------------------------------------
// Item 2 — zeros(-1)/ones(-1)/series(-1): acceptance and exact refusals
// (WideArgumentRefusalTests; already conformant, pinned as guards)
// ---------------------------------------------------------------------------

#[test]
fn zeros_and_ones_refuse_a_negative_dimension_with_the_position_and_value() {
    for (script, expected) in [
        (
            "zeros(-1)",
            "zeros(): argument 1 (a dimension) must be a Natural or Integer that is a non-negative count (0 or more, at most 9223372036854775807); got -1.",
        ),
        (
            "zeros(2, -1)",
            "zeros(): argument 2 (a dimension) must be a Natural or Integer that is a non-negative count (0 or more, at most 9223372036854775807); got -1.",
        ),
        (
            "ones(-1)",
            "ones(): argument 1 (a dimension) must be a Natural or Integer that is a non-negative count (0 or more, at most 9223372036854775807); got -1.",
        ),
        (
            "ones(2, -1)",
            "ones(): argument 2 (a dimension) must be a Natural or Integer that is a non-negative count (0 or more, at most 9223372036854775807); got -1.",
        ),
    ] {
        let error = refusal(script);
        assert_eq!(error.message, expected, "{script}");
        assert_eq!(
            (error.code, error.category),
            ("InvalidArgument", "TypeMismatch")
        );
        assert!(
            !error.message.contains("overflow"),
            "raw CLR text: {}",
            error.message
        );
    }
}

#[test]
fn zeros_and_ones_with_zero_still_answer_an_empty_array() {
    assert_eq!(display("zeros(0)"), "[]");
    assert_eq!(display("ones(0)"), "[]");
}

#[test]
fn series_refuses_a_non_positive_or_too_wide_order() {
    let error = refusal("x = symbol(\"x\"); series(sin(x), x, 0, -1)");
    assert_eq!(
        error.message,
        "series(): argument 4 (order) must be a Natural or Integer between 1 and 2147483647; got -1."
    );
    let zero = refusal("x = symbol(\"x\"); series(sin(x), x, 0, 0)");
    assert!(
        zero.message.ends_with("between 1 and 2147483647; got 0."),
        "{}",
        zero.message
    );
    let wide = refusal("x = symbol(\"x\"); series(sin(x), x, 0, 10^30)");
    assert_eq!(
        wide.message,
        "series(): argument 4 (order) must be a Natural or Integer that denotes an exact integer; got the Natural 1000000000000000000000000000000."
    );
    assert_eq!(
        (wide.code, wide.category),
        ("InvalidArgument", "TypeMismatch")
    );
}

// ---------------------------------------------------------------------------
// Item 3 — the wide-index taxonomy (WideArgumentRefusalTests)
// ---------------------------------------------------------------------------

#[test]
fn an_index_wider_than_int64_is_the_argument_refusal_that_names_it() {
    let error = refusal("[1, 2, 3][10^30]");
    assert_eq!(error.code, "InvalidArgument");
    assert_eq!(error.category, "TypeMismatch");
    assert_eq!(
        error.message,
        "index(): argument 'index' must be a Natural or Integer that fits a signed 64-bit integer (at most 9223372036854775807 in absolute value); got 1000000000000000000000000000000."
    );
}

#[test]
fn a_real_index_is_refused_by_kind_never_floored() {
    let fractional = refusal("[1, 2, 3][1.5]");
    assert_eq!(
        fractional.message,
        "index(): argument 'index' must be a Natural or Integer that denotes an exact integer; got the Real 1.5."
    );
    let integral = refusal("[1, 2, 3][1.0]");
    assert_eq!(
        integral.message,
        "index(): argument 'index' must be a Natural or Integer; got the Real 1, which is integral — write it as 1."
    );
    assert_eq!(integral.code, "InvalidArgument");
}

#[test]
fn a_non_numeric_index_is_the_same_typed_argument_refusal() {
    let error = refusal("[1, 2, 3][\"a\"]");
    assert_eq!(
        error.message,
        "index(): argument 'index' must be a Natural or Integer that denotes an exact integer; got the Text."
    );
}

#[test]
fn a_matrix_index_out_of_range_names_the_dimension_and_its_size() {
    let error = refusal("[[1,2],[3,4]][5, 0]");
    assert_eq!(error.code, "InvalidOperation");
    assert_eq!(error.category, "DomainError");
    assert_eq!(
        error.message,
        "Index 5 is out of range for dimension 0 of size 2."
    );
}

#[test]
fn controls_keep_their_documented_refusals() {
    let negative = refusal("[1, 2, 3][-1]");
    assert_eq!(
        negative.message,
        "Index -1 is out of range for vector of length 3."
    );
    assert_eq!(negative.code, "InvalidOperation");
    let out_of_range = refusal("[1, 2, 3][5]");
    assert_eq!(
        out_of_range.message,
        "Index 5 is out of range for vector of length 3."
    );
    let wide_matrix = refusal("[[1,2],[3,4]][10^30, 0]");
    assert!(
        wide_matrix.message.contains("fits a signed 64-bit integer"),
        "{}",
        wide_matrix.message
    );
}

// ---------------------------------------------------------------------------
// Item 4 — the solve session assumption store (SolveSessionAssumptionTests)
// ---------------------------------------------------------------------------

#[test]
fn solve_keeps_only_the_roots_the_assumption_admits() {
    assert_eq!(
        solutions("x = symbol(\"x\"); assume(x > 0); solve(x^2 == 4, x)"),
        vec!["2"]
    );
    assert_eq!(
        solutions("x = symbol(\"x\"); assume_positive(x); solve(x^2 == 4, x)"),
        vec!["2"]
    );
    assert_eq!(
        solutions("x = symbol(\"x\"); assume(x < 0); solve(x^2 == 4, x)"),
        vec!["-2"]
    );
    assert_eq!(
        solutions("x = symbol(\"x\"); assume(x == 2); solve(x^2 == 4, x)"),
        vec!["2"]
    );
    assert_eq!(
        solutions("x = symbol(\"x\"); assume(x > -10); solve(x^2 == 4, x)"),
        vec!["-2", "2"]
    );
}

#[test]
fn solve_does_not_publish_a_root_the_session_assumption_excludes() {
    let script = "x = symbol(\"x\"); assume(x > 5); solve(x == 3, x)";
    assert_eq!(status(script), "NoSolutions");
    let value = eval(script).expect("record");
    assert_eq!(record_field(&value, "complete").display(), "True");
    assert_eq!(record_field(&value, "completeness").display(), "Complete");
    assert_eq!(solutions(script), Vec::<String>::new());
    // the record's own note array stays empty: the store, not a kernel defect, emptied the set
    match record_field(&value, "diagnostics") {
        Value::Vector(items) => assert!(items.is_empty()),
        other => panic!("diagnostics is {}", other.kind_name()),
    }
}

#[test]
fn solve_honours_a_domain_assumption() {
    assert_eq!(
        status("x = symbol(\"x\"); assume_real(x); solve(x^2 + 1 == 0, x)"),
        "NoSolutions"
    );
    assert_eq!(
        status("x = symbol(\"x\"); assume_integer(x); solve(x == 1/2, x)"),
        "NoSolutions"
    );
    assert!(solutions("x = symbol(\"x\"); assume_real(x); solve(x^2 + 1 == 0, x)").is_empty());
    assert!(solutions("x = symbol(\"x\"); assume_integer(x); solve(x == 1/2, x)").is_empty());
}

#[test]
fn a_response_that_cannot_decide_is_carried_as_a_condition() {
    assert_eq!(
        family_conditions("x = symbol(\"x\"); assume(x > 5); solve(sin(x) == 0, x)"),
        vec!["x > 5"]
    );
}

#[test]
fn one_session_one_assumption_store_for_simplify_and_solve() {
    assert_eq!(
        solutions("x = symbol(\"x\"); assume(x > 0); solve(x^2 == 4, x)"),
        vec!["2"]
    );
    assert_eq!(
        display("x = symbol(\"x\"); assume(x > 0); simplify(sqrt(x^2))"),
        "x"
    );
}

#[test]
fn system_solve_honours_the_same_store() {
    assert_eq!(
        status("x = symbol(\"x\"); assume(x > 5); solve_system([x == 3], [x])"),
        "NoSolutions"
    );
    let filtered = display(
        "x = symbol(\"x\"); y = symbol(\"y\"); assume(y > 0); solve_system([x == 1, y^2 == 4], [x, y])",
    );
    assert!(filtered.contains("value: 2"), "{filtered}");
    assert!(!filtered.contains("value: -2"), "{filtered}");
    let unfiltered =
        display("x = symbol(\"x\"); y = symbol(\"y\"); solve_system([x == 1, y^2 == 4], [x, y])");
    assert!(unfiltered.contains("value: -2"), "{unfiltered}");
}

#[test]
fn controls_without_assumptions_publish_exactly_what_they_did() {
    assert_eq!(
        solutions("x = symbol(\"x\"); solve(x^2 == 4, x)"),
        vec!["-2", "2"]
    );
    assert_eq!(
        solutions("x = symbol(\"x\"); solve(x^2 + 1 == 0, x)"),
        vec!["i", "-i"]
    );
    assert_eq!(
        solutions("x = symbol(\"x\"); y = symbol(\"y\"); assume(y > 5); solve(x == 3, x)"),
        vec!["3"]
    );
    assert!(family_conditions("x = symbol(\"x\"); solve(sin(x) == 0, x)").is_empty());
}

#[test]
fn the_domain_assume_builtins_answer_the_symbol_they_assumed() {
    for script in [
        "x = symbol(\"x\"); assume_positive(x)",
        "x = symbol(\"x\"); assume_nonnegative(x)",
        "x = symbol(\"x\"); assume_negative(x)",
        "x = symbol(\"x\"); assume_real(x)",
        "x = symbol(\"x\"); assume_integer(x)",
    ] {
        assert_eq!(display(script), "x", "{script}");
    }
}

// ---------------------------------------------------------------------------
// Item 5 — the runner-neutral position rules (SurfacePositionAgreementTests,
// SourcePositionTests.LeadingBom): the engine strips a leading BOM and maps every
// published offset back into the caller's text.
// ---------------------------------------------------------------------------

fn timing_positions(engine: &Engine) -> Vec<usize> {
    engine
        .operation_timings()
        .iter()
        .map(|timing| timing.position)
        .collect()
}

#[test]
fn a_leading_byte_order_mark_is_source_text_not_a_parse_error() {
    let mut engine = Engine::new();
    let error = engine
        .eval("\u{feff}a = 1\nb = 2\ndet(1)")
        .expect_err("det(1) refuses");
    assert_eq!(error.code, "InvalidArgument");
    assert_eq!(error.position, 13);
    assert_eq!(timing_positions(&engine), vec![1, 7, 13]);
}

#[test]
fn without_a_byte_order_mark_the_same_script_reports_offsets_from_zero() {
    let mut engine = Engine::new();
    let error = engine
        .eval("a = 1\nb = 2\ndet(1)")
        .expect_err("det(1) refuses");
    assert_eq!(error.position, 12);
    assert_eq!(timing_positions(&engine), vec![0, 6, 12]);
}

#[test]
fn positions_are_utf16_code_units_on_surrogate_input() {
    let mut engine = Engine::new();
    let error = engine
        .eval("s = \"\u{1F600}\"; det(1)")
        .expect_err("det(1) refuses");
    // the emoji is ONE scalar value but TWO UTF-16 code units; the caller's offset counts both
    assert_eq!(error.position, 10);
    assert_eq!(timing_positions(&engine), vec![0, 10]);
}

// ---------------------------------------------------------------------------
// Item 6 — the evalf digit-cap notice (EvalfDigitCapHonestyTests)
// ---------------------------------------------------------------------------

#[test]
fn a_request_beyond_the_cap_is_marked_as_clamped() {
    let value = structured_json("setprecision(5000); evalf(sqrt(2), 5000)");
    let digits = json_text(&value, "value");
    let places = digits.split('.').nth(1).map(str::len).unwrap_or(0);
    assert_eq!(places, 1000);
    assert!(
        json_flag(&value, "truncated"),
        "the 1000-place clamp crossed unmarked"
    );
    assert_eq!(json_text(&value, "truncationReason"), "digit-cap");
    assert_eq!(json_text(&value, "budget"), "1000");
    assert!(!json_flag(&value, "exact"));
}

#[test]
fn the_smallest_over_cap_request_is_marked_too() {
    let value = structured_json("evalf(1/3, 1001)");
    assert_eq!(
        json_text(&value, "value"),
        format!("0.{}", "3".repeat(1000))
    );
    assert!(json_flag(&value, "truncated"));
    assert_eq!(json_text(&value, "truncationReason"), "digit-cap");
    assert_eq!(json_text(&value, "budget"), "1000");
}

#[test]
fn a_request_at_the_cap_is_not_marked() {
    let value = structured_json("evalf(pi, 1000)");
    assert_eq!(json_field(&value, "truncated"), None);
    assert_eq!(json_field(&value, "truncationReason"), None);
    assert_eq!(json_field(&value, "budget"), None);
}

#[test]
fn an_exact_answer_to_an_over_cap_request_is_not_marked() {
    let value = structured_json("evalf(1/2, 5000)");
    assert_eq!(json_text(&value, "value"), "0.5");
    assert!(json_flag(&value, "exact"));
    assert_eq!(json_field(&value, "truncated"), None);
}

#[test]
fn a_plain_expression_under_the_same_precision_is_not_marked() {
    let value = structured_json("setprecision(5000); sqrt(2)");
    assert_eq!(json_field(&value, "truncated"), None);
}

// ---------------------------------------------------------------------------
// Item 8 — eye/mean/norm two-argument forms (ArityPropertyTests,
// BuiltinArgumentShapeSweepTests)
// ---------------------------------------------------------------------------

#[test]
fn eye_accepts_a_column_count() {
    assert_eq!(display("eye(2,3)"), "[[1, 0, 0], [0, 1, 0]]");
    assert_eq!(display("eye(3,2)"), "[[1, 0], [0, 1], [0, 0]]");
    assert_eq!(display("eye(2)"), "[[1, 0], [0, 1]]");
}

#[test]
fn eye_reads_both_dimensions_through_the_positive_contract() {
    let rows = refusal("eye(0)");
    assert_eq!(
        rows.message,
        "eye(): argument 1 (rows) must be a Natural or Integer that is a positive count (1 or more, at most 9223372036854775807); got 0."
    );
    let cols = refusal("eye(2, -1)");
    assert_eq!(
        cols.message,
        "eye(): argument 2 (cols) must be a Natural or Integer that is a positive count (1 or more, at most 9223372036854775807); got -1."
    );
    let arity = refusal("eye(2,3,4)");
    assert_eq!(arity.message, "eye(): expected 1 to 2 arguments; got 3.");
}

#[test]
fn mean_and_norm_accept_an_axis_and_publish_the_reduced_shape() {
    assert_eq!(display("mean([1,2,3])"), "2");
    assert_eq!(display("mean([1,2,3], 0)"), "2");
    assert_eq!(display("norm([3,4])"), "5");
    assert_eq!(display("mean([[1,2],[3,4]], 1)"), "[1.5, 3.5]");
    assert_eq!(display("mean([[1,2],[3,4]], 0)"), "[2, 3]");
    assert_eq!(display("norm([[3,4],[0,0]], 1)"), "[5, 0]");
    assert_eq!(display("norm([[3,4],[0,0]], 0)"), "[3, 4]");
}

#[test]
fn mean_and_norm_refuse_a_non_array_input_with_the_shape_grammar() {
    for name in ["mean", "norm"] {
        let error = refusal(&format!("{name}(1)"));
        assert_eq!(
            error.message,
            format!("{name}(): argument 1 must be an array or vector; got Natural.")
        );
        assert_eq!(
            (error.code, error.category),
            ("InvalidArgument", "TypeMismatch")
        );
        let signature = error
            .signature
            .as_ref()
            .expect("a shape refusal carries its signature");
        assert_eq!(signature.position, Some(1));
        assert_eq!(signature.parameter.as_deref(), Some("a"));
        assert_eq!(
            signature.expected_shape.as_deref(),
            Some("an array or vector")
        );
        assert_eq!(signature.arrived_kind.as_deref(), Some("Natural"));
    }
}

#[test]
fn mean_and_norm_refuse_an_axis_the_value_does_not_have() {
    let one = refusal("mean([1,2,3], 1)");
    assert_eq!(
        one.message,
        "mean(): argument 2 (axis) must be the only axis (0); got 1."
    );
    let negative = refusal("norm([3,4], -1)");
    assert_eq!(
        negative.message,
        "norm(): argument 2 (axis) must be the only axis (0); got -1."
    );
    let rank_two = refusal("norm([[3,4],[0,0]], 2)");
    assert_eq!(
        rank_two.message,
        "norm(): argument 2 (axis) must be one of the value's 2 axes, 0 to 1; got 2."
    );
}

#[test]
fn mean_and_norm_keep_their_arity_and_empty_refusals() {
    let too_many = refusal("mean([1,2,3], 0, 1)");
    assert_eq!(
        too_many.message,
        "mean(): expected 1 to 2 arguments; got 3."
    );
    let empty = refusal("mean([])");
    assert_eq!(empty.message, "mean() cannot reduce an empty array.");
    assert_eq!(empty.code, "InvalidOperation");
}
