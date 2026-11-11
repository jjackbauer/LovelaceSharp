# Round 40 — curating the round-39 unblocks into the parity manifest

Status: **complete**. The 99 deferred census rows the two round-39 production lanes unblocked are
now mapped to the Rust tests that landed with them in `05f5426`. This round wrote **zero new Rust
tests**: every listed id already had a covering test in the landed tree, and every one of those
tests is in the measured inventory and ran green. No production file, `.cs` file, fixture, or
contract document was edited. Only `parity/manifest-map.json` (99 rows) and the generated parity
artifacts changed.

## 0. Population

| Measure | Rows |
|---|---:|
| Deferred census rows at round start | 470 |
| Natural lane candidates (`docs/goal-parity-contract/round-39/natural-api.md`) | 40 |
| Real lane candidates (of 47 deferred; 10 `RealTrigFastPathTests` refused) | 37 |
| Symbolics lane candidates (`docs/goal-parity-contract/round-39/symbolics-residue.md`, one row already ported) | 22 |
| **Mapped by this round** | **99** |
| New Rust tests written by this round | 0 |
| Refusals among the 99 | 0 |
| Still refused (unchanged, already curated deferred) | 10 |

Eighteen rows sit in six shared-test groups (five `CrossCheck_GivenRandomOperands_MatchesBigInteger#0..#4`
rows behind one theory test, the two divide-leading-zero rows `#2`/`#3`, five indeterminate-exponential
rows, the two determined-limit rows, the two integration rows, and the two cubic-root rows), so 99
rows resolve to **87** test functions (Natural 36, Real 36, Symbolics+lang 15).

## 1. Result

Coverage moves **5,621/6,209 (90.53 %) -> 5,720/6,209 (92.12 %)**; deferred **470 -> 371**;
exclusions unchanged at **118**. Per project:

| Project | Cases | Ported | Deferred | Excluded | Coverage |
|---|---:|---:|---:|---:|---:|
| Lovelace.Natural.Tests | 195 | 195 | 0 | 0 | 100.00 % |
| Lovelace.Real.Tests | 2,537 | 2,527 | 10 | 0 | 99.61 % |
| Lovelace.Symbolics.Tests | 1,226 | 1,033 | 193 | 0 | 84.26 % |

## 2. Mappings

The tables abbreviate the census prefix: a Natural id is
`Lovelace.Natural.Tests/Lovelace.Natural.Tests.<Class>/<Method>#<case>`, a Real id is
`Lovelace.Real.Tests/...`, a Symbolics id is `Lovelace.Symbolics.Tests/...`.

### 2a. Natural (40: 15 structure, 8 statics, 17 residual)

