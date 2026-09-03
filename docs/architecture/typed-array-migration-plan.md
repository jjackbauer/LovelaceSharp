# Typed Array Migration Plan

> Status: **Ready to execute** — language-design decisions D1–D7 locked; Stage 0 baseline measured; Stages 1–6 sequenced below. No array-layer implementation exists yet.
>
> Derived from `docs/architecture/typed-array-requirements.md` (normative requirements), `docs/architecture/typed-array-benchmark-baseline.md` (measured baseline), and `docs/architecture/limited-real-plan.md` (the scalar fast-path effort this plan consumes).
>
> How to read: `[ ]` items are the actionable checklist; each cites its requirement ID. `[x]` = done. Every file reference is relative to the repo root; line numbers are against the current HEAD.

## 0. Summary

Replace the boxed `NdArray<Value>` array payload with a homogeneous, typed `ArrayValue` (`buffer + offset + shape + strides` + dtype/precision metadata), staged so the language never changes behavior that isn't a locked decision. The migration is a *representation and execution* change, not a *semantics* change: the exact scalar result (value **and** `Kind`) that per-element arithmetic produces today is the contract the typed path must reproduce.

Three measured facts drive the shape of this plan (from `arraybench`, see the baseline doc):

1. **Boxing/dispatch is not the bottleneck** — ~9–15% of boxed elementwise cost. The typed path recovers this, but it is not the headline.
2. **`Real` arithmetic + allocation is the bottleneck** — ~86–91% (add 537 ns/elem, mul 1080 ns/elem; 356 B–1.8 KB/elem). That is addressed by the **parallel** `LReal64`/`LReal128` effort (`limited-real-plan.md`), which this plan slots into its kernel layer (Stage 5).
3. **This redesign's clearly-attributable wins are structural** — zero-copy views (transpose 26 ms → ~0), contiguous buffers, kernel dispatch, dtype metadata for plugins, plus the ~9–15% boxing removal.

Non-goals (unchanged from the requirements doc): no `double` default; no MATLAB semantics; no flag-day rewrite; no per-element `Kind` retention; no Modus coupling to interpreter internals.

---

## 1. Locked decisions (Gate 0)

One governing principle binds every stage:

> **PRINCIPLE — semantics match today; performance is an implementation detail.** Array results must reproduce the *exact* scalar result (value and `Kind`) the current per-element arithmetic already produces. Homogeneous storage and typed kernels are how we make that *fast*; they must not change what the language computes.

| # | Decision | Locked answer | Requirement |
|---|---|---|---|
| D1 | Subtraction of Natural arrays | **Narrow-back**: compute in Integer; narrow to Natural only if *no* element underflowed. `[5,3]-[4,2]` → Natural; `[5,3]-[6,4]` → Integer. Matches `1-2 = -1 (Integer)`. | PRM-004a |
| D2 | Division of any numeric arrays | **Narrow-back (extended to Real)**: compute in Real; narrow to Natural/Integer only if *every* quotient is exact — including `Real / Real`. `[4,6]/[2,2]` → Natural; `[1,2]/[2,2]` → Real; `[2.5]/[0.25]` → Natural. Deliberate, documented extension over today's Real division (which never narrowed). | PRM-004b |
| D3 | Mixed numeric literal `[1, 2.5, 3]` | **Promote to the max numeric kind** → `DenseArray<Real>`. | PRM-002 |
| D4 | Vector vs Array | **Unify** rank 1..N into a single `ArrayValue`. `ValueKind.Vector`/`Array` become presentation labels over it (kept for API/Studio compat, COMP-003). | §13-C |
| D5 | Zero-length dimensions | **Support** (`zeros(0)`, reshape-to-0, broadcasting edges). | EMP-001 |
| D6 | Empty reductions | `sum→0`, `prod→1` (identity); `min/max→error` (no neutral element). | EMP-002 |
| D7 | `zeros/ones/eye` dtype | **No dtype argument** — stay `Natural`-seeded exactly as today. Mixing with other dtypes happens at interaction time through normal promotion (e.g. `zeros(3) + 0.5` → Real). | §13-C |

