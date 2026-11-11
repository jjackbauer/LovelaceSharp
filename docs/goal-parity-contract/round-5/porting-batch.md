# Round 5 porting batch — Array, Integer, Natural (+ Real sample)

Census oracle: `1cc28f255eca6029849f880949fc7be1d40ab8af` (6,209 C# test cases).
Baseline: 20 ported cases (0.32%). Result: **262 ported (4.22%)**, of which **242 are new verified mappings**
and **157 are new Rust tests** added in this batch.

## Mappings added per crate

| Rust crate | New Rust tests | New mappings | Mapped total | Notes |
|---|---:|---:|---:|---|
| `lovelace-array` | 8 | 11 | 11 | 8 new tests; 3 C# cases matched an existing Rust test exactly |
| `lovelace-int` | 73 | 100 | 114 | 14 pre-existing mappings kept |
| `lovelace-nat` | 72 | 125 | 131 | 6 pre-existing mappings kept |
| `lovelace-real` | 4 | 6 | 6 | sampled: `RealToStringTests` only |
| **total** | **157** | **242** | **262** | coverage **4.22%** |

Every accepted mapping is recorded in `parity/manifest-map.json` with a note giving the C# file:line,
the concrete assertion, and the Rust source line of the test. A mapping points either at a new test
(most rows) or at an existing Rust test that already asserted the same concrete value (Array reshape /
transpose / concat, Integer parse rejection, Natural default/ulong-max/factorial).

### What was ported

- **Lovelace.Array.Tests (11/19).** Shape/rank/numel/strides of the {2,2,2} cube; mismatched data;
  full and partial indices; partial index returning a lower rank; reshape success and mismatch;
  transpose (reverse axes and explicit {2,0,1} permutation); squeeze; concat on axis 0.
- **Lovelace.Integer.Tests (114/148).** `Add`, `Subtract`, `Multiply`, `DivRem`/`div_rem_signed`,
  `Negate`, `Pow`, `Factorial`, parse/format, equality/ordering/sign, and the concrete theory rows
  (multiply-by-zero, multiply-by-one, sign table). Operators are exercised through the methods they
  delegate to, because Rust `Int` has no operator overloads.
- **Lovelace.Natural.Tests (131/195).** `Add`, `Sub`, `Mul`, `DivRem`, `Pow`, `Factorial`, parse/format,
  clone independence, hash consistency, and `mul_pow10`; all concrete theory rows and the limb-boundary
  values (`2^64-1`, `(2^64-1)^2`, `2^64`, `10^150 * 10^150`).
- **Lovelace.Real.Tests (6/2537, sampled).** `ToString` of `"42"`, `"3.14"`, `"0.0005"`, `"-1.5"`,
  and the periodic round-trips `"0.(3)"` / `"0.1(6)"`.

## Porting rules applied

1. The C# assertion is the contract: each Rust test reproduces the same concrete input and the same
   concrete expected value (or a strictly stronger one, e.g. exact decimal rendering in addition to sign).
2. Where C# throws and Rust returns an `Option`/`Result`, the case was mapped only when the same input
   is rejected with a concrete value (`ArrayError::ShapeMismatch`, `ArrayError::IndexOutOfBounds`-class
   `None`, `None`) and the note records the substitution. Cases where the *result* or the input domain
   differs were refused (see below), e.g. `Integer.Pow(0)` throws in C# but returns 1 in Rust.
3. This batch changed no production code. Its diff touches only `#[cfg(test)]` modules inside the
   five permitted crates and `parity/manifest-map.json`; the `lovelace-array`, `lovelace-int` and
   `lovelace-nat` production prefixes are byte-identical to `HEAD`. The working tree already carried
   uncommitted production edits in other files before this batch (e.g. `lovelace-real` provenance
   fields); those pre-date this batch and were not modified here.
4. No pre-existing Rust test was removed (verified by comparing test-function names against `HEAD`).

## Verification

### Observed `cargo test`

Abridged: the binary hash suffix and the repeated doc-test blocks are elided; the per-crate
`test result` lines are verbatim.

```
$ cargo test --manifest-path rust/Cargo.toml --workspace
     Running unittests src\lib.rs (rust\target\debug\deps\lovelace_array-...exe)
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src\main.rs (rust\target\debug\deps\lovelace_bench-...exe)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src\main.rs (rust\target\debug\deps\lovelace_corpus-...exe)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src\main.rs (rust\target\debug\deps\lovelace_desktop-...exe)
test result: ok. 67 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running unittests src\lib.rs (rust\target\debug\deps\lovelace_int-...exe)
test result: ok. 125 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running unittests src\lib.rs (rust\target\debug\deps\lovelace_json-...exe)
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src\lib.rs (rust\target\debug\deps\lovelace_lang-...exe)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src\lib.rs (rust\target\debug\deps\lovelace_nat-...exe)
test result: ok. 134 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s
     Running unittests src\lib.rs (rust\target\debug\deps\lovelace_real-...exe)
test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src\main.rs (rust\target\debug\deps\lovelace_run-...exe)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src\lib.rs (rust\target\debug\deps\lovelace_symbolics-...exe)
test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests ... (all crates)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Sum of the test-result lines: **450 passed, 0 failed** (inventory: `crates=11 tests=450 suiteExit=0
passed=450 failed=0`).

### Observed manifest gate

```
$ node parity/gen-manifest.mjs
{
  "rows": 6209,
  "dispositions": {
    "deferred": 5947,
    "ported": 262
  },
  "coveragePercent": 4.22,
  "rustTests": 450
}

$ node parity/tools/gate-manifest.mjs
{
  "censusCases": 6209,
  "manifestRows": 6209,
  "dispositions": {
    "ported": 262,
    "consolidated": 0,
    "excluded": 0,
    "deferred": 5947
  },
  "coveragePercent": 4.22,
  "claimedRustTests": 262,
  "projectsInCensus": 15,
  "failures": []
}
PASS (structural): 6209 test cases dispositioned; 262 claim Rust coverage (4.22%).
```

## Refusals

Rows group a C# test method; `#0..#N` means the theory rows of that method. All were refused because
the Rust crate cannot express the assertion without a production change, the behaviour differs, or the
input/oracle does not exist in Rust.

| Census id | C# file:line | Reason |
|---|---|---|
| Lovelace.Array.Tests/Lovelace.Array.Tests.NdArrayTests/Det_Inverse_Trace#0 | Lovelace.Array.Tests/NdArrayTests.cs:185 | Rust lovelace-array has no ArrayMath (Sum/Min/Max/Mean/Norm/Dot/Cross/MatMul/Det/Trace/Inverse); the crate exposes NdArray reductions only. Porting it would require new production code (out of scope). |
| Lovelace.Array.Tests/Lovelace.Array.Tests.NdArrayTests/DotAndCross#0 | Lovelace.Array.Tests/NdArrayTests.cs:154 | Rust lovelace-array has no ArrayMath (Sum/Min/Max/Mean/Norm/Dot/Cross/MatMul/Det/Trace/Inverse); the crate exposes NdArray reductions only. Porting it would require new production code (out of scope). |
| Lovelace.Array.Tests/Lovelace.Array.Tests.NdArrayTests/Inverse_Singular_Throws#0 | Lovelace.Array.Tests/NdArrayTests.cs:197 | Rust lovelace-array has no ArrayMath (Sum/Min/Max/Mean/Norm/Dot/Cross/MatMul/Det/Trace/Inverse); the crate exposes NdArray reductions only. Porting it would require new production code (out of scope). |
| Lovelace.Array.Tests/Lovelace.Array.Tests.NdArrayTests/MatMul_2x2#0 | Lovelace.Array.Tests/NdArrayTests.cs:166 | Rust lovelace-array has no ArrayMath (Sum/Min/Max/Mean/Norm/Dot/Cross/MatMul/Det/Trace/Inverse); the crate exposes NdArray reductions only. Porting it would require new production code (out of scope). |
| Lovelace.Array.Tests/Lovelace.Array.Tests.NdArrayTests/MatMul_Batched#0 | Lovelace.Array.Tests/NdArrayTests.cs:176 | Rust lovelace-array has no ArrayMath (Sum/Min/Max/Mean/Norm/Dot/Cross/MatMul/Det/Trace/Inverse); the crate exposes NdArray reductions only. Porting it would require new production code (out of scope). |
| Lovelace.Array.Tests/Lovelace.Array.Tests.NdArrayTests/MeanAndNorm#0 | Lovelace.Array.Tests/NdArrayTests.cs:145 | Rust lovelace-array has no ArrayMath (Sum/Min/Max/Mean/Norm/Dot/Cross/MatMul/Det/Trace/Inverse); the crate exposes NdArray reductions only. Porting it would require new production code (out of scope). |
| Lovelace.Array.Tests/Lovelace.Array.Tests.NdArrayTests/MinMax#0 | Lovelace.Array.Tests/NdArrayTests.cs:137 | Rust lovelace-array has no ArrayMath (Sum/Min/Max/Mean/Norm/Dot/Cross/MatMul/Det/Trace/Inverse); the crate exposes NdArray reductions only. Porting it would require new production code (out of scope). |
| Lovelace.Array.Tests/Lovelace.Array.Tests.NdArrayTests/Sum_AllAndAxis#0 | Lovelace.Array.Tests/NdArrayTests.cs:128 | Rust lovelace-array has no ArrayMath (Sum/Min/Max/Mean/Norm/Dot/Cross/MatMul/Det/Trace/Inverse); the crate exposes NdArray reductions only. Porting it would require new production code (out of scope). |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerConstructorTests/Constructor_GivenInt_DelegatesToLongOverload#0 | Lovelace.Integer.Tests/IntegerConstructorTests.cs:72 | Rust has a single from_i64; there is no int/long overload pair to compare. |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerConstructorTests/Constructor_GivenNatural_IsAlwaysPositive#0 | Lovelace.Integer.Tests/IntegerConstructorTests.cs:163 | Rust Int has no public Natural-to-Int conversion and no raw (magnitude, sign) constructor; the fields are private, so the test cannot be expressed without production changes. |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerConstructorTests/Constructor_GivenNonZeroMagnitudeAndIsNegativeTrue_IsNegative#0 | Lovelace.Integer.Tests/IntegerConstructorTests.cs:183 | Rust Int has no public Natural-to-Int conversion and no raw (magnitude, sign) constructor; the fields are private, so the test cannot be expressed without production changes. |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerConstructorTests/Constructor_GivenZeroMagnitudeAndIsNegativeTrue_NormalisedToPositive#0 | Lovelace.Integer.Tests/IntegerConstructorTests.cs:191 | Rust Int has no public Natural-to-Int conversion and no raw (magnitude, sign) constructor; the fields are private, so the test cannot be expressed without production changes. |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerConstructorTests/Constructor_GivenZeroNatural_IsZero#0 | Lovelace.Integer.Tests/IntegerConstructorTests.cs:171 | Rust Int has no public Natural-to-Int conversion and no raw (magnitude, sign) constructor; the fields are private, so the test cannot be expressed without production changes. |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerIncrementTests/Decrement_GivenNegativeValue_DecrementsFurtherNegative#0 | Lovelace.Integer.Tests/IntegerIncrementTests.cs:48 | Rust Int has no increment/decrement method or ++/--; not expressible without production code. |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerIncrementTests/Decrement_GivenPositiveValue_DecrementsByOne#0 | Lovelace.Integer.Tests/IntegerIncrementTests.cs:34 | Rust Int has no increment/decrement method or ++/--; not expressible without production code. |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerIncrementTests/Decrement_GivenZero_ReturnsNegativeOne#0 | Lovelace.Integer.Tests/IntegerIncrementTests.cs:41 | Rust Int has no increment/decrement method or ++/--; not expressible without production code. |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerEqualityTests/Equals_GivenReferenceToSelf_ReturnsTrue#0 | Lovelace.Integer.Tests/IntegerEqualityTests.cs:40 | C# asserts reference identity of a class instance; Rust Int is a value type, so x == x is a tautology with no distinct assertion. |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerFactorialTests/Factorial_GivenNegativeInteger_Throws#0 | Lovelace.Integer.Tests/IntegerFactorialTests.cs:27 | Rust Int::factorial takes u64; a negative input is not representable (C# throws InvalidOperationException). |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerIncrementTests/Increment_GivenNegativeValue_MovesTowardsZero#0 | Lovelace.Integer.Tests/IntegerIncrementTests.cs:20 | Rust Int has no increment/decrement method or ++/--; not expressible without production code. |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerIncrementTests/Increment_GivenPositiveValue_IncrementsByOne#0 | Lovelace.Integer.Tests/IntegerIncrementTests.cs:13 | Rust Int has no increment/decrement method or ++/--; not expressible without production code. |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerIncrementTests/Increment_GivenZero_ReturnsOne#0 | Lovelace.Integer.Tests/IntegerIncrementTests.cs:27 | Rust Int has no increment/decrement method or ++/--; not expressible without production code. |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerPredicateTests/IsEvenInteger_GivenEvenNegativeValue_ReturnsTrue#0 | Lovelace.Integer.Tests/IntegerPredicateTests.cs:109 | Rust Int has no even/odd predicate; not expressible without production code. |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerPredicateTests/IsEvenInteger_GivenEvenPositiveValue_ReturnsTrue#0 | Lovelace.Integer.Tests/IntegerPredicateTests.cs:95 | Rust Int has no even/odd predicate; not expressible without production code. |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerPredicateTests/IsEvenInteger_GivenOddValue_ReturnsFalse#0 | Lovelace.Integer.Tests/IntegerPredicateTests.cs:102 | Rust Int has no even/odd predicate; not expressible without production code. |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerPredicateTests/IsEvenInteger_GivenZero_ReturnsTrue#0 | Lovelace.Integer.Tests/IntegerPredicateTests.cs:117 | Rust Int has no even/odd predicate; not expressible without production code. |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerPredicateTests/IsOddInteger_GivenEvenValue_ReturnsFalse#0 | Lovelace.Integer.Tests/IntegerPredicateTests.cs:143 | Rust Int has no even/odd predicate; not expressible without production code. |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerPredicateTests/IsOddInteger_GivenOddNegativeValue_ReturnsTrue#0 | Lovelace.Integer.Tests/IntegerPredicateTests.cs:136 | Rust Int has no even/odd predicate; not expressible without production code. |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerPredicateTests/IsOddInteger_GivenOddPositiveValue_ReturnsTrue#0 | Lovelace.Integer.Tests/IntegerPredicateTests.cs:129 | Rust Int has no even/odd predicate; not expressible without production code. |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerPredicateTests/IsOddInteger_GivenZero_ReturnsFalse#0 | Lovelace.Integer.Tests/IntegerPredicateTests.cs:150 | Rust Int has no even/odd predicate; not expressible without production code. |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerMultiplyTests/OperatorStar_ProducesSameResultAsMultiplyMethod#0 | Lovelace.Integer.Tests/IntegerMultiplyTests.cs:140 | C# asserts the operator agrees with the named method; Rust has no operator overloads, so the two-path equality has no analogue. |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerPowTests/Pow_GivenNegativeExponent_Throws#0 | Lovelace.Integer.Tests/IntegerPowTests.cs:42 | Rust pow takes u64; a negative exponent is not representable (C# throws ArgumentOutOfRangeException). |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerPowTests/Pow_GivenZeroBase_Throws#0 | Lovelace.Integer.Tests/IntegerPowTests.cs:48 | C# throws ArgumentOutOfRangeException; Rust Int::zero().pow(3) returns 0. Behaviour differs, so refused (rule 2). |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerPowTests/Pow_GivenZeroExponent_Throws#0 | Lovelace.Integer.Tests/IntegerPowTests.cs:36 | C# throws ArgumentOutOfRangeException; Rust pow(0) returns 1. Behaviour differs, so refused (rule 2). |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerIncrementTests/PreDecrementOperator_MutatesAndReturnsNewValue#0 | Lovelace.Integer.Tests/IntegerIncrementTests.cs:64 | Rust Int has no increment/decrement method or ++/--; not expressible without production code. |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerIncrementTests/PreIncrementOperator_MutatesAndReturnsNewValue#0 | Lovelace.Integer.Tests/IntegerIncrementTests.cs:55 | Rust Int has no increment/decrement method or ++/--; not expressible without production code. |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerSubtractTests/SubtractOperator_IsEquivalentToSubtractMethod#0 | Lovelace.Integer.Tests/IntegerSubtractTests.cs:112 | C# asserts the operator agrees with the named method; Rust has no operator overloads, so the two-path equality has no analogue. |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerToStringTests/ToString_WithFormatProvider_ReturnsSameAsToString#0 | Lovelace.Integer.Tests/IntegerToStringTests.cs:51 | Rust Int has no format-provider overload; to_decimal() is the only rendering path. |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerToStringTests/TryFormat_GivenInsufficientBuffer_ReturnsFalse#0 | Lovelace.Integer.Tests/IntegerToStringTests.cs:41 | Rust Int has no buffer-writing TryFormat API. |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerToStringTests/TryFormat_GivenSufficientBuffer_WritesRepresentationAndReturnsTrue#0 | Lovelace.Integer.Tests/IntegerToStringTests.cs:31 | Rust Int has no buffer-writing TryFormat API. |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerNegationTests/UnaryPlus_GivenNegativeValue_ReturnsCopyWithSameSign#0 | Lovelace.Integer.Tests/IntegerNegationTests.cs:96 | Rust has no unary +; the C# assertion only restates identity (+n == n), so there is no distinct behaviour to port. |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerNegationTests/UnaryPlus_GivenPositiveValue_ReturnsCopyWithSameSign#0 | Lovelace.Integer.Tests/IntegerNegationTests.cs:87 | Rust has no unary +; the C# assertion only restates identity (+n == n), so there is no distinct behaviour to port. |
| Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerNegationTests/UnaryPlus_GivenZero_ReturnsZero#0 | Lovelace.Integer.Tests/IntegerNegationTests.cs:105 | Rust has no unary +; the C# assertion only restates identity (+n == n), so there is no distinct behaviour to port. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalDecimalCacheTests/ArithmeticResult_ToString_CachesAfterFirstCall#0 | Lovelace.Natural.Tests/NaturalDecimalCacheTests.cs:46 | Asserts Assert.Same reference identity of the cached decimal string (a C# cache detail); Rust to_decimal() returns a fresh String each call. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalDecimalCacheTests/ConcurrentArithmeticOnLazyParsedValue_MatchesBigInteger#0 | Lovelace.Natural.Tests/NaturalDecimalCacheTests.cs:128 | Asserts thread-safety of the C# lazy decimal/limb cache via Parallel.For; Rust Nat is an immutable value with no shared mutable cache, so the race cannot be reproduced. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalDecimalCacheTests/ConcurrentParseToString_GivenManyValues_MatchesBigInteger#0 | Lovelace.Natural.Tests/NaturalDecimalCacheTests.cs:107 | Asserts thread-safety of the C# lazy decimal/limb cache via Parallel.For; Rust Nat is an immutable value with no shared mutable cache, so the race cannot be reproduced. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalDecimalCacheTests/ConcurrentToString_GivenSharedValue_ReturnsConsistentString#0 | Lovelace.Natural.Tests/NaturalDecimalCacheTests.cs:89 | Asserts thread-safety of the C# lazy decimal/limb cache via Parallel.For; Rust Nat is an immutable value with no shared mutable cache, so the race cannot be reproduced. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalConstructorTests/Constructor_GivenNegativeInt_ThrowsArgumentOutOfRangeException#0 | Lovelace.Natural.Tests/NaturalConstructorTests.cs:82 | Rust Nat construction takes u64; a negative input is not representable (C# throws ArgumentOutOfRangeException). |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalDecimalCacheTests/CopyConstructor_SharesCachedStringButIndependentLimbs#0 | Lovelace.Natural.Tests/NaturalDecimalCacheTests.cs:73 | Asserts Assert.Same reference identity of the cached decimal string (a C# cache detail); Rust to_decimal() returns a fresh String each call. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalRandomizedCrossCheckTests/CrossCheck_GivenAdversarialCarryPatterns_MatchesBigInteger#0 | Lovelace.Natural.Tests/NaturalRandomizedCrossCheckTests.cs:60 | Expected values come from .NET BigInteger over randomized operands; the Rust crate has no independent BigInteger oracle and the inputs are not concrete literals. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalRandomizedCrossCheckTests/CrossCheck_GivenRandomOperands_MatchesBigInteger#0..#4 | Lovelace.Natural.Tests/NaturalRandomizedCrossCheckTests.cs:21 | Expected values come from .NET BigInteger over randomized operands; the Rust crate has no independent BigInteger oracle and the inputs are not concrete literals. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalSubtractTests/DecrementPostfix_GivenValue_ReturnsPreviousValue#0 | Lovelace.Natural.Tests/NaturalSubtractTests.cs:128 | Rust Nat has no decrement method or operator. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalSubtractTests/DecrementPrefix_GivenLargeCarry_ProducesCorrectResult#0 | Lovelace.Natural.Tests/NaturalSubtractTests.cs:119 | Rust Nat has no decrement method or operator. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalSubtractTests/DecrementPrefix_GivenOne_ReturnsZero#0 | Lovelace.Natural.Tests/NaturalSubtractTests.cs:101 | Rust Nat has no decrement method or operator. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalSubtractTests/DecrementPrefix_GivenTen_ReturnsNine#0 | Lovelace.Natural.Tests/NaturalSubtractTests.cs:110 | Rust Nat has no decrement method or operator. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalSubtractTests/DecrementPrefix_GivenZero_ThrowsInvalidOperationException#0 | Lovelace.Natural.Tests/NaturalSubtractTests.cs:137 | Rust Nat has no decrement method or operator. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalStaticPropertyTests/DisplayDigits_AfterMultipleSets_ReturnsLastValue#0 | Lovelace.Natural.Tests/NaturalStaticPropertyTests.cs:74 | Rust Nat has no mutable static DisplayDigits/Precision configuration; the properties do not exist. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalStaticPropertyTests/DisplayDigits_AfterSet_ReturnsSetValue#0 | Lovelace.Natural.Tests/NaturalStaticPropertyTests.cs:59 | Rust Nat has no mutable static DisplayDigits/Precision configuration; the properties do not exist. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalStaticPropertyTests/DisplayDigits_ConcurrentReadWrite_NeverReturnsTornValue#0 | Lovelace.Natural.Tests/NaturalStaticPropertyTests.cs:98 | Rust Nat has no mutable static DisplayDigits/Precision configuration; the properties do not exist. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalStaticPropertyTests/DisplayDigits_DefaultValue_IsMinusOne#0 | Lovelace.Natural.Tests/NaturalStaticPropertyTests.cs:45 | Rust Nat has no mutable static DisplayDigits/Precision configuration; the properties do not exist. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalBinaryLimbTests/DivRem_GivenLargeOperands_ProducesCorrectQuotientAndRemainder#0 | Lovelace.Natural.Tests/NaturalBinaryLimbTests.cs:43 | Expected values come from .NET BigInteger over randomized operands; the Rust crate has no independent BigInteger oracle and the inputs are not concrete literals. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalRandomizedCrossCheckTests/DivRem_GivenOverflowAlignment_MatchesBigInteger#0 | Lovelace.Natural.Tests/NaturalRandomizedCrossCheckTests.cs:91 | Expected values come from .NET BigInteger over randomized operands; the Rust crate has no independent BigInteger oracle and the inputs are not concrete literals. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalRandomizedCrossCheckTests/DivRem_GivenPowerOfTenDivisor_MatchesBigInteger#0 | Lovelace.Natural.Tests/NaturalRandomizedCrossCheckTests.cs:109 | Expected values come from .NET BigInteger over randomized operands; the Rust crate has no independent BigInteger oracle and the inputs are not concrete literals. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalRandomizedCrossCheckTests/Factorial_GivenN_MatchesBigInteger#0..#4 | Lovelace.Natural.Tests/NaturalRandomizedCrossCheckTests.cs:53 | Expected values come from .NET BigInteger over randomized operands; the Rust crate has no independent BigInteger oracle and the inputs are not concrete literals. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalIncrementTests/IncrementPostfix_GivenValue_ReturnsPreviousValue#0 | Lovelace.Natural.Tests/NaturalIncrementTests.cs:59 | Rust Nat has no increment method or operator. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalIncrementTests/IncrementPostfix_GivenZero_ReturnsPreviousValueAndIncrementsVariable#0 | Lovelace.Natural.Tests/NaturalIncrementTests.cs:72 | Rust Nat has no increment method or operator. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalIncrementTests/IncrementPrefix_AppliedMultipleTimes_AccumulatesCorrectly#0 | Lovelace.Natural.Tests/NaturalIncrementTests.cs:86 | Rust Nat has no increment method or operator. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalIncrementTests/IncrementPrefix_GivenAllNines_PropagatesCarryCorrectly#0..#2 | Lovelace.Natural.Tests/NaturalIncrementTests.cs:39 | Rust Nat has no increment method or operator. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalIncrementTests/IncrementPrefix_GivenArbitraryValue_ReturnsValuePlusOne#0 | Lovelace.Natural.Tests/NaturalIncrementTests.cs:47 | Rust Nat has no increment method or operator. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalIncrementTests/IncrementPrefix_GivenNine_ReturnsTen#0 | Lovelace.Natural.Tests/NaturalIncrementTests.cs:27 | Rust Nat has no increment method or operator. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalIncrementTests/IncrementPrefix_GivenZero_ReturnsOne#0 | Lovelace.Natural.Tests/NaturalIncrementTests.cs:19 | Rust Nat has no increment method or operator. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalMultiplyTests/Multiply_GivenLargeMultiplicationsRunConcurrently_AllReturnCorrectResults#0 | Lovelace.Natural.Tests/NaturalMultiplyTests.cs:212 | C# stresses a parallel multiplication path over shared state; Rust Nat is immutable, so there is no shared-state race to assert. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalMultiplyTests/Multiply_GivenSharedLargeOperandReadConcurrently_AllResultsConsistent#0 | Lovelace.Natural.Tests/NaturalMultiplyTests.cs:191 | C# stresses a parallel multiplication path over shared state; Rust Nat is immutable, so there is no shared-state race to assert. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalStructureTests/Natural_ImplementsIAdditionOperators#0 | Lovelace.Natural.Tests/NaturalStructureTests.cs:58 | Asserts CLR interface implementation or the C# type name; Rust has no runtime analogue (traits are compile-time) and the type is already Nat. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalStructureTests/Natural_ImplementsIComparable#0 | Lovelace.Natural.Tests/NaturalStructureTests.cs:28 | Asserts CLR interface implementation or the C# type name; Rust has no runtime analogue (traits are compile-time) and the type is already Nat. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalStructureTests/Natural_ImplementsIComparisonOperators#0 | Lovelace.Natural.Tests/NaturalStructureTests.cs:100 | Asserts CLR interface implementation or the C# type name; Rust has no runtime analogue (traits are compile-time) and the type is already Nat. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalStructureTests/Natural_ImplementsIDecrementOperators#0 | Lovelace.Natural.Tests/NaturalStructureTests.cs:94 | Asserts CLR interface implementation or the C# type name; Rust has no runtime analogue (traits are compile-time) and the type is already Nat. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalStructureTests/Natural_ImplementsIDivisionOperators#0 | Lovelace.Natural.Tests/NaturalStructureTests.cs:76 | Asserts CLR interface implementation or the C# type name; Rust has no runtime analogue (traits are compile-time) and the type is already Nat. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalStructureTests/Natural_ImplementsIEquatable#0 | Lovelace.Natural.Tests/NaturalStructureTests.cs:22 | Asserts CLR interface implementation or the C# type name; Rust has no runtime analogue (traits are compile-time) and the type is already Nat. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalStructureTests/Natural_ImplementsIIncrementOperators#0 | Lovelace.Natural.Tests/NaturalStructureTests.cs:88 | Asserts CLR interface implementation or the C# type name; Rust has no runtime analogue (traits are compile-time) and the type is already Nat. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalStructureTests/Natural_ImplementsIModulusOperators#0 | Lovelace.Natural.Tests/NaturalStructureTests.cs:82 | Asserts CLR interface implementation or the C# type name; Rust has no runtime analogue (traits are compile-time) and the type is already Nat. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalStructureTests/Natural_ImplementsIMultiplyOperators#0 | Lovelace.Natural.Tests/NaturalStructureTests.cs:70 | Asserts CLR interface implementation or the C# type name; Rust has no runtime analogue (traits are compile-time) and the type is already Nat. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalStructureTests/Natural_ImplementsINumber#0 | Lovelace.Natural.Tests/NaturalStructureTests.cs:52 | Asserts CLR interface implementation or the C# type name; Rust has no runtime analogue (traits are compile-time) and the type is already Nat. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalStructureTests/Natural_ImplementsIParsable#0 | Lovelace.Natural.Tests/NaturalStructureTests.cs:34 | Asserts CLR interface implementation or the C# type name; Rust has no runtime analogue (traits are compile-time) and the type is already Nat. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalStructureTests/Natural_ImplementsISpanFormattable#0 | Lovelace.Natural.Tests/NaturalStructureTests.cs:46 | Asserts CLR interface implementation or the C# type name; Rust has no runtime analogue (traits are compile-time) and the type is already Nat. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalStructureTests/Natural_ImplementsISpanParsable#0 | Lovelace.Natural.Tests/NaturalStructureTests.cs:40 | Asserts CLR interface implementation or the C# type name; Rust has no runtime analogue (traits are compile-time) and the type is already Nat. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalStructureTests/Natural_ImplementsISubtractionOperators#0 | Lovelace.Natural.Tests/NaturalStructureTests.cs:64 | Asserts CLR interface implementation or the C# type name; Rust has no runtime analogue (traits are compile-time) and the type is already Nat. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalStructureTests/Natural_TypeName_IsNatural#0 | Lovelace.Natural.Tests/NaturalStructureTests.cs:16 | Asserts CLR interface implementation or the C# type name; Rust has no runtime analogue (traits are compile-time) and the type is already Nat. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalParseTests/Parse_GivenNullString_ThrowsFormatException#0 | Lovelace.Natural.Tests/NaturalParseTests.cs:78 | Rust &str cannot be null; the rejected input is not representable. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalStaticPropertyTests/Precision_AfterMultipleSets_ReturnsLastValue#0 | Lovelace.Natural.Tests/NaturalStaticPropertyTests.cs:174 | Rust Nat has no mutable static DisplayDigits/Precision configuration; the properties do not exist. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalStaticPropertyTests/Precision_AfterSet_ReturnsSetValue#0 | Lovelace.Natural.Tests/NaturalStaticPropertyTests.cs:159 | Rust Nat has no mutable static DisplayDigits/Precision configuration; the properties do not exist. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalStaticPropertyTests/Precision_ConcurrentReadWrite_NeverReturnsTornValue#0 | Lovelace.Natural.Tests/NaturalStaticPropertyTests.cs:194 | Rust Nat has no mutable static DisplayDigits/Precision configuration; the properties do not exist. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalStaticPropertyTests/Precision_DefaultValue_IsMinusOne#0 | Lovelace.Natural.Tests/NaturalStaticPropertyTests.cs:145 | Rust Nat has no mutable static DisplayDigits/Precision configuration; the properties do not exist. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalBinaryLimbTests/RoundTrip_GivenRandomLargeValues_ParseToStringIdentity#0 | Lovelace.Natural.Tests/NaturalBinaryLimbTests.cs:65 | Inputs come from a seeded .NET RNG; there is no concrete expected literal and no independent oracle in the Rust crate. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalDecimalCacheTests/RoundTrip_GivenRandomLargeValues_ParseToStringIdentity#0 | Lovelace.Natural.Tests/NaturalDecimalCacheTests.cs:35 | Inputs come from a seeded .NET RNG; there is no concrete expected literal and no independent oracle in the Rust crate. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalDecimalCacheTests/ToString_GivenParsedValue_IsStableAcrossRepeatedCalls#0 | Lovelace.Natural.Tests/NaturalDecimalCacheTests.cs:26 | Asserts Assert.Same reference identity of the cached decimal string (a C# cache detail); Rust to_decimal() returns a fresh String each call. |
| Lovelace.Natural.Tests/Lovelace.Natural.Tests.NaturalParseTests/TryParse_GivenNullString_ReturnsFalse#0 | Lovelace.Natural.Tests/NaturalParseTests.cs:122 | Rust &str cannot be null; the rejected input is not representable. |

### Lovelace.Representation.Tests — all 91 refused

The C# `DigitStore` is a mutable, thread-safe nibble buffer (`DigitCount`, `ByteCount`, `GetDigit`,
`SetDigit`, `GetBitwise`, `SetBitwise`, `GrowDigits`, `ShrinkDigits`, `TrimLeadingZeros`,
`SnapshotDigits`, `CopyDigitsFrom`, separator-aware `ToString`). Rust `lovelace-nat` stores private
little-endian base-10^9 limbs and exposes no digit-level mutable store, so none of these tests can be
expressed without new production code.

