# Round 61 — DSP behavioural family closed (7 rows)

**Status: complete.** The seven `dsp` rows deferred by
`docs/goal-parity-contract/round-50/resweep.md` were re-executed against the pinned C# oracle
`1cc28f255eca6029849f880949fc7be1d40ab8af` and the release Rust runner at HEAD `95de16a`.
All seven now publish the same normalised envelope (the differential's volatility mask:
`revision`, `elapsed`, `elapsedTime`, `timings`). Four rows were already answered by the
round-51 dispatcher arms for their bare `filter` script; the remaining measured gaps were the
empty FFT, the non-power-of-two FFT, the truncated Complex widening refusal — and the C# rows'
own `conv` composition, which the round-51 mapping had translated rather than executed.

No `.cs` file, fixture, `parity/` file or existing test expectation was changed. Production
work is in `rust/lovelace-lang/src/builtins.rs`; the seven new tests are in
`rust/lovelace-lang/src/dsp_family_round61.rs`.

## 1. Result

| # | Census id | Before | After |
|---:|---|---|---|
| 1 | `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.DspTests/Filter_GivenFir_MatchesConvolution#0` | filter script matched; C# `conv` prefix differed in element kind (Natural vs Complex) | filter and `conv` prefix match |
| 2 | `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.DspTests/Filter_GivenIir_MatchesImpulseResponseConvolution#0` | filter script matched; `conv([1,1,1,1], filter([1,-0.5],[1],4))` refused | full composition matches |
| 3 | `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.FftTests/Fft_GivenEmpty_ReturnsEmpty#0` | exit **1 vs 0**; Rust `DomainError: dft needs a non-empty vector.` | both exit 0, `[] (Vector)` |
| 4 | `Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.FftTests/Fft_GivenNonPowerOfTwo_ThrowsArgumentException#0` | exit **0 vs 1**; Rust answered `[3, 0, 0]` | both exit 1, `InvalidArgument` / `TypeMismatch`, oracle sentence |
| 5 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.DspPluginTests/ComplexArithmetic_GivenComplex_ThrowsActionableError#0` | both exit 1; Rust message stopped after `… support widening.` | full oracle message incl. `re()/im()/conj()/abs()` |
| 6 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.DspPluginTests/Filter_GivenLength_ReturnsImpulseResponse#0` | matched | matched (pinned by a new test) |
| 7 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.DspPluginTests/Filter_GivenSignal_MatchesConvolution#0` | filter matched; the row's own `conv` composition refused | full composition matches |

Normalised probe (`--eval <script> --omit-functions`, oracle vs Rust): **zero differing paths**
for all nine scripts below, including both C# compositions.

## 2. Exact before/after values

### Row 1 — FIR filter equals the convolution prefix

`filter([1], [1, 2, 3], [1, 1, 1, 1])`

| Field | Oracle | Rust before | Rust after |
|---|---|---|---|
| `$.result.display` | `[1, 3, 6, 6]` | `[1, 3, 6, 6]` | `[1, 3, 6, 6]` |
| `conv([1,1,1,1],[1,2,3])[0].kind` | `Complex` (`re=1`, `im=0`) | `Natural` (`value=1`) | `Complex` (`re=1`, `im=0`) |
| `conv` over a filter result | answers | `Cannot apply '*' to 'Natural' and 'Complex'.` | answers |

The C# assertion (`DspTests.cs:140`) is `filtered[i] == Convolve(x,b)[i]`; the oracle's
four-sample prefix is exactly `[1, 3, 6, 6]`.

### Row 2 — IIR filter equals the impulse-response convolution

`filter([1, -0.5], [1], [1, 1, 1, 1])`

| Field | Oracle | Rust before | Rust after |
|---|---|---|---|
| `filter` `$.result.display` | `[1, 1.5, 1.75, 1.875]` | `[1, 1.5, 1.75, 1.875]` | same |
| `conv([1,1,1,1], filter([1,-0.5],[1],4))` | exit 0, `[1, 1.5, 1.75, 1.875, 0.875, 0.375, 0.125]` | exit 1, `Cannot apply '*' to 'Natural' and 'Complex'.` | exit 0, same vector |

### Row 3 — empty FFT is the empty transform

`fft([])`

| Field | Oracle | Rust before | Rust after |
|---|---|---|---|
| `$.exitCode` | `0` | `1` | `0` |
| `$.result.display` | `[]` | — | `[]` |
| `$.code` / `$.category` | — | `InvalidOperation` / `DomainError` | — |
| `$.diagnostics[0].message` | — | `dft needs a non-empty vector.` | — |

`dft([])` has the same `N == 0` early return in `DspMath.Dft`; it is now empty in Rust too
(pinned in the same test).

### Row 4 — non-power-of-two FFT is a typed refusal

`fft([1, 1, 1])`

| Field | Oracle | Rust before | Rust after |
|---|---|---|---|
| `$.exitCode` | `1` | `0` | `1` |
| `$.result.display` | — | `[3, 0, 0]` | — |
| `$.code` | `InvalidArgument` | — | `InvalidArgument` |
| `$.category` | `TypeMismatch` | — | `TypeMismatch` |
| `$.diagnostics[0].message` | `FFT length must be a power of two, but got 3. (Parameter 'x')` | — | identical |
| `$.diagnostics[0].position` | `0` | — | `0` |

### Row 5 — Complex widening refusal is actionable

`fft([0, 1, 0, 0])[1] + 1`

