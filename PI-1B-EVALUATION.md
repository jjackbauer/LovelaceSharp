# Evaluating the path to 1,000,000,000 digits of π in LovelaceSharp

**Scope:** an assessment of what it would take for the current `Lovelace.Real.Pi(long)`
to reach 1 billion decimal digits "in a reasonable time", grounded in the actual
code and in measurements taken on the development machine.

**Machine used for measurements:** AMD Ryzen 9 5900X (12 cores / 24 threads), 64 GB RAM,
.NET 10.0.103.

---

## 1. Verdict (TL;DR)

The algorithm is already right; the **number representation and the multiplication
engine are not**. π is computed today by the Chudnovsky series with binary splitting —
which is asymptotically optimal (`O(M(n)·log n)`, where `M(n)` is multiplication cost) —
so the *formula* is a solved problem. The entire obstacle to 1 billion digits is that
every big integer in the system is stored **base-10, two decimal digits per byte**, and
multiplication tops out at a **single-prime NTT capped at 2²³ points (~8.4 M digits)**,
above which it falls back to **Karatsuba (`O(n^1.585)`)** — which is hopeless at this scale.

**As written today, `Pi(1_000_000_000)` is not achievable at all** — not just slowly,
but structurally: it would first hit an `int` overflow in the digit-array plumbing
(~2.1×10⁹ digits is the hard ceiling on any single `byte[]`), and long before that it
degrades to Karatsuba and, empirically, the current code already terminates abnormally
around the 1-million-digit mark.

Reaching 1 billion digits "in a reasonable time" (say minutes to a few hours on this
machine) requires, in order of leverage:

1. **Binary limbs (base 2⁶⁴) for the arithmetic core** instead of BCD digits — the single
   highest-impact change.
2. **A real fast-multiply engine**: multi-prime CRT NTT or an FFT / Schönhage–Strassen
   (SSA) convolution, with no ~8 M-digit ceiling.
3. **Removing the per-operation overhead** (locks, `List<byte>`, snapshot copies,
   string round-trips) and fixing the `int` length overflows.
4. **Streaming output** (the final `ToString` of a 1-billion-digit number must not
   build a 2 GB `string`).

With those in place, a clean C# implementation on this 24-thread box should reach
1 billion digits in roughly **1–4 hours**; binding to a tuned native library (GMP/MPFR)
or bundling a y-cruncher-class backend would bring it to **minutes**.

---

## 2. What the code does today

| Concern | Where | Current state |
|---|---|---|
| Storage | `Lovelace.Representation/DigitStore.cs` | BCD: **2 decimal digits per byte**, in a `List<byte>`; the only project allowed to touch the raw `byte[]`. |
| Add / Sub / Mul operands | `Lovelace.Natural/Natural.cs` | Arithmetic runs on **unpacked `byte[]` (1 decimal digit per element)**, snapshotted under a `lock` (`RentDigitSnapshot`), then re-packed via `SetDigitsBulk`. |
| Multiply | `Natural.cs:841–1137` | Schoolbook ≤ 1024 digits → **Karatsuba** 1024–8192 → **NTT** ≥ 8192. |
| NTT | `Natural.cs:1021–1137` | **Single prime 998244353** (=119·2²³+1), max transform length `MaxNttLength = 1<<23 = 8,388,608`. Used only when `aLen + bLen ≤ 8,388,608`. |
| Divide | `Natural.cs:1147–1361` | Newton reciprocal, `O(M(n)·log n)` — actually well-designed. |
| √ | `Lovelace.Real/Real.cs:754–852` | Newton–Raphson, progressive precision doubling, seeded from `double`. |
| π | `Real.cs:878–956` + `PiSegment` at `998–1033` | Chudnovsky, **binary splitting (BSP)**, parallelized over ≤ 64 term sub-ranges, then merged left-to-right serially. Guard digits `+10`. |
| Digit cap | `Real.cs:44, 880` | `MaxComputationDecimalPlaces` defaults to **1000**; `Pi` reads `_maxComputationDecimalPlaces` directly, so `pi(10⁹)` throws out of the box. `setprecision(n)` raises it. |

