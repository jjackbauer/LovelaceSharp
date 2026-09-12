# Audit D — "can an agent do real work through the published binary using only the documented surface?"

**Auditor:** Falsifier-auditor D (fresh, cycle 6 / round 13). **Target:** the published Native AOT binary
`C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe` (built from the final tree; the tree was not
otherwise touched). **Contract under test:** `docs/symbolics/dsh-protocol.md` plus the machine surface the binary
advertises (`capabilities()`, `functions[]`). **Ground truth:** SymPy 1.14.0 / mpmath 1.3.0 at
`C:\Users\ricar\dev\.lovelace-tools\python`, and the repository's own tests/descriptors.

**Method.** Every run below is against the published exe with `--json`. Scripts with embedded quotes were fed
through `--stdin` from a PowerShell here-string (PS 5.1 strips embedded `"` when passing a single-quoted
argument to a native command — my first probe of the doc's own solve example failed on that shell artifact,
not on the product; the stdin form is what all quoted evidence below used). No file in the product, any test,
any fixture or any document was edited.

Reproducible command forms used throughout (both were exercised; the `--eval` form only for scripts without
embedded double quotes, which is every script it is used on below):

```powershell
$exe = 'C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe'
# form A (no embedded quotes)
& $exe --eval 'det(1)' --json --omit-functions --omit-variables
# form B (arbitrary script)
$s = @'
x = symbol("x"); limit(1/x, x, 0)
'@
$s | & $exe --stdin --json --omit-functions --omit-variables
```

---

## F1 — P1 — NEW — a wrong-**shaped** argument is reported as an internal invariant failure, not as the documented recoverable argument error

**Commands** (form A; each repeated twice, deciding fields byte-identical):

```powershell
& $exe --eval 'det(1)'        --json --omit-functions --omit-variables
& $exe --eval 'matmul(1, 1)'  --json --omit-functions --omit-variables
& $exe --eval 'symbol(1)'     --json --omit-functions --omit-variables
& $exe --eval 'assume(1)'     --json --omit-functions --omit-variables
& $exe --eval 'transpose(1, 1)' --json --omit-functions --omit-variables
```

**Observed** (`det(1)`, verbatim, deciding fields; `EXIT=1`):

```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,
 "code":"InternalError","category":"InternalInvariantFailure",
 "message":"Specified cast is not valid.","recoverable":false,
 "diagnostics":[{"message":"Specified cast is not valid.","position":0,"line":1,"column":1}], ...}
```

The other four reproduce the same four deciding fields exactly (`ok:false`, `InternalError`,
`InternalInvariantFailure`, `"Specified cast is not valid."`, `recoverable:false`). The message names neither the
builtin, nor the argument position, nor the expectation.

**Extent (two independent sweeps, identical case sets).** 369 calls = all 123 entries of `functions[]` × 3 argument
shapes (`1`, `[1,2,3,4]`, `x` with `x = symbol("x")`), arity always filled from the advertised
`parameterCount`. Result: **52 calls returned `InternalError`/`InternalInvariantFailure`, spanning 26 builtins** —
`append, assume, assume_integer, assume_negative, assume_nonnegative, assume_positive, assume_real, concat, cross, det, dot, flatten, inv_full, linsolve, linsolve_full, matmul, matrix_rank, ndims, numel, rank, reshape, shape, squeeze, symbol, trace, transpose`.
The second sweep returned the identical 52 case indices.

**What the correct behaviour is, and how I know.**

* `Lovelace.Run.Tests/SystemBuiltinArgumentShapeTests.cs:6-19`: *"An argument of the wrong SHAPE … is a call-site
  mistake the caller can fix and retry, exactly like a wrong argument COUNT. The frozen protocol
  (docs/symbolics/dsh-protocol.md, "Error envelope") says such a call crosses as a recoverable argument
  error, `code: InvalidArgument` / `category: TypeMismatch`, naming the builtin and what it expected, and **NEVER
  as an internal invariant failure**."*
* `docs/symbolics/dsh-protocol.md:26` (errors are structural: code/category/message/recoverable/diagnostics),
  `:84-85` and `:207-209` (`TypeMismatch` is a member of the one taxonomy).
* `Lovelace.Symbolics/SymbolicsPlugin.cs:169-172` states the implemented contract for a coercion failure:
  *"reported with the function, the argument position and the actual payload kind"*.

**Contrast showing the contract is implemented elsewhere** (same command form, declared symbols):

```powershell
$s = @'
x = symbol("x"); y = symbol("y"); solve_system(x + y == 2, x - y == 0)
'@
$s | & $exe --stdin --json --omit-functions --omit-variables
```