### Implementation note (narrow-back ≠ slow)

D1/D2 require the result dtype to depend on computed values ("was there an underflow?", "were all quotients exact?"). This does **not** mean per-element boxed arithmetic. The intended fast shape:

- Kernels **compute in the widest type** for the op (Integer for Natural subtraction; Real for all division), in one typed contiguous pass.
- They **track a single narrowing flag** (`anyUnderflow` / `allExact`) rather than a per-element tag.
- If the flag allows, a **cheap final narrow pass** (a no-op when the buffer is already the narrow type) produces the exact-result array.

This preserves D1/D2's observable semantics while keeping storage homogeneous and kernels contiguous.

---

## 2. Ground truth — current code (verified at HEAD)

The migration touches a small, high-leverage seam. These are the exact sites the stages below modify.

### 2.1 The generic layer that already exists (`Lovelace.Array`, no `Value` dependency)

- `Lovelace.Array/NdArray.cs` — `public sealed class NdArray<T>` with `long[] Shape`, `IReadOnlyList<T> Data`, `int Rank`, `long Numel`, `long[] Strides` (length `Rank+1`, `Strides[i] = ∏Shape[i..]`, row-major). Methods: `Get` (53), `Slice` (65 — *materializes* the trailing block into a new `List<T>`), `Reshape` (86 — *shares* `Data`, i.e. already zero-copy), `Flatten` (95), `Transpose()`/`Transpose(perm)` (98/101 — *materializes* a fresh `List<T>`), `Squeeze` (133), `Fill` (146), `Concat` (154). The constructor (25) and `Product` (264) throw on any dimension `< 1` (no zero-length dims).
- `Lovelace.Array/IField.cs` — `public interface IField<T>`: `Zero`, `One`, `FromLong(long)`, `Add/Subtract/Multiply/Divide`, `Negate`, `IsZero`, `Compare`, `Sqrt`. The element-arithmetic seam (kept — KRN-002).
- `Lovelace.Array/ArrayMath.cs` — `public static class ArrayMath` with `Zeros/Ones/Eye/Sum/Prod/Min/Max/Mean/Norm/Dot/Cross/MatMul/Det/Trace/Inverse` over `IField<T>`. Stage 5 turns this into the **reference backend + dispatch table**.

### 2.2 The boxed path that gets replaced (`Lovelace.Suite`)

- `Lovelace.Suite/Value.cs` — `ValueKind` enum (17: `Natural, Integer, Real, Boolean, Text, Vector, Function, Void, Array`); `Value` (42) = `object _inner` + `Kind`. Numeric widening `Natural → Integer → Real` via `Widen` (159) / `WidenPair` (193). `ValueKind.Array` ctor (93) wraps `NdArray<Value>`; `AsArray()` (145) casts it back.
- `Lovelace.Suite/ValueField.cs` — `ValueField : IField<Value>` (singleton `Instance`), bridging `ArrayMath` to `NumericOps`.
- `Lovelace.Suite/NumericOps.cs` — `Apply` (30) widens via `WidenPair` then dispatches per `Kind`. The semantics the typed path must reproduce: `SubtractNatural` (151) widens to Integer on underflow; `DivideNatural`/`DivideInteger` (165/176) return Natural/Integer on exact quotient else Real. `ApplyRealBinary` (70) already tries `LReal64` → `LReal128` → class `Real` when `MaxComputationDecimalPlaces ≤ 37` (the LReal fast path).
- `Lovelace.Suite/Interpreter.cs` — the array/vector paths:
  - `EvaluateLiteral` (248): `.`/`(` → Real, else Natural.
  - `EvaluateBinaryAsync` (281): `Array` → `EvaluateArrayBinary` (335); `Vector` → `EvaluateVectorBinary` (305).
  - `EvaluateArrayBinary` (335): same-shape **or** scalar broadcast, per-element `ApplyScalarBinary`; builds `new NdArray<Value>(shape, List<Value>)`.
  - `IndexValue` (494): full index → `Get`; partial index → `Slice` (rank-1 sub → re-wrapped as `Vector`).
  - `BuildList` (535): nested rectangular containers → `NdArray<Value>`; ragged → error; flat → `IReadOnlyList<Value>` (rank 1, no homogenization — mixed `[1, 2.5, 3]` keeps per-element kinds).
  - `BuildRange` (585): `1..5` / `1..2..7` → `Vector`.
  - `ToNdArray` (1020) / `FromNdArray` (1033): the Vector⇄Array normalization shim.
  - `RegisterArrayBuiltins` (1082): `zeros/ones/eye/reshape/shape/rank/ndims/numel/flatten/transpose/squeeze/sum/prod/min/max/mean/norm/dot/cross/matmul/det/trace/concat/append`.
