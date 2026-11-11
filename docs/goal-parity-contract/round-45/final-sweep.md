# Round 45 — the final deferred sweep (three-way adjudication)

**Status: complete.** All 371 census rows that were deferred at the round-44 differential closure
were re-executed through both runners with the same `--eval` argv against oracle
`1cc28f255eca6029849f880949fc7be1d40ab8af` and `rust/target/release/lovelace-run.exe`.
Each row is now either mapped to a passing Rust test, deferred with a measured named family, or
deferred as a documented UNRUNNABLE block naming the missing Rust API. No production source,
`.cs` file or fixture was edited; the only code written is the test module
`rust/lovelace-run/src/round45_final_sweep.rs` (15 tests) plus the existing
`lovelace-symbolics::round39_residue::cubic_roots_satisfy_polynomial_symbolically` reused as the
cover for `DebugCubic`.

## 1. Result

| Classification | Rows | Disposition |
|---|---:|---|
| MATCH-NOW | 16 | ported to a Rust test and mapped |
| STILL-DIFFERENT | 52 | deferred with a measured family and field/values |
| UNRUNNABLE | 303 | deferred with the missing Rust API named |
| **Total adjudicated** | **371** | |

Manifest: **5,736 / 6,209 ported (92.38 %)** — up from 5,720 (92.12 %); deferred **371 → 355**;
excluded unchanged at **118**. Every one of the 355 deferred rows carries a family or API reason in
`parity/manifest-map.json` (326 of them had no curated entry before this round and were added).

## 2. Population and method

1. Extracted the 371 `disposition = "deferred"` ids from `parity/manifest.toml` and joined each to
   `parity/census.json` (project, class, method, case index, source file and line).
2. Read the C# case body at that line — the assertion is the contract — including the theory data
   rows for `UsageDocumentationTests` (README `lovelace`/`result` pairs), `SeriesLaurent…`,
   `RecursionBudgetComposition…`, `EvalirDigitArgumentContract…` and `StructuredRealExactness…`.
3. Ran the case's own script through both binaries with the differential's exact normalisation
   (`revision`, `elapsed`, `elapsedTime`, `timings` replaced; key presence is structural), using
   `--omit-functions` to keep the envelope readable and additional argv (`--plot-dir`,
   `--plot-file`, `--print-budget`) where the C# case used them.
4. Classified: full normalised envelope equality ⇒ MATCH-NOW; any exit-code/JSON/envelope
   difference ⇒ STILL-DIFFERENT with the first differing path and both values; a case whose
   contract is a C#-only library API with no Rust counterpart ⇒ UNRUNNABLE.
5. Ported every MATCH-NOW row whose C# assertion is observable through the runner (or already
   covered by an existing Rust test) and asserted the C# case's concrete values, not a replay of
   the Rust envelope.

## 3. MATCH-NOW — 16 mappings (15 new tests, 1 existing test)

All 15 new tests live in `rust/lovelace-run/src/round45_final_sweep.rs` and drive
`parity_batch::run_args` — the runner's own execute/envelope pair. The sixteenth row is covered by
the existing round-39 kernel test.

