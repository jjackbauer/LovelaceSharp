//! Wire-contract ports from Lovelace.Run.Tests: print capture, registry queries, limit/divrem
//! records, structured projection, arity metadata, committed state and source positions.

use super::parity_batch::*;
use super::*;
use lovelace_json::Json;

fn eval(script: &str) -> RunResult {
    envelope(script, &[])
}

fn eval_keep_vars(script: &str) -> RunResult {
    run_args(&["--eval", script, "--omit-functions"])
}

fn output(r: &RunResult) -> Vec<&str> {
    array(get(&r.env, "output")).iter().map(text).collect()
}

fn has_output(r: &RunResult) -> Vec<bool> {
    array(get(&r.env, "timings"))
        .iter()
        .map(|t| boolean(get(t, "hasOutput")))
        .collect()
}

// ---------------------------------------------------------------------------
// PrintOutputTests
// ---------------------------------------------------------------------------

#[test]
fn print_two_lines() {
    let r = eval("print(\"A\"); print(\"B\")");
    assert_eq!(r.exit, 0);
    assert_eq!(output(&r), vec!["A", "B"]);
    assert!(
        r.stdout.contains("\"output\":[\"A\",\"B\"]"),
        "raw: {}",
        r.stdout
    );
    assert!(!r.stdout.contains("\\r"));
}

#[test]
fn print_three_lines() {
    let r = eval("print(\"A\"); print(\"B\"); print(\"C\")");
    assert_eq!(r.exit, 0);
    assert_eq!(output(&r), vec!["A", "B", "C"]);
    assert!(
        r.stdout.contains("\"output\":[\"A\",\"B\",\"C\"]"),
        "raw: {}",
        r.stdout
    );
    assert!(!r.stdout.contains("\\r"));
}

#[test]
fn print_formatted_values() {
    let r = eval("x = symbol(\"x\"); print(x^2 + 1); print(42); print([1, 2])");
    assert_eq!(r.exit, 0);
    assert_eq!(output(&r), vec!["x^2 + 1", "42", "[1, 2]"]);
    assert!(!r.stdout.contains("\\r"));
}

#[test]
fn print_blank_line_is_empty_element() {
    let r = eval("print(\"\"); print(\"A\")");
    assert_eq!(r.exit, 0);
    assert_eq!(output(&r), vec!["", "A"]);
    assert!(
        r.stdout.contains("\"output\":[\"\",\"A\"]"),
        "raw: {}",
        r.stdout
    );
    assert!(!r.stdout.contains("\\r"));
}

#[test]
fn print_no_element_has_carriage_return() {
    let r = eval("print(\"A\"); print(\"B\"); print(1); print(\"\")");
    for line in output(&r) {
        assert!(!line.contains('\r'));
    }
}

// ---------------------------------------------------------------------------
// PrintOutputTrailingLineTests
// ---------------------------------------------------------------------------

#[test]
fn trailing_blank_print_is_kept() {
    let r = eval("print(\"a\"); print()");
    assert_eq!(r.exit, 0);
    assert_eq!(output(&r), vec!["a", ""]);
    assert_eq!(has_output(&r), vec![true, true]);
    assert!(
        r.stdout.contains("\"output\":[\"a\",\"\"]"),
        "raw: {}",
        r.stdout
    );
}

#[test]
fn trailing_two_blank_prints_are_kept() {
    let r = eval("print(\"a\"); print(); print()");
    assert_eq!(r.exit, 0);
    assert_eq!(output(&r), vec!["a", "", ""]);
    assert_eq!(has_output(&r), vec![true, true, true]);
}

#[test]
fn single_blank_print_is_one_element() {
    let r = eval("print()");
    assert_eq!(r.exit, 0);
    assert_eq!(output(&r), vec![""]);
}

#[test]
fn interior_blank_print_unchanged() {
    let r = eval("print(\"a\"); print(\"\"); print(\"b\")");
    assert_eq!(r.exit, 0);
    assert_eq!(output(&r), vec!["a", "", "b"]);
}

#[test]
fn lone_print_has_no_trailing_empty() {
    let r = eval("print(\"a\")");
    assert_eq!(r.exit, 0);
    assert_eq!(output(&r), vec!["a"]);
}

#[test]
fn failing_path_keeps_trailing_blank() {
    let r = eval("print(\"a\"); print(); det(1)");
    assert_eq!(r.exit, 1);
    assert!(!boolean(get(&r.env, "ok")));
    assert_eq!(output(&r), vec!["a", ""]);
    // The statement that FAILED is still a statement that ran and is timed: the C# assertion
    // (PrintOutputTrailingLineTests.TheFailingPath_KeepsItsTrailingBlankLine) expects three
    // entries, the last one hasOutput=false for the refused det() call.
    assert_eq!(has_output(&r), vec![true, true, false]);
}

#[test]
fn printing_statements_agree_with_output_lines() {
    let r = eval("print(\"a\"); print(); print(\"b\"); print()");
    assert_eq!(output(&r), vec!["a", "", "b", ""]);
    let printed = has_output(&r).iter().filter(|b| **b).count();
    assert_eq!(printed, output(&r).len());
}

// ---------------------------------------------------------------------------
// PrintOutputOnFailureTests
// ---------------------------------------------------------------------------

#[test]
fn failure_keeps_line_printed_before_it() {
    let r = eval("print(\"hello\"); det(1)");
    assert_eq!(r.exit, 1);
    assert!(!boolean(get(&r.env, "ok")), "stdout: {}", r.stdout);
    assert_eq!(output(&r), vec!["hello"]);
    assert!(
        r.stdout.contains("\"output\":[\"hello\"]"),
        "raw: {}",
        r.stdout
    );
    assert!(boolean(get(&array(get(&r.env, "timings"))[0], "hasOutput")));
}

