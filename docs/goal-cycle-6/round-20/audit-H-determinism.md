# Cycle-6 · round-20 · audit H — determinism and idempotence

**Claim under test:** the published binary's answer depends only on the script, never on how or how often it is run.

**Verdict: FALSIFIED.** Two findings: one **P0** — the same script returns a *different value* on every run of the
same process image (12/12 distinct in 12 concurrent runs) — and one **P2** where a flag does not do what its own
help text says. Everything else I attacked held.

Severity ladder used below: **P0** = a wrong value or an abort/crash on valid input; **P1** = a false claim in the
machine API, a wrong refusal, or data the API says it carries being lost or altered; **P2** =
cosmetic/documentation.

---

## Method, and the rule used to declare a difference

* **Object under test:** `C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe`,
  **5 804 544 bytes**, last written **2026-09-12 18:00:55** — the published Native AOT binary built from the
  current tree. (`Get-Item out\aot\Lovelace.Run.exe`.)
* **Volatile-field rule.** Every comparison strips exactly the three documented volatile fields and nothing else:
  the top-level `elapsed` string, the top-level `elapsedTime` object, and each `timings[].elapsed` object
  (`docs/symbolics/dsh-protocol.md:21-23,152-155`). Any other difference is a finding. The normalizer, used
  verbatim in every probe below:

  ```powershell
  $t = $t -replace '"elapsed":"[^"]*"','"elapsed":"T"'
  $t = $t -replace '"elapsed":\{[^}]*\}','"elapsed":{T}'
  $t = $t -replace '"elapsedTime":\{[^}]*\}','"elapsedTime":{T}'
  ```

* **Harness caveat — not a product finding.** Windows PowerShell 5.1 does not escape embedded double quotes when
  it passes arguments to a native executable: `& $exe --eval 'x = symbol("x"); x'` reaches the binary as
  `x = symbol(x); x` and the binary correctly answers `Undefined variable 'x'.` (this is exactly why audit D's
  report is titled "form A (no embedded quotes)" — `round-13/audit-D-workflow.md:20-22`). Every probe that
  contains a quote therefore used **`--stdin`** (bytes through the pipe) or **`--file`**; the quote-free
  channels agree with one another on every script I compared (`stdin == file == eval` fingerprint equality for
  `1/3 + sin(1/2)`).
* **Coverage of the repeat sweep.** Two sweeps: 22 script shapes × 6 runs, and 99 script shapes × 4 runs (constants,
  truncated constants (`setprecision`), trig, `evalf`, limits, integrals, series, solvers and systems (real,
  complex, quartic), arrays, linear algebra, DSP (`fft/dft/conv/filter/movingavg/noise/step/impulse`), the
  registry (`capabilities/assumptions`), errors (invalid argument, unknown function, parse) and `print`).
  Both are byte-compared under the rule above. **3 of the 99 varied, all one family, reported as H-1.**
  Nothing else varied, ever, in any sweep.

---

## H-1 — **P0** — `series()` of a non-smooth integrand publishes a per-process GUID symbol, so the SAME script returns a DIFFERENT value on every run

**New?** **NEW.** No prior report mentions this; `grep -r "__t" docs` finds only an old cycle-5 patch with a
*fixed* dummy name (`docs/goal-cycle-5/patches/r16-limits.diff:758`), and no cycle-5/c6 document probes
`series(abs(...))`. `git blame` puts the current line in `9dc44ee` ("land the symbolic-numeric kernel…",
2026-09-08) — it is not a fresh regression, but it has never been reported.

### Exact command

```powershell
$exe = 'C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe'
'x = symbol("x"); series(abs(x), x, 0, 3)' | & $exe --stdin --json --omit-functions --omit-variables
```

Run it twice (or in twelve processes at once). The same command also reproduces through `--file`
(`[System.IO.File]::WriteAllText($f, 'x = symbol("x"); series(abs(x), x, 0, 3)')` then
`& $exe --file $f --json --omit-functions --omit-variables`) and through `--text` and through `print(...)`.

### Observed envelope (verbatim, trimmed to the deciding fields; two consecutive runs, exit 0 both)

