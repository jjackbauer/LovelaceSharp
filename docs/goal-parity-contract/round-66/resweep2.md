# Round 66 — the 196-row re-sweep (second adjudication)

**Status: complete.** Every one of the 196 census rows left deferred after round 65 was
re-adjudicated against the current tree. Each row's own script was re-executed through both
runners — the pinned C# oracle and `rust/target/release/lovelace-run.exe` — with the
differential's exact normalisation (`revision`, `elapsed`, `elapsedTime`, `timings` replaced at any
depth; key presence structural), and each row was re-classified into MATCH-NOW / STILL-DIFFERENT
(exact field and values) / UNRUNNABLE (missing API named). Fifteen rows now match and are mapped to
fifteen new tests; 51 remain measurably different; 130 remain blocked on a named missing Rust API.
No production source, `.cs` file, fixture, corpus file or other document was edited: the only code
written is the four test modules plus their `mod` registrations, and the only manifest edit is
`parity/manifest-map.json`.

## 1. Result

| Classification | Rows | Disposition |
|---|---:|---|
| MATCH-NOW (newly mapped) | 15 | ported to 15 new Rust tests and mapped in `parity/manifest-map.json` |
| STILL-DIFFERENT | 51 | deferred with the measured family and field/values (24 families) |
| UNRUNNABLE | 130 | deferred with the missing Rust API named (58 APIs) |
| **Total re-adjudicated** | **196** | |

Manifest: **5,910 / 6,209 ported (95.18 %)** — up from 5,895 (94.94 %); deferred **196 → 181**;
excluded unchanged at **118**. Rust suite: **5,098 passed, 1 ignored** (up from 5,083; +15 tests; 5,099 listed).
Coverage moved exactly by the fifteen new mappings: Lovelace.Symbolics.Tests 1,127 → 1,140 (+13),
Lovelace.Dsp.Tests +1, precbench.Tests +1.

## 2. Population and method

1. Extracted the 196 `disposition = "deferred"` ids from `parity/manifest-map.json` and joined each
   to `parity/census.json` (project, class, method, case index, source file and line) — 90 classes.
2. Read the C# case body at that line, including the theory data rows that decide a case index
   (`Series_AcrossAPoleOrABranchPoint`, `TheDeepestAcceptedCall`, `LanguageStatuses`,
   `AnExactlyIntegralRealLiteral`, `EveryStatusCarriesTheDocumentedPair`), because the C#
   assertion — not the Rust output — is the contract.
3. Ran the script the case actually drives through both binaries with the differential's argv and
   normalisation, through a Node harness (exact argv, `--file` temp scripts, no PowerShell quoting),
   then re-checked every difference with the raw binaries. 87 probes cover the 196 rows; a theory
   row shares one script with its siblings and each aggregate row is probed at every shape its body
   drives.
4. Classified: full normalised envelope equality **and** a runner-observable (or library-observable)
   translation of every C# assertion ⇒ MATCH-NOW; any exit-code/JSON/envelope difference ⇒
   STILL-DIFFERENT with the first differing path and both values; a contract that only a C#-library
   object can decide ⇒ UNRUNNABLE, with the API named and the named surface checked against the Rust
   tree.
5. Re-measured every row that was already a measured family in round 50, because the round-51..65
   landings (dispatcher porting, Value widen, Compilation/IrProgram/MathIR typing, ExprContext,
   RationalFunctions.Cancel + symbolic matrix, rewrite registry, ScriptSource, .NET Random corpora,
   RecordSchemas, fixed-point DSP, Real fast-path, numeric piecewise/relation arms, integrate
   substitution, canonical idempotence, fft/conv/complex) can change either side.

## 3. MATCH-NOW — 15 rows (15 new tests)

The tests live in `rust/lovelace-symbolics/src/round66_resweep.rs` (11),
`rust/lovelace-lang/src/round66_resweep.rs` (1), `rust/lovelace-run/src/round66_resweep.rs` (2) and
`rust/lovelace-real/src/round66_resweep.rs` (1). Each is the C# case's own assertion on the field or
library function that carries it.

