# Round 85 — round-84 evalir-multi porting map flip

**Status: complete.** The four `evalir-multi` census rows the round-84 block closed are mapped
to their row tests, and the regenerated gate resolves 4/4. Coverage moved from 5,987/6,209
(96.42 %) to **5,991/6,209 (96.49 %)**; deferred 104 -> 100; excluded unchanged at 118.

Measured 2026-11-12 (America/Sao_Paulo) on the working tree. Scope: `parity/manifest-map.json`,
the generated parity files and this report; no Rust, C#, fixture, corpus or other document was
edited by this round.

## 1. Pre-regeneration baseline — stale inventory

The checked-in inventory predated the round-84 file and the concurrent desktop lane's tests:
`parity/rust-tests.json` recorded `generatedAtUtc 2026-11-12T15:04:35Z`, suite
`{exitCode 0, passed 5290, failed 0}`, 11 crates / 5,291 tests,
`sourcesSha256 792b64e921c0536210bfcbea4515bd58329b2830e61cdae3dc2e998543016376`, and **zero**
`round84_*` names. The manifest still deferred the four rows, so the recorded coverage was:

**Pre-regen line: 5,987/6,209 ported (96.42 %), 118 excluded, 104 deferred; inventory STALE
(5,291 listed / 5,290 passed, 0 `round84_*` names), so the gate's freshness check could not
pass — the stale `sourcesSha256` 792b… was not the current source walk.**

The task snapshot quoted 5,307 passed / 1 ignored. That figure is 17 more tests than the checked-in
5,290: the round-84 file supplies 8, so 9 desktop tests were already registered when the snapshot
was taken. The desktop lane kept writing after that moment; 19 further desktop tests landed before
the final source quiescence. The measured baseline is therefore larger than the snapshot, and the
observed numbers below are the truthful baseline for the tree that was actually measured.

## 2. Quiescence and the fresh inventory (three attempts)

The desktop lane was actively writing `rust/lovelace-desktop` while this round ran, so every
measurement was gated on **90 s with no `.rs` change** and on the pre-run source walk matching
the inventory's recorded `sourcesSha256`. The inventory is stale if that hash differs.

**Attempt 1 — discarded (gate staleness).** Quiescence at 15:46:43Z (last edit `ui/app.rs`
15:45:13.991Z); `parity/gen-rust-tests.ps1` produced `generatedAtUtc 2026-11-12T15:49:38Z`,
5,325 listed / 5,324 passed, 0 failed, `sourcesSha256 ff9866a55ac94150f243cb7d6477690f3f1de5e4e5731ae0c4f98f8b7528022d`; the pre-run and post-run walks matched. With the map still
unflipped it gave 5,987/6,209 (96.42 %) and a PASS, but between that run and the post-flip gate the
desktop lane edited `engine.rs` (15:51:52Z) and `workbench.rs` (15:51:48Z); the gate correctly
failed with `rust-tests.json is stale`. The measurement was discarded.

**Attempt 2 — discarded (source changed mid-run).** Quiescence at 15:53:22Z (last edit
`engine.rs` 15:51:52.448Z); pre-run walk `25144adad791e62d3d987c5952060b2174e9332180150bdbe8afafb8d3fb9ff0`.
The run reported 5,327 listed / 5,326 passed, but the finished inventory recorded
`1926e15c5e8578bc6648d3f5f17db917089377ace4c5e1f89ed299e677fb7205` because `ui/editor.rs`
changed at 15:55:17Z inside the run window. A later fresh listing agreed with the recorded names,
but the run was not measured against one stable tree, so it was discarded rather than argued into
validity.

**Attempt 3 — clean (used for the baseline).** The tree had been quiet since 15:55:17.983Z;
pre-run walk `1926e15c5e8578bc6648d3f5f17db917089377ace4c5e1f89ed299e677fb7205`, and after the
suite the inventory recorded the same hash, so pre-run = inventory = post-run. Observed line
(`generatedAtUtc 2026-11-12T16:00:11Z`, suite exit 0):

```
crates=11 tests=5327 suiteExit=0 passed=5326 failed=0
```

5,327 listed = 5,326 passed + the workspace's one ignored test
(`property_assertions::mathir_evaluates_like_symbolic`, blocked on the known 1-ulp
`numeric::evaluate` / `IrEvaluator` difference). The inventory lists 36 tests more than the
checked-in one and **removes none**: the 8 round-84 tests plus 28 desktop tests that landed after
15:04Z (8 `completion`, 2 `language`, 5 `ui::app` completion/decoration, 5 `diagnostic_map`,
3 `engine`, 3 `workbench`, 1 `statements`, 1 `execution`). All 8 round-84 names are present.

With the fresh inventory and the map still unflipped, `node parity/gen-manifest.mjs` and
`parity/gate-manifest.ps1` gave the pre-flip baseline (observed during attempt 1, so its
`rustTests` count is 5,325; the dispositions are inventory-independent):

```
{
  "rows": 6209,
  "dispositions": {
    "excluded": 118,
    "ported": 5987,
    "deferred": 104
  },
  "coveragePercent": 96.42,
  "rustTests": 5325
}
PASS (structural): 6209 test cases dispositioned; 5987 claim Rust coverage (96.42%).
```

## 3. The four mappings

Each C# body was re-read at its census source line; each mapped Rust test is the row's own test,
not a boundary/neighbour test. The other four tests in `round84_evalir_multi.rs` — positional
binding, and the zero/one/duplicate-parameter boundaries — are supporting evidence, not census
rows, and are not mapped.

