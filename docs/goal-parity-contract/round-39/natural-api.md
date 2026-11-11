# Round 39 — the Natural/Real API residue: additive members and their tests

Status: production final for `rust/lovelace-nat` and `rust/lovelace-real`. This lane added 75
Rust tests (38 Natural, 37 Real) and the additive members they exercise. It did not edit any
`.cs` file, fixture, `parity/` artifact, or other documentation.

The 77 census rows named below are the rows those tests unblock. `parity/` was out of scope, so
they are **candidates for curation**, not manifest claims: once a mapping lane points the named
ids at the named tests, coverage moves 5,621/6,209 (90.53 %) to an expected **5,698/6,209
(91.77 %)**, deferred 470 -> 393. The gate numbers below are the measured source-tree gates; the
manifest gates still see the uncurated rows.

## 0. Population

| Measure | Rows |
|---|---:|
| Deferred census rows at round start | 470 |
| — Lovelace.Natural.Tests | 40 |
| — Lovelace.Real.Tests | 47 |
| (a) mapped by this round's tests | 77 |
| — Natural | 40 |
| — Real | 37 |
| (b) still refused | 10 (RealTrigFastPath only) |

The Natural 40 split exactly as round 36 recorded: 15 CLR-structure rows, 8 static-property rows,
17 residual rows. The Real 37 are every deferred Real row except the 10 private-fast-path rows of
`RealTrigFastPathTests`.

## 1. Natural — C# member -> Rust member

| C# member / assertion | Rust equivalent | Note |
|---|---|---|
| `typeof(Natural).Name == "Natural"` | `Nat`, alias `lovelace_nat::Natural` | Rust's canonical type name is `Nat`; the alias preserves the reference's spelling. |
| `IEquatable<Natural>` | `Eq` | |
| `IComparable<Natural>` | `Ord` | |
| `IParsable<Natural>` | `FromStr for Nat` (`ParseNatError`) | |
| `ISpanParsable<Natural>` | `FromStr` + `Nat::from_ascii(&[u8])` | |
| `ISpanFormattable` | `Display` + `Nat::write_decimal(&mut [u8])` | |
| `INumber<Natural>` | `lovelace_nat::Number` (`zero`, `one`, `is_zero`, `radix`) | No CLR interface inheritance in Rust; the trait is the additive identity surface. |
| `IAdditionOperators` | `Add<&Nat> for &Nat` | Reference-only impls: a by-value `impl Add for Nat` would be selected ahead of the existing inherent `Nat::add(&self, &Nat)` and silently move the receiver. |
| `ISubtractionOperators` | `Sub<&Nat> for &Nat` | Underflow panics, like the reference's refusal. |
| `IMultiplyOperators` | `Mul<&Nat> for &Nat` | |
| `IDivisionOperators` | `Div<&Nat> for &Nat` | Zero divisor panics, like the reference's exception. |
| `IModulusOperators` | `Rem<&Nat> for &Nat` | |
| `IIncrementOperators` (`++`) | `AddAssign<u64>` + `Nat::increment` | |
| `IDecrementOperators` (`--`) | `SubAssign<u64>` + `Nat::decrement` | `decrement` returns false on zero. |
| `IComparisonOperators` | `PartialOrd` (through `Ord`) | |
| `Natural.DisplayDigits` | `Nat::display_digits()` / `set_display_digits(i64)` | `AtomicI64`, default `-1`. Configuration only: Rust arithmetic has no ambient display setting. |
| `Natural.Precision` | `Nat::precision()` / `set_precision(i64)` | `AtomicI64`, default `-1`; configuration only. |
| `Natural(ulong)` | `From<u64>` / `from_u64` | |
| `Natural(int)`, negative throws | `Nat::from_i64(i64) -> Option<Nat>` / `TryFrom<i64>` | Negative -> `Err(NegativeNatural)` / `None`. |
| `Parse(null)` / `TryParse(null)` | `Nat::from_decimal_opt(Option<&str>)` | `None` input -> `None` result. |
| cached `ToString` identity | `Nat::as_decimal_str() -> &str`, backed by `OnceLock<Arc<str>>` | `Clone` shares the cached allocation, so `Assert.Same` has a Rust pointer-identity equivalent. |
| `TryGetExactInt64` | `try_get_exact_i64` / `to_i64` | |
| `DecimalDigitCount()` | `decimal_digit_count()` | |
| limb layout / width queries | `limb_count()`, `bit_length()`, `to_limbs()` | Base-2^64 little-endian limbs; zero has no limbs and bit length 0. |
| `ShiftLeftDecimal(k)` | `shift_left_decimal(k)` | Alias of `mul_pow10`. |
| `Gcd` / `Lcm` | `gcd` / `lcm` | |
| `TryFormat` | `write_decimal` | `None` when the destination is too small. |

