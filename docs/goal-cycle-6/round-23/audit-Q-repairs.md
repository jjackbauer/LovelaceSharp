# Audit Q — the round-23 repairs attacked where their own tests do not look

**Persona Q, round 23 (goal-cycle 6), wave 6.**
**Artefact under test (only SUT):** `C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe`
— Native AOT, 5,767,680 bytes, written 2026-09-12 21:51:38, published from the tree carrying
(1) the N-1 repair in `Runner.SplitLines`, (2) the argument-refusal grammar in
`Interpreter.ToLong`/`ParseShape`/`setprecision` and `SymbolicsPlugin.AsLong`, (3) nothing else
product-side (P-2 reverted; false-precision digits from `Real.Sin` are the known-open P-2).
**Target of this wave:** the repairs themselves, not the surface they were written for.
**Method:** every script is a real file driven through `--file` (PS 5.1 strips embedded double quotes
from native argv — EVD-299) or through `--stdin`; every probe was parse-validated before its numbers
were believed (EVD-330). Probe scripts live in `%TEMP%\qr23` (nothing in the repo was modified except
this report). `--omit-functions --omit-variables` is used only to keep the envelope readable; it does
not change `output`, `timings` or the failure code (verified on the `--text`/full-envelope spot checks).

**Headline.** The N-1 repair itself survived every boundary the brief names (blank-only, blank-twice,
whitespace-ended, inside `if`/inside `for`, print-then-throw, interleaved, CRLF/LF/no-newline/BOM,
`--stdin` pipe vs redirect, the `--text` and failing paths): **no probe found a line lost by the
repair**. But the *invariant the repair was justified by* — `count(timings[].hasOutput == true)` equals
the number of lines in `output[]` — still fails, and it fails on **Q-1** below for a reason the repair's
own tests never touch. Two refusal-grammar gaps next door are also live (**Q-2**, **Q-3**).

---

## 1. Probe table — A. the N-1 repair

All commands are `Lovelace.Run.exe --file <script> --omit-functions --omit-variables`; "out" is the
verbatim `output[]` array, "flags" is the number of `timings[]` entries with `hasOutput: true`.

### 1.1 Boundaries the repair creates

| # | script (LF file, one statement per line) | observed | verdict |
|---|---|---|---|
| c01 | `print("a")` | outCount=1 flags=1 out=[a] | ok |
| c02 | `print("a")` / `print("b")` | outCount=2 flags=2 out=[a\|b] | ok |
| c03 | `print("a")` / `print()` | outCount=2 flags=2 out=[a\|\\0] | **fix holds — the N-1 line is back** |
| c04 | `print()` / `print()` | outCount=2 flags=2 out=[\\0\|\\0] | ok |
| c05 | `print()` / `print("a")` | outCount=2 flags=2 out=[\\0\|a] | ok |
| c06 | `print("a")` / `print()` / `print("b")` | outCount=3 flags=3 out=[a\|\\0\|b] | ok |
| c21 | `print("a")`/`print("")`/`print("b")`/`print()` | outCount=4 flags=4 out=[a\|\\0\|b\|\\0] | ok |
| p01 | `print()` as the ONLY statement | raw envelope: `"output":[""],` `"timings":[{"position":0,...,"hasOutput":true}]` | ok (no phantom element) |
| c13 | `print("a   ")` (argument ends in whitespace) | outCount=1 flags=1 out=[`a   `] — the trailing blanks survive | ok |
| c09 | `if (1 == 1) { print("q") }` | outCount=1 flags=1 out=[q] | ok |
| c16 | `if (1 == 1) { print("q") }` / `print("r")` | outCount=2 flags=2 out=[q\|r] | ok |
| c20 | `if (1 == 2) { print("q") }` | outCount=0 flags=0 (a timing entry exists, false) | ok |
| c10 | `for i in 1..2 { print(i) }` | outCount=2 flags=1 out=[1\|2] | **MISMATCH — Q-1** |
| c17 | `for i in 1..3 { print("z") }` | outCount=3 flags=1 out=[z\|z\|z] | **MISMATCH — Q-1** |
| m04 | `for i in 1..2 { print() }` | outCount=2 flags=1 out=[\\0\|\\0] | **MISMATCH — Q-1** |
| c11 | `print("a")` / `det(1)` | code=InvalidArgument, outCount=1 flags=1 out=[a] | ok (failing path keeps the line) |
| c12 | `print("a")`/`print("b")`/`det(1)` | code=InvalidArgument, outCount=2 flags=2 out=[a\|b] | ok |
| c19 | `for i in 1..2 { print(i) }` / `det(1)` | code=InvalidArgument, outCount=2 flags=1 out=[1\|2] | **MISMATCH — Q-1 (failing path too)** |
| p08 | `print("a")`/`print("b")`/`det(1)` (prints interleaved with an error in a block) | raw: `"output":["a","b"]`, flags T,T,F | ok |
| k01 | `print("a")`/`print()`/`print("b")`/`evalf(pi(3000), 3000)` | code=InvalidArgument (pi caps at 1000), `"output":["a","","b"]`, flags T,T,T,F | ok — the failing path publishes the blank line |
| p09 | `print("a")` / blank / `x = (` (PARSE error with a trailing blank line) | code=ParseError, `"timings":[],"output":[]` | ok — nothing ran, nothing claimed |

