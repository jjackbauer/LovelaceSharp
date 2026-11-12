//! Round 90 — the language half of the four deferred symbolic families.
//!
//! Each test names the C# row it closes and drives the same surface the row drives:
//!
//! * `DomainFailureVocabularyTests` — the evaluator API's `EvaluationException` vocabulary, read
//!   through `evaluation_boundary::evaluate_to_num` (the additive typed path) and, for the Real
//!   division, through the engine envelope.
//! * `RelationsAndFamiliesTests` — Kleene evaluation (True/False/Unknown, never coerced),
//!   the compound-guard piecewise through both numeric evaluation and its MathIR lowering, and the
//!   structured integration result with its `x > 0` condition.
//! * `IntegerPowTests` — the checked `Integer.Pow(Integer)` refusals.
//! * `ExactnessFlagTests/SolverSolutionExactness_AndValueExact_Agree` — the published Solution
//!   record's own two fields must agree.

use lovelace_int::Int;
use lovelace_nat::Nat;
use lovelace_symbolics::{Expr, Rat, RelOp};

use crate::Value;
use crate::evaluation_boundary::{
    EvaluationError, Tristate, builtin_numeric_function, evaluate_condition, evaluate_to_num,
};
use crate::mathir::compile_kernel;
use crate::numeric::{Num, integer_pow};

fn rat(numerator: i64, denominator: u64) -> Rat {
    Rat::from_parts(Int::from_i64(numerator), Nat::from_u64(denominator))
}

fn num(value: i64) -> Num {
    Num::Exact(Rat::from_i64(value))
}

fn exact_rational(value: &Num) -> Rat {
    match value {
        Num::Exact(value) => value.clone(),
        other => panic!("expected an exact rational, got {other:?}"),
    }
}

fn engine_eval(source: &str) -> Value {
    let mut engine = crate::Engine::new();
    engine
        .eval(source)
        .unwrap_or_else(|error| panic!("{source} failed: {}", error.message))
}

fn engine_error(source: &str) -> crate::LangError {
    let mut engine = crate::Engine::new();
    engine.eval(source).expect_err("the script must be refused")
}

fn record_field<'a>(value: &'a Value, name: &str) -> &'a Value {
    match value {
        Value::Record(type_name, fields) => fields
            .iter()
            .find(|(field, _)| field == name)
            .map(|(_, value)| value)
            .unwrap_or_else(|| panic!("{type_name} has no field {name}")),
        other => panic!("expected a record, got {other:?}"),
    }
}

fn enum_name<'a>(value: &'a Value, type_name: &str) -> &'a str {
    match value {
        Value::Enum(actual_type, variant) => {
            assert_eq!(actual_type, type_name);
            variant
        }
        other => panic!("expected the {type_name} enum, got {other:?}"),
    }
}

