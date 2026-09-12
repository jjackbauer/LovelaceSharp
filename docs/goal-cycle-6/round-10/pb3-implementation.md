# Cycle 6 · round 10 — P-B3 / O-B10 (first half): a quotient whose leading zeros outlive the digit budget

**Revision.** HEAD = `dfe42cf938d8949b710131e41fe1d9cda8afdf2f` (`git -C C:\Users\ricar\dev\LovelaceSharp rev-parse HEAD`).

**Trees used — nothing was committed, staged or pushed.**

| Tree | Revision | Working-tree state |
|---|---|---|
| `.worktrees/c6-pb3` (the fix) | `dfe42cf` detached | `M Lovelace.Real/Real.cs`, `?? Lovelace.Real.Tests/RealDivideLeadingZeroRunTests.cs` |
| `.worktrees/c6-pb3-ctl` (the control) | `dfe42cf` detached | `?? Lovelace.Real.Tests/RealDivideLeadingZeroRunTests.cs` (the identical file, sha256 `783D9790…59CF`) — product source pristine, `Real.cs` sha256 `6B0B327D…003` |

The fixed `Real.cs` sha256 is `62F9F003D8D5B33D9630168FAD977ABE5EB3E2D0B4E9465C5A3E22CF909D6B4C`; the diff is 1 file, +45 −4
(`git -C .worktrees/c6-pb3 diff --stat`). `Lovelace.Complex` was not touched in either tree.

Everything below was observed on this machine (Windows PowerShell 5.1). The runner used for the wire probes is
`<tree>\Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll`, referred to as `$RUN`; scripts live in
`%T% = C:\Users\ricar\AppData\Local\Temp\lov6\pb3`, each probe a `.ls` file of the quoted text, run as
`dotnet $RUN --file <probe>.ls --json --omit-functions --omit-variables`. The oracle is
`$env:PATH = 'C:\Users\ricar\dev\.lovelace-tools\python;' + $env:PATH` with `$env:LOVELACE_REQUIRE_SYMPY='1'`
(Python 3.12.14, mpmath 1.3.0, SymPy 1.14.0).

---

## 1. Pre-fix behaviour (pristine tree) and the mpmath ground truth

The two probes of the defect, verbatim from the pristine tree (the deciding part of the envelope; the envelope also
carries `display`/`typed`, quoted in §5):

| Probe (`.ls` text) | `result.structured` before the fix |
|---|---|
| `1/(3*10^1000)` | `{"kind":"Real","value":"0","exact":false}` |
| `evalf(1/(3*10^1000), 30)` | `{"kind":"Integer","value":"0","exact":true}` |

Boundaries, pristine tree (`%T%\pb3\probe-summary-ctl.ps1`; `value` elided to 58 chars + length):

```
p_3e99   | 1/(3*10^99)    | kind=Real    exact=True  value=0.0000…[104] num=1 den=100d
p_3e100  | 1/(3*10^100)   | kind=Real    exact=True  value=0.0000…[105] num=1 den=101d
p_3e999  | 1/(3*10^999)   | kind=Real    exact=False value=0.0000…[1002] num= den=
p_3e1000 | 1/(3*10^1000)  | kind=Real    exact=False value=0            num= den=
p_7e5000 | 1/(7*10^5000)  | kind=Real    exact=False value=0            num= den=
```

Independent ground truth — `python %T%\pb3_gt.py` / `pb3_gt2.py` (mpmath, 80 dps):

```
CASE 1/(3*10^99)   = 3.33333333333333333333333333333333333333333333e-100
CASE 1/(3*10^100)  = 3.33333333333333333333333333333333333333333333e-101
CASE 1/(3*10^1000) = 3.33333333333333333333333333333333333333333333e-1001
CASE 1/(7*10^5000) = 1.42857142857142857142857142857142857142857143e-5001
1/(1009*10^1000) = 9.91080277502477700693756194252e-1004 ; ord_1009(10) = 252
```

