# dspbench — Fixed-Width DSP Benchmark Results

> Fixed-width DSP benchmark (`dspbench/`): scalar complex arithmetic and whole-array DSP
> workloads over the fixed-width structs `LComplex64` (pair of `LReal64`, 19 significant digits)
> and `LComplex128` (pair of `LReal128`, 38 significant digits), with the arbitrary-precision
> `Complex` class included at a fixed precision knob (18 ≤ 37, so its operators silently dispatch
> to the structs and promote on overflow). A script-level twin (`setprecision`) covers the same
> workload end to end through the language.
>
> The arbitrary-precision ladder (B1–B6) was **dropped** — fixed width is the path a user should
> actually run, and this suite completes in ~6 minutes instead of hours.
>
> Run: `dotnet run -c Release --project dspbench -- --filter "*" --iterationCount 3 --warmupCount 1`

---

## 1. Structure

| Benchmark class | What it measures |
|---|---|
| `ComplexClassP18` / `ComplexClassP37` | the `Complex` **class** at knob 18 / 37 (struct fast path + silent promotion), scalar ops |
| `LComplexStruct64` / `LComplexStruct128` | the fixed **structs**, called raw, scalar ops |
| `FixedConvolveBenchmarks` | 256-tap linear convolution over the structs (`Lovelace.Dsp.FixedDsp.Convolve`) |
| `FixedFilterBenchmarks` | pure-FIR impulse response over the structs (`FixedDsp.ImpulseResponse`) |
| `FixedMovingAverageBenchmarks` | 16-sample moving average over the structs (`FixedDsp.MovingAverage`) |
| `DspScriptBenchmarks` | `setprecision(18/37); conv(1..512, 1..128)` through `SuiteEngine` — script-level e2e |

Workload inputs are sized to fit the fixed widths (small integers, 2⁸ denominators), so the fixed
path never promotes and never rounds. Scalar `Div` uses its own terminating-quotient pair
(`(2.5+1.25i)/(1+2i) = 1−0.75i`).

## 2. Scalar ops (per operation)

| Representation | Add | Sub | Mul | Div |
|---|---|---|---|---|
| `ComplexClassP18` (class, knob 18 → struct64) | 133.8 ns | 140.2 ns | 204.4 ns | 248.4 ns |
| `ComplexClassP37` (class, knob 37 → struct128) | 242.8 ns | 181.6 ns | 1.62 µs | 232.4 ns |
| `LComplexStruct64` (struct, 19 sig) | 47.6 ns | 48.2 ns | 91.8 ns | 168.4 ns |
| `LComplexStruct128` (struct, 38 sig) | 81.9 ns | 182.4 ns | 162.6 ns | 338.4 ns |

## 3. Whole-array workloads

| Workload | N | `Struct64` | `Struct128` | `Class` (knob 18) |
|---|---|---|---|---|
| Convolve (256-tap) | 10 000 | 242.6 ms | 461.5 ms | 951.9 ms |
| Convolve (256-tap) | 100 000 | 2.50 s | 4.66 s | 9.62 s |
| FIR filter (256-tap) | 10 000 | 100.4 ms | 94.7 ms | 398.9 ms |
| FIR filter (256-tap) | 100 000 | 992.9 ms | 911.7 ms | 4.26 s |
| Moving average (w=16) | 10 000 | 8.79 ms | 8.35 ms | 32.6 ms |
| Moving average (w=16) | 100 000 | 88.2 ms | 83.6 ms | 323.4 ms |

Memory (per operation): Convolve `Struct64` **480.73 KB** vs `Class` **1.78 GB** — ~3700× less;
Filter `Struct64` **1.23 MB** vs `Class` **1.03 GB**.

## 4. Script-level e2e (`setprecision(P); conv(1..512, 1..128)`)

| `setprecision` | Mean | Allocated |
|---|---|---|
| 18 | 15.30 ms | 35.41 MB |
| 37 | 15.12 ms | 35.41 MB |

## 5. Findings

1. **Fixed width is the right default.** The whole suite runs in ~6 minutes; the structs execute
   every scenario without promoting or rounding.
2. **The structs are 2–4× faster than the class at the fixed knob** on whole-array workloads, and
   allocate **three orders of magnitude less** — the class form's per-operation
   `TryFromComplex`/`ToComplex` wrapper (string round-trips) is the entire difference. The class's
   value is silent promotion and unbounded width, paid per operation.
3. **Struct128 ≈ 2× Struct64 on arithmetic-bound ops** (convolve: 461.5 vs 242.6 ms), and ~1× on
   memory-bound ops (filter, moving average) — matching the `LReal64`→`LReal128` width cost.
4. **The precision knob works e2e at both levels**: class rows at knob ≤37 take the struct fast
   path (133–248 ns scalar), and the script rows at `setprecision(18/37)` allocate 35 MB — the
   same fixed path, driven from the language.
5. Scalar class overhead over the raw struct is ~100–150 ns per op (the promotion wrapper);
   `Mul` at 37 digits is 1.62 µs because 18-sig × 18-sig promotes from `LComplex64` to
   `LComplex128` inside the dispatch.

## 6. Reproducing

```powershell
# Full fixed-width suite (~6 min):
dotnet run -c Release --project dspbench -- --filter "*" --iterationCount 3 --warmupCount 1

# One scenario:
dotnet run -c Release --project dspbench -- --filter "*Convolve*" --job short
```

> **Build note:** this machine's .NET 10.0.103 SDK has an intermittent MSB4276 workload-resolver
> race under multi-node MSBuild; use `-m:1` for the surrounding `dotnet build`/`test`/`publish`
> commands.