| Census row (prefix omitted) | Rust test | Assertion |
|---|---|---|
| `NaturalBinaryLimbTests/DivRem_GivenLargeOperands_ProducesCorrectQuotientAndRemainder#0` | `div_rem_given_large_operands_produces_correct_quotient_and_remainder` | 2000-digit / 1200-digit DivRem equals BigInteger's quotient and remainder |
| `NaturalBinaryLimbTests/RoundTrip_GivenRandomLargeValues_ParseToStringIdentity#0` | `round_trip_given_random_large_values_parse_to_string_identity` | 30 random values up to 20,000 digits parse/format identically |
| `NaturalConstructorTests/Constructor_GivenNegativeInt_ThrowsArgumentOutOfRangeException#0` | `constructor_given_negative_int_throws_argument_out_of_range_exception` | Natural(-1) is refused |
| `NaturalDecimalCacheTests/ArithmeticResult_ToString_CachesAfterFirstCall#0` | `arithmetic_result_to_string_caches_after_first_call` | an arithmetic result's ToString is computed once, cached, and correct |
| `NaturalDecimalCacheTests/ConcurrentParseToString_GivenManyValues_MatchesBigInteger#0` | `concurrent_parse_to_string_given_many_values_matches_expected` | 200 values parsed and formatted in parallel equal their canonical strings |
| `NaturalDecimalCacheTests/ConcurrentToString_GivenSharedValue_ReturnsConsistentString#0` | `concurrent_to_string_given_shared_value_returns_consistent_string` | parallel ToString calls on one value return the shared cached string |
| `NaturalDecimalCacheTests/CopyConstructor_SharesCachedStringButIndependentLimbs#0` | `copy_constructor_shares_cached_string_but_independent_limbs` | a copy shares the cached string and is independent of the original |
| `NaturalDecimalCacheTests/RoundTrip_GivenRandomLargeValues_ParseToStringIdentity#0` | `round_trip_given_random_large_values_parse_to_string_identity_decimal_cache` | 20 random values up to 20,000 digits round-trip through the cache |
| `NaturalDecimalCacheTests/ToString_GivenParsedValue_IsStableAcrossRepeatedCalls#0` | `to_string_given_parsed_value_is_stable_across_repeated_calls` | repeated ToString calls on a parsed value return the same string |
| `NaturalParseTests/Parse_GivenNullString_ThrowsFormatException#0` | `parse_given_null_string_throws_format_exception` | a missing string input is refused |
| `NaturalParseTests/TryParse_GivenNullString_ReturnsFalse#0` | `try_parse_given_null_string_returns_false` | TryParse of a missing string returns false |
| `NaturalRandomizedCrossCheckTests/CrossCheck_GivenRandomOperands_MatchesBigInteger#0` | `cross_check_given_random_operands_matches_reference` | random +, -, *, DivRem and Pow across five size tiers |
| `NaturalRandomizedCrossCheckTests/CrossCheck_GivenRandomOperands_MatchesBigInteger#1` | `cross_check_given_random_operands_matches_reference` | random +, -, *, DivRem and Pow across five size tiers |
| `NaturalRandomizedCrossCheckTests/CrossCheck_GivenRandomOperands_MatchesBigInteger#2` | `cross_check_given_random_operands_matches_reference` | random +, -, *, DivRem and Pow across five size tiers |
| `NaturalRandomizedCrossCheckTests/CrossCheck_GivenRandomOperands_MatchesBigInteger#3` | `cross_check_given_random_operands_matches_reference` | random +, -, *, DivRem and Pow across five size tiers |
| `NaturalRandomizedCrossCheckTests/CrossCheck_GivenRandomOperands_MatchesBigInteger#4` | `cross_check_given_random_operands_matches_reference` | random +, -, *, DivRem and Pow across five size tiers |
| `NaturalRandomizedCrossCheckTests/DivRem_GivenPowerOfTenDivisor_MatchesBigInteger#0` | `div_rem_given_power_of_ten_divisor_matches_reference` | DivRem by 10^k reads quotient/remainder off the decimal prefix/suffix |
| `NaturalStaticPropertyTests/DisplayDigits_AfterMultipleSets_ReturnsLastValue#0` | `display_digits_after_multiple_sets_returns_last_value` | DisplayDigits returns the last of several writes |
| `NaturalStaticPropertyTests/DisplayDigits_AfterSet_ReturnsSetValue#0` | `display_digits_after_set_returns_set_value` | DisplayDigits returns the value set |
| `NaturalStaticPropertyTests/DisplayDigits_ConcurrentReadWrite_NeverReturnsTornValue#0` | `display_digits_concurrent_read_write_never_returns_torn_value` | concurrent DisplayDigits reads never observe a torn 64-bit value |
| `NaturalStaticPropertyTests/DisplayDigits_DefaultValue_IsMinusOne#0` | `display_digits_default_value_is_minus_one` | DisplayDigits defaults to -1 |
| `NaturalStaticPropertyTests/Precision_AfterMultipleSets_ReturnsLastValue#0` | `precision_after_multiple_sets_returns_last_value` | Precision returns the last of several writes |
| `NaturalStaticPropertyTests/Precision_AfterSet_ReturnsSetValue#0` | `precision_after_set_returns_set_value` | Precision returns the value set |
| `NaturalStaticPropertyTests/Precision_ConcurrentReadWrite_NeverReturnsTornValue#0` | `precision_concurrent_read_write_never_returns_torn_value` | concurrent Precision reads never observe a torn 64-bit value |
| `NaturalStaticPropertyTests/Precision_DefaultValue_IsMinusOne#0` | `precision_default_value_is_minus_one` | Precision defaults to -1 |
| `NaturalStructureTests/Natural_ImplementsIAdditionOperators#0` | `natural_implements_iadditionoperators` | Natural implements IAdditionOperators |
| `NaturalStructureTests/Natural_ImplementsIComparable#0` | `natural_implements_icomparable` | Natural implements IComparable<Natural> |
| `NaturalStructureTests/Natural_ImplementsIComparisonOperators#0` | `natural_implements_icomparisonoperators` | Natural implements IComparisonOperators |
| `NaturalStructureTests/Natural_ImplementsIDecrementOperators#0` | `natural_implements_idecrementoperators` | Natural implements IDecrementOperators |
| `NaturalStructureTests/Natural_ImplementsIDivisionOperators#0` | `natural_implements_idivisionoperators` | Natural implements IDivisionOperators |
| `NaturalStructureTests/Natural_ImplementsIEquatable#0` | `natural_implements_iequatable` | Natural implements IEquatable<Natural> |
| `NaturalStructureTests/Natural_ImplementsIIncrementOperators#0` | `natural_implements_iincrementoperators` | Natural implements IIncrementOperators |
| `NaturalStructureTests/Natural_ImplementsIModulusOperators#0` | `natural_implements_imodulusoperators` | Natural implements IModulusOperators |
| `NaturalStructureTests/Natural_ImplementsIMultiplyOperators#0` | `natural_implements_imultiplyoperators` | Natural implements IMultiplyOperators |
| `NaturalStructureTests/Natural_ImplementsINumber#0` | `natural_implements_inumber` | Natural implements INumber<Natural> |
| `NaturalStructureTests/Natural_ImplementsIParsable#0` | `natural_implements_iparsable` | Natural implements IParsable<Natural> |
| `NaturalStructureTests/Natural_ImplementsISpanFormattable#0` | `natural_implements_ispanformattable` | Natural implements ISpanFormattable |
| `NaturalStructureTests/Natural_ImplementsISpanParsable#0` | `natural_implements_ispanparsable` | Natural implements ISpanParsable<Natural> |
| `NaturalStructureTests/Natural_ImplementsISubtractionOperators#0` | `natural_implements_isubtractionoperators` | Natural implements ISubtractionOperators |
| `NaturalStructureTests/Natural_TypeName_IsNatural#0` | `natural_type_name_is_natural` | the public type is named Natural (Rust canonical name Nat, alias Natural) |

