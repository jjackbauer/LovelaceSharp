using Lovelace.Console.Repl;

namespace Lovelace.Console.Tests;

/// <summary>
/// Tests for <see cref="Parser.Parse"/> — covering primary expressions,
/// unary/postfix operators, binary operator precedence and associativity,
/// and top-level assignment, as well as error cases.
/// (Test plan items 26–44.)
/// </summary>
public class ParserTests
{
    // Helper: tokenize then parse in one call.
    private static Expr Parse(string input)
    {
        var tokens = new Tokenizer().Tokenize(input);
        return new Parser().Parse(tokens);
    }

    // -----------------------------------------------------------------------
    // Primary expressions — number literal (Test 26)
    // -----------------------------------------------------------------------

    [Fact]
    public void Parse_GivenNumberLiteral_ReturnsLiteralExpr()
    {
        var expr = Parse("42");
        var lit = Assert.IsType<LiteralExpr>(expr);
        Assert.Equal("42", lit.RawText);
    }

    // -----------------------------------------------------------------------
    // Primary expressions — identifier (Test 27)
    // -----------------------------------------------------------------------

    [Fact]
    public void Parse_GivenIdentifier_ReturnsVariableExpr()
    {
        var expr = Parse("x");
        var v = Assert.IsType<VariableExpr>(expr);
        Assert.Equal("x", v.Name);
    }

    // -----------------------------------------------------------------------
    // Primary expressions — function call, single argument (Test 28)
    // -----------------------------------------------------------------------

    [Fact]
    public void Parse_GivenFunctionCallSingleArg_ReturnsCallExpr()
    {
        var expr = Parse("abs(x)");
        var call = Assert.IsType<CallExpr>(expr);
        Assert.Equal("abs", call.FunctionName);
        Assert.Single(call.Arguments);
        var arg = Assert.IsType<VariableExpr>(call.Arguments[0]);
        Assert.Equal("x", arg.Name);
    }

    // -----------------------------------------------------------------------
    // Primary expressions — function call, multiple arguments (Test 29)
    // -----------------------------------------------------------------------

    [Fact]
    public void Parse_GivenFunctionCallMultipleArgs_ReturnsCallExpr()
    {
        var expr = Parse("divrem(a, b)");
        var call = Assert.IsType<CallExpr>(expr);
        Assert.Equal("divrem", call.FunctionName);
        Assert.Equal(2, call.Arguments.Count);
        Assert.Equal("a", Assert.IsType<VariableExpr>(call.Arguments[0]).Name);
        Assert.Equal("b", Assert.IsType<VariableExpr>(call.Arguments[1]).Name);
    }

    // -----------------------------------------------------------------------
    // Primary expressions — parenthesized group (Test 30)
    // -----------------------------------------------------------------------

    [Fact]
    public void Parse_GivenParenthesizedExpression_ReturnsInnerExpr()
    {
        // "(1 + 2)" — the outer parens are consumed as grouping; the result is
        // the BinaryExpr for 1 + 2, not a wrapper node.
        var expr = Parse("(1 + 2)");
        var bin = Assert.IsType<BinaryExpr>(expr);
        Assert.Equal(BinaryOp.Add, bin.Op);
        Assert.Equal("1", Assert.IsType<LiteralExpr>(bin.Left).RawText);
        Assert.Equal("2", Assert.IsType<LiteralExpr>(bin.Right).RawText);
    }

    // -----------------------------------------------------------------------
    // Unary prefix — minus (Test 31)
    // -----------------------------------------------------------------------

    [Fact]
    public void Parse_GivenUnaryMinus_ReturnsUnaryExprNegate()
    {
        var expr = Parse("-5");
        var unary = Assert.IsType<UnaryExpr>(expr);
        Assert.Equal(UnaryOp.Negate, unary.Op);
        Assert.Equal("5", Assert.IsType<LiteralExpr>(unary.Operand).RawText);
    }

    // -----------------------------------------------------------------------
    // Unary prefix — plus (Test 32)
    // -----------------------------------------------------------------------

    [Fact]
    public void Parse_GivenUnaryPlus_ReturnsUnaryExprPlus()
    {
        var expr = Parse("+5");
        var unary = Assert.IsType<UnaryExpr>(expr);
        Assert.Equal(UnaryOp.Plus, unary.Op);
        Assert.Equal("5", Assert.IsType<LiteralExpr>(unary.Operand).RawText);
    }

