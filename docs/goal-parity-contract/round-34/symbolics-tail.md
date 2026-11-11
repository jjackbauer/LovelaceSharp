# Round 34 - Symbolics tail: per-row adjudication and port

Status: complete and verified. 424 deferred `Lovelace.Symbolics.Tests` rows were adjudicated one by one:
69 are mappable now and ported, 347 are blocked by named production gaps grouped into 51 measured
families, and 8 need a live SymPy oracle. The workspace suite is green (4,563 passed, 0 failed) and
the manifest gate passes at 5,362/6,209 (86.36 %). The full verification block is section 4.

## 0. Population

| Measure | Rows |
|---|---:|
| Deferred `Lovelace.Symbolics.Tests` rows in `parity/manifest.toml` | 424 |
| (a) mappable now and ported | 69 |
| (b) blocked by a named production gap | 347 |
| (c) blocked by a missing external capability (live SymPy) | 8 |
| Distinct deferred classes | 62 |

Every row is classified in the appendix. A row is (a) only when a Rust test asserts the same case
the C# assertion demands; a weaker proxy is recorded as (b). The 69 new tests are:

- `rust/lovelace-lang/src/symbolics_tail_round34.rs` - 66 tests
- `rust/lovelace-symbolics/src/tail_round34.rs` - 3 tests (kernel arithmetic/polynomial rows)

## 1. (a) Mapped rows (69)

| Census row | Rust test |
|---|---|
| `AcceptanceTests/Diff_ProductAndChain_Work#0` | `lovelace-lang::symbolics_tail_round34::acceptance_diff_product_and_chain_work` |
| `AcceptanceTests/Limit_SinOverX_AtZero_IsOne#0` | `lovelace-lang::symbolics_tail_round34::acceptance_limit_sin_over_x_at_zero_is_one` |
| `AcceptanceTests/Optimize_Lower_Evaluate_RoundTrip#0` | `lovelace-lang::symbolics_tail_round34::acceptance_optimize_lower_evaluate_round_trip` |
| `AcceptanceTests/Rational_NormalizationAndArithmetic#0` | `lovelace-symbolics::tail_round34::acceptance_rational_normalization_and_arithmetic` |
| `AcceptanceTests/Simplify_Pythagorean_CollapsesToOne#0` | `lovelace-lang::symbolics_tail_round34::acceptance_simplify_pythagorean_collapses_to_one` |
| `CapabilitiesBuiltinTests/Exactness_SaysWhetherTheSetsAreExhaustiveOrBestEffort#0` | `lovelace-lang::symbolics_tail_round34::capabilities_exactness_says_exhaustive_or_best_effort` |
| `CapabilitiesBuiltinTests/UnsupportedOperations_AreRecordsCarryingCodeCategoryAndMessage#0` | `lovelace-lang::symbolics_tail_round34::capabilities_unsupported_operations_are_records` |
| `CompilationTests/Kernel_ScalarEvaluation#0` | `lovelace-lang::symbolics_tail_round34::compilation_kernel_scalar_evaluation` |
| `DiagnosticContractTests/BoundedTransformation_CarriesABudgetExceededDiagnostic#0` | `lovelace-lang::symbolics_tail_round34::diagnostic_contract_bounded_transformation_carries_budget_diagnostic` |
| `DxSemanticClosureTests/Language_SolveMultiplicityAndNoSolutions_AreHonest#0` | `lovelace-lang::symbolics_tail_round34::dx_semantic_solve_multiplicity_and_no_solutions_are_honest` |
| `DxSemanticClosureTests/Language_SolveQuartic_MixedRoots_IsPartial_NotComplete#0` | `lovelace-lang::symbolics_tail_round34::dx_semantic_solve_quartic_mixed_roots_is_partial_not_complete` |
| `DxSemanticClosureTests/Transform_ExpLog_CarriesFiniteLogCondition#0` | `lovelace-lang::symbolics_tail_round34::dx_semantic_transform_exp_log_carries_finite_log_condition` |
| `DxStructuredResultsTests/CompileFull_ExposesKernelMetadata_AndBatchAcceptsRecord#0` | `lovelace-lang::symbolics_tail_round34::dx_structured_compile_full_exposes_kernel_metadata_and_batch` |
| `DxStructuredResultsTests/DomainValues_AreFirstClass#0` | `lovelace-lang::symbolics_tail_round34::dx_structured_domain_values_are_first_class` |
| `DxStructuredResultsTests/LimitFull_ExposesExistenceAndOneSidedValues#0` | `lovelace-lang::symbolics_tail_round34::dx_structured_limit_full_exposes_existence_and_one_sided_values` |
| `DxStructuredResultsTests/SimplifyFull_CarriesConditionsAndSteps#0` | `lovelace-lang::symbolics_tail_round34::dx_structured_simplify_full_carries_conditions_and_steps` |
| `DxStructuredResultsTests/SimplifyFull_ExpLog_CarriesFiniteCondition#0` | `lovelace-lang::symbolics_tail_round34::dx_structured_simplify_full_exp_log_carries_finite_condition` |
| `DxStructuredResultsTests/SolveFull_DomainFieldAndRealOption#0` | `lovelace-lang::symbolics_tail_round34::dx_structured_solve_full_domain_field_and_real_option` |
| `DxStructuredResultsTests/SolveFull_ExposesStatusSolutionsAndConditions#0` | `lovelace-lang::symbolics_tail_round34::dx_structured_solve_full_exposes_status_solutions_and_conditions` |
| `DxStructuredResultsTests/Type_UsesOneVocabulary_ForEveryKind#9` | `lovelace-lang::symbolics_tail_round34::dx_structured_type_uses_one_vocabulary_for_every_kind` |
| `EvalirDigitArgumentContractTests/ACountOutsideTheBound_IsRefused#0` | `lovelace-lang::symbolics_tail_round34::evalir_digit_count_outside_bound_evalir_zero` |
| `EvalirDigitArgumentContractTests/ACountOutsideTheBound_IsRefused#1` | `lovelace-lang::symbolics_tail_round34::evalir_digit_count_outside_bound_evalir_batch_zero` |
| `EvalirDigitArgumentContractTests/ACountOutsideTheBound_IsRefused#2` | `lovelace-lang::symbolics_tail_round34::evalir_digit_count_outside_bound_evalir_minus_five` |
| `EvalirDigitArgumentContractTests/ACountOutsideTheBound_IsRefused#3` | `lovelace-lang::symbolics_tail_round34::evalir_digit_count_outside_bound_evalir_batch_minus_five` |
| `EvalirDigitArgumentContractTests/ACountWiderThanInt64_IsRefusedAndNamed#0` | `lovelace-lang::symbolics_tail_round34::evalir_digit_count_wider_than_int64_evalir` |
| `EvalirDigitArgumentContractTests/ACountWiderThanInt64_IsRefusedAndNamed#1` | `lovelace-lang::symbolics_tail_round34::evalir_digit_count_wider_than_int64_evalir_batch` |
| `EvalirDigitArgumentContractTests/ANonIntegralCount_IsNotReportedAsTheIntegralKindCase#0` | `lovelace-lang::symbolics_tail_round34::evalir_non_integral_count_two_point_five` |
| `EvalirDigitArgumentContractTests/ANonIntegralCount_IsNotReportedAsTheIntegralKindCase#1` | `lovelace-lang::symbolics_tail_round34::evalir_non_integral_count_two_point_nine` |
| `EvalirDigitArgumentContractTests/ANonIntegralCount_IsNotReportedAsTheIntegralKindCase#2` | `lovelace-lang::symbolics_tail_round34::evalir_non_integral_count_zero_point_five` |
| `EvalirDigitArgumentContractTests/ANonIntegralCount_IsNotReportedAsTheIntegralKindCase#3` | `lovelace-lang::symbolics_tail_round34::evalir_non_integral_count_one_point_zero_zero_zero_zero_zero_zero_one` |
| `EvalirDigitArgumentContractTests/ANonIntegralDigitCount_IsRefusedAndNamed#0` | `lovelace-lang::symbolics_tail_round34::evalir_non_integral_digit_count_evalir` |
| `EvalirDigitArgumentContractTests/ANonIntegralDigitCount_IsRefusedAndNamed#1` | `lovelace-lang::symbolics_tail_round34::evalir_non_integral_digit_count_evalir_batch` |
| `EvalirDigitArgumentContractTests/ATextDigitCount_IsRefusedAsTheSameArgumentError#0` | `lovelace-lang::symbolics_tail_round34::evalir_text_digit_count_is_refused_as_the_same_argument_error` |
| `ExactnessFlagTests/SqrtOfTwoInTheLanguage_IsARealValue_NotASymbolicRadical#0` | `lovelace-lang::symbolics_tail_round34::exactness_sqrt_of_two_is_a_real_value` |
| `ExpectedFailureBoundaryTests/Limit_TwoSidedPole_StillReportsOpposingSides#0` | `lovelace-lang::symbolics_tail_round34::expected_failure_limit_two_sided_pole_reports_opposing_sides` |
| `HyperbolicBuiltinsAndNegativeIntegerPowersTests/NegativeSquareRootsAndHalfPowers_NowAnswerExactly_WhileOtherNonIntegerPowersStillThrow#0` | `lovelace-lang::symbolics_tail_round34::hyperbolic_negative_square_roots_and_half_powers_answer_exactly` |
| `HyperbolicBuiltinsAndNegativeIntegerPowersTests/SymbolToTheMinusOne_StaysTheSymbolicReciprocalPower#0` | `lovelace-lang::symbolics_tail_round34::hyperbolic_symbol_to_the_minus_one_stays_symbolic` |
| `IndeterminateExponentialLimitTests/UndeterminedLimit_ClaimsNoExactness_AndDeterminedOnesKeepTheirs#0` | `lovelace-lang::symbolics_tail_round34::indeterminate_undetermined_limit_claims_no_exactness` |
| `KernelHardeningTests/AbsSquare_NotSimplified_ForComplex#0` | `lovelace-lang::symbolics_tail_round34::kernel_hardening_abs_square_not_simplified_for_complex` |
| `KernelHardeningTests/AddDroppingZeroCoefficientOverPole_KeepsDefinedness#0` | `lovelace-lang::symbolics_tail_round34::kernel_hardening_add_dropping_zero_coefficient_over_pole_keeps_definedness` |
| `KernelHardeningTests/ExpLog_SafeSimplify_RequiresDefinednessConditions#0` | `lovelace-lang::symbolics_tail_round34::kernel_hardening_exp_log_safe_simplify_requires_definedness_conditions` |
| `KernelHardeningTests/Limit_OneOverX_LeftMinusInf#0` | `lovelace-lang::symbolics_tail_round34::kernel_hardening_limit_one_over_x_left_minus_inf` |
| `KernelHardeningTests/Limit_OneOverX_RightPlusInf#0` | `lovelace-lang::symbolics_tail_round34::kernel_hardening_limit_one_over_x_right_plus_inf` |
| `KernelHardeningTests/Limit_OneOverX_TwoSidedDoesNotExist#0` | `lovelace-lang::symbolics_tail_round34::kernel_hardening_limit_one_over_x_two_sided_does_not_exist` |
| `KernelHardeningTests/Limit_OneOverXSquared_BothSidesPlusInf#0` | `lovelace-lang::symbolics_tail_round34::kernel_hardening_limit_one_over_x_squared_both_sides_plus_inf` |
| `KernelHardeningTests/Limit_SeriesOffset_OneMinusCosOverX_IsZero#0` | `lovelace-lang::symbolics_tail_round34::kernel_hardening_limit_series_offset_one_minus_cos_over_x_is_zero` |
| `KernelHardeningTests/LogExp_NotSimplified_Unconstrained#0` | `lovelace-lang::symbolics_tail_round34::kernel_hardening_log_exp_not_simplified_unconstrained` |
| `KernelHardeningTests/MinusOne_HugeExponent_ParityIsExact#0` | `lovelace-lang::symbolics_tail_round34::kernel_hardening_minus_one_huge_exponent_parity_is_exact` |
| `KernelHardeningTests/RationalRoots_IncludeZero#0` | `lovelace-symbolics::tail_round34::kernel_hardening_rational_roots_include_zero` |
| `KernelHardeningTests/RootOf_MultipleRoot_IsSquareFreeNormalized#0` | `lovelace-symbolics::tail_round34::kernel_hardening_root_of_multiple_root_is_square_free_normalized` |
| `KernelHardeningTests/Simplify_XOverX_CancelsWithCondition#0` | `lovelace-lang::symbolics_tail_round34::kernel_hardening_simplify_x_over_x_cancels_with_condition` |
| `KernelHardeningTests/Solve_RationalEquation_HonorsExcludedDenominator#0` | `lovelace-lang::symbolics_tail_round34::kernel_hardening_solve_rational_equation_honors_excluded_denominator` |
| `KernelHardeningTests/XOverX_IsNotFoldedByConstruction#0` | `lovelace-lang::symbolics_tail_round34::kernel_hardening_x_over_x_is_not_folded_by_construction` |
| `KernelHardeningTests/ZeroPowZero_IsOne_DocumentedConvention#0` | `lovelace-lang::symbolics_tail_round34::kernel_hardening_zero_pow_zero_is_one` |
| `LimitExistenceTriStateTests/KernelUndeterminedSide_HasNoValue#0` | `lovelace-lang::symbolics_tail_round34::limit_existence_undetermined_side_has_no_value` |
| `PrettyParenthesesTests/ReportedCase_FactoredSquare_HasOnePair#0` | `lovelace-lang::symbolics_tail_round34::pretty_parentheses_factored_square_has_one_pair` |
| `RewriteProtocolTests/ConditionalRule_CarriesDeclaredConditions#0` | `lovelace-lang::symbolics_tail_round34::rewrite_protocol_conditional_rule_carries_declared_conditions` |
| `RewriteProtocolTests/ConditionalRule_DoesNotFireOnUnknown_ButFiresWhenProven#0` | `lovelace-lang::symbolics_tail_round34::rewrite_protocol_conditional_rule_does_not_fire_on_unknown` |
| `SeriesKinkDeterminismTests/KinkSeries_TakesTheSymPyValueAtTheSamplePoint#7` | `lovelace-lang::symbolics_tail_round34::series_kink_sin_takes_the_value_at_the_sample_point` |
| `SeriesKinkDeterminismTests/LimitEngine_TwoSidedKinkAnswers_AreUnchanged#1` | `lovelace-lang::symbolics_tail_round34::series_kink_limit_engine_one_minus_cos_over_x_squared` |
| `SeriesKinkDeterminismTests/LimitEngine_TwoSidedKinkAnswers_AreUnchanged#3` | `lovelace-lang::symbolics_tail_round34::series_kink_limit_engine_abs_over_x_is_unevaluated` |
| `SeriesLaurentAndBranchPointTests/Series_AcrossAPoleOrABranchPoint_PublishesTheExpansion#3` | `lovelace-lang::symbolics_tail_round34::series_laurent_sqrt_at_zero_is_unchanged` |
| `SuiteIntegrationTests/Symbol_Builtin_Works#0` | `lovelace-lang::symbolics_tail_round34::suite_integration_symbol_builtin_works` |
| `SuiteIntegrationTests2/Apart_ThroughEngine#0` | `lovelace-lang::symbolics_tail_round34::suite_integration_apart_through_engine` |
| `SystemSolveProofTests/Wire_AbsenceOfSolutions_IsUnevaluatedWithADiagnostic#0` | `lovelace-lang::symbolics_tail_round34::system_solve_proof_wire_absence_case_zero` |
| `SystemSolveProofTests/Wire_AbsenceOfSolutions_IsUnevaluatedWithADiagnostic#1` | `lovelace-lang::symbolics_tail_round34::system_solve_proof_wire_absence_case_one` |
| `SystemSolveProofTests/Wire_AbsenceOfSolutions_IsUnevaluatedWithADiagnostic#2` | `lovelace-lang::symbolics_tail_round34::system_solve_proof_wire_absence_case_two` |
| `SystemSolveTests/LinearSystem_SingleSolution#0` | `lovelace-lang::symbolics_tail_round34::system_solve_linear_system_single_solution` |
| `UnsatisfiablePropagationTests/ConditionPayload_IsStructuredForEveryReachableAtomKind#0` | `lovelace-lang::symbolics_tail_round34::unsatisfiable_propagation_condition_payload_is_structured` |

