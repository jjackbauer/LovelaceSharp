using Lovelace.Console.Repl;

namespace Lovelace.Console.Tests;

/// <summary>
/// Tests for <see cref="Tokenizer.Tokenize"/> — operator and punctuation scanning,
/// whitespace skipping, and descriptive errors on unknown characters.
/// (Checklist item: "scan single-char operators … and two-char operators … before
/// single-char; skip whitespace; descriptive error on unknown characters")
/// </summary>
public class TokenizerOperatorTests
{
    private static List<Token> Tokenize(string input) => new Tokenizer().Tokenize(input);

    // -----------------------------------------------------------------------
    // Test 20 — single-char operators in one shot
    // -----------------------------------------------------------------------

    [Fact]
    public void Tokenize_GivenSingleCharOperators_ReturnsCorrectTokenKinds()
    {
        // "+-*/%^=(,)!" — every single-char operator/punctuation in sequence.
        // '!' is placed last to avoid being greedily consumed together with '='
        // as the two-char '!=' operator.
        var tokens = Tokenize("+-*/%^=(,)!");

        Assert.Equal(12, tokens.Count); // 11 operators + Eof
        Assert.Equal(new Token(TokenKind.Plus,    "+", 0),  tokens[0]);
        Assert.Equal(new Token(TokenKind.Minus,   "-", 1),  tokens[1]);
        Assert.Equal(new Token(TokenKind.Star,    "*", 2),  tokens[2]);
        Assert.Equal(new Token(TokenKind.Slash,   "/", 3),  tokens[3]);
        Assert.Equal(new Token(TokenKind.Percent, "%", 4),  tokens[4]);
        Assert.Equal(new Token(TokenKind.Caret,   "^", 5),  tokens[5]);
        Assert.Equal(new Token(TokenKind.Equals,  "=", 6),  tokens[6]);
        Assert.Equal(new Token(TokenKind.LParen,  "(", 7),  tokens[7]);
        Assert.Equal(new Token(TokenKind.Comma,   ",", 8),  tokens[8]);
        Assert.Equal(new Token(TokenKind.RParen,  ")", 9),  tokens[9]);
        Assert.Equal(new Token(TokenKind.Bang,    "!", 10), tokens[10]);
        Assert.Equal(TokenKind.Eof, tokens[11].Kind);
    }

    // -----------------------------------------------------------------------
    // Test 20 (companion) — Greater and Less single-char (not followed by =)
    // -----------------------------------------------------------------------

    [Fact]
    public void Tokenize_GivenGreaterAndLess_ReturnsCorrectTokenKinds()
    {
        var tokens = Tokenize("> <");

        Assert.Equal(3, tokens.Count); // Greater, Less, Eof
        Assert.Equal(new Token(TokenKind.Greater, ">", 0), tokens[0]);
        Assert.Equal(new Token(TokenKind.Less,    "<", 2), tokens[1]);
        Assert.Equal(TokenKind.Eof, tokens[2].Kind);
    }

    // -----------------------------------------------------------------------
    // Test 21 — two-char operators matched before single-char
    // -----------------------------------------------------------------------

    [Fact]
    public void Tokenize_GivenTwoCharOperators_ReturnsCorrectTokenKinds()
    {
        // "== != >= <=" — all four two-char operators separated by spaces
        var tokens = Tokenize("== != >= <=");

        Assert.Equal(5, tokens.Count); // 4 operators + Eof
        Assert.Equal(new Token(TokenKind.DoubleEquals,   "==", 0), tokens[0]);
        Assert.Equal(new Token(TokenKind.BangEquals,     "!=", 3), tokens[1]);
        Assert.Equal(new Token(TokenKind.GreaterEquals,  ">=", 6), tokens[2]);
        Assert.Equal(new Token(TokenKind.LessEquals,     "<=", 9), tokens[3]);
        Assert.Equal(TokenKind.Eof, tokens[4].Kind);
    }

    // -----------------------------------------------------------------------
    // Test 21 (companion) — two-char vs single-char disambiguation
    // "==" must not be tokenised as two Equals tokens
    // -----------------------------------------------------------------------

    [Fact]
    public void Tokenize_GivenDoubleEquals_ProducesOneDoubleEqualsToken()
    {
        var tokens = Tokenize("==");

        Assert.Equal(2, tokens.Count); // DoubleEquals + Eof
        Assert.Equal(new Token(TokenKind.DoubleEquals, "==", 0), tokens[0]);
        Assert.Equal(TokenKind.Eof, tokens[1].Kind);
    }

    // -----------------------------------------------------------------------
    // Test 22 — whitespace is skipped; positions reflect source offset
    // -----------------------------------------------------------------------

    [Fact]
    public void Tokenize_GivenWhitespace_SkipsWhitespacePreservesPositions()
    {
        // "1 + 2" — single space between each token
        var tokens = Tokenize("1 + 2");

        Assert.Equal(4, tokens.Count); // NumberLiteral, Plus, NumberLiteral, Eof
        Assert.Equal(new Token(TokenKind.NumberLiteral, "1", 0), tokens[0]);
        Assert.Equal(new Token(TokenKind.Plus,          "+", 2), tokens[1]);
        Assert.Equal(new Token(TokenKind.NumberLiteral, "2", 4), tokens[2]);
        Assert.Equal(TokenKind.Eof, tokens[3].Kind);
    }

    // -----------------------------------------------------------------------
    // Test 23 — unknown character throws a descriptive error
    // -----------------------------------------------------------------------

    [Fact]
    public void Tokenize_GivenUnknownChar_ThrowsDescriptiveError()
    {
        var ex = Assert.Throws<InvalidOperationException>(() => Tokenize("@"));

        // Message must identify both the bad character and its position
        Assert.Contains("@", ex.Message);
        Assert.Contains("0",  ex.Message);
    }

    [Fact]
    public void Tokenize_GivenUnknownCharMidInput_ThrowsWithCorrectPosition()
    {
        // "1 + @" — error at position 4
        var ex = Assert.Throws<InvalidOperationException>(() => Tokenize("1 + @"));

        Assert.Contains("@", ex.Message);
        Assert.Contains("4",  ex.Message);
    }

    // -----------------------------------------------------------------------
    // Test 24 — empty string produces only Eof
    // -----------------------------------------------------------------------

    [Fact]
    public void Tokenize_GivenEmptyString_ReturnsEofOnly()
    {
        var tokens = Tokenize("");

        Assert.Single(tokens);
        Assert.Equal(TokenKind.Eof, tokens[0].Kind);
    }

    // -----------------------------------------------------------------------
    // Test 25 — complex expression "a = 3.14 * b"
    // -----------------------------------------------------------------------

    [Fact]
    public void Tokenize_GivenComplexExpression_ReturnsCorrectSequence()
    {
        // "a = 3.14 * b"
        //  0   2  4      9  11
        var tokens = Tokenize("a = 3.14 * b");

        Assert.Equal(6, tokens.Count); // Identifier, Equals, NumberLiteral, Star, Identifier, Eof
        Assert.Equal(new Token(TokenKind.Identifier,    "a",    0),  tokens[0]);
        Assert.Equal(new Token(TokenKind.Equals,        "=",    2),  tokens[1]);
        Assert.Equal(new Token(TokenKind.NumberLiteral, "3.14", 4),  tokens[2]);
        Assert.Equal(new Token(TokenKind.Star,          "*",    9),  tokens[3]);
        Assert.Equal(new Token(TokenKind.Identifier,    "b",    11), tokens[4]);
        Assert.Equal(TokenKind.Eof, tokens[5].Kind);
    }
}