| Census row (class/method#case) | Rust test | C# contract asserted |
|---|---|---|
| `Lovelace.Symbolics.Tests.AcceptanceTests/Interning_EqualStructureSharesReference#0` | `lovelace-symbolics::round66_resweep::acceptance_interned_equal_structure_is_the_same_value` | `Add(Power(x,2), x)` and `Add(x, Power(x,2))` are the same expression; `Assert.Same` translates to value equality because Rust `Expr` is a value (SmokeTests.cs:245). |
| `…/AcceptanceTests/PrintParse_RoundTrips#0` | `…::acceptance_canonical_print_parse_round_trips` | canonical print/parse round-trip; the reference's interning identity is asserted as canonical-form identity (SmokeTests.cs:224). |
| `…/AcceptanceTests/Inverse_SymbolicMatrix_TimesOriginalIsIdentity#0` | `lovelace-lang::round66_resweep::acceptance_inverse_times_original_is_identity` | `A·A⁻¹ = I` verified numerically at x = 3, y = 2 (SmokeTests.cs:158). |
| `…/UsageExamples/Snippet_16_2_EqualityAndHashConsing#0` | `…::usage_snippet_16_2_equality_and_hash_consing` | the published snippet: both construction orders are one expression (UsageExamples.cs:27). |
| `…/UsageExamples/Snippet_16_4_CanonicalTextRoundTrip#0` | `…::usage_snippet_16_4_canonical_text_round_trip` | `x^2 + 2*x + 1` canonical text round-trips (UsageExamples.cs:50). |
| `…/UsageExamples/Snippet_16_8_ExactnessDiscipline#0` | `…::usage_snippet_16_8_exactness_discipline` | exact rational / radical / closed form are exact; the Real leaf is not (UsageExamples.cs:96). |
| `…/MatrixAdvanceTests/InverseWithConditions_Singular_ReturnsNullMatrix_And_Note#0` | `…::matrix_inverse_with_conditions_singular_reports_no_matrix_and_the_note` | singular matrix: `Matrix` null, note `matrix is singular`, empty conditions (MatrixAdvanceTests.cs:57). |
| `…/PolynomialsApiTests/SquareFree_X2Times_XMinus1#0` | `…::polynomials_square_free_of_x_squared_times_x_minus_one` | `x^3 - x^2` → `(x - 1)` once and `x` twice (PolynomialsApiTests.cs:70). |
| `…/UnsatisfiablePropagationTests/Transform_RequirementConsistentWithAssumptions_KeepsItsConditions#0` | `…::transform_requirement_consistent_with_assumptions_keeps_its_conditions` | under `x > 5`, `Transform(x/x)` keeps its conditions and requires NonZero (line 46). |
| `…/UnsatisfiablePropagationTests/Transform_RequirementRefutedByAssumptions_IsUnsatisfiable#0` | `…::transform_requirement_refuted_by_assumptions_is_unsatisfiable` | under `x == 0` the same branch is unsatisfiable (line 31). |
| `…/DxSemanticClosureTests/Transform_BudgetExceeded_IsReported#0` | `…::dx_semantic_transform_budget_exceeded_is_reported` | `MaxSteps: 0` reports the exceeded budget (line 146). |
| `…/DxSemanticClosureTests/Transform_TraceIsOptIn_ButConditionsAlwaysSurvive#0` | `…::dx_semantic_transform_trace_is_opt_in_but_conditions_always_survive` | trace off ⇒ empty steps, conditions survive, budget not exceeded (line 134). |
| `…/SolveCompletenessMappingTests/LanguageStatuses_ReportExactlyWhatTheMappingReturns#5` | `lovelace-run::round66_resweep::solve_language_statuses_publish_the_unevaluated_pair` | `solve_full(x^4 + 1 == 0, x)` publishes Unevaluated / false / Unknown (line 67). |
| `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.DspTests/ImpulseResponse_GivenFirstOrderIir_MatchesHandComputed#0` | `lovelace-run::round66_resweep::dsp_impulse_response_of_a_first_order_iir_matches_the_hand_computed_sequence` | IIR `y[n] = 0.1x[n] + 0.9y[n-1]` → `[0.1, 0.09, 0.081, 0.0729]` (DspTests.cs:109). |
| `precbench.Tests/PrecBench.Tests.AccuracyTests/WithPrecision_GivenScope_RestoresGlobalState#0` | `lovelace-real::round66_resweep::with_precision_scope_restores_the_global_state` | a `WithPrecision(7, …)` scope restores the two global decimal-place values (AccuracyTests.cs:76). |

Two rows were decided by the wire and re-confirmed against the oracle envelope for the same argv:
the solve-completeness triple (`x = symbol("x"); solve_full(x^4 + 1 == 0, x)`) and the DSP impulse
response (`filter([1, -0.9], [0.1], 4)`), both byte-identical after normalisation. The other
thirteen are library-level rows whose contract the Rust tree now exposes directly (`Expr::canonical`
/ `canonical_parse` / `is_exact`, `SymbolicMatrix::inverse_with_conditions`/`multiply`,
`square_free_univariate`, `rewrite_registry::transform`, `numeric::evaluate` and the Real precision
scope).

## 4. STILL-DIFFERENT — 51 rows, 24 families

Field paths are from the differential's normalised comparison. "Oracle" is the pinned C# runner,
"Rust" is `rust/target/release/lovelace-run.exe` (or the library surface named); both received the
same argv.

| Family | Rows | Measured field and values |
|---|---:|---|
| `arity-shape` | 5 | `min()` and `compile_full(x^2+1)`: `$.category` oracle **TypeMismatch** vs Rust **DomainError**; `filter([1],[1])` now matches. The two wrong-shape sweeps and the five-probe row include `min()`/`compile_full`, so they inherit the divergence. |
| `evalf-precision` | 4 | `evalf(pi,1000)` `$.result.structured.value`: **1000 vs 100** fractional digits (2 rows); `sin(0)` display **"0" vs "sin(0)"**; `evalf(sin(pi(30)),40)` display **"0" vs "0.0000000000000000000000000000005028841971"**. |
| `plot-wire` | 4 | `--plot-dir ""` exit **1 vs 0**; `--plot-file ""` message names the requested path vs **"Cannot write plot: The system cannot find the path specified. (os error 3)."**; `--plot-file nope/x.svg` exit **1 vs 0**; `plot(sin(x))`/`plot(1)` `$.category` **DomainError vs TypeMismatch**. |
| `abs-rewrite` | 4 | `simplify_full(sqrt(x^2))`: oracle `expression: abs(x), changed True, conditions [DomainCondition(real)]` vs Rust `expression: sqrt(x^2), changed False, conditions []` (2 rows); `subs(abs(x),x,-3)` display **"3" vs "abs(-3)"**; the fourth row is the same condition value on the TransformResult. |
| `evalir-multi` | 4 | `compile_full(x+y,[x,y])` `result_type`: oracle **unknown** vs Rust **Integer**; `evalir_batch(compile(x*y+1,[x,y]), [1,2,3], 40)` oracle **"evalir_batch(): kernel expects 2 parameters per row; got 3 values …"** vs Rust exit 1 **"evalir supports one free symbol."**; the row-shaped and width-mismatch rows exit **0 vs 1** and keep the same message divergence. |
| `series` | 4 | `series(1/x,x,0,2)` display **"x^-1 + O(x^2)" vs "0"**; `series(1/x^2,x,0,2)` **"x^-2 + O(x^2)" vs "0"**; `series(sin(x)/x,x,0,3)` **"1 - 1/6*x^2 + O(x^3)" vs "1 + O(x^3)"**; `series(sqrt(abs(x)),x,0,2)` **"sqrt(x)" vs "sqrt(abs(x))"**. |
| `recursion` | 3 | 5 calls inside 120 nested `if`s: exit **1 (DepthExceeded) vs 0**; `f(85)` in a `for` body and in a `while` body: `$.result` **Natural 0 vs missing** (Rust returns a refusal envelope). |
| `broadcast` | 2 | `[[1],[2]] + [[10,20],[30,40]]` exit **0 vs 1**; batched 3-D `matmul(a,a)` exit **0 vs 1**. |
| `cancellation` | 2 | `--cancel-after 1500` on `2^1000000000` and on `2000000!`: the oracle settles with a cancellation envelope; the Rust runner **does not terminate within 25 s** (measured twice). |
| `variables` | 2 | `func f() { local = 42 }; f()` and `{ z = 99 }`: `$.variables` length **1 vs 2** (the function-local / block-local leaks). |
| `capability-advertisement` | 2 | `plot.symbolic-expression` live `$.category` **DomainError vs TypeMismatch**; `max(1,2)` and `x = symbol("x"); sum(x,5)` exit **1 vs 0** (the oracle refuses, Rust answers). |
| `solve-partial` | 2 | `linsolve([[x,x],[x,x]],[1,1])` exit **1 vs 0** (the convenience projection does not throw; the `linsolve_full` record matches); the 2^8 system: status/complete/completeness **Partial/false/Partial vs Solved/true/Complete**. |
| `evalir-digits` | 2 | `evalir(…,[1],2.0)` and `evalir_batch(…,[1,2],2.0)`: `$.category` **DomainError vs TypeMismatch** (both exit 1). |
| `parser` | 1 | `x = y = 5`: exit **0 (display 5) vs 1** — Rust ParseError "Expected ';' or end of input but found '=' at position 6." |
| `power-zero` | 1 | `0^(1/2)`: exit **0 (display 0) vs 1** (the other scope counterexamples `(-4)^(1/2)`, `(-1)^(1/2)`, `(-2)^(1/2)`, `(-4)^0.5`, `(-4)^(2/4)` match). |
| `cancel` | 1 | `cancel((x^2+1)/(x-1))`: display **"(x^2 + 1)/(x - 1)" vs "x + 1"** (`cancel(x/x)` and `cancel((x^2-1)/(x-1))` match; `cancel_full(x/x)` matches too). |
| `large-exponent` | 1 | `2^-100000`: oracle publishes `1 / 2^100000` exactly; the Rust process **does not terminate within 25 s** (no envelope). |
| `record-value` | 1 | `v = [1,2,3]; v.length` diagnostic message: oracle **"member 'length' is not available on type 'Vector': member access requires a record result."** vs Rust **"Cannot read field 'length' from a value of kind 'Vector'."** |
| `help` | 1 | `help::overview()` lists the categories with counts but **not the "help <function>" line** the C# row requires. |
| `name-suggestions` | 1 | `registry::suggest("fft")` returns **["fft"]** (the exact registered name) where the C# row requires an empty list; `nosuchfn`/`push`/`zip` are empty in both. |
| `latex` | 1 | `latex(derivative(f, [x_1]))`: **"\operatorname{diff}(f, x\_1)" vs "\operatorname{derivative}(f, x\_1)"**; the escaped product rendering matches. |
| `solve-note` | 1 | `x = symbol("x"); solve_full(x == x, x)` diagnostic message: **"0 = 0: every value is a solution." vs "No solver for this structure."** (`limit_full(a/x,x,0)` matches). |
| `assumptions` | 1 | `AssumptionSet`: `x == 0` then `NonZero(x)` adds **without** a contradiction (the forward order does contradict); the C# row requires both orders to throw. |
| `mathir-ulp` | 1 | `MathIR_EvaluatesLikeSymbolic`: `numeric::evaluate` and the lowered evaluation differ by **1 in the 50th decimal place** on corpus expression 2 (round-62 measurement, unchanged). |

### 4.1 Exact rows, by family

**arity-shape (5)**: `ArityPropertyTests/EveryBuiltin_RefusesEveryCountOutsideItsDeclaredArity…`,
`ArityValidationTests/TooFewArguments…`, `BuiltinArgumentShapeSweepTests/EveryWrongShapedArgument…
(2 rows)`, `CallSignatureDiagnosticsTests/EveryArityRefusalAgreesWithTheRegistryEntry` (all
`Lovelace.Run.Tests`).

**evalf-precision (4)**: `EvalfDigitCapHonestyTests/ARequestAtTheCap_IsNotMarked`,
`StructuredRealPrecisionTests/EvalfCountAboveTheAmbientDisplayPrecision_CrossesInFull`,
`EvalfTruncatedConstantExactnessTests/ExactControls_AreUntouched`,
`EvalfWorkingPrecisionHonoursRequestedDecimalsTests/ValuesBelowTheComputationCap_StayInexact`.

**plot-wire (4)**: `PlotDirectoryErrorTests/EmptyPlotDirectory…`,
`PlotFileErrorTests/EmptyPlotFile…`, `PlotFileErrorTests/PlotFileUnderAMissingDirectory…` and
`CapabilitiesBuiltinTests/SymbolicPlot_IsATypedRefusal…`.

**abs-rewrite (4)**: `AbsSymbolicRoundTripTests/Abs_Of_Symbol…`,
`AbsSymbolicRoundTripTests/Simplify_Sqrt_Square…`,
`AbsSymbolicRoundTripTests/Subs_Abs_Symbolic_At_Negative_Value_Gives_Three`,
`UnsatisfiablePropagationTests/SimplifyFull_DomainCondition…`.

**evalir-multi (4)**: `DxStructuredResultsTests/EvalirBatch_AcceptsNestedRowValues`,
`…/EvalirBatch_FlatInputNotAWholeNumberOfRows_IsRejected`,
`…/EvalirBatch_RowOrientedInput_EvaluatesRowByRow`,
`…/EvalirBatch_RowWidthMismatch_IsRejectedWithTheDocumentedShape`.

**series (4)**: `SeriesLaurentAndBranchPointTests/Series_AcrossAPoleOrABranchPoint…` cases 0, 1, 2, 4
(case 3, `series(sqrt(x),…)`, matches and is already mapped).

**recursion (3)**: `RecursionDepthEnvelopeTests/NestedBlocksInsideARecursiveBody…`,
`RecursionBudgetCompositionTests/TheDeepestAcceptedCall…#14` (for body), `#15` (while body).

**broadcast (2)**: `NdArrayTests/MatMul_Batched`, `BroadcastTests/ColumnVector_BroadcastsToMatrix`.

**cancellation (2)**: `CancellationObservationTests/Evaluate_GivenLongFactorialAndShortBudget…`,
`…/Evaluate_GivenLongNaturalPowerAndShortBudget…`.

**variables (2)**: `FunctionTests/Evaluate_GivenFunctionLocalVariable…`,
`StatementTests/Evaluate_GivenVariableAssignmentInsideBlock…`.

**capability-advertisement (2)**: `CapabilitiesBuiltinTests/AdvertisedCodesAndCategories…`,
`…/ExactnessBestEffort_IsBackedByLiveRefusals…`.

**solve-partial (2)**: `DxStructuredResultsTests/LinsolveFull_SingularSystem…`,
`SolveCompletenessPairingTests/EverySolveStatus_PublishesTheDocumentedPair…`.

**evalir-digits (2)**: `EvalirDigitArgumentContractTests/AnExactlyIntegralRealLiteral…#0`, `#1`.

Singles: `help` = `HelpServiceTests/Help_Overview_ListsAllCategories`; `name-suggestions` =
`NameSuggestionsTests/ANameNothingIsCloseTo`; `parser` = `ParserTests/Parse_GivenChainedAssignment`;
`power-zero` = `CapabilitiesBuiltinTests/PowerExponentClass…`; `cancel` =
`MetamorphicSetClosureTests/CancelBuiltin…`; `large-exponent` =
`StructuredRealExactnessTests/TerminatingQuotient…#2`; `record-value` =
`RecordValueTests/MemberAccess_OnNonRecord…`; `solve-note` = `DiagnosticContractTests/KernelNotes…`;
`latex` = `LatexSymbolNameEscapingTests/…InsideLargerExpressions…`; `assumptions` =
`AssumptionSoundnessTests/PropertyAndRelationAtoms_Contradict`; `mathir-ulp` =
`PropertyTests/MathIR_EvaluatesLikeSymbolic`.

## 5. Ranked remaining — 130 UNRUNNABLE over 58 APIs

Ranked by row count; each API was checked against the Rust tree (an internal analogue is not an
observable equivalent, and the row's reason says so).

| Rows | Missing Rust API |
|---:|---|
| 8 | SymPy oracle session (`SympyOracle.Require`) |
| 6 | modus plugin registration and kernel dispatch seam |
| 5 | `PlotValue` and `SvgPlotRenderer` API |
| 5 | Polynomial content/primitive-part/gcd/lcm/resultant/discriminant/factor API |
| 4 | modus plugin arity registration seam |
| 4 | `StructuralEquality.FirstDifference` path reporting |
| 4 | `SuiteEngine` CaptureState, events and SetVariable/RemoveVariable API |
| 4 | `EvaluationException` vocabulary on the evaluator API |
| 4 | `Evaluation.EvaluateToNum/EvaluateToExpr` with a throwing evaluator |
| 4 | `SymbolicMatrix` inverse/rank/solve/solve_with_conditions (only `inverse_with_conditions` exists) |
| 4 | assumption-scoped numeric evaluation (`EvaluateToNum` with caller assumptions) |
| 4 | `Integration.IntegrateResult` / `Evaluation` / `Exprs` construction API |
| 3 | `DenseArray<Complex>` construction/span API |
| 3 | Sum/Product/Sequence signal composition API (complex arrays refuse `+`/`*`) |
| 3 | `Integer.Pow(Integer)` refusal semantics (`pow` takes `u64`) |
| 3 | modus builtin registration that coerces/casts its payload |
| 3 | modus plugin load/duplicate-name refusal seam |
| 3 | `SuiteEngine` `PlotFileWriteException` typed refusal |
| 3 | `Expr.IsExact` leaf rule over a corpus with RootOf/Complex leaves |
| 2 | `Exprs.Real(RealLiteral)` provenance and `NumReal` payload API |
| 2 | `Integer.TryFormat(Span<char>)` fallible formatting API |
| 2 | plugin-defined record producer (modus seam) for the DemoResult schema |
| 2 | plugin-instance assumption-scope API |
| 2 | `SuiteEngine` output writer / `ResetPlotCapture` API |
| 2 | `ValueFormatter` depth guard API |
| 2 | README/UsageExamples sync checker |
| 2 | `Domains.DomainOf` / `Solvers.Solve` internals |
| 2 | `SymbolicMatrix.Solve` with numeric-substitution verification |
| 2 | `Printing.PrintOptions/PrintBudget/PrintOutcome` (library max depth / node budget) |
| 2 | `Printing.PrintOptions/PrintOutcome` token-boundary API |
| 2 | rewrite proof-obligation bridge API |
| 2 | public `SolveCompletenessMapping.Of(status, completeness)` API |
| 2 | `RealLiteral.FromRealExact/ToReal` and `NumReal` payload API |
| 1 | distinct `Integer(Int)`/`Integer(Long)` constructor overloads |
| 1 | `Integer` reference-identity equality (Rust `Int` is a value) |
| 1 | `Integer` `operator*` overload |
| 1 | `Integer` `operator-` overload |
| 1 | `Real.Cos` fast path meeting the 10 s budget at 1000 places (`fast_path_tests.rs` documents the miss) |
| 1 | `Lovelace.Representation` crate (the C# case has an empty body and no observable) |
| 1 | registry `FunctionDefinition.MinArity/Variadic` introspection |
| 1 | `ErrorCategory` enum and the three mirrored taxonomy arrays |
| 1 | golden-fixture recorder (`TestSupport.Normalise` + `SourceGoldenPath`) |
| 1 | engine computation/display decimal-place projection API |
| 1 | `AllocationRefusalException` type |
| 1 | `ArgumentContractException` type |
| 1 | modus seam carrying `EnumValue` payloads |
| 1 | `UnicodeOutput` formatting toggle on the engine |
| 1 | public `NameSuggestions.EditDistance` API (`registry::edit_distance` is private) |
| 1 | `syntax::Expr` unary-plus AST node (`+5` parses to the literal) |
| 1 | deep-expression AST construction and `EvaluationException` API |
| 1 | `OperationTiming.ElapsedDisplay/ElapsedScale` API |
| 1 | `ErrorCategory` enum (the seven frozen members) |
| 1 | `SymbolicsPlugin.ParameterDomainOf` |
| 1 | factor/expand corpus API (`expand(factor(p)) == p` over the shared corpus) |
| 1 | library builtin count (`CaptureState().Functions` builtin count) |
| 1 | `Solvers.Solve` inverse-branch candidate verification API |
| 1 | complex constant leaf (`Exprs.Complex`) for the model-level printer |
| 1 | complex constant leaf (`Exprs.Complex`) for the pretty printer |

The largest single cause is still a **library-surface absence, not a wire defect**: 5 of the top 6
families (SymPy 8, modus 24 across its five seams, PlotValue 5, Polynomials 5, StructuralEquality 4)
have no runner-observable field at all. The biggest *wire* families are arity taxonomy (5), evalf
digit cap (4), plot wire (4), abs rewrite (4), evalir multi-parameter batch (4) and series (4).

## 6. What changed since round 50

1. **Fifteen rows now match and are mapped** — the canonical-form/interning cluster (3 Acceptance +
   3 UsageExamples), the symbolic matrix inverse pair, the rewrite-registry `Transform` protocol
   (conditions, unsatisfiable branch, budget, trace), square-free decomposition, the solve status
   pair, the DSP first-order IIR impulse response and the Real precision scope.
2. **Two rows moved UNRUNNABLE → MATCH-NOW without a new production surface**: the `Transform`
   rows were blocked on `Simplify.Transform`; the round-59 rewrite registry now publishes the same
   protocol, and `AssumptionSet::add/ask` supplies the condition atoms.
3. **One row moved MATCH-NOW → STILL-DIFFERENT during this sweep**: `AssumptionSoundnessTests/
   PropertyAndRelationAtoms_Contradict` looked portable (the forward order reports a contradiction),
   but the measured reverse order (`x == 0` then `NonZero(x)`) is accepted; the row requires both
   orders to throw, so it is deferred with that exact field. No test asserts a weakened form.
4. **The dispatcher gap measured in round 50 is closed**: `filter`, `scale`, `ndims` and
   `cancel_full` all answer now, and `filter([1],[1])` matches the oracle's `TypeMismatch`. No
   deferred row is blocked on a registered-but-undispatched builtin any more.
5. **The abs rewrite and the series Laurent tail are unchanged** (round 64's numeric fixes touched
   piecewise/relation arms, not `sqrt(x^2)` or the Laurent tail), and `0^(1/2)` is still refused.

## 7. Verification (observed this round)

Run on 2026-11-12 (America/Sao_Paulo) with the pinned toolchain.

| Step | Command | Observed line | Exit |
|---|---|---|---:|
| Inventory + suite | `powershell -ExecutionPolicy Bypass -File parity/gen-rust-tests.ps1` | `crates=11 tests=5099 suiteExit=0 passed=5098 failed=0` | 0 |
| Full suite (ignored count) | `cargo test --workspace` | `5,098 passed, 0 failed, 1 ignored` over 21 binaries | 0 |
| Manifest | `node parity/gen-manifest.mjs` | `{"rows": 6209, "dispositions": {"excluded": 118, "ported": 5910, "deferred": 181}, "coveragePercent": 95.18, "rustTests": 5099}` | 0 |
| Manifest gate | `node parity/tools/gate-manifest.mjs` | `"failures": []`; `PASS (structural): 6209 test cases dispositioned; 5910 claim Rust coverage (95.18%).` | 0 |
| Coverage report | `node parity/tools/coverage-report.mjs` | `coverage.md written: 5910/6209 ported (95.18%)` | 0 |
| Documentation corpus | `cargo run --release -p lovelace-corpus` | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| Protocol fixtures | `node parity/tools/gate-fixtures.mjs` | `oracle: 20/20`, `rust: 20/20`, `PASS: oracle 20/20, rust 20/20` | 0 |
| Differential (generated + replay) | `node parity/diff/run.mjs --cases 200 --seed 1` | generated **200 match / 0 mismatch / 0 error**; replayed **1758 match / 0 mismatch / 0 error** (1958 executions, 115 duplicates skipped) | 0 |

After regeneration every one of the fifteen mapped ids was re-read from the **generated**
`parity/manifest.toml` and its `rust` test id resolved against the measured
`parity/rust-tests.json` inventory: **15/15 resolve, 0 unresolved**, and every generated manifest
row carries the mapped `rust` target.

## 8. Residual

* 181 rows remain deferred: 51 measured differences (dominated by the arity taxonomy, the evalf
  digit cap, the plot refusals, the abs rewrite, the evalir multi-parameter batch and the series
  Laurent tail) and 130 library-only rows over 58 APIs.
* 118 exclusions unchanged (user-delegated, reversible).
* No production change was made: the arity taxonomy, the evalf structured-digit cap, the plot wire,
  the abs rewrite, the evalir multi-parameter batch, the series Laurent tail, the cancellation
  observation and the `0^(1/2)` support are production work for a later lane; this round measures
  them and maps what the current tree already answers.
