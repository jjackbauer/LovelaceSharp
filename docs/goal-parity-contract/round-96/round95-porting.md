# Round 96 — round-95 singleton mappings applied and certified

**Status: 18/18 mappings applied (deferred -> ported), gate passed on the first attempt, coverage
certified at 6 072/6 209 (97.79 %). UnitTest1 stayed deferred.**

This lane's scope was `parity/manifest-map.json` plus the generated parity files and this report.
No Rust source, `.cs`, fixture or other doc was edited.

## 1. Result

| Item | Before | After |
|---|---|---|
| Deferred census rows | 37 | 19 |
| Ported census rows | 6 054 | 6 072 |
| Coverage | 6 054/6 209 (97.50 %) | **6 072/6 209 (97.79 %)** |
| Excluded | 118 | 118 (unchanged) |

* Exactly the 18 ids from the two round-95 reports were changed: `deferred -> ported` and given the
  `rust` test id plus an evidence note. `git diff --numstat -- parity/manifest-map.json` is
  `72 54` — 18 entries, +4/−3 lines each, no other key touched.
* `Lovelace.Representation.Tests/.../UnitTest1/Test1#0` is **still `deferred` with no `rust`
  id**: its C# body is empty, so it needs a disposition decision, not a test. It was not mapped.

Generated files rewritten by the certification sequence:

| File | Change |
|---|---|
| `parity/manifest.toml` | 18 rows regenerated from the map (126 changed lines, 18 × 7) |
| `parity/rust-tests.json` | fresh inventory: 11 crates, 5 571 listed tests, suite exit 0, 5 570 passed |
| `parity/coverage.md` | coverage report from the generated manifest |

## 2. Key diff — the 18 rows

Each row below is `parity/manifest-map.json`: `disposition` `deferred -> ported`, the `rust`
field added, `reason` replaced with the round-95 resolution and `note` replaced with the
C#-row/Rust-test evidence. The test path is the file that holds the named test.

### Numeric cluster — `docs/goal-parity-contract/round-95/numeric-singletons.md` (8)

| # | Census id (class segment) | Rust test | Test path |
|---:|---|---|---|
| 1 | `IntegerConstructorTests/Constructor_GivenInt_DelegatesToLongOverload#0` | `lovelace-int::round95_singletons::constructor_given_int_delegates_to_long_overload` | `rust/lovelace-int/src/round95_singletons.rs:9` |
| 2 | `IntegerEqualityTests/Equals_GivenReferenceToSelf_ReturnsTrue#0` | `lovelace-int::round95_singletons::equals_given_reference_to_self_returns_true` | `rust/lovelace-int/src/round95_singletons.rs:19` |
| 3 | `IntegerMultiplyTests/OperatorStar_ProducesSameResultAsMultiplyMethod#0` | `lovelace-int::round95_singletons::operator_star_produces_same_result_as_multiply_method` | `rust/lovelace-int/src/round95_singletons.rs:29` |
| 4 | `IntegerSubtractTests/SubtractOperator_IsEquivalentToSubtractMethod#0` | `lovelace-int::round95_singletons::subtract_operator_is_equivalent_to_subtract_method` | `rust/lovelace-int/src/round95_singletons.rs:41` |
| 5 | `StructuredRealExactnessTests/TerminatingQuotientBeyondTheDigitBudget_IsCertifiedExact#2` | `lovelace-real::round95_singletons::terminating_quotient_beyond_the_digit_budget_is_certified_exact` | `rust/lovelace-real/src/round95_singletons.rs:12` |
| 6 | `RealTrigFastPathTests/Cos_AtDefaultPrecision_StaysInteractive#0` | `lovelace-real::round95_singletons::cos_at_default_precision_stays_interactive` | `rust/lovelace-real/src/round95_singletons.rs:32` |
| 7 | `ArrayAllocationRefusalTests/TheSuiteDeclaresTheAllocationRefusalTheRunnerClassifies#0` | `lovelace-lang::round95_singletons::suite_declares_the_allocation_refusal_the_runner_classifies` | `rust/lovelace-lang/src/round95_singletons.rs:30` |
| 8 | `NdArrayTests/MatMul_Batched#0` | `lovelace-lang::round95_singletons::matmul_batched_multiplies_each_three_dimensional_slice` | `rust/lovelace-lang/src/round95_singletons.rs:17` |

