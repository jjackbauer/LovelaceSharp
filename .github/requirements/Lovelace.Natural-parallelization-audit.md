# Parallelization Audit  `Lovelace.Natural.Natural`

## 1. Data Mutability Table

| Member | Type | Mutable? | Shared across calls? | Thread-safety risk |
|---|---|---|---|---|
| `_store` (`DigitStore`) | reference type | Yes  `SetDigit`, `TrimLeadingZeros` called on it | Between threads sharing the same `Natural` instance | **Low**  `DigitStore` guards every read/write with its own `_syncRoot` lock |
| `_displayDigits` (static `long` backing field) | value type | Yes  set via `Interlocked.Exchange` | All threads/instances | **None**  `Interlocked.Read`/`Exchange` used consistently |
| `_precision` (static `long` backing field) | value type | Yes  set via `Interlocked.Exchange` | All threads/instances | **None**  same guard as `_displayDigits` |
| `DisplayDigits` / `Precision` (static properties) | `long` | Yes  public setters | All threads/instances | **None**  backed by `Interlocked` fields |
| `One`, `Zero`, `AdditiveIdentity`, `MultiplicativeIdentity` | `Natural` | No  each access constructs a new instance | N/A | **None** |
| `Radix` | `int` constant | No | N/A | **None** |

**Key architectural observation:** All arithmetic operators follow a purely functional pattern  they allocate and return a *new* `Natural()` without mutating any operand. Mutations to the freshly-allocated `result._store` occur before the object is published (returned), making them invisible to other threads. The only genuine sharing risk lies in the two static `long` fields and in any caller that holds a shared reference to the same `Natural` instance.

## 2. Sequential Dependency Graph

| Method / Loop | Reads shared state | Writes shared state | Calls mutating methods on `this` | Loops w/ independent iterations? |
|---|---|---|---|---|
| `Natural()` / `Natural(Natural)` / `Natural(ulong)` / `Natural(int)` | None / `other._store` | `_store` (new instance) | None | `ulong`/`int` ctor: No  `SetDigit` position constraint requires sequential ascending writes |
| `Natural(string)` / `Natural(ReadOnlySpan<char>)` | None | Delegates to `TryParse` | None | See `TryParse` |
| All `Is*` predicates | `_store.IsZero`, `_store.GetDigit(0)` | None | None | No loop / O(1) |
| `Abs`, `MaxMagnitude`, `MinMagnitude`, `*MagnitudeNumber` | `_store` | None on operands | None | No loop |
| `Equals(Natural?)` | `_store.IsZero`, `_store.DigitCount`, `_store.GetDigit(i)` | None | None |  Read-only scan  but `&&` short-circuit makes PLINQ counter-productive for typical sizes |
| `CompareTo(Natural?)` | Same as `Equals` | None | None |  Read-only scan  same short-circuit issue |
| `GetHashCode` | `_store` via `ToString()` | None | None | Parallelized inside `DigitStore.ToString(char)` |
| `operator +` carry loop | `left._store.GetDigit(c)`, `right._store.GetDigit(c)` | `result._store.SetDigit(c, )` (private) | None | **No**  carry chain: `carry` from position `c` feeds position `c+1` |
| `operator -` borrow loop | `left._store.GetDigit(c)` | `result._store.SetDigit(c, )` (private); `result._store.TrimLeadingZeros()` afterward | None | **No**  borrow chain |
| `operator *` outer partial-product loop (sequential path) | `aux._store.GetDigit(c)`, `aux1._store.DigitCount`, `aux1._store.GetDigit(c2)` | `temp._store.SetDigit()` (private); `result` accumulated | None | Each `temp` is independent; accumulation into `result` is serial |
| `operator *` outer loop (parallel path) | `aux._store.GetDigit(c)` (lock per call!), `aux1._store.DigitCount` (lock), `aux1._store.GetDigit(c2)` (lock per call inside Parallel.For!) | `partials[c]` (non-overlapping array slot per lambda) | None |  Each lambda owns a distinct slot; **but lock contention on `aux1._store._syncRoot` is O(outerCount � innerCount) per run** |
| `operator *` partial-products serial combination | `partials[c]` | `result` | None | **No**  `result += partials[c]` must run serially |
| `operator *` inner digit loop (overflow chain) | `aux1._store.GetDigit(c2)` | `temp._store.SetDigit(c2+c, )` | None | **No**  overflow chain |
| `DivRem` bring-down loop | `left._store.GetDigit(i)`, `partial` | `partial`, `quotientDigits[]` | `BringDownDigit`, `right * new Natural(k)` (�up to 9 per digit) | **No**  `partial` fed from prior iteration |
| `DivRem` trial-quotient inner loop (k=1..9) | `right`, `partial` | `q`, `qTimesDivisor` | `right * new Natural((ulong)k)`  **allocates and multiplies a new Natural for each k, every digit** |  9 independent candidates  too small to parallelize; **eliminate via precomputation** |
| `BringDownDigit` | `partial._store.GetDigit(i)` | `result._store.SetDigit()` (private) | None | **No**  sequential position writes |
| `Pow` binary-exponentiation loop | `e._store`, `result`, `b._store` | `result`, `b` (reassigned each step) | None | **No**  each step requires prior `result` and `b` |
| `Factorial` sequential path | `this._store`, `result`, `aux._store` | `result` (reassigned each step) | None | **No** as coded  splittable into sub-ranges (see Phase 2) |
| `Factorial` parallel path  `Parallel.For` body | `n` (read-only `ulong`) | `partials[i]` (non-overlapping slot) | None |  Each sub-range independent |
| `Factorial` parallel path  serial combination | `partials` | `result` | None | **No**  serial `foreach` |
| `TryParse` digit-write loop | Input `ReadOnlySpan<char>` (read-only) | `n._store.SetDigit(i, )` (private, new instance) | None | Data-independent per iteration; **blocked** by `DigitStore.SetDigit` sequential-position guard |
| `ToString()` / `ToString(string?, )` / `TryFormat` | `_store` | None | Delegates to `DigitStore.ToString(char)`  **already parallelized inside** |  No work needed here |

