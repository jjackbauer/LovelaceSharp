using Lovelace.Real;

namespace Lovelace.Real.Tests;

/// <summary>
/// Functional tests for <see cref="Real.PiAsync(long)"/>.
/// Checklist item: PiAsync — Task.Run wrapper over the parallel Pi implementation.
/// Tests 17–19 from the Lovelace.Real.Parallelism test plan.
/// </summary>
public class RealPiAsyncTests
{
    // -------------------------------------------------------------------------
    // Test 17 — correct value for one digit
    // -------------------------------------------------------------------------

    [Fact]
    public async Task PiAsync_GivenOneDigit_ReturnsCorrectValue()
    {
        // π = 3.1...; PiAsync is a Task.Run wrapper and must return the same result as Pi(1).
        Real result = await Real.PiAsync(1);
        Assert.Equal("3.1", result.ToString());
    }

    // -------------------------------------------------------------------------
    // Test 18 — validation error propagates through the task
    // -------------------------------------------------------------------------

    [Fact]
    public async Task PiAsync_GivenInvalidDigits_PropagatesArgumentOutOfRangeException()
    {
        // Pi(0) throws ArgumentOutOfRangeException; Task.Run captures it and
        // re-throws on await, so the caller sees ArgumentOutOfRangeException.
        await Assert.ThrowsAsync<ArgumentOutOfRangeException>(() => Real.PiAsync(0));
    }

    // -------------------------------------------------------------------------
    // Test 19 — concurrent awaits all return consistent values
    // -------------------------------------------------------------------------

    [Fact]
    public async Task PiAsync_GivenConcurrentAwaits_AllReturnCorrectValues()
    {
        // Two independently scheduled calls must both produce the same correct result.
        Real[] results = await Task.WhenAll(Real.PiAsync(10), Real.PiAsync(10));
        Assert.Equal("3.1415926535", results[0].ToString());
        Assert.Equal("3.1415926535", results[1].ToString());
    }
}
