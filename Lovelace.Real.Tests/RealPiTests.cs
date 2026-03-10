using Lovelace.Real;

namespace Lovelace.Real.Tests;

/// <summary>
/// Functional tests for <see cref="Real.Pi(long)"/>.
/// Checklist items:
///   - Real.Pi(long digits) implementation (Chudnovsky algorithm)
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
        Real result = Real.Pi(1);
        Assert.Equal("3.1", result.ToString());
    }

    [Fact]
    public void Pi_GivenTenDigits_ReturnsCorrectFirstTenFractionalDigits()
    {
        // π = 3.14159265358979...; first 10 fractional digits are 1415926535.
        Real result = Real.Pi(10);
        Assert.Equal("3.1415926535", result.ToString());
    }

    [Fact]
    public void Pi_GivenFiftyDigits_MatchesKnownReference()
    {
        // π to 50 decimal places:
        // 3.14159265358979323846264338327950288419716939937510
        Real result = Real.Pi(50);
        Assert.Equal(
            "3.14159265358979323846264338327950288419716939937510",
            result.ToString());
    }

    // -------------------------------------------------------------------------
    // Input validation
    // -------------------------------------------------------------------------

    [Fact]
    public void Pi_GivenZeroDigits_ThrowsArgumentOutOfRangeException()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => Real.Pi(0));
    }

    [Fact]
    public void Pi_GivenNegativeDigits_ThrowsArgumentOutOfRangeException()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => Real.Pi(-1));
    }

    [Fact]
    public void Pi_GivenDigitsExceedingMaxComputationDecimalPlaces_ThrowsArgumentOutOfRangeException()
    {
        long max = Real.MaxComputationDecimalPlaces;
        Assert.Throws<ArgumentOutOfRangeException>(() => Real.Pi(max + 1));
    }

    // -------------------------------------------------------------------------
    // Result contract
    // -------------------------------------------------------------------------

    [Fact]
    public void Pi_GivenAnyValidDigitCount_ResultIsPositive()
    {
        Real result = Real.Pi(10);
        Assert.True(Real.IsPositive(result));
    }

    [Fact]
    public void Pi_GivenAnyValidDigitCount_ResultIsNotPeriodic()
    {
        Real result = Real.Pi(10);
        Assert.False(result.IsPeriodic);
    }

    [Fact]
    public void Pi_GivenDigits_ResultExponentEqualsNegativeDigits()
    {
        Real result = Real.Pi(10);
        Assert.Equal(-10L, result.Exponent);
    }
}
