# Lovelace.Representation

Binary-Coded Decimal (BCD) digit storage layer for the LovelaceSharp arbitrary-precision number library.

This is the **only** project in the solution that is allowed to read or write the backing `byte[]` array directly. Every other project (`Lovelace.Natural`, `Lovelace.Integer`, `Lovelace.Real`) must go through the `DigitStore` API.

---

## Class: `DigitStore`

**Namespace:** `Lovelace.Representation`

Stores an arbitrary sequence of decimal digits (0–9) packed two per byte using BCD encoding:

- **High nibble** (bits 7–4) → even-indexed digit (position 0, 2, 4, …)
- **Low nibble** (bits 3–0) → odd-indexed digit (position 1, 3, 5, …)

Position `0` is the **least-significant digit** (little-endian digit order).

### Sentinel values

| Value | Meaning |
|---|---|
| `0x0C` | Byte slot allocated but not yet written (appended by `GrowDigits`) |
| `0x0F` | Low nibble freed (written by `ShrinkDigits` for even `DigitCount`) |

---

## Public API

### Constructors

| Signature | Behaviour |
|---|---|
| `DigitStore()` | Creates an empty zero store. `DigitCount = 0`, `IsZero = true`, `ByteCount = 0`. |
| `DigitStore(DigitStore other)` | Deep-copies all bytes and metadata from `other`. No-op when `other.IsZero` is true (no bytes to copy). |

### Properties

| Property | Type | Description |
|---|---|---|
| `DigitCount` | `long` | Number of logical decimal digits stored. `internal set`. |
| `ByteCount` | `long` | Number of bytes in the backing store. Read-only. |
| `IsZero` | `bool` | `true` when the stored value is zero. `internal set`. Cleared only when `SetDigit(0, …)` is called. |

### Digit access

```csharp
byte GetDigit(long position)
```
Returns the decimal digit (0–9) at `position`.  
- Position 0 = least-significant digit.  
- Returns `0` when `IsZero` is `true` or `position` is out of `[0, DigitCount)`.

```csharp
void SetDigit(long position, byte digit)
```
Writes a decimal digit (0–9) at `position`.  
- **Sequential writes only**: `position` must be ≤ `DigitCount`. Throws `ArgumentOutOfRangeException` otherwise.  
- Calling `SetDigit(0, …)` sets `IsZero = false`.  
- Automatically grows the backing store when a new byte slot is needed.

### Formatting

```csharp
string ToString()
string ToString(char separator)
```
Returns the decimal string representation with most-significant digit first.  
- `ToString()` calls `ToString('\0')` (no separator).  
- When `separator != '\0'`, it is inserted every three digits from the right (e.g. thousands separator).  
- Returns `"0"` when `IsZero` is `true`.

```csharp
void Dump(bool showNibbles = false)
```
Debug helper. Prints `ByteCount`, `DigitCount`, `IsZero`, and the formatted value (with `'.'` as separator) to `Console`. Pass `showNibbles: true` to also print raw nibble pairs.

---

## Internal API (visible to `Lovelace.Representation.Tests` via `InternalsVisibleTo`)

These members are used by upper-layer projects and tests but must **not** be called from outside the `Lovelace.*` namespace.

| Member | Description |
|---|---|
| `GetBitwise(long pos, out byte high, out byte low)` | Splits byte at `pos` into high and low nibbles. Silent no-op when out of range. |
| `SetBitwise(long pos, byte high, byte low)` | Packs two nibbles into the byte at `pos`. Calls `GrowDigits()` when `pos == ByteCount`. Silent no-op when `pos > ByteCount`. |
| `GrowDigits()` | Appends sentinel byte `0x0C` — increases `ByteCount` by 1. |
| `ShrinkDigits()` | Removes the most-significant digit slot. Odd `DigitCount` → removes last byte; Even → sets low nibble to `0x0F`. Decrements `DigitCount`. |
| `ClearDigits()` | Clears all bytes from the backing list. Does not reset metadata. |
| `CopyDigitsFrom(DigitStore other)` | Deep-copies backing bytes from `other`. No-op when `other` is the same instance or `IsZero`. |
| `Initialize()` | Resets `DigitCount = 0` and `IsZero = true` without touching the byte list. |
| `Reset()` | Clears bytes and reinitializes to zero state. No-op when already zero. |

---

## Digit ordering examples

Writing the number `12345`:

```csharp
store.SetDigit(0, 5); // LSB
store.SetDigit(1, 4);
store.SetDigit(2, 3);
store.SetDigit(3, 2);
store.SetDigit(4, 1); // MSB
store.ToString(); // "12345"
```

Byte layout after those five writes (`ByteCount = 3`):

```
_bytes[0]: high=5, low=4   (digit positions 0 and 1)
_bytes[1]: high=3, low=2   (digit positions 2 and 3)
_bytes[2]: high=1, low=0xF (digit position 4; sentinel low nibble)
```

---

## Constraints and rules

1. **Only `Lovelace.Representation` may access `_bytes` directly.** All upper-layer code uses `GetDigit`/`SetDigit`.
2. Digits are stored **little-endian** (position 0 = least significant).
3. `SetDigit` enforces **strict sequential writes**: you cannot write position `n+2` before position `n+1`.
4. `IsZero` is only cleared by writing to position `0`. Resetting a store must go through `Reset()`.
5. `ByteCount` is always `⌈DigitCount / 2⌉`.

---

## Running the tests

```bash
dotnet test ../Lovelace.Representation.Tests/Lovelace.Representation.Tests.csproj
```
