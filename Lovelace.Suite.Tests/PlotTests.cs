using Lovelace.Suite;
using Nat = global::Lovelace.Natural.Natural;
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

    [Fact]
    public void PlotValue_GivenHugeNatural_ToRealPreservesDigitsExactly()
    {
        string digits = "1" + new string('0', 400);
        var value = new Value(Nat.Parse(digits, null));

        Rl real = PlotValue.ToReal(value);

        Assert.Equal(digits, real.ToString());
    }

    [Fact]
    public void SvgPlotRenderer_GivenHugeCloseXValues_KeepsPointsDistinct()
    {
        var model = new PlotModel();
        var series = new PlotSeries();
        series.Points.Add(new PlotPoint(new Rl("100000000000000000000"), new Rl("0")));
        series.Points.Add(new PlotPoint(new Rl("100000000000000000001"), new Rl("1")));
        model.Series.Add(series);

        string svg = new SvgPlotRenderer().Render(model);

        Assert.DoesNotContain("Infinity", svg);
        Assert.DoesNotContain("NaN", svg);

        int idx = svg.IndexOf("points=\"", StringComparison.Ordinal);
        Assert.True(idx >= 0);
        string tail = svg[(idx + "points=\"".Length)..];
        string pts = tail[..tail.IndexOf('"')];
        string[] coords = pts.Split(' ');
        Assert.Equal(2, coords.Length);

        double x1 = double.Parse(coords[0].Split(',')[0], System.Globalization.CultureInfo.InvariantCulture);
        double x2 = double.Parse(coords[1].Split(',')[0], System.Globalization.CultureInfo.InvariantCulture);
        Assert.NotEqual(x1, x2);
    }

    [Fact]
    public void SvgPlotRenderer_GivenThreePoints_UsesDenseSplinePolyline()
    {
        var model = new PlotModel();
        var series = new PlotSeries();
        series.Points.Add(new PlotPoint(1, 1));
        series.Points.Add(new PlotPoint(2, 4));
        series.Points.Add(new PlotPoint(3, 9));
        model.Series.Add(series);

        string svg = new SvgPlotRenderer().Render(model);

        // Smooth rendering is a dense polyline (not a <path>), so the curve looks
        // smooth on any renderer without relying on cubic-Bézier flattening.
        Assert.Contains("<polyline", svg);
        Assert.DoesNotContain("<path", svg);
        Assert.True(PolylinePointCount(svg) > 3, "expected the spline to be densely sampled");
    }

    [Fact]
    public void SvgPlotRenderer_GivenTwoPoints_FallsBackToPolyline()
    {
        var model = new PlotModel();
        var series = new PlotSeries();
        series.Points.Add(new PlotPoint(1, 2));
        series.Points.Add(new PlotPoint(2, 4));
        model.Series.Add(series);

        string svg = new SvgPlotRenderer().Render(model);

        Assert.Contains("<polyline", svg);
        Assert.DoesNotContain("<path", svg);
    }

    [Fact]
    public void SvgPlotRenderer_GivenLinearInterpolation_EmitsPolyline()
    {
        var model = new PlotModel();
        var series = new PlotSeries { Interpolation = PlotInterpolation.Linear };
        series.Points.Add(new PlotPoint(1, 1));
        series.Points.Add(new PlotPoint(2, 4));
        series.Points.Add(new PlotPoint(3, 9));
        model.Series.Add(series);

        string svg = new SvgPlotRenderer().Render(model);

        Assert.Contains("<polyline", svg);
        Assert.DoesNotContain("<path", svg);
    }

    [Fact]
    public void SvgPlotRenderer_GivenCubicSpline_PassesThroughDataEndpoints()
    {
        var data = new[] { new PlotPoint(1, 1), new PlotPoint(2, 4), new PlotPoint(3, 9), new PlotPoint(4, 16) };

        var linear = new PlotModel();
        var linearSeries = new PlotSeries { Interpolation = PlotInterpolation.Linear };
        linearSeries.Points.AddRange(data);
        linear.Series.Add(linearSeries);

        var spline = new PlotModel();
        var splineSeries = new PlotSeries { Interpolation = PlotInterpolation.CubicSpline };
        splineSeries.Points.AddRange(data);
        spline.Series.Add(splineSeries);

        string linearSvg = new SvgPlotRenderer().Render(linear);
        string splineSvg = new SvgPlotRenderer().Render(spline);

        Assert.Equal(PolylineEndpoints(linearSvg).First, PolylineEndpoints(splineSvg).First);
        Assert.Equal(PolylineEndpoints(linearSvg).Last, PolylineEndpoints(splineSvg).Last);
    }

    [Fact]
    public void SvgPlotRenderer_GivenCollinearPoints_SplineStaysCollinear()
    {
        var model = new PlotModel();
        var series = new PlotSeries();
        series.Points.Add(new PlotPoint(1, 1));
        series.Points.Add(new PlotPoint(2, 2));
        series.Points.Add(new PlotPoint(3, 3));
        series.Points.Add(new PlotPoint(4, 4));
        model.Series.Add(series);

        string svg = new SvgPlotRenderer().Render(model);

        var coords = PolylineCoordinates(svg);
        Assert.True(coords.Count >= 4, "expected the dense spline to contain many points");

        var (ax, ay) = coords[0];
        var (bx, by) = coords[coords.Count - 1];
        double lineLength = Math.Sqrt(((bx - ax) * (bx - ax)) + ((by - ay) * (by - ay)));
        foreach (var (x, y) in coords)
        {
            // Perpendicular distance from the line through the two endpoints, in pixels.
            double cross = ((bx - ax) * (y - ay)) - ((by - ay) * (x - ax));
            double distance = Math.Abs(cross) / lineLength;
            Assert.True(distance < 0.05, $"point ({x},{y}) deviates {distance:F6} px from the straight line");
        }
    }

    private static int PolylinePointCount(string svg)
    {
        string points = ExtractAttribute(svg, "points");
        return points.Split(' ', StringSplitOptions.RemoveEmptyEntries).Length;
    }

    private static (string First, string Last) PolylineEndpoints(string svg)
    {
        string points = ExtractAttribute(svg, "points");
        string[] coords = points.Split(' ', StringSplitOptions.RemoveEmptyEntries);
        return (coords[0], coords[^1]);
    }

    private static List<(double X, double Y)> PolylineCoordinates(string svg)
    {
        string points = ExtractAttribute(svg, "points");
        var result = new List<(double X, double Y)>();
        foreach (string token in points.Split(' ', StringSplitOptions.RemoveEmptyEntries))
        {
            string[] parts = token.Split(',');
            result.Add((double.Parse(parts[0], System.Globalization.CultureInfo.InvariantCulture),
                        double.Parse(parts[1], System.Globalization.CultureInfo.InvariantCulture)));
        }
        return result;
    }

    private static string ExtractAttribute(string svg, string attribute)
    {
        string needle = attribute + "=\"";
        int start = svg.IndexOf(needle, StringComparison.Ordinal);
        Assert.True(start >= 0, $"attribute '{attribute}' not found in SVG");
        int valueStart = start + needle.Length;
        int end = svg.IndexOf('"', valueStart);
        return svg[valueStart..end];
    }
}
