using Lovelace.Natural;

namespace Lovelace.Natural.Tests;

/// <summary>
/// Functional tests for <see cref="Natural"/> increment operator:
/// <c>operator++</c> (prefix and postfix).
/// Checklist item: "operator++ (prefix &amp; postfix) — IIncrementOperators&lt;Natural&gt;".
/// Maps to C++ <c>incrementar()</c> which calls <c>somar(aux=1)</c>.
/// </summary>
public class NaturalIncrementTests
{
    // ─────────────────────────────────────────────────────────────────────────
    // Prefix increment (++n)
    // ─────────────────────────────────────────────────────────────────────────

    [Fact]
    public void IncrementPrefix_GivenZero_ReturnsOne()
    {
        var n = new Natural(0UL);
        ++n;
        Assert.Equal("1", n.ToString());
    }

    [Fact]
    public void IncrementPrefix_GivenNine_ReturnsTen()
    {
        // Carry must propagate from digit 0 into digit 1.
        var n = new Natural(9UL);
        ++n;
        Assert.Equal("10", n.ToString());
    }

    [Theory]
    [InlineData(99UL,    "100")]
    [InlineData(999UL,   "1000")]
    [InlineData(9999UL,  "10000")]
    public void IncrementPrefix_GivenAllNines_PropagatesCarryCorrectly(ulong initial, string expected)
    {
        var n = new Natural(initial);
        ++n;
        Assert.Equal(expected, n.ToString());
    }

    [Fact]
    public void IncrementPrefix_GivenArbitraryValue_ReturnsValuePlusOne()
    {
        var n = new Natural(12345UL);
        ++n;
        Assert.Equal("12346", n.ToString());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Postfix increment (n++)
    // ─────────────────────────────────────────────────────────────────────────

    [Fact]
    public void IncrementPostfix_GivenValue_ReturnsPreviousValue()
    {
        // n++ must yield the pre-increment value and leave n incremented.
        var n   = new Natural(5UL);
        var old = n++;

        // The returned value is the state BEFORE increment.
        Assert.Equal("5", old.ToString());
        // The variable itself is now incremented.
        Assert.Equal("6", n.ToString());
    }

    [Fact]
    public void IncrementPostfix_GivenZero_ReturnsPreviousValueAndIncrementsVariable()
    {
        var n   = new Natural(0UL);
        var old = n++;

        Assert.Equal("0", old.ToString());
        Assert.Equal("1", n.ToString());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Multi-step increments
    // ─────────────────────────────────────────────────────────────────────────

    [Fact]
    public void IncrementPrefix_AppliedMultipleTimes_AccumulatesCorrectly()
    {
        var n = new Natural(0UL);
        for (int i = 0; i < 10; i++)
            ++n;

        Assert.Equal("10", n.ToString());
    }
}