So the loss starts exactly where the leading-zero run reaches the 1000-place budget: `1/(3*10^999)` (run 999) is
still a non-zero truncation, `1/(3*10^1000)` (run 1000) is 0, and the wire then read that zero as exact through
`evalf`.

**Root cause, confirmed.** `Lovelace.Real/Real.cs:820` `Divide`: after the operand alignment the fractional loop
ran `while (… position < MaxComputationDecimalPlaces)` and appended every digit it generated, so the leading zeros
of the quotient spent the budget, `Nat.TryParse("0" + "000…0")` produced `Nat.Zero`, and `resultExponent =
-fracLen` had no digit left to place. The comment at `Real.cs:868-878` (the earlier `1/0.9` → `1` and
`1/0.99862…` → `0` repair) is what warned against carrying the scale in the result exponent naively; the fix
below keeps that invariant instead of re-breaking it.

---

## 2. The failing test, written first

New file `Lovelace.Real.Tests/RealDivideLeadingZeroRunTests.cs` (sha256 `783D9790…59CF`), 6 cases:
4 boundary cases in one `[Theory]` (`1/(3*10^99)`, `1/(3*10^100)`, `1/(3*10^1000)`, `1/(7*10^5000)`), one
truncation/honesty `[Fact]` (`setprecision(20); 1/(1009*10^1000)`), one neighbour `[Fact]` (`1/(3*10^100)`).
Each boundary case asserts the magnitude (`:60`), the **scale** (`:61`), `PeriodStart`/`PeriodLength`
(`:65-66`), `IsExact` (`:69`), the rendered value `"0." + <zeroRun> + "(" + <period> + ")"` (`:73`) and an
independent `Real.Parse` of that same ground-truth shape (`:74`) — never merely "non-zero".

Observed failure on the pristine tree (control tree, identical file, same command in `.worktrees/c6-pb3-ctl`):

```
dotnet test .worktrees/c6-pb3-ctl/Lovelace.Real.Tests/Lovelace.Real.Tests.csproj -c Release --nologo \
  --filter "FullyQualifiedName~RealDivideLeadingZeroRunTests"

  Failed …(coefficient: "7", zeroRun: 5000, period: "142857") [69 ms]
   Assert.Equal() Failure: Values differ
Expected: 142857
Actual:   0
     …RealDivideLeadingZeroRunTests.cs:line 60
  Failed …(coefficient: "3", zeroRun: 1000, period: "3") [44 ms]
   Assert.Equal() Failure: Values differ
Expected: 3
Actual:   0
     …RealDivideLeadingZeroRunTests.cs:line 60
  Failed …Divide_GivenQuotientTheBudgetCannotHold_IsInexactAndKeepsItsScale [1 ms]
   Assert.False() Failure
Expected: False
Actual:   True
     …RealDivideLeadingZeroRunTests.cs:line 92

Failed!  - Failed:     3, Passed:     3, Skipped:     0, Total:     6, Duration: 280 ms
```

The three passes are the two boundary cases already inside the budget and the neighbour — the failure is the defect
and only the defect. After the fix the same six cases: `Passed! - Failed: 0, Passed: 6, … Total: 6`.

---

## 3. The fix

