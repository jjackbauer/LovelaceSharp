# Round 02 — `Lovelace.Real` division scale alignment and periodic rendering

Fixes the two `Lovelace.Real` defects the SymPy differential oracle exposed. Both are PRE-EXISTING:
the two pre-fix lines this round replaces are present unchanged at `35e2609` and at `HEAD` (`3992d80`)
— `git show 35e2609:Lovelace.Real/Real.cs` and `git show HEAD:Lovelace.Real/Real.cs` both print
`long resultExponent = -fracLen + exponentAdjustment;` and
`allFrac[(int)PeriodStart..(int)(PeriodStart + PeriodLength)]`.

Files changed (SCOPE):

| File | Change |
| --- | --- |
| `Lovelace.Real/Real.cs` | fix (4 hunks, +56/−13) |
| `Lovelace.Real.Tests/RealDivideTests.cs` | +78/−0 (new tests only) |
| `Lovelace.Real.Tests/RealToStringTests.cs` | +58/−0 (new tests only) |

`git diff --numstat -- Lovelace.Real.Tests Lovelace.Real/Real.cs`:

```
78      0       Lovelace.Real.Tests/RealDivideTests.cs
58      0       Lovelace.Real.Tests/RealToStringTests.cs
56      13      Lovelace.Real/Real.cs
```

No pre-existing assertion was edited: both test files are pure additions (136 added, 0 removed), and
the 13 removed lines in `Real.cs` are exactly the three replaced statements plus their old comments.
Nothing in `Lovelace.Representation/**` was needed — the digit representation (`Nat` magnitude +
`Exponent` + `PeriodStart`/`PeriodLength`) is sound; the defect was in how `Divide` built values in
that representation. `Lovelace.Symbolics/**` was not touched.

## Root cause (one paragraph)

`a/b = (A·10^eA)/(B·10^eB)` equals `A/B` **only when `eA == eB`**. The old `Divide` ran the integer
and remainder digit division on the raw magnitudes and carried the whole scale difference
`exponentAdjustment = eA − eB` in the result exponent (`resultExponent = -fracLen + exponentAdjustment`),
while the fractional-digit loop derives the quotient's decimal point from the digit string it
accumulates. Whenever the divisor carried fractional digits (`eB < 0`) the two disagreed: the stored
digits no longer ended at the value's decimal point, so the raw quotient's leading fractional zeros
were absorbed by the exponent and shifted the value (`1 / 0.9` → `1`), or consumed the whole digit
budget (`1 / 0.99862888499828389309965043538706203025` under a 20-place cap → `0`), and
`PeriodStart`/`PeriodLength` ended up naming fractional positions the magnitude never stored
(`1 / 0.99` → magnitude `1`, `Exponent 0`, period length 2 — which is why `ToString()` then sliced a
1-character digit string with `PeriodStart..PeriodStart+PeriodLength` and threw
`ArgumentOutOfRangeException`).

## 1. Test-first (failing tests written and run before the fix)

Re-run of the pre-fix revision with only the new test files, in a throwaway worktree at `HEAD`
(the shared working tree was not reverted — the parent agent was working in it concurrently):

```
git worktree add --detach C:\Users\ricar\dev\.lovelace-round02-prefix HEAD
Copy-Item Lovelace.Real.Tests\RealDivideTests.cs   .lovelace-round02-prefix\Lovelace.Real.Tests\ -Force
Copy-Item Lovelace.Real.Tests\RealToStringTests.cs .lovelace-round02-prefix\Lovelace.Real.Tests\ -Force
cd C:\Users\ricar\dev\.lovelace-round02-prefix
dotnet test Lovelace.Real.Tests -c Release --nologo --filter "FullyQualifiedName~RealDivideTests|FullyQualifiedName~RealToStringTests"
```

OBSERVED failing output (`docs/goal-cycle-4/round-02/test-first-failure.log`):

