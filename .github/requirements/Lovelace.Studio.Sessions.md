# Requirements: Lovelace.Studio — Sessions, Precision UI, Incremental Compute, Async Progress, Autocomplete

> Scope: Evolve the **web IDE** (`Lovelace.Studio`) and its **engine host** from the current
> "one shared engine + request/response + plain textarea" shape into a multi-session,
> live-updating IDE. Six capabilities are in scope: (1) a visible, UI-settable **precision**;
> (2) a **session model** with per-session state; (3) **incremental script execution**
> (hash-based "compute only what changed"); (4) **asynchronous** front-end ↔ back-end
> integration; (5) a **progress dialog** driven by backend state; and (6) **autocomplete** in
> the editor.
>
> This is a **requirements document for review — no implementation yet.** It supersedes the
> v1 scope in [Lovelace.Studio.md](Lovelace.Studio.md) wherever the two conflict. All "Decisions"
> below were resolved interactively with the requester.

---

## 1. Context (current state, confirmed in code)

| Today | Where |
|---|---|
| One process-wide `SuiteEngine` singleton; a `SemaphoreSlim` serializes every evaluation. | `Lovelace.Studio/Program.cs`, `EngineHost.cs` |
| Precision is a **process-global static**, not per-session: `Real.MaxComputationDecimalPlaces` (static + an `AsyncLocal` override) and `Real.DisplayDecimalPlaces` (pure static). `setprecision(n)` and the REPL `set precision` mutate the globals. | `Lovelace.Real/Real.cs:40-98`, `Lovelace.Suite/Interpreter.cs:873-889` |
| The front-end is a plain `<textarea>`; no autocomplete. | `Lovelace.Studio/wwwroot/index.html`, `app.js` |
| Sync is request/response: `POST /api/evaluate` returns one full snapshot; the editor awaits it. | `app.js` `run()`, `EngineHost.EvaluateAsync` |
| The engine records **per-statement** elapsed time/result/output **after** a run (no progress *during* the run). | `Lovelace.Suite/Timing.cs`, `Interpreter.cs:175` |
| The engine raises `VariableChanged` / `FunctionDefined` events. | `Interpreter.cs:107,1216,1222` |

The six capabilities are additive to the engine and host; none require changing the language
grammar or the numeric types.

---

## 2. Goals and Non-Goals

### Goals

| # | Goal |
|---|---|
| G1 | **Precision** is visible at all times in a fixed UI corner and settable from the UI, per session. |
| G2 | **Sessions**: each tab gets a session the backend hosts and keeps; many sessions run concurrently with independent variables, functions, precision, and computation cache. |
| G3 | **Incremental execution**: re-running a script detects which statements are already computed and unchanged (content hash + dependency check) and computes only what is new or invalidated. |
| G4 | **Async integration**: Run returns immediately and the front-end polls a run-status endpoint; the UI is never blocked on one response. |
| G5 | **Progress dialog**: long operations show a progress dialog whose steps, sub-progress, and status come from the backend. |
| G6 | **Autocomplete**: the editor offers completion suggestions while typing, drawn from the live session. |
| G7 | Preserve existing IDE value: variables table, functions panel, inline SVG plots, logs bar, error highlighting — all now session-scoped. |

### Non-Goals / Deferred

- Multi-user auth/authorization, cross-machine hosting, or sandboxing (still a localhost tool; arbitrary scripts remain the feature).
- Persisting sessions/snapshots across server restarts (in-memory sessions with TTL).
- Distributed execution or shared caches across processes.
- Breakpoints, step-debugging, profiling.
- Collaborative editing / multi-client fan-out to a *shared* session.
- Interrupting a single numeric operation mid-flight (cancellation is between statements; see D8).

---

## 3. Decisions (resolved with the requester)