#[test]
fn failure_keeps_nothing_that_never_ran() {
    let r = eval("print(\"before\"); print(\"also before\"); det(1); print(\"UNREACHED\")");
    assert_eq!(r.exit, 1);
    assert_eq!(output(&r), vec!["before", "also before"]);
    assert!(!r.stdout.contains("UNREACHED"));
}

#[test]
fn failure_that_printed_nothing_has_empty_output() {
    let r = eval("det(1)");
    assert_eq!(r.exit, 1);
    assert!(array(get(&r.env, "output")).is_empty());
    assert!(
        array(get(&r.env, "timings"))
            .iter()
            .all(|t| !boolean(get(t, "hasOutput")))
    );
}

#[test]
fn parse_failure_has_empty_output() {
    let r = eval("print(\"UNREACHED\"); 1+1;\n)");
    assert_eq!(r.exit, 1);
    assert!(array(get(&r.env, "output")).is_empty());
    assert!(!r.stdout.contains("UNREACHED"));
}

#[test]
fn success_path_output_unchanged() {
    let r = eval("print(\"hello\"); 1+1");
    assert_eq!(r.exit, 0);
    assert_eq!(output(&r), vec!["hello"]);
    assert!(
        r.stdout.contains("\"output\":[\"hello\"]"),
        "raw: {}",
        r.stdout
    );
}

// ---------------------------------------------------------------------------
// RegistryQueryTests
// ---------------------------------------------------------------------------

#[test]
fn registry_listing_answers_without_evaluating() {
    let r = run_args(&["--functions", "det"]);
    assert_eq!(r.exit, 0);
    assert!(boolean(get(&r.env, "ok")));
    assert!(opt(&r.env, "result").is_none());
    assert!(opt(&r.env, "timings").is_none());
    assert!(opt(&r.env, "revision").is_none());
    assert_eq!(text(get(&r.env, "pattern")), "det");
    let functions = array(get(&r.env, "functions"));
    assert_eq!(functions.len(), 1);
    assert_eq!(text(get(&functions[0], "name")), "det");
    assert_eq!(text(get(&functions[0], "category")), "Linear Algebra");
    assert_eq!(number(get(&functions[0], "parameterCount")), 1);
}

#[test]
fn registry_listing_carries_no_documentation() {
    let r = run_args(&["--functions", "sin"]);
    let functions = array(get(&r.env, "functions"));
    assert!(!functions.is_empty());
    for entry in functions {
        for key in [
            "name",
            "parameters",
            "minArity",
            "parameterCount",
            "variadic",
            "category",
        ] {
            assert!(opt(entry, key).is_some(), "missing {key}");
        }
        for key in ["summary", "examples", "related"] {
            assert!(opt(entry, key).is_none(), "listing carried {key}");
        }
    }
}

#[test]
fn registry_describes_det_with_the_declared_metadata() {
    let r = run_args(&["--function", "det"]);
    assert_eq!(r.exit, 0);
    let entry = get(&r.env, "function");
    assert!(opt(&r.env, "functions").is_none());
    assert_eq!(text(get(entry, "name")), "det");
    assert_eq!(
        text(get(entry, "summary")),
        "Determinant (fraction-free for symbolic matrices)."
    );
    assert_eq!(text(get(entry, "returnKind")), "Real | Symbolic");
    assert_eq!(text(get(entry, "category")), "Linear Algebra");
    let params: Vec<&str> = array(get(entry, "parameters")).iter().map(text).collect();
    assert_eq!(params, vec!["m"]);
    let related: Vec<&str> = array(get(entry, "related")).iter().map(text).collect();
    assert_eq!(related, vec!["inv", "matrix_rank"]);
}

#[test]
fn registry_page_reports_its_own_truncation() {
    let page = run_args(&["--functions", "--function-limit", "3"]);
    assert_eq!(number(get(&page.env, "returned")), 3);
    assert!(number(get(&page.env, "total")) > 3);
    assert!(boolean(get(&page.env, "truncated")));
    let whole = run_args(&["--functions", "zzz-no-such-fragment"]);
    assert_eq!(number(get(&whole.env, "total")), 0);
    assert!(!boolean(get(&whole.env, "truncated")));
}

#[test]
fn registry_listing_is_ordinal_and_stable() {
    let first = run_args(&["--functions", "--function-limit", "200"]);
    let names: Vec<&str> = array(get(&first.env, "functions"))
        .iter()
        .map(|f| text(get(f, "name")))
        .collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted);
    let again = run_args(&["--functions", "--function-limit", "200"]);
    let names2: Vec<&str> = array(get(&again.env, "functions"))
        .iter()
        .map(|f| text(get(f, "name")))
        .collect();
    assert_eq!(names, names2);
}

#[test]
fn registry_envelope_still_carries_the_full_registry() {
    let r = run_args(&["--eval", "1 + 1"]);
    assert_eq!(r.exit, 0);
    let functions = array(get(&r.env, "functions"));
    let det = functions
        .iter()
        .find(|f| text(get(f, "name")) == "det")
        .expect("det in registry");
    assert_eq!(text(get(det, "category")), "Linear Algebra");
    assert!(opt(det, "summary").is_some());
}

// ---------------------------------------------------------------------------
// LimitShortFormRecordTests
// ---------------------------------------------------------------------------

fn limit_record(r: &RunResult) -> &Json {
    let structured = get(get(&r.env, "result"), "structured");
    assert_eq!(text(get(structured, "kind")), "Record");
    assert_eq!(text(get(structured, "type")), "LimitResult");
    structured
}

fn enum_field<'a>(field: &'a Json, type_name: &str) -> &'a str {
    assert_eq!(text(get(field, "kind")), "Enum");
    assert_eq!(text(get(field, "type")), type_name);
    text(get(field, "value"))
}

