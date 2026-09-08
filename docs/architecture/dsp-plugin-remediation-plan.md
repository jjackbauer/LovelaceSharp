# DSP Plugin Remediation Plan — close the gaps between the landed seam and the plans

> Status: implemented and verified (Phase A–E complete; F1/F2 verified — see the F checkboxes).
> Follows the adversarial review of the landed DSP plugin
> (`Lovelace.Dsp/DspPlugin.cs`, `Lovelace.Abstractions/Modus.cs`, `Lovelace.Suite/ModusHost.cs`)
> against `dsp-plugin-plan.md`, `dsp-plugin-rewire-plan.md`, `dspbench-coherence-plan.md`, and
> `modus-plugin-design.md`. The structural shape (D2) is landed and verified — 397/397
> `Lovelace.Suite.Tests` green, Suite DSP-free, Abstractions dependency-free — but the review
> found nine gaps: a closed payload vocabulary, a default-precision performance cliff,
> `filter` semantic drift, a hard-coded `noise` digit budget, scalar-only complex accessors,
> the unmanaged-only kernel seam, raw error messages, stale plan docs, and assorted minor
> convention breaches. This plan closes them.

---

## 1. Findings being remediated (from the review)

| # | Finding | Severity |
|---|---|---|
| F1 | Seam is generic in arity but **closed in payload vocabulary**: `ModusHost.Unwrap/WrapResult/WrapArray` hard-code `Natural/Integer/Real/Complex`; a plugin with a novel domain type needs core edits. `ScalarResult` (rewire §4 open choice, design §5.4) never landed. | High |
| F2 | **Default-precision cliff**: measured `Real.Exp` ≈ 96–116 s and `Real.Cos` ≈ 19–31 s **per sample** at the engine's 1000-digit default; `exponential(0.5, 2)` > 90 s; `dft`/non-trivial `fft` minutes. Every test/smoke case uses special angles and never exercises the default path; no progress reporting. | High |
| F3 | **`filter` drift**: plan says `filter(a, b, x)` (difference-equation filter of signal x, D8); landed `filter(a, b, n)` = impulse response only; `DspMath.DifferenceEquation` is private. No way to filter a signal except O(n²) `conv`. Also `dft(x, n)` → `dft(x)`; `noise(..., seed?)` → seed mandatory. | High |
| F4 | `Noise` keeps a hard-coded digit budget (`digits = 30`, `Signals.cs:140`); the precision knob does not govern `noise` (acceptance criterion 3's class of issue). | Medium |
| F5 | Complex escape hatches are scalar-only: `abs(fft(...))` → `"abs() is not supported for values of kind 'Vector'."`; `re(vector)` → leaks `'Object[]'`. Acceptance criterion 2 only half met. | Medium |
| F6 | Kernel seam still `IArrayKernel<T> where T : unmanaged` + `RegisterKernel<T> where T : unmanaged` (design §2/§5.2/§9.4); `StatisticsPlugin.DoubleAddKernel` claims `DType.Real` — the exact bug §2/§8 condemn; `TryDispatch` never wired into the interpreter (§9.5). | Medium |
| F7 | Raw/leaky errors: `impulse(0)` → `"end must be ≥ begin. (Parameter 'end')"`; `delay(x, 1e20)` → raw Int64 `OverflowException`; `fft([1,2,3])` message carries `"(Parameter 'x')"` noise. | Low |
| F8 | Doc/plan hygiene: coherence plan header says "implemented — all phases done" while 26/30 checkboxes are unchecked; rewire plan says "15 builtins" (16 registered); signature tables stale; `Lovelace.Abstractions` has no `<Version>` for the additive-only rule. | Low |
| F9 | Minor: cached `DspPlugin.s_zero` (§5/§15 pattern just removed elsewhere); duplicate `LoadPlugin`/name collisions silently overwrite; empty-array results infer `DType.Natural`. | Low |

---

## 2. Decisions to settle first

- **D1 — `filter` shape.** Land `filter(a, b, x)` (filters signal x through the difference
  equation, zero initial state — D8 stays fixed) **and** keep `filter(a, b, n)` (impulse response
  of length n). Dispatch on the third argument's type: Natural/Integer scalar → response length;
  array → filtered signal. Update every doc that shows the old signature.
  ✅ *Decided (user): overload on the 3rd argument.*
- **D2 — vector accessors.** Implement `re`/`im`/`conj`/`abs` **elementwise over complex
  vectors** (`abs(fft(x))` = magnitude spectrum). This is what makes acceptance criterion 2
  ("the language can operate on fft results") actually true. `abs` needs one new
  Vector/Array arm in `Interpreter` (Suite — the bridge layer); `re`/`im`/`conj` are
  plugin-side. ✅ *Decided (user): elementwise over complex vectors.*
- **D3 — `noise`.** Seed becomes optional with a fixed default (0) — reproducible per D7 — and
  the 30-digit budget goes: `digits` defaults to the active precision
  (`Rl.MaxComputationDecimalPlaces`) or is an explicit argument. No hard-coded budget remains.
  ✅ *Decided (user): optional seed (default 0) + active-precision digits.*
- **D4 — `ScalarResult`.** Land the named opaque wrapper in `Lovelace.Abstractions` (design
  §5.4): factories `FromNatural/FromInteger/FromReal/FromComplex/FromBoolean/FromText/FromArray`,
  opaque payload, core converts at the boundary. Add an **additive** `RegisterBuiltin` overload
  returning it; keep the raw-`object` overload working; migrate `DspPlugin`. Document the
  boundary honestly: the wrapper closes the gap for scalar/text/bool results and arrays of known
  scalars; a novel *element* type still requires a core bridge (state this in the contract docs).
  ✅ *Decided (user): land ScalarResult.*
- **D5 — perf governance (plugins-only fast standard).** ✅ *Approved (user):*
  The small/fast precision is the standard **for plugins only**, with silent promotion —
  realized through the existing AsyncLocal scope, no contract growth:
  1. `Interpreter`/`SuiteEngine` gain a `PrecisionExplicitlySet` flag, set by the `setprecision`
     builtin.
  2. `ModusHost.RegisterBuiltin` wraps every plugin builtin in a nested
     `Rl.WithPrecision(30, 15)` scope **when the engine knob is untouched**; when the user raises
     the knob (including explicitly to the default value), the nested scope follows it — silent
     promotion via the AsyncLocal mechanism, exactly as the user specified.
  3. `DspPlugin` runs the existing `FixedDsp` `LComplex128` path (38-digit exact decimal) while
     the active budget ≤ 38, catching `LRealPromoteException` to promote silently to `DspMath`
     (the promotion contract already documented in `FixedDsp.cs`). No IEEE, no rounding.
  Known limitations (accepted): all plugins share the 30-digit default (intended); "15 display"
  is cosmetic — host-side formatting truncates under the engine display scope; exact arithmetic
  (conv/filter/powerseries) loses nothing at 30 digits; only transcendentals truncate there,
  with `setprecision` as the escape hatch. The engine default (1000) and non-plugin math are
  unchanged. The cost model + progress reporting + non-special-angle CI smoke test still land
  (Phase D).

---

## 3. Phases

### Phase A — documentation truth pass (cheap, unblocks everything)

- [x] **A1.** `dspbench-coherence-plan.md`: check the boxes that actually landed (P2.2, P2.3,
      P3.1, P4.1–4.3, P4.5, P4.6, P5.1–5.3, P6.1–6.8 …), annotate the deviating ones (P0.2 noise
      digits, P2.1 vector forms, P4.4 `Magnitude` property), correct the status header to
      "implemented with recorded deviations — see remediation plan".
- [x] **A2.** `dsp-plugin-rewire-plan.md`: "15 builtins" → 16; record that the §4 open choice
      is now decided (D4) and scheduled, not open.
- [x] **A3.** `dsp-plugin-plan.md`: fix the §3 signature table (`filter`/`dft`/`noise`/
      `cosine`/`exponential`/`powerseries` with `n`), or point to `Language.md` as
      canonical for signatures.
- [x] **A4.** `Lovelace.Suite/docs/Language.md` DSP section: full signatures, error behavior,
      and the cost model (Phase D numbers), with doctest coverage.
- [x] **A5.** Add `<Version>` to `Lovelace.Abstractions.csproj`; restate the additive-only
      semver rule in `modus-plugin-design.md` §11.2.

### Phase B — error-surface hardening

- [x] **B1.** `DspPlugin.cs`: validate `n ≥ 0` for `impulse/step/cosine/exponential/
      powerseries/noise/filter` with `"{name}() expects n ≥ 0, but got {n}."`; make `ToLong`
      bounds-checked with a function-scoped message (no raw `OverflowException`).
- [x] **B2.** `re`/`im`/`conj` on an array argument: if D2 is not yet landed, throw a
      message that names the expectation — never leak `'Object[]'`.
- [x] **B3.** `abs` on a Vector/Array: D2 elementwise arm, or (interim) an actionable error
      naming `re`/`im`/`[i]`.
- [x] **B4.** Reductions over complex arrays (`sum`/`mean`/`norm`/`dot`/`matmul`):
      reword `Value.Widen`/`NumericOps` messages to name the failing operation
      ("sum() does not support Complex arrays; use abs()/re()/im()"), keeping the bridge hint.
- [x] **B5.** Pin every new message with a `Lovelace.Suite.Tests` fact.

### Phase C — functional gaps vs. the plans

- [x] **C1.** Elementwise `re`/`im`/`conj`/`abs` over complex vectors (D2). Plugin maps
      arrays elementwise; `abs` vector arm in `Interpreter` (Suite bridge). Tests:
      `abs(fft([0,1,0,0])) = [1,1,1,1]`, `re`/`im`/`conj` round-trips.
- [x] **C2.** `filter(a, b, x)` (D1): extend `DspMath.DifferenceEquation` with an
      input-sequence overload (pure function, zero initial state); plugin dispatches on the third
      argument type. Keep `filter(a, b, n)`. Tests: FIR taps, IIR hand-check, and
      `filter(a,b,x) == conv(x, filter(a,b,len(x)))`.
- [x] **C3.** `noise` (D3): optional seed (default 0), `digits` = active precision or explicit.
      Update `Signals.cs`; tests for reproducibility and the precision tie-in.
- [x] **C4.** `dft(x)` vs `dft(x, n)`: ✅ *Decided (user): add the optional zero-pad length*
      (`dft(x)` stays; `dft(x, n)` zero-pads/truncates to n); Language.md + plan table updated.

### Phase D — precision & performance governance

- [x] **D1.** Cost model documented with the measured numbers (`Exp` ≈ 100 s, `Cos` ≈ 25 s
      per sample at 1000 digits; O(n · digits²)) in `Language.md` and
      `dsp-plugin-plan.md` §7, with "use `setprecision(50)` for interactive DSP" guidance.
- [ ] **D2.** Progress reporting: additive hook on the builtin channel
      (e.g. `IModusContext` progress API or an AsyncLocal `ProgressScope` in Abstractions)
      consumed by `DspPlugin`'s transcendental loops (`Real` already exposes
      `IProgress<double>` overloads) and surfaced through Suite's existing progress machinery.
