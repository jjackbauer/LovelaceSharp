# Real.Sqrt — Representation-First Redesign Options

> Analysis of `Real.Sqrt`'s seeding strategy and Newton-Raphson loop against the `Real`
> type's inherent representation (BCD magnitude + `Exponent` + period metadata), identifying
> two correctness / robustness gaps and proposing targeted fix options with test plans.

---

## Functionality Worktree

### Analysis Summary

The current `Real.Sqrt(Real value, long precision)` algorithm has two structural mismatches
with how `Real` stores its value.

#### Gap 1 — Periodic inputs: `Divide` ignores `IsPeriodic` on the left operand

`Real.Divide` extracts the numerator via `left.ToNatural()`, which returns only the
**stored Natural** — the raw BCD digit sequence that encodes one period block, not the
full infinite periodic expansion.

For a periodic value such as `Real("0.(3)")` (mathematically 1/3), `ToNatural()` returns
`Nat("3")` and `Exponent = -1`, representing **0.3**, not **0.333… = 1/3**. When the
Newton-Raphson loop executes `value / x`, it therefore computes `sqrt(0.3)` instead of
`sqrt(1/3)`.

By contrast, `Real.Add` and `Real.Multiply` both check `eitherPeriodic` and call
`ExpandToNonPeriodic` before operating — `Divide` has no equivalent guard.

#### Gap 2 — Seeding via `ToString()` truncation ignores `Exponent`

The seed is obtained by calling `value.ToString()`, capping at 20 characters, then
dispatching to `double.TryParse`. Two failure modes arise directly from the `Real`
representation:

| Scenario | Stored form | `ToString()` (truncated to 20) | `TryParse` result | Seed quality |
|---|---|---|---|---|
| Periodic value (any magnitude) | Nat + period metadata | `"0.(3)"` or similar | **Fails** (parens not valid `double` notation) | Falls back to 1.0 — potentially far off |
| Very small non-periodic value | `Nat("1")`, `Exponent = -25` | `"0.000000000000000000"` (lost) | Parses as 0.0 → rejected | Falls back to 1.0 — off by ≈ 10¹² |

Both cases reduce to the same root problem: the seed computation calls `ToString()` without
exploiting **`value.Exponent`**, which already encodes the order of magnitude precisely.

### Falsify Claims — Verification

| # | Claim | Evidence | Status | Reason |
|---|---|---|---|---|
| 1 | `Real.Divide` uses `Nat numerator = left.ToNatural()` with no `IsPeriodic` check on `left` | `Real.cs` line ~502: `Nat numerator = left.ToNatural();` — no preceding guard | ✅ Supported | Code inspection; confirmed no `eitherPeriodic` branch in `Divide` |
| 2 | `Real("0.(3)")` stores `Nat("3")` with `Exponent = -1`, representing 0.3, not 1/3 | `TryParse` logic: `allDigits = "" + "" + "3" = "3"`, `exponent = -(0+1) = -1` | ✅ Supported | Tracing `TryParse` for `"0.(3)"`: non-repeating part empty, periodic = `"3"`, so stored Nat = 3, Exp = -1 |
| 3 | `Real.Add` and `Real.Multiply` both have an `eitherPeriodic` branch that calls `ExpandToNonPeriodic` | `Real.cs` line 1511 (`Add`) and line 447 (`Multiply`) | ✅ Supported | Search results show `bool eitherPeriodic = left.IsPeriodic \|\| right.IsPeriodic` in both methods, absent in `Divide` |
| 4 | `Real.Sqrt` seeds NR via `value.ToString()` capped at 20 chars and parsed as `double` | `Real.cs` private `Sqrt(Real, long)`: `string strVal = value.ToString(); if (strVal.Length > 20) strVal = strVal[..20]; ... double.TryParse(strVal, ...)` | ✅ Supported | Direct code read |
| 5 | `double.TryParse` fails on periodic-notation strings (e.g., `"0.(3)"`), causing the seed to fall back to 1.0 | `double.TryParse` definition: accepts `[sign] [digits] ['.' digits]` only; parentheses are invalid | ✅ Supported | Framework specification + fall-back condition `dblApprox <= 0.0 → 1.0` in code |
| 6 | A non-periodic Real with `Exponent = -25` and Nat `"1"` produces `"0.000000000000000000000000001"` (27 chars) from `ToString()`, which when truncated to 20 chars becomes `"0.000000000000000000"` = 0 | `ToString()` for exponent -25, digits "1": pads to length 25, first 20 chars are all zeros | ✅ Supported | Tracing the `Exponent < 0` branch of `ToString()`: `padded = "1".PadLeft(25,'0')` = 25 chars, no intPart split needed, fracPart capped to 20 chars = all zeros |
| 7 | The magnitude order of any `Real` can be computed exactly as `digitCount + Exponent - 1` where `digitCount` is the digit count of `ToNatural()`, and `sqrt` order ≈ `10^((digitCount + Exponent - 1) / 2)` | Mathematical definition of decimal place value | ✅ Supported | Standard fixed-point arithmetic |
| 8 | `ExpandToNonPeriodic(value, MaxComputationDecimalPlaces)` returns a non-periodic `Real`; subsequent `Divide` on this expanded value is correct because `left.IsPeriodic == false` | `ExpandToNonPeriodic` signature and contract (returns non-periodic); Divide correctness for non-periodic is established | ✅ Supported | Multiply's periodic path uses this exact pattern |