→ `ok:false, code:InvalidArgument, category:TypeMismatch, recoverable:true, message:"solve_system(): argument 1 must be a list of equations, e.g. [x + y == 2, x - y == 0]; got Symbolic."` —
the exact shape the 26 builtins above do not produce.

**NEW / recorded.** The *class* is long recorded (`docs/goal-cycle-4/round-09/audit-P2P6-wire.md:91` P2-22; EVD-195,
EVD-207; `docs/goal-cycle-5/audit2/B1-differential.md` F4) and was closed **per call site** (solve_system, jacobian,
plot, the `mean(1)/max(1,2)/sum(x,5)` arity route — EVD-217, `SystemBuiltinArgumentShapeTests`, and the r11 assertion
flip `Assert.DoesNotContain("InternalError/InternalInvariantFailure", observed)`). The nearest cycle-6 record is
`docs/goal-cycle-6/round-5/bounds-reprobe.md:150`: *"one `InternalError/InternalInvariantFailure` remains on
`reshape(1,1,1,1,1,1,1,1,1)`"* — that 246-call sweep varied argument **counts** (a wrong count is caught at the call
site), so it never exercised the wrong-**shape** path; 25 of my 26 builtins appear in no recorded instance.
I therefore report this as **NEW**: the class is not closed, and its measured extent is 52 calls / 26 builtins,
not one residual.

*Severity note:* every input here is type-invalid (a scalar where a matrix/list/name is required), so this is
an error-classification defect (the agent is told the kernel broke and that the failure is unrecoverable),
not a crash or a wrong value on valid input → **P1**, not P0.

---

## F2 — P1 — NEW — the short `limit` family answers a refusal as bare prose `Text` under `ok:true`, with no code, category or enum

**Commands** (form B; each repeated twice, identical):

```powershell
$s = @'
x = symbol("x"); limit(1/x, x, 0)
'@
$s | & $exe --stdin --json --omit-functions --omit-variables
```

**Observed** (verbatim, trimmed; `EXIT=0`):

```json
{"ok":true,"revision":82,"result":{"kind":"Text",
  "display":"does not exist (left: -inf, right: +inf)",
  "typed":"does not exist (left: -inf, right: +inf)",
  "structured":{"kind":"Text","value":"does not exist (left: -inf, right: +inf)"}},
 "output":[],"timings":[{"position":0,...,"resultKind":"Symbolic"},{"position":17,...,"resultKind":"Text"}]}
```

`limit(sin(x), x, inf)` likewise → `{"kind":"Text","value":"unevaluated: coefficient does not evaluate at the point"}`,
and `limit_left(1/x, x, 0)`/`limit_right(1/x, x, 0)` produce the same *shape* of answer for the one-sided refusals. The
`_full` form of the **same input** answers a record (`x = symbol("x"); limit_full(1/x, x, 0)`):

```json
"structured":{"kind":"Record","type":"LimitResult","fields":[
  {"name":"status","value":{"kind":"Enum","type":"LimitStatus","value":"DoesNotExist"}},
  {"name":"exists","value":{"kind":"Boolean","value":"false"}}, ...]}
```

**What the correct behaviour is, and how I know.**

* `docs/symbolics/dsh-protocol.md:3-5`: the envelope is *"designed so that an agent never has to parse a display
  string to recover mathematical meaning."*  Here the only signal that the limit was **not** computed is the
  prose prefix `"unevaluated: "` / `"does not exist "` inside a `Text` value, under `ok:true`.
* `dsh-protocol.md:99-101` condemns exactly this shape for the short form: *"the short form no longer answers a
  vector (or, worse, **a prose sentence**) for one shape of answer and a record for another."*
* `dsh-protocol.md:61-94`: a refusal carries a stable `code` and an `ErrorCategory` enum, and *"message text is for
  humans and is never the contract."*  The kernel already produces both (`limit.unevaluated` /
  `UnsupportedOperation`; see `capabilities()` entry `limit.unevaluated-in-record-diagnostics`) — the short form
  discards them.
* The source of the prose is `Lovelace.Symbolics/SymbolicsPlugin.cs:1575-1583` (`LimitToExpr`: `DoesNotExist` →
  `$"does not exist (left: …)"`, default → `"unevaluated: " + reason`).
* The JSON machine surface cannot even warn an agent: `functions[]` publishes only
  `name,parameters,minArity,parameterCount,variadic,builtin,plugin` (verified over all 123 entries) — no return kind. The
  `Text` return is declared only in the REPL help descriptor (`SymbolicsPlugin.cs:388-391`, `ReturnKind` =
  `"Symbolic | Text"`), which is not part of the envelope.

