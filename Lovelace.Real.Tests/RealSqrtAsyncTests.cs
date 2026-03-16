using Lovelace.Real;

namespace Lovelace.Real.Tests;

/// <summary>
/// Functional tests for <see cref="Real.SqrtAsync(Real)"/>.
/// Checklist item: SqrtAsync — Task.Run wrapper over Sqrt(Real).
/// Tests 20–22 from the Lovelace.Real.Parallelism test plan.
/// </summary>
public class RealSqrtAsyncTests
{
    // -------------------------------------------------------------------------
    // Test 20 — perfect square returns exact value
    // -------------------------------------------------------------------------

    [Fact]
    public async Task SqrtAsync_GivenPerfectSquareFour_ReturnsExactlyTwo()
    {
        // SqrtAsync is a thin Task.Run wrapper; the result must match Real.Sqrt(new Real(4)).
        Real result = await Real.SqrtAsync(new Real(4.0));
        Assert.Equal(Real.Parse("2"), result);
    }

    // -------------------------------------------------------------------------
    // Test 21 — negative input propagates ArithmeticException
    // -------------------------------------------------------------------------

    [Fact]
    public async Task SqrtAsync_GivenNegativeInput_PropagatesArithmeticException()
    {
        // Real.Sqrt throws ArithmeticException for negative inputs.
        // Task.Run captures exceptions from the delegate and re-throws them on await.
        await Assert.ThrowsAsync<ArithmeticException>(() => Real.SqrtAsync(new Real("-1")));
    }

    // -------------------------------------------------------------------------
    // Test 22 — concurrent awaits each return correct values
    // -------------------------------------------------------------------------

    [Fact]
    public async Task SqrtAsync_GivenConcurrentAwaits_AllReturnCorrectValues()
    {
        // Each task operates on independent inputs and produces the same result
        // as the corresponding serial Real.Sqrt call.
        // Use WithLocalPrecision to bound the computation: 20 digits is far more
        // than needed to verify the 13-character StartsWith assertions.
        using var _ = Real.WithLocalPrecision(20);
        Real[] results = await Task.WhenAll(
            Real.SqrtAsync(new Real("2")),
            Real.SqrtAsync(new Real("3"))
        );

        string sqrt2 = results[0].ToString();
        string sqrt3 = results[1].ToString();

        Assert.StartsWith("1.41421356237", sqrt2);
        Assert.StartsWith("1.73205080756", sqrt3);
    }
}
