using Lovelace.Console.Repl;
using Nat = global::Lovelace.Natural.Natural;
using Int = global::Lovelace.Integer.Integer;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Console.Tests;

public class ValueTests
{
    // -----------------------------------------------------------------------
    // Constructor & Kind
    // -----------------------------------------------------------------------

    [Fact]
    public void Value_GivenNatural_StoresNaturalKind()
    {
        var n = Nat.Parse("42", null);
        var v = new Value(n);

        Assert.Equal(ValueKind.Natural, v.Kind);
    }

    [Fact]
    public void Value_GivenInteger_StoresIntegerKind()
    {
        var i = Int.Parse("42", null);
        var v = new Value(i);

        Assert.Equal(ValueKind.Integer, v.Kind);
    }

    [Fact]
    public void Value_GivenReal_StoresRealKind()
    {
        var r = Rl.Parse("3.14", null);
        var v = new Value(r);

        Assert.Equal(ValueKind.Real, v.Kind);
    }

    // -----------------------------------------------------------------------
    // ToString
    // -----------------------------------------------------------------------

    [Fact]
    public void Value_ToString_GivenNatural_PrefixesKindLabel()
    {
        var n = Nat.Parse("42", null);
        var v = new Value(n);

        Assert.StartsWith("Natural:", v.ToString());
        Assert.Contains("42", v.ToString());
    }

    // -----------------------------------------------------------------------
    // Widen
    // -----------------------------------------------------------------------

    [Fact]
    public void Widen_GivenNaturalToInteger_ReturnsIntegerValue()
    {
        var v = new Value(Nat.Parse("5", null));
        var widened = v.Widen(ValueKind.Integer);

        Assert.Equal(ValueKind.Integer, widened.Kind);
        Assert.Equal(Int.Parse("5", null), widened.AsInteger());
    }

    [Fact]
    public void Widen_GivenNaturalToReal_ReturnsRealValue()
    {
        var v = new Value(Nat.Parse("5", null));
        var widened = v.Widen(ValueKind.Real);

        Assert.Equal(ValueKind.Real, widened.Kind);
        Assert.Equal(Rl.Parse("5", null), widened.AsReal());
    }

    [Fact]
    public void Widen_GivenIntegerToReal_ReturnsRealValue()
    {
        var v = new Value(Int.Parse("-3", null));
        var widened = v.Widen(ValueKind.Real);

        Assert.Equal(ValueKind.Real, widened.Kind);
        Assert.Equal(Rl.Parse("-3", null), widened.AsReal());
    }

    [Fact]
    public void Widen_GivenSameKind_ReturnsSameValue()
    {
        var v = new Value(Nat.Parse("7", null));
        var widened = v.Widen(ValueKind.Natural);

        Assert.Same(v, widened);
    }

    [Fact]
    public void Widen_GivenWiderToNarrower_ThrowsInvalidOperationException()
    {
        var v = new Value(Int.Parse("3", null));

        Assert.Throws<InvalidOperationException>(() => v.Widen(ValueKind.Natural));
    }

    // -----------------------------------------------------------------------
    // WidenPair
    // -----------------------------------------------------------------------

    [Fact]
    public void WidenPair_GivenNaturalAndInteger_ReturnsBothAsInteger()
    {
        var a = new Value(Nat.Parse("5", null));
        var b = new Value(Int.Parse("3", null));

        var (wa, wb) = Value.WidenPair(a, b);

        Assert.Equal(ValueKind.Integer, wa.Kind);
        Assert.Equal(ValueKind.Integer, wb.Kind);
    }

    [Fact]
    public void WidenPair_GivenNaturalAndReal_ReturnsBothAsReal()
    {
        var a = new Value(Nat.Parse("5", null));
        var b = new Value(Rl.Parse("3.14", null));

        var (wa, wb) = Value.WidenPair(a, b);

        Assert.Equal(ValueKind.Real, wa.Kind);
        Assert.Equal(ValueKind.Real, wb.Kind);
    }

    [Fact]
    public void WidenPair_GivenSameKind_ReturnsUnchanged()
    {
        var a = new Value(Nat.Parse("5", null));
        var b = new Value(Nat.Parse("3", null));

        var (wa, wb) = Value.WidenPair(a, b);

        Assert.Equal(ValueKind.Natural, wa.Kind);
        Assert.Equal(ValueKind.Natural, wb.Kind);
    }
}
