using System.Globalization;
using Lovelace.Suite;
using Xunit;

namespace Lovelace.Suite.Tests;

/// <summary>
/// Structured durations (Cycle 2 item 12): the human `elapsed` string and the machine
/// `{value, unit}` pair come from ONE unit selector, so they can never disagree. An agent reads
/// the pair instead of parsing a suffix.
/// </summary>
public class TimingScaleTests
{
    [Theory]
    [InlineData(0.0)]                       // ns
    [InlineData(0.000_000_5)]               // 500 ns
    [InlineData(0.000_5)]                   // 500 µs
    [InlineData(0.012_57)]                  // 12.57 ms
    [InlineData(2.5)]                       // 2.5 s
    [InlineData(90.0)]                      // 1.5 min
    [InlineData(7200.0)]                    // 2 h
    public void Scale_And_Format_AlwaysAgree(double seconds)
    {
        var elapsed = TimeSpan.FromSeconds(seconds);
        var (value, unit) = Timing.Scale(elapsed);
        var text = Timing.Format(elapsed);

        Assert.EndsWith(" " + unit, text);
        var numeric = text[..^(unit.Length + 1)];
        Assert.Equal(value, double.Parse(numeric, CultureInfo.InvariantCulture));
    }

    [Fact]
    public void Scale_PicksTheLargestWholeUnit()
    {
        Assert.Equal("ns", Timing.Scale(TimeSpan.FromTicks(5)).Unit);
        Assert.Equal("µs", Timing.Scale(TimeSpan.FromTicks(5_000)).Unit);
        Assert.Equal("ms", Timing.Scale(TimeSpan.FromTicks(5_000_000)).Unit);
        Assert.Equal("s", Timing.Scale(TimeSpan.FromSeconds(5)).Unit);
        Assert.Equal("min", Timing.Scale(TimeSpan.FromMinutes(5)).Unit);
        Assert.Equal("h", Timing.Scale(TimeSpan.FromHours(5)).Unit);
    }

    [Fact]
    public void OperationTiming_ExposesTheSameScaleAsItsDisplay()
    {
        var timing = new OperationTiming(0, Value.Void, string.Empty, TimeSpan.FromMilliseconds(3.5));
        Assert.Equal("3.5 ms", timing.ElapsedDisplay);
        Assert.Equal((3.5, "ms"), timing.ElapsedScale);
    }
}
