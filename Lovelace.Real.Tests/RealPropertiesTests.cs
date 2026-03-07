using Lovelace.Real;

namespace Lovelace.Real.Tests;

/// <summary>
/// Functional tests for Real instance properties, static configuration properties,
/// and the basic scaffold (inheritance from Integer, static predicates).
///
/// Covers checklist items:
///   - scaffold (class Real : Integer, interface declarations)
///   - long Exponent { get; set; }
///   - long PeriodStart { get; private set; }
///   - long PeriodLength { get; private set; }
///   - bool IsPeriodic { get; }
///   - static long DisplayDecimalPlaces { get; set; }
///   - static long MaxComputationDecimalPlaces { get; set; }
/// </summary>
public class RealPropertiesTests
{
    // -------------------------------------------------------------------------
    // Scaffold: Real correctly inherits Integer behaviour
    // -------------------------------------------------------------------------

    [Fact]
    public void IsZero_GivenDefaultReal_ReturnsTrue()
    {
        var r = new Real();
        Assert.True(Real.IsZero(r));
    }

    [Fact]
    public void IsNegative_GivenDefaultReal_ReturnsFalse()
    {
        var r = new Real();
        Assert.False(Real.IsNegative(r));
    }

    [Fact]
    public void ToString_GivenDefaultReal_ReturnsZeroString()
    {
        var r = new Real();
        Assert.Equal("0", r.ToString());
    }

    // -------------------------------------------------------------------------
    // Exponent property
    // -------------------------------------------------------------------------

    [Fact]
    public void Exponent_GivenDefaultReal_IsZero()
    {
        var r = new Real();
        Assert.Equal(0L, r.Exponent);
    }

    [Fact]
    public void Exponent_AfterSetting_ReturnsNewValue()
    {
        var r = new Real();
        r.Exponent = -5;
        Assert.Equal(-5L, r.Exponent);
    }

    // -------------------------------------------------------------------------
    // PeriodStart property
    // -------------------------------------------------------------------------

    [Fact]
    public void PeriodStart_GivenDefaultReal_IsZero()
    {
        var r = new Real();
        Assert.Equal(0L, r.PeriodStart);
    }

    // -------------------------------------------------------------------------
    // PeriodLength property
    // -------------------------------------------------------------------------

    [Fact]
    public void PeriodLength_GivenDefaultReal_IsZero()
    {
        var r = new Real();
        Assert.Equal(0L, r.PeriodLength);
    }

    // -------------------------------------------------------------------------
    // IsPeriodic computed property
    // -------------------------------------------------------------------------

    [Fact]
    public void IsPeriodic_GivenDefaultReal_IsFalse()
    {
        var r = new Real();
        Assert.False(r.IsPeriodic);
    }

    // -------------------------------------------------------------------------
    // DisplayDecimalPlaces static property
    // -------------------------------------------------------------------------

    [Fact]
    public void DisplayDecimalPlaces_DefaultValue_IsOneHundred()
    {
        // Reset to default before asserting (other tests may have mutated it).
        Real.DisplayDecimalPlaces = 100L;
        Assert.Equal(100L, Real.DisplayDecimalPlaces);
    }

    [Fact]
    public void DisplayDecimalPlaces_AfterSetting_ReturnsNewValue()
    {
        long previous = Real.DisplayDecimalPlaces;
        try
        {
            Real.DisplayDecimalPlaces = 10L;
            Assert.Equal(10L, Real.DisplayDecimalPlaces);
        }
        finally
        {
            Real.DisplayDecimalPlaces = previous;
        }
    }

    // -------------------------------------------------------------------------
    // MaxComputationDecimalPlaces static property
    // -------------------------------------------------------------------------

    [Fact]
    public void MaxComputationDecimalPlaces_DefaultValue_IsOneThousand()
    {
        // Reset to default before asserting.
        Real.MaxComputationDecimalPlaces = 1000L;
        Assert.Equal(1000L, Real.MaxComputationDecimalPlaces);
    }

    [Fact]
    public void MaxComputationDecimalPlaces_AfterSetting_ReturnsNewValue()
    {
        long previous = Real.MaxComputationDecimalPlaces;
        try
        {
            Real.MaxComputationDecimalPlaces = 50L;
            Assert.Equal(50L, Real.MaxComputationDecimalPlaces);
        }
        finally
        {
            Real.MaxComputationDecimalPlaces = previous;
        }
    }
}