- `Lovelace.Suite/ValueFormatter.cs` — `Format` (13) routes `ValueKind.Array` → `FormatArray(NdArray<Value>)` (28); `FormatTyped` (54) appends the `(Array)`/`(Vector)` suffix. **Display output is a compat contract (COMP-004).**
- `Lovelace.Suite/Ast.cs` — `RangeExpr` (76), `IndexExpr(Expr Target, List<Expr> Indices)` (79). **There is no slice/colon syntax** — `a[i:j:k]` does not parse (STO-003 is new syntax).

### 2.3 Coupling map (exact `NdArray<Value>` sites)

| File | Sites | Notes |
|---|---|---|
| `Lovelace.Suite/Interpreter.cs` | 12 | 347, 356, 363 (elementwise), 553 (`BuildList`), 1020/1025/1033 (`ToNdArray`/`FromNdArray`), 1061–1062 & 1073 (reduction delegates), 1222/1233 (`Concat`) |
| `Lovelace.Suite/Value.cs` | 2 | 93 (ctor), 145 (`AsArray`) |
| `Lovelace.Suite/ValueFormatter.cs` | 1 | 28 (`FormatArray`) |

15 production sites in three files — the migration seam is genuinely small and stageable without a flag day.

### 2.4 The scalar fast path that already exists (consumed by Stage 5)

`Lovelace.Real/LReal64.cs` and `LReal128.cs` exist; `NumericOps.ApplyRealBinary` dispatches to them (conversion-based, measured ~3.5× scalar add). The full 21–44× win needs storing LReal in the value union (`limited-real-plan.md` Stage 3, still open). This array plan does **not** depend on that union; its `DenseArray<Real>` kernels route through the same `NumericOps`/LReal path and inherit whatever scalar speedup exists at the time.

---

## 3. Target architecture

### 3.1 New assembly `Lovelace.Abstractions` (MOD-001)

A new `net10.0`, `IsAotCompatible=true`, source-gen-JSON-friendly contract assembly owning the array/numerical contracts so plugins never depend on `Parser`/`Ast`/`Interpreter`/`Value`. No project references (leaf assembly), mirroring `Lovelace.Array.csproj`.

### 3.2 `ArrayValue` / `DenseArray<T>` (proposed default shape — interface alternatives left open per KRN-005, §11)

```csharp
namespace Lovelace.Abstractions;

public enum DType { Natural, Integer, Real }        // + Complex later (MOD-004)

/// <summary>Language-facing, non-generic array handle — the single source of truth.</summary>
public abstract class ArrayValue
{
    public abstract DType DType { get; }
    public abstract Precision Precision { get; }    // see §3.3
    public abstract int  Rank { get; }
    public abstract long[] Shape { get; }
    public abstract long[] Strides { get; }
    public abstract long Offset { get; }
    public abstract long Numel { get; }
    public abstract bool IsContiguous { get; }      // STO-002
    public abstract ArrayValue AsContiguous();      // STO-002

    public abstract object GetElement(long flat);   // only at the Value boundary; never in kernels
    public abstract ArrayValue Slice(/* range per axis */);
    public abstract ArrayValue Transpose(long[] perm);
    public abstract ArrayValue Reshape(long[] shape);
    // broadcasting, elementwise ops, reductions live on the kernel layer, not here (KRN-001/002)
}

/// <summary>Homogeneous dense storage — one T[] plus offset/shape/strides (STO-001).</summary>
public sealed class DenseArray<T> : ArrayValue
{
    private readonly T[] _buffer;
    // T[] = Natural, Integer, Real (later LReal64/LReal128 internally, Complex)
    public ReadOnlySpan<T> AsSpan() => _buffer.AsSpan((int)Offset, checked((int)Numel));
}
```

