# Round 31 — the DigitStore parity surface

Scope: this round adds `rust/lovelace-nat/src/digit_store.rs` (new module and its tests),
two additive lines in `rust/lovelace-nat/src/lib.rs` (module declaration and re-export) and
this report. No `.cs` file, fixture, corpus, tool, `parity/` artifact or other document was
changed, and no existing `Nat` operation changed.

Result: **90 of the 91 deferred `Lovelace.Representation.Tests` census cases are mirrored**
by 85 Rust test functions in `digit_store::tests` (theories share a function), plus seven
additional conversion and limb-access tests. The one unmapped case is `UnitTest1/Test1#0`, an empty
template test with no assertion or expected output. `cargo test -p lovelace-nat` is
**226 passed, 0 failed** (134 pre-existing + 92 new). The workspace, corpus, fixture and
CLI gates are recorded in section 7.

## 1. The C# contract as read

The contract is `Lovelace.Representation/DigitStore.cs` (696 lines) plus the eight test
files under `Lovelace.Representation.Tests`. The census project has **86 methods / 91
cases**:

| C# test class | Census cases |
|---|---:|
| `DigitStoreTests` | 44 |
| `DigitStoreSnapshotDigitsTests` | 13 |
| `DigitStoreResetTests` | 7 |
| `DigitStoreCopyDigitsFromTests` | 6 |
| `DigitStoreTrimLeadingZerosTests` | 6 |
| `DigitStoreMutatorTests` | 5 |
| `DigitStoreToStringParallelTests` | 5 |
| `DigitStoreThreadSafetyTests` | 4 |
| `UnitTest1` | 1 |
| **total** | **91** |

The tests use the default/copy constructors, the three properties, digit and nibble
access, the snapshot helpers, `GrowDigits`/`ShrinkDigits`, `Reset`, `CopyDigitsFrom`,
`TrimLeadingZeros`, `SetDigitCount`/`SetIsZero`, both `ToString` overloads and the four
concurrency scenarios. They do **not** call a decimal, byte-array or `u32` constructor,
because the C# store has none. The only bulk input member is `SetDigitsBulk(ReadOnlySpan<byte>)`
(Lovelace.Representation/DigitStore.cs:535), which takes unpacked decimal digits, not big-endian
bytes or binary limbs.

## 2. Base difference — measured, and a correction to the brief

The brief says "Rust's `lovelace-nat` uses base-10^9 limbs internally". That is not what this
checkout contains. `rust/lovelace-nat/src/lib.rs:1-5` states the representation and the code
implements it as **little-endian base-2^64 limbs, no most-significant zero limb, zero = empty**;
`Nat` holds a lazily materialised `OnceLock<Vec<u64>>` plus a lazily rendered decimal
`OnceLock<String>`. The decimal conversion uses `DEC_CHUNK = 10^19` (lib.rs:17); there is no
base-10^9 limb path in the crate.

The C# `DigitStore` is a **base-10 BCD buffer**: two decimal digits per byte, high nibble =
even position, low nibble = odd position, position 0 = least significant. The new Rust
`DigitStore` is therefore a standalone BCD value, not a view over `Nat`. It reproduces the
C# base exactly; the two representations meet only through canonical decimal text (and the
explicit `u32` limb conversions below). This is why the port is additive: no `Nat` operation
or buffer layout was touched.

## 3. API map (C# member -> Rust member)

