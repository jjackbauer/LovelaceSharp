using Lovelace.Real;

namespace Lovelace.Real.Tests;

/// <summary>
/// Guards the odd/even symmetry of <see cref="Real.Sin(Real)"/> / <see cref="Real.Cos(Real)"/>, and the
/// exactness of argument reduction for angles that are exact rational multiples of π.
///
/// <para>Regression history: the round-02 division scale-alignment fix made <c>Pi / 6</c> truncate at
/// exactly the requested number of fractional digits.  Argument reduction rebuilt a negative angle as
/// <c>2*Pi + value</c> at that scale, and <c>2*Pi - Pi/6</c> landed one unit in the last place above
/// <c>11*Pi/6</c> (<c>…2912</c> vs <c>…2911</c>), so the special-angle table's exact comparison missed
/// and <c>Sin(-(Pi/6))</c> fell through to the Taylor series, returning
/// <c>-0.4999…4484</c> instead of exactly <c>-0.5</c> — observed as
/// <c>Lovelace.Dsp.Tests.TrigTests.Sin_GivenNegativeAngle_IsNegated</c> failing.  Sin and Cos now apply
/// their symmetry to the magnitude, so every equality below is exact by construction at any precision.</para>
/// </summary>
public class RealTrigSymmetryTests
{
    /// <summary>π·num/den for the multiples of π/6 and π/4, plus angles beyond one turn and a
    /// couple of non-special ones (the last exercise the Taylor path).</summary>
    private static IEnumerable<Real> Angles(Real pi)
    {
        (long Num, long Den)[] multiples =
        {
            (1, 6), (1, 4), (1, 3), (1, 2), (2, 3), (3, 4), (5, 6), (1, 1),
            (7, 6), (5, 4), (4, 3), (3, 2), (5, 3), (7, 4), (11, 6),
            (13, 6), (25, 4),
        };
        foreach ((long num, long den) in multiples)
            yield return num == 1 && den == 1 ? pi : pi * new Real(num.ToString()) / new Real(den.ToString());

        yield return Real.Parse("0.52359877559829887307710723054658381403286156656251763682915743205130273438103483310467247089035284466");
        yield return Real.Parse("1");
        yield return Real.Parse("2");
        yield return Real.Parse("7");
        yield return Real.Parse("0.5");
        yield return Real.Parse("-3");
    }

    [Fact]
    public void Sin_GivenNegativeAngle_IsExactlyTheNegatedSineOfItsMagnitude()
    {
        using var precision = Real.WithPrecision(60, 40);

        foreach (Real angle in Angles(Real.Pi))
            Assert.Equal(-Real.Sin(angle), Real.Sin(-angle));
    }

    [Fact]
    public void Cos_GivenNegativeAngle_IsExactlyTheCosineOfItsMagnitude()
    {
        using var precision = Real.WithPrecision(60, 40);

        foreach (Real angle in Angles(Real.Pi))
            Assert.Equal(Real.Cos(angle), Real.Cos(-angle));
    }

    [Fact]
    public void Sin_GivenNegativeSpecialAngles_KeepsTheirExactValues()
    {
        using var precision = Real.WithPrecision(60, 40);
        Real pi = Real.Pi;

        // The value the failing Dsp test pins, stated directly: sin(-pi/6) is exactly -1/2.
        Assert.Equal(new Real("-0.5"), Real.Sin(-(pi / new Real("6"))));
        Assert.Equal(-Real.Sqrt(new Real("2")) / new Real("2"), Real.Sin(-(pi / new Real("4"))));
        Assert.Equal(Real.Sqrt(new Real("3")) / new Real("2"), Real.Cos(-(pi / new Real("6"))));
        Assert.Equal(new Real("-1"), Real.Cos(-pi));
    }
}
