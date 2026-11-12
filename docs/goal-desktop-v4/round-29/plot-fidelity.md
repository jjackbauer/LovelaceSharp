# PLT-001 / PLT-002 - full-SVG plot fidelity and asset safety (round 29)

Status: **PASS** for the four-document corpus, the security check and the window proof.
The product's resvg 0.50 / tiny-skia 0.11.4 path renders the engine SVG within 0.41 % of an
independent headless-Edge raster on every document, with **zero** structural ink uncovered at a
3 px radius, and the hostile SVG changes **zero** pixels.

This round does not claim that resvg "handles SVG". It reports the measured difference between
the product raster and a second rasteriser, and records the tolerance that turns that difference
into a verdict. The numbers below are from `fidelity.json`; the images are in `compare/`.

## 1. What the product renders and how it was measured

The plot pane (`rust/lovelace-desktop/src/ui/plot.rs`) parses `PlotCapture.svg` with
`usvg::Tree::from_str` using the system font database, rasterises it with
`resvg::render` into a `tiny_skia::Pixmap`, and uploads that pixmap as an egui texture. Round 29
adds no second renderer: the harness calls the pane's own `Plot::set_svg` and `raster_pixmap`
through the new acceptance mode `lovelace-desktop --raster-svg <in.svg> --out <out.png>`.

| Rasteriser | Implementation | Version | Invocation |
|---|---|---|---|
| Product | usvg 0.50.0 + tiny-skia 0.11.4, system fonts, PLT-002 image resolver | resvg 0.50.0 | `lovelace-desktop.exe --raster-svg <svg> --out <png>` |
| Independent | Microsoft Edge, headless screenshot | 157.0.4341.46 | `msedge --headless=new --disable-gpu --hide-scrollbars --force-device-scale-factor=1 --window-size=W,H --screenshot=<png> <file.svg>` |

Both rasterise each document at its intrinsic 800 x 600 size with a device scale of 1.0. The
Edge window is sized from the product PNG; every Edge screenshot came out at exactly the product
size (asserted by the harness), so the comparison is 1:1 with no resampling on either side.
Product binary SHA-256: `B553765C...91660F52` (65,442,816 bytes), recorded in `fidelity.json`.

## 2. Corpus

| Document | Provenance | Bytes | SHA-256 (first 16) | Element census (rect / line / text / polyline / path / g / clipPath) |
|---|---|---:|---|---|
| `corpus/ref-plot.svg` | Live C# Studio capture, extracted verbatim from `docs/goal-desktop-v4/reference/REF-plot.json` (`steps[0].final.response.plot.svg`); the 15,000-byte response recorded by EVD-029/EVD-033 | 15,000 | `0B23D6F74C16BDD1` | 2 / 13 / 11 / 1 / 0 / 0 / 0 |
| `corpus/engine-square.svg` | Rust engine: `lovelace-run --file work/engine-square/plot.lv` with `plot(1..10, (1..10)^2, "square")`; titled, positive series | 15,098 | `2141C042D7DA8256` | 2 / 13 / 12 / 1 / 0 / 0 / 0 |
| `corpus/engine-cubic.svg` | Rust engine: `plot([-10,...,10], [-1000,...,1000])`; untitled, both axes cross zero, negative tick labels | 14,711 | `21166F96299BD9CA` | 2 / 12 / 10 / 1 / 0 / 0 / 0 |
| `corpus/feature-coverage.svg` | Harness-derived from `engine-square.svg`: the engine's own polyline is embedded inside `<g clip-path="url(#left-panel)" transform="translate(0 -6)">`, with a dashed cubic `<path>` and a rotated `<text>` added | 13,484 | `BF5EFE5B97CBE1AF` | 3 / 0 / 2 / 1 / 1 / 2 / 1 |

Notes on the corpus:

- The two Rust-engine documents differ in real element mixes: a title versus no title, 12 versus
  10 `<text>` labels, 13 versus 12 `<line>` ticks/axes, and a positive range versus one that
  crosses zero.
- The current engine renderer (`SvgPlotRenderer`) emits only `rect`, `line`, `text` and
  `polyline`. It does **not** emit `path`, `clipPath`, `transform` or `stroke-dasharray`, so no
  engine-produced document can exercise those features. `feature-coverage.svg` exists for exactly
  that gap: its inner curve is the Rust engine's own `<polyline>`, and the clipping, transform,
  dashed path and rotated label around it are harness markup. It is labelled "harness-derived"
  in the table and is not presented as engine output.
- The 15,000-byte claim is re-checked at generation time: the harness refuses to continue if the
  extracted reference is not exactly 15,000 bytes.

