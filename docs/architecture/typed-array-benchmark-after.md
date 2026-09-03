# Typed Array — After-Migration Benchmark & Comparison

> Measured by `arraybench` (Release, .NET 10, x64, single-threaded, Stopwatch) with the boxed
> `NdArray<Value>` reference and the typed `DenseArray<Value>` path in the **same run**, so the
> delta is the migration itself. 16-significant-digit (P16-class) `Real` operands, `MaxComputationDecimalPlaces`
> at its default (so the `LReal` fast path is **not** active here — this isolates the array-layer change).

## 1. Head-to-head (1M elements unless noted)

| Benchmark | Boxed `NdArray<Value>` | Typed `DenseArray<Value>` | Delta |
|---|---|---|---|
| elementwise add | 620.8 ms (240 MB) | 635.1 ms (240 MB) | **+2.3% — parity** |
| elementwise multiply | 3872.5 ms (774 MB) | 3816.9 ms (774 MB) | **−1.4% — parity** |
| reduction `sum` | 404.8 ms (265 MB) | 406.5 ms (265 MB) | **+0.4% — parity** |
| transpose 1000×1000 | 25.9 ms (48 MB) | 0.0007 ms (344 B) | **~36,000× faster, ~140,000× less allocation** |

## 2. What this means

1. **Elementwise/reduction is at parity.** The per-element cost is dominated by arbitrary-precision
   `Real` arithmetic + allocation (`elem-add-real-raw` = 566 ms of the 635 ms total), which the array
   redesign does **not** touch. Removing the `ValueField`/`IField` dispatch and the `NdArray<Value>`
   materialization is a wash (~±2%), not a headline — exactly what the Stage-0 baseline predicted
   ("boxing is only ~9–15% of the boxed cost").

2. **Zero-copy views are the headline structural win.** `transpose` is now a stride/permutation view
   (O(rank), 344 B) instead of a materialized copy (26 ms, 48 MB). `slice`/`reshape`-where-contiguous
   are likewise zero-copy. This is the migration's clearly-attributable speedup.

3. **The large elementwise speedup still requires the `LReal`/machine-type work.** With the language's
   precision knob at ≤ 37 digits, `NumericOps.ApplyRealBinary` already dispatches `LReal64`/`LReal128`
   (see `limited-real-plan.md`); that scalar path is orthogonal to this array-layer migration. The
   dedicated `DenseArray<Real>` storage (no `Value` box per element) is the union-in-`Value` effort in
   `limited-real-plan.md` Stage 3, not part of this migration.

## 3. Raw numbers (same run)

```
elem-add-boxed-value           620.8 ms   240,000,091 B   1M
elem-add-typed-value           635.1 ms   240,000,108 B   1M
elem-mul-boxed-value          3872.5 ms   774,401,744 B   1M
elem-mul-typed-value          3816.9 ms   774,403,332 B   1M
sum-boxed-value                404.8 ms   264,610,589 B   1M
sum-typed-value                406.5 ms   264,610,485 B   1M
transpose-1000x1000-boxed-value  25.9 ms    48,000,468 B   1M
transpose-1000x1000-typed-value   0.0007 ms       344 B   1M
```

*Caveats:* Stopwatch + `GC.GetAllocatedBytesForCurrentThread`, small reps (2–5) for the slow `Real`
paths; treat as order-of-magnitude. The "before" baseline (`typed-array-benchmark-baseline.md`) was
measured on the pre-migration code; this run re-measures the boxed reference side-by-side with the
typed path so the comparison is apples-to-apples.
