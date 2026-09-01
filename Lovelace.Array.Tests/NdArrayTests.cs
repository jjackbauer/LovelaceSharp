using Lovelace.Arrays;

namespace Lovelace.Array.Tests;

/// <summary>A minimal <see cref="IField{T}"/> over double for exercising <see cref="ArrayMath"/>.</summary>
internal sealed class DoubleField : IField<double>
{
    public static readonly DoubleField Instance = new();
    private DoubleField() { }

    public double Zero => 0;
    public double One => 1;
    public double FromLong(long value) => value;
    public double Add(double a, double b) => a + b;
    public double Subtract(double a, double b) => a - b;
    public double Multiply(double a, double b) => a * b;
    public double Divide(double a, double b) => a / b;
    public double Negate(double a) => -a;
    public bool IsZero(double a) => a == 0;
    public int Compare(double a, double b) => a.CompareTo(b);
    public double Sqrt(double a) => Math.Sqrt(a);
}

public class NdArrayTests
{
    private static NdArray<double> Arr(params long[] shape) => NdArray<double>.Fill(shape, 0);
    private static NdArray<double> Arr(IReadOnlyList<long> shape, params double[] data) => new(shape, data);

    [Fact]
    public void GivenShapeData_ExposesShapeRankNumelStrides()
    {
        var a = Arr(new long[] { 2, 2, 2 }, 1, 2, 3, 4, 5, 6, 7, 8);
        Assert.Equal(new long[] { 2, 2, 2 }, a.Shape);
        Assert.Equal(3, a.Rank);
        Assert.Equal(8, a.Numel);
        Assert.Equal(new long[] { 8, 4, 2, 1 }, a.Strides);
    }

    [Fact]
    public void GivenMismatchedData_Throws()
    {
        Assert.Throws<ArgumentException>(() => Arr(new long[] { 2, 2 }, 1, 2, 3));
    }

    [Fact]
    public void Get_FullIndex_ReturnsElement()
    {
        var a = Arr(new long[] { 2, 2 }, 1, 2, 3, 4);
        Assert.Equal(4, a.Get(new long[] { 1, 1 }));
        Assert.Equal(2, a.Get(new long[] { 0, 1 }));
    }

    [Fact]
    public void Get_OutOfRange_Throws()
    {
        var a = Arr(new long[] { 2, 2 }, 1, 2, 3, 4);
        Assert.Throws<InvalidOperationException>(() => a.Get(new long[] { 2, 0 }));
    }

    [Fact]
    public void Slice_PartialIndex_ReturnsLowerRank()
    {
        var a = Arr(new long[] { 2, 2, 2 }, 1, 2, 3, 4, 5, 6, 7, 8);
        var sub = a.Slice(new long[] { 1 });
        Assert.Equal(new long[] { 2, 2 }, sub.Shape);
        Assert.Equal(new[] { 5d, 6, 7, 8 }, sub.Data);
    }

    [Fact]
    public void Reshape_MatchingNumel_ReturnsNewShape()
    {
        var a = Arr(new long[] { 2, 3 }, 1, 2, 3, 4, 5, 6);
        var r = a.Reshape(new long[] { 3, 2 });
        Assert.Equal(new long[] { 3, 2 }, r.Shape);
        Assert.Equal(a.Data, r.Data);
    }

    [Fact]
    public void Reshape_Mismatch_Throws()
    {
        var a = Arr(new long[] { 2, 3 }, 1, 2, 3, 4, 5, 6);
        Assert.Throws<ArgumentException>(() => a.Reshape(new long[] { 2, 2 }));
    }

    [Fact]
    public void Transpose_ReverseAxes()
    {
        var a = Arr(new long[] { 2, 3 }, 1, 2, 3, 4, 5, 6);
        var t = a.Transpose();
        Assert.Equal(new long[] { 3, 2 }, t.Shape);
        Assert.Equal(new[] { 1d, 4, 2, 5, 3, 6 }, t.Data);
    }

    [Fact]
    public void Transpose_ExplicitPermutation()
    {
        var a = Arr(new long[] { 2, 2, 2 }, 1, 2, 3, 4, 5, 6, 7, 8);
        var t = a.Transpose(new long[] { 2, 0, 1 });
        Assert.Equal(new long[] { 2, 2, 2 }, t.Shape);
        Assert.Equal(new[] { 1d, 3, 5, 7, 2, 4, 6, 8 }, t.Data);
    }

