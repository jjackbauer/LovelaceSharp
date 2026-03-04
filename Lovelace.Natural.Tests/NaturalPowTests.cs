using Lovelace.Natural;
using Xunit;

namespace Lovelace.Natural.Tests;

public class NaturalPowTests
{
    // --- Pow_GivenExponentZero_ReturnsOne ---
    // C++ exponenciar: resultado initialised to 1, loop guard is if (!(X.eZero())),
    // so any base raised to 0 returns 1.
    [Fact]
    public void Pow_GivenExponentZero_ReturnsOne()
    {
        var result = new Natural(7UL).Pow(new Natural(0UL));
        Assert.Equal(new Natural(1UL), result);
    }

    // --- Pow_GivenExponentOne_ReturnsSelf ---
    // Loop executes once: result = 1 * base = base.
    [Fact]
    public void Pow_GivenExponentOne_ReturnsSelf()
    {
        var result = new Natural(13UL).Pow(new Natural(1UL));
        Assert.Equal(new Natural(13UL), result);
    }

    // --- Pow_GivenKnownValue_ReturnsCorrectPower ---
    // 2^10 = 1024.
    [Fact]
    public void Pow_GivenKnownValue_ReturnsCorrectPower()
    {
        var result = new Natural(2UL).Pow(new Natural(10UL));
        Assert.Equal(new Natural(1024UL), result);
    }

    // --- Pow_GivenBaseZeroAndNonZeroExponent_ReturnsZero ---
    // Loop runs 5 times, each iteration: result *= 0 → result = 0.
    [Fact]
    public void Pow_GivenBaseZeroAndNonZeroExponent_ReturnsZero()
    {
        var result = new Natural(0UL).Pow(new Natural(5UL));
        Assert.True(Natural.IsZero(result));
    }

    // --- Pow_GivenLargeExponent_ProducesCorrectResult ---
    // 2^64 = 18446744073709551616 (one more than ulong.MaxValue).
    [Fact]
    public void Pow_GivenLargeExponent_ProducesCorrectResult()
    {
        var result = new Natural(2UL).Pow(new Natural(64UL));
        Assert.Equal("18446744073709551616", result.ToString());
    }
}
