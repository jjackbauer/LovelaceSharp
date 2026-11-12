# Round 60 — the ScriptSource and rewrite-registry blocks: fourteen mappings and a truthful baseline

**Status: the 14 census rows the round-59 ScriptSource and rewrite-registry lanes unblocked are now
`ported`; all 14/14 resolve in the measured inventory, and the stale-inventory failure inherited
with the tree is cleared.** This is a mapping round only: `parity/manifest-map.json` gained
exactly fourteen dispositions, and `parity/manifest.toml`, `parity/rust-tests.json` and
`parity/coverage.md` were regenerated from the census and the curated map. No Rust source, `.cs`
file, fixture, corpus file or other document was edited.

Measured 2026-11-11 (America/Sao_Paulo) on the working tree at `e7f1bda` plus the two uncommitted
round-59 modules (`rust/lovelace-run/src/script_source.rs`, +15 tests;
`rust/lovelace-symbolics/src/rewrite_registry.rs`, +10 tests) and the four edited files they touch
in `rust/lovelace-run`. The tree was quiescent during the measurement and this lane was its only writer.

## 1. Regeneration first: resolving the staleness

The first step was the pre-regeneration manifest gate, against the tree exactly as inherited. It
exited 1 with a single failure: the Rust inventory predated the 25 tests the two round-59 lanes had
added (5,029 measured before, 5,054 in the tree). The manifest itself was the committed baseline,
so the failure isolated the stale inventory rather than hiding a structural problem.

| Step | Command | Observed | Exit |
|---|---|---|---:|
| Pre-regeneration manifest gate | `node parity/tools/gate-manifest.mjs` | `"coveragePercent": 94.57`, `"claimedRustTests": 5872`, `"failures": ["rust-tests.json is stale: the Rust sources changed since the inventory was measured (run parity/gen-rust-tests.ps1)"]`; `FAIL: 1 issue(s)`; dispositions ported 5,872 / deferred 219 / excluded 118 | 1 |
| Rust inventory + suite | `powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\parity\gen-rust-tests.ps1` | `crates=11 tests=5054 suiteExit=0 passed=5054 failed=0` | 0 |
| Manifest generation (pre-mapping) | `node parity/gen-manifest.mjs` | `"rows": 6209`, `"ported": 5872`, `"deferred": 219`, `"excluded": 118`, `"coveragePercent": 94.57`, `"rustTests": 5054` | 0 |
| Manifest gate (pre-mapping) | `node parity/tools/gate-manifest.mjs` | `"claimedRustTests": 5872`, `"failures": []`; `PASS (structural): 6209 test cases dispositioned; 5872 claim Rust coverage (94.57%).` | 0 |

The regenerated inventory measured `crates=11 tests=5054 suiteExit=0 passed=5054 failed=0` — the
5,029 committed baseline plus the 25 round-59 tests (15 in `lovelace-run`, 10 in
`lovelace-symbolics`) — and its `sourcesSha256` then matched the current Rust sources. The
pre-mapping gate failed closed only on the inventory hash and otherwise reproduced the committed
baseline exactly: 5,872/6,209 (94.57%), 219 deferred, 118 excluded, `"failures": []`.
**No other failure appeared, so no other change is hidden behind the staleness.**

## 2. The fourteen mappings

All 14 rows were `deferred` with no `rust` value. The manifest `rust` id is the measured
inventory id from the two round-59 reports.

### 2.1 ScriptSourceTests (7)

