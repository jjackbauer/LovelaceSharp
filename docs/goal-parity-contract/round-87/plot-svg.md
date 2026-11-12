# Round 87 — the plot SVG: the 326-byte stub is now the reference chart

**Status: fixed and verified.** Descriptor examples **#97** `plot([1, 2, 3])` and **#98**
`plot([1, 2, 3], [1, 4, 9], "title")` now render the reference document **byte for byte**:
14,636 and 14,595 bytes against the oracle's 14,636/14,595, from 326/331 before. The census rows
are:
`Lovelace.Suite.Tests/Lovelace.Suite.Tests.DescriptorExampleTests/Example_Executes#97` and
`Lovelace.Suite.Tests/Lovelace.Suite.Tests.DescriptorExampleTests/Example_Executes#98`.

Measured 2026-11-12 (America/Sao_Paulo) against the pinned oracle
`Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll` and `rust/target/release/lovelace-run.exe`,
both invoked with `--omit-functions --plot-dir <same dir> --plot-file plot.svg` and the example
script, so the returned path (the one field that depends on the invocation) is identical too.

## 1. What the reference emits that the runner did not

Both documents are the reference's `SvgPlotRenderer` output: an 800×600 canvas, the plot-area
frame, axes at the range's zero crossing (the border when the range does not cross zero), `NiceTicks` tick pairs (target 6) on both axes, and one densely sampled spline `<polyline>` in the palette's
first colour. The runner emitted a 640×400 document with a white rect, an optional title text and a
three-point linear polyline — a stub with **no chart furniture at all**. Element by element:

| # | Element | Reference (#97 / #98) | Rust stub before | Rust after |
|---:|---|---|---|---|
| 1 | `<svg>` root | 1 × `width="800" height="600" viewBox="0 0 800 600"` | 1 × 640×400 | same as reference |
| 2 | background `<rect>` | 1 × 800×600 `fill="white"` | 1 × `fill="#ffffff"` | same as reference |
| 3 | title `<text>` | #97: **none** (title absent); #98: 1 × `x="400" y="28" text-anchor="middle" font-size="20"` body `title` | always emitted, `x="40" y="24" font-size="14"`, **empty body for #97** | same as reference |
| 4 | plot-area frame `<rect>` | 1 × `x="70" y="50" width="700" height="495" fill="none" stroke="#cccccc"` | **absent** | present |
| 5 | y-axis `<line>` | 1 × `(70,50)-(70,545) stroke="#333333"` | **absent** | present |
| 6 | x-axis `<line>` | 1 × `(70,545)-(770,545) stroke="#333333"` | **absent** | present |
| 7 | x-tick `<line>` | 5 × at x = 101.81818, 260.90909, 420, 579.09091, 738.18182; y 541…549 | **absent** | present |
| 8 | x-tick `<text>` | 5 × labels `1, 1.5, 2, 2.5, 3` at `y="563" text-anchor="middle" font-size="12"` | **absent** | present |
| 9 | y-tick `<line>` | #97: 5 × at y = 522.5, 410, 297.5, 185, 72.5; #98: 4 × at y = 466.25, 353.75, 241.25, 128.75; x 66…74 | **absent** | present |
| 10 | y-tick `<text>` | #97: 5 × labels `1, 1.5, 2, 2.5, 3`; #98: 4 × labels `2, 4, 6, 8`; `x="62" text-anchor="end" font-size="12"` | **absent** | present |
| 11 | series `<polyline>` | 1 × 639 sampled points, `stroke="#e6194b" stroke-width="2"` | 1 × 3 points, `stroke="#1f6feb" stroke-width="1.5"` | 639 points, `#e6194b` |
| 12 | `<path>` | 0 (the curve is a polyline) | 0 | 0 |

Element counts per document:

| Document | bytes before | lines before | elements before | bytes after | lines after | elements after |
|---|---:|---:|---|---:|---:|---|
| #97 | 326 | 5 | svg 1, rect 1, text 1, polyline 1 | 14,636 | 27 | svg 1, rect 2, line 12, text 10, polyline 1 |
| #98 | 331 | 5 | svg 1, rect 1, text 1, polyline 1 | 14,595 | 26 | svg 1, rect 2, line 11, text 10, polyline 1 |

(`line` differs by one because #98's y range [1, 9] asks `NiceTicks` for four ticks where #97's
[1, 3] asks for five. `text` is 10 in both: five x labels plus five y labels for #97; #98 spends
one of its ten on the title and one fewer on the y axis.)

