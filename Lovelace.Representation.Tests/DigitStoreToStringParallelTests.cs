using Lovelace.Representation;

namespace Lovelace.Representation.Tests;

/// <summary>
/// Functional tests that verify the <c>ToString(char)</c> parallel-loop implementation
/// produces the correct digit ordering for numbers large enough to exercise multiple
/// interior bytes. Any wrong index formula (e.g., <c>offset + 2 * (lastByteIdx - 1 - c)</c>)
/// produces an incorrect string and causes these tests to fail.
/// </summary>
public class DigitStoreToStringParallelTests
{
    // =========================================================
    // Helpers
    // =========================================================

    /// <summary>
    /// Populates <paramref name="store"/> with <paramref name="digitCount"/> digits.
    /// Digit at position i receives value <c>i % 10</c>.
    /// When read back MSB-first the result string is built right-to-left:
    /// digit at position <c>digitCount-1</c> is printed first.
    /// </summary>
    private static DigitStore BuildStore(int digitCount)
    {
        var store = new DigitStore();
        for (int i = 0; i < digitCount; i++)
            store.SetDigit(i, (byte)(i % 10));
        return store;
    }

    /// <summary>
    /// Returns the expected string for a store built with <see cref="BuildStore"/>.
    /// Iterates from MSB (position <c>digitCount-1</c>) down to position 0.
    /// </summary>
    private static string ExpectedString(int digitCount)
    {
        var chars = new char[digitCount];
        for (int i = 0; i < digitCount; i++)
            chars[i] = (char)('0' + (digitCount - 1 - i) % 10);
        return new string(chars);
    }

    // =========================================================
    // Even digit count (exercises parallel loop with many iterations)
    // =========================================================

    [Fact]
    public void ToString_Given20Digits_ReturnsCorrectDigitOrder()
    {
        // 20 digits → lastByteIdx=9, 9 interior bytes → 9 Parallel.For iterations.
        // Any wrong offset or index calculation produces a different 20-char string.
        DigitStore store = BuildStore(20);
        string expected = ExpectedString(20); // "98765432109876543210"
        Assert.Equal(expected, store.ToString());
    }

    [Fact]
    public void ToString_Given100Digits_ReturnsCorrectDigitOrder()
    {
        // 100 digits → lastByteIdx=49, 49 interior bytes → 49 Parallel.For iterations.
        DigitStore store = BuildStore(100);
        string expected = ExpectedString(100);
        Assert.Equal(expected, store.ToString());
    }

    // =========================================================
    // Odd digit count (off-by-one in offset would misplace MSB)
    // =========================================================

    [Fact]
    public void ToString_Given19Digits_ReturnsCorrectDigitOrder()
    {
        // 19 digits (odd) → offset=1; any wrong offset shifts all interior digits by 1.
        DigitStore store = BuildStore(19);
        string expected = ExpectedString(19); // "8765432109876543210"
        Assert.Equal(expected, store.ToString());
    }

    [Fact]
    public void ToString_Given99Digits_ReturnsCorrectDigitOrder()
    {
        // 99 digits (odd) → 48 interior bytes → 48 Parallel.For iterations.
        DigitStore store = BuildStore(99);
        string expected = ExpectedString(99);
        Assert.Equal(expected, store.ToString());
    }

    // =========================================================
    // Separator path is unaffected by parallelization
    // =========================================================

    [Fact]
    public void ToString_Given20DigitsWithSeparator_ReturnsCorrectSeparatedString()
    {
        // 20 digits → "98,765,432,109,876,543,210"
        DigitStore store = BuildStore(20);
        // Build expected manually: "98765432109876543210" with commas every 3 from right
        string raw = ExpectedString(20);
        int len = raw.Length;
        var sb = new System.Text.StringBuilder(len + len / 3);
        for (int i = 0; i < len; i++)
        {
            int distFromRight = len - 1 - i;
            sb.Append(raw[i]);
            if (distFromRight > 0 && distFromRight % 3 == 0)
                sb.Append(',');
        }
        Assert.Equal(sb.ToString(), store.ToString(','));
    }
}
