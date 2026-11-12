# Round 58 — the RationalFunctions.Cancel block: seven mappings and the baseline re-verification

**Status: the seven census rows the round-57 `RationalFunctions.Cancel` / `SymbolicMatrix` inverse
block unblocked are now `ported`; all 7/7 resolve in the measured inventory, and the stale-inventory
failure left by the concurrent lane at `db30f6d` is cleared.** This is a mapping round only:
`parity/manifest-map.json` gained exactly seven dispositions, and `parity/manifest.toml`,
`parity/rust-tests.json` and `parity/coverage.md` were regenerated from the census and the curated
map. No Rust source, `.cs` file, fixture, corpus file or other document was edited.

Measured 2026-11-11 (America/Sao_Paulo) on the working tree at `db30f6d` plus the uncommitted
round-57 symbolics module; workspace suite 5,029 tests green. The tree was quiescent during the
measurement and this lane was its only writer.

## 1. Regeneration first: resolving the staleness

The first regeneration step was the pre-regeneration manifest gate, against the tree exactly as inherited. It
exited 1 with a single failure — the Rust inventory was older than the 18 tests the round-57 block
added. The manifest itself was already the committed baseline, so the failure isolated the
staleness rather than hiding a structural problem.

| Step | Command | Observed | Exit |
|---|---|---|---:|
| Pre-regeneration manifest gate | `node parity/tools/gate-manifest.mjs` | `rust-tests.json is stale: the Rust sources changed since the inventory was measured (run parity/gen-rust-tests.ps1)`; `FAIL: 1 issue(s)`; dispositions ported 5,865 / deferred 226 / excluded 118, coverage 94.46 % | 1 |
| Rust inventory + suite | `powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\parity\gen-rust-tests.ps1` | `crates=11 tests=5029 suiteExit=0 passed=5029 failed=0` | 0 |
| Manifest generation | `node parity/gen-manifest.mjs` | `"rows": 6209`, `"ported": 5865`, `"deferred": 226`, `"excluded": 118`, `"coveragePercent": 94.46`, `"rustTests": 5029` | 0 |
| Manifest gate | `node parity/tools/gate-manifest.mjs` | `"claimedRustTests": 5865`, `"failures": []`; `PASS (structural): 6209 test cases dispositioned; 5865 claim Rust coverage (94.46%).` | 0 |

The pre-regeneration gate's single failure was exactly
`rust-tests.json is stale: the Rust sources changed since the inventory was measured`. The
regenerated inventory measured `crates=11 tests=5029 suiteExit=0 passed=5029 failed=0` — the
committed baseline plus the 18 round-57 tests (13 `rational_functions` + 5 `symbolic_matrix`) —
and its `sourcesSha256` then matched the current Rust sources. Re-running the manifest generation
and gate reproduced the committed baseline exactly: 5,865/6,209 (94.46 %), 226 deferred, 118
excluded, `"failures": []`. **No other failure appeared, so no other change is hidden behind the
staleness.**

## 2. The seven mappings

All seven rows are in `Lovelace.Symbolics.Tests` and all seven targets are the new tests in
`rust/lovelace-symbolics/src/rational_functions.rs` and `rust/lovelace-symbolics/src/symbolic_matrix.rs`.
The manifest `rust` id is the measured inventory id `lovelace-symbolics::<module>::<test>`.

| # | Census id | C# source | Rust test (`rust` field) |
|---:|---|---|---|
| 1 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.MetamorphicSetClosureTests/Cancel_ExpandsAnUnexpandedPowerOfASumBeforeReducing#0` | `MetamorphicSetClosureTests.cs:657` | `lovelace-symbolics::rational_functions::tests::cancel_expands_an_unexpanded_power_of_a_sum_before_reducing` |
| 2 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.MetamorphicSetClosureTests/Cancel_PoleExclusionIsCarriedAsANonZeroConditionByTheRewritePath#0` | `MetamorphicSetClosureTests.cs:499` | `lovelace-symbolics::rational_functions::tests::cancel_conditions_are_the_same_atoms_as_non_zero_condition` |
| 3 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.MetamorphicSetClosureTests/Cancel_PreservesValueOffPoles_AndThePoleIsExcluded#0` | `MetamorphicSetClosureTests.cs:433` | `lovelace-symbolics::rational_functions::tests::cancel_preserves_value_off_poles_and_the_pole_is_excluded` |
| 4 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.MetamorphicSetClosureTests/CancelWithConditions_BuildsTheSameConditionAtomsAsTheRewritePath#0` | `MetamorphicSetClosureTests.cs:606` | `lovelace-symbolics::rational_functions::tests::cancel_with_conditions_reports_the_removed_factor_as_a_non_zero_atom` |
| 5 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.MetamorphicSetClosureTests/Inverse_TimesOriginal_IsIdentitySymbolically_UnderDetNonZeroCondition#0` | `MetamorphicSetClosureTests.cs:209` | `lovelace-symbolics::symbolic_matrix::tests::inverse_times_original_is_identity_symbolically_under_det_non_zero_condition` |
| 6 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.MetamorphicSetClosureTests/InverseIdentity_HoldsOnlyWhereDetIsNonZero#0` | `MetamorphicSetClosureTests.cs:325` | `lovelace-symbolics::symbolic_matrix::tests::inverse_identity_holds_only_where_det_is_non_zero` |
| 7 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.MetamorphicSetClosureTests/InverseIdentityCheck_RejectsInversesThatPassTheNumericSpotCheck#0` | `MetamorphicSetClosureTests.cs:250` | `lovelace-symbolics::symbolic_matrix::tests::inverse_identity_check_rejects_inverses_that_pass_the_numeric_spot_check` |