`Value` stops being per-element array storage: `ValueKind.Array` (and, per D4, `ValueKind.Vector`) wraps one `ArrayValue`; the `T[]` is the real storage.

### 3.3 `DType` / `Precision`

`DType` is the enum above. `Precision` is a `readonly struct` (significant-digit count) *derived at construction from the process-global `Real.MaxComputationDecimalPlaces`* — precision stays global for now; the exact per-array model is an open item (§11) because it may require a `Real` change.

### 3.4 Kernel / backend dispatch (KRN-001..005)

`ArrayMath` is refactored into: (a) a **reference backend** (the current exact algorithms, unchanged results), and (b) a **dispatch table** keyed by `(op, DType, precision, shape-class, contiguity)`. Backend selection is **fallible**: a kernel declines an unsupported `(dtype, shape, precision, strides)` and the reference backend runs.

```csharp
public interface IArrayKernel<T> where T : unmanaged { /* elementwise/reduce over ReadOnlySpan<T> */ }
public interface ILinearAlgebraBackend<T> where T : unmanaged { /* matmul/det/inv */ }
```

Prototype before locking (KRN-005). The stable plugin contract (`MOD-002/003`) exposes: consume/return `ArrayValue`, inspect `shape/dtype/precision/strides`, request `AsContiguous()`, register builtins + kernels.

### 3.5 Where LReal slots in

The language-facing `DType` stays `Real` (no tier leak). `DenseArray<Real>` kernels call the same `NumericOps`/LReal scalar path; a later optimization (not gating this plan) is a whole-array "all elements fit LReal64 → run the LReal64 kernel" fast path, mirroring `ApplyRealBinary`.

---

## 4. Stages

> Dependencies: 0 → 1 → 2 are strictly sequential. 3 needs D1–D7. 4 needs 3. 5's *contract prototyping* overlaps 4; 5's *proof plugin* needs 4's views/strides. 6 last. Every stage keeps the full test suite green at its exit.

### Stage 0 — Characterization (baseline) — **DONE**

- [x] Scaffold `arraybench` (`arraybench/Program.cs`, Stopwatch harness; BenchmarkDotNet deliberately avoided). — PERF-001.
- [x] Implement the §6.1 matrix (**partial**): scalar Real P8/P16; elementwise add/mul 1K/1M/10M (double only); reductions (sum); transpose 1000×1000; matmul 100×100 (all impls) + 1000×1000 (double only). *Not yet measured:* scalar broadcast, construction/literal promotion, dtype coercion, stride slicing (no syntax yet). — PERF-001.
- [x] Implement the §6.2 cost-attribution harness (`double`-raw vs `NdArray<double>` vs `Real`-raw vs `NdArray<Real>` vs `NdArray<Value>`+`NumericOps`). — PERF-002.
- [x] Record the **before** baseline → `docs/architecture/typed-array-benchmark-baseline.md`.
- [ ] Approve/revise this document (review action, not research).

**Exit:** baseline recorded; decisions D1–D7 locked.

### Stage 1 — Introduce `ArrayValue` beside `Value` (no interpreter change)

No `Lovelace.Suite` code changes; `NdArray<Value>` stays fully working.

