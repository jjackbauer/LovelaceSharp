# Skill: Using `DigitStore` from Upper-Layer Classes

> **Usage**: Include this file whenever implementing or reviewing `Lovelace.Natural`, `Lovelace.Integer`, or `Lovelace.Real`.
> ```
> #file:.github/prompts/skill-use-digit-store.prompt.md
> ```

---

## What `DigitStore` is

`DigitStore` (`Lovelace.Representation`) is the sole BCD digit storage layer.  
Upper-layer classes **own** a `DigitStore` instance (composition, not inheritance) and interact with it exclusively through the public and internal members listed below.

**Absolute rule**: No class outside `Lovelace.Representation` may read or write the backing `byte[]` directly.

---

## The digit model

- Digits are stored **little-endian**: position `0` is the **least-significant** digit.
- `GetDigit(0)` → units digit; `GetDigit(1)` → tens digit; etc.
- `ToString()` returns digits **most-significant first** (normal reading order).
- `DigitCount` tracks how many logical digits are stored.
- `IsZero == true` means the represented value is zero; `GetDigit` returns `0` for all positions in that state.

### Writing digits

Writes must be **sequential**: you can only write position `n` after position `n-1` already exists, i.e. `position ≤ DigitCount`.

```csharp
// Build "42" in a fresh store (LSB-first)
var store = new DigitStore();
store.SetDigit(0, 2); // units    → IsZero becomes false
store.SetDigit(1, 4); // tens
// store.ToString() == "42"
```

### Reading digits

```csharp
for (long i = 0; i < store.DigitCount; i++)
{
    byte d = store.GetDigit(i); // 0–9, LSB at i=0
}
```

Out-of-range reads return `0` silently — no exception.

---

## Common patterns used by upper-layer classes

### Constructing from a `ulong` (or similar primitive)

```csharp
// Decompose into digits LSB-first and write sequentially
ulong value = 123456789UL;
long pos = 0;
if (value == 0)
{
    // leave store in default zero state
}
else
{
    while (value > 0)
    {
        _store.SetDigit(pos++, (byte)(value % 10));
        value /= 10;
    }
}
```

### Copying another number's store

```csharp
// Use the DigitStore copy constructor — deep copy in one call
_store = new DigitStore(other._store);
```

Or use `CopyDigitsFrom` when the target store already exists:

```csharp
_store.Reset();
_store.DigitCount = other._store.DigitCount;
_store.IsZero     = other._store.IsZero;
_store.CopyDigitsFrom(other._store);
```

### Resetting to zero

```csharp
_store.Reset(); // clears bytes and sets IsZero=true, DigitCount=0
```

### Shrinking the most-significant digit

Used when an arithmetic result has a leading zero that must be trimmed:

```csharp
while (_store.DigitCount > 1 && _store.GetDigit(_store.DigitCount - 1) == 0)
    _store.ShrinkDigits();
```

`ShrinkDigits` handles the BCD packing automatically (removes the byte when `DigitCount` was odd, otherwise marks the low nibble as freed with `0x0F`).

### Growing the store for a new digit

Upper-layer arithmetic rarely needs to call `GrowDigits` / `SetBitwise` directly — prefer `SetDigit` which calls `GrowDigits` internally. Use the low-level pair only when you need fine-grained control over nibble packing:

```csharp
// Equivalent to SetDigit — prefer SetDigit in normal code
_store.SetBitwise(pos / 2, highNibble, lowNibble);
```

---

## Interacting with `IsZero` and `DigitCount`

Both have `internal set`, accessible from the same assembly and from `Lovelace.Representation.Tests` (via `InternalsVisibleTo`).  
Upper-layer classes in separate assemblies **can read** these properties, but **cannot set them directly** — they must go through `SetDigit`, `Reset`, `Initialize`, or `ShrinkDigits`.

If an upper-layer class needs to manipulate these directly (e.g. during arithmetic result normalisation), it must do so by calling the public/internal mutating methods, not by assigning the property.

---

## `ToString` behaviour

```csharp
store.ToString()         // no separator, e.g. "1234567"
store.ToString(',')      // thousands separator, e.g. "1,234,567"
store.ToString('\0')     // same as ToString()
```

`ToString` always returns `"0"` when `IsZero` is `true`, regardless of what bytes may be in the store.

---

## Error conditions

| Scenario | Behaviour |
|---|---|
| `SetDigit(position < 0)` | `ArgumentOutOfRangeException` |
| `SetDigit(position > DigitCount)` | `ArgumentOutOfRangeException` (gap write not allowed) |
| `GetDigit` out of range (`position >= DigitCount` or negative) | Returns `0` silently |
| `SetBitwise` beyond `ByteCount + 1` | Silent no-op |
| `GetBitwise` out of range | Returns `high=0, low=0` silently |

---

## Quick-reference: which method to call

| Goal | Call |
|---|---|
| Read digit at position `i` | `store.GetDigit(i)` |
| Write next digit (append) | `store.SetDigit(store.DigitCount, digit)` |
| Overwrite existing digit | `store.SetDigit(i, digit)` where `i < store.DigitCount` |
| Deep-copy entire store | `new DigitStore(source)` |
| Reset to zero | `store.Reset()` |
| Remove MSD | `store.ShrinkDigits()` |
| Number of digits | `store.DigitCount` |
| Number of bytes | `store.ByteCount` (always `⌈DigitCount / 2⌉`) |
| Is it zero? | `store.IsZero` |
| Format as string | `store.ToString()` or `store.ToString(separator)` |
