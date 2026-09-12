# F3 — `evalf(f, N)` publishes N decimal places but computed at N significant digits

**Round 14 / cycle 6 / implementer.** Status: **fixed, tested, control-confirmed.**

## 0. Trees, revisions, artefacts

| Role | Path | Revision |
|---|---|---|
| implementation | `C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-f3` | `d8eb50d` (detached) |
| control (pristine HEAD) | `C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-f3-ctl` | `d8eb50d` (detached) |
| published binary used for the first re-observation | `C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe` | built from the main working tree |

Both worktrees were created with `git -C C:\Users\ricar\dev\LovelaceSharp worktree add .worktrees/c6-f3 HEAD`
and `... .worktrees/c6-f3-ctl HEAD`. **HEAD moved during the session** (from `141f34e` to `d8eb50d`,
a docs-only commit: `git diff --stat 141f34e d8eb50d -- "*.cs" "*.csproj" "*.slnx"` printed nothing), so
both worktrees were recreated at the current HEAD and the work was re-applied; the whole run below —
failing test, suite, control — is on `d8eb50d` for both trees. Nothing was staged or committed
(`f3-tree-status.txt`: one modified source file, one new test file).

Evidence files written next to this document (`docs/goal-cycle-6/round-14/`):
`f3-published-aot.txt`, `f3-probes-before.txt`, `f3-probes-after.txt`,
`f3-setprecision60-before.txt`, `f3-setprecision60-after.txt`, `f3-control-pristine-head.txt`,
`f3-code.diff`, `f3-full-suite.txt`, `f3-timing-Lovelace.{Suite,Run,Real,Symbolics}.Tests.txt`,
`f3-costly-symbolics.txt`, `f3-tree-status.txt`.

---

## 1. Pre-fix behaviour (requirement 1)

### 1a. The published binary, first re-observation

```powershell
& 'C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe' --eval "print(evalf(sinh(34/3), 30)); print(evalf(sinh(34/3), 100)); print(evalf(sinh(11.5), 30))" --json --omit-functions --omit-variables
```

Observed (`f3-published-aot.txt`, `ok:true`, exit 0):

```json
"output":["41780.548053564065925188446077553412",
          "41780.5480535640659251884460775673391926758349273862253644277306454458985775018289223615326471730513338634",
          "49357.885500315201914739778692889335"]
```

The audit's F3 reproduces exactly, digit for digit.

### 1b. The full pre-fix battery, on a binary built from the pristine HEAD worktree

The published AOT binary is a stale artifact, so every `before`/`after` pair below is produced by two
binaries built from the same commit `d8eb50d`: `.worktrees/c6-f3-ctl` (untouched HEAD) and
`.worktrees/c6-f3` (my change). Script (`f3-probes-{before,after}.txt`), run once per tree:

```powershell
$env:PATH = 'C:\Users\ricar\dev\.lovelace-tools\python;' + $env:PATH

$s = @'
print("N20", evalf(sinh(34/3), 20))
print("N30", evalf(sinh(34/3), 30))
print("N50", evalf(sinh(34/3), 50))
print("N100", evalf(sinh(34/3), 100))
print("ctrl30", evalf(sinh(11.5), 30))
...
'@
$s | & '<tree>\Lovelace.Run\bin\Release\net10.0\Lovelace.Run.exe' --stdin --json --omit-functions --omit-variables
```

**Before (`c6-f3-ctl`, `d8eb50d`)** — verbatim `output` array, only the deciding entries:

