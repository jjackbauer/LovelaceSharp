namespace Lovelace.Suite;

// -------------------------------------------------------------------------
// Operator enumerations
// -------------------------------------------------------------------------

/// <summary>Binary operators supported in expressions.</summary>
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

/// <summary>Prefix unary operators supported in expressions.</summary>
public enum UnaryOp
{
    /// <summary>Arithmetic negation (<c>-x</c>).</summary>
    Negate,

    /// <summary>Unary plus — identity (<c>+x</c>).</summary>
    Plus,
}

/// <summary>Postfix operators supported in expressions.</summary>
public enum PostfixOp
{
    /// <summary>Factorial (<c>x!</c>).</summary>
    Factorial,
}

// -------------------------------------------------------------------------
// Expr — abstract base and concrete subtypes
// -------------------------------------------------------------------------

/// <summary>Abstract base for all expression AST nodes.</summary>
public abstract record Expr;

/// <summary>A numeric literal; raw text is kept verbatim so type inference is deferred.</summary>
public sealed record LiteralExpr(string RawText) : Expr;

/// <summary>A reference to a named variable.</summary>
public sealed record VariableExpr(string Name) : Expr;

/// <summary>An assignment expression <c>name = Value</c>.</summary>
public sealed record AssignExpr(string Name, Expr Value) : Expr;

/// <summary>A binary infix expression <c>Left Op Right</c>.</summary>
public sealed record BinaryExpr(Expr Left, BinaryOp Op, Expr Right) : Expr;

/// <summary>A prefix unary expression <c>Op Operand</c>.</summary>
public sealed record UnaryExpr(UnaryOp Op, Expr Operand) : Expr;

/// <summary>A postfix expression <c>Operand Op</c> (e.g. <c>5!</c>).</summary>
public sealed record PostfixExpr(Expr Operand, PostfixOp Op) : Expr;

/// <summary>A function call <c>FunctionName(arg0, arg1, …)</c>.</summary>
public sealed record CallExpr(string FunctionName, List<Expr> Arguments) : Expr;

/// <summary>A plain string literal <c>"text"</c>.</summary>
public sealed record StringExpr(string Value) : Expr;

/// <summary>A range expression <c>start..end</c> or <c>start..step..end</c>.</summary>
public sealed record RangeExpr(Expr Start, Expr? Step, Expr End) : Expr;

/// <summary>An element access <c>target[index]</c>.</summary>
public sealed record IndexExpr(Expr Target, Expr Index) : Expr;

/// <summary>A list literal <c>[a, b, c]</c>.</summary>
public sealed record ListExpr(List<Expr> Elements) : Expr;

/// <summary>An interpolated string <c>$"… {expr} …"</c>.</summary>
public sealed record InterpolatedStringExpr(List<InterpolationPart> Parts) : Expr;

/// <summary>A part of an interpolated string: literal text or an embedded expression.</summary>
public abstract record InterpolationPart;

/// <summary>Literal text within an interpolated string.</summary>
public sealed record TextPart(string Text) : InterpolationPart;

/// <summary>An embedded <c>{expr}</c> within an interpolated string.</summary>
public sealed record ExpressionPart(Expr Expression) : InterpolationPart;

// -------------------------------------------------------------------------
// Statement — abstract base and concrete subtypes
// -------------------------------------------------------------------------

/// <summary>Abstract base for all statement AST nodes.</summary>
public abstract record Statement;

/// <summary>A statement consisting of a single expression.</summary>
public sealed record ExpressionStatement(Expr Expression) : Statement;

/// <summary>A block <c>{ s1; s2; … }</c> introducing a lexical scope.</summary>
public sealed record BlockStatement(List<Statement> Statements) : Statement;

/// <summary>An <c>if (cond) then [else otherwise]</c> statement.</summary>
public sealed record IfStatement(Expr Condition, Statement Then, Statement? Else) : Statement;

/// <summary>A <c>while (cond) body</c> loop.</summary>
public sealed record WhileStatement(Expr Condition, Statement Body) : Statement;

/// <summary>A <c>for name in range body</c> counted loop.</summary>
public sealed record ForStatement(string Variable, Expr Range, Statement Body) : Statement;

/// <summary>A <c>return [expr]</c> statement.</summary>
public sealed record ReturnStatement(Expr? Value) : Statement;

/// <summary>A <c>break</c> statement.</summary>
public sealed record BreakStatement : Statement;

/// <summary>A <c>continue</c> statement.</summary>
public sealed record ContinueStatement : Statement;

/// <summary>A <c>func name(params) …</c> definition statement.</summary>
public sealed record FunctionStatement(FunctionDefinition Definition) : Statement;

/// <summary>A parsed program: an ordered list of statements.</summary>
public sealed record Program(List<Statement> Statements);
