# Requirements: Lovelace.Suite — N-Dimensional Array Language Integration

> Scope: Integrate the **`Lovelace.Array`** project (N-D array type + operations) into the
> `Lovelace.Suite` language: a `ValueKind.Array` wrapping `NdArray<Value>`, N-D literals and
> multi-index syntax, and built-in functions exposed to every front-end (`Lovelace.Console` REPL,
> `Lovelace.Studio` web IDE, `Lovelace.Run`). **Requirements document for review — no implementation yet.**

---

## 1. Status of the lift

The v1 Suite requirements deferred matrices / N-D arrays and advanced vector algebra. That work now lives
in the **`Lovelace.Array` project** (see `.github/requirements/Lovelace.Array.md`). This document covers
only what `Lovelace.Suite` adds on top: the `Value`/language integration and the user-facing surface.

---

## 2. Goals and Non-Goals

### Goals (v2)

| # | Goal |
|---|---|
| S1 | Add `ValueKind.Array` wrapping `NdArray<Value>` (rank ≥ 2); keep `ValueKind.Vector` (rank 1) unchanged. |
| S2 | Add N-D literal syntax (nested list literals of any depth) and N-D indexing `a[i]`, `a[i, j]`, `a[i, j, k]`, … with partial indexing. |
| S3 | Expose every `Lovelace.Array` operation as a **built-in function** (via a `ValueField : IField<Value>`). |
| S4 | Element-wise operators `+ - * / % ^` for arrays (same-shape + scalar broadcast). |
| S5 | Keep `docs/Language.md` doctested; update the REPL `help` text. |

### Non-Goals / Deferred

- General (NumPy-style) broadcasting, fancy indexing/masks, stride slicing, in-place mutation.
- Eigen/SVD/sparse/symbolic arrays.
- `*` as matrix product (matrix product is the `matmul` built-in).

---

## 3. Design decisions (require your sign-off)

| # | Decision | Proposed choice | Rationale / risk |
|---|---|---|---|
| **S-D1** | Where array logic lives | `Lovelace.Array` owns the type + algorithms; `Lovelace.Suite` only wraps `NdArray<Value>` and provides `ValueField : IField<Value>`. | Matches `Natural`/`Integer`/`Real`; no circular dependency. |
| **S-D2** | `Array * Array` | Element-wise (consistent with `Vector * Vector`); matrix/N-D product = `matmul(a, b)`. | Preserves the existing element-wise `*` contract. |
| **S-D3** | Broadcasting | Same-shape + scalar only; general broadcasting deferred. | Cheap and well-defined; full broadcasting is a separate increment. |
| **S-D4** | Ragged nested literal | Rectangular nested lists → rank-k array; ragged → positioned error (previously a vector-of-vectors). | Rectangular invariant required for a sound N-D type; the sole backward-incompatible change (currently untested/undocumented). |
| **S-D5** | `len(a)` | First dimension size (`shape[0]`); `shape(a)` → `[dims]`; `numel(a)` → count. | NumPy/C intuition. |
| **S-D6** | Reductions | All elements by default; optional `axis` (0-based) → rank−1. | Makes N-D useful without MATLAB column-by-default semantics. |
| **S-D7** | `mean` kind | `sum / numel` exact (`Natural` when exact, else `Real`). | Reuses exact widening/division. |
| **S-D8** | `det`/`inv` domain | `ValueField` over exact `Real` (Gaussian elimination); rank ≠ 2 or singular → error. | Exact, no `double` rounding. |
| **S-D9** | `matmul` dispatch | `rank-1 · rank-1` → `Dot` (scalar); otherwise `MatMul` (batched included). | Mirrors `Lovelace.Array`'s `Dot`/`MatMul` split. |

---

## 4. Language specification

### 4.1 Values

`ValueKind.Array` wraps `NdArray<Value>` (rank ≥ 2). `ValueKind.Vector` (rank 1) is unchanged.