```diff
diff --git a/Lovelace.Real/Real.cs b/Lovelace.Real/Real.cs
index af7a157..e0c1518 100644
--- a/Lovelace.Real/Real.cs
+++ b/Lovelace.Real/Real.cs
@@ -911,7 +911,31 @@ public class Real :
         bool foundPeriod  = false;
         long position     = 0L;
 
-        while (!Nat.IsZero(remainder) && position < MaxComputationDecimalPlaces)
+        // The expansion of a quotient below one opens with a run of zeros.  The budget counts decimal
+        // PLACES, and a zero occupies a place like any other digit — until the run ALONE fills the
+        // budget.  At that point the loop has generated a string of zeros with no significant digit
+        // in it, and the quotient comes back as zero with its scale gone: 1/(3·10^1000) spent the
+        // whole 1000-place budget on 1000 leading zeros and returned 0 where mpmath has 3.33…e-1001,
+        // while 1/(3·10^100), a run of 100, was already the exact periodic 0.(3) at scale 10^-100.
+        // That is the recorded defect, and it is the only state this changes.  A Real is
+        // digits × 10^Exponent, so a run that has outlived the budget is SCALE, not digit: from there
+        // the loop WALKS it at no cost — walking it keeps the remainder history on the true
+        // fractional positions, which is what lets a period that begins inside the run
+        // (1/99 = 0.(01)) still be found — stores only the digits that carry information, and the
+        // exponent below places the decimal point where the quotient's first significant digit
+        // really is.  A quotient whose run fits inside the budget never enters that state: it walks
+        // the identical positions, keeps the identical remainder history, and ends with the identical
+        // stored digits, period, exponent and rendering it had before.
+        long leadingZeros  = 0L;
+        long storedFrac    = 0L;
+        bool freed         = false;
+        bool skippingZeros = Nat.IsZero(quotient);
+
+        // Normal accounting: the budget bounds the decimal places generated.  Once the run has
+        // outlived it, the budget bounds the stored digits instead — the places it spends are all
+        // zeros, and a truncation to nothing is not a truncation of the quotient.
+        while (!Nat.IsZero(remainder) &&
+               (freed ? storedFrac < MaxComputationDecimalPlaces : position < MaxComputationDecimalPlaces))
         {
             string remKey = remainder.ToString();
 
@@ -932,8 +956,22 @@ public class Real :
 
             // digitNat is guaranteed to be in [0, 9].
             char digitChar = (char)('0' + (Nat.IsZero(digitNat) ? 0 : int.Parse(digitNat.ToString())));
-            fracDigits.Add(digitChar);
             position++;
+
+            // Leading zeros: scale, never a stored digit — in normal accounting they already spent
+            // the place that bounds the loop above, and under the freed accounting they spend
+            // nothing.  Either way the exponent below carries them.
+            if (skippingZeros && digitChar == '0')
+            {
+                leadingZeros++;
+                if (!freed && position >= MaxComputationDecimalPlaces)
+                    freed = true;   // the run alone has filled the budget: from here it is scale
+                continue;
+            }
+
+            skippingZeros = false;
+            fracDigits.Add(digitChar);
+            storedFrac++;
         }
 
         // Build combined digit string: integer part + fractional digits generated.
@@ -946,8 +984,11 @@ public class Real :
         // For periodic results the stored fraction is exactly periodStart + periodLength chars.
         // (The loop breaks without adding the repeated digit, so fracDigits already has the right count.)
         // The operands were aligned to a single exponent above, so the decimal point sits exactly
-        // fracLen digits from the end of the digit string: no leftover scale term.
-        long resultExponent = -fracLen;
+        // fracLen digits from the end of the digit string — and the leading zeros the loop walked but
+        // did not store sit to the left of it, carried by the exponent.  For a periodic result the
+        // break position is periodStart + periodLength, so this exponent is exactly the one the old
+        // loop produced: Exponent == -(PeriodStart + PeriodLength) still holds, digit for digit.
+        long resultExponent = -(leadingZeros + fracLen);
 
         if (!Nat.TryParse(allDigits, null, out Nat? mag))
             mag = Nat.Zero;
```

**Why this shape and not the obvious ones.** Two wider designs were implemented and rejected by the tree's own
suites (each is a recorded observation, not a preference):