- [ ] Create `Lovelace.Abstractions` (leaf `net10.0`, `IsAotCompatible=true`). — MOD-001.
- [ ] Define `DType` enum and `Precision` descriptor. — ARR-004.
- [ ] Define `ArrayValue` (`Shape/Rank/Numel/Strides/Offset/DType/Precision/IsContiguous/AsContiguous`). — STO-001, STO-002, ARR-004.
- [ ] Define `DenseArray<T>` (rank ≥ 1; element count = ∏shape; zero-length dims supported per D5). — ARR-001, EMP-001.
- [ ] Implement row-major `ComputeStrides`, `Offset(flat→coords)`, `IsContiguous`, `AsContiguous`. — STO-001/002.
- [ ] Unit-test invariants: shape/stride/numel, contiguity, zero-dim construction, bad-shape errors (new `Lovelace.Abstractions.Tests` or inside `Lovelace.Array.Tests`).
- [ ] Add `Lovelace.Suite` a `ProjectReference` to `Lovelace.Abstractions` (compile-only for now; no call sites yet).
- [ ] Confirm `Lovelace.Array.Tests` + `Lovelace.Suite.Tests` stay green.

**Exit:** `ArrayValue`/`DenseArray<T>` compile and are fully unit-tested; zero production behavior change.

### Stage 2 — Adapter `NdArray<Value>` ⇄ `ArrayValue`

- [ ] Implement a temporary bidirectional adapter: `ArrayValue ⇄ NdArray<Value>` (and rank-1 ⇄ `IReadOnlyList<Value>`).
- [ ] Migrate `Value.cs` so `ValueKind.Array` carries an `ArrayValue`; `AsArray()` synthesizes the old `NdArray<Value>` view for the remaining call sites.
- [ ] Migrate `ValueFormatter.FormatArray` to render from `ArrayValue` (typed path). — COMP-004.
- [ ] **Gate:** `LanguageDocumentationTests` green — formatter output byte-identical for every `Language.md` doctest (`[1,2]`, `[[1,2],[3,4]]`, `sum`, `mean`, `matmul`, `inv`, …). — COMP-004.

**Exit:** `Value.cs` + `ValueFormatter.cs` on the typed path; display output unchanged; full suite green.

### Stage 3 — Interpreter on the typed path (the semantic change)

This is where promotion (D1–D3, D4, D6, D7) becomes observable.

- [ ] Rewrite `BuildList` (Interpreter.cs 535) to compute a single `dtype = max numeric kind` and widen elements (D3); flat and nested literals both produce `ArrayValue`. — PRM-002.
- [ ] Implement whole-array elementwise ops with **narrow-back** (D1/D2): compute in the widest type, track `anyUnderflow`/`allExact`, narrow to the narrowest exact type. Other ops follow the `Natural→Integer→Real` lattice. Replace `EvaluateArrayBinary`/`EvaluateVectorBinary` (335/305). — PRM-003, PRM-004.
- [ ] Move `IndexValue` (494) to the typed path; partial index returns a view where possible (full view semantics land in Stage 4). — STO-003.
- [ ] Collapse the `Vector`/`Array` split onto `ArrayValue` (D4): remove/reduce `ToNdArray`/`FromNdArray` (1020/1033). — §13-C.
- [ ] Move `RegisterArrayBuiltins` (1082) to the typed path; keep `zeros/ones/eye` Natural-seeded with no dtype arg (D7); apply empty-reduction identity/error (D6). — PRM-005.
- [ ] Update `ArrayTests.cs`/`VectorTests.cs` for the new (observably unchanged) promotion semantics; add promotion tests for D1–D3. — COMP-005.
- [ ] Update `Lovelace.Suite/docs/Language.md` examples + doctests to match (D3 mixed literals now `Real`). — COMP-004.

**Exit:** mixed-literal promotion, narrow-back subtraction/division, and unified Vector/Array all correct; doctests green.

### Stage 4 — Views, strides, broadcasting, empty dims

- [ ] Implement zero-copy `transpose` as a stride/permutation view (`NdArray.Transpose` currently copies). — STO-003.
- [ ] Add stride/range slice syntax `a[i:j:k]` to `Parser.cs`/`Ast.cs` (extend `IndexExpr` with a slice arm) and implement it as a zero-copy view. — STO-003.
- [ ] Make `reshape` zero-copy when contiguous; materialize only when required; document the materialization rule. — STO-004, STO-005.
- [ ] Implement N-dimensional broadcasting (right-aligned; dim equal-or-1) with explicit new error messages. — BDC-001, BDC-003.
- [ ] Preserve scalar broadcast as a special case. — BDC-002.
- [ ] Implement zero-length-dimension support (D5) and empty-reduction identity/error (D6). — EMP-001, EMP-002.
- [ ] Add view/broadcast/empty tests; update any affected existing tests (mismatched-length error becomes a broadcast). — BDC-003.

