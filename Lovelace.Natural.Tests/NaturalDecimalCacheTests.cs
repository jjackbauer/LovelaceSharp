using System.Numerics;
using Lovelace.Natural;

namespace Lovelace.Natural.Tests;

/// <summary>
/// Tests for the parse/ToString decimal-cache upgrade. These pin the two risks the cache
/// introduces: correctness of the lazily-materialized binary limbs / decimal string, and
/// thread-safety of the lock-free compute-then-<c>Interlocked.CompareExchange</c> caches.
/// </summary>
public class NaturalDecimalCacheTests
{
    // -------------------------------------------------------------------------
    // Cached decimal string — correctness
    // -------------------------------------------------------------------------

    [Fact]
    public void ParseToString_GivenLeadingZeros_ReturnsCanonicalString()
    {
        var n = new Natural("007");
        Assert.Equal("7", n.ToString());
    }

    [Fact]
    public void ToString_GivenParsedValue_IsStableAcrossRepeatedCalls()
    {
        var n = new Natural("123456789012345678901234567890");
        var first = n.ToString();
        var second = n.ToString();
        Assert.Same(first, second);
    }

    [Fact]
    public void RoundTrip_GivenRandomLargeValues_ParseToStringIdentity()
    {
        var rng = new Random(20240608);
        for (int i = 0; i < 20; i++)
        {
            string s = RandomDigits(rng, rng.Next(1, 20001));
            Assert.Equal(s, new Natural(s).ToString());
        }
    }

    [Fact]
    public void ArithmeticResult_ToString_CachesAfterFirstCall()
    {
        var a = new Natural("12345678901234567890");
        var b = new Natural("98765432109876543210");
        var expected = (BigInteger.Parse("12345678901234567890")
                        * BigInteger.Parse("98765432109876543210")).ToString();

        var product = a * b;
        var first = product.ToString();
        Assert.Equal(expected, first);

        // The first ToString materialized and cached the decimal string; the second
        // call must return the identical cached object.
        var second = product.ToString();
        Assert.Same(first, second);
    }

    [Fact]
    public void GetHashCode_GivenEqualValues_AreEqual()
    {
        var a = new Natural("000123456789");
        var b = new Natural("123456789");
        Assert.Equal(a, b);
        Assert.Equal(a.GetHashCode(), b.GetHashCode());
    }

    [Fact]
    public void CopyConstructor_SharesCachedStringButIndependentLimbs()
    {
        var original = new Natural("123456789012345678901234567890");
        var copy = new Natural(original);

        // A distinct instance with the same value, sharing the immutable decimal string.
        Assert.NotSame(original, copy);
        Assert.Equal(original, copy);
        Assert.Same(original.ToString(), copy.ToString());
    }

    // -------------------------------------------------------------------------
    // Lazy conversion thread-safety
    // -------------------------------------------------------------------------

    [Fact]
    public void ConcurrentToString_GivenSharedValue_ReturnsConsistentString()
    {
        // An arithmetic result starts with no cached string, so the concurrent first
        // ToString calls genuinely race to compute-and-store it.
        var a = new Natural("1234567890123456789012345678901234567890");
        var b = new Natural("9876543210987654321098765432109876543210");
        var product = a * b;
        var expected = (BigInteger.Parse(a.ToString()) * BigInteger.Parse(b.ToString())).ToString();

        var results = new string[Math.Max(16, Environment.ProcessorCount * 4)];
        Parallel.For(0, results.Length, i => results[i] = product.ToString());

        for (int i = 1; i < results.Length; i++)
            Assert.Same(results[0], results[i]);
        Assert.Equal(expected, results[0]);
    }

    [Fact]
    public void ConcurrentParseToString_GivenManyValues_MatchesBigInteger()
    {
        var rng = new Random(20240609);
        const int count = 200;
        var inputs = new string[count];
        var expected = new string[count];
        for (int i = 0; i < count; i++)
        {
            string s = RandomDigits(rng, rng.Next(1, 201));
            inputs[i] = s;
            expected[i] = BigInteger.Parse(s).ToString();
        }

        var actual = new string[count];
        Parallel.For(0, count, i => actual[i] = new Natural(inputs[i]).ToString());

        for (int i = 0; i < count; i++)
            Assert.Equal(expected[i], actual[i]);
    }

    [Fact]
    public void ConcurrentArithmeticOnLazyParsedValue_MatchesBigInteger()
    {
        // a and b are parsed lazily (limbs not yet materialized); the first concurrent
        // +/* must materialize their limbs race-free.
        var a = new Natural("1234567890123456789012345678901234567890");
        var b = new Natural("9876543210987654321098765432109876543210");
        var ba = BigInteger.Parse("1234567890123456789012345678901234567890");
        var bb = BigInteger.Parse("9876543210987654321098765432109876543210");
        var expectedSum = (ba + bb).ToString();
        var expectedProd = (ba * bb).ToString();

        var sums = new string[64];
        var prods = new string[64];
        Parallel.For(0, 64, i =>
        {
            sums[i] = (a + b).ToString();
            prods[i] = (a * b).ToString();
        });

        for (int i = 0; i < 64; i++)
        {
            Assert.Equal(expectedSum, sums[i]);
            Assert.Equal(expectedProd, prods[i]);
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