| Design | What it broke |
|---|---|
| the budget counts *stored* digits everywhere (zeros always free) | `Lovelace.Real.Tests` 2495/5 failed — a purely periodic block that opens with a zero is stored by the loop as `0.(0588235294117647)` for 1/17 (`RealMultiplyPeriodicExactTests.cs:153,199`) and `LReal64Tests.cs:100` / `LReal128Tests.cs:56`; freeing every run re-anchored the block at the first significant digit and rendered `0.0(5882352941176470)`. `Lovelace.Symbolics.Tests` 1 failed — `TruncatingEvaluationStaysInexactTests.FunctionOfATruncatedArgument_IsInexact` threw `InvalidOperationException : Periodic Real values convert to Rational, not RealLiteral` from `RealLiteral.FromRealExact` (`Expr.cs:116`) because `tan(pi(30))` folds through `NumOps.Tan` → `Real.Divide` (`Evaluation.cs:339`) and the longer scan found a 40-digit period in `sin(pi(30))/cos(pi(30))`. `Lovelace.Suite.Tests` 1 failed — `StructuredRealExactnessTests.TruncatedQuotientAtTheBudget_IsNotCertifiedExact` (`StructuredRealExactnessTests.cs:101-111`) pins `setprecision(18); 1/1009` to `0.000991080277502477`, i.e. 18 decimal **places**, and the wider rule returned 18 significant digits (21 places). |
| free the run whenever it starts, but decline a repeat whose block opens inside it | kept all suites green, but it declined exactly-representable quotients whose period starts inside the run (e.g. `1/(17*10^1000)`, period start 1000 < run 1001) and made them inexact. Unnecessary once the minimal rule above was found. |

The shipped rule touches only the one state the defect names — a run that *alone* fills the budget — so every
quotient whose run fits keeps its pre-fix digits, period, exponent and rendering. The strongest evidence for that
is §7: all three suites are at their pristine counts.

---

## 4. Boundary table against mpmath, after the fix

Probe values are the wire's own `structured` payload; the fraction check is
`python %T%\pb3\boundary_check.py` (`Fraction(numerator, denominator) == Fraction(1, c·10^n)`), the shape check is
against `mp.nstr(1/(c·10^n), 45)`.

| `.ls` probe | `exact` | published `numerator/denominator` | display shape | mpmath |
|---|---|---|---|---|
| `1/(3*10^99)` | true | `1 / 3·10^99` ✓ | 99 zeros then `(3)` (104 chars) | `3.33333333333333333333333333333333333333333333e-100` |
| `1/(3*10^100)` | true | `1 / 3·10^100` ✓ | 100 zeros then `(3)` (105 chars) | `3.33333333333333333333333333333333333333333333e-101` |
| `1/(3*10^999)` | **false** | none | 999 zeros then `3` (1002 chars) | truncation to 1000 places of `3.333…e-1000` — run 999 < budget, untouched by this change (identical on both trees) |
| `1/(3*10^1000)` | true | `1 / 3·10^1000` (1001 digits) ✓ | 1000 zeros then `(3)` (1005 chars) | `3.33333333333333333333333333333333333333333333e-1001` |
| `1/(7*10^5000)` | true | `1 / 7·10^5000` (5001 digits) ✓ | 5000 zeros then `(142857)` (5010 chars) | `1.42857142857142857142857142857142857142857143e-5001` |

Honesty (requirement 3), on the wire:

| probe | before | after |
|---|---|---|
| `setprecision(20); 1/(1009*10^1000)` | `Real value=0 exact=false` | `Real value="0.<1003 zeros>99108027750247770069" exact=false`, **no `numerator`/`denominator`** — a genuine truncation (period 252 > 20 places), matching mpmath `9.9108027750247770069…e-1004` truncated, not rounded |
| `1/(1009*10^1000)` (default budget) | `Real value=0 exact=false` | `Real exact=true numerator=1 denominator=<1004 digits>` — period 252 fits, so it is exact |

