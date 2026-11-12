//! Round 80 — the throwing-evaluator block.
//!
//! The four deferred `Lovelace.Symbolics.Tests.ExpectedFailureBoundaryTests` rows in the
//! `Evaluation.EvaluateToNum/EvaluateToExpr with a throwing evaluator` block:
//!
//! * `EvaluateToExpr_ExactArgument_ThrowingEvaluatorPropagates#0`
//! * `EvaluateToExpr_ApproximateArgument_ThrowingEvaluatorPropagates#0`
//! * `Limit_ThrowingEvaluatorPropagates#0`
//! * `TryCompare_ReportsUnorderedInsteadOfThrowing#0`
//!
//! Each test drives the same expression shape the C# row uses, with a registered evaluator that
//! raises the reference's `InvalidOperationException` kind and message. The negative controls are
//! the same expression with an evaluator that answers, the built-in table's own successful fold,
//! and the two-sided pole the C# class pins as "still behaves exactly as before".

use std::cmp::Ordering;

use lovelace_real::Real;
use lovelace_symbolics::{Expr, Rat};

use crate::Value;
use crate::complex::Complex;
use crate::evaluation_boundary::{
    EvaluationError, LimitDirection, builtin_numeric_function, compare, evaluate_to_expr,
    evaluate_to_num, limit, try_compare,
};
use crate::numeric::Num;

/// ExpectedFailureBoundaryTests.cs:17 — the exact message the C# rows assert.
const BOOM_MESSAGE: &str = "boom: deliberately unexpected failure";

/// The reference's `ContextWithThrowingFunction`: the one registered definition, `boom`, raises
/// the defect kind. It is deliberately NOT an [`EvaluationError::Evaluation`]; that kind is the
/// one the fold is allowed to absorb. Other names defer to the built-in table exactly as the
/// reference's registry does.
fn throwing(name: &str, _args: &[Num], _places: usize) -> Result<Option<Num>, EvaluationError> {
    if name == "boom" {
        Err(EvaluationError::invalid_operation(BOOM_MESSAGE))
    } else {
        Ok(None)
    }
}

/// The same slot, answering a value for the one name it owns: the negative control where the
/// oracle succeeds.
fn answering(name: &str, _args: &[Num], _places: usize) -> Result<Option<Num>, EvaluationError> {
    if name == "boom" {
        Ok(Some(Num::Exact(Rat::from_i64(7))))
    } else {
        Ok(None)
    }
}

fn boom(argument: Expr) -> Expr {
    Expr::Fun("boom".to_string(), vec![argument])
}

fn record_fields<'a>(value: &'a Value, type_name: &str) -> &'a [(String, Value)] {
    match value {
        Value::Record(name, fields) if name == type_name => fields,
        other => panic!("expected record {type_name}, got {other:?}"),
    }
}

fn field<'a>(fields: &'a [(String, Value)], name: &str) -> &'a Value {
    fields
        .iter()
        .find(|(field, _)| field == name)
        .map(|(_, value)| value)
        .unwrap_or_else(|| panic!("record has no field {name}"))
}

fn limit_status(value: &Value) -> &str {
    match field(record_fields(value, "LimitResult"), "status") {
        Value::Enum(type_name, variant) => {
            assert_eq!(type_name, "LimitStatus");
            variant
        }
        other => panic!("expected a LimitStatus enum, got {other:?}"),
    }
}

fn limit_value(value: &Value) -> &Expr {
    match field(record_fields(value, "LimitResult"), "value") {
        Value::Symbolic(expression) => expression,
        other => panic!("expected a Symbolic limit value, got {other:?}"),
    }
}

/// `ExpectedFailureBoundaryTests.cs:35 EvaluateToExpr_ExactArgument_ThrowingEvaluatorPropagates`:
/// an exact argument selects the fold; the evaluator's defect must escape with its kind and
/// message instead of being read as "no numeric value".
#[test]
fn evaluate_to_expr_exact_argument_throwing_evaluator_propagates() {
    let expression = boom(Expr::num(2));

    let error = evaluate_to_expr(&expression, 40, &[], &throwing)
        .expect_err("the exact-argument fold must propagate the defect");

    assert_eq!(error.kind(), "InvalidOperationException");
    assert_eq!(error.message(), BOOM_MESSAGE);
    assert!(matches!(error, EvaluationError::InvalidOperation(_)));
}