| Census id | C# row assertion | Rust test (file:line) | Why this is the row's test |
|---|---|---|---|
| `Lovelace.Symbolics.Tests/.../DxStructuredResultsTests/EvalirBatch_AcceptsNestedRowValues#0` | `DxStructuredResultsTests.cs:424`: `evalir_batch(compile_full(x + y, [x, y]), [[1, 2], [3, 4]], 40)` is `[3, 7] (Vector)`, and `compile_full` is a record | `lovelace-lang::round84_evalir_multi::evalir_batch_accepts_nested_row_values` (`rust/lovelace-lang/src/round84_evalir_multi.rs:85`) | Same script through `Engine::eval`; asserts the same `[3, 7] (Vector)` batch, the `CompilationResult` record, and `result_type` `unknown` (the reference's forward-inference answer for the unconstrained parameter). |
| `.../EvalirBatch_RowOrientedInput_EvaluatesRowByRow#0` | `DxStructuredResultsTests.cs:252`: `evalir_batch(compile(x*y + 1, [x, y]), [[1, 2], [3, 4]], 40)` is `[3, 13] (Vector)` | `lovelace-lang::round84_evalir_multi::evalir_batch_row_oriented_input_evaluates_row_by_row` (`:115`) | Same script; asserts the same `[3, 13] (Vector)` (x=1,y=2 -> 3; x=3,y=4 -> 13) and the flat row-major `[1, 2, 3, 4]` form of the same rows, so pairing is row-by-row, not re-flowed. |
| `.../EvalirBatch_RowWidthMismatch_IsRejectedWithTheDocumentedShape#0` | `DxStructuredResultsTests.cs:261`: three values in a row of a 2-parameter kernel throws `InvalidOperationException` with `evalir_batch(): kernel expects 2 parameters per row; row 0 contains 3.` | `lovelace-lang::round84_evalir_multi::evalir_batch_row_width_mismatch_is_rejected_with_the_documented_shape` (`:140`) | Same script `[[1, 2, 5], [3, 4, 6]]`; asserts the identical message and `InvalidOperation`/`DomainError`, i.e. the documented refusal rather than a silent re-flow. |
| `.../EvalirBatch_FlatInputNotAWholeNumberOfRows_IsRejected#0` | `DxStructuredResultsTests.cs:273`: `[1, 2, 3]` for a 2-wide kernel throws `InvalidOperationException` with `evalir_batch(): kernel expects 2 parameters per row; got 3 values, which is not a whole number of rows.` | `lovelace-lang::round84_evalir_multi::evalir_batch_flat_input_not_a_whole_number_of_rows_is_rejected` (`:160`) | Same script; asserts the identical whole-rows sentence and `InvalidOperation`/`DomainError`. |

The map stores for each row `disposition = "ported"`, the `lovelace-lang::round84_evalir_multi::…`
test id, a note citing the C# line, the Rust test line and the assertion, and a reason recording
the round-84 lift. The obsolete round-45 deferral reason was replaced.

## 4. Final regeneration — 4/4 resolve

After the map write (map re-read immediately before writing; only the four entries changed, all
other entries byte-identical), `node parity/gen-manifest.mjs`, `parity/gate-manifest.ps1` and
`node parity/tools/coverage-report.mjs` gave, against the clean attempt-3 inventory:

```
{
  "rows": 6209,
  "dispositions": {
    "excluded": 118,
    "ported": 5991,
    "deferred": 100
  },
  "coveragePercent": 96.49,
  "rustTests": 5327
}
```

```
{
  "censusCases": 6209,
  "manifestRows": 6209,
  "dispositions": {
    "ported": 5991,
    "consolidated": 0,
    "excluded": 118,
    "deferred": 100
  },
  "coveragePercent": 96.49,
  "claimedRustTests": 5991,
  "projectsInCensus": 15,
  "failures": []
}
PASS (structural): 6209 test cases dispositioned; 5991 claim Rust coverage (96.49%).
```

```
coverage.md written: 5991/6209 ported (96.49%)
```

**Final coverage line:** `| **TOTAL** | **6209** | **5991** | **100** | **118** | **96.49 %** | |`

Each of the four ids resolves in the generated manifest to its named Rust test, and each named test
exists in the measured inventory (suite exit 0):

| Census id suffix | Manifest rust id | Resolves |
|---|---|---|
| `EvalirBatch_AcceptsNestedRowValues#0` | `lovelace-lang::round84_evalir_multi::evalir_batch_accepts_nested_row_values` | yes |
| `EvalirBatch_RowOrientedInput_EvaluatesRowByRow#0` | `lovelace-lang::round84_evalir_multi::evalir_batch_row_oriented_input_evaluates_row_by_row` | yes |
| `EvalirBatch_RowWidthMismatch_IsRejectedWithTheDocumentedShape#0` | `lovelace-lang::round84_evalir_multi::evalir_batch_row_width_mismatch_is_rejected_with_the_documented_shape` | yes |
| `EvalirBatch_FlatInputNotAWholeNumberOfRows_IsRejected#0` | `lovelace-lang::round84_evalir_multi::evalir_batch_flat_input_not_a_whole_number_of_rows_is_rejected` | yes |

## 5. Scope and tree hygiene

Files changed by this round: `parity/manifest-map.json` (4 curated entries), the generated
`parity/rust-tests.json`, `parity/manifest.toml`, `parity/coverage.md`, and this report.
`docs/goal-parity-contract/round-84/evalir-multi.md` and the Rust/desktop files were already
modified in the shared working tree by the concurrent lane and were not touched here. No Rust
source, C# source, fixture or corpus file was edited.

Three freshness attempts were used (maximum 3): attempts 1 and 2 were discarded for a stale and a
mid-run-changed inventory respectively; attempt 3 was measured against one stable source walk and
the gate's staleness check passed.