**NEW.** Cycle 6 records `limit.unevaluated` only as a *record-carried* diagnostic (`capabilities()` entry,
`docs/goal-cycle-6/final/final.txt:71`, amendment P-5). The short form's prose refusal is not recorded anywhere in
`docs/goal-cycle-6/`. It is also a different defect from the recorded P-B4 (which is about digits, with an honest flag).

---

## F3 — P2 — NEW — `--print-budget n` is not monotone: a larger budget returns strictly **less** text

**Command** (form B, `N` swept; two complete sweeps, byte-identical):

```powershell
$s = @'
x = symbol("x"); expand((x+1)^10)
'@
$s | & $exe --stdin --json --omit-functions --omit-variables --print-budget N
```

**Observed** (value: nodeCount 48, unbudgeted `pretty` length **92**, canonical 398):

| budget | 1–8 | 9 | 10 | 12 | 16 | 20 | 24 | 32 | 47 | 48 / 50 / 64 / 100 |
|---|---|---|---|---|---|---|---|---|---|---|
| `pretty` length | 47 | 56 | 61 | **73** | **31** | 37 | **47** | 63 | 92 (`truncated:true`) | 92, full, no truncation fields |

Budget 16 returns **31** characters where budget 12 returned **73**; budget 24 returns 47. Every answer is
still a truthful prefix terminated by `" …"` with `truncated:true`, `truncationReason:"node-budget"` and `budget:N`
(verified: the returned text minus `" …"` is a prefix of the unbudgeted rendering).

**What the correct behaviour is, and how I know.** `Lovelace.Run/StructuredProjection.cs:135-138`: *"The prefix is
proportional to the fraction of the expression the budget allows, **so a larger budget still returns at
least as much**."* The observed behaviour is strictly decreasing in the budget, so this repo-stated
contract is false. An agent that retries with a larger budget can lose information it already had. Cause:
`Printing.Print` abbreviates against `max(48, 6×budget)` characters (`Printing.cs:387-394`) and
`EnforceNodeBudget` passes an already-abbreviated rendering through untouched, then proportionally cuts an
un-abbreviated one (`StructuredProjection.cs:140-156`).

**Honest scope of this finding:** the protocol document's literal invariant-4 text (`:17-20`) promises a
`" …"`-terminated prefix, `truncated`, `truncationReason` and `budget` — all of which hold. The false claim is the
monotonicity contract in the implementing function. No wrong mathematical value, no dishonest flag → **P2**.
Cycle 5 recorded that raising the budget does not *lengthen* output at low budgets
(`docs/goal-cycle-5/audit2/B3-surface.md` PB06) and closed the "totality" defect; the *shrinking* measured here is
not recorded (repo-wide grep for `monoton` in `docs/` finds no such row).

---

## F4 — P2 — NEW — the protocol document's own solve example is stale: its `timings` carries one entry for a two-statement script

**Command** — the document's example script, verbatim (`dsh-protocol.md:98`):

```powershell
$s = @'
x = symbol("x"); solve(x^2 - 4 == 0, x)
'@
$s | & $exe --stdin --json --omit-functions
```

**Observed** (two runs identical):

```json
"timings":[{"position":0,"elapsed":{...},"resultKind":"Symbolic","hasOutput":false},
           {"position":17,"elapsed":{...},"resultKind":"Record","hasOutput":false}]
"revision":82
```

The document prints a **single** `timings` entry (`position 0`, `resultKind "Record"`, lines 146-148) for the
same two-statement script, and `revision: 83`.

**Why this is a defect in the document and not merely "abridged".** `dsh-protocol.md:22-23` states *"`timings`
carries one entry per top-level statement"*, and `:101` limits the example's abridgement to *"the nested symbolic
renderings"* — a missing statement entry is neither. The same example also omits the `type` field the document's
own value form mandates for arrays (`:33`): live `solutions` is `{"kind":"Array","type":"Vector",...}`, and live
`multiplicity` carries `"exact":true`. I do **not** count `revision`: the document never defines it, and it
varies with the script in the live binary (80/81/82 observed).

**Everything else in the document I could execute matched**, including: the error example
(`x = symbol("x"); solve(x^2 - 4 == 0, x, integer)` → `InvalidOperation`/`DomainError`, message byte-identical to
`:192`, `timings` = 2 entries with the failed statement `Void`); the arity example
(`compile_full(1)` → `InvalidArgument`/`TypeMismatch`, `"compile_full(): expected 2 arguments; got 1."`);
parse errors and an unreadable file (`FileReadError`/`ParseError`) with `elapsedTime` present and `timings` present and
empty; exit codes 0/1/2. See the verified list below.

