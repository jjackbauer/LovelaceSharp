namespace Lovelace.Abstractions;

/// <summary>A half-open index slice <c>start:stop:step</c>; null means the axis default (0, size, 1).</summary>
public readonly record struct Slice(long? Start, long? Stop, long? Step)
{
    /// <summary>The full-axis slice <c>:</c>.</summary>
    public static Slice All => new(null, null, null);
}

/// <summary>One axis of an index expression: either a scalar index or a <see cref="Slice"/>.</summary>
public readonly record struct IndexSpec(long? Index, Slice? Slice)
{
    public static IndexSpec Scalar(long index) => new(index, null);
    public static IndexSpec Range(long? start, long? stop, long? step) => new(null, new Slice(start, stop, step));
}
