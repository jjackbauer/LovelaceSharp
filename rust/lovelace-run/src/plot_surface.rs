//! The additive plot surface: `PlotValue`, the renderer model, renderer options and the wire
//! serialisation of a rendered plot.
//!
//! This is the Rust equivalent of the reference's `Lovelace.Suite.Plotting.cs`: the model is
//! renderer-agnostic and holds its coordinates in arbitrary-precision `Real`, so bounds, padding
//! and normalisation stay exact until the final `[0,1] -> pixel` conversion. It is ADDITIVE: the
//! runner's `plot()` builtin keeps its own SVG pipeline, so no corpus or fixture byte moves. The
//! equivalence is member-for-member (`PlotValue`, `PlotPoint`, `PlotSeries`, `PlotModel`,
//! `PlotInterpolation`, `SvgPlotRenderer`, `PlotCapture`), with two Rust-shaped additions: explicit
//! `RenderOptions` and JSON serialisation of the model and the capture.

// LangError carries the wire payload inline by design (the language crate allows the same lint);
// boxing it only here would make the plot surface's error type differ from the engine's.
#![allow(clippy::result_large_err)]

use std::fmt::Write as _;

use lovelace_int::Int;
use lovelace_json::Json;
use lovelace_lang::{LangError, Value};
use lovelace_real::Real;

/// A single `(x, y)` point held in arbitrary-precision `Real` (the reference's `PlotPoint`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlotPoint {
    /// The exact x coordinate.
    pub x: Real,
    /// The exact y coordinate.
    pub y: Real,
}

impl PlotPoint {
    /// Builds a point from two exact reals.
    pub fn new(x: Real, y: Real) -> PlotPoint {
        PlotPoint { x, y }
    }

    /// Builds a point from two `f64` coordinates (the reference's double convenience ctor).
    pub fn from_f64(x: f64, y: f64) -> PlotPoint {
        PlotPoint {
            x: real_from_f64(x),
            y: real_from_f64(y),
        }
    }
}

/// How the renderer connects the points of a series (the reference's `PlotInterpolation`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlotInterpolation {
    /// Connect successive points with straight line segments.
    Linear,
    /// Draw a smooth natural cubic spline through the points, emitted as a dense polyline.
    #[default]
    CubicSpline,
}

impl PlotInterpolation {
    /// The stable wire name, as the model JSON publishes it.
    pub fn wire_name(self) -> &'static str {
        match self {
            PlotInterpolation::Linear => "Linear",
            PlotInterpolation::CubicSpline => "CubicSpline",
        }
    }
}

/// One plotted series (the reference's `PlotSeries`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlotSeries {
    /// The series' points, in drawing order.
    pub points: Vec<PlotPoint>,
    /// How successive points are connected; defaults to `CubicSpline` like the reference.
    pub interpolation: PlotInterpolation,
}

impl PlotSeries {
    /// An empty series with the reference's default interpolation.
    pub fn new() -> PlotSeries {
        PlotSeries::default()
    }

    /// An empty series with an explicit interpolation.
    pub fn with_interpolation(interpolation: PlotInterpolation) -> PlotSeries {
        PlotSeries {
            points: Vec::new(),
            interpolation,
        }
    }

    /// Appends one point.
    pub fn push(&mut self, point: PlotPoint) {
        self.points.push(point);
    }

    /// Appends every point of an iterator.
    pub fn with_points(mut self, points: impl IntoIterator<Item = PlotPoint>) -> PlotSeries {
        self.points.extend(points);
        self
    }
}

/// A renderer-agnostic 2-D line plot (the reference's `PlotModel`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlotModel {
    /// Optional title rendered above the plot area.
    pub title: Option<String>,
    /// The plotted series, in drawing order.
    pub series: Vec<PlotSeries>,
}

impl PlotModel {
    /// An empty model.
    pub fn new() -> PlotModel {
        PlotModel::default()
    }

    /// Adds a series and returns the model for chaining.
    pub fn with_series(mut self, series: PlotSeries) -> PlotModel {
        self.series.push(series);
        self
    }