```
Failed!  - Failed:    14, Passed:    30, Skipped:     0, Total:    44, Duration: 278 ms - Lovelace.Real.Tests.dll (net10.0)
```

The 14 failures, with the observed messages (verbatim from that log):

```
System.ArgumentOutOfRangeException : Index and length must refer to a location within the string. (Parameter 'length')
   ToString_GivenRepeatingQuotient_EmitsPeriodicNotation(dividend: "1", divisor: "0.99", expected: "1.(01)")
   ToString_GivenRepeatingQuotient_ParsesBackToAnEqualValue(dividend: "1", divisor: "0.99")
   ToString_GivenPeriodLongerThanStoredDigits_DoesNotThrow
Assert.Equal() Failure: Strings differ
Expected: "1.(1)"      Actual:   "1(1)"        <- 1 / 0.9, missing decimal point
Expected: "0.1(6)"     Actual:   "0.(1)"       <- 0.5 / 3, wrong period position
Assert.Equal() Failure: Values differ
Expected: 0.1(6)       Actual:   0.(1)         <- Divide_GivenDivisorWithFractionalDigits_ReturnsTheExactQuotient("0.5", "3")
Expected: 1.(1)        Actual:   1(1)
Expected: 1.(01)       Actual:   TargetInvocationException was thrown formatting an object of type "Lovelace.Real.Real"
                                 ---- System.ArgumentOutOfRangeException : Index and length must refer to a location within the string. (Parameter 'length')
Expected: 1.001372997539239477447020588085923996879   Actual: 1        <- 1 / 0.99862888499828389309965043538706203025
Expected: 1.000686263290967473969590465527195755929   Actual: 1.00068626329096747396   <- 1 / 0.9993142073433579945 (scale-shifted 20-digit loss)
Expected: 11           Actual:   1             <- Divide_GivenDivisorJustBelowOne_StoresTheQuotientAtItsOwnScale (magnitude)
Expected: 101          Actual:   1             <- Divide_GivenTwoDigitPeriod_StoresBothPeriodDigits (magnitude)
Assert.False() Failure                          <- Divide_GivenQuotientSmallerThanItsScale_KeepsTheSignificantDigits: quotient collapsed to exactly 0
```

The 14th failure is `ToString_GivenRepeatingQuotient_ParsesBackToAnEqualValue(dividend: "1", divisor: "0.9")`
(the reparse of the malformed `"1(1)"` fails — `Parse` rejects a period with no decimal point).

Baseline of the suite before any new test was added (same tree, before the edits):
`docs/goal-cycle-4/round-02/baseline-real-tests.log` →

```
Passed!  - Failed:     0, Passed:   272, Skipped:     0, Total:   272, Duration: 654 ms - Lovelace.Real.Tests.dll (net10.0)
```

## 2. The fix (`Lovelace.Real/Real.cs`)

**(a) `Divide` — align both magnitudes to one decimal exponent before the digit division;
`Real.cs:582-597`** (`git diff` hunk `@@ -580,8 +580,22 @@`):

```csharp
        Nat numerator   = left.ToNatural();
        Nat denominator = right.ToNatural();
        if (exponentAdjustment > 0L)
            numerator = numerator.ShiftLeftDecimal(exponentAdjustment);
        else if (exponentAdjustment < 0L)
            denominator = denominator.ShiftLeftDecimal(-exponentAdjustment);
```

`(A × 10^(eA−eB)) / B` when `eA ≥ eB`, `A / (B × 10^(eB−eA))` otherwise — the same
"shift the operand with the larger exponent left" alignment `Add` already uses
(`Real.cs:2134-2141`). The digit loop then produces the digits of the actual quotient.

**(b) drop the leftover scale term from the quotient exponent; `Real.cs:646-649`** (hunk
`@@ -631,5 +645,7 @@`): `long resultExponent = -fracLen + exponentAdjustment;` → `long resultExponent = -fracLen;`
The operands are already aligned, so the decimal point sits exactly `fracLen` digits from the end of
the digit string, which is the invariant the rest of the class (and `Parse`) relies on.

