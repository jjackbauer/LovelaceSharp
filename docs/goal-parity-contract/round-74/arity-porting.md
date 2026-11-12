# Round 74 — arity taxonomy: five mappings, gate withheld

**Status: mapping complete; final gate deliberately not claimed.** The five arity census rows are
now `ported` in `parity/manifest-map.json`, each against the round-73 Rust test that asserts the
row's documented behaviour. On the disposition counts the manifest would move
**5,947 / 6,209 (95.78 %) -> 5,952 / 6,209 (95.86 %)**, deferred **144 -> 139**, exclusions
unchanged at **118**. The five mappings resolve **5/5** against the freshly measured inventory and
the focused test run is green. The workspace suite is red for nine tests in
`rust/lovelace-desktop/src/execution.rs`, an untracked module owned by a concurrent, out-of-scope
desktop goal session; the coordinator instructed this round to stop after the map edit and not to
regenerate the manifest or claim a gate result. The parent will regenerate and gate once that
tree is green.

Measured 2026-11-12 (America/Sao_Paulo) with the pinned toolchain.

## 1. Regeneration first — and the baseline it exposed

The pre-regeneration gate, run before any file was changed:

```text
{
  "censusCases": 6209,
  "manifestRows": 6209,
  "dispositions": { "ported": 5947, "consolidated": 0, "excluded": 118, "deferred": 144 },
  "coveragePercent": 95.78,
  "claimedRustTests": 5947,
  "projectsInCensus": 15,
  "failures": ["rust-tests.json is stale: the Rust sources changed since the inventory was measured (run parity/gen-rust-tests.ps1)"]
}

FAIL: 1 issue(s)   (exit 1)
```

So the inherited line is **5,947 / 6,209 (95.78 %), 144 deferred, 118 exclusions**, and it was not
a measured baseline. The inherited inventory had `generatedAtUtc` `2026-11-12T11:50:10Z`,
`sourcesSha256` `3041213a903f8f0a1e56bce4e5c03b69e26b1fdca8dd61c2ebc1db6d74c9a1bb`, **5,161
tests**, suite `exitCode 0 / passed 5160 / failed 0` (1 ignored) — and it contained **no**
`arity_taxonomy_round73` test at all. It predated the five arity tests; that is the stale part.

Regeneration steps actually run, in order:

| Step | Command | Observed | Exit |
|---|---|---|---:|
| Rust inventory + suite | `powershell -ExecutionPolicy Bypass -File parity/gen-rust-tests.ps1` | `crates=11 tests=5180 suiteExit=101 passed=148 failed=9` | 1 |
| Manifest generation (pre-mapping) | `node parity/gen-manifest.mjs` | `rows 6209`, `excluded 118`, `ported 5947`, `deferred 144`, `coveragePercent 95.78`, `rustTests 5180` | 0 |
| Manifest gate (pre-mapping) | `node parity/tools/gate-manifest.mjs` | not a pass: one `claims a Rust test but the measured suite exit is 101` failure per ported row | 1 |

The post-regeneration inventory has `generatedAtUtc` `2026-11-12T12:15:50Z`, `sourcesSha256`
`0d08670471adb2c5b6a27655f09d66162bb4177d249cf46a1090bad6b29891cb` and **5,180 tests**. The
increase is accounted for exactly: **5,161 + 5 arity tests + 14 new desktop tests = 5,180**.
`lovelace-lang` grew 1,549 -> 1,554 (the five arity tests are listed in the inventory) and
`lovelace-desktop` grew 67 -> 81. Removing the 14 desktop tests gives 5,166 = the round-73
baseline of **5,165 passed + 1 ignored**. That clean baseline is the round-73 record and the task
context; it is not a green workspace run observed in this session, because cargo fail-fast stopped
the suite at the desktop binary.

The 9 failures are listed in section 5. They are not arity tests and this round made no
`rust/` edit.

## 2. The five rows and the test that carries each

The map was re-read from disk immediately before the write. All five rows were `deferred` with
their round-45 reasons; none had a `rust` value.

