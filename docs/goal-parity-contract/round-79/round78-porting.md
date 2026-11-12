# Round 79 — round-78 lane porting map flip

**Status: complete.** The 8 census rows the round-78 scoped-evaluation and StructuralEquality
lanes unblocked are mapped to their row tests, and the regenerated gate resolves 8/8. Coverage
moved from 5,963/6,209 (96.04 %) to 5,971/6,209 (96.17 %); deferred 128 -> 120; excluded unchanged
at 118.

Measured 2026-11-12 (America/Sao_Paulo) on the working tree. Scope: `parity/manifest-map.json`
plus the generated parity files and this report; no Rust, C#, fixture or other document was edited.

## 1. Pre-regeneration baseline — stale inventory

The checked-in inventory predated the round-78 Rust modules: `parity/rust-tests.json` recorded
`generatedAtUtc 2026-11-12T13:11:06Z`, `suite {exitCode 0, passed 5219, failed 0}`, 11 crates /
5,220 tests, and no `round78_*` test name. The gate therefore failed on freshness alone, while the
checked-in map still deferred all eight rows:

```
"dispositions": { "ported": 5963, "consolidated": 0, "excluded": 118, "deferred": 128 },
"coveragePercent": 96.04,
"claimedRustTests": 5963,
"failures": ["rust-tests.json is stale: the Rust sources changed since the inventory was measured (run parity/gen-rust-tests.ps1)"]
FAIL: 1 issue(s)
```

**Pre-regen line: 5,963/6,209 ported (96.04 %), 118 excluded, 128 deferred, gate FAIL (stale
inventory).**

## 2. First regeneration — re-established baseline

Commands: `parity/gen-rust-tests.ps1`, `node parity/gen-manifest.mjs`,
`parity/gate-manifest.ps1`. Observed lines:

```
crates=11 tests=5248 suiteExit=0 passed=5247 failed=0
```

```
{
  "rows": 6209,
  "dispositions": {
    "excluded": 118,
    "ported": 5963,
    "deferred": 128
  },
  "coveragePercent": 96.04,
  "rustTests": 5248
}
```

```
PASS (structural): 6209 test cases dispositioned; 5963 claim Rust coverage (96.04%).
```

The fresh inventory (`generatedAtUtc 2026-11-12T13:39:41Z`, suite exit 0) lists 5,248 tests and
contains all 20 `round78_*` tests (5 scoped-eval, 15 structural-equality). The measured workspace
total is **5,247 passed / 0 failed / 1 ignored** (5,248 tested). That is 28 more than the stale
5,219-passed inventory: the 20 round-78 tests plus 8 tests from the concurrent lanes (5
`lovelace-symbolics::cancel::tests::*`, 2 `lovelace-lang::kernel_cancel_round9::*`, and 1
`lovelace-desktop` test). The task context's earlier 5,244-passed measurement is superseded by
this fresh run.

After this first regeneration the baseline is truthful but unchanged in coverage:
**5,963/6,209 (96.04 %), 118 excluded, 128 deferred, gate PASS**.

## 3. The eight mappings

Each C# body was read at its census source line; each mapped Rust test is the row's own test, not
a supporting/neighbour test. The scoped-eval module's fifth test,
`a_guard_decided_without_assumptions_reports_none`, pins the "no assumptions needed" branch and is
not mapped. The structural-equality module's eleven supporting tests (commutativity, record/enum/
array paths, assumptions non-interaction, ...) are not mapped either, and the class's fifth row,
`IdenticalRecords_AreEqual#0`, was already ported in round 19 and was not touched.

