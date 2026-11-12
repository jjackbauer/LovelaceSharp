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
was taken; the remaining 17 desktop tests landed before source quiescence was reached (§2). The
measured baseline is therefore larger than the snapshot, and the observed numbers below are the
truthful baseline for the tree that was actually measured, not the earlier hint.

## 2. Quiescence and the fresh inventory

The desktop lane was actively writing `rust/lovelace-desktop` when this round started and for
most of the first six minutes (its last edit was `ui/app.rs` at 15:45:13.991Z). After **90 s with
no `.rs` change** (reached at 2026-11-12T15:46:43Z, attempt 1 of 3) the source walk was hashed, then
`parity/gen-rust-tests.ps1` was run. The pre-run walk, the inventory's recorded
`sourcesSha256` and the post-run walk are the same value (`ff9866a55ac94150f243cb7d6477690f3f1de5e4e5731ae0c4f98f8b7528022d`), so the run was not discarded for staleness and the gate's
freshness check passed.

Observed line (`generatedAtUtc 2026-11-12T15:49:38Z`, suite exit 0):

```
crates=11 tests=5325 suiteExit=0 passed=5324 failed=0
```

5,325 listed = 5,324 passed + the workspace's one ignored test
(`property_assertions::mathir_evaluates_like_symbolic`, blocked on the known 1-ulp
`numeric::evaluate` / `IrEvaluator` difference). The inventory lists 34 tests more than the
checked-in one and **removes none**: the 8 round-84 tests plus 26 desktop tests that landed after
15:04Z (8 `completion`, 2 `language`, 5 `ui::app` completion/decoration, 5 `diagnostic_map`,
3 `engine`, 1 `statements`, 1 `workbench`, 1 `execution`). All 8 round-84 names are present.

With the fresh inventory and the map still unflipped, `node parity/gen-manifest.mjs` and
`parity/gate-manifest.ps1` gave the pre-flip baseline:

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
test id, and a note citing the C# line, the Rust test line and the assertion. The obsolete round-45
deferral reason was removed with the flip.

## 4. Final regeneration — 4/4 resolve

After the map write (map re-read immediately before writing; only the four entries changed, all
other entries byte-identical), `node parity/gen-manifest.mjs`, `parity/gate-manifest.ps1` and
`node parity/tools/coverage-report.mjs` gave:

```
{
  "rows": 6209,
  "dispositions": {
    "excluded": 118,
    "ported": 5991,
    "deferred": 100
  },
  "coveragePercent": 96.49,
  "rustTests": 5325
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

One freshness attempt was needed: the tree was quiescent for 90 s before the measurement and
remained unchanged through the suite run (pre-run = inventory = post-run `sourcesSha256`), so the
gate's staleness check passed with no discard/retry.
