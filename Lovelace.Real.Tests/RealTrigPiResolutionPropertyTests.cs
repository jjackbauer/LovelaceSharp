using Lovelace.Real;
using Xunit;

using Int = Lovelace.Integer.Integer;

namespace Lovelace.Real.Tests;

/// <summary>
/// Properties of the trigonometry of arguments that carry MORE places than the ambient computation
/// budget — the family the differential audit probed with <c>pi(100)</c> under the wire's 30-digit
/// plugin budget.
///
/// <para>The defect these pin: the reduction was done against <see cref="Real.Pi"/> at the ambient
/// precision, so the residual it measured was <c>argument − multiple·π_ambient</c>.  When the
/// argument is a multiple of a FINER truncation of π, that residual is dominated by
/// <c>π − π_ambient</c> (order <c>10^-30</c>) instead of by the argument's own distance to a
/// multiple of π (order <c>10^-100</c>), and the sign comes out of the artifact:
/// <c>sin(pi(100)·2)</c> returned <c>+1e-30</c> where the value is <c>−1.64e-100</c>, and
/// <c>cos(pi(100)/2)</c> came back at that same scale instead of <c>+4.11e-101</c> (which is why
/// the tangent of that angle was refused as a division by zero).</para>
///
/// <para>The expected values come from π itself, not from the implementation: with
/// <c>P = π truncated at d places</c> and <c>δ = π − P</c> (computed from a π that reaches many
/// places further down), <c>sin(k·P) = (−1)^(k+1)·sin(k·δ)</c> and <c>cos(P/2) = sin(δ/2)</c>; for
/// a rational multiple <c>m/n</c> the exact table value is at most <c>(m/n)·δ</c> away from
/// <c>sin(m·P/n)</c>, because the sine is 1-Lipschitz.</para>
/// </summary>
public class RealTrigPiResolutionPropertyTests
{
    /// <summary>The wire's plugin budget: the ambient precision this family is evaluated under.</summary>
    private const long Ambient = 30L;

    /// <summary>The special angles as (numerator, denominator), the 16 rows of the fast-path table.</summary>
    private static readonly (long Num, long Den)[] SpecialAngles =
    {
        (0, 1), (1, 6), (1, 4), (1, 3), (1, 2), (2, 3), (3, 4), (5, 6),
        (1, 1), (7, 6), (5, 4), (4, 3), (3, 2), (5, 3), (7, 4), (11, 6),
    };

    /// <summary>10^-<paramref name="places"/> as an exact Real literal.</summary>
    private static Real Tiny(long places) =>
        Real.Parse("0." + new string('0', (int)places - 1) + "1");

    /// <summary>π truncated at <paramref name="places"/> places, built under a scope wide enough
    /// for the request whatever the ambient cap happens to be.</summary>
    private static Real PiAt(long places)
    {
        using var scope = Real.WithPrecision(places, places);
        return Real.PiTo(places);
    }

    /// <summary>The 16 exact sin/cos pairs, built at <paramref name="places"/> precision.</summary>
    private static (Real Sin, Real Cos)[] SpecialValues(long places)
    {
        using var scope = Real.WithPrecision(places, places);
        Real half = new Real("0.5");
        Real negHalf = new Real("-0.5");
        Real negOne = new Real("-1");
        Real sqrt2Half = Real.Sqrt(new Real("2")) / new Real("2");
        Real sqrt3Half = Real.Sqrt(new Real("3")) / new Real("2");

        return new (Real, Real)[]
        {
            (Real.Zero, Real.One),
            (half, sqrt3Half),
            (sqrt2Half, sqrt2Half),
            (sqrt3Half, half),
            (Real.One, Real.Zero),
            (sqrt3Half, negHalf),
            (sqrt2Half, -sqrt2Half),
            (half, -sqrt3Half),
            (Real.Zero, negOne),
            (negHalf, -sqrt3Half),
            (-sqrt2Half, -sqrt2Half),
            (-sqrt3Half, negHalf),
            (negOne, Real.Zero),
            (-sqrt3Half, half),
            (-sqrt2Half, sqrt2Half),
            (negHalf, sqrt3Half),
        };
    }

    /// <summary>
    /// <c>sin(k·P)</c> for <c>P = pi(d)</c> and <c>k = 1..12</c>, measured against
    /// <c>(−1)^(k+1)·k·δ</c>.  Every case must land within <c>10^-(d+5)</c> of that value: the
    /// family carries d places, so nothing in it may move at the 10^-30 scale of a coarser π.
    /// </summary>
    [Theory]
    [InlineData(40L)]
    [InlineData(100L)]
    [InlineData(200L)]
    public void Sin_IntegerMultiplesOfPiTruncatedAtDPlaces_ReproduceTheDistanceToPi(long places)
    {
        long high = places + 100L;
        Real pi = PiAt(places);
        Real delta;
        using (var scope = Real.WithPrecision(high, high))
            delta = Real.PiTo(high) - pi;

        Real tolerance = Tiny(places + 5L);

        using var ambient = Real.WithPrecision(Ambient, Ambient);
        for (long k = 1; k <= 12; k++)
        {
            Real expected = delta * new Real(new Int(k));
            if (k % 2 == 0)
                expected = -expected;

            Real actual = Real.Sin(pi * new Real(new Int(k)), Ambient);

            Assert.True(Real.Abs(actual - expected) < tolerance,
                $"sin(pi({places})*{k}) at {Ambient} places = {actual}, expected {expected} " +
                $"(|error| = {Real.Abs(actual - expected)}, tolerance {tolerance})");
        }
    }

