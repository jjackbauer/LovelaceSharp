//! A deterministic, dependency-free interpreter used as the default engine.
//!
//! This is intentionally not the Lovelace language: it is the smallest evaluator that lets the
//! desktop application, its tests and --selftest exercise real document state before
//! lovelace-lang lands. It understands integer and text values, assignment, + - * / %,
//! parentheses, print(), plot(), abs(), min(), max() and clear(), with statements separated by
//! semicolons or newlines. Error messages follow the corpus style of the real engine: a plain
//! sentence ending with a period.

use std::collections::BTreeMap;

use super::{DEFAULT_PRECISION, EngineFunction, EngineState, EngineVariable, Evaluator, PlotCapture};
use crate::svg;

/// Creates a boxed FakeEngine.
#[cfg(not(feature = "real-engine"))]
pub fn boxed() -> Box<dyn Evaluator> {
    Box::new(FakeEngine::new())
}

/// A value in the fake engine.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Value {
    Int(i64),
    Text(String),
    Void,
}

impl Value {
    fn display(&self) -> String {
        match self {
            Value::Int(value) => value.to_string(),
            Value::Text(text) => text.clone(),
            Value::Void => "Void".to_string(),
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            Value::Int(_) => "Natural",
            Value::Text(_) => "Text",
            Value::Void => "Void",
        }
    }
}

/// A tiny interpreter that keeps variables, print output and the last plot.
#[derive(Debug)]
pub struct FakeEngine {
    vars: BTreeMap<String, Value>,
    precision: usize,
    revision: u64,
    output: Vec<String>,
    plot: Option<PlotCapture>,
    plots: u64,
}

impl Default for FakeEngine {
    fn default() -> FakeEngine {
        FakeEngine::new()
    }
}

impl FakeEngine {
    /// Creates an engine with the default precision and no variables.
    pub fn new() -> FakeEngine {
        FakeEngine {
            vars: BTreeMap::new(),
            precision: DEFAULT_PRECISION,
            revision: 0,
            output: Vec::new(),
            plot: None,
            plots: 0,
        }
    }

    fn statement(&mut self, src: &str) -> Result<Option<String>, String> {
        if let Some((name, rhs)) = split_assignment(src) {
            let mut parser = ExprParser::new(rhs);
            let value = parser.expr(self)?;
            parser.finish()?;
            if matches!(value, Value::Void) {
                return Err("The right-hand side of an assignment must be a value.".to_string());
            }
            self.vars.insert(name, value.clone());
            return Ok(Some(value.display()));
        }
        let mut parser = ExprParser::new(src);
        let value = parser.expr(self)?;
        parser.finish()?;
        Ok(if matches!(value, Value::Void) {
            None
        } else {
            Some(value.display())
        })
    }

    fn builtin(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        match name {
            "print" => {
                let mut line = String::new();
                for (index, arg) in args.iter().enumerate() {
                    if index > 0 {
                        line.push(' ');
                    }
                    line.push_str(&arg.display());
                }
                self.output.push(line);
                Ok(Value::Void)
            }
            "plot" => {
                let mut title = "plot".to_string();
                let mut ys: Vec<i64> = Vec::new();
                for arg in args {
                    match arg {
                        Value::Text(text) => title = text,
                        Value::Int(value) => ys.push(value),
                        Value::Void => {
                            return Err("plot does not accept a Void argument.".to_string());
                        }
                    }
                }
                if ys.len() < 2 {
                    return Err("plot needs at least two values.".to_string());
                }
                self.plots += 1;
                let svg = svg::make_plot_svg(&title, &ys);
                self.plot = Some(PlotCapture {
                    path: format!("plots/plot-{}.svg", self.plots),
                    title,
                    svg,
                });
                Ok(Value::Void)
            }
            "abs" => {
                let value = expect_int(args.into_iter().next(), "abs")?;
                value
                    .checked_abs()
                    .map(Value::Int)
                    .ok_or_else(|| "Integer overflow.".to_string())
            }
            "min" | "max" => {
                if args.is_empty() {
                    return Err(format!("{name} needs at least one argument."));
                }
                let mut accumulator: Option<i64> = None;
                for arg in args {
                    let value = match arg {
                        Value::Int(value) => value,
                        other => {
                            return Err(format!("Expected a number, but got {}.", other.kind()));
                        }
                    };
                    accumulator = Some(match accumulator {
                        None => value,
                        Some(current) if name == "min" => current.min(value),
                        Some(current) => current.max(value),
                    });
                }
                Ok(Value::Int(accumulator.unwrap_or(0)))
            }
            "clear" => {
                self.vars.clear();
                Ok(Value::Void)
            }
            other => Err(format!("Unknown function '{other}'.")),
        }
    }
}