/// `ExpectedFailureBoundaryTests.cs:46 EvaluateToExpr_ApproximateArgument_ThrowingEvaluatorPropagates`:
/// a Real beyond the exact-decimal window selects the approximate folding branch; the same defect
/// must escape there too. The C# test builds `0.` followed by 25 ones.
#[test]
fn evaluate_to_expr_approximate_argument_throwing_evaluator_propagates() {
    let approximate = Real::from_str("0.1111111111111111111111111")
        .expect("twenty-five ones is a decimal the real tier parses");
    let expression = boom(Expr::real(&approximate.to_string()));

    let error = evaluate_to_expr(&expression, 40, &[], &throwing)
        .expect_err("the approximate-argument fold must propagate the defect");

    assert_eq!(error.kind(), "InvalidOperationException");
    assert_eq!(error.message(), BOOM_MESSAGE);
}

/// `ExpectedFailureBoundaryTests.cs:63 Limit_ThrowingEvaluatorPropagates`: the limit's
/// direct-substitution step evaluates `boom(0)`; the defect must escape the narrowed catch that
/// turns an `EvaluationException` into a series fallback.
#[test]
fn limit_throwing_evaluator_propagates() {
    let expression = boom(Expr::sym("x"));

    let error = limit(
        &expression,
        "x",
        &Expr::num(0),
        LimitDirection::TwoSided,
        &throwing,
    )
    .expect_err("the limit's substitution step must propagate the defect");

    assert_eq!(error.kind(), "InvalidOperationException");
    assert_eq!(error.message(), BOOM_MESSAGE);
}

/// `ExpectedFailureBoundaryTests.cs:94 TryCompare_ReportsUnorderedInsteadOfThrowing`:
/// TryCompare orders a real pair, reports an unordered (complex) pair as `None` (the C#
/// `false` plus the zero comparison value), and the ordered Compare keeps its kind-specific
/// `InvalidOperationException` message.
#[test]
fn try_compare_reports_unordered_instead_of_throwing() {
    let real = Num::Exact(Rat::from_i64(2));
    let three = Num::Exact(Rat::from_i64(3));

    assert_eq!(try_compare(&real, &three, 40), Some(Ordering::Less));
    assert_eq!(try_compare(&three, &real, 40), Some(Ordering::Greater));

    let complex = Num::Complex(Complex::new(Real::from_i64(1), Real::from_i64(1)));
    assert_eq!(
        try_compare(&real, &complex, 40),
        None,
        "an unordered pair is reported as a value, not an exception"
    );

    let error = compare(&real, &complex, 40).expect_err("the ordered form keeps its defect kind");
    assert_eq!(error.kind(), "InvalidOperationException");
    assert_eq!(
        error.message(),
        "Complex value cannot convert to Real (use re()/im())."
    );
}

// ---------------------------------------------------------------------------
// Negative controls: the same shapes where the oracle succeeds
// ---------------------------------------------------------------------------

/// The same expression succeeds when the registered evaluator answers instead of throwing, and
/// the built-in table still folds `abs(-3)` to 3. This is the control that the failure is the
/// evaluator's, not the walk's.
#[test]
fn the_same_expression_succeeds_when_the_evaluator_does_not_throw() {
    let expression = boom(Expr::num(2));

    let folded = evaluate_to_expr(&expression, 40, &[], &answering)
        .expect("an answering evaluator makes the fold succeed");
    assert_eq!(folded, Expr::num(7));

    let absolute = Expr::Fun("abs".to_string(), vec![Expr::num(-3)]);
    let folded = evaluate_to_expr(&absolute, 40, &[], &builtin_numeric_function)
        .expect("the built-in table folds abs(-3)");
    assert_eq!(folded, Expr::num(3));
}

/// The limit of the same expression answers its value at the point when the evaluator answers:
/// `limit(boom(x), x, 0) = 7` under the non-throwing evaluator.
#[test]
fn limit_of_the_same_expression_succeeds_when_the_evaluator_answers() {
    let expression = boom(Expr::sym("x"));

    let record = limit(
        &expression,
        "x",
        &Expr::num(0),
        LimitDirection::TwoSided,
        &answering,
    )
    .expect("the answering evaluator gives a value at the point");

    assert_eq!(limit_status(&record), "Value");
    assert_eq!(limit_value(&record), &Expr::num(7));
}

