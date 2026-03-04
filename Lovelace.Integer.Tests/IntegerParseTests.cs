using Lovelace.Integer;

namespace Lovelace.Integer.Tests;

/// <summary>
/// Functional tests for Integer.Parse and TryParse.
/// ChecklistItem: Parse / TryParse
/// </summary>
public class IntegerParseTests
{
    [Fact]
    public void Parse_GivenPositiveDigits_ReturnsCorrectInteger()
    {
        var result = Integer.Parse("42", null);
        Assert.Equal(new Integer(42L), result);
    }

    [Fact]
    public void Parse_GivenLeadingMinus_ReturnsNegativeInteger()
    {
        var result = Integer.Parse("-42", null);
        Assert.Equal(new Integer(-42L), result);
    }

    [Fact]
    public void Parse_GivenLeadingZeros_SkipsThem()
    {
        var result = Integer.Parse("007", null);
        Assert.Equal(new Integer(7L), result);
    }

    [Fact]
    public void Parse_GivenNegativeZero_ReturnsZeroPositive()
    {
        var result = Integer.Parse("-0", null);
        Assert.True(Integer.IsZero(result));
        Assert.False(Integer.IsNegative(result));
    }

    [Fact]
    public void Parse_GivenInvalidInput_ThrowsFormatException()
    {
        Assert.Throws<FormatException>(() => Integer.Parse("12a3", null));
    }

    [Fact]
    public void Parse_GivenEmptyString_ThrowsFormatException()
    {
        Assert.Throws<FormatException>(() => Integer.Parse("", null));
    }

    [Fact]
    public void TryParse_GivenValidInput_ReturnsTrueAndCorrectValue()
    {
        bool ok = Integer.TryParse("-99", null, out var result);
        Assert.True(ok);
        Assert.Equal(new Integer(-99L), result);
    }

    [Fact]
    public void TryParse_GivenInvalidInput_ReturnsFalseAndDefaultValue()
    {
        bool ok = Integer.TryParse("abc", null, out var result);
        Assert.False(ok);
    }

    [Fact]
    public void Parse_GivenSpan_ReturnsCorrectInteger()
    {
        ReadOnlySpan<char> span = "-7".AsSpan();
        var result = Integer.Parse(span, null);
        Assert.Equal(new Integer(-7L), result);
    }

    [Fact]
    public void Parse_GivenArbitraryPrecisionString_ParsesCorrectly()
    {
        const string bigNum = "123456789012345678901234567890";
        var result = Integer.Parse(bigNum, null);
        Assert.Equal(bigNum, result.ToString());
    }

    [Fact]
    public void Parse_GivenNegativeArbitraryPrecisionString_ParsesCorrectly()
    {
        const string bigNeg = "-123456789012345678901234567890";
        var result = Integer.Parse(bigNeg, null);
        Assert.Equal(bigNeg, result.ToString());
    }
}
