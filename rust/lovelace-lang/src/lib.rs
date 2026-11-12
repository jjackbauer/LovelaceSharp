//! The Lovelace scripting language: values, the tree-walking interpreter and the engine facade.
//!
//! Rendering is the contract: every value renders with its documented display form and reports a
//! kind, and the engine reports errors as a message plus a UTF-16 position into the source.

// LangError carries its ErrorSignature inline by design: it is the wire-protocol error payload
// (message, code, category, position, signature). Boxing it would touch construction and matching
// across the envelope path for a performance heuristic, so the restructure is deferred rather than
// risk the frozen wire contract.
#![allow(clippy::result_large_err)]
#![allow(clippy::needless_range_loop)]

pub mod argument_contract;
pub mod builtins;
mod cancellable;
pub mod complex;
pub mod complex_math;
pub mod context;
pub mod diagnostic;
pub mod display_format;
pub mod docs_sync;
pub mod evaluation_boundary;
pub mod fixed_dsp;
pub mod help;
pub mod lexer;
pub mod mathir;
pub mod modus;
pub mod name_suggestions;
pub mod numeric;
pub mod plot_file;
pub mod plot_surface;
pub mod print_budget;
pub mod real_literal_route;
pub mod registry;
pub mod signals;
pub mod structural_equality;
pub mod structured;
pub mod syntax;
mod unicode;
pub mod value_widen;

#[cfg(test)]
mod round95_engine_singletons;

/// The cooperative cancellation token every long-running kernel polls. Re-exported so a host can
/// arm a deadline at the same seam the engine does (see `Engine::set_cancel_after`).
pub mod cancel {
    pub use lovelace_symbolics::cancel::*;
}

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use lovelace_array::{Field, NdArray};
use lovelace_int::Int;
use lovelace_nat::Nat;
use lovelace_real::Real;
use lovelace_symbolics::Expr as Sym;

use syntax::{Expr, IndexArg, InterpPart, Parser, Stmt};

/// The call-site signature an argument refusal was measured against.
///
/// The wire names are the reference's CallSignatureDto fields; a field is None when the failure
/// does not have it, and the runner omits it rather than publishing a placeholder.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ErrorSignature {
    /// The builtin whose call site was refused.
    pub function: String,
    /// The declared parameter names.
    pub parameters: Option<Vec<String>>,
    /// The declared minimum argument count.
    pub expected_min: Option<usize>,
    /// The declared maximum argument count (None when variadic).
    pub expected_max: Option<usize>,
    /// Whether extra trailing arguments are accepted.
    pub variadic: Option<bool>,
    /// The argument count the call supplied.
    pub actual: Option<usize>,
    /// The 1-based position of the offending argument, when one is named.
    pub position: Option<usize>,
    /// The declared parameter name at that position.
    pub parameter: Option<String>,
    /// What that position requires, as prose.
    pub expected_shape: Option<String>,
    /// The kind that actually arrived.
    pub arrived_kind: Option<String>,
}

/// An evaluation failure: a message and a UTF-16 code-unit position.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LangError {
    /// The message, a complete sentence ending with a period.
    pub message: String,
    /// UTF-16 code-unit offset into the source.
    pub position: usize,
    /// The stable wire code of the failure (the published error taxonomy).
    pub code: &'static str,
    /// The stable wire category of the failure.
    pub category: &'static str,
    /// The call-site signature, when the failure is an argument refusal.
    pub signature: Option<ErrorSignature>,
    /// The closest registered names, for an unresolved-name failure.
    pub suggestions: Vec<String>,
    /// Syntax notes that apply to an unresolved name.
    pub hints: Vec<String>,
    /// How many names the engine registers, for an unresolved-name failure.
    pub registry_size: Option<usize>,
}

impl LangError {
    /// Builds an error without a position.
    pub fn at(message: impl Into<String>, position: usize) -> LangError {
        LangError {
            message: message.into(),
            position,
            code: "InvalidOperation",
            category: "DomainError",
            signature: None,
            suggestions: Vec::new(),
            hints: Vec::new(),
            registry_size: None,
        }
    }

    /// Builds an error at position zero.
    pub fn plain(message: impl Into<String>) -> LangError {
        LangError::at(message, 0)
    }

    /// Builds an error at a position with an explicit code and category.
    pub fn coded(
        message: impl Into<String>,
        position: usize,
        code: &'static str,
        category: &'static str,
    ) -> LangError {
        LangError {
            message: message.into(),
            position,
            code,
            category,
            signature: None,
            suggestions: Vec::new(),
            hints: Vec::new(),
            registry_size: None,
        }
    }

    /// A depth-budget refusal: a valid input (or value) past one of the published budgets.
    pub fn depth(stage: &str, depth: usize, limit: usize, measure: &str) -> LangError {
        LangError::coded(
            format!(
                "{stage} depth {depth} exceeds the maximum supported {measure} depth of {limit}. Rewrite the input with less nesting (the engine refuses it before a recursive walk can exhaust the process stack)."
            ),
            0,
            "DepthExceeded",
            "BudgetExceeded",
        )
    }

    /// The same failure, placed at a source position.
    pub fn with_position(mut self, position: usize) -> LangError {
        self.position = position;
        self
    }

    /// The same failure, carrying the call-site signature it was measured against.
    pub fn with_signature(mut self, signature: ErrorSignature) -> LangError {
        self.signature = Some(signature);
        self
    }
}

impl std::fmt::Display for LangError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for LangError {}

/// The reference's cancellation message (`EvaluationCancelledException`), published as the
/// failure's message so a consumer that only reads `message` sees the same sentence both runners
/// use.
pub const CANCELLED_MESSAGE: &str = "the evaluation was cancelled by the caller.";

/// The structured cancellation failure: the deadline stopped the run, which is neither a success
/// nor a domain failure. Position is zero here and is supplied by the statement that was running
/// (the evaluation fills it in, exactly as it does for any other positionless failure).
pub fn cancelled_error() -> LangError {
    LangError::coded(CANCELLED_MESSAGE, 0, "Cancelled", "BudgetExceeded")
}

/// A runtime value.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// A natural number.
    Natural(Nat),
    /// A signed integer.
    Integer(Int),
    /// A real number.
    Real(Real),
    /// A real number whose published digits were clamped by a builtin's own computation cap:
    /// the value, the reason digits were dropped (`"digit-cap"`) and the budget that stopped
    /// it. It is a Real in every other respect — kind, display, arithmetic — and the extra
    /// fields exist so the wire can say the count was not honoured in full instead of
    /// publishing a value indistinguishable from one it was.
    RealClamped(Real, &'static str, u64),
    /// A boolean.
    Boolean(bool),
    /// Text.
    Text(String),
    /// A rank-1 array.
    Vector(Vec<Value>),
    /// A rank >= 2 array.
    Array(NdArray<Value>),
    /// A user-defined function.
    Function(String),
    /// No value.
    Void,
    /// A record with named fields.
    Record(String, Vec<(String, Value)>),
    /// A complex number (re, im).
    Complex(Real, Real),
    /// A symbolic expression.
    Symbolic(Sym),
    /// A domain such as real or complex.
    Domain(String),
    /// A declared enumeration variant: (type name, variant name), e.g. ("SolveStatus", "Solved").
    Enum(String, String),
}

impl Value {
    /// The kind name published in typed renderings.
    pub fn kind_name(&self) -> &str {
        match self {
            Value::Natural(_) => "Natural",
            Value::Integer(_) => "Integer",
            Value::Real(_) | Value::RealClamped(_, _, _) => "Real",
            Value::Boolean(_) => "Boolean",
            Value::Text(_) => "Text",
            Value::Vector(_) => "Vector",
            Value::Array(_) => "Array",
            Value::Function(_) => "Function",
            Value::Void => "Void",
            Value::Complex(_, _) => "Complex",
            Value::Symbolic(_) => "Symbolic",
            Value::Domain(_) => "Domain",
            Value::Enum(_, _) => "Enum",
            // A record's kind is Record; its own type name is published in the structured form.
            Value::Record(_, _) => "Record",
        }
    }

    /// The display form, with the reference's ASCII symbolic rendering.
    pub fn display(&self) -> String {
        self.display_with_unicode(false)
    }

    /// The display form under the reference's Unicode symbolic rendering (∞ π ≤ ≥ ≠).
    pub fn display_unicode(&self) -> String {
        self.display_with_unicode(true)
    }

    /// The display form, selecting the reference's symbolic rendering. Every kind other than
    /// Symbolic is identical in both forms; an array or record threads the choice through its
    /// elements (the reference's `ValueFormatter.Format(value, unicode)`).
    pub fn display_with_unicode(&self, unicode: bool) -> String {
        match self {
            Value::Natural(n) => n.to_decimal(),
            Value::Integer(i) => i.to_decimal(),
            Value::Real(r) | Value::RealClamped(r, _, _) => r.to_string(),
            Value::Boolean(b) => if *b { "True" } else { "False" }.to_string(),
            Value::Text(t) => t.clone(),
            Value::Vector(items) => {
                let mut s = String::from("[");
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        s.push_str(", ");
                    }
                    if i >= MAX_DISPLAY_NODES {
                        s.push_str("...");
                        break;
                    }
                    s.push_str(&v.display_with_unicode(unicode));
                }
                s.push(']');
                s
            }
            Value::Array(a) => render_array(a, unicode),
            Value::Function(name) => format!("<function {}>", name),
            Value::Void => String::new(),
            Value::Symbolic(e) => {
                if unicode {
                    crate::unicode::symbolic(e)
                } else {
                    e.to_string()
                }
            }
            Value::Domain(name) => name.clone(),
            Value::Enum(_, name) => name.clone(),
            Value::Complex(re, im) => {
                // The reference Complex.ToString: a zero part is omitted entirely, so a real
                // value carried as a Complex (e.g. a DFT element) still displays as "6".
                if im.is_zero() {
                    re.to_string()
                } else if re.is_zero() {
                    format!("{}i", im)
                } else {
                    let sign = if im.cmp(&Real::zero()) == std::cmp::Ordering::Less {
                        "-"
                    } else {
                        "+"
                    };
                    format!("{} {} {}i", re, sign, im.abs())
                }
            }
            Value::Record(name, fields) => {
                let mut s = String::from(name.as_str());
                s.push('(');
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        s.push_str(", ");
                    }
                    s.push_str(k);
                    s.push_str(": ");
                    s.push_str(&v.display_with_unicode(unicode));
                }
                s.push(')');
                s
            }
        }
    }

    /// The display form after refusing a value deeper than the published value budget.
    ///
    /// The reference's `ValueFormatter.Format` walks the value recursively and refuses with
    /// `InputDepthExceededException` before the walk can exhaust the stack. This is that fallible
    /// entry point: a value at the budget renders, one past it is refused with the same typed
    /// message instead of overflowing.
    pub fn try_display(&self) -> Result<String, LangError> {
        check_value_depth(self)?;
        Ok(self.display())
    }

    /// The typed counterpart of [`Value::try_display`].
    pub fn try_typed(&self) -> Result<String, LangError> {
        check_value_depth(self)?;
        Ok(self.typed())
    }

    /// The typed form: display plus the kind in parentheses, ASCII symbolic rendering.
    pub fn typed(&self) -> String {
        self.typed_with_unicode(false)
    }

    /// The typed form under the selected symbolic rendering.
    ///
    /// A record publishes its OWN type name (SolveResult, Inspection, …), and an enum publishes
    /// its bare member name — exactly the reference formatter's rules.
    pub fn typed_with_unicode(&self, unicode: bool) -> String {
        match self {
            Value::Void => "(void)".to_string(),
            // Text and enum values are published without a kind suffix, matching the reference.
            Value::Text(_) => self.display_with_unicode(unicode),
            Value::Enum(_, name) => name.clone(),
            Value::Record(name, _) => {
                format!("{} ({})", self.display_with_unicode(unicode), name)
            }
            _ => format!(
                "{} ({})",
                self.display_with_unicode(unicode),
                self.kind_name()
            ),
        }
    }

    /// True when this is one of the numeric kinds.
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            Value::Natural(_) | Value::Integer(_) | Value::Real(_) | Value::RealClamped(_, _, _)
        )
    }
}

/// The greatest number of display NODES (scalars, plus one per nested group) one array or
/// vector rendering will produce. The shape of an array is user data: `zeros(9223372036854775807,
/// 0)` legitimately holds zero elements but its display would otherwise iterate 2^63-1 empty rows
/// and never return. The budget bounds the RENDERING, never the value: the array is still
/// constructible, `shape`/`numel` still answer it, and a truncated rendering ends with `...`
/// (inside the brackets) so the abbreviation is visible. Normal (small) values render byte for
/// byte as before.
pub(crate) const MAX_DISPLAY_NODES: usize = 100_000;

