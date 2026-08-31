# Lovelace.Studio

A browser IDE over the [Lovelace.Suite](../Lovelace.Suite/README.md) scripting engine: a script
editor, a variables table + functions panel, an inline SVG graph display, and a logs bar at the
bottom. It is a thin HTTP/JSON projection of the engine — the front-end renders engine DTOs only,
and all language logic lives in Lovelace.Suite.

---

## Architecture

~~~
Browser (HTML/CSS/JS, static)
        |  POST /api/evaluate, GET /api/state, DELETE /api/state, DELETE /api/variables/{name}
        v
Lovelace.Studio (ASP.NET Core minimal API — Program.cs, EngineHost.cs, Dtos.cs)
        |
        v
Lovelace.Suite (SuiteEngine — the scripting engine)
~~~

EngineHost is a **pure projection**: it maps CaptureState() → variables/functions, the per-call
output writer → logs, LastPlot → inline SVG, and the thrown exception + Diagnostics → error
position. It reuses the engine's tokenizer/parser/evaluator; the only transformation it performs
is rewriting top-level newlines to ';' so a newline-separated script parses as one program (the
engine's grammar is semicolon-separated, exactly like the REPL which submits one line at a time).

---

## API

| Method | Path | Purpose |
|---|---|---|
| POST | /api/evaluate | Evaluate { source }; returns result, variables, functions, logs, plot, diagnostics, revision. |
| GET | /api/state | Current { revision, variables[], functions[] } snapshot. |
| DELETE | /api/state | Clear all variables (functions remain). |
| DELETE | /api/variables/{name} | Remove one variable. |
| GET | / | Serve the IDE (static wwwroot/). |

---

## Run

~~~bash
dotnet run --project Lovelace.Studio
~~~

Then open the localhost URL printed by the server (default http://localhost:5000).

**Local, single-user tool.** The server binds to localhost only, holds one shared engine session
(like the REPL), and intentionally runs arbitrary scripts — do not expose it beyond your machine.

---

## Front-end

Vanilla HTML/CSS/JS under [wwwroot/](wwwroot/) — no npm, no build step, no framework. The editor
is a plain monospace textarea with line/column readout and error highlighting (persisted to
localStorage); the graph pane injects the returned SVG inline; the logs bar aggregates print
output, results, and errors, with a quick-eval one-liner input.

---

## See also

- Engine: [Lovelace.Suite/README.md](../Lovelace.Suite/README.md)
- Requirements: [.github/requirements/Lovelace.Studio.md](../.github/requirements/Lovelace.Studio.md)
