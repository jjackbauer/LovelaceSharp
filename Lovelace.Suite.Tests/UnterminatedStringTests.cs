using Lovelace.Suite;
using Xunit;

namespace Lovelace.Suite.Tests;

/// <summary>
/// Malformed string literals are rejected (Cycle 2 round 30). Found by the adversarial audit's
/// fuzz case: a string running to end-of-input without its closing quote was silently accepted and
/// became a value, so a typo flowed through the pipeline as data.
/// </summary>
public class UnterminatedStringTests
{
    private static IReadOnlyList<Token> Tokens(string input) => new Tokenizer().Tokenize(input);

    [Fact]
    public void UnterminatedString_IsAParseError()
    {
        var ex = Assert.Throws<FormatException>(() => Tokens("\"abc"));
        Assert.Contains("Unterminated string literal", ex.Message);
        Assert.Contains("position 0", ex.Message);
    }

    [Fact]
    public void UnterminatedString_AtANonZeroPosition_ReportsThatPosition()
    {
        var ex = Assert.Throws<FormatException>(() => Tokens("x = 1; \"abc"));
        Assert.Contains("position 7", ex.Message);
    }

    [Fact]
    public void UnterminatedInterpolatedString_IsAParseError()
    {
        Assert.Throws<FormatException>(() => Tokens("$\"a{x}"));
    }

    [Fact]
    public void WellFormedStrings_StillTokenize()
    {
        // the token stream ends with an EOF token, so assert on the first token
        var plain = Tokens("\"abc\"")[0];
        Assert.Equal(TokenKind.StringLiteral, plain.Kind);
        Assert.Equal("abc", plain.Text);

        var interpolated = Tokens("$\"a{x}\"")[0];
        Assert.Equal(TokenKind.InterpolatedString, interpolated.Kind);
    }
}