/// The C# class's other public control (`Limit_TwoSidedPole_StillReportsOpposingSides`) still
/// holds through the new entry point: the wrapper only intercepts the defect, it does not
/// replace the engine's successful answer.
#[test]
fn limit_of_a_two_sided_pole_still_reports_opposing_sides() {
    let expression = Expr::div(Expr::num(1), Expr::sym("x"));

    let record = limit(
        &expression,
        "x",
        &Expr::num(0),
        LimitDirection::TwoSided,
        &builtin_numeric_function,
    )
    .expect("a pole is a typed non-answer, not a defect");

    assert_eq!(limit_status(&record), "DoesNotExist");
}

// ---------------------------------------------------------------------------
// The kind boundary itself (supporting evidence, not a separate census row)
// ---------------------------------------------------------------------------

/// The boundary the C# class exists to pin: a typed `EvaluationException` is absorbed by the
/// fold (the node stays symbolic), while the defect kind is not. The reference's narrowed catch
/// does exactly this, and the two tests above use the other half.
#[test]
fn the_fold_absorbs_the_domain_kind_and_leaves_the_node_symbolic() {
    let refusing = |_name: &str, _args: &[Num], _places: usize| {
        Err(EvaluationError::evaluation("not numerically defined here"))
    };
    let expression = boom(Expr::num(2));

    let folded = evaluate_to_expr(&expression, 40, &[], &refusing)
        .expect("a domain refusal is not a defect");

    assert_eq!(
        folded, expression,
        "the typed domain refusal leaves the node symbolic"
    );
}

/// The typed domain vocabulary the boundary guards: the same walk reports a division by zero,
/// `log(0)` and a zero base under a negative power as `EvaluationException` with the reference's
/// messages. This is supporting evidence for the kind split; it is not one of the four rows.
#[test]
fn evaluate_to_num_reports_the_typed_domain_vocabulary() {
    let quotient = Expr::Div(Box::new(Expr::num(1)), Box::new(Expr::num(0)));
    let error = evaluate_to_num(&quotient, 40, &[], &builtin_numeric_function)
        .expect_err("division by zero is a domain refusal");
    assert_eq!(error.kind(), "EvaluationException");
    assert_eq!(
        error.message(),
        "Division by zero during numeric evaluation."
    );

    let logarithm = Expr::Fun("log".to_string(), vec![Expr::num(0)]);
    let error = evaluate_to_num(&logarithm, 40, &[], &builtin_numeric_function)
        .expect_err("log(0) is a domain refusal");
    assert_eq!(error.kind(), "EvaluationException");
    assert_eq!(error.message(), "Ln(x) requires x > 0. (Parameter 'x')");

    let zero = Expr::pow(Expr::num(0), Expr::num(-1));
    let error = evaluate_to_num(&zero, 40, &[], &builtin_numeric_function)
        .expect_err("0^-1 is a domain refusal");
    assert_eq!(error.kind(), "EvaluationException");
    assert_eq!(error.message(), "0 raised to a non-positive power.");
}

/// EvaluateToNum is the other half of the block name: it does not absorb a typed domain refusal
/// either, and a defect keeps its kind and message through the same function arm.
#[test]
fn evaluate_to_num_propagates_both_kinds() {
    let expression = boom(Expr::num(2));

    let defect = evaluate_to_num(&expression, 40, &[], &throwing)
        .expect_err("a defect is not absorbed by EvaluateToNum");
    assert_eq!(defect.kind(), "InvalidOperationException");
    assert_eq!(defect.message(), BOOM_MESSAGE);

    let refusal = |_name: &str, _args: &[Num], _places: usize| {
        Err(EvaluationError::evaluation("not numerically defined here"))
    };
    let domain = evaluate_to_num(&expression, 40, &[], &refusal)
        .expect_err("EvaluateToNum propagates the typed domain refusal");
    assert_eq!(domain.kind(), "EvaluationException");
    assert_eq!(domain.message(), "not numerically defined here");
}