Full-envelope evidence for the raw rows (kept out of the table for width):
`print("a"); print()` → `{"ok":true,...,"output":["a",""],...,"timings":[{"position":0,...,"hasOutput":true},{"position":11,...,"hasOutput":true}]}`;
`print("a"); det(1)` → `{"ok":false,"code":"InvalidArgument","output":["a"],"timings":[{...,"hasOutput":true},{...,"hasOutput":false}]}`
(no `partialOutput` key at all on a non-cancelled failure — the array is `output`).

### 1.2 Terminator shapes, source encodings and `--stdin`

| # | shape | observed | verdict |
|---|---|---|---|
| e02 | CRLF file, `print("a")`/CRLF/`print()`/CRLF | outCount=2 flags=2 out=[a\|\\0] | ok |
| e03 | LF file with NO trailing newline (`print("a")`) | outCount=1 flags=1 out=[a] | ok |
| e04 | BOM + LF file | outCount=2 flags=2 out=[a\|\\0] | ok (the O-3 message-offset issue is separate and already recorded) |
| e05 | BOM + CRLF file | outCount=2 flags=2 out=[a\|\\0] | ok |
| e06 | LF file with three trailing blank lines after `print("a")` | outCount=1 flags=1 out=[a] | ok |
| s-src | c03 through the PS 5.1 pipe: `Get-Content c03.ls \| exe --stdin --omit-functions` | `"output":["a",""]` | ok — identical to `--file` |
| s-cmd | c03 raw bytes: `cmd /c "exe --stdin ... < c03.ls"` | `"output":["a",""]` | ok |
| s-crlf | e02 raw bytes through the same cmd redirect | `"output":["a",""]` | ok — CRLF source does not change the captured-line split |

### 1.3 A printed string that contains a real newline (the capture, not the source)

| # | script | observed | verdict |
|---|---|---|---|
| m01 | `s = "a`/LF/`b"` then `print(s)` | outCount=2 flags=1 out=[a\|b] | **MISMATCH — Q-1** |
| s08 | same, printed twice | outCount=4 flags=2 out=[A\|B\|A\|B] | **MISMATCH — Q-1** |
| m02 | `s = "a`/LF/`"` then `print(s)` / `print("z")` | outCount=3 flags=2 out=[a\|\\0\|z] | **MISMATCH — Q-1** (the repair's one-terminator rule leaves the printed line's own newline as a real empty line) |
| m03 | `s = "`/LF/`"` then `print(s)` | outCount=2 flags=1 out=[\\0\|\\0] | **MISMATCH — Q-1** |

### 1.4 The invariant directly, over the generated corpus

Invariant under test: *when every printing statement prints exactly one line,
`count(timings[].hasOutput == true)` == `len(output[])`.*
Corpus: c01–c21 above (21 scripts: blank-only, blank-pairs, interior blanks, whitespace-ended,
`if`/`for`, success/failing). **Result: 17 agree, 4 disagree (c10, c17, c18, c19)** — every
disagreement is a printing statement executed more than once, i.e. a loop. (`c18` = `print("a")` then
`for i in 1..2 { print(i) }` → outCount=3 flags=2, out=[a\|1\|2].)

