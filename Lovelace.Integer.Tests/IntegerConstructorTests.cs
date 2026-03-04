using Lovelace.Integer;
using Nat = global::Lovelace.Natural.Natural;

namespace Lovelace.Integer.Tests;

/// <summary>
/// Functional tests for Integer constructors.
/// ChecklistItem: Structural / Type-level + Constructors / Assignment.
/// </summary>
public class IntegerConstructorTests
{
    // -------------------------------------------------------------------------
    // Integer() — default constructor
    // -------------------------------------------------------------------------

    [Fact]
    public void Constructor_Default_MagnitudeIsZero()
    {
        var n = new Integer();
        Assert.True(Integer.IsZero(n));
    }

    [Fact]
    public void Constructor_Default_SignIsPositive()
    {
        var n = new Integer();
        Assert.False(Integer.IsNegative(n));
    }

    // -------------------------------------------------------------------------
    // Integer(long) — value constructor
    // -------------------------------------------------------------------------

    [Fact]
    public void Constructor_GivenPositiveLong_StoresCorrectMagnitudeAndSign()
    {
        var n = new Integer(42L);
        Assert.Equal("42", n.ToString());
        Assert.True(Integer.IsPositive(n));
    }

    [Fact]
    public void Constructor_GivenNegativeLong_StoresCorrectMagnitudeAndSign()
    {
        var n = new Integer(-42L);
        Assert.Equal("-42", n.ToString());
        Assert.True(Integer.IsNegative(n));
    }

    [Fact]
    public void Constructor_GivenZeroLong_IsZeroAndPositive()
    {
        var n = new Integer(0L);
        Assert.True(Integer.IsZero(n));
        Assert.False(Integer.IsNegative(n));
    }

    [Fact]
    public void Constructor_GivenLongMinValue_ParsesSign()
    {
        var n = new Integer(long.MinValue);
        Assert.True(Integer.IsNegative(n));
        Assert.Equal("-9223372036854775808", n.ToString());
    }

    // -------------------------------------------------------------------------
    // Integer(int) — delegates to long overload
    // -------------------------------------------------------------------------

    [Fact]
    public void Constructor_GivenInt_DelegatesToLongOverload()
    {
        var fromInt  = new Integer(-7);
        var fromLong = new Integer(-7L);
        Assert.Equal(fromLong.ToString(), fromInt.ToString());
        Assert.Equal(Integer.IsNegative(fromLong), Integer.IsNegative(fromInt));
    }

    // -------------------------------------------------------------------------
    // Integer(string) — string parsing constructor
    // -------------------------------------------------------------------------

    [Fact]
    public void Constructor_GivenPositiveDecimalString_ReturnsCorrectValue()
    {
        var n = new Integer("42");
        Assert.Equal("42", n.ToString());
        Assert.True(Integer.IsPositive(n));
    }

    [Fact]
    public void Constructor_GivenNegativeDecimalString_ReturnsCorrectValue()
    {
        var n = new Integer("-7");
        Assert.Equal("-7", n.ToString());
        Assert.True(Integer.IsNegative(n));
    }

    [Fact]
    public void Constructor_GivenZeroString_IsZeroAndPositive()
    {
        var n = new Integer("0");
        Assert.True(Integer.IsZero(n));
        Assert.False(Integer.IsNegative(n));
    }

    [Fact]
    public void Constructor_GivenStringWithLeadingZeros_StripsLeadingZeros()
    {
        var n = new Integer("007");
        Assert.Equal(new Integer(7L), n);
    }

    [Fact]
    public void Constructor_GivenInvalidString_ThrowsFormatException()
    {
        Assert.Throws<FormatException>(() => new Integer("12a3"));
    }

    [Fact]
    public void Constructor_GivenArbitraryPrecisionString_ParsesCorrectly()
    {
        // 30-digit number — verifiable via round-trip ToString().
        const string thirtyDigits = "123456789012345678901234567890";
        var n = new Integer(thirtyDigits);
        Assert.Equal(thirtyDigits, n.ToString());
    }

    // -------------------------------------------------------------------------
    // Integer(ReadOnlySpan<char>) — span parsing constructor
    // -------------------------------------------------------------------------

    [Fact]
    public void Constructor_GivenPositiveSpan_ReturnsCorrectValue()
    {
        ReadOnlySpan<char> span = "99";
        var n = new Integer(span);
        Assert.Equal("99", n.ToString());
        Assert.True(Integer.IsPositive(n));
    }

    [Fact]
    public void Constructor_GivenNegativeSpan_ReturnsCorrectValue()
    {
        ReadOnlySpan<char> span = "-3";
        var n = new Integer(span);
        Assert.Equal("-3", n.ToString());
        Assert.True(Integer.IsNegative(n));
    }

    [Fact]
    public void Constructor_GivenEmptySpan_ThrowsFormatException()
    {
        Assert.Throws<FormatException>(() => new Integer(ReadOnlySpan<char>.Empty));
    }

    // -------------------------------------------------------------------------
    // Integer(Natural) — wraps Natural with positive sign
    // -------------------------------------------------------------------------

    [Fact]
    public void Constructor_GivenNatural_IsAlwaysPositive()
    {
        var mag = new Nat(99UL);
        var n = new Integer(mag);
        Assert.False(Integer.IsNegative(n));
    }

    [Fact]
    public void Constructor_GivenZeroNatural_IsZero()
    {
        var mag = new Nat();
        var n = new Integer(mag);
        Assert.True(Integer.IsZero(n));
    }

    // -------------------------------------------------------------------------
    // Integer(Natural, bool) — internal raw constructor
    // -------------------------------------------------------------------------

    [Fact]
    public void Constructor_GivenNonZeroMagnitudeAndIsNegativeTrue_IsNegative()
    {
        var mag = new Nat(5UL);
        var n = new Integer(mag, true);
        Assert.True(Integer.IsNegative(n));
    }

    [Fact]
    public void Constructor_GivenZeroMagnitudeAndIsNegativeTrue_NormalisedToPositive()
    {
        var mag = new Nat(); // zero
        var n = new Integer(mag, true);
        Assert.False(Integer.IsNegative(n));
        Assert.True(Integer.IsZero(n));
    }
}