| Census row (class/method#case) | Rust test | C# contract asserted |
|---|---|---|
| `ArrayTests/CaptureState_ReportsArrayKind#0` | `round45_final_sweep::capture_state_reports_the_array_kind` | the captured variable m of [[1,2],[3,4]] publishes kind Array with a 2x2 structured shape. |
| `StructuredRealExactnessTests/TruncatedQuotientAtTheBudget_IsNotCertifiedExact#0` | `round45_final_sweep::truncated_quotient_at_the_budget_is_not_certified_exact` | setprecision(18); 1/1009 publishes 0.000991080277502477 with exact=false and no numerator/denominator. |
| `AbsSymbolicRoundTripTests/Diff_Of_Abs_Symbolic_Does_Not_Throw#0` | `round45_final_sweep::diff_of_abs_symbolic_carries_the_nonzero_guard` | diff(abs(x),x) publishes exactly the C# guarded piecewise and its canonical form. |
| `AcceptanceTests/Hessian_Computes#0` | `round45_final_sweep::hessian_computes_a_symmetric_matrix` | the Hessian of (x^2+y^2)*exp(-(x+y)) is a 2x2 matrix and its two mixed partials are the same symbolic node (canonical form included). |
| `CapabilitiesBuiltinTests/IntegrationRefusal_IsTypedStructuredAndClassLevel#0` | `round45_final_sweep::integration_refusal_is_typed_structured_and_class_level` | each of the three unevaluated integrals publishes both integration.unevaluated (class) and integration.no-closed-form (specific) top-level diagnostics, same reason on both, distinct per integrand. |
| `DebugCubic/Cubic_Roots_SatisfyPolynomial#0` | `round39_residue::cubic_roots_satisfy_polynomial_symbolically` | every root of x^3-1 substituted into the cubic and expanded is exactly 0 - the C# assertion, already covered by the round-39 kernel test. |
| `DxSemanticClosureTests/Language_SolveQuartic_ReportsUnevaluatedNote#0` | `round45_final_sweep::solve_quartic_reports_the_unevaluated_diagnostic` | solve(x^4+1==0,x) is a SolveResult Unevaluated whose single diagnostic is solve.unevaluated with the reference message. |
| `DxStructuredResultsTests/InvFull_ExposesDetCondition#0` | `round45_final_sweep::inv_full_exposes_the_determinant_condition` | inv_full([[x,0],[0,y]]) is MatrixInverseResult Solved with conditions [x*y != 0] typed Vector. |
| `HyperbolicBuiltinsAndNegativeIntegerPowersTests/ZeroToTheMinusOne_IsATypedRecoverableErrorThatNamesTheBase#0` | `round45_final_sweep::zero_to_the_minus_one_is_a_typed_recoverable_error_that_names_the_base` | 0^-1 is InvalidArgument/TypeMismatch, recoverable, message names the base and repeats in diagnostics; 1+1 still answers 2. |
| `SeriesLaurentAndBranchPointTests/Series_OfARemovableDenominator_IsUnchanged#0` | `round45_final_sweep::series_of_a_removable_denominator_is_unchanged` | series(1/(x-1),x,0,2) publishes exactly -x + O(x^2) - 1 with no sqrt(0) or /0 in the envelope. |
| `SolveCompletenessPairingTests/Unevaluated_IsNotComplete#0` | `round45_final_sweep::solve_full_unevaluated_is_not_complete` | solve_full(x^4+1==0,x) publishes (Unevaluated, complete=false, Completeness=Unknown). |
| `UsageDocumentationTests/DocumentedExample_MatchesEngine#61` | `round45_final_sweep::documented_example_61_matches_the_engine` | README example #61 solve_system publishes the documented typed SystemSolveResult byte for byte. |
| `UsageDocumentationTests/DocumentedExample_MatchesEngine#70` | `round45_final_sweep::documented_example_70_matches_the_engine` | README example #70 lower(optimize(...)) publishes the documented MathIR text byte for byte. |
| `UsageDocumentationTests/DocumentedExample_MatchesEngine#71` | `round45_final_sweep::documented_example_71_matches_the_engine` | README example #71 evalir(lower(...),[2],40) publishes 92 (Integer). |
| `UsageDocumentationTests/DocumentedExample_MatchesEngine#73` | `round45_final_sweep::documented_example_73_matches_the_engine` | README example #73 assumptions() publishes the documented AssumptionSet rendering. |
| `UsageDocumentationTests/DocumentedExample_MatchesEngine#99` | `round45_final_sweep::documented_example_99_matches_the_engine` | README example #99 linsolve_full(...).conditions publishes [x*y != 0] (Vector). |

## 4. STILL-DIFFERENT — 19 named families (52 rows)

Field paths are from `parity/diff/compare.mjs`'s normalised comparison. "Rust" is
`rust/target/release/lovelace-run.exe`; "oracle" is the frozen C# binary.

| Family | Rows | Measured field and values |
|---|---:|---|
| `dsp` | 7 | `filter(a,b,x)` / `filter(a,b,n)` exit 1 vs 0; `fft([])` exit 1 vs 0; `fft([1,1,1])` exit 0 vs 1; `fft([0,1,0,0])[1]+1` diagnostic prose differs |
| `arity-shape` | 5 | `min()` and `compile_full(...)` category `DomainError` vs `TypeMismatch`; `filter([1],[1])` `UnsupportedOperation` vs `TypeMismatch`; the registry sweeps inherit those rows |
| `evalf-precision` | 4 | `$.result.structured.value` of `evalf(pi,1000)` has 100 fractional digits vs 1000; `evalf(sin(pi(30)),40)` display `0.0000…5028…` vs `0`; `sin(0)` display `sin(0)` vs exact `0` |
| `plot-wire` | 4 | symbolic-plot refusal category `TypeMismatch` vs `DomainError`; `--plot-dir ""` exit 0 vs 1; `--plot-file` refusal names the OS error vs the requested path |
| `abs-rewrite` | 4 | `simplify_full(sqrt(x^2))` expression `sqrt(x^2)` vs `abs(x)`; `subs(abs(x),x,-3)` is `abs(-3)` vs `3` |
| `evalir-multi` | 4 | `evalir supports one free symbol.` vs the C# multi-parameter batch result/refusal messages |
| `series` | 4 | `series(1/x,x,0,2)` display `0` vs `x^-1 + O(x^2)`; `series(1/x^2,…)` `0` vs `x^-2 + O(x^2)`; `series(sin(x)/x,x,0,3)` `1 + O(x^3)` vs `1 - 1/6*x^2 + O(x^3)`; `series(sqrt(abs(x)),…)` `sqrt(abs(x))` vs `sqrt(x)` |
| `recursion` | 3 | 5 calls inside 120 nested `if`s exit 0 vs 1 (`DepthExceeded`); `f(85)` in a `for`/`while` body returns an error envelope vs the documented `0` |
| `broadcast` | 2 | `[[1],[2]] + [[10,20],[30,40]]` exit 1 vs 0; batched 3-D `matmul` exit 1 vs 0 |
| `variables` | 2 | `$.variables` length 2 vs 1: a function-local and a block-local assignment leak into the envelope |
| `capability-advertisement` | 2 | `plot.symbolic-expression` trigger category differs; `max(1,2)` and `sum(x,5)` answer in Rust vs refused in the oracle |
| `solve-partial` | 2 | the 2^8 system publishes `Solved/true/Complete` vs `Partial/false/Partial`; `linsolve` on a singular system exits 0 vs 1 |
| `evalir-digits` | 2 | `evalir(…, 2.0)` refused as “got the Real 2, which is integral” vs accepted as the integer `2` |
| `cancel` | 2 | `cancel((x^2+1)/(x-1))` is `x + 1` vs the unchanged quotient; `cancel_full(x/x)` exits 1 vs 0 |
| `parser` | 1 | `x = y = 5` is `ParseError` in both runners vs the reference parser accepting the right-associative assignment |
| `large-exponent` | 1 | `2^-100000` aborts the Rust runner (no envelope, exit -1) vs `1 / 2^100000` published exactly |
| `power-zero` | 1 | `0^(1/2)` refused vs the supported `0` |
| `solve-selfeq` | 1 | `solve_full(x == x, x)` differs (status/diagnostics text) from the oracle’s `Unevaluated` note |
| `registry-count` | 1 | `--functions` total 123 builtins vs 107 in the reference |

Ranking for the next production round (highest row yield first): `dsp` (7), `arity-shape` (5),
then the five 4-row families `evalf-precision`, `plot-wire`, `abs-rewrite`, `evalir-multi`,
`series`; `recursion` (3). The remaining ten families are single- or two-row closing gaps.

## 5. UNRUNNABLE — 303 rows, 82 missing APIs

These rows exercise C# library surfaces that the Rust workspace does not expose, so no runner
comparison could decide them. Each manifest row names its own API; grouped:

| Rows | Missing Rust API |
|---:|---|
| 20 | compiled lexer TokenKind enum (syntax::Tok has 8 variants; token.rs is dead) |
| 13 | falsification loop API (ObservationClassifier, FalsificationProbeSet, GateReport) |
| 10 | falsification probe API (FuzzIdentity, Evaluation regression harness) |
| 10 | fixed-point Cplx64/Cplx128 Convolve/ImpulseResponse/MovingAverage API |
| 10 | Real private fast-path internals (ReduceToTwoPi, WholeQuotient, Normalize, IsBelowDecimalGuard, ToInteger, TrySpecialAngle) |
| 9 | falsification registry gate API (RegistryFalsificationGate, GateReport) |
| 9 | RecordSchemas.For/Validate record-schema registry |
| 9 | Value widen/widen-pair API |
| 8 | Compilation.Compile / IrProgram multi-parameter compilation API |
| 8 | SymPy oracle session (SympyOracle.Require) |
| 8 | thread-scoped ExprContext/assumption scopes |
| 7 | IrProgram.Deserialize/Validate/Infer typing API |
| 7 | RationalFunctions.Cancel/CancelWithConditions and SymbolicMatrix inverse APIs |
| 7 | rewrite rule registry with preconditions and trace API |
| 7 | ScriptSource.ToSemicolonStatements source normaliser |
| 7 | System.Random corpus + Printing.CanonicalParse round-trip API |
| 6 | BuiltinDescriptor example corpus + HelpService descriptor enumeration |
| 6 | content/gcd/square-free/resultant/discriminant polynomial API |
| 6 | modus plugin registration and kernel dispatch API |
| 6 | SuiteEngine CaptureState, events and SetVariable/RemoveVariable API |
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
| 3 | DenseArray<Complex> construction/span API |
| 3 | Expr.IsExact leaf-rule API |
| 3 | Exprs hash-consing/reference-identity API |
| 3 | Exprs/SymbolicMatrix construction API (Hessian, interning identity) |
| 3 | HelpService.Function descriptor text |
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
| 1 | InterpreterBuiltinDescriptors/BuiltinDescriptor metadata API |
| 1 | Latex printer escaping over Exprs.Symbol names |
| 1 | Latex printer over Exprs.Complex construction |
| 1 | Lovelace.Representation crate (the C# case has an empty body and no observable) |
| 1 | modus plugin loading seam (an engine without the Dsp plugin registered) |
| 1 | modus plugin seam carrying EnumValue payloads |
| 1 | OperationTiming.ElapsedDisplay scale |
| 1 | pretty printer over Exprs.Complex construction |
| 1 | registry FunctionDefinition.MinArity/Variadic introspection |
| 1 | Solvers.Solve inverse-branch candidate verification API |
| 1 | SuiteEngine ComputationDecimalPlaces/DisplayDecimalPlaces + ProjectValue above the 1000-digit language cap |
| 1 | SymbolicsPlugin.ParameterDomainOf |
| 1 | syntax::Expr unary-plus AST node |

## 6. Verification (observed this round)

| Check | Observed |
|---|---|
| Rust suite / inventory | `crates=11 tests=4843 suiteExit=0 passed=4843 failed=0` |
| Manifest | `rows 6209: ported 5736, deferred 355, excluded 118; coverage 92.38 %` |
| Manifest gate | `PASS (structural): 6209 test cases dispositioned; 5736 claim Rust coverage (92.38 %).` failures: `[]` |
| Coverage report | `coverage.md written: 5736/6209 ported (92.38 %)` |
| Documentation corpus | `corpus: 214 passed, 0 failed, 214 total` |
| Protocol fixtures | `oracle: 20/20`, `rust: 20/20`, `PASS: oracle 20/20, rust 20/20` |
| CLI matrix | `CLI matrix: 16/16`, `PASS: every RUN-001..RUN-014 row behaves like the oracle` |
| Differential, generated | 200 executed, 200 match, 0 mismatch, 0 error |
| Differential, replay | 1,758 executed, 1,758 match, 0 mismatch, 0 error |
| Formatting | `cargo fmt --check` exit 0 |
| Lints | `cargo clippy --workspace --all-targets -- -D warnings` exit 0 |
| Mutation gate | 12/12 at the round-44 commit `1b66318`; not re-run this round (test-only change) |

## 7. Notes and limits

- **Correctness over count.** Only full normalised-envelope equality was accepted as MATCH-NOW. A
  row whose asserted field matched but whose envelope differed elsewhere (for example
  `ExactControls_AreUntouched`, where the `evalf` halves now match but `sin(0)` does not) stays
  in its measured family rather than being claimed.
- **No assertion was weakened.** The 15 new tests state the same typed/display/canonical strings,
  record types, statuses, diagnostics, exit codes and recovery the C# cases state. Where a C#
  assertion is a non-wire library call inside an otherwise matching row (`RecordSchemas.Validate`,
  `HelpService.Function`, `CaptureState` function snapshots) the row is UNRUNNABLE, not MATCH-NOW.
- **DebugCubic** was already covered by the round-39 kernel test; it is mapped rather than
  duplicated.
- The 82 UNRUNNABLE APIs are not a recommendation to port every C# internal: they are the honest
  blocks for the next cycle's ranking. The top ones by row count are the lexer `TokenKind` (20), the
  falsification harness (32 across three APIs), fixed-point DSP (10), Real fast-path internals (10),
  and the record-schema registry (9).