### 1.5 `--text` (the `= result` line / variable lines / ledger line as consumers)

| # | command | observed (CR shown as `<CR>`) | verdict |
|---|---|---|---|
| t1 | `--file c03.ls --text --omit-functions` | `a<CR>` `<CR>` (the blank line is emitted) | ok |
| t2 | `--file c06.ls --text` | `a<CR>` `<CR>` `b<CR>` | ok |
| t3 | `--file c08.ls --text` (`x = 1` / `print("a")`) | `a<CR>` `  x = 1<CR>` | ok (a blank printed line is emitted *before* the variable lines, so the two are still distinguishable only by the two-space prefix) |
| k01 | cancelled-branch ledger line | not produced: the run's `cancellation.exceeded` is `false` because the failure was a refusal, not a deadline | could-not-check (see §5) |

---

## 2. Probe table — B. the refusal repairs and the same class next door

Every row: script through `--file`, `--omit-functions --omit-variables --cancel-after 3000` (the
bound is a probe guard; no row below needed it except where `Cancelled` is reported).

| # | argument | observed code | observed message / value | verdict |
|---|---|---|---|---|
| b01 | `zeros(0)` | ok | `output: []` | **control held** |
| b02 | `[1,2,3][-1]` | InvalidOperation | `Index -1 is out of range for vector of length 3.` | **control held (numbers kept)** |
| b03 | `[1,2,3][3]` | InvalidOperation | `Index 3 is out of range for vector of length 3.` | **control held (numbers kept)** |
| b04 | `setprecision(-5)` | InvalidOperation | `setprecision() expects a positive digit count, but got -5.` | **control held** |
| b05 | `setprecision(0)` | InvalidOperation | `setprecision() expects a positive digit count, but got 0.` | **control held** |
| b06 | `pi(1000000000)` | InvalidArgument | `pi(): argument 1 (digits) must be a digit count between 1 and 1000 (the engine's computation precision); got 1000000000.` | **control held (K-1 grammar)** |
| b34 | `e(100000000000)` | InvalidArgument | same grammar, `got 100000000000` | control held |
| b07 | `zeros(100000000000)` | AllocationRefused | `zeros() requested one array of shape [100000000000], which holds 100000000000 element(s) — more than the maximum single-allocation budget of 268435456 element(s).` | **control held (typed budget)** |
| b28 | `ones(100000000000)` | AllocationRefused | same shape of message | control held |
| b08 | `eye(100000000000)` | AllocationRefused | `eye() requested one array of shape [100000000000, 100000000000], which holds more than 9223372036854775807 element(s) — more than the maximum single-allocation budget of ...` | ok (typed; the product is saturated in the message) |
| b12 | `zeros(2,-1)` | InvalidArgument | `zeros(): argument 2 (a dimension) must be a non-negative count; got -1.` | ok |
| b31 | `zeros(-1)` | InvalidArgument | `zeros(): argument 1 (a dimension) must be a non-negative count; got -1.` | ok |
| b13 | `ones(-1)` | InvalidArgument | `ones(): argument 1 (a dimension) must be a non-negative count; got -1.` | ok |
| b14 | `reshape([1,2,3,4], -1)` | InvalidArgument | `reshape(): argument 1 (a dimension) must be a non-negative count; got -1.` | ok |
| b09/b10/b11/b29 | `eye(-1)` / `eye(0)` / `eye(2,-1)` / `eye(3,-2)` | InvalidArgument | all four: `eye() dimensions must be positive.` (no value, no argument index) | **Q-4** |
| b15 | `reshape([1,2,3,4], 100000000000)` | InvalidArgument | `reshape() to [100000000000] requires 100000000000 element(s), but this array has 4. (Parameter 'shape')` | **Q-4 (raw CLR text)** |
| b32 | `reshape([1,2,3,4], 0)` | InvalidArgument | `reshape() to [0] requires 0 element(s), but this array has 4. (Parameter 'shape')` | **Q-4** |
| b25 | `fft([1,2,3])` | InvalidArgument | `FFT length must be a power of two, but got 3. (Parameter 'x')` | **Q-4 (raw CLR text)** |
| b16/b17/b18/b19/b30 | `reshape([1,2,3,4], 2.5)` / `eye(2.5)` / `zeros(2.5)` / `[1,2,3][1.5]` / `[1,2,3][2.0]` | InvalidOperation | all five: `Index must be Natural or Integer, but got 'Real'.` | **Q-4 (names an "index" for a dimension, names no function)** |
| b21 | `setprecision(1.5)` | InvalidOperation | `setprecision() expects a Natural or Integer digit count, but got 'Real'.` | ok |
| b20 | `setprecision(100000000000)` | **ok** | accepted; no message | **Q-3** |
| w01 | `setprecision(100000000000)` / `print(sqrt(2))` | **Cancelled** at the 3000 ms bound (5713 ms wall) | `the evaluation was cancelled by the caller.`, `output: []` | **Q-3** |
| w04 | `setprecision(1000000000)` / `print(sqrt(2))` | **Cancelled** (6053 ms) | same | **Q-3 (2nd reproduction)** |
| w05/w06 | `10^7` / `10^6` the same way | Cancelled | same | **Q-3** |
| w07 | `setprecision(100000)` / `print(sqrt(2))` | ok, 452 ms | `1.41421356237309504880168872420969807856…` | boundary control |
| v04 | `setprecision(2000)` / `print(sqrt(2))` | ok, 35 ms | output string length **2002** vs `v01` length **102** | setprecision is not a no-op: the accepted magnitude is honoured |
| w02/w03 | `setprecision(100000000000)` / `print(1)` or `print(sqrt(4))` | ok, 32/33 ms | `1` / `2` (exact values need no expansion) | severity evidence |
| b33 | `evalf(pi(30), 100000000000)` | InvalidArgument | `evalf(): argument 2 (digits) must be a Natural or Integer digit count between 1 and 2147483647; got 100000000000.` | ok (but note the cap is `int.MaxValue`, not 1000) |
| u05/u06 | `setprecision(9223372036854775808)` / `setprecision(99999999999999999999999)` | InvalidArgument | `setprecision(): argument 1 (digits) must be a digit count no wider than 9223372036854775807; got …` | ok — no raw CLR text |
| u07–u10/u12 | `eye(…)`, `zeros(…)`, `[1,2,3][…]`, `reshape(…, …)`, `zeros(2, …)` with a 23-digit count | InvalidArgument | `An index or dimension must fit a signed 64-bit integer (at most 9223372036854775807 in absolute value); got …` | ok — no raw CLR text |
| u11 | `pi(99999999999999999999999)` | InvalidArgument | K-1 grammar, `got 99999999999999999999999` | ok |
| u01/u02 | `setprecision(100000000000)` / `print(2^0.5)` | UnsupportedOperation | `Non-integer exponents are not yet supported.` | ok |
| b26 | `linspace(0,1,100000000000)` | InvalidOperation | `Unknown function 'linspace'.` | no such builtin |
| b27 | `factorial(100000000000)` | InvalidOperation | `Unknown function 'factorial'.` | no such builtin |
| x03 | `series(sin(x), x, 0, 0)` | InvalidArgument | `a series expansion order must be at least 1 (the number of terms to carry, with the O-term…` (tail not captured for 0); `s06` order -1 carries the full tail: `…with the O-term); got -1.` | control held (order < 1 refuses) |
| x02 | `series(sin(x), x, 0, 1)` | ok | `0` | series semantics, not this audit |
| x01/y02 | `series(sin(x), x, 0, 2.5)` | **ok** | `x + O(x^2)` | **Q-2 (floored, silently)** |
| y04 | `series(sin(x), x, 0, 3.5)` | **ok** | `x + O(x^3)` | **Q-2** |
| y05 | `series(sin(x), x, 0, 2.9999)` | **ok** | `x + O(x^2)` | **Q-2** |
| s04 | `series(sin(x), x, 0, 100000000000)` | **Cancelled** | `the evaluation was cancelled by the caller.` | **Q-2 (no upper bound)** |
| x06 | `series(sin(x), x, 0, 5000)` | **Cancelled** at 3034 ms | same | Q-2 (cost note) |
| x05 | `series(sin(x), x, 0, 500)` | ok, 579 ms | `x - 1/6*x^3 - 1/5040*x^7 - …` | cost reference |