| Census id | C# assertion | Rust test | Why this is the row's test |
|---|---|---|---|
| `Lovelace.Symbolics.Tests/.../PiecewiseAssumptionTests/MirroredSpelling_IsAlsoDecided#0` | `PiecewiseAssumptionTests.cs:57`: guard written constant-first `0 < x`; under `[x > 0]`, `EvaluateToNum(piecewise(1 if guard, 2))` equals `1` | `lovelace-lang::round78_scoped_eval::mirrored_spelling_is_also_decided` (`round78_scoped_eval.rs:90`) | Same constant-first guard, same scope, same value; additionally pins the reported condition `x > 0`. |
| `Lovelace.Symbolics.Tests/.../PiecewiseAssumptionTests/ProvenGuard_TakesTheBranch#0` | `PiecewiseAssumptionTests.cs:42`: guard `x >= 0`; under `[x > 0]` the value equals `1` | `lovelace-lang::round78_scoped_eval::proven_guard_takes_the_branch` (`round78_scoped_eval.rs:76`) | Same guard and scope, same value `1`; the reported condition is the assumed atom `x > 0`, not the weaker guard. |
| `Lovelace.Symbolics.Tests/.../PiecewiseAssumptionTests/RefutedGuard_TakesTheOtherwiseBranch#0` | `PiecewiseAssumptionTests.cs:27`: guard `x >= 0`; under `[x < 0]` the value equals `2` | `lovelace-lang::round78_scoped_eval::refuted_guard_takes_the_otherwise_branch` (`round78_scoped_eval.rs:62`) | Same guard and scope, same otherwise value `2`; conditions equal `[x < 0]`. |
| `Lovelace.Symbolics.Tests/.../PiecewiseAssumptionTests/WithoutAssumptions_TheGuardIsStillUndecidable#0` | `PiecewiseAssumptionTests.cs:75`: no assumptions; `EvaluateToNum` throws `EvaluationException` | `lovelace-lang::round78_scoped_eval::without_assumptions_the_guard_is_still_undecidable` (`round78_scoped_eval.rs:111`) | Same refusal: both `evaluate` and `evaluate_scoped` with an empty scope answer the piecewise-guard message. |
| `Lovelace.Suite.Tests/.../StructuralEqualityTests/DifferenceInANestedField_IsNamedByPath#0` | `StructuralEqualityTests.cs:32`: `FirstDifference(Sample(2), Sample(3))` contains `solutions[1].multiplicity` | `lovelace-lang::round78_structural_equality::difference_in_a_nested_field_is_named_by_path` (`round78_structural_equality.rs:71`) | Same pair and path; asserts the exact diagnostic `$.solutions[1].multiplicity: Integer value differs`. |
| `Lovelace.Suite.Tests/.../StructuralEqualityTests/DifferentKinds_AreReportedByKind#0` | `StructuralEqualityTests.cs:64`: Integer 1 vs Text `"1"` contains `kind` | `lovelace-lang::round78_structural_equality::different_kinds_are_reported_by_kind` (`round78_structural_equality.rs:83`) | Same pair; asserts `$: kind Integer != Text`. |
| `Lovelace.Suite.Tests/.../StructuralEqualityTests/DifferentShape_IsReportedAsShape#0` | `StructuralEqualityTests.cs:40`: Vector `[1,2]` vs `[1]` contains `shape` | `lovelace-lang::round78_structural_equality::different_shape_is_reported_as_shape` (`round78_structural_equality.rs:95`) | Same pair; asserts `$: shape [2] != [1]`. |
| `Lovelace.Suite.Tests/.../StructuralEqualityTests/SymbolicEquality_IsByCanonicalForm_NotPrettyText#0` | `StructuralEqualityTests.cs:50`: `FirstDifference(left, left)` is null (the `ValuesEqual` assertion is tautological) | `lovelace-lang::round78_structural_equality::symbolic_equality_is_by_canonical_form_not_pretty_text` (`round78_structural_equality.rs:105`) | Same self-difference null; strengthens the intended fact with `x+x` equal to `2*x` canonically and a rational `-1/2` leaf unequal to a quotient with the same display. |

The map reasons record the block lift (`numeric::evaluate_scoped` / `ScopedEvaluation`; and
`structural_equality::StructuralEquality`) and the notes cite the C# line, the Rust test line and
the round-78 report section (`scoped-eval.md` / `structural-equality.md` section 4 / section 5).

## 4. Second regeneration — 8/8 resolve

After the map write (`parity/manifest-map.json` sha256
`7bfa168f76ba5a67cad0f67b9f20ae420b8948287a8c685e921c640f88144bed` ->
`69770df3f87d6c146c458a9d722acfde98cab6124f8c29a7756bee67d47004d3`; exactly the 8 keys changed and
no other key was touched, verified by comparing the parsed objects), `node parity/gen-manifest.mjs`,
`parity/gate-manifest.ps1` and `node parity/tools/coverage-report.mjs` were run again:

```
{
  "rows": 6209,
  "dispositions": {
    "excluded": 118,
    "ported": 5971,
    "deferred": 120
  },
  "coveragePercent": 96.17,
  "rustTests": 5248
}
```

```
"dispositions": { "ported": 5971, "consolidated": 0, "excluded": 118, "deferred": 120 },
"coveragePercent": 96.17,
"claimedRustTests": 5971,
"failures": []
PASS (structural): 6209 test cases dispositioned; 5971 claim Rust coverage (96.17%).
```

```
coverage.md written: 5971/6209 ported (96.17%)
```

Targeted check over the generated manifest and the measured inventory:

```
RESOLVED 8/8; suite exit 0; unresolved: none
```

Per project: Lovelace.Suite.Tests 900 -> 904 ported (96.88 % -> 97.31 %);
Lovelace.Symbolics.Tests 1,154 -> 1,158 (94.13 % -> 94.45 %).

**Final coverage line: 5,971/6,209 ported (96.17 %), 118 excluded, 120 deferred.**

## 5. Scope

Changed by this round:

* `parity/manifest-map.json` — 8 entries flipped `deferred` -> `ported` with their round-78 Rust
  test ids; no other key changed.
* `parity/manifest.toml`, `parity/rust-tests.json`, `parity/coverage.md` — regenerated.
* `docs/goal-parity-contract/round-79/round78-porting.md` — this report.

No `.rs`, `.cs`, fixture, corpus or other document was edited; the round-78 Rust files and test
modules remain the two lanes' uncommitted work, used here only as the measured inventory. The
`kernel_cancel_round9` and `lovelace-desktop` files were not touched.