### 2b. Real (37)

| Census row (prefix omitted) | Rust test | Assertion |
|---|---|---|
| `RealAsyncLocalTests/Pi_GivenConcurrentTasksWithDifferentLocalPrecisions_PrecisionDoesNotLeakAcrossTasks#0` | `pi_given_concurrent_tasks_with_different_local_precisions_precision_does_not_leak_across_tasks` | sibling tasks with different local precisions each observe only their own value |
| `RealAsyncLocalTests/Pi_GivenNestedPrecisionScopes_OuterScopeRestoredAfterInnerDisposes#0` | `pi_given_nested_precision_scopes_outer_scope_restored_after_inner_disposes` | nested precision scopes restore LIFO back to the outer value and then the global default |
| `RealAsyncLocalTests/Pi_StaticDisplayDecimalPlaces_ConcurrentReadsMutationsAreAtomic#0` | `pi_static_display_decimal_places_concurrent_reads_mutations_are_atomic` | 60 concurrent reads/writes of DisplayDecimalPlaces observe only written values |
| `RealAsyncLocalTests/Sqrt_GivenBatchWithCallerLocalPrecision_EachChildTaskInheritsCallerPrecision#0` | `sqrt_given_batch_with_caller_local_precision_each_child_task_inherits_caller_precision` | child tasks spawned inside a scoped precision observe the caller's precision |
| `RealConstructorTests/Constructor_GivenDecimal_RoundTripEqualsStringConstructor#0` | `constructor_given_decimal_round_trip_equals_string_constructor` | Real(1.2345678901234567890m) equals the string constructor |
| `RealConstructorTests/Constructor_GivenDouble_StoresCorrectDigitsAndExponent#0` | `constructor_given_double_stores_correct_digits_and_exponent` | Real(3.14) equals new Real("3.14") |
| `RealConstructorTests/Constructor_GivenDoubleZero_ProducesZero#0` | `constructor_given_double_zero_produces_zero` | Real(0.0) is zero |
| `RealConstructorTests/Constructor_GivenNegativeDouble_IsNegative#0` | `constructor_given_negative_double_is_negative` | Real(-1.5) is negative |
| `RealConstructorTests/Constructor_GivenPositiveDecimal_ParsesCorrectly#0` | `constructor_given_positive_decimal_parses_correctly` | Real(1.5m) equals new Real("1.5") |
| `RealDivideLeadingZeroRunTests/Divide_GivenLeadingZeroRunLongerThanTheBudget_KeepsTheValueAndItsScale#2` | `divide_given_leading_zero_run_longer_than_the_budget_keeps_the_value_and_its_scale` | 1/(3*10^1000) keeps its digits, exponent, period metadata, exactness and rendering |
| `RealDivideLeadingZeroRunTests/Divide_GivenLeadingZeroRunLongerThanTheBudget_KeepsTheValueAndItsScale#3` | `divide_given_leading_zero_run_longer_than_the_budget_keeps_the_value_and_its_scale` | 1/(7*10^5000) keeps its digits, exponent, period metadata, exactness and rendering |
| `RealDivideLeadingZeroRunTests/Divide_GivenQuotientTheBudgetCannotHold_IsInexactAndKeepsItsScale#0` | `divide_given_quotient_the_budget_cannot_hold_is_inexact_and_keeps_its_scale` | 1/(1009*10^1000) at a 20-digit budget is inexact, non-periodic, and keeps its scale |
| `RealParseTests/TryParse_GivenNullString_ReturnsFalse#0` | `try_parse_given_null_string_returns_false` | TryParse(null) returns false |
| `RealPiSegmentTests/PiSegment_GivenFullRangeTenDigits_ProducesPiMatchingCurrentSerial#0` | `pi_segment_given_full_range_ten_digits_produces_pi_matching_current_serial` | PiSegment(0,5) equals the serial Chudnovsky Q and T |
| `RealPiSegmentTests/PiSegment_GivenMidpointSplit_MergedTMatchesFullRangeT#0` | `pi_segment_given_midpoint_split_merged_t_matches_full_range_t` | the BSP merge identity T(a,b)=T(a,m)Q(m,b)+P(a,m)T(m,b) holds |
| `RealPiSegmentTests/PiSegment_GivenSingleTermRange_MatchesManualTermOneComputation#0` | `pi_segment_given_single_term_range_matches_manual_term_one_computation` | PiSegment(1,2) returns the manual P/Q/T term values |
| `RealPiTests/Pi_GivenDigitsExceedingMaxComputationDecimalPlaces_ThrowsArgumentOutOfRangeException#0` | `pi_given_digits_exceeding_max_computation_decimal_places_throws_argument_out_of_range_exception` | PiTo(11) under a 10-place cap is refused; 0 and -1 are refused; 10 is accepted |
| `RealPiToAsyncTests/PiToAsync_GivenInvalidDigits_PropagatesArgumentOutOfRangeException#0` | `pi_to_async_given_invalid_digits_propagates_argument_out_of_range_exception` | a worker computing PiTo(0) propagates the refusal through join |
| `RealPowZeroBaseTests/Pow_NonZeroBaseNegativeExponent_KeepsItsExistingRefusal#0` | `pow_nonzero_base_negative_exponent_keeps_its_existing_refusal` | 2^-3 keeps the reference's not-implemented refusal |
| `RealPowZeroBaseTests/Pow_ZeroBaseNegativeRealExponent_RefusesWithTheIntegerPathsRefusal#0` | `pow_zero_base_negative_real_exponent_refuses_with_the_integer_paths_refusal` | 0^-1 refuses with the integer path's message and parameter name |
| `RealPowZeroBaseTests/Pow_ZeroBasePositiveFractionalExponent_IsStillZero#0` | `pow_zero_base_positive_exponent_is_still_zero` | 0^0.5 is still zero |
| `RealPropertiesTests/DisplayDecimalPlaces_AfterSetting_ReturnsNewValue#0` | `display_decimal_places_after_setting_returns_new_value` | DisplayDecimalPlaces returns the value set |
| `RealPropertiesTests/DisplayDecimalPlaces_DefaultValue_IsOneHundred#0` | `display_decimal_places_default_value_is_one_hundred` | DisplayDecimalPlaces defaults to 100 |
| `RealPropertiesTests/Exponent_AfterSetting_ReturnsNewValue#0` | `exponent_after_setting_returns_new_value` | Exponent returns the value set |
| `RealPropertiesTests/MaxComputationDecimalPlaces_AfterSetting_ReturnsNewValue#0` | `max_computation_decimal_places_after_setting_returns_new_value` | MaxComputationDecimalPlaces returns the value set |
| `RealPropertiesTests/MaxComputationDecimalPlaces_DefaultValue_IsOneThousand#0` | `max_computation_decimal_places_default_value_is_one_thousand` | MaxComputationDecimalPlaces defaults to 1000 |
| `RealPropertiesTests/PeriodStart_GivenDefaultReal_IsZero#0` | `period_start_given_default_real_is_zero` | a default Real reports PeriodStart 0 |
| `RealSqrtAsyncTests/SqrtAsync_GivenNegativeInput_PropagatesArithmeticException#0` | `sqrt_async_given_negative_input_propagates_arithmetic_exception` | a worker computing sqrt(-1) propagates the arithmetic refusal through join |
| `RealSqrtBatchTests/Sqrt_GivenBatchContainingNegativeValue_PropagatesArithmeticException#0` | `sqrt_given_batch_containing_negative_value_propagates_arithmetic_exception` | a batch containing -1 refuses as a whole |
| `RealSqrtBatchTests/Sqrt_GivenBatchMixedIrrationals_MatchesElementwiseSerial#0` | `sqrt_given_batch_mixed_irrationals_matches_elementwise_serial` | a batch of 2,3,5 equals the elementwise serial roots |
| `RealSqrtBatchTests/Sqrt_GivenBatchPerfectSquares_ReturnsExactRoots#0` | `sqrt_given_batch_perfect_squares_returns_exact_roots` | a batch of 4,9,16,25 returns the exact roots 2,3,4,5 |
| `RealSqrtBatchTests/Sqrt_GivenBatchSingleElement_ReturnsArrayOfOneMatchingSerial#0` | `sqrt_given_batch_single_element_returns_array_of_one_matching_serial` | a one-element batch returns one value equal to the serial root |
| `RealSqrtBatchTests/Sqrt_GivenEmptyBatch_ReturnsEmptyArray#0` | `sqrt_given_empty_batch_returns_empty_array` | an empty batch returns an empty array |
| `RealSqrtTests/Sqrt_GivenNegativeInput_ThrowsArithmeticException#0` | `sqrt_given_negative_input_throws_arithmetic_exception` | sqrt(-1) refuses with the arithmetic error |
| `RealToStringTests/ToString_GivenNonPeriodicValue_TruncatesAtDisplayDecimalPlaces#0` | `to_string_given_non_periodic_value_truncates_at_display_decimal_places` | a non-periodic value renders at most DisplayDecimalPlaces fractional digits, here 1.12345 |
| `RealToStringTests/TryFormat_GivenInsufficientBuffer_ReturnsFalse#0` | `try_format_given_insufficient_buffer_returns_false` | TryFormat into an empty buffer returns false and writes nothing |
| `RealToStringTests/TryFormat_GivenSufficientBuffer_ReturnsTrueAndWritesCorrectly#0` | `try_format_given_sufficient_buffer_returns_true_and_writes_correctly` | TryFormat into a sufficient buffer returns true and writes exactly ToString() |

