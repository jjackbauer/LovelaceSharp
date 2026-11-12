//! MathIR: the typed, validated computational DAG below the symbolic layer.
//!
//! This is the Rust counterpart of Lovelace.MathIR: a compiled program is a constant pool, a
//! parameter list and a list of typed instructions (IrNode) whose operands are indices into
//! that same node list (children precede parents). Types are inferred forward (IrTyping),
//! validation enforces arity, operand ranges, ordered-comparison domains and Bool Select
//! guards, and a compiled kernel executes the DAG with lazy Select at the ambient Real
//! precision.
//!
//! The home is lovelace-lang because this is where the lowering already lives
//! (builtins::ir_text), where the symbolic expression type (lovelace_symbolics::Expr) and the
//! numeric tier (crate::numeric::Num) live, and where the C# rows' analogue sits: the C#
//! Lovelace.MathIR library depends only on Lovelace.Symbolics and is exercised from
//! Lovelace.Symbolics.Tests, not from the runner. lovelace-run is the CLI/envelope crate and
//! must keep publishing exactly what it publishes today.
//!
//! The surface is additive: nothing in builtins or the runner consults it, so the published
//! lower/compile text and every existing behaviour are unchanged.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;

use lovelace_int::Int;
use lovelace_nat::Nat;
use lovelace_real::Real;
use lovelace_symbolics::{Expr as Sym, Rat, RelOp, canonical_parse};

use crate::LangError;
use crate::complex::Complex;
use crate::complex_math;
use crate::numeric::Num;

/// The MathIR text format version written in the "#!mathir" header.
pub const FORMAT_VERSION: u32 = 2;

/// The typed instruction kinds of a MathIR program.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IrOpKind {
    /// A constant-pool entry.
    Constant,
    /// A kernel parameter.
    Parameter,
    /// Addition.
    Add,
    /// Subtraction.
    Sub,
    /// Multiplication.
    Mul,
    /// Division.
    Div,
    /// Unary negation.
    Negate,
    /// The reciprocal 1/x.
    Reciprocal,
    /// A power with an exact integer exponent.
    PowInt,
    /// A power with a general exponent.
    Pow,
    /// The principal square root.
    Sqrt,
    /// The exponential.
    Exp,
    /// The natural logarithm.
    Log,
    /// Sine.
    Sin,
    /// Cosine.
    Cos,
    /// Tangent.
    Tan,
    /// Arcsine.
    Asin,
    /// Arccosine.
    Acos,
    /// Arctangent.
    Atan,
    /// Hyperbolic sine.
    Sinh,
    /// Hyperbolic cosine.
    Cosh,
    /// Hyperbolic tangent.
    Tanh,
    /// Absolute value.
    Abs,
    /// Sign.
    Sign,
    /// Floor.
    Floor,
    /// Ceiling.
    Ceil,
    /// Minimum.
    Min,
    /// Maximum.
    Max,
    /// Equality.
    Eq,
    /// Inequality.
    Ne,
    /// Less than.
    Lt,
    /// Less than or equal.
    Le,
    /// Greater than.
    Gt,
    /// Greater than or equal.
    Ge,
    /// A lazy conditional: guard, taken value, otherwise value.
    Select,
}

impl IrOpKind {
    /// The instruction name as it appears in serialized MathIR.
    pub fn name(self) -> &'static str {
        match self {
            IrOpKind::Constant => "Constant",
            IrOpKind::Parameter => "Parameter",
            IrOpKind::Add => "Add",
            IrOpKind::Sub => "Sub",
            IrOpKind::Mul => "Mul",
            IrOpKind::Div => "Div",
            IrOpKind::Negate => "Negate",
            IrOpKind::Reciprocal => "Reciprocal",
            IrOpKind::PowInt => "PowInt",
            IrOpKind::Pow => "Pow",
            IrOpKind::Sqrt => "Sqrt",
            IrOpKind::Exp => "Exp",
            IrOpKind::Log => "Log",
            IrOpKind::Sin => "Sin",
            IrOpKind::Cos => "Cos",
            IrOpKind::Tan => "Tan",
            IrOpKind::Asin => "Asin",
            IrOpKind::Acos => "Acos",
            IrOpKind::Atan => "Atan",
            IrOpKind::Sinh => "Sinh",
            IrOpKind::Cosh => "Cosh",
            IrOpKind::Tanh => "Tanh",
            IrOpKind::Abs => "Abs",
            IrOpKind::Sign => "Sign",
            IrOpKind::Floor => "Floor",
            IrOpKind::Ceil => "Ceil",
            IrOpKind::Min => "Min",
            IrOpKind::Max => "Max",
            IrOpKind::Eq => "Eq",
            IrOpKind::Ne => "Ne",
            IrOpKind::Lt => "Lt",
            IrOpKind::Le => "Le",
            IrOpKind::Gt => "Gt",
            IrOpKind::Ge => "Ge",
            IrOpKind::Select => "Select",
        }
    }

    /// The exact operand count validation requires (the C# IrTyping.Validate arity table).
    pub fn arity(self) -> usize {
        match self {
            IrOpKind::Constant
            | IrOpKind::Parameter
            | IrOpKind::Negate
            | IrOpKind::Reciprocal
            | IrOpKind::Sqrt
            | IrOpKind::Exp
            | IrOpKind::Log
            | IrOpKind::Sin
            | IrOpKind::Cos
            | IrOpKind::Tan
            | IrOpKind::Asin
            | IrOpKind::Acos
            | IrOpKind::Atan
            | IrOpKind::Sinh
            | IrOpKind::Cosh
            | IrOpKind::Tanh
            | IrOpKind::Abs
            | IrOpKind::Sign
            | IrOpKind::Floor
            | IrOpKind::Ceil => 1,
            IrOpKind::Add
            | IrOpKind::Sub
            | IrOpKind::Mul
            | IrOpKind::Div
            | IrOpKind::Pow
            | IrOpKind::PowInt
            | IrOpKind::Min
            | IrOpKind::Max
            | IrOpKind::Eq
            | IrOpKind::Ne
            | IrOpKind::Lt
            | IrOpKind::Le
            | IrOpKind::Gt
            | IrOpKind::Ge => 2,
            IrOpKind::Select => 3,
        }
    }

    fn from_name(name: &str) -> Option<IrOpKind> {
        [
            IrOpKind::Constant,
            IrOpKind::Parameter,
            IrOpKind::Add,
            IrOpKind::Sub,
            IrOpKind::Mul,
            IrOpKind::Div,
            IrOpKind::Negate,
            IrOpKind::Reciprocal,
            IrOpKind::PowInt,
            IrOpKind::Pow,
            IrOpKind::Sqrt,
            IrOpKind::Exp,
            IrOpKind::Log,
            IrOpKind::Sin,
            IrOpKind::Cos,
            IrOpKind::Tan,
            IrOpKind::Asin,
            IrOpKind::Acos,
            IrOpKind::Atan,
            IrOpKind::Sinh,
            IrOpKind::Cosh,
            IrOpKind::Tanh,
            IrOpKind::Abs,
            IrOpKind::Sign,
            IrOpKind::Floor,
            IrOpKind::Ceil,
            IrOpKind::Min,
            IrOpKind::Max,
            IrOpKind::Eq,
            IrOpKind::Ne,
            IrOpKind::Lt,
            IrOpKind::Le,
            IrOpKind::Gt,
            IrOpKind::Ge,
            IrOpKind::Select,
        ]
        .into_iter()
        .find(|kind| kind.name() == name)
    }
}

