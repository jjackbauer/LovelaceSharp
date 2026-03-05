using Lovelace.Representation;

namespace Lovelace.Representation.Tests;

/// <summary>
/// Functional tests for <see cref="DigitStore.Reset"/>.
///
/// These tests verify the observable contract that Reset() must satisfy
/// after the P1 refactoring that eliminates redundant reentrant lock
/// acquisitions (replacing calls to locking <c>ClearDigits()</c> and
/// <c>Initialize()</c> with their lock-free <c>*Unsafe</c> counterparts).
/// </summary>
public class DigitStoreResetTests
{
    // =========================================================
    // Non-zero store → fully cleared
    // =========================================================

    /// <summary>
    /// After writing a single digit and calling Reset(), IsZero must be true.
    /// </summary>
    [Fact]
    public void Reset_GivenNonZeroStore_IsZeroBecomesTrue()
    {
        var store = new DigitStore();
        store.SetDigit(0, 5);   // IsZero = false, DigitCount = 1

        store.Reset();

        Assert.True(store.IsZero);
    }

    /// <summary>
    /// After writing a single digit and calling Reset(), DigitCount must be 0.
    /// </summary>
    [Fact]
    public void Reset_GivenNonZeroStore_DigitCountBecomesZero()
    {
        var store = new DigitStore();
        store.SetDigit(0, 7);

        store.Reset();

        Assert.Equal(0L, store.DigitCount);
    }

    /// <summary>
    /// After writing a single digit and calling Reset(), ByteCount must be 0
    /// because ClearDigits()/ClearDigitsUnsafe() empties the backing list.
    /// </summary>
    [Fact]
    public void Reset_GivenNonZeroStore_ByteCountBecomesZero()
    {
        var store = new DigitStore();
        store.SetDigit(0, 3);

        store.Reset();

        Assert.Equal(0L, store.ByteCount);
    }

    /// <summary>
    /// After writing digit 9 at position 0 and calling Reset(), GetDigit(0)
    /// must return 0 (the default for an IsZero store).
    /// </summary>
    [Fact]
    public void Reset_GivenNonZeroStore_GetDigitReturnsZero()
    {
        var store = new DigitStore();
        store.SetDigit(0, 9);

        store.Reset();

        Assert.Equal((byte)0, store.GetDigit(0));
    }

    /// <summary>
    /// After writing three digits spanning two BCD bytes and calling Reset(),
    /// IsZero must be true, DigitCount must be 0, and ByteCount must be 0.
    /// </summary>
    [Fact]
    public void Reset_GivenMultiDigitStore_ClearsEverything()
    {
        var store = new DigitStore();
        store.SetDigit(0, 1);
        store.SetDigit(1, 2);
        store.SetDigit(2, 3);   // Two bytes consumed (digits 0-1 in byte 0; digit 2 in byte 1)

        store.Reset();

        Assert.True(store.IsZero);
        Assert.Equal(0L, store.DigitCount);
        Assert.Equal(0L, store.ByteCount);
    }

    // =========================================================
    // Already-zero store → no-op
    // =========================================================

    /// <summary>
    /// Calling Reset() on the default (zero) store must not throw and must
    /// leave IsZero true and DigitCount 0.
    /// </summary>
    [Fact]
    public void Reset_GivenAlreadyZeroStore_RemainsZero()
    {
        var store = new DigitStore();    // IsZero = true by default

        store.Reset();                   // Should be a no-op

        Assert.True(store.IsZero);
        Assert.Equal(0L, store.DigitCount);
    }

    // =========================================================
    // Reset → reuse: store is usable after Reset
    // =========================================================

    /// <summary>
    /// After Reset(), the store must accept new digits via SetDigit without error,
    /// and those digits must be readable via GetDigit.
    /// </summary>
    [Fact]
    public void Reset_ThenSetDigit_StoreIsReusable()
    {
        var store = new DigitStore();
        store.SetDigit(0, 4);

        store.Reset();

        // Write new digits after reset
        store.SetDigit(0, 6);
        store.SetDigit(1, 8);

        Assert.False(store.IsZero);
        Assert.Equal(2L, store.DigitCount);
        Assert.Equal((byte)6, store.GetDigit(0));
        Assert.Equal((byte)8, store.GetDigit(1));
    }
}
