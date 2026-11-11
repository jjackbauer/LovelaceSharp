//! The observation classes, the stated tolerance and the classifier.

use std::cmp::Ordering;

use lovelace_symbolics::assumptions::AssumptionSet;
use lovelace_symbolics::{Expr, Rat};

use crate::LangError;
use crate::numeric::Num;

/// The computation precision every sample is taken at (the reference's Rl.WithPrecision(40, 20)).
pub(crate) const PRECISION: usize = 40;

/// The generator version stamped on every probe record.
pub(crate) const GENERATOR_VERSION: &str = "adv11-falsifier/1";

/// An exact rational from a numerator and a denominator.
pub(crate) fn rational(n: i64, d: i64) -> Rat {
    Rat::from_i64(n)
        .div(&Rat::from_i64(d))
        .expect("a nonzero denominator")
}

/// The falsifier's rational rendering: n/d, or n when integral.
pub(crate) fn rat_text(value: &Rat) -> String {
    value.to_string()
}

/// The probe symbol the whole harness uses.
pub(crate) fn x() -> Expr {
    Expr::sym("x")
}

/// The named constant pi as a function leaf.
pub(crate) fn pi() -> Expr {
    Expr::Fun("pi".to_string(), Vec::new())
}

/// A function application.
pub(crate) fn fun(name: &str, args: Vec<Expr>) -> Expr {
    Expr::Fun(name.to_string(), args)
}

/// The assumption set an observation was taken under: {} or {atom; atom}.
pub(crate) fn environment_text(env: &AssumptionSet) -> String {
    if env.atoms().is_empty() {
        "{}".to_string()
    } else {
        let rendered: Vec<String> = env.atoms().iter().map(|atom| atom.describe()).collect();
        format!("{{{}}}", rendered.join("; "))
    }
}

/// Escapes the four characters that would break the probe's line-delimited text form.
pub(crate) fn escape(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => result.push_str("\\\\"),
            '|' => result.push_str("\\p"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            other => result.push(other),
        }
    }
    result
}

/// The inverse of [escape], one pass, so no escape is applied twice.
pub(crate) fn unescape(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            result.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => result.push('\n'),
            Some('r') => result.push('\r'),
            Some('t') => result.push('\t'),
            Some('p') => result.push('|'),
            Some('\\') => result.push('\\'),
            Some(other) => result.push(other),
            None => result.push('\\'),
        }
    }
    result
}

/// A kernel number rendered deterministically.
pub(crate) fn num_text(value: &Num) -> String {
    match value {
        Num::Exact(r) => Expr::Num(r.clone()).to_string(),
        Num::Real(r) => r.to_string(),
        Num::Complex(z) => format!("({} + {}*i)", z.re(), z.im()),
    }
}

/// Orders two non-complex numbers; None when either side is complex.
pub(crate) fn num_cmp(left: &Num, right: &Num) -> Option<Ordering> {
    let left = left.to_real(PRECISION)?;
    let right = right.to_real(PRECISION)?;
    Some(left.cmp(&right))
}

/// The reference's NumOps.IsZero.
pub(crate) fn num_is_zero(value: &Num) -> bool {
    match value {
        Num::Exact(r) => r.is_zero(),
        Num::Real(r) => r.is_zero(),
        Num::Complex(z) => z.re().is_zero() && z.im().is_zero(),
    }
}

/// The reference's NumOps.Abs: a complex value yields its magnitude.
pub(crate) fn num_abs(value: &Num) -> Num {
    match value {
        Num::Exact(r) => Num::Exact(r.abs()),
        Num::Real(r) => Num::Real(r.abs()),
        Num::Complex(z) => Num::Real(z.magnitude(PRECISION)),
    }
}

/// |delta|^2 without a square root: exact for complex differences.
pub(crate) fn num_squared_magnitude(value: &Num) -> Num {
    if let Num::Complex(z) = value {
        return Num::Real(z.magnitude_squared());
    }
    let abs = num_abs(value);
    abs.mul(&abs, PRECISION).unwrap_or(Num::Exact(Rat::zero()))
}

/// The exact expression a bound numeric value denotes.
pub(crate) fn value_expr(value: &Num) -> Expr {
    match value {
        Num::Exact(r) => Expr::Num(r.clone()),
        Num::Real(r) => Expr::Real(r.to_string()),
        Num::Complex(z) => {
            let real = Expr::Real(z.re().to_string());
            let imaginary = Expr::mul(
                Expr::Real(z.im().to_string()),
                Expr::Fun("i".to_string(), Vec::new()),
            );
            Expr::add(real, imaginary)
        }
    }
}

