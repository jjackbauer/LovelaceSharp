using Lovelace.Real;

namespace Lovelace.Real.Tests;

/// <summary>
/// Functional tests for Option A of the Sqrt redesign:
/// "Pre-expand periodic input in Sqrt" — verifies that Sqrt correctly handles
/// periodic Real inputs by expanding them before seeding and iterating.
///
/// Without Option A, <c>Sqrt(Real("0.(3)"))</c> seeds from <c>double.TryParse("0.(3)")</c>
/// which fails (parentheses invalid), falls back to seed=1.0, and then iterates using
/// the stored value 0.3 (Nat="3", Exponent=-1) — producing sqrt(0.3)≈0.5477 instead of
/// sqrt(1/3)≈0.57735.  All tests below are written so they FAIL without the fix and
/// PASS after Option A is applied.
/// </summary>
public class RealSqrtPeriodicTests
{
    // -------------------------------------------------------------------------
    // Test 1 — sqrt(0.(1)) = sqrt(1/9) = 1/3 ≈ 0.3333…
    // Without fix: converges to sqrt(0.1) ≈ 0.3162… (distinctly wrong)
    // -------------------------------------------------------------------------

    [Fact]
    public void Sqrt_GivenOneNinthAsPeriodic_ReturnsExactlyOneThird()
    {
        // 0.(1) = 0.111… = 1/9; sqrt(1/9) = 1/3 = 0.333…
        // A broken seed landing on sqrt(0.1)≈0.316 cannot start with "0.3333333".
        long saved = Real.MaxComputationDecimalPlaces;
        try
        {
            Real.MaxComputationDecimalPlaces = 50;
            Real result = Real.Sqrt(new Real("0.(1)"));
            Assert.StartsWith("0.3333333", result.ToString());
        }
        finally
        {
            Real.MaxComputationDecimalPlaces = saved;
        }
    }

    // -------------------------------------------------------------------------
    // Test 2 — sqrt(0.(4)) = sqrt(4/9) = 2/3 ≈ 0.6666…
    // Without fix: converges to sqrt(0.4) ≈ 0.6324… (distinctly wrong)
    // -------------------------------------------------------------------------

    [Fact]
    public void Sqrt_GivenFourNinthsAsPeriodic_ReturnsExactlyTwoThirds()
    {
        // 0.(4) = 0.444… = 4/9; sqrt(4/9) = 2/3 = 0.666…
        // A broken computation on 0.4 gives ≈0.6324, which does not start with "0.6666666".
        long saved = Real.MaxComputationDecimalPlaces;
        try
        {
            Real.MaxComputationDecimalPlaces = 50;
            Real result = Real.Sqrt(new Real("0.(4)"));
            Assert.StartsWith("0.6666666", result.ToString());
        }
        finally
        {
            Real.MaxComputationDecimalPlaces = saved;
        }
    }

    // -------------------------------------------------------------------------
    // Test 3 — sqrt(0.(3)) = sqrt(1/3) ≈ 0.57735026918…  (KEY discriminating test)
    // Without fix: converges to sqrt(0.3) ≈ 0.54772255750… — differs at digit 2
    // -------------------------------------------------------------------------

    [Fact]
    public void Sqrt_GivenOneThirdAsPeriodic_MatchesSqrtOneThirdKnownDigits()
    {
        // 0.(3) = 0.333… = 1/3; sqrt(1/3) = 1/√3 ≈ 0.57735026918962576450914…
        // Without Option A the result starts with "0.5477" (sqrt(0.3)) — clearly wrong.
        long saved = Real.MaxComputationDecimalPlaces;
        try
        {
            Real.MaxComputationDecimalPlaces = 50;
            Real result = Real.Sqrt(new Real("0.(3)"));
            Assert.StartsWith("0.57735026", result.ToString());
        }
        finally
        {
            Real.MaxComputationDecimalPlaces = saved;
        }
    }

    // -------------------------------------------------------------------------
    // Test 4 — periodic form and 10-digit finite expansion agree to 8 frac digits
    // Both should start with "0.57735026" after the fix.
    // Without fix: periodic gives ≈0.5477, expanded gives ≈0.57735 — they disagree.
    // -------------------------------------------------------------------------

    [Fact]
    public void Sqrt_GivenPeriodicAndExpandedEquivalent_ProduceSameResult()
    {
        // sqrt(0.(3)) and sqrt(0.3333333333) are mathematical expressions of the same
        // value (the finite one is 10^-10 smaller), so their square roots agree at
        // least to 8 fractional digits.  Both should start with "0.57735026".
        long saved = Real.MaxComputationDecimalPlaces;
        try
        {
            Real.MaxComputationDecimalPlaces = 50;
            string periodicResult  = Real.Sqrt(new Real("0.(3)")).ToString();
            string expandedResult  = Real.Sqrt(new Real("0.3333333333")).ToString();

            Assert.StartsWith("0.57735026", periodicResult);
            Assert.StartsWith("0.57735026", expandedResult);
        }
        finally
        {
            Real.MaxComputationDecimalPlaces = saved;
        }
    }

    // -------------------------------------------------------------------------
    // Test 5 — sqrt(0.(2)) = sqrt(2/9) = √2/3 ≈ 0.47140452079103…
    // Without fix: converges to sqrt(0.2) ≈ 0.44721… (distinctly wrong)
    // -------------------------------------------------------------------------

    [Fact]
    public void Sqrt_GivenTwoNinthsAsPeriodic_MatchesSqrtTwoNinthsKnownDigits()
    {
        // 0.(2) = 0.222… = 2/9; sqrt(2/9) = sqrt(2)/3 ≈ 0.47140452079103…
        // A broken seed on 0.2 gives sqrt(0.2)≈0.44721 — cannot start with "0.47140".
        long saved = Real.MaxComputationDecimalPlaces;
        try
        {
            Real.MaxComputationDecimalPlaces = 50;
            Real result = Real.Sqrt(new Real("0.(2)"));
            Assert.StartsWith("0.47140452", result.ToString());
        }
        finally
        {
            Real.MaxComputationDecimalPlaces = saved;
        }
    }
}