| # | Census id | C# source | Rust test (`rust` field) |
|---:|---|---|---|
| 1 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ScriptSourceTests/ToSemicolonStatements_GivenNewlineSeparatedStatements_JoinsWithSemicolons#0` | `ScriptSourceTests.cs:8` | `lovelace-run::script_source::tests::to_semicolon_statements_given_newline_separated_statements_joins_with_semicolons` |
| 2 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ScriptSourceTests/ToSemicolonStatements_GivenCrlf_NormalizesLineEndings#0` | `ScriptSourceTests.cs:16` | `lovelace-run::script_source::tests::to_semicolon_statements_given_crlf_normalizes_line_endings` |
| 3 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ScriptSourceTests/ToSemicolonStatements_GivenBlock_PreservesNewlinesInsideBraces#0` | `ScriptSourceTests.cs:24` | `lovelace-run::script_source::tests::to_semicolon_statements_given_block_preserves_newlines_inside_braces` |
| 4 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ScriptSourceTests/ToSemicolonStatements_GivenNewlineAfterBlock_EmitsSemicolon#0` | `ScriptSourceTests.cs:32` | `lovelace-run::script_source::tests::to_semicolon_statements_given_newline_after_block_emits_semicolon` |
| 5 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ScriptSourceTests/ToSemicolonStatements_GivenStringWithNewline_PreservesIt#0` | `ScriptSourceTests.cs:40` | `lovelace-run::script_source::tests::to_semicolon_statements_given_string_with_newline_preserves_it` |
| 6 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ScriptSourceTests/ToSemicolonStatements_GivenTrailingSemicolon_SuppressesBlankSeparator#0` | `ScriptSourceTests.cs:48` | `lovelace-run::script_source::tests::to_semicolon_statements_given_trailing_semicolon_suppresses_blank_separator` |
| 7 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ScriptSourceTests/ToSemicolonStatements_GivenLeadingBom_StripsIt#0` | `ScriptSourceTests.cs:56` | `lovelace-run::script_source::tests::to_semicolon_statements_given_leading_bom_strips_it` |

Each mirror asserts the C# `Assert.Equal` byte-for-byte on the same input literal. The seven
cover semicolon joining of newline-separated statements, CRLF (and lone CR) normalisation,
newlines preserved inside braces, a newline after a block becoming a semicolon, a newline
preserved inside a double-quoted string, trailing-semicolon separator suppression, and one leading
BOM stripped. See `docs/goal-parity-contract/round-59/script-source.md` sections 2 and 4.

### 2.2 RewriteProtocolTests (7)

| # | Census id | Rust test (`rust` field) | What the mirror asserts |
|---:|---|---|---|
| 8 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.RewriteProtocolTests/Budget_CountsAppliedRewrites_AndStopsFurtherGroups#0` | `lovelace-symbolics::rewrite_registry::tests::budget_counts_applied_rewrites_and_stops_further_groups` | `transform(x/x, MaxSteps: 0)` is `BudgetExceeded`, kind `rewrite_steps`, no steps, unchanged expression |
| 9 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.RewriteProtocolTests/DomainSpecificRule_DeclaresItsCondition#0` | `lovelace-symbolics::rewrite_registry::tests::domain_specific_rule_declares_its_condition` | `x` in `Real`; `sqrt(x^2)` -> `abs(x)`; one `pow.sqrt-square-real` step, DomainSpecific, non-empty conditions |
| 10 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.RewriteProtocolTests/EveryRule_HasAStableUniqueId_AndUniversalRulesAreUnconditional#0` | `lovelace-symbolics::rewrite_registry::tests::every_rule_has_a_stable_unique_id_and_universal_rules_are_unconditional` | nine distinct ids in registration order; Universal rules have no declared conditions and no condition builder |
| 11 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.RewriteProtocolTests/RuleEvaluationException_IsTheOnlyCaughtState#0` | `lovelace-symbolics::rewrite_registry::tests::rule_evaluation_exception_is_the_only_caught_state` | the declared-undecidable probe is caught; expression unchanged; matched attempt is `Unknown` with the probe message |
| 12 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.RewriteProtocolTests/ThrowingPrecondition_Propagates#0` | `lovelace-symbolics::rewrite_registry::tests::throwing_precondition_propagates` | a non-`RuleEvaluationException` defect propagates out of `apply`, never becomes `NotApplicable` |
| 13 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.RewriteProtocolTests/TraceDisabled_KeepsConditionsAndAllocatesNoSteps#0` | `lovelace-symbolics::rewrite_registry::tests::trace_disabled_keeps_conditions_and_allocates_no_steps` | `x/x` -> `1`; empty steps; non-empty `x != 0` conditions |
| 14 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.RewriteProtocolTests/UnknownApplicability_NeverFires#0` | `lovelace-symbolics::rewrite_registry::tests::unknown_applicability_never_fires` | a rule that answers `Unknown` leaves the node unchanged |