| C# member | Rust member | Notes |
|---|---|---|
| `DigitStore()` | `DigitStore::new()` / `Default` | empty canonical zero |
| `DigitStore(DigitStore other)` | `Clone::clone` | deep copy; zero source contributes no bytes |
| `long ByteCount` | `byte_count() -> usize` | backing byte count |
| `long DigitCount` | `digit_count() -> usize` | logical decimal digits |
| `bool IsZero` | `is_zero() -> bool` | canonical zero flag |
| `SetDigitCount(long)` | `set_digit_count(usize)` | locked metadata write |
| `SetIsZero(bool)` | `set_is_zero(bool)` | locked metadata write |
| `byte GetDigit(long)` | `get_digit(i64) -> u8` | 0 when zero/out of range |
| `void SetDigit(long, byte)` | `set_digit(i64, u8) -> Result<(), DigitStoreError>` | sequential write; `ArgumentOutOfRangeException` becomes `PositionOutOfRange` |
| `byte[] SnapshotDigits()` | `snapshot_digits() -> Vec<u8>` | independent copy, LSB first |
| `(byte[], int) RentDigitSnapshot()` | `rent_digit_snapshot() -> (Vec<u8>, usize)` | owned buffer + valid length |
| `void ReturnDigitSnapshot(byte[])` | `return_digit_snapshot(Vec<u8>)` | drops the owned buffer; no shared pool |
| `GetBitwise(long, out byte, out byte)` | `get_bitwise(i64) -> (u8, u8)` | silent `(0,0)` out of range |
| `SetBitwise(long, byte, byte)` | `set_bitwise(i64, u8, u8)` | grows at `pos == ByteCount`; silent beyond |
| `GrowDigits()` | `grow_digits()` | appends `0x0C` |
| `ShrinkDigits()` | `shrink_digits()` | odd count drops byte, even count sets low `0x0F` |
| `ClearDigits()` | `clear_digits()` | bytes only, metadata untouched |
| `CopyDigitsFrom(DigitStore)` | `copy_digits_from(&DigitStore)` | bytes only; self/zero source no-op; canonical lock order |
| `Initialize()` | `initialize()` | metadata only, bytes untouched |
| `Reset()` | `reset()` | clears bytes and metadata, no-op when zero |
| `SetDigitsBulk(ReadOnlySpan<byte>)` | `set_digits_bulk(&[u8])` | trims leading zeros, all-zero -> zero |
| `TrimLeadingZeros()` | `trim_leading_zeros()` | trims and resets an all-zero value |
| `string ToString()` | `Display` / `to_string()` | most significant first, `"0"` when zero |
| `string ToString(char)` | `to_string_with_separator(char)` | every three digits from the right; `'\0'` = no separator |
| `void Dump(bool = false)` | `dump(bool)` | debug print, same labels |

Rust extensions with no C# member (documented in the module):

| Rust member | Purpose |
|---|---|
| `from_digits(&[u8])` | constructor form of `SetDigitsBulk` |
| `from_decimal(&str) -> Option<DigitStore>` | canonical decimal text; same grammar as `Nat::from_decimal` |
| `from_bcd_bytes(&[u8]) -> Option<DigitStore>` | canonical BCD backing bytes; sentinels accepted, other nibbles rejected |
| `to_bcd_bytes() -> Vec<u8>` | exact backing bytes including sentinels |
| `from_u32_limbs(&[u32])` | little-endian base-2^32 limbs, leading zero limbs trimmed |
| `to_u32_limbs() -> Vec<u32>` | little-endian base-2^32 limbs, no leading zero limb |
| `limb_count() -> usize` | number of base-2^32 limbs (zero has none) |
| `get_u32_limb(usize) -> u32` | indexed limb, 0 when out of range |
| `from_nat(&Nat)` / `to_nat()` | bridge through canonical decimal text |

## 4. Semantics preserved and explicit divergences

Preserved:

* BCD nibble order and the `0x0C` / `0x0F` sentinels; odd digit counts leave the last low
  nibble `0x0F`; `SetDigit` on a new byte starts with the low-nibble sentinel.
* `GetDigit` returns 0 for the zero store and for out-of-range positions; `SetDigit` rejects
  negative positions and positions greater than `DigitCount` without changing state.
* `DigitCount` increases only when `position >= DigitCount`; `IsZero` is cleared only by a
  write to position 0.
* `CopyDigitsFrom` copies only bytes; a zero source is a no-op; a self-copy is a no-op; the
  destination's `DigitCount`/`IsZero` are deliberately untouched. Two stores copied in opposite
  directions lock in instance-id order, so the C# ABBA-deadlock test maps to a real lock order.
* `TrimLeadingZeros`, `Reset`, `Initialize`, `ClearDigits`, `GrowDigits` and `ShrinkDigits`
  follow the C# state transitions, including the sentinel writes.
* The rendering contract: most significant first, `"0"` for zero, separator every three digits
  counted from the right. The C# reference parallelises interior bytes; the Rust renderer is
  sequential, but the observable string is identical and the 19/20/99/100-digit cases assert it.
* Thread safety: every method is `&self` and takes the store mutex, so the four C# concurrency
  tests (concurrent trim/read, render/reset, same-destination copy, self-copy/read) are mirrored
  with shared `Arc<DigitStore>` values.

