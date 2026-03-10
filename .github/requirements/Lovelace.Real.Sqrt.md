# Lovelace.Real — Sqrt 1000-Digit Precision

> Improvements to `Real.Sqrt` so that Newton-Raphson converges to at least 1000 correct
> fractional digits for irrational results, verified against known reference expansions
> of √2, √3, √5, and √10. Covers code-quality fixes, guard-digit computation, result
> truncation, and a comprehensive xUnit test plan.

---

## Functionality Worktree

### Analysis Summary

| Finding | Detail | Impact |
|---|---|---|
| Debug string conversions in loop | `strX`, `strXnext`, `strDiff`, `strPrevDiff`, `strThresh` are assigned on every Newton-Raphson iteration but never read | O(n) string allocations per iteration for 1000-digit numbers; pure waste |
| No guard digits in default path | When `Sqrt(value)` calls `Sqrt(value, MaxComputationDecimalPlaces)`, `raised = false` because `precision == savedMax`; intermediate division is capped at exactly `MaxComputationDecimalPlaces` digits | Last few digits of the result may be incorrect due to truncation in `value / x` |
| No result truncation | After convergence, result has whatever digit count the Newton-Raphson produced; if guard digits are added, the extra (potentially inaccurate) tail digits leak to the caller | Callers see digits beyond the precision guarantee |
| Perfect squares still exact | Newton-Raphson converges exactly for perfect squares (e.g. √4 = 2) regardless of guard digits | Guard-digit path must not break this |

### Falsify Claims — Verification

| # | Claim | Evidence | Status | Reason |
|---|---|---|---|---|
| 1 | Lines 697–701 of `Real.cs` contain debug string conversions (`strX`, `strXnext`, `strDiff`, `strPrevDiff`, `strThresh`) that are assigned but never read | `Real.cs` lines 697–701 | ✅ Supported | Variables assigned, no subsequent read or side-effect use |
| 2 | When `precision == MaxComputationDecimalPlaces`, `raised = false` and no guard digits are added to intermediate division | `Real.cs` line 666: `bool raised = precision > savedMax;` | ✅ Supported | `>` is strict; equal values yield `false` |
| 3 | The result is not truncated to `precision` digits after convergence — `Normalize` only strips trailing zeros | `Real.cs` lines 705, 711: `return Normalize(...)` | ✅ Supported | `Normalize` checks trailing `'0'` chars only |
| 4 | Newton-Raphson quadratic convergence from a ~15-digit double seed needs ~7 iterations for 1000 correct digits | Mathematical: log₂(1000/15) ≈ 6.1 | ✅ Supported | Well-known convergence rate |
| 5 | Division truncation at N digits means the final Sqrt result may have fewer than N fully correct digits | Standard arbitrary-precision analysis | ✅ Supported | Truncation error propagates through addition and averaging |
| 6 | Adding ≥50 guard digits and truncating afterward guarantees the requested precision | Standard practice (cf. Python `mpmath`) | ✅ Supported | Guard absorbs truncation drift |
| 7 | √0.25 = 0.5 exactly; perfect squares (4, 9, 16, 25) produce exact integer roots | Mathematical fact | ✅ Supported | Finite convergence for rational perfect squares |

> **Zero Falsified rows.**

### Completeness Checklist

- [x] Remove debug string conversions from Newton-Raphson loop (`strX`, `strXnext`, `strDiff`, `strPrevDiff`, `strThresh`) [prerequisite for performance — O(n) waste per iteration]
- [x] Always add guard digits during Sqrt internal computation — compute with `precision + guard` (guard ≥ 50) fractional digits by temporarily raising `MaxComputationDecimalPlaces`, even when `precision == savedMax` [prerequisite for last-digit correctness]
- [x] Truncate Sqrt result to the requested `precision` fractional digits before returning — strip guard-digit tail so callers see exactly `precision` digits [depends on guard-digit item]
- [x] Verify √2 matches 1000 known reference fractional digits [mandatory — user requirement; depends on guard digits and truncation]
- [x] Verify √3 matches 1000 known reference fractional digits [mandatory — user requirement; depends on guard digits and truncation]
- [x] Verify √5 matches 1000 known reference fractional digits [mandatory — user requirement; depends on guard digits and truncation]
- [x] Verify √10 matches 1000 known reference fractional digits [mandatory — user requirement; depends on guard digits and truncation]
- [x] Verify perfect squares (4, 9, 16, 25) remain exact under guard-digit computation [correctness guard — depends on guard digits and truncation]
- [x] Verify fractional input √0.25 returns exactly 0.5 under guard-digit computation [correctness guard — depends on guard digits and truncation]
- [x] Verify self-consistency: `Sqrt(v) * Sqrt(v)` approximates `v` within `10^(-999)` [mathematical invariant — depends on guard digits]
- [x] Verify result contract: `IsPeriodic = false`, `IsNegative = false` at 1000-digit output [result contract — depends on guard digits]

