# Round 21 — DSP porting batch

Scope: append-only tests in `rust/lovelace-lang/src` and `rust/lovelace-run/src`, plus the
`parity/manifest-map.json` entries those tests resolve. No production code, no C# source, no
fixture, corpus or other document was changed.

## 1. Result

- **118 new verified mappings**, all counted only after `gate-manifest.mjs` resolved them to a
  test in the measured inventory and the measured suite ran green.
- Coverage: **4,680 / 6,209 = 75.37 %** (was 4,562 / 6,209 = 73.47 %), **+118**.
- Dispositions: 118 excluded (unchanged), 1,411 deferred (was 1,529).
- Rust suite: **4,043 passed, 0 failed** (was 3,963; +80 tests: 60 in lovelace-lang, 20 in
  lovelace-run). The new tests are `rust/lovelace-lang/src/dsp_round21.rs` and
  `rust/lovelace-run/src/dsp_round21.rs`.
- The brief's 200-row target set (all deferred rows of Lovelace.Dsp.Tests, DspPluginTests,
  DspIntegerArgumentContractTests, PluginIntegerArgumentWireTests, IntegerArgumentBoundaryMatrixTests
  and the eight DSP descriptor examples) resolves to **116 mapped / 84 refused**. Two further
  adjacent `ArityPropertyTests` rows (the noise short/long forms) were mapped outside that set,
  giving 118. The refusals are enumerated in section 4; the observable-equivalence substitutions
  used by the accepted rows are disclosed in section 3.

## 2. Mapped rows by C# class

| C# class | Target rows | Mapped | Refused |
|---|---:|---:|---:|
| Lovelace.Dsp.Tests.DspTests | 17 | 11 | 6 |
| Lovelace.Dsp.Tests.FftTests | 11 | 9 | 2 |
| Lovelace.Dsp.Tests.FixedDspTests | 10 | 0 | 10 |
| Lovelace.Dsp.Tests.FourierTests | 9 | 9 | 0 |
| Lovelace.Dsp.Tests.NoFloatingPointTests | 2 | 2 | 0 |
| Lovelace.Dsp.Tests.TrigTests | 12 | 12 | 0 |
| Lovelace.Suite.Tests.DspIntegerArgumentContractTests | 16 | 15 | 1 |
| Lovelace.Suite.Tests.DspPluginTests | 16 | 5 | 11 |
| Lovelace.Run.Tests.PluginIntegerArgumentWireTests | 13 | 7 | 6 |
| Lovelace.Run.Tests.IntegerArgumentBoundaryMatrixTests | 86 | 38 | 48 |
| Lovelace.Suite.Tests.DescriptorExampleTests (DSP builtins) | 8 | 8 | 0 |
| **Target total** | **200** | **116** | **84** |
| Lovelace.Run.Tests.ArityPropertyTests (adjacent) | 5 | 2 | 3 |
| **Total new mappings** | | **118** | |

Per project: Lovelace.Dsp.Tests 43/61 (70.49 %), Lovelace.Suite.Tests +28 (767/929),
Lovelace.Run.Tests +47 (616/751).

## 3. How the assertions were reproduced

The C# assertion is the contract. Where the C# test exercises a signal or DspMath type the Rust
language surface does not expose, the Rust test composes the builtins that produce the same
observable vector; each such row is called out here and in its manifest note.

| C# assertion shape | Rust evidence |
|---|---|
| Signal samples at an index (Impulse/Step/Delay/Sequence) | finite builtin windows; an out-of-support sample at n = -1 is read as `delay(signal, 1)[0]`, and at n = length as `delay(signal, -1)[length-1]` |
| Scalar(2, Step), ImpulseResponse(unit/FIR), Convolve | one-tap or impulse/tap convolution through `conv`; for a FIR the impulse response is exactly that convolution |
| Fft structured N8/N16/N64 matches Dft | `x = fft([1..n]); fft(x) == dft(x)`; the language has no complex-literal constructor, so the same complex-output transform supplies the complex signal and the asserted fact (fft == dft element for element) is identical |
| Fft real-cosine spectral peaks | the eight bins of `fft(cosine(1/8, 0, 8))` are checked against 16 / 0 with the reference's 1e-41 tolerance |
| Rl.Exp / Rl.Sin / Rl.Cos / Complex.Exp | `exponential` for exp; `sin(x) = cos(pi/2 - x)` through `cosine`, driven at `setprecision(100)` because the DSP test fixture runs at 100 computation digits (TestPrecision.cs) |
| NoFloatingPoint source scans | the Rust test scans the same Lovelace.Dsp / Lovelace.Complex / Lovelace.Real directories with the same patterns; the assertion is a source invariant, reproduced exactly |
| Integer argument contract | the CLR exception subtype and the Failure enum are not on the wire; the Rust tests assert the published code/category/message, which is the surface a consumer branches on. The message is the contract's own sentence (builtin, parameter, bound, received digits) |
| DspPlugin precision knob | the C# test also reads engine.PrecisionExplicitlySet, which is engine-internal; the Rust test asserts the observable digit promotion (15..45 fast digits, then at least +15 after setprecision(60)) |
| Boundary matrix | the same literal, the same must-accept/must-refuse outcome, the same recoverable/code/category rules and the same exit codes through the published runner; the sentinel template proves each row reaches its argument |

