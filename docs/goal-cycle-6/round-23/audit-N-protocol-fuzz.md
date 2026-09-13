# Audit N — protocol / state-machine fuzzing against the documented surface (cycle 6, round 23)

**Oracles.** `docs/symbolics/dsh-protocol.md` (305 lines: Invariants 1–6, the value forms, the Diagnostic
form, positions, exit codes, the CI section), `Lovelace.Suite/docs/Language.md` (1160 lines, the
machine-checked language reference) and the binary's own `--help` (the flag list and the advertised
surfaces). The builtin surface was read off the envelope's own `functions[]` registry (123 entries)
rather than guessed.

**Binary under test (the only artefact used by every row below).**
`C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe` — verified in this session:
**5,764,608 bytes, LastWriteTime 2026-09-12 21:05:23** (the AOT publish of HEAD `dfae21b`, product code
`c8a6966`). The JIT twin was never used.

**Strategy (new to this cycle).** The documented surface was treated as the *specification of a state
machine* and fuzzed: a generator emitted **valid** programs over the documented constructs, crossed
them with the documented flag space and the three input surfaces, and after **every** run a mechanical
checker asserted the protocol's own invariants (exit/`ok`, single-JSON-value stdout, `output[]` = the
print lines, one timing per executed statement with the failed one `Void`, position usability
(`text.Substring(position)` names the statement), cross-surface identity, run-to-run byte-identity, the
cancellation ledger, the value-form/enum/Diagnostic rules, the doc's `status`→`complete`/`completeness`
table, and "no raw .NET text at the language surface").

**Numbers.** Final pass: **778 audited runs** over 156 atoms and 45 programs × {`--eval`, `--stdin`,
`--file`} × {none, `--omit-functions`, `--omit-variables`, `--print-budget 1/3/10/100`, `--plot-dir`,
`--cancel-after`, `--text`}; plus 111 Language.md examples through the binary, and the targeted
state-transition probes in §3. Every script was **validated before it was trusted** (EVD-330):
**152 / 156 atoms answer `ok:true`** as written; the 4 that do not are *my* probe errors and were
dropped from the corpus (§6.10), so no failure below measures the probe.

**Honesty / scope.** All scratch material (the generator, the checker, the `.ls` files, the raw
evidence) lives **outside the repo** in `%TEMP%\dsh-audit-N\`; the only repo file written is this one.
The harness drove the binary from Node 24 `spawnSync` with a **literal argv array** (no shell), so the
PS 5.1 embedded-quote trap (EVD-299) was never in play; scripts containing `"` also went through
`--stdin`/`--file` bytes. Every finding below was reproduced in two separate runs.

---

## 1. The generator's shape

* **Atoms** — 156 single-*statement* programs, each one documented construct: arithmetic (`1 + 2`,
  `3 - 5`, `2 ^ 3 ^ 2`, `5!`, `10 % 3`, `-7 % 3`, `1 / 3`, `2 + 0.5`, `0.(3)`), relations (`2 > 1`,
  `1 == 1`, `3 <= 3`, `2 != 2`), variables (`x = 42`, `y = x + 1`), vectors/ranges (`[1, 2, 3]`,
  `1..5`, `1..2..7`, `-2..2`, `5..-1..1`, broadcast `[1, 2] + [[1, 2], [3, 4]]`), indexing/slicing
  (`[10, 20, 30][0]`, `[0, 1, 2, 3, 4][1:4]`, `[[1, 2, 3], [4, 5, 6]][:, 1]`), strings and
  interpolation, blocks, `if/else`, `for`, `while`, `func` in both body forms, the documented array
  builtins (`zeros`, `reshape`, `shape`, `sum`, `prod`, `min/max/mean/norm`, `matmul`, `dot`, `cross`,
  `det`, `inv`, `trace`, `transpose`, `flatten`, `append`, `eye`, `ones`, `concat`, `squeeze`, `numel`,
  `ndims`, `rank`), the DSP set (`fft`, `dft`, `conv`, `filter`, `movingavg`, `impulse`, `step`,
  `cosine`, `exponential`, `powerseries`, `noise`, `delay`, `scale`, `re/im/conj/abs`) and the symbolic
  set (`symbol`, `diff`, `factor`, `expand`, `simplify`, `solve`, `solve_full`, `solve_system_full`,
  `series`, `limit[_full]`, `integrate[_full]`, `cancel[_full]`, `linsolve`, `compile_full`,
  `optimize_full`, `subs`, `evalf`, `type`, `assume*`, `capabilities`, `plot`), plus 11 runtime-refusal
  atoms and 2 parse-error atoms.
