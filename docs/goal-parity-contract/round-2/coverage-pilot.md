# Round 2 — parity coverage pilot

Status: **delivered, bounded pilot**. This file records only mappings that were verified by reading both
the C# test body and the Rust test body and by checking that the Rust test id exists in
`parity/rust-tests.json` (suite: 293 passed, exit 0). It does not claim coverage that was not inspected.

- Data source: `parity/census.json` (6,209 cases, 15 projects), oracle commit recorded there.
- Rust suite: `parity/rust-tests.json`, 293 tests across 11 crates, `suite.exitCode = 0`.
- Mapping ledger: `parity/manifest-map.json`, 20 entries, keys sorted, every value
  `{ disposition: "ported", rust: "<crate>::<test>" }`.
- Scope of the pilot: the five named projects (616 census cases). The remaining ten projects were not
  examined and stay deferred.

## Method and acceptance rule

1. Read the concrete inputs and the asserted result/failure from the C# source and cite the assertion line.
2. Read the Rust test body and cite the assertion line in the crate source.
3. Accept only when the operands and expected observable are the same, or when the Rust test asserts the
   same fact plus strictly more (e.g. exact string plus digit count or a u64 round-trip).
4. Reject when the Rust side is missing, uses different operands, asserts a weaker fact, or signals failure
   differently (a specific C# exception versus Rust `Option::None`). Rejections are counted below.
5. A shared test name was never used as evidence; every accepted row has a value or literal comparison.

All paths in this document are relative to the repository root; Rust citations name the file and line.

## Accepted count per project

| Project | Census cases | Accepted | Remaining (deferred) |
|---|---:|---:|---:|
| Lovelace.Natural.Tests | 195 | **6** | 189 |
| Lovelace.Integer.Tests | 148 | **14** | 134 |
| Lovelace.Representation.Tests | 91 | **0** | 91 |
| Lovelace.Complex.Tests | 121 | **0** | 121 |
| Lovelace.Dsp.Tests | 61 | **0** | 61 |
| **Pilot total** | **616** | **20** | **596** |

## Accepted mappings (20)

| Census id | Rust test id | C# assertion | Rust assertion | Why equivalent |
|---|---|---|---|---|
| `Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalConstructorTests/Constructor_GivenDefault_ProducesZero#0` | `lovelace-nat::tests::default_GivenNoArguments_EqualsZero` | `new Natural()` is zero (`NaturalConstructorTests.cs:19-20`) | `Nat::default() == Nat::zero()`; zero pinned canonical at `lib.rs:836-842` (`lib.rs:846`) | Both define default construction as canonical zero; the composed Rust assertions cover exactly that. |
| `Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalConstructorTests/Constructor_GivenUlongZero_ProducesZero#0` | `lovelace-nat::tests::from_u64_GivenZero_ReturnsZero` | `new Natural(0UL)` is zero (`NaturalConstructorTests.cs:30-31`) | `from_u64(0).to_decimal() == "0"` (`lib.rs:851`) | Same input 0 and same canonical-zero outcome; Rust asserts the rendering. |
| `Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalConstructorTests/Constructor_GivenUlongMaxValue_StoresAllDigits#0` | `lovelace-nat::tests::from_u64_GivenMaximum_RoundTripsThroughU64` | `new Natural(ulong.MaxValue).ToString() == "18446744073709551615"` (`NaturalConstructorTests.cs:44-45`) | same decimal string plus `to_u64() == Some(u64::MAX)` (`lib.rs:861-863`) | Same input and same literal; Rust adds the round-trip, strictly stronger. |
| `Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalFactorialTests/Factorial_GivenZero_ReturnsOne#0` | `lovelace-nat::tests::factorial_GivenZero_ReturnsOne` | `0! == 1` (`NaturalFactorialTests.cs:14-15`) | `Nat::factorial(0).is_one()` (`lib.rs:1058`) | Same input and result. |
| `Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalFactorialTests/Factorial_GivenOne_ReturnsOne#0` | `lovelace-nat::tests::factorial_GivenOne_ReturnsOne` | `1! == 1` (`NaturalFactorialTests.cs:23-24`) | `Nat::factorial(1).is_one()` (`lib.rs:1063`) | Same input and result. |
| `Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalFactorialTests/Factorial_Given100_ReturnsCorrectResult#0` | `lovelace-nat::tests::factorial_GivenHundred_Has158DigitsAndKnownValue` | exact 158-digit `100!` compared to `ToString()` (`NaturalFactorialTests.cs:60-63`) | byte-identical 158-digit string plus `len() == 158` (`lib.rs:1073-1078`) | Expected literals verified equal programmatically; Rust adds the digit count. |
| `Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerConstructorTests/Constructor_Default_MagnitudeIsZero#0` | `lovelace-int::tests::default_GivenNoArguments_EqualsZero` | `new Integer()` is zero (`IntegerConstructorTests.cs:19-20`) | `Int::default() == Int::zero()`; zero pinned at `lib.rs:292-298` (`lib.rs:303`) | Both define default construction as canonical zero. |
| `Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerConstructorTests/Constructor_GivenNegativeLong_StoresCorrectMagnitudeAndSign#0` | `lovelace-int::tests::from_i64_GivenNegativeValue_SetsSign` | `new Integer(-42L)` renders `"-42"` and is negative (`IntegerConstructorTests.cs:45-47`) | `sign() == -1` and `to_decimal() == "-42"` (`lib.rs:321-323`) | Same input -42; same rendering and sign. |
| `Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerConstructorTests/Constructor_GivenZeroLong_IsZeroAndPositive#0` | `lovelace-int::tests::from_i64_GivenZero_NormalizesToNonNegative` | `new Integer(0L)` is zero and not negative (`IntegerConstructorTests.cs:53-55`) | `sign() == 0` and `!is_negative()` (`lib.rs:328-330`) | Same input 0; same normalised non-negative zero. |
| `Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerConstructorTests/Constructor_GivenLongMinValue_ParsesSign#0` | `lovelace-int::tests::from_i64_GivenMinimum_HandlesMagnitudeWithoutOverflow` | `long.MinValue` is negative and renders `"-9223372036854775808"` (`IntegerConstructorTests.cs:62-63`) | same string, magnitude `"9223372036854775808"`, negative (`lib.rs:308-311`) | Same input and literal; Rust additionally pins the magnitude. |
| `Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerAddTests/Add_GivenPositiveAndNegativeWhereNegativeLarger_ReturnsNegativeDifference#0` | `lovelace-int::tests::add_GivenOppositeSigns_LargerNegative_ReturnsNegativeDifference` | `3 + (-10) == -7` and negative (`IntegerAddTests.cs:53-55`) | `Int(3).add(&Int(-10)) == Int(-7)` (`lib.rs:426`) | Identical operands and sum; the sign follows from the exact value. |
| `Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerAddTests/Add_GivenOppositeValues_ReturnsZero#0` | `lovelace-int::tests::add_GivenOpposites_ReturnsNonNegativeZero` | `5 + (-5)` is zero and not negative (`IntegerAddTests.cs:63-65`) | same, plus `sign() == 0` (`lib.rs:431-434`) | Same operands and same non-negative zero; Rust is stronger. |
| `Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerSubtractTests/Subtract_GivenPositiveFromSmallerPositive_ReturnsNegative#0` | `lovelace-int::tests::sub_GivenSmallerMinusLarger_ReturnsNegative` | `3 - 10 == -7` and negative (`IntegerSubtractTests.cs:28-30`) | `Int(3).sub(&Int(10)) == Int(-7)` (`lib.rs:446`) | Identical operands and difference. |
| `Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerMultiplyTests/Multiply_GivenZeroOperand_ReturnsZero#2` | `lovelace-int::tests::mul_GivenZeroOperand_ReturnsNonNegativeZero` | theory (-5, 0): zero and not negative (`IntegerMultiplyTests.cs:76-86`) | `Int(-5).mul(&Int::zero())` zero, non-negative, sign 0 (`lib.rs:488-491`) | Only theory case #2 has the exact Rust inputs; Rust is stronger. |
| `Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerFactorialTests/Factorial_GivenZero_ReturnsOne#0` | `lovelace-int::tests::factorial_GivenZero_ReturnsOne` | `0! == 1` (`IntegerFactorialTests.cs:21-22`) | `Int::factorial(0) == Int(1)` (`lib.rs:588`) | Same input and result, structural equality. |
| `Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerFactorialTests/Factorial_GivenTen_ReturnsCorrectValue#0` | `lovelace-int::tests::factorial_GivenTen_ReturnsKnownValue` | `10! == 3628800` (`IntegerFactorialTests.cs:41-42`) | `Int::factorial(10) == Int(3_628_800)` (`lib.rs:593`) | Same input and exact result. |
| `Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerParseTests/Parse_GivenNegativeZero_ReturnsZeroPositive#0` | `lovelace-int::tests::from_decimal_GivenNegativeZero_NormalizesToPositiveZero` | `Parse("-0")` zero and not negative (`IntegerParseTests.cs:35-37`) | `from_decimal("-0")` zero, non-negative, renders `"0"` (`lib.rs:335-338`) | Same literal and canonicalisation; Rust adds rendering. |
| `Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerNegationTests/Negate_GivenZero_ReturnsZeroAndPositive#0` | `lovelace-int::tests::negate_GivenZero_RemainsPositiveZero` | `Integer(0).Negate()` zero and not negative (`IntegerNegationTests.cs:37-39`) | `Int::zero().negate()` zero, non-negative, sign 0, `"0"` (`lib.rs:405-408`) | Same operation and result; Rust is stronger. |
| `Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerToStringTests/ToString_GivenZero_ReturnsZeroWithoutSign#0` | `lovelace-int::tests::zero_GivenNoArguments_IsNonNegativeZero` | `new Integer(0L).ToString() == "0"` (`IntegerToStringTests.cs:26`) | `Int::zero().to_decimal() == "0"` (`lib.rs:297`) | Same formatting operation on zero and same string. |
| `Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerEqualityTests/CompareTo_GivenNegativeLessThanPositive_ReturnsNegative#0` | `lovelace-int::tests::ord_GivenNegativeAndPositive_OrdersBySign` | `(-1).CompareTo(1) < 0` (`IntegerEqualityTests.cs:58`) | `Int(-1) < Int(1)` (`lib.rs:617`) | Same pair and ordering fact; Rust uses `Ord` rather than `CompareTo`. |

## Rejections

### Rejection-accounting for every non-accepted case in the five target projects

Every one of the 596 non-accepted cases is accounted for by class below. "Rust side" names the tests that
were compared when a same-operation test exists; otherwise it states that no counterpart exists.

| Project / class | Cases | Accepted | Rejected | Reason for rejection |
|---|---:|---:|---:|---|
| Natural / NaturalAddTests | 19 | 0 | 19 | Rust `add` tests use 999,999,999+1 and 10^18+1 (`lovelace-nat/src/lib.rs:912-923`); no C# add case has those operands. Commutativity/associativity/leading-zero properties are not ported. |
| Natural / NaturalBinaryLimbTests | 6 | 0 | 6 | Rust has no 2^64 limb-boundary case (uses 10^9/10^18); `ShiftLeftDecimal(7)` is not exercised by `mul_pow10` (tests use exponents 0, 9, 19); the 20,000-digit random round-trip is not ported. |
| Natural / NaturalConstructorTests | 29 | 3 | 26 | Remaining cases use different literals ("007", "0", "12345", 2^64) or assert `FormatException` where Rust `from_decimal` returns `None`; Rust has no int/span/copy constructor. |
| Natural / NaturalDecimalCacheTests | 9 | 0 | 9 | Rust has no decimal-string cache: `to_decimal` recomputes a `String`, so reference identity across calls and concurrent cache fills are unobservable. |
| Natural / NaturalDivideTests | 18 | 0 | 18 | Rust `div_rem` cases are 42/100, 10^18/10^9, (10^18+1)/10^9, 98765432109876543210/123456789 and x/x; none matches a C# operand pair. Divide-by-zero is `None` vs `DivideByZeroException`. |
| Natural / NaturalFactorialTests | 6 | 3 | 3 | Remaining inputs 5, 10 and 21 are not tested by `Nat::factorial` (Rust tests 0, 1, 20, 100, 1000). |
| Natural / NaturalIncrementTests | 9 | 0 | 9 | The Rust `Nat` API has no increment/decrement operator; only `add` exists. |
| Natural / NaturalMultiplyTests | 21 | 0 | 21 | Rust `mul` cases are 0, 1, (10^9−1)^2, 1.234…e19 × 9.876…e19 and transform-vs-Karatsuba random limbs; none matches the C# pairs (9×9, 99×99, 25×4, ulongMax×2, 15/150/200-digit values). |
| Natural / NaturalParseTests | 20 | 0 | 20 | Rust `from_decimal` cases are "00000123", "12a3", "", "-1", all-zeros and one 51-digit literal; C# literals differ and invalid input is an exception, not `None`. |
| Natural / NaturalPowTests | 7 | 0 | 7 | Rust `pow` cases are huge^0, 0^0, 2^100, 0^37; C# pairs 7^0, 13^1, 2^10, 0^5, 2^64, 3^7, 1^50 have no exact (base, exponent) match. |
| Natural / NaturalRandomizedCrossCheckTests | 13 | 0 | 13 | The C# oracle is `System.Numerics.BigInteger`; Rust has no BigInteger-backed differential harness for add/sub/div/pow/factorial. Its only differential test is transform-vs-Karatsuba multiplication. |
| Natural / NaturalStaticPropertyTests | 8 | 0 | 8 | `Natural.DisplayDigits` / `Natural.Precision` static properties do not exist in Rust. |
| Natural / NaturalStructureTests | 15 | 0 | 15 | These assert .NET type identity and `System.Numerics` interface implementation; Rust `Nat` has no equivalent reflection/interface surface. |
| Natural / NaturalSubtractTests | 15 | 0 | 15 | Rust `sub` cases are 1−2, x−x, 10^18−1, 10^9−1; C# operands (10−3, 5−5, 42−0, 1000−…, 3−10, 0−1, ulongMax−1, 100−99) do not match any. |
| Integer / IntegerAddTests | 10 | 2 | 8 | Remaining cases use 3+4, −3+(−4), 100+23, 42+0, 0+(−17), 10+(−3) and 20-nines+1; Rust `add` uses −2+(−3), −3+10, 3+(−10), 5+(−5), 18-nines+1. |
| Integer / IntegerConstructorTests | 20 | 4 | 16 | Remaining cases cover constructors Rust does not have (int, span, `Natural`) or literals/assertions Rust does not test ("42", "−7", "0", "007", "12a3", 30-digit, int.MaxValue); invalid input is `FormatException` vs `None`. |
| Integer / IntegerDivideTests | 12 | 0 | 12 | Rust `div_rem`/signed cases are −7/2, 42/5, 42/0, −3/7, −12/4, 7/−2, −7/−2; no C# pair (10/3, −10/3, −10/−3, 12/4, 100/7) matches, and the C# operator tests have no Rust operator surface. |
| Integer / IntegerEqualityTests | 14 | 1 | 13 | Remaining cases assert equality/relational operators over 42, 0, 7, 3, 5, −5, −4, 10; the Rust Ord/PartialEq tests only pin −1 < 1, −10 < −2 and equal 20-digit values. |
| Integer / IntegerFactorialTests | 5 | 2 | 3 | `Int::factorial` tests only 0 and 10; 1!, 5! and negative-input rejection are not tested (negative is unrepresentable in the `u64` API). |
| Integer / IntegerIncrementTests | 8 | 0 | 8 | The Rust `Int` API has no increment/decrement operator. |
| Integer / IntegerMultiplyTests | 14 | 1 | 13 | Only the (−5, 0) theory row matches the Rust zero test; 0×7/7×0/0×0, ±3×±4, ×1, 10^18 and −10^12 have no matching Rust input. |
| Integer / IntegerNegationTests | 10 | 1 | 9 | Rust `negate` tests only ±42 and zero; remaining C# cases use ±5, ±99, 3 and the unary operators, which Rust does not expose separately. |
| Integer / IntegerParseTests | 11 | 1 | 10 | Rust tests only "−0", "−00042", "12a3", "−", "+7" and a 40-digit literal; "42", "−42", "007", "", span "−7" and the 30-digit literals are untested; invalid input is an exception vs `None`. |
| Integer / IntegerPowTests | 6 | 0 | 6 | Rust `pow` tests (−2)^10, (−2)^11, (−7)^0, 0^9; C# tests 2^10, (−2)^4, (−2)^3, and three throwing cases. The zero-exponent and zero-base cases are **contradictory** (see below). |
| Integer / IntegerPredicateTests | 20 | 0 | 20 | Rust has no `IsEvenInteger`/`IsOddInteger` predicates and no direct `IsPositive`/`IsZero` tests; its sign test uses 9/−9/0 against C#'s 10/−10/0 and separate zero predicates. |
| Integer / IntegerSubtractTests | 12 | 1 | 11 | Rust `sub` cases are 3−10, −3−(−7), x−x and 10^18−1; remaining C# pairs (10−3, 3−(−4), −3−(−4), −3−4, 42−42, 17−0, 0−5, 100−37, 10^9−999999999) do not match. |
| Integer / IntegerToStringTests | 6 | 1 | 5 | Rust only asserts zero, −42 (via `from_i64`) and −98765432109876543210; 123, −123, TryFormat buffer behaviour and the format-provider overload have no Rust test. |
| Representation / all classes | 91 | 0 | 91 | No `lovelace-representation` crate is a workspace member. `DigitStore` (digits, snapshots, reset, trim, copy, thread safety) has no Rust type; `Nat`/`Int` use `Vec<u64>` with no digit-storage surface. |
| Complex / all classes | 121 | 0 | 121 | No `lovelace-complex` crate. The only Rust `Complex` is a helper with `new/zero/add/mul/scale` in `rust/lovelace-lang/src/value.rs:15-56`, has no tests, and exposes no exp/sqrt/trig/pi/parse/format/inexact-argument provenance surface. |
| Dsp / all classes | 61 | 0 | 61 | No `lovelace-dsp` crate. `lovelace-lang` does have `fft`/`dft`/`conv` builtins (`rust/lovelace-lang/src/builtins.rs:1848-1850`), but no Rust test in `parity/rust-tests.json` exercises them, and they expose none of the `DspMath`/`ISignal`/fixed-precision API that these 61 cases assert. |

### Specific near-miss candidates examined and rejected

These are the cases where a Rust test looked like a plausible counterpart but the bodies did not agree.

| C# case | Rust test considered | Why rejected |
|---|---|---|
| `Natural Factorial_GivenLargeValue` (21! = 51090942171709440000, `NaturalFactorialTests.cs:41-42`) | `factorial_GivenTwenty_ReturnsKnownValue` (`lib.rs:1067-1068`) | Different input (21 vs 20); 21! is nowhere asserted. |
| `Natural Pow_GivenLargeExponent` (2^64, `NaturalPowTests.cs:50-51`) | `pow_GivenTwoToHundred_ReturnsKnownValue` (`lib.rs:1044-1048`) | Different exponent (64 vs 100). The 2^64 literal appears only in `to_u64_GivenValueAboveMaximum_ReturnsNone`, which asserts `to_u64() == None`, not the power. |
| `Natural Pow_GivenKnownValue` (2^10 = 1024) | `pow_GivenTwoToHundred` | 2^10 is not tested; 2^100 is. |
| `Natural ShiftLeftDecimal_GivenK` (123456789 × 10^7, `NaturalBinaryLimbTests.cs:60`) | `mul_pow10_GivenNine` / `mul_pow10_GivenNineteen` (`lib.rs:1102-1115`) | Same operation family, but operands and exponent differ (123 × 10^9, 42 × 10^19); exponent 7 is never exercised. |
| `Natural RoundTrip_GivenRandomLargeValues` (30 values up to 20,000 digits) | `parse_then_format_GivenLargeLiteral_RoundTrips` (one 51-digit literal, `lib.rs:1131-1135`) | Rust is far weaker: one fixed literal instead of 30 random large values. |
| `Natural Mul_GivenLimbOverflow` ((2^64−1)^2) | `mul_GivenLimbBoundaryValues_ReturnsExactSquare` ((10^9−1)^2, `lib.rs:968-971`) | Different limb boundary; 2^64 is not exercised. |
| `Natural DivRem_GivenExactDivision` (12/4 → 3 r0) | `div_rem_GivenExactDivision_ReturnsZeroRemainder` (10^18/10^9, `lib.rs:996-1002`) | Different operands. |
| `Natural DivRem_GivenDivisorZero` (5/0 throws `DivideByZeroException`) | `div_rem_GivenZeroDivisor_ReturnsNone` (42/0, `lib.rs:984-986`) | Failure channel and identity differ: Rust returns `None`, not an exception of the asserted type. |
| `Natural Parse_GivenNonDigitCharacters` ("12a3" → `FormatException`) | `from_decimal_GivenNonDigit_ReturnsNone` ("12a3", `lib.rs:877-879`) | Same input and same refusal, but C# asserts a named exception; Rust asserts only `None`, so the failure identity is not covered. Same reason rejects `Constructor_GivenEmptyString` vs `from_decimal_GivenEmptyString`. |
| `Natural Constructor_GivenStringZero` ("0") | `from_decimal_GivenAllZeros_ReturnsZero` ("000000000000") | Different literal; "0" itself is never passed. |
| `Natural Constructor_GivenPositiveInt_MatchesExpectedString` case (1, "1") | `from_u64_GivenOne_IsOne` (`lib.rs:855-857`) | Rust asserts only `is_one()`; it does not assert the decimal rendering "1" that the C# case requires. |
| `Integer Pow_GivenZeroExponent_Throws` (2^0 throws `ArgumentOutOfRangeException`) | `pow_GivenZeroExponent_ReturnsOne` (`lib.rs:575-577`) | **Direct contradiction**: C# requires a throw, Rust returns 1. This is the strongest rejection. |
| `Integer Pow_GivenZeroBase_Throws` (0^3 throws) | `pow_GivenZeroBaseAndPositiveExponent_ReturnsZero` (`lib.rs:580-584`) | **Direct contradiction**: C# throws, Rust returns 0. |
| `Integer Pow_GivenNegativeBaseAndEvenExponent` ((−2)^4 = 16) | `pow_GivenNegativeBaseEvenExponent` ((−2)^10 = 1024) | Exponent differs (4 vs 10). |
| `Integer Pow_GivenNegativeBaseAndOddExponent` ((−2)^3 = −8) | `pow_GivenNegativeBaseOddExponent` ((−2)^11 = −2048) | Exponent differs (3 vs 11). |
| `Integer Add_GivenPositiveAndNegativeWherePositiveLarger` (10 + (−3) = 7) | `add_GivenOppositeSigns_LargerPositive` ((−3) + 10, `lib.rs:420-422`) | Same pair but operand order is reversed; Rust asserts only the (−3)+10 direction. Rejected to keep the "same inputs" rule strict. |
| `Integer CompareTo_GivenPositiveGreaterThanNegative` (1 > −1) | `ord_GivenNegativeAndPositive` (−1 < 1, `lib.rs:616-617`) | Mirror of the relation; Rust does not assert the `1 > −1` direction. |
| `Integer CompareTo_GivenTwoNegatives` (−10 vs −3) | `ord_GivenTwoNegatives` (−10 vs −2, `lib.rs:621-623`) | Second operand differs (−3 vs −2). |
| `Integer Sign_*` (10/−10/0) | `sign_GivenPositiveNegativeAndZero` (9/−9/0, `lib.rs:636-640`) | Representative values differ; Rust does not test 10/−10. |
| `Integer Parse_GivenLeadingMinus` ("−42" → −42) | `from_decimal_GivenLeadingZerosAndMinus` ("−00042" → −42) | Different literal; parsing plain "−42" is not tested (the `from_i64(-42)` test is a different operation). |
| `Integer Constructor_GivenPositiveLong` (42 → "42", positive) | `from_i64_GivenNegativeValue` / `sign_Given...` | No Rust test renders +42 or asserts it is positive; the 42 in the Rust tests is negated. |
| `Integer Multiply_GivenZeroOperand` #0, #1, #3 ((0,7), (7,0), (0,0)) | `mul_GivenZeroOperand_ReturnsNonNegativeZero` (−5, 0) | Only the (−5, 0) row has the exact Rust operands; the other zero rows are not exercised. |
| `Integer Multiply_GivenOne` (×1 for 42, −7, 0) | none | Rust `Int` has no multiply-by-one test. |
| `Integer Factorial_GivenOne` (1! = 1) | none | Rust `Int::factorial` tests only 0 and 10. |
| `Integer Factorial_GivenNegativeInteger_Throws` | none | `Int::factorial` takes `u64`; a negative input is unrepresentable and no rejection test exists. |
| `Integer ToString_GivenNegativeValue` (−123) | `to_decimal_GivenNegativeLarge` (−98765432109876543210) | Different value; −123 is not asserted. |
| `Integer Constructor_GivenInvalidString` / `Parse_GivenInvalidInput` / `Parse_GivenEmptyString` | `from_decimal_GivenNonDigit_ReturnsNone` / `from_decimal_GivenEmptyString_ReturnsNone` | Failure identity differs: `FormatException` vs `None`. |
| `Integer Increment_*` / `Decrement_*` (8 cases) | none | No increment/decrement API or test in Rust `Int`. |

## Coverage added

- Accepted in `parity/manifest-map.json`: **20** of **6,209** census cases = **0.322 %** (`20 / 6209`).
- Within the five pilot projects: **20 / 616 = 3.25 %**.
- Per project: Natural **6/195**, Integer **14/148**, Representation **0/91**, Complex **0/121**, Dsp **0/61**.
- All other 6,189 census rows remain `deferred`; `parity/manifest.toml` was not edited. Regenerating it
  (`node parity/gen-manifest.mjs`) will pick up these 20 curated rows and keep the deferred default for
  the rest.

## What was NOT verified

- The Rust suite was **not re-run** by this audit. Existence and pass status are taken from
  `parity/rust-tests.json` (`suite.exitCode = 0`, 293 passed). The mappings are claims about test bodies,
  not a fresh test execution.
- The C# tests were **not executed**; inputs and expectations were read from source only.
- The remaining 596 cases in the five pilot projects were not adjudicated one by one against a candidate
  Rust body; they are rejected at class level with the reasons above. In particular, no claim is made that
  a class-level rejection could not contain a future valid mapping with a different Rust test.
- For Representation, Complex and Dsp, absence of a Rust crate was established from the workspace
  `Cargo.toml` and a repository-wide search for `Complex`, `DigitStore`, `Fft`, `Dsp`; absence of tests is
  a negative claim bounded to this checkout.
- Line numbers refer to the working tree at audit time; they may shift if the cited files change.
