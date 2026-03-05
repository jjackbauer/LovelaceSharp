using Lovelace.Representation;

namespace Lovelace.Representation.Tests;

/// <summary>
/// Functional tests for <see cref="DigitStore.SnapshotDigits()"/>.
/// Parallelization audit item: "operator * — snapshot operand digit arrays
/// before Parallel.For"; the snapshot helper is the new internal API
/// that enables lock-free reads inside the parallel lambda.
/// </summary>
public class DigitStoreSnapshotDigitsTests
{
    // ─────────────────────────────────────────────────────────────────────────
    // Zero / empty store
    // ─────────────────────────────────────────────────────────────────────────

    [Fact]
    public void SnapshotDigits_GivenZeroStore_ReturnsEmptyArray()
    {
        var store = new DigitStore(); // IsZero = true, DigitCount = 0
        byte[] snapshot = store.SnapshotDigits();
        Assert.Empty(snapshot);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Single digit (occupies low nibble of the first byte)
    // ─────────────────────────────────────────────────────────────────────────

    [Fact]
    public void SnapshotDigits_GivenSingleDigit_ReturnsSingleElementArray()
    {
        var store = new DigitStore();
        store.SetDigit(0, 5);

        byte[] snapshot = store.SnapshotDigits();

        Assert.Equal(1, snapshot.Length);
        Assert.Equal(5, snapshot[0]);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Two digits packed in the same byte (high nibble + low nibble)
    // ─────────────────────────────────────────────────────────────────────────

    [Fact]
    public void SnapshotDigits_GivenPairedDigitsInSameByte_CorrectlyUnpacksBothNibbles()
    {
        // Position 0 (even → high nibble) = 4
        // Position 1 (odd  → low nibble)  = 9
        // Both live in byte[0], so unpacking must read each nibble independently.
        var store = new DigitStore();
        store.SetDigit(0, 4);
        store.SetDigit(1, 9);

        byte[] snapshot = store.SnapshotDigits();

        Assert.Equal(2, snapshot.Length);
        Assert.Equal(4, snapshot[0]);
        Assert.Equal(9, snapshot[1]);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Multiple digits across several bytes
    // ─────────────────────────────────────────────────────────────────────────

    [Fact]
    public void SnapshotDigits_GivenMultipleDigits_ReturnsAllDigitsLeastSignificantFirst()
    {
        // Represents the number whose digits are (LSB first): 3, 7, 2, 5
        var store = new DigitStore();
        store.SetDigit(0, 3);
        store.SetDigit(1, 7);
        store.SetDigit(2, 2);
        store.SetDigit(3, 5);

        byte[] snapshot = store.SnapshotDigits();

        Assert.Equal(new byte[] { 3, 7, 2, 5 }, snapshot);
    }

    [Theory]
    [InlineData(new byte[] { 0, 1, 2, 3, 4, 5, 6, 7, 8, 9 })]
    [InlineData(new byte[] { 9, 8, 7, 6, 5, 4, 3, 2, 1, 0 })]
    [InlineData(new byte[] { 1, 0, 1, 0, 1, 0, 1, 0, 1, 0 })]
    public void SnapshotDigits_GivenTenDigitSequence_ReturnsIdenticalSequence(byte[] digits)
    {
        var store = new DigitStore();
        for (int i = 0; i < digits.Length; i++)
            store.SetDigit(i, digits[i]);

        byte[] snapshot = store.SnapshotDigits();

        Assert.Equal(digits, snapshot);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Snapshot is an independent copy — mutations do not affect the store
    // ─────────────────────────────────────────────────────────────────────────

    [Fact]
    public void SnapshotDigits_ReturnsCopyNotReference_MutatingSnapshotDoesNotAffectStore()
    {
        var store = new DigitStore();
        store.SetDigit(0, 7);

        byte[] snapshot = store.SnapshotDigits();
        snapshot[0] = 0xFF; // corrupt the snapshot

        // The store's digit at position 0 must be unchanged.
        Assert.Equal(7, store.GetDigit(0));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Snapshot length matches DigitCount
    // ─────────────────────────────────────────────────────────────────────────

    [Theory]
    [InlineData(1)]
    [InlineData(3)]
    [InlineData(7)]
    [InlineData(11)]
    public void SnapshotDigits_LengthEqualsDigitCount(int count)
    {
        var store = new DigitStore();
        for (int i = 0; i < count; i++)
            store.SetDigit(i, (byte)(i % 10));

        byte[] snapshot = store.SnapshotDigits();

        Assert.Equal(store.DigitCount, snapshot.Length);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Snapshot is thread-safe: concurrent reads on a fully-written store
    // all see the same digit sequence
    // ─────────────────────────────────────────────────────────────────────────

    [Fact]
    public async Task SnapshotDigits_GivenConcurrentReads_AllSnapshotsAreIdentical()
    {
        // Build a 20-digit store once.
        byte[] expected = Enumerable.Range(0, 20).Select(i => (byte)(i % 10)).ToArray();
        var store = new DigitStore();
        for (int i = 0; i < expected.Length; i++)
            store.SetDigit(i, expected[i]);

        // 32 concurrent snapshots must all match the expected array.
        const int taskCount = 32;
        var tasks = new Task<byte[]>[taskCount];
        for (int i = 0; i < taskCount; i++)
            tasks[i] = Task.Run(() => store.SnapshotDigits());

        byte[][] results = await Task.WhenAll(tasks);

        foreach (byte[] result in results)
            Assert.Equal(expected, result);
    }
}