### Engine cluster — `docs/goal-parity-contract/round-95/engine-singletons.md` (10)

| # | Census id (class segment) | Rust test | Test path |
|---:|---|---|---|
| 9 | `BroadcastTests/ColumnVector_BroadcastsToMatrix#0` | `lovelace-run::round95_singletons::broadcasttests_column_vector_broadcasts_to_matrix` | `rust/lovelace-run/src/round95_singletons.rs:115` |
| 10 | `FunctionTests/Evaluate_GivenFunctionLocalVariable_DoesNotLeakToGlobal#0` | `lovelace-run::round95_singletons::functiontests_function_local_variable_does_not_leak_to_global` | `rust/lovelace-run/src/round95_singletons.rs:144` |
| 11 | `StatementTests/Evaluate_GivenVariableAssignmentInsideBlock_DoesNotLeak#0` | `lovelace-run::round95_singletons::statementtests_block_assignment_does_not_leak` | `rust/lovelace-run/src/round95_singletons.rs:154` |
| 12 | `RecursionDepthEnvelopeTests/NestedBlocksInsideARecursiveBody_AreRefused_InsteadOfDying#0` | `lovelace-run::round95_singletons::recursiondepthenvelope_nested_blocks_inside_a_recursive_body_are_refused` | `rust/lovelace-run/src/round95_singletons.rs:44` |
| 13 | `RecursionDepthGuardTests/ADeepExpressionInsideTheRecursiveBody_IsRefused#0` | `lovelace-run::round95_singletons::recursiondepthguard_a_deep_expression_inside_the_recursive_body_is_refused` | `rust/lovelace-run/src/round95_singletons.rs:68` |
| 14 | `ArityPropertyTests/DeclaredArity_TravelsWithEveryPluginBuiltin#0` | `lovelace-lang::round95_engine_singletons::arityproperty_declared_arity_travels_with_every_plugin_builtin` | `rust/lovelace-lang/src/round95_engine_singletons.rs:272` |
| 15 | `ErrorCategoryMirrorTests/EveryMirroredTaxonomy_EqualsTheEnum#0` | `lovelace-run::round95_singletons::errorcategorymirror_every_mirrored_taxonomy_equals_the_enum` | `rust/lovelace-run/src/round95_singletons.rs:206` |
| 16 | `GoldenFixtureRecorder/RecordGoldenFixtures#0` | `lovelace-run::golden_batch::a_recorded_envelope_reproduces_every_golden` | `rust/lovelace-run/src/golden_batch.rs:131` |
| 17 | `LibraryProjectionAgreementTests/TheFacade_PublishesEveryStoredDigit_WhenTheDisplayBoundIsLower#0` | `lovelace-lang::round95_engine_singletons::libraryprojectionagreement_the_facade_publishes_every_stored_digit` | `rust/lovelace-lang/src/round95_engine_singletons.rs:388` |
| 18 | `DspIntegerArgumentContractTests/TheExactlyIntegralRealRefusal_IsStillTheContractsContract#0` | `lovelace-lang::round95_engine_singletons::dspintegerargumentcontract_the_exactly_integral_real_refusal_is_the_contracts_contract` | `rust/lovelace-lang/src/round95_engine_singletons.rs:348` |

Representative map hunk (row 16):

```diff
   "Lovelace.Run.Tests/.../GoldenFixtureRecorder/RecordGoldenFixtures#0": {
-    "disposition": "deferred",
-    "reason": "Round 45 UNRUNNABLE: ...",
-    "note": "Library-only census row; ..."
+    "disposition": "ported",
+    "rust": "lovelace-run::golden_batch::a_recorded_envelope_reproduces_every_golden",
+    "reason": "Round 95 resolved the documented block: ...",
+    "note": "C#: Lovelace.Run.Tests/GoldenFixtureRecorder.cs:35 ... Rust: rust/lovelace-run/src/golden_batch.rs:131 ..."
   },
```

## 3. Per-id verification

For every row I (a) located the test method in the crate source, (b) read its assertions against
the report's documented behaviour, and (c) confirmed the exact `crate::module::fn` id appears in
the freshly generated `parity/rust-tests.json` and that the workspace suite ran exit 0. All 18 ids
resolve; 0 unverified.

