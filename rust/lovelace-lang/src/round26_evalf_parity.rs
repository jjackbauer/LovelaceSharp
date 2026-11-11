//! Round 26: counted parity for the `evalf` numeric-function rows.
//!
//! Every assertion is the C# one: the published digits, the requested decimal count, the exactness
//! flag and the absence of a rational form, read off the same structured projection the wire
//! publishes. Rows the current evaluator does not satisfy (the `pi(1)` shapes it cannot parse, the
//! `subs` fold that still crosses as exact, the symbolic-node agreement rows) are recorded in the
//! report as refusals instead of being weakened here.

use lovelace_json::Json;
use lovelace_real::Real;

use crate::{Engine, Value};

fn eval(script: &str) -> Value {
    Engine::new()
        .eval(script)
        .unwrap_or_else(|error| panic!("{script}: {}", error.message))
}

fn display(script: &str) -> String {
    eval(script).display()
}

fn structured(script: &str) -> Json {
    let mut engine = Engine::new();
    let value = engine
        .eval(script)
        .unwrap_or_else(|error| panic!("{script}: {}", error.message));
    crate::structured::project_value(&engine, &value, None)
}

fn field<'a>(json: &'a Json, key: &str) -> &'a Json {
    match json {
        Json::Obj(map) => map
            .get(key)
            .unwrap_or_else(|| panic!("missing {key} in {}", json.to_string())),
        other => panic!("not an object: {}", other.to_string()),
    }
}

fn has_field(json: &Json, key: &str) -> bool {
    match json {
        Json::Obj(map) => map.contains_key(key),
        other => panic!("not an object: {}", other.to_string()),
    }
}

fn text(json: &Json) -> &str {
    match json {
        Json::Str(value) => value,
        other => panic!("not a string: {}", other.to_string()),
    }
}

fn boolean(json: &Json) -> bool {
    match json {
        Json::Bool(value) => *value,
        other => panic!("not a boolean: {}", other.to_string()),
    }
}

fn decimals(rendering: &str) -> usize {
    match rendering.split_once('.') {
        Some((_, frac)) => frac.len(),
        None => 0,
    }
}

fn assert_inexact_without_rational(script: &str) {
    let projection = structured(script);
    assert_eq!(text(field(&projection, "kind")), "Real", "{script}");
    assert!(!boolean(field(&projection, "exact")), "{script} claims exactness");
    assert!(!has_field(&projection, "numerator"), "{script} publishes a numerator");
    assert!(!has_field(&projection, "denominator"), "{script} publishes a denominator");
    match eval(script) {
        Value::Real(value) => assert!(!value.is_exact(), "{script} carries an exact Real"),
        other => panic!("{script}: not a Real ({})", other.kind_name()),
    }
}

// ---------------------------------------------------------------------------
// EvalfWorkingPrecisionHonoursRequestedDecimalsTests (11 of 12 rows)
// ---------------------------------------------------------------------------

#[test]
fn evalf_sinh_working_precision_honours_the_requested_decimals() {
    // RequestedDecimalPlaces_AreTheDigitsDelivered rows 0..8: the value, the EXACT requested
    // decimal count (a prefix that stops short of the last decimal is not a pass), and inexactness.
    let cases: [(&str, usize, &str); 9] = [
        ("sinh(28/15)", 30, "3.156033243073207922905472664339"),
        (
            "sinh(28/15)",
            100,
            "3.1560332430732079229054726643398808098958171050319303402544132472926926925879526189618018339120690466",
        ),
        ("sinh(34/3)", 20, "41780.54805356406592518844"),
        ("sinh(34/3)", 30, "41780.548053564065925188446077567339"),
        (
            "sinh(34/3)",
            50,
            "41780.54805356406592518844607756733919267583492738622536",
        ),
        (
            "sinh(34/3)",
            100,
            "41780.5480535640659251884460775673391926758349273862253644277306454458985775018289223615326471730513352561",
        ),
        ("sinh(65/3)", 30, "1284351149.965741094255005311674965079318"),
        (
            "sinh(65/3)",
            100,
            "1284351149.9657410942550053116749650793184322151669392890793309161544689964010798237111274464943036288273900369",
        ),
        (
            "sinh(1000/3)",
            30,
            "2909358940723497999622983496672289621777576895087778819744496982533636420191216688947898956693539789328530352713870924771820635012087408585306867.359401747177724438543006142556",
        ),
    ];
    for (body, digits, expected) in cases {
        let script = format!("evalf({body}, {digits})");
        assert_eq!(display(&script), expected, "{script}");
        let projection = structured(&script);
        assert_eq!(text(field(&projection, "value")), expected, "{script}");
        assert_eq!(decimals(expected), digits, "{script}: delivered decimal count");
        assert!(!boolean(field(&projection, "exact")), "{script} claims exactness");
    }
}

