# Round 70 — plot surface (PlotValue / SvgPlotRenderer) and plot wire

**Lane:** plot-surface. **Status:** complete — the five `PlotValue`/`SvgPlotRenderer` rows and
the four `plot-wire` rows are unblocked; every guard is green.

This document was written surface-first: section 1 and the id table were written from the pinned C#
sources and the nine census test bodies **before** any Rust code was added. Sections 2–6 record the
implementation and the observed gates.

## 1. Observed C# surface (read first)

Source: `Lovelace.Suite/Plotting.cs` (603 lines), `Lovelace.Suite/Interpreter.cs:2043-2143`
(`BuiltinPlot`), `Lovelace.Suite/EngineExceptions.cs:20-42` (`PlotFileWriteException`), and the
plot rows of `Lovelace.Suite.Tests/PlotTests.cs`, `Lovelace.Run.Tests/PlotDirectoryErrorTests.cs`,
`Lovelace.Run.Tests/PlotFileErrorTests.cs`, `Lovelace.Symbolics.Tests/CapabilitiesBuiltinTests.cs`.

### 1.1 Model and renderer members

| C# member | Shape / behaviour (verbatim from the source) |
|---|---|
| `readonly record struct PlotPoint(Rl X, Rl Y)` | Holds both coordinates in arbitrary-precision `Rl` so bounds, padding and normalisation stay exact until the final pixel conversion. Extra ctor `PlotPoint(double x, double y)`. |
| `enum PlotInterpolation { Linear, CubicSpline }` | How successive points are connected. |
| `sealed class PlotSeries` | `List<PlotPoint> Points` (get-only list) and `PlotInterpolation Interpolation` defaulting to **CubicSpline**. |
| `sealed class PlotModel` | `string? Title` and `List<PlotSeries> Series` (get-only list). |
| `sealed record PlotCapture(string? Svg, string? Title)` | Host-side capture of the last rendered plot. |
| `interface IPlotRenderer` | `string Render(PlotModel model)`. |
| `sealed class SvgPlotRenderer : IPlotRenderer` | Deterministic, self-contained SVG. Constants: width **800**, height **600**, margins left **70**, right **30**, top **50**, bottom **55**. Bounds/padding/normalisation in `Rl`; one double conversion at `[0,1] → pixel`. Output: `<svg xmlns=… width height viewBox>`, white `<rect>`, optional centred 20px title, plot border `<rect>`, zero-crossing axes (else the border), x/y ticks from `NiceTicks(min,max,6)`, then one `<polyline>` per series (series colours `#e6194b,#3cb44b,#4363d8,#f58231,#911eb4`). `Fmt` is invariant `"0.#####"` with `-0 → 0`. Title escapes `& < > "`. |
| spline rule | `Points.Count < 3` or `Linear` ⇒ direct polyline. Else, when x is strictly increasing, a **natural cubic spline** in data space mapped back through the affine data→pixel transform and emitted as a dense polyline (`samples = max(200, ceil(|px_last-px_first|)+1)`); non-increasing x falls back to a densely sampled parametric Catmull-Rom. Never `<path>`. |
| `static class PlotValue` | `Rl ToReal(Value)`: Natural → `new Rl(new Int(nat))`, Integer → `new Rl(int)`, Real → the stored Real, else `InvalidOperationException("Cannot convert value of kind '<Kind>' to a number for plotting.")`. `double ToDouble(Value)`: `ToDouble(ToReal(value))`. `double ToDouble(Rl)`: `double.Parse(ToPlainDecimal(value))`. `string ToPlainDecimal(Rl value, int repeat = 20)`: expands a `0.(3)` period `repeat` times (default 20), otherwise the value's own string. |

### 1.2 Wire shape the runner publishes

- Success: the envelope carries `plot: { "path": <absolute path>, "title": <string, "" when none>,
  "svg": <the same document written to the file> }`; `artifacts` carries
  `{ "kind": "plot", "path": <path>, "bytes": <file length>[, "title"] }`; the `result` is
  `Text` naming the path.
