# Round 23 — Audit R: the envelope as a CONSUMER's data structure

**Persona R — reconstruction, not inspection.** The product's source was never read: the published
envelope and the published protocol (`docs/symbolics/dsh-protocol.md`) are the only things that exist.
Everything below is what a consumer of the wire can see.

**Artefact under test:** `C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe`
(Native AOT, 5,767,680 bytes, written 2026-09-12 21:51:38 — verified by `Get-Item`).
Invoked as `$BIN` below. Nothing under `out/` was rebuilt or deleted; no build/test command was run;
git state untouched.

**Method (raw bytes, never a pipeline).** Every case was run inside a `.bat` executed by `cmd /c`,
which is the only shim that both survives PowerShell 5.1's argv stripping of `"` (EVD-299) and preserves
raw bytes:

    "%BIN%" --file <case.ls> [flags] > raw\<case>.out 2> raw\<case>.err
    echo %ERRORLEVEL% > raw\<case>.rc

The exit-code line is a **separate cmd line on purpose**: in the one-line
`cmd & echo %ERRORLEVEL%` form cmd expands `%ERRORLEVEL%` before the run, so the recorded code is the
*previous* case's. Two early batches used that form; every rc quoted below was re-measured with the
separate-line form. Every case is driven from a `.ls` file (never `--eval`), so no script text crosses
argv and no probe measures the probe (EVD-330).

Harness: `r23-R\run.bat`, `run2.bat`…`run5.bat`, `text.bat`, analysers `analyze.js`, `check.js`,
`check2.js`, `check3.js`; corpus 40 runs in `r23-R\ls`, captures in `r23-R\raw`.
(The `--text` cases are the redirection-risk cases, so they were captured twice, by two different
`.bat` files, with identical bytes.)

---

## 1. Corpus

| case | script (`r23-R\ls`) | flags | rc | stdout | stderr |
|---|---|---|---|---|---|
| c01 | `print("a"); print("b")` | — | 0 | 15646 B | 0 B |
| c03 | `print("x"); 1+1; print("y")` | — | 0 | 15744 B | 0 B |
| c04 | `print("héllo ☃ 🙂"); 1` | — | 0 | 15898 B | 0 B |
| c05 | `print(""); print("after")` | — | 0 | 15656 B | 0 B |
| c06 | `print("before"); det(1)` | — | 1 | 644 B | 0 B |
| c07 | `det(1); print("after")` | — | 1 | 545 B | 0 B |
| c08 | `print("p"); (` | — | 1 | 568 B | 0 B |
| c09 | `p = plot(1..5); p` | `--plot-dir` | 0 | 32934 B | 0 B |
| c10 | `plot([1,2,3]); det(1)` | `--plot-dir` | 1 | 545 B | 0 B |
| c11 | `print("a"); x = 42; x` | — / `--stdin` | 0 | 16075 B (stdin) | 0 B |
| c11t / c11to | same | `--text` / `--text --omit-variables` | 0 | 39 B / 19 B | 0 B |
| c12 | `for i in 1..3000000 { print(i) }` | `--cancel-after 60` | 1 | 770 B | 0 B |
| c13 | `print("keep"); sum(1..20000000)` | `--cancel-after 30` | 1 | 655 B | 0 B |
| c14/c15 | `det(1)` / `compile_full(1)` | — | 1 | 545 / 522 B | 0 B |
| c16 | `x = symbol("x"); solve(x^2 - 4 == 0, x)` | — | 0 | 21054 B | 0 B |
| c18/c38 | `z = symbol("z"); y = expand((z+1)^8); y` | `--print-budget 20` | 0 | 17629 B | 0 B |
| c19 | `solve_full(x^2 + 1 == 0, x, real())` | — | 0 | 20188 B | 0 B |
| c21/c21crlf/c21bom/c21nonl | `a = 1`/`b = 2`/`det(a)` | — | 1 | 737/736/737/732 B | 0 B |
| c22/c33/c37 | `setprecision("x")` / `setprecision(5, 6)` / `dft("x")` | — | 1 | 599/520/… B | 0 B |
| c28missing | `--file <nonexistent>` | — | 1 | 520 B | 0 B |
| c29/c30/c31 | real-LF prints `print("a<LF>b")`,`…b<LF>`,`a<LF><LF>b` | — | 0 | ~15.8 kB | 0 B |
| c36 | `print("a"); print()` (the N-1 control) | — | 0 | 15652 B | 0 B |
| r0/r1 | `x = -2; x^2 - 4` / `x = 2; x^2 - 4` (values taken **from the envelope**) | — | 0 | 15780 B | 0 B |
| t12/t13 | `for i in 1..3000000 { print(i) }` / `print("keep"); sum(1..20000000)` | `--text --cancel-after` | 1 | **0 B** | **79 B** |