**(c) `ToString` periodic branch — read the digits through `GetDecimalDigit`, always emit the
decimal point; `Real.cs:1783-1809`** (hunk `@@ -1752,19 +1768,28 @@`). `GetDecimalDigit` is the
accessor that owns the period arithmetic (wrap-around, and zeros beyond the stored range), so the
renderer can no longer index past the end of the digit string; the previous `digits[(int)PeriodStart..]`
fallback threw for any value whose period was not stored. The `.` is now unconditional because a
periodic value always carries a fractional part — `1 / 0.9` renders `1.(1)`, never `1(1)`.

**(d) `DetectAndNormalizePeriod` — an all-zero period is not a period; `Real.cs:2085-2101`** (hunk
`@@ -2044,2 +2069,20 @@`). `0.(0) = 0` and `3.0(0) = 3`, so the canonical form carries no period and
no trailing zeros (symmetric with the existing all-nines rule that turns `0.999…` into `1`). This is
what keeps the periodic guard exact once (a) and (b) are correct: dividing by an operand expanded to
`workingFrac` digits now yields the *value-correct* quotient `3.000…0003` (free of the old
scale misalignment), whose fraction the guard's documented `slack = 1` truncation retry reports as a
zero period; without this rule the guard's result was `3.(0)` instead of exactly `3` — it broke the
pre-existing tests `Divide_GivenNonPeriodicByPeriodic_PreservesCorrectness` /
`…_ExponentCompensatedCorrectly` (observed `Expected: 3 / Actual: 3.(0)`).

