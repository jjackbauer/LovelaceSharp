using Lovelace.Suite;

namespace Lovelace.Suite.Tests;

/// <summary>Language-level tests for N-dimensional array values, indexing, and built-ins.</summary>
public class ArrayTests
{
    private static async Task<Value> Eval(string source) => await new SuiteEngine().EvaluateAsync(source);

    private static async Task<string> Typed(string source) =>
        ValueFormatter.FormatTyped(await new SuiteEngine().EvaluateAsync(source));

    // ------------------------------------------------------------------
    // Literals
    // ------------------------------------------------------------------

    [Fact]
    public async Task GivenMatrixLiteral_ProducesRank2Array()
    {
        var result = await Eval("[[1, 2], [3, 4]]");
        Assert.Equal(ValueKind.Array, result.Kind);
        Assert.Equal(new long[] { 2, 2 }, result.AsArray().Shape);
        Assert.Equal("[[1, 2], [3, 4]] (Array)", ValueFormatter.FormatTyped(result));
    }

    [Fact]
    public async Task GivenRank3Literal_ProducesRank3Array()
    {
        var result = await Eval("[[[1, 2], [3, 4]], [[5, 6], [7, 8]]]");
        Assert.Equal(ValueKind.Array, result.Kind);
        Assert.Equal(3, result.AsArray().Rank);
        Assert.Equal(new long[] { 2, 2, 2 }, result.AsArray().Shape);
    }

    [Fact]
    public async Task GivenFlatList_StillProducesVector()
    {
        var result = await Eval("[1, 2, 3]");
        Assert.Equal(ValueKind.Vector, result.Kind);
    }

    [Fact]
    public async Task GivenRaggedNestedList_ReportsError()
    {
        await Assert.ThrowsAsync<InvalidOperationException>(() => Eval("[[1, 2], [3]]"));
    }

    // ------------------------------------------------------------------
    // Indexing
    // ------------------------------------------------------------------

    [Fact]
    public async Task GivenPartialIndex_ReturnsLowerRank()
    {
        var result = await Eval("[[[1, 2], [3, 4]], [[5, 6], [7, 8]]][0]");
        Assert.Equal(ValueKind.Array, result.Kind);
        Assert.Equal(new long[] { 2, 2 }, result.AsArray().Shape);
    }

    [Fact]
    public async Task GivenFullIndex_ReturnsElement()
    {
        var result = await Eval("[[[1, 2], [3, 4]], [[5, 6], [7, 8]]][1, 0, 1]");
        Assert.Equal(ValueKind.Natural, result.Kind);
        Assert.Equal("6", result.AsNatural().ToString());
    }

    [Fact]
    public async Task GivenIndexOutOfRange_ReportsError()
    {
        await Assert.ThrowsAsync<InvalidOperationException>(() => Eval("[[1, 2]][0, 2]"));
    }

    // ------------------------------------------------------------------
    // Construction / introspection / manipulation
    // ------------------------------------------------------------------

    [Fact]
    public async Task Zeros_ProducesRequestedShape() =>
        Assert.Equal("[[0, 0, 0], [0, 0, 0]] (Array)", await Typed("zeros(2, 3)"));

    [Fact]
    public async Task Ones_SingleDimension_IsVector() =>
        Assert.Equal("[1, 1, 1] (Vector)", await Typed("ones(3)"));

    [Fact]
    public async Task Eye_ProducesIdentity() =>
        Assert.Equal("[[1, 0], [0, 1]] (Array)", await Typed("eye(2)"));

    [Fact]
    public async Task Reshape_ProducesNewShape() =>
        Assert.Equal("[[1, 2, 3], [4, 5, 6]] (Array)", await Typed("reshape(1..6, 2, 3)"));

    [Fact]
    public async Task ShapeRankNumel_ReportMetadata()
    {
        Assert.Equal("[2, 3] (Vector)", await Typed("shape(zeros(2, 3))"));
        Assert.Equal("2 (Natural)", await Typed("rank(zeros(2, 3))"));
        Assert.Equal("6 (Natural)", await Typed("numel(zeros(2, 3))"));
    }

    [Fact]
    public async Task Flatten_ReturnsVector() =>
        Assert.Equal("[1, 2, 3, 4] (Vector)", await Typed("flatten([[1, 2], [3, 4]])"));

