# Typed Array Migration Plan

> Status: decisions locked — ready for your review. Derived from `docs/architecture/typed-array-requirements.md`. No implementation yet.

> How to read: `[ ]` items are the actionable checklist, each citing its requirement ID. `[x]` = a decision you have made.

## 0. Locked decisions (Gate 0)

All language-design choices are now resolved. One governing principle emerged from your answers and is treated as binding throughout:

**PRINCIPLE — semantics match today; performance is an implementation detail.** Array results must reproduce the *exact* scalar result (value and Kind) that the current per-element arithmetic already produces. Homogeneous storage and typed kernels are how we make that *fast*; they must not change what the language computes.

| # | Decision | Locked answer | Requirement |
|---|---|---|---|
| D1 | Subtraction of Natural arrays | **Narrow-back**: compute in Integer; narrow to Natural only if *no* element underflowed. `[5,3]-[4,2]` → Natural; `[5,3]-[6,4]` → Integer. Matches `1-2 = -1 (Integer)`. | PRM-004a |
| D2 | Division of any numeric arrays | **Narrow-back (extended to Real)**: compute in Real; narrow to Natural/Integer only if *every* quotient is exact — including `Real / Real`. `[4,6]/[2,2]` → Natural; `[1,2]/[2,2]` → Real; `[2.5]/[0.25]` → Natural. Deliberate, documented extension over today's Real division (which never narrowed). | PRM-004b |
| D3 | Mixed numeric literal `[1, 2.5, 3]` | **Promote to the max numeric kind** → `DenseArray<Real>`. | PRM-002 |
| D4 | Vector vs Array | **Unify** rank 1..N into a single `ArrayValue`. `ValueKind.Vector`/`Array` become presentation labels over it (kept for API/Studio compat, COMP-003). | §13-C |
| D5 | Zero-length dimensions | **Support** (`zeros(0)`, reshape-to-0, broadcasting edges). | EMP-001 |
| D6 | Empty reductions | `sum→0`, `prod→1` (identity); `min/max→error` (no neutral element). | EMP-002 |
| D7 | `zeros/ones/eye` dtype | **No dtype argument** — stay `Natural`-seeded exactly as today. Mixing with other dtypes happens at interaction time through normal promotion (e.g. `zeros(3) + 0.5` → Real). | §13-C |

## Implementation note (narrow-back ≠ slow)

D1/D2 require the result dtype to depend on the computed values (e.g. 'was there an underflow?', 'were all quotients exact?'). This does **not** mean per-element boxed arithmetic. The intended fast shape is:

- Kernels **compute in the widest type** for the op (Integer for Natural subtraction; Real for all division), in one typed contiguous pass.
- They **track a single narrowing flag** (anyUnderflow / allExact) rather than a per-element tag.
- If the flag allows, a **cheap final narrow pass** (or a no-op when the buffer is already the narrow type) produces the exact-result array.
This preserves D1/D2's observable semantics while keeping storage homogeneous and kernels contiguous.

## Stage 0 — Characterization (baseline before touching anything)

- [ ] Approve/revise `docs/architecture/typed-array-requirements.md` (your review action — not research).
- [x] Scaffold `arraybench` (scratch console project; Stopwatch harness — BenchmarkDotNet avoided to skip a NuGet dependency). — PERF-001.
- [x] Implement the §6.1 matrix (**partial**): scalar Real P8/P16; elementwise add/mul 1K/1M/10M (double only); reductions (sum); transpose 1000×1000; matmul 100×100 (all impls) + 1000×1000 (double only). **Not yet measured:** scalar broadcast, construction/literal promotion, dtype coercion, stride slicing (no syntax yet). — PERF-001.
- [x] Implement the §6.2 cost-attribution harness (`double`-raw vs `NdArray<double>` vs `Real`-raw vs `NdArray<Real>` vs `NdArray<Value>`+`NumericOps`). — PERF-002.
- [x] Run and record the **before** baseline → `docs/architecture/typed-array-benchmark-baseline.md`.

### Stage 0 findings (headline)

- Boxing/dispatch is **not** the bottleneck: ~9–15% of boxed elementwise cost.
- Real arithmetic + allocation **is** the bottleneck: ~86–91% (add 537 ns/elem, mul 1080 ns/elem; 356 B–1.8 KB/elem allocated).
- Real is ~189× (add) / ~337× (mul) slower than double; matmul ~243–279× slower with ~4.1 GB per 100×100 product.
- Consequence: homogeneous typed arrays recover only ~9–15% on elementwise; the big speedup needs a low-precision Real or opt-in machine types. Views (transpose 26 ms → ~0) and kernel dispatch are this redesign's structural wins.
- Full data + caveats: `docs/architecture/typed-array-benchmark-baseline.md`.

## Stage 1 — Introduce ArrayValue beside Value (no interpreter change)

- [ ] Create contract assembly `Lovelace.Abstractions` (AOT-compatible, source-gen JSON ready). — MOD-001.
- [ ] Define `DType`/`Precision` descriptors (Natural/Integer/Real, later Complex). — ARR-004.
- [ ] Define `ArrayValue` = `buffer + offset + shape + strides` exposing `Shape/Rank/Numel/Strides/ElementType/Precision`. — STO-001, ARR-004.
- [ ] Add a **contiguity/simplicity flag** (`IsContiguous` + a 'simple dense' marker) to enable fast paths. — STO-002.
- [ ] Implement `IsContiguous` and `AsContiguous`. — STO-002.
- [ ] Implement `DenseArray<T>` construction/validation (rank ≥ 1; element count = ∏shape; zero-length dims supported per D5). — ARR-001, EMP-001.
- [ ] Write unit tests for `ArrayValue`/`DenseArray<T>` invariants (shape/stride/numel, contiguity, zero-dim, construction errors).
- [ ] Leave `NdArray<Value>` fully intact; confirm `Lovelace.Array.Tests` + `Lovelace.Suite.Tests` stay green.
## Stage 2 — Adapter NdArray<Value> ⇄ ArrayValue