---

## Verified and explicitly NOT findings (so the next round does not re-litigate them)

* **Capability honesty, both directions.** All 19 entries of `capabilities().unsupported_operations` were driven
  through their own advertised `trigger`, twice each: 19/19 reproduce their advertised `code` **and**
  `category`, in the advertised carrier (error envelope vs record `diagnostics`). All 19 triggers are
  reachable; `solve_domains_refused` = integer/rational each refuse. The advertised entries match EVD-260.
* **Diagnostic form.** `solve_full(x^4 - x^2 - 1 == 0, x).diagnostics[0]` is, verbatim, a `Record` of type
  `Diagnostic` with exactly six fields in the documented order — `code` (Text), `category` (`ErrorCategory.UnsupportedOperation`),
  `message`, `recoverable` (Boolean), `location` = `{"kind":"Null"}`, `details` = `Array[0]` — exactly the
  document's example. All seven rich records (`SolveResult, SystemSolveResult, TransformResult, LimitResult, IntegrationResult, OptimizationResult, CompilationResult`)
  end with `diagnostics` as an Array. `transform.unsatisfiable-conditions` is reachable
  (`x = symbol("x"); assume(x == 0); simplify_full(x/x)` → `TransformStatus.Unsatisfiable`) and carries
  `recoverable:false` / `DomainError`, exactly as the document says.
* **Enum discipline.** In a 369-call sweep, no field named `status|completeness|exactness|classification|category`
  crossed as non-`Enum`; `type(r.status)` → `SolveStatus`; `solve_full(...).status` is
  `{"kind":"Enum","type":"SolveStatus","value":"Solved"}`.
* **Solve record.** `solve` and `solve_full` publish the identical structured record for the same input
  (byte-identical `result.structured`), as the document claims. The status→`complete`/`completeness` mapping was
  reproduced for `Solved`, `NoSolutions` (including `x^2 + 1 == 0` over `real` → complete, empty),
  `Partial` (`unrepresented_count:2` + reason) and `Unevaluated`. `BudgetExceeded` was not reached (below).
* **Invariant 4 durations.** `elapsedTime` agrees with `elapsed` in value, unit and magnitude on success and error;
  `timings[].position` is the exact zero-based source offset (verified 0/17/34 on a three-statement script);
  `resultKind`/`hasOutput` behave (`print("hi")` → `output:["hi"]`, `hasOutput:true`). Without the flag the
  canonical form is unabbreviated (checked to 3133 chars, 60/60 x-powers present for `expand((x+1)^60)`, no
  truncation triple).
* **Error sweep.** Across the 369 calls, no failure lacked `code`/`category`/`message`/`recoverable`/`diagnostics`/`elapsedTime`/`timings`.
* **Chains complete.** `solve → subs → evalf → print` (`print(evalf(subs(x^2+1, x, r.solutions[1].value), 20))` → prints
  `2.99999999999999999999522356663907438144`, i.e. the **already-recorded P-A5** inexact-numeric route with an honest
  `exact:false`; the exact value is 3, mpmath confirms the truncation starts at the 21st decimal);
  `integrate → diff → simplify` (`integrate(x*sin(x), x)` → `sin(x) - x*cos(x)`, SymPy identical);
  numeric matrix solve and residual (`matmul(A, matmul(inv(A), b))` → `[[1],[2]]` exactly, matching SymPy's
  `[1/5, 3/5]`); array build→transform→read (`reshape(1..6, 2, 3)` → shape `[2,3]`, `transpose` → `[[1,4],[2,5],[3,6]]`,
  `b[1,0] = 4`, `c[0,1] = 5`, `sum = 21`, `shape = [2,3]`, `numel = 6`). The documented member-access form
  `solve_full(x^2 - 4 == 0, x).status` (`Language.md:1136-1142`) works.
