# Round 77 — round-76 lane porting map flip

**Status: complete.** The 8 census rows the round-76 lanes unblocked are mapped to their
same-name round-76 Rust tests, and the regenerated gate resolves 8/8. Coverage moved from
5,955/6,209 (95.91 %) to 5,963/6,209 (96.04 %); deferred 136 -> 128; excluded unchanged at 118.

Measured 2026-11-12 (America/Sao_Paulo) on the working tree. Scope: `parity/manifest-map.json`
plus the generated parity files and this report; no Rust, C#, fixture or other document was edited.

## 1. Pre-regeneration baseline — stale inventory

The checked-in inventory predated the round-76 Rust modules: `parity/rust-tests.json` recorded
`generatedAtUtc 2026-11-12T12:46:11Z`, `suite {exitCode 0, passed 5191, failed 0}`, 11 crates /
5,192 tests, and no `round76_*` test name. The manifest gate therefore failed on freshness alone:

```
"dispositions": { "ported": 5955, "consolidated": 0, "excluded": 118, "deferred": 136 },
"coveragePercent": 95.91,
"claimedRustTests": 5955,
"failures": ["rust-tests.json is stale: the Rust sources changed since the inventory was measured (run parity/gen-rust-tests.ps1)"]
FAIL: 1 issue(s)
```

**Pre-regen line: 5,955/6,209 ported (95.91 %), 118 excluded, 136 deferred, gate FAIL (stale
inventory).**

## 2. First regeneration — re-established baseline

Commands: `parity/gen-rust-tests.ps1`, `node parity/gen-manifest.mjs`,
`parity/gate-manifest.ps1` (the two scripts were started through
`powershell -NoProfile -ExecutionPolicy Bypass -File`; the host execution policy rejects unsigned
scripts invoked directly). Observed lines:

```
crates=11 tests=5220 suiteExit=0 passed=5219 failed=0
```

```
{
  "rows": 6209,
  "dispositions": { "excluded": 118, "ported": 5955, "deferred": 136 },
  "coveragePercent": 95.91,
  "rustTests": 5220
}
```

```
PASS (structural): 6209 test cases dispositioned; 5955 claim Rust coverage (95.91%).
```

The fresh inventory (`generatedAtUtc 2026-11-12T13:11:06Z`) lists all 13 round-76 tests: 8
`lovelace-lang::round76_capture_events::*` and 5 `lovelace-symbolics::round76_matrix::*`, with
suite exit 0. The measured workspace total is **5,219 passed / 0 failed / 1 ignored** (5,220 tested):
28 more than the stale 5,191-passed inventory. The two round-76 lanes contribute 13 of those and the
other 15 are tests in the concurrent worktree; the task context's earlier 5,206 measurement is
superseded by this fresh run. The mapping below rests on this green run.

After this first regeneration the baseline is truthful but unchanged in coverage:
**5,955/6,209 (95.91 %), 118 excluded, 136 deferred, gate PASS**.

## 3. The eight mappings

Each C# body was read at the census line; each mapped Rust test is the row's own same-name test,
not a supporting/neighbour test (the modules' refusal, ordering and supporting tests such as
`events_are_ordered_and_carry_their_payloads`, `removing_a_missing_variable_is_refused_without_an_event`,
`capture_state_records_variables_and_user_functions` and `rank_and_solve_refusal_cases` were not
mapped).

