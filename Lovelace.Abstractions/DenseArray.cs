namespace Lovelace.Abstractions;

/// <summary>
/// Homogeneous dense N-dimensional array backed by a single <c>T[]</c> buffer with an
/// offset, shape, and row-major strides (STO-001). The offset/strides enable zero-copy
/// views while the buffer stays packed and contiguous for the common case. Immutable by
/// contract — the buffer is shared by reference, but no operation mutates it.
/// </summary>
public sealed class DenseArray<T> : ArrayValue
{
    private readonly T[] _buffer;
    private readonly long[] _shape;
    private readonly long[] _strides;
    private readonly long _offset;
    private readonly long _numel;
    private readonly bool _isContiguous;

    /// <summary>Packed construction: offset 0, canonical row-major strides.</summary>
    public DenseArray(long[] shape, T[] buffer, DType dtype, Precision precision)
    {
        ArgumentNullException.ThrowIfNull(shape);
        ArgumentNullException.ThrowIfNull(buffer);
        ValidateShape(shape);

        _numel = Product(shape);
        if (buffer.Length != _numel)
            throw new ArgumentException(
                $"Shape [{string.Join(", ", shape)}] requires {_numel} element(s), but got {buffer.Length}.", nameof(buffer));

        _shape = (long[])shape.Clone();
        _strides = ComputeStrides(_shape);
        _offset = 0;
        _isContiguous = true;
        _buffer = buffer;
        DType = dtype;
        Precision = precision;
    }

    /// <summary>View construction: explicit strides and offset over a shared buffer.</summary>
    public DenseArray(long[] shape, long[] strides, long offset, T[] buffer, DType dtype, Precision precision)
    {
        ArgumentNullException.ThrowIfNull(shape);
        ArgumentNullException.ThrowIfNull(strides);
        ArgumentNullException.ThrowIfNull(buffer);
        ValidateShape(shape);

        if (strides.Length != shape.Length)
            throw new ArgumentException($"Strides length {strides.Length} must match rank {shape.Length}.", nameof(strides));
        if (offset < 0)
            throw new ArgumentOutOfRangeException(nameof(offset), "Offset must be non-negative.");

        _numel = Product(shape);
        if (_numel > 0)
        {
            var (min, max) = Reach(shape, strides, offset);
            if (min < 0 || max >= buffer.Length)
                throw new ArgumentException("View lies outside the buffer bounds.", nameof(buffer));
        }

        _shape = (long[])shape.Clone();
        _strides = (long[])strides.Clone();
        _offset = offset;
        _isContiguous = offset == 0 && IsPacked(shape, strides);
        _buffer = buffer;
        DType = dtype;
        Precision = precision;
    }

    public override DType DType { get; }
    public override Precision Precision { get; }
    public override int Rank => _shape.Length;
    public override ReadOnlyMemory<long> Shape => _shape;
    public override ReadOnlyMemory<long> Strides => _strides;
    public override long Offset => _offset;
    public override long Numel => _numel;
    public override Type ElementType => typeof(T);
    public override bool IsContiguous => _isContiguous;

    public override object GetElement(long flatIndex)
    {
        if (flatIndex < 0 || flatIndex >= _numel)
            throw new ArgumentOutOfRangeException(nameof(flatIndex));
        return _isContiguous ? _buffer[_offset + flatIndex]! : _buffer[FlatIndexToBuffer(flatIndex)]!;
    }

    public override ArrayValue AsContiguous()
    {
        if (IsContiguous)
            return this;

        var copy = new T[_numel];
        for (long i = 0; i < _numel; i++)
            copy[i] = _buffer[FlatIndexToBuffer(i)];
        return new DenseArray<T>(_shape, copy, DType, Precision);
    }

    /// <summary>Reads the contiguous span; valid only when <see cref="IsContiguous"/>.</summary>
    public ReadOnlySpan<T> AsSpan()
    {
        if (!IsContiguous)
            throw new InvalidOperationException("A non-contiguous view must be materialized with AsContiguous() first.");
        return _buffer.AsSpan(checked((int)_offset), checked((int)_numel));
    }

