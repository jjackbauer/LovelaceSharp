using Lovelace.Console.Repl;

namespace Lovelace.Console.Tests;

/// <summary>
/// Tests for <see cref="Tokenizer.Tokenize"/> — identifier scanning
/// (checklist item: "scan identifiers matching [a-zA-Z_][a-zA-Z0-9_]*").
/// </summary>
public class TokenizerIdentifierTests
{
    private static List<Token> Tokenize(string input) => new Tokenizer().Tokenize(input);

    // -----------------------------------------------------------------------
    // Test 18 — plain alphabetic identifier
    // -----------------------------------------------------------------------

    [Fact]
    public void Tokenize_GivenIdentifier_ReturnsIdentifierToken()
    {
        var tokens = Tokenize("abc");

        Assert.Equal(2, tokens.Count);
        Assert.Equal(new Token(TokenKind.Identifier, "abc", 0), tokens[0]);
        Assert.Equal(TokenKind.Eof, tokens[1].Kind);
    }

    // -----------------------------------------------------------------------
    // Test 19 — identifier with underscore
    // -----------------------------------------------------------------------

    [Fact]
    public void Tokenize_GivenIdentifierWithUnderscore_ReturnsIdentifierToken()
    {
        var tokens = Tokenize("my_var");

        Assert.Equal(2, tokens.Count);
        Assert.Equal(new Token(TokenKind.Identifier, "my_var", 0), tokens[0]);
        Assert.Equal(TokenKind.Eof, tokens[1].Kind);
    }

    // -----------------------------------------------------------------------
    // Additional — leading-underscore identifier
    // -----------------------------------------------------------------------

    [Fact]
    public void Tokenize_GivenLeadingUnderscoreIdentifier_ReturnsIdentifierToken()
    {
        var tokens = Tokenize("_result");

        Assert.Equal(2, tokens.Count);
        Assert.Equal(new Token(TokenKind.Identifier, "_result", 0), tokens[0]);
        Assert.Equal(TokenKind.Eof, tokens[1].Kind);
    }

    // -----------------------------------------------------------------------
    // Additional — identifier with digits (not the first character)
    // -----------------------------------------------------------------------

    [Fact]
    public void Tokenize_GivenIdentifierWithDigitSuffix_ReturnsIdentifierToken()
    {
        var tokens = Tokenize("x1");

        Assert.Equal(2, tokens.Count);
        Assert.Equal(new Token(TokenKind.Identifier, "x1", 0), tokens[0]);
        Assert.Equal(TokenKind.Eof, tokens[1].Kind);
    }
}
