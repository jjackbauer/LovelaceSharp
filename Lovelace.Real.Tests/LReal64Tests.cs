using Lovelace.Real;
using Rl = Lovelace.Real.Real;

namespace Lovelace.Real.Tests;

/// <summary>
/// Exactness parity: LReal64 must reproduce class Real's decimal/period semantics within 19
/// significant digits, and throw LRealPromoteException (never round) beyond them.
/// </summary>
public class LReal64Tests
{
    static LReal64Tests()
    {
        // Compare exact values: match class Real's display precision so ToString() parity is about
        // exactness, not the (now float-class, 7-digit) display default.
        LReal64.DisplayDecimalPlaces = (int)Rl.DisplayDecimalPlaces;
    }

    private static void Same(string expr, Func<Rl> r, Func<LReal64> l)
    {
        Assert.Equal(r().ToString(), l().ToString());
    }

    private static void Promotes(string expr, Func<LReal64> l)
    {
        Assert.Throws<LRealPromoteException>(() => l());
    }

    // ---------- parse / format round-trip ----------

    [Theory]
    [InlineData("0")]
    [InlineData("1")]
    [InlineData("-1")]
    [InlineData("42")]
    [InlineData("-42")]
    [InlineData("3.14")]
    [InlineData("0.5")]
    [InlineData("0.05")]
    [InlineData("1.5")]
    [InlineData("10.5")]
    [InlineData("100")]
    [InlineData("100.0")]
    [InlineData("0.10")]
    [InlineData("1234567890123456789")]
    [InlineData("0.1234567890123456789")]
    [InlineData("-0.0001")]
    [InlineData("0.(3)")]
    [InlineData("0.1(6)")]
    [InlineData("0.(142857)")]
    [InlineData("1.(3)")]
    [InlineData("0.(9)")]
    public void Parse_GivenValue_RoundTripsLikeReal(string s)
    {
        Same(s, () => Rl.Parse(s), () => LReal64.Parse(s));
    }

    // ---------- arithmetic parity ----------

    [Theory]
    [InlineData("1", "2")]
    [InlineData("0.5", "0.25")]
    [InlineData("0.1", "0.2")]
    [InlineData("1.5", "2.25")]
    [InlineData("10", "100")]
    [InlineData("3.14", "2.71")]
    public void Add_GivenPair_MatchesReal(string a, string b)
        => Same($"{a}+{b}", () => Rl.Parse(a) + Rl.Parse(b), () => LReal64.Parse(a) + LReal64.Parse(b));

    [Theory]
    [InlineData("1", "2")]
    [InlineData("3", "5")]
    [InlineData("0.5", "0.25")]
    [InlineData("0.3", "0.1")]
    [InlineData("10", "3")]
    public void Subtract_GivenPair_MatchesReal(string a, string b)
        => Same($"{a}-{b}", () => Rl.Parse(a) - Rl.Parse(b), () => LReal64.Parse(a) - LReal64.Parse(b));

    [Theory]
    [InlineData("2", "3")]
    [InlineData("0.1", "0.2")]
    [InlineData("1.5", "2.0")]
    [InlineData("12345678", "87654321")]
    public void Multiply_GivenPair_MatchesReal(string a, string b)
        => Same($"{a}*{b}", () => Rl.Parse(a) * Rl.Parse(b), () => LReal64.Parse(a) * LReal64.Parse(b));

    [Theory]
    [InlineData("1", "3")]
    [InlineData("1", "7")]
    [InlineData("1", "6")]
    [InlineData("1", "9")]
    [InlineData("2", "3")]
    [InlineData("3", "7")]
    [InlineData("7", "12")]
    [InlineData("10", "11")]
    [InlineData("1", "17")]
    [InlineData("3", "16")]
    [InlineData("100", "13")]
    public void Divide_GivenPair_MatchesReal(string a, string b)
        => Same($"{a}/{b}", () => Rl.Parse(a) / Rl.Parse(b), () => LReal64.Parse(a) / LReal64.Parse(b));

    [Fact]
    public void Add_Periodic_MatchesReal()
    {
        Same("1/3+1/3", () => Rl.Parse("0.(3)") + Rl.Parse("0.(3)"), () => LReal64.Parse("0.(3)") + LReal64.Parse("0.(3)"));
        Same("1/3+1/6", () => Rl.Parse("0.(3)") + Rl.Parse("0.1(6)"), () => LReal64.Parse("0.(3)") + LReal64.Parse("0.1(6)"));
        Same("1/3*2", () => Rl.Parse("0.(3)") * Rl.Parse("2"), () => LReal64.Parse("0.(3)") * LReal64.Parse("2"));
    }

    // ---------- exactness (not IEEE-754) ----------

    [Fact]
    public void Add_Tenths_IsExact()
    {
        // 0.1 + 0.2 == 0.3 exactly (double would give 0.30000000000000004).
        Assert.Equal("0.3", (LReal64.Parse("0.1") + LReal64.Parse("0.2")).ToString());
    }

    // ---------- promotion (never rounds) ----------

    [Fact]
    public void Divide_LongPeriod_Promotes() => Promotes("1/97", () => LReal64.Parse("1") / LReal64.Parse("97"));

    [Fact]
    public void Add_Overflow_Promotes() =>
        Promotes("19digit+frac", () => LReal64.Parse("9999999999999999999") + LReal64.Parse("0.000000001"));

    [Fact]
    public void Multiply_Overflow_Promotes() =>
        Promotes("16digit*16digit", () => LReal64.Parse("2.345678901234567") * LReal64.Parse("1.234567890123456"));

    [Fact]
    public void Parse_TooManyDigits_Fails()
    {
        Assert.False(LReal64.TryParse("12345678901234567890123", out _)); // 23 digits
    }

    // ---------- comparison ----------

    [Theory]
    [InlineData("0.1", "0.2")]
    [InlineData("0.(3)", "0.33333")]
    [InlineData("0.1(6)", "0.16666")]
    [InlineData("-1", "1")]
    [InlineData("3.14", "3.14159")]
    public void Compare_MatchesReal(string a, string b)
    {
        int r = Rl.Parse(a).CompareTo(Rl.Parse(b));
        int l = LReal64.Parse(a).CompareTo(LReal64.Parse(b));
        Assert.Equal(Math.Sign(r), Math.Sign(l));
    }
}
