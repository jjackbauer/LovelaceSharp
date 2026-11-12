# Round 70 — `ParameterDomainOf`: one mapping and a truthful baseline

**Status: complete.** The single census row unblocked by the round-69 ParameterDomain work —
`Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DxStructuredResultsTests/ParameterDomainOf_MapsEveryDeclaredArm#0`
— is now `ported` and resolves **1/1** in a freshly measured inventory. Coverage moved
**5,937 / 6,209 (95.62 %)** -> **5,938 / 6,209 (95.64 %)**; deferred 154 -> 153; exclusions
unchanged at 118.

The inherited baseline was untruthful: its inventory predated the round-69 Rust work, so the gate
reported a stale `rust-tests.json`. The first action was to regenerate the inventory, then the
manifest; only then was the map edited. This is a mapping round: `parity/manifest-map.json` gained
exactly one disposition and `parity/manifest.toml`, `parity/rust-tests.json` and
`parity/coverage.md` were regenerated. No Rust source, `.cs` file, fixture, corpus file or other
document was edited.

Measured 2026-11-12 (America/Sao_Paulo) with the round-69 lane work uncommitted in the tree.

## 1. Regeneration first

Pre-regeneration gate, before any file was changed:

```text
rust-tests.json is stale: the Rust sources changed since the inventory was measured (run parity/gen-rust-tests.ps1)
FAIL: 1 issue(s)   (exit 1)
```

That stale gate still printed the inherited counts: ported 5,937, deferred 154, excluded 118,
coveragePercent 95.62, claimedRustTests 5,937, projectsInCensus 15, failures = the one stale line.
So 5,937 / 6,209 (95.62 %) is the pre-regen line, and it was not a measured baseline: the inventory
predated the ten round-69 tests.

| Step | Command | Observed | Exit |
|---|---|---|---:|
| Rust inventory + suite | `powershell -ExecutionPolicy Bypass -File parity/gen-rust-tests.ps1` | `crates=11 tests=5147 suiteExit=0 passed=5146 failed=0` | 0 |
| Manifest generation (pre-mapping) | `node parity/gen-manifest.mjs` | `rows 6209`, `ported 5937`, `deferred 154`, `excluded 118`, `coveragePercent 95.62`, `rustTests 5147` | 0 |
| Manifest gate (pre-mapping) | `node parity/tools/gate-manifest.mjs` | `"claimedRustTests": 5937`, `"failures": []`; `PASS (structural): 6209 test cases dispositioned; 5937 claim Rust coverage (95.62%).` | 0 |

The regenerated inventory lists 5,147 tests (5,146 passed, 0 failed, 1 ignored — the known
`property_assertions::mathir_evaluates_like_symbolic`) with `generatedAtUtc`
`2026-11-12T11:15:37Z` and `sourcesSha256`
`6d5bec3ed075569a6a7f8d235e34455c0040d36a48d7e2adab3794e05fa33599`, which the gate re-verifies
against the Rust sources. The +10 tests are exactly the round-69 work: 2 unit tests in
`rust/lovelace-symbolics/src/parameter_domain.rs`, 5 in
`rust/lovelace-symbolics/src/round69_parameter_domain.rs`, and 3 in
`rust/lovelace-lang/src/round69_parameter_domain.rs`.

## 2. Which Rust test carries the row

The C# row (`Lovelace.Symbolics.Tests/DxStructuredResultsTests.cs:562`) does three things in one
test: it pins a table whose keys must equal `Enum.GetValues<ParameterDomain>()`, it requires every
declared arm to map to `MathDomain.Integer`, and it requires `(ParameterDomain)99` to throw
`ArgumentOutOfRangeException`. I read every new test plus the downstream tests that mention the
field, and checked each against those three facts:

| Rust test | Whole enum | Declared arms -> integer | 99 refused | Carries the row? |
|---|---|---|---|---|
| `lovelace-symbolics::round69_parameter_domain::parameter_domain_of_maps_every_declared_arm` | yes | yes, both entry points | yes, exact identity + message | **chosen — all three** |
| `lovelace-symbolics::round69_parameter_domain::declared_arms_are_the_reference_enumeration` | yes | no | no | enum only |
| `lovelace-symbolics::round69_parameter_domain::integers_arm_maps_to_the_integer_domain` | no | one arm only | no | one arm, no refusal |
| `lovelace-symbolics::round69_parameter_domain::non_negative_integers_arm_maps_to_the_integer_domain` | no | one arm only | no | one arm, no refusal |
| `lovelace-symbolics::round69_parameter_domain::undeclared_values_are_refused` | no | no | yes, several ordinals | refusal only |
| `lovelace-symbolics::parameter_domain::tests::every_declared_arm_projects_to_the_integer_domain` | no | yes, both arms | no | mapping only |
| `lovelace-symbolics::parameter_domain::tests::undeclared_ordinal_is_refused_with_the_reference_message` | no | no | yes | refusal only |
| `lovelace-lang::round69_parameter_domain::modus_seam_maps_every_declared_arm_to_the_integer_domain_payload` | no | yes, through the Modus payload | no | seam mapping only |
| `lovelace-lang::round69_parameter_domain::modus_seam_refuses_an_undeclared_parameter_domain` | no | yes, through the Modus payload | yes, through the seam helper | seam-level both facts |
| `lovelace-lang::round69_parameter_domain::solve_full_family_parameter_domain_is_the_projected_integer_domain` | no | reachable arm only | no | downstream field only |
| `lovelace-lang::circular_family_round31` family-field tests | no | reachable arm only, as the string `"integer"` | no | downstream field only |

**Chosen: `lovelace-symbolics::round69_parameter_domain::parameter_domain_of_maps_every_declared_arm`.**
It is the only test that asserts the whole contract in one place against the native projection:
the pinned table is compared to `ParameterDomain::ALL` (the whole enum), both declared arms are
checked through both `parameter_domain_of` and `parameter_domain_of_ordinal` for `Dom::Integer`,
and `parameter_domain_of_ordinal(99)` must return `UndeclaredParameterDomain { ordinal: 99 }`
whose `Display` is the reference message
`No wire Domain is declared for this parameter domain. (Parameter 'domain') Actual value was 99.`
Its scope and name mirror the C# test exactly. `Dom` is the kernel lattice that the reference's
`MathDomain` crosses the wire as.

The other tests each drop at least one facet: the enumeration test asserts no mapping and no
refusal; the two per-arm tests pin one arm each and never call the refusal path;
`undeclared_values_are_refused` never asserts a declared arm maps to the integer domain; the two
`parameter_domain::tests` split mapping and refusal; and the language-side tests exercise the Modus
payload channel or a downstream `SolutionFamily` field. The closest runner-up,
`modus_seam_refuses_an_undeclared_parameter_domain`, does carry both facts through the seam, but it
asserts the payload envelope (`Payload::Domain("integer")`) rather than the native
`MathDomain`/`Dom::Integer` projection and the whole-enum table; the chosen test is strictly
stronger for the row's documented surface.

## 3. The one mapping

The map was re-read from disk immediately before the write. The target was still `deferred` with
the round-45 reason and no `rust` value. Only that entry changed:

```json
"Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DxStructuredResultsTests/ParameterDomainOf_MapsEveryDeclaredArm#0": {
  "disposition": "ported",
  "rust": "lovelace-symbolics::round69_parameter_domain::parameter_domain_of_maps_every_declared_arm",
  "reason": "Round 70 mapping: ...",
  "note": "Round 69: C# DxStructuredResultsTests.cs:562 ..."
}
```

* Before: 3,635,734 bytes, 6,209 keys, SHA-256
  `7b8a4b68a86c73254d54881e3b6815096ab62d543806da933cd0b023d898e99e`.
* After: 3,636,491 bytes, 6,209 keys, SHA-256
  `1deb62d924a4198f9f0cdeb88b24a3b118b4f184c7a54b23dface8e09f54db4f`.
* A parsed per-key comparison against a pre-write copy reports exactly 1 changed key
  (`deferred` -> `ported`), 0 added keys, 0 removed keys, 0 changed keys outside the target.