impl Evaluator for FakeEngine {
    fn eval(&mut self, src: &str) -> Result<String, String> {
        let mut last: Option<String> = None;
        for raw in split_statements(src) {
            let statement = raw.trim();
            if statement.is_empty() {
                continue;
            }
            match self.statement(statement) {
                Ok(Some(display)) => last = Some(display),
                Ok(None) => {}
                Err(error) => {
                    self.revision = self.revision.saturating_add(1);
                    return Err(error);
                }
            }
        }
        self.revision = self.revision.saturating_add(1);
        Ok(last.unwrap_or_else(|| "Void".to_string()))
    }

    fn set_precision(&mut self, decimals: usize) {
        self.precision = decimals.max(1);
    }

    fn precision(&self) -> usize {
        self.precision
    }

    fn revision(&self) -> u64 {
        self.revision
    }

    fn variables(&self) -> Vec<(String, String)> {
        self.vars
            .iter()
            .map(|(name, value)| (name.clone(), value.display()))
            .collect()
    }

    fn functions(&self) -> Vec<(String, Vec<String>)> {
        builtin_functions()
    }

    /// The fake's own symbol table: the kinds the values actually hold and its own
    /// [`builtin_functions`] registry. Overriding the trait default is what makes completion in
    /// `--no-default-features` builds report real kinds instead of `Value`.
    fn state(&self) -> Option<EngineState> {
        Some(EngineState {
            variables: self
                .vars
                .iter()
                .map(|(name, value)| EngineVariable {
                    name: name.clone(),
                    kind: value.kind().to_string(),
                    display: value.display(),
                })
                .collect(),
            functions: builtin_functions()
                .into_iter()
                .map(|(name, parameters)| EngineFunction {
                    name,
                    parameters,
                    is_builtin: true,
                })
                .collect(),
        })
    }

    fn take_output(&mut self) -> Vec<String> {
        std::mem::take(&mut self.output)
    }

    fn last_plot(&self) -> Option<PlotCapture> {
        self.plot.clone()
    }

    fn clear(&mut self) {
        self.vars.clear();
        self.plot = None;
        self.revision = self.revision.saturating_add(1);
    }

    fn name(&self) -> &'static str {
        "fake"
    }
}

/// The builtin signatures the workspace pane lists.
pub fn builtin_functions() -> Vec<(String, Vec<String>)> {
    vec![
        ("abs".to_string(), vec!["x".to_string()]),
        ("clear".to_string(), Vec::new()),
        ("max".to_string(), vec!["a".to_string(), "b".to_string()]),
        ("min".to_string(), vec!["a".to_string(), "b".to_string()]),
        ("plot".to_string(), vec!["values".to_string()]),
        ("print".to_string(), vec!["values".to_string()]),
    ]
}