    /// Serialises the model to its stable wire shape:
    /// `{"series":[{"interpolation":"CubicSpline","points":[{"x":"1","y":"2"}]}],"title":null}`.
    ///
    /// Coordinates are kept as exact decimal STRINGS because a periodic value such as `0.(3)`
    /// cannot be a JSON number; every other string is the value's canonical `Real` rendering.
    pub fn to_wire_json(&self) -> Json {
        let series = self
            .series
            .iter()
            .map(|series| {
                let points = series
                    .points
                    .iter()
                    .map(|point| {
                        Json::obj(vec![
                            ("x", Json::Str(point.x.to_string())),
                            ("y", Json::Str(point.y.to_string())),
                        ])
                    })
                    .collect();
                Json::obj(vec![
                    (
                        "interpolation",
                        Json::Str(series.interpolation.wire_name().to_string()),
                    ),
                    ("points", Json::Arr(points)),
                ])
            })
            .collect();
        let title = match &self.title {
            Some(title) => Json::Str(title.clone()),
            None => Json::Null,
        };
        Json::obj(vec![("series", Json::Arr(series)), ("title", title)])
    }
}

/// The capture of one rendered plot (the reference's `PlotCapture`), with the runner's path.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlotCapture {
    /// The rendered SVG document.
    pub svg: String,
    /// The plot title, when the model carried one.
    pub title: Option<String>,
}

impl PlotCapture {
    /// Captures one render.
    pub fn new(svg: String, title: Option<String>) -> PlotCapture {
        PlotCapture { svg, title }
    }

    /// Serialises the runner's `plot` envelope block:
    /// `{"path":"...","svg":"...","title":""}` (an absent title publishes as the empty string,
    /// exactly as the runner's wire block does).
    pub fn to_wire_json(&self, path: &str) -> Json {
        Json::obj(vec![
            ("path", Json::Str(path.to_string())),
            ("title", Json::Str(self.title.clone().unwrap_or_default())),
            ("svg", Json::Str(self.svg.clone())),
        ])
    }
}

/// Renderer options (the reference's constants, made explicit so a host can size the document).
#[derive(Clone, Debug, PartialEq)]
pub struct RenderOptions {
    /// Document width in pixels (reference: 800).
    pub width: f64,
    /// Document height in pixels (reference: 600).
    pub height: f64,
    /// Left plot margin in pixels (reference: 70).
    pub margin_left: f64,
    /// Right plot margin in pixels (reference: 30).
    pub margin_right: f64,
    /// Top plot margin in pixels (reference: 50).
    pub margin_top: f64,
    /// Bottom plot margin in pixels (reference: 55).
    pub margin_bottom: f64,
    /// Minimum number of samples on a smooth curve (reference: 200).
    pub samples: usize,
    /// One stroke colour per series, cycled in order (the reference's palette).
    pub series_colors: Vec<String>,
}

impl Default for RenderOptions {
    fn default() -> RenderOptions {
        RenderOptions {
            width: 800.0,
            height: 600.0,
            margin_left: 70.0,
            margin_right: 30.0,
            margin_top: 50.0,
            margin_bottom: 55.0,
            samples: 200,
            series_colors: vec![
                "#e6194b".to_string(),
                "#3cb44b".to_string(),
                "#4363d8".to_string(),
                "#f58231".to_string(),
                "#911eb4".to_string(),
            ],
        }
    }
}

/// Renders a `PlotModel` to a deterministic, self-contained SVG (the reference's
/// `SvgPlotRenderer`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SvgPlotRenderer {
    options: RenderOptions,
}

impl SvgPlotRenderer {
    /// A renderer with the reference's default geometry and palette.
    pub fn new() -> SvgPlotRenderer {
        SvgPlotRenderer {
            options: RenderOptions::default(),
        }
    }

    /// A renderer with explicit options.
    pub fn with_options(options: RenderOptions) -> SvgPlotRenderer {
        SvgPlotRenderer { options }
    }

    /// The options in force.
    pub fn options(&self) -> &RenderOptions {
        &self.options
    }

