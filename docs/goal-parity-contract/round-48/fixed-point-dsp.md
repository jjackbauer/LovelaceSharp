# Round 48 — the fixed-point DSP surface (`FixedDsp`), ported additively

**Status: implemented and verified.** The 10 census rows that round 45 deferred as
`UNRUNNABLE` on "fixed-point Cplx64/Cplx128 Convolve/ImpulseResponse/MovingAverage API" now
have one mirrored Rust test each, and all of them pass. The surface is
`rust/lovelace-lang/src/fixed_dsp.rs` (production) with `rust/lovelace-lang/src/fixed_dsp_tests.rs`
(tests); `lib.rs` gains two declarations (`pub mod fixed_dsp;` and the `#[cfg(test)]` test
module). No `.cs`, fixture, `parity/` file or other documentation was edited.

## 1. Result

| Item | Value |
|---|---|
| Missing API closed | fixed-width complex DSP kernels over 19- and 38-digit exact decimals |
| Census rows unblocked | **10** (`Lovelace.Dsp.Tests … FixedDspTests`, one mirrored test each) |
| Production file | `rust/lovelace-lang/src/fixed_dsp.rs` (355 lines) |
| Test file | `rust/lovelace-lang/src/fixed_dsp_tests.rs` (303 lines, 17 tests: the 10 rows + 7 extra contracts) |
| Values asserted | the C# arrays, element for element, plus parity with the arbitrary-precision path |
| Shared workspace edits | `lib.rs`: one `pub mod`, one test-module declaration |

## 2. The C# surface, member by member

`Lovelace.Dsp.FixedDsp` (`Lovelace.Dsp/FixedDsp.cs`) is the whole-array DSP tier over the
fixed-width complex structs `LComplex64` (over `LReal64`) and `LComplex128` (over `LReal128`).
It has eight public members (two widths × four operations) and one private recurrence:

| C# member | C# line | Rust item |
|---|---:|---|
| `Convolve(IReadOnlyList<Cplx64>, IReadOnlyList<Cplx64>)` | 25 | `fixed_dsp::convolve::<W64>` |
| `Convolve(IReadOnlyList<Cplx128>, IReadOnlyList<Cplx128>)` | 51 | `fixed_dsp::convolve::<W128>` |
| `ImpulseResponse(a, b, n)` over `Cplx64` | 81 | `fixed_dsp::impulse_response::<W64>` |
| `StepResponse(a, b, n)` over `Cplx64` | 85 | `fixed_dsp::step_response::<W64>` |
| `ImpulseResponse(a, b, n)` over `Cplx128` | 89 | `fixed_dsp::impulse_response::<W128>` |
| `StepResponse(a, b, n)` over `Cplx128` | 93 | `fixed_dsp::step_response::<W128>` |
| `MovingAverage(x, window)` over `Cplx64` | 171 | `fixed_dsp::moving_average::<W64>` |
| `MovingAverage(x, window)` over `Cplx128` | 193 | `fixed_dsp::moving_average::<W128>` |
| `DifferenceEquation(a, b, n, stepInput)` (private) | 96, 131 | `fixed_dsp::difference_equation` (private, generic) |

Value type: the C# kernels operate on `LComplex64`/`LComplex128`, whose components are
`LReal64`/`LReal128` and whose arithmetic is component-wise (`Lovelace.Complex/LComplex64.cs`):
`+`/`-` are component-wise (46-47), `*` is the FOIL form
`new(a.Re*b.Re - a.Im*b.Im, a.Re*b.Im + a.Im*b.Re)` (49-50), `/ LReal` is component-wise
division (54), `/ LComplex64` is `a * b.Reciprocal` (53) and
`Reciprocal = Conjugate / MagnitudeSquared` (67). The Rust value is
`fixed_dsp::Cplx<W>` (`Cplx64 = Cplx<W64>`, `Cplx128 = Cplx<W128>`), a pair of
`lovelace_real::lreal::Fixed<W>` components with exactly those operations:

| `Cplx<W>` item | Reference member |
|---|---|
| `new`, `from_real`, `zero`, `one`, `from_i64` | `LComplex64` constructors, `Zero`, `One` |
| `re`, `im`, `is_zero` | `Re`, `Im` |
| `add`, `sub`, `mul` | `+`, `-`, `*` (49-50 order) |
| `div_scalar` | `LComplex64 / LReal` (component-wise division) |
| `reciprocal`, `div` | `Reciprocal` (67), `LComplex64 / LComplex64` (53) |
| `to_complex` | `ToComplex` (84) |
| `Display` | `ToString` (103-110): a zero part is omitted, a nonzero pair is `re ± im·i` |