| Census id | C# assertion | Rust test | Why this is the row's test |
|---|---|---|---|
| `Lovelace.Suite.Tests/.../CaptureState_IsImmutable#0` | `SuiteEngineTests.cs:72`: empty snapshot, `SetVariable("x", 1)`, count unchanged and no `x` | `lovelace-lang::round76_capture_events::capture_state_is_immutable` | Asserts the same two facts on `Engine::capture_state` (lines 30-39). |
| `Lovelace.Suite.Tests/.../VariableChanged_GivenAssignment_RaisesEvent#0` | `SuiteEngineTests.cs:84`: subscribe, evaluate `x = 7`, names contain `x` | `lovelace-lang::round76_capture_events::variable_changed_given_assignment_raises_event` | Same subscription, script and `contains x` assertion (lines 45-62). |
| `Lovelace.Suite.Tests/.../FunctionDefined_GivenDefinition_RaisesEvent#0` | `SuiteEngineTests.cs:96`: subscribe, evaluate `func g() { 1 }`, name is `g` | `lovelace-lang::round76_capture_events::function_defined_given_definition_raises_event` | Same subscription, script and exact `g` assertion (lines 67-78). |
| `Lovelace.Suite.Tests/.../SetVariableAndRemoveVariable_Work#0` | `SuiteEngineTests.cs:109`: set `x = 5`, TryGet true/value 5, Remove true, absent | `lovelace-lang::round76_capture_events::set_variable_and_remove_variable_work` | Same set/try-get/remove lifecycle; asserts the value's display `"5"` (lines 83-95). |
| `Lovelace.Symbolics.Tests/.../Rank_GenericOverSymbols_And_Numeric#0` | `MatrixAdvanceTests.cs:16`: ranks 2, 1, 1, 3 on four matrices | `lovelace-symbolics::round76_matrix::rank_generic_over_symbols_and_numeric` | The same four matrices and the same four rank assertions (lines 30-63). |
| `Lovelace.Symbolics.Tests/.../InverseWithConditions_NonSingular_MatchesInverse_And_CarriesCondition#0` | `MatrixAdvanceTests.cs:40`: matrix non-null, note null, conditions non-empty, equal to Inverse | `lovelace-symbolics::round76_matrix::inverse_with_conditions_non_singular_matches_inverse_and_carries_condition` | Same facts, plus exactly the determinant NonZero condition (lines 71-120). |
| `Lovelace.Symbolics.Tests/.../SolveWithConditions_MatchesSolve_And_CarriesCondition#0` | `MatrixAdvanceTests.cs:67`: vector non-null, note null, conditions non-empty, entrywise equal to Solve | `lovelace-symbolics::round76_matrix::solve_with_conditions_matches_solve_and_carries_condition` | Same system and facts, plus the exact condition and the solution at (2, 3) (lines 129-179). |
| `Lovelace.Symbolics.Tests/.../Inverse_Invertible_BehaviorPreserved#0` | `MatrixAdvanceTests.cs:86`: Inverse non-null and entrywise equal to InverseWithConditions matrix | `lovelace-symbolics::round76_matrix::inverse_invertible_behavior_preserved` | Same equality, plus concrete entries at (2, 3) (lines 188-228). |

The full ids are the eight census ids listed in the round-76 reports
(`docs/goal-parity-contract/round-76/suite-capture.md` section 4 and
`docs/goal-parity-contract/round-76/matrix-rank-solve.md` section 4). Each map entry was replaced
deferred -> ported with the `rust` id above and a reason/note that records the C# line, the Rust
test location and the section citations; the stale "no Rust API exists" reason was removed.

## 4. Second regeneration — 8/8 resolve

After the map write (`parity/manifest-map.json` sha256 `a953fc41...` -> `7bfa168f...`, no other
key changed), `node parity/gen-manifest.mjs`, `parity/gate-manifest.ps1` and
`node parity/tools/coverage-report.mjs` were run again:

```
{
  "rows": 6209,
  "dispositions": { "excluded": 118, "ported": 5963, "deferred": 128 },
  "coveragePercent": 96.04,
  "rustTests": 5220
}
```

```
"dispositions": { "ported": 5963, "consolidated": 0, "excluded": 118, "deferred": 128 },
"coveragePercent": 96.04,
"claimedRustTests": 5963,
"failures": []
PASS (structural): 6209 test cases dispositioned; 5963 claim Rust coverage (96.04%).
```

```
coverage.md written: 5963/6209 ported (96.04%)
```

Targeted check over the generated manifest and the measured inventory:

```
RESOLVED 8/8; suite exit 0; unresolved: none
```

**Final coverage line: 5,963/6,209 ported (96.04 %), 118 excluded, 128 deferred.**

## 5. Scope

Changed by this round:

* `parity/manifest-map.json` — 8 entries flipped to `ported` with their round-76 Rust test ids.
* `parity/manifest.toml`, `parity/rust-tests.json`, `parity/coverage.md` — regenerated.
* `docs/goal-parity-contract/round-77/round76-porting.md` — this report.

No `.rs`, `.cs`, fixture, corpus or other document was edited; the round-76 Rust files and test
modules remain the two lanes' uncommitted work, used here only as the measured inventory.
