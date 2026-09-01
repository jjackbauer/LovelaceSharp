using System.Globalization;
using System.Text;
using Nat = global::Lovelace.Natural.Natural;
using Int = global::Lovelace.Integer.Integer;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Suite;

// -------------------------------------------------------------------------
// Plot model (renderer-agnostic)
// -------------------------------------------------------------------------

/// <summary>
/// A single (x, y) point, held in arbitrary-precision <see cref="Rl"/> so that bounds,
/// padding, and normalization stay exact until the final pixel conversion.
/// </summary>
public readonly record struct PlotPoint(Rl X, Rl Y)
{
    /// <summary>
    /// Convenience constructor for <see cref="double"/> coordinates. The input is
    /// inherently limited to double precision; prefer the <see cref="Rl"/> overload
    /// when the source is already a Lovelace value.
    /// </summary>
    public PlotPoint(double x, double y) : this(new Rl(x), new Rl(y)) { }
}

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
/// Bounds, padding, and normalization are computed in arbitrary-precision
/// <see cref="Rl"/>; a value is converted to <see cref="double"/> only at the final
/// <c>[0,1] → pixel</c> step, so distinct inputs never collapse through double rounding.
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

        Rl rangeX = maxX - minX;
        Rl rangeY = maxY - minY;

        // Local mapping helpers: exact Real normalization, then one double multiply.
        double MapX(Rl x) =>
            plotLeft + PlotValue.ToDouble((x - minX) / rangeX) * (plotRight - plotLeft);

        double MapY(Rl y) =>
            plotTop + PlotValue.ToDouble(Rl.One - (y - minY) / rangeY) * (plotBottom - plotTop);

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
        double yAxisX = minX <= Rl.Zero && maxX >= Rl.Zero ? MapX(Rl.Zero) : plotLeft;
        double xAxisY = minY <= Rl.Zero && maxY >= Rl.Zero ? MapY(Rl.Zero) : plotBottom;

        sb.Append("  <line x1=\"").Append(Fmt(yAxisX)).Append("\" y1=\"").Append(Fmt(plotTop))
          .Append("\" x2=\"").Append(Fmt(yAxisX)).Append("\" y2=\"").Append(Fmt(plotBottom))
          .Append("\" stroke=\"#333333\"/>\n");
        sb.Append("  <line x1=\"").Append(Fmt(plotLeft)).Append("\" y1=\"").Append(Fmt(xAxisY))
          .Append("\" x2=\"").Append(Fmt(plotRight)).Append("\" y2=\"").Append(Fmt(xAxisY))
          .Append("\" stroke=\"#333333\"/>\n");

        // X ticks
        foreach (Rl tick in NiceTicks(minX, maxX, 6))
        {
            double tx = MapX(tick);
            sb.Append("  <line x1=\"").Append(Fmt(tx)).Append("\" y1=\"").Append(Fmt(xAxisY - 4))
              .Append("\" x2=\"").Append(Fmt(tx)).Append("\" y2=\"").Append(Fmt(xAxisY + 4))
              .Append("\" stroke=\"#333333\"/>\n");
            sb.Append("  <text x=\"").Append(Fmt(tx)).Append("\" y=\"").Append(Fmt(xAxisY + 18))
              .Append("\" text-anchor=\"middle\" font-family=\"sans-serif\" font-size=\"12\">")
              .Append(FormatTick(tick)).Append("</text>\n");
        }

        // Y ticks
        foreach (Rl tick in NiceTicks(minY, maxY, 6))
        {
            double ty = MapY(tick);
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
                double sx = MapX(p.X);
                double sy = MapY(p.Y);
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

    private static (Rl, Rl, Rl, Rl) ComputeBounds(PlotModel model)
    {
        Rl minX = Rl.Zero, maxX = Rl.Zero, minY = Rl.Zero, maxY = Rl.Zero;
        bool any = false;

        foreach (var series in model.Series)
        {
            foreach (var p in series.Points)
            {
                if (!any)
                {
                    minX = maxX = p.X;
                    minY = maxY = p.Y;
                    any = true;
                }
                else
                {
                    if (p.X < minX) minX = p.X;
                    if (p.X > maxX) maxX = p.X;
                    if (p.Y < minY) minY = p.Y;
                    if (p.Y > maxY) maxY = p.Y;
                }
            }
        }

        if (!any)
            return (Rl.Zero, Rl.One, Rl.Zero, Rl.One);

        return (minX, maxX, minY, maxY);
    }

    private static (Rl, Rl) PadBounds(Rl min, Rl max)
    {
        if (min == max)
            return (min - Rl.One, max + Rl.One);

        // 5% padding each side = range / 20, computed exactly.
        Rl pad = (max - min) / new Rl(20.0);
        return (min - pad, max + pad);
    }

    private static List<Rl> NiceTicks(Rl min, Rl max, int targetCount)
    {
        var ticks = new List<Rl>();
        if (min >= max)
        {
            ticks.Add(min);
            return ticks;
        }

        Rl range = max - min;
        Rl rough = range / new Rl((double)(targetCount - 1));
        long kk = OrderOfMagnitude(rough);
        Rl scaled = rough / Pow10(kk);
        double norm = PlotValue.ToDouble(scaled); // safe: scaled ∈ [1, 10)
        long nice = norm < 1.5 ? 1 : norm < 3 ? 2 : norm < 7 ? 5 : 10;
        Rl step = new Rl((double)nice) * Pow10(kk);

        Rl q = min / step;
        Rl start = Ceil(q) * step;

        const int cap = 1000;
        for (Rl t = start; ticks.Count < cap && t <= max; t += step)
            ticks.Add(t);

        return ticks;
    }

    /// <summary>floor(log10(|r|)), computed from the decimal representation (no transcendentals).</summary>
    private static long OrderOfMagnitude(Rl r)
    {
        if (Rl.IsZero(r)) return 0;

        if (!r.IsPeriodic)
            return (long)r.ToNatural().ToString().Length - 1 + r.Exponent;

        return OrderOfMagnitudeFromDecimal(PlotValue.ToPlainDecimal(r));
    }

    private static long OrderOfMagnitudeFromDecimal(string s)
    {
        if (s.StartsWith('-')) s = s[1..];

        int dot = s.IndexOf('.');
        string intPart = dot < 0 ? s : s[..dot];

        int i = 0;
        while (i < intPart.Length && intPart[i] == '0') i++;
        if (i < intPart.Length)
            return (long)(intPart.Length - i) - 1;

        string frac = dot < 0 ? "" : s[(dot + 1)..];
        int j = 0;
        while (j < frac.Length && frac[j] == '0') j++;
        return -(long)(j + 1);
    }

    /// <summary>10^k as an exact <see cref="Rl"/>.</summary>
    private static Rl Pow10(long k)
    {
        if (k >= 0)
            return new Rl("1" + new string('0', (int)k));
        return new Rl("0." + new string('0', (int)(-k - 1)) + "1");
    }

    /// <summary>Smallest integer ≥ <paramref name="q"/>, as an exact <see cref="Rl"/>.</summary>
    private static Rl Ceil(Rl q)
    {
        Rl trunc = TruncateTowardZero(q);
        return q > trunc ? trunc + Rl.One : trunc;
    }

    /// <summary>Integer part of <paramref name="q"/> (truncated toward zero), as an exact <see cref="Rl"/>.</summary>
    private static Rl TruncateTowardZero(Rl q)
    {
        string s = q.ToString();
        bool neg = s.StartsWith('-');
        if (neg) s = s[1..];

        int dot = s.IndexOf('.');
        string intPart = dot < 0 ? s : s[..dot];

        int paren = intPart.IndexOf('(');
        if (paren >= 0) intPart = intPart[..paren];

        if (intPart.Length == 0 || intPart.All(c => c == '0'))
            return Rl.Zero;

        return new Rl((neg ? "-" : "") + intPart);
    }

    private static string Fmt(double value)
    {
        // Invariant, round-trip-ish but trimmed for stable output.
        string s = value.ToString("0.#####", CultureInfo.InvariantCulture);
        return s == "-0" ? "0" : s;
    }

    private static string FormatTick(Rl value)
    {
        if (Rl.IsZero(value)) return "0";
        // Ticks are exact "nice" decimals (1/2/5 × 10^k), so their decimal form is clean.
        return PlotValue.ToPlainDecimal(value);
    }

    private static string Escape(string text) =>
        text.Replace("&", "&amp;").Replace("<", "&lt;").Replace(">", "&gt;").Replace("\"", "&quot;");
}

