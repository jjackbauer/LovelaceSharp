//! Abstract syntax tree for the Lovelace scripting language.
//!
//! The tree mirrors the reference C# parser: expressions form a precedence chain
//! (assignment, comparison, additive, multiplicative, power, range, unary, postfix,
//! primary) and statements carry the block structure the interpreter walks.

use std::rc::Rc;

/// A binary infix operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    /// Addition (`+`).
    Add,
    /// Subtraction (`-`).
    Subtract,
    /// Multiplication (`*`).
    Multiply,
    /// Division (`/`).
    Divide,
    /// Remainder (`%`).
    Modulo,
    /// Power (`^`).
    Power,
    /// Equality (`==`).
    Equal,
    /// Inequality (`!=`).
    NotEqual,
    /// Greater than (`>`).
    Greater,
    /// Less than (`<`).
    Less,
    /// Greater than or equal (`>=`).
    GreaterEqual,
    /// Less than or equal (`<=`).
    LessEqual,
}

impl BinOp {
    /// The canonical source spelling of this operator.
    pub fn symbol(self) -> &'static str {
        match self {
            BinOp::Add => "+",
            BinOp::Subtract => "-",
            BinOp::Multiply => "*",
            BinOp::Divide => "/",
            BinOp::Modulo => "%",
            BinOp::Power => "^",
            BinOp::Equal => "==",
            BinOp::NotEqual => "!=",
            BinOp::Greater => ">",
            BinOp::Less => "<",
            BinOp::GreaterEqual => ">=",
            BinOp::LessEqual => "<=",
        }
    }

    /// True for the six relational/equality operators.
    pub fn is_comparison(self) -> bool {
        matches!(
            self,
            BinOp::Equal
                | BinOp::NotEqual
                | BinOp::Greater
                | BinOp::Less
                | BinOp::GreaterEqual
                | BinOp::LessEqual
        )
    }
}

/// A prefix unary operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnOp {
    /// Arithmetic negation (`-x`).
    Negate,
    /// Identity (`+x`).
    Plus,
}

/// One part of an interpolated string.
#[derive(Clone, Debug)]
pub enum InterpPart {
    /// Literal text.
    Text(String),
    /// An embedded expression.
    Expr(Expr),
}

/// One axis specifier inside `target[...]`.
#[derive(Clone, Debug)]
pub enum IndexSpec {
    /// A single index expression.
    Scalar(Expr),
    /// A half-open slice `start:stop:step`, each bound optional.
    Slice {
        /// Inclusive start bound.
        start: Option<Expr>,
        /// Exclusive stop bound.
        stop: Option<Expr>,
        /// Positive step.
        step: Option<Expr>,
    },
}

/// An expression node.
#[derive(Clone, Debug)]
pub enum Expr {
    /// A numeric literal, kept as source text so its kind is inferred at evaluation.
    Literal(String),
    /// A plain text literal.
    Text(String),
    /// An interpolated string.
    Interpolated(Vec<InterpPart>),
    /// A variable reference.
    Var(String),
    /// An assignment `name = value`.
    Assign(String, Box<Expr>),
    /// A binary infix expression.
    Binary(BinOp, Box<Expr>, Box<Expr>),
    /// A prefix unary expression.
    Unary(UnOp, Box<Expr>),
    /// A postfix factorial `x!`.
    Factorial(Box<Expr>),
    /// A function call `name(args...)`.
    Call(String, Vec<Expr>),
    /// A range `start..end` or `start..step..end`.
    Range {
        /// First element.
        start: Box<Expr>,
        /// Explicit step, when the three-part form is used.
        step: Option<Box<Expr>>,
        /// Last element (inclusive).
        end: Box<Expr>,
    },
    /// An index or slice access.
    Index(Box<Expr>, Vec<IndexSpec>),
    /// A record member access `target.name`.
    Member(Box<Expr>, String),
    /// A list literal `[a, b, c]`.
    List(Vec<Expr>),
}

/// A user-defined function.
#[derive(Clone, Debug)]
pub struct FunctionDef {
    /// The function name.
    pub name: String,
    /// Parameter names in declaration order.
    pub params: Vec<String>,
    /// The function body.
    pub body: Vec<Stmt>,
}

/// A statement node.
#[derive(Clone, Debug)]
pub enum Stmt {
    /// An expression statement.
    Expr(Expr),
    /// A braced block introducing a lexical scope.
    Block(Vec<Stmt>),
    /// `if (condition) then [else otherwise]`.
    If {
        /// The condition expression.
        condition: Expr,
        /// The true branch.
        then_branch: Box<Stmt>,
        /// The optional false branch.
        else_branch: Option<Box<Stmt>>,
    },
    /// `while (condition) body`.
    While {
        /// The loop condition.
        condition: Expr,
        /// The loop body.
        body: Box<Stmt>,
    },
    /// `for name in range body`.
    For {
        /// The loop variable.
        variable: String,
        /// The iterable expression.
        range: Expr,
        /// The loop body.
        body: Box<Stmt>,
    },
    /// `return [expr]`.
    Return(Option<Expr>),
    /// `break`.
    Break,
    /// `continue`.
    Continue,
    /// A function definition.
    Func(Rc<FunctionDef>),
}