    /// <summary>
    /// <c>cos(P/2)</c> for <c>P = pi(d)</c> is <c>sin(δ/2)</c>, a tiny POSITIVE number — the value
    /// the tangent of that angle divides by.  A reduction against a coarser π answers with an
    /// artifact of that coarser π instead (zero, or one unit in its last place), which is what made
    /// <c>tan(pi(100)/2)</c> divide by zero.
    /// </summary>
    [Theory]
    [InlineData(40L)]
    [InlineData(100L)]
    [InlineData(200L)]
    public void Cos_OfHalfPiTruncatedAtDPlaces_IsTheHalfDistanceToPi(long places)
    {
        long high = places + 100L;
        Real pi = PiAt(places);
        Real halfPi = Real.Divide(pi, new Real("2"));
        Real delta;
        using (var scope = Real.WithPrecision(high, high))
            delta = (Real.PiTo(high) - pi) / new Real("2");

        Real tolerance = Tiny(places + 5L);

        using var ambient = Real.WithPrecision(Ambient, Ambient);
        Real actual = Real.Cos(halfPi, Ambient);

        Assert.True(actual > Real.Zero,
            $"cos(pi({places})/2) = {actual} must be positive (π/2 is approached from below)");
        Assert.True(Real.Abs(actual - delta) < tolerance,
            $"cos(pi({places})/2) at {Ambient} places = {actual}, expected {delta}");
    }

    /// <summary>
    /// Rational multiples of a finer π: <c>sin(m·P/n)</c> and <c>cos(m·P/n)</c> must land on the
    /// exact value of that angle — the audit's artifacts sat ONE UNIT IN THE AMBIENT LAST PLACE
    /// (≈4.4e-31 at 30 digits) away from angles whose values are exact, so the budget asserted here
    /// is <c>10^-(Ambient+5)</c>, which the series' own truncation clears by five places and the
    /// artifacts do not.
    /// </summary>
    /// <remarks>
    /// The angle is built as a NON-periodic decimal truncated well past the ambient budget (a
    /// division inside the ambient scope would itself be truncated to it, and a periodic argument
    /// takes a different path through the series).  The truncation moves the angle by less than its
    /// own last place, far below the budget asserted here.
    /// </remarks>
    [Theory]
    [InlineData(50L)]
    [InlineData(100L)]
    [InlineData(200L)]
    public void SinAndCos_RationalMultiplesOfPiTruncatedAtDPlaces_StayOnTheExactAngle(long places)
    {
        long high = places + 100L;
        Real pi = PiAt(places);
        (Real, Real)[] values = SpecialValues(high);
        Real tolerance = Tiny(Ambient + 5L);

        using var ambient = Real.WithPrecision(Ambient, Ambient);
        for (int i = 0; i < SpecialAngles.Length; i++)
        {
            (long num, long den) = SpecialAngles[i];
            if (num == 0)
                continue;

            Real angle = Real.DivideNonPeriodic(pi * new Real(new Int(num)), new Real(new Int(den)), high);
            Real expectedSin = values[i].Item1;
            Real expectedCos = values[i].Item2;

            Real sin = Real.Sin(angle, Ambient);
            Real cos = Real.Cos(angle, Ambient);

            Real sinError = Real.Abs(sin - expectedSin);
            Real cosError = Real.Abs(cos - expectedCos);

            Assert.True(sinError < tolerance,
                $"sin(pi({places})*{num}/{den}) at {Ambient} places = {sin}, exact value {expectedSin}, |error| {sinError}, budget {tolerance}");
            Assert.True(cosError < tolerance,
                $"cos(pi({places})*{num}/{den}) at {Ambient} places = {cos}, exact value {expectedCos}, |error| {cosError}, budget {tolerance}");
        }
    }

    /// <summary>
    /// The other half of the contract, and the reason the finer reduction is scoped to arguments
    /// that carry more places than the ambient π does: an exact rational multiple of the ambient π
    /// itself still hits the special-angle table and returns its exact value at every precision —
    /// <c>sin(π) = 0</c>, <c>cos(π) = −1</c>, <c>sin(π/6) = 1/2</c>.  Nothing in this family may
    /// drift.
    /// </summary>
    [Theory]
    [InlineData(20L)]
    [InlineData(50L)]
    [InlineData(100L)]
    public void SinAndCos_ExactMultiplesOfTheAmbientPi_KeepTheirExactTableValues(long places)
    {
        using var scope = Real.WithPrecision(places, places);
        Real pi = Real.Pi;
        (Real, Real)[] values = SpecialValues(places);

        for (int i = 0; i < SpecialAngles.Length; i++)
        {
            (long num, long den) = SpecialAngles[i];
            Real angle = num == 0 ? Real.Zero : pi * new Real(new Int(num)) / new Real(new Int(den));

            Real sin = Real.Sin(angle);
            Real cos = Real.Cos(angle);

            Assert.True(sin == values[i].Item1,
                $"sin(pi*{num}/{den}) at {places} places = {sin} (exact {sin.IsExact}), expected {values[i].Item1}");
            Assert.True(cos == values[i].Item2,
                $"cos(pi*{num}/{den}) at {places} places = {cos} (exact {cos.IsExact}), expected {values[i].Item2}");

            // The table's rational entries are exact values; the sqrt-based ones are the same
            // inexact root constants every other caller gets, and their provenance must say so.
            Assert.Equal(values[i].Item1.IsExact, sin.IsExact);
            Assert.Equal(values[i].Item2.IsExact, cos.IsExact);
        }
    }
}
