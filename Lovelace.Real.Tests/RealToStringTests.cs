using Lovelace.Real;

namespace Lovelace.Real.Tests;

/// <summary>
/// Functional tests for <see cref="Real.ToString()"/>,
/// <see cref="Real.ToString(string?, IFormatProvider?)"/>, and
/// <see cref="Real.TryFormat"/>.
///
/// Covers checklist item:
///   - string ToString() + ToString(string?, IFormatProvider?) + TryFormat(...)
///     non-periodic: emit up to DisplayDecimalPlaces fractional digits;
///     periodic: emit non-repeating part then (repeating_block) with no truncation.
/// </summary>
public class RealToStringTests
{
    // -------------------------------------------------------------------------
    // ToString — non-periodic, integer exponent
    // -------------------------------------------------------------------------

    [Fact]
    public void ToString_GivenIntegerExponent_EmitsNoDecimalPoint()
    {
        // Real("42") has Exponent=0, digits="42" → no decimal point.
        var r = new Real("42");
        Assert.Equal("42", r.ToString());
    }

    // -------------------------------------------------------------------------
    // ToString — non-periodic, negative exponent (fractional digits)
    // -------------------------------------------------------------------------

    [Fact]
    public void ToString_GivenNegativeExponent_InsertsDecimalPoint()
    {
        // Exponent=-2, raw digits="314" → "3.14".
        var r = new Real("3.14");
        Assert.Equal("3.14", r.ToString());
    }

    [Fact]
    public void ToString_GivenExponentLargerThanDigitCount_PrependsLeadingZero()
    {
        // Exponent=-4, raw digits="5" → "0.0005" (three leading-zero fractional digits).
        var r = new Real("0.0005");
        Assert.Equal("0.0005", r.ToString());
    }

    // -------------------------------------------------------------------------
    // ToString — sign handling
    // -------------------------------------------------------------------------

    [Fact]
    public void ToString_GivenNegativeValue_IncludesMinusSign()
    {
        // new Real("-1.5") must produce a string starting with '-'.
        var r = new Real("-1.5");
        Assert.StartsWith("-", r.ToString());
        Assert.Equal("-1.5", r.ToString());
    }

    // -------------------------------------------------------------------------
    // ToString — zero
    // -------------------------------------------------------------------------

    [Fact]
    public void ToString_GivenZero_ReturnsZeroString()
    {
        // new Real(0.0) must produce "0".
        var r = new Real(0.0);
        Assert.Equal("0", r.ToString());
    }

    // -------------------------------------------------------------------------
    // TryFormat — sufficient buffer
    // -------------------------------------------------------------------------

    [Fact]
    public void TryFormat_GivenSufficientBuffer_ReturnsTrueAndWritesCorrectly()
    {
        var r = new Real("7.25");
        string expected = r.ToString();
        Span<char> buf = stackalloc char[expected.Length + 10];

        bool ok = r.TryFormat(buf, out int written, default, null);

        Assert.True(ok);
        Assert.Equal(expected.Length, written);
        Assert.Equal(expected, new string(buf[..written]));
    }

    // -------------------------------------------------------------------------
    // TryFormat — insufficient buffer
    // -------------------------------------------------------------------------

    [Fact]
    public void TryFormat_GivenInsufficientBuffer_ReturnsFalse()
    {
        var r = new Real("3.14");
        Span<char> empty = stackalloc char[0];

        bool ok = r.TryFormat(empty, out int written, default, null);

        Assert.False(ok);
        Assert.Equal(0, written);
    }

    // -------------------------------------------------------------------------
    // ToString — purely periodic value
    // -------------------------------------------------------------------------

    [Fact]
    public void ToString_GivenPurelyPeriodicValue_EmitsParentheses()
    {
        // 0.(3) — period starts immediately after the decimal point.
        var r = Real.Parse("0.(3)");
        Assert.Equal("0.(3)", r.ToString());
    }

    // -------------------------------------------------------------------------
    // ToString — mixed (non-repeating prefix + period)
    // -------------------------------------------------------------------------

    [Fact]
    public void ToString_GivenMixedPeriodicValue_EmitsCorrectNotation()
    {
        // 0.1(6) — one non-repeating fractional digit then the period block.
        var r = Real.Parse("0.1(6)");
        Assert.Equal("0.1(6)", r.ToString());
    }

    // -------------------------------------------------------------------------
    // ToString — DisplayDecimalPlaces truncation for non-periodic values
    // -------------------------------------------------------------------------

    [Fact]
    public void ToString_GivenNonPeriodicValue_TruncatesAtDisplayDecimalPlaces()
    {
        // "1.123456789" has 9 fractional digits; with DisplayDecimalPlaces=5 only 5 are emitted.
        long saved = Real.DisplayDecimalPlaces;
        try
        {
            Real.DisplayDecimalPlaces = 5;
            var r = new Real("1.123456789");
            string result = r.ToString();
            // Must not emit more than 5 fractional digits.
            int dotIdx = result.IndexOf('.');
            Assert.True(dotIdx >= 0, "Expected a decimal point in result.");
            int fracDigits = result.Length - dotIdx - 1;
            Assert.True(fracDigits <= 5, $"Expected at most 5 fractional digits but got {fracDigits}: '{result}'");
            Assert.Equal("1.12345", result);
        }
        finally
        {
            Real.DisplayDecimalPlaces = saved;
        }
    }

    // -------------------------------------------------------------------------
    // ToString — negative periodic value
    // -------------------------------------------------------------------------

    [Fact]
    public void ToString_GivenNegativePeriodicValue_IncludesSign()
    {
        // -0.(142857) must keep the minus prefix and the period notation unchanged.
        var r = Real.Parse("-0.(142857)");
        Assert.Equal("-0.(142857)", r.ToString());
    }

    // -------------------------------------------------------------------------
    // ToString(string?, IFormatProvider?) — delegates to ToString()
    // -------------------------------------------------------------------------

    [Fact]
    public void ToString_WithFormatAndProvider_DelegatesToParamlessToString()
    {
        var r = new Real("2.71828");
        // The overload must return the same value as the parameterless version.
        Assert.Equal(r.ToString(), r.ToString(null, null));
    }
}
