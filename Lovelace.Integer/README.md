# Lovelace.Integer

Signed arbitrary-precision integer type for the LovelaceSharp library, backed by a `Natural` magnitude and a Boolean sign flag.

This project sits between `Lovelace.Natural` and `Lovelace.Real` in the dependency chain and is migrated from the C++ `InteiroLovelace` class.

---

## Class: `Integer`

**Namespace:** `Lovelace.Integer`

`Integer` represents any element of ℤ (positive, negative, or zero) with no fixed size limit. Magnitude storage is delegated entirely to `Lovelace.Natural.Natural`; `Integer` adds a `_isNegative` flag and sign-aware arithmetic.

Zero is always normalised to positive (there is no "negative zero").

---

## Public API

### Constructors

| Signature | Behaviour |
|---|---|
| `Integer()` | Default — magnitude = 0, sign = positive. |
| `Integer(long value)` | Infers sign from `value < 0`; handles `long.MinValue` without overflow. |
| `Integer(int value)` | Delegates to `Integer((long)value)`. |
| `Integer(Natural magnitude)` | Wraps a `Natural` with positive sign. |
| `Integer(Natural magnitude, bool isNegative)` | Internal raw constructor; normalises sign of zero (zero is always positive). |
| `Integer(string s)` | Parses a decimal string, optionally prefixed with `-`. Throws `FormatException` on invalid input. |
| `Integer(ReadOnlySpan<char> s)` | Span-based parsing variant. Throws `FormatException` on invalid input. |

### Properties

| Property | Type | Description |
|---|---|---|
| `Sign` | `int` | Returns `−1`, `0`, or `+1` depending on the integer's sign. |
| `One` | `static Integer` | The multiplicative identity (1). |
| `Zero` | `static Integer` | The additive identity (0). |
| `NegativeOne` | `static Integer` | The value −1. |
| `Radix` | `static int` | `10` — decimal representation. |
| `AdditiveIdentity` | `static Integer` | Same as `Zero`. |
| `MultiplicativeIdentity` | `static Integer` | Same as `One`. |

### Static predicates

| Method | Description |
|---|---|
| `IsZero(Integer)` | `true` when magnitude is zero. |
| `IsPositive(Integer)` | `true` when sign flag is not set (not negative). |
| `IsNegative(Integer)` | `true` when sign flag is set (required by `ISignedNumber<T>`). |
| `IsEvenInteger(Integer)` | Delegates to `Natural.IsEvenInteger` on the magnitude. |
| `IsOddInteger(Integer)` | Delegates to `Natural.IsOddInteger` on the magnitude. |

### Internal helper

```csharp
Natural ToNatural()
```
Returns the magnitude as a `Natural` (strips sign). Corresponds to C++ `toLovelace`.

### Negation

```csharp
Integer Negate()
static Integer operator -(Integer value)  // unary
static Integer operator +(Integer value)  // unary — returns unchanged copy
```
`Negate()` returns a copy with the sign flipped; zero remains positive.

### Arithmetic

```csharp
Integer Add(Integer other)
static Integer operator +(Integer left, Integer right)
```
Same-sign operands: add magnitudes, keep sign.  
Different-sign operands: subtract magnitudes; result sign follows the operand with the larger magnitude.

```csharp
Integer Subtract(Integer other)
static Integer operator -(Integer left, Integer right)
```
Implemented as `Add(other.Negate())`.

```csharp
Integer Multiply(Integer other)
static Integer operator *(Integer left, Integer right)
```
Multiplies magnitudes; result is negative iff exactly one operand is negative (XOR of signs).

```csharp
Integer DivRem(Integer divisor, out Integer remainder)
static Integer operator /(Integer left, Integer right)   // quotient
static Integer operator %(Integer left, Integer right)   // remainder
```
Divides magnitudes via `Natural.DivRem`; quotient and remainder signs follow the equal-signs rule (same-sign operands → positive quotient; different-sign → negative).

```csharp
Integer Pow(Integer exponent)
```
Raises this integer to `exponent`. Guards: base ≠ 0, exponent > 0.  
Result is negative iff base is negative **and** exponent is odd.  
Throws `ArgumentOutOfRangeException` for zero base or non-positive exponent.

