# Round 30 - PLT-004 (zoom / pan / fit), PLT-006 (SVG and PNG export), DEF-003

Status: **PASS**. The plot pane's wheel zoom keeps the point under the cursor, the drag pans, Fit
restores the whole plot and 100% is a separate natural-size reset. SVG export writes the engine's
`PlotCapture.svg` bytes unchanged through the **native Windows save dialog**, PNG export writes a
valid raster at a chosen scale and background, and the stale `--gui-selftest` plot assertion is
fixed against the current engine shape. `--gui-selftest` is **15/15 with a writable APPDATA**; with
the default (unwritable) `%APPDATA%` exactly the two checks that need the plot directory fail with
"Access is denied", and that is the known environment condition, not an assertion failure.

All code lives in `rust/lovelace-desktop/**`; all evidence in this folder. No engine crate,
`tools/` file or gate file was modified.

## 1. What changed

| File | Change |
|---|---|
| [`rust/lovelace-desktop/src/ui/plot.rs`](../../../rust/lovelace-desktop/src/ui/plot.rs) | `PlotView`, the whole zoom/pan transform as a pure value (`fit_scale`, `zoom_at`, `pan_by`, `clamped`, `actual_size`, `percent`); the pane's wheel and drag handlers now call it; Fit and 100% methods; `export_svg_to`, `export_png`, `export_png_to`, `export_pixels`, `PngBackground`, `save_plot_dialog`; 14 new tests |
| [`rust/lovelace-desktop/src/ui/app.rs`](../../../rust/lovelace-desktop/src/ui/app.rs) | Plot header: `Fit`, `100%`, `Zoom out`, `Zoom in`, live percentage; `Export SVG...` and an `Export PNG...` options window (`1x/2x/4x`, `Opaque white/Transparent`, output size, `Save PNG...`); export status line; the save dialog starts in the engine capture's writable folder |
| [`rust/lovelace-desktop/src/win32/ffi.rs`](../../../rust/lovelace-desktop/src/win32/ffi.rs) | `save_file_dialog_with(owner, suggested, title, filters, default_extension, initial_dir)` and `foreground_window_of_this_process()`; the legacy script dialog is now one caller of it |
| [`rust/lovelace-desktop/src/gui_selftest.rs`](../../../rust/lovelace-desktop/src/gui_selftest.rs) | DEF-003: `check_plot` asserts the current engine plot shape (element census, 800x600 canvas, one 580-point polyline through the three data vertices, title text) |

No other code file changed.

## 2. PLT-004 - the transform model and its tests

The pane keeps the view as `PlotView { zoom, pan }`:

- `zoom` is relative to the **fit scale** (`min(pane.w - 2*8, pane.h - 2*8) / svg size`), so 1.0 is
  "the whole plot fits".
- `pan` is the offset of the document's centre from the pane's centre, in screen points.
- `scale = fit * zoom` screen points per SVG point. `zoom_at(pointer, factor)` records the scene
  point under the pointer with the *old* transform, then chooses the new pan so
  `scene_to_screen(anchor) == pointer`; it does the algebra on the same values the paint pass uses.
- Every mutation ends in `clamped()`: a scene that fits on an axis is centred (a drag cannot hide
  part of an already fully visible document); a scene larger than the pane may move until exactly
  `MIN_VISIBLE = 32` screen points of it remain visible at the leading or trailing edge:
  `|pan| <= pane/2 - 32 + scene/2`.
- Zoom range 0.05x..64x of the fit scale. The wheel factor is `exp(scroll * 0.0025)`; the
  `Zoom in` / `Zoom out` buttons are one `WHEEL_STEP = 1.25` step around the last hovered point (or
  the pane centre).
- `Fit` = `PlotView::IDENTITY` (zoom 1, pan 0): the whole document, one click.
- `100%` = natural size: `fit * zoom == 1.0` screen point per SVG point, centred. On this pane
  (fit 1.21) that is a different view from Fit, which the test below pins.
