using Lovelace.Suite;

namespace Lovelace.Suite.Tests;

public class TimingTests
{
    [Fact]
    public void Format_GivenNanoseconds_ReturnsNsScale()
    {
        Assert.Equal("0 ns", Timing.Format(TimeSpan.Zero));
        Assert.Equal("500 ns", Timing.Format(TimeSpan.FromTicks(5)));
    }

    [Fact]
    public void Format_GivenMicroseconds_ReturnsUsScale()
    {
        Assert.Equal("1.5 µs", Timing.Format(TimeSpan.FromTicks(15)));
        Assert.Equal("5 µs", Timing.Format(TimeSpan.FromTicks(50)));
    }

    [Fact]
    public void Format_GivenMilliseconds_ReturnsMsScale()
    {
        Assert.Equal("12.5 ms", Timing.Format(TimeSpan.FromMilliseconds(12.5)));
    }

    [Fact]
    public void Format_GivenSeconds_ReturnsSecondsScale()
    {
        Assert.Equal("2.5 s", Timing.Format(TimeSpan.FromSeconds(2.5)));
    }

    [Fact]
    public void Format_GivenMinutes_ReturnsMinutesScale()
    {
        Assert.Equal("1.5 min", Timing.Format(TimeSpan.FromMinutes(1.5)));
    }

    [Fact]
    public void Format_GivenHours_ReturnsHoursScale()
    {
        Assert.Equal("2.25 h", Timing.Format(TimeSpan.FromHours(2.25)));
    }
}