## 3. Falsify Claims Result

| # | Claim | Evidence | Status | Reason |
|---|---|---|---|---|
| 1 | All arithmetic operators create a fresh `Natural()` result and never mutate `this` or any operand | Every operator opens with `var result = new Natural()` or an equivalent new-instance expression |  Supported | Confirmed throughout `+`, `-`, `*`; `/` and `%` delegate to `DivRem` which also builds fresh locals |
| 2 | `operator +` carry loop is data-dependent across iterations | `carry = sum / 10` from position `c` consumed at `c+1` |  Supported | Classic BCD carry chain |
| 3 | `operator -` borrow chain is data-dependent across iterations | `carry = current / 10`; next iteration uses `(1 - carry)` |  Supported | Classic BCD borrow chain |
| 4 | Each `operator *` outer iteration constructs an independent `temp = new Natural()` partial product | `var temp = new Natural()` inside the outer `for` / `Parallel.For` lambda |  Supported | No cross-iteration sharing of `temp` |
| 5 | The accumulation `result += partials[c]` in `operator *` is serial | Done in a `for` loop after `Parallel.For` returns |  Supported | O(outerCount) serial additions |
| 6 | Inside `operator *`'s `Parallel.For`, every `aux1._store.GetDigit(c2)` call acquires `aux1._store._syncRoot`, causing lock contention proportional to `outerCount � aux1.DigitCount` | `GetDigit` body: `lock (_syncRoot) {  }`; called in every inner iteration of every lambda |  Supported | High-contention hot path |
| 7 | `DivRem`'s trial-division loop runs up to 9 multiplications per dividend digit (`right * new Natural((ulong)k)` for k = 1..9) all of which could be precomputed once before the outer loop | `DivRem`  `for (byte k = 1; k <= 9; k++) { var candidate = right * new Natural((ulong)k); if (candidate <= partial) {...} else break; }` inside outer `for (i = n-1; i >= 0; i--)` |  Supported | O(9 � digitCount(right)) work repeated for every dividend digit |
| 8 | `Factorial`'s serial combination `foreach (var p in partials) result *= p` is O(t) multiplications where t = `ProcessorCount` | `Factorial` body: `foreach (var p in partials) result *= p;` |  Supported | Could be O(log t) with tree reduction |
| 9 | `operator *` sequential path is used when `outerCount  processorCount * 2`; parallel path when larger | `if (outerCount > processorCount * 2)` branch |  Supported | Threshold confirmed |
| 10 | `TryParse` digit-write loop cannot be parallelized under the current `DigitStore.SetDigit` API: position must be  DigitCount | `DigitStore.SetDigit`  `if (position < 0 || position > _digitCount) throw new ArgumentOutOfRangeException()` |  Supported | Hard sequential-write constraint; parallel random-access writes require a new API |
| 11 | `ToString` on `Natural` delegates everything to `DigitStore.ToString(char)` which already runs `Parallel.For` on interior bytes | `Natural.cs`  `return _store.ToString()`; `DigitStore.cs`  `Parallel.For(0L, lastByteIdx, c => {  })` |  Supported | No additional work needed |
| 12 | `DisplayDigits` and `Precision` use `Interlocked.Read`/`Interlocked.Exchange` on explicit `long` backing fields, ensuring atomic 64-bit access on 32-bit runtimes | `Natural.cs`  `private static long _displayDigits = -1L; public static long DisplayDigits { get => Interlocked.Read(ref _displayDigits); set => Interlocked.Exchange(ref _displayDigits, value); }` |  Supported | Thread-safe on both 32-bit and 64-bit runtimes |
| 13 | `_store` mutations in arithmetic operators occur on freshly-allocated, thread-local `Natural()` instances not yet reachable by other threads | E.g. `operator +`: `var result = new Natural();  result._store.SetDigit(); return result;` |  Supported | Object goes live only at `return`; all writes are exclusive to the constructing thread |
| 14 | `Equals` compares digits one at a time via `_store.GetDigit(i)`, each of which acquires `_store._syncRoot`  O(DigitCount) lock acquisitions per comparison | `Equals` body: `for (long i = 0; i < _store.DigitCount; i++) if (_store.GetDigit(i) != other._store.GetDigit(i)) return false;` |  Supported | High lock-acquisition overhead for large numbers |

