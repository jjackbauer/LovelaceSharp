# Requirements: `Lovelace.Natural` — Parse/ToString decimal-cache upgrade

> Requirements and plan for closing the last measured gap between the binary-limb `Natural`
> and the BCD baseline: decimal↔binary conversion. The binary build wins every arithmetic
> operation (add/sub/mul/div/pow/factorial/pi), but `Parse` and `ToString` are ~6× and ~11×
> slower than BCD because BCD is decimal-native (O(n) nibble pack/unpack) while a binary
> working form must convert bases (O(M(n)·log n)). This upgrade caches both representations so
> the conversion happens at most once per value, not once per operation.

---

## Functionality Worktree

### Analysis Summary

`Natural` today holds **only** `private readonly ulong[] _limbs` (binary limbs, base 2⁶⁴):

- **Parse** (`TryParse` → `ParseDigits` → `ParseDigitsPair`) is divide-and-conquer,
  `O(M(n)·log n)` with the Karatsuba/NTT multiply. `Natural(string s)` runs it and then
  **discards the decimal text** (`_limbs = Parse(s, null)._limbs;`).
- **ToString** (`ToString` → `ToStringRecursive`) is divide-and-conquer: it splits the value
  at `10^half` via `DivRem` and recurses. `DivRem` is Knuth `O(n·m)` below 262144 limbs, so
  `ToString` is effectively `O(n²)` at practical sizes.
- The class is otherwise immutable; both forms are recomputed from scratch every time a value
  crosses the decimal↔binary boundary.

The BCD baseline (`DigitStore`) is decimal-native: parse is a per-character `SetDigit` (O(n)),
and `ToString` is a parallel nibble→char unpack (O(n)). Measured at 100 000 digits:

| Op | BCD (main) | binary | ratio |
|---|---|---|---|
| parse | 2.1 ms | 13.7 ms | 6.5× slower |
| tostring | 4.7 ms | 53 ms | 11× slower |

No algorithmic improvement can make a binary representation beat BCD's O(n) pack/unpack — the
win must come from **avoiding the conversion**, i.e. caching the decimal form (and optionally
making the limb conversion lazy).

### Falsify Claims — Verification

| # | Claim | Evidence | Status |
|---|---|---|---|
| 1 | `Natural` stores only `readonly ulong[] _limbs`; no decimal form is retained | `Natural.cs:36` `private readonly ulong[] _limbs;` | ✅ Supported |
| 2 | `Natural(string)` discards the input text after parsing | `Natural.cs:115` `public Natural(string s) => _limbs = Parse(s, null)._limbs;` | ✅ Supported |
| 3 | Parse is recursive divide-and-conquer (`ParseDigitsPair`), base case ≤ 19 digits | `Natural.cs:1306–1332` | ✅ Supported |
| 4 | `ToString` is divide-and-conquer via `DivRem` by `10^half` (`ToStringRecursive`) | `Natural.cs:510–527` | ✅ Supported |
| 5 | `DivRem` is Knuth O(n·m) for combined sizes &lt; 262144 limbs | `DivNewtonThreshold = 1L << 18` | ✅ Supported |
| 6 | BCD parse is a per-character `SetDigit` loop (O(n)) | main `Natural.TryParse` | ✅ Supported |
| 7 | BCD `ToString` snapshots bytes and unpacks nibbles in `Parallel.For` (O(n)) | main `DigitStore.ToString` | ✅ Supported |
| 8 | `GetHashCode()` is `ToString().GetHashCode()`, so it pays the conversion today | `Natural.cs` `GetHashCode` | ✅ Supported |

> **Zero Falsified rows.**

### Design — two additive options

**Option A — Cached decimal string (limbs stay primary).** The smallest change, and it removes
the 11× `ToString` gap for the dominant "parse once, print" pattern.

- Add `private string? _decimal;` — a lazily-computed, thread-safe cache of the canonical
  decimal digits (mutating only this field; the rest stays immutable).
- Constructors from text (`Natural(string)`, `Natural(ReadOnlySpan<char>)`, `TryParse`) set
  `_decimal` to the canonical (leading-zero-stripped) digit string alongside `_limbs`.
- `ToString()` returns `_decimal` if present, else computes `ToStringRecursive(_limbs)` and
  caches via `Interlocked.CompareExchange`. `GetHashCode()`/`TryFormat`/`ToString("N")` all
  reuse it.
- Copy ctor copies `_limbs` and shares `_decimal` (strings are immutable).
- Does **not** touch parse cost (parse still converts to limbs) and does not help the
  "compute once, print many" first print (an arithmetic result has no cached string until its
  first `ToString`).

**Option B — Lazy limbs (decimal string primary).** The complete fix; also removes the 6.5×
`parse` gap, at the cost of touching every limb access.

- Make `_limbs` nullable (`ulong[]?`) and add `_decimal`. Invariant: at least one form is
  non-null; zero is `"0"` / empty limbs.
