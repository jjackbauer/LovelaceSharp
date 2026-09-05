using Lovelace.Abstractions;
using Lovelace.Complex;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Complex.Tests;

/// <summary>
/// Pins the <see cref="DType.Complex"/> descriptor and verifies a homogeneous
/// <see cref="DenseArray{Complex}"/> carries complex elements with correct metadata. This is the
/// typed-array form (the kernel/`IArrayKernel&lt;T&gt;` surface); the language layer's DSP builtins
/// instead return a boxed <c>DenseArray&lt;Value&gt;</c> whose <see cref="DType"/> is inferred as
/// <see cref="DType.Complex"/> (see <c>Lovelace.Suite.Tests.DspBuiltinsTests</c>).
/// </summary>
public class DenseArrayComplexTests
{
    private static readonly Complex s_a = new(new Rl("1"), new Rl("2"));
    private static readonly Complex s_b = new(new Rl("3"), new Rl("-4"));

    [Fact]
    public void DenseArrayOfComplex_GivenConstruction_ReportsComplexDTypeAndElementType()
    {
        var array = new DenseArray<Complex>(new long[] { 2 }, new[] { s_a, s_b }, DType.Complex, new Precision(16));

        Assert.Equal(DType.Complex, array.DType);
        Assert.Equal(typeof(Complex), array.ElementType);
        Assert.Equal(2, array.Numel);
        Assert.True(array.IsContiguous);
    }

    [Fact]
    public void DenseArrayOfComplex_GivenElements_GetElementRoundTrips()
    {
        var array = new DenseArray<Complex>(new long[] { 2 }, new[] { s_a, s_b }, DType.Complex, new Precision(16));

        Assert.Equal(s_a, (Complex)array.GetElement(0));
        Assert.Equal(s_b, (Complex)array.GetElement(1));
    }

    [Fact]
    public void DenseArrayOfComplex_GivenContiguous_AsSpanIterates()
    {
        var array = new DenseArray<Complex>(new long[] { 2 }, new[] { s_a, s_b }, DType.Complex, new Precision(16));

        var span = array.AsSpan();
        Assert.Equal(s_a, span[0]);
        Assert.Equal(s_b, span[1]);
    }
}
