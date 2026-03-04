using Lovelace.Integer;

namespace Lovelace.Integer.Tests;

/// <summary>
/// Functional tests for Integer.ToString, TryFormat.
/// ChecklistItem: ToString() / ToString(string, IFormatProvider?) / TryFormat
/// </summary>
public class IntegerToStringTests
{
    [Fact]
    public void ToString_GivenPositiveValue_ReturnsDigitsWithoutSign()
    {
        Assert.Equal("123", new Integer(123L).ToString());
    }

    [Fact]
    public void ToString_GivenNegativeValue_PrependsMinus()
    {
        Assert.Equal("-123", new Integer(-123L).ToString());
    }

    [Fact]
    public void ToString_GivenZero_ReturnsZeroWithoutSign()
    {
        Assert.Equal("0", new Integer(0L).ToString());
    }

    [Fact]
    public void TryFormat_GivenSufficientBuffer_WritesRepresentationAndReturnsTrue()
    {
        var n = new Integer(-42L);
        Span<char> buf = stackalloc char[10];
        bool ok = n.TryFormat(buf, out int written, default, null);
        Assert.True(ok);
        Assert.Equal("-42", new string(buf[..written]));
    }

    [Fact]
    public void TryFormat_GivenInsufficientBuffer_ReturnsFalse()
    {
        var n = new Integer(-12345L);
        Span<char> buf = stackalloc char[3]; // "-12345" needs 6 chars
        bool ok = n.TryFormat(buf, out int written, default, null);
        Assert.False(ok);
        Assert.Equal(0, written);
    }

    [Fact]
    public void ToString_WithFormatProvider_ReturnsSameAsToString()
    {
        var n = new Integer(-7L);
        Assert.Equal(n.ToString(), n.ToString(null, null));
    }
}
