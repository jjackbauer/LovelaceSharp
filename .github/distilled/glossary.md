# Glossary

> **Scope**: Domain terminology — Portuguese ↔ English name mapping and BCD-encoding vocabulary for the LovelaceSharp codebase
> **Confidence**: Medium
> **Last updated**: 2026-03-12
> **Source entries**: OBS-001, OBS-002, OBS-005, OBS-008, OBS-011, OBS-021, OBS-026, OBS-027, VAL-006, VAL-007

---

## Purpose

This document provides a single, findable reference for every domain-specific term used in the
LovelaceSharp codebase. It extends `.github/prompts/legacy-knowledge-map.md`, which is the
authoritative translation table, by adding plain-language definitions and BCD-specific
vocabulary that agents and reviewers need when reading legacy C++ or writing new C# code.

Use this glossary to:
- Decode a Portuguese identifier encountered in `Legacy/` source files.
- Look up the canonical English counterpart to use in C# code.
- Understand what a BCD sentinel or nibble term means in practice.

For the full translation table (class names, method names, representation contract) see:
`.github/prompts/legacy-knowledge-map.md`

---

## 1. Class-Level Names

| Portuguese (C++) | English (C#) | Plain-language definition |
|---|---|---|
| `Lovelace` (digit storage) | `Representation` / `DigitStore` | ⚠️ The BCD backing store. Packs two decimal digits per byte. Only project that may touch `byte[]` directly. |
| `Lovelace` (arithmetic) | `Natural` | ⚠️ Arbitrary-precision natural number (≥ 0). Reads/writes digits exclusively through `GetDigit`/`SetDigit`. |
| `InteiroLovelace` | `Integer` | ⚠️ Signed arbitrary-precision integer. Wraps `Natural` and adds a sign flag (`bool IsNegative`). |
| `RealLovelace` | `Real` | ⚠️ Arbitrary-precision real number. Wraps `Integer` and adds a decimal exponent (`long Exponent`). |
| `VetorLovelace` | *(not yet migrated)* | ⚠️ Vector of `Real` elements. Exists in Legacy; not yet migrated to C# (OBS-026). |
| `VetorMultidimensionalLovelace` | *(not yet migrated)* | ⚠️ Multi-dimensional array of `Real` elements. Exists in Legacy; not yet migrated to C# (OBS-026). |

---

## 2. Method-Level Names

### 2.1 Constructors and Assignment

| Portuguese (C++) | English (C#) | Definition |
|---|---|---|
| `atribuir(unsigned long long int)` | Constructor overload / `Assign(ulong)` | ⚠️ Initialises the number from an unsigned 64-bit integer. |
| `atribuir(const int&)` | Constructor overload / `Assign(int)` | ⚠️ Initialises from a signed 32-bit integer. |
| `atribuir(const Lovelace&)` | Copy constructor / `Assign(Natural)` | ⚠️ Deep-copies another instance. |
| `atribuir(string)` | `Parse(string)` / `TryParse` | ⚠️ Parses a decimal string into the number. Implements `IParsable<T>`. |

### 2.2 Arithmetic Operations

| Portuguese (C++) | English (C#) | Definition |
|---|---|---|
| `somar` | `Add` / `operator+` | ✅ Addition. Implements `IAdditionOperators<T,T,T>`. |
| `subtrair` | `Subtract` / `operator-` | ✅ Subtraction. Implements `ISubtractionOperators<T,T,T>`. |
| `multiplicar` | `Multiply` / `operator*` | ✅ Multiplication. Implements `IMultiplyOperators<T,T,T>`. |
| `multiplicar_burro` | *(private)* | ✅ Naïve repeated-addition multiplication in Legacy (Lovelace.cpp:476). Removed in C# migration; replaced by grade-school algorithm with optional `Parallel.For` (OBS-030). |
| `dividir` | `DivRem` / `operator/` | ✅ Division with remainder. Implements `IDivisionOperators<T,T,T>`. |
| `dividir_burro` | *(private)* | ✅ Naïve long-division fallback in Legacy (Lovelace.cpp:638). Removed in C# migration (OBS-030). |
| `exponenciar` | `Pow` | ⚠️ Exponentiation (integer exponent). |
| `fatorial` | `Factorial` | ⚠️ Factorial (natural numbers only). |
| `incrementar` | `Increment` / `operator++` | ⚠️ Adds one in-place. Implements `IIncrementOperators<T>`. |
| `decrementar` | `Decrement` / `operator--` | ⚠️ Subtracts one in-place. Implements `IDecrementOperators<T>`. |
| `inverterSinal` | `Negate` / unary `operator-` | ⚠️ Flips sign. Implements `IUnaryNegationOperators<T,T>`. |
| `inverter` | `Invert` | ✅ C++ body was empty stub (RealLovelace.cpp:164). C# `Real.Invert()` fully implements the multiplicative inverse as `Real.One / this` (OBS-033). |

### 2.3 Comparison and Predicates

| Portuguese (C++) | English (C#) | Definition |
|---|---|---|
| `eIgualA` | `Equals` / `operator==` | ⚠️ Structural equality. Implements `IEquatable<T>`. |
| `eDiferenteDe` | `!Equals` / `operator!=` | ⚠️ Inequality. |
| `eMaiorQue` | `GreaterThan` / `operator>` | ⚠️ Greater-than comparison. |
| `eMenorQue` | `LessThan` / `operator<` | ⚠️ Less-than comparison. |
| `eMaiorOuIgualA` | `GreaterThanOrEqual` / `operator>=` | ⚠️ Greater-than-or-equal comparison. |
| `eMenorOuIgualA` | `LessThanOrEqual` / `operator<=` | ⚠️ Less-than-or-equal comparison. |
| `eZero` | `IsZero` (static, `INumber<T>`) | ⚠️ Returns true when the value is zero. |
| `naoEZero` | `!IsZero` | ⚠️ Returns true when the value is non-zero. |
| `ePar` | `IsEvenInteger` (static, `INumber<T>`) | ⚠️ Returns true for even integers. |
| `eImpar` | `IsOddInteger` (static, `INumber<T>`) | ⚠️ Returns true for odd integers. |
| `ePositivo` | `IsPositive` (static, `INumber<T>`) | ⚠️ `Integer`/`Real` only — true when value > 0. |
| `eNegativo` | `IsNegative` (static, `INumber<T>`) | ⚠️ `Integer`/`Real` only — true when value < 0. |
| `getSinal` | `Sign` property / `IsNegative` | ⚠️ Sign indicator for `Integer`. |

### 2.4 I/O and Formatting

| Portuguese (C++) | English (C#) | Definition |
|---|---|---|
| `imprimir` | `ToString` | ⚠️ Converts to decimal string. Implements `IFormattable`, `ISpanFormattable`. |
| `ler` | `Parse` / `TryParse` | ⚠️ Parses a decimal string. Implements `IParsable<T>`, `ISpanParsable<T>`. |
| `imprimirInfo` | `Dump()` (debug helper) | ⚠️ Prints internal state (tamanho, quantidadeAlgarismos, algarismos pointer) in Legacy (Lovelace.cpp:317). Replaced by unit tests in C# (OBS-026). |

### 2.5 Digit Storage (Representation layer only)

| Portuguese (C++) | English (C#) | Definition |
|---|---|---|
| `getBitwise` | `GetBitwise(long pos, out byte high, out byte low)` | ✅ Splits a BCD byte into its two nibbles (high = even index, low = odd index). `internal` in C# — visible to Natural only (OBS-031). |
| `setBitwise` | `SetBitwise(long pos, byte high, byte low)` | ✅ Packs two nibbles into one BCD byte: `(byte)((high << 4) | (low & 0x0F))`. `internal` in C# — visible to Natural only (OBS-031). |
| `getDigito` | `GetDigit(long position)` | ✅ Returns a single decimal digit (0–9) at the given logical position. Public API. |
| `setDigito` | `SetDigit(long position, byte digit)` | ✅ Stores a single decimal digit (0–9) at the given logical position. Public API. |
| `getTamanho` | `ByteCount` property | ✅ Number of backing bytes allocated. |
| `getQuantidadeAlgarismos` | `DigitCount` property | ✅ Number of logical decimal digits stored. |
| `expandirAlgarismos` | `GrowDigits()` (internal) | ✅ Resizes the backing array upward; pushes `0x0C` sentinel. Confirmed in DigitStore.cs:296 (OBS-027). |
| `reduzirAlgarismos` | `ShrinkDigits()` (internal) | ✅ Shrinks the backing array; vacated low nibble set to `0x0F`. Confirmed in DigitStore.cs:438 (OBS-027). |

### 2.6 RealLovelace-specific

| Portuguese (C++) | English (C#) | Definition |
|---|---|---|
| `getExpoente` | `Exponent` property | ✅ Decimal exponent. Negative value = fractional part. |
| `setExpoente` | `Exponent` setter | ✅ Sets the decimal exponent. `Real.Exponent` has a public setter (RISK-002, OBS-011). |
| `getCasasDecimaisExibicao` | `DisplayDecimalPlaces` static property | ⚠️ Number of decimal places shown when formatting. |
| `setCasasDecimaisExibicao` | `DisplayDecimalPlaces` static setter | ⚠️ Sets the display decimal precision globally. |
| `toInteiroLovelace` | *(no direct equivalent)* | ⚠️ Legacy method (RealLovelace.cpp:23) for conversion to Integer. C# achieves this through `ToNatural()` + constructor composition rather than a dedicated method (OBS-026, VAL-007). |

---

## 3. BCD Vocabulary

| Term | Definition |
|---|---|
| **BCD** (Binary-Coded Decimal) | ✅ Encoding scheme where each decimal digit 0–9 is stored in a 4-bit nibble. LovelaceSharp packs **two** digits per byte (OBS-002). |
| **Nibble** | ✅ A 4-bit half-byte. The high nibble (bits 7–4) holds the even-indexed digit; the low nibble (bits 3–0) holds the odd-indexed digit (OBS-002). |
| **High nibble** | ✅ Bits 7–4 of a BCD byte — stores the digit at even logical position `2*byteIndex` (OBS-002). |
| **Low nibble** | ✅ Bits 3–0 of a BCD byte — stores the digit at odd logical position `2*byteIndex + 1` (OBS-002). |
| **Sentinel `0x0C` (12)** | ✅ Value pushed by `GrowDigits` to mark a newly allocated but unused digit slot. Confirmed in DigitStore.cs (OBS-027). |
| **Sentinel `0x0F` (15)** | ✅ Value written to the low nibble by `ShrinkDigits` when a digit slot is vacated after shrinking. Confirmed in DigitStore.cs (OBS-027). |
| **Digit position** | ✅ Logical 0-based index of a decimal digit. Even positions occupy the high nibble of byte `position / 2`; odd positions occupy the low nibble. Position 0 = LSD (OBS-002). |
| **Byte count** | ✅ Number of bytes in the backing `List<byte>`. Equals `ceil(DigitCount / 2)` (OBS-001). |
| **Digit count** | ⚠️ Number of logical decimal digits that the current number occupies, including leading zeros if any. |

---

## 4. Static Members

| Portuguese (C++) | English (C#) | Location | Definition |
|---|---|---|---|
| `Lovelace::algarismosExibicao` | `Natural.DisplayDigits` | `Lovelace.Natural` | ⚠️ Static: number of digits to show when formatting. |
| `Lovelace::Precisao` | `Natural.Precision` | `Lovelace.Natural` | ⚠️ Static: global precision cap for arithmetic operations. |
| `Lovelace::TabelaDeConversao` | *(not translated)* | — | ⚠️ C++ digit-to-char lookup table (Lovelace.cpp:4) replaced by inline `(char)('0' + digit)` arithmetic in C# (OBS-026). |
| `RealLovelace::casasDecimaisExibicao` | `Real.DisplayDecimalPlaces` | `Lovelace.Real` | ⚠️ Static: number of decimal places shown when formatting a real number. |

---

## 5. Dependency-Chain Vocabulary

| Term | Definition |
|---|---|
| **Representation layer** | ⚠️ `Lovelace.Representation` — the only project that may read or write the raw `byte[]`. Exposes `GetDigit`/`SetDigit`. |
| **Natural layer** | ⚠️ `Lovelace.Natural` — arbitrary-precision ℕ₀ arithmetic built on top of the representation layer. |
| **Integer layer** | ⚠️ `Lovelace.Integer` — signed ℤ arithmetic; adds a boolean sign flag to `Natural`. |
| **Real layer** | ⚠️ `Lovelace.Real` — fixed-point/floating-point ℝ arithmetic; adds a `long Exponent` to `Integer`. |
| **BCD boundary** | ⚠️ The contract that only `Lovelace.Representation` may access `byte[]` directly; all upper layers call `GetDigit`/`SetDigit`. |

---

*All ❓ Unverified entries were promoted to ⚠️ Tentative (or higher) during Cycle 2 (2026-03-12) after source-level validation (VAL-006, VAL-007). Remaining ⚠️ entries await formal hypothesis/validation cycles to reach ✅ Verified.*