    [Fact]
    public void Squeeze_RemovesSingletonDims()
    {
        var a = Arr(new long[] { 1, 2, 1, 3 }, 1, 2, 3, 4, 5, 6);
        var s = a.Squeeze();
        Assert.Equal(new long[] { 2, 3 }, s.Shape);
    }

    [Fact]
    public void Concat_AlongAxis()
    {
        var a = Arr(new long[] { 2, 2 }, 1, 2, 3, 4);
        var b = Arr(new long[] { 2, 2 }, 5, 6, 7, 8);
        var c = NdArray<double>.Concat(a, b, 0);
        Assert.Equal(new long[] { 4, 2 }, c.Shape);
        Assert.Equal(new[] { 1d, 2, 3, 4, 5, 6, 7, 8 }, c.Data);
    }

    // ------------------------------------------------------------------
    // ArrayMath (over DoubleField)
    // ------------------------------------------------------------------

    [Fact]
    public void Sum_AllAndAxis()
    {
        var m = Arr(new long[] { 2, 2 }, 1, 2, 3, 4);
        Assert.Equal(10, ArrayMath.Sum(DoubleField.Instance, m));
        Assert.Equal(new[] { 4d, 6 }, ArrayMath.Sum(DoubleField.Instance, m, 0).Data);
        Assert.Equal(new[] { 3d, 7 }, ArrayMath.Sum(DoubleField.Instance, m, 1).Data);
    }

    [Fact]
    public void MinMax()
    {
        var v = Arr(new long[] { 4 }, 3, 1, 4, 1);
        Assert.Equal(1, ArrayMath.Min(DoubleField.Instance, v));
        Assert.Equal(4, ArrayMath.Max(DoubleField.Instance, v));
    }

    [Fact]
    public void MeanAndNorm()
    {
        var v = Arr(new long[] { 3 }, 1, 2, 3);
        Assert.Equal(2, ArrayMath.Mean(DoubleField.Instance, v));
        var u = Arr(new long[] { 2 }, 3, 4);
        Assert.Equal(5, ArrayMath.Norm(DoubleField.Instance, u));
    }

    [Fact]
    public void DotAndCross()
    {
        var a = Arr(new long[] { 3 }, 1, 2, 3);
        var b = Arr(new long[] { 3 }, 4, 5, 6);
        Assert.Equal(32, ArrayMath.Dot(DoubleField.Instance, a, b));

        var x = Arr(new long[] { 3 }, 1, 0, 0);
        var y = Arr(new long[] { 3 }, 0, 1, 0);
        Assert.Equal(new[] { 0d, 0, 1 }, ArrayMath.Cross(DoubleField.Instance, x, y).Data);
    }

    [Fact]
    public void MatMul_2x2()
    {
        var a = Arr(new long[] { 2, 2 }, 1, 2, 3, 4);
        var b = Arr(new long[] { 2, 2 }, 5, 6, 7, 8);
        var r = ArrayMath.MatMul(DoubleField.Instance, a, b);
        Assert.Equal(new long[] { 2, 2 }, r.Shape);
        Assert.Equal(new[] { 19d, 22, 43, 50 }, r.Data);
    }

    [Fact]
    public void MatMul_Batched()
    {
        var a = Arr(new long[] { 2, 2, 2 }, 1, 2, 3, 4, 5, 6, 7, 8);
        var r = ArrayMath.MatMul(DoubleField.Instance, a, a);
        Assert.Equal(new long[] { 2, 2, 2 }, r.Shape);
        Assert.Equal(new[] { 7d, 10, 15, 22, 67, 78, 91, 106 }, r.Data);
    }

    [Fact]
    public void Det_Inverse_Trace()
    {
        var m = Arr(new long[] { 2, 2 }, 1, 2, 3, 4);
        Assert.Equal(-2, ArrayMath.Det(DoubleField.Instance, m));
        Assert.Equal(5, ArrayMath.Trace(DoubleField.Instance, m));

        var inv = ArrayMath.Inverse(DoubleField.Instance, m);
        Assert.Equal(new long[] { 2, 2 }, inv.Shape);
        Assert.Equal(new[] { -2d, 1, 1.5, -0.5 }, inv.Data);
    }

    [Fact]
    public void Inverse_Singular_Throws()
    {
        var m = Arr(new long[] { 2, 2 }, 1, 2, 2, 4);
        Assert.Throws<InvalidOperationException>(() => ArrayMath.Inverse(DoubleField.Instance, m));
    }
}
