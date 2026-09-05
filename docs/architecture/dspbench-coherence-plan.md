# DSP Plugin — Coherence Remediation Plan (revised)

> **Status:** implemented — all phases done; full solution builds, all 12 test suites pass (1,278 tests), and `Lovelace.Run --eval "fft([1,0,0,0])"` succeeds under both JIT and Native AOT.
> **Scope:** make the whole DSP feature (`Lovelace.Complex`, `Lovelace.Dsp`, the `fft`/`dft`/…
> builtins, their language integration, and the `dspbench` harness) coherent with the repo's
> established layering, precision model, and conventions (`.github/prompts/codebase-patterns.md`,
> `Lovelace.Abstractions` plugin seam, `precbench` / `precbench.Tests`).
> **Trigger:** the original `dspbench` plan only saw the benchmark scaffolding. The audit showed the
> real incoherence is one layer down — in how DSP was wired into the language. This revision covers
> both layers.

---

## Problem statement

The DSP math itself (`DspMath.Dft/Fft/Convolve/…`, `Complex` arithmetic) is sound. Two things are
not, in order of severity:

1. **Language integration is broken.** `Complex` was added as a `ValueKind`/`DType` but never
   integrated with the scalar/array operation layer, so an `fft(...)` result is *printable but
   unusable*. DSP builtins are hard-wired into the core interpreter instead of going through the
   plugin seam, and they ignore the language's precision knob.
2. **The benchmark scaffolding re-implements what the repo already provides** (assertions, timing,
   signal builders, the no-floating-point grep) and misuses the precision mechanism.

### Layer 1 — language / plugin integration

| # | Where | Problem | Should be |
|---|---|---|---|
| L1 | `Lovelace.Suite/DspBuiltins.cs` + `Interpreter.cs` | DSP builtins are hard-wired via `Interpreter.RegisterBuiltins()` → `DspBuiltins.Register(this)`; every interpreter always loads DSP, bypassing `IModusPlugin`/`ModusHost`/`SuiteEngine.LoadPlugin` | opt-in registration through the plugin seam (or an explicit `SuiteEngine` method), never unconditional |
| L2 | `Lovelace.Abstractions/Modus.cs` | `IModusContext` only offers `RegisterArrayBuiltin` (1-arg `ArrayValue→ArrayValue`) and `RegisterKernel<T: unmanaged>` — too narrow for `conv(x,h)`, `filter(a,b,n)`, `cosine(freq,phase,n)` | a general multi-arg, `Value`-level registration surface (or a Suite-level plugin contract) |
| L3 | `Lovelace.Suite/NumericOps.cs`, `Value.cs` | `Complex` is a `ValueKind` but `NumericOps.Apply/Compare/Negate/IsZero` and `Value.Widen` have no complex arm — arithmetic on an `fft` result throws | decide: extraction builtins to bridge back to `Real`, or first-class complex arithmetic |
| L4 | `Lovelace.Suite/Interpreter.cs` | `abs(x)` has no complex arm; no `re`/`im`/`conj`/`mag`/`angle` builtins; no complex literal in `Tokenizer`/`Parser` | `abs(complex)` + `re`/`im`/`conj`/`mag` builtins (minimum viable surface) |
| L5 | `DspBuiltins.cs`, `DspMath.cs`, `Signals.cs` | DSP hard-codes `digits = 50` (`Precision` const + library defaults), ignoring the `Rl.WithPrecision` scope the engine sets on every evaluation and `setprecision` raises | resolve `Rl.MaxComputationDecimalPlaces` (the AsyncLocal-aware getter), like `Real.Sin/Cos/Sqrt` |
| L6 | `DspBuiltins.cs` vs `DenseArrayComplexTests.cs` | two competing complex-array representations: boxed `DenseArray<Value>` (what `fft` returns) vs homogeneous `DenseArray<Complex>` (what the tests assert) | one representation, or explicitly retire the unused one |
| L7 | `DspBuiltins.cs` | missing generators the library already has: `exponential`, `powerseries`, `noise` | expose them as builtins |

### Layer 2 — benchmark scaffolding

