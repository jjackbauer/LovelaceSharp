# Lovelace Desktop — Native Rust Parity & Modern UX Requirements (v4.0)

> **Status:** Proposed normative implementation contract; not evidence that the requirements are already met.
> **Transcribed verbatim** from the user request of 2026-11-12 (sections 0–2). This file is the
> authoritative requirement catalog for `docs/goal-desktop-v4/`. Requirement IDs are stable and may
> not be renumbered or re-worded; deviations are recorded as `DEC` entries with `Owner: user`.
>
> **Source baselines (reviewed 2026-11-12):** [Rust branch](https://github.com/jjackbauer/LovelaceSharp/tree/rust),
> web `Lovelace.Studio/wwwroot/{app.js,index.html,styles.css}`, `Lovelace.Studio/README.md`,
> `Lovelace.Studio/IncrementalRunner.cs`, Rust `rust/lovelace-desktop/src/{app.rs,win32/shell.rs,engine/real.rs,selftest.rs}`,
> and [.github/requirements/Lovelace.Rust.Desktop.md](https://github.com/jjackbauer/LovelaceSharp/blob/rust/.github/requirements/Lovelace.Rust.Desktop.md).

**Product goal:** A modern, genuinely pleasant, self-contained native scientific scripting workbench
built in Rust that preserves every meaningful user-facing capability of the existing JavaScript Studio
and meaningfully improves the desktop ergonomics. A Win32 demo shell that happens to execute scripts
does **not** satisfy this goal.

## 0. Scope, evidence and priority semantics

- **W — Web behavioral parity:** preserve behavior and data semantics, not HTML/REST implementation.
  **D — Desktop experience:** deliberate improvement. **Q — Quality:** cross-cutting correctness,
  usability, or reliability.
- **P0 — required for initial product-quality release**; **P1 — expected for a polished 1.0** (waive
  only with an explicit product decision); **P2 — optional extension**. All P0 gates are blocking.
- Core functions such as editing, completion, progress, cancellation and rendering are **not**
  complete when only their headless state models pass. They require real GUI evidence.
- Use the existing web app as the parity reference, but prefer native abstractions: no localhost HTTP,
  JavaScript, CDN, browser storage, or embedded webview. The Rust desktop owns local documents and
  typed in-process events.
- Do not change language semantics, the C# oracle, fixture expectations, or web Studio behavior to
  make desktop tests pass. Document and escalate unavoidable deviations.

### Verified baseline: what exists and what is missing

| Product capability | JavaScript Studio | Rust desktop currently | Priority finding |
|---|---|---|---|
| Code editor | CodeMirror 6, completion, error-line decoration, caret position | Plain Win32 multiline EDIT in shell; document/undo model exists | Replace editor, P0 |
| Runtime | Asynchronous polling, statement/sub-operation progress, cancel | Shell performs `eval_full` inline on UI thread | Real workers + cancel, P0 |
| Incremental execution | Source hash + dependency tracking + per-session cache | No equivalent desktop incremental runner verified | Port cache semantics, P0 |
| Precision | Session-scoped computation/display, UI setter | Per-document precision exists in core/status, but toolbar control not demonstrated | Visible native control, P0 |
| Tabs/session isolation | One engine per browser tab | One engine per Rust document exists | Finish GUI interactions and concurrent execution, P0 |
| Workspace | Structured variable table; function table; delete variable | Read-only flattened EDIT text; `clear` rebuilds engine and loses functions | Table/view/semantics P0 |
| Results + symbolic inspection | Structured typed values, nested records, canonical, trace, assumptions, MathIR | Basic text log and partial SVG; no equivalent rich inspector verified | Structured inspector P0 |
| Graph | Browser renders actual engine SVG | Limited SVG parser drawn with GDI | Full SVG renderer, P0 |
| Quick eval | One-line console input | Core has QuickEval command; no working shell input demonstrated | Complete native UI, P0 |
| Files | Browser localStorage buffer | Native file open/save, tabs, recent files, settings | Add autosave/restore/conflict safety, P0 |
| Visual design | Dark, four-pane CodeMirror UI | Windows stock controls / GDI styling | Modern design system P0 |
| Automated tests | Web/engine scenarios; API contracts | 15 headless `--selftest` checks | GUI + visual + human usability P0 |

**Architecture discrepancy:** The old Rust desktop requirements chose `egui/eframe` as a target with a
fallback, but the implementation chose an intentionally minimal Win32/GDI shell. Treat the framework
choice as an explicit design decision to revisit using a real editor prototype, rather than blindly
replacing one technology with another.

## 1. Product UX north star

**A quiet technical instrument:** strong typography, a few intentional surfaces, useful whitespace,
excellent keyboard access, instant-feeling interaction, and precise results. Avoid a generic dashboard
of equal-weight cards. Favor the editor for creation, the result/plot for interpretation, and the
workspace/inspector for investigation.

### Recommended information architecture

```text
┌────────────────────────────────────────────────────────────────────────────────┐
│ Lovelace  | File Edit View Run Help     document tabs           — □ ×          │
├────────────────────────────────────────────────────────────────────────────────┤
│  ▶ Run   ■ Cancel  Precision [ 1000 ▾ ]   ASCII/Unicode   Search  Command ⌘P │
├─────────────────────────────────────────────┬──────────────────────────────────┤
│ [tab: script.ls ●]                           │ Workspace                         │
│                                             │ Variables | Functions | Inspector │
│  1  x = symbol("x")                        │  x    Symbolic         x         │
│  2  f = diff(x^3 + 2*x, x)                 │  f    Symbolic         3*x²+2    │
│  3  plot(1..10, (1..10)^2)                │                                  │
│                                             │ [Symbolic]  [Assumptions] [MathIR] │
│                                             │                                  │
├─────────────────────────────────────────────┼──────────────────────────────────┤
│ Results  |  Plot                           │ Details / plot metadata          │
│ [typed result] [interactive zoomable graph]│ [copy/export, precision, traces] │
├─────────────────────────────────────────────┴──────────────────────────────────┤
│ Console ▸ print / errors / timings | Quick eval: [ ...              ]        │
├────────────────────────────────────────────────────────────────────────────────┤
│ ● Ready  rev 12   Ln 3 Col 29     1000 digits  cache: 2 reused  124 ms       │
└────────────────────────────────────────────────────────────────────────────────┘
```

**Layout:** This is an information hierarchy, not a fixed grid. At 900×600 use tabs/drawers for
secondary panes; at large sizes use persistent split views. The editor and results/plot remain
primary, the workspace inspector is discoverable but nonintrusive.

### Design-token proposal (adjust after screenshot/user testing)

| Token | Dark theme starting point | Light theme starting point |
|---|---|---|
| App background | `#15171B` | `#F4F5F7` |
| Panels | `#1D2025` | `#FFFFFF` |
| Editor surface | `#111317` | `#FCFCFD` |
| Raised/header surface | `#24282F` | `#EDEFF3` |
| Primary foreground | `#E8EBF0` | `#1E2229` |
| Secondary foreground | `#ABB2BF` | `#596273` |
| Accent | `#729EFA` | `#315FCA` |
| Positive | `#61CAA0` | `#137855` |
| Error | `#F08282` | `#B52E39` |
| Border | `#363B45` | `#D9DEE6` |

These are **proposed** tokens, not verified accessible colors. A theme generator/validator must check
contrast rather than blindly relying on these starting values. Use restrained 6–10 px corners, a
consistent spacing scale and a compact technical type system. Reserve strong accent color for Run,
focus, selection and active graph controls.

## 2. Normative requirement catalog

### CON — Contract, scope and traceability

| ID | Priority | Type | Requirement | Acceptance / falsification criterion |
|---|---|---|---|---|
| **CON-001** | P0 | W | Declare the authoritative reference surfaces | Pin C# Studio backend, Studio JS/CSS and Rust engine/desktop commit SHAs in a machine-readable baseline; all parity claims name those SHAs. |
| **CON-002** | P0 | W | Maintain a feature-by-feature parity register | Every visible web feature, route-derived behavior and desktop-only behavior has owner, source path, status, test and waiver; omissions fail the audit. |
| **CON-003** | P0 | W | Distinguish behavioral parity from implementation parity | REST and browser storage are replaced by in-process native mechanisms; parity tests compare behavior, not transport or widget technology. |
| **CON-004** | P0 | W | Preserve one authoritative evaluation engine | Desktop commands use the same Rust kernel as Rust runner; no math/parser/precision semantics are reimplemented in widgets or renderer. |
| **CON-005** | P0 | Q | Forbid phantom completion | Release remains blocked when any P0 acceptance test is absent, skipped, stale, unrun or failed, regardless of headless suite count. |
| **CON-006** | P0 | Q | Record intentional deviations | Every non-parity choice has human-approved rationale, expiry/review criterion and evidence; absence is not counted as equivalence. |
| **CON-007** | P0 | W | Adopt independent golden user journeys | Record deterministic reference state/output/diagnostics/plots for the named web Studio journeys and execute corresponding desktop scenarios. |
| **CON-008** | P0 | Q | Separate visual, functional and performance acceptance | Release scorecard reports three independent gates and never substitutes screenshots for semantics or engine unit tests for real-window behavior. |
| **CON-009** | P0 | D | Document supported platforms explicitly | Publish Windows, Linux and macOS target matrix, clearly marking initial release and later ports; no platform claim without a built, run GUI test. |
| **CON-010** | P1 | D | Set quality-first prioritization | Delivery plan prioritizes editor responsiveness, real cancellation, readability, working plots and useful inspection before optional decoration. |

### ARC — Native architecture and concurrency

| ID | Priority | Type | Requirement | Acceptance / falsification criterion |
|---|---|---|---|---|
| **ARC-001** | P0 | D | Choose and validate a native rendering stack | Run a time-boxed spike comparing candidate Rust UI frameworks against real editor, accessibility, text/IME, plotting and docking acceptance scenarios; record choice. |
| **ARC-002** | P0 | D | No browser/runtime dependency | Production desktop has no embedded webview, JavaScript engine or mandatory localhost service; application operates with all network interfaces disabled. |
| **ARC-003** | P0 | Q | Preserve UI-core separation | Headless app state and commands are independent of platform window handles; native renderer consumes state and publishes typed commands. |
| **ARC-004** | P0 | W | Introduce typed event-driven execution | Worker emits RunStarted/StepStarted/StepComputed/StepReused/SubProgress/Output/Plot/Diagnostic/Completed/Cancelled with run and document IDs. |
| **ARC-005** | P0 | W | Never execute evaluation on the UI thread | Even a 30-second script leaves pointer, keyboard, window close, pane resize and animations responsive; automated GUI watchdog verifies. |
| **ARC-006** | P0 | W | Per-document serialized engine mutation | Only one computation mutates a document engine at a time; concurrent requests are rejected or queued with clear UI policy. |
| **ARC-007** | P0 | W | Permit independent documents to run concurrently | With tab A busy, tab B can run and cancel and tab A continues unaffected; no process-wide engine lock. |
| **ARC-008** | P0 | Q | Revision-bound completion and events | Every event has document ID, run ID, source revision and sequence; events from obsolete revisions never overwrite current pane state. |
| **ARC-009** | P0 | W | Cancellation tokens reach long operations | Worker passes a cooperative token into evaluator, high-precision math and symbolic loops; cancellation interrupts a test operation rather than only setting UI state. |
| **ARC-010** | P0 | W | Use structured values end-to-end | UI gets typed numeric, array, record, symbolic and diagnostic projections; display text is not reparsed to infer structure. |
| **ARC-011** | P1 | D | Keep native OS glue isolated | Platform-specific unsafe code stays in small adapters with reviewed invariants; shared UI, workspace, execution and settings logic is safe Rust. |
| **ARC-012** | P1 | Q | Resist unbounded memory and event queues | Progress and output delivery has coalescing/backpressure and bounded storage without losing terminal events, final result or required diagnostics. |

### VIS — Modern visual design system

| ID | Priority | Type | Requirement | Acceptance / falsification criterion |
|---|---|---|---|---|
| **VIS-001** | P0 | D | Define a deliberate visual direction | Product design describes a calm scientific workbench: information-dense, precise, low-noise, professional; final screens are reviewed against reference mockups. |
| **VIS-002** | P0 | D | Ship excellent dark and light themes | Both themes have complete token sets for surfaces, text, borders, focus, selection, syntax, plots and semantic status; no unthemed native child control. |
| **VIS-003** | P1 | D | Use a systematic spacing scale | Layout spacing uses 4/8/12/16/24/32 logical-pixel tokens with documented exceptions; an automated style lint flags raw one-off margins. |
| **VIS-004** | P0 | D | Establish a typography hierarchy | UI uses legible proportional controls and configurable monospaced code/output; titles, labels, table text and metadata have consistent hierarchy. |
| **VIS-005** | P0 | D | Favor legibility over visual effects | No gradients, heavy glass effects or gratuitous animations obscure content; active pane, selection, errors and actions remain legible in both themes. |
| **VIS-006** | P0 | D | Define reusable control states | Every button/input/tab/menu/popup/splitter implements rest, hover, pressed, disabled, selected, focus and error states with consistent interaction. |
| **VIS-007** | P0 | D | Provide semantic color without color dependence | Success, failure, progress, cancelled and warning use text/icon/shape plus color; color is never the only carrier of state. |
| **VIS-008** | P0 | D | Implement meaningful empty states | First launch and empty variables/functions/plots/logs/inspector panes explain what to do next and offer an appropriate next action. |
| **VIS-009** | P0 | Q | Set minimum contrast ratios | Normal text reaches WCAG AA 4.5:1 and large text/non-text UI indicators reach 3:1 in each theme, validated from rendered colors. |
| **VIS-010** | P0 | D | Design unmistakable primary action | Run is discoverable in toolbar and menu; Cancel visibly replaces or accompanies Run while active and communicates progress. |
| **VIS-011** | P1 | D | Make iconography coherent | One consistent icon family, stroke weight and size; icons with ambiguous meaning include visible text or tooltips with shortcut. |
| **VIS-012** | P1 | D | Prefer progressive disclosure | Primary viewport keeps editor and output prominent; secondary metadata, verbose JSON and advanced settings open on demand. |
| **VIS-013** | P1 | D | Make density adjustable | Compact, comfortable and default density modes change row height/padding but preserve layout, accessible hit areas and typography hierarchy. |
| **VIS-014** | P1 | D | Specify polished transient feedback | Success uses subtle confirmation; errors are near their cause; progress never uses indefinite spinner where measured work exists. |
| **VIS-015** | P1 | Q | Respect motion preferences | Animation is modest (typically 120–200 ms) and reduced-motion setting removes nonessential motion while maintaining feedback. |
| **VIS-016** | P0 | D | Ensure coherent responsive window design | The 900×600, 1280×800 and 1920×1080 layouts remain usable without clipping, accidental overlays or illegible content. |
| **VIS-017** | P1 | D | Provide consistent platform chrome | Title, toolbar, menus and window controls feel native while retaining one cross-platform product identity; no hand-drawn imitation of OS close buttons. |
| **VIS-018** | P0 | D | Design a usable first-run experience | Fresh launch presents working example/Open File/New Script affordances plus a concise keyboard hint; user can run first script in at most two actions. |

### LAY — Layout, panes and navigation

| ID | Priority | Type | Requirement | Acceptance / falsification criterion |
|---|---|---|---|---|
| **LAY-001** | P0 | W | Make the editor the primary surface | At default 1280×800 editor receives at least ~45% width and clear visual priority without making results/plot inaccessible. |
| **LAY-002** | P0 | W | Provide complete Studio pane parity | Visible reachable Editor, Workspace, Graph, Logs and Symbolic Inspector; each has a titled pane and its own empty/error states. |
| **LAY-003** | P0 | W | Support draggable splitters | Mouse and keyboard resize all adjacent panes with min sizes; double click or command restores sensible defaults. |
| **LAY-004** | P0 | W | Persist pane sizes and visibility | After close/relaunch the layout matches user choices including hidden/maximized panes, unless screen geometry makes that unsafe. |
| **LAY-005** | P1 | D | Support focus mode | User can maximize editor or graph without losing document state; one action returns to previous layout. |
| **LAY-006** | P1 | D | Provide layout presets | At least Standard, Explore (larger plot) and Focus (larger editor) presets, with reversible user customization. |
| **LAY-007** | P2 | D | Allow dock/undock where supported | Workspace, graph and inspector may be rearranged or detached without losing state; optional feature cannot block core parity. |
| **LAY-008** | P0 | W | Offer an unobtrusive status strip | Status always reports active document, revision, precision, cursor line/column, run state and recent elapsed time without truncating essential fields. |
| **LAY-009** | P0 | Q | Make keyboard pane navigation predictable | Ctrl/Cmd+Tab switches documents; a documented shortcut rotates focus between panes with visible focus. |
| **LAY-010** | P1 | D | Minimize mode-specific confusion | Graph and Inspector clearly indicate which run/document produced current content; no silent cross-tab displays. |
| **LAY-011** | P0 | D | Handle narrow windows gracefully | At minimum supported size ancillary panes collapse to tabs or drawer rather than shrink editor below usable width. |
| **LAY-012** | P1 | D | Keep action locations stable | Run, Cancel, precision, search, tab close and pane visibility retain consistent placement across layouts. |

### EDT — Code editor parity and professional editing

| ID | Priority | Type | Requirement | Acceptance / falsification criterion |
|---|---|---|---|---|
| **EDT-001** | P0 | W | Replace multiline Win32 EDIT as the primary code editor | Real editor supports rich text decorations, gutter, completions, line diagnostics and virtualization; no plain textarea/EDIT as release fallback. |
| **EDT-002** | P0 | W | Implement Lovelace grammar highlighting | Numbers, exact rationals, strings/interpolation, keywords, operators, function definitions, builtins, delimiters, brackets and symbolic tokens highlight contextually. |
| **EDT-003** | P0 | W | Offer live context-aware completion | Completion includes keywords, registered builtins, signatures, user functions, live variables and snippets from active engine; values update after execution. |
| **EDT-004** | P0 | D | Display signatures and argument help | Typing `sqrt(` or `compile(` shows signature and active parameter; documentation is from registered metadata, never duplicate hardcoded text. |
| **EDT-005** | P1 | D | Support fast fuzzy filtering | Completion/palette search ranks exact prefix first and tolerates useful nonprefix matches without blocking typing. |
| **EDT-006** | P0 | W | Correct error underlining and positioning | Errors show underlines and gutter markers at exact reported range with hover tooltip and keyboard navigation, including UTF-16 astral cases. |
| **EDT-007** | P0 | D | Show current line and matching braces | Caret line has subtle emphasis; matching `()[]{}` highlighted with clear unmatched indicator. |
| **EDT-008** | P0 | D | Implement indentation and formatting basics | Indent/outdent selection, smart Enter, auto-close configurable brackets, tab-width and tabs/spaces setting work with the language grammar. |
| **EDT-009** | P0 | D | Support full find and replace | Find, replace, replace-all, case-sensitive, whole-word and optional regex search are reversible and highlight all matches. |
| **EDT-010** | P0 | D | Provide navigation primitives | Go to line, go to matching bracket, find symbol in file and keyboard document navigation function without pointer input. |
| **EDT-011** | P1 | D | Implement multi-cursor and selection essentials | At least multiple carets, select next match and column selection; undo remains coherent across all cursors. |
| **EDT-012** | P0 | W | Provide robust undo/redo semantics | Burst typing is coalesced; selection/caret positions restored; all editing actions, replacements and auto-format are undoable per document. |
| **EDT-013** | P0 | W | Preserve file encoding and newline policy | Open/save UTF-8, BOM handling and CRLF/LF round-trip without unrequested rewrites; clearly warn on unsupported encoding. |
| **EDT-014** | P0 | Q | Prevent Unicode offset corruption | Diagnostic offsets, selections and source ranges correctly convert UTF-16 <-> Rust byte/grapheme positions on emoji, combining marks and newlines. |
| **EDT-015** | P0 | W | Show accurate line and column | Always 1-based source line/column (as in web Studio); moving cursor and selecting text updates promptly. |
| **EDT-016** | P0 | Q | Support large files without freezing | A 500 KB/5,000-line script supports 60 Hz scroll target on reference hardware and typing input p95 under 16 ms; measure in GUI. |
| **EDT-017** | P0 | Q | Use IME and international input correctly | Compose accented and East-Asian text without losing composition, double committing or invalidating undo; platform UI test required. |
| **EDT-018** | P1 | D | Offer editor zoom and font control | Ctrl/Cmd plus/minus adjusts editor font size; cursor, selection and diagnostics remain aligned and readable. |
| **EDT-019** | P1 | D | Add code-aware context actions | Right-click or command palette supports Run selection/line where valid, Copy, Paste, Go to line and Show help. |
| **EDT-020** | P1 | D | Show modified lines and run markers | Gutter distinguishes dirty source, computed/reused statement and error using consistent icon/tooltip, never replacing actual diagnosis. |
| **EDT-021** | P1 | D | Avoid unsolicited intrusive autocomplete | Completions dismiss via Esc, preserve text/selection, and do not hijack typing or cover the active line for long. |

### DOC — Documents, files and session model

| ID | Priority | Type | Requirement | Acceptance / falsification criterion |
|---|---|---|---|---|
| **DOC-001** | P0 | W | Document-tab isolation | Every tab owns buffer, engine, variables, functions, precision, revision, cache, diagnostics and last figure; tests show no leakage. |
| **DOC-002** | P0 | D | Implement standard native file lifecycle | New/Open/Save/Save As/Close use native dialogs; unmodified file round-trips byte-identically under chosen newline and encoding policy. |
| **DOC-003** | P0 | D | Prompt on unsaved work | Close tab, close window and Quit offer Save/Discard/Cancel; Cancel leaves buffer and focus untouched. |
| **DOC-004** | P0 | D | Support recent files and reopen session | Recent files list deduplicates and removes missing paths; relaunch restores tab order/path and active tab. |
| **DOC-005** | P0 | D | Autosave unsaved buffers for crash recovery | Edits are journaled atomically to a recovery directory; forced process termination then restart offers intact recovery before overwriting anything. |
| **DOC-006** | P0 | D | Detect external file changes | External edit while tab open offers Reload/Compare/Keep; silent overwrite is forbidden. |
| **DOC-007** | P1 | D | Enable drag and drop opening | Dropping one or more .ls files opens tabs; duplicates focus existing tab unless user requests another independent session. |
| **DOC-008** | P0 | W | Keep editor buffer separate from runtime state | Loading or editing a file never automatically executes code; each run explicitly targets a source snapshot. |
| **DOC-009** | P0 | W | Preserve per-document precision and formatting settings | Two tabs can have different precision and ASCII/Unicode preferences; selection and process restart do not cause cross-talk. |
| **DOC-010** | P0 | Q | Clarify engine-state restart policy | On restart, restore files/unsaved text/layout; any nonrestored computed variables/functions are explicitly shown as reset until rerun. |
| **DOC-011** | P1 | D | Implement reset session safely | Reset Engine is a separately labeled action with clear consequence; clearing variables alone does not reset functions. |
| **DOC-012** | P1 | D | Support tab reorder and close shortcuts | Mouse drag reorder and Ctrl/Cmd+W, Ctrl/Cmd+Tab and middle-click behave consistently. |
| **DOC-013** | P1 | D | Handle deleted/renamed files | An open file renamed/deleted externally remains editable and warns before save; original data is never silently lost. |
| **DOC-014** | P0 | Q | Version persisted state | Settings and recovery documents carry schema version and are migrated or backed up with warning; corruption never crashes startup. |

### RUN — Real asynchronous execution, progress and cancellation

| ID | Priority | Type | Requirement | Acceptance / falsification criterion |
|---|---|---|---|---|
| **RUN-001** | P0 | W | Run whole script asynchronously | Ctrl/Cmd+Enter starts a background run and reports running state within 100 ms even when evaluation lasts 30 seconds. |
| **RUN-002** | P0 | W | Support quick evaluation | Logs input executes a one-liner in active document engine, appends result, and does not change editor source. |
| **RUN-003** | P0 | W | Render all committed outputs coherently | After each committed statement, variables, functions, log output, plots, revision and diagnostics reflect the same coherent engine snapshot. |
| **RUN-004** | P0 | W | Expose step-level progress | UI shows completed/total statements, current label and reused count, with stable progress and cancellable status. |
| **RUN-005** | P0 | W | Expose numeric subprogress | Long sqrt/pi/factorial or comparable operations publish human-readable phase and fraction; progress stays meaningful and monotonic where defined. |
| **RUN-006** | P0 | W | Implement working cancellation | Cancelling a real long-running script ends work cooperatively, returns UI to idle and never requires terminating the application. |
| **RUN-007** | P0 | Q | Specify cancellation state semantics | Contract defines which completed statements remain committed and what happens to in-flight statement; tests compare with web Studio contract. |
| **RUN-008** | P0 | Q | Use bounded cancellation latency | Representative cancellation-aware loops stop within 2 s after request on reference hardware; uninterruptible kernels report why and are budgeted. |
| **RUN-009** | P0 | W | Distinguish statuses | Queued, Running, Cancelling, Completed, Cancelled, Failed and Stale states have distinct labels and terminal events. |
| **RUN-010** | P0 | W | Reject duplicate active runs per tab | Pressing Run during active run cannot corrupt engine; explicit queue/reject behavior is observable. |
| **RUN-011** | P0 | W | Allow multi-tab concurrency | Tab A long calculation does not prevent tab B execution or cancellation. |
| **RUN-012** | P0 | Q | Protect against stale results after source edits | Result event includes snapshot revision; editing while run active never falsely presents stale result as current; user can inspect old result with label. |
| **RUN-013** | P0 | W | Preserve partial results safely | On error or cancellation, completed-statement workspace and outputs remain consistent; no half-written variable or duplicate print line. |
| **RUN-014** | P0 | W | Present structured errors and recovery | Diagnostics display code, category, message, line, column and suggestions; user can jump to source and retry after correction. |
| **RUN-015** | P1 | W | Expose per-statement timing history | Computed/reused modes, elapsed times and total run time are inspectable without rerunning and without conflating cached timing with work performed. |
| **RUN-016** | P0 | Q | Prevent UI starvation from output floods | 10,000 emitted log events do not freeze editor; visible log updates batch without changing log order or final integrity. |
| **RUN-017** | P0 | Q | Handle engine panics and worker failures gracefully | A simulated worker failure produces recoverable error state, releases run lock and preserves buffer/recovery files; process remains usable. |
| **RUN-018** | P1 | D | Make progress feedback informative rather than noisy | Progress surfaces a compact inline indicator for short jobs and an expandable detail view for long jobs, without modal interruption. |
| **RUN-019** | P0 | W | Expose precision controls on the execution bar | One visibly labeled input sets computation/display precision for the active session; invalid values fail locally with clear guidance. |
| **RUN-020** | P0 | W | Synchronize script-set precision | A `setprecision(n)` statement updates toolbar, engine, cache invalidation and affected value presentation consistently. |

### INC — Incremental execution and cache correctness

| ID | Priority | Type | Requirement | Acceptance / falsification criterion |
|---|---|---|---|---|
| **INC-001** | P0 | W | Port the web incremental strategy | Top-level statement slicing, normalized source hashes and per-document computation cache follow reference behavior; no naïve whole-file rerun substitute. |
| **INC-002** | P0 | W | Track variable and function read dependencies | Editing one variable definition recomputes dependent statements transitively, while independent statements report reused status. |
| **INC-003** | P0 | W | Track user-function revisions transitively | Redefining a function invalidates all call sites and dependent calculations, including functions that call that function. |
| **INC-004** | P0 | W | Avoid caching side effects | `print`, `plot`, `setprecision` and other effectful calls rerun rather than replaying stale side effects. |
| **INC-005** | P0 | W | Invalidate correctly when precision changes | Precision-dependent cache keys include compute settings; a precision change cannot reuse an old low-precision result. |
| **INC-006** | P0 | W | Use exact full-value signatures | Cache dependency hashes derive from full canonical typed value, never truncated display or formatted tooltip. |
| **INC-007** | P0 | W | Preserve code order and scope semantics | Reused assignments reproduce correct downstream visible state without leaking function-local or block-local variables. |
| **INC-008** | P0 | Q | Report reuse honestly | Progress and diagnostics report computed/reused count from actual cache hits; instrumentation can falsify the claim. |
| **INC-009** | P1 | D | Offer an explicit full rerun option | Run All Fresh clears only execution memoization for active tab and yields the same values/output as incremental run. |
| **INC-010** | P0 | Q | Test edit/rerun metamorphic equivalence | After random edits and cache invalidations, incremental output equals a fresh-run engine snapshot for the identical script and settings. |
| **INC-011** | P1 | Q | Bound cache growth | Cache uses documented memory limits and deterministic eviction; cache eviction never changes results. |

### WKS — Workspace variables, functions and registry

| ID | Priority | Type | Requirement | Acceptance / falsification criterion |
|---|---|---|---|---|
| **WKS-001** | P0 | W | Provide sortable variables table | Show name, kind and display value for all globals including `_`, sorted by ordinal name by default. |
| **WKS-002** | P0 | W | Provide function table with source distinctions | Show name, parameters, builtin/user/plugin origin and summary when available; counts agree with current engine registry. |
| **WKS-003** | P0 | W | Delete one variable without side effects | Per-row delete removes only selected variable, updates revision/completions and invalidates affected cache; functions unchanged. |
| **WKS-004** | P0 | W | Clear variables while preserving functions | Clear Variables empties globals but retains user-defined functions, matching Studio; regression test fails current rebuild-engine workaround. |
| **WKS-005** | P1 | D | Filter and search workspace | Text search filters tables and supports keyboard focus, clear and match navigation; large registries stay responsive. |
| **WKS-006** | P0 | D | Show type-aware value preview | Numbers, matrices, complex, record and symbolic values render readable previews; detailed content opens without truncating in memory. |
| **WKS-007** | P1 | D | Copy multiple value formats | Copy display, canonical and JSON/structured data when available; copy preserves exact numerical digits. |
| **WKS-008** | P1 | D | Provide context menus for functions | Function rows offer signature, help, insert call/example and user-function source navigation where available. |
| **WKS-009** | P0 | Q | Never show stale document state | Changing active tab switches workspace and plot to matching engine in same repaint, even during another tab’s run. |
| **WKS-010** | P1 | D | Show state origin and revision | Workspace title indicates active tab, revision and whether values are live, cached or from a previous run. |
| **WKS-011** | P1 | D | Make long values explorable | Huge integers and symbolic expressions use selectable scrolling/copy rather than forcing entire value into table cell or trimming source. |

### RES — Structured results and exploration

| ID | Priority | Type | Requirement | Acceptance / falsification criterion |
|---|---|---|---|---|
| **RES-001** | P0 | W | Render structured DTOs directly | Recursive typed renderer uses engine structured values and does not reconstruct arrays or records by parsing pretty strings. |
| **RES-002** | P0 | W | Present exactness/provenance clearly | Where supplied, show Natural/Integer/Real/Complex kind, exact/approximate, precision and truncation reasons without suggesting unproven exactness. |
| **RES-003** | P0 | W | Present arrays and matrices as spatial structures | N-dimensional shapes and element kinds are visible; 2-D matrices use readable rows/columns and large arrays virtualize. |
| **RES-004** | P0 | W | Expand nested records progressively | Solve, inverse, compile, limit and transform records can be expanded/collapsed by field without data loss. |
| **RES-005** | P0 | W | Show canonical and pretty expressions | Symbolic result has visible pretty text and optionally expandable canonical representation, domain, free symbols and node count. |
| **RES-006** | P0 | Q | Avoid duplicate evaluation for inspection | Selecting result detail uses captured value and revision; changing source after run does not silently recompute inspected value. |
| **RES-007** | P1 | D | Allow exact-copy/export | Numeric values can be copied/exported at full precision even when display is clipped for readability. |
| **RES-008** | P1 | D | Expose diagnostic detail drilldown | Error detail shows code, category, exact message, source location and optional structured signature/hints; no opaque generic failure. |
| **RES-009** | P1 | D | Provide breadcrumbs for nested structures | Record/array explorer indicates path (e.g. result.solutions[1].bindings) and allows backtracking without resetting expansion. |
| **RES-010** | P0 | Q | Keep huge result rendering bounded | Rendering a million-digit result or 100k matrix cells neither blocks UI nor allocates one widget per element. |
| **RES-011** | P1 | D | Separate results from logs | Dedicated Results view (or pinned inspector) keeps the last typed result while Logs displays chronological events; toggling is one click. |

### SYM — Symbolic inspection, MathIR and explanation

| ID | Priority | Type | Requirement | Acceptance / falsification criterion |
|---|---|---|---|---|
| **SYM-001** | P0 | W | Match web inspector information | Display pretty, canonical, expression tree, current assumptions, simplification trace and MathIR, where engine supplies them. |
| **SYM-002** | P0 | W | Honor result origin | Inspector operates on last committed result from active document, not on current unsaved editor text or a background fresh evaluation. |
| **SYM-003** | P0 | D | Make expression tree navigable | Tree supports expand/collapse, node type, child order, copy node canonical form, and keyboard traversal. |
| **SYM-004** | P0 | W | Show conditions and assumptions structurally | Nonzero/domain/branch conditions are typed symbolic relations, distinguishable from plain text notes. |
| **SYM-005** | P0 | W | Expose proof and rewrite steps | Trace displays rule identifier, before/after, classification, required conditions and applicability without hiding conditional rewrites. |
| **SYM-006** | P0 | Q | Visually distinguish incomplete results | Partial, Unevaluated and conditional results cannot be presented as fully Solved/Exact; explicit status near the result. |
| **SYM-007** | P0 | W | Make MathIR legible | IR view is monospaced, scrollable, selectable with syntax-like instruction highlighting, copy and export. |
| **SYM-008** | P1 | D | Correlate related representations | Selecting a symbolic subtree highlights corresponding pretty/canonical section when mappings are available, otherwise no misleading link. |
| **SYM-009** | P0 | W | Handle unsupported inspect paths honestly | An expression that cannot lower to MathIR shows a specific reason and leaves other inspector sections usable. |
| **SYM-010** | P1 | D | Provide context-aware symbolic actions | Offer Copy canonical, Insert pretty, Copy MathIR and inspect assumptions with clear action labels; no silent script execution. |
| **SYM-011** | P0 | Q | Avoid rendering giant symbolic trees eagerly | Tree initially loads bounded depth and virtualizes deeper views; expand remains responsive for >=10k nodes. |

### PLT — Plots and graph interaction

| ID | Priority | Type | Requirement | Acceptance / falsification criterion |
|---|---|---|---|---|
| **PLT-001** | P0 | W | Match full supported Studio plot output | Render all SVG elements/attributes actually emitted by Lovelace plotting, including text, polyline, path, clipping and labels; screenshot comparison verifies. |
| **PLT-002** | P0 | W | Replace partial GDI SVG interpretation | Current minimal polyline/text parser is not accepted as SVG parity; use a complete supported SVG renderer and document supported feature set. |
| **PLT-003** | P0 | Q | Render crisp plots at HiDPI | At 100/150/200% scale, strokes, labels and mathematical text remain sharp and correctly positioned with no clipping. |
| **PLT-004** | P0 | D | Provide intuitive zoom/pan/fit | Wheel/buttons zoom around pointer, drag pans, one-click Fit restores full plot and 100% reset is available. |
| **PLT-005** | P0 | W | Expose plot provenance | Title and active document/run identity appear with plot; opening other tab never silently shows another tab’s chart. |
| **PLT-006** | P0 | D | Support SVG and PNG export | Save SVG preserves engine bytes; PNG export supports chosen dimensions/scaling and transparent/opaque background options. |
| **PLT-007** | P1 | D | Support clipboard copy | Copy Image produces usable bitmap; Copy SVG produces vector data where OS supports it. |
| **PLT-008** | P0 | Q | Handle invalid SVG safely | Malformed/oversized SVG produces bounded, informative failure without crashes, file access or network loads. |
| **PLT-009** | P0 | W | Persist last plot per active document | After successful run plot stays inspectable until replaced/reset and is isolated between tabs. |
| **PLT-010** | P1 | D | Support mouse and keyboard inspection | Plot focus offers keyboard zoom/pan and readable point/cursor values when plot data supports it. |
| **PLT-011** | P0 | D | Provide sensible no-plot and error states | Graph pane says No plot yet or Plot unavailable with reason and next action; no blank white mystery pane. |
| **PLT-012** | P0 | Q | Keep rendering off UI thread for heavy SVG | Complex plot rasterization occurs in worker with cancellation and cached zoomable bitmap/vector scene; GUI remains responsive. |
| **PLT-013** | P2 | D | Allow plot history within a session | Optional mini-gallery allows compare/return to previous plots with source and timestamp; feature cannot gate web parity. |

### LOG — Console, logs and feedback

| ID | Priority | Type | Requirement | Acceptance / falsification criterion |
|---|---|---|---|---|
| **LOG-001** | P0 | W | Capture print output exactly once | Repeated polls/events, reruns and cancellation never duplicate print lines or change order; byte-sensitive tests cover line endings. |
| **LOG-002** | P0 | W | Show chronological typed event lines | Logs distinguish echo, stdout, result, diagnostic, timing and progress with readable icons, text and timestamps optional. |
| **LOG-003** | P0 | W | Include quick eval input | Enter executes active session command and clears input only after accepting request; previous commands are navigable with keyboard. |
| **LOG-004** | P0 | W | Keep errors actionable | Click diagnostic opens and focuses source, highlights position, and provides copyable full error details. |
| **LOG-005** | P0 | D | Keep logs selectable and copyable | Copy selected, copy all, clear current view and save log actions work with keyboard and context menu. |
| **LOG-006** | P0 | Q | Avoid truncating significant output silently | When storage cap is reached show truncated-history indicator and offer export of available full run log if configured; never hide current fatal error. |
| **LOG-007** | P1 | W | Use human-readable timings | Display total and per-statement elapsed with units; cached runs identify reused count and avoid implying reused output was recomputed. |
| **LOG-008** | P1 | D | Provide optional log filtering | Filter by errors, outputs, run and document without changing stored event order. |
| **LOG-009** | P1 | D | Make run completion unobtrusive | Success/fail/cancel indication visible without intrusive modal; long-running jobs receive optional desktop notification only by user opt-in. |
| **LOG-010** | P0 | Q | Keep performance stable under log load | A run with 100k log records remains scrollable via virtualization and bounded live updates; no quadratic text-control replacement. |

### A11Y — Accessibility, keyboard ergonomics and internationalization

| ID | Priority | Type | Requirement | Acceptance / falsification criterion |
|---|---|---|---|---|
| **A11Y-001** | P0 | Q | Provide full keyboard navigation | All commands and pane actions accessible without mouse; an automated keyboard journey completes primary tasks. |
| **A11Y-002** | P0 | Q | Support screen reader roles and names | Windows UIA and corresponding platform accessibility trees expose name/role/value/state for editor, tabs, tables, buttons and progress. |
| **A11Y-003** | P0 | D | Show strong keyboard focus | Every focusable control has visible 3:1 focus indicator; focus never becomes invisible in dark or light mode. |
| **A11Y-004** | P0 | D | Respect system text scaling | 125–200% DPI/text scale retains readable buttons/labels and usable layout without clipped content. |
| **A11Y-005** | P0 | D | Provide adjustable editor typography | Users can configure mono font and 12–24 pt range, line-height and optionally ligatures; all decorations remain aligned. |
| **A11Y-006** | P0 | D | Support light/dark system integration | OS theme change updates controls and plot chrome without losing user focus, run or document state. |
| **A11Y-007** | P0 | D | Provide a keyboard shortcut reference | Help > Keyboard Shortcuts and command palette display all global and editor shortcuts. |
| **A11Y-008** | P0 | Q | Avoid conflicts with IME and screen readers | Text composition and shortcut accelerators do not steal native IME shortcuts or screen-reader navigation. |
| **A11Y-009** | P1 | D | Offer high-contrast mode | High contrast appearance uses semantic tokens rather than merely raising window brightness; contrast validated. |
| **A11Y-010** | P1 | Q | Honor reduced motion | OS reduced-motion preference disables decorative animations while retaining progress values and completion feedback. |
| **A11Y-011** | P1 | D | Ensure minimum pointer target | Most interactive targets are >=32×32 logical pixels; compact density exceptions preserve alternative keyboard access. |
| **A11Y-012** | P0 | Q | Make errors independent of color | Errors include icon, text and location; color-blind tests identify errors and successes without hue. |
| **A11Y-013** | P1 | D | Use clear accessible language | Controls and errors use concise, predictable text; abbreviations have tooltips; accessible description includes action and consequence. |
| **A11Y-014** | P0 | W | Support unicode mathematical glyph rendering | ASCII/Unicode setting updates structured result, workspace and logs consistently; missing glyphs fall back without tofu squares. |

### PLF — OS integration, security and packaging

| ID | Priority | Type | Requirement | Acceptance / falsification criterion |
|---|---|---|---|---|
| **PLF-001** | P0 | D | Support native file dialogs and clipboard | Open/Save/Save As, paste and clipboard integration work via OS APIs with expected keyboard shortcuts. |
| **PLF-002** | P0 | D | Define and honor offline-first policy | App launches, evaluates, plots and opens files with network disabled; no HTTP listener or surprise external requests. |
| **PLF-003** | P0 | Q | Never load untrusted plot resources | Embedded SVG external references and active content do not trigger network/file reads or script execution. |
| **PLF-004** | P0 | Q | Protect against dangerous file overwrite | Saving to existing changed file prompts before overwrite; atomic temp-write/rename and recovery preserve original on failure. |
| **PLF-005** | P0 | D | Package correct assets locally | Fonts fall back to OS-installed fonts; syntax/theme/icon assets are bundled or resolved offline; no CDN at runtime. |
| **PLF-006** | P0 | D | Use app-scoped settings locations | Windows APPDATA, Linux XDG, macOS Application Support are used; no writes to installed binary directory by default. |
| **PLF-007** | P1 | D | Provide installer/release artifacts | Signed or notarized where applicable, versioned binaries for supported OS/arch, reliable uninstall and OS integration. |
| **PLF-008** | P1 | D | Integrate file associations safely | Opening a .ls file from shell launches or focuses desktop with that file, without unexpected execution. |
| **PLF-009** | P0 | Q | Expose clear error on unsupported platform | A platform without UI support is explicitly marked unsupported by release metadata, not silently shipped as headless placeholder. |
| **PLF-010** | P0 | Q | Keep platform-specific behavior tested | Keyboard modifiers, dialogs, caret/IME, clipboard and DPI tested on every advertised platform. |
| **PLF-011** | P1 | D | Provide local diagnostics bundle | User can export redacted app version/config/logs for bug reports without private source content unless explicitly opted in. |

### PERF — Responsiveness, memory and reliability budgets

| ID | Priority | Type | Requirement | Acceptance / falsification criterion |
|---|---|---|---|---|
| **PERF-001** | P0 | Q | Set cold-start budget | On reference machine cold start to interactive editor p95 <= 2.0 s and warm start <= 1.0 s; measure signed release binary. |
| **PERF-002** | P0 | Q | Set UI input-latency budget | Typing/caret movement p95 <= 16 ms and no input freeze >100 ms under normal work, measured with a live GUI profiler. |
| **PERF-003** | P0 | Q | Set scrolling frame budget | 500 KB/5k-line document maintains >=55 fps p95 scrolling on reference desktop; 10k-line stress run is explicitly reported. |
| **PERF-004** | P0 | Q | Set run-start latency budget | Run button/shortcut acknowledges within 100 ms; worker scheduling not blocked by rendering or queued logs. |
| **PERF-005** | P0 | Q | Set cancel acknowledgement budget | Cancel action updates visible state within 100 ms and invokes token immediately; termination meets RUN cancellation bound for cooperative workloads. |
| **PERF-006** | P1 | Q | Set workspace update budget | Updating 10k variable/function rows takes <=200 ms to visible state via diff/virtualization, without recreating all children. |
| **PERF-007** | P1 | Q | Set plot rendering target | Typical SVG <100 KB fits pane in <=250 ms after receipt; complex >1 MB remains interactive and shows pending state. |
| **PERF-008** | P1 | Q | Set idle resource target | Idle process does not continuously poll at high rate or consume >2% of one reference CPU core; measure with window open. |
| **PERF-009** | P0 | Q | Avoid quadratic undo and text copying | 10k typed characters do not cause unbounded full-buffer clones per keystroke; memory/CPU profiling confirms acceptable scaling. |
| **PERF-010** | P0 | Q | Recover gracefully from corrupt settings | Invalid config and recovery files produce warning/default/backup, never launch crash or unrecoverable data loss. |
| **PERF-011** | P0 | Q | Monitor resource budgets for enormous math | Extreme precision requests receive estimated memory/time warning or budget refusal; no uncontrolled process abort for documented refusal classes. |
| **PERF-012** | P0 | Q | Preserve results during paint failures | Renderer crash or invalid plot does not corrupt engine/document; recoverable error surfaces and next run still works. |

### TST — Automated and human-centered acceptance

| ID | Priority | Type | Requirement | Acceptance / falsification criterion |
|---|---|---|---|---|
| **TST-001** | P0 | W | Maintain a golden web-to-desktop journey suite | Each reference journey has input, action sequence, expected state/events/plots, desktop driver and a reproducible result artifact. |
| **TST-002** | P0 | Q | Run actual native GUI end-to-end tests | CI or dedicated test VM launches a window and drives pointer/keyboard; headless AppState selftest alone cannot satisfy GUI gate. |
| **TST-003** | P0 | D | Include screenshot visual regression | Baseline screenshots at 900×600, 1280×800, 1920×1080 and 100/150/200% scale reviewed for both themes. |
| **TST-004** | P0 | W | Test real cancellation with a slow kernel | GUI test cancels a script while worker is inside lengthy computation; asserts UI responsiveness, state consistency and worker termination. |
| **TST-005** | P0 | W | Test concurrent tab isolation | Two tabs execute with different precision and variables concurrently; snapshots and plots never cross, and tab B completes while A runs. |
| **TST-006** | P0 | W | Test incremental vs fresh equivalence | Random edit sequences compare incremental run snapshot to fresh-run reference across dependencies, functions, effects and precision. |
| **TST-007** | P0 | W | Test editor and diagnostic UTF-16 mapping | Emoji, combining marks, CRLF, multiline, nested scopes and parse errors highlight exact location with correct accessible announcement. |
| **TST-008** | P0 | W | Test structured projections | Golden outputs cover exact fractions, 1k-digit numbers, matrix/inverse, solve records, MathIR, symbols, conditions and truncated prints. |
| **TST-009** | P0 | W | Test SVG fidelity | Reference plot corpus rasterized in both renderers; text, strokes and clip regions compared with documented pixel tolerance and manual review. |
| **TST-010** | P0 | Q | Test file recovery and corruption | Crash during dirty buffer edit and interrupted save are replayed; user data remains recoverable and app relaunches cleanly. |
| **TST-011** | P0 | Q | Test screen reader and keyboard flows | One Windows UIA screen reader and one other supported platform accessibility path are exercised; all primary actions reachable. |
| **TST-012** | P0 | D | Run human usability sessions | At least 5 target users complete 8 scripted tasks; >=90% task success, median <2 min for common tasks, and document qualitative friction. |
| **TST-013** | P1 | D | Use a usability quality threshold | Target System Usability Scale >=80 in pilot, with no unresolved severity-1 usability finding at launch. |
| **TST-014** | P0 | D | Audit layout/content across DPI and text scaling | Visual tests at 100/125/150/200% DPI plus system text enlargement identify no overlapping/clipped primary controls. |
| **TST-015** | P0 | Q | Validate accessibility contrast automatically | Theme token checker verifies normal text >=4.5:1 and large/control indicators >=3:1; exemptions documented. |
| **TST-016** | P0 | Q | Require clean release provenance | Artifact manifest records OS, build SHA, Rust toolchain, source hash, GUI test results, screenshot bundle and test fixture checksums. |
| **TST-017** | P0 | Q | Make unsupported features visible, not silently skipped | Feature register distinguishes PASS, FAIL, BLOCKED, NOT_TESTED, APPROVED_DEVIATION; missing evidence blocks final parity. |
| **TST-018** | P1 | Q | Track objective usability regressions | Continuous measurements compare startup/input/frame metrics and task completion across releases, with declared regression thresholds. |

## 3. Implementation architecture and data contracts

**Preferred seams (framework-independent):**

```rust
struct DocumentId(u64);
struct RunId(u64);
struct SourceRevision(u64);

enum DesktopCommand {
    Run { document: DocumentId, revision: SourceRevision },
    Cancel { document: DocumentId, run: RunId },
    QuickEval { document: DocumentId, expression: String },
    SetPrecision { document: DocumentId, digits: usize },
    RemoveVariable { document: DocumentId, name: String },
    ClearVariables { document: DocumentId },
    InspectResult { document: DocumentId, run: RunId },
}

enum DesktopEvent {
    RunStarted { document: DocumentId, run: RunId, revision: SourceRevision, total: usize },
    StepStarted { document: DocumentId, run: RunId, index: usize, label: String },
    SubProgress { document: DocumentId, run: RunId, fraction: f32, label: String },
    StepCommitted { document: DocumentId, run: RunId, index: usize, reused: bool },
    Output { document: DocumentId, run: RunId, sequence: u64, text: String },
    Snapshot { document: DocumentId, run: RunId, state: WorkspaceSnapshot },
    Finished { document: DocumentId, run: RunId, result: StructuredResult },
    Cancelled { document: DocumentId, run: RunId, committed_steps: usize },
    Failed { document: DocumentId, run: RunId, diagnostic: StructuredDiagnostic },
}
```

**Important:** Illustrative interface, not an implementation demand. Source revision and monotonically
increasing sequence are required for stale-result protection and deduplication. The engine owns
structured values and semantic errors; UI renders them. The worker owns evaluation, not any window
control. Long ops receive a cancellation token. Cache is per-document, keyed by normalized
statement/source and canonical read-set state, never display strings.

**Framework selection:** conduct a genuine native UI/editor spike on Windows and Linux before choosing
between egui/eframe, GPUI, Iced, Slint or another proven Rust stack. Evaluate editor fidelity,
accessible text primitives, IME, docking, SVG, HiDPI, shipping complexity, and long-term maintenance.
The original requirements prefer egui, but acceptance quality—not preference for a framework—decides.
If the no-JavaScript/no-webview contract is mandatory, Tauri is **not** an equivalent fallback without
an explicit scope decision.

## 4. Mandatory end-to-end acceptance journeys

| ID | Journey | Required observable outcome |
|---|---|---|
| J-01 | First-run | Fresh install, launch app, open example, run; editor, typed result and status correct within two primary actions. |
| J-02 | Basic script | Enter a=2; b=a^10; b; verify result, variables, cursor and log output. |
| J-03 | Precision | Set precision 20, evaluate sqrt(2), set 1000, rerun; full exact stored digits, visible precision and cache invalidation. |
| J-04 | Session isolation | Two tabs with x=10 vs x=99 and different precision; switch tabs during run; no state leak. |
| J-05 | Concurrent evaluation | Start long run A, type and execute B, cancel A; UI stays responsive, B result correct. |
| J-06 | Real cancellation | Run a cancellation-aware expensive operation; cancel from UI; underlying work terminates, terminal event seen, no pending spinner. |
| J-07 | Incremental reuse | Run a=2;b=a+1;c=pi(100); edit only independent statement; see actual reused count and correct output. |
| J-08 | Dependency invalidation | Change a or redefine function f; all transitive dependents recompute; independent statements reuse. |
| J-09 | Side effects | Repeated `print` and `plot` are executed each requested run; output not duplicated within one run. |
| J-10 | Clear semantics | Define function f and variable x; Clear Variables removes x but not f; deleting one var preserves all other state. |
| J-11 | Autocomplete | Type `sq`; completion offers sqrt with signature; define function and variable and see them appear; accepting suggestion edits source predictably. |
| J-12 | Error navigation | Trigger Unicode multiline error; exact line/column highlighted; error code/category accessible, jump-to-source works. |
| J-13 | Structured mathematics | Inspect symbolic simplification, matrix inverse, solve record and 1000-digit number without losing exactness metadata. |
| J-14 | MathIR inspection | Compile symbolic expression, navigate IR and copy byte-preserved IR; unrelated source edits do not reevaluate inspector. |
| J-15 | Plot presentation | Generate plot with paths, text, axes, clipping; zoom, pan, fit, copy and export SVG/PNG, compare reference screenshot. |
| J-16 | File safety | Open file, edit, external modify, resolve conflict; save preserving line endings and data; no silent overwrite. |
| J-17 | Crash recovery | Edit unsaved tab, kill process, relaunch, recover data, reopen tabs and layout without automatically executing. |
| J-18 | Theme/scale | Switch light/dark, 100/150/200% scale and 900×600 layout; no cropped main actions or unreadable text. |
| J-19 | Accessibility | Keyboard-only open, edit, run, inspect, export; screen reader announces relevant status/progress/diagnostics. |
| J-20 | Stress | Open 500KB script, output 100k log messages, render 1MB SVG; measure latency and bound memory. |
| J-21 | Offline | Disable network entirely and run all primary workflows; no listener, external CDN, or unplanned traffic. |
| J-22 | Release evidence | Build from clean pinned SHA, run GUI suite, save screenshot/semantic/perf evidence with hashes and source-bound release gate. |

## 5. Roadmap and acceptance gates

| Phase | Goal | Exit gate |
|---|---|---|
| D0 — Reference and UI technology | Freeze web reference; prototype rich text editor, IME, SVG, accessibility and docking in native Rust | CON baseline + chosen native stack evidence; no massive rewrite before editor spike |
| D1 — Solid native workbench | Design tokens, themes, layout, rich editor, files, tabs and real window tests | VIS/LAY/EDT/DOC P0 tests green; basic editing usable and visually reviewed |
| D2 — Real execution parity | Background workers, real cancel, progress, quick eval, correct snapshot, cache/precision | RUN/INC P0 gates and J-03 through J-10 green |
| D3 — Mathematical exploration | Structured results, variable/function tables, symbolic inspector, complete SVG plots | WKS/RES/SYM/PLT P0 gates and J-13..15 green |
| D4 — Polished multi-platform desktop | Accessibility, native integrations, first-run guidance, performance, crash recovery | A11Y/PLF/PERF P0 gates; supported-platform manual+automated GUI tests green |
| D5 — Independent certification | Strict cross-path parity + screenshot comparisons + usability session + release provenance | All P0, all 22 J journeys, zero critical usability/accessibility bugs, no stale evidence |

### Release decision protocol

**Never conflate:** (1) engine semantic tests; (2) AppState unit tests; (3) GUI workflow tests;
(4) screenshot/theming tests; (5) performance and accessibility. Each is an independent gate.

A production-quality desktop parity release requires: **all P0 requirements** passing or human-waived;
every Web-parity item explicitly accepted; all 22 mandatory journeys executed and green; native GUI
launched on each advertised OS; no skipped proof counted as PASS; clean source-bound build and evidence
hashes; and all severity-1 usability defects closed. P1 is required to call the experience "polished
v1.0" unless explicitly deferred with a quality impact documented. P2 remains optional.

### Intentionally excluded without explicit scope change

Multi-user remote hosting, browser runtime, plugin marketplace, agentic code editing, collaborative
cloud workspaces, debugger/stepper, and an embedded HTTP server. These can become future features but
are **not** pretexts to defer web parity.

## 6. Sources reviewed

- [`Lovelace.Studio/README.md`](https://github.com/jjackbauer/LovelaceSharp/blob/rust/Lovelace.Studio/README.md)
- [`Lovelace.Studio/wwwroot/index.html`](https://github.com/jjackbauer/LovelaceSharp/blob/rust/Lovelace.Studio/wwwroot/index.html)
- [`Lovelace.Studio/wwwroot/app.js`](https://github.com/jjackbauer/LovelaceSharp/blob/rust/Lovelace.Studio/wwwroot/app.js)
- [`Lovelace.Studio/wwwroot/styles.css`](https://github.com/jjackbauer/LovelaceSharp/blob/rust/Lovelace.Studio/wwwroot/styles.css)
- [`Lovelace.Studio/IncrementalRunner.cs`](https://github.com/jjackbauer/LovelaceSharp/blob/rust/Lovelace.Studio/IncrementalRunner.cs)
- [`Lovelace.Studio/EngineHost.cs`](https://github.com/jjackbauer/LovelaceSharp/blob/rust/Lovelace.Studio/EngineHost.cs)
- [`Lovelace.Studio/Program.cs`](https://github.com/jjackbauer/LovelaceSharp/blob/rust/Lovelace.Studio/Program.cs)
- [`rust/lovelace-desktop/src/app.rs`](https://github.com/jjackbauer/LovelaceSharp/blob/rust/rust/lovelace-desktop/src/app.rs)
- [`rust/lovelace-desktop/src/win32/shell.rs`](https://github.com/jjackbauer/LovelaceSharp/blob/rust/rust/lovelace-desktop/src/win32/shell.rs)
- [`rust/lovelace-desktop/src/engine/real.rs`](https://github.com/jjackbauer/LovelaceSharp/blob/rust/rust/lovelace-desktop/src/engine/real.rs)
- [`rust/lovelace-desktop/src/selftest.rs`](https://github.com/jjackbauer/LovelaceSharp/blob/rust/rust/lovelace-desktop/src/selftest.rs)
- [`.github/requirements/Lovelace.Rust.Desktop.md`](https://github.com/jjackbauer/LovelaceSharp/blob/rust/.github/requirements/Lovelace.Rust.Desktop.md)
- [`.github/requirements/Lovelace.Studio.Sessions.md`](https://github.com/jjackbauer/LovelaceSharp/blob/rust/.github/requirements/Lovelace.Studio.Sessions.md)

**Catalog statistics:** 229 independently identified requirements across 17 groups: P0: 171, P1: 56,
P2: 2. Types: D: 90, Q: 60, W: 79.