    // -----------------------------------------------------------------------
    // Postfix — factorial (Test 33)
    // -----------------------------------------------------------------------

    [Fact]
    public void Parse_GivenPostfixFactorial_ReturnsPostfixExpr()
    {
        var expr = Parse("5!");
        var postfix = Assert.IsType<PostfixExpr>(expr);
        Assert.Equal(PostfixOp.Factorial, postfix.Op);
        Assert.Equal("5", Assert.IsType<LiteralExpr>(postfix.Operand).RawText);
    }

    // -----------------------------------------------------------------------
    // Unary + postfix precedence: postfix binds tighter than unary (Test 34)
    // -----------------------------------------------------------------------

    [Fact]
    public void Parse_GivenChainedPostfixAndUnary_CorrectPrecedence()
    {
        // "-5!" should parse as -(5!) not (-5)!
        var expr = Parse("-5!");
        var unary = Assert.IsType<UnaryExpr>(expr);
        Assert.Equal(UnaryOp.Negate, unary.Op);
        var postfix = Assert.IsType<PostfixExpr>(unary.Operand);
        Assert.Equal(PostfixOp.Factorial, postfix.Op);
        Assert.Equal("5", Assert.IsType<LiteralExpr>(postfix.Operand).RawText);
    }

    // -----------------------------------------------------------------------
    // Binary — addition (Test 35)
    // -----------------------------------------------------------------------

    [Fact]
    public void Parse_GivenAddition_ReturnsBinaryExprAdd()
    {
        var expr = Parse("1 + 2");
        var bin = Assert.IsType<BinaryExpr>(expr);
        Assert.Equal(BinaryOp.Add, bin.Op);
        Assert.Equal("1", Assert.IsType<LiteralExpr>(bin.Left).RawText);
        Assert.Equal("2", Assert.IsType<LiteralExpr>(bin.Right).RawText);
    }

    // -----------------------------------------------------------------------
    // Binary — multiply before add (Test 36)
    // -----------------------------------------------------------------------

    [Fact]
    public void Parse_GivenMultiplyBeforeAdd_CorrectPrecedence()
    {
        // "1 + 2 * 3" → Add(1, Multiply(2, 3))
        var expr = Parse("1 + 2 * 3");
        var outer = Assert.IsType<BinaryExpr>(expr);
        Assert.Equal(BinaryOp.Add, outer.Op);
        Assert.Equal("1", Assert.IsType<LiteralExpr>(outer.Left).RawText);
        var inner = Assert.IsType<BinaryExpr>(outer.Right);
        Assert.Equal(BinaryOp.Multiply, inner.Op);
        Assert.Equal("2", Assert.IsType<LiteralExpr>(inner.Left).RawText);
        Assert.Equal("3", Assert.IsType<LiteralExpr>(inner.Right).RawText);
    }

    // -----------------------------------------------------------------------
    // Binary — power is right-associative (Test 37)
    // -----------------------------------------------------------------------

    [Fact]
    public void Parse_GivenPowerRightAssociative_CorrectTree()
    {
        // "2 ^ 3 ^ 4" → Power(2, Power(3, 4))
        var expr = Parse("2 ^ 3 ^ 4");
        var outer = Assert.IsType<BinaryExpr>(expr);
        Assert.Equal(BinaryOp.Power, outer.Op);
        Assert.Equal("2", Assert.IsType<LiteralExpr>(outer.Left).RawText);
        var inner = Assert.IsType<BinaryExpr>(outer.Right);
        Assert.Equal(BinaryOp.Power, inner.Op);
        Assert.Equal("3", Assert.IsType<LiteralExpr>(inner.Left).RawText);
        Assert.Equal("4", Assert.IsType<LiteralExpr>(inner.Right).RawText);
    }

    // -----------------------------------------------------------------------
    // Binary — parentheses override precedence (Test 38)
    // -----------------------------------------------------------------------

