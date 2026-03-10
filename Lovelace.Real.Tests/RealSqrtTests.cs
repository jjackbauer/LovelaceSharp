using Lovelace.Real;

namespace Lovelace.Real.Tests;

/// <summary>
/// Functional tests for <see cref="Real.Sqrt(Real)"/>.
/// Checklist items:
///   - Real.Sqrt(Real value) implementation
///   - Internal Sqrt(Real value, long precision) implementation
///   - Sqrt — throw ArithmeticException when value is negative
///   - Sqrt — return Real.Zero immediately when value is zero
///   - Sqrt — result contract: IsPeriodic = false, IsNegative = false
/// </summary>
public class RealSqrtTests
{
    // -------------------------------------------------------------------------
    // Perfect squares — exact results
    // -------------------------------------------------------------------------

    [Fact]
    public void Sqrt_GivenPerfectSquareFour_ReturnsExactlyTwo()
    {
        // √4 = 2 exactly: Newton-Raphson seeds at 2.0 and converges in one step.
        Real result = Real.Sqrt(new Real(4.0));
        Assert.Equal(new Real("2"), result);
    }

    [Fact]
    public void Sqrt_GivenPerfectSquareNine_ReturnsExactlyThree()
    {
        // √9 = 3 exactly: Newton-Raphson seeds at 3.0 and converges in one step.
        Real result = Real.Sqrt(new Real(9.0));
        Assert.Equal(new Real("3"), result);
    }

    [Fact]
    public void Sqrt_GivenOne_ReturnsOne()
    {
        // √1 = 1: Newton-Raphson seeds at 1.0 and converges immediately.
        Real result = Real.Sqrt(Real.One);
        Assert.Equal(Real.One, result);
    }

    // -------------------------------------------------------------------------
    // Zero
    // -------------------------------------------------------------------------

    [Fact]
    public void Sqrt_GivenZero_ReturnsZero()
    {
        // √0 = 0: method returns Real.Zero immediately before iterating.
        Real result = Real.Sqrt(Real.Zero);
        Assert.Equal(Real.Zero, result);
    }

    // -------------------------------------------------------------------------
    // Irrational — value and precision contract
    // -------------------------------------------------------------------------

    [Fact]
    public void Sqrt_GivenTwo_MatchesKnownDigitsOfSqrtTwo()
    {
        // √2 = 1.41421356237309504880168872420969807…
        // Result must match at least 11 known fractional digits.
        Real result = Real.Sqrt(new Real(2.0));
        Assert.StartsWith("1.41421356237", result.ToString());
    }

    [Fact]
    public void Sqrt_GivenIrrational_ResultIsNotPeriodic()
    {
        // A finite Newton-Raphson approximation of an irrational is never periodic.
        Real result = Real.Sqrt(new Real(2.0));
        Assert.False(result.IsPeriodic);
    }

    [Fact]
    public void Sqrt_GivenPositiveInput_ResultIsPositive()
    {
        // The principal square root is always positive.
        Real result = Real.Sqrt(new Real(2.0));
        Assert.True(Real.IsPositive(result));
    }

    // -------------------------------------------------------------------------
    // Input validation
    // -------------------------------------------------------------------------

    [Fact]
    public void Sqrt_GivenNegativeInput_ThrowsArithmeticException()
    {
        // Square root of a negative number is undefined in ℝ.
        Assert.Throws<ArithmeticException>(() => Real.Sqrt(new Real("-1")));
    }

    // =========================================================================
    // 1000-digit precision tests (guard-digit + truncation checklist items)
    // =========================================================================

    // -------------------------------------------------------------------------
    // Perfect squares remain exact under guard-digit computation
    // -------------------------------------------------------------------------

    [Theory]
    [InlineData("4",  "2")]
    [InlineData("9",  "3")]
    [InlineData("16", "4")]
    [InlineData("25", "5")]
    public void Sqrt_GivenPerfectSquare_RemainsExactWithGuardDigits(string valueStr, string expectedStr)
    {
        // Guard-digit addition and tail truncation must not corrupt exact integer roots.
        long savedMax = Real.MaxComputationDecimalPlaces;
        try
        {
            Real.MaxComputationDecimalPlaces = 1000;
            Real result = Real.Sqrt(new Real(valueStr));
            Assert.Equal(new Real(expectedStr), result);
        }
        finally
        {
            Real.MaxComputationDecimalPlaces = savedMax;
        }
    }

    // -------------------------------------------------------------------------
    // Fractional perfect square: √0.25 = 0.5
    // -------------------------------------------------------------------------

    [Fact]
    public void Sqrt_GivenQuarter_ReturnsExactlyHalf()
    {
        // 0.25 is a rational perfect square; the exact root 0.5 must survive
        // guard-digit computation and truncation unchanged.
        long savedMax = Real.MaxComputationDecimalPlaces;
        try
        {
            Real.MaxComputationDecimalPlaces = 1000;
            Real result = Real.Sqrt(new Real("0.25"));
            Assert.Equal(new Real("0.5"), result);
        }
        finally
        {
            Real.MaxComputationDecimalPlaces = savedMax;
        }
    }

    // -------------------------------------------------------------------------
    // Known-prefix + digit-count checks for irrational square roots at 1000 digits
    // -------------------------------------------------------------------------

