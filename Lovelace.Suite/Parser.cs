namespace Lovelace.Suite;

/// <summary>
/// Recursive-descent parser. Transforms a token list into either a single
/// <see cref="Expr"/> (backward-compatible with the REPL's expression parser) or
/// a <see cref="Program"/> of statements (the script language).
/// </summary>
/// <remarks>
/// Expression precedence, lowest to highest:
/// assignment → range (<c>..</c>) → comparison → additive → multiplicative →
/// power → unary prefix → postfix (factorial/index) → primary.
/// </remarks>
public sealed class Parser
{
    private List<Token> _tokens = [];
    private int _pos;

    // ------------------------------------------------------------------
    // Public entry points
    // ------------------------------------------------------------------

    /// <summary>
    /// Parses <paramref name="tokens"/> into a single expression. Throws if the
    /// token stream does not form exactly one expression.
    /// </summary>
    public Expr Parse(List<Token> tokens)
    {
        _tokens = tokens;
        _pos = 0;

        if (Current.Kind == TokenKind.Eof)
            throw new InvalidOperationException(
                "Unexpected end of input: expected an expression.");

        var expr = ParseAssignment();

        if (Current.Kind != TokenKind.Eof)
            throw new InvalidOperationException(
                $"Unexpected token '{Current.Text}' at position {Current.Position}.");

        return expr;
    }

