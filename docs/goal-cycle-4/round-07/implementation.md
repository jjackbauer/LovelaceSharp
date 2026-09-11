# N23 — `Real.Cos` at the default precision: measured bottleneck and fix

**Verdict.** `cos(1/2)` at the project's default precision (1000 places) went from
**25,014 ms to 914 ms for a fresh process's first call** and from **23,724 ms to 379 ms** once the
cached π is warm (63×), with **every stored digit byte-identical** — a 4,519,306-byte before/after
dump of `sin`/`cos`/`tan` over five arguments at four precisions matches on SHA-256.
The cost was never the number of series terms (204 terms at 1000 places, and 204 at every higher
precision too); it was **decimal rendering inside the arithmetic**: every multiply, divide and add
rendered a magnitude to a decimal string and parsed it back, and the series' magnitudes reach
**204,270 digits** for an answer that needs 1,010.

---

## 1. Where the time actually goes (measured, not guessed)

Probe: `Lovelace.Real.Tests/RealTrigCostProbeTests.cs` (Heavy category), run at 1000 places with the
π cache warm. Raw output is in `docs/goal-cycle-4/round-07/probe-before/` (pre-fix) and
`probe-after/` (post-fix); the same file ran against both trees.

### 1.1 Phase breakdown — `Real.Cos(new Real("0.5"))`, 1000 places, `Real.WithPrecision(1000, 1000)`

| phase | before | after |
|---|---:|---:|
| `Pi` (cached) | 0.0 ms | 0.0 ms |
| `twoPi = pi*2` | 0.1 ms | 0.0 ms |
| `halfPi = pi/2` (period-detecting `Divide`) | 16.8 ms | 19.1 ms *(probe still times the old expression)* |
| `halfPi` through `HalfOf` — **the path `Sin`/`Cos` now use** | n/a | **0.1 ms** |
| `ReduceToTwoPi(0.5)` (no-op branch) | 0.1 ms | 0.1 ms |
| `ReduceToTwoPi(100)` (real reduction) | 31.3 ms | **0.3 ms** |
| `ReduceToTwoPi(1/3)` (no-op branch) | 0.0 ms | 0.0 ms |
| `TrySpecialAngle(0.5)` — misses, so all 16 rows | 271.8 ms | **0.1 ms** |
| `TrySpecialAngle(pi/6)` — hits at row 1 | 16.1 ms | **0.3 ms** |
| `CosTaylor(0.5, 1010)` | 23,360.9 ms | **183.8 ms** |
| `SinTaylor(0.5, 1010)` | 23,210.3 ms | **183.0 ms** |
| **`Real.Cos(0.5)` public total** | **23,982.0 ms** | **200.8 ms** |
| stored result | `exp=-204270`, 204,270 digits | `exp=-204270`, 204,270 digits (unchanged) |

The 16 special-angle rows on their own (`pi*num/den` through the public `Divide`, as the old table
built them): **275.2 ms** per call, spread over 15 divisions of ~16–28 ms each. The table was a real
cost, but it is not the 24 seconds; **97 % of the call was the Taylor series.**

### 1.2 Inside the series (204 iterations, 1000 places)

Probe: `probe-before/series-internals.txt`, `probe-after/series-internals.txt`. Every operation of
the loop is timed individually.

| operation | before | after |
|---|---:|---:|
| `-term * x2` (`Multiply` → `Normalize`) | 6,441.3 ms | 44.8 ms |
| `DivideNonPeriodic` total | 7,862.5 ms | 66.0 ms |
| &nbsp;&nbsp;· `ShiftLeftDecimal` (`numerator * 10^guard`) | 1,203.3 ms | 36.4 ms |
| &nbsp;&nbsp;· `Nat.DivRem` (short division) | 8.7 ms | 9.1 ms |
| &nbsp;&nbsp;· `Normalize` | **6,650.4 ms** | **20.6 ms** |
| `sum + term` (`Add` → `ToInteger` alignment) | **9,148.7 ms** | **53.9 ms** |
| `Abs(term) < threshold` (old test) | 0.2 ms | 6,569.6 ms *(only if the old test is kept — the library no longer runs it)* |
| `IsBelowDecimalGuard` (new test) | n/a | 129.5 ms over 204 terms |
| loop total as replicated | 23,452.8 ms | 6,829.0 ms *(dominated by the old comparison, which the library no longer performs)* |

**The bottleneck, stated with numbers.** Genuine integer arithmetic on 204,270-digit magnitudes
costs ~165 ms for the whole series (multiply 45 + divide 66 + add 54). The same loop cost 23.5 s
because three operations each rendered a ~200k-digit magnitude to decimal and parsed it back:

* `Normalize` (called by `Multiply`, `DivideNonPeriodic`, `Add` and `Negate`) called
  `ToNatural().ToString()` **unconditionally**, then `Nat.TryParse` on the stripped result:
  6,650 ms in `DivideNonPeriodic` alone, 6,441 ms more inside `Multiply`.
* `ToInteger` (called by `Add` to align exponents) rendered the digits, appended the zero padding as
  a `string`, and reparsed: 9,149 ms.
* The old `Abs(term) < threshold` looked free (0.2 ms) *only because* `Normalize` had already
  rendered the same magnitude one line earlier; with the rendering removed it would itself have cost
  6,570 ms, which is why the termination test had to become integer work too.

The magnitudes are that large because of the series' exponent bookkeeping:
`DivideNonPeriodic(..., guard)` sets the result exponent to `-guard + (product.Exponent - den.Exponent)`,
so each iteration pushes the term's exponent down by `guard + 2` while its magnitude grows by the
same ~1,012 digits. 204 terms ⇒ a 204,270-digit sum. That growth is the *value* of the result (the
stored real genuinely has 204,270 significant places), so it cannot be compacted without changing
the result — see §5.

### 1.3 Cost vs digit count, before the fix

`probe-before/timing-sweep.txt` (in-process, π warm) and `probe-before/single-call-before.txt`
(one fresh process per precision; `first` includes π, `warm` is the same call again):

| places | sweep | first call (fresh process) | warm call |
|---:|---:|---:|---:|
| 20 | 0.6 ms | 12.8 ms | 2.1 ms |
| 80 | 7.8 ms | 86.3 ms | 58.3 ms |
| 200 | 83.2 ms | 546.1 ms | 124.1 ms |
| 1000 | 23,798.9 ms | 25,014.4 ms | 23,723.6 ms |

An earlier run of the same probe gave 13.9 / 90.5 / 313.2 / 25,398.1 ms, so run-to-run variance on
this machine is up to ~4× on the small cases and ~20 % at 1000 places. For reference, the task
statement quoted 44,363 ms / 219 ms / 174 ms / 1 ms; my reproduction lands at the same order of
magnitude but not at the same numbers, so the before/after table below uses **my** measured pairs
(both taken on this machine, same methodology, one fresh process per data point).

---

## 2. The fix

All changes are in `Lovelace.Real/Real.cs`. Every one of them is value-preserving by construction
(no precision cap, no reordering of any sum, no change to any truncation schedule), which is what
lets the stored results stay bit-identical.

