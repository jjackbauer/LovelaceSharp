# Requirements: Lovelace.Rust.Desktop — Native Desktop IDE

> Scope: define the desktop application that **replaces** the `Lovelace.Studio` browser IDE when
> the LovelaceSharp stack is ported to Rust. The app is a native, single-user, offline desktop
> program built in Rust on the same engine as every other front-end: no localhost HTTP server, no
> browser, no JavaScript. This document specifies the user-visible behaviour, the transport that
> replaces the Studio REST endpoints, and the acceptance tests. It is a **requirements document
> for review — no implementation yet**. It is a companion to
> [Lovelace.Rust.Port.md](Lovelace.Rust.Port.md) and [Lovelace.Rust.Parity.md](Lovelace.Rust.Parity.md).

---

## 1. What changes, and what must not

The Studio is a browser IDE served by an ASP.NET Core process. Its *capabilities* are the product;
its *transport* is an accident of the browser. The port keeps the first and deletes the second.

| Dimension | Lovelace.Studio (C#) | Lovelace.Rust.Desktop (target) | Parity obligation |
|---|---|---|---|
| Process shape | HTTP server on localhost + browser tab | One native process, one window | **Changed by design** |
| UI technology | HTML + CSS + vendored CodeMirror 6 (780 KB JS) | Rust immediate-mode widgets, no web runtime | **Changed by design** |
| State transport | 12 routes (11 JSON endpoints plus the page), `X-Session-Id` header | In-process command messages | **Semantics preserved, routes deleted** |
| Session persistence | `sessionStorage` token, in-memory server sessions, idle TTL | Document tabs owning live engines, no TTL | **Strengthened** (state survives until the tab closes) |
| Editor content | `localStorage` | Native file documents (`.ls`) + crash-safe autosave | **Strengthened** (real files) |
| Execution | Background task + client polling of run status | Worker thread + channel events | **Semantics preserved** |
| Precision | Toolbar readout + input, per session | Same control, per document | **Preserved** |
| Plot | Inline SVG in a `<div>` | SVG rendered to a texture, zoom/pan/export | **Preserved** |
| Symbolic inspector | `POST /api/symbolic/inspect` | In-process inspection of the last symbolic value | **Preserved** |
| Multi-user | Explicitly out of scope (localhost, arbitrary code) | Structurally impossible (single user, no listener) | **Preserved by construction** |

The IDE's **functional** requirements are already written: [Lovelace.Studio.md](Lovelace.Studio.md)
specifies the editor, workspace, graph, logs and symbolic-inspector behaviour, and
[Lovelace.Studio.Sessions.md](Lovelace.Studio.Sessions.md) specifies sessions, precision, incremental
execution and the async-run model. Both are inherited unchanged. This document specifies the **desktop
delta**: the transport that replaces HTTP, the document model, persistence, platform integration, and
the tests that replace the ones being deleted.

**The transport deletion is a requirement, not a side effect.** The desktop binary must not open a
TCP listener, must not ship an embedded HTTP stack, and must not expose the engine to any local
process. This removes the localhost attack surface the Studio documentation warns about and is
what distinguishes "a desktop app" from "a web app in a window".

---

## 2. Goals and non-goals

### Goals

| # | Goal |
|---|---|
| G1 | A native window with the four Studio panes (editor, workspace, graph, logs) plus the symbolic inspector, laid out the way the Studio lays them out. |
| G2 | Document tabs, each with its own live engine, precision, revision counter and incremental cache — the desktop form of "one session per tab". |
| G3 | Scripts are real files: open, edit, save, save-as, recent files, dirty-state prompts, external-change detection. |
| G4 | Run, stop, and observe: asynchronous execution with statement-level progress, sub-progress for long numeric operations, and a working **Cancel**. |
| G5 | Errors land on the offending line and column, with the same message text the engine reports to the CLI. |
| G6 | Plots render inline from the engine's SVG, and can be zoomed, exported and opened externally. |
| G7 | Autocomplete, syntax highlighting and bracket matching good enough that the desktop app is a better writing surface than the Studio's earlier textarea. |
| G8 | The whole application is Rust, ships as one binary per platform, and starts fast enough to feel like a tool rather than a service. |
| G9 | Nothing is lost when the app closes: layout, open documents, unsaved buffers and per-document precision are restored. |
| G10 | Every command is reachable from the keyboard and announced to assistive technology. |

### Non-goals (v1)

- Multi-user, remote, or collaborative use; authentication; sandboxing of user scripts (the engine runs arbitrary scripts by design — the user is the author).
- Debuggers, breakpoints, stepping, profiling.
- A plugin marketplace or runtime-loaded UI extensions.
- Notebook-style cell execution, although the incremental runner (D-EXEC-6) is the substrate for it later.
- Mobile, web, or terminal-wasm targets.
- Telemetry, crash reporting, update pings, or any outbound network call at runtime.

---

## 3. Framework decision

The framework choice is the one decision that is expensive to reverse, so it is recorded here with
its criteria and its escape hatch.

| Criterion | Weight | egui / eframe | Tauri 2 | Slint | iced |
|---|---|---|---|---|---|
| UI layer written in Rust (no JS) | **must** | ✅ | ❌ (HTML/CSS/JS webview) | ✅ | ✅ |
| No webview / no system browser dependency | **must** | ✅ | ❌ (WebView2, WebKitGTK) | ✅ | ✅ |
| Code-editor fidelity for a scripting language | high | ⚠️ custom widget needed | ✅ (CodeMirror reusable) | ❌ limited text widget | ⚠️ plain `text_editor` |
| Inline SVG plot rendering | high | ✅ via `resvg` → texture | ✅ native DOM | ⚠️ raster only | ⚠️ raster only |
| Dockable, resizable, persisted panes | high | ✅ (`egui_dock` or hand-rolled) | ✅ CSS | ⚠️ manual | ⚠️ manual |
| Accessibility (AccessKit) | medium | ✅ built in | ✅ (DOM) | ⚠️ partial | ❌ |
| Single static binary | medium | ✅ (~10–20 MB) | ❌ (sidecar + web assets) | ✅ (small) | ✅ |
| Effort to reach feature parity | high | medium | low | high | high |
| Long-term maintenance surface | high | one language, one toolchain | two stacks (Rust + JS + Node tooling) | one stack | one stack |

**Decision: `egui`/`eframe` with `egui_dock` for panes, `resvg`/`tiny-skia` for plot rasterisation,
`syntect` for highlighting, `rfd` for native dialogs, `directories` for platform paths.**

Rationale: the brief is "the whole stack in Rust, with a desktop app instead of a web app". A
Tauri build would keep 780 KB of vendored JavaScript and a system webview as the UI runtime, so
the stack would be *half* ported and the desktop requirement would be satisfied only nominally.
`egui` is the only option in the table that satisfies both **must** rows while keeping a mature
widget set, AccessKit support, and a single-binary deployment.

**Consequences accepted:**

- The editor is a bespoke widget (D-EDIT-1..11). Budget for it explicitly; do not assume a drop-in
  CodeMirror replacement exists.
- Long documents (>5 000 lines) must stay smooth; the editor is virtualised from the start
  (D-EDIT-9), not retrofitted.
- Text input relies on the framework's IME and clipboard handling; these must be smoke-tested on
  all three platforms (T-D-02).

**Escape hatch:** if the editor spike (Phase D0, see the port roadmap) fails its acceptance
thresholds — 60 fps scrolling at 5 000 lines, correct IME composition, correct multi-cursor
undo — the fallback is Tauri 2 with the existing CodeMirror bundle, and this document's
sections 4–14 still apply unchanged because they specify behaviour, not widgets. The fallback
converts G8 into "Rust core + JS shell" and must be recorded as a decision, not slipped in.

---

## 4. Window and layout

| ID | Requirement |
|---|---|
| D-LAY-1 | One main window, resizable, with a minimum size of 900×600 logical pixels. |
| D-LAY-2 | Four resizable panes: **Editor** (top-left), **Workspace** (top-right; variables above functions), **Graph** (bottom-left), **Logs** (bottom). The symbolic inspector is a collapsible section in the Workspace pane, as in the Studio. |
| D-LAY-3 | Splitters between every adjacent pane, draggable, with a minimum pane extent; double-click resets a splitter to its default. |
| D-LAY-4 | Each pane can be minimised and maximised independently, exactly as the Studio's pane buttons do. |
| D-LAY-5 | Pane sizes and collapsed states persist per user and are restored on launch (D-PERSIST-2). |
| D-LAY-6 | Layout must survive window resize and DPI change without overlapping or clipped controls. |
| D-LAY-7 | A status strip shows: engine revision, cursor line/column, precision, run state, and elapsed time of the last run — the Studio shows the first four. |
| D-LAY-8 | Layout is keyboard-navigable: pane focus cycles with a documented shortcut, and every splitter has a keyboard equivalent. |

---

## 5. Documents, tabs and sessions

The Studio's "session" is the unit of isolated engine state. On the desktop that unit is the
**document**: a tab that owns a script buffer *and* a live engine.

| ID | Requirement |
|---|---|
| D-DOC-1 | A tab = one script buffer + one engine instance (variables, functions, `_`, last plot, revision, precision, incremental cache). Nothing leaks between tabs. |
| D-DOC-2 | Tabs are independent for execution: a long run in one tab never blocks typing, running or cancelling in another. |
| D-DOC-3 | Creating a tab creates a fresh engine with core + plugin builtins loaded, in the same order the CLI hosts load them (DSP, Symbolics, MathIR, Statistics) so autocomplete and `capabilities()` agree with the CLI. |
| D-DOC-4 | Closing a tab with unsaved edits prompts save / discard / cancel. |
| D-DOC-5 | A tab's engine lives until the tab closes — there is no idle eviction. Memory pressure is addressed by an explicit "reset session" command, not by silent eviction. |
| D-DOC-6 | "Clear variables" empties globals but keeps user functions, matching the Studio and the REPL. |
| D-DOC-7 | Removing a single variable is available from the variables table's context menu. |
| D-DOC-8 | Reopening a saved layout restores tabs, their file paths, their unsaved buffers (if any) and their precision, but not their computed variables — document state is restored, engine state is not. This is an explicit, documented behaviour change from "a browser reload keeps the session". |

> D-DOC-8 is a deliberate narrowing. Silent resurrection of engine state across process restarts
> would require a serialisation format for every value kind (including symbolic DAGs and records),
> which is a feature in its own right and is out of scope for v1.

---

## 6. Execution, progress and cancellation

| ID | Requirement |
|---|---|
| D-EXEC-1 | Run evaluates the whole document as one script. The result, output, variables, functions, plot, diagnostics and timings are published to every pane as one coherent update. |
| D-EXEC-2 | Execution happens off the UI thread. The UI must remain interactive (paint, scroll, type, switch tabs) for the entire run. |
| D-EXEC-3 | Runs are **single-flight per document**: a second Run while one is active either queues (default) or is rejected with a visible reason; it must never interleave engine calls. |
| D-EXEC-4 | Progress is published as `OperationProgress { label, fraction }` events: one per top-level statement, plus sub-progress emitted from `sqrt`, `pi` and factorial. |
| D-EXEC-5 | Cancel stops the run **between statements** and inside long operations at the operation's own check points, and leaves the document in its previous committed state. Cancellation must not corrupt partial variable state. |
| D-EXEC-6 | Incremental execution is preserved: re-running an edited script recomputes only statements whose content hash or read-set changed. Statements that print or plot always re-run. |
| D-EXEC-7 | Per-statement status (reused / computed / cancelled / failed) is visible in the logs and as a gutter marker. |
| D-EXEC-8 | The last run's elapsed time and per-statement timings are inspectable without re-running. |
| D-EXEC-9 | Quick-eval: a one-line input in the logs pane evaluates a single expression in the current document's engine and appends the result, exactly as the Studio's quick-eval field does. |

---

## 7. Editor

| ID | Requirement |
|---|---|
| D-EDIT-1 | Plain text buffer, UTF-8, with LF/CRLF preserved per file. Every offset the engine reports is a **UTF-16 code-unit offset** (see parity risk R1); the editor must convert at its boundary and be correct for astral-plane characters. |
| D-EDIT-2 | Syntax highlighting for the Lovelace language: numbers, strings, interpolated strings, keywords, operators, comments, builtin names, user-defined names. |
| D-EDIT-3 | Autocomplete from a live catalog: keywords, builtins with signatures, user functions, live variables — the same four sources the Studio's `/api/completions` serves. Accepting a signature completes the parentheses and placeholders. |
| D-EDIT-4 | Diagnostics are rendered inline at the reported line/column with a hover message; the first diagnostic on a failed run scrolls into view and places the caret. |
| D-EDIT-5 | Bracket matching and auto-closing for `()`, `[]`, `{}`, and `"`. |
| D-EDIT-6 | Indent/unindent selection; auto-indent inside blocks; Tab inserts spaces per a configurable width. |
| D-EDIT-7 | Find and replace, case-sensitive toggle, whole-word toggle, wrap-around, "replace all" as one undo step. |
| D-EDIT-8 | Undo/redo with a per-tab history, coalescing typing bursts; undo must not lose cursor position. |
| D-EDIT-9 | Virtualised rendering: files of 5 000 lines and 500 KB must scroll at display refresh rate, and editing them must not allocate per keystroke proportional to file size. |
| D-EDIT-10 | Go-to-line; duplicate line; move line up/down; comment/uncomment. |
| D-EDIT-11 | The cursor line/column readout shows 1-based line and column, matching the Studio's `Ln 1, Col 1`. |

---

## 8. Workspace panes

| ID | Requirement |
|---|---|
| D-WS-1 | **Variables table**: name, kind, display text per row, ordered by name (ordinal), including `_`. |
| D-WS-2 | Selecting a variable shows its **structured** projection in a detail area (the same structure the CLI publishes), so a user can read `canonical` for a symbolic value instead of a truncated display string. |
| D-WS-3 | **Functions table**: name, parameter list, kind (builtin vs user). |
| D-WS-4 | Both tables are searchable/filterable when they exceed one screen. |
| D-WS-5 | Copy a value's display or structured form to the clipboard. |
| D-WS-6 | **Symbolic inspector**: for the last symbolic value, show canonical form, expression tree, active assumptions, simplification trace and compiled MathIR — the five views the Studio's inspector exposes. Absent inputs show an explicit "no symbolic result yet" state, never an empty pane. |
| D-WS-7 | A builtin browser ("help") lists categories, functions and per-function help from the live registry, and can insert an example into the editor. |

---

## 9. Graph pane

| ID | Requirement |
|---|---|
| D-PLOT-1 | The SVG produced by `plot(...)` renders inline at device-pixel resolution (crisp on HiDPI). |
| D-PLOT-2 | Zoom (wheel, keyboard, buttons), pan (drag), and "fit window"; zoom state resets when a new plot arrives. |
| D-PLOT-3 | Export the current plot as PNG at a chosen scale and as SVG (the engine's own bytes, unmodified). |
| D-PLOT-4 | The plot's source path is shown and "reveal in file manager" is offered. |
| D-PLOT-5 | When no plot has been produced, an explicit placeholder is shown ("no plot yet"). |
| D-PLOT-6 | Plot render is asynchronous for large SVGs; the UI must not block on rasterisation. |

---

## 10. Logs pane

| ID | Requirement |
|---|---|
| D-LOG-1 | Append-only, newest at bottom, with severity classes (info / result / error / progress). |
| D-LOG-2 | Configurable buffer cap with oldest-first eviction; the cap must not lose the current run's output. |
| D-LOG-3 | Copy selection, copy all, clear. |
| D-LOG-4 | `print()` output appears exactly once, in order, with no line terminators introduced or removed. |
| D-LOG-5 | Errors show message plus `code` and `category` from the engine's diagnostic contract. |

---

## 11. Precision

| ID | Requirement |
|---|---|
| D-PREC-1 | A persistent readout shows the document's precision in decimal places (default 1 000). |
| D-PREC-2 | A control sets it (numeric input + apply, and the presets 20 / 50 / 1000 / 2500). |
| D-PREC-3 | Setting precision is document-scoped and invalidates incremental cache entries whose results depend on precision. |
| D-PREC-4 | `setprecision(n)` inside a script updates the readout and the document's precision, exactly as in the Studio. |
| D-PREC-5 | A precision that would make a run exceed memory is refused with a clear message *before* the run starts, not by hanging. |

---

## 12. Persistence

| ID | Requirement |
|---|---|
| D-PERSIST-1 | Persistent state lives in the platform configuration directory (`%APPDATA%\Lovelace`, `~/Library/Application Support/Lovelace`, `~/.config/lovelace`). |
| D-PERSIST-2 | Persisted: window geometry, layout, open documents and their order, per-document precision, recent files, editor settings. |
| D-PERSIST-3 | Unsaved buffers are autosaved to the config directory (crash recovery) and offered for restore on the next launch. |
| D-PERSIST-4 | Settings file is human-readable, versioned with a schema number, and forward/backward tolerant: an unreadable or newer file is backed up and defaults are used rather than crashing. |
| D-PERSIST-5 | Nothing in the config directory is a secret; no credentials exist. |

---

## 13. Platform integration

| ID | Requirement |
|---|---|
| D-PLAT-1 | Native file dialogs for open/save/save-as, filtered to `*.ls`. |
| D-PLAT-2 | Drag-and-drop a `.ls` file onto the window opens it in a new tab. |
| D-PLAT-3 | A native menu bar on macOS; an in-window menu on Windows/Linux. Every menu command appears in the command palette with its accelerator. |
| D-PLAT-4 | Accelerators: Run `Ctrl/Cmd+Enter`, Save `Ctrl/Cmd+S`, Save As `Ctrl/Cmd+Shift+S`, New tab `Ctrl/Cmd+N`, Close tab `Ctrl/Cmd+W`, Cancel run `Esc` (when a run is active), Find `Ctrl/Cmd+F`, Command palette `Ctrl/Cmd+Shift+P`. |
| D-PLAT-5 | External modification of an open file is detected and the user is asked to reload or keep the buffer; no silent overwrite. |
| D-PLAT-6 | Single-instance behaviour: launching the binary again focuses the existing window and opens the passed file. |
| D-PLAT-7 | The system light/dark appearance is followed; the app ships both themes and a manual override. |
| D-PLAT-8 | macOS: standard window controls, "Quit" closes all documents through the dirty-check flow, and the app does not terminate on last-window-close if documents are open. |

---

## 14. Accessibility and internationalisation

| ID | Requirement |
|---|---|
| D-A11Y-1 | Every interactive control exposes a name, role and value to AccessKit. |
| D-A11Y-2 | Full keyboard operation with a visible focus indicator; focus order follows reading order. |
| D-A11Y-3 | Text respects the system font scaling; layouts do not clip at 200 % scale. |
| D-A11Y-4 | Colour is never the sole carrier of meaning — errors carry text and an icon, selected panes carry a border. |
| D-I18N-1 | All user-facing strings are externalised; English ships first. |
| D-I18N-2 | Number and value rendering is always culture-invariant, matching the engine's output on every locale (this is a parity requirement, not a preference). |

---

## 15. Performance budgets

| ID | Metric | Budget |
|---|---|---|
| D-PERF-1 | Cold start to interactive window | ≤ 400 ms on a 2020-class laptop |
| D-PERF-2 | Keystroke-to-paint latency in a 5 000-line file | ≤ 16 ms p95 |
| D-PERF-3 | Idle memory, one empty document | ≤ 120 MB RSS |
| D-PERF-4 | Memory for a 200 000-digit computation | No hard cap, but a warning above 1 GB and a documented refusal path |
| D-PERF-5 | Tab switch with live engines | ≤ 50 ms |
| D-PERF-6 | Distribution size | ≤ 40 MB unpacked per platform |

---

## 16. How the desktop app is tested

The desktop app is the one place where "port the tests" does not apply, because the tests being
replaced exercise an HTTP surface that no longer exists. Its test strategy is therefore specified
from scratch.

| Layer | Approach |
|---|---|
| Engine integration | Headless tests that drive the same command bus the UI drives, asserting pane-model state — no window, no rendering. |
| Widget behaviour | `egui_kittest`-style harness: build the widget tree, inject input events, assert on the resulting state and on captured snapshots. |
| Editor | Differential tests against a reference text model (buffer, selections, undo stack) plus property tests for edit/undo invariants. |
| Persistence | Round-trip tests over the settings schema, including corrupt-file and future-version cases. |
| Rendering | Golden-image tests for the four panes at two DPI scales, with a bounded pixel-difference tolerance. |
| Manual acceptance | The scenario tests in `## Test Plan` below, run once per release on Windows, macOS and Linux. |

The 22 `Lovelace.Studio.Tests` facts are **not ported**: they assert HTTP routing, session tokens
and JSON DTOs that the desktop app deletes. Their *intent* is carried by T-D-08 (session isolation),
T-D-13 (one coherent evaluate result) and T-D-21 (the structured projection behind the DTOs).
This drop is recorded explicitly in the parity manifest so that it reads as a decision rather than
an omission (see [Lovelace.Rust.Parity.md](Lovelace.Rust.Parity.md), §6).

---

## 17. Risks specific to this document

| ID | Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|---|
| D-R1 | The bespoke editor is the critical path and is underestimated. | High | High | Phase D0 spike with hard thresholds (60 fps @ 5 000 lines, IME, multi-cursor undo) **before** accepting the framework choice; Tauri fallback documented in §3. |
| D-R2 | Immediate-mode GUI IME/clipboard gaps on Windows and Linux make text entry feel wrong. | Medium | High | Spike the IME path on all three platforms; treat any unresolved IME defect as a framework-gate failure equal to the editor threshold. |
| D-R3 | Every pane wanting its own copy of engine state produces a lock-ordering deadlock. | Medium | High | One owner thread per document engine; the UI holds only snapshots published by message. No `Mutex<Engine>` shared with the UI thread. |
| D-R4 | Cancellation mid-statement leaves partially mutated variables. | Medium | High | Cancellation is checked at statement boundaries; a cancelled statement's writes are staged and discarded (the engine's transaction boundary, see Port §5.8). |
| D-R5 | Accessibility promises outrun what AccessKit plus the framework deliver. | Medium | Medium | Verify with a screen reader per platform in the acceptance pass; publish the verified matrix, and state unverified rows as unverified. |
| D-R6 | Plot rasterisation of a very large SVG blocks the frame. | Low | Medium | Rasterise off-thread with a generation counter; drop stale results. |

---

## 18. Completeness checklist

- [ ] All `T-D` tests pass on Windows, macOS and Linux. [mandatory — release gate]
- [ ] Framework gate (Phase D0) passed against the thresholds in §3 and §15. [prerequisite for all UI work]
- [ ] No TCP listener is opened by the binary — verified by port scan during a full run. [mandatory — defines "desktop app"]
- [ ] No browser/WebView runtime is a dependency in the shipped artifact. [mandatory — defines "desktop app"]
- [ ] All twelve Studio capabilities in §1 map to a delivered control. [parity with Lovelace.Studio]
- [ ] Settings schema round-trip and corrupt-file recovery verified. [prerequisite for D-PERSIST-2..4]
- [ ] Accessibility pass recorded with a per-platform verified/unverified matrix. [mandatory — G10]
- [ ] Performance budgets measured and recorded, not estimated. [mandatory — §15]

---

## Test Plan

### Framework gate

1. `EditorScroll_GivenFiveThousandLineFile_MaintainsDisplayRefreshRate`
   *Assumption*: 5 000 lines × 100 characters is a realistic upper bound for a user script in this language.
2. `EditorIme_GivenComposedCjkInput_CommitsComposedTextNotRawKeys`
   *Assumption*: IME composition is a first-class input path on Windows and macOS, not an edge case.
3. `EditorUndo_GivenMultiCursorEdit_UndoesInOneStepPerCursorGroup`
   *Assumption*: users treat a multi-cursor edit as one logical action.

### Layout and persistence

4. `Layout_GivenSavedSplitterSizes_RestoresExactExtentsOnLaunch`
   *Assumption*: splitter sizes are part of the user's working setup, so they must round-trip exactly.
5. `Layout_GivenWindowResizedBelowMinimum_ClampsInsteadOfOverlappingPanes`
   *Assumption*: the window manager may violate the minimum size, so the app must clamp anyway.
6. `Settings_GivenCorruptFile_BacksUpAndStartsWithDefaults`
   *Assumption*: a corrupt config must never be a start-up crash.
7. `Settings_GivenNewerSchemaVersion_RefusesToDowngradeSilently`
   *Assumption*: a user may run an older binary against a newer config; silent field-dropping loses data.

### Documents and sessions

8. `Tabs_GivenTwoDocuments_RunInOneDoesNotMutateVariablesInTheOther`
   *Assumption*: engine isolation is the desktop equivalent of the Studio's per-tab session guarantee.
9. `Tabs_GivenLongRunInTabA_TypingInTabBStaysResponsive`
   *Assumption*: the UI thread must never be joined to an engine thread.
10. `Close_GivenDirtyBuffer_PromptsAndHonoursCancel`
    *Assumption*: "cancel" must abort the close, not merely dismiss the dialog.
11. `Session_GivenClearVariables_KeepsUserFunctions`
    *Assumption*: this matches both the Studio and the REPL, and users rely on it.
12. `Tabs_GivenRestoredLayout_DoesNotResurrectComputedVariables`
    *Assumption*: D-DOC-8 is a documented narrowing, so a test must pin it to prevent silent drift.

### Execution

13. `Run_GivenMultiStatementScript_PublishesOneCoherentUpdate`
    *Assumption*: partial repaints of panes from different statements would show variables a later statement already replaced.
14. `Cancel_GivenLongFactorial_StopsAtNextStatementBoundaryAndKeepsPreviousState`
    *Assumption*: cancellation is between statements (Studio D8), so state is that of the last completed statement.
15. `Run_GivenSecondRunWhileActive_QueuesRatherThanInterleaves`
    *Assumption*: the engine serialises evaluations, so interleaving would be a correctness bug, not a latency trade-off.
16. `Incremental_GivenEditedMiddleLine_RecomputesOnlyDependentStatements`
    *Assumption*: the dependency-aware cache semantics of the Studio are a feature users depend on for iteration speed.
17. `Progress_GivenPiAtThousandDigits_EmitsSubProgressBeforeCompletion`
    *Assumption*: sub-progress is what distinguishes "working" from "hung" for multi-second operations.

### Editor and diagnostics

18. `Diagnostics_GivenAstralPlaneCharacterBeforeError_HighlightsSameOffsetAsCli`
    *Assumption*: the engine reports UTF-16 offsets; the editor must not reinterpret them as chars.
19. `Autocomplete_GivenUserFunction_OffersSignatureWithParameterNames`
    *Assumption*: parameter names are the difference between a useful completion and a guess.
20. `Editor_GivenCrlfFile_SavesSameLineEndingsItLoaded`
    *Assumption*: silent line-ending normalisation produces spurious version-control diffs.

### Workspace, graph, logs

21. `Variables_GivenSymbolicValue_ShowsCanonicalFormNotTruncatedDisplay`
    *Assumption*: the structured projection exists precisely because display strings are lossy.
22. `Plot_GivenLargeSvg_DoesNotBlockFrameWhileRasterising`
    *Assumption*: a plot is frequently produced by the very script the user is iterating on.
23. `Logs_GivenPrintOutput_PreservesExactOrderAndNoAddedTerminators`
    *Assumption*: the CLI already guarantees this, and users compare the two surfaces.

### Desktop-defining invariants

24. `Process_GivenFullRun_OpensNoListeningSocket`
    *Assumption*: "no TCP listener" is the operational definition of the desktop requirement and must be tested, not asserted.
25. `Performance_GivenColdStart_ReachesInteractiveWithinBudget`
    *Assumption*: §15 budgets are measured on the reference hardware class, not extrapolated.

---

*All assumptions derived from the implemented Studio behaviour and its requirements documents; Zero Falsified rows — every claim in §1 was checked against Lovelace.Studio/README.md, wwwroot/index.html, .github/requirements/Lovelace.Studio.Sessions.md and the runner's published protocol.*
