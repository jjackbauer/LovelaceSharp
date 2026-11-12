# Round 65 - Round 64 porting: three mappings and a truthful baseline

**Status: complete.** The three census rows unblocked by the round-64 canonical-order and
Diff/Integrate fixes are now `ported` and resolve in the measured inventory. Regeneration came
first and cleared the inherited stale-inventory failure. This is a mapping round only:
`parity/manifest-map.json` gained three dispositions, and `parity/manifest.toml`,
`parity/rust-tests.json` and `parity/coverage.md` were regenerated from the census and the curated
map. No Rust source, `.cs` file, fixture, corpus file or other document was edited.

Measured 2026-11-11 (America/Sao_Paulo) on the working tree at `6f2b853` plus the uncommitted
round-64 work (`rust/lovelace-corpus/src/{property_assertions,property_corpus}.rs`,
`rust/lovelace-lang/src/{builtins,numeric,printing_round16}.rs` and
`rust/lovelace-symbolics/src/{lib,printing_round16}.rs`). The tree was quiescent during the
measurement; this lane was its only writer.

## 1. Regeneration first: the stale inventory

The pre-regeneration manifest gate ran against the tree exactly as inherited. It exited 1 with one
failure, the inventory hash:

```json
{
  "censusCases": 6209,
  "manifestRows": 6209,
  "dispositions": {
    "ported": 5892,
    "consolidated": 0,
    "excluded": 118,
    "deferred": 199
  },
  "coveragePercent": 94.89,
  "claimedRustTests": 5892,
  "projectsInCensus": 15,
  "failures": [
    "rust-tests.json is stale: the Rust sources changed since the inventory was measured (run parity/gen-rust-tests.ps1)"
  ]
}
FAIL: 1 issue(s)
```

The inherited inventory was generated at `2026-11-12T02:09:40Z` from sources hash
`297fa3dd12bfba6f59133d0857320a40db0854c9cbfdf7cc095be91392f9fc19`, with 5,078 listed tests and a
suite of 5,074 passed / 0 failed (the four round-62 blocker probes were still `#[ignore]`). The
current sources hash to `54de681835d576e582946eb7e93b6fea3da2754cd936217feb3162020a98c456`.

| Step | Command | Observed | Exit |
|---|---|---|---:|
| Pre-regeneration manifest gate | `node parity/tools/gate-manifest.mjs` | `coveragePercent` 94.89, ported 5,892, deferred 199, excluded 118; one failure: `rust-tests.json is stale ...`; `FAIL: 1 issue(s)` | 1 |
| Rust inventory + suite | `powershell -File parity/gen-rust-tests.ps1` | `crates=11 tests=5084 suiteExit=0 passed=5083 failed=0` | 0 |
| Manifest generation (pre-mapping) | `node parity/gen-manifest.mjs` | `"rows": 6209`, `"ported": 5892`, `"deferred": 199`, `"excluded": 118`, `"coveragePercent": 94.89`, `"rustTests": 5084` | 0 |
| Manifest gate (pre-mapping) | `node parity/tools/gate-manifest.mjs` | `"claimedRustTests": 5892`, `"failures": []`; `PASS (structural): 6209 test cases dispositioned; 5892 claim Rust coverage (94.89%).` | 0 |
| Full suite (ignored count) | `cargo test --workspace` | 5,083 passed, 0 failed, 1 ignored, 21 test binaries, exit 0 | 0 |

The regenerated inventory lists **5,084 tests** (11 crates) and its freshness hash matches the
current Rust sources. `gen-rust-tests.ps1` records only the passed/failed fields, so the full suite
was run once more with the `test result:` lines aggregated; it reproduced **5,083 passed, 0 failed,
1 ignored** over 21 binaries. The delta from the inherited inventory is +9 passed and -3 ignored
(the three unignored row tests) plus six new tests the two round-64 lanes added. The pre-mapping
gate passed with an empty failure list, so no other problem was hidden behind the staleness.

## 2. The three mappings

The map was re-read immediately before the write. Every target was still `deferred` with no
`rust` value; only the disposition, reason and note changed, and each entry gained its `rust`
field.

| Row | Census id | Rust test (`rust` field) | Evidence |
|---|---|---|---|
| CanonicalText | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PropertyTests/CanonicalText_RoundTripsAndReinterns#0` | `lovelace-corpus::property_assertions::canonical_text_round_trips_and_reinterns` | Round 64 unignored the row test at `property_assertions.rs:84`. It asserts `canonical_parse(canonical(e)).canonical() == canonical(e)` for all 200 generated expressions; item 49 is fixed and the corpus FNV-1a 64 hash `6,083,718,521,819,931,755` matches the C# oracle. `Assert.Same` is translated to canonical-text idempotence because Rust `Expr` is a value (Context interns Symbol/Function ids only). |
| Diff | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PropertyTests/Diff_MatchesNumericalDerivative#0` | `lovelace-corpus::property_assertions::diff_matches_numerical_derivative` | Round 64 unignored the row test at `property_assertions.rs:131`; it runs the row's 60-expression central-difference assertion at the three points -3/2, 1/4 and 2/3. Production added the reference Piecewise/Relation arms and the `sign` evaluator; the oracle guard values are pinned by `numeric_piecewise_relation_arms_match_the_oracle` (`property_assertions.rs:339`). |
| Integrate | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PropertyTests/Integrate_DiffRoundTrips#0` | `lovelace-corpus::property_assertions::integrate_diff_round_trips` | Round 64 unignored the row test at `property_assertions.rs:194`. All six integrands now produce a closed form and `diff(integral) == integrand`; `integrate_by_substitution` closes `x^2*sin(x^3)` to `-1/3*cos(x^3)`, matching the C# library (the C# CLI leaves the parsed literal unevaluated; recorded as a reference defect). Closed forms are pinned by `integrate_by_substitution_matches_the_reference` (`property_assertions.rs:395`). |