**Exit:** new semantics tests green; `arraybench` shows the expected transpose/slice wins; compatibility notes in §7 documented.

### Stage 5 — Kernel/backend dispatch + Modus boundary

- [ ] Refactor `ArrayMath` into a **reference backend** + dispatch table. — KRN-002.
- [ ] Prototype kernel interfaces (`IArrayKernel<T>`/`ILinearAlgebraBackend<T>`) — evaluate, don't assume. — KRN-005.
- [ ] Implement fallible backend selection (backend declines on unsupported dtype/shape/precision → reference runs). — KRN-003.
- [ ] Define the stable plugin contract in `Lovelace.Abstractions` (consume/return `ArrayValue`; inspect shape/dtype/precision/strides; request `AsContiguous`). — MOD-001/002.
- [ ] Expose builtin + kernel registration to plugins. — MOD-003.
- [ ] Land a first **proof** Modus package (e.g. statistics) that consumes/returns typed arrays and registers a builtin + an optimized kernel, with **zero** dependency on `Parser`/`Interpreter`/`Ast`/`Value`. — MOD-001..006.
- [ ] Wire `DenseArray<Real>` kernels through the existing LReal scalar fast path; measure. — limited-real-plan Stage 5.
- [ ] Confirm the contract is AOT/source-gen-JSON friendly. — MOD-005.

**Exit:** a plugin consumes/returns typed arrays and registers a builtin + kernel without touching interpreter internals.

### Stage 6 — Retire the boxed path

- [ ] Remove `NdArray<Value>` instantiation and the Stage-2 adapter from `Lovelace.Suite`.
- [ ] grep-verify no `NdArray<Value>` remains in `Lovelace.Suite`.
- [ ] Decide whether to keep an explicit opt-in heterogeneous escape hatch (excluded from fast kernels).
- [ ] Re-run `arraybench`; produce the before/after table. — PERF-002.

**Exit:** `grep -R "NdArray<Value>" Lovelace.Suite` returns nothing; before/after table published.

---

## 5. File-by-file change inventory

| File | Current role | Change | Stage |
|---|---|---|---|
| `Lovelace.Abstractions/*` | *(new)* | `DType`, `Precision`, `ArrayValue`, `DenseArray<T>`, kernel/plugin contracts | 1, 5 |
| `Lovelace.Array/NdArray.cs` | generic container | unchanged (stays as a utility); `NdArray<Value>` use removed from Suite in 6 | — |
| `Lovelace.Array/ArrayMath.cs` | algorithms over `IField<T>` | refactor into reference backend + dispatch table | 5 |
| `Lovelace.Array/IField.cs` | element seam | kept as the scalar building block (KRN-002) | — |
| `Lovelace.Suite/Value.cs` | boxed union | `ValueKind.Array` (and Vector, D4) wraps `ArrayValue`; `AsArray()` adapter | 2, 3 |
| `Lovelace.Suite/ValueField.cs` | `IField<Value>` | retained as scalar seam; array ops bypass it for typed kernels | 3, 6 |
| `Lovelace.Suite/NumericOps.cs` | scalar widening | add whole-array promotion + narrow-back helpers; keep LReal dispatch | 3 |
| `Lovelace.Suite/Interpreter.cs` | array paths | `BuildList`/`EvaluateArrayBinary`/`EvaluateVectorBinary`/`IndexValue`/`RegisterArrayBuiltins` on typed path; drop `ToNdArray`/`FromNdArray` | 3 |
| `Lovelace.Suite/ValueFormatter.cs` | rendering | `FormatArray` reads `ArrayValue` | 2 |
| `Lovelace.Suite/Ast.cs`, `Parser.cs` | syntax | add slice/stride arm to `IndexExpr` | 4 |
| `Lovelace.Suite/Plotting.cs` | plots via `Value` | unchanged (consumes via public `Value`/`AsReal`) | — |
| `Lovelace.Suite/SuiteEngine.cs` | public API | unchanged (COMP-001/002/003) | — |
| `Lovelace.Studio/*`, `Lovelace.Run/*` | JSON projection | unchanged (string-based boundary) | — |
| `Lovelace.Suite.Tests/*`, `Lovelace.Array.Tests/*` | tests | see §6 | 1–6 |