/// Splits a program into statements at semicolons and newlines that are not inside brackets
/// or string literals.
fn split_statements(src: &str) -> Vec<&str> {
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut depth = 0i32;
    let mut in_string: Option<u8> = None;
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if let Some(quote) = in_string {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == quote {
                in_string = None;
            }
            i += 1;
            continue;
        }
        match b {
            b'"' | b'\'' => in_string = Some(b),
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b';' | b'\n' if depth <= 0 => {
                out.push(&src[start..i]);
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    out.push(&src[start..]);
    out
}

/// Recognises "name = expression" at the top level, outside strings and brackets.
fn split_assignment(src: &str) -> Option<(String, &str)> {
    let bytes = src.as_bytes();
    let mut depth = 0i32;
    let mut in_string: Option<u8> = None;
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if let Some(quote) = in_string {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == quote {
                in_string = None;
            }
            i += 1;
            continue;
        }
        match b {
            b'"' | b'\'' => in_string = Some(b),
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b'=' if depth == 0 => {
                if bytes.get(i + 1).copied() == Some(b'=') {
                    return None;
                }
                let lhs = src[..i].trim();
                if is_ident(lhs) {
                    return Some((lhs.to_string(), &src[i + 1..]));
                }
                return None;
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// True when the text is a single identifier.
fn is_ident(text: &str) -> bool {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) if first.is_alphabetic() || first == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_alphanumeric() || c == '_')
}

fn expect_int(value: Option<Value>, name: &str) -> Result<i64, String> {
    match value {
        Some(Value::Int(value)) => Ok(value),
        Some(other) => Err(format!("Expected a number, but got {}.", other.kind())),
        None => Err(format!("{name} needs an argument.")),
    }
}

fn add(left: Value, right: Value) -> Result<Value, String> {
    match (left, right) {
        (Value::Int(a), Value::Int(b)) => a
            .checked_add(b)
            .map(Value::Int)
            .ok_or_else(|| "Integer overflow.".to_string()),
        (Value::Text(a), Value::Text(b)) => Ok(Value::Text(format!("{a}{b}"))),
        (a, b) => Err(format!("Cannot add {} and {}.", a.kind(), b.kind())),
    }
}

fn sub(left: Value, right: Value) -> Result<Value, String> {
    match (left, right) {
        (Value::Int(a), Value::Int(b)) => a
            .checked_sub(b)
            .map(Value::Int)
            .ok_or_else(|| "Integer overflow.".to_string()),
        (a, b) => Err(format!("Cannot subtract {} and {}.", a.kind(), b.kind())),
    }
}

fn mul(left: Value, right: Value) -> Result<Value, String> {
    match (left, right) {
        (Value::Int(a), Value::Int(b)) => a
            .checked_mul(b)
            .map(Value::Int)
            .ok_or_else(|| "Integer overflow.".to_string()),
        (a, b) => Err(format!("Cannot multiply {} and {}.", a.kind(), b.kind())),
    }
}

fn div(left: Value, right: Value) -> Result<Value, String> {
    match (left, right) {
        (Value::Int(_), Value::Int(0)) => Err("Cannot divide by zero.".to_string()),
        (Value::Int(a), Value::Int(b)) => a
            .checked_div(b)
            .map(Value::Int)
            .ok_or_else(|| "Integer overflow.".to_string()),
        (a, b) => Err(format!("Cannot divide {} by {}.", a.kind(), b.kind())),
    }
}

fn rem(left: Value, right: Value) -> Result<Value, String> {
    match (left, right) {
        (Value::Int(_), Value::Int(0)) => Err("Cannot divide by zero.".to_string()),
        (Value::Int(a), Value::Int(b)) => a
            .checked_rem(b)
            .map(Value::Int)
            .ok_or_else(|| "Integer overflow.".to_string()),
        (a, b) => Err(format!("Cannot divide {} by {}.", a.kind(), b.kind())),
    }
}

/// A recursive-descent expression parser over one statement.
struct ExprParser {
    chars: Vec<char>,
    pos: usize,
}

impl ExprParser {
    fn new(src: &str) -> ExprParser {
        ExprParser {
            chars: src.chars().collect(),
            pos: 0,
        }
    }

    fn finish(&self) -> Result<(), String> {
        match self.chars.get(self.pos) {
            None => Ok(()),
            Some(ch) if ch.is_whitespace() => Ok(()),
            Some(ch) => Err(format!("Unexpected character '{ch}'.")),
        }
    }

    fn skip_ws(&mut self) {
        while let Some(ch) = self.chars.get(self.pos) {
            if ch.is_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn expr(&mut self, engine: &mut FakeEngine) -> Result<Value, String> {
        let mut left = self.term(engine)?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some('+') => {
                    self.pos += 1;
                    let right = self.term(engine)?;
                    left = add(left, right)?;
                }
                Some('-') => {
                    self.pos += 1;
                    let right = self.term(engine)?;
                    left = sub(left, right)?;
                }
                _ => return Ok(left),
            }
        }
    }

    fn term(&mut self, engine: &mut FakeEngine) -> Result<Value, String> {
        let mut left = self.unary(engine)?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some('*') => {
                    self.pos += 1;
                    let right = self.unary(engine)?;
                    left = mul(left, right)?;
                }
                Some('/') => {
                    self.pos += 1;
                    let right = self.unary(engine)?;
                    left = div(left, right)?;
                }
                Some('%') => {
                    self.pos += 1;
                    let right = self.unary(engine)?;
                    left = rem(left, right)?;
                }
                _ => return Ok(left),
            }
        }
    }

    fn unary(&mut self, engine: &mut FakeEngine) -> Result<Value, String> {
        self.skip_ws();
        match self.peek() {
            Some('-') => {
                self.pos += 1;
                let value = self.unary(engine)?;
                match value {
                    Value::Int(number) => number
                        .checked_neg()
                        .map(Value::Int)
                        .ok_or_else(|| "Integer overflow.".to_string()),
                    other => Err(format!("Cannot negate {}.", other.kind())),
                }
            }
            Some('+') => {
                self.pos += 1;
                self.unary(engine)
            }
            _ => self.primary(engine),
        }
    }

    fn primary(&mut self, engine: &mut FakeEngine) -> Result<Value, String> {
        self.skip_ws();
        match self.peek() {
            Some('(') => {
                self.pos += 1;
                let value = self.expr(engine)?;
                self.skip_ws();
                self.expect(')')?;
                Ok(value)
            }
            Some('"') | Some('\'') => self.string(),
            Some(ch) if ch.is_ascii_digit() => self.number(),
            Some(ch) if ch.is_alphabetic() || ch == '_' => {
                let name = self.ident();
                self.skip_ws();
                if self.peek() == Some('(') {
                    self.call(&name, engine)
                } else {
                    engine
                        .vars
                        .get(&name)
                        .cloned()
                        .ok_or_else(|| format!("Undefined variable '{name}'."))
                }
            }
            Some(ch) => Err(format!("Unexpected character '{ch}'.")),
            None => Err("Expected an expression.".to_string()),
        }
    }

    fn call(&mut self, name: &str, engine: &mut FakeEngine) -> Result<Value, String> {
        self.expect('(')?;
        let mut args = Vec::new();
        self.skip_ws();
        if self.peek() == Some(')') {
            self.pos += 1;
        } else {
            loop {
                args.push(self.expr(engine)?);
                self.skip_ws();
                match self.peek() {
                    Some(',') => {
                        self.pos += 1;
                    }
                    Some(')') => {
                        self.pos += 1;
                        break;
                    }
                    _ => {
                        return Err("Expected ',' or ')' in an argument list.".to_string());
                    }
                }
            }
        }
        engine.builtin(name, args)
    }

    fn ident(&mut self) -> String {
        let start = self.pos;
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                self.pos += 1;
            } else {
                break;
            }
        }
        self.chars[start..self.pos].iter().collect()
    }

    fn number(&mut self) -> Result<Value, String> {
        let start = self.pos;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        text.parse::<i64>()
            .map(Value::Int)
            .map_err(|_| "Integer literal is out of range.".to_string())
    }

    fn string(&mut self) -> Result<Value, String> {
        let quote = match self.peek() {
            Some(ch) => ch,
            None => return Err("Expected a string literal.".to_string()),
        };
        self.pos += 1;
        let mut text = String::new();
        loop {
            match self.chars.get(self.pos).copied() {
                None => return Err("Unterminated string literal.".to_string()),
                Some('\\') => {
                    self.pos += 1;
                    match self.chars.get(self.pos).copied() {
                        Some('n') => text.push('\n'),
                        Some('t') => text.push('\t'),
                        Some('\\') => text.push('\\'),
                        Some('"') => text.push('"'),
                        Some('\'') => text.push('\''),
                        Some(other) => text.push(other),
                        None => return Err("Unterminated string literal.".to_string()),
                    }
                    self.pos += 1;
                }
                Some(ch) if ch == quote => {
                    self.pos += 1;
                    return Ok(Value::Text(text));
                }
                Some(ch) => {
                    text.push(ch);
                    self.pos += 1;
                }
            }
        }
    }

    fn expect(&mut self, expected: char) -> Result<(), String> {
        self.skip_ws();
        match self.peek() {
            Some(ch) if ch == expected => {
                self.pos += 1;
                Ok(())
            }
            Some(ch) => Err(format!("Expected '{expected}', found '{ch}'.")),
            None => Err(format!("Expected '{expected}'.")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_uses_the_corpus_error_style() {
        let mut engine = FakeEngine::new();
        assert_eq!(engine.eval("1 + 2 * 3").unwrap(), "7");
        assert_eq!(engine.eval("(1 + 2) * 3").unwrap(), "9");
        assert_eq!(engine.eval("7 / 2").unwrap(), "3");
        assert_eq!(engine.eval("7 % 2").unwrap(), "1");
        assert_eq!(engine.eval("-4 + 10").unwrap(), "6");
        assert_eq!(engine.eval("1 / 0").unwrap_err(), "Cannot divide by zero.");
    }

    #[test]
    fn assignment_then_reference() {
        let mut engine = FakeEngine::new();
        assert_eq!(engine.eval("n = 41\nn + 1").unwrap(), "42");
        assert_eq!(
            engine.variables(),
            vec![("n".to_string(), "41".to_string())]
        );
        assert_eq!(engine.revision(), 1);
    }

    #[test]
    fn undefined_variable_names_itself() {
        let mut engine = FakeEngine::new();
        assert_eq!(
            engine.eval("nope + 1").unwrap_err(),
            "Undefined variable 'nope'."
        );
    }

    #[test]
    fn print_collects_output_and_returns_void() {
        let mut engine = FakeEngine::new();
        let value = engine.eval("print(\"hello\")\nprint(1 + 1)").unwrap();
        assert_eq!(value, "Void");
        assert_eq!(
            engine.take_output(),
            vec!["hello".to_string(), "2".to_string()]
        );
        assert!(engine.take_output().is_empty());
    }

    #[test]
    fn plot_generates_svg_the_reader_understands() {
        let mut engine = FakeEngine::new();
        engine.eval("plot(\"trend\", 1, 4, 2, 8)").unwrap();
        let plot = engine.last_plot().expect("a plot");
        assert_eq!(plot.title, "trend");
        assert!(plot.path.ends_with(".svg"));
        let parsed = svg::parse(&plot.svg);
        assert_eq!(parsed.title.as_deref(), Some("trend"));
        assert_eq!(parsed.polylines.len(), 1);
        assert_eq!(parsed.polylines[0].points.len(), 4);
    }

    #[test]
    fn builtins_and_clear() {
        let mut engine = FakeEngine::new();
        assert_eq!(engine.eval("abs(-9)").unwrap(), "9");
        assert_eq!(engine.eval("min(3, 1, 2)").unwrap(), "1");
        assert_eq!(engine.eval("max(3, 1, 2)").unwrap(), "3");
        engine.eval("x = 1").unwrap();
        engine.clear();
        assert!(engine.variables().is_empty());
        assert!(engine.eval("x").is_err());
        assert_eq!(engine.functions().len(), builtin_functions().len());
    }

    #[test]
    fn precision_is_stored_and_clamped() {
        let mut engine = FakeEngine::new();
        assert_eq!(engine.precision(), DEFAULT_PRECISION);
        engine.set_precision(50);
        assert_eq!(engine.precision(), 50);
        engine.set_precision(0);
        assert_eq!(engine.precision(), 1);
    }

    #[test]
    fn earlier_statements_survive_a_later_error() {
        let mut engine = FakeEngine::new();
        assert!(engine.eval("kept = 7\nbroken = ").is_err());
        assert_eq!(
            engine.variables(),
            vec![("kept".to_string(), "7".to_string())]
        );
    }

    #[test]
    fn statements_split_on_semicolons_and_newlines_but_not_in_strings() {
        let mut engine = FakeEngine::new();
        let value = engine.eval("a = 1; b = 2\nc = \"x;y\"\na + b").unwrap();
        assert_eq!(value, "3");
        assert_eq!(engine.variables().len(), 3);
    }

    #[test]
    fn text_values_concatenate_and_mixing_kinds_is_an_error() {
        let mut engine = FakeEngine::new();
        assert_eq!(engine.eval("\"a\" + \"b\"").unwrap(), "ab");
        assert_eq!(
            engine.eval("1 + \"b\"").unwrap_err(),
            "Cannot add Natural and Text."
        );
    }
}