struct DisplayBudget {
    remaining: usize,
}

impl DisplayBudget {
    fn new() -> DisplayBudget {
        DisplayBudget {
            remaining: MAX_DISPLAY_NODES,
        }
    }

    /// Consumes one node; false when the rendering must stop.
    fn spend(&mut self) -> bool {
        if self.remaining == 0 {
            return false;
        }
        self.remaining -= 1;
        true
    }
}

fn render_array(a: &NdArray<Value>, unicode: bool) -> String {
    fn rec(
        a: &NdArray<Value>,
        shape: &[usize],
        dim: usize,
        idx: &mut Vec<usize>,
        budget: &mut DisplayBudget,
        out: &mut String,
        unicode: bool,
    ) {
        if dim == shape.len() {
            if let Some(v) = a.get(idx) {
                out.push_str(&v.display_with_unicode(unicode));
            }
            return;
        }
        out.push('[');
        for i in 0..shape[dim] {
            if i > 0 {
                out.push_str(", ");
            }
            // The loop bound is a declared dimension, not the stored element count: a zero-width
            // inner dimension still makes the outer loop user-sized. Spend BEFORE descending so a
            // dimension wider than the budget cannot spin.
            if !budget.spend() {
                out.push_str("...");
                break;
            }
            idx.push(i);
            rec(a, shape, dim + 1, idx, budget, out, unicode);
            idx.pop();
        }
        out.push(']');
    }
    let shape = a.shape().to_vec();
    if shape.is_empty() {
        return a
            .get(&[])
            .map(|v| v.display_with_unicode(unicode))
            .unwrap_or_default();
    }
    let mut out = String::new();
    rec(
        a,
        &shape,
        0,
        &mut Vec::new(),
        &mut DisplayBudget::new(),
        &mut out,
        unicode,
    );
    out
}

impl Field for Value {
    fn zero() -> Self {
        Value::Natural(Nat::zero())
    }
    fn one() -> Self {
        Value::Natural(Nat::from_u64(1))
    }
    fn add(&self, other: &Self) -> Self {
        builtins::add(self, other).unwrap_or_else(|_| Value::Void)
    }
    fn sub(&self, other: &Self) -> Self {
        builtins::sub(self, other).unwrap_or_else(|_| Value::Void)
    }
    fn mul(&self, other: &Self) -> Self {
        builtins::mul(self, other).unwrap_or_else(|_| Value::Void)
    }
    fn div(&self, other: &Self) -> Self {
        builtins::div(self, other, 1000).unwrap_or_else(|_| Value::Void)
    }
    fn neg(&self) -> Self {
        builtins::neg(self).unwrap_or_else(|_| Value::Void)
    }
    fn is_zero(&self) -> bool {
        match self {
            Value::Natural(n) => n.is_zero(),
            Value::Integer(i) => i.is_zero(),
            Value::Real(r) | Value::RealClamped(r, _, _) => r.is_zero(),
            _ => false,
        }
    }
    fn from_i64(v: i64) -> Self {
        if v < 0 {
            Value::Integer(Int::from_i64(v))
        } else {
            Value::Natural(Nat::from_u64(v as u64))
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for Value {}

impl Ord for Value {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Value::Natural(a), Value::Natural(b)) => a.cmp(b),
            (Value::Integer(a), Value::Integer(b)) => a.cmp(b),
            (Value::Real(a), Value::Real(b)) => a.cmp(b),
            _ => {
                if let (Some(a), Some(b)) = (builtins::as_real(self), builtins::as_real(other)) {
                    return a.cmp(&b);
                }
                self.display().cmp(&other.display())
            }
        }
    }
}

/// One timed top-level statement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpTiming {
    /// The statement's offset in the caller's source (UTF-16 code units).
    pub position: usize,
    /// The kind of the value the statement produced.
    pub result_kind: String,
    /// Whether the statement wrote print output.
    pub has_output: bool,
    /// How long the statement took. One measurement drives both the human `elapsed_display`
    /// and the machine `elapsed_scale` pair, so the two can never disagree (the reference's
    /// `OperationTiming.Elapsed`).
    pub elapsed: Duration,
}

impl OpTiming {
    /// The reference's `OperationTiming.ElapsedDisplay`: the elapsed time rendered with an
    /// auto-selected unit (ns/µs/ms/s/min/h), at most two decimals, trailing zeros trimmed.
    pub fn elapsed_display(&self) -> String {
        let (value, unit) = self.elapsed_scale();
        if unit == "ns" {
            format!("{value:.0} {unit}")
        } else {
            format!("{} {}", format_two_decimals(value), unit)
        }
    }

    /// The reference's `OperationTiming.ElapsedScale`: the `(value, unit)` pair behind
    /// `elapsed_display`. The SAME selector drives both forms, so an agent never parses a
    /// suffix to recover the unit.
    pub fn elapsed_scale(&self) -> (f64, &'static str) {
        scale_duration(self.elapsed)
    }
}

/// `Timing.Scale`: the largest unit whose value is at least one whole unit, with a fractional
/// value rounded to two decimals. Nanoseconds round to a whole number, as the reference's
/// `Format` asks.
fn scale_duration(elapsed: Duration) -> (f64, &'static str) {
    let total_nanoseconds = elapsed.as_nanos() as f64;
    if total_nanoseconds < 1_000.0 {
        return (total_nanoseconds.round(), "ns");
    }
    let total_microseconds = total_nanoseconds / 1_000.0;
    if total_microseconds < 1_000.0 {
        return (round_two(total_microseconds), "µs");
    }
    let total_milliseconds = total_nanoseconds / 1_000_000.0;
    if total_milliseconds < 1_000.0 {
        return (round_two(total_milliseconds), "ms");
    }
    let total_seconds = total_nanoseconds / 1_000_000_000.0;
    if total_seconds < 60.0 {
        return (round_two(total_seconds), "s");
    }
    let total_minutes = total_seconds / 60.0;
    if total_minutes < 60.0 {
        return (round_two(total_minutes), "min");
    }
    (round_two(total_seconds / 3_600.0), "h")
}

/// .NET's `Math.Round(value, 2)` (banker's rounding at the midpoint), which is what the
/// reference's `Timing.Scale` calls.
fn round_two(value: f64) -> f64 {
    let scaled = value * 100.0;
    let floor = scaled.floor();
    let fraction = scaled - floor;
    let rounded = if (fraction - 0.5).abs() < f64::EPSILON {
        // Midpoint: choose the even integer.
        if (floor as i64) % 2 == 0 {
            floor
        } else {
            floor + 1.0
        }
    } else {
        scaled.round()
    };
    rounded / 100.0
}

/// Two decimals at most, trailing zeros trimmed (the reference's `"0.##"` format).
fn format_two_decimals(value: f64) -> String {
    let mut text = format!("{value:.2}");
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
    text
}

/// The captured plot of the last plot call.
#[derive(Clone, Debug)]
pub struct PlotCapture {
    /// Absolute or relative path of the written SVG.
    pub path: String,
    /// The plot title.
    pub title: String,
    /// The SVG document.
    pub svg: String,
}

/// One variable in a [`StateSnapshot`]: its name, published kind and display rendering.
///
/// The reference's `StateVariable` record (`Name`, `Kind`, `Display`). Built once, when
/// `capture_state` runs, so later engine mutations cannot change it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateVariable {
    /// The variable name as written in source.
    pub name: String,
    /// The published kind name (`Natural`, `Real`, `Vector`, …).
    pub kind: String,
    /// The display rendering at the engine's display precision.
    pub display: String,
}

/// One function in a [`StateSnapshot`], or the definition carried by a
/// [`FunctionDefined`] event.
///
/// The reference's `StateFunction` record (`Name`, `Parameters`, `IsBuiltin`, `Span`,
/// `Plugin`). The Rust engine does not track a source span on a user definition, so the span
/// field of the reference record has no counterpart here; every other field is published.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateFunction {
    /// The function name as written in source.
    pub name: String,
    /// The ordered parameter names.
    pub parameters: Vec<String>,
    /// The smallest accepted argument count. The declaration TRAVELS with the function (the
    /// reference's `FunctionDefinition.MinArity`), so the one call-site validator can enforce
    /// what the descriptor declares instead of reading a second, private table.
    pub min_arity: usize,
    /// Whether extra trailing arguments are accepted (the reference's
    /// `FunctionDefinition.Variadic`): an upper bound of infinity rather than the parameter
    /// count.
    pub variadic: bool,
    /// True for a builtin (native or plugin) and false for a user definition.
    pub is_builtin: bool,
    /// The plugin that registered the builtin, when one did.
    pub plugin: Option<String>,
}

/// An immutable capture of the engine's variables and functions, with the revision it was
/// taken at.
///
/// The reference's `StateSnapshot`: a host takes one capture to read a consistent view and
/// compares `revision` later to detect staleness. The maps are owned copies sorted by name, so
/// a mutating engine call after the capture cannot alter the snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateSnapshot {
    /// The engine revision when the snapshot was taken.
    pub revision: u64,
    /// The captured variables, keyed and sorted by name.
    pub variables: BTreeMap<String, StateVariable>,
    /// The captured functions, keyed and sorted by name.
    pub functions: BTreeMap<String, StateFunction>,
}

/// Event payload for a global variable change: define, reassign, or remove.
///
/// The reference's `VariableChangedEventArgs`. `removed` distinguishes a removal, whose
/// `value` is the value that was removed, from a definition or reassignment.
#[derive(Clone, Debug, PartialEq)]
pub struct VariableChanged {
    /// The variable name.
    pub name: String,
    /// The value written, or the value removed when `removed` is true.
    pub value: Value,
    /// True when the event reports a removal.
    pub removed: bool,
}

/// Event payload for a function definition.
///
/// The reference's `FunctionDefinedEventArgs`; the definition's name and declared parameters
/// are the observable identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionDefined {
    /// The definition that was registered.
    pub definition: StateFunction,
}

/// A subscriber to a [`VariableChanged`] event.
type VariableChangedListener = Box<dyn FnMut(&VariableChanged) + Send>;

/// A subscriber to a [`FunctionDefined`] event.
type FunctionDefinedListener = Box<dyn FnMut(&FunctionDefined) + Send>;

/// One interval endpoint: the value and whether the endpoint is included.
type IntervalEnd = (lovelace_symbolics::Rat, bool);
/// One optional endpoint of an assumption interval.
type Bound = Option<IntervalEnd>;

/// The engine: variables, functions, precision and captured output.
pub struct Engine {
    vars: HashMap<String, Value>,
    funcs: HashMap<String, (Vec<String>, Stmt)>,
    precision: usize,
    /// True once the precision knob was changed from its default (mirrors the reference
    /// interpreter's `PrecisionExplicitlySet`, which plugin builtins read).
    precision_explicit: bool,
    /// The DISPLAY bound when a host pins it below the computation bound (the reference's
    /// `SuiteEngine.DisplayDecimalPlaces`). `None` keeps the historical coupling: 100 until
    /// `setprecision` moves it, then the computation bound.
    display_precision_override: Option<usize>,
    output: Vec<String>,
    /// Whether display formatting prefers the Unicode math glyphs (∞ π ≤ ≥ ≠), the
    /// reference's `SuiteEngine.UnicodeOutput` knob. ASCII is the default.
    unicode_output_enabled: bool,
    plot: Option<PlotCapture>,
    plot_dir: String,
    plot_file: String,
    revision: u64,
    timings: Vec<OpTiming>,
    /// The live lexical chain below the global store: one map per open block, loop or user-frame
    /// scope (the reference's `Scope` chain). Lookup walks it innermost-first and falls back to
    /// the globals; assignment updates the nearest existing binding, a definition binds in the
    /// innermost scope.
    scopes: Vec<HashMap<String, Value>>,
    /// The interpreter's LIVE evaluation depth in budget units (the reference's
    /// `InputDepth.MaxEvaluationDepth` walk): one per statement executed, one per descending
    /// expression node (a leaf and a call argument cost nothing), and one frame's units per user
    /// function level. Every charge is undone when the node or statement finishes, so the counter
    /// measures nesting rather than width.
    eval_depth: usize,
    /// The depth at which the current top-level statement entered user code, minus the frame's own
    /// units: the zero the budget compares against.
    eval_base: usize,
    /// Whether the current top-level statement has already found its evaluation base.
    eval_base_established: bool,
    /// When non-zero, expression charging is suspended: a call argument's subtree is charged
    /// nothing, exactly as the reference's argument routing leaves it uncharged.
    free_charge: usize,
    /// The caller's deadline, when one was armed for the next evaluation (--cancel-after).
    cancel_after: Option<Duration>,
    /// The live cancellation flag the host may set while an evaluation is in flight (the desktop
    /// Cancel button). It is passed into the per-evaluation cancellation scope beside the
    /// deadline, so one long kernel stops without waiting for a statement boundary.
    cancel_flag: Option<Arc<AtomicBool>>,
    /// Whether a kernel observed the armed deadline during the last evaluation.
    cancelled: bool,
    /// The top-level statement that is running, so a cancellation raised at the end of a kernel
    /// names the statement the deadline passed inside.
    current_position: usize,
    /// The last top-level statement that RETURNED. The reference positions a cancellation
    /// diagnostic at the last entry of its timings list, which is this statement when the deadline
    /// is observed at a statement boundary (its published position for RUN-010) and the stopped
    /// statement when it is observed inside a kernel.
    last_completed_position: usize,
    positive_symbols: Vec<String>,
    nonzero_symbols: Vec<String>,
    assumption_texts: Vec<String>,
    /// The SAME session assumptions as expressions, so a consumer (solve, a family's
    /// conditions) can substitute a candidate value and decide the atom without parsing prose.
    assumption_atoms: Vec<Sym>,
    /// Domain assumptions per symbol: (name, "real" | "integer"). `assume_real(x)` restricts
    /// the session's domain, which is what decides whether a complex or fractional root is a
    /// solution of THIS session's equation.
    domain_assumptions: Vec<(String, String)>,
    bounds: Vec<(String, Bound, Bound)>,
    /// The session's assumption set: the tri-state kernel the assume builtins feed and merge,
    /// solve, simplify, simplify_full and inspect read. One set per engine, so two engines that
    /// run concurrently never answer each other's queries.
    assumptions: lovelace_symbolics::assumptions::AssumptionSet,
    /// The plugin seam: registered builtins, kernels, loaded plugin names and the optional
    /// plugin-instance assumption context (the reference's `ModusHost`).
    modus: modus::ModusHost,
    /// `None` keeps every advertised builtin available (the runner's behaviour). `Some` is a
    /// bare engine that exposes only core builtins plus the plugins it has loaded.
    enabled_plugins: Option<HashSet<String>>,
    /// Subscribers to the [`VariableChanged`] event, in subscription order.
    variable_listeners: Vec<VariableChangedListener>,
    /// Subscribers to the [`FunctionDefined`] event, in subscription order.
    function_listeners: Vec<FunctionDefinedListener>,
}