---

## 2. Invariants checked — one row per invariant

| # | invariant (protocol line) | verdict | command | observed |
|---|---|---|---|---|
| R-I1 | **Invariant 1, direction A**: every byte stdout carries is the envelope and nothing else (protocol:9) | **PASS** | 37 JSON runs, `cmd` redirection, `analyze.js` | on every run: `stdout == JSON.parse-able body + CRLF` (last bytes `7d 0d 0a`), no tail bytes, no second line, **stderr = 0 B in all 37** |
| R-I2 | **Invariant 1, direction B**: every byte the script printed is in the envelope (`output[]`) | **PASS** | `--file c36.ls` (the N-1 case) | `print("a"); print()` → `"output":["a",""]`, **both** timings `hasOutput:true` (15652 B, `revision:80`) — the N-1 repair holds; `c05` → `["","after"]`, `c29` → `["a","b"]`, `c30` → `["a","b",""]`, `c31` → `["a","","b"]` — all four are exactly *"concatenate each print()'s text plus one terminator, split, drop one trailing terminator"* |
| R-I3 | `output[]` is the print stream, so `hasOutput` counts it (protocol:172,181) | **PASS** | `c03`/`c05`/`c29` | `c03`: `hasOutput` = `[true,false,true]`, `output` has 2 entries; `c29`: one `hasOutput:true` statement → 2 lines |
| R-I4 | the error path keeps the committed print stream | **PASS** | `--file c06.ls`, `--file c07.ls` | `c06`: `ok:false,code:InvalidArgument` **carries `"output":["before"]`**; `c07`: `"output":[]` — correct, the failure preceded the print |
| R-I5 | a position indexes the caller's text, `text.Substring(position)` names the statement (protocol:240–252) | **PASS** | `c21`/`c21crlf`/`c21bom`/`c21nonl`, `c11`/`c26stdin` | `a=1`/`b=2`/`det(a)`: third statement at **12** (LF), **14** (CRLF), **15** (BOM+CRLF) — byte-exact doc:254–259; the slice at each position starts at that statement's first token; `--file` and `--stdin` of the same bytes give identical 0/12/20 |
| R-I6 | `diagnostics[].position` and `timings[].position` cannot disagree (protocol:303–307) | **PASS** | `c06`, `c21` | `c06`: `timings` `[0,17]`, diagnostic `pos=17 L1 C18`; `c21`: `timings` `[0,6,12]`, diagnostic `pos=12 L3 C1` |
| R-I7 | durations are structural — no consumer parses a unit suffix (protocol:29–31, 280–282) | **PASS** | 10 envelopes incl. parse error, unreadable file, cancelled | `elapsed` ↔ `elapsedTime` agree value **and** unit in all 10 (`"93.9 µs"`↔`{93.9,"µs"}` … `"0 ns"`↔`{0,"ns"}`); `timings` present-and-empty on the parse error (`c08`) and the unreadable file (`c28missing`) |
| R-I8 | the truncation flag is honest (protocol:18–28) | **PASS** | `--file c38.ls --print-budget 20` vs `c39` control | truncated: `"truncated":true,"truncationReason":"node-budget","budget":20`, `pretty` = `z^8 + 8*z^7 + 28*z^6 + 56*z^5 +  …` — the cut is on a **token boundary**, the last kept token is whole; control run has none of the three fields; `display` stays full-length as documented |
| R-I9 | an absent field is `{"kind":"Null"}`, never `""` (protocol:15) | **PASS (sampled)** | `c16`, `c19` | empty `diagnostics`/`conditions`/`families` cross as `{"kind":"Array","type":"Vector","shape":[0],"elements":[]}`; `unrepresented_reason` is `{"kind":"Text","value":""}` — the doc's own example (protocol:172); no sampled slot is ever both `Null` and empty `Text` |
| R-I10 | `solutions[].value` fed back into the equation gives residual 0 | **PASS** | envelope of `c16` → `r0.ls`=`x = -2; x^2 - 4`, `r1.ls`=`x = 2; x^2 - 4` | `"display":"0"` / `"display":"0"` — the published `pretty` values are re-feedable source and the residual is exactly 0 (`Integer`/`Natural`) |
| R-I11 | `revision` is real, monotone-per-engine and NOT a statement count; the doc's numbers are live (protocol:127–130) | **PASS** | `c16` / `c17`=`1+1` / `c32`=`1+1;1+1;1+1` | **82 / 81 / 81** — byte-exact agreement with the document |
| R-I12 | `complete` and `completeness` cannot disagree (protocol:80–83, 203–214) | **PASS (sampled)** | `c16`, `c19`, `c35` | `Solved/true/Complete`; `NoSolutions/true/Complete`; no counterexample in 6 solve envelopes |
| R-I13 | nothing representable ⇒ `Unevaluated`, never a complete-looking subset (protocol:199–201) | **PASS** | `--file c19.ls` = `solve_full(x^2 + 1 == 0, x, real())` | `status:NoSolutions, complete:true, completeness:Complete, represented_count:0, unrepresented_count:0` — over the **requested** domain the set is provably empty, which is the doc's own `NoSolutions` case |
| R-I14 | the argument-refusal grammar (protocol:289–301) | **PASS for arity / FAIL for consistency** | `c15`=`compile_full(1)`, `c33`=`setprecision(5, 6)` vs `c14`=`det(1)`, `c22`=`setprecision("x")`, `c37`=`dft("x")` | arity: `InvalidArgument`/`TypeMismatch`, message byte-identical to the doc's example; wrong **type**: two different machine branches — see **R-2** |
| R-I15 | the document's own examples, byte for byte | **PASS (4/4 checked)** | §3 below | `compile_full` message, revisions 82/81/81, positions 12/14/15, error-envelope shape |
| R-I16 | the cancelled path: `output[]` + `partialOutput[]` reconstruct one print stream | **FAIL (ambiguous)** | `c13`, `c12` | `c13`: `"output":["keep"]` **and** `"partialOutput":["keep"]` — the same entry in both, with no field saying whether `partialOutput` is additive or a superset; concatenating (the reconstruction a consumer is told to do) double-counts. See **R-4** |
| R-I17 | the cancellation ledger is machine-checkable arithmetic | **FAIL** | `x1..x4`, `c12`, `c13` | `excessMs ≠ elapsedMs − budgetMs` on every exceeded run. See **R-1** |
| R-I18 | `--text` is only a display change (protocol:40–45) | **FAIL (display only)** | `c11 --text`, `c11 --text --omit-variables`, `c12 --text --cancel-after 60` | the `--text` stream opens with a **nameless`= 42 (Natural)` line** and carries no envelope; on cancellation stdout is **0 bytes** and the only record is 79 B of human text on stderr. See **R-3** (partly already recorded as "–text drops the committed output") |