| # | Census row | Chosen Rust test | Why this test carries the row |
|---|---|---|---|
| 1 | `Lovelace.Run.Tests/Lovelace.Run.Tests.ArityPropertyTests/EveryBuiltin_RefusesEveryCountOutsideItsDeclaredArity_WithTheOneDocumentedShape#0` | `lovelace-lang::arity_taxonomy_round73::every_builtin_refuses_every_count_outside_its_declared_arity` | The C# body (`ArityPropertyTests.cs:102`) drives every registered builtin with every count `0..Parameters.Count+2`, requiring the one documented `InvalidArgument`/`TypeMismatch` sentence outside `[MinArity, Parameters.Count]`, no arity-shaped refusal inside it, and `print` as the only fully-covering declaration. The Rust test (`arity_taxonomy_round73.rs:76`) runs the same loop over `registry::BUILTINS`, compares the exact expected sentence, and ends with the same `no_wrong_arity_exists == ["print"]` assertion. **This is the test that carries the 558-probe property row**: 408 probes outside the declared range and 150 inside, over 123 registry entries. The count was independently recounted from `rust/lovelace-lang/src/registry.rs:34` as `sum(parameter_count + 3)` = 558; the test itself enforces `probed >= 500`. |
| 2 | `...ArityValidationTests/TooFewArguments_IsATypedStructuralError_NeverAnInternalFailure#0` | `lovelace-lang::arity_taxonomy_round73::too_few_arguments_is_a_typed_structural_error` | The C# body (`ArityValidationTests.cs:26`) runs `x = symbol("x"); compile_full(x^2 + 1)` and requires exit 1, `ok:false`, not `InternalError`/`InternalInvariantFailure`, `recoverable:true`, and `compile_full(): expected 2 arguments; got 1.`. The Rust test (`:157`) asserts `InvalidArgument`/`TypeMismatch`, the identical sentence, position 17, and the declared signature (`compile_full`, parameters `[f, params]`, `expectedMin/Max 2`, `variadic false`, `actual 1`). |
| 3 | `...BuiltinArgumentShapeSweepTests/EveryWrongShapedArgument_IsNeverAnInternalFailure_AndEveryArgumentRefusalNamesTheBuiltinAndThePosition#0` | `lovelace-lang::arity_taxonomy_round73::every_wrong_shaped_argument_is_a_typed_refusal_naming_the_position` | The C# body (`BuiltinArgumentShapeSweepTests.cs:112`) drives the 115 positional builtins x 3 audit shapes = 345 probes: never an internal failure, an unrecoverable refusal or raw framework text; every `InvalidArgument` refusal must carry `TypeMismatch` and name the builtin and a 1-based position inside the declaration. The Rust test (`:192`) runs the same 3-shape sweep (345 probes) and additionally pins the two classifications the deferred family measured: `min(1, 1)` -> `min(): argument 1 must be an array or vector; got Natural.` (signature position 1, parameter `a`), and `compile_full(x, x)` -> the argument-2 list refusal (`InvalidOperation`/`DomainError`). |
| 4 | `...BuiltinArgumentShapeSweepTests/EveryWrongShapedArgument_OverAWiderShapeLattice_IsStillNeverAnInternalFailure#0` | `lovelace-lang::arity_taxonomy_round73::every_wrong_shaped_argument_over_the_wider_lattice_is_still_typed` | The C# body (`BuiltinArgumentShapeSweepTests.cs:228`) widens to eight shapes over the 115 positional builtins (920 probes) and unconditionally forbids internal failures, unrecoverable refusals, raw framework text and a non-1 exit. The Rust test (`:306`) runs the same 920-probe lattice and pins `min(inspect(1), inspect(1))` -> `min(): argument 1 must be an array or vector; got Record.` and `compile_full("x", "x")` -> `Expected a symbolic expression.` (`InvalidOperation`/`DomainError`). |
| 5 | `...CallSignatureDiagnosticsTests/EveryArityRefusalAgreesWithTheRegistryEntry#0` | `lovelace-lang::arity_taxonomy_round73::every_arity_refusal_agrees_with_the_registry_entry` | The C# body (`CallSignatureDiagnosticsTests.cs:78`) drives the five too-few calls `matmul([1, 2])`, `cross([1, 2, 3])`, `append([1, 2])`, `conv([1, 2])`, `filter([1], [1])` and requires each signature's `function`/`expectedMin`/`variadic` to equal the registry entry's declaration and `actual < min`. The Rust test (`:389`) drives the same five probes against `registry::find` and additionally checks `expectedMax` (`None` when variadic), the rendered message and the parameter list. |

