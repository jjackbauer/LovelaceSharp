# Round 68 — Round 67 porting: 27 mappings and a truthful baseline

**Status: complete.** The 27 census rows unblocked by the two round-67 lanes — the five
`PolynomialsApiTests` polynomial-algebra rows and 22 of the 23 deferred Modus-family rows — are now
`ported` and resolve in the regenerated measured inventory. `ParameterDomainOf_MapsEveryDeclaredArm#0`
remains deferred for the documented Symbolics-only reason.

Coverage moved **5,910 / 6,209 (95.18 %)** -> **5,937 / 6,209 (95.62 %)**; deferred 181 -> 154;
exclusions unchanged at 118. The map changed by exactly 27 entries; no other key changed.

Regeneration came first. The inherited inventory had been measured before the two lanes delivered:
`generatedAtUtc` 2026-11-12T03:06:54Z, 5,099 listed tests, suite 5,098 passed / 0 failed, sources hash
`353d7478c61cfa2fed232532e5f1696d5d2b16de1ba7c7ccbf778e6d89ca6c37`. The current sources hash to
`ae3d36e3ee7ea8575f2363005dd33fc5a49ef614f88fc789a7df8856a9a7b109`; the regenerated inventory has
5,137 tests and a green suite of 5,136 passed / 0 failed / 1 ignored. The +38 listed tests are exactly
the two lane modules (16 `round67_polynomial_algebra` + 22 `round67_modus`).

This is a mapping round only: `parity/manifest-map.json` gained the 27 dispositions and
`parity/manifest.toml`, `parity/rust-tests.json` and `parity/coverage.md` were regenerated. No Rust
source, `.cs` file, fixture, corpus file or other document was edited.

Measured 2026-11-12 (America/Sao_Paulo) on `HEAD 42fb0b2` plus the uncommitted round-67 lane work. The
tree was quiescent during the measurement; this lane was the only writer to `parity/`.

## 1. Regeneration first

The inherited inventory was stale by the 38 tests the two round-67 lanes added (both modules are new,
untracked files). Regeneration ran before any gate measurement, as the objective required:

| Step | Command | Observed | Exit |
|---|---|---|---:|
| Rust inventory + suite | `powershell -File parity/gen-rust-tests.ps1` | `crates=11 tests=5137 suiteExit=0 passed=5136 failed=0` | 0 |
| Ignored tests | `cargo test --workspace -- --list --ignored` (`rust/`) | exactly one: `property_assertions::mathir_evaluates_like_symbolic` | 0 |
| Manifest generation (pre-mapping) | `node parity/gen-manifest.mjs` | `rows 6209`, `ported 5910`, `deferred 181`, `excluded 118`, `coveragePercent 95.18`, `rustTests 5137` | 0 |
| Manifest gate (pre-mapping) | `node parity/tools/gate-manifest.mjs` | `"claimedRustTests": 5910`, `"failures": []`; `PASS (structural): 6209 test cases dispositioned; 5910 claim Rust coverage (95.18%).` | 0 |

```json
{
  "rows": 6209,
  "dispositions": { "excluded": 118, "ported": 5910, "deferred": 181 },
  "coveragePercent": 95.18,
  "rustTests": 5137
}
```

The regenerated inventory's `sourcesSha256` matches the Rust sources (the gate re-verifies it), so the
pre-mapping gate is green against a fresh measurement. The one ignored test is the known MathIR row
(`mathir_evaluates_like_symbolic`); the inventory's 5,137 listed tests are 5,136 passing plus that one
ignored test.

## 2. The 27 mappings

The map was re-read from disk immediately before the write. Every target was still `deferred` with no
`rust` value; each entry changed only `disposition` (`deferred` -> `ported`), gained its `rust` value,
and replaced the round-45 reason/note with the round-67 evidence.

### 2.1 Five `PolynomialsApiTests` rows (`lovelace-symbolics`)

All tests live in `rust/lovelace-symbolics/src/round67_polynomial_algebra.rs`; the production surface is
`rust/lovelace-symbolics/src/polynomials.rs` (round-67 report `polynomial-algebra.md`).