The C# rows are `PropertyTests.cs:68`, `:115` and `:147` per the census. The two round-64 reports
name these rows and their fixes; `docs/goal-parity-contract/round-64/canonical-idempotence.md` section
5.1 supplies the CanonicalText mapping, and `docs/goal-parity-contract/round-64/diff-integrate.md`
sections 3 and 4 supply the Diff and Integrate evidence.

## 3. Resolution of the three ids

The measured inventory (11 crates, 5,084 tests, suite exit 0) contains every target, and the
generated manifest carries each one as a `ported` row with the same `rust` value. A per-mapping
check read the inventory set, the generated manifest row, the map entry and the census:

```
OK lovelace-corpus::property_assertions::canonical_text_round_trips_and_reinterns resolves=true manifestRow=true mapRow=true
OK lovelace-corpus::property_assertions::diff_matches_numerical_derivative resolves=true manifestRow=true mapRow=true
OK lovelace-corpus::property_assertions::integrate_diff_round_trips resolves=true manifestRow=true mapRow=true
```

**3/3 resolved. No id was unresolved and no test name was guessed.** Every name the round-64
reports supplied is present in the measured inventory.

## 4. Ignored count: now 1 (the MathIR row)

The workspace-wide suite is 5,083 passed / 0 failed / **1 ignored** across 21 binaries. The one
ignored test is:

```
property_assertions::mathir_evaluates_like_symbolic: test
```

`cargo test -p lovelace-corpus -- --list --ignored` lists exactly that test, and a grep for
`#[ignore` across every `rust/**/*.rs` file finds exactly one annotation, at
`rust/lovelace-corpus/src/property_assertions.rs:306` (the MathIR 1-ulp production gap). The three
formerly ignored row probes now run and pass. The corpus binary reports
`18 passed; 0 failed; 1 ignored`, so the ignored count moved from 4 (inherited: 5,078
listed tests, 5,074 passed) to 1.

## 5. Gate outputs after the mapping

| Gate | Command | Observed | Exit |
|---|---|---|---:|
| Manifest generation | `node parity/gen-manifest.mjs` | `"rows": 6209`, `"ported": 5895`, `"deferred": 196`, `"excluded": 118`, `"coveragePercent": 94.94`, `"rustTests": 5084` | 0 |
| Manifest gate | `node parity/tools/gate-manifest.mjs` | `"claimedRustTests": 5895`, `"failures": []`; `PASS (structural): 6209 test cases dispositioned; 5895 claim Rust coverage (94.94%).` | 0 |
| Coverage report | `node parity/tools/coverage-report.mjs` | `coverage.md written: 5895/6209 ported (94.94%)` | 0 |

The mapping changed no Rust source, so the measured suite and the inventory remain valid: the
inventory's freshness hash still matches the tree and the gate re-verified it.

### Coverage movement

| Project | Before | After | Deferred |
|---|---|---|---|
| Lovelace.Symbolics.Tests | 1,124/1,226 (91.68 %), 102 deferred | **1,127/1,226 (91.92 %)**, 99 deferred | -3 |
| **TOTAL** | **5,892/6,209 (94.89 %)**, 199 deferred | **5,895/6,209 (94.94 %)**, 196 deferred | **-3** |

Exclusions are unchanged at 118; no row is `consolidated`. All three mappings land in
`Lovelace.Symbolics.Tests`.

## 6. Map-write evidence and scope

* `parity/manifest-map.json` was re-read immediately before the write. Before: 3,628,694 bytes,
  6,209 keys, SHA-256 `e5d41adacd05451cb1afee781c3b64455cf2ff723add1f30b2f452ed17f8f659` (the
  committed round-63 map). After: 3,629,521 bytes, 6,209 keys, SHA-256
  `a9287419cb36be58ccec0ce0d6216c5af5d31e3fb233838e00835f08baab8155`.
* `git diff --numstat` is `15 15` for the map. A per-key comparison against `HEAD` reports exactly
  three changed entries, all `deferred` -> `ported`, each gaining a `rust` field; no other key
  changed and all 6,209 census keys remain. The generated `manifest.toml` diff is `12 9` with
  exactly the same three rows changed; `coverage.md` changes two lines (the Symbolics row and the
  total).
* The JSON parses, the map retains every key, and the generated manifest has one row per census
  case (6,209) with no duplicate.
* **Scope kept:** `parity/manifest-map.json` plus generated `parity/manifest.toml`,
  `parity/rust-tests.json`, `parity/coverage.md` and this report. No `rust/` source, `.cs` file,
  fixture, corpus file or other document was edited. The uncommitted round-64 Rust changes are
  exactly as inherited.

## 7. Reproduction

```powershell
$repo = 'C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME = "$repo\.toolchain\rustup"
$env:CARGO_HOME  = "$repo\.toolchain\cargo"
$env:PATH = "$repo\.toolchain\cargo\bin;" + $env:PATH
$env:TMP = "$repo\rust\target\tmp"; $env:TEMP = $env:TMP
Set-Location $repo

node parity/tools/gate-manifest.mjs          # pre-regen: stale inventory, exit 1
powershell -File parity/gen-rust-tests.ps1   # crates=11 tests=5084 suiteExit=0 passed=5083 failed=0
node parity/gen-manifest.mjs
node parity/tools/gate-manifest.mjs
# map edit: three deferred -> ported entries, re-read immediately before the write
node parity/gen-manifest.mjs
node parity/tools/gate-manifest.mjs
node parity/tools/coverage-report.mjs
Set-Location "$repo\rust"; cargo test --workspace   # 5083 passed, 1 ignored
```