Rejected as neighbouring, not mapped: `the_audits_five_reproductions_cross_as_the_documented_argument_error`
(only the five audit calls, not either sweep), `an_arity_refusal_publishes_the_signature` (a single
`dot(1)` probe, not the five-probe registry-agreement row), `declared_arity_travels_with_every_plugin_builtin`
(metadata propagation, a different property), and the positive controls `declared_argument_count_still_succeeds`,
`optional_trailing_parameters_stay_callable_in_the_shorter_form`, `adocumented_shorter_or_longer_form_stays_callable`
and `the_declared_form_stays_callable` (they assert calls still succeed, not the refusal contract).

## 3. Resolution: 5 / 5

A per-mapping check read the census, the curated map and the measured inventory and required: the
id occurs exactly once in the census; the map entry is `ported` with the chosen `rust`; the test
name exists in `parity/rust-tests.json`. The suite-exit clause is withheld with the gate (section 1);
the focused run is the per-test evidence.

```text
resolved 5 / 5
unresolved []
inventory: 5 arity tests present in lovelace-lang (1,554 tests)
focused:   5 passed; 0 failed; 0 ignored; 1549 filtered out   (exit 0)
suite:     exit=101 passed=148 failed=9 total=5180   (the 9 failures are desktop, section 5)
```

The focused run:

```text
cargo test -p lovelace-lang arity_taxonomy_round73
running 5 tests
test arity_taxonomy_round73::too_few_arguments_is_a_typed_structural_error ... ok
test arity_taxonomy_round73::every_arity_refusal_agrees_with_the_registry_entry ... ok
test arity_taxonomy_round73::every_wrong_shaped_argument_is_a_typed_refusal_naming_the_position ... ok
test arity_taxonomy_round73::every_builtin_refuses_every_count_outside_its_declared_arity ... ok
test arity_taxonomy_round73::every_wrong_shaped_argument_over_the_wider_lattice_is_still_typed ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 1549 filtered out; finished in 0.23s
```

**`parity/manifest.toml` was deliberately not regenerated after the map edit**, so it still
carries the five rows as `deferred` until the parent regenerates. The map is the curated source of
truth; running `node parity/gen-manifest.mjs` against a red suite would only produce a manifest the
gate rejects.

## 4. Coverage the map encodes (not gate-observed)

| Project | Before | After | Deferred |
|---|---|---|---|
| Lovelace.Run.Tests | 739 / 751 (98.40 %), 12 deferred | **744 / 751 (99.07 %)**, 7 deferred | -5 |
| **TOTAL** | **5,947 / 6,209 (95.78 %)**, 144 deferred | **5,952 / 6,209 (95.86 %)**, 139 deferred | **-5** |

Exclusions stay at 118. The expected final `parity/coverage.md` total line is:

```text
| **TOTAL** | **6209** | **5952** | **139** | **118** | **95.86 %** | |
```

This is arithmetic over the map's disposition counts, not a `coverage-report.mjs` output: coverage.md
was not regenerated because the suite is red. `5952 / 6209 = 95.8608...`, rounded to 95.86.

## 5. The red suite — nine failures outside this round's scope

The full failure list emitted by the captured `cargo test --workspace` rerun, all in
`lovelace-desktop`'s untracked `src/execution.rs` module:

```text
execution::tests::diagnostic_position_is_translated_back_to_the_full_source ... FAILED
execution::tests::real_engine_executes_sliced_statements_and_snapshots_variables ... FAILED
execution::tests::t2_submit_does_not_block_and_run_started_arrives_quickly ... FAILED
execution::tests::t3_events_are_sequenced_and_ordered_with_run_identity ... FAILED
execution::tests::t4_cancellation_stops_a_real_run_and_keeps_the_committed_snapshot ... FAILED
execution::tests::t5_duplicate_submit_is_busy_and_other_documents_run_concurrently ... FAILED
execution::tests::t6_panicking_engine_fails_one_run_and_the_host_stays_usable ... FAILED
execution::tests::t7_stale_finished_events_are_ignored_by_the_sink ... FAILED
execution::tests::t8_output_events_are_exactly_the_printed_lines_and_do_not_accumulate_across_runs ... FAILED
test result: FAILED. 72 passed; 9 failed; 0 ignored; 0 measured; 0 filtered out
error: test failed, to rerun pass `-p lovelace-desktop --bin lovelace-desktop`
```

