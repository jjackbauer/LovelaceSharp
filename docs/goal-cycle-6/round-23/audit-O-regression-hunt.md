# Round 23 — Audit O: fresh-eyes regression hunt on cycle 6's OWN fixes

**Auditor:** persona O (did not write any of the fixes). **Strategy (not used by earlier waves):** read each
cycle-6 fix's diff and its own tests, then attack **one step outside** the case each test pins.

**Artefact under test (unless a row says otherwise):** `C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe`
(Native AOT, 5,764,608 bytes, 2026-09-12 21:05:23, published from HEAD dfae21b / product c8a6966).
**The JIT twin was NOT used** — every execution row below is the AOT binary named above; rows marked
"source read" used only `git show`/`read` on the checkout and ran nothing.
Scratch scripts (`.ls`) live outside the repo in `%TEMP%\auditO`; nothing under `out/` was touched, no
build/test command was run, git state was only read.

**Probe-validity note (EVD-330):** my first series batch was invalid (`series(abs(x), x, 0, 3)` answers
"Undefined variable 'x'" — the variable needs `x = symbol("x")`); that batch measured the probe and was
discarded and re-run. All scripts below were validated by first observing a **known-good** answer on the
same shape before trusting any failure.

---

## 1. One row per fix

| # | Fix | Verdict | Command (all `--json --omit-functions --omit-variables` unless noted) | Observed (AOT) |
|---|-----|---------|----------------------------------------------------------------------|----------------|
| 1 | **9852a2f** series at a kink | **SURVIVED** (20 boundary probes; no wrong value, no Guid leak found) | `--file kink.ls` over: abs(±x), abs(±2x), abs(3x−6), abs(x²), abs(x³), abs(x+1), abs(x−1), x·abs(x), abs(x)², abs(x)^½, abs(sin x), abs(cos x), abs(x)@x=1, abs(x²−1)@o2/o3, 1/abs(x), abs(x)/x, x²/abs(x), abs(a·x) with `a` a symbol, abs(x³−x), abs(x−x³) | All match the SymPy `dir='+'` reading, e.g. `series(abs(x),x,0,3)` → `x + O(x^3)`; `series(abs(-2*x),x,0,3)` → `2*x + O(x^3)`; `series(abs(sin(x)),x,0,4)` → `x - 1/6*x^3 + O(x^4)`; `series(abs(x)^(1/2),x,0,3)` → `sqrt(x)`. **No `__t`/Guid/`diff(`/`piecewise(` in any published value.** A symbol literally named `__t` in scope does not collide (`series(abs(__t*x),x,0,3)` → `__t*x*sign(__t) + O(x^3)`, deterministic). |
| 2 | **6f7e371** series across a pole/branch point | **SURVIVED** (25 probes; Laurent coefficients hand-checked) | same style: 1/x@o2/o5, 1/x³, sin(x)/x², 1/sin(x), tan(x), 1/(x·sin x), sin(x)/x³, 1/(x²(1+x)), 1/(x(1+x)), (1−cos x)/x², control 1/(x−1), x^½, x^{3/2}, 1/√x, √x@o2/o3, √x@x=1, √(x²), √x·√x, log(x), exp(1/x), √abs(x) | `1/x^3`→`x^-3 + O(x^2)`; `sin(x)/x^2`→`x^-1 - 1/6*x + O(x^3)`; `1/(x*sin(x))`→`x^-2 + 1/6 + 7/360*x^2 + O(x^3)` (**coefficient verified by hand**: 1/(x·sin x) = x⁻²(1 + x²/6 + (1/36−1/120)x⁴) → 7/360 ✓); `sqrt(x)`→`sqrt(x)`; `sqrt(x)*sqrt(x)`→`x + O(x^3)`. Control `1/(x-1)`→`-x + O(x^2) - 1` unchanged. **But see O-2** (the same builtin's *order* argument is a raw CLR message). |
| 3 | **effd054** one ScriptText reader / BOM | **SURVIVED** on every surface I could drive | see §3 for the full matrix | `--file`, `--stdin` (real process, `cmd /c "type f | exe --stdin"`, and `Get-Content -Raw | exe`) and `--eval` with the **same bytes** all answer positions **1 / 7 / 13** and `position 13 line 3 col 1` for ⟨BOM⟩a = 1\nb = 2\ndet(a)\n. UTF-16LE+BOM+CRLF (1/8/15, equal to `--eval` of the same decoded text), UTF-16BE+BOM (1/7/13) and UTF-32LE+BOM (1/7/13) all decode. Mid-file BOM (2nd statement) is **refused at its own offset** (`position 6 line 2 col 1`). BOM inside a string literal is data (`print("a⟨BOM⟩b")` → `a\uFEFFb`). BOM-only file and empty file both answer `ok:true` with empty timings. → **O-3** is the one crack. |
| 4 | **fd658d0 + d3a1f50** cancel ledger | **SURVIVED** (24-run battery) | `--file long.ls --cancel-after {30,100,300}` × 8 each (`long.ls` = `sum(1..20000000)`) | Invariant held in **24/24**: `exceeded ⇔ excessMs > 0` and **exactly one** `cancellation deadline exceeded` diagnostic **iff** `exceeded`. The documented prompt stop (`stopped:true, exceeded:false, excessMs:0, diags:0`) still occurs (7/24) — that is the d3a1f50 reading, not a new defect. Full table in §4. |
| 5 | **5e85c28** capability prose (four typed refusals) | **SURVIVED** (prose is true of the binary) | `--eval 'mean(1)'`, `'max(1,2)'`, `'sum(1, 5)'`, and 300-deep nesting | `mean(1)` → `InvalidArgument/TypeMismatch :: mean(): argument 1 must be an array or vector; got Natural.` — **verbatim the sentence the prose quotes**; `max(1,2)` → same shape naming `max()`; `sum(1, 5)` → same shape naming `sum()`; 300-deep parens/blocks/arrays/calls/if → `DepthExceeded/BudgetExceeded` ("expression/statement nesting depth 257 exceeds the maximum supported nesting depth of 256"), 600-term flat chain → "expression tree depth 513 exceeds … 512". No internal failure, no abort. |
| 6 | **8d61b4b** `--omit-*` help sentence | **SURVIVED** | `--eval '1+1' --json --omit-functions --omit-variables` | Envelope carries `"variables":[]` and `"functions":[]` — **present and empty**, which is what the new sentence says ("send an EMPTY … , not the populated one"). With neither flag both arrays are populated. |
| 7 | **7df2a68** evalf clamp + printer | **SURVIVED on the clamp; the printer half untested** (see could-not-check 7) | `--eval`/`--file` with `setprecision(5000); evalf(sqrt(2), 5000)`, `evalf(sqrt(2), 1500)`, `evalf(sqrt(2), 1000)`, `evalf(sqrt(2), 1001)`, `evalf(1/2, 5000)`, `pi(1000)`, `pi(1001)`, `setprecision(5000); evalf(sqrt(-2), 5000)` | `structured.truncated=true, truncationReason="digit-cap", budget=1000` for the 5000-, 1500- and 1001-place requests; **`truncated` absent** for `evalf(sqrt(2),1000)` (exactly at cap), for the exact `evalf(1/2,5000)`, and for `pi(1000)` — i.e. "exact answer not marked" is true. `pi(1001)` is the K-1 typed refusal naming 1001 and the cap. |
| 8 | **523240b** one structured projection (K-2) | **PARTIAL — CLI side only** (the host side is unreachable from this artefact) | `--eval 'setprecision(1100); pi(1100)' --json` | `result.structured.value` carries **all 1100 digits** (`truncated` absent) — the CLI publishes what the value stores, at computation precision. The *library* half (SuiteEngine.ProjectValue driven by a host) is **not checkable from the binary** → could-not-check 2. |
| 9 | **34ee4b3** plugin factory (K-3) | **NOT CHECKABLE on the binary** (source read only) | `git show 34ee4b3`; grep for any CLI route that loads one plugin into two engines | No CLI surface loads a plugin at all (no `loadplugin` builtin; the Runner builds one `SymbolicsPlugin` and one `SuiteEngine`, Runner.cs:180-182). One instantiation cannot be observed from `--eval/--file/--stdin`. Ledger verdict: **unverified**, not "clean". |
| 10 | **2572c59** K-1 digit refusals | **SURVIVED for pi/e; FALSIFIED one line away** | `--eval 'pi(0)'`, `pi(-1)`, `pi(10^19)`, `pi(10^30)`, `e(10^30)`, `setprecision(1000); pi(2^63)` | `pi`/`e` always answer `InvalidArgument/TypeMismatch` naming the builtin, **the count and the cap** ("…between 1 and 1000 (the engine's computation precision); got 10000000000000000000."). But **`setprecision` — the sibling call four lines above in the same file — still escapes as the raw CLR message K-1 was created to remove → O-1**. |
| 11 | **28d4109** M-2 evaluation budget | **SURVIVED** | `func f(n) { if (n == 0) { return 0 }; return f(n - 1) }; ` + shapes | Refusal number is **stable at 513** in all 8 shapes I drove (`f(86)`, `{1;f(86)}`, `{1;1;f(86)}`, `{1;1;1;1;1;f(86)}`, `1;1;1;1;f(86)`, `print(f(86))`, `f(86) + 0`, `ident(f(86))`) — the 513/514/515 non-monotonicity is gone; `f(85)` answers `0` in `f(85)`, `{1;f(85)}`, `print(f(85))`, `for i in 1..5 { f(85) }`, `{{ {f(85)} }}`. All adjacent nesting budgets (256/512) refuse typed and name their numbers; **no process abort** at 50,000 nested parens/arrays/braces/5000 nested calls. |
| 12 | **f35e5a7** solve reads assumptions (M-3) | **SURVIVED** (10 probes; details §5) | `x = symbol("x"); assume(x > 5); solve(x == 3, x)` and boundary/contradiction variants | Assumption-refuted roots are dropped and the store decides: `assume(x > 5); solve(x == 3, x)` → refuted (no `3`); `assume(x >= 3); solve(x == 3, x)` → `3` (the boundary admits); `assume(x > 3); solve(x == 3, x)` → refuted; `assume(x != 3)` refutes 3; `assume(x < 0); solve(x^2 == 4, x)` → `-2` only; a contradicting second `assume` is refused, not silently absorbed. |
| 13 | **d3a1f50** (test correction) | **SURVIVED** — the reading it writes down is what the binary does | see row 4 | `stopped:true + exceeded:false + excessMs:0 + 0 diagnostics` reproduced; nothing in those envelopes contradicts itself. |

---

## 2. Findings

### O-1 — **P1** — `setprecision(N)` above Int64 still escapes as the raw CLR message K-1 removed from `pi`/`e`

K-1 (2572c59) says in its own doc-comment (Interpreter.cs:1907-1910) that a too-wide count "used to escape
as the raw CLR conversion message (`Value was either too large or too small for an Int64.` as
`ArithmeticError`/`DomainError`)" and fixes it **"applied to pi(n) and e(n)"** (commit message).
`setprecision` sits four lines above those registrations in the same file and still converts with a bare
`long.Parse` (Interpreter.cs:1846-1847). It answers with the **identical raw message, code and category**
K-1 exists to abolish — no numbers, no bound, no builtin name.

Command (script `sp.ls` = `setprecision(123456789012345678901234567890)`):

```
& out\aot\Lovelace.Run.exe --file sp.ls --json --omit-functions --omit-variables
```

Run 1 (verbatim, exit 1):
```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"ArithmeticError","category":"DomainError","message":"Value was either too large or too small for an Int64.","recoverable":true,"diagnostics":[{"message":"Value was either too large or too small for an Int64.","position":0,"line":1,"column":1}],"elapsed":"227.8 µs","elapsedTime":{"value":227.8,"unit":"µs"},"timings":[{"position":0,"elapsed":{"value":70.5,"unit":"µs"},"resultKind":"Void","hasOutput":false}],"output":[]}
```
Run 2 (verbatim, exit 1) — same message, same code/category, different timings only:
```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"ArithmeticError","category":"DomainError","message":"Value was either too large or too small for an Int64.","recoverable":true,"diagnostics":[{"message":"Value was either too large or too small for an Int64.","position":0,"line":1,"column":1}],"elapsed":"225.7 µs","elapsedTime":{"value":225.7,"unit":"µs"},"timings":[{"position":0,"elapsed":{"value":69.5,"unit":"µs"},"resultKind":"Void","hasOutput":false}],"output":[]}
```

Second, independent count (script `sp19.ls` = `setprecision(10000000000000000000)`, two runs, both exit 1):
```
{"…","ok":false,"code":"ArithmeticError","category":"DomainError","message":"Value was either too large or too small for an Int64.","diagnostics":[{"message":"Value was either too large or too small for an Int64.","position":0,"line":1,"column":1}],…}
{"…","ok":false,"code":"ArithmeticError","category":"DomainError","message":"Value was either too large or too small for an Int64.","diagnostics":[{"message":"Value was either too large or too small for an Int64.","position":0,"line":1,"column":1}],…}
```

Context produced by the same binary in the same shape (so the fix *is* there, one builtin away):
`pi(123456789012345678901234567890)` → `InvalidArgument/TypeMismatch :: pi(): argument 1 (digits) must be a digit count between 1 and 1000 (the engine's computation precision); got 123456789012345678901234567890.`
**Angle (b)+(e):** the refusal is not from the fixed taxonomy, its message names no number, and it is a raw
.NET message at the language surface — a standing invariant. K-1 moved the defect's *class* to pi/e only;
`setprecision` is the same bug one line away.

### O-2 — **P1** — the rebuilt `series` still refuses a bad `order` with a raw ArgumentOutOfRangeException

`series` is the builtin 9852a2f and 6f7e371 rewrote (the `(la−lb)` shift, the branch-point rule).
Its `order` argument is still guarded by the kernel, and the refusal at the language surface is
**`Specified argument was out of the range of valid values. (Parameter 'order')`** — raw CLR text naming
no numbers, exactly the shape K-1's doc-comment rejects ("tells a caller neither what was asked for nor what
bounded it"). An order wider than Int64 escapes as the raw Int64 message.

Command (script `so2.ls` = `x = symbol("x"); series(sin(x), x, 0, -1)`):
```
& out\aot\Lovelace.Run.exe --file so2.ls --json --omit-functions --omit-variables
```
Run 1 (verbatim, exit 1):
```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"Specified argument was out of the range of valid values. (Parameter 'order')","recoverable":true,"diagnostics":[{"message":"Specified argument was out of the range of valid values. (Parameter 'order')","position":17,"line":1,"column":18}],"elapsed":"325.9 µs","elapsedTime":{"value":325.9,"unit":"µs"},"timings":[{"position":0,"elapsed":{"value":51,"unit":"µs"},"resultKind":"Symbolic","hasOutput":false},{"position":17,"elapsed":{"value":119.2,"unit":"µs"},"resultKind":"Void","hasOutput":false}],"output":[]}
```
Run 2 (verbatim, exit 1) — same message, positions and code:
```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"Specified argument was out of the range of valid values. (Parameter 'order')","recoverable":true,"diagnostics":[{"message":"Specified argument was out of the range of valid values. (Parameter 'order')","position":17,"line":1,"column":18}],"elapsed":"237.3 µs","elapsedTime":{"value":237.3,"unit":"µs"},"timings":[{"position":0,"elapsed":{"value":49.1,"unit":"µs"},"resultKind":"Symbolic","hasOutput":false},{"position":17,"elapsed":{"value":73.2,"unit":"µs"},"resultKind":"Void","hasOutput":false}],"output":[]}
```

Order 0 behaves identically (two further runs, `so.ls` = `x = symbol("x"); series(sin(x), x, 0, 0)`,
position 17 / line 1 / column 18 in both), and the wide order
(`x = symbol("x"); series(sin(x), x, 0, 123456789012345678901234567890)`) answers the raw
`Value was either too large or too small for an Int64.` again (one run).
**Angle (a)+(b):** the boundary one step outside the fix's own pinned rows (its tests only ever pass a
*valid* order) is a raw .NET refusal in the very function the two series fixes rebuilt.