* `git diff --numstat` for the map is `4 3` (one disposition line changed, plus the new `rust`
  line and the replaced `reason`/`note`).

## 4. Resolution: 1/1

A per-mapping check read the census, the curated map, the generated manifest and the measured
inventory, and required all of: the id occurs exactly once in the census; the map entry is
`ported` with the chosen `rust`; the manifest row is `ported` with the same `rust`; the test name
exists in `parity/rust-tests.json`; and the measured suite exit is 0.

```text
resolved 1 / 1
unresolved []
changed keys 1
changed outside target 0
manifest rust = lovelace-symbolics::round69_parameter_domain::parameter_domain_of_maps_every_declared_arm
inventory contains test: yes (suiteExit 0)
focused run: 1 passed; 0 failed; 0 ignored (174 filtered out)
```

The focused run was
`cargo test -p lovelace-symbolics round69_parameter_domain::parameter_domain_of_maps_every_declared_arm -- --exact`.

## 5. Final coverage and gate

| Step | Command | Observed | Exit |
|---|---|---|---:|
| Manifest generation | `node parity/gen-manifest.mjs` | `rows 6209`, `ported 5938`, `deferred 153`, `excluded 118`, `coveragePercent 95.64`, `rustTests 5147` | 0 |
| Manifest gate | `node parity/tools/gate-manifest.mjs` | `"claimedRustTests": 5938`, `"failures": []`; `PASS (structural): 6209 test cases dispositioned; 5938 claim Rust coverage (95.64%).` | 0 |
| Coverage report | `node parity/tools/coverage-report.mjs` | `coverage.md written: 5938/6209 ported (95.64%)` | 0 |

```json
{
  "censusCases": 6209,
  "manifestRows": 6209,
  "dispositions": { "ported": 5938, "consolidated": 0, "excluded": 118, "deferred": 153 },
  "coveragePercent": 95.64,
  "claimedRustTests": 5938,
  "projectsInCensus": 15,
  "failures": []
}
```

| Project | Before | After | Deferred |
|---|---|---|---|
| Lovelace.Symbolics.Tests | 1,145 / 1,226 (93.39 %), 81 deferred | **1,146 / 1,226 (93.47 %)**, 80 deferred | -1 |
| **TOTAL** | **5,937 / 6,209 (95.62 %)**, 154 deferred | **5,938 / 6,209 (95.64 %)**, 153 deferred | **-1** |

**Unresolved ids: `[]`.** The row that was the last deferred member of the round-67 Modus family is
now ported; no id in this round remains unresolved.

## 6. Scope and reproduction

Changed files: `parity/manifest-map.json`, `parity/manifest.toml`, `parity/coverage.md`,
`parity/rust-tests.json`, and this report. No `rust/` source, `.cs` file, fixture, corpus file or
other document was edited. The inherited uncommitted round-69 Rust work is untouched.

```powershell
$repo = 'C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME = "$repo\.toolchain\rustup"
$env:CARGO_HOME  = "$repo\.toolchain\cargo"
$env:PATH = "$repo\.toolchain\cargo\bin;" + $env:PATH
$env:TMP = "$repo\rust\target\tmp"; $env:TEMP = $env:TMP
Set-Location $repo

node parity/tools/gate-manifest.mjs           # pre-regen: stale rust-tests.json, exit 1
powershell -File parity/gen-rust-tests.ps1   # crates=11 tests=5147 suiteExit=0 passed=5146 failed=0
node parity/gen-manifest.mjs                  # pre-mapping: 5937 ported (95.62%)
node parity/tools/gate-manifest.mjs           # pre-mapping: PASS
# map edit: the one deferred -> ported entry, re-read immediately before the write
node parity/gen-manifest.mjs                  # 5938 ported (95.64%)
node parity/tools/gate-manifest.mjs           # PASS
node parity/tools/coverage-report.mjs         # coverage.md written: 5938/6209 ported (95.64%)
Set-Location "$repo\rust"; cargo test -p lovelace-symbolics round69_parameter_domain::parameter_domain_of_maps_every_declared_arm -- --exact
```
