# Round 50 — the 275-row re-sweep (three-way adjudication)

**Status: complete.** Every one of the 275 census rows left deferred after round 49 was
re-adjudicated against the current tree. Each row's script was re-executed through both runners —
the pinned C# oracle `1cc28f255eca6029849f880949fc7be1d40ab8af` and
`rust/target/release/lovelace-run.exe` — with the differential's exact normalisation
(`revision`, `elapsed`, `elapsedTime`, `timings` replaced at any depth; key presence structural),
and each was re-classified into MATCH-NOW / STILL-DIFFERENT (exact field and values) / UNRUNNABLE
(missing API named). Six rows now match and are mapped to six new tests in
`rust/lovelace-run/src/round50_resweep.rs`; 57 remain measurably different; 212 remain blocked on a
named missing Rust API. No production source, `.cs` file, fixture or other document was edited; the
only code written is the test module plus its `mod` registration, and the only manifest edit is
`parity/manifest-map.json`.

## 1. Result

| Classification | Rows | Disposition |
|---|---:|---|
| MATCH-NOW (newly mapped) | 6 | ported to a new Rust test and mapped in `parity/manifest-map.json` |
| STILL-DIFFERENT | 57 | deferred with the measured family and field/values (19 families) |
| UNRUNNABLE | 212 | deferred with the missing Rust API named (74 APIs) |
| **Total re-adjudicated** | **275** | |

Manifest: **5,822 / 6,209 ported (93.77 %)** — up from 5,816 (93.67 %); deferred **275 → 269**;
excluded unchanged at **118**. Rust suite: **4,967 passed, 0 failed** (up from 4,961; +6 tests).
Coverage moved exactly by the six new mappings: Lovelace.Suite.Tests 842 → 844 (+2),
Lovelace.Symbolics.Tests 1,079 → 1,083 (+4).

## 2. Population and method

1. Extracted the 275 `disposition = "deferred"` ids from `parity/manifest-map.json` and joined each to
   `parity/census.json` (project, class, method, case index, source file and line) — 99 distinct
   classes, 275 rows.
2. Read the C# case body at that line (including the theory data rows for
   `Series_AcrossAPoleOrABranchPoint…`, `RecursionBudgetComposition…`, `HyperbolicBuiltins…`,
   `SolveCompletenessPairing…`, `StructuredRealExactness…` and `DescriptorExampleTests`), because the
   C# assertion — not the Rust output — is the contract.
3. Ran every script the case actually drives through both binaries with the differential's argv and
   normalisation, first through a Node harness (the same `spawn` + exact-argv path the differential
   uses, so an embedded `"` survives), then re-checked every difference with the raw binaries.
   90 probes cover the 275 rows (a theory row shares one script with its siblings; an aggregate row
   was probed at each shape its body drives).