`\\0` in the tables marks an empty array element.

---

## 3. Findings

### Q-1 — `timings[].hasOutput` is a per-statement-site boolean while `output[]` is per-line, so the envelope's line ledger disagrees with its own array (P1)

**What.** The N-1 repair was justified by the claim (Runner.cs:598-601) that the old split "made the
envelope contradict its own machine-readable flag". That contradiction is only fixed for the
trailing-blank case. `hasOutput` is set from whether *the statement site* wrote anything
(`Runner.cs` `Timings`: `t.Output.Length > 0`), and there is exactly one timing entry per *site*
(one per loop, not one per iteration), while `output[]` grows one element per printed line and per
embedded newline. A consumer that reconstructs "which statement produced these lines" from the pair
therefore under-counts whenever a printing site executes more than once, and there is no way to
recover the per-iteration split from the envelope.

**Severity P1** — a false claim in the machine API: the documented pairing of `timings[]` with
`output[]` is not injective, and an agent loop that uses the flags to attribute lines loses them.
It is not a *data* loss (every line is in `output[]`), which is why it is not P0.

**Reproduction 1 (loop).**
```
script c10.ls:            for i in 1..2 { print(i) }
command:  out\aot\Lovelace.Run.exe --file c10.ls --omit-functions --omit-variables
observed: {"ok":true, ... ,"output":[1,2], ... ,"timings":[{"position":0,...,"hasOutput":true}]}
```
2 lines, 1 true flag.