`git status` shows `rust/lovelace-desktop/src/execution.rs` and
`rust/lovelace-desktop/src/statements.rs` untracked, with `main.rs` modified, all written at
09:12-09:13 local while this lane was measuring; the files belong to the concurrent desktop goal
session (`docs/goal-desktop-v4/`). They are not arity tests, they are not touched by this round,
and no `rust/` file was edited here. Cargo's fail-fast stopped the run after the desktop binary,
which is why the inventory records `passed 148` rather than the full workspace total; the
`lovelace-json`, `lovelace-nat`, `lovelace-int`, `lovelace-real`, `lovelace-symbolics`,
`lovelace-lang` and `lovelace-run` binaries were not run in that pass.

## 6. The map write

The map was re-read from disk immediately before the write, and written only after the five
entries were rebuilt.

* Before: 3,639,006 bytes, 6,209 keys, SHA-256
  `5c8d4e9cacde37bbfcbc1150a1436e1ced57b23edaa1b46e4413dc286ff45f99`.
* After: 3,642,468 bytes, 6,209 keys, SHA-256
  `0429838b735270240ff03969879724b92c9f4f10963fb3e3a7209ecb3ebe4284`.
* A parsed per-key comparison against the pre-write text reports exactly **5 changed keys**
  (`deferred` -> `ported` with a `rust` value), **0 added**, **0 removed**, and 0 changed keys
  outside the five targets. Dispositions went `{ ported: 5947, excluded: 118, deferred: 144 }` ->
  `{ ported: 5952, excluded: 118, deferred: 139 }`.

Each entry has the shape:

```json
"Lovelace.Run.Tests/Lovelace.Run.Tests.ArityPropertyTests/EveryBuiltin_RefusesEveryCountOutsideItsDeclaredArity_WithTheOneDocumentedShape#0": {
  "disposition": "ported",
  "rust": "lovelace-lang::arity_taxonomy_round73::every_builtin_refuses_every_count_outside_its_declared_arity",
  "reason": "Round 74 mapping: the row's whole-registry arity property is reproduced by the named Rust test, including its 558 probes and the print-only assertion (see note). Round 73 implementation: the registry-driven validator at builtins::call.",
  "note": "Round 74: ... See docs/goal-parity-contract/round-74/arity-porting.md section 2."
}
```

## 7. Scope and reproduction

Changed files: `parity/manifest-map.json` (five curated dispositions), `parity/rust-tests.json`
(regenerated, and it now records the red suite), and this report. `parity/manifest.toml` and
`parity/coverage.md` are unchanged. No `rust/` source, `.cs` file, fixture, corpus file or other
document was edited.

```powershell
$repo = 'C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME = "$repo\.toolchain\rustup"
$env:CARGO_HOME  = "$repo\.toolchain\cargo"
$env:PATH = "$repo\.toolchain\cargo\bin;" + $env:PATH
$env:TMP = "$repo\rust\target\tmp"; $env:TEMP = $env:TMP
Set-Location $repo

node parity/tools/gate-manifest.mjs                     # pre-regen: stale rust-tests.json, exit 1
powershell -ExecutionPolicy Bypass -File parity/gen-rust-tests.ps1  # crates=11 tests=5180 suiteExit=101 passed=148 failed=9
node parity/gen-manifest.mjs                           # pre-mapping: 5947 ported (95.78%)
# map edit: five deferred -> ported entries, re-read immediately before the write
Set-Location "$repo\rust"; cargo test -p lovelace-lang arity_taxonomy_round73  # 5 passed
```

## 8. Handoff

The five mappings are ready; nothing in this lane blocks them. The only blocker to counting them
is the red workspace suite, which is the desktop session's nine `execution::tests` failures. Once
`cargo test --workspace` is green, run `parity/gen-rust-tests.ps1`, then `parity/gen-manifest.mjs`,
then `parity/tools/gate-manifest.mjs`; the expected result is `claimedRustTests: 5952`,
`failures: []`, `PASS (structural): 6209 test cases dispositioned; 5952 claim Rust coverage
(95.86%).` Do not accept a gate run against the current inventory: its suite exit is 101 and the
gate rejects every ported row because of it.
