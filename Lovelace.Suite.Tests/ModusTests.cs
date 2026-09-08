using Lovelace.Abstractions;
using Lovelace.Dsp;
using Lovelace.Suite;
using Lovelace.Statistics;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Suite.Tests;

/// <summary>Pins the Stage-5 Modus plugin contract (MOD-001..006, KRN-003) over exact types.</summary>
public class ModusTests
{
    [Fact]
    public void PluginLoad_RegistersKernel_AndDispatchRuns()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new StatisticsPlugin());

        var left = new Rl[] { new("1"), new("2"), new("3") };
        var right = new Rl[] { new("4"), new("5"), new("6") };
        var result = new Rl[3];

        bool handled = engine.TryDispatchKernel(ArrayOp.Add, left.AsSpan(), right.AsSpan(), result.AsSpan());

        Assert.True(handled);
        Assert.Equal(new Rl("5"), result[0]);
        Assert.Equal(new Rl("7"), result[1]);
        Assert.Equal(new Rl("9"), result[2]);
    }

    [Fact]
    public void RealAddKernel_GivenRealOperands_MatchesDirectRealAddition()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new StatisticsPlugin());

        var a = new Rl("1.5");
        var b = new Rl("2.25");
        var left = new[] { a };
        var right = new[] { b };
        var result = new Rl[1];

        bool handled = engine.TryDispatchKernel(ArrayOp.Add, left.AsSpan(), right.AsSpan(), result.AsSpan());

        Assert.True(handled);
        Assert.Equal(a + b, result[0]);
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

        var left = new Rl[] { new("1"), new("2"), new("3") };
        var right = new Rl[] { new("4"), new("5"), new("6") };
        var result = new Rl[3];

        // The Real kernel only handles Add; a Multiply request must fall back.
        bool handled = engine.TryDispatchKernel(ArrayOp.Multiply, left.AsSpan(), right.AsSpan(), result.AsSpan());

        Assert.False(handled);
    }

    [Fact]
    public async Task ScalarResult_GivenBooleanAndText_ReturnValues()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new ScalarResultPlugin());

        Assert.Equal("True (Boolean)", ValueFormatter.FormatTyped(await engine.EvaluateAsync("flag(1)")));
        Assert.Equal("hello", ValueFormatter.FormatTyped(await engine.EvaluateAsync("greet(1)")));
    }

    private sealed class ScalarResultPlugin : IModusPlugin
    {
        public string Name => "ScalarResult";

        public void Register(IModusContext context)
        {
            context.RegisterBuiltin("flag", new[] { "x" }, _ => ScalarResult.FromBoolean(true));
            context.RegisterBuiltin("greet", new[] { "x" }, _ => ScalarResult.FromText("hello"));
        }
    }

    [Fact]
    public async Task DspPlugin_LoadedThroughModusSeam_RegistersDspBuiltins()
    {
        var engine = new SuiteEngine();

        // DSP loads through the same IModusPlugin/IModusContext seam as any extension package.
        engine.LoadPlugin(new DspPlugin());

        Assert.Equal("Lovelace.Dsp", new DspPlugin().Name);
        var result = await engine.EvaluateAsync("fft([1, 0, 0, 0])");
        Assert.Equal("[1, 1, 1, 1] (Vector)", ValueFormatter.FormatTyped(result));
    }
}
