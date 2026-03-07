# Lovelace.Real

Arbitrary-precision real number class for the LovelaceSharp library, built on top of `Lovelace.Integer` and migrated from the C++ `RealLovelace` class.

---

## Class: `Real`

**Namespace:** `Lovelace.Real`

`Real` extends `Integer` with a decimal exponent and optional period metadata, enabling exact representation of rational numbers via periodic decimal notation (e.g. `"0.(3)"`, `"0.1(6)"`, `"0.(142857)"`).

- The **decimal exponent** (`Exponent`) records how many fractional digits are stored — a value of `-2` means there are 2 digits to the right of the decimal point.
- **Period metadata** (`PeriodStart`, `PeriodLength`) pin-point the repeating block of a rational quotient discovered during division.
- Non-periodic values are stored and displayed up to `DisplayDecimalPlaces` fractional digits. Periodic values are stored compactly (one block) and reproduced exactly on output.

Migrated from C++ `RealLovelace` (≥ `Lovelace.Integer` ≥ `Lovelace.Natural` ≥ `Lovelace.Representation`).

---

## Public API

### Constructors

| Signature | Behaviour |
|---|---|
| `Real()` | Value zero, `Exponent = 0`, no period. |
| `Real(double value)` | Parses the double's round-trip string (`"R"` format) via `Parse`. |
| `Real(decimal value)` | Converts via `value.ToString("G29", CultureInfo.InvariantCulture)` then `Parse`; preserves up to 29 significant digits. |
| `Real(string value)` | Delegates to `Parse(value)`. Accepts plain decimal (`"3.14"`) or periodic notation (`"0.(142857)"`). |
| `Real(ReadOnlySpan<char> value)` | Delegates to `Parse(value, null)`. Same formats as `Real(string)`. |
| `Real(Real other)` | Deep-copies digits, exponent, sign, zero flag, `PeriodStart`, `PeriodLength`. |
| `Real(Integer other)` | Copies digits and sign from an `Integer`; sets `Exponent = 0`. |

### Instance Properties

| Property | Type | Description |
|---|---|---|
| `Exponent` | `long` | Decimal exponent (`0` = integer, `-n` = n fractional digits). Readable and writable. |
| `PeriodStart` | `long` | Zero-based fractional-digit index where the repeating block begins. Read-only after construction. |
| `PeriodLength` | `long` | Length of the repeating block; `0` = non-periodic. Read-only after construction. |
| `IsPeriodic` | `bool` | Computed: `PeriodLength > 0`. |

### Static Configuration Properties

| Property | Type | Default | Description |
|---|---|---|---|
| `DisplayDecimalPlaces` | `long` | `100` | Maximum fractional digits emitted by `ToString()` for non-periodic values. Thread-safe via `Interlocked`. |
| `MaxComputationDecimalPlaces` | `long` | `1000` | Hard cap on fractional digits generated during division and periodic-operand arithmetic before giving up on period detection. Thread-safe via `Interlocked`. |

### Static Constants

`Real.One`, `Real.Zero`, `Real.NegativeOne`, `Real.Radix`, `Real.AdditiveIdentity`, `Real.MultiplicativeIdentity` — inherited analogues returning `Real` instances.

### Methods and Operators

#### Arithmetic

| Member | Description |
|---|---|
| `static Real Add(Real, Real)` / `operator+` | Aligns exponents; uses `GetDecimalDigit` for periodic operands; detects period in result. |
| `static Real Subtract(Real, Real)` / `operator-` (binary) | Negates the right operand then calls `Add`. |
| `static Real Multiply(Real, Real)` / `operator*` | Multiplies magnitudes; result exponent = sum of operand exponents; periodic operands expanded via `GetDecimalDigit`. |
| `static Real Divide(Real, Real)` / `operator/` | Remainder-tracked long division with exact period detection via a remainder-history dictionary; falls back to `MaxComputationDecimalPlaces` truncation for irrationals. Throws `DivideByZeroException` for zero divisor. |
| `static Real Negate(Real)` / `operator-` (unary) | Flips the sign; preserves exponent and period metadata; zero stays positive. |
| `Real Invert()` | Computes `Real.One / this`. Throws `DivideByZeroException` for zero. |
| `Real Pow(Real exponent)` | Integer-exponent fast path via binary exponentiation. Non-integer or negative exponents throw `NotImplementedException`. |
| `Real Assign(Real other)` | Returns a new `Real` that is a deep copy of `other`. |
| `operator++` | Increments by `Real.One` (i.e. `value + Real.One`). |
| `operator--` | Decrements by `Real.One` (i.e. `value - Real.One`). |
| `operator%` | Truncated remainder: `left - Truncate(left / right) * right` (truncated towards zero). Throws `DivideByZeroException` for zero divisor. |

