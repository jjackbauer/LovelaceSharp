# Oracle defects observed while capturing references

These are behaviours of the live C# Studio (`http://localhost:5000`) observed by
`tools/reference-capture.ps1`. They are recorded because the desktop must **not** reproduce them, and
because a parity test that expected them would encode a bug as the contract. Nothing here changes the
web Studio; it is evidence and a deviation record (CON-006).

## OD-1 — A malformed script never reaches a terminal run state

- **Observed**: a script with a syntax error (`w = (1 +`, and earlier `y = ` and `f(x) = x + 1` before
  the `func` syntax was known) is accepted by `POST /api/evaluate`, but `GET /api/run/{id}` keeps
  reporting `queued` for at least 10 s (40 polls at 250 ms). The run never becomes `finished`,
  `error` or `cancelled`, and the session stays locked: the next `POST /api/evaluate` on that session
  returns **409 Conflict** ("A run is already in progress for this session.").
- **Artifact**: `reference/oracle-syntax-hang.json` (source, accepted run id, all 40 poll statuses).
- **Contrast**: a runtime error is handled correctly — `y = 1` / `z = y + missing_name` returns
  `status=error` with one diagnostic (`reference/REF-error.json`).
- **Why it matters**: the browser UI would show a progress strip forever; the v4 contract's RUN-014
  requires a diagnostic and a return to idle. The desktop implements RUN-014 (terminal error +
  diagnostic at the reported position) and documents this as an intentional divergence from a broken
  oracle behaviour, not as a parity gap.
- **Not caused by the harness**: reproduced with three different malformed scripts and a fresh
  session each time.

## OD-2 — Diagnostics carry line and column but no code or category

- **Observed**: the `diagnostics[]` entries in a run response have `line` and `column` (1-based) and a
  message, but `code` is empty. Artifact: `reference/REF-error.json`.
- **Why it matters**: RUN-014 asks for a code and category. The desktop's `LangError` already carries a
  coded error (`code`, `category`) from the Rust engine, so the desktop can display more than the
  oracle. Parity requires the message, position and jump-to-source; code/category are a desktop
  improvement the contract explicitly asks for.

## OD-3 — `code`-less diagnostics are returned with HTTP 200 and a run status of `error`

Recorded so a later test does not assume a non-2xx status for an evaluation error: the run is
accepted (200), and the failure is a property of the terminal run state.

## Reference artifacts captured at the same time

`reference/REF-*.json` hold the full request/response for: basic evaluation, precision changes,
1000-digit `sqrt(2)`, incremental reuse (`reused=3`, modes `reuse,reuse,reuse,compute` on a second
run), `print` streaming, a runtime error, numeric and symbolic matrix inverse, symbolic simplify,
solve, a plot (15 000-byte SVG), clear-variables semantics (variables 0, user function `f` retained),
and MathIR compile. `reference/completions.json` holds the 132-item completion catalog.