What each mirror asserts (the same text is in the manifest note):

1. **Row 1** — `(x-1)^2/(x-1)` and its expanded spelling both cancel to `x - 1`; the parser
   boundary is unchanged: `Polynomial::from_expr` still refuses a power of a sum and accepts a
   power of a symbol.
2. **Row 2 (composite)** — `Simplify.Transform(x/x)` returns `1` with one
   `SymbolProperty(x, NonZero)` atom and a `rat.cancel-x-over-x` step classified
   `Conditional`; safe mode refuses the rewrite until the atom is assumed; `x/x` is undefined at
   0 while `1` is defined.
3. **Row 3** — the ten-case corpus reduces to the documented expressions; at eight rational sample
   points the original is defined and the values agree exactly; at every pole the original is
   undefined and the reduced form is defined iff the pole was removed.
4. **Row 4** — `x/x` is `Conditional`, `Changed`, expression `1`, one
   `SymbolProperty(x, NonZero)`; `(x-1)^2/(x-1)` carries one
   `ExpressionProperty(x - 1, NonZero)`; the irreducible case is `Exact`, empty, unchanged, and
   the bare form agrees.
5. **Row 5** — the five-matrix corpus (2x2 general, 2x2 `[[x,1],[y,x]]`, 3x3 tridiagonal, 3x3
   cyclic, 3x3 Vandermonde): one `ExpressionProperty(det, NonZero)` atom equal to `a.det()`, det
   polynomial-equal to the independent ground truth, and the cleared-denominator identity proven
   exactly.
6. **Row 6** — at `(x,y) = (2,4)` (det = 0) the inverse entries are undefined; at `(2,3)`
   (det = 1) the product entries evaluate exactly to `1, 0, 0, 1`.
7. **Row 7** — the clean inverse passes; the transposed adjugate, a sign-flipped cofactor and a
   `+1` numerator perturbation are each rejected with non-empty detail; a perturbation that
   vanishes at `(x,y) = (3,2)` passes the numeric spot check yet is rejected symbolically.

**Row 2 is a composite and is mapped to the new atom test.** Its rewrite-path half is covered by
the existing measured tests
`lovelace-lang::suite_parity_batch::dxsemanticclosuretests_transform_xoverx_carriesnonzerocondition`
(rewrite trace / rule id / classification) and
`lovelace-lang::suite_parity_batch::dxsemanticclosuretests_language_simplify_issafebydefault`
(safe-mode refusal / licensing). The single manifest `rust` field names the new
`cancel_conditions_are_the_same_atoms_as_non_zero_condition` test — the atom half — and the note
names both `lovelace-lang` tests, because one field cannot name three tests. This follows the
round-58 instruction to map the composite row to the atom test; the round-57 proposal had suggested
the `lovelace-lang` test as the single field.

The other six rows are exact one-to-one mirrors.

### Map-write evidence

The map was re-read immediately before the write. All seven entries were `deferred` with no
`rust` value; the update changed exactly those seven and nothing else:

* `parity/manifest-map.json` before: 3,617,562 bytes, 6,209 keys, SHA-256
  `0d41e2c474013e81535719be2607eac5eb971d734a86bee9981ed1239bc2c380`.
* After: 3,622,448 bytes, 6,209 keys, SHA-256
  `d4da103920b625f1839939b37b0cfb4e814b1e8ca17b9c5c3611d8c9d9c95f68`.
* A JSON parse/serialise round-trip of the untouched file was byte-identical before the write, so
  the formatting and all 6,202 untouched entries are preserved.
* `git diff --numstat parity/manifest-map.json` is `28 21`: seven entries × (4 added lines − 3
  removed lines), i.e. exactly the seven dispositions, with no key inserted or removed.
* A per-key comparison against `HEAD` reports exactly the same seven changed map entries (all
  `deferred` → `ported`) and exactly seven changed `manifest.toml` rows; every other row is
  unchanged.