The C# source is `Lovelace.Symbolics.Tests/RewriteProtocolTests.cs`; the Rust module is
`rust/lovelace-symbolics/src/rewrite_registry.rs`. See
`docs/goal-parity-contract/round-59/rewrite-registry.md` section 4.

Two adjacent rows are **not** mapped here and remain `deferred`:
`RewriteProofBridgeTests/EveryObligation_IsStatedAndHonestAboutItsStatus#0` and
`.../EveryShippedRule_HasExactlyOneProofObligation#0`, because the proof-obligation bridge is still
absent. The eighth `ScriptSourceTests` method,
`EvaluateAsync_GivenMultiLineScript_PlotsOneOverXSquared`, was already `ported` before this
round and is untouched.

## 3. Resolution of the fourteen ids

The measured inventory (`parity/rust-tests.json`, 11 crates, 5,054 tests, suite exit 0) contains
all fourteen targets, and the generated manifest carries each one as a `ported` row with the same
`rust` value. A per-mapping check read the generated manifest row and the inventory set:

```
OK lovelace-run::script_source::tests::to_semicolon_statements_given_newline_separated_statements_joins_with_semicolons resolves=true manifestRow=true
OK lovelace-run::script_source::tests::to_semicolon_statements_given_crlf_normalizes_line_endings resolves=true manifestRow=true
OK lovelace-run::script_source::tests::to_semicolon_statements_given_block_preserves_newlines_inside_braces resolves=true manifestRow=true
OK lovelace-run::script_source::tests::to_semicolon_statements_given_newline_after_block_emits_semicolon resolves=true manifestRow=true
OK lovelace-run::script_source::tests::to_semicolon_statements_given_string_with_newline_preserves_it resolves=true manifestRow=true
OK lovelace-run::script_source::tests::to_semicolon_statements_given_trailing_semicolon_suppresses_blank_separator resolves=true manifestRow=true
OK lovelace-run::script_source::tests::to_semicolon_statements_given_leading_bom_strips_it resolves=true manifestRow=true
OK lovelace-symbolics::rewrite_registry::tests::budget_counts_applied_rewrites_and_stops_further_groups resolves=true manifestRow=true
OK lovelace-symbolics::rewrite_registry::tests::domain_specific_rule_declares_its_condition resolves=true manifestRow=true
OK lovelace-symbolics::rewrite_registry::tests::every_rule_has_a_stable_unique_id_and_universal_rules_are_unconditional resolves=true manifestRow=true
OK lovelace-symbolics::rewrite_registry::tests::rule_evaluation_exception_is_the_only_caught_state resolves=true manifestRow=true
OK lovelace-symbolics::rewrite_registry::tests::throwing_precondition_propagates resolves=true manifestRow=true
OK lovelace-symbolics::rewrite_registry::tests::trace_disabled_keeps_conditions_and_allocates_no_steps resolves=true manifestRow=true
OK lovelace-symbolics::rewrite_registry::tests::unknown_applicability_never_fires resolves=true manifestRow=true
changed=14 ALL_MAPPINGS_RESOLVE=true suiteExit=0 inventoryTests=5054
```

**No id was unresolved and no id was guessed.** Every test name the two round-59 reports supplied
is present in the measured inventory, so there is nothing to report as absent.

### Map-write evidence