- `Natural(string)`/`TryParse` store **only** the string (O(n)); limbs are computed lazily.
- Introduce `GetLimbs()` (`_limbs ??= ParseToLimbs(_decimal!)`) and `GetDecimal()`
  (`_decimal ??= ComputeDecimal(_limbs!)`), both thread-safe and lazy.
- Every internal `_limbs` read (Equals, CompareTo, the arithmetic operators, `DivRem`, `Pow`,
  `Factorial`, `ShiftLeftDecimal`, `IsZero`, `IsOddInteger`, `ToStringRecursive`) routes
  through `GetLimbs()`. `IsZero`/`IsOddInteger` get cheap string/fast paths so they never
  force a conversion.
- Arithmetic results are created limb-first (`_decimal == null`); their first `ToString`
  computes and caches the string, so "compute once, print many" is O(M(n)·log n) once then
  O(1).

**Recommendation.** Ship A first (low risk, ~6 call sites, closes the tostring gap), then B
(incremental refactor, closes the parse gap). The end state is the hybrid: both forms cached
lazily, each conversion paid at most once.

### Completeness Checklist

- [ ] (A) Add `private string? _decimal;` cache field to `Natural`
- [ ] (A) Populate `_decimal` in `TryParse` / text constructors with the canonical string
- [ ] (A) `ToString()` returns cached `_decimal` (compute-and-cache otherwise), thread-safe
- [ ] (A) `GetHashCode()` / `TryFormat` / `ToString("N")` reuse the cached string
- [ ] (A) Copy constructor shares `_decimal`
- [ ] (B) Make `_limbs` nullable; add `GetLimbs()` lazy accessor
- [ ] (B) `Natural(string)` / `TryParse` store only the decimal string (lazy limbs)
- [ ] (B) Route all internal `_limbs` reads through `GetLimbs()`
- [ ] (B) Cheap `IsZero` / `IsOddInteger` paths that avoid forcing conversion
- [ ] (B) Thread-safety for both lazy caches (lock-free compute + `Interlocked.CompareExchange`)

---

## Test Plan

The existing suites (`Natural`, `Integer`, `Real`, `Representation`, `Suite`, `Studio`) must
stay green — they pin the public contract and already cover parse/round-trip/format. New tests
target the cache's two risks: correctness of the lazy conversion and thread-safety.

### `Natural` — cached conversion (new file `NaturalDecimalCacheTests.cs`)

1. `ParseToString_GivenLeadingZeros_ReturnsCanonicalString`
   *Assumption*: `new Natural("007").ToString()` is `"7"` (the cache stores the canonical form,
   not the raw input).

2. `ToString_GivenParsedValue_IsStableAcrossRepeatedCalls`
   *Assumption*: calling `ToString()` twice on one instance returns the identical string object
   (reference equality) after the cache is populated.

3. `RoundTrip_GivenRandomLargeValues_ParseToStringIdentity`
   *Assumption*: for random 1–20000-digit values, `new Natural(s).ToString() == s` (canonical).

4. `ArithmeticResult_ToString_CachesAfterFirstCall`
   *Assumption*: `(a * b).ToString()` matches `BigInteger`'s decimal form, and a second call is
   reference-equal to the first (cache populated).

5. `GetHashCode_GivenEqualValues_AreEqual`
   *Assumption*: two separately-parsed equal values produce equal hash codes (via the cached
   string, so hashing is consistent with `Equals`).

6. `CopyConstructor_SharesCachedStringButIndependentLimbs`
   *Assumption*: a copy of a parsed value returns the same `ToString()` content; mutating the
   original's cache cannot affect the copy's value.

### `Natural` — lazy conversion thread-safety

7. `ConcurrentToString_GivenSharedValue_ReturnsConsistentString`
   *Assumption*: `Parallel.For` over many threads calling `ToString()` on one shared parsed
   value all observe the same string (no torn/partial cache).

8. `ConcurrentParseToString_GivenManyValues_MatchesBigInteger`
   *Assumption*: parsing many random values concurrently and formatting them all match
   `BigInteger` (exercises the lazy limbs path under contention).

9. `ConcurrentArithmeticOnLazyParsedValue_MatchesBigInteger`
   *Assumption*: a value parsed lazily (limbs not yet materialized) used concurrently in
   `+`/`*` produces correct results (the first `GetLimbs()` call is race-free).

---

## Benchmark Plan

Re-run the existing `bench` `parse` / `tostring` ops (and `mulbench check` for correctness)
against BCD (`main`) and the upgraded binary build, comparing:

| Op | digits | before | target |
|---|---|---|---|
| tostring | 100k | 53 ms | ~0 (cached string) |
| parse | 100k | 13.7 ms | ~2 ms (lazy limbs, Option B) |

Success criteria: (1) all suites green; (2) `tostring` on parsed values is O(1); (3) `parse`
drops to the string-copy bound under Option B; (4) the "compute once, print many" pattern pays
one O(M(n)·log n) conversion then O(1); (5) memory overhead of the cache is documented
(~2 bytes/digit for the UTF-16 string, ~0.42 bytes/digit for limbs).