/// How one side of one sample failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum EvalFailure {
    /// The kernel's typed domain refusal: the expression is not defined at this input.
    Undefined(String),
    /// A typed resource refusal (budget, depth, allocation, cancellation).
    Refused(String),
    /// Any other failure. A defect, reported as itself.
    Threw(String),
}

impl EvalFailure {
    /// The free-text detail the record carries.
    pub(crate) fn message(&self) -> &str {
        match self {
            EvalFailure::Undefined(message)
            | EvalFailure::Refused(message)
            | EvalFailure::Threw(message) => message,
        }
    }

    /// The side outcome this failure classifies as.
    pub(crate) fn side(&self) -> SideOutcome {
        match self {
            EvalFailure::Undefined(_) => SideOutcome::Undefined,
            EvalFailure::Refused(_) => SideOutcome::Refused,
            EvalFailure::Threw(_) => SideOutcome::Threw,
        }
    }

    /// The reference's exception description: a type name and a message.
    pub(crate) fn describe(&self) -> String {
        match self {
            EvalFailure::Undefined(message) => format!("EvaluationException: {message}"),
            EvalFailure::Refused(message) => format!("BudgetRefusal: {message}"),
            EvalFailure::Threw(message) => format!("InvalidOperationException: {message}"),
        }
    }
}

/// Maps a numeric failure onto the classifier's side outcome. Only a domain refusal licenses
/// "undefined"; a resource refusal is a refusal, and anything else is a defect.
pub(crate) fn map_lang_error(error: LangError) -> EvalFailure {
    let budget = error.category == "BudgetExceeded"
        || matches!(
            error.code,
            "Cancelled" | "DepthExceeded" | "BudgetExceeded" | "AllocationRefused"
        );
    if budget {
        EvalFailure::Refused(format!("LangError: {}", error.message))
    } else {
        EvalFailure::Undefined(format!("LangError: {}", error.message))
    }
}

/// Evaluates a bound expression, adding the two accessors the reference numeric tier has.
fn bound_eval(expression: &Expr, places: usize) -> Result<Num, LangError> {
    match expression {
        Expr::Fun(name, args) if args.len() == 1 && (name == "re" || name == "im") => {
            let value = bound_eval(&args[0], places)?;
            if name == "im" {
                match value {
                    Num::Complex(z) => Ok(Num::Real(z.im().clone())),
                    _ => Ok(Num::Exact(Rat::zero())),
                }
            } else {
                match value {
                    Num::Complex(z) => Ok(Num::Real(z.re().clone())),
                    other => Ok(other),
                }
            }
        }
        _ => crate::numeric::evaluate(expression, places),
    }
}

/// The evaluator: substitutes the probe value for the variable and runs the numeric tier.
pub(crate) fn default_eval(
    expression: &Expr,
    variable: &str,
    value: &Num,
) -> Result<Num, EvalFailure> {
    let bound = lovelace_symbolics::substitute(expression, variable, &value_expr(value));
    bound_eval(&bound, PRECISION).map_err(map_lang_error)
}

/// The five observation classes a falsifier may draw from one sample.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ObservationClass {
    /// Both sides evaluated and the difference is inside the tolerance.
    Agree,
    /// Both sides evaluated and the difference reaches the tolerance.
    Disagree,
    /// The left side is not defined at this input.
    UndefinedLeft,
    /// The right side is not defined at this input.
    UndefinedRight,
    /// The sample hit a resource budget the product refuses.
    BudgetOrRefusal,
}

impl ObservationClass {
    /// The enum spelling the text form stores.
    pub(crate) fn name(self) -> &'static str {
        match self {
            ObservationClass::Agree => "Agree",
            ObservationClass::Disagree => "Disagree",
            ObservationClass::UndefinedLeft => "UndefinedLeft",
            ObservationClass::UndefinedRight => "UndefinedRight",
            ObservationClass::BudgetOrRefusal => "BudgetOrRefusal",
        }
    }

    /// Parses the enum spelling, refusing an unknown name.
    pub(crate) fn from_name(name: &str) -> Result<ObservationClass, String> {
        match name {
            "Agree" => Ok(ObservationClass::Agree),
            "Disagree" => Ok(ObservationClass::Disagree),
            "UndefinedLeft" => Ok(ObservationClass::UndefinedLeft),
            "UndefinedRight" => Ok(ObservationClass::UndefinedRight),
            "BudgetOrRefusal" => Ok(ObservationClass::BudgetOrRefusal),
            other => Err(format!("unknown observation class '{other}'.")),
        }
    }
}