#### Comparison and Equality

| Member | Description |
|---|---|
| `bool Equals(Real?)` | Structural equality (exponent-aligned digit-by-digit for non-periodic; `ToString()` comparison for periodic). |
| `int CompareTo(Real?)` | Negative/zero/positive trichotomy with exponent alignment. |
| `operator ==`, `!=`, `<`, `<=`, `>`, `>=` | Delegate to `Equals` / `CompareTo`. |

#### Static Predicates

`IsZero`, `IsPositive`, `IsNegative`, `IsEvenInteger`, `IsOddInteger`, `IsInteger`, `Abs`, `MaxMagnitude`, `MinMagnitude` — all inherited from `Integer` and overridden to return `Real` where applicable.

#### Parsing

| Member | Description |
|---|---|
| `static Real Parse(string s)` / `Parse(ReadOnlySpan<char>, IFormatProvider?)` | Parses sign, integer part, optional `.` and fractional part; also handles periodic notation `"0.(142857)"` — sets `PeriodStart` / `PeriodLength`. |
| `static bool TryParse(string?, IFormatProvider?, out Real)` / span overload | Non-throwing variant of `Parse`; returns `false` on invalid input. |

#### Formatting

| Member | Description |
|---|---|
| `string ToString()` | Sign + integer digits + optional `.` + fractional digits (truncated at `DisplayDecimalPlaces` for non-periodic) or `(period_block)` for periodic. Examples: `"3.14"`, `"0.(3)"`, `"0.1(6)"`. |
| `string ToString(string?, IFormatProvider?)` | Delegates to parameterless `ToString()`. |
| `bool TryFormat(Span<char>, out int, ReadOnlySpan<char>, IFormatProvider?)` | Writes the same content as `ToString()` into the provided span; returns `false` when the span is too short. |

### Implemented Interfaces

- `INumber<Real>`
- `ISignedNumber<Real>`
- `IComparable<Real>`
- `IEquatable<Real>`
- `IParsable<Real>` / `ISpanParsable<Real>`
- `ISpanFormattable`
- `IAdditionOperators<Real, Real, Real>`
- `ISubtractionOperators<Real, Real, Real>`
- `IMultiplyOperators<Real, Real, Real>`
- `IDivisionOperators<Real, Real, Real>`
- `IUnaryNegationOperators<Real, Real>`
- `IIncrementOperators<Real>`
- `IDecrementOperators<Real>`
- `IModulusOperators<Real, Real, Real>`
- `IComparisonOperators<Real, Real, bool>`

---

## Usage

```csharp
using Lovelace.Real;

// Construction
Real a = new Real("3.14");       // from string
Real b = new Real(2.71828);      // from double
Real c = Real.Parse("0.(3)");    // periodic 1/3

// Arithmetic
Real sum     = a + b;            // "5.85828"
Real product = a * new Real("2"); // "6.28"
Real third   = Real.Parse("1") / Real.Parse("3");
// third.IsPeriodic == true
// third.ToString() == "0.(3)"

Real inv = new Real("4").Invert();
// inv.ToString() == "0.25"  (IsPeriodic == false)

// Formatting
Console.WriteLine(third.ToString());  // "0.(3)"

Real.DisplayDecimalPlaces = 5;
Real pi = new Real("3.1415926535");
Console.WriteLine(pi.ToString());     // "3.14159"

// Parsing periodic notation
Real r = Real.Parse("0.1(6)");  // 1/6
// r.PeriodStart == 1, r.PeriodLength == 1
// r.ToString()  == "0.1(6)"
```

---

## See also

- [Requirements document](.github/requirements/Lovelace.Real.md)
- Legacy C++ source: [`Legacy/RealLovelace.hpp`](../Legacy/RealLovelace.hpp), [`Legacy/RealLovelace.cpp`](../Legacy/RealLovelace.cpp)
- Dependency chain: [`Lovelace.Representation`](../Lovelace.Representation/README.md) ← [`Lovelace.Natural`](../Lovelace.Natural/README.md) ← [`Lovelace.Integer`](../Lovelace.Integer/README.md) ← **`Lovelace.Real`**
