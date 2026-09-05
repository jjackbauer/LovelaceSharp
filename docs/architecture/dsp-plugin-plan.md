# DSP Extension — Plan (DSPcpp → Lovelace, arbitrary precision)

> Status: plan, corrected (no floating point anywhere)
> Scope: bring DSP capabilities (from `jjackbauer/DSPcpp`) into Lovelace as a **Lovelace-native
> extension** built on `Natural`/`Integer`/`Real` — with a Modus platform plugin only as an
> optional HTTP wrapper.
> Sources read in full: the Modus repo (`README`, `Modus.Core`/`Modus.Host` contracts, `PluginBase`,
> `TimerPlugin`, `OrdersFulfillmentPlugin`, messaging + WebApi dispatch + typed-dispatch tests) and
> the DSPcpp repo (all headers, all `.cpp`, `main.cpp`, `README`).

---

## 1. Summary

DSP functions become **Lovelace language builtins** over Lovelace's own numeric tower. There is
**no IEEE floating point** anywhere in the design.

Two hard rules, learned the hard way this session:

1. **`double`/`float`/`System.Numerics.Complex` have no place in Lovelace.** Every value is
   `Natural`, `Integer`, `Real`, or a `Complex` **over `Real`**.
2. **The DSP works through Lovelace's existing extension seam** (`Lovelace.Abstractions`), not
   through Modus.Host's HTTP/DI machinery. Modus needs **no** modification.

---

## 2. The numeric basis

### 2.1 `Real` is configurable precision, not floating point

`Lovelace.Real` already proves the pattern: `Pi(long digits)` (Chudnovsky) and `Sqrt(Real)`
(Newton with progressive precision doubling) compute to *any* requested digit count, with guard
digits, then truncate. `sqrt(2)` returns `1.4142135623730950488…` — as many digits as asked.

The DSP extension uses the **same machinery** for the transcendentals DSP needs. It never calls
`System.Math` for results.

### 2.2 A `Complex` over `Real`

Lovelace has no complex type today, so the DSP extension introduces one as a **domain type**
(exactly the "complex or image type" MOD-004 anticipated in `modus-plugin-design.md`):

```csharp
// proposed domain type, lives in the DSP extension (or Lovelace.Abstractions)
public readonly struct Complex
{
    public Real Re { get; }   // Lovelace.Real.Real — not double
    public Real Im { get; }
    public static Complex operator +(Complex a, Complex b) => new(a.Re + b.Re, a.Im + b.Im);
    public static Complex operator *(Complex a, Complex b) =>
        new(a.Re * b.Re - a.Im * b.Im, a.Re * b.Im + a.Im * b.Re);
    // magnitude/conjugate/division via Real.Sqrt/Real division
}
```

Complex arithmetic is **exact where the inputs are rational**, and configurable-precision where
an operation introduces an irrational (`Sqrt`, `Sin`, `Cos`, `Exp`, `π`).

### 2.3 The one prerequisite: `Real` transcendentals

`Lovelace.Real` currently has `Pi` and `Sqrt` but **not** `Sin`/`Cos`/`Exp`/`Log`. Adding them
(argument reduction mod `2π` at high precision + Taylor series, reusing `Pi`/`Sqrt`) is real work
but fits the existing pattern exactly. It is only needed by the *transcendental* operations below.

| Operation | Transcendental? | Basis |
|---|---|---|
| impulse, step, delay, scalar, sum, product | no | exact `Integer`/`Natural`/`Real` |
| convolution, filter (difference eq), moving-average, power-series | no | **exact** `Real` arithmetic |
| cosine, exponential, DFT | yes (`cos`/`sin`/`exp`/`π`) | `Real` at active precision |

---

## 3. DSPcpp → Lovelace mapping (semantics, with defect fixes)

DSPcpp is a compositional `std::complex<double>` signal model. We port the **model**, over
`Complex`-of-`Real`, and fix the defects rather than reproduce them.

| DSPcpp class | Lovelace builtin | Semantics (corrected) |
|---|---|---|
| `impulse` | `impulse()` | `1` at `n=0`, else `0` |
| `step` | `step()` | `1` for `n≥0`, else `0` |
| `delay` | `delay(x, k)` | `x(n−k)` |
| `scalar` | `scale(x, k)` | `k·x(n)` |
| `sum` | `x + y` / `add(x,y)` | elementwise add |
| `product` | `x * y` / `mul(x,y)` | elementwise multiply |
| `cosine` | `cosine(freq, phase)` | `cos(2π·freq·n + phase)` — phase in **radians** (D6 fixed) |
| `exponential` | `exponential(c)` | `exp(c·n)` at precision |
| `powerSeries` | `powerseries(k, a)` | `k·n·aⁿ` (D1 copy bug moot — value types) |
| `noise` | `noise(scale, disp, seed?)` | seeded, reproducible (D7 fixed) |
| `movingAverage` | `movingavg(x, w)` | exact `w`-sample window (D3 fixed) |
| `convolution` | `conv(x, h)` | standard linear convolution (D9 fixed) |
| `differenceEquation` | `filter(a, b, x)` | IIR/FIR, pure function (D8 fixed) |
| `fourierTransformation` | `dft(x, n)` | forward DFT `e^{−j2πkn/n}` (D4 fixed) |

Defect list (D1–D9) carried over from the DSPcpp source review: `powerSeries` copy loses `k`;
`movingAverage` self-assign + off-by-one window; DFT uses `+j`; `sequence` silent no-op on size
mismatch; `cosine` phase in degrees; `noise` unseeded; `differenceEquation` hidden state;
`convolution` symmetric `[-k,k]` window. All are fixed, documented in `DEVIATIONS.md`.