`Lovelace.Real.Tests`: `RealDivideLeadingZeroRunTests.Divide_GivenQuotientTheBudgetCannotHold_IsInexactAndKeepsItsScale`
asserts the same value and flag at `Real` level (`IsExact == false`, no period) and
`…KeepsTheValueAndItsScale` asserts `IsExact == true` for the four exactly-representable boundaries.

---

## 5. Wire probe before / after

Command (each row): `dotnet $RUN --file %T%\pb3\<name>.ls --json --omit-functions --omit-variables`.

```
BEFORE (pristine .worktrees/c6-pb3-ctl)                     AFTER (.worktrees/c6-pb3, Real.cs fixed)
1/(3*10^1000)
  {"kind":"Real","display":"0","typed":"0 (Real)",
   "structured":{"kind":"Real","value":"0","exact":false}}
                                                        ->  {"kind":"Real","display":"0.<1000 zeros>(3)",
                                                             "typed":"0.<1000 zeros>(3) (Real)",
                                                             "structured":{"kind":"Real",
                                                               "value":"0.<1000 zeros>(3)","exact":true,
                                                               "numerator":"1",
                                                               "denominator":"3<1000 zeros>"}}
     (<1000 zeros> elided for width; the display is 1005 chars and the denominator 1001 digits — the
      full strings were read back and compared to mpmath in §4)

evalf(1/(3*10^1000), 30)
  {"kind":"Integer","display":"0","typed":"0 (Integer)",
   "structured":{"kind":"Integer","value":"0","exact":true}}
                                                        ->  {"kind":"Real","display":"0","typed":"0 (Real)",
                                                             "structured":{"kind":"Real","value":"0","exact":false}}
```

Unchanged neighbours, both trees, byte-identical deciding fields: `1/(3*10^99)` `exact=true`; `1/(3*10^100)`
`exact=true num=1 den=101 digits`; `(5/6)/(7/11)` `value=1.3(095238) exact=true num=55 den=42`; `1/0.9`
`value=1.(1) exact=true num=10 den=9`; `1/0.99862888499828389309965043538706203025`
`exact=false`, same 100-digit rendering; `1/3` `0.(3) exact=true num=1 den=3`; `1/(3*10^999)` non-zero
inexact with the same 1002-char display.

---

## 6. Control tree

`git -C .worktrees/c6-pb3-ctl worktree add .worktrees/c6-pb3-ctl HEAD` (created from HEAD `dfe42cf`), then
**only** `RealDivideLeadingZeroRunTests.cs` copied in — one entry in `git status --porcelain`
(`?? Lovelace.Real.Tests/RealDivideLeadingZeroRunTests.cs`), product source untouched
(`Real.cs` sha256 `6B0B327D…003`). Result of the filtered run: **`Failed: 3, Passed: 3, Total: 6`** — the same
three failures quoted in §2. The test fails on the pre-fix tree and passes on the fixed tree, and the file is
byte-identical in both (sha256 `783D9790A06071CEE873B2B2D1E220A3352298ED5EDF79421C050E977DCA59CF`).

---

## 7. Suite totals

All runs `dotnet test <proj>.csproj --configuration Release --nologo`, no filter and no runsettings, so the
`Heavy` trait is included; `--filter "Category=Heavy"` on the fixed tree selects 13 tests, all passing, and the
unfiltered run reports `Skipped: 0`.

| Suite | Pristine (`.worktrees/c6-pb3-ctl`) | Fixed (`.worktrees/c6-pb3`) |
|---|---|---|
| `Lovelace.Real.Tests` | `2489 passed, 0 failed, 0 skipped` (new file excluded via `--filter "FullyQualifiedName!~RealDivideLeadingZeroRunTests"`; with the file: 3 failed / 2489+3 passed) | **`Passed: 2495, Failed: 0, Skipped: 0, Total: 2495`** = the 2489 pristine cases + the 6 new ones |
| `Lovelace.Symbolics.Tests` | `Passed: 1097, Skipped: 8, Total: 1105` | `Passed: 1097, Skipped: 8, Total: 1105` (identical) |
| `Lovelace.Suite.Tests` | `Passed: 825, Failed: 0, Total: 825` | `Passed: 825, Failed: 0, Total: 825` (identical) |

