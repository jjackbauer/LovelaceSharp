using Lovelace.Real;

namespace Lovelace.Real.Tests;

/// <summary>
/// Functional tests for Option B of the Sqrt redesign:
/// "Fix seeding to use Exponent + leading BCD digits" — verifies that the Newton-Raphson
/// seed is computed from <c>value.ToNatural()</c> digits and <c>value.Exponent</c>
/// directly, eliminating the old 20-char <c>ToString()</c> truncation path that silently
/// collapsed very small values to 0.0, causing a fallback seed of 1.0 and divergent
/// Newton-Raphson iteration.
///
/// Without Option B the 20-char truncation of
/// <c>"0.0000000000000000000000000001"</c> (30 chars) yields
/// <c>"0.000000000000000000"</c> → 0.0 → fallback 1.0 → NR halves ~4 times
/// → result ≈ 0.0625 instead of 10⁻¹⁴.
/// Tests 6 and 7 are written so they FAIL without the fix and PASS after Option B is
/// applied.  Test 8 is a backward-compatibility regression check.
/// </summary>
public class RealSqrtSeedingTests
{
    // -------------------------------------------------------------------------
    // Test 6 — Very small number: correct convergence
    // sqrt(1×10⁻²⁸) = 1×10⁻¹⁴ = "0.00000000000001"
    // Without fix: seed=1.0 → NR halves ≈4 times → ≈0.0625 (completely wrong)
    // -------------------------------------------------------------------------

    [Fact]
    public void Sqrt_GivenVerySmallNumber_ConvergesCorrectly()
    {
        // 0.0000000000000000000000000001 = 1×10⁻²⁸; sqrt = 1×10⁻¹⁴.
        // "0.00000000000001" is the string representation of 1×10⁻¹⁴ (13 leading zeros
        // after the decimal point, then "1").
        // A divergent seed of 1.0 produces ≈0.0625 after 4 NR halvings — never starts
        // with "0.00000000000001".
        Real value = new Real("0.0000000000000000000000000001");
        Real result = Real.Sqrt(value);
        Assert.StartsWith("0.00000000000001", result.ToString());
    }

    // -------------------------------------------------------------------------
    // Test 7 — Very small number: result Exponent is correct order of magnitude
    // sqrt(1×10⁻²⁸) has Exponent = -14; the seed must place NR in the right
    // order of magnitude for the early-convergence check to fire immediately.
    // Without fix: seed=1.0 → result Exponent ≈ -1 or -2 (many orders off)
    // -------------------------------------------------------------------------

    [Fact]
    public void Sqrt_GivenVerySmallNumber_ExponentIsCorrectOrderOfMagnitude()
    {
        // The Exponent property of a Real(Nat=1, isNeg=false, Exponent=-14)
        // is exactly -14.  For sqrt(1e-28) the correct seed is 1e-14, which
        // causes x*x == value after one NR step, hitting the early-exit path and
        // returning a Real whose Exponent is exactly -14.
        // With the broken seed of 1.0, the result would have Exponent ≈ -1 (≈ 0.0625),
        // not -14.
        Real value = new Real("0.0000000000000000000000000001");
        Real result = Real.Sqrt(value);
        Assert.Equal(-14L, result.Exponent);
    }

    // -------------------------------------------------------------------------
    // Test 8 — Backward-compatibility: normal input (Real("2")) is unaffected
    // New seeding must produce the same NR convergence result as the old path
    // for values well within double's representable range.
    // -------------------------------------------------------------------------

    [Fact]
    public void Sqrt_GivenNormalInput_NewSeedingProducesSameResult()
    {
        // sqrt(2) = 1.41421356237309504880168872…
        // Both the old seed (from double.Parse("2")) and the new seed (from
        // natDigits="2", exp10=0, seedStr="1.4142135623731") converge to the same
        // Newton-Raphson fixed point.  The result must start with the known prefix.
        Real result = Real.Sqrt(new Real("2"));
        Assert.StartsWith("1.41421356237", result.ToString());
    }
}