### O-3 — **P2** — on the BOM path the parser's *message* carries the engine's offset while `diagnostics[].position` carries the caller's

effd054 made every published **position** index the caller's text (BOM included). The parser's refusal
**message** still embeds the engine's own (BOM-stripped) offset, so the two numbers in one envelope
disagree, and a consumer that slices the caller's text at the number *inside the message* slices the wrong
character. Fixture `mid2.ls` = `EF BB BF` + `a = 1\n` + `U+FEFF` + `b = 2\ndet(a)\n` (leading
BOM **and** a BOM before the second statement).

Command:
```
& out\aot\Lovelace.Run.exe --file mid2.ls --json --omit-functions --omit-variables
```
Run 1 and run 2 (verbatim extracts; both exit 1):
```
run 1: message=Unexpected character '﻿' at position 6. | diagnostics.position=7 line=2 col=1
run 2: message=Unexpected character '﻿' at position 6. | diagnostics.position=7 line=2 col=1
```
The caller's own text, sliced at the two numbers (same probe):
```
len=20
slice at 6 => U+FEFF
slice at 7 => U+0062      ('b' — the character the diagnostics entry names)
```
The same off-by-one is structurally present for CRLF (the engine's text collapses each CRLF to one
character) — see could-not-check 5.

### O-4 — **P1 (pre-existing; introduced by none of the 13 fixes)** — three more raw CLR refusals at the language surface