- The header percentage is the rendered scale as a percentage of natural size, so Fit reads
  `121%` on this window and one zoom step reads `151%`; the widget's accessible name carries both
  `zoom 1.25` and `151%`.

Tests in `ui::plot::tests` (all run by the suite):

| Test | What it pins |
|---|---|
| `fit_scale_shows_the_whole_document_inside_the_pane_margin` | the fit scale puts the 800x600 document inside the 1000x800 pane minus the 8 px margin and touches at least one axis (largest such scale) |
| `zooming_at_a_known_point_keeps_that_point_stationary` | for the pane centre and an off-centre anchor, after a 2x step the scene point under the pointer moves < 1e-3 screen points, and maps back to the pointer |
| `successive_pointer_zoom_steps_keep_the_anchor_through_the_clamp` | six 1.25x wheel steps at one anchor keep it stationary within 0.01 screen points while zoom passes 2x |
| `pan_then_fit_returns_the_identity_view` | pan/zoom, then the Fit view is exactly zoom 1 / pan 0 and the whole document is inside the pane |
| `pan_is_clamped_so_the_scene_never_leaves_the_pane` | five ±10,000 px drags at 3x: the visible intersection is never empty and never narrower than 32 px on either axis |
| `a_scene_that_fits_is_centred_and_cannot_be_panned_away` | at 0.5x a 900,-700 drag leaves pan exactly zero and the scene fully inside |
| `zoom_is_clamped_to_the_supported_range` | 100 double-steps stop at 64x; 200 halvings stop at 0.05x |
| `actual_size_is_one_svg_point_per_screen_point_and_fit_is_not` | 100% gives exactly 1.0 screen point per SVG point and 100% of natural size; Fit is 121% on this pane, so the two resets are not the same view |
| `the_pane_wheel_and_drag_handlers_use_the_anchored_transform` | drives the real `Plot::ui` with egui `PointerMoved`, `MouseWheel` and `PointerButton` events: the wheel keeps the anchor while egui's smoothed scroll settles, and a press + 40,30 px move pans by exactly 40,30 px |

The last test is the integration seam: it uses the same `Plot::ui` the window paints, not a model
of it. The window proof below adds the real-window Fit/zoom evidence.

## 3. Window proof (PLT-004)

The console session was **locked** during the capture. UIA Invoke (AccessKit) and
`PrintWindow(PW_RENDERFULLCONTENT)` still reach the app window, but injected pointer input cannot
be delivered to a window the lock screen covers, so the window proof does **not** attempt a mouse
drag; the pan is proven by `the_pane_wheel_and_drag_handlers_use_the_anchored_transform` above. The
two required captures are real frames of the shipped `lovelace-desktop.exe` at 2500x1380:

| Capture | UIA plot widget | Raster line | Bytes | SHA-256 |
|---|---|---|---:|---|
| [plot-fit.png](plot-fit.png) | `SVG plot canvas, 800 by 600 points, zoom 1.00, 121%` | `raster 964x723 @ 1.21x` | 432,134 | `02D8AFD6...437EA1F6` |
| [plot-zoom.png](plot-zoom.png) | `SVG plot canvas, 800 by 600 points, zoom 1.25, 151%` | `raster 1206x904 @ 1.51x` | 423,352 | `FD1538BA...C7EC9039` |

The sequence is: run `plot.lv`, `Zoom in` twice (widget 1.00 -> 1.25 -> 1.56), `Fit` (widget back
to 1.00), capture, one `Zoom in` (widget 1.25), capture. The Fit frame shows the complete frame,
axes, ticks, title and curve inside the pane; the zoom frame is clipped at the pane's right and
bottom edges with enlarged labels and the re-rasterised 1206x904 texture. Both captures are in the
folder with their UIA dumps: [plot-fit-uia.txt](plot-fit-uia.txt),
[plot-zoom-uia.txt](plot-zoom-uia.txt).

The APPDATA redirect matters and was used: `%APPDATA%` is
`docs/goal-desktop-v4/round-30/appdata` and the app starts in
`docs/goal-desktop-v4/round-30/work`, so the resolved plot directory
(`appdata\Lovelace\plots`) is writable and is not below the working directory. Without that, every
plot statement fails with "Access is denied" (see section 6) and this proof could not exist. The
UI log pane in both captures shows that resolved path.