## 3. Per-image difference metrics

`Diff > 16` and `Diff > 64` count pixels whose largest RGB channel delta exceeds that threshold.
`Mean abs` is the mean absolute per-channel delta over all pixels. `Ink uncovered` is the
structural check described in section 4. All dimensions were 800 x 600 (480,000 px).

| Document | Product px | Edge px | Diff > 16 | Diff > 64 | Mean abs | Max | Ink uncovered | Diff bbox | Verdict |
|---|---|---:|---:|---:|---:|---:|---:|---|---|
| `ref-plot` | 800 x 600 | 800 x 600 | 0.2342 % (1,124 px) | 0.1254 % | 0.1405 | 255 | 0.0000 % | 43,68 - 744,545 | PASS |
| `engine-square` | 800 x 600 | 800 x 600 | 0.3256 % (1,563 px) | 0.1798 % | 0.1918 | 255 | 0.0000 % | 43,17 - 744,545 | PASS |
| `engine-cubic` | 800 x 600 | 800 x 600 | 0.2448 % (1,175 px) | 0.1435 % | 0.1428 | 223 | 0.0000 % | 93,68 - 744,526 | PASS |
| `feature-coverage` | 800 x 600 | 800 x 600 | 0.4090 % (1,963 px) | 0.1283 % | 0.1518 | 178 | 0.0000 % | 80,50 - 447,577 | PASS |

The difference images (product | Edge | pixels above threshold) were opened and inspected:

- [ref-plot comparison](compare/ref-plot-compare.png) (43,330 bytes)
- [engine-square comparison](compare/engine-square-compare.png) (45,822 bytes)
- [engine-cubic comparison](compare/engine-cubic-compare.png) (49,041 bytes)
- [feature-coverage comparison](compare/feature-coverage-compare.png) (43,147 bytes)

The raw rasters are in `rasters/product/` and `rasters/edge/`, and every number above is also in
`fidelity.json`. The difference panel is black where the two rasters agree exactly, dark blue for
a delta of 1-16, and red for a delta above 16. What is visible is the outline of glyphs and thin
strokes: the two renderers place every element at the same coordinates but cover edge pixels
differently.

## 4. Tolerance and why

| Parameter | Value | Rationale |
|---|---:|---|
| Per-pixel difference threshold | 16 / 255 | Both rasters are lossless PNGs of the same source, so there is no compression noise to absorb. A delta above 16 is a real coverage difference, not a rounding artefact. Low-amplitude antialias fringes stay below it. |
| Maximum differing pixels | 1.0 % | Observed maximum is 0.4090 %, so the pass limit has a 2.4x margin. A missing 1 px-wide curve would only add about 0.3 %, which is why the percentage alone is not the only criterion. |
| Strict threshold | 64 / 255 | Reported, not used as a gate. It separates half-tone fringes from near-full-coverage flips; all strict differences are edge pixels. |
| Structural ink threshold | luminance < 245 | White canvas (255) is not ink; the light grey frame (204), axes (51), glyphs and the red curve are ink. Channel-independent and insensitive to colour. |
| Structural radius | 3 px | The two rasterisers use different glyph and line antialiasing; a 1-3 px edge shift is expected. A genuinely moved or missing element is farther than 3 px from any matching ink. |
| Maximum uncovered ink | 0.1 % | Observed 0.0000 % on all four images. A dropped polyline would leave roughly 14 % of the product's ink unmatched, and a missing tick label several tenths of a percent, so 0.1 % is a strict structural gate, not a rubber stamp. |

Why a second structural metric is needed: a thin 2 px polyline is only about 0.3 % of the image.
If it disappeared, `Diff > 16` could remain under the 1.0 % limit. The ink-coverage check asks a
different question - "does every painted pixel of each raster have painted pixels within 3 px in
the other?" - so a missing, clipped-away or displaced element fails even when the raw percentage
is small. On this corpus both directions are 0 uncovered pixels, which is the strongest result
the check can return.

What the tolerance explicitly does **not** excuse: a missing element, a wrong position beyond
3 px, an ignored clip path, an ignored transform, or a moved label. Each of those would produce
uncovered ink and/or a large, spatially contiguous diff region; the corpus exercises all of these
features in `feature-coverage.svg`.

Font note: both rasterisers resolved `font-family="sans-serif"` to Arial on this machine. The
harness does not force a font file, because the product must use the system fonts the pane loads.
A machine whose generic sans-serif differs could raise the text-edge percentage; the structural
check would still localise any real layout change. This is a Windows-only measurement.

## 5. Asset / security check (PLT-002)

