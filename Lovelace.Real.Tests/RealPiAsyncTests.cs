using Lovelace.Real;

namespace Lovelace.Real.Tests;

/// <summary>
/// Functional tests for <see cref="Real.PiToAsync(long)"/>.
/// Checklist item: PiToAsync — Task.Run wrapper over the parallel Pi implementation.
/// Tests 17–19 from the Lovelace.Real.Parallelism test plan.
/// </summary>
public class RealPiToAsyncTests
{
    // -------------------------------------------------------------------------
    // Test 17 — correct value for one digit
    // -------------------------------------------------------------------------

    [Fact]
    public async Task PiToAsync_GivenOneDigit_ReturnsCorrectValue()
    {
        // π = 3.1...; PiToAsync is a Task.Run wrapper and must return the same result as PiTo(1).
        using (Real.WithPrecision(1, 1))
        {
            Real result = await Real.PiToAsync(1);
            Assert.Equal("3.1", result.ToString());
        }
    }

    // -------------------------------------------------------------------------
    // Test 18 — validation error propagates through the task
    // -------------------------------------------------------------------------

    [Fact]
    public async Task PiToAsync_GivenInvalidDigits_PropagatesArgumentOutOfRangeException()
    {
        // PiTo(0) throws ArgumentOutOfRangeException; Task.Run captures it and
        // re-throws on await, so the caller sees ArgumentOutOfRangeException.
        await Assert.ThrowsAsync<ArgumentOutOfRangeException>(() => Real.PiToAsync(0));
    }

    // -------------------------------------------------------------------------
    // Test 19 — concurrent awaits all return consistent values
    // -------------------------------------------------------------------------

    [Fact]
    public async Task PiToAsync_GivenConcurrentAwaits_AllReturnCorrectValues()
    {
        // Two independently scheduled calls must both produce the same correct result.
        // Pin the computation/display precision in an AsyncLocal scope so concurrent tests
        // mutating the global Real.DisplayDecimalPlaces static cannot truncate the result.
        using (Real.WithPrecision(10, 10))
        {
            Real[] results = await Task.WhenAll(Real.PiToAsync(10), Real.PiToAsync(10));
            Assert.Equal("3.1415926535", results[0].ToString());
            Assert.Equal("3.1415926535", results[1].ToString());
        }
    }
}