---

## 3. The document's own examples, byte for byte

| doc line | the document says | the binary says | verdict |
|---|---|---|---|
| protocol:294–297 | `compile_full(): expected 2 arguments; got 1.` with `code: InvalidArgument, category: TypeMismatch, recoverable: true` | `c15` → `{"ok":false,"code":"InvalidArgument","category":"TypeMismatch",...,"message":"compile_full(): expected 2 arguments; got 1."}` | **byte-identical** |
| protocol:127–130 | `82` for `x = symbol("x"); solve(x^2 - 4 == 0, x)`, `81` for `1 + 1` three times | `c16` → `"revision":82`; `c17` → `81`; `c32` → `81` | **exact** |
| protocol:254–259 | third statement at 12 (LF) / 14 (CRLF) / 15 (BOM+CRLF) | `c21`=12, `c21crlf`=14, `c21bom`=15 | **exact** |
| protocol:265–282 | error envelope: `diagnostics` parse-form, `elapsedTime` **and** `timings` present, `timings` empty for a pre-engine error | `c08` (parse error) and `c28missing` (unreadable file): both carry `elapsedTime` and `"timings":[]` | **exact** |
| protocol:90 | empty array is `{"kind":"Array","shape":[0],"elements":[]}` | live carries `"type":"Vector"` too | **already recorded (J-2)** — not re-reported |
| protocol:172 | `unrepresented_reason` = `{"kind":"Text","value":""}` | `c16` exactly that | **exact** |
| protocol:42 | `--text` lines are "the variables array rendered" | first line is `= 42 (Natural)` with no variable name | **drifted — R-3** |

---

## 4. Findings

### R-1 — P2 (P1 if the ledger is ever documented): `cancellation.excessMs` contradicts `elapsedMs − budgetMs`
Two machine-readable fields of the **same envelope** compute the same fact — how far past the deadline
the run went — and disagree by a systematic **+0.23 … +0.33 ms** on every `exceeded:true` run. The
disagreement is not noise: it is one-sided and reproducible, and it is absent on the `exceeded:false`
control, which pins it to the ledger and not to my extraction.