impl Default for Engine {
    fn default() -> Self {
        Engine::new()
    }
}

impl Engine {
    /// Creates an engine with the core builtins available.
    pub fn new() -> Engine {
        let dir = std::env::temp_dir().join("lovelace-plots");
        let _ = std::fs::create_dir_all(&dir);
        Engine {
            vars: HashMap::new(),
            funcs: HashMap::new(),
            precision: 1000,
            precision_explicit: false,
            display_precision_override: None,
            output: Vec::new(),
            unicode_output_enabled: false,
            plot: None,
            plot_dir: dir.to_string_lossy().to_string(),
            plot_file: "plot.svg".to_string(),
            revision: 0,
            timings: Vec::new(),
            scopes: Vec::new(),
            eval_depth: 0,
            eval_base: 0,
            eval_base_established: false,
            free_charge: 0,
            cancel_after: None,
            cancel_flag: None,
            cancelled: false,
            current_position: 0,
            last_completed_position: 0,
            positive_symbols: Vec::new(),
            nonzero_symbols: Vec::new(),
            assumption_texts: Vec::new(),
            assumption_atoms: Vec::new(),
            domain_assumptions: Vec::new(),
            bounds: Vec::new(),
            assumptions: lovelace_symbolics::assumptions::AssumptionSet::empty(),
            modus: modus::ModusHost::new(),
            enabled_plugins: None,
            variable_listeners: Vec::new(),
            function_listeners: Vec::new(),
        }
    }

    /// Creates a BARE engine: core builtins only, with the descriptors advertised by a plugin
    /// unavailable until that plugin is loaded. This is the Rust equivalent of the reference's
    /// `new SuiteEngine()`, which does not expose `fft` until `DspPlugin` is loaded. `Engine::new()`
    /// remains the runner's all-plugins-loaded engine, so no existing behaviour moves.
    pub fn new_bare() -> Engine {
        let mut engine = Engine::new();
        engine.enabled_plugins = Some(HashSet::new());
        engine
    }

    /// Loads a Modus plugin, stamping its registrations and enabling the builtins its descriptor
    /// advertises on a bare engine.
    pub fn load_plugin(&mut self, plugin: &mut dyn modus::ModusPlugin) -> Result<(), LangError> {
        self.modus.load(plugin)?;
        if let Some(enabled) = &mut self.enabled_plugins {
            enabled.insert(plugin.name().to_string());
        }
        Ok(())
    }

    /// Registers a host builtin directly (the reference's `SuiteEngine.RegisterBuiltin`).
    pub fn register_builtin<F>(
        &mut self,
        name: &str,
        parameters: &[&str],
        implementation: F,
    ) -> Result<(), LangError>
    where
        F: Fn(&modus::PluginArgs) -> Result<modus::Payload, modus::PluginError>
            + Send
            + Sync
            + 'static,
    {
        self.modus
            .register_builtin(name, parameters, implementation)?;
        self.raise_function_defined(StateFunction {
            name: name.to_string(),
            parameters: parameters.iter().map(|p| (*p).to_string()).collect(),
            min_arity: parameters.len(),
            variadic: false,
            is_builtin: true,
            plugin: self.modus.plugin_name_of(name).map(str::to_string),
        });
        Ok(())
    }

    /// Registers a descriptor-carrying host builtin.
    pub fn register_builtin_descriptor<F>(
        &mut self,
        descriptor: modus::BuiltinDescriptor,
        implementation: F,
    ) -> Result<(), LangError>
    where
        F: Fn(&modus::PluginArgs) -> Result<modus::Payload, modus::PluginError>
            + Send
            + Sync
            + 'static,
    {
        let definition = StateFunction {
            name: descriptor.name.clone(),
            min_arity: descriptor.min_arity.unwrap_or(descriptor.parameters.len()),
            variadic: descriptor.variadic,
            parameters: descriptor.parameters.clone(),
            is_builtin: true,
            plugin: self
                .modus
                .plugin_name_of(&descriptor.name)
                .map(str::to_string),
        };
        self.modus
            .register_builtin_descriptor(descriptor, implementation)?;
        self.raise_function_defined(definition);
        Ok(())
    }

    /// Registers an array-to-array host builtin.
    pub fn register_array_builtin<F>(
        &mut self,
        name: &str,
        implementation: F,
    ) -> Result<(), LangError>
    where
        F: Fn(&modus::PayloadArray) -> Result<modus::PayloadArray, modus::PluginError>
            + Send
            + Sync
            + 'static,
    {
        self.modus.register_array_builtin(name, implementation)
    }

    /// Registers an exact-field elementwise kernel.
    pub fn register_kernel<T: modus::KernelScalar>(
        &mut self,
        kernel: Box<dyn modus::FieldKernel<T>>,
    ) {
        self.modus.register_kernel(kernel);
    }

    /// Fallible kernel dispatch over the plugin host.
    pub fn try_dispatch_kernel<T: modus::KernelScalar>(
        &self,
        op: modus::ArrayOp,
        left: &[T],
        right: &[T],
        result: &mut [T],
    ) -> bool {
        self.modus.try_dispatch(op, left, right, result)
    }

    /// The plugin that registered a builtin: a host registration's plugin stamp, otherwise the
    /// static descriptor's plugin (core builtins carry none).
    pub fn plugin_name_of(&self, name: &str) -> Option<&str> {
        self.modus
            .plugin_name_of(name)
            .or_else(|| crate::registry::find(name).and_then(|info| info.plugin))
    }

    /// True when the name is available on this engine: core names always, plugin-advertised
    /// names only when the engine is all-plugins (`Engine::new`) or the plugin was loaded.
    fn plugin_enabled(&self, name: &str) -> bool {
        let Some(enabled) = &self.enabled_plugins else {
            return true;
        };
        match crate::registry::find(name).and_then(|info| info.plugin) {
            Some(plugin) => enabled.contains(plugin),
            None => true,
        }
    }

    /// The descriptor a host registration stored, when it carried metadata.
    pub fn registered_descriptor(&self, name: &str) -> Option<&modus::BuiltinDescriptor> {
        self.modus.descriptor_of(name)
    }

    /// True when the engine resolves the name as a user function or a registered builtin.
    pub fn has_function(&self, name: &str) -> bool {
        self.funcs.contains_key(name)
            || self.modus.contains(name)
            || crate::registry::find(name).is_some()
    }

    /// Registers the plugin-instance assumption context whose caller scopes the engine honours.
    pub fn register_assumption_context(&mut self, context: std::sync::Arc<context::Context>) {
        self.modus.register_assumption_context(context);
    }

    /// Sets the computation and display precision (decimal places).
    pub fn set_precision(&mut self, decimals: usize) {
        self.precision = decimals.max(1);
        self.precision_explicit = true;
    }

    /// The current precision.
    pub fn precision(&self) -> usize {
        self.precision
    }

    /// The DISPLAY precision: the fractional digit count a rendered non-periodic value shows.
    /// The reference keeps two knobs (computation 1000, display 100) and `setprecision` moves
    /// both; until it is moved explicitly a constant such as `pi` renders at the display default.
    pub fn display_precision(&self) -> usize {
        if let Some(decimals) = self.display_precision_override {
            return decimals;
        }
        if self.precision_explicit {
            self.precision
        } else {
            100
        }
    }

    /// Pins the DISPLAY bound without moving the computation bound (the reference's
    /// `SuiteEngine.DisplayDecimalPlaces`). A host that asks for 1 100 stored digits can still
    /// render 100; the structured projection keeps publishing every digit the value stores, so the
    /// two bounds cannot make one value cross as two different payloads.
    pub fn set_display_precision(&mut self, decimals: usize) {
        self.display_precision_override = Some(decimals.max(1));
    }

    /// Sets the directory plots are written to.
    pub fn set_plot_dir(&mut self, dir: &str) {
        self.plot_dir = dir.to_string();
        let _ = std::fs::create_dir_all(&self.plot_dir);
    }

    /// Arms the caller's deadline for the NEXT evaluation: every kernel loop polls it and a run
    /// that reaches it stops with the structured `Cancelled` refusal. `None` disarms, and an
    /// un-budgeted evaluation is byte-for-byte the run it always was.
    pub fn set_cancel_after(&mut self, after: Option<Duration>) {
        self.cancel_after = after;
    }

    /// The armed deadline, if any.
    pub fn cancel_after(&self) -> Option<Duration> {
        self.cancel_after
    }

    /// Installs (or clears) the live cancellation flag the next evaluation polls.
    ///
    /// Unlike [`Engine::set_cancel_after`], which only bounds the run from the inside, this flag
    /// may be set by another thread WHILE the evaluation is running — it is the desktop Cancel
    /// button. The engine clones the handle into the per-evaluation cancellation scope, so
    /// setting it stops the current kernel at its next poll seam with the same structured
    /// `Cancelled` failure a spent deadline produces. `None` restores the deadline-only path.
    pub fn set_cancel_flag(&mut self, flag: Option<Arc<AtomicBool>>) {
        self.cancel_flag = flag;
    }

    /// Whether the deadline stopped the last evaluation (the token every kernel polls).
    pub fn was_cancelled(&self) -> bool {
        self.cancelled
    }

    /// The position of the last top-level statement that returned, or 0 when none did.
    pub fn last_completed_position(&self) -> usize {
        self.last_completed_position
    }

    /// Monotonic revision counter.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// The captured plot, if the last evaluation produced one.
    pub fn last_plot(&self) -> Option<PlotCapture> {
        self.plot.clone()
    }

    /// Clears the captured plot (the reference's `SuiteEngine.ResetPlotCapture`). The engine's
    /// plot directory and file name are untouched; only the `LastPlot` capture is forgotten.
    pub fn reset_plot_capture(&mut self) {
        self.plot = None;
    }

    /// Drains captured print output.
    pub fn take_output(&mut self) -> Vec<String> {
        std::mem::take(&mut self.output)
    }

    /// Renders a value in display form at the engine's DISPLAY precision (the reference's
    /// Real.DisplayDecimalPlaces: 100 until setprecision moves it). The scope covers the whole
    /// nested rendering, so a Real inside an array or record is capped too.
    pub fn format(&self, value: &Value) -> String {
        lovelace_real::with_display_decimal_places(self.display_precision(), || {
            value.display_with_unicode(self.unicode_output_enabled)
        })
    }

    /// Renders a value with its kind, under the same display-precision scope.
    pub fn format_typed(&self, value: &Value) -> String {
        lovelace_real::with_display_decimal_places(self.display_precision(), || {
            value.typed_with_unicode(self.unicode_output_enabled)
        })
    }

