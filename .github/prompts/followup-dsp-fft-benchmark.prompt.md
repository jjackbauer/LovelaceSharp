# Follow-up Prompt — DSP FFT + Large-Signal Benchmark

> Type: follow-up task prompt (start here in a fresh session)
> Predecessor work: the Lovelace DSP core, transcendentals, and language/Studio integration are already landed (see Context below). This session's job is **twofold**: (1) add a radix-2 **FFT** alongside the existing O(N²) DFT, and (2) build a **dedicated benchmark project** over realistic, larger signals.

---

## 0. Hard rules (non-negotiable, carried over)

1. **No IEEE floating point anywhere.** Every numeric value is `Lovelace.Real.Real` (`Rl`) or `Lovelace.Complex.Complex` (`Cplx`). No `double`, `float`, `System.Numerics.Complex`, `Math.Cos/Sin/Exp`, or `Random.NextDouble()`.
2. **Irrational division must use `Real.DivideNonPeriodic`** (internal), not `operator /`. `operator /` runs period detection and invents a spurious short period on truncated-irrational operands (this already bit us — see `Real.SinTaylor`/`CosTaylor`/`ExpTaylor`).
3. **Special angles are exact only when the angle is built as a reduced rational multiple of the cached `Real.Pi`.** Use the same pattern as `DspMath.Dft`: `gcd(2k, N)` → `num`/`den` → `angle = Pi * num / den`. This is what makes `cos(π/3)`, `cos(π/4)`, DFT roots-of-unity for N = 2,3,4,6 return *exact* rationals/sqrts.
4. **The `Complex` type is `Lovelace.Complex.Complex`** — a namespace name collides with the type name. Outside `Lovelace.Complex`, alias it (`using Cplx = global::Lovelace.Complex.Complex;`). Same for `Rl`/`Int`/`Nat`.
5. **AOT**: every new project is `net10.0` + `<IsAotCompatible>true</IsAotCompatible>`; confirm the Studio Native-AOT publish still succeeds.
6. **Always verify end-to-end** (`.github/copilot-instructions.md` rule #6): run the real host surface — `Lovelace.Run --eval "<expr>"` and the Studio HTTP API (`POST /api/session` → `POST /api/evaluate` → `GET /api/run/{id}`) — and confirm observable output, plus the Studio AOT publish.

---

## 1. Context — what already exists (do not rebuild)

| Project | Contents |
|---|---|
| `Lovelace.Real` | Arbitrary-precision `Real`. Cached `Pi`/`E` (Lazy, `MaxComputationDecimalPlaces`), `PiTo(long)`/`ETo(long)`, `Sqrt`, `Sin(Real[,long])`/`Cos`/`Exp` (special-angle table + `DivideNonPeriodic` Taylor). `CompareTo` has a zero fast-path. |
| `Lovelace.Complex` | `Complex` over `Real` (Re/Im), field `+ - * /`, `Conjugate`, `Magnitude(Squared)`, `Reciprocal`, `Exp(long)`, `Parse`/`ToString`, constants `Zero/One/I/Pi/E`. |
| `Lovelace.Abstractions` | `DType` (now includes `Complex` descriptor), `ArrayValue`, `DenseArray<T>`, plugin contracts. |
| `Lovelace.Dsp` | `ISignal`, `Sequence`, generators `Impulse/Step/Scalar/Sum/Product/Delay/PowerSeries/Cosine/Exponential/Noise`, and `DspMath` (`Convolve`, `PowInt`, `ImpulseResponse`, `StepResponse`, `MovingAverage`, **`Dft` (O(N²), gcd-reduced roots)**). |
| `Lovelace.Suite` | `ValueKind.Complex` + `Value(Complex)`/`AsComplex`, formatter support, and `DspBuiltins` registering `conv`, `dft`, `filter`, `movingavg`, `impulse`, `step`, `cosine`, `delay`, `scale`. |
| `Lovelace.Studio` | DSP is live; `ValueHasher` and `Value.ToString()` already handle `Complex`; Native-AOT publish verified. |

Tests live in `Lovelace.Dsp.Tests` (`DspTests`, `TrigTests`, `FourierTests`) and `Lovelace.Complex.Tests`.

The language is already proven end-to-end:
```
conv([1,1],[1,1])   → [1, 2, 1]
dft([1,0,0,0])      → [1, 1, 1, 1]
cosine(1/4,0,4)     → [1, 0, -1, 0]
```

---

## 2. Task A — add a radix-2 FFT

### 2.1 Core (`Lovelace.Dsp`)

Add `DspMath.Fft` next to `Dft`:

- **Algorithm**: iterative radix-2 decimation-in-time Cooley–Tukey, `O(N log N)`, for `N` a power of two.
- **Signature**: `public static Cplx[] Fft(IReadOnlyList<Cplx> x, long digits = 50)`.
- **Twiddle factors**: precompute `W_N[k] = e^(−j·2π·k/N)` for `k = 0 .. N/2−1` using the **gcd-reduced angle construction** (same as `Dft`) so power-of-2 twiddles hit exact special/sqrt values: `gcd(k, N)` → `angle = Pi * num / den` → `new Cplx(Rl.Cos(angle, digits), -Rl.Sin(angle, digits))`.
- **Bit-reversal permutation**, then the standard butterflies `t = W · x[k+j+h/2]; u = x[k+j]; x[k+j] = u + t; x[k+j+h/2] = u − t`.
- **Non-power-of-2 length**: throw a clear `ArgumentException` (or, if you prefer, fall back to `Dft` — but state the choice explicitly). The language builtin can decide.
- **Correctness contract**: FFT must agree with `Dft` to within a small tolerance at the chosen `digits` (they differ in the *last few digits* because the butterfly summation order differs — this is expected and correct, not a bug). For N = 2, 4 the results should still be **exact**.

### 2.2 Language + Studio

- Register `fft(x)` in `Lovelace.Suite/DspBuiltins.cs` (mirror `dft`).
- Verify e2e: `fft([1,0,0,0])` → `[1, 1, 1, 1]`, and `fft` of a longer power-of-2 signal matches `dft` within tolerance.

### 2.3 Tests

- N = 1, 2, 4: exact (`fft([1,0,0,0])`, `fft([1,1,1,1])`, etc.).
- N = 8, 16, 64: matches `Dft` within tolerance (assert per-element `Rl` equality via `Equals`, or a max-absolute-difference bound).
- `fft` of a real cosine signal shows the expected spectral peaks (magnitude).
- Non-power-of-2 throws (or falls back — per your chosen contract).
- Speedup: (assert nothing about timing in unit tests — that's the benchmark's job).

---

## 3. Task B — the "huge" benchmark project

### 3.1 New project

Create **`dspbench`** (a new `net10.0` console, `IsAotCompatible` optional — it does not need to publish AOT), modeled on the existing `precbench` / `arraybench` conventions: **BenchmarkDotNet** with `[MemoryDiagnoser]` and machine-parseable output, plus a deterministic correctness check before timing.

### 3.2 Scenarios ("more real" + "bigger")

Port the *intent* of the DSPcpp `main.cpp` scenarios (see `dsp-plugin-plan.md`) to `Lovelace.Dsp`, and scale them up:

| # | Scenario | Realism / size |
|---|---|---|
| B1 | **DFT vs FFT** on `cosine(0.05,0,N) + cosine(0.47,0,N)` | N = 1024, 4096, 16384, 65536 |
| B2 | **Convolution** of two long sequences (e.g. a 256-tap kernel over a 10k–100k-sample signal) | linear convolution, `Convolve` |
| B3 | **Difference-equation (IIR) filter** `filter(a, b, n)` with the DSPcpp coefficient sets, n = 10⁴–10⁶ | impulse/step response |
| B4 | **Moving average** over a large noisy/step signal | window sweep, large N |
| B5 | **Noise + moving-average + DFT** (the DSPcpp `p1_question3A` shape: `powerseries + noise → movingavg`) | N ≈ 10⁴ |
| B6 | **End-to-end filtering + spectrum** (the DSPcpp `p2_question1A/B` shape: sum-of-cosines → movingavg/convolution → DFT/FFT → magnitude) | N ≈ 10⁴ |

Use a **fixed, documented precision** (e.g. `digits = 50`) for the transcendental paths via the `digits` overload — do **not** default to the 1000-digit path (naive Taylor at 1000 digits is O(n²) and slow). Report the precision in every result table.

### 3.3 Report

For each scenario emit: **mean time, allocations, and — for B1 — the DFT→FFT speedup ratio** (expect ~`N / log₂N`). Include a **correctness gate** before timing: assert `Fft(x)` equals `Dft(x)` within tolerance for a fixed seed/input, and that convolution/filter outputs match hand-computed or cross-checked expectations (exact where possible).

### 3.4 Anti-regression checks

- A test (or benchmark preamble) that greps `Lovelace.Dsp`, `Lovelace.Complex`, `Lovelace.Real` for `double`/`float`/`System.Numerics.Complex` and fails on a hit — the "no floating point" invariant must be mechanically enforced.
- Re-run the existing `Lovelace.Dsp.Tests` and confirm the Studio AOT publish still succeeds after the benchmark/FFT work (rule #6).

---

## 4. Definition of done

1. `DspMath.Fft` exists, is correct (matches `Dft` within tolerance; exact for N ≤ 4), and is exposed as the `fft` language builtin.
2. `dspbench` runs all B1–B6 scenarios and prints timing/allocation/speedup tables with the documented precision.
3. The correctness gate + no-floating-point grep both pass.
4. E2E: `Lovelace.Run --eval "fft([1,0,0,0])"` (and a bigger power-of-2 signal) and the Studio `/api/evaluate` flow return correct output; Studio Native-AOT publish succeeds.
5. Existing suites stay green (`Lovelace.Dsp.Tests`, `Lovelace.Complex.Tests`, `Lovelace.Suite.Tests`, `Lovelace.Real.Tests`).

---

## 5. Gotchas to remember

- **`Complex` namespace/type collision** → always alias `Cplx` outside `Lovelace.Complex`.
- **`operator /` on irrationals is wrong** (spurious period) → use `DivideNonPeriodic`.
- **Special-angle exactness needs gcd-reduced `Pi * num / den`** angles, matching the `Dft` pattern.
- **`Real` global precision is mutable** (`DisplayDecimalPlaces`/`MaxComputationDecimalPlaces`) and tests race on it in parallel — the pre-existing `setprecision` doctest can flake; don't add to that footgun (don't mutate global precision in the benchmark except via explicit, isolated `WithPrecision`/`digits` scopes).
- **Full 1000-digit transcendentals are slow**; the benchmark and any non-special-angle call should use a `digits` budget (e.g. 50).