| # | Census id | Rust test |
|---:|---|---|
| 1 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PolynomialsApiTests/Content_PrimitivePart_SplitCoefficients#0` | `lovelace-symbolics::round67_polynomial_algebra::content_primitive_part_split_coefficients` |
| 2 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PolynomialsApiTests/Gcd_Lcm_AreConsistent#0` | `lovelace-symbolics::round67_polynomial_algebra::gcd_lcm_are_consistent` |
| 3 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PolynomialsApiTests/Resultant_MatchesKnownValues#0` | `lovelace-symbolics::round67_polynomial_algebra::resultant_matches_known_values` |
| 4 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PolynomialsApiTests/Discriminant_MatchesKnownValues#0` | `lovelace-symbolics::round67_polynomial_algebra::discriminant_matches_known_values` |
| 5 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PolynomialsApiTests/Factor_QuarticWithRationalRoots_FourLinearFactors#0` | `lovelace-symbolics::round67_polynomial_algebra::factor_quartic_with_rational_roots_four_linear_factors` |

### 2.2 Twenty-two Modus seam rows (`lovelace-lang`)

All tests live in `rust/lovelace-lang/src/round67_modus.rs` (round-67 report `modus-seams.md`).

| # | Census id | Rust test |
|---:|---|---|
| 1 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.BuiltinArgumentShapeGuardTests/ACoercionFailureOnAnEngineInternalValue_IsNotDisguisedAsAnArgumentError#0` | `lovelace-lang::round67_modus::coercion_failure_on_an_engine_internal_value_stays_internal` |
| 2 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.BuiltinArgumentShapeGuardTests/ARegisteredBuiltinThatCastsItsOwnPayload_IsAttributedToTheArgumentItReadLast#0` | `lovelace-lang::round67_modus::registered_builtin_that_casts_its_payload_is_attributed_to_the_last_argument` |
| 3 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.BuiltinArgumentShapeGuardTests/ARegisteredBuiltinThatCoercesItsArgument_IsAttributedToTheBuiltinAndThePosition#0` | `lovelace-lang::round67_modus::registered_builtin_that_coerces_its_argument_is_attributed_to_the_position` |
| 4 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.DspPluginTests/DspPlugin_GivenUnregisteredEngine_ThrowsUnknownFunction#0` | `lovelace-lang::round67_modus::dsp_plugin_unregistered_engine_refuses_fft` |
| 5 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.DspPluginTests/DspPlugin_LoadedTwice_Throws#0` | `lovelace-lang::round67_modus::dsp_plugin_loaded_twice_is_refused` |
| 6 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.DspPluginTests/PluginBuiltin_GivenCoreNameCollision_Throws#0` | `lovelace-lang::round67_modus::plugin_builtin_core_name_collision_is_refused` |
| 7 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.EnumPayloadSeamTests/EnumValue_RoundTripsThroughTheModusSeam#0` | `lovelace-lang::round67_modus::enum_value_round_trips_through_the_modus_seam` |
| 8 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ModusArityTests/ExactCount_RequiresTheDeclaredCount#0` | `lovelace-lang::round67_modus::exact_count_requires_the_declared_count` |
| 9 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ModusArityTests/MinArity_AllowsTheOptionalTail_AndStillRefusesLess#0` | `lovelace-lang::round67_modus::min_arity_allows_the_optional_tail_and_still_refuses_less` |
| 10 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ModusArityTests/ParametersOnlyRegistration_UsesTheSameRule#0` | `lovelace-lang::round67_modus::parameters_only_registration_uses_the_same_rule` |
| 11 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ModusArityTests/Variadic_RemovesTheUpperBound_AndHonoursTheLowerOne#0` | `lovelace-lang::round67_modus::variadic_removes_the_upper_bound_and_honours_the_lower_one` |
| 12 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ModusTests/Dispatch_UnsupportedElementType_FallsBack#0` | `lovelace-lang::round67_modus::dispatch_unsupported_element_type_falls_back` |
| 13 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ModusTests/DspPlugin_LoadedThroughModusSeam_RegistersDspBuiltins#0` | `lovelace-lang::round67_modus::dsp_plugin_loaded_through_modus_seam_registers_dsp_builtins` |
| 14 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ModusTests/Kernel_DeclinesUnsupportedOp#0` | `lovelace-lang::round67_modus::kernel_declines_unsupported_op` |
| 15 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ModusTests/PluginLoad_RegistersKernel_AndDispatchRuns#0` | `lovelace-lang::round67_modus::plugin_load_registers_kernel_and_dispatch_runs` |
| 16 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ModusTests/RealAddKernel_GivenRealOperands_MatchesDirectRealAddition#0` | `lovelace-lang::round67_modus::real_add_kernel_matches_direct_real_addition` |
| 17 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ModusTests/ScalarResult_GivenBooleanAndText_ReturnValues#0` | `lovelace-lang::round67_modus::scalar_result_boolean_and_text_return_values` |
| 18 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecordValueTests/MemberAccess_OnNonRecord_ExplainsRequirement#0` | `lovelace-lang::round67_modus::member_access_on_a_non_record_explains_the_requirement` |
| 19 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecordValueTests/MemberAccess_UnknownField_NamesAvailableMembers#0` | `lovelace-lang::round67_modus::member_access_unknown_field_names_available_members` |
| 20 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecordValueTests/Record_CrossesModusPluginBoundary#0` | `lovelace-lang::round67_modus::record_crosses_the_modus_plugin_boundary` |
| 21 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.SharedPluginInstanceAssumptionIsolationTests/CallerInstalledAssumptionScope_KeepsPrecedenceInsideAnEngineEvaluation#0` | `lovelace-lang::round67_modus::caller_installed_assumption_scope_keeps_precedence_inside_an_engine_evaluation` |
| 22 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.SharedPluginInstanceAssumptionIsolationTests/OnePluginTwoEngines_AssumptionCommittedWhileAIsRunning_DoesNotDecideB#0` | `lovelace-lang::round67_modus::assumption_committed_while_a_is_running_does_not_decide_b` |

