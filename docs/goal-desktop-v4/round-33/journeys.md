# Round 33 — five mandatory journeys recorded and reproducible

Status: **5/5 PASS**. `--journey <ID>` runs each journey's script through the shipped workbench and
the same `ExecutionHost` path the window owns; `--journey J-15` additionally drives the same `Plot`
pane the window paints. Each run prints the observed values and exits non-zero if the expected
observable in [`../journeys.md`](../journeys.md) does not match. J-02 and J-15 are also proven in
the real window (captures + UIA dumps below).

All code lives in `rust/lovelace-desktop/**`; all evidence is in this folder. This round edited no
engine crate, `tools/` file, gate file or other journey row. (The working tree also contains
unrelated round-98 parity work in `rust/lovelace-lang`/`rust/lovelace-symbolics`; it is not part of
this deliverable.)

## 1. What changed

| File | Change |
|---|---|
| [`src/journey.rs`](../../../rust/lovelace-desktop/src/journey.rs) | New: `--journey` mode, the five journey functions, the per-check printer, SHA-256 for the artifact hashes, and the `Options` parser. It uses only `Workbench` (the window's state), the engine plot capture and the real `Plot` pane; it never evaluates source itself. |
| [`src/main.rs`](../../../rust/lovelace-desktop/src/main.rs) | `--journey` dispatch (Windows), a documented non-Windows refusal, and the usage text. |
| [`src/engine.rs`](../../../rust/lovelace-desktop/src/engine.rs) | WKS-004: `Evaluator::remove_variable` (selective seam) and the provided `clear_variables`; tests on the real engine and the fake engine. |
| [`src/engine/real.rs`](../../../rust/lovelace-desktop/src/engine/real.rs) | `remove_variable` delegates to `lovelace_lang::Engine::remove_variable`; `clear()` is now Clear Variables (removes globals, keeps the function registry) instead of rebuilding the engine. |
| [`src/engine/fake.rs`](../../../rust/lovelace-desktop/src/engine/fake.rs) | The fake implements the same selective seam and bumps its revision. |
| [`src/workbench.rs`](../../../rust/lovelace-desktop/src/workbench.rs) | `committed_results` (per-statement display values, used by J-03), `reused` (reported reuse count), `clear_variables`/`clear_variables_active` (WKS-004), and a workspace test for the clear. |
| [`src/ui/app.rs`](../../../rust/lovelace-desktop/src/ui/app.rs) | WKS-004 affordance: `Clear Variables` beside the variables table, disabled while a run is in flight or before the first workspace; it clears through `Workbench::clear_variables_active`. |
| [`src/gui_selftest.rs`](../../../rust/lovelace-desktop/src/gui_selftest.rs) | `plot_element_counts` is `pub(crate)` so J-15 reuses the exact census the GUI selftest pins. |

The existing `--gui-selftest` suite is unchanged in behaviour: 19/19 checks pass (below).

## 2. Per-journey results

`journey-run.log` is the unedited stdout of:

```powershell
$repo='C:\Users\ricar\dev\LovelaceSharp'
$env:APPDATA="$repo\docs\goal-desktop-v4\round-33\appdata"
$env:TMP="$repo\rust\target-desktop\tmp"; $env:TEMP=$env:TMP
Set-Location "$repo\docs\goal-desktop-v4\round-33\work"
& "$repo\rust\target-desktop\debug\lovelace-desktop.exe" --journey J-02 --journey J-03 `
    --journey J-09 --journey J-10 --journey J-15 --out "$repo\docs\goal-desktop-v4\round-33\journey-out"
# JOURNEY summary journeys=5 failed=0 ; exit 0
```

| # | Input | Expected (journeys.md / REF) | Observed (round 33) | Artifacts | Result |
|---|---|---|---|---|---|
| J-02 | `a = 2` / `b = a^10` / `b` | result `1024`; variables `a`,`b` with kinds; log line; cursor | result `1024`; `_=1024 (Natural), a=2 (Natural), b=1024 (Natural)`; revision 1, reused 0, committed 3/3, precision 1000; log `Run started: 3 statements … [result] = 1024 … Run finished: 3 statements committed`; cursor `Ln 1, Col 1`, `run Finished` | [journey-run.log](journey-run.log) (lines `JOURNEY J-02 *`); window proof [J-02.png](J-02.png), [J-02-uia.txt](J-02-uia.txt) | **PASS** |
| J-03 | `setprecision(20)`; `sqrt(2)`; `setprecision(1000)`; `sqrt(2)`; then precision 1000 + rerun | full stored digits at each precision; visible precision; precision change invalidates the cache | at 20: `1.4142135623730950488` (19 fractional digits, trailing zero trimmed); at 1000: 1000 fractional digits, `sha256=92fa8c84…36b63c` (the REF-digits value byte for byte); visible precision 20 then 1000; run 2 reused **0** and re-rendered 1000 digits | [journey-run.log](journey-run.log) (`JOURNEY J-03 *`) | **PASS** |
| J-09 | `print("alpha")` / `plot(1..10, (1..10)^2, "side effects")` / `print("beta")`; run twice | each run executes its effects; no duplicate print line within a run | each run's output is exactly `["alpha", "beta"]` (one of each); stdout log lines 2 after run 1 and 4 after run 2; each run wrote a fresh engine plot (15 104 bytes, `sha256=6a80a443…82de35`) whose file equals the capture | [journey-run.log](journey-run.log); [J-09-run1.svg](journey-out/J-09-run1.svg), [J-09-run2.svg](journey-out/J-09-run2.svg) | **PASS** |
| J-10 | `func f(x) = x + 1` / `y = 41` / `f(y)`; then Clear Variables | `y` removed, `f` retained | before: result `42`, variables `_=42, y=41`, `f(x) origin=user builtin=false`, 124 functions; after clear: **removed 2**, 0 variables in both projections, `f(x) origin=user builtin=false`, **124 functions**, result display unchanged, log `Variables cleared: 2 removed; 124 functions retained.` | [journey-run.log](journey-run.log) (`JOURNEY J-10 *`); engine-seam tests in `cargo-test.log` | **PASS** |
| J-15 | `plot(1..10, (1..10)^2)` | engine SVG bytes; zoom/pan/fit; SVG export | SVG 15 000 bytes, census **2 rect / 13 line / 11 text / 1 polyline**, `sha256=0b23d6f7…1406c` — byte-identical to the `REF-plot` SVG; pane parses 800×600; Fit = zoom 1/pan 0; one zoom step = 1.25 (123.0% → 153.8%); pan (-40, 30) accepted and Fit restores identity; raster 800×600 @1x and 1600×1200 @2x; exported SVG 15 000 bytes, byte-identical to the capture and to the engine's own file | [journey-run.log](journey-run.log); [J-15-export.svg](journey-out/J-15-export.svg), [J-15-plot-2x.png](journey-out/J-15-plot-2x.png); window proof [J-15.png](J-15.png), [J-15-zoom.png](J-15-zoom.png), [J-15-window-plot.svg](J-15-window-plot.svg), [J-15-uia.txt](J-15-uia.txt), [J-15-zoom-uia.txt](J-15-zoom-uia.txt) | **PASS** |

Honest notes:

- **J-03 / reused = 0.** The execution core has no incremental cache, so every `StepCommitted`
  carries `reused: false` and the counter is 0 for both runs. The journey therefore asserts what
  the product can actually claim: the second run re-rendered the 1000-digit value (same digest as
  REF-digits) after the precision control moved from 20 to 1000, and reported 0 reused statements.
  The check is not weakened into "a run happened"; a cached 20-place value at that point would have
  failed the 1000-digit assertion.
- **J-10 / where the clear acts.** One engine is constructed per run (the execution contract in
  `execution.rs`), so after a run the only surviving workspace is the committed
  `WorkspaceSnapshot`. Clear Variables removes the globals from both variable projections of that
  workspace and leaves the function table untouched — the same outcome as the reference's
  `DELETE /api/state` (`REF-clear`: variables empty, `f` present). The engine-level half of WKS-004
  is separately tested on a live `RealEngine`: `clear_variables` removes every global through
  `Engine::remove_variable`, the retained `func f` still evaluates (`f(1) = 2`), and the cleared
  `y` no longer resolves. Both tests are in `cargo-test.log`
  (`clear_variables_removes_globals_and_keeps_the_real_user_function`,
  `clear_variables_removes_globals_and_keeps_the_user_function_in_the_workspace`).
- The `_` variable is the engine's answer global; the reference publishes it too (`REF-basic`,
  `REF-precision`), so it is listed rather than hidden. The J-02 assertion is that `a` and `b`
  exist with the right displays and kinds.

## 3. Window proof (J-02 and J-15)

The console session was **locked** during the capture (foreground `Windows Default Lock Screen`),
so the proof uses only input paths that reach a locked window: the app is launched on the journey's
script file, buttons are pressed with **UIA Invoke** (AccessKit), and pixels come from
**PrintWindow(PW_RENDERFULLCONTENT)** into a 1600×1000 bitmap. No injected keyboard or pointer input
is used. The full check list is [window-proof.json](window-proof.json) (28 checks, 28 PASS) and the
driver log is [capture-journeys.log](capture-journeys.log).

| Journey | Capture | Bytes | SHA-256 | What it shows |
|---|---|---:|---|---|
| J-02 | [J-02.png](J-02.png) | 219 426 | `72573493CF8888CDBF06766FEDAC14398931BA435EDC76254F179E733D061094` | after Run: Result tree `display: 1024`, variables `_ a b` with `Natural` kinds and values `1024 / 2 / 1024`, log `[result] = 1024`, status `rev 1 … Ln 1, Col 1 … run Finished` |
| J-15 | [J-15.png](J-15.png) | 239 651 | `97A9107782C1E66D279C89D80D48B6B35E58BC2CBBE76BB608C891122BA520C3` | the rendered engine plot in the real plot pane after Run; UIA: `SVG plot canvas, 800 by 600 points, zoom 1.00, 76%`, `raster 604x453 @ 0.75x … ms raster` |
| J-15 zoom | [J-15-zoom.png](J-15-zoom.png) | 234 301 | `A6279EC9EE346119B92AAA777CF66890578F5B01B5414FBC9798DEC8DDDE057D` | after one UIA-invoked `Zoom in`: UIA `zoom 1.25, 94%` and `raster 756x567 @ 0.94x` (the pane re-rasterised, not only re-scaled a texture) |

UIA details that make the captures checkable:

- J-02 [J-02-uia.txt](J-02-uia.txt) (591 nodes): the status strip
  `J-02.ls | rev 1 | precision 1000 | Ln 1, Col 1 | run Finished`; `[result] = 1024`;
  `[progress] statement 3/3: b`; `VARIABLES · 3` and the row sequence
  `_ · Natural · 1024 | a · Natural · 2 | b · Natural · 1024`.
- J-15 [J-15-uia.txt](J-15-uia.txt) (583 nodes): the image widget's accessible name
  `SVG plot canvas, 800 by 600 points, zoom 1.00, 76%`; the raster line
  `raster 604x453 @ 0.75x · 10.75 ms raster · 8.91 ms upload`; the result line names the engine's
  `…\plots\run-1\plot.svg`.
- J-15 zoom `[J-15-zoom-uia.txt](J-15-zoom-uia.txt)` (583 nodes): `zoom 1.25, 94%` and
  `raster 756x567 @ 0.94x`.

After a clean quit the window's engine plot is copied to
[J-15-window-plot.svg](J-15-window-plot.svg): 15 000 bytes,
`sha256=0B23D6F7…1406C`, **byte-identical** to the headless export
[journey-out/J-15-export.svg](journey-out/J-15-export.svg) and to the `REF-plot` SVG.

## 4. Artifact inventory

| Artifact | Bytes | SHA-256 |
|---|---:|---|
| [journey-run.log](journey-run.log) | 14 062 | `410EBDDBB86A5AF0A72F76040120E9A180B791863A707306CDD7FA8262BBCC9D` |
| [cargo-test.log](cargo-test.log) | 30 620 | `CD98466FB99E5E7166C591D25F30C70D5396F64CB547B727E22787E0DB9C6711` |
| [gui-selftest.log](gui-selftest.log) | 4 819 | `695149C340352D022D94F15F076B55A29B2A67FB5019B4E8970451288D080478` |
| [window-proof.json](window-proof.json) | 5 994 | `3CB179CFF4710D86CC86BC3B4A05FED1B45B1177AC19E60BEB026B43AC7C6DE0` |
| [capture-journeys.log](capture-journeys.log) | 6 502 | `8ED72EF325044DF9580B3AE333F9D3BD21B6D64EB10FD05BE75A88E928F55EA1` |
| [J-15-window-plot.svg](J-15-window-plot.svg) | 15 000 | `0B23D6F74C16BDD102CEE23E9388D884A9212FC614CCA55B9F814EA66271406C` |
| [journey-out/J-15-export.svg](journey-out/J-15-export.svg) | 15 000 | `0B23D6F74C16BDD102CEE23E9388D884A9212FC614CCA55B9F814EA66271406C` |
| [journey-out/J-15-plot-2x.png](journey-out/J-15-plot-2x.png) | 29 776 | `048627E57811DACE80844FF33807045262F8CB15AB55BCC2CF3828D13F4AE1E7` |
| [journey-out/J-09-run1.svg](journey-out/J-09-run1.svg) | 15 104 | `6A80A4430F13F8A80D73A9CAA06562A47E1A29D53BAF2B039109B58DA482DE35` |
| [journey-out/J-09-run2.svg](journey-out/J-09-run2.svg) | 15 104 | `6A80A4430F13F8A80D73A9CAA06562A47E1A29D53BAF2B039109B58DA482DE35` |

Scripts: [run-journeys.ps1](run-journeys.ps1) rebuilds and reruns all five headlessly;
[capture-journeys.ps1](capture-journeys.ps1) reproduces the window proof. Fixtures:
[J-02.ls](J-02.ls) (17 bytes), [J-15.ls](J-15.ls) (23 bytes). `appdata/`, `work/`,
`window-appdata/`, `window-work/` and `gui-selftest-*/` are the redirected per-run state the
scripts create and clean; they are not primary evidence.

## 5. Suite summary

| Suite | Command | Result |
|---|---|---|
| Desktop crate tests | `cargo test -p lovelace-desktop` (environment below) | **354 passed, 0 failed, 0 ignored** (347 before this round + 7: 4 `journey` helper/verdict tests, 2 engine clear tests, 1 workbench clear test) — [cargo-test.log](cargo-test.log) |
| GUI selftest | `lovelace-desktop.exe --gui-selftest` with `APPDATA=round-33/gui-selftest-appdata`, cwd `gui-selftest-work` | **checks=19 failed=0** — [gui-selftest.log](gui-selftest.log) |
| Journeys | `lovelace-desktop.exe --journey J-02 --journey J-03 --journey J-09 --journey J-10 --journey J-15` | **journeys=5 failed=0**, exit 0 — [journey-run.log](journey-run.log) |
| Window proof | `capture-journeys.ps1` (locked session) | **passed=28 failed=0** — [window-proof.json](window-proof.json) |

Environment (the task's exact settings):

```powershell
$repo='C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME="$repo\.toolchain\rustup"; $env:CARGO_HOME="$repo\.toolchain\cargo"
$env:TMP="$repo\rust\target-desktop\tmp"; $env:TEMP=$env:TMP
$env:CARGO_TARGET_DIR="$repo\rust\target-desktop"
& "$repo\.toolchain\cargo\bin\cargo.exe" test -p lovelace-desktop --manifest-path "$repo\rust\Cargo.toml"
```

Reproduce the whole round from this folder:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\run-journeys.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File .\capture-journeys.ps1
```

The scripts set `APPDATA` and the process current directory themselves, so the resolved plot
directory is writable and is never below the working directory (`PLF-006`).
