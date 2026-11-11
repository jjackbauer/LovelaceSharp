# Round 35 - the assumption/tri-state and RealLiteral/RealConstant families

Status: complete for the two families this lane owned, with the measured limits recorded in
section 5. The workspace suite is green, the corpus is at its documented 212/214, the fixture and
CLI gates pass, and the differential run discovers no new regression (`new=0`).

## 0. What was built

| Area | Production change |
|---|---|
| Tri-state kernel | New `lovelace-symbolics::assumptions` module: `Tristate` (True/False/Unknown/Unsatisfiable), `Domain`, `SymbolPredicate`, the five assumption atoms, and an immutable `AssumptionSet` with ordered insertion, the implication lattice, constant-bound reasoning, interval reasoning, contradiction detection (negation clash and integer-domain emptiness), canonical ordering and `FromAtoms`. |
| Language surface | `Engine` now owns ONE `AssumptionSet` per engine. `assume`, `assume_positive/nonnegative/negative`, `assume_real/integer`, the new `assume_clear`, `symbol(name, domain)`, `simplify`, `simplify_full` and `inspect` all read and write it. |
| Published conditions | `assumptions()` renders the set ("x in Real; x is Positive; x > 5") and projects every atom as a structured leaf (`DomainCondition`, `x > 0`, `IntervalCondition`, `FiniteCondition`, `UnsatisfiableConditions`), never a string. `inspect(...).assumptions` publishes the atoms that constrain the expression's free symbols. |
| Contradiction | `assume` refuses a contradictory add with the reference sentence, the `UnsatisfiableAssumptions` code and the `DomainError` category; the integer-domain emptiness is detected too. |
| Unsatisfiable transform | `simplify_full` reports `status: Unsatisfiable`, `conditions: [UnsatisfiableConditions()]` and the `transform.unsatisfiable-conditions` diagnostic (`recoverable: False`) when the session's assumptions deny the rewrite's conditions. |
| Domain-conditional rewrites | `sqrt(x^2)` -> `abs(x)` under provably-real x (and -> `x` when provably nonnegative), `abs(x)^2` -> `x^2` and `log(exp(x))` -> `x` under real x. |
| Real constant carrier | New `Expr::Real(String)` leaf: prints its decimal, canonicalises to `(real ...)`, is inexact by kind, real-valued, one node, no free symbols; `as_expr` installs it for an inexact Real value, and `numeric::evaluate` reads it back as an inexact Real. |
| Real arithmetic boundary | A Real value crosses into an expression as an exact rational while its decimal has at most 19 significant digits, and as a Real leaf beyond that - the boundary is measured (section 3), not assumed. |

## 1. Family A - assumption/tri-state (33 rows)

Before/after, both runners, same script (oracle = `dotnet exec Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll`,
rust = `rust/target/release/lovelace-run.exe`, `--omit-functions`):

| Probe | Before (Rust) | Oracle | After (Rust) |
|---|---|---|---|
| `x = symbol("x"); assume_real(x); assume(x > 0); assume(x > 5); assumptions()` | `x > 0; x > 5` (no domain atom) | `x in Real; x > 0; x > 5` | **matches** |
| `x = symbol("x"); assume_positive(x); assumptions()` | `x > 0` (rendered as a relation) | `x is Positive` | **matches** |
| `x = symbol("x"); assume(x != 2); assume(x > 5); assume_real(x); assume(x > 0); assumptions()` | `x in Real; x > 0; x > 5; x != 2` | `x in Real; x > 5; x > 0; x != 2` | **matches** (ties keep insertion order) |
| `x = symbol("x"); assume(x > 0); assume_clear(); assumptions()` | error: unknown function | `AssumptionSet(display: , assumptions: [])` | **matches** |
| `x = symbol("x"); assume(x > 5); simplify(x < 5)` | `x < 5` | `False` | **matches** |
| `x = symbol("x"); assume(x > 5); simplify(x < 7)` | `x < 7` | `x < 7` | **matches** (no over-claim) |
| `x = symbol("x", real); simplify(sqrt(x^2))` | `sqrt(x^2)` | `abs(x)` | **matches** |
| `x = symbol("x", real); simplify(abs(x)^2)` | `abs(x)^2` | `x^2` | **matches** |
| `x = symbol("x", real); simplify(log(exp(x)))` | `log(exp(x))` | `x` | **matches** |
| `x = symbol("x"); assume_integer(x); assume(x > 1/2); assume(x < 1)` | `x < 1` accepted | `UnsatisfiableAssumptions: ... leaves no integer ...` | **matches** |
| `x = symbol("x"); assume(x == 0); simplify_full(x/x)` | `Satisfied`, `conditions: [x != 0]` | `Unsatisfiable`, `[UnsatisfiableConditions()]` + diagnostic | **matches** |
| `x = symbol("x"); assume(x > 0); inspect(sqrt(x^2))` | `assumptions: []` | `assumptions: [x > 0]` | **matches** |
| `x = symbol("x", real); inspect(sqrt(x^2))` | `assumptions: []` | `[DomainCondition(variable: x, domain: real)]` | **matches** |

