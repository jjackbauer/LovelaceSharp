using Lovelace.Natural;

namespace Lovelace.Natural.Tests;

/// <summary>
/// Functional tests for <see cref="Natural"/> addition:
/// <c>Add(Natural, Natural)</c> and <c>operator+</c>.
/// Checklist item: "static Natural Add(Natural left, Natural right) / operator+".
/// </summary>
public class NaturalAddTests
{
    // ─────────────────────────────────────────────────────────────────────────
    // Basic addition
    // ─────────────────────────────────────────────────────────────────────────

    [Fact]
    public void Add_GivenTwoPositiveNumbers_ReturnsCorrectSum()
    {
        var left  = new Natural(123UL);
        var right = new Natural(456UL);
        var sum   = left + right;
        Assert.Equal("579", sum.ToString());
    }

    [Fact]
    public void Add_GivenZeroAndN_ReturnsN()
    {
        var zero = new Natural(0UL);
        var n    = new Natural(42UL);
        var sum  = zero + n;
        Assert.Equal("42", sum.ToString());
    }

    [Fact]
    public void Add_GivenNAndZero_ReturnsN()
    {
        var n    = new Natural(42UL);
        var zero = new Natural(0UL);
        var sum  = n + zero;
        Assert.Equal("42", sum.ToString());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Carry propagation
    // ─────────────────────────────────────────────────────────────────────────

    [Theory]
    [InlineData(9UL,   1UL,   "10")]
    [InlineData(99UL,  1UL,   "100")]
    [InlineData(999UL, 1UL,   "1000")]
    [InlineData(999UL, 999UL, "1998")]
    public void Add_GivenCarryPropagation_ProducesCorrectResult(ulong a, ulong b, string expected)
    {
        var sum = new Natural(a) + new Natural(b);
        Assert.Equal(expected, sum.ToString());
    }

    [Fact]
    public void Add_GivenSingleDigitCarry_ProducesCorrectResult()
    {
        // 9 + 9 = 18  (single digit result with carry producing two-digit number)
        var sum = new Natural(9UL) + new Natural(9UL);
        Assert.Equal("18", sum.ToString());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Large numbers (beyond ulong)
    // ─────────────────────────────────────────────────────────────────────────

    [Fact]
    public void Add_GivenLargeNumbers_ProducesCorrectSumBeyondUlongMax()
    {
        // ulong.MaxValue = 18446744073709551615
        // 18446744073709551615 + 18446744073709551615 = 36893488147419103230
        var a   = new Natural(ulong.MaxValue);
        var sum = a + a;
        Assert.Equal("36893488147419103230", sum.ToString());
    }

    [Fact]
    public void Add_GivenUlongMaxPlusOne_ProducesCorrectResult()
    {
        // ulong.MaxValue + 1
        // 18446744073709551615 + 1 = 18446744073709551616
        var a   = new Natural(ulong.MaxValue);
        var one = new Natural(1UL);
        var sum = a + one;
        Assert.Equal("18446744073709551616", sum.ToString());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Algebraic properties
    // ─────────────────────────────────────────────────────────────────────────

    [Theory]
    [InlineData(123UL, 456UL)]
    [InlineData(1UL,   999UL)]
    [InlineData(0UL,   12345UL)]
    public void Add_IsCommutative_GivenAnyTwoValues(ulong a, ulong b)
    {
        var na = new Natural(a);
        var nb = new Natural(b);
        Assert.Equal((na + nb).ToString(), (nb + na).ToString());
    }

    [Theory]
    [InlineData(1UL, 2UL, 3UL)]
    [InlineData(100UL, 200UL, 300UL)]
    [InlineData(999UL, 1UL, 1000UL)]
    public void Add_IsAssociative_GivenThreeValues(ulong a, ulong b, ulong c)
    {
        var na = new Natural(a);
        var nb = new Natural(b);
        var nc = new Natural(c);
        Assert.Equal(((na + nb) + nc).ToString(), (na + (nb + nc)).ToString());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Result integrity — no leading zeros
    // ─────────────────────────────────────────────────────────────────────────

    [Theory]
    [InlineData(5UL,   5UL)]
    [InlineData(100UL, 900UL)]
    [InlineData(999UL, 1UL)]
    public void Add_GivenResult_HasNoLeadingZeros(ulong a, ulong b)
    {
        var sum = new Natural(a) + new Natural(b);
        string s = sum.ToString();
        // A non-zero result must not start with '0'
        Assert.False(s.StartsWith('0'), $"Result '{s}' has a spurious leading zero.");
    }
}
