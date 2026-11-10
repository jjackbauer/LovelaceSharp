//! Tree-walking interpreter: lexical scopes, statements, expressions, and calls.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use lovelace_int::Int;
use lovelace_nat::Nat;

use crate::ast::{BinOp, Expr, FunctionDef, IndexSpec, InterpPart, Stmt, UnOp};
use crate::format;
use crate::parser;
use crate::token::{normalize_source, tokenize};
use crate::value::{
    apply_array_binary, apply_comparison, apply_scalar, exact_i64, factorial_value, int_from_nat,
    negate_value, promote_numeric, Value,
};
use crate::LangError;

/// A lexical environment: a chain of name-to-value bindings.
pub(crate) struct Env {
    pub(crate) values: BTreeMap<String, Value>,
    pub(crate) parent: Option<EnvRef>,
}

/// A shared reference to an environment.
pub(crate) type EnvRef = Rc<RefCell<Env>>;

/// Builds a new environment with an optional parent.
pub(crate) fn new_env(parent: Option<EnvRef>) -> EnvRef {
    Rc::new(RefCell::new(Env { values: BTreeMap::new(), parent }))
}

/// Non-local control flow raised while executing a statement.
pub(crate) enum Flow {
    /// A recoverable language error.
    Error(LangError),
    /// `return` with its value.
    Return(Value),
    /// `break`.
    Break,
    /// `continue`.
    Continue,
}

pub(crate) type FResult<T> = Result<T, Flow>;

impl From<LangError> for Flow {
    fn from(value: LangError) -> Flow {
        Flow::Error(value)
    }
}

/// Builds an `Error` flow from a message.
pub(crate) fn ferr(message: impl Into<String>) -> Flow {
    Flow::Error(LangError::new(message))
}

fn env_get(env: &EnvRef, name: &str) -> Option<Value> {
    let (found, parent) = {
        let current = env.borrow();
        (current.values.get(name).cloned(), current.parent.clone())
    };
    if found.is_some() {
        return found;
    }
    parent.and_then(|parent| env_get(&parent, name))
}

fn find_scope(env: &EnvRef, name: &str) -> Option<EnvRef> {
    let (found, parent) = {
        let current = env.borrow();
        (current.values.contains_key(name), current.parent.clone())
    };
    if found {
        return Some(env.clone());
    }
    parent.and_then(|parent| find_scope(&parent, name))
}

impl crate::Engine {
    /// Evaluates a script and returns its final value.
    ///
    /// # Errors
    ///
    /// Returns [`LangError`] for tokenizer, parser, and runtime failures.
    pub fn eval(&mut self, source: &str) -> Result<Value, LangError> {
        let normalized = normalize_source(source);
        let tokens = tokenize(&normalized).map_err(LangError::new)?;
        let program = parser::parse_program(tokens)?;
        let globals = self.globals.clone();
        let mut last = Value::Void;
        for statement in &program {
            match self.exec_stmt(&globals, statement) {
                Ok(value) => last = value,
                Err(Flow::Return(value)) => return Ok(value),
                Err(Flow::Break) => {
                    return Err(LangError::new("'break' is only valid inside a loop."));
                }
                Err(Flow::Continue) => {
                    return Err(LangError::new("'continue' is only valid inside a loop."));
                }
                Err(Flow::Error(error)) => return Err(error),
            }
        }
        Ok(last)
    }

    fn exec_list(&mut self, env: &EnvRef, statements: &[Stmt]) -> FResult<Value> {
        let mut last = Value::Void;
        for statement in statements {
            last = self.exec_stmt(env, statement)?;
        }
        Ok(last)
    }