| probe | published (before) |
|---|---|
| `evalf(sinh(34/3), 20)`  | `41780.548053564065925049175`  *(21 decimals)* |
| `evalf(sinh(34/3), 30)`  | `41780.548053564065925188446077553412` |
| `evalf(sinh(34/3), 50)`  | `41780.548053564065925188446077567339192675834927386086095` *(51 decimals)* |
| `evalf(sinh(34/3), 100)` | `41780.5480535640659251884460775673391926758349273862253644277306454458985775018289223615326471730513338634` |
| `setprecision(60); evalf(sinh(34/3), 30)` | `41780.548053564065925188446077553412` — **the same wrong value** |
| `setprecision(60); evalf(sinh(34/3), 100)` | `41780.548053564065925188446077567339192675834927386225364427730645` (60 decimals) |
| `setprecision(60); evalf(sinh(28/15), 30)` | `3.156033243073207922905472664338` |
| `setprecision(60); evalf(sinh(65/3), 30)` | `1284351149.9657410942550053116741088452185` |
| `evalf(sinh(11.5), 30)` (terminating control) | `49357.885500315201914739778692889335` — all 30 decimals correct |
| `evalf(sinh(28/15), 30)` (1 integer digit) | `3.156033243073207922905472664338` |
| `evalf(sinh(65/3), 30)` (10 integer digits) | `1284351149.9657410942550053116741088452185` |
| `evalf(sinh(1000/3), 30)` (145 integer digits) | integer part `…1319835464002395751158566363127121981327396…`, **wrong from its 31st digit**, decimals all `9`s from the 28th |

`setprecision(60)` leaves the answer byte-identical (`f3-setprecision60-before.txt` vs
`f3-setprecision60-after.txt`: the before file has `sp60N30 41780.548053564065925188446077553412`,
the plain run has `N30 41780.548053564065925188446077553412`) — so this is **not** the ambient budget.

**After (`.worktrees/c6-f3`, same commit, `f3-setprecision60-after.txt`):** `sp60N30 41780.548053564065925188446077567339`,
`sp60N100 41780.548053564065925188446077567339192675834927386225364427730645`,
`sp60A30 3.156033243073207922905472664339`, `sp60C30 1284351149.965741094255005311674965079318`,
`sp60ctrl30 49357.885500315201914739778692889335` — identical to the plain run, so the repair is a
property of the request and not of the ambient budget either.

---

## 2. The failing test, written first (requirement 2)

New file `Lovelace.Symbolics.Tests/EvalfWorkingPrecisionHonoursRequestedDecimalsTests.cs`. Every expected
string in it is a literal copied from **mpmath 1.3.0 at `mp.dps = 250`**, truncated (not rounded) at the
requested count, computed through the repo's oracle interpreter:

```powershell
$env:PATH = 'C:\Users\ricar\dev\.lovelace-tools\python;' + $env:PATH
$py = @'
from mpmath import mp, mpf, sinh, nstr
mp.dps = 250
for name, x in [("sinh(28/15)", mpf(28)/15), ("sinh(34/3)", mpf(34)/3), ("sinh(65/3)", mpf(65)/3), ("sinh(11.5)", mpf(23)/2)]:
    s = nstr(sinh(x), 160); ip, fp = s.split('.')
    for n in (20, 30, 50, 100): print("t%d" % n, ip + "." + fp[:n])
'@
$py | python -
```

The three magnitudes are the SAME function on three non-terminating rationals: `sinh(28/15)` (1 integer
digit), `sinh(34/3)` (5), `sinh(65/3)` (10), plus `sinh(1000/3)` (145 — the case a fixed guard cannot
cover) and the terminating control `sinh(11.5)`.

**Failing-first transcript.** The test was written before any source change, against the unmodified tree
in `.worktrees/c6-f3`:

