namespace Lovelace.Abstractions;

/// <summary>
/// Language-facing, non-generic handle for a homogeneous N-dimensional array: a buffer
/// plus offset, shape, and row-major strides, with first-class dtype and precision
/// metadata (ARR-004, STO-001). Immutable by contract — every operation returns a new
/// instance. Kernels operate on concrete <see cref="DenseArray{T}"/> spans; this base is
/// the metadata and boundary-access surface.
/// </summary>
public abstract class ArrayValue
{
    public abstract DType DType { get; }
    public abstract Precision Precision { get; }
    public abstract int Rank { get; }
    public abstract ReadOnlyMemory<long> Shape { get; }
    public abstract ReadOnlyMemory<long> Strides { get; }
    public abstract long Offset { get; }
    public abstract long Numel { get; }
    public abstract Type ElementType { get; }
    public abstract bool IsContiguous { get; }
    public abstract ArrayValue AsContiguous();

    /// <summary>Returns a zero-copy view with the given axis permutation (null = reverse axes).</summary>
    public abstract ArrayValue Transpose(long[]? permutation);

    /// <summary>Returns this data viewed under a new shape (zero-copy when contiguous; else materializes).</summary>
    public abstract ArrayValue Reshape(long[] shape);

    /// <summary>Returns a (zero-copy where possible) view from per-axis scalar/slice index specs.</summary>
    public abstract ArrayValue Slice(IReadOnlyList<IndexSpec> specs);

    /// <summary>Reads the element at a logical flat row-major index in [0, Numel).</summary>
    public abstract object GetElement(long flatIndex);

    /// <summary>Reads the element at an n-dimensional coordinate (one index per axis).</summary>
    public object GetElement(ReadOnlySpan<long> indices) => GetElement(FlatIndex(indices));

    /// <summary>True when the given shape/strides describe a packed row-major layout.</summary>
    protected static bool IsPacked(long[] shape, long[] strides)
    {
        var packed = ComputeStrides(shape);
        for (int i = 0; i < packed.Length; i++)
            if (packed[i] != strides[i])
                return false;
        return true;
    }

    /// <summary>Row-major strides: <c>strides[i] = product(shape[i+1..])</c>, innermost is 1.</summary>
    protected static long[] ComputeStrides(long[] shape)
    {
        var s = new long[shape.Length];
        long acc = 1;
        for (int i = shape.Length - 1; i >= 0; i--)
        {
            s[i] = acc;
            acc = checked(acc * shape[i]);
        }
        return s;
    }

    /// <summary>Maps an n-dimensional coordinate to a logical flat index.</summary>
    protected long FlatIndex(ReadOnlySpan<long> indices)
    {
        if (indices.Length != Rank)
            throw new ArgumentException($"A rank-{Rank} array requires {Rank} index(es), but got {indices.Length}.");

        var shape = Shape.Span;
        var strides = Strides.Span;
        long flat = 0;
        for (int i = 0; i < Rank; i++)
        {
            long idx = indices[i];
            if (idx < 0 || idx >= shape[i])
                throw new InvalidOperationException($"Index {idx} is out of range for dimension {i} of size {shape[i]}.");
            flat += idx * strides[i];
        }
        return flat;
    }
}
