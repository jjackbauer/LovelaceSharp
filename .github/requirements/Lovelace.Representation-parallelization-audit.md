# Parallelization Audit — `Lovelace.Representation.DigitStore`

## 1. Data Mutability Table

| Member | Type | Mutable? | Shared across calls? | Thread-safety risk |
|---|---|---|---|---|
| `_bytes` (`List<byte>`) | reference type (collection) | Yes — `Add`, `RemoveAt`, indexer write, `Clear`, `AddRange` | Yes | **High** |
| `_digitCount` (`long`) | value type | Yes — incremented in `SetDigit`, decremented in `ShrinkDigits`, zeroed in `Initialize`, set via property | Yes | **Medium** |
| `_isZero` (`bool`) | value type | Yes — set `false` in `SetDigit`, set `true` in `Initialize`, set via property | Yes | **Medium** |
| `ByteCount` (computed) | `long` (computed get-only) | No — derived from `_bytes.Count` | Yes (indirectly) | **Low** (race if `_bytes` is unsynchronised) |
| `DigitCount` (internal set) | `long` | Yes — thin wrapper over `_digitCount` | Yes | **Medium** |
| `IsZero` (internal set) | `bool` | Yes — thin wrapper over `_isZero` | Yes | **Medium** |

## 2. Sequential Dependency Graph

| Method | Reads shared state | Writes shared state | Calls mutating methods on `this` | Loop with independent iterations? |
|---|---|---|---|---|
| `GetDigit` | `_isZero`, `_digitCount`, `_bytes` (via `GetBitwise`) | None | None | No loop |
| `SetDigit` | `_digitCount`, `_isZero`, `_bytes` (via `GetBitwise`) | `_digitCount` (`++`), `_isZero` | `GetBitwise`, `SetBitwise` → `GrowDigits` | No loop |
| `TrimLeadingZeros` | `_digitCount` (loop condition), `_bytes` (via `GetDigit`) | `_digitCount`, `_bytes` (via `ShrinkDigits`) | `ShrinkDigits`, `Reset` | **No** — carry chain: each iteration reads `_digitCount` as mutated by the prior `ShrinkDigits` call |
| `ToString(char)` | `_isZero`, `_digitCount`, `_bytes` (via `GetBitwise`) | None | None | **Yes** — each byte index `c` produces two independent output characters with no cross-iteration dependency |
| `ToString()` | delegates | — | `ToString('\0')` | No loop |
| `Dump` | all fields | None | `ToString`, `GetBitwise` | Debug-only read-only loop |
| `GetBitwise` | `_bytes[(int)pos]` | None (out params only) | None | No loop |
| `SetBitwise` | `_bytes` length (`ByteCount`) | `_bytes[(int)pos]` | `GrowDigits` (conditional) | No loop |
| `GrowDigits` | None | `_bytes` (`Add`) | None | No loop |
| `ShrinkDigits` | `_digitCount` | `_bytes` (`RemoveAt` or `SetBitwise`), `_digitCount` (`--`) | `GetBitwise`, `SetBitwise` | No loop |
| `ClearDigits` | None | `_bytes` (`Clear`) | None | No loop |
| `CopyDigitsFrom` | `other._bytes`, `other._isZero` | `_bytes` (`Clear` + `AddRange`) | None | No loop (bulk copy) |
| `Initialize` | None | `_digitCount`, `_isZero` | None | No loop |
| `Reset` | `_isZero` | (via callees) | `ClearDigits`, `Initialize` | No loop |

## 3. Falsify Claims Result

Claims collected from Steps 2–3:

