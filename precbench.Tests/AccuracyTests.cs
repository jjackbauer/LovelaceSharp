using System.Globalization;
using Lovelace.Real;

namespace PrecBench.Tests;

/// <summary>
/// Accuracy tests for the precbench comparison: Lovelace.Real pinned to 8 and 16
/// significant digits vs float and double, asserted against a high-precision
/// Lovelace.Real reference. This is the "test" half of precbench; the throughput
/// half is the BenchmarkDotNet project ../precbench.
/// </summary>
public class AccuracyTests
{
    // -------------------------------------------------------------------------
    // Precision scoping helpers
    // -------------------------------------------------------------------------

    /// <summary>
    /// Runs <paramref name="body"/> with both Real precision statics set to
    /// <paramref name="frac"/> fractional places, then restores the prior values.
    /// </summary>
    private static T WithPrecision<T>(long frac, Func<T> body)
    {
        long savedMax = Real.MaxComputationDecimalPlaces;
        long savedDisp = Real.DisplayDecimalPlaces;
        try
        {
            Real.MaxComputationDecimalPlaces = frac;
            Real.DisplayDecimalPlaces = frac;
            return body();
        }
        finally
        {
            Real.MaxComputationDecimalPlaces = savedMax;
            Real.DisplayDecimalPlaces = savedDisp;
        }
    }

    /// <summary>Converts a Real to double via ToString, expanding periodic notation.</summary>
    private static double ToDouble(Real r)
    {
        string s = r.ToString();
        int open = s.IndexOf('(');
        if (open >= 0)
        {
            string head = s[..open];
            string block = s[(open + 1)..^1];
            var sb = new System.Text.StringBuilder(head);
            while (sb.Length - head.Length < 40)
                sb.Append(block);
            s = sb.ToString();
        }
        return double.Parse(s, CultureInfo.InvariantCulture);
    }

    /// <summary>
    /// Relative error |approx − exact| / |exact|, computed in Real at 60 digits so
    /// differences far below double precision survive, then narrowed to double.
    /// </summary>
    private static double RelativeError(Real approx, Real exact)
    {
        Real num = Real.Abs(approx - exact);
        Real rel = WithPrecision(60, () => num / Real.Abs(exact));
        return ToDouble(rel);
    }

    private static Real SqrtAt(string value, long frac) =>
        WithPrecision(frac, () => Real.Sqrt(new Real(value)));

    // -------------------------------------------------------------------------
    // Precision scoping
    // -------------------------------------------------------------------------

    [Fact]
    public void WithPrecision_GivenScope_RestoresGlobalState()
    {
        long savedMax = Real.MaxComputationDecimalPlaces;
        long savedDisp = Real.DisplayDecimalPlaces;

        _ = WithPrecision(7, () => Real.Sqrt(new Real("2")));

        Assert.Equal(savedMax, Real.MaxComputationDecimalPlaces);
        Assert.Equal(savedDisp, Real.DisplayDecimalPlaces);
    }

    [Fact]
    public void Sqrt_GivenTwo_AtP8AndP16_ProduceDifferentDigitCounts()
    {
        Real p8 = SqrtAt("2", 7);
        Real p16 = SqrtAt("2", 15);

        // 8 significant digits == 1 integer + 7 fractional; 16 == 1 + 15.
        Assert.Equal(-7L, p8.Exponent);
        Assert.Equal(-15L, p16.Exponent);
    }

    // -------------------------------------------------------------------------
    // Irrational accuracy — Lovelace P8/P16 within the float/double class
    // -------------------------------------------------------------------------

    [Theory]
    [InlineData("2")]
    [InlineData("3")]
    [InlineData("5")]
    public void Sqrt_GivenIrrational_AtP8_ErrorWithinFloatClass(string input)
    {
        Real exact = SqrtAt(input, 100);
        Real lovelace8 = SqrtAt(input, 7);
        float nativeF = MathF.Sqrt(float.Parse(input, CultureInfo.InvariantCulture));

        double errL = RelativeError(lovelace8, exact);
        double errF = RelativeError(new Real((double)nativeF), exact);

        Assert.True(errL <= 1e-6, $"Lovelace8 relative error {errL:E2} exceeds 1e-6");
        Assert.True(errF <= 1e-6, $"float relative error {errF:E2} exceeds 1e-6");
        Assert.True(errL <= 8.0 * errF, $"Lovelace8 error {errL:E2} exceeds 8x float error {errF:E2}");
    }

    [Theory]
    [InlineData("2")]
    [InlineData("3")]
    [InlineData("5")]
    public void Sqrt_GivenIrrational_AtP16_ErrorWithinDoubleClass(string input)
    {
        Real exact = SqrtAt(input, 100);
        Real lovelace16 = SqrtAt(input, 15);
        double nativeD = Math.Sqrt(double.Parse(input, CultureInfo.InvariantCulture));

        double errL = RelativeError(lovelace16, exact);
        double errD = RelativeError(new Real(nativeD), exact);

        Assert.True(errL <= 1e-14, $"Lovelace16 relative error {errL:E2} exceeds 1e-14");
        Assert.True(errD <= 1e-14, $"double relative error {errD:E2} exceeds 1e-14");
        Assert.True(errL <= 8.0 * errD, $"Lovelace16 error {errL:E2} exceeds 8x double error {errD:E2}");
    }

    // -------------------------------------------------------------------------
    // Rational exactness — Lovelace is exact where float/double round
    // -------------------------------------------------------------------------

    [Fact]
    public void Divide_GivenOneThird_IsExactPeriodic()
    {
        Real third = Real.One / new Real("3");
        Assert.True(third.IsPeriodic);
        Assert.Equal(1L, third.PeriodLength);
        Assert.Equal("0.(3)", third.ToString());
    }

    [Fact]
    public void Divide_GivenOneSeventh_IsExactPeriodic()
    {
        Assert.Equal("0.(142857)", (Real.One / new Real("7")).ToString());
    }

    [Fact]
    public void Divide_GivenOneSixth_IsExactMixedPeriod()
    {
        Assert.Equal("0.1(6)", (Real.One / new Real("6")).ToString());
    }

    [Fact]
    public void Add_GivenPointOneAndPointTwo_IsExactlyPointThree()
    {
        Real sum = new Real("0.1") + new Real("0.2");
        Assert.Equal(new Real("0.3"), sum);

        // Contrast: the same operation in double is inexact (0.30000000000000004).
        Assert.NotEqual(0.3, 0.1 + 0.2);
    }

    [Fact]
    public void Parse_GivenPointOne_LovelaceExactFloatInexact()
    {
        Assert.Equal(new Real("0.1"), new Real("0.1"));

        // float cannot represent 0.1 exactly; widening to double exposes the error.
        Assert.NotEqual(0.1, (double)0.1f);
    }
}
