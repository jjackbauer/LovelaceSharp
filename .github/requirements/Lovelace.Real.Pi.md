# Pi Computation Feature — Lovelace.Real & Lovelace.Console

> Adds `Real.Sqrt(Real value)` (Newton-Raphson) and `Real.Pi(long digits)` (Chudnovsky algorithm)
> to **Lovelace.Real**, and exposes `sqrt()` and `pi()` as built-in functions in
> **Lovelace.Console**'s REPL evaluator. `Sqrt` is a dependency of `Pi`.

---

## Functionality Worktree

### Feature Scope

| Component | Deliverable | Scope |
|---|---|---|
| `Lovelace.Real` | `Real.Sqrt(Real value)` static method | Arbitrary-precision square root via Newton-Raphson (Heron's method); precision controlled by `MaxComputationDecimalPlaces` |
| `Lovelace.Real` | Internal `Sqrt(Real value, long precision)` overload | Same algorithm but iterates until the result agrees to the caller-supplied `precision` decimal places, with no cap; used exclusively by `Pi` to compute `√10005` at `digits + 10` guard-digit precision |
| `Lovelace.Real` | Sqrt input validation | Throw `ArithmeticException` for negative input; return zero immediately for zero input |
| `Lovelace.Real` | `Real.Pi(long digits)` static method | Compute π to `digits` decimal places via the Chudnovsky algorithm (~14.18 digits per term); requires the internal `Sqrt` overload |
| `Lovelace.Real` | Guard-digit computation | `Pi` internally computes with `digits + 10` extra decimal places, including calling the internal `Sqrt` overload with `digits + 10` precision, and truncates to `digits` fractional places before returning |
| `Lovelace.Real` | Pi input validation | Throw `ArgumentOutOfRangeException` for `digits ≤ 0` or `digits > MaxComputationDecimalPlaces` |

| `Lovelace.Console` | `"sqrt"` registration in `EvaluateCall` | Wire the name `"sqrt"` to `BuiltinSqrt` in the switch |
| `Lovelace.Console` | `BuiltinSqrt(CallExpr call)` handler | Accept exactly 1 argument of any numeric kind; widen to Real; return Real |
| `Lovelace.Console` | `"pi"` registration in `EvaluateCall` | Wire the name `"pi"` to `BuiltinPi` in the switch |
| `Lovelace.Console` | `BuiltinPi(CallExpr call)` handler | Accept 0 or 1 Natural/Integer argument; reject Real and wrong arity |

### Completeness Checklist

- [x] `Real.Sqrt(Real value)` implementation — static method using Newton-Raphson iteration (`x_{n+1} = (x_n + value/x_n) / 2`), seeded from the `double` approximation, iterated until two consecutive results agree to `MaxComputationDecimalPlaces` digits; delegates to the internal `Sqrt(Real, long)` overload with `MaxComputationDecimalPlaces` [prerequisite for Console sqrt and for the internal overload]
- [x] Internal `Sqrt(Real value, long precision)` implementation — private static overload with the same Newton-Raphson algorithm but iterates until two consecutive results agree to `precision` decimal places, imposing no upper cap on `precision`; input validation (negative → `ArithmeticException`, zero → `Real.Zero`) is shared with the public overload [prerequisite for Real.Pi]
- [x] Sqrt — throw `ArithmeticException` when `value` is negative [depends on Real.Sqrt]
- [x] Sqrt — return `Real.Zero` immediately when `value` is zero [depends on Real.Sqrt]
- [x] Sqrt — result contract: `IsPeriodic = false`, `IsNegative = false` for all non-negative input [depends on Real.Sqrt]
- [x] `Real.Pi(long digits)` implementation — static method using the Chudnovsky algorithm: `π = 426880·√10005 / Σ_k [ (-1)^k·(6k)!·(13591409 + 545140134k) / ((3k)!·(k!)³·640320^(3k)) ]`; accumulates terms until the term magnitude falls below 10^(-digits-10); calls the internal `Sqrt(Real, long)` overload with precision `digits + 10` for `√10005` so guard digits are not silently truncated [depends on internal Sqrt overload, prerequisite for all Console pi items]
- [x] Pi — throw `ArgumentOutOfRangeException` when `digits ≤ 0` [depends on Real.Pi]
- [x] Pi — compute internally with `digits + 10` guard digits (including passing `digits + 10` to the internal `Sqrt` overload) and truncate to `digits` fractional places before returning [depends on Real.Pi]
- [x] Pi — result contract: `PeriodLength = 0`, `IsNegative = false`, `Exponent = -digits` [depends on Real.Pi]
- [x] Register `"sqrt"` in `Evaluator.EvaluateCall` switch [depends on Real.Sqrt]
- [x] `BuiltinSqrt` — exactly 1 argument; widen to Real; reject wrong arity with `InvalidOperationException` [depends on Console sqrt registration]
- [x] Register `"pi"` in `Evaluator.EvaluateCall` switch [depends on Real.Pi]
- [x] `BuiltinPi` — 0 arguments path uses `Real.DisplayDecimalPlaces` as the digit count [depends on Console pi registration]
- [x] `BuiltinPi` — 1 Natural or Integer argument path uses its value as the digit count [depends on BuiltinPi zero-arg path]
- [x] `BuiltinPi` — reject `Real` argument with `InvalidOperationException` [depends on BuiltinPi one-arg path]
- [x] `BuiltinPi` — reject argument count ≠ 0 and ≠ 1 with `InvalidOperationException` [depends on BuiltinPi one-arg path]

---

## Test Plan

### `Real.Sqrt`

1. `Sqrt_GivenPerfectSquareFour_ReturnsExactlyTwo`
   *Assumption*: `Real.Sqrt(new Real(4))` produces a result whose `ToString()` is `"2"` (or `"2.000..."`), because √4 = 2 exactly.

2. `Sqrt_GivenPerfectSquareNine_ReturnsExactlyThree`
   *Assumption*: `Real.Sqrt(new Real(9))` produces a result whose integer part is `"3"` and whose fractional digits are all zero, because √9 = 3 exactly.

3. `Sqrt_GivenOne_ReturnsOne`
   *Assumption*: `Real.Sqrt(Real.One)` returns a value equal to `Real.One` because √1 = 1.

4. `Sqrt_GivenZero_ReturnsZero`
   *Assumption*: `Real.Sqrt(Real.Zero)` returns `Real.Zero` immediately without iterating, because √0 = 0.

5. `Sqrt_GivenTwo_MatchesKnownDigitsOfSqrtTwo`
   *Assumption*: `Real.Sqrt(new Real(2))` produces a result whose `ToString()` starts with `"1.41421356237"`, matching the known decimal expansion of √2 to at least 11 fractional digits.

6. `Sqrt_GivenIrrational_ResultIsNotPeriodic`
   *Assumption*: For any irrational input (e.g. `new Real(2)`), `result.IsPeriodic == false` because the result is a truncated rational approximation, not a true repeating decimal.

7. `Sqrt_GivenPositiveInput_ResultIsPositive`
   *Assumption*: For any positive input, `Real.IsPositive(result) == true` because the principal square root is always positive.

8. `Sqrt_GivenNegativeInput_ThrowsArithmeticException`
   *Assumption*: `Real.Sqrt(new Real(-1))` throws `ArithmeticException` because square roots of negative numbers are not in ℝ.

9. `Sqrt_GivenExplicitPrecisionExceedingMax_ProducesResultWithRequestedPrecision`
   *Assumption*: The internal `Sqrt(Real value, long precision)` overload (exercised indirectly by `Real.Pi(MaxComputationDecimalPlaces)`) produces a result with at least `MaxComputationDecimalPlaces + 10` significant fractional digits. Verified by calling `Real.Pi(Real.MaxComputationDecimalPlaces)` and confirming no truncation occurs and the result matches the known leading digits of π to `MaxComputationDecimalPlaces` places.

### `Real.Pi`

10. `Pi_GivenTenDigits_ReturnsCorrectFirstTenFractionalDigits`
   *Assumption*: `Real.Pi(10)` produces a result whose `ToString()` starts with `"3.1415926535"`, matching the first 10 fractional digits of π.

11. `Pi_GivenOneDigit_ReturnsValueStartingWithThreePointOne`
    *Assumption*: `Real.Pi(1)` returns a Real whose `ToString()` starts with `"3.1"`, confirming the first fractional digit of π is 1.

12. `Pi_GivenFiftyDigits_MatchesKnownReference`
    *Assumption*: `Real.Pi(50)` produces a result whose `ToString()` contains the known first 50 fractional digits of π (`14159265358979323846264338327950288419716939937510`).

13. `Pi_GivenZeroDigits_ThrowsArgumentOutOfRangeException`
    *Assumption*: Calling `Real.Pi(0)` throws `ArgumentOutOfRangeException` because `digits` must be strictly positive.

14. `Pi_GivenNegativeDigits_ThrowsArgumentOutOfRangeException`
    *Assumption*: Calling `Real.Pi(-1)` throws `ArgumentOutOfRangeException` because negative digit counts are nonsensical.

16. `Pi_GivenAnyValidDigitCount_ResultIsPositive`
    *Assumption*: For any `digits > 0`, the returned `Real` satisfies `Real.IsPositive(result) == true` because π > 0.

17. `Pi_GivenAnyValidDigitCount_ResultIsNotPeriodic`
    *Assumption*: For any `digits > 0`, `result.IsPeriodic == false` because π is transcendental and the result is a finite truncation, not a repeating decimal.

18. `Pi_GivenDigits_ResultExponentEqualsNegativeDigits`
    *Assumption*: `Real.Pi(n).Exponent == -n` because all `n` computed decimal places are stored as fractional digits in the backing store.

### `BuiltinSqrt` (Lovelace.Console)

19. `BuiltinSqrt_GivenNaturalArgument_ReturnsRealSquareRoot`
    *Assumption*: `sqrt(4)` with a Natural argument returns a Real equal to `Real.Sqrt(new Real(4))`, i.e., 2.

20. `BuiltinSqrt_GivenIntegerArgument_ReturnsRealSquareRoot`
    *Assumption*: `sqrt(9)` with a positive Integer argument is accepted and returns the same result as passing a Natural with the same value.

21. `BuiltinSqrt_GivenRealArgument_ReturnsRealSquareRoot`
    *Assumption*: `sqrt(2.0)` with a Real argument is accepted and returns `Real.Sqrt(new Real(2))`.

22. `BuiltinSqrt_GivenNoArguments_ThrowsInvalidOperationException`
    *Assumption*: `sqrt()` with zero arguments throws `InvalidOperationException` because `sqrt` requires exactly 1 argument.

23. `BuiltinSqrt_GivenTooManyArguments_ThrowsInvalidOperationException`
    *Assumption*: `sqrt(4, 9)` with two arguments throws `InvalidOperationException` because `sqrt` accepts exactly 1 argument.

### `BuiltinPi` (Lovelace.Console)

24. `BuiltinPi_GivenNoArguments_ReturnsRealWithDisplayDecimalPlacesDigits`
    *Assumption*: `pi()` with 0 arguments delegates to `Real.Pi(Real.DisplayDecimalPlaces)` and returns the resulting Real value.

25. `BuiltinPi_GivenNaturalArgument_ReturnsRealWithRequestedDigits`
    *Assumption*: `pi(10)` with a Natural argument returns a Real whose fractional-digit count equals the argument's value (10 in this case).

26. `BuiltinPi_GivenIntegerArgument_ReturnsRealWithRequestedDigits`
    *Assumption*: `pi(10)` with an Integer argument (positive) is accepted and returns the same Real as the Natural case with the same digit value.

27. `BuiltinPi_GivenRealArgument_ThrowsInvalidOperationException`
    *Assumption*: `pi(3.0)` with a Real argument throws `InvalidOperationException` because the digit count must be a whole number expressed as Natural or Integer.

28. `BuiltinPi_GivenTooManyArguments_ThrowsInvalidOperationException`
    *Assumption*: `pi(10, 20)` with two arguments throws `InvalidOperationException` because `pi()` accepts at most one argument.

---

*All assumptions verified by Falsify Claims. Zero Falsified rows.*