## 2. (b) Blocked families (347 rows)

Ranked by rows. Each entry names the exact Rust behaviour measured against the pinned oracle
(`dotnet exec Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll`); "measured" means the same script
was run through both runners and the divergence recorded.

| # | Family | Rows | Measured Rust behaviour vs the C# assertion |
|---:|---|---:|---|
| 1 | assumption/tri-state kernel | 33 | Rust has `assume`/`assume_positive` only as simplification inputs: no Tri-state query, AssumptionSet, contradiction or interval surface, and the domain-conditional rewrites are absent. Measured: `symbol("x", real); simplify(sqrt(x^2))` -> `sqrt(x^2)` (C# `abs(x)`); `simplify(abs(x)^2)` under real -> `abs(x)^2` (C# `x^2`); `simplify(log(exp(x)))` under real -> unchanged (C# `x`); `assume(x == 0); simplify_full(x/x)` -> `Satisfied` with `[x != 0]` (C# `Unsatisfiable` + `UnsatisfiableConditions()`). |
| 2 | RealLiteral / RealConstant carrier | 33 | The Rust Expr model has no real-constant leaf and the crate has no RealLiteral API, so the 500-digit and approximate-argument rows cannot be constructed at all. Measured: `inspect(subs(x+1,x,pi(30))).exact` -> `True` (C# `False`); `evalf(sin(pi(1)*1/6), 40)` -> error `sin needs a number or an expression.` (C# 0.4939846521590624789945640658702374513097); the same substitution row returns exact=true with a rational form. |
| 3 | falsification subsystem absent | 32 | No `ObservationClassifier`, `BoundaryBias`, `FalsificationProbeSet`, `FalsificationLoop` or report type exists anywhere in the Rust workspace; the rows assert that subsystem's own types and fingerprints. |
| 4 | system-solve nonlinear/contradiction | 21 | Measured: `solve_system_full([x^2 + y^2 == 1], [x, y])` -> Unevaluated `system-solve.non-polynomial: only polynomial systems are supported in v1.` (C# elimination-incomplete message); `solve_system_full([x + y == 1, x + y == 2], [x, y])` -> Unevaluated (C# NoSolutions/complete); `solve_system_full([x^2 + y == 1, x - y == 0], [x, y])` -> Unevaluated (C# two exact solutions). |
| 5 | matrix conditions/verification | 19 | `inv_full([[x,0],[0,y]])` publishes `conditions: []` where C# carries `[x*y != 0]`; `linsolve` on a singular matrix answers `[]` where C# throws `System is singular or underdetermined.`; `rank([[x,x],[x,x]])` -> 2 (C# 1) and the 3x3 identity -> 2 (C# 3); `linsolve` renders `-1/(x*y)` where C# renders `-x/x/(x*y)`. |
| 6 | pretty/latex grouping | 17 | Rust renders `x/(y*z)` where the reference prints `x/y/z`, and `x - y - a` where the reference prints `x - (y - a)`; the Pretty and LaTeX trees therefore group differently (the C# assertion parses both renderings and compares the trees). |
| 7 | series at a kink/pole | 17 | Measured: `series(abs(x), x, 0, 3)` -> `abs(x)` (C# `x + O(x^3)`); `series(1/x, x, 0, 2)` -> `0 + O(x^2)` (C# `x^-1 + O(x^2)`); `series(sin(x)/x, x, 0, 3)` -> `1 + O(x^3)` (C# `1 - 1/6*x^2 + O(x^3)`); `series(sqrt(abs(x)), x, 0, 2)` -> `sqrt(abs(x))` (C# `sqrt(x)`). |
| 8 | series truncation node representation | 16 | The smooth sin expansion has the right display but the wrong canonical form: Rust `(add (sym x) (mul (rat -1 6) (pow (sym x) (rat 3 1))) (fn O (pow (sym x) (rat 5 1))))` vs C# `(add (sym x) (mul (rat -1 6) (pow (sym x) (rat 3 1))) (order (sym x) (rat 0 1) (rat 5 1)))`. All 16 rows assert the C# canonical text. |
| 9 | documentation fixture / truncated census display | 15 | The census display name truncates the guide snippet (`DocumentedExample_MatchesEngine#61`, `#70`, `#71`, `#73`, `#75`, `#99`; UsageExamples rows), so the row cannot be tied to a specific documented result without guessing. |
| 10 | MathIR deserialize/lowering | 11 | No IrProgram deserialize, typing or validate surface: only `lower`/`optimize` to `#!mathir 2` text and `evalir`. The C# rows construct malformed/old-version IR and assert typed refusals. |
| 11 | solve complex algebraic roots | 8 | Measured: `solve_full(x^4 + 1 == 0, x)` -> `Partial`, `unrepresented_count 0`, no diagnostics (C# `Unevaluated`, 4 unrepresented, message `complex algebraic roots not supported (RootOf is real-only in v1).`); `solve(x^2 - 2 == 0, x, real)` -> `NoSolutions` (C# two exact `sqrt` roots). |
| 12 | piecewise node absent | 8 | The Rust Expr model and parser have no `piecewise`; `diff(piecewise(...))`, lazy-select kernels and piecewise-assumption rows cannot be expressed (`Unknown function 'piecewise'`). |
| 13 | NumOps/Evaluation domain vocabulary | 8 | No `NumOps`/`EvaluationException` layer exists in Rust; the C# rows assert CLR exception types from the numeric tower, while Rust failures are `LangError` at the language boundary. |
| 14 | limit engine incompleteness | 8 | Measured: `limit(x^2 + 1, x, 2)` -> record value `1` (C# 5); `limit((1 + 1/x)^x, x, inf)` -> Unevaluated `coefficient does not evaluate at the point` (C# `e`, AlgebraicExact); `limit((1 + 2/x)^x, x, inf)` -> Unevaluated (C# `e^2`); `limit(abs(x), x, 0)` -> Unevaluated (C# 0). |
| 15 | polynomial API surface absent | 8 | No `Content`, `PrimitivePart`, `Discriminant`, `Resultant`, `Gcd`, `Lcm` or `SquareFree` surface in the Rust polynomial helpers. |
| 16 | multi-parameter evalir | 7 | `evalir`/`evalir_batch` answer `evalir supports one free symbol.` for a two-parameter kernel, while C# evaluates row-wise (`[3, 7]`, `[3, 13]`) and refuses a non-whole-row flat input with a documented message. |
| 17 | rewrite engine protocol/options | 7 | `simplify_full` has no `MaxSteps`/`Trace`/`Options` surface and there are no `RewriteRule`/`RewriteEngine` objects, so the budget/trace/exception-protocol rows cannot be stated. |
| 18 | concurrency/async scope | 6 | The C# rows assert Task/AsyncLocal scope restoration and parallel identity; the Rust engine has no counterpart scope model. |
| 19 | solve completeness mapping | 6 | No `SolveCompletenessMapping` type; the `x^4 + 1` record differs (Partial/0 vs Unevaluated/4), so the documented status pairs cannot all be produced. |
| 20 | help/descriptor metadata | 5 | The engine exposes names/arity only (runner `--functions` lists 50 functions); there is no BuiltinDescriptor/help summary/examples/related metadata surface, and `help` is not a builtin in either runner. |
| 21 | cubic-root verification | 5 | `solve_polynomial` returns RootExpressions for cubics, but the crate has no numeric evaluator for them; the C# rows verify each root by a 60-digit residual (and symbolically by expand). |
| 22 | C# System.Random corpus (unreproducible) | 5 | The rows generate 40-200 expressions with a fixed `System.Random` seed; that sequence is not reproducible in Rust, so the corpus (and the `Assert.Same` half) cannot be rebuilt. |
| 23 | abs symbolic round-trip | 4 | `abs(x)` prints, but `diff(abs(x), x)` -> `0` (C# `piecewise(sign(x) if x != 0, diff(abs(x), x))`); `subs(abs(x), x, -3)` -> `abs(-3)` (C# `3`); `simplify_full(sqrt(x^2)).expression` stays `sqrt(x^2)` under real x (C# `abs(x)`). |
| 24 | print/node budget policy | 4 | No depth budget exists, and the node-budget policy differs: C# leaves a 3-node `x + 1` whole at MaxNodes 1 via the kernel character allowance, while the Rust projection abbreviates whenever node_count > budget. |
| 25 | family verification | 4 | The Rust solver reports family templates, but every SinFamily member is not verified numerically against the original equation. |
| 26 | unsatisfiable propagation | 4 | No `UnsatisfiableConditions` marker: `assume(x == 0); simplify_full(x/x)` stays `Satisfied` with `[x != 0]` where C# publishes `Unsatisfiable` and `[UnsatisfiableConditions()]`. |
| 27 | hash-consing identity | 3 | `Expr` is a value enum and there is no interning: `Assert.Same(e, back)` (the identity half of the fact) cannot be reproduced; a structural-equality test would be weaker. |
| 28 | debug harness (console-only) | 3 | DebugCubic/DebugEvalir/DebugIntegrate are console probes without fixture assertions; they have no wire surface to port. |
| 29 | diagnostics vocabulary | 3 | No `ErrorCategory` seven-member type at the language boundary, and kernel note text differs: `solve_full(x == x, x)` -> `No solver for this structure.` (C# `0 = 0: every value is a solution.`). |
| 30 | integral Real literal normalization | 2 | `evalir(..., 2.0)` is refused: `argument 3 (digits) must be a Natural or Integer; got the Real 2, which is integral - write it as 2.` while C# normalizes an exactly integral Real to the integer and answers 0.33. |
| 31 | abs/floor derivative (piecewise/derivative node) | 2 | `diff(abs(x), x)` -> 0 and `floor` is not a registered function in the Rust engine; C# asserts a guarded piecewise derivative and an unevaluated derivative respectively. |
| 32 | RootOf numeric indexing | 2 | `real_root_count` exists, but `N(RootOf(p, i))` and index isolation have no Rust surface; `rootof` is not a builtin in either runner. |
| 33 | rewrite proof registry absent | 2 | No `RewriteProofs` obligation registry (rule id -> statement/status/theorem) exists in Rust. |
| 34 | limit at a kink | 2 | `limit(abs(x), x, 0)` -> Unevaluated (C# 0) and `limit(sqrt(x), x, 0)` -> Unevaluated (C# 0); the smooth 1-cos row is mapped. |
| 35 | symbolic matrix equality | 1 | The Hessian entries are equal but print in a different commutative order, and `==` on symbolic values builds a relation rather than comparing structure; C# compares the entries structurally. |
| 36 | integration table gap | 1 | `integrate(x*exp(x), x)` stays `integrate(x*exp(x), x)` (C# `exp(x)*(x - 1)`), so the seven-case integrate table row cannot pass. |
| 37 | reduction argument-shape refusals | 1 | `max(1, 2)` -> 1 and `sum(x, 5)` -> x in Rust, where C# refuses both with `argument 1 must be an array or vector`. |
| 38 | integration diagnostics shape | 1 | `integrate_full(exp(x^2), x)` returns `IntegrationResult` with `diagnostics: []`, where C# carries two diagnostics (`integration.unevaluated` first, then `integration.no-closed-form`). |
| 39 | zero base under half power | 1 | `0^(1/2)` errors `Non-integer exponents are not yet supported.` where C# answers `0`; the supported list in this row therefore fails. |
| 40 | plot symbolic refusal | 1 | `plot(sin(x))` proceeds to plot output and fails with `Cannot write plot: Access is denied. (os error 5)` instead of the C# typed `InvalidOperation/DomainError` refusal. |
| 41 | exact numeric tier (NumRat) absent | 1 | `evalir(compile(2*x^3 + 1/2, [x]), [1/2], 40)` -> `0.75 (Real)`, where C# keeps an exact `NumRat` 3/4. |
| 42 | evalir polynomial-only kernel | 1 | `evalir(compile(exp(x), [x]), [0], 40)` -> `evalir needs a polynomial expression.` where C# returns 1 (approximate tier). |
| 43 | factor/expand corpus identity | 1 | The single `factor(-x^2 - 2*x - 1)` rendering matches, but the row asserts `expand(factor(p)) == p` over the whole corpus; that corpus identity is not measured and one rendering is not the row. |
| 44 | builtin count/metadata | 1 | The Rust runner registry lists 50 functions; C# asserts 107 builtins, with descriptors for the three hyperbolics. |
| 45 | zero-base negative power message | 1 | `0^-1` -> `Cannot divide by zero.` where C# expects `Base cannot be zero. (Parameter 'base')` and the diagnostic to name that message. |
| 46 | inverse-branch verification | 1 | `sqrt(x^2 + 1) - 2` is not solved to `x^2 = 3`; the C# row requires the two verified solutions. |
| 47 | huge negative rational power | 1 | `2^-65536` produced no runner result (no JSON) where C# returns the exact reciprocal with a 2^65536 denominator. |
| 48 | variadic min arity divergence | 1 | `min(x, y, 1)` -> `min(): argument 2 (axis) must be a Natural or Integer...` where C# accepts variadic min with any count >= 2. |
| 49 | zero-over-x rewrite rule absent | 1 | `simplify_full(0/x)` is unchanged where C# cancels it to 0 with `[x != 0]` via `rat.cancel-zero-over-x`. |
| 50 | polynomial square-free API | 1 | No `SquareFreeUnivariate` multiplicity API; `real_root_count` alone cannot state the factor/multiplicity assertion. |
| 51 | derivative node absent | 1 | The Rust Expr model has no derivative node; `diff(f, x_1)` folds to 0 in both runners, so the LaTeX-of-Derivative assertion cannot be reproduced. |

## 3. (c) Live-SymPy rows (8)

These rows call `SympyOracle.Require()`/`SympySession` and need a live SymPy installation; the Rust
workspace has no SymPy bridge, and asserting a recorded shape instead of the oracle call would be a
weaker assertion.

| Census row | What the oracle call decides |
|---|---|
| `DifferentialOracleTests/SympyOracle_Derivatives_Agree#0` | cross-checks the kernel operation against SymPy over the oracle corpus |
| `DifferentialOracleTests/SympyOracle_Factorization_Agrees#0` | cross-checks the kernel operation against SymPy over the oracle corpus |
| `DifferentialOracleTests/SympyOracle_Limits_Agree#0` | cross-checks the kernel operation against SymPy over the oracle corpus |
| `DifferentialOracleTests/SympyOracle_MatrixOperations_Agree#0` | cross-checks the kernel operation against SymPy over the oracle corpus |
| `DifferentialOracleTests/SympyOracle_Roots_Agree#0` | cross-checks the kernel operation against SymPy over the oracle corpus |
| `DifferentialOracleTests/SympyOracle_Solve_Agrees#0` | cross-checks the kernel operation against SymPy over the oracle corpus |
| `InfiniteFamilyCompletenessTests/SympyOracle_ExpOverComplexes_IsAnImageSet_AndTheTemplateMatchesIt#0` | solveset(exp(x)-c, x, Complexes) must print an ImageSet over the Integers with the 2*pi*i period |
| `SystemSolveProofTests/NonlinearElimination_AndTheBoundary_AgreeWithSympySolve#0` | sympy.solve must produce the same two solutions as the elimination |

## 4. Verification

The workspace suite was red while the concurrent `lovelace-real` lane was mid-change (3 then 2
Real/DSP precision tests failed). After its last source edit at 11:18:07, a 90-second quiet window was
required, then the suite and the pipeline below were run on the settled tree.

| Gate | Command | Observed result |
|---|---|---|
| Suite | `cargo test --workspace` | `4,563 passed; 0 failed` |
| Inventory | `parity/gen-rust-tests.ps1` | `crates=11 tests=4563 suiteExit=0 passed=4563 failed=0` |
| Manifest | `node parity/gen-manifest.mjs` | rows 6209: ported 5362, deferred 729, excluded 118; coverage 86.36 % |
| Manifest gate | `node parity/tools/gate-manifest.mjs` | `PASS (structural): 6209 test cases dispositioned; 5362 claim Rust coverage (86.36 %).` failures: [] |
| Coverage | `node parity/tools/coverage-report.mjs` | `coverage.md written: 5362/6209 ported (86.36 %)` |
| Fixtures | `node parity/tools/gate-fixtures.mjs` | `PASS: oracle 20/20, rust 20/20` |
| Corpus | `cargo run --release -p lovelace-corpus` | `corpus: 212 passed, 2 failed, 214 total` (sym/071 optimize IR text, sym/076 linsolve grouping - both pre-existing, outside this change) |

Symbolics project line: 1,226 cases, 871 ported, 355 deferred, 71.04 % (was 802 / 424 / 65.42 %).

The corpus failures are the two documented pre-existing divergences (the optimize lower text and the
flat-division linsolve rendering), not rows mapped in this round: the Acceptance row
`Optimize_Lower_Evaluate_RoundTrip` is asserted by the evaluated value (92), which the Rust test
checks, while the corpus example additionally pins the IR text that Rust renders without the repeated
constant.

## Appendix - every deferred row

| Census row | Class | Cat | Family |
|---|---|---|---|
| `AbsSymbolicRoundTripTests/Abs_Of_Symbol_Matches_The_Rewriters_Own_Abs_Node#0` | AbsSymbolicRoundTripTests | b | abs symbolic round-trip |
| `AbsSymbolicRoundTripTests/Diff_Of_Abs_Symbolic_Does_Not_Throw#0` | AbsSymbolicRoundTripTests | b | abs symbolic round-trip |
| `AbsSymbolicRoundTripTests/Simplify_Sqrt_Square_Under_Real_Is_Still_Abs_X#0` | AbsSymbolicRoundTripTests | b | abs symbolic round-trip |
| `AbsSymbolicRoundTripTests/Subs_Abs_Symbolic_At_Negative_Value_Gives_Three#0` | AbsSymbolicRoundTripTests | b | abs symbolic round-trip |
| `AcceptanceTests/Diff_ProductAndChain_Work#0` | AcceptanceTests | a | mapped |
| `AcceptanceTests/Hessian_Computes#0` | AcceptanceTests | b | symbolic matrix equality |
| `AcceptanceTests/Integrate_Table_AllVerify#0` | AcceptanceTests | b | integration table gap |
| `AcceptanceTests/Interning_EqualStructureSharesReference#0` | AcceptanceTests | b | hash-consing identity |
| `AcceptanceTests/Inverse_SymbolicMatrix_TimesOriginalIsIdentity#0` | AcceptanceTests | b | matrix conditions/verification |
| `AcceptanceTests/Limit_SinOverX_AtZero_IsOne#0` | AcceptanceTests | a | mapped |
| `AcceptanceTests/Optimize_Lower_Evaluate_RoundTrip#0` | AcceptanceTests | a | mapped |
| `AcceptanceTests/PrintParse_RoundTrips#0` | AcceptanceTests | b | hash-consing identity |
| `AcceptanceTests/Rational_NormalizationAndArithmetic#0` | AcceptanceTests | a | mapped |
| `AcceptanceTests/Simplify_Pythagorean_CollapsesToOne#0` | AcceptanceTests | a | mapped |
| `AcceptanceTests/Simplify_SqrtSquare_UnderAssumptions#0` | AcceptanceTests | b | assumption/tri-state kernel |
| `AssumptionAddScalingTests/Add_MixedAtomsInAnyInsertionOrder_YieldsTheSameCanonicalOrder#0` | AssumptionAddScalingTests | b | assumption/tri-state kernel |
| `AssumptionAddScalingTests/Add_OrderEqualsHistoricalAppendAndResort_ForEveryInsertionOrder#0` | AssumptionAddScalingTests | b | assumption/tri-state kernel |
| `AssumptionAddScalingTests/Add_SameAtomTwiceFromDifferentContexts_IsStoredOnce#0` | AssumptionAddScalingTests | b | assumption/tri-state kernel |
| `AssumptionAddScalingTests/Add_Scaling_800AtomsStaysUnder250ms_AndKeepsEveryAtomInOrder#0` | AssumptionAddScalingTests | b | assumption/tri-state kernel |
| `AssumptionScopeIsolationTests/NestedScopes_RestoreTheEnclosingScopeOnDispose#0` | AssumptionScopeIsolationTests | b | assumption/tri-state kernel |
| `AssumptionScopeIsolationTests/TwoConcurrentScopes_AreIsolated#0` | AssumptionScopeIsolationTests | b | assumption/tri-state kernel |
| `AssumptionSoundnessTests/BoundReasoning_MatchesReferenceTruthTable#0` | AssumptionSoundnessTests | b | assumption/tri-state kernel |
| `AssumptionSoundnessTests/ContradictionDetection_IsNeitherMissedNorInvented#0` | AssumptionSoundnessTests | b | assumption/tri-state kernel |
| `AssumptionSoundnessTests/DomainAtoms_MergeToTheNarrowest#0` | AssumptionSoundnessTests | b | assumption/tri-state kernel |
| `AssumptionSoundnessTests/IntervalBounds_WithUnevaluableBound_AreUnknown#0` | AssumptionSoundnessTests | b | assumption/tri-state kernel |
| `AssumptionSoundnessTests/PropertyAndRelationAtoms_Contradict#0` | AssumptionSoundnessTests | b | assumption/tri-state kernel |
| `AssumptionSoundnessTests/RealValuedBounds_ParticipateInReasoning#0` | AssumptionSoundnessTests | b | assumption/tri-state kernel |
| `AssumptionSoundnessTests/UnsatisfiableSet_AnswersUnsatisfiable_NotUnknown#0` | AssumptionSoundnessTests | b | assumption/tri-state kernel |
| `CapabilitiesBuiltinTests/AdvertisedCodesAndCategories_MatchTheLiveEnvelope#0` | CapabilitiesBuiltinTests | b | help/descriptor metadata |
| `CapabilitiesBuiltinTests/Capabilities_IsOneRegisteredBuiltin_WithCompleteMetadata#0` | CapabilitiesBuiltinTests | b | help/descriptor metadata |
| `CapabilitiesBuiltinTests/Exactness_SaysWhetherTheSetsAreExhaustiveOrBestEffort#0` | CapabilitiesBuiltinTests | a | mapped |
| `CapabilitiesBuiltinTests/ExactnessBestEffort_IsBackedByLiveRefusalsTheListDoesNotCarry#0` | CapabilitiesBuiltinTests | b | reduction argument-shape refusals |
| `CapabilitiesBuiltinTests/IntegrationRefusal_IsTypedStructuredAndClassLevel#0` | CapabilitiesBuiltinTests | b | integration diagnostics shape |
| `CapabilitiesBuiltinTests/PowerExponentClass_RefusesWhatItSays_AndNamesWhatIsSupported#0` | CapabilitiesBuiltinTests | b | zero base under half power |
| `CapabilitiesBuiltinTests/SolveDomainStatement_NamesItsOperation_AndEveryDomainStaysUsableElsewhere#0` | CapabilitiesBuiltinTests | b | solve complex algebraic roots |
| `CapabilitiesBuiltinTests/SymbolicPlot_IsATypedRefusal_NeverAnInternalInvariantFailure#0` | CapabilitiesBuiltinTests | b | plot symbolic refusal |
| `CapabilitiesBuiltinTests/UnsupportedOperations_AreRecordsCarryingCodeCategoryAndMessage#0` | CapabilitiesBuiltinTests | a | mapped |
| `CompilationTests/CompilationResult_IrText_RoundTrips#0` | CompilationTests | b | MathIR deserialize/lowering |
| `CompilationTests/Kernel_BatchEvaluation_Columnwise#0` | CompilationTests | b | multi-parameter evalir |
| `CompilationTests/Kernel_BatchLazySelect_MixedGuards#0` | CompilationTests | b | piecewise node absent |
| `CompilationTests/Kernel_BatchMatchesScalarEvaluation#0` | CompilationTests | b | multi-parameter evalir |
| `CompilationTests/Kernel_ExactTiers_Preserved#0` | CompilationTests | b | exact numeric tier (NumRat) absent |
| `CompilationTests/Kernel_LazySelect_UntakenBranchNeverEvaluates#0` | CompilationTests | b | piecewise node absent |
| `CompilationTests/Kernel_NamedAndPositional_Evaluate#0` | CompilationTests | b | multi-parameter evalir |
| `CompilationTests/Kernel_ScalarEvaluation#0` | CompilationTests | a | mapped |
| `CompilationTests/Kernel_Transcendental_ApproximateTier#0` | CompilationTests | b | evalir polynomial-only kernel |
| `ConcurrencyTests/AssumptionScopes_RestoreOnDispose#0` | ConcurrencyTests | b | concurrency/async scope |
| `ConcurrencyTests/ParallelCompilation_AgreesWithSerial#0` | ConcurrencyTests | b | concurrency/async scope |
| `ConcurrencyTests/ParallelFunctionCreation_NoDuplicateIds#0` | ConcurrencyTests | b | concurrency/async scope |
| `ConcurrencyTests/ParallelParse_AgreesWithSerial#0` | ConcurrencyTests | b | concurrency/async scope |
| `ConcurrencyTests/ParallelSimplifyDiff_AgreeWithSerial#0` | ConcurrencyTests | b | concurrency/async scope |
| `ConcurrencyTests/ParallelSymbolCreation_NoDuplicateIds#0` | ConcurrencyTests | b | concurrency/async scope |
| `DebugCubic/Cubic_Roots_SatisfyPolynomial#0` | DebugCubic | b | debug harness (console-only) |
| `DebugEvalir/Lower_Evaluate_FiveTerm#0` | DebugEvalir | b | debug harness (console-only) |
| `DebugIntegrate/RationalDenominator_IntegratesWithSelfVerification#0` | DebugIntegrate | b | debug harness (console-only) |
| `DiagnosticContractTests/BoundedTransformation_CarriesABudgetExceededDiagnostic#0` | DiagnosticContractTests | a | mapped |
| `DiagnosticContractTests/ErrorCategory_HasExactlyTheSevenFrozenMembers#0` | DiagnosticContractTests | b | diagnostics vocabulary |
| `DiagnosticContractTests/KernelNotes_CrossAsDiagnosticMessages#0` | DiagnosticContractTests | b | diagnostics vocabulary |
| `DiagnosticContractTests/UnsatisfiableTransformation_CarriesADomainErrorDiagnostic#0` | DiagnosticContractTests | b | diagnostics vocabulary |
| `DifferentialOracleTests/SympyOracle_Derivatives_Agree#0` | DifferentialOracleTests | c | c |
| `DifferentialOracleTests/SympyOracle_Factorization_Agrees#0` | DifferentialOracleTests | c | c |
| `DifferentialOracleTests/SympyOracle_Limits_Agree#0` | DifferentialOracleTests | c | c |
| `DifferentialOracleTests/SympyOracle_MatrixOperations_Agree#0` | DifferentialOracleTests | c | c |
| `DifferentialOracleTests/SympyOracle_Roots_Agree#0` | DifferentialOracleTests | c | c |
| `DifferentialOracleTests/SympyOracle_Solve_Agrees#0` | DifferentialOracleTests | c | c |
| `DocsSyncTests/Every_CSharpSnippet_AppearsInUsageExamples#0` | DocsSyncTests | b | documentation fixture / truncated census display |
| `DocsSyncTests/Every_LovelaceExample_HasAnExpectedResult#0` | DocsSyncTests | b | documentation fixture / truncated census display |
| `DomainFailureVocabularyTests/DivisionByZero_IsAnEvaluationFailure#0` | DomainFailureVocabularyTests | b | NumOps/Evaluation domain vocabulary |
| `DomainFailureVocabularyTests/DivisionByZero_OnTheRealPath_IsAnEvaluationFailure#0` | DomainFailureVocabularyTests | b | NumOps/Evaluation domain vocabulary |
| `DomainFailureVocabularyTests/LogOfZero_IsAnEvaluationFailure#0` | DomainFailureVocabularyTests | b | NumOps/Evaluation domain vocabulary |
| `DomainFailureVocabularyTests/ZeroToANegativePower_IsAnEvaluationFailure#0` | DomainFailureVocabularyTests | b | NumOps/Evaluation domain vocabulary |
| `DxSemanticClosureTests/AskRelation_ProvesWeakerBounds#0` | DxSemanticClosureTests | b | assumption/tri-state kernel |
| `DxSemanticClosureTests/AskRelation_RefutesContradictoryBounds#0` | DxSemanticClosureTests | b | assumption/tri-state kernel |
| `DxSemanticClosureTests/Assume_CompatibleRelations_DoNotThrow#0` | DxSemanticClosureTests | b | assumption/tri-state kernel |
| `DxSemanticClosureTests/DomainOf_NegativeIntegerPower_IsRationalNotInteger#0` | DxSemanticClosureTests | b | assumption/tri-state kernel |
| `DxSemanticClosureTests/Language_SolveMultiplicityAndNoSolutions_AreHonest#0` | DxSemanticClosureTests | a | mapped |
| `DxSemanticClosureTests/Language_SolveQuartic_MixedRoots_IsPartial_NotComplete#0` | DxSemanticClosureTests | a | mapped |
| `DxSemanticClosureTests/Language_SolveQuartic_ReportsUnevaluatedNote#0` | DxSemanticClosureTests | b | solve complex algebraic roots |
| `DxSemanticClosureTests/Solve_QuarticNoRealRoots_DefaultDomain_ReportsUnevaluated#0` | DxSemanticClosureTests | b | solve complex algebraic roots |
| `DxSemanticClosureTests/Solve_QuarticWithRealRoots_DefaultDomain_KeepsRootOfRoots#0` | DxSemanticClosureTests | b | solve complex algebraic roots |
| `DxSemanticClosureTests/Transform_BudgetExceeded_IsReported#0` | DxSemanticClosureTests | b | solve complex algebraic roots |
| `DxSemanticClosureTests/Transform_ExpLog_CarriesFiniteLogCondition#0` | DxSemanticClosureTests | a | mapped |
| `DxSemanticClosureTests/Transform_TraceIsOptIn_ButConditionsAlwaysSurvive#0` | DxSemanticClosureTests | b | solve complex algebraic roots |
| `DxSemanticClosureTests/Unsatisfiable_Sentinel_IsSticky#0` | DxSemanticClosureTests | b | assumption/tri-state kernel |
| `DxStructuredResultsTests/CompileFull_ExposesKernelMetadata_AndBatchAcceptsRecord#0` | DxStructuredResultsTests | a | mapped |
| `DxStructuredResultsTests/DomainValues_AreFirstClass#0` | DxStructuredResultsTests | a | mapped |
| `DxStructuredResultsTests/EvalirBatch_AcceptsNestedRowValues#0` | DxStructuredResultsTests | b | multi-parameter evalir |
| `DxStructuredResultsTests/EvalirBatch_FlatInputNotAWholeNumberOfRows_IsRejected#0` | DxStructuredResultsTests | b | multi-parameter evalir |
| `DxStructuredResultsTests/EvalirBatch_RowOrientedInput_EvaluatesRowByRow#0` | DxStructuredResultsTests | b | multi-parameter evalir |
| `DxStructuredResultsTests/EvalirBatch_RowWidthMismatch_IsRejectedWithTheDocumentedShape#0` | DxStructuredResultsTests | b | multi-parameter evalir |
| `DxStructuredResultsTests/Inspect_ReportsTheAssumptionsThatConstrainTheFreeSymbols#0` | DxStructuredResultsTests | b | assumption/tri-state kernel |
| `DxStructuredResultsTests/InvFull_ExposesDetCondition#0` | DxStructuredResultsTests | b | matrix conditions/verification |
| `DxStructuredResultsTests/LimitFull_ExposesExistenceAndOneSidedValues#0` | DxStructuredResultsTests | a | mapped |
| `DxStructuredResultsTests/LinsolveFull_SingularSystem_ReportsStatusNotThrow#0` | DxStructuredResultsTests | b | matrix conditions/verification |
| `DxStructuredResultsTests/ParameterDomainOf_MapsEveryDeclaredArm#0` | DxStructuredResultsTests | b | assumption/tri-state kernel |
| `DxStructuredResultsTests/SimplifyFull_CarriesConditionsAndSteps#0` | DxStructuredResultsTests | a | mapped |
| `DxStructuredResultsTests/SimplifyFull_ExpLog_CarriesFiniteCondition#0` | DxStructuredResultsTests | a | mapped |
| `DxStructuredResultsTests/SolveFull_DomainFieldAndRealOption#0` | DxStructuredResultsTests | a | mapped |
| `DxStructuredResultsTests/SolveFull_ExposesStatusSolutionsAndConditions#0` | DxStructuredResultsTests | a | mapped |
| `DxStructuredResultsTests/Type_UsesOneVocabulary_ForEveryKind#9` | DxStructuredResultsTests | a | mapped |
| `EvalfTruncatedConstantExactnessTests/ExactControls_AreUntouched#0` | EvalfTruncatedConstantExactnessTests | b | RealLiteral / RealConstant carrier |
| `EvalfWorkingPrecisionHonoursRequestedDecimalsTests/ValuesBelowTheComputationCap_StayInexact#0` | EvalfWorkingPrecisionHonoursRequestedDecimalsTests | b | RealLiteral / RealConstant carrier |
| `EvalirDigitArgumentContractTests/ACountOutsideTheBound_IsRefused#0` | EvalirDigitArgumentContractTests | a | mapped |
| `EvalirDigitArgumentContractTests/ACountOutsideTheBound_IsRefused#1` | EvalirDigitArgumentContractTests | a | mapped |
| `EvalirDigitArgumentContractTests/ACountOutsideTheBound_IsRefused#2` | EvalirDigitArgumentContractTests | a | mapped |
| `EvalirDigitArgumentContractTests/ACountOutsideTheBound_IsRefused#3` | EvalirDigitArgumentContractTests | a | mapped |
| `EvalirDigitArgumentContractTests/ACountWiderThanInt64_IsRefusedAndNamed#0` | EvalirDigitArgumentContractTests | a | mapped |
| `EvalirDigitArgumentContractTests/ACountWiderThanInt64_IsRefusedAndNamed#1` | EvalirDigitArgumentContractTests | a | mapped |
| `EvalirDigitArgumentContractTests/AnExactlyIntegralRealLiteral_IsTheIntegerItDenotes#0` | EvalirDigitArgumentContractTests | b | integral Real literal normalization |
| `EvalirDigitArgumentContractTests/AnExactlyIntegralRealLiteral_IsTheIntegerItDenotes#1` | EvalirDigitArgumentContractTests | b | integral Real literal normalization |
| `EvalirDigitArgumentContractTests/ANonIntegralCount_IsNotReportedAsTheIntegralKindCase#0` | EvalirDigitArgumentContractTests | a | mapped |
| `EvalirDigitArgumentContractTests/ANonIntegralCount_IsNotReportedAsTheIntegralKindCase#1` | EvalirDigitArgumentContractTests | a | mapped |
| `EvalirDigitArgumentContractTests/ANonIntegralCount_IsNotReportedAsTheIntegralKindCase#2` | EvalirDigitArgumentContractTests | a | mapped |
| `EvalirDigitArgumentContractTests/ANonIntegralCount_IsNotReportedAsTheIntegralKindCase#3` | EvalirDigitArgumentContractTests | a | mapped |
| `EvalirDigitArgumentContractTests/ANonIntegralDigitCount_IsRefusedAndNamed#0` | EvalirDigitArgumentContractTests | a | mapped |
| `EvalirDigitArgumentContractTests/ANonIntegralDigitCount_IsRefusedAndNamed#1` | EvalirDigitArgumentContractTests | a | mapped |
| `EvalirDigitArgumentContractTests/ATextDigitCount_IsRefusedAsTheSameArgumentError#0` | EvalirDigitArgumentContractTests | a | mapped |
| `ExactnessFlagTests/CachedFlag_EqualsTheLeafRule_OverTheWholeCorpus#0` | ExactnessFlagTests | b | RealLiteral / RealConstant carrier |
| `ExactnessFlagTests/ElementaryFunctionOfAnApproximateArgument_IsInexact#0` | ExactnessFlagTests | b | RealLiteral / RealConstant carrier |
| `ExactnessFlagTests/ElementaryFunctionOfAnApproximateArgument_IsInexact#1` | ExactnessFlagTests | b | RealLiteral / RealConstant carrier |
| `ExactnessFlagTests/ElementaryFunctionOfAnApproximateArgument_IsInexact#2` | ExactnessFlagTests | b | RealLiteral / RealConstant carrier |
| `ExactnessFlagTests/ElementaryFunctionOfAnApproximateArgument_IsInexact#3` | ExactnessFlagTests | b | RealLiteral / RealConstant carrier |
| `ExactnessFlagTests/ElementaryFunctionOfAnApproximateArgument_IsInexact#4` | ExactnessFlagTests | b | RealLiteral / RealConstant carrier |
| `ExactnessFlagTests/ElementaryFunctionOfAnApproximateArgument_IsInexact#5` | ExactnessFlagTests | b | RealLiteral / RealConstant carrier |
| `ExactnessFlagTests/InspectExact_IsFalse_WhenARealApproximationIsInTheTree#0` | ExactnessFlagTests | b | RealLiteral / RealConstant carrier |
| `ExactnessFlagTests/RadicalsOfExactBases_AreExact_AndApproximateBasesAreNot#0` | ExactnessFlagTests | b | RealLiteral / RealConstant carrier |
| `ExactnessFlagTests/SolverSolutionExactness_AndValueExact_Agree#0` | ExactnessFlagTests | b | solve complex algebraic roots |
| `ExactnessFlagTests/SqrtOfTwoInTheLanguage_IsARealValue_NotASymbolicRadical#0` | ExactnessFlagTests | a | mapped |
| `ExpectedFailureBoundaryTests/EvaluateToExpr_ApproximateArgument_ThrowingEvaluatorPropagates#0` | ExpectedFailureBoundaryTests | b | NumOps/Evaluation domain vocabulary |
| `ExpectedFailureBoundaryTests/EvaluateToExpr_ExactArgument_ThrowingEvaluatorPropagates#0` | ExpectedFailureBoundaryTests | b | NumOps/Evaluation domain vocabulary |
| `ExpectedFailureBoundaryTests/Limit_ThrowingEvaluatorPropagates#0` | ExpectedFailureBoundaryTests | b | NumOps/Evaluation domain vocabulary |
| `ExpectedFailureBoundaryTests/Limit_TwoSidedPole_StillReportsOpposingSides#0` | ExpectedFailureBoundaryTests | a | mapped |
| `ExpectedFailureBoundaryTests/TryCompare_ReportsUnorderedInsteadOfThrowing#0` | ExpectedFailureBoundaryTests | b | NumOps/Evaluation domain vocabulary |
| `FactorMetamorphicTests/Factor_ExpandsBackToItsInput_OverTheWholeCorpus#0` | FactorMetamorphicTests | b | factor/expand corpus identity |
| `Falsification.FalsificationLoopTests/Classifier_NamesEachClass_AndNeverFoldsADefectIntoOne#0` | Falsification.FalsificationLoopTests | b | falsification subsystem absent |
| `Falsification.FalsificationLoopTests/FalsificationFingerprintChild#0` | Falsification.FalsificationLoopTests | b | falsification subsystem absent |
| `Falsification.FalsificationLoopTests/FrozenBattery_IsAPrefixOfWhatTheSubstrateOffers_InItsOwnOrder#0` | Falsification.FalsificationLoopTests | b | falsification subsystem absent |
| `Falsification.FalsificationLoopTests/Generator_BracketsAKnownBoundary_TheNaturalSubtractionWidening#0` | Falsification.FalsificationLoopTests | b | falsification subsystem absent |
| `Falsification.FalsificationLoopTests/Generator_BracketsANonIntegralBoundary_AndEveryPointIsStrictlyInside#0` | Falsification.FalsificationLoopTests | b | falsification subsystem absent |
| `Falsification.FalsificationLoopTests/Generator_BroadPhaseSequenceIsStableUnderTheSameSeed_AndMovesWithIt#0` | Falsification.FalsificationLoopTests | b | falsification subsystem absent |
| `Falsification.FalsificationLoopTests/Hypothesis_BoundaryBiasBeatsUniform_OnAPlantedDefect#0` | Falsification.FalsificationLoopTests | b | falsification subsystem absent |
| `Falsification.FalsificationLoopTests/Loop_IsDeterministic_AcrossProcesses#0` | Falsification.FalsificationLoopTests | b | falsification subsystem absent |
| `Falsification.FalsificationLoopTests/Loop_IsDeterministic_SameSeedProducesAByteIdenticalReport#0` | Falsification.FalsificationLoopTests | b | falsification subsystem absent |
| `Falsification.FalsificationLoopTests/PlantedDefect_IsFalsified_AndTheReportNamesTheRegionAndTheInput#0` | Falsification.FalsificationLoopTests | b | falsification subsystem absent |
| `Falsification.FalsificationLoopTests/ProbeRecord_RoundTripsThroughItsDeterministicTextForm#0` | Falsification.FalsificationLoopTests | b | falsification subsystem absent |
| `Falsification.FalsificationLoopTests/RegistryLoop_ClassifiesEveryShippedRule_AndFindsNoDefect#0` | Falsification.FalsificationLoopTests | b | falsification subsystem absent |
| `Falsification.FalsificationLoopTests/Tolerance_IsAParameter_TheSameValuesDecideDifferentlyUnderDifferentTolerances#0` | Falsification.FalsificationLoopTests | b | falsification subsystem absent |
| `FalsificationGateTests/AssumptionBoundary_GuardFlipsExactlyAtZero#0` | FalsificationGateTests | b | falsification subsystem absent |
| `FalsificationGateTests/ComplexExclusion_EveryRealOnlyRuleHasAnObservedComplexCounterexample#0` | FalsificationGateTests | b | falsification subsystem absent |
| `FalsificationGateTests/ComplexExclusion_LogExp_IsStoppedByRealness_NotByUndefinedness#0` | FalsificationGateTests | b | falsification subsystem absent |
| `FalsificationGateTests/ComplexTreatment_PartitionsTheRegistry_WithExplicitCounts#0` | FalsificationGateTests | b | falsification subsystem absent |
| `FalsificationGateTests/Gate_Fails_WhenARuleThrows#0` | FalsificationGateTests | b | falsification subsystem absent |
| `FalsificationGateTests/Gate_ReportsFalsification_WhenARuleIsWrong#0` | FalsificationGateTests | b | falsification subsystem absent |
| `FalsificationGateTests/Gate_StillFalsifies_AComplexOnlyWrongRule#0` | FalsificationGateTests | b | falsification subsystem absent |
| `FalsificationGateTests/RegistryGate_EveryRegisteredRuleIsSampled_OnEveryRegion#0` | FalsificationGateTests | b | falsification subsystem absent |
| `FalsificationGateTests/Sampling_IsASuperset_OfTheOriginalPoints_AndEveryProbeIsDocumented#0` | FalsificationGateTests | b | falsification subsystem absent |
| `FalsificationTests/AbsNeg_Universal#0` | FalsificationTests | b | falsification subsystem absent |
| `FalsificationTests/AbsSquare_IsNotUniversal_OverComplex#0` | FalsificationTests | b | falsification subsystem absent |
| `FalsificationTests/AbsSquare_Real#0` | FalsificationTests | b | falsification subsystem absent |
| `FalsificationTests/CancelXOverX_NonZeroPoints#0` | FalsificationTests | b | falsification subsystem absent |
| `FalsificationTests/CancelZeroOverX_NonZeroPoints#0` | FalsificationTests | b | falsification subsystem absent |
| `FalsificationTests/ExpLog_Universal_OnPositivePoints#0` | FalsificationTests | b | falsification subsystem absent |
| `FalsificationTests/LogExp_Real#0` | FalsificationTests | b | falsification subsystem absent |
| `FalsificationTests/SqrtSquare_NonNegative#0` | FalsificationTests | b | falsification subsystem absent |
| `FalsificationTests/SqrtSquare_Real#0` | FalsificationTests | b | falsification subsystem absent |
| `FalsificationTests/TrigPythagorean_Universal#0` | FalsificationTests | b | falsification subsystem absent |
| `GroebnerTests/CircleHyperbola_GrLex#0` | GroebnerTests | b | system-solve nonlinear/contradiction |
| `GroebnerTests/Eliminate_X_FromCircleHyperbola#0` | GroebnerTests | b | system-solve nonlinear/contradiction |
| `GroebnerTests/LexOrder_OfOrderedMonomials#0` | GroebnerTests | b | system-solve nonlinear/contradiction |
| `GroebnerTests/Reduce_NormalForm_IsIdempotent#0` | GroebnerTests | b | system-solve nonlinear/contradiction |
| `GroebnerTests/SinglePolynomial_BasisIsItself#0` | GroebnerTests | b | system-solve nonlinear/contradiction |
| `GroebnerTests/TrivialIdeal_UnitGenerators_ProducesOne#0` | GroebnerTests | b | system-solve nonlinear/contradiction |
| `HyperbolicBuiltinsAndNegativeIntegerPowersTests/BuiltinCount_IncludesTheThreeHyperbolicFunctions#0` | HyperbolicBuiltinsAndNegativeIntegerPowersTests | b | builtin count/metadata |
| `HyperbolicBuiltinsAndNegativeIntegerPowersTests/HyperbolicBuiltins_HaveCompleteDescriptors#0` | HyperbolicBuiltinsAndNegativeIntegerPowersTests | b | help/descriptor metadata |
| `HyperbolicBuiltinsAndNegativeIntegerPowersTests/HyperbolicBuiltins_HaveCompleteDescriptors#1` | HyperbolicBuiltinsAndNegativeIntegerPowersTests | b | help/descriptor metadata |
| `HyperbolicBuiltinsAndNegativeIntegerPowersTests/HyperbolicBuiltins_HaveCompleteDescriptors#2` | HyperbolicBuiltinsAndNegativeIntegerPowersTests | b | help/descriptor metadata |
| `HyperbolicBuiltinsAndNegativeIntegerPowersTests/NegativeSquareRootsAndHalfPowers_NowAnswerExactly_WhileOtherNonIntegerPowersStillThrow#0` | HyperbolicBuiltinsAndNegativeIntegerPowersTests | a | mapped |
| `HyperbolicBuiltinsAndNegativeIntegerPowersTests/SymbolToTheMinusOne_StaysTheSymbolicReciprocalPower#0` | HyperbolicBuiltinsAndNegativeIntegerPowersTests | a | mapped |
| `HyperbolicBuiltinsAndNegativeIntegerPowersTests/ZeroToTheMinusOne_IsATypedRecoverableErrorThatNamesTheBase#0` | HyperbolicBuiltinsAndNegativeIntegerPowersTests | b | zero-base negative power message |
| `IndeterminateExponentialLimitTests/DeterminedLimits_KeepExactlyTheirPreviousAnswers#0` | IndeterminateExponentialLimitTests | b | limit engine incompleteness |
| `IndeterminateExponentialLimitTests/Family_AnswersEveryExactExponent#0` | IndeterminateExponentialLimitTests | b | limit engine incompleteness |
| `IndeterminateExponentialLimitTests/IndeterminateShapes_AreRefused_NeverAnsweredWithTheValueAtThePoint#0` | IndeterminateExponentialLimitTests | b | limit engine incompleteness |
| `IndeterminateExponentialLimitTests/Probe_LimitOfOnePlusOneOverXToTheX_IsE#0` | IndeterminateExponentialLimitTests | b | limit engine incompleteness |
| `IndeterminateExponentialLimitTests/Probe_LimitOfOnePlusTwoOverXToTheX_IsESquared#0` | IndeterminateExponentialLimitTests | b | limit engine incompleteness |
| `IndeterminateExponentialLimitTests/Probe_LimitOfOnePlusXToTheOneOverX_IsE#0` | IndeterminateExponentialLimitTests | b | limit engine incompleteness |
| `IndeterminateExponentialLimitTests/Probes_CrossTheWireAsExactClosedForms#0` | IndeterminateExponentialLimitTests | b | limit engine incompleteness |
| `IndeterminateExponentialLimitTests/SubstitutionShapedLimits_StillAnswerImmediately#0` | IndeterminateExponentialLimitTests | b | limit engine incompleteness |
| `IndeterminateExponentialLimitTests/UndeterminedLimit_ClaimsNoExactness_AndDeterminedOnesKeepTheirs#0` | IndeterminateExponentialLimitTests | a | mapped |
| `InfiniteFamilyCompletenessTests/SympyOracle_ExpOverComplexes_IsAnImageSet_AndTheTemplateMatchesIt#0` | InfiniteFamilyCompletenessTests | c | c |
| `IntegerIntervalContradictionTests/Integer_NonEmptyInterval_IsAccepted#0` | IntegerIntervalContradictionTests | b | assumption/tri-state kernel |
| `IntegerIntervalContradictionTests/Integer_OpenBoundaryLeavesItsNeighbourReachable#0` | IntegerIntervalContradictionTests | b | assumption/tri-state kernel |
| `IntegerIntervalContradictionTests/Integer_StrictFractionalSandwich_IsContradictory#0` | IntegerIntervalContradictionTests | b | assumption/tri-state kernel |
| `IntegerIntervalContradictionTests/Integer_UnitInterval_IsContradictory#0` | IntegerIntervalContradictionTests | b | assumption/tri-state kernel |
| `IntegerIntervalContradictionTests/Reals_WithTheSameFractionalBounds_StaySatisfiable#0` | IntegerIntervalContradictionTests | b | assumption/tri-state kernel |
| `InverseBranchCandidateVerificationTests/EveryRepresentedSolutionSatisfiesTheOriginalEquation#4` | InverseBranchCandidateVerificationTests | b | inverse-branch verification |
| `KernelHardeningTests/AbsSquare_NotSimplified_ForComplex#0` | KernelHardeningTests | a | mapped |
| `KernelHardeningTests/AbsSquare_Simplified_UnderReal#0` | KernelHardeningTests | b | assumption/tri-state kernel |
| `KernelHardeningTests/AddDroppingZeroCoefficientOverPole_KeepsDefinedness#0` | KernelHardeningTests | a | mapped |
| `KernelHardeningTests/CubicRoots_SatisfyPolynomial_Numerically#0` | KernelHardeningTests | b | cubic-root verification |
| `KernelHardeningTests/CubicRoots_SatisfyPolynomial_Numerically#1` | KernelHardeningTests | b | cubic-root verification |
| `KernelHardeningTests/CubicRoots_SatisfyPolynomial_Numerically#2` | KernelHardeningTests | b | cubic-root verification |
| `KernelHardeningTests/CubicRoots_SatisfyPolynomial_Symbolically#0` | KernelHardeningTests | b | cubic-root verification |
| `KernelHardeningTests/CubicRoots_SatisfyPolynomial_Symbolically#1` | KernelHardeningTests | b | cubic-root verification |
| `KernelHardeningTests/Diff_Abs_CarriesNonzeroCondition#0` | KernelHardeningTests | b | abs/floor derivative (piecewise/derivative node) |
| `KernelHardeningTests/Diff_Floor_IsUnevaluatedDerivative_NotZero#0` | KernelHardeningTests | b | abs/floor derivative (piecewise/derivative node) |
| `KernelHardeningTests/Diff_Piecewise_DifferentiatesBranches#0` | KernelHardeningTests | b | piecewise node absent |
| `KernelHardeningTests/DomainPredicates_AreRealGuarded#0` | KernelHardeningTests | b | assumption/tri-state kernel |
| `KernelHardeningTests/EvenOdd_ImplyInteger_OddImpliesNonZero#0` | KernelHardeningTests | b | assumption/tri-state kernel |
| `KernelHardeningTests/ExpLog_SafeSimplify_RequiresDefinednessConditions#0` | KernelHardeningTests | a | mapped |
| `KernelHardeningTests/FreeOf_DoesNotTreatPiecewiseAsConstant#0` | KernelHardeningTests | b | piecewise node absent |
| `KernelHardeningTests/HugeExponent_CompactForm_RoundTrips#0` | KernelHardeningTests | b | RealLiteral / RealConstant carrier |
| `KernelHardeningTests/Integer_DoesNotImply_EvenOddNonZero#0` | KernelHardeningTests | b | assumption/tri-state kernel |
| `KernelHardeningTests/IntervalCondition_ImZero_ForProvablyReal#0` | KernelHardeningTests | b | assumption/tri-state kernel |
| `KernelHardeningTests/Limit_OneOverX_LeftMinusInf#0` | KernelHardeningTests | a | mapped |
| `KernelHardeningTests/Limit_OneOverX_RightPlusInf#0` | KernelHardeningTests | a | mapped |
| `KernelHardeningTests/Limit_OneOverX_TwoSidedDoesNotExist#0` | KernelHardeningTests | a | mapped |
| `KernelHardeningTests/Limit_OneOverXSquared_BothSidesPlusInf#0` | KernelHardeningTests | a | mapped |
| `KernelHardeningTests/Limit_SeriesOffset_OneMinusCosOverX_IsZero#0` | KernelHardeningTests | a | mapped |
| `KernelHardeningTests/LogExp_NotSimplified_Unconstrained#0` | KernelHardeningTests | a | mapped |
| `KernelHardeningTests/LogExp_Simplified_UnderReal#0` | KernelHardeningTests | b | assumption/tri-state kernel |
| `KernelHardeningTests/MathIR_Deserialize_RejectsOldVersion#0` | KernelHardeningTests | b | MathIR deserialize/lowering |
| `KernelHardeningTests/MathIR_HugeExponent_LowersAndEvaluates#0` | KernelHardeningTests | b | MathIR deserialize/lowering |
| `KernelHardeningTests/MinusOne_HugeExponent_ParityIsExact#0` | KernelHardeningTests | a | mapped |
| `KernelHardeningTests/NumericToSymbolicToNumeric_500Digits_RoundTrips#0` | KernelHardeningTests | b | RealLiteral / RealConstant carrier |
| `KernelHardeningTests/Or_ReturnsFalse_WhenAllDisjunctsFalse#0` | KernelHardeningTests | b | assumption/tri-state kernel |
| `KernelHardeningTests/RationalPower_HugeExponent_IsExact#0` | KernelHardeningTests | b | huge negative rational power |
| `KernelHardeningTests/RationalRoots_IncludeZero#0` | KernelHardeningTests | a | mapped |
| `KernelHardeningTests/RealConstantAdd_Preserves500Digits#0` | KernelHardeningTests | b | RealLiteral / RealConstant carrier |
| `KernelHardeningTests/RealConstantMultiply_PreservesPrecision#0` | KernelHardeningTests | b | RealLiteral / RealConstant carrier |
| `KernelHardeningTests/RegisteredFunctionArity_IsEnforced#0` | KernelHardeningTests | b | variadic min arity divergence |
| `KernelHardeningTests/RootOf_CloseRoots_AllFoundAndIndexedStably#0` | KernelHardeningTests | b | RootOf numeric indexing |
| `KernelHardeningTests/RootOf_MultipleRoot_IsSquareFreeNormalized#0` | KernelHardeningTests | a | mapped |
| `KernelHardeningTests/RootOf_NoRealRoots_FailsExplicitly#0` | KernelHardeningTests | b | RootOf numeric indexing |
| `KernelHardeningTests/Simplify_XOverX_CancelsWithCondition#0` | KernelHardeningTests | a | mapped |
| `KernelHardeningTests/Simplify_ZeroOverX_CancelsWithCondition#0` | KernelHardeningTests | b | zero-over-x rewrite rule absent |
| `KernelHardeningTests/Solve_QuarticNoRealRoots_DefaultDomain_Unevaluated_RealDomain_Empty#0` | KernelHardeningTests | b | solve complex algebraic roots |
| `KernelHardeningTests/Solve_RationalEquation_HonorsExcludedDenominator#0` | KernelHardeningTests | a | mapped |
| `KernelHardeningTests/XOverX_IsNotFoldedByConstruction#0` | KernelHardeningTests | a | mapped |
| `KernelHardeningTests/YunSquareFree_MultiplicitiesAreCorrect#0` | KernelHardeningTests | b | polynomial square-free API |
| `KernelHardeningTests/ZeroPowZero_IsOne_DocumentedConvention#0` | KernelHardeningTests | a | mapped |
| `LatexPrinterTests/GroupingParity_LatexAndPrettyHaveTheSameStructuralNesting#0` | LatexPrinterTests | b | pretty/latex grouping |
| `LatexPrinterTests/GroupingParity_LatexAndPrettyHaveTheSameStructuralNesting#15` | LatexPrinterTests | b | pretty/latex grouping |
| `LatexPrinterTests/GroupingParity_LatexAndPrettyHaveTheSameStructuralNesting#16` | LatexPrinterTests | b | pretty/latex grouping |
| `LatexPrinterTests/GroupingParity_LatexAndPrettyHaveTheSameStructuralNesting#17` | LatexPrinterTests | b | pretty/latex grouping |
| `LatexPrinterTests/GroupingParity_LatexAndPrettyHaveTheSameStructuralNesting#18` | LatexPrinterTests | b | pretty/latex grouping |
| `LatexPrinterTests/GroupingParity_LatexAndPrettyHaveTheSameStructuralNesting#2` | LatexPrinterTests | b | pretty/latex grouping |
| `LatexPrinterTests/GroupingParity_LatexAndPrettyHaveTheSameStructuralNesting#42` | LatexPrinterTests | b | pretty/latex grouping |
| `LatexPrinterTests/GroupingParity_LatexAndPrettyHaveTheSameStructuralNesting#43` | LatexPrinterTests | b | pretty/latex grouping |
| `LatexPrinterTests/KnownRendering_PowerOfAComplexConstant_IsAlreadyAGroup#0` | LatexPrinterTests | b | pretty/latex grouping |
| `LatexSymbolNameEscapingTests/LatexSymbolName_InsideLargerExpressions_IsStillEscaped#0` | LatexSymbolNameEscapingTests | b | derivative node absent |
| `LimitExistenceTriStateTests/KernelUndeterminedSide_HasNoValue#0` | LimitExistenceTriStateTests | a | mapped |
| `LinsolveCheckTests/Solve_3x3_VerifiesByNumericSubstitution#0` | LinsolveCheckTests | b | matrix conditions/verification |
| `LinsolveCheckTests/Solve_VerifiesByNumericSubstitution#0` | LinsolveCheckTests | b | matrix conditions/verification |
| `MathIrTypingTests/Deserialize_RejectsMalformedProgram#0` | MathIrTypingTests | b | MathIR deserialize/lowering |
| `MathIrTypingTests/Infer_TypesForLoweredExpressions#0` | MathIrTypingTests | b | MathIR deserialize/lowering |
| `MathIrTypingTests/Validate_AcceptsLoweredCorpus#0` | MathIrTypingTests | b | MathIR deserialize/lowering |
| `MathIrTypingTests/Validate_RejectsNonBoolSelectGuard#0` | MathIrTypingTests | b | MathIR deserialize/lowering |
| `MathIrTypingTests/Validate_RejectsOrderedComparisonOfComplex#0` | MathIrTypingTests | b | MathIR deserialize/lowering |
| `MathIrTypingTests/Validate_RejectsOutOfRangeOperand#0` | MathIrTypingTests | b | MathIR deserialize/lowering |
| `MathIrTypingTests/Validate_RejectsWrongArity#0` | MathIrTypingTests | b | MathIR deserialize/lowering |
| `MatrixAdvanceTests/Inverse_Invertible_BehaviorPreserved#0` | MatrixAdvanceTests | b | matrix conditions/verification |
| `MatrixAdvanceTests/InverseWithConditions_NonSingular_MatchesInverse_And_CarriesCondition#0` | MatrixAdvanceTests | b | matrix conditions/verification |
| `MatrixAdvanceTests/InverseWithConditions_Singular_ReturnsNullMatrix_And_Note#0` | MatrixAdvanceTests | b | matrix conditions/verification |
| `MatrixAdvanceTests/Rank_GenericOverSymbols_And_Numeric#0` | MatrixAdvanceTests | b | matrix conditions/verification |
| `MatrixAdvanceTests/SolveWithConditions_MatchesSolve_And_CarriesCondition#0` | MatrixAdvanceTests | b | matrix conditions/verification |
| `MetamorphicSetClosureTests/Cancel_ExpandsAnUnexpandedPowerOfASumBeforeReducing#0` | MetamorphicSetClosureTests | b | matrix conditions/verification |
| `MetamorphicSetClosureTests/Cancel_PoleExclusionIsCarriedAsANonZeroConditionByTheRewritePath#0` | MetamorphicSetClosureTests | b | matrix conditions/verification |
| `MetamorphicSetClosureTests/Cancel_PreservesValueOffPoles_AndThePoleIsExcluded#0` | MetamorphicSetClosureTests | b | matrix conditions/verification |
| `MetamorphicSetClosureTests/CancelBuiltin_ReducesAndRefusesToCancelWhenNoFactorIsCommon#0` | MetamorphicSetClosureTests | b | matrix conditions/verification |
| `MetamorphicSetClosureTests/CancelFull_ReportsTheConditionsTheBareValueCannotCarry#0` | MetamorphicSetClosureTests | b | matrix conditions/verification |
| `MetamorphicSetClosureTests/CancelWithConditions_BuildsTheSameConditionAtomsAsTheRewritePath#0` | MetamorphicSetClosureTests | b | matrix conditions/verification |
| `MetamorphicSetClosureTests/Inverse_TimesOriginal_IsIdentitySymbolically_UnderDetNonZeroCondition#0` | MetamorphicSetClosureTests | b | matrix conditions/verification |
| `MetamorphicSetClosureTests/InverseIdentity_HoldsOnlyWhereDetIsNonZero#0` | MetamorphicSetClosureTests | b | matrix conditions/verification |
| `MetamorphicSetClosureTests/InverseIdentityCheck_RejectsInversesThatPassTheNumericSpotCheck#0` | MetamorphicSetClosureTests | b | matrix conditions/verification |
| `NegativePowerEvaluationTests/DerivativeOfTanOfXCubed_EvaluatesToTheSymPyValue#0` | NegativePowerEvaluationTests | b | RealLiteral / RealConstant carrier |
| `NegativePowerEvaluationTests/NegativeIntegerPowerOfAReal_IsTheReciprocal#0` | NegativePowerEvaluationTests | b | RealLiteral / RealConstant carrier |
| `NegativePowerEvaluationTests/ReciprocalOfARealJustBelowOne_KeepsItsQuotient#0` | NegativePowerEvaluationTests | b | RealLiteral / RealConstant carrier |
| `NegativePowerEvaluationTests/ReciprocalOfARealJustBelowOne_KeepsItsQuotient#1` | NegativePowerEvaluationTests | b | RealLiteral / RealConstant carrier |
| `NegativePowerEvaluationTests/ReciprocalOfARealJustBelowOne_KeepsItsQuotient#2` | NegativePowerEvaluationTests | b | RealLiteral / RealConstant carrier |
| `NegativePowerEvaluationTests/ReciprocalOfARealJustBelowOne_KeepsItsQuotient#3` | NegativePowerEvaluationTests | b | RealLiteral / RealConstant carrier |
| `NegativePowerEvaluationTests/ReciprocalOfARealJustBelowOne_KeepsItsQuotient#4` | NegativePowerEvaluationTests | b | RealLiteral / RealConstant carrier |
| `PiecewiseAssumptionTests/MirroredSpelling_IsAlsoDecided#0` | PiecewiseAssumptionTests | b | piecewise node absent |
| `PiecewiseAssumptionTests/ProvenGuard_TakesTheBranch#0` | PiecewiseAssumptionTests | b | piecewise node absent |
| `PiecewiseAssumptionTests/RefutedGuard_TakesTheOtherwiseBranch#0` | PiecewiseAssumptionTests | b | piecewise node absent |
| `PiecewiseAssumptionTests/WithoutAssumptions_TheGuardIsStillUndecidable#0` | PiecewiseAssumptionTests | b | piecewise node absent |
| `PolynomialsApiTests/Content_PrimitivePart_SplitCoefficients#0` | PolynomialsApiTests | b | polynomial API surface absent |
| `PolynomialsApiTests/Discriminant_MatchesKnownValues#0` | PolynomialsApiTests | b | polynomial API surface absent |
| `PolynomialsApiTests/Divide_X3Minus1_By_XMinus1#0` | PolynomialsApiTests | b | polynomial API surface absent |
| `PolynomialsApiTests/Factor_QuarticWithRationalRoots_FourLinearFactors#0` | PolynomialsApiTests | b | polynomial API surface absent |
| `PolynomialsApiTests/FromExpr_RoundTrip_DegreeCoefficientTerms#0` | PolynomialsApiTests | b | polynomial API surface absent |
| `PolynomialsApiTests/Gcd_Lcm_AreConsistent#0` | PolynomialsApiTests | b | polynomial API surface absent |
| `PolynomialsApiTests/Resultant_MatchesKnownValues#0` | PolynomialsApiTests | b | polynomial API surface absent |
| `PolynomialsApiTests/SquareFree_X2Times_XMinus1#0` | PolynomialsApiTests | b | polynomial API surface absent |
| `PrettyParenthesesTests/ModelLevel_PowerOfAComplexConstant_HasADelimitedBase#0` | PrettyParenthesesTests | b | pretty/latex grouping |
| `PrettyParenthesesTests/ReportedCase_FactoredSquare_HasOnePair#0` | PrettyParenthesesTests | a | mapped |
| `PrettyParenthesesTests/ReportedCase_FlatDenominators_DivideOncePerFactor#0` | PrettyParenthesesTests | b | pretty/latex grouping |
| `PrettyParenthesesTests/RoundTrip_PrettyRenderingReparsesToTheSameCanonicalForm#21` | PrettyParenthesesTests | b | pretty/latex grouping |
| `PrettyParenthesesTests/RoundTrip_PrettyRenderingReparsesToTheSameCanonicalForm#22` | PrettyParenthesesTests | b | pretty/latex grouping |
| `PrettyParenthesesTests/RoundTrip_PrettyRenderingReparsesToTheSameCanonicalForm#23` | PrettyParenthesesTests | b | pretty/latex grouping |
| `PrettyParenthesesTests/RoundTrip_PrettyRenderingReparsesToTheSameCanonicalForm#24` | PrettyParenthesesTests | b | pretty/latex grouping |
| `PrettyParenthesesTests/RoundTrip_PrettyRenderingReparsesToTheSameCanonicalForm#4` | PrettyParenthesesTests | b | pretty/latex grouping |
| `PrettyParenthesesTests/RoundTrip_PrettyRenderingReparsesToTheSameCanonicalForm#6` | PrettyParenthesesTests | b | pretty/latex grouping |
| `PrintBudgetTests/DepthBudget_ReportsDepthLimit#0` | PrintBudgetTests | b | print/node budget policy |
| `PrintBudgetTests/NodeBudget_OnACompactRendering_DropsNothingAndSaysSo#0` | PrintBudgetTests | b | print/node budget policy |
| `PrintBudgetTokenBoundaryTests/AnIdentifierLongerThanTheAllowance_IsKeptWhole#0` | PrintBudgetTokenBoundaryTests | b | print/node budget policy |
| `PrintBudgetTokenBoundaryTests/ARenderingWithinTheAllowance_IsUntouched#0` | PrintBudgetTokenBoundaryTests | b | print/node budget policy |
| `PropertyTests/CanonicalText_RoundTripsAndReinterns#0` | PropertyTests | b | hash-consing identity |
| `PropertyTests/Diff_MatchesNumericalDerivative#0` | PropertyTests | b | C# System.Random corpus (unreproducible) |
| `PropertyTests/Integrate_DiffRoundTrips#0` | PropertyTests | b | C# System.Random corpus (unreproducible) |
| `PropertyTests/MathIR_EvaluatesLikeSymbolic#0` | PropertyTests | b | MathIR deserialize/lowering |
| `PropertyTests/Optimize_PreservesValues#0` | PropertyTests | b | C# System.Random corpus (unreproducible) |
| `PropertyTests/Simplify_PreservesValues_UnderRealAssumption#0` | PropertyTests | b | C# System.Random corpus (unreproducible) |
| `PropertyTests/Solve_QuadraticRoots_SatisfyPolynomial#0` | PropertyTests | b | C# System.Random corpus (unreproducible) |
| `RelationsAndFamiliesTests/Integrate_StructuredResult_ReportsConditions#0` | RelationsAndFamiliesTests | b | family verification |
| `RelationsAndFamiliesTests/KleeneEvaluation_AndOrNot#0` | RelationsAndFamiliesTests | b | family verification |
| `RelationsAndFamiliesTests/LogicalCanonicalization#0` | RelationsAndFamiliesTests | b | family verification |
| `RelationsAndFamiliesTests/Piecewise_WithCompoundGuard_Evaluates#0` | RelationsAndFamiliesTests | b | family verification |
| `RewriteProofBridgeTests/EveryObligation_IsStatedAndHonestAboutItsStatus#0` | RewriteProofBridgeTests | b | rewrite proof registry absent |
| `RewriteProofBridgeTests/EveryShippedRule_HasExactlyOneProofObligation#0` | RewriteProofBridgeTests | b | rewrite proof registry absent |
| `RewriteProtocolTests/Budget_CountsAppliedRewrites_AndStopsFurtherGroups#0` | RewriteProtocolTests | b | rewrite engine protocol/options |
| `RewriteProtocolTests/ConditionalRule_CarriesDeclaredConditions#0` | RewriteProtocolTests | a | mapped |
| `RewriteProtocolTests/ConditionalRule_DoesNotFireOnUnknown_ButFiresWhenProven#0` | RewriteProtocolTests | a | mapped |
| `RewriteProtocolTests/DomainSpecificRule_DeclaresItsCondition#0` | RewriteProtocolTests | b | rewrite engine protocol/options |
| `RewriteProtocolTests/EveryRule_HasAStableUniqueId_AndUniversalRulesAreUnconditional#0` | RewriteProtocolTests | b | rewrite engine protocol/options |
| `RewriteProtocolTests/RuleEvaluationException_IsTheOnlyCaughtState#0` | RewriteProtocolTests | b | rewrite engine protocol/options |
| `RewriteProtocolTests/ThrowingPrecondition_Propagates#0` | RewriteProtocolTests | b | rewrite engine protocol/options |
| `RewriteProtocolTests/TraceDisabled_KeepsConditionsAndAllocatesNoSteps#0` | RewriteProtocolTests | b | rewrite engine protocol/options |
| `RewriteProtocolTests/UnknownApplicability_NeverFires#0` | RewriteProtocolTests | b | rewrite engine protocol/options |
| `SeriesKinkDeterminismTests/KinkSeries_NeighbouringShapes_FollowTheSameRule#0` | SeriesKinkDeterminismTests | b | series at a kink/pole |
| `SeriesKinkDeterminismTests/KinkSeries_NeighbouringShapes_FollowTheSameRule#1` | SeriesKinkDeterminismTests | b | series at a kink/pole |
| `SeriesKinkDeterminismTests/KinkSeries_NeighbouringShapes_FollowTheSameRule#2` | SeriesKinkDeterminismTests | b | series at a kink/pole |
| `SeriesKinkDeterminismTests/KinkSeries_NeighbouringShapes_FollowTheSameRule#3` | SeriesKinkDeterminismTests | b | series at a kink/pole |
| `SeriesKinkDeterminismTests/KinkSeries_PublishesNoInternalSymbolAndNoUnevaluatedDerivative#0` | SeriesKinkDeterminismTests | b | series truncation node representation |
| `SeriesKinkDeterminismTests/KinkSeries_PublishesNoInternalSymbolAndNoUnevaluatedDerivative#1` | SeriesKinkDeterminismTests | b | series truncation node representation |
| `SeriesKinkDeterminismTests/KinkSeries_PublishesNoInternalSymbolAndNoUnevaluatedDerivative#2` | SeriesKinkDeterminismTests | b | series truncation node representation |
| `SeriesKinkDeterminismTests/KinkSeries_PublishesNoInternalSymbolAndNoUnevaluatedDerivative#3` | SeriesKinkDeterminismTests | b | series truncation node representation |
| `SeriesKinkDeterminismTests/KinkSeries_PublishesNoInternalSymbolAndNoUnevaluatedDerivative#4` | SeriesKinkDeterminismTests | b | series truncation node representation |
| `SeriesKinkDeterminismTests/KinkSeries_PublishesNoInternalSymbolAndNoUnevaluatedDerivative#5` | SeriesKinkDeterminismTests | b | series truncation node representation |
| `SeriesKinkDeterminismTests/KinkSeries_PublishesNoInternalSymbolAndNoUnevaluatedDerivative#6` | SeriesKinkDeterminismTests | b | series truncation node representation |
| `SeriesKinkDeterminismTests/KinkSeries_PublishesNoInternalSymbolAndNoUnevaluatedDerivative#7` | SeriesKinkDeterminismTests | b | series truncation node representation |
| `SeriesKinkDeterminismTests/KinkSeries_PublishesTheSymPyExpansion_ByteIdenticallyOnEveryRun#0` | SeriesKinkDeterminismTests | b | series truncation node representation |
| `SeriesKinkDeterminismTests/KinkSeries_PublishesTheSymPyExpansion_ByteIdenticallyOnEveryRun#1` | SeriesKinkDeterminismTests | b | series truncation node representation |
| `SeriesKinkDeterminismTests/KinkSeries_PublishesTheSymPyExpansion_ByteIdenticallyOnEveryRun#2` | SeriesKinkDeterminismTests | b | series truncation node representation |
| `SeriesKinkDeterminismTests/KinkSeries_PublishesTheSymPyExpansion_ByteIdenticallyOnEveryRun#3` | SeriesKinkDeterminismTests | b | series truncation node representation |
| `SeriesKinkDeterminismTests/KinkSeries_PublishesTheSymPyExpansion_ByteIdenticallyOnEveryRun#4` | SeriesKinkDeterminismTests | b | series truncation node representation |
| `SeriesKinkDeterminismTests/KinkSeries_PublishesTheSymPyExpansion_ByteIdenticallyOnEveryRun#5` | SeriesKinkDeterminismTests | b | series truncation node representation |
| `SeriesKinkDeterminismTests/KinkSeries_PublishesTheSymPyExpansion_ByteIdenticallyOnEveryRun#6` | SeriesKinkDeterminismTests | b | series truncation node representation |
| `SeriesKinkDeterminismTests/KinkSeries_PublishesTheSymPyExpansion_ByteIdenticallyOnEveryRun#7` | SeriesKinkDeterminismTests | b | series truncation node representation |
| `SeriesKinkDeterminismTests/KinkSeries_TakesTheSymPyValueAtTheSamplePoint#0` | SeriesKinkDeterminismTests | b | series at a kink/pole |
| `SeriesKinkDeterminismTests/KinkSeries_TakesTheSymPyValueAtTheSamplePoint#1` | SeriesKinkDeterminismTests | b | series at a kink/pole |
| `SeriesKinkDeterminismTests/KinkSeries_TakesTheSymPyValueAtTheSamplePoint#2` | SeriesKinkDeterminismTests | b | series at a kink/pole |
| `SeriesKinkDeterminismTests/KinkSeries_TakesTheSymPyValueAtTheSamplePoint#3` | SeriesKinkDeterminismTests | b | series at a kink/pole |
| `SeriesKinkDeterminismTests/KinkSeries_TakesTheSymPyValueAtTheSamplePoint#4` | SeriesKinkDeterminismTests | b | series at a kink/pole |
| `SeriesKinkDeterminismTests/KinkSeries_TakesTheSymPyValueAtTheSamplePoint#5` | SeriesKinkDeterminismTests | b | series at a kink/pole |
| `SeriesKinkDeterminismTests/KinkSeries_TakesTheSymPyValueAtTheSamplePoint#6` | SeriesKinkDeterminismTests | b | series at a kink/pole |
| `SeriesKinkDeterminismTests/KinkSeries_TakesTheSymPyValueAtTheSamplePoint#7` | SeriesKinkDeterminismTests | a | mapped |
| `SeriesKinkDeterminismTests/KinkSeries_TruncationGap_IsTheDroppedTerm_NotASilentApproximation#0` | SeriesKinkDeterminismTests | b | series at a kink/pole |
| `SeriesKinkDeterminismTests/LimitEngine_TwoSidedKinkAnswers_AreUnchanged#0` | SeriesKinkDeterminismTests | b | limit at a kink |
| `SeriesKinkDeterminismTests/LimitEngine_TwoSidedKinkAnswers_AreUnchanged#1` | SeriesKinkDeterminismTests | a | mapped |
| `SeriesKinkDeterminismTests/LimitEngine_TwoSidedKinkAnswers_AreUnchanged#2` | SeriesKinkDeterminismTests | b | limit at a kink |
| `SeriesKinkDeterminismTests/LimitEngine_TwoSidedKinkAnswers_AreUnchanged#3` | SeriesKinkDeterminismTests | a | mapped |
| `SeriesLaurentAndBranchPointTests/Series_AcrossAPoleOrABranchPoint_PublishesTheExpansion#0` | SeriesLaurentAndBranchPointTests | b | series at a kink/pole |
| `SeriesLaurentAndBranchPointTests/Series_AcrossAPoleOrABranchPoint_PublishesTheExpansion#1` | SeriesLaurentAndBranchPointTests | b | series at a kink/pole |
| `SeriesLaurentAndBranchPointTests/Series_AcrossAPoleOrABranchPoint_PublishesTheExpansion#2` | SeriesLaurentAndBranchPointTests | b | series at a kink/pole |
| `SeriesLaurentAndBranchPointTests/Series_AcrossAPoleOrABranchPoint_PublishesTheExpansion#3` | SeriesLaurentAndBranchPointTests | a | mapped |
| `SeriesLaurentAndBranchPointTests/Series_AcrossAPoleOrABranchPoint_PublishesTheExpansion#4` | SeriesLaurentAndBranchPointTests | b | series at a kink/pole |
| `SeriesLaurentAndBranchPointTests/Series_OfARemovableDenominator_IsUnchanged#0` | SeriesLaurentAndBranchPointTests | b | series at a kink/pole |
| `SolveCompletenessMappingTests/BudgetExceeded_IsIncomplete_WithTheKernelCompletenessOfThePartialSubset#0` | SolveCompletenessMappingTests | b | solve completeness mapping |
| `SolveCompletenessMappingTests/EveryStatusCarriesTheDocumentedPair#3` | SolveCompletenessMappingTests | b | solve completeness mapping |
| `SolveCompletenessMappingTests/LanguageStatuses_ReportExactlyWhatTheMappingReturns#5` | SolveCompletenessMappingTests | b | solve completeness mapping |
| `SolveCompletenessPairingTests/EverySolveStatus_PublishesTheDocumentedPair_OnBothRecords#0` | SolveCompletenessPairingTests | b | solve completeness mapping |
| `SolveCompletenessPairingTests/SystemSolve_NoSolutions_IsACompleteAnswer#0` | SolveCompletenessPairingTests | b | solve completeness mapping |
| `SolveCompletenessPairingTests/Unevaluated_IsNotComplete#0` | SolveCompletenessPairingTests | b | solve completeness mapping |
| `SuiteIntegrationTests/Symbol_Builtin_Works#0` | SuiteIntegrationTests | a | mapped |
| `SuiteIntegrationTests2/Apart_ThroughEngine#0` | SuiteIntegrationTests2 | a | mapped |
| `SystemSolveProofTests/Kernel_AbsenceOfSolutions_IsUnevaluatedWithAStableNote#0` | SystemSolveProofTests | b | system-solve nonlinear/contradiction |
| `SystemSolveProofTests/Kernel_DerivedContradiction_IsNoSolutionsWithoutANote#0` | SystemSolveProofTests | b | system-solve nonlinear/contradiction |
| `SystemSolveProofTests/Kernel_EmptyEquationList_IsATypedRefusal#0` | SystemSolveProofTests | b | system-solve nonlinear/contradiction |
| `SystemSolveProofTests/NonlinearElimination_AndTheBoundary_AgreeWithSympySolve#0` | SystemSolveProofTests | c | c |
| `SystemSolveProofTests/NonlinearElimination_FindsBothSolutionsOfCircleAndLine#0` | SystemSolveProofTests | b | system-solve nonlinear/contradiction |
| `SystemSolveProofTests/NonlinearElimination_FindsBothSolutionsOfParabolaAndLine#0` | SystemSolveProofTests | b | system-solve nonlinear/contradiction |
| `SystemSolveProofTests/Wire_AbsenceOfSolutions_IsUnevaluatedWithADiagnostic#0` | SystemSolveProofTests | a | mapped |
| `SystemSolveProofTests/Wire_AbsenceOfSolutions_IsUnevaluatedWithADiagnostic#1` | SystemSolveProofTests | a | mapped |
| `SystemSolveProofTests/Wire_AbsenceOfSolutions_IsUnevaluatedWithADiagnostic#2` | SystemSolveProofTests | a | mapped |
| `SystemSolveProofTests/Wire_DerivedContradiction_IsCompleteNoSolutions#0` | SystemSolveProofTests | b | system-solve nonlinear/contradiction |
| `SystemSolveProofTests/Wire_DerivedContradiction_IsCompleteNoSolutions#1` | SystemSolveProofTests | b | system-solve nonlinear/contradiction |
| `SystemSolveProofTests/Wire_DerivedContradiction_IsCompleteNoSolutions#2` | SystemSolveProofTests | b | system-solve nonlinear/contradiction |
| `SystemSolveProofTests/Wire_EmptyEquationList_IsARecord_NotAClrException#0` | SystemSolveProofTests | b | system-solve nonlinear/contradiction |
| `SystemSolveProofTests/Wire_NonlinearShapes_PublishTheirTwoSolutions#0` | SystemSolveProofTests | b | system-solve nonlinear/contradiction |
| `SystemSolveProofTests/Wire_NonlinearShapes_PublishTheirTwoSolutions#1` | SystemSolveProofTests | b | system-solve nonlinear/contradiction |
| `SystemSolveTests/CircleHyperbola_FourSolutions#0` | SystemSolveTests | b | system-solve nonlinear/contradiction |
| `SystemSolveTests/Inconsistent_NoSolutions#0` | SystemSolveTests | b | system-solve nonlinear/contradiction |
| `SystemSolveTests/LinearSystem_SingleSolution#0` | SystemSolveTests | a | mapped |
| `SystemSolveTests/NonPolynomial_ReportsNote#0` | SystemSolveTests | b | system-solve nonlinear/contradiction |
| `SystemSolveTests/ParabolaLine_TwoSolutions#0` | SystemSolveTests | b | system-solve nonlinear/contradiction |
| `TruncatingEvaluationStaysInexactTests/ExactControls_StayExact#0` | TruncatingEvaluationStaysInexactTests | b | RealLiteral / RealConstant carrier |
| `TruncatingEvaluationStaysInexactTests/FunctionOfATruncatedArgument_IsInexact#0` | TruncatingEvaluationStaysInexactTests | b | RealLiteral / RealConstant carrier |
| `TruncatingEvaluationStaysInexactTests/FunctionOfATruncatedValue_PublishesExactFalse_AndNoRationalForm#0` | TruncatingEvaluationStaysInexactTests | b | RealLiteral / RealConstant carrier |
| `TruncatingEvaluationStaysInexactTests/FunctionOfATruncatedValue_PublishesExactFalse_AndNoRationalForm#1` | TruncatingEvaluationStaysInexactTests | b | RealLiteral / RealConstant carrier |
| `TruncatingEvaluationStaysInexactTests/FunctionOfATruncatedValue_PublishesExactFalse_AndNoRationalForm#11` | TruncatingEvaluationStaysInexactTests | b | RealLiteral / RealConstant carrier |
| `TruncatingEvaluationStaysInexactTests/LiteralRoundTrip_KeepsGenuinelyExactValuesExact#0` | TruncatingEvaluationStaysInexactTests | b | RealLiteral / RealConstant carrier |
| `TruncatingEvaluationStaysInexactTests/LiteralRoundTrip_PreservesBothTheDigitsAndTheRoute#0` | TruncatingEvaluationStaysInexactTests | b | RealLiteral / RealConstant carrier |
| `TruncatingEvaluationStaysInexactTests/SymbolicTreeFlag_AndNumericReadBack_Agree#0` | TruncatingEvaluationStaysInexactTests | b | RealLiteral / RealConstant carrier |
| `TruncatingEvaluationStaysInexactTests/SymbolicTreeFlag_AndNumericReadBack_Agree#1` | TruncatingEvaluationStaysInexactTests | b | RealLiteral / RealConstant carrier |
| `TruncatingEvaluationStaysInexactTests/SymbolicTreeFlag_AndNumericReadBack_Agree#2` | TruncatingEvaluationStaysInexactTests | b | RealLiteral / RealConstant carrier |
| `TruncatingEvaluationStaysInexactTests/SymbolicTreeFlag_AndNumericReadBack_Agree#3` | TruncatingEvaluationStaysInexactTests | b | RealLiteral / RealConstant carrier |
| `UnsatisfiablePropagationTests/ConditionPayload_IsStructuredForEveryReachableAtomKind#0` | UnsatisfiablePropagationTests | a | mapped |
| `UnsatisfiablePropagationTests/SimplifyFull_DomainCondition_CarriesADomainValueNotText#0` | UnsatisfiablePropagationTests | b | unsatisfiable propagation |
| `UnsatisfiablePropagationTests/SimplifyFull_UnsatisfiableConditions_SerializesAsAnExplicitMarker#0` | UnsatisfiablePropagationTests | b | unsatisfiable propagation |
| `UnsatisfiablePropagationTests/Transform_RequirementConsistentWithAssumptions_KeepsItsConditions#0` | UnsatisfiablePropagationTests | b | unsatisfiable propagation |
| `UnsatisfiablePropagationTests/Transform_RequirementRefutedByAssumptions_IsUnsatisfiable#0` | UnsatisfiablePropagationTests | b | unsatisfiable propagation |
| `UsageDocumentationTests/DocumentedExample_MatchesEngine#61` | UsageDocumentationTests | b | documentation fixture / truncated census display |
| `UsageDocumentationTests/DocumentedExample_MatchesEngine#70` | UsageDocumentationTests | b | documentation fixture / truncated census display |
| `UsageDocumentationTests/DocumentedExample_MatchesEngine#71` | UsageDocumentationTests | b | documentation fixture / truncated census display |
| `UsageDocumentationTests/DocumentedExample_MatchesEngine#73` | UsageDocumentationTests | b | documentation fixture / truncated census display |
| `UsageDocumentationTests/DocumentedExample_MatchesEngine#75` | UsageDocumentationTests | b | documentation fixture / truncated census display |
| `UsageDocumentationTests/DocumentedExample_MatchesEngine#99` | UsageDocumentationTests | b | documentation fixture / truncated census display |
| `UsageExamples/Snippet_16_2_EqualityAndHashConsing#0` | UsageExamples | b | documentation fixture / truncated census display |
| `UsageExamples/Snippet_16_3_AssumptionsDriveSimplification#0` | UsageExamples | b | documentation fixture / truncated census display |
| `UsageExamples/Snippet_16_4_CanonicalTextRoundTrip#0` | UsageExamples | b | documentation fixture / truncated census display |
| `UsageExamples/Snippet_16_5_NumericEvaluationAtPrecision#0` | UsageExamples | b | documentation fixture / truncated census display |
| `UsageExamples/Snippet_16_6_MathIrLowerAndEvaluate#0` | UsageExamples | b | documentation fixture / truncated census display |
| `UsageExamples/Snippet_16_7_SymbolicMatrices#0` | UsageExamples | b | documentation fixture / truncated census display |
| `UsageExamples/Snippet_16_8_ExactnessDiscipline#0` | UsageExamples | b | documentation fixture / truncated census display |