`corpus/security-hostile.svg` contains, in one 200 x 120 document:

- `<script type="text/javascript">document.getElementById('marker').setAttribute('fill','#ff00ff');</script>`;
- `<image xlink:href="file:///C:/Windows/win.ini" ...>`;
- `<image href="file:///<path to a real magenta PNG>" ...>` (the PNG exists and is readable);
- `<image href="<plain absolute path to the same PNG>" ...>`.

`corpus/security-control.svg` is the same document without the script and the three images.
`corpus/security-image-only.svg` has the images but no script.

| Renderer | Hostile vs control | Image-only vs control | Magenta pixels in hostile | Script executed | Verdict |
|---|---:|---:|---:|---|---|
| Product (usvg + tiny-skia, pane resolver) | **0** differing pixels | **0** differing pixels | **0** | no (static renderer has no JS) | PASS |
| Microsoft Edge 157.0.4341.46 (browser, for contrast) | 15,516 differing pixels | 11,916 differing pixels | 15,291 | yes - `--dump-dom` contains the `ff00ff` fill the script sets | browser behaviour, not the product criterion |

The product's three security PNGs are byte-identical (1,284 bytes, SHA-256
`F09596792D926B12...`), not merely pixel-identical. Evidence:

- [product control](rasters/product/security-control.png), [product hostile](rasters/product/security-hostile.png), [product image-only](rasters/product/security-image-only.png)
- [security comparison, product](compare/security-product-compare.png) (control | hostile | diff)
- [security comparison, Edge](compare/security-edge-compare.png) (control | hostile | diff)
- [Edge DOM dump](work/security-hostile-dom.txt), which shows the browser executed the script.

The product-side part of PLT-002 is a code change, not only an observation: `Plot::new` now
installs a usvg `ImageHrefResolver` whose string resolver always returns `None`. usvg's documented
default treats a string href as a filesystem path and opens it; the pane no longer does. Only
`data:` image URLs resolve. `<script>` is not implemented by usvg and is inert. The same
`Plot` object was checked after the hostile document: it reports no parse error, `clear()` works,
and it renders the control document pixel-identically again (unit test
`external_references_are_refused_and_scripts_do_not_run`). That is the "app remains usable" half
of the check at the renderer layer; the window session in section 6 is the same product binary
running a normal plot through the GUI.

Scope limit, stated rather than papered over: the hostile SVG is loaded through the exact raster
path the pane uses, not through the GUI's engine capture (the Rust engine can only emit its own
generated SVG). The GUI path is therefore not an exposure surface for this payload; the security
property is tested at the layer that would have to resolve the reference or run the script.

## 6. Window proof

`capture-window.ps1` launches the shipped `lovelace-desktop.exe` on `work/window/window-plot.lv`:

```lovelace
x = 1..24
y = (x - 12)^2
plot(x, y, "fidelity plot: text and polyline")
```

It resizes the window to 2500 x 1380, invokes Run through UI Automation, waits for "run Finished",
captures with `PrintWindow(..., PW_RENDERFULLCONTENT)`, clicks `1.5x`, waits for the plot widget to
report zoom 1.50, captures again, then clicks `Fit` and verifies the same session returns to zoom
1.00. UI Automation exposes the plot as `Image | SVG plot canvas, 800 by 600 points, zoom 1.00/1.50`.

| Capture | Size | Bytes | SHA-256 | Plot widget | Raster line | Pane / image rect |
|---|---|---:|---|---|---|---|
| `plot-100.png` | 2500 x 1380 | 442,463 | `EF44A99B...0D6CF5A7` | zoom 1.00 | `raster 800x600 @ 1.00x - 16.79 ms raster - 14.84 ms upload` | pane 980 x 975; image 964 x 723 |
| `plot-150.png` | 2500 x 1380 | 428,690 | `0DEFD337...21C4567` | zoom 1.50 | `raster 1200x900 @ 1.50x - 26.36 ms raster - 33.49 ms upload` | pane 980 x 975; image 1446 x 1084.5 |

The 1.5x image is 1446 x 1084.5 against a 980 x 975 pane: it is 466 px wider and 109.5 px taller
than the pane, so the overflow is clipped by the egui painter rectangle. The capture shows
the plot's white canvas running to the pane edge and the right side of the curve/frame cut off;
the left axis and the enlarged title remain. The two frames differ on 16.4276 % of the window's
pixels, so the zoom step demonstrably repainted rather than reusing the 100 % texture.

Palette counts inside the 100 % plot image prove it is the rendered SVG and not an empty state:
white canvas 640,436 px, series red 1,779 px, dark axes/labels 4,722 px, UI accent teal 4,798 px.
`window-proof.json` carries the UIA assertions, the script, the palette and the hashes.

