//! The expected-failure boundary of numeric evaluation, as a library surface.
//!
//! The reference evaluator has two failure kinds, and the boundary between them is a contract:
//!
//! * `EvaluationException` is the kernel's typed "not defined here" refusal. The fold, limit,
//!   integration and solver-verification paths catch exactly this kind and read it as a
//!   mathematical non-answer.
//! * Any other exception is a defect. The reference's own expected-failure tests use
//!   `InvalidOperationException`; a foreign exception must escape every narrowed catch instead
//!   of being mistaken for "no value".
//!
//! `crate::numeric::evaluate` reports every failure as a [`LangError`]. That is the right shape
//! for the script envelope, but it cannot express the kind boundary: its code and category are
//! the wire taxonomy, not the exception the kernel used to decide whether to absorb the failure.
//! This module is the additive, library-level path used by the expected-failure boundary rows:
//! it returns [`EvaluationError`], whose variants are named after the reference's exception
//! types, so a host can observe exactly which failure the reference would have thrown.
//!
//! Successful evaluation is unchanged. Nothing here is called by the engine or by
//! `numeric::evaluate`; the entry points below are the only callers of the new walk.

use std::cmp::Ordering;
use std::fmt;

use lovelace_real::Real;
use lovelace_symbolics::{Expr, Rat};

use crate::LangError;
use crate::Value;
use crate::numeric::Num;

/// The reference's `EvaluationException`: the expression is not defined at this input.
///
/// The message is the reference's message verbatim. The kernel's narrowed catches absorb this
/// kind and leave the expression symbolic or report a typed non-answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluationException {
    message: String,
}

impl EvaluationException {
    /// Builds the typed domain refusal with the reference's message.
    pub fn new(message: impl Into<String>) -> EvaluationException {
        EvaluationException {
            message: message.into(),
        }
    }

    /// The message the reference would have published.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for EvaluationException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for EvaluationException {}

/// The defect kind: the reference's `InvalidOperationException` at the evaluation boundary.
///
/// The reference's expected-failure tests deliberately make a numeric evaluator throw this kind
/// and require it to escape. A Rust `Err(EvaluationError::InvalidOperation(_))` is the same
/// observable: a failure that no typed catch may absorb.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidOperationException {
    message: String,
}

impl InvalidOperationException {
    /// Builds the defect with the reference's message.
    pub fn new(message: impl Into<String>) -> InvalidOperationException {
        InvalidOperationException {
            message: message.into(),
        }
    }

    /// The message the reference would have published.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for InvalidOperationException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for InvalidOperationException {}

/// The reference's exception kind, plus the message it carried.
///
/// The variant is the kind; [`EvaluationError::kind`] spells it with the reference's type name
/// for a test that wants to assert the kind as data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvaluationError {
    /// The reference's `EvaluationException`: absorbable domain refusal.
    Evaluation(EvaluationException),
    /// The reference's `InvalidOperationException`: a defect that must propagate.
    InvalidOperation(InvalidOperationException),
}

impl EvaluationError {
    /// A typed domain refusal.
    pub fn evaluation(message: impl Into<String>) -> EvaluationError {
        EvaluationError::Evaluation(EvaluationException::new(message))
    }

    /// A defect that must propagate.
    pub fn invalid_operation(message: impl Into<String>) -> EvaluationError {
        EvaluationError::InvalidOperation(InvalidOperationException::new(message))
    }

    /// The reference's exception type name: `EvaluationException` or
    /// `InvalidOperationException`.
    pub fn kind(&self) -> &'static str {
        match self {
            EvaluationError::Evaluation(_) => "EvaluationException",
            EvaluationError::InvalidOperation(_) => "InvalidOperationException",
        }
    }

    /// The message the reference would have published.
    pub fn message(&self) -> &str {
        match self {
            EvaluationError::Evaluation(error) => error.message(),
            EvaluationError::InvalidOperation(error) => error.message(),
        }
    }
}

impl From<EvaluationException> for EvaluationError {
    fn from(error: EvaluationException) -> EvaluationError {
        EvaluationError::Evaluation(error)
    }
}

impl From<InvalidOperationException> for EvaluationError {
    fn from(error: InvalidOperationException) -> EvaluationError {
        EvaluationError::InvalidOperation(error)
    }
}

impl fmt::Display for EvaluationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind(), self.message())
    }
}

impl std::error::Error for EvaluationError {}