| # | line (post-fix) | change |
|---|---|---|
| 1 | 2120 `Normalize` (+ 2149 `StripTrailingDecimalZeros`, 2135/2138 `s_tenTo18`/`s_ten`) | strips trailing decimal zeros with integer division (one division by 10^18 decides 18 digits, two short divisions in the common no-zero case) instead of rendering the digits and parsing them back. Same magnitude, same exponent, same value. |
| 2 | 2039 `ToInteger` | exponent alignment now shifts the binary magnitude (`Nat.ShiftLeftDecimal`) instead of `digits + new string('0', zeros)` + reparse. Same integer. |
| 3 | 1490 `IsBelowDecimalGuard` (used by 1435 `SinTaylor`, 1456 `CosTaylor`) | the termination test `Abs(term) < 10^-(guard+1)` becomes `magnitude < 10^k`, `k = -(guard+1) - Exponent`, one `Nat` comparison; the power of ten is carried forward between terms (k advances monotonically), so each term costs one short multiply. Exactly the same inequality, including the boundary (`"0." + guard zeros + "1"` is 10^-(guard+1), not 10^-guard — an off-by-one the new boundary test caught while the implementation was being written). |
| 4 | 1359 `HalfOf` (used at 1262, 1293) | `pi / 2` for a value stored at the active precision is one integer halving at the operand's own scale — identical to what `Divide` produced, without its remainder-tracking decimal loop (16.8 ms → 0.1 ms per `Sin`/`Cos` call). |
| 5 | 1332 `WholeQuotient` (used by 1310 `ReduceToTwoPi`) | the reduction's integer quotient is one `Nat.DivRem`; `Truncate(Divide(value, twoPi))` equals the integer part of the exact quotient (nested floor), so the reduced angle is unchanged (31.3 ms → 0.3 ms). Periodic operands keep the original path. |
| 6 | 1380 `TrySpecialAngle` | each candidate angle is `floor(π's magnitude · num / den)` at π's scale — the same value `pi*num/den` produced, because `Divide` aligns the operands to a single exponent — built with one `Nat` division instead of a period-detecting `Divide` (16–28 ms per row). A non-periodic `x` that stores nothing as far down as the angle's last place is rejected before the comparison (the angle is normalised, so its last stored digit is non-zero). |

Retained exactly as the previous round left them: the `Sin(-x) => -Sin(x, …)` / `Cos(-x) => Cos(x, …)`
symmetry guards, the special-angle table's rows and exact values, and every public signature.

Helper added for #3: `TenToThe(long)` (binary exponentiation on `Nat`, no decimal round trip).

---

## 3. Before / after timings

### 3.1 Fresh process, one precision per run (`single-call-before.txt` / `single-call-after.txt`)

| places | before: first call (incl. π) | after: first call (incl. π) | before: warm call | after: warm call | warm speed-up |
|---:|---:|---:|---:|---:|---:|
| 20 | 12.8 ms | 10.5 ms | 2.1 ms | **0.3 ms** | 7.0× |
| 80 | 86.3 ms | 24.0 ms | 58.3 ms | **4.4 ms** | 13.3× |
| 200 | 546.1 ms | 87.8 ms | 124.1 ms | **24.8 ms** | 5.0× |
| 1000 | 25,014.4 ms | **913.6 ms** | 23,723.6 ms | **379.4 ms** | **62.5×** |

The first call also pays the cold `Real.Pi` (Chudnovsky BSP + √10005, computed once per process and
cached): ~10 ms at 20 places, ~20 ms at 80, ~63 ms at 200 and ~534 ms at 1000 (inferred as
first − warm in the same run). The trig work itself is the warm column. In-process warm calls
(sweep, same process, JIT settled) go much lower:

### 3.2 In-process sweep (π warm)

| places | before | after | speed-up |
|---:|---:|---:|---:|
| 20 | 0.6 ms | 0.2 ms | 3.0× |
| 80 | 7.8 ms | 0.3 ms | 26× |
| 200 | 83.2 ms | 2.9 ms | 29× |
| 1000 | 23,798.9 ms | 389.0 ms | **61×** |

### 3.3 Where the remaining 200 ms at 1000 places goes

`CosTaylor` 183.8 ms = integer arithmetic over the 204,270-digit representation (multiply 45 ms,
divide 66 ms of which `Normalize` 21 ms, add 54 ms) + the carried-power termination test 129.5 ms,
plus ~10 ms of glue. That is the algorithm's own cost at that representation size; the decimal
rendering that used to surround it is gone.

---

## 4. Digit identity