```
Failed  RequestedDecimalPlaces_AreTheDigitsDelivered(body: "sinh(28/15)", digits: 30, expected: "3.156033243073207922905472664339")
  Assert.Equal() Failure: Strings differ
  Expected: "3.156033243073207922905472664339"
  Actual:   "3.156033243073207922905472664338"
Failed  RequestedDecimalPlaces_AreTheDigitsDelivered(body: "sinh(34/3)",  digits: 20, expected: "41780.54805356406592518844")
  Actual:   "41780.548053564065925049175"
Failed  RequestedDecimalPlaces_AreTheDigitsDelivered(body: "sinh(34/3)",  digits: 30, expected: "41780.548053564065925188446077567339")
  Actual:   "41780.548053564065925188446077553412"
Failed  RequestedDecimalPlaces_AreTheDigitsDelivered(body: "sinh(34/3)",  digits: 50, …)  Actual: …834927386086095
Failed  RequestedDecimalPlaces_AreTheDigitsDelivered(body: "sinh(34/3)",  digits: 100, …) Actual: …471730513338634
Failed  RequestedDecimalPlaces_AreTheDigitsDelivered(body: "sinh(65/3)",  digits: 30,  …) Actual: …3116741088452185
Failed  RequestedDecimalPlaces_AreTheDigitsDelivered(body: "sinh(28/15)", digits: 100, …) Actual: …69120690464
Failed  RequestedDecimalPlaces_AreTheDigitsDelivered(body: "sinh(65/3)",  digits: 100, …) Actual: …36287417666269
Failed  TerminatingArgument_KeepsDeliveringItsDigits(digits: 100, …)                      Actual: …05117305
Failed!  - Failed: 9, Passed: 11, Skipped: 0, Total: 20
```

(9 failures out of 20 on the 20-case version; after the large-magnitude row was added the same file
fails 10 of 21 — see §6.)

---

## 3. Diagnosis — where the digits are lost

The request is a count of **decimal places**, and the pre-fix body handed it straight to the ambient
budget, which in this engine is also a count of **fractional places**:

* `Lovelace.Symbolics/SymbolicsPlugin.cs:482` (pre-fix): `using (Rl.WithPrecision(digits, Math.Min(digits, 50)))`.
* `Lovelace.Symbolics/SymbolicsPlugin.cs:484` (pre-fix): `Evaluation.EvaluateToNum(f, Context, …)` runs inside that scope, so `Rl.MaxComputationDecimalPlaces == digits` for the whole evaluation.

Everything downstream then bounds itself at `digits` **fractional** places:

* `Lovelace.Symbolics/Evaluation.cs:45-55` — `ToReal(NumRat)` truncates the argument to `Rl.MaxComputationDecimalPlaces` places; `Lovelace.Symbolics/RationalReal.cs:22-27` builds that truncation digit by digit. So the exact rational `34/3` enters the evaluation as `11.333…3` with **absolute error 3.3e-31** at N = 30.
* `Lovelace.Complex/ComplexMath.cs:34-47` — `Exp` computes at `digits + GuardDigits` and returns `TruncateFrac(…, digits)`.
* `Lovelace.Symbolics/Evaluation.cs:347-353` — `Sinh(a) = (exp(a) − 1/exp(a))/2`.

The derivative of `sinh` is `cosh`, which at `34/3` is `41780.55` — five integer digits. The argument
error is amplified by it: **1.39e-26**, i.e. the last five of the thirty published decimals are noise.
The measured rule is exactly *correct decimals = N − k* for a result with `k` integer digits, which the
audit's table recorded and §5 confirms; the "N significant digits" phrasing in the finding is the same
fact seen from the other end.

The diagnosis lands in the **evalf digit-count path** (`SymbolicsPlugin.cs`), not in `Lovelace.Real`:
the arithmetic layers are behaving as documented — they honour the ambient budget they are given. The
one thing that was wrong is the budget `evalf` chose. No file outside `Lovelace.Symbolics/**` and the
two test projects was touched.

---

## 4. The fix

`f3-code.diff` (also reproduced below in full).

