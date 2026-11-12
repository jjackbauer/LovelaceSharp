# Round 57 — the thread-scoped ExprContext block: eight mappings and their resolution

**Status: the eight census rows the round-56 context surface unblocked are now `ported`; every
gate is green and 8/8 resolve.** This round is a mapping round only: `parity/manifest-map.json`
gained exactly eight dispositions, and `parity/manifest.toml`, `parity/coverage.md` and
`parity/rust-tests.json` were regenerated from it. No Rust source, `.cs` file, fixture or corpus
file was edited; the round-56 module and its eleven tests already existed.

Measured 2026-11-11 (America/Sao_Paulo) on the current working tree (`ea609f7` plus the uncommitted
round-56 context module), workspace suite 5,011 tests green.

## 1. What this round changed

* **Edited:** `parity/manifest-map.json` — exactly the eight rows below moved from `deferred` to
  `ported`, each with its exact `lovelace-lang::context_round56::…` target and a note naming the
  C# assertion, the C# source line and the Rust test location. A structural key diff against
  `HEAD` shows 8 changed keys and no other entry changed.
* **Regenerated:** `parity/rust-tests.json` — 11 crates, 5,011 tests, suite exit 0; the inventory
  now contains the eleven `context_round56` tests (the eight mapped mirrors plus the three
  supporting tests). Before this regeneration the manifest gate refused the tree with
  `rust-tests.json is stale: the Rust sources changed since the inventory was measured`, because
  the round-56 additions had not been measured yet.
* **Regenerated:** `parity/manifest.toml` — 6,209 rows; ported 5,865, deferred 226, excluded 118;
  coverage 94.46 %.
* **Regenerated:** `parity/coverage.md` — total 5,865/6,209 (94.46 %);
  `Lovelace.Symbolics.Tests` 1,107/1,226 (90.29 %).
* **Untouched:** every `.cs` file, every `rust/` source, fixtures, corpus, runner envelope,
  `projects.toml` and `requirements.json`.

## 2. The eight mappings

All eight C# rows live in `Lovelace.Symbolics.Tests`; all eight Rust tests live in
`rust/lovelace-lang/src/context_round56.rs`. The manifest `rust` id is the inventory id
`lovelace-lang::context_round56::<test>`.

| # | Census id | C# source | Rust test | What the mirror asserts |
|---:|---|---|---|---|
| 1 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.AssumptionScopeIsolationTests/NestedScopes_RestoreTheEnclosingScopeOnDispose#0` | `AssumptionScopeIsolationTests.cs:49` | `context_round56::nested_scopes_restore_the_enclosing_scope_on_dispose` | Outer `x > 0` scope visible; inner `x < 0` scope visible and refutes the outer query; outer visible after the inner dispose; base set empty after both. |
| 2 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.AssumptionScopeIsolationTests/TwoConcurrentScopes_AreIsolated#0` | `AssumptionScopeIsolationTests.cs:15` | `context_round56::two_concurrent_scopes_are_isolated` | Two scopes open behind a `Barrier(2)`: positive flow answers `True`, negative flow answers `False`, base set empty afterwards. |
| 3 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.ConcurrencyTests/AssumptionScopes_RestoreOnDispose#0` | `ConcurrencyTests.cs:100` | `context_round56::assumption_scopes_restore_on_dispose` | Worker-thread `x in Real` scope simplifies `sqrt(x^2)` to `abs(x)`; drop restores the baseline `sqrt(x^2)`; atoms empty. |
| 4 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.ConcurrencyTests/ParallelCompilation_AgreesWithSerial#0` | `ConcurrencyTests.cs:87` | `context_round56::parallel_compilation_agrees_with_serial` | One polynomial lowered on 16 threads; every serialized program equals the serial one. |
| 5 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.ConcurrencyTests/ParallelFunctionCreation_NoDuplicateIds#0` | `ConcurrencyTests.cs:34` | `context_round56::parallel_function_creation_no_duplicate_ids` | 8 × 250 function names give 2,000 unique ids; every id resolves back to its name. |
| 6 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.ConcurrencyTests/ParallelParse_AgreesWithSerial#0` | `ConcurrencyTests.cs:51` | `context_round56::parallel_parse_agrees_with_serial` | 40 canonical texts parse to the same canonical results serially and in parallel; `x` is interned. |
| 7 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.ConcurrencyTests/ParallelSimplifyDiff_AgreeWithSerial#0` | `ConcurrencyTests.cs:66` | `context_round56::parallel_simplify_diff_agree_with_serial` | 40 `sin(x^2)^k` expressions; canonical diff and simplify renderings agree serially and in parallel. |
| 8 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.ConcurrencyTests/ParallelSymbolCreation_NoDuplicateIds#0` | `ConcurrencyTests.cs:15` | `context_round56::parallel_symbol_creation_no_duplicate_ids` | 8 × 250 symbol names give 2,000 unique ids; id/name round-trip; equality by name and repeated interning is stable. |

