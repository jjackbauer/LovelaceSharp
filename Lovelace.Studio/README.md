# Lovelace.Studio

A browser IDE over the [Lovelace.Suite](../Lovelace.Suite/README.md) scripting engine: a CodeMirror
script editor with autocomplete, a variables table + functions panel, an inline SVG graph display,
a logs bar, a **session-per-tab** model, **per-session precision**, **incremental (hash-based)
script execution**, and an **async run model with a progress dialog**.

It is a thin HTTP/JSON projection of the engine — the front-end renders engine DTOs only, and all
language logic lives in Lovelace.Suite.

---

## Architecture

~~~
Browser (ES module: CodeMirror 6 via CDN + autocomplete + progress dialog)
        |  session-scoped REST (X-Session-Id header)
        |  POST /api/session, /api/evaluate, GET /api/run/{id}, PUT /api/precision, …
        v
Lovelace.Studio (ASP.NET Core minimal API)
        ├── SessionRegistry      (many concurrent sessions, idle TTL)
        ├── Session              (SuiteEngine + precision + computation cache + gate)
        ├── IncrementalRunner    (statement split → content hash → dependency-aware reuse)
        ├── RunState             (pollable progress/result store per run)
        └── EngineHost           (DTO projection + completions)
        |
        v
Lovelace.Suite (SuiteEngine — per-evaluation precision scoping + sub-progress hooks)
        |
        v
Lovelace.Natural / Integer / Real  (arbitrary-precision numerics; sqrt/pi/factorial progress)
~~~

---

## Sessions

Every browser tab gets its own session (an opaque id stored in `sessionStorage`, sent as the
`X-Session-Id` header). Sessions are in-memory (lost on server restart), idle-evicted, and fully
independent: variables, functions, **precision**, and the **computation cache** never leak across
sessions. A long computation in one tab does not block another.

---

## Precision

Precision is a single, visible, per-session knob (decimal places, applied to both computation and
display, mirroring `setprecision(n)`). It is shown in the toolbar and set from the UI or via
`PUT /api/precision`. The engine scopes precision per evaluation through an `AsyncLocal` scope
(`Real.WithPrecision`), so sessions stay isolated.

---

## Incremental execution

Re-running a script computes only what is new. The runner splits the script into top-level
statements, hashes each (SHA-256 of normalized source), and reuses a statement when its content
hash **and** its read-set (variables/functions, transitively through user functions) are unchanged.
Side-effecting statements (`print`, `plot`, `setprecision`) always recompute. Reused vs
computed status is reported per line and in the progress dialog.

---

## Async run + progress

`POST /api/evaluate` starts a background run and returns `{ runId }` immediately. The front-end
polls `GET /api/run/{runId}`, applying partial state (variables/logs/plot) as it commits and
painting a progress dialog: statement-level bar, sub-progress inside long operations (`sqrt`,
`pi`, `!`), a reused-results counter, and Cancel.

---

## API

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/session` | Create a session; returns `{ sessionId, precision, revision }`. |
| GET | `/api/session` | Resume/check a session. |
| DELETE | `/api/session` | Destroy a session. |
| GET | `/api/state` | Variables + functions + revision + precision. |
| POST | `/api/evaluate` | Start a run; returns `{ runId, sessionId }`. |
| GET | `/api/run/{runId}` | Poll run status/progress/result. |
| POST | `/api/run/{runId}/cancel` | Cancel an in-flight run. |
| PUT | `/api/precision` | Set session precision (`{ digits }`). |
| GET | `/api/completions` | Autocomplete catalog (keywords, built-ins, functions, variables). |
| DELETE | `/api/state` | Clear variables (functions remain). |
| DELETE | `/api/variables/{name}` | Remove one variable. |
| GET | `/` | Serve the IDE (static wwwroot/). |

---

## Front-end

An ES module under [wwwroot/](wwwroot/) — no npm build step. The editor is
**CodeMirror 6** (loaded via CDN) with autocomplete backed by `/api/completions` (keywords,
built-ins with signatures, user functions, and live variables). The UI also includes the precision
readout/setter, the session token handling, and the polling progress dialog.

---

## Run

~~~bash
make studio    # publish a Native AOT binary and run it (default)
~~~

Then open the localhost URL (default http://localhost:5000).

For JIT development iteration: `dotnet run --project Lovelace.Studio`.

**Local, single-user tool.** The server binds to localhost only and intentionally runs arbitrary
scripts — do not expose it beyond your machine.

---

## See also

- Engine: [Lovelace.Suite/README.md](../Lovelace.Suite/README.md)
- Requirements: [.github/requirements/Lovelace.Studio.md](../.github/requirements/Lovelace.Studio.md)
- Sessions plan: [.github/requirements/Lovelace.Studio.Sessions.md](../.github/requirements/Lovelace.Studio.Sessions.md)
