using Lovelace.Real;

namespace Lovelace.Real.Tests;

/// <summary>
/// Functional tests covering the five public constructors and <see cref="Real.Assign"/>.
///
/// Checklist items:
///   - Constructors: Real(), Real(double), Real(string), Real(Real), Real(Integer)
///   - Real Assign(Real other)
/// </summary>
public class RealConstructorTests
{
    // -------------------------------------------------------------------------
    // Real() — default constructor
    // -------------------------------------------------------------------------

    [Fact]
    public void Constructor_GivenDefault_ProducesZeroWithExponentZero()
    {
        var r = new Real();
        Assert.True(Real.IsZero(r));
        Assert.Equal(0L, r.Exponent);
    }

    // -------------------------------------------------------------------------
    // Real(double) constructor
    // -------------------------------------------------------------------------

    [Fact]
    public void Constructor_GivenDouble_StoresCorrectDigitsAndExponent()
    {
        var r = new Real(3.14);
        Assert.Equal(new Real("3.14"), r);
    }

    [Fact]
    public void Constructor_GivenDoubleZero_ProducesZero()
    {
        var r = new Real(0.0);
        Assert.True(Real.IsZero(r));
    }

    [Fact]
    public void Constructor_GivenNegativeDouble_IsNegative()
    {
        var r = new Real(-1.5);
        Assert.True(Real.IsNegative(r));
    }

    // -------------------------------------------------------------------------
    // Real(string) constructor
    // -------------------------------------------------------------------------

    [Fact]
    public void Constructor_GivenStringInteger_ExponentIsZero()
    {
        var r = new Real("42");
        Assert.Equal(0L, r.Exponent);
        Assert.Equal(new Real("42"), r);
    }

    [Fact]
    public void Constructor_GivenStringDecimal_ExponentIsNegativePlaceCount()
    {
        var r = new Real("3.14");
        Assert.Equal(-2L, r.Exponent);
    }

    [Fact]
    public void Constructor_GivenStringNegativeDecimal_StoredCorrectly()
    {
        var r = new Real("-0.001");
        Assert.True(Real.IsNegative(r));
        Assert.Equal(new Real("-0.001"), r);
    }

    // -------------------------------------------------------------------------
    // Real(Real) — copy constructor
    // -------------------------------------------------------------------------

    [Fact]
    public void Constructor_GivenRealCopy_ProducesIndependentCopy()
    {
        var original = new Real("3.14");
        var copy = new Real(original);
        // Modifying copy's Exponent must not affect original.
        copy.Exponent = -99L;
        Assert.Equal(-2L, original.Exponent);
    }

    [Fact]
    public void Constructor_GivenRealCopy_CopiesAllFields()
    {
        var original = Real.Parse("0.(3)");
        var copy = new Real(original);
        Assert.Equal(original.Exponent, copy.Exponent);
        Assert.Equal(original.PeriodStart, copy.PeriodStart);
        Assert.Equal(original.PeriodLength, copy.PeriodLength);
        Assert.Equal(original, copy);
    }

    // -------------------------------------------------------------------------
    // Real(Integer) constructor
    // -------------------------------------------------------------------------

    [Fact]
    public void Constructor_GivenInteger_ExponentIsZero()
    {
        var intVal = Lovelace.Integer.Integer.Parse("7", null);
        var r = new Real(intVal);
        Assert.Equal(0L, r.Exponent);
        Assert.Equal(new Real("7"), r);
    }

    [Fact]
    public void Constructor_GivenNegativeInteger_IsNegativeAndExponentZero()
    {
        var intVal = Lovelace.Integer.Integer.Parse("-42", null);
        var r = new Real(intVal);
        Assert.True(Real.IsNegative(r));
        Assert.Equal(0L, r.Exponent);
        Assert.Equal(new Real("-42"), r);
    }

    // -------------------------------------------------------------------------
    // Assign(Real other)
    // -------------------------------------------------------------------------