    fn exec_stmt(&mut self, env: &EnvRef, statement: &Stmt) -> FResult<Value> {
        match statement {
            Stmt::Expr(expression) => self.eval_expr(env, expression),
            Stmt::Block(statements) => {
                let child = new_env(Some(env.clone()));
                self.exec_list(&child, statements)
            }
            Stmt::If { condition, then_branch, else_branch } => {
                let condition = self.eval_expr(env, condition)?;
                match condition {
                    Value::Boolean(true) => self.exec_stmt(env, then_branch),
                    Value::Boolean(false) => match else_branch {
                        Some(branch) => self.exec_stmt(env, branch),
                        None => Ok(Value::Void),
                    },
                    other => Err(ferr(format!(
                        "if condition must be Boolean, but got '{}'.",
                        other.kind_name()
                    ))),
                }
            }
            Stmt::While { condition, body } => {
                let mut last = Value::Void;
                loop {
                    let condition = self.eval_expr(env, condition)?;
                    match condition {
                        Value::Boolean(true) => {}
                        Value::Boolean(false) => break,
                        other => {
                            return Err(ferr(format!(
                                "while condition must be Boolean, but got '{}'.",
                                other.kind_name()
                            )))
                        }
                    }
                    match self.exec_stmt(env, body) {
                        Ok(value) => last = value,
                        Err(Flow::Break) => break,
                        Err(Flow::Continue) => continue,
                        Err(other) => return Err(other),
                    }
                }
                Ok(last)
            }
            Stmt::For { variable, range, body } => {
                let range = self.eval_expr(env, range)?;
                let Value::Array(array) = &range else {
                    return Err(ferr(format!(
                        "for loop range must be a vector, but got '{}'.",
                        range.kind_name()
                    )));
                };
                if array.rank() != 1 {
                    return Err(ferr(format!(
                        "for loop range must be a vector, but got '{}'.",
                        range.kind_name()
                    )));
                }
                let elements = array.as_slice().to_vec();
                let mut last = Value::Void;
                for element in elements {
                    env.borrow_mut().values.insert(variable.clone(), element);
                    match self.exec_stmt(env, body) {
                        Ok(value) => last = value,
                        Err(Flow::Break) => break,
                        Err(Flow::Continue) => continue,
                        Err(other) => return Err(other),
                    }
                }
                Ok(last)
            }
            Stmt::Return(value) => {
                let value = match value {
                    Some(expression) => self.eval_expr(env, expression)?,
                    None => Value::Void,
                };
                Err(Flow::Return(value))
            }
            Stmt::Break => Err(Flow::Break),
            Stmt::Continue => Err(Flow::Continue),
            Stmt::Func(definition) => {
                self.functions.insert(definition.name.clone(), definition.clone());
                self.revision += 1;
                Ok(Value::Void)
            }
        }
    }