```diff
             var f = AsExpr(NumericAtRequestedPrecision(args[0], digits) ?? args[0]);
-            using (Rl.WithPrecision(digits, Math.Min(digits, 50)))
-            {
-                var num = Evaluation.EvaluateToNum(f, Context, new Dictionary<Symbol, Num>());
-                return NumToPayload(num);
-            }
+            // The request is a count of DECIMAL PLACES, so it is NOT the working precision. The
+            // evaluation runs at the request plus the integer digits the value turns out to carry
+            // plus a guard (see EvalfWorkingPrecision), and the published value is bounded back to
+            // the request (see EvalfPayload). Running at the bare request made the last k decimals
+            // of a value with k integer digits noise: an inexact argument is materialised at the
+            // ambient budget, so an argument error of 10^-N is amplified by the function's
+            // derivative into an error of 10^(k-N) in the result.
+            return EvalfPayload(EvalfWorkingPrecision(f, digits), digits);
         },
```

with, at `Lovelace.Symbolics/SymbolicsPlugin.cs`:

* `:1583` `private const int EvalfGuardDigits = 20;` — the same guard `ComplexMath` already reserves internally (`Lovelace.Complex/ComplexMath.cs:19`).
* `:1603` `EvalfWorkingPrecision(f, digits)` — evaluate at `digits + 20`; read the result's integer digits `k`; when `k >= 20` (the case the guard cannot cover) **re-evaluate** at `digits + k + 20`. The second pass is needed because `k` is a property of the *value*, not of the expression — it cannot be known before the value is computed.
* `:1620` `EvaluateAtPlaces(f, places)` — one evaluation inside `Rl.WithPrecision(places, Math.Min(places, 50))`, disposed before the caller reads the value.
* `:1630-1663` `IntegerDigits(Num)` and the per-tier `IntegerDigitsOf(Int | Rl | Rat)` — the integer-digit count. The rational overload is an **upper bound** (numerator width − denominator width + 1), which is the safe direction: over-estimating only adds working room.
* `:1681` `EvalfPayload(n, digits)` — publishes the value truncated back to the requested count. It is the old `NumToPayload` (now removed) with the requested count passed explicitly instead of being read off the ambient budget, plus the bound. `NumInt` stays an Integer payload, `NumRat` keeps `RationalReal.ToReal(r, Math.Min(digits, 1000))` (byte-identical to the old ambient-derived bound), and Real/Complex components go through `BoundDecimalPlaces`.
* `:1702` `BoundDecimalPlaces(value, places)` — returns the value untouched when it already fits, otherwise re-reads the rational it denotes and truncates it; an inexact value is re-marked inexact (`Rl.AsInexact`) so a truncation never crosses as exact. A periodic value has no last place to cut and is left alone.
* `:492` the builtin's descriptor now states the working-precision rule instead of implying that the printed count is all there is.

The dead helper `NumToPayload` was deleted (its two XML-doc references now point at `EvalfPayload`).

---

## 5. Before/after, digit by digit, against mpmath (requirements 2 and 3)

Ground truth: mpmath 1.3.0, `mp.dps = 250`, truncated at the requested place. "first wrong decimal"
is the 1-based fractional position of the first digit that differs from that truncation.

| expression | N | before: decimals published | before: first wrong decimal | after: decimals published | after: first wrong decimal |
|---|---|---|---|---|---|
| `sinh(28/15)` (1 int. digit)  | 30  | 30  | **30** | 30 | all 30 correct |
| `sinh(28/15)`                 | 100 | 100 | **100** | 100 | all 100 correct |
| `sinh(34/3)` (5 int. digits)  | 20  | 21  | **16** | 20 | all 20 correct |
| `sinh(34/3)`                  | 30  | 30  | **26** | 30 | all 30 correct |
| `sinh(34/3)`                  | 50  | 51  | **46** | 50 | all 50 correct |
| `sinh(34/3)`                  | 100 | 100 | **96** | 100 | all 100 correct |
| `sinh(65/3)` (10 int. digits) | 30  | 31  | **22** | 30 | all 30 correct |
| `sinh(65/3)`                  | 100 | 100 | **91** | 100 | all 100 correct |
| `sinh(11.5)` (terminating control, 5 int. digits) | 30  | 30 | all 30 correct | 30 | all 30 correct (unchanged) |
| `sinh(11.5)`                  | 100 | 100 | **100** | 100 | all 100 correct |
| `exp(100)` (44 int. digits, exact argument) | 30 | 30 | **5** | 30 | all 30 correct |