/// The reference's numeric-evaluator slot (`FunctionDefinition.NumericEvaluator`).
///
/// A host registers an implementation for the names it owns. `Ok(None)` defers to the built-in
/// elementary-function table; `Ok(Some(value))` is the evaluated value; `Err` is the failure.
/// In the reference the slot could throw any exception; here the two kinds are explicit so the
/// fold can decide whether to absorb the failure.
pub trait NumericEvaluator {
    /// Evaluates `name(args)`, `None` when this evaluator does not own `name`.
    fn evaluate(
        &self,
        name: &str,
        args: &[Num],
        places: usize,
    ) -> Result<Option<Num>, EvaluationError>;
}

impl<F> NumericEvaluator for F
where
    F: Fn(&str, &[Num], usize) -> Result<Option<Num>, EvaluationError>,
{
    fn evaluate(
        &self,
        name: &str,
        args: &[Num],
        places: usize,
    ) -> Result<Option<Num>, EvaluationError> {
        self(name, args, places)
    }
}

/// The reference's built-in elementary-function table (`CoreFunctions.Register`).
///
/// Returning `Some` is the reference's registered numeric evaluator; the `Err` is classified by
/// message because the Rust numeric tier reports both kinds as [`LangError`]. The only
/// `InvalidOperationException` the table itself raises is the complex-to-real refusal, which is
/// exactly the message the reference's `NumOps.ToReal` uses.
pub fn builtin_numeric_function(
    name: &str,
    args: &[Num],
    places: usize,
) -> Result<Option<Num>, EvaluationError> {
    match crate::numeric::evaluate_function(name, args, places) {
        Ok(value) => Ok(Some(value)),
        Err(error) => Err(classify_lang_error(error)),
    }
}

/// The reference's `NumOps.ToReal` refusal, which is an `InvalidOperationException`.
const COMPLEX_TO_REAL_MESSAGE: &str = "Complex value cannot convert to Real (use re()/im()).";

/// Maps the wire error onto the reference exception kind. The numeric tier reports every failure
/// as [`LangError`]; the only failure the reference raises as `InvalidOperationException` on
/// these paths is the complex-to-real conversion, so that one message keeps the defect kind.
fn classify_lang_error(error: LangError) -> EvaluationError {
    if error.message.starts_with(COMPLEX_TO_REAL_MESSAGE) {
        EvaluationError::invalid_operation(error.message)
    } else {
        EvaluationError::evaluation(error.message)
    }
}

/// `NumOps.Compare`: orders two numbers, refusing a complex operand with the reference's
/// `InvalidOperationException` and message.
pub fn compare(left: &Num, right: &Num, places: usize) -> Result<Ordering, EvaluationError> {
    if matches!(left, Num::Complex(_)) || matches!(right, Num::Complex(_)) {
        return Err(EvaluationError::invalid_operation(COMPLEX_TO_REAL_MESSAGE));
    }
    match (left, right) {
        (Num::Exact(left), Num::Exact(right)) => Ok(left.cmp(right)),
        _ => {
            let left = left
                .to_real(places)
                .ok_or_else(|| EvaluationError::invalid_operation(COMPLEX_TO_REAL_MESSAGE))?;
            let right = right
                .to_real(places)
                .ok_or_else(|| EvaluationError::invalid_operation(COMPLEX_TO_REAL_MESSAGE))?;
            Ok(left.cmp(&right))
        }
    }
}

/// `NumOps.TryCompare`: orders two numbers, answering `None` for an unordered (complex) pair
/// instead of throwing. This is the reference's `false` plus its zero comparison value.
pub fn try_compare(left: &Num, right: &Num, places: usize) -> Option<Ordering> {
    if matches!(left, Num::Complex(_)) || matches!(right, Num::Complex(_)) {
        return None;
    }
    compare(left, right, places).ok()
}

/// The numeric value of a constant leaf, or `None` for anything else.
fn constant_to_num(expression: &Expr) -> Option<Num> {
    match expression {
        Expr::Num(value) => Some(Num::Exact(value.clone())),
        Expr::Real(text) => Real::from_str(text).map(|value| Num::Real(value.mark_inexact())),
        _ => None,
    }
}

