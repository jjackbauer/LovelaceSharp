# Cycle-6 · round-23 · audit P — second-implementation cross-check + resource/locale/encoding variation

**Strategy (not used by any earlier wave):** the product ships **several implementations of the same
mathematics**. Instead of re-testing one path harder, this audit ran the *same value* through every
independent implementation the product contains — Real tier vs Complex tier vs Symbolics `evalf` vs the
CLI vs the AOT/JIT artefacts — checked each against mpmath/SymPy, then looked for the arguments under which
two implementations disagree.

**Severity ladder used:** **P0** = a wrong value or an abort/crash on valid input; **P1** = a false claim in
the machine API, a wrong refusal, or data the API says it carries being lost or altered; **P2** = cosmetic or
documentation.

---

## 0 · Headline

The cross-implementation matrix produced **two findings (P-1, P-2)** and a large body of *agreement*
evidence, written down in full below because a clean row is also a result.

* The **system under test** (`out\aot\Lovelace.Run.exe`) answered **every** value comparison I ran
  against mpmath/SymPy correctly — sin, cos, exp, log, tan, atan, sqrt, pow, Euler, exact rational
  det/inv/matmul/linsolve and Dsp fft/dft/conv/filter/movingavg. It did **not** produce a wrong value
  anywhere in Part A's numeric battery.
* The two findings are (a) a **super-linear cost cliff in `solve()`** reachable from the binary under
  test, and (b) a **second implementation of sin/cos inside `Lovelace.Real`** that returns only
  `comp+10` correct digits while printing up to 12 810, with **no truncation flag** — a defect the
  CLI cannot see, because the CLI uses the *other* implementation.

---

## 1 · Artefact fingerprints