    /// Renders the model to SVG. The output is deterministic: no timestamps, random ids or
    /// culture-dependent formatting.
    pub fn render(&self, model: &PlotModel) -> String {
        let width = self.options.width;
        let height = self.options.height;
        let plot_left = self.options.margin_left;
        let plot_right = width - self.options.margin_right;
        let plot_top = self.options.margin_top;
        let plot_bottom = height - self.options.margin_bottom;

        let (min_x, max_x, min_y, max_y) = compute_bounds(model);
        let (min_x, max_x) = pad_bounds(min_x, max_x);
        let (min_y, max_y) = pad_bounds(min_y, max_y);

        let range_x = max_x.sub(&min_x);
        let range_y = max_y.sub(&min_y);

        let map_x = |x: &Real| {
            plot_left
                + PlotValue::to_double_real(&divide(&x.sub(&min_x), &range_x))
                    * (plot_right - plot_left)
        };
        let map_y = |y: &Real| {
            plot_top
                + PlotValue::to_double_real(
                    &real_from_i64(1).sub(&divide(&y.sub(&min_y), &range_y)),
                ) * (plot_bottom - plot_top)
        };

        let mut out = String::new();
        let _ = writeln!(
            out,
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">"
        );
        let _ = writeln!(
            out,
            "  <rect width=\"{width}\" height=\"{height}\" fill=\"white\"/>"
        );

        if let Some(title) = &model.title {
            if !title.is_empty() {
                let _ = writeln!(
                    out,
                    "  <text x=\"{}\" y=\"28\" text-anchor=\"middle\" font-family=\"sans-serif\" font-size=\"20\">{}</text>",
                    fmt(width / 2.0),
                    escape(title)
                );
            }
        }

        let _ = writeln!(
            out,
            "  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"#cccccc\"/>",
            fmt(plot_left),
            fmt(plot_top),
            fmt(plot_right - plot_left),
            fmt(plot_bottom - plot_top)
        );

        let zero = Real::zero();
        let y_axis_x = if min_x.cmp(&zero) != std::cmp::Ordering::Greater
            && max_x.cmp(&zero) != std::cmp::Ordering::Less
        {
            map_x(&zero)
        } else {
            plot_left
        };
        let x_axis_y = if min_y.cmp(&zero) != std::cmp::Ordering::Greater
            && max_y.cmp(&zero) != std::cmp::Ordering::Less
        {
            map_y(&zero)
        } else {
            plot_bottom
        };

        let _ = writeln!(
            out,
            "  <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#333333\"/>",
            fmt(y_axis_x),
            fmt(plot_top),
            fmt(y_axis_x),
            fmt(plot_bottom)
        );
        let _ = writeln!(
            out,
            "  <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#333333\"/>",
            fmt(plot_left),
            fmt(x_axis_y),
            fmt(plot_right),
            fmt(x_axis_y)
        );

        for tick in nice_ticks(&min_x, &max_x, 6) {
            let tx = map_x(&tick);
            let _ = writeln!(
                out,
                "  <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#333333\"/>",
                fmt(tx),
                fmt(x_axis_y - 4.0),
                fmt(tx),
                fmt(x_axis_y + 4.0)
            );
            let _ = writeln!(
                out,
                "  <text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"sans-serif\" font-size=\"12\">{}</text>",
                fmt(tx),
                fmt(x_axis_y + 18.0),
                format_tick(&tick)
            );
        }
        for tick in nice_ticks(&min_y, &max_y, 6) {
            let ty = map_y(&tick);
            let _ = writeln!(
                out,
                "  <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#333333\"/>",
                fmt(y_axis_x - 4.0),
                fmt(ty),
                fmt(y_axis_x + 4.0),
                fmt(ty)
            );
            let _ = writeln!(
                out,
                "  <text x=\"{}\" y=\"{}\" text-anchor=\"end\" font-family=\"sans-serif\" font-size=\"12\">{}</text>",
                fmt(y_axis_x - 8.0),
                fmt(ty + 4.0),
                format_tick(&tick)
            );
        }

        for (index, series) in model.series.iter().enumerate() {
            let colors = &self.options.series_colors;
            let stroke = if colors.is_empty() {
                "#e6194b"
            } else {
                colors[index % colors.len()].as_str()
            };
            if series.points.len() < 3 || series.interpolation == PlotInterpolation::Linear {
                append_polyline(&mut out, series, &map_x, &map_y, stroke);
            } else {
                self.append_smooth_curve(&mut out, series, &map_x, &map_y, stroke);
            }
        }

        out.push_str("</svg>\n");
        out
    }

    /// Draws a smooth curve through the series as a plain, densely sampled `<polyline>`. Where x
    /// is strictly increasing it fits a natural cubic spline in data space; otherwise it falls
    /// back to a densely sampled parametric Catmull-Rom, exactly like the reference.
    fn append_smooth_curve(
        &self,
        out: &mut String,
        series: &PlotSeries,
        map_x: &impl Fn(&Real) -> f64,
        map_y: &impl Fn(&Real) -> f64,
        stroke: &str,
    ) {
        let n = series.points.len();
        if n < 3 {
            append_polyline(out, series, map_x, map_y, stroke);
            return;
        }

        let mut xd = Vec::with_capacity(n);
        let mut yd = Vec::with_capacity(n);
        let mut px = Vec::with_capacity(n);
        let mut py = Vec::with_capacity(n);
        for point in &series.points {
            xd.push(PlotValue::to_double_real(&point.x));
            yd.push(PlotValue::to_double_real(&point.y));
            px.push(map_x(&point.x));
            py.push(map_y(&point.y));
        }

        let increasing = (1..n).all(|i| xd[i] > xd[i - 1]);
        if increasing {
            append_natural_spline(out, &xd, &yd, &px, &py, stroke, self.options.samples);
        } else {
            append_parametric_spline(out, &px, &py, stroke);
        }
    }
}

