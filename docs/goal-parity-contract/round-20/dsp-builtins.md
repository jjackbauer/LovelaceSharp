# Round 20 — the eight missing DSP builtins

Scope: production code in `rust/lovelace-lang/src/builtins.rs` (dispatch plus implementations),
one `#[cfg(test)] mod` line in `rust/lovelace-lang/src/lib.rs`, and the new test module
`rust/lovelace-lang/src/dsp_round20.rs`. No C# source, fixture, corpus, `parity/` file or other
document was changed.

## 1. Result

All eight C# DSP builtins that the Rust registry advertised but the dispatcher refused are now
implemented and dispatched:

| Builtin | Signature | Result |
|---|---|---|
| `movingavg(x, w)` | 2 arguments, positive exact-integer window | moving average over the causal zero-padded window |
| `delay(x, k)` | 2 arguments, signed 64-bit sample offset | `x(n-k)` over the input's own sample range |
| `impulse(n)` | 1 argument, non-negative count | δ[n] over `0..n-1` |
| `step(n)` | 1 argument, non-negative count | u[n] over `0..n-1` |
| `exponential(c, n)` | 2 arguments, non-negative count | `exp(c·k)` for `k = 0..n-1` |
| `cosine(freq, phase, n)` | 3 arguments, non-negative count | `cos(2π·freq·k + phase)` for `k = 0..n-1` |
| `powerseries(k, a, n)` | 3 arguments, non-negative count | `k·a^k·k` for `k = 0..n-1` |
| `noise(scale, disp[, seed], n)` | 3 or 4 arguments; seed is Int32 | deterministic `scale·u + disp` complex samples |

Every result is a `Vector` of `Complex` values, as the C# `FromComplexArray` produces, including
the zero imaginary parts.

## 2. Argument contract

`dsp_integer` is a Rust port of the reference's `ExactIntegerArgument` path:

- Natural and Integer are read exactly; a wider value is named with its own digits.
- A Real is accepted only when it denotes an exact integer (`2.0` is, `2.5` is not), matching
  `ReadExactInteger`'s `Real.TryGetExactInt64` arm. This is wider than the digit-count call
  sites, which refuse every Real; the DSP call sites in C# accept an integral Real and the probe
  `impulse(2.0)` confirms it.
- Bounds are `Unbounded` (delay `k`), `NonNegative` (all sample counts), `Positive` (movingavg
  window) and the Int32 seed range; the refusal text is the reference's own
  `Int64Range.Describe()` tail.
- A rejected Real is `NotAnInteger` with `Real <value>`; a wider Natural/Integer is
  `TooWide`/`OutOfBounds` with its digits. Non-numeric payloads are named by the plugin payload
  type (`PayloadArray`, `String`, `Expr`, …), not by the language kind.
- Arity refusals carry the registry signature (`function`, `parameters`, `expectedMin`,
  `expectedMax`, `variadic`, `actual`), so `noise(1, 0)` publishes exactly
  `expectedMin: 3, expectedMax: 4`.
- `Signal.Sample`'s own guard is reproduced: a count above `int.MaxValue` is refused as
  `The sample range [0, 2147483647] spans more than 2147483647 samples. (Parameter 'end')`.

## 3. Before / after probes

### 3.1 Before

Observed with the pre-change release binary (the build from HEAD, before this round's dispatch
arms existed). The names were present in the registry metadata but absent from dispatch:

| Call | Before |
|---|---|
| `impulse(4)` | `UnknownFunction`: `Unknown function 'impulse'. Did you mean 'impulse'?` |
| `step(4)` | `UnknownFunction`: `Unknown function 'step'. Did you mean 'step'?` |
| `movingavg([1, 2, 3], 2)` | `UnknownFunction`: `Unknown function 'movingavg'. Did you mean 'movingavg'?` |
| `delay([1, 2, 3], 1)` | `UnknownFunction`: `Unknown function 'delay'. Did you mean 'delay'?` |
| `cosine(0.25, 0, 8)` | `UnknownFunction`: `Unknown function 'cosine'. Did you mean one of: 'cosine', 'cos'?` |
| `exponential(0.5, 2)` | `UnknownFunction`: `Unknown function 'exponential'. Did you mean one of: 'exponential', 'exp'?` |
| `powerseries(2, 0.5, 4)` | `UnknownFunction`: `Unknown function 'powerseries'. Did you mean 'powerseries'?` |
| `noise(1, 0, 3)` | `UnknownFunction`: `Unknown function 'noise'. Did you mean 'noise'?` |