### 2c. Symbolics (22)

| Census row (prefix omitted) | Rust test | Assertion |
|---|---|---|
| `AcceptanceTests/Integrate_Table_AllVerify#0` | `integration_table_rows_are_exact_and_verify` | the seven table entries integrate to the reference antiderivatives and verify |
| `DebugIntegrate/RationalDenominator_IntegratesWithSelfVerification#0` | `integration_table_rows_are_exact_and_verify` | 1/(x^2-1) integrates to the table antiderivative instead of the unevaluated fallback |
| `DxSemanticClosureTests/Solve_QuarticWithRealRoots_DefaultDomain_KeepsRootOfRoots#0` | `solve_quartic_with_real_roots_keeps_the_exact_radicals` | x^4-5x^2 solves Exactly with 3 solutions: -sqrt(20)/2, sqrt(20)/2, 0 |
| `IndeterminateExponentialLimitTests/DeterminedLimits_KeepExactlyTheirPreviousAnswers#0` | `determined_limits_keep_their_answers` | sin(x)/x, 1/x's two-sided pole, the ratio at infinity, x^2+1 at 2 and x log x keep their answers |
| `IndeterminateExponentialLimitTests/Family_AnswersEveryExactExponent#0` | `indeterminate_exponential_limits_answer_the_exact_closed_form` | e^3, e at -inf, e^(3/2) and the e^a template are answered exactly |
| `IndeterminateExponentialLimitTests/IndeterminateShapes_AreRefused_NeverAnsweredWithTheValueAtThePoint#0` | `indeterminate_exponential_shapes_stay_unevaluated_with_a_reason` | four indeterminate shapes stay Unevaluated with a diagnostic instead of the value at the point |
| `IndeterminateExponentialLimitTests/Probe_LimitOfOnePlusOneOverXToTheX_IsE#0` | `indeterminate_exponential_limits_answer_the_exact_closed_form` | limit((1+1/x)^x,x,inf) is the exact e |
| `IndeterminateExponentialLimitTests/Probe_LimitOfOnePlusTwoOverXToTheX_IsESquared#0` | `indeterminate_exponential_limits_answer_the_exact_closed_form` | limit((1+2/x)^x,x,inf) is the exact e^2 |
| `IndeterminateExponentialLimitTests/Probe_LimitOfOnePlusXToTheOneOverX_IsE#0` | `indeterminate_exponential_limits_answer_the_exact_closed_form` | limit((1+x)^(1/x),x,0) is the exact e |
| `IndeterminateExponentialLimitTests/Probes_CrossTheWireAsExactClosedForms#0` | `indeterminate_exponential_limits_answer_the_exact_closed_form` | the first probe crosses the wire with status Value and exactness AlgebraicExact |
| `IndeterminateExponentialLimitTests/SubstitutionShapedLimits_StillAnswerImmediately#0` | `determined_limits_keep_their_answers` | the substitution-shaped limits (x^2+1, (1+x)^2, 2^x, x^x) still answer immediately |
| `KernelHardeningTests/CubicRoots_SatisfyPolynomial_Symbolically#0` | `cubic_roots_satisfy_polynomial_symbolically` | every Cardano root of x^3-1 expanded back into the cubic is exactly zero |
| `KernelHardeningTests/CubicRoots_SatisfyPolynomial_Symbolically#1` | `cubic_roots_satisfy_polynomial_symbolically` | every root of x^3-x expanded back into the cubic is exactly zero |
| `KernelHardeningTests/Diff_Abs_CarriesNonzeroCondition#0` | `diff_abs_carries_nonzero_condition` | diff(abs(x),x) is a piecewise with one != branch carrying diff(abs(x),x) otherwise |
| `KernelHardeningTests/Diff_Floor_IsUnevaluatedDerivative_NotZero#0` | `diff_floor_is_unevaluated_derivative_not_zero` | diff(floor(x),x) is an unevaluated derivative, never zero |
| `KernelHardeningTests/Diff_Piecewise_DifferentiatesBranches#0` | `diff_piecewise_differentiates_branches` | piecewise branches differentiate to 2x / otherwise -2x with guards carried |
| `KernelHardeningTests/FreeOf_DoesNotTreatPiecewiseAsConstant#0` | `free_of_does_not_treat_piecewise_as_constant` | a symbol in a branch or guard is not free; y^2 stays free of x |
| `KernelHardeningTests/HugeExponent_CompactForm_RoundTrips#0` | `huge_exponent_compact_form_round_trips` | 15e1000000000 prints compactly and parses back to the same pair |
| `KernelHardeningTests/MathIR_Deserialize_RejectsOldVersion#0` | `mathir_old_version_is_rejected_with_the_reference_message` | MathIR version 1 is refused with ParseError and the reference message quoting the version |
| `KernelHardeningTests/RootOf_CloseRoots_AllFoundAndIndexedStably#0` | `rootof_close_roots_all_found_and_indexed_stably` | x(x-1/1000)(x-1) counts 3 roots and each RootOf index isolates 0, 0.001, 1 |
| `KernelHardeningTests/RootOf_NoRealRoots_FailsExplicitly#0` | `rootof_no_real_roots_fails_explicitly` | N(RootOf(x^2+1,0)) refuses explicitly instead of returning a complex value |
| `KernelHardeningTests/YunSquareFree_MultiplicitiesAreCorrect#0` | `yun_square_free_multiplicities_are_correct` | x^2(x-1) factors to (x-1) once and x twice |