impl fmt::Display for IrOpKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// One typed instruction: an operation, its operand node indices, and a small auxiliary tag.
///
/// The aux field is 1 on a Constant node that names a named constant (Pi/E/I) and 0 everywhere
/// else, exactly as the reference stores it. Constant/Parameter operands index the constant
/// pool / parameter list; every other operand indexes the node list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrNode {
    op: IrOpKind,
    operands: Vec<usize>,
    aux: i32,
}

impl IrNode {
    /// Builds an instruction.
    pub fn new(op: IrOpKind, operands: Vec<usize>, aux: i32) -> IrNode {
        IrNode { op, operands, aux }
    }

    /// The operation.
    pub fn op(&self) -> IrOpKind {
        self.op
    }

    /// The operand indices.
    pub fn operands(&self) -> &[usize] {
        &self.operands
    }

    /// The auxiliary tag.
    pub fn aux(&self) -> i32 {
        self.aux
    }

    /// The number of operands present.
    pub fn arity(&self) -> usize {
        self.operands.len()
    }
}

/// A compiled MathIR program: a constant pool, a parameter list and a typed instruction list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IrProgram {
    constants: Vec<String>,
    parameters: Vec<String>,
    nodes: Vec<IrNode>,
}

impl IrProgram {
    /// The text format version this program serializes with.
    pub const FORMAT_VERSION: u32 = FORMAT_VERSION;

    /// An empty program.
    pub fn new() -> IrProgram {
        IrProgram::default()
    }

    /// The constant pool, in insertion order.
    pub fn constants(&self) -> &[String] {
        &self.constants
    }

    /// The kernel parameters, in declaration order.
    pub fn parameters(&self) -> &[String] {
        &self.parameters
    }

    /// The instructions, in dependency order (children precede parents).
    pub fn nodes(&self) -> &[IrNode] {
        &self.nodes
    }

    /// The instruction at the given index, when it exists.
    pub fn node(&self, index: usize) -> Option<&IrNode> {
        self.nodes.get(index)
    }

    /// The pool entry at the given index, when it exists.
    pub fn constant(&self, index: usize) -> Option<&str> {
        self.constants.get(index).map(String::as_str)
    }

    /// The parameter name at the given index, when it exists.
    pub fn parameter(&self, index: usize) -> Option<&str> {
        self.parameters.get(index).map(String::as_str)
    }

    /// The index of the result instruction (the last node), when the program is not empty.
    pub fn root(&self) -> Option<usize> {
        self.nodes.len().checked_sub(1)
    }

    /// Serializes the program in the versioned line format (#!mathir 2, param, const, one
    /// instruction per line), byte-identical to the reference's IrProgram.Serialize.
    pub fn serialize(&self) -> String {
        let mut out = String::new();
        out.push_str("#!mathir ");
        out.push_str(&FORMAT_VERSION.to_string());
        out.push('\n');
        for parameter in &self.parameters {
            out.push_str("param ");
            out.push_str(parameter);
            out.push('\n');
        }
        for constant in &self.constants {
            out.push_str("const ");
            out.push_str(constant);
            out.push('\n');
        }
        for node in &self.nodes {
            out.push_str(node.op().name());
            out.push(' ');
            out.push_str(&node.aux().to_string());
            out.push(' ');
            let operands: Vec<String> = node.operands().iter().map(usize::to_string).collect();
            out.push_str(&operands.join(","));
            out.push('\n');
        }
        out
    }

    /// Deserializes and validates MathIR text.
    ///
    /// The format-version gate and the structural/type validation carry the reference's own
    /// sentences; a text that validates here round-trips through IrProgram::serialize.
    pub fn deserialize(text: &str) -> Result<IrProgram, LangError> {
        let mut program = IrProgram::new();
        for raw in text.split('\n') {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(version) = line.strip_prefix("#!mathir ") {
                if version != FORMAT_VERSION.to_string() {
                    return Err(LangError::coded(
                        format!(
                            "Unsupported MathIR format version '{version}' (expected {FORMAT_VERSION})."
                        ),
                        0,
                        "InvalidInput",
                        "ParseError",
                    ));
                }
                continue;
            }
            if line.starts_with('#') {
                continue;
            }
            if let Some(rest) = line.strip_prefix("param ") {
                program.parameters.push(rest.to_string());
                continue;
            }
            if let Some(rest) = line.strip_prefix("const ") {
                program.constants.push(rest.to_string());
                continue;
            }
            let mut fields = line.splitn(3, ' ');
            let op_text = fields.next().unwrap_or_default();
            let aux_text = fields.next().ok_or_else(|| malformed_line(line))?;
            let operands_text = fields.next().unwrap_or_default();
            let op = IrOpKind::from_name(op_text).ok_or_else(|| malformed_line(line))?;
            let aux: i32 = aux_text.parse().map_err(|_| malformed_line(line))?;
            let mut operands: Vec<usize> = Vec::new();
            if !operands_text.is_empty() {
                for piece in operands_text.split(',') {
                    operands.push(piece.parse::<usize>().map_err(|_| malformed_line(line))?);
                }
            }
            program.nodes.push(IrNode::new(op, operands, aux));
        }
        IrTyping::validate(&program)?;
        Ok(program)
    }

    /// Lowers a symbolic expression over the named parameters into a MathIR program.
    ///
    /// This is the reference's Lowering.Lower: parameters are indexed by name, repeated
    /// subexpressions share their node (structural CSE), x^-1 becomes Reciprocal, x^(1/2)
    /// becomes Sqrt, an integer exponent becomes PowInt with a fresh exponent constant node,
    /// and sums/products are left-folded into binary Add/Mul nodes. A symbol that is not a
    /// parameter, an unsupported function, an order term, a relation and a malformed piecewise
    /// guard all refuse with the reference's sentences.
    pub fn lower(expr: &Sym, parameters: &[String]) -> Result<IrProgram, LangError> {
        IrProgram::lower_with(expr, parameters, false)
    }

    /// Lowers with the LANGUAGE SURFACE's operand order: Add/Multiply children are emitted in
    /// the reference's canonical term order, not in this crate's display-ordered storage order.
    /// The reference stores an Add's terms canonically ("1 + x") and prints them display-ordered
    /// ("x + 1"); Rust Sym stores display order, so the language's lower/compile output needs the
    /// explicit canonical sort to stay byte-identical to the reference. Deserialize validation
    /// is applied by the caller (compile_ordered), exactly as Compilation.Compile does.
    pub fn lower_ordered(expr: &Sym, parameters: &[String]) -> Result<IrProgram, LangError> {
        IrProgram::lower_with(expr, parameters, true)
    }

    fn lower_with(
        expr: &Sym,
        parameters: &[String],
        ordered: bool,
    ) -> Result<IrProgram, LangError> {
        let mut program = IrProgram::new();
        program.parameters = parameters.to_vec();
        let mut cache: Vec<(Sym, usize)> = Vec::new();
        emit(expr, parameters, &mut program, &mut cache, ordered)?;
        Ok(program)
    }
}

/// Forward type inference and structural validation (the reference's IrTyping).
pub struct IrTyping;

