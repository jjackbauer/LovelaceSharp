# Round 49 — the 28 round-48 rows, mapped

**Status: complete.** The three round-48 lanes unblocked 28 census rows — 9
`Lovelace.Suite.Tests.RecordSchemaTests`, 10 `Lovelace.Dsp.Tests.FixedDspTests`, and 9
`Lovelace.Real.Tests.RealTrigFastPathTests`. All 28 are now `ported` in
`parity/manifest-map.json` and resolve through the manifest gate to tests in the measured
inventory. Coverage moved from **5,788/6,209 (93.22 %)** to **5,816/6,209 (93.67 %)**;
deferred fell 303 → 275 and exclusions are unchanged at 118. The manifest gate reports
`"failures": []`. No Rust, C#, fixture, corpus or other documentation file was edited.

## 1. What changed

The map already carried one entry per census id: all 28 keys existed as `deferred`
placeholders. The edit therefore **changed 28 values, in place** — it did not add or remove
keys (6,209 keys before and after). Each changed entry now carries:

* `"disposition": "ported"`;
* `"rust"`: the crate-qualified test id from the lane report, verbatim;
* `"note"`: the C# row and Rust test file, plus the report section it was read from.

The stale deferred `reason` was removed from each mapped row, so no `ported` row carries a
blocking reason. `git diff --numstat parity/manifest-map.json` shows `84 84` — 28 rows ×
(disposition, note, rust) lines, and no other line touched.

Generated files: `parity/rust-tests.json` (regenerated), `parity/manifest.toml`
(regenerated), `parity/coverage.md` (regenerated).

## 2. Inputs, and what each was trusted for

| Input | Rows | Mapping |
|---|---:|---|
| `docs/goal-parity-contract/round-48/record-schemas.md` §4 | 9 | `Lovelace.Suite.Tests/…/RecordSchemaTests/<method>#0` → `lovelace-run::record_schema_batch::<test>` |
| `docs/goal-parity-contract/round-48/fixed-point-dsp.md` §4 | 10 | `Lovelace.Dsp.Tests/…/FixedDspTests/<method>#0` → `lovelace-lang::fixed_dsp_tests::<test>` |
| `docs/goal-parity-contract/round-48/real-fast-path.md` §5 | 9 | `Lovelace.Real.Tests/…/RealTrigFastPathTests/<method>#0` → `lovelace-real::fast_path_tests::<test>` |

Census ids were taken from `parity/census.json`, not retyped from the reports: all 28 exist
in the census exactly once. Test names were taken from the reports and then **verified
against the freshly measured inventory** (`parity/rust-tests.json`, 4,961 tests, suite exit
0), not assumed. The tenth Real row — `Cos_AtDefaultPrecision_StaysInteractive#0` — was
deliberately left `deferred` (the lane measured 30.2 s against its 10 s bound; see
`real-fast-path.md` §6) and is not mapped.

## 3. Verification performed before the write

| Check | Observed |
|---|---|
| Census ids present | 28/28 in `parity/census.json` |
| Rust names present in the measured inventory | 28/28; the 9 `record_schema_batch`, 10 `fixed_dsp_tests` and 9 `fast_path_tests` names are all listed |
| Names unique | 28 distinct census ids and 28 distinct Rust test ids |
| Rows previously `deferred` | 28/28 (only disposition/rust/note changed) |
| Names not in the inventory | **none** — no id was guessed or dropped |
| `Cos_AtDefaultPrecision_StaysInteractive#0` | still `deferred`, as the lane required |

## 4. The 28 mappings

### 4.1 Record schemas — `Lovelace.Suite.Tests.RecordSchemaTests` (9)