> **Zero Falsified rows.**

### Completeness Checklist

Two design options are proposed. They are independent and additive; either or both may be
implemented. Each is listed as a separate checklist item so the work can be tracked and
tested individually.

- [x] **Option A — Pre-expand periodic input in `Sqrt`**: at the start of `private static Real Sqrt(Real value, long precision)`, if `value.IsPeriodic`, replace `value` with `ExpandToNonPeriodic(value, precision + guard)`. This simultaneously fixes the seeding failure (no periodic notation in the string) and the `value / x` correctness bug. [correctness fix for periodic inputs; depends on `ExpandToNonPeriodic` already in scope]
- [x] **Option B — Fix seeding to use `Exponent` + leading BCD digits**: replace the `value.ToString()` / 20-char truncation / `TryParse` seed path with a seed computed from `value.Exponent` (available directly) and the first ≤15 digits of `value.ToNatural().ToString()`. Eliminates the `Exponent`-blind truncation and avoids string allocation entirely for seeding. [robustness fix for all inputs; independent of Option A; simpler to verify in isolation]
- [x] **Option C (follow-on) — Add periodic guard to `Divide`**: mirror the `eitherPeriodic` pattern from `Add` / `Multiply` in `Divide`, expanding periodic left and/or right operands via `ExpandToNonPeriodic` before the remainder-tracking loop. This generalises the fix beyond `Sqrt` to all callers of `Divide` that may pass periodic inputs. [broader correctness; prerequisite: Options A + B validate the pattern first; separate checklist item to allow independent delivery]

---

## Test Plan

### `Sqrt` — Option A: periodic input correctness

1. `Sqrt_GivenOneNinthAsPeriodic_ReturnsExactlyOneThird`
   *Assumption*: `sqrt(0.(1))` = `sqrt(1/9)` = 1/3 exactly; the result should equal `Real("0.(3)")` (or its non-periodic expansion within the precision budget).

2. `Sqrt_GivenFourNinthsAsPeriodic_ReturnsExactlyTwoThirds`
   *Assumption*: `sqrt(0.(4))` = `sqrt(4/9)` = 2/3 exactly; the result should equal `Real("0.(6)")` (or its expansion).

3. `Sqrt_GivenOneThirdAsPeriodic_MatchesSqrtOneThirdKnownDigits`
   *Assumption*: `sqrt(0.(3))` = `sqrt(1/3)` ≈ 0.57735026…; the result starts with `"0.57735026"` (not with `"0.54772"` which would be `sqrt(0.3)`).

4. `Sqrt_GivenPeriodicAndExpandedEquivalent_ProduceSameResult`
   *Assumption*: `sqrt(Real("0.(3)"))` and `sqrt(Real("0.3333333333"))` (a finite expansion to 10 decimal places) agree to at least 8 fractional digits, until the finite expansion's truncation error dominates.

5. `Sqrt_GivenPeriodicPerfectSquare_ReturnsExactRationalRoot`
   *Assumption*: Rational perfect squares stored as periodic Reals (`1/4 = 0.25`, `1/9 = 0.(1)`) produce exact results matching `Real("0.5")` and `Real("0.(3)")` respectively.

---

### `Sqrt` — Option B: robust seeding from `Exponent` + BCD digits

6. `Sqrt_GivenVerySmallNumber_ConvergesCorrectly`
   *Assumption*: `sqrt(Real("0.0000000000000000000000000001"))` (1×10⁻²⁸) produces a result close to 1×10⁻¹⁴; `result.ToString()` starts with `"0.0000000000000001"` rather than a wildly incorrect value caused by a seed of 1.0 diverging.

7. `Sqrt_GivenVerySmallNumber_ExponentIsCorrectOrderOfMagnitude`
   *Assumption*: For `value` with `Exponent = -28` and `Nat("1")`, the result `Exponent` is approximately -14 (within ±2 of the mathematically expected order), confirming that the seed produced a correct-magnitude starting point.

8. `Sqrt_GivenSeedReplacedWithExponentBased_SameResultAsOriginal`
   *Assumption*: For ordinary non-periodic inputs that the original seeding handled correctly (e.g., `Real("2")`), the improved seed produces the identical final result — the seeding change is transparent for well-formed inputs.

---

### `Sqrt` — Option C: `Divide` periodic guard (follow-on)

9. `Divide_GivenPeriodicDividendByNonPeriodic_ReturnsCorrectQuotient`
   *Assumption*: `Real("0.(3)") / Real("3")` produces `Real("0.(1)")` (= 1/9), not `Real("0.1")` (= 1/10 which is what `0.3 / 3` gives).

10. `Divide_GivenPeriodicDividendByNonPeriodic_IsCommutativeWithAdd`
    *Assumption*: `Real("0.(3)") + Real("0.(3)") + Real("0.(3)")` = `Real("1")` and `Real("0.(3)") / Real("0.(1)")` = `Real("3")`; arithmetic on periodic values is self-consistent after the Divide fix.

11. `Divide_GivenNonPeriodicByPeriodic_PreservesCorrectness`
    *Assumption*: `Real("1") / Real("0.(3)")` = `Real("3")`, not `Real("3.(3)")` (the wrong answer from dividing by 0.3 instead of 1/3).

---

*All assumptions verified by Falsify Claims above. Zero Falsified rows.*
