# Round 82 — the evalf-precision family (4 rows)

**Status: closed.** The four deferred `evalf-precision` rows now match the pinned oracle on their own
scripts, full-envelope after the differential's normalisation (`revision`, `elapsed`,
`elapsedTime`, `timings` replaced at any depth). **8 new tests** (4 language, 4 runner) assert the
four rows plus the digit-cap boundary at, one below and one above the cap. **No production
expectation was relaxed and no existing test was edited**: every assertion that moved is an
assertion that *gained* a value the oracle publishes.

Measured 2026-11-12 (America/Sao_Paulo) against the pinned oracle
`Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll` and `rust/target/release/lovelace-run.exe`.

## 1. The family and its census ids

The classification (`round-75/probe-classification.md` §4, ranked table row 3) lists four
`evalf-precision` rows; `round-66/resweep2.md:94,125` calls the same four the "evalf digit cap"
family. They are one defect with four faces — the value's OWN stored digits versus the rendering's
display bound, and the cap notice that follows from it:

| # | Census row | Script | Field that differed |
|---:|---|---|---|
| 1 | `Lovelace.Run.Tests/Lovelace.Run.Tests.EvalfDigitCapHonestyTests/ARequestAtTheCap_IsNotMarked#0` | `evalf(pi, 1000)` | `$.result.structured.value` — **100** fractional digits vs the oracle's **1 000** |
| 2 | `Lovelace.Run.Tests/Lovelace.Run.Tests.StructuredRealPrecisionTests/EvalfCountAboveTheAmbientDisplayPrecision_CrossesInFull#0` | `evalf(pi, 1000)` | same field, plus the `_` variable's `structured.value` |
| 3 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.EvalfTruncatedConstantExactnessTests/ExactControls_AreUntouched#0` | `sin(0)` | `$.result.display` — **"sin(0)"** vs the oracle's **"0"** |
| 4 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.EvalfWorkingPrecisionHonoursRequestedDecimalsTests/ValuesBelowTheComputationCap_StayInexact#0` | `evalf(sin(pi(30)), 40)` | `$.result.structured.value` — **"0.0000000000000000000000000000005028841971"** vs the oracle's **"0"** |

## 2. Per-row probe, before and after

Both runners were driven with `--eval <script> --omit-functions`; the whole envelope was compared,
not just the named field. "Before" is the pinned pre-fix binary, "after" the fixed one; the oracle
column is the same script on the frozen C# runner.

| Row | Field | Before (Rust) | Oracle | After (Rust) |
|---:|---|---|---|---|
| 1 | `structured.value` length | 102 chars / **100** decimals | 1 002 chars / **1 000** decimals | **1 000** decimals, equal to the oracle byte-for-byte |
| 1 | `truncated`/`truncationReason`/`budget` | absent | absent | absent (nothing was dropped at the cap) |
| 1 | `result.display` | 100 decimals | 100 decimals | 100 decimals |
| 2 | `variables[_].structured.value` | 100 decimals | 1 000 | 1 000, equal to `result.structured.value` |
| 2 | value prefix / suffix | `3.1415926535897932384626433832795028841971693993751058209749` (the first 60 characters) | `3.` + pi's first 60 … ends `66111959092164201989` | identical to the oracle (digits 981-1000 present) |
| 3 | `result.kind` | Symbolic | Symbolic | Symbolic |
| 3 | `result.display` / `structured.pretty` | `sin(0)` | `0` | `0` |
| 3 | `structured.canonical` | `(fn sin (rat 0 1))` | `(rat 0 1)` | `(rat 0 1)` |
| 3 | `structured.domain` / `nodeCount` | `real` / 2 | `rational` / 1 | `rational` / 1 |
| 4 | `structured.value` | `0.0000000000000000000000000000005028841971` (40 decimals) | `0` | `0` |
| 4 | `structured.exact` / numerator | `false` / absent | `false` / absent | `false` / absent |

The other two expressions of row 4 (`evalf(1/(3*10^1000), 30)`, `evalf(1/(3*10^1001), 30)`) already
matched ("0", inexact, no rational form) and still do; they are asserted beside the fixed one.

The deferral ledger records the same before-state verbatim, so the "before" column is not a
re-derivation: row 1's reason is `$.result.structured.value: evalf(pi,1000) publishes 100 fractional
digits in Rust vs 1000 in the oracle` and row 4's is `$.result.display: evalf(sin(pi(30)),40) is
0.0000000000000000000000000000005028... in Rust vs 0 in the oracle`
(`parity/manifest-map.json:30652` and `:26049`). Both reasons now describe a closed defect; the
map is not edited here.

## 3. The cap boundary — at, one below and one above

The builtin's computation cap is **1 000 decimal places** (`EVALF_DECIMAL_PLACE_CAP`,
`SymbolicsPlugin.EvalfDecimalPlaceCap`). A request *at* the cap is honoured in full, so it must never
carry the clamp notice; the marker means "digits were DROPPED", not "the request was at/above 1000".

| Script | Before (Rust) | Oracle | After (Rust) |
|---|---|---|---|
| `evalf(pi, 999)` | 100 decimals, unmarked | 999 decimals, unmarked | 999 decimals, unmarked |
| `evalf(pi, 1000)` | 100 decimals, unmarked | 1 000 decimals, unmarked | 1 000 decimals, unmarked |
| `evalf(pi, 1001)` | 100 decimals, `truncated:true`/`digit-cap`/`1000` | 1 000 decimals, same notice | 1 000 decimals, same notice |
| `evalf(1/3, 999)` | 999 decimals, unmarked | 999, unmarked | unchanged (already matched) |
| `evalf(1/3, 1000)` | 1 000, unmarked | 1 000, unmarked | unchanged (already matched) |
| `evalf(1/3, 1001)` | 1 000, `digit-cap`/`1000` | same | unchanged (already matched) |
| `evalf(1/2, 5000)` | `0.5`, exact, unmarked | same | unchanged (control: an exact answer dropped nothing) |
| `setprecision(5000); evalf(sqrt(2), 5000)` | 1 000, `digit-cap`/`1000` | same | unchanged |
| `setprecision(5000); sqrt(2)` | 5 000, unmarked | same | unchanged (control: the engine's scope is not the builtin's cap) |
| `setprecision(999); evalf(pi, 1000)` | 100 | 999, unmarked | 999, unmarked |
| `setprecision(1000); evalf(pi, 1001)` | 100, marked | 1 000, marked | 1 000, marked |
| `setprecision(2000); evalf(pi, 1001)` | 100, marked | 1 000, marked | 1 000, marked |
| `setprecision(30); evalf(pi, 1000)` | 30 | 30 (the value's own digits, not padded) | 30 |

## 4. Root cause and fixes

1. **`pi`/`e` were computed at the DISPLAY precision.** The language materialised the constants with
   `Engine::display_precision()` (100 until `setprecision` moves it), so the VALUE itself only ever
   stored 100 digits and `evalf(pi, 1000)` could not serve the request from it. The reference
   computes `Real.Pi` at the COMPUTATION precision (1 000 by default) and bounds only the RENDERING
   at the display precision — which is why `pi` itself publishes 1 000 digits and displays 100.
   `Expr::Var("pi"|"e")` and the zero-argument `pi()`/`e()` builtin now use
   `Engine::precision_value()`. (`rust/lovelace-lang/src/lib.rs`, `builtins.rs`.)
2. **`evalf` returned a Real it could not improve untouched.** The Real arm short-circuited whenever
   the stored digit count fitted the request, so a value whose scale was the display's (or a
   zero carrying a computed scale) crossed at its own scale while the reference re-materialises
   every Real argument at `min(requested, cap)` places (`NumericAtRequestedPrecision` →
   `BoundDecimalPlaces`). The arm now always renders the value's own digits at that bound and reads
   them back as a decimal, which also normalises the stored scale (the `setprecision(50); evalf(pi,
   100)` case now publishes pi's 49 surviving digits, as the oracle does). (`builtins.rs`.)
3. **The elementary functions did not fold an inexact real constant.** The reference's
   `Evaluation.FoldFunction` has two arms: the exact special points (sin(0), cos(0), tan(0), exp(0),
   log(1), atan(0), asin(0), acos(1) — and the hyperbolic six), and an APPROXIMATE fold that
   evaluates the function on an inexact real constant at the system computation precision (30
   places until `setprecision`) and truncates there with `TruncateFrac`. Rust only had the
   inverse-hyperbolic special cases and never folded an approximation, which is why `sin(0)`
   crossed as `sin(0)` and `sin(pi(30))` crossed as a symbolic node that `evalf` then
   reconstructed at 40 places. The `sin|cos|tan|…|atanh` arm now folds both arms: exact constants
   through `fold_unary` (extended with the six hyperbolic rows), inexact real constants through
   `fold_real_constant` at `Engine::dsp_precision()`, leaving the node symbolic on a domain edge.
   (`builtins.rs`.)
4. **The `evalf` payload bound rescued significant digits.** `Num::into_value` used
   `TruncateFrac`, whose significant-digit rescue belongs to the COMPUTATION (a folded constant
   whose value sits below the ambient window keeps its digits). The PUBLISHED bound is
   `BoundDecimalPlaces`, a plain truncation that flushes a magnitude below the window to zero at
   the requested resolution — the difference between `evalf(sin(pi), 20)` = `0` (oracle) and
   `0.000…82148…`. `into_value`'s `Num::Real` arm now uses `Real::truncated_frac`.
   (`rust/lovelace-lang/src/numeric.rs`.)

## 5. Rows the same fix moves (measured, not claimed as census rows)

A 70-script probe matrix (the four rows, the boundary, the whole `EvalfDigitCapHonestyTests` /
`EvalfTruncatedConstantExactnessTests` control set, `setprecision` variants, and every fold shape)
compares full envelopes: **70 / 70 match** the oracle after normalisation. The fold and the payload
bound also close these previously-different rows of the same defect, each re-probed on both
runners after the fix and each an exact match:

| Script | Before (Rust) | Oracle = after |
|---|---|---|
| `sin(pi(30))` | Symbolic `sin(3.14159…279)` | Symbolic `0.000000000000000000000000000000` (30-place window zero) |
| `cos(pi(30))` | Symbolic `cos(3.14159…279)` | Symbolic `-1` (inexact) |
| `sin(pi)` | Symbolic `sin(3.14159…)` | 1 030-decimal `0.000…38095…` (truncation window) |
| `sin(pi(20))` / `sin(pi(31))` / `sin(pi(40))` / `sin(pi(100))` | Symbolic nodes | 30 / 62 / 70 / 130-decimal folds |
| `exp(pi(30)/10)` / `atan(pi(30)/4)` | Symbolic nodes | `1.36910777062484688483648708032` / `0.665773750028353863590517708512` |
| `evalf(sin(pi), 20)` | `0.000…82148…` (a nonzero 1e-101 tail) | `0` |
| `evalf(sin(pi(30)/6), 40)` and `…, 100` | `0.4999…9274` (40 digits) | `0.499999999999999999999999999999` (30) |
| `evalf(cos(pi(30)/6), 40)`, `evalf(sin(pi(30)/4), 40)`, `evalf(sin(pi(45)/6), 40)`, `evalf(exp(pi(30)/10), 40)`, `evalf(atan(pi(30)/4), 40)` | 40-place reconstructions | the 30-place folds |
| `evalf(cos(pi), 20)` | matched before | still matches |
| `setprecision(50); evalf(pi, 100)` | 50 decimals | 49 (the 50th is a trailing zero) |

Not claimed as census rows: they are the same defect but the four rows above are the ones the
manifest defers. The census map is **not** edited here (`parity/manifest-map.json` is out of this
lane's scope).

## 6. Tests added (8) and the ids they close

`rust/lovelace-lang/src/round82_evalf_precision.rs` (4 tests, driven through `structured::project_value`):

| Test | Census row |
|---|---|
| `evalf_exact_controls_are_untouched_after_the_precision_repair` | **3** — `…EvalfTruncatedConstantExactnessTests/ExactControls_AreUntouched#0` |
| `evalf_values_below_the_computation_cap_stay_inexact_zeros` | **4** — `…EvalfWorkingPrecisionHonoursRequestedDecimalsTests/ValuesBelowTheComputationCap_StayInexact#0` |
| `sin_of_an_inexact_real_constant_folds_at_the_computation_precision` | the production rule row 4 depends on (`sin(pi(30))` at 30 places, `evalf` at 30/40/100) |
| `evalf_digit_cap_boundary_is_at_the_cap_not_below_it` | boundary: 999 / 1 000 / 1 001 + the exact control |

`rust/lovelace-run/src/round82_evalf_precision.rs` (4 tests, driven through `parity_batch::run_args`):

| Test | Census row |
|---|---|
| `a_request_at_the_cap_is_honoured_in_full_and_unmarked` | **1** — `…EvalfDigitCapHonestyTests/ARequestAtTheCap_IsNotMarked#0` |
| `evalf_count_above_the_ambient_display_precision_crosses_in_full` | **2** — `…StructuredRealPrecisionTests/EvalfCountAboveTheAmbientDisplayPrecision_CrossesInFull#0` |
| `the_digit_cap_boundary_cuts_only_above_the_cap` | boundary: 999 / 1 000 / 1 001 for pi and 1/3, on the result and on the `_` variable |
| `the_over_cap_controls_are_never_marked` | controls: `evalf(1/2, 5000)`, `setprecision(5000); sqrt(2)`, `setprecision(5000); evalf(sqrt(2), 5000)` |

No assertion was weakened: the runner tests compare the 1 002-character value to pi's measured
first-60 and digits-981-1000 tails, and the language tests read the published projection
(`pretty`/`canonical`/`domain`/`nodeCount`/`exact`) rather than re-deriving it.

## 7. Gates

| Gate | Command | Observed | Exit |
|---|---|---|---:|
| Focused, language | `cargo test -p lovelace-lang round82_evalf` | **4 passed; 0 failed; 0 ignored** (1 604 filtered) | 0 |
| Focused, runner | `cargo test -p lovelace-run round82_evalf` | **4 passed; 0 failed; 0 ignored** (413 filtered) | 0 |
| Workspace | `cargo test --workspace` | **5,277 passed; 0 failed; 1 ignored** (21 suites; every result block `ok`). The shared tree already ran **5,269 passed / 1 ignored** before this lane's 8 tests were compiled in (round-80's baseline was 5,261, so other lanes had landed 8 tests); the delta here is exactly these 8. | 0 |
| Corpus | `cargo run --release -p lovelace-corpus` | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| Fixtures | `node parity/tools/gate-fixtures.mjs` | `oracle: 20/20`, `rust: 20/20`, `PASS` | 0 |
| CLI | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16`, `PASS` | 0 |
| Differential | `node parity/diff/run.mjs --cases 200 --seed 1` | generated **200 match / 0 mismatch / 0 error**; replayed **1,758 match / 0 mismatch / 0 error**; `regressions: discovered=1873 replayed=1758 stillMismatching=0 resolved=1758 new=0`; `harness exit: 0`; run `parity/diff/runs/20261112T143418Z-seed1` (precision category 20/20 generated, 215/215 replayed) | 0 |
| Clippy | `cargo clippy --workspace --all-targets --exclude lovelace-desktop -- -D warnings` | `Finished`, no warnings | 0 |
| Format | `cargo fmt -p lovelace-lang -p lovelace-run -p lovelace-real -- --check` | clean (the two new modules were rustfmt'd; no other file in those packages was touched) | 0 |

Oracle sha256 `04eba2de…f90921`. The differential was run twice while this report was being
written, because the shared tree's release binary was rebuilt by concurrent lane work in between:
`3dd0814d…93b651b` (run `…T142837Z`) and `512dda67…332e8a` (run `…T143418Z`). **Both runs were
200/200 generated + 1,758/1,758 replayed with `harness exit: 0`**, and both binaries were built
after this lane's production changes landed (which is why the four row scripts are step 0 of the
reproduction below).

## 8. Scope, shared files and limits

Changed by this round:

* `rust/lovelace-lang/src/lib.rs` — **shared file**: `pi`/`e` at the computation precision (one
  hunk) plus the `#[cfg(test)] mod round82_evalf_precision;` declaration. The concurrent series
  lane's `mod round82_series_laurent;` declaration is untouched.
