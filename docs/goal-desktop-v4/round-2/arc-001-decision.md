# ARC-001 — Native rendering-stack decision

**Decision (provisional, Windows-validated):** adopt **egui/eframe 0.37** with **winit**, **AccessKit**
(through eframe's default features), **resvg 0.50 + tiny-skia** for SVG scene rasterisation, and the
platform file dialogs. Retire the raw Win32/GDI shell as the product UI: it cannot carry a rich
editor, a UI Automation tree, or theme tokens without hand-writing all three.

**Status:** the Windows half of the spike is complete and measured. The Linux half is deferred by
DEC-006. Competitor prototypes are minimal (DEV-007) and are recorded below as criteria-based
rejections, not as measurements.

## What the spike actually demonstrated

| Criterion | Measurement | Evidence |
|---|---|---|
| Editor fidelity at scale | 5 000-line / 504 999-byte document, first editor pass 13.4 ms, 46 visible rows / 45 laid out, typing p50 0.359 ms / p95 0.635 ms / max 0.708 ms over 200 edits | EVD-041, EVD-034 |
| Syntax highlighting + gutter | contextual colours through a custom layouter, gutter line numbers, current-line highlight, accurate Ln/Col | EVD-034 |
| Accessibility | AccessKit publishes 120 nodes; independent UIA dump shows 63 named raw-view nodes including `Edit` "Script editor, 5000 lines, caret line 1 column 9", `Image` "SVG plot canvas…", `Separator` splitters, buttons | EVD-042, EVD-035 |
| SVG plots | rect, polyline, text, path, clipPath, g-transform rendered; raster 6.1 / 11.7 / 20.8 ms at 100/150/200 %; malformed SVG rejected inline, no panic | EVD-043 |
| Docking / layout | four panes, draggable splitters (39.6 px drag), keyboard resize (7.9 px), minimum sizes hold, fits 1280x800, 900x600 and even 640x400 | EVD-034, `spike-bench.json` s5 |
| HiDPI | pane fractions stable at 2560x1600 across 1.0/1.5/2.0; glyph pixels scale with ppp; plot texture scales | `spike-bench.json` s6 |
| Startup / idle | first painted frame 557 ms; idle 2.8 % of one core over 5 s (153 repaints) | EVD-044 |
| IME | IME-capable text field present; composition not driven (not verified) | `spike-bench.json` s2 |

## Why the alternatives were not chosen

| Candidate | Deciding factor | Strength of evidence |
|---|---|---|
| Raw Win32 + GDI (incumbent) | No UIA provider, no rich-text decorations, no theming in `shell.rs`/`ffi.rs`; every editor feature would be hand-written on top of an `EDIT` control | Verified by census (EVD-004..006, OBS-006) |
| iced | Text editor widget exists but was not prototyped here; no run evidence for accessibility or a 5 000-line document | Criteria-based only (probe report pending, DEV-007) |
| slint | Declarative UI; text input is basic and no 5 000-line editor evidence; not prototyped here | Criteria-based only (DEV-007) |
| GPUI | Not evaluated beyond documentation; no stable public API story for this product | Not evaluated |
| Tauri | Excluded by the contract: it embeds a system webview and keeps a JavaScript UI runtime | Contract (ARC-002) |

## Risks this decision carries

1. **Idle repaint**: 2.8 % CPU / 153 frames per 5 s exceeds PERF-008's 2 % ceiling. The product must
   request repaints only on change (and prove it), or the budget fails.
2. **High-DPI minimum sizes**: at 1280x800 with 1.5x/2.0 zoom the configured pane fractions cannot be
   held because minimum sizes bind (EVD-045). VIS-016/A11Y-004 need a layout strategy for that case.
3. **First full-buffer layout**: one 244 ms pass over a 500 KB buffer at open. Acceptable once, but it
   must never happen per keystroke; the product needs a virtualised text widget or a windowed
   layouter.
4. **IME** is unverified. EDT-017 remains open until a composition test is driven.
5. **Only one candidate was measured.** The comparison for iced/slint is criteria-based; if either is
   later shown to meet the same measured criteria more cheaply, this decision should be revisited.

## What would change this decision

A second candidate reaching equivalent measurements — typing p95 ≤ 16 ms on 5 000 lines, a real UIA
tree with named editor/plot nodes, SVG raster ≤ 250 ms, stable HiDPI layout and idle ≤ 2 % — with a
lower maintenance or dependency cost. The decision is a measurement, not a preference.