| # | Claim | Evidence (file:line) | Status | Reason |
|---|---|---|---|---|
| 1 | `_bytes` is mutable despite the `readonly` field modifier | `DigitStore.cs:19` — `private readonly List<byte> _bytes`; mutated via `Add`, `Clear`, `RemoveAt`, indexer throughout | ✅ Supported | `readonly` restricts reference reassignment, not content mutation of the `List<byte>` |
| 2 | `_digitCount` is mutated in four locations: `SetDigit`, `ShrinkDigits`, `Initialize`, and the copy constructor | `DigitStore.cs` — `SetDigit` (`_digitCount++`), `ShrinkDigits` (`_digitCount--`), `Initialize` (`_digitCount = 0`), copy ctor (`_digitCount = other._digitCount`) | ✅ Supported | All four mutation sites confirmed |
| 3 | `_isZero` is mutated in three locations: `SetDigit`, `Initialize`, and the `IsZero` internal setter | `DigitStore.cs` — `SetDigit` (`_isZero = false` when `position == 0`), `Initialize` (`_isZero = true`), `IsZero` setter | ✅ Supported | All three sites confirmed |
| 4 | `GetDigit` is purely read-only — it never writes `_bytes`, `_digitCount`, or `_isZero` | `DigitStore.cs` — reads `_isZero`, `_digitCount`, calls `GetBitwise`; no field assignment | ✅ Supported | No writes found |
| 5 | `SetDigit` writes both `_digitCount` and `_isZero` and indirectly mutates `_bytes` via `SetBitwise`/`GrowDigits` | `DigitStore.cs` — `_digitCount++`, `_isZero = false`, calls `SetBitwise` → `GrowDigits` → `_bytes.Add` | ✅ Supported | All three shared mutable members touched |
| 6 | `TrimLeadingZeros` has a carry chain: each loop iteration reads `_digitCount` as modified by the preceding `ShrinkDigits` | `DigitStore.cs` — while condition `_digitCount > 1` is re-evaluated after each `ShrinkDigits()` decrements `_digitCount` | ✅ Supported | Sequential dependency confirmed |
| 7 | The `for` loop in `ToString(char)` reads `_bytes` via `GetBitwise` with no writes; iterations are independent | `DigitStore.cs` — loop iterates `c` from `lastByteIdx-1` to 0, calls only `GetBitwise` and `sb.Append`; no field writes | ✅ Supported | Pure read loop; output positions are non-overlapping |
| 8 | `ShrinkDigits` reads `_digitCount % 2` before mutating, then decrements `_digitCount` | `DigitStore.cs` — `if (_digitCount % 2 == 1)` branches, then `_bytes.RemoveAt` or `SetBitwise(..., 0x0F)`, then `_digitCount--` | ✅ Supported | Sequence confirmed |
| 9 | `CopyDigitsFrom` guards against self-copy via `ReferenceEquals` | `DigitStore.cs` — `if (!ReferenceEquals(other, this) && !other._isZero)` | ✅ Supported | Guard present |
| 10 | `GetBitwise` is purely read-only — only reads `_bytes[(int)pos]`, assigns only to `out` params | `DigitStore.cs` — no field assignment in body | ✅ Supported | No writes found |
| 11 | `SetBitwise` writes `_bytes[(int)pos]` and conditionally calls `GrowDigits` when `pos == ByteCount` | `DigitStore.cs` — `if (pos == size) GrowDigits(); _bytes[(int)pos] = packed;` | ✅ Supported | Conditional growth confirmed |
| 12 | `GrowDigits` appends exactly one sentinel byte `0x0C` to `_bytes` | `DigitStore.cs` — `_bytes.Add(0x0C);` | ✅ Supported | Single-byte append only |

**Falsified rows: 0.** All claims confirmed; proceeding.

## 4. Thread Safety Assessment

| Member | Risk | Recommendation | Priority |
|---|---|---|---|
| `_bytes` (`List<byte>`) | **High** | Add `private readonly object _syncRoot = new();`. All methods that perform a read-check-mutate sequence on `_bytes` (`SetBitwise`, `ShrinkDigits`, `ClearDigits`, `CopyDigitsFrom`, `GrowDigits`) must hold `lock (_syncRoot)` for the entire sequence to eliminate TOCTOU races. | **P0** |
| `_digitCount` (`long`) | **Medium** | Always update under the same `_syncRoot` lock as the accompanying `_bytes` mutation. `_digitCount` and `_bytes` are never updated independently; a bare `Interlocked.Increment` is insufficient because both must stay in sync atomically. | **P0** |
| `_isZero` (`bool`) | **Medium** | Update under the same `_syncRoot` lock as `_digitCount` and `_bytes`. The `SetDigit` update (`_isZero = false`) is conditional on `position`, so it cannot be detached from the combined lock. | **P0** |
| `ByteCount` (computed) | Low | Derived from `_bytes.Count` — reading it outside a lock in a concurrent context is a race on `_bytes`. No independent lock needed; cover it under `_syncRoot`. | **P1** |
| `DigitCount` / `IsZero` internal setters | Medium | Called from upper-layer projects (`Lovelace.Natural`). Consider replacing with internal locked mutator methods to prevent callers from bypassing the lock and creating unsynchronised writes. | **P1** |
| `GetDigit` (read path) | Low | Acquires no lock; concurrent calls while `SetDigit` mutates `_bytes` yield a data race. Add a read lock (or snapshot `_isZero` and `_digitCount` into locals under lock before reading) when concurrent access is expected. | **P1** |
| `ToString(char)` (read path) | Low | Same as `GetDigit` — reads `_bytes` without a lock. Must snapshot or acquire a read lock before entering any parallel loop added in Phase 1. | **P1** (prerequisite for Phase 1) |

## 5. Parallelization Opportunities

