namespace Lovelace.Abstractions;

/// <summary>
/// The closed interval an integer-semantic argument is allowed to occupy. Declared at the CALL SITE
/// so the bound is visible where the argument is read, and published in the refusal so a caller can
/// act on it without reading the implementation.
/// </summary>
public readonly record struct Int64Range(long Min, long Max)
{
    /// <summary>Every signed 64-bit integer.</summary>
    public static Int64Range Unbounded { get; } = new(long.MinValue, long.MaxValue);

    /// <summary>The counts: 0 and up.</summary>
    public static Int64Range NonNegative { get; } = new(0L, long.MaxValue);

    /// <summary>The counts that are also sizes: 1 and up.</summary>
    public static Int64Range Positive { get; } = new(1L, long.MaxValue);

    public bool Contains(long value) => value >= Min && value <= Max;

    /// <summary>The bound, in bytes, as the refusal message carries it. An unbounded range says so
    /// rather than printing a sentinel.</summary>
    /// <summary>
    /// The bound as the refusal message carries it. It reads as the tail of
    /// "<c>&lt;builtin&gt;(): argument '&lt;name&gt;' must be a Natural or Integer …</c>", so an unbounded range says
    /// so in words rather than printing a sentinel, and every bounded case reads as a phrase.
    /// </summary>
    internal string Describe() => (Min, Max) switch
    {
        (long.MinValue, long.MaxValue) =>
            "that fits a signed 64-bit integer (at most 9223372036854775807 in absolute value)",
        (0L, long.MaxValue) =>
            "that is a non-negative count (0 or more, at most 9223372036854775807)",
        (1L, long.MaxValue) =>
            "that is a positive count (1 or more, at most 9223372036854775807)",
        (long.MinValue, 0L) =>
            "that is a non-positive count (at most 0, at least -9223372036854775808)",
        _ => $"between {Min} and {Max}",
    };
}