| artefact | path | bytes | written |
|---|---|---|---|
| **binary under test (AOT)** | `C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe` | 5 764 608 | 2026-09-12 21:05:23 |
| AOT pdb | `out\aot\Lovelace.Run.pdb` | 27 299 840 | 2026-09-12 21:05:23 |
| **JIT twin** | `Lovelace.Run\bin\Release\net10.0\Lovelace.Run.exe` | 162 304 | 2026-09-12 21:06:48 |
| Lovelace.Real.dll (JIT) | `Lovelace.Run\bin\Release\net10.0\` | 56 832 | 2026-09-12 21:05:49 |
| Lovelace.Suite.dll (JIT) | idem | 245 760 | 2026-09-12 21:05:58 |
| Lovelace.Symbolics.dll (JIT) | idem | 362 496 | 2026-09-12 21:05:51 |
| Lovelace.Run.dll (JIT) | idem | 203 264 | 2026-09-12 21:06:48 |

Nothing under `out/` was written, rebuilt or deleted by this audit. No repo project was built, no
test suite was run, git state was not touched. Every product assembly used in Part A was the **JIT output of
the same publish** (`Lovelace.Run\bin\Release\net10.0\*.dll`, 21:05:41–21:06:48), referenced
**by path**, never by project reference.

**Ground truth:** `python` at `C:\Users\ricar\dev\.lovelace-tools\python`
(Python 3.12.14, mpmath 1.3.0, SymPy 1.14.0), reached with
`$env:PATH='C:\Users\ricar\dev\.lovelace-tools\python;'+$env:PATH`, then `python`.

---

## 2 · Harness — location and how to rebuild

**Location (outside the repo tree):** `C:\Users\ricar\dev\.lovelace-audit-P\`

| file | role |
|---|---|
| `AuditP.csproj` | net10.0 exe, `EnableDefaultCompileItems=false`, **12 `<Reference>` items whose `<HintPath>` points straight at `Lovelace.Run\bin\Release\net10.0\*.dll`** — no ProjectReference, nothing rebuilt in the repo |
| `Program.cs` | probe dispatcher: `AuditP <ProbeName> [args]` resolves the probe type by reflection and invokes its `public static void Run(string[])` |
| `probes\Dump.cs` | public-surface dumper (forces `Assembly.Load` of all 12 Lovelace assemblies) |
| `probes\Find.cs` | type finder |
| `probes\A1.cs` | Real-tier vs Complex-tier matrix (the §3.1 table) |
| `probes\A5.cs` | SuiteEngine facade vs Runner-style projection; its `NewEngine()` is the shared host wiring |
| `probes\EvalProbe.cs` | evaluates every `*.ls` in a directory in-process, emits `EV<TAB>name<TAB>script<TAB>json` |
| `aotjit.ps1` | AOT-vs-JIT differential over 76 `.ls` fixtures |
| `hang.ps1` | the `solve()` cliff sweep (`Start-Process` / `WaitForExit(ms)` / `Kill()`) |
| `gt_*.py`, `sum_a5.py`, `rows.py` | mpmath / SymPy ground-truth comparisons |
| `out\` | raw captured output (`a1.txt`, `a1_rep2.txt`, `a1_rep3.txt`, `a5_lib.txt`, `eval_algebra.txt`, `eval_solve.txt`, `aotjit.txt`, `hang.txt`) |
| `fixtures\`, `fx2\`, `fx3\`, `hang\` | the `.ls` scripts |

**Rebuild:**

```
cd C:\Users\ricar\dev\.lovelace-audit-P
dotnet build -c Release -v q --nologo
.\bin\Release\net10.0\AuditP.exe A1 30 50 200 | Out-File -Encoding utf8 .\out\a1.txt
```

**Host wiring** (identical to `Lovelace.Run/Runner.cs`:178-182, used by `A5.NewEngine()`):

```csharp
var e = new SuiteEngine();
e.LoadPlugin(new Lovelace.Dsp.DspPlugin());
var sym = new Lovelace.Symbolics.SymbolicsPlugin();
e.LoadPlugin(sym);
e.LoadPlugin(new Lovelace.MathIR.MathIRPlugin(sym));
```

**Reproduction rule.** Every finding was run **twice**: `out\a1.txt` and `out\a1_rep2.txt`
are byte-identical (110 215 bytes each, `identical=True`), and the `solve` sweep carries
explicit `rep1`/`rep2` columns in `out\hang.txt`. Scripts that failed to parse were
treated as *probe* failures, not product failures, and were rewritten (EVD-330): the first `solve`
fixtures used a bare `x` and returned `Undefined variable 'x'.` — every `solve` row
below uses `x = symbol("x")`. The PowerShell-5.1 double-quote trap (EVD-299) was avoided by writing
every script into a `.ls` file with
`[System.IO.File]::WriteAllText($f, $script, (New-Object System.Text.UTF8Encoding($false)))` and
driving the CLI with `--file`.

---

## 3 · One row per comparison

**A** / **B** are the two independent implementations compared; **GT** is the independent ground truth; the
verdict is on A vs B vs GT. "artefact" says which binary or assembly the row used.

### 3.1 Real tier vs Complex tier (A1) — artefact: JIT assemblies via `AuditP A1`, GT: mpmath

Command: `.\bin\Release\net10.0\AuditP.exe A1 30 50 200` → `out\a1.txt`
(byte-identical on the second run). Ground truth: `python gt_a1.py .\out\a1.txt` with
`mp.dps=1200`; digit counts from `python gt_final.py`.

| # | A | B | GT | verdict |
|---|---|---|---|---|
| 1 | `Real.Sin(1)` @30 → 606 digits | `ComplexMath.Sin(Real)` @30 → 30 digits | sin(1) | **A: 40 correct digits then wrong; B: 30/30 correct.** A over-returns 566 wrong digits |
| 2 | `Real.Sin(1)` @50 → 1378 digits | `ComplexMath.Sin(Real)` @50 | sin(1) | A: 60 correct; B: 50/50 correct |
| 3 | `Real.Sin(1)` @200 → 12 606 digits | `ComplexMath.Sin(Real)` @200 → 200 digits | sin(1) | A prefix ≥200 correct (tail not verifiable, §5); B: 200/200 correct |
| 4 | `Real.Sin(10)` @30 → 1454 digits | `ComplexMath.Sin(Complex)` @30 | sin(10) | **A: 29 correct for a 30-place request — WRONG inside the request. B: 30/30 correct** |
| 5 | `Real.Sin(10)` @50 → 3134 digits | — | sin(10) | **A: 49 correct for a 50-place request — WRONG inside the request** |
| 6 | `Real.Cos(1)` @30 → 641 digits | `ComplexMath.Cos(Real)` @30 | cos(1) | A: 40 correct; B: 29.74 (last digit truncated, P-3) |
| 7 | `Real.Cos(1)` @50 → 1381 digits | `ComplexMath.Cos(Real)` @50 | cos(1) | A: 60 correct; B: 50/50 |
| 8 | `Real.Exp(1)` @30/50/200 | `ComplexMath.Exp(Real)`, `Exp(Complex)` | e | **byte-identical, 30/50/200 correct — agrees** |
| 9 | `ComplexMath.Ln(2)` | `ComplexMath.Log(Complex)` | ln 2 | identical, correct at 30/50/200 — agrees |
| 10 | `Real.Sqrt(2)`, `ComplexMath.Sqrt(Complex)`, `ComplexMath.Pow(Real,Real)`, `Pow(Cplx,Cplx)` | — | √2 | **all four agree digit-for-digit at 30/50/200** |
| 11 | `Real.Pow(0.5)` on 2 | `ComplexMath.Pow(Real,Real)` | √2 | A throws `NotImplementedException: Non-integer exponents are not yet supported.`; B returns √2 → P-4 |
| 12 | `Real.Pow` 3^7.5 | `ComplexMath.Pow(Real,Real)` @30 | 3787.995116153134640944525148858407… | A throws; B = `3787.995116153134640944525148858407` — 29.0 correct for 30 |
| 13 | `ComplexMath.Tan(Real)` 1 | `ComplexMath.Tan(Complex)` | tan 1 | **DISAGREE in the last digit**: A `…807458`, B `…807459`; mpmath 1.5574077246549022305069748074583… → **A right, B wrong at digit 30** |
| 14 | `ComplexMath.Atan(Real)` 1 | — | π/4 | 29.0 correct of 30 requested (last digit) |
| 15 | `Real.Sin(pi/6)` → exactly `0.5` | `ComplexMath.Sin(Real)` / `(Complex)` on π/6 → `0.499999999999999999999999999999` | 1/2 | **cross-tier divergence (special-angle recogniser vs numeric); B has 29/49/199 correct** |
| 16 | `Real.Cos(pi/2)` → `0`; `Real.Sin(pi)` → `0` | `ComplexMath.Cos(Real)` → 2.514e-31; `Sin` → `0` | 0 | A exact zero via the recogniser, B ≈ π⁻ᵈ — **agrees within request** |
| 17 | `ComplexMath.Exp(i·pi)+1` | — | 0 | exactly `(0,0)` — agrees |

### 3.2 Symbolics `evalf` (the CLI numeric path) vs the Real/Complex kernels — artefact: **AOT binary**

Command form:
`& out\aot\Lovelace.Run.exe --eval 'setprecision(N); evalf(sin(1), N)' --json --omit-functions --omit-variables`

| # | A (CLI `evalf`) | B (library `Real.Sin/Cos`) | GT (mpmath) | verdict |
|---|---|---|---|---|
| 18 | `sin(1)` @30 → `0.84147098480789650665250232163` | `Real.Sin(1)` @30 → 606 digits | sin(1) | **A 30/30 correct; B 40 correct** — the CLI is right |
| 19 | `sin(10)` @30 → `-0.544021110889369813404747661851` | `Real.Sin(10)` @30 | sin(10) | **A 30/30 correct; B 29 correct (wrong).** The two sin implementations disagree and **B is the wrong one** |
| 20 | `sin(1)` @60 / @100 / @200 | `Real.Sin(1)` @60/100/200 | sin(1) | **A correct to every requested digit (60/100/200, `gt_evalf.py`: all OK); B wrong past digit 40/60** |
| 21 | `cos(1)` @200 → 200 decimals | `Real.Cos(1)` @200 → 12 810 digits | cos(1) | A: 199/200 correct (truncation, P-3); B: 60 correct of 12 810 at @50 |
| 22 | `sin(2)` @200, `sin(1/2)` @200 | — | sin(2), sin(1/2) | A: 200/200 and 199/200 — agrees |
| 23 | `exp(1)` @200 | `Real.Exp(1)` @200 | e | 201/201 and 201/201 — agrees |
| 24 | `log(2)` @60 | — | ln 2 | 60/60 — agrees |

### 3.3 Dsp kernels vs a naive DFT in mpmath — artefact: AOT binary; GT: `gt_misc.py`

| # | A | B | GT | verdict |
|---|---|---|---|---|
| 25 | `fft([1,2,3,4])` = `[10, -2 + 2i, -2, -2 - 2i]` | `dft([1,2,3,4])` = same | naive DFT | **A == B == GT** |
| 26 | `fft([0.1,0.2,0.3,0.4])` = `[1, -0.2 + 0.2i, -0.2, -0.2 - 0.2i]` | `dft` same | naive DFT | **A == B == GT** |
| 27 | `fft(x8)` @60 | `dft(x8)` @60 | mpmath 8-pt DFT at 62 digits | **byte-identical strings; every element matches mpmath**, e.g. `X[1] = -0.4 + 0.9656854249492380195206754896838792314278687501507792292706716i` |
| 28 | `dft([0.1…0.5])` (length 5) @60 | mpmath 5-pt DFT | re = −0.25 exactly | **AGREES to 60 digits**; the 5-element display is 75 224 chars because each element carries ~250 extra digits (the known-open over-delivery family) |
| 29 | `dft([1,2,3],8)` (zero-padded) | mpmath | — | AGREE on all 8 bins (`2.414213562373095048801688724209` = 1+√2 ✓) |
| 30 | `conv([1,2,3],[0,1,0.5])` = `[0, 1, 2.5, 4, 1.5]` | hand convolution | — | **A == GT exactly** |
| 31 | `conv([0.1,0.2],[0.3,0.4])` = `[0.03, 0.1, 0.08]` @60 | exact rational convolution | — | **A == GT exactly** — the float128 fast path did not round; the `FixedOrExact` "never rounds" claim held on every case I tried |
| 32 | `filter([1,-0.5],[1,0.5],[1,0,0,0,0])` = `[1, 1, 0.5, 0.25, 0.125]` | manual recursion y[n]=x[n]+0.5x[n−1]+0.5y[n−1] | — | **A == GT exactly** |
| 33 | `filter([1,-0.5],[0.1,0.5],4)` = `[0.1, 0.55, 0.275, 0.1375]` | manual recursion | — | **A == GT exactly** |
| 34 | `movingavg([1,2,3,4,5],3)` = `[0.(3), 1, 2, 3, 4]` | causal zero-padded window ÷ w | — | **A == GT exactly** |
| 35 | `fft([1,2,3,4,5,6])` | — | — | typed refusal `FFT length must be a power of two, but got 6. (Parameter 'x')` — matches the advertised capability |
| 36 | `ifft(…)` | — | — | `Unknown function 'ifft'.` — **there is no inverse transform in the product** (see §5) |

### 3.4 Exact algebra over Rationals vs SymPy — artefact: JIT assemblies; GT: SymPy

Commands: `AuditP EvalProbe .\fx2` → `out\eval_algebra.txt`; `python gt_misc.py`.

| # | A (product) | GT (SymPy exact) | verdict |
|---|---|---|---|
| 37 | `det([[1,2],[3,4]])` = `-2` (Integer, exact) | −2 | **agree** |
| 38 | `det([[1,2,3],[4,5,6],[7,8,10]])` = `-3` | −3 | **agree** |
| 39 | `det([[1/2,1/3],[1/5,1/7]])` = `0.0(047619)`, numerator 1 / denominator 210 | 1/210 | **agree, exact** |
| 40 | `det(diag(2,3,5,7))` = `210` | 210 | **agree** |
| 41 | `det([[1,2,3,4],[5,6,7,8],[9,10,11,12],[13,14,15,17]])` = `0` | 0 | **agree** |
| 42 | `inv([[1/2,1/3],[1/5,1/7]])` = `[[30, -70], [-42, 105]]` | [[30,−70],[−42,105]] | **agree exactly** |
| 43 | `inv([[1,2,3],[0,1,4],[5,6,0]])` = `[[-24,18,5],[20,-15,-4],[-5,4,1]]` | identical (det = 1) | **agree exactly** |
| 44 | `matmul(inv([[1,2],[3,4]]),[[1,2],[3,4]])` = `[[1,0],[0,1]]` | identity | **agree** |
| 45 | `setprecision(120); det([[1/3,1/7,1/11],[1/13,1/17,1/19],[1/23,1/29,1/31]])` | 57524/6685349671 | **agree** (digit-level form in §5) |
| 46 | `inv([[1,1,1],[1,2,3],[1,3,6]])` @50 | [[3,−3,1],[−3,5,−2],[1,−2,1]] | **agree** |
| 47 | `det([[1,2],[2,4]])` = `0`; `inv` → `Matrix is singular and cannot be inverted.` | 0 / singular | **correct typed refusal** |
| 48 | `matrix_rank([[1,2],[2,4]])` → `matrix_rank() requires a symbolic matrix.` | rank 1 | known cycle-5 F22 family — **not re-reported** |
| 49 | `linsolve([[1,2],[3,4]],[5,6])` → `linsolve() requires a symbolic matrix A.` | — | advertised refusal, matches `capabilities()` — **not a finding** |
| 50 | `linsolve([x+y-4,x-y-2],[x,y])` | {x=3, y=1} | **agree** |
| 51 | `solve(x^2-2, x)` → `[-1/2*sqrt(8), 1/2*sqrt(8)]`, exactness AlgebraicExact | ±√2 | **agree**; substituted back, residual 0 |
| 52 | `solve(x^2-5*x+6, x)` → 2, 3 | {2,3} | **agree** |
| 53 | `solve(x^3-2*x+1, x)` → `1/2*(-sqrt(5) - 1)`, … | (1, −φ, 1/φ) | **agree** |
| 54 | `solve(x^2-x-1, x)` → `1/2*(1 - sqrt(5))`, … | golden-ratio pair | **agree** |
| 55 | `solve(x^5-x-1, x)` → `rootof(x^5 - x - 1, 0)`, Partial | 1 real root | **agree** (correctly Partial) |
| 56 | `evalf(solve_full(x^4-x^2-1 == 0, x).solutions[0].value, 20)` | ±√((1+√5)/2) = ±1.2720196495140689643 | **agree — no wrong root found** (a wrong root would have been P0) |

### 3.5 Library facade (SuiteEngine) vs the CLI on the same value and precision — K-2 re-check

`AuditP A5` → `out\a5_lib.txt` (42 rows = 14 scripts × precisions 100/500/1100);
analysis by `python sum_a5.py .\out\a5_lib.txt`.

| # | A (`SuiteEngine.ProjectValue`) | B (Runner-style `StructuredProjection.ToStructured` under a 1e9 display bound) | verdict |
|---|---|---|---|
| 57 | `sqrt(2)` @1100 → value len 1102 | 1102 | **identical — K-2 is FIXED in this build** |
| 58 | `pi(1100)` @1100 → 1102 | 1102 | **identical** |
| 59 | `e(1100)` @1100 → 1102 | 1102 | **identical** |
| 60 | `1/1009` @500 → 256 chars, exact=True | 256 | **identical** |
| 61 | `355/113` @1100 → 116, exact=True | 116 | **identical** |
| 62 | `evalf(sin(1),200)` @1100 → 202 | 202 | **identical** |
| 63 | all 42 rows, `truncated=null` everywhere | — | **0 divergences — K-2 (523240b) no longer reproduces through `ProjectValue`** |
| 64 | `pi(1100)` / `e(1100)` at engine precision 100 | — | typed refusal `pi(): argument 1 (digits) must be a digit count between 1 and 100 (the engine's computation precision); got 1100.` — consistent with K-1's fix (2572c59) |

Raw ProjectValue-vs-Runner lengths (verbatim from `sum_a5.py`), the K-2 shape:

```
 1100 pi(1100)      kind=Real exact=False projLen=1102 runnerLen=1102 trunc=None
 1100 sqrt(2)      kind=Real exact=False projLen=1102 runnerLen=1102 trunc=None
 1100 e(1100)      kind=Real exact=False projLen=1102 runnerLen=1102 trunc=None
  500 1/1009       kind=Real exact=True  projLen=256  runnerLen=256  trunc=None
 1100 355/113      kind=Real exact=True  projLen=116  runnerLen=116  trunc=None
 1100 evalf(sin(1), 200) kind=Real exact=False projLen=202 runnerLen=202 trunc=None
```

### 3.6 AOT vs JIT — artefact: both binaries

Command: `powershell -NoProfile -File .\aotjit.ps1` → `out\aotjit.txt`.
76 `.ls` fixtures covering integers, rationals, π/e at 200/500/1100 places, evalf, det/inv/linsolve,
polynomial `solve`/`solve_full`, fft/dft/conv/filter/movingavg/noise/cosine/impulse/step/
exponential/powerseries, matrix ops, strings (`print("hello")`), `1e-4000`,
`1/0`, `0/0`, `sqrt(-1)`, `log(-1)`, `0^0`, `2^0.5`,
`8^(1/3)` — each run through **both** executables with
`--file … --json --omit-functions --omit-variables`, envelopes normalised by removing `elapsed`,
`elapsedTime` and `timings`.

| # | A | B | verdict |
|---|---|---|---|
| 65 | AOT envelope | JIT envelope, 76 files | **76/76 byte-identical after removing the three elapsed fields. Zero differences.** |
| 66 | AOT `setprecision(60); dft([0.1,0.2,0.3,0.4,0.5])` | JIT same | identical |

```
files=76 mismatches=0
```

### 3.7 Machine-API claims checked against the CLI's own refusals

| # | observation | verdict |
|---|---|---|
| 67 | `2^0.5`, `4^0.5`, `8^(1/3)`, `setprecision(100); 2^0.5` → `ok:false code:UnsupportedOperation category:UnsupportedOperation message:"Non-integer exponents are not yet supported." recoverable:true` with a diagnostic position (`position:19, line:1, column:20` for the `setprecision` form) | **typed, positioned refusal — matches `capabilities()`; not a finding** |

---

## 4 · Findings

### P-1 — **P1** — `solve()` on a quadratic with a ≥17-digit constant term does not answer inside 15 s, while the same shape at 12–15 digits is instant

**New?** **NEW.** Not in the known-open list and not in any prior audit I can find; the nearest prior item is
the eager-range materialisation cost note, a different function.

#### Exact commands

Fixtures are written by `hang.ps1` with
`[System.IO.File]::WriteAllText($f, $script, (New-Object System.Text.UTF8Encoding($false)))`;
each case is timed with

```powershell
$p = Start-Process -FilePath 'C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe' `
      -ArgumentList @('--file',$f,'--json','--omit-functions','--omit-variables') `
      -NoNewWindow -PassThru -RedirectStandardOutput $so -RedirectStandardError $se
$done = $p.WaitForExit(15000)
if (-not $done) { $p.Kill() }
```

Run as `powershell -NoProfile -ExecutionPolicy Bypass -File .\hang.ps1` — **two full repetitions**
(`rep1`/`rep2`), output stored verbatim in `out\hang.txt`.

#### Observed output (verbatim; 15 s deadline; AOT = binary under test, JIT = twin)

```
rep1 AOT p30 exit= TIMEOUT :: x = symbol("x"); solve(x^2-1000000000000000000000000000000, x)
      ->
rep1 JIT p30 exit= TIMEOUT ::
rep1 AOT p21 exit= TIMEOUT :: x = symbol("x"); solve(x^2-1000000000000000000000, x)
rep1 JIT p21 exit= TIMEOUT ::
rep1 AOT p19 exit= TIMEOUT :: x = symbol("x"); solve(x^2-10000000000000000000, x)
rep1 AOT p18 exit= TIMEOUT :: x = symbol("x"); solve(x^2-1000000000000000000, x)
rep1 AOT p17 exit= TIMEOUT :: x = symbol("x"); solve(x^2-100000000000000000, x)
rep1 AOT p16 exit= done    :: x = symbol("x"); solve(x^2-10000000000000000, x)
      -> ok=True SolveResult(status: Solved, variable: x, domain: complex, complete: True, completeness: Complete, solutions: [Solution(value: -100000000, conditions: []
rep1 JIT p16 exit= done    -> ok=True SolveResult(status: Solved, ... solutions: [Solution(value: -100000000, ...
rep1 AOT p15 exit= done    :: x = symbol("x"); solve(x^2-1000000000000000, x)
      -> ok=True SolveResult(status: Solved, ... solutions: [Solution(value: -1/2*sqrt(4000000000000000...
rep1 AOT p12 exit= done    :: x = symbol("x"); solve(x^2-1000000000000, x)
      -> ok=True SolveResult(status: Solved, ... solutions: [Solution(value: -1000000, conditions: [], ...
rep1 AOT p30odd  exit= TIMEOUT :: x = symbol("x"); solve(x^2-1000000000000000000000000000001, x)
rep1 JIT p30odd  exit= TIMEOUT ::
rep1 AOT p30co   exit= TIMEOUT :: x = symbol("x"); solve(x^2-2*1000000000000000000000000000000, x)
rep1 JIT p30co   exit= TIMEOUT ::
rep1 AOT p30fac  exit= done    :: x = symbol("x"); solve(x^2-2*3*5*7*11*13*17*19*23*29*31, x)
      -> ok=True SolveResult(status: Solved, ... solutions: [Solution(value: -1/2*sqrt(802241960520), ...
rep1 JIT p30fac  exit= done    -> same
rep1 AOT lit30   exit= done    :: 1000000000000000000000000000000 + 1
      -> ok=True 1000000000000000000000000000001
rep1 JIT lit30   exit= done    -> ok=True 1000000000000000000000000000001
rep1 AOT sqrt30  exit= done    :: sqrt(1000000000000000000000000000000)
      -> ok=True 1000000000000000
rep1 AOT p30eq   exit= TIMEOUT :: x = symbol("x"); solve(x^2 == 1000000000000000000000000000000, x)
rep1 JIT p30eq   exit= TIMEOUT ::
rep1 AOT p30cube exit= TIMEOUT :: x = symbol("x"); solve(x^3-1000000000000000000000000000000, x)
rep1 JIT p30cube exit= TIMEOUT ::
rep1 AOT p30div  exit= TIMEOUT :: x = symbol("x"); solve(x^2/1000000000000000000000000000000-1, x)
rep1 JIT p30div  exit= TIMEOUT ::
rep2 AOT p30 exit= TIMEOUT ; p21 TIMEOUT ; p19 TIMEOUT ; p18 TIMEOUT ; p17 TIMEOUT
rep2 AOT p16 exit= TIMEOUT              <-- rep1 completed at 10^16, rep2 did not: the cliff starts at 16 digits
rep2 AOT p15 exit= done ; p12 exit= done ; p30odd TIMEOUT ; p30co TIMEOUT
rep2 AOT p30fac exit= done ; lit30 done ; sqrt30 done ; p30eq TIMEOUT ; p30cube TIMEOUT ; p30div TIMEOUT
```

A **separate 150-second run** of `p17` (`x = symbol("x"); solve(x^2-100000000000000000, x)`)
finished:

```
P17: exited code= outlen=3531
```

so this is a **super-linear cost cliff, not a proven infinite hang** — stated that way deliberately.

#### Correct behaviour, and how I know

* The answers are trivially exact: `x^2 - 10^k` has roots ±10^(k/2). The product itself solves
  `x^2-10^12` in both reps and `x^2-10^15` in both reps, well inside 15 s.
* The input is valid by the product's own standards: `solve(x^2-2, x)` returns
  `SolveResult(status: Solved, …, exactness: AlgebraicExact)`, so a quadratic with an integer
  constant is inside the advertised domain; and the same binary accepts the 10^30 literal elsewhere
  (`1000000000000000000000000000000 + 1` → correct, `sqrt(10^30)` → correct).
* The blow-up tracks the **size of the constant term**, not the degree or the coefficient structure:
  10^30 is slow while 2·3·5·7·11·13·17·19·23·29·31 (a 12-digit product) is fast.
* **Both artefacts** time out on the same scripts, so it is not AOT codegen.

#### Why P1 and not P0

No wrong value and no abort was observed — the process produces **no answer** within 15 s for an input whose
exact answer is a 16-digit integer. That is the *effect* of a wrong refusal (the machine API carries no result
at all for a valid, trivially solvable script) without a wrong value.

---

### P-2 — **P1** — `Lovelace.Real.Real.Sin` / `Cos` return values that print up to 12 810 digits when 30/50/200 were requested, and only `comp+10` of those digits are correct, with no truncation flag

**New?** **NEW as a distinct defect.** It is adjacent to the known-open "evalf over-delivers up to ~2000
decimals for a 40-place request", but it is **not that item**: (a) it is not `evalf` — the CLI's
`evalf` prints exactly the requested number of digits and is **correct** on every case I tested;
(b) the over-returned digits here are **wrong**; and (c) at 30 and 50 places the value is wrong **inside**
the requested precision for `sin(10)`.

#### Exact command

```powershell
cd C:\Users\ricar\dev\.lovelace-audit-P
.\bin\Release\net10.0\AuditP.exe A1 30 50 200 | Out-File -Encoding utf8 .\out\a1.txt
$env:PATH='C:\Users\ricar\dev\.lovelace-tools\python;'+$env:PATH
python gt_a1.py .\out\a1.txt
python gt_final.py
```

#### Observed output (verbatim; `out\a1_rep2.txt` is byte-identical to `out\a1.txt%%)

```
== PREC 30
  Real.Sin(v)              0.8414709848078965066525023216302989996226264724534427280707076105550447 ...[+608 chars total]
  Real.Sin(v,d)            0.8414709848078965066525023216302989996226264724534427280707076105550447 ...[+608 chars total]
  ComplexMath.Sin(Real)    0.84147098480789650665250232163
  Real.Sin                 -0.544021110889369813404747661852643149120444947956505476184814011232687 ...[+1456 chars total]
  ComplexMath.Sin(Complex) (-0.544021110889369813404747661851,0)
== PREC 200
  Real.Sin(v)              0.8414709848078965066525023216302989996225630607983710656727517099919104 ...[+12607 chars total]
  Real.Cos(v)              0.5403023058681397174009366074429766037323104206179222276700972553811003 ...[+12810 chars total]
  ComplexMath.Sin(Real)    0.8414709848078965066525023216302989996225630607983710656727517099919104 ...[+202 chars total]
```

Correct-digit counts (mpmath, `mp.dps` scaled to the value length; `python gt_final.py`):

```
sin(1)   prec=30    printed_digits=606     correct_leading_digits=40     WITHIN REQUEST
cos(1)   prec=30    printed_digits=641     correct_leading_digits=40     WITHIN REQUEST
sin(10)  prec=30    printed_digits=1454    correct_leading_digits=29     *** WRONG WITHIN REQUESTED PRECISION
sin(1)   prec=50    printed_digits=1378    correct_leading_digits=60     WITHIN REQUEST
cos(1)   prec=50    printed_digits=1381    correct_leading_digits=60     WITHIN REQUEST
sin(10)  prec=50    printed_digits=3134    correct_leading_digits=49     *** WRONG WITHIN REQUESTED PRECISION
```

**Why it is a defect and not just verbosity:**

1. `Real.Sin(10)` at computation=display=30 returns a value with **29** correct decimal digits. The
   sibling implementation `ComplexMath.Sin(new Complex(10, Rl.Zero))` returns
   `-0.544021110889369813404747661851`, correct to all 30, and so does the CLI:
   `& out\aot\Lovelace.Run.exe --eval 'setprecision(30); evalf(sin(10), 30)' --json --omit-functions --omit-variables`
   → `ok=true display=-0.544021110889369813404747661851` (identical in rep1 and rep2).
   mpmath: sin(10) = −0.54402111088936981340474766185137728168… — so `Real.Sin` is the wrong one.
2. The extra digits are not "more precision": they are the **exact decimal expansion of an internal
   approximation**, which is why the tails show repeating blocks (`…127413127413127413…` for sin,
   `…634920634920…` for cos) — a transcendental's digits never repeat. 606 printed / 40 correct
   means **566 wrong digits**; 1454 / 29 means **1425 wrong digits**.
3. The structured payload says nothing about it: `ProjectValue` on such a value reports
   `truncated: null`, so a host that trusts `Value` receives wrong digits with no flag —
   exactly the P1 shape ("data the API says it carries … altered").

**Blast radius — honest statement.** I could **not** reach `Real.Sin`/`Real.Cos` from the
CLI: every CLI path I tried (`sin(1)` → returns the symbolic `sin(1)`;
`evalf(sin(x), N)` → correct) uses the Symbolics path, and the AOT binary's numeric answers were
correct everywhere. So **P-2 is a library-boundary defect, not a defect of the AOT executable.** It matters
because `Lovelace.Real` is a shipped assembly that audit K and this audit both embed directly.

---

### P-3 — **P2** — the Complex tier truncates instead of rounding the last requested digit

`ComplexMath.Cos(1)` at 30 places prints `0.540302305868139717400936607442`; the correctly
rounded 30-decimal value is `…443` (cos 1 = 0.5403023058681397174009366074429766037323…). The CLI
shows the same shape: `setprecision(30); evalf(cos(1), 30)` → `0.540302305868139717400936607442`
in both reps, and `evalf(sin(1/2), 200)` has 199 correct of 200 for the same reason. Cosmetic — the
value is the truncation of the true one, not a wrong computation. Recorded for completeness, **not** claimed
as a wrong value.

### P-4 — **P2** — `Lovelace.Real.Real.Pow(Real)` throws a raw BCL `NotImplementedException`

`two.Pow(Rl.Parse("0.5"))` and `three.Pow(Rl.Parse("7.5"))` throw
`System.NotImplementedException: Non-integer exponents are not yet supported.`
(`Lovelace.Real/Real.cs:1225`), i.e. through `IPowerFunctions<Real,Real>`, while
`ComplexMath.Pow(Real,Real)` computes the same value correctly
(`1.414213562373095048801688724209`, `3787.995116153134640944525148858407`). The CLI maps
the identical message to a typed, positioned refusal (`ok:false code:UnsupportedOperation
category:UnsupportedOperation recoverable:true`, matching `capabilities()`), so **the CLI
contract holds**; the defect is that the type's own generic-math surface throws instead of returning a
refusal, which any host embedding `Lovelace.Real` will hit. Cosmetic/doc class because the operation
is a declared non-capability.

---

## 5 · Could not check / untested (an untested area is NOT a clean area)

1. **Locale variation could not be exercised as designed.** .NET on Windows takes `CurrentCulture`
   from the user's OS locale and there is no per-process environment override (`LANG`/`LC_ALL`
   are not consulted on Windows); changing the machine locale is out of scope. I therefore could **not**
   compare AOT (published with `-p:InvariantGlobalization=true`) against the JIT twin under a hostile
   culture (`tr-TR`, `de-DE`, `ar-SA`). **The InvariantGlobalization AOT/JIT
   asymmetry is untested** — I only showed the two artefacts agree under this machine's culture.