// -------------------------------------------------------------------------
// Value → Real / double conversion (for plotting)
// -------------------------------------------------------------------------

/// <summary>Converts numeric <see cref="Value"/>s for plotting.</summary>
public static class PlotValue
{
    /// <summary>
    /// Promotes a Natural/Integer/Real <see cref="Value"/> to <see cref="Rl"/> exactly
    /// (no string or double round-trip), for arbitrary-precision plotting.
    /// </summary>
    public static Rl ToReal(Value value) => value.Kind switch
    {
        ValueKind.Natural => new Rl(new Int(value.AsNatural())),
        ValueKind.Integer => new Rl(value.AsInteger()),
        ValueKind.Real    => value.AsReal(),
        _ => throw new InvalidOperationException($"Cannot convert value of kind '{value.Kind}' to a number for plotting."),
    };

    /// <summary>
    /// Converts a numeric <see cref="Value"/> to <see cref="double"/> for rendering.
    /// Prefer <see cref="ToReal(Value)"/> for exact plotting; this exists for callers
    /// that need a double and for periodic-real expansion tests.
    /// </summary>
    public static double ToDouble(Value value) => ToDouble(ToReal(value));

    /// <summary>
    /// Converts an <see cref="Rl"/> to <see cref="double"/>, expanding periodic notation
    /// (e.g. <c>0.(3)</c>) into a decimal approximation first. Safe for values in [0, 1];
    /// magnitude-limited by <see cref="double"/> like any double conversion.
    /// </summary>
    public static double ToDouble(Rl value) =>
        double.Parse(ToPlainDecimal(value), CultureInfo.InvariantCulture);

    /// <summary>
    /// Returns a plain decimal string for <paramref name="value"/>, expanding a periodic
    /// notation block <paramref name="repeat"/> times (default 20 — far more than double
    /// precision needs, but bounded).
    /// </summary>
    public static string ToPlainDecimal(Rl value, int repeat = 20)
    {
        string s = value.ToString();
        if (!s.Contains('('))
            return s;

        int open = s.IndexOf('(');
        int close = s.IndexOf(')');
        string period = s[(open + 1)..close];
        string prefix = s[..open];

        var sb = new StringBuilder(prefix.Length + period.Length * repeat);
        sb.Append(prefix);
        for (int i = 0; i < repeat; i++)
            sb.Append(period);
        return sb.ToString();
    }
}
