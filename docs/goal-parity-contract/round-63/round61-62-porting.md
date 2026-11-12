# Round 63 — Round 61–62 porting: six mappings and a truthful baseline

**Status: complete.** The six census rows unblocked by the round-61 DSP family and the round-62
.NET Random corpus are now `ported` and resolve in the measured inventory; the four remaining
`PropertyTests` rows stay `deferred` for the production blockers round 62 named. Regeneration came
first and cleared the inherited stale-inventory failure. This is a mapping round only:
`parity/manifest-map.json` gained six dispositions and four corrected deferral reasons, and
`parity/manifest.toml`, `parity/rust-tests.json` and `parity/coverage.md` were regenerated from
the census and the curated map. No Rust source, `.cs` file, fixture, corpus file or other document
was edited.

Measured 2026-11-11 (America/Sao_Paulo) on the working tree at `95de16a` plus the uncommitted
round-61/62 work (`rust/lovelace-lang/src/dsp_family_round61.rs`,
`rust/lovelace-corpus/src/{dotnet_random,property_corpus,property_assertions}.rs` and the edits
they touch). The tree was quiescent during the measurement; this lane was its only writer.

## 1. Regeneration first: the stale inventory

The pre-regeneration manifest gate ran against the tree exactly as inherited. It exited 1 with one
failure: the inventory predated the tests the two lanes had added. The committed manifest was the
round-60 baseline, so the failure isolated the stale inventory rather than hiding a structural
problem.

```json
{
  "censusCases": 6209,
  "manifestRows": 6209,
  "dispositions": {
    "ported": 5886,
    "consolidated": 0,
    "excluded": 118,
    "deferred": 205
  },
  "coveragePercent": 94.8,
  "claimedRustTests": 5886,
  "projectsInCensus": 15,
  "failures": [
    "rust-tests.json is stale: the Rust sources changed since the inventory was measured (run parity/gen-rust-tests.ps1)"
  ]
}
FAIL: 1 issue(s)
```

| Step | Command | Observed | Exit |
|---|---|---|---:|
| Pre-regeneration manifest gate | `node parity/tools/gate-manifest.mjs` | `coveragePercent` 94.8, ported 5,886, deferred 205, excluded 118; one failure: `rust-tests.json is stale …`; `FAIL: 1 issue(s)` | 1 |
| Rust inventory + suite | `powershell -File parity/gen-rust-tests.ps1` | `crates=11 tests=5078 suiteExit=0 passed=5074 failed=0` | 0 |
| Manifest generation (pre-mapping) | `node parity/gen-manifest.mjs` | `"rows": 6209`, `"ported": 5886`, `"deferred": 205`, `"excluded": 118`, `"coveragePercent": 94.8`, `"rustTests": 5078` | 0 |
| Manifest gate (pre-mapping) | `node parity/tools/gate-manifest.mjs` | `"claimedRustTests": 5886`, `"failures": []`; `PASS (structural): 6209 test cases dispositioned; 5886 claim Rust coverage (94.8%).` | 0 |

The regenerated inventory lists **5,078 tests**; the suite ran **5,074 passed, 0 failed**, so the
difference is exactly the **four `#[ignore]` blocker probes** in `lovelace-corpus` (the four
round-62 rows that remain deferred below). Its `sourcesSha256` is
`297fa3dd12bfba6f59133d0857320a40db0854c9cbfdf7cc095be91392f9fc19`, which matches the current
Rust sources. The pre-mapping gate passed with an empty failure list, so no other problem was hidden
behind the staleness.

## 2. The six mappings

The map was re-read immediately before the write. Every target was still `deferred` with no
`rust` value; only the fields below changed.

### 2.1 DSP family (3 rows)

Rows 1, 2, 6 and 7 of `docs/goal-parity-contract/round-61/dsp-family.md` were already `ported` to
`lovelace-lang::dispatcher_guard_round51::filter_signal_and_impulse_forms_match_the_reference_arithmetic`;
that test is still in the measured inventory and those four mappings still resolve. They were left
untouched. The three rows that were still deferred are:

| # | Census id | Rust test (`rust` field) | Evidence |
|---:|---|---|---|
| 1 | `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.FftTests/Fft_GivenEmpty_ReturnsEmpty#0` | `lovelace-lang::dsp_family_round61::fft_of_an_empty_signal_is_the_empty_transform` | exit 0 and `[] (Vector)`; the round-45 failure was exit 1 / DomainError `dft needs a non-empty vector.` (round-61 §2 row 3) |
| 2 | `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.FftTests/Fft_GivenNonPowerOfTwo_ThrowsArgumentException#0` | `lovelace-lang::dsp_family_round61::fft_of_a_non_power_of_two_length_is_a_typed_refusal` | exit 1, `InvalidArgument`/`TypeMismatch`, position 0, sentence `FFT length must be a power of two, but got 3. (Parameter 'x')` (round-61 §2 row 4) |
| 3 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.DspPluginTests/ComplexArithmetic_GivenComplex_ThrowsActionableError#0` | `lovelace-lang::dsp_family_round61::complex_arithmetic_refusal_names_the_bridge_builtins` | exit 1, `InvalidOperation`/`DomainError`, identical sentence including `use re()/im()/conj()/abs() to bridge back to Real.` (round-61 §2 row 5) |