fn symbolic(value: &Value) -> &Expr {
    match value {
        Value::Symbolic(expression) => expression,
        other => panic!("expected a Symbolic value, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// DomainFailureVocabularyTests
// ---------------------------------------------------------------------------

/// DomainFailureVocabularyTests.cs:19 `DivisionByZero_IsAnEvaluationFailure`: the exact-division
/// refusal is the evaluator's typed domain kind, not the defect kind.
#[test]
fn division_by_zero_is_an_evaluation_failure() {
    let quotient = Expr::Div(Box::new(Expr::num(1)), Box::new(Expr::num(0)));
    let error = evaluate_to_num(&quotient, 40, &[], &builtin_numeric_function)
        .expect_err("division by zero is a domain refusal");
    assert_eq!(error.kind(), "EvaluationException");
    assert!(matches!(error, EvaluationError::Evaluation(_)));
    assert_eq!(
        error.message(),
        "Division by zero during numeric evaluation."
    );

    // the language envelope of the same failure keeps the same taxonomy
    let envelope = engine_error("1/0");
    assert_eq!(envelope.code, "DivisionByZero");
    assert_eq!(envelope.category, "DomainError");
    assert_eq!(envelope.message, "Cannot divide by zero.");
}

/// DomainFailureVocabularyTests.cs:26 `DivisionByZero_OnTheRealPath_IsAnEvaluationFailure`: the
/// Real zero divisor is the same typed failure, and the language envelope names the Real path.
#[test]
fn division_by_zero_on_the_real_path_is_an_evaluation_failure() {
    let quotient = Expr::Div(Box::new(Expr::num(1)), Box::new(Expr::real("0.0")));
    let error = evaluate_to_num(&quotient, 40, &[], &builtin_numeric_function)
        .expect_err("division by a Real zero is a domain refusal");
    assert_eq!(error.kind(), "EvaluationException");
    assert_eq!(
        error.message(),
        "Division by zero during numeric evaluation."
    );

    let envelope = engine_error("1/0.0");
    assert_eq!(envelope.code, "DivisionByZero");
    assert_eq!(envelope.category, "DomainError");
    assert_eq!(envelope.message, "Cannot divide a Real by zero.");
}

/// DomainFailureVocabularyTests.cs:34 `LogOfZero_IsAnEvaluationFailure`: ln(0) is the typed
/// domain refusal with the reference's own message.
#[test]
fn log_of_zero_is_an_evaluation_failure() {
    let logarithm = Expr::Fun("log".to_string(), vec![Expr::num(0)]);
    let error = evaluate_to_num(&logarithm, 40, &[], &builtin_numeric_function)
        .expect_err("log(0) is a domain refusal");
    assert_eq!(error.kind(), "EvaluationException");
    assert!(matches!(error, EvaluationError::Evaluation(_)));
    assert_eq!(error.message(), "Ln(x) requires x > 0. (Parameter 'x')");

    let envelope = engine_error("evalf(log(0), 20)");
    assert_eq!(envelope.code, "EvaluationError");
    assert_eq!(envelope.category, "DomainError");
    assert_eq!(envelope.message, "Ln(x) requires x > 0. (Parameter 'x')");
}

/// DomainFailureVocabularyTests.cs:41 `ZeroToANegativePower_IsAnEvaluationFailure`: 0^-1 is the
/// typed domain refusal; the language envelope publishes EvaluationError, not InvalidOperation.
#[test]
fn zero_to_a_negative_power_is_an_evaluation_failure() {
    let zero = Expr::pow(Expr::num(0), Expr::num(-1));
    let error = evaluate_to_num(&zero, 40, &[], &builtin_numeric_function)
        .expect_err("0^-1 is a domain refusal");
    assert_eq!(error.kind(), "EvaluationException");
    assert!(matches!(error, EvaluationError::Evaluation(_)));
    assert_eq!(error.message(), "0 raised to a non-positive power.");

    let envelope = engine_error("x = symbol(\"x\")\nevalf(subs(0^x, x, -1), 20)");
    assert_eq!(envelope.code, "EvaluationError");
    assert_eq!(envelope.category, "DomainError");
    assert_eq!(envelope.message, "0 raised to a non-positive power.");
}

// ---------------------------------------------------------------------------
// RelationsAndFamiliesTests
// ---------------------------------------------------------------------------

fn bound(name: &str, numerator: i64, denominator: u64) -> [(&str, Num); 1] {
    [(name, Num::Exact(rat(numerator, denominator)))]
}

fn condition_and(x: &Expr) -> Expr {
    Expr::and_all(vec![
        Expr::relation(RelOp::Gt, x.clone(), Expr::num(0)),
        Expr::relation(RelOp::Lt, x.clone(), Expr::num(1)),
    ])
}

/// RelationsAndFamiliesTests.cs:48 `KleeneEvaluation_AndOrNot`: And/Or/Not decide
/// True/False/Unknown through the three-valued condition evaluator, and EvaluateToNum answers the
/// exact 1/0 but REFUSES Unknown.
#[test]
fn kleene_evaluation_and_or_not() {
    let x = Expr::sym("x");
    let cond = condition_and(&x);
    let half = bound("x", 1, 2);
    let two = bound("x", 2, 1);
    let empty: [(&str, Num); 0] = [];

    assert_eq!(
        evaluate_condition(&cond, 40, &half, &builtin_numeric_function),
        Ok(Tristate::True)
    );
    assert_eq!(
        evaluate_condition(&cond, 40, &two, &builtin_numeric_function),
        Ok(Tristate::False)
    );
    assert_eq!(
        evaluate_condition(&cond, 40, &empty, &builtin_numeric_function),
        Ok(Tristate::Unknown)
    );

    // numeric evaluation: 1/0 with Unknown rejected
    assert_eq!(
        exact_rational(
            &evaluate_to_num(&cond, 40, &half, &builtin_numeric_function).expect("decided true")
        ),
        Rat::from_i64(1)
    );
    assert_eq!(
        exact_rational(
            &evaluate_to_num(&cond, 40, &two, &builtin_numeric_function).expect("decided false")
        ),
        Rat::from_i64(0)
    );
    let unknown = evaluate_to_num(&cond, 40, &empty, &builtin_numeric_function)
        .expect_err("an undecidable condition has no numeric value");
    assert_eq!(unknown.kind(), "EvaluationException");
    assert_eq!(
        unknown.message(),
        "Logical condition cannot be decided numerically."
    );

    // Not + Or
    let neg = Expr::logical_not(Expr::relation(RelOp::Gt, x.clone(), Expr::num(0)));
    assert_eq!(
        evaluate_condition(&neg, 40, &half, &builtin_numeric_function),
        Ok(Tristate::False)
    );
    let either = Expr::or_all(vec![
        Expr::relation(RelOp::Gt, x.clone(), Expr::num(1)),
        Expr::relation(RelOp::Lt, x, Expr::num(0)),
    ]);
    assert_eq!(
        evaluate_condition(&either, 40, &half, &builtin_numeric_function),
        Ok(Tristate::False)
    );
    assert_eq!(
        evaluate_condition(&either, 40, &two, &builtin_numeric_function),
        Ok(Tristate::True)
    );
}

/// RelationsAndFamiliesTests.cs:73 `Piecewise_WithCompoundGuard_Evaluates`: the And guard picks
/// the branch numerically AND through the MathIR lowering with the same binding.
#[test]
fn piecewise_with_compound_guard_evaluates() {
    let x = Expr::sym("x");
    let piecewise = Expr::piecewise(vec![(condition_and(&x), Expr::num(1))], Expr::num(0));

    let half = bound("x", 1, 2);
    let two = bound("x", 2, 1);
    assert_eq!(
        exact_rational(
            &evaluate_to_num(&piecewise, 40, &half, &builtin_numeric_function)
                .expect("the branch applies at 1/2")
        ),
        Rat::from_i64(1)
    );
    assert_eq!(
        exact_rational(
            &evaluate_to_num(&piecewise, 40, &two, &builtin_numeric_function)
                .expect("otherwise applies at 2")
        ),
        Rat::from_i64(0)
    );

    // and it lowers to MathIR with the same semantics
    let kernel = compile_kernel(&piecewise, &["x".to_string()]).expect("compound guard compiles");
    assert_eq!(
        exact_rational(&kernel.evaluate(&[Num::Exact(rat(1, 2))]).expect("numeric")),
        Rat::from_i64(1),
        "1/2 is inside the compound guard"
    );
    assert_eq!(
        exact_rational(&kernel.evaluate(&[num(2)]).expect("numeric")),
        Rat::from_i64(0)
    );
    // the boundary: x = 1 fails the strict upper bound, so otherwise still applies
    assert_eq!(
        exact_rational(&kernel.evaluate(&[num(1)]).expect("numeric")),
        Rat::from_i64(0)
    );
}

/// RelationsAndFamiliesTests.cs:25 `LogicalCanonicalization` at the language surface: the
/// constructors fold the exact constants and a double negation, and they flatten a nested
/// same-kind node. (The library-level assertions are pinned in lovelace-symbolics.)
#[test]
fn logical_canonicalization_through_the_engine() {
    let double_negation = engine_eval("x = symbol(\"x\")\nnot(not(x > 0))");
    assert_eq!(symbolic(&double_negation).to_string(), "x > 0");

    let identity = engine_eval("x = symbol(\"x\")\nand(x > 0, 1)");
    assert_eq!(symbolic(&identity).to_string(), "x > 0");

    let false_constant = engine_eval("x = symbol(\"x\")\nand(x > 0, 0)");
    assert_eq!(symbolic(&false_constant).to_string(), "0");

    let true_constant = engine_eval("x = symbol(\"x\")\nor(x > 0, 1)");
    assert_eq!(symbolic(&true_constant).to_string(), "1");

    // The reference engine does NOT deduplicate two separately evaluated equal relations: its
    // constructor's equality is reference equality on the relation's children. The language fold
    // keeps both operands (the library constructor, exercised above when the SAME node is passed
    // twice, does collapse them).
    let duplicates = engine_eval("x = symbol(\"x\")\nand(x > 0, x > 0)");
    let duplicates = symbolic(&duplicates);
    assert_eq!(
        duplicates.canonical(),
        "(and (gt (sym x) (rat 0 1)) (gt (sym x) (rat 0 1)))"
    );
    assert_eq!(duplicates.node_count(), 7);

    let flattened =
        engine_eval("x = symbol(\"x\")\ny = symbol(\"y\")\nand(x > 0, and(x < 1, y > 0))");
    let flattened = symbolic(&flattened);
    assert_eq!(flattened.to_string(), "x < 1 and x > 0 and y > 0");
    match flattened {
        Expr::Fun(name, operands) => {
            assert_eq!(name, "and");
            assert_eq!(operands.len(), 3, "the nested And must be flattened");
        }
        other => panic!("expected an And node, got {other:?}"),
    }
}

/// RelationsAndFamiliesTests.cs:223 `Integrate_StructuredResult_ReportsConditions`: 1/x is
/// SolvedConditional with the x > 0 condition, a polynomial is SolvedExact with no condition, and
/// exp(x^2) is the typed Unevaluated integral.
#[test]
fn integrate_structured_result_reports_conditions() {
    let result = engine_eval("x = symbol(\"x\")\nintegrate_full(1/x, x)");
    assert_eq!(
        enum_name(record_field(&result, "status"), "IntegrationStatus"),
        "SolvedConditional"
    );
    assert_eq!(
        symbolic(record_field(&result, "expression")).to_string(),
        "log(x)"
    );
    match record_field(&result, "conditions") {
        Value::Vector(conditions) => {
            assert_eq!(conditions.len(), 1);
            let condition = symbolic(&conditions[0]);
            assert_eq!(condition.to_string(), "x > 0");
            assert_eq!(condition.canonical(), "(gt (sym x) (rat 0 1))");
        }
        other => panic!("conditions must be a vector, got {other:?}"),
    }

    let polynomial = engine_eval("x = symbol(\"x\")\nintegrate_full(x^5, x)");
    assert_eq!(
        enum_name(record_field(&polynomial, "status"), "IntegrationStatus"),
        "SolvedExact"
    );
    match record_field(&polynomial, "conditions") {
        Value::Vector(conditions) => assert!(conditions.is_empty()),
        other => panic!("conditions must be an empty vector, got {other:?}"),
    }

    let hard = engine_eval("x = symbol(\"x\")\nintegrate_full(exp(x^2), x)");
    assert_eq!(
        enum_name(record_field(&hard, "status"), "IntegrationStatus"),
        "Unevaluated"
    );
    assert_eq!(
        symbolic(record_field(&hard, "expression")).canonical(),
        "(integ (x) (fn exp (pow (sym x) (rat 2 1))))"
    );
}

// ---------------------------------------------------------------------------
// ExactnessFlagTests
// ---------------------------------------------------------------------------

/// ExactnessFlagTests.cs:369 `SolverSolutionExactness_AndValueExact_Agree`: for every published
/// Solution, the value's own exact flag and the record's exactness enum agree, and the exact
/// claims are the ones exercised (the approximate claim is not published for these).
#[test]
fn solver_solution_exactness_and_value_exact_agree() {
    let scripts = [
        "solve_full(x^2 - 2 == 0, x, real)", // irrational algebraic roots
        "solve_full(x^2 - 4 == 0, x, real)", // rational roots
        "solve_full(x^2 + 1 == 0, x)",       // complex roots
        "solve_full(x^3 - 2 == 0, x, real)", // a cube radical
        "solve_full(exp(x) == 1, x)",        // a fold to a rational (family, no solutions)
    ];
    let mut published: Vec<String> = Vec::new();
    for script in scripts {
        let result = engine_eval(&format!("x = symbol(\"x\")\n{script}"));
        let Value::Vector(solutions) = record_field(&result, "solutions") else {
            panic!("solutions must be a vector");
        };
        for solution in solutions {
            let exactness = enum_name(record_field(solution, "exactness"), "SolutionExactness");
            let value = symbolic(record_field(solution, "value"));
            let claims_exact = exactness != "Approximate";
            assert_eq!(
                value.is_exact(),
                claims_exact,
                "{script}: value {} has exact={} but exactness={exactness}",
                value,
                value.is_exact()
            );
            published.push(exactness.to_string());
        }
    }
    assert!(published.iter().any(|name| name == "Exact"));
    assert!(!published.iter().any(|name| name == "Approximate"));
}

// ---------------------------------------------------------------------------
// IntegerPowTests
// ---------------------------------------------------------------------------

fn pow_refusal(base: i64, exponent: i64) -> crate::LangError {
    integer_pow(&Int::from_i64(base), &Int::from_i64(exponent))
        .expect_err("the reference refuses this integer power")
}

/// IntegerPowTests.cs:35 `Pow_GivenZeroExponent_Throws`: the exponent refusal names the exponent.
#[test]
fn pow_given_zero_exponent_throws() {
    let error = pow_refusal(2, 0);
    assert_eq!(error.code, "InvalidArgument");
    assert_eq!(error.category, "TypeMismatch");
    assert_eq!(
        error.message,
        "Exponent must be positive. (Parameter 'exponent')"
    );
}

/// IntegerPowTests.cs:41 `Pow_GivenNegativeExponent_Throws`: a negative exponent is the same
/// exponent refusal (the reference never computes a reciprocal in Integer.Pow).
#[test]
fn pow_given_negative_exponent_throws() {
    let error = pow_refusal(2, -1);
    assert_eq!(error.code, "InvalidArgument");
    assert_eq!(error.category, "TypeMismatch");
    assert_eq!(
        error.message,
        "Exponent must be positive. (Parameter 'exponent')"
    );
}

/// IntegerPowTests.cs:47 `Pow_GivenZeroBase_Throws`: a zero base is checked FIRST, so 0^3 names
/// the base; the positive/negative-base controls stay the reference's results.
#[test]
fn pow_given_zero_base_throws() {
    let error = pow_refusal(0, 3);
    assert_eq!(error.code, "InvalidArgument");
    assert_eq!(error.category, "TypeMismatch");
    assert_eq!(error.message, "Base cannot be zero. (Parameter 'base')");

    // the base check runs before the exponent check, exactly as in Integer.cs:416-423
    let both = pow_refusal(0, 0);
    assert_eq!(both.message, "Base cannot be zero. (Parameter 'base')");

    // controls: the three ported rows still answer the documented values
    let ten = integer_pow(&Int::from_i64(2), &Int::from_i64(10)).expect("2^10");
    assert_eq!(ten, Int::from_i64(1024));
    let even = integer_pow(&Int::from_i64(-2), &Int::from_i64(4)).expect("(-2)^4");
    assert_eq!(even, Int::from_i64(16));
    let odd = integer_pow(&Int::from_i64(-2), &Int::from_i64(3)).expect("(-2)^3");
    assert_eq!(odd, Int::from_i64(-8));
}