Mapping choices worth recording:

* Rows 9–13 and 15 have both a language-tier and a runner-tier Rust test. They are mapped to the
  **runner-tier** test because the deferred field in each case was a wire field
  (`$.exitCode` or `$.variables`), and the runner test drives the same `run_args` pair the
  binary uses.
* Row 14 and 17 are mapped to the language-tier tests named after the row; the CLI/facade equality
  sibling for 17 (`lovelace-run::round95_singletons::libraryprojectionagreement_the_cli_and_the_facade_publish_the_same_digits`)
  is cited in the note.
* Row 16 is mapped to the re-record property test; the recorder's inertness sibling
  (`lovelace-run::golden_batch::the_recorder_is_inert_unless_enabled`, line 120) is cited in the
  note. The recorder is deliberately inert during a gate run, so the content-identity test is the
  one that asserts the recorder's output under the gate.

## 4. Certification — observed gate lines

Sequence run from `C:\Users\ricar\dev\LovelaceSharp` with the task's toolchain environment
(`RUSTUP_HOME`, `CARGO_HOME`, `PATH`, `TMP`/`TEMP` under `rust\target\tmp`). Windows
PowerShell runs the checked-in scripts with `-ExecutionPolicy Bypass`; the `node` gates are
invoked directly.

```text
> powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\parity\gen-rust-tests.ps1
crates=11 tests=5571 suiteExit=0 passed=5570 failed=0

> node parity/gen-manifest.mjs
{
  "rows": 6209,
  "dispositions": {
    "excluded": 118,
    "ported": 6072,
    "deferred": 19
  },
  "coveragePercent": 97.79,
  "rustTests": 5571
}

> node parity/tools/gate-manifest.mjs
{
  "censusCases": 6209,
  "manifestRows": 6209,
  "dispositions": {
    "ported": 6072,
    "consolidated": 0,
    "excluded": 118,
    "deferred": 19
  },
  "coveragePercent": 97.79,
  "claimedRustTests": 6072,
  "projectsInCensus": 15,
  "failures": []
}
PASS (structural): 6209 test cases dispositioned; 6072 claim Rust coverage (97.79%).

> node parity/tools/coverage-report.mjs
coverage.md written: 6072/6209 ported (97.79%)
```

No `stale` line was reported: the fresh `sourcesSha256` in `parity/rust-tests.json`
(`0e9f80eca074ed22…`) matched the gate's walk of the current Rust sources on the first attempt.
The 3 × 45 s stale-retry path was therefore not used, and the coverage claim is made against the
gate-passing baseline.

Coverage report totals:

| C# test project | Cases | Ported | Deferred | Excluded | Coverage |
|---|---:|---:|---:|---:|---:|
| Lovelace.Integer.Tests | 148 | 148 | 0 | 0 | 100.00 % |
| Lovelace.Array.Tests | 19 | 19 | 0 | 0 | 100.00 % |
| Lovelace.Real.Tests | 2537 | 2537 | 0 | 0 | 100.00 % |
| Lovelace.Run.Tests | 751 | 751 | 0 | 0 | 100.00 % |
| Lovelace.Suite.Tests | 929 | 928 | 1 | 0 | 99.89 % |
| Lovelace.Representation.Tests | 91 | 90 | 1 | 0 | 98.90 % |
| **TOTAL** | **6209** | **6072** | **19** | **118** | **97.79 %** |

The one Representation deferred row is UnitTest1 (#0). The 19 remaining deferred rows are outside
this lane's two reports.

## 5. Deferred

`Lovelace.Representation.Tests/Lovelace.Representation.Tests.UnitTest1/Test1#0` — **still
deferred, not mapped, no Rust test invented**. Its C# body is empty; it needs a disposition
decision (delete the placeholder or exclude it with a recorded rationale), which belongs to the
ledger owner, not to this porting lane.

## 6. Files touched

* `parity/manifest-map.json` — the 18 curated entries (only).
* `parity/manifest.toml`, `parity/rust-tests.json`, `parity/coverage.md` — generated.
* `docs/goal-parity-contract/round-96/round95-porting.md` — this report.

No Rust, C#, fixture or other documentation file was modified by this lane.