Expected census ids (all 33 rows of the family in `round-34/symbolics-tail.md`):

```
AcceptanceTests/Simplify_SqrtSquare_UnderAssumptions#0
AssumptionAddScalingTests/Add_MixedAtomsInAnyInsertionOrder_YieldsTheSameCanonicalOrder#0
AssumptionAddScalingTests/Add_OrderEqualsHistoricalAppendAndResort_ForEveryInsertionOrder#0
AssumptionAddScalingTests/Add_SameAtomTwiceFromDifferentContexts_IsStoredOnce#0
AssumptionAddScalingTests/Add_Scaling_800AtomsStaysUnder250ms_AndKeepsEveryAtomInOrder#0
AssumptionScopeIsolationTests/NestedScopes_RestoreTheEnclosingScopeOnDispose#0
AssumptionScopeIsolationTests/TwoConcurrentScopes_AreIsolated#0
AssumptionSoundnessTests/BoundReasoning_MatchesReferenceTruthTable#0
AssumptionSoundnessTests/ContradictionDetection_IsNeitherMissedNorInvented#0
AssumptionSoundnessTests/DomainAtoms_MergeToTheNarrowest#0
AssumptionSoundnessTests/IntervalBounds_WithUnevaluableBound_AreUnknown#0
AssumptionSoundnessTests/PropertyAndRelationAtoms_Contradict#0
AssumptionSoundnessTests/RealValuedBounds_ParticipateInReasoning#0
AssumptionSoundnessTests/UnsatisfiableSet_AnswersUnsatisfiable_NotUnknown#0
DxSemanticClosureTests/AskRelation_ProvesWeakerBounds#0
DxSemanticClosureTests/AskRelation_RefutesContradictoryBounds#0
DxSemanticClosureTests/Assume_CompatibleRelations_DoNotThrow#0
DxSemanticClosureTests/DomainOf_NegativeIntegerPower_IsRationalNotInteger#0   (see section 5)
DxSemanticClosureTests/Unsatisfiable_Sentinel_IsSticky#0
DxStructuredResultsTests/Inspect_ReportsTheAssumptionsThatConstrainTheFreeSymbols#0
DxStructuredResultsTests/ParameterDomainOf_MapsEveryDeclaredArm#0              (see section 5)
IntegerIntervalContradictionTests/Integer_NonEmptyInterval_IsAccepted#0
IntegerIntervalContradictionTests/Integer_OpenBoundaryLeavesItsNeighbourReachable#0
IntegerIntervalContradictionTests/Integer_StrictFractionalSandwich_IsContradictory#0
IntegerIntervalContradictionTests/Integer_UnitInterval_IsContradictory#0
IntegerIntervalContradictionTests/Reals_WithTheSameFractionalBounds_StaySatisfiable#0
KernelHardeningTests/AbsSquare_Simplified_UnderReal#0
KernelHardeningTests/DomainPredicates_AreRealGuarded#0
KernelHardeningTests/EvenOdd_ImplyInteger_OddImpliesNonZero#0
KernelHardeningTests/Integer_DoesNotImply_EvenOddNonZero#0
KernelHardeningTests/IntervalCondition_ImZero_ForProvablyReal#0
KernelHardeningTests/LogExp_Simplified_UnderReal#0
KernelHardeningTests/Or_ReturnsFalse_WhenAllDisjunctsFalse#0
```

