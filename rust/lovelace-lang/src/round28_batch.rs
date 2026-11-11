//! Round-28 ports of the Run/Suite rows the round-19 refusals left deferred and the round-27
//! production fixes unblock.
//!
//! Every test repeats the C# assertion it is mapped to: the C# test text is the contract, and a
//! refusal (not a weaker assertion) is recorded in the round report when the Rust observable
//! differs.

use crate::{Engine, LangError, Value};

fn eval(script: &str) -> Result<Value, LangError> {
    Engine::new().eval(script)
}

fn display(script: &str) -> String {
    eval(script)
        .unwrap_or_else(|error| panic!("'{script}' failed: {}", error.message))
        .display()
}

fn typed(script: &str) -> String {
    let mut engine = Engine::new();
    let value = engine
        .eval(script)
        .unwrap_or_else(|error| panic!("'{script}' failed: {}", error.message));
    engine.format_typed(&value)
}

fn refusal(script: &str) -> LangError {
    match eval(script) {
        Err(error) => error,
        Ok(value) => panic!("'{script}' answered {}", value.display()),
    }
}

fn record_field<'a>(value: &'a Value, name: &str) -> &'a Value {
    match value {
        Value::Record(type_name, fields) => fields
            .iter()
            .find(|(field, _)| field.as_str() == name)
            .map(|(_, value)| value)
            .unwrap_or_else(|| panic!("record {type_name} has no field '{name}'")),
        other => panic!("expected a record, got {}", other.kind_name()),
    }
}

fn array_items(value: &Value) -> &[Value] {
    match value {
        Value::Vector(items) => items,
        Value::Array(array) => array.as_slice(),
        other => panic!("expected an array, got {}", other.kind_name()),
    }
}

fn array_len(value: &Value) -> usize {
    array_items(value).len()
}

fn array_shape(value: &Value) -> Vec<usize> {
    match value {
        Value::Vector(items) => vec![items.len()],
        Value::Array(array) => array.shape().to_vec(),
        other => panic!("expected an array, got {}", other.kind_name()),
    }
}

fn solution_values(value: &Value) -> Vec<String> {
    array_items(record_field(value, "solutions"))
        .iter()
        .map(|solution| record_field(solution, "value").display())
        .collect()
}

fn solution_conditions(value: &Value) -> Vec<String> {
    array_items(record_field(value, "solutions"))
        .iter()
        .flat_map(|solution| {
            array_items(record_field(solution, "conditions"))
                .iter()
                .map(Value::display)
        })
        .collect()
}

fn family_conditions(value: &Value) -> Vec<String> {
    array_items(record_field(value, "families"))
        .iter()
        .flat_map(|family| {
            array_items(record_field(family, "conditions"))
                .iter()
                .map(Value::display)
        })
        .collect()
}

