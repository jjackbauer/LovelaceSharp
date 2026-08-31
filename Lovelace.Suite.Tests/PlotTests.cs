using Lovelace.Suite;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Suite.Tests;

public class PlotTests
{
    [Fact]
    public async Task Plot_GivenSingleVector_WritesSvgFile()
    {
        var engine = new SuiteEngine();
        string dir = Path.Combine(Path.GetTempPath(), "lovelace-plot-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(dir);

        try
        {
            engine.PlotOutputDirectory = dir;
            engine.PlotFileName = "test.svg";

            var result = await engine.EvaluateAsync("plot([4, 9, 16])");

            Assert.Equal(ValueKind.Text, result.Kind);
            string file = result.AsText();
            Assert.True(File.Exists(file));

            string svg = File.ReadAllText(file);
            Assert.StartsWith("<svg", svg);
            Assert.Contains("<polyline", svg);
        }
        finally
        {
            Directory.Delete(dir, recursive: true);
        }
    }

    [Fact]
    public async Task Plot_GivenMismatchedVectorLengths_Throws()
    {
        var engine = new SuiteEngine();

        await Assert.ThrowsAsync<InvalidOperationException>(() => engine.EvaluateAsync("plot(1..3, [1, 2])"));
    }

    [Fact]
    public async Task Plot_GivenEmptyVector_Throws()
    {
        var engine = new SuiteEngine();

        await Assert.ThrowsAsync<InvalidOperationException>(() => engine.EvaluateAsync("plot([])"));
    }

    [Fact]
    public async Task Plot_GivenTitle_IncludesTitleInSvg()
    {
        var engine = new SuiteEngine();
        string dir = Path.Combine(Path.GetTempPath(), "lovelace-plot-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(dir);

        try
        {
            engine.PlotOutputDirectory = dir;
            engine.PlotFileName = "titled.svg";

            var result = await engine.EvaluateAsync("plot(1..3, [1, 4, 9], \"My Plot\")");

            string svg = File.ReadAllText(result.AsText());
            Assert.Contains("My Plot", svg);
        }
        finally
        {
            Directory.Delete(dir, recursive: true);
        }
    }

    [Fact]
    public void SvgPlotRenderer_GivenFixedModel_ProducesDeterministicOutput()
    {
        var model = new PlotModel { Title = "T" };
        var series = new PlotSeries();
        series.Points.Add(new PlotPoint(1, 2));
        series.Points.Add(new PlotPoint(2, 4));
        series.Points.Add(new PlotPoint(3, 9));
        model.Series.Add(series);

        var renderer = new SvgPlotRenderer();

        Assert.Equal(renderer.Render(model), renderer.Render(model));
    }

    [Fact]
    public void PlotValue_GivenPeriodicReal_ExpandsToDouble()
    {
        var value = new Value(Rl.Parse("0.(3)", null));

        double d = PlotValue.ToDouble(value);

        Assert.Equal(1.0 / 3.0, d, 3);
    }
}
