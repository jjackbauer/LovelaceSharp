# Round 46 — the C# falsification harness, ported additively

**Status: implemented and verified.** The 32 census rows that round 45 deferred as
`UNRUNNABLE` on the falsification harness now have one mirrored Rust test each, and all 32 pass.
The harness is test-support, the same role the C# surface plays: it lives in
`rust/lovelace-lang/src/falsification_round46/` and is compiled only under `cfg(test)`. No C#
file, fixture, `parity/` source or other documentation was edited. One one-line correction in
`lovelace-real` was required and is disclosed in section 6.

## 1. Home decision

The C# harness is not a library project: it is
`Lovelace.Symbolics.Tests/Falsification/{ObservationClass,BoundaryBias,FalsificationProbe}.cs`
plus `FalsificationLoopTests.cs`, `FalsificationTests.cs` and `FalsificationGateTests.cs`. It
evaluates expressions through `Evaluation.EvaluateToNum` and inspects engine value kinds through
`SuiteEngine`.

`lovelace-symbolics` cannot host it: its dependencies are only `lovelace-nat` and
`lovelace-int`, it has no numeric evaluator, and `numeric::evaluate` (the port of
`EvaluateToNum`) lives in `lovelace-lang`. Adding a dependency on `lovelace-lang` would be a
cycle (`lovelace-lang` depends on `lovelace-symbolics`). The harness therefore lives in
`lovelace-lang`, next to the evaluator it drives, with the frozen probe data and the assumption
handling reused from `lovelace-symbolics::assumptions`. The `lib.rs` change is one
`#[cfg(test)] mod` line.

## 2. The three APIs and every member the rows exercise

### 2.1 Probe API (`FalsificationTests.cs` + `Falsification/FalsificationProbe.cs`)

| C# member | Rust item |
|---|---|
| `FalsificationTests.FuzzIdentity(ruleId, lhs, rhs, ctx, x, assumptions, points)` | `tests::fuzz_identity` (shared helper driving the ten probe tests) |
| `FalsificationProbe` (18 fields: RuleId, Region, Label, Input, IsComplex, Class, LeftOutcome, RightOutcome, Left, Right, LeftValue, RightValue, DeltaText, Precision, Tolerance, Environment, GeneratorVersion, Why) | `probe::FalsificationProbe`, same 18 fields |
| `CurrentGeneratorVersion = "adv11-falsifier/1"` | `GENERATOR_VERSION` |
| `IsCounterexample`, `IsDefect` | same methods |
| `ToText()`, `Parse(string)`, `IdentityKey` | `to_text`, `parse`, `identity_key` |
| `FalsificationReport(GeneratorVersion, SampleCount, Probes, Defects)` | `probe::FalsificationReport` |
| `Counterexamples`, `Failed`, `ToText`, `Parse`, `Fingerprint` (SHA-256, uppercase hex) | same methods; SHA-256 implemented locally |

### 2.2 Loop API (`Falsification/FalsificationLoopTests.cs` + `ObservationClass.cs` + `BoundaryBias.cs`)

| C# member | Rust item |
|---|---|
| `ObservationClass` (Agree, Disagree, UndefinedLeft, UndefinedRight, BudgetOrRefusal) | `observation::ObservationClass` |
| `SideOutcome` (Evaluated, Undefined, Refused, Threw) | `SideOutcome` |
| `ComparisonTolerance(Value, Text)`, `TenToTheMinusEleven()`, `OfRational()` | `ComparisonTolerance`, `ten_to_the_minus_eleven`, `of_rational` |
| `SideObservation(Outcome, Value, Detail)` + predicates | `SideObservation` |
| `ClassifiedObservation(Class, Left, Right, DeltaText)` + `IsCounterexample`/`IsDefect` | `ClassifiedObservation` |
| `ObservationClassifier.Classify`, `Agrees`, `IsTypedResourceRefusal` | `ObservationClassifier::classify`, `classify_with`, `agrees`, `is_typed_resource_refusal` |
| `FalsificationText.Rational/Num/Environment/Escape/Unescape` | `observation::{rat_text, num_text, environment_text, escape, unescape}` |
| `ProbePoint(Region, Label, Value, Why)` | `probeset::ProbePoint` |
| `FalsificationProbeSet.GeneratedRegion/Frozen/Generated/All` | `probeset::{GENERATED_REGION, frozen, generated, all}` |
| `RuleShape(Environment, Left, Right)` + `EnvironmentText` | `looping::RuleShape` |
| `FalsificationLoop.Precision/ClassifyAt/AssumptionHolds/AtomHolds/Run` | `looping::{classify_at, assumption_holds, atom_holds_at}`, `FalsificationLoop::run` |
| `DeterministicRng.NextUInt64/NextInt` (splitmix64) | `boundary::DeterministicRng` |
| `BiasStepLadder.Default/Coarse/Fine/Contains` | `BiasStepLadder::{default_ladder, coarse, fine, contains}` |
| `BoundaryBiasSpec(Seed, Ladder, SamplesPerLevel, MaxNarrowSteps).Default` | `BoundaryBiasSpec`, `default_spec` |
| `BoundaryBias.Spec/BroadPhase/InteriorPoint/NarrowPhase` | `BoundaryBias::{spec, broad_phase, interior_point, narrow_phase}` |

