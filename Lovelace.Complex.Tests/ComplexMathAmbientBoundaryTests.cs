using Lovelace.Complex;
using Lovelace.Real;
using Xunit;

using Rl = Lovelace.Real.Real;

namespace Lovelace.Complex.Tests;

/// <summary>
/// The ambient-precision boundary (cycle 6, round 13, audit B L2 / P0): an argument that is a
/// COARSER truncation of π than the ambient precision carries must still be resolved against a π
/// that reaches below the argument.
///
/// <para>Ground truth is mpmath 1.3.0 at 2400 dps (oracle run recorded in
/// <c>docs/goal-cycle-6/round-17/p0b-implementation.md</c>):</para>
/// <code>
/// sin(pi(30)) = 5.0288419716939937510582097494459230781640628620899862803482532092112635556795782e-31
/// </code>
/// <para>so at 31 decimal places the value truncates to <c>5e-31</c>, at 32 to <c>5.0e-31</c> and at
/// 60 to <c>5.02884197169399375105820974944e-31</c>.  The engine published <c>4e-31</c>, <c>4.9e-31</c>
/// and <c>5.02884197169399375105820974943e-31</c>: at 31 places the LEADING significant digit was
/// wrong, because the residual was resolved against the ambient (31-place) π, which leaves the
/// exactly-one-digit value <c>5e-31</c>; the series' own correction (<c>r³/6 ≈ 2.1e-92</c>) then
/// borrows the last stored digit of that value and the truncation to 31 places lands on 4.</para>
///
/// <para>The controls below are the two shapes whose values must NOT move: an argument that IS an
/// exact multiple of the ambient π keeps reducing exactly to zero, and an argument that is a FINER
/// truncation of π keeps answering its own distance to π (cycle 5).</para>
/// </summary>
public class ComplexMathAmbientBoundaryTests
{
    private static string Zeros(int count) => new string('0', count);

    /// <summary>π truncated at <paramref name="places"/> decimal places — the wire's <c>pi(d)</c>.</summary>
    private static Rl PiAt(long places)
    {
        using var scope = Rl.WithPrecision(places, places);
        return Rl.PiTo(places);
    }

    /// <summary>
    /// <c>sin(pi(30))</c> under four ambient precisions, against the mpmath digits.  At ambient 30
    /// the argument IS the ambient π — the same truncation, so the reduction is exact and the value
    /// is 0 (the honest boundary: every digit of <c>5.0288…e-31</c> lies below that window).  At 31,
    /// 32 and 60 the argument is a DIFFERENT truncation of π and the digits the engine prints must be
    /// mpmath's.
    /// </summary>
    [Fact]
    public void Sin_OfPiAt30Places_PrintsTheMpmathDigitsAtTheAmbientPrecision()
    {
        Rl x = PiAt(30);

        (long ambient, string expected, string printed)[] cases =
        {
            (30L, "0", "0"),
            (31L, "0." + Zeros(30) + "5", "0." + Zeros(30) + "5"),
            // The value is 5.0e-31, whose trailing zero the engine does not print: the 32nd digit is
            // pinned by the value, not by a character the canonical form drops.
            (32L, "0." + Zeros(30) + "50", "0." + Zeros(30) + "5"),
            (60L, "0." + Zeros(30) + "502884197169399375105820974944",
                  "0." + Zeros(30) + "502884197169399375105820974944"),
        };

        foreach ((long ambient, string expected, string printed) in cases)
        {
            using var scope = Rl.WithPrecision(ambient, ambient);

            Rl actual = ComplexMath.Sin(x);
            Rl expectedValue = Rl.Parse(expected);

            Assert.True(Rl.IsZero(actual - expectedValue),
                $"sin(pi(30)) at {ambient} places = {actual}, expected {expectedValue} (mpmath)");
            Assert.StartsWith(printed, actual.ToString());
        }
    }

    /// <summary>
    /// The pinned exact-multiple control: the ambient π reduces to exactly zero against itself, so
    /// <c>Sin(π) == 0</c> and <c>Cos(π) == −1</c> as VALUES at every precision — the values
    /// <c>ComplexMathProvenanceTests.ExactResults_KeepTheirExactness</c> pins at one budget, here at
    /// the four budgets this family lives on.
    /// </summary>
    [Theory]
    [InlineData(30L)]
    [InlineData(31L)]
    [InlineData(32L)]
    [InlineData(60L)]
    public void SinAndCos_OfTheAmbientPi_StayTheExactTableValues(long ambient)
    {
        using var scope = Rl.WithPrecision(ambient, ambient);

        Rl sin = ComplexMath.Sin(Rl.Pi);
        Rl cos = ComplexMath.Cos(Rl.Pi);

        Assert.True(Rl.IsZero(sin), $"sin(pi) at {ambient} places = {sin}, expected 0");
        Assert.Equal(Rl.Zero, sin);
        Assert.Equal(-Rl.One, cos);
    }

    /// <summary>
    /// The cycle-5 pin: an argument FINER than the ambient π answers its own distance to π.
    /// <c>sin(2·pi(100))</c> at the 30-place budget is <c>−1.64296…e-100</c>, not 0 and not the
    /// <c>1e-30</c> artifact a 30-place π would measure.
    /// </summary>
    [Fact]
    public void Sin_OfATwoPiTruncationFinerThanTheAmbientPrecision_KeepsItsResidual()
    {
        Rl x = PiAt(100) * new Rl("2");

        using var scope = Rl.WithPrecision(30, 30);

        Rl actual = ComplexMath.Sin(x);
        Rl expected = Rl.Parse("-0." + Zeros(99) +
            "164296173026564613294187689219101164463450718816256962234900568205403877042111192892458979098607639288576219513318668922569512965");

        Assert.False(Rl.IsZero(actual), $"sin(2*pi(100)) at 30 places = 0, expected {expected}");
        Assert.True(Rl.Abs(actual - expected) < Rl.Parse("0." + Zeros(104) + "1"),
            $"sin(2*pi(100)) at 30 places = {actual}, expected {expected}");
        Assert.StartsWith("-0." + Zeros(99) + "16429617302656461329", actual.ToString());
    }
}