    /// <summary>
    /// Parses <paramref name="tokens"/> into a <see cref="Program"/> of statements.
    /// Statements are separated by <c>;</c> (a trailing separator is optional).
    /// </summary>
    public Program ParseProgram(List<Token> tokens)
    {
        _tokens = tokens;
        _pos = 0;

        var statements = new List<Statement>();

        while (Current.Kind != TokenKind.Eof)
        {
            statements.Add(ParseStatement());

            if (Current.Kind == TokenKind.Semicolon)
            {
                Advance();
            }
            else if (Current.Kind != TokenKind.Eof && Current.Kind != TokenKind.RBrace)
            {
                throw new InvalidOperationException(
                    $"Expected ';' or end of input but found '{Current.Text}' at position {Current.Position}.");
            }
        }

        return new Program(statements);
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    private Token Current => _tokens[_pos];

    private Token Peek(int offset = 1)
    {
        int idx = _pos + offset;
        return idx < _tokens.Count ? _tokens[idx] : _tokens[^1];
    }

    private Token Advance()
    {
        var t = Current;
        if (t.Kind != TokenKind.Eof) _pos++;
        return t;
    }

    private Token Expect(TokenKind kind)
    {
        if (Current.Kind != kind)
            throw new InvalidOperationException(
                $"Expected '{kind}' but found '{Current.Text}' at position {Current.Position}.");
        return Advance();
    }

    private bool MatchIdentifier(string text) =>
        Current.Kind == TokenKind.Identifier && Current.Text == text;

    // ------------------------------------------------------------------
    // Statement level
    // ------------------------------------------------------------------

    private Statement ParseStatement()
    {
        if (Current.Kind == TokenKind.LBrace)
            return ParseBlock();

        if (Current.Kind == TokenKind.Identifier)
        {
            switch (Current.Text)
            {
                case "func": return ParseFunction();
                case "if": return ParseIf();
                case "while": return ParseWhile();
                case "for": return ParseFor();
                case "return": return ParseReturn();
                case "break":
                    Advance();
                    return new BreakStatement();
                case "continue":
                    Advance();
                    return new ContinueStatement();
            }
        }

        return new ExpressionStatement(ParseAssignment());
    }

    private Statement ParseBlock()
    {
        Expect(TokenKind.LBrace);
        var statements = new List<Statement>();

        while (Current.Kind != TokenKind.RBrace && Current.Kind != TokenKind.Eof)
        {
            statements.Add(ParseStatement());
            if (Current.Kind == TokenKind.Semicolon)
            {
                Advance();
            }
            else if (Current.Kind != TokenKind.RBrace)
            {
                throw new InvalidOperationException(
                    $"Expected ';' or '}}' but found '{Current.Text}' at position {Current.Position}.");
            }
        }

        Expect(TokenKind.RBrace);
        return new BlockStatement(statements);
    }

    private Statement ParseFunction()
    {
        int startPos = Current.Position;
        Advance(); // consume 'func'

        if (Current.Kind != TokenKind.Identifier)
            throw new InvalidOperationException(
                $"Expected function name but found '{Current.Text}' at position {Current.Position}.");

        string name = Advance().Text;

        Expect(TokenKind.LParen);
        var parameters = new List<string>();
        if (Current.Kind != TokenKind.RParen)
        {
            parameters.Add(Expect(TokenKind.Identifier).Text);
            while (Current.Kind == TokenKind.Comma)
            {
                Advance();
                parameters.Add(Expect(TokenKind.Identifier).Text);
            }
        }
        Expect(TokenKind.RParen);

        IReadOnlyList<Statement> body;
        int endPos;

        if (Current.Kind == TokenKind.LBrace)
        {
            var block = (BlockStatement)ParseBlock();
            body = block.Statements;
            endPos = Current.Position;
        }
        else if (Current.Kind == TokenKind.Equals)
        {
            Advance(); // consume '='
            var expr = ParseAssignment();
            body = [new ExpressionStatement(expr)];
            endPos = Current.Position;
        }
        else
        {
            throw new InvalidOperationException(
                $"Expected '{{' or '=' after function signature but found '{Current.Text}' at position {Current.Position}.");
        }

        var definition = new FunctionDefinition(name, parameters, body, new SourceSpan(1, startPos + 1, 1, endPos + 1));
        return new FunctionStatement(definition);
    }

    private Statement ParseIf()
    {
        Advance(); // consume 'if'
        Expect(TokenKind.LParen);
        var condition = ParseAssignment();
        Expect(TokenKind.RParen);

        var then = ParseStatement();

        Statement? otherwise = null;
        if (MatchIdentifier("else"))
        {
            Advance(); // consume 'else'
            otherwise = ParseStatement();
        }

        return new IfStatement(condition, then, otherwise);
    }

    private Statement ParseWhile()
    {
        Advance(); // consume 'while'
        Expect(TokenKind.LParen);
        var condition = ParseAssignment();
        Expect(TokenKind.RParen);

        var body = ParseStatement();
        return new WhileStatement(condition, body);
    }

    private Statement ParseFor()
    {
        Advance(); // consume 'for'
        string variable = Expect(TokenKind.Identifier).Text;

        if (!MatchIdentifier("in"))
            throw new InvalidOperationException(
                $"Expected 'in' after loop variable but found '{Current.Text}' at position {Current.Position}.");
        Advance(); // consume 'in'

        var range = ParseAssignment();
        var body = ParseStatement();
        return new ForStatement(variable, range, body);
    }

    private Statement ParseReturn()
    {
        Advance(); // consume 'return'

        Expr? value = null;
        if (Current.Kind is not (TokenKind.Semicolon or TokenKind.RBrace or TokenKind.Eof))
            value = ParseAssignment();

        return new ReturnStatement(value);
    }

    // ------------------------------------------------------------------
    // Expression precedence levels
    // ------------------------------------------------------------------

    // Assignment (right-associative)
    private Expr ParseAssignment()
    {
        if (Current.Kind == TokenKind.Identifier && Peek().Kind == TokenKind.Equals)
        {
            string name = Advance().Text;
            Advance(); // consume '='
            Expr value = ParseAssignment();
            return new AssignExpr(name, value);
        }

        return ParseRange();
    }

    // Range — start..end and start..step..end (binds looser than comparison)
    private Expr ParseRange()
    {
        var first = ParseComparison();

        if (Current.Kind != TokenKind.DotDot)
            return first;

        Advance(); // consume first '..'
        var second = ParseComparison();

        if (Current.Kind == TokenKind.DotDot)
        {
            Advance(); // consume second '..'
            var third = ParseComparison();
            return new RangeExpr(first, second, third);
        }

        return new RangeExpr(first, null, second);
    }

    // Comparison (left-associative)
    private Expr ParseComparison()
    {
        var left = ParseAdditive();

        while (true)
        {
            BinaryOp? op = Current.Kind switch
            {
                TokenKind.DoubleEquals   => BinaryOp.Equal,
                TokenKind.BangEquals     => BinaryOp.NotEqual,
                TokenKind.Greater        => BinaryOp.Greater,
                TokenKind.Less           => BinaryOp.Less,
                TokenKind.GreaterEquals  => BinaryOp.GreaterEqual,
                TokenKind.LessEquals     => BinaryOp.LessEqual,
                _                        => null,
            };
            if (op is null) break;
            Advance();
            var right = ParseAdditive();
            left = new BinaryExpr(left, op.Value, right);
        }

        return left;
    }

    // Additive (left-associative)
    private Expr ParseAdditive()
    {
        var left = ParseMultiplicative();

        while (true)
        {
            BinaryOp? op = Current.Kind switch
            {
                TokenKind.Plus  => BinaryOp.Add,
                TokenKind.Minus => BinaryOp.Subtract,
                _               => null,
            };
            if (op is null) break;
            Advance();
            var right = ParseMultiplicative();
            left = new BinaryExpr(left, op.Value, right);
        }

        return left;
    }

    // Multiplicative (left-associative)
    private Expr ParseMultiplicative()
    {
        var left = ParsePower();

        while (true)
        {
            BinaryOp? op = Current.Kind switch
            {
                TokenKind.Star    => BinaryOp.Multiply,
                TokenKind.Slash   => BinaryOp.Divide,
                TokenKind.Percent => BinaryOp.Modulo,
                _                 => null,
            };
            if (op is null) break;
            Advance();
            var right = ParsePower();
            left = new BinaryExpr(left, op.Value, right);
        }

        return left;
    }

    // Power (right-associative)
    private Expr ParsePower()
    {
        var left = ParseUnary();
        if (Current.Kind == TokenKind.Caret)
        {
            Advance();
            var right = ParsePower();
            return new BinaryExpr(left, BinaryOp.Power, right);
        }
        return left;
    }

    // Unary prefix (right-associative)
    private Expr ParseUnary()
    {
        if (Current.Kind == TokenKind.Minus)
        {
            Advance();
            return new UnaryExpr(UnaryOp.Negate, ParseUnary());
        }
        if (Current.Kind == TokenKind.Plus)
        {
            Advance();
            return new UnaryExpr(UnaryOp.Plus, ParseUnary());
        }
        return ParsePostfix();
    }

    // Postfix (left-associative: factorial '!' and index '[...]')
    private Expr ParsePostfix()
    {
        var operand = ParsePrimary();

        while (true)
        {
            if (Current.Kind == TokenKind.Bang)
            {
                Advance();
                operand = new PostfixExpr(operand, PostfixOp.Factorial);
                continue;
            }

            if (Current.Kind == TokenKind.LBracket)
            {
                Advance(); // consume '['
                var index = ParseAssignment();
                Expect(TokenKind.RBracket);
                operand = new IndexExpr(operand, index);
                continue;
            }

            break;
        }

        return operand;
    }

    // Primary
    private Expr ParsePrimary()
    {
        // Number literal
        if (Current.Kind == TokenKind.NumberLiteral)
            return new LiteralExpr(Advance().Text);

        // Plain string literal
        if (Current.Kind == TokenKind.StringLiteral)
            return new StringExpr(Advance().Text);

        // Interpolated string
        if (Current.Kind == TokenKind.InterpolatedString)
            return ParseInterpolatedString(Advance().Text);

        // List literal [a, b, c]
        if (Current.Kind == TokenKind.LBracket)
        {
            Advance(); // consume '['
            var elements = new List<Expr>();

            if (Current.Kind != TokenKind.RBracket)
            {
                elements.Add(ParseAssignment());
                while (Current.Kind == TokenKind.Comma)
                {
                    Advance();
                    elements.Add(ParseAssignment());
                }
            }

            Expect(TokenKind.RBracket);
            return new ListExpr(elements);
        }

        // Identifier: variable reference or function call
        if (Current.Kind == TokenKind.Identifier)
        {
            string name = Advance().Text;

            if (Current.Kind == TokenKind.LParen)
            {
                Advance(); // consume '('
                var args = new List<Expr>();

                if (Current.Kind != TokenKind.RParen)
                {
                    args.Add(ParseAssignment());
                    while (Current.Kind == TokenKind.Comma)
                    {
                        Advance();
                        args.Add(ParseAssignment());
                    }
                }

                Expect(TokenKind.RParen);
                return new CallExpr(name, args);
            }

            return new VariableExpr(name);
        }

        // Parenthesized group
        if (Current.Kind == TokenKind.LParen)
        {
            Advance();
            var inner = ParseAssignment();
            Expect(TokenKind.RParen);
            return inner;
        }

        throw new InvalidOperationException(
            $"Unexpected token '{Current.Text}' at position {Current.Position}: " +
            "expected a number, string, identifier, '[', or '('.");
    }

    // ------------------------------------------------------------------
    // Interpolated strings
    // ------------------------------------------------------------------

    private InterpolatedStringExpr ParseInterpolatedString(string raw)
    {
        var parts = new List<InterpolationPart>();
        var sb = new System.Text.StringBuilder();
        int i = 0;

        while (i < raw.Length)
        {
            char c = raw[i];

            // Escaped braces: {{ and }}
            if (c == '{' && i + 1 < raw.Length && raw[i + 1] == '{')
            {
                sb.Append('{');
                i += 2;
                continue;
            }
            if (c == '}' && i + 1 < raw.Length && raw[i + 1] == '}')
            {
                sb.Append('}');
                i += 2;
                continue;
            }

            // Embedded expression { ... }
            if (c == '{')
            {
                int close = raw.IndexOf('}', i + 1);
                if (close < 0)
                    throw new InvalidOperationException("Unterminated interpolation expression in string.");

                if (sb.Length > 0)
                {
                    parts.Add(new TextPart(sb.ToString()));
                    sb.Clear();
                }

                string exprText = raw[(i + 1)..close];
                parts.Add(new ExpressionPart(ParseSubExpression(exprText)));
                i = close + 1;
                continue;
            }

            sb.Append(c);
            i++;
        }

        if (sb.Length > 0)
            parts.Add(new TextPart(sb.ToString()));

        return new InterpolatedStringExpr(parts);
    }

    private static Expr ParseSubExpression(string text)
    {
        var tokens = new Tokenizer().Tokenize(text);
        return new Parser().Parse(tokens);
    }
}