### 2.3 Gate API (`FalsificationGateTests.cs`)

| C# member | Rust item |
|---|---|
| `FalsificationRegions.Legacy/NearPole/NearBranchCut/NearDiscontinuity/AssumptionBoundary/AllRealProbes/AllRealPoints/ComplexProbes/ComplexExclusionWitnesses/RequiredRegions/ComplexRegion/OutsideStripRegion` | `probeset::{frozen, legacy_points, complex_probes, complex_exclusion_witnesses, required_regions, COMPLEX_REGION, OUTSIDE_STRIP_REGION}` (frozen = 33 probes, generated = 27, total 60) |
| `GateReport` (Registered, Sampled, Uninstantiable, Unlicensed, Degenerate, Threw, Falsified, LicenseNotes, UnhandledAtoms, Compared, Regions, ComplexCompared, LicensedShapes, Failed, Count, AllCoveredRegions, Summary) | `gate::GateReport`, same members |
| `RegistryFalsificationGate.Run`, `AtomHoldsForTesting` | `gate::run_gate`, `atom_holds` |
| `RewriteRule`, `Match`, `ExprContext`, `Pattern`, `RuleApplicability` | `gate::GateRule` (id, instantiated lhs, `Result<Expr, String>` replacement, declared conditions, applicability predicate); the reference's pattern machinery has no Rust counterpart, so the nine rules are instantiated data |
| `Simplify.RulesForTesting` / `ShippedRuleIds` | `gate::gate_registry` — the reference's nine shipped identities as shapes. Five of the nine ids also appear in the Rust simplifier's step provenance (trig, sqrt-square-nonnegative, exp-log, cancel-x, cancel-zero); four are reference-only identities (sqrt-square-real, log-exp, abs-square, abs-neg) that the gate sweeps as data |
| `ComplexDisposition`, `ComplexTreatment`, `ComplexTreatmentAudit.Run` | `ComplexDisposition`, `ComplexTreatment`, `complex_treatment_audit` |

### 2.4 Evaluator adaptation

`default_eval(expr, variable, value)` substitutes the probe value into the expression and calls
`numeric::evaluate` at 40 fractional digits. `re`/`im` function leaves are evaluated by the
harness (the numeric tier has no `re`/`im` case); everything else is the kernel. `LangError` is
mapped to `EvalFailure` by kind: a resource code (`Cancelled`, `DepthExceeded`,
`BudgetExceeded`, `AllocationRefused`) is `Refused`, a domain/type error is `Undefined`, and any
other failure is `Threw`. This preserves the C# rule that only the typed domain refusal licenses
"undefined".

## 3. The 32 census rows

Prefix `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.` is omitted below; each entry is the
full manifest key with that prefix and the mirrored Rust test
(`falsification_round46::tests::<module>::<test>`).

### Probe API (10) — `...FalsificationTests`

| Census row | Rust test |
|---|---|
| `FalsificationTests/AbsNeg_Universal#0` | `probe_api::abs_neg_universal` |
| `FalsificationTests/AbsSquare_IsNotUniversal_OverComplex#0` | `probe_api::abs_square_is_not_universal_over_complex` |
| `FalsificationTests/AbsSquare_Real#0` | `probe_api::abs_square_real` |
| `FalsificationTests/CancelXOverX_NonZeroPoints#0` | `probe_api::cancel_x_over_x_non_zero_points` |
| `FalsificationTests/CancelZeroOverX_NonZeroPoints#0` | `probe_api::cancel_zero_over_x_non_zero_points` |
| `FalsificationTests/ExpLog_Universal_OnPositivePoints#0` | `probe_api::exp_log_universal_on_positive_points` |
| `FalsificationTests/LogExp_Real#0` | `probe_api::log_exp_real` |
| `FalsificationTests/SqrtSquare_NonNegative#0` | `probe_api::sqrt_square_non_negative` |
| `FalsificationTests/SqrtSquare_Real#0` | `probe_api::sqrt_square_real` |
| `FalsificationTests/TrigPythagorean_Universal#0` | `probe_api::trig_pythagorean_universal` |