impl IrTyping {
    /// Infers each instruction's scalar type, forward over the node list.
    ///
    /// None is the reference's null: a parameter is unconstrained, and an unknown
    /// classification (never reached by a validated program) is empty too.
    pub fn infer(program: &IrProgram) -> Vec<Option<IrType>> {
        let nodes = program.nodes();
        let mut types: Vec<Option<IrType>> = vec![None; nodes.len()];
        for (index, node) in nodes.iter().enumerate() {
            let left = operand_type(&types, node, 0);
            let right = operand_type(&types, node, 1);
            let inferred = match node.op() {
                IrOpKind::Constant => Some(classify_constant(pool_text(program, node))),
                IrOpKind::Parameter => None,
                IrOpKind::Add | IrOpKind::Sub | IrOpKind::Mul | IrOpKind::Min | IrOpKind::Max => {
                    promote(left, right)
                }
                IrOpKind::Negate => left,
                IrOpKind::PowInt => left,
                IrOpKind::Div
                | IrOpKind::Reciprocal
                | IrOpKind::Pow
                | IrOpKind::Sqrt
                | IrOpKind::Exp
                | IrOpKind::Log
                | IrOpKind::Sin
                | IrOpKind::Cos
                | IrOpKind::Tan
                | IrOpKind::Asin
                | IrOpKind::Acos
                | IrOpKind::Atan
                | IrOpKind::Sinh
                | IrOpKind::Cosh
                | IrOpKind::Tanh => {
                    // The reference's rule takes ONE input for every op but Pow (Typing.cs:
                    // Transcendental(T(0)) vs Transcendental(T(0), T(1))). The unary form must not
                    // be written transcendental(left, None): the second parameter is an INPUT
                    // there, so a synthetic None answers None for every typed operand (exp(2)
                    // would not be Real). Div is this surface's stand-in for the reference
                    // quotient Multiply(x, Pow(y, -1)), whose promote reads both sides, so Div
                    // passes both.
                    if matches!(node.op(), IrOpKind::Div | IrOpKind::Pow) {
                        transcendental(left, right)
                    } else {
                        transcendental_unary(left)
                    }
                }
                IrOpKind::Abs => Some(IrType::Real),
                IrOpKind::Sign => {
                    if left == Some(IrType::Complex) {
                        Some(IrType::Complex)
                    } else {
                        Some(IrType::Integer)
                    }
                }
                IrOpKind::Floor | IrOpKind::Ceil => {
                    if left == Some(IrType::Complex) {
                        None
                    } else {
                        Some(IrType::Integer)
                    }
                }
                IrOpKind::Eq
                | IrOpKind::Ne
                | IrOpKind::Lt
                | IrOpKind::Le
                | IrOpKind::Gt
                | IrOpKind::Ge => Some(IrType::Bool),
                IrOpKind::Select => promote(right, operand_type(&types, node, 2)),
            };
            types[index] = inferred;
        }
        types
    }

    /// Validates a program, returning the reference's first refusal.
    ///
    /// The structural pass runs before type inference, so an out-of-range operand is reported
    /// as an operand-range failure and never reached as a type lookup.
    pub fn validate(program: &IrProgram) -> Result<(), LangError> {
        let nodes = program.nodes();
        for (index, node) in nodes.iter().enumerate() {
            let expected = node.op().arity();
            if node.arity() != expected {
                return Err(arity_error(index, node));
            }
            for &operand in node.operands() {
                if operand >= nodes.len() {
                    return Err(operand_error(index, node, operand));
                }
            }
            if node.op() == IrOpKind::Constant {
                let pool = node.operands()[0];
                if pool >= program.constants().len() {
                    return Err(pool_error(index, pool));
                }
            }
            if node.op() == IrOpKind::Parameter {
                let parameter = node.operands()[0];
                if parameter >= program.parameters().len() {
                    return Err(parameter_error(index, parameter));
                }
            }
        }
        let types = IrTyping::infer(program);
        for (index, node) in nodes.iter().enumerate() {
            if matches!(
                node.op(),
                IrOpKind::Lt
                    | IrOpKind::Le
                    | IrOpKind::Gt
                    | IrOpKind::Ge
                    | IrOpKind::Min
                    | IrOpKind::Max
            ) {
                for &operand in node.operands() {
                    if types.get(operand).copied().flatten() == Some(IrType::Complex) {
                        return Err(LangError::plain(format!(
                            "IR validation: ordered comparison {} requires real operands, node {index} operand {operand} is Complex.",
                            node.op()
                        )));
                    }
                }
            }
            if node.op() == IrOpKind::Select {
                let guard = node.operands()[0];
                let guard_type = types.get(guard).copied().flatten();
                if guard_type != Some(IrType::Bool) {
                    let observed = guard_type
                        .map(|kind| kind.name().to_string())
                        .unwrap_or_else(|| "unconstrained".to_string());
                    return Err(LangError::plain(format!(
                        "IR validation: Select node {index} requires a Bool guard, got {observed}."
                    )));
                }
            }
        }
        Ok(())
    }
}

/// The scalar types of a MathIR instruction (the ordered types are Integer/Rational/Real).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IrType {
    /// An exact integer.
    Integer,
    /// An exact rational.
    Rational,
    /// A fixed-point real.
    Real,
    /// A complex value.
    Complex,
    /// A boolean (a comparison result, or a Select guard).
    Bool,
}

impl IrType {
    /// The type name as the reference renders it.
    pub fn name(self) -> &'static str {
        match self {
            IrType::Integer => "Integer",
            IrType::Rational => "Rational",
            IrType::Real => "Real",
            IrType::Complex => "Complex",
            IrType::Bool => "Bool",
        }
    }
}

impl fmt::Display for IrType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A compilation outcome: the executable program plus its parameter contract
/// (the reference's CompilationResult).
#[derive(Clone, Debug)]
pub struct CompilationResult {
    program: IrProgram,
    parameters: Vec<String>,
}

impl CompilationResult {
    /// The compiled program.
    pub fn program(&self) -> &IrProgram {
        &self.program
    }

    /// The parameters, in declaration order.
    pub fn parameters(&self) -> &[String] {
        &self.parameters
    }

    /// The serialized program (the reference's CompilationResult.IrText).
    pub fn ir_text(&self) -> String {
        self.program.serialize()
    }

    /// The inferred type of the result instruction, when it is not unconstrained.
    ///
    /// This is the field the language surface publishes as compile_full(...).result_type.
    pub fn result_type(&self) -> Option<IrType> {
        IrTyping::infer(&self.program).last().copied().flatten()
    }
}

/// Compiles a symbolic expression into a validated program over the named parameters
/// (the reference's Compilation.Compile).
pub fn compile(expr: &Sym, parameters: &[String]) -> Result<CompilationResult, LangError> {
    if parameters.is_empty() {
        return Err(LangError::coded(
            "Compilation requires at least one parameter. (Parameter 'parameters')",
            0,
            "InvalidArgument",
            "TypeMismatch",
        ));
    }
    let program = IrProgram::lower(expr, parameters)?;
    IrTyping::validate(&program)?;
    Ok(CompilationResult {
        program,
        parameters: parameters.to_vec(),
    })
}

/// Compiles with the LANGUAGE SURFACE's operand order (see `IrProgram::lower_ordered`), applying
/// the same zero-parameter and IrTyping.Validate refusals as compile. The reader and the
/// compiler must agree on operand order: the reference's IR text is what evalir reads back.
pub fn compile_ordered(expr: &Sym, parameters: &[String]) -> Result<CompilationResult, LangError> {
    if parameters.is_empty() {
        return Err(LangError::coded(
            "Compilation requires at least one parameter. (Parameter 'parameters')",
            0,
            "InvalidArgument",
            "TypeMismatch",
        ));
    }
    let program = IrProgram::lower_ordered(expr, parameters)?;
    IrTyping::validate(&program)?;
    Ok(CompilationResult {
        program,
        parameters: parameters.to_vec(),
    })
}

