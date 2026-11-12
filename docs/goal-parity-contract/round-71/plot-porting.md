# Round 71 — plot census: nine mappings and a truthful baseline

**Status: complete.** The nine rows the round-70 plot lane unblocked — the five
`PlotValue`/`SvgPlotRenderer` rows and the four `plot-wire` rows — are now `ported` and resolve
**9/9** in a freshly measured inventory. Coverage moved **5,938 / 6,209 (95.64 %)** ->
**5,947 / 6,209 (95.78 %)**; deferred 153 -> 144; exclusions unchanged at 118.

The inherited baseline was untruthful: `parity/rust-tests.json` was generated at
`2026-11-12T11:15:37Z`, before the fourteen round-70 plot tests existed, so the gate reported
`rust-tests.json is stale`. The first action was to regenerate the inventory, then the manifest;
only then was the map edited. This is a mapping round: `parity/manifest-map.json` gained exactly
nine dispositions and `parity/manifest.toml`, `parity/rust-tests.json` and
`parity/coverage.md` were regenerated. No Rust source, `.cs` file, fixture, corpus file or other
document was edited.

Measured 2026-11-12 (America/Sao_Paulo) with the round-70 lane work uncommitted in the tree.

## 1. Regeneration first

Pre-regeneration gate, before any file was changed:

```text
rust-tests.json is stale: the Rust sources changed since the inventory was measured (run parity/gen-rust-tests.ps1)
FAIL: 1 issue(s)   (exit 1)
```

It still printed the inherited counts: ported 5,938, deferred 153, excluded 118,
coveragePercent 95.64, claimedRustTests 5,938, projectsInCensus 15, with that one stale failure.
So 5,938 / 6,209 (95.64 %) is the pre-regen line, and it was not a measured baseline: the
inventory listed 5,147 tests (5,146 passed, 1 ignored; `sourcesSha256`
`6d5bec3ed075569a6a7f8d235e34455c0040d36a48d7e2adab3794e05fa33599`) and predated the 14 new
tests.

| Step | Command | Observed | Exit |
|---|---|---|---:|
| Rust inventory + suite | `powershell -ExecutionPolicy Bypass -File parity/gen-rust-tests.ps1` | `crates=11 tests=5161 suiteExit=0 passed=5160 failed=0` | 0 |
| Manifest generation (pre-mapping) | `node parity/gen-manifest.mjs` | `rows 6209`, `ported 5938`, `deferred 153`, `excluded 118`, `coveragePercent 95.64`, `rustTests 5161` | 0 |
| Manifest gate (pre-mapping) | `node parity/tools/gate-manifest.mjs` | `"claimedRustTests": 5938`, `"failures": []`; `PASS (structural): 6209 test cases dispositioned; 5938 claim Rust coverage (95.64%).` | 0 |

The regenerated inventory lists **5,161 tests** (5,160 passed, 0 failed, 1 ignored — the known
`property_assertions::mathir_evaluates_like_symbolic`), with `generatedAtUtc`
`2026-11-12T11:37:49Z` and `sourcesSha256`
`ac15d493be07300f65cc32abc40552af3189c9afa6f7ec10ebd1379f957ede9f`. The +14 tests are exactly
the round-70 work: nine in `rust/lovelace-run/src/plot_surface.rs` and five in
`rust/lovelace-run/src/plot_surface_wire.rs`; `lovelace-run` now carries 412 tests. The gate
re-verifies the new `sourcesSha256` against the Rust tree.

## 2. Which Rust test carries each row

I read all fourteen new tests, the nine census bodies
(`Lovelace.Suite.Tests/PlotTests.cs:92,102,113,173,189`,
`Lovelace.Run.Tests/PlotDirectoryErrorTests.cs:61`,
`Lovelace.Run.Tests/PlotFileErrorTests.cs:31,39`,
`Lovelace.Symbolics.Tests/CapabilitiesBuiltinTests.cs:621`), and the nearest older plot tests
(`round19_batch` plot family, `round19_run_batch` plot rows,
`plot_value_refuses_non_numeric_kinds_with_the_reference_message`). Each mapping below repeats the
row's distinguishing assertions, not merely a neighbouring case.

