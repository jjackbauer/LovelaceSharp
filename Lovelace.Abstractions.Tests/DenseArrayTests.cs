using Lovelace.Abstractions;

namespace Lovelace.Abstractions.Tests;

public class DenseArrayTests
{
    private static readonly Precision P16 = new(16);

    [Fact]
    public void PackedConstruction_ComputesCanonicalStrides()
    {
        var a = new DenseArray<int>(new long[] { 2, 3, 4 }, new int[24], DType.Integer, P16);

        Assert.Equal(3, a.Rank);
        Assert.Equal(24, a.Numel);
        Assert.Equal(0, a.Offset);
        Assert.True(a.IsContiguous);
        Assert.Equal(new long[] { 2, 3, 4 }, a.Shape.ToArray());
        Assert.Equal(new long[] { 12, 4, 1 }, a.Strides.ToArray());
        Assert.Equal(typeof(int), a.ElementType);
        Assert.Equal(DType.Integer, a.DType);
        Assert.Equal(P16, a.Precision);
    }

    [Fact]
    public void RankOne_IsSupported()
    {
        var a = new DenseArray<int>(new long[] { 5 }, new int[] { 1, 2, 3, 4, 5 }, DType.Natural, P16);

        Assert.Equal(1, a.Rank);
        Assert.Equal(5, a.Numel);
        Assert.Equal(new long[] { 1 }, a.Strides.ToArray());
        Assert.True(a.IsContiguous);
    }

    [Fact]
    public void ZeroLengthDimension_IsSupported()
    {
        var a = new DenseArray<int>(new long[] { 2, 0, 3 }, new int[0], DType.Integer, P16);

        Assert.Equal(3, a.Rank);
        Assert.Equal(0, a.Numel);
        Assert.True(a.IsContiguous);
        Assert.Equal(new long[] { 2, 0, 3 }, a.Shape.ToArray());
    }

    [Fact]
    public void EmptyShape_Throws()
    {
        Assert.Throws<ArgumentException>(() =>
            new DenseArray<int>(Array.Empty<long>(), new int[0], DType.Integer, P16));
    }

    [Fact]
    public void NegativeDimension_Throws()
    {
        Assert.Throws<ArgumentException>(() =>
            new DenseArray<int>(new long[] { 2, -1 }, new int[0], DType.Integer, P16));
    }

    [Fact]
    public void BufferLengthMismatch_Throws()
    {
        Assert.Throws<ArgumentException>(() =>
            new DenseArray<int>(new long[] { 2, 2 }, new int[3], DType.Integer, P16));
    }

    [Fact]
    public void GetElement_Flat_IsRowMajorOrder()
    {
        var a = new DenseArray<int>(new long[] { 2, 3 }, new int[] { 0, 1, 2, 3, 4, 5 }, DType.Integer, P16);

        for (int i = 0; i < 6; i++)
            Assert.Equal(i, (int)a.GetElement(i));
    }

    [Fact]
    public void GetElement_Coordinates_MatchesFlat()
    {
        var a = new DenseArray<int>(new long[] { 2, 3 }, new int[] { 0, 1, 2, 3, 4, 5 }, DType.Integer, P16);

        Assert.Equal(5, (int)a.GetElement(new long[] { 1, 2 }));
        Assert.Equal(3, (int)a.GetElement(new long[] { 1, 0 }));
    }

    [Fact]
    public void GetElement_OutOfRange_Throws()
    {
        var a = new DenseArray<int>(new long[] { 2, 2 }, new int[] { 0, 1, 2, 3 }, DType.Integer, P16);

        Assert.Throws<ArgumentOutOfRangeException>(() => a.GetElement(4));
        Assert.Throws<InvalidOperationException>(() => a.GetElement(new long[] { 2, 0 }));
    }

    [Fact]
    public void NonContiguousView_IsNotContiguous_AndMapsCorrectly()
    {
        // A 2x3 matrix [[0,1,2],[3,4,5]] viewed as its 3x2 transpose.
        // Transposed logical order is [0,3,1,4,2,5].
        var buffer = new int[] { 0, 1, 2, 3, 4, 5 };
        var t = new DenseArray<int>(new long[] { 3, 2 }, new long[] { 1, 3 }, 0, buffer, DType.Integer, P16);

        Assert.False(t.IsContiguous);
        Assert.Equal(6, t.Numel);
        Assert.Equal(new int[] { 0, 3, 1, 4, 2, 5 },
            Enumerable.Range(0, 6).Select(i => (int)t.GetElement(i)).ToArray());
    }

