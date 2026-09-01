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

/// <summary>How the renderer connects the points of a series.</summary>
public enum PlotInterpolation
{
    /// <summary>Connect successive points with straight line segments.</summary>
    Linear,

    /// <summary>
    /// Draw a smooth curve through the points using a Catmull-Rom spline converted to
    /// cubic Bézier segments (C¹ continuous, passing through every data point). Series
    /// with fewer than three points fall back to <see cref="Linear"/> automatically.
    /// </summary>
    CubicSpline,
}

/// <summary>One plotted series.</summary>
public sealed class PlotSeries
{
    public List<PlotPoint> Points { get; } = new();

    /// <summary>
    /// How successive points are connected. Defaults to <see cref="PlotInterpolation.CubicSpline"/>
    /// so a coarse sample of a smooth function renders as a smooth curve rather than a
    /// piecewise-linear (angular) polygon.
    /// </summary>
    public PlotInterpolation Interpolation { get; set; } = PlotInterpolation.CubicSpline;
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

        // Series lines: smooth cubic splines by default, straight segments when
        // requested (or when there are too few points to fit a spline).
        int colorIndex = 0;
        foreach (var series in model.Series)
        {
            string stroke = SeriesColors[colorIndex % SeriesColors.Length];
            colorIndex++;

            if (series.Points.Count < 3 || series.Interpolation == PlotInterpolation.Linear)
                AppendPolyline(sb, series, MapX, MapY, stroke);
            else
                AppendSmoothCurve(sb, series, MapX, MapY, stroke);
        }