The components are the period-preserving fixed reals in `lovelace-real`, not the fixed complex in
`lovelace-lang::complex`: `complex::LComplex64` states that it does not model the reference's
periodic expansions, and `MovingAverage` divides by the window with `LReal64.Divide`, which
stores a repeating quotient as a period. `lovelace_real::lreal` is the statement-for-statement
port of `LReal64`/`LReal128`, so this tier keeps the reference's values for every input.

## 3. The quantization and rounding rules (read from the C# source)

Numbers below are `LReal64`; the `LReal128` values are in parentheses. The width constants are
`MaxSignificantDigits` 19 (38) and `WorkingFractionalDigits` 18 (37) — `LReal64.cs:26,29`,
`LReal128.cs:119-120`.

1. **Representation.** A value is `significand × 10^exponent`, significand an unsigned integer of
   at most 19 (38) digits, plus optional period metadata `(periodStart, periodLength)`. A
   non-periodic fraction is normalized by stripping trailing zeros while the exponent is negative;
   a zero is canonical and never signed (`LReal64.cs:194-201`, `203-210`).
2. **Addition/subtraction.** Zero is an identity (`a + 0` returns `a`, carrying provenance); the
   operands are aligned at the smaller exponent by multiplying the higher-exponent significand by
   10, and the integers are added/subtracted exactly. A shifted magnitude that overflows
   `UInt128` promotes; a result whose normalized significand exceeds the width promotes
   (`216-245`, `352-361`).
3. **Multiplication.** Zero short-circuits to zero (provenance kept). Otherwise the exact
   significand product is formed (`UInt128` for 64, `UInt256` for 128) and normalized; a product
   that does not fit the width promotes (`247-262`; `LReal128.cs:288-300`). This is where
   `Convolve64_GivenWideOperands` promotes: 19 nines squared needs 38 digits, and 38 nines squared
   needs 76.
4. **Division.** Zero divisor throws `DivideByZeroException`; zero dividend is zero. Otherwise the
   exact long division runs, recording each remainder's first position. A repeated remainder is a
   period: it is stored as `(pStart, pLen)`, and `pLen > 19` (38) promotes. A terminating quotient
   is normalized and promoted if it exceeds the width. The loop is bounded at 256 (512) fractional
   digits; exhausting it promotes. Never rounds (`264-321`; `LReal128.cs:302-346`).
5. **Periodic operands.** Before an operation involving a periodic operand, the periodic value is
   written out to exactly 18 (37) fractional places, truncating the repeat and marking the result
   inexact (`380-398`). After the operation the result is scanned for the shortest period — first
   exactly, then with one slack digit — and stored as a period (`400-455`). The all-nines period
   is the one place a digit is **altered**: it carries +1 into the integer/non-repeating prefix
   (`417-427`).
6. **The expansion boundary is not a period.** If the quotient of two expanded periodic operands
   has a period at least as long as the working width, that repeat is the expansion boundary, not
   the value's own period: it is stripped to an inexact prefix (`274-279`; `LReal128.cs:311-315`).
7. **Provenance.** `IsExact` is false once a truncation produced the digits; it propagates through
   every operation, and equality, ordering and rendering ignore it (`46-60`). `0 × anything` is
   the zero, marked inexact when either operand is inexact.
8. **The DSP layer's fixed operation order**, which is part of the contract because each
   intermediate is quantized:
   * `Convolve`: `sum = 0`; for `m` ascending, `sum += x[m]·h[n−m]`; an empty input returns the
     empty array before any arithmetic (`25-48`).
   * `DifferenceEquation`: `acc = 0`; for `j = 1…` ascending, `acc -= a[j]·y[j−1]`; then for
     `j = 0…` ascending, `acc += b[j]·x[j]`; then `acc /= a[0]`, which is the complex reciprocal
     path `acc · (conj(a[0]) / |a[0]|²)`. The input history is pre-filled with `One` for a step
     and `x[0] = One` for an impulse; the output history starts at zero; `y` and (for the impulse)
     `x` shift right one place per sample and `x[0]` becomes zero for the impulse (`96-164`).
   * `MovingAverage`: `divisor = LReal64.Parse(window.ToString())`; for each `n` the sum runs over
     `k = max(0, n−window+1) … n` ascending and the sample is `sum / divisor` through the
     component-wise `LComplex64 / LReal` operator, so a repeating quotient keeps its period
     `(1/3 = 0.(3))` rather than promoting (`171-190`).
   * Guards: an empty `a` throws `ArgumentException`; `window <= 0` throws
     `ArgumentOutOfRangeException` before any sample; a zero `a[0]` reaches
     `DivideByZeroException` (`100-101`, `174-175`, `117`).
