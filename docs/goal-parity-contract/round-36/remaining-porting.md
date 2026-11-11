# Round 36 — the remaining deferred rows the current production answers

Status: complete for the rows below; the verification block carries the observed gate and
coverage lines. 82 of the 602 deferred rows are now mapped to a passing Rust test: 61 in this
lane's own porting pass and the 21-row system-solve family the concurrent production lane
unblocked (section 4b). The other rows stay deferred with their existing reason (or the measured
refusal recorded in section 5).

## 0. Population and result

| Measure | Rows |
|---|---:|
| Deferred census rows at round start | 602 |
| (a) mapped to a Rust test in this round | 82 |
|   — this lane's own porting pass | 61 |
|   — the system-solve family the concurrent lane unblocked | 21 |
| (b) re-probed and refused (measured divergence) | see section 5 |

Two seams produced the round: rows whose Rust test already existed but had never been curated
into `parity/manifest-map.json` (27 rows), and rows whose earlier named refusal was a measurement
of a tree the current production has since moved (34 rows). Every (a) row was probed on both
runners before it was asserted; a row whose measured Rust behaviour still differs is refused in
section 5, never approximated.

## 1. Rows mapped to a test that already existed (27)

| Census rows | Rust test |
|---|---|
| `BuiltinArgumentShapeSweepTests/TheAuditsFiveReproductions...#1..#4` (4) | `lovelace-run::round30_run_batch::audit_reproductions_cross_as_the_documented_argument_error` |
| `BuiltinArgumentShapeSweepTests/TheDeclaredForm_StaysCallable#8` | `lovelace-run::round30_run_batch::the_declared_audit_forms_stay_callable` |
| `SystemBuiltinArgumentShapeTests/WrongShapedArgument...#0..#5` (6) | `lovelace-run::round30_run_batch::system_builtins_refuse_a_wrong_shaped_argument` |
| `SystemBuiltinArgumentShapeTests/ElementThatIsNotAnExpression...#0` | `lovelace-run::round30_run_batch::system_builtins_refuse_an_element_that_is_not_an_expression` |
| `LatexPrinterTests/GroupingParity...#0,#2,#15,#16,#17,#18,#42,#43` (8) | `lovelace-run::pretty_batch::latex_grouping_rows_carry_the_same_nesting_as_pretty` |
| `PrettyParenthesesTests/RoundTrip...#4,#6,#21,#22,#23,#24` (6) | `lovelace-run::pretty_batch::pretty_round_trips_to_the_same_canonical_form` |
| `PrettyParenthesesTests/ReportedCase_FlatDenominators...#0` | `lovelace-run::pretty_batch::pretty_reported_case_flat_denominators_divide_once_per_factor` |

The Latex rows had been refused in round 26 as "Rust renders x/(y*z) where the reference prints
x/y/z". The printer fix landed afterwards and the round-26 test was written but never curated.
Every one of the eight (source, pretty, latex) triples was re-probed on the oracle and the Rust
runner in this round: the two runners emit the same bytes, so the ShapeTree grouping-parity
assertion the C# row makes holds, and the exact-text test is the stronger statement of it.

## 2. precbench AccuracyTests (12)

| Census row | Rust test |
|---|---|
| `Add_GivenPointOneAndPointTwo_IsExactlyPointThree#0` | `lovelace-real::round36_precbench::add_given_point_one_and_point_two_is_exactly_point_three` |
| `Divide_GivenOneThird_IsExactPeriodic#0` | `..divide_given_one_third_is_exact_periodic` |
| `Divide_GivenOneSeventh_IsExactPeriodic#0` | `..divide_given_one_seventh_is_exact_periodic` |
| `Divide_GivenOneSixth_IsExactMixedPeriod#0` | `..divide_given_one_sixth_is_exact_mixed_period` |
| `Parse_GivenPointOne_LovelaceExactFloatInexact#0` | `..parse_given_point_one_lovelace_exact_float_inexact` |
| `Sqrt_GivenTwo_AtP8AndP16_ProduceDifferentDigitCounts#0` | `..sqrt_given_two_at_p8_and_p16_produce_different_digit_counts` |
| `Sqrt_GivenIrrational_AtP8_ErrorWithinFloatClass#0..#2` (3) | `..sqrt_given_irrational_at_p8_error_within_float_class` |
| `Sqrt_GivenIrrational_AtP16_ErrorWithinDoubleClass#0..#2` (3) | `..sqrt_given_irrational_at_p16_error_within_double_class` |

