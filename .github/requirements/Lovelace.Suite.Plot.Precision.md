# Requirements: Lovelace.Suite — Arbitrary-Precision Plot Coordinates (Bounds + Normalization in Real)

> Scope: Define the requirements for improving plotting precision by keeping plot coordinates in Lovelace's arbitrary-precision `Real` type through bounds computation and normalization, and dropping to `double` only for the final `[0,1] → pixel` multiply. This closes the genuine data-loss cases in the current `double`-based pipeline (values beyond 2⁵³, values separated by less than one ULP, and magnitudes beyond `double`'s range) without rewriting the SVG renderer in arbitrary precision. This is a **requirements document for review — no implementation yet**.

---

## Background

Today the plotting pipeline converts every value to `double` before any geometry is computed:

- `PlotPoint` is `record struct PlotPoint(double X, double Y)` (`Lovelace.Suite\Plotting.cs`).
- `PlotValue.ToDouble` promotes each `Value` by `ToString()` + `double.Parse`, expanding periodic `Real` notation 20 times (`Lovelace.Suite\Plotting.cs:245-268`).
- `BuiltinPlot` builds `PlotPoint`s directly from the x/y vectors (`Lovelace.Suite\Interpreter.cs:833-901`).
- `SvgPlotRenderer` computes bounds, padding, and linear mapping entirely in `double` (`ComputeBounds`, `PadBounds`, `MapX`, `MapY`, `NiceTicks`).

This loses precision in four concrete ways:

1. **Integers/Naturals beyond 2⁵³ ≈ 9.0×10¹⁵** cannot round-trip through `double`. Distinct values (e.g. `10²⁰` and `10²⁰+1`) collapse to the same `double`, merging points and erasing the slope between them.
2. **Reals with more than ~17 significant digits** are truncated (the `Real` type stores up to `MaxComputationDecimalPlaces` = 1000 fractional digits).
3. **Periodic reals** are approximated (period expanded only 20 times).
4. **Out-of-range magnitudes** (e.g. a 400-digit `Natural`) parse to `double.PositiveInfinity`, breaking bounds and ticks.

The SVG writer additionally quantizes coordinates to 5 decimals (`Fmt = "0.#####"`) and tick labels to 4 decimals (`FormatTick = "0.####"`) in an 800×600 viewBox.

---

## Target design

Promote every numeric `Value` to `Real` **exactly** (via `Value.Widen(ValueKind.Real)`, no string/double round-trip), carry points as `Real` coordinates, compute `min`/`max`/padding/`range` in `Real`, then compute the normalized fraction `fx = (x − minX) / (maxX − minX)` in `Real`. Only **then** convert that bounded `[0,1]` fraction to `double` and multiply by the pixel width/height. The SVG string quantization stays as-is.

---

## Goals and Non-Goals

### Goals

| # | Goal |
|---|---|
| G1 | Keep plot coordinates in `Real` through bounds, padding, and normalization. |
| G2 | Compute the normalized `[0,1]` fraction exactly in `Real` before any `double` is involved. |
| G3 | Eliminate the 2⁵³ collapse, overflow-to-∞, and underflow-to-0 failure modes. |
| G4 | Preserve byte-identical SVG output for ordinary in-range data (backward compatibility). |

### Non-Goals / Deferred

- Arbitrary-precision SVG text (5 decimals is already sub-pixel for the 800×600 viewBox).
- Adding `Log`/`Exp`/`Pow` to `Real` beyond what tick generation strictly needs.
- Changing number *computation* semantics (`Sqrt`, `Pi`, division) — only the plotting pipeline.
- PNG export, interactive/zoomable plots, and multi-series legend styling.

---

## Requirements

### A. Data-model requirements

- **A1 — Exact point storage.** `PlotPoint` shall carry `Real` X/Y (not `double`). Decide explicitly whether to change the existing public `record struct PlotPoint(double X, double Y)` or add a parallel `RealPlotPoint`; the migration must be deliberate because `PlotModel`/`IPlotRenderer`/`PlotCapture` are public and consumed by `Lovelace.Studio.EngineHost` and the tests.
- **A2 — Exact value promotion.** Replace `PlotValue.ToDouble` with `PlotValue.ToReal(Value)` returning `value.Widen(ValueKind.Real).AsReal()`. This must be exact for all three numeric kinds:
  - `Natural → Real` via `new Real(new Int(natural))`
  - `Integer → Real` via `new Real(integer)`
  - `Real → Real` (identity)

  No `ToString()` + `double.Parse`, no `Math.*`, no 20-repetition period expansion at this stage.
- **A3 — Renderer model parity.** `PlotModel`/`PlotSeries` must expose the same structure (title, multiple series, point ordering) so the existing `<polyline>` path, 5-color palette, and title rendering are unchanged.

### B. Bounds, padding, and normalization (the precision core)

- **B1 — Exact bounds.** Compute `minX/maxX/minY/maxY` across all series using `Real.CompareTo` (`<`, `>`), never via `double`. The empty-plot fallback shall be the exact `Real` values `(0, 1, 0, 1)`.
- **B2 — Exact padding.** Reimplement `PadBounds` in `Real`: 5% of `range` on each side, or `±Real.One` when `min == max`.
- **B3 — Exact range.** `rangeX = maxX − minX` and `rangeY = maxY − minY` must be exact `Real` subtractions.
- **B4 — Exact normalization.** For every point and every tick:
  - `fx = (x − minX) / rangeX`
  - `fy = (y − minY) / rangeY` (exact `Real` division — period detection makes rational results exact)

  The Y fraction is then inverted for top-origin mapping (`sy = 1 − fy`).
