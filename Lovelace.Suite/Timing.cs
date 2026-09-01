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
        double totalNanoseconds = elapsed.Ticks * NsPerTick;

        if (totalNanoseconds < 1_000)
            return FormatValue(totalNanoseconds, 0) + " ns";

        double totalMicroseconds = totalNanoseconds / 1_000;
        if (totalMicroseconds < 1_000)
            return FormatValue(totalMicroseconds, 2) + " µs";

        double totalMilliseconds = elapsed.TotalMilliseconds;
        if (totalMilliseconds < 1_000)
            return FormatValue(totalMilliseconds, 2) + " ms";

        double totalSeconds = elapsed.TotalSeconds;
        if (totalSeconds < 60)
            return FormatValue(totalSeconds, 2) + " s";

        double totalMinutes = elapsed.TotalMinutes;
        if (totalMinutes < 60)
            return FormatValue(totalMinutes, 2) + " min";

        return FormatValue(elapsed.TotalHours, 2) + " h";
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
}