/// Compiles a symbolic expression directly into an executable kernel
/// (the reference's Compilation.CompileKernel; the reference does not validate here either).
pub fn compile_kernel(expr: &Sym, parameters: &[String]) -> Result<CompiledKernel, LangError> {
    let program = IrProgram::lower(expr, parameters)?;
    Ok(CompiledKernel {
        program,
        parameters: parameters.to_vec(),
    })
}

/// Wraps an existing compilation as a kernel (the reference's Compilation.ToKernel).
pub fn to_kernel(result: &CompilationResult) -> CompiledKernel {
    CompiledKernel {
        program: result.program.clone(),
        parameters: result.parameters.clone(),
    }
}

/// An executable kernel over a compiled program: scalar positional/named evaluation and
/// column-wise batch evaluation with lazy Select (the reference's CompiledKernel).
#[derive(Clone, Debug)]
pub struct CompiledKernel {
    program: IrProgram,
    parameters: Vec<String>,
}

impl CompiledKernel {
    /// The parameters, in declaration order.
    pub fn parameters(&self) -> &[String] {
        &self.parameters
    }

    /// The compiled program.
    pub fn program(&self) -> &IrProgram {
        &self.program
    }

    /// The serialized program.
    pub fn ir_text(&self) -> String {
        self.program.serialize()
    }

    /// Scalar evaluation with positional arguments (arity-checked).
    pub fn evaluate(&self, args: &[Num]) -> Result<Num, LangError> {
        if args.len() != self.parameters.len() {
            return Err(LangError::coded(
                format!(
                    "Kernel expects {} argument(s), got {}.",
                    self.parameters.len(),
                    args.len()
                ),
                0,
                "InvalidArgument",
                "TypeMismatch",
            ));
        }
        let mut bindings: HashMap<String, Num> = HashMap::with_capacity(self.parameters.len());
        for (name, value) in self.parameters.iter().zip(args) {
            bindings.insert(name.clone(), value.clone());
        }
        IrEvaluator::evaluate_at(&self.program, &bindings, ambient_places())
    }

    /// Scalar evaluation with named arguments.
    pub fn evaluate_named(&self, args: &HashMap<String, Num>) -> Result<Num, LangError> {
        IrEvaluator::evaluate_at(&self.program, args, ambient_places())
    }

    /// Batch evaluation: one lane per row of the provided columns.
    ///
    /// Every column must have the same length, and every parameter must have a column; the
    /// refusals are the reference's own sentences.
    pub fn evaluate_batch(
        &self,
        columns: &HashMap<String, Vec<Num>>,
    ) -> Result<Vec<Num>, LangError> {
        if columns.is_empty() {
            return Ok(Vec::new());
        }
        let lanes = columns.values().map(Vec::len).max().unwrap_or(0);
        for (name, column) in columns {
            if column.len() != lanes {
                return Err(LangError::coded(
                    format!(
                        "Column '{name}' has {} lanes; expected {lanes}.",
                        column.len()
                    ),
                    0,
                    "InvalidArgument",
                    "TypeMismatch",
                ));
            }
        }
        for parameter in &self.parameters {
            if !columns.contains_key(parameter) {
                return Err(LangError::coded(
                    format!("Missing column for parameter '{parameter}'."),
                    0,
                    "InvalidArgument",
                    "TypeMismatch",
                ));
            }
        }
        IrEvaluator::evaluate_batch_at(&self.program, columns, ambient_places())
    }
}

/// The MathIR interpreter: a demand-driven traversal of the typed DAG whose Select is lazy.
pub struct IrEvaluator;

impl IrEvaluator {
    /// Evaluates a program at the ambient Real computation precision.
    pub fn evaluate(
        program: &IrProgram,
        bindings: &HashMap<String, Num>,
    ) -> Result<Num, LangError> {
        IrEvaluator::evaluate_at(program, bindings, ambient_places())
    }

    /// Evaluates a program at the given number of fractional digits.
    pub fn evaluate_at(
        program: &IrProgram,
        bindings: &HashMap<String, Num>,
        places: usize,
    ) -> Result<Num, LangError> {
        let nodes = program.nodes();
        if nodes.is_empty() {
            return Err(LangError::plain("Empty IR program."));
        }
        let constants = parse_constants(program, places)?;
        let mut memo: Vec<Option<Num>> = vec![None; nodes.len()];
        let root = nodes.len() - 1;
        let mut work: Vec<usize> = vec![root];
        let mut pushes: u64 = 1;
        while let Some(&index) = work.last() {
            pushes += 1;
            if pushes > nodes.len() as u64 * 64 + 64 {
                return Err(LangError::plain(format!(
                    "IR evaluator cycle near node {index} (op {}).",
                    nodes[index].op()
                )));
            }
            if memo[index].is_some() {
                work.pop();
                continue;
            }
            let node = &nodes[index];
            check_node(program, index, node)?;
            if matches!(node.op(), IrOpKind::Constant | IrOpKind::Parameter) {
                memo[index] = Some(constant_or_parameter(program, node, &constants, bindings)?);
                work.pop();
                continue;
            }
            if node.op() == IrOpKind::Select {
                let guard = node.operands()[0];
                if memo[guard].is_none() {
                    work.push(guard);
                    continue;
                }
                let condition = memo[guard].clone().expect("guard is ready");
                let taken = if is_zero(&condition) {
                    node.operands()[2]
                } else {
                    node.operands()[1]
                };
                if memo[taken].is_none() {
                    work.push(taken);
                    continue;
                }
                memo[index] = memo[taken].clone();
                work.pop();
                continue;
            }
            let mut ready = true;
            for &operand in node.operands() {
                if memo[operand].is_none() {
                    work.push(operand);
                    ready = false;
                }
            }
            if !ready {
                continue;
            }
            let args: Vec<Num> = node
                .operands()
                .iter()
                .map(|&operand| memo[operand].clone().expect("operand is ready"))
                .collect();
            memo[index] = Some(apply(node.op(), &args, places)?);
            work.pop();
        }
        memo[root]
            .take()
            .ok_or_else(|| LangError::plain("Empty IR program."))
    }

    /// Batch-evaluates a program at the ambient Real computation precision.
    pub fn evaluate_batch(
        program: &IrProgram,
        columns: &HashMap<String, Vec<Num>>,
    ) -> Result<Vec<Num>, LangError> {
        IrEvaluator::evaluate_batch_at(program, columns, ambient_places())
    }