The three magnitudes required by the brief, at N = 30:

```
sinh(28/15)                       (1 integer digit)
  mpmath : 3.1560332430732079229054726643398808098958171050319303402544132472926926925879526189618018339120690466…
  truth  : 3.156033243073207922905472664339
  before : 3.156033243073207922905472664338     <- wrong at decimal 30
  after  : 3.156033243073207922905472664339

sinh(34/3)                        (5 integer digits)
  truth  : 41780.548053564065925188446077567339
  before : 41780.548053564065925188446077553412 <- wrong from decimal 26 (the 5 integer digits)
  after  : 41780.548053564065925188446077567339

sinh(65/3)                        (10 integer digits)
  truth  : 1284351149.965741094255005311674965079318
  before : 1284351149.9657410942550053116741088452185  <- wrong from decimal 22, and 31 decimals
  after  : 1284351149.965741094255005311674965079318
```

Two further cases in the same family, both verified against mpmath:

| expression | N | before | after (mpmath truncation) |
|---|---|---|---|
| `tan(749/600)` | 30 | `2.992890802757492698946721758785` | `2.992890802757492698946721758777` = `2.9928908027574926989467217587777978…` |
| `exp(100)` | 30 | `…73611118.773732453701326647009403646909` | `…73611118.773741922415191608615280287034` |
| `sinh(1000/3)` | 30 | integer part wrong from its 31st digit, decimals `…6199999…` | `2909358940723497999622983496672289621777576895087778819744496982533636420191216688947898956693539789328530352713870924771820635012087408585306867.359401747177724438543006142556` — integer part **and** all 30 decimals match mpmath exactly (second-pass path: k = 145) |

### No regression (requirement 3)

Every one of these is byte-identical before and after (`f3-probes-{before,after}.txt`):