fn assignments(value: &Value) -> Vec<String> {
    array_items(record_field(value, "solutions"))
        .iter()
        .map(|solution| {
            array_items(record_field(solution, "bindings"))
                .iter()
                .map(|binding| {
                    format!(
                        "{}={}",
                        record_field(binding, "name").display(),
                        record_field(binding, "value").display()
                    )
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
        .collect()
}

// ---------------------------------------------------------------------------
// SolveSessionAssumptionTests (7 deferred rows)
// ---------------------------------------------------------------------------

#[test]
fn solve_does_not_publish_a_root_the_session_assumption_excludes() {
    let value = eval("x = symbol(\"x\"); assume(x > 5); solve(x == 3, x)").expect("record");
    assert_eq!(record_field(&value, "status").display(), "NoSolutions");
    assert_eq!(record_field(&value, "complete").display(), "True");
    assert_eq!(record_field(&value, "completeness").display(), "Complete");
    assert!(solution_values(&value).is_empty());
    assert_eq!(array_len(record_field(&value, "diagnostics")), 0);
}

#[test]
fn solve_keeps_only_the_roots_the_assumption_admits() {
    let positive =
        eval("x = symbol(\"x\"); assume_positive(x); solve(x^2 == 4, x)").expect("record");
    assert_eq!(record_field(&positive, "status").display(), "Solved");
    assert_eq!(record_field(&positive, "complete").display(), "True");
    assert_eq!(solution_values(&positive), vec!["2"]);
    assert!(solution_conditions(&positive).is_empty());

    let negative = eval("x = symbol(\"x\"); assume(x < 0); solve(x^2 == 4, x)").expect("record");
    assert_eq!(record_field(&negative, "status").display(), "Solved");
    assert_eq!(record_field(&negative, "complete").display(), "True");
    assert_eq!(solution_values(&negative), vec!["-2"]);
}

#[test]
fn solve_honours_an_equality_assumption() {
    let value = eval("x = symbol(\"x\"); assume(x == 2); solve(x^2 == 4, x)").expect("record");
    assert_eq!(record_field(&value, "status").display(), "Solved");
    assert_eq!(solution_values(&value), vec!["2"]);
}

#[test]
fn solve_honours_a_domain_assumption() {
    let real = eval("x = symbol(\"x\"); assume_real(x); solve(x^2 + 1 == 0, x)").expect("record");
    assert_eq!(record_field(&real, "status").display(), "NoSolutions");
    assert_eq!(record_field(&real, "complete").display(), "True");
    assert!(solution_values(&real).is_empty());

    let integer = eval("x = symbol(\"x\"); assume_integer(x); solve(x == 1/2, x)").expect("record");
    assert_eq!(record_field(&integer, "status").display(), "NoSolutions");
    assert!(solution_values(&integer).is_empty());
}

#[test]
fn one_session_one_assumption_store_for_simplify_and_solve() {
    let solved = eval("x = symbol(\"x\"); assume(x > 0); solve(x^2 == 4, x)").expect("record");
    assert_eq!(solution_values(&solved), vec!["2"]);
    assert_eq!(
        display("x = symbol(\"x\"); assume(x > 0); simplify(sqrt(x^2))"),
        "x"
    );
}

#[test]
fn a_response_that_cannot_decide_is_carried_as_a_condition() {
    let family = eval("x = symbol(\"x\"); assume(x > 5); solve(sin(x) == 0, x)").expect("record");
    assert_eq!(record_field(&family, "status").display(), "Solved");
    assert_eq!(array_len(record_field(&family, "families")), 1);
    assert_eq!(family_conditions(&family), vec!["x > 5"]);
}

#[test]
fn system_solve_honours_the_same_store() {
    let refuted =
        eval("x = symbol(\"x\"); assume(x > 5); solve_system([x == 3], [x])").expect("record");
    assert_eq!(record_field(&refuted, "status").display(), "NoSolutions");
    assert_eq!(record_field(&refuted, "complete").display(), "True");
    assert!(solution_values(&refuted).is_empty());

    let filtered = eval(
        "x = symbol(\"x\"); y = symbol(\"y\"); assume(y > 0); solve_system([x == 1, y^2 == 4], [x, y])",
    )
    .expect("record");
    assert_eq!(record_field(&filtered, "status").display(), "Solved");
    assert_eq!(assignments(&filtered), vec!["x=1, y=2"]);
}

// ---------------------------------------------------------------------------
// DescriptorExampleTests (4 deferred rows whose snippets the round-27 builtins
// unblock: abs, assume_integer, assume_negative, assume_real)
// ---------------------------------------------------------------------------

const EXAMPLE_PRELUDE: &str = "x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\"); ";

#[test]
fn descriptor_examples_that_were_refused_now_execute() {
    let examples: &[(&str, &str)] = &[
        ("abs", "abs(fft([0, 1, 0, 0]))"),
        ("assume_integer", "assume_integer(x)"),
        ("assume_negative", "assume_negative(x)"),
        ("assume_real", "assume_real(x)"),
    ];
    for (function, example) in examples {
        let mut engine = Engine::new();
        engine
            .eval(EXAMPLE_PRELUDE)
            .unwrap_or_else(|error| panic!("descriptor prelude failed: {}", error.message));
        engine.eval(example).unwrap_or_else(|error| {
            panic!(
                "the descriptor example for '{function}' does not execute: {example}: {}",
                error.message
            )
        });
    }
}

// ---------------------------------------------------------------------------
// DspPluginTests (5 deferred rows)
// ---------------------------------------------------------------------------

#[test]
fn abs_of_a_complex_vector_is_the_magnitude_spectrum() {
    assert_eq!(typed("abs(fft([0, 1, 0, 0]))"), "[1, 1, 1, 1] (Vector)");
}

#[test]
fn abs_of_a_non_complex_array_names_the_complex_requirement() {
    let error = refusal("abs([1, 2, 3])");
    assert_eq!(error.code, "InvalidOperation");
    assert!(error.message.contains("Complex array"), "{}", error.message);
}

#[test]
fn complex_accessors_read_real_components() {
    assert_eq!(typed("z = fft([0, 1, 0, 0])[1]; re(z)"), "0 (Real)");
    assert_eq!(typed("z = fft([0, 1, 0, 0])[1]; im(z)"), "-1 (Real)");
    assert_eq!(typed("z = fft([0, 1, 0, 0])[1]; abs(z)"), "1 (Real)");
    assert_eq!(typed("z = fft([0, 1, 0, 0])[1]; conj(z)"), "1i (Complex)");
}

#[test]
fn re_im_conj_map_a_complex_vector_elementwise() {
    assert_eq!(typed("re(fft([0, 1, 0, 0]))"), "[1, 0, -1, 0] (Vector)");
    assert_eq!(typed("im(fft([0, 1, 0, 0]))"), "[0, -1, 0, 1] (Vector)");
    assert_eq!(
        typed("conj(fft([0, 1, 0, 0]))"),
        "[1, 1i, -1, -1i] (Vector)"
    );
}

// ---------------------------------------------------------------------------
// ArrayTests.Squeeze_RemovesSingletonDims (1 deferred row)
// ---------------------------------------------------------------------------

#[test]
fn squeeze_of_a_singleton_dimension_is_a_vector() {
    assert_eq!(typed("squeeze([[[1, 2]]])"), "[1, 2] (Vector)");
}

// ---------------------------------------------------------------------------
// SolveEmptySetTests.TheFoldedBooleanIsUnchangedOutsideTheSymbolicChannel (1 row)
// ---------------------------------------------------------------------------

#[test]
fn the_folded_boolean_is_unchanged_outside_the_symbolic_channel() {
    assert_eq!(display("type(0 == 1)"), "Boolean");
    assert_eq!(display("1 == 1"), "True");
    assert_eq!(display("not(0 == 1)"), "True");
}

// ---------------------------------------------------------------------------
// AbsSymbolicRoundTripTests.Abs_Of_Integer_Still_Returns_The_Magnitude (1 row)
// ---------------------------------------------------------------------------

#[test]
fn abs_of_an_integer_is_still_the_magnitude() {
    assert_eq!(display("abs(-3) == 3"), "True");
    assert_eq!(display("abs(3) == 3"), "True");
}

// ---------------------------------------------------------------------------
// RelationsAndFamiliesTests (2 engine-level deferred rows)
// ---------------------------------------------------------------------------

#[test]
fn assumptions_conjunction_and_negation_through_the_engine() {
    let mut engine = Engine::new();
    engine.eval("x = symbol(\"x\")").expect("symbol");
    engine
        .eval("assume(and(x > 0, x < 5))")
        .expect("conjunction");
    let value = engine.eval("assumptions()").expect("assumptions");
    assert_eq!(record_field(&value, "display").display(), "x > 0; x < 5");
    assert_eq!(array_len(record_field(&value, "assumptions")), 2);
    engine.eval("assume(not(x == 2))").expect("negation");
    let value = engine.eval("assumptions()").expect("assumptions");
    assert_eq!(
        record_field(&value, "display").display(),
        "x > 0; x < 5; x != 2"
    );
    assert_eq!(array_len(record_field(&value, "assumptions")), 3);
    assert_eq!(
        display("x = symbol(\"x\"); and(x > 0, x < 1)"),
        "x < 1 and x > 0"
    );
}

#[test]
fn solve_sin_zero_publishes_the_family() {
    let value = eval("x = symbol(\"x\"); solve(sin(x) == 0, x)").expect("record");
    assert_eq!(record_field(&value, "status").display(), "Solved");
    let families = array_items(record_field(&value, "families"));
    assert_eq!(families.len(), 1);
    let family = &families[0];
    assert_eq!(record_field(family, "template").display(), "k*pi");
    assert_eq!(record_field(family, "parameter").display(), "k");
    match record_field(family, "parameter_domain") {
        Value::Domain(domain) => assert_eq!(domain, "integer"),
        other => panic!("parameter_domain is {}", other.kind_name()),
    }
}

// ---------------------------------------------------------------------------
// EvalirDigitArgumentContractTests (4 language-level rows)
// ---------------------------------------------------------------------------

#[test]
fn evalir_integral_digit_counts_are_still_honoured() {
    for builtin in ["evalir", "evalir_batch"] {
        let points = if builtin == "evalir" { "[1]" } else { "[1, 2]" };
        let script = format!("x = symbol(\"x\"); {builtin}(compile(x*1/3, [x]), {points}, 40)");
        let rendered = display(&script);
        assert!(
            rendered.contains("3333333333333333333333333333333333333333"),
            "{builtin}: {rendered}"
        );
    }
}

#[test]
fn two_evalir_places_answer_two_places() {
    for builtin in ["evalir", "evalir_batch"] {
        let points = if builtin == "evalir" { "[1]" } else { "[1, 2]" };
        let rendered = display(&format!(
            "x = symbol(\"x\"); {builtin}(compile(x*1/3, [x]), {points}, 2)"
        ));
        assert!(rendered.contains("0.33"), "{builtin}: {rendered}");
        assert!(!rendered.contains("0.333"), "{builtin}: {rendered}");
    }
}

#[test]
fn evalf_digit_count_refusal_is_unchanged() {
    let error = refusal("x = symbol(\"x\"); evalf(1/3, 2.5)");
    assert_eq!(
        error.message,
        "evalf(): argument 2 (digits) must be a Natural or Integer digit count between 1 and 2147483647; got Real."
    );
    assert_eq!(
        (error.code, error.category),
        ("InvalidArgument", "TypeMismatch")
    );
}

// ---------------------------------------------------------------------------
// ArrayAllocationRefusalTests.Zeros_ShapesWithinTheBudget_StillBuild (1 row)
// ---------------------------------------------------------------------------

#[test]
fn zeros_shapes_within_the_budget_still_build() {
    let mut engine = Engine::new();
    let value = engine.eval("zeros(2, 3)").expect("zeros(2, 3)");
    assert_eq!(array_shape(&value), vec![2, 3]);
    let value = engine.eval("zeros(2, 0)").expect("zeros(2, 0)");
    assert_eq!(array_shape(&value), vec![2, 0]);
    let value = engine.eval("zeros(1000000)").expect("zeros(1000000)");
    assert_eq!(array_len(&value), 1_000_000);
}
// ---------------------------------------------------------------------------
// CapabilitiesBuiltinTests.Capabilities_ReturnsARecordNamedCapabilitiesResult (1 row)
// ---------------------------------------------------------------------------

#[test]
fn capabilities_returns_a_record_named_capabilities_result() {
    match eval("capabilities()").expect("capabilities") {
        Value::Record(type_name, _) => assert_eq!(type_name, "CapabilitiesResult"),
        other => panic!("capabilities() is {}, not a Record", other.kind_name()),
    }
}