## 2. Real — C# member -> Rust member

| C# member / assertion | Rust equivalent | Note |
|---|---|---|
| `Real(double)` | `Real::from_f64(f64) -> Option<Real>` | Parses Rust's shortest round-trip decimal; NaN/inf refuse. |
| `Real(decimal)` | `Real::from_decimal_scaled(mantissa: i128, scale: u32)` | The reference's `G29` text is the mantissa with the point inserted; trailing zeros normalise. |
| `TryParse(null)` | `Real::from_str_opt(Option<&str>)` | |
| `Real.Divide` at the ambient cap | `div_default` | Existing `div(&Real, digits)` keeps the explicit argument. |
| `Real.PiTo(long)` | `Real::pi_checked(i64) -> Option<Real>` | Positive and <= active `MaxComputationDecimalPlaces`; existing `pi(digits)` unchanged. |
| `Real.PiToAsync(long)` | worker thread around `pi_checked` | No runtime dependency; the refusal propagates through `join`. |
| `Real.PiSegment(a,b)` | `round39_api::pi_segment(i64,i64) -> Option<(Nat,Nat,Int)>` | Same Chudnovsky BSP recurrence and merge identity. |
| `Real.Pow(Real)` | `pow_real_checked(&Real) -> Result<Real, RealPowError>` | Refusals carry the reference's `message`/`param_name`; see the zero-base rows. |
| `Real.Sqrt(Real)` | `sqrt_checked() -> Result<Real, NegativeSqrt>` | Existing `sqrt(digits)` still returns zero for a negative input (unchanged); the checked entry point is the additive refusal. |
| `Real.SqrtAsync(Real)` | worker thread around `sqrt_checked` | |
| `Real.Sqrt(IReadOnlyList<Real>)` | `sqrt_batch(&[Real]) -> Result<Vec<Real>, NegativeSqrt>` | Empty -> empty; one negative refuses the batch. |
| `Real.DisplayDecimalPlaces` | `display_decimal_places()` / `set_display_decimal_places(i64)` | `AtomicI64`, default 100; thread-local scope wins. |
| `Real.MaxComputationDecimalPlaces` | `max_computation_decimal_places()` / `set_max_computation_decimal_places(i64)` | `AtomicI64`, default 1000; thread-local scope wins. |
| `Real.WithPrecision` / `WithLocalPrecision` | `with_precision(computation, display) -> PrecisionScope` | RAII drop restores the previous thread-local pair; nesting is LIFO. |
| `AsyncLocal` flow into a child task | `precision_snapshot()` + `with_precision_values` | Rust has no ExecutionContext capture; the caller passes the pair explicitly. |
| `Exponent { get; set; }` | `exponent()` / `set_exponent(i64)` | Setter is representation-level, no renormalisation. |
| `PeriodStart` (default 0) | `period_start_or_zero()` | Existing `period_start()` keeps the `-1` no-period sentinel; the compatibility accessor reports the reference's 0. |
| `PeriodLength`, `IsPeriodic` | `period_length()` (existing), `is_periodic()` | |
| `ToNatural()` | `to_natural() -> Nat` | Magnitude, sign dropped. |
| `IsNegative` / `IsPositive` | `is_negative()` / `is_positive()` | |
| `ToString()` at `DisplayDecimalPlaces` | `to_string_with_display(digits)` | Periodic values render in full; terminating values publish exactly the display window; integer/no-fraction values keep their canonical form. |
| `TryFormat` | `write_display(&mut [u8]) -> Option<usize>` | `None` and no write when the buffer is too small. |