    /// Batch-evaluates a program at the given number of fractional digits.
    ///
    /// One traversal carries a lane vector per node. Select demands only the branches a lane
    /// set can take: when every lane takes the same branch, only that branch is evaluated; a
    /// mixed guard evaluates both.
    pub fn evaluate_batch_at(
        program: &IrProgram,
        columns: &HashMap<String, Vec<Num>>,
        places: usize,
    ) -> Result<Vec<Num>, LangError> {
        let lanes = columns.values().map(Vec::len).max().unwrap_or(0);
        if lanes == 0 {
            return Ok(Vec::new());
        }
        let nodes = program.nodes();
        if nodes.is_empty() {
            return Err(LangError::plain("Empty IR program."));
        }
        let constants = parse_constants(program, places)?;
        let mut memo: Vec<Option<Vec<Num>>> = vec![None; nodes.len()];
        let root = nodes.len() - 1;
        let mut work: Vec<usize> = vec![root];
        let mut pushes: u64 = 1;
        while let Some(&index) = work.last() {
            pushes += 1;
            if pushes > nodes.len() as u64 * 64 + 64 {
                return Err(LangError::plain(format!(
                    "IR evaluator cycle near node {index}."
                )));
            }
            if memo[index].is_some() {
                work.pop();
                continue;
            }
            let node = &nodes[index];
            check_node(program, index, node)?;
            if matches!(node.op(), IrOpKind::Constant | IrOpKind::Parameter) {
                memo[index] = Some(constant_or_parameter_vector(
                    program, node, &constants, columns, lanes,
                )?);
                work.pop();
                continue;
            }
            if node.op() == IrOpKind::Select {
                let guard = node.operands()[0];
                if memo[guard].is_none() {
                    work.push(guard);
                    continue;
                }
                let condition = memo[guard].clone().expect("guard is ready");
                let mut any_left = false;
                let mut any_right = false;
                for lane in 0..lanes {
                    if is_zero(&condition[lane]) {
                        any_right = true;
                    } else {
                        any_left = true;
                    }
                }
                if any_left && memo[node.operands()[1]].is_none() {
                    work.push(node.operands()[1]);
                    continue;
                }
                if any_right && memo[node.operands()[2]].is_none() {
                    work.push(node.operands()[2]);
                    continue;
                }
                let left = memo[node.operands()[1]].clone();
                let right = memo[node.operands()[2]].clone();
                let mut result: Vec<Num> = Vec::with_capacity(lanes);
                for lane in 0..lanes {
                    let taken = if is_zero(&condition[lane]) {
                        right.as_ref().map(|values| values[lane].clone())
                    } else {
                        left.as_ref().map(|values| values[lane].clone())
                    };
                    result.push(taken.unwrap_or_else(|| Num::Exact(Rat::zero())));
                }
                memo[index] = Some(result);
                work.pop();
                continue;
            }
            let mut ready = true;
            for &operand in node.operands() {
                if memo[operand].is_none() {
                    work.push(operand);
                    ready = false;
                }
            }
            if !ready {
                continue;
            }
            let operands: Vec<Vec<Num>> = node
                .operands()
                .iter()
                .map(|&operand| memo[operand].clone().expect("operand is ready"))
                .collect();
            let mut result: Vec<Num> = Vec::with_capacity(lanes);
            for lane in 0..lanes {
                let args: Vec<Num> = operands.iter().map(|values| values[lane].clone()).collect();
                result.push(apply(node.op(), &args, places)?);
            }
            memo[index] = Some(result);
            work.pop();
        }
        memo[root]
            .take()
            .ok_or_else(|| LangError::plain("Empty IR program."))
    }
}

// ---------------------------------------------------------------------------
// Lowering (the reference's Lowering)
// ---------------------------------------------------------------------------

fn emit(
    expr: &Sym,
    parameters: &[String],
    program: &mut IrProgram,
    cache: &mut Vec<(Sym, usize)>,
    ordered: bool,
) -> Result<usize, LangError> {
    if let Some((_, index)) = cache.iter().find(|(key, _)| key == expr) {
        return Ok(*index);
    }
    let index = match expr {
        Sym::Sym(name) => {
            // The reference's name -> index dictionary is filled in declaration order, so a
            // repeated parameter name binds to its LAST position (and later validation refuses
            // the resulting out-of-range operand when the node list is shorter).
            let position = parameters
                .iter()
                .rposition(|parameter| parameter == name)
                .ok_or_else(|| LangError::plain("Node kind Symbol cannot be lowered to MathIR."))?;
            add_node(program, IrOpKind::Parameter, vec![position], 0)
        }
        Sym::Num(value) => {
            let pool = add_constant(program, &canonical_rational(value));
            add_node(program, IrOpKind::Constant, vec![pool], 0)
        }
        Sym::Real(text) => {
            let pool = add_constant(program, &format!("(real {text})"));
            add_node(program, IrOpKind::Constant, vec![pool], 0)
        }
        Sym::Add(terms) => emit_nary(terms, IrOpKind::Add, parameters, program, cache, ordered)?,
        Sym::Mul(factors) => {
            emit_nary(factors, IrOpKind::Mul, parameters, program, cache, ordered)?
        }
        Sym::Pow(base, exponent) => match exponent.as_ref() {
            Sym::Num(value) if value.is_integer() && value.numerator().to_decimal() == "-1" => {
                let base = emit(base, parameters, program, cache, ordered)?;
                add_node(program, IrOpKind::Reciprocal, vec![base], 0)
            }
            Sym::Num(value)
                if value.denominator().to_decimal() == "2"
                    && value.numerator().to_decimal() == "1" =>
            {
                let base = emit(base, parameters, program, cache, ordered)?;
                add_node(program, IrOpKind::Sqrt, vec![base], 0)
            }
            Sym::Num(value) if value.is_integer() => {
                let base = emit(base, parameters, program, cache, ordered)?;
                // The exponent is a FRESH constant node on every occurrence; the pool entry
                // is deduplicated by value but the node is not (the reference's PowInt path).
                let canonical = canonical_rational(value);
                let exponent = add_constant(program, &canonical);
                let exponent = add_node(program, IrOpKind::Constant, vec![exponent], 0);
                add_node(program, IrOpKind::PowInt, vec![base, exponent], 0)
            }
            _ => {
                let base = emit(base, parameters, program, cache, ordered)?;
                let exponent = emit(exponent, parameters, program, cache, ordered)?;
                add_node(program, IrOpKind::Pow, vec![base, exponent], 0)
            }
        },
        Sym::Div(left, right) => {
            let left = emit(left, parameters, program, cache, ordered)?;
            let right = emit(right, parameters, program, cache, ordered)?;
            add_node(program, IrOpKind::Div, vec![left, right], 0)
        }
        Sym::Fun(name, arguments) => match name.as_str() {
            "piecewise" => emit_piecewise(arguments, parameters, program, cache, ordered)?,
            "and" | "or" | "not" => {
                return Err(LangError::plain(
                    "Piecewise guards must be boolean-valued expressions.",
                ));
            }
            "pi" | "e" | "i" | "infinity" if arguments.is_empty() => {
                let named = match name.as_str() {
                    "pi" => "Pi",
                    "e" => "E",
                    "i" => "I",
                    _ => "Infinity",
                };
                let pool = add_constant(program, named);
                add_node(program, IrOpKind::Constant, vec![pool], 1)
            }
            "sqrt" if arguments.len() == 1 => {
                let argument = emit(&arguments[0], parameters, program, cache, ordered)?;
                add_node(program, IrOpKind::Sqrt, vec![argument], 0)
            }
            _ => {
                let op = function_op(name).ok_or_else(|| {
                    LangError::plain(format!("Function '{name}' cannot be lowered to MathIR."))
                })?;
                let mut operands = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    operands.push(emit(argument, parameters, program, cache, ordered)?);
                }
                add_node(program, op, operands, 0)
            }
        },
        Sym::Rel(_, _, _) => {
            return Err(LangError::plain(
                "Node kind Relation cannot be lowered to MathIR.",
            ));
        }
    };
    cache.push((expr.clone(), index));
    Ok(index)
}