    [Fact]
    public void Parse_GivenParenthesesOverridePrecedence_CorrectTree()
    {
        // "(1 + 2) * 3" → Multiply(Add(1, 2), 3)
        var expr = Parse("(1 + 2) * 3");
        var outer = Assert.IsType<BinaryExpr>(expr);
        Assert.Equal(BinaryOp.Multiply, outer.Op);
        var left = Assert.IsType<BinaryExpr>(outer.Left);
        Assert.Equal(BinaryOp.Add, left.Op);
        Assert.Equal("1", Assert.IsType<LiteralExpr>(left.Left).RawText);
        Assert.Equal("2", Assert.IsType<LiteralExpr>(left.Right).RawText);
        Assert.Equal("3", Assert.IsType<LiteralExpr>(outer.Right).RawText);
    }

    // -----------------------------------------------------------------------
    // Binary — comparison (Test 39)
    // -----------------------------------------------------------------------

    [Fact]
    public void Parse_GivenComparison_ReturnsBinaryExprWithOp()
    {
        var expr = Parse("a == b");
        var bin = Assert.IsType<BinaryExpr>(expr);
        Assert.Equal(BinaryOp.Equal, bin.Op);
        Assert.Equal("a", Assert.IsType<VariableExpr>(bin.Left).Name);
        Assert.Equal("b", Assert.IsType<VariableExpr>(bin.Right).Name);
    }

    // -----------------------------------------------------------------------
    // Binary — comparison has lower precedence than additive (Test 40)
    // -----------------------------------------------------------------------

    [Fact]
    public void Parse_GivenComparisonLowerThanArithmetic_CorrectPrecedence()
    {
        // "a + 1 > b" → Greater(Add(a, 1), b)
        var expr = Parse("a + 1 > b");
        var outer = Assert.IsType<BinaryExpr>(expr);
        Assert.Equal(BinaryOp.Greater, outer.Op);
        var left = Assert.IsType<BinaryExpr>(outer.Left);
        Assert.Equal(BinaryOp.Add, left.Op);
        Assert.Equal("a", Assert.IsType<VariableExpr>(left.Left).Name);
        Assert.Equal("1", Assert.IsType<LiteralExpr>(left.Right).RawText);
        Assert.Equal("b", Assert.IsType<VariableExpr>(outer.Right).Name);
    }

    // -----------------------------------------------------------------------
    // Assignment (Test 41)
    // -----------------------------------------------------------------------

    [Fact]
    public void Parse_GivenAssignment_ReturnsAssignExpr()
    {
        var expr = Parse("x = 5");
        var assign = Assert.IsType<AssignExpr>(expr);
        Assert.Equal("x", assign.Name);
        Assert.Equal("5", Assert.IsType<LiteralExpr>(assign.Value).RawText);
    }

    // -----------------------------------------------------------------------
    // Assignment — right-associative (Test 42)
    // -----------------------------------------------------------------------

    [Fact]
    public void Parse_GivenChainedAssignment_RightAssociative()
    {
        // "x = y = 5" → AssignExpr("x", AssignExpr("y", LiteralExpr("5")))
        var expr = Parse("x = y = 5");
        var outer = Assert.IsType<AssignExpr>(expr);
        Assert.Equal("x", outer.Name);
        var inner = Assert.IsType<AssignExpr>(outer.Value);
        Assert.Equal("y", inner.Name);
        Assert.Equal("5", Assert.IsType<LiteralExpr>(inner.Value).RawText);
    }

    // -----------------------------------------------------------------------
    // Error — empty input throws (Test 43)
    // -----------------------------------------------------------------------

    [Fact]
    public void Parse_GivenEmptyInput_ThrowsError()
    {
        var tokens = new Tokenizer().Tokenize("");
        var ex = Assert.Throws<InvalidOperationException>(() => new Parser().Parse(tokens));
        Assert.False(string.IsNullOrWhiteSpace(ex.Message));
    }

    // -----------------------------------------------------------------------
    // Error — unexpected token throws descriptive error (Test 44)
    // -----------------------------------------------------------------------

    [Fact]
    public void Parse_GivenUnexpectedToken_ThrowsDescriptiveError()
    {
        var tokens = new Tokenizer().Tokenize("+ +");
        var ex = Assert.Throws<InvalidOperationException>(() => new Parser().Parse(tokens));
        // The error message should mention the position or the unexpected token.
        Assert.False(string.IsNullOrWhiteSpace(ex.Message));
    }
}
