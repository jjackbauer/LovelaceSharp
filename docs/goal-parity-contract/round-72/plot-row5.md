# Round 72 — row 5: the Linear case now exercises Linear

**Status: complete.** The row-5 test now builds its series with `PlotInterpolation::Linear`, as
the C# body does, and pins the resulting three-coordinate polyline. The Linear case matches the
C# oracle, so `SvgPlotRenderer_GivenLinearInterpolation_EmitsPolyline#0` can stay mapped. Only
the Rust test and this report changed.

Measured 2026-11-12 (America/Sao_Paulo).

## 1. What the C# row asserts

`Lovelace.Suite.Tests/PlotTests.cs:173-186`:

```csharp
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
```

Three points — `(1,1)`, `(2,4)`, `(3,9)` — with **explicit** `Linear`; the rendered document
must contain a `<polyline` and no `<path`. The renderer contract behind the row is
`Lovelace.Suite/Plotting.cs:175-176`:

```csharp
if (series.Points.Count < 3 || series.Interpolation == PlotInterpolation.Linear)
    AppendPolyline(sb, series, MapX, MapY, stroke);
else
    AppendSmoothCurve(sb, series, MapX, MapY, stroke);
```

`AppendPolyline` (`Plotting.cs:186-205`) appends exactly one `Fmt(x),Fmt(y)` pair per data
point, so the row's oracle output is a **three-coordinate** polyline. The old Rust test never
entered that branch.

## 2. Before / after

Before (`rust/lovelace-run/src/plot_surface.rs`, test at line 908):

```rust
#[test]
fn svg_given_linear_interpolation_emits_polyline() {
    let model = PlotModel::new().with_series(points(&[(1.0, 1.0), (2.0, 4.0), (3.0, 9.0)]));
    let svg = SvgPlotRenderer::new().render(&model);
    assert!(svg.contains("<polyline"), "{svg}");
    assert!(!svg.contains("<path"), "{svg}");
}
```

`points()` builds `PlotSeries::new()` (`plot_surface.rs:863-869`), whose interpolation is the
default `CubicSpline` (`:48-55`, `:76-80`). With three points that takes
`append_smooth_curve` (`:390-394`), which also emits a `<polyline` and no `<path`, so both
assertions passed without the Linear branch. This is exactly the round-71 §2.3 caveat: the test
exercised the neighbouring CubicSpline path.

After (test at line 908, only the setup and one added assertion differ):

```rust
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
```

The coordinate-count assertion is the C# `AppendPolyline` contract made observable: it fails if
the renderer ever routes this series back through dense sampling. The sibling
`svg_given_three_points_uses_dense_spline_polyline` (same three points, default CubicSpline)
already asserts `count() > 3`, which is what the old row-5 setup actually exercised. No other
test, helper or production line changed. The test name is unchanged, so the census id and the map
target still resolve.

## 3. Does Linear match the oracle?

**Yes — no divergence, and no assertion was weakened or adjusted to fit the implementation.**

The Linear series renders as a single `<polyline`, contains no `<path`, and carries exactly
three coordinates, one per data point. That is the same output shape the C# row asserts and the
same count `AppendPolyline` writes (`Plotting.cs:186-205`). The focused run exercised the
`series.interpolation == PlotInterpolation::Linear` disjunct at `plot_surface.rs:390`, which the
old test never reached.

## 4. Gates

| Gate | Command | Observed | Exit |
|---|---|---|---:|
| Focused | `cargo test -p lovelace-run plot_surface` | `14 passed; 0 failed; 0 ignored; 397 filtered out` | 0 |
| Workspace | `cargo test --workspace` | `5,160 passed; 0 failed; 1 ignored` — 5,161 total across 21 result blocks | 0 |
| Corpus | `cargo run --release -p lovelace-corpus` | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| Fixtures | `node parity/tools/gate-fixtures.mjs` | `oracle: 20/20`, `rust: 20/20` | 0 |
| CLI | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16` | 0 |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | no diagnostics; Finished dev profile | 0 |
| Format | `cargo fmt --all -- --check` | clean | 0 |

The one ignored test is the known
`lovelace-corpus::property_assertions::mathir_evaluates_like_symbolic`
(`rust/lovelace-corpus/src/property_assertions.rs:305-307`), the same ignored row as the
round-71 baseline; the suite total stays 5,161.

## 5. Mapping verdict and out-of-scope follow-ups

Row 5 **stays mapped**:

```text
Lovelace.Suite.Tests/Lovelace.Suite.Tests.PlotTests/SvgPlotRenderer_GivenLinearInterpolation_EmitsPolyline#0
  -> lovelace-run::plot_surface::tests::svg_given_linear_interpolation_emits_polyline
```

The round-71 §2.3 qualification is resolved: the test now sets the row's interpolation and
observes the Linear shortcut, so it is no longer passing through the CubicSpline neighbour.

Two follow-ups are outside this round's permitted scope (`.cs`, fixtures, `parity/`, other docs):

1. `parity/manifest-map.json` still carries the round-71 note saying the test uses the default
   `CubicSpline`; that note is now stale and should be refreshed when the map is next touched.
2. `parity/rust-tests.json`'s `sourcesSha256` covers every `.rs` file
   (`parity/gen-rust-tests.ps1:48-65`), so this edit makes the inventory stale even though the
   test count and name are unchanged. Run `parity/gen-rust-tests.ps1` before the manifest gate;
   no parity file was edited here.

No corpus or fixture bytes moved (the corpus is 214/214 and the fixtures 20/20 above).

## 6. Scope and reproduction

Changed files: `rust/lovelace-run/src/plot_surface.rs` (the `svg_given_linear_interpolation_emits_polyline`
test only) and this report. No `.cs` file, fixture, `parity/` file or other document was edited.

```powershell
$repo = 'C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME = "$repo\.toolchain\rustup"
$env:CARGO_HOME  = "$repo\.toolchain\cargo"
$env:PATH = "$repo\.toolchain\cargo\bin;" + $env:PATH
$env:TMP = "$repo\rust\target\tmp"; $env:TEMP = $env:TMP
Set-Location "$repo\rust"

cargo test -p lovelace-run plot_surface
cargo test --workspace
cargo run --release -p lovelace-corpus
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
Set-Location $repo
node parity/tools/gate-fixtures.mjs
node parity/tools/gate-cli.mjs
```