**Reproduction 1** (`r23-R\run5.bat`, exact lines, separate rc line as required):
```
"%BIN%" --file C:\...\r23-R\ls\c13b.ls --cancel-after 30 > C:\...\r23-R\raw\x2.out 2> C:\...\r23-R\raw\x2.err
echo %ERRORLEVEL% > C:\...\r23-R\raw\x2.rc
```
verbatim (fields extracted from `x2.out`):
```
budgetMs=30 elapsedMs=44.304 excessMs=14.551 exceeded=true   elapsedMs-budgetMs=14.304   DIFF=+0.247
```
**Reproduction 2** (same `.bat`, third line):
```
budgetMs=30 elapsedMs=32.815 excessMs=3.049  exceeded=true   elapsedMs-budgetMs=2.815    DIFF=+0.234
```
Supporting runs, same `.bat`: `x4` `32.536 / 2.766` **+0.230**, and `c12`
(`--cancel-after 60`, `68.031 / 8.357`) **+0.326** — the same +0.33 ms offset at a different budget, so the offset is not tied
to one deadline. Control, *not* exceeded: `x1` `elapsedMs=20.443, budgetMs=30, excessMs=0, exceeded=false`;
`c13` `24.171/30/0/false` — the ledger reports exactly 0 when the deadline was met, so the offset appears
only on the overshoot path.
Consumer impact: an agent that sizes the next budget as `budget − excessMs` (or that reports the
overshoot to a user) gets a number that its own sibling fields refute.

### R-2 — P2: the same fact (an argument of the wrong TYPE) crosses with two different machine branches
`det(1)` refuses as `InvalidArgument`/`TypeMismatch`; `setprecision("x")` and `dft("x")` refuse the same
kind of fact as `InvalidOperation`/`DomainError`. Arity refusals are uniform (both `c15` and `c33`
answer `InvalidArgument`/`TypeMismatch`), so the split is type-specific, not surface-specific. A
consumer that switches on `category` to decide "my argument is the wrong type → convert it" versus
"the math domain is wrong → change the problem" cannot: the branch it lands on depends on the builtin.
Scope note (honesty): the doc's `InvalidArgument` sentence is scoped to the wrong **number** of
arguments, so this is an internal inconsistency of the machine API rather than a literal doc
contradiction — hence P2, not P1.

**Reproduction 1:** `"%BIN%" --file C:\...\r23-R\ls\c37.ls > c37b.out 2> c37b.err` then
`echo %ERRORLEVEL% > c37b.rc` → rc=1, verbatim:
```
{"code":"InvalidOperation","category":"DomainError","recoverable":true,"message":"DSP builtins expect an array argument, but got a scalar."}
```
**Reproduction 2:** `"%BIN%" --file C:\...\r23-R\ls\c22.ls > c22.out 2> c22.err` → rc=1, verbatim:
```
{"code":"InvalidOperation","category":"DomainError","recoverable":true,"message":"setprecision() expects a Natural or Integer digit count, but got 'Text'."}
```
Opposite branch, same class of fact: `det(1)` → `{"code":"InvalidArgument","category":"TypeMismatch",...,"message":"det(): argument 1 must be a square matrix; got Natural."}` (`c14`, rc=1).

### R-3 — P2 (display path): the `--text` stream loses its shape, and on cancellation loses everything
`--text` for `print("a"); x = 42; x` is **39 bytes**:
```
3D 20 34 32 20 28 4E 61 74 75 72 61 6C 29 0D 0A 61 0D 0A 20 20 5F 20 3D 20 34 32 0D 0A 20 20 78 20 3D 20 34 32 0D 0A
= 42 (Natural)\r\n a\r\n "  _ = 42"\r\n "  x = 42"\r\n
```
i.e. the stream **opens with a nameless `= 42 (Natural)` line** (every real variable line is
`"  <name> = <value>"`), then the committed print output, then the variables. With
`--omit-variables` the legitimate lines go and the nameless line **stays** (19 bytes:
`= 42 (Natural)\r\na\r\n`) — so the one line a consumer cannot attribute to any variable is the one
line the flag does not remove. Independent samples, verbatim: `u01` `x = 42; x` → 36 B
`3D 20 34 32 20 28 4E 61 74 75 72 61 6C 29 0D 0A 20 20 5F 20 3D 20 34 32 0D 0A 20 20 78 20 3D 20 34 32 0D 0A`
(`= 42 (Natural)` / `  _ = 42` / `  x = 42`) — the bare line *precedes* a correctly named `  _ = 42`; and
`u02` `print("a"); 1+1` → 27 B `= 2 (Natural)\r\na\r\n  _ = 2\r\n`, where the bare line even precedes the
captured print output. `t01`/`t03` (Void last statement) are clean `a\r\nb\r\n` / `x\r\ny\r\n`, so the stray
line is tied to the presence of a result, not to `print`.
On the **cancelled** path `--text` writes **0 bytes to stdout** and 79 bytes to stderr, rc=1:
```
ERR: Error [Cancelled/BudgetExceeded]: the evaluation was cancelled by the caller.
```
Reproduced on two scripts (`t12b` `for i in 1..3000000 { print(i) }` `--cancel-after 60`; `t13b`
`print("keep"); sum(1..20000000)` `--cancel-after 30`) — both 0 B stdout / 79 B stderr / rc=1. The
"committed output is dropped on `--text`" half is **already recorded**; what is new here is the
byte-level shape: the nameless first line, and that the cancellation reason exists *only* on stderr.

