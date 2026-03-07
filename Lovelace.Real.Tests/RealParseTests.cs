using Lovelace.Real;
using Xunit;

namespace Lovelace.Real.Tests;

/// <summary>
/// Functional tests for <see cref="Real.Parse(string)"/>, <see cref="Real.TryParse(string, out Real)"/>,
/// and the span-based overloads (IParsable&lt;Real&gt;, ISpanParsable&lt;Real&gt;).
/// Corresponds to checklist item: static Real Parse(string s) + TryParse.
/// </summary>
public class RealParseTests
{
    // -----------------------------------------------------------------------
    // Parse — integer strings
    // -----------------------------------------------------------------------

    [Fact]
    public void Parse_GivenIntegerString_ExponentIsZero()
    {
        var r = Real.Parse("42");
        Assert.Equal(0L, r.Exponent);
        Assert.Equal("42", r.ToString());
    }

    [Fact]
    public void Parse_GivenNegativeIntegerString_IsNegativeAndCorrect()
    {
        var r = Real.Parse("-7");
        Assert.True(Real.IsNegative(r));
        Assert.Equal("-7", r.ToString());
    }

    // -----------------------------------------------------------------------
    // Parse — decimal strings
    // -----------------------------------------------------------------------

    [Fact]
    public void Parse_GivenDecimalString_SetsCorrectExponent()
    {
        var r = Real.Parse("3.14");
        Assert.Equal(-2L, r.Exponent);
    }

    [Fact]
    public void Parse_GivenDecimalString_ToStringRoundTrips()
    {
        var r = Real.Parse("3.14");
        Assert.Equal("3.14", r.ToString());
    }

    [Fact]
    public void Parse_GivenNegativeDecimalString_IsNegativeAndCorrect()
    {
        var r = Real.Parse("-0.001");
        Assert.True(Real.IsNegative(r));
        Assert.Equal(-3L, r.Exponent);
        Assert.Equal("-0.001", r.ToString());
    }

    [Fact]
    public void Parse_GivenLeadingZeros_StripsLeadingZeros()
    {
        // Leading zeros in the integer part are stripped; value is 7.5
        var r = Real.Parse("007.5");
        Assert.Equal("7.5", r.ToString());
    }

    [Fact]
    public void Parse_GivenTrailingZerosAfterDecimal_NormalizesThemAway()
    {
        // Trailing zeros after the decimal point are stripped by Normalize.
        // "1.50" normalises to "1.5" with Exponent == -1.
        // (Falsified: original assumption Exponent==-2 is incorrect; Normalize strips trailing zeros.)
        var r = Real.Parse("1.50");
        Assert.Equal(-1L, r.Exponent);
        Assert.Equal("1.5", r.ToString());
    }

    // -----------------------------------------------------------------------
    // TryParse — valid and invalid inputs
    // -----------------------------------------------------------------------

    [Fact]
    public void TryParse_GivenValidDecimalString_ReturnsTrueAndCorrectResult()
    {
        bool ok = Real.TryParse("3.14", out var r);
        Assert.True(ok);
        Assert.Equal("3.14", r.ToString());
    }

    [Fact]
    public void TryParse_GivenInvalidString_ReturnsFalse()
    {
        bool ok = Real.TryParse("abc", out _);
        Assert.False(ok);
    }

    [Fact]
    public void TryParse_GivenNullString_ReturnsFalse()
    {
        bool ok = Real.TryParse((string?)null, out _);
        Assert.False(ok);
    }

    [Fact]
    public void Parse_GivenStringWithSignOnly_ThrowsFormatException()
    {
        Assert.Throws<FormatException>(() => Real.Parse("-"));
    }

    [Fact]
    public void Parse_GivenEmptyString_ThrowsFormatException()
    {
        Assert.Throws<FormatException>(() => Real.Parse(""));
    }

    // -----------------------------------------------------------------------
    // Parse — periodic notation
    // -----------------------------------------------------------------------

    [Fact]
    public void Parse_GivenPeriodicNotation_SetsPeriodMetadata()
    {
        // "0.(3)" → purely periodic, PeriodStart=0, PeriodLength=1
        var r = Real.Parse("0.(3)");
        Assert.True(r.IsPeriodic);
        Assert.Equal(0L, r.PeriodStart);
        Assert.Equal(1L, r.PeriodLength);
        Assert.Equal("0.(3)", r.ToString());
    }

    [Fact]
    public void Parse_GivenPeriodicWithMixedPart_SetsCorrectPeriodStart()
    {
        // "0.1(6)" → one non-repeating fractional digit then period
        var r = Real.Parse("0.1(6)");
        Assert.Equal(1L, r.PeriodStart);
        Assert.Equal(1L, r.PeriodLength);
        Assert.Equal("0.1(6)", r.ToString());
    }

    [Fact]
    public void Parse_GivenNegativePeriodicNotation_IsNegativeAndPeriodic()
    {
        var r = Real.Parse("-0.(142857)");
        Assert.True(Real.IsNegative(r));
        Assert.True(r.IsPeriodic);
        Assert.Equal("-0.(142857)", r.ToString());
    }

    [Fact]
    public void TryParse_GivenPeriodicNotation_ReturnsTrueAndPeriodicResult()
    {
        bool ok = Real.TryParse("0.(3)", out var r);
        Assert.True(ok);
        Assert.True(r!.IsPeriodic);
    }

    [Fact]
    public void Parse_GivenMalformedPeriodicNotation_UnclosedParen_ThrowsFormatException()
    {
        // Unclosed parenthesis is invalid
        Assert.Throws<FormatException>(() => Real.Parse("0.(3"));
    }

    [Fact]
    public void Parse_GivenMalformedPeriodicNotation_EmptyParen_ThrowsFormatException()
    {
        // Empty period block is invalid: "()" has no repeating digits
        Assert.Throws<FormatException>(() => Real.Parse("0.()"));
    }

    // -----------------------------------------------------------------------
    // Span-based overloads
    // -----------------------------------------------------------------------

    [Fact]
    public void TryParse_GivenValidSpan_ReturnsTrueAndCorrectResult()
    {
        bool ok = Real.TryParse("2.71828".AsSpan(), null, out var r);
        Assert.True(ok);
        Assert.Equal("2.71828", r!.ToString());
    }

    [Fact]
    public void TryParse_GivenInvalidSpan_ReturnsFalse()
    {
        bool ok = Real.TryParse("not-a-number".AsSpan(), null, out _);
        Assert.False(ok);
    }

    [Fact]
    public void Parse_GivenSpanDecimal_SetsCorrectExponent()
    {
        var r = Real.Parse("9.99".AsSpan(), null);
        Assert.Equal(-2L, r.Exponent);
        Assert.Equal("9.99", r.ToString());
    }
}