```
run 1 exit=0 ok=True revision=82 kind=Symbolic nodeCount=29
  display   = 1/2*x^2*piecewise(diff(0, __te8fa6c) if 0 != 0, diff(0, __te8fa6c, __te8fa6c)) + x*piecewise(0 if 0 != 0, diff(0, __te8fa6c)) + O(x^3)
  canonical = (add (mul (rat 1 2) (pow (sym x) (rat 2 1)) (pw ((ne (rat 0 1) (rat 0 1)) (der (__te8fa6c) (rat 0 1))) (der (__te8fa6c __te8fa6c) (rat 0 1)))) (mul (sym x) (pw ((ne (rat 0 1) (rat 0 1)) (rat 0 1)) (der (__te8fa6c) (rat 0 1)))) (order (sym x) (rat 0 1) (rat 3 1)))
  freeSymbols = x
run 2 exit=0 ok=True revision=82 kind=Symbolic nodeCount=29
  display   = 1/2*x^2*piecewise(diff(0, __t9c814e) if 0 != 0, diff(0, __t9c814e, __t9c814e)) + x*piecewise(0 if 0 != 0, diff(0, __t9c814e)) + O(x^3)
  canonical = (add (mul (rat 1 2) (pow (sym x) (rat 2 1)) (pw ((ne (rat 0 1) (rat 0 1)) (der (__t9c814e) (rat 0 1))) (der (__t9c814e __t9c814e) (rat 0 1)))) (mul (sym x) (pw ((ne (rat 0 1) (rat 0 1)) (rat 0 1)) (der (__t9c814e) (rat 0 1)))) (order (sym x) (rat 0 1) (rat 3 1)))
  freeSymbols = x
```

Earlier runs of the same command produced `__t7299ad`, `__t79c6fc`, `__t1db310`, `__t0fe651`,
`__tcd82b9`, `__tde4634`, `__tb25bda`, `__t42b885`, `__ta34f9f`, `__t7333e3`, `__t6f334c` —
every invocation a different value.

### Scale of the reproduction (each of these is a separate process)

| probe | result |
|---|---|
| 4 runs, `series(abs(x), x, 0, 2)` | **4 distinct** normalized envelopes of 4 |
| 4 runs, `series(abs(x), x, 0, 3)` | **4 distinct** of 4 |
| 4 runs, `series(abs(x), x, 0, 4)` | **4 distinct** of 4 |
| **12 processes started concurrently** (`Start-Process … --file`), same file | **12 distinct leaked symbols of 12**: `__tcd9dd2 __t75ee29 __t06bc38 __t17fc3c __t2da686 __tae021e __t5af363 __t5adffc __t555126 __t28568a __t9b5bef __te9f2f9` |
| 3 runs through the **non-AOT Release build** (`dotnet Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll --file …`) | **3 distinct** of 3 (`__t78da2a`, `__t491ac2`, `__t7d465c`) — source-level, not an AOT artifact |
| 4 runs, `series(abs(x), x, 0, 1)` (control) | **1 distinct** of 4 — the order-1 path does not take the leaking branch |

Other inputs that vary the same way (4/4 distinct each): `series(abs(x), x, 1, 3)`,
`series(abs(x), x, -1, 3)`, `series(abs(sin(x)), x, 0, 3)`, `series(abs(x)*x, x, 0, 3)`,
`series(x*abs(x), x, 0, 3)`, `series(abs(x), x, 0, 5)`. Inputs that do **not** vary:
`series(exp(x),…)`, `series(sin(x),…)`, `series(log(x), x, 1, 5)`, `series(sqrt(x), x, 1, 5)`,
`series(tan(x),…)`, `series(asin(x)/atan(x)/acos(x)/x^x/…)`, `series(sqrt(x^2), x, 0, 3)`,
`series(sign(x), x, 0, 2)`, `series(max(x,0)/min(x,0), x, 0, 3)`, `diff(abs(x), x)`,
`integrate(abs(x), x)`, `limit(abs(x), x, 0)`, `limit(x*abs(x), x, 0)`, `evalf(abs(x), 30)`,
`simplify/expand/factor/apart/jacobian/hessian/optimize/latex` on `abs` (all stable, 4/4).

### What is wrong, and how I know

1. **The value is not a function of the script.** `docs/symbolics/architecture.md:1479` (INV-15):
   "*Determinism: same input + same context state + same options + same seed ⇒ byte-identical output everywhere
   (no hash-order iteration, no culture dependence, no reflection).*" Two runs of one script return two different
   `display`s and two different `canonical`s; they cannot both be the answer, so at least one run publishes a
   wrong value. This is the exact claim the cycle is being audited against.