**Reproduction 2 (loop, failing path, 3 lines vs 1 flag).**
```
script c19.ls:            for i in 1..2 { print(i) }
                          det(1)
command:  out\aot\Lovelace.Run.exe --file c19.ls --omit-functions --omit-variables
observed: {"ok":false,"code":"InvalidArgument", ... ,"output":[1,2],
           "timings":[{"position":0,...,"hasOutput":true},{"position":24,...,"hasOutput":false}]}
```
Second success-path witness: `c17.ls` = `for i in 1..3 { print("z") }` → `"output":["z","z","z"]`,
one true flag. Blank-printing witness: `m04.ls` = `for i in 1..2 { print() }` → `["",""]`, one true flag.

**Reproduction 3 (a single print, a string with a real newline).**
```
script s08.ls:  s = "A
B"
                print(s)
                print(s)
command:  out\aot\Lovelace.Run.exe --file s08.ls --omit-functions --omit-variables
observed: "output":["A","B","A","B"]   true flags = 2
```
One `print` call, two lines; the flag cannot say so. `m01.ls` is the one-print version
(`["a","b"]`, 1 true flag).

### Q-2 — `series()` accepts an order that is not an integer and silently floors it, and accepts an order with no upper bound (P1)

**What.** The refusal grammar that `zeros(2.5)` and `[1,2,3][1.5]` answer with
`Index must be Natural or Integer, but got 'Real'.` does not apply to `series`'s `order`
(`SymbolicsPlugin.AsLong` is in the landing set). `2.5` and `2.9999` are accepted and behave as
`2`; `3.5` behaves as `3`. The same argument accepts `100000000000`, which cannot finish, so the
refusal that protects `pi`/`e` (`between 1 and 1000`) has no counterpart here.

**Severity P1** — a caller's argument is silently altered and a value the API accepted makes a valid
script unservable (the only guard left is the caller's `--cancel-after`; without it the run is
unbounded, as the recorded `evalf`/solve cost cliffs are).

**Reproduction 1 (silent flooring).**
```
scripts:  y01.ls: x = symbol("x") ; print(series(sin(x), x, 0, 2))
          y02.ls: x = symbol("x") ; print(series(sin(x), x, 0, 2.5))
          y05.ls: x = symbol("x") ; print(series(sin(x), x, 0, 2.9999))
command:  out\aot\Lovelace.Run.exe --file <script> --omit-functions --omit-variables
observed: y01 -> {"ok":true,...,"output":["x + O(x^2)"]}
          y02 -> {"ok":true,...,"output":["x + O(x^2)"]}      <- 2.5 IS 2
          y05 -> {"ok":true,...,"output":["x + O(x^2)"]}      <- 2.9999 IS 2
          (y03 order 3 and y04 order 3.5 both -> "x + O(x^3)")
```

