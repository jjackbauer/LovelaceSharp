//! The Lovelace scripting language: values, the tree-walking interpreter and the engine facade.
//!
//! Rendering is the contract: every value renders with its documented display form and reports a
//! kind, and the engine reports errors as a message plus a UTF-16 position into the source.

#![allow(clippy::needless_range_loop)]

pub mod builtins;
pub mod help;
pub mod complex;
pub mod complex_math;
pub mod numeric;
pub mod registry;
pub mod structured;
pub mod syntax;

/// The cooperative cancellation token every long-running kernel polls. Re-exported so a host can
/// arm a deadline at the same seam the engine does (see `Engine::set_cancel_after`).
pub mod cancel {
    pub use lovelace_symbolics::cancel::*;
}

use std::collections::HashMap;
use std::time::{Duration, Instant};

use lovelace_array::{Field, NdArray};
use lovelace_int::Int;
use lovelace_nat::Nat;
use lovelace_real::Real;
use lovelace_symbolics::Expr as Sym;

use syntax::{BinOp, Expr, IndexArg, InterpPart, Parser, Stmt, Tok};

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
            format!("{stage} depth {depth} exceeds the maximum supported {measure} depth of {limit}. Rewrite the input with less nesting (the engine refuses it before a recursive walk can exhaust the process stack)."),
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

    /// The display form.
    pub fn display(&self) -> String {
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
                    s.push_str(&v.display());
                }
                s.push(']');
                s
            }
            Value::Array(a) => render_array(a),
            Value::Function(name) => format!("<function {}>", name),
            Value::Void => String::new(),
            Value::Symbolic(e) => e.to_string(),
            Value::Domain(name) => name.clone(),
            Value::Enum(_, name) => name.clone(),
            Value::Complex(re, im) => {
                // The reference Complex.ToString: a zero part is omitted entirely, so a real
                // value carried as a Complex (e.g. a DFT element) still displays as "6".
                if im.is_zero() {
                    re.to_string()
                } else if re.is_zero() {
                    format!("{}i", im.to_string())
                } else {
                    let sign = if im.cmp(&Real::zero()) == std::cmp::Ordering::Less { "-" } else { "+" };
                    format!("{} {} {}i", re.to_string(), sign, im.abs().to_string())
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
                    s.push_str(&v.display());
                }
                s.push(')');
                s
            }
        }
    }

    /// The typed form: display plus the kind in parentheses.
    ///
    /// A record publishes its OWN type name (SolveResult, Inspection, …), and an enum publishes
    /// its bare member name — exactly the reference formatter's rules.
    pub fn typed(&self) -> String {
        match self {
            Value::Void => "(void)".to_string(),
            // Text and enum values are published without a kind suffix, matching the reference.
            Value::Text(_) => self.display(),
            Value::Enum(_, name) => name.clone(),
            Value::Record(name, _) => format!("{} ({})", self.display(), name),
            _ => format!("{} ({})", self.display(), self.kind_name()),
        }
    }

    /// True when this is one of the numeric kinds.
    pub fn is_numeric(&self) -> bool {
        matches!(self, Value::Natural(_) | Value::Integer(_) | Value::Real(_) | Value::RealClamped(_, _, _))
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
        DisplayBudget { remaining: MAX_DISPLAY_NODES }
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

fn render_array(a: &NdArray<Value>) -> String {
    fn rec(
        a: &NdArray<Value>,
        shape: &[usize],
        dim: usize,
        idx: &mut Vec<usize>,
        budget: &mut DisplayBudget,
        out: &mut String,
    ) {
        if dim == shape.len() {
            if let Some(v) = a.get(idx) {
                out.push_str(&v.display());
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
            rec(a, shape, dim + 1, idx, budget, out);
            idx.pop();
        }
        out.push(']');
    }
    let shape = a.shape().to_vec();
    if shape.is_empty() {
        return a.get(&[]).map(|v| v.display()).unwrap_or_default();
    }
    let mut out = String::new();
    rec(a, &shape, 0, &mut Vec::new(), &mut DisplayBudget::new(), &mut out);
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

/// The engine: variables, functions, precision and captured output.
pub struct Engine {
    vars: HashMap<String, Value>,
    funcs: HashMap<String, (Vec<String>, Stmt)>,
    precision: usize,
    /// True once the precision knob was changed from its default (mirrors the reference
    /// interpreter's `PrecisionExplicitlySet`, which plugin builtins read).
    precision_explicit: bool,
    output: Vec<String>,
    plot: Option<PlotCapture>,
    plot_dir: String,
    plot_file: String,
    revision: u64,
    timings: Vec<OpTiming>,
    /// Live user-function frames, in budget units of six each (the reference's frame + scope +
    /// body entry plus the statement/expression charges its body walk pays).
    call_units: usize,
    /// The caller's deadline, when one was armed for the next evaluation (--cancel-after).
    cancel_after: Option<Duration>,
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
    bounds: Vec<(String, Option<(lovelace_symbolics::Rat, bool)>, Option<(lovelace_symbolics::Rat, bool)>)>,
    /// The session's assumption set: the tri-state kernel the assume builtins feed and merge,
    /// solve, simplify, simplify_full and inspect read. One set per engine, so two engines that
    /// run concurrently never answer each other's queries.
    assumptions: lovelace_symbolics::assumptions::AssumptionSet,
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
            output: Vec::new(),
            plot: None,
            plot_dir: dir.to_string_lossy().to_string(),
            plot_file: "plot.svg".to_string(),
            revision: 0,
            timings: Vec::new(),
            call_units: 0,
            cancel_after: None,
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
        }
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
        if self.precision_explicit {
            self.precision
        } else {
            100
        }
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

    /// Drains captured print output.
    pub fn take_output(&mut self) -> Vec<String> {
        std::mem::take(&mut self.output)
    }

    /// Renders a value in display form.
    pub fn format(&self, value: &Value) -> String {
        value.display()
    }

    /// Renders a value with its kind.
    pub fn format_typed(&self, value: &Value) -> String {
        value.typed()
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

    /// Evaluates a script and returns its final value.
    ///
    /// The deadline is armed for exactly this evaluation (and restored on the way out, so a
    /// second run on the same engine starts un-budgeted unless the caller arms one again). A
    /// kernel that observed it unwinds early; whatever value it returned is discarded and the
    /// structured cancellation failure is published instead.
    pub fn eval(&mut self, source: &str) -> Result<Value, LangError> {
        let deadline = self.cancel_after.map(|after| Instant::now() + after);
        let scope = lovelace_symbolics::cancel::arm(deadline);
        self.cancelled = false;
        let mut result = self.eval_inner(source);
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
            return Err(LangError::depth("expression tree", depth, MAX_PARSED_TREE_DEPTH, "nesting"));
        }
        self.call_units = 0;
        let mut last = Value::Void;
        for stmt in &program {
            let position = statement_position(stmt);
            self.current_position = position;
            // Statement granularity, exactly as the reference's interpreter checks: a script whose
            // budget expires between two statements stops before the next one runs.
            if lovelace_symbolics::cancel::check() {
                return Err(cancelled_error().with_position(position));
            }
            let printed_before = self.output.len();
            match self.exec(stmt) {
                Ok(Flow::Value(v)) => {
                    self.timings.push(OpTiming {
                        position,
                        result_kind: v.kind_name().to_string(),
                        has_output: self.output.len() > printed_before,
                    });
                    self.last_completed_position = position;
                    last = v;
                }
                Ok(Flow::Return(v)) => {
                    self.timings.push(OpTiming {
                        position,
                        result_kind: v.kind_name().to_string(),
                        has_output: self.output.len() > printed_before,
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
        let last_depth = value_depth(&last);
        if last_depth > MAX_VALUE_DEPTH {
            return Err(LangError::depth("value", last_depth, MAX_VALUE_DEPTH, "nesting"));
        }
        for value in self.vars.values() {
            let depth = value_depth(value);
            if depth > MAX_VALUE_DEPTH {
                return Err(LangError::depth("value", depth, MAX_VALUE_DEPTH, "nesting"));
            }
        }
        self.revision += 1;
        if !matches!(last, Value::Void) {
            self.vars.insert("_".to_string(), last.clone());
        }
        Ok(last)
    }

    fn exec(&mut self, stmt: &Stmt) -> Result<Flow, LangError> {
        match stmt {
            Stmt::Empty => Ok(Flow::Value(Value::Void)),
            Stmt::Expr(e, _) => {
                let v = self.eval_expr(e)?;
                Ok(Flow::Value(v))
            }
            Stmt::Block(stmts) => {
                let mut last = Value::Void;
                for s in stmts {
                    match self.exec(s)? {
                        Flow::Value(v) => last = v,
                        other => return Ok(other),
                    }
                }
                Ok(Flow::Value(last))
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
                            )))
                        }
                    }
                    match self.exec(body)? {
                        Flow::Break => break,
                        Flow::Continue | Flow::Value(_) => {}
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                    }
                }
                Ok(Flow::Value(Value::Void))
            }
            Stmt::For(var, range, body) => {
                let r = self.eval_expr(range)?;
                let items = match r {
                    Value::Vector(v) => v,
                    other => {
                        return Err(LangError::plain(format!(
                            "for loop needs a range, but got '{}'.",
                            other.kind_name()
                        )))
                    }
                };
                for (iteration, item) in items.into_iter().enumerate() {
                    if lovelace_symbolics::cancel::poll(iteration as u64) {
                        return Err(cancelled_error());
                    }
                    self.vars.insert(var.clone(), item);
                    match self.exec(body)? {
                        Flow::Break => break,
                        Flow::Continue => continue,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        Flow::Value(_) => {}
                    }
                }
                Ok(Flow::Value(Value::Void))
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
                self.funcs.insert(name.clone(), (params.clone(), (**body).clone()));
                Ok(Flow::Value(Value::Void))
            }
        }
    }

    fn eval_expr(&mut self, e: &Expr) -> Result<Value, LangError> {
        match e {
            Expr::Number(text) => {
                if text.contains('.') {
                    Ok(Value::Real(Real::from_str(text).ok_or_else(|| {
                        LangError::plain(format!("Invalid number literal '{}'.", text))
                    })?))
                } else {
                    Ok(Value::Natural(Nat::from_decimal(text).ok_or_else(|| {
                        LangError::plain(format!("Invalid number literal '{}'.", text))
                    })?))
                }
            }
            Expr::Repeating(text) => Ok(Value::Real(Real::from_str(text).ok_or_else(|| {
                LangError::plain(format!("Invalid number literal '{}'.", text))
            })?)),
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
            Expr::Var(name, pos) => match self.vars.get(name) {
                Some(v) => Ok(v.clone()),
                // Mathematical constants, resolved after the variable store so a user
                // assignment shadows them; `pi` and `e` render at the display precision.
                None if name == "pi" => Ok(Value::Real(Real::pi(self.display_precision()))),
                None if name == "e" => Ok(Value::Real(builtins::e_constant(self.display_precision()))),
                None if name == "inf" => Ok(Value::Symbolic(Sym::Fun("inf".to_string(), Vec::new()))),
                None if matches!(name.as_str(), "real" | "complex" | "integer" | "rational") => {
                    Ok(Value::Domain(name.clone()))
                }
                None => match self.funcs.get(name) {
                    Some(_) => Ok(Value::Function(name.clone())),
                    None => Err(LangError::at(
                        format!("Undefined variable '{}'.", name),
                        *pos,
                    )),
                },
            },
            Expr::Call(name, args, pos) => {
                let mut values = Vec::with_capacity(args.len());
                for a in args {
                    values.push(self.eval_expr(a)?);
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
                let mut acc = self.eval_expr(node)?;
                for operand in spine.iter().rev() {
                    let rv = self.eval_expr(operand)?;
                    acc = builtins::binary(*op, &acc, &rv, self.precision)?;
                }
                Ok(acc)
            }
            Expr::Neg(inner) => {
                let v = self.eval_expr(inner)?;
                builtins::neg(&v)
            }
            Expr::Not(inner) => {
                let v = self.eval_expr(inner)?;
                match v {
                    Value::Symbolic(e) => Ok(Value::Symbolic(Sym::Fun("not".to_string(), vec![e]))),
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
                        Some(k) => Ok(Value::Natural(Nat::factorial(k))),
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
                            Some(k) => Ok(Value::Integer(Int::factorial(k))),
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
                    Value::Record(_, fields) => fields
                        .iter()
                        .find(|(k, _)| k == field)
                        .map(|(_, v)| v.clone())
                        .ok_or_else(|| {
                            LangError::plain(format!("No field named '{}'.", field))
                        }),
                    other => Err(LangError::plain(format!(
                        "Cannot read field '{}' from a value of kind '{}'.",
                        field,
                        other.kind_name()
                    ))),
                }
            }
            Expr::Assign(name, value) => {
                let v = self.eval_expr(value)?;
                self.vars.insert(name.clone(), v.clone());
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
            let saved: Vec<(String, Option<Value>)> = params
                .iter()
                .map(|p| (p.clone(), self.vars.get(p).cloned()))
                .collect();
            for (p, v) in params.iter().zip(args.into_iter()) {
                self.vars.insert(p.clone(), v);
            }
            // One call LEVEL of the evaluation budget. The reference charges four units for the
            // frame, its scope and the body entry, and the body's statements and descending
            // expressions pay the rest: six units per level reproduces its published boundary
            // (f(85) answers, f(86) is refused; a live wrapper costs one level).
            self.call_units += 6;
            // The statement's entry into user code is the ZERO of the measurement: the first
            // frame's six units are the base, exactly as the reference establishes its base
            // before charging the frame. f(85) therefore measures 510 and answers; f(86)
            // measures 516 and is refused.
            let depth = self.call_units - 6;
            if depth > MAX_EVALUATION_DEPTH {
                self.call_units -= 6;
                // The reference checks one unit at a time and publishes the FIRST level it found
                // past the budget, so that number is always limit + 1 even when the remaining
                // call chain would measure deeper.
                let reported = (MAX_EVALUATION_DEPTH + 1).min(depth);
                let result = Err(LangError::depth("evaluation", reported, MAX_EVALUATION_DEPTH, "evaluation"));
                for (p, old) in saved {
                    match old {
                        Some(v) => self.vars.insert(p, v),
                        None => self.vars.remove(&p),
                    };
                }
                return result;
            }
            let result = self.exec(&body);
            self.call_units -= 6;
            for (p, old) in saved {
                match old {
                    Some(v) => self.vars.insert(p, v),
                    None => self.vars.remove(&p),
                };
            }
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
        // A builtin that failed without naming its own position failed at the CALL: the
        // published diagnostic points at the call site, which is where the caller must look.
        match builtins::call(self, name, args, pos) {
            // A cancellation is not a call-site argument refusal: its position is the statement
            // that was running, supplied by the statement loop, never the inner call site.
            Err(e) if e.position == 0 && e.code != "Cancelled" => {
                Err(e.with_position(pos))
            }
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
        if self.precision_explicit { self.precision } else { 30 }
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

    pub(crate) fn bounds_of(&self, var: &str) -> Option<(Option<(lovelace_symbolics::Rat, bool)>, Option<(lovelace_symbolics::Rat, bool)>)> {
        self.bounds
            .iter()
            .find(|(name, _, _)| name == var)
            .map(|(_, lo, hi)| (lo.clone(), hi.clone()))
    }

    pub(crate) fn assumption_texts(&self) -> Vec<String> {
        self.assumption_texts.clone()
    }

    /// Records one assumption ATOM (the expression form of the same fact the text carries).
    pub(crate) fn record_assumption_atom(&mut self, atom: &Sym) {
        let text = atom.to_string();
        if !self.assumption_atoms.iter().any(|existing| existing.to_string() == text) {
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
        if !self.domain_assumptions.iter().any(|(name, existing)| name == var && existing == domain) {
            self.domain_assumptions.push((var.to_string(), domain.to_string()));
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

    pub(crate) fn positive_symbols(&self) -> &[String] {
        &self.positive_symbols
    }

    pub(crate) fn nonzero_symbols(&self) -> &[String] {
        &self.nonzero_symbols
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
        return Err(LangError::depth("symbolic expression", reported, MAX_SYMBOLIC_DEPTH, "nesting"));
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
            Sym::Num(_) | Sym::Real(_) | Sym::Sym(_) => {}
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
    let mut stack: Vec<(AstNode<'_>, usize)> =
        program.iter().rev().map(|s| (AstNode::Stmt(s), 2usize)).collect();
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
                Expr::Neg(inner) | Expr::Not(inner) | Expr::Fact(inner) => {
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
mod dsp_round20;

#[cfg(test)]
mod dsp_round21;

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

mod symbolics_round23;

#[cfg(test)]
mod assumptions_round35;

#[cfg(test)]
mod real_constant_round35;

#[cfg(test)]
mod suite_tail_round35;

#[cfg(test)]
mod round35_extra_tail;