#[test]
fn evalf_sinh_terminating_argument_keeps_its_digits() {
    // TerminatingArgument_KeepsDeliveringItsDigits rows 30 and 100: the representable argument is
    // unaffected by the working-precision repair, and the count is still the request.
    for (digits, expected) in [
        (30usize, "49357.885500315201914739778692889335"),
        (
            100,
            "49357.8855003152019147397786928893350025676060913315150701208567110467729030338740090072170635948405117304",
        ),
    ] {
        let script = format!("evalf(sinh(11.5), {digits})");
        assert_eq!(display(&script), expected, "{script}");
        let projection = structured(&script);
        assert_eq!(decimals(text(field(&projection, "value"))), digits);
        assert!(!boolean(field(&projection, "exact")));
    }
}

// ---------------------------------------------------------------------------
// EvalfDigitCountContractTests (5 rows) + the Run.Tests digits-one row
// ---------------------------------------------------------------------------

#[test]
fn evalf_digits_one_is_accepted_for_every_transcendental_shape() {
    // DigitsOne_IsAccepted_ForEveryShape rows sin/cos/exp/log/2*sin, and
    // Lovelace.Run.Tests.BuiltinSurfaceContractTests.Evalf_DigitsOne_IsTheSmallestAcceptedCount:
    // one is the smallest accepted count and every shape answers exactly one decimal place. The
    // zero count is the recoverable argument refusal, never an answer.
    for (body, expected) in [
        ("sin(1)", "0.8"),
        ("cos(1)", "0.5"),
        ("exp(1)", "2.7"),
        ("log(2)", "0.6"),
        ("2*sin(1)", "1.6"),
    ] {
        let script = format!("evalf({body}, 1)");
        assert_eq!(display(&script), expected, "{script}");
        let projection = structured(&script);
        assert_eq!(text(field(&projection, "kind")), "Real");
        assert_eq!(text(field(&projection, "value")), expected);
    }
    let error = Engine::new().eval("evalf(sin(1), 0)").expect_err("digits 0 must be refused");
    assert_eq!(error.code, "InvalidArgument");
    assert_eq!(error.category, "TypeMismatch");
}

// ---------------------------------------------------------------------------
// TruncatingEvaluationStaysInexactTests (9 + 11 + 1 rows)
// ---------------------------------------------------------------------------

#[test]
fn evalf_function_of_a_truncated_value_publishes_exact_false() {
    // FunctionOfATruncatedValue_PublishesExactFalse_AndNoRationalForm rows 2..10: every result is
    // a function of a trivially truncated value, so it must not claim exactness and must not
    // publish a rational form.
    for script in [
        "evalf(sin(pi(1)*1/4), 40)",
        "evalf(atan(1), 40)",
        "evalf(sin(pi(30)/6), 40)",
        "evalf(cos(pi(30)/6), 40)",
        "evalf(sin(pi(30)/4), 40)",
        "evalf(atan(pi(30)/4), 40)",
        "evalf(exp(pi(30)/10), 40)",
        "evalf(sin(pi(45)/6), 40)",
        "evalf(sin(pi(30)/6), 100)",
    ] {
        assert_inexact_without_rational(script);
    }
}

#[test]
fn evalf_elementary_function_of_an_exact_argument_stays_exact() {
    // ElementaryFunctionOfAnExactArgument_StaysExact, all eleven rows: the exact values are the
    // folded special points and they keep their exact tier.
    for (function, argument, expected) in [
        ("sin", "0", "0"),
        ("tan", "0", "0"),
        ("atan", "0", "0"),
        ("asin", "0", "0"),
        ("sinh", "0", "0"),
        ("tanh", "0", "0"),
        ("cos", "0", "1"),
        ("exp", "0", "1"),
        ("cosh", "0", "1"),
        ("acosh", "1", "0"),
        ("log", "1", "0"),
    ] {
        let script = format!("evalf({function}({argument}), 40)");
        let projection = structured(&script);
        assert_eq!(text(field(&projection, "kind")), "Integer", "{script}");
        assert!(boolean(field(&projection, "exact")), "{script} lost its exactness");
        assert_eq!(text(field(&projection, "value")), expected, "{script}");
    }
}

#[test]
fn evalf_truncated_value_that_is_a_rational_is_still_inexact() {
    // TruncatedValueThatHappensToBeARational_IsStillInexact: sin(pi(30)/6) is 0.4999...9, a finite
    // decimal and therefore a rational, one 10^-29 away from 1/2 — and still not exact.
    let script = "evalf(sin(pi(30)/6), 40)";
    let projection = structured(script);
    assert_eq!(text(field(&projection, "kind")), "Real");
    assert!(!boolean(field(&projection, "exact")), "{script} claims exactness");
    assert!(!has_field(&projection, "numerator"), "{script} publishes a rational form");
    let value = match eval(script) {
        Value::Real(value) => value,
        other => panic!("{script}: not a Real ({})", other.kind_name()),
    };
    assert!(!value.is_exact());
    assert_ne!(value, Real::from_str("0.5").expect("0.5"));
    assert!(!value.to_string().contains('('), "the value must not be a periodic expansion");
    let distance = Real::from_str("0.5").expect("0.5").sub(&value).abs();
    let budget = Real::from_str(&format!("0.{}1", "0".repeat(28))).expect("1e-29");
    assert!(
        distance.cmp(&budget) == std::cmp::Ordering::Less,
        "the truncation agrees with 1/2 to 28 places ({value})"
    );
}