    fn eval_expr(&mut self, env: &EnvRef, expression: &Expr) -> FResult<Value> {
        match expression {
            Expr::Literal(text) => {
                if text.contains('.') || text.contains('(') {
                    lovelace_real::Real::from_str(text)
                        .map(Value::Real)
                        .ok_or_else(|| ferr(format!("Invalid real literal '{text}'.")))
                } else {
                    Nat::from_decimal(text)
                        .map(Value::Natural)
                        .ok_or_else(|| ferr(format!("Invalid natural literal '{text}'.")))
                }
            }
            Expr::Text(text) => Ok(Value::Text(text.clone())),
            Expr::Interpolated(parts) => {
                let mut out = String::new();
                for part in parts {
                    match part {
                        InterpPart::Text(text) => out.push_str(text),
                        InterpPart::Expr(expression) => {
                            let value = self.eval_expr(env, expression)?;
                            out.push_str(&format::format(&value));
                        }
                    }
                }
                Ok(Value::Text(out))
            }
            Expr::Var(name) => env_get(env, name)
                .ok_or_else(|| ferr(format!("Undefined variable '{name}'."))),
            Expr::Assign(name, expression) => {
                let value = self.eval_expr(env, expression)?;
                self.assign(env, name, value.clone());
                Ok(value)
            }
            Expr::Binary(op, left, right) => {
                let left = self.eval_expr(env, left)?;
                let right = self.eval_expr(env, right)?;
                if op.is_comparison() {
                    apply_comparison(*op, &left, &right).map_err(Flow::Error)
                } else if left.is_array_like() || right.is_array_like() {
                    apply_array_binary(*op, &left, &right, self.precision).map_err(Flow::Error)
                } else {
                    apply_scalar(*op, &left, &right, self.precision).map_err(Flow::Error)
                }
            }
            Expr::Unary(op, operand) => {
                let operand = self.eval_expr(env, operand)?;
                match op {
                    UnOp::Plus => Ok(operand),
                    UnOp::Negate => negate_value(&operand).map_err(Flow::Error),
                }
            }
            Expr::Factorial(operand) => {
                let operand = self.eval_expr(env, operand)?;
                factorial_value(&operand).map_err(Flow::Error)
            }
            Expr::Call(name, arguments) => {
                let mut args = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    args.push(self.eval_expr(env, argument)?);
                }
                self.call_function(name, args)
            }
            Expr::Range { start, step, end } => {
                let start = self.eval_expr(env, start)?;
                let end = self.eval_expr(env, end)?;
                let step = match step {
                    Some(step) => Some(self.eval_expr(env, step)?),
                    None => None,
                };
                self.eval_range(start, step, end).map_err(Flow::Error)
            }
            Expr::Index(target, specs) => {
                let target = self.eval_expr(env, target)?;
                let specs = self.eval_specs(env, specs)?;
                index_value(&target, &specs).map_err(Flow::Error)
            }
            Expr::Member(target, member) => {
                let target = self.eval_expr(env, target)?;
                match &target {
                    Value::Record(record) => record.get(member).cloned().ok_or_else(|| {
                        let names: Vec<&str> =
                            record.fields.iter().map(|(name, _)| name.as_str()).collect();
                        ferr(format!(
                            "record '{}' has no member '{}'. Available members: {}.",
                            record.type_name,
                            member,
                            names.join(", ")
                        ))
                    }),
                    other => Err(ferr(format!(
                        "member '{member}' is not available on type '{}': member access requires a record result.",
                        other.kind_name()
                    ))),
                }
            }
            Expr::List(elements) => {
                let mut values = Vec::with_capacity(elements.len());
                for element in elements {
                    values.push(self.eval_expr(env, element)?);
                }
                build_list(values)
            }
        }
    }

    fn assign(&mut self, env: &EnvRef, name: &str, value: Value) {
        let target = find_scope(env, name).unwrap_or_else(|| env.clone());
        let global = Rc::ptr_eq(&target, &self.globals);
        target.borrow_mut().values.insert(name.to_string(), value);
        if global {
            self.revision += 1;
        }
    }

    fn eval_specs(&mut self, env: &EnvRef, specs: &[IndexSpec]) -> FResult<Vec<EvalSpec>> {
        let mut out = Vec::with_capacity(specs.len());
        for spec in specs {
            match spec {
                IndexSpec::Scalar(expression) => {
                    let value = self.eval_expr(env, expression)?;
                    let index = exact_i64(&value).ok_or_else(|| {
                        ferr(format!(
                            "Index must be Natural or Integer, but got '{}'.",
                            value.kind_name()
                        ))
                    })?;
                    out.push(EvalSpec::Scalar(index));
                }
                IndexSpec::Slice { start, stop, step } => {
                    let start = match start {
                        Some(expression) => {
                            let value = self.eval_expr(env, expression)?;
                            Some(exact_i64(&value).ok_or_else(|| {
                                ferr(format!(
                                    "Slice bound must be Natural or Integer, but got '{}'.",
                                    value.kind_name()
                                ))
                            })?)
                        }
                        None => None,
                    };
                    let stop = match stop {
                        Some(expression) => {
                            let value = self.eval_expr(env, expression)?;
                            Some(exact_i64(&value).ok_or_else(|| {
                                ferr(format!(
                                    "Slice bound must be Natural or Integer, but got '{}'.",
                                    value.kind_name()
                                ))
                            })?)
                        }
                        None => None,
                    };
                    let step = match step {
                        Some(expression) => {
                            let value = self.eval_expr(env, expression)?;
                            Some(exact_i64(&value).ok_or_else(|| {
                                ferr(format!(
                                    "Slice step must be Natural or Integer, but got '{}'.",
                                    value.kind_name()
                                ))
                            })?)
                        }
                        None => None,
                    };
                    if let Some(step) = step {
                        if step <= 0 {
                            return Err(ferr("Slice step must be positive."));
                        }
                    }
                    out.push(EvalSpec::Slice { start, stop, step });
                }
            }
        }
        Ok(out)
    }

    fn eval_range(
        &self,
        start: Value,
        step: Option<Value>,
        end: Value,
    ) -> Result<Value, LangError> {
        let natural = step.is_none()
            && matches!(start, Value::Natural(_))
            && matches!(end, Value::Natural(_));
        let start = range_component(&start)?;
        let end = range_component(&end)?;
        let step = match &step {
            Some(value) => range_component(value)?,
            None => Int::from_i64(1),
        };
        if step.is_zero() {
            return Err(LangError::new("Range step must not be zero."));
        }
        let mut current = start;
        let mut elements = Vec::new();
        if step.is_negative() {
            while current.cmp(&end) != std::cmp::Ordering::Less {
                push_range_element(&mut elements, &current, natural)?;
                current = current.add(&step);
            }
        } else {
            while current.cmp(&end) != std::cmp::Ordering::Greater {
                push_range_element(&mut elements, &current, natural)?;
                current = current.add(&step);
            }
        }
        Ok(Value::vector(elements))
    }

    fn call_function(&mut self, name: &str, args: Vec<Value>) -> FResult<Value> {
        if let Some(function) = self.functions.get(name).cloned() {
            return self.call_user_function(&function, args);
        }
        self.call_builtin(name, args)
    }

    fn call_user_function(
        &mut self,
        function: &Rc<FunctionDef>,
        args: Vec<Value>,
    ) -> FResult<Value> {
        if args.len() != function.params.len() {
            return Err(ferr(format!(
                "Function '{}' expects {} argument(s), but got {}.",
                function.name,
                function.params.len(),
                args.len()
            )));
        }
        let frame = new_env(Some(self.globals.clone()));
        {
            let mut scope = frame.borrow_mut();
            for (name, value) in function.params.iter().zip(args) {
                scope.values.insert(name.clone(), value);
            }
        }
        match self.exec_list(&frame, &function.body) {
            Ok(value) => Ok(value),
            Err(Flow::Return(value)) => Ok(value),
            Err(other) => Err(other),
        }
    }
}