### 4.2 Literals

| Construct | Syntax | Result |
|---|---|---|
| Vector | `[1, 2, 3]` | rank-1 `Vector` |
| Matrix | `[[1, 2], [3, 4]]` | rank-2 `Array`, shape `[2, 2]` |
| N-D | `[[[1, 2], [3, 4]], [[5, 6], [7, 8]]]` | rank-3 `Array`, shape `[2, 2, 2]` |
| Ragged | `[[1, 2], [3]]` | error (S-D4) |

### 4.3 Indexing

| Construct | Syntax | Result |
|---|---|---|
| Vector element | `v[i]` | element, 0-based |
| Full index | `a[i, j, …, k]` | element |
| Partial index | `a[i, …]` (fewer indices than rank) | sub-array; rank-1 → `Vector`, rank ≥ 2 → `Array` |

### 4.4 Element-wise operators

`+ - * / % ^` element-wise between same-shape arrays, or scalar∘array / array∘scalar. Mismatched shapes are a positioned error.

### 4.5 Built-in functions

Each maps to a `Lovelace.Array` operation over `ValueField`.

| Built-in | Signature | Maps to | Returns |
|---|---|---|---|
| `zeros` | `zeros(d1, …, dn)` | `ArrayMath.Zeros` | `Array` |
| `ones` | `ones(d1, …, dn)` | `ArrayMath.Ones` | `Array` |
| `eye` | `eye(n)` / `eye(r, c)` | `ArrayMath.Eye` | `Array` (rank 2) |
| `reshape` | `reshape(a, d1, …, dn)` | `NdArray.Reshape` | `Vector`/`Array` |
| `shape` | `shape(a)` | `NdArray.Shape` | `Vector` |
| `rank` / `ndims` | `rank(a)` | `NdArray.Rank` | `Natural` |
| `numel` | `numel(a)` | `NdArray.Numel` | `Natural` |
| `len` | `len(a)` | `Shape[0]` | `Natural` |
| `flatten` | `flatten(a)` | `NdArray.Flatten` | `Vector` |
| `transpose` | `transpose(a)` / `transpose(a, perm)` | `NdArray.Transpose` | `Array` |
| `squeeze` | `squeeze(a)` | `NdArray.Squeeze` | `Vector`/`Array` |
| `sum` | `sum(a)` / `sum(a, axis)` | `ArrayMath.Sum` | scalar / rank−1 |
| `prod` | `prod(a[, axis])` | `ArrayMath.Prod` | scalar / rank−1 |
| `min` | `min(a[, axis])` | `ArrayMath.Min` | scalar / rank−1 |
| `max` | `max(a[, axis])` | `ArrayMath.Max` | scalar / rank−1 |
| `mean` | `mean(a[, axis])` | `ArrayMath.Mean` | scalar / rank−1 |
| `norm` | `norm(a[, axis])` | `ArrayMath.Norm` | scalar / rank−1 |
| `dot` | `dot(a, b)` | `ArrayMath.Dot` | numeric |
| `cross` | `cross(a, b)` | `ArrayMath.Cross` | `Vector` |
| `matmul` | `matmul(a, b)` | `Dot` or `MatMul` | scalar / `Vector` / `Array` |
| `det` | `det(m)` | `ArrayMath.Det` | `numeric` |
| `inv` | `inv(m)` | `ArrayMath.Inverse` | `Array` (rank 2); scalar `inv(x)` unchanged |
| `trace` | `trace(m)` | `ArrayMath.Trace` | numeric |
| `concat` | `concat(a, b[, axis])` | `NdArray.Concat` | `Vector`/`Array` |
| `append` | `append(a, b)` | `NdArray.Concat` (axis 0) | `Vector` |

---

## 5. Public API / front-end exposure

- **Suite-side API.** `ValueKind.Array` + `Value(NdArray<Value>)` + `AsArray()`; `ValueField : IField<Value>`
  (delegating to `NumericOps` and `Rl.Sqrt`); `ValueFormatter` renders nested brackets.
