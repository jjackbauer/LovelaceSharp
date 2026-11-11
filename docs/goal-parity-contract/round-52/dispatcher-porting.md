# Round 52 — the eleven dispatcher-gap census rows are mapped

**Status: mapped, regenerated and gated.** The 11 census rows listed in
`docs/goal-parity-contract/round-51/dispatcher-gap.md` §6 are now
`disposition = "ported"` in `parity/manifest-map.json`, each resolving to a test in the
round-51 guard module. The manifest gate reports `failures: []`, and the coverage report
reads 5,833/6,209 ported (93.94 %). All 11 ids were matched verbatim against
`parity/census.json` before writing; exactly 11 map keys changed and no other row moved.

Measured 2026-11-11 (America/Sao_Paulo) against the round-51 tree
(`sourcesSha256 = b222e723227e6fdf0fd2238b7cbb243627ebb4b731b3a8921a2a8583f2a2df67`).

## 1. What was mapped

The round-51 report names 6 descriptor-example rows, 4 DSP filter rows and 1 CancelFull
row. All 11 ids are present verbatim in `parity/census.json`, and each was
`deferred` in the map before this round.

### Descriptor examples (6) → `lovelace-lang::dispatcher_guard_round51::the_four_gap_builtins_run_their_published_examples`

* `Lovelace.Suite.Tests/Lovelace.Suite.Tests.DescriptorExampleTests/Example_Executes#21`
* `Lovelace.Suite.Tests/Lovelace.Suite.Tests.DescriptorExampleTests/Example_Executes#22`
* `Lovelace.Suite.Tests/Lovelace.Suite.Tests.DescriptorExampleTests/Example_Executes#53`
* `Lovelace.Suite.Tests/Lovelace.Suite.Tests.DescriptorExampleTests/Example_Executes#86`
* `Lovelace.Suite.Tests/Lovelace.Suite.Tests.DescriptorExampleTests/Example_Executes#107`
* `Lovelace.Suite.Tests/Lovelace.Suite.Tests.DescriptorExampleTests/Example_Executes#108`

The Rust test asserts the exact typed results of the six published examples
(`cancel_full(x/x)`, `cancel_full((x^2 - 1)/(x - 1))`, `filter([1], [1], [1, 2, 3])`,
`ndims([[1, 2], [3, 4]])`, `scale([1, 2, 3], 2)`, `scale([1, 2, 3], 0.5)`). The C# rows
only require the example to execute after `HelpService.ExamplePreludeSource`
(`DescriptorExampleTests.cs:104`); the typed-string assertion is therefore a stronger
check of the same example.

### DSP filter (4) → `lovelace-lang::dispatcher_guard_round51::filter_signal_and_impulse_forms_match_the_reference_arithmetic`

* `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.DspTests/Filter_GivenFir_MatchesConvolution#0`
* `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.DspTests/Filter_GivenIir_MatchesImpulseResponseConvolution#0`
* `Lovelace.Suite.Tests/Lovelace.Suite.Tests.DspPluginTests/Filter_GivenLength_ReturnsImpulseResponse#0`
* `Lovelace.Suite.Tests/Lovelace.Suite.Tests.DspPluginTests/Filter_GivenSignal_MatchesConvolution#0`

The Rust test asserts the FIR, IIR and impulse-response forms at the exact values the
C# assertions use as their convolution prefixes: `[1, 3, 6, 6]`,
`[1, 1.5, 1.75, 1.875]` and `[1, 0.5, 0.25, 0.125]` (`DspTests.cs:140,156`;
`DspPluginTests.cs:117,145`).

### CancelFull (1) → `lovelace-lang::dispatcher_guard_round51::cancel_full_publishes_the_conditions_the_bare_value_cannot_carry`

* `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.MetamorphicSetClosureTests/CancelFull_ReportsTheConditionsTheBareValueCannotCarry#0`

The Rust test asserts the same `CancelResult` fields as the C# row
(`MetamorphicSetClosureTests.cs:556`): `Conditional`/`x/x`/`1`/`changed`/
`conditions = [x != 0]`, the unchanged `Exact` result for `(x^2+1)/(x-1)` with empty
conditions, and `(x-1)^2/(x-1)` → `x - 1` with `[x - 1 != 0]`.

The fourth test in the module,
`lovelace-lang::dispatcher_guard_round51::every_registered_builtin_dispatches_instead_of_answering_unknown_function`,
is the structural registry/dispatcher guard. It does not correspond to a single census
row and is not claimed by one; it is the test that would have caught the gap.

## 2. The measured inventory

The pre-existing `parity/rust-tests.json` was stale for the round-51 tree: it listed
4,967 tests and none from `dispatcher_guard_round51`. Regenerating it was therefore a
precondition for resolving the new rows, not a formality. The fresh inventory was
measured as:

```text
$ powershell -File parity/gen-rust-tests.ps1
crates=11 tests=4971 suiteExit=0 passed=4971 failed=0
```

It records `generatedAtUtc = 2026-11-11T23:37:44Z`,
`suite = {"exitCode": 0, "passed": 4971, "failed": 0}`, and 4 tests in
`lovelace-lang::dispatcher_guard_round51`, including all three candidate test ids
named in the round-51 report. **No reported test name was absent from the measured
inventory; no mapping was guessed.**