/// Connects a series' points with straight line segments.
fn append_polyline(
    out: &mut String,
    series: &PlotSeries,
    map_x: &impl Fn(&Real) -> f64,
    map_y: &impl Fn(&Real) -> f64,
    stroke: &str,
) {
    let mut points = String::new();
    for (index, point) in series.points.iter().enumerate() {
        if index > 0 {
            points.push(' ');
        }
        let _ = write!(points, "{},{}", fmt(map_x(&point.x)), fmt(map_y(&point.y)));
    }
    let _ = writeln!(
        out,
        "  <polyline fill=\"none\" stroke=\"{stroke}\" stroke-width=\"2\" points=\"{points}\"/>"
    );
}

/// Fits a natural cubic spline through `(x, y)` in data space and emits a densely sampled
/// polyline, mapping each sample through the affine data-to-pixel transform derived from the
/// mapped data points (the reference's `AppendNaturalSpline`).
fn append_natural_spline(
    out: &mut String,
    x: &[f64],
    y: &[f64],
    px: &[f64],
    py: &[f64],
    stroke: &str,
    minimum_samples: usize,
) {
    let n = x.len();
    let mut h = vec![0.0; n - 1];
    for i in 0..n - 1 {
        h[i] = x[i + 1] - x[i];
    }

    let mut m = vec![0.0; n];
    if n > 2 {
        let mut l = vec![0.0; n];
        let mut mu = vec![0.0; n];
        let mut z = vec![0.0; n];
        l[0] = 1.0;
        for i in 1..n - 1 {
            l[i] = 2.0 * (x[i + 1] - x[i - 1]) - (h[i - 1] * mu[i - 1]);
            mu[i] = h[i] / l[i];
            z[i] = (6.0 * (((y[i + 1] - y[i]) / h[i]) - ((y[i] - y[i - 1]) / h[i - 1]))
                - (h[i - 1] * z[i - 1]))
                / l[i];
        }
        l[n - 1] = 1.0;
        for j in (1..=n - 2).rev() {
            m[j] = z[j] - (mu[j] * m[j + 1]);
        }
    }

    let ax = (px[n - 1] - px[0]) / (x[n - 1] - x[0]);
    let bx = px[0] - (ax * x[0]);
    let y_span = y[n - 1] - y[0];
    let ay = if y_span.abs() > 1e-12 {
        (py[n - 1] - py[0]) / y_span
    } else {
        0.0
    };
    let by = py[0] - (ay * y[0]);

    let x_lo = x[0];
    let x_hi = x[n - 1];
    let samples = ((px[n - 1] - px[0]).abs().ceil() as usize + 1).max(minimum_samples);
    let mut points = String::new();
    for s in 0..=samples {
        let xs = x_lo + (((x_hi - x_lo) * s as f64) / samples as f64);
        let ys = spline_value(x, y, &m, &h, xs);
        if s > 0 {
            points.push(' ');
        }
        let _ = write!(points, "{},{}", fmt((ax * xs) + bx), fmt((ay * ys) + by));
    }
    let _ = writeln!(
        out,
        "  <polyline fill=\"none\" stroke=\"{stroke}\" stroke-width=\"2\" points=\"{points}\"/>"
    );
}

/// Evaluates the natural cubic spline at `xs`.
fn spline_value(x: &[f64], y: &[f64], m: &[f64], h: &[f64], xs: f64) -> f64 {
    let n = x.len();
    let mut i = match x.binary_search_by(|probe| probe.partial_cmp(&xs).unwrap()) {
        Ok(index) => index,
        Err(index) => index.saturating_sub(1),
    };
    if i > n - 2 {
        i = n - 2;
    }
    let a = x[i + 1] - xs;
    let b = xs - x[i];
    let hi = h[i];
    (m[i] * a * a * a / (6.0 * hi))
        + (m[i + 1] * b * b * b / (6.0 * hi))
        + (((y[i] / hi) - (m[i] * hi / 6.0)) * a)
        + (((y[i + 1] / hi) - (m[i + 1] * hi / 6.0)) * b)
}