9. **No rounding.** Every member is exact; a value that does not fit promotes. The only value
   change anywhere in the chain is rule 5's all-nines carry.
10. **Rendering vs. equality.** The C# fixed real carries `DisplayDecimalPlaces = 7` by default
    (`LReal64.cs:32`), while the Rust port renders at 100 places
    (`lovelace-real` `DEFAULT_DISPLAY_DECIMAL_PLACES`). The mirrored assertions compare values
    (bit-for-bit decimal equality), which is display-independent; the two tests that assert text
    use short and periodic values (`2`, `0.(3)`, `0.(6)`, `1`) where both renderings agree.

## 4. The 10 census rows and their mirrored tests

Every id is the full census key. The Rust test is `lovelace-lang::fixed_dsp_tests::<name>`; the
concrete arrays and expected values are the C# ones.

| # | Census id | Rust test | Asserted values |
|---:|---|---|---|
| 1 | `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.FixedDspTests/Convolve64_GivenTwoOnes_ProducesTriangle#0` | `convolve64_given_two_ones_produces_triangle` | `[1,1] ⊗ [1,1] = [1, 2, 1]`, narrowed into `complex::LComplex64` as `[One, Parse("2"), One]` |
| 2 | `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.FixedDspTests/Convolve64_Given123SelfConvolution_MatchesDspMath#0` | `convolve64_given_123_self_convolution_matches_dsp_math` | `[1,2,3] ⊗ [1,2,3] = [1, 4, 10, 12, 9]` and equal to the `Complex` (DspMath) result |
| 3 | `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.FixedDspTests/Convolve128_Given123SelfConvolution_MatchesDspMath#0` | `convolve128_given_123_self_convolution_matches_dsp_math` | same values over the 38-digit width, equal to the `Complex` result |
| 4 | `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.FixedDspTests/Convolve64_GivenWideOperands_ThrowsPromoteException#0` | `convolve64_given_wide_operands_throws_promote_exception` | `9999999999999999999²` → `Promote` |
| 5 | `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.FixedDspTests/Convolve128_GivenWideOperands_ThrowsPromoteException#0` | `convolve128_given_wide_operands_throws_promote_exception` | `(38 nines)²` → `Promote` |
| 6 | `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.FixedDspTests/ImpulseResponse64_GivenFir_MatchesTaps#0` | `impulse_response64_given_fir_matches_taps` | `a=[1], b=[1,2,3], n=5` → `[1, 2, 3, 0, 0]` |
| 7 | `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.FixedDspTests/ImpulseResponse128_GivenFir_MatchesTaps#0` | `impulse_response128_given_fir_matches_taps` | same taps over the 38-digit width |
| 8 | `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.FixedDspTests/MovingAverage64_GivenPowerOfTwoWindow_MatchesDspMath#0` | `moving_average64_given_power_of_two_window_matches_dsp_math` | samples `i % 7` for `i = 0…7`, window 4 → `[0, 0.25, 0.75, 1.5, 2.5, 3.5, 4.5, 3.75]` and equal to the `Complex` result |
| 9 | `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.FixedDspTests/MovingAverage128_GivenPowerOfTwoWindow_MatchesDspMath#0` | `moving_average128_given_power_of_two_window_matches_dsp_math` | same samples and averages over the 38-digit width |
| 10 | `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.FixedDspTests/MovingAverage64_GivenNonPositiveWindow_ThrowsArgumentOutOfRange#0` | `moving_average64_given_non_positive_window_throws_argument_out_of_range` | window `0` and `-3` → `WindowNotPositive` |

The other seven tests cover the contract the ten rows share but do not themselves state: the step
response pre-fills the input history (`[6,6,6,6,6]` for `a=[1], b=[1,2,3]`); an IIR recurrence
(`a=[1,−0.5], b=[1]` → `[1, 0.5, 0.25, 0.125]`); a scalar denominator goes through the complex
reciprocal (`a=[2], b=[1]` → `[0.5, 0, 0]`); FOIL order (`(1+i)(1−i) = 2`); the period rule
(`window 3` over `[1,1,1]` → `0.(3), 0.(6), 1`); the width boundary (19 nines × 1 keeps 19
digits, × 10 promotes); and the empty-denominator guard.

## 5. Where the values were checked