The map was re-read immediately before the write. All fourteen entries were `deferred` with no
`rust` value and the exact round-45 reason; the update changed exactly those fourteen and nothing
else:

* `parity/manifest-map.json` before: 3,622,561 bytes, 6,209 keys, SHA-256
  `d4da103920b625f1839939b37b0cfb4e814b1e8ca17b9c5c3611d8c9d9c95f68` (the committed round-58 map).
* After: 3,626,221 bytes, 6,209 keys, SHA-256
  `5f9955bb6df55bac5178d8df9d1e5e76325061aab1d0381ea904cb57e45834bc`.
* `git diff --numstat parity/manifest-map.json` is `56 42`: fourteen entries × (4 added lines −
  3 removed lines), i.e. exactly the fourteen dispositions, with no key inserted or removed.
* A per-key comparison against `HEAD` reports exactly the same fourteen changed map entries (all
  `deferred` → `ported`), and only their `disposition`, `rust`, `reason` and `note` fields
  differ. Every other key is unchanged.
* A JSON parse of the written file succeeds; all 6,209 census keys are present.

## 4. Gate outputs after the mapping

The mapping feeds only the manifest, so the Rust inventory was not regenerated a second time (no
Rust source changed); its freshness hash still matches the tree and the gate re-verified it.

| Gate | Command | Observed | Exit |
|---|---|---|---:|
| Manifest generation | `node parity/gen-manifest.mjs` | `"rows": 6209`, `"ported": 5886`, `"deferred": 205`, `"excluded": 118`, `"coveragePercent": 94.8`, `"rustTests": 5054` | 0 |
| Manifest gate | `node parity/tools/gate-manifest.mjs` | `"claimedRustTests": 5886`, `"failures": []`; `PASS (structural): 6209 test cases dispositioned; 5886 claim Rust coverage (94.8%).` | 0 |
| Coverage report | `node parity/tools/coverage-report.mjs` | `coverage.md written: 5886/6209 ported (94.80%)` | 0 |

The final gate line is `PASS (structural): 6209 test cases dispositioned; 5886 claim Rust coverage
(94.8%).` with exit 0 and an empty failure list. `parity/coverage.md` now carries:

```
| Lovelace.Suite.Tests | 929 | 868 | 61 | 0 | 93.43 % | partial |
| Lovelace.Symbolics.Tests | 1226 | 1121 | 105 | 0 | 91.44 % | partial |
| **TOTAL** | **6209** | **5886** | **205** | **118** | **94.80 %** | |
```

## 5. Coverage and what remains

* Total coverage moved from **5,872/6,209 (94.57 %)** to **5,886/6,209 (94.80 %)**. Deferred moved
  from 219 to 205; exclusions are unchanged at 118; no row is `consolidated`.
* `Lovelace.Suite.Tests` moved from 861/929 (92.68 %, 68 deferred) to
  **868/929 (93.43 %, 61 deferred)**, 0 excluded.
* `Lovelace.Symbolics.Tests` moved from 1,114/1,226 (90.86 %, 112 deferred) to
  **1,121/1,226 (91.44 %, 105 deferred)**, 0 excluded.
* **Scope kept:** `parity/manifest-map.json` plus its generated files
  (`manifest.toml`, `rust-tests.json`, `coverage.md`) and this report. No `rust/` source,
  `.cs` file, fixture, corpus file or other document was edited. The uncommitted round-59 Rust
  modules and their tests are exactly as inherited.
* **Still outside this block:** the rewrite proof-obligation bridge
  (`RewriteProofBridgeTests`) remains unported and its two rows stay deferred; the new registry is
  not wired into the language engine; source identity is host API and is deliberately not promoted
  to the four-field diagnostic wire shape; safe mode is still the language simplifier's policy and
  is not duplicated in the registry. Those are the round-59 reports' own "what remains" items, not
  new gaps created by this mapping round.