* `dft([1,2,3,4])`/`dft([1,2,3,4], 2)` (the document's shorter-form example) and `symbol("x")` work;
  `--file` works on the shipped fixture `harness/examples/invx2.ls` (ok:true).

## Already recorded elsewhere — **not** re-reported as findings

* **A `Void` top-level statement omits `result` entirely** (`print(1)`, `setprecision(1)` → `ok:true`, no `result` key).
  Recorded: `docs/goal-cycle-5/audit1/A3-symbolic.md:727` (F17, P2), `A4-docs.md:371` (A4-F10, P3),
  `audit2/B3-surface.md:684` (F10, P2). I reproduced it and am **not** claiming it.
* **Usage errors (exit 2) emit no envelope at all.** Recorded: `docs/goal-cycle-5/audit1/A2-wire.md:44` (F23, P2).
  Reproduced (`--bogus`, no args, `--print-budget 0|-3|abc` → exit 2, empty stdout, usage text on stderr).
* **`truncationReason:"depth-limit"` is unreachable through the CLI.** Recorded as INCONCLUSIVE in cycle 5
  (`docs/goal-cycle-5/audit1/A4-docs.md` C2). New static cause: `Lovelace.Run/Runner.cs:394-395` constructs only
  `new PrintBudget(MaxNodes: nodes)`; `MaxDepth` is never set, so the `depth-limit` arm (`Printing.cs:373-377`) cannot fire
  from any documented input. Enum member over-claim, **P2 at most** — and it is a documented bound of an
  already-recorded item, so I do not count it as a finding.
* **`capabilities()` does not list every reachable refusal** (e.g. `matrix_rank([[1,2],[3,4]])`,
  `linsolve_full([[1,2],[3,4]], [[1],[2]])`, `inv_full(...)` → `InvalidOperation`/`DomainError` "requires a symbolic
  matrix"). The record's own documentation says it is *"exhaustive over the refusal classes the round-09 audit
  exhibited, and explicitly not exhaustive over the runtime's whole error taxonomy"* (`SymbolicsPlugin.cs:788-827`),
  and the live record carries `exactness: {type: CapabilitiesExactness, value: BestEffort}`. **A documented bound — not a
  finding**; stated here so it is not counted as one later.
* **Numeric matrices cannot be solved with `linsolve`** (advertised as `linsolve.non-symbolic-matrix`); the
  workaround (`inv`/`matmul`) is exact and was verified.

---

## Findings table

| id | severity | new? | one-line repro |
|---|---|---|---|
| F1 | P1 | NEW | `& $exe --eval 'det(1)' --json --omit-functions --omit-variables` → `InternalError/InternalInvariantFailure` `"Specified cast is not valid."` `recoverable:false`; 52/369 swept calls, 26 builtins |
| F2 | P1 | NEW | `x = symbol("x"); limit(1/x, x, 0)` → `ok:true`, `result.kind:"Text"`, `"does not exist (left: -inf, right: +inf)"` — no code/category; `limit_full` on the same input returns `LimitResult` |
| F3 | P2 | NEW | `expand((x+1)^10)` with `--print-budget 12` → 73 chars, `--print-budget 16` → 31 chars (larger budget, less text) |
| F4 | P2 | NEW | the doc's own example script yields `timings` with two entries (positions 0 and 17); the document prints one |

## Could not check

| Item | Reason |
|---|---|
| `status=BudgetExceeded` row of the solve mapping table | No documented CLI input produced a `SolveStatus.BudgetExceeded` record; `--cancel-after` yields a `Cancelled/BudgetExceeded` **error envelope**, not that record. Not claimed either way. |
| `transform.budget-exceeded` diagnostic | `simplify_full` exposes no rewrite-budget knob on the documented surface; I found no reachable trigger. |
| `truncationReason:"depth-limit"` | Unreachable by construction from the CLI (`Runner.cs:395` sets only `MaxNodes`); already recorded as inconclusive in cycle 5. |
| `plot()` success path | It writes an SVG file; I did not want to exercise it again (see disclosure below). stdout purity was checked by source instead: the only console writer in the Suite/Run assembly is the interpreter's injectable `Output` (`Interpreter.cs:124`), which the runner captures into `output`. |
| Absolute completeness of the canonical text for very large values | Verified as (a) no truncation triple and (b) term counts (40/40 and 60/60 x-powers); there is no independent oracle for the canonical text itself. |
| `--text` output mode | Not documented in the protocol document; out of scope. |
| Exact extent of the F1 family | 369 calls = 3 shapes per builtin, not the full type lattice; **52 / 26 builtins is a lower bound.** |
| The 19 capability triggers under Native AOT vs `dotnet` DLL | Only the published `.exe` was used — which is the surface under test. |

## Disclosure (self-inflicted, restored)

While verifying the documented `--file` route I ran the shipped fixture `harness/examples/invx2.ls`, whose last
statement is `plot(x, y, "1/x^2")`; without `--plot-dir` it wrote
`C:\Users\ricar\dev\LovelaceSharp\plot.svg` (15 116 B, git-ignored). I deleted it immediately
(`Test-Path` → `False`). `git status --porcelain` then showed only ` M docs/goal-cycle-6/state.md`, which is **not
mine** — I wrote no file before this report. The run was also informative: the envelope for a plotting script
carries an undocumented top-level `plot` object (`path`, `title`, inline `svg`) and a `Text` result holding the
absolute path — additive, not a false claim, so not filed as a finding.
