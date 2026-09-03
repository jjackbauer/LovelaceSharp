using Lovelace.Real;
using Rl = Lovelace.Real.Real;

namespace Lovelace.Real.Tests;

public class LReal128Tests
{
    static LReal128Tests()
    {
        // Compare exact values: match class Real's display precision so ToString() parity is about
        // exactness, not the (now double-class, 16-digit) display default.
        LReal128.DisplayDecimalPlaces = (int)Rl.DisplayDecimalPlaces;
    }

    private static void Same(string expr, Func<Rl> r, Func<LReal128> l) =>
        Assert.Equal(r().ToString(), l().ToString());

    private static void Promotes(Func<LReal128> l) => Assert.Throws<LRealPromoteException>(() => l());

    [Theory]
    [InlineData("0")]
    [InlineData("1")]
    [InlineData("-42")]
    [InlineData("3.14")]
    [InlineData("0.5")]
    [InlineData("0.10")]
    [InlineData("12345678901234567890123456789012345678")] // 38 digits
    [InlineData("0.(3)")]
    [InlineData("0.1(6)")]
    [InlineData("0.(142857)")]
    public void Parse_GivenValue_RoundTripsLikeReal(string s)
        => Same(s, () => Rl.Parse(s), () => LReal128.Parse(s));

    [Theory]
    [InlineData("1", "2")]
    [InlineData("0.1", "0.2")]
    [InlineData("3.14", "2.71")]
    [InlineData("9999999999999999999", "0.000000001")] // 28 sig digits — fits in 38, promoted in LReal64
    public void Add_GivenPair_MatchesReal(string a, string b)
        => Same($"{a}+{b}", () => Rl.Parse(a) + Rl.Parse(b), () => LReal128.Parse(a) + LReal128.Parse(b));

    [Theory]
    [InlineData("2.345678901234567", "1.234567890123456")] // 16×16 = 32 digits — promoted in LReal64, fits in 38
    [InlineData("0.1", "0.2")]
    [InlineData("1234567890123456789", "9876543210987654321")] // 19×19 = 38 digits
    public void Multiply_GivenPair_MatchesReal(string a, string b)
        => Same($"{a}*{b}", () => Rl.Parse(a) * Rl.Parse(b), () => LReal128.Parse(a) * LReal128.Parse(b));

    [Theory]
    [InlineData("1", "3")]
    [InlineData("1", "7")]
    [InlineData("1", "29")] // period 28 — fits in 38, promoted in LReal64
    [InlineData("1", "17")]
    [InlineData("100", "13")]
    public void Divide_GivenPair_MatchesReal(string a, string b)
        => Same($"{a}/{b}", () => Rl.Parse(a) / Rl.Parse(b), () => LReal128.Parse(a) / LReal128.Parse(b));

    [Fact]
    public void Add_Periodic_MatchesReal()
    {
        Same("1/3+1/3", () => Rl.Parse("0.(3)") + Rl.Parse("0.(3)"), () => LReal128.Parse("0.(3)") + LReal128.Parse("0.(3)"));
        Same("1/3+1/6", () => Rl.Parse("0.(3)") + Rl.Parse("0.1(6)"), () => LReal128.Parse("0.(3)") + LReal128.Parse("0.1(6)"));
    }

    [Fact]
    public void Multiply_Overflow_Promotes() =>
        Promotes(() => LReal128.Parse("123456789012345678901234567890") * LReal128.Parse("123456789012345678901234567890")); // 30×30 = 60 digits

    [Fact]
    public void Divide_LongPeriod_Promotes() =>
        Promotes(() => LReal128.Parse("1") / LReal128.Parse("97")); // period 96 > 38

    [Fact]
    public void Add_Tenths_IsExact() =>
        Assert.Equal("0.3", (LReal128.Parse("0.1") + LReal128.Parse("0.2")).ToString());
}