/// What one side of one sample did. Threw is deliberately not an observation class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SideOutcome {
    /// The side produced a value.
    Evaluated,
    /// The kernel's typed domain refusal.
    Undefined,
    /// A typed resource refusal.
    Refused,
    /// Any other failure: a defect.
    Threw,
}

impl SideOutcome {
    /// The enum spelling the text form stores.
    pub(crate) fn name(self) -> &'static str {
        match self {
            SideOutcome::Evaluated => "Evaluated",
            SideOutcome::Undefined => "Undefined",
            SideOutcome::Refused => "Refused",
            SideOutcome::Threw => "Threw",
        }
    }

    /// Parses the enum spelling, refusing an unknown name.
    pub(crate) fn from_name(name: &str) -> Result<SideOutcome, String> {
        match name {
            "Evaluated" => Ok(SideOutcome::Evaluated),
            "Undefined" => Ok(SideOutcome::Undefined),
            "Refused" => Ok(SideOutcome::Refused),
            "Threw" => Ok(SideOutcome::Threw),
            other => Err(format!("unknown side outcome '{other}'.")),
        }
    }
}

/// The one tolerance a comparison uses, as a kernel value plus the text it printed from.
#[derive(Clone, Debug)]
pub(crate) struct ComparisonTolerance {
    /// The tolerance as a kernel value.
    pub(crate) value: Num,
    /// The tolerance rendered deterministically.
    pub(crate) text: String,
}

impl ComparisonTolerance {
    /// The gate's historical tolerance: 1e-11, rendered as it is always rendered.
    pub(crate) fn ten_to_the_minus_eleven() -> ComparisonTolerance {
        ComparisonTolerance {
            value: Num::Exact(rational(1, 100_000_000_000)),
            text: "1/100000000000".to_string(),
        }
    }

    /// An exact rational tolerance.
    pub(crate) fn of_rational(tolerance: Rat) -> ComparisonTolerance {
        ComparisonTolerance {
            text: rat_text(&tolerance),
            value: Num::Exact(tolerance),
        }
    }
}

/// One side's outcome plus whatever it has to say for itself.
#[derive(Clone, Debug)]
pub(crate) struct SideObservation {
    /// What the side did.
    pub(crate) outcome: SideOutcome,
    /// The value it produced, when it produced one.
    pub(crate) value: Option<Num>,
    /// The failure message, when it failed.
    pub(crate) detail: String,
}

impl SideObservation {
    /// True for the kernel's typed domain refusal.
    pub(crate) fn is_undefined(&self) -> bool {
        self.outcome == SideOutcome::Undefined
    }

    /// True for a typed resource refusal.
    pub(crate) fn is_refused(&self) -> bool {
        self.outcome == SideOutcome::Refused
    }

    /// True for an unexpected failure.
    pub(crate) fn threw(&self) -> bool {
        self.outcome == SideOutcome::Threw
    }
}

/// The full result of classifying one sample.
#[derive(Clone, Debug)]
pub(crate) struct ClassifiedObservation {
    /// The class the falsifier branches on.
    pub(crate) class: ObservationClass,
    /// The left side's outcome.
    pub(crate) left: SideObservation,
    /// The right side's outcome.
    pub(crate) right: SideObservation,
    /// The measured difference, rendered deterministically.
    pub(crate) delta_text: Option<String>,
}

impl ClassifiedObservation {
    /// True when this sample is a counterexample to the identity under test.
    pub(crate) fn is_counterexample(&self) -> bool {
        self.class == ObservationClass::Disagree
    }

    /// True when the sample revealed a defect rather than a mathematical fact.
    pub(crate) fn is_defect(&self) -> bool {
        self.left.threw() || self.right.threw()
    }
}

/// The classifier: evaluates both sides under one binding, classifies each side, then folds the
/// two outcomes into one class with an explicit precedence so a defect can never be masked.
pub(crate) struct ObservationClassifier;

