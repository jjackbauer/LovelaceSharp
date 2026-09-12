# F3 (rebase) — `evalf(f, N)` publishes N decimal places but computed at N significant digits

**Round 14 / cycle 6 / implementer — RE-APPLIED ON CURRENT MAIN.** Status: **fixed, tested, control-confirmed on `d9f2a7c`.**

This is the same change as `f3-implementation.md` (which was made against `d8eb50d` and does not
apply to `9a671c0`). The diagnosis is unchanged and is not repeated here beyond §2; this document
records the re-application and the re-run evidence.

## 0. Trees, revisions, artefacts

| Role | Path | Revision |
|---|---|---|
| implementation | `C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-f3b` | `d9f2a7c` (detached) |
| control (pristine) | `C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-f3b-ctl` | `d9f2a7c` (detached) |

Both created with `git -C C:\Users\ricar\dev\LovelaceSharp worktree add .worktrees/c6-f3b d9f2a7c`
(and `... c6-f3b-ctl d9f2a7c`). `f3b-revision.txt` records the exact revision.
Nothing is staged or committed — `f3b-tree-status.txt`:

```
 M Lovelace.Symbolics/SymbolicsPlugin.cs
?? Lovelace.Symbolics.Tests/EvalfWorkingPrecisionHonoursRequestedDecimalsTests.cs
```

### Did it re-apply cleanly?

The patch could not be applied mechanically (the F1/F2 hunks moved the surrounding context), so it was
**re-applied by hand** onto `d9f2a7c`. To prove the re-application is the *same* change and not a new
one, the `+`/`-` lines of the new diff were compared with the `+`/`-` lines of the `d8eb50d` patch:

```powershell
$old = Get-Content docs\goal-cycle-6\round-14\f3-code.diff    -Encoding utf8 | Where-Object { $_ -match '^[+-]' -and $_ -notmatch '^[+-]{3}' }
$new = Get-Content docs\goal-cycle-6\round-14\f3b-code.diff   -Encoding utf8 | Where-Object { $_ -match '^[+-]' -and $_ -notmatch '^[+-]{3}' }
Compare-Object $old $new
```

Observed: **no output** — `CHANGED LINES IDENTICAL to the d8eb50d patch` (165 changed lines each).
Only the context lines differ, because F1/F2 changed the surrounding code.

`git -C .worktrees\c6-f3b diff --stat` is `1 file changed, 150 insertions(+), 15 deletions(-)`, all in
`Lovelace.Symbolics/SymbolicsPlugin.cs`, with four hunks: `@@ -486,14 +486,17 @@` (the evalf body),
`@@ -1577,7 +1580,7 @@` and `@@ -1601,7 +1604,7 @@` (the two XML-doc references) and
`@@ -1611,16 +1614,148 @@` (the new helpers, replacing `NumToPayload`).

F1 and F2 are intact: the only source file touched is `Lovelace.Symbolics/SymbolicsPlugin.cs`, and the
only region removed by the patch is the now-unused `NumToPayload` helper (with its two XML-doc
references repointed at `EvalfPayload`). `AsTextName`, `AsRelation`, `LimitRecord`, the rewritten
`limit`/`limit_left`/`limit_right` registrations and the argument-shape guards are untouched — my diff
adds evalf helpers and removes nothing of theirs. `f3b-code.diff` is the whole source change.

---

## 1. Failing first, on a PRISTINE `d9f2a7c` tree

Only the new test file was copied into the control tree:

```powershell
Copy-Item .worktrees\c6-f3b\Lovelace.Symbolics.Tests\EvalfWorkingPrecisionHonoursRequestedDecimalsTests.cs .worktrees\c6-f3b-ctl\Lovelace.Symbolics.Tests\ -Force
cd .worktrees\c6-f3b-ctl
git status --short      # ?? Lovelace.Symbolics.Tests/EvalfWorkingPrecisionHonoursRequestedDecimalsTests.cs
git rev-parse HEAD      # d9f2a7cdf4fe72ea84d1effef84539f6e319dce8
dotnet test Lovelace.Symbolics.Tests\Lovelace.Symbolics.Tests.csproj --configuration Release --nologo --filter "FullyQualifiedName~EvalfWorkingPrecisionHonoursRequestedDecimalsTests"
```

Observed (`f3b-control-pristine-head.txt`), exit code **1**:

```
Failed  RequestedDecimalPlaces_AreTheDigitsDelivered(body: "sinh(28/15)",  digits: 30,  expected "3.156033243073207922905472664339")  actual "3.156033243073207922905472664338"
Failed  RequestedDecimalPlaces_AreTheDigitsDelivered(body: "sinh(28/15)",  digits: 100, …)  expected …183391206904 66   actual …183391206904 64
Failed  RequestedDecimalPlaces_AreTheDigitsDelivered(body: "sinh(34/3)",   digits: 20,  …)  expected "41780.54805356406592518844"       actual "41780.548053564065925049175"
Failed  RequestedDecimalPlaces_AreTheDigitsDelivered(body: "sinh(34/3)",   digits: 30,  …)  expected …446077567339                       actual …446077553412
Failed  RequestedDecimalPlaces_AreTheDigitsDelivered(body: "sinh(34/3)",   digits: 50,  …)  expected …492738622536                         actual …4927386086095
Failed  RequestedDecimalPlaces_AreTheDigitsDelivered(body: "sinh(34/3)",   digits: 100, …)  expected …471730513352561                      actual …471730513338634
Failed  RequestedDecimalPlaces_AreTheDigitsDelivered(body: "sinh(65/3)",   digits: 30,  …)  expected "1284351149.965741094255005311674965079318"  actual "1284351149.9657410942550053116741088452185"
Failed  RequestedDecimalPlaces_AreTheDigitsDelivered(body: "sinh(65/3)",   digits: 100, …)  expected …3036288273900369                    actual …3036287417666269
Failed  RequestedDecimalPlaces_AreTheDigitsDelivered(body: "sinh(1000/3)", digits: 30,  …)  expected "29093589407234979996229834966722896217775768950877…"  actual "29093589407234979996229834966713198354640023957511…"
Failed  TerminatingArgument_KeepsDeliveringItsDigits(digits: 100, …)                         expected …48405117304                           actual …48405117305
Failed!  - Failed: 10, Passed: 11, Skipped: 0, Total: 21
```

The same test file on the fixed tree (`f3b-focused-test.txt`): `Passed! - Failed: 0, Passed: 21`.

---

## 2. The change (unchanged from `f3-implementation.md`)

`Lovelace.Symbolics/SymbolicsPlugin.cs`, evalf only:

* `EvalfGuardDigits = 20` — the requested decimal count plus the integer digits of the result plus a
  guard is the working precision.
* `EvalfWorkingPrecision` — evaluate at `N + 20`; read the result's integer digits `k`; when
  `k >= 20` (the case the guard cannot cover) re-evaluate at `N + k + 20`.
* `EvaluateAtPlaces` — one evaluation inside `Rl.WithPrecision(places, Math.Min(places, 50))`.
* `IntegerDigits` / `IntegerDigitsOf(Int | Rl | Rat)` — the magnitude read that sizes the room.
* `EvalfPayload` + `BoundDecimalPlaces` — publish the value truncated back to exactly `N` places,
  provenance preserved (`IsExact`/`AsInexact`), integers and exact rationals neither padded nor flattened.
* the descriptor now states the rule; the dead `NumToPayload` was removed.

Reason in one line: the pre-fix body handed the decimal count to a budget that counts FRACTIONAL
places, and the exact rational argument is materialised truncated at that budget, so an argument error
of `10^-N` is amplified by the derivative (`sinh` → `cosh ≈ 10^k`) into `10^(k-N)` in the result.

---

## 3. After table (requirement 3)

Ground truth: mpmath 1.3.0, `mp.dps = 260–300`, truncated at the requested place (`f3b-mpmath.txt`).
"first wrong decimal" = 1-based fractional position of the first digit differing from that truncation
(`f3b-probes-before.txt`, `f3b-probes-after.txt`).

| probe | N | before: decimals | before: first wrong | after: decimals | after: first wrong |
|---|---|---|---|---|---|
| `evalf(sinh(34/3), N)` | 20  | 21  | **16** | 20  | all 20 correct |
| `evalf(sinh(34/3), N)` | 30  | 30  | **26** | 30  | all 30 correct |
| `evalf(sinh(34/3), N)` | 50  | 51  | **46** | 50  | all 50 correct |
| `evalf(sinh(34/3), N)` | 100 | 100 | **96** | 100 | all 100 correct |
| `evalf(sinh(28/15), 30)` (1 int. digit)  | 30  | 30  | **30** | 30 | all correct |
| `evalf(sinh(28/15), 100)`                | 100 | 100 | **100** | 100 | all correct |
| `evalf(sinh(65/3), 30)` (10 int. digits) | 30  | 31  | **22** | 30 | all correct |
| `evalf(sinh(65/3), 100)`                 | 100 | 100 | **91** | 100 | all correct |
| `evalf(sinh(11.5), 30)` (terminating control) | 30 | 30 | all correct | 30 | all correct |
| `evalf(sinh(11.5), 100)`                 | 100 | 100 | **100** | 100 | all correct |
| `evalf(exp(100), 30)`                     | 30  | 30  | **5**  | 30 | all correct |
| `evalf(tan(749/600), 30)`                 | 30  | 30  | **29** | 30 | all correct |

`evalf(sinh(34/3), N)`, digit for digit against mpmath:

```
N = 20
  mpmath : 41780.54805356406592518844
  before : 41780.548053564065925049175              (21 decimals, wrong from 16)
  after  : 41780.54805356406592518844
N = 30
  mpmath : 41780.548053564065925188446077567339
  before : 41780.548053564065925188446077553412     (wrong from 26 = the 5 integer digits)
  after  : 41780.548053564065925188446077567339
N = 50
  mpmath : 41780.54805356406592518844607756733919267583492738622536
  before : 41780.548053564065925188446077567339192675834927386086095   (51 decimals, wrong from 46)
  after  : 41780.54805356406592518844607756733919267583492738622536
N = 100
  mpmath : 41780.5480535640659251884460775673391926758349273862253644277306454458985775018289223615326471730513352561
  before : 41780.5480535640659251884460775673391926758349273862253644277306454458985775018289223615326471730513338634
  after  : 41780.5480535640659251884460775673391926758349273862253644277306454458985775018289223615326471730513352561
```