/// The expression a numeric value denotes: the reference's `Evaluation.NumToExpr`.
fn num_to_expr(value: &Num) -> Expr {
    match value {
        Num::Exact(rational) => Expr::Num(rational.clone()),
        Num::Real(real) => Expr::real(&real.to_string()),
        Num::Complex(complex) => Expr::add(
            Expr::real(&complex.re().to_string()),
            Expr::mul(
                Expr::real(&complex.im().to_string()),
                Expr::Fun("i".to_string(), Vec::new()),
            ),
        ),
    }
}

/// The binding for a symbol name, in the reference's name-first order.
fn binding<'a>(bindings: &'a [(&str, Num)], name: &str) -> Option<&'a Num> {
    bindings
        .iter()
        .find(|(key, _)| *key == name)
        .map(|(_, value)| value)
}

/// `Evaluation.EvaluateToExpr`: substitutes numeric bindings and folds a function application
/// only when every argument is a numeric constant.
///
/// The reference folds `Exprs.Function` eagerly at construction as well; this Rust tree has no
/// eager fold, so the same fold happens here. The kind boundary is the point: a typed
/// [`EvaluationError::Evaluation`] from the evaluator leaves the node symbolic (the reference's
/// narrowed catch), while [`EvaluationError::InvalidOperation`] escapes.
pub fn evaluate_to_expr(
    expression: &Expr,
    places: usize,
    bindings: &[(&str, Num)],
    functions: &dyn NumericEvaluator,
) -> Result<Expr, EvaluationError> {
    let places = places.max(1);
    match expression {
        Expr::Sym(name) => Ok(match binding(bindings, name) {
            Some(value) => num_to_expr(value),
            None => expression.clone(),
        }),
        Expr::Add(terms) => {
            let mut rebuilt: Vec<Expr> = Vec::with_capacity(terms.len());
            for term in terms {
                rebuilt.push(evaluate_to_expr(term, places, bindings, functions)?);
            }
            Ok(rebuilt
                .into_iter()
                .reduce(Expr::add)
                .unwrap_or_else(|| Expr::num(0)))
        }
        Expr::Mul(factors) => {
            let mut rebuilt: Vec<Expr> = Vec::with_capacity(factors.len());
            for factor in factors {
                rebuilt.push(evaluate_to_expr(factor, places, bindings, functions)?);
            }
            Ok(rebuilt
                .into_iter()
                .reduce(Expr::mul)
                .unwrap_or_else(|| Expr::num(1)))
        }
        Expr::Pow(base, exponent) => Ok(Expr::pow(
            evaluate_to_expr(base, places, bindings, functions)?,
            evaluate_to_expr(exponent, places, bindings, functions)?,
        )),
        Expr::Div(numerator, denominator) => Ok(Expr::div(
            evaluate_to_expr(numerator, places, bindings, functions)?,
            evaluate_to_expr(denominator, places, bindings, functions)?,
        )),
        Expr::Fun(name, arguments) => {
            // The reference model has dedicated node kinds for these; its EvaluateToExpr default
            // arm returns them unchanged, without substituting inside them. The Rust model keeps
            // them as function applications, so the same opaque treatment is expressed by name.
            if is_opaque_evaluate_to_expr_node(name) {
                return Ok(expression.clone());
            }
            let mut rebuilt: Vec<Expr> = Vec::with_capacity(arguments.len());
            for argument in arguments {
                rebuilt.push(evaluate_to_expr(argument, places, bindings, functions)?);
            }
            let mut numbers: Vec<Num> = Vec::with_capacity(rebuilt.len());
            let mut all_constants = true;
            for argument in &rebuilt {
                match constant_to_num(argument) {
                    Some(value) => numbers.push(value),
                    None => {
                        all_constants = false;
                        break;
                    }
                }
            }
            if !all_constants {
                return Ok(Expr::Fun(name.clone(), rebuilt));
            }
            match functions.evaluate(name, &numbers, places) {
                Ok(Some(value)) => Ok(num_to_expr(&value)),
                Ok(None) => match crate::numeric::evaluate_function(name, &numbers, places) {
                    Ok(value) => Ok(num_to_expr(&value)),
                    Err(error) => match classify_lang_error(error) {
                        // The reference catches EvaluationException at the fold and leaves the
                        // node symbolic; any other failure is a defect and propagates.
                        EvaluationError::Evaluation(_) => Ok(Expr::Fun(name.clone(), rebuilt)),
                        defect => Err(defect),
                    },
                },
                // The fold's narrowed catch: a typed domain refusal means "no value here".
                Err(EvaluationError::Evaluation(_)) => Ok(Expr::Fun(name.clone(), rebuilt)),
                Err(defect) => Err(defect),
            }
        }
        // Constant leaves, relations, and every node family the reference's EvaluateToExpr does
        // not traverse are returned unchanged.
        _ => Ok(expression.clone()),
    }
}

