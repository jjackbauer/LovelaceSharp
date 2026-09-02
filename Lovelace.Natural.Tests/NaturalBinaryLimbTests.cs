using System.Numerics;
using Lovelace.Natural;

namespace Lovelace.Natural.Tests;

/// <summary>
/// Functional tests targeting the binary-limb (base 2^64) representation of
/// <see cref="Natural"/>: carry/borrow across limb boundaries, 128-bit product
/// overflow, large-operand division, decimal shifts, and parse/ToString round-trips.
/// These complement the existing decimal-level test suite.
/// </summary>
public class NaturalBinaryLimbTests
{
    private static Natural N(BigInteger b) => new(b.ToString());

    [Fact]
    public void Add_GivenCarryAcrossLimbBoundary_ProducesCorrectResult()
    {
        var max = N(BigInteger.Pow(2, 64) - 1);
        var one = new Natural("1");
        Assert.Equal(N(BigInteger.Pow(2, 64)), max + one);
    }

    [Fact]
    public void Sub_GivenBorrowAcrossLimbBoundary_ProducesCorrectResult()
    {
        var pow64 = N(BigInteger.Pow(2, 64));
        var one = new Natural("1");
        Assert.Equal(N(BigInteger.Pow(2, 64) - 1), pow64 - one);
    }

    [Fact]
    public void Mul_GivenLimbOverflow_ProducesCorrectResult()
    {
        // (2^64 − 1)² = 2^128 − 2^65 + 1 — a 128-bit product split across two limbs.
        var max = N(BigInteger.Pow(2, 64) - 1);
        var expected = N((BigInteger.Pow(2, 64) - 1) * (BigInteger.Pow(2, 64) - 1));
        Assert.Equal(expected, max * max);
    }

    [Fact]
    public void DivRem_GivenLargeOperands_ProducesCorrectQuotientAndRemainder()
    {
        var rng = new Random(42);
        var a = new Natural(RandomDigits(rng, 2000));
        var b = new Natural(RandomDigits(rng, 1200));
        var ba = BigInteger.Parse(a.ToString());
        var bb = BigInteger.Parse(b.ToString());

        var q = Natural.DivRem(a, b, out var r);

        Assert.Equal((ba / bb).ToString(), q.ToString());
        Assert.Equal((ba % bb).ToString(), r.ToString());
    }

    [Fact]
    public void ShiftLeftDecimal_GivenK_AppendsKDecimalZeros()
    {
        var n = new Natural("123456789");
        Assert.Equal("123456789" + new string('0', 7), n.ShiftLeftDecimal(7).ToString());
    }

    [Fact]
    public void RoundTrip_GivenRandomLargeValues_ParseToStringIdentity()
    {
        var rng = new Random(7);
        for (int i = 0; i < 30; i++)
        {
            string s = RandomDigits(rng, rng.Next(1, 20000));
            Assert.Equal(s, new Natural(s).ToString());
        }
    }

    private static string RandomDigits(Random rng, int n)
    {
        var sb = new System.Text.StringBuilder(n);
        sb.Append((char)('1' + rng.Next(9)));
        for (int i = 1; i < n; i++) sb.Append((char)('0' + rng.Next(10)));
        return sb.ToString();
    }
}
