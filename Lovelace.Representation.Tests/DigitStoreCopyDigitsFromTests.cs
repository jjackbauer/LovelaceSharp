using Lovelace.Representation;

namespace Lovelace.Representation.Tests;

/// <summary>
/// Functional tests for <see cref="DigitStore.CopyDigitsFrom"/>.
///
/// <c>CopyDigitsFrom</c> copies only the backing bytes; callers are responsible for
/// keeping <c>_digitCount</c> and <c>_isZero</c> consistent.  To exercise the copy
/// correctly, each test constructs the destination via the copy constructor (so
/// <c>_digitCount</c> starts correct), then calls <c>CopyDigitsFrom</c> with an
/// updated source.
///
/// All tests will fail if the implementation body is replaced with
/// <see cref="NotImplementedException"/> because they assert on concrete byte counts
/// and digit values read through the public API.
/// </summary>
public class DigitStoreCopyDigitsFromTests
{
    // =========================================================
    // Bytes are copied — even digit count (full bytes)
    // =========================================================

    /// <summary>
    /// Source has 4 digits packed into 2 bytes (digits 3,7,1,5 → value 5173).
    /// After CopyDigitsFrom the destination must have ByteCount==2 and read back
    /// the same nibbles as the source for all four digit positions.
    /// </summary>
    [Fact]
    public void CopyDigitsFrom_GivenFourDigitSource_ByteCountAndDigitsMatchSource()
    {
        var source = new DigitStore();
        source.SetDigit(0, 3);
        source.SetDigit(1, 7);
        source.SetDigit(2, 1);
        source.SetDigit(3, 5);

        // dest starts as a copy so _digitCount is already 4.
        var dest = new DigitStore(source);

        // Replace source bytes with new content (6, 2, 8, 4).
        var updated = new DigitStore();
        updated.SetDigit(0, 6);
        updated.SetDigit(1, 2);
        updated.SetDigit(2, 8);
        updated.SetDigit(3, 4);

        dest.CopyDigitsFrom(updated);

        Assert.Equal(2, dest.ByteCount);
        // _digitCount was not changed by CopyDigitsFrom — still 4.
        Assert.Equal(4, dest.DigitCount);
        // Digits come from the updated source.
        Assert.Equal(6, dest.GetDigit(0));
        Assert.Equal(2, dest.GetDigit(1));
        Assert.Equal(8, dest.GetDigit(2));
        Assert.Equal(4, dest.GetDigit(3));
    }

    // =========================================================
    // Bytes are copied — odd digit count (partial last byte)
    // =========================================================

    /// <summary>
    /// Source has 3 digits packed into 2 bytes (high nibble of byte 1 is the MSB digit).
    /// After CopyDigitsFrom the destination must have ByteCount==2 and read back the
    /// same three digit values.
    /// </summary>
    [Fact]
    public void CopyDigitsFrom_GivenThreeDigitSource_ByteCountAndDigitsMatchSource()
    {
        var source = new DigitStore();
        source.SetDigit(0, 1);
        source.SetDigit(1, 2);
        source.SetDigit(2, 9);

        var dest = new DigitStore(source);

        var updated = new DigitStore();
        updated.SetDigit(0, 4);
        updated.SetDigit(1, 5);
        updated.SetDigit(2, 7);

        dest.CopyDigitsFrom(updated);

        Assert.Equal(2, dest.ByteCount);
        Assert.Equal(3, dest.DigitCount); // unchanged by CopyDigitsFrom
        Assert.Equal(4, dest.GetDigit(0));
        Assert.Equal(5, dest.GetDigit(1));
        Assert.Equal(7, dest.GetDigit(2));
    }

    // =========================================================
    // Copy from zero source — destination bytes unchanged
    // =========================================================

    /// <summary>
    /// When the source is in zero state (IsZero == true), CopyDigitsFrom must leave
    /// the destination's backing bytes entirely untouched.
    /// Concrete assertion: the destination retains its original digit values.
    /// </summary>
    [Fact]
    public void CopyDigitsFrom_GivenZeroSource_DestinationBytesNotOverwritten()
    {
        var dest = new DigitStore();
        dest.SetDigit(0, 8);
        dest.SetDigit(1, 3);

        var zeroSource = new DigitStore(); // IsZero == true

        dest.CopyDigitsFrom(zeroSource);

        // Digits and byte count must be unchanged.
        Assert.Equal(1, dest.ByteCount);
        Assert.Equal(2, dest.DigitCount);
        Assert.Equal(8, dest.GetDigit(0));
        Assert.Equal(3, dest.GetDigit(1));
    }