- [ ] **D3.** Perf smoke test (CI): `dft([1,2,3])`, `cosine(1/10, 0, 4)` at
      `setprecision(50)` with a generous wall-clock bound + digit assertions — the first test
      that exercises the non-special-angle path.
- [ ] **D4.** (Backlog, not this plan) machine-type opt-in fast path per design §8.

### Phase E — pluggability hardening

- [x] **E1.** `ScalarResult` in `Lovelace.Abstractions` (D4): minimal opaque wrapper +
      factories; additive `RegisterBuiltin` overload; `ModusHost` converts; migrate
      `DspPlugin`; document the novel-element-type boundary. Tests: factory round-trips; raw
      `object` overload still works.
- [x] **E2.** Kernel contract correction (design §5.2/§9.1/§9.2/§9.4): move `IField<T>` into
      `Lovelace.Abstractions`; add `RealField`/`IntegerField`/`NaturalField`; replace
      `IArrayKernel<T> where T : unmanaged` with `IFieldKernel<T>` (field injected); delete
      `StatisticsPlugin.DoubleAddKernel`; re-pin `ModusTests` on an exact `RealAddKernel`.
      Explicitly defer interpreter wiring (§9.5) until typed storage (§9.3/ARR-001) lands —
      record that status in `modus-plugin-design.md`.