#[test]
fn limit_short_forms_publish_limit_result_records() {
    let rows = [
        (
            "x = symbol(\"x\"); limit(1/x, x, 0)",
            "DoesNotExist",
            "Boolean",
            "false",
        ),
        (
            "x = symbol(\"x\"); limit_left(1/x, x, 0)",
            "MinusInfinity",
            "Boolean",
            "true",
        ),
        (
            "x = symbol(\"x\"); limit_right(1/x, x, 0)",
            "PlusInfinity",
            "Boolean",
            "true",
        ),
        (
            "x = symbol(\"x\"); limit(sin(x), x, inf)",
            "Unevaluated",
            "Null",
            "",
        ),
        (
            "x = symbol(\"x\"); limit_left(sin(x), x, inf)",
            "Unevaluated",
            "Null",
            "",
        ),
        (
            "x = symbol(\"x\"); limit_right(sin(x), x, inf)",
            "Unevaluated",
            "Null",
            "",
        ),
        (
            "x = symbol(\"x\"); limit(sin(x)/x, x, 0)",
            "Value",
            "Boolean",
            "true",
        ),
        (
            "x = symbol(\"x\"); limit_left(sin(x)/x, x, 0)",
            "Value",
            "Boolean",
            "true",
        ),
        (
            "x = symbol(\"x\"); limit_right(sin(x)/x, x, 0)",
            "Value",
            "Boolean",
            "true",
        ),
    ];
    for (script, status, exists_kind, exists_value) in rows {
        let r = eval(script);
        assert_eq!(r.exit, 0, "{script}");
        assert!(boolean(get(&r.env, "ok")), "{script}");
        let record = limit_record(&r);
        assert_eq!(
            enum_field(record_field(record, "status"), "LimitStatus"),
            status,
            "{script}"
        );
        let exists = record_field(record, "exists");
        assert_eq!(text(get(exists, "kind")), exists_kind, "{script}");
        if exists_kind == "Boolean" {
            assert_eq!(text(get(exists, "value")), exists_value, "{script}");
        }
    }
}

#[test]
fn limit_two_sided_short_form_is_the_full_record() {
    let pairs = [
        ("limit(1/x, x, 0)", "limit_full(1/x, x, 0)"),
        ("limit(sin(x)/x, x, 0)", "limit_full(sin(x)/x, x, 0)"),
        ("limit(sin(x), x, inf)", "limit_full(sin(x), x, inf)"),
    ];
    for (short, full) in pairs {
        let a = eval(&format!("x = symbol(\"x\"); {short}"));
        let b = eval(&format!("x = symbol(\"x\"); {full}"));
        assert_eq!(a.exit, 0);
        assert_eq!(b.exit, 0);
        assert_eq!(
            get(get(&a.env, "result"), "structured"),
            get(get(&b.env, "result"), "structured"),
            "{short} vs {full}"
        );
    }
}

#[test]
fn limit_no_limit_keeps_every_prose_value() {
    let r = eval("x = symbol(\"x\"); limit(1/x, x, 0)");
    assert_eq!(r.exit, 0);
    let record = limit_record(&r);
    assert_eq!(text(get(record_field(record, "left"), "pretty")), "-inf");
    assert_eq!(text(get(record_field(record, "right"), "pretty")), "inf");
    let left: Vec<&str> = array(get(record_field(record, "left_conditions"), "elements"))
        .iter()
        .map(|e| text(get(e, "value")))
        .collect();
    assert_eq!(left, vec!["x < 0"]);
    let right: Vec<&str> = array(get(record_field(record, "right_conditions"), "elements"))
        .iter()
        .map(|e| text(get(e, "value")))
        .collect();
    assert_eq!(right, vec!["x > 0"]);
    assert_eq!(text(get(record_field(record, "value"), "kind")), "Null");
    assert_eq!(
        enum_field(record_field(record, "exactness"), "SolutionExactness"),
        "Exact"
    );
    assert!(array(get(record_field(record, "diagnostics"), "elements")).is_empty());
}

#[test]
fn limit_one_sided_short_forms_publish_structure() {
    let left = eval("x = symbol(\"x\"); limit_left(1/x, x, 0)");
    assert_eq!(left.exit, 0);
    let record = limit_record(&left);
    assert_eq!(
        enum_field(record_field(record, "status"), "LimitStatus"),
        "MinusInfinity"
    );
    assert_eq!(text(get(record_field(record, "value"), "pretty")), "-inf");
    assert_eq!(
        enum_field(record_field(record, "exactness"), "SolutionExactness"),
        "Exact"
    );

    let right = eval("x = symbol(\"x\"); limit_right(1/x, x, 0)");
    assert_eq!(right.exit, 0);
    let record = limit_record(&right);
    assert_eq!(
        enum_field(record_field(record, "status"), "LimitStatus"),
        "PlusInfinity"
    );
    assert_eq!(text(get(record_field(record, "value"), "pretty")), "inf");
    assert_eq!(
        enum_field(record_field(record, "exactness"), "SolutionExactness"),
        "Exact"
    );
}

#[test]
fn limit_success_publishes_value_and_exactness() {
    let r = eval("x = symbol(\"x\"); limit(sin(x)/x, x, 0)");
    assert_eq!(r.exit, 0);
    let record = limit_record(&r);
    assert_eq!(
        enum_field(record_field(record, "status"), "LimitStatus"),
        "Value"
    );
    let value = record_field(record, "value");
    assert_eq!(text(get(value, "pretty")), "1");
    assert_eq!(text(get(value, "domain")), "rational");
    assert!(boolean(get(value, "exact")));
    assert_eq!(
        enum_field(record_field(record, "exactness"), "SolutionExactness"),
        "Exact"
    );
    assert!(array(get(record_field(record, "diagnostics"), "elements")).is_empty());
    assert!(array(get(record_field(record, "left_conditions"), "elements")).is_empty());
    assert!(array(get(record_field(record, "right_conditions"), "elements")).is_empty());
}