- `plot()` argument contract (`Interpreter.BuiltinPlot`): 1–3 args; every vector argument must be
  `ValueKind.Vector` (**kind check first**), 2/3-arg forms must have equal lengths, empty refuses;
  a 3rd argument must be `Text`; each element goes through `PlotValue.ToReal`. Refusals are
  `InvalidOperationException` ⇒ runner code `InvalidOperation`, category `DomainError`,
  recoverable, exit 1. Exact messages:
  `plot() argument 1 must be a vector, but got 'Symbolic'.`,
  `plot() argument 2 must be a vector, but got '<Kind>'.`,
  `plot() title must be a string, but got '<Kind>'.`,
  `plot() expects 1 to 3 arguments, but got <n>.`,
  `plot() vectors must have the same length (<a> vs <b>).`,
  `plot() cannot plot an empty vector.`,
  `Cannot convert value of kind '<Kind>' to a number for plotting.`
- Write failure: `PlotFileWriteException(path, cause)` ⇒ `PlotFileError` / `TypeMismatch` /
  recoverable / exit 1, message `Cannot write the plot file '<caller path>': <cause>`.
  `plot()` writes `Path.GetFullPath(Path.Combine(PlotOutputDirectory, PlotFileName))` and does
  **not** create a missing parent; the runner pre-creates only `PlotOutputDirectory`
  (`Directory.CreateDirectory`), and an empty/uncreatable directory is
  `PlotDirectoryError` / `TypeMismatch` / recoverable / exit 1 with
  `Cannot use plot directory '<dir>': <cause>`.
- Capability: `capabilities()` advertises `plot.symbolic-expression` with code
  `InvalidOperation`, category `DomainError` and the live message
  `plot() argument 1 must be a vector, but got 'Symbolic'.`; `plot.symbolic-element` with
  `Cannot convert value of kind 'Symbolic' to a number for plotting.`.

## 2. Implementation

### 2.1 Additive surface — `rust/lovelace-run/src/plot_surface.rs`

The reference members map one-for-one; two Rust-shaped additions are called out.

| C# member | Rust equivalent |
|---|---|
| `PlotPoint(Rl X, Rl Y)` | `plot_surface::PlotPoint { pub x: Real, pub y: Real }`, `new`, `from_f64` |
| `PlotInterpolation.Linear/CubicSpline` | `plot_surface::PlotInterpolation::{Linear, CubicSpline}` (`Default = CubicSpline`) + `wire_name` |
| `PlotSeries` | `plot_surface::PlotSeries { pub points, pub interpolation }`, `new`, `with_interpolation`, `push`, `with_points` |
| `PlotModel` | `plot_surface::PlotModel { pub title: Option<String>, pub series }`, `new`, `with_series` |
| `PlotCapture(Svg, Title)` | `plot_surface::PlotCapture { pub svg: String, pub title: Option<String> }`, `new` |
| `IPlotRenderer.Render` | `SvgPlotRenderer::render(&PlotModel) -> String` |
| `SvgPlotRenderer` constants | **addition:** explicit `RenderOptions { width, height, margin_left/right/top/bottom, samples, series_colors }`; `SvgPlotRenderer::new()` uses the reference's 800×600 / 70-30-50-55 / 200-sample / five-colour defaults, `with_options` overrides them |
| `PlotValue.ToReal/ToDouble/ToPlainDecimal` | `PlotValue::{to_real, to_double, to_double_real, to_plain_decimal}`; `to_real` returns `Result<Real, LangError>` with the reference message, so the runner classifies a bad kind as `InvalidOperation`/`DomainError` |
| `PlotCapture` on the wire | **addition:** `PlotModel::to_wire_json()` and `PlotCapture::to_wire_json(path)`; the latter is byte-for-byte the runner's `plot` block (`path`, `title`, `svg`) |

The renderer ports `ComputeBounds`, `PadBounds`, `MapX/MapY` (exact `Real` normalisation, one
`f64` conversion), `NiceTicks`, `OrderOfMagnitude`, `Pow10`, `Ceil/TruncateTowardZero`,
`Fmt`, `FormatTick`, `Escape`, `AppendPolyline`, `AppendNaturalSpline`, `SplineValue`,
`AppendParametricSpline` and `CatmullRom`. It is additive: the runner's `plot()` builtin keeps its
own SVG pipeline, so no corpus/fixture byte moves.

### 2.2 Engine error paths — `rust/lovelace-lang/src/builtins.rs`, `builtins_plot` only

The parent authorised this minimal engine change (error paths only); successful SVG bytes are
unchanged and proven below.