fn emit_nary(
    items: &[Sym],
    op: IrOpKind,
    parameters: &[String],
    program: &mut IrProgram,
    cache: &mut Vec<(Sym, usize)>,
    ordered: bool,
) -> Result<usize, LangError> {
    if items.is_empty() {
        // The reference's identity constant for an empty n-ary node is a literal zero node.
        let pool = add_constant(program, &canonical_rational(&Rat::zero()));
        return Ok(add_node(program, IrOpKind::Constant, vec![pool], 0));
    }
    let mut ordered_items: Vec<&Sym> = items.iter().collect();
    if ordered {
        ordered_items.sort_by(|a, b| lovelace_symbolics::canonical_order(a, b));
    }
    let mut emitted = Vec::with_capacity(items.len());
    for item in ordered_items {
        emitted.push(emit(item, parameters, program, cache, ordered)?);
    }
    let mut accumulator = emitted[0];
    for operand in &emitted[1..] {
        accumulator = add_node(program, op, vec![accumulator, *operand], 0);
    }
    Ok(accumulator)
}

fn emit_piecewise(
    arguments: &[Sym],
    parameters: &[String],
    program: &mut IrProgram,
    cache: &mut Vec<(Sym, usize)>,
    ordered: bool,
) -> Result<usize, LangError> {
    if arguments.len() < 3 || arguments.len() % 2 == 0 {
        return Err(LangError::plain(
            "Piecewise guards must be boolean-valued expressions.",
        ));
    }
    let mut result = emit(
        arguments.last().expect("at least one argument"),
        parameters,
        program,
        cache,
        ordered,
    )?;
    let branches = (arguments.len() - 1) / 2;
    for branch in (0..branches).rev() {
        let guard = emit_guard(&arguments[2 * branch], parameters, program, cache, ordered)?;
        let value = emit(
            &arguments[2 * branch + 1],
            parameters,
            program,
            cache,
            ordered,
        )?;
        result = add_node(program, IrOpKind::Select, vec![guard, value, result], 0);
    }
    Ok(result)
}

fn emit_guard(
    guard: &Sym,
    parameters: &[String],
    program: &mut IrProgram,
    cache: &mut Vec<(Sym, usize)>,
    ordered: bool,
) -> Result<usize, LangError> {
    match guard {
        Sym::Rel(op, left, right) => {
            let left = emit(left, parameters, program, cache, ordered)?;
            let right = emit(right, parameters, program, cache, ordered)?;
            let op = match op {
                RelOp::Eq => IrOpKind::Eq,
                RelOp::Ne => IrOpKind::Ne,
                RelOp::Lt => IrOpKind::Lt,
                RelOp::Le => IrOpKind::Le,
                RelOp::Gt => IrOpKind::Gt,
                RelOp::Ge => IrOpKind::Ge,
            };
            Ok(add_node(program, op, vec![left, right], 0))
        }
        Sym::Fun(name, operands) if name == "and" && !operands.is_empty() => {
            let mut result = emit_guard(&operands[0], parameters, program, cache, ordered)?;
            for operand in &operands[1..] {
                let next = emit_guard(operand, parameters, program, cache, ordered)?;
                let zero = const_node(program, Rat::zero());
                result = add_node(program, IrOpKind::Select, vec![result, next, zero], 0);
            }
            Ok(result)
        }
        Sym::Fun(name, operands) if name == "or" && !operands.is_empty() => {
            let mut result = emit_guard(&operands[0], parameters, program, cache, ordered)?;
            for operand in &operands[1..] {
                let next = emit_guard(operand, parameters, program, cache, ordered)?;
                let one = const_node(program, Rat::one());
                result = add_node(program, IrOpKind::Select, vec![result, one, next], 0);
            }
            Ok(result)
        }
        Sym::Fun(name, operands) if name == "not" && operands.len() == 1 => {
            let inner = emit_guard(&operands[0], parameters, program, cache, ordered)?;
            let zero = const_node(program, Rat::zero());
            let one = const_node(program, Rat::one());
            Ok(add_node(
                program,
                IrOpKind::Select,
                vec![inner, zero, one],
                0,
            ))
        }
        Sym::Num(value) => {
            let value = if value.is_zero() {
                Rat::zero()
            } else {
                Rat::one()
            };
            Ok(const_node(program, value))
        }
        _ => Err(LangError::plain(
            "Piecewise guards must be boolean-valued expressions.",
        )),
    }
}

fn const_node(program: &mut IrProgram, value: Rat) -> usize {
    let pool = add_constant(program, &canonical_rational(&value));
    add_node(program, IrOpKind::Constant, vec![pool], 0)
}

fn function_op(name: &str) -> Option<IrOpKind> {
    Some(match name {
        "exp" => IrOpKind::Exp,
        "log" => IrOpKind::Log,
        "sin" => IrOpKind::Sin,
        "cos" => IrOpKind::Cos,
        "tan" => IrOpKind::Tan,
        "asin" => IrOpKind::Asin,
        "acos" => IrOpKind::Acos,
        "atan" => IrOpKind::Atan,
        "sinh" => IrOpKind::Sinh,
        "cosh" => IrOpKind::Cosh,
        "tanh" => IrOpKind::Tanh,
        "abs" => IrOpKind::Abs,
        "sign" => IrOpKind::Sign,
        "floor" => IrOpKind::Floor,
        "ceil" => IrOpKind::Ceil,
        "min" => IrOpKind::Min,
        "max" => IrOpKind::Max,
        _ => return None,
    })
}

fn canonical_rational(value: &Rat) -> String {
    format!(
        "(rat {} {})",
        value.numerator().to_decimal(),
        value.denominator().to_decimal()
    )
}

fn add_constant(program: &mut IrProgram, canonical: &str) -> usize {
    match program
        .constants()
        .iter()
        .position(|entry| entry == canonical)
    {
        Some(index) => index,
        None => {
            program.constants.push(canonical.to_string());
            program.constants.len() - 1
        }
    }
}

fn add_node(program: &mut IrProgram, op: IrOpKind, operands: Vec<usize>, aux: i32) -> usize {
    program.nodes.push(IrNode::new(op, operands, aux));
    program.nodes.len() - 1
}

// ---------------------------------------------------------------------------
// Typing helpers (the reference's IrTyping internals)
// ---------------------------------------------------------------------------

fn operand_type(types: &[Option<IrType>], node: &IrNode, position: usize) -> Option<IrType> {
    node.operands()
        .get(position)
        .and_then(|operand| types.get(*operand))
        .copied()
        .flatten()
}

fn pool_text<'a>(program: &'a IrProgram, node: &IrNode) -> &'a str {
    node.operands()
        .first()
        .and_then(|index| program.constant(*index))
        .unwrap_or_default()
}

fn classify_constant(canonical: &str) -> IrType {
    if canonical.starts_with("(int ") {
        return IrType::Integer;
    }
    if let Some(body) = canonical
        .strip_prefix("(rat ")
        .map(|rest| rest.trim_end_matches(')'))
    {
        let mut parts = body.split(' ');
        let _numerator = parts.next();
        if parts.next() == Some("1") {
            return IrType::Integer;
        }
        return IrType::Rational;
    }
    if canonical.starts_with("(real ") {
        return IrType::Real;
    }
    if canonical.starts_with("(cplx ") {
        return IrType::Complex;
    }
    if canonical == "I" {
        IrType::Complex
    } else {
        IrType::Real
    }
}