| expression | N | value (before = after) |
|---|---|---|
| `sqrt(2)` | 30 | `1.414213562373095048801688724209` |
| `sqrt(2)` | 50 | `1.41421356237309504880168872420969807856967187537694` |
| `sqrt(2)` | 5  | `1.41421` |
| `sqrt(2)` | 1  | `1.4` |
| `1/3` | 30 | `0.333333333333333333333333333333` |
| `1/3` | 2000 | 1000 decimals (the 1000-place cap), tail `…333333` |
| `pi` | 30 | `3.141592653589793238462643383279` |
| `pi(30)` | 30 | `3.141592653589793238462643383279` |
| `pi(30)` | 40 | `3.141592653589793238462643383279` (still the constant's own 30 digits, not padded) |
| `1/2` | 40 | `0.5`, exact, `numerator 1 / denominator 2` |
| `2` | 30 | `2`, value kind `Integer` |
| `1/(3*10^1000)` | 30 | `0` |
| `1/(3*10^1001)` | 30 | `0` |
| `sin(pi(30))` | 40 | `0` |
| `sqrt(2)` | 2000 | 100 decimals (the already-numeric argument's ambient width — unchanged, and owned by the argument-coercion round) |

The two zeros are asserted **inexact with no numerator/denominator** in the new test
(`ValuesBelowTheComputationCap_StayInexact`): raising the working precision did not turn them into
exact rationals. `sin(pi(30))` and `1/(3*10^1000)` publish `{"kind":"Real","value":"0","exact":false}`
(measured with `--eval` + `result.structured`).

---

## 6. Control — the same test files on a pristine tree (requirement 5)

```powershell
git -C C:\Users\ricar\dev\LovelaceSharp worktree add .worktrees/c6-f3-ctl HEAD
Copy-Item <.worktrees\c6-f3\Lovelace.Symbolics.Tests\EvalfWorkingPrecisionHonoursRequestedDecimalsTests.cs> <.worktrees\c6-f3-ctl\Lovelace.Symbolics.Tests\> -Force
cd C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-f3-ctl
git status --short
# ?? Lovelace.Symbolics.Tests/EvalfWorkingPrecisionHonoursRequestedDecimalsTests.cs      <- ONLY the new test file
git rev-parse HEAD
# d8eb50d84946d7f76b7897be29115f694bc999d8
dotnet test Lovelace.Symbolics.Tests\Lovelace.Symbolics.Tests.csproj --configuration Release --nologo --filter "FullyQualifiedName~EvalfWorkingPrecisionHonoursRequestedDecimalsTests"
```

Observed (`f3-control-pristine-head.txt`), exit code **1**:

```
Failed  RequestedDecimalPlaces_AreTheDigitsDelivered(body: "sinh(28/15)", digits: 30,  expected "3.156033243073207922905472664339")            actual "3.156033243073207922905472664338"
Failed  RequestedDecimalPlaces_AreTheDigitsDelivered(body: "sinh(28/15)", digits: 100, …)  expected …183391206904 66   actual …183391206904 64
Failed  RequestedDecimalPlaces_AreTheDigitsDelivered(body: "sinh(34/3)",  digits: 20,  …)  expected "41780.54805356406592518844"          actual "41780.548053564065925049175"
Failed  RequestedDecimalPlaces_AreTheDigitsDelivered(body: "sinh(34/3)",  digits: 30,  …)  expected …446077567339                          actual …446077553412
Failed  RequestedDecimalPlaces_AreTheDigitsDelivered(body: "sinh(34/3)",  digits: 50,  …)  expected …492738622536                            actual …4927386086095
Failed  RequestedDecimalPlaces_AreTheDigitsDelivered(body: "sinh(34/3)",  digits: 100, …)  expected …471730513352561                         actual …471730513338634
Failed  RequestedDecimalPlaces_AreTheDigitsDelivered(body: "sinh(65/3)",  digits: 30,  …)  expected "1284351149.965741094255005311674965079318"  actual "1284351149.9657410942550053116741088452185"
Failed  RequestedDecimalPlaces_AreTheDigitsDelivered(body: "sinh(65/3)",  digits: 100, …)  expected …3036288273900369                       actual …3036287417666269
Failed  RequestedDecimalPlaces_AreTheDigitsDelivered(body: "sinh(1000/3)", digits: 30, …)  expected "29093589407234979996229834966722896217775768950877…"  actual "29093589407234979996229834966713198354640023957511…"
Failed  TerminatingArgument_KeepsDeliveringItsDigits(digits: 100, …)                        expected …48405117304                              actual …48405117305
Failed!  - Failed: 10, Passed: 11, Skipped: 0, Total: 21, Duration: 1 s - Lovelace.Symbolics.Tests.dll (net10.0)
```

**The control fails exactly as required, and only the new test file is present in that tree.** The 11
passing cases are the controls inside the file that were already correct before the repair.

---

## 7. Suite results (requirement 4)

```powershell
cd C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-f3
dotnet test LovelaceSharp.slnx --configuration Release --nologo
```

Exit code **0**. Per project (`f3-full-suite.txt`):

| project | failed | passed | total |
|---|---|---|---|
| Lovelace.Abstractions.Tests | 0 | 20 | 20 |
| Lovelace.Array.Tests | 0 | 19 | 19 |
| Lovelace.Complex.Tests | 0 | 115 | 115 |
| Lovelace.Console.Tests | 0 | 15 | 15 |
| Lovelace.Dsp.Tests | 0 | 61 | 61 |
| Lovelace.Integer.Tests | 0 | 148 | 148 |
| Lovelace.Knowledge.Tests | 0 | 28 | 28 |
| Lovelace.Natural.Tests | 0 | 195 | 195 |
| Lovelace.Real.Tests | 0 | 2495 | 2495 |
| Lovelace.Representation.Tests | 0 | 91 | 91 |
| Lovelace.Run.Tests | 0 | 206 | 206 |
| Lovelace.Studio.Tests | 0 | 22 | 22 |
| Lovelace.Suite.Tests | 0 | 825 | 825 |
| Lovelace.Symbolics.Tests | 0 | 1148 | 1148 |
| precbench.Tests | 0 | 13 | 13 |
| **total** | **0** | **5401** | **5401** |

Filtered subsets, all exit code 0:

| subset | file | failed / passed / total |
|---|---|---|
| `Lovelace.Suite.Tests` `Category=Timing` | `f3-timing-Lovelace.Suite.Tests.txt` | 0 / 8 / 8 |
| `Lovelace.Run.Tests` `Category=Timing` | `f3-timing-Lovelace.Run.Tests.txt` | 0 / 5 / 5 |
| `Lovelace.Real.Tests` `Category=Timing` | `f3-timing-Lovelace.Real.Tests.txt` | 0 / 1 / 1 |
| `Lovelace.Symbolics.Tests` `Category=Timing` | `f3-timing-Lovelace.Symbolics.Tests.txt` | 0 / 1 / 1 |
| `Lovelace.Symbolics.Tests` `Category=Costly` | `f3-costly-symbolics.txt` | 0 / 32 / 32 |

No assertion anywhere was weakened, deleted, skipped or loosened; no tolerance was widened; no file
outside `Lovelace.Symbolics/**` and `Lovelace.Symbolics.Tests/**` was modified (the only other writes
are this document and the evidence files beside it); nothing was `git add`ed, committed or pushed.

---

## 8. Could not verify

1. **Cost and outcome for astronomically large magnitudes.** The second pass computes at
   `N + k + 20` places, and `k` grows with the value, so the work grows with the integer part. I
   measured `k = 145` (`sinh(1000/3)` at N = 30 → 195 places, 352 ms) and `k = 10` and `k = 5`;
   I did **not** measure `k` in the thousands (e.g. `evalf(sinh(10^6), 30)`), so I cannot state its
   cost or that it terminates in a useful time. There is no ceiling on the working precision; a cap
   would reintroduce the defect for values above it, so none was added.
2. **Cancellation-dominated expressions.** The guard is sized from the result's integer digits, i.e.
   from `|f'| ~ 10^k`. A function whose derivative outruns its value — `tan` near a pole, where
   `|f'| ~ f^2` and the needed room is `2k` rather than `k` — is therefore still under-guarded in
   principle. I did not construct a failing case for it: my `tan(749/600)` probe has `k = 1` and
   `|f'| ≈ 10` and is correct after the fix. This is an untested residual, not a demonstrated one.
3. **Rounding.** `evalf` truncates and always has (`ComplexMath.TruncateFrac`, `Real.ToString`); the
   fix keeps that. The *correctly rounded* N-place value is **not** what this tree publishes, and I
   did not change it — the new test pins the truncation. If the intended contract is rounding, that
   is a separate decision.
4. **Digits beyond the 1000-place cap.** `EvalfPayload` bounds the published count at
   `Math.Min(digits, 1000)`, the pre-existing cap the old `NumToPayload` also applied, so
   `evalf(1/3, 2000)` still publishes 1000 decimals (measured identical before and after). Whether
   that cap itself is right is not this finding.
5. **`setprecision` interaction** was measured only at 60, for N ∈ {30, 100}, on three expressions.
6. **The CI workflow and the instrumented coverage loop** were not run (out of scope); the runs above
   are plain `dotnet test` invocations on this machine (Windows, PowerShell 5.1).
7. **Concurrency.** Both worktrees are at `d8eb50d`; the main working tree's HEAD advanced once
   during the session (`141f34e` → `d8eb50d`). I verified the two commits differ only in `docs/`,
   but if HEAD moves again the revisions above are the ones the numbers belong to.