| # | File | Problem | Should be |
|---|---|---|---|
| B1 | `dspbench/CorrectnessGate.cs` | hand-rolled assertions (`AssertEq`/`RequireClose`/`Pow10`) + re-checks of FFT==DFT/conv/filter + the grep | xUnit tests (`precbench` splits BDN from `precbench.Tests`) |
| B2 | `dspbench/Report.cs` | hand-rolled `Stopwatch` + `double` speedup ratio | delete; ratio = `DftBenchmarks` mean ÷ `FftBenchmarks` mean from BDN output |
| B3 | `dspbench/Signals.cs` | `TwoTone` ≈ `SumOfCosines`; `Ramp`/`NoisyStep`/`Kernel256` re-wrap `Dsp` generators | compose `Cosine`/`Noise`/`Step`/`Signal.Sample`/`Sequence` inline in `[GlobalSetup]` |
| B4 | `dspbench/Program.cs` | pins the Pi cache with `Rl.WithPrecision` in the **host** process — BDN runs benchmarks in a separate generated process, so the pin never reaches the timed code | precision set/restored inside `[GlobalSetup]`/`[GlobalCleanup]` (precbench pattern) |
| B5 | `Lovelace.Dsp.Tests/TestPrecision.cs` | `[ModuleInitializer]` mutating the global `Rl.MaxComputationDecimalPlaces` | scoped `Rl.WithPrecision` (AsyncLocal), not a global setter |
| B6 | `DspMath.Fft` | duplicates the gcd-reduced root-of-unity loop from `Dft` | one shared `RootOfUnity` helper |

---

## Non-negotiable constraints (why the fixes must look like this)

- **Layering.** `Lovelace.Abstractions` is the lowest layer; it must not reference `Value` (which
  lives in `Lovelace.Suite`). `Lovelace.Dsp` stays dependency-isolated (references only
  `Lovelace.Complex` → `Real`). The `Value ↔ Complex` bridge belongs *where `Value` is visible*
  (Suite, or a dedicated bridge project), and registration must be opt-in from the host.
- **Precision model.** The single precision knob is the engine's
  `ComputationDecimalPlaces`/`DisplayDecimalPlaces`, delivered per-evaluation via
  `Rl.WithPrecision(...)` (an **AsyncLocal** scope — `Real.cs`). `Rl.MaxComputationDecimalPlaces`
  and `Rl.DisplayDecimalPlaces` *read* that scope. Anything that wants "the active precision" must
  read those getters, never assume a constant.
- **`Pi` is cached once.** `s_pi` is a `Lazy<Real>` computed at `MaxComputationDecimalPlaces` on
  first access (`Real.cs`). The precision of the first `Pi` access is **sticky** process-wide, so any
  precision scheme must pin it deliberately and once, in the right process, before any `Sin`/`Cos`.
- **Conventions.** `codebase-patterns.md` §3 (73-hyphen banners), §5/§15 (no cached `One`/`Zero`),
  §10–§13 (test naming, summaries, assertion style), §14 (save/restore statics).

---

## Decisions to settle first

- **D1 — Complex in the language.** Either (a) *bridged domain type* (keep `Complex` outside the
  widening lattice; add `re`/`im`/`abs`/`conj`/`mag` so users pull values back into `Real`), or
  (b) *first-class* (`NumericOps` complex arms, full arithmetic/comparison). (a) is consistent with
  the `DType.Complex` doc comment ("outside the lattice, constructed explicitly") and is far smaller.
- **D2 — Plugin seam shape.** Whether to widen `IModusContext` with a general builtin registration
  (requires moving that surface above `Abstractions`) or introduce a Suite-level plugin contract and
  let `DspPlugin` implement that. Either way: registration becomes host-opt-in.
- **D3 — Complex array representation.** Route DSP output through `DenseArray<Complex>` end-to-end,
  or retire the unused `DenseArray<Complex>`/`DType.Complex` typed path and treat complex arrays as
  boxed `DenseArray<Value>` with descriptive `DType.Complex` metadata. (Complex kernels are deferred
  regardless: `IArrayKernel<T>` requires `unmanaged` and `Complex` is a class.)

---

## Todo

### Phase 0 — Precision model coherence (unblocks everything else)

- [ ] **P0.1.** In `Lovelace.Dsp`, remove the `digits = 50` defaults on `DspMath.Dft`/`Fft`,
      `Cosine`, and `Exponential`; default them to `Rl.MaxComputationDecimalPlaces` (the pattern
      `Real.Sin/Cos/Sqrt` already use) or require an explicit `digits`.
- [ ] **P0.2.** In `DspBuiltins`, delete `private const long Precision = 50` and pass
      `Rl.MaxComputationDecimalPlaces` (resolves the active AsyncLocal scope) to every
      transcendental call, so `setprecision`/engine precision governs `fft`/`dft`/`cosine`.
- [ ] **P0.3.** Keep the benchmark's fixed budget *at the benchmark*: `dspbench` passes an explicit
      `digits` and pins it in `[GlobalSetup]`/`[GlobalCleanup]` (see Phase 6), not in `Program.cs`.

### Phase 1 — Plugin seam & opt-in registration (L1, L2)

- [ ] **P1.1.** Remove `DspBuiltins.Register(this)` from `Interpreter.RegisterBuiltins()`; the
      interpreter constructor must not load DSP.