**Reproduction 2 (accepted magnitude that cannot finish).**
```
script s04.ls:  x = symbol("x")
                series(sin(x), x, 0, 100000000000)
command:  out\aot\Lovelace.Run.exe --file s04.ls --omit-functions --omit-variables --cancel-after 3000
observed: {"ok":false,"code":"Cancelled",...,"message":"the evaluation was cancelled by the caller.", "output":[]}
control:  x06.ls order 5000 -> Cancelled at 3034 ms; x05.ls order 500 -> ok in 579 ms.
```

### Q-3 — `setprecision()` has no upper bound while every other digit count in the language is capped, so an accepted argument puts the next print on an unbounded path (P1)

**What.** The landing added only the positivity/non-integer refusals:
`setprecision(-5)` → `setprecision() expects a positive digit count, but got -5.`. The magnitude
grammar is missing: `setprecision(100000000000)` is **ok**, where `pi(100000000000)` and
`e(100000000000)` answer `must be a digit count between 1 and 1000 (the engine's computation
precision)` and `evalf(..., 100000000000)` answers `between 1 and 2147483647`. `setprecision` is
not a no-op — the accepted value is honoured (`setprecision(2000)` makes `print(sqrt(2))` return
2002 characters instead of 102) — so the accepted value is real work, and at `10^6` and above the next
surd print never returns inside any sane budget.

**Severity P1** — an argument the API accepts cannot be served; the envelope's own verdict for the
resulting run is `Cancelled`, i.e. the caller's budget becomes the error message for a bad *argument*.
Not P0: no wrong value is printed and no process aborts.

**Reproduction 1.**
```
script w01.ls:  setprecision(100000000000)
                print(sqrt(2))
command:  out\aot\Lovelace.Run.exe --file w01.ls --omit-functions --omit-variables --cancel-after 3000
observed: {"ok":false,"code":"Cancelled",...,"message":"the evaluation was cancelled by the caller.","output":[]}
          wall clock 5713 ms (the bound is checked late — the deadline was observed late)
```

**Reproduction 2.**
```
script w04.ls:  setprecision(1000000000)
                print(sqrt(2))
command:  same, --cancel-after 3000
observed: {"ok":false,"code":"Cancelled",...,"output":[]}   wall clock 6053 ms
```
Boundary controls, same command shape: `w07` `setprecision(100000)` → `{"ok":true,...,"output":["1.414…"]}`
in 452 ms; `v04` `setprecision(2000)` → ok, first element 2002 characters (`v01` without it: 102);
`w02` `setprecision(100000000000)` + `print(1)` → `{"ok":true,...,"output":["1"]}` in 32 ms, so the
cost is the digit expansion, not the setting itself.

### Q-4 — refused arguments still cross with raw CLR text or with a message that names the wrong thing (P2)

**What.** Three shapes, all after the refusal repair, all reachable from a script:
1. `reshape` carries the CLR `ArgumentException` parameter suffix: `(Parameter 'shape')`;
   `fft` carries `(Parameter 'x')`. These are .NET parameter names, not language names.
2. A non-integer dimension/index answers `Index must be Natural or Integer, but got 'Real'.` — it
   names an *index* when the caller passed a dimension, and names no function, so
   `zeros(2.5)`, `eye(2.5)`, `reshape([1,2,3,4], 2.5)` and `[1,2,3][1.5]` are indistinguishable.
3. `eye`'s positivity refusal carries neither the value nor the argument index:
   `eye() dimensions must be positive.` for `eye(-1)`, `eye(0)`, `eye(2,-1)` and `eye(3,-2)`,
   while the sibling `zeros()` message does carry both (`zeros(): argument 2 (a dimension) must be a
   non-negative count; got -1.`). The record's control ("messages WITH the numbers") holds for the
   index controls but not for `eye`.

**Severity P2** — cosmetic/documentation: the refusal is correct and typed on every row; only the
message is wrong-shaped, and a caller diagnosing `eye(-1)` vs `eye(0)` cannot tell them apart.

