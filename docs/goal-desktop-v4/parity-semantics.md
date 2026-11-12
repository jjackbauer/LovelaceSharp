# Behavioural parity is the contract, not the transport

CON-003 requires the desktop to preserve **behaviour and data semantics**, not the browser
implementation. This note records, mechanism by mechanism, what replaces what, and what a parity test
is therefore allowed to compare.

| Browser mechanism | Native replacement | Behaviour that must survive | What is deliberately not compared |
|---|---|---|---|
| `POST /api/evaluate` + `GET /api/run/{id}` polling at 150 ms | In-process `DesktopCommand::Run` + typed `DesktopEvent` stream from a worker | Accepted/refused decision, statement order, per-statement result and mode, reused count, diagnostics, terminal status, elapsed units | The transport, the poll cadence, the JSON field names |
| `X-Session-Id` + server session registry | `DocumentId` owned by one engine, one worker | Isolation of variables, functions, precision, revision, cache and last plot between documents | The header, the TTL, the registry |
| `sessionStorage` session token | Native file document + recovery journal | Unsaved text survives a crash; reopening does not auto-execute | The key name, the storage location |
| `localStorage` editor buffer and pane sizes | Real files plus versioned settings (`APPDATA`) | Buffer round-trips; layout restores; no silent overwrite | The storage location, the key names |
| CodeMirror 6 with `basicSetup`/`oneDark` | Native editor widget | Caret/line/column, error line, completion list, undo/redo, selection | Widget identity, DOM structure, the CodeMirror extension list |
| `innerHTML = plot.svg` | Native SVG scene rendered with resvg into a texture | The same engine SVG bytes; zoom/pan around the pointer; fit | The DOM node, CSS layout |
| `POST /api/symbolic/inspect` (never called by the web client) | In-process inspection of the last committed result | Pretty, canonical, tree, assumptions, trace, MathIR where supplied | The route, which has no client |
| Browser-supplied accessibility tree | AccessKit → Windows UI Automation | Roles, names, values, states, keyboard reachability | The browser's DOM accessibility mapping |
| ASCII/Unicode and precision toolbar controls | Native toolbar controls, per document | Server-side formatting and precision semantics, cache invalidation | The HTML inputs, their event handlers |

## Consequences for tests

1. A parity test compares **state snapshots** (variables with kinds, functions, precision, revision),
   **outputs** (result text, print lines in order), **diagnostics** (message, 1-based line/column,
   code where the engine has one), **plot bytes** and **terminal run status**.
2. A parity test may not assert on process shape, REST routes, JSON field names, or which widget
   renders a value.
3. Where the oracle is defective, the deviation is recorded in [`deviations.md`](deviations.md) and
   the desktop follows the requirement rather than the defect. Two such cases are known: a syntax
   error never terminates in the oracle (OD-1), and oracle diagnostics carry no code (OD-2).
4. Reference data lives in [`reference/`](reference/) and was captured through the same routes the
   browser uses, so a reviewer can re-derive it with `tools/reference-capture.ps1`.