#[test]
fn limit_unevaluated_carries_diagnostic_code_and_category() {
    let rows = [
        (
            "limit(sin(x), x, inf)",
            "coefficient does not evaluate at the point",
        ),
        (
            "limit_left(sin(x), x, inf)",
            "coefficient does not evaluate at the point",
        ),
        (
            "limit_right(sin(x), x, inf)",
            "coefficient does not evaluate at the point",
        ),
    ];
    for (call, message) in rows {
        let r = eval(&format!("a = symbol(\"a\"); x = symbol(\"x\"); {call}"));
        assert_eq!(r.exit, 0, "{call}");
        assert!(boolean(get(&r.env, "ok")));
        let record = limit_record(&r);
        assert_eq!(
            enum_field(record_field(record, "status"), "LimitStatus"),
            "Unevaluated"
        );
        assert_eq!(text(get(record_field(record, "exists"), "kind")), "Null");
        let diagnostics = array(get(record_field(record, "diagnostics"), "elements"));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(text(get(&diagnostics[0], "type")), "Diagnostic");
        let d = &diagnostics[0];
        assert_eq!(
            text(get(record_field(d, "code"), "value")),
            "limit.unevaluated"
        );
        assert_eq!(
            enum_field(record_field(d, "category"), "ErrorCategory"),
            "UnsupportedOperation"
        );
        assert_eq!(text(get(record_field(d, "message"), "value")), message);
        assert_eq!(text(get(record_field(d, "recoverable"), "value")), "true");
    }
}

// ---------------------------------------------------------------------------
// DivRemRecordTests
// ---------------------------------------------------------------------------

fn divrem(a: &str, b: &str) -> RunResult {
    eval(&format!("A = {a}; B = {b}; divrem(A, B)"))
}

fn divrem_fields(r: &RunResult) -> (&Json, &Json) {
    let structured = get(get(&r.env, "result"), "structured");
    assert_eq!(text(get(structured, "kind")), "Record");
    assert_eq!(text(get(structured, "type")), "DivRemResult");
    let names: Vec<&str> = array(get(structured, "fields"))
        .iter()
        .map(|f| text(get(f, "name")))
        .collect();
    assert_eq!(names, vec!["quotient", "remainder"]);
    (
        record_field(structured, "quotient"),
        record_field(structured, "remainder"),
    )
}

#[test]
fn divrem_identity_cases() {
    let rows = [
        ("7", "2", "3", "1"),
        (
            "2^1000",
            "3",
            "3571695357287557736494750163533339368538016039018445358145834627901170170416453741643994596052319527091982243058510489417290484285641046811994859191566191601311522591608076995140358201687457047292651394051015491661193980422466255853055181315359020971523732159228847389220143277217541462279068556023125",
            "1",
        ),
        (
            "10^100",
            "7",
            "1428571428571428571428571428571428571428571428571428571428571428571428571428571428571428571428571428",
            "4",
        ),
        ("-17", "5", "-3", "-2"),
    ];
    for (a, b, q, rem) in rows {
        let r = divrem(a, b);
        assert_eq!(r.exit, 0, "divrem({a}, {b})");
        let (quotient, remainder) = divrem_fields(&r);
        let qk = text(get(quotient, "kind"));
        let rk = text(get(remainder, "kind"));
        assert!(qk == "Natural" || qk == "Integer");
        assert!(rk == "Natural" || rk == "Integer");
        assert_eq!(text(get(quotient, "value")), q, "divrem({a}, {b}) quotient");
        assert_eq!(
            text(get(remainder, "value")),
            rem,
            "divrem({a}, {b}) remainder"
        );
    }
}

#[test]
fn divrem_negative_operand_is_integer_on_both_fields() {
    let r = divrem("-17", "5");
    let (quotient, remainder) = divrem_fields(&r);
    assert_eq!(text(get(quotient, "kind")), "Integer");
    assert_eq!(text(get(remainder, "kind")), "Integer");
    assert_eq!(text(get(quotient, "value")), "-3");
    assert_eq!(text(get(remainder, "value")), "-2");
}

#[test]
fn divrem_keeps_documented_sign_rule() {
    for (a, b, q, rem) in [("-17", "5", "-3", "-2"), ("1", "5", "0", "1")] {
        let r = divrem(a, b);
        let (quotient, remainder) = divrem_fields(&r);
        assert_eq!(text(get(quotient, "value")), q);
        assert_eq!(text(get(remainder, "value")), rem);
    }
}

#[test]
fn divrem_by_zero_is_recoverable_domain_error() {
    let r = eval("divrem(7, 0)");
    assert_eq!(r.exit, 1);
    assert!(!boolean(get(&r.env, "ok")));
    assert_eq!(text(get(&r.env, "code")), "DivisionByZero");
    assert_eq!(text(get(&r.env, "category")), "DomainError");
    assert!(boolean(get(&r.env, "recoverable")));
}

// ---------------------------------------------------------------------------
// VariableStructuredProjectionTests
// ---------------------------------------------------------------------------

const EVERY_KIND: &str = "x = symbol(\"x\"); r = 1/3; q = sqrt(2); z = dft([1, 2, 3])[1]; \
b = is_even(4); t = \"hi\"; v = [1, 2]; m = [[1, 2], [3, 4]]; d = real; n = 42; k = -7; \
g = solve_full(x^2 - 4 == 0, x); s = g.status; i = inspect(x)";

fn every_kind(extra: &[&str]) -> RunResult {
    let mut args = vec!["--eval", EVERY_KIND, "--omit-functions"];
    args.extend_from_slice(extra);
    run_args(&args)
}

fn variable<'a>(r: &'a RunResult, name: &str) -> &'a Json {
    array(get(&r.env, "variables"))
        .iter()
        .find(|v| text(get(v, "name")) == name)
        .unwrap_or_else(|| panic!("no variable {name}"))
}