## 3. Resolution of the seven ids

The measured inventory (`parity/rust-tests.json`, 11 crates, 5,029 tests, suite exit 0) contains
all seven targets. A per-mapping check read the generated manifest row and the inventory set:

```
OK lovelace-symbolics::rational_functions::tests::cancel_expands_an_unexpanded_power_of_a_sum_before_reducing resolves=true manifestRow=true
OK lovelace-symbolics::rational_functions::tests::cancel_conditions_are_the_same_atoms_as_non_zero_condition resolves=true manifestRow=true
OK lovelace-symbolics::rational_functions::tests::cancel_preserves_value_off_poles_and_the_pole_is_excluded resolves=true manifestRow=true
OK lovelace-symbolics::rational_functions::tests::cancel_with_conditions_reports_the_removed_factor_as_a_non_zero_atom resolves=true manifestRow=true
OK lovelace-symbolics::symbolic_matrix::tests::inverse_times_original_is_identity_symbolically_under_det_non_zero_condition resolves=true manifestRow=true
OK lovelace-symbolics::symbolic_matrix::tests::inverse_identity_holds_only_where_det_is_non_zero resolves=true manifestRow=true
OK lovelace-symbolics::symbolic_matrix::tests::inverse_identity_check_rejects_inverses_that_pass_the_numeric_spot_check resolves=true manifestRow=true
OK supporting lovelace-lang::suite_parity_batch::dxsemanticclosuretests_transform_xoverx_carriesnonzerocondition resolves=true
OK supporting lovelace-lang::suite_parity_batch::dxsemanticclosuretests_language_simplify_issafebydefault resolves=true
changed=7 ALL_MAPPINGS_RESOLVE=true suiteExit=0 inventoryTests=5029
```

**No id was unresolved and no id was guessed.** No test name was absent from the measured
inventory, so there is nothing to report as missing. The `lovelace-symbolics` crate now measures
128 tests, including the 18 round-57 additions; the manifest gate accepted every one of the 5,872
claimed ids against the same measured inventory.

## 4. Gate outputs after the mapping

The mapping feeds only the manifest, so the Rust inventory was not regenerated a second time (no
Rust source changed); its freshness hash still matches the tree.

| Gate | Command | Observed | Exit |
|---|---|---|---:|
| Manifest generation | `node parity/gen-manifest.mjs` | `"rows": 6209`, `"ported": 5872`, `"deferred": 219`, `"excluded": 118`, `"coveragePercent": 94.57`, `"rustTests": 5029` | 0 |
| Manifest gate | `node parity/tools/gate-manifest.mjs` | `"claimedRustTests": 5872`, `"failures": []`; `PASS (structural): 6209 test cases dispositioned; 5872 claim Rust coverage (94.57%).` | 0 |
| Coverage report | `node parity/tools/coverage-report.mjs` | `coverage.md written: 5872/6209 ported (94.57%)` | 0 |

The final gate line is `PASS (structural): 6209 test cases dispositioned; 5872 claim Rust coverage
(94.57%).` with exit 0 and an empty failure list. `parity/coverage.md` now carries:

```
| Lovelace.Symbolics.Tests | 1226 | 1114 | 112 | 0 | 90.86 % | partial |
| **TOTAL** | **6209** | **5872** | **219** | **118** | **94.57 %** | |
```

## 5. Coverage and what remains

* Total coverage moved from **5,865/6,209 (94.46 %)** to **5,872/6,209 (94.57 %)**. Deferred moved
  from 226 to 219; exclusions are unchanged at 118; no row is `consolidated`.
* `Lovelace.Symbolics.Tests` moved from 1,107/1,226 (90.29 %, 119 deferred) to
  **1,114/1,226 (90.86 %, 112 deferred)**, 0 excluded.
* **Scope kept:** `parity/manifest-map.json` plus its generated files
  (`manifest.toml`, `rust-tests.json`, `coverage.md`) and this report. No `rust/` source,
  `.cs` file, fixture, corpus file or other document was edited. The uncommitted round-57 Rust
  module and its tests are exactly as inherited.
* **Still outside this block:** `CancelBuiltin_ReducesAndRefusesToCancelWhenNoFactorIsCommon#0`
  is the separate measured `cancel` builtin row (the existing language-layer univariate path still
  returns `x + 1` for `(x^2+1)/(x-1)`), and it remains deferred. `SymbolicMatrix.SolveWithConditions`,
  `Rank`, the singular-inverse row, and the other matrix-adjacent rows remain outside the minimal
  inverse surface. `RationalFunctions.Apart` is not ported. The new library modules are not wired
  into the language engine; the corpus, fixtures and CLI are unchanged by this round.