| C# test class | Cases | Example census id | Reason |
|---|---:|---|---|
| `DigitStoreTests` | 44 | `Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/Constructor_GivenDefault_DigitCountIsZero#0` | No Rust `DigitStore` type or digit-level API. |
| `DigitStoreSnapshotDigitsTests` | 13 | `Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreSnapshotDigitsTests/SnapshotDigits_GivenSingleDigit_ReturnsSingleElementArray#0` | No snapshot/digit array API. |
| `DigitStoreResetTests` | 7 | `Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreResetTests/Reset_GivenNonZeroStore_ByteCountBecomesZero#0` | No mutable reset. |
| `DigitStoreCopyDigitsFromTests` | 6 | `Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreCopyDigitsFromTests/CopyDigitsFrom_GivenFourDigitSource_ByteCountAndDigitsMatchSource#0` | No mutable copy-into API. |
| `DigitStoreTrimLeadingZerosTests` | 6 | `Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTrimLeadingZerosTests/TrimLeadingZeros_GivenNoLeadingZeros_LeavesValueUnchanged#0` | Trimming is internal to `Nat::normalized`; no public test hook. |
| `DigitStoreMutatorTests` | 5 | `Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreMutatorTests/SetDigitCount_GivenPositiveValue_DigitCountIsUpdated#0` | No mutable digit count. |
| `DigitStoreToStringParallelTests` | 5 | `Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreToStringParallelTests/ToString_Given100Digits_ReturnsCorrectDigitOrder#0` | No nibble-digit rendering or separator API. |
| `DigitStoreThreadSafetyTests` | 4 | `Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreThreadSafetyTests/TrimLeadingZeros_ConcurrentWithGetDigit_NeverThrowsAndDigitsInRange#0` | No shared mutable store to race. |
| `UnitTest1` | 1 | `Lovelace.Representation.Tests/Lovelace.Representation.Tests.UnitTest1/Test1#0` | Template placeholder with no assertion. |