/// The Rust function spellings of the reference's non-function node kinds. `EvaluateToExpr`
/// returns these unchanged.
fn is_opaque_evaluate_to_expr_node(name: &str) -> bool {
    matches!(
        name,
        "piecewise" | "and" | "or" | "not" | "inf" | "pi" | "e" | "i"
    )
}

/// `Evaluation.EvaluateToNum`: the whole tree to a [`Num`], or the reference exception kind.
///
/// The arms mirror `Evaluation.cs`; the function arm calls the host evaluator first and the
/// built-in table second. A typed domain refusal from either propagates (the reference's
/// `EvaluateToNum` does not absorb it); a defect propagates with its own kind.
pub fn evaluate_to_num(
    expression: &Expr,
    places: usize,
    bindings: &[(&str, Num)],
    functions: &dyn NumericEvaluator,
) -> Result<Num, EvaluationError> {
    let places = places.max(1);
    match expression {
        Expr::Num(value) => Ok(Num::Exact(value.clone())),
        Expr::Real(text) => Real::from_str(text)
            .map(|value| Num::Real(value.mark_inexact()))
            .ok_or_else(|| {
                EvaluationError::evaluation(format!(
                    "The real constant '{text}' is not a decimal value."
                ))
            }),
        Expr::Sym(name) => binding(bindings, name)
            .cloned()
            .ok_or_else(|| EvaluationError::evaluation(format!("No value for symbol '{name}'."))),
        Expr::Add(terms) => {
            let mut accumulator = Num::Exact(Rat::zero());
            for term in terms {
                let value = evaluate_to_num(term, places, bindings, functions)?;
                accumulator = accumulator
                    .add(&value, places)
                    .map_err(classify_lang_error)?;
            }
            Ok(accumulator)
        }
        Expr::Mul(factors) => {
            let mut accumulator = Num::Exact(Rat::one());
            for factor in factors {
                let value = evaluate_to_num(factor, places, bindings, functions)?;
                accumulator = accumulator
                    .mul(&value, places)
                    .map_err(classify_lang_error)?;
            }
            Ok(accumulator)
        }
        Expr::Div(numerator, denominator) => {
            let numerator = evaluate_to_num(numerator, places, bindings, functions)?;
            let denominator = evaluate_to_num(denominator, places, bindings, functions)?;
            numerator
                .div(&denominator, places)
                .map_err(classify_lang_error)
        }
        Expr::Pow(base, exponent) => {
            let base = evaluate_to_num(base, places, bindings, functions)?;
            let exponent = evaluate_to_num(exponent, places, bindings, functions)?;
            let zero = Num::Exact(Rat::zero());
            if base.is_zero() && compare(&exponent, &zero, places)? <= Ordering::Equal {
                return Err(EvaluationError::evaluation(
                    "0 raised to a non-positive power.",
                ));
            }
            base.pow(&exponent, places).map_err(classify_lang_error)
        }
        // The reference's default arm: a relation has no numeric value.
        Expr::Rel(..) => Err(EvaluationError::evaluation(
            "Node of kind Relation cannot be evaluated numerically.",
        )),
        Expr::Fun(name, arguments) => {
            // The reference's NamedConstantExpr arm: these are not registry functions, so the
            // host evaluator is not consulted for them.
            if arguments.is_empty() {
                match name.as_str() {
                    "inf" => {
                        return Err(EvaluationError::evaluation(
                            "Infinity has no numeric value.",
                        ));
                    }
                    "pi" => return Ok(Num::Real(Real::pi(places))),
                    "e" => return Ok(Num::Real(Real::e(places))),
                    "i" => return Ok(Num::Complex(crate::complex::Complex::i())),
                    _ => {}
                }
            }
            // The reference's PiecewiseExpr arm: take the first numerically True guard, skip
            // False, refuse anything else, and use otherwise when every guard is False.
            if name == "piecewise" {
                if let Some((branches, otherwise)) = expression.as_piecewise() {
                    for (guard, value) in branches {
                        match evaluate_condition(&guard, places, bindings, functions)? {
                            Tristate::True => {
                                return evaluate_to_num(&value, places, bindings, functions);
                            }
                            Tristate::False => continue,
                            Tristate::Unknown => {
                                return Err(EvaluationError::evaluation(
                                    "Piecewise guard cannot be decided numerically.",
                                ));
                            }
                        }
                    }
                    return evaluate_to_num(&otherwise, places, bindings, functions);
                }
            }
            // The reference's And/Or/Not arm: answer the exact integers 1/0, refuse Unknown.
            if matches!(name.as_str(), "and" | "or" | "not") {
                return match evaluate_condition(expression, places, bindings, functions)? {
                    Tristate::True => Ok(Num::Exact(Rat::one())),
                    Tristate::False => Ok(Num::Exact(Rat::zero())),
                    Tristate::Unknown => Err(EvaluationError::evaluation(
                        "Logical condition cannot be decided numerically.",
                    )),
                };
            }
            let mut values: Vec<Num> = Vec::with_capacity(arguments.len());
            for argument in arguments {
                values.push(evaluate_to_num(argument, places, bindings, functions)?);
            }
            match functions.evaluate(name, &values, places) {
                Ok(Some(value)) => Ok(value),
                Ok(None) => crate::numeric::evaluate_function(name, &values, places)
                    .map_err(classify_lang_error),
                Err(error) => Err(error),
            }
        }
    }
}

