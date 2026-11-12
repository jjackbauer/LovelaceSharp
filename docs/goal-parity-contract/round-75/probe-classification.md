# Round 75 — deferred-row probe classification (UNGATED)

> **UNGATED — NO GATE CLAIM.** This report does **not** claim a gate-observed coverage
> number. `parity/gen-rust-tests.ps1` and `parity/tools/gate-manifest.mjs` were **not**
> run, and `parity/manifest.toml` was **not** regenerated. Every count below is arithmetic over
> the curated `parity/manifest-map.json`, labelled **UNGATED**. The workspace suite is red for
> nine `lovelace-desktop` `execution::tests` owned by another session, so no green-gate claim
> is possible or made. The added tests were run in their own packages (§7).

**Status: classification complete.** The **139** rows still deferred after the five round-74
arity mappings were re-adjudicated. **3 are MATCH-NOW and are now ported and mapped**; **41 are
STILL-DIFFERENT** with an exact field and both values; **95 are UNRUNNABLE** with the missing
Rust API named. The map gains **+3 mappings**.

Measured 2026-11-12 (America/Sao_Paulo) against the pinned oracle
`1cc28f255eca6029849f880949fc7be1d40ab8af` and `rust/target/release/lovelace-run.exe`.

## 1. Three-way result and the map delta

| Classification | Rows | Disposition |
|---|---:|---|
| MATCH-NOW | 3 | ported to 3 new tests and mapped |
| STILL-DIFFERENT | 41 | deferred with the measured field and both values |
| UNRUNNABLE | 95 | deferred with the missing Rust API named |
| **Total** | **139** | 144 manifest-deferred rows minus the 5 round-74 arity ids |

The 5 excluded ids are exactly the round-74 arity mappings, already `ported` in the map and
still `deferred` in the un-regenerated `manifest.toml`:
`ArityPropertyTests/EveryBuiltin_Refuses…`, `ArityValidationTests/TooFewArguments…`,
`BuiltinArgumentShapeSweepTests/EveryWrongShapedArgument…` (3-shape and wide-lattice), and
`CallSignatureDiagnosticsTests/EveryArityRefusalAgrees…`. They are not counted in the 139:

```text
Lovelace.Run.Tests/Lovelace.Run.Tests.ArityPropertyTests/EveryBuiltin_RefusesEveryCountOutsideItsDeclaredArity_WithTheOneDocumentedShape#0
Lovelace.Run.Tests/Lovelace.Run.Tests.ArityValidationTests/TooFewArguments_IsATypedStructuralError_NeverAnInternalFailure#0
Lovelace.Run.Tests/Lovelace.Run.Tests.BuiltinArgumentShapeSweepTests/EveryWrongShapedArgument_IsNeverAnInternalFailure_AndEveryArgumentRefusalNamesTheBuiltinAndThePosition#0
Lovelace.Run.Tests/Lovelace.Run.Tests.BuiltinArgumentShapeSweepTests/EveryWrongShapedArgument_OverAWiderShapeLattice_IsStillNeverAnInternalFailure#0
Lovelace.Run.Tests/Lovelace.Run.Tests.CallSignatureDiagnosticsTests/EveryArityRefusalAgreesWithTheRegistryEntry#0
```

| State | Ported | Deferred | Excluded | Arithmetic coverage |
|---|---:|---:|---:|---|
| Gate-observed baseline (round 74; not re-run here) | 5,947 | 144 | 118 | 5,947 / 6,209 = 95.78 % |
| Curated map **before** this round | 5,952 | 139 | 118 | 5,952 / 6,209 = 95.86 % |
| Curated map **after** this round | **5,955** | **136** | 118 | **5,955 / 6,209 = 95.9092 % → 95.91 % (UNGATED)** |

- **Map delta: +3** (`deferred` → `ported`), 0 keys added, 0 removed, 0 changed outside the
  three targets. Map SHA-256: `0429838b…4284` → `a953fc41…bd8b`; 6,209 keys, 3,643,446 bytes.
- The arithmetic is honest only in the map: `manifest.toml` still carries **5,947 ported /
  144 deferred** because it was not regenerated. The 5 arity mappings plus these 3 are the
  8-row difference between the map (5,955) and the un-regenerated manifest (5,947). No coverage
  number is claimed from that file or from the gate.

## 2. What was run

1. Extracted the 144 `disposition = "deferred"` ids from `parity/manifest.toml`, joined each to
   `parity/census.json` (project, class, method, case index, source file and line), and excluded
   the 5 arity ids. That is the 139-row population.
2. Read the C# case body at the census line. For a theory row, read the data row that selects the
   case index. The C# assertion, not the Rust output, is the contract.
3. Ran the case's own script through **both** binaries:
   `dotnet exec Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll --file <script>
   --omit-functions` and `rust/target/release/lovelace-run.exe --file <script>
   --omit-functions`, with `--eval` for one-line probes and `--cancel-after` /
   `--function-limit` where the row used them. Scripts were written as `.ls` files under
   `parity/.tmp/scripts` to avoid shell quoting.
4. Normalised the envelopes exactly as the differential does (`revision`, `elapsed`,
   `elapsedTime`, `timings` replaced at any depth; key presence structural) and recorded the
   first differing path with both values. A timeout is a difference, never a pass.
5. Classified: every assertion of the C# body reproduced ⇒ MATCH-NOW; any exit-code/JSON/envelope
   difference ⇒ STILL-DIFFERENT with the field; a contract whose only carrier is a C#-library
   object with no Rust counterpart ⇒ UNRUNNABLE, with the API named and checked against the Rust
   tree.
6. **100 script invocations** across the 41 pre-classified STILL-DIFFERENT rows produced **64
   full-envelope matches**, **33 differences and 3 timeouts**. A row is MATCH-NOW only when every
   probe its own body drives matched; a matching individual probe was not enough (see the
   capability row in §4).
7. The remaining 98 rows are library-only. Each named API was checked against the current Rust
   tree. One of them now passes and is MATCH-NOW (`factor-metamorphic`), and two moved from
   UNRUNNABLE to STILL-DIFFERENT because a runner-observable equivalent still differs
   (`integrate-conditions`, `exactness-boundary`). The other 95 keep their named API. The other
   two MATCH-NOW rows (`assumptions` and the exactness-best-effort capability row) came from the
   41 pre-classified STILL-DIFFERENT rows, not from this 98.

Commands used (no parity generator or gate):

```powershell
$repo = 'C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME = "$repo\.toolchain\rustup"; $env:CARGO_HOME = "$repo\.toolchain\cargo"
$env:PATH = "$repo\.toolchain\cargo\bin;" + $env:PATH
$env:TMP = "$repo\rust\target\tmp"; $env:TEMP = $env:TMP
Set-Location $repo

# oracle and runner, per script
dotnet exec Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll --file <script> --omit-functions
rust/target/release/lovelace-run.exe --file <script> --omit-functions

# package tests for the added tests (§7)
Set-Location "$repo\rust"
cargo test -p lovelace-symbolics round75_probe
cargo test -p lovelace-run round75_probe
```

## 3. MATCH-NOW — 3 rows, 3 new tests

| Census row | Rust test | Evidence |
|---|---|---|
| `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.AssumptionSoundnessTests/PropertyAndRelationAtoms_Contradict#0` | `lovelace-symbolics::round75_probe::property_and_relation_atoms_contradict_for_the_five_pairs` | The C# body's four property-then-relation pairs (Positive/Le 0, Negative/Ge 0, NonNegative/Lt 0, NonZero/Eq 0) and the reverse Le-0-then-Positive order all return `Err(Contradiction)` from `AssumptionSet::add`. Focused run: 1 passed. |
| `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.FactorMetamorphicTests/Factor_ExpandsBackToItsInput_OverTheWholeCorpus#0` | `lovelace-run::round75_probe::factor_expands_back_to_its_input_over_the_whole_corpus` | The C# 18-polynomial corpus (nine negative-leading) driven through the runner's `factor`/`expand`: all 18 `expand(factor(p))` envelopes and all 18 `expand(p)` envelopes match the oracle, and the canonical forms are equal for every polynomial. Focused run: 2 passed in the module. |
| `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.CapabilitiesBuiltinTests/ExactnessBestEffort_IsBackedByLiveRefusalsTheListDoesNotCarry#0` | `lovelace-run::round75_probe::exactness_best_effort_is_backed_by_live_refusals_the_list_does_not_carry` | `capabilities().exactness = BestEffort`; `mean(1)`, `max(1,2)`, `sum(x,5)`, `1 +`, `0^-1` are each refused with the oracle's code/category/message and are absent from the 19 advertised unsupported operations; at least one is `InvalidArgument/TypeMismatch`, none is `InternalError/InternalInvariantFailure`. Focused run: 2 passed in the module. |

The three tests live in the new `round75_probe.rs` module of each package (one symbolics test,
two runner tests). Each test passes in its own package run (§7). No assertion was weakened: the
assumption test drives all five pairs, the factor test drives all 18 corpus members, and the
capability test drives all five refusals.

## 4. STILL-DIFFERENT — 41 rows, 22 families

The largest measured families are `abs-rewrite` (4), `evalf-precision` (4), `evalir-multi`
(4) and `series` (4), followed by `recursion` (3). Exact fields and values:

- **evalf-precision (4)**: `evalf(pi,1000)` publishes **1,000** fractional digits in the oracle
  and **100** in Rust; `sin(0)` displays **"0"** vs **"sin(0)"**; `evalf(sin(pi(30)),40)`
  displays **"0"** vs **"0.0000000000000000000000000000005028841971"**.
- **series (4)**: `$.result.display` — `series(1/x,x,0,2)` **"x^-1 + O(x^2)"** vs **"0"**;
  `1/x^2` **"x^-2 + O(x^2)"** vs **"0"**; `sin(x)/x` **"1 - 1/6*x^2 + O(x^3)"** vs
  **"1 + O(x^3)"**; `sqrt(abs(x))` **"sqrt(x)"** vs **"sqrt(abs(x))"**.
- **evalir-multi (4)**: `compile_full(x+y,[x,y])` `result_type` **unknown** vs **Integer**;
  `evalir_batch(compile(x*y+1,[x,y]), [1,2,3], 40)` oracle **"evalir_batch(): kernel expects 2
  parameters per row; got 3 values, which is not a whole number of rows."** vs Rust exit 1
  **"evalir supports one free symbol."**; the row-shaped and width-mismatch probes exit **0 vs 1**.
