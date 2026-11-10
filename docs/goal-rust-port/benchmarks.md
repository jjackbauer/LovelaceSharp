# Rust vs C# on one machine

**Paired measurements.** The host degrades under sustained load, so a number from one round cannot
be compared with a number from another. Every claim below is from a run where the Rust and C#
harnesses executed **interleaved in the same session** on this machine.

## Paired results (EVD-036)

| Operation | Size | Rust | C# | |
|---|---|---|---|---|
| multiply | 100 000 digits | 22.1 ms | 19.5 ms | 1.13x slower |
| divide | 100 000 / 25 000 | **132.4 ms** | 140.2 ms | **1.06x faster** |
| factorial | 20 000! | 51.3 ms | 12.1 ms | 4.2x slower |
| parse | 100 000 digits | 3.2 ms | 0.99 ms | 3.2x slower |
| tostring (warm) | 100 000 digits | 0.017 ms | 0.0003 ms | cache hit on both sides |

## What the Rust tower has

- base-2^64 limbs, canonical, held as limbs **or** canonical decimal digits with lazy materialisation
  (the reference's own design)
- schoolbook multiply below 32 limbs, Karatsuba above, exact NTT when **both** operands are wide, the
  three NTT moduli and the forward transform pair on separate workers
- Burnikel-Ziegler division above 128 divisor limbs, ported from the OpenJDK reference, always
  verified with a fall back to long division
- balanced product tree for factorials, parallelised
- divide-and-conquer decimal conversion, parallelised, with a power-of-ten cache

## Same-session A/B measurements

| Comparison | Result |
|---|---|
| division 100k: long division vs Burnikel-Ziegler | 211.2 vs **107.8 ms** (1.96x) |
| rendering 100k: 1 worker vs 24 | 548.9 vs **273.9 ms** (2.0x) |
| multiplication 100k: 1 worker vs 24 | 47.4 vs **21.4 ms** (2.2x) |
| factorial 20000: reference-shaped ranges vs balanced tree | 100 vs **48 ms** |

## Hypotheses that did not survive measurement

- **Newton-reciprocal division beats long division.** 5 649 vs 91.7 ms; the later port of the
  reference's recursive reciprocal failed outright and was deleted.
- **Butterfly-level threading adds throughput.** 24.8 vs 20.9 ms; disabled.
- **A finer factorial split is better.** 512 was 50% slower than 2048.
- **The conversion target needed a faster division.** It needed the lazy representation.
- **The reference's factorial shape is the shape to copy.** It is twice as slow here.
- **Benchmarks are comparable across rounds.** They are not.