Divergences (all deliberate and documented in the module):

* `DigitCount`/`ByteCount` are `usize` instead of `long`; negative C# positions are still
  representable through `i64` digit positions.
* The C# exception becomes `Result<(), DigitStoreError>`.
* `RentDigitSnapshot` returns an owned `Vec<u8>`; `ReturnDigitSnapshot` drops it instead of
  returning it to `ArrayPool<byte>.Shared`. Content and valid length are the observable contract.
* `ShrinkDigits` on an empty store saturates at zero; C# would decrement its `long` to -1. The
  C# tests never call it on an empty store.
* Rendering a mutator-created inconsistent state (non-zero flag, `DigitCount == 0`) returns
  `"0"`; the C# renderer would index its empty buffer and throw. The mutator tests assert the
  flag only and never render that state.
* The conversion and limb-access helpers are additive; the C# store has no such constructors. `from_bcd_bytes`
  normalises allocated-but-unwritten sentinel bytes away, so a zero store with `GrowDigits` bytes
  round-trips to the canonical zero (value preserved, allocation padding not preserved).

## 5. Tests and expected values

Every Rust test is in `rust/lovelace-nat/src/digit_store.rs` under `mod tests`; the census
id for a row is `<class>/<method>#<case index>` (section 6 lists the exact ids). All mirrored
tests use the C# method name verbatim, so the method column below is also the Rust test name.

### 5.1 `DigitStoreTests` (44 cases)

| Census methods | Concrete input -> expected |
|---|---|
| `Constructor_GivenDefault_{DigitCountIsZero, IsZeroIsTrue, ByteCountIsZero}` | `new()` -> `DigitCount=0`, `IsZero=true`, `ByteCount=0` |
| `CopyConstructor_Given{NonZeroSource, ZeroSource, Self}` | digits `[3,7,1]` cloned -> same `DigitCount=3`, `ByteCount=2`, `IsZero=false`, all digits/nibbles equal; zero clone -> `IsZero=true`, `ByteCount=0`; `[5]` self-copy -> unchanged |
| `GetBitwise_GivenPackedByte_Extracts{High,Low}NibbleCorrectly` | `[9]` -> high 9; `[4,6]` -> low 6 |
| `SetBitwise_Given{TwoNibbles, PositionEqualsByteCount, PositionBeyondByteCountPlusOne}` | `[3,7]` -> `(3,7)`; empty + `set_bitwise(0,5,3)` -> `ByteCount=1`; empty + `set_bitwise(2,5,3)` -> `ByteCount=0` |
| `GrowDigits_GivenCall_{IncreasesByteCountByOne, NewByteLowNibbleIsSentinel0x0C}` | `GrowDigits` -> `ByteCount+1`; byte 0 nibbles `(0, 0x0C)` |
| `ShrinkDigits_Given{Odd, Even}DigitCount...` | `[5]` -> `DigitCount=0`, `ByteCount=0`; `[5,3]` -> `DigitCount=1`, `ByteCount=1`, low nibble `0x0F` |
| `GetDigit_Given{IsZeroTrue, EvenPosition, OddPosition, PositionAtOrBeyondDigitCount, TwoConsecutiveDigits}` | zero -> 0 at 0 and 5; `[7]` -> pos0=7; `[4,9]` -> pos1=9; `[3]` -> pos1=0, pos100=0; `[5,8]` -> 5,8 |
| `SetDigit_Given{PositionZero, NewPosition, EvenPosition, OddPosition, OddPositionOnNewByte, PositionBeyondDigitCountByMoreThanOne, PositionNonZeroOnly}` | pos0=3 flips `IsZero`; `DigitCount` 0->1->2; `[6]` high=6; `[2,8]` low=8; `[4]` -> `(4,0x0F)`; `set_digit(2,5)` -> `Err(PositionOutOfRange)`, `DigitCount=0`; `[7,3]` -> `IsZero=false` |
| `ByteCount_Given{No,One,Two,Three}Digits...` | `[]` -> 0; `[3]` -> 1; `[3,7]` -> 1; `[3,7,1]` -> 2 |
| `DigitCount_Given{Default, OneSetDigit, TwoSetDigitsInSequence}...` | `[]` -> 0; `[4]` -> 1; `[4,6]` -> 2 |
| `IsZero_Given{Default, DigitSetAtPositionZero, DigitSetAtPositionNonZeroOnly}...` | `new` -> true; `[9]` -> false; `[5,3]` -> false |
| `ToString_Given{IsZeroTrue, SingleDigit, OddDigitCount, EvenDigitCount, MultipleBytes}` | `"0"`, `"7"`, `"123"`, `"1234"`, `"12345"` |
| `ToString_GivenSeparator_...` | `[7,6,5,4,3,2,1]` + `','` -> `"1,234,567"` |
| `ToString_GivenSeparatorAndLessThanFourDigits_...` | `[3,2,1]` + `','` -> `"123"` |