| Method / Loop | Shared Writes | Iterations Independent? | Parallelizable? | Suggested .NET API |
|---|---|---|---|---|
| `TrimLeadingZeros` while loop | `_bytes`, `_digitCount` (via `ShrinkDigits`) | ❌ No — `_digitCount` from iteration N feeds iteration N+1's condition and `GetDigit` | ❌ Sequential | — |
| `ToString(char)` for loop over bytes | None — read-only | ✅ Yes — byte index `c` writes chars at output offsets `2c` and `2c+1`; no cross-iteration dependency | ✅ Embarrassingly parallel | Pre-allocate `char[]` of known length; `Parallel.For` fills each pair of positions; assemble with `new string(chars)` |
| `SetDigit` (single op, no inner loop) | `_bytes`, `_digitCount`, `_isZero` | N/A | ❌ No inner loop | — |
| `GetDigit` (single op, no inner loop) | None | N/A | ❌ No inner loop | — |
| `GetBitwise` (single op, no inner loop) | None | N/A | ❌ No inner loop | — |
| `SetBitwise` (single op, no inner loop) | `_bytes` | N/A | ❌ No inner loop | — |
| `ShrinkDigits` (single op, no inner loop) | `_bytes`, `_digitCount` | N/A | ❌ No inner loop | — |
| `CopyDigitsFrom` (`_bytes.AddRange`) | `_bytes` | Already a bulk O(n) copy | ✅ Already near-optimal — `AddRange` compiles to `Array.Copy` internally | Replace with `CollectionsMarshal.AsSpan` + `Span<byte>.CopyTo` for a zero-allocation path in a future refactor |
| `Dump` for loop (debug helper) | None — read-only | ✅ Yes | ✅ Low value — debug helper only | Not worth parallelizing |

## 6. Impl Completeness Coverage

`Lovelace.Representation` intentionally implements only the *storage layer* of `Lovelace` (C++); arithmetic methods belong to `Lovelace.Natural`. All C# members are covered in the tables above:

| C# Member | Covered in audit? |
|---|---|
| `_bytes`, `_digitCount`, `_isZero` | ✅ Sections 1–2 |
| `ByteCount`, `DigitCount`, `IsZero` | ✅ Section 1 |
| `DigitStore()`, `DigitStore(DigitStore)` | ✅ Section 2 (copy ctor claim #2) |
| `GetDigit`, `SetDigit` | ✅ Sections 2, 4, 5 |
| `TrimLeadingZeros` | ✅ Sections 2, 5 |
| `ToString()`, `ToString(char)` | ✅ Sections 2, 5 |
| `Dump` | ✅ Sections 2, 5 |
| `GetBitwise`, `SetBitwise` | ✅ Sections 2, 4, 5 |
| `GrowDigits`, `ShrinkDigits` | ✅ Sections 2, 4 |
| `ClearDigits`, `CopyDigitsFrom` | ✅ Sections 2, 5 |
| `Initialize`, `Reset` | ✅ Section 2 |

**No omissions.** All 20 members covered; zero ⬜ Missing from audit.

## 7. Improvement Checklist

```
## Parallelization Audit Checklist for `Lovelace.Representation.DigitStore`

### Phase 0 — Thread Safety (complete before Phase 1)
- [x] Add `private readonly object _syncRoot = new();` field [P0 — prerequisite for all parallelization]
- [x] `_bytes`: wrap every read-check-mutate sequence in `lock (_syncRoot)` — affects
      SetBitwise, GrowDigits, ShrinkDigits, ClearDigits, CopyDigitsFrom [P0]
- [x] `_digitCount` + `_bytes`: hold the same lock for the combined increment in `SetDigit`
      and decrement in `ShrinkDigits` to keep both in sync [P0]
- [x] `_isZero` + `_digitCount`: update atomically under the same `_syncRoot` lock in
      `SetDigit` and `Initialize` [P0]
- [x] `DigitCount` / `IsZero` internal setters: replace with locked internal mutator methods
      to prevent upper-layer callers (Lovelace.Natural) from bypassing the lock [P1]
- [x] `GetDigit`: acquire a read lock (or snapshot `_isZero`/`_digitCount` under lock)
      when concurrent read/write is possible [P1]
- [x] `ToString(char)`: acquire a read lock (or snapshot `_bytes` into a local span)
      before entering the parallel loop added in Phase 1 [P1 — prerequisite for Phase 1 item below]

### Phase 1 — Parallelization
- [x] `ToString(char)` digit-extraction for loop — pre-allocate `char[]` of length `_digitCount`,
      use `Parallel.For` over byte indices, write two chars per iteration at known offsets;
      assemble result with `new string(chars)`. Depends on Phase 0 read-lock for `_bytes`. [depends on Phase 0]
- [x] `CopyDigitsFrom` — replace `_bytes.Clear(); _bytes.AddRange(other._bytes)` with a
      `Span<byte>`-based copy (`CollectionsMarshal.AsSpan` + snapshot of `other._bytes`)
      for reduced allocation; lock both instances during copy. [independent]
```
