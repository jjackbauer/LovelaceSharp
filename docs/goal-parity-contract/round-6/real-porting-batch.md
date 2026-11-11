# Round 6 — Lovelace.Real.Tests porting batch

Census oracle: `1cc28f255eca6029849f880949fc7be1d40ab8af` (6,209 C# test cases; 2537 in Lovelace.Real.Tests).
Baseline: 6 Real cases mapped (0.24% of the project; 262 overall = 4.22%). Result: **1201 Real cases ported (47.34% of the project; 1,457 overall = 23.47%)**, of which **1195 are new verified mappings** backed by **1195 new Rust tests**.

Every new test lives in the appended `#[cfg(test)] mod real_parity_batch` of `rust/lovelace-real/src/lib.rs`; no production line was modified for this batch (the file's non-test prefix is byte-identical to the one this session started with).

## Mappings added per C# class

| C# test class | Cases | Ported (incl. pre-existing) | New mappings | Refused | Coverage |
|---|---:|---:|---:|---:|---:|
| `RealAddPeriodicExactTests` | 1732 | 726 | 726 | 1006 | 41.92% |
| `RealMultiplyPeriodicExactTests` | 400 | 307 | 307 | 93 | 76.75% |
| `LReal64Tests` | 58 | 0 | 0 | 58 | 0.00% |
| `LReal128Tests` | 26 | 0 | 0 | 26 | 0.00% |
| `RealConstructorTests` | 25 | 10 | 10 | 15 | 40.00% |
| `RealTrigPrecisionOracleTests` | 25 | 0 | 0 | 25 | 0.00% |
| `RealToStringTests` | 23 | 18 | 12 | 5 | 78.26% |
| `RealDivideTests` | 21 | 18 | 18 | 3 | 85.71% |
| `RealParseTests` | 21 | 20 | 20 | 1 | 95.24% |
| `RealSqrtTests` | 21 | 20 | 20 | 1 | 95.24% |
| `RealTrigCertifiedPrefixTests` | 17 | 0 | 0 | 17 | 0.00% |
| `RealAddTests` | 12 | 11 | 11 | 1 | 91.67% |
| `RealPropertiesTests` | 12 | 3 | 3 | 9 | 25.00% |
| `RealTrigPiResolutionPropertyTests` | 12 | 0 | 0 | 12 | 0.00% |
| `RealPowZeroBaseTests` | 11 | 4 | 4 | 7 | 36.36% |
| `RealPiTests` | 10 | 7 | 7 | 3 | 70.00% |
| `RealTrigFastPathTests` | 10 | 0 | 0 | 10 | 0.00% |
| `RealExactQuotientPropertyTests` | 9 | 0 | 0 | 9 | 0.00% |
| `RealModulusTests` | 8 | 0 | 0 | 8 | 0.00% |
| `RealMultiplyTests` | 8 | 8 | 8 | 0 | 100.00% |
| `RealSubtractTests` | 7 | 7 | 7 | 0 | 100.00% |
| `RealDivideLeadingZeroRunTests` | 6 | 0 | 0 | 6 | 0.00% |
| `RealInvertTests` | 6 | 6 | 6 | 0 | 100.00% |
| `RealNegationTests` | 6 | 6 | 6 | 0 | 100.00% |
| `RealDecrementTests` | 5 | 5 | 5 | 0 | 100.00% |
| `RealIncrementTests` | 5 | 5 | 5 | 0 | 100.00% |
| `RealPowTests` | 5 | 5 | 5 | 0 | 100.00% |
| `RealSqrtBatchTests` | 5 | 0 | 0 | 5 | 0.00% |
| `RealSqrtPeriodicTests` | 5 | 5 | 5 | 0 | 100.00% |
| `RealAsyncLocalTests` | 4 | 0 | 0 | 4 | 0.00% |
| `RealPeriodicIntegerValuePropertyTests` | 4 | 3 | 3 | 1 | 75.00% |
| `RealDivideExactnessPropertyTests` | 3 | 0 | 0 | 3 | 0.00% |
| `RealPiSegmentTests` | 3 | 0 | 0 | 3 | 0.00% |
| `RealPiToAsyncTests` | 3 | 2 | 2 | 1 | 66.67% |
| `RealSqrtAsyncTests` | 3 | 2 | 2 | 1 | 66.67% |
| `RealSqrtSeedingTests` | 3 | 3 | 3 | 0 | 100.00% |
| `RealTrigSymmetryTests` | 3 | 0 | 0 | 3 | 0.00% |
| **total** | **2537** | **1201** | **1195** | **1336** | **47.34%** |

The 1195 new mappings break down as:

| Corpus | New mappings | Shape |
|---|---:|---|
| `RealAddPeriodicExactTests.Add_ExactRationalPair_Property_DenotesTheExactRationalSum` | 363 | theory rows (p,q,r,s); exact-rational and Real equality |
| `RealAddPeriodicExactTests.Subtract_ExactRationalPair_Property_DenotesTheExactRationalDifference` | 363 | theory rows (p,q,r,s); exact-rational and Real equality |
| `RealMultiplyPeriodicExactTests.Multiply_PeriodicQuotientTimesItsDivisor_Property_IsExactlyTheDividend` | 300 | theory rows (p, divisor); (p/q)*q == p as Real and rational |
| Other rows: the add/subtract/multiply/negate/increment/decrement/invert/parse/constructor/ToString/sqrt/pi/pow/divide/periodic/sqrt-seeding/async facts and theory rows, plus the multiply audit shapes and facts | 169 | one test per census case (or per theory row), concrete value assertions |
| **total** | **1,195** | |

## How the ports were verified

- The C# assertion is the contract. Each generated corpus test reproduces the row's concrete `(p,q,r,s)` (or `(p,divisor)`) operands, computes the expected exact fraction **in the test** from those operands (lowest terms via a test-local gcd), and asserts (a) `to_rational()` equals that fraction and (b) the `Real` compares equal to the `Real` built from that fraction. The C# corpus guard (the left operand must be periodic) is reproduced through the canonical rendering (a period block renders in parentheses).
- The corpus is exercised at the C# default of 1000 computation places; the C# tests pin 1000/100 with `WithPrecision`, and Rust's `div` takes the digit budget explicitly, so `1000` is passed.
- Non-corpus tests reproduce the literal inputs and expected values of each C# fact/theory row (for example `1.5 + 2.3 == 3.8`, `Real::pi(50)` digits, the 1000-digit `sqrt(2)` prefix, `1/0.99 = 1.(01)`).
- `cargo test -p lovelace-real`: **1,219 passed, 0 failed** (24 pre-existing + 1,195 new). `cargo test --workspace`: **1,645 passed, 0 failed** (was 450).

## The one behavioural difference behind the large refusal group

The Rust crate converts a periodic value to a fraction incorrectly whenever the period starts after a non-empty preperiod. In `Real::to_fraction` (rust/lovelace-real/src/lib.rs:349-392) the term for the repeating block is multiplied by `10^period_start`:

```text
sum = int_part*den + pre*(10^k - 1) + per*10^s     // current
sum = int_part*den + pre*(10^k - 1) + per           // the value the digits denote
```

Observed in this tree before this batch's tests were written:

```text
Real::from_str("0.1(6)").to_rational()  -> 23/30      (true value 1/6)
Real::from_str("0.(3)").to_rational()   -> 1/3        (period starts at 0: correct)
Real::from_str("0.27(02)").to_rational() -> 2873/9900 (true value 107/396)
```

Because `add`/`sub`/`mul`/`cmp` all go through that conversion when an operand is periodic, every C# periodic-arithmetic row whose operand or result has `period_start > 0` fails the C# exact-rational assertion in Rust. Those rows are refused, not weakened. The rendering path is correct, which is why the 363 add / 363 subtract / 300 multiply rows with no preperiod (and the rendering-focused ToString rows) pass. Fixing this is a production change and out of scope for this batch; it is recorded here so the next round can decide.

## Reproductions that are observably equivalent but not accessor-identical

Nine ported rows assert a C# `Exponent`/`PeriodStart`/`PeriodLength` value. Rust exposes no accessor for those fields; the exact canonical rendering pins them (a terminating value keeps its stored scale, and a periodic value renders its preperiod, `(` block and `)` exactly where the metadata says). Those notes say so explicitly in `parity/manifest-map.json` (search "pins the stored scale"). For all other rows the assertion is the value itself.

## Refusals

Refusals are grouped by C# method; the census-id column gives the exact `#caseIndex` set of that method (commas separate indices, `-#` a run). Every refused case keeps its census id intact and remains `deferred` in `parity/manifest.toml`; no exclusion was requested.

### Reason codes

| Code | Meaning |
|---|---|
| `PREC` | Rust `Real::to_rational` (via `to_fraction`, rust/lovelace-real/src/lib.rs:349-392) mis-converts a periodic value whose period starts after a non-empty preperiod: `0.1(6)` and `1/6` are read back as 23/30, `0.27(02)` as 2873/9900. The C# assertion is on the exact rational the result denotes (and Real equality uses the same conversion), so the row cannot be reproduced without changing production code. |
| `NO_MODULUS` | Rust `Real` exposes no modulus/truncate operation, so `RealModulusTests` has no API to exercise. |
| `POW_REFUSAL` | C# asserts a typed `ArgumentOutOfRangeException` (with the integer path's message/param) or `NotImplementedException`; Rust `pow_int` returns `Option` (None for 0^-1) or computes 2^-3 = 0.125. The refusal type/message cannot be reproduced. |
| `NEG_SQRT` | Rust `Real::sqrt` returns `Real::zero()` for a negative input (lib.rs:545-547); C# throws `ArithmeticException`. Behaviour differs, so the throw row is refused. |
| `PI_VALIDATION` | Rust `Real::pi` performs no digit validation (`pi(0)` returns 3, negative counts are not rejected); C# throws `ArgumentOutOfRangeException`. |
| `NO_PROPERTY` | Rust `Real` exposes no `Exponent`/`PeriodStart`/`PeriodLength`/`DisplayDecimalPlaces`/`MaxComputationDecimalPlaces` member and no mutable counterpart; these rows assert only the accessor or its default, so there is no concrete value to reproduce. |
| `NO_CTOR` | Rust `Real` has no `double`/`decimal`/`Integer`/copy constructor, no `Assign`, and no mutable `Exponent`; the constructor/aliasing assertion cannot be reproduced (the string and span constructor rows are ported). |
| `NO_TRYFORMAT` | Rust has no buffer-writing `TryFormat` API. |
| `DISPLAY_CAP` | Rust `to_string` has no display-decimal cap (it prints the stored scale); the C# assertion is that `DisplayDecimalPlaces = 5` truncates the rendering. |
| `NO_OVERLOAD` | Rust has a single `to_string`; the C# row asserts that a format-provider overload delegates to it (API shape only). |
| `MALFORMED` | The row constructs the malformed `Real(Nat, sign, exp=0, periodStart=0, periodLength=2)` state through an internal constructor Rust does not expose; the assertion is that rendering that state does not throw. |
| `NULL` | Rust has no null string; the empty/invalid inputs are ported and the null row cannot be expressed. |
| `NO_BATCH` | Rust has no batch `Sqrt(IReadOnlyList<Real>)` API, so the batch and empty-batch assertions have no counterpart. |
| `NO_PISEGMENT` | `PiSegment` is C#'s internal Chudnovsky splitting helper; Rust's `pi` uses Machin's formula and exposes no segment API. |
| `NO_PRECISION_SCOPE` | Rust has no process-global/AsyncLocal precision scope or static display/computation properties; these rows assert the scope and atomicity machinery itself. |
| `EXACT_BUDGET` | C# treats a terminating denominator (2^a·5^b) as exact at every digit budget; Rust `Real::from_fraction` charges the terminating expansion against `max_digits` and truncates. Different exactness/budget semantics. |
| `LEADING_ZEROS` | C# carries the leading zero run in the exponent (1/(3·10^1000) is the exact period 0.(3) at scale 10^-1001); Rust accumulates those zeros as digits and truncates at the budget. |
| `DIGIT_BUDGET` | Rust truncates one digit later than C# at a given budget (39 places -> 40 stored digits; 20 -> 21), so the expected literal cannot be reproduced exactly. |
| `NO_LREAL` | The fixed-precision `LReal64`/`LReal128` types do not exist in the Rust crate. |
| `NO_TRIG` | The Rust `lovelace-real` crate exposes no trigonometric functions; the trig corpora cannot be exercised. |

### Refused cases

| Census id (method + case indices) | C# file:line | Reason |
|---|---|---|
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealAddPeriodicExactTests/Add_ExactRationalPair_Property_DenotesTheExactRationalSum` #12-#23, #36-#47, #60-#95, #108-#143, #156-#167, #180-#191, #204-#215, #228-#239, #252-#263, #276-#287, #300-#311, #324-#335, #348-#431, #444-#455, #468-#479, #492-#503, #516-#527, #540-#551, #564-#575, #582-#584, #588-#599, #606-#608, #612-#623, #630-#632, #636-#647, #660-#671, #684-#695, #708-#719, #732-#743, #756-#767, #780-#791, #804-#815, #828-#839, #852-#863 | Lovelace.Real.Tests/RealAddPeriodicExactTests.cs:91 | `PREC` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealAddPeriodicExactTests/Subtract_ExactRationalPair_Property_DenotesTheExactRationalDifference` #12-#23, #36-#47, #60-#95, #108-#143, #156-#167, #180-#191, #204-#215, #228-#239, #252-#263, #276-#287, #300-#311, #324-#335, #348-#431, #444-#455, #468-#479, #492-#503, #516-#527, #540-#551, #564-#575, #582-#584, #588-#599, #606-#608, #612-#623, #630-#632, #636-#647, #660-#671, #684-#695, #708-#719, #732-#743, #756-#767, #780-#791, #804-#815, #828-#839, #852-#863 | Lovelace.Real.Tests/RealAddPeriodicExactTests.cs:126 | `PREC` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealMultiplyPeriodicExactTests/Multiply_PeriodicQuotientTimesItsDivisor_Property_IsExactlyTheDividend` #20, #22, #50-#52, #60, #63-#67, #80, #82-#83, #100, #102-#103, #110-#112, #120, #122-#124, #130-#131, #133-#134, #140-#142, #160, #162-#166, #168-#173, #180, #182-#184, #200, #202-#204, #210-#213, #215-#218, #220, #222-#224, #230-#232, #250-#254, #260, #262-#268, #270-#271, #273-#274, #280, #282-#284, #290-#292 | Lovelace.Real.Tests/RealMultiplyPeriodicExactTests.cs:86 | `PREC` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigPrecisionOracleTests/TheRequestedFractionalDigitsAreCorrect` #0-#23 | Lovelace.Real.Tests/RealTrigPrecisionOracleTests.cs:83 | `NO_TRIG` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.LReal64Tests/Parse_GivenValue_RoundTripsLikeReal` #0-#20 | Lovelace.Real.Tests/LReal64Tests.cs:54 | `NO_LREAL` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.LReal64Tests/Divide_GivenPair_MatchesReal` #0-#10 | Lovelace.Real.Tests/LReal64Tests.cs:100 | `NO_LREAL` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.LReal128Tests/Parse_GivenValue_RoundTripsLikeReal` #0-#9 | Lovelace.Real.Tests/LReal128Tests.cs:32 | `NO_LREAL` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealExactQuotientPropertyTests/Divide_TerminatingDenominator_ReturnsTheExactQuotient` #0-#6 | Lovelace.Real.Tests/RealExactQuotientPropertyTests.cs:56 | `EXACT_BUDGET` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.LReal64Tests/Add_GivenPair_MatchesReal` #0-#5 | Lovelace.Real.Tests/LReal64Tests.cs:68 | `NO_LREAL` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.LReal128Tests/Divide_GivenPair_MatchesReal` #0-#4 | Lovelace.Real.Tests/LReal128Tests.cs:56 | `NO_LREAL` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.LReal64Tests/Compare_MatchesReal` #0-#4 | Lovelace.Real.Tests/LReal64Tests.cs:147 | `NO_LREAL` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.LReal64Tests/Subtract_GivenPair_MatchesReal` #0-#4 | Lovelace.Real.Tests/LReal64Tests.cs:77 | `NO_LREAL` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.LReal128Tests/Add_GivenPair_MatchesReal` #0-#3 | Lovelace.Real.Tests/LReal128Tests.cs:40 | `NO_LREAL` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.LReal64Tests/Multiply_GivenPair_MatchesReal` #0-#3 | Lovelace.Real.Tests/LReal64Tests.cs:85 | `NO_LREAL` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealDivideLeadingZeroRunTests/Divide_GivenLeadingZeroRunLongerThanTheBudget_KeepsTheValueAndItsScale` #0-#3 | Lovelace.Real.Tests/RealDivideLeadingZeroRunTests.cs:51 | `LEADING_ZEROS` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealPowZeroBaseTests/Pow_ZeroBaseNegativeExponent_Refuses` #0-#3 | Lovelace.Real.Tests/RealPowZeroBaseTests.cs:63 | `POW_REFUSAL` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigCertifiedPrefixTests/ArgumentAwareWidth_AppliesAtEveryMagnitude` #0-#3 | Lovelace.Real.Tests/RealTrigCertifiedPrefixTests.cs:132 | `NO_TRIG` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.LReal128Tests/Multiply_GivenPair_MatchesReal` #0-#2 | Lovelace.Real.Tests/LReal128Tests.cs:47 | `NO_LREAL` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealAddPeriodicExactTests/Subtract_OneThirdMinusNThrees_IsExactlyOneOverThreeTimesTenToTheN` #0-#2 | Lovelace.Real.Tests/RealAddPeriodicExactTests.cs:214 | `PREC` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigCertifiedPrefixTests/Sin_OfTen_PublishesTheDigitsItClaims` #0-#2 | Lovelace.Real.Tests/RealTrigCertifiedPrefixTests.cs:104 | `NO_TRIG` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigPiResolutionPropertyTests/Cos_OfHalfPiTruncatedAtDPlaces_IsTheHalfDistanceToPi` #0-#2 | Lovelace.Real.Tests/RealTrigPiResolutionPropertyTests.cs:128 | `NO_TRIG` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigPiResolutionPropertyTests/Sin_IntegerMultiplesOfPiTruncatedAtDPlaces_ReproduceTheDistanceToPi` #0-#2 | Lovelace.Real.Tests/RealTrigPiResolutionPropertyTests.cs:93 | `NO_TRIG` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigPiResolutionPropertyTests/SinAndCos_ExactMultiplesOfTheAmbientPi_KeepTheirExactTableValues` #0-#2 | Lovelace.Real.Tests/RealTrigPiResolutionPropertyTests.cs:207 | `NO_TRIG` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigPiResolutionPropertyTests/SinAndCos_RationalMultiplesOfPiTruncatedAtDPlaces_StayOnTheExactAngle` #0-#2 | Lovelace.Real.Tests/RealTrigPiResolutionPropertyTests.cs:165 | `NO_TRIG` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealDivideTests/Divide_GivenDivisorWithFractionalDigits_ReturnsTheExactQuotient` #4-#5 | Lovelace.Real.Tests/RealDivideTests.cs:176 | `DIGIT_BUDGET` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigCertifiedPrefixTests/Cos_OfOne_PublishesTheDigitsItClaims` #0-#1 | Lovelace.Real.Tests/RealTrigCertifiedPrefixTests.cs:85 | `NO_TRIG` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.LReal128Tests/Add_Periodic_MatchesReal` #0 | Lovelace.Real.Tests/LReal128Tests.cs:60 | `NO_LREAL` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.LReal128Tests/Add_Tenths_IsExact` #0 | Lovelace.Real.Tests/LReal128Tests.cs:75 | `NO_LREAL` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.LReal128Tests/Divide_LongPeriod_Promotes` #0 | Lovelace.Real.Tests/LReal128Tests.cs:71 | `NO_LREAL` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.LReal128Tests/Multiply_Overflow_Promotes` #0 | Lovelace.Real.Tests/LReal128Tests.cs:67 | `NO_LREAL` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.LReal64Tests/Add_Overflow_Promotes` #0 | Lovelace.Real.Tests/LReal64Tests.cs:126 | `NO_LREAL` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.LReal64Tests/Add_Periodic_MatchesReal` #0 | Lovelace.Real.Tests/LReal64Tests.cs:104 | `NO_LREAL` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.LReal64Tests/Add_Tenths_IsExact` #0 | Lovelace.Real.Tests/LReal64Tests.cs:114 | `NO_LREAL` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.LReal64Tests/Divide_LongPeriod_Promotes` #0 | Lovelace.Real.Tests/LReal64Tests.cs:122 | `NO_LREAL` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.LReal64Tests/Multiply_Overflow_Promotes` #0 | Lovelace.Real.Tests/LReal64Tests.cs:130 | `NO_LREAL` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.LReal64Tests/Parse_TooManyDigits_Fails` #0 | Lovelace.Real.Tests/LReal64Tests.cs:134 | `NO_LREAL` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealAddPeriodicExactTests/Subtract_PeriodicOperandMinusItsOwnTruncation_IsTheTruncationRemainder` #0 | Lovelace.Real.Tests/RealAddPeriodicExactTests.cs:159 | `PREC` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealAddTests/Add_GivenPeriodic1Over6Plus1Over6_ReturnsOneThird` #0 | Lovelace.Real.Tests/RealAddTests.cs:129 | `PREC` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealAsyncLocalTests/Pi_GivenConcurrentTasksWithDifferentLocalPrecisions_PrecisionDoesNotLeakAcrossTasks` #0 | Lovelace.Real.Tests/RealAsyncLocalTests.cs:21 | `NO_PRECISION_SCOPE` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealAsyncLocalTests/Pi_GivenNestedPrecisionScopes_OuterScopeRestoredAfterInnerDisposes` #0 | Lovelace.Real.Tests/RealAsyncLocalTests.cs:93 | `NO_PRECISION_SCOPE` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealAsyncLocalTests/Pi_StaticDisplayDecimalPlaces_ConcurrentReadsMutationsAreAtomic` #0 | Lovelace.Real.Tests/RealAsyncLocalTests.cs:123 | `NO_PRECISION_SCOPE` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealAsyncLocalTests/Sqrt_GivenBatchWithCallerLocalPrecision_EachChildTaskInheritsCallerPrecision` #0 | Lovelace.Real.Tests/RealAsyncLocalTests.cs:64 | `NO_PRECISION_SCOPE` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealConstructorTests/Assign_GivenOtherReal_CopiesAllFields` #0 | Lovelace.Real.Tests/RealConstructorTests.cs:132 | `NO_CTOR` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealConstructorTests/Assign_GivenOtherReal_ProducesDeepCopy` #0 | Lovelace.Real.Tests/RealConstructorTests.cs:144 | `NO_CTOR` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealConstructorTests/Assign_GivenPeriodicReal_CopiesPeriodMetadata` #0 | Lovelace.Real.Tests/RealConstructorTests.cs:153 | `NO_CTOR` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealConstructorTests/Constructor_GivenDecimal_RoundTripEqualsStringConstructor` #0 | Lovelace.Real.Tests/RealConstructorTests.cs:246 | `NO_CTOR` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealConstructorTests/Constructor_GivenDecimalMaxValue_ParsesWithoutThrow` #0 | Lovelace.Real.Tests/RealConstructorTests.cs:238 | `NO_CTOR` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealConstructorTests/Constructor_GivenDouble_StoresCorrectDigitsAndExponent` #0 | Lovelace.Real.Tests/RealConstructorTests.cs:32 | `NO_CTOR` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealConstructorTests/Constructor_GivenDoubleZero_ProducesZero` #0 | Lovelace.Real.Tests/RealConstructorTests.cs:39 | `NO_CTOR` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealConstructorTests/Constructor_GivenInteger_ExponentIsZero` #0 | Lovelace.Real.Tests/RealConstructorTests.cs:109 | `NO_CTOR` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealConstructorTests/Constructor_GivenNegativeDecimal_ParsesCorrectly` #0 | Lovelace.Real.Tests/RealConstructorTests.cs:223 | `NO_CTOR` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealConstructorTests/Constructor_GivenNegativeDouble_IsNegative` #0 | Lovelace.Real.Tests/RealConstructorTests.cs:46 | `NO_CTOR` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealConstructorTests/Constructor_GivenNegativeInteger_IsNegativeAndExponentZero` #0 | Lovelace.Real.Tests/RealConstructorTests.cs:118 | `NO_CTOR` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealConstructorTests/Constructor_GivenPositiveDecimal_ParsesCorrectly` #0 | Lovelace.Real.Tests/RealConstructorTests.cs:216 | `NO_CTOR` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealConstructorTests/Constructor_GivenRealCopy_CopiesAllFields` #0 | Lovelace.Real.Tests/RealConstructorTests.cs:94 | `NO_CTOR` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealConstructorTests/Constructor_GivenRealCopy_ProducesIndependentCopy` #0 | Lovelace.Real.Tests/RealConstructorTests.cs:84 | `NO_CTOR` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealConstructorTests/Constructor_GivenZeroDecimal_ReturnsZero` #0 | Lovelace.Real.Tests/RealConstructorTests.cs:231 | `NO_CTOR` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealDivideExactnessPropertyTests/DividingPeriodicOperands_KeepsRepresentableQuotientsExact` #0 | Lovelace.Real.Tests/RealDivideExactnessPropertyTests.cs:69 | `PREC` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealDivideExactnessPropertyTests/DivisionAndItsMultiplicationTwin_AgreeOnValueAndExactness` #0 | Lovelace.Real.Tests/RealDivideExactnessPropertyTests.cs:33 | `PREC` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealDivideExactnessPropertyTests/QuotientsThatCannotBeRepresented_StayInexact` #0 | Lovelace.Real.Tests/RealDivideExactnessPropertyTests.cs:117 | `PREC` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealDivideLeadingZeroRunTests/Divide_GivenQuotientInsideTheBudget_StillReportsTheExactPeriod` #0 | Lovelace.Real.Tests/RealDivideLeadingZeroRunTests.cs:115 | `LEADING_ZEROS` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealDivideLeadingZeroRunTests/Divide_GivenQuotientTheBudgetCannotHold_IsInexactAndKeepsItsScale` #0 | Lovelace.Real.Tests/RealDivideLeadingZeroRunTests.cs:83 | `LEADING_ZEROS` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealDivideTests/Divide_GivenQuotientSmallerThanItsScale_KeepsTheSignificantDigits` #0 | Lovelace.Real.Tests/RealDivideTests.cs:217 | `DIGIT_BUDGET` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealExactQuotientPropertyTests/Divide_TerminatingExpansionLongerThanTheDefaultBudget_IsExact` #0 | Lovelace.Real.Tests/RealExactQuotientPropertyTests.cs:108 | `EXACT_BUDGET` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealExactQuotientPropertyTests/Divide_TerminatingQuotient_DoesNotDependOnTheDigitBudget` #0 | Lovelace.Real.Tests/RealExactQuotientPropertyTests.cs:80 | `EXACT_BUDGET` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealModulusTests/Modulus_GivenDecimalOperands_ReturnsCorrectRemainder` #0 | Lovelace.Real.Tests/RealModulusTests.cs:62 | `NO_MODULUS` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealModulusTests/Modulus_GivenDividendLessThanDivisor_ReturnsDividend` #0 | Lovelace.Real.Tests/RealModulusTests.cs:26 | `NO_MODULUS` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealModulusTests/Modulus_GivenDivisorZero_ThrowsDivideByZeroException` #0 | Lovelace.Real.Tests/RealModulusTests.cs:40 | `NO_MODULUS` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealModulusTests/Modulus_GivenExactMultiple_ReturnsZero` #0 | Lovelace.Real.Tests/RealModulusTests.cs:70 | `NO_MODULUS` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealModulusTests/Modulus_GivenNegativeDividend_ReturnsNegativeRemainder` #0 | Lovelace.Real.Tests/RealModulusTests.cs:46 | `NO_MODULUS` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealModulusTests/Modulus_GivenNegativeDivisor_FollowsTruncatedSemantics` #0 | Lovelace.Real.Tests/RealModulusTests.cs:54 | `NO_MODULUS` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealModulusTests/Modulus_GivenTwoPositiveIntegers_ReturnsRemainder` #0 | Lovelace.Real.Tests/RealModulusTests.cs:19 | `NO_MODULUS` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealModulusTests/Modulus_GivenZeroDividend_ReturnsZero` #0 | Lovelace.Real.Tests/RealModulusTests.cs:33 | `NO_MODULUS` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealMultiplyPeriodicExactTests/Multiply_DividendOverDivisorTimesDivisor_AuditShapes_AreExactlyTheDividend` #2 | Lovelace.Real.Tests/RealMultiplyPeriodicExactTests.cs:127 | `PREC` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealMultiplyPeriodicExactTests/Multiply_PeriodicTimesNonPeriodic_IsExactWhenTheProductIsRational` #0 | Lovelace.Real.Tests/RealMultiplyPeriodicExactTests.cs:209 | `PREC` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealMultiplyPeriodicExactTests/Multiply_PeriodicTimesTruncatedIrrational_IsExactForTheStoredRationalsOnly` #0 | Lovelace.Real.Tests/RealMultiplyPeriodicExactTests.cs:229 | `PREC` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealParseTests/TryParse_GivenNullString_ReturnsFalse` #0 | Lovelace.Real.Tests/RealParseTests.cs:101 | `NULL` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealPeriodicIntegerValuePropertyTests/PeriodicTailAfterNine_IsTheNextInteger` #0 | Lovelace.Real.Tests/RealPeriodicIntegerValuePropertyTests.cs:79 | `PREC` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealPiSegmentTests/PiSegment_GivenFullRangeTenDigits_ProducesPiMatchingCurrentSerial` #0 | Lovelace.Real.Tests/RealPiSegmentTests.cs:37 | `NO_PISEGMENT` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealPiSegmentTests/PiSegment_GivenMidpointSplit_MergedTMatchesFullRangeT` #0 | Lovelace.Real.Tests/RealPiSegmentTests.cs:79 | `NO_PISEGMENT` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealPiSegmentTests/PiSegment_GivenSingleTermRange_MatchesManualTermOneComputation` #0 | Lovelace.Real.Tests/RealPiSegmentTests.cs:19 | `NO_PISEGMENT` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealPiTests/Pi_GivenDigitsExceedingMaxComputationDecimalPlaces_ThrowsArgumentOutOfRangeException` #0 | Lovelace.Real.Tests/RealPiTests.cs:74 | `PI_VALIDATION` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealPiTests/Pi_GivenNegativeDigits_ThrowsArgumentOutOfRangeException` #0 | Lovelace.Real.Tests/RealPiTests.cs:68 | `PI_VALIDATION` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealPiTests/Pi_GivenZeroDigits_ThrowsArgumentOutOfRangeException` #0 | Lovelace.Real.Tests/RealPiTests.cs:62 | `PI_VALIDATION` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealPiToAsyncTests/PiToAsync_GivenInvalidDigits_PropagatesArgumentOutOfRangeException` #0 | Lovelace.Real.Tests/RealPiAsyncTests.cs:32 | `PI_VALIDATION` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealPowZeroBaseTests/Pow_NonZeroBaseNegativeExponent_KeepsItsExistingRefusal` #0 | Lovelace.Real.Tests/RealPowZeroBaseTests.cs:91 | `POW_REFUSAL` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealPowZeroBaseTests/Pow_ZeroBaseNegativeRealExponent_RefusesWithTheIntegerPathsRefusal` #0 | Lovelace.Real.Tests/RealPowZeroBaseTests.cs:30 | `POW_REFUSAL` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealPowZeroBaseTests/Pow_ZeroBasePositiveFractionalExponent_IsStillZero` #0 | Lovelace.Real.Tests/RealPowZeroBaseTests.cs:82 | `POW_REFUSAL` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealPropertiesTests/DisplayDecimalPlaces_AfterSetting_ReturnsNewValue` #0 | Lovelace.Real.Tests/RealPropertiesTests.cs:111 | `NO_PROPERTY` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealPropertiesTests/DisplayDecimalPlaces_DefaultValue_IsOneHundred` #0 | Lovelace.Real.Tests/RealPropertiesTests.cs:103 | `NO_PROPERTY` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealPropertiesTests/Exponent_AfterSetting_ReturnsNewValue` #0 | Lovelace.Real.Tests/RealPropertiesTests.cs:58 | `NO_PROPERTY` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealPropertiesTests/Exponent_GivenDefaultReal_IsZero` #0 | Lovelace.Real.Tests/RealPropertiesTests.cs:51 | `NO_PROPERTY` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealPropertiesTests/IsPeriodic_GivenDefaultReal_IsFalse` #0 | Lovelace.Real.Tests/RealPropertiesTests.cs:92 | `NO_PROPERTY` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealPropertiesTests/MaxComputationDecimalPlaces_AfterSetting_ReturnsNewValue` #0 | Lovelace.Real.Tests/RealPropertiesTests.cs:138 | `NO_PROPERTY` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealPropertiesTests/MaxComputationDecimalPlaces_DefaultValue_IsOneThousand` #0 | Lovelace.Real.Tests/RealPropertiesTests.cs:130 | `NO_PROPERTY` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealPropertiesTests/PeriodLength_GivenDefaultReal_IsZero` #0 | Lovelace.Real.Tests/RealPropertiesTests.cs:81 | `NO_PROPERTY` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealPropertiesTests/PeriodStart_GivenDefaultReal_IsZero` #0 | Lovelace.Real.Tests/RealPropertiesTests.cs:70 | `NO_PROPERTY` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealSqrtAsyncTests/SqrtAsync_GivenNegativeInput_PropagatesArithmeticException` #0 | Lovelace.Real.Tests/RealSqrtAsyncTests.cs:29 | `NEG_SQRT` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealSqrtBatchTests/Sqrt_GivenBatchContainingNegativeValue_PropagatesArithmeticException` #0 | Lovelace.Real.Tests/RealSqrtBatchTests.cs:68 | `NO_BATCH` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealSqrtBatchTests/Sqrt_GivenBatchMixedIrrationals_MatchesElementwiseSerial` #0 | Lovelace.Real.Tests/RealSqrtBatchTests.cs:46 | `NO_BATCH` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealSqrtBatchTests/Sqrt_GivenBatchPerfectSquares_ReturnsExactRoots` #0 | Lovelace.Real.Tests/RealSqrtBatchTests.cs:29 | `NO_BATCH` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealSqrtBatchTests/Sqrt_GivenBatchSingleElement_ReturnsArrayOfOneMatchingSerial` #0 | Lovelace.Real.Tests/RealSqrtBatchTests.cs:83 | `NO_BATCH` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealSqrtBatchTests/Sqrt_GivenEmptyBatch_ReturnsEmptyArray` #0 | Lovelace.Real.Tests/RealSqrtBatchTests.cs:18 | `NO_BATCH` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealSqrtTests/Sqrt_GivenNegativeInput_ThrowsArithmeticException` #0 | Lovelace.Real.Tests/RealSqrtTests.cs:91 | `NEG_SQRT` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealToStringTests/ToString_GivenNonPeriodicValue_TruncatesAtDisplayDecimalPlaces` #0 | Lovelace.Real.Tests/RealToStringTests.cs:140 | `DISPLAY_CAP` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealToStringTests/ToString_GivenPeriodLongerThanStoredDigits_DoesNotThrow` #0 | Lovelace.Real.Tests/RealToStringTests.cs:229 | `MALFORMED` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealToStringTests/ToString_WithFormatAndProvider_DelegatesToParamlessToString` #0 | Lovelace.Real.Tests/RealToStringTests.cs:179 | `NO_OVERLOAD` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealToStringTests/TryFormat_GivenInsufficientBuffer_ReturnsFalse` #0 | Lovelace.Real.Tests/RealToStringTests.cs:100 | `NO_TRYFORMAT` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealToStringTests/TryFormat_GivenSufficientBuffer_ReturnsTrueAndWritesCorrectly` #0 | Lovelace.Real.Tests/RealToStringTests.cs:82 | `NO_TRYFORMAT` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigCertifiedPrefixTests/AWiderAmbientScope_IsPreserved` #0 | Lovelace.Real.Tests/RealTrigCertifiedPrefixTests.cs:162 | `NO_TRIG` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigCertifiedPrefixTests/PublishedWidth_IsTheDisplayLimit_ForEveryNonPeriodicValue` #0 | Lovelace.Real.Tests/RealTrigCertifiedPrefixTests.cs:189 | `NO_TRIG` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigCertifiedPrefixTests/Sin_And_Cos_OfTenToTheHundred_PublishTheDigitsTheyClaim` #0 | Lovelace.Real.Tests/RealTrigCertifiedPrefixTests.cs:116 | `NO_TRIG` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigCertifiedPrefixTests/Sin_OfOne_AtFiftyPlaces_PublishesFiftyCorrectDigits` #0 | Lovelace.Real.Tests/RealTrigCertifiedPrefixTests.cs:66 | `NO_TRIG` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigCertifiedPrefixTests/Sin_OfOne_AtTwoHundredPlaces_PublishesTwoHundredCorrectDigits` #0 | Lovelace.Real.Tests/RealTrigCertifiedPrefixTests.cs:74 | `NO_TRIG` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigCertifiedPrefixTests/Sin_OfOne_PublishesTheDigitsItClaims` #0 | Lovelace.Real.Tests/RealTrigCertifiedPrefixTests.cs:59 | `NO_TRIG` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigCertifiedPrefixTests/SmallAngles_KeepTheirMagnitude` #0 | Lovelace.Real.Tests/RealTrigCertifiedPrefixTests.cs:208 | `NO_TRIG` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigCertifiedPrefixTests/SpecialAngles_AreStillExactAtEveryPrecision` #0 | Lovelace.Real.Tests/RealTrigCertifiedPrefixTests.cs:146 | `NO_TRIG` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigFastPathTests/ArgumentReduction_IntegerQuotient_MatchesTheDividedForm` #0 | Lovelace.Real.Tests/RealTrigFastPathTests.cs:136 | `NO_TRIG` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigFastPathTests/Cos_AtDefaultPrecision_StaysInteractive` #0 | Lovelace.Real.Tests/RealTrigFastPathTests.cs:271 | `NO_TRIG` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigFastPathTests/HalfOf_MatchesTheDividedForm_ForPiShapedValues` #0 | Lovelace.Real.Tests/RealTrigFastPathTests.cs:351 | `NO_TRIG` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigFastPathTests/NonSpecialAngles_AreNotCapturedByTheTable` #0 | Lovelace.Real.Tests/RealTrigFastPathTests.cs:101 | `NO_TRIG` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigFastPathTests/Normalize_BinaryStrip_MatchesTheDigitStringStrip` #0 | Lovelace.Real.Tests/RealTrigFastPathTests.cs:293 | `NO_TRIG` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigFastPathTests/Normalize_BinaryStrip_MatchesTheDigitStringStrip_ForLongZeroRuns` #0 | Lovelace.Real.Tests/RealTrigFastPathTests.cs:380 | `NO_TRIG` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigFastPathTests/SeriesTermination_IntegerTest_MatchesTheThresholdComparison` #0 | Lovelace.Real.Tests/RealTrigFastPathTests.cs:172 | `NO_TRIG` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigFastPathTests/SeriesTermination_IntegerTest_TracksTheCarriedPowerAcrossManyTerms` #0 | Lovelace.Real.Tests/RealTrigFastPathTests.cs:213 | `NO_TRIG` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigFastPathTests/SpecialAngles_BuiltTheOriginalWay_StillHitTheTableExactly` #0 | Lovelace.Real.Tests/RealTrigFastPathTests.cs:51 | `NO_TRIG` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigFastPathTests/ToInteger_BinaryShift_MatchesTheDigitPadding` #0 | Lovelace.Real.Tests/RealTrigFastPathTests.cs:322 | `NO_TRIG` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigPrecisionOracleTests/TheOracleRejectsATruncatedValue` #0 | Lovelace.Real.Tests/RealTrigPrecisionOracleTests.cs:107 | `NO_TRIG` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigSymmetryTests/Cos_GivenNegativeAngle_IsExactlyTheCosineOfItsMagnitude` #0 | Lovelace.Real.Tests/RealTrigSymmetryTests.cs:52 | `NO_TRIG` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigSymmetryTests/Sin_GivenNegativeAngle_IsExactlyTheNegatedSineOfItsMagnitude` #0 | Lovelace.Real.Tests/RealTrigSymmetryTests.cs:43 | `NO_TRIG` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigSymmetryTests/Sin_GivenNegativeSpecialAngles_KeepsTheirExactValues` #0 | Lovelace.Real.Tests/RealTrigSymmetryTests.cs:61 | `NO_TRIG` |

Total refused: **1336** cases. Total ported in this project: **1201** (47.34%).

## Verification (observed)

### `cargo test`

```text
$ cargo test --manifest-path rust/Cargo.toml -p lovelace-real
test result: ok. 1219 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s

$ cargo test --manifest-path rust/Cargo.toml --workspace
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s   (lovelace-array)
test result: ok. 67 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s   (lovelace-desktop)
test result: ok. 125 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s  (lovelace-int)
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s    (lovelace-json)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s    (lovelace-lang)
test result: ok. 134 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s  (lovelace-nat)
test result: ok. 1219 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s (lovelace-real)
test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s   (lovelace-symbolics)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out                   (bins + doc-tests)
```

Sum: **1,645 passed, 0 failed**.

### Inventory, gate and coverage

```text
$ powershell -File parity/gen-rust-tests.ps1
crates=11 tests=1645 suiteExit=0 passed=1645 failed=0

$ node parity/gen-manifest.mjs
{ "rows": 6209, "dispositions": { "deferred": 4752, "ported": 1457 }, "coveragePercent": 23.47, "rustTests": 1645 }

$ node parity/tools/gate-manifest.mjs
{ "censusCases": 6209, "manifestRows": 6209, "dispositions": { "ported": 1457, "consolidated": 0, "excluded": 0, "deferred": 4752 },
  "coveragePercent": 23.47, "claimedRustTests": 1457, "projectsInCensus": 15, "failures": [] }
PASS (structural): 6209 test cases dispositioned; 1457 claim Rust coverage (23.47%).

$ node parity/tools/coverage-report.mjs
coverage.md written: 1457/6209 ported (23.47%)
```

`parity/coverage.md` line for this project after regeneration:

```text
| Lovelace.Real.Tests | 2537 | 1201 | 1336 | 0 | 47.34 % | port |
```

## Files changed

- `rust/lovelace-real/src/lib.rs` — appended `#[cfg(test)] mod real_parity_batch` (1,195 tests: helpers, generated corpus rows and per-row ports). Production prefix untouched.
- `parity/manifest-map.json` — 1,195 new `ported` entries (262 -> 1,457).
- `parity/manifest.toml`, `parity/rust-tests.json` — regenerated by the documented generators.
- `parity/coverage.md` — regenerated.
- `docs/goal-parity-contract/round-6/real-porting-batch.md` — this file.

## What could not be verified

The 1,336 refused cases above. The dominant reason is the preperiod fraction conversion (1,104 of the 1,336 refusals: 501 add + 501 subtract + 90 multiply corpus rows, plus 12 fact and divide-exactness rows that depend on the same conversion). The rest are APIs the Rust crate does not have (modulus, trig, LReal64/128, batch sqrt, PiSegment, precision scopes, buffer TryFormat, display caps), typed C# exceptions Rust does not throw, and digit-budget semantics that differ by one digit or by the treatment of terminating/leading-zero expansions.