Found while sweeping argument-boundary refusals for O-1/O-2's class (detection rule: a refusal whose message
is a CLR exception text rather than a language refusal). Two runs each, identical but for timings:

1. `[1,2,3][123456789012345678901234567890]` (`vecbig.ls`) — both runs exit 1:
   `"code":"ArithmeticError","category":"DomainError","message":"Value was either too large or too small for an Int64."`
2. `zeros(-1)` (`zerosneg.ls`) — both runs exit 1:
   `"code":"ArithmeticError","category":"DomainError","message":"Arithmetic operation resulted in an overflow."`
3. `zeros(123456789012345678901234567890)` — one run observed in the sweep: the same raw Int64 message.

Outside this cycle's diffs (indexing and array-shape builtins); reported because the standing invariant is
unqualified — **no raw .NET message at the language surface, and a refusal names the numbers.** No root-cause
chase (no code edits allowed).

---

## 3. BOM matrix (effd054) — raw evidence behind row 3

Fixture `pos1.ls` = bytes `EF BB BF 61 20 3D 20 31 0A 62 20 3D 20 32 0A 64 65 74 28 61 29 0A`
(⟨BOM⟩a = 1\nb = 2\ndet(a)\n), the commit's own shape. Every surface answered the **same** refusal with the
**same** positions `[1, 7, 13]` and `position 13 / line 3 / column 1`:

