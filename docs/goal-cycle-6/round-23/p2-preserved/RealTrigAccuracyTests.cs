using Lovelace.Real;
using Xunit;

namespace Lovelace.Real.Tests;

/// <summary>
/// Audit P (wave 5, round 23, finding P-2): <see cref="Real.Sin(Real, long)"/> and
/// <see cref="Real.Cos(Real, long)"/> returned FALSE PRECISION. Their Taylor series is summed as exact
/// rationals, so the partial sum's decimal expansion runs on for hundreds of digits of which only the
/// first <c>digits + 10</c> (the internal guard) are digits of sin/cos; the rest is the exact tail of
/// an approximation. Measured by the orchestrator on the harness the persona left behind
/// (<c>a1.txt</c>): <c>Real.Sin(1)</c> at precision 30 printed <b>607</b> fractional digits of which
/// <b>40</b> matched mpmath's <c>sin(1)</c> to 1900 dps — the mismatch starting exactly at the guard.
/// The sibling implementation <c>ComplexMath.Sin(Complex)</c> printed 30 digits and the CLI answered
/// 30, so the library was the outlier. These tests pin the contract the summary states ("Computes
/// sin(x) to <paramref name="digits"/> decimal places") for the series path, and pin the exact
/// special-angle path unchanged.
/// </summary>
public class RealTrigAccuracyTests
{
    /// <summary>Truncated, per the product's own convention (audit P's P-3 records the same for the
    /// last requested digit), so equality is on the PREFIX here, not on a rounded value.</summary>
    private const string SinOne30 = "0.841470984807896506652502321630";
    private const string CosOne30 = "0.540302305868139717400936607442";

    private static string FractionalDigits(Real value)
    {
        string text = value.ToString();
        int dot = text.IndexOf('.');
        return dot < 0 ? "" : text.Substring(dot + 1);
    }

    [Fact]
    public void Sin_OfAGeneralAngle_CarriesNoMoreThanTheRequestedFractionalDigits()
    {
        using var precision = Real.WithPrecision(30, 30);

        Real value = Real.Sin(new Real("1"), 30);

        Assert.True(FractionalDigits(value).Length <= 30,
            $"Real.Sin(1, 30) printed {FractionalDigits(value).Length} fractional digits: {value}");
        Assert.StartsWith(SinOne30, value.ToString());
    }

    [Fact]
    public void Cos_OfAGeneralAngle_CarriesNoMoreThanTheRequestedFractionalDigits()
    {
        using var precision = Real.WithPrecision(30, 30);

        Real value = Real.Cos(new Real("1"), 30);

        Assert.True(FractionalDigits(value).Length <= 30,
            $"Real.Cos(1, 30) printed {FractionalDigits(value).Length} fractional digits: {value}");
        Assert.StartsWith(CosOne30, value.ToString());
    }

    /// <summary>The whole point of the finding: the digits PAST the requested precision were not
    /// digits of the function at all, so a caller could not tell correct from invented ones. At 50 the
    /// value must stop at 50 too — the guard is internal and must not leak into the value.</summary>
    [Fact]
    public void Sin_AtFiftyPlaces_AlsoStopsAtFifty()
    {
        using var precision = Real.WithPrecision(50, 50);

        Real value = Real.Sin(new Real("1"), 50);

        Assert.True(FractionalDigits(value).Length <= 50,
            $"Real.Sin(1, 50) printed {FractionalDigits(value).Length} fractional digits: {value}");
        // Truncation, so the printed value is a PREFIX of the true 50-place expansion. Ground truth
        // from mpmath at 120 dps: 0.8414709848078965066525023216302989996225630607983710657…
        // (The pre-fix value agreed only to the internal guard and then ran on with the exact tail of
        // the Taylor rational: …99622626472453442728…, i.e. digits 41+ were NOT sin(1)'s.)
        Assert.StartsWith(value.ToString(), "0.84147098480789650665250232163029899962256306079837");
    }

    /// <summary>A smaller request must still be honoured exactly (the truncation is to the CALLER's
    /// precision, not to the ambient one).</summary>
    [Fact]
    public void Sin_AtATenthOfTheAmbientPrecision_StopsAtTen()
    {
        using var precision = Real.WithPrecision(60, 60);

        Real value = Real.Sin(new Real("1"), 10);

        Assert.True(FractionalDigits(value).Length <= 10,
            $"Real.Sin(1, 10) printed {FractionalDigits(value).Length} fractional digits: {value}");
        Assert.Equal("0.8414709848", value.ToString());
    }

    // ---- controls: the exact paths must NOT move ----------------------------------------------

    /// <summary>Special angles never reach the series, so their exactness is untouched.</summary>
    [Fact]
    public void SpecialAngles_AreStillExact()
    {
        using var precision = Real.WithPrecision(30, 30);

        Assert.Equal(new Real("0.5"), Real.Sin(Real.Pi / new Real("6"), 30));
        Assert.Equal(new Real("1"), Real.Sin(Real.Pi / new Real("2"), 30));
        Assert.Equal(new Real("-1"), Real.Cos(Real.Pi, 30));
        Assert.Equal(Real.Sqrt(new Real("3")) / new Real("2"), Real.Cos(Real.Pi / new Real("6"), 30));
    }

    [Fact]
    public void SmallAngles_KeepTheirMagnitude()
    {
        using var precision = Real.WithPrecision(30, 30);

        // sin(10^-5) = 10^-5 - 10^-15/6 ... — 30 fractional places is far more than the value needs, so
        // truncation must not collapse it to zero, and what it does print must be a PREFIX of the true
        // expansion (mpmath at 120 dps: 0.000009999999999833333333334166666666664682540…).
        Real value = Real.Sin(new Real("0.00001"), 30);

        Assert.NotEqual("0", value.ToString());
        Assert.StartsWith(value.ToString(), "0.000009999999999833333333334166");
    }
}
