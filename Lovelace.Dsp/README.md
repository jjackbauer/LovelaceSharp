# Lovelace.Dsp

Digital signal processing for the LovelaceSharp language, built on Lovelace's own arbitrary-precision
numeric tower (`Natural` → `Integer` → `Real` → `Complex`). The DSP functions are **Lovelace language
builtins** registered through the Modus plugin seam — there is **no IEEE floating point** anywhere in
the design.

---

## The plugin model

`DspPlugin` is a plain `IModusPlugin` — the same seam as any extension package. The core owns the
`Value ↔ payload` mapping; the plugin only ever sees the public scalar types and flat lists of them.

```csharp
var engine = new SuiteEngine();
engine.LoadPlugin(new DspPlugin());   // opt-in, per host
```

- `Lovelace.Suite` knows **nothing** about DSP — a bare engine has no `fft` symbol.
- The CLI hosts (`Lovelace.Run`, `Lovelace.Studio`, `Lovelace.Console`) and `dspbench` opt in.
- Plugins are **compile-time linked** and AOT-compatible (`IsAotCompatible=true`): no reflection,
  no `Assembly.Load`, no runtime plugin discovery.

## Builtins

| Builtin | Signature | Semantics |
|---|---|---|
| `conv` | `conv(x, h)` | Standard linear convolution, `x.Count + h.Count − 1` samples |
| `dft` | `dft(x)` / `dft(x, n)` | Forward DFT; `n` zero-pads (or truncates) the input |
| `fft` | `fft(x)` | Radix-2 Cooley–Tukey FFT (length must be a power of two) |
| `filter` | `filter(a, b, x)` / `filter(a, b, n)` | Difference-equation filter of signal `x` (zero initial state, pure function); scalar `n` returns the impulse response of length `n` |
| `movingavg` | `movingavg(x, w)` | Exact `w`-sample moving average |
| `impulse` | `impulse(n)` | Unit impulse δ[n], `n` samples |
| `step` | `step(n)` | Unit step u[n], `n` samples |
| `cosine` | `cosine(freq, phase, n)` | `cos(2π·freq·n + phase)`, phase in radians |
| `exponential` | `exponential(c, n)` | `exp(c·n)` |
| `powerseries` | `powerseries(k, a, n)` | `k·n·aⁿ` |
| `noise` | `noise(scale, disp, n)` / `noise(scale, disp, seed, n)` | Uniform [0,1) noise, seeded and reproducible (default seed 0); digits at the active precision |
| `delay` | `delay(x, k)` | `x(n − k)` (negative `k` advances) |
| `scale` | `scale(x, k)` | `k·x(n)` |
| `re` / `im` / `conj` | scalar or elementwise over vectors | Bridge complex values back to `Real` |
| `abs` | scalar or elementwise over complex vectors | Magnitude / magnitude spectrum |

```lovelace
fft([1, 0, 0, 0])                    # [1, 1, 1, 1]
conv([1, 1], [1, 1])                 # [1, 2, 1]
dft([1, 2, 3])                       # [6, -1.5 + 0.866…i, -1.5 - 0.866…i]
abs(fft([0, 1, 0, 0]))               # [1, 1, 1, 1] — the magnitude spectrum
filter([1, -0.5], [1], [1, 1, 1, 1]) # [1, 1.5, 1.75, 1.875] — IIR y[n] = x[n] + 0.5·y[n−1]
z = fft([0, 1, 0, 0])[1]; [re(z), im(z), abs(z)]   # [0, -1, 1]
```

## Precision & performance

Lovelace's single precision knob (`setprecision` / the engine's `ComputationDecimalPlaces`) governs
the DSP builtins, with a **plugins-only fast standard**:

- While the engine's knob is untouched, plugin builtins run at a **fast default budget of 30
  computation digits** (a nested `Rl.WithPrecision(30, 15)` AsyncLocal scope in `ModusHost`).
- Raising the knob (`setprecision`) **silently promotes** — plugin results follow it. Promotion only
  ever increases precision.
- Whole-array ops (`conv`, `filter`, `movingavg`) take an `LComplex128` fixed-width fast path
  (38 significant digits, exact decimal) and silently promote to the arbitrary-precision `DspMath`
  on width overflow (`LRealPromoteException`). The fast path never rounds.
- Exact operations (conv/filter/powerseries/movingavg) lose nothing at the fast budget; only
  transcendentals truncate there, and `setprecision` is the escape hatch.

Measured: at the engine's 1000-digit default, `Real.Exp` costs ≈ 100 s per sample — at the fast
budget `exponential(1, 3)` ≈ 0.2 s and `dft([1, 2, 3])` ≈ 0.1 s.

## Numeric basis

- `Lovelace.Complex` — `Complex` over `Real` (Re/Im arbitrary-precision, no `System.Numerics.Complex`),
  plus the fixed-width exact decimal `LComplex64`/`LComplex128` structs used by the fast path.
- Transcendentals use `Real`'s own machinery (`Pi`, `Sqrt`, `Sin`, `Cos`, `Exp`) at the active
  precision — never `System.Math`.
- DFT/FFT twiddles share one gcd-reduced `RootOfUnity`, so rational roots (N = 3, 4, 6, …) are exact.

## Layout

| File | Role |
|---|---|
| `DspPlugin.cs` | The `IModusPlugin`: builtin registration, payload coercion, fast-path dispatch |
| `DspMath.cs` | Whole-array exact algorithms: `Convolve`, `Dft`, `Fft`, `Filter`, `ImpulseResponse`, `StepResponse`, `MovingAverage` |
| `FixedDsp.cs` | The fixed-width (`LComplex64`/`LComplex128`) mirrors with promote-on-overflow |
| `Signals.cs` | Compositable signal model: `Impulse`, `Step`, `Cosine`, `Exponential`, `PowerSeries`, `Noise`, `Delay`, `Scalar`, `Sequence`, `Signal.Sample` |
| `Lovelace.Dsp.Tests` | Exactness, parity, fixed-width, and no-floating-point tests |

Dependencies: `Lovelace.Abstractions` (the plugin contract) + `Lovelace.Complex` → `Lovelace.Real` →
`Lovelace.Integer` → `Lovelace.Natural`. Nothing else.

## Verification

- `Lovelace.Suite.Tests` 412/412, `Lovelace.Dsp.Tests` 61/61, `Lovelace.Real.Tests` 285/285 green.
- `NoFloatingPointTests` mechanically enforces the invariant: no `double`/`float`/
  `System.Numerics.Complex` in `Lovelace.Dsp` or `Lovelace.Complex`.
- The published Native AOT runner (`make runner`) passes the DSP smoke tests.

## Further reading

- `docs/architecture/dsp-plugin-plan.md` — the original extension plan (DSPcpp mapping, D1–D9 fixes)
- `docs/architecture/dsp-plugin-rewire-plan.md` — making DSP a true Modus plugin
- `docs/architecture/dsp-plugin-remediation-plan.md` — the remediation pass (fast budget, filter
  signal form, elementwise accessors, `ScalarResult`, `IFieldKernel`)
- `docs/architecture/dspbench-coherence-plan.md` — precision-model coherence, verified checkboxes
- `docs/architecture/modus-plugin-design.md` — the plugin contract (`IModusContext`,
  `IModusPlugin`, `IFieldKernel`, golden rule)
- `Lovelace.Suite/docs/Language.md` — the language-level DSP surface and doctests