## 3. Census ids each test unblocks

The tables abbreviate the census prefix: every Natural id is
`Lovelace.Natural.Tests/Lovelace.Natural.Tests.<Class>/<Method>#<case>` and every Real id is
`Lovelace.Real.Tests/Lovelace.Real.Tests.<Class>/<Method>#<case>`.

### 3a. Natural (40)

All tests are in `rust/lovelace-nat/src/round39_natural_api.rs`.

| Census ids | Rust test |
|---|---|
| `NaturalStructureTests/Natural_TypeName_IsNatural#0` | `natural_type_name_is_natural` |
| `NaturalStructureTests/Natural_ImplementsIEquatable#0` | `natural_implements_iequatable` |
| `NaturalStructureTests/Natural_ImplementsIComparable#0` | `natural_implements_icomparable` |
| `NaturalStructureTests/Natural_ImplementsIParsable#0` | `natural_implements_iparsable` |
| `NaturalStructureTests/Natural_ImplementsISpanParsable#0` | `natural_implements_ispanparsable` |
| `NaturalStructureTests/Natural_ImplementsISpanFormattable#0` | `natural_implements_ispanformattable` |
| `NaturalStructureTests/Natural_ImplementsINumber#0` | `natural_implements_inumber` |
| `NaturalStructureTests/Natural_ImplementsIAdditionOperators#0` | `natural_implements_iadditionoperators` |
| `NaturalStructureTests/Natural_ImplementsISubtractionOperators#0` | `natural_implements_isubtractionoperators` |
| `NaturalStructureTests/Natural_ImplementsIMultiplyOperators#0` | `natural_implements_imultiplyoperators` |
| `NaturalStructureTests/Natural_ImplementsIDivisionOperators#0` | `natural_implements_idivisionoperators` |
| `NaturalStructureTests/Natural_ImplementsIModulusOperators#0` | `natural_implements_imodulusoperators` |
| `NaturalStructureTests/Natural_ImplementsIIncrementOperators#0` | `natural_implements_iincrementoperators` |
| `NaturalStructureTests/Natural_ImplementsIDecrementOperators#0` | `natural_implements_idecrementoperators` |
| `NaturalStructureTests/Natural_ImplementsIComparisonOperators#0` | `natural_implements_icomparisonoperators` |
| `NaturalStaticPropertyTests/DisplayDigits_DefaultValue_IsMinusOne#0` | `display_digits_default_value_is_minus_one` |
| `NaturalStaticPropertyTests/DisplayDigits_AfterSet_ReturnsSetValue#0` | `display_digits_after_set_returns_set_value` |
| `NaturalStaticPropertyTests/DisplayDigits_AfterMultipleSets_ReturnsLastValue#0` | `display_digits_after_multiple_sets_returns_last_value` |
| `NaturalStaticPropertyTests/DisplayDigits_ConcurrentReadWrite_NeverReturnsTornValue#0` | `display_digits_concurrent_read_write_never_returns_torn_value` |
| `NaturalStaticPropertyTests/Precision_DefaultValue_IsMinusOne#0` | `precision_default_value_is_minus_one` |
| `NaturalStaticPropertyTests/Precision_AfterSet_ReturnsSetValue#0` | `precision_after_set_returns_set_value` |
| `NaturalStaticPropertyTests/Precision_AfterMultipleSets_ReturnsLastValue#0` | `precision_after_multiple_sets_returns_last_value` |
| `NaturalStaticPropertyTests/Precision_ConcurrentReadWrite_NeverReturnsTornValue#0` | `precision_concurrent_read_write_never_returns_torn_value` |
| `NaturalBinaryLimbTests/DivRem_GivenLargeOperands_ProducesCorrectQuotientAndRemainder#0` | `div_rem_given_large_operands_produces_correct_quotient_and_remainder` |
| `NaturalBinaryLimbTests/RoundTrip_GivenRandomLargeValues_ParseToStringIdentity#0` | `round_trip_given_random_large_values_parse_to_string_identity` |
| `NaturalConstructorTests/Constructor_GivenNegativeInt_ThrowsArgumentOutOfRangeException#0` | `constructor_given_negative_int_throws_argument_out_of_range_exception` |
| `NaturalDecimalCacheTests/ArithmeticResult_ToString_CachesAfterFirstCall#0` | `arithmetic_result_to_string_caches_after_first_call` |
| `NaturalDecimalCacheTests/ConcurrentParseToString_GivenManyValues_MatchesBigInteger#0` | `concurrent_parse_to_string_given_many_values_matches_expected` |
| `NaturalDecimalCacheTests/ConcurrentToString_GivenSharedValue_ReturnsConsistentString#0` | `concurrent_to_string_given_shared_value_returns_consistent_string` |
| `NaturalDecimalCacheTests/CopyConstructor_SharesCachedStringButIndependentLimbs#0` | `copy_constructor_shares_cached_string_but_independent_limbs` |
| `NaturalDecimalCacheTests/RoundTrip_GivenRandomLargeValues_ParseToStringIdentity#0` | `round_trip_given_random_large_values_parse_to_string_identity_decimal_cache` |
| `NaturalDecimalCacheTests/ToString_GivenParsedValue_IsStableAcrossRepeatedCalls#0` | `to_string_given_parsed_value_is_stable_across_repeated_calls` |
| `NaturalParseTests/Parse_GivenNullString_ThrowsFormatException#0` | `parse_given_null_string_throws_format_exception` |
| `NaturalParseTests/TryParse_GivenNullString_ReturnsFalse#0` | `try_parse_given_null_string_returns_false` |
| `NaturalRandomizedCrossCheckTests/CrossCheck_GivenRandomOperands_MatchesBigInteger#0..#4` (5 rows) | `cross_check_given_random_operands_matches_reference` |
| `NaturalRandomizedCrossCheckTests/DivRem_GivenPowerOfTenDivisor_MatchesBigInteger#0` | `div_rem_given_power_of_ten_divisor_matches_reference` |

