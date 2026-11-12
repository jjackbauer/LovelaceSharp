//! Round 14: the language/engine surface behind the refused Suite cases.
//!
//! Every expectation here was observed on the pinned C# oracle first (Lovelace.Run at
//! 1cc28f255eca6029849f880949fc7be1d40ab8af) and then asserted against this crate. The tests name
//! the capability, not the census row, because the production change is the deliverable: the
//! surface (parser, builtin dispatch, arithmetic rules, structured payload) is shared by every row
//! the surface unblocks.

use crate::syntax::{Expr, IndexArg, Parser, tokenize};
use crate::{Engine, LangError, Value};
use lovelace_json::Json;

fn eval(script: &str) -> (Engine, Value) {
    let mut engine = Engine::new();
    match engine.eval(script) {
        Ok(value) => (engine, value),
        Err(err) => panic!("script: {script} unexpectedly failed: {}", err.message),
    }
}

fn typed(script: &str) -> String {
    let (engine, value) = eval(script);
    engine.format_typed(&value)
}

fn display(script: &str) -> String {
    let (engine, value) = eval(script);
    engine.format(&value)
}

fn error(script: &str) -> LangError {
    let mut engine = Engine::new();
    match engine.eval(script) {
        Ok(value) => panic!(
            "script: {script} unexpectedly answered {}",
            engine.format_typed(&value)
        ),
        Err(err) => err,
    }
}

fn structured(script: &str) -> Json {
    let (engine, value) = eval(script);
    crate::structured::project_value(&engine, &value, None)
}

fn json_bool(j: &Json, key: &str) -> Option<bool> {
    match j.get(key) {
        Some(Json::Bool(b)) => Some(*b),
        _ => None,
    }
}

fn json_str<'a>(j: &'a Json, key: &str) -> Option<&'a str> {
    j.get(key).and_then(|v| v.as_str())
}

fn elements(j: &Json, script: &str) -> Vec<String> {
    let list = j
        .get("elements")
        .and_then(|v| v.as_arr())
        .unwrap_or_else(|| panic!("script: {script} published no elements"));
    list.iter()
        .map(|el| {
            format!(
                "{}={}",
                json_str(el, "kind").unwrap_or("?"),
                json_str(el, "value").unwrap_or("")
            )
        })
        .collect()
}

fn number(value: &Value) -> String {
    match value {
        Value::Natural(n) => n.to_decimal(),
        Value::Integer(i) => i.to_decimal(),
        other => panic!("not an integer value: {other:?}"),
    }
}

