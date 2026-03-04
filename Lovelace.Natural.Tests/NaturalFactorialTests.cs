using Lovelace.Natural;
using Xunit;

namespace Lovelace.Natural.Tests;

public class NaturalFactorialTests
{
    // --- Factorial_GivenZero_ReturnsOne ---
    // C++ fatorial: resultado = 1; the loop body is guarded by naoEZero(),
    // so 0! = 1 directly.
    [Fact]
    public void Factorial_GivenZero_ReturnsOne()
    {
        var result = new Natural(0UL).Factorial();
        Assert.Equal(new Natural(1UL), result);
    }

    // --- Factorial_GivenOne_ReturnsOne ---
    // Loop: aux starts at 2; 2 <= 1 is false, so no iterations. result = 1.
    [Fact]
    public void Factorial_GivenOne_ReturnsOne()
    {
        var result = new Natural(1UL).Factorial();
        Assert.Equal(new Natural(1UL), result);
    }

    // --- Factorial_GivenSmallValue_ReturnsCorrectResult ---
    // 5! = 120.
    [Fact]
    public void Factorial_GivenSmallValue_ReturnsCorrectResult()
    {
        var result = new Natural(5UL).Factorial();
        Assert.Equal(new Natural(120UL), result);
    }

    // --- Factorial_GivenLargeValue_ProducesResultBeyondUlongMax ---
    // 21! = 51090942171709440000, which exceeds ulong.MaxValue (18446744073709551615).
    [Fact]
    public void Factorial_GivenLargeValue_ProducesResultBeyondUlongMax()
    {
        var result = new Natural(21UL).Factorial();
        Assert.Equal("51090942171709440000", result.ToString());
    }
}