### 3b. Real (37)

All tests are in `rust/lovelace-real/src/round39_real_tests.rs`.

| Census ids | Rust test |
|---|---|
| `RealAsyncLocalTests/Pi_GivenConcurrentTasksWithDifferentLocalPrecisions_PrecisionDoesNotLeakAcrossTasks#0` | `pi_given_concurrent_tasks_with_different_local_precisions_precision_does_not_leak_across_tasks` |
| `RealAsyncLocalTests/Pi_GivenNestedPrecisionScopes_OuterScopeRestoredAfterInnerDisposes#0` | `pi_given_nested_precision_scopes_outer_scope_restored_after_inner_disposes` |
| `RealAsyncLocalTests/Pi_StaticDisplayDecimalPlaces_ConcurrentReadsMutationsAreAtomic#0` | `pi_static_display_decimal_places_concurrent_reads_mutations_are_atomic` |
| `RealAsyncLocalTests/Sqrt_GivenBatchWithCallerLocalPrecision_EachChildTaskInheritsCallerPrecision#0` | `sqrt_given_batch_with_caller_local_precision_each_child_task_inherits_caller_precision` |
| `RealConstructorTests/Constructor_GivenDecimal_RoundTripEqualsStringConstructor#0` | `constructor_given_decimal_round_trip_equals_string_constructor` |
| `RealConstructorTests/Constructor_GivenDouble_StoresCorrectDigitsAndExponent#0` | `constructor_given_double_stores_correct_digits_and_exponent` |
| `RealConstructorTests/Constructor_GivenDoubleZero_ProducesZero#0` | `constructor_given_double_zero_produces_zero` |
| `RealConstructorTests/Constructor_GivenNegativeDouble_IsNegative#0` | `constructor_given_negative_double_is_negative` |
| `RealConstructorTests/Constructor_GivenPositiveDecimal_ParsesCorrectly#0` | `constructor_given_positive_decimal_parses_correctly` |
| `RealDivideLeadingZeroRunTests/Divide_GivenLeadingZeroRunLongerThanTheBudget_KeepsTheValueAndItsScale#2` | `divide_given_leading_zero_run_longer_than_the_budget_keeps_the_value_and_its_scale` |
| `RealDivideLeadingZeroRunTests/Divide_GivenLeadingZeroRunLongerThanTheBudget_KeepsTheValueAndItsScale#3` | same test |
| `RealDivideLeadingZeroRunTests/Divide_GivenQuotientTheBudgetCannotHold_IsInexactAndKeepsItsScale#0` | `divide_given_quotient_the_budget_cannot_hold_is_inexact_and_keeps_its_scale` |
| `RealParseTests/TryParse_GivenNullString_ReturnsFalse#0` | `try_parse_given_null_string_returns_false` |
| `RealPiSegmentTests/PiSegment_GivenFullRangeTenDigits_ProducesPiMatchingCurrentSerial#0` | `pi_segment_given_full_range_ten_digits_produces_pi_matching_current_serial` |
| `RealPiSegmentTests/PiSegment_GivenMidpointSplit_MergedTMatchesFullRangeT#0` | `pi_segment_given_midpoint_split_merged_t_matches_full_range_t` |
| `RealPiSegmentTests/PiSegment_GivenSingleTermRange_MatchesManualTermOneComputation#0` | `pi_segment_given_single_term_range_matches_manual_term_one_computation` |
| `RealPiTests/Pi_GivenDigitsExceedingMaxComputationDecimalPlaces_ThrowsArgumentOutOfRangeException#0` | `pi_given_digits_exceeding_max_computation_decimal_places_throws_argument_out_of_range_exception` |
| `RealPiToAsyncTests/PiToAsync_GivenInvalidDigits_PropagatesArgumentOutOfRangeException#0` | `pi_to_async_given_invalid_digits_propagates_argument_out_of_range_exception` |
| `RealPowZeroBaseTests/Pow_NonZeroBaseNegativeExponent_KeepsItsExistingRefusal#0` | `pow_nonzero_base_negative_exponent_keeps_its_existing_refusal` |
| `RealPowZeroBaseTests/Pow_ZeroBaseNegativeRealExponent_RefusesWithTheIntegerPathsRefusal#0` | `pow_zero_base_negative_real_exponent_refuses_with_the_integer_paths_refusal` |
| `RealPowZeroBaseTests/Pow_ZeroBasePositiveFractionalExponent_IsStillZero#0` | `pow_zero_base_positive_exponent_is_still_zero` |
| `RealPropertiesTests/DisplayDecimalPlaces_AfterSetting_ReturnsNewValue#0` | `display_decimal_places_after_setting_returns_new_value` |
| `RealPropertiesTests/DisplayDecimalPlaces_DefaultValue_IsOneHundred#0` | `display_decimal_places_default_value_is_one_hundred` |
| `RealPropertiesTests/Exponent_AfterSetting_ReturnsNewValue#0` | `exponent_after_setting_returns_new_value` |
| `RealPropertiesTests/MaxComputationDecimalPlaces_AfterSetting_ReturnsNewValue#0` | `max_computation_decimal_places_after_setting_returns_new_value` |
| `RealPropertiesTests/MaxComputationDecimalPlaces_DefaultValue_IsOneThousand#0` | `max_computation_decimal_places_default_value_is_one_thousand` |
| `RealPropertiesTests/PeriodStart_GivenDefaultReal_IsZero#0` | `period_start_given_default_real_is_zero` |
| `RealSqrtAsyncTests/SqrtAsync_GivenNegativeInput_PropagatesArithmeticException#0` | `sqrt_async_given_negative_input_propagates_arithmetic_exception` |
| `RealSqrtBatchTests/Sqrt_GivenBatchContainingNegativeValue_PropagatesArithmeticException#0` | `sqrt_given_batch_containing_negative_value_propagates_arithmetic_exception` |
| `RealSqrtBatchTests/Sqrt_GivenBatchMixedIrrationals_MatchesElementwiseSerial#0` | `sqrt_given_batch_mixed_irrationals_matches_elementwise_serial` |
| `RealSqrtBatchTests/Sqrt_GivenBatchPerfectSquares_ReturnsExactRoots#0` | `sqrt_given_batch_perfect_squares_returns_exact_roots` |
| `RealSqrtBatchTests/Sqrt_GivenBatchSingleElement_ReturnsArrayOfOneMatchingSerial#0` | `sqrt_given_batch_single_element_returns_array_of_one_matching_serial` |
| `RealSqrtBatchTests/Sqrt_GivenEmptyBatch_ReturnsEmptyArray#0` | `sqrt_given_empty_batch_returns_empty_array` |
| `RealSqrtTests/Sqrt_GivenNegativeInput_ThrowsArithmeticException#0` | `sqrt_given_negative_input_throws_arithmetic_exception` |
| `RealToStringTests/ToString_GivenNonPeriodicValue_TruncatesAtDisplayDecimalPlaces#0` | `to_string_given_non_periodic_value_truncates_at_display_decimal_places` |
| `RealToStringTests/TryFormat_GivenInsufficientBuffer_ReturnsFalse#0` | `try_format_given_insufficient_buffer_returns_false` |
| `RealToStringTests/TryFormat_GivenSufficientBuffer_ReturnsTrueAndWritesCorrectly#0` | `try_format_given_sufficient_buffer_returns_true_and_writes_correctly` |