### R-4 — P2: on the cancelled path `output[]` and `partialOutput[]` duplicate the same entries
`c13` (`print("keep"); sum(1..20000000)` `--cancel-after 30`) verbatim:
```
"output":["keep"], "partialOutput":["keep"], "partialVariables":[],
"cancellation":{"budgetMs":30,"elapsedMs":24.171,"stopped":true,"exceeded":false,"excessMs":0}
```
and `c12` verbatim: `"output":[], "partialOutput":[], "cancellation":{"budgetMs":60,"elapsedMs":68.031,
"stopped":true,"exceeded":true,"excessMs":8.357}`. The two arrays are never *disjoint* in my corpus, so
a consumer cannot tell whether `partialOutput` is "everything, including what `output` already has" or
"only the extra lines of the interrupted statement". A reconstruction that concatenates them — the
natural reading of "output[] (+ partialOutput[] where present)" — double-counts `keep`. The doc names
neither field (already recorded, B3-F15), so this is an ambiguity rather than a contradiction; it is
filed because it is exactly the shape a consumer hits first.

---

## 5. Could NOT check (an untested area is not a clean area)

1. **Non-BMP positions** (protocol:245): the emoji probe `c04` carries the emoji in `output[]`, but no
   probe placed a non-BMP character *before* a reported `position`, so the "two UTF-16 code units" rule
   is unmeasured. Same for a CR-only (classic-Mac) file — protocol:261–263 claims CR ends a line; only
   LF, CRLF and BOM+CRLF were run.
2. **`Diagnostic.location` non-Null branch**: every diagnostic I produced is `{"kind":"Null"}`;
   round-22 J already records that no shipped producer builds a `DiagnosticLocation`, so the second
   half of protocol:117–119 stays untestable from the wire.
3. **`truncationReason:"depth-limit"` and the `evalf(f, digits) > 1000` digit cap** (protocol:24–28):
   only `node-budget` was exercised (`c38`). The digit-cap path was not probed at all this round.
4. **Plot byte-equality**: I checked that the `plot` block is present and that `c10` publishes none after
   a later failure (`plot([1,2,3]); det(1)` → error envelope, no `plot`), but did not re-diff the SVG
   bytes against the block. N-5 territory; not re-derived.
5. **stderr on the JSON path**: 0 bytes in all 37 JSON runs, but no probe closed stdout or broke the
   pipe, so "nothing else ever reaches stderr" is unmeasured for I/O failures.
6. **JIT twin** (`Lovelace.Run\bin\Release\net10.0\Lovelace.Run.exe`): not run at all; every number here
   is the AOT artefact.
7. **Two captures vanished silently**: in the two batch runs where a case line for `c11` (and once
   `c11t --text`) sat between a `--plot-dir` case and an `--omit-variables` case, no `.out`/`.err`/`.rc`
   file was created for those cases at all (not even an empty one), while the surrounding cases ran.
   I could not root-cause it inside the file sandbox and did not assume product blame: every conclusion
   about those cases was re-measured by direct invocation, and no finding above rests on a missing
   capture. Recorded because a harness that silently drops a run is a measurement hazard for the next
   wave.
8. **`revision` monotonicity *within* an engine**: one envelope per process by construction, so the
   doc's "monotone within one engine" claim is not observable from a single run; only the fresh-process
   numbers (82/81/81) were verified.
9. **Concurrency, `--plot-dir` failure modes, `--stdin` with CRLF/BOM, and `--eval` positions** were
   left to the waves that already cover them (N rows I-6/I-10/I-23) — I used `--file`/`--stdin` only.