The chart geometry itself: first/last series point `101.81818,522.5` / `738.18182,72.5` in both
examples, i.e. the 5%-padded exact-`Real` bounds map to the 700×495 frame.

## 2. The cause

The reference's `Interpreter.BuiltinPlot` (Interpreter.cs:2043-2121) builds a `PlotModel` from the
**exact** `PlotValue.ToReal` coordinates and renders it with `SvgPlotRenderer` (Plotting.cs:84).
The runner's `plot()` builtin (builtins.rs) instead kept its own 640×400 pipeline: it parsed each
coordinate through the value's *display string* into `f64`, laid the series out inside a 40 px
margin, and printed one `<polyline>`. The faithful renderer already existed in Rust — `lovelace-run/src/plot_surface.rs`
(round 70, a member-for-member port of Plotting.cs) — but it was **additive and unreachable**: it
lived in the binary crate, and the builtin that must call it lives in `lovelace-lang`, which cannot
depend on `lovelace-run`. Two renderers existed; `plot()` used the wrong one.

## 3. The fix

1. `rust/lovelace-lang/src/plot_surface.rs` — the surface moved into the language crate (the
   reference-faithful model, `PlotValue`, `RenderOptions`, `SvgPlotRenderer`, wire serialisation).
   The code is the round-70 port, unchanged except for the crate-local `use crate::{LangError, Value}`
   and the header that now documents the builtin as its consumer.
2. `rust/lovelace-lang/src/builtins.rs` — `builtins_plot` now builds `PlotSeries`/`PlotModel` from
   `PlotValue::to_real` (exact `Real`, no display-string round-trip), sets the model title to
   `None` for an absent title (the renderer emits no title text for it, like `string.IsNullOrEmpty`),
   and renders with `SvgPlotRenderer::new()`. The arity, vector, length, empty-vector, title-kind and
   non-numeric-element refusals are unchanged, in the reference's order. The now-dead `to_f64`,
   `plot_coordinate` and `escape_xml` helpers were removed.
3. `rust/lovelace-run/src/plot_surface.rs` — re-exports the language crate's surface and keeps the
   nine ported `PlotTests` unit tests in place, so the census ids
   `lovelace-run::plot_surface::tests::…` still resolve to the test that exercises the same code.
4. `rust/lovelace-run/src/main.rs`, `rust/lovelace-lang/src/lib.rs` — module wiring and its comment.

Nothing else moved: no C# source, fixture, corpus file, parity ledger or desktop file was edited.

## 4. Equivalence evidence

* **SVG bytes:** `#97` 14,636 = 14,636 and `#98` 14,595 = 14,595 (SHA-256 equal; release binary,
  the same `--plot-dir` for both runners).