### Loop API (13) — `...Falsification.FalsificationLoopTests`

| Census row | Rust test |
|---|---|
| `FalsificationLoopTests/Classifier_NamesEachClass_AndNeverFoldsADefectIntoOne#0` | `loop_api::classifier_names_each_class_and_never_folds_a_defect_into_one` |
| `FalsificationLoopTests/FalsificationFingerprintChild#0` | `loop_api::falsification_fingerprint_child` |
| `FalsificationLoopTests/FrozenBattery_IsAPrefixOfWhatTheSubstrateOffers_InItsOwnOrder#0` | `loop_api::frozen_battery_is_a_prefix_of_what_the_substrate_offers_in_its_own_order` |
| `FalsificationLoopTests/Generator_BracketsAKnownBoundary_TheNaturalSubtractionWidening#0` | `loop_api::generator_brackets_a_known_boundary_the_natural_subtraction_widening` |
| `FalsificationLoopTests/Generator_BracketsANonIntegralBoundary_AndEveryPointIsStrictlyInside#0` | `loop_api::generator_brackets_a_non_integral_boundary_and_every_point_is_strictly_inside` |
| `FalsificationLoopTests/Generator_BroadPhaseSequenceIsStableUnderTheSameSeed_AndMovesWithIt#0` | `loop_api::generator_broad_phase_sequence_is_stable_under_the_same_seed_and_moves_with_it` |
| `FalsificationLoopTests/Hypothesis_BoundaryBiasBeatsUniform_OnAPlantedDefect#0` | `loop_api::hypothesis_boundary_bias_beats_uniform_on_a_planted_defect` |
| `FalsificationLoopTests/Loop_IsDeterministic_AcrossProcesses#0` | `loop_api::loop_is_deterministic_across_processes` |
| `FalsificationLoopTests/Loop_IsDeterministic_SameSeedProducesAByteIdenticalReport#0` | `loop_api::loop_is_deterministic_same_seed_produces_a_byte_identical_report` |
| `FalsificationLoopTests/PlantedDefect_IsFalsified_AndTheReportNamesTheRegionAndTheInput#0` | `loop_api::planted_defect_is_falsified_and_the_report_names_the_region_and_the_input` |
| `FalsificationLoopTests/ProbeRecord_RoundTripsThroughItsDeterministicTextForm#0` | `loop_api::probe_record_round_trips_through_its_deterministic_text_form` |
| `FalsificationLoopTests/RegistryLoop_ClassifiesEveryShippedRule_AndFindsNoDefect#0` | `loop_api::registry_loop_classifies_every_shipped_rule_and_finds_no_defect` |
| `FalsificationLoopTests/Tolerance_IsAParameter_TheSameValuesDecideDifferentlyUnderDifferentTolerances#0` | `loop_api::tolerance_is_a_parameter_the_same_values_decide_differently_under_different_tolerances` |

### Gate API (9) — `...FalsificationGateTests`

| Census row | Rust test |
|---|---|
| `FalsificationGateTests/AssumptionBoundary_GuardFlipsExactlyAtZero#0` | `gate_api::assumption_boundary_guard_flips_exactly_at_zero` |
| `FalsificationGateTests/ComplexExclusion_EveryRealOnlyRuleHasAnObservedComplexCounterexample#0` | `gate_api::complex_exclusion_every_real_only_rule_has_an_observed_complex_counterexample` |
| `FalsificationGateTests/ComplexExclusion_LogExp_IsStoppedByRealness_NotByUndefinedness#0` | `gate_api::complex_exclusion_log_exp_is_stopped_by_realness_not_by_undefinedness` |
| `FalsificationGateTests/ComplexTreatment_PartitionsTheRegistry_WithExplicitCounts#0` | `gate_api::complex_treatment_partitions_the_registry_with_explicit_counts` |
| `FalsificationGateTests/Gate_Fails_WhenARuleThrows#0` | `gate_api::gate_fails_when_a_rule_throws` |
| `FalsificationGateTests/Gate_ReportsFalsification_WhenARuleIsWrong#0` | `gate_api::gate_reports_falsification_when_a_rule_is_wrong` |
| `FalsificationGateTests/Gate_StillFalsifies_AComplexOnlyWrongRule#0` | `gate_api::gate_still_falsifies_a_complex_only_wrong_rule` |
| `FalsificationGateTests/RegistryGate_EveryRegisteredRuleIsSampled_OnEveryRegion#0` | `gate_api::registry_gate_every_registered_rule_is_sampled_on_every_region` |
| `FalsificationGateTests/Sampling_IsASuperset_OfTheOriginalPoints_AndEveryProbeIsDocumented#0` | `gate_api::sampling_is_a_superset_of_the_original_points_and_every_probe_is_documented` |