    [Fact]
    public void Assign_GivenOtherReal_CopiesAllFields()
    {
        var a = new Real();
        var b = new Real("3.14");
        var result = a.Assign(b);
        Assert.Equal(b.Exponent, result.Exponent);
        Assert.Equal(b.PeriodStart, result.PeriodStart);
        Assert.Equal(b.PeriodLength, result.PeriodLength);
        Assert.Equal(b, result);
    }

    [Fact]
    public void Assign_GivenOtherReal_ProducesDeepCopy()
    {
        var b = new Real("3.14");
        var a = new Real().Assign(b);
        b.Exponent = -99L;
        Assert.Equal(-2L, a.Exponent);
    }

    [Fact]
    public void Assign_GivenPeriodicReal_CopiesPeriodMetadata()
    {
        var periodic = Real.Parse("0.(3)");
        var copy = new Real().Assign(periodic);
        Assert.Equal(0L, copy.PeriodStart);
        Assert.Equal(1L, copy.PeriodLength);
        Assert.True(copy.IsPeriodic);
        Assert.Equal(Real.Parse("0.(3)"), copy);
    }

    // -------------------------------------------------------------------------
    // Real(ReadOnlySpan<char>) constructor
    // -------------------------------------------------------------------------

    [Fact]
    public void Constructor_GivenReadOnlySpanIntegerString_ParsesCorrectly()
    {
        var r = new Real("42".AsSpan());
        Assert.Equal(Real.Parse("42"), r);
        Assert.Equal(0L, r.Exponent);
    }

    [Fact]
    public void Constructor_GivenReadOnlySpanDecimalString_ParsesCorrectly()
    {
        var r = new Real("1.5".AsSpan());
        Assert.Equal(Real.Parse("1.5"), r);
        Assert.Equal(-1L, r.Exponent);
    }

    [Fact]
    public void Constructor_GivenReadOnlySpanPeriodicString_ParsesCorrectly()
    {
        var r = new Real("0.(3)".AsSpan());
        Assert.True(r.IsPeriodic);
        Assert.Equal(1L, r.PeriodLength);
    }

    [Fact]
    public void Constructor_GivenReadOnlySpanNegativeString_ParsesCorrectly()
    {
        var r = new Real("-2.5".AsSpan());
        Assert.True(Real.IsNegative(r));
        Assert.Equal(Real.Parse("-2.5"), r);
    }

    [Fact]
    public void Constructor_GivenReadOnlySpanEmpty_ThrowsFormatException()
    {
        Assert.Throws<FormatException>(() => new Real(ReadOnlySpan<char>.Empty));
    }

    [Fact]
    public void Constructor_GivenReadOnlySpanMalformed_ThrowsFormatException()
    {
        Assert.Throws<FormatException>(() => new Real("abc".AsSpan()));
    }

    // -------------------------------------------------------------------------
    // Real(decimal) constructor
    // -------------------------------------------------------------------------

    [Fact]
    public void Constructor_GivenPositiveDecimal_ParsesCorrectly()
    {
        var r = new Real(1.5m);
        Assert.Equal(Real.Parse("1.5"), r);
    }

    [Fact]
    public void Constructor_GivenNegativeDecimal_ParsesCorrectly()
    {
        var r = new Real(-3.14m);
        Assert.True(Real.IsNegative(r));
        Assert.Equal(Real.Parse("-3.14"), r);
    }

    [Fact]
    public void Constructor_GivenZeroDecimal_ReturnsZero()
    {
        var r = new Real(0m);
        Assert.True(Real.IsZero(r));
    }

    [Fact]
    public void Constructor_GivenDecimalMaxValue_ParsesWithoutThrow()
    {
        var expected = decimal.MaxValue.ToString("G29", System.Globalization.CultureInfo.InvariantCulture);
        var r = new Real(decimal.MaxValue);
        Assert.Equal(expected, r.ToString());
    }

    [Fact]
    public void Constructor_GivenDecimal_RoundTripEqualsStringConstructor()
    {
        decimal d = 1.2345678901234567890m;
        var fromDecimal = new Real(d);
        var fromString  = new Real(d.ToString("G29", System.Globalization.CultureInfo.InvariantCulture));
        Assert.Equal(fromString, fromDecimal);
    }
}