### 2.2 .NET Random corpus (3 rows)

`docs/goal-parity-contract/round-62/dotnet-random.md` §4 and §5 mark exactly these three rows
UNBLOCKED and name the green contract tests:

| # | Census id | Rust test (`rust` field) | Contract |
|---:|---|---|---|
| 4 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PropertyTests/Simplify_PreservesValues_UnderRealAssumption#0` | `lovelace-corpus::property_assertions::simplify_preserves_values_under_real_assumption` | seed 20260910, 150 expressions × 5 points; `eval(simplify(f)) == eval(f)` under `x in Real`, exact `Num` comparison |
| 5 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PropertyTests/Solve_QuadraticRoots_SatisfyPolynomial#0` | `lovelace-corpus::property_assertions::solve_quadratic_roots_satisfy_polynomial` | seed 20260913, 40 triples; `Solve` is Exact, exactly two roots, substitution+expand is rational zero |
| 6 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PropertyTests/Optimize_PreservesValues#0` | `lovelace-corpus::property_assertions::optimize_preserves_values` | seed 20260914, 40 polynomials × 3 points; `eval(optimize(f)) == eval(f)` exactly |

All seven corpora are reproduced draw-for-draw; the transcript hashes in round-62 §3 are the
evidence that the Rust tests consume the same seeded corpus as the C# rows.

## 3. Resolution of the six ids

The measured inventory (11 crates, 5,078 tests, suite exit 0) contains every target, and the
generated manifest carries each one as a `ported` row with the same `rust` value. A per-mapping
check read the inventory set, the generated manifest row and the map entry:

```
OK lovelace-lang::dsp_family_round61::fft_of_an_empty_signal_is_the_empty_transform resolves=true manifestRow=true mapRow=true
OK lovelace-lang::dsp_family_round61::fft_of_a_non_power_of_two_length_is_a_typed_refusal resolves=true manifestRow=true mapRow=true
OK lovelace-lang::dsp_family_round61::complex_arithmetic_refusal_names_the_bridge_builtins resolves=true manifestRow=true mapRow=true
OK lovelace-corpus::property_assertions::simplify_preserves_values_under_real_assumption resolves=true manifestRow=true mapRow=true
OK lovelace-corpus::property_assertions::solve_quadratic_roots_satisfy_polynomial resolves=true manifestRow=true mapRow=true
OK lovelace-corpus::property_assertions::optimize_preserves_values resolves=true manifestRow=true mapRow=true
```

**No id was unresolved and no test name was guessed.** Every name the two round reports supplied is
present in the measured inventory. The four already-ported DSP rows also still resolve to
`lovelace-lang::dispatcher_guard_round51::filter_signal_and_impulse_forms_match_the_reference_arithmetic`.

## 4. Blocked rows: still deferred