- Argument gates ported from `BuiltinPlot`: `plot_vector` (`Value::Vector` only, reference
  messages), `plot_coordinate` (Natural/Integer/Real/RealClamped only, reference message), the
  title-`Text` check, the length message, and the empty-vector message. The index vector is now
  1-based, matching `BuildIndexVector`.
- The write no longer calls `create_dir_all(parent)`: the reference writes exactly the combined
  path, so a missing parent refuses. The runner's `--plot-dir` pre-creation is unchanged.
- The write error is `Cannot write the plot file '<dir joined with file name>': <cause>`, using the
  caller's path spelling (before `GetFullPath`), as `PlotFileWriteException` does.

### 2.3 Wire preflight — `rust/lovelace-run/src/main.rs` (+ the same mirror in `parity_batch`)

`--plot-dir ""` now refuses before the engine with `PlotDirectoryError`/`TypeMismatch`/
recoverable and the reference's empty-path sentence; previously `create_dir_all("")` succeeded and
the run exited 0. The in-process test harness mirrors it so `run_args` and `main` cannot diverge.

### 2.4 Guard: successful SVG bytes are identical

Captured from the pre-change release binary and the post-change release binary, byte-compared:

| probe script | SHA-256 (before = after) |
|---|---|
| `plot([4, 9, 16])` | `d44d8d48cd264da2d6a0dbd89ccdfedfbdb23c0e2d65b45fdf8a8c12ba9bc403` |
| `plot(1..10, 1 / (1..10 ^ 2), "1/x^2")` (the corpus row) | `24f880d2bdcf42d5c5a487511add01ef838a01fa2de8aadc725d8b7d5e1928b1` |
| `plot(1..3, [1, 4, 9], "My Plot")` | `ad204dd1408874962a41cc46d5f9e7e1f8a3a6cc3f2695320d61b2739a9754be` |

### 2.5 Exact before / after of the three error-path behaviours

| behaviour | before (release binary 00:27) | after |
|---|---|---|
| `x = symbol("x"); plot(sin(x))` | exit **0**, `ok:true`, SVG written to `...\dir\plot.svg` (295 B) | exit **1**, `code:InvalidOperation`, `category:DomainError`, `recoverable:true`, message `plot() argument 1 must be a vector, but got 'Symbolic'.` |
| `plot([1, 2, 3])` with `--plot-file nope/x.svg` | exit **0**, `ok:true`, auto-created `...\dir\nope\` and wrote `x.svg` (326 B) | exit **1**, `code:PlotFileError`, `category:TypeMismatch`, message `Cannot write the plot file '...\dir\nope/x.svg': The system cannot find the path specified. (os error 3)`; `nope` **not** created |
| `plot([1, 2, 3])` with `--plot-file ""` | exit **1**, `PlotFileError`, message `Cannot write plot: The system cannot find the path specified. (os error 3).` (no caller path) | exit **1**, `PlotFileError`, message `Cannot write the plot file '...\dir\': The system cannot find the path specified. (os error 3)` (names the caller path) |
| `--eval 1 --plot-dir ""` | exit **0**, `ok:true` | exit **1**, `code:PlotDirectoryError`, `category:TypeMismatch`, message `Cannot use plot directory '': The value cannot be an empty string. (Parameter 'path')` |

## 3. Tests and the nine census ids

Fourteen new tests: nine in `plot_surface.rs` (the five rows plus four surface/wire-shape tests) and
five in `plot_surface_wire.rs` (the four rows plus a success-path guard). Each repeats the C# case's
own assertion with concrete values.

