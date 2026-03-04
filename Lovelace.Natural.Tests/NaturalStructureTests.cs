using System.Numerics;
using Lovelace.Natural;

namespace Lovelace.Natural.Tests;

/// <summary>
/// Tests that verify the <see cref="Natural"/> class is correctly named and
/// implements all required <see cref="System.Numerics"/> interfaces.
/// These are scaffolding-level tests for checklist item
/// "Rename class Class1 → Natural; declare all interfaces on the type declaration".
/// </summary>
public class NaturalStructureTests
{
    [Fact]
    public void Natural_TypeName_IsNatural()
    {
        Assert.Equal("Natural", typeof(Natural).Name);
    }

    [Fact]
    public void Natural_ImplementsIEquatable()
    {
        Assert.True(typeof(IEquatable<Natural>).IsAssignableFrom(typeof(Natural)));
    }

    [Fact]
    public void Natural_ImplementsIComparable()
    {
        Assert.True(typeof(IComparable<Natural>).IsAssignableFrom(typeof(Natural)));
    }

    [Fact]
    public void Natural_ImplementsIParsable()
    {
        Assert.True(typeof(IParsable<Natural>).IsAssignableFrom(typeof(Natural)));
    }

    [Fact]
    public void Natural_ImplementsISpanParsable()
    {
        Assert.True(typeof(ISpanParsable<Natural>).IsAssignableFrom(typeof(Natural)));
    }

    [Fact]
    public void Natural_ImplementsISpanFormattable()
    {
        Assert.True(typeof(ISpanFormattable).IsAssignableFrom(typeof(Natural)));
    }

    [Fact]
    public void Natural_ImplementsINumber()
    {
        Assert.True(typeof(INumber<Natural>).IsAssignableFrom(typeof(Natural)));
    }

    [Fact]
    public void Natural_ImplementsIAdditionOperators()
    {
        Assert.True(typeof(IAdditionOperators<Natural, Natural, Natural>).IsAssignableFrom(typeof(Natural)));
    }

    [Fact]
    public void Natural_ImplementsISubtractionOperators()
    {
        Assert.True(typeof(ISubtractionOperators<Natural, Natural, Natural>).IsAssignableFrom(typeof(Natural)));
    }

    [Fact]
    public void Natural_ImplementsIMultiplyOperators()
    {
        Assert.True(typeof(IMultiplyOperators<Natural, Natural, Natural>).IsAssignableFrom(typeof(Natural)));
    }

    [Fact]
    public void Natural_ImplementsIDivisionOperators()
    {
        Assert.True(typeof(IDivisionOperators<Natural, Natural, Natural>).IsAssignableFrom(typeof(Natural)));
    }

    [Fact]
    public void Natural_ImplementsIModulusOperators()
    {
        Assert.True(typeof(IModulusOperators<Natural, Natural, Natural>).IsAssignableFrom(typeof(Natural)));
    }

    [Fact]
    public void Natural_ImplementsIIncrementOperators()
    {
        Assert.True(typeof(IIncrementOperators<Natural>).IsAssignableFrom(typeof(Natural)));
    }

    [Fact]
    public void Natural_ImplementsIDecrementOperators()
    {
        Assert.True(typeof(IDecrementOperators<Natural>).IsAssignableFrom(typeof(Natural)));
    }

    [Fact]
    public void Natural_ImplementsIComparisonOperators()
    {
        Assert.True(typeof(IComparisonOperators<Natural, Natural, bool>).IsAssignableFrom(typeof(Natural)));
    }
}
