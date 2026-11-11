//! Round 34: the individually adjudicated Symbolics tail.
//!
//! Every test here is one deferred census row from Lovelace.Symbolics.Tests. The C# assertion is
//! the contract: the script is the C# row's own input (or the language-surface form of the same
//! kernel call) and the expected value is what the C# assertion demands. A row whose Rust
//! behaviour differs is NOT here; it is refused in docs/goal-parity-contract/round-34/symbolics-tail.md
//! with the measured divergence.
//!
//! Two shapes appear:
//!   * language-surface rows: the C# row drives SuiteEngine with a script, so the test drives the
//!     same script through Engine and asserts the same record fields;
//!   * kernel rows whose kernel operation is reachable through a builtin (limit, simplify, solve,
//!     print, ...): the test asserts the same semantic case through that builtin.

use crate::{Engine, LangError, Value};

fn eval(script: &str) -> Value {
    let mut engine = Engine::new();
    engine
        .eval(script)
        .unwrap_or_else(|error| panic!("script: {script} failed: {}", error.message))
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

fn boolean(value: &Value) -> bool {
    match value {
        Value::Boolean(truth) => *truth,
        other => panic!("expected Boolean, got {}", other.kind_name()),
    }
}

fn variant(value: &Value) -> &str {
    match value {
        Value::Enum(_, name) => name,
        other => panic!("expected Enum, got {}", other.kind_name()),
    }
}

fn vector(value: &Value) -> &[Value] {
    match value {
        Value::Vector(items) => items,
        _ => panic!("expected Vector, got {}", value.kind_name()),
    }
}

fn symbolic(value: &Value) -> &lovelace_symbolics::Expr {
    match value {
        Value::Symbolic(expression) => expression,
        other => panic!("expected Symbolic, got {}", other.kind_name()),
    }
}

fn display(value: &Value) -> String {
    value.display()
}

fn typed(value: &Value) -> String {
    value.typed()
}

fn symbolic_text(script: &str) -> String {
    symbolic(&eval(script)).to_string()
}

// ---------------------------------------------------------------------------
// DxStructuredResultsTests (8 rows)
// ---------------------------------------------------------------------------

/// DxStructuredResultsTests.cs:46 — compile_full publishes kernel metadata and evalir_batch
/// accepts the record.
#[test]
fn dx_structured_compile_full_exposes_kernel_metadata_and_batch() {
    let mut engine = Engine::new();
    eval_in(&mut engine, "x = symbol(\"x\")");
    let compilation = eval_in(&mut engine, "compile_full(x^2 + 1, [x])");
    let fields = record_fields(&compilation, "CompilationResult");

    let parameters = vector(field(fields, "parameters"));
    let first = record_fields(&parameters[0], "ParameterInfo");
    assert_eq!(text(field(first, "name")), "x");
    assert!(matches!(field(first, "domain"), Value::Domain(_)));
    assert!(matches!(field(fields, "result_domain"), Value::Domain(_)));
    assert_eq!(text(field(fields, "precision_policy")), "caller_supplied");
    assert_eq!(text(field(fields, "optimization_policy")), "none");
    assert_eq!(display(field(fields, "mathir_version")), "2");

    let batch = eval_in(
        &mut engine,
        "evalir_batch(compile_full(x^2 + 1, [x]), [1, 2, 3], 40)",
    );
    assert_eq!(typed(&batch), "[2, 5, 10] (Vector)");
}

/// DxStructuredResultsTests.cs:78 — domains are first-class values with one vocabulary.
#[test]
fn dx_structured_domain_values_are_first_class() {
    let mut engine = Engine::new();
    assert_eq!(typed(&engine.eval("type(complex)").unwrap()), "Domain");
    let domain = engine.eval("inspect(complex).domain").unwrap();
    assert!(matches!(domain, Value::Domain(_)));
    assert_eq!(display(&domain), "complex");
    assert_eq!(
        display(&engine.eval("inspect(real).domain").unwrap()),
        "real"
    );
}

/// DxStructuredResultsTests.cs:113 — limit_full exposes existence and both one-sided values;
/// the short form is the same record.
#[test]
fn dx_structured_limit_full_exposes_existence_and_one_sided_values() {
    let mut engine = Engine::new();
    eval_in(&mut engine, "x = symbol(\"x\")");
    let record = eval_in(&mut engine, "limit_full(1/x, x, 0)");
    let fields = record_fields(&record, "LimitResult");

    assert!(!boolean(field(fields, "exists")));
    assert_eq!(display(field(fields, "left")), "-inf");
    assert_eq!(display(field(fields, "right")), "inf");
    assert_eq!(typed(field(fields, "left_conditions")), "[x < 0] (Vector)");
    assert_eq!(typed(field(fields, "right_conditions")), "[x > 0] (Vector)");
    assert_eq!(variant(field(fields, "exactness")), "Exact");

    let short = eval_in(&mut engine, "limit(1/x, x, 0)");
    let short_fields = record_fields(&short, "LimitResult");
    assert!(!boolean(field(short_fields, "exists")));
    assert_eq!(display(field(short_fields, "left")), "-inf");
    assert_eq!(display(field(short_fields, "right")), "inf");
    assert_eq!(
        typed(field(short_fields, "left_conditions")),
        typed(field(fields, "left_conditions"))
    );
    assert_eq!(
        typed(field(short_fields, "right_conditions")),
        typed(field(fields, "right_conditions"))
    );
}

/// DxStructuredResultsTests.cs:152 — simplify_full carries conditions, steps and property access.
#[test]
fn dx_structured_simplify_full_carries_conditions_and_steps() {
    let mut engine = Engine::new();
    eval_in(&mut engine, "x = symbol(\"x\")");
    let record = eval_in(&mut engine, "simplify_full(x/x)");
    let fields = record_fields(&record, "TransformResult");

    assert_eq!(variant(field(fields, "status")), "Satisfied");
    assert_eq!(display(field(fields, "original")), "x/x");
    assert_eq!(display(field(fields, "expression")), "1");
    assert!(boolean(field(fields, "changed")));
    assert_eq!(typed(field(fields, "conditions")), "[x != 0] (Vector)");

    let steps = vector(field(fields, "steps"));
    let first = record_fields(&steps[0], "RewriteStep");
    assert_eq!(text(field(first, "rule_id")), "rat.cancel-x-over-x");
    assert_eq!(
        text(&eval_in(&mut engine, "simplify_full(x/x).steps[0].rule_id")),
        "rat.cancel-x-over-x"
    );
}

/// DxStructuredResultsTests.cs:188 — simplify_full(exp(log(x))) carries the finite condition.
#[test]
fn dx_structured_simplify_full_exp_log_carries_finite_condition() {
    let mut engine = Engine::new();
    eval_in(&mut engine, "x = symbol(\"x\")");
    let record = eval_in(&mut engine, "simplify_full(exp(log(x)))");
    let fields = record_fields(&record, "TransformResult");

    assert_eq!(display(field(fields, "expression")), "x");
    let conditions = vector(field(fields, "conditions"));
    let leaf = record_fields(&conditions[0], "FiniteCondition");
    assert_eq!(display(field(leaf, "expression")), "log(x)");
    assert_eq!(
        typed(field(fields, "conditions")),
        "[FiniteCondition(expression: log(x))] (Vector)"
    );
}

/// DxStructuredResultsTests.cs:213 — solve_full publishes a Domain value and the real option.
#[test]
fn dx_structured_solve_full_domain_field_and_real_option() {
    let mut engine = Engine::new();
    eval_in(&mut engine, "x = symbol(\"x\")");
    let real = eval_in(&mut engine, "solve_full(x^2 + 1 == 0, x, real)");
    let fields = record_fields(&real, "SolveResult");
    assert_eq!(variant(field(fields, "status")), "NoSolutions");
    assert!(matches!(field(fields, "domain"), Value::Domain(_)));

    let complex_domain = eval_in(&mut engine, "solve_full(x^2 + 1 == 0, x).domain");
    assert!(matches!(complex_domain, Value::Domain(_)));
    assert_eq!(display(&complex_domain), "complex");

    assert_eq!(
        variant(&eval_in(
            &mut engine,
            "solve_full(x^4 - x^2 - 1 == 0, x).status"
        )),
        "Partial"
    );
    assert!(!boolean(&eval_in(
        &mut engine,
        "solve_full(x^4 - x^2 - 1 == 0, x).complete"
    )));
    assert_eq!(
        display(&eval_in(
            &mut engine,
            "solve_full(x^4 - x^2 - 1 == 0, x).unrepresented_count"
        )),
        "2"
    );
}

/// DxStructuredResultsTests.cs:238 — solve_full exposes status, solutions and conditions.
#[test]
fn dx_structured_solve_full_exposes_status_solutions_and_conditions() {
    let mut engine = Engine::new();
    eval_in(&mut engine, "x = symbol(\"x\")");
    let record = eval_in(&mut engine, "solve_full((x^2 - 1)/(x - 1) == 0, x)");
    let fields = record_fields(&record, "SolveResult");
    assert_eq!(variant(field(fields, "status")), "Solved");

    let solutions = vector(field(fields, "solutions"));
    let solution = record_fields(&solutions[0], "Solution");
    assert_eq!(display(field(solution, "value")), "-1");
    assert_eq!(display(field(solution, "conditions")), "[x - 1 != 0]");
    assert_eq!(display(field(solution, "multiplicity")), "1");
    assert_eq!(variant(field(solution, "exactness")), "Exact");

    assert_eq!(
        variant(&eval_in(&mut engine, "solve_full(x^2 - 4 == 0, x).status")),
        "Solved"
    );
    assert!(boolean(&eval_in(
        &mut engine,
        "solve_full(x^2 - 4 == 0, x).complete"
    )));
    assert_eq!(
        display(&eval_in(
            &mut engine,
            "solve_full((x^2 - 1)/(x - 1) == 0, x).common_conditions"
        )),
        "[x - 1 != 0]"
    );
}

/// DxStructuredResultsTests.cs:283 — one type vocabulary for every kind (row rational()).
#[test]
fn dx_structured_type_uses_one_vocabulary_for_every_kind() {
    assert_eq!(text(&eval("type(rational())")), "Domain");
}

// ---------------------------------------------------------------------------
// DxSemanticClosureTests (2 rows)
// ---------------------------------------------------------------------------

/// DxSemanticClosureTests.cs:64 — multiplicity survives the factor -> root projection and a pole
/// excluded by every candidate is NoSolutions.
#[test]
fn dx_semantic_solve_multiplicity_and_no_solutions_are_honest() {
    let mut engine = Engine::new();
    eval_in(&mut engine, "x = symbol(\"x\")");
    let repeated = eval_in(&mut engine, "solve_full(x^2 - 2*x + 1 == 0, x)");
    let fields = record_fields(&repeated, "SolveResult");
    let solutions = vector(field(fields, "solutions"));
    let solution = record_fields(&solutions[0], "Solution");
    assert_eq!(display(field(solution, "multiplicity")), "2");
    assert_eq!(
        variant(&eval_in(
            &mut engine,
            "solve_full((x^2-1)/(x^2-1) == 0, x).status"
        )),
        "NoSolutions"
    );
}

/// DxSemanticClosureTests.cs:84 — mixed roots are Partial, never a complete complex set.
#[test]
fn dx_semantic_solve_quartic_mixed_roots_is_partial_not_complete() {
    let mut engine = Engine::new();
    eval_in(&mut engine, "x = symbol(\"x\")");
    assert_eq!(
        variant(&eval_in(
            &mut engine,
            "solve_full(x^4 - x^2 - 1 == 0, x).status"
        )),
        "Partial"
    );
    assert!(!boolean(&eval_in(
        &mut engine,
        "solve_full(x^4 - x^2 - 1 == 0, x).complete"
    )));
    assert_eq!(
        display(&eval_in(
            &mut engine,
            "solve_full(x^4 - x^2 - 1 == 0, x).unrepresented_count"
        )),
        "2"
    );
    let short = eval_in(&mut engine, "solve(x^4 - x^2 - 1 == 0, x)");
    let fields = record_fields(&short, "SolveResult");
    assert_eq!(variant(field(fields, "status")), "Partial");
    assert_eq!(display(field(fields, "represented_count")), "2");
    let real = eval_in(&mut engine, "solve(x^4 - x^2 - 1 == 0, x, real)");
    let real_fields = record_fields(&real, "SolveResult");
    assert_eq!(variant(field(real_fields, "status")), "Solved");
    assert_eq!(variant(field(real_fields, "completeness")), "Complete");
}

/// DxSemanticClosureTests.cs:202 — Transform(exp(log(x))) carries the finite-log condition.
#[test]
fn dx_semantic_transform_exp_log_carries_finite_log_condition() {
    let mut engine = Engine::new();
    eval_in(&mut engine, "x = symbol(\"x\")");
    let record = eval_in(&mut engine, "simplify_full(exp(log(x)))");
    let fields = record_fields(&record, "TransformResult");
    assert_eq!(display(field(fields, "expression")), "x");
    let steps = vector(field(fields, "steps"));
    let step = record_fields(&steps[0], "RewriteStep");
    assert_eq!(text(field(step, "rule_id")), "logexp.exp-log");
    assert_eq!(variant(field(step, "classification")), "Conditional");
    let conditions = vector(field(step, "required_conditions"));
    let condition = record_fields(&conditions[0], "FiniteCondition");
    assert_eq!(display(field(condition, "expression")), "log(x)");
}

// ---------------------------------------------------------------------------
// KernelHardeningTests (14 rows)
// ---------------------------------------------------------------------------

/// KernelHardeningTests.cs:166 — |x|^2 is not folded without a real assumption.
#[test]
fn kernel_hardening_abs_square_not_simplified_for_complex() {
    assert_eq!(
        symbolic_text("x = symbol(\"x\"); simplify(abs(x)^2)"),
        "abs(x)^2"
    );
}

/// KernelHardeningTests.cs:186 — dropping 0/x would define the expression at 0.
#[test]
fn kernel_hardening_add_dropping_zero_coefficient_over_pole_keeps_definedness() {
    let mut engine = Engine::new();
    eval_in(&mut engine, "x = symbol(\"x\")");
    eval_in(&mut engine, "print(x + 0/x)");
    let out = engine.take_output();
    assert_eq!(out, vec!["x + 0/x".to_string()]);
    assert!(out[0].contains("0/x"));
}

/// KernelHardeningTests.cs:205 — exp(log(x)) is kept by the safe API and rewrites only when the
/// definedness condition is carried by the full transform.
#[test]
fn kernel_hardening_exp_log_safe_simplify_requires_definedness_conditions() {
    let mut engine = Engine::new();
    eval_in(&mut engine, "x = symbol(\"x\")");
    assert_eq!(
        symbolic_text("x = symbol(\"x\"); simplify(exp(log(x)))"),
        "exp(log(x))"
    );
    let record = eval_in(&mut engine, "simplify_full(exp(log(x)))");
    let fields = record_fields(&record, "TransformResult");
    assert_eq!(display(field(fields, "expression")), "x");
    let steps = vector(field(fields, "steps"));
    let step = record_fields(&steps[0], "RewriteStep");
    assert_eq!(text(field(step, "rule_id")), "logexp.exp-log");
    assert_eq!(variant(field(step, "classification")), "Conditional");
}

/// KernelHardeningTests.cs:231 — one-sided pole statuses.
#[test]
fn kernel_hardening_limit_one_over_x_left_minus_inf() {
    let record = eval("x = symbol(\"x\"); limit_left(1/x, x, 0)");
    assert_eq!(
        variant(field(record_fields(&record, "LimitResult"), "status")),
        "MinusInfinity"
    );
}

/// KernelHardeningTests.cs:241 — one-sided pole statuses.
#[test]
fn kernel_hardening_limit_one_over_x_right_plus_inf() {
    let record = eval("x = symbol(\"x\"); limit_right(1/x, x, 0)");
    assert_eq!(
        variant(field(record_fields(&record, "LimitResult"), "status")),
        "PlusInfinity"
    );
}

/// KernelHardeningTests.cs:251 — the two-sided limit does not exist, but both sides do.
#[test]
fn kernel_hardening_limit_one_over_x_two_sided_does_not_exist() {
    let record = eval("x = symbol(\"x\"); limit(1/x, x, 0)");
    assert_eq!(
        variant(field(record_fields(&record, "LimitResult"), "status")),
        "DoesNotExist"
    );
    let left = eval("x = symbol(\"x\"); limit_left(1/x, x, 0)");
    let right = eval("x = symbol(\"x\"); limit_right(1/x, x, 0)");
    assert_eq!(
        variant(field(record_fields(&left, "LimitResult"), "status")),
        "MinusInfinity"
    );
    assert_eq!(
        variant(field(record_fields(&right, "LimitResult"), "status")),
        "PlusInfinity"
    );
}

/// KernelHardeningTests.cs:267 — 1/x^2 is +infinity from both sides.
#[test]
fn kernel_hardening_limit_one_over_x_squared_both_sides_plus_inf() {
    let left = eval("x = symbol(\"x\"); limit_left(1/x^2, x, 0)");
    let right = eval("x = symbol(\"x\"); limit_right(1/x^2, x, 0)");
    assert_eq!(
        variant(field(record_fields(&left, "LimitResult"), "status")),
        "PlusInfinity"
    );
    assert_eq!(
        variant(field(record_fields(&right, "LimitResult"), "status")),
        "PlusInfinity"
    );
}

/// KernelHardeningTests.cs:281 — the series offset (1 - cos x)/x tends to 0.
#[test]
fn kernel_hardening_limit_series_offset_one_minus_cos_over_x_is_zero() {
    let record = eval("x = symbol(\"x\"); limit((1 - cos(x))/x, x, 0)");
    let fields = record_fields(&record, "LimitResult");
    assert_eq!(variant(field(fields, "status")), "Value");
    assert_eq!(display(field(fields, "value")), "0");
}

/// KernelHardeningTests.cs:302 — log(exp(x)) is not simplified without x real.
#[test]
fn kernel_hardening_log_exp_not_simplified_unconstrained() {
    assert_eq!(
        symbolic_text("x = symbol(\"x\"); simplify(log(exp(x)))"),
        "log(exp(x))"
    );
}

/// KernelHardeningTests.cs:317 — (-1)^(2^40) parity is exact.
#[test]
fn kernel_hardening_minus_one_huge_exponent_parity_is_exact() {
    assert_eq!(display(&eval("(-1)^(2^40)")), "1");
    assert_eq!(display(&eval("(-1)^(2^40 + 1)")), "-1");
}

/// KernelHardeningTests.cs:414 — x/x cancels with the non-zero condition and a conditional step.
#[test]
fn kernel_hardening_simplify_x_over_x_cancels_with_condition() {
    let record = eval("x = symbol(\"x\"); simplify_full(x/x)");
    let fields = record_fields(&record, "TransformResult");
    assert_eq!(display(field(fields, "expression")), "1");
    assert_eq!(typed(field(fields, "conditions")), "[x != 0] (Vector)");
    let steps = vector(field(fields, "steps"));
    assert!(!steps.is_empty());
    for step in steps {
        assert_eq!(
            variant(field(record_fields(step, "RewriteStep"), "classification")),
            "Conditional"
        );
    }
}

/// KernelHardeningTests.cs:456 — the pole of the cancelled denominator is not a solution.
#[test]
fn kernel_hardening_solve_rational_equation_honors_excluded_denominator() {
    let record = eval("x = symbol(\"x\"); solve_full((x^2 - 1)/(x - 1) == 0, x)");
    let fields = record_fields(&record, "SolveResult");
    let solutions = vector(field(fields, "solutions"));
    let values: Vec<String> = solutions
        .iter()
        .map(|solution| display(field(record_fields(solution, "Solution"), "value")))
        .collect();
    assert_eq!(values, vec!["-1".to_string()]);
    assert_eq!(display(field(fields, "represented_count")), "1");
}

/// KernelHardeningTests.cs:487 — construction does not fold x/x or 0/x.
#[test]
fn kernel_hardening_x_over_x_is_not_folded_by_construction() {
    let mut engine = Engine::new();
    eval_in(&mut engine, "x = symbol(\"x\")");
    eval_in(&mut engine, "print(x/x)");
    eval_in(&mut engine, "print(0/x)");
    assert_eq!(
        engine.take_output(),
        vec!["x/x".to_string(), "0/x".to_string()]
    );
}

/// KernelHardeningTests.cs:517 — 0^0 is 1 by the documented convention.
#[test]
fn kernel_hardening_zero_pow_zero_is_one() {
    assert_eq!(display(&eval("0^0")), "1");
}

// ---------------------------------------------------------------------------
// EvalirDigitArgumentContractTests (13 rows)
// ---------------------------------------------------------------------------

const KERNEL_CALL: &str = "compile(x*1/3, [x])";

fn evalir_script(builtin: &str, digits: &str) -> String {
    if builtin == "evalir" {
        format!("x = symbol(\"x\"); evalir({KERNEL_CALL}, [1], {digits})")
    } else {
        format!("x = symbol(\"x\"); evalir_batch({KERNEL_CALL}, [1, 2], {digits})")
    }
}

fn assert_out_of_bounds(builtin: &str, digits: &str) {
    let error = refusal(&evalir_script(builtin, digits));
    assert_eq!(error.code, "InvalidArgument");
    assert_eq!(error.category, "TypeMismatch");
    assert_eq!(
        error.message,
        format!(
            "{builtin}(): argument 3 (digits) must be a Natural or Integer between 1 and 2147483647; got {digits}."
        )
    );
}

/// EvalirDigitArgumentContractTests.cs:97 — a count outside [1, i32::MAX] is refused and named.
#[test]
fn evalir_digit_count_outside_bound_evalir_zero() {
    assert_out_of_bounds("evalir", "0");
}

/// EvalirDigitArgumentContractTests.cs:97 (case 1).
#[test]
fn evalir_digit_count_outside_bound_evalir_batch_zero() {
    assert_out_of_bounds("evalir_batch", "0");
}

/// EvalirDigitArgumentContractTests.cs:97 (case 2).
#[test]
fn evalir_digit_count_outside_bound_evalir_minus_five() {
    assert_out_of_bounds("evalir", "-5");
}

/// EvalirDigitArgumentContractTests.cs:97 (case 3).
#[test]
fn evalir_digit_count_outside_bound_evalir_batch_minus_five() {
    assert_out_of_bounds("evalir_batch", "-5");
}

fn assert_too_wide(builtin: &str) {
    let error = refusal(&evalir_script(builtin, "10^30"));
    assert_eq!(error.code, "InvalidArgument");
    assert_eq!(error.category, "TypeMismatch");
    let received = "1000000000000000000000000000000";
    assert!(error.message.contains(received), "{}", error.message);
    assert!(
        !error.message.contains("too large or too small"),
        "{}",
        error.message
    );
    assert_eq!(
        error.message,
        format!(
            "{builtin}(): argument 3 (digits) must be a Natural or Integer between 1 and 2147483647; got {received}."
        )
    );
}

/// EvalirDigitArgumentContractTests.cs:118 — a count wider than Int64 is refused and named.
#[test]
fn evalir_digit_count_wider_than_int64_evalir() {
    assert_too_wide("evalir");
}

/// EvalirDigitArgumentContractTests.cs:118 (case 1).
#[test]
fn evalir_digit_count_wider_than_int64_evalir_batch() {
    assert_too_wide("evalir_batch");
}

fn assert_not_an_integer(digits: &str) {
    let error = refusal(&evalir_script("evalir", digits));
    assert_eq!(error.code, "InvalidArgument");
    assert_eq!(error.category, "TypeMismatch");
    assert!(
        error
            .message
            .contains("must be a Natural or Integer that denotes an exact integer")
    );
    assert!(error.message.contains(digits), "{}", error.message);
    assert!(
        !error.message.contains("between 1 and 2147483647"),
        "{}",
        error.message
    );
}

/// EvalirDigitArgumentContractTests.cs:140 — a non-integral count is not the integral-kind case.
#[test]
fn evalir_non_integral_count_two_point_five() {
    assert_not_an_integer("2.5");
}

/// EvalirDigitArgumentContractTests.cs:140 (case 1).
#[test]
fn evalir_non_integral_count_two_point_nine() {
    assert_not_an_integer("2.9");
}

/// EvalirDigitArgumentContractTests.cs:140 (case 2).
#[test]
fn evalir_non_integral_count_zero_point_five() {
    assert_not_an_integer("0.5");
}

/// EvalirDigitArgumentContractTests.cs:140 (case 3).
#[test]
fn evalir_non_integral_count_one_point_zero_zero_zero_zero_zero_zero_one() {
    assert_not_an_integer("1.0000001");
}

/// EvalirDigitArgumentContractTests.cs:158 — a non-integral digits argument names builtin and value.
#[test]
fn evalir_non_integral_digit_count_evalir() {
    let error = refusal(&evalir_script("evalir", "2.9"));
    assert_eq!(error.code, "InvalidArgument");
    assert_eq!(error.category, "TypeMismatch");
    assert!(error.message.contains("2.9"), "{}", error.message);
    assert!(error.message.starts_with("evalir"), "{}", error.message);
    assert!(error.message.contains("(digits)"), "{}", error.message);
}

/// EvalirDigitArgumentContractTests.cs:158 (case 1).
#[test]
fn evalir_non_integral_digit_count_evalir_batch() {
    let error = refusal(&evalir_script("evalir_batch", "2.9"));
    assert_eq!(error.code, "InvalidArgument");
    assert_eq!(error.category, "TypeMismatch");
    assert!(error.message.contains("2.9"), "{}", error.message);
    assert!(
        error.message.starts_with("evalir_batch"),
        "{}",
        error.message
    );
    assert!(error.message.contains("(digits)"), "{}", error.message);
}

/// EvalirDigitArgumentContractTests.cs:182 — a Text digits argument is the same argument error.
#[test]
fn evalir_text_digit_count_is_refused_as_the_same_argument_error() {
    let error = refusal("x = symbol(\"x\"); evalir(compile(x*1/3, [x]), [1], \"forty\")");
    assert_eq!(error.code, "InvalidArgument");
    assert_eq!(error.category, "TypeMismatch");
    assert_eq!(
        error.message,
        "evalir(): argument 3 (digits) must be a Natural or Integer that denotes an exact integer; got Text."
    );
}

// ---------------------------------------------------------------------------
// IndeterminateExponentialLimitTests (1 row)
// ---------------------------------------------------------------------------

/// IndeterminateExponentialLimitTests.cs:229 — undetermined limits claim no exactness; determined
/// ones keep theirs, and a pole derives its exactness from the two sides.
#[test]
fn indeterminate_undetermined_limit_claims_no_exactness() {
    let mut engine = Engine::new();
    eval_in(&mut engine, "x = symbol(\"x\")");
    let undetermined = eval_in(&mut engine, "limit_full(x*sin(1/x), x, 0)");
    let fields = record_fields(&undetermined, "LimitResult");
    assert_eq!(variant(field(fields, "status")), "Unevaluated");
    assert!(matches!(field(fields, "exists"), Value::Void));
    assert!(matches!(field(fields, "value"), Value::Void));
    assert_eq!(variant(field(fields, "exactness")), "None");

    let determined = eval_in(&mut engine, "limit_full(sin(x)/x, x, 0)");
    assert_eq!(
        variant(field(
            record_fields(&determined, "LimitResult"),
            "exactness"
        )),
        "Exact"
    );
    let pole = eval_in(&mut engine, "limit_full(1/x, x, 0)");
    let pole_fields = record_fields(&pole, "LimitResult");
    assert_eq!(variant(field(pole_fields, "status")), "DoesNotExist");
    assert_eq!(variant(field(pole_fields, "exactness")), "Exact");
}

// ---------------------------------------------------------------------------
// SeriesKinkDeterminismTests (5 rows)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// SeriesKinkDeterminismTests (3 rows)
// ---------------------------------------------------------------------------

/// SeriesKinkDeterminismTests.cs:206 — the polynomial part takes the SymPy value at the sample.
#[test]
fn series_kink_sin_takes_the_value_at_the_sample_point() {
    let display = symbolic(&eval("x = symbol(\"x\"); series(sin(x), x, 0, 5)")).to_string();
    let polynomial = display.split(" + O(").next().unwrap();
    let value = symbolic(&eval(&format!(
        "x = symbol(\"x\"); subs({polynomial}, x, 1/2)"
    )))
    .to_string();
    assert_eq!(value, "23/48");
}

/// SeriesKinkDeterminismTests.cs:263 — two-sided limits at a smooth point are unchanged.
#[test]
fn series_kink_limit_engine_one_minus_cos_over_x_squared() {
    let record = eval("x = symbol(\"x\"); limit((1 - cos(x))/x^2, x, 0)");
    let limit_fields = record_fields(&record, "LimitResult");
    assert_eq!(variant(field(limit_fields, "status")), "Value");
    assert_eq!(display(field(limit_fields, "value")), "1/2");
}

/// SeriesKinkDeterminismTests.cs:263 (case 3) — a kink limit is refused, never answered with the
/// value at the point.
#[test]
fn series_kink_limit_engine_abs_over_x_is_unevaluated() {
    let record = eval("x = symbol(\"x\"); limit(abs(x)/x, x, 0)");
    let limit_fields = record_fields(&record, "LimitResult");
    assert_eq!(variant(field(limit_fields, "status")), "Unevaluated");
    assert!(matches!(field(limit_fields, "value"), Value::Void));
    let diagnostics = vector(field(limit_fields, "diagnostics"));
    let diagnostic = record_fields(&diagnostics[0], "Diagnostic");
    assert_eq!(text(field(diagnostic, "code")), "limit.unevaluated");
}

// ---------------------------------------------------------------------------
// AcceptanceTests (4 rows)
// ---------------------------------------------------------------------------

/// AcceptanceTests.cs:31 — product and chain rule, compared by exact expansion.
#[test]
fn acceptance_diff_product_and_chain_work() {
    assert_eq!(
        symbolic_text("x = symbol(\"x\"); diff(exp(-x^2)*sin(x), x)"),
        "-2*x*exp(-x^2)*sin(x) + cos(x)*exp(-x^2)"
    );
}

/// AcceptanceTests.cs:141 — sin(x)/x at zero is 1.
#[test]
fn acceptance_limit_sin_over_x_at_zero_is_one() {
    let record = eval("x = symbol(\"x\"); limit(sin(x)/x, x, 0)");
    let fields = record_fields(&record, "LimitResult");
    assert_eq!(variant(field(fields, "status")), "Value");
    assert_eq!(display(field(fields, "value")), "1");
}

/// AcceptanceTests.cs:197 — optimize -> lower -> evaluate round trip.
#[test]
fn acceptance_optimize_lower_evaluate_round_trip() {
    let value = eval(
        "x = symbol(\"x\"); k = optimize(x^5 + 2*x^4 + 3*x^3 + x^2, [x]); lower(k, [x]) \
         ; ir = lower(k, [x]); evalir(ir, [2], 40)",
    );
    assert_eq!(typed(&value), "92 (Integer)");
}

/// AcceptanceTests.cs:230 — sin^2 + cos^2 collapses to one.
#[test]
fn acceptance_simplify_pythagorean_collapses_to_one() {
    assert_eq!(
        symbolic_text("x = symbol(\"x\"); simplify(sin(x)^2 + cos(x)^2)"),
        "1"
    );
}

// ---------------------------------------------------------------------------
// SystemSolveTests (1 row)
// ---------------------------------------------------------------------------

/// SystemSolveTests.cs:32 — a linear system has the single solution (1/2, 1/2).
#[test]
fn system_solve_linear_system_single_solution() {
    let mut engine = Engine::new();
    eval_in(&mut engine, "x = symbol(\"x\"); y = symbol(\"y\")");
    let record = eval_in(
        &mut engine,
        "solve_system_full([x + y == 1, x - y == 0], [x, y])",
    );
    let fields = record_fields(&record, "SystemSolveResult");
    let solutions = vector(field(fields, "solutions"));
    assert_eq!(solutions.len(), 1);
    let bindings = vector(field(
        record_fields(&solutions[0], "SystemSolution"),
        "bindings",
    ));
    assert_eq!(
        display(field(record_fields(&bindings[0], "Binding"), "value")),
        "1/2"
    );
    assert_eq!(
        display(field(record_fields(&bindings[1], "Binding"), "value")),
        "1/2"
    );
}

// ---------------------------------------------------------------------------
// UnsatisfiablePropagationTests (1 row)
// ---------------------------------------------------------------------------

/// UnsatisfiablePropagationTests.cs:21 — the condition payload is structured for every reachable
/// atom kind.
#[test]
fn unsatisfiable_propagation_condition_payload_is_structured() {
    let mut engine = Engine::new();
    eval_in(&mut engine, "x = symbol(\"x\")");
    eval_in(&mut engine, "assume_positive(x)");
    let relation = eval_in(&mut engine, "simplify_full(x/x)");
    let relation_conditions = vector(field(
        record_fields(&relation, "TransformResult"),
        "conditions",
    ));
    assert!(matches!(&relation_conditions[0], Value::Symbolic(_)));

    let finite = eval_in(&mut engine, "simplify_full(exp(log(x)))");
    let finite_conditions = vector(field(
        record_fields(&finite, "TransformResult"),
        "conditions",
    ));
    let leaf = record_fields(&finite_conditions[0], "FiniteCondition");
    assert_eq!(display(field(leaf, "expression")), "log(x)");
    assert!(!typed(&finite_conditions[0]).contains("Assumption"));
}

// ---------------------------------------------------------------------------
// CapabilitiesBuiltinTests (2 rows)
// ---------------------------------------------------------------------------

/// CapabilitiesBuiltinTests.cs:70 — the exactness enum says exhaustive or best effort.
#[test]
fn capabilities_exactness_says_exhaustive_or_best_effort() {
    let value = eval("capabilities().exactness");
    match &value {
        Value::Enum(type_name, name) => {
            assert_eq!(*type_name, "CapabilitiesExactness");
            assert!(["Exhaustive", "BestEffort"].contains(&name.as_str()));
        }
        other => panic!("expected Enum, got {}", other.kind_name()),
    }
}

/// CapabilitiesBuiltinTests.cs:150 — unsupported operations are records carrying code, category,
/// message and trigger; the category is a declared ErrorCategory and scope is Text or Void.
#[test]
fn capabilities_unsupported_operations_are_records() {
    let operations_value = eval("capabilities().unsupported_operations");
    let operations = vector(&operations_value);
    assert!(!operations.is_empty());
    let declared = [
        "BudgetExceeded",
        "DomainError",
        "InternalInvariantFailure",
        "NoSolution",
        "ParseError",
        "TypeMismatch",
        "UnsupportedOperation",
    ];
    for element in operations {
        let fields = record_fields(element, "UnsupportedCapability");
        for name in ["operation_class", "code", "message", "trigger"] {
            assert!(!text(field(fields, name)).is_empty(), "{name} is empty");
        }
        match field(fields, "category") {
            Value::Enum(type_name, name) => {
                assert_eq!(*type_name, "ErrorCategory");
                assert!(declared.contains(&name.as_str()), "unknown category {name}");
            }
            other => panic!("category is not an Enum: {}", other.kind_name()),
        }
        assert!(matches!(
            field(fields, "scope"),
            Value::Text(_) | Value::Void
        ));
    }
}

// ---------------------------------------------------------------------------
// RewriteProtocolTests (2 rows)
// ---------------------------------------------------------------------------

/// RewriteProtocolTests.cs:52 — a conditional rule carries its declared conditions.
#[test]
fn rewrite_protocol_conditional_rule_carries_declared_conditions() {
    let record = eval("x = symbol(\"x\"); simplify_full(x/x)");
    let fields = record_fields(&record, "TransformResult");
    assert_eq!(display(field(fields, "expression")), "1");
    assert!(!vector(field(fields, "conditions")).is_empty());
    let steps = vector(field(fields, "steps"));
    assert_eq!(steps.len(), 1);
    let step = record_fields(&steps[0], "RewriteStep");
    assert!(!vector(field(step, "required_conditions")).is_empty());
    assert_eq!(text(field(step, "rule_id")), "rat.cancel-x-over-x");
}

/// RewriteProtocolTests.cs:80 — the rule does not fire on unknown, but fires when proven.
#[test]
fn rewrite_protocol_conditional_rule_does_not_fire_on_unknown() {
    assert_eq!(symbolic_text("x = symbol(\"x\"); simplify(x/x)"), "x/x");
    assert_eq!(
        symbolic_text("y = symbol(\"y\"); assume(y != 0); simplify(y/y)"),
        "1"
    );
}

// ---------------------------------------------------------------------------
// ExactnessFlagTests (1 row)
// ---------------------------------------------------------------------------

/// ExactnessFlagTests.cs:141 — sqrt(2) in the language is a Real value, not a symbolic radical.
#[test]
fn exactness_sqrt_of_two_is_a_real_value() {
    let inspection = eval("inspect(sqrt(2))");
    let fields = record_fields(&inspection, "Inspection");
    assert_eq!(text(field(fields, "type")), "Real");
    assert!(!boolean(field(fields, "exact")));
}

// ---------------------------------------------------------------------------
// FactorMetamorphicTests / PrettyParenthesesTests (2 rows)
// ---------------------------------------------------------------------------

/// PrettyParenthesesTests.cs:96 — the factored square has one pair.
#[test]
fn pretty_parentheses_factored_square_has_one_pair() {
    assert_eq!(
        symbolic_text("x = symbol(\"x\"); factor(-x^2 - 2*x - 1)"),
        "-(x + 1)^2"
    );
}

// ---------------------------------------------------------------------------
// SeriesLaurentAndBranchPointTests (1 row)
// ---------------------------------------------------------------------------

/// SeriesLaurentAndBranchPointTests.cs:33 — series(sqrt(x)) is unchanged at the branch point.
#[test]
fn series_laurent_sqrt_at_zero_is_unchanged() {
    assert_eq!(
        symbolic_text("x = symbol(\"x\"); series(sqrt(x), x, 0, 2)"),
        "sqrt(x)"
    );
}

// ---------------------------------------------------------------------------
// SystemSolveProofTests (3 rows)
// ---------------------------------------------------------------------------

fn assert_wire_absence(script: &str) {
    let record = eval(script);
    let fields = record_fields(&record, "SystemSolveResult");
    assert_eq!(variant(field(fields, "status")), "Unevaluated");
    assert!(!boolean(field(fields, "complete")));
    assert!(vector(field(fields, "solutions")).is_empty());
    let diagnostics = vector(field(fields, "diagnostics"));
    assert!(!diagnostics.is_empty());
    let diagnostic = record_fields(&diagnostics[0], "Diagnostic");
    assert_eq!(text(field(diagnostic, "code")), "system-solve.unevaluated");
    assert_ne!(
        variant(field(diagnostic, "category")),
        "InternalInvariantFailure"
    );
    assert!(text(field(diagnostic, "message")).starts_with("system-solve."));
}

/// SystemSolveProofTests.cs:372 — an unproved empty set is Unevaluated with a diagnostic.
#[test]
fn system_solve_proof_wire_absence_case_zero() {
    assert_wire_absence(
        "x = symbol(\"x\"); y = symbol(\"y\"); solve_system_full([x^2 + y^2 == 1], [x, y])",
    );
}

/// SystemSolveProofTests.cs:372 (case 1).
#[test]
fn system_solve_proof_wire_absence_case_one() {
    assert_wire_absence(
        "x = symbol(\"x\"); y = symbol(\"y\"); solve_system_full([x^2 == 1], [x, y])",
    );
}

/// SystemSolveProofTests.cs:372 (case 2).
#[test]
fn system_solve_proof_wire_absence_case_two() {
    assert_wire_absence(
        "x = symbol(\"x\"); y = symbol(\"y\"); solve_system_full([x^4 + 1 == 0, y == x], [x, y])",
    );
}

// ---------------------------------------------------------------------------
// ExpectedFailureBoundaryTests (1 row)
// ---------------------------------------------------------------------------

/// ExpectedFailureBoundaryTests.cs:55 — a two-sided pole still reports opposing sides.
#[test]
fn expected_failure_limit_two_sided_pole_reports_opposing_sides() {
    let record = eval("x = symbol(\"x\"); limit(1/x, x, 0)");
    assert_eq!(
        variant(field(record_fields(&record, "LimitResult"), "status")),
        "DoesNotExist"
    );
    let left = eval("x = symbol(\"x\"); limit_left(1/x, x, 0)");
    let right = eval("x = symbol(\"x\"); limit_right(1/x, x, 0)");
    assert_eq!(
        variant(field(record_fields(&left, "LimitResult"), "status")),
        "MinusInfinity"
    );
    assert_eq!(
        variant(field(record_fields(&right, "LimitResult"), "status")),
        "PlusInfinity"
    );
}

// ---------------------------------------------------------------------------
// LimitExistenceTriStateTests (1 row)
// ---------------------------------------------------------------------------

/// LimitExistenceTriStateTests.cs:21 — an undetermined side has no value in any direction.
#[test]
fn limit_existence_undetermined_side_has_no_value() {
    for script in [
        "x = symbol(\"x\"); limit(x*sin(1/x), x, 0)",
        "x = symbol(\"x\"); limit_left(x*sin(1/x), x, 0)",
        "x = symbol(\"x\"); limit_right(x*sin(1/x), x, 0)",
    ] {
        let record = eval(script);
        let fields = record_fields(&record, "LimitResult");
        assert_eq!(variant(field(fields, "status")), "Unevaluated", "{script}");
        assert!(matches!(field(fields, "value"), Value::Void), "{script}");
        assert!(matches!(field(fields, "left"), Value::Void), "{script}");
        assert!(matches!(field(fields, "right"), Value::Void), "{script}");
    }
}

// ---------------------------------------------------------------------------
// SuiteIntegrationTests / SuiteIntegrationTests2 (2 rows)
// ---------------------------------------------------------------------------

/// SuiteIntegrationTests.cs:14 — symbol() returns a Symbolic value.
#[test]
fn suite_integration_symbol_builtin_works() {
    let value = eval("z = symbol(\"x\")");
    assert!(matches!(value, Value::Symbolic(_)));
    assert_eq!(symbolic(&value).to_string(), "x");
}

/// SuiteIntegrationTests2.cs:12 — apart() through the engine returns a Symbolic value.
#[test]
fn suite_integration_apart_through_engine() {
    let value = eval("x = symbol(\"x\"); apart(1/(x^2 - 1), x)");
    assert!(matches!(value, Value::Symbolic(_)));
    assert_eq!(symbolic(&value).to_string(), "-1/2/(x + 1) + 1/2/(x - 1)");
}

// ---------------------------------------------------------------------------
// HyperbolicBuiltinsAndNegativeIntegerPowersTests (2 rows)
// ---------------------------------------------------------------------------

/// HyperbolicBuiltinsAndNegativeIntegerPowersTests.cs:120 — negative bases under the principal
/// square root answer exactly, while other non-integer powers keep the typed error.
#[test]
fn hyperbolic_negative_square_roots_and_half_powers_answer_exactly() {
    assert_eq!(symbolic_text("sqrt(-1)"), "i");
    assert_eq!(symbolic_text("(-1)^(1/2)"), "i");
    assert_eq!(symbolic_text("sqrt(-4)"), "2*i");
    for script in ["2^(1/2)", "(-8)^(1/3)", "(-1)^(1/3)"] {
        let error = refusal(script);
        assert_eq!(
            error.message, "Non-integer exponents are not yet supported.",
            "{script}"
        );
    }
}

/// HyperbolicBuiltinsAndNegativeIntegerPowersTests.cs:160 — x^-1 stays the symbolic reciprocal.
#[test]
fn hyperbolic_symbol_to_the_minus_one_stays_symbolic() {
    let value = eval("x = symbol(\"x\"); x^-1");
    assert!(matches!(value, Value::Symbolic(_)));
    assert_eq!(display(&value), "x^-1");
}

// ---------------------------------------------------------------------------
// CompilationTests (1 row)
// ---------------------------------------------------------------------------

/// CompilationTests.cs:52 — scalar kernel evaluation is exact and answers 5.
#[test]
fn compilation_kernel_scalar_evaluation() {
    let value = eval("x = symbol(\"x\"); evalir(compile(x^2 + 1, [x]), [2], 40)");
    match &value {
        Value::Integer(n) => assert_eq!(n.to_decimal(), "5"),
        other => panic!("expected Integer 5, got {}", other.kind_name()),
    }
}

// ---------------------------------------------------------------------------
// DiagnosticContractTests (1 row)
// ---------------------------------------------------------------------------

/// DiagnosticContractTests.cs:30 — a budget-exceeded transformation carries its diagnostic.
#[test]
fn diagnostic_contract_bounded_transformation_carries_budget_diagnostic() {
    let terms: Vec<String> = (1..=420).map(|i| format!("exp(log(x+{i}))")).collect();
    let script = format!("x = symbol(\"x\"); simplify_full({})", terms.join(" + "));
    let record = eval(&script);
    let fields = record_fields(&record, "TransformResult");
    assert_eq!(variant(field(fields, "status")), "BudgetExceeded");
    assert!(boolean(field(fields, "budget_exceeded")));
    let diagnostics = vector(field(fields, "diagnostics"));
    assert!(!diagnostics.is_empty());
    let diagnostic = record_fields(&diagnostics[0], "Diagnostic");
    assert_eq!(text(field(diagnostic, "code")), "transform.budget-exceeded");
    assert_eq!(variant(field(diagnostic, "category")), "BudgetExceeded");
}