---

## 6. Test plan

**Existing suites that must stay green at every stage:** `Lovelace.Array.Tests` (incl. `NdArrayTests.cs`), `Lovelace.Suite.Tests` (incl. `ArrayTests.cs`, `VectorTests.cs`, `ValueTests.cs`, `InterpreterLiteralTests.cs`, `InterpreterBinaryArithmeticTests.cs`, `InterpreterComparisonTests.cs`, `LanguageDocumentationTests.cs`, `PlotTests.cs`), plus `Lovelace.Real.Tests`/`Integer.Tests`/`Natural.Tests`.

**New tests:**

- Stage 1: `ArrayValue`/`DenseArray<T>` invariants — shape/stride/numel, contiguity (`IsContiguous`/`AsContiguous`), zero-dim, bad-shape errors, `AsSpan` bounds.
- Stage 2: formatter byte-parity across every `Language.md` doctest; adapter round-trips (`ArrayValue ⇄ NdArray<Value>` ⇄ rank-1 list).
- Stage 3: promotion semantics — D1 (`[5,3]-[4,2]`→Natural, `[5,3]-[6,4]`→Integer), D2 (`[4,6]/[2,2]`→Natural, `[1,2]/[2,2]`→Real, `[2.5]/[0.25]`→Natural), D3 (`[1,2.5,3]`→`DenseArray<Real>`), D4 (rank-1 and rank-2 unify), D6 (empty reductions), D7 (`zeros(3)+0.5`→Real).
- Stage 4: views (transpose/slice/reshape aliasing is unobservable per MUT-002/003), broadcasting (right-aligned, dim-equal-or-1, scalar special case), empty dims (reshape-to-0, `sum([])`→0, `min([])`→error).
- Stage 5: fallible backend selection (a declining kernel falls through to reference); plugin contract (consume/return `ArrayValue`; register builtin + kernel; AOT/source-gen JSON round-trip).
- Stage 6: `NdArray<Value>` absence grep; before/after benchmark table.

**Tests that will break and must be updated deliberately (per §11.1 of the requirements):** `ArrayTests.cs` (partial-index/materialization assumptions), `VectorTests.cs` (mismatched-length error becomes broadcast), any test asserting per-element `Kind` after mixed literals, `NdArrayTests.cs` (transpose now a view).

---

## 7. Risks and mitigations

| Risk | Severity | Mitigation |
|---|---|---|
| **Promotion observability** (D1/D2 change the `Kind` of results that are element-exact today) | High | Locked at Gate 0; promotion tests written *before* Stage 3 code; doctests pin the new Kind. |
| **Aliasing under views/COW** (MUT-002/003) | High | Arrays are immutable at the language level; view aliasing is unobservable because no op mutates in place; explicit COW tests in Stage 4. |
| **Precision is global/static** — per-array `Precision` may need a `Real` change | Medium | Ship `Precision` as a derived snapshot of the global knob; defer per-array precision (§11). |
| **Broadcasting changes error surfaces** (`VectorTests` mismatch error) | Medium | Deliberate, documented (BDC-003); versioned in `Language.md`. |
| **Over-specifying kernel interfaces** before a real backend exists | Medium | Prototype per KRN-005; keep `IField<T>` as the scalar seam; the proof plugin is the contract's validation. |
| **Native AOT constraints** on `DType`/kernel registry (reflection-free) | Medium | `IsAotCompatible=true` from day one; source-gen JSON; no reflection-based discovery. |
| **`LReal` fast path vs class `Real` divergence** (exactness) | Medium | LReal throws/promotes rather than rounds; `LRealDispatchTests` already pins byte-parity; array kernels route through the same path. |
| **Stale docs** (`module-map.md`/`system-overview.md` still say BCD `DigitStore`) | Low | Fix in cross-cutting; the code uses binary limbs since `Lovelace.Natural.BinaryLimb.md`. |