* **Programs** — atoms joined with `; ` (so every statement's expected UTF-16 offset is known exactly):
  success chains of 2–8 statements, **failure-in-the-middle** programs (`1+2; det(1); 2+0.5`),
  print-bearing chains, a block that prints *then* fails (`{ print("before"); det(1) }`), nested
  blocks, nested loops with prints, parse errors before and after valid statements, a trailing `;`, a
  trailing newline, newline-joined statements.
* **Crossings** — each program through `--eval`, `--stdin` (bytes) and `--file` (bytes), then under
  `--omit-functions`, `--omit-variables`, `--print-budget {1,3,10,100}`, `--plot-dir`, `--text`, and
  twice for byte-identity.
* **Checker** — after each run: exit ∈ {0,1,2} and agreeing with `ok`; stdout is exactly one JSON
  value; the three version fields; `timings` count/positions/`resultKind`/`hasOutput` against the known
  statement list; `output[]` against the known print lines; `result.kind` = last timing kind; every
  `timings[].position` and error `diagnostics[].position` sliced out of the caller's text must start
  the statement it indexes; error envelopes carry exactly `message/position/line/column` diagnostics
  with a `category` from the seven-member taxonomy; every value form's kind is in the documented set;
  arrays' `shape` product = `elements` length; `Integer`/`Natural` payloads parse as integers; enums
  carry a declared `type`; rich records end with `diagnostics`; Diagnostic records have exactly the six
  documented fields in order with an `ErrorCategory` enum and an Array `details`; the
  `status`→`complete`/`completeness` table; no string anywhere matches a .NET-exception pattern; and
  two identical runs are byte-identical apart from `elapsed`/`elapsedTime`/`timings`.


---

## 2. Invariant table

`BIN` = the AOT binary above. "778-run corpus" = the generator pass. Verdicts are the *measured* result
of the checker, not a reading of the source.

| id | invariant (source) | verdict | command that decided it | observed |
|---|---|---|---|---|
| I-1 | stdout carries the envelope and nothing else (protocol:9) | **PASS** | 778-run corpus, all flags/surfaces | `JSON.parse(stdout)` succeeds on every run (no trailing value); stdout is one line; stderr empty on every JSON path |
| I-2 | exit code ∈ {0,1,2}, consistent with `ok` (protocol:278) | **PASS** | 778 runs + 12 usage probes (`--cancel-after 0`/`-1`/`abc`/`2147483648`, `--print-budget 0`, `--nope`, no args, `-`, `--eval` with no value) | `ok:true`⇒0, `ok:false`⇒1 (0 exceptions); usage errors ⇒2 with **0 bytes** on stdout and the message on stderr |
| I-3 | `protocolVersion`/`symbolicFormatVersion`/`mathIrVersion` always present (protocol:16) | **PASS** | every envelope, incl. parse error, unreadable file, unusable plot dir, cancelled | `1` / `"#!lovelace-sym 1"` / `2` on all paths |
| I-4 | `output[]` = what the script printed (protocol:9–10) | **FAIL — N-1** | `BIN --eval 'print("a"); print()' --omit-functions --omit-variables` | `output:["a"]` while **both** timings say `hasOutput:true`; the trailing line is lost (§3) |
| I-5 | `timings` = one entry per executed statement; `Void` for the failed statement; present-and-empty for a parse error (protocol:31, 266–274) | **PASS** | `1+1; det(1); 2+2`; `1+2; (`; 45 generated programs × 3 surfaces | failure: 2 entries at the failing statement's offset, last `resultKind:"Void"`, no entry for statements after the failure; parse error: `"timings":[]` |
| I-6 | a position indexes the caller's text: `text.Substring(position)` names the statement (protocol:240–243) | **PASS** | 778 runs incl. CRLF/CR/LF, trailing newline, no trailing newline, multi-line blocks, **non-BMP** (`print("😀"); det(1)` → 13 = UTF-16 offset, not code-point 12) | 0 violations; eval/file/stdin identical |
| I-7 | `hasOutput` says whether the statement wrote with `print` (protocol:172, 181) | **FAIL — N-1** | `BIN --eval 'print("a"); print()' ...` | the second statement reports `hasOutput:true` but contributes no `output[]` entry; control `print("a"); print(""); print("b")` → `["a","","b"]` (interior empty line kept) |
| I-8 | `result.kind` is the last executed statement's kind | **PASS** | 778 runs | 0 disagreements; a void last statement has no `result` key and last `resultKind:"Void"` |
| I-9 | two identical runs byte-identical except `elapsed`/`elapsedTime`/`timings` (protocol:29–31) | **PASS** | 45 programs × 2 runs + `series(abs(x),x,0,3)`, `noise(1,0,4)`, `dft`, `evalf(pi(),30)`, `solve_full(x^4-x^2-1==0,x)` | 0 differences (no residual H-1 class) |
| I-10 | byte-identical text reports identical positions on every surface (protocol:240) | **PASS** | 6 texts (quotes, runtime error, solve, LF, CRLF, multi-print) × `--eval`/`--file`/`--stdin` | positions tuples identical; the whole envelope minus durations identical across surfaces |
| I-11 | print content round-trips | **PASS** | `print("a\nb")` (real LF inside the literal) | `output:["a","b"]` — the printed newline becomes the line break |
| I-12 | `--print-budget` bounds only the structured rendering; `output[]`/`display` untouched (protocol:18–23) | **PASS** | `x = symbol("x"); print(expand((x+1)^10))` and `y = expand((x+1)^10)` with `--print-budget 1/3/10/100` | `output[]` and `result.display` byte-identical to the unbudgeted run; `structured.pretty` = `"x^10 + 10*x^9 + 45*x^8 + 210*x^6 +  …"`, `truncated:true`, `truncationReason:"node-budget"`, `budget:3` |
| I-13 | `--omit-*` replace their arrays with EMPTY arrays, keys present, no value/display/output change (protocol:36–40) | **PASS (JSON path)** | `BIN --eval 'print("hi"); x = symbol("x"); solve(x^2 - 4 == 0, x)'` with/without both flags | `"functions":[]`/`"variables":[]`, keys present, envelope identical modulo durations |
| I-14 | … and change no **display** | **FAIL — N-2** | `BIN --eval 'x = 42; x*2' --text [--omit-variables]` | the human summary loses `  _ = 84` / `  x = 42` (§3) |
| I-15 | cancellation ledger self-consistent (`stopped`/`exceeded`/`excessMs`/`budgetMs`) and an overrun carries its diagnostic | **PASS** | `sum(1..20000000)` `--cancel-after 30` (3 runs); `sum(1..50000)` `--cancel-after 1`; `1+1` `--cancel-after 1` | prompt stop: `stopped:true, exceeded:false, excessMs:0`, no diagnostic (the amended semantics); overrun: `exceeded:true, excessMs:23.648`, diagnostic at the last timing's position; completion-after-deadline: `ok:true`, `stopped:false`, `exceeded:true` + diagnostic at `timings[0].position` |
| I-16 | an absent field is `{"kind":"Null"}`, never `""` (protocol:15) | **PASS** | corpus slot census: every JSON path key × the value forms seen at it | no key is ever both `Null` and empty `Text`; the only empty-`Text` slot is `value` (`unrepresented_reason`, `""` — the doc's own example) |
| I-17 | enum-valued fields carry the declared enum type, never `Text` (protocol:63–72) | **PASS, with N-4** | `solve_full(...).status`, `type(r.status)`, `cancel_full(...).status`, `limit_full`, `integrate_full`, `simplify_full` | `SolveStatus`, `Completeness`, `SolutionExactness`, `LimitStatus`, `IntegrationStatus`, `TransformStatus`, `ErrorCategory` all as declared, and `type()` answers the same name; **`CancelStatus` is published but is not in the doc's parenthesised list** (§3 N-4) |
| I-18 | the `status`→`complete`/`completeness` table (protocol:199–211) | **PASS** | `solve_full(x^2-4==0,x)`, `solve_full(x^2+1==0,x,real())`, `solve_full(x^4-x^2-1==0,x)`, `solve_full(x-x==0,x)`, `solve_full(exp(x)==0,x)` | `Solved`→`true/Complete`; `NoSolutions`→`true/Complete`; `Partial`→`false/Partial` (+`unrepresented_count 2`); `Unevaluated`→`false/Unknown`. `BudgetExceeded` row **UNTESTABLE** |
| I-19 | every rich result record ends with an Array `diagnostics` of `Diagnostic` records, never text/Null (protocol:81–98) | **PASS** | the 7 documented types (`solve_full`, `solve_system_full`, `simplify_full` → TransformResult, `limit_full`, `integrate_full`, `optimize_full`, `compile_full`) | last field `diagnostics` in all 7; empty ⇒ `{"kind":"Array","type":"Vector","shape":[0],"elements":[]}`; a populated one: `[Diagnostic(code:"solve.unrepresented-roots", category:UnsupportedOperation, recoverable:true, location:{"kind":"Null"}, details:[])]` — 4/4 Diagnostic records have exactly `code,category,message,recoverable,location,details` in that order |
| I-20 | the error envelope carries parser-form diagnostics and the two forms are never mixed (protocol:294–299) | **PASS** | every failing run in the corpus | error diagnostics have exactly `message,position,line,column`; no `code`/`category` key; record Diagnostics never carry `position` |
| I-21 | a wrong-arity/typed call crosses as `InvalidArgument`/`TypeMismatch`, never an internal failure (protocol:280–288) | **PASS** | `det(1)`, `compile_full(1)`, `inv_full(x)`, `linsolve_full(...)` | `code:"InvalidArgument", category:"TypeMismatch", recoverable:true` naming the builtin |
| I-22 | no message at the language surface is raw .NET exception text | **PASS (language surface)** | 778-run corpus, every string in every envelope | **0** hits for `System.*`, stack frames, `Parameter '…'`, `Object reference`, `.cs:line` |
| I-23 | `--plot-dir` is created when missing; an unusable one is a typed caller error, never a crash or empty stdout (round-15 fix) | **PASS** | `--plot-dir <deep/new>`, `--plot-dir ''`, `'   '`, `'C:\<bad>'`, `nul`, `<a file>\sub` | deep path created; each unusable path ⇒ `PlotDirectoryError`/`TypeMismatch`, `elapsed:"0 ns"`, `timings:[]`, exit 1, no crash |
| I-24 | `plot()` returns the written path; the `plot` block names the same file and carries the same bytes (Language.md §12, protocol:42–45) | **PASS on success / N-5 on failure** | `p = plot(1..5); p` with `--plot-dir D --plot-file a.svg` | `result.display` = `plot.path` = the file on disk; the SVG in the envelope == the file's bytes (14,657 = 14,657); one plot per evaluation |
| I-25 | Language.md's machine-checked examples hold through the binary | **PASS 111/111** | all 111 `lovelace`/`result` pairs extracted from `Language.md` and run through `--stdin` | values (`42 (Natural)`, `SolveResult(...)`), `error:` messages, `prints:` text and `plot:` titles all match; 0 failures |
| I-26 | `solve`/`solve_full` publish one record; the short form is not a vector or prose (protocol:123–126) | **PASS** | `solve(x^4-5*x^2+4==0,x)` vs `solve_full(...)` | identical `SolveResult` record; parametric answers arrive in `families[]` (`sin(x)==0` → `SolutionFamily(template: k*pi, parameter_domain: integer)`) and are *not* a complete-looking empty `solutions[]` |
| I-27 | the documented revision numbers are real (protocol:118–121) | **PASS** | fresh processes: `1+1`; `1+1;1+1;1+1`; `x = symbol("x"); solve(x^2 - 4 == 0, x)` | 81 / 81 / **82** — as the doc states |


---

## 3. Findings

### N-1 — **P1** — the last line of captured print output is dropped, while the same statement's `hasOutput` says it wrote one

**Exact command** (literal argv; the harness passed this as an argv array, no shell):

```
BIN --eval "print(\"a\"); print()" --omit-functions --omit-variables
```

PS 5.1-safe reproduction (PS 5.1 strips embedded `"` from native argv — EVD-299 — so the script goes
through a file; the `--file` surface is byte-identical):

```powershell
$BIN = 'C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe'
Set-Content -Path .\n1.ls -NoNewline -Value 'print("a"); print()'
& $BIN --file .\n1.ls --omit-functions --omit-variables
```

**Run 1 (exit 0), verbatim:**

```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":80,"output":["a"],"variables":[],"functions":[],"elapsed":"101 \u00B5s","elapsedTime":{"value":101,"unit":"\u00B5s"},"timings":[{"position":0,"elapsed":{"value":38.1,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":true},{"position":12,"elapsed":{"value":11.7,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":true}]}
```

**Run 2 (exit 0), verbatim:**

```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":80,"output":["a"],"variables":[],"functions":[],"elapsed":"92.2 \u00B5s","elapsedTime":{"value":92.2,"unit":"\u00B5s"},"timings":[{"position":0,"elapsed":{"value":32.1,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":true},{"position":12,"elapsed":{"value":11.5,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":true}]}
```

**What is wrong.** `print()` writes an empty line (Language.md §11: print writes each argument's display
form, "followed by a newline"); this script printed `a\n` and then `\n`. The envelope reports
`output:["a"]` — one line — while *both* timing entries say `hasOutput:true`, i.e. the envelope's own
machine-readable flag says the second statement wrote output that `output[]` does not carry. A consumer
that rebuilds the print stream (or that cross-checks `hasOutput` against `output[]`) loses a line and
sees a contradiction. It is not an empty-print artifact: an **interior** empty line is kept.

**Controls (same command shape, two runs each, all reproduced):**

| control | observed |
|---|---|
| `print("a"); print(""); print("b")` | `output:["a","","b"]` — interior empty line **kept** |
| `print(); print("a")` | `output:["","a"]` — leading empty line kept |
| `print()` (single statement) | `output:[""]` |
| `print("a\n")` (string ending in a real LF) | `output:["a"]` — the trailing newline the script printed is dropped |
| `print("a"); print(); det(1)` (failing path, exit 1) | `output:["a"]`, 3 timings, the second `hasOutput:true` |

**Why P1 and not P2.** Invariant 1 ("Anything the script prints with `print(...)` is captured and
returned in the top-level `output` array") and `hasOutput` ("says whether it wrote anything with
print") are both machine-readable claims; the envelope contradicts itself and the data is gone. The
loss is *bounded* (trailing empty lines / a trailing newline), which is why it is not P0.

**Failure-path evidence (exit 1), run 1 of 2 — verbatim (run 2 identical but durations):**

```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"det(): argument 1 must be a square matrix; got Natural.","recoverable":true,"diagnostics":[{"message":"det(): argument 1 must be a square matrix; got Natural.","position":21,"line":1,"column":22}],"elapsed":"522.1 \u00B5s","elapsedTime":{"value":522.1,"unit":"\u00B5s"},"timings":[{"position":0,"elapsed":{"value":33.1,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":true},{"position":12,"elapsed":{"value":6.7,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":true},{"position":21,"elapsed":{"value":293.4,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":false}],"output":["a"]}
```

---

### N-2 — **P2** — `--omit-variables` changes the `--text` display, which the payload-control paragraph says no omit flag may do

**Exact commands** (two runs each; stdout verbatim):

```
BIN --eval "x = 42; x*2" --text
BIN --eval "x = 42; x*2" --text --omit-variables
```

Runs 1 and 2 of the first command, both `exit=0`, stdout byte-identical:

```text
= 84 (Natural)
  _ = 84
  x = 42
```

Runs 1 and 2 of the second command, both `exit=0`:

```text
= 84 (Natural)
```

**What is wrong.** `docs/symbolics/dsh-protocol.md:39-40` says of both omit flags: "They change no value,
no **display** and no `output[]` entry." On the JSON path that holds (I-13 PASS). On the `--text`
surface the flag removes two display lines from stdout, so a consumer using `--text` plus
`--omit-variables` sees a different summary. The protocol document never describes `--text` (recorded by
round-22 audit J, I1-c), which is why this is charged P2 (documentation/scope) and not P1: the JSON
envelope — the machine API — is unchanged. `--print-budget` on the same surface changes nothing
(PASS, I-12), and `--omit-functions` alone changes nothing.

---

### N-3 — **P2** — the documented `Function` value kind cannot be produced by any documented construct

Language.md §1 lists exactly seven value kinds and gives the `Function` row as "First-class function
reference — `Function: square (Function)`". In the reference host a function name is not a value.

**Exact command** (two runs, verbatim):

```
BIN --eval "func square(x) = x ^ 2; square" --omit-functions --omit-variables
```

Run 1 (exit 1):

```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Undefined variable \u0027square\u0027.","recoverable":true,"diagnostics":[{"message":"Undefined variable \u0027square\u0027.","position":24,"line":1,"column":25}],"elapsed":"377.7 \u00B5s","elapsedTime":{"value":377.7,"unit":"\u00B5s"},"timings":[{"position":0,"elapsed":{"value":1.8,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":false},{"position":24,"elapsed":{"value":44.2,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":false}],"output":[]}
```

Run 2 (exit 1): identical envelope except `"elapsed":"365.9 µs"` and the two timing durations
(`1.5 µs` / `42.7 µs`); every other byte identical.

**Forms tried, each twice, each refused with the same shape** (all exit 1,
`code:"InvalidOperation"`, `category:"DomainError"`): `func square(x) = x ^ 2; print(square)`,
`… type(square)`, `… h = square`, `… [square]`; the block-body `func add(a, b) { a + b }; add` is the
same. `square(5)` answers `25 (Natural)`, so the *call* path works — only the value does not.
Engine-side there is a `ValueKind.Function` with the rendering `Function: {name}` and a structured
projection `{"kind":"Function","value":name}`, but no documented construct reaches it (a bare
identifier is looked up in the variable table only).

**Why P2.** The claim lives in prose (the value-kind table), not in the machine API; the behaviour is a
refusal, not a wrong value. But a reader who writes the documented example gets
`Undefined variable 'square'.`

---

### N-4 — **P2** — the protocol's closed lists of enum-type names and rich result records omit what `cancel_full` publishes

Protocol §"Enum-valued fields" says the declared enum type is one of `SolveStatus`, `Completeness`,
`SolutionExactness`, `LimitStatus`, `IntegrationStatus`, `TransformStatus`, `RuleClassification`,
`ErrorCategory`; §"The Diagnostic form" enumerates the rich result records as `SolveResult`,
`SystemSolveResult`, `TransformResult`, `LimitResult`, `IntegrationResult`, `OptimizationResult`,
`CompilationResult`. The binary publishes an eighth enum type and an eighth rich-looking record.

**Exact command** (two runs; run 1 verbatim, run 2 shown below it):

```
BIN --eval "x = symbol(\"x\"); cancel_full((x^2 - 1)/(x - 1))" --omit-functions --omit-variables
```

```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":82,"result":{"kind":"Record","display":"CancelResult(status: Conditional, original: (x^2 - 1)/(x - 1), expression: x \u002B 1, changed: True, conditions: [x - 1 != 0])","typed":"CancelResult(status: Conditional, original: (x^2 - 1)/(x - 1), expression: x \u002B 1, changed: True, conditions: [x - 1 != 0]) (CancelResult)","structured":{"kind":"Record","type":"CancelResult","fields":[{"name":"status","value":{"kind":"Enum","type":"CancelStatus","value":"Conditional"}},{"name":"original","value":{"kind":"Symbolic","pretty":"(x^2 - 1)/(x - 1)","canonical":"(mul (pow (add (rat -1 1) (sym x)) (rat -1 1)) (add (rat -1 1) (pow (sym x) (rat 2 1))))","domain":"complex","exact":true,"nodeCount":11,"freeSymbols":["x"]}},{"name":"expression","value":{"kind":"Symbolic","pretty":"x \u002B 1","canonical":"(add (rat 1 1) (sym x))","domain":"complex","exact":true,"nodeCount":3,"freeSymbols":["x"]}},{"name":"changed","value":{"kind":"Boolean","value":"true"}},{"name":"conditions","value":{"kind":"Array","type":"Vector","shape":[1],"elements":[{"kind":"Symbolic","pretty":"x - 1 != 0","canonical":"(ne (add (rat -1 1) (sym x)) (rat 0 1))","domain":"complex","exact":true,"nodeCount":5,"freeSymbols":["x"]}]}}]}},"output":[],"variables":[],"functions":[],"elapsed":"806.4 \u00B5s","elapsedTime":{"value":806.4,"unit":"\u00B5s"},"timings":[{"position":0,"elapsed":{"value":205.7,"unit":"\u00B5s"},"resultKind":"Symbolic","hasOutput":false},{"position":17,"elapsed":{"value":550.1,"unit":"\u00B5s"},"resultKind":"Record","hasOutput":false}]}
```

Run 2 (verbatim; only the three duration sites differ, shown to satisfy the two-run rule):

```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":82,"result":{"kind":"Record","display":"CancelResult(status: Conditional, original: (x^2 - 1)/(x - 1), expression: x \u002B 1, changed: True, conditions: [x - 1 != 0])","typed":"CancelResult(status: Conditional, original: (x^2 - 1)/(x - 1), expression: x \u002B 1, changed: True, conditions: [x - 1 != 0]) (CancelResult)","structured":{"kind":"Record","type":"CancelResult","fields":[{"name":"status","value":{"kind":"Enum","type":"CancelStatus","value":"Conditional"}},{"name":"original","value":{"kind":"Symbolic","pretty":"(x^2 - 1)/(x - 1)","canonical":"(mul (pow (add (rat -1 1) (sym x)) (rat -1 1)) (add (rat -1 1) (pow (sym x) (rat 2 1))))","domain":"complex","exact":true,"nodeCount":11,"freeSymbols":["x"]}},{"name":"expression","value":{"kind":"Symbolic","pretty":"x \u002B 1","canonical":"(add (rat 1 1) (sym x))","domain":"complex","exact":true,"nodeCount":3,"freeSymbols":["x"]}},{"name":"changed","value":{"kind":"Boolean","value":"true"}},{"name":"conditions","value":{"kind":"Array","type":"Vector","shape":[1],"elements":[{"kind":"Symbolic","pretty":"x - 1 != 0","canonical":"(ne (add (rat -1 1) (sym x)) (rat 0 1))","domain":"complex","exact":true,"nodeCount":5,"freeSymbols":["x"]}]}}]}},"output":[],"variables":[],"functions":[],"elapsed":"780.1 \u00B5s","elapsedTime":{"value":780.1,"unit":"\u00B5s"},"timings":[{"position":0,"elapsed":{"value":201.1,"unit":"\u00B5s"},"resultKind":"Symbolic","hasOutput":false},{"position":17,"elapsed":{"value":531.2,"unit":"\u00B5s"},"resultKind":"Record","hasOutput":false}]}
```

`type(r.status)` for the same record answers `CancelStatus` (measured twice). `CancelResult`'s fields are
`status,original,expression,changed,conditions` — it has **no** `diagnostics` field, so a consumer that
walks "the rich records all end with diagnostics" meets a record type the document does not mention.
All the enum types the document *does* list behaved as documented in the corpus (I-17);
`RuleClassification` was never produced by any documented call (§6.3).

---

### N-5 — **P2** — a `plot()` written before a later failure is on disk but the error envelope is silent about it

**Exact command** (fresh directory per run, two runs; stdout verbatim plus the directory listing):

```
BIN --eval "plot(1..5, 1/(1..5), \"T\"); det(1)" --plot-dir <fresh dir> --omit-functions --omit-variables
```

Run 1 (exit 1):

```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"det(): argument 1 must be a square matrix; got Natural.","recoverable":true,"diagnostics":[{"message":"det(): argument 1 must be a square matrix; got Natural.","position":27,"line":1,"column":28}],"elapsed":"1.96 ms","elapsedTime":{"value":1.96,"unit":"ms"},"timings":[{"position":0,"elapsed":{"value":1.59,"unit":"ms"},"resultKind":"Text","hasOutput":false},{"position":27,"elapsed":{"value":171.6,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":false}],"output":[]}
PLOT DIR LISTING: ["plot.svg(14750 bytes)"]
```

Run 2 (exit 1):

```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"det(): argument 1 must be a square matrix; got Natural.","recoverable":true,"diagnostics":[{"message":"det(): argument 1 must be a square matrix; got Natural.","position":27,"line":1,"column":28}],"elapsed":"1.95 ms","elapsedTime":{"value":1.95,"unit":"ms"},"timings":[{"position":0,"elapsed":{"value":1.61,"unit":"ms"},"resultKind":"Text","hasOutput":false},{"position":27,"elapsed":{"value":155.3,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":false}],"output":[]}
PLOT DIR LISTING: ["plot.svg(14750 bytes)"]
```

**What is wrong.** The first statement really plotted (14,750 bytes of SVG, the same size in both runs)
and `timings[0].resultKind` is `Text` — but the error envelope publishes no `plot` block and no `result`,
so the path the caller needs in order to find the artifact appears nowhere: the only publication
channel the protocol defines for a plot ("the `plot` block carries that capture", protocol:42–45) is
absent, and a caller can only guess `--plot-dir` + `--plot-file`. On the **success** path everything
agrees (I-24: `result.display` = `plot.path` = the file on disk, and the envelope's SVG equals the file
byte-for-byte). Charged P2, not P1, because `plot()`'s return value is a *result* and a failed run
publishes no result by design — but the side effect is real and unmentioned, which is a contract gap
rather than a cosmetic one.


---

## 4. Crossings that held (driven to the edge, no finding)

* **Failure in the middle** — `1+1; det(1); 2+2`: 2 timings at 0 and 5, the second `Void`; the third
  statement did **not** run; the error's `position`/`line`/`column` name the failing statement; the print
  output of earlier statements is carried (F5's fix holds, and a statement that prints *and* then fails
  correctly reports `hasOutput:true`).
* **Parse error before/after valid statements** — `print("a"); 1+1; (`: exit 1,
  `ParseError/ParseError`, `timings:[]` (present and empty), `output:[]` (nothing ran — the whole source
  is parsed first), diagnostic at 18/1/19 in the caller's coordinates, identical on all three surfaces.
* **Empty and degenerate input** — `--eval ''`, `--eval '   '`, closed `--stdin`: `ok:true`, `timings:[]`,
  `output:[]`, exit 0; `;`, `;;`, `1+1;;2+2`, `1+1; ; 2+2` are `ParseError` at the second `;`; a trailing
  `;` or a trailing newline adds no statement (`1+1;` / `1+1` + newline → 1 timing at 0).
* **Line endings and non-BMP** — LF/CRLF/CR give the same statements and the same offsets (0/4/8 for
  `1+1<sep>2+2<sep>3+3`); a file with no trailing newline is identical to one with; an emoji counts as
  **two** UTF-16 units for both `timings[].position` and `diagnostics.position` on all three surfaces
  (a script whose first statement prints U+1F600 and whose second is `det(1)` reports the second at
  offset 13, not at the 12 a code-point count would give). BOM was **not** re-probed (closed in `effd054`).
* **Input-source precedence** — `--eval` wins over `--file` over `--stdin`; the last of a repeated flag
  wins; a bare positional path sets the file only if no `--file` was given; `-` is `Unknown argument '-'`
  (exit 2).
* **Usage errors** — 12 spellings (`--cancel-after 0/-1/abc/2147483648`, missing value;
  `--print-budget 0/-5/abc`; unknown flag; no input at all; `--eval`/`--file` with no value) all: exit 2,
  **0 bytes on stdout**, one `Error: …` line plus the usage block on stderr. `--cancel-after 2147483647` is
  accepted and answers `ok:true` with `stopped:false`.
* **print-budget times text times omit flags** — `--print-budget` changes neither the `--text` summary
  nor `output[]`/`display`; a budgeted `pretty` is a real prefix ending in ` …`, always with
  `truncated:true`, a `node-budget`/`depth-limit`/`digit-cap` reason and a numeric `budget`.
* **Two evaluations of one text in one invocation** — there is no such surface: `--eval A --eval B`
  evaluates **B once** (and `--stdin` alongside is ignored). The doc's two-evaluation sentence was
  therefore checked as its revision numbers instead (I-27: 81/81/82 as written).
* **Determinism** — 45 programs × 2 runs, plus four historical nondeterminism suspects
  (`series(abs(x),…)`, `noise`, `dft`, `evalf(pi(),30)`): byte-identical apart from durations.

## 5. Reproduced but already recorded — **not** counted as findings

| item (recorded) | reproduced here |
|---|---|
| M-1 / EVD-332 — the interrupted statement's print output is dropped and `hasOutput` is false | `for i in 1..5000000 { print(i) }` `--cancel-after 30` → `output:[]`, `partialOutput:[]`, `hasOutput:false`, 0 lines |
| I-1 / `d3a1f50` — the prompt-stop ledger | `sum(1..20000000)` `--cancel-after 30` → `stopped:true, exceeded:false, excessMs:0`, no diagnostic (the amended semantics) |
| `--omit-variables` ignored on the cancelled path | `--cancel-after 30` with the flag → `partialVariables` present |
| `--text` drops the committed output on the failing path | `print("hi"); det(1)` `--text` → stdout 0 bytes, the `Error [code/category]` line on stderr |
| raw framework text in caller-level messages (K-1 / `pi(30)` class) | `--plot-dir ''` → `Cannot use plot directory '': The value cannot be an empty string. (Parameter 'path')`; `--file <missing>` → `…: Could not find file '…'.` |
| eager range materialisation | not re-run (cost note only) |


## 6. Could not check (an untested area is not a clean area)

1. **`Diagnostic.location` as a `DiagnosticLocation` record.** The doc's second branch
   (`start_line`/`start_column`/`end_line`/`end_column`) was never produced: all four Diagnostics I
   reached (`solve.unrepresented-roots`, `integration.unevaluated`, `integration.no-closed-form` twice,
   `limit.unevaluated`) carry `{"kind":"Null"}`. Zero samples of the documented alternative.
2. **`recoverable: false`.** The doc's example is `transform.unsatisfiable-conditions`; I could not reach
   any diagnostic with `recoverable:false`. The nearest behaviour — `assume(x > 5); assume(x < 1)` — is a
   *thrown* `UnsatisfiableAssumptions`/`DomainError` (exit 1), not a diagnostic.
3. **`transform.budget-exceeded`** and **`RuleClassification`**: no documented call produced either the
   code or the enum.
4. **`SolveStatus.BudgetExceeded`** (the fifth row of the doc's mapping table): no documented surface sets
   a solver budget; `--cancel-after` yields the host-level `Cancelled`, not this status. The other four
   rows were measured (I-18).
5. **`classification` fields** (protocol:65): no record in the corpus carries one.
6. **Two evaluations of the same text inside one invocation**: the CLI has no such mode (section 4), so
   the only measurable part of the doc's sentence is the revision numbers.
7. **`--print-budget` non-monotonicity** (known-open): not re-measured; my runs asserted only the
   prefix/truncation contract, not monotonicity across N.
8. **Concurrency and locale**: one process at a time, invariant culture only (the invariant-culture claim
   was not probed under a comma-decimal locale).
9. **Encodings other than UTF-8/LF/CRLF/CR** (e.g. a UTF-16 script file) were not driven through `--file`.
10. **Four atoms were dropped from the corpus because *my* call was wrong** (EVD-330 discipline):
    `linsolve([[1,2],[3,4]],[5,6])` ("requires a symbolic matrix A"), `collect(x^2 + x*y + x, x)` and
    `hessian(x^2*y, [x, y])` (needed `y = symbol("y")` — collected without it), and `evalir(1, [1], 5)`
    (needs a `compile()` IR). Their corrected forms were not re-entered into the corpus.
11. **Plot outside the plot directory** via `--plot-file ..\escaped.svg` **works** (the file lands one
    level up and `plot.path` reports that real path). It is the caller's own argument, so I did not charge
    it; absolute paths and UNC targets were not tested.

## 7. Findings table

| # | severity | one line | runs | surface |
|---|---|---|---|---|
| N-1 | **P1** | the trailing line of captured print output is dropped while the same statement's `hasOutput` is `true` | 2 (+5 controls) | `--eval`/`--file`/`--stdin`, success and failure paths |
| N-2 | P2 | `--omit-variables` removes the variable lines from the `--text` summary, but the payload-control paragraph says the omit flags change no display | 2 + 2 | `--text` |
| N-3 | P2 | the documented `Function` value kind is unreachable; a function name is `Undefined variable` | 2 (+5 forms) | `--eval` |
| N-4 | P2 | `CancelStatus`/`CancelResult` are published but absent from the document's closed lists (and `CancelResult` has no `diagnostics`) | 2 | `--eval` |
| N-5 | P2 | a plot written before a later failure survives on disk while the error envelope publishes neither `plot` nor the path | 2 | `--eval` + `--plot-dir` |

**Verdict on the documented surface.** Of 27 mechanical invariant rows, **24 PASS**, **3 FAIL**
(I-4/I-7 are the two faces of N-1, I-14 is N-2), and 5 documented branches are **UNTESTABLE** from the
documented surface (6.1–6.5). The strongest results are the ones a state-machine fuzzer is for:
778 runs with **zero** exit/`ok` disagreements, **zero** position mismatches (including non-BMP, CRLF
and CR), **zero** cross-surface differences, **zero** nondeterminism, **zero** value-form or
Diagnostic-shape violations, and 111/111 Language.md examples honoured by the binary. The generator's
one language-level failure is N-1, which every surface and both the success and failure paths share.