    /// Whether display formatting prefers the Unicode math glyphs (the reference's
    /// `SuiteEngine.UnicodeOutput`). ASCII is the default.
    pub fn unicode_output(&self) -> bool {
        self.unicode_output_enabled
    }

    /// Selects the Unicode math glyphs (∞ π ≤ ≥ ≠) for symbolic rendering, or ASCII when false.
    /// The change is formatting-only: values, equality and the wire protocol are untouched.
    pub fn set_unicode_output(&mut self, enabled: bool) {
        self.unicode_output_enabled = enabled;
    }

    /// Global variables as (name, typed rendering), sorted by name.
    pub fn variables(&self) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = self
            .vars
            .iter()
            .filter(|(k, _)| k.as_str() != "true" && k.as_str() != "false")
            .map(|(k, v)| (k.clone(), v.typed()))
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    /// Global variables as (name, value), sorted by name and excluding the true/false constants.
    pub fn variable_values(&self) -> Vec<(String, Value)> {
        let mut out: Vec<(String, Value)> = self
            .vars
            .iter()
            .filter(|(k, _)| k.as_str() != "true" && k.as_str() != "false")
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    /// One entry per top-level statement that ran, in statement order.
    pub fn operation_timings(&self) -> &[OpTiming] {
        &self.timings
    }

    /// User functions as (name, parameters), sorted by name.
    pub fn functions(&self) -> Vec<(String, Vec<String>)> {
        let mut out: Vec<(String, Vec<String>)> = self
            .funcs
            .iter()
            .map(|(k, (params, _))| (k.clone(), params.clone()))
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }
    /// Captures an immutable snapshot of the global variables and the function table, with the
    /// revision it was taken at.
    ///
    /// The Rust counterpart of the reference's `SuiteEngine.CaptureState`. Variables carry the
    /// same three fields as `StateVariable` (name, kind, display). Functions carry the same
    /// observable identity as `StateFunction` — name, declared parameters, builtin flag and the
    /// registering plugin — for the engine's user definitions, its Modus registrations and every
    /// builtin the engine currently exposes. The reference's `Span` has no Rust counterpart and
    /// is not published. The maps are owned copies, so later mutation cannot change them.
    pub fn capture_state(&self) -> StateSnapshot {
        let mut variables: BTreeMap<String, StateVariable> = BTreeMap::new();
        for (name, value) in &self.vars {
            // `true`/`false` are language constants, not globals (the `variables()` filter).
            if name == "true" || name == "false" {
                continue;
            }
            variables.insert(
                name.clone(),
                StateVariable {
                    name: name.clone(),
                    kind: value.kind_name().to_string(),
                    display: self.format(value),
                },
            );
        }

        let mut functions: BTreeMap<String, StateFunction> = BTreeMap::new();
        for builtin in crate::registry::BUILTINS {
            if !self.plugin_enabled(builtin.name) {
                continue;
            }
            functions.insert(
                builtin.name.to_string(),
                StateFunction {
                    name: builtin.name.to_string(),
                    parameters: builtin
                        .parameters
                        .iter()
                        .map(|p| (*p).to_string())
                        .collect(),
                    min_arity: builtin.min_arity,
                    variadic: builtin.variadic,
                    is_builtin: true,
                    plugin: builtin.plugin.map(|p| p.to_string()),
                },
            );
        }
        for name in self.modus.names() {
            functions.insert(name.to_string(), self.host_function(name));
        }
        for (name, (parameters, _)) in &self.funcs {
            functions.insert(
                name.clone(),
                StateFunction {
                    name: name.clone(),
                    parameters: parameters.clone(),
                    min_arity: parameters.len(),
                    variadic: false,
                    is_builtin: false,
                    plugin: None,
                },
            );
        }

        StateSnapshot {
            revision: self.revision,
            variables,
            functions,
        }
    }

    /// Defines or overwrites a global variable and raises [`VariableChanged`].
    ///
    /// The reference's `SuiteEngine.SetVariable`: the value is stored, the revision moves once,
    /// and the event carries `(name, value, removed: false)`.
    pub fn set_variable(&mut self, name: &str, value: Value) {
        self.vars.insert(name.to_string(), value.clone());
        self.revision += 1;
        self.raise_variable_changed(name, &value, false);
    }

    /// Looks up a global variable without throwing; `None` is the reference's false return.
    pub fn try_get_variable(&self, name: &str) -> Option<&Value> {
        self.vars.get(name)
    }

    /// Removes a global variable, returning whether it existed.
    ///
    /// A present variable raises [`VariableChanged`] with `removed: true` and the value that was
    /// removed; a missing name returns false and raises nothing, exactly the reference's
    /// `SuiteEngine.RemoveVariable` false branch.
    pub fn remove_variable(&mut self, name: &str) -> bool {
        match self.vars.remove(name) {
            Some(value) => {
                self.revision += 1;
                self.raise_variable_changed(name, &value, true);
                true
            }
            None => false,
        }
    }

    /// Subscribes to [`VariableChanged`]. Listeners run in subscription order, with the payload
    /// for one mutation, before the engine call returns.
    pub fn on_variable_changed<F>(&mut self, listener: F)
    where
        F: FnMut(&VariableChanged) + Send + 'static,
    {
        self.variable_listeners.push(Box::new(listener));
    }

    /// Subscribes to [`FunctionDefined`]. Listeners run in subscription order, with the payload
    /// for one definition, before the engine call returns.
    pub fn on_function_defined<F>(&mut self, listener: F)
    where
        F: FnMut(&FunctionDefined) + Send + 'static,
    {
        self.function_listeners.push(Box::new(listener));
    }

    /// Raises one variable-change event to every subscriber.
    fn raise_variable_changed(&mut self, name: &str, value: &Value, removed: bool) {
        if self.variable_listeners.is_empty() {
            return;
        }
        let event = VariableChanged {
            name: name.to_string(),
            value: value.clone(),
            removed,
        };
        for listener in &mut self.variable_listeners {
            listener(&event);
        }
    }

    /// Raises one function-defined event to every subscriber.
    fn raise_function_defined(&mut self, definition: StateFunction) {
        if self.function_listeners.is_empty() {
            return;
        }
        let event = FunctionDefined { definition };
        for listener in &mut self.function_listeners {
            listener(&event);
        }
    }

    /// The snapshot view of one Modus-registered builtin.
    fn host_function(&self, name: &str) -> StateFunction {
        let descriptor = self.modus.descriptor_of(name);
        let parameters = descriptor
            .map(|descriptor| descriptor.parameters.clone())
            .unwrap_or_default();
        StateFunction {
            name: name.to_string(),
            min_arity: descriptor
                .and_then(|descriptor| descriptor.min_arity)
                .unwrap_or(parameters.len()),
            variadic: descriptor.is_some_and(|descriptor| descriptor.variadic),
            parameters,
            is_builtin: true,
            plugin: self.modus.plugin_name_of(name).map(str::to_string),
        }
    }

    /// Evaluates a script and returns its final value.
    ///
    /// The deadline and the live cancellation flag are armed for exactly this evaluation (and
    /// restored on the way out, so a second run on the same engine starts un-budgeted unless the
    /// caller arms one again). A kernel that observed either unwinds early; whatever value it
    /// returned is discarded and the structured cancellation failure is published instead.
    pub fn eval(&mut self, source: &str) -> Result<Value, LangError> {
        let deadline = self.cancel_after.map(|after| Instant::now() + after);
        let flag = self.cancel_flag.clone();
        let scope = lovelace_symbolics::cancel::arm_with_flag(deadline, flag);
        self.cancelled = false;
        // A caller-installed plugin scope takes precedence INSIDE this evaluation, and the
        // engine's own session set is restored when the evaluation ends (the reference's
        // "the engine installs its own store only when the caller has not installed one").
        let scoped = self.modus.active_assumption_scope();
        let saved_assumptions = scoped.map(|assumptions| {
            let previous = self.assumptions.clone();
            self.assumptions = assumptions;
            previous
        });
        let mut result = self.eval_inner(source);
        if let Some(previous) = saved_assumptions {
            self.assumptions = previous;
        }
        if lovelace_symbolics::cancel::observed() {
            self.cancelled = true;
            // A kernel stopped early and returned its identity value; that value is not a result.
            result = match result {
                Err(error) if error.code == "Cancelled" => {
                    // The statement loop normally placed it; a cancellation that escaped before
                    // the first statement still names the statement that was running.
                    if error.position == 0 {
                        Err(error.with_position(self.current_position))
                    } else {
                        Err(error)
                    }
                }
                _ => Err(cancelled_error().with_position(self.current_position)),
            };
        }
        drop(scope);
        result
    }

    /// Evaluates with a caller-supplied output writer, the reference's
    /// `EvaluateAsync(source, output, token)` overload. Print lines produced by THIS evaluation
    /// go to `writer`; the engine's previously captured output is restored afterwards (the
    /// reference saves and restores `Output` in a finally block). The value or failure is the
    /// ordinary `eval` outcome.
    pub fn eval_with_output<W: Write>(
        &mut self,
        source: &str,
        writer: &mut W,
    ) -> Result<Value, LangError> {
        let already_captured = self.output.len();
        let result = self.eval(source);
        let produced: Vec<String> = self.output.drain(already_captured..).collect();
        for line in produced {
            let _ = writeln!(writer, "{line}");
        }
        result
    }

    /// Evaluates under a caller-supplied deadline, the reference's cancellation-token budget, and
    /// restores the engine's previous deadline afterwards. A kernel that observes the spent budget
    /// returns `Cancelled`/`BudgetExceeded`; [`Engine::was_cancelled`] records the observation.
    pub fn eval_with_budget(&mut self, source: &str, budget: Duration) -> Result<Value, LangError> {
        let previous = self.cancel_after.replace(budget);
        let result = self.eval(source);
        self.cancel_after = previous;
        result
    }

    fn eval_inner(&mut self, source: &str) -> Result<Value, LangError> {
        // The reference's ScriptSource.ToSemicolonStatements strips a leading byte-order mark
        // before the engine sees the text and its ScriptPositions maps every published engine
        // offset back across that one skipped character. Reproduce both halves here: the mark is
        // accepted (not a parse error), the script runs identically to the same text without it,
        // and every timing/diagnostic position still names the CALLER's text.
        let (text, offset) = match source.strip_prefix('\u{feff}') {
            Some(rest) => (rest, 1usize),
            None => (source, 0usize),
        };
        let mut result = self.eval_text(text);
        if offset > 0 {
            for timing in &mut self.timings {
                timing.position += offset;
            }
            self.current_position += offset;
            self.last_completed_position += offset;
            if let Err(error) = &mut result {
                error.position += offset;
            }
        }
        result
    }

    // -----------------------------------------------------------------
    // Lexical scopes
    // -----------------------------------------------------------------

    /// Looks a name up in the live lexical chain, innermost first, then the global store.
    fn lookup(&self, name: &str) -> Option<&Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Some(value);
            }
        }
        self.vars.get(name)
    }

    /// Assigns to the nearest existing binding, or defines in the innermost scope when the name is
    /// new (the reference's `Scope.Assign`). Returns whether a GLOBAL binding was written, which is
    /// the reference's `VariableChanged` condition.
    fn assign(&mut self, name: &str, value: Value) -> bool {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(slot) = scope.get_mut(name) {
                *slot = value;
                return false;
            }
        }
        if let Some(slot) = self.vars.get_mut(name) {
            *slot = value;
            return true;
        }
        // A name nothing binds yet is DEFINED in the innermost scope; at top level that scope is
        // the global store, so the definition is still a published global change.
        if self.scopes.is_empty() {
            self.vars.insert(name.to_string(), value);
            return true;
        }
        self.define(name, value);
        false
    }

    /// Defines a binding in the innermost scope only (the reference's `Scope.Define`).
    fn define(&mut self, name: &str, value: Value) {
        match self.scopes.last_mut() {
            Some(scope) => {
                scope.insert(name.to_string(), value);
            }
            None => {
                self.vars.insert(name.to_string(), value);
            }
        }
    }

    // -----------------------------------------------------------------
    // Evaluation budget (InputDepth.MaxEvaluationDepth)
    // -----------------------------------------------------------------

    /// The units one user-function level costs beyond its statements and expressions (the
    /// reference's `CallLevelUnits`).
    const FRAME_UNITS: usize = 4;

    /// The depth BELOW the current top-level statement's base, which is what the budget bounds.
    fn depth_below_base(&self) -> usize {
        if self.eval_base_established {
            self.eval_depth.saturating_sub(self.eval_base)
        } else {
            self.eval_depth
        }
    }

    /// Charges `units` of the evaluation budget, refusing when the measured depth passes the
    /// published bound. The charge is undone by the caller's own exit.
    fn enter_depth(&mut self, units: usize) -> Result<(), LangError> {
        self.eval_depth += units;
        let depth = self.depth_below_base();
        if depth > MAX_EVALUATION_DEPTH {
            self.eval_depth -= units;
            // The reference checks one unit at a time and publishes the FIRST level it found past
            // the budget, so the reported number is always limit + 1 even when the remaining walk
            // would measure deeper.
            let reported = (MAX_EVALUATION_DEPTH + 1).min(depth);
            return Err(LangError::depth(
                "evaluation",
                reported,
                MAX_EVALUATION_DEPTH,
                "evaluation",
            ));
        }
        Ok(())
    }

    fn exit_depth(&mut self, units: usize) {
        self.eval_depth = self.eval_depth.saturating_sub(units);
    }

    /// Marks the BASE of the budget for the current top-level statement: the first user-function
    /// frame together with everything the statement charged to reach it (the reference's
    /// `EstablishEvaluationBase`). The frame about to be pushed and the statement above it are the
    /// zero of the measurement, so f(85) answers and f(86) is refused.
    fn establish_eval_base(&mut self) {
        if !self.eval_base_established {
            self.eval_base_established = true;
            self.eval_base = self.eval_depth + Self::FRAME_UNITS;
        }
    }

    /// The evaluation proper, on the engine's own text (a leading byte-order mark already
    /// stripped by eval_inner).
    fn eval_text(&mut self, source: &str) -> Result<Value, LangError> {
        self.last_completed_position = 0;
        self.plot = None;
        self.timings.clear();
        let toks = syntax::tokenize(source).map_err(as_parse_error)?;
        let mut parser = Parser::new(toks);
        let program = parser.program().map_err(as_parse_error)?;
        // The parser's counter bounds the DESCENT; a left-associative chain never descends, so its
        // TREE is measured once, before any walk touches it (the reference's AstDepth rule).
        if let Some(depth) = ast_depth(&program) {
            return Err(LangError::depth(
                "expression tree",
                depth,
                MAX_PARSED_TREE_DEPTH,
                "nesting",
            ));
        }
        self.eval_depth = 0;
        self.free_charge = 0;
        let mut last = Value::Void;
        for stmt in &program {
            let position = statement_position(stmt);
            self.current_position = position;
            // Every top-level statement is its own entry into user code and finds its own base
            // (the reference resets the evaluation budget per statement).
            self.eval_base = 0;
            self.eval_base_established = false;
            // Statement granularity, exactly as the reference's interpreter checks: a script whose
            // budget expires between two statements stops before the next one runs.
            if lovelace_symbolics::cancel::check() {
                return Err(cancelled_error().with_position(position));
            }
            let printed_before = self.output.len();
            let started = Instant::now();
            match self.exec(stmt) {
                Ok(Flow::Value(v)) => {
                    self.timings.push(OpTiming {
                        position,
                        result_kind: v.kind_name().to_string(),
                        has_output: self.output.len() > printed_before,
                        elapsed: started.elapsed(),
                    });
                    self.last_completed_position = position;
                    last = v;
                }
                Ok(Flow::Return(v)) => {
                    self.timings.push(OpTiming {
                        position,
                        result_kind: v.kind_name().to_string(),
                        has_output: self.output.len() > printed_before,
                        elapsed: started.elapsed(),
                    });
                    self.last_completed_position = position;
                    last = v;
                    break;
                }
                Ok(Flow::Break | Flow::Continue) => {
                    return Err(LangError::plain("'break' is only valid inside a loop."));
                }
                Err(mut e) => {
                    // A failure that did not name its own position failed at the statement it
                    // was executing: publish the statement's offset, which is what an agent
                    // repairing the script needs (the reference's PublishDiagnostic rule).
                    if e.position == 0 {
                        e.position = position;
                    }
                    // The statement that FAILED is still a statement that ran: it gets its own
                    // timing entry (Void, plus whether it printed), exactly as the reference
                    // publishes it on the failure path.
                    self.timings.push(OpTiming {
                        position,
                        result_kind: "Void".to_string(),
                        has_output: self.output.len() > printed_before,
                        elapsed: started.elapsed(),
                    });
                    return Err(e);
                }
            }
        }
        // A symbolic VALUE deeper than the published canonical-construction budget is refused
        // with its own stage name (the reference refuses it while the node is built, so the first
        // refused level is always limit + 1).
        for value in std::iter::once(&last).chain(self.vars.values()) {
            if let Value::Symbolic(expr) = value {
                let depth = symbolic_depth(expr);
                if depth > MAX_SYMBOLIC_DEPTH {
                    let reported = (MAX_SYMBOLIC_DEPTH + 1).min(depth);
                    return Err(LangError::depth(
                        "symbolic expression",
                        reported,
                        MAX_SYMBOLIC_DEPTH,
                        "nesting",
                    ));
                }
            }
        }
        // A VALUE deeper than the published budget is a valid input the renderer cannot walk:
        // refuse it structurally before any capture or projection recurses over it.
        check_value_depth(&last)?;
        for value in self.vars.values() {
            check_value_depth(value)?;
        }
        self.revision += 1;
        if !matches!(last, Value::Void) {
            self.vars.insert("_".to_string(), last.clone());
            // The reference stores the result through `SetVariable("_", result)`, so the
            // result variable is itself a variable-change event, after every statement event.
            self.raise_variable_changed("_", &last, false);
        }
        Ok(last)
    }

    fn exec(&mut self, stmt: &Stmt) -> Result<Flow, LangError> {
        // A BlockStatement is GROUPING rather than a step of its own: the statements inside it are
        // charged individually, so it costs nothing (the reference's ExecuteAsync).
        let charged = if matches!(stmt, Stmt::Block(_)) { 0 } else { 1 };
        if charged > 0 {
            self.enter_depth(charged)?;
        }
        let result = self.exec_statement(stmt);
        if charged > 0 {
            self.exit_depth(charged);
        }
        result
    }

    /// Executes one statement with its budget unit already charged.
    fn exec_statement(&mut self, stmt: &Stmt) -> Result<Flow, LangError> {
        match stmt {
            Stmt::Empty => Ok(Flow::Value(Value::Void)),
            Stmt::Expr(e, _) => {
                let v = self.eval_expr(e)?;
                Ok(Flow::Value(v))
            }
            Stmt::Block(stmts) => {
                // A braced block introduces a lexical scope (the reference's BlockStatement): names
                // it defines do not leak outward, while an assignment to an existing outer binding
                // still updates that binding.
                self.scopes.push(HashMap::new());
                let mut last = Value::Void;
                let mut outcome = Ok(Flow::Value(Value::Void));
                for s in stmts {
                    match self.exec(s) {
                        Ok(Flow::Value(v)) => last = v,
                        Ok(other) => {
                            outcome = Ok(other);
                            break;
                        }
                        Err(error) => {
                            outcome = Err(error);
                            break;
                        }
                    }
                }
                self.scopes.pop();
                match outcome? {
                    Flow::Value(_) => Ok(Flow::Value(last)),
                    other => Ok(other),
                }
            }
            Stmt::If(cond, then, other) => {
                let c = self.eval_expr(cond)?;
                match c {
                    Value::Boolean(true) => self.exec(then),
                    Value::Boolean(false) => match other {
                        Some(o) => self.exec(o),
                        None => Ok(Flow::Value(Value::Void)),
                    },
                    other => Err(LangError::plain(format!(
                        "if condition must be Boolean, but got '{}'.",
                        other.kind_name()
                    ))),
                }
            }
            Stmt::While(cond, body) => {
                // A live body is one statement per iteration, so check on every iteration: the
                // check is a thread-local read against the microseconds the body costs, and a
                // loop that never allocates would otherwise run for minutes unwatched.
                //
                // The loop's value is the last completed body value, as in the reference: a
                // sibling statement or loop scaffold does not turn a value into Void. A break
                // or continue ends the iteration without completing a value, so the previous
                // completed value stands.
                let mut last = Value::Void;
                let mut iteration = 0u64;
                loop {
                    if lovelace_symbolics::cancel::poll(iteration) {
                        return Err(cancelled_error());
                    }
                    iteration += 1;
                    match self.eval_expr(cond)? {
                        Value::Boolean(true) => {}
                        Value::Boolean(false) => break,
                        other => {
                            return Err(LangError::plain(format!(
                                "while condition must be Boolean, but got '{}'.",
                                other.kind_name()
                            )));
                        }
                    }
                    match self.exec(body)? {
                        Flow::Break => break,
                        Flow::Continue => continue,
                        Flow::Value(v) => last = v,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                    }
                }
                Ok(Flow::Value(last))
            }
            Stmt::For(var, range, body) => {
                let r = self.eval_expr(range)?;
                let items = match r {
                    Value::Vector(v) => v,
                    other => {
                        return Err(LangError::plain(format!(
                            "for loop needs a range, but got '{}'.",
                            other.kind_name()
                        )));
                    }
                };
                // The loop's value is the last completed body value (the reference's for/while
                // statement value); a loop that never runs a body answers Void.
                let mut last = Value::Void;
                for (iteration, item) in items.into_iter().enumerate() {
                    if lovelace_symbolics::cancel::poll(iteration as u64) {
                        return Err(cancelled_error());
                    }
                    // The loop variable is DEFINED in the current scope (the reference's
                    // Scope.Define), so a top-level loop leaves it global and a loop inside a
                    // block or a function body keeps it there.
                    self.define(var, item);
                    match self.exec(body)? {
                        Flow::Break => break,
                        Flow::Continue => continue,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        Flow::Value(v) => last = v,
                    }
                }
                Ok(Flow::Value(last))
            }
            Stmt::Return(e, _) => {
                let v = match e {
                    Some(x) => self.eval_expr(x)?,
                    None => Value::Void,
                };
                Ok(Flow::Return(v))
            }
            Stmt::Break => Ok(Flow::Break),
            Stmt::Continue => Ok(Flow::Continue),
            Stmt::Func(name, params, body) => {
                self.funcs
                    .insert(name.clone(), (params.clone(), (**body).clone()));
                self.raise_function_defined(StateFunction {
                    name: name.clone(),
                    parameters: params.clone(),
                    min_arity: params.len(),
                    variadic: false,
                    is_builtin: false,
                    plugin: None,
                });
                Ok(Flow::Value(Value::Void))
            }
        }
    }

    fn eval_expr(&mut self, e: &Expr) -> Result<Value, LangError> {
        // A LEAF -- a literal, a variable, a string -- descends nowhere and costs no unit, and a
        // node inside a call argument's subtree is uncharged too, exactly as the reference routes
        // arguments. Everything else re-enters here once per descent.
        let leaf = matches!(
            e,
            Expr::Number(_) | Expr::Repeating(_) | Expr::Str(_) | Expr::Var(_, _)
        );
        let charged = if leaf || self.free_charge > 0 { 0 } else { 1 };
        if charged > 0 {
            self.enter_depth(charged)?;
        }
        let result = self.eval_expr_node(e);
        if charged > 0 {
            self.exit_depth(charged);
        }
        result
    }

    /// Evaluates one expression node with its budget unit already charged.
    fn eval_expr_node(&mut self, e: &Expr) -> Result<Value, LangError> {
        match e {
            Expr::Number(text) => {
                if text.contains('.') {
                    Ok(Value::Real(Real::from_str(text).ok_or_else(|| {
                        LangError::plain(format!("Invalid number literal '{}'.", text))
                    })?))
                } else {
                    Ok(Value::Natural(Nat::from_decimal(text).ok_or_else(
                        || LangError::plain(format!("Invalid number literal '{}'.", text)),
                    )?))
                }
            }
            Expr::Repeating(text) => {
                Ok(Value::Real(Real::from_str(text).ok_or_else(|| {
                    LangError::plain(format!("Invalid number literal '{}'.", text))
                })?))
            }
            Expr::Str(s) => Ok(Value::Text(unescape(s))),
            Expr::Interp(parts) => {
                let mut out = String::new();
                for p in parts {
                    match p {
                        InterpPart::Text(t) => out.push_str(t),
                        InterpPart::Expr(x) => out.push_str(&self.eval_expr(x)?.display()),
                    }
                }
                Ok(Value::Text(out))
            }
            Expr::Var(name, _pos) => match self.lookup(name) {
                Some(v) => Ok(v.clone()),
                // Mathematical constants, resolved after the variable store so a user
                // assignment shadows them. The value is COMPUTED at the engine's computation
                // precision (1000 until setprecision moves it) and only RENDERED at the display
                // precision, exactly as the reference's Real.Pi: the machine payload carries
                // every stored digit (`pi` crosses with 1000 fractional digits and a 100-digit
                // display), and a wider `evalf` request can be served from them.
                None if name == "pi" => Ok(Value::Real(Real::pi(self.precision_value()))),
                None if name == "e" => {
                    Ok(Value::Real(builtins::e_constant(self.precision_value())))
                }
                None if name == "inf" => {
                    Ok(Value::Symbolic(Sym::Fun("inf".to_string(), Vec::new())))
                }
                None if matches!(name.as_str(), "real" | "complex" | "integer" | "rational") => {
                    Ok(Value::Domain(name.clone()))
                }
                None => match self.funcs.get(name) {
                    Some(_) => Ok(Value::Function(name.clone())),
                    // An undefined name is reported at the STATEMENT the evaluator is running,
                    // not at the symbol: the reference's diagnostics put the failure at the
                    // statement start (diff(x) -> position 0/column 1; after "a = 1; " -> 7/8),
                    // and the evaluation loop positions a zero-position error that way.
                    None => Err(LangError::plain(format!("Undefined variable '{}'.", name))),
                },
            },
            Expr::Call(name, args, pos) => {
                let mut values = Vec::with_capacity(args.len());
                for a in args {
                    // An argument's subtree is uncharged: the budget measures the DESCENT the
                    // function bodies add, and the reference's argument routing charges nothing.
                    self.free_charge += 1;
                    let evaluated = self.eval_expr(a);
                    self.free_charge -= 1;
                    values.push(evaluated?);
                }
                self.call(name, values, *pos)
            }
            Expr::Binary(op, _, _) => {
                // A flat operator chain (a + b + c + ...) parses LEFT-DEEP: evaluating it by
                // recursion costs one stack frame per term, so a legitimate 420-term sum
                // exhausts a 2 MiB stack before any depth budget can refuse it. Fold the
                // SAME-operator spine iteratively — strict left-to-right, exactly the recursive
                // association — and recurse only into the operands themselves.
                let mut spine: Vec<&Expr> = Vec::new();
                let mut node = e;
                while let Expr::Binary(inner_op, left, right) = node {
                    if inner_op != op {
                        break;
                    }
                    spine.push(right);
                    node = left.as_ref();
                }
                // The folded spine nodes are LIVE for as long as the fold runs, exactly as the
                // recursive walk would keep them: charge the ones the wrapper did not charge, so a
                // deep flat chain measures the same depth as its nested spelling.
                let spine_extra = spine.len().saturating_sub(1);
                if spine_extra > 0 {
                    self.enter_depth(spine_extra)?;
                }
                let folded = (|| {
                    let mut acc = self.eval_expr(node)?;
                    for operand in spine.iter().rev() {
                        let rv = self.eval_expr(operand)?;
                        acc = builtins::binary(*op, &acc, &rv, self.precision)?;
                    }
                    Ok(acc)
                })();
                if spine_extra > 0 {
                    self.exit_depth(spine_extra);
                }
                folded
            }
            Expr::Neg(inner) => {
                let v = self.eval_expr(inner)?;
                builtins::neg(&v)
            }
            // Unary plus is the identity value; the node exists so the parsed shape matches the
            // reference's UnaryExpr(UnaryOp.Plus) without changing any evaluation result.
            Expr::Plus(inner) => self.eval_expr(inner),
            Expr::Not(inner) => {
                let v = self.eval_expr(inner)?;
                match v {
                    Value::Symbolic(e) => Ok(Value::Symbolic(Sym::logical_not(e))),
                    Value::Boolean(b) => Ok(Value::Boolean(!b)),
                    other => Err(LangError::plain(format!(
                        "not needs a Boolean, but got '{}'.",
                        other.kind_name()
                    ))),
                }
            }
            Expr::Fact(inner) => {
                let v = self.eval_expr(inner)?;
                match v {
                    Value::Natural(n) => match n.to_u64() {
                        Some(k) => Ok(Value::Natural(cancellable::factorial(k)?)),
                        None => Err(LangError::plain("Factorial argument is too large.")),
                    },
                    // An Integer factorial is the same value with the Integer kind: zero and
                    // positive integers answer, a negative one is a named domain refusal.
                    Value::Integer(i) => {
                        if i.is_negative() {
                            return Err(LangError::plain(
                                "Factorial is not defined for negative integers.",
                            ));
                        }
                        match i.magnitude().to_u64() {
                            Some(k) => {
                                let magnitude = cancellable::factorial(k)?;
                                Ok(Value::Integer(
                                    Int::from_decimal(&magnitude.to_decimal())
                                        .unwrap_or_else(Int::zero),
                                ))
                            }
                            None => Err(LangError::plain("Factorial argument is too large.")),
                        }
                    }
                    Value::Real(_) => Err(LangError::plain(
                        "Factorial is not supported for Real numbers.",
                    )),
                    other => Err(LangError::plain(format!(
                        "Factorial is not supported for type '{}'.",
                        other.kind_name()
                    ))),
                }
            }
            Expr::List(items) => {
                let mut values = Vec::with_capacity(items.len());
                for i in items {
                    values.push(self.eval_expr(i)?);
                }
                builtins::make_list(values)
            }
            Expr::Range(a, step, b) => {
                let av = self.eval_expr(a)?;
                let bv = self.eval_expr(b)?;
                let sv = match step {
                    Some(s) => Some(self.eval_expr(s)?),
                    None => None,
                };
                builtins::make_range(&av, sv.as_ref(), &bv)
            }
            Expr::Index(target, args) => {
                let t = self.eval_expr(target)?;
                let mut specs = Vec::with_capacity(args.len());
                for a in args {
                    specs.push(match a {
                        IndexArg::Single(x) => builtins::IndexSpec::Single(self.eval_expr(x)?),
                        IndexArg::Full => builtins::IndexSpec::Full,
                        IndexArg::Slice(x, y, z) => builtins::IndexSpec::Slice(
                            x.as_ref().map(|e| self.eval_expr(e)).transpose()?,
                            y.as_ref().map(|e| self.eval_expr(e)).transpose()?,
                            z.as_ref().map(|e| self.eval_expr(e)).transpose()?,
                        ),
                    });
                }
                builtins::index(&t, &specs)
            }
            Expr::Member(target, field) => {
                let t = self.eval_expr(target)?;
                match t {
                    Value::Record(type_name, fields) => fields
                        .iter()
                        .find(|(k, _)| k == field)
                        .map(|(_, v)| v.clone())
                        .ok_or_else(|| {
                            let names: Vec<&str> =
                                fields.iter().map(|(name, _)| name.as_str()).collect();
                            LangError::plain(format!(
                                "record '{}' has no member '{}'. Available members: {}.",
                                type_name,
                                field,
                                names.join(", ")
                            ))
                        }),
                    other => {
                        let hint = if matches!(other, Value::Symbolic(_)) {
                            " Symbolic expressions have no members; call the *_full builtin (e.g. solve_full) for structured results."
                        } else {
                            ""
                        };
                        Err(LangError::plain(format!(
                            "member '{}' is not available on type '{}': member access requires a record result.{}",
                            field,
                            other.kind_name(),
                            hint
                        )))
                    }
                }
            }
            Expr::Assign(name, value) => {
                let v = self.eval_expr(value)?;
                // Standard block-scoped assignment: update the nearest existing binding, otherwise
                // define in the innermost scope. Only a GLOBAL write raises `VariableChanged`
                // (the reference's DefineInScope).
                let global = self.assign(name, v.clone());
                if global {
                    self.raise_variable_changed(name, &v, false);
                }
                Ok(v)
            }
        }
    }

    fn call(&mut self, name: &str, args: Vec<Value>, pos: usize) -> Result<Value, LangError> {
        if let Some((params, body)) = self.funcs.get(name).cloned() {
            if params.len() != args.len() {
                return Err(LangError::at(
                    format!(
                        "'{}' expects {} argument(s), but {} were given.",
                        name,
                        params.len(),
                        args.len()
                    ),
                    pos,
                ));
            }
            // A frame's parent is the GLOBAL scope (the reference's `new Scope(_global)`), so a
            // callee never sees its caller's block locals; the caller's chain is restored when the
            // call returns, including down an error path.
            let saved_scopes = std::mem::take(&mut self.scopes);
            let mut frame = HashMap::new();
            for (p, v) in params.iter().zip(args) {
                frame.insert(p.clone(), v);
            }
            self.scopes.push(frame);
            // The argument-suppression state belongs to the CALLER's argument list; the callee's
            // body is charged normally.
            let saved_free = std::mem::replace(&mut self.free_charge, 0);
            // The first user frame of the statement is the ZERO of the measurement; every level
            // below it costs the frame's units plus the statements and descending expressions its
            // body walks (f(85) answers, f(86) is refused).
            self.establish_eval_base();
            if let Err(error) = self.enter_depth(Self::FRAME_UNITS) {
                self.free_charge = saved_free;
                self.scopes = saved_scopes;
                return Err(error);
            }
            let result = self.exec(&body);
            self.exit_depth(Self::FRAME_UNITS);
            self.free_charge = saved_free;
            self.scopes = saved_scopes;
            return match result? {
                Flow::Return(v) | Flow::Value(v) => Ok(v),
                Flow::Break | Flow::Continue => {
                    Err(LangError::plain("'break' is only valid inside a loop."))
                }
            };
        }
        // The call boundary is the second cancellation seam: a kernel that observed the deadline
        // while the engine was not inside its inner loop still stops here, before the next call.
        // The cancellation carries NO position: the reference publishes the top-level statement
        // the deadline passed inside, and only the statement loop knows that position.
        if lovelace_symbolics::cancel::check() {
            return Err(cancelled_error());
        }
        // A dynamically registered builtin (a plugin or host registration) is dispatched before
        // the static builtin table, so a plugin can own a name the language match does not know.
        if let Some(result) = self.modus.call(name, &args, pos) {
            return result;
        }
        // A bare engine hides the builtins a plugin descriptor advertises until that plugin is
        // loaded; the unresolved call is the same vocabulary failure an absent builtin gives.
        if !self.plugin_enabled(name) {
            return Err(builtins::unknown_function_among(name, pos, &|candidate| {
                self.plugin_enabled(candidate)
            }));
        }
        // A builtin that failed without naming its own position failed at the CALL: the
        // published diagnostic points at the call site, which is where the caller must look.
        match builtins::call(self, name, args, pos) {
            // A cancellation is not a call-site argument refusal: its position is the statement
            // that was running, supplied by the statement loop, never the inner call site.
            Err(e) if e.position == 0 && e.code != "Cancelled" => Err(e.with_position(pos)),
            // The kernel returned a partial value after observing the deadline; the value is not
            // an answer.
            Ok(_) if lovelace_symbolics::cancel::observed() => Err(cancelled_error()),
            other => other,
        }
    }

    pub(crate) fn precision_value(&self) -> usize {
        self.precision
    }

    /// The precision a DSP (plugin-style) builtin computes at: the reference runs plugin
    /// builtins at a 30-place fast default and follows the engine knob once it is set
    /// explicitly (`ModusHost.PluginPrecisionScope` / `PluginStandardComputationPrecision`).
    pub(crate) fn dsp_precision(&self) -> usize {
        if self.precision_explicit {
            self.precision
        } else {
            30
        }
    }

    pub(crate) fn capture_plot(&mut self, capture: PlotCapture) {
        self.plot = Some(capture);
    }

    pub(crate) fn plot_dir(&self) -> String {
        self.plot_dir.clone()
    }

    /// The file name a plot is written under.
    pub(crate) fn plot_file(&self) -> String {
        self.plot_file.clone()
    }

    /// Sets the plot file name (--plot-file).
    pub fn set_plot_file(&mut self, name: &str) {
        self.plot_file = name.to_string();
    }

    pub(crate) fn push_output(&mut self, line: String) {
        self.output.push(line);
    }

    /// This engine's assumption set.
    pub(crate) fn assumptions(&self) -> &lovelace_symbolics::assumptions::AssumptionSet {
        &self.assumptions
    }

    /// Installs an assumption set supplied by a [`context::Context`] scope. The simplifier reads
    /// this set exactly as it reads a session's own; no other engine state is touched.
    pub(crate) fn set_assumptions(
        &mut self,
        assumptions: lovelace_symbolics::assumptions::AssumptionSet,
    ) {
        self.assumptions = assumptions;
    }

    /// Adds one atom to this engine's set; a contradiction is the reference's refusal sentence
    /// (the caller publishes it with the reference's error code and category).
    pub(crate) fn add_assumption(
        &mut self,
        atom: lovelace_symbolics::assumptions::Assumption,
    ) -> Result<(), String> {
        let current = std::mem::replace(
            &mut self.assumptions,
            lovelace_symbolics::assumptions::AssumptionSet::empty(),
        );
        match current.add_owned(atom) {
            Ok(next) => {
                self.assumptions = next;
                Ok(())
            }
            Err((contradiction, unchanged)) => {
                self.assumptions = unchanged;
                Err(contradiction.message)
            }
        }
    }

    /// Drops every assumption of THIS engine: the set, the legacy projections solve still reads,
    /// and the interval bounds. A fresh engine and a cleared one answer alike.
    pub(crate) fn clear_assumptions(&mut self) {
        self.assumptions = lovelace_symbolics::assumptions::AssumptionSet::empty();
        self.positive_symbols.clear();
        self.nonzero_symbols.clear();
        self.assumption_texts.clear();
        self.assumption_atoms.clear();
        self.domain_assumptions.clear();
        self.bounds.clear();
    }

    pub(crate) fn record_assumption(&mut self, text: &str) {
        if !self.assumption_texts.iter().any(|t| t == text) {
            self.assumption_texts.push(text.to_string());
        }
    }

    pub(crate) fn add_bound(
        &mut self,
        var: &str,
        lower: Option<(lovelace_symbolics::Rat, bool)>,
        upper: Option<(lovelace_symbolics::Rat, bool)>,
    ) -> Result<(), ()> {
        let entry = match self.bounds.iter_mut().find(|(name, _, _)| name == var) {
            Some(e) => e,
            None => {
                self.bounds.push((var.to_string(), None, None));
                self.bounds.last_mut().unwrap()
            }
        };
        if let Some((value, inclusive)) = &lower {
            let stricter = match &entry.1 {
                None => true,
                Some((existing, existing_inc)) => match value.cmp(existing) {
                    std::cmp::Ordering::Greater => true,
                    std::cmp::Ordering::Equal => !inclusive || *existing_inc,
                    std::cmp::Ordering::Less => false,
                },
            };
            if stricter {
                entry.1 = Some((value.clone(), *inclusive));
            }
        }
        if let Some((value, inclusive)) = &upper {
            let stricter = match &entry.2 {
                None => true,
                Some((existing, existing_inc)) => match value.cmp(existing) {
                    std::cmp::Ordering::Less => true,
                    std::cmp::Ordering::Equal => !inclusive || *existing_inc,
                    std::cmp::Ordering::Greater => false,
                },
            };
            if stricter {
                entry.2 = Some((value.clone(), *inclusive));
            }
        }
        // Contradiction: lower bound above the upper bound.
        if let (Some((lo, lo_inc)), Some((hi, hi_inc))) = (&entry.1, &entry.2) {
            match lo.cmp(hi) {
                std::cmp::Ordering::Greater => return Err(()),
                std::cmp::Ordering::Equal if !(*lo_inc && *hi_inc) => return Err(()),
                _ => {}
            }
        }
        Ok(())
    }

    /// Records one assumption ATOM (the expression form of the same fact the text carries).
    pub(crate) fn record_assumption_atom(&mut self, atom: &Sym) {
        let text = atom.to_string();
        if !self
            .assumption_atoms
            .iter()
            .any(|existing| existing.to_string() == text)
        {
            self.assumption_atoms.push(atom.clone());
        }
    }

    /// The active atoms that constrain `var` — an assumption about another symbol is not a
    /// condition on this solve. Matches the reference's AmbientAtomsOn.
    pub(crate) fn assumption_atoms_on(&self, var: &str) -> Vec<Sym> {
        self.assumption_atoms
            .iter()
            .filter(|atom| mentions(atom, var))
            .cloned()
            .collect()
    }

    /// Records a domain assumption for one symbol ("real" or "integer").
    pub(crate) fn record_domain_assumption(&mut self, var: &str, domain: &str) {
        if !self
            .domain_assumptions
            .iter()
            .any(|(name, existing)| name == var && existing == domain)
        {
            self.domain_assumptions
                .push((var.to_string(), domain.to_string()));
        }
    }

    /// The domains assumed for one symbol, in declaration order.
    pub(crate) fn domain_assumptions_on(&self, var: &str) -> Vec<String> {
        self.domain_assumptions
            .iter()
            .filter(|(name, _)| name == var)
            .map(|(_, domain)| domain.clone())
            .collect()
    }

    pub(crate) fn is_positive(&self, name: &str) -> bool {
        self.positive_symbols.iter().any(|s| s == name)
    }

    pub(crate) fn is_nonzero(&self, name: &str) -> bool {
        self.nonzero_symbols.iter().any(|s| s == name)
    }

    pub(crate) fn mark_positive(&mut self, name: &str) {
        if !self.is_positive(name) {
            self.positive_symbols.push(name.to_string());
        }
    }

    pub(crate) fn mark_nonzero(&mut self, name: &str) {
        if !self.is_nonzero(name) {
            self.nonzero_symbols.push(name.to_string());
        }
    }

    #[cfg(test)]
    pub(crate) fn set_var(&mut self, name: &str, value: Value) {
        self.vars.insert(name.to_string(), value);
    }
}

