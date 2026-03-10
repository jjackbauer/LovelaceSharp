# Sqrt 1000-Digit Investigation — Complete

> Updated March 9, 2026 (session 3). Progressive precision fix implemented and all tests passing.

---

## What Was Done

### Changes committed to source (across both sessions)

| File | Change |
|---|---|
| `Lovelace.Real/Real.cs` — private `Sqrt(Real, long)` | Replaced the old `bool raised` / conditional guard-digit block with an unconditional guard-digit block. Added `TruncateFracDigits` local function to strip guard tail before returning. |
| `Lovelace.Real/Real.cs` — `_localMaxComputationDecimalPlaces` | Added `AsyncLocal<long?>` field to hold a per-call-stack precision override. |
| `Lovelace.Real/Real.cs` — `MaxComputationDecimalPlaces` getter | Now reads the `AsyncLocal` first; falls back to the global `_maxComputationDecimalPlaces`. Setter still writes to the global field (preserving public API). |
| `Lovelace.Real/Real.cs` — `WithLocalPrecision` + `PrecisionScope` | New private helper + `readonly struct` disposable that sets / restores the `AsyncLocal` around a `using` scope. |
| `Lovelace.Real/Real.cs` — `Sqrt(Real, long)` body | Rewrote to use `using var _precisionScope = WithLocalPrecision(workingPrecision)` and removed the old `savedMax`/`finally` pattern. Stall-detection and convergence-check loop retained unchanged. |
| `Lovelace.Real.Tests/RealSqrtTests.cs` | Added 9 new functional tests for the 1000-digit precision requirement (see below). |

### New tests added

```
Sqrt_GivenPerfectSquare_RemainsExactWithGuardDigits   [Theory: 4,9,16,25]
Sqrt_GivenQuarter_ReturnsExactlyHalf
Sqrt_GivenTwo_Matches1000KnownDigitsOfSqrtTwo
Sqrt_GivenThree_Matches1000KnownDigitsOfSqrtThree
Sqrt_GivenFive_Matches1000KnownDigitsOfSqrtFive
Sqrt_GivenTen_Matches1000KnownDigitsOfSqrtTen
Sqrt_GivenTwo_ResultHasExactly1000FractionalDigits
Sqrt_GivenTwo_SquaredApproximatesInput
Sqrt_GivenIrrational_ResultIsNotPeriodicAt1000Digits
Sqrt_GivenPositiveInput_ResultIsPositiveAt1000Digits
```

### Current test state

- **All pre-existing Sqrt tests**: pass.
- **All new 1000-digit tests**: **FAIL** — produce only ~15 correct fractional digits instead of 1000.

Failing sample output (√2):
```
Actual:   "1.4142135623730925122004221..."
Expected: "1.4142135623730950488016887..."
```
Note the divergence begins at fractional digit 14 (`25` vs `50`). This is consistent with a double-precision (~15 digit) result leaking through rather than the arbitrary-precision Newton-Raphson converging.

---

## Session 2 — Status Update

### Hypothesis A (race condition) — **Refuted / Irrelevant**

The `AsyncLocal<long?>` fix was implemented and is in source. The race condition is eliminated. Running the 1000-digit test in complete isolation (`--filter "FullyQualifiedName~Sqrt_GivenTwo_Matches1000"` with a fresh build) still produces the wrong result:

```
Actual:   "1.414213562373092512200422181052424751331..."
Expected: "1.414213562373095048801688724209698078569..."
```

The failure persists in a single-test, single-thread run. Concurrency was never the cause.

---

## True Root Cause — Division precision budget exhausted by x_n's digit count

The `Divide` method's fractional-digit generation loop is:

```csharp
while (!Nat.IsZero(remainder) && position < MaxComputationDecimalPlaces)
```

`position` starts at 0 and counts the total number of fractional digits produced in the loop output (stored in `fracStr`). The final result exponent is:

```
resultExponent = -fracLen + exponentAdjustment
               = -MaxComputationDecimalPlaces + (left.Exponent - right.Exponent)
```

**Key observation**: if the divisor (`right`) has `k` fractional digits, then `right.Exponent = -k` and `exponentAdjustment = left.Exponent + k`. For `value = Real(2)` (Exponent = 0) and `x_n` with `k` fractional digits:

```
exponentAdjustment = 0 - (-k) = k
resultExponent     = -MaxComputationDecimalPlaces + k
significant frac digits in quotient = MaxComputationDecimalPlaces - k
```

**Iteration 0**: seed `x_0` has `k = 16` fractional digits (from `double.ToString("R")`).  
`2/x_0` produces `1050 - 16 = 1034` significant fractional digits. ✓  
`x_1 = (x_0 + 2/x_0)/2` inherits the longer operand's exponent → `x_1` has **1034 fractional digits** (Exponent = −1034).

