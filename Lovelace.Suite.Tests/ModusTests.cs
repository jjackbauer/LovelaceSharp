using Lovelace.Abstractions;
using Lovelace.Suite;
using Lovelace.Statistics;

namespace Lovelace.Suite.Tests;

/// <summary>Pins the Stage-5 Modus plugin contract (MOD-001..006, KRN-003).</summary>
public class ModusTests
{
    [Fact]
    public void PluginLoad_RegistersKernel_AndDispatchRuns()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new StatisticsPlugin());

        var left = new double[] { 1, 2, 3 };
        var right = new double[] { 4, 5, 6 };
        var result = new double[3];

        bool handled = engine.TryDispatchKernel(ArrayOp.Add, left.AsSpan(), right.AsSpan(), result.AsSpan());

        Assert.True(handled);
        Assert.Equal(new double[] { 5, 7, 9 }, result);
    }

    [Fact]
    public void Dispatch_UnsupportedElementType_FallsBack()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new StatisticsPlugin());

        var left = new int[] { 1, 2, 3 };
        var right = new int[] { 4, 5, 6 };
        var result = new int[3];

        bool handled = engine.TryDispatchKernel(ArrayOp.Add, left.AsSpan(), right.AsSpan(), result.AsSpan());

        Assert.False(handled);
    }

    [Fact]
    public void Kernel_DeclinesUnsupportedOp()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new StatisticsPlugin());

        var left = new double[] { 1, 2, 3 };
        var right = new double[] { 4, 5, 6 };
        var result = new double[3];

        // The double kernel only handles Add; a Multiply request must fall back.
        bool handled = engine.TryDispatchKernel(ArrayOp.Multiply, left.AsSpan(), right.AsSpan(), result.AsSpan());

        Assert.False(handled);
    }
}