- **evalir-digits (2)**: `evalir(compile(x*1/3, [x]), [1], 2.0)` exits **0** in the oracle (2.0 is
  normalized to the integer 2 and answers 0.33) vs **1 InvalidArgument/TypeMismatch** in Rust
  ("evalir(): argument 3 (digits) must be a Natural or Integer; got the Real 2, which is integral
  — write it as 2."); the batch form is the same.
- **abs-rewrite (4)**: `simplify_full(sqrt(x^2))` oracle **expression "abs(x)", changed True,
  conditions [DomainCondition(real)]** vs Rust **"sqrt(x^2)", changed False, conditions []**;
  `subs(abs(x),x,-3)` **"3"** vs **"abs(-3)"**.
- **recursion (3)**: five calls inside 120 nested `if`s exit **1 DepthExceeded** in the oracle
  vs **0** in Rust; `f(85)` in a `for`/`while` body returns **Natural 0** vs a result missing
  from the refusal envelope.
- **broadcast (2)**: `[[1],[2]] + [[10,20],[30,40]]` exit **0 vs 1**; batched 3-D
  `matmul(a,a)` exit **0 vs 1**.
- **cancellation (2)**: `--cancel-after 1500` on `2000000!` and `2^1000000000` — the oracle
  settles with a cancellation envelope; Rust **does not terminate within 35 s** (measured twice).
- **variables (2)**: a function-local `local = 42` and a block-local `{ z = 99 }` leak into
  `$.variables`: length **1 vs 2**.
- **solve-partial (2)**: `linsolve([[x,x],[x,x]],[1,1])` exits **1** in the oracle (the
  convenience projection throws) vs **0** in Rust while `linsolve_full` matches; the 2^8 system
  publishes **Partial/false/Partial** vs **Solved/true/Complete**.
- Singles: `cancel((x^2+1)/(x-1))` **"(x^2 + 1)/(x - 1)"** vs **"x + 1"**; `x = y = 5` exit
  **0 vs 1 ParseError**; `0^(1/2)` **"0"** vs exit **1**; `solve_full(x == x, x)` diagnostic
  **"0 = 0: every value is a solution."** vs **"No solver for this structure."**; `latex`
  derivative **\operatorname{diff}** vs **\operatorname{derivative}**; `help::overview()`
  lacks **"help <function>"**; `registry::suggest("fft")` returns **["fft"]** instead of empty;
  `2^-100000` is exact in the oracle and Rust **times out at 35 s**; MathIR differs by **1 ulp in
  the 50th decimal**; the two rows moved from UNRUNNABLE are `integrate-conditions` (oracle
  `SolvedConditional` with `[x > 0]` vs Rust `SolvedExact` with `[]`) and
  `exactness-boundary` (`x^1.5`: exact `3/2` power vs `Pow(Sym x, Real 1.5)`, exact=false).

### 4.1 The capability row that is still red

`CapabilitiesBuiltinTests.AdvertisedCodesAndCategories_MatchTheLiveEnvelope` has **6 of 19**
advertised entries still diverging. The plot entries are now fixed (the round-70/71 work); the
six remaining messages are
`solve.non-symbolic-variable`, `solve.non-symbolic-expression`, `integrate.non-symbolic-variable`,
`limit.non-symbolic-variable`, `dsp.symbolic-element` (`'SymbolExpr'` vs `'Symbolic'`) and
`solve.unevaluated` (`0 = 0: every value is a solution.` vs `No solver for this structure.`).
A single row cannot be mapped while any advertised entry fails.

## 5. UNRUNNABLE — 95 rows, 47 named API blocks

The largest remaining API block is the **SymPy oracle session** (8 rows,
`SympyOracle.Require`), followed by six 4-row blocks: assumption-scoped numeric
evaluation, `Evaluation.EvaluateToNum/EvaluateToExpr` with a throwing evaluator, the
`EvaluationException` vocabulary, `StructuralEquality.FirstDifference`, the
`SuiteEngine` CaptureState/events/SetVariable API, and the missing
`SymbolicMatrix` inverse/rank/solve variants. Five 3-row blocks follow
(`DenseArray<Complex>`, `Integer.Pow(Integer)` refusal semantics,
`Integration.IntegrateResult` + `Evaluation` construction, `SuiteEngine`
`PlotFileWriteException`, and Sum/Product/Sequence signal composition), then thirteen
2-row blocks and 22 single-row APIs.

Of the 98 library-only rows, one became MATCH-NOW (`factor-metamorphic`) and two were measured
at a Rust surface and moved to STILL-DIFFERENT (`integrate-conditions`, `exactness-boundary`).
The 95 that remain have no Rust counterpart for the named contract. They are not approximations
and no weaker test is offered.

## 6. Ranked remaining table (69 families)

Ties are ordered by class then family. The "Representative rows" column gives the first two exact
census ids (project prefix shortened); the appendix (§9) has all 139 exact ids.

| Rank | Rows | Class | Family / API | Representative rows | Measured field / missing API |
|---:|---:|---|---|---|---|
| 1 | 8 | UNRUNNABLE | SymPy oracle session (SympyOracle.Require) | `Symbolics.DifferentialOracleTests/SympyOracle_Derivatives_Agree#0`<br>`Symbolics.DifferentialOracleTests/SympyOracle_Factorization_Agrees#0` | SymPy oracle session (SympyOracle.Require) |
| 2 | 4 | STILL-DIFFERENT | abs-rewrite | `Symbolics.AbsSymbolicRoundTripTests/Abs_Of_Symbol_Matches_The_Rewriters_Own_Abs_Node#0`<br>`Symbolics.AbsSymbolicRoundTripTests/Simplify_Sqrt_Square_Under_Real_Is_Still_Abs_X#0` | simplify_full(sqrt(x^2)): oracle expression abs(x), changed True, conditions [DomainCondition(real)] vs rust sqrt(x^2), changed False, conditions []; subs(abs(x),x,-3) 3 vs abs(-3); abs(x) itself matches. |
| 3 | 4 | STILL-DIFFERENT | evalf-precision | `Run.EvalfDigitCapHonestyTests/ARequestAtTheCap_IsNotMarked#0`<br>`Run.StructuredRealPrecisionTests/EvalfCountAboveTheAmbientDisplayPrecision_CrossesInFull#0` | evalf(pi,1000) $.result.structured.value: oracle 1000 fractional digits vs rust 100; the other three evalf-precision rows add sin(0) display 0 vs sin(0) and evalf(sin(pi(30)),40) 0 vs 0.0000000000000000000000000000005028841971. |
| 4 | 4 | STILL-DIFFERENT | evalir-multi | `Symbolics.DxStructuredResultsTests/EvalirBatch_AcceptsNestedRowValues#0`<br>`Symbolics.DxStructuredResultsTests/EvalirBatch_FlatInputNotAWholeNumberOfRows_IsRejected#0` | compile_full(x+y,[x,y]) result_type: oracle "unknown" vs rust "Integer"; evalir_batch(compile(x*y+1,[x,y]), [1,2,3], 40) oracle "kernel expects 2 parameters per row; got 3 values..." vs rust exit 1 "evalir supports one free symbol." |
| 5 | 4 | STILL-DIFFERENT | series | `Symbolics.SeriesLaurentAndBranchPointTests/Series_AcrossAPoleOrABranchPoint_PublishesTheExpansion#0`<br>`Symbolics.SeriesLaurentAndBranchPointTests/Series_AcrossAPoleOrABranchPoint_PublishesTheExpansion#1` | $.result.display: series(1/x,x,0,2) oracle "x^-1 + O(x^2)" vs rust "0"; 1/x^2 oracle "x^-2 + O(x^2)" vs "0"; sin(x)/x oracle "1 - 1/6*x^2 + O(x^3)" vs "1 + O(x^3)"; sqrt(abs(x)) oracle "sqrt(x)" vs "sqrt(abs(x))" |
| 6 | 4 | UNRUNNABLE | assumption-scoped numeric evaluation (EvaluateToNum with caller assumptions) | `Symbolics.PiecewiseAssumptionTests/MirroredSpelling_IsAlsoDecided#0`<br>`Symbolics.PiecewiseAssumptionTests/ProvenGuard_TakesTheBranch#0` | assumption-scoped numeric evaluation (EvaluateToNum with caller assumptions) |
| 7 | 4 | UNRUNNABLE | Evaluation.EvaluateToNum/EvaluateToExpr with a throwing evaluator | `Symbolics.ExpectedFailureBoundaryTests/EvaluateToExpr_ApproximateArgument_ThrowingEvaluatorPropagates#0`<br>`Symbolics.ExpectedFailureBoundaryTests/EvaluateToExpr_ExactArgument_ThrowingEvaluatorPropagates#0` | Evaluation.EvaluateToNum/EvaluateToExpr with a throwing evaluator |
| 8 | 4 | UNRUNNABLE | EvaluationException vocabulary on the evaluator API | `Symbolics.DomainFailureVocabularyTests/DivisionByZero_IsAnEvaluationFailure#0`<br>`Symbolics.DomainFailureVocabularyTests/DivisionByZero_OnTheRealPath_IsAnEvaluationFailure#0` | EvaluationException vocabulary on the evaluator API |
| 9 | 4 | UNRUNNABLE | StructuralEquality.FirstDifference path reporting | `Suite.StructuralEqualityTests/DifferenceInANestedField_IsNamedByPath#0`<br>`Suite.StructuralEqualityTests/DifferentKinds_AreReportedByKind#0` | StructuralEquality.FirstDifference path reporting |
| 10 | 4 | UNRUNNABLE | SuiteEngine CaptureState, events and SetVariable/RemoveVariable API | `Suite.SuiteEngineTests/CaptureState_IsImmutable#0`<br>`Suite.SuiteEngineTests/FunctionDefined_GivenDefinition_RaisesEvent#0` | SuiteEngine CaptureState, events and SetVariable/RemoveVariable API |
| 11 | 4 | UNRUNNABLE | SymbolicMatrix inverse/rank/solve/solve_with_conditions API (only inverse_with_conditions exists) | `Symbolics.MatrixAdvanceTests/Inverse_Invertible_BehaviorPreserved#0`<br>`Symbolics.MatrixAdvanceTests/InverseWithConditions_NonSingular_MatchesInverse_And_CarriesCondition#0` | SymbolicMatrix inverse/rank/solve/solve_with_conditions API (only inverse_with_conditions exists) |
| 12 | 3 | STILL-DIFFERENT | recursion | `Run.RecursionDepthEnvelopeTests/NestedBlocksInsideARecursiveBody_AreRefused_InsteadOfDying#0`<br>`Suite.RecursionBudgetCompositionTests/TheDeepestAcceptedCall_Answers_InEveryComposition#14` | 5 calls inside 120 nested ifs: oracle exit 1 DepthExceeded vs rust exit 0; f(85) in a for/while body: oracle Natural 0 vs rust result missing (refusal envelope). |
| 13 | 3 | UNRUNNABLE | DenseArray<Complex> construction/span API | `Complex.DenseArrayComplexTests/DenseArrayOfComplex_GivenConstruction_ReportsComplexDTypeAndElementType#0`<br>`Complex.DenseArrayComplexTests/DenseArrayOfComplex_GivenContiguous_AsSpanIterates#0` | DenseArray<Complex> construction/span API |
| 14 | 3 | UNRUNNABLE | Integer.Pow(Integer) refusal semantics (Rust pow takes u64) | `Integer.IntegerPowTests/Pow_GivenNegativeExponent_Throws#0`<br>`Integer.IntegerPowTests/Pow_GivenZeroBase_Throws#0` | Integer.Pow(Integer) refusal semantics (Rust pow takes u64) |
| 15 | 3 | UNRUNNABLE | Integration.IntegrateResult / Evaluation / Exprs construction API | `Symbolics.RelationsAndFamiliesTests/KleeneEvaluation_AndOrNot#0`<br>`Symbolics.RelationsAndFamiliesTests/LogicalCanonicalization#0` | Integration.IntegrateResult / Evaluation / Exprs construction API |
| 16 | 3 | UNRUNNABLE | SuiteEngine PlotFileWriteException typed refusal | `Suite.PlotFileWriteRefusalTests/EmptyPlotFile_IsRefusedByType#0`<br>`Suite.PlotFileWriteRefusalTests/PlotFileUnderAMissingDirectory_IsRefusedByType#0` | SuiteEngine PlotFileWriteException typed refusal |
| 17 | 3 | UNRUNNABLE | Sum/Product/Sequence signal composition API (complex arrays refuse +/*) | `Dsp.DspTests/Product_GivenImpulseAndStep_MultipliesPointwise#0`<br>`Dsp.DspTests/Sequence_GivenLengthMismatch_ThrowsArgumentException#0` | Sum/Product/Sequence signal composition API (complex arrays refuse +/*) |
| 18 | 2 | STILL-DIFFERENT | broadcast | `Array.NdArrayTests/MatMul_Batched#0`<br>`Suite.BroadcastTests/ColumnVector_BroadcastsToMatrix#0` | [[1],[2]] + [[10,20],[30,40]]: oracle exit 0 vs rust exit 1; batched 3-D matmul(a,a): oracle exit 0 vs rust exit 1. |
| 19 | 2 | STILL-DIFFERENT | cancellation | `Suite.CancellationObservationTests/Evaluate_GivenLongFactorialAndShortBudget_CancelsPromptly#0`<br>`Suite.CancellationObservationTests/Evaluate_GivenLongNaturalPowerAndShortBudget_CancelsPromptly#0` | --cancel-after 1500 on 2000000! and 2^1000000000: the oracle settles with a cancellation envelope; the rust runner does not terminate within 35 s (measured twice). |
| 20 | 2 | STILL-DIFFERENT | evalir-digits | `Symbolics.EvalirDigitArgumentContractTests/AnExactlyIntegralRealLiteral_IsTheIntegerItDenotes#0`<br>`Symbolics.EvalirDigitArgumentContractTests/AnExactlyIntegralRealLiteral_IsTheIntegerItDenotes#1` | evalir(compile(x*1/3,[x]), [1], 2.0): oracle exit 0 (2.0 normalized to 2, answers 0.33) vs rust exit 1 InvalidArgument/TypeMismatch "got the Real 2, which is integral — write it as 2."; batch form same. |
| 21 | 2 | STILL-DIFFERENT | solve-partial | `Symbolics.DxStructuredResultsTests/LinsolveFull_SingularSystem_ReportsStatusNotThrow#0`<br>`Symbolics.SolveCompletenessPairingTests/EverySolveStatus_PublishesTheDocumentedPair_OnBothRecords#0` | linsolve([[x,x],[x,x]],[1,1]): oracle exit 1 (the convenience projection throws) vs rust exit 0; the 2^8 system publishes Partial/false/Partial vs Solved/true/Complete. |
| 22 | 2 | STILL-DIFFERENT | variables | `Suite.FunctionTests/Evaluate_GivenFunctionLocalVariable_DoesNotLeakToGlobal#0`<br>`Suite.StatementTests/Evaluate_GivenVariableAssignmentInsideBlock_DoesNotLeak#0` | func f() { local = 42 }; f() and { z = 99 }: oracle $.variables length 1 vs rust 2 (the function-local/block-local leaks). |
| 23 | 2 | UNRUNNABLE | Domains.DomainOf / Solvers.Solve internals | `Symbolics.DxSemanticClosureTests/DomainOf_NegativeIntegerPower_IsRationalNotInteger#0`<br>`Symbolics.DxSemanticClosureTests/Solve_QuarticNoRealRoots_DefaultDomain_ReportsUnevaluated#0` | Domains.DomainOf / Solvers.Solve internals |
| 24 | 2 | UNRUNNABLE | Expr.IsExact leaf rule over a corpus with RootOf/Complex leaves | `Symbolics.ExactnessFlagTests/CachedFlag_EqualsTheLeafRule_OverTheWholeCorpus#0`<br>`Symbolics.ExactnessFlagTests/SolverSolutionExactness_AndValueExact_Agree#0` | Expr.IsExact leaf rule over a corpus with RootOf/Complex leaves |
| 25 | 2 | UNRUNNABLE | Exprs.Real(RealLiteral) provenance and NumReal payload API | `Complex.ComplexMathInexactArgumentProvenanceTests/SymbolicNodeAndNumericResult_AgreeForSine#0`<br>`Complex.ComplexMathInexactArgumentProvenanceTests/SymbolicNodeAndNumericResult_AgreeForTheSameExpression#0` | Exprs.Real(RealLiteral) provenance and NumReal payload API |
| 26 | 2 | UNRUNNABLE | Integer.TryFormat(Span<char>) fallible formatting API | `Integer.IntegerToStringTests/TryFormat_GivenInsufficientBuffer_ReturnsFalse#0`<br>`Integer.IntegerToStringTests/TryFormat_GivenSufficientBuffer_WritesRepresentationAndReturnsTrue#0` | Integer.TryFormat(Span<char>) fallible formatting API |
| 27 | 2 | UNRUNNABLE | Printing.PrintOptions/PrintBudget/PrintOutcome API (library max depth / node budget) | `Symbolics.PrintBudgetTests/DepthBudget_ReportsDepthLimit#0`<br>`Symbolics.PrintBudgetTests/NodeBudget_OnACompactRendering_DropsNothingAndSaysSo#0` | Printing.PrintOptions/PrintBudget/PrintOutcome API (library max depth / node budget) |
| 28 | 2 | UNRUNNABLE | Printing.PrintOptions/PrintOutcome token-boundary API | `Symbolics.PrintBudgetTokenBoundaryTests/AnIdentifierLongerThanTheAllowance_IsKeptWhole#0`<br>`Symbolics.PrintBudgetTokenBoundaryTests/ARenderingWithinTheAllowance_IsUntouched#0` | Printing.PrintOptions/PrintOutcome token-boundary API |
| 29 | 2 | UNRUNNABLE | public SolveCompletenessMapping.Of(status, completeness) API | `Symbolics.SolveCompletenessMappingTests/BudgetExceeded_IsIncomplete_WithTheKernelCompletenessOfThePartialSubset#0`<br>`Symbolics.SolveCompletenessMappingTests/EveryStatusCarriesTheDocumentedPair#3` | public SolveCompletenessMapping.Of(status, completeness) API |
| 30 | 2 | UNRUNNABLE | README/UsageExamples sync checker | `Symbolics.DocsSyncTests/Every_CSharpSnippet_AppearsInUsageExamples#0`<br>`Symbolics.DocsSyncTests/Every_LovelaceExample_HasAnExpectedResult#0` | README/UsageExamples sync checker |
| 31 | 2 | UNRUNNABLE | RealLiteral.FromRealExact/ToReal and NumReal payload API | `Symbolics.TruncatingEvaluationStaysInexactTests/FunctionOfATruncatedArgument_IsInexact#0`<br>`Symbolics.TruncatingEvaluationStaysInexactTests/LiteralRoundTrip_KeepsGenuinelyExactValuesExact#0` | RealLiteral.FromRealExact/ToReal and NumReal payload API |
| 32 | 2 | UNRUNNABLE | rewrite proof-obligation bridge API | `Symbolics.RewriteProofBridgeTests/EveryObligation_IsStatedAndHonestAboutItsStatus#0`<br>`Symbolics.RewriteProofBridgeTests/EveryShippedRule_HasExactlyOneProofObligation#0` | rewrite proof-obligation bridge API |
| 33 | 2 | UNRUNNABLE | SuiteEngine output writer / ResetPlotCapture API | `Suite.SuiteEngineOutputPlotTests/EvaluateAsync_GivenOutputWriter_CapturesPrintLinesToThatWriter#0`<br>`Suite.SuiteEngineOutputPlotTests/ResetPlotCapture_GivenPreviousPlot_ClearsLastPlot#0` | SuiteEngine output writer / ResetPlotCapture API |
| 34 | 2 | UNRUNNABLE | SymbolicMatrix.Solve with numeric substitution verification | `Symbolics.LinsolveCheckTests/Solve_3x3_VerifiesByNumericSubstitution#0`<br>`Symbolics.LinsolveCheckTests/Solve_VerifiesByNumericSubstitution#0` | SymbolicMatrix.Solve with numeric substitution verification |
| 35 | 2 | UNRUNNABLE | ValueFormatter depth guard API | `Suite.ValueDepthGuardTests/ADeepRecordChain_IsRefusedTheSameWay#0`<br>`Suite.ValueDepthGuardTests/ValueOnePastTheBudget_IsRefusedByTheStructuredProjection#0` | ValueFormatter depth guard API |
| 36 | 1 | STILL-DIFFERENT | cancel | `Symbolics.MetamorphicSetClosureTests/CancelBuiltin_ReducesAndRefusesToCancelWhenNoFactorIsCommon#0` | cancel((x^2+1)/(x-1)): oracle "(x^2 + 1)/(x - 1)" vs rust "x + 1" (x/x and (x^2-1)/(x-1) match) |
| 37 | 1 | STILL-DIFFERENT | capability-advertisement | `Symbolics.CapabilitiesBuiltinTests/AdvertisedCodesAndCategories_MatchTheLiveEnvelope#0` | 6 of 19 advertised entries differ: solve.non-symbolic-variable/expression, integrate.non-symbolic-variable and limit.non-symbolic-variable messages; dsp.symbolic-element `'SymbolExpr'` vs `'Symbolic'`; solve.unevaluated `0 = 0: every value is a solution.` vs `No solver for this structure.` The plot entries now match. |
| 38 | 1 | STILL-DIFFERENT | exactness-boundary | `Symbolics.ExactnessFlagTests/RadicalsOfExactBases_AreExact_AndApproximateBasesAreNot#0` | x^1.5: oracle canonicalizes the numeric exponent to the exact 3/2 (exact=true) vs rust Pow(Sym x, Real 1.5) (exact=false); the other seven radical/RootOf assertions agree. |
| 39 | 1 | STILL-DIFFERENT | help | `Suite.HelpServiceTests/Help_Overview_ListsAllCategories#0` | help::overview() lists the categories but not the "help <function>" line the C# row requires (measured on the Rust surface) |
| 40 | 1 | STILL-DIFFERENT | integrate-conditions | `Symbolics.RelationsAndFamiliesTests/Integrate_StructuredResult_ReportsConditions#0` | $.result.display: oracle IntegrationResult(status: SolvedConditional, expression: log(x), conditions: [x > 0], ...) vs rust status SolvedExact, conditions []; the x^5 and exp(x^2) probes match. |
| 41 | 1 | STILL-DIFFERENT | large-exponent | `Suite.StructuredRealExactnessTests/TerminatingQuotientBeyondTheDigitBudget_IsCertifiedExact#2` | 2^-100000: oracle publishes 1/2^100000 exactly; the rust process does not terminate within 35 s (no envelope). |
| 42 | 1 | STILL-DIFFERENT | latex | `Symbolics.LatexSymbolNameEscapingTests/LatexSymbolName_InsideLargerExpressions_IsStillEscaped#0` | latex(derivative(f, [x_1])): oracle "\\operatorname{diff}(f, x\\_1)" vs rust "\\operatorname{derivative}(f, x\\_1)"; the escaped product rendering matches |
| 43 | 1 | STILL-DIFFERENT | mathir-ulp | `Symbolics.PropertyTests/MathIR_EvaluatesLikeSymbolic#0` | MathIR_EvaluatesLikeSymbolic: numeric::evaluate and the lowered evaluation differ by 1 in the 50th decimal place on corpus expression 2 (round-62 measurement, unchanged) |
| 44 | 1 | STILL-DIFFERENT | name-suggestions | `Suite.NameSuggestionsTests/ANameNothingIsCloseTo_IsNotGuessedAt#0` | registry::suggest("fft") returns ["fft"] (the exact registered name) where the C# row requires an empty suggestion list; nosuchfn/push/zip are empty in both |
| 45 | 1 | STILL-DIFFERENT | parser | `Suite.ParserTests/Parse_GivenChainedAssignment_RightAssociative#0` | x = y = 5: oracle exit 0 (display 5) vs rust exit 1 ParseError "Expected ';' or end of input but found '=' at position 6." |
| 46 | 1 | STILL-DIFFERENT | power-zero | `Symbolics.CapabilitiesBuiltinTests/PowerExponentClass_RefusesWhatItSays_AndNamesWhatIsSupported#0` | 0^(1/2): oracle exit 0 display "0" vs rust exit 1 (refused); the other scope counterexamples (-4)^(1/2) etc. match |
| 47 | 1 | STILL-DIFFERENT | solve-note | `Symbolics.DiagnosticContractTests/KernelNotes_CrossAsDiagnosticMessages#0` | x = symbol("x"); solve_full(x == x, x) diagnostic message: oracle "0 = 0: every value is a solution." vs rust "No solver for this structure." (limit_full(a/x,x,0) matches) |
| 48 | 1 | UNRUNNABLE | AllocationRefusalException type | `Suite.ArrayAllocationRefusalTests/TheSuiteDeclaresTheAllocationRefusalTheRunnerClassifies#0` | AllocationRefusalException type |
| 49 | 1 | UNRUNNABLE | ArgumentContractException type | `Suite.DspIntegerArgumentContractTests/TheExactlyIntegralRealRefusal_IsStillTheContractsContract#0` | ArgumentContractException type |
| 50 | 1 | UNRUNNABLE | complex constant leaf (Exprs.Complex) for the model-level printer | `Symbolics.LatexPrinterTests/KnownRendering_PowerOfAComplexConstant_IsAlreadyAGroup#0` | complex constant leaf (Exprs.Complex) for the model-level printer |
| 51 | 1 | UNRUNNABLE | complex constant leaf (Exprs.Complex) for the pretty printer | `Symbolics.PrettyParenthesesTests/ModelLevel_PowerOfAComplexConstant_HasADelimitedBase#0` | complex constant leaf (Exprs.Complex) for the pretty printer |
| 52 | 1 | UNRUNNABLE | deep-expression AST construction and EvaluationException API | `Suite.RecursionDepthGuardTests/ADeepExpressionInsideTheRecursiveBody_IsRefused#0` | deep-expression AST construction and EvaluationException API |
| 53 | 1 | UNRUNNABLE | distinct Integer(Int)/Integer(Long) constructor overloads | `Integer.IntegerConstructorTests/Constructor_GivenInt_DelegatesToLongOverload#0` | distinct Integer(Int)/Integer(Long) constructor overloads |
| 54 | 1 | UNRUNNABLE | engine computation/display decimal-place projection API | `Run.LibraryProjectionAgreementTests/TheFacade_PublishesEveryStoredDigit_WhenTheDisplayBoundIsLower#0` | engine computation/display decimal-place projection API |
| 55 | 1 | UNRUNNABLE | ErrorCategory enum (the seven frozen members) | `Symbolics.DiagnosticContractTests/ErrorCategory_HasExactlyTheSevenFrozenMembers#0` | ErrorCategory enum (the seven frozen members) |
| 56 | 1 | UNRUNNABLE | ErrorCategory enum and the three mirrored taxonomy arrays | `Run.ErrorCategoryMirrorTests/EveryMirroredTaxonomy_EqualsTheEnum#0` | ErrorCategory enum and the three mirrored taxonomy arrays |
| 57 | 1 | UNRUNNABLE | golden-fixture recorder (TestSupport.Normalise + SourceGoldenPath) | `Run.GoldenFixtureRecorder/RecordGoldenFixtures#0` | golden-fixture recorder (TestSupport.Normalise + SourceGoldenPath) |
| 58 | 1 | UNRUNNABLE | Integer operator- operator overload | `Integer.IntegerSubtractTests/SubtractOperator_IsEquivalentToSubtractMethod#0` | Integer operator- operator overload |
| 59 | 1 | UNRUNNABLE | Integer operator* operator overload | `Integer.IntegerMultiplyTests/OperatorStar_ProducesSameResultAsMultiplyMethod#0` | Integer operator* operator overload |
| 60 | 1 | UNRUNNABLE | Integer reference-identity equality (Rust Int is a value) | `Integer.IntegerEqualityTests/Equals_GivenReferenceToSelf_ReturnsTrue#0` | Integer reference-identity equality (Rust Int is a value) |
| 61 | 1 | UNRUNNABLE | library builtin count (CaptureState().Functions builtin count) | `Symbolics.HyperbolicBuiltinsAndNegativeIntegerPowersTests/BuiltinCount_IncludesTheThreeHyperbolicFunctions#0` | library builtin count (CaptureState().Functions builtin count) |
| 62 | 1 | UNRUNNABLE | Lovelace.Representation crate (the C# case has an empty body and no observable) | `Representation.UnitTest1/Test1#0` | Lovelace.Representation crate (the C# case has an empty body and no observable) |
| 63 | 1 | UNRUNNABLE | OperationTiming.ElapsedDisplay/ElapsedScale API | `Suite.TimingScaleTests/OperationTiming_ExposesTheSameScaleAsItsDisplay#0` | OperationTiming.ElapsedDisplay/ElapsedScale API |
| 64 | 1 | UNRUNNABLE | public NameSuggestions.EditDistance API (registry::edit_distance is private) | `Suite.NameSuggestionsTests/EditDistance_IsTheStructuralMetric#3` | public NameSuggestions.EditDistance API (registry::edit_distance is private) |
| 65 | 1 | UNRUNNABLE | Real.Cos fast path meeting the 10 s budget at 1000 places (fast_path_tests.rs documents the miss) | `Real.RealTrigFastPathTests/Cos_AtDefaultPrecision_StaysInteractive#0` | Real.Cos fast path meeting the 10 s budget at 1000 places (fast_path_tests.rs documents the miss) |
| 66 | 1 | UNRUNNABLE | registry FunctionDefinition.MinArity/Variadic introspection | `Run.ArityPropertyTests/DeclaredArity_TravelsWithEveryPluginBuiltin#0` | registry FunctionDefinition.MinArity/Variadic introspection |
| 67 | 1 | UNRUNNABLE | Solvers.Solve inverse-branch candidate verification API | `Symbolics.InverseBranchCandidateVerificationTests/EveryRepresentedSolutionSatisfiesTheOriginalEquation#4` | Solvers.Solve inverse-branch candidate verification API |
| 68 | 1 | UNRUNNABLE | syntax::Expr unary-plus AST node (+5 parses to the literal) | `Suite.ParserTests/Parse_GivenUnaryPlus_ReturnsUnaryExprPlus#0` | syntax::Expr unary-plus AST node (+5 parses to the literal) |
| 69 | 1 | UNRUNNABLE | UnicodeOutput formatting toggle on the engine | `Suite.HelpServiceTests/UnicodeOutput_ThreadsThroughEngineFormatting#0` | UnicodeOutput formatting toggle on the engine |

## 7. Package test results

| Run | Command | Observed |
|---|---|---|
| Focused, symbolics | `cargo test -p lovelace-symbolics round75_probe` | **1 passed, 0 failed, 0 ignored** (175 filtered out) |
| Focused, runner | `cargo test -p lovelace-run round75_probe` | **2 passed, 0 failed, 0 ignored** (411 filtered out) |
| Full symbolics package | `cargo test -p lovelace-symbolics` | **176 passed, 0 failed, 0 ignored** |
| Full runner package | `cargo test -p lovelace-run` | **413 passed, 0 failed, 0 ignored** unit + **1 passed** recursion_depth integration |

The workspace suite was **not** used as a gate: it is red for nine `lovelace-desktop`
`execution::tests` in another session's untracked `execution.rs`. No desktop file was read or
edited by this round.

## 8. Scope, changed files and reproduction

Changed files:

- `parity/manifest-map.json` — exactly three `deferred` → `ported` entries.
- `rust/lovelace-symbolics/src/round75_probe.rs` — new (1 test) and
  `rust/lovelace-symbolics/src/lib.rs` — test-module declaration.
- `rust/lovelace-run/src/round75_probe.rs` — new (2 tests) and
  `rust/lovelace-run/src/main.rs` — test-module declaration.
- `docs/goal-parity-contract/round-75/probe-classification.md` — this report.

Working files only: probe lists, scripts and raw results under `parity/.tmp/`
(`round75-probes-*.json`, `round75-results-*.json`, `round75-*.mjs`, `round75-still-bodies.txt`).
No production `.rs` logic, no `.cs` file, no fixture, corpus file, `manifest.toml`,
`coverage.md` or `rust-tests.json` was edited; the only edits to existing Rust files are the two
`#[cfg(test)] mod` declarations. `rust/lovelace-desktop` and `docs/goal-desktop-v4` were not touched.

Reproduction of the map side:

```powershell
node parity/.tmp/round75-classify2.mjs   # 139 rows: 3 MATCH / 41 STILL / 95 UNRUN
node parity/.tmp/round75-tables2.mjs     # ranked table + appendix
```

**Gate confirmation: I did not run `gen-rust-tests`, `gate-manifest`, `coverage-report` or
`run-all`, I did not regenerate `manifest.toml`, and I make no gate or coverage claim. The only
coverage figure in this report is the explicitly labelled UNGATED map arithmetic in §1.**

## 9. Appendix — all 139 deferred rows

| # | Kind | Census row | Class/method | Family / measured field |
|---:|---|---|---|---|
| 1 | MATCH-NOW | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.AssumptionSoundnessTests/PropertyAndRelationAtoms_Contradict#0` | Symbolics.AssumptionSoundnessTests/PropertyAndRelationAtoms_Contradict#0 | AssumptionSet::add refuses all five property/relation pairs, reverse order included; new test passes. |
| 2 | MATCH-NOW | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.CapabilitiesBuiltinTests/ExactnessBestEffort_IsBackedByLiveRefusalsTheListDoesNotCarry#0` | Symbolics.CapabilitiesBuiltinTests/ExactnessBestEffort_IsBackedByLiveRefusalsTheListDoesNotCarry#0 | capabilities().exactness=BestEffort; the five unenumerated scripts are refused and not advertised; new test passes. |
| 3 | MATCH-NOW | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.FactorMetamorphicTests/Factor_ExpandsBackToItsInput_OverTheWholeCorpus#0` | Symbolics.FactorMetamorphicTests/Factor_ExpandsBackToItsInput_OverTheWholeCorpus#0 | 18/18 corpus polynomials: canonical(expand(factor(p))) == canonical(expand(p)); all 36 envelopes match the oracle; new test passes. |
| 4 | STILL-DIFFERENT | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.AbsSymbolicRoundTripTests/Abs_Of_Symbol_Matches_The_Rewriters_Own_Abs_Node#0` | Symbolics.AbsSymbolicRoundTripTests/Abs_Of_Symbol_Matches_The_Rewriters_Own_Abs_Node#0 | simplify_full(sqrt(x^2)): oracle expression abs(x), changed True, conditions [DomainCondition(real)] vs rust sqrt(x^2), changed False, conditions []; subs(abs(x),x,-3) 3 vs abs(-3); abs(x) itself matches. |
| 5 | STILL-DIFFERENT | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.AbsSymbolicRoundTripTests/Simplify_Sqrt_Square_Under_Real_Is_Still_Abs_X#0` | Symbolics.AbsSymbolicRoundTripTests/Simplify_Sqrt_Square_Under_Real_Is_Still_Abs_X#0 | simplify_full(sqrt(x^2)): oracle expression "abs(x)" changed True, conditions [DomainCondition(real)] vs rust expression "sqrt(x^2)" changed False, conditions [] |
| 6 | STILL-DIFFERENT | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.AbsSymbolicRoundTripTests/Subs_Abs_Symbolic_At_Negative_Value_Gives_Three#0` | Symbolics.AbsSymbolicRoundTripTests/Subs_Abs_Symbolic_At_Negative_Value_Gives_Three#0 | subs(abs(x),x,-3): oracle "3" vs rust "abs(-3)" |
| 7 | STILL-DIFFERENT | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.UnsatisfiablePropagationTests/SimplifyFull_DomainCondition_CarriesADomainValueNotText#0` | Symbolics.UnsatisfiablePropagationTests/SimplifyFull_DomainCondition_CarriesADomainValueNotText#0 | conditions of simplify_full(sqrt(x^2)): oracle [DomainCondition(variable x, domain real)] vs rust [] |
| 8 | STILL-DIFFERENT | `Lovelace.Array.Tests/Lovelace.Array.Tests.NdArrayTests/MatMul_Batched#0` | Array.NdArrayTests/MatMul_Batched#0 | [[1],[2]] + [[10,20],[30,40]]: oracle exit 0 vs rust exit 1; batched 3-D matmul(a,a): oracle exit 0 vs rust exit 1. |
| 9 | STILL-DIFFERENT | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.BroadcastTests/ColumnVector_BroadcastsToMatrix#0` | Suite.BroadcastTests/ColumnVector_BroadcastsToMatrix#0 | [[1],[2]] + [[10,20],[30,40]]: oracle exit 0 vs rust exit 1 |
| 10 | STILL-DIFFERENT | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.MetamorphicSetClosureTests/CancelBuiltin_ReducesAndRefusesToCancelWhenNoFactorIsCommon#0` | Symbolics.MetamorphicSetClosureTests/CancelBuiltin_ReducesAndRefusesToCancelWhenNoFactorIsCommon#0 | cancel((x^2+1)/(x-1)): oracle "(x^2 + 1)/(x - 1)" vs rust "x + 1" (x/x and (x^2-1)/(x-1) match) |
| 11 | STILL-DIFFERENT | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.CancellationObservationTests/Evaluate_GivenLongFactorialAndShortBudget_CancelsPromptly#0` | Suite.CancellationObservationTests/Evaluate_GivenLongFactorialAndShortBudget_CancelsPromptly#0 | --cancel-after 1500 on 2000000! and 2^1000000000: the oracle settles with a cancellation envelope; the rust runner does not terminate within 35 s (measured twice). |
| 12 | STILL-DIFFERENT | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.CancellationObservationTests/Evaluate_GivenLongNaturalPowerAndShortBudget_CancelsPromptly#0` | Suite.CancellationObservationTests/Evaluate_GivenLongNaturalPowerAndShortBudget_CancelsPromptly#0 | --cancel-after 1500 on 2^1000000000: the oracle settles with a cancellation envelope; the rust runner does not terminate within 35 s |
| 13 | STILL-DIFFERENT | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.CapabilitiesBuiltinTests/AdvertisedCodesAndCategories_MatchTheLiveEnvelope#0` | Symbolics.CapabilitiesBuiltinTests/AdvertisedCodesAndCategories_MatchTheLiveEnvelope#0 | 6 of 19 advertised entries differ: solve.non-symbolic-variable/expression, integrate.non-symbolic-variable and limit.non-symbolic-variable messages; dsp.symbolic-element `'SymbolExpr'` vs `'Symbolic'`; solve.unevaluated `0 = 0: every value is a solution.` vs `No solver for this structure.` The plot entries now match. |
| 14 | STILL-DIFFERENT | `Lovelace.Run.Tests/Lovelace.Run.Tests.EvalfDigitCapHonestyTests/ARequestAtTheCap_IsNotMarked#0` | Run.EvalfDigitCapHonestyTests/ARequestAtTheCap_IsNotMarked#0 | evalf(pi,1000) $.result.structured.value: oracle 1000 fractional digits vs rust 100; the other three evalf-precision rows add sin(0) display 0 vs sin(0) and evalf(sin(pi(30)),40) 0 vs 0.0000000000000000000000000000005028841971. |
| 15 | STILL-DIFFERENT | `Lovelace.Run.Tests/Lovelace.Run.Tests.StructuredRealPrecisionTests/EvalfCountAboveTheAmbientDisplayPrecision_CrossesInFull#0` | Run.StructuredRealPrecisionTests/EvalfCountAboveTheAmbientDisplayPrecision_CrossesInFull#0 | evalf(pi,1000) $.result.structured.value: oracle 1000 fractional digits vs rust 100 |
| 16 | STILL-DIFFERENT | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.EvalfTruncatedConstantExactnessTests/ExactControls_AreUntouched#0` | Symbolics.EvalfTruncatedConstantExactnessTests/ExactControls_AreUntouched#0 | sin(0) $.result.display: oracle "0" vs rust "sin(0)" |
| 17 | STILL-DIFFERENT | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.EvalfWorkingPrecisionHonoursRequestedDecimalsTests/ValuesBelowTheComputationCap_StayInexact#0` | Symbolics.EvalfWorkingPrecisionHonoursRequestedDecimalsTests/ValuesBelowTheComputationCap_StayInexact#0 | evalf(sin(pi(30)),40) $.result.display: oracle "0" vs rust "0.0000000000000000000000000000005028841971" |
| 18 | STILL-DIFFERENT | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.EvalirDigitArgumentContractTests/AnExactlyIntegralRealLiteral_IsTheIntegerItDenotes#0` | Symbolics.EvalirDigitArgumentContractTests/AnExactlyIntegralRealLiteral_IsTheIntegerItDenotes#0 | evalir(compile(x*1/3,[x]), [1], 2.0): oracle exit 0 (2.0 normalized to 2, answers 0.33) vs rust exit 1 InvalidArgument/TypeMismatch "got the Real 2, which is integral — write it as 2." |
| 19 | STILL-DIFFERENT | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.EvalirDigitArgumentContractTests/AnExactlyIntegralRealLiteral_IsTheIntegerItDenotes#1` | Symbolics.EvalirDigitArgumentContractTests/AnExactlyIntegralRealLiteral_IsTheIntegerItDenotes#1 | evalir_batch(compile(x*1/3,[x]), [1,2], 2.0): oracle exit 0 vs rust exit 1 InvalidArgument/TypeMismatch "got the Real 2, which is integral — write it as 2." |
| 20 | STILL-DIFFERENT | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DxStructuredResultsTests/EvalirBatch_AcceptsNestedRowValues#0` | Symbolics.DxStructuredResultsTests/EvalirBatch_AcceptsNestedRowValues#0 | compile_full(x+y,[x,y]) result_type: oracle "unknown" vs rust "Integer"; evalir_batch(compile(x*y+1,[x,y]), [1,2,3], 40) oracle "kernel expects 2 parameters per row; got 3 values..." vs rust exit 1 "evalir supports one free symbol." |
| 21 | STILL-DIFFERENT | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DxStructuredResultsTests/EvalirBatch_FlatInputNotAWholeNumberOfRows_IsRejected#0` | Symbolics.DxStructuredResultsTests/EvalirBatch_FlatInputNotAWholeNumberOfRows_IsRejected#0 | compile_full(x+y,[x,y]) result_type: oracle "unknown" vs rust "Integer"; evalir_batch(compile(x*y+1,[x,y]), [1,2,3], 40) oracle "kernel expects 2 parameters per row; got 3 values..." vs rust exit 1 "evalir supports one free symbol." |
| 22 | STILL-DIFFERENT | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DxStructuredResultsTests/EvalirBatch_RowOrientedInput_EvaluatesRowByRow#0` | Symbolics.DxStructuredResultsTests/EvalirBatch_RowOrientedInput_EvaluatesRowByRow#0 | compile_full(x+y,[x,y]) result_type: oracle "unknown" vs rust "Integer"; evalir_batch(compile(x*y+1,[x,y]), [1,2,3], 40) oracle "kernel expects 2 parameters per row; got 3 values..." vs rust exit 1 "evalir supports one free symbol." |
| 23 | STILL-DIFFERENT | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DxStructuredResultsTests/EvalirBatch_RowWidthMismatch_IsRejectedWithTheDocumentedShape#0` | Symbolics.DxStructuredResultsTests/EvalirBatch_RowWidthMismatch_IsRejectedWithTheDocumentedShape#0 | compile_full(x+y,[x,y]) result_type: oracle "unknown" vs rust "Integer"; evalir_batch(compile(x*y+1,[x,y]), [1,2,3], 40) oracle "kernel expects 2 parameters per row; got 3 values..." vs rust exit 1 "evalir supports one free symbol." |
| 24 | STILL-DIFFERENT | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.ExactnessFlagTests/RadicalsOfExactBases_AreExact_AndApproximateBasesAreNot#0` | Symbolics.ExactnessFlagTests/RadicalsOfExactBases_AreExact_AndApproximateBasesAreNot#0 | x^1.5: oracle canonicalizes the numeric exponent to the exact 3/2 (exact=true) vs rust Pow(Sym x, Real 1.5) (exact=false); the other seven radical/RootOf assertions agree. |
| 25 | STILL-DIFFERENT | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.HelpServiceTests/Help_Overview_ListsAllCategories#0` | Suite.HelpServiceTests/Help_Overview_ListsAllCategories#0 | help::overview() lists the categories but not the "help <function>" line the C# row requires (measured on the Rust surface) |
| 26 | STILL-DIFFERENT | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.RelationsAndFamiliesTests/Integrate_StructuredResult_ReportsConditions#0` | Symbolics.RelationsAndFamiliesTests/Integrate_StructuredResult_ReportsConditions#0 | $.result.display: oracle IntegrationResult(status: SolvedConditional, expression: log(x), conditions: [x > 0], ...) vs rust status SolvedExact, conditions []; the x^5 and exp(x^2) probes match. |
| 27 | STILL-DIFFERENT | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.StructuredRealExactnessTests/TerminatingQuotientBeyondTheDigitBudget_IsCertifiedExact#2` | Suite.StructuredRealExactnessTests/TerminatingQuotientBeyondTheDigitBudget_IsCertifiedExact#2 | 2^-100000: oracle publishes 1/2^100000 exactly; the rust process does not terminate within 35 s (no envelope). |
| 28 | STILL-DIFFERENT | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.LatexSymbolNameEscapingTests/LatexSymbolName_InsideLargerExpressions_IsStillEscaped#0` | Symbolics.LatexSymbolNameEscapingTests/LatexSymbolName_InsideLargerExpressions_IsStillEscaped#0 | latex(derivative(f, [x_1])): oracle "\\operatorname{diff}(f, x\\_1)" vs rust "\\operatorname{derivative}(f, x\\_1)"; the escaped product rendering matches |
| 29 | STILL-DIFFERENT | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PropertyTests/MathIR_EvaluatesLikeSymbolic#0` | Symbolics.PropertyTests/MathIR_EvaluatesLikeSymbolic#0 | MathIR_EvaluatesLikeSymbolic: numeric::evaluate and the lowered evaluation differ by 1 in the 50th decimal place on corpus expression 2 (round-62 measurement, unchanged) |
| 30 | STILL-DIFFERENT | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.NameSuggestionsTests/ANameNothingIsCloseTo_IsNotGuessedAt#0` | Suite.NameSuggestionsTests/ANameNothingIsCloseTo_IsNotGuessedAt#0 | registry::suggest("fft") returns ["fft"] (the exact registered name) where the C# row requires an empty suggestion list; nosuchfn/push/zip are empty in both |
| 31 | STILL-DIFFERENT | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ParserTests/Parse_GivenChainedAssignment_RightAssociative#0` | Suite.ParserTests/Parse_GivenChainedAssignment_RightAssociative#0 | x = y = 5: oracle exit 0 (display 5) vs rust exit 1 ParseError "Expected ';' or end of input but found '=' at position 6." |
| 32 | STILL-DIFFERENT | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.CapabilitiesBuiltinTests/PowerExponentClass_RefusesWhatItSays_AndNamesWhatIsSupported#0` | Symbolics.CapabilitiesBuiltinTests/PowerExponentClass_RefusesWhatItSays_AndNamesWhatIsSupported#0 | 0^(1/2): oracle exit 0 display "0" vs rust exit 1 (refused); the other scope counterexamples (-4)^(1/2) etc. match |
| 33 | STILL-DIFFERENT | `Lovelace.Run.Tests/Lovelace.Run.Tests.RecursionDepthEnvelopeTests/NestedBlocksInsideARecursiveBody_AreRefused_InsteadOfDying#0` | Run.RecursionDepthEnvelopeTests/NestedBlocksInsideARecursiveBody_AreRefused_InsteadOfDying#0 | 5 calls inside 120 nested ifs: oracle exit 1 DepthExceeded vs rust exit 0; f(85) in a for/while body: oracle Natural 0 vs rust result missing (refusal envelope). |
| 34 | STILL-DIFFERENT | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecursionBudgetCompositionTests/TheDeepestAcceptedCall_Answers_InEveryComposition#14` | Suite.RecursionBudgetCompositionTests/TheDeepestAcceptedCall_Answers_InEveryComposition#14 | f(85) in a for body and in a while body: oracle $.result Natural 0 vs rust no result (refusal envelope) |
| 35 | STILL-DIFFERENT | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecursionBudgetCompositionTests/TheDeepestAcceptedCall_Answers_InEveryComposition#15` | Suite.RecursionBudgetCompositionTests/TheDeepestAcceptedCall_Answers_InEveryComposition#15 | f(85) in a for body and in a while body: oracle $.result Natural 0 vs rust no result (refusal envelope) |
| 36 | STILL-DIFFERENT | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.SeriesLaurentAndBranchPointTests/Series_AcrossAPoleOrABranchPoint_PublishesTheExpansion#0` | Symbolics.SeriesLaurentAndBranchPointTests/Series_AcrossAPoleOrABranchPoint_PublishesTheExpansion#0 | $.result.display: series(1/x,x,0,2) oracle "x^-1 + O(x^2)" vs rust "0"; 1/x^2 oracle "x^-2 + O(x^2)" vs "0"; sin(x)/x oracle "1 - 1/6*x^2 + O(x^3)" vs "1 + O(x^3)"; sqrt(abs(x)) oracle "sqrt(x)" vs "sqrt(abs(x))" |
| 37 | STILL-DIFFERENT | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.SeriesLaurentAndBranchPointTests/Series_AcrossAPoleOrABranchPoint_PublishesTheExpansion#1` | Symbolics.SeriesLaurentAndBranchPointTests/Series_AcrossAPoleOrABranchPoint_PublishesTheExpansion#1 | $.result.display: series(1/x,x,0,2) oracle "x^-1 + O(x^2)" vs rust "0"; 1/x^2 oracle "x^-2 + O(x^2)" vs "0"; sin(x)/x oracle "1 - 1/6*x^2 + O(x^3)" vs "1 + O(x^3)"; sqrt(abs(x)) oracle "sqrt(x)" vs "sqrt(abs(x))" |
| 38 | STILL-DIFFERENT | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.SeriesLaurentAndBranchPointTests/Series_AcrossAPoleOrABranchPoint_PublishesTheExpansion#2` | Symbolics.SeriesLaurentAndBranchPointTests/Series_AcrossAPoleOrABranchPoint_PublishesTheExpansion#2 | $.result.display: series(1/x,x,0,2) oracle "x^-1 + O(x^2)" vs rust "0"; 1/x^2 oracle "x^-2 + O(x^2)" vs "0"; sin(x)/x oracle "1 - 1/6*x^2 + O(x^3)" vs "1 + O(x^3)"; sqrt(abs(x)) oracle "sqrt(x)" vs "sqrt(abs(x))" |
| 39 | STILL-DIFFERENT | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.SeriesLaurentAndBranchPointTests/Series_AcrossAPoleOrABranchPoint_PublishesTheExpansion#4` | Symbolics.SeriesLaurentAndBranchPointTests/Series_AcrossAPoleOrABranchPoint_PublishesTheExpansion#4 | $.result.display: series(1/x,x,0,2) oracle "x^-1 + O(x^2)" vs rust "0"; 1/x^2 oracle "x^-2 + O(x^2)" vs "0"; sin(x)/x oracle "1 - 1/6*x^2 + O(x^3)" vs "1 + O(x^3)"; sqrt(abs(x)) oracle "sqrt(x)" vs "sqrt(abs(x))" |
| 40 | STILL-DIFFERENT | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DiagnosticContractTests/KernelNotes_CrossAsDiagnosticMessages#0` | Symbolics.DiagnosticContractTests/KernelNotes_CrossAsDiagnosticMessages#0 | x = symbol("x"); solve_full(x == x, x) diagnostic message: oracle "0 = 0: every value is a solution." vs rust "No solver for this structure." (limit_full(a/x,x,0) matches) |
| 41 | STILL-DIFFERENT | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DxStructuredResultsTests/LinsolveFull_SingularSystem_ReportsStatusNotThrow#0` | Symbolics.DxStructuredResultsTests/LinsolveFull_SingularSystem_ReportsStatusNotThrow#0 | linsolve([[x,x],[x,x]],[1,1]): oracle exit 1 (the convenience projection throws) vs rust exit 0; the 2^8 system publishes Partial/false/Partial vs Solved/true/Complete. |
| 42 | STILL-DIFFERENT | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.SolveCompletenessPairingTests/EverySolveStatus_PublishesTheDocumentedPair_OnBothRecords#0` | Symbolics.SolveCompletenessPairingTests/EverySolveStatus_PublishesTheDocumentedPair_OnBothRecords#0 | 2^8 system: oracle status Partial / complete False / completeness Partial vs rust Solved / True / Complete |
| 43 | STILL-DIFFERENT | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.FunctionTests/Evaluate_GivenFunctionLocalVariable_DoesNotLeakToGlobal#0` | Suite.FunctionTests/Evaluate_GivenFunctionLocalVariable_DoesNotLeakToGlobal#0 | func f() { local = 42 }; f() and { z = 99 }: oracle $.variables length 1 vs rust 2 (the function-local/block-local leaks). |
| 44 | STILL-DIFFERENT | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.StatementTests/Evaluate_GivenVariableAssignmentInsideBlock_DoesNotLeak#0` | Suite.StatementTests/Evaluate_GivenVariableAssignmentInsideBlock_DoesNotLeak#0 | { z = 99 }: oracle $.variables length 1 vs rust 2 (the block-local leaks) |
| 45 | UNRUNNABLE | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ArrayAllocationRefusalTests/TheSuiteDeclaresTheAllocationRefusalTheRunnerClassifies#0` | Suite.ArrayAllocationRefusalTests/TheSuiteDeclaresTheAllocationRefusalTheRunnerClassifies#0 | AllocationRefusalException type |
| 46 | UNRUNNABLE | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.DspIntegerArgumentContractTests/TheExactlyIntegralRealRefusal_IsStillTheContractsContract#0` | Suite.DspIntegerArgumentContractTests/TheExactlyIntegralRealRefusal_IsStillTheContractsContract#0 | ArgumentContractException type |
| 47 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PiecewiseAssumptionTests/MirroredSpelling_IsAlsoDecided#0` | Symbolics.PiecewiseAssumptionTests/MirroredSpelling_IsAlsoDecided#0 | assumption-scoped numeric evaluation (EvaluateToNum with caller assumptions) |
| 48 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PiecewiseAssumptionTests/ProvenGuard_TakesTheBranch#0` | Symbolics.PiecewiseAssumptionTests/ProvenGuard_TakesTheBranch#0 | assumption-scoped numeric evaluation (EvaluateToNum with caller assumptions) |
| 49 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PiecewiseAssumptionTests/RefutedGuard_TakesTheOtherwiseBranch#0` | Symbolics.PiecewiseAssumptionTests/RefutedGuard_TakesTheOtherwiseBranch#0 | assumption-scoped numeric evaluation (EvaluateToNum with caller assumptions) |
| 50 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PiecewiseAssumptionTests/WithoutAssumptions_TheGuardIsStillUndecidable#0` | Symbolics.PiecewiseAssumptionTests/WithoutAssumptions_TheGuardIsStillUndecidable#0 | assumption-scoped numeric evaluation (EvaluateToNum with caller assumptions) |
| 51 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.LatexPrinterTests/KnownRendering_PowerOfAComplexConstant_IsAlreadyAGroup#0` | Symbolics.LatexPrinterTests/KnownRendering_PowerOfAComplexConstant_IsAlreadyAGroup#0 | complex constant leaf (Exprs.Complex) for the model-level printer |
| 52 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PrettyParenthesesTests/ModelLevel_PowerOfAComplexConstant_HasADelimitedBase#0` | Symbolics.PrettyParenthesesTests/ModelLevel_PowerOfAComplexConstant_HasADelimitedBase#0 | complex constant leaf (Exprs.Complex) for the pretty printer |
| 53 | UNRUNNABLE | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecursionDepthGuardTests/ADeepExpressionInsideTheRecursiveBody_IsRefused#0` | Suite.RecursionDepthGuardTests/ADeepExpressionInsideTheRecursiveBody_IsRefused#0 | deep-expression AST construction and EvaluationException API |
| 54 | UNRUNNABLE | `Lovelace.Complex.Tests/Lovelace.Complex.Tests.DenseArrayComplexTests/DenseArrayOfComplex_GivenConstruction_ReportsComplexDTypeAndElementType#0` | Complex.DenseArrayComplexTests/DenseArrayOfComplex_GivenConstruction_ReportsComplexDTypeAndElementType#0 | DenseArray<Complex> construction/span API |
| 55 | UNRUNNABLE | `Lovelace.Complex.Tests/Lovelace.Complex.Tests.DenseArrayComplexTests/DenseArrayOfComplex_GivenContiguous_AsSpanIterates#0` | Complex.DenseArrayComplexTests/DenseArrayOfComplex_GivenContiguous_AsSpanIterates#0 | DenseArray<Complex> construction/span API |
| 56 | UNRUNNABLE | `Lovelace.Complex.Tests/Lovelace.Complex.Tests.DenseArrayComplexTests/DenseArrayOfComplex_GivenElements_GetElementRoundTrips#0` | Complex.DenseArrayComplexTests/DenseArrayOfComplex_GivenElements_GetElementRoundTrips#0 | DenseArray<Complex> construction/span API |
| 57 | UNRUNNABLE | `Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerConstructorTests/Constructor_GivenInt_DelegatesToLongOverload#0` | Integer.IntegerConstructorTests/Constructor_GivenInt_DelegatesToLongOverload#0 | distinct Integer(Int)/Integer(Long) constructor overloads |
| 58 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DxSemanticClosureTests/DomainOf_NegativeIntegerPower_IsRationalNotInteger#0` | Symbolics.DxSemanticClosureTests/DomainOf_NegativeIntegerPower_IsRationalNotInteger#0 | Domains.DomainOf / Solvers.Solve internals |
| 59 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DxSemanticClosureTests/Solve_QuarticNoRealRoots_DefaultDomain_ReportsUnevaluated#0` | Symbolics.DxSemanticClosureTests/Solve_QuarticNoRealRoots_DefaultDomain_ReportsUnevaluated#0 | Domains.DomainOf / Solvers.Solve internals |
| 60 | UNRUNNABLE | `Lovelace.Run.Tests/Lovelace.Run.Tests.LibraryProjectionAgreementTests/TheFacade_PublishesEveryStoredDigit_WhenTheDisplayBoundIsLower#0` | Run.LibraryProjectionAgreementTests/TheFacade_PublishesEveryStoredDigit_WhenTheDisplayBoundIsLower#0 | engine computation/display decimal-place projection API |
| 61 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DiagnosticContractTests/ErrorCategory_HasExactlyTheSevenFrozenMembers#0` | Symbolics.DiagnosticContractTests/ErrorCategory_HasExactlyTheSevenFrozenMembers#0 | ErrorCategory enum (the seven frozen members) |
| 62 | UNRUNNABLE | `Lovelace.Run.Tests/Lovelace.Run.Tests.ErrorCategoryMirrorTests/EveryMirroredTaxonomy_EqualsTheEnum#0` | Run.ErrorCategoryMirrorTests/EveryMirroredTaxonomy_EqualsTheEnum#0 | ErrorCategory enum and the three mirrored taxonomy arrays |
| 63 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.ExpectedFailureBoundaryTests/EvaluateToExpr_ApproximateArgument_ThrowingEvaluatorPropagates#0` | Symbolics.ExpectedFailureBoundaryTests/EvaluateToExpr_ApproximateArgument_ThrowingEvaluatorPropagates#0 | Evaluation.EvaluateToNum/EvaluateToExpr with a throwing evaluator |
| 64 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.ExpectedFailureBoundaryTests/EvaluateToExpr_ExactArgument_ThrowingEvaluatorPropagates#0` | Symbolics.ExpectedFailureBoundaryTests/EvaluateToExpr_ExactArgument_ThrowingEvaluatorPropagates#0 | Evaluation.EvaluateToNum/EvaluateToExpr with a throwing evaluator |
| 65 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.ExpectedFailureBoundaryTests/Limit_ThrowingEvaluatorPropagates#0` | Symbolics.ExpectedFailureBoundaryTests/Limit_ThrowingEvaluatorPropagates#0 | Evaluation.EvaluateToNum/EvaluateToExpr with a throwing evaluator |
| 66 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.ExpectedFailureBoundaryTests/TryCompare_ReportsUnorderedInsteadOfThrowing#0` | Symbolics.ExpectedFailureBoundaryTests/TryCompare_ReportsUnorderedInsteadOfThrowing#0 | Evaluation.EvaluateToNum/EvaluateToExpr with a throwing evaluator |
| 67 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DomainFailureVocabularyTests/DivisionByZero_IsAnEvaluationFailure#0` | Symbolics.DomainFailureVocabularyTests/DivisionByZero_IsAnEvaluationFailure#0 | EvaluationException vocabulary on the evaluator API |
| 68 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DomainFailureVocabularyTests/DivisionByZero_OnTheRealPath_IsAnEvaluationFailure#0` | Symbolics.DomainFailureVocabularyTests/DivisionByZero_OnTheRealPath_IsAnEvaluationFailure#0 | EvaluationException vocabulary on the evaluator API |
| 69 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DomainFailureVocabularyTests/LogOfZero_IsAnEvaluationFailure#0` | Symbolics.DomainFailureVocabularyTests/LogOfZero_IsAnEvaluationFailure#0 | EvaluationException vocabulary on the evaluator API |
| 70 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DomainFailureVocabularyTests/ZeroToANegativePower_IsAnEvaluationFailure#0` | Symbolics.DomainFailureVocabularyTests/ZeroToANegativePower_IsAnEvaluationFailure#0 | EvaluationException vocabulary on the evaluator API |
| 71 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.ExactnessFlagTests/CachedFlag_EqualsTheLeafRule_OverTheWholeCorpus#0` | Symbolics.ExactnessFlagTests/CachedFlag_EqualsTheLeafRule_OverTheWholeCorpus#0 | Expr.IsExact leaf rule over a corpus with RootOf/Complex leaves |
| 72 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.ExactnessFlagTests/SolverSolutionExactness_AndValueExact_Agree#0` | Symbolics.ExactnessFlagTests/SolverSolutionExactness_AndValueExact_Agree#0 | Expr.IsExact leaf rule over a corpus with RootOf/Complex leaves |
| 73 | UNRUNNABLE | `Lovelace.Complex.Tests/Lovelace.Complex.Tests.ComplexMathInexactArgumentProvenanceTests/SymbolicNodeAndNumericResult_AgreeForSine#0` | Complex.ComplexMathInexactArgumentProvenanceTests/SymbolicNodeAndNumericResult_AgreeForSine#0 | Exprs.Real(RealLiteral) provenance and NumReal payload API |
| 74 | UNRUNNABLE | `Lovelace.Complex.Tests/Lovelace.Complex.Tests.ComplexMathInexactArgumentProvenanceTests/SymbolicNodeAndNumericResult_AgreeForTheSameExpression#0` | Complex.ComplexMathInexactArgumentProvenanceTests/SymbolicNodeAndNumericResult_AgreeForTheSameExpression#0 | Exprs.Real(RealLiteral) provenance and NumReal payload API |
| 75 | UNRUNNABLE | `Lovelace.Run.Tests/Lovelace.Run.Tests.GoldenFixtureRecorder/RecordGoldenFixtures#0` | Run.GoldenFixtureRecorder/RecordGoldenFixtures#0 | golden-fixture recorder (TestSupport.Normalise + SourceGoldenPath) |
| 76 | UNRUNNABLE | `Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerSubtractTests/SubtractOperator_IsEquivalentToSubtractMethod#0` | Integer.IntegerSubtractTests/SubtractOperator_IsEquivalentToSubtractMethod#0 | Integer operator- operator overload |
| 77 | UNRUNNABLE | `Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerMultiplyTests/OperatorStar_ProducesSameResultAsMultiplyMethod#0` | Integer.IntegerMultiplyTests/OperatorStar_ProducesSameResultAsMultiplyMethod#0 | Integer operator* operator overload |
| 78 | UNRUNNABLE | `Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerEqualityTests/Equals_GivenReferenceToSelf_ReturnsTrue#0` | Integer.IntegerEqualityTests/Equals_GivenReferenceToSelf_ReturnsTrue#0 | Integer reference-identity equality (Rust Int is a value) |
| 79 | UNRUNNABLE | `Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerPowTests/Pow_GivenNegativeExponent_Throws#0` | Integer.IntegerPowTests/Pow_GivenNegativeExponent_Throws#0 | Integer.Pow(Integer) refusal semantics (Rust pow takes u64) |
| 80 | UNRUNNABLE | `Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerPowTests/Pow_GivenZeroBase_Throws#0` | Integer.IntegerPowTests/Pow_GivenZeroBase_Throws#0 | Integer.Pow(Integer) refusal semantics (Rust pow takes u64) |
| 81 | UNRUNNABLE | `Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerPowTests/Pow_GivenZeroExponent_Throws#0` | Integer.IntegerPowTests/Pow_GivenZeroExponent_Throws#0 | Integer.Pow(Integer) refusal semantics (Rust pow takes u64) |
| 82 | UNRUNNABLE | `Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerToStringTests/TryFormat_GivenInsufficientBuffer_ReturnsFalse#0` | Integer.IntegerToStringTests/TryFormat_GivenInsufficientBuffer_ReturnsFalse#0 | Integer.TryFormat(Span<char>) fallible formatting API |
| 83 | UNRUNNABLE | `Lovelace.Integer.Tests/Lovelace.Integer.Tests.IntegerToStringTests/TryFormat_GivenSufficientBuffer_WritesRepresentationAndReturnsTrue#0` | Integer.IntegerToStringTests/TryFormat_GivenSufficientBuffer_WritesRepresentationAndReturnsTrue#0 | Integer.TryFormat(Span<char>) fallible formatting API |
| 84 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.RelationsAndFamiliesTests/KleeneEvaluation_AndOrNot#0` | Symbolics.RelationsAndFamiliesTests/KleeneEvaluation_AndOrNot#0 | Integration.IntegrateResult / Evaluation / Exprs construction API |
| 85 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.RelationsAndFamiliesTests/LogicalCanonicalization#0` | Symbolics.RelationsAndFamiliesTests/LogicalCanonicalization#0 | Integration.IntegrateResult / Evaluation / Exprs construction API |
| 86 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.RelationsAndFamiliesTests/Piecewise_WithCompoundGuard_Evaluates#0` | Symbolics.RelationsAndFamiliesTests/Piecewise_WithCompoundGuard_Evaluates#0 | Integration.IntegrateResult / Evaluation / Exprs construction API |
| 87 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.HyperbolicBuiltinsAndNegativeIntegerPowersTests/BuiltinCount_IncludesTheThreeHyperbolicFunctions#0` | Symbolics.HyperbolicBuiltinsAndNegativeIntegerPowersTests/BuiltinCount_IncludesTheThreeHyperbolicFunctions#0 | library builtin count (CaptureState().Functions builtin count) |
| 88 | UNRUNNABLE | `Lovelace.Representation.Tests/Lovelace.Representation.Tests.UnitTest1/Test1#0` | Representation.UnitTest1/Test1#0 | Lovelace.Representation crate (the C# case has an empty body and no observable) |
| 89 | UNRUNNABLE | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.TimingScaleTests/OperationTiming_ExposesTheSameScaleAsItsDisplay#0` | Suite.TimingScaleTests/OperationTiming_ExposesTheSameScaleAsItsDisplay#0 | OperationTiming.ElapsedDisplay/ElapsedScale API |
| 90 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PrintBudgetTests/DepthBudget_ReportsDepthLimit#0` | Symbolics.PrintBudgetTests/DepthBudget_ReportsDepthLimit#0 | Printing.PrintOptions/PrintBudget/PrintOutcome API (library max depth / node budget) |
| 91 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PrintBudgetTests/NodeBudget_OnACompactRendering_DropsNothingAndSaysSo#0` | Symbolics.PrintBudgetTests/NodeBudget_OnACompactRendering_DropsNothingAndSaysSo#0 | Printing.PrintOptions/PrintBudget/PrintOutcome API (library max depth / node budget) |
| 92 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PrintBudgetTokenBoundaryTests/AnIdentifierLongerThanTheAllowance_IsKeptWhole#0` | Symbolics.PrintBudgetTokenBoundaryTests/AnIdentifierLongerThanTheAllowance_IsKeptWhole#0 | Printing.PrintOptions/PrintOutcome token-boundary API |
| 93 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PrintBudgetTokenBoundaryTests/ARenderingWithinTheAllowance_IsUntouched#0` | Symbolics.PrintBudgetTokenBoundaryTests/ARenderingWithinTheAllowance_IsUntouched#0 | Printing.PrintOptions/PrintOutcome token-boundary API |
| 94 | UNRUNNABLE | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.NameSuggestionsTests/EditDistance_IsTheStructuralMetric#3` | Suite.NameSuggestionsTests/EditDistance_IsTheStructuralMetric#3 | public NameSuggestions.EditDistance API (registry::edit_distance is private) |
| 95 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.SolveCompletenessMappingTests/BudgetExceeded_IsIncomplete_WithTheKernelCompletenessOfThePartialSubset#0` | Symbolics.SolveCompletenessMappingTests/BudgetExceeded_IsIncomplete_WithTheKernelCompletenessOfThePartialSubset#0 | public SolveCompletenessMapping.Of(status, completeness) API |
| 96 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.SolveCompletenessMappingTests/EveryStatusCarriesTheDocumentedPair#3` | Symbolics.SolveCompletenessMappingTests/EveryStatusCarriesTheDocumentedPair#3 | public SolveCompletenessMapping.Of(status, completeness) API |
| 97 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DocsSyncTests/Every_CSharpSnippet_AppearsInUsageExamples#0` | Symbolics.DocsSyncTests/Every_CSharpSnippet_AppearsInUsageExamples#0 | README/UsageExamples sync checker |
| 98 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DocsSyncTests/Every_LovelaceExample_HasAnExpectedResult#0` | Symbolics.DocsSyncTests/Every_LovelaceExample_HasAnExpectedResult#0 | README/UsageExamples sync checker |
| 99 | UNRUNNABLE | `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigFastPathTests/Cos_AtDefaultPrecision_StaysInteractive#0` | Real.RealTrigFastPathTests/Cos_AtDefaultPrecision_StaysInteractive#0 | Real.Cos fast path meeting the 10 s budget at 1000 places (fast_path_tests.rs documents the miss) |
| 100 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.TruncatingEvaluationStaysInexactTests/FunctionOfATruncatedArgument_IsInexact#0` | Symbolics.TruncatingEvaluationStaysInexactTests/FunctionOfATruncatedArgument_IsInexact#0 | RealLiteral.FromRealExact/ToReal and NumReal payload API |
| 101 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.TruncatingEvaluationStaysInexactTests/LiteralRoundTrip_KeepsGenuinelyExactValuesExact#0` | Symbolics.TruncatingEvaluationStaysInexactTests/LiteralRoundTrip_KeepsGenuinelyExactValuesExact#0 | RealLiteral.FromRealExact/ToReal and NumReal payload API |
| 102 | UNRUNNABLE | `Lovelace.Run.Tests/Lovelace.Run.Tests.ArityPropertyTests/DeclaredArity_TravelsWithEveryPluginBuiltin#0` | Run.ArityPropertyTests/DeclaredArity_TravelsWithEveryPluginBuiltin#0 | registry FunctionDefinition.MinArity/Variadic introspection |
| 103 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.RewriteProofBridgeTests/EveryObligation_IsStatedAndHonestAboutItsStatus#0` | Symbolics.RewriteProofBridgeTests/EveryObligation_IsStatedAndHonestAboutItsStatus#0 | rewrite proof-obligation bridge API |
| 104 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.RewriteProofBridgeTests/EveryShippedRule_HasExactlyOneProofObligation#0` | Symbolics.RewriteProofBridgeTests/EveryShippedRule_HasExactlyOneProofObligation#0 | rewrite proof-obligation bridge API |
| 105 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.InverseBranchCandidateVerificationTests/EveryRepresentedSolutionSatisfiesTheOriginalEquation#4` | Symbolics.InverseBranchCandidateVerificationTests/EveryRepresentedSolutionSatisfiesTheOriginalEquation#4 | Solvers.Solve inverse-branch candidate verification API |
| 106 | UNRUNNABLE | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.StructuralEqualityTests/DifferenceInANestedField_IsNamedByPath#0` | Suite.StructuralEqualityTests/DifferenceInANestedField_IsNamedByPath#0 | StructuralEquality.FirstDifference path reporting |
| 107 | UNRUNNABLE | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.StructuralEqualityTests/DifferentKinds_AreReportedByKind#0` | Suite.StructuralEqualityTests/DifferentKinds_AreReportedByKind#0 | StructuralEquality.FirstDifference path reporting |
| 108 | UNRUNNABLE | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.StructuralEqualityTests/DifferentShape_IsReportedAsShape#0` | Suite.StructuralEqualityTests/DifferentShape_IsReportedAsShape#0 | StructuralEquality.FirstDifference path reporting |
| 109 | UNRUNNABLE | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.StructuralEqualityTests/SymbolicEquality_IsByCanonicalForm_NotPrettyText#0` | Suite.StructuralEqualityTests/SymbolicEquality_IsByCanonicalForm_NotPrettyText#0 | StructuralEquality.FirstDifference path reporting |
| 110 | UNRUNNABLE | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.SuiteEngineTests/CaptureState_IsImmutable#0` | Suite.SuiteEngineTests/CaptureState_IsImmutable#0 | SuiteEngine CaptureState, events and SetVariable/RemoveVariable API |
| 111 | UNRUNNABLE | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.SuiteEngineTests/FunctionDefined_GivenDefinition_RaisesEvent#0` | Suite.SuiteEngineTests/FunctionDefined_GivenDefinition_RaisesEvent#0 | SuiteEngine CaptureState, events and SetVariable/RemoveVariable API |
| 112 | UNRUNNABLE | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.SuiteEngineTests/SetVariableAndRemoveVariable_Work#0` | Suite.SuiteEngineTests/SetVariableAndRemoveVariable_Work#0 | SuiteEngine CaptureState, events and SetVariable/RemoveVariable API |
| 113 | UNRUNNABLE | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.SuiteEngineTests/VariableChanged_GivenAssignment_RaisesEvent#0` | Suite.SuiteEngineTests/VariableChanged_GivenAssignment_RaisesEvent#0 | SuiteEngine CaptureState, events and SetVariable/RemoveVariable API |
| 114 | UNRUNNABLE | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.SuiteEngineOutputPlotTests/EvaluateAsync_GivenOutputWriter_CapturesPrintLinesToThatWriter#0` | Suite.SuiteEngineOutputPlotTests/EvaluateAsync_GivenOutputWriter_CapturesPrintLinesToThatWriter#0 | SuiteEngine output writer / ResetPlotCapture API |
| 115 | UNRUNNABLE | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.SuiteEngineOutputPlotTests/ResetPlotCapture_GivenPreviousPlot_ClearsLastPlot#0` | Suite.SuiteEngineOutputPlotTests/ResetPlotCapture_GivenPreviousPlot_ClearsLastPlot#0 | SuiteEngine output writer / ResetPlotCapture API |
| 116 | UNRUNNABLE | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.PlotFileWriteRefusalTests/EmptyPlotFile_IsRefusedByType#0` | Suite.PlotFileWriteRefusalTests/EmptyPlotFile_IsRefusedByType#0 | SuiteEngine PlotFileWriteException typed refusal |
| 117 | UNRUNNABLE | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.PlotFileWriteRefusalTests/PlotFileUnderAMissingDirectory_IsRefusedByType#0` | Suite.PlotFileWriteRefusalTests/PlotFileUnderAMissingDirectory_IsRefusedByType#0 | SuiteEngine PlotFileWriteException typed refusal |
| 118 | UNRUNNABLE | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.PlotFileWriteRefusalTests/TheSuiteDeclaresThePlotFileWriteRefusalTheRunnerClassifies#0` | Suite.PlotFileWriteRefusalTests/TheSuiteDeclaresThePlotFileWriteRefusalTheRunnerClassifies#0 | SuiteEngine PlotFileWriteException typed refusal |
| 119 | UNRUNNABLE | `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.DspTests/Product_GivenImpulseAndStep_MultipliesPointwise#0` | Dsp.DspTests/Product_GivenImpulseAndStep_MultipliesPointwise#0 | Sum/Product/Sequence signal composition API (complex arrays refuse +/*) |
| 120 | UNRUNNABLE | `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.DspTests/Sequence_GivenLengthMismatch_ThrowsArgumentException#0` | Dsp.DspTests/Sequence_GivenLengthMismatch_ThrowsArgumentException#0 | Sum/Product/Sequence signal composition API (complex arrays refuse +/*) |
| 121 | UNRUNNABLE | `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.DspTests/Sum_GivenImpulseAndStep_AddsPointwise#0` | Dsp.DspTests/Sum_GivenImpulseAndStep_AddsPointwise#0 | Sum/Product/Sequence signal composition API (complex arrays refuse +/*) |
| 122 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.MatrixAdvanceTests/Inverse_Invertible_BehaviorPreserved#0` | Symbolics.MatrixAdvanceTests/Inverse_Invertible_BehaviorPreserved#0 | SymbolicMatrix inverse/rank/solve/solve_with_conditions API (only inverse_with_conditions exists) |
| 123 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.MatrixAdvanceTests/InverseWithConditions_NonSingular_MatchesInverse_And_CarriesCondition#0` | Symbolics.MatrixAdvanceTests/InverseWithConditions_NonSingular_MatchesInverse_And_CarriesCondition#0 | SymbolicMatrix inverse/rank/solve/solve_with_conditions API (only inverse_with_conditions exists) |
| 124 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.MatrixAdvanceTests/Rank_GenericOverSymbols_And_Numeric#0` | Symbolics.MatrixAdvanceTests/Rank_GenericOverSymbols_And_Numeric#0 | SymbolicMatrix inverse/rank/solve/solve_with_conditions API (only inverse_with_conditions exists) |
| 125 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.MatrixAdvanceTests/SolveWithConditions_MatchesSolve_And_CarriesCondition#0` | Symbolics.MatrixAdvanceTests/SolveWithConditions_MatchesSolve_And_CarriesCondition#0 | SymbolicMatrix inverse/rank/solve/solve_with_conditions API (only inverse_with_conditions exists) |
| 126 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.LinsolveCheckTests/Solve_3x3_VerifiesByNumericSubstitution#0` | Symbolics.LinsolveCheckTests/Solve_3x3_VerifiesByNumericSubstitution#0 | SymbolicMatrix.Solve with numeric substitution verification |
| 127 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.LinsolveCheckTests/Solve_VerifiesByNumericSubstitution#0` | Symbolics.LinsolveCheckTests/Solve_VerifiesByNumericSubstitution#0 | SymbolicMatrix.Solve with numeric substitution verification |
| 128 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DifferentialOracleTests/SympyOracle_Derivatives_Agree#0` | Symbolics.DifferentialOracleTests/SympyOracle_Derivatives_Agree#0 | SymPy oracle session (SympyOracle.Require) |
| 129 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DifferentialOracleTests/SympyOracle_Factorization_Agrees#0` | Symbolics.DifferentialOracleTests/SympyOracle_Factorization_Agrees#0 | SymPy oracle session (SympyOracle.Require) |
| 130 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DifferentialOracleTests/SympyOracle_Limits_Agree#0` | Symbolics.DifferentialOracleTests/SympyOracle_Limits_Agree#0 | SymPy oracle session (SympyOracle.Require) |
| 131 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DifferentialOracleTests/SympyOracle_MatrixOperations_Agree#0` | Symbolics.DifferentialOracleTests/SympyOracle_MatrixOperations_Agree#0 | SymPy oracle session (SympyOracle.Require) |
| 132 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DifferentialOracleTests/SympyOracle_Roots_Agree#0` | Symbolics.DifferentialOracleTests/SympyOracle_Roots_Agree#0 | SymPy oracle session (SympyOracle.Require) |
| 133 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DifferentialOracleTests/SympyOracle_Solve_Agrees#0` | Symbolics.DifferentialOracleTests/SympyOracle_Solve_Agrees#0 | SymPy oracle session (SympyOracle.Require) |
| 134 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.InfiniteFamilyCompletenessTests/SympyOracle_ExpOverComplexes_IsAnImageSet_AndTheTemplateMatchesIt#0` | Symbolics.InfiniteFamilyCompletenessTests/SympyOracle_ExpOverComplexes_IsAnImageSet_AndTheTemplateMatchesIt#0 | SymPy oracle session (SympyOracle.Require) |
| 135 | UNRUNNABLE | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.SystemSolveProofTests/NonlinearElimination_AndTheBoundary_AgreeWithSympySolve#0` | Symbolics.SystemSolveProofTests/NonlinearElimination_AndTheBoundary_AgreeWithSympySolve#0 | SymPy oracle session (SympyOracle.Require) |
| 136 | UNRUNNABLE | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ParserTests/Parse_GivenUnaryPlus_ReturnsUnaryExprPlus#0` | Suite.ParserTests/Parse_GivenUnaryPlus_ReturnsUnaryExprPlus#0 | syntax::Expr unary-plus AST node (+5 parses to the literal) |
| 137 | UNRUNNABLE | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.HelpServiceTests/UnicodeOutput_ThreadsThroughEngineFormatting#0` | Suite.HelpServiceTests/UnicodeOutput_ThreadsThroughEngineFormatting#0 | UnicodeOutput formatting toggle on the engine |
| 138 | UNRUNNABLE | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ValueDepthGuardTests/ADeepRecordChain_IsRefusedTheSameWay#0` | Suite.ValueDepthGuardTests/ADeepRecordChain_IsRefusedTheSameWay#0 | ValueFormatter depth guard API |
| 139 | UNRUNNABLE | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ValueDepthGuardTests/ValueOnePastTheBudget_IsRefusedByTheStructuredProjection#0` | Suite.ValueDepthGuardTests/ValueOnePastTheBudget_IsRefusedByTheStructuredProjection#0 | ValueFormatter depth guard API |