No stub remains: every changed path is fully implemented (`git diff` adds no `throw`/`TODO`); the only
`NotImplementedException`s in the file are the pre-existing `Pow` guards at `Real.cs:728/746/755`,
which this round neither introduced nor touched, and which the negative-power path never reaches
(the oracle's negative-integer-power cases pass). No `#pragma`, `NoWarn` or suppression attribute was
added. The change is AOT-safe: it uses only `Nat`/`string`/`char` operations — no reflection, no
dynamic codegen.

## 3. Post-fix observed results

Focused (all pre-existing + new tests in the two touched classes):

```
dotnet test Lovelace.Real.Tests -c Release --nologo --filter "FullyQualifiedName~RealDivideTests|FullyQualifiedName~RealToStringTests"
Passed!  - Failed:     0, Passed:    44, Skipped:     0, Total:    44, Duration: 639 ms - Lovelace.Real.Tests.dll (net10.0)
```

Full Real suite — `docs/goal-cycle-4/round-02/real-tests-after-fix.log`:

```
dotnet test Lovelace.Real.Tests -c Release --nologo --filter "Category!=Heavy"
Passed!  - Failed:     0, Passed:   292, Skipped:     0, Total:   292, Duration: 1 s - Lovelace.Real.Tests.dll (net10.0)
```

292 = the 272 pre-existing tests + 20 new test cases (9 in `RealDivideTests`, 11 in `RealToStringTests`); 0 failed, 0 skipped, no existing test weakened, deleted or skipped.

Differential oracle (SymPy on PATH via `C:\Users\ricar\dev\.lovelace-tools\python`,
`LOVELACE_REQUIRE_SYMPY=1`, so nothing skips):

```
dotnet test Lovelace.Symbolics.Tests -c Release --nologo --filter "FullyQualifiedName~DifferentialOracle"
Test Run Successful.
Total tests: 6
     Passed: 6
```

At that point the six oracle comparisons (derivatives, solve, roots, factorization, limits, matrix
operations) all agreed with SymPy with `Skipped 0` — the previously failing
`SympyOracle_Derivatives_Agree` (`diff(tan(x**3))` at `x = 1/3`: kernel `0` vs sympy
`0.333790999179746492481090015847`, see `round-01/oracle-run-after-N20-fix.log`) now passes.

USER-VISIBLE check through the published runner (`dotnet Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll --file … --omit-functions`):

| script | pre-fix (`runner-before-fix.log`) | post-fix (`runner-after-fix.log`) |
| --- | --- | --- |
| `1 / 0.99` | `{"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"Index and length must refer to a location within the string. (Parameter 'length')", …}` | `{"ok":true,…,"display":"1.(01)","exact":true,"numerator":"100","denominator":"99"}` |
| `1 / 0.9` | `"display":"1(1)"` | `"display":"1.(1)"` |
| `10 / 9` | `"display":"1.(1)"` | `"display":"1.(1)"` |
| `1 / 0.9993142073433579945` | — | `"display":"1.0006862632909674739695904655271957559293067…"` (matches the published 39-digit value) |

## 4. What is NOT passing, and why it is not this fix

The brief's required command also matches `Lovelace.Symbolics.Tests/NegativePowerEvaluationTests.cs`,
a file the parent agent owns and was rewriting while this round ran. Timeline, from file mtimes and
the captured logs:

* 11:38:18 `real-tests-after-fix.log` — 292/0 with this fix.
* 11:38 the six `DifferentialOracle` tests passed (6/6, Skipped 0) with this fix.
* 11:39:18 the parent committed a new `OracleCorpus.cs` revision (`REPAIR (round 03)`, line 74)
  that reinstates the `x*log(x)` sample point `x = -1/2`.
* 11:40:52 / 11:41:32 the required command and the `DifferentialOracle` filter now report
  `Failed: 1, Passed: 12, Total: 13` and `Failed: 1, Passed: 5, Total: 6`
  (`oracle-required-command.log`, `oracle-differential-only.log`), both failing the same case:

```
diff(x*log(x)) at x = -1/2: the kernel side did not evaluate
(EvaluationException: Ln(x) requires x > 0.  (Parameter 'x'))
kernel form (add (fn log (sym x)) (mul (sym x) (pow (sym x) (rat -1 1)))) vs sympy sympy.diff(x*log(x), x)
```

That is a `Lovelace.Symbolics` logarithm-domain/branch issue (the corpus note itself says the kernel
must produce SymPy's `log|x| + i*pi` closed form there). It is outside this round's SCOPE
(`Lovelace.Symbolics/**`), it involves no `Real` division, and the same run passed before that corpus
edit — this fix neither causes nor can repair it. The parent has separately confirmed the
`cos(1/27)^-2` expectation in `NegativePowerEvaluationTests.cs` was its own paste error (SymPy 1.14.0
`cos(1/27)^-2 = 1.00137299753923947744327004754154925203741`, which the kernel matches to 40 digits),
so no kernel change was made to satisfy it.

## Artifacts (all under `docs/goal-cycle-4/round-02/`)

| File | Content |
| --- | --- |
| `baseline-real-tests.log` | `Lovelace.Real.Tests` before the new tests: 272 passed |
| `test-first-failure.log` | pre-fix worktree with the new tests: Failed 14 / Passed 30 / Total 44 |
| `after-fix-focused.log` | the two touched classes after the fix: 44/44 |
| `real-tests-after-fix.log` | full Real suite after the fix: 292 passed, 0 failed, 0 skipped |
| `runner-before-fix.log` / `runner-after-fix.log` | published runner envelopes for `1 / 0.99`, `1 / 0.9`, `10 / 9` (and `1 / 0.9993142073433579945`) before and after |
| `oracle-required-command.log` | the required oracle command, current tree state |
| `oracle-differential-only.log` | the six `DifferentialOracle` comparisons, current tree state |
| `oracle-after-fix.log` | the required command at 11:3x, when the only failure was the parent's `NegativePowerEvaluationTests.cs` literal |

## 5. Round 03 — the round-02 fix broke `Lovelace.Dsp.Tests/TrigTests.cs:56`; exact symmetry restored

The round-02 edits above made an EXISTING, unmodified test fail. Two-worktree comparison (the parent's
measurement, reproduced independently here in `.lovelace-round03-probe`):

| tree | `dotnet test Lovelace.Dsp.Tests -c Release --nologo` |
| --- | --- |
| worktree at HEAD `fca8277` WITHOUT the round-02 `Real.cs` edit | `Failed: 0, Passed: 61, Total: 61` |
| worktree at HEAD `fca8277` WITH the round-02 `Real.cs` edit | `Failed: 1, Passed: 60, Total: 61` |
| working tree with the round-02 edit **and** the symmetry fix below | `Failed: 0, Passed: 61, Total: 61` |

The failing assertion (`Lovelace.Dsp.Tests/TrigTests.cs:56`, unmodified):

```
public void Sin_GivenNegativeAngle_IsNegated()
    => Assert.Equal(-Rl.Sin(Rl.Pi / new Rl("6")), Rl.Sin(-(Rl.Pi / new Rl("6"))));

Expected: -0.5
Actual:   -0.4999999999999999999999999999999999999999999999999999999999996258325960938648951279436670591597716318…
```

Independent reproduction: `docs/goal-cycle-4/round-02/regression-repro-round02-only.log` — the round-02
`Real.cs` with only the two symmetry blocks removed gives exactly
`Failed Lovelace.Dsp.Tests.TrigTests.Sin_GivenNegativeAngle_IsNegated … Failed! - Failed: 1, Passed: 60, Total: 61`.

### Mechanism (measured, not assumed)

`Sin` reduced a negative angle by rebuilding it as `2π + value`. My round-02 change moved the division
truncation boundary, and the rebuilt angle then differs from `11π/6` by one unit in the last place.
Probe output (`probe-sin-prefix.txt` = HEAD, `probe-symmetry-fixed.txt` = round-02 only):

```
HEAD  (pre-round-02)  Pi/6  exp=-101  digits=101   -> reduced.Equals(11*Pi/6) = True    sin(-Pi/6) == -sin(Pi/6) = True
round-02 only         Pi/6  exp=-100  digits=100   -> reduced.Equals(11*Pi/6) = False   sin(-Pi/6) == -sin(Pi/6) = False
round-02 + this fix   Pi/6  exp=-60   digits=60    -> sin(-Pi/6) == -sin(Pi/6) = True    cos(-Pi/6) == cos(Pi/6) = True
```

Why one ulp: `Pi/6` is `P = trunc₁₀₀(π)` divided by 6, and `P/6` terminates exactly at **101**
fractional digits (`P mod 6 = 3` → fraction `0.5`). Pre-fix the loop stored 101 fractional digits, so
`2π − Pi/6` reproduced `11π/6` (also exactly 101 digits) digit-for-digit. Post-fix the loop stops at
exactly the requested 100 digits, so `Pi/6` carries a `5e-101` truncation error; `2P − trunc₁₀₀(P/6)`
is then `11P/6 + 5e-101 + 5e-101` = `11π/6 + 1e-100` — exactly one ulp at the working scale
(`…2912` vs `…2911`) — `TrySpecialAngle`'s exact comparison misses, and the angle falls through to the
Taylor series. The same defect, unobserved by any existing test, made `Cos(-(Pi/6)) ≠ Cos(Pi/6)`.

### Fix (`Real.cs:1250-1259` and `Real.cs:1284-1290`)

`Sin` and `Cos` now apply their symmetry to the magnitude before any reduction — `sin(-x) = -sin(x)`,
`cos(-x) = cos(x)` — instead of rebuilding a negative angle at the active scale:

```csharp
        if (value < Zero)
            return -Sin(-value, digits, progress);      // Sin
        if (value < Zero)
            return Cos(-value, digits, progress);       // Cos
```

This is structural, not a tolerance or a special case: the symmetry holds at every precision, the
positive-angle reduction path (where the exact special-angle table lives) is unchanged, and negative
arguments are computed on `|x|` instead of on a value reconstructed through a cancellation. The
private `ReduceToTwoPi` is left as it is; both its callers now hand it only non-negative arguments.

### Guard test (new)

`Lovelace.Real.Tests/RealTrigSymmetryTests.cs` asserts, exactly (never through a rendered string):
`Sin(-x) == -Sin(x)` and `Cos(-x) == Cos(x)` for the 15 special angles in `(0, 2π)`, for angles beyond
one turn (`13π/6`, `25π/4`), for the non-special angles `1`, `2`, `7`, `0.5`, `-3` and a 100-digit
decimal that only approximates `π/6`,
and the exact values `Sin(-(π/6)) = -0.5`, `Sin(-(π/4)) = -√2/2`, `Cos(-(π/6)) = √3/2`, `Cos(-π) = -1`.
Test-first evidence: in the round-02-only worktree the guard reports
`Failed! - Failed: 3, Passed: 0, Total: 3` (`regression-guard-test-first.log`:
`Expected: -0.5 / Actual: -0.4999…`); with the fix it is 3/3.

### Acceptance runs (observed)

```
dotnet test Lovelace.Dsp.Tests -c Release --nologo
Passed!  - Failed:     0, Passed:    61, Skipped:     0, Total:    61, Duration: 18 s - Lovelace.Dsp.Tests.dll (net10.0)

dotnet test Lovelace.Real.Tests -c Release --nologo --filter "Category!=Heavy"
Passed!  - Failed:     0, Passed:   295, Skipped:     0, Total:   295, Duration: 2 s - Lovelace.Real.Tests.dll (net10.0)
```

(`real-tests-after-symmetry-fix.log`, `dsp-after-symmetry-fix.log`.) 295 = 272 pre-existing + the 20
round-02 tests + the 3 new symmetry guards; the test files remain pure additions
(`RealDivideTests.cs` +78/−0, `RealToStringTests.cs` +58/−0, `RealTrigSymmetryTests.cs` new, 71 lines).
No existing assertion was touched, no tolerance changed, no suppression added.

Collateral check on the differential oracle (it evaluates `sin(x²)` and `sin(x)/x` at `x = -1/2`, i.e.
the negative-argument path this section changes) — `oracle-after-symmetry-fix.log`:

```
dotnet test Lovelace.Symbolics.Tests -c Release --nologo --filter "FullyQualifiedName~DifferentialOracle"
Passed!  - Failed:     0, Passed:     6, Skipped:     0, Total:     6, Duration: 29 s - Lovelace.Symbolics.Tests.dll (net10.0)
```

The `x*log(x)` case recorded as failing in section 4 has since been repaired on the Symbolics side by
its owner; this fix neither caused nor touched it.

The falsification round's `Complex.Magnitude`/`DivideByZeroException` claim for tiny magnitudes was
measured by the parent as not reproducible (what differs is `1 / tiny`, which the round-02 fix
*improves*); no workaround for it was added here.

### Round-03 artifacts

| File | Content |
| --- | --- |
| `regression-repro-round02-only.log` | Dsp suite in the round-02-only worktree: Failed 1 / Passed 60 / Total 61 |
| `regression-guard-test-first.log` | the new symmetry guard in that worktree: Failed 3 / Passed 0 / Total 3 |
| `probe-sin-prefix.txt` / `probe-symmetry-fixed.txt` / `probe-symmetry-after-symfix.txt` | representation of `Pi/6`, `11π/6`, `2π − Pi/6` and the symmetry table in each tree |
| `dsp-after-symmetry-fix.log` | `Lovelace.Dsp.Tests`: 61 passed, 0 failed |
| `real-tests-after-symmetry-fix.log` | `Lovelace.Real.Tests` (non-Heavy): 295 passed, 0 failed |