**Falsified rows: 0.** All 14 claims verified; proceeding.

## 4. Thread Safety Assessment

| Member | Risk | Current state | Recommendation | Priority |
|---|---|---|---|---|
| `_store` (`DigitStore`) | Low | `DigitStore` locks every read/write internally | No action  arithmetic operators never share the pre-publication `result` |  |
| `_displayDigits` / `_precision` (static `long`) | ~~Medium~~ |  Fixed  `Interlocked.Read`/`Exchange` on explicit backing fields | No further action | Done |
| Hot-path lock contention in `operator *` parallel body | ~~Medium~~ |  Fixed  `DigitStore.SnapshotDigits()` called once per operand before the branch; all digit reads in both paths use plain `byte[]` arrays, no lock | No further action | Done |
| Hot-path lock contention in `DivRem` | Low |  `right._store.GetDigit()` inside `right * new Natural((ulong)k)` multiplications rerun for every dividend digit | Precompute `Natural[] multiples = new Natural[10]` (multiples[k] = right * k) before the outer loop | **P2** |

## 5. Parallelization Opportunities

| Method / Loop | Shared Writes | Iterations Independent? | Parallelizable? | Suggested .NET API |
|---|---|---|---|---|
| `operator +` carry loop | `result._store` (thread-local) |  No  carry chain |  Sequential |  |
| `operator -` borrow loop | `result._store` (thread-local) |  No  borrow chain |  Sequential |  |
| `operator *` outer loop  partial product construction | `partials[c]` (non-overlapping) |  Yes |  **Implemented**  `Parallel.For` for `outerCount > processorCount * 2` | No structural change needed |
| `operator *` parallel path  **operand snapshot** | `aux._store._syncRoot` (contention) |  Yes  after snapshot, no lock needed |  **Done**  `DigitStore.SnapshotDigits()` called for both operands before the branch; lambdas (and the sequential path) index plain `byte[]` arrays directly | `DigitStore.SnapshotDigits()` → `byte[]` |
| `operator *` partial-products combination | `result` (serial accumulation) |  No (as coded  serial) |  **New**  parallel tree reduction: pair up `partials`, sum pairs concurrently in `Parallel.For`, repeat for O(log outerCount) rounds | `Parallel.For` + `Natural[]` ping-pong buffers |
| `operator *` inner digit loop (overflow chain) | `temp._store` (thread-local) |  No  overflow chain |  Sequential |  |
| `DivRem` bring-down outer loop | `partial` (sequential state) |  No |  Sequential |  |
| `DivRem` trial-quotient inner loop (k=1..9) | `q`, `qTimesDivisor` |  9 independent candidates |  Too small  overhead > gain; **eliminate via precomputation** instead | Precompute `Natural[] multiples = [Zero, right, right*2, , right*9]` once; binary-search or scan the array |
| `Factorial` parallel body | `partials[i]` (non-overlapping) |  Yes |  **Implemented**  `Parallel.For` over sub-ranges | No structural change needed |
| `Factorial` serial combination (`foreach`) | `result` |  No (as coded  serial) |  **New**  parallel tree reduction of `partials` array from O(t) to O(log t) multiplications | Same tree-reduction pattern as `operator *` combination |
| `Equals` digit comparison loop | None  read-only |  Yes |  Short-circuit semantics defeat PLINQ; **snapshot + `SequenceEqual`** is faster for large numbers without parallelism | Snapshot both `_store` byte arrays under lock; compare with `ReadOnlySpan<byte>.SequenceEqual` |
| `CompareTo` digit comparison loop | None  read-only |  Yes (read-only) |  Must compare MSDLSD and return first difference; short-circuit; not amenable to parallel reduction |  |
| `TryParse` digit-write loop | `n._store` (thread-local) |  Data-independent |  Blocked  `DigitStore.SetDigit` sequential-position guard | Requires new `DigitStore` random-access API |
| `ToString` / `TryFormat` | None (read-only snapshot) |  Yes |  **Already implemented** inside `DigitStore.ToString(char)` | No work needed here |
| All `Is*` predicates | None | N/A  O(1) |  Already O(1) |  |
| `Pow` binary-exponentiation loop | `result`, `b` (sequential) |  No  each step needs prior values |  Sequential |  |