#[test]
fn variable_every_kind_has_a_structured_form() {
    let r = every_kind(&[]);
    assert_eq!(r.exit, 0);
    let documented = [
        "Natural", "Integer", "Real", "Complex", "Boolean", "Text", "Array", "Symbolic", "Record",
        "Domain", "Enum", "Function", "Null",
    ];
    for variable in array(get(&r.env, "variables")) {
        let structured = opt(variable, "structured").expect("variable has structured form");
        let kind = text(get(structured, "kind"));
        assert!(documented.contains(&kind), "undocumented kind {kind}");
        assert_ne!(kind, "Null");
        assert!(!text(get(variable, "display")).is_empty());
    }
    assert_eq!(
        text(get(get(variable(&r, "g"), "structured"), "kind")),
        "Record"
    );
    assert_eq!(
        text(get(get(variable(&r, "m"), "structured"), "kind")),
        "Array"
    );
    assert_eq!(
        text(get(get(variable(&r, "z"), "structured"), "kind")),
        "Complex"
    );
    assert_eq!(
        text(get(get(variable(&r, "r"), "structured"), "kind")),
        "Real"
    );
}

#[test]
fn variable_structured_forms_carry_meaning() {
    let r = every_kind(&[]);
    let x = get(variable(&r, "x"), "structured");
    assert_eq!(text(get(x, "kind")), "Symbolic");
    assert_eq!(text(get(x, "canonical")), "(sym x)");
    assert_eq!(text(get(x, "pretty")), "x");
    assert_eq!(text(get(x, "domain")), "complex");
    assert!(boolean(get(x, "exact")));
    assert_eq!(number(get(x, "nodeCount")), 1);
    assert_eq!(
        array(get(x, "freeSymbols"))
            .iter()
            .map(text)
            .collect::<Vec<_>>(),
        vec!["x"]
    );

    let third = get(variable(&r, "r"), "structured");
    assert_eq!(text(get(third, "kind")), "Real");
    assert!(boolean(get(third, "exact")));
    assert_eq!(text(get(third, "numerator")), "1");
    assert_eq!(text(get(third, "denominator")), "3");

    let sqrt_two = get(variable(&r, "q"), "structured");
    assert!(!boolean(get(sqrt_two, "exact")));
    assert!(opt(sqrt_two, "numerator").is_none());

    let complex = get(variable(&r, "z"), "structured");
    assert_eq!(text(get(complex, "kind")), "Complex");
    assert!(opt(complex, "re").is_some());
    assert!(opt(complex, "im").is_some());
    assert!(!boolean(get(complex, "exact")));

    let matrix = get(variable(&r, "m"), "structured");
    assert_eq!(text(get(matrix, "type")), "Array");
    assert_eq!(
        array(get(matrix, "shape"))
            .iter()
            .map(number)
            .collect::<Vec<_>>(),
        vec![2, 2]
    );
    assert_eq!(array(get(matrix, "elements")).len(), 4);
    let vector = get(variable(&r, "v"), "structured");
    assert_eq!(text(get(vector, "type")), "Vector");

    assert_eq!(
        text(get(get(variable(&r, "n"), "structured"), "value")),
        "42"
    );
    assert_eq!(
        text(get(get(variable(&r, "k"), "structured"), "value")),
        "-7"
    );
    assert_eq!(
        text(get(get(variable(&r, "b"), "structured"), "value")),
        "true"
    );
    assert_eq!(
        text(get(get(variable(&r, "t"), "structured"), "value")),
        "hi"
    );
    assert_eq!(
        text(get(get(variable(&r, "d"), "structured"), "domain")),
        "real"
    );

    let status = get(variable(&r, "s"), "structured");
    assert_eq!(text(get(status, "kind")), "Enum");
    assert_eq!(text(get(status, "type")), "SolveStatus");
    assert_eq!(text(get(status, "value")), "Solved");

    let record = get(variable(&r, "g"), "structured");
    assert_eq!(text(get(record, "kind")), "Record");
    assert_eq!(text(get(record, "type")), "SolveResult");
    assert_eq!(text(get(record_field(record, "status"), "value")), "Solved");
}

#[test]
fn variable_result_carries_the_same_structure_as_result() {
    let r = every_kind(&[]);
    let result = get(get(&r.env, "result"), "structured");
    assert_eq!(text(get(result, "type")), "Inspection");
    assert_eq!(result, get(variable(&r, "_"), "structured"));
}

#[test]
fn variable_omit_still_emits_no_entries() {
    let r = every_kind(&["--omit-variables"]);
    assert!(array(get(&r.env, "variables")).is_empty());
}

// ---------------------------------------------------------------------------
// StructuredRealPrecisionTests
// ---------------------------------------------------------------------------

fn real_result(r: &RunResult) -> &Json {
    let structured = get(get(&r.env, "result"), "structured");
    assert_eq!(text(get(structured, "kind")), "Real");
    structured
}

#[test]
fn real_requested_precision_crosses_every_stored_digit() {
    let r = eval_keep_vars("setprecision(1100); pi(1100)");
    let structured = real_result(&r);
    let value = text(get(structured, "value"));
    assert_eq!(decimals(value), 1100);
    assert!(value.starts_with("3.141592653589793238462643383279502884197169399375105820974944"));
    assert!(value.ends_with("24972177528347913151"));
    assert!(!boolean(get(structured, "exact")));
    assert!(opt(structured, "numerator").is_none());
    assert!(opt(structured, "truncated").is_none());
    let variable = array(get(&r.env, "variables"))
        .iter()
        .find(|v| text(get(v, "name")) == "_")
        .expect("result variable");
    assert_eq!(text(get(get(variable, "structured"), "value")), value);
}