/// Densely sampled parametric Catmull-Rom, used when x is not a function of y.
fn append_parametric_spline(out: &mut String, px: &[f64], py: &[f64], stroke: &str) {
    let n = px.len();
    let mut points = String::new();
    let _ = write!(points, "{},{}", fmt(px[0]), fmt(py[0]));

    let last = n - 1;
    for i in 0..last {
        let p0 = if i > 0 {
            (px[i - 1], py[i - 1])
        } else {
            (px[i], py[i])
        };
        let p1 = (px[i], py[i]);
        let p2 = (px[i + 1], py[i + 1]);
        let p3 = if i + 1 < last {
            (px[i + 2], py[i + 2])
        } else {
            (px[i + 1], py[i + 1])
        };

        let dx = p2.0 - p1.0;
        let dy = p2.1 - p1.1;
        let length = (dx * dx + dy * dy).sqrt();
        let sub = ((length / 2.0).ceil() as usize).max(1);
        for s in 1..=sub {
            let t = s as f64 / sub as f64;
            let (qx, qy) = catmull_rom(p0, p1, p2, p3, t);
            points.push(' ');
            points.push_str(&format!("{},{}", fmt(qx), fmt(qy)));
        }
    }

    let _ = writeln!(
        out,
        "  <polyline fill=\"none\" stroke=\"{stroke}\" stroke-width=\"2\" points=\"{points}\"/>"
    );
}

/// Evaluates a uniform Catmull-Rom spline at parameter `t` in `[0, 1]`.
fn catmull_rom(
    p0: (f64, f64),
    p1: (f64, f64),
    p2: (f64, f64),
    p3: (f64, f64),
    t: f64,
) -> (f64, f64) {
    let t2 = t * t;
    let t3 = t2 * t;
    let x = 0.5
        * ((2.0 * p1.0)
            + ((-p0.0 + p2.0) * t)
            + ((2.0 * p0.0 - 5.0 * p1.0 + 4.0 * p2.0 - p3.0) * t2)
            + ((-p0.0 + 3.0 * p1.0 - 3.0 * p2.0 + p3.0) * t3));
    let y = 0.5
        * ((2.0 * p1.1)
            + ((-p0.1 + p2.1) * t)
            + ((2.0 * p0.1 - 5.0 * p1.1 + 4.0 * p2.1 - p3.1) * t2)
            + ((-p0.1 + 3.0 * p1.1 - 3.0 * p2.1 + p3.1) * t3));
    (x, y)
}

/// The bounds of every point in the model; an empty model uses the unit square.
fn compute_bounds(model: &PlotModel) -> (Real, Real, Real, Real) {
    let mut min_x = Real::zero();
    let mut max_x = Real::zero();
    let mut min_y = Real::zero();
    let mut max_y = Real::zero();
    let mut any = false;

    for series in &model.series {
        for point in &series.points {
            if !any {
                min_x = point.x.clone();
                max_x = point.x.clone();
                min_y = point.y.clone();
                max_y = point.y.clone();
                any = true;
            } else {
                if point.x.cmp(&min_x) == std::cmp::Ordering::Less {
                    min_x = point.x.clone();
                }
                if point.x.cmp(&max_x) == std::cmp::Ordering::Greater {
                    max_x = point.x.clone();
                }
                if point.y.cmp(&min_y) == std::cmp::Ordering::Less {
                    min_y = point.y.clone();
                }
                if point.y.cmp(&max_y) == std::cmp::Ordering::Greater {
                    max_y = point.y.clone();
                }
            }
        }
    }

    if !any {
        return (
            Real::zero(),
            Real::from_i64(1),
            Real::zero(),
            Real::from_i64(1),
        );
    }
    (min_x, max_x, min_y, max_y)
}

/// Pads a range by 5% on each side; a single value pads by one unit on each side.
fn pad_bounds(min: Real, max: Real) -> (Real, Real) {
    if min == max {
        return (min.sub(&Real::from_i64(1)), max.add(&Real::from_i64(1)));
    }
    let pad = divide(&max.sub(&min), &Real::from_i64(20));
    (min.sub(&pad), max.add(&pad))
}