## 3. Regeneration and gate, as observed

| Step | Command | Observed line | Exit |
|---|---|---|---:|
| Inventory + suite | `powershell -File parity/gen-rust-tests.ps1` | `crates=11 tests=4971 suiteExit=0 passed=4971 failed=0` | 0 |
| Manifest | `node parity/gen-manifest.mjs` | `{"rows": 6209, "dispositions": {"excluded": 118, "ported": 5833, "deferred": 258}, "coveragePercent": 93.94, "rustTests": 4971}` | 0 |
| Manifest gate | `node parity/tools/gate-manifest.mjs` | `"failures": []` and `PASS (structural): 6209 test cases dispositioned; 5833 claim Rust coverage (93.94%).` | 0 |
| Coverage report | `node parity/tools/coverage-report.mjs` | `coverage.md written: 5833/6209 ported (93.94%)` | 0 |

The manifest gate's full summary, verbatim:

```json
{
  "censusCases": 6209,
  "manifestRows": 6209,
  "dispositions": {
    "ported": 5833,
    "consolidated": 0,
    "excluded": 118,
    "deferred": 258
  },
  "coveragePercent": 93.94,
  "claimedRustTests": 5833,
  "projectsInCensus": 15,
  "failures": []
}
PASS (structural): 6209 test cases dispositioned; 5833 claim Rust coverage (93.94%).
```

The map diff was checked structurally against `HEAD`: exactly 11 keys changed, no key
was added, removed or reordered, and the remaining 6,198 entries are unchanged.
Map SHA-256 was `a8727432…252446` before and `781cb3ca…c41e4e` after the edit.

## 4. Resolution of the 11 new mappings

Each row below was re-read from the **generated** `parity/manifest.toml` (not from the
map), then its `rust` id was looked up in the measured `parity/rust-tests.json`:

```text
RESOLVED  Example_Executes#21   -> the_four_gap_builtins_run_their_published_examples
RESOLVED  Example_Executes#22   -> the_four_gap_builtins_run_their_published_examples
RESOLVED  Example_Executes#53   -> the_four_gap_builtins_run_their_published_examples
RESOLVED  Example_Executes#86   -> the_four_gap_builtins_run_their_published_examples
RESOLVED  Example_Executes#107  -> the_four_gap_builtins_run_their_published_examples
RESOLVED  Example_Executes#108  -> the_four_gap_builtins_run_their_published_examples
RESOLVED  Filter_GivenFir_MatchesConvolution#0                  -> filter_signal_and_impulse_forms_match_the_reference_arithmetic
RESOLVED  Filter_GivenIir_MatchesImpulseResponseConvolution#0   -> filter_signal_and_impulse_forms_match_the_reference_arithmetic
RESOLVED  Filter_GivenLength_ReturnsImpulseResponse#0           -> filter_signal_and_impulse_forms_match_the_reference_arithmetic
RESOLVED  Filter_GivenSignal_MatchesConvolution#0               -> filter_signal_and_impulse_forms_match_the_reference_arithmetic
RESOLVED  CancelFull_ReportsTheConditionsTheBareValueCannotCarry#0 -> cancel_full_publishes_the_conditions_the_bare_value_cannot_carry
resolved=11/11 unresolved=0
```

**11/11 resolve; 0 unresolved.** The gate independently accepts all 5,833 claimed tests
against the inventory and the suite exit recorded as 0.

## 5. Coverage movement

`parity/coverage.md` was regenerated from the gated manifest. The three affected
projects move exactly as the 11 rows predict (+8 Suite: 6 descriptor + 2 DspPlugin;
+2 Dsp.Tests; +1 Symbolics):

| C# project | Cases | Ported before | Ported after | Deferred after | Coverage after |
|---|---:|---:|---:|---:|---:|
| Lovelace.Dsp.Tests | 61 | 53 | 55 | 6 | 90.16 % |
| Lovelace.Suite.Tests | 929 | 844 | 852 | 77 | 91.71 % |
| Lovelace.Symbolics.Tests | 1226 | 1083 | 1084 | 142 | 88.42 % |
| **TOTAL** | **6209** | **5822** | **5833** | **258** | **93.94 %** |

The total rises 93.77 % → 93.94 %; deferred falls 269 → 258; the 118 user-approved
exclusions are untouched.

## 6. Scope and residual

* **Changed by hand:** `parity/manifest-map.json` (11 entries) and this report.
* **Regenerated:** `parity/rust-tests.json`, `parity/manifest.toml`,
  `parity/coverage.md`, by the commands in §3.
* **Not touched:** no Rust source, no `.cs`, no fixture, no corpus, no contract or other
  document. The round-51 tree (including the `dispatcher_guard_round51.rs` module and
  the `builtins.rs` arms) was already in the working tree and was not modified here.
* **Residual:** 258 rows remain `deferred` and 118 remain `excluded` of the 6,209
  census cases. The remaining deferred rows now include no row from the dispatcher-gap
  list.