```
--file  pos1.ls                     -> timings [1,7,13], diagnostics position 13 line 3 column 1
--stdin (cmd /c "type pos1.ls | …") -> timings [1,7,13], diagnostics position 13 line 3 column 1
--stdin (Get-Content -Raw | …)      -> timings [1,7,13], diagnostics position 13 line 3 column 1
--eval  ⟨BOM⟩a = 1\nb = 2\ndet(a)\n -> timings [1,7,13], diagnostics position 13 line 3 column 1
```
Extract of the `--stdin` route (the process-level reader the fix's own test cannot exercise, since
`SurfacePositionAgreementTests` hands the runner a `StringReader`):
```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"det(): argument 1 must be a square matrix; got Natural.","recoverable":true,"diagnostics":[{"message":"det(): argument 1 must be a square matrix; got Natural.","position":13,"line":3,"column":1}],"timings":[{"position":1,…},{"position":7,…},{"position":13,…}]}
```
Encoding coverage, same file content:

| bytes | `--file` result |
|---|---|
| UTF-8 + BOM, LF | positions 1/7/13 |
| UTF-16 LE + BOM, CRLF | positions **1/8/15** — and `--eval` of the identical decoded text (`U+FEFF` + `a = 1\r\nb = 2\r\ndet(a)\r\n`) also 1/8/15 → the CRLF map agrees cross-surface |
| UTF-16 BE + BOM, LF | 1/7/13 |
| UTF-32 LE + BOM, LF | 1/7/13 |
| UTF-8, no leading BOM, BOM before stmt 2 | `ParseError` `Unexpected character '\uFEFF' at position 6.`, `position 6 line 2 col 1` (the BOM's own offset) |
| UTF-8 + BOM, `print("a⟨BOM⟩b")` | `ok:true`, `output:["a\uFEFFb"]` |
| UTF-8 + BOM only | `ok:true`, no timings |
| empty file | `ok:true`, no timings |

A first fixture accidentally carried a **double** BOM (`EF BB BF EF BB BF`); it was refused as
`Unexpected character '\uFEFF' at position 0.` with `position 1 line 1 column 2` on `--file` **and** on
`--stdin` — kept because a broken probe that agrees with itself is still a control (EVD-330).

---

## 4. Cancel ledger battery (fd658d0 + d3a1f50) — full 24-run table

`long.ls` = `sum(1..20000000)`; each row one process; `diags` counts `cancellation deadline exceeded`
entries; `inv1` = `exceeded ⇔ excessMs > 0`.

```
b=30  exit=1 elapsedMs=28.106  stopped=True exceeded=False excessMs=0      diags=0 inv1=True
b=30  exit=1 elapsedMs=41.234  stopped=True exceeded=True  excessMs=11.635 diags=1 inv1=True
b=30  exit=1 elapsedMs=48.392  stopped=True exceeded=True  excessMs=18.707 diags=1 inv1=True
b=30  exit=1 elapsedMs=45.334  stopped=True exceeded=True  excessMs=15.642 diags=1 inv1=True
b=30  exit=1 elapsedMs=36.75   stopped=True exceeded=True  excessMs=7.057  diags=1 inv1=True
b=30  exit=1 elapsedMs=34.192  stopped=True exceeded=True  excessMs=4.559  diags=1 inv1=True
b=30  exit=1 elapsedMs=45.104  stopped=True exceeded=True  excessMs=15.404 diags=1 inv1=True
b=30  exit=1 elapsedMs=33.943  stopped=True exceeded=True  excessMs=4.311  diags=1 inv1=True
b=100 exit=1 elapsedMs=109.482 stopped=True exceeded=True  excessMs=9.85   diags=1 inv1=True
b=100 exit=1 elapsedMs=106.525 stopped=True exceeded=True  excessMs=6.839  diags=1 inv1=True
b=100 exit=1 elapsedMs=144.559 stopped=True exceeded=True  excessMs=44.87  diags=1 inv1=True
b=100 exit=1 elapsedMs=98.633  stopped=True exceeded=False excessMs=0      diags=0 inv1=True
b=100 exit=1 elapsedMs=141.475 stopped=True exceeded=True  excessMs=41.807 diags=1 inv1=True
b=100 exit=1 elapsedMs=100.021 stopped=True exceeded=True  excessMs=0.331  diags=1 inv1=True
b=100 exit=1 elapsedMs=104.786 stopped=True exceeded=True  excessMs=5.097  diags=1 inv1=True
b=100 exit=1 elapsedMs=104.716 stopped=True exceeded=True  excessMs=5.039  diags=1 inv1=True
b=300 exit=1 elapsedMs=292.462 stopped=True exceeded=False excessMs=0      diags=0 inv1=True
b=300 exit=1 elapsedMs=295.266 stopped=True exceeded=False excessMs=0      diags=0 inv1=True
b=300 exit=1 elapsedMs=321.112 stopped=True exceeded=True  excessMs=21.485 diags=1 inv1=True
b=300 exit=1 elapsedMs=306.579 stopped=True exceeded=True  excessMs=6.95   diags=1 inv1=True
b=300 exit=1 elapsedMs=298.253 stopped=True exceeded=False excessMs=0      diags=0 inv1=True
b=300 exit=1 elapsedMs=293.887 stopped=True exceeded=False excessMs=0      diags=0 inv1=True
b=300 exit=1 elapsedMs=310.606 stopped=True exceeded=True  excessMs=10.987 diags=1 inv1=True
b=300 exit=1 elapsedMs=288.715 stopped=True exceeded=False excessMs=0      diags=0 inv1=True
```
Reading: the fix's guarantee holds 24/24, and the *opposite* contradiction the fix could have created
(`exceeded:true` while `elapsedMs < budgetMs` — a diagnostic that would say "elapsed 98 ms (2 ms
over budget)" for a 100 ms budget) did **not** appear in 24 runs; the window for it is ~microseconds wide
(reasoned from Runner.cs:197-200 + 461-483, **not** reproduced → could-not-check 4). Note that the published
`elapsedMs` and `excessMs` come from **different clocks by design**, so `elapsedMs − budgetMs ≠ excessMs` is
normal (e.g. 41.234 − 30 = 11.234 vs published 11.635).

---

## 5. Solve-under-assumptions probes (f35e5a7) — evidence behind row 12

Scripts run through `--file`, `--json --omit-functions --omit-variables`; `x` declared first in each.

| script | observed |
|---|---|
| `assume(x > 5); solve(x == 3, x)` | refuted — no `3` published (the audit's M-3 repro no longer reproduces) |
| `assume(x >= 3); solve(x == 3, x)` | `3` published — the **boundary admits** the root (assumption at the constraint's edge) |
| `assume(x > 3); solve(x == 3, x)` | refuted |
| `assume(x != 3); solve(x == 3, x)` | refuted |
| `assume(x < 0); solve(x^2 == 4, x)` | `-2` only (the positive root is dropped) |
| `assume(x > 5)` then `assume(x < 0)` | the second `assume` is refused (typed) — the empty store is not silently accepted |

Both the display and the structured projection of the surviving roots matched the assumption-free session
for the admitted cases (no digit or shape change from the store).

---

## 6. Could not check / untested (an untested area is NOT a clean area)

1. **34ee4b3 (K-3, plugin as a factory) — no CLI route exists.** The binary builds exactly one
   `SymbolicsPlugin` and one `SuiteEngine` (Runner.cs:180-182) and has no plugin-loading builtin, so
   "one instance in two engines" is unobservable from `--eval/--file/--stdin`. Verdict for that row:
   **unverified**, resting on the commit's own tests, which I did not run (hard rule 1).
2. **523240b's host half (SuiteEngine.ProjectValue driven by a library caller).** The binary only proves the
   CLI side (1100 digits at `setprecision(1100)`). The host/CLI *disagreement* the fix closes needs a
   second process outside the CLI — persona P's scratch-project territory, not mine.
3. **34ee4b3's shared Context** (symbol/function tables, hash-consing pool): no observable surface in the
   binary; source read only.
4. **The cancel clock's residual window** — `exceeded:true` with `elapsedMs < budgetMs` — reasoned
   from the code as reachable only if the OS timer fires late by less than the (microsecond) gap between the
   deadline clock and the engine's stopwatch. 24 runs never hit it; I could not construct it.
5. **CRLF counterpart of O-3** (message-embedded offset vs the mapped offset for a Windows line break) —
   the mechanism is the same (the engine's text is one character shorter per CRLF), but I measured only the
   BOM instance, twice; the CRLF instance is a prediction from ScriptPositions.cs:74-86, not a probe.
6. **The huge-range ceiling**: `len(1..123456789012345678901234567890)` produced **0 bytes of stdout and
   was still running after 90 s** (killed; earlier 180 s runs of the same command also returned nothing, and
   `len(1..100000000)` succeeds in 41 s). Consistent with the already-recorded **M-1** eager range
   materialisation (EVD-332), so **not** reported as a new finding — but whether it would eventually answer,
   OOM or be killed is unestablished.
7. **The `--print-budget` token-boundary half of 7df2a68 (I-3) is untested by me.** I ran the
   evalf-clamp half only; the printer half (54-char identifier at `--print-budget 1`, 40-char name through
   the proportional cut) is left for whoever owns the printer, and its row above is a **partial** verdict.
8. **The JIT twin was not used at all** — every execution row is the AOT binary, so AOT/JIT agreement is not
   attested by this audit.
9. **No test suite was run** (hard rule 1): every row is a wire-level observation of the binary, not a suite
   result. Similarly, the determinism check I attempted compared **whole envelopes** and reported
   "identical=False" purely because `elapsed`/`elapsedTime`/`timings[].elapsed` carry wall-clock
   measurements; the value-bearing fields (`result.display`) were identical across the two runs
   (`x + O(x^3)`; `x^-2 + 1/6 + 7/360*x^2 + O(x^3)`). That probe was too strict — reported as a
   probe defect, not as a determinism finding.

---

### Strategy note (why the negative rows still count)

The attack was deliberately *lateral*: for each fix I generated the family one coefficient / one order / one
encoding / one sibling away from its pinned case rather than stressing the pinned case. That produced
**O-1 and O-2 as the only new refusals inside the fixed surfaces** (both: a raw CLR message surviving one
argument away from a message K-1 / 7df2a68 just made honest), plus **O-3** (a position that is right in the
field and wrong in the sentence). It produced **no wrong value** anywhere in the series, BOM, depth,
projection or cancel families — every fix's own boundary held.