The three supporting tests (`a_scope_is_invisible_to_another_thread`,
`a_scope_restores_the_base_set_when_the_thread_unwinds`,
`the_base_setter_does_not_override_an_active_scope`) are not tied to a census row and were not
mapped. No other census id changed disposition.

## 3. Resolution of the eight ids

The measured inventory (`parity/rust-tests.json`, suite exit 0) contains exactly eleven
`context_round56::` tests. A per-mapping check read the manifest row and the inventory set:

    OK lovelace-lang::context_round56::nested_scopes_restore_the_enclosing_scope_on_dispose resolves=true manifestRow=true
    OK lovelace-lang::context_round56::two_concurrent_scopes_are_isolated resolves=true manifestRow=true
    OK lovelace-lang::context_round56::assumption_scopes_restore_on_dispose resolves=true manifestRow=true
    OK lovelace-lang::context_round56::parallel_compilation_agrees_with_serial resolves=true manifestRow=true
    OK lovelace-lang::context_round56::parallel_function_creation_no_duplicate_ids resolves=true manifestRow=true
    OK lovelace-lang::context_round56::parallel_parse_agrees_with_serial resolves=true manifestRow=true
    OK lovelace-lang::context_round56::parallel_simplify_diff_agree_with_serial resolves=true manifestRow=true
    OK lovelace-lang::context_round56::parallel_symbol_creation_no_duplicate_ids resolves=true manifestRow=true
    changed=8 ALL_MAPPINGS_RESOLVE=true suiteExit=0 inventoryTests=5011

The manifest gate's own freshness check passed against the same tree: the `sourcesSha256` in the
regenerated inventory matches the current Rust sources, so each claimed test exists in the measured
inventory and its suite ran green. **No id was unresolved and no id was guessed;** every Rust test
name in section 2 was already named by the round-56 contract and is present in the measured
inventory.

## 4. Gate outputs

Commands ran from the repository root. The script gate ran through Windows PowerShell 5.1 with
`-ExecutionPolicy Bypass` because the repository's local scripts carry the machine's
`RemoteSigned` policy; the script itself sets `RUSTUP_HOME`, `CARGO_HOME`, `PATH`, `TMP`,
`TEMP` and `TMPDIR`.

| Gate | Command | Observed | Exit |
|---|---|---|---:|
| Rust inventory + suite | `powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\parity\gen-rust-tests.ps1` | `crates=11 tests=5011 suiteExit=0 passed=5011 failed=0` | 0 |
| Manifest generation | `node parity/gen-manifest.mjs` | `"rows": 6209`, `"ported": 5865`, `"deferred": 226`, `"excluded": 118`, `"coveragePercent": 94.46`, `"rustTests": 5011` | 0 |
| Manifest gate | `node parity/tools/gate-manifest.mjs` | `"claimedRustTests": 5865`, `"failures": []`; `PASS (structural): 6209 test cases dispositioned; 5865 claim Rust coverage (94.46%).` | 0 |
| Coverage report | `node parity/tools/coverage-report.mjs` | `coverage.md written: 5865/6209 ported (94.46%)` | 0 |

The pre-regeneration manifest gate (same command) exited 1 with
`rust-tests.json is stale: the Rust sources changed since the inventory was measured` and no
other failure; that is the only failure the regeneration cleared, and the post-regeneration run
above has `"failures": []`.

## 5. Coverage and what remains

* Total coverage moved from **5,857/6,209 (94.33 %)** to **5,865/6,209 (94.46 %)**. Deferred moved
  from 234 to 226; exclusions are unchanged at 118.
* `Lovelace.Symbolics.Tests` moved from 1,099/1,226 (89.64 %, 127 deferred) to
  **1,107/1,226 (90.29 %, 119 deferred)**, 0 excluded.
* **No unresolved ids.** All 8 rows of this round resolve to exact tests (8/8), and no other
  census row was added, removed or re-dispositioned.
* The C# `Functions` registry, `Intern` hash-consing pool and `AsyncLocal` flow semantics across
  `await` remain outside the additive Rust module, as recorded in
  `docs/goal-parity-contract/round-56/expr-context.md` section 6; none of the eight rows observes
  them.
