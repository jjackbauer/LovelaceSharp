# Todo Plan: Lovelace.Studio — Sessions, Precision, Incremental Compute, Async Progress, Autocomplete

> Companion to [Lovelace.Studio.Sessions.md](Lovelace.Studio.Sessions.md). Ordered, dependency-aware
> implementation plan. **Do not start implementation until this plan (and the requirements doc) are
> approved.** Each task lists its deliverable, dependency, and acceptance criteria.

## Phase ordering & rationale

```text
Phase 0  Precision scoping (engine)      ← everything depends on session isolation
   │
Phase 1  Session model (backend)
   │
Phase 2  Precision UI ───────────────┐
   │                                  │
Phase 3  Incremental compute (backend)│
   │                                  │
Phase 4  Async run + polling ─────────┼── (3 feeds 4's plan; 4 feeds 5's dialog)
   │                                  │
Phase 5  Progress dialog (UI+backend) │
   │                                  │
Phase 6  Autocomplete (UI+backend)    │
   │                                  │
Phase 7  Hardening / cross-cutting    ┘
```

Phase 0 precedes Phase 1 because per-session precision requires the engine-side scoping to exist
first; without it, concurrent sessions would clobber each other's precision. Phase 4 (async run)
depends on Phase 3's statement-level planner because the progress dialog needs the per-statement
plan and reuse/compute modes.

---

## Phase 0 — Precision scoping in the engine

**Goal.** Make real-number precision an evaluation-scoped value instead of a process-global.

- [ ] **T0.1** Add a scoped override for `Real.DisplayDecimalPlaces` (an `AsyncLocal` scope analogous to `WithLocalPrecision`), and a combined helper that scopes *both* computation and display precision. *Dep: none. Deliverable: `Lovelace.Real` internal/public scope API.*
- [ ] **T0.2** Add per-engine precision to `SuiteEngine`: a `PrecisionDecimalPlaces` property that `EvaluateAsync` applies inside a scope and restores on exit. *Dep: T0.1. Deliverable: `SuiteEngine` evaluation is precision-isolated.*
- [ ] **T0.3** Tests: nested scopes restore correctly; concurrent evaluations with different precision do not leak sideways. *Dep: T0.2.*

**Exit criterion.** Two engines can evaluate concurrently at different precisions with correct results.

---

## Phase 1 — Session model (backend)

**Goal.** Replace the shared singleton with a per-session registry.

- [ ] **T1.1** Introduce `SessionState` (owns a `SuiteEngine`, precision, and a cache slot) and a `SessionRegistry` (id → session, TTL, LRU eviction, in-memory only). *Dep: Phase 0. Deliverable: registry + session type.*
- [ ] **T1.2** Session endpoints: `POST/GET/DELETE /api/session`; session identity via `X-Session-Id` (front-end stores a per-tab token in `sessionStorage`). Extend DTOs + `StudioJsonContext`. *Dep: T1.1.*
- [ ] **T1.3** Route all existing endpoints (`/api/state`, `/api/evaluate`, `DELETE /api/state`, `DELETE /api/variables/{name}`) through the session; replace the global `SemaphoreSlim` with a per-session gate. *Dep: T1.2.*
- [ ] **T1.4** Tests: cross-session variable/precision isolation; concurrent runs in different sessions don't block each other; expired sessions are evicted and rejected. *Dep: T1.3.*

**Exit criterion.** Two tabs hold independent state; a slow run in one doesn't stall the other.

---

## Phase 2 — Precision UI

**Goal.** Visible, settable, per-session precision (F1).

- [ ] **T2.1** `PUT /api/precision` and include `precision` in the session/state DTOs. *Dep: Phase 1.*
- [ ] **T2.2** Toolbar precision readout + setter (input + presets 20/50/100/1000/2500); apply updates the session and repaints. *Dep: T2.1.*
- [ ] **T2.3** Persist the session token in `sessionStorage` so precision (and state) survive a reload. *Dep: Phase 1.*

**Exit criterion.** Precision is visible in the corner, settable, session-scoped, and survives reload.

---

## Phase 3 — Incremental compute (backend)

**Goal.** Hash-based, dependency-aware "compute only what changed" (F3).

- [ ] **T3.1** Statement splitter: normalize via `ScriptSource`, parse to `Program`, and extract top-level statements with positions and stable normalized source text. *Dep: Phase 1. Deliverable: `StatementSlice` list.*
- [ ] **T3.2** Content hashing of each statement slice (SHA-256 over normalized text). *Dep: T3.1.*
- [ ] **T3.3** Per-session computation cache: statement key → { result, output, elapsed, plot, read-set, revision }. *Dep: T3.1.*
- [ ] **T3.4** Read-set instrumentation: the tree-walker records which variables/functions each top-level statement reads (single resolution choke point). *Dep: T3.1. Deliverable: read-set per statement.*
- [ ] **T3.5** Incremental planner: diff slices against the cache; decide reuse vs recompute per statement using content hash + read-set hashes; drive the engine statement-by-statement and record outcomes. Side-effect statements (`print`/`plot`) always recompute. *Dep: T3.2–T3.4.*
- [ ] **T3.6** Surface per-line `reuse|compute` mode in the timing/step rows. *Dep: T3.5.*
- [ ] **T3.7** Tests: repeat run reuses all; mid-edit recomputes only dependents; first-line edit recomputes all; `print` insertion recomputes dependents only; cached == fresh (determinism). *Dep: T3.5.*