### 5.2 `DigitStoreSnapshotDigitsTests` (13 cases)

| Census methods | Concrete input -> expected |
|---|---|
| `SnapshotDigits_GivenZeroStore...` | zero -> empty vector |
| `SnapshotDigits_GivenSingleDigit...` | `[5]` -> `[5]` |
| `SnapshotDigits_GivenPairedDigitsInSameByte...` | `[4,9]` -> `[4,9]` |
| `SnapshotDigits_GivenMultipleDigits...` | `[3,7,2,5]` -> `[3,7,2,5]` |
| `SnapshotDigits_GivenTenDigitSequence...#0-#2` | `[0..9]`, `[9..0]`, `[1,0,1,0,1,0,1,0,1,0]` -> identical vectors |
| `SnapshotDigits_ReturnsCopyNotReference...` | snapshot `[0]=0xFF` -> store digit 0 still 7 |
| `SnapshotDigits_LengthEqualsDigitCount#0-#3` | counts 1,3,7,11 -> `snapshot.len() == DigitCount` |
| `SnapshotDigits_GivenConcurrentReads...` | 20 digits (`i%10`), 32 threads -> every snapshot equals the input |

### 5.3 `DigitStoreResetTests` (7 cases)

| Census methods | Concrete input -> expected |
|---|---|
| `Reset_GivenNonZeroStore_{IsZeroBecomesTrue, DigitCountBecomesZero, ByteCountBecomesZero, GetDigitReturnsZero}` | `[5]` -> `IsZero=true`; `[7]` -> `DigitCount=0`; `[3]` -> `ByteCount=0`; `[9]` -> `GetDigit(0)=0` |
| `Reset_GivenMultiDigitStore_ClearsEverything` | `[1,2,3]` -> `IsZero=true`, `DigitCount=0`, `ByteCount=0` |
| `Reset_GivenAlreadyZeroStore_RemainsZero` | `new` -> `IsZero=true`, `DigitCount=0` |
| `Reset_ThenSetDigit_StoreIsReusable` | reset then `6,8` -> `DigitCount=2`, digits `6,8`, `IsZero=false` |

### 5.4 `DigitStoreCopyDigitsFromTests` (6 cases)

| Census methods | Concrete input -> expected |
|---|---|
| `CopyDigitsFrom_GivenFourDigitSource...` | dest from `[3,7,1,5]`, copy from `[6,2,8,4]` -> `ByteCount=2`, `DigitCount=4`, digits `6,2,8,4` |
| `CopyDigitsFrom_GivenThreeDigitSource...` | dest from `[1,2,9]`, copy from `[4,5,7]` -> `ByteCount=2`, `DigitCount=3`, digits `4,5,7` |
| `CopyDigitsFrom_GivenZeroSource...` | dest `[8,3]`, copy from zero -> `ByteCount=1`, `DigitCount=2`, digits `8,3` |
| `CopyDigitsFrom_GivenNonZeroSource_MutatingSourceAfterCopy...` | copy `[5,9]`, then zero source -> dest digits still `5,9` |
| `CopyDigitsFrom_GivenTenDigitSource_AllDigitsPreserved` | copy `[9,8,7,6,5,4,3,2,1,0]` -> `ByteCount=5`, all ten digits |
| `CopyDigitsFrom_CrossCopyConcurrent...` | `a=[1,2]`, `b=[3,4]`, 500 cross-copies each way -> both destinations `ByteCount=1`, no deadlock |

### 5.5 `DigitStoreMutatorTests` (5 cases)