## 6. Impl Completeness Coverage

| Member Group | Members | Covered? |
|---|---|---|
| Constructors | `Natural()`, `Natural(Natural)`, `Natural(ulong)`, `Natural(int)`, `Natural(string)`, `Natural(ReadOnlySpan<char>)` |  All |
| Static config properties | `DisplayDigits`, `Precision` |  All |
| `INumberBase` static properties | `One`, `Zero`, `Radix`, `AdditiveIdentity`, `MultiplicativeIdentity` |  All |
| `Is*` predicates (17 methods) | `IsZero`  `IsSubnormal` |  All (grouped) |
| Magnitude helpers | `Abs`, `MaxMagnitude`, `MaxMagnitudeNumber`, `MinMagnitude`, `MinMagnitudeNumber` |  All |
| Equality / comparison | `Equals(Natural?)`, `Equals(object?)`, `CompareTo(Natural?)`, `CompareTo(object?)`, `GetHashCode()` |  All |
| Unary operators | `operator +(Natural)`, `operator -(Natural)` |  All |
| Binary arithmetic operators | `operator +`, `-`, `*`, `/`, `%` |  All |
| Increment / decrement | `operator ++`, `--` |  (delegate to `+`/`-`) |
| Comparison operators | `==`, `!=`, `>`, `>=`, `<`, `<=` |  (delegate to `CompareTo`) |
| Domain operations | `DivRem` (static + instance), `BringDownDigit` (private), `Pow`, `Factorial` |  All |
| Formatting | `ToString()`, `ToString(string?, IFormatProvider?)`, `TryFormat` |  All |
| Parsing (8 overloads) | All `Parse`/`TryParse` variants |  All (grouped) |
| Generic conversion stubs (6) | `TryConvertFrom*/TryConvertTo*` |  (all throw; no parallelism surface) |

** Missing from audit: none.** All 60+ members accounted for.

## 7. Improvement Checklist

