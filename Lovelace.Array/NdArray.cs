namespace Lovelace.Arrays;

/// <summary>
/// A generic N-dimensional array: an explicit shape (ordered positive dimension sizes)
/// plus row-major flat data. Rank 1 is a vector, rank 2 a matrix, rank &gt;= 3 an N-D
/// array. Values are immutable: every operation returns a new array.
/// </summary>
public sealed class NdArray<T>
{
    /// <summary>Dimension sizes, outermost (index 0) to innermost (index Rank-1).</summary>
    public long[] Shape { get; }

    /// <summary>Row-major flat element storage (the last index varies fastest).</summary>
    public IReadOnlyList<T> Data { get; }

    /// <summary>Number of dimensions.</summary>
    public int Rank => Shape.Length;

    /// <summary>Total element count.</summary>
    public long Numel => Data.Count;

    /// <summary>Strides of length <c>Rank + 1</c>; <c>Strides[i]</c> is the product of <c>Shape[i..Rank]</c>.</summary>
    public long[] Strides { get; }

    public NdArray(IReadOnlyList<long> shape, IReadOnlyList<T> data)
    {
        if (shape is null || shape.Count == 0)
            throw new ArgumentException("An array must have at least one dimension.", nameof(shape));

        long total = 1;
        foreach (var d in shape)
        {
            if (d < 1)
                throw new ArgumentException($"Array dimensions must be positive, but got {d}.", nameof(shape));
            total *= d;
        }

        if (data is null || data.Count != total)
            throw new ArgumentException(
                $"Array shape [{string.Join(", ", shape)}] requires {total} element(s), but got {data?.Count ?? 0}.",
                nameof(data));

        Shape = shape.ToArray();
        Data = data;
        Strides = ComputeStrides(Shape);
    }

    // ------------------------------------------------------------------
    // Indexing
    // ------------------------------------------------------------------

    /// <summary>Returns the element at a full index (one coordinate per dimension).</summary>
    public T Get(IReadOnlyList<long> indices)
    {
        if (indices is null || indices.Count != Rank)
            throw new ArgumentException($"A rank-{Rank} array requires exactly {Rank} index(es), but got {indices?.Count ?? 0}.");

        return Data[(int)Offset(indices)];
    }

    /// <summary>
    /// Returns a lower-rank sub-array from a partial index of 1..Rank-1 leading
    /// coordinates. The result keeps the trailing dimensions.
    /// </summary>
    public NdArray<T> Slice(IReadOnlyList<long> indices)
    {
        if (indices is null || indices.Count == 0 || indices.Count >= Rank)
            throw new ArgumentException($"A partial index for a rank-{Rank} array needs 1..{Rank - 1} coordinate(s), but got {indices?.Count ?? 0}.");

        long offset = Offset(indices);

        long[] subShape = Shape.Skip(indices.Count).ToArray();
        long count = Strides[indices.Count];
        var sub = new List<T>((int)count);
        for (long t = 0; t < count; t++)
            sub.Add(Data[(int)(offset + t)]);

        return new NdArray<T>(subShape, sub);
    }

    // ------------------------------------------------------------------
    // Shape algebra
    // ------------------------------------------------------------------

    /// <summary>Returns the same data viewed under a new shape; element count must match.</summary>
    public NdArray<T> Reshape(IReadOnlyList<long> shape)
    {
        long total = Product(shape);
        if (total != Numel)
            throw new ArgumentException($"reshape() to [{string.Join(", ", shape)}] requires {total} element(s), but this array has {Numel}.");
        return new NdArray<T>(shape, Data);
    }

    /// <summary>Returns a rank-1 view/copy of the row-major data.</summary>
    public NdArray<T> Flatten() => new NdArray<T>(new[] { Numel }, Data);

    /// <summary>Transposes by reversing the axes.</summary>
    public NdArray<T> Transpose() => Transpose(ReversePermutation());

    /// <summary>Transposes by reordering axes according to a permutation of <c>0..Rank-1</c>.</summary>
    public NdArray<T> Transpose(IReadOnlyList<long> permutation)
    {
        int r = Rank;
        var p = ValidatePermutation(permutation, r);

        long[] outShape = new long[r];
        for (int i = 0; i < r; i++)
            outShape[i] = Shape[p[i]];

        long[] outStrides = ComputeStrides(outShape);
        var inv = new int[r];
        for (int i = 0; i < r; i++)
            inv[p[i]] = i;

        var outData = new List<T>((int)Numel);
        for (long lin = 0; lin < Numel; lin++)
        {
            var coords = new long[r];
            for (int i = 0; i < r; i++)
                coords[i] = (lin / outStrides[i + 1]) % outShape[i];

            long inOffset = 0;
            for (int i = 0; i < r; i++)
                inOffset += coords[inv[i]] * Strides[i + 1];

            outData.Add(Data[(int)inOffset]);
        }

        return new NdArray<T>(outShape, outData);
    }