| Census id | Rust test |
|---|---|
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecordSchemaTests/DiagnosticSchema_DeclaresTheSixFrozenFields#0` | `lovelace-run::record_schema_batch::diagnostic_schema_declares_the_six_frozen_fields` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecordSchemaTests/EveryProducedRecord_ConformsToItsDeclaredSchema#0` | `lovelace-run::record_schema_batch::every_produced_record_conforms_to_its_declared_schema` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecordSchemaTests/SingularMatrix_IsACompleteNoSolutions_CarryingAStructuredDiagnostic#0` | `lovelace-run::record_schema_batch::singular_matrix_inv_full_is_a_complete_no_solutions_carrying_a_structured_diagnostic` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecordSchemaTests/SingularMatrix_IsACompleteNoSolutions_CarryingAStructuredDiagnostic#1` | `lovelace-run::record_schema_batch::singular_matrix_linsolve_full_is_a_complete_no_solutions_carrying_a_structured_diagnostic` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecordSchemaTests/SolvedMatrix_IsComplete_WithAnEmptyDiagnosticsArray#0` | `lovelace-run::record_schema_batch::solved_matrix_inv_full_is_complete_with_an_empty_diagnostics_array` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecordSchemaTests/SolvedMatrix_IsComplete_WithAnEmptyDiagnosticsArray#1` | `lovelace-run::record_schema_batch::solved_matrix_linsolve_full_is_complete_with_an_empty_diagnostics_array` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecordSchemaTests/SolverSchemas_DeclareTheStructuralKinds#0` | `lovelace-run::record_schema_batch::solver_schemas_declare_the_structural_kinds` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecordSchemaTests/UnknownRecordType_IsReportedAsUnknown_NotAsConforming#0` | `lovelace-run::record_schema_batch::unknown_record_type_is_reported_as_unknown_not_as_conforming` |
| `Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecordSchemaTests/Validate_ReportsMissingAndUndeclaredFields#0` | `lovelace-run::record_schema_batch::validate_reports_missing_and_undeclared_fields` |

### 4.2 Fixed-point DSP — `Lovelace.Dsp.Tests.FixedDspTests` (10)

| Census id | Rust test |
|---|---|
| `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.FixedDspTests/Convolve64_GivenTwoOnes_ProducesTriangle#0` | `lovelace-lang::fixed_dsp_tests::convolve64_given_two_ones_produces_triangle` |
| `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.FixedDspTests/Convolve64_Given123SelfConvolution_MatchesDspMath#0` | `lovelace-lang::fixed_dsp_tests::convolve64_given_123_self_convolution_matches_dsp_math` |
| `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.FixedDspTests/Convolve128_Given123SelfConvolution_MatchesDspMath#0` | `lovelace-lang::fixed_dsp_tests::convolve128_given_123_self_convolution_matches_dsp_math` |
| `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.FixedDspTests/Convolve64_GivenWideOperands_ThrowsPromoteException#0` | `lovelace-lang::fixed_dsp_tests::convolve64_given_wide_operands_throws_promote_exception` |
| `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.FixedDspTests/Convolve128_GivenWideOperands_ThrowsPromoteException#0` | `lovelace-lang::fixed_dsp_tests::convolve128_given_wide_operands_throws_promote_exception` |
| `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.FixedDspTests/ImpulseResponse64_GivenFir_MatchesTaps#0` | `lovelace-lang::fixed_dsp_tests::impulse_response64_given_fir_matches_taps` |
| `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.FixedDspTests/ImpulseResponse128_GivenFir_MatchesTaps#0` | `lovelace-lang::fixed_dsp_tests::impulse_response128_given_fir_matches_taps` |
| `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.FixedDspTests/MovingAverage64_GivenPowerOfTwoWindow_MatchesDspMath#0` | `lovelace-lang::fixed_dsp_tests::moving_average64_given_power_of_two_window_matches_dsp_math` |
| `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.FixedDspTests/MovingAverage128_GivenPowerOfTwoWindow_MatchesDspMath#0` | `lovelace-lang::fixed_dsp_tests::moving_average128_given_power_of_two_window_matches_dsp_math` |
| `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.FixedDspTests/MovingAverage64_GivenNonPositiveWindow_ThrowsArgumentOutOfRange#0` | `lovelace-lang::fixed_dsp_tests::moving_average64_given_non_positive_window_throws_argument_out_of_range` |

### 4.3 Real fast paths — `Lovelace.Real.Tests.RealTrigFastPathTests` (9 of 10)