- **REPL.** `help` text lists the new built-ins; `vars`/`funcs` render the `Array` kind and new functions.
- **Studio.** `EngineHost` projects `StateSnapshot` → DTOs; `Kind == Array` flows via `StateVariable.Kind`
  → `VariableRow.Kind`; new functions flow via `FunctionRow`. **No DTO/endpoint change.**

---

## 6. Non-functional requirements

- **Conciseness** — Suite adds only: `ValueKind.Array` + `AsArray()`, `ValueField`, array element-wise/indexing in the interpreter, and built-in registration. Array algorithms stay in `Lovelace.Array`.
- **Determinism** — inherited from `Lovelace.Array` + `NumericOps`.
- **Backward compatibility** — 277 Suite tests + 133 REPL tests stay green; the only deliberate change is S-D4 (ragged literal → error).
- **Error model** — array exceptions are wrapped into positioned diagnostics by the existing `SuiteEngine` mechanism.
- **Async** — `norm` uses the synchronous `Rl.Sqrt` via `ValueField.Sqrt`; the existing `sqrt`/`pi` built-ins remain async. No async is forced on array arithmetic.

---

## 7. Test plan (Lovelace.Suite.Tests)

1. `Evaluate_GivenNestedEqualLengthLists_ProducesArrayOfExpectedRank` — `[[[1,2],[3,4]],[[5,6],[7,8]]]` → rank 3, shape `[2,2,2]`.
2. `Evaluate_GivenFlatList_StillProducesVector` / `Evaluate_GivenRaggedNestedList_ReportsError`.
3. `Evaluate_GivenPartialIndex_ReturnsLowerRankArray` / `Evaluate_GivenFullIndex_ReturnsElement` / `Evaluate_GivenIndexOutOfRange_ReportsError`.
4. `Evaluate_GivenZeros_ProducesShapeRequested` / `Evaluate_GivenReshape_ProducesNewShape` / `Evaluate_GivenTransposePerm_ReordersAxes`.
5. `Evaluate_GivenShapeRankNumel_ReportMetadata`.
6. `Evaluate_GivenSum_AllAndAxis` / `Evaluate_GivenMean_Exact` / `Evaluate_GivenNorm_Euclidean`.
7. `Evaluate_GivenMatmul_2x2` / `Evaluate_GivenDot` / `Evaluate_GivenCross`.
8. `Evaluate_GivenDet` / `Evaluate_GivenInv` / `Evaluate_GivenSingular_ReportsError` / `Evaluate_GivenTrace`.
9. `Functions_GivenFreshEngine_IncludeArrayBuiltins` / `CaptureState_GivenArrayVariable_ReportsKindArray`.
10. `DocumentedExample_GivenArrayExamples_MatchesEngine` (doctest over `docs/Language.md`).

---

## 8. Completeness checklist (to be marked during implementation)

- [x] Create `Lovelace.Array` + `Lovelace.Array.Tests`; reference from `Lovelace.Suite`; add to the solution.
- [x] Add `ValueKind.Array` + `Value(NdArray<Value>)` + `AsArray()` + `ValueFormatter` rendering.
- [x] Extend `IndexExpr` to a multi-index list; parse `a[i, j, …]` and N-D nested-list literals (rectangular validation).
- [x] Implement array element-wise operators + indexing in the interpreter.
- [x] Implement `ValueField : IField<Value>` over `NumericOps`.
- [x] Register all built-ins (construction, introspection, manipulation, reductions, linear algebra, concat).
- [x] Update REPL `help` text.
- [x] Update `docs/Language.md` with doctested examples.
- [x] Add `ArrayTests.cs` (suite-level) and run the full `Lovelace.Suite.Tests` green.

---

*Decisions S-D1 … S-D9 are proposals awaiting review. Zero Falsified rows.*
