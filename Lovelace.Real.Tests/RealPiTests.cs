using Lovelace.Real;

namespace Lovelace.Real.Tests;

/// <summary>
/// Functional tests for <see cref="Real.PiTo(long)"/>.
/// Checklist items:
///   - Real.PiTo(long digits) implementation (Chudnovsky algorithm)
///   - Pi — throw ArgumentOutOfRangeException when digits ≤ 0
///   - Pi — throw ArgumentOutOfRangeException when digits > MaxComputationDecimalPlaces
///   - Pi — compute internally with digits + 10 guard digits and truncate
///   - Pi — result contract: PeriodLength = 0, IsNegative = false, Exponent = -digits
/// </summary>
public class RealPiTests
{
    // -------------------------------------------------------------------------
    // Known-value accuracy tests
    // -------------------------------------------------------------------------

    [Fact]
    public void Pi_GivenOneDigit_ReturnsValueStartingWithThreePointOne()
    {
        // π = 3.1...; the first fractional digit is 1.
        using (Real.WithPrecision(1, 1))
        {
            Real result = Real.PiTo(1);
            Assert.Equal("3.1", result.ToString());
        }
    }

    [Fact]
    public void Pi_GivenTenDigits_ReturnsCorrectFirstTenFractionalDigits()
    {
        // π = 3.14159265358979...; first 10 fractional digits are 1415926535.
        using (Real.WithPrecision(10, 10))
        {
            Real result = Real.PiTo(10);
            Assert.Equal("3.1415926535", result.ToString());
        }
    }

    [Fact]
    public void Pi_GivenFiftyDigits_MatchesKnownReference()
    {
        // π to 50 decimal places:
        // 3.14159265358979323846264338327950288419716939937510
        using (Real.WithPrecision(50, 50))
        {
            Real result = Real.PiTo(50);
            Assert.Equal(
                "3.14159265358979323846264338327950288419716939937510",
                result.ToString());
        }
    }

    // -------------------------------------------------------------------------
    // Input validation
    // -------------------------------------------------------------------------

    [Fact]
    public void Pi_GivenZeroDigits_ThrowsArgumentOutOfRangeException()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => Real.PiTo(0));
    }

    [Fact]
    public void Pi_GivenNegativeDigits_ThrowsArgumentOutOfRangeException()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => Real.PiTo(-1));
    }

    [Fact]
    public void Pi_GivenDigitsExceedingMaxComputationDecimalPlaces_ThrowsArgumentOutOfRangeException()
    {
        // Pin a known computation cap so the assertion does not race with other tests
        // mutating the global MaxComputationDecimalPlaces static (which would otherwise
        // change the threshold between the read and the PiTo call).
        using (Real.WithPrecision(10, 10))
        {
            Assert.Throws<ArgumentOutOfRangeException>(() => Real.PiTo(11));
        }
    }

    // -------------------------------------------------------------------------
    // Result contract
    // -------------------------------------------------------------------------

    [Fact]
    public void Pi_GivenAnyValidDigitCount_ResultIsPositive()
    {
        Real result = Real.PiTo(10);
        Assert.True(Real.IsPositive(result));
    }

    [Fact]
    public void Pi_GivenAnyValidDigitCount_ResultIsNotPeriodic()
    {
        Real result = Real.PiTo(10);
        Assert.False(result.IsPeriodic);
    }

    [Fact]
    public void Pi_GivenDigits_ResultExponentEqualsNegativeDigits()
    {
        Real result = Real.PiTo(10);
        Assert.Equal(-10L, result.Exponent);
    }

    // -------------------------------------------------------------------------
    // Parallel-Pi refactoring tests
    // -------------------------------------------------------------------------

    [Fact]
    public void Pi_GivenConcurrentCallsFromMultipleThreads_AllReturnConsistentResults()
    {
        // Launching 8 concurrent PiTo(10) computations must all return "3.1415926535".
        // BSP sub-range lambdas operate on independent local variables, so no
        // data corruption from shared mutable state is expected.
        //
        // Pin the computation/display precision in an AsyncLocal scope so that concurrent
        // tests mutating the GLOBAL Real.DisplayDecimalPlaces / MaxComputationDecimalPlaces
        // statics cannot truncate (or otherwise alter) this test's ToString() results.
        const int taskCount = 8;
        const string expected = "3.1415926535";
        using (Real.WithPrecision(10, 10))
        {
            var tasks = Enumerable.Range(0, taskCount)
                .Select(_ => Task.Run(() => Real.PiTo(10).ToString()))
                .ToArray();
            string[] results = Task.WhenAll(tasks).GetAwaiter().GetResult();
            Assert.All(results, r => Assert.Equal(expected, r));
        }
    }
}