- **B5 — Single, bounded lossy step.** Convert only the resulting `[0,1]` fraction to `double` for the pixel multiply (`left + fx × width`). Everything upstream of this step shall be exact.

### C. Precision acceptance requirements

- **P1 — No 2⁵³ collapse.** Distinct inputs that differ by at least one pixel's worth of the padded range must render as distinct coordinates. Regression target: `plot([10^20, 10^20+1], [0,1])` currently collapses to one column; it must render two distinct columns.
- **P2 — No overflow to ∞.** Values with magnitude beyond `double.MaxValue` (~1.8e308, e.g. a 400-digit `Natural`) must produce finite bounds and valid SVG (no `NaN`/`Infinity` leaking from `double.Parse`).
- **P3 — No underflow to 0.** Values below double's denormal floor (~4.9e-324) must survive to the normalization stage.
- **P4 — Fraction error far below one pixel.** With an 800×600 viewBox, one pixel ≈ 1/800 of the range (≈1.25e-3 in fraction units). The final `double` conversion of a `[0,1]` fraction must have error `≤ 1e-9`, i.e. orders of magnitude below one pixel. This is the measurable "better precision" guarantee.
- **P5 — Periodic fractions exact.** The `Real → double` step at the fraction stage must expand periodic notation (e.g. `0.(3)`) before parsing, since `double.Parse` cannot consume `"0.(3)"`. Reuse the existing expansion logic, now applied only to the bounded `[0,1]` fraction.

### D. Tick-generation requirements

- **D1 — Ticks in `Real`.** `NiceTicks` shall operate on `Real` bounds and produce exact `Real` tick values (e.g. `2`, `0.5`, `-1`), not doubles.
- **D2 — Remove `Math.Log10`/`Math.Pow`.** Either:
  1. add an exact `Real` logarithm/power for tick stepping, **or**
  2. replace the "nice step" logic with a decimal digit-count algorithm over the `Real` bounds' decimal representation (recommended — no new transcendental machinery).
- **D3 — Exact tick labels.** Tick labels shall be formatted from the exact `Real` value (invariant culture), not from a `double`. Keep the axis-crossing-at-zero behavior (`min ≤ 0 ≤ max` → axis through zero) unchanged.

### E. Backward-compatibility & determinism requirements

- **E1 — Identical output for ordinary data.** For in-range, human-scale data (|value| < 2⁵³, non-pathological ranges), the rendered SVG shall remain byte-identical to today's output (modulo any intentional tick improvements). Existing `Lovelace.Suite.Tests\PlotTests.cs` and `SuiteEngineOutputPlotTests.cs` must still pass.
- **E2 — Determinism preserved.** Same inputs → byte-identical SVG, invariant culture, no timestamps/randomness. The `SvgPlotRenderer` already guarantees this; the `Real` path must not reintroduce nondeterminism.
- **E3 — Output boundary unchanged.** `Fmt` (`"0.#####"`) and `FormatTick` (`"0.####"`) may remain the final string boundary; they are already sub-pixel for the 800×600 viewBox, so increasing SVG string precision is **not** required.

### F. Performance / non-functional requirements

- **F1 — O(points) with no per-point `ToString()`.** Bounds and normalization shall not round-trip through strings in the hot loop; use `Real` arithmetic operators directly.
- **F2 — Bounded normalization precision.** Exact `Real.Divide` runs long division up to `MaxComputationDecimalPlaces` (1000) when no period is found — too expensive per point for large vectors. Introduce a dedicated plot-normalization precision (e.g. ~20–30 significant digits, still ≫ pixel resolution) and apply it via a local precision scope around `B4`'s division. This likely requires exposing the existing internal `Real.WithLocalPrecision` or an equivalent public/internals-visible API.
- **F3 — Public contract stability.** The `plot(...)` builtin signature, `IPlotRenderer.Render(PlotModel)`, `PlotCapture`, and `EngineHost`'s consumption of `PlotCapture.Svg` shall not change unless the migration is deliberately coordinated with `Lovelace.Studio`.

---

## Acceptance criteria (concrete regression targets)

1. `plot([10^20, 10^20 + 1], [0, 1], "huge")` → two distinct x columns; today it collapses to one.
2. Plotting a 400-digit `Natural` → finite bounds, valid SVG, no `Infinity`.
3. `plot([0, 0.1, 0.2], [0, 1e-400, 2e-400])` → three distinct y positions; today the tiny y values collapse toward 0.
4. Existing plot tests pass byte-identical for ordinary data.
5. Two runs of the same script produce byte-identical SVG.

---

## Open decisions

1. **`PlotPoint` API migration** — mutate the existing public record to `Real` fields, or add a parallel type to preserve the current `double`-based public surface (A1)?
2. **Tick algorithm** — decimal digit-count based (recommended, no new `Real` transcendental functions) vs. adding `Real.Log10`/`Pow` (D2)?