## 3. Refusals

The ten `RealTrigFastPathTests` rows are **not** mapped and stay deferred; their curated reasons
were already in `parity/manifest-map.json` from round 26 and were re-checked, not rewritten. They
assert private static/instance members by reflection (`ReduceToTwoPi`, `WholeQuotient`,
`Truncate`, `HalfOf`, `TrySpecialAngle`, `SpecialAngleCandidate`, `Normalize`,
`IsBelowDecimalGuard`, `CosTaylor`, `ToInteger`) or a wall-clock budget; the Rust kernel has no
matching private surface, and adding test-only hooks that reproduce the private implementation
would compare the helper with itself rather than the public API.

## 4. Translation notes

These are representational differences recorded so the mapping is not read as byte-identical:

- **Natural's CLR interfaces.** Rust has no interface assignability; each structure row resolves to
  the compile-time trait bound or trait method that carries the same capability. `INumber` is the
  crate's additive `Number` trait. `Natural_TypeName_IsNatural` pins Rust's canonical name
  `lovelace_nat::Nat` and the `Natural` alias that preserves the reference spelling.
- **Null string inputs.** `Parse(null)`/`TryParse(null)` become `from_decimal_opt(None)` /
  `from_str_opt(None)`: Rust has no null string, and the missing input is refused the same way.