Every number in the tower (`Natural → Integer → Real`) inherits the base-10 digit store,
and the `Real` type is *fixed-point decimal* (`Exponent`), so π is materialised as a
`Natural` of `digits+1` base-10 digits and printed by `DigitStore.ToString()`.

---

## 3. Measurements (this machine)

`bench` project, Release build, `RESULT` lines are the in-process medians (JIT-warmed,
excluding operand construction):

### π (single computation)

| digits | time | factor per 10× |
|---|---|---|
| 1 000 | 201 ms | — |
| 10 000 | 3.0 s | 14.9× |
| 100 000 | 34.5 s | 11.5× |
| 200 000 | 77.4 s | — |
| 400 000 | 135.2 s | — |
| 500 000 | **> 600 s** (not finished in 10 min) | — |
| 1 000 000 | **terminates abnormally** (exit −1, no output) | — |

The sub-linear growth from 100 k → 400 k (3.9× time for 4× digits) shows the NTT is
doing its job in that window. Then the behaviour collapses: **400 k → 500 k jumps from
135 s to >600 s** (a >4.4× cost for a 1.25× size increase — not NTT behaviour), and
1 M exits abnormally. This is a *hard wall*, not a graceful slowdown.

### Multiplication (isolated, a×b with b = a/2 digits)

| a digits | time |
|---|---|
| 10 000 | 34 ms |
| 100 000 | 64 ms |
| 200 000 | 133 ms |
| 400 000 | 293 ms |
| 800 000 | 609 ms |
| 4 000 000 (a×b/2) | **terminates abnormally** (exit 1, no result) |

Clean `O(n log n)` NTT behaviour up to ~1 M combined digits, then the multiply itself
**crashes** in the 1–4 M-operand range (a 4 M × 2 M multiply fails; this is still well
under the 2²³ NTT length cap, so the failure is a bug, not a ceiling). The operands for a
1-billion-digit π are ~1000× larger than anything that currently works.

---

## 4. Why 1 billion digits is out of reach as written

### 4.1 The intermediates are bigger than the answer

Chudnovsky gives ~14.1816 digits per term (`log10(640320³·27 / 6⁶) = 14.1816`), so

```
N ≈ 10⁹ / 14.1816 ≈ 7.05 × 10⁷ terms
```

Binary splitting builds the exact integers `P = ∏a_k`, `Q = ∏b_k`, `T = numerator`,
with `b_k = (3k)(3k−1)(3k−2)·k³·640320³ ≈ 27·k⁶·640320³`. Therefore

```
log10(Q) = Σ log10(b_k) ≈ 6·N·log10(N) + 16.2·N
```

which, at `N = 7.05×10⁷`, is **≈ 4.5 × 10⁹ digits**. `P` and `T` are the same order.
So a "1 billion digit" π actually requires multiplying integers of **~4.5 billion
decimal digits** (~232 M 64-bit limbs; top-level FFT length ≈ 2²⁹). That inflation
factor (`≈ 0.42·log10(D) + 0.66`) is a property of the Chudnovsky BSP, not a bug — but
it is why the multiplication engine, not the formula, is the whole game.

### 4.2 Base-10 BCD is the master constant factor

- Density: BCD stores 2 digits/byte vs. binary base-2⁶⁴ storing ~19.27 digits/8 bytes —
  only ~19 % denser, so **memory is not** the dominant issue.
- **The dominant issue is arithmetic.** A 64×64→128 hardware multiply covers ~19×19
  decimal digits of work in one instruction. The base-10 schoolbook/Karatsuba/NTT here
  multiplies **single decimal digits** with a `% 10` per step. For the schoolbook leaf
  (≤1024 digits) this is a ~10–50× constant penalty, and it inflates NTT sizes (base-10
  coefficients need bigger transforms than base-2⁶⁴ limbs for the same number).

### 4.3 The single-prime NTT has a hard ceiling

`NttMultiply` requires `aLen + bLen ≤ 8,388,608` (2²³). In the BSP, the top-level merge
multiplies two ~`Q/2`-sized operands, i.e. combined `≈ 4.5·D` digits. That stays under
the cap only while `D ≤ ~1.86 M` digits. The final divisions are capped at `D ≤ ~4.2 M`.
Above those points the code falls back to **Karatsuba at base-10**, `O(n^1.585)`, which
at `n = 4.5×10⁹` digits is `≈ 10¹⁴` digit-operations per top-level multiply — weeks-to-
months per full π.

### 4.4 `int` overflow blocks the goal outright

Multiple hot-path casts are 32-bit: `RentDigitSnapshot` does `(int)_digitCount`,
`SetDigitsBulk` does `int byteCount = (len+1)/2`, `NttMultiply` does `int need = aLen+bLen`,
and `SchoolbookMultiply` does `new byte[aLen + bLen + 1]`. All overflow at ~2.1×10⁹ digits —
below the ~4.5×10⁹-digit intermediates that 1-billion-digit π produces. Even with an
unbounded NTT, this plumbing would break first.

### 4.5 Per-operation overhead (a large but fixable constant)

- `SetDigit`/`SetBitwise` take a `Monitor` lock **per digit**; parsing a large literal
  is therefore ~tens of millions of uncontended lock acquisitions.
- `List<byte>` growth, snapshot copies, and `SetDigitsBulk` re-packing happen on every
  arithmetic result.
- `Pi`/`Sqrt` truncation uses `ToNatural().ToString()` → `TryParse` round-trips (a
  full string build+parse of the entire result) just to drop guard digits.
- `Pi` merges its ≤64 BSP sub-results **serially left-to-right** (`Real.cs:918–927`),
  leaving the biggest multiplications unparallelised at the merge level.

### 4.6 Output is not free

`DigitStore.ToString()` allocates a `char[digitCount]` and a `string`. For 10⁹ digits
that is a **2 GB** `char[]`/`string` — at the .NET default 2 GB object-size limit, and it
would need to be replaced by a chunked/streaming base conversion.

### 4.7 Empirical: a hard wall between 400 k and 500 k digits

`Pi(400_000)` completes in 135 s; `Pi(500_000)` does not finish in 10 minutes (a >4.4×
cost for 1.25× the digits); `Pi(1_000_000)` terminates abnormally (exit code −1, no
stderr, consistent with an uncatchable fault — e.g. stack overflow or OOM). This cliff is
**not** explained by the single-prime NTT ceiling alone: my size model puts the BSP and
division operands for `Pi(1_000_000)` at ~3–5 M digits (still under the 2²³ cap), so the
expected cost is minutes, not a wall. The wall is therefore a **suspected bug** (an
unexpected super-linear path or an unbounded recursion/overflow in the large-number
code), and it must be root-caused *before* any optimisation work — it is an independent
correctness/robustness issue on top of the structural performance ceiling. The failure
also reproduces in **isolated multiplication** (`mul 4 M × 2 M` exits with code 1 and no
result), which points at the large-operand NTT path itself rather than at π-specific code.

---

## 5. Optimization roadmap (ranked by leverage)

### Tier 0 — unblock the API (minutes)
- Let `pi()` honour any requested digit count without the 1000-digit default gate
  (`Real.cs:880` reads the raw field; `setprecision` already exists). Trivial, but
  necessary for any large run.

### Tier 1 — lift the ceiling without changing representation (days)
Target: make ~10⁷–10⁸ digits feasible; no architectural break.

1. **Multi-prime CRT NTT.** Replace the single 998244353 prime with three ~61-bit
   NTT-friendly primes and CRT-reconstruct the convolution. This removes the 2²³ length
   ceiling (each prime still needs a power-of-two length, but three primes raise the
   coefficient bound so huge lengths work) — or, equivalently, switch to a negacyclic
   convolution to halve the length.
2. **Toom-Cook between Karatsuba and NTT** (and retune thresholds at 32-bit limb sizes
   if Tier 2 lands).
3. **Fix the `int` overflows** (`long`/nint lengths, checked where appropriate) so
   operands beyond 2.1×10⁹ digits are representable.
4. **Remove string round-trips** in `Pi`/`Sqrt` (truncate guard digits by slicing the
   digit array, not by `ToString`+`Parse`).
5. **Balanced parallel BSP merge** — pairwise tree reduction of the sub-range triples
   instead of the serial left-to-right fold, and spread the top-level multiplications
   across cores.
6. **Streaming/chunked `ToString`** for huge values.

### Tier 2 — the decisive change: binary limbs + FFT (weeks)
This is the only path that actually reaches 1 billion digits "in a reasonable time."