---

## 8. Sequencing, parallelism, and effort

- **Critical path:** Stage 1 → 2 → 3 → 4 → 6. Stage 5 overlaps 4 (contract prototyping) but its proof plugin needs 4's views/strides.
- **Parallelizable:** Stage 1's `ArrayValue`/`DenseArray<T>` and the `LReal` union-in-`Value` work (`limited-real-plan.md` Stage 3) are independent; the LReal scalar fast path is already landed.
- **Effort (rough, person-days):** Stage 1 ≈ 2–3; Stage 2 ≈ 1–2; Stage 3 ≈ 4–6 (highest-risk promotion work); Stage 4 ≈ 3–4; Stage 5 ≈ 3–5 (plus the proof plugin); Stage 6 ≈ 1. Total ≈ 14–21, dominated by Stage 3's semantic verification.

---

## 9. Rollback strategy

Each stage lands behind the same public surface (`SuiteEngine`, `ValueKind` names, string JSON, formatter output), so a stage can be reverted independently:

- Stages 1–2 are additive and revert cleanly (no behavior change).
- Stage 3 is the semantic change; it is the only stage that alters observable `Kind` (D1–D3), so it ships with its promotion tests and doctest updates in the same change — reverting it reverts the tests together.
- Stages 4–6 revert to the Stage-3 behavior (views/broadcasting are additive capabilities; retiring the boxed path is the final, separable step).

---

## 10. Definition of done

- [ ] `Lovelace.Abstractions` exists, is `IsAotCompatible`, and owns the array/numerical contracts.
- [ ] `ValueKind.Array`/`Vector` wrap a unified `ArrayValue`; `NdArray<Value>` is gone from `Lovelace.Suite` (grep-verified).
- [ ] D1–D7 are implemented and covered by tests; `Language.md` doctests green.
- [ ] Views (slice/transpose/reshape), N-D broadcasting, and empty dims work with tests.
- [ ] `ArrayMath` is a reference backend + dispatch table; one proof Modus plugin consumes/returns typed arrays with zero interpreter dependency.
- [ ] `arraybench` before/after table is published (PERF-002).

---

## 11. Deliberately deferred decisions (kept open by design)

- Exact kernel interface shape (`IArrayKernel<T>` vs descriptor+delegate registry vs generic-math) — decide after the Stage 5 prototype (KRN-005).
- The concrete buffer type (`T[]` vs `Memory<T>`/`ArrayPool<T>` vs a custom `IBuffer`) and its span/AOT implications.
- The precise `DType`/`Precision` descriptor model and whether per-array precision (vs global) becomes real — depends on a possible `Real` scoping change.
- Whether `Value` keeps `ValueKind.Vector`/`Array` as-is or adds a unified `ArrayValue` kind — D4 unifies the *payload*; the enum surface stays per COMP-003 unless renamed deliberately.
- Whether/where to expose a public structured array serialization (today's boundary is string-only).
- Naming/namespace ownership of `Lovelace.Abstractions` and any split of `Lovelace.Array`.

---

### Notes

- `ArrayValue` (dtype + buffer + shape + strides) is the single source of truth for arrays; `Value` remains the language value union but stops being per-element array storage. Heterogeneity stays available via non-numeric `Value` kinds and, if chosen, an opt-in heterogeneous escape hatch.
- Keep `SuiteEngine` public API source-compatible and the JSON boundary string-stable throughout (COMP-001/002/003).
- Cross-cutting cleanup (any stage): fix stale `module-map.md`/`system-overview.md` BCD references; add a Studio dtype column only as an additive change.