## 4. Translation notes (representation differences)

* **Natural is base-2^64, not decimal.** The new structural members (`limb_count`,
  `bit_length`, `to_limbs`) expose that layout; the decimal cache is an `Arc<str>` so a clone
  shares the rendering exactly as the reference's copy constructor shares its immutable string.
* **CLR interface identity does not exist in Rust.** Each structure row is asserted as the
  compile-time trait bound that carries the same capability. `INumber` is represented by the
  crate's reduced `Number` trait (identities + radix); there is no generic numeric tower.
* **Natural's statics are configuration-only.** The reference's arithmetic has no ambient
  Natural precision either; the two atomic values exist so a host can read/write the same API.
* **Real's precision is explicit on the Rust side.** Existing `sqrt(digits)`, `pi(digits)`,
  `div(&d, digits)` and `pow_int(exp)` are unchanged. The new ambient statics are consulted
  only by the new `*_checked`/`*_default`/`*_with_display` helpers.
* **AsyncLocal has no Rust analogue.** `with_precision` is thread-local; a child thread does
  not inherit it, so the inheritance row uses the additive `precision_snapshot` /
  `with_precision_values` explicit capture and the report names that difference.
* **PeriodStart sentinel.** Rust's existing `period_start()` reports `-1` for "no period";
  the reference property reports `0`. `period_start_or_zero()` is the compatibility accessor;
  the existing accessor is untouched.