Two of them are explicitly NOT claimed as mapped (section 5): the kernel has no
`DomainOf(NegativeIntegerPower)` language surface and no `ParameterDomain` enum, so those two
rows stay deferred rather than being asserted by a weaker proxy.

## 2. Family B - RealLiteral / RealConstant (33 rows)

| Probe | Before (Rust) | Oracle | After (Rust) |
|---|---|---|---|
| `x = symbol("x"); subs(x + 1, x, pi(30))` | `4141592653589793238462643383279/1000000000000000000000000000000` | `1 + 3.141592653589793238462643383279` | **matches** |
| `inspect(subs(x + 1, x, pi(30)))` | `domain: rational, exact: True, canonical: (rat ...)` | `domain: real, exact: False, canonical: (add (rat 1 1) (real 3.141...))` | **matches** |
| `x = symbol("x"); subs(x + 1, x, e(30))` | rational | `1 + 2.718281828459045235360287471352` | **matches** |
| `x = symbol("x"); subs(x + 1.5, x, pi(30))` | rational | `3/2 + 3.141592653589793238462643383279` | **matches** |
| `x = symbol("x"); subs(x^2, x, pi(30))` | `24649/2500`-style rational | `3.141592653589793238462643383279^2` | **matches** |
| `x = symbol("x"); subs(x^2, x, pi(2))` | `24649/2500` | `24649/2500` | **matches** |
| `x = symbol("x"); subs(x + 1, x, pi(2))` | `207/50` | `207/50` | **matches** |
| `evalf(sin(pi(1)*1/6), 40)` | error: `sin needs a number or an expression.` | `0.4939846521590624789945640658702374513097` | **matches** |
| `x = symbol("x"); simplify(pi(30) + 1)` | rational | `4.141592653589793238462643383279` | **matches** |

The rational/real boundary was measured on the oracle, not guessed:
`subs(x + 1, x, pi(n))` answers a rational for n <= 18 and a Real leaf for n >= 19, i.e. the
boundary is 19 significant decimal digits. `subs(x + 1, x, pi(18))` ->
`2070796326794896619/500000000000000000` in both runners; `pi(19)` -> `1 + 3.1415926535897932384`
in both.

Expected census ids (all 33 rows of the family):

```
EvalfTruncatedConstantExactnessTests/ExactControls_AreUntouched#0
EvalfWorkingPrecisionHonoursRequestedDecimalsTests/ValuesBelowTheComputationCap_StayInexact#0
ExactnessFlagTests/CachedFlag_EqualsTheLeafRule_OverTheWholeCorpus#0
ExactnessFlagTests/ElementaryFunctionOfAnApproximateArgument_IsInexact#0..#5
ExactnessFlagTests/InspectExact_IsFalse_WhenARealApproximationIsInTheTree#0
ExactnessFlagTests/RadicalsOfExactBases_AreExact_AndApproximateBasesAreNot#0
KernelHardeningTests/HugeExponent_CompactForm_RoundTrips#0
KernelHardeningTests/NumericToSymbolicToNumeric_500Digits_RoundTrips#0
KernelHardeningTests/RealConstantAdd_Preserves500Digits#0
KernelHardeningTests/RealConstantMultiply_PreservesPrecision#0
NegativePowerEvaluationTests/DerivativeOfTanOfXCubed_EvaluatesToTheSymPyValue#0
NegativePowerEvaluationTests/NegativeIntegerPowerOfAReal_IsTheReciprocal#0
NegativePowerEvaluationTests/ReciprocalOfARealJustBelowOne_KeepsItsQuotient#0..#4
TruncatingEvaluationStaysInexactTests/ExactControls_StayExact#0
TruncatingEvaluationStaysInexactTests/FunctionOfATruncatedArgument_IsInexact#0
TruncatingEvaluationStaysInexactTests/FunctionOfATruncatedValue_PublishesExactFalse_AndNoRationalForm#0
TruncatingEvaluationStaysInexactTests/FunctionOfATruncatedValue_PublishesExactFalse_AndNoRationalForm#1
TruncatingEvaluationStaysInexactTests/FunctionOfATruncatedValue_PublishesExactFalse_AndNoRationalForm#11
TruncatingEvaluationStaysInexactTests/LiteralRoundTrip_KeepsGenuinelyExactValuesExact#0
TruncatingEvaluationStaysInexactTests/LiteralRoundTrip_PreservesBothTheDigitsAndTheRoute#0
TruncatingEvaluationStaysInexactTests/SymbolicTreeFlag_AndNumericReadBack_Agree#0..#3
```