fn promote(left: Option<IrType>, right: Option<IrType>) -> Option<IrType> {
    if left == Some(IrType::Complex) || right == Some(IrType::Complex) {
        return Some(IrType::Complex);
    }
    if left == Some(IrType::Real) || right == Some(IrType::Real) {
        return Some(IrType::Real);
    }
    if left == Some(IrType::Rational) || right == Some(IrType::Rational) {
        return Some(IrType::Rational);
    }
    if left == Some(IrType::Integer) && right == Some(IrType::Integer) {
        return Some(IrType::Integer);
    }
    if left.is_none() { right } else { left }
}

/// The reference's UNARY transcendental rule (`Typing.cs`: `Transcendental(T(0))`): an
/// unconstrained input stays unconstrained, a Complex input is Complex, any other typed input is
/// Real. Deliberately not `transcendental(input, None)`: the two-input form treats the synthetic
/// second input as unknown and would answer None for every typed operand.
fn transcendental_unary(input: Option<IrType>) -> Option<IrType> {
    match input {
        None => None,
        Some(IrType::Complex) => Some(IrType::Complex),
        Some(_) => Some(IrType::Real),
    }
}

fn transcendental(left: Option<IrType>, right: Option<IrType>) -> Option<IrType> {
    let inputs = [left, right];
    if inputs.contains(&Some(IrType::Complex)) {
        return Some(IrType::Complex);
    }
    if inputs.contains(&None) {
        return None;
    }
    Some(IrType::Real)
}

fn arity_error(index: usize, node: &IrNode) -> LangError {
    LangError::plain(format!(
        "IR validation: node {index} ({}) has {} operands; expected {}.",
        node.op(),
        node.arity(),
        node.op().arity()
    ))
}

fn operand_error(index: usize, node: &IrNode, operand: usize) -> LangError {
    LangError::plain(format!(
        "IR validation: node {index} ({}) references out-of-range operand {operand}.",
        node.op()
    ))
}

fn pool_error(index: usize, pool: usize) -> LangError {
    LangError::plain(format!(
        "IR validation: Constant node {index} references out-of-range pool index {pool}."
    ))
}

fn parameter_error(index: usize, parameter: usize) -> LangError {
    LangError::plain(format!(
        "IR validation: Parameter node {index} references out-of-range parameter index {parameter}."
    ))
}

fn malformed_line(line: &str) -> LangError {
    LangError::coded(
        format!("Malformed IR line: {line}"),
        0,
        "InvalidInput",
        "ParseError",
    )
}