/// The three-valued condition result the reference's `EvaluateCondition` uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tristate {
    /// The condition is true under the bindings.
    True,
    /// The condition is false under the bindings.
    False,
    /// At least one operand could not be decided; never coerced to either truth value.
    Unknown,
}

/// `Evaluation.EvaluateCondition` over the empty assumption scope: relations, And/Or/Not, and
/// the exact constants 0/1. A typed numeric failure inside a guard is Unknown (the reference
/// catches `EvaluationException` there); a defect propagates.
///
/// Public because it is the row-level surface of the reference's Kleene evaluation: a host can
/// read the True/False/Unknown answer directly instead of inferring it from the 1/0 that
/// `EvaluateToNum` publishes, and the distinction is the whole contract (Unknown must never be
/// coerced to False).
pub fn evaluate_condition(
    condition: &Expr,
    places: usize,
    bindings: &[(&str, Num)],
    functions: &dyn NumericEvaluator,
) -> Result<Tristate, EvaluationError> {
    match condition {
        Expr::Num(value) if value.is_zero() => Ok(Tristate::False),
        Expr::Num(value) if value.is_one() => Ok(Tristate::True),
        Expr::Rel(op, left, right) => {
            let left = match evaluate_to_num(left, places, bindings, functions) {
                Ok(value) => value,
                Err(EvaluationError::Evaluation(_)) => return Ok(Tristate::Unknown),
                Err(defect) => return Err(defect),
            };
            let right = match evaluate_to_num(right, places, bindings, functions) {
                Ok(value) => value,
                Err(EvaluationError::Evaluation(_)) => return Ok(Tristate::Unknown),
                Err(defect) => return Err(defect),
            };
            let Some(ordering) = try_compare(&left, &right, places) else {
                return Ok(Tristate::Unknown);
            };
            let truth = match op {
                lovelace_symbolics::RelOp::Eq => ordering == Ordering::Equal,
                lovelace_symbolics::RelOp::Ne => ordering != Ordering::Equal,
                lovelace_symbolics::RelOp::Lt => ordering == Ordering::Less,
                lovelace_symbolics::RelOp::Le => ordering != Ordering::Greater,
                lovelace_symbolics::RelOp::Gt => ordering == Ordering::Greater,
                lovelace_symbolics::RelOp::Ge => ordering != Ordering::Less,
            };
            Ok(if truth {
                Tristate::True
            } else {
                Tristate::False
            })
        }
        Expr::Fun(name, arguments) if name == "not" && arguments.len() == 1 => Ok(
            match evaluate_condition(&arguments[0], places, bindings, functions)? {
                Tristate::True => Tristate::False,
                Tristate::False => Tristate::True,
                Tristate::Unknown => Tristate::Unknown,
            },
        ),
        Expr::Fun(name, arguments) if name == "and" => {
            let mut unknown = false;
            for argument in arguments {
                match evaluate_condition(argument, places, bindings, functions)? {
                    Tristate::False => return Ok(Tristate::False),
                    Tristate::Unknown => unknown = true,
                    Tristate::True => {}
                }
            }
            Ok(if unknown {
                Tristate::Unknown
            } else {
                Tristate::True
            })
        }
        Expr::Fun(name, arguments) if name == "or" => {
            let mut unknown = false;
            for argument in arguments {
                match evaluate_condition(argument, places, bindings, functions)? {
                    Tristate::True => return Ok(Tristate::True),
                    Tristate::Unknown => unknown = true,
                    Tristate::False => {}
                }
            }
            Ok(if unknown {
                Tristate::Unknown
            } else {
                Tristate::False
            })
        }
        _ => Ok(Tristate::Unknown),
    }
}