- Introduce a **binary big-integer core** (`UInt64` limbs, little-endian), used by
  `Natural` for arithmetic, with decimal retained **only** at the parse/format boundary.
  This is a deliberate break with `DigitStore`'s "only project touching raw BCD bytes"
  contract — that contract is exactly what caps performance — so it should be done as a
  new internal engine with `Natural` kept as the public face.
- Implement **FFT multiplication**: either a 3-prime CRT NTT (~2⁵⁷–2⁶¹ primes) or a
  floating-point FFT with coefficient splitting / Schönhage–Strassen. At ~232 M limbs,
  the top convolution is ~2²⁹ points.
- Expected result on this machine (12 C / 24 T, no AVX-512): **1–4 hours for 10⁹ digits**,
  several GB to low-tens-of-GB peak memory (fits in 64 GB).

### Tier 3 — pragmatic shortcut (days)
- P/Invoke **GMP/MPFR** (`mpfr_const_pi`) — or bundle a **y-cruncher**-class native
  binary — behind `Real.Pi` for the large case. This reaches 10⁹ digits in **minutes**
  (y-cruncher does it in ~6 s on a dual-socket server; low single-digit minutes on a
  high-end desktop). Cost: it abandons the self-contained Native-AOT / no-dependency
  ethos and moves the "trusted" core outside the verified C#/Lean boundary.

### Not worth doing
- **Switching the π formula.** Chudnovsky is already asymptotically optimal. Gauss–
  Legendre AGM is also `O(M(n) log n)` but with a ~4–5× larger constant; Machin-style
  arctan is `O(M(n) log²n)` (worse); BBP/hex spigot is for extracting a *single* digit,
  not bulk computation. Keep Chudnovsky.
- **Micro-optimising the base-10 schoolbook** before Tier 2 — it cannot rescue the
  `O(n^1.585)` cliff at 4.5×10⁹ digits.

---

## 6. What "reasonable time" actually looks like

| Approach | 10⁹ digits, this machine | Notes |
|---|---|---|
| Current code (BCD + single-prime NTT) | **infeasible** (int overflow + Karatsuba cliff; crashes before 10⁶) | — |
| Tier 1 only (CRT-NTT, keep base-10) | ~10⁷–10⁸ digits ceiling; 10⁹ still not practical | ~day(s) at best, still BCD-bound |
| Tier 2 (binary limbs + FFT/CRT-NTT), clean C# | **~1–4 hours** | the "native to the repo" answer |
| Tier 3 (GMP/MPFR or y-cruncher backend) | **~minutes** | fastest, but external dependency |

For scale: the standing record is ~314 trillion digits (Dec 2025) computed over ~110 days
on a single large machine — so 10⁹ digits is a *modest* target by the field's standards;
it is only large relative to the current base-10 BCD engine.

---

## 7. Recommendation

1. **Root-cause the `Pi(1_000_000)` crash first** (Tier 0 + a debug session); it is a
   robustness bug independent of performance.
2. **Land Tier 2's binary-limb core + one FFT multiplier** as a focused internal project,
   because it is the only change that makes 10⁹ digits achievable natively; do Tier 1's
   CRT-NTT, overflow fixes, and output streaming as part of the same effort (they are
   prerequisites or near-free wins).
3. If time-to-demo is the dominant criterion, **ship Tier 3 (GMP/MPFR)** as a
   "fast path" now, and keep the pure C# Chudnovsky+BSP as the correctness reference and
   the small-size engine.
4. Preserve the repo's proof story: the Lean proofs are over the base-`b` schoolbook
   arithmetic (representation-agnostic), so they remain valid for the digit engine; the
   new FFT/limb path should be verified by **differential testing** against
   `System.Numerics.BigInteger` and known π digits (the `bench check` / `bench verify`
   scaffolding already does exactly this).

### Bottom line
The π formula is done. To reach 1 billion digits in reasonable time, replace the
base-10 digit store with 64-bit binary limbs and give the multiply path a real
FFT/NTT with no 8-million-digit ceiling; everything else is plumbing. On this machine
that lands somewhere between **minutes (with GMP) and a few hours (pure C#)** — versus
"structurally impossible" today.