    [Fact]
    public void AsContiguous_MaterializesPackedCopy()
    {
        var buffer = new int[] { 0, 1, 2, 3, 4, 5 };
        var t = new DenseArray<int>(new long[] { 3, 2 }, new long[] { 1, 3 }, 0, buffer, DType.Integer, P16);

        var c = (DenseArray<int>)t.AsContiguous();

        Assert.True(c.IsContiguous);
        Assert.Equal(0, c.Offset);
        Assert.Equal(new long[] { 2, 1 }, c.Strides.ToArray());
        Assert.Equal(new int[] { 0, 3, 1, 4, 2, 5 }, c.AsSpan().ToArray());
    }

    [Fact]
    public void AsSpan_OnNonContiguous_Throws()
    {
        var buffer = new int[] { 0, 1, 2, 3, 4, 5 };
        var t = new DenseArray<int>(new long[] { 3, 2 }, new long[] { 1, 3 }, 0, buffer, DType.Integer, P16);

        Assert.Throws<InvalidOperationException>(() => t.AsSpan());
    }

    [Fact]
    public void AsSpan_OnContiguous_ReturnsFullBuffer()
    {
        var a = new DenseArray<int>(new long[] { 2, 3 }, new int[] { 0, 1, 2, 3, 4, 5 }, DType.Integer, P16);

        Assert.Equal(new[] { 0, 1, 2, 3, 4, 5 }, a.AsSpan().ToArray());
    }

    [Fact]
    public void ViewOutsideBuffer_Throws()
    {
        var buffer = new int[] { 0, 1, 2, 3, 4, 5 };

        Assert.Throws<ArgumentException>(() =>
            new DenseArray<int>(new long[] { 3, 2 }, new long[] { 1, 3 }, 1, buffer, DType.Integer, P16));
    }

    [Fact]
    public void ArrayValue_TypeIsExposedAsBase()
    {
        ArrayValue v = new DenseArray<int>(new long[] { 2 }, new int[] { 7, 8 }, DType.Integer, P16);

        Assert.Equal(2, v.Numel);
        Assert.Equal(7, (int)v.GetElement(0));
    }

    [Fact]
    public void Transpose_ReturnsZeroCopyView()
    {
        var buffer = new int[] { 0, 1, 2, 3, 4, 5 }; // 2x3
        var a = new DenseArray<int>(new long[] { 2, 3 }, buffer, DType.Integer, P16);

        var t = (DenseArray<int>)a.Transpose(null);

        Assert.False(t.IsContiguous);
        Assert.Equal(new long[] { 3, 2 }, t.Shape.ToArray());
        Assert.Equal(new long[] { 1, 3 }, t.Strides.ToArray());
        Assert.Equal(new int[] { 0, 3, 1, 4, 2, 5 },
            Enumerable.Range(0, 6).Select(i => (int)t.GetElement(i)).ToArray());
    }

    [Fact]
    public void Reshape_Contiguous_ReturnsZeroCopyView()
    {
        var a = new DenseArray<int>(new long[] { 2, 3 }, new int[] { 0, 1, 2, 3, 4, 5 }, DType.Integer, P16);

        var r = a.Reshape(new long[] { 3, 2 });

        Assert.True(r.IsContiguous);
        Assert.Equal(new long[] { 3, 2 }, r.Shape.ToArray());
        Assert.Equal(new int[] { 0, 1, 2, 3, 4, 5 },
            Enumerable.Range(0, 6).Select(i => (int)r.GetElement(i)).ToArray());
    }

    [Fact]
    public void Reshape_NonContiguous_Materializes()
    {
        var buffer = new int[] { 0, 1, 2, 3, 4, 5 };
        var t = new DenseArray<int>(new long[] { 3, 2 }, new long[] { 1, 3 }, 0, buffer, DType.Integer, P16);

        var r = t.Reshape(new long[] { 2, 3 });

        Assert.True(r.IsContiguous);
        Assert.Equal(new long[] { 2, 3 }, r.Shape.ToArray());
        Assert.Equal(new int[] { 0, 3, 1, 4, 2, 5 },
            Enumerable.Range(0, 6).Select(i => (int)r.GetElement(i)).ToArray());
    }
}