```
## Parallelization Audit Checklist for `Lovelace.Natural.Natural`

### Phase 0  Thread Safety (complete before Phase 1)
- [x] `_displayDigits` / `_precision`: backed by explicit `long` fields;
      getter uses `Interlocked.Read`, setter uses `Interlocked.Exchange` [Done  P1]

### Phase 1  Eliminate Lock Contention in Hot Parallel Paths

- [x] **`operator *`  snapshot operand digit arrays before `Parallel.For`** [P1]
      Before the parallel block, snapshot `aux._store` and `aux1._store` into plain
      `byte[]` digit arrays (one digit per slot, index = position) under
      `DigitStore._syncRoot`. Inside the `Parallel.For` lambda, read digits from
      the snapshots without acquiring any lock.
      Expected gain: eliminates O(outerCount � aux1.DigitCount) `Monitor.Enter`
      overhead per multiplication; benchmark target  20 % speedup for 10 000-digit
      operands.
      Approach:
        1. Expose `internal byte[] SnapshotDigits()` on `DigitStore` that locks and
           returns a fresh `byte[]` of length `DigitCount` (one byte per digit).
        2. In `operator *`, call `byte[] dAux = aux._store.SnapshotDigits()` and
           `byte[] dAux1 = aux1._store.SnapshotDigits()` before the `if (outerCount
           > processorCount * 2)` branch.
        3. Replace `aux._store.GetDigit(c)`  `dAux[c]`
           and `aux1._store.GetDigit(c2)`  `dAux1[c2]` inside the lambda.

- [ ] **`operator *`  parallel tree reduction of partial products** [P1]
      After `Parallel.For` finishes, combine `partials` with a binary tree reduction
      instead of a linear serial sum.
      Algorithm:
        1. Let `buf = partials` (compacted to remove `null` entries).
        2. While `buf.Length > 1`:
             int half = buf.Length / 2;
             var next = new Natural[half + buf.Length % 2];
             Parallel.For(0, half, i => next[i] = buf[2*i] + buf[2*i+1]);
             if (buf.Length % 2 == 1) next[half] = buf[^1];
             buf = next;
        3. `return buf[0]`.
      Expected gain: reduces combination from O(outerCount) serial additions to
      O(log outerCount) parallel rounds; significant for large second operands.

- [ ] **`Factorial()`  parallel tree reduction of partial products** [P1]
      After `Parallel.For(0, t, ...)` fills `partials[0..t-1]`, replace
      `foreach (var p in partials) result *= p` with the same binary tree reduction
      as `operator *` above.
      Expected gain: O(t)  O(log t) serial multiplications during combination;
      10-20 % speedup for ProcessorCount > 4.

### Phase 2  Algorithmic Improvements (no parallelism required)

- [ ] **`DivRem`  precompute divisor multiples** [P2]
      Before the outer `for (long i = n - 1; i >= 0; i--)` loop, compute:
        Natural[] multiples = new Natural[10];
        multiples[0] = new Natural(); // 0 * right
        for (byte k = 1; k <= 9; k++) multiples[k] = right * new Natural((ulong)k);
      Inside the outer loop, replace the inner `for k = 1..9` search with a
      linear or binary scan over `multiples`.
      Expected gain: eliminates O(9 � digitCount(right)) repeated multiplications
      per dividend digit; O(9 � n_digits � digitCount(right))  O(9 � digitCount(right))
      net allocation.

- [ ] **`Equals`  snapshot + `SequenceEqual` for large numbers** [P2]
      Expose `internal byte[] SnapshotDigits()` on `DigitStore` (same helper as
      Phase 1 `operator *`). In `Equals`, when `_store.DigitCount > threshold`
      (e.g. 512 digits), snapshot both stores and compare via
      `ReadOnlySpan<byte>.SequenceEqual` (SIMD-accelerated on .NET 10).
      Eliminates O(DigitCount) individual `Monitor.Enter` calls, replacing them
      with two bulk snapshots under lock and one SIMD byte comparison.

### Out of Scope
- [ ] `TryParse` digit-write loop  not parallelizable without a `DigitStore`
      random-access `SetDigit` API. Track as a future Representation enhancement.
- [ ] `operator +`, `operator -`  carry/borrow chains; not parallelizable.
- [ ] `CompareTo`  short-circuit MSDLSD scan; not parallelizable.
- [ ] `Pow`  binary exponentiation is already O(log n); no further gain.
- [ ] All `Is*` predicates  already O(1).
- [ ] `ToString` / `TryFormat`  already parallelized inside `DigitStore`.
```

*All assumptions verified by Falsify Claims. Zero Falsified rows.*
