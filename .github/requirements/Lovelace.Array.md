# Requirements: Lovelace.Array — N-Dimensional Array Data Structure & Numeric Operations

> Scope: A new reusable project `Lovelace.Array` that owns the **N-dimensional array** data type and
> the complete numeric operation set over it. This realizes the `VetorLovelace` migration the v1
> `Lovelace.Suite` requirements deferred. `Lovelace.Suite` **consumes** this project (like it consumes
> `Lovelace.Natural`/`Integer`/`Real`); it does not re-implement array logic.
> **Requirements document for review — no implementation yet.**

---

## 1. Status and rationale

The v1 Suite requirements deferred "Matrices / N-dimensional arrays … and advanced vector algebra" to a
later vector/matrix layer. That layer is **`Lovelace.Array`** — a proper project on the same footing as
`Lovelace.Natural`, `Lovelace.Integer`, and `Lovelace.Real`, holding:

1. the **container** (shape + rank + row-major data + strides + indexing + shape algebra), and
2. the **numeric algorithms** (reductions, linear algebra).

The language element type is Suite's `Value` (a widened union of the numeric kinds), so `Lovelace.Array`
cannot depend on `Lovelace.Suite` (that would be circular). It is instead **generic over the element type**
and parameterized by a small **field abstraction** that supplies element arithmetic. Suite provides the
`Value` field and the language syntax/built-ins; `Lovelace.Array` stays dependency-free and reusable.

Dependency direction:

\`\`\`
Lovelace.Array            (generic NdArray<T> + IField<T> + all array algorithms)
        ▲  references
Lovelace.Suite            (Value = wraps NdArray<Value>; ValueField : IField<Value>; syntax + built-ins)
        ▲  references
Lovelace.Console / Lovelace.Studio / Lovelace.Run
\`\`\`

---

## 2. Goals and Non-Goals

### Goals (v1)

| # | Goal |
|---|---|
| L1 | `NdArray<T>`: a generic N-dimensional array (rank ≥ 1) with an explicit shape, row-major data, and strides; immutable values. |
| L2 | Structural operations: full and partial indexing, `Reshape`, `Flatten`, `Transpose` (reverse axes + explicit permutation), `Squeeze`, `Concat`, `Fill`. |
| L3 | A minimal `IField<T>` abstraction (zero, one, `FromLong`, add/subtract/multiply/divide, negate, is-zero, compare, sqrt) so all numeric algorithms are element-type-agnostic. |
| L4 | Numeric operations over `IField<T>`: reductions (`Sum`, `Prod`, `Min`, `Max`, `Mean`, `Norm` — with optional axis) and linear algebra (`Dot`, `Cross`, `MatMul` incl. batched, `Det`, `Inverse`, `Trace`). |
| L5 | Construction helpers: `Zeros`, `Ones`, `Eye`. |
| L6 | Deterministic, dependency-free (base class library only), and unit-tested in `Lovelace.Array.Tests`. |

### Non-Goals / Deferred

- A concrete non-generic `Value`-based array (that is Suite's `Value` wrapping `NdArray<Value>`).
- General (NumPy-style) shape broadcasting between arrays of different shapes.
- Fancy indexing, boolean masks, stride slicing (`a[1:10:2]`), in-place mutation.
- Eigenvalues/eigenvectors, LU/QR/SVD, sparse or symbolic arrays.
- Named axes / labels / dtype metadata.

---

## 3. Design decisions (require your sign-off)

| # | Decision | Proposed choice | Rationale / risk |
|---|---|---|---|
| **L-D1** | Element abstraction | Generic `NdArray<T>` + `IField<T>` (not `INumber<T>`). | The language's `Value` is a widened union, not an `INumber`; a small field interface lets `Lovelace.Array` own the algorithms while Suite owns `Value` arithmetic. |
| **L-D2** | Rank coverage | `NdArray<T>` supports **rank ≥ 1** (a rank-1 `NdArray` is a vector). | Uniform indexing/reshape/transpose; Suite keeps its existing rank-1 `Vector` kind and wraps only rank ≥ 2 as `ValueKind.Array`. |
| **L-D3** | Layout | Row-major (last index varies fastest), 0-based indexing, positive dimensions. | Matches the existing `Vector` and NumPy/C conventions. |
| **L-D4** | Immutability | All operations return new arrays; no in-place mutation. | Keeps the container thread-safe and race-free for the future GUI. |
| **L-D5** | Error model | Throw `InvalidOperationException`/`ArgumentException` with precise messages (shape, rank, dimension, index). Suite maps these to positioned diagnostics. | No bare strings; no silent truncation. |
| **L-D6** | `MatMul` return | `Dot` handles rank-1·rank-1 → `T`; `MatMul` handles all rank ≥ 1 results (rank-2·rank-2, rank-2·rank-1, rank-1·rank-2, batched rank ≥ 2) and returns `NdArray<T>`. | Avoids a rank-0 `NdArray`; the language layer dispatches `matmul` accordingly. |

---

## 4. Type specification

### 4.1 `NdArray<T>`

| Member | Signature | Description |
|---|---|---|
| ctor | `NdArray(IReadOnlyList<long> shape, IReadOnlyList<T> data)` | Validates rank ≥ 1, each dim ≥ 1, and `∏ shape == data.Count`. |
| `Shape` | `long[]` | Dimension sizes, outer → inner. |
| `Data` | `IReadOnlyList<T>` | Row-major flat storage. |
| `Rank` | `int` | `Shape.Length`. |
| `Numel` | `long` | Total element count. |
| `Strides` | `long[]` (length `Rank+1`) | `Strides[i] = ∏ Shape[i..Rank]`; `Strides[Rank] = 1`. |
| `Get` | `T Get(IReadOnlyList<long> indices)` | Full index (one per dim); bounds-checked. |
| `Slice` | `NdArray<T> Slice(IReadOnlyList<long> indices)` | Partial index (1..Rank−1 indices) → lower-rank `NdArray<T>`. |
| `Reshape` | `NdArray<T> Reshape(IReadOnlyList<long> shape)` | Same data, new shape; `Numel` must match. |
| `Flatten` | `NdArray<T> Flatten()` | Rank-1 copy of the data. |
| `Transpose` | `NdArray<T> Transpose()` / `Transpose(IReadOnlyList<long> perm)` | Reverse axes, or reorder per a validated permutation. |
| `Squeeze` | `NdArray<T> Squeeze()` | Remove size-1 dimensions. |
| `Concat` | `static NdArray<T> Concat(a, b, long axis)` | Concatenate along one axis; equal rank, matching shapes except on `axis`. |
| `Fill` | `static NdArray<T> Fill(IReadOnlyList<long> shape, T value)` | Constant array. |

### 4.2 `IField<T>`

| Member | Description |
|---|---|
| `Zero`, `One` | Additive / multiplicative identity. |
| `FromLong(long)` | Injects a count/size as a `T` (for `Mean`). |
| `Add`, `Subtract`, `Multiply`, `Divide`, `Negate` | Field arithmetic. |
| `IsZero(T)` | Zero test (for pivoting). |
| `Compare(T, T)` | -1/0/1 (for `Min`/`Max`). |
| `Sqrt(T)` | Square root (for `Norm`). |

### 4.3 `ArrayMath` (numeric operations, parameterized by `IField<T>`)

| Operation | Signature | Returns | Notes |
|---|---|---|---|
| `Zeros` | `Zeros(f, shape…)` | `NdArray<T>` | fill `f.Zero`. |
| `Ones` | `Ones(f, shape…)` | `NdArray<T>` | fill `f.One`. |
| `Eye` | `Eye(f, r, c)` | `NdArray<T>` (rank 2) | `f.One` on the main diagonal. |
| `Sum` | `Sum(f, a)` / `Sum(f, a, axis)` | `T` / `NdArray<T>` | reduce all, or along one axis. |
| `Prod` | `Prod(f, a[, axis])` | `T` / `NdArray<T>` | — |
| `Min` | `Min(f, a[, axis])` | `T` / `NdArray<T>` | via `f.Compare`. |
| `Max` | `Max(f, a[, axis])` | `T` / `NdArray<T>` | via `f.Compare`. |
| `Mean` | `Mean(f, a[, axis])` | `T` / `NdArray<T>` | `f.Divide(sum, f.FromLong(count))`. |
| `Norm` | `Norm(f, a[, axis])` | `T` / `NdArray<T>` | `f.Sqrt(sum of squares)`. |
| `Dot` | `Dot(f, a, b)` | `T` | rank-1 · rank-1, equal length. |
| `Cross` | `Cross(f, a, b)` | `NdArray<T>` (rank 1, length 3) | 3-D cross product. |
| `MatMul` | `MatMul(f, a, b)` | `NdArray<T>` | rank-2·rank-2, rank-2·rank-1, rank-1·rank-2, and batched rank ≥ 2 (equal leading dims). |
| `Det` | `Det(f, m)` | `T` | rank-2 square; exact Gaussian elimination with pivoting. |
| `Inverse` | `Inverse(f, m)` | `NdArray<T>` (rank 2) | Gauss–Jordan; singular → error. |
| `Trace` | `Trace(f, m)` | `T` | rank-2 square; sum of main diagonal. |

---

## 5. Non-functional requirements

- **Conciseness & maintainability** — one concern per file: `NdArray.cs` (container + shape algebra), `IField.cs`,
  `ArrayMath.cs` (numeric algorithms). Base class library only; no third-party dependencies.
- **Determinism** — pure functions; results reproduce byte-for-byte given the same field and inputs.
- **Immutability & thread-safety** — arrays are read-only; operations allocate.
- **Error model** — precise `InvalidOperationException`/`ArgumentException` messages (no bare strings).
- **Reusability** — no `Lovelace.Suite` or numeric-project references; usable by any field implementation.

---

## 6. Test plan (Lovelace.Array.Tests)

1. `NdArray_GivenShapeData_ExposesShapeRankNumelStrides` — `NdArray<int>([2,2,2], 1..8)` reports shape `[2,2,2]`, rank `3`, numel `8`, strides `[8,4,2,1]`.
2. `NdArray_GivenMismatchedData_Throws` — data length ≠ ∏ shape throws.
3. `Get_GivenFullIndex_ReturnsElement` / `Get_GivenOutOfRange_Throws`.
4. `Slice_GivenPartialIndex_ReturnsLowerRankArray`.
5. `Reshape_GivenMatchingNumel_ReturnsNewShape` / `Reshape_GivenMismatch_Throws`.
6. `Transpose_GivenPerm_ReordersAxes` / `Transpose_GivenBadPerm_Throws`.
7. `Flatten_ReturnsRank1`, `Squeeze_RemovesSingletonDims`, `Concat_AlongAxis`.
8. `Sum_AllAndAxis`, `Mean_Exact`, `Norm_SqrtSumSquares`, `Dot`, `Cross`.
9. `MatMul_2x2`, `MatMul_Batched`, `MatMul_InnerMismatch_Throws`.
10. `Det_2x2`, `Inverse_2x2`, `Inverse_Singular_Throws`, `Trace`.
11. `Field` conformance via a trivial `IField<double>` and a deterministic `IField` over `Value` (suite-side).

---

## 7. Completeness checklist (to be marked during implementation)

- [x] Create `Lovelace.Array` project + `Lovelace.Array.Tests`; add both to the solution.
- [x] Implement `NdArray<T>` (shape/data/strides, Get, Slice, Reshape, Flatten, Transpose, Squeeze, Concat, Fill).
- [x] Implement `IField<T>`.
- [x] Implement `ArrayMath` (Zeros, Ones, Eye, Sum, Prod, Min, Max, Mean, Norm, Dot, Cross, MatMul, Det, Inverse, Trace).
- [x] Port/author `Lovelace.Array.Tests` and keep them green.

---

*Decisions L-D1 … L-D6 are proposals awaiting review. Zero Falsified rows.*
