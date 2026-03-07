namespace Lovelace.Console.Repl;

/// <summary>
/// Recursive-descent parser that transforms a flat list of <see cref="Token"/>
/// values (produced by <see cref="Tokenizer.Tokenize"/>) into a typed expression
/// AST rooted at an <see cref="Expr"/> subtype.
/// </summary>
/// <remarks>
/// Precedence levels, lowest to highest:
/// <list type="number">
///   <item>Assignment         right-associative  <c>x = expr</c></item>
///   <item>Comparison         left-associative   <c>== != &gt; &lt; &gt;= &lt;=</c></item>
///   <item>Additive           left-associative   <c>+ -</c></item>
///   <item>Multiplicative     left-associative   <c>* / %</c></item>
///   <item>Power              right-associative  <c>^</c></item>
///   <item>Unary prefix       right-associative  <c>- +</c></item>
///   <item>Postfix            left-associative   <c>!</c></item>
///   <item>Primary            literals, variables, calls, grouped expressions</item>
/// </list>
/// </remarks>
public sealed class Parser
{
    private List<Token> _tokens = [];
    private int _pos;

    // ------------------------------------------------------------------
    // Public entry point
    // ------------------------------------------------------------------

    /// <summary>
    /// Parses <paramref name="tokens"/> into an expression tree.
    /// </summary>
    /// <param name="tokens">
    /// The complete token list produced by <see cref="Tokenizer.Tokenize"/>,
    /// including the final <see cref="TokenKind.Eof"/> sentinel.
    /// </param>
    /// <returns>The root <see cref="Expr"/> representing the input.</returns>
    /// <exception cref="InvalidOperationException">
    /// Thrown when the token stream does not form a valid expression.
    /// </exception>
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

    // ------------------------------------------------------------------
    // Precedence levels
    // ------------------------------------------------------------------

    // Level 1 — Assignment (right-associative)
    // identifier = expr   (only valid when identifier is followed by single '=')
    private Expr ParseAssignment()
    {
        // Lookahead: Identifier followed immediately by non-compound '='
        if (Current.Kind == TokenKind.Identifier &&
            Peek().Kind == TokenKind.Equals)
        {
            string name = Advance().Text; // consume identifier
            Advance();                    // consume '='
            Expr value = ParseAssignment(); // right-recursive
            return new AssignExpr(name, value);
        }

        return ParseComparison();
    }

    // Level 2 — Comparison (left-associative)
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

    // Level 3 — Additive (left-associative)
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

    // Level 4 — Multiplicative (left-associative)
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

    // Level 5 — Power (right-associative)
    private Expr ParsePower()
    {
        var left = ParseUnary();
        if (Current.Kind == TokenKind.Caret)
        {
            Advance(); // consume '^'
            var right = ParsePower(); // right-recursive for right-associativity
            return new BinaryExpr(left, BinaryOp.Power, right);
        }
        return left;
    }

    // Level 6 — Unary prefix (right-associative)
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

    // Level 7 — Postfix (left-associative; only '!' currently)
    private Expr ParsePostfix()
    {
        var operand = ParsePrimary();
        while (Current.Kind == TokenKind.Bang)
        {
            Advance(); // consume '!'
            operand = new PostfixExpr(operand, PostfixOp.Factorial);
        }
        return operand;
    }

    // Level 8 — Primary
    private Expr ParsePrimary()
    {
        // Number literal
        if (Current.Kind == TokenKind.NumberLiteral)
            return new LiteralExpr(Advance().Text);

        // Identifier: variable reference or function call
        if (Current.Kind == TokenKind.Identifier)
        {
            string name = Advance().Text;

            // Function call: identifier followed by '('
            if (Current.Kind == TokenKind.LParen)
            {
                Advance(); // consume '('
                var args = new List<Expr>();

                if (Current.Kind != TokenKind.RParen)
                {
                    args.Add(ParseAssignment());
                    while (Current.Kind == TokenKind.Comma)
                    {
                        Advance(); // consume ','
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
            Advance(); // consume '('
            var inner = ParseAssignment();
            Expect(TokenKind.RParen);
            return inner;
        }

        throw new InvalidOperationException(
            $"Unexpected token '{Current.Text}' at position {Current.Position}: " +
            "expected a number, identifier, or '('.");
    }
}
