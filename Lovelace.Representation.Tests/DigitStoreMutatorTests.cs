using Lovelace.Representation;

namespace Lovelace.Representation.Tests;

/// <summary>
/// Functional tests for the locked internal mutator methods
/// <c>SetDigitCount</c> and <c>SetIsZero</c> on <see cref="DigitStore"/>.
///
/// These replace the former public internal setters (which bypassed <c>_syncRoot</c>)
/// with methods that hold the monitor lock during the write.
/// </summary>
public class DigitStoreMutatorTests
{
    // =========================================================
    // SetDigitCount
    // =========================================================

    /// <summary>
    /// SetDigitCount with a positive value must update the DigitCount property to that value.
    /// </summary>
    [Fact]
    public void SetDigitCount_GivenPositiveValue_DigitCountIsUpdated()
    {
        var store = new DigitStore();

        store.SetDigitCount(7);

        Assert.Equal(7L, store.DigitCount);
    }

    /// <summary>
    /// SetDigitCount with zero must set DigitCount to zero.
    /// </summary>
    [Fact]
    public void SetDigitCount_GivenZero_DigitCountIsZero()
    {
        var store = new DigitStore();
        store.SetDigit(0, 3); // DigitCount becomes 1 via SetDigit
        store.SetDigitCount(0);

        Assert.Equal(0L, store.DigitCount);
    }

    /// <summary>
    /// Concurrent calls to SetDigitCount from many tasks must not produce a corrupted
    /// (out-of-range) final value. The last write wins; the result must be one of the
    /// values written by a task, i.e. in [0, taskCount).
    /// </summary>
    [Fact]
    public async Task SetDigitCount_ConcurrentCalls_FinalValueIsOneOfTheWrittenValues()
    {
        const int TaskCount = 200;
        var store = new DigitStore();
        var tasks = Enumerable.Range(0, TaskCount)
            .Select(i => Task.Run(() => store.SetDigitCount(i)))
            .ToArray();

        await Task.WhenAll(tasks);

        // The final DigitCount must be one of the values written: [0, TaskCount)
        Assert.InRange(store.DigitCount, 0L, (long)(TaskCount - 1));
    }

    // =========================================================
    // SetIsZero
    // =========================================================

    /// <summary>
    /// SetIsZero(true) must flip IsZero to true even when a digit was previously set.
    /// </summary>
    [Fact]
    public void SetIsZero_GivenTrue_IsZeroBecomesTrue()
    {
        var store = new DigitStore();
        store.SetDigit(0, 5); // IsZero becomes false internally after a digit write

        store.SetIsZero(true);

        Assert.True(store.IsZero);
    }

    /// <summary>
    /// SetIsZero(false) on a freshly constructed (all-zero) store must flip IsZero to false.
    /// </summary>
    [Fact]
    public void SetIsZero_GivenFalse_IsZeroBecomesFalse()
    {
        var store = new DigitStore(); // IsZero == true by construction

        store.SetIsZero(false);

        Assert.False(store.IsZero);
    }
}