## 4. PLT-006 - export

### 4.1 SVG: the engine's bytes, unchanged

`Export SVG...` opens the native Windows common Save dialog (`GetSaveFileNameW`, the dialog the
legacy shell already used, now generic in `win32::ffi::save_file_dialog_with`). It starts in the
folder the engine wrote this run's capture into (which is writable by construction) with a
title-derived file name. The capture script invoked the button through UI Automation, pressed the
dialog's Save button with a posted `BM_CLICK` (the locked session blocks injected keyboard input),
and the app wrote the file.

| File | Bytes | SHA-256 |
|---|---:|---|
| [plot-export.svg](plot-export.svg) (deliverable, copied from the dialog's file) | 14,914 | `3A76EC02C2C3448C3D680958FEF93A5DB2D9654E5715D1AF3FC5F840D23A599F` |
| Dialog-written file: `appdata\Lovelace\plots\run-1\round 30 plot- zoom pan export.svg` | 14,914 | `3A76EC02...23A599F` |
| This run's `PlotCapture.svg`: `appdata\Lovelace\plots\run-1\plot.svg` | 14,914 | `3A76EC02...23A599F` |
| Independent `lovelace-run` capture of the same script, `work\engine-capture\engine-capture.svg` | 14,914 | `3A76EC02...23A599F` |

All four byte sequences are equal (the capture script compares every byte, not only the hash;
`window-proof.json` records `byte_identical_to_this_run_capture: true`,
`byte_identical_to_independent: true`, `copy_matches_written_file: true`). The exported file starts
with the engine's `<svg ...>` line, with no XML declaration and no re-serialised attributes.

Crate-level evidence:

- `svg_export_matches_the_captured_plot_capture_bytes` runs a real `RealEngine`, keeps its
  `PlotCapture`, loads it into `Plot`, exports it, and asserts `std::fs::read(exported) ==
  capture.svg.as_bytes()`; it also asserts the file the engine itself wrote equals the capture.
- `svg_export_copies_the_loaded_bytes_verbatim` uses a document with CRLF line endings, a UTF-8
  title (`Café & co`) and a redundant self-closing space, and asserts the exported file is that
  exact byte string (a re-serialiser would change all three).

The export function is `Plot::export_svg_to`; it writes `self.svg_source.as_bytes()`, the string
`Plot::set_svg` received from `PlotCapture.svg`, and nothing else.

### 4.2 PNG: chosen scale and background

`Export PNG...` opens an in-window options step (`1x`, `2x`, `4x`; `Opaque white` or
`Transparent`; the exact output size) and `Save PNG...` then uses the same native dialog. The
window proof chose `2x` + `Transparent`; the UIA dump
[plot-export-dialog-uia.txt](plot-export-dialog-uia.txt) contains `Selected: 2x, Transparent`,
all five option buttons and `Save PNG...`, and the exported raster is:

| File | Bytes | SHA-256 | Dimensions | Checks |
|---|---:|---|---|---|
| [plot-export.png](plot-export.png) | 42,891 | `C8FFEA6442E7FB3F247330ECE143B8884083C79DB520199CD5F37A13E687DAEB` | 1600 x 1200 | PNG signature `89 50 4E 47 0D 0A 1A 0A`, IHDR reads 1600x1200, `System.Drawing.Bitmap` decodes it to 1600x1200 |

`export_pixels(svg_w, svg_h, scale)` is the single definition of the written size:
`round(w * scale)` by `round(h * scale)`, with a 16384 px per-side refusal instead of a silent
shrink. The test `png_export_writes_the_requested_dimensions_and_decodes` writes 1x, 2x and 3.5x
of a 100x80 document, checks 100x80, 200x160 and 350x280 from the IHDR bytes and from
`tiny_skia::Pixmap::load_png`, and checks that a zero or NaN scale and a 20,000 px request are
errors.

Background semantics are measured in `png_export_honours_the_transparent_and_opaque_background`
with a 100x80 document that paints a red rectangle over only its left half:

- `Transparent` at 2x: a pixel outside the rectangle has alpha **0**; a pixel inside is
  `(255, 0, 0, 255)`.
- `Opaque white` at 2x: the same outside pixel is `(255, 255, 255, 255)`; the two files differ.
- The scale-background matrix is the test's; the window proof uses one combination.

Honest note: the engine chart paints its own opaque white 800x600 background rectangle, so for
**this** document the `Transparent` and `Opaque` exports are pixel-identical - there is nothing
transparent to reveal. The window proof therefore demonstrates the option is exposed and selected
(`Selected: 2x, Transparent`) while the alpha-0/white difference is demonstrated by the crate test
on a document that actually has transparent regions. This is recorded in `window-proof.json` too,
not hidden.

## 5. DEF-003 - the stale plot assertion

**Observed before the fix** (writable APPDATA, `gui-selftest-appdata.log` from round 29 behaviour
reproduced this round):

```
GUI_SELFTEST FAIL plot: the engine SVG parsed 580 points, expected 3
```

**What the assertion was.** `check_plot` asserted exactly one polyline with exactly three points.
That describes the pre-round-87 640x400 stub (one background rect + one 3-vertex polyline). The
current engine publishes the round-87 chart in `lovelace-lang`:
`<svg width="800" height="600">`, a white background rect, a plot-area frame rect, both axes,
tick geometry and labels, the title text, and **one densely sampled series polyline**.

**Is the current shape a regression? No.** For `plot([1, 4, 2], [4, 5, 6], "selftest")` the x
values are not strictly increasing, so the renderer uses its parametric Catmull-Rom path and
samples it at `1 + ceil(segment1_px/2) + ceil(segment2_px/2) = 1 + 338 + 241 = 580` points. The
three plotted points are still represented: the polyline starts at `(1,4)` = `101.81818,522.5`,
passes through `(4,5)` = `738.18182,297.5` and ends at `(2,6)` = `313.93939,72.5` - the same
mapped positions the engine's own tick lines publish. `lovelace-lang`'s
`plot_structure_round87.rs` pins the same dense sampling independently (639 points for monotone
data, `svg.len() > 10_000`), so this is the engine's published structure, not a drift the desktop
should paper over. Evidence is also in
[`gui-selftest-appdata.log`](gui-selftest-appdata.log): 13,906 engine SVG bytes.

**The fix.** `check_plot` now asserts the **current** shape, and is stricter than the old one:

- element census of the raw document: 2 `<rect>`, 14 `<line>` (2 axes + 7 x ticks + 5 y ticks),
  13 `<text>` (1 title + 7 x labels + 5 y labels), 1 `<polyline>`;
- canvas 800x600 points from the parsed viewBox;
- exactly one polyline and exactly **580** sampled points (the dense parametric spline, not a
  stub);
- the polyline passes through all three mapped data vertices
  `(101818,522500)`, `(738181,297500)`, `(313939,72500)` in thousandths of a point;
- the title text `selftest` exists.

The old check would also have passed a 3-point stub with no axes or labels; the new one would not.
No assertion was deleted or loosened.

**After the fix:**

- With a writable APPDATA and a working directory outside it: `summary checks=15 failed=0`
  ([`gui-selftest-appdata.log`](gui-selftest-appdata.log)), including
  `ok plot: 13906 SVG bytes, title "selftest", 2 rect / 14 lines / 13 texts / 1 polyline, 580 sampled points through the three data vertices`.
- Without the APPDATA redirect (`gui-selftest-no-appdata.log`): 15 checks, 2 failures, both the
  known environment condition - `plot` has no capture because the engine cannot write the plot,
  and `plot-dir` reports the `Access is denied` probe. That is the "known APPDATA plot-write
  check"; there is no other failure.

## 6. Suite summary

Command (the environment in the task statement):

```powershell
$repo='C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME="$repo\.toolchain\rustup"; $env:CARGO_HOME="$repo\.toolchain\cargo"
$env:TMP="$repo\rust\target-desktop\tmp"; $env:TEMP=$env:TMP
$env:CARGO_TARGET_DIR="$repo\rust\target-desktop"
& "$repo\.toolchain\cargo\bin\cargo.exe" test -p lovelace-desktop --manifest-path "$repo\rust\Cargo.toml"
```

Result: **277 passed, 0 failed, 0 ignored** in 1.47 s ([`cargo-test.log`](cargo-test.log)). Round
29 had 263 tests; this round adds the 14 new `ui::plot` tests (16 plot tests total, 2 of them older
raster/security tests). No pre-existing test changed except the DEF-003 check in
`--gui-selftest`.

`--gui-selftest`:

| Run | Environment | Result |
|---|---|---|
| [`gui-selftest-appdata.log`](gui-selftest-appdata.log) | `APPDATA` = `rust\target-desktop\round30-selftest-appdata`, cwd = `rust\target-desktop\round30-selftest-cwd` | `checks=15 failed=0`, exit 0 |
| [`gui-selftest-no-appdata.log`](gui-selftest-no-appdata.log) | default `%APPDATA%\Lovelace\plots` (not writable) | `checks=15 failed=2`: `plot` (no capture; engine cannot write) and `plot-dir` (`Access is denied`), exit 1 |

## 7. Reproduce

```powershell
# build and unit tests
$repo='C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME="$repo\.toolchain\rustup"; $env:CARGO_HOME="$repo\.toolchain\cargo"
$env:TMP="$repo\rust\target-desktop\tmp"; $env:TEMP=$env:TMP
$env:CARGO_TARGET_DIR="$repo\rust\target-desktop"
& "$repo\.toolchain\cargo\bin\cargo.exe" build -p lovelace-desktop --manifest-path "$repo\rust\Cargo.toml"
& "$repo\.toolchain\cargo\bin\cargo.exe" test  -p lovelace-desktop --manifest-path "$repo\rust\Cargo.toml"

# window proof: captures + both exports (UIA invoke + PrintWindow; APPDATA redirected internally)
powershell -NoProfile -ExecutionPolicy Bypass -File docs/goal-desktop-v4/round-30/capture-plot.ps1

# --gui-selftest, both configurations
& "$repo\rust\target-desktop\debug\lovelace-desktop.exe" --gui-selftest
$env:APPDATA="$repo\rust\target-desktop\round30-selftest-appdata"
Push-Location "$repo\rust\target-desktop\round30-selftest-cwd"
& "$repo\rust\target-desktop\debug\lovelace-desktop.exe" --gui-selftest
Pop-Location
```

`capture-plot.ps1` also writes `window-proof.json` (all sizes, hashes, UIA strings and the
byte-identity verdicts) and `capture-run.log`.

## 8. Limits, stated rather than hidden

- The console session was locked during the window capture. UIA Invoke and PrintWindow work on a
  locked desktop; injected mouse input does not. Therefore the window proof has no drag capture:
  the pan handler is proven by the headless egui input test and the transform tests, while the
  required Fit and one-zoom captures are from the real window. On an unlocked session the same
  script drives the buttons identically (UIA Invoke), so the only difference would be an optional
  drag step.
- The wheel handler is not injected into the real window for the same reason; it is exercised with
  the same egui events winit delivers in `the_pane_wheel_and_drag_handlers_use_the_anchored_transform`.
- The deliverable `plot-export.svg` / `plot-export.png` are byte-identical copies of the files the
  native dialog wrote inside the engine's plot folder; both paths and hashes are recorded in
  `window-proof.json`. The dialog itself wrote the files; nothing was re-encoded.
- One DPI (device scale factor 1.0) and one Windows font set were measured.
- The transparent/opaque PNG difference cannot show on the engine chart because the chart paints
  its own white background; the difference is measured in the crate test on a document with
  transparent regions.
- No C# Studio export button exists to compare against; the requirement is the preservation of the
  engine's bytes and a valid raster, both of which are measured above.