| Census methods | Concrete input -> expected |
|---|---|
| `SetDigitCount_GivenPositiveValue...` | `SetDigitCount(7)` -> 7 |
| `SetDigitCount_GivenZero...` | `[3]` then `SetDigitCount(0)` -> 0 |
| `SetDigitCount_ConcurrentCalls...` | 200 threads write 0..199 -> final value `< 200` (one of the written values) |
| `SetIsZero_GivenTrue...` | `[5]` then `SetIsZero(true)` -> true |
| `SetIsZero_GivenFalse...` | `new` then `SetIsZero(false)` -> false |

### 5.6 `DigitStoreTrimLeadingZerosTests` (6 cases)

| Census methods | Concrete input -> expected |
|---|---|
| `TrimLeadingZeros_GivenMultipleLeadingZeroDigits...` | `[5,0,0]` -> `DigitCount=1`, digit 5, not zero |
| `TrimLeadingZeros_GivenEvenDigitCountWithSingleLeadingZero...` | `[4,0]` -> `DigitCount=1`, digit 4 |
| `TrimLeadingZeros_GivenNoLeadingZeros...` | `[3,2,1]` -> `DigitCount=3`, `"123"` |
| `TrimLeadingZeros_GivenSingleNonZeroDigit...` | `[7]` -> `DigitCount=1`, digit 7 |
| `TrimLeadingZeros_GivenAllZeroDigits...` | `[0,0,0]` -> `IsZero=true` |
| `TrimLeadingZeros_GivenSingleZeroDigit...` | `[0]` -> `IsZero=true` |

### 5.7 `DigitStoreToStringParallelTests` (5 cases)

| Census methods | Concrete input -> expected |
|---|---|
| `ToString_Given20Digits...` | digits `i%10` for 0..19 -> `"98765432109876543210"` |
| `ToString_Given100Digits...` | 100 digits -> generated 100-char expected string |
| `ToString_Given19Digits...` | 19 digits -> `"8765432109876543210"` |
| `ToString_Given99Digits...` | 99 digits -> generated 99-char expected string |
| `ToString_Given20DigitsWithSeparator...` | 20 digits + `','` -> `"98,765,432,109,876,543,210"` |

### 5.8 `DigitStoreThreadSafetyTests` (4 cases)

| Census methods | Concrete input -> expected |
|---|---|
| `TrimLeadingZeros_ConcurrentWithGetDigit...` | `[5,0,0]`, 1000 trim/read iterations -> every read in 0..9, final digit 0 = 5 |
| `ToString_ConcurrentWithReset...` | `[1,2,3]`, 1000 reset/restore iterations -> every render non-empty, decimal-only, no leading zero (except `"0"`) |
| `CopyDigitsFrom_ConcurrentToSameDestination...` | source `[4,2]`, two 500-copy threads -> `ByteCount=1`, `DigitCount=2`, digits `4,2` |
| `CopyDigitsFrom_SelfCopyUnderConcurrentRead...` | `[7]`, 1000 self-copies while reading -> digit 0 always 7 |

### 5.9 Additive conversion and limb-access tests (no census id)

| Rust test | Input -> expected |
|---|---|
| `FromDecimal_GivenLeadingZeros_CanonicalisesAndRejectsNonDigits` | `"000123"` -> `"123"`, `DigitCount=3`, `ByteCount=2`; `"0000"` -> zero/0/0; `""`, `"  "`, `"-5"`, `"12a"` -> `None`; `" 42 "` -> `"42"` |
| `FromDigits_GivenLeadingZeroDigits_TrimsToCanonicalValue` | `[1,2,0,0]` -> `"21"`, `DigitCount=2`, `ByteCount=1`; `[0,0,0]` -> zero, `ByteCount=0` |
| `FromBcdBytes_GivenCanonicalBytes_RoundTripsValue` | `[5,4,3,2,1]` -> bytes `[0x54,0x32,0x1F]` -> `"12345"`, 5 digits/3 bytes; `[0x54,0x32,0x00]` -> `"2345"`; `[0x0C]` -> zero |
| `FromBcdBytes_GivenNonDecimalNibble_ReturnsNone` | `[0x0A]`, `[0x5A]`, `[0xE5]` -> `None` |
| `FromU32Limbs_GivenLeadingZeroLimbs_NormalisesAndRoundTrips` | `[u32::MAX,0]` -> `"4294967295"` -> `[u32::MAX]`; `[1,1]` -> `"4294967297"` -> `[1,1]`; `[0,0]` -> zero, empty limbs |
| `U32LimbAccess_GivenValue_ReturnsLittleEndianLimbsByIndex` | `[1,2,3]` -> `limb_count=3`, limbs `1,2,3`, index 9 -> 0; zero -> `limb_count=0`; `[u32::MAX]` -> `u32::MAX` |
| `NatConversions_GivenCanonicalValue_RoundTrip` | `Nat("123456789012345678901234567890")` -> store -> same text -> equal `Nat`; `Nat::zero` -> zero store -> zero `Nat` |

