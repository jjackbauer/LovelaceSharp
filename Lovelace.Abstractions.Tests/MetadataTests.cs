using Lovelace.Abstractions;

namespace Lovelace.Abstractions.Tests;

public class MetadataTests
{
    [Fact]
    public void DType_WideningLattice_IsOrdered()
    {
        Assert.True((int)DType.Natural < (int)DType.Integer);
        Assert.True((int)DType.Integer < (int)DType.Real);
    }

    [Fact]
    public void Precision_ComparesByValue()
    {
        Assert.Equal(new Precision(16), new Precision(16));
        Assert.NotEqual(new Precision(16), new Precision(38));
        Assert.True(new Precision(16).SignificantDigits < new Precision(38).SignificantDigits);
    }
}