fn range_component(value: &Value) -> Result<Int, LangError> {
    match value {
        Value::Natural(nat) => Ok(int_from_nat(nat)),
        Value::Integer(int) => Ok(int.clone()),
        other => Err(LangError::new(format!(
            "Range bounds must be Natural or Integer, but got '{}'.",
            other.kind_name()
        ))),
    }
}

fn push_range_element(elements: &mut Vec<Value>, value: &Int, natural: bool) -> Result<(), LangError> {
    if elements.len() >= 10_000_000 {
        return Err(LangError::new("Range is too large."));
    }
    if natural {
        elements.push(Value::Natural(value.magnitude().clone()));
    } else {
        elements.push(Value::Integer(value.clone()));
    }
    Ok(())
}

/// An evaluated index axis.
enum EvalSpec {
    Scalar(i64),
    Slice { start: Option<i64>, stop: Option<i64>, step: Option<i64> },
}

fn build_list(mut elements: Vec<Value>) -> FResult<Value> {
    let all_containers = !elements.is_empty() && elements.iter().all(Value::is_array_like);
    if all_containers {
        let first_shape = elements[0]
            .as_array()
            .map(|array| array.shape().to_vec())
            .unwrap_or_default();
        let same = elements.iter().all(|element| {
            element
                .as_array()
                .map(|array| array.shape() == first_shape.as_slice())
                .unwrap_or(false)
        });
        if !same {
            return Err(ferr("Ragged nested list literal: every row must have the same shape."));
        }
        let mut shape = vec![elements.len()];
        shape.extend_from_slice(&first_shape);
        let mut data = Vec::new();
        for element in &elements {
            if let Some(array) = element.as_array() {
                data.extend_from_slice(array.as_slice());
            }
        }
        promote_numeric(&mut data);
        return Value::array(shape, data).map_err(Flow::Error);
    }
    promote_numeric(&mut elements);
    Ok(Value::vector(elements))
}

fn flat_offset(shape: &[usize], coords: &[usize]) -> usize {
    let mut offset = 0usize;
    let mut stride = 1usize;
    for axis in (0..shape.len()).rev() {
        offset += coords[axis] * stride;
        stride *= shape[axis];
    }
    offset
}