| Field | Oracle | Rust before | Rust after |
|---|---|---|---|
| `$.exitCode` | `1` | `1` | `1` |
| `$.code` / `$.category` | `InvalidOperation` / `DomainError` | same | same |
| `$.diagnostics[0].message` | `Cannot widen from Natural to Complex: only numeric kinds (Natural, Integer, Real) support widening. Reductions over Complex arrays (sum/mean/dot/norm/matmul) are not supported; use re()/im()/conj()/abs() to bridge back to Real.` | `Cannot widen from Natural to Complex: only numeric kinds (Natural, Integer, Real) support widening.` | identical to oracle |

### Row 6 — scalar length returns the impulse response

`filter([1, -0.5], [1], 4)` — oracle and Rust both answer `[1, 0.5, 0.25, 0.125] (Vector)`
before and after. This row was already correct at HEAD; the new test pins it.

### Row 7 — signal filter equals the convolution with its impulse response

`filter([1, -0.5], [1], [1, 1, 1, 1])` and
`conv([1, 1, 1, 1], filter([1, -0.5], [1], 4))`

Identical to row 2: the direct filter already matched, while the row's own convolution
composition refused before the `conv` fix; both now match elementwise in `re` and `im`.

## 3. Production changes

All in `rust/lovelace-lang/src/builtins.rs`:

1. **`fft` is no longer an alias of `dft`.** The reference `DspMath.Fft` returns `[]` for an
   empty signal *before* the length test, and throws `ArgumentException` for a non-power-of-two
   length. `fft` now implements that exact order and taxonomy
   (`InvalidArgument` / `TypeMismatch`, `(Parameter 'x')`). `dft` keeps the generic
   any-length/padded transform and now also returns `[]` for an empty input (the reference
   `DspMath.Dft` `N == 0` path). Both share one `transform` kernel, so no summation-order
   behaviour changed for power-of-two inputs.
2. **`conv` is Complex-valued and empty-safe.** `DspPlugin.Convolve` converts every sample
   through `ToComplex` and returns the `Cplx` array, so the result is a Complex vector even when
   both inputs are Natural, and `[]` when either input is empty. The old generic
   `add`/`mul` path published Natural elements and refused empty inputs; it also refused a
   Complex operand, which is what rows 2 and 7's C# compositions exercise.
3. **The widening refusal carries the `Value.Widen` hint.** The binary `+` path now appends the
   reference's Complex hint (and the Symbolic hint for the same code path), matching
   `Lovelace.Suite/Value.cs:252`; Natural-to-Text and other non-numeric pairs still carry no
   hint.

## 4. Tests

`rust/lovelace-lang/src/dsp_family_round61.rs` (7 tests, registered in `lib.rs`):

| Census row | Rust test |
|---|---|
| `Filter_GivenFir_MatchesConvolution#0` | `filter_fir_signal_matches_the_convolution_prefix` |
| `Filter_GivenIir_MatchesImpulseResponseConvolution#0` | `filter_iir_signal_matches_the_impulse_response_convolution` |
| `Filter_GivenLength_ReturnsImpulseResponse#0` | `filter_scalar_length_returns_the_impulse_response` |
| `Filter_GivenSignal_MatchesConvolution#0` | `filter_signal_matches_the_convolution_with_its_impulse_response` |
| `Fft_GivenEmpty_ReturnsEmpty#0` | `fft_of_an_empty_signal_is_the_empty_transform` |
| `Fft_GivenNonPowerOfTwo_ThrowsArgumentException#0` | `fft_of_a_non_power_of_two_length_is_a_typed_refusal` |
| `ComplexArithmetic_GivenComplex_ThrowsActionableError#0` | `complex_arithmetic_refusal_names_the_bridge_builtins` |

Each test asserts oracle-observed fields: the typed/displayed values, the wire code and category
of the two refusals, and the refusals' exact sentences. No existing expectation was edited.

## 5. Gate outputs (observed this round)

| Gate | Command | Observed | Exit |
|---|---|---|---:|
| Full suite | `cargo test --workspace` | `5061 passed, 0 failed, 0 ignored` over 19 suites (baseline 5,054 + 7) | 0 |
| New tests | `cargo test -p lovelace-lang dsp_family_round61` | `7 passed; 0 failed` | 0 |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | `Finished` dev profile, no warnings | 0 |
| Format | `cargo fmt --all -- --check` | no diff | 0 |
| Corpus | `cargo run --release -p lovelace-corpus` | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| Fixtures | `node parity/tools/gate-fixtures.mjs` | `oracle: 20/20`, `rust: 20/20` | 0 |
| CLI matrix | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16` | 0 |
| Differential | `node parity/diff/run.mjs --cases 200 --seed 1 --no-minimize` | generated `200/200`, replayed `1758/1758`, `stillMismatching=0`, `new=0` | 0 |

## 6. Residual / handoff

* **Manifest mapping is not mine to write.** `parity/` is outside this lane's scope. At HEAD
  the map still has rows 1, 2, 6, 7 mapped to the round-51 dispatch test and rows 3, 4, 5
  `deferred`; these seven ids should be re-pointed to the tests above by the parity/manifest lane
  (the tests exist and pass).
* **Radix-2 rounding is not pinned.** The oracle's `Fft` is a radix-2 Cooley–Tukey transform whose
  butterfly order can differ from the DFT in the last digits for irrational twiddles. No measured
  `dsp` row exercises that difference, and the Rust path keeps the one shared DFT kernel so all
  currently green `fft`/corpus cases stay byte-identical. This is not a measured divergence.
* **Unmeasured adjacent surfaces now moved with `conv`:** empty inputs answer `[]` and symbolic
  inputs are refused with the DSP element sentence, both matching `DspPlugin`; no row or fixture
  depended on the old behaviour.