### Lovelace.Real.Tests — sampled, 2,531 not attempted

Per the brief, Real was sampled rather than finished: 6 `RealToStringTests` cases were ported and the
remaining 2,531 cases were left deferred, not refused. The sample exposed these blockers:

| Census id | C# file:line | Reason |
|---|---|---|
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealConstructorTests/Constructor_GivenDefault_ProducesZeroWithExponentZero#0` | Lovelace.Real.Tests/RealConstructorTests.cs:20 | Asserts the public `Exponent` field; Rust `Real.exp` is private with no accessor. |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealConstructorTests/Constructor_GivenDouble_StoresCorrectDigitsAndExponent#0` | Lovelace.Real.Tests/RealConstructorTests.cs:32 | Rust `Real` has no `double` constructor (only `from_str`, `from_i64`, `from_int`, `from_fraction`). |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealConstructorTests/Constructor_GivenStringDecimal_ExponentIsNegativePlaceCount#0` | Lovelace.Real.Tests/RealConstructorTests.cs:65 | Asserts `Exponent == -2`; no public exponent accessor. |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealConstructorTests/Assign_GivenOtherReal_CopiesAllFields#0` | Lovelace.Real.Tests/RealConstructorTests.cs:131 | No `Assign`; Rust values are moved/cloned. |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealToStringTests/TryFormat_GivenSufficientBuffer_ReturnsTrueAndWritesCorrectly#0` | Lovelace.Real.Tests/RealToStringTests.cs:80 | No `TryFormat` buffer API. |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealToStringTests/ToString_GivenNonPeriodicValue_TruncatesAtDisplayDecimalPlaces#0` | Lovelace.Real.Tests/RealToStringTests.cs:138 | No ambient `DisplayDecimalPlaces`; Rust uses explicit `to_decimal_fixed(digits)`. |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealToStringTests/ToString_GivenRepeatingQuotient_EmitsPeriodicNotation#0..#4` | Lovelace.Real.Tests/RealToStringTests.cs:195 | Needs `Real.WithPrecision` ambient scope; Rust `div` takes an explicit `max_digits`. |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealIncrementTests/Increment_GivenPositiveInteger_ReturnsNextInteger#0` | Lovelace.Real.Tests/RealIncrementTests.cs:16 | No increment/decrement on Rust `Real`. |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealInvertTests/Invert_GivenFraction_ReturnsCorrectResult#0` | Lovelace.Real.Tests/RealInvertTests.cs:42 | No reciprocal API. |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.LReal64Tests/Parse_GivenValue_RoundTripsLikeReal#0..#20` | Lovelace.Real.Tests/LReal64Tests.cs:54 | `LReal64`/`LReal128` fixed-precision wrappers are not ported. |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealAddPeriodicExactTests/Add_ExactRationalPair_Property_DenotesTheExactRationalSum#0..#863` | Lovelace.Real.Tests/RealAddPeriodicExactTests.cs:91 | 864-row property test over generated rational pairs; no equivalent generator/oracle in Rust. |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigCertifiedPrefixTests/Sin_OfOne_PublishesTheDigitsItClaims#0` | Lovelace.Real.Tests/RealTrigCertifiedPrefixTests.cs:59 | Rust `Real` has no `sin`/`cos` API (`sqrt` and `pi` only). |

## Not verified / limitations

- The Real sample is not exhaustive; the 2,531 remaining Real cases were not attempted in this batch.
- Representation has no Rust counterpart at all; all 91 cases are refused rather than deferred.
- Mappings whose C# test asserts a specific exception type assert the corresponding Rust error value
  (`ArrayError` variant or `None`); the channel differs, and each note states this explicitly.
- `Lovelace.Suite.Tests`, `Lovelace.Symbolics.Tests`, `Lovelace.Run.Tests` and the other projects were
  outside this batch and keep their previous dispositions.
- The working tree contains uncommitted production edits from earlier sessions (visible in `git diff`
  for `lovelace-real`, `lovelace-lang`, `lovelace-json`, `lovelace-run`, `lovelace-symbolics`). They
  were present before this batch and are outside its diff; only the test modules listed above changed.