| # | Decision | Choice | Consequence |
|---|---|---|---|
| D1 | Session lifecycle | **Resume the same session** on reload/reopen | State survives a browser reload; bounded by in-memory server lifetime |
| D2 | Session scope | **One session per tab** (sessionStorage token) | The same user can run several independent sessions side by side |
| D3 | Persistence | **In-memory only** | Sessions lost on server restart; a stale token yields a fresh session |
| D4 | Incremental semantics | **Dependency-aware memoization** | Editing a mid-script line only recomputes statements that depend on it |
| D5 | Cache scope | **Per-session cache** | Each session remembers only what it computed |
| D6 | Precision knob | **One knob** = computation + display together (mirror `setprecision`) | Digits shown = digits computed |
| D7 | Editor | **CodeMirror 6 via CDN** | Real editor with autocomplete + error decoration; no npm build step |
| D8 | Async transport | **Client polling** | `POST /api/evaluate` returns `{ runId }`; front-end polls run status |
| D9 | Progress granularity | **Statement-level + sub-progress + Cancel** | Bar per statement, finer bar inside long ops, Cancel between statements |
| D10 | Autocomplete sources | **Full session context** | Keywords + built-ins + user functions + live variables |

---

## 4. Feature requirements

### F1 — Precision display + set from the UI

**Objective.** The active precision is always visible in a fixed, unobtrusive corner and can be changed from the UI.

- **F1.1** A persistent readout in the toolbar/status area shows the session's precision, e.g. `1000 digits`.
- **F1.2** A control (numeric input + Apply, and/or preset dropdown: 20 / 50 / 100 / 1000 / 2500) sets it.
- **F1.3** Setting precision is **session-scoped**: it must not affect other sessions.
- **F1.4** One value drives both the computation cap and the display precision, mirroring `setprecision(n)` (D6). Default 1000; `n > 0` required.
- **F1.5** Changing precision invalidates the session's incremental cache for statements whose results depend on precision (see F3).
- **F1.6** Precision is part of the session snapshot and survives reload (D1).

**Acceptance criteria.** Two tabs → two sessions → different precision values coexist without cross-talk; the readout updates immediately; `sqrt(2)` after setting 50 digits shows 50 fractional digits; the value persists across a reload.

### F2 — Session model (multi-session backend)

**Objective.** Every tab gets a session the backend hosts; the backend keeps many live sessions with independent state.

- **F2.1** A **session registry** keyed by an opaque session id (GUID).
- **F2.2** On first load the front-end obtains a session id, stores it in **sessionStorage** (per-tab, D2), and sends it on every request (`X-Session-Id` header or cookie).
- **F2.3** Each session owns its own `SuiteEngine` (variables, functions, `_`, last plot, revision) **and** its own precision **and** its own incremental cache.
- **F2.4** Sessions run concurrently — one session's long computation must not block another (no global `SemaphoreSlim`).
- **F2.5** Lifecycle: idle TTL + explicit reset (`DELETE /api/session`); bounded registry with LRU eviction; sessions are in-memory only (D3).
- **F2.6** All state endpoints are session-scoped.

**Acceptance criteria.** Two tabs have independent variable tables; deleting a variable in one does not touch the other; a 30-second `pi(100000)` in one session does not stall `1+1` in another.

**Architectural note (precision isolation).** Precision is a process-global static today: `Real.DisplayDecimalPlaces` is a plain `Interlocked` static with **no** scoped override, while `Real.MaxComputationDecimalPlaces` already has an `AsyncLocal` override (`internal WithLocalPrecision`). Making precision per-session **needs AsyncLocal scoping, but AsyncLocal alone is not sufficient** — four pieces are required:

1. **Add the missing display-precision scope** — give `DisplayDecimalPlaces` an `AsyncLocal` override mirroring `WithLocalPrecision`, so both settings can be scoped together around one evaluation.
2. **Store the value per session** — AsyncLocal only makes a static *act* local while a scope is active; the session's precision must live on the session/`SuiteEngine` and be applied at the start of each operation.
3. **Scope the whole host operation, not just evaluation** — display precision is also consumed when *rendering* results and snapshots (`ValueFormatter.Format` inside `CaptureState`), which runs outside `EvaluateAsync`; both compute **and** format/snapshot must run inside the scope.
4. **Point `setprecision(n)` at the session** — today it writes the process-global statics; with per-session precision it must mutate the session's precision via the engine, otherwise a script changing precision clobbers every session again.

AsyncLocal flows down the call tree (through `await`, `Task.Run`, and `Parallel.*` workers) and not sideways to sibling work — exactly the isolation required — and `RealAsyncLocalTests` already proves this pattern for the computation cap.

### F3 — Incremental script execution (hash-based, dependency-aware)