- [plot-100.png](plot-100.png) - 100 % fit
- [plot-150.png](plot-150.png) - one zoom step; clipped at the pane
- [plot-100-uia.txt](plot-100-uia.txt), [plot-150-uia.txt](plot-150-uia.txt)
- [window-proof.json](window-proof.json), [capture-run.log](capture-run.log)

## 7. Supported feature set (PLT-002 statement)

The renderer is usvg 0.50's static SVG subset with one deliberate restriction:

- Shapes: `rect`, `line`, `polyline`, `polygon`, `path`, `circle`, `ellipse`.
- Text: system fonts, `font-size`, `font-family`, `font-weight`, `text-anchor`, nested `tspan`.
- Structure: `g`, `defs`, `clipPath`, `mask`, `transform`, `opacity`, gradients and patterns
  (usvg's supported set), `stroke-dasharray`, `fill`/`stroke`, stroke widths and joins.
- Embedded images: `data:` URLs only; PNG, JPEG, GIF, WebP and SVG data images.
- Deliberately refused: any non-`data:` image href (`file:`, `http:`, `https:`, absolute or
  relative paths). Scripts are not executed; animations/SMIL are not rendered.

The corpus exercises text labels, `rect`, `line`, `polyline`, `path`, `clipPath`, `g` transform
and `stroke-dasharray`. `circle`, `ellipse`, `polygon`, gradients, masks and data images are
supported by usvg but not exercised by this corpus, so they are listed as supported by the
renderer, not as measured here.

## 8. Suite summary

Command (the environment in the task statement):

```powershell
$repo='C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME="$repo\.toolchain\rustup"; $env:CARGO_HOME="$repo\.toolchain\cargo"
$env:TMP="$repo\rust\target-desktop\tmp"; $env:TEMP=$env:TMP
$env:CARGO_TARGET_DIR="$repo\rust\target-desktop"
& "$repo\.toolchain\cargo\bin\cargo.exe" test -p lovelace-desktop --manifest-path "$repo\rust\Cargo.toml"
```

Result: **263 passed, 0 failed, 0 ignored** in 1.20 s (`cargo-test.log`). The two new tests are
`ui::plot::tests::renders_clipping_transforms_dashed_paths_and_labels` and
`ui::plot::tests::external_references_are_refused_and_scripts_do_not_run`.

`--gui-selftest` is **not green** in this environment, and the round does not claim it is. It runs
15 checks and fails the two plot checks for reasons that predate this round:

- With `APPDATA` inside the workspace (`gui-selftest-workspace-appdata.log`), `plot` runs but
  `check_plot` still asserts the pre-round-87 3-point stub and the current engine emits a
  580-point densely sampled spline; `plot-dir` fails because the resolved directory is under the
  working directory, which the check refuses by design.
- With `APPDATA` outside the workspace (`gui-selftest.log`), the app cannot create
  `%APPDATA%\Lovelace\plots` in this harness and both checks fail earlier with "Access is denied".

Neither failure is in the resvg path; `gui_selftest.rs` is unchanged by this round, and the window
proof in section 6 exercises the real engine-to-plot path end to end.

## 9. Reproduce

```powershell
# corpus + both rasters + metrics + security (~25 s)
powershell -NoProfile -ExecutionPolicy Bypass -File tools/plot-fidelity.ps1

# window proof (needs an unlocked session; uses the normal UIA harness)
powershell -NoProfile -ExecutionPolicy Bypass -File docs/goal-desktop-v4/round-29/capture-window.ps1
```

The fidelity harness refuses to skip the independent raster: it locates Edge (or Chrome) and
throws if neither exists. Edge 157.0.4341.46 was found at
`C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe`.

## 10. Not verified / limits

- No C# Studio raster was produced; the Studio is a web app, and its captured SVG is the input to
  both rasterisers. Edge is an independent rasteriser for this comparison, not an oracle for what
  the C# Studio's browser showed.
- `http:`/`https:` references were not exercised, but the resolver refuses every non-`data:`
  string href, so the code path is the same as the tested `file:`/absolute-path cases.
- Animations, SMIL, filters and masks are outside this corpus. They are usvg-supported in
  general, but no claim is made that this round measured them.
- One DPI setting (device scale factor 1.0) and one Windows font set were measured; the pane's
  texture filtering for non-integer screen scales is outside the metric.
- The hostile SVG is not loaded through the GUI's engine capture path; see the scope note in
  section 5.
- The `--gui-selftest` failures above are reported as observed, not fixed in this round.