        sb.Append("</svg>\n");
        return sb.ToString();
    }

    /// <summary>Connects a series' points with straight line segments.</summary>
    private static void AppendPolyline(
        StringBuilder sb,
        PlotSeries series,
        Func<Rl, double> mapX,
        Func<Rl, double> mapY,
        string stroke)
    {
        var points = new StringBuilder();
        for (int i = 0; i < series.Points.Count; i++)
        {
            var p = series.Points[i];
            double sx = mapX(p.X);
            double sy = mapY(p.Y);
            if (i > 0) points.Append(' ');
            points.Append(Fmt(sx)).Append(',').Append(Fmt(sy));
        }

        sb.Append("  <polyline fill=\"none\" stroke=\"").Append(stroke)
          .Append("\" stroke-width=\"2\" points=\"").Append(points).Append("\"/>\n");
    }

    /// <summary>
    /// Draws a smooth curve through the series as a plain, densely sampled <c>&lt;polyline&gt;</c>.
    /// Where the x values are strictly increasing (a function of x), it fits a natural cubic spline
    /// <c>y(x)</c> — C² smooth, so it cannot introduce the kinks that a Catmull-Rom/order spline
    /// can overshoot into — and reproduces polynomials exactly (e.g. <c>y = x²</c>). For x that is
    /// not a function it falls back to a densely sampled parametric Catmull-Rom. All samples are
    /// mapped to pixels through the exact <see cref="Rl"/> mapping, and the curve is emitted as a
    /// plain polyline so every renderer draws it identically.
    /// </summary>
    private static void AppendSmoothCurve(
        StringBuilder sb,
        PlotSeries series,
        Func<Rl, double> mapX,
        Func<Rl, double> mapY,
        string stroke)
    {
        int n = series.Points.Count;
        if (n < 3)
        {
            AppendPolyline(sb, series, mapX, mapY, stroke);
            return;
        }

        // Fit the curve in data space (well conditioned for a function like y = x²) with a
        // natural cubic spline, then map each sample to a pixel through the affine transform that
        // maps the data points — avoiding any Rl round-trips on interpolated samples.
        var xd = new double[n];
        var yd = new double[n];
        var px = new double[n];
        var py = new double[n];
        for (int i = 0; i < n; i++)
        {
            xd[i] = PlotValue.ToDouble(series.Points[i].X);
            yd[i] = PlotValue.ToDouble(series.Points[i].Y);
            px[i] = mapX(series.Points[i].X);
            py[i] = mapY(series.Points[i].Y);
        }

        bool increasing = true;
        for (int i = 1; i < n; i++)
            if (!(xd[i] > xd[i - 1])) { increasing = false; break; }

        if (increasing)
            AppendNaturalSpline(sb, xd, yd, px, py, stroke);
        else
            AppendParametricSpline(sb, series, mapX, mapY, stroke);
    }

    /// <summary>
    /// Fits a natural cubic spline through (x, y) in data space and emits a densely sampled
    /// polyline, mapping each sample to a pixel via the affine transform derived from the mapped
    /// data points (so interpolated samples never go through an Rl round-trip).
    /// </summary>
    private static void AppendNaturalSpline(
        StringBuilder sb,
        double[] x,
        double[] y,
        double[] px,
        double[] py,
        string stroke)
    {
        int n = x.Length;
        var h = new double[n - 1];
        for (int i = 0; i < n - 1; i++)
            h[i] = x[i + 1] - x[i];

        // Second derivatives M under natural boundary conditions (M[0] = M[n-1] = 0).
        var M = new double[n];
        if (n > 2)
        {
            var l = new double[n];
            var mu = new double[n];
            var z = new double[n];
            l[0] = 1.0;
            mu[0] = 0.0;
            z[0] = 0.0;
            for (int i = 1; i < n - 1; i++)
            {
                l[i] = 2.0 * (x[i + 1] - x[i - 1]) - (h[i - 1] * mu[i - 1]);
                mu[i] = h[i] / l[i];
                z[i] = (6.0 * (((y[i + 1] - y[i]) / h[i]) - ((y[i] - y[i - 1]) / h[i - 1])) - (h[i - 1] * z[i - 1])) / l[i];
            }
            l[n - 1] = 1.0;
            for (int j = n - 2; j >= 1; j--)
                M[j] = z[j] - (mu[j] * M[j + 1]);
        }

        // Affine data → pixel, derived from the mapped data points.
        double ax = (px[n - 1] - px[0]) / (x[n - 1] - x[0]);
        double bx = px[0] - (ax * x[0]);
        double ySpan = y[n - 1] - y[0];
        double ay = Math.Abs(ySpan) > 1e-12 ? (py[n - 1] - py[0]) / ySpan : 0.0;
        double by = py[0] - (ay * y[0]);

        double xLo = x[0];
        double xHi = x[n - 1];
        int samples = Math.Max(200, (int)Math.Ceiling(Math.Abs(px[n - 1] - px[0])) + 1);
        var points = new StringBuilder();
        for (int s = 0; s <= samples; s++)
        {
            double xs = xLo + (((xHi - xLo) * s) / samples);
            double ys = SplineValue(x, y, M, h, xs);
            double sx = (ax * xs) + bx;
            double sy = (ay * ys) + by;
            if (s > 0) points.Append(' ');
            points.Append(Fmt(sx)).Append(',').Append(Fmt(sy));
        }

        sb.Append("  <polyline fill=\"none\" stroke=\"").Append(stroke)
          .Append("\" stroke-width=\"2\" points=\"").Append(points).Append("\"/>\n");
    }

    /// <summary>Evaluates the natural cubic spline at <paramref name="xs"/>.</summary>
    private static double SplineValue(double[] x, double[] y, double[] M, double[] h, double xs)
    {
        int n = x.Length;
        int i = Array.BinarySearch(x, xs);
        if (i < 0) i = ~i - 1;
        if (i < 0) i = 0;
        if (i > n - 2) i = n - 2;

        double A = x[i + 1] - xs;
        double B = xs - x[i];
        double hi = h[i];
        return (M[i] * A * A * A / (6.0 * hi))
             + (M[i + 1] * B * B * B / (6.0 * hi))
             + ((y[i] / hi) - (M[i] * hi / 6.0)) * A
             + ((y[i + 1] / hi) - (M[i + 1] * hi / 6.0)) * B;
    }

    /// <summary>
    /// Densely sampled parametric Catmull-Rom, used when x is not a function of y (i.e. the points
    /// do not trace a single-valued curve). Emitted as a plain polyline.
    /// </summary>
    private static void AppendParametricSpline(
        StringBuilder sb,
        PlotSeries series,
        Func<Rl, double> mapX,
        Func<Rl, double> mapY,
        string stroke)
    {
        int n = series.Points.Count;
        var pts = new (double x, double y)[n];
        for (int i = 0; i < n; i++)
            pts[i] = (mapX(series.Points[i].X), mapY(series.Points[i].Y));

        var points = new StringBuilder();
        points.Append(Fmt(pts[0].x)).Append(',').Append(Fmt(pts[0].y));

        int last = n - 1;
        for (int i = 0; i < last; i++)
        {
            (double x, double y) p0 = i > 0 ? pts[i - 1] : pts[i];
            (double x, double y) p1 = pts[i];
            (double x, double y) p2 = pts[i + 1];
            (double x, double y) p3 = i + 1 < last ? pts[i + 2] : pts[i + 1];

            double dx = p2.x - p1.x;
            double dy = p2.y - p1.y;
            double length = Math.Sqrt((dx * dx) + (dy * dy));
            int sub = Math.Max(1, (int)Math.Ceiling(length / 2.0));

            for (int s = 1; s <= sub; s++)
            {
                double t = (double)s / sub;
                var q = CatmullRom(p0, p1, p2, p3, t);
                points.Append(' ').Append(Fmt(q.x)).Append(',').Append(Fmt(q.y));
            }
        }

        sb.Append("  <polyline fill=\"none\" stroke=\"").Append(stroke)
          .Append("\" stroke-width=\"2\" points=\"").Append(points).Append("\"/>\n");
    }

    /// <summary>Evaluates a uniform Catmull-Rom spline at parameter <paramref name="t"/> ∈ [0, 1].</summary>
    private static (double x, double y) CatmullRom(
        (double x, double y) p0,
        (double x, double y) p1,
        (double x, double y) p2,
        (double x, double y) p3,
        double t)
    {
        double t2 = t * t;
        double t3 = t2 * t;
        double x = 0.5 * ((2.0 * p1.x) + ((-p0.x + p2.x) * t) + ((2.0 * p0.x - 5.0 * p1.x + 4.0 * p2.x - p3.x) * t2) + ((-p0.x + 3.0 * p1.x - 3.0 * p2.x + p3.x) * t3));
        double y = 0.5 * ((2.0 * p1.y) + ((-p0.y + p2.y) * t) + ((2.0 * p0.y - 5.0 * p1.y + 4.0 * p2.y - p3.y) * t2) + ((-p0.y + 3.0 * p1.y - 3.0 * p2.y + p3.y) * t3));
        return (x, y);
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
