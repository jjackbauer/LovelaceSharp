using Lovelace.Natural;

namespace Lovelace.Natural.Tests;

/// <summary>
/// Functional tests for Natural constructors:
/// Natural(), Natural(ulong), Natural(int), Natural(Natural).
/// Checklist item: "Constructors: Natural(), Natural(ulong), Natural(int), copy ctor Natural(Natural)".
/// </summary>
public class NaturalConstructorTests
{
    // -------------------------------------------------------------------------
    // Natural() — default constructor
    // -------------------------------------------------------------------------

    [Fact]
    public void Constructor_GivenDefault_ProducesZero()
    {
        var n = new Natural();
        Assert.True(Natural.IsZero(n));
    }

    // -------------------------------------------------------------------------
    // Natural(ulong) — constructor
    // -------------------------------------------------------------------------

    [Fact]
    public void Constructor_GivenUlongZero_ProducesZero()
    {
        var n = new Natural(0UL);
        Assert.True(Natural.IsZero(n));
    }

    [Fact]
    public void Constructor_GivenUlong_StoresCorrectDigitSequence()
    {
        var n = new Natural(12345UL);
        Assert.Equal("12345", n.ToString());
    }

    [Fact]
    public void Constructor_GivenUlongMaxValue_StoresAllDigits()
    {
        var n = new Natural(ulong.MaxValue);
        Assert.Equal("18446744073709551615", n.ToString());
    }

    [Theory]
    [InlineData(1UL, "1")]
    [InlineData(9UL, "9")]
    [InlineData(10UL, "10")]
    [InlineData(100UL, "100")]
    [InlineData(999UL, "999")]
    [InlineData(1000UL, "1000")]
    public void Constructor_GivenUlong_ToStringRoundTrips(ulong value, string expected)
    {
        var n = new Natural(value);
        Assert.Equal(expected, n.ToString());
    }

    // -------------------------------------------------------------------------
    // Natural(int) — constructor
    // -------------------------------------------------------------------------

    [Fact]
    public void Constructor_GivenPositiveInt_StoresCorrectValue()
    {
        var fromInt  = new Natural(42);
        var fromUlong = new Natural(42UL);
        Assert.Equal(fromUlong.ToString(), fromInt.ToString());
    }

    [Fact]
    public void Constructor_GivenIntZero_ProducesZero()
    {
        var n = new Natural(0);
        Assert.True(Natural.IsZero(n));
    }

    [Fact]
    public void Constructor_GivenNegativeInt_ThrowsArgumentOutOfRangeException()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new Natural(-1));
    }

    [Theory]
    [InlineData(1, "1")]
    [InlineData(42, "42")]
    [InlineData(int.MaxValue, "2147483647")]
    public void Constructor_GivenPositiveInt_MatchesExpectedString(int value, string expected)
    {
        var n = new Natural(value);
        Assert.Equal(expected, n.ToString());
    }

    // -------------------------------------------------------------------------
    // Natural(Natural) — copy constructor
    // -------------------------------------------------------------------------

    [Fact]
    public void CopyConstructor_GivenNonZero_ProducesEqualValue()
    {
        var original = new Natural(98765UL);
        var copy = new Natural(original);
        Assert.Equal("98765", copy.ToString());
    }

    [Fact]
    public void CopyConstructor_GivenZero_ProducesZero()
    {
        var original = new Natural();
        var copy = new Natural(original);
        Assert.True(Natural.IsZero(copy));
    }

    [Fact]
    public void CopyConstructor_ProducesIndependentInstance()
    {
        // Original's string should be unchanged after creating a copy
        // and constructing a new value independently.
        var original = new Natural(12345UL);
        var copy = new Natural(original);

        // Overwrite the local variable — original must be unaffected
        copy = new Natural(99999UL);

        Assert.Equal("12345", original.ToString());
        Assert.Equal("99999", copy.ToString());
    }

    // -------------------------------------------------------------------------
    // Natural(string) — parse-based convenience constructor
    // -------------------------------------------------------------------------

    [Fact]
    public void Constructor_GivenValidString_ProducesCorrectValue()
    {
        var n = new Natural("12345");
        Assert.Equal("12345", n.ToString());
    }

    [Fact]
    public void Constructor_GivenStringZero_ProducesZero()
    {
        var n = new Natural("0");
        Assert.True(Natural.IsZero(n));
    }

    [Fact]
    public void Constructor_GivenStringWithLeadingZeros_TrimsToCanonicalForm()
    {
        var n = new Natural("007");
        Assert.Equal("7", n.ToString());
    }

    [Fact]
    public void Constructor_GivenStringLargeNumber_StoresCorrectly()
    {
        const string bigNum = "18446744073709551616"; // ulong.MaxValue + 1
        var n = new Natural(bigNum);
        Assert.Equal(bigNum, n.ToString());
    }

    [Fact]
    public void Constructor_GivenEmptyString_ThrowsFormatException()
    {
        Assert.Throws<FormatException>(() => new Natural(""));
    }

    [Fact]
    public void Constructor_GivenNonDigitString_ThrowsFormatException()
    {
        Assert.Throws<FormatException>(() => new Natural("12a3"));
    }

    // -------------------------------------------------------------------------
    // Natural(ReadOnlySpan<char>) — parse-based convenience constructor
    // -------------------------------------------------------------------------

    [Fact]
    public void Constructor_GivenValidSpan_ProducesCorrectValue()
    {
        var n = new Natural("12345".AsSpan());
        Assert.Equal("12345", n.ToString());
    }

    [Fact]
    public void Constructor_GivenSpanZero_ProducesZero()
    {
        var n = new Natural("0".AsSpan());
        Assert.True(Natural.IsZero(n));
    }

    [Fact]
    public void Constructor_GivenEmptySpan_ThrowsFormatException()
    {
        Assert.Throws<FormatException>(() => new Natural(ReadOnlySpan<char>.Empty));
    }

    [Fact]
    public void Constructor_GivenNonDigitSpan_ThrowsFormatException()
    {
        Assert.Throws<FormatException>(() => new Natural("12a3".AsSpan()));
    }
}