#[test]
fn real_counts_at_or_below_request_are_carried_exactly() {
    for (script, digits) in [("pi(30)", 30), ("setprecision(1100); sqrt(2)", 1100)] {
        let r = eval_keep_vars(script);
        let structured = real_result(&r);
        assert_eq!(decimals(text(get(structured, "value"))), digits, "{script}");
        let variable = array(get(&r.env, "variables"))
            .iter()
            .find(|v| text(get(v, "name")) == "_")
            .expect("result variable");
        assert_eq!(
            decimals(text(get(get(variable, "structured"), "value"))),
            digits,
            "{script} variable"
        );
    }
}

#[test]
fn real_stored_digits_are_not_padded() {
    let r = envelope("evalf(sqrt(2), 5)", &[]);
    assert_eq!(text(get(real_result(&r), "value")), "1.41421");
}

#[test]
fn real_results_display_follows_script_precision() {
    let r = eval_keep_vars("setprecision(1100); pi(1100)");
    let result = get(&r.env, "result");
    assert_eq!(decimals(text(get(result, "display"))), 1100);
    assert_eq!(decimals(text(get(result, "typed"))), 1100);
    let variable = array(get(&r.env, "variables"))
        .iter()
        .find(|v| text(get(v, "name")) == "_")
        .expect("result variable");
    assert_eq!(decimals(text(get(variable, "display"))), 1100);
}

// ---------------------------------------------------------------------------
// ArityPropertyTests
// ---------------------------------------------------------------------------

#[test]
fn arity_documented_call_forms_stay_callable() {
    let scripts = [
        "pi()",
        "zeros(2, 2)",
        "ones(3)",
        "eye(3)",
        "transpose([[1, 2], [3, 4]])",
        "sum([1, 2, 3])",
        "prod([1, 2, 3])",
        "min([3, 1, 2])",
        "max([3, 1, 2])",
        "mean([1, 2, 3])",
        "norm([3, 4])",
        "reshape([1, 2, 3, 4], 2, 2)",
        "concat([1, 2], [3, 4])",
        "print(\"A\", \"B\")",
        "plot([1, 2, 3])",
        "symbol(\"x\")",
        "x = symbol(\"x\"); solve(x^2 - 4 == 0, x)",
        "x = symbol(\"x\"); solve_full(x^2 - 4 == 0, x)",
        "dft([0, 1, 0, 0])",
    ];
    for script in scripts {
        let r = run_args(&[
            "--eval",
            script,
            "--omit-functions",
            "--omit-variables",
            "--plot-dir",
            &plot_dir(),
        ]);
        assert_eq!(r.exit, 0, "'{script}' exited {}", r.exit);
        assert!(boolean(get(&r.env, "ok")), "'{script}' must still evaluate");
    }
}

// ---------------------------------------------------------------------------
// FunctionArityMetadataTests
// ---------------------------------------------------------------------------

#[test]
fn functions_publish_their_declared_arity() {
    let r = run_args(&["--eval", "x = symbol(\"x\"); x + 1"]);
    assert_eq!(r.exit, 0);
    let functions = array(get(&r.env, "functions"));
    assert_eq!(functions.len(), registry::BUILTINS.len());
    for entry in functions {
        let name = text(get(entry, "name"));
        let info =
            registry::find(name).unwrap_or_else(|| panic!("published unknown builtin {name}"));
        let params: Vec<&str> = array(get(entry, "parameters")).iter().map(text).collect();
        assert_eq!(params, info.parameters, "{name} parameters");
        assert_eq!(
            number(get(entry, "parameterCount")),
            info.parameter_count as i64,
            "{name}"
        );
        assert_eq!(
            number(get(entry, "minArity")),
            info.min_arity as i64,
            "{name}"
        );
        assert_eq!(boolean(get(entry, "variadic")), info.variadic, "{name}");
        let min = number(get(entry, "minArity"));
        assert!(
            (0..=number(get(entry, "parameterCount"))).contains(&min),
            "{name}"
        );
    }
}

#[test]
fn functions_show_optional_and_variadic_parameters() {
    let r = run_args(&["--eval", "x = symbol(\"x\"); x + 1"]);
    let functions = array(get(&r.env, "functions"));
    let find = |name: &str| {
        functions
            .iter()
            .find(|f| text(get(f, "name")) == name)
            .unwrap_or_else(|| panic!("no {name}"))
    };
    for (name, min, count, variadic) in [
        ("solve", 2, 3, false),
        ("symbol", 1, 2, false),
        ("concat", 2, 3, false),
        ("dft", 1, 2, false),
        ("abs", 1, 1, false),
        ("print", 0, 1, true),
        ("zeros", 1, 1, true),
    ] {
        let entry = find(name);
        assert_eq!(number(get(entry, "minArity")), min, "{name}");
        assert_eq!(number(get(entry, "parameterCount")), count, "{name}");
        assert_eq!(boolean(get(entry, "variadic")), variadic, "{name}");
    }
    assert!(functions.iter().any(|f| boolean(get(f, "variadic"))));
    assert!(
        functions
            .iter()
            .any(|f| number(get(f, "minArity")) < number(get(f, "parameterCount")))
    );
}

// ---------------------------------------------------------------------------
// InspectionExactnessTests
// ---------------------------------------------------------------------------

#[test]
fn inspect_exactness_is_a_boolean_for_natural_and_wide_quotient() {
    for (source, expected) in [("1", "true"), ("1/(10^19)", "true")] {
        let env = eval(&format!("v = {source}; i = inspect(v)")).env;
        let record = get(get(&env, "result"), "structured");
        let exact = record_field(record, "exact");
        assert_eq!(text(get(exact, "kind")), "Boolean");
        assert_eq!(text(get(exact, "value")), expected);
    }
}

// ---------------------------------------------------------------------------
// CommittedStateTests
// ---------------------------------------------------------------------------

#[test]
fn committed_failure_before_any_commit_is_empty() {
    let r = eval("det(1)");
    assert!(opt(&r.env, "partialVariables").is_some());
    assert!(array(get(&r.env, "partialVariables")).is_empty());
}

