# Lovelace.Natural

Arbitrary-precision natural number (`ℕ₀`) arithmetic for the LovelaceSharp library. Builds on `Lovelace.Representation` and is itself the foundation for `Lovelace.Integer`.

---

## Class: `Natural`

**Namespace:** `Lovelace.Natural`

Represents a non-negative integer of unlimited size. All arithmetic is performed digit-by-digit via the `DigitStore` BCD backing store — no digit is ever read from or written to the raw `byte[]` directly from this layer.

`Natural` is a `sealed class` that implements every relevant `System.Numerics` generic-math interface, making it usable in any generic algorithm constrained to `INumber<T>`.

---

## Public API

### Constructors

| Signature | Behaviour |
|---|---|
| `Natural()` | Produces zero. |
| `Natural(Natural other)` | Deep copy of `other` (independent backing store). |
| `Natural(ulong value)` | Constructs from an unsigned 64-bit integer. |
| `Natural(int value)` | Constructs from a non-negative `int`. Throws `ArgumentOutOfRangeException` for negative values. |
| `Natural(string s)` | Parses a decimal digit string. Equivalent to `Parse(s)`. Throws `FormatException` for empty or non-digit input. |
| `Natural(ReadOnlySpan<char> s)` | Parses a span of decimal digit characters. Equivalent to the span `Parse` overload. Throws `FormatException` for empty or non-digit input. |

### Static Properties

| Property | Type | Description |
|---|---|---|
| `Zero` | `Natural` | Additive identity (0). |
| `One` | `Natural` | Multiplicative identity (1). |
| `Radix` | `int` | Always `10` (decimal). |
| `AdditiveIdentity` | `Natural` | Alias for `Zero`. |
| `MultiplicativeIdentity` | `Natural` | Alias for `One`. |
| `DisplayDigits` | `long` | Maximum digits to display when formatting. `-1` means no limit. Mirrors C++ `algarismosExibicao`. |
| `Precision` | `long` | Precision hint. `-1` by default (body absent in C++ source). |

### Classification Predicates

| Method | Description |
|---|---|
| `static bool IsZero(Natural value)` | `true` when the value is zero. |
| `static bool IsEvenInteger(Natural value)` | `true` when the least-significant digit is even (0 is even). |
| `static bool IsOddInteger(Natural value)` | `true` when the least-significant digit is odd (0 is never odd). |
| `static bool IsPositive(Natural value)` | Always `true` (ℕ₀ is always ≥ 0). |
| `static bool IsNegative(Natural value)` | Always `false`. |
| `static bool IsInteger(Natural value)` | Always `true`. |
| `static bool IsFinite(Natural value)` | Always `true`. |

### Comparison

| Member | Description |
|---|---|
| `bool Equals(Natural? other)` | Value equality via digit-by-digit comparison. |
| `int CompareTo(Natural? other)` | Standard three-way comparison. |
| `operator ==`, `!=` | Value equality. |
| `operator >`, `>=`, `<`, `<=` | Value ordering. |

### Arithmetic Operators

| Operator | Description |
|---|---|
| `operator +(Natural left, Natural right)` | Addition with carry propagation. |
| `operator -(Natural left, Natural right)` | Subtraction. Throws `InvalidOperationException` when `right > left` (ℕ₀ has no negatives). |
| `operator *(Natural left, Natural right)` | Long multiplication (grade-school algorithm). |
| `operator /(Natural left, Natural right)` | Integer division (quotient only); delegates to `DivRem`. |
| `operator %(Natural left, Natural right)` | Remainder; delegates to `DivRem`. |
| `operator ++(Natural value)` | Prefix and postfix increment (adds 1). |
| `operator --(Natural value)` | Prefix and postfix decrement. Throws `InvalidOperationException` when value is zero. |

### Methods

```csharp
static Natural DivRem(Natural left, Natural right, out Natural remainder)
```
Long division returning the quotient; sets `remainder` to the remainder.  
Throws `DivideByZeroException` when `right` is zero.

