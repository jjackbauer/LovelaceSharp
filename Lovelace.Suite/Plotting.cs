using System.Globalization;
using System.Text;
using Nat = global::Lovelace.Natural.Natural;
using Int = global::Lovelace.Integer.Integer;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Suite;

// -------------------------------------------------------------------------
// Plot model (renderer-agnostic)
// -------------------------------------------------------------------------

/// <summary>A single (x, y) point, already converted to floating point for rendering.</summary>
public readonly record struct PlotPoint(double X, double Y);

/// <summary>One plotted series.</summary>
public sealed class PlotSeries
{
    public List<PlotPoint> Points { get; } = new();
}

/// <summary>A renderer-agnostic 2D line plot.</summary>
public sealed class PlotModel
{
    public string? Title { get; set; }
    public List<PlotSeries> Series { get; } = new();
}

// -------------------------------------------------------------------------
// Renderers
// -------------------------------------------------------------------------

/// <summary>
/// Captures the SVG and title of the most recently rendered plot, so a host can
/// return the plot inline without reading the plot file.
/// </summary>
public sealed record PlotCapture(string? Svg, string? Title);

/// <summary>Renders a <see cref="PlotModel"/> to a string.</summary>
public interface IPlotRenderer
{
    string Render(PlotModel model);
}

/// <summary>
/// Renders a <see cref="PlotModel"/> to a self-contained SVG document. Output is
/// deterministic: no timestamps, random ids, or culture-dependent formatting.
/// </summary>
public sealed class SvgPlotRenderer : IPlotRenderer
{
    private const int Width = 800;
    private const int Height = 600;
    private const double MarginLeft = 70;
    private const double MarginRight = 30;
    private const double MarginTop = 50;
    private const double MarginBottom = 55;