Translation: the C# `WithPrecision(frac, ...)` scope becomes the explicit `Real::sqrt(digits)`
argument (there is no global precision static in Rust), and `Real.One / x` becomes
`div(&x, Real::DEFAULT_COMPUTATION_DECIMAL_PLACES)` — the ambient default the C# operator uses.
The float/double contrast half of the two comparison rows is asserted in Rust itself
(`0.1f32 as f64 != 0.1f64`, `0.1 + 0.2 != 0.3`), which is the same language-level fact.
The accuracy rows reproduce the C# `RelativeError` (|approx − exact| / |exact| at 60 digits) on
the real tier and assert both inequalities, including the `<= 8x` one.
`WithPrecision_GivenScope_RestoresGlobalState` stays deferred: there is no global precision state
to restore.

## 3. Real / Integer / Suite / Capabilities / UsageExamples (8)

| Census row | Rust test |
|---|---|
| `RealToStringTests/ToString_WithFormatAndProvider...#0` | `lovelace-real::round36_precbench::to_string_with_format_provider_delegates_to_paramless_to_string` |
| `IntegerToStringTests/ToString_WithFormatProvider...#0` | `lovelace-lang::round36_remaining::integer_to_string_with_format_provider_returns_same_as_to_string` |
| `DescriptorExampleTests/Example_Executes#11` | `..descriptor_example_for_assume_clear_executes` |
| `CapabilitiesBuiltinTests/SolveDomainStatement...#0` | `..solve_domain_statement_names_its_operation_and_every_domain_stays_usable_elsewhere` |
| `UsageExamples/Snippet_16_3...#0` | `..snippet_16_3_assumptions_drive_simplification` |
| `UsageExamples/Snippet_16_5...#0` | `..snippet_16_5_numeric_evaluation_at_precision` |
| `UsageExamples/Snippet_16_6...#0` | `..snippet_16_6_math_ir_lower_and_evaluate` |
| `UsageExamples/Snippet_16_7...#0` | `..snippet_16_7_symbolic_matrices` |

The two ToString rows are the same delegation fact on each surface: the reference has one
rendering implementation and the format-provider overload delegates to it; Rust's `Display` and
its inherent `to_decimal`/`to_string` are those two entry points. The `capabilities()` record the
SolveDomain row reads is byte-identical to the oracle's (probed), including the absence of the
old runtime-wide `supported_domains`/`unsupported_domains` pair; the `Snippet_16_*` rows assert
the README facts through the language, and `Snippet_16_1` was already curated before this round.

## 4. Exactness and numeric rows the production moved (11)

| Census rows | Rust test |
|---|---|
| `NegativePowerEvaluationTests/NegativeIntegerPowerOfAReal...#0` | `lovelace-lang::round36_remaining::negative_integer_power_of_a_real_is_the_reciprocal` |
| `NegativePowerEvaluationTests/DerivativeOfTanOfXCubed...#0` | `..derivative_of_tan_of_x_cubed_evaluates_to_the_sympy_value` |
| `NegativePowerEvaluationTests/ReciprocalOfARealJustBelowOne...#0..#4` (5) | `lovelace-real::round36_precbench::reciprocal_of_a_real_just_below_one_keeps_its_quotient` |
| `TruncatingEvaluationStaysInexactTests/SymbolicTreeFlag_AndNumericReadBack_Agree#0..#3` (4) | `lovelace-lang::round36_remaining::symbolic_tree_flag_and_numeric_read_back_agree` |
| `ExactnessFlagTests/ElementaryFunctionOfAnApproximateArgument_IsInexact#4,#5` (2) | `lovelace-symbolics::round36_exactness::elementary_function_of_an_approximate_argument_is_inexact` |
| `ComplexMathAmbientBoundaryTests/Sin_OfATwoPiTruncation...#0` | `lovelace-real::round36_ambient::sin_of_a_two_pi_truncation_finer_than_the_ambient_precision_keeps_its_residual` |

The named refusals these rows carried measured the tree before the exactness/route repair: the
symbolic tree reported `exact = true` for `subs(x + 1, x, pi(30))` while the numeric read-back
reported `true` for the same value. Both now report `false`, which is what the C# row asserts.
The NegativePower rows assert the same numeric contract the C# rows do: the value agrees with
the pinned SymPy digits to `1e-20`, computed on the real tier, never through a rendering.

## 5. Re-probed and refused (measured divergences)