fn check_node(program: &IrProgram, index: usize, node: &IrNode) -> Result<(), LangError> {
    if node.arity() != node.op().arity() {
        return Err(arity_error(index, node));
    }
    for &operand in node.operands() {
        if operand >= program.nodes().len() {
            return Err(operand_error(index, node, operand));
        }
    }
    if node.op() == IrOpKind::Constant && node.operands()[0] >= program.constants().len() {
        return Err(pool_error(index, node.operands()[0]));
    }
    if node.op() == IrOpKind::Parameter && node.operands()[0] >= program.parameters().len() {
        return Err(parameter_error(index, node.operands()[0]));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Evaluation (the reference's IrEvaluator and NumOps)
// ---------------------------------------------------------------------------

fn ambient_places() -> usize {
    Real::max_computation_decimal_places().max(1) as usize
}

fn parse_constants(program: &IrProgram, places: usize) -> Result<Vec<Num>, LangError> {
    program
        .constants()
        .iter()
        .map(|constant| parse_constant(constant, places))
        .collect()
}

fn parse_constant(canonical: &str, places: usize) -> Result<Num, LangError> {
    match canonical {
        "Pi" => Ok(Num::Real(Real::pi(places))),
        "E" => Ok(Num::Real(Real::e(places))),
        "I" => Ok(Num::Complex(Complex::i())),
        "Infinity" => Err(LangError::plain("Infinity has no numeric value.")),
        _ => {
            let expr = canonical_parse(canonical).map_err(|_| {
                LangError::plain(format!(
                    "MathIR constant '{canonical}' is not a canonical expression."
                ))
            })?;
            crate::numeric::evaluate(&expr, places)
        }
    }
}

fn constant_or_parameter(
    program: &IrProgram,
    node: &IrNode,
    constants: &[Num],
    bindings: &HashMap<String, Num>,
) -> Result<Num, LangError> {
    match node.op() {
        IrOpKind::Constant => constants
            .get(node.operands()[0])
            .cloned()
            .ok_or_else(|| pool_error(0, node.operands()[0])),
        IrOpKind::Parameter => {
            let name = program
                .parameter(node.operands()[0])
                .ok_or_else(|| parameter_error(0, node.operands()[0]))?;
            bindings
                .get(name)
                .cloned()
                .ok_or_else(|| LangError::plain(format!("No value for parameter '{name}'.")))
        }
        _ => unreachable!("only Constant and Parameter are pool/binding nodes"),
    }
}

fn constant_or_parameter_vector(
    program: &IrProgram,
    node: &IrNode,
    constants: &[Num],
    columns: &HashMap<String, Vec<Num>>,
    lanes: usize,
) -> Result<Vec<Num>, LangError> {
    match node.op() {
        IrOpKind::Constant => {
            let value = constants
                .get(node.operands()[0])
                .cloned()
                .ok_or_else(|| pool_error(0, node.operands()[0]))?;
            Ok(vec![value; lanes])
        }
        IrOpKind::Parameter => {
            let name = program
                .parameter(node.operands()[0])
                .ok_or_else(|| parameter_error(0, node.operands()[0]))?;
            columns.get(name).cloned().ok_or_else(|| {
                LangError::coded(
                    format!("Missing column for parameter '{name}'."),
                    0,
                    "InvalidArgument",
                    "TypeMismatch",
                )
            })
        }
        _ => unreachable!("only Constant and Parameter are pool/binding nodes"),
    }
}

fn is_zero(value: &Num) -> bool {
    match value {
        Num::Exact(rational) => rational.is_zero(),
        Num::Real(real) => real.is_zero(),
        Num::Complex(complex) => complex.re().is_zero() && complex.im().is_zero(),
    }
}

fn to_real(value: &Num, places: usize) -> Result<Real, LangError> {
    value.to_real(places).ok_or_else(complex_to_real_error)
}

fn compare(left: &Num, right: &Num, places: usize) -> Result<Ordering, LangError> {
    if let (Num::Exact(a), Num::Exact(b)) = (left, right) {
        return Ok(a.cmp(b));
    }
    Ok(to_real(left, places)?.cmp(&to_real(right, places)?))
}

fn complex_to_real_error() -> LangError {
    LangError::plain("Complex value cannot convert to Real (use re()/im()).")
}

fn bool_num(value: bool) -> Num {
    Num::Exact(Rat::from_i64(i64::from(value)))
}

fn apply(op: IrOpKind, args: &[Num], places: usize) -> Result<Num, LangError> {
    match op {
        IrOpKind::Add => args[0].add(&args[1], places),
        IrOpKind::Sub => args[0].sub(&args[1], places),
        IrOpKind::Mul => args[0].mul(&args[1], places),
        IrOpKind::Div => args[0].div(&args[1], places),
        IrOpKind::Negate => Ok(args[0].neg()),
        IrOpKind::Reciprocal => Num::Exact(Rat::one()).div(&args[0], places),
        IrOpKind::PowInt => match &args[1] {
            Num::Exact(exponent) if exponent.is_integer() => args[0].pow(&args[1], places),
            _ => Err(LangError::plain(
                "PowInt requires an exact integer exponent.",
            )),
        },
        IrOpKind::Pow => args[0].pow(&args[1], places),
        IrOpKind::Sqrt => {
            let half = Rat::from_parts(Int::from_i64(1), Nat::from_u64(2));
            args[0].pow(&Num::Exact(half), places)
        }
        IrOpKind::Exp => match &args[0] {
            Num::Complex(value) => Ok(Num::Complex(complex_math::exp(value, places))),
            other => Ok(Num::Real(to_real(other, places)?.exp(places))),
        },
        IrOpKind::Log => match &args[0] {
            Num::Complex(value) => complex_math::log(value, places)
                .map(Num::Complex)
                .ok_or_else(ln_domain_error),
            other => {
                let real = to_real(other, places)?;
                if real.is_negative() {
                    complex_math::log(&Complex::from_real(real), places)
                        .map(Num::Complex)
                        .ok_or_else(ln_domain_error)
                } else if real.is_zero() {
                    Err(ln_domain_error())
                } else {
                    real.ln(places).map(Num::Real).ok_or_else(ln_domain_error)
                }
            }
        },
        IrOpKind::Sin => match &args[0] {
            Num::Complex(value) => Ok(Num::Complex(complex_math::sin(value, places))),
            other => {
                let real = to_real(other, places)?;
                let (sine, _) = real.sin_cos_raw(places);
                Ok(Num::Real(lovelace_real::transcendental::truncate_frac(
                    &sine, places,
                )))
            }
        },
        IrOpKind::Cos => match &args[0] {
            Num::Complex(value) => Ok(Num::Complex(complex_math::cos(value, places))),
            other => {
                let real = to_real(other, places)?;
                let (_, cosine) = real.sin_cos_raw(places);
                Ok(Num::Real(lovelace_real::transcendental::truncate_frac(
                    &cosine, places,
                )))
            }
        },
        IrOpKind::Tan => {
            let sine = apply(IrOpKind::Sin, args, places)?;
            let cosine = apply(IrOpKind::Cos, args, places)?;
            sine.div(&cosine, places)
        }
        IrOpKind::Asin => {
            let real = to_real(&args[0], places)?;
            real.asin(places)
                .map(Num::Real)
                .ok_or_else(|| domain_argument_error("AsinReal(x) requires |x| <= 1."))
        }
        IrOpKind::Acos => {
            let real = to_real(&args[0], places)?;
            real.acos(places)
                .map(Num::Real)
                .ok_or_else(|| domain_argument_error("AcosReal(x) requires |x| <= 1."))
        }
        IrOpKind::Atan => {
            let real = to_real(&args[0], places)?;
            Ok(Num::Real(real.atan(places)))
        }
        IrOpKind::Sinh => {
            let real = to_real(&args[0], places)?;
            Ok(Num::Real(real.sinh(places)))
        }
        IrOpKind::Cosh => {
            let real = to_real(&args[0], places)?;
            Ok(Num::Real(real.cosh(places)))
        }
        IrOpKind::Tanh => {
            let real = to_real(&args[0], places)?;
            Ok(Num::Real(real.tanh(places)))
        }
        IrOpKind::Abs => match &args[0] {
            Num::Exact(rational) => Ok(Num::Exact(rational.abs())),
            Num::Real(real) => Ok(Num::Real(real.abs())),
            Num::Complex(complex) => Ok(Num::Real(complex.magnitude(places))),
        },
        IrOpKind::Sign => {
            if is_zero(&args[0]) {
                return Ok(Num::Exact(Rat::zero()));
            }
            match &args[0] {
                Num::Complex(complex) => {
                    let magnitude = complex.magnitude(places);
                    let quotient = complex
                        .div_at(&Complex::from_real(magnitude), places)
                        .ok_or_else(|| {
                            LangError::plain("Division by zero during numeric evaluation.")
                        })?;
                    Ok(Num::Complex(quotient))
                }
                other => {
                    let positive =
                        compare(other, &Num::Exact(Rat::zero()), places)? == Ordering::Greater;
                    Ok(Num::Exact(Rat::from_i64(if positive { 1 } else { -1 })))
                }
            }
        }
        IrOpKind::Floor => floor_num(&args[0]),
        IrOpKind::Ceil => {
            let floor = floor_num(&args[0])?;
            let difference = args[0].sub(&floor, places)?;
            if is_zero(&difference) {
                Ok(floor)
            } else {
                floor.add(&Num::Exact(Rat::one()), places)
            }
        }
        IrOpKind::Min => {
            if compare(&args[1], &args[0], places)? == Ordering::Less {
                Ok(args[1].clone())
            } else {
                Ok(args[0].clone())
            }
        }
        IrOpKind::Max => {
            if compare(&args[1], &args[0], places)? == Ordering::Greater {
                Ok(args[1].clone())
            } else {
                Ok(args[0].clone())
            }
        }
        IrOpKind::Eq => Ok(bool_num(
            compare(&args[0], &args[1], places)? == Ordering::Equal,
        )),
        IrOpKind::Ne => Ok(bool_num(
            compare(&args[0], &args[1], places)? != Ordering::Equal,
        )),
        IrOpKind::Lt => Ok(bool_num(
            compare(&args[0], &args[1], places)? == Ordering::Less,
        )),
        IrOpKind::Le => Ok(bool_num(
            compare(&args[0], &args[1], places)? != Ordering::Greater,
        )),
        IrOpKind::Gt => Ok(bool_num(
            compare(&args[0], &args[1], places)? == Ordering::Greater,
        )),
        IrOpKind::Ge => Ok(bool_num(
            compare(&args[0], &args[1], places)? != Ordering::Less,
        )),
        IrOpKind::Constant | IrOpKind::Parameter | IrOpKind::Select => {
            Err(LangError::plain(format!("Unsupported IR op {op}.")))
        }
    }
}

fn floor_num(value: &Num) -> Result<Num, LangError> {
    match value {
        Num::Exact(rational) => Ok(Num::Exact(rational_floor(rational))),
        Num::Real(real) => Ok(Num::Real(real_floor(real))),
        Num::Complex(_) => Err(LangError::plain("floor() is real-only.")),
    }
}

fn rational_floor(value: &Rat) -> Rat {
    let negative = value.is_negative();
    let magnitude = value.numerator().magnitude();
    let (whole, remainder) = magnitude
        .div_rem(value.denominator())
        .expect("the denominator is never zero");
    let mut integer = Int::from_decimal(&whole.to_decimal()).unwrap_or_else(Int::zero);
    if negative {
        integer = integer.negate();
        if !remainder.is_zero() {
            integer = integer.add(&Int::from_i64(1).negate());
        }
    }
    Rat::from_parts(integer, Nat::from_u64(1))
}

fn real_floor(value: &Real) -> Real {
    let truncated = lovelace_real::fast_path::truncate(value);
    if value.is_negative() && !value.is_integer() {
        truncated.sub(&Real::from_i64(1))
    } else {
        truncated
    }
}

fn ln_domain_error() -> LangError {
    LangError::plain("Ln(x) requires x > 0. (Parameter 'x')")
}

fn domain_argument_error(message: &str) -> LangError {
    LangError::coded(
        format!("{message} (Parameter 'x')"),
        0,
        "InvalidArgument",
        "TypeMismatch",
    )
}
