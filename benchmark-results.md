# Binary-limb vs BCD — benchmark results

Measured on the same machine with `bench/Program.cs` and `mulbench/Program.cs` (`Release`,
warmed), `main` (BCD baseline) vs `feature/binary-limb-natural` (64-bit binary limbs).
`speedup = main_ms / binary_ms` (> 1 means the binary build is faster).

## Headline (existing `bench` harness, unbalanced b = digits/2)

| Op | Digits | BCD (main) | Binary | Speedup |
|---|---|---|---|---|
| add | 100 000 | 0.378 ms | 0.022 ms | **17×** |
| sub | 10 000 | 1.264 ms | 0.100 ms | **13×** |
| mul | 100 | 0.201 ms | 0.010 ms | **20×** |
| mul | 100 000 | 14.99 ms | 5.94 ms | **2.5×** |
| div | 100 | 3.54 ms | 0.012 ms | **298×** |
| div | 10 000 | 19.0 ms | 0.40 ms | **48×** |
| factorial | 5 000 | 34.3 ms | 8.3 ms | **4.1×** |
| pi | 1 000 | 49.3 ms | 21.2 ms | **2.3×** |
| parse | 100 000 | 2.1 ms | 13.7 ms | **0.15× (binary slower)** |
| tostring | 100 000 | 4.7 ms | 53 ms | **0.09× (binary slower)** |

## Balanced multiply (`mulbench`, a = b = digits)

| Digits | BCD (main) | Binary | Speedup | Binary algorithm |
|---|---|---|---|---|
| 100 000 | ~42 ms | 7.5 ms | **~5.6×** | Karatsuba |
| 500 000 | 137.9 ms | 106.4 ms | **1.30×** | Karatsuba |
| 1 000 000 | 292.6 ms | 277.0 ms | **1.06×** | NTT |
| 2 000 000 | 618.3 ms | 592.5 ms | **1.04×** | NTT |

NTT/Karatsuba crossover for balanced operands is ≈ 900k digits; the threshold is set at
100 000 limbs total (≈ 1.9M decimal digits), so NTT only runs where it wins.

## Interpretation

**Binary limbs win the arithmetic core.** Add/sub/div are the big wins (div up to ~300×)
because every 64-bit limb processes ~19.3 decimal digits in one native instruction, versus one
BCD digit per inner-loop iteration with a `%10`/`/10` divide. Multiply wins ~20× at small
sizes, ~2.5–5× at 100k digits, and ~1.04–1.3× at 1M+ digits (where both sides run an O(n log n)
NTT, so the constant-factor gap closes).

**Binary limbs lose at decimal↔binary conversion.** BCD formats/parses in O(n) (it just
unpacks/packs nibbles, in parallel). A binary representation must *convert* bases —
divide-and-conquer parse is `O(M(n)·log n)` and `ToString` is still O(n²) (Knuth division at
each split). This is the fundamental tax of a binary working form, not a bug. It matters
because `Real.Sqrt`/`Real.Pi` round-trip through strings constantly (`ToNatural().ToString()`
+ `Nat.TryParse`), which is why the Pi speedup is only ~2–3×.

## Status

- [x] **NTT multiply** (Phase 2.1) — exact two-prime NTT over base-2^16 pieces; cross-checked
      against `BigInteger` at 2M digits. Binary multiply now wins or ties BCD at every size.
- [x] **Newton-reciprocal division** (Phase 2.2) — recursive half-precision Newton, correct
      (cross-checked to 200k digits) and dispatched at ≥ 262144 combined limbs (~5M digits).
      *Finding:* binary Knuth division has a small enough constant that it beats Newton up to
      ~5M digits (1.77s at 1M digits vs Newton's 3.5s), so Newton only pays off asymptotically.
      It is kept for completeness; practical division stays on the fast Knuth path.
- [ ] **Cached decimal form / hybrid** (Phase 2.3) — the one remaining *practical* gap. Cache
      the decimal string on parse so `ToString` is free; or keep BCD purely as a display cache.
      This closes the conversion gap entirely for the "parse once, print many" workload.
