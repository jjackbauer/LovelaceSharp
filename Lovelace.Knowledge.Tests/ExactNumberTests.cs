using Lovelace.Knowledge;

namespace Lovelace.Knowledge.Tests;

public class ExactNumberTests
{
    [Theory]
    [InlineData("42", "42")]
    [InlineData("0", "0")]
    [InlineData("-7", "-7")]
    [InlineData("3.14", "3.14")]
    [InlineData("0.5", "0.5")]
    [InlineData("0.25", "0.25")]
    [InlineData("-0.5", "-0.5")]
    [InlineData("0.(3)", "0.(3)")]
    [InlineData("0.1(6)", "0.1(6)")]
    [InlineData("0.(6)", "0.(6)")]
    [InlineData("1.5", "1.5")]
    public void RoundTrips_CanonicalLiteral(string literal, string expected)
    {
        var n = ExactNumber.Parse(literal);
        Assert.Equal(expected, n.ToLovelaceLiteral());
    }

    [Fact]
    public void Parse_Periodic_IsExactFraction()
    {
        var third = ExactNumber.Parse("0.(3)");
        Assert.Equal(System.Numerics.BigInteger.One, third.Num);
        Assert.Equal(new System.Numerics.BigInteger(3), third.Den);
    }

    [Fact]
    public void Parse_Sixth_IsOneOverSix()
    {
        var sixth = ExactNumber.Parse("0.1(6)");
        Assert.Equal(System.Numerics.BigInteger.One, sixth.Num);
        Assert.Equal(new System.Numerics.BigInteger(6), sixth.Den);
    }

    [Fact]
    public void Add_PeriodicThirds_GivesTwoThirds()
    {
        var third = ExactNumber.Parse("0.(3)");
        var sum = ExactNumber.Add(third, third);
        Assert.Equal(ExactNumber.Parse("0.(6)"), sum);
        Assert.Equal("0.(6)", sum.ToLovelaceLiteral());
    }

    [Fact]
    public void Compare_PeriodicVsDecimal()
    {
        Assert.True(ExactNumber.Parse("0.(3)") > ExactNumber.Parse("0.3"));
        Assert.True(ExactNumber.Parse("0.5") == ExactNumber.Parse("0.5"));
        Assert.True(ExactNumber.Parse("-1") < ExactNumber.Parse("0"));
    }

    [Fact]
    public void Format_OneSeventh_ShortestPeriod()
    {
        Assert.Equal("0.(142857)", ExactNumber.Parse("0.(142857)").ToLovelaceLiteral());
    }
}