The `sinh(1000/3)` case (145 integer digits — the second-pass path, `k = 145 >= 20`):

```
mpmath : 2909358940723497999622983496672289621777576895087778819744496982533636420191216688947898956693539789328530352713870924771820635012087408585306867.359401747177724438543006142556
before : 2909358940723497999622983496671319835464002395751158566363127121981327396456541016218649959698835480742697564957936934509801097305704256209222731.7923087730064255542804717216199999999999999999999999999999999999999999999999999999999999999999999999
after  : 2909358940723497999622983496672289621777576895087778819744496982533636420191216688947898956693539789328530352713870924771820635012087408585306867.359401747177724438543006142556
```

Before the repair **not one of the 145 integer digits was right either** (the integer part diverges at
its 31st digit); after it the integer part *and* all 30 decimals match mpmath exactly. This is the case
a fixed guard cannot cover, which is why the working precision is re-sized from the value.

### No regression — byte-identical before and after

`sqrt(2)` at 30 / 50 / 5 / 1, `1/3` at 30 and at 2000 (1000-place cap), `pi` at 30, `pi(30)` at 30 and
40, `1/2` at 40 (exact `1/2`), `2` at 30 (value kind `Integer`), `1/(3*10^1000)` and
`1/(3*10^1001)` at 30 (both `0`, **inexact**, no numerator/denominator), `sin(pi(30))` at 40 (`0`,
inexact) — the comparison script printed `SAME` for every one of these probes.

`setprecision(60)` (`f3b-setprecision60-{before,after}.txt`): before `sp60N30` is the same wrong
`…446077553412` (so the defect was not the ambient budget); after it is `…446077567339`, identical to
the plain run.

---

## 4. Suite results (requirement 4)

```powershell
cd C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-f3b
dotnet test LovelaceSharp.slnx --configuration Release --nologo
```

Exit code **0** (`f3b-full-suite.txt`):

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
| Lovelace.Run.Tests | 0 | 247 | 247 |
| Lovelace.Studio.Tests | 0 | 22 | 22 |
| Lovelace.Suite.Tests | 0 | 828 | 828 |
| Lovelace.Symbolics.Tests | 0 | 1148 | 1148 |
| precbench.Tests | 0 | 13 | 13 |
| **total** | **0** | **5445** | **5445** |

Filtered subsets, all exit code 0:

| subset | file | failed / passed / total |
|---|---|---|
| `Lovelace.Suite.Tests` `Category=Timing` | `f3b-timing-Lovelace.Suite.Tests.txt` | 0 / 8 / 8 |
| `Lovelace.Run.Tests` `Category=Timing` | `f3b-timing-Lovelace.Run.Tests.txt` | 0 / 5 / 5 |
| `Lovelace.Real.Tests` `Category=Timing` | `f3b-timing-Lovelace.Real.Tests.txt` | 0 / 1 / 1 |
| `Lovelace.Symbolics.Tests` `Category=Timing` | `f3b-timing-Lovelace.Symbolics.Tests.txt` | 0 / 1 / 1 |
| `Lovelace.Symbolics.Tests` `Category=Costly` | `f3b-costly-symbolics.txt` | 0 / 32 / 32 |

No assertion was weakened, deleted, skipped or loosened and no tolerance was widened. No file outside
`Lovelace.Symbolics/**` and `Lovelace.Symbolics.Tests/**` was modified; nothing was `git add`ed,
committed or pushed.

---

## 5. Could not verify

1. **Cost/outcome for integer parts in the thousands.** No ceiling was added to the working precision
   (a ceiling would reintroduce the defect above it). `k = 145` was measured (360 ms for
   `sinh(1000/3)` at N = 30 → 195 places); `k` in the thousands was not measured.
2. **Cancellation-dominated expressions**, where `|f'|` outruns the value (`tan` near a pole needs
   roughly `2k` of room). No failing case was constructed; `tan(749/600)` (`k = 1`, `|f'| ≈ 10`) is
   correct after the fix. Untested residual, not a demonstrated one.
3. **Rounding.** `evalf` truncates and still truncates; the correctly-rounded N-place value is not
   what this tree publishes. The test pins the truncation.
4. **Digits beyond the existing 1000-place publish cap** — `evalf(1/3, 2000)` still publishes 1000
   decimals, identical before and after.
5. **`setprecision`** was measured only at 60, for N ∈ {30, 100}.
6. **The CI workflow** was not run; the runs above are plain `dotnet test` on this machine
   (Windows, PowerShell 5.1). F1/F2's own tests (argument-shape sweep, limit short forms) pass in the
   same run, which is the only statement I make about them.