```csharp
Integer Factorial()
```
Returns `n!`. Throws `InvalidOperationException` for negative input.  
Delegates to `Natural.Factorial()` after the sign check.

### Increment / Decrement

```csharp
Integer Increment()
Integer Decrement()
static Integer operator ++(Integer value)
static Integer operator --(Integer value)
```
`Increment()` returns `this + 1`; `Decrement()` returns `this − 1`.  
The operators follow standard C# pre-increment/pre-decrement semantics.

### Equality and Comparison

```csharp
bool Equals(Integer? other)
int  CompareTo(Integer? other)
static bool operator ==(Integer, Integer)
static bool operator !=(Integer, Integer)
static bool operator > (Integer, Integer)
static bool operator >=(Integer, Integer)
static bool operator < (Integer, Integer)
static bool operator <=(Integer, Integer)
```
`Equals` checks sign equality first (both zero → equal regardless of sign flag), then delegates to `Natural.Equals` for magnitude.  
`CompareTo`: positive > negative for cross-sign; same-sign delegates to `Natural.CompareTo` with magnitude flip when both negative.

### Formatting

```csharp
override string ToString()
string ToString(string? format, IFormatProvider? provider)
bool TryFormat(Span<char> destination, out int charsWritten, ReadOnlySpan<char> format, IFormatProvider? provider)
```
Prepends `−` for negative values then delegates to `Natural.ToString()`.  
`TryFormat` writes to the span and returns `false` if the buffer is too small.

### Parsing

```csharp
static Integer Parse(string s, IFormatProvider? provider)
static Integer Parse(ReadOnlySpan<char> s, IFormatProvider? provider)
static bool TryParse(string? s, IFormatProvider? provider, out Integer result)
static bool TryParse(ReadOnlySpan<char> s, IFormatProvider? provider, out Integer result)
```
Strips leading whitespace; recognises optional leading `−`; passes remaining digits to `Natural.Parse`.  
`Parse` throws `FormatException` on invalid or empty input.  
`TryParse` returns `false` without throwing.

### Implemented interfaces

- `ISignedNumber<Integer>`
- `INumber<Integer>`
- `IComparable<Integer>`, `IEquatable<Integer>`
- `IParsable<Integer>`, `ISpanParsable<Integer>`
- `ISpanFormattable`
- `IUnaryNegationOperators<Integer, Integer>`, `IUnaryPlusOperators<Integer, Integer>`
- `IAdditionOperators<Integer, Integer, Integer>`
- `ISubtractionOperators<Integer, Integer, Integer>`
- `IMultiplyOperators<Integer, Integer, Integer>`
- `IDivisionOperators<Integer, Integer, Integer>`
- `IModulusOperators<Integer, Integer, Integer>`
- `IIncrementOperators<Integer>`, `IDecrementOperators<Integer>`
- `IComparisonOperators<Integer, Integer, bool>`

---

## Usage

```csharp
using Lovelace.Integer;
using Lovelace.Natural;

// Construction
var a = new Integer(42L);
var b = new Integer(-7L);
var c = Integer.Parse("1000000000000000000000000000000");

// Arithmetic
var sum      = a + b;         // 35
var product  = a * b;         // -294
var quotient = a / new Integer(5L);   // 8
var rem      = a % new Integer(5L);   // 2
var power    = new Integer(2L).Pow(new Integer(32L)); // 4294967296

// Predicates
bool isNeg  = Integer.IsNegative(b);   // true
bool isEven = Integer.IsEvenInteger(a); // true

// Formatting
Console.WriteLine(product); // "-294"

// Parsing
var parsed = Integer.Parse("-00042", null); // Integer(-42)
```

---

## See also

- [Requirements document](.github/requirements/Lovelace.Integer.md)
- [Legacy C++ source](../Legacy/InteiroLovelace.hpp) / [`InteiroLovelace.cpp`](../Legacy/InteiroLovelace.cpp)
- Dependency chain:  
  [`Lovelace.Representation`](../Lovelace.Representation/README.md) ← [`Lovelace.Natural`](../Lovelace.Natural/README.md) ← **`Lovelace.Integer`** ← `Lovelace.Real`