### 2.1 Five surface rows — all in `lovelace-run::plot_surface::tests`

| # | Census row | Chosen Rust test | Why it carries the row |
|---|---|---|---|
| 1 | `PlotValue_GivenHugeNatural_ToRealPreservesDigitsExactly#0` | `plot_value_given_huge_natural_to_real_preserves_digits_exactly` | The C# body builds a 401-digit `Nat`, calls `PlotValue.ToReal`, and requires the exact 401-character decimal string. The Rust test builds the same 401 digits, calls `PlotValue::to_real`, and asserts `real.to_string() == digits`; no double/string round-trip. |
| 2 | `PlotValue_GivenPeriodicReal_ExpandsToDouble#0` | `plot_value_given_periodic_real_expands_to_double` | The C# body parses `Rl.Parse("0.(3)")`, calls `PlotValue.ToDouble`, and requires `1/3` to 3 places. The Rust test parses `Real::from_str("0.(3)")`, calls `to_double`, and asserts `|d - 1/3| < 0.001`. |
| 3 | `SvgPlotRenderer_GivenCubicSpline_PassesThroughDataEndpoints#0` | `svg_given_cubic_spline_passes_through_data_endpoints` | The C# body renders a Linear and a CubicSpline model over the same four points and requires equal first and last polyline coordinates. The Rust test constructs exactly those two series, renders both, and compares the first/last `points` tokens. |
| 4 | `SvgPlotRenderer_GivenHugeCloseXValues_KeepsPointsDistinct#0` | `svg_given_huge_close_x_values_keeps_points_distinct` | The C# body plots `10^20` and `10^20 + 1`, requires no `Infinity`/`NaN`, exactly two coordinates, and unequal x pixels. The Rust test uses the same two exact Reals and asserts the same four facts. |
| 5 | `SvgPlotRenderer_GivenLinearInterpolation_EmitsPolyline#0` | `svg_given_linear_interpolation_emits_polyline` | The C# body sets `Interpolation = Linear` on a three-point series and requires `<polyline` and no `<path`. The Rust test asserts the same two output facts through the same `SvgPlotRenderer::render` entry point. **Qualification:** its series is `PlotSeries::new()`, whose documented default is `CubicSpline`, so the Linear shortcut itself is not entered here; that shortcut is exercised as the Linear baseline inside the row-3 test. See 2.3. |