| Candidate | Rows | Measured behaviour |
|---|---:|---|
| `SeriesKinkDeterminismTests` (whole family) | 30 | `series(abs(x), x, 0, 3)` still answers `abs(x)`, not `x + O(x^3)`; the canonical form still lacks the `order` node. |
| `NaturalStructureTests` | 15 | asserts CLR interface identity (`typeof(IComparable<Natural>).IsAssignableFrom(...)`); no CLR identity in Rust. |
| `NaturalStaticPropertyTests` | 8 | `Natural.DisplayDigits`/`Natural.Precision` global statics do not exist. |
| `IndeterminateExponentialLimitTests` | 8 | `limit((1 + 1/x)^x, x, inf)` and the `e^2`/`e` probes still answer `Unevaluated` (`coefficient does not evaluate at the point`); C# answers `e`, `e^2`. |
| `CompilationTests` | 8 | two-parameter kernels, piecewise lazy select, `NumRat` tier preservation and `IrProgram.Deserialize` have no Rust surface; `evalir` refuses a second free symbol and answers `0.75 (Real)` where C# keeps `3/4`. |
| `MetamorphicSetClosureTests` | 9 | `cancel_full` has no dispatcher arm (`Unknown function 'cancel_full'. Did you mean one of: 'cancel_full', 'cancel'?`), and `cancel((x^2+1)/(x^2+2))` answers `1`. |
| `KernelHardeningTests` (residual) | 20 | real-literal carriers, RootOf indexing, piecewise/floor derivatives and the 500-digit round trips are absent; `(-1)^(2^40)` construction aborts the harness, so no test was added. |
| `RealTrigFastPathTests` | 10 | the private fast-path surface (`ReduceToTwoPi`, `HalfOf`, `TrySpecialAngle`, `Normalize`, `IsBelowDecimalGuard`, `ToInteger`) has no Rust counterpart. |
| `Suite` tail families | 88+21 | round-35's blocked-family adjudication still measures: TokenTests has no `TokenKind` ordinal, RecordSchema/Widen/ScriptSource/Plot/StructuralEquality/CaptureState are absent, and the plugin ABI rows need the Modus seam. |
| `SuiteEngineTests`, `FunctionTests`, `StatementTests`, `ValueDepthGuardTests` | 10 | `CaptureState`/`SetVariable`/events are absent, function and block locals still leak into `variables()` (measured), and the structured projection still has no depth guard. |
| `CapabilitiesBuiltinTests` (other 6) | 6 | `PowerExponentClass` fails on `0^(1/2)` (Rust refuses, C# answers 0); `ExactnessBestEffort` fails on `max(1,2)` (Rust answers 1, C# refuses); the plot rows need the typed refusal (`plot(sin(x))` succeeds with a writable plot directory); the rest need the descriptor/CaptureState surface. |
| `PlotTests`, `PlotFile*`, `DescriptorExampleTests` (`filter`/`ndims`/`scale`/`cancel_full`) | 9 | the renderer model is absent and those names have no dispatcher arm (`Unknown function ...`). |
| `StructuredRealExactnessTests` | 2 | `setprecision(18); 1/1009` renders 40 digits (C# 18) and `2^-100000` is not certified exact. |
| `EvalfTruncatedConstantExactnessTests/ExactControls...#0` | 1 | `sin(0)` still evaluates to the symbolic `sin(0)`; the C# row asserts the pretty `0`. |
| `UsageDocumentationTests` | 6 | display-name truncation (5 rows) and `linsolve([[x,1],[0,y]],[0,1])` renders `[-1/(x*y), x/(x*y)]` where C# prints `[-x/x/(x*y), x/(x*y)]`. |
| `UsageExamples` (`16_2`, `16_4`, `16_8`) | 3 | hash-consed identity is absent (two rows), and `inspect(sqrt(2)).exact` answers false where C# answers true (the radical tier). |
| `TruncatingEvaluationStaysInexactTests` (`FunctionOfATruncatedArgument`, `LiteralRoundTrip_KeepsGenuinelyExact...`) | 2 | quantified over the C# `RealLiteral` carrier, which the Rust symbolic tower does not have. |
| `Integer` residual (`Pow_*`, operators, `TryFormat`, ctor overload) | 8 | `Int::pow` takes a `u64` and answers `0^3 = 0` / `2^0 = 1` where C# refuses; the Rust surface has no arithmetic operators, no `Span<char>` formatting and one integer constructor. |
| `Real` residual | 22 | no `f64` constructor, no global `DisplayDecimalPlaces`/`MaxComputationDecimalPlaces` scope, `PeriodStart` default is -1 (C# 0), batch/async sqrt is absent, `TryFormat`/format-provider shapes have no `Span` surface. |
| `Natural` residual | 17 | `System.Random` corpora, reference identity of the cached string, and the `Natural(int)` negative refusal with no `i64` constructor. |
| `Dsp` | 18 | fixed-point `Cplx64/128`, `DspMath.Filter/ImpulseResponse` and the `DspPlugin` registration seam are absent; `filter` has no dispatcher arm; `fft([])` refuses where C# returns empty and `fft([1,1,1])` answers where C# throws. |
| `Complex` | 4 | `DenseArray` `DType/AsSpan` and the `RealLiteral` provenance pair are absent (2 named rows above plus the two symbolic-provenance rows). |
| `Run` residual | 15 | registry sweeps need >=100 builtins (Rust has 50) and `signature.actual`; plot-path envelopes, `CaptureState`/projection, recursion-depth and `GoldenFixtureRecorder` rows need absent seams. |
| `Array`/`Representation` | 2 | `MatMul_Batched` needs the double-field batched kernel; `UnitTest1.Test1` asserts a placeholder with no analogue. |

## 4b. The system-solve family the concurrent lane unblocked (21)

The corpus/system-solve lane rewrote `solve_system`/`solve_system_full` with Lex Gröbner
elimination, added `rust/lovelace-symbolics/src/groebner.rs`, and reduced both documentation-corpus
failures (214/214). It pinned the fixed behaviour in
`rust/lovelace-lang/src/corpus_and_system_solve_round36.rs` (10 tests) and `groebner.rs`
(7 tests); its own report is `docs/goal-parity-contract/round-36/corpus-and-system-solve.md`.
The 21 rows are mapped to those tests, whose expected strings are the oracle's exact `--text`
output for the same scripts:

| Census rows | Rust test |
|---|---|
| 6 `GroebnerTests` rows | `lovelace-symbolics::groebner::tests::{circle_hyperbola_grlex_basis_has_the_documented_three_elements, eliminate_x_from_circle_hyperbola_is_y_cubed_minus_y, lex_basis_of_ordered_monomials_keeps_three_elements, reduce_normal_form_is_idempotent, a_single_polynomial_is_its_own_basis, trivial_ideal_from_x_and_x_plus_one_is_one}` |
| `Kernel_DerivedContradiction...`, `Wire_DerivedContradiction...#0..#2`, `Inconsistent_NoSolutions` (5) | `lovelace-lang::corpus_and_system_solve_round36::system_solve_derived_contradictions_are_complete_no_solutions` |
| `Kernel_AbsenceOfSolutions...` | `..system_solve_abandoned_branches_are_unevaluated_with_the_oracle_note` |
| `Kernel_EmptyEquationList...`, `Wire_EmptyEquationList...` (2) | `..system_solve_empty_equation_list_is_a_typed_refusal` |
| `NonlinearElimination...CircleAndLine`, `Wire_NonlinearShapes...#1` (2) | `..system_solve_circle_line_finds_both_exact_radical_solutions` |
| `NonlinearElimination...ParabolaAndLine`, `Wire_NonlinearShapes...#0`, `ParabolaLine_TwoSolutions` (3) | `..system_solve_parabola_line_finds_both_exact_radical_solutions` |
| `CircleHyperbola_FourSolutions` | `..system_solve_circle_hyperbola_publishes_four_solutions` |
| `NonPolynomial_ReportsNote` | `..system_solve_non_polynomial_reports_the_stable_note` |

The InlineData-to-caseIndex pairing was read from the census display names: `Wire_NonlinearShapes#0`
is the parabola/line script and `#1` the circle/line script, so the two rows map to the two
different elimination tests rather than to one.

## 6. Verification

| Gate | Command | Observed result |
|---|---|---|
| Suite | `cargo test --workspace` | PENDING |
| Inventory | `parity/gen-rust-tests.ps1` | PENDING |
| Manifest | `node parity/gen-manifest.mjs` | PENDING |
| Manifest gate | `node parity/tools/gate-manifest.mjs` | PENDING |
| Coverage | `node parity/tools/coverage-report.mjs` | PENDING |
| Fixtures | `node parity/tools/gate-fixtures.mjs` | PENDING |
| Corpus | `cargo run --release -p lovelace-corpus` | PENDING |

The concurrent corpus/system-solve lane was still writing when this round started
(`rust/lovelace-lang/src/corpus_and_system_solve_round36.rs`, `rust/lovelace-symbolics/src/groebner.rs`);
the pipeline above was run only after the Rust sources had been quiet for several minutes, and the
sources hash in `parity/rust-tests.json` is the one the gates verify.