impl ObservationClassifier {
    /// Evaluates and classifies one sample at one input.
    pub(crate) fn classify(
        lhs: &Expr,
        rhs: &Expr,
        variable: &str,
        value: &Num,
        tolerance: &ComparisonTolerance,
    ) -> ClassifiedObservation {
        Self::classify_with(lhs, rhs, variable, value, tolerance, &default_eval)
    }

    /// Classifies with a caller-supplied evaluator, so the defect path is reachable in a test.
    pub(crate) fn classify_with<F>(
        lhs: &Expr,
        rhs: &Expr,
        variable: &str,
        value: &Num,
        tolerance: &ComparisonTolerance,
        evaluator: &F,
    ) -> ClassifiedObservation
    where
        F: Fn(&Expr, &str, &Num) -> Result<Num, EvalFailure>,
    {
        let left = evaluate_side(lhs, variable, value, evaluator);
        let right = evaluate_side(rhs, variable, value, evaluator);

        // Precedence, stated once:
        //   1. a typed resource refusal outranks everything (the sample never got to run);
        //   2. then definedness, left before right;
        //   3. only when both sides produced a value does the comparison decide.
        if left.is_refused() || right.is_refused() {
            return ClassifiedObservation {
                class: ObservationClass::BudgetOrRefusal,
                left,
                right,
                delta_text: None,
            };
        }
        if left.is_undefined() {
            return ClassifiedObservation {
                class: ObservationClass::UndefinedLeft,
                left,
                right,
                delta_text: None,
            };
        }
        if right.is_undefined() {
            return ClassifiedObservation {
                class: ObservationClass::UndefinedRight,
                left,
                right,
                delta_text: None,
            };
        }
        if left.threw() || right.threw() {
            return ClassifiedObservation {
                class: ObservationClass::BudgetOrRefusal,
                left,
                right,
                delta_text: None,
            };
        }

        let (agrees, delta_text) = match Self::agrees(
            left.value.as_ref().expect("a value"),
            right.value.as_ref().expect("a value"),
            tolerance,
        ) {
            Ok(pair) => pair,
            Err(_) => (false, "|delta| unavailable".to_string()),
        };
        ClassifiedObservation {
            class: if agrees {
                ObservationClass::Agree
            } else {
                ObservationClass::Disagree
            },
            left,
            right,
            delta_text: Some(delta_text),
        }
    }

    /// |lhs - rhs| against the stated tolerance; a complex difference is decided on its exact
    /// squared magnitude rather than through a square root.
    pub(crate) fn agrees(
        left: &Num,
        right: &Num,
        tolerance: &ComparisonTolerance,
    ) -> Result<(bool, String), EvalFailure> {
        let delta = left
            .sub(right, PRECISION)
            .map_err(|error| EvalFailure::Threw(format!("LangError: {}", error.message)))?;
        if let Num::Complex(complex) = &delta {
            let squared = Num::Real(complex.magnitude_squared());
            let tolerance_squared = tolerance
                .value
                .mul(&tolerance.value, PRECISION)
                .map_err(|error| EvalFailure::Threw(format!("LangError: {}", error.message)))?;
            let agrees = num_cmp(&squared, &tolerance_squared) == Some(Ordering::Less);
            return Ok((agrees, format!("|delta|^2 = {}", num_text(&squared))));
        }
        let absolute = num_abs(&delta);
        let agrees = num_cmp(&absolute, &tolerance.value) == Some(Ordering::Less);
        Ok((agrees, format!("|delta| = {}", num_text(&absolute))))
    }

    /// Whether a failure is one of the typed resource refusals. Checked by kind, never by message.
    pub(crate) fn is_typed_resource_refusal(failure: &EvalFailure) -> bool {
        matches!(failure, EvalFailure::Refused(_))
    }
}

/// Evaluates one side and turns its result into an observation.
fn evaluate_side<F>(
    expression: &Expr,
    variable: &str,
    value: &Num,
    evaluator: &F,
) -> SideObservation
where
    F: Fn(&Expr, &str, &Num) -> Result<Num, EvalFailure>,
{
    match evaluator(expression, variable, value) {
        Ok(result) => SideObservation {
            outcome: SideOutcome::Evaluated,
            value: Some(result),
            detail: String::new(),
        },
        Err(failure) => SideObservation {
            outcome: failure.side(),
            value: None,
            detail: failure.describe(),
        },
    }
}