**Strongest evidence — full stored representation, byte-for-byte.** `RealTrigDigitDumpTests`
(Heavy) prints, for `sin`, `cos` and the tangent (`DivideNonPeriodic(sin, cos, p+10)` — `Real`
exposes no `Tan`) at arguments `1/2`, `1`, `-1/3` (exact periodic `-1/3`), `Pi/6` and `100`, at
precisions `20`, `80`, `200`, `1000`:

* the sign, `Exponent`, `PeriodStart`/`PeriodLength`,
* the **FULL stored magnitude string** (`ToNatural().ToString()`, e.g. all 204,270 digits at 1000 places),
* the rendered `ToString()` at display = computation places.

| artifact | bytes | SHA-256 |
|---|---:|---|
| `probe-before/digits-dump.txt` | 4,519,306 | `57D36319794C186B56CF4922671F5AA0EF37225BA38C70DD97400762BC5B3375` |
| `probe-after/digits-dump.txt` | 4,519,306 | `57D36319794C186B56CF4922671F5AA0EF37225BA38C70DD97400762BC5B3375` |

Identical: not one digit, exponent or period flag moved — including the 204,270-digit stored values,
which is a stricter claim than the rendered digits the task asked for. The dump run also got 8×
faster (8 m 47 s → 1 m 05 s for the same 60 results plus π).

**Equivalence tests** (`Lovelace.Real.Tests/RealTrigFastPathTests.cs`, 9 tests, all in the default
suite) pin each fast path against the expression it replaced:

* `SpecialAngles_BuiltTheOriginalWay_StillHitTheTableExactly` — all 16 rows at 20/80/200 places,
  built as `pi * num / den` through the public divide, still hit the table with their exact values.
* `NonSpecialAngles_AreNotCapturedByTheTable` — including each special angle minus one unit in the
  last place, so neither the cheap angle nor the pre-reject can manufacture or lose a match.
* `ArgumentReduction_IntegerQuotient_MatchesTheDividedForm` — `WholeQuotient` vs
  `Truncate(value / twoPi)` and the full reduction, 10 arguments × 3 precisions.
* `SeriesTermination_IntegerTest_MatchesTheThresholdComparison` and
  `SeriesTermination_IntegerTest_TracksTheCarriedPowerAcrossManyTerms` — the integer test against
  `Abs(term) < threshold` exactly on, just under and just over the boundary, both signs, and over a
  full 204-term run whose final sum is compared with the private `CosTaylor`.
* `Normalize_BinaryStrip_MatchesTheDigitStringStrip`, `ToInteger_BinaryShift_MatchesTheDigitPadding`,
  `HalfOf_MatchesTheDividedForm_ForPiShapedValues` — the three arithmetic-core changes against their
  digit-string references.
* `Cos_AtDefaultPrecision_StaysInteractive` — a loose (10 s) guard against the per-term rendering
  coming back.

---

## 5. What I could not improve (and why it is not shipped)

1. **The series' 200× representation growth (the remaining ~180 ms).** At 1000 places the algorithm
   stores 204,270 digits to deliver 1,010 correct ones, because `DivideNonPeriodic`'s
   `exponentAdjustment` makes each term's exponent fall by `guard + 2` per iteration. Computing the
   series at a fixed scale, or by binary splitting the way `Pi` does, would make the whole series
   ~milliseconds — but both change the truncation schedule, so the *stored value* would change in
   its low-order places (the first ~1007 digits, i.e. every rendered digit, would still agree).
   The mandate forbids changing any digit; I kept the value bit-identical and report the cost
   instead. This is the one real lever left.
2. **`Divide`'s period-detecting decimal loop.** It is still ~16–28 ms per irrational division at
   1000 places (string-keyed remainder history, one rendering per fractional place). It is off the
   trig path now (every division `Sin`/`Cos` performs has an exact integer equivalent), but user code
   writing `pi / 6` still pays it. Rewriting it means touching period detection, whose exact
   periodic results other tests pin — outside the N23 blast radius and not needed for the goal.