Rejected as neighbouring, not mapped: `svg_given_three_points_uses_dense_spline_polyline` (dense
spline sampling only — the row-70 sibling of the C# `GivenThreePoints` row),
`renderer_options_size_the_document_and_change_the_palette` (width/palette only),
`wire_serialisation_publishes_the_model_and_the_runner_plot_block` (serialisation only), and
`plot_value_refuses_non_numeric_kinds_with_the_reference_message` (the `ToReal` refusal message
only).

### 2.2 Four wire rows — all in `lovelace-run::plot_surface_wire`

The four wire rows assert the runner's JSON envelope, so their tests live in `lovelace-run`, not
`lovelace-lang`. The refusals are produced by `lovelace-lang::builtins_plot` and the two
advertisements by the `lovelace-lang` capability table, but no `lovelace-lang` test asserts the
envelope (exit code, `code`, `category`, `recoverable`, message and diagnostics). Mapping these
rows to a lang test would exercise only the neighbouring engine path.

| # | Census row | Chosen Rust test | Why it carries the row |
|---|---|---|---|
| 6 | `EmptyPlotDirectory_IsAnErrorEnvelopeWithExitOne#0` | `empty_plot_directory_is_an_error_envelope_with_exit_one` | The C# body runs `--plot-dir ""` and requires exit 1, `protocolVersion` 1, `ok:false`, `PlotDirectoryError`/`TypeMismatch`, recoverable, the path in the message, and `diagnostics`/`elapsed`/`elapsedTime.value`/`elapsedTime.unit`/`timings`. The Rust test drives `run_args` with the same argv and asserts every one. |
| 7 | `EmptyPlotFile_IsAnErrorEnvelopeWithExitOne#0` | `empty_plot_file_is_an_error_envelope_with_exit_one` | The C# body runs `--plot-file ""` and requires exit 1, `PlotFileError`/`TypeMismatch`, recoverable, the caller path in the envelope message, and a non-null `diagnostics`. The Rust test asserts the same against the plot directory itself and additionally pins the single diagnostic's message to that path. |
| 8 | `PlotFileUnderAMissingDirectory_IsAnErrorEnvelopeWithExitOne#0` | `plot_file_under_a_missing_directory_is_an_error_envelope_with_exit_one` | The C# body runs `--plot-file nope/x.svg` and requires the same envelope, the caller path in the message, and a non-null `diagnostics`. The Rust test asserts the same platform join in the envelope and the diagnostic, and additionally that `nope` was not created. |
| 9 | `SymbolicPlot_IsATypedRefusal_NeverAnInternalInvariantFailure#0` | `symbolic_plot_is_a_typed_refusal` | The C# body drives `plot(sin(x))`, `plot(1)` and `plot([x, 1, 2])`, requires `InvalidOperation`/`DomainError` (never `InternalInvariantFailure`), recoverable, and equality with the live `plot.symbolic-expression` advertisement. The Rust test drives the same three scripts, asserts the exact messages, checks both advertisements (`plot.symbolic-expression` and `plot.symbolic-element`), and asserts the envelope through `run_args`. |

Rejected as neighbouring, not mapped:
`round19_run_batch::run_plot_directory_naming_an_existing_file_is_an_error_envelope_with_exit_one`
and `...under_an_existing_file...` (different `--plot-dir` values — an existing file, not the empty
one), `round19_run_batch::run_writable_plot_file_is_still_written` (success path, a different
row), and the three deferred `PlotFileWriteRefusalTests` rows (library-type surface, no Rust type
yet).

### 2.3 Qualification on row 5

The row-5 test is the only new test named for the linear row, and it asserts the row's documented
output contract (`<polyline`, no `<path`). However, its model is
`PlotModel::new().with_series(points(...))`, and `points()` uses `PlotSeries::new()`, whose default
is `CubicSpline` (see `plot_surface.rs:48-55,76-80`). The C# body explicitly sets
`Interpolation = PlotInterpolation.Linear`. The `Linear` shortcut is executed by the Linear
baseline in `svg_given_cubic_spline_passes_through_data_endpoints` (row 3), but row 5's own test
would still pass if the Linear branch were removed. I recorded this in the map note rather than
silently claiming a stronger port. No Rust edit was in scope for this round; one line
(`PlotSeries::with_interpolation(PlotInterpolation::Linear)`) would make row 5 exact. This is the
only qualification among the nine mappings.

Rows 7 and 8 assert the row's distinguishing fields and the caller-path diagnostic but not the
shared envelope shape (`protocolVersion`, `elapsed`, `timings`); row 6's test asserts that shape
over the same `run_args` path.

## 3. The nine mappings

The map was re-read from disk immediately before the write. All nine targets were still
`deferred` with the round-45 reasons; none had a `rust` value. Only those nine entries changed.

* Before: 3,636,491 bytes, 6,209 keys, SHA-256
  `1deb62d924a4198f9f0cdeb88b24a3b118b4f184c7a54b23dface8e09f54db4f`.
* After: 3,639,006 bytes, 6,209 keys, SHA-256
  `5c8d4e9cacde37bbfcbc1150a1436e1ced57b23edaa1b46e4413dc286ff45f99`.
* A parsed per-key comparison against the reconstructed pre-write bytes reports exactly 9 changed
  keys (`deferred` -> `ported` with a `rust` value), 0 added keys, 0 removed keys, 0 changed keys
  outside the nine targets. The reconstructed pre-write bytes and hash equal the round-70
  after-state (`1deb62d9...`, 3,636,491 bytes), so no inherited work was touched.

Each entry now has the shape:

```json
"Lovelace.Run.Tests/Lovelace.Run.Tests.PlotFileErrorTests/PlotFileUnderAMissingDirectory_IsAnErrorEnvelopeWithExitOne#0": {
  "disposition": "ported",
  "rust": "lovelace-run::plot_surface_wire::plot_file_under_a_missing_directory_is_an_error_envelope_with_exit_one",
  "reason": "Round 71 mapping: the row's documented behaviour is reproduced by the named Rust test in the measured inventory (see note). Round 70 implementation: plot_surface.rs and the builtins_plot error paths.",
  "note": "Round 70: ... lovelace-run plot_surface_wire::plot_file_under_a_missing_directory_is_an_error_envelope_with_exit_one asserts the same and that the missing parent is not created."
}
```

The nine id -> test pairs:

```text
PlotTests/PlotValue_GivenHugeNatural_ToRealPreservesDigitsExactly#0
  -> lovelace-run::plot_surface::tests::plot_value_given_huge_natural_to_real_preserves_digits_exactly
PlotTests/PlotValue_GivenPeriodicReal_ExpandsToDouble#0
  -> lovelace-run::plot_surface::tests::plot_value_given_periodic_real_expands_to_double
PlotTests/SvgPlotRenderer_GivenCubicSpline_PassesThroughDataEndpoints#0
  -> lovelace-run::plot_surface::tests::svg_given_cubic_spline_passes_through_data_endpoints
PlotTests/SvgPlotRenderer_GivenHugeCloseXValues_KeepsPointsDistinct#0
  -> lovelace-run::plot_surface::tests::svg_given_huge_close_x_values_keeps_points_distinct
PlotTests/SvgPlotRenderer_GivenLinearInterpolation_EmitsPolyline#0
  -> lovelace-run::plot_surface::tests::svg_given_linear_interpolation_emits_polyline
PlotDirectoryErrorTests/EmptyPlotDirectory_IsAnErrorEnvelopeWithExitOne#0
  -> lovelace-run::plot_surface_wire::empty_plot_directory_is_an_error_envelope_with_exit_one
PlotFileErrorTests/EmptyPlotFile_IsAnErrorEnvelopeWithExitOne#0
  -> lovelace-run::plot_surface_wire::empty_plot_file_is_an_error_envelope_with_exit_one
PlotFileErrorTests/PlotFileUnderAMissingDirectory_IsAnErrorEnvelopeWithExitOne#0
  -> lovelace-run::plot_surface_wire::plot_file_under_a_missing_directory_is_an_error_envelope_with_exit_one
CapabilitiesBuiltinTests/SymbolicPlot_IsATypedRefusal_NeverAnInternalInvariantFailure#0
  -> lovelace-run::plot_surface_wire::symbolic_plot_is_a_typed_refusal
```

(The rows above are prefixed by their owning test project; the complete census ids are the
`Lovelace.Suite.Tests/`, `Lovelace.Run.Tests/` and `Lovelace.Symbolics.Tests/` paths from the
sections 2.1 and 2.2 tables.)

## 4. Resolution: 9/9

A per-mapping check read the census, the curated map, the generated manifest and the measured
inventory, and required all of: the id occurs exactly once in the census; the map entry is
`ported` with the chosen `rust`; the manifest row is `ported` with the same `rust`; the test name
exists in `parity/rust-tests.json`; and the measured suite exit is 0.

```text
resolved 9 / 9
unresolved []
suite exit=0 passed=5160 failed=0 total=5161
```

Focused run:

```text
cargo test -p lovelace-run plot_surface
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 397 filtered out; finished in 0.01s
cargoExit=0
```

## 5. Final coverage and gate

| Step | Command | Observed | Exit |
|---|---|---|---:|
| Manifest generation | `node parity/gen-manifest.mjs` | `rows 6209`, `ported 5947`, `deferred 144`, `excluded 118`, `coveragePercent 95.78`, `rustTests 5161` | 0 |
| Manifest gate | `node parity/tools/gate-manifest.mjs` | `"claimedRustTests": 5947`, `"failures": []`; `PASS (structural): 6209 test cases dispositioned; 5947 claim Rust coverage (95.78%).` | 0 |
| Coverage report | `node parity/tools/coverage-report.mjs` | `coverage.md written: 5947/6209 ported (95.78%)` | 0 |

```json
{
  "censusCases": 6209,
  "manifestRows": 6209,
  "dispositions": { "ported": 5947, "consolidated": 0, "excluded": 118, "deferred": 144 },
  "coveragePercent": 95.78,
  "claimedRustTests": 5947,
  "projectsInCensus": 15,
  "failures": []
}
```

| Project | Before | After | Deferred |
|---|---|---|---|
| Lovelace.Suite.Tests | 891 / 929 (95.91 %), 38 deferred | **896 / 929 (96.45 %)**, 33 deferred | -5 |
| Lovelace.Run.Tests | 736 / 751 (98.00 %), 15 deferred | **739 / 751 (98.40 %)**, 12 deferred | -3 |
| Lovelace.Symbolics.Tests | 1,146 / 1,226 (93.47 %), 80 deferred | **1,147 / 1,226 (93.56 %)**, 79 deferred | -1 |
| **TOTAL** | **5,938 / 6,209 (95.64 %)**, 153 deferred | **5,947 / 6,209 (95.78 %)**, 144 deferred | **-9** |

Final `parity/coverage.md` total line:

```text
| **TOTAL** | **6209** | **5947** | **144** | **118** | **95.78 %** | |
```

**Unresolved ids: `[]`.** All nine rows resolve to a test that exists in the fresh inventory and
whose suite run recorded exit 0. The row-5 qualification in 2.3 is a test-strength caveat, not a
resolution failure: the mapped test exists, runs, and asserts the row's documented output.

## 6. Scope and reproduction

Changed files: `parity/manifest-map.json`, `parity/manifest.toml`, `parity/coverage.md`,
`parity/rust-tests.json`, and this report. No `rust/` source, `.cs` file, fixture, corpus file or
other document was edited; the inherited uncommitted round-70 Rust work is untouched.

```powershell
$repo = 'C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME = "$repo\.toolchain\rustup"
$env:CARGO_HOME  = "$repo\.toolchain\cargo"
$env:PATH = "$repo\.toolchain\cargo\bin;" + $env:PATH
$env:TMP = "$repo\rust\target\tmp"; $env:TEMP = $env:TMP
Set-Location $repo

node parity/tools/gate-manifest.mjs                     # pre-regen: stale rust-tests.json, exit 1
powershell -ExecutionPolicy Bypass -File parity/gen-rust-tests.ps1  # crates=11 tests=5161 suiteExit=0 passed=5160 failed=0
node parity/gen-manifest.mjs                           # pre-mapping: 5938 ported (95.64%)
node parity/tools/gate-manifest.mjs                    # pre-mapping: PASS
# map edit: nine deferred -> ported entries, re-read immediately before the write
node parity/gen-manifest.mjs                           # 5947 ported (95.78%)
node parity/tools/gate-manifest.mjs                    # PASS
node parity/tools/coverage-report.mjs                  # coverage.md written: 5947/6209 ported (95.78%)
Set-Location "$repo\rust"; cargo test -p lovelace-run plot_surface
```

## 7. Handoff finding

The one item for the next plot round is row 5: add
`PlotSeries::with_interpolation(PlotInterpolation::Linear)` to
`svg_given_linear_interpolation_emits_polyline` (or assert the Linear branch directly) so the
test's setup matches the C# body's. Until then the map note records the difference; the row is
counted as ported on the same basis as the pre-existing `GivenThreePoints` mapping, whose note
also records a partial shape-only repeat.