### 3.2 After — exact sequences

Each row is one execution of the C# oracle and one of the Rust release runner; the two typed
strings are byte-identical.

| Call | Oracle = Rust |
|---|---|
| `impulse(0)` | `[] (Vector)` |
| `impulse(4)` | `[1, 0, 0, 0] (Vector)` |
| `step(0)` | `[] (Vector)` |
| `step(4)` | `[1, 1, 1, 1] (Vector)` |
| `delay([1, 2, 3], 1)` | `[0, 1, 2] (Vector)` |
| `delay([1, 2, 3], 0)` | `[1, 2, 3] (Vector)` |
| `delay([1, 2, 3], -1)` | `[2, 3, 0] (Vector)` |
| `delay([1, 2, 3], 5)` | `[0, 0, 0] (Vector)` |
| `movingavg([1, 2, 3, 4], 2)` | `[0.5, 1.5, 2.5, 3.5] (Vector)` |
| `movingavg([1, 2, 3, 4, 5], 3)` | `[0.(3), 1, 2, 3, 4] (Vector)` |
| `movingavg([1, 2, 3], 7)` | `[0.(142857), 0.(428571), 0.(857142)] (Vector)` |
| `movingavg([], 3)` | `[] (Vector)` |
| `movingavg([2.5], 3)` | `[0.(8)] (Vector)` |
| `movingavg([0.1], 3)` | `[0.(0)] (Vector)` |
| `powerseries(2, 0.5, 4)` | `[0, 1, 1, 0.75] (Vector)` |
| `powerseries(2, 1/3, 3)` | `[0, 0.(6), 0.(4)] (Vector)` |
| `powerseries(2, 0.5, 0)` | `[] (Vector)` |
| `noise(1, 0, 3)` | `[0.78752594922464908963889065965 + 0.142967033957129734124764188786i, 0.628808168294678177191453648877 + 0.925632821066692909601169416526i, 0.038027993525502486174916686511 + 0.629502856590518655693044202144i] (Vector)` |
| `noise(1, 0, 0, 3)` | identical to `noise(1, 0, 3)` |
| `noise(1, 0, 1, 1)` | `[0.214764391602396626779013717388 + 0.576098805529634664235136319094i] (Vector)` |
| `noise(1, 0, -1, 2)` | `[0.214764391602396626779013717388 + 0.576098805529634664235136319094i, 0.601203539872114947914304333068 + 0.870082843113435301247118939686i] (Vector)` |
| `setprecision(20); noise(1, 0, 2)` | `[0.78752594922464908963 + 0.88906596501429670339i, 0.57129734124764188786 + 0.62880816829467817719i] (Vector)` |
| `noise(0, 1, 2)` | `[1 + 1i, 1 + 1i] (Vector)` |
| `setprecision(50); movingavg([1/3], 3)` | `[0.(1)] (Vector)` |

`noise` is value-for-value deterministic for a given seed: the digits come from a Rust port of
the .NET seeded `Random` (Knuth subtractive generator), and the seed is normalised exactly as
the reference does (`int.MinValue` and `int.MaxValue` both behave like the generator's special
absolute seed).

### 3.3 After — wide transcendental values

`exponential` and `cosine` compute at the DSP precision (30 by default, the engine value after
`setprecision`) using ports of `Real.Exp`, `Real.Sin`/`Real.Cos`, `Real.DivideNonPeriodic`, the
argument reduction, the 16-angle special table and the guard-aware Taylor loops. The stored
rational value is compared through the structured projection, which expands it at the engine
precision. For example:

| Call | Oracle display (100 fractional digits) | Structured `re` |
|---|---|---|
| `exponential(0.5, 2)` | `1.6487212707001281468486507878141635716537760393860552423690659505907263702979744082624533553446847255` | byte-identical to Rust |
| `cosine(0.1, 0, 2)` | `0.8090169943749474241022934171828781764430960130108972599434590525276857908971012151389058336936375942` | byte-identical to Rust |
| `cosine(0.25, 0, 8)` | `[1, 0, -1, 0, 1, 0, -1, 0] (Vector)` | exact values, identical |
| `cosine(1/6, 0, 4)` | `[1, 0.5, -0.5, -1] (Vector)` | exact special angles, identical |
| `exponential(0, 3)` | `[1, 1, 1] (Vector)` | exact ones, identical |

The display strings for the first two rows differ in the Rust runner only because the core
formatter prints every stored digit while the C# formatter truncates at the display bound; the
structured values match digit for digit. This is the pre-existing `sqrt(2)` formatter difference
(see §5), not a DSP value difference.

### 3.4 Argument refusals

All of these produce identical `code`, `category` and `message` in both runners
(`InvalidArgument` / `TypeMismatch`):

| Call | Message |
|---|---|
| `impulse(2.5)` | `impulse(): argument 1 (n) must be a Natural or Integer that denotes an exact integer; got Real 2.5.` |
| `impulse(-1)` | `impulse(): argument 1 (n) must be a Natural or Integer that is a non-negative count (0 or more, at most 9223372036854775807); got -1.` |
| `impulse(999999999999999999999999)` | the same bound, `got 999999999999999999999999.` |
| `movingavg([1, 2, 3], 0)` | `movingavg(): argument 2 (w) must be a Natural or Integer that is a positive count (1 or more, at most 9223372036854775807); got 0.` |
| `movingavg([1, 2, 3], 2.5)` | `movingavg(): argument 2 (w) must be a Natural or Integer that denotes an exact integer; got Real 2.5.` |
| `delay([1, 2, 3], 1.5)` | `delay(): argument 2 (k) must be a Natural or Integer that denotes an exact integer; got Real 1.5.` |
| `delay([1, 2, 3], 999999999999999999999999)` | `delay(): argument 2 (k) must be a Natural or Integer that fits a signed 64-bit integer (at most 9223372036854775807 in absolute value); got 999999999999999999999999.` |
| `cosine(0.25, 0, -2)` | `cosine(): argument 3 (n) must be a Natural or Integer that is a non-negative count (0 or more, at most 9223372036854775807); got -2.` |
| `powerseries(2, 0.5, 2.5)` | `powerseries(): argument 3 (n) must be a Natural or Integer that denotes an exact integer; got Real 2.5.` |
| `noise(1, 0, 2147483648, 2)` | `noise(): argument 3 (seed) must be a Natural or Integer between -2147483648 and 2147483647; got 2147483648.` |
| `noise(1, 0)` | `noise(): expected 3 to 4 arguments; got 2.` |
| `delay([], 1)` | `end must be ≥ begin. (Parameter 'end')` |
| `movingavg(5, 2)` | `DSP builtins expect an array argument, but got a scalar.` (InvalidOperation / DomainError) |
| `impulse(2147483648)` | `The sample range [0, 2147483647] spans more than 2147483647 samples. (Parameter 'end')` |

### 3.5 Differential probe matrix

A 99-call matrix over the eight builtins (three or more inputs each, including `n = 0`, `n = 1`,
negative/zero counts and windows, empty signals, missing optional seed forms, integral-Real
counts, precision changes, periodic operands and the malformed-period fixed-width cases) was
compared envelope-by-envelope against the oracle:

`~
cases=99  structured diffs=0
`~

A further 42-call matrix exercised complex inputs (`fft` results), seed boundaries, very large
offsets, `setprecision(10/50/100)` and the refusal boundary cells:

`~
cases=42  structured diffs=1   (exponential(1/3, 2), see §5)
`~

The tests in §4 also pin every one of those behaviours in the Rust suite.

## 4. Tests and gates

New test module: `rust/lovelace-lang/src/dsp_round20.rs`, 39 tests. They cover exact sequences,
the empty and one-sample edges, the argument refusals above (including the arity signature), the
deterministic noise sequence and seed sensitivity, the explicit-precision digit count, the
fixed-width moving-average period rendering, the periodic exactness flags, and the first 100
published digits of `exponential(0.5, 2)` and `cosine(0.1, 0, 2)`.

| Gate | Result |
|---|---|
| `cargo test --workspace` | **3,963 passed, 0 failed** (was 3,924; +39) |
| `cargo run --release -p lovelace-corpus` | **212 passed, 2 failed, 214 total** (unchanged: `sym/071`, `sym/076`) |
| `node parity/tools/gate-fixtures.mjs` | **oracle 20/20, rust 20/20, PASS** |
| `node parity/tools/gate-cli.mjs` | **CLI matrix 16/16, PASS** |
| `node parity/diff/run.mjs --cases 200 --seed 1 --no-minimize` | **154 match / 46 mismatch / 0 error**; identical to the prior 200-case run `20261110T213641Z-seed1` (delta 0) |

The differential gate still replays the historical regression corpus; its 46 generated-case
mismatches are the pre-existing symbolics/printing/cancellation surface, not DSP cases (the
generator has no DSP family).

## 5. What remains divergent

1. **Display truncation for wide values.** The Rust core renders every digit a `Real` stores;
   C# truncates at `DisplayDecimalPlaces` (100 by default). This affects every wide value, not
   only DSP: `sqrt(2)` prints 1,000 digits in Rust and 100 in the oracle. The DSP values
   themselves match: the structured projections are byte-identical. Fixing this belongs to the
   core formatter, outside this round's scope.
2. **`exponential` with a periodic input.** `exponential(1/3, 2)` returns the mathematically
   correct `1.395612425086…` in Rust; the C# fixed-width complex path returns
   `1.355809135274…`. The same C# fixed-width quirk is visible for `exponential(2/3, 2)`,
   `exponential(1/6, 2)` and `exponential(-1/3, 2)`. Finite-decimal inputs (`0.5`, `0.1`, `1`,
   `2`, `-0.5`) match exactly. This is the only divergence found in the two probe matrices and
   is recorded rather than silently "fixed" to a mathematically wrong value.
3. **Fixed-width moving-average malformed periods.** The C# `LReal128.Divide` emits malformed
   period metadata when the remainder repeats at the first fractional step with a non-zero
   integer quotient; e.g. `movingavg([2.5], 3)` renders `0.(8)` where the correctly formed
   quotient is `0.8(3)`, and `movingavg([0.1], 3)` renders `0.(0)`. The Rust implementation
   reproduces the observable rendering and exactness flag for the builtin result. It cannot
   reproduce the same malformed *internal* rational because `lovelace-real` exposes no way to
   construct that inconsistent state; chained arithmetic on such a result (for example
   `movingavg([2.5], 3) * 9`) therefore follows the rendered value rather than the reference's
   hidden 5/6. No corpus/fixture/differential case exercises that chain.

## 6. Reproduction

`~powershell
$repo='C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME="$repo\.toolchain\rustup"; $env:CARGO_HOME="$repo\.toolchain\cargo"
$env:PATH="$repo\.toolchain\cargo\bin;" + $env:PATH
$env:TMP="$repo\rust\target\tmp"; $env:TEMP=$env:TMP
cd $repo\rust
cargo test --workspace
cargo run --release -p lovelace-corpus
cd $repo
node parity/tools/gate-fixtures.mjs
node parity/tools/gate-cli.mjs
node parity/diff/run.mjs --cases 200 --seed 1 --no-minimize
`~