- **BigInteger cross-checks.** The Natural randomized rows replace `BigInteger` with exact
  algebraic identities (and a `u128` reference on the single-digit tier); the division identity
  `q*b + r == a, r < b` determines the quotient and remainder uniquely.
- **AsyncLocal.** Rust threads do not capture AsyncLocal state. The additive
  `precision_snapshot`/`with_precision_values` seam is the explicit equivalent, and the async
  sqrt/pi rows assert propagation through a worker thread's `join` because Rust has no async
  surface to await.
- **PeriodStart.** Rust's existing `period_start()` keeps the `-1` no-period sentinel; the
  additive `period_start_or_zero()` reports the reference's `0`.
- **MathIR version refusal.** Asserted as the same `InvalidInput`/`ParseError` code and the
  verbatim message the reference quotes, instead of a CLR `FormatException` type.
- **Limits.** The short-form builtin publishes the `LimitResult` record; the expected display text
  is the oracle's own exact closed form (`e`, `e^2`, `e^3`, `e^(3/2)`, `e^a`).
- **Quartic solver.** The expression-root carrier has no multiplicity channel; `Solve_Quartic...`
  asserts the count (3) and the exact radicals, both of which the C# row asserts. The reported
  multiplicity of the zero root (1 vs the reference's 2) is not asserted by that row and is recorded
  in the lane report, not fixed here.

## 5. Verification

Environment: `RUSTUP_HOME=<repo>/.toolchain/rustup`, `CARGO_HOME=<repo>/.toolchain/cargo`,
`TMP=TEMP=<repo>/rust/target/tmp`. The mutation gate is the prerequisite:
`docs/goal-parity-contract/round-39/mutation-gate.md` records **12/12 detected, 12/12 reverted
trees pass** and a clean `git status` for `rust/` before this round ran.

| Gate | Command | Observed |
|---|---|---|
| Inventory + full suite | `powershell -File parity/gen-rust-tests.ps1` | `crates=11 tests=4808 suiteExit=0 passed=4808 failed=0` |
| Manifest generation | `node parity/gen-manifest.mjs` | `rows: 6209`, `ported: 5720`, `deferred: 371`, `excluded: 118`, `coveragePercent: 92.12`, `rustTests: 4808` |
| Manifest gate | `node parity/tools/gate-manifest.mjs` | `failures: []`; `PASS (structural): 6209 test cases dispositioned; 5720 claim Rust coverage (92.12%).` |
| Coverage report | `node parity/tools/coverage-report.mjs` | `coverage.md written: 5720/6209 ported (92.12%)` |
| Corpus | `rust/target/release/lovelace-corpus.exe` | `corpus: 214 passed, 0 failed, 214 total` |
| Fixtures | `node parity/tools/gate-fixtures.mjs` | `PASS: oracle 20/20, rust 20/20` |
| CLI matrix | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16`; `PASS: every RUN-001..RUN-014 row behaves like the oracle` |

The generated `parity/manifest.toml` and `parity/coverage.md` carry the same 5,720/6,209 number the
gate reads. `git status` after the run shows only the regenerated parity artifacts
(`coverage.md`, `manifest.toml`, `rust-tests.json`), this report, and the pre-existing untracked
`cryptanalysis/`; no `rust/` source is modified.

## 6. What remains deferred

371 rows remain deferred: 193 in `Lovelace.Symbolics.Tests` (the largest single gap), 10
`RealTrigFastPathTests` refusals, and 168 in the other projects. The round-39 lanes closed the rows
they names; the next mapping lane needs new production surfaces (or approved exclusions) before the
remaining 371 can move.