| Census id | Rust test |
|---|---|
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigFastPathTests/SpecialAngles_BuiltTheOriginalWay_StillHitTheTableExactly#0` | `lovelace-real::fast_path_tests::special_angles_built_the_original_way_still_hit_the_table_exactly` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigFastPathTests/NonSpecialAngles_AreNotCapturedByTheTable#0` | `lovelace-real::fast_path_tests::non_special_angles_are_not_captured_by_the_table` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigFastPathTests/ArgumentReduction_IntegerQuotient_MatchesTheDividedForm#0` | `lovelace-real::fast_path_tests::argument_reduction_integer_quotient_matches_the_divided_form` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigFastPathTests/SeriesTermination_IntegerTest_MatchesTheThresholdComparison#0` | `lovelace-real::fast_path_tests::series_termination_integer_test_matches_the_threshold_comparison` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigFastPathTests/SeriesTermination_IntegerTest_TracksTheCarriedPowerAcrossManyTerms#0` | `lovelace-real::fast_path_tests::series_termination_integer_test_tracks_the_carried_power_across_many_terms` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigFastPathTests/Normalize_BinaryStrip_MatchesTheDigitStringStrip#0` | `lovelace-real::fast_path_tests::normalize_binary_strip_matches_the_digit_string_strip` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigFastPathTests/ToInteger_BinaryShift_MatchesTheDigitPadding#0` | `lovelace-real::fast_path_tests::to_integer_binary_shift_matches_the_digit_padding` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigFastPathTests/HalfOf_MatchesTheDividedForm_ForPiShapedValues#0` | `lovelace-real::fast_path_tests::half_of_matches_the_divided_form_for_pi_shaped_values` |
| `Lovelace.Real.Tests/Lovelace.Real.Tests.RealTrigFastPathTests/Normalize_BinaryStrip_MatchesTheDigitStringStrip_ForLongZeroRuns#0` | `lovelace-real::fast_path_tests::normalize_binary_strip_matches_the_digit_string_strip_for_long_zero_runs` |

`Cos_AtDefaultPrecision_StaysInteractive#0` is intentionally absent: it remains `deferred`.

## 5. Gate and coverage outputs, as observed

Run on 2026-11-11 (America/Sao_Paulo) with the pinned toolchain
`cargo 1.99.1 (5f94df478 2026-08-27)`. The regenerated inventory recorded
`generatedAtUtc = 2026-11-11T22:46:29Z` and `sourcesSha256 = 54d761fd6b03b307b197ff42b15bc32a24416d6ec30ac1565d68c94782a473d4`.

| Step | Command | Observed line | Exit |
|---|---|---|---:|
| Inventory + suite | `powershell -File parity/gen-rust-tests.ps1` | `crates=11 tests=4961 suiteExit=0 passed=4961 failed=0` | 0 |
| Manifest | `node parity/gen-manifest.mjs` | `{"rows": 6209, "dispositions": {"excluded": 118, "ported": 5816, "deferred": 275}, "coveragePercent": 93.67, "rustTests": 4961}` | 0 |
| Manifest gate | `node parity/tools/gate-manifest.mjs` | `"failures": []`; `PASS (structural): 6209 test cases dispositioned; 5816 claim Rust coverage (93.67%).` | 0 |
| Coverage report | `node parity/tools/coverage-report.mjs` | `coverage.md written: 5816/6209 ported (93.67%)` | 0 |

After regeneration, every one of the 28 rows was re-read from the **generated**
`parity/manifest.toml`: each has `disposition = "ported"` with the expected `rust = …`
line, and that id exists in `parity/rust-tests.json`. **28/28 resolve; 0 unresolved.**
The manifest gate independently claims 5,816 Rust tests, up exactly 28 from 5,788.

## 6. Coverage by affected project (`parity/coverage.md`)

| C# project | Cases | Ported | Deferred | Excluded | Coverage |
|---|---:|---:|---:|---:|---:|
| Lovelace.Dsp.Tests | 61 | 53 | 8 | 0 | 86.89 % |
| Lovelace.Real.Tests | 2537 | 2536 | 1 | 0 | 99.96 % |
| Lovelace.Suite.Tests | 929 | 842 | 87 | 0 | 90.64 % |
| **TOTAL** | **6209** | **5816** | **275** | **118** | **93.67 %** |

The three moves are exactly the expected deltas: Suite 833 → 842 (+9), Dsp 43 → 53 (+10),
Real 2527 → 2536 (+9); +28 total. Deferred falls 303 → 275; exclusions are untouched at 118.

## 7. Notes and residual

* **Scope:** this round changed only `parity/manifest-map.json`, the generated parity files
  (`rust-tests.json`, `manifest.toml`, `coverage.md`) and this report. No Rust, C#, fixture,
  corpus or contract file was edited.
* **Inherited tree state:** the three round-48 lanes' new Rust files and `lib.rs`/`main.rs`
  registrations are in the working tree; this round did not modify them. The fresh
  `sourcesSha256` covers that tree, which is why the gate accepts the mappings.
* **Residual:** 275 rows remain `deferred` and 118 remain `excluded` of 6,209 census cases.
  The Real fast-path lane's `Cos_AtDefaultPrecision_StaysInteractive#0` remains deferred
  pending a production decision on the 1000-place cosine cost (`real-fast-path.md` §6).
