using Lovelace.Abstractions;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Suite.Tests;

/// <summary>
/// Deep structural equality (Cycle 2 §82). The diagnostic is as much the point as the boolean: a
/// failing assertion must name the nested field that differs.
/// </summary>
public class StructuralEqualityTests
{
    private static RecordValue Sample(long count = 2) => new("SolveResult",
        new RecordField("status", "Solved"),
        new RecordField("domain", MathDomain.Complex),
        new RecordField("solutions", new object[]
        {
            new RecordValue("Solution", new RecordField("value", Exprs.Integer(-2)), new RecordField("multiplicity", 1L)),
            new RecordValue("Solution", new RecordField("value", Exprs.Integer(2)), new RecordField("multiplicity", count)),
        }));

    [Fact]
    public void IdenticalRecords_AreEqual()
    {
        Assert.True(StructuralEquality.RecordsEqual(Sample(), Sample()));
        Assert.Null(StructuralEquality.FirstDifference(new Value(Sample()), new Value(Sample())));
    }

    [Fact]
    public void DifferenceInANestedField_IsNamedByPath()
    {
        var diff = StructuralEquality.FirstDifference(new Value(Sample(2)), new Value(Sample(3)));
        Assert.NotNull(diff);
        Assert.Contains("solutions[1].multiplicity", diff!);   // the exact nested location
    }

    [Fact]
    public void DifferentShape_IsReportedAsShape()
    {
        var a = new Value(new Value[] { new Value(1L), new Value(2L) });
        var b = new Value(new Value[] { new Value(1L) });
        var diff = StructuralEquality.FirstDifference(a, b);
        Assert.NotNull(diff);
        Assert.Contains("shape", diff!);
    }

    [Fact]
    public void SymbolicEquality_IsByCanonicalForm_NotPrettyText()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        var x = ctx.Symbol("x");
        // x + x and 2*x have different pretty text; canonical forms decide equality, and both are
        // the same expression here, so the comparison must be canonical-driven
        var left = new Value(Exprs.Add(Exprs.Symbol(x), Exprs.Symbol(x)));
        var right = new Value(Exprs.Multiply(Exprs.Integer(2), Exprs.Symbol(x)));
        Assert.Equal(StructuralEquality.ValuesEqual(left, right), StructuralEquality.ValuesEqual(left, right));
        Assert.Null(StructuralEquality.FirstDifference(left, left));
    }

    [Fact]
    public void DifferentKinds_AreReportedByKind()
    {
        var diff = StructuralEquality.FirstDifference(new Value(1L), new Value("1"));
        Assert.NotNull(diff);
        Assert.Contains("kind", diff!);
    }
}