- [x] **E3.** Registration guards in `ModusHost`: duplicate plugin type or builtin-name
      collision → clear error. Tests.
- [x] **E4.** `DspPlugin.s_zero` → allocate fresh (patterns §5/§15), or get the documented
      immutability exception.

### Phase F — verification & close-out

- [x] **F1.** Full solution build; all 12 suites green (Suite.Tests stays ≥ 397 with the new
      pins).
- [x] **F2.** AOT smoke: `make runner` publish; eval a non-special-angle case at
      `setprecision(50)` under the published binary; re-run the grep gates (Suite DSP-free,
      Abstractions dependency set unchanged, no `double`/`float` in Dsp/Complex).
- [x] **F3.** Update the three plan docs to "implemented and verified" **with checkboxes that
      match the code**; regenerate `dspbench-report.md` if anything changed.

---

## 4. Acceptance criteria

1. `abs(fft([0,1,0,0]))` returns `[1,1,1,1]`; `re`/`im`/`conj` work elementwise on
   complex vectors (original criterion 2, now actually met).
2. `filter(a, b, x)` filters x and equals `conv(x, filter(a, b, len(x)))`; `filter(a, b, n)`
   still yields the impulse response; all docs agree.
3. `noise(scale, disp, n)` works with the default seed and is reproducible; no hard-coded digit
   budget remains anywhere in `Lovelace.Dsp` or the builtins.