#[test]
fn committed_parse_error_commits_nothing() {
    let r = eval("x = 42\nx = (");
    assert_eq!(text(get(&r.env, "code")), "ParseError");
    assert!(array(get(&r.env, "partialVariables")).is_empty());
}

#[test]
fn committed_artifact_written_before_failure_is_published() {
    let dir = format!("{}/committed-artifact", plot_dir());
    let _ = std::fs::create_dir_all(&dir);
    let r = run_args(&[
        "--eval",
        "plot([1, 2, 3])\ndet(1)",
        "--omit-functions",
        "--plot-dir",
        &dir,
    ]);
    assert_eq!(r.exit, 1);
    let artifacts = array(get(&r.env, "artifacts"));
    assert_eq!(artifacts.len(), 1);
    let artifact = &artifacts[0];
    assert_eq!(text(get(artifact, "kind")), "plot");
    let path = text(get(artifact, "path"));
    assert!(
        std::path::Path::new(path).exists(),
        "artifact path must exist: {path}"
    );
    assert_eq!(
        number(get(artifact, "bytes")),
        std::fs::metadata(path).unwrap().len() as i64
    );
    assert!(opt(&r.env, "plot").is_none());
}

#[test]
fn committed_run_that_wrote_nothing_has_empty_artifacts() {
    let r = eval("1 + 1");
    assert!(boolean(get(&r.env, "ok")));
    assert!(array(get(&r.env, "artifacts")).is_empty());
}

#[test]
fn committed_successful_plot_publishes_the_same_artifact() {
    let dir = format!("{}/committed-plot", plot_dir());
    let _ = std::fs::create_dir_all(&dir);
    let r = run_args(&[
        "--eval",
        "plot([1, 2, 3])",
        "--omit-functions",
        "--plot-dir",
        &dir,
    ]);
    assert_eq!(r.exit, 0);
    let artifacts = array(get(&r.env, "artifacts"));
    assert_eq!(artifacts.len(), 1);
    let artifact = &artifacts[0];
    assert_eq!(text(get(artifact, "kind")), "plot");
    assert_eq!(
        text(get(artifact, "path")),
        text(get(get(&r.env, "plot"), "path"))
    );
}

// ---------------------------------------------------------------------------
// SourcePositionTests
// ---------------------------------------------------------------------------

fn diagnostic_position(env: &Json) -> (i64, i64, i64) {
    let d = &array(get(env, "diagnostics"))[0];
    (
        number(get(d, "position")),
        number(get(d, "line")),
        number(get(d, "column")),
    )
}

fn timing_positions(env: &Json) -> Vec<i64> {
    array(get(env, "timings"))
        .iter()
        .map(|t| number(get(t, "position")))
        .collect()
}

#[test]
fn positions_parse_failure_lf() {
    let r = eval("1+1;\n2+2;\n)");
    assert_eq!(r.exit, 1);
    assert_eq!(diagnostic_position(&r.env), (10, 3, 1));
    assert!(timing_positions(&r.env).is_empty());
}

#[test]
fn positions_parse_failure_crlf_file() {
    let path = format!("{}/positions-crlf.ls", plot_dir());
    std::fs::write(&path, b"1+1;\r\n2+2;\r\n)").unwrap();
    let r = run_args(&["--file", &path, "--omit-functions"]);
    assert_eq!(r.exit, 1);
    assert_eq!(diagnostic_position(&r.env), (12, 3, 1));
    assert!(timing_positions(&r.env).is_empty());
}

#[test]
fn positions_timings_are_caller_offsets_crlf() {
    let r = eval("1+1;\r\n2+2;\r\n3+3");
    assert_eq!(r.exit, 0);
    assert_eq!(timing_positions(&r.env), vec![0, 6, 12]);
}

#[test]
fn positions_timings_are_caller_offsets_lf_and_cr() {
    for source in ["1+1;\n2+2;\n3+3", "1+1;\r2+2;\r3+3"] {
        let r = eval(source);
        assert_eq!(r.exit, 0);
        assert_eq!(timing_positions(&r.env), vec![0, 5, 10]);
    }
}

#[test]
fn positions_parse_failure_cr_only() {
    let r = eval("1+1;\r2+2;\r)");
    assert_eq!(r.exit, 1);
    assert_eq!(diagnostic_position(&r.env), (10, 3, 1));
}

#[test]
fn positions_failure_in_second_statement_names_it() {
    let r = eval("1+1; det(1)");
    assert_eq!(r.exit, 1);
    assert_eq!(diagnostic_position(&r.env), (5, 1, 6));
}

// ---------------------------------------------------------------------------
// EnvelopeDurationTests
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// LimitExistenceTriStateTests
// ---------------------------------------------------------------------------