- [ ] **P1.2.** Per decision D2, introduce a registration surface that can express multi-arg,
      `Value`-level builtins (`conv`, `filter`, `cosine`, …), and implement `DspPlugin`/`DspBuiltins`
      against it.
- [ ] **P1.3.** Wire the DSP plugin in explicitly at the hosts that want it (`Lovelace.Run`,
      `Lovelace.Studio`) via `SuiteEngine.LoadPlugin(...)` (or the equivalent). Add a
      `SuiteEngine`-level test that an engine *without* the plugin has no `fft` symbol.
- [ ] **P1.4.** Keep the `Value ↔ Complex` mapping in the bridge (not in `Lovelace.Dsp`), and route
      it through `TypedArrayAdapter` rather than hand-rolled `ToComplexArray`/`FromComplexArray`.

### Phase 2 — Complex language surface (L3, L4, L7)

- [ ] **P2.1.** Per decision D1, add the bridge builtins: `re(x)`, `im(x)`, `conj(x)`, `mag(x)` (or
      extend `abs`), and `abs(x)` for `ValueKind.Complex` (returns `Complex.Magnitude`, a `Real`).
- [ ] **P2.2.** Register the missing generators as builtins: `exponential(c, n)`, `powerseries(k, a, n)`,
      `noise(scale, disp, seed?, n)` (matching the `Signals.cs` types the benchmark already drives).
- [ ] **P2.3.** Give `NumericOps`/`Value` an explicit, clear failure for complex in
      `Apply`/`Compare`/`Negate`/`IsZero` (a message naming `re`/`im`/`abs` as the escape hatch),
      unless D1 chooses full arithmetic — in which case add the complex arms.
- [ ] **P2.4.** Decide and document whether the grammar gets a complex literal; if not, note that
      `Complex.Parse` is library-only and add a round-trip test (`Parse(ToString())`) under
      `Lovelace.Complex.Tests`.

### Phase 3 — Representation consistency (L6, D3)

- [ ] **P3.1.** Resolve decision D3. If boxed: update `DenseArrayComplexTests` to assert the
      `DenseArray<Value>` + inferred-`DType.Complex` shape production actually emits, and document
      `DType.Complex` as descriptive metadata. If typed: add a `Value`/`ArrayValue` path that holds
      `DenseArray<Complex>` and make `fft`/`dft` produce it end-to-end.
- [ ] **P3.2.** Make `fft`/`dft` output survive array reductions (`sum`, `mean`, `norm`, `dot`,
      `matmul`) either by supporting complex there or by a clear, actionable error (ties to P2.3).

### Phase 4 — DSP core de-duplication & polish

- [ ] **P4.1.** Extract `DspMath.RootOfUnity(long k, long n, long digits)` (gcd → `Pi·num/den` →
      `Cos − i·Sin`) and call it from both `Dft` and `Fft`; delete the duplicated loops.
- [ ] **P4.2.** De-duplicate `ImpulseResponse`/`StepResponse` (near-identical IIR loop; differ only
      in input seeding) into one shared difference-equation driver.
- [ ] **P4.3.** Move `MovingAverage`'s `Window <= 0` validation from `Get` to construction (or a
      `record` with a validating body), and hoist the per-sample window scaling.
- [ ] **P4.4.** Add `Complex.Magnitude(long digits)` to match `Exp(long)` and the "honor active
      precision" convention.
- [ ] **P4.5.** Replace cached identities with allocating properties per §5/§15:
      `Complex.Zero`/`One`/`I` → `=> new(...)`; drop shared `s_zero`/`s_one` and `DspUtil.Zero`
      (allocate fresh, or document immutability justifies the exception and get sign-off).
- [ ] **P4.6.** Improve `Signal.Sample` range validation to report the `> int.MaxValue` overflow case
      with a message instead of a bare `OverflowException`.

### Phase 5 — Test conformance (§10–§15) — covers **both** test projects

- [ ] **P5.1.** Rename tests to `MethodName_GivenScenario_ExpectedResult` in `Lovelace.Dsp.Tests`
      (`FftTests`, `DspTests`, `FourierTests`, `TrigTests`) **and** `Lovelace.Complex.Tests`
      (`ComplexTests`, `DenseArrayComplexTests`).
- [ ] **P5.2.** Add class XML `<summary>` referencing the type/method under test; use the 73-hyphen
      section banners; confirm no `using Xunit;` (rely on the global `<Using Include="Xunit" />`).