    public string Render(PlotModel model)
    {
        double plotLeft = MarginLeft;
        double plotRight = Width - MarginRight;
        double plotTop = MarginTop;
        double plotBottom = Height - MarginBottom;

        var (minX, maxX, minY, maxY) = ComputeBounds(model);
        (minX, maxX) = PadBounds(minX, maxX);
        (minY, maxY) = PadBounds(minY, maxY);

        var sb = new StringBuilder();
        sb.Append("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"")
          .Append(Width).Append("\" height=\"").Append(Height)
          .Append("\" viewBox=\"0 0 ").Append(Width).Append(' ').Append(Height).Append("\">\n");
        sb.Append("  <rect width=\"").Append(Width).Append("\" height=\"").Append(Height).Append("\" fill=\"white\"/>\n");

        if (!string.IsNullOrEmpty(model.Title))
        {
            sb.Append("  <text x=\"").Append(Width / 2).Append("\" y=\"28\" text-anchor=\"middle\" font-family=\"sans-serif\" font-size=\"20\">")
              .Append(Escape(model.Title!)).Append("</text>\n");
        }

        // Border of the plot area
        sb.Append("  <rect x=\"").Append(Fmt(plotLeft)).Append("\" y=\"").Append(Fmt(plotTop))
          .Append("\" width=\"").Append(Fmt(plotRight - plotLeft))
          .Append("\" height=\"").Append(Fmt(plotBottom - plotTop))
          .Append("\" fill=\"none\" stroke=\"#cccccc\"/>\n");

        // Axes (at zero when the range crosses zero, otherwise along the plot border)
        double yAxisX = minX <= 0 && maxX >= 0 ? MapX(0, minX, maxX, plotLeft, plotRight) : plotLeft;
        double xAxisY = minY <= 0 && maxY >= 0 ? MapY(0, minY, maxY, plotTop, plotBottom) : plotBottom;

        sb.Append("  <line x1=\"").Append(Fmt(yAxisX)).Append("\" y1=\"").Append(Fmt(plotTop))
          .Append("\" x2=\"").Append(Fmt(yAxisX)).Append("\" y2=\"").Append(Fmt(plotBottom))
          .Append("\" stroke=\"#333333\"/>\n");
        sb.Append("  <line x1=\"").Append(Fmt(plotLeft)).Append("\" y1=\"").Append(Fmt(xAxisY))
          .Append("\" x2=\"").Append(Fmt(plotRight)).Append("\" y2=\"").Append(Fmt(xAxisY))
          .Append("\" stroke=\"#333333\"/>\n");

        // X ticks
        foreach (double tick in NiceTicks(minX, maxX, 6))
        {
            double tx = MapX(tick, minX, maxX, plotLeft, plotRight);
            sb.Append("  <line x1=\"").Append(Fmt(tx)).Append("\" y1=\"").Append(Fmt(xAxisY - 4))
              .Append("\" x2=\"").Append(Fmt(tx)).Append("\" y2=\"").Append(Fmt(xAxisY + 4))
              .Append("\" stroke=\"#333333\"/>\n");
            sb.Append("  <text x=\"").Append(Fmt(tx)).Append("\" y=\"").Append(Fmt(xAxisY + 18))
              .Append("\" text-anchor=\"middle\" font-family=\"sans-serif\" font-size=\"12\">")
              .Append(FormatTick(tick)).Append("</text>\n");
        }

        // Y ticks
        foreach (double tick in NiceTicks(minY, maxY, 6))
        {
            double ty = MapY(tick, minY, maxY, plotTop, plotBottom);
            sb.Append("  <line x1=\"").Append(Fmt(yAxisX - 4)).Append("\" y1=\"").Append(Fmt(ty))
              .Append("\" x2=\"").Append(Fmt(yAxisX + 4)).Append("\" y2=\"").Append(Fmt(ty))
              .Append("\" stroke=\"#333333\"/>\n");
            sb.Append("  <text x=\"").Append(Fmt(yAxisX - 8)).Append("\" y=\"").Append(Fmt(ty + 4))
              .Append("\" text-anchor=\"end\" font-family=\"sans-serif\" font-size=\"12\">")
              .Append(FormatTick(tick)).Append("</text>\n");
        }

        // Series polylines
        int colorIndex = 0;
        foreach (var series in model.Series)
        {
            string stroke = SeriesColors[colorIndex % SeriesColors.Length];
            colorIndex++;

            var points = new StringBuilder();
            for (int i = 0; i < series.Points.Count; i++)
            {
                var p = series.Points[i];
                double sx = MapX(p.X, minX, maxX, plotLeft, plotRight);
                double sy = MapY(p.Y, minY, maxY, plotTop, plotBottom);
                if (i > 0) points.Append(' ');
                points.Append(Fmt(sx)).Append(',').Append(Fmt(sy));
            }

            sb.Append("  <polyline fill=\"none\" stroke=\"").Append(stroke)
              .Append("\" stroke-width=\"2\" points=\"").Append(points).Append("\"/>\n");
        }

        sb.Append("</svg>\n");
        return sb.ToString();
    }

    private static readonly string[] SeriesColors = ["#e6194b", "#3cb44b", "#4363d8", "#f58231", "#911eb4"];

    private static (double, double, double, double) ComputeBounds(PlotModel model)
    {
        double minX = double.PositiveInfinity, maxX = double.NegativeInfinity;
        double minY = double.PositiveInfinity, maxY = double.NegativeInfinity;

        foreach (var series in model.Series)
        {
            foreach (var p in series.Points)
            {
                if (p.X < minX) minX = p.X;
                if (p.X > maxX) maxX = p.X;
                if (p.Y < minY) minY = p.Y;
                if (p.Y > maxY) maxY = p.Y;
            }
        }

        if (double.IsPositiveInfinity(minX))
            return (0, 1, 0, 1);

        return (minX, maxX, minY, maxY);
    }

    private static (double, double) PadBounds(double min, double max)
    {
        if (min == max)
        {
            min -= 1;
            max += 1;
            return (min, max);
        }

        double pad = (max - min) * 0.05;
        return (min - pad, max + pad);
    }

    private static double MapX(double x, double minX, double maxX, double left, double right) =>
        left + (x - minX) / (maxX - minX) * (right - left);

    private static double MapY(double y, double minY, double maxY, double top, double bottom) =>
        top + (maxY - y) / (maxY - minY) * (bottom - top);

    private static List<double> NiceTicks(double min, double max, int targetCount)
    {
        if (double.IsNaN(min) || double.IsNaN(max) || min == max)
            return [min];

        double range = max - min;
        double roughStep = range / Math.Max(1, targetCount - 1);
        double mag = Math.Pow(10, Math.Floor(Math.Log10(Math.Abs(roughStep))));
        double norm = roughStep / mag;

        double step = norm switch
        {
            < 1.5 => 1,
            < 3 => 2,
            < 7 => 5,
            _ => 10,
        } * mag;

        double start = Math.Ceiling(min / step) * step;
        var ticks = new List<double>();
        for (double t = start; t <= max + step * 1e-9; t += step)
            ticks.Add(t);
        return ticks;
    }

    private static string Fmt(double value)
    {
        // Invariant, round-trip-ish but trimmed for stable output.
        string s = value.ToString("0.#####", CultureInfo.InvariantCulture);
        return s == "-0" ? "0" : s;
    }

    private static string FormatTick(double value)
    {
        if (value == 0) return "0";
        if (Math.Abs(value) >= 1e-4 && Math.Abs(value) < 1e7 && value == Math.Floor(value))
            return ((long)value).ToString(CultureInfo.InvariantCulture);
        return value.ToString("0.####", CultureInfo.InvariantCulture);
    }

    private static string Escape(string text) =>
        text.Replace("&", "&amp;").Replace("<", "&lt;").Replace(">", "&gt;").Replace("\"", "&quot;");
}

// -------------------------------------------------------------------------
// Value → double conversion (for plotting)
// -------------------------------------------------------------------------

/// <summary>Converts a numeric <see cref="Value"/> to a <see cref="double"/> for rendering.</summary>
public static class PlotValue
{
    /// <summary>
    /// Converts Natural/Integer/Real values to <see cref="double"/>, expanding
    /// periodic Real notation (e.g. <c>0.(3)</c>) into a decimal approximation.
    /// </summary>
    public static double ToDouble(Value value) => value.Kind switch
    {
        ValueKind.Natural => double.Parse(value.AsNatural().ToString(), CultureInfo.InvariantCulture),
        ValueKind.Integer => double.Parse(value.AsInteger().ToString(), CultureInfo.InvariantCulture),
        ValueKind.Real    => RealToDouble(value.AsReal().ToString()),
        _ => throw new InvalidOperationException($"Cannot convert value of kind '{value.Kind}' to a number for plotting."),
    };

    private static double RealToDouble(string s)
    {
        if (!s.Contains('('))
            return double.Parse(s, CultureInfo.InvariantCulture);

        int open = s.IndexOf('(');
        int close = s.IndexOf(')');
        string period = s[(open + 1)..close];
        string prefix = s[..open];

        var sb = new StringBuilder(prefix);
        for (int i = 0; i < 20; i++)
            sb.Append(period);

        return double.Parse(sb.ToString(), CultureInfo.InvariantCulture);
    }
}