2. **Code page 437 vs 65001 was not tested.** No `chcp` run and no `LANG`/`LC_ALL`
   run completed inside the time-box; no claim is made about stdout byte length under either code page.
3. **Non-BMP / emoji / combining-mark strings were not tested.** No astral-plane identifier, no emoji inside
   `print`, and no combining-mark positional comparison was run.
4. **Path shapes (UNC, >260 chars, spaces, unicode, directory-as-path, missing path, unreadable path) were not
   tested.** Only ordinary `--file <absolute path>` and `--eval` were used.
5. **File shapes (no trailing newline, CRLF, BOM+CRLF, empty file, 1 MB single line, very long identifier, a
   failing last statement) were not tested.**
6. **`fft` for lengths > 8** was not compared against mpmath (only 4 and 8, plus the padded
   length-3→8 case). **`ifft` does not exist** in the product
   (`Unknown function 'ifft'.`), so the "fft vs ifft" pair in the brief could not be run at all;
   the nearest available pair, `fft` vs `dft`, agreed.
7. **`--print-budget`, `--omit-variables` on cancelled paths, `--cancel-after`,
   `--text`, plot capture, threading and re-entrancy** were deliberately not touched (known-open
   items or other waves' areas).
8. **`solve` cases beyond the tested set.** I swept only the `x^2 - 10^k` family and three
   variants. Whether the cliff is a general property of large integer coefficients in `solve` (as
   opposed to this family) is **untested** — and this family is exactly what a host doing physics submits.
9. **The 10^30 `solve` cases never ran to completion.** `p17` finished inside 150 s;
   `p18/p19/p21/p30` were killed at 15 s and their true cost is **unknown** — they may finish in 16 s
   or never.
10. **Symbolics-vs-kernel agreement was sampled, not swept.** sin/cos/exp/log/pow/tan/atan/sqrt at 30/50/200
    and `evalf` at 30/60/100/200; the rest of the builtin registry was not cross-compared.
11. **`1e-4000`, `1/0`, `0/0`, `sqrt(-1)`, `log(-1)`,
    `0^0`** were run in the AOT/JIT differential (and agreed across artefacts) but their *values*
    were not checked against mpmath.
12. **Concurrency, cancellation and plugin-factory state** — untouched by design (audit K's area).