    public override ArrayValue Transpose(long[]? permutation)
    {
        var perm = permutation ?? ReversePermutation();
        if (perm.Length != Rank)
            throw new ArgumentException($"transpose() expects a permutation of {Rank} axes.", nameof(permutation));
        var seen = new bool[Rank];
        foreach (var p in perm)
        {
            if (p < 0 || p >= Rank || seen[p])
                throw new ArgumentException("transpose() permutation must be a valid reordering of the axes.", nameof(permutation));
            seen[p] = true;
        }

        var newShape = new long[Rank];
        var newStrides = new long[Rank];
        for (int i = 0; i < Rank; i++)
        {
            newShape[i] = _shape[(int)perm[i]];
            newStrides[i] = _strides[(int)perm[i]];
        }
        return new DenseArray<T>(newShape, newStrides, _offset, _buffer, DType, Precision);
    }

    public override ArrayValue Reshape(long[] shape)
    {
        ArgumentNullException.ThrowIfNull(shape);
        ValidateShape(shape);

        long total = Product(shape);
        if (total != _numel)
            throw new ArgumentException(
                $"reshape() to [{string.Join(", ", shape)}] requires {total} element(s), but this array has {_numel}.", nameof(shape));

        if (IsContiguous)
            return new DenseArray<T>(shape, _buffer, DType, Precision);
        return AsContiguous().Reshape(shape);
    }

    public override ArrayValue Slice(IReadOnlyList<IndexSpec> specs)
    {
        var newShape = new List<long>(Rank);
        var newStrides = new List<long>(Rank);
        long offset = _offset;

        for (int axis = 0; axis < Rank; axis++)
        {
            var spec = axis < specs.Count ? specs[axis] : default;
            long dim = _shape[axis];
            long stride = _strides[axis];

            if (spec.Index is long idx)
            {
                if (idx < 0) idx += dim;
                if (idx < 0 || idx >= dim)
                    throw new InvalidOperationException($"Index {idx} is out of range for dimension {axis} of size {dim}.");
                offset += idx * stride;
                continue; // drop this axis
            }

            var s = spec.Slice ?? global::Lovelace.Abstractions.Slice.All;
            long start = s.Start ?? 0;
            long stop = s.Stop ?? dim;
            long step = s.Step ?? 1;
            if (step <= 0)
                throw new ArgumentException("Slice step must be positive.");

            if (start < 0) start += dim;
            if (stop < 0) stop += dim;
            start = Math.Clamp(start, 0, dim);
            stop = Math.Clamp(stop, 0, dim);

            long size = start < stop ? (stop - start + step - 1) / step : 0;
            if (size == 0)
            {
                newShape.Add(0);
                newStrides.Add(stride);
            }
            else
            {
                offset += start * stride;
                newShape.Add(size);
                newStrides.Add(step * stride);
            }
        }

        if (newShape.Count == 0)
            throw new InvalidOperationException("A full scalar index returns an element, not an array.");

        return new DenseArray<T>(newShape.ToArray(), newStrides.ToArray(), offset, _buffer, DType, Precision);
    }

    private long[] ReversePermutation()
    {
        var p = new long[Rank];
        for (int i = 0; i < Rank; i++)
            p[i] = Rank - 1 - i;
        return p;
    }

    /// <summary>Maps a logical flat row-major index to the underlying buffer index.</summary>
    private long FlatIndexToBuffer(long flat)
    {
        long off = _offset;
        long rem = flat;
        for (int i = _shape.Length - 1; i >= 0; i--)
        {
            long coord = rem % _shape[i];
            rem /= _shape[i];
            off += coord * _strides[i];
        }
        return off;
    }

    /// <summary>The [min, max] buffer indices this view can reach.</summary>
    private static (long min, long max) Reach(long[] shape, long[] strides, long offset)
    {
        long min = offset;
        long max = offset;
        for (int i = 0; i < shape.Length; i++)
        {
            if (shape[i] <= 0)
                continue;
            long hi = shape[i] - 1;
            min += Math.Min(0, hi * strides[i]);
            max += Math.Max(0, hi * strides[i]);
        }
        return (min, max);
    }

    private static void ValidateShape(long[] shape)
    {
        if (shape.Length == 0)
            throw new ArgumentException("An array must have at least one dimension.", nameof(shape));
        foreach (var d in shape)
            if (d < 0)
                throw new ArgumentException($"Array dimensions must be non-negative, but got {d}.", nameof(shape));
    }

    private static long Product(long[] shape)
    {
        long total = 1;
        foreach (var d in shape)
            total = checked(total * d);
        return total;
    }

    public override string ToString() =>
        $"DenseArray<{typeof(T).Name}>(shape [{string.Join(", ", _shape)}], dtype {DType})";
}