    [Fact]
    public async Task Transpose_ReturnsTranspose() =>
        Assert.Equal("[[1, 3], [2, 4]] (Array)", await Typed("transpose([[1, 2], [3, 4]])"));

    [Fact]
    public async Task Squeeze_RemovesSingletonDims() =>
        Assert.Equal("[1, 2] (Vector)", await Typed("squeeze([[[1, 2]]])"));

    // ------------------------------------------------------------------
    // Reductions
    // ------------------------------------------------------------------

    [Fact]
    public async Task Sum_AllAndAxis()
    {
        Assert.Equal("10 (Natural)", await Typed("sum([[1, 2], [3, 4]])"));
        Assert.Equal("[4, 6] (Vector)", await Typed("sum([[1, 2], [3, 4]], 0)"));
        Assert.Equal("[3, 7] (Vector)", await Typed("sum([[1, 2], [3, 4]], 1)"));
    }

    [Fact]
    public async Task Mean_Exact()
    {
        Assert.Equal("2 (Natural)", await Typed("mean([1, 2, 3])"));
        Assert.Equal("1.5 (Real)", await Typed("mean([1, 2])"));
    }

    [Fact]
    public async Task Norm_Euclidean() =>
        Assert.Equal("5 (Real)", await Typed("norm([3, 4])"));

    // ------------------------------------------------------------------
    // Linear algebra
    // ------------------------------------------------------------------

    [Fact]
    public async Task MatMul_2x2() =>
        Assert.Equal("[[19, 22], [43, 50]] (Array)",
            await Typed("matmul([[1, 2], [3, 4]], [[5, 6], [7, 8]])"));

    [Fact]
    public async Task Dot_InnerProduct() =>
        Assert.Equal("11 (Natural)", await Typed("dot([1, 2], [3, 4])"));

    [Fact]
    public async Task Cross_3D() =>
        Assert.Equal("[0, 0, 1] (Vector)", await Typed("cross([1, 0, 0], [0, 1, 0])"));

    [Fact]
    public async Task Det_ReturnsDeterminant() =>
        Assert.Equal("-2 (Integer)", await Typed("det([[1, 2], [3, 4]])"));

    [Fact]
    public async Task Inv_ReturnsInverse() =>
        Assert.Equal("[[-2, 1], [1.5, -0.5]] (Array)", await Typed("inv([[1, 2], [3, 4]])"));

    [Fact]
    public async Task Inv_Singular_ReportsError() =>
        await Assert.ThrowsAsync<InvalidOperationException>(() => Eval("inv([[1, 2], [2, 4]])"));

    [Fact]
    public async Task Trace_ReturnsDiagonalSum() =>
        Assert.Equal("5 (Natural)", await Typed("trace([[1, 2], [3, 4]])"));

    // ------------------------------------------------------------------
    // Concatenation
    // ------------------------------------------------------------------

    [Fact]
    public async Task ConcatAndAppend_Concatenate()
    {
        Assert.Equal("[1, 2, 3, 4] (Vector)", await Typed("concat([1, 2], [3, 4])"));
        Assert.Equal("[1, 2, 3, 4] (Vector)", await Typed("append([1, 2], [3, 4])"));
    }

    // ------------------------------------------------------------------
    // Exposure
    // ------------------------------------------------------------------

    [Fact]
    public async Task FreshEngine_IncludesArrayBuiltins()
    {
        var engine = new SuiteEngine();
        await engine.EvaluateAsync("1"); // force builtin registration via evaluate (already done in ctor)
        foreach (var name in new[] { "sum", "prod", "min", "max", "mean", "norm", "dot", "cross", "matmul", "det", "inv", "trace", "zeros", "ones", "eye", "reshape", "shape", "rank", "numel", "flatten", "transpose", "squeeze", "concat", "append" })
            Assert.True(engine.Functions.ContainsKey(name), $"missing builtin: {name}");
    }

    [Fact]
    public async Task CaptureState_ReportsArrayKind()
    {
        var engine = new SuiteEngine();
        await engine.EvaluateAsync("m = [[1, 2], [3, 4]]");
        var snapshot = engine.CaptureState();
        Assert.Equal(ValueKind.Array, snapshot.Variables["m"].Kind);
    }
}