**Exit criterion.** Re-running an unchanged script reports 100% reused; a mid-script edit recomputes only what depends on it.

---

## Phase 4 — Async run + polling

**Goal.** Run returns immediately; the front-end polls run status (F4).

- [ ] **T4.1** `RunStatus` store: per-session current run (`queued/running/finished/error/cancelled`) with the incremental plan, progress, partial state, and final snapshot. *Dep: Phase 3.*
- [ ] **T4.2** `POST /api/evaluate` starts a background run and returns `{ runId }`; add `GET /api/run/{runId}` and `POST /api/run/{runId}/cancel`. *Dep: T4.1.*
- [ ] **T4.3** Front-end poller (adaptive interval) that updates variables/logs/plot as partial state commits; Run is non-blocking. *Dep: T4.2.*
- [ ] **T4.4** Enforce one run per session; reject a second concurrent Run with a clear status. *Dep: T4.2.*

**Exit criterion.** The UI stays responsive during a long run and reflects partial results before completion.

---

## Phase 5 — Progress dialog

**Goal.** Backend-driven progress dialog with sub-progress and Cancel (F5).

- [ ] **T5.1** Progress dialog UI: statement-level bar, live step label, elapsed/ETA, "reused cached results" counter, and Cancel. *Dep: Phase 4.*
- [ ] **T5.2** **Numerical-layer progress hooks (required).** Add a progress callback to the long numeric algorithms and relay it to the front-end:
  - Define a progress contract in `Lovelace.Suite` (an `IProgress`-style callback or delegate carried into evaluation).
  - `Lovelace.Real.Sqrt` — report Newton-Raphson iteration progress (current target precision vs requested).
  - `Lovelace.Real.Pi` (Chudnovsky) — report series term/segment progress.
  - `Lovelace.Natural.Factorial` — report parallel chunk progress (chunks done / total).
  - Host maps callbacks to `subProgress`/`subLabel` in the run-status payload.
  *Dep: Phase 4. Deliverable: a tested progress hook on each long numeric operation.*
- [ ] **T5.3** Cancel semantics: stop between statements; committed state stays consistent. *Dep: T4.4.*
- [ ] **T5.4** Tests: dialog advances with truthful steps (statement + sub-progress); Cancel stops cleanly; a cached-heavy script reports the reuse count. *Dep: T5.1–T5.3.*

**Exit criterion.** Long runs show truthful progress (statement-level **and** sub-progress inside `sqrt`/`pi`/`!`), can be cancelled, and report cached-vs-computed counts.

---

## Phase 6 — Autocomplete

**Goal.** CodeMirror editor with session-context autocomplete (F6).

- [ ] **T6.1** `GET /api/completions` provider: keywords, built-ins (name + signature + doc), user functions, and live variables from the session. *Dep: Phase 1. Deliverable: completion DTO.*
- [ ] **T6.2** Replace the `<textarea>` with CodeMirror 6 (CDN) and port error highlighting (line/column + caret) onto it. *Dep: Phase 1.*
- [ ] **T6.3** Autocomplete UI: as-you-type + Ctrl+Space, popup with keyboard navigation, wired to `/api/completions`. *Dep: T6.1, T6.2.*
- [ ] **T6.4** Snippet completion for `func`/`for`/`if`/`plot` (stretch). *Dep: T6.3.*

**Exit criterion.** Typing `sq`→`sqrt(x)`; `x` suggests the live variable; `squ` suggests a user-defined `square`.

---

## Phase 7 — Hardening & cross-cutting

- [ ] **T7.1** Cross-session isolation + determinism test suite (precision, variables, cache). *Dep: all.*
- [ ] **T7.2** Native AOT build + smoke test (`make studio`), confirming all new endpoints/DTOs serialize under the source-generated context. *Dep: all.*
- [ ] **T7.3** Update docs: `Lovelace.Studio/README.md`, `index.html`/``app.js` comments, and the requirements checklist. *Dep: all.*

---

## Definition of Done

All of the following hold:

- Precision is visible, settable, per-session, and survives reload.
- Each tab has its own in-memory session; sessions are isolated and concurrent.
- Re-running a script computes only new/invalidated statements (dependency-aware, per-session cache), and the UI shows cached vs computed.
- Run returns immediately; the front-end polls progress; a progress dialog shows statement-level progress, sub-progress in long ops, and Cancel works.
- The editor autocompletes from keywords, built-ins, user functions, and live variables.
- Existing IDE value (variables/functions panels, inline SVG plots, logs, error highlighting) is preserved.
- Native AOT build succeeds; cross-session and determinism tests pass.