* **Envelope:** with both runners writing to the same directory, the normalised envelopes
  (revision, elapsed, elapsedTime, timings replaced at any depth — the differential's own rule)
  are **identical, 0 differences**, including `$.plot.svg`, `$.plot.title` (`""`/`"title"`),
  `$.artifacts[0].bytes` (14636/14595) and `$.artifacts[0].path`.
* **Sweep beyond the two examples (17 scripts):** 16 are byte-identical to the oracle as well —
  `plot([1])`, `plot([1, 2])`, `plot([3, 2, 1])` (parametric fallback), `plot([-1, 0, 1])`,
  `plot([1, 2, 3], [9, 4, 1], "descending")`, `plot(1..10, 1/(1..10^2), "1/x^2")` (the corpus
  example, periodic reals), `plot([0.1, 0.2], [0.3, 0.4])`,
  `plot([100000, 200000, 300000], [1, 2, 3])`, `plot([1, 1, 1])`,
  `plot(1..5, [2, 4, 16, 256, 65536])`, `plot([-5, -4, -3], [-1, 0, 1])`,
  `plot(1..2, [1, 4], "t<&>")` (title escaping), `plot([10^400])`, `plot([1/3, 2/3, 1])`,
  `plot([-1000000, 0, 1000000], [1, 2, 3])`, `plot([1, 2, 3], [1, 2, 3], "")`.
* **The one known residual:** `plot([10^400, 10^400+1, 10^400+2])` (y overflows `f64`) diverges in
  one token per point: the reference's `Fmt` prints `NaN`, the port prints `0`. That is the
  port's deliberate hardening — `svg_given_huge_close_x_values_keeps_points_distinct` requires no
  `NaN`/`Infinity` in the document — so it is kept, not fixed here.

## 5. Tests added

`rust/lovelace-lang/src/plot_structure_round87.rs` (2 tests, module wired in `lib.rs`):

| Test | Pins |
|---|---|
| `descriptor_example_097_plot_single_vector_publishes_the_reference_chart` | 800×600 root + viewBox, white rect, frame `(70,50,700×495) #cccccc`, y axis `(70,50)-(70,545)`, x axis `(70,545)-(770,545)`; counts `svg 1 / rect 2 / line 12 / text 10 / polyline 1`; no 20 px title text; the first and last x-tick lines (`101.81818`, `738.18182`); label order `1, 1.5, 2, 2.5, 3, 1, 1.5, 2, 2.5, 3`; series `#e6194b` width 2 with 639 points and endpoints `101.81818,522.5` / `738.18182,72.5`; document > 10 kB. |
| `descriptor_example_098_plot_two_vectors_and_title_publishes_the_reference_chart` | the same canvas/frame/axes; counts `svg 1 / rect 2 / line 11 / text 10 / polyline 1`; the title text `x="400" y="28" … font-size="20">title`; the y-tick line at `y="466.25"`; label order `title, 1, 1.5, 2, 2.5, 3, 2, 4, 6, 8`; the same 639-point series endpoints; document > 10 kB. |

The pre-existing plot tests stay green: `lovelace-run::plot_surface::tests::*` (9), the
`round19_batch`/`suite_engine_batch`/`suite_parity_batch` plot rows (polyline-not-path, title
reaches the SVG, collinearity, determinism, the writable/refusal rows) and
`plot_surface_wire::a_writable_plot_file_is_still_written_after_the_refusals`.

## 6. Gates (measured on this tree)

| Gate | Command | Result |
|---|---|---|
| workspace tests | `cargo test --workspace` (rust/Cargo.toml) | **PASS** — 5,365 passed, 0 failed, 1 ignored, exit 0. Two of the tests are this round's; the rest of the delta from the stated 5,345 baseline is the concurrent desktop lane's additions (its binary target grew 174 → 186 between two consecutive runs here) in `ui/editor.rs`/`ui/syntax.rs`, which this round did not touch. No test was removed and no existing test regressed. |
| corpus | `cargo run --release -p lovelace-corpus` | **PASS** — `corpus: 214 passed, 0 failed, 214 total` |
| fixtures | `node parity/tools/gate-fixtures.mjs` | **PASS** — `oracle 20/20, rust 20/20` |
| CLI | `node parity/tools/gate-cli.mjs` | **PASS** — `CLI matrix: 16/16`. RUN-012 compares placement/naming and deliberately ignores `$.plot.svg` and `$.artifacts.0.bytes`; both ignored fields are now equal to the oracle as well. |
| differential (generated) | `node parity/diff/run.mjs --cases 200` | **PASS** — `counts {match: 200, mismatch: 0, error: 0}`, `harnessExit 0` |
| differential (replayed) | same run: `regressionCounts {match: 1758, mismatch: 0}`; dedicated `--regressions`: `1873 discovered, 1873 replayed, 1873 match, 0 stillMismatching` | **PASS** |
| clippy | `cargo clippy -p lovelace-lang -p lovelace-run --all-targets -- -D warnings` | **PASS** — exit 0, no warnings |
| fmt | `rustfmt --edition 2024 --check` over the crate (via `lib.rs`) and `lovelace-run/src/plot_surface.rs` | **PASS** — exit 0. `cargo fmt --all --check` still reports diffs under `rust/lovelace-desktop/src` (the concurrent lane's files; out of this round's scope). |

**Parity ledger note (handoff, not touched here):** editing Rust sources changes
`parity/rust-tests.json`'s `sourcesSha256`, so `parity/tools/gate-manifest.mjs` will report
`rust-tests.json is stale` until `parity/gen-rust-tests.ps1` is re-run. `parity/` is out of this
round's scope by instruction, so the inventory was deliberately **not** regenerated. No manifest row
changed: every existing `lovelace-run::plot_surface::tests::…` id still resolves (the tests moved
nowhere), and the two new tests need no census row (descriptor #97/#98 already map to
`lovelace-lang::suite_engine_batch::descriptor_examples_execute`, which remains valid).

## 7. What remains

* The `NaN` residual of §4 (the reference prints `NaN` for a coordinate that overflows `f64`; the
  port prints `0` by the round-70 hardening). No published example or corpus row reaches it.
* `parity/gen-rust-tests.ps1` + `gate-manifest.mjs` re-run by the ledger owner (§6 note).
* Nothing else: both examples are byte-identical, and the runner and the oracle now share one
  renderer implementation.