The four remaining `PropertyTests` rows keep `disposition = "deferred"` and no `rust` field. Their
round-45 reason (“no Rust API exists to assert the row's contract”) was stale: the corpus and
`Context::canonical_parse` now exist. It was replaced with the measured round-62 blocker, so the
deferral is truthful. The blocker probes compile and reproduce the failures under
`cargo test -p lovelace-corpus property_assertions -- --ignored`.

| # | Census id | Measured blocker | Required production change |
|---:|---|---|---|
| 1 | `.../PropertyTests/CanonicalText_RoundTripsAndReinterns#0` | No expression interning in Rust (`Context` interns Symbol/Function only, so `Assert.Same` has no Rust spelling); `canonical_parse(canonical()).canonical()` differs for 1/200 items (item 49, commutative factor order). | Decide an interning translation and make canonical text/parse idempotent for all 200 items (`lovelace-symbolics`). |
| 3 | `.../PropertyTests/Diff_MatchesNumericalDerivative#0` | `numeric::evaluate` has no Piecewise/Relation arm; the generated derivative contains `(pw ((ne ...) 0) (derivative))` and is refused (`lovelace-lang/src/numeric.rs:70-72`). | Piecewise/Relation/And/Or/Not arms matching `Evaluation.EvaluateToNum`/`EvaluateCondition`. |
| 4 | `.../PropertyTests/Integrate_DiffRoundTrips#0` | Integration closes five of six integrands; `x^2*sin(x^3)` stays unevaluated, so the C# closed-form round trip cannot be asserted (`builtins.rs:1463`, `integrate_basic` at `builtins.rs:6392`). | The u-substitution shape in production `integrate()`. |
| 7 | `.../PropertyTests/MathIR_EvaluatesLikeSymbolic#0` | At `x=1/3` on expression 2, `numeric::evaluate` and `IrEvaluator` differ by 1 in the 50th decimal place; the C# row asserts exact `NumOps.Compare(direct, lowered) == 0`. | Exact 50-place agreement between the numeric and IR evaluators (a tolerance would weaken the row). |

The round-62 report's suggested disposition — map rows 2, 5 and 6; keep 1, 3, 4 and 7 deferred — is
exactly what this round did.

## 5. Gate outputs after the mapping

| Gate | Command | Observed | Exit |
|---|---|---|---:|
| Manifest generation | `node parity/gen-manifest.mjs` | `"rows": 6209`, `"ported": 5892`, `"deferred": 199`, `"excluded": 118`, `"coveragePercent": 94.89`, `"rustTests": 5078` | 0 |
| Manifest gate | `node parity/tools/gate-manifest.mjs` | `"claimedRustTests": 5892`, `"failures": []`; `PASS (structural): 6209 test cases dispositioned; 5892 claim Rust coverage (94.89%).` | 0 |
| Coverage report | `node parity/tools/coverage-report.mjs` | `coverage.md written: 5892/6209 ported (94.89%)` | 0 |

The measured suite was not re-run after the mapping: no Rust source changed, so the regenerated
inventory's freshness hash still matches the tree and the gate re-verified it.

### Coverage movement

| Project | Before | After | Deferred |
|---|---|---|---|
| Lovelace.Dsp.Tests | 55/61 (90.16 %), 6 deferred | **57/61 (93.44 %)**, 4 deferred | −2 |
| Lovelace.Suite.Tests | 868/929 (93.43 %), 61 deferred | **869/929 (93.54 %)**, 60 deferred | −1 |
| Lovelace.Symbolics.Tests | 1,121/1,226 (91.44 %), 105 deferred | **1,124/1,226 (91.68 %)**, 102 deferred | −3 |
| **TOTAL** | **5,886/6,209 (94.80 %)** | **5,892/6,209 (94.89 %)** | **199** |

Exclusions are unchanged at 118; no row is `consolidated`. The three Random mappings land in
`Lovelace.Symbolics.Tests`; the empty/non-power-of-two FFT mappings land in `Lovelace.Dsp.Tests`;
the Complex-refusal row lands in `Lovelace.Suite.Tests`.

## 6. Map-write evidence and scope

* `parity/manifest-map.json` was re-read immediately before the write. Before: 3,626,221 bytes,
  6,209 keys, SHA-256 `5f9955bb6df55bac5178d8df9d1e5e76325061aab1d0381ea904cb57e45834bc` (the
  committed round-60 map). After: 3,628,694 bytes, 6,209 keys, SHA-256
  `e5d41adacd05451cb1afee781c3b64455cf2ff723add1f30b2f452ed17f8f659`.
* `git diff --numstat` is `32 26` for the map: ten entries changed and nothing else. Six entries
  (`deferred` → `ported`) each gained `rust` and a new reason/note; four deferred entries changed
  reason/note only. A per-key comparison against `HEAD` reports exactly those ten entries and
  exactly six disposition changes; all 6,209 census keys remain.
* The JSON parses, the map retains every key, and the generated manifest has one row per census
  case (6,209) with no duplicate.
* **Scope kept:** `parity/manifest-map.json` plus generated `parity/manifest.toml`,
  `parity/rust-tests.json`, `parity/coverage.md` and this report. No `rust/` source, `.cs` file,
  fixture, corpus file or other document was edited. The uncommitted round-61/62 Rust changes are
  exactly as inherited.
* **Residual note for the manifest lane:** round-61 §6 suggested re-pointing all seven DSP ids to
  the new `dsp_family_round61` tests. This round's objective scoped only the three unported ids,
  so rows 1, 2, 6 and 7 keep their round-51 mapping. That mapping still resolves; re-pointing it is
  a separate, optional cleanup.

## 7. Reproduction

```powershell
$repo = 'C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME = "$repo\.toolchain\rustup"
$env:CARGO_HOME  = "$repo\.toolchain\cargo"
$env:PATH = "$repo\.toolchain\cargo\bin;" + $env:PATH
$env:TMP = "$repo\rust\target\tmp"; $env:TEMP = $env:TMP
Set-Location $repo

node parity/tools/gate-manifest.mjs          # pre-regen: stale inventory, exit 1
powershell -File parity/gen-rust-tests.ps1   # crates=11 tests=5078 suiteExit=0 passed=5074 failed=0
node parity/gen-manifest.mjs
node parity/tools/gate-manifest.mjs
# map edit
node parity/gen-manifest.mjs
node parity/tools/gate-manifest.mjs
node parity/tools/coverage-report.mjs
```