/// How a statement finished.
pub enum Flow {
    /// Fell through with a value.
    Value(Value),
    /// Returned from a function.
    Return(Value),
    /// Broke out of a loop.
    Break,
    /// Continued a loop.
    Continue,
}

/// Whether an assumption atom mentions `name` — the reference's MentionsAny rule, which makes
/// an assumption about another symbol irrelevant to this solve.
fn mentions(atom: &Sym, name: &str) -> bool {
    atom.free_symbols().iter().any(|symbol| symbol == name)
}

/// The source offset of a statement, for the per-statement timing ledger.
fn statement_position(stmt: &Stmt) -> usize {
    match stmt {
        Stmt::Expr(_, pos) => *pos,
        _ => 0,
    }
}

/// The published parsed-tree budget (the reference's InputDepth.MaxParsedTreeDepth).
const MAX_PARSED_TREE_DEPTH: usize = 512;

/// The published value-nesting budget (the reference's InputDepth.MaxValueDepth).
const MAX_VALUE_DEPTH: usize = 1024;

/// The published evaluation-depth budget (the reference's InputDepth.MaxEvaluationDepth).
const MAX_EVALUATION_DEPTH: usize = 512;

/// The published symbolic-expression budget (the reference's InputDepth.Max).
const MAX_SYMBOLIC_DEPTH: usize = 256;