## 4. Refusals (84 target rows, 3 adjacent Arity rows)

### Lovelace.Dsp.Tests — 18

| Row(s) | Reason |
|---|---|
| DspTests Sum_GivenImpulseAndStep_AddsPointwise, Product_GivenImpulseAndStep_MultipliesPointwise (2) | Rust has no elementwise vector add/multiply (`Cannot apply '+' to 'Complex' and 'Complex'`); no production change is allowed in this batch, so the pointwise signal algebra has no counterpart. |
| DspTests ImpulseResponse_GivenFirstOrderIir_MatchesHandComputed (1) | No filter/recurrence builtin exists; the IIR response 0.1*0.9^n cannot be expressed by the available builtins without a test-local recurrence, which would test the test. |
| DspTests Sequence_GivenLengthMismatch_ThrowsArgumentException (1) | No Sequence constructor exists in Rust; the length-mismatch refusal is unreachable. |
| DspTests Filter_GivenFir_MatchesConvolution, Filter_GivenIir_MatchesImpulseResponseConvolution (2) | `filter` is advertised in the registry but not dispatched in Rust (descriptor example #53 is refused for the same reason). |
| FixedDspTests all 10 | Rust has no LComplex64/LComplex128 fixed-width type and no LRealPromoteException; the fixed-path promote-on-overflow contract has no counterpart. Mapping these to the class-real builtins would test a different implementation. |
| FftTests Fft_GivenEmpty_ReturnsEmpty (1) | Rust refuses an empty FFT (`dft needs a non-empty vector`); the reference returns an empty array. Divergence, not a weaker assertion. |
| FftTests Fft_GivenNonPowerOfTwo_ThrowsArgumentException (1) | Rust accepts `fft([1,2,3])` and computes a DFT; the reference throws. This is the pre-existing fft-length divergence (the registry's own capability entry still advertises the refusal), recorded rather than mapped. |

### Lovelace.Suite.Tests — 12

| Row(s) | Reason |
|---|---|
| DspIntegerArgumentContractTests TheExactlyIntegralRealRefusal_IsStillTheContractsContract (1) | The test constructs the C# factory's message for an integral Real ("write it as 3"). In the Rust language surface an integral Real is accepted (impulse(2.0) evaluates), and no factory exposes that message; there is nothing to assert. |
| DspPluginTests ComplexAccessors_GivenComplexValue_ReturnRealComponents (1) | re/im/abs match, but Rust `conj` returns the input unchanged (-1i stays -1i) where the reference returns 1i. One of the four assertions fails, so the row cannot be mapped. |
| DspPluginTests ComplexArithmetic_GivenComplex_ThrowsActionableError (1) | The reference message contains "re()/im()/conj()/abs()"; Rust says "Cannot widen from Natural to Complex ...". Message differs. |
| DspPluginTests Abs_GivenNonComplexArray_ThrowsActionableError (1) | The reference says "Complex array"; Rust says "abs needs a number, but got 'Vector'". Message differs. |
| DspPluginTests Abs_GivenComplexVector_ReturnsMagnitudeSpectrum (1) | Rust `abs` refuses a Vector; the reference maps it elementwise. |
| DspPluginTests ReImConj_GivenComplexVector_MapElementwise (1) | Rust re() returns the complex vector unchanged, im() returns a scalar 0 and conj() is identity; the reference maps re/im/conj elementwise. |
| DspPluginTests Dft_GivenLength_MatchesPaddedSignal (1) | Rust `dft(x, n)` ignores the optional length: dft([1,2,3],4) equals dft([1,2,3]), not dft([1,2,3,0]). |
| DspPluginTests Filter_GivenSignal_MatchesConvolution, Filter_GivenLength_ReturnsImpulseResponse (2) | `filter` is not dispatched. |
| DspPluginTests DspPlugin_GivenUnregisteredEngine_ThrowsUnknownFunction, DspPlugin_LoadedTwice_Throws, PluginBuiltin_GivenCoreNameCollision_Throws (3) | Rust has no plugin-loading seam: the DSP names are registered by the core engine, so an unregistered-engine refusal, a double-load refusal and a core-name-collision refusal are unreachable. |

### Lovelace.Run.Tests — 54

| Row(s) | Reason |
|---|---|
| PluginIntegerArgumentWireTests evalir/evalir_batch rows (6: AnIntegralDigitCount, ANonIntegralDigitCount, ADigitCountOutsideTheBound x2, ADigitCountWiderThanInt64, EvalirBatch) | Rust `evalir` ignores the digit argument entirely: every count (including 2.5, 0, -5, 10^30) is accepted and answers the periodic 0.(3). None of the six digit-contract assertions reproduces. |
| Boundary matrix — 27 magnitude/axis cells | 25 deferred cells still diverge: norm(x,0) and mean(x,0) are refused although the row must accept; eye(2,k) refuses 2 columns; concat/dft/range accept a non-integral Real (2.5, 0.5, 2.9999, (5/2)) although the row must refuse; evalir/evalir_batch accept every fractional digit count. Two further null-fate cells (eye(rows, 2^63) and 1..2^63) panic in Rust (arithmetic overflow at builtins.rs:2309) instead of returning a value or a typed recoverable refusal; the C# cell forbids an internal failure, so both are refused. These are pre-existing non-DSP argument-contract gaps, not caused by the DSP builtins. |
| Boundary matrix — 18 template rows | Each sentinel refusal fails the C# grammar: the Rust text is a raw sentence ("Dimensions must be non-negative integers.", "Axis must be an integer.", "Index must be an Integer.", "Range bounds must be numeric.", "series needs an order.", "eye needs an integer size.") instead of the required `builtin(): argument N ...` form, or the call accepts the text (concat, dft, evalir, evalir_batch). |
| Boundary matrix — 3 TheAcceptedValue rows | The evalir digit count does not change the answer in Rust (0.(3) for every count), so the "larger count publishes a longer answer" property fails. |

### Adjacent ArityPropertyTests — 3 not mapped

`eye(2, 3)` is refused by Rust (the C# row requires it to stay callable), and the two
registry-wide sweeps (`DeclaredArity_TravelsWithEveryPluginBuiltin`,
`EveryBuiltin_RefusesEveryCountOutsideItsDeclaredArity`) describe the C# plugin-descriptor
carrier; the Rust registry is the descriptor and there is no separate carrier to compare.

## 5. Verification

All commands were run after the final regeneration.

| Gate | Observed |
|---|---|
| `cargo test --workspace` | 4,043 passed, 0 failed (recorded by gen-rust-tests.ps1: suiteExit=0, passed=4043) |
| `cargo run --release -p lovelace-corpus` | 212 passed, 2 failed, 214 total (unchanged sym/071 and sym/076) |
| `node parity/tools/gate-fixtures.mjs` | oracle 20/20, rust 20/20, PASS |
| `node parity/tools/gate-cli.mjs` | CLI matrix 16/16, PASS |
| `node parity/gen-manifest.mjs` | 6,209 rows; ported 4,680; excluded 118; deferred 1,411; 75.37 % |
| `node parity/tools/gate-manifest.mjs` | PASS (structural): 6,209 dispositioned; 4,680 claim coverage; failures [] |
| `node parity/tools/coverage-report.mjs` | coverage.md written: 4680/6209 ported (75.37 %) |

## 6. What could not be verified

- The C# test suite itself cannot be executed in this environment (VSTest testhost OpenProcess
  denial, CAP-001); the census is the row inventory and the pinned C# runner is the behavioural
  oracle. No C# test was run for this batch, and no row was counted without the Rust suite running.
- The 84 target refusals above are behavioural divergences; none is an unrun check. They stay
  deferred with their existing reasons, not excluded.
- The two null-fate boundary cells #31 (eye(rows, 2^63)) and #316 (1..2^63) are a newly observed
  internal failure: the Rust evaluation panics with an arithmetic overflow inside builtins.rs:2309
  instead of refusing. They are recorded as deferred with that reason; no test asserts a panic.
- The C# PrecisionExplicitlySet flag and the CLR exception subtype/Failure enum have no wire
  counterpart; the Rust evidence is the observable digit count and the published code/category/
  message respectively. These substitutions are disclosed in the manifest notes.

## 7. Reproduction

~~~powershell
$repo='C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME="$repo\.toolchain\rustup"; $env:CARGO_HOME="$repo\.toolchain\cargo"
$env:PATH="$repo\.toolchain\cargo\bin;" + $env:PATH
$env:TMP="$repo\rust\target\tmp"; $env:TEMP=$env:TMP
cd $repo
powershell -ExecutionPolicy Bypass -File parity/gen-rust-tests.ps1
node parity/gen-manifest.mjs
node parity/tools/gate-manifest.mjs
node parity/tools/coverage-report.mjs
cd rust
cargo run --release -p lovelace-corpus
cd ..
node parity/tools/gate-fixtures.mjs
~~~
