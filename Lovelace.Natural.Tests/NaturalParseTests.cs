using Lovelace.Natural;

namespace Lovelace.Natural.Tests;

public class NaturalParseTests
{
    // -------------------------------------------------------------------------
    // Parse — valid inputs
    // -------------------------------------------------------------------------

    [Fact]
    public void Parse_GivenValidDecimalString_ReturnsCorrectNatural()
    {
        var expected = new Natural(12345UL);
        var actual = Natural.Parse("12345");
        Assert.Equal(expected, actual);
    }

    [Fact]
    public void Parse_GivenZeroString_ReturnsZero()
    {
        var actual = Natural.Parse("0");
        Assert.True(Natural.IsZero(actual));
    }

    [Fact]
    public void Parse_GivenLeadingZeros_TrimsToCanonicalForm()
    {
        var expected = new Natural(7UL);
        var actual = Natural.Parse("007");
        Assert.Equal(expected, actual);
    }

    [Fact]
    public void Parse_GivenSingleDigit_ReturnsCorrectNatural()
    {
        var expected = new Natural(9UL);
        var actual = Natural.Parse("9");
        Assert.Equal(expected, actual);
    }

    [Fact]
    public void Parse_GivenLargeNumber_ReturnsCorrectNatural()
    {
        // 18446744073709551615 == ulong.MaxValue
        var expected = new Natural(ulong.MaxValue);
        var actual = Natural.Parse("18446744073709551615");
        Assert.Equal(expected, actual);
    }

    [Fact]
    public void Parse_GivenNumberBeyondUlongMax_ReturnsCorrectNatural()
    {
        // ulong.MaxValue + 1 = 18446744073709551616
        var expectedStr = "18446744073709551616";
        var actual = Natural.Parse(expectedStr);
        Assert.Equal(expectedStr, actual.ToString());
    }

    // -------------------------------------------------------------------------
    // Parse — invalid inputs
    // -------------------------------------------------------------------------

    [Fact]
    public void Parse_GivenEmptyString_ThrowsFormatException()
    {
        Assert.Throws<FormatException>(() => Natural.Parse(""));
    }

    [Fact]
    public void Parse_GivenNonDigitCharacters_ThrowsFormatException()
    {
        Assert.Throws<FormatException>(() => Natural.Parse("12a3"));
    }

    [Fact]
    public void Parse_GivenNullString_ThrowsFormatException()
    {
        Assert.Throws<FormatException>(() => Natural.Parse(null!));
    }

    [Fact]
    public void Parse_GivenWhitespaceString_ThrowsFormatException()
    {
        Assert.Throws<FormatException>(() => Natural.Parse("   "));
    }

    // -------------------------------------------------------------------------
    // TryParse — valid inputs
    // -------------------------------------------------------------------------

    [Fact]
    public void TryParse_GivenValidString_ReturnsTrueAndCorrectValue()
    {
        bool success = Natural.TryParse("999", out var result);
        Assert.True(success);
        Assert.Equal(new Natural(999UL), result);
    }

    [Fact]
    public void TryParse_GivenZeroString_ReturnsTrueAndZero()
    {
        bool success = Natural.TryParse("0", out var result);
        Assert.True(success);
        Assert.NotNull(result);
        Assert.True(Natural.IsZero(result));
    }

    // -------------------------------------------------------------------------
    // TryParse — invalid inputs
    // -------------------------------------------------------------------------

    [Fact]
    public void TryParse_GivenInvalidString_ReturnsFalse()
    {
        bool success = Natural.TryParse("abc", out _);
        Assert.False(success);
    }

    [Fact]
    public void TryParse_GivenNullString_ReturnsFalse()
    {
        bool success = Natural.TryParse((string?)null, out _);
        Assert.False(success);
    }

    [Fact]
    public void TryParse_GivenEmptyString_ReturnsFalse()
    {
        bool success = Natural.TryParse("", out _);
        Assert.False(success);
    }

    // -------------------------------------------------------------------------
    // Round-trip
    // -------------------------------------------------------------------------

    [Theory]
    [InlineData("1")]
    [InlineData("42")]
    [InlineData("12345")]
    [InlineData("9999999999999999999")]
    [InlineData("18446744073709551615")] // ulong.MaxValue
    public void Parse_GivenRoundTrip_ReturnsOriginalValue(string original)
    {
        var n = Natural.Parse(original);
        Assert.Equal(original, n.ToString());
    }
}