- [ ] **P5.3.** Replace `Lovelace.Dsp.Tests/TestPrecision.cs`'s `[ModuleInitializer]` with a scoped
      approach that keeps the `Lazy` Pi cache small **and** is race-free:
      - use `Rl.WithPrecision` (AsyncLocal), **not** `precbench.Tests.WithPrecision` (which mutates
        the global statics and would race across the five parallel test classes), and
      - ensure the first `Pi` access happens under a reduced scope (an assembly fixture that pins it
        once, or an explicit pin in a single collection fixture) so the suite doesn't fall back to a
        1000-digit cache.

### Phase 6 — `dspbench` → clean BenchmarkDotNet project (mirror `precbench`)

- [ ] **P6.1.** Delete `dspbench/Report.cs`. The speedup ratio becomes `DftBenchmarks` mean ÷
      `FftBenchmarks` mean from BDN output. Do **not** reference `Lovelace.Suite/Timing.cs` (it is a
      `TimeSpan` formatter, not a timing harness, and would drag the interpreter into a benchmark).
- [ ] **P6.2.** Delete `dspbench/CorrectnessGate.cs`. Correctness relocates to tests (P6.5/P6.6).
- [ ] **P6.3.** Delete `dspbench/Signals.cs`; build inputs from `Cosine`/`Noise`/`Step`/
      `Signal.Sample`/`Sequence` inline in each `[GlobalSetup]`.
- [ ] **P6.4.** Rewrite `dspbench/Program.cs` to the `precbench` shape: `BenchmarkSwitcher.FromAssembly(...).Run(args, config)`
      only, `BuildTimeout` raised for the cold deterministic rebuild; drop the custom
      `--benchmark`/`--gate-only` flags (BDN has `--filter`/`--job`).
- [ ] **P6.5.** Add precision set/restore (`Rl.MaxComputationDecimalPlaces` + `DisplayDecimalPlaces`)
      to each benchmark class's `[GlobalSetup]`/`[GlobalCleanup]` (the `precbench` pattern), so the
      BDN **process** runs at the reduced budget — the pin must not live in `Program.cs`.
- [ ] **P6.6.** Confirm correctness coverage in `Lovelace.Dsp.Tests`; **add the missing**
      `Convolve([1,2,3],[1,2,3]) = [1,4,10,12,9]` case (currently only in the gate, not in
      `DspTests`), keep the FFT==DFT and FIR/IIR hand-checks.
- [ ] **P6.7.** Move the no-floating-point grep into a `[Fact]` under `Lovelace.Dsp.Tests`,
      scanning `Lovelace.Dsp`/`Lovelace.Complex`/`Lovelace.Real` with the same patterns (strict
      `double`/`float`/`System.Numerics.Complex` for Dsp/Complex; `System.Numerics.Complex` +
      `Math.Cos/Sin/Exp/Tan` + `Random.NextDouble` for Real), skipping `obj`/`bin`.
- [ ] **P6.8.** Regenerate `dspbench-report.md` from the new BDN means (the single-shot speedup and
      gate sections are orphaned by P6.1/P6.2) and commit it alongside `precbench-report.md`.

---

## Acceptance criteria

1. No `Interpreter` instance loads DSP unless the host opts in; `Lovelace.Run`/`Lovelace.Studio`
   still expose `fft`/`dft`/… through the seam.
2. `fft([1,0,0,0])` returns a value the language can operate on: `abs`/`re`/`im`/`conj`/`mag` work,
   and complex arrays either reduce or fail with an actionable message.
3. `setprecision`/engine precision governs the DSP builtins (no hard-coded `50` in the library or
   the builtins).
4. `Dft` and `Fft` share one `RootOfUnity`; `ImpulseResponse`/`StepResponse` share one driver.
5. Precision is scoped with `Rl.WithPrecision` (AsyncLocal) or a pinned fixture — never a global
   setter in parallel tests, never a `[ModuleInitializer]`.
6. `dspbench` contains only `Program.cs` + `Benchmarks.cs`; no hand-rolled timing/assertions/grep/
   signal builders; precision pinned in `[GlobalSetup]`/`[GlobalCleanup]`.
7. Test naming/style conforms to `codebase-patterns.md` §10–§15 across `Lovelace.Dsp.Tests` **and**
   `Lovelace.Complex.Tests`.
8. All 12 test suites stay green; `Lovelace.Run --eval "fft([1,0,0,0])"` and the Studio AOT publish
   still succeed.

---

## Suggested order

1. **Phase 0** — precision model (small, unblocks the benchmark and makes builtins coherent).
2. **Phase 1 + Phase 4** — de-wire the interpreter and clean the DSP core (highest leverage, low risk).
3. **Phase 2 + Phase 3** — make `Complex` usable and resolve the representation (the seamless-experience work).
4. **Phase 5** — test conformance across both test projects.
5. **Phase 6** — the `dspbench` cleanup, last, once the library and language are coherent.