## 6. Census ids covered (90 of 91)

All ids below are from `parity/census.json`, `project = Lovelace.Representation.Tests`.
The 90 mapped ids:

```text
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreCopyDigitsFromTests/CopyDigitsFrom_CrossCopyConcurrent_NeitherDeadlocksNorThrows#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreCopyDigitsFromTests/CopyDigitsFrom_GivenFourDigitSource_ByteCountAndDigitsMatchSource#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreCopyDigitsFromTests/CopyDigitsFrom_GivenNonZeroSource_MutatingSourceAfterCopyDoesNotAffectDest#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreCopyDigitsFromTests/CopyDigitsFrom_GivenTenDigitSource_AllDigitsPreserved#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreCopyDigitsFromTests/CopyDigitsFrom_GivenThreeDigitSource_ByteCountAndDigitsMatchSource#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreCopyDigitsFromTests/CopyDigitsFrom_GivenZeroSource_DestinationBytesNotOverwritten#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreMutatorTests/SetDigitCount_ConcurrentCalls_FinalValueIsOneOfTheWrittenValues#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreMutatorTests/SetDigitCount_GivenPositiveValue_DigitCountIsUpdated#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreMutatorTests/SetDigitCount_GivenZero_DigitCountIsZero#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreMutatorTests/SetIsZero_GivenFalse_IsZeroBecomesFalse#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreMutatorTests/SetIsZero_GivenTrue_IsZeroBecomesTrue#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreResetTests/Reset_GivenAlreadyZeroStore_RemainsZero#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreResetTests/Reset_GivenMultiDigitStore_ClearsEverything#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreResetTests/Reset_GivenNonZeroStore_ByteCountBecomesZero#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreResetTests/Reset_GivenNonZeroStore_DigitCountBecomesZero#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreResetTests/Reset_GivenNonZeroStore_GetDigitReturnsZero#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreResetTests/Reset_GivenNonZeroStore_IsZeroBecomesTrue#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreResetTests/Reset_ThenSetDigit_StoreIsReusable#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreSnapshotDigitsTests/SnapshotDigits_GivenConcurrentReads_AllSnapshotsAreIdentical#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreSnapshotDigitsTests/SnapshotDigits_GivenMultipleDigits_ReturnsAllDigitsLeastSignificantFirst#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreSnapshotDigitsTests/SnapshotDigits_GivenPairedDigitsInSameByte_CorrectlyUnpacksBothNibbles#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreSnapshotDigitsTests/SnapshotDigits_GivenSingleDigit_ReturnsSingleElementArray#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreSnapshotDigitsTests/SnapshotDigits_GivenTenDigitSequence_ReturnsIdenticalSequence#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreSnapshotDigitsTests/SnapshotDigits_GivenTenDigitSequence_ReturnsIdenticalSequence#1
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreSnapshotDigitsTests/SnapshotDigits_GivenTenDigitSequence_ReturnsIdenticalSequence#2
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreSnapshotDigitsTests/SnapshotDigits_GivenZeroStore_ReturnsEmptyArray#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreSnapshotDigitsTests/SnapshotDigits_LengthEqualsDigitCount#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreSnapshotDigitsTests/SnapshotDigits_LengthEqualsDigitCount#1
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreSnapshotDigitsTests/SnapshotDigits_LengthEqualsDigitCount#2
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreSnapshotDigitsTests/SnapshotDigits_LengthEqualsDigitCount#3
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreSnapshotDigitsTests/SnapshotDigits_ReturnsCopyNotReference_MutatingSnapshotDoesNotAffectStore#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/ByteCount_GivenNoDigits_ReturnsZero#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/ByteCount_GivenOneDigit_ReturnsOne#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/ByteCount_GivenThreeDigits_ReturnsTwo#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/ByteCount_GivenTwoDigits_ReturnsOne#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/Constructor_GivenDefault_ByteCountIsZero#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/Constructor_GivenDefault_DigitCountIsZero#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/Constructor_GivenDefault_IsZeroIsTrue#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/CopyConstructor_GivenNonZeroSource_ProducesBitwiseIdenticalStore#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/CopyConstructor_GivenSelf_DoesNotCorruptState#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/CopyConstructor_GivenZeroSource_ProducesZeroInstanceWithNoBytes#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/DigitCount_GivenDefault_ReturnsZero#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/DigitCount_GivenOneSetDigit_ReturnsOne#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/DigitCount_GivenTwoSetDigitsInSequence_ReturnsTwo#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/GetBitwise_GivenPackedByte_ExtractsHighNibbleCorrectly#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/GetBitwise_GivenPackedByte_ExtractsLowNibbleCorrectly#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/GetDigit_GivenEvenPosition_ReturnsHighNibbleOfByteAtHalfPosition#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/GetDigit_GivenIsZeroTrue_ReturnsZeroForAnyPosition#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/GetDigit_GivenOddPosition_ReturnsLowNibbleOfByteAtHalfPosition#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/GetDigit_GivenPositionAtOrBeyondDigitCount_ReturnsZero#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/GetDigit_GivenTwoConsecutiveDigits_BothRoundTripCorrectly#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/GrowDigits_GivenCall_IncreasesByteCountByOne#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/GrowDigits_GivenCall_NewByteLowNibbleIsSentinel0x0C#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/IsZero_GivenDefault_ReturnsTrue#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/IsZero_GivenDigitSetAtPositionNonZeroOnly_RemainsTrue#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/IsZero_GivenDigitSetAtPositionZero_ReturnsFalse#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/SetBitwise_GivenPositionBeyondByteCountPlusOne_DoesNotWrite#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/SetBitwise_GivenPositionEqualsByteCount_CallsGrowDigits#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/SetBitwise_GivenTwoNibbles_PacksIntoExpectedByte#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/SetDigit_GivenEvenPosition_StoresDigitInHighNibble#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/SetDigit_GivenNewPosition_IncrementsDigitCount#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/SetDigit_GivenOddPositionOnNewByte_InitializesHighNibbleToFifteen#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/SetDigit_GivenOddPosition_StoresDigitInLowNibble#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/SetDigit_GivenPositionBeyondDigitCountByMoreThanOne_DoesNotWrite#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/SetDigit_GivenPositionNonZeroOnly_IsZeroRemainsTrue#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/SetDigit_GivenPositionZero_SetsIsZeroToFalse#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/ShrinkDigits_GivenEvenDigitCount_SetsLowNibbleToFifteenAndDecrementsCount#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/ShrinkDigits_GivenOddDigitCount_RemovesLastByteAndDecrementsCount#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/ToString_GivenEvenDigitCount_PrintsBothNibblesOfLastByte#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/ToString_GivenIsZeroTrue_ReturnsStringZero#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/ToString_GivenMultipleBytes_IteratesInReverseOrder#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/ToString_GivenOddDigitCount_PrintsHighNibbleOfLastByte#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/ToString_GivenSeparatorAndLessThanFourDigits_NoSeparatorInserted#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/ToString_GivenSeparator_InsertsCharacterEveryThreeDigitsFromRight#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTests/ToString_GivenSingleDigit_ReturnsThatDigitAsString#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreThreadSafetyTests/CopyDigitsFrom_ConcurrentToSameDestination_NeverThrowsAndFinalStateValid#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreThreadSafetyTests/CopyDigitsFrom_SelfCopyUnderConcurrentRead_NeverCorruptsDigit#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreThreadSafetyTests/ToString_ConcurrentWithReset_AlwaysReturnsValidDecimalString#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreThreadSafetyTests/TrimLeadingZeros_ConcurrentWithGetDigit_NeverThrowsAndDigitsInRange#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreToStringParallelTests/ToString_Given100Digits_ReturnsCorrectDigitOrder#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreToStringParallelTests/ToString_Given19Digits_ReturnsCorrectDigitOrder#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreToStringParallelTests/ToString_Given20DigitsWithSeparator_ReturnsCorrectSeparatedString#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreToStringParallelTests/ToString_Given20Digits_ReturnsCorrectDigitOrder#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreToStringParallelTests/ToString_Given99Digits_ReturnsCorrectDigitOrder#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTrimLeadingZerosTests/TrimLeadingZeros_GivenAllZeroDigits_ResetsToZeroState#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTrimLeadingZerosTests/TrimLeadingZeros_GivenEvenDigitCountWithSingleLeadingZero_TrimsToSingleDigit#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTrimLeadingZerosTests/TrimLeadingZeros_GivenMultipleLeadingZeroDigits_DigitCountReducedAndValuePreserved#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTrimLeadingZerosTests/TrimLeadingZeros_GivenNoLeadingZeros_LeavesValueUnchanged#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTrimLeadingZerosTests/TrimLeadingZeros_GivenSingleNonZeroDigit_LeavesValueUnchanged#0
Lovelace.Representation.Tests/Lovelace.Representation.Tests.DigitStoreTrimLeadingZerosTests/TrimLeadingZeros_GivenSingleZeroDigit_ResetsToZeroState#0
```

