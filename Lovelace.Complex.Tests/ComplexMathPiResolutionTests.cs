using Lovelace.Complex;
using Lovelace.Real;
using Xunit;

using Int = Lovelace.Integer.Integer;
using Rl = Lovelace.Real.Real;

namespace Lovelace.Complex.Tests;

/// <summary>
/// Properties of the trigonometry of arguments that carry MORE places than the ambient computation
/// budget — the family the wire probes with <c>pi(100)</c> under the symbolic plugin's 30-place
/// budget.
///
/// <para>The defect these pin: <c>SinCos</c> resolved π at the ambient precision, so the reduction
/// measured <c>argument − k·π_ambient</c>.  For an argument built from a FINER truncation of π that
/// residual is dominated by <c>π − π_ambient</c> (order 1e-30) instead of by the argument's own
/// distance to a multiple of π (order 1e-100), and the sign comes out of the artifact:
/// <c>sin(pi(100)·2)</c> was <c>+1.005768394338798750211e-30</c> where the value is
/// <c>−1.642961730265646132941e-100</c>.  <c>Tan</c> additionally divided the ALREADY rounded pair,
/// so a cosine that is genuinely tiny (4.107404325664115332354e-101) became zero and
/// <c>tan(pi(100)/2)</c> was refused as a division by zero although the value is ≈ 2.43e100.</para>
///
/// <para>The expected values come from π itself: with <c>P = π truncated at d places</c> and
/// <c>δ = π − P</c> (from a π that reaches further down), <c>sin(k·P) = (−1)^(k+1)·sin(k·δ)</c> and
/// <c>cos(P/2) = sin(δ/2)</c>.</para>
/// </summary>
public class ComplexMathPiResolutionTests
{
    /// <summary>The symbolic plugin's budget: the ambient precision this family is evaluated under.</summary>
    private const long Ambient = 30L;

    private static Rl Tiny(long places) =>
        Rl.Parse("0." + new string('0', (int)places - 1) + "1");

    private static Rl PiAt(long places)
    {
        using var scope = Rl.WithPrecision(places, places);
        return Rl.PiTo(places);
    }

    /// <summary>
    /// <c>sin(k·P)</c> for <c>P = pi(d)</c>: every k = 1..6 must land within
    /// <c>10^-(d+5)</c> of <c>(−1)^(k+1)·k·δ</c>, not at the 1e-30 scale of a coarser π.
    /// </summary>
    [Theory]
    [InlineData(40L)]
    [InlineData(100L)]
    [InlineData(200L)]
    public void Sin_IntegerMultiplesOfPiTruncatedAtDPlaces_ReproduceTheDistanceToPi(long places)
    {
        long high = places + 100L;
        Rl pi = PiAt(places);
        Rl delta;
        using (var scope = Rl.WithPrecision(high, high))
            delta = Rl.PiTo(high) - pi;

        Rl tolerance = Tiny(places + 5L);

        using var ambient = Rl.WithPrecision(Ambient, Ambient);
        for (long k = 1; k <= 6; k++)
        {
            Rl expected = delta * new Rl(new Int(k));
            if (k % 2 == 0)
                expected = -expected;

            Rl actual = ComplexMath.Sin(pi * new Rl(new Int(k)));

            Assert.True(Rl.Abs(actual - expected) < tolerance,
                $"sin(pi({places})*{k}) at {Ambient} places = {actual}, expected {expected}");
        }
    }

    /// <summary>
    /// <c>cos(P/2)</c> is <c>sin(δ/2)</c>, a tiny POSITIVE number — and <c>tan(P/2)</c> is its
    /// reciprocal, ≈ 2.43e100, not a division by zero.  This is the pair the audit's
    /// <c>tan(pi(100)/2)</c> probe needed.
    /// </summary>
    [Theory]
    [InlineData(40L)]
    [InlineData(100L)]
    [InlineData(200L)]
    public void Tan_OfHalfPiTruncatedAtDPlaces_IsTheReciprocalOfTheHalfDistanceToPi(long places)
    {
        long high = places + 100L;
        Rl pi = PiAt(places);
        Rl halfPi = Rl.Divide(pi, new Rl("2"));
        Rl halfDelta;
        using (var scope = Rl.WithPrecision(high, high))
            halfDelta = (Rl.PiTo(high) - pi) / new Rl("2");

        Rl tolerance = Tiny(places + 5L);

        using var ambient = Rl.WithPrecision(Ambient, Ambient);
        Rl cos = ComplexMath.Cos(halfPi);
        Rl tan = ComplexMath.Tan(halfPi);

        Assert.True(cos > Rl.Zero, $"cos(pi({places})/2) = {cos} must be positive");
        Assert.True(Rl.Abs(cos - halfDelta) < tolerance,
            $"cos(pi({places})/2) at {Ambient} places = {cos}, expected {halfDelta}");
        // Both factors are themselves rounded to the ambient precision, so the product is 1 to
        // about 10^-Ambient rather than to the tolerance a single value would need.
        Assert.True(Rl.Abs(tan * cos - Rl.One) < Tiny(Ambient - 3L),
            $"tan(pi({places})/2) * cos(pi({places})/2) = {tan * cos}, expected 1");
        Assert.True(tan > Rl.One, $"tan(pi({places})/2) = {tan}");
    }

    /// <summary>
    /// The other half of the contract: an exact multiple of the AMBIENT π still reduces exactly, so
    /// <c>sin(π) = 0</c>, <c>cos(π) = −1</c> and <c>sin(π/2) = 1</c> at every precision — and those
    /// are the values that must not drift.
    /// </summary>
    [Theory]
    [InlineData(20L)]
    [InlineData(50L)]
    [InlineData(100L)]
    public void SinCos_ExactMultiplesOfTheAmbientPi_KeepTheirExactValues(long places)
    {
        using var scope = Rl.WithPrecision(places, places);
        Rl pi = Rl.Pi;

        // Value only: reducing an exact multiple of the ambient π multiplies by that π, so the
        // zero and the one it lands on keep the approximation's provenance, as everywhere else in
        // the library (Divide(0, inexact) is an inexact zero too).
        Assert.Equal(Rl.Zero, ComplexMath.Sin(pi));
        Assert.Equal(-Rl.One, ComplexMath.Cos(pi));
        Assert.True(Rl.Abs(ComplexMath.Sin(pi / new Rl("2")) - Rl.One) < Tiny(places - 1L),
            $"sin(pi/2) at {places} places = {ComplexMath.Sin(pi / new Rl("2"))}");
        Assert.True(Rl.Abs(ComplexMath.Cos(pi / new Rl("3")) - new Rl("0.5")) < Tiny(places - 1L),
            $"cos(pi/3) at {places} places = {ComplexMath.Cos(pi / new Rl("3"))}");
        Assert.True(Rl.Abs(ComplexMath.Tan(pi / new Rl("4")) - Rl.One) < Tiny(places - 1L),
            $"tan(pi/4) at {places} places = {ComplexMath.Tan(pi / new Rl("4"))}");
    }
}