2. **The varying token is an implementation temporary.** `Lovelace.Symbolics/Calculus/Series.cs:56`:
   `var t = ctx.Symbol("__t" + Guid.NewGuid().ToString("N")[..6]);` — the substitution variable of
   `Series.OfNoSplit` (`:53-69`). It is supposed to be eliminated when the coefficients are substituted back at
   `t = 0` (`:64`); for `abs` at the kink it survives into a `piecewise(diff(0, __t…))` node that is
   published verbatim.
3. **It escapes on every surface, not just the JSON display:** `--text` prints
   `= 1/2*x^2*piecewise(diff(0, __t42b885) …`) and `print(series(abs(x), x, 0, 3))` puts the same varying name
   into `output[0]`. The exit code is 0 in every run — the run is not refused, it is *answered* differently.
4. **A second, independent inconsistency in the same envelope:** the `canonical` contains `(sym __te8fa6c)`
   inside `der`, while `freeSymbols` is `["x"]`. The protocol's own value form
   (`docs/symbolics/dsh-protocol.md:31-32`) makes `freeSymbols` part of every Symbolic value. I state this as an
   observation, not as the primary claim: one could argue `der`*binds* its variable, but the *display*
   `diff(0, __te8fa6c)` still publishes a name that exists nowhere in the script and changes between runs.

### Correct behaviour, and how I know

The answer must be a fixed function of the script text. Whatever the kernel decides the series of `|x|` at 0 is,
it must be the same expression on the second run, and it must not name a freshly minted internal symbol — the
protocol presents `display`/`canonical` as renderings of a value the agent can act on
(`docs/symbolics/dsh-protocol.md:3-5, 11-14`). For reference the same request in the stated ground truth,
SymPy 1.14.0, evaluates to `x`:

```powershell
$env:PATH = 'C:\Users\ricar\dev\.lovelace-tools\python;' + $env:PATH
python -c "from sympy import *; x=symbols('x'); print(series(Abs(x), x, 0, 3))"   # -> x
```

I do **not** use that as the criterion: `|x|` has no power series at 0, and a complete answer could legitimately be
a piecewise form or an `O(x^3)` remainder. The criterion is INV-15 and the identity of the value across runs.

---

## H-2 — **P2** — `--omit-functions` / `--omit-variables` **empty** the array instead of **omitting** it, so `[]` is indistinguishable from a genuinely empty array

**New?** **NEW as stated.** Cycle 5 mentions the flag only in passing ("`--omit-variables` (`Runner.cs:84-88`)
keeps the payload opt-out-able", `docs/goal-cycle-5/diagnosis/D-D-robustness.md:462`); no report records the
empty-array shape. The *sibling* cancel-path defect (E, F4) is recorded and is **not** re-counted here (§
"documented bounds…", item 6).

**Important honesty note:** the requirement I was asked to test — *"–omit-functions / --omit-variables must not
change any structured value or the display"* — **HOLDS**. I compared the full envelopes (four flag combinations ×
10 scripts: Real, Symbolic, SolveResult, Array, print, invalid-argument error, parse error, `capabilities()`,
limit, `pi(200)`) after deleting the omitted keys from the unflagged envelope: every other byte is unchanged, the
`revision` is unchanged (81/81/81/81, 82/82/82/82), and the error/parse envelopes are byte-identical with and
without the flags. Only the flag's stated *shape* is wrong.

### Exact command

```powershell
$exe = 'C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe'
'x = 1; y = 2' | & $exe --stdin --json --omit-variables
'x = 1; y = 2' | & $exe --stdin --json --omit-functions
'x = 1; y = 2' | & $exe --stdin --json
```

### Observed envelope (verbatim, trimmed; volatile fields normalized)

```
--omit-variables : …, "ok":true,"revision":83, "result":{…"display":"2"…}, "output":[],
                   "variables":[], "functions":[{ "name":"abs", … }], "timings":[…]
--omit-functions : …, "ok":true,"revision":83, "result":{…"display":"2"…}, "output":[],
                   "variables":[{"name":"_",…},{"name":"x",…},{"name":"y",…}], "functions":[], "timings":[…]
no flags         : "variables":[{"name":"_",…},{"name":"x",…},{"name":"y",…}], "functions":[{ "name":"abs", … }, …]
```