Not mapped, deliberately: `Lovelace.Representation.Tests/Lovelace.Representation.Tests.UnitTest1/Test1#0`.
The C# body is empty (`UnitTest1.cs:5-9`), so there is no input or expected output to mirror;
adding a vacuous Rust test would claim a behavioural mapping that does not exist.

## 7. Verification

| Check | Command | Result |
|---|---|---|
| DigitStore tests | `cargo test -p lovelace-nat digit_store` | **92 passed, 0 failed** (85 mirrored + 7 additive) |
| Whole crate | `cargo test -p lovelace-nat` | **226 passed, 0 failed** (134 pre-existing + 92 new) |
| Workspace | `cargo test --workspace` | **4,458 passed, 0 failed across 19 suites**, exit 0; isolated contribution 4,355 baseline + 92 new = 4,447, and the shared worktree simultaneously gained 11 `Lovelace.Symbolics` tests, giving the measured 4,458 |
| Corpus | `rust\\target\\release\\lovelace-corpus.exe` with `TMP=rust/target/tmp` | **212 passed, 2 failed, 214 total**; the two failures are the pre-existing `sym/071` and `sym/076` |
| Fixtures | `node parity/tools/gate-fixtures.mjs` | **PASS: oracle 20/20, rust 20/20** |
| CLI | `node parity/tools/gate-cli.mjs` | **CLI matrix: 16/16**, PASS |