fn record_field<'a>(value: &'a Value, field: &str) -> &'a Value {
    match value {
        Value::Record(_, fields) => fields
            .iter()
            .find(|(name, _)| name == field)
            .map(|(_, v)| v)
            .unwrap_or_else(|| panic!("record has no field {field}")),
        other => panic!("not a record: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Unary plus (SignPrecedenceTests + InterpreterUnaryTests)
// ---------------------------------------------------------------------------

#[test]
fn unary_plus_is_the_identity_and_binds_looser_than_power() {
    assert_eq!(typed("+5"), "5 (Natural)");
    assert_eq!(typed("+2.71"), "2.71 (Real)");
    assert_eq!(typed("++1"), "1 (Natural)");
    assert_eq!(typed("+ -1"), "-1 (Integer)");
    assert_eq!(typed("+2^2"), "4 (Natural)");
    assert_eq!(typed("+2^3^2"), "512 (Natural)");
    assert_eq!(typed("+1..2"), "[1, 2] (Vector)");
    // The AST shape is the reference's UnaryExpr(Plus, Power(2, 2)): the sign is a node,
    // and the power stays under it (no parse failure).
    let expr = {
        let tokens = tokenize("+2^2").expect("tokenize");
        let mut parser = Parser::new(tokens);
        parser.single_expression().expect("parse")
    };
    match expr {
        Expr::Plus(inner) => assert!(
            matches!(*inner, Expr::Binary(crate::syntax::BinOp::Pow, _, _)),
            "the power is not under the plus: {inner:?}"
        ),
        other => panic!("not a unary plus: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Slice syntax (SliceTests, STO-003)
// ---------------------------------------------------------------------------

#[test]
fn slice_syntax_matches_the_reference_forms() {
    assert_eq!(typed("[0, 1, 2, 3, 4][1:4]"), "[1, 2, 3] (Vector)");
    assert_eq!(typed("[0, 1, 2, 3, 4][::2]"), "[0, 2, 4] (Vector)");
    assert_eq!(typed("[[1, 2, 3], [4, 5, 6]][:, 1]"), "[2, 5] (Vector)");
    assert_eq!(typed("[[1, 2, 3], [4, 5, 6]][1:2]"), "[[4, 5, 6]] (Array)");
    assert_eq!(typed("[1, 2, 3, 4, 5][::2] * 2"), "[2, 6, 10] (Vector)");
    // The other optional-part spellings, all parsed by the same axis grammar.
    assert_eq!(typed("[0, 1, 2, 3, 4][::]"), "[0, 1, 2, 3, 4] (Vector)");
    assert_eq!(typed("[0, 1, 2, 3, 4][:]"), "[0, 1, 2, 3, 4] (Vector)");
    assert_eq!(typed("[0, 1, 2, 3, 4][1::2]"), "[1, 3] (Vector)");
    assert_eq!(typed("[0, 1, 2, 3, 4][:3:2]"), "[0, 2] (Vector)");
    assert_eq!(
        typed("[[1, 2, 3], [4, 5, 6]][:, ::2]"),
        "[[1, 3], [4, 6]] (Array)"
    );
    let expr = {
        let tokens = tokenize("[0,1,2,3,4][::2]").expect("tokenize");
        let mut parser = Parser::new(tokens);
        parser.single_expression().expect("parse")
    };
    assert!(
        matches!(expr, Expr::Index(_, ref args) if matches!(args[0], IndexArg::Slice(_, _, Some(_))))
    );
}

#[test]
fn slice_step_must_be_positive() {
    for (script, got) in [
        ("[0, 1, 2][::0]", "0"),
        ("[0, 1, 2][::-1]", "-1"),
        ("[0, 1, 2][0:2:0]", "0"),
    ] {
        let err = error(script);
        assert_eq!(
            err.message,
            format!(
                "range(): argument 'step' must be a Natural or Integer that is a positive count (1 or more, at most 9223372036854775807); got {got}."
            ),
            "script: {script}"
        );
        assert_eq!(
            (err.code, err.category),
            ("InvalidArgument", "TypeMismatch"),
            "script: {script}"
        );
    }
}

// ---------------------------------------------------------------------------
// Empty reductions (D6) — EmptyReductionTests
// ---------------------------------------------------------------------------

#[test]
fn empty_reductions_answer_the_monoid_identity_and_refuse_min_max() {
    assert_eq!(typed("sum([])"), "0 (Natural)");
    assert_eq!(typed("prod([])"), "1 (Natural)");
    assert_eq!(
        error("min([])").message,
        "min() cannot reduce an empty array."
    );
    assert_eq!(
        error("max([])").message,
        "max() cannot reduce an empty array."
    );
    assert_eq!(
        error("mean([])").message,
        "mean() cannot reduce an empty array."
    );
    assert_eq!(
        error("norm([])").message,
        "norm() cannot reduce an empty array."
    );
}

// ---------------------------------------------------------------------------
// evalf: integer payloads and the exactness flag (StructuredRealExactnessTests)
// ---------------------------------------------------------------------------

#[test]
fn evalf_publishes_the_exactness_the_digits_deserve() {
    let cut = structured("evalf(1/3, 50)");
    assert_eq!(json_str(&cut, "kind"), Some("Real"));
    assert_eq!(json_bool(&cut, "exact"), Some(false));
    assert!(cut.get("numerator").is_none());
    assert!(cut.get("denominator").is_none());
    assert_eq!(
        json_str(&cut, "value").map(str::len),
        Some(52),
        "50 fractional digits"
    );

    // Default plugin budget is 30 places: 29 digits still read as a finite exact literal,
    // 30 is the truncation residue (the oracle's own Real.Parse boundary).
    let exact_29 = structured("evalf(1/3, 29)");
    assert_eq!(json_bool(&exact_29, "exact"), Some(true));
    assert_eq!(
        json_str(&exact_29, "denominator"),
        Some("100000000000000000000000000000")
    );
    assert_eq!(
        json_bool(&structured("evalf(1/3, 30)"), "exact"),
        Some(false)
    );

    // setprecision lifts the plugin budget, so the same request is exact again.
    assert_eq!(
        json_bool(&structured("setprecision(1000); evalf(1/3, 50)"), "exact"),
        Some(true)
    );
    assert_eq!(
        json_bool(&structured("evalf(1/2, 40)"), "exact"),
        Some(true)
    );

    // Integer payloads stay integers; an exact Real with an integral value becomes Integer.
    assert_eq!(typed("evalf(2, 5)"), "2 (Integer)");
    assert_eq!(typed("evalf(4.0, 5)"), "4 (Integer)");

    // An already-inexact value keeps its inexactness and is NOT padded past its stored digits.
    let truncated = structured("evalf(pi(30), 40)");
    assert_eq!(json_bool(&truncated, "exact"), Some(false));
    assert_eq!(
        json_str(&truncated, "value"),
        Some("3.141592653589793238462643383279")
    );
    assert_eq!(
        json_bool(&structured("evalf(sqrt(2), 5)"), "exact"),
        Some(false)
    );
}

#[test]
fn evalf_arity_and_digit_count_are_guarded() {
    assert_eq!(
        error("evalf(1/3)").message,
        "evalf(): expected 2 arguments; got 1."
    );
    assert_eq!(
        error("evalf(1/3, 0)").message,
        "evalf(): argument 2 (digits) must be a Natural or Integer digit count between 1 and 2147483647; got 0."
    );
    assert_eq!(
        error("evalf(1/3, 3.0)").message,
        "evalf(): argument 2 (digits) must be a Natural or Integer digit count between 1 and 2147483647; got Real."
    );
}

// ---------------------------------------------------------------------------
// Hyperbolic builtins (HyperbolicBuiltinsAndNegativeIntegerPowersTests)
// ---------------------------------------------------------------------------

#[test]
fn inverse_hyperbolics_fold_at_their_defining_point() {
    assert_eq!(typed("asinh(0)"), "0 (Symbolic)");
    assert_eq!(typed("acosh(1)"), "0 (Symbolic)");
    assert_eq!(typed("atanh(0)"), "0 (Symbolic)");
    assert_eq!(typed("asinh(1)"), "asinh(1) (Symbolic)");
    assert_eq!(typed("acosh(2)"), "acosh(2) (Symbolic)");
    assert_eq!(typed("atanh(0.5)"), "atanh(1/2) (Symbolic)");
    for name in ["asinh", "acosh", "atanh"] {
        assert!(
            crate::registry::find(name).is_some(),
            "{name} is not registered"
        );
    }
}

// ---------------------------------------------------------------------------
// pi and e constants (TruncatedIrrational_IsNotCertifiedExact, descriptor examples)
// ---------------------------------------------------------------------------

#[test]
fn pi_and_e_constants_exist_and_render_at_display_precision() {
    let pi = display("pi");
    assert_eq!(pi.len(), 102);
    assert!(pi.starts_with("3.14159265358979323846264338327950288419716939937510"));
    assert_eq!(display("pi()"), pi);
    assert_eq!(typed("e(10)"), "2.7182818284 (Real)");
    let e = display("e");
    assert_eq!(e.len(), 102);
    assert!(e.starts_with("2.71828182845904523536028747135266249775724709369995"));

    // setprecision moves both knobs, so the constants follow.
    assert_eq!(display("setprecision(50); pi").len(), 52);
    assert_eq!(display("setprecision(50); e").len(), 52);

    // A user assignment shadows the constant.
    assert_eq!(typed("pi = 3; pi"), "3 (Natural)");

    // Both are inexact: their digits are a truncation, never a rational payload.
    for script in ["pi", "e"] {
        let j = structured(script);
        assert_eq!(json_bool(&j, "exact"), Some(false), "script: {script}");
        assert!(j.get("numerator").is_none(), "script: {script}");
    }
}

#[test]
fn pi_e_and_setprecision_digit_counts_are_guarded() {
    assert_eq!(
        error("setprecision(0)").message,
        "setprecision(): argument 1 (digits) must be a Natural or Integer that is a positive count (1 or more, at most 9223372036854775807); got 0."
    );
    assert_eq!(
        error("setprecision(-10)").message,
        "setprecision(): argument 1 (digits) must be a Natural or Integer that is a positive count (1 or more, at most 9223372036854775807); got -10."
    );
    assert_eq!(
        error("setprecision(3.0)").message,
        "setprecision(): argument 1 (digits) must be a Natural or Integer; got the Real 3, which is integral — write it as 3."
    );
    assert_eq!(
        error("pi(0)").message,
        "pi(): argument 1 (digits) must be a Natural or Integer between 1 and 1000; got 0."
    );
    assert_eq!(
        error("pi(1001)").message,
        "pi(): argument 1 (digits) must be a Natural or Integer between 1 and 1000; got 1001."
    );
    assert_eq!(
        error("e(0)").message,
        "e(): argument 1 (digits) must be a Natural or Integer between 1 and 1000; got 0."
    );
    assert_eq!(
        error("e(3.0)").message,
        "e(): argument 1 (digits) must be a Natural or Integer; got the Real 3, which is integral — write it as 3."
    );
    // The bound follows the engine precision setprecision raised.
    assert_eq!(display("setprecision(1100); pi(1100)").len(), 1102);
}

// ---------------------------------------------------------------------------
// Factorial: Integer path and the two refusal sentences (InterpreterFactorialTests)
// ---------------------------------------------------------------------------

#[test]
fn factorial_handles_integers_and_names_the_refusals() {
    assert_eq!(typed("5!"), "120 (Natural)");
    assert_eq!(typed("(-0)!"), "1 (Integer)");
    assert_eq!(typed("(-(-5))!"), "120 (Integer)");
    assert_eq!(typed("-5!"), "-120 (Integer)");
    assert_eq!(
        error("(-5)!").message,
        "Factorial is not defined for negative integers."
    );
    assert_eq!(
        error("3.14!").message,
        "Factorial is not supported for Real numbers."
    );
    assert_eq!(
        error("[1, 2]!").message,
        "Factorial is not supported for type 'Vector'."
    );
}

// ---------------------------------------------------------------------------
// Modulus: integer sign rule and the Real operator
// ---------------------------------------------------------------------------

#[test]
fn modulus_matches_the_reference_sign_and_real_rules() {
    assert_eq!(typed("5 % 3"), "2 (Natural)");
    assert_eq!(typed("5.0 % 3.0"), "2 (Real)");
    assert_eq!(typed("5.5 % 2"), "1.5 (Real)");
    assert_eq!(typed("-5.5 % 2"), "-1.5 (Real)");
    assert_eq!(typed("5.5 % -2"), "1.5 (Real)");
    assert_eq!(typed("-5.5 % -2"), "-1.5 (Real)");
    // The INTEGER remainder follows the QUOTIENT's sign, unlike C#'s dividend rule.
    assert_eq!(typed("-7 % -3"), "1 (Integer)");
    assert_eq!(typed("17 % -5"), "-2 (Integer)");
    assert_eq!(typed("-7 % 3"), "-1 (Integer)");
    assert_eq!(typed("7 % -3"), "-1 (Integer)");
    let zero = error("5 % 0");
    assert_eq!(zero.message, "Cannot divide by zero.");
    assert_eq!(
        (zero.code, zero.category),
        ("DivisionByZero", "DomainError")
    );
}

#[test]
fn divrem_publishes_the_reference_pair_and_kinds() {
    let (_, value) = eval("divrem(17, 5)");
    assert_eq!(number(record_field(&value, "quotient")), "3");
    assert_eq!(number(record_field(&value, "remainder")), "2");
    assert_eq!(record_field(&value, "quotient").kind_name(), "Natural");
    assert_eq!(record_field(&value, "remainder").kind_name(), "Natural");

    for (script, quotient, remainder) in [
        ("divrem(-17, 5)", "-3", "-2"),
        ("divrem(17, -5)", "-3", "-2"),
        ("divrem(-17, -5)", "3", "2"),
        ("divrem(-7, -3)", "2", "1"),
    ] {
        let (_, value) = eval(script);
        assert_eq!(
            number(record_field(&value, "quotient")),
            quotient,
            "script: {script}"
        );
        assert_eq!(
            number(record_field(&value, "remainder")),
            remainder,
            "script: {script}"
        );
        assert_eq!(
            record_field(&value, "quotient").kind_name(),
            "Integer",
            "script: {script}"
        );
    }

    assert_eq!(
        error("divrem(3.14, 1.5)").message,
        "divrem() is not supported for values of kind 'Real'. Use Natural or Integer operands."
    );
}

// ---------------------------------------------------------------------------
// Ranges: empty descending bounds, stepped Integer elements (StatementTests)
// ---------------------------------------------------------------------------

#[test]
fn ranges_are_inclusive_and_default_to_a_positive_step() {
    assert_eq!(typed("1..5"), "[1, 2, 3, 4, 5] (Vector)");
    assert_eq!(typed("1..0"), "[] (Vector)");
    assert_eq!(typed("5..1"), "[] (Vector)");
    assert_eq!(typed("len(1..0)"), "0 (Natural)");
    let stepped = structured("1..2..7");
    assert_eq!(
        elements(&stepped, "1..2..7"),
        vec![
            "Integer=1".to_string(),
            "Integer=3".to_string(),
            "Integer=5".to_string(),
            "Integer=7".to_string()
        ]
    );
    let descending = structured("5..-1..1");
    assert_eq!(
        elements(&descending, "5..-1..1"),
        vec![
            "Integer=5".to_string(),
            "Integer=4".to_string(),
            "Integer=3".to_string(),
            "Integer=2".to_string(),
            "Integer=1".to_string()
        ]
    );
    assert_eq!(typed("-2..2"), "[-2, -1, 0, 1, 2] (Vector)");
}

// ---------------------------------------------------------------------------
// Division narrowing (D2) — InterpreterBinaryArithmeticTests / TypedArrayPromotionTests
// ---------------------------------------------------------------------------

#[test]
fn scalar_integer_division_narrows_only_when_it_is_exact() {
    assert_eq!(typed("100 / 5"), "20 (Natural)");
    assert_eq!(typed("4 / 2"), "2 (Natural)");
    assert_eq!(typed("2 / 2"), "1 (Natural)");
    assert_eq!(typed("0 / 5"), "0 (Natural)");
    assert_eq!(typed("-6 / 2"), "-3 (Integer)");
    assert_eq!(typed("6 / -2"), "-3 (Integer)");
    assert_eq!(typed("-4 / -2"), "2 (Integer)");
    assert_eq!(typed("1 / 2"), "0.5 (Real)");
    assert_eq!(typed("1 / 4"), "0.25 (Real)");
    assert_eq!(typed("4.0 / 2"), "2 (Real)");
    assert_eq!(typed("1 / 3"), "0.(3) (Real)");
}

#[test]
fn array_division_narrows_homogeneously() {
    assert_eq!(
        elements(&structured("[4, 6] / [2, 2]"), "[4, 6] / [2, 2]"),
        vec!["Natural=2".to_string(), "Natural=3".to_string()]
    );
    assert_eq!(
        elements(&structured("[1, 2] / [2, 2]"), "[1, 2] / [2, 2]"),
        vec!["Real=0.5".to_string(), "Real=1".to_string()]
    );
    assert_eq!(
        elements(&structured("[2.5] / [0.25]"), "[2.5] / [0.25]"),
        vec!["Natural=10".to_string()]
    );
    assert_eq!(
        elements(&structured("[-4, -6] / [-2, -2]"), "[-4, -6] / [-2, -2]"),
        vec!["Natural=2".to_string(), "Natural=3".to_string()]
    );
    assert_eq!(
        elements(&structured("[-4] / [2]"), "[-4] / [2]"),
        vec!["Integer=-2".to_string()]
    );
    assert_eq!(
        elements(&structured("[4, 6] / 2"), "[4, 6] / 2"),
        vec!["Natural=2".to_string(), "Natural=3".to_string()]
    );
    assert_eq!(
        elements(&structured("[1, 2] / 2"), "[1, 2] / 2"),
        vec!["Real=0.5".to_string(), "Real=1".to_string()]
    );
}

// ---------------------------------------------------------------------------
// D3 mixed-literal promotion (TypedArrayPromotionTests)
// ---------------------------------------------------------------------------

#[test]
fn mixed_numeric_literals_promote_homogeneously() {
    assert_eq!(
        elements(&structured("[1, 2.5, 3]"), "[1, 2.5, 3]"),
        vec![
            "Real=1".to_string(),
            "Real=2.5".to_string(),
            "Real=3".to_string()
        ]
    );
    assert_eq!(
        elements(&structured("[[1, 2.5], [3, 4]]"), "[[1, 2.5], [3, 4]]"),
        vec![
            "Real=1".to_string(),
            "Real=2.5".to_string(),
            "Real=3".to_string(),
            "Real=4".to_string()
        ]
    );
    assert_eq!(
        elements(&structured("[1, -2]"), "[1, -2]"),
        vec!["Integer=1".to_string(), "Integer=-2".to_string()]
    );
    assert_eq!(
        elements(&structured("[1, 2, 3]"), "[1, 2, 3]"),
        vec![
            "Natural=1".to_string(),
            "Natural=2".to_string(),
            "Natural=3".to_string()
        ]
    );
    assert_eq!(
        elements(&structured("[1, 2, [3]]"), "[1, 2, [3]]"),
        vec![
            "Natural=1".to_string(),
            "Natural=2".to_string(),
            "Array=".to_string()
        ]
    );
}

// ---------------------------------------------------------------------------
// Arity grammar (already green on the pinned tree; pinned here so a regression is loud)
// ---------------------------------------------------------------------------

#[test]
fn arity_refusals_keep_the_reference_grammar() {
    assert_eq!(
        error("sqrt()").message,
        "sqrt(): expected 1 argument; got 0."
    );
    assert_eq!(
        error("sqrt(4, 9)").message,
        "sqrt(): expected 1 argument; got 2."
    );
    assert_eq!(
        error("pi(10, 20)").message,
        "pi(): expected 0 to 1 arguments; got 2."
    );
    assert_eq!(
        error("setprecision(10, 20)").message,
        "setprecision(): expected 1 argument; got 2."
    );
    for (script, code) in [
        ("sqrt()", "InvalidArgument"),
        ("pi(10, 20)", "InvalidArgument"),
    ] {
        assert_eq!(error(script).code, code, "script: {script}");
    }
}
