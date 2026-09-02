# Binary-limb vs BCD — benchmark results

Measured on the same machine with `bench/Program.cs` (`Release`, warmed), `main` (BCD
baseline) vs `feature/binary-limb-natural` (64-bit binary limbs). `speedup = main_ms / binary_ms`
(> 1 means the binary build is faster). Raw numbers in `benchmark-results.csv`.

## Headline

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

## Interpretation

**Binary limbs win the arithmetic core.** Add/sub/div are the big wins (div up to ~300×),
because every 64-bit limb processes ~19.3 decimal digits in one native instruction, versus one
BCD digit per inner-loop iteration with a `%10`/`/10` divide. Multiply is ~20× at small sizes
and ~2.5× at 100k digits (schoolbook + Karatsuba; see the Phase-2 note below).

**Binary limbs lose at decimal↔binary conversion.** BCD formats/parses in O(n) (it just
unpacks/packs nibbles, in parallel). A binary representation must *convert* bases —
`O(M(n)·log n)` for parse and divide-and-conquer `ToString` — so `parse` and `toString` are
~3–10× slower than BCD at large sizes. This is the fundamental tax of a binary working form,
not a bug. It matters because `Real.Sqrt`/`Real.Pi` round-trip through strings constantly
(they use `ToNatural().ToString()` + `Nat.TryParse`), which is why the Pi speedup is only
~2–3× instead of the raw-multiply speedup.

## Phase 2 (not yet done)

1. **NTT / FFT multiply** (port the existing BCD `NttMultiply` to 64-bit limbs) — would flip
   the multiply crossover at very large operand sizes and speed up `parse`/`Pi` further.
2. **Newton-reciprocal division** (`O(M(n) log n)`) — currently division is Knuth `O(n²)`,
   which caps `ToString` at large sizes; Newton division + divide-and-conquer `ToString`
   would reach `O(M(n) log n)`.
3. **Cached decimal form / hybrid** — cache the decimal string on parse so `ToString` is
   free; or keep BCD purely as a display cache. This closes the conversion gap entirely for
   the "parse once, print many" workload.