4. Classified: full normalised envelope equality **and** a runner-observable translation of every C#
   assertion ⇒ MATCH-NOW; any exit-code/JSON/envelope difference ⇒ STILL-DIFFERENT with the first
   differing path and both values; a contract that only a C#-library object can decide ⇒ UNRUNNABLE,
   with the API named and the named surface checked against the Rust tree (an internal analogue —
   e.g. `value::widen` is `pub(crate)` — is not an observable equivalent, and the row's reason says so).
5. Re-measured every row that was already a measured family in round 45, because a landing can change
   either side. The five post-45 surfaces (lexer TokenKind, falsification harness, RecordSchemas,
   fixed-point DSP, Real fast-path) had already been mapped in rounds 46–49; this sweep confirms they
   unblocked no further deferred row by themselves, but the descriptor `--function` surface they
   exposed does decide four rows round 45 had called UNRUNNABLE, and the published example corpus
   decides six more.

## 3. MATCH-NOW — 6 rows (6 new tests)

All six live in `rust/lovelace-run/src/round50_resweep.rs` and drive
`parity_batch::run_args` — the runner's own parse/evaluate/envelope path. Each expected value is
the C# case's own assertion, translated onto the field the reference renders it from; the oracle
publishes the same normalised envelope for the same argv (measured this round).

| Census row (class/method#case) | Rust test | C# contract asserted |
|---|---|---|
| `Lovelace.Suite.Tests.SuiteEngineTests/CaptureState_GivenVariablesAndFunctions_ReturnsMatchingSnapshot#0` | `round50_resweep::capture_state_publishes_variables_and_user_functions` | after `x = 3.14` and `func f(y) = y * 2`, x is captured as a Real and f is present as a **non-builtin** function (SuiteEngineTests.cs:56). |
| `Lovelace.Suite.Tests.SuiteEngineTests/EvaluateAsync_GivenScript_RecordsElapsedTime#0` | `round50_resweep::a_completed_script_records_its_elapsed_time` | a completed `1 + 1` records a non-negative elapsed and a non-empty display: `elapsedTime.value >= 0`, `elapsedTime.unit` and `elapsed` non-empty (SuiteEngineTests.cs:152). |
| `Lovelace.Symbolics.Tests.HyperbolicBuiltinsAndNegativeIntegerPowersTests/HyperbolicBuiltins_HaveCompleteDescriptors#0` | `round50_resweep::asinh_descriptor_is_complete_and_cross_references_sinh` | asinh: summary present, parameter list `[x]`, example `asinh(0)`, return kind Symbolic, and a sinh cross-reference in both directions (line 78, row `("asinh","sinh","asinh(0)")`). |
| `…/HyperbolicBuiltins_HaveCompleteDescriptors#1` | `round50_resweep::acosh_descriptor_is_complete_and_cross_references_cosh` | same contract for `("acosh","cosh","acosh(1)")`. |
| `…/HyperbolicBuiltins_HaveCompleteDescriptors#2` | `round50_resweep::atanh_descriptor_is_complete_and_cross_references_tanh` | same contract for `("atanh","tanh","atanh(0)")`. |
| `Lovelace.Symbolics.Tests.CapabilitiesBuiltinTests/Capabilities_IsOneRegisteredBuiltin_WithCompleteMetadata#0` | `round50_resweep::capabilities_is_one_registered_builtin_with_complete_metadata` | exactly one capabilities() builtin, registered by `Lovelace.Symbolics`, category Introspection, no parameters, summary/examples/related present, return kind `CapabilitiesResult`, no placeholder help (line 654). |

The four descriptor rows are decidable because the runner's `--function <name>` publishes the same
`BuiltinDescriptor` the C# `HelpService.Function` text is rendered from: a textual help assertion
becomes the field that renders it (e.g. "help contains `asinh(x)`" ⇒ `parameters == ["x"]`;
"not Returns: Value" ⇒ `returnKind == "Symbolic"`). Both runners publish byte-identical
descriptors for all seven names (`asinh/sinh/acosh/cosh/atanh/tanh/capabilities`).

## 4. STILL-DIFFERENT — 19 families, 57 rows

Field paths are from the differential's normalised comparison. "Oracle" is the pinned C# runner,
"Rust" is `rust/target/release/lovelace-run.exe`; both received the same argv.

| Family | Rows | Measured field and values |
|---|---:|---|
| `dsp` | 7 | `filter([1],[1,2,3],[1,1,1,1])` / `filter([1,-0.5],[1],[1,1,1,1])` / `filter([1,-0.5],[1],4)`: exit **1 vs 0** ("Unknown function 'filter'. Did you mean 'filter'?"); `fft([])` exit **1 vs 0**; `fft([1,1,1])` exit **0 vs 1**; `fft([0,1,0,0])[1] + 1` `$.diagnostics[0].message` stops after "…support widening." — the oracle appends "Reductions over Complex arrays (sum/mean/dot/norm/matmul) are not supported; use re()/im()/conj()/abs() to bridge back to Real." |
| `descriptor-example` | 6 | the six published examples `cancel_full(x/x)`, `cancel_full((x^2-1)/(x-1))`, `filter([1],[1],[1,2,3])`, `ndims([[1,2],[3,4]])`, `scale([1,2,3],2)`, `scale([1,2,3],0.5)`: exit **0 vs 1**, message "Unknown function '<name>'. Did you mean …?" — the builtin is registered but has no dispatcher arm. (Newly measured; round 45 had these as UNRUNNABLE.) |
| `arity-shape` | 5 | `min()` and `x = symbol("x"); compile_full(x^2+1)`: `$.category` **TypeMismatch vs DomainError**; `filter([1],[1])`: `$.category` **TypeMismatch vs UnsupportedOperation**; the two registry sweeps inherit these rows. |
| `evalf-precision` | 4 | `evalf(sin(pi(30)),40)` `$.result.display` **"0" vs "0.0000000000000000000000000000005028841971"**; `sin(0)` display **"0" vs "sin(0)"**; `evalf(pi,1000)` `$.result.structured.value` carries **1000 vs 100** fractional digits (2 rows; both displays keep 100 digits, both inexact). |
| `plot-wire` | 4 | `--plot-dir ""` exit **1 vs 0**; `--plot-file ""` `$.diagnostics[0].message` names the requested path vs "Cannot write plot: The system cannot find the path specified. (os error 3)."; `--plot-file nope/x.svg` exit **1 vs 0**; `plot(sin(x))` `$.category` **DomainError vs TypeMismatch**. |
| `abs-rewrite` | 4 | `simplify_full(sqrt(x^2))` `$.result.display` **TransformResult(… expression: abs(x), changed: True) vs (… expression: sqrt(x^2), changed: False)** (2 rows: Simplify_Sqrt_Square and SimplifyFull_DomainCondition); `subs(abs(x),x,-3)` display **"3" vs "abs(-3)"**. |
| `evalir-multi` | 4 | `evalir_batch(compile(x*y+1,[x,y]), [[1,2],[3,4]], 40)` exit **0 vs 1**; width/flat refusals: `$.diagnostics[0].message` **"evalir_batch(): kernel expects 2 parameters per row; …" vs "evalir supports one free symbol."**; `compile_full(x+y,[x,y])` with two free symbols is refused by Rust. |
| `series` | 4 | `series(1/x,x,0,2)` display **"x^-1 + O(x^2)" vs "0"**; `series(1/x^2,x,0,2)` **"x^-2 + O(x^2)" vs "0"**; `series(sin(x)/x,x,0,3)` **"1 - 1/6*x^2 + O(x^3)" vs "1 + O(x^3)"**; `series(sqrt(abs(x)),x,0,2)` **"sqrt(x)" vs "sqrt(abs(x))"**. |
| `recursion` | 3 | 5 calls inside 120 nested `if`s: exit **1 vs 0**; `f(85)` inside a `for` body (and a `while` body): `$.result` **Natural 0 vs null** (Rust returns a refusal envelope). |
| `broadcast` | 2 | `[[1],[2]] + [[10,20],[30,40]]` exit **0 vs 1**; batched 3-D `matmul(a,a)` exit **0 vs 1**. |
| `variables` | 2 | `$.variables` **1 vs 2** entries: a function-local (`local`) and a block-local (`z`) assignment leak into the Rust envelope. |
| `capability-advertisement` | 2 | `plot(sin(x))` `$.category` **DomainError vs TypeMismatch** (the advertised entry no longer equals the live envelope); `max(1,2)` and `sum(x,5)` exit **1 vs 0** — the oracle refuses, Rust answers. |
| `solve-partial` | 2 | the 2^8 system `$.result.display` **Partial/complete False/Partial vs Solved/complete True/Complete**; `linsolve([[x,x],[x,x]],[1,1])` exit **1 vs 0**. |
| `evalir-digits` | 2 | `evalir(…,[1],2.0)` and `evalir_batch(…,[1,2],2.0)`: exit **0 vs 1**; the oracle answers 0.33 (Real, inexact) — the exactly-integral Real 2.0 is normalised to the integer 2 — while Rust refuses with "evalir(): argument 3 (digits) must be a Natural or Integer; got the Real 2, which is integral — write it as 2." |
| `cancel` | 2 | `cancel((x^2+1)/(x-1))` display **"(x^2 + 1)/(x - 1)" vs "x + 1"**; `cancel_full(x/x)` exit **0 vs 1** ("Unknown function 'cancel_full'. Did you mean …?"). |
| `large-exponent` | 1 | `2^-100000`: exit **0 vs -1** — the Rust process aborts with no envelope. |
| `parser` | 1 | `x = y = 5`: exit **0 (display 5) vs 1**, Rust code ParseError "Expected ';' or end of input but found '=' at position 6.". The reference parser (and the oracle runner) accept the right-associative assignment. |
| `power-zero` | 1 | `0^(1/2)` exit **0 vs 1** — the oracle supports the zero base under the principal square root, Rust refuses it. |
| `solve-selfeq` | 1 | `solve_full(x == x, x)` `$.result.display`: same status/code/category, but the diagnostic message is **"0 = 0: every value is a solution." vs "No solver for this structure."**. |

### 4.1 Exact rows, by family

**dsp** (7)

- `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.DspTests/Filter_GivenFir_MatchesConvolution#0`
- `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.DspTests/Filter_GivenIir_MatchesImpulseResponseConvolution#0`
- `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.FftTests/Fft_GivenEmpty_ReturnsEmpty#0`
- `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.FftTests/Fft_GivenNonPowerOfTwo_ThrowsArgumentException#0`
- `Lovelace.Suite.Tests/Lovelace.Suite.Tests.DspPluginTests/ComplexArithmetic_GivenComplex_ThrowsActionableError#0`
- `Lovelace.Suite.Tests/Lovelace.Suite.Tests.DspPluginTests/Filter_GivenLength_ReturnsImpulseResponse#0`
- `Lovelace.Suite.Tests/Lovelace.Suite.Tests.DspPluginTests/Filter_GivenSignal_MatchesConvolution#0`

**descriptor-example** (6): `Example_Executes#21`, `#22`, `#53`, `#86`, `#107`, `#108` (class `Lovelace.Suite.Tests.DescriptorExampleTests`).

**arity-shape** (5): `ArityPropertyTests/EveryBuiltin_RefusesEveryCountOutsideItsDeclaredArity_WithTheOneDocumentedShape#0`, `ArityValidationTests/TooFewArguments_IsATypedStructuralError_NeverAnInternalFailure#0`, `BuiltinArgumentShapeSweepTests/EveryWrongShapedArgument_IsNeverAnInternalFailure_AndEveryArgumentRefusalNamesTheBuiltinAndThePosition#0`, `BuiltinArgumentShapeSweepTests/EveryWrongShapedArgument_OverAWiderShapeLattice_IsStillNeverAnInternalFailure#0`, `CallSignatureDiagnosticsTests/EveryArityRefusalAgreesWithTheRegistryEntry#0` (all `Lovelace.Run.Tests`).

**abs-rewrite** (4): `AbsSymbolicRoundTripTests/Abs_Of_Symbol_Matches_The_Rewriters_Own_Abs_Node#0`, `AbsSymbolicRoundTripTests/Simplify_Sqrt_Square_Under_Real_Is_Still_Abs_X#0`, `AbsSymbolicRoundTripTests/Subs_Abs_Symbolic_At_Negative_Value_Gives_Three#0`, `UnsatisfiablePropagationTests/SimplifyFull_DomainCondition_CarriesADomainValueNotText#0`.

**evalf-precision** (4): `EvalfDigitCapHonestyTests/ARequestAtTheCap_IsNotMarked#0`, `StructuredRealPrecisionTests/EvalfCountAboveTheAmbientDisplayPrecision_CrossesInFull#0`, `EvalfTruncatedConstantExactnessTests/ExactControls_AreUntouched#0`, `EvalfWorkingPrecisionHonoursRequestedDecimalsTests/ValuesBelowTheComputationCap_StayInexact#0`.

**evalir-multi** (4): `DxStructuredResultsTests/EvalirBatch_AcceptsNestedRowValues#0`, `EvalirBatch_FlatInputNotAWholeNumberOfRows_IsRejected#0`, `EvalirBatch_RowOrientedInput_EvaluatesRowByRow#0`, `EvalirBatch_RowWidthMismatch_IsRejectedWithTheDocumentedShape#0`.

**plot-wire** (4): `PlotDirectoryErrorTests/EmptyPlotDirectory_IsAnErrorEnvelopeWithExitOne#0`, `PlotFileErrorTests/EmptyPlotFile_IsAnErrorEnvelopeWithExitOne#0`, `PlotFileErrorTests/PlotFileUnderAMissingDirectory_IsAnErrorEnvelopeWithExitOne#0` (Run.Tests) and `CapabilitiesBuiltinTests/SymbolicPlot_IsATypedRefusal_NeverAnInternalInvariantFailure#0`.

**series** (4): `SeriesLaurentAndBranchPointTests/Series_AcrossAPoleOrABranchPoint_PublishesTheExpansion#0`, `#1`, `#2`, `#4`.

**recursion** (3): `Run.Tests/RecursionDepthEnvelopeTests/NestedBlocksInsideARecursiveBody_AreRefused_InsteadOfDying#0`, `Suite.Tests/RecursionBudgetCompositionTests/TheDeepestAcceptedCall_Answers_InEveryComposition#14`, `#15`.

**broadcast** (2): `Array.Tests/NdArrayTests/MatMul_Batched#0`, `Suite.Tests/BroadcastTests/ColumnVector_BroadcastsToMatrix#0`.

**variables** (2): `Suite.Tests/FunctionTests/Evaluate_GivenFunctionLocalVariable_DoesNotLeakToGlobal#0`, `Suite.Tests/StatementTests/Evaluate_GivenVariableAssignmentInsideBlock_DoesNotLeak#0`.

**capability-advertisement** (2): `CapabilitiesBuiltinTests/AdvertisedCodesAndCategories_MatchTheLiveEnvelope#0`, `CapabilitiesBuiltinTests/ExactnessBestEffort_IsBackedByLiveRefusalsTheListDoesNotCarry#0`.

**solve-partial** (2): `DxStructuredResultsTests/LinsolveFull_SingularSystem_ReportsStatusNotThrow#0`, `SolveCompletenessPairingTests/EverySolveStatus_PublishesTheDocumentedPair_OnBothRecords#0`.

**evalir-digits** (2): `EvalirDigitArgumentContractTests/AnExactlyIntegralRealLiteral_IsTheIntegerItDenotes#0`, `#1`.

**cancel** (2): `MetamorphicSetClosureTests/CancelBuiltin_ReducesAndRefusesToCancelWhenNoFactorIsCommon#0`, `MetamorphicSetClosureTests/CancelFull_ReportsTheConditionsTheBareValueCannotCarry#0`.

Singles: `large-exponent` = StructuredRealExactnessTests/TerminatingQuotientBeyondTheDigitBudget_IsCertifiedExact#2`;
`parser` = Suite.Tests/ParserTests/Parse_GivenChainedAssignment_RightAssociative#0`;
`power-zero` = CapabilitiesBuiltinTests/PowerExponentClass_RefusesWhatItSays_AndNamesWhatIsSupported#0`;
`solve-selfeq` = DiagnosticContractTests/KernelNotes_CrossAsDiagnosticMessages#0`.

## 5. UNRUNNABLE — 212 rows, 74 missing APIs

These rows exercise a C#-library surface with no runner-visible field to assert; each manifest row
names its own API. Grouped and ranked by row count:

| Rows | Missing Rust API |
|---:|---|
| 9 | Value widen/widen-pair API |
| 8 | Compilation.Compile / IrProgram multi-parameter compilation API |
| 8 | SymPy oracle session (SympyOracle.Require) |
| 8 | thread-scoped ExprContext/assumption scopes |
| 7 | IrProgram.Deserialize/Validate/Infer typing API |
| 7 | RationalFunctions.Cancel/CancelWithConditions and SymbolicMatrix inverse APIs |
| 7 | rewrite rule registry with preconditions and trace API |
| 7 | ScriptSource.ToSemicolonStatements source normaliser |
| 7 | System.Random corpus + Printing.CanonicalParse round-trip API |
| 6 | content/gcd/square-free/resultant/discriminant polynomial API |
| 6 | modus plugin registration and kernel dispatch API |
| 5 | PlotValue and SvgPlotRenderer API |
| 5 | SymbolicMatrix inverse/solve-with-conditions API |
| 4 | Domains.DomainOf / Solvers.Solve / Simplify.Transform internals |
| 4 | DspMath.Sequence/Product/Sum and DspMath.Filter/ImpulseResponse library API |
| 4 | Evaluation.EvaluateToNum/EvaluateToExpr with a throwing evaluator |
| 4 | EvaluationException vocabulary on the evaluator API |
| 4 | Exprs.Piecewise + assumption decision API |
| 4 | Integration.IntegrateResult / Evaluation / Exprs construction API |
| 4 | modus plugin registration seam (pair/opt/plain/variadic_min) |
| 4 | StructuralEquality.FirstDifference path reporting |
| 4 | SuiteEngine CaptureState, events and SetVariable/RemoveVariable API |
| 3 | DenseArray<Complex> construction/span API |
| 3 | Expr.IsExact leaf-rule API |
| 3 | Exprs hash-consing/reference-identity API |
| 3 | Exprs/SymbolicMatrix construction API (Hessian, interning identity) |
| 3 | Integer.Pow(Integer) refusal semantics |
| 3 | modus plugin registration seam (register a builtin that coerces/casts its payload) |
| 3 | RecordValue member access + modus plugin seam |
| 3 | SolveCompletenessMapping.Of API |
| 3 | SuiteEngine PlotFileWriteException typed refusal |
| 2 | Exprs.Real/RealLiteral provenance API |
| 2 | HelpService overview/UnicodeOutput formatting API |
| 2 | Integer.TryFormat(Span<char>) API |
| 2 | modus plugin registration seam (duplicate load / core-name collision refusal) |
| 2 | NameSuggestions.edit-distance suggester |
| 2 | plugin-instance assumption-scope API |
| 2 | Printing.PrintOptions/PrintBudget token-boundary API |
| 2 | Printing.PrintOptions/PrintBudget/PrintOutcome API |
| 2 | README/UsageExamples sync checker |
| 2 | RealLiteral/NumReal exactness API |
| 2 | rewrite proof-obligation bridge API |
| 2 | Simplify.Transform with caller assumptions |
| 2 | SuiteEngine cancellation-token API (CancellationToken on EvaluateAsync) |
| 2 | SuiteEngine Output writer / ResetPlotCapture API |
| 2 | SymbolicMatrix linsolve numeric-verification API |
| 2 | ValueFormatter depth guard API |
| 1 | AllocationRefusalException type declaration |
| 1 | ArgumentContractException type |
| 1 | AssumptionSet contradiction API |
| 1 | deep-expression AST construction and EvaluationException API |
| 1 | ErrorCategory enum (seven frozen members) |
| 1 | ErrorCategory enum and the three mirrored taxonomy arrays |
| 1 | factor/expand corpus API (expand(factor(p)) == p) |
| 1 | global Real precision-scope state (WithPrecision restores MaxComputationDecimalPlaces) |
| 1 | golden-fixture recorder (TestSupport.Normalise + SourceGoldenPath) |
| 1 | Integer operator- / Subtract API |
| 1 | Integer operator* / Multiply API |
| 1 | Integer reference/value equality API |
| 1 | Integer(Int) constructor and operator overloads |
| 1 | Latex printer escaping over Exprs.Symbol names |
| 1 | Latex printer over Exprs.Complex construction |
| 1 | Lovelace.Representation crate (the C# case has an empty body and no observable) |
| 1 | modus plugin loading seam (an engine without the Dsp plugin registered) |
| 1 | modus plugin seam carrying EnumValue payloads |
| 1 | OperationTiming.ElapsedDisplay scale |
| 1 | pretty printer over Exprs.Complex construction |
| 1 | Real private fast-path internals (ReduceToTwoPi, WholeQuotient, Normalize, IsBelowDecimalGuard, ToInteger, TrySpecialAngle) |
| 1 | registry FunctionDefinition.MinArity/Variadic introspection |
| 1 | Solvers.Solve inverse-branch candidate verification API |
| 1 | SuiteEngine ComputationDecimalPlaces/DisplayDecimalPlaces + ProjectValue above the 1000-digit language cap |
| 1 | SuiteEngine.CaptureState().Functions builtin count (no Rust library count API) |
| 1 | SymbolicsPlugin.ParameterDomainOf |
| 1 | syntax::Expr unary-plus AST node |

## 6. What changed since round 45

1. **Six rows now match and are mapped** — the two SuiteEngine state rows and four descriptor rows
   (three hyperbolic + capabilities). The descriptor rows are decided by the `--function` query
   surface, which publishes the same BuiltinDescriptor the C# help text renders from; the oracle and
   Rust descriptors are byte-identical for all seven names probed.
2. **Six descriptor-example rows moved UNRUNNABLE → STILL-DIFFERENT.** The example corpus is now
   reconstructible without a C# API: the oracle's own evaluation envelope publishes
   `functions[].examples` in deterministic (function, example) order (141 examples over 123
   functions). Rows `Example_Executes#21/#22/#53/#86/#107/#108` were located in that corpus and run
   after the documented prelude; all six exit 0 in the oracle and 1 in Rust.
3. **One row moved STILL-DIFFERENT → UNRUNNABLE.** `BuiltinCount_IncludesTheThreeHyperbolicFunctions`
   asserts the C# **library** count (107 builtins in a Symbolics-only engine). The runner surface
   `--functions` publishes `total: 123` in **both** runners, so round 45's "registry-count:
   123 vs 107" compared two different surfaces; there is no wire difference, and no Rust API exposes
   the library count.
4. **The parser row is now precisely measured**: `x = y = 5` is accepted by the oracle (exit 0,
   display 5) and refused by Rust with ParseError — the reference parser keeps its right-associative
   assignment.
5. **The biggest single blocker is a dispatcher gap, not a missing API.** `filter`, `scale`, `ndims`
   and `cancel_full` are published by the registry (`--functions` lists them) and rejected by the
   evaluator with "Unknown function '<name>'. Did you mean '<name>'?" — the suggestion proves the
   name is registered. This accounts for 11 of the 57 STILL-DIFFERENT rows (4 `filter`, 2 `scale`,
   1 `ndims`, 1 `cancel_full` descriptor rows, the `dsp` `filter` rows and the `cancel_full` row).
6. **`Cos_AtDefaultPrecision_StaysInteractive#0` stays UNRUNNABLE**: its contract needs
   `Real.WithPrecision(1000, 40)`, which the Rust Real exposes only as an explicit `places` argument
   on internal kernels; the round-48 lane measured the equivalent path at 30.2 s against the test's
   10 s bound, so the row is blocked on a production decision, not on a test.

## 7. Verification (observed this round)

Run on 2026-11-11 (America/Sao_Paulo) with the pinned toolchain.

| Step | Command | Observed line | Exit |
|---|---|---|---:|
| Inventory + suite | `powershell -ExecutionPolicy Bypass -File parity/gen-rust-tests.ps1` | `crates=11 tests=4967 suiteExit=0 passed=4967 failed=0` | 0 |
| Manifest | `node parity/gen-manifest.mjs` | `{"rows": 6209, "dispositions": {"excluded": 118, "ported": 5822, "deferred": 269}, "coveragePercent": 93.77, "rustTests": 4967}` | 0 |
| Manifest gate | `node parity/tools/gate-manifest.mjs` | `"failures": []`; `PASS (structural): 6209 test cases dispositioned; 5822 claim Rust coverage (93.77%).` | 0 |
| Coverage report | `node parity/tools/coverage-report.mjs` | `coverage.md written: 5822/6209 ported (93.77%)` | 0 |
| Documentation corpus | `cargo run --release -p lovelace-corpus` | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| Protocol fixtures | `node parity/tools/gate-fixtures.mjs` | `oracle: 20/20`, `rust: 20/20`, `PASS: oracle 20/20, rust 20/20` | 0 |
| CLI matrix | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16`, `PASS: every RUN-001..RUN-014 row behaves like the oracle` | 0 |

After regeneration every one of the six mapped ids was re-read from the **generated**
`parity/manifest.toml` and its `rust` test id was resolved against the measured
`parity/rust-tests.json` inventory: **6/6 resolve, 0 unresolved**. The manifest gate independently
claims 5,822 Rust tests, up exactly 6 from 5,816.

## 8. Residual

* 269 rows remain deferred: 57 measured differences (dominated by the dispatcher gap and the
  `dsp`/evalf/plot/series/arithmetic` semantic families) and 212 library-only rows over 74 APIs.
* 118 exclusions unchanged (user-delegated, reversible).
* No production change was made: the dispatcher gap, the abs rewrite, the series Laurent tail, the
  evalf structured-digit cap, the plot refusals and the evalir multi-parameter batch are all
  production work for a later lane; this round only measured them and mapped what the current tree
  already answers.