/// Refuses a canonical expression deeper than the published symbolic budget (the reference's
/// one construction funnel). A symbolic-producing builtin calls this after building its node.
pub(crate) fn ensure_symbolic_budget(expr: &Sym) -> Result<(), LangError> {
    let depth = symbolic_depth(expr);
    if depth > MAX_SYMBOLIC_DEPTH {
        let reported = (MAX_SYMBOLIC_DEPTH + 1).min(depth);
        return Err(LangError::depth(
            "symbolic expression",
            reported,
            MAX_SYMBOLIC_DEPTH,
            "nesting",
        ));
    }
    Ok(())
}

/// The nesting depth of a canonical symbolic expression, measured iteratively.
pub(crate) fn symbolic_depth(root: &Sym) -> usize {
    let mut max = 1usize;
    let mut stack: Vec<(&Sym, usize)> = vec![(root, 1)];
    while let Some((expr, depth)) = stack.pop() {
        if depth > max {
            max = depth;
        }
        let next = depth + 1;
        match expr {
            Sym::Add(items) | Sym::Mul(items) | Sym::Fun(_, items) => {
                for item in items {
                    stack.push((item, next));
                }
            }
            Sym::Pow(left, right) | Sym::Div(left, right) | Sym::Rel(_, left, right) => {
                stack.push((left, next));
                stack.push((right, next));
            }
            Sym::Num(_) | Sym::Real(_) | Sym::Sym(_) | Sym::Complex(_, _) => {}
        }
    }
    max
}