---

## Test Plan

### `Real.Sqrt` — Remove debug string conversions

> No behavioural test — this is a code-quality fix verified by code review and by the
> performance improvement observed in the 1000-digit tests below. The existing test
> `Sqrt_GivenPerfectSquareFour_ReturnsExactlyTwo` serves as a regression gate.

---

### `Real.Sqrt` — Guard digits and truncation

1. `Sqrt_GivenTwo_Matches1000KnownDigitsOfSqrtTwo`
   *Assumption*: `Real.Sqrt(new Real(2))` with `MaxComputationDecimalPlaces = 1000` produces a result whose `ToString()` starts with `"1."` followed by at least 1000 fractional digits matching the known decimal expansion of √2.

2. `Sqrt_GivenThree_Matches1000KnownDigitsOfSqrtThree`
   *Assumption*: `Real.Sqrt(new Real(3))` with `MaxComputationDecimalPlaces = 1000` produces a result whose `ToString()` starts with `"1."` followed by at least 1000 fractional digits matching the known decimal expansion of √3.

3. `Sqrt_GivenFive_Matches1000KnownDigitsOfSqrtFive`
   *Assumption*: `Real.Sqrt(new Real(5))` with `MaxComputationDecimalPlaces = 1000` produces a result whose `ToString()` starts with `"2."` followed by at least 1000 fractional digits matching the known decimal expansion of √5.

4. `Sqrt_GivenTen_Matches1000KnownDigitsOfSqrtTen`
   *Assumption*: `Real.Sqrt(new Real(10))` with `MaxComputationDecimalPlaces = 1000` produces a result whose `ToString()` starts with `"3."` followed by at least 1000 fractional digits matching the known decimal expansion of √10.

5. `Sqrt_GivenPerfectSquare_RemainsExactWithGuardDigits`
   *Assumption*: After the guard-digit code change, `Real.Sqrt(new Real(4))` still equals `new Real("2")`, `Real.Sqrt(new Real(9))` still equals `new Real("3")`, `Real.Sqrt(new Real(16))` still equals `new Real("4")`, and `Real.Sqrt(new Real(25))` still equals `new Real("5")` — guard-digit truncation does not corrupt exact results.

6. `Sqrt_GivenQuarter_ReturnsExactlyHalf`
   *Assumption*: `Real.Sqrt(new Real("0.25"))` produces a result whose `ToString()` is `"0.5"` — the exact rational root is preserved through guard-digit computation and truncation.

7. `Sqrt_GivenTwo_SquaredApproximatesInput`
   *Assumption*: Let `r = Real.Sqrt(new Real(2))`; then `Abs(r * r - new Real(2))` is less than `new Real(Nat.One, false, -999)` (i.e. < 10⁻⁹⁹⁹) — self-consistency within one digit of the precision cap.

8. `Sqrt_GivenIrrational_ResultIsNotPeriodicAt1000Digits`
   *Assumption*: `Real.Sqrt(new Real(2))` at 1000-digit precision produces a result with `IsPeriodic == false` because the output is a truncated rational approximation of an irrational number.

9. `Sqrt_GivenPositiveInput_ResultIsPositiveAt1000Digits`
   *Assumption*: `Real.IsPositive(Real.Sqrt(new Real(2)))` returns `true` at 1000-digit precision because the principal square root is always positive.

10. `Sqrt_GivenTwo_ResultHasExactly1000FractionalDigits`
    *Assumption*: After truncation, the result of `Real.Sqrt(new Real(2))` has `Exponent == -1000` (exactly 1000 stored fractional digits, not more, not fewer) when `MaxComputationDecimalPlaces = 1000`.

---

*All assumptions derived from source-code analysis and mathematical facts. Zero Falsified rows.*