`Lovelace.Console.Tests`, `Lovelace.Run.Tests` and the remaining projects were **not** run — outside the
requested set.

---

## 8. Could not verify / residuals (stated, not accepted silently)

1. **`evalf(1/(3*10^1000), 30)` no longer claims exactness but still publishes 0.** After the fix it is
   `{"kind":"Real","value":"0","exact":false}` — the "wire declares a wrong zero exact" half is closed, the
   "returns 0" half is not, one layer up. It is **not** caused by `Divide`: on the pristine tree the in-budget
   neighbour `evalf(1/(3*10^100), 30)` already answered `{"kind":"Real","value":"0","exact":false}` while
   `1/(3*10^100)` itself is exact. The site is `Lovelace.Symbolics/RationalReal.cs:29-51`
   (`TruncatingDecimalString` emits a fixed number of FRACTIONAL digits and stops, so any `|x| < 10^-digits`
   renders `"0.000…0"`), reached from `SymbolicsPlugin.cs:1521-1531` → `Evaluation.cs:51`. Fixing it means
   changing `evalf`'s "decimal places" contract in `Lovelace.Symbolics`, which is outside this round's scope
   (`Lovelace.Real/Real.cs` + `Lovelace.Real.Tests`), so it is left open and flagged. **Not closed.**
2. **The band just inside the budget stays a truncation.** `1/(3*10^999)` (run 999, budget 1000) is non-zero and
   inexact although its period is one digit; `1/(3*10^1000)` (run 1000) is exact. That boundary is the price of
   keeping the budget's meaning as decimal PLACES, which
   `Lovelace.Suite.Tests/StructuredRealExactnessTests.cs:101-111` pins for `setprecision(18); 1/1009`. The value
   is right and the pre-fix behaviour there is unchanged (verified on both trees). **Not closed, by design.**
3. **Cost of the freed walk is measured only to 5000 zeros.** The freed state walks the run one big-integer step
   at a time; `%T%\pb3\p_7e5000.ls` completes in `elapsed 41.21 ms` (pre-fix: `73.73 ms`), and `p_3e1000` in
   `21.29 ms` (pre-fix: `63.72 ms`). Runs orders of magnitude longer (e.g. `1/(3*10^1000000)`) were **not**
   timed — no claim is made about them.
4. **No exhaustive equivalence check** that *every* in-budget quotient is bit-identical to pre-fix; the evidence is
   the three suites at their pristine counts plus the `1/(3*10^999)` probe, not a proof.
5. **Not run:** `Lovelace.Run.Tests`, `Lovelace.Console.Tests`, and the wire-level differential oracle
   (`LOVELACE_REQUIRE_SYMPY` was set for the mpmath/SymPy ground-truth scripts, not for a full oracle pass).
6. **No commit, stage or push** was performed; the change exists only as a working-tree modification in
   `.worktrees/c6-pb3` plus the untracked test file (also present in `.worktrees/c6-pb3-ctl`).

## 9. Files

- Fix: `.worktrees/c6-pb3/Lovelace.Real/Real.cs` (`Divide`, diff in §3; sha256 `62F9F003…B4C`).
- Test: `.worktrees/c6-pb3/Lovelace.Real.Tests/RealDivideLeadingZeroRunTests.cs` (and the identical copy in
  `.worktrees/c6-pb3-ctl/Lovelace.Real.Tests/`).
- Probes/scripts: `%T%\pb3` (`prefix-probes.ps1`, `extra-probes.ps1`, `probe-summary.ps1`,
  `probe-summary-ctl.ps1`, `boundary.ps1`, `boundary_check.py`, `pb3_gt.py`, `pb3_gt2.py`,
  `tan-probes.ps1`).