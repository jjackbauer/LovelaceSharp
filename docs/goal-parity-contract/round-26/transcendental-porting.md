# Round 26 — counted transcendental parity

Status: **implemented and gated**. Scope: test modules appended in `rust/lovelace-real/src` and
`rust/lovelace-lang/src` (plus their `#[cfg(test)] mod` registrations), and
`parity/manifest-map.json`. No production code, no `.cs`, no fixture, corpus or gate source was
edited. No documentation outside this report was edited.

## 1. Outcome

| Measure | Before | After |
|---|---|---|
| Manifest coverage | 4 680 / 6 209 (75.37 %) | **4 874 / 6 209 (78.50 %)** |
| New verified mappings | — | **194 net** (195 census rows covered; one row was already ported) |
| Deferred | 1 411 | 1 217 |
| Excluded (unchanged) | 118 | 118 |
| Rust tests | 4 112 | **4 168** (+56), all green |
| Refusals recorded | — | **44** (deferred with the measured reason) |

`node parity/tools/gate-manifest.mjs` prints
`PASS (structural): 6209 test cases dispositioned; 4874 claim Rust coverage (78.5%).` with
`"failures": []`.

## 2. What was ported, by pool

| Pool | Rows | Mapped | Refused | Rust module |
|---|---|---|---|---|
| `Lovelace.Real.Tests` trig (5 classes) | 67 | 42 | 25 | `lovelace-real::round26_trig_parity` (20 tests) |
| `Lovelace.Complex.Tests.ComplexMath` (6 classes) | 69 | 66 | 3 | `lovelace-lang::round26_complex_math_parity` (26 tests) |
| `Lovelace.Complex.Tests` value type (adjacent: the round-24 `Complex`/`LComplex` surface) | 52 | 49 | 3 | 41 rows resolve to `lovelace-lang::complex_round24::*`; 8 rows to `round26_complex_value_parity` (4 tests) |
| `Lovelace.Symbolics.Tests` `evalf` numeric functions | 50 | 36 new + 1 already ported | 13 | `lovelace-lang::round26_evalf_parity` (6 tests) |
| `Lovelace.Run.Tests.BuiltinSurfaceContractTests.Evalf_DigitsOne_IsTheSmallestAcceptedCount` | 1 | 1 | 0 | same `round26_evalf_parity` test |
| **Total** | **239** | **194 new (+1 existing)** | **44** | 56 new tests |

The 42 Real rows are the certified-prefix rows (15), the mpmath precision-oracle rows the kernel
answers correctly (15), the finer-π resolution rows (9), the two symmetry rows, and the oracle
falsifier (1). The 66 ComplexMath rows are 31 functional rows, 9 pi-resolution rows, 4 provenance
rows, 7 Exp/Sqrt inexact-argument rows, 10 inexact-argument-provenance rows and 5 ambient-boundary
rows. The 36 new evalf rows are the 11 `sinh` working-precision rows, 5 digits-one rows, 9
truncated-value exactness rows, 10 exact-fold controls and 1 truncated-rational row.

Two calls preserve the reference semantics rather than a canonical rendering: the real and complex
trig published-digit assertions slice the **raw** `sin_cos_raw` pair (the reference's
`Sin(x, digits)` returns the guarded series value and only its renderer truncates), and the
ComplexMath identity/tolerance rows use the same 10^-40/10^-45 budgets as the C# file.

## 3. Refusals (44), with the measured reason

| Cause | Rows | Evidence |
|---|---|---|
| Range-reduction turn count is an `i64` | 10 | `sin(10^20)`, `sin(10^100)`, `cos(10^100)` at 30/50/100 places: the quotient (~1.6e19 / ~1.6e99) overflows, `trunc_to_long` falls back to 0 and the kernel returns a value outside [-1,1] (e.g. `sin(10^100)` = 0.945509… at 30 places; the reference is −0.372376…). Rows: `Sin_And_Cos_OfTenToTheHundred` and the 9 `1e20/1e100` precision-oracle rows. |
| No special-angle table in the Rust kernel | 5 | `SpecialAngles_AreStillExactAtEveryPrecision`, the 3 `SinAndCos_ExactMultiplesOfTheAmbientPi` rows and `Sin_GivenNegativeSpecialAngles`: exact equality (and equal exactness provenance) with the table is asserted, but `sin(pi/2)` is 0.999… and `cos(pi/6)` is one ulp off the exact value. |
| Fast-path internals / timing | 10 | The ten `RealTrigFastPathTests` rows assert private `TrySpecialAngle`, `ReduceToTwoPi`, `WholeQuotient`, `IsBelowDecimalGuard`, `CosTaylor`, `Normalize`, `ToInteger` and `HalfOf`, or a wall-clock <10 s bound at the 1 000-place default. None of those private surfaces exists in Rust, and a debug-build 1 000-place cosine exceeds the timing budget. |
| Reference display window differs | 1 | `ComplexMathAmbientBoundary.Sin_OfATwoPiTruncation…`: the value and 10^-104 closeness pass, but the row also pins a 30-**fractional-digit** rendering of a 10^-100 residual; Rust `Real` renders 30 **significant** digits for a sub-window value. Dropping that clause would be weaker. |
| Symbolic/numeric exactness still disagree | 7 | `inspect(cos(pi(30))).exact` is `true` (the numeric `evalf` value is inexact); `evalf(subs(x + 1, x, pi(30)), 40)` publishes `exact:true` with numerator `4141592653589793238462643383279` over `10^30`. Covers 2 ComplexMath agreement rows and the 4 `SymbolicTreeFlag_AndNumericReadBack_Agree` rows plus the `subs` truncated-value row. |
| `evalf` cannot parse `pi(1)*1/6` | 2 | `evalf(sin(pi(1)*1/6), 40)` and its cosine are engine refusals: `sin needs a number or an expression.` |
| `(-4)^(1/2)` unsupported | 2 | `evalf((-4)^(1/2), 40)` is `UnsupportedOperation: Non-integer exponents are not yet supported.` This is one control in each of `ExactControls_StayExact` and `ExactControls_AreUntouched`. |
| `evalf` below-cap value moved | 1 | `ValuesBelowTheComputationCap_StayInexact` pins `evalf(sin(pi(30)), 40)` as `0`; Rust publishes `0.0000000000000000000000000000005028841971` (5.0288…e-31). The other two expressions match. |
| No `RealLiteral` carrier in Rust symbolics | 3 | `LiteralRoundTrip_PreservesBothTheDigitsAndTheRoute`, `…KeepsGenuinelyExactValuesExact` and `FunctionOfATruncatedArgument_IsInexact` are quantified over `RealLiteral`/`RealConstantExpr` nodes, which the Rust symbolic tower does not have (`Expr` is rational/symbol/operator only). |
| No `DType`/`DenseArray` typed-array surface | 3 | `DenseArrayOfComplex_*` asserts `DType.Complex`, `ElementType`, `IsContiguous`, `GetElement` and `AsSpan`; the workspace has `NdArray<T>` only, so asserting shape/numel alone would drop most of the C# assertion. |

## 4. Kernel gaps found (production, out of scope for this round)

These are the reason the 250-row objective could not be reached honestly in this batch. Each is
reproducible from the frozen Rust tree; none was worked around by weakening an assertion.

1. **Large-argument reduction overflows `i64`.** `transcendental.rs` converts the turn count to
   `i64` in `trunc_to_long`. For arguments ≥ ~5.6e19 the quotient overflows and silently becomes 0,
   so `sin`/`cos` evaluate the Taylor series on the full argument. `sin(1e20)` returns a
   400-digit integer-part value; `sin(1e100)` ≈ 0.9455 instead of −0.3724. This blocks 10 C# rows.
2. **No special-angle table.** C# `Real.Sin/Cos` reduce exact rational multiples of π onto a 16-row
   table and return exact values; Rust always runs the series. Exact-equality and provenance rows
   (5) are blocked, and it is also why `sin(pi/2)` can be 0.999… at some widths.
3. **Symbolic exactness laundering survives.** `inspect(cos(pi(30))).exact` is true and the
   `subs(x+1, x, pi(30))` fold publishes an exact rational, while the numeric evaluator marks the
   same values inexact. 7 rows are blocked.
4. **`evalf` cannot parse a `pi(1)` product** (`sin needs a number or an expression.`) — 2 rows.
5. **`(-4)^(1/2)` still refused** — 2 control rows.
6. **Reference display window (fractional vs significant digits)** — 1 row.

## 5. Gates (observed, after)

| Gate | Observed | Log |
|---|---|---|
| `cargo test --workspace` | **4 168 passed, 0 failed** across 11 crates / 19 binaries (4 112 + 56 new tests) | `rust/target/tmp/round26-cargo-full.log` |
| `parity/gen-rust-tests.ps1` | `crates=11 tests=4168 suiteExit=0 passed=4168 failed=0` | `rust/target/tmp/round26-gen-rust-tests.log` |
| `node parity/gen-manifest.mjs` | `{"rows":6209,"dispositions":{"excluded":118,"ported":4874,"deferred":1217},"coveragePercent":78.5,"rustTests":4168}` | `rust/target/tmp/round26-gen-manifest.log` |
| `node parity/tools/gate-manifest.mjs` | `PASS (structural): 6209 test cases dispositioned; 4874 claim Rust coverage (78.5%).` `"failures": []` | `rust/target/tmp/round26-gate-manifest.log` |
| `node parity/tools/coverage-report.mjs` | `coverage.md written: 4874/6209 ported (78.50%)` | `rust/target/tmp/round26-coverage.log` |
| `cargo run --release -p lovelace-corpus` | **212 passed, 2 failed, 214 total** — only the pre-existing `sym/071`, `sym/076` | `rust/target/tmp/round26-corpus.log` |
| `node parity/tools/gate-fixtures.mjs` | oracle **20/20**, rust **20/20**, `PASS` | `rust/target/tmp/round26-fixtures.log` |

Per-project coverage after the batch: Real 2 358/2 537 (92.94 %), Complex 115/121 (95.04 %),
Symbolics 718/1 226 (58.56 %), Run 617/751 (82.16 %), total 4 874/6 209 (78.50 %).

## 6. What was not verified, and why 250 was not reached

* The C# test suite itself cannot execute in this environment (VSTest testhost is denied
  `OpenProcess`, CAP-001). Every expected string is either the literal in the C# test (`InlineData`
  / constants) or a value produced by the frozen C# runner / assembly. The trig precision-oracle
  rows are compared with the frozen `evalf` output at the same count, which is the reference
  numeric path; no independent mpmath process is available here.
* The named pools contain **239** rows in total (67 Real trig + 69 ComplexMath + 52 Complex value +
  50 Symbolics evalf + 1 Run evalf), not 250. Of those, 44 are blocked by the measured production
  gaps above, leaving **194 net new mappings** (+1 row already ported) — the honest ceiling of this
  batch. The 52 Complex
  value rows are adjacent rather than transcendental: they were unblocked by the round-24
  `Complex`/`LComplex` surface and were still 0/52 mapped here, so they are included and labelled.
* The 25 refused Real rows and 6 refused Complex rows keep their census disposition `deferred`
  with the measured reason in `parity/manifest-map.json`; none was turned into an exclusion.
* `ln` spelling: the oracle has no `ln` builtin and spells it `log`; every language-level evalf
  assertion uses `log`, and the Real-crate rows call `Real::ln` directly (the port's name).

## 7. Reproduction

```powershell
$repo='C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME="$repo\.toolchain\rustup"; $env:CARGO_HOME="$repo\.toolchain\cargo"
$env:PATH="$repo\.toolchain\cargo\bin;" + $env:PATH
$env:TMP="$repo\rust\target\tmp"; $env:TEMP=$env:TMP
Set-Location "$repo\rust"
cargo test --workspace
cargo run --release -p lovelace-corpus
Set-Location $repo
powershell -ExecutionPolicy Bypass -File parity/gen-rust-tests.ps1
node parity/gen-manifest.mjs
node parity/tools/gate-manifest.mjs
node parity/tools/coverage-report.mjs
node parity/tools/gate-fixtures.mjs
```

## 8. Scope compliance

* Appended tests only in `rust/lovelace-real/src` and `rust/lovelace-lang/src`; no test in
  `rust/lovelace-run/src` was needed because the `evalf` rows are engine-level. The only
  non-test edits are the `#[cfg(test)] mod round26_*` registrations in the two `lib.rs` files;
  production behaviour is unchanged.
* `parity/manifest-map.json` was re-read immediately before the write and merged: 195 mapped rows,
  44 measured refusal reasons, no exclusion touched, no other agent's entries overwritten. One
  previously ported row (`ElementaryFunctionOfAnExactArgument_StaysExact#9`) was restored to its
  original `r15_…` mapping after the overlap was detected, so the reported 194 is net-new.
* `manifest.toml` and `rust-tests.json` were regenerated by their own tools, not hand-edited.
