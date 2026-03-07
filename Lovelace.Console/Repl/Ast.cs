namespace Lovelace.Console.Repl;

// -------------------------------------------------------------------------
// Operator enumerations
// -------------------------------------------------------------------------

/// <summary>Binary operators supported in REPL expressions.</summary>
public enum BinaryOp
{
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,

    // Comparison
    Equal,
    NotEqual,
    Greater,
    Less,
    GreaterEqual,
    LessEqual,
}

/// <summary>Prefix unary operators supported in REPL expressions.</summary>
public enum UnaryOp
{
    /// <summary>Arithmetic negation (<c>-x</c>).</summary>
    Negate,

    /// <summary>Unary plus — identity (<c>+x</c>).</summary>
    Plus,
}

/// <summary>Postfix operators supported in REPL expressions.</summary>
public enum PostfixOp
{
    /// <summary>Factorial (<c>x!</c>).</summary>
    Factorial,
}

// -------------------------------------------------------------------------
// Expr — abstract base and concrete subtypes
// -------------------------------------------------------------------------

/// <summary>
/// Abstract base for all expression AST nodes produced by <c>Parser</c> and
/// consumed by <c>Evaluator</c>.
/// </summary>
public abstract record Expr;

/// <summary>
/// A numeric literal at the source position; the raw text is kept verbatim
/// so that type inference can be deferred to the <c>Evaluator</c>.
/// </summary>
/// <param name="RawText">The exact character sequence from the source.</param>
public sealed record LiteralExpr(string RawText) : Expr;

/// <summary>A reference to a named variable.</summary>
/// <param name="Name">The variable name as written in source.</param>
public sealed record VariableExpr(string Name) : Expr;

/// <summary>
/// An assignment expression <c>name = Value</c>.
/// Assignment is right-associative; <c>Value</c> may itself be an
/// <see cref="AssignExpr"/>.
/// </summary>
/// <param name="Name">The variable name being assigned to.</param>
/// <param name="Value">The right-hand-side expression.</param>
public sealed record AssignExpr(string Name, Expr Value) : Expr;

/// <summary>A binary infix expression <c>Left Op Right</c>.</summary>
/// <param name="Left">Left operand.</param>
/// <param name="Op">The binary operator.</param>
/// <param name="Right">Right operand.</param>
public sealed record BinaryExpr(Expr Left, BinaryOp Op, Expr Right) : Expr;

/// <summary>A prefix unary expression <c>Op Operand</c>.</summary>
/// <param name="Op">The unary operator.</param>
/// <param name="Operand">The operand expression.</param>
public sealed record UnaryExpr(UnaryOp Op, Expr Operand) : Expr;

/// <summary>
/// A postfix expression <c>Operand Op</c> (e.g. <c>5!</c>).
/// </summary>
/// <param name="Operand">The operand expression.</param>
/// <param name="Op">The postfix operator.</param>
public sealed record PostfixExpr(Expr Operand, PostfixOp Op) : Expr;

/// <summary>
/// A built-in function call <c>FunctionName(arg0, arg1, …)</c>.
/// </summary>
/// <param name="FunctionName">The name of the built-in function.</param>
/// <param name="Arguments">Ordered list of argument expressions.</param>
public sealed record CallExpr(string FunctionName, List<Expr> Arguments) : Expr;