/// The reference's `NiceTicks`: 1/2/5/10 steps chosen from the range's order of magnitude.
fn nice_ticks(min: &Real, max: &Real, target_count: usize) -> Vec<Real> {
    let mut ticks = Vec::new();
    if min.cmp(max) != std::cmp::Ordering::Less {
        ticks.push(min.clone());
        return ticks;
    }
    if target_count < 2 {
        return ticks;
    }

    let range = max.sub(min);
    let rough = divide(&range, &Real::from_i64((target_count - 1) as i64));
    let kk = order_of_magnitude(&rough);
    let scaled = divide(&rough, &pow10(kk));
    let norm = PlotValue::to_double_real(&scaled);
    let nice = if norm < 1.5 {
        1
    } else if norm < 3.0 {
        2
    } else if norm < 7.0 {
        5
    } else {
        10
    };
    let step = real_from_i64(nice).mul(&pow10(kk));

    let start = ceil(&divide(min, &step)).mul(&step);
    let cap = 1000usize;
    let mut tick = start;
    while tick.cmp(max) != std::cmp::Ordering::Greater && ticks.len() < cap {
        ticks.push(tick.clone());
        tick = tick.add(&step);
    }
    ticks
}

/// `floor(log10(|r|))`, computed from the decimal representation (no transcendentals).
fn order_of_magnitude(r: &Real) -> i64 {
    if r.is_zero() {
        return 0;
    }
    let text = PlotValue::to_plain_decimal(r, 20);
    let text = text.strip_prefix('-').unwrap_or(&text);
    let (int_part, frac_part) = text.split_once('.').unwrap_or((text, ""));
    let trimmed = int_part.trim_start_matches('0');
    if !trimmed.is_empty() {
        return trimmed.len() as i64 - 1;
    }
    let leading_zeros = frac_part.len() - frac_part.trim_start_matches('0').len();
    -(leading_zeros as i64 + 1)
}

/// `10^k` as an exact `Real`.
fn pow10(k: i64) -> Real {
    if k >= 0 {
        real_from_text(&format!("1{}", "0".repeat(k as usize)))
    } else {
        real_from_text(&format!("0.{}1", "0".repeat((-k - 1) as usize)))
    }
}

/// Smallest integer `>=` the value, as an exact `Real`.
fn ceil(value: &Real) -> Real {
    let trunc = truncate_toward_zero(value);
    if value.cmp(&trunc) == std::cmp::Ordering::Greater {
        trunc.add(&Real::from_i64(1))
    } else {
        trunc
    }
}

/// The integer part of the value (truncated toward zero), as an exact `Real`.
fn truncate_toward_zero(value: &Real) -> Real {
    let text = value.to_string();
    let negative = text.starts_with('-');
    let text = text.strip_prefix('-').unwrap_or(&text);
    let int_part = text.split('.').next().unwrap_or(text);
    let int_part = int_part.split('(').next().unwrap_or(int_part);
    if int_part.is_empty() || int_part.chars().all(|c| c == '0') {
        return Real::zero();
    }
    let signed = if negative {
        format!("-{int_part}")
    } else {
        int_part.to_string()
    };
    real_from_text(&signed)
}

/// Exact division; the bound is the reference's computation cap and the divisor is never zero in
/// the renderer (the padded range is at least one).
fn divide(left: &Real, right: &Real) -> Real {
    left.div(right, Real::DEFAULT_COMPUTATION_DECIMAL_PLACES)
        .unwrap_or_else(Real::zero)
}

fn real_from_i64(value: i64) -> Real {
    Real::from_i64(value)
}

fn real_from_text(text: &str) -> Real {
    Real::from_str(text).unwrap_or_else(Real::zero)
}

fn real_from_f64(value: f64) -> Real {
    Real::from_f64(value).unwrap_or_else(Real::zero)
}

/// The reference's `Fmt`: invariant `0.#####`, trailing zeros trimmed, `-0` normalised.
fn fmt(value: f64) -> String {
    if !value.is_finite() {
        return "0".to_string();
    }
    let mut text = format!("{value:.5}");
    if text.contains('.') {
        text = text.trim_end_matches('0').trim_end_matches('.').to_string();
    }
    if text == "-0" { "0".to_string() } else { text }
}