fn index_value(target: &Value, specs: &[EvalSpec]) -> Result<Value, LangError> {
    let Some(array) = target.as_array() else {
        return Err(LangError::new(format!(
            "Indexing is not supported for type '{}'.",
            target.kind_name()
        )));
    };
    let shape = array.shape().to_vec();
    let rank = shape.len();

    if specs.iter().all(|spec| matches!(spec, EvalSpec::Scalar(_))) {
        let indices: Vec<i64> = specs
            .iter()
            .map(|spec| match spec {
                EvalSpec::Scalar(index) => *index,
                EvalSpec::Slice { .. } => 0,
            })
            .collect();
        if rank == 1 {
            if indices.len() != 1 {
                return Err(LangError::new(format!(
                    "Vector indexing expects exactly 1 index, but got {}.",
                    indices.len()
                )));
            }
            let index = indices[0];
            if index < 0 || index as usize >= array.numel() {
                return Err(LangError::new(format!(
                    "Index {index} is out of range for vector of length {}.",
                    array.numel()
                )));
            }
            return Ok(array.as_slice()[index as usize].clone());
        }
        if indices.len() > rank {
            return Err(LangError::new(format!(
                "A rank-{rank} array cannot be indexed with {} indices.",
                indices.len()
            )));
        }
        let mut coords = Vec::with_capacity(indices.len());
        for (axis, &index) in indices.iter().enumerate() {
            if index < 0 || index as usize >= shape[axis] {
                return Err(LangError::new(format!(
                    "Index {index} is out of range for array with shape [{}].",
                    shape
                        .iter()
                        .map(|dim| dim.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )));
            }
            coords.push(index as usize);
        }
        if indices.len() == rank {
            return Ok(array.as_slice()[flat_offset(&shape, &coords)].clone());
        }
        let mut rest = Vec::with_capacity(shape.len() - coords.len());
        rest.extend_from_slice(&shape[coords.len()..]);
        let count: usize = rest.iter().product();
        let data = array.as_slice()[flat_offset(&shape, &coords)..flat_offset(&shape, &coords) + count]
            .to_vec();
        return Value::array(rest, data);
    }

    if specs.len() != rank {
        return Err(LangError::new(format!(
            "Slicing expects {rank} axis specifiers, but got {}.",
            specs.len()
        )));
    }
    let mut selected: Vec<Vec<usize>> = Vec::with_capacity(rank);
    let mut out_shape: Vec<usize> = Vec::new();
    for (axis, spec) in specs.iter().enumerate() {
        let dim = shape[axis];
        match spec {
            EvalSpec::Scalar(index) => {
                if *index < 0 || *index as usize >= dim {
                    return Err(LangError::new(format!(
                        "Index {index} is out of range for axis of length {dim}."
                    )));
                }
                selected.push(vec![*index as usize]);
            }
            EvalSpec::Slice { start, stop, step } => {
                let step = step.unwrap_or(1);
                if step <= 0 {
                    return Err(LangError::new("Slice step must be positive."));
                }
                let start = normalize_bound(*start, dim, 0);
                let stop = normalize_bound(*stop, dim, dim as i64);
                let mut axis_indices = Vec::new();
                let mut index = start;
                while index < stop {
                    axis_indices.push(index as usize);
                    index += step;
                }
                out_shape.push(axis_indices.len());
                selected.push(axis_indices);
            }
        }
    }
    if out_shape.is_empty() {
        return Err(LangError::new("A slice must select at least one axis."));
    }
    let out_numel: usize = out_shape.iter().product();
    let mut data = Vec::with_capacity(out_numel);
    let mut coords = vec![0usize; out_shape.len()];
    for flat in 0..out_numel {
        let mut remainder = flat;
        for axis in (0..out_shape.len()).rev() {
            coords[axis] = if out_shape[axis] == 0 { 0 } else { remainder % out_shape[axis] };
            if out_shape[axis] != 0 {
                remainder /= out_shape[axis];
            }
        }
        let mut source_coords = Vec::with_capacity(rank);
        let mut slice_axis = 0usize;
        for spec in specs.iter() {
            match spec {
                EvalSpec::Scalar(index) => source_coords.push(*index as usize),
                EvalSpec::Slice { .. } => {
                    source_coords.push(selected[axis_for_spec(specs, slice_axis)][coords[slice_axis]]);
                    slice_axis += 1;
                }
            }
        }
        data.push(array.as_slice()[flat_offset(&shape, &source_coords)].clone());
    }
    Value::array(out_shape, data)
}

fn axis_for_spec(specs: &[EvalSpec], slice_index: usize) -> usize {
    let mut seen = 0usize;
    for (axis, spec) in specs.iter().enumerate() {
        if matches!(spec, EvalSpec::Slice { .. }) {
            if seen == slice_index {
                return axis;
            }
            seen += 1;
        }
    }
    specs.len()
}

fn normalize_bound(value: Option<i64>, dim: usize, default: i64) -> i64 {
    let dim_i = dim as i64;
    let mut result = value.unwrap_or(default);
    if result < 0 {
        result += dim_i;
    }
    result.clamp(0, dim_i)
}