The keys are present in all three envelopes; with a flag they carry `[]`. On this script `--omit-variables`
turns a three-entry `variables` array into `[]` — the same bytes a script with no bindings produces.

### Correct behaviour, and how I know

The binary's own help text is the contract: "`--omit-functions     omit the builtin registry from the envelope
(agent loops)`" and "`--omit-variables     omit the variables array from the envelope (agent loops)`"
(`Lovelace.Run/Runner.cs:652-653`, printed by `--help`). "Omit the array" is not "send an empty array". The
implementation builds `Array.Empty<VariableDto>()` / `Array.Empty<FunctionDto>()` and the DTO always serialises
the key (`Lovelace.Run/Runner.cs:218-230`), which is why the key survives. Consequence for a consumer: an
envelope carrying `"variables":[]` no longer tells it whether the script bound anything, which is the whole point
of the field. This is a documentation/payload-shape mismatch, hence P2, not a data loss: nothing the consumer asked
to keep was altered.

---

## Documented bounds and recorded items I measured and do **not** charge as findings

1. **Display precision vs structured precision.** `setprecision(1100); pi(1100)` carries **1100** decimals in
   `result.structured.value` (EVD-287's closure confirmed on this binary) while the *display* is capped at the
   documented 100 (`Lovelace.Real/Real.cs:43,56` — `_displayDecimalPlaces = 100L`, "Corresponds to C++
   `casasExibicao`"; `docs/symbolics/a-plus-cycle-6-report.md:88`). So re-entering a *display* of
   `sqrt(2)` (100 decimals) denotes a different — exactly rational — value than the 1002-digit
   `structured.value`. **Not charged:** the protocol states the envelope exists so that "an agent never has to
   parse a display string to recover mathematical meaning" (`dsh-protocol.md:3-5`), and the cap is documented.
2. **`timings[].position` for a BOM-carrying stdin.** The same 46 bytes reach the binary as `0,8,20,31`
   through `--file` and `cmd /c "… --stdin < file"`, but as `1,9,21,32` through a PowerShell pipe (PS 5.1
   prepends a UTF-8 BOM) and `1,9,21,32` for a deliberately BOM-prefixed file. Explicitly documented and pinned
   by the positions round: "`--file` decodes with BOM detection … so its offsets index the text after the BOM,
   while `--eval`/`--stdin` keep the caller's BOM and include it. Both are literal offsets into the text the
   runner received" (`round-19/positions-implementation.md:546-548`; test
   `LeadingBom_PositionsStayOffsetsIntoTheCallersText`, `:1122-1127`). **Not charged** — it is a documented,
   test-pinned boundary, and the LF/CR-free case is correct (`--file`, LF: `0,8,20,31` = the true offsets).
3. **`--cancel-after` with a budget that never fires.** It adds exactly one documented key,
   `"cancellation":{"budgetMs":600000,"elapsedMs":1.148,"stopped":false,"exceeded":false,"excessMs":0}`
   ("the envelope's cancellation block reports the deadline verdict", `Runner.cs` `--help`), while `result`,
   `variables` and `output` are byte-identical to the unflagged run (verified by JSON comparison). Cycle 5
   measured the same (T08/CB5). **Not charged.**
4. **Fired `--cancel-after` cut point varies with machine speed.** Cycle 5 recorded this as inherent
   (CB10, "cut point varies run to run"); I did not re-probe it beyond the partial-value check below.
   **Not charged** (documented wall-clock bound).
5. **`--print-budget` non-monotone pretty rendering.** `expand((x + 1)^10)` with `--print-budget 12` shows 8
   pretty terms, with `16` only 3. This is the already-recorded audit-D **F3** / audit-E **F7** (P2), so it is a
   **match, not a re-report**. The protocol's own truncation contract (`dsh-protocol.md:17-23`) *held* in every
   probe: the budgeted `canonical` is a genuine ` …`-terminated prefix of the unbudgeted one
   (`prefixOfFull=True` at budgets 4/8/12/16/20/40), with `truncated:true`, `truncationReason:"node-budget"`
   and `budget` = the requested value.
6. **`--omit-variables` on the cancelled path** leaves `partialVariables` populated (audit-E **F4**, P2,
   recorded in EVD-291). Reproduced: the cancelled envelope's key set is
   `…,output,partialOutput,partialVariables,cancellation` with `partialVariables = x,y`. **Match, not a
   re-report.**

## Closed items of the two earlier waves that I re-verified on this binary (no re-report)

All confirmed closed on the published binary, so none of them is a finding here:
**EVD-283** `det(1)` → `InvalidArgument/TypeMismatch` "det(): argument 1 must be a square matrix; got
Natural."; **EVD-286** `setprecision(31); evalf(sin(pi(30)), 40)` →
`0.0000000000000000000000000000005` (leading digit 5, not 4); **EVD-287** `setprecision(1100); pi(1100)` →
1100 decimals in `structured.value`; **EVD-288** `zeros(1000000000)` →
`AllocationRefused/BudgetExceeded` and `len([[],[]])` → `2`; **EVD-293** LF script positions
`0,8,20,31` = the true source offsets; **EVD-294** `print("hello"); det(1)` → exit 1 with
`"output":["hello"]`. The position-related P1s (audit E F1/F2/F3) show no trace on this binary.

## What I attacked and could not break (no finding)

* **Repeat runs, byte-for-byte.** 22 shapes × 6 runs and 99 shapes × 4 runs; 3 of the 99 varied (H-1).
* **Order independence.** Three blocks of semantically independent statements in 3 permutations each
  (`a = 1/3`, `b = sqrt(2)`, `c = sin(1)`, `d = det(eye(2))`; `h = 2^100`, `i = fft([1,2,3,4])`,
  `j = sum(1..50)`; `e = factor(x^2-1)`, `f = diff(cos(x),x)`, `g = solve(x-7,x)` with `x` defined
  first): **every named variable's `structured` value is identical across all orderings**; the only variable
  that differs is `_`, the last-result alias, which *correctly* follows the last statement.
* **How often it is run.** The same expression after 0…6 preceding statements
  (`'0;' * k + '1/3 + sin(1/2)'`) returns the identical canonical every time, with `timings` growing 1→7 as
  documented.
* **Within-process repetition and precision scoping.** `setprecision(30); print(pi(30)); print(pi(30))`;
  `print(sin(1)) ×3`; `setprecision(30); a = pi(30); setprecision(80); b = pi(80); setprecision(30);
  c = pi(30); print(a); print(c); print(a - c)` → `a == c`, `a - c = 0`.
* **Failing statement does not disturb earlier values.** `a = 1/3; b = sqrt(2); print(a); print(b); det(1)`
  → exit 1, `output` identical to the succeeding control (`["0.(3)", "1.414…"]`), code
  `InvalidArgument/TypeMismatch`. On the cancelled path `partialVariables` for `a` and `b` match the full
  run's `variables` byte-for-byte, and `partialOutput` carries both prints.
* **Direct vs `subs` vs solver.** `subs(x^2 + x, x, 1/3)` = `4/9` against `(1/3)^2 + 1/3` = `0.(4)` —
  same number, `a - b` prints `0`; `subs(sin(x), x, 1)` = `sin(1)`, difference `0`; `solve(x - 1/3, x)`
  root `1/3`; `solve(x^2 - 2, x)` roots `±1/2*sqrt(8)` re-enter as the same values.
* **`--omit-*` differential** (H-2 section): no structured value, display, `revision`, `output` or timing is
  altered by either flag.
* **Round-trip of published forms.** 44 expressions printed and re-entered: exact for Naturals, exact rationals
  (`0.(3)`, `0.(142857)`, `3.(142857)`, `-0.(3)`, `1.75`), decimal literals with non-zero last digit (50/50
  digits), vectors, `sin(1)`, and `pi(50)`'s digit string (the trailing zero is suppressed: `…7510` → `…751`,
  numerically the same value). The only failures are the documented display-precision case (item 1 above) and a
  display containing a free symbol (`(x + 1)^2` cannot re-enter without `x`, which `freeSymbols` tells you).
* **Positions as a round-trip key.** Slicing an LF source at the reported `timings[].position` and re-entering
  each slice reproduces that statement's `variables` value exactly (script
  `a = 1/3\nb = sqrt(2)\nc = sin(1)\nd = det(eye(2))`, positions `0,8,20,31`).
* **Flag permutations and duplicates.** `--json --omit-functions --omit-variables` and four permutations
  (including `--omit-functions` twice) produce the identical normalized envelope.
* **Environment, culture, cwd.** Identical 16-hex digest for the same file under `LANG/LC_ALL/LANGUAGE=de_DE`,
  `DOTNET_SYSTEM_GLOBALIZATION_INVARIANT=1`, `chcp 437`, `chcp 65001`, and `cwd = C:\Windows`
  (baseline `371BF2D1A3F4C6AF` in all six).
* **Concurrency.** 6 simultaneous processes on one script → 6 identical normalized envelopes (the concurrency
  probe does not rescue H-1: the 12-process run of the `series(abs…)` script yields 12 *different* ones).
* **Hash-order risks.** 30 free symbols in one expression → `freeSymbols` and `canonical` digests identical over
  5 runs; multivariate `expand/factor/hessian/jacobian/solve_system/collect/apart` each 4/4 identical;
  `capabilities()` identical (it is name-sorted, `Runner.cs:228`); `assumptions()` identical.
* **Default-seeded DSP.** `noise(1, 0.5, 4)` and `noise(2, 0.1, 6)` with the seed omitted are 4/4 identical
  (the RNG is `new Random(seed)`, `Lovelace.Dsp/Signals.cs:148`).
* **Filesystem side effects.** A non-plot script leaves the working directory empty (`0` files, run in a fresh
  `%TEMP%\auditH\work1`); `plot([1,2,3])` writes `plot.svg` into the cwd by documented default and its
  `plot.svg` payload is byte-identical over three runs (SVG SHA-256 `4CA0B257FC27EE9E`, 14 636 chars, also
  identical on disk).
* **Empty and whitespace scripts** (`""`, `" "`, `"\n"`, `"\t"`) → `ok:true`, no `result` key, no error;
  `";"` is a `ParseError` (a dialect choice, not a determinism defect).
* **Channels.** `--stdin`, `--file` and `--eval` give the same value fingerprint for the same text;
  `--text` agrees with the JSON `display` (`= 1/3 + sin(1/2) (Symbolic)`).

## Findings table

| id | severity | new? | one-line repro |
|---|---|---|---|
| H-1 | **P0** | NEW | `'x = symbol("x"); series(abs(x), x, 0, 3)' | & out\aot\Lovelace.Run.exe --stdin --json --omit-functions --omit-variables` → two runs give `…diff(0, __te8fa6c)…` and `…diff(0, __t9c814e)…`; 4/4 and 12/12 runs distinct; `Series.cs:56` `Guid.NewGuid()` |
| H-2 | P2 | NEW | `'x = 1; y = 2' | & … --stdin --json --omit-variables` → `"variables":[]` (key present, emptied) while `--help` says "omit the variables array from the envelope" |

## Could not check

* **Platforms other than win-x64.** Determinism was probed only on this machine (the published AOT exe and the
  local Release JIT build). Float formatting or ordering on another OS/architecture is untested.
* **The Studio/REPL surfaces.** Only the non-interactive runner was driven; the same kernel temporary could
  surface there, but I did not test it.
* **Fired `--cancel-after` determinism.** I confirmed partial values are preserved and identical to a full run,
  but I did not attempt to characterize the varying cut point beyond cycle 5's recorded CB10.
* **Concurrency at the plot layer.** I ran 12 concurrent non-plot processes; I did not run concurrent
  `plot(...)` writers into one `--plot-dir`/`--plot-file`.
* **Every builtin.** The sweeps cover ~99 shapes; builtins outside those shapes (e.g. `compile/evalir`,
  `solve_system_full`, `optimize_full`, the `*_full` limit/integrate records beyond those probed) were not
  each repeated N times.
* **SymPy cross-checks of values.** Ground truth was used only for the series-of-`Abs` reference; this audit is
  about agreement between runs, not about numerical correctness, and I did not re-verify values against mpmath.
* **Older published binaries.** I did not bisect when the leak was introduced beyond `git blame` (it is present
  in `9dc44ee`).

## Scope / files

Nothing outside this file was written in the repository. Scratch inputs (`.ls` files, plot SVGs, redirected
stdout) lived in `%TEMP%\auditH`, which was deleted after the last probe; the repository's product, tests,
fixtures and documents were only read. `git status --porcelain` before this file was written showed only
pre-existing changes (` M docs/goal-cycle-6/evidence.md`, ` M docs/goal-cycle-6/journal.md`,
`?? docs/goal-cycle-6/round-20/`).