fn format_tick(value: &Real) -> String {
    if value.is_zero() {
        "0".to_string()
    } else {
        PlotValue::to_plain_decimal(value, 20)
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The reference's `PlotValue`: numeric `Value`s to exact `Real`s for plotting.
pub struct PlotValue;

impl PlotValue {
    /// Promotes a Natural/Integer/Real `Value` to `Real` exactly (no double round-trip).
    ///
    /// The refusal is the reference's own: `InvalidOperationException` with
    /// `Cannot convert value of kind '<Kind>' to a number for plotting.`, which the runner
    /// classifies as `InvalidOperation`/`DomainError`/`recoverable`.
    pub fn to_real(value: &Value) -> Result<Real, LangError> {
        match value {
            Value::Natural(nat) => {
                let int = Int::from_decimal(&nat.to_decimal())
                    .ok_or_else(|| PlotValue::refusal(value))?;
                Ok(Real::from_int(&int))
            }
            Value::Integer(int) => Ok(Real::from_int(int)),
            Value::Real(real) | Value::RealClamped(real, _, _) => Ok(real.clone()),
            _ => Err(PlotValue::refusal(value)),
        }
    }

    /// Converts a numeric `Value` to `f64` for rendering (via `to_real`).
    pub fn to_double(value: &Value) -> Result<f64, LangError> {
        PlotValue::to_real(value).map(|real| PlotValue::to_double_real(&real))
    }

    /// Converts a `Real` to `f64`, expanding a periodic block before parsing.
    pub fn to_double_real(value: &Real) -> f64 {
        PlotValue::to_plain_decimal(value, 20)
            .parse::<f64>()
            .unwrap_or(0.0)
    }

    /// A plain decimal string for the value, expanding a periodic block `repeat` times.
    pub fn to_plain_decimal(value: &Real, repeat: usize) -> String {
        let text = value.to_string();
        let Some(open) = text.find('(') else {
            return text;
        };
        let Some(close) = text.find(')') else {
            return text;
        };
        if close <= open {
            return text;
        }
        let period = &text[open + 1..close];
        let prefix = &text[..open];
        let mut out = String::with_capacity(prefix.len() + period.len() * repeat);
        out.push_str(prefix);
        for _ in 0..repeat {
            out.push_str(period);
        }
        out
    }

    fn refusal(value: &Value) -> LangError {
        LangError::plain(format!(
            "Cannot convert value of kind '{}' to a number for plotting.",
            value.kind_name()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lovelace_nat::Nat;

    fn points(values: &[(f64, f64)]) -> PlotSeries {
        let mut series = PlotSeries::new();
        for (x, y) in values {
            series.push(PlotPoint::from_f64(*x, *y));
        }
        series
    }

    fn points_attribute(svg: &str) -> &str {
        let needle = "points=\"";
        let start = svg.find(needle).expect("the SVG carries a polyline") + needle.len();
        let tail = &svg[start..];
        &tail[..tail.find('"').expect("the attribute closes")]
    }

    #[test]
    fn plot_value_given_huge_natural_to_real_preserves_digits_exactly() {
        // PlotTests.PlotValue_GivenHugeNatural_ToRealPreservesDigitsExactly: a 401-digit
        // Natural must not round-trip through a double or a lossy string.
        let digits = format!("1{}", "0".repeat(400));
        let value = Value::Natural(Nat::from_decimal(&digits).expect("401 digits parse"));
        let real = PlotValue::to_real(&value).expect("a Natural promotes exactly");
        assert_eq!(real.to_string(), digits);
    }

    #[test]
    fn plot_value_given_periodic_real_expands_to_double() {
        // PlotTests.PlotValue_GivenPeriodicReal_ExpandsToDouble: Rl.Parse("0.(3)") to double is
        // 1/3 to three decimal places.
        let value = Value::Real(Real::from_str("0.(3)").expect("periodic syntax parses"));
        let value = PlotValue::to_double(&value).expect("a Real converts");
        assert!((value - 1.0 / 3.0).abs() < 0.001, "{value}");
    }

    #[test]
    fn plot_value_refuses_non_numeric_kinds_with_the_reference_message() {
        let error = PlotValue::to_real(&Value::Text("x".to_string())).expect_err("text refuses");
        assert_eq!(error.code, "InvalidOperation");
        assert_eq!(error.category, "DomainError");
        assert_eq!(
            error.message,
            "Cannot convert value of kind 'Text' to a number for plotting."
        );
    }

    #[test]
    fn svg_given_linear_interpolation_emits_polyline() {
        // PlotTests.SvgPlotRenderer_GivenLinearInterpolation_EmitsPolyline: the C# body builds a
        // three-point series with `Interpolation = PlotInterpolation.Linear`, then requires a
        // `<polyline` and no `<path`.
        let series = PlotSeries::with_interpolation(PlotInterpolation::Linear).with_points(
            [(1.0, 1.0), (2.0, 4.0), (3.0, 9.0)]
                .iter()
                .map(|(x, y)| PlotPoint::from_f64(*x, *y)),
        );
        let model = PlotModel::new().with_series(series);
        let svg = SvgPlotRenderer::new().render(&model);
        assert!(svg.contains("<polyline"), "{svg}");
        assert!(!svg.contains("<path"), "{svg}");
        // Linear keeps one coordinate per data point (the C# `AppendPolyline`); the CubicSpline
        // default densely samples the same three points, so this pins the row's branch.
        assert_eq!(points_attribute(&svg).split(' ').count(), 3, "{svg}");
    }

    #[test]
    fn svg_given_cubic_spline_passes_through_data_endpoints() {
        let data = [(1.0, 1.0), (2.0, 4.0), (3.0, 9.0), (4.0, 16.0)];
        let linear = PlotModel::new().with_series(
            PlotSeries::with_interpolation(PlotInterpolation::Linear)
                .with_points(data.iter().map(|(x, y)| PlotPoint::from_f64(*x, *y))),
        );
        let spline = PlotModel::new().with_series(
            PlotSeries::with_interpolation(PlotInterpolation::CubicSpline)
                .with_points(data.iter().map(|(x, y)| PlotPoint::from_f64(*x, *y))),
        );

        let linear_svg = SvgPlotRenderer::new().render(&linear);
        let spline_svg = SvgPlotRenderer::new().render(&spline);
        let linear_points: Vec<&str> = points_attribute(&linear_svg).split(' ').collect();
        let spline_points: Vec<&str> = points_attribute(&spline_svg).split(' ').collect();
        assert_eq!(linear_points.first(), spline_points.first());
        assert_eq!(linear_points.last(), spline_points.last());
    }

    #[test]
    fn svg_given_huge_close_x_values_keeps_points_distinct() {
        // PlotTests.SvgPlotRenderer_GivenHugeCloseXValues_KeepsPointsDistinct: 10^20 and
        // 10^20 + 1 differ by one unit in exact arithmetic, so they cannot collapse through a
        // double rounding at the range-normalisation step.
        let mut series = PlotSeries::new();
        series.push(PlotPoint::new(
            Real::from_str("100000000000000000000").expect("huge x1"),
            Real::zero(),
        ));
        series.push(PlotPoint::new(
            Real::from_str("100000000000000000001").expect("huge x2"),
            Real::from_i64(1),
        ));
        let model = PlotModel::new().with_series(series);
        let svg = SvgPlotRenderer::new().render(&model);

        assert!(!svg.contains("Infinity"), "{svg}");
        assert!(!svg.contains("NaN"), "{svg}");
        let coordinates: Vec<&str> = points_attribute(&svg).split(' ').collect();
        assert_eq!(coordinates.len(), 2, "{svg}");
        let x1: f64 = coordinates[0].split(',').next().unwrap().parse().unwrap();
        let x2: f64 = coordinates[1].split(',').next().unwrap().parse().unwrap();
        assert_ne!(x1, x2, "{svg}");
    }

    #[test]
    fn svg_given_three_points_uses_dense_spline_polyline() {
        let model = PlotModel::new().with_series(points(&[(1.0, 1.0), (2.0, 4.0), (3.0, 9.0)]));
        let svg = SvgPlotRenderer::new().render(&model);
        assert!(svg.contains("<polyline"), "{svg}");
        assert!(!svg.contains("<path"), "{svg}");
        assert!(
            points_attribute(&svg).split(' ').count() > 3,
            "expected a densely sampled spline: {svg}"
        );
    }

    #[test]
    fn renderer_options_size_the_document_and_change_the_palette() {
        let options = RenderOptions {
            width: 320.0,
            height: 200.0,
            series_colors: vec!["#000000".to_string()],
            ..RenderOptions::default()
        };
        let model = PlotModel::new().with_series(points(&[(1.0, 2.0), (2.0, 4.0)]));
        let svg = SvgPlotRenderer::with_options(options).render(&model);
        assert!(svg.contains("width=\"320\" height=\"200\""), "{svg}");
        assert!(svg.contains("stroke=\"#000000\""), "{svg}");
    }

    #[test]
    fn wire_serialisation_publishes_the_model_and_the_runner_plot_block() {
        let mut series = PlotSeries::with_interpolation(PlotInterpolation::Linear);
        series.push(PlotPoint::from_f64(1.0, 2.0));
        let model = PlotModel {
            title: Some("T".to_string()),
            series: vec![series],
        };
        assert_eq!(
            model.to_wire_json().to_string(),
            r#"{"series":[{"interpolation":"Linear","points":[{"x":"1","y":"2"}]}],"title":"T"}"#
        );

        let capture = PlotCapture::new("<svg/>".to_string(), Some("T".to_string()));
        assert_eq!(
            capture.to_wire_json("C:/plots/p.svg").to_string(),
            r#"{"path":"C:/plots/p.svg","svg":"<svg/>","title":"T"}"#
        );
    }
}