```csharp
Natural DivRem(Natural divisor, out Natural remainder)
```
Instance convenience overload of the above.

```csharp
Natural Pow(Natural exponent)
```
Raises this value to `exponent` using **binary (repeated-squaring) exponentiation** — O(log n) multiplications rather than O(n).  
Any base raised to exponent zero returns `One`.

```csharp
Natural Factorial()
```
Returns `this!`. Returns `One` for `0! = 1! = 1`.  
For large values (n > 2 × `Environment.ProcessorCount`), uses a **parallel tree reduction**: the factor range `[2..n]` is partitioned into `ProcessorCount` sub-ranges multiplied concurrently via `Parallel.For`, then the partial products are combined serially.

### Parsing

```csharp
static Natural Parse(string s)
static Natural Parse(string s, IFormatProvider? provider)
static Natural Parse(ReadOnlySpan<char> s, IFormatProvider? provider)
```
Parses a decimal string. Leading zeros are silently trimmed.  
Throws `FormatException` for empty, null, or non-digit input.

```csharp
static bool TryParse(string? s, out Natural result)
static bool TryParse(string? s, IFormatProvider? provider, out Natural result)
static bool TryParse(ReadOnlySpan<char> s, IFormatProvider? provider, out Natural result)
```
Non-throwing counterparts; return `false` on invalid input.

### Formatting

```csharp
override string ToString()
```
Returns the decimal string with most-significant digit first. No leading zeros. Returns `"0"` for zero.  
Delegates to `DigitStore.ToString()`, which already parallelises byte extraction internally via `Parallel.For`.

```csharp
string ToString(string? format, IFormatProvider? formatProvider)
```
Format `"N"` or `"n"` inserts a comma thousands separator (mirrors C++ `imprimir(char separador)`). All other formats fall back to plain decimal.

```csharp
bool TryFormat(Span<char> destination, out int charsWritten, ReadOnlySpan<char> format, IFormatProvider? provider)
```
Writes formatted characters to `destination`. Returns `false` if the buffer is too small.

### Implemented Interfaces

- `INumber<Natural>`
- `IComparable<Natural>`, `IEquatable<Natural>`
- `IParsable<Natural>`, `ISpanParsable<Natural>`
- `ISpanFormattable`
- `IAdditionOperators<Natural, Natural, Natural>`
- `ISubtractionOperators<Natural, Natural, Natural>`
- `IMultiplyOperators<Natural, Natural, Natural>`
- `IDivisionOperators<Natural, Natural, Natural>`
- `IModulusOperators<Natural, Natural, Natural>`
- `IIncrementOperators<Natural>`
- `IDecrementOperators<Natural>`
- `IComparisonOperators<Natural, Natural, bool>`

---

## Usage

```csharp
using Lovelace.Natural;

// Construction
var a = new Natural(12345UL);
var b = Natural.Parse("99999999999999999999");

// Arithmetic
Natural sum = a + b;
Natural product = a * new Natural(3UL);
Natural quotient = Natural.DivRem(b, a, out Natural remainder);

// Formatting
Console.WriteLine(sum);                         // "100000000000000012344"
Console.WriteLine(product.ToString("N", null)); // "37,035"

// Predicates
Console.WriteLine(Natural.IsZero(new Natural())); // True
Console.WriteLine(Natural.IsEvenInteger(sum));     // true/false

// Parsing round-trip
var n = Natural.Parse("314159265358979323846");
Console.WriteLine(n); // "314159265358979323846"
```

---

## See Also

- [Requirements & test plan](.github/requirements/Lovelace.Natural.md)
- [Parallelization audit](.github/requirements/Lovelace.Natural-parallelization-audit.md)
- [Legacy C++ source](../Legacy/Lovelace.hpp) / [implementation](../Legacy/Lovelace.cpp)
- Depends on: [`Lovelace.Representation`](../Lovelace.Representation/README.md)
- Used by: [`Lovelace.Integer`](../Lovelace.Integer/README.md)