| # | Census id | Rust test |
|---|---|---|
| 1 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.PlotTests/PlotValue_GivenHugeNatural_ToRealPreservesDigitsExactly#0` | `plot_surface::tests::plot_value_given_huge_natural_to_real_preserves_digits_exactly` |
| 2 | `…/PlotValue_GivenPeriodicReal_ExpandsToDouble#0` | `plot_surface::tests::plot_value_given_periodic_real_expands_to_double` |
| 3 | `…/SvgPlotRenderer_GivenCubicSpline_PassesThroughDataEndpoints#0` | `plot_surface::tests::svg_given_cubic_spline_passes_through_data_endpoints` |
| 4 | `…/SvgPlotRenderer_GivenHugeCloseXValues_KeepsPointsDistinct#0` | `plot_surface::tests::svg_given_huge_close_x_values_keeps_points_distinct` |
| 5 | `…/SvgPlotRenderer_GivenLinearInterpolation_EmitsPolyline#0` | `plot_surface::tests::svg_given_linear_interpolation_emits_polyline` |
| 6 | `Lovelace.Run.Tests/Lovelace.Run.Tests.PlotDirectoryErrorTests/EmptyPlotDirectory_IsAnErrorEnvelopeWithExitOne#0` | `plot_surface_wire::empty_plot_directory_is_an_error_envelope_with_exit_one` |
| 7 | `Lovelace.Run.Tests/Lovelace.Run.Tests.PlotFileErrorTests/EmptyPlotFile_IsAnErrorEnvelopeWithExitOne#0` | `plot_surface_wire::empty_plot_file_is_an_error_envelope_with_exit_one` |
| 8 | `Lovelace.Run.Tests/Lovelace.Run.Tests.PlotFileErrorTests/PlotFileUnderAMissingDirectory_IsAnErrorEnvelopeWithExitOne#0` | `plot_surface_wire::plot_file_under_a_missing_directory_is_an_error_envelope_with_exit_one` |
| 9 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.CapabilitiesBuiltinTests/SymbolicPlot_IsATypedRefusal_NeverAnInternalInvariantFailure#0` | `plot_surface_wire::symbolic_plot_is_a_typed_refusal` |

The inventory id is `lovelace-run::` + the path in the table (e.g.
`lovelace-run::plot_surface::tests::plot_value_given_huge_natural_to_real_preserves_digits_exactly`);
`cargo test -p lovelace-run -- --list` confirms all fourteen names.

Supporting tests (not census rows): `plot_value_refuses_non_numeric_kinds_with_the_reference_message`,
`svg_given_three_points_uses_dense_spline_polyline`, `renderer_options_size_the_document_and_change_the_palette`,
`wire_serialisation_publishes_the_model_and_the_runner_plot_block`, and
`a_writable_plot_file_is_still_written_after_the_refusals`.

Row 9 is checked exactly as the C# body does it: the three live refusals (`plot(sin(x))`,
`plot(1)`, `plot([x,1,2])`) plus equality of code/category/message against the live
`capabilities()` entry for `plot.symbolic-expression` and `plot.symbolic-element`.

## 4. Gates (observed this round)

Run on 2026-11-12 (America/Sao_Paulo) with the pinned toolchain, after the production edits.

| Gate | Command | Observed line | Exit |
|---|---|---|---:|
| Corpus | `cargo run --release -p lovelace-corpus` | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| Fixtures | `node parity/tools/gate-fixtures.mjs` | `oracle: 20/20`; `rust: 20/20`; `PASS: oracle 20/20, rust 20/20` | 0 |
| CLI matrix | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16`; `PASS: every RUN-001..RUN-014 row behaves like the oracle` | 0 |
| Differential | `node parity/diff/run.mjs --cases 200 --seed 1` | `GENERATED 200 200 0 0`; `REPLAYED 1758 1758 0 0`; `regressions: discovered=1873 replayed=1758 stillMismatching=0 resolved=1758 new=0` | 0 |
| Workspace suite | `cargo test --workspace` | 21 binaries: **5,160 passed, 0 failed, 1 ignored** | 0 |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | clean (`Finished dev profile`) | 0 |
| Formatting | `cargo fmt --all -- --check` | clean (no diff) | 0 |

Per-test confirmation: `cargo test -p lovelace-run plot_surface` — **14 passed, 0 failed**
(the nine surface tests plus the five wire tests), and
`cargo test -p lovelace-lang plot` — **17 passed, 0 failed** (the pre-existing plot suite,
unchanged by the error-path edits).

## 5. Blockers

**None for the nine rows.** All four `plot-wire` rows now pass their C# assertions and the five
`PlotValue`/`SvgPlotRenderer` rows have direct Rust equivalents.

Adjacent deferred rows deliberately not touched (different APIs, outside this block):
`PlotFileWriteRefusalTests` (3 rows, needs a public `PlotFileWriteException`-shaped library type and
`SuiteEngine.PlotFileName` refusal surface) and `SuiteEngineOutputPlotTests` (2 rows, needs the
output writer / `ResetPlotCapture` API).