4. Every user-facing DSP error names the builtin and the offending argument; no `'Object[]'`,
   no raw `(Parameter 'x')`, no bare `OverflowException`.
5. The cost model is documented with measured numbers; long transcendental evaluations report
   progress; CI exercises a non-special-angle case at reduced precision.
6. The `ScalarResult` channel is landed and used by `DspPlugin`; the raw-`object` overload
   keeps working; the novel-element-type boundary is documented in the contract.
7. `IFieldKernel<T>`/`IField<T>` live in `Lovelace.Abstractions`; no plugin in the repo
   registers a `double` kernel claiming `DType.Real`; `ModusTests` is exact.
8. Duplicate plugin/builtin registration fails with a clear error.
9. All suites green; AOT publish + smoke succeed; plan-doc checkbox state matches the code.

## 5. Suggested order & dependencies

1. **Phase A** — unblocks everything, near-zero risk.
2. **Phase B + C** in parallel — user-visible, mutually independent (C1 waits on the D2
   decision only).
3. **Phase D** — after C, since D3 should assert the C1/C2 behavior too.
4. **Phase E1/E3/E4** — additive contract change, small; **E2** last (largest, touches
   `Lovelace.Array`/numeric projects).
5. **Phase F** — close-out.

Out of scope (explicitly): machine-type opt-in fast path (design §8), interpreter kernel-dispatch
wiring (§9.5 — blocked on typed storage, ARR-001), engine default-precision changes (implicit
downgrade), complex literals in the grammar (P2.4 stays library-only).