**Reproduction 1.**
```
script b15.ls:  reshape([1,2,3,4], 100000000000)
command:  out\aot\Lovelace.Run.exe --file b15.ls --omit-functions --omit-variables
observed: {"ok":false,"code":"InvalidArgument","message":
           "reshape() to [100000000000] requires 100000000000 element(s), but this array has 4. (Parameter 'shape')"}
and b25.ls fft([1,2,3]) -> "...FFT length must be a power of two, but got 3. (Parameter 'x')"
```

**Reproduction 2.**
```
scripts: b09.ls eye(-1)   b10.ls eye(0)   b11.ls eye(2,-1)   b29.ls eye(3,-2)
command:  out\aot\Lovelace.Run.exe --file <script> --omit-functions --omit-variables
observed: all four: {"ok":false,"code":"InvalidArgument","message":"eye() dimensions must be positive."}
```

---

## 4. Controls that must NOT have moved — status

| control | required | observed | status |
|---|---|---|---|
| `zeros(0)` | `[]` | `{"ok":true,...,"output":[]}` | held |
| `[1,2,3][-1]` | message with the numbers | `Index -1 is out of range for vector of length 3.` | held |
| `[1,2,3][3]` | message with the numbers | `Index 3 is out of range for vector of length 3.` | held |
| `setprecision(-5)` | keeps its message | `setprecision() expects a positive digit count, but got -5.` | held |
| `setprecision(0)` | keeps its message | `setprecision() expects a positive digit count, but got 0.` | held |
| `pi(huge)` | K-1 grammar | `pi(): argument 1 (digits) must be a digit count between 1 and 1000 (the engine's computation precision); got 1000000000.` | held |
| `zeros` unservable size | typed allocation budget | `AllocationRefused` … `maximum single-allocation budget of 268435456 element(s)` | held |

---

## 5. Could not check (an untested area is not a clean area)

1. **The cancelled branch's own ledger line and `partialOutput` with the repair.** `--cancel-after`
   on a script that prints and then runs long enough was not reached with a *committed* print set: the
   only cancellation witnesses in this session were `Q-2`/`Q-3` runs that cancelled **before** any
   print executed (`output: []`), so `partialOutput[]` vs `output[]` with a trailing blank line was
   never observed. The brief's "cancellation ledger line" in `--text` is therefore unverified here.
   (The M-1 record — lines of the *interrupted* statement are lost — is already filed and was not
   re-tested.)
2. **A real `>` file redirect on `--stdin` from a genuine anonymous pipe** (as opposed to the cmd
   `<` redirect and the PS 5.1 text pipe used here). Both tested paths agreed with `--file`.
3. **`CR`-only (classic Mac) source files and mixed CR/LF inside one file** — not probed; the parser's
   handling of a lone CR as an engine line terminator is untested by me.
4. **`--stdin` with a BOM and with invalid UTF-8 bytes** — only `--file` BOM was probed (e04/e05).
5. **The second host's copy of the split** (`Lovelace.Studio/EngineHost.cs:319-326`,
   `IncrementalRunner.cs:418-425`) — read only in the implementation note; this audit touched the CLI
   artefact only, per the "only system under test" rule.
6. **`series`'s other argument positions** (the point `x0` and the symbol) with non-integer/oversized
   values — only `order` was probed.
7. **`linspace` and `factorial`** are not builtins in this artefact (`Unknown function`), so the
   brief's "huge counts to linspace/factorial" rows have no product to measure.
8. **`eye` with a ragged/huge column count on a `--print-budget`-limited envelope** and the
   `AllocationRefused` message's saturated product — the message was read but its arithmetic was not
   independently recomputed for shapes whose product exceeds `long.MaxValue`.
9. **The JIT twin** (`Lovelace.Run\bin\Release\net10.0\Lovelace.Run.exe`) — every probe above ran
   against the AOT artefact only; the brief's "same code, different artefact" claim was not re-verified
   in this session.

---

## 6. Scope note

No product code, test, or artefact was rebuilt, re-published, moved or deleted; git state was read-only
and untouched. The only file written inside the repository is this report. The probe scripts and the raw
transcripts they were reduced from live in `%TEMP%\qr23` (`batchA3.ps1`, `batchA4.ps1`,
`batchB.ps1`, `batchC2.ps1`, `batchU.ps1`, `batchV.ps1`, `batchW.ps1`, `batchX.ps1`,
`batchY.ps1` plus the `.ls` files they generate) and are reproducible from the commands quoted above.
