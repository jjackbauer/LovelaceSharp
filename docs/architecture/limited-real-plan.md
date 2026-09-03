# LimitedReal (LReal64 / LReal128) — Implementation Plan

> Goal: add two fixed-width, hardware-friendly, **exact-decimal** struct variants — `LReal64` (19 significant digits, 64-bit significand) and `LReal128` (38 significant digits, 128-bit significand) — benchmark them against the current arbitrary-precision class `Real`, and, if faster, use them transparently when precision is low. The tier names MUST NOT leak into the script language.

> Companion: `docs/architecture/typed-array-benchmark-baseline.md` (scalar `Real` add = 537 ns/356 B, mul = 1080 ns/1.8 KB — the object graph is the bottleneck).

## 0. Locked decisions

| # | Decision | Locked answer |
|---|---|---|
| D-1 | Precision trigger | **Keep the existing fractional knobs.** Map significant = fractional + 1 (on [1,10)). Dispatch on `MaxComputationDecimalPlaces`: **≤ 18 → LReal64** (19 sig), **19–37 → LReal128** (38 sig), **≥ 38 → BigReal**. This is a heuristic — values with many integer digits or long periods may exceed the tier and promote (D-3), so exactness is preserved regardless. |
| D-2 | Default precision | **19 significant digits** → default `MaxComputationDecimalPlaces = 18` (and `DisplayDecimalPlaces = 18`), so LReal64 is the default tier. (Changes today's default of 1000.) |
| D-3 | Overflow / period-overflow | **Promote to BigReal silently.** Exactness is never dropped; non-representable results widen. |
| D-4 | `sqrt`/`pi` | **Reimplement low-precision sqrt/pi** on the structs, and **port the existing `RealSqrt*`/`RealPi*` test suites** to the new types to verify them. |
| D-5 | `Real` shape | **Do NOT de-inherit/rewrite `Real`.** The class `Real` stays as-is (the arbitrary-precision fallback). LReal64/LReal128 are **new** `readonly struct` types. |
| D-6 | Dispatch scope | **Language number abstraction behind `Value`/`NumericOps`.** Scalars AND arrays get the fast path; the class `Real` remains untouched as the fallback arm. |

**Consequence of D-5 + D-6:** the language's numeric "Real" slot becomes an internal tagged union { LReal64 | LReal128 | Real-class }, introduced at the `Value`/`NumericOps` layer. The class `Real` keeps its inheritance and API unchanged; the union is what `Value` actually holds. `setprecision` stays the public knob and silently selects the tier.

## Stage 0 — Baseline (benchmark the current class Real)

- [ ] Scaffold `realbench` (scratch console project, Stopwatch harness like `arraybench`): benchmark class `Real` at P8/P16/P19/P38/P40 over add, sub, mul, div, sqrt, parse, tostring — with allocations.
- [ ] Record the before-baseline. (Partial data exists: add 537 ns/356 B, mul 1080 ns/1.8 KB at P16 — `arraybench`.)
- [ ] **Exit:** a reproducible per-op table at each precision tier.
## Stage 1 — LReal64 (19 significant digits, 64-bit significand)

- [ ] Define `LReal64` `readonly struct`: `ulong significand` + `int exponent` + `int periodStart` + `short periodLength` + sign. (significand = decimal digits as an integer; value = significand × 10^exponent.)
- [ ] Implement exact `Parse` (string → significand+exponent, incl. periodic `0.(3)`, `0.1(6)`).
- [ ] Implement exact `Add/Subtract` (align exponents; one `Math.BigMul`/carry where needed).
- [ ] Implement exact `Multiply` (one `Math.BigMul` → 128-bit intermediate → normalize to 19 digits).
- [ ] Implement exact `Divide` with remainder-tracked period detection (mirror `NumericOps.DivideNatural`).
- [ ] Implement `CompareTo/Equals/GetHashCode` (exact, no conversion).
- [ ] Implement `ToString` matching class `Real` (`0.(3)` notation, `DisplayDecimalPlaces`).
- [ ] Implement **overflow/period-overflow detection**: any result needing >19 significant digits or a period >19 digits returns a 'promote' signal instead of rounding.
- [ ] Implement low-precision `Sqrt` (and later `Pi`) per D-4.
- [ ] Unit tests — **exactness parity**: randomized cross-check of add/sub/mul/div/parse/tostring against class `Real`; edges: 0, negative, `1e400`, max 19-digit, `1/97` (period 96 → promote). Port `RealSqrt*` tests.
- [ ] **Exit:** LReal64 matches class `Real` within its 19-digit envelope; promotion fires correctly outside it.
## Stage 2 — LReal128 (38 significant digits, 128-bit significand)

- [ ] Define `LReal128` `readonly struct` with `UInt128 significand` + exponent + period + sign.
- [ ] Reuse the Stage-1 algorithms parameterized by width; multiply via schoolbook/Karatsuba over `Math.BigMul`.
- [ ] Port the same unit + parity tests at 38 digits; promotion fires at >38 digits or period >38.
- [ ] **Exit:** same exactness contract at the wider tier.
## Stage 3 — Language number abstraction (dispatch behind Value/NumericOps)

- [ ] Introduce the internal number union type — { LReal64 | LReal128 | `Real`-class } — as the real slot the language actually carries (name TBD; internal).
- [ ] Wire `Value`/`ValueKind.Real` to hold the union; keep `AsReal()` behavior for the fallback arm (the class `Real` is untouched).
- [ ] Update `NumericOps` to dispatch arithmetic by precision (D-1) and auto-promote on overflow/period-overflow (D-3).
- [ ] Update `ValueField`/`Plotting` call sites that use `Rl` directly to go through the union (observable behavior unchanged).
- [ ] Dispatch rule: `MaxComputationDecimalPlaces ≤ 18 → LReal64; 19–37 → LReal128; ≥ 38 → BigReal` — with promotion as the safety net.
- [ ] **No-leak gate:** `Lovelace.Suite.Tests`, `Lovelace.Real.Tests`, `Integer/Natural.Tests`, and `Language.md` doctests stay green **unchanged**.
- [ ] **Exit:** the script language, REPL, Studio, and JSON API show one `Real` type; no `LReal*` name appears anywhere.
## Stage 4 — Benchmark + threshold gate

- [ ] Run `realbench` before/after: class-Real vs LReal64 vs LReal128 vs a `double` floor (add/sub/mul/div/sqrt, scalar + 1M-element array loop).
- [ ] **Gate:** enable the dispatch only if the structs are measurably faster than class `Real` at their own tiers (expected: LReal64 ~5–15 ns vs 537 ns; LReal128 ~2–4× LReal64).
- [ ] Confirm the D-1 thresholds (18 / 37) with real numbers; adjust if the fractional→significant mapping misbehaves at scale.
- [ ] Update `docs/architecture/typed-array-benchmark-baseline.md` and this plan with results and chosen thresholds.
- [ ] **Exit:** a benchmark-grounded default (19 sig / LReal64) and threshold table.
## Stage 5 — Array/kernel integration (after the array redesign)

- [ ] Slot LReal64/LReal128 into the homogeneous `DenseArray<Real>` kernel layer so the struct significand enables contiguous iteration and (where possible) SIMD.
- [ ] Confirm no tier name appears in the script language, AST, interpreter, Studio, REPL, or JSON API surface.
## Risks

- **Broad rewiring surface.** D-5/D-6 avoid touching `Real` itself, but `Value`/`NumericOps`/`ValueField`/`Plotting` all hold `Rl`-typed values; every one of those call sites must move to the union without changing observable behavior. This is the new main risk.
- **Exactness parity.** Fixed-width period detection is the classic off-by-one bug source (`0.1(6)` vs `0.(3)`); the randomized cross-check against class `Real` is mandatory.
- **Thread-safe dispatch.** `MaxComputationDecimalPlaces` is global/`AsyncLocal`; the tier choice must read the *effective* precision correctly under async/parallel (`Sqrt`/`Pi` parallelize).
- **`Plotting.cs`** relies on arbitrary-precision `Real` for exact bounds/ticks — verify it still behaves through the union (it may need to force the BigReal arm).
- **Scalar `Value` still boxes** one wrapper per number; this plan removes `Real`'s *internal* allocations, not the `Value` box — that is the array redesign's job.
## Non-goals

- Do **not** adopt IEEE-754 rounding — exactness is preserved; non-representable results promote to BigReal.
- Do **not** change the script language surface, `setprecision` semantics (beyond D-1's fractional→significant mapping), or the `ValueKind` names.
- Do **not** expose `LReal64`/`LReal128` to the language, Studio, REPL, or API.
- Do **not** de-inherit or rewrite the class `Real` (D-5) — it remains the arbitrary-precision fallback.
- Do **not** make 128-bit the default — LReal64 (19 sig) is the default tier.
- Do **not** fold this into the array redesign — separate, parallel project the array kernel layer later consumes.
## Implementation status (final for this session)

**Both tiers are DONE and verified, and the dispatch is wired into the language.**

- `Lovelace.Real/LReal64.cs` — 19-digit struct (+ `LRealPromoteException`). `LReal64Tests.cs`: **58 tests green**.
- `Lovelace.Real/LReal128.cs` — 38-digit struct + internal `UInt256` (4×64-bit limbs) for the 256-bit multiply intermediate. `LReal128Tests.cs`: **26 tests green**.
- `Lovelace.Suite/NumericOps.cs` — `ApplyRealBinary` dispatch: tries LReal64 → LReal128 → class Real, gated on `MaxComputationDecimalPlaces ≤ 37`, auto-promoting on overflow. `LRealDispatchTests.cs`: **6 tests green** (byte-identical to class Real at low precision).

**Measured results:**

| Path | vs class Real | notes |
|---|---|---|
| LReal64 elementwise add (1M) | **21×** faster, 0 alloc | 445 → 21 ns/elem |
| LReal64 elementwise mul (1M) | **44×** faster, 0 alloc | 422 → 9.6 ns/elem |
| LReal64 scalar add (pure struct) | 4.8× | 598 → 125 ns |
| LReal128 elementwise mul 16-digit (1M) | **70×** faster, 0 alloc | 1133 → 16.3 ns/elem |
| LReal128 elementwise add (1M) | 6.3× faster, 0 alloc | 488 → 77.5 ns/elem |
| LReal128 scalar mul 16-digit | 1.4× | 477 → 338 ns |
| LReal128 scalar add | 1.3× | 665 → 514 ns (UInt256 overkill for 16-digit; optimize later) |
| **NumericOps dispatch** scalar add | **3.5×** | 652 → 187 ns (includes Real↔LReal conversion, 200 B alloc) |
| NumericOps dispatch LReal128 scalar add | ~1× (no win) | UInt256 add + conversion ≈ class Real cost; use only for 16-digit multiply |

**Key finding (confirmed by measurement):** the conversion-based dispatch in `NumericOps` delivers a real but partial win (3.5× scalar add). The full 21–44× win requires the **union-in-Value** (store LReal64/LReal128 in `Value` so arithmetic never converts per-op) — that is the remaining, larger Stage-3 work. LReal64 handles 16-digit **add**; 16-digit **multiply** (32-digit result) requires LReal128.

**Surfaced, pre-existing, NOT caused by this work:** (1) `LanguageDocumentationTests` fails — `Language.md` `setprecision(n)` example has no `result` block. (2) `RealDivideTests` / precision-dependent tests are flaky under parallel runs because `Real.MaxComputationDecimalPlaces`/`DisplayDecimalPlaces` are global statics — each passes in isolation. (3) class `Real` computes `0.(3) * 0.(3)` = `0.111…10(8)` (wrong); LReal64/LReal128 correctly promote that case.