    /// <summary>Removes size-1 dimensions. A shape of all singletons collapses to rank 1.</summary>
    public NdArray<T> Squeeze()
    {
        var newShape = Shape.Where(d => d != 1).ToArray();
        if (newShape.Length == 0)
            newShape = new[] { 1L };
        return new NdArray<T>(newShape, Data);
    }

    // ------------------------------------------------------------------
    // Construction
    // ------------------------------------------------------------------

    /// <summary>Builds a constant array of the given shape.</summary>
    public static NdArray<T> Fill(IReadOnlyList<long> shape, T value)
    {
        long total = Product(shape);
        var data = Enumerable.Repeat(value, (int)total).ToList();
        return new NdArray<T>(shape, data);
    }

    /// <summary>Concatenates two arrays along one axis (equal rank; shapes match except on that axis).</summary>
    public static NdArray<T> Concat(NdArray<T> a, NdArray<T> b, long axis)
    {
        if (a.Rank != b.Rank)
            throw new ArgumentException($"concat() operands must have the same rank ({a.Rank} vs {b.Rank}).");

        int r = a.Rank;
        int ax = CheckAxis(axis, r);

        for (int i = 0; i < r; i++)
        {
            if (i != ax && a.Shape[i] != b.Shape[i])
                throw new ArgumentException($"concat() operands must have the same shape except along axis {ax}.");
        }

        long[] outShape = (long[])a.Shape.Clone();
        outShape[ax] = a.Shape[ax] + b.Shape[ax];

        long numel = Product(outShape);
        long[] outStrides = ComputeStrides(outShape);
        var outData = new List<T>((int)numel);

        for (long lin = 0; lin < numel; lin++)
        {
            var c = new long[r];
            for (int i = 0; i < r; i++)
                c[i] = (lin / outStrides[i + 1]) % outShape[i];

            if (c[ax] < a.Shape[ax])
            {
                outData.Add(a.Data[(int)Linear(a.Strides, c)]);
            }
            else
            {
                var bc = (long[])c.Clone();
                bc[ax] -= a.Shape[ax];
                outData.Add(b.Data[(int)Linear(b.Strides, bc)]);
            }
        }

        return new NdArray<T>(outShape, outData);
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    private long Offset(IReadOnlyList<long> indices)
    {
        long offset = 0;
        for (int i = 0; i < indices.Count; i++)
        {
            long idx = indices[i];
            if (idx < 0 || idx >= Shape[i])
                throw new InvalidOperationException($"Index {idx} is out of range for dimension {i} of size {Shape[i]}.");
            offset += idx * Strides[i + 1];
        }
        return offset;
    }

    private static long Linear(long[] strides, long[] coords)
    {
        long offset = 0;
        for (int i = 0; i < coords.Length; i++)
            offset += coords[i] * strides[i + 1];
        return offset;
    }

    private long[] ReversePermutation()
    {
        var p = new long[Rank];
        for (int i = 0; i < Rank; i++)
            p[i] = Rank - 1 - i;
        return p;
    }

    private static long[] ValidatePermutation(IReadOnlyList<long> permutation, int rank)
    {
        if (permutation is null || permutation.Count != rank)
            throw new ArgumentException($"transpose() expects a permutation of {rank} axes.");

        var p = permutation.ToArray();
        var seen = new bool[rank];
        foreach (var x in p)
        {
            if (x < 0 || x >= rank || seen[x])
                throw new ArgumentException("transpose() permutation must be a valid reordering of the axes.");
            seen[x] = true;
        }
        return p;
    }

    private static int CheckAxis(long axis, int rank)
    {
        if (axis < 0 || axis >= rank)
            throw new ArgumentOutOfRangeException(nameof(axis), $"Axis {axis} is out of range for rank {rank}.");
        return (int)axis;
    }

    /// <summary>Computes strides: <c>s[i] = product(shape[i..])</c>, with <c>s[rank] = 1</c>.</summary>
    internal static long[] ComputeStrides(long[] shape)
    {
        int r = shape.Length;
        var s = new long[r + 1];
        s[r] = 1;
        for (int i = r - 1; i >= 0; i--)
            s[i] = s[i + 1] * shape[i];
        return s;
    }

    /// <summary>Product of the shape, validating positivity on the way.</summary>
    internal static long Product(IReadOnlyList<long> shape)
    {
        if (shape is null || shape.Count == 0)
            throw new ArgumentException("A shape must have at least one dimension.");
        long total = 1;
        foreach (var d in shape)
        {
            if (d < 1)
                throw new ArgumentException($"Array dimensions must be positive, but got {d}.");
            total *= d;
        }
        return total;
    }

    public override string ToString() => $"NdArray<{typeof(T).Name}>(shape [{string.Join(", ", Shape)}])";
}