## 5. Still refused (10 rows)

All ten are `RealTrigFastPathTests`: `ArgumentReduction_IntegerQuotient_MatchesTheDividedForm`,
`Cos_AtDefaultPrecision_StaysInteractive`, `HalfOf_MatchesTheDividedForm_ForPiShapedValues`,
`NonSpecialAngles_AreNotCapturedByTheTable`,
`Normalize_BinaryStrip_MatchesTheDigitStringStrip` (and `...ForLongZeroRuns`),
`SeriesTermination_IntegerTest_MatchesTheThresholdComparison` (and `...TracksTheCarriedPower`),
`SpecialAngles_BuiltTheOriginalWay_StillHitTheTableExactly`,
`ToInteger_BinaryShift_MatchesTheDigitPadding`.

Measurement: the C# rows invoke private static/instance members by reflection
(`ReduceToTwoPi`, `WholeQuotient`, `Truncate`, `HalfOf`, `TrySpecialAngle`,
`SpecialAngleCandidate`, `Normalize`, `IsBelowDecimalGuard`, `CosTaylor`, `ToInteger`).
Round 26 refused them because the Rust kernel has no matching private surface; that measurement
still holds. Adding public test hooks that reproduce the private implementation would make the
row a comparison of the helper with itself, not evidence about a public API. The timing row is a
wall-clock contract and is excluded from a portable gate. These rows stay deferred.

