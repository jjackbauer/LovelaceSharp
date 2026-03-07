using Lovelace.Console.Repl;

namespace Lovelace.Console.Tests;

/// <summary>
/// Tests for <see cref="Tokenizer.Tokenize"/> — number literal scanning
/// (checklist item: "scan number literals: digits, optional dot, optional digits,
/// optional (digits) for periodic notation").
/// </summary>
public class TokenizerNumberTests
{
    private static List<Token> Tokenize(string input) => new Tokenizer().Tokenize(input);

    // -----------------------------------------------------------------------
    // Test 13 — integer literal
    // -----------------------------------------------------------------------

    [Fact]
    public void Tokenize_GivenIntegerLiteral_ReturnsSingleNumberToken()
    {
        var tokens = Tokenize("42");

        Assert.Equal(2, tokens.Count);
        Assert.Equal(new Token(TokenKind.NumberLiteral, "42", 0), tokens[0]);
        Assert.Equal(TokenKind.Eof, tokens[1].Kind);
    }

    // -----------------------------------------------------------------------
    // Test 14 — decimal literal
    // -----------------------------------------------------------------------

    [Fact]
    public void Tokenize_GivenDecimalLiteral_ReturnsSingleNumberToken()
    {
        var tokens = Tokenize("3.14");

        Assert.Equal(2, tokens.Count);
        Assert.Equal(new Token(TokenKind.NumberLiteral, "3.14", 0), tokens[0]);
        Assert.Equal(TokenKind.Eof, tokens[1].Kind);
    }

    // -----------------------------------------------------------------------
    // Test 15 — periodic literal: "0.(3)"
    // -----------------------------------------------------------------------

    [Fact]
    public void Tokenize_GivenPeriodicLiteral_ReturnsSingleNumberToken()
    {
        var tokens = Tokenize("0.(3)");

        Assert.Equal(2, tokens.Count);
        Assert.Equal(new Token(TokenKind.NumberLiteral, "0.(3)", 0), tokens[0]);
        Assert.Equal(TokenKind.Eof, tokens[1].Kind);
    }

    // -----------------------------------------------------------------------
    // Test 16 — leading-dot decimal: ".5"
    // -----------------------------------------------------------------------

    [Fact]
    public void Tokenize_GivenLeadingDot_ReturnsSingleNumberToken()
    {
        var tokens = Tokenize(".5");

        Assert.Equal(2, tokens.Count);
        Assert.Equal(new Token(TokenKind.NumberLiteral, ".5", 0), tokens[0]);
        Assert.Equal(TokenKind.Eof, tokens[1].Kind);
    }

    // -----------------------------------------------------------------------
    // Test 17 — periodic with multiple digits: "1.2(345)"
    // -----------------------------------------------------------------------

    [Fact]
    public void Tokenize_GivenPeriodicWithMultipleDigits_ReturnsSingleNumberToken()
    {
        var tokens = Tokenize("1.2(345)");

        Assert.Equal(2, tokens.Count);
        Assert.Equal(new Token(TokenKind.NumberLiteral, "1.2(345)", 0), tokens[0]);
        Assert.Equal(TokenKind.Eof, tokens[1].Kind);
    }

    // -----------------------------------------------------------------------
    // Test 24 (partial) — empty string → Eof only
    // -----------------------------------------------------------------------

    [Fact]
    public void Tokenize_GivenEmptyString_ReturnsEofOnly()
    {
        var tokens = Tokenize(string.Empty);

        Assert.Single(tokens);
        Assert.Equal(TokenKind.Eof, tokens[0].Kind);
        Assert.Equal(string.Empty, tokens[0].Text);
        Assert.Equal(0, tokens[0].Position);
    }

    // -----------------------------------------------------------------------
    // Position accuracy — number at offset
    // -----------------------------------------------------------------------

    [Fact]
    public void Tokenize_GivenNumberAfterWhitespace_ReportsCorrectPosition()
    {
        // "  42" — number starts at position 2 (after two spaces)
        var tokens = Tokenize("  42");

        Assert.Equal(2, tokens.Count);
        Assert.Equal(new Token(TokenKind.NumberLiteral, "42", 2), tokens[0]);
    }
}