3. **Uncached decimal rendering of a 200k-digit magnitude: ~90 ms** (`Nat.ToStringRecursive` spawns a
   `Task.Run` per recursion node). `Lovelace.Natural` is out of scope, so the fix was to stop calling
   it in the arithmetic rather than to make it faster. (For scale: the old code paid ~33 ms of
   render+reparse per series term, 204 times.)
4. **The termination test's carried power costs 129.5 ms** because growing `10^k` by `10^1012` is a
   10.7k-limb × 53-limb multiply per term. A digit-count query on `Nat` would make it a comparison,
   but `Nat` exposes no such API and `Lovelace.Natural` is out of scope.
5. **Cold π (~534 ms at 1000 places)** is included in the first-call numbers and is unchanged; it is
   not what N23 asked for.

---

## 6. Acceptance evidence (observed output)

```
dotnet test Lovelace.Real.Tests -c Release --nologo --filter "Category!=Heavy"
Passed!  - Failed:     0, Passed:   304, Skipped:     0, Total:   304, Duration: 2 s - Lovelace.Real.Tests.dll (net10.0)

dotnet test Lovelace.Dsp.Tests -c Release --nologo
Passed!  - Failed:     0, Passed:    61, Skipped:     0, Total:    61, Duration: 2 s - Lovelace.Dsp.Tests.dll (net10.0)

dotnet test Lovelace.Natural.Tests -c Release --nologo
Passed!  - Failed:     0, Passed:   195, Skipped:     0, Total:   195, Duration: 132 ms - Lovelace.Natural.Tests.dll (net10.0)

dotnet test Lovelace.Integer.Tests -c Release --nologo
Passed!  - Failed:     0, Passed:   148, Skipped:     0, Total:   148, Duration: 98 ms - Lovelace.Integer.Tests.dll (net10.0)
```

304 = the 295 baseline plus the 9 new tests in `RealTrigFastPathTests`. The baseline was verified
directly: the pre-fix copy of the tree (kept at `%TEMP%\n23-before`, used for the before-measurements)
reports `Passed! - Failed: 0, Passed: 295, ...` for the same filter. As an extra check the Heavy
category of `Lovelace.Real.Tests` (12 pre-existing heavy tests plus the three new evidence probes)
was run too:

```
dotnet test Lovelace.Real.Tests -c Release --nologo --filter "Category=Heavy"
Passed!  - Failed:     0, Passed:    18, Skipped:     0, Total:    18, Duration: 1 m 15 s - Lovelace.Real.Tests.dll (net10.0)
```

No test was weakened, skipped, widened or deleted; no `NoWarn`, pragma or suppression was added; no
project outside `Lovelace.Real` / `Lovelace.Real.Tests` was touched (the pre-fix comparison tree
lives in `%TEMP%`, not in the repository).

---

## 7. Files

**Changed**

* `Lovelace.Real/Real.cs` — the six changes tabled in §2; the previous round's symmetry guards and
  all public signatures are untouched.

**Added (tests)**

* `Lovelace.Real.Tests/RealTrigFastPathTests.cs` — 9 equivalence/cost tests (default suite).
* `Lovelace.Real.Tests/RealTrigCostProbeTests.cs` — Heavy: phase breakdown, series internals,
  precision sweep, cold/warm single call per precision.
* `Lovelace.Real.Tests/RealTrigDigitDumpTests.cs` — Heavy: full stored-representation dump used for
  the SHA-256 before/after comparison.

**Added (evidence, raw output)**

* `docs/goal-cycle-4/round-07/probe-before/` — `phase-breakdown.txt`, `series-internals.txt`,
  `timing-sweep.txt`, `single-call-before.txt`, `digits-dump.txt` (4.5 MB), `probe-console.txt`,
  `digits-dump-console-before.txt`.
* `docs/goal-cycle-4/round-07/probe-after/` — the same files for the fixed tree
  (`single-call-after.txt`, `digits-dump.txt`, …).
