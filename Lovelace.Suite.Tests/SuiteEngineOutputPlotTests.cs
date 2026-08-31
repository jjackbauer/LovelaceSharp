using Lovelace.Suite;

namespace Lovelace.Suite.Tests;

public class SuiteEngineOutputPlotTests
{
    [Fact]
    public async Task EvaluateAsync_GivenOutputWriter_CapturesPrintLinesToThatWriter()
    {
        var engine = new SuiteEngine();
        var original = engine.Output;
        var capture = new StringWriter();

        await engine.EvaluateAsync("print(\"hi\")", capture);

        Assert.Equal("hi", capture.ToString().Trim());
        Assert.Same(original, engine.Output);
    }

    [Fact]
    public async Task EvaluateAsync_GivenPlotCall_SetsLastPlotSvg()
    {
        var engine = new SuiteEngine();
        string dir = Path.Combine(Path.GetTempPath(), "lovelace-plot-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(dir);

        try
        {
            engine.PlotOutputDirectory = dir;
            engine.PlotFileName = "test.svg";

            await engine.EvaluateAsync("plot(1..3, [1, 4, 9])");

            Assert.NotNull(engine.LastPlot);
            Assert.NotNull(engine.LastPlot!.Svg);
            Assert.StartsWith("<svg", engine.LastPlot.Svg);
        }
        finally
        {
            Directory.Delete(dir, recursive: true);
        }
    }

    [Fact]
    public async Task ResetPlotCapture_GivenPreviousPlot_ClearsLastPlot()
    {
        var engine = new SuiteEngine();
        string dir = Path.Combine(Path.GetTempPath(), "lovelace-plot-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(dir);

        try
        {
            engine.PlotOutputDirectory = dir;
            engine.PlotFileName = "test.svg";

            await engine.EvaluateAsync("plot([1, 2, 3])");
            Assert.NotNull(engine.LastPlot);

            engine.ResetPlotCapture();

            Assert.Null(engine.LastPlot);
        }
        finally
        {
            Directory.Delete(dir, recursive: true);
        }
    }
}