The C# assertions are the contract and were transcribed from `FixedDspTests.cs` itself. The two
`MatchesDspMath` families were then re-derived independently in Rust: the class result is computed
with `complex::Complex` arithmetic using `DspMath`'s own summation/moving-average shape
(`DspMath.cs:16-39, 165-175`), and the fixed result is promoted with `Cplx::to_complex` and
compared to it — exactly the C# comparison `Assert.Equal(classResult, fixedResult.Select(v =>
v.ToComplex()).ToArray())`. All 17 tests pass:

```
cargo test -p lovelace-lang fixed_dsp
running 17 tests
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 1454 filtered out
```

The C# oracle test run was attempted (`dotnet test Lovelace.Dsp.Tests --filter FixedDspTests`)
but the vstest testhost cannot start in this environment: it fails with
`System.ComponentModel.Win32Exception (5): Access is denied` while arming its parent-process exit
watch (a harness process-handle restriction, not a code failure). The assertions were therefore
taken from the C# source, which is the contract, and cross-checked against the independent
arbitrary-precision path above.

## 6. Gate outputs (observed this round)

Run on 2026-11-11 (America/Sao_Paulo), with the pinned toolchain
`cargo 1.99.1 (5f94df478 2026-08-27)` / `rustc 1.99.1 (58cf8f9c3 2026-10-13)` and
`node v24.16.0`.

| Check | Command | Observed | Exit |
|---|---|---|---:|
| Rust suite | `cargo test --manifest-path rust/Cargo.toml --workspace` | `workspace exit=0 binaries=19 passed=4952 failed=0` | 0 |
| Fixed-point tests | `cargo test -p lovelace-lang fixed_dsp` | `running 17 tests`; `17 passed; 0 failed; 0 ignored; 0 measured; 1454 filtered out` | 0 |
| Documentation corpus | `cargo run --release -p lovelace-corpus` | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| Protocol fixtures | `node parity/tools/gate-fixtures.mjs` | `oracle: 20/20`, `rust: 20/20`, `PASS: oracle 20/20, rust 20/20` | 0 |
| CLI matrix | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16`, `PASS: every RUN-001..RUN-014 row behaves like the oracle` | 0 |
| Lints | `cargo clippy --workspace --all-targets -- -D warnings` | no `warning:`/`error` line | 0 |
| Formatting | `cargo fmt --all -- --check` | no output | 0 |

**Arithmetic of the count.** 4,926 (the round-47 inventory) + 17 (this round) + 9 (the parallel
`lovelace-run` lane's new `record_schema_batch.rs` tests, which are present in the working tree)
= 4,952. The 17 `fixed_dsp` tests are part of the 1,471-test `lovelace-lang` lib binary.

**Caveat on the runner-based gates.** `gate-fixtures.mjs` and `gate-cli.mjs` execute the prebuilt
`rust/target/release/lovelace-run.exe` (built 18:57, at commit `d92fbcb`, before this round); they
do not build. The additive module is not referenced by the runner and adds no builtin, so the
runner's behaviour is unchanged. The release build of the modified crate is nevertheless exercised:
the corpus gate compiled `lovelace-lang` in release (with `fixed_dsp`) before its 214/214 pass, and
the workspace suite compiled every crate including the parallel lane's runner changes
(`binaries=19`).

## 7. Notes, boundaries and residual

* **Cancellation is not modelled.** `FixedDsp` polls `KernelCancellation` once per output sample
  and once per inner term; these kernels are pure slices-in/value-out with no cancellation seam,
  because no caller drives them yet. When a caller adopts them, the seam is one poll per loop.
* **The 64/128 split is generic.** The reference has two overloads per member; the Rust surface is
  one generic function over `LRealWidth` with the `Cplx64`/`Cplx128` aliases, which is an
  additive equivalent and not a behaviour change.
* **Periodic vs. `complex::LComplex64`.** A component that repeats keeps its period in
  `Cplx`; narrowing it with `complex::LComplex64::try_from_complex` returns `None` for exactly
  those values (that type's stated boundary), so the two types are composable but not identical.
* **Out of scope, unchanged.** `parity/manifest-map.json` was not edited: the 10 ids are still
  `deferred` until the parent maps them to these tests. 293 of the 303 round-45 UNRUNNABLE rows
  remain; `.cs` files, fixtures, `parity/` and every other document are untouched.
* **Tree state inherited, not touched.** Another lane's `rust/lovelace-run` edits and the
  already-modified `parity/rust-tests.json` were in the working tree and were not modified here.
