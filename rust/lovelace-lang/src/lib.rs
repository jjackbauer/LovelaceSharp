//! The Lovelace scripting language: values, the tree-walking interpreter and the engine facade.
//!
//! Rendering is the contract: every value renders with its documented display form and reports a
//! kind, and the engine reports errors as a message plus a UTF-16 position into the source.

#![allow(clippy::needless_range_loop)]

pub mod builtins;
pub mod syntax;

use std::collections::HashMap;

use lovelace_array::{Field, NdArray};
use lovelace_int::Int;
use lovelace_nat::Nat;
use lovelace_real::Real;
use lovelace_symbolics::Expr as Sym;

use syntax::{BinOp, Expr, IndexArg, InterpPart, Parser, Stmt, Tok};

/// An evaluation failure: a message and a UTF-16 code-unit position.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LangError {
    /// The message, a complete sentence ending with a period.
    pub message: String,
    /// UTF-16 code-unit offset into the source.
    pub position: usize,
}

impl LangError {
    /// Builds an error without a position.
    pub fn at(message: impl Into<String>, position: usize) -> LangError {
        LangError { message: message.into(), position }
    }

    /// Builds an error at position zero.
    pub fn plain(message: impl Into<String>) -> LangError {
        LangError { message: message.into(), position: 0 }
    }
}

impl std::fmt::Display for LangError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for LangError {}

/// A runtime value.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// A natural number.
    Natural(Nat),
    /// A signed integer.
    Integer(Int),
    /// A real number.
    Real(Real),
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
}

impl Value {
    /// The kind name published in typed renderings.
    pub fn kind_name(&self) -> &str {
        match self {
            Value::Natural(_) => "Natural",
            Value::Integer(_) => "Integer",
            Value::Real(_) => "Real",
            Value::Boolean(_) => "Boolean",
            Value::Text(_) => "Text",
            Value::Vector(_) => "Vector",
            Value::Array(_) => "Array",
            Value::Function(_) => "Function",
            Value::Void => "Void",
            Value::Complex(_, _) => "Complex",
            Value::Symbolic(_) => "Symbolic",
            Value::Domain(_) => "Domain",
            // A record is published under its own type name (DivRemResult, SolveResult, ...).
            Value::Record(name, _) => name.as_str(),
        }
    }

    /// The display form.
    pub fn display(&self) -> String {
        match self {
            Value::Natural(n) => n.to_decimal(),
            Value::Integer(i) => i.to_decimal(),
            Value::Real(r) => r.to_string(),
            Value::Boolean(b) => if *b { "True" } else { "False" }.to_string(),
            Value::Text(t) => t.clone(),
            Value::Vector(items) => {
                let mut s = String::from("[");
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        s.push_str(", ");
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
            Value::Complex(re, im) => {
                let sign = if im.cmp(&Real::zero()) == std::cmp::Ordering::Less { "-" } else { "+" };
                format!("{} {} {}i", re.to_string(), sign, im.abs().to_string())
            }
            Value::Record(name, fields) => {
                let mut s = String::from(name);
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
    pub fn typed(&self) -> String {
        if matches!(self, Value::Void) {
            return "() (Void)".to_string();
        }
        // Text and domain values are published without a kind suffix, matching the reference.
        if matches!(self, Value::Text(_)) {
            return self.display();
        }
        format!("{} ({})", self.display(), self.kind_name())
    }

    /// True when this is one of the numeric kinds.
    pub fn is_numeric(&self) -> bool {
        matches!(self, Value::Natural(_) | Value::Integer(_) | Value::Real(_))
    }
}

fn render_array(a: &NdArray<Value>) -> String {
    fn rec(a: &NdArray<Value>, shape: &[usize], dim: usize, idx: &mut Vec<usize>, out: &mut String) {
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
            idx.push(i);
            rec(a, shape, dim + 1, idx, out);
            idx.pop();
        }
        out.push(']');
    }
    let shape = a.shape().to_vec();
    if shape.is_empty() {
        return a.get(&[]).map(|v| v.display()).unwrap_or_default();
    }
    let mut out = String::new();
    rec(a, &shape, 0, &mut Vec::new(), &mut out);
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
            Value::Real(r) => r.is_zero(),
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
    output: Vec<String>,
    plot: Option<PlotCapture>,
    plot_dir: String,
    revision: u64,
    positive_symbols: Vec<String>,
    nonzero_symbols: Vec<String>,
    assumption_texts: Vec<String>,
    bounds: Vec<(String, Option<(lovelace_symbolics::Rat, bool)>, Option<(lovelace_symbolics::Rat, bool)>)>,
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
            output: Vec::new(),
            plot: None,
            plot_dir: dir.to_string_lossy().to_string(),
            revision: 0,
            positive_symbols: Vec::new(),
            nonzero_symbols: Vec::new(),
            assumption_texts: Vec::new(),
            bounds: Vec::new(),
        }
    }

    /// Sets the computation and display precision (decimal places).
    pub fn set_precision(&mut self, decimals: usize) {
        self.precision = decimals.max(1);
    }

    /// The current precision.
    pub fn precision(&self) -> usize {
        self.precision
    }

    /// Sets the directory plots are written to.
    pub fn set_plot_dir(&mut self, dir: &str) {
        self.plot_dir = dir.to_string();
        let _ = std::fs::create_dir_all(&self.plot_dir);
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
    pub fn eval(&mut self, source: &str) -> Result<Value, LangError> {
        self.plot = None;
        let toks = syntax::tokenize(source)?;
        let mut parser = Parser::new(toks);
        let program = parser.program()?;
        let mut last = Value::Void;
        for stmt in &program {
            match self.exec(stmt)? {
                Flow::Value(v) => last = v,
                Flow::Return(v) => {
                    last = v;
                    break;
                }
                Flow::Break | Flow::Continue => {
                    return Err(LangError::plain("'break' is only valid inside a loop."));
                }
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
                loop {
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
                for item in items {
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
            Expr::Binary(op, l, r) => {
                let lv = self.eval_expr(l)?;
                let rv = self.eval_expr(r)?;
                builtins::binary(*op, &lv, &rv, self.precision)
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
                    other => Err(LangError::plain(format!(
                        "Factorial needs a Natural, but got '{}'.",
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
            let result = self.exec(&body);
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
        builtins::call(self, name, args, pos)
    }

    pub(crate) fn precision_value(&self) -> usize {
        self.precision
    }

    pub(crate) fn capture_plot(&mut self, capture: PlotCapture) {
        self.plot = Some(capture);
    }

    pub(crate) fn plot_dir(&self) -> String {
        self.plot_dir.clone()
    }

    pub(crate) fn push_output(&mut self, line: String) {
        self.output.push(line);
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
