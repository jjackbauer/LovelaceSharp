# Round 31 — Section 143 Adversarial Audit (Falsifier)

Auditor: Falsifier subagent. Window: 2026-09-11 09:34 → 10:10 (local -03:00), 35-minute box.
Mode: **report only — nothing was fixed, and nothing was committed to git.**

---

## 0. Re-publish evidence (MANDATORY FIRST STEP — DONE)

I did **not** audit a stale binary. The runner was re-published from the current source before any probe:

```
dotnet publish Lovelace.Run/Lovelace.Run.csproj -c Release -p:PublishAot=true -p:InvariantGlobalization=true -o out/aot
  -> Lovelace.Run -> C:\Users\ricar\dev\LovelaceSharp\out\aot\   (exit code 0)
```

| Artifact | Value |
|---|---|
| `out/aot/Lovelace.Run.exe` size | **5,662,208 bytes** |
| `out/aot/Lovelace.Run.exe` mtime | **2026-09-11T09:34:31.5366114-03:00** |
| SHA-256 | `9C8AB1CEF131351087C72C648D76D14734BD6958F829EBEE02245E4B88A3E45A` |
| Newest source file (excl. obj/bin) | `Lovelace.Integer/Integer.cs` — 2026-09-11T09:07:43.7399393-03:00 |
| Runner source `Lovelace.Run/Runner.cs` | older than the exe (mtime < 09:34:31) |
| Delta | **exe is 1,607.8 s (26 m 48 s) NEWER than the newest source file** |

**Freshness statement: the published exe is NEWER than every source file in the tree, so it cannot be a stale
binary.** Every runner probe below executed `C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe`
(Native AOT). The only non-runner probes are the C# API probes (personas 4/5/6), which by definition compile
against source; that is stated where it applies. Probe scripts and raw outputs: `out/audit31/`.

---

## 1. Persona: NEW USER

20 expressions typed the way a beginner types them (`out/audit31/p1/*.ls`, run via `--file`).