Of these, the rows this change measures end to end are the substitution/leaf/exactness/evalf
ones above. The rows that assert the C# API shape (500-digit add/multiply preservation, the
`RealLiteral` struct itself, per-Real negative powers) are NOT claimed as mapped: Rust has no
`RealLiteral` type, only the leaf, so those rows need a second slice (section 5).

## 3. Tests added (39)

- `lovelace-symbolics::assumptions::tests` - 17 tests: truth table, contradiction matrix,
  integer-domain emptiness, interval bounds, real-valued bounds, domain merge, canonical order,
  scaling (800 atoms; 194 ms in release, under the 250 ms tripwire; 637 ms debug with a
  documented 2 s debug ceiling because the reference itself excludes its timing case from
  instrumented runs).
- `lovelace-lang::assumptions_round35` - 17 tests: the language surface (published atoms,
  `assume_clear`, contradictions with the reference sentence and code, tri-state relation
  answers, `simplify_full` Unsatisfiable marker and diagnostic, `inspect` relevant atoms,
  engine isolation, canonical order, scaling).
- `lovelace-lang::real_constant_round35` - 5 tests: the Real leaf and its exactness, the
  measured rational/real boundary, `evalf(sin(...))` of an approximate argument.

## 4. Gates

| Gate | Command | Observed |
|---|---|---|
| Suite | `cargo test --workspace` | 4644 passed, 0 failed (4563 baseline + 39 this lane + 42 from a concurrent lane's `suite_tail_round35`) |
| Corpus | `cargo run --release -p lovelace-corpus` | `corpus: 212 passed, 2 failed, 214 total` (sym/071, sym/076 - the documented pre-existing divergences) |
| Fixtures | `node parity/tools/gate-fixtures.mjs` | `PASS: oracle 20/20, rust 20/20` |
| CLI matrix | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16`, PASS |
| Differential | `node parity/diff/run.mjs --cases 200 --seed 1 --no-minimize` | `discovered=1845 replayed=1752 stillMismatching=978 resolved=774 new=0` (no new regression) |

No `.cs`, fixture, corpus, `parity/` or manifest file was edited; the manifest is untouched, so
coverage stays at the round-34 figure until the rows above are mapped.

## 5. What remains (honest limits)

1. **Atom order on tied keys.** The reference orders atoms by the record's generated rendering,
   in which an expression contributes its node TYPE name (the kernel types do not override
   `ToString`). Two relations that differ only in their rational bound therefore tie, and the
   reference's `Add` falls back to append-and-sort (an UNSTABLE `ImmutableArray.Sort`). Rust
   reproduces the observable contract with a total order plus stable insertion among equal keys,
   which matches every measured oracle sequence; an exact reproduction of .NET's unstable sort
   order for a tie is deliberately not attempted, and the add-scaling tests assert the
   non-decreasing-key contract the reference itself asserts.
2. **Two family-A rows stay deferred.** `DxSemanticClosureTests/DomainOf_NegativeIntegerPower_IsRationalNotInteger`
   needs a published `domain(...)`/`DomainOf` language surface; `DxStructuredResultsTests/ParameterDomainOf_MapsEveryDeclaredArm`
   needs a `ParameterDomain` enum. Neither exists in Rust, and asserting a proxy would be weaker
   than the row.
3. **Family B's C#-API rows.** There is no `RealLiteral` type and no public real-constant
   arithmetic API in Rust; the leaf covers substitution, exactness, rendering, domain and evalf,
   but the rows that construct 500-digit literals and add/multiply them through the kernel API
   still need that API (a `Real`-backed literal with `WithExactness`).
4. **Real arithmetic in sums.** `expr + real` keeps the Real leaf as a term rather than folding a
   second real into it; the measured single-leaf probes match, but two reals in one sum are not
   yet merged into one numeric tier the way the reference's `AddImpl` does.
