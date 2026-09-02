# Requirements: `Lovelace.Natural` — BCD → Binary-Limb Rewrite

> Requirements and plan for replacing `Natural`'s BCD (`DigitStore`, two decimal digits per
> byte) backing store with native 64-bit binary limbs, **without changing the public API**.
> `Integer`, `Real`, `Lovelace.Suite`, the REPL, and the Studio all consume `Natural` only via
> its public surface, so this rewrite is transparent to every upper layer. The goal is a
> measured speedup on **standard hardware** (no custom silicon), with correctness pinned by the
> existing xUnit suites plus new randomized cross-checks against `System.Numerics.BigInteger`.

---

## Functionality Worktree

### Analysis Summary

`Natural` is the arithmetic core of the whole library. Today it stores values as
binary-coded decimal and runs every inner loop **one base-10 digit at a time**:

- `DigitStore` packs two decimal digits per byte (BCD). The arithmetic hot path
  (`RentDigitSnapshot`) expands that to **one digit per `byte`** (3.32 bits of entropy in an
  8-bit slot).
- `operator+`/`operator-` loop per digit with `%10` / `/10` (a division per digit).
- `operator*` uses schoolbook O(n²) below 256 digits and an exact Number-Theoretic Transform
  (`NttMultiply`) above, but the NTT packs digits into **base-10⁵ limbs — ~16.6 bits used per
  64-bit coefficient** — and its Cooley–Tukey loop is serial.
- `DivRem` uses grade-school long division below 1024 digits and a Newton reciprocal above.
- `Pow` is binary exponentiation; `Factorial` is a `Parallel.For` tree reduction.
- `Sqrt` and `Pi` (in `Real`) are Newton–Raphson and Chudnovsky/binary-splitting; both are
  multiply-bound **and** heavily bound by `ToNatural().ToString()` + `Nat.TryParse(...)`
  round-trips, so decimal↔binary conversion is on the hot path of `Real` too.

The algorithmic big-O is already modern. What remains is the constant factor, and it is dominated
by the decimal representation:

| Cost driver | BCD today | Binary limbs (target) |
|---|---|---|
| Information per arithmetic word | ~3.32 bits (1 digit) | 64 bits (~19.3 digits) |
| Inner-loop divide | `%10` / `/10` per digit | none (native carry / `BigMul`) |
| NTT coefficient width | ~16.6 bits / 64-bit slot | 64 bits / 64-bit slot |
| Memory traffic | 1 byte / digit (hot path) | ~0.42 bytes / digit |
| SIMD friendliness | none (per-digit scalar) | natural (`Vector<ulong>`, AVX2/512) |

### Target design

- **Representation** — `private ulong[] _limbs` (little-endian, base 2⁶⁴). Canonical form: no
  most-significant zero limbs; zero is the empty array. Instances stay immutable (operators
  return new instances, matching the current contract). `INumber<Natural>.Radix` stays `10`
  (the type remains decimal-exposed; the *internal* radix is 2⁶⁴).
- **Add / Sub** — 64-bit carry / borrow propagation.
- **Compare / Equals** — limb-count first, then most-significant-first limb compare.
- **Multiply** — schoolbook with `Math.BigMul` (64×64→128) below a threshold; Karatsuba above
  (exact). *(Phase 2: port the existing NTT to 64-bit limbs for very large operands.)*
- **Divide** — single-limb short division for small divisors (feeds base conversion); grade-school
  long division for small operands; Newton reciprocal (`floor(2^(2k)/d)`), exact after
  correction, for large operands (multiply-bound).
- **Base conversion (critical)** — divide-and-conquer in both directions:
  - *Parse* (decimal → limbs): recursive split `value = hi·10^(len_low) + lo`.
  - *ToString* (limbs → decimal): recursive split via `DivRem(x, 10^(half))`.
  Both are `O(M(n) log n)` and replace the current O(n²)/per-digit paths. This is a hard
  requirement because `Real.Sqrt`/`Real.Pi` round-trip through strings constantly.
- **`ShiftLeftDecimal(k)`** — multiply by 10^k (append k decimal zeros); kept, since `Real`
  calls it (`DivideNonPeriodic`).
- **`Pow` / `Factorial`** — unchanged algorithms on top of the new multiply (Factorial keeps its
  `Parallel.For` tree reduction).
- **Project seam** — `Lovelace.Natural.csproj` drops its `ProjectReference` to
  `Lovelace.Representation`; `Natural` no longer imports `Lovelace.Representation`.
  `Lovelace.Representation` remains in the solution (its own tests stay green) but becomes
  dormant w.r.t. `Natural`.
- **AOT / allocations** — keep `IsAotCompatible=true`; reuse `ArrayPool<ulong>`/`MemoryPool`
  on hot paths where it pays; prefer `UInt128` and `Math.BigMul` (no `BigInteger` in inner loops).

### Falsify Claims — Verification

| # | Claim | Evidence | Status |
|---|---|---|---|
| 1 | `DigitStore` is referenced only by `Representation`, `Representation.Tests`, and `Natural` (`Natural.cs:35` `private DigitStore _store`) | grep `DigitStore` across `*.cs` | ✅ Supported |
| 2 | `Integer` uses `Natural` only via public API (ctors, `IsZero`, `IsEvenInteger`, `IsOddInteger`, `+ − *`, `DivRem`, `Pow`, `Factorial`, `Equals`, `CompareTo`, `ToString`) | grep `_magnitude`/`Nat.` in `Integer.cs` — no `DigitStore`/`_store` access | ✅ Supported |
| 3 | `Real` uses `Natural` only via public API, including `ShiftLeftDecimal` (`Real.cs:609`), `DivRem`, `Pow`, `Parse`/`TryParse`, `ToString`, `One`, `Zero` | grep `Nat.`/`ToNatural()` in `Real.cs` | ✅ Supported |
| 4 | `Natural`'s arithmetic hot path is one decimal digit per byte with `%10`/`/10` | `Natural.cs` `operator+` (`s % 10`, `s / 10`), `RentDigitSnapshot` unpacks one digit/byte | ✅ Supported |
| 5 | `NttMultiply` packs digits into base-10⁵ limbs (~16.6 bits per 64-bit coefficient) | `Natural.cs:1033–1038` `NttLimbDigits = 5`, `NttLimbBase = 100000` | ✅ Supported |
| 6 | `Natural` exposes `public Natural ShiftLeftDecimal(long k)` used by `Real` | `Natural.cs:665`, `Real.cs:609` | ✅ Supported |
| 7 | Existing tests exercise `Natural` only through its public API | `Lovelace.Natural.Tests/*.cs` — operator/ctor/Parse/ToString functional tests | ✅ Supported |
| 8 | `bench/Program.cs` already cross-checks add/sub/mul/div against `System.Numerics.BigInteger` (`op == "check"`) | `bench/Program.cs:39–67` | ✅ Supported |

> **Zero Falsified rows.**

---

### Completeness Checklist

- [ ] Replace `DigitStore _store` with `ulong[] _limbs`; keep all public members and signatures
- [ ] Constructors: default, copy, `ulong`, `int`, `string`, `ReadOnlySpan<char>`
- [ ] `IsZero`, `IsEvenInteger`, `IsOddInteger` + remaining `INumber<T>` predicates
- [ ] `Equals` / `CompareTo` / comparison operators on limbs
- [ ] `operator+` / `operator-` (carry/borrow propagation; `-` throws on underflow)
- [ ] `operator*` — schoolbook (`Math.BigMul`) + Karatsuba
- [ ] `DivRem` (static + instance) — short division + long division + Newton reciprocal
- [ ] `operator/`, `operator%`, `++`, `--`
- [ ] `Pow` (binary exponentiation), `Factorial` (parallel tree)
- [ ] `ShiftLeftDecimal(long k)`
- [ ] `Parse` / `TryParse` (all overloads) — divide-and-conquer decimal→limbs
- [ ] `ToString` / `TryFormat` — divide-and-conquer limbs→decimal
- [ ] `DisplayDigits` / `Precision` static properties preserved
- [ ] Drop `Lovelace.Representation` reference from `Lovelace.Natural.csproj`; keep AOT-compatible
- [ ] (Phase 2) Port NTT to 64-bit limbs for very large multiply

---

## Test Plan

The existing `Lovelace.Natural.Tests`, `Lovelace.Integer.Tests`, `Lovelace.Real.Tests`,
`Lovelace.Suite.Tests`, `Lovelace.Studio.Tests`, and `Lovelace.Representation.Tests` must all
stay green unchanged (they pin the public contract). New, additive tests target the rewrite's
specific risks:

### `Natural` — binary-limb invariants (new file `NaturalBinaryLimbTests.cs`)

1. `RoundTrip_GivenRandomLargeValues_ParseToStringIdentity`
   *Assumption*: for random 1–20000-digit values, `Natural.Parse(n.ToString()) == n`.

2. `Add_GivenCarryAcrossLimbBoundary_ProducesCorrectResult`
   *Assumption*: adding values that force a carry out of the most-significant 64-bit limb (e.g.
   `(2^64 − 1) + 1`) yields exactly `2^64` as a 2-limb value.

3. `Sub_GivenBorrowAcrossLimbBoundary_ProducesCorrectResult`
   *Assumption*: `2^64 − 1` equals `18446744073709551615` (borrow across limbs).

4. `Mul_GivenLimbOverflow_ProducesCorrectResult`
   *Assumption*: `(2^64 − 1) × (2^64 − 1)` equals the exact 128-bit product `2^128 − 2^65 + 1`.

5. `DivRem_GivenLargeOperands_ProducesCorrectQuotientAndRemainder`
   *Assumption*: `DivRem(a, b, out r)` on 2000-digit operands matches `BigInteger.DivRem`.

6. `ShiftLeftDecimal_GivenK_AppendsKDecimalZeros`
   *Assumption*: `n.ShiftLeftDecimal(k).ToString() == n.ToString() + new string('0', k)`.

7. `Factorial_GivenLargeValue_MatchesBigIntegerReference`
   *Assumption*: `new Natural(100).Factorial()` matches the known 100! value.

### `Natural` — randomized cross-check (new file `NaturalRandomizedCrossCheckTests.cs`)

8. `CrossCheck_GivenRandomOperands_MatchesBigInteger`
   *Assumption*: for N random cases per size tier (1, 10, 100, 1000, 10000 digits), `+`, `−`
   (when defined), `×`, `DivRem`, `%`, `Pow` (small exponent) all match `BigInteger` exactly.

9. `CrossCheck_GivenAdversarialCarryPatterns_MatchesBigInteger`
   *Assumption*: values like `10^k − 1`, `2^64 − 1`, `10^k`, and their sums/products match
   `BigInteger` (exercises carry/borrow and base-conversion edges).

---

## Benchmark Plan

The existing `bench/Program.cs` (ops `add|sub|mul|div|pow|factorial|tostring|parse|pi|check`)
is run **twice — once against the BCD baseline (main) and once against the binary-limb build
(worktree)** — in `Release`, same machine, warmed up, and the `RESULT <op> <digits> mean ms`
lines are compared into a speedup table.

| Op | Digit sweep (suggested) | Notes |
|---|---|---|
| `add`, `sub` | 1k, 10k, 100k, 1M | O(n), memory-bound — expect ~10–60× |
| `mul` | 100, 1k, 10k, 50k | schoolbook/Karatsuba region — expect ~10–100× |
| `div` | 100, 1k, 10k | Newton reciprocal — expect ~5–50× |
| `pow` | small base, exp ≤ 2000 | binary exponentiation |
| `factorial` | n ≤ 5000 | parallel tree reduction |
| `tostring`, `parse` | 1k, 10k, 100k | divide-and-conquer base conversion — expect ~10–50× |
| `pi` | 100, 1k, 10k | multiply + parse/ToString bound |
| `check` | 1, 10, 100, 1000, 10000 | correctness gate before any timing |

Success criteria: (1) all six test projects green on the binary-limb build; (2) `bench check`
passes at all sizes; (3) a measured, reproducible speedup table with a written explanation of
each regime (including any size where the missing Phase-2 NTT would matter).