The 24-row family's 24th member,
`DspPluginTests/DspPlugin_GivenUntouchedKnob_ComputesAtFastBudget_AndPromotesWhenRaised#0`, was already
`ported` before this round (`lovelace-run::dsp_round21::...`) and is not remapped. The 23rd,
`ParameterDomainOf_MapsEveryDeclaredArm#0`, is the one that stays deferred.

## 3. Resolution: 27/27

A per-mapping check read the measured inventory (`parity/rust-tests.json`), the curated map and the
generated manifest and required all four facts: the id is in the census, the map entry is `ported`, the
manifest row is `ported` with the same `rust`, and the `rust` name exists in the measured inventory.

```text
resolved 27 / 27
unresolved []
changed keys vs HEAD 27
changed outside targets 0
inventory lovelace-lang::round67_modus tests 22
inventory lovelace-symbolics::round67_polynomial_algebra tests 16
```

**27/27 resolved. No id was unresolved and no test name was guessed.** Every name the two round-67
reports supplied is present in the regenerated inventory; the five polynomial rows are five of the 16
polynomial-algebra tests, and all 22 Modus ids have their own test (the inventory lists 22 tests in
`round67_modus`).

## 4. Final coverage and the row that remains deferred

```text
coverage.md written: 5937/6209 ported (95.62%)
PASS (structural): 6209 test cases dispositioned; 5937 claim Rust coverage (95.62%).
```

| Project | Before | After | Deferred |
|---|---|---|---|
| Lovelace.Suite.Tests | 869 / 929 (93.54 %), 60 deferred | **891 / 929 (95.91 %)**, 38 deferred | -22 |
| Lovelace.Symbolics.Tests | 1,140 / 1,226 (92.99 %), 86 deferred | **1,145 / 1,226 (93.39 %)**, 81 deferred | -5 |
| **TOTAL** | **5,910 / 6,209 (95.18 %)**, 181 deferred | **5,937 / 6,209 (95.62 %)**, 154 deferred | **-27** |

The one row of the round-67 family that remains deferred is:

`Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DxStructuredResultsTests/ParameterDomainOf_MapsEveryDeclaredArm#0`