- [ ] Implement a temporary bidirectional adapter `NdArray<Value>` ⇄ `ArrayValue`.
- [ ] Migrate `Value.cs` so `ValueKind.Array` carries an `ArrayValue`, with `AsArray()` synthesizing the old `NdArray<Value>` view for remaining call sites.
- [ ] Migrate `ValueFormatter.FormatArray` to render the typed path. — COMP-004.
- [ ] Verify formatter output is byte-identical for every `Language.md` doctest example (`LanguageDocumentationTests` green).
## Stage 3 — Interpreter on the typed path (the semantic change)

- [ ] Rewrite `BuildList` to compute a single dtype = max numeric kind and widen elements (D3). — PRM-002.
- [ ] Implement whole-array elementwise ops with the **narrow-back** rule (D1/D2): compute in the widest type, track the narrowing flag, narrow to the narrowest exact type. Other ops follow the `Natural→Integer→Real` lattice. — PRM-003, PRM-004.
- [ ] Move `IndexValue` to the typed path; partial index returns a view where possible. — STO-003.
- [ ] Collapse the `Vector`/`Array` split onto the unified `ArrayValue` (D4): remove/reduce the `ToNdArray`/`FromNdArray` shim in `Interpreter.cs`.
- [ ] Move `RegisterArrayBuiltins` (zeros/ones/eye/sum/matmul/…) to the typed path; keep `zeros/ones/eye` Natural-seeded with no dtype arg (D7); apply empty-reduction identity/error (D6). — PRM-005.
- [ ] Update `ArrayTests.cs` / `VectorTests.cs` for the new (unchanged-observable) promotion semantics. — COMP-005.
- [ ] Update `Lovelace.Suite/docs/Language.md` examples + doctests to match. — COMP-004.
## Stage 4 — Views, strides, broadcasting, empty dims

- [ ] Implement zero-copy `transpose` as a stride/permutation view. — STO-003.
- [ ] Add stride/range slice syntax (`a[i:j:k]`) to `Parser.cs` and implement it as a zero-copy view. — STO-003.
- [ ] Make `reshape` zero-copy when contiguous; materialize only when required; document the materialization rule. — STO-005, STO-004.
- [ ] Implement N-dimensional broadcasting (right-aligned, dim equal-or-1) with explicit new error messages. — BDC-001, BDC-003.
- [ ] Preserve scalar broadcast as a special case. — BDC-002.
- [ ] Implement zero-length-dimension support (D5) and empty-reduction identity/error rules (D6). — EMP-001, EMP-002.
- [ ] Add tests for views, broadcasting, and empty cases; update any affected existing tests.
## Stage 5 — Kernel/backend dispatch + Modus boundary

- [ ] Refactor `ArrayMath` into a **reference backend** plus a dispatch table. — KRN-002.
- [ ] Prototype kernel interfaces (`IArrayKernel<T>` / `IMatrixKernel<T>` / `ILinearAlgebraBackend<T>`) — evaluate, don't assume. — KRN-005.
- [ ] Implement fallible backend selection (backend declines on unsupported dtype/shape/precision → reference runs). — KRN-003.
- [ ] Define the stable plugin contract in `Lovelace.Abstractions` (consume/return typed arrays; inspect shape/dtype/precision/strides; request contiguous). — MOD-001, MOD-002.
- [ ] Expose builtin registration + kernel registration to plugins. — MOD-003.
- [ ] Land a first **proof** Modus package (e.g. statistics) that consumes/returns typed arrays and registers a builtin + an optimized kernel, with zero dependency on `Parser`/`Interpreter`/`Ast`/`Value`. — MOD-001..006.
- [ ] Confirm the contract is AOT/source-gen-JSON friendly. — MOD-005.
## Stage 6 — Retire the boxed path

- [ ] Remove `NdArray<Value>` instantiation and the Stage-2 adapter from `Lovelace.Suite`.
- [ ] grep-verify no `NdArray<Value>` remains in `Lovelace.Suite`.
- [ ] Decide whether to keep `ObjectArray<Value>` as an explicit opt-in heterogeneous escape hatch (excluded from fast kernels).
- [ ] Re-run the benchmark suite; produce the before/after table. — PERF-002.
## Cross-cutting (any stage)

- [ ] Fix stale docs (`module-map.md` / `system-overview.md` still describe BCD `DigitStore`; update to binary limbs).
- [ ] Keep `SuiteEngine` public API source-compatible and the JSON boundary string-stable throughout. — COMP-001/002/003.
- [ ] Add a Studio dtype column only as an additive change if desired.
## Notes

- Dependencies: Stages 0→1→2 are strictly sequential and can start now; Stage 3 needs D1–D7 (now locked); Stage 4 depends on Stage 3's typed path; Stage 5's contract prototyping can overlap Stage 4, but the proof plugin needs Stage 4's views/strides.

- `ArrayValue` (dtype + buffer + shape + strides) is the single source of truth for arrays; `Value` remains the language value union but stops being per-element array storage. Heterogeneity stays available via non-numeric `Value` kinds and, if chosen, `ObjectArray<Value>`.