## 6. Verification

Environment for every command:
`RUSTUP_HOME=<repo>/.toolchain/rustup`, `CARGO_HOME=<repo>/.toolchain/cargo`,
`TMP=TEMP=<repo>/rust/target/tmp`.

| Gate | Command | Observed |
|---|---|---|
| Natural + Real | `cargo test -p lovelace-nat -p lovelace-real --no-fail-fast` | nat 264 passed, 0 failed; real 2,475 passed, 0 failed; exit 0 |
| Workspace (no symbolics) | `cargo test --workspace --exclude lovelace-symbolics --no-fail-fast` | 4,692 passed, 0 failed; includes lang 1,345, nat 264, real 2,475; exit 0 |
| Corpus | `rust/target/release/lovelace-corpus.exe` | `corpus: 214 passed, 0 failed, 214 total` |
| Fixtures | `node parity/tools/gate-fixtures.mjs` | `PASS: oracle 20/20, rust 20/20` |
| CLI matrix | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16`; PASS |
| Full workspace | `cargo test --workspace --no-fail-fast` | blocked by the concurrent symbolics lane; see the note below |

Full-workspace note: this lane could not obtain a clean full-workspace run because the
concurrent symbolics lane was editing and testing that crate at the same time. On the shared
`rust/target`, its running `cargo test -p lovelace-symbolics round39_residue` held the
symbolics test executable, so our run failed with `LNK1104: cannot open
...lovelace_symbolics-*.exe` before any test ran. A second run in an isolated
`CARGO_TARGET_DIR=rust/target-verify` compiled the same source tree; every crate before
symbolics was green except the pre-existing wall-clock flake
`assumptions_round35::eight_hundred_atoms_stay_ordered_and_fast` (lang 1,344 passed, 1 failed;
the round-38 document already records that tripwire flaking under full parallel load), with nat
264/0 and real 2,475/0. That isolated run then reached the concurrent lane's new
`lovelace-symbolics::round39_residue` module, where
`diff_abs_carries_nonzero_condition`, `yun_square_free_multiplicities_are_correct` and
`rootof_close_roots_all_found_and_indexed_stably` failed and
`rootof_evaluates_the_quadratic_radicals` hung for over 60 seconds; the run was stopped. None
of those tests or sources is part of this lane, and no symbolics file was touched here. The
shared-target full-workspace gate must be rerun by the lane that owns that edit.

The new tests themselves are 75: 38 in `lovelace-nat` (`round39_natural_api`) and 37 in
`lovelace-real` (`round39_real_tests`), all passing in the runs above. The suite therefore
grows by 75 over the 4,720-test baseline cited for this round (the final total depends on the
concurrent lanes' symbolics and lang edits).

## 7. Files

| File | Change |
|---|---|
| `rust/lovelace-nat/src/lib.rs` | additive alias/trait, statics, conversions, structure queries, operator and parse impls, `Arc<str>` decimal cache sharing |
| `rust/lovelace-nat/src/round39_natural_api.rs` | 38 tests (new) |
| `rust/lovelace-real/src/lib.rs` | declares the two new modules |
| `rust/lovelace-real/src/round39_api.rs` | additive Real members and `pi_segment` (new) |
| `rust/lovelace-real/src/round39_real_tests.rs` | 37 tests (new) |
| `docs/goal-parity-contract/round-39/natural-api.md` | this report |