/// One node of the parsed tree during the iterative depth walk.
enum AstNode<'a> {
    Stmt(&'a Stmt),
    Expr(&'a Expr),
}

/// The first parsed-tree level past the published budget, measured iteratively (a recursive walk
/// would overflow on exactly the input this guards). The program is level 1, each statement level
/// 2, and each expression child one level below its parent — the reference's AstDepth rule. None
/// means the tree is within budget; Some(depth) is the FIRST level the walk found past it, which is
/// the number the reference publishes (513 for a chain whose deepest leaf is far deeper).
fn ast_depth(program: &[Stmt]) -> Option<usize> {
    let mut stack: Vec<(AstNode<'_>, usize)> = program
        .iter()
        .rev()
        .map(|s| (AstNode::Stmt(s), 2usize))
        .collect();
    while let Some((node, depth)) = stack.pop() {
        if depth > MAX_PARSED_TREE_DEPTH {
            return Some(depth);
        }
        let next = depth + 1;
        match node {
            AstNode::Stmt(stmt) => match stmt {
                Stmt::Expr(e, _) => stack.push((AstNode::Expr(e), next)),
                Stmt::If(cond, then, other) => {
                    stack.push((AstNode::Expr(cond), next));
                    stack.push((AstNode::Stmt(then), next));
                    if let Some(other) = other {
                        stack.push((AstNode::Stmt(other), next));
                    }
                }
                Stmt::While(cond, body) => {
                    stack.push((AstNode::Expr(cond), next));
                    stack.push((AstNode::Stmt(body), next));
                }
                Stmt::For(_, range, body) => {
                    stack.push((AstNode::Expr(range), next));
                    stack.push((AstNode::Stmt(body), next));
                }
                Stmt::Block(stmts) => {
                    for s in stmts {
                        stack.push((AstNode::Stmt(s), next));
                    }
                }
                Stmt::Return(Some(e), _) => stack.push((AstNode::Expr(e), next)),
                Stmt::Func(_, _, body) => stack.push((AstNode::Stmt(body), next)),
                Stmt::Return(None, _) | Stmt::Break | Stmt::Continue | Stmt::Empty => {}
            },
            AstNode::Expr(expr) => match expr {
                Expr::Binary(_, left, right) => {
                    stack.push((AstNode::Expr(left), next));
                    stack.push((AstNode::Expr(right), next));
                }
                Expr::Neg(inner) | Expr::Plus(inner) | Expr::Not(inner) | Expr::Fact(inner) => {
                    stack.push((AstNode::Expr(inner), next));
                }
                Expr::Call(_, args, _) => {
                    for a in args {
                        stack.push((AstNode::Expr(a), next));
                    }
                }
                Expr::List(items) => {
                    for item in items {
                        stack.push((AstNode::Expr(item), next));
                    }
                }
                Expr::Range(start, step, end) => {
                    stack.push((AstNode::Expr(start), next));
                    if let Some(step) = step {
                        stack.push((AstNode::Expr(step), next));
                    }
                    stack.push((AstNode::Expr(end), next));
                }
                Expr::Index(target, args) => {
                    stack.push((AstNode::Expr(target), next));
                    for arg in args {
                        match arg {
                            IndexArg::Single(e) => stack.push((AstNode::Expr(e), next)),
                            IndexArg::Slice(a, b, c) => {
                                for e in [a, b, c].into_iter().flatten() {
                                    stack.push((AstNode::Expr(e), next));
                                }
                            }
                            IndexArg::Full => {}
                        }
                    }
                }
                Expr::Member(target, _) => stack.push((AstNode::Expr(target), next)),
                Expr::Assign(_, value) => stack.push((AstNode::Expr(value), next)),
                Expr::Interp(parts) => {
                    for part in parts {
                        if let InterpPart::Expr(e) = part {
                            stack.push((AstNode::Expr(e), next));
                        }
                    }
                }
                Expr::Number(_) | Expr::Repeating(_) | Expr::Str(_) | Expr::Var(_, _) => {}
            },
        }
    }
    None
}

/// The nesting depth of a run-time value: a leaf is level 1, a container one level below its
/// deepest child. Measured iteratively, for the same reason as [`ast_depth`].
fn value_depth(root: &Value) -> usize {
    let mut max = 1usize;
    let mut stack: Vec<(&Value, usize)> = vec![(root, 1)];
    while let Some((value, depth)) = stack.pop() {
        if depth > max {
            max = depth;
        }
        let next = depth + 1;
        match value {
            Value::Vector(items) => {
                for item in items {
                    stack.push((item, next));
                }
            }
            Value::Array(array) => {
                // An Array is its SHAPE plus flat elements: a rank-r array nests r levels above
                // its elements, so the children sit r levels below this node (a rank-1 array of
                // scalars is depth 2, a rank-20000 nested list is depth 20001).
                let child = depth + array.shape().len();
                if child > max {
                    max = child;
                }
                for item in array.iter() {
                    stack.push((item, child));
                }
            }
            Value::Record(_, fields) => {
                for (_, item) in fields {
                    stack.push((item, next));
                }
            }
            _ => {}
        }
    }
    max
}

/// Refuses a value whose nesting exceeds the published value budget.
///
/// This is the check the reference's `ValueFormatter.Format` and
/// `StructuredProjection.ToStructured` perform before their recursive walks. It is public so a
/// host that constructs values itself can ask the same question the engine asks after every
/// evaluation, and the fallible display/projection entry points ([Value::try_display],
/// [structured::try_project_value]) call it.
///
/// The measured depth is the value's real depth, so a refusal at the boundary names `1025`; the
/// walk is iterative, which is what makes a value 20000 levels deep refuse instead of dying.
pub fn check_value_depth(root: &Value) -> Result<(), LangError> {
    let depth = value_depth(root);
    if depth > MAX_VALUE_DEPTH {
        return Err(LangError::depth("value", depth, MAX_VALUE_DEPTH, "nesting"));
    }
    Ok(())
}

/// A tokenizer or parser refusal: both are the PARSE layer, so both carry its wire code. A
/// tokenizer refusal that already names a finer code (the unterminated-string `InvalidInput`) and
/// a depth refusal that carries `DepthExceeded` keep their own taxonomy.
fn as_parse_error(mut e: LangError) -> LangError {
    if e.code == "InvalidOperation" {
        e.code = "ParseError";
        e.category = "ParseError";
    }
    e
}

fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
            continue;
        }
        out.push(c);
    }
    out
}