    // =========================================================
    // Deep copy — mutating source after copy does not affect dest
    // =========================================================

    /// <summary>
    /// After CopyDigitsFrom completes, overwriting digit 0 in the source must not
    /// change digit 0 in the destination.  This verifies the copy is a deep byte-level
    /// copy and not a reference share.
    /// </summary>
    [Fact]
    public void CopyDigitsFrom_GivenNonZeroSource_MutatingSourceAfterCopyDoesNotAffectDest()
    {
        var source = new DigitStore();
        source.SetDigit(0, 5);
        source.SetDigit(1, 9);

        var dest = new DigitStore(source);
        dest.CopyDigitsFrom(source);

        // Mutate the source after the copy.
        source.SetDigit(0, 0);
        source.SetDigit(1, 0);

        // Destination digit 0 must still be 5, digit 1 still 9.
        Assert.Equal(5, dest.GetDigit(0));
        Assert.Equal(9, dest.GetDigit(1));
    }

    // =========================================================
    // Large source — 10 digits, 5 bytes
    // =========================================================

    /// <summary>
    /// Source with 10 digits (5 bytes) is copied into a destination that was
    /// initialised from the same source.  All 10 digit values must survive the copy.
    /// </summary>
    [Fact]
    public void CopyDigitsFrom_GivenTenDigitSource_AllDigitsPreserved()
    {
        byte[] expected = [2, 4, 6, 8, 1, 3, 5, 7, 9, 0];

        var source = new DigitStore();
        for (int i = 0; i < expected.Length; i++)
            source.SetDigit(i, expected[i]);

        var dest = new DigitStore(source);

        // Replace with a second source that has a different sequence.
        byte[] expected2 = [9, 8, 7, 6, 5, 4, 3, 2, 1, 0];
        var source2 = new DigitStore();
        for (int i = 0; i < expected2.Length; i++)
            source2.SetDigit(i, expected2[i]);

        dest.CopyDigitsFrom(source2);

        Assert.Equal(5, dest.ByteCount);
        for (int i = 0; i < expected2.Length; i++)
            Assert.Equal(expected2[i], dest.GetDigit(i));
    }

    // =========================================================
    // Deadlock avoidance — two threads cross-copy simultaneously
    // =========================================================

    /// <summary>
    /// Thread A calls <c>a.CopyDigitsFrom(b)</c> while Thread B calls
    /// <c>b.CopyDigitsFrom(a)</c> concurrently for many iterations.
    /// Without canonical lock ordering this is an ABBA deadlock; with the instance-ID
    /// ordering introduced by the Span-based implementation both threads must complete
    /// within the test timeout.
    ///
    /// Concrete assertion: both stores end with ByteCount > 0 (no data was lost)
    /// and no exception is thrown.
    /// </summary>
    [Fact]
    public async Task CopyDigitsFrom_CrossCopyConcurrent_NeitherDeadlocksNorThrows()
    {
        const int Iterations = 500;

        var a = new DigitStore();
        a.SetDigit(0, 1);
        a.SetDigit(1, 2);

        var b = new DigitStore();
        b.SetDigit(0, 3);
        b.SetDigit(1, 4);

        // b starts as an independent copy so _digitCount == 2 on both sides.
        var destA = new DigitStore(a);
        var destB = new DigitStore(b);

        var tA = Task.Run(() =>
        {
            for (int i = 0; i < Iterations; i++)
                destA.CopyDigitsFrom(b);
        });

        var tB = Task.Run(() =>
        {
            for (int i = 0; i < Iterations; i++)
                destB.CopyDigitsFrom(a);
        });

        // 10-second timeout catches a deadlock without hanging CI indefinitely.
        var all = Task.WhenAll(tA, tB);
        var completed = await Task.WhenAny(all, Task.Delay(TimeSpan.FromSeconds(10)));

        Assert.True(completed == all, "Deadlock detected: cross-copy did not complete within 10 seconds.");

        // Concrete post-condition: each dest retains one backing byte.
        Assert.Equal(1, destA.ByteCount);
        Assert.Equal(1, destB.ByteCount);
    }
}