| # | Attempt (exact script) | Observed (exact output) | Verdict |
|---|---|---|---|
| 1 | `2 + 3` | exit 0, `5 (Natural)` | clean |
| 2 | `2^10` | exit 0, `1024 (Natural)` | clean |
| 3 | `7 / 2` | exit 0, `3.5 (Real)` | clean |
| 4 | `10 % 3` | exit 0, `1 (Natural)` | clean |
| 5 | `pi` | exit 0, 100-digit Real | clean |
| 6 | `Pi` | exit 1, `Undefined variable 'Pi'.` | CONFIRMED friction (cosmetic; message is honest) |
| 7 | `2x` (implicit multiplication) | exit 1, `Expected ';' or end of input but found 'x' at position 1.` | **CONFIRMED friction** — parser jargon; no hint that implicit multiplication is unsupported |
| 8 | `x = 1 # comment` | exit 1, `Unexpected character '#' at position 6.` | **CONFIRMED friction** — the language has no comment syntax, and nothing says so |
| 9 | `solve(x^2 = 4, x)` | exit 1, `Expected 'RParen' but found '=' at position 10.` | **CONFIRMED friction** — no hint that equations use `==` |
| 10 | `sqrt(4` (missing paren) | exit 1, `Expected 'RParen' but found '' at position 6.` | CONFIRMED friction (empty token rendered as `''`) |
| 11 | `2 +* 3` | exit 1, `Unexpected token '*' at position 3: expected a number, string, identifier, '[', or '('.` | clean (actionable) |
| 12 | `sqrt(-1)` | exit 1, `ArithmeticError/DomainError: Square root is not defined for negative numbers.` | clean for a beginner (honest, typed) |
| 13 | `foo(3)` | exit 1, `Unknown function 'foo'.` | clean |
| 14 | `y + 1` (typo'd variable) | exit 1, `Undefined variable 'y'.` | clean |
| 15 | `x = 3; x == 3` | exit 0, `True (Boolean)` | clean |
| 16 | `x = 1;` (trailing `;`) | exit 0, `1 (Natural)` | clean |
| 17 | `5 km` | exit 1, `Expected ';' or end of input but found 'km' at position 2.` | clean-ish (honest, but jargon) |
| 18 | `"hello"` | exit 0, `hello (Text)` | clean |
| 19 | `sin(90)` | exit 0 (symbolic) — **exit 0, `sin(90) (Symbolic)`** | clean |
| 20 | `solve(x^2 = 4, x)` variant `solve(x^2 - 4, x)` | see persona 2 | — |

**Verdict: friction confirmed** (rows 6, 7, 8, 9, 10). None of the errors read like a *defect* (no stack
traces, no internal invariants, all `recoverable: true`), but four of them are tokenizer/parser vocabulary
that a beginner cannot act on.

---

## 2. Persona: MATHEMATICIAN (answers verified, not just "a call returned")

All probes use `x = symbol("x")` first (`out/audit31/p2/`, `out/audit31/p2b/`).

| # | Attempt | Observed | Verdict |
|---|---|---|---|
| 1 | `factor(-x^2 - 2*x - 1)` | exit 0, `-(x + 1)^2` in **56 ms** | **clean — regression area verified correct** |
| 2 | `factor(-x^3 - x^2 + x + 1)` | exit 0, `-(x + 1)^2*(x - 1)` in **52 ms** | **clean — regression area verified correct** |
| 3 | `expand(factor(-x^3 - x^2 + x + 1))` | exit 0, `-x^3 - x^2 + x + 1` (exact round-trip) | clean |
| 4 | `expand(factor(-x^2 - 2*x - 1))` | exit 0, `-x^2 - 2*x - 1` (exact round-trip) | clean |
| 5 | `factor(-x^4 - 2*x^3 - x^2)` (new, neg. lead + repeated) | exit 0, `-x^2*(x + 1)^2` | clean |
| 6 | `factor(-x^4 - 2*x^3 - 2*x^2 - 2*x - 1)` (new, neg. lead + repeated + irreducible) | exit 0, `-(x + 1)^2*(x^2 + 1)`; `expand` round-trips exactly | clean |
| 7 | `solve(x^5 - x + 1 == 0, x)` | exit 0, `kind=Text`, display `partially representable (4 root(s) missing: complex algebraic roots not supported (RootOf is real-only in v1).); representable: [rootof(x^5 - x + 1, 0)]` | Partial with an honest reason, but see friction F3 |
| 8 | `solve_full(x^5 - x + 1 == 0, x)` | exit 0, `SolveResult(status: Partial, complete: False, completeness: Partial, represented_count: 1, unrepresented_count: 4, unrepresented_reason: complex algebraic roots not supported (RootOf is real-only in v1)., diagnostics: [Diagnostic(code: solve.unrepresented-roots, ...)])` | clean (structured + honest) |
| 9 | `integrate(sin(x^2), x)` | exit 0, `integrate(sin(x^2), x)` (unevaluated, returned as a symbolic value) | clean |
| 10 | `integrate_full(sin(x^2), x)` | exit 0, `IntegrationResult(status: Unevaluated, verified: False, exactness: Approximate)` | clean |
| 11 | `factor(x^4 + 1)` (irreducible over Q) | exit 0, `x^4 + 1` (unchanged) | clean |
| 12 | `factor(x^2 + 1)` | exit 0, `x^2 + 1` (unchanged) | clean |
| 13 | `limit(sin(1/x), x, 0)` (does not exist) | exit 0, `kind=Text`, `unevaluated: coefficient does not evaluate at the point` | see F3/F4 |
| 14 | `limit_full(sin(1/x), x, 0)` | `LimitResult(status: Unevaluated, exists: False, ..., diagnostics: [Diagnostic(code: limit.unevaluated, ...)])` | clean (true, but by accident — see F4) |
| 15 | `limit_full(x*sin(1/x), x, 0)` (**limit exists = 0**) | `LimitResult(status: Unevaluated, exists: False, ...)` | **CONFIRMED friction F4** |
| 16 | `limit(x*sin(1/x), x, 0)` | exit 0, Text `unevaluated: coefficient does not evaluate at the point` | honest (Unevaluated), see F3 |
| 17 | `limit_left/right(abs(x)/x, x, 0)` | `[unevaluated: ..., unevaluated: ...]` | honest |
| 18 | `solve(x^2 + 1 == 0, x, real)` (**empty solution set**) | exit 0, Text `no real solutions` | correct answer, prose kind (F3) |
| 19 | `solve_full(x^2 + 1 == 0, x, real)` | `SolveResult(status: NoSolutions, complete: True, completeness: Complete, solutions: [], diagnostics: [Diagnostic(code: solve.no-solutions, category: NoSolution, ...)])` | clean |
| 20 | `solve_system([x + y == 1, x + y == 2], [x, y])` (empty) | exit 0, Text `no solutions` | correct, prose kind (F3) |
| 21 | `det([[1, 2], [2, 4]])` (singular) | exit 0, `0 (Natural)` | clean |
| 22 | `inv([[1, 2], [2, 4]])` | exit 1, `InvalidOperation/DomainError: Matrix is singular and cannot be inverted.` | clean |
| 23 | `linsolve([[1, 2], [2, 4]], [1, 2])` | exit 1, `linsolve() requires a symbolic matrix A.` | **CONFIRMED friction F11** — misleading message |
| 24 | `solve(x^2 - 2 == 0, x)` | exit 0, `[-1/2*sqrt(8), 1/2*sqrt(8)]` — mathematically right, **not normalized** (√8 not reduced to 2√2) | CONFIRMED friction F5 |
| 25 | `solve_full(x^2 + 1 == 0, x)` | exit 0, `status: Solved`, solutions `-1/2*sqrt(-4)`, `1/2*sqrt(-4)`, `exactness: AlgebraicExact` | CONFIRMED friction F5 |
| 26 | `simplify([-1/2*sqrt(-4), 1/2*sqrt(-4)])` | exit 1, `ArithmeticError/DomainError: Square root is not defined for negative numbers.` | **CONFIRMED friction F5** — the engine emits `sqrt(-4)` as an exact solution yet rejects it as input |
| 27 | `simplify(solve(x^2 - 2 == 0, x))` | exit 1, `simplify(): argument 1 must be a symbolic expression; got Vector.` | CONFIRMED friction F6 — no way to normalize a solution vector |
| 28 | `inf - inf` | exit 0, `0 (Symbolic)` | **CONFIRMED friction F2 — indeterminate form silently = 0** |
| 29 | `0 * inf` | exit 0, `0 (Symbolic)` | **CONFIRMED friction F2** |
| 30 | `inf / inf` | exit 0, `inf/inf` (unevaluated) | clean (honest) |
| 31 | `inf + inf` | exit 0, `2*inf` | clean-ish |

**Verdict: the specifically named factor() area is CLEAN and correct** (rows 1–6, including two new adversarial
cases; every result round-trips through `expand`). Friction confirmed elsewhere: indeterminate forms (F2),
prose-valued results (F3), `exists: false` on an existing limit (F4), radical normalization (F5/F6),
linsolve's message (F11).

---

## 3. Persona: DSH AGENT (envelope, purity, structure)

| # | Attempt | Observed | Verdict |
|---|---|---|---|
| 1 | `--eval '1 + 1' --json` version fields | `"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2` present on success **and** on error | clean |
| 2 | stdout purity: `print("hello"); print("world"); 42`, stdout/stderr captured to files | **650 bytes on stdout, 0 bytes on stderr**, exactly one JSON object + trailing `\n`, `print` captured in `output` | clean |
| 3 | print payload shape | `"output":["hello\r","world"]` | **CONFIRMED friction F8** — per-line `\r` is not stripped |
| 4 | array shape `[1, 2, 3]` | `structured: {kind:"Array", type:"Vector", shape:[3], elements:[{kind:"Natural", value:"1", exact:true}, ...]}` | clean |
| 5 | matrix shape `[[1,2,3],[4,5,6]]` | `structured.shape=[2,3]`, flat elements | clean |
| 6 | structured error `foo(1)` | `"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Unknown function 'foo'.","recoverable":true,"diagnostics":[{"message":...,"position":0,"line":1,"column":1}]` + exit 1 | clean |
| 7 | Enum kinds and Diagnostic records (`solve_full`) | `{"name":"status","value":{"kind":"Enum","type":"SolveStatus","value":"Partial"}}`, `{"kind":"Enum","type":"Completeness","value":"Partial"}`, `{"kind":"Domain","domain":"complex"}`, diagnostics as records with code/category/recoverable | clean — genuinely structural |
| 8 | `capabilities()` | `CapabilitiesResult(supported_domains: [real, complex], unsupported_domains: [integer, rational], unsupported_operations: [UnsupportedCapability(operation_class: complex.sqrt-negative, code: ArithmeticError, ...), ...], exactness: BestEffort)` | clean |
| 9 | `--print-budget 20` on `expand((x+1)^20)` | `"nodeCount":98,"truncated":true,"truncationReason":"node-budget","budget":20` | clean |
| 10 | `--text` mode | `= x^20 + ... (Symbolic)` + variable rows | clean |
| 11 | bare `solve` / `limit` results | `kind: "Text"` carrying prose (`no real solutions`, `unevaluated: coefficient does not evaluate at the point`, `partially representable (...)`) | **CONFIRMED friction F3 — an agent must parse display text** |

**Verdict: the envelope is clean and genuinely structural** (rows 1, 2, 4–10). The one thing an agent still has
to parse out of display text is the *status of the non-`_full` solver/limit builtins* (row 11).

---

## 4. Persona: C# API CONSUMER (first 5 minutes)

Probe project: `out/audit31/api/ApiProbe.csproj` (a scratch project referencing the real projects; this is
source-level, not the AOT exe — stated explicitly).

| # | Attempt | Observed | Verdict |
|---|---|---|---|
| 1 | `new SuiteEngine(); await EvaluateAsync("evalf(1/3, 50)")` | `THREW System.InvalidOperationException: Unknown function 'evalf'.` | **CONFIRMED friction F7** |
| 2 | bare engine; `symbol("x"); solve_full(x^2 - 4 == 0, x)` | `THREW System.InvalidOperationException: Unknown function 'symbol'.` | CONFIRMED friction F7 (worse: even symbol declaration fails) |
| 3 | bare engine; `factor(x^2 - 1)` | `THREW ... Unknown function 'symbol'.` | CONFIRMED friction F7 |
| 4 | documented wiring (`SymbolicsPlugin` + `MathIRPlugin` + `DspPlugin`) | `OK kind=Real value=0.3333333333333333333333333333` | clean |
| 5 | does the failure carry a plugin hint anywhere? | `engine.Diagnostics` = 1 entry: `"Unknown function 'nope'."` — same text, no hint | CONFIRMED friction F7 |
| 6 | documentation compensation | `SuiteEngine` XML doc (lines 12–21) explains plugin loading and names `Lovelace.Run/Program.cs` as the reference wiring. But `README.md` contains **zero** `LoadPlugin`/`SymbolicsPlugin` mentions, and outside cycle-3 the only docs hits are `docs/symbolics/a-plus-cycle-2-status.md` (an audit artifact) and `docs/architecture/modus-plugin-design.md` | **Documentation does NOT compensate** for a consumer who starts at the README; it compensates only for one who reads the type's XML doc (F7) |

**Verdict: friction confirmed and unchanged since cycle 2.**

---

## 5. Persona: STUDIO USER — **NOT VERIFIED (no browser, API surface only)**

I did **not** attempt a browser and I am **not** implying any UI verification. What I actually checked is the
public API surface by reading source:

| # | Attempt | Observed | Verdict |
|---|---|---|---|
| 1 | enumerate public types in `Lovelace.Studio` | `EngineHost`, `SessionRegistry`, `Session` + request/response DTOs (`EvaluateRequest`, `EvaluateResponse`, `SymbolicInspectRequest/Response`, `StateResponse`, `CompletionResponse`, `StartRunResponse`, `RunStatusResponse`, `PlotPayload`, `TimingRow`, `DiagnosticRow`, `FunctionRow(…, string? Plugin)`) | API surface exists and mirrors the runner's envelope shapes |
| 2 | run the Studio server, exercise an endpoint, render any UI | **not done** | NOT CHECKED |

---

## 6. Persona: CONCURRENCY TESTER

All rows from `out/audit31/api` (C#), except where noted.

| # | Attempt | Observed | Verdict |
|---|---|---|---|
| 1 | two engines, different variables + different `Output` writers | `A.x=111  B.x=222`; `writerA=[print-from-A]  writerB=[print-from-B]` | clean — no cross-engine leakage |
| 2 | two engines, different assumption sets | `C.assumptions=x > 0`, `D.assumptions=y < 0` | clean — no leakage |
| 3 | nested `EvaluateAsync` on the **same** engine from inside a registered builtin | inner call threw `ReentrancyNotSupportedException: this engine is already evaluating in the current flow: nested EvaluateAsync on one engine is not supported (it would deadlock on the evaluation gate)...`; outer evaluation returned `outer OK 0` in well under 10 s | clean — structured, **no deadlock** |
| 4 | two concurrent flows on the same engine (`Task.WhenAll`) | `flow1=1` / `flow2=2`, elapsed=0ms | clean |
| 5 | **OQ-002**: cancellation of a long single statement — CLI `--cancel-after 2000` on `2^1000000000` | **process still running at 30,027 ms → killed** (`EXIT=TIMEOUT`) | **CONFIRMED DEFECT F1** |
| 6 | **OQ-002**: same from C# — `EvaluateAsync("2^1000000000", null, cts.Token)`, token cancelled at 1500 ms | `NOT CANCELLED: still running 12 s after a 1500 ms token — token NOT observed` (engine JIT build, no runner involved) | **CONFIRMED DEFECT F1** |
| 7 | control: `(x + 1)^100000` with the same 1500 ms token | returned in 78 ms (`(x + 1)^100000`, `nodeCount: 5` — powers are lazy, so this is *not* a valid cancellation probe) | clean (control only) |
| 8 | orphan cleanup | after the killed runs, `Get-Process Lovelace.Run` → 0 remaining (the child had to be force-killed by the wrapper) | note |

**Verdict: OQ-002 is SETTLED — NO, a long single statement does not observe the cancellation token.** Engine
isolation and reentrancy are clean.

---

## 7. Persona: MALICIOUS / FUZZ INPUT

17 hostile inputs in `out/audit31/p7/` + 5 in `out/audit31/p7b/`, each run with a hard per-probe timeout.

| # | Attempt | Observed (exit / output) | Verdict |
|---|---|---|---|
| 1 | empty script (0 bytes) | exit 0, `ok:true`, result `void` | clean |
| 2 | 200-deep nesting `((((…1…))))` | exit 0, `1` in 71 ms | clean |
| 3 | 1000-deep nesting | exit 0, `1` in 555 ms | clean |
| 4 | unterminated string `x = "abc` | exit 1, `InvalidInput/ParseError: Unterminated string literal at position 4.`, recoverable | clean |
| 5 | self-assignment `x = x` | exit 1, `Undefined variable 'x'.` | clean |
| 6 | contradictory assumptions `assume(x > 1); assume(x < 0)` | exit 1, `UnsatisfiableAssumptions: Assumption x < 0 contradicts the existing assumptions (its negation x >= 0 is provable).` | clean |
| 7 | `assume(x > 1)` with `x` never declared | exit 1, `Undefined variable 'x'.` | clean |
| 8 | **huge exponent `2^1000000000`** | **TIMEOUT: still running at 20,054 ms, killed** | **CONFIRMED DEFECT F1 — HANG** |
| 9 | `2^100000` (bounded big integer) | exit 0, 30,103-digit Natural in 183 ms | clean |
| 10 | `1/0` | exit 1, `DivisionByZero/DomainError: Cannot divide by zero.` | clean |
| 11 | `1.5/0` | exit 1, `DivisionByZero/DomainError: Cannot divide a Real by zero.` | clean |
| 12 | 5,000-character identifier | exit 0, `1` in 129 ms | clean |
| 13 | unicode `π = 3; "héllo→😀"` | exit 1, `Unexpected character 'π' at position 0.` (typed, recoverable) | clean (unicode identifiers unsupported; message is honest) |
| 14 | unicode string literal `"héllo→😀"` | exit 0, `héllo→😀 (Text)` | clean |
| 15 | ragged matrix `[[1, 2], [3]]` | exit 1, `Ragged nested list literal: every row must have the same shape.` | clean |
| 16 | `;;;` (empty statements) | exit 1, `Unexpected token ';' at position 0: expected a number, string, identifier, '[', or '('.` | CONFIRMED friction F12 (empty script and trailing `;` are fine; leading `;` is not) |
| 17 | `inf - inf`, `0 * inf` | exit 0, both `0` | **CONFIRMED DEFECT F2** |
| 18 | CRLF + tab source | exit 0, `2` | clean |
| 19 | `inf / inf` | exit 0, `inf/inf` (unevaluated) | clean |

Nothing crashed the process, produced a stack trace, or surfaced an internal invariant failure. The one hang
is F1. Note the hang is *not* memory-related: it is unbounded CPU in exact integer power.

---

## 8. Consolidated friction list (severity ordered, each with a reproducer)

### F1 — HIGH (DEFECT): a long single statement ignores cancellation and hangs forever
- `out/aot/Lovelace.Run.exe --eval "2^1000000000" --json` → still running at 30 s (killed).
- `out/aot/Lovelace.Run.exe --eval "2^1000000000" --json --cancel-after 2000` → **still running at 30,027 ms** (killed).
- C#: `cts = new CancellationTokenSource(1500); await engine.EvaluateAsync("2^1000000000", null, cts.Token)` → still running 12 s later; `EvaluationCancelledException` never raised.
- Impact: no host or agent timeout policy can stop a single statement; the process must be killed. **This settles OQ-002 as a NO.**

### F2 — HIGH (DEFECT): indeterminate forms silently collapse to 0
- `--eval "inf - inf"` → exit 0, `0 (Symbolic)`. `--eval "0 * inf"` → exit 0, `0 (Symbolic)`.
- Contrast: `inf/inf` stays unevaluated, so the engine is inconsistent about which indeterminate forms it
  refuses.
- Impact: a mathematician gets a confident wrong answer, not an Unevaluated/NaN.

### F3 — HIGH (agent-facing): `solve`/`limit` return prose in a value of `kind: "Text"`
- `--eval 'x = symbol("x"); solve(x^2 + 1 == 0, x, real)'` → `"kind":"Text"`, display `no real solutions`.
- `--eval 'x = symbol("x"); limit(sin(1/x), x, 0)'` → `kind: "Text"`, `unevaluated: coefficient does not evaluate at the point`.
- `--eval 'x = symbol("x"); solve(x^5 - x + 1 == 0, x)'` → `kind: "Text"`, `partially representable (4 root(s) missing: …)`.
- Impact: status, completeness and diagnostics exist as structure **only** via `solve_full`/`limit_full`; the
  documented one-liners force display-text parsing. Inconsistent result *kinds* for one mathematical operation.

### F4 — MEDIUM (DEFECT): `limit_full` says `exists: False` for a limit that exists
- `--eval 'x = symbol("x"); limit_full(x*sin(1/x), x, 0)'` → `LimitResult(status: Unevaluated, exists: False, value: , …, exactness: Exact)`; the true limit is **0**.
- Source: `SymbolicsPlugin.cs:373` sets `exists = status is Value or ±Infinity`, so "could not evaluate" is
  reported as "does not exist" — while `status: Unevaluated` sits right beside it. `exactness: Exact` on an
  unevaluated result is also meaningless.
- Impact: any consumer that branches on `exists` draws a false mathematical conclusion.

### F5 — MEDIUM: solver output is not normalized, and the engine contradicts itself about `sqrt(-4)`
- `solve(x^2 - 2 == 0, x)` → `[-1/2*sqrt(8), 1/2*sqrt(8)]` (√8 not reduced to 2√2).
- `solve_full(x^2 + 1 == 0, x)` → solutions `-1/2*sqrt(-4)`, `1/2*sqrt(-4)` marked `exactness: AlgebraicExact`.
- `--eval 'simplify([-1/2*sqrt(-4), 1/2*sqrt(-4)])'` → exit 1, `ArithmeticError: Square root is not defined for negative numbers.`
- Impact: the engine both produces and rejects the same expression; a user cannot clean up a solution set.

### F6 — MEDIUM: no vector-level `simplify`
- `--eval 'x = symbol("x"); simplify(solve(x^2 - 2 == 0, x))'` → exit 1, `simplify(): argument 1 must be a symbolic expression; got Vector.`

### F7 — MEDIUM (C# consumer, unchanged since cycle 2): bare engine + no plugin hint
- `new SuiteEngine()` then `evalf(1/3, 50)` → `InvalidOperationException: Unknown function 'evalf'.`
- `symbol("x")` is *also* unknown, so the consumer cannot even reach the solve path.
- `engine.Diagnostics` repeats the same string — no hint naming a plugin.
- Documentation: the `SuiteEngine` XML doc covers it; **README.md does not** (0 hits for `LoadPlugin`), and no
  user-facing doc outside cycle-3 audit files shows the wiring. Compensation is partial at best.

### F8 — LOW: `output` entries keep a trailing carriage return
- stdout for `print("hello"); print("world"); 42` contains `"output":["hello\r","world"]` (split on `\n` only after trimming the whole string).

### F9 — LOW: beginner parse errors are tokenizer vocabulary
- `2x` → `Expected ';' or end of input but found 'x' at position 1.`
- `solve(x^2 = 4, x)` → `Expected 'RParen' but found '=' at position 10.`
- `sqrt(4` → `Expected 'RParen' but found '' at position 6.` (empty token rendered as `''`)

### F10 — LOW: no comment syntax, with no hint
- `x = 1 # comment` → `Unexpected character '#' at position 6.`

### F11 — LOW: `linsolve` message misdescribes the mismatch
- `linsolve([[1, 2], [2, 4]], [1, 2])` → `linsolve() requires a symbolic matrix A.` (the argument *was* a matrix; no documented way to build the "symbolic" one.)

### F12 — LOW: empty statements are inconsistent
- empty script → exit 0; `x = 1;` → exit 0; `;;;` → `Unexpected token ';' at position 0…` (exit 1).

### F13 — LOW: inconsistent infinity arithmetic (related to F2)
- `inf + inf` → `2*inf`; `inf/inf` → `inf/inf` (unevaluated); `inf - inf` → `0`.

**Confirmed friction count: 13 rows (F1–F13). Two are outright correctness defects a mathematician would hit
(F1 hang, F2 indeterminate→0); one is a structured-output correctness defect (F4).**

### What was clean (explicitly observed)
- **The named factor() regression area: correct in 6/6 cases**, including both required expressions
  (`factor(-x^2 - 2*x - 1)` → `-(x + 1)^2`, `factor(-x^3 - x^2 + x + 1)` → `-(x + 1)^2*(x - 1)`), two new
  adversarial cases with a negative leading coefficient + repeated root + irreducible factor, and exact
  `expand` round-trips. Runtime 52–77 ms — **no hang**.
- Degree-5 solve = Partial with an honest, structured reason; unevaluated integral = explicit
  `IntegrationResult(status: Unevaluated, verified: False)`; irreducible polynomials left alone; empty solution
  set = `NoSolutions, completeness: Complete`; singular matrix = `det 0` + typed `inv` error.
- Envelope: versions present, stdout is byte-pure JSON (0 bytes on stderr), array/matrix shape, Enum kinds,
  Diagnostic records, structured truncation (`truncated/truncationReason/budget`).
- Concurrency: no cross-engine leakage (variables, writers, assumptions), reentrancy reported as a structured
  exception instead of a deadlock, concurrent flows on one engine fine.
- Fuzz: no crash, no stack trace, no internal invariant failure, no orphaned process left behind.

---

## 9. NOT CHECKED (with reason)

| Not checked | Reason |
|---|---|
| **Studio UI** (browser, DOM, endpoints, front-end bundle) | No browser in this environment and the brief forbids attempting one. I read the `Lovelace.Studio` public surface only; I did not start the server or call an endpoint. |
| Studio front-end/web-asset ↔ DTO agreement | Follows from the row above. |
| `--plot-dir`/`plot()` SVG correctness | Out of scope for the seven personas; no probe run. |
| Test suites (`dotnet test`) as an independent signal | Time-boxed to 35 min; the audit is behaviour-based, not test-based. |
| Cancellation behaviour of long statements *other than* exact big-integer power | Only `2^1000000000` was probed; other long paths (e.g. large polynomial factoring) may or may not observe the token. F1 proves at least one does not. |
| Python | No Python probing was needed or attempted; no Python environment was inspected. |
| Performance, memory, GC, benchmark suites | Not part of the seven personas. |
| Native AOT internals / trim warnings | Not probed beyond the fact that the AOT publish succeeded and the exe runs. |
| Windows PowerShell 5.1 as the *embedding* host for the C# API | The C# probes ran on the .NET 10 JIT build; the AOT exe was used for every CLI probe. |
| Git state / commits | Explicitly forbidden; no commit was made. |

---

## 10. Worst finding (plain statement)

**The engine cannot be interrupted, and one four-character expression proves it.**
`out/aot/Lovelace.Run.exe --eval "2^1000000000" --json --cancel-after 2000` was still running 30,027 ms
later and had to be killed; the same statement under a C# `CancellationTokenSource(1500)` was still running
12 seconds later with no `EvaluationCancelledException`. That is OQ-002 answered **NO** — a long single
statement does not observe the cancellation token — and it means any agent or host that trusts its timeout
policy has no defence other than killing the process. Second worst: `inf - inf` and `0 * inf` both return
`0` with exit code 0, i.e. an indeterminate form is reported as a confident, wrong answer.
