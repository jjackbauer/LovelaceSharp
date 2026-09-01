# Requirements: Lovelace.Suite — Smooth Curve Interpolation for Plots (Cubic Spline)

> Scope: Define the requirements for connecting a plot series' points with a smooth cubic spline instead of straight line segments, so a coarse sample of a smooth function (e.g. `x = 1..10; y = 1 / x^2`) renders as a continuous curve rather than an angular, piecewise-linear polygon. This is a **rendering** concern only: the exact arbitrary-precision `Real` bounds/normalization pipeline (see `Lovelace.Suite.Plot.Precision.md`) is unchanged, and the spline is fitted in **data space** then mapped to pixels through the data points' affine transform.

---

## Background

`SvgPlotRenderer` connects each series' points with a `<polyline>` (`Lovelace.Suite\Plotting.cs`). For a smooth function sampled at a coarse set of x values — the natural input from range literals such as `1..10` — the straight segments between samples visibly depart from the true curve, producing angular artifacts and the appearance of noise. The arbitrary-precision plot-coordinate work (commit `88861bc`) fixed coordinate *accuracy* but does not address *smoothness*: a polyline of exact points is still a polyline.

---

## Target design

Render each series as a **natural cubic spline** `y(x)` fitted in **data space** and densely sampled into an SVG `<polyline>` (≤ ~1px between samples). The curve passes exactly through every data point, is C² smooth (no kinks or curvature wiggles), and each sample is mapped to a pixel through the affine transform derived from the mapped data points — so the data points stay exact and only the shape *between* them is synthesized. Series with fewer than three points, series whose x is not a function, and series explicitly marked `Linear` fall back to a straight `<polyline>` (or a densely sampled parametric Catmull-Rom for non-function x).

---

## Goals and Non-Goals

### Goals

| # | Goal |
|---|---|
| G1 | A series of three or more points renders as a smooth cubic-spline curve by default. |
| G2 | The curve passes through every data point (each is a knot), so the plot still honors the data. |
| G3 | Straight (collinear) data stays straight — the spline introduces no ringing for linear input. |
| G4 | Exact `Real` bounds and normalization are preserved; the spline is fitted in data space and mapped through the data points' affine pixel transform. |
| G5 | Linear interpolation remains available via the public API for scatter-style data. |

### Non-Goals / Deferred

- Shape-preserving (monotone) splines such as PCHIP or Akima — Catmull-Rom may overshoot for strongly non-monotonic data; deferred to a later pass if the need arises.
- On-canvas markers for the data points.
- Adaptive resampling driven by screen-space error.

---

## Requirements

### A. Data-model requirements

- **A1 — Interpolation property.** `PlotSeries` exposes an `Interpolation` property of a new `PlotInterpolation` enum with members `Linear` and `CubicSpline`, defaulting to `CubicSpline`.
- **A2 — Unchanged plot builtin.** The `plot(...)` builtin signature is unchanged; it constructs series with the default (smooth) interpolation.

### B. Rendering requirements

- **B1 — Dense smooth polyline.** A series with ≥3 points and `CubicSpline` interpolation renders as a `<polyline>` sampled finely enough (≤ ~1px between samples) that the curve is visually smooth on any SVG renderer without relying on the renderer to subdivide cubic Bézier commands.
- **B2 — Natural cubic spline.** When the x values are strictly increasing (a single-valued function of x), a natural cubic spline `y(x)` is fitted in data space and densely sampled — it is C² and reproduces polynomials (e.g. `y = x²`) exactly, so it cannot introduce the kinks or curvature wiggles a pixel-space Catmull-Rom spline can. For x that is not a function it falls back to a densely sampled parametric Catmull-Rom.
- **B3 — Linear fallback.** A series with fewer than three points, or a series with `Interpolation = Linear`, renders as the existing `<polyline>`.
- **B4 — Same string boundary.** All emitted coordinates use the existing `Fmt` 5-decimal formatting; no new number formatting is introduced.

### C. Precision and determinism requirements

- **C1 — Exact data mapping preserved.** Bounds, padding, and the data-to-pixel mapping remain exact `Real`; the spline adds no new `double` round-trip to the data points themselves.
- **C2 — Determinism preserved.** Rendering remains pure (no timestamps, random ids, or culture-dependent formatting); the same input yields byte-identical SVG.

---

## Acceptance criteria (concrete regression targets)

1. `plot(1..10, 1 / (1..10 ^ 2))` emits a single dense `<polyline>` (not the raw straight-segment polyline of the input points).
2. A collinear series (e.g. `(1,1),(2,2),(3,3),(4,4)`) renders a straight line — no ringing.
3. A two-point series still renders a `<polyline>`.
4. A series with `Interpolation = Linear` renders a `<polyline>` even with ≥3 points.
5. Existing precision guarantees (no 2⁵³ collapse, no overflow/underflow) are unaffected; `SvgPlotRenderer_GivenHugeCloseXValues_KeepsPointsDistinct` still passes.

---

## Open decisions

1. **Default interpolation** — `CubicSpline` by default (matches "graphs should look smooth") vs. `Linear` by default with an explicit opt-in. Resolved in favour of `CubicSpline`; the property remains public so hosts can opt out.
2. **Spline type** — Catmull-Rom (local, overshoots on anisotropic pixel axes) vs. natural cubic spline (global solve, C², exact for polynomials) vs. PCHIP (monotone, C¹). Resolved in favour of a **natural cubic spline fitted in data space**; a parametric Catmull-Rom is retained only as the fallback for non-function x.