Its reason (round-67 `modus-seams.md` §6): the row calls the internal
`SymbolicsPlugin.ParameterDomainOf(ParameterDomain)`, including an undeclared `(ParameterDomain)99` that
must throw. It needs a public/total `ParameterDomain` mapping inside `lovelace-symbolics`; both declared
arms map to `MathDomain.Integer` and the undeclared value must be refused. The lane did not add a
`lovelace-lang` duplicate because that would be a shim, not the kernel's own projection. The manifest
keeps the round-45 documented-block reason, which remains true ("no Rust API exists to assert the row's
contract"); the round-67 report records the specific current blocker.

Of the 6,209 census cases, 154 remain deferred overall; `ParameterDomainOf` is the only remaining row
of the 24-row round-67 family. Exclusions are unchanged at 118; no row is `consolidated`.

## 5. Gate outputs after the mapping

| Step | Command | Observed | Exit |
|---|---|---|---:|
| Manifest generation | `node parity/gen-manifest.mjs` | `rows 6209`, `ported 5937`, `deferred 154`, `excluded 118`, `coveragePercent 95.62`, `rustTests 5137` | 0 |
| Manifest gate | `node parity/tools/gate-manifest.mjs` | `"claimedRustTests": 5937`, `"failures": []`; `PASS (structural): 6209 test cases dispositioned; 5937 claim Rust coverage (95.62%).` | 0 |
| Coverage report | `node parity/tools/coverage-report.mjs` | `coverage.md written: 5937/6209 ported (95.62%)` | 0 |

```json
{
  "censusCases": 6209,
  "manifestRows": 6209,
  "dispositions": { "ported": 5937, "consolidated": 0, "excluded": 118, "deferred": 154 },
  "coveragePercent": 95.62,
  "claimedRustTests": 5937,
  "projectsInCensus": 15,
  "failures": []
}
```

## 6. Map-write evidence and scope

* Before: 3,629,730 bytes, 6,209 keys, SHA-256
  `b267c1987dc61403db2b2a2036ae21ef650712437568369640fedeaa7b6add15`.
* After: 3,635,734 bytes, 6,209 keys, SHA-256
  `7b8a4b68a86c73254d54881e3b6815096ab62d543806da933cd0b023d898e99e`.
* `git diff --numstat parity/manifest-map.json` is `108 81`: 27 entries × (4 added lines — `disposition`,
  `rust`, `reason`, `note`; 3 removed — `disposition`, old `reason`, old `note`). A per-key comparison
  against `HEAD` reports exactly 27 changed entries, all `deferred` -> `ported`, and 0 changed keys
  outside the target set. All 6,209 keys remain and the JSON parses.
* `parity/manifest.toml` diff is `108 81` (the same 27 rows); `parity/coverage.md` diff is `3 3` (the
  Suite line, the Symbolics line and TOTAL); `parity/rust-tests.json` is regenerated (5,137 tests, fresh
  sources hash).
* **Scope kept:** `parity/manifest-map.json` plus the generated parity files and this report. No `rust/`
  source, `.cs` file, fixture, corpus file or other document was edited. The inherited uncommitted
  round-67 Rust changes are untouched.

## 7. Reproduction

```powershell
$repo = 'C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME = "$repo\.toolchain\rustup"
$env:CARGO_HOME  = "$repo\.toolchain\cargo"
$env:PATH = "$repo\.toolchain\cargo\bin;" + $env:PATH
$env:TMP = "$repo\rust\target\tmp"; $env:TEMP = $env:TMP
Set-Location $repo

powershell -File parity/gen-rust-tests.ps1   # crates=11 tests=5137 suiteExit=0 passed=5136 failed=0
node parity/gen-manifest.mjs                  # pre-mapping: 5910 ported (95.18%)
node parity/tools/gate-manifest.mjs           # pre-mapping: PASS
# map edit: 27 deferred -> ported entries, re-read immediately before the write
node parity/gen-manifest.mjs                  # 5937 ported (95.62%)
node parity/tools/gate-manifest.mjs           # PASS
node parity/tools/coverage-report.mjs         # coverage.md written: 5937/6209 ported (95.62%)
Set-Location "$repo\rust"; cargo test --workspace -- --list --ignored   # 1 ignored
```
