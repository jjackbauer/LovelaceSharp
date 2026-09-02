using System.Numerics;
using Lovelace.Natural;

namespace Lovelace.Natural.Tests;

/// <summary>
/// Randomized differential tests: every arithmetic operation on <see cref="Natural"/>
/// is compared against <see cref="System.Numerics.BigInteger"/> (a trusted independent
/// binary-limb implementation) across several size tiers, including adversarial
/// carry/borrow patterns at limb boundaries.
/// </summary>
public class NaturalRandomizedCrossCheckTests
{
    [Theory]
    [InlineData(1, 200)]
    [InlineData(10, 50)]
    [InlineData(100, 20)]
    [InlineData(1000, 5)]
    [InlineData(10000, 2)]
    public void CrossCheck_GivenRandomOperands_MatchesBigInteger(int maxDigits, int cases)
    {
        var rng = new Random(20240607);
        for (int t = 0; t < cases; t++)
        {
            string sa = RandomDigits(rng, rng.Next(1, maxDigits + 1));
            string sb = RandomDigits(rng, rng.Next(1, maxDigits + 1));
            var a = new Natural(sa);
            var b = new Natural(sb);
            var ba = BigInteger.Parse(sa);
            var bb = BigInteger.Parse(sb);

            Assert.Equal((ba + bb).ToString(), (a + b).ToString());
            Assert.Equal((ba * bb).ToString(), (a * b).ToString());
            if (a >= b)
                Assert.Equal((ba - bb).ToString(), (a - b).ToString());

            var q = Natural.DivRem(a, b, out var r);
            Assert.Equal((ba / bb).ToString(), q.ToString());
            Assert.Equal((ba % bb).ToString(), r.ToString());

            int e = rng.Next(0, 9);
            Assert.Equal(BigInteger.Pow(ba, e).ToString(), a.Pow(new Natural((ulong)e)).ToString());
        }
    }

    [Theory]
    [InlineData(0)]
    [InlineData(1)]
    [InlineData(5)]
    [InlineData(20)]
    [InlineData(100)]
    public void Factorial_GivenN_MatchesBigInteger(int n)
    {
        var expected = BigIntegerFactorial(n).ToString();
        Assert.Equal(expected, new Natural((ulong)n).Factorial().ToString());
    }

    [Fact]
    public void CrossCheck_GivenAdversarialCarryPatterns_MatchesBigInteger()
    {
        string[] values =
        {
            "9999999999999999999999999999999999999999",
            "10000000000000000000000000000000000000000",
            "18446744073709551615",                                  // 2^64 − 1
            "18446744073709551616",                                  // 2^64
            "340282366920938463463374607431768211455",               // 2^128 − 1
        };

        foreach (string x in values)
        foreach (string y in values)
        {
            var a = new Natural(x);
            var b = new Natural(y);
            var ba = BigInteger.Parse(x);
            var bb = BigInteger.Parse(y);

            Assert.Equal((ba + bb).ToString(), (a + b).ToString());
            Assert.Equal((ba * bb).ToString(), (a * b).ToString());
            if (ba >= bb)
                Assert.Equal((ba - bb).ToString(), (a - b).ToString());

            var q = Natural.DivRem(a, b, out var r);
            Assert.Equal((ba / bb).ToString(), q.ToString());
            Assert.Equal((ba % bb).ToString(), r.ToString());
        }
    }

    [Fact]
    public void DivRem_GivenOverflowAlignment_MatchesBigInteger()
    {
        // Regression: divisor = 10^30 with dividend = (10^40 − 1) · 2^64 exercises the
        // Knuth "top limb equal" overflow branch that random divisors rarely trigger.
        var dividend = new Natural("9999999999999999999999999999999999999999")
                       * new Natural("18446744073709551616");
        var divisor = new Natural("1" + new string('0', 30));
        var bd = BigInteger.Parse("9999999999999999999999999999999999999999")
                 * BigInteger.Parse("18446744073709551616");
        var bv = BigInteger.Pow(10, 30);

        var q = Natural.DivRem(dividend, divisor, out var r);

        Assert.Equal((bd / bv).ToString(), q.ToString());
        Assert.Equal((bd % bv).ToString(), r.ToString());
    }

    [Fact]
    public void DivRem_GivenPowerOfTenDivisor_MatchesBigInteger()
    {
        var rng = new Random(99);
        for (int k = 1; k <= 40; k++)
        {
            var divisor = BigInteger.Pow(10, k);
            var divNat = new Natural(divisor.ToString());
            for (int t = 0; t < 20; t++)
            {
                string sd = RandomDigits(rng, rng.Next(1, 80));
                var a = new Natural(sd);
                var ba = BigInteger.Parse(sd);
                if (ba < divisor) continue;
                var q = Natural.DivRem(a, divNat, out var r);
                Assert.Equal((ba / divisor).ToString(), q.ToString());
                Assert.Equal((ba % divisor).ToString(), r.ToString());
            }
        }
    }

    private static string RandomDigits(Random rng, int n)
    {
        var sb = new System.Text.StringBuilder(n);
        sb.Append((char)('1' + rng.Next(9)));
        for (int i = 1; i < n; i++) sb.Append((char)('0' + rng.Next(10)));
        return sb.ToString();
    }

    private static BigInteger BigIntegerFactorial(int n)
    {
        var result = BigInteger.One;
        for (int i = 2; i <= n; i++)
            result *= i;
        return result;
    }
}
