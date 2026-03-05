using Lovelace.Representation;

namespace Lovelace.Representation.Tests;

/// <summary>
/// Functional tests for <see cref="DigitStore.TrimLeadingZeros"/>.
///
/// Tests verify observable behaviour (DigitCount, IsZero, GetDigit) so they
/// exercise the refactored implementation (single outer lock + Unsafe helpers)
/// identically to the original. Removing the implementation body would cause
/// each test to fail with <see cref="InvalidOperationException"/> or an
/// incorrect assertion.
/// </summary>
public class DigitStoreTrimLeadingZerosTests
{
    // =========================================================
    // Leading zeros removed
    // =========================================================

    /// <summary>
    /// Digits [5, 0, 0] (value 005): two leading zeros must be removed,
    /// leaving a single digit 5 at position 0.
    /// </summary>
    [Fact]
    public void TrimLeadingZeros_GivenMultipleLeadingZeroDigits_DigitCountReducedAndValuePreserved()
    {
        var store = new DigitStore();
        store.SetDigit(0, 5);
        store.SetDigit(1, 0);
        store.SetDigit(2, 0);

        store.TrimLeadingZeros();

        Assert.Equal(1L, store.DigitCount);
        Assert.Equal(5, store.GetDigit(0));
        Assert.False(store.IsZero);
    }

    /// <summary>
    /// Digits [4, 0] (even digit count with one leading zero): one leading zero
    /// must be removed, leaving a single digit 4 at position 0.
    /// </summary>
    [Fact]
    public void TrimLeadingZeros_GivenEvenDigitCountWithSingleLeadingZero_TrimsToSingleDigit()
    {
        var store = new DigitStore();
        store.SetDigit(0, 4);
        store.SetDigit(1, 0);

        store.TrimLeadingZeros();

        Assert.Equal(1L, store.DigitCount);
        Assert.Equal(4, store.GetDigit(0));
    }

    // =========================================================
    // No leading zeros — value unchanged
    // =========================================================

    /// <summary>
    /// Digits [3, 2, 1] (value 123, no leading zeros): DigitCount and all digit
    /// values must remain unchanged.
    /// </summary>
    [Fact]
    public void TrimLeadingZeros_GivenNoLeadingZeros_LeavesValueUnchanged()
    {
        var store = new DigitStore();
        store.SetDigit(0, 3);
        store.SetDigit(1, 2);
        store.SetDigit(2, 1);

        store.TrimLeadingZeros();

        Assert.Equal(3L, store.DigitCount);
        Assert.Equal("123", store.ToString());
    }

    /// <summary>
    /// Single non-zero digit [7]: DigitCount must remain 1 and GetDigit(0) must
    /// return 7.
    /// </summary>
    [Fact]
    public void TrimLeadingZeros_GivenSingleNonZeroDigit_LeavesValueUnchanged()
    {
        var store = new DigitStore();
        store.SetDigit(0, 7);

        store.TrimLeadingZeros();

        Assert.Equal(1L, store.DigitCount);
        Assert.Equal(7, store.GetDigit(0));
    }

    // =========================================================
    // All-zero input resets to zero state
    // =========================================================

    /// <summary>
    /// Digits [0, 0, 0]: after trimming IsZero must be true.
    /// (SetDigit sets IsZero = false; TrimLeadingZeros must call Reset when
    /// only a zero digit remains.)
    /// </summary>
    [Fact]
    public void TrimLeadingZeros_GivenAllZeroDigits_ResetsToZeroState()
    {
        var store = new DigitStore();
        store.SetDigit(0, 0);
        store.SetDigit(1, 0);
        store.SetDigit(2, 0);

        store.TrimLeadingZeros();

        Assert.True(store.IsZero);
    }

    /// <summary>
    /// Single zero digit [0] (SetDigit(0, 0) sets IsZero = false):
    /// TrimLeadingZeros must detect the lone zero digit and call Reset,
    /// restoring IsZero = true.
    /// </summary>
    [Fact]
    public void TrimLeadingZeros_GivenSingleZeroDigit_ResetsToZeroState()
    {
        var store = new DigitStore();
        store.SetDigit(0, 0); // IsZero becomes false; DigitCount becomes 1

        store.TrimLeadingZeros();

        Assert.True(store.IsZero);
    }
}