/// Convenience: parses a standalone expression.
pub fn parse_expression(source: &str) -> Result<Expr, LangError> {
    let toks = syntax::tokenize(source)?;
    let mut p = Parser::new(toks);
    p.single_expression()
}

/// Convenience: the number of tokens in a source (used by tests).
pub fn token_count(source: &str) -> Result<usize, LangError> {
    Ok(syntax::tokenize(source)?.len())
}

/// Re-exported so hosts can name binary operators.
pub use diagnostic::ErrorCategory;
pub use syntax::BinOp as Operator;
/// Re-exported token type.
pub use syntax::Tok as Token;

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]
    use super::*;

    fn eval(src: &str) -> String {
        let mut e = Engine::new();
        match e.eval(src) {
            Ok(v) => e.format_typed(&v),
            Err(err) => err.message,
        }
    }

    #[test]
    fn Eval_GivenNaturalLiteral_RendersNatural() {
        assert_eq!(eval("42"), "42 (Natural)");
    }

    #[test]
    fn Eval_GivenDivision_RendersRepeatingReal() {
        assert_eq!(eval("1 / 3"), "0.(3) (Real)");
    }

    #[test]
    fn Eval_GivenUnderflowingSubtraction_WidensToInteger() {
        assert_eq!(eval("3 - 5"), "-2 (Integer)");
    }

    #[test]
    fn Eval_GivenRange_RendersVector() {
        assert_eq!(eval("1..5"), "[1, 2, 3, 4, 5] (Vector)");
    }

    #[test]
    fn Eval_GivenRangeThenPower_AppliesPowerToRange() {
        assert_eq!(eval("1..3 ^ 2"), "[1, 4, 9] (Vector)");
    }

    /// Unary minus binds LOOSER than exponentiation, as in the reference grammar: `-2^2` is
    /// `-(2^2)` = -4 while `(-2)^2` is 4. The exponent may carry its own sign (`2^-2`), powers
    /// stay right-associative, and a signed range start still signs only the start (`-1..2`).
    #[test]
    fn Eval_GivenUnaryMinusBeforePower_BindsLooserThanPower() {
        assert_eq!(eval("-2^2"), "-4 (Integer)");
        assert_eq!(eval("-2^3"), "-8 (Integer)");
        assert_eq!(eval("x = 3; -x^2"), "-9 (Integer)");
        assert_eq!(eval("(-2)^2"), "4 (Integer)");
        assert_eq!(eval("2^-2"), "0.25 (Real)");
        assert_eq!(eval("-2^2+1"), "-3 (Integer)");
        assert_eq!(eval("2^3^2"), "512 (Natural)");
        assert_eq!(eval("-1..2"), "[-1, 0, 1, 2] (Vector)");
        // A signed term's rendering is unchanged: only the power's binding moved.
        assert_eq!(eval("-2*3"), "-6 (Integer)");
        assert_eq!(eval("-2+3"), "1 (Integer)");
    }

    #[test]
    fn Eval_GivenRecursiveFunction_Computes() {
        assert_eq!(
            eval("func f(n) { if (n == 0) { 1 } else { n * f(n - 1) } }; f(5)"),
            "120 (Natural)"
        );
    }

    #[test]
    fn Eval_GivenUndefinedVariable_ReportsDocumentedMessage() {
        assert_eq!(eval("nope + 1"), "Undefined variable 'nope'.");
    }

    #[test]
    fn Eval_GivenBlock_ReturnsLastValue() {
        assert_eq!(eval("{ 1; 2; 3 }"), "3 (Natural)");
    }
}

#[cfg(test)]
mod mathir_round54;

#[cfg(test)]
mod mathir_typing_round55;

#[cfg(test)]
mod context_round56;

#[cfg(test)]
mod suite_parity_batch;

#[cfg(test)]
mod suite_lex_parse_batch;

#[cfg(test)]
mod suite_engine_batch;

#[cfg(test)]
mod latex_and_sqrt_parity;

#[cfg(test)]
mod latex_printer_parity;

#[cfg(test)]
mod robustness_round13;

#[cfg(test)]
mod suite_surface_round14;

#[cfg(test)]
mod printing_round16;

#[cfg(test)]
mod printing_round17;

#[cfg(test)]
mod cancellation_round18;

#[cfg(test)]
mod round19_batch;

#[cfg(test)]
mod round66_resweep;

/// Round-73 arity-taxonomy family: the five deferred census rows for the reference's declared
/// argument-count classification (the one documented arity refusal and the call signature it
/// publishes).
#[cfg(test)]
mod arity_taxonomy_round73;

/// Round-67 Modus plugin-seam member map: the 22 deferred census rows whose contract is the
/// registration surface, the payload channel, load-time refusals, kernel dispatch, descriptor
/// arity and the plugin-instance assumption scope.
#[cfg(test)]
mod round67_modus;

/// Round-69 ParameterDomain row: the Modus-seam projection and the solution-family wire field.
#[cfg(test)]
mod round69_parameter_domain;

#[cfg(test)]
mod dsp_round20;

#[cfg(test)]
mod dsp_round21;

#[cfg(test)]
mod dispatcher_guard_round51;

#[cfg(test)]
mod panic_sweep_round22;

#[cfg(test)]
#[cfg(test)]
mod complex_round24;

#[cfg(test)]
mod transcendental_round25;

#[cfg(test)]
mod round26_complex_math_parity;

#[cfg(test)]
mod round26_complex_value_parity;

#[cfg(test)]
mod round26_evalf_parity;

#[cfg(test)]
mod round27_batch;

#[cfg(test)]
mod round28_batch;

#[cfg(test)]
mod round30_batch;

#[cfg(test)]
mod circular_family_round31;

#[cfg(test)]
mod inverse_branch_round33;

#[cfg(test)]
mod relations_families_round33;

#[cfg(test)]
mod symbolics_tail_round34;

#[cfg(test)]
mod symbolics_round23;

#[cfg(test)]
mod assumptions_round35;

#[cfg(test)]
mod real_constant_round35;

#[cfg(test)]
mod suite_tail_round35;

#[cfg(test)]
mod round35_extra_tail;
#[cfg(test)]
mod round36_remaining;

#[cfg(test)]
mod corpus_and_system_solve_round36;

#[cfg(test)]
mod round38_remaining;

#[cfg(test)]
mod round38_series_kink_hardening;

#[cfg(test)]
mod round39_residue;

#[cfg(test)]
mod round42_differential_classes;

#[cfg(test)]
mod round43_display_classes;

#[cfg(test)]
mod round44_parity_classes;

#[cfg(test)]
mod round46_token_kind;

#[cfg(test)]
mod falsification_round46;

/// Round-48 fixed-point DSP parity: the ten `FixedDspTests` census rows, mirrored over the
/// additive [`fixed_dsp`] kernels.
#[cfg(test)]
mod fixed_dsp_tests;

/// Round-61 DSP family: the seven round-50 `dsp` census rows, each pinned to the oracle.
#[cfg(test)]
mod dsp_family_round61;

/// Round-76 SuiteEngine capture/event surface: the four deferred `SuiteEngineTests` rows for
/// `CaptureState`, `VariableChanged`, `FunctionDefined` and `SetVariable`/`RemoveVariable`.
#[cfg(test)]
mod round76_capture_events;

/// Round-78 assumption-scoped numeric evaluation: the four deferred `PiecewiseAssumptionTests`
/// rows for `EvaluateToNum` under a caller-supplied `AssumptionSet`.
#[cfg(test)]
mod round78_scoped_eval;

#[cfg(test)]
mod round80_throwing_evaluator;

/// Round-78 StructuralEquality surface: the four deferred `Suite.StructuralEqualityTests` rows
/// (nested-field path, kind, shape and canonical-symbolic equality).
#[cfg(test)]
mod round78_structural_equality;

/// Round-80 abs-rewrite family: the four deferred `AbsSymbolicRoundTripTests` and
/// `UnsatisfiablePropagationTests` rows (the rewriter's own `abs` node, `sqrt(x^2)` under a
/// real domain, substitution at a negative value, and the structured DomainCondition payload).
#[cfg(test)]
mod round80_abs_rewrite;

/// Round-82 series Laurent-tail family: the four deferred `SeriesLaurentAndBranchPointTests`
/// rows (a pole's negative leading power, a second-order pole, sin(x)/x's hidden x^2 term and the
/// kink-resolved branch-point source) plus the truncation-order and negative-exponent boundaries.
#[cfg(test)]
mod round82_series_laurent;

/// Round-82 evalf-precision family: the two `Lovelace.Symbolics.Tests` census rows (the exact
/// controls the truncation repair must not sweep up, and the below-the-cap values that stay
/// inexact zeros), the approximate constant fold that produces those zeros, and the 1000-place
/// digit-cap boundary.
#[cfg(test)]
mod round82_evalf_precision;

/// Round-84 evalir-multi family: the four deferred `DxStructuredResultsTests` census rows whose
/// contract needs a MULTI-PARAMETER kernel (row-shaped batch input, flat whole-row input, the
/// row-width refusal and the whole-rows refusal, plus `compile_full`'s `unknown` result type),
/// with the zero-, one- and duplicate-parameter boundaries.
#[cfg(test)]
mod round84_evalir_multi;

/// Round-9 desktop kernel cancellation: a live flag set from another thread stops ONE long
/// polling kernel (`matmul`) with the structured `Cancelled` failure inside 2 s. The opaque
/// `sqrt(2)` at precision 200000 is documented as uninterruptible, not claimed as cancelled.
#[cfg(test)]
mod kernel_cancel_round9;

/// Round-87 plot-SVG family: descriptor examples #97 (`plot([1, 2, 3])`) and #98
/// (`plot([1, 2, 3], [1, 4, 9], "title")`) whose artefacts diverged 14,636/14,595 vs 326/331
/// bytes. The tests pin the published structure per example — element kinds and counts, the
/// 800x600 canvas, the plot-area frame, both axes, tick geometry and labels, the title text and
/// the dense series polyline — not the byte string.
#[cfg(test)]
mod plot_structure_round87;

/// Round-90 runner/DSP families: the 13 deferred rows across PlotFileWriteRefusal, Dsp,
/// DenseArrayComplex, IntegerToString and Parser. Each test names its census id and asserts the
/// C# case's own observable (typed refusal, signal sample, array metadata, TryFormat buffer, or
/// parser AST shape).
#[cfg(test)]
mod round90_runner_families;

/// Round-90 symbolic families: DomainFailureVocabulary, RelationsAndFamilies, IntegerPow and
/// the solver half of ExactnessFlag. One test per closed rows' observable.
#[cfg(test)]
mod round90_symbolic_families;

/// Round-92 runner/Suite families: the twelve deferred rows across CancellationObservation,
/// SuiteEngineOutputPlot, HelpService, NameSuggestions, CapabilitiesBuiltin and
/// DiagnosticContract. One test per census id, each asserting the oracle-observed observable.
#[cfg(test)]
mod round92_runner_families;

/// Round-92 small symbolic families: the complex symbolic/numeric exactness pairing, linsolve
/// verification by numeric substitution, and the two loop-body recursion-composition rows.
#[cfg(test)]
mod round92_symbolic_families;

/// Round-95 numeric singletons: the batched NdArray matmul and the allocation-refusal declaration
/// rows. One test per census id, asserting the oracle-observed observable.
#[cfg(test)]
mod round95_singletons;

/// Round-93 eval/print families: one test per census id across EvalirDigitArgumentContract,
/// PrintBudget, PrintBudgetTokenBoundary, TruncatingEvaluationStaysInexact, ValueDepthGuard and
/// DocsSync. Every expected value is the C# assertion.
#[cfg(test)]
mod round93_eval_print;

/// Round-97 final tail: the last twelve mappable deferred census rows.
#[cfg(test)]
mod round97_tail;

/// Round-98 differential oracle: the six DifferentialOracleTests census rows, with the SymPy
/// 1.14.0 ground truth embedded as literals.
#[cfg(test)]
mod round98_differential_oracle;