* `rust/lovelace-lang/src/builtins.rs` — **shared file**: the zero-argument `pi()`/`e()` precision,
  the two-arm fold in the elementary-function branch, the `fold_real_constant` helper, six rows
  added to `fold_unary`, and the `evalf` Real-arm re-materialisation. Hunk-local edits only; no
  other lane's hunk was touched.
* `rust/lovelace-lang/src/numeric.rs` — `into_value`'s `Num::Real` arm uses the plain payload
  bound.
* `rust/lovelace-run/src/main.rs` — one `#[cfg(test)] mod` declaration.
* `rust/lovelace-lang/src/round82_evalf_precision.rs` — new (4 tests).
* `rust/lovelace-run/src/round82_evalf_precision.rs` — new (4 tests).
* `docs/goal-parity-contract/round-82/evalf-precision.md` — this report.

No `.cs` file, fixture, corpus file, `parity/` file, `rust/lovelace-desktop` file or other document
was read for edit or written. `parity/manifest-map.json` still marks the four rows `deferred`; the
map flip belongs to the porting round.

Known, measured differences that remain out of this family (not rows): the C# `evalf` guard
re-evaluation for values with ≥20 integer digits is unchanged; the digit-count refusal vocabulary,
the arity taxonomy and the plot wire rows are other families.