The first corpus invocation in this session ran without the mandated `TMP` value and failed
`lang/076` with `Cannot write plot: Access is denied` (211/214); rerunning with
`TMP=rust/target/tmp` produced the stable 212/214 result above. `sym/071` and `sym/076` are
the same two failures recorded in the round-23/round-30 reports and are outside this change.

The new tests appear in the inventory as `digit_store::tests::<C# method name>` (crate
`lovelace-nat`). `parity/rust-tests.json`, `parity/manifest-map.json` and every other
`parity/` artifact were **not** regenerated or edited; the porting lane owns those mappings.

## 8. Files and what remains

Changed by this round:

| File | Change |
|---|---|
| `rust/lovelace-nat/src/digit_store.rs` | new: the module, API surface and 91 tests |
| `rust/lovelace-nat/src/lib.rs` | additive: `pub mod digit_store;` and `pub use digit_store::{DigitStore, DigitStoreError};` |
| `docs/goal-parity-contract/round-31/digit-store.md` | this report |

Remaining for the parent/porting lane:

1. Add the 90 census ids in section 6 to `parity/manifest-map.json`, each pointing at the
   matching `digit_store::tests::<method>` test, and regenerate the inventory. This report does
   not edit `parity/`.
2. Decide the disposition of the empty `UnitTest1/Test1#0` placeholder; there is no behaviour to
   port.
3. If a later consumer actually needs `DigitStore` inside `Nat`, that is a separate production
   integration. This round deliberately does not change `Nat`: the C# `Lovelace.Natural` in this
   checkout does not consume `DigitStore` either (its README claim is stale), so the 91 rows are
   a library-surface gap, not an arithmetic gap.
