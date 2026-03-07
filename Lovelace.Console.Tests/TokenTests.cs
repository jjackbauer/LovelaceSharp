using Lovelace.Console.Repl;

namespace Lovelace.Console.Tests;

public class TokenTests
{
    // -----------------------------------------------------------------------
    // Token — property access
    // -----------------------------------------------------------------------

    [Fact]
    public void Token_GivenNumberLiteralKind_ReturnsCorrectKind()
    {
        var token = new Token(TokenKind.NumberLiteral, "42", 0);

        Assert.Equal(TokenKind.NumberLiteral, token.Kind);
    }

    [Fact]
    public void Token_GivenText_ReturnsCorrectText()
    {
        var token = new Token(TokenKind.Identifier, "myVar", 3);

        Assert.Equal("myVar", token.Text);
    }

    [Fact]
    public void Token_GivenPosition_ReturnsCorrectPosition()
    {
        var token = new Token(TokenKind.Plus, "+", 7);

        Assert.Equal(7, token.Position);
    }

    // -----------------------------------------------------------------------
    // Token — record structural equality
    // -----------------------------------------------------------------------

    [Fact]
    public void Token_GivenEqualTokens_AreStructurallyEqual()
    {
        var t1 = new Token(TokenKind.NumberLiteral, "42", 0);
        var t2 = new Token(TokenKind.NumberLiteral, "42", 0);

        Assert.Equal(t1, t2);
    }

    [Fact]
    public void Token_GivenDifferentKinds_AreNotEqual()
    {
        var t1 = new Token(TokenKind.NumberLiteral, "42", 0);
        var t2 = new Token(TokenKind.Identifier, "42", 0);

        Assert.NotEqual(t1, t2);
    }

    [Fact]
    public void Token_GivenDifferentTexts_AreNotEqual()
    {
        var t1 = new Token(TokenKind.Identifier, "a", 0);
        var t2 = new Token(TokenKind.Identifier, "b", 0);

        Assert.NotEqual(t1, t2);
    }

    [Fact]
    public void Token_GivenDifferentPositions_AreNotEqual()
    {
        var t1 = new Token(TokenKind.Plus, "+", 0);
        var t2 = new Token(TokenKind.Plus, "+", 5);

        Assert.NotEqual(t1, t2);
    }

    // -----------------------------------------------------------------------
    // TokenKind — all 20 values present and ordered
    // -----------------------------------------------------------------------

    [Theory]
    [InlineData(TokenKind.NumberLiteral,  0)]
    [InlineData(TokenKind.Identifier,     1)]
    [InlineData(TokenKind.Plus,           2)]
    [InlineData(TokenKind.Minus,          3)]
    [InlineData(TokenKind.Star,           4)]
    [InlineData(TokenKind.Slash,          5)]
    [InlineData(TokenKind.Percent,        6)]
    [InlineData(TokenKind.Caret,          7)]
    [InlineData(TokenKind.Bang,           8)]
    [InlineData(TokenKind.Equals,         9)]
    [InlineData(TokenKind.DoubleEquals,  10)]
    [InlineData(TokenKind.BangEquals,    11)]
    [InlineData(TokenKind.Greater,       12)]
    [InlineData(TokenKind.Less,          13)]
    [InlineData(TokenKind.GreaterEquals, 14)]
    [InlineData(TokenKind.LessEquals,    15)]
    [InlineData(TokenKind.LParen,        16)]
    [InlineData(TokenKind.RParen,        17)]
    [InlineData(TokenKind.Comma,         18)]
    [InlineData(TokenKind.Eof,           19)]
    public void TokenKind_GivenAllValues_HasExpectedOrdinal(TokenKind kind, int expectedOrdinal)
    {
        Assert.Equal(expectedOrdinal, (int)kind);
    }
}