#[test]
fn limit_exists_is_three_valued() {
    let undetermined = eval("x = symbol(\"x\"); limit_full(x*sin(1/x), x, 0)");
    let record = limit_record(&undetermined);
    assert_eq!(
        enum_field(record_field(record, "status"), "LimitStatus"),
        "Unevaluated"
    );
    assert_eq!(text(get(record_field(record, "exists"), "kind")), "Null");
    assert_eq!(text(get(record_field(record, "left"), "kind")), "Null");
    assert_eq!(text(get(record_field(record, "right"), "kind")), "Null");
    for name in ["left_conditions", "right_conditions"] {
        let field = record_field(record, name);
        assert!(text(get(field, "kind")) == "Array" || text(get(field, "kind")) == "Vector");
        assert!(array(get(field, "elements")).is_empty());
    }

    let determined = eval("x = symbol(\"x\"); limit_full(sin(x)/x, x, 0)");
    let record = limit_record(&determined);
    assert_eq!(
        enum_field(record_field(record, "status"), "LimitStatus"),
        "Value"
    );
    assert_eq!(text(get(record_field(record, "exists"), "kind")), "Boolean");
    assert_eq!(text(get(record_field(record, "exists"), "value")), "true");
    assert_eq!(text(get(record_field(record, "value"), "pretty")), "1");

    let missing = eval("x = symbol(\"x\"); limit_full(1/x, x, 0)");
    let record = limit_record(&missing);
    assert_eq!(
        enum_field(record_field(record, "status"), "LimitStatus"),
        "DoesNotExist"
    );
    assert_eq!(text(get(record_field(record, "exists"), "kind")), "Boolean");
    assert_eq!(text(get(record_field(record, "exists"), "value")), "false");
    assert_eq!(text(get(record_field(record, "left"), "pretty")), "-inf");
    assert_eq!(text(get(record_field(record, "right"), "pretty")), "inf");

    let left = eval("x = symbol(\"x\"); limit_left(x*sin(1/x), x, 0)");
    let record = limit_record(&left);
    assert_eq!(
        enum_field(record_field(record, "status"), "LimitStatus"),
        "Unevaluated"
    );
    assert_eq!(text(get(record_field(record, "value"), "kind")), "Null");

    let right = eval("x = symbol(\"x\"); limit_right(x*sin(1/x), x, 0)");
    let record = limit_record(&right);
    assert_eq!(
        enum_field(record_field(record, "status"), "LimitStatus"),
        "Unevaluated"
    );
    assert_eq!(text(get(record_field(record, "value"), "kind")), "Null");

    let left_pole = eval("x = symbol(\"x\"); limit_left(1/x, x, 0)");
    let record = limit_record(&left_pole);
    assert_eq!(
        enum_field(record_field(record, "status"), "LimitStatus"),
        "MinusInfinity"
    );
    assert_eq!(text(get(record_field(record, "exists"), "value")), "true");
    assert_eq!(text(get(record_field(record, "value"), "pretty")), "-inf");

    let right_pole = eval("x = symbol(\"x\"); limit_right(1/x, x, 0)");
    let record = limit_record(&right_pole);
    assert_eq!(
        enum_field(record_field(record, "status"), "LimitStatus"),
        "PlusInfinity"
    );
    assert_eq!(text(get(record_field(record, "exists"), "value")), "true");
    assert_eq!(text(get(record_field(record, "value"), "pretty")), "inf");
}

// ---------------------------------------------------------------------------
// BuiltinArgumentShapeSweepTests — the declared call forms stay callable.
// ---------------------------------------------------------------------------

#[test]
fn builtin_shape_declared_forms_stay_callable() {
    let scripts = [
        "det([[1, 2], [3, 4]])",
        "matmul([[1, 2], [3, 4]], [[5], [6]])",
        "transpose([[1, 2], [3, 4]])",
        "trace([[1, 2], [3, 4]])",
        "shape([[1, 2], [3, 4]])",
        "symbol(\"x\")",
        "x = symbol(\"x\"); assume(x > 5)",
        "x = symbol(\"x\"); assume_positive(x)",
    ];
    for script in scripts {
        let r = run_args(&[
            "--eval",
            script,
            "--omit-functions",
            "--omit-variables",
            "--plot-dir",
            &plot_dir(),
        ]);
        assert_eq!(r.exit, 0, "'{script}' exited {}", r.exit);
        assert!(boolean(get(&r.env, "ok")), "'{script}' must evaluate");
    }
}

// ---------------------------------------------------------------------------
// BuiltinSurfaceContractTests — the parts whose fields Rust publishes unchanged.
// ---------------------------------------------------------------------------

#[test]
fn builtin_solve_short_form_publishes_the_same_record_as_full() {
    let short = eval("x = symbol(\"x\"); solve(x^2 - 4 == 0, x)");
    let full = eval("x = symbol(\"x\"); solve_full(x^2 - 4 == 0, x)");
    assert_eq!(short.exit, 0);
    assert_eq!(full.exit, 0);
    let a = get(get(&short.env, "result"), "structured");
    let b = get(get(&full.env, "result"), "structured");
    assert_eq!(text(get(a, "kind")), "Record");
    assert_eq!(text(get(a, "type")), "SolveResult");
    assert_eq!(a, b);
    assert_eq!(text(get(record_field(a, "status"), "value")), "Solved");
    assert_eq!(
        number(&array(get(record_field(a, "solutions"), "shape"))[0]),
        2
    );
    let values: Vec<&str> = array(get(record_field(a, "solutions"), "elements"))
        .iter()
        .map(|s| text(get(record_field(s, "value"), "canonical")))
        .collect();
    assert_eq!(values, vec!["(rat -2 1)", "(rat 2 1)"]);
    let family_env = eval("x = symbol(\"x\"); solve(sin(x) == 0, x)");
    let family = &array(get(
        record_field(
            get(get(&family_env.env, "result"), "structured"),
            "families",
        ),
        "elements",
    ))[0];
    assert_eq!(text(get(family, "type")), "SolutionFamily");
    assert_eq!(
        text(get(record_field(family, "template"), "canonical")),
        "(mul (sym k) (pi))"
    );
    assert_eq!(
        text(get(record_field(family, "parameter_domain"), "domain")),
        "integer"
    );
}

#[test]
fn builtin_protocol_document_examples_match_the_binary() {
    let doc = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/symbolics/dsh-protocol.md"
    ))
    .expect("docs/symbolics/dsh-protocol.md");
    let env = eval("x = symbol(\"x\"); x^2 + 1");
    let canonical = text(get(get(get(&env.env, "result"), "structured"), "canonical"));
    assert!(doc.contains(&format!("\"canonical\": \"{canonical}\"")));
    let error = eval("x = symbol(\"x\"); solve(x^2 - 2 == 0, x, integer)");
    assert_eq!(error.exit, 1);
    assert!(!boolean(get(&error.env, "ok")));
    for key in ["elapsed", "elapsedTime", "timings"] {
        assert!(
            opt(&error.env, key).is_some(),
            "live error envelope missing {key}"
        );
    }
}