**Objective.** Re-running a script computes only what is new or invalidated (D4, D5).

- **F3.1** The backend splits a script into **top-level statements** (the engine parses a `Program` of statements and reports each statement's `Position`; normalize via `ScriptSource` so hashes match what the engine evaluates).
- **F3.2** Each statement is identified by a **content hash** of its normalized source text.
- **F3.3** Per session, a **computation cache** maps a statement key to its outcome: result value, `print` output, per-statement elapsed time, plot capture, and the **read-set** (variables/functions it read).
- **F3.4** Per statement, decide **reuse vs recompute**:
  - **Reuse** only if its content hash is unchanged **and** every value in its read-set is unchanged **and** it has no side effects.
  - **Recompute** otherwise. Side-effect statements — `print`, `plot`, or anything that mutates external state — always recompute (or at least re-emit their output).
- **F3.5** **Mid-script edits are handled**: changing statement *i* recomputes *i* and only the statements that (transitively) depend on what changed; independent later statements are reused.
- **F3.6** The cache is invalidated by: variable/function mutation outside the script (UI delete), a precision change (F1.5), an engine revision bump, or session reset.
- **F3.7** The UI shows per-line **cached** vs **computed** status so the optimization is observable.

**Dependency model (to pin down in design).** The read-set of a statement is obtained by instrumenting the tree-walker's variable/function resolution (the single choke point). Control flow (`if`/loops/`for`) and user-function calls that read globals participate in the read-set; `_` (last result) is treated as a read of the prior statement's output. A statement that calls a user function reading global `y` depends on `y`. The **safe fallback** is prefix-replay (recompute from the first changed statement); the **target** is dependency-graph memoization (D4).

**Acceptance criteria.** Run `a = 2^1000; b = sqrt(a); c = a + b` twice → second run reuses all three (reported as cached). Edit only the middle line → `a` reused, `b` and `c` recomputed. Edit the first line → everything recomputes. Insert a `print` mid-script → that statement and its dependents recompute, unrelated statements are reused.

### F4 — Asynchronous front-end ↔ back-end integration (polling)

**Objective.** Run returns immediately and the front-end polls a run-status endpoint; the UI is never blocked (D8).

- **F4.1** `POST /api/evaluate` starts a **background run** in the session and returns `{ runId }` immediately.
- **F4.2** The backend records the run's progress and final state in a **per-run status store**.
- **F4.3** The front-end polls `GET /api/run/{runId}` (adaptive interval) and re-renders from the returned snapshot as work commits.
- **F4.4** The run-status payload carries: status (`queued/running/finished/error/cancelled`), statement-level progress (current/total + per-line mode `reuse|compute`), sub-progress (when available), partial variables/functions/logs/plot, and the final snapshot + diagnostics on completion.
- **F4.5** Legacy `GET /api/state` remains for initial hydration and as a non-run fallback.
- **F4.6** Only one run is in flight per session; a new Run while one is running is rejected (or queued) with a clear status.

**Acceptance criteria.** During a multi-statement run the variables table and logs update before the run finishes; a refresh mid-run re-attaches via `/api/run/{runId}` or `/api/state`.

### F5 — Progress dialog (backend-driven)

**Objective.** Long operations show a progress dialog whose content is transmitted from the backend (D9).

- **F5.1** Statement-level progress: current step / total, a per-step label (the statement's trimmed text or built-in name), and a status (`pending/running/reused/done/error`).
- **F5.2** **Numeric-layer progress hooks (required).** Long single operations must expose a progress callback so the backend can report sub-progress: `Real.Sqrt` (Newton-Raphson iterations), Chudnovsky `Pi` (term/series segments), `Natural.Factorial` (parallel chunk progress), and any other long algorithm. The engine surfaces this as an `IProgress`-style callback carried into evaluation, which the host relays as `subProgress`/`subLabel`. Where a specific algorithm has no hook yet, the backend shows a determinate statement-level bar with an in-progress spinner — never a fabricated percentage.
- **F5.3** The front-end renders a **non-blocking progress dialog**: status line, a progress bar (statement-level + a sub-bar when available), elapsed/ETA, a "reused cached results" counter, and a **Cancel** button.
- **F5.4** The dialog auto-dismisses on completion/error but keeps a dismissible summary; errors surface line/column + message.
- **F5.5** **Cancel** stops further statements and leaves already-committed state consistent (cancellation is a positioned, well-formed outcome, not a corrupted session). Cancellation is **between statements** (mid-`sqrt` interruption deferred).

**Acceptance criteria.** A script with a deliberately slow statement shows an advancing progress dialog with live labels; Cancel stops it and the workspace stays consistent; a 90%-cached script shows "9/10 reused" and completes near-instantly.

### F6 — Editor autocomplete

**Objective.** The editor offers completion suggestions as the user types (D7, D10).

- **F6.1** Replace the `<textarea>` with **CodeMirror 6 via CDN** (no npm build step).
- **F6.2** Completion sources, in priority order: (a) **keywords**/operators; (b) **built-in functions** with signature + doc hint (`sqrt(x)`, `pi([digits])`, `matmul(a,b)`, …); (c) **user functions** (name + parameters); (d) **variables in scope** (name + type).
- **F6.3** The built-in/function/variable lists come from the **backend** (engine registry + session state — single source of truth), via `/api/completions` or the state payload, so suggestions never diverge from what the engine knows.
- **F6.4** Trigger: as-you-type (debounced) and/or Ctrl+Space; popup with keyboard navigation (↑/↓/Tab/Enter) and mouse.
- **F6.5** Snippet/template completion for common constructs (`func`, `for … in …`, `if/else`, `plot(x,y,"title")`) as a stretch item.
- **F6.6** Existing error highlighting (v1 F5) must keep working alongside the editor upgrade.

**Acceptance criteria.** Typing `sq` suggests `sqrt(x)` with its signature; after `x = 42`, typing `x` suggests the live variable `x (Natural)`; after `func square(x)=x^2`, `squ` suggests `square(x)`.

---

## 5. Target architecture

~~~mermaid
flowchart TB
    subgraph Browser
        UI[IDE UI: editor + precision readout + progress dialog + workspace + plot + logs]
        CM[CodeMirror 6 + autocomplete]
        POLL[Run-status poller]
    end

    subgraph Studio["Lovelace.Studio (ASP.NET Core minimal API)"]
        SR[SessionRegistry<br/>id → Session]
        S1[Session<br/>SuiteEngine + precision + cache]
        S2[Session]
        SN[Session …]
        RUN[RunStatus store<br/>runId → progress/result]
        INCR[Incremental Planner<br/>hash + dependency cache]
        COMP[Completion Provider]
    end

    subgraph Suite["Lovelace.Suite"]
        EN[SuiteEngine / Interpreter]
        EV[events: VariableChanged / FunctionDefined / statement progress]
        PC[per-eval precision scope]
    end

    UI -->|session-scoped REST| SR
    UI -->|poll /api/run/{id}| RUN
    SR --> S1 & S2 & SN
    S1 --> INCR --> EN
    EN --> EV --> RUN
    COMP --> S1
    EN --> PC --> Suite
~~~

- **Session registry** replaces the singleton + `SemaphoreSlim`; each session is an isolated engine + precision + cache. One run at a time per session; sessions run in parallel.
- **Run-status store** holds progress/partial state for the poller; **incremental planner** sits between host and engine (normalize → split statements → hash → diff against cache → drive the engine statement-by-statement → record results).
- **Precision scope** makes precision a per-evaluation `AsyncLocal` concern instead of a global.

---

## 6. Backend API (proposed)

| Method | Path | Purpose | Notes |
|---|---|---|---|
| POST | `/api/session` | Create a session; returns `{ sessionId, precision, revision }` | front-end reuses a stored id |
| GET | `/api/session` | Session metadata (id, precision, revision, TTL) | for hydration |
| DELETE | `/api/session` | Destroy the current session | clears engine + cache |
| GET | `/api/state` | Variables + functions + revision + precision | session-scoped |
| POST | `/api/evaluate` | Start a run; returns `{ runId, sessionId }` immediately | progress via polling (D8) |
| GET | `/api/run/{runId}` | Run status/progress/partial+final state | polled by the front-end |
| POST | `/api/run/{runId}/cancel` | Cancel the run | graceful, between statements |
| PUT | `/api/precision` | Set session precision | `{ digits }` |
| DELETE | `/api/state` | Clear variables (functions remain) | session-scoped |
| DELETE | `/api/variables/{name}` | Remove one variable | session-scoped |
| GET | `/api/completions?prefix=&line=&col=` | Completion candidates | engine registry + session state |

Session identity is carried in `X-Session-Id` (or a cookie) and validated against the registry; unknown/expired ids yield 404/410.

---

## 7. Run-status & DTO contract (proposed)

**Run-status payload** (returned by `GET /api/run/{runId}`, polled):

~~~json
{
  "runId": "…",
  "status": "running",            // queued | running | finished | error | cancelled
  "sessionId": "…",
  "totalStatements": 10,
  "completedStatements": 4,
  "reusedCount": 3,
  "current": { "index": 5, "label": "sqrt(a)", "mode": "compute", "subProgress": 0.42, "subLabel": "Newton iteration 3/8" },
  "steps": [ { "index":1, "mode":"reuse", "elapsed":"12µs" }, … ],
  "variables": […], "functions": […], "logs": […], "plot": {…},
  "result": {…},
  "diagnostics": […],
  "elapsed": "1.2s", "eta": "3.4s"
}
~~~

**New/extended DTOs** extend the existing `Dtos.cs`/`StudioJsonContext.cs` records with `sessionId`, `precision`, `runId`, reuse/mode flags, sub-progress, and the run-status shape above. All JSON must remain Native-AOT-safe via the source-generated context.

---

## 8. Non-functional requirements

- **Session isolation correctness** — precision, variables, functions, and cache must never leak across sessions (make-or-break; guarded by a cross-session test).
- **Determinism** — reused results equal recomputed results for the same inputs; a "reused" statement is semantically indistinguishable from a fresh evaluation.
- **Native AOT compatibility** — run-status DTOs, completions, and polling must keep working under the source-generated JSON context and trimmed reflection.
- **Local scope** — still binds to localhost; arbitrary script execution remains the intended feature.
- **Back-pressure & bounds** — one run per session; bounded registry + bounded caches (evictable); a completed run's status is retained only briefly (or until replaced).
- **Progress honesty** — never show a fabricated percentage; indeterminate where the engine is one long atomic operation.
- **Concise & maintainable** — the front-end renders backend state; no language logic duplicated in JS.

---

## 9. Open questions / risks

1. **Precision is process-global today** — retrofitting per-session precision requires a scoped override for `DisplayDecimalPlaces` (and exposing the computation-cap scope). Highest correctness risk; must be solved before F2/F3 land.
2. **Dependency tracking is the hard part of F3** — a naive read-set can under-invalidate (wrong results) or over-invalidate (no speedup). Mitigation: content-hash prefix reuse first, then add read-sets behind a correctness test that compares cached vs fresh results.
3. **Control flow / functions reading globals** complicate the read-set (a `func f(x)=x+y` reads `y` at call time). Mitigation: a statement calling a user function that reads a global depends on that global.
4. **Sub-progress requires numeric-layer cooperation** (D9, now a hard requirement) — `Sqrt`/Chudnovsky `Pi`/parallel `Factorial` iterate/segment, so hooks are feasible but are a deeper engine change touching `Lovelace.Real` and `Lovelace.Natural`; still sequenced after the statement-level dialog ships so the host/UI plumbing exists first.
5. **Polling latency vs responsiveness** — choose an adaptive interval (fast during active runs, slow when idle) and a small status retention window to bound memory.
6. **Native AOT + new endpoints** — keep all new serialization on the source-generated context; avoid reflection-only paths.

---

## 10. Completeness checklist (to be filled by implementation)

- [x] Precision readout + control in the toolbar; per-session precision isolation (F1).
- [x] Session registry, session-scoped endpoints, lifecycle/TTL, per-tab token (F2).
- [x] Per-evaluation precision scope in the engine (F2/F1 prerequisite).
- [x] Statement splitting + content hashing + per-session computation cache (F3).
- [x] Dependency/read-set tracking and invalidation rules (F3).
- [x] Reuse-vs-recompute reporting in the UI (F3).
- [x] Background run + run-status store + polling client (F4).
- [x] Progress dialog with statement-level progress, sub-progress, reused count, Cancel (F5).
- [x] CodeMirror editor + completion provider backed by engine state (F6).
- [x] Cross-session isolation and determinism tests (NFR).