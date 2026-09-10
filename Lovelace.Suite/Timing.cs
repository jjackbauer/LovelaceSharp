using System.Globalization;

namespace Lovelace.Suite;

/// <summary>
/// Formats an elapsed <see cref="TimeSpan"/> with an automatically selected unit
/// scale. Short durations render in <c>ns</c>/<c>µs</c>/<c>ms</c>, longer ones in
/// <c>s</c>/<c>min</c>/<c>h</c>, so timing output stays compact and readable at
/// any magnitude without a fixed unit or hard-coded precision.
/// </summary>
public static class Timing
{
    // One TimeSpan tick is exactly 100 nanoseconds.
    private const double NsPerTick = 100.0;

    /// <summary>
    /// Formats <paramref name="elapsed"/> using the largest unit whose value is at
    /// least one whole unit, falling back to nanoseconds at the bottom of the scale.
    /// Fractional values use at most two decimal places (trailing zeros trimmed).
    /// </summary>
    public static string Format(TimeSpan elapsed)
    {
        var (value, unit) = Scale(elapsed);
        return FormatValue(value, unit == "ns" ? 0 : 2) + " " + unit;
    }

    /// <summary>
    /// The auto-scaled <c>(value, unit)</c> pair behind <see cref="Format"/>. Agents read this
    /// instead of parsing a unit suffix; one selector drives both forms, so the human string and
    /// the structured duration can never disagree.
    /// </summary>
    public static (double Value, string Unit) Scale(TimeSpan elapsed)
    {
        double totalNanoseconds = elapsed.Ticks * NsPerTick;

        if (totalNanoseconds < 1_000)
            return (Math.Round(totalNanoseconds), "ns");

        double totalMicroseconds = totalNanoseconds / 1_000;
        if (totalMicroseconds < 1_000)
            return (Math.Round(totalMicroseconds, 2), "µs");

        double totalMilliseconds = elapsed.TotalMilliseconds;
        if (totalMilliseconds < 1_000)
            return (Math.Round(totalMilliseconds, 2), "ms");

        double totalSeconds = elapsed.TotalSeconds;
        if (totalSeconds < 60)
            return (Math.Round(totalSeconds, 2), "s");

        double totalMinutes = elapsed.TotalMinutes;
        if (totalMinutes < 60)
            return (Math.Round(totalMinutes, 2), "min");

        return (Math.Round(elapsed.TotalHours, 2), "h");
    }

    private static string FormatValue(double value, int maxDecimals)
    {
        string format = maxDecimals == 0 ? "0" : "0." + new string('#', maxDecimals);
        return value.ToString(format, CultureInfo.InvariantCulture);
    }
}

/// <summary>
/// A timed top-level operation (one script statement): its zero-based source position,
/// the result value it produced, any print output it wrote, and its elapsed time.
/// </summary>
public sealed record OperationTiming(int Position, Value Result, string Output, TimeSpan Elapsed)
{
    /// <summary><see cref="Elapsed"/> rendered with an auto-scaled unit (ns/µs/ms/…).</summary>
    public string ElapsedDisplay => Timing.Format(Elapsed);

    /// <summary><see cref="Elapsed"/> as a unit-scaled value/unit pair (never a parsed suffix).</summary>
    public (double Value, string Unit) ElapsedScale => Timing.Scale(Elapsed);
}