## 4. The contracts the tests assert

The assertions are the C# ones, not a replay of Rust output:

* Probe API: every frozen point agrees within 1e-11 under the 40-digit evaluator; the complex
  `|z|^2 vs z^2` counterexample is observed at `1.5+2i`; no side throws unexpectedly.
* Loop API: the frozen 33 are a prefix of the 60-point supply; the generated 27 are value-distinct
  and collision-free; the five observation classes are each reachable; a probe round-trips through
  its 18-field escaped text form and its SHA-256 fingerprint survives a report round-trip; two
  in-process runs and two fresh-process runs produce the same fingerprint; the boundary generator
  brackets the documented Natural→Integer subtraction widening at `b = 6` with no midpoint offered
  below the ladder's resolution; a planted band defect is falsified and named; the hypothesis
  measurement is pinned to the measured verdict (0 biased wins, 0 uniform wins, 6 misses); the nine
  shipped rule shapes produce no defect and no counterexample over the whole battery.
* Gate API: every one of the nine rules is sampled, every required region contributes a compared
  point, the throwing and wrong-rule controls fail the gate, the complex partition is exactly
  5 sampled / 4 exclusion-proven with the pinned witnessed points (`1+i`, `5i`, `-1+i`, `1+i`) and
  `|delta|^2 > 1`, the guard flips exactly at zero, and the complex-only wrong rule is still
  falsified on the `complex:` region.

## 5. Gate outputs (observed this round)

| Check | Observed |
|---|---|
| `cargo test -p lovelace-lang falsification_round46` | **32 passed, 0 failed** |
| `cargo test --workspace` | `binaries=19 passed=4926 failed=0`, exit 0 (4,843 baseline + the 32 new + the lexer lane's tests) |
| Documentation corpus | `corpus: 214 passed, 0 failed, 214 total` |
| Protocol fixtures | `oracle: 20/20`, `rust: 20/20`, `PASS: oracle 20/20, rust 20/20` |
| CLI matrix | `CLI matrix: 16/16`, `PASS: every RUN-001..RUN-014 row behaves like the oracle` |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| Differential (extra evidence for section 6) | generated 200/200 match; replay 1758 replayed, 0 new regressions, 0 errors |

## 6. One required correction outside the additive module

The gate rows initially failed for a real reason, not a harness reason: `exp(log(z))` disagreed
with `z` by ~0.05 at `1.5-0.5i` because `Real::atan2` divided with the ambient period-preserving
`div` and then handed `atan` the periodic value `-0.(3)`, which its series mis-evaluates
(`atan(-0.(3))` returned `-0.290049…` instead of `-0.321750…`). C# `ComplexMath.Atan2` uses
`DivideTruncate` (line 148), so its series never sees a periodic quotient.

The fix is one line in `rust/lovelace-real/src/transcendental.rs`: `Real::atan2` now uses
`div_truncate(y, x, work)`, matching the reference. It changes only atan2 results for inputs whose
quotient repeats (previously wrong); the full workspace, corpus, fixtures, CLI and the 200/200
generated + 1758 replay differential are green after it. The parent should decide whether to keep
this correction or to keep the three gate rows blocked until it lands separately.

## 7. What remains

* `parity/manifest-map.json` was **not** edited (out of scope): the 32 ids above are still
  `deferred` until the parent maps them to these tests.
* 271 of the 303 round-45 UNRUNNABLE rows remain; this block closes the falsification 32.
* The gate sweeps the reference's nine rule shapes, not a live Rust registry: `lovelace-lang` has
  no queryable rewrite-rule registry (its simplify is a monolithic function with five of the nine
  ids in its step provenance). A registry would let the gate auto-cover future rules.
* `FalsificationText.Complex`/`Decimal` and the `ToNatural` helper are not ported: no mirrored row
  calls them.