/// The direction a limit approaches its point from (`LimitDirection`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LimitDirection {
    /// Both sides; a disagreement is published as `DoesNotExist`.
    TwoSided,
    /// From below.
    FromLeft,
    /// From above.
    FromRight,
}

/// `Limits.Limit`: the limit of `expression` as `variable` approaches `point`.
///
/// The direct-substitution step is the reference's `LimitCore` guard: the substituted expression
/// is evaluated with the host's evaluator; a typed [`EvaluationError::Evaluation`] falls through
/// to the existing symbolic limit engine (a pole or branch point is not a defect), while
/// [`EvaluationError::InvalidOperation`] escapes. An EXACT successful substitution is the limit
/// value (the reference's `LimitResult.Of`); an approximate or complex value keeps the engine's
/// existing answer, so successful evaluation is unchanged. The exactness boundary is deliberate:
/// the engine's `limit_value_record` is the exactness carrier, and delegating an approximate
/// value to the series path is what the current engine already does.
pub fn limit(
    expression: &Expr,
    variable: &str,
    point: &Expr,
    direction: LimitDirection,
    functions: &dyn NumericEvaluator,
) -> Result<Value, EvaluationError> {
    const PLACES: usize = 30;
    let at_infinity = is_infinity(point) || is_negated_infinity(point);
    let substituted = if at_infinity {
        // The reference's AtInfinity substitutes x = 1/t and recurses at t -> 0. The direct
        // substitution site is the same one, so the defect probe runs on that transformed tree.
        let transformed = lovelace_symbolics::substitute(
            expression,
            variable,
            &Expr::div(Expr::num(1), Expr::sym("__t")),
        );
        lovelace_symbolics::substitute(&transformed, "__t", &Expr::num(0))
    } else {
        lovelace_symbolics::substitute(expression, variable, point)
    };
    let unresolved = if at_infinity { "__t" } else { variable };
    if !substituted.contains(unresolved) {
        match evaluate_to_num(&substituted, PLACES, &[], functions) {
            // An exact value at the point is the limit. An exactly-zero real/ complex is the
            // engine's published zero (sqrt(0) crosses the numeric tier as an exact-zero Real).
            Ok(Num::Exact(rational)) => {
                return Ok(crate::builtins::limit_value_record(Value::Symbolic(
                    Expr::Num(rational),
                )));
            }
            Ok(Num::Real(real)) if real.is_zero() => {
                return Ok(crate::builtins::limit_value_record(Value::Symbolic(
                    Expr::num(0),
                )));
            }
            Ok(Num::Complex(complex)) if complex.re().is_zero() && complex.im().is_zero() => {
                return Ok(crate::builtins::limit_value_record(Value::Symbolic(
                    Expr::num(0),
                )));
            }
            Ok(_) => {}
            Err(EvaluationError::Evaluation(_)) => {}
            Err(defect) => return Err(defect),
        }
    }
    let side = match direction {
        LimitDirection::TwoSided => crate::builtins::Side::Both,
        LimitDirection::FromLeft => crate::builtins::Side::Left,
        LimitDirection::FromRight => crate::builtins::Side::Right,
    };
    Ok(crate::builtins::limit_result(
        expression,
        variable,
        point,
        at_infinity,
        side,
    ))
}

/// True for the reference's `+inf` point node.
fn is_infinity(expression: &Expr) -> bool {
    matches!(expression, Expr::Fun(name, arguments) if name == "inf" && arguments.is_empty())
}

/// True for the reference's `-inf` point node (a product carrying the infinity node).
fn is_negated_infinity(expression: &Expr) -> bool {
    let infinity = Expr::Fun("inf".to_string(), Vec::new());
    match expression {
        Expr::Mul(factors) => factors.contains(&infinity),
        _ => false,
    }
}