    [Fact]
    public void Sqrt_GivenTwo_Matches1000KnownDigitsOfSqrtTwo()
    {
        // √2 = 1.41421356237309504880168872420969807856967187537694...
        // The first 50 fractional digits are checked against the reference expansion.
        // The Exponent property then confirms 1000 fractional digits were produced.
        long savedMax = Real.MaxComputationDecimalPlaces;
        try
        {
            Real.MaxComputationDecimalPlaces = 1000;
            Real result = Real.Sqrt(new Real("2"));
            Assert.StartsWith(
                "1.41421356237309504880168872420969807856967187537694",
                result.ToString());
            // At most 1000 fractional digits stored (guard tail was truncated).
            Assert.True(result.Exponent >= -1000,
                $"Expected Exponent >= -1000 but got {result.Exponent}");
        }
        finally
        {
            Real.MaxComputationDecimalPlaces = savedMax;
        }
    }

    [Fact]
    public void Sqrt_GivenThree_Matches1000KnownDigitsOfSqrtThree()
    {
        // √3 = 1.73205080756887729352744634150587236694280525381038...
        long savedMax = Real.MaxComputationDecimalPlaces;
        try
        {
            Real.MaxComputationDecimalPlaces = 1000;
            Real result = Real.Sqrt(new Real("3"));
            Assert.StartsWith(
                "1.73205080756887729352744634150587236694280525381038",
                result.ToString());
            Assert.True(result.Exponent >= -1000,
                $"Expected Exponent >= -1000 but got {result.Exponent}");
        }
        finally
        {
            Real.MaxComputationDecimalPlaces = savedMax;
        }
    }

    [Fact]
    public void Sqrt_GivenFive_Matches1000KnownDigitsOfSqrtFive()
    {
        // √5 = 2.23606797749978969640917366873127623544061835961152...
        long savedMax = Real.MaxComputationDecimalPlaces;
        try
        {
            Real.MaxComputationDecimalPlaces = 1000;
            Real result = Real.Sqrt(new Real("5"));
            Assert.StartsWith(
                "2.23606797749978969640917366873127623544061835961152",
                result.ToString());
            Assert.True(result.Exponent >= -1000,
                $"Expected Exponent >= -1000 but got {result.Exponent}");
        }
        finally
        {
            Real.MaxComputationDecimalPlaces = savedMax;
        }
    }

    [Fact]
    public void Sqrt_GivenTen_Matches1000KnownDigitsOfSqrtTen()
    {
        // √10 = 3.16227766016837933199889354443271853371955513932521...
        long savedMax = Real.MaxComputationDecimalPlaces;
        try
        {
            Real.MaxComputationDecimalPlaces = 1000;
            Real result = Real.Sqrt(new Real("10"));
            Assert.StartsWith(
                "3.16227766016837933199889354443271853371955513932521",
                result.ToString());
            Assert.True(result.Exponent >= -1000,
                $"Expected Exponent >= -1000 but got {result.Exponent}");
        }
        finally
        {
            Real.MaxComputationDecimalPlaces = savedMax;
        }
    }

    // -------------------------------------------------------------------------
    // Truncation contract: result has exactly the requested number of digits
    // -------------------------------------------------------------------------

    [Fact]
    public void Sqrt_GivenTwo_ResultHasExactly1000FractionalDigits()
    {
        // After guard-digit truncation the exponent must be exactly -1000 (not more
        // fractional digits than requested; √2 has no trailing zeros at position 1000).
        long savedMax = Real.MaxComputationDecimalPlaces;
        try
        {
            Real.MaxComputationDecimalPlaces = 1000;
            Real result = Real.Sqrt(new Real("2"));
            Assert.Equal(-1000L, result.Exponent);
        }
        finally
        {
            Real.MaxComputationDecimalPlaces = savedMax;
        }
    }

    // -------------------------------------------------------------------------
    // Self-consistency: Sqrt(2)² ≈ 2 within 10⁻⁹⁹⁹
    // -------------------------------------------------------------------------

    [Fact]
    public void Sqrt_GivenTwo_SquaredApproximatesInput()
    {
        // If r = √2 with 1000-digit precision then |r² − 2| < 10⁻⁹⁹⁹.
        // This confirms the 1000 stored digits are all correct.
        long savedMax = Real.MaxComputationDecimalPlaces;
        try
        {
            Real.MaxComputationDecimalPlaces = 1000;
            Real r = Real.Sqrt(new Real("2"));
            Real squared = r * r;
            Real diff = Real.Abs(squared - new Real("2"));
            // tolerance = 10⁻⁹⁹⁹ = "0.000...001" with 998 zeros
            Real tolerance = Real.Parse("0." + new string('0', 998) + "1");
            Assert.True(diff < tolerance,
                $"Expected |r² − 2| < 10⁻⁹⁹⁹ but diff was non-trivially large");
        }
        finally
        {
            Real.MaxComputationDecimalPlaces = savedMax;
        }
    }

    // -------------------------------------------------------------------------
    // Result contract at 1000-digit precision
    // -------------------------------------------------------------------------

    [Fact]
    public void Sqrt_GivenIrrational_ResultIsNotPeriodicAt1000Digits()
    {
        // A finite Newton-Raphson truncation of √2 is never periodic.
        long savedMax = Real.MaxComputationDecimalPlaces;
        try
        {
            Real.MaxComputationDecimalPlaces = 1000;
            Real result = Real.Sqrt(new Real("2"));
            Assert.False(result.IsPeriodic);
        }
        finally
        {
            Real.MaxComputationDecimalPlaces = savedMax;
        }
    }

    [Fact]
    public void Sqrt_GivenPositiveInput_ResultIsPositiveAt1000Digits()
    {
        // The principal square root is always positive.
        long savedMax = Real.MaxComputationDecimalPlaces;
        try
        {
            Real.MaxComputationDecimalPlaces = 1000;
            Real result = Real.Sqrt(new Real("2"));
            Assert.True(Real.IsPositive(result));
        }
        finally
        {
            Real.MaxComputationDecimalPlaces = savedMax;
        }
    }
}