## 9. Reproduction

```powershell
$repo = 'C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME = "$repo\.toolchain\rustup"; $env:CARGO_HOME = "$repo\.toolchain\cargo"
$env:PATH = "$repo\.toolchain\cargo\bin;" + $env:PATH
$env:TMP = "$repo\rust\target\tmp"; $env:TEMP = $env:TMP
Set-Location $repo

# the four rows and the boundary
$scripts = 'evalf(pi, 999)', 'evalf(pi, 1000)', 'evalf(pi, 1001)', 'sin(0)',
           'evalf(sin(pi(30)), 40)', 'evalf(1/(3*10^1000), 30)', 'evalf(1/(3*10^1001), 30)'
foreach ($s in $scripts) {
  dotnet exec Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll --eval $s --omit-functions
  rust/target/release/lovelace-run.exe --eval $s --omit-functions
}

Set-Location "$repo\rust"
cargo test -p lovelace-lang round82_evalf
cargo test -p lovelace-run round82_evalf
cargo test --workspace
cargo run --release -p lovelace-corpus
cargo clippy --workspace --all-targets --exclude lovelace-desktop -- -D warnings
cargo fmt -p lovelace-lang -p lovelace-run -p lovelace-real -- --check

Set-Location $repo
node parity/tools/gate-fixtures.mjs
node parity/tools/gate-cli.mjs
node parity/diff/run.mjs --cases 200 --seed 1
```