**Iteration 1**: now `k = 1034` (x_1 has 1034 frac digits).  
`2/x_1` produces `1050 - 1034 = 16` significant fractional digits. ✗  
`x_2` is now the average of a 1034-digit value and a 16-digit value, giving a result that only refines positions 1–16. Newton-Raphson stagnates at 16-digit accuracy from this point forward.

**Root cause in one sentence**: each iteration's output carries the previous iteration's full digit count as a "shift" into the next division, consuming the entire `MaxComputationDecimalPlaces` budget. After the first iteration, only 16 significant digits survive in each subsequent quotient — the same as the double seed.

The stall-detection condition (`diff >= prevDiff`) then fires within a handful of iterations because the garbage digits at positions 17–1034 do not monotonically decrease.

---

## Proposed Fix — Progressive precision (doubling strategy)

Instead of a single global working precision, each Newton-Raphson iteration is computed at a growing target precision that doubles from the seed to the final goal. The division precision for iteration `i` is set to `x_prev.fracDigits + iterationTarget` so that the quotient has exactly `iterationTarget` significant fractional digits.

### Algorithm

```
seed precision s₀ = 16 (from double)
target T = precision + 50   (e.g. 1050 for precision = 1000)
x = new Real(Math.Sqrt(dblApprox))

currentTarget = s₀
while currentTarget < T:
    currentTarget = min(currentTarget × 2, T)
    xFracDigits  = -x.Exponent          // current stored frac digits
    divPrecision = xFracDigits + currentTarget
    using WithLocalPrecision(divPrecision):
        x = (x + value / x) / two

// x now has T fractional digits; strip guard digits and return
return Normalize(TruncateFracDigits(x, precision))
```

### Concrete digit trace (target = 1050, seed = 16)

| Iter | Starting `xFracDigits` | `currentTarget` | `divPrecision` | `x_new.fracDigits` |
|------|------------------------|-----------------|----------------|---------------------|
| 1    | 16                     | 32              | 48             | 32                  |
| 2    | 32                     | 64              | 96             | 64                  |
| 3    | 64                     | 128             | 192            | 128                 |
| 4    | 128                    | 256             | 384            | 256                 |
| 5    | 256                    | 512             | 768            | 512                 |
| 6    | 512                    | 1024            | 1536           | 1024                |
| 7    | 1024                   | 1050            | 2074           | 1050                |

After 7 iterations, `x` has 1050 fractional digits all accurately representing √2. `TruncateFracDigits(x, 1000)` strips the 50 guard digits, leaving exactly 1000 correct fractional digits.

### Why the stall condition is no longer needed

With progressive precision, each iteration produces exactly `currentTarget` correct fractional digits in `x_new`. The error halves super-linearly (quadratic convergence), so `diff` is monotonically decreasing by construction. The stall condition should be **removed** — it was a fallback for the broken flat-precision case and would fire spuriously here if kept (because the diff at positions beyond the current target may not be monotone).

### Convergence condition

Replace the stall-detection loop with a simple counted loop: run until `currentTarget >= T`. After the loop, `x` is known to be correct to `T` fractional digits. No stall guard is needed.

---

## Files Changed (Session 3)

1. **`Lovelace.Real/Real.cs`** — `Sqrt(Real, long)`: Progressive precision (doubling strategy) implemented. Stall-detection removed. Per-iteration `WithLocalPrecision` scoping. `TruncateFracDigits` truncation. Early exact-convergence check for perfect squares.
2. **`Lovelace.Real.Tests/RealSqrtTests.cs`** — all 10 new 1000-digit tests written (sessions 1–2); no changes needed in session 3.
3. **`.github/requirements/Lovelace.Real.Sqrt.md`** — all checklist items marked complete.
4. **`Lovelace.Real/README.md`** — added `Sqrt` to public API table and Sqrt requirements link.
5. **`README.md`** — added Sqrt requirements row to the status table.

---

## Final Checklist State

- [x] Remove debug string conversions from Newton-Raphson loop
- [x] Always add guard digits during Sqrt internal computation
- [x] Truncate Sqrt result to the requested `precision` fractional digits
- [x] Verify √2 matches 1000 known reference fractional digits
- [x] Verify √3 matches 1000 known reference fractional digits
- [x] Verify √5 matches 1000 known reference fractional digits
- [x] Verify √10 matches 1000 known reference fractional digits
- [x] Verify perfect squares remain exact under guard-digit computation
- [x] Verify fractional input √0.25 returns exactly 0.5
- [x] Verify self-consistency: `Sqrt(v) * Sqrt(v)` approximates `v` within 10^(-999)
- [x] Verify result contract: `IsPeriodic = false`, `IsNegative = false` at 1000-digit output