---

## 4. Architecture

```
Lovelace script:      y = conv(x, h)        # a builtin, computed exactly

  Interpreter → builtin table
       │
  ModusHost / Lovelace.Abstractions seam      (the ONLY interpreter-aware code; already exists)
       │   unwrap Value → DenseArray<Real> / DenseArray<Complex>
       ▼
  DSP extension (Lovelace.Dsp)                (no floating point; no Modus dependency)
       │   pure functions over Real/Complex + typed arrays
       ▼
  wrap back → Value(Vector|Array)
```

The DSP extension depends only on `Lovelace.Abstractions` + the scalar numeric projects
(`Lovelace.Real`, `Lovelace.Integer`, `Lovelace.Natural`). It registers builtins through
`IModusContext.RegisterBuiltin` / `RegisterScalarBuiltin` and, later, kernels — the seam already
specified in `modus-plugin-design.md`.

### 4.1 Project layout

```
Lovelace.Dsp/                  # the extension (Lovelace-native, AOT-compatible)
  Lovelace.Dsp.csproj          # net10.0, IsAotCompatible=true
  Complex.cs                   # Complex over Real (domain type)
  Signals/                     # impulse, step, cosine, exponential, powerseries, noise
  Ops/                         # delay, scale, add, mul, movingavg, conv, filter, dft
  DspPlugin.cs                 # IModusPlugin: Register(IModusContext) → builtins + kernels
  DEVIATIONS.md                # D1–D9 + exact-vs-transcendental notes

Lovelace.Dsp.Tests/            # parity + exactness tests
```

### 4.2 Modus platform plugin (optional, later)

If an HTTP surface is ever wanted, a thin `Plugin.DSP` (Modus.Core) wraps the same core via
`ISyncResponder<SyncRequest<DspRequest>, SyncResponse<DspResponse>>`. This works **in-process
today with no Modus change** (typed dispatch already exists). Exposing it over REST with JSON
signal bodies would require the small, optional `PluginEndpointMapper` enhancement to forward
`request.Payload` — **not** a prerequisite for the DSP itself.

---

## 5. Two-phase delivery

### Phase A — exact DSP (no new math; pure Lovelace scalars)

Everything that is sums/products/shifts of `Real`: `impulse`, `step`, `delay`, `scale`, `add`,
`mul`, `conv`, `filter`, `movingavg`, `powerseries`, `noise` (seeded). These are **exact** and
need nothing beyond the existing `Real`/`Integer`/`Natural`.

### Phase B — configurable-precision transcendentals

`cosine`, `exponential`, `dft`. Prerequisite: add `Real.Sin`/`Cos`/`Exp` (mirror `Pi`/`Sqrt`).
Results are `Real` at the active precision — no IEEE.

---

## 6. Todo plan

**Phase 0 — decisions (this document)**
- [ ] Approve: no floating point; `Complex` over `Real`; Lovelace-native builtins via the extension seam.
- [ ] Approve the exact-first / transcendentals-later split.
- [ ] Decide `Complex` home (`Lovelace.Dsp` vs `Lovelace.Abstractions`).

**Phase 1 — `Real` transcendentals (enables Phase B; do in parallel with Phase A)**
- [ ] `Real.Sin` / `Real.Cos` / `Real.Exp` via argument reduction + series at precision.
- [ ] Unit tests vs high-precision reference values; guard-digit + truncation parity with `Pi`/`Sqrt`.

**Phase 2 — `Complex` over `Real` + DSP core (`Lovelace.Dsp`)**
- [ ] `Complex` (Re/Im over `Real`), arithmetic, conjugate, magnitude, divide.
- [ ] Generators: `impulse`, `step`, `cosine`, `exponential`, `powerseries`, `noise` (seeded).
- [ ] Ops: `delay`, `scale`, `add`, `mul`, `movingavg`, `conv` (linear), `filter` (IIR/FIR), `dft` (forward).
- [ ] Exactness tests: convolution/filter over rational inputs are exact (no guard-digit drift).

**Phase 3 — register as Lovelace builtins**
- [ ] `DspPlugin : IModusPlugin` registers `conv`, `filter`, `dft`, `movingavg`, `cosine`, `exponential`, `powerseries`, `noise`, `delay`, `scale` via `IModusContext`.
- [ ] `Value` ↔ `DenseArray<Real>`/`DenseArray<Complex>` adapter in the core (core owns mapping).
- [ ] Doctested language examples in `Language.md` style (`conv([1,1],[1,1])` etc.).

**Phase 4 — parity harness (semantic, not bit-exact)**
- [ ] Build DSPcpp + JSON driver for the fixed scenario set.
- [ ] Correct cases match within tolerance; deviations (D1–D9) asserted against the *fixed* expectation.

**Phase 5 — optional Modus platform wrapper**
- [ ] Thin `Plugin.DSP` over the same core, typed in-process dispatch (no Modus change).
- [ ] Only if REST is wanted: the small `PluginEndpointMapper` payload-forwarding enhancement.

---

## 7. Risks & open questions

- **`Real` transcendentals are the long pole** for Phase B. They're well-understood but non-trivial;
  Phase A is deliverable and useful without them.
- **Performance**: arbitrary-precision DFT is slow. Acceptable as the exact/correct path; a
  machine-precision fast path, if ever added, must be an explicit opt-in (`DType.F64`) — never the
  default, never `DType.Real`.
- **Two "Modus" concepts**: `Modus.Core`/`Modus.Host` (platform) vs `Lovelace.Abstractions`
  (language seam). The DSP targets the **latter**; the platform is optional.
