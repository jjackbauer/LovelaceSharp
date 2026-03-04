# Legacy Knowledge Map

> **Usage**: Include this file in any prompt via `#file:.github/prompts/legacy-knowledge-map.md`.  
> This file is a reference-only document — it is not a runnable prompt.

---

## 1. Class → C# Project Mapping

| C++ Class | C# Project | Role |
|---|---|---|
| `Lovelace` (digit storage — `getBitwise`/`setBitwise`, `getDigito`/`setDigito`) | `Lovelace.Representation` | Internal BCD digit store. Only project that accesses `byte[]` directly. |
| `Lovelace` (arithmetic — `somar`, `subtrair`, `multiplicar`, `dividir`, etc.) | `Lovelace.Natural` | Arbitrary-precision natural numbers (≥ 0). |
| `InteiroLovelace` | `Lovelace.Integer` | Signed arbitrary-precision integers. Adds `bool sinal` (sign). |
| `RealLovelace` | `Lovelace.Real` | Arbitrary-precision reals. Adds `long long int expoente` (decimal exponent). |
| `VetorLovelace` | *(not yet migrated)* | Arbitrary-precision vector of `RealLovelace` elements. |
| `VetorMultidimensionalLovelace` | *(not yet migrated)* | Multi-dimensional array of `RealLovelace` elements. |

---

## 2. Method Name Dictionary (Portuguese → English)

### Constructors / Assignment

| C++ (Portuguese) | C# (English) | Notes |
|---|---|---|
| `Lovelace()` | `Natural()` / constructor | Default constructor |
| `atribuir(unsigned long long int)` | `Assign(ulong)` or constructor overload | |
| `atribuir(const int&)` | `Assign(int)` or constructor overload | |
| `atribuir(const Lovelace&)` | `Assign(Natural)` or copy constructor | |
| `atribuir(string)` | `Parse(string)` / `TryParse` | Implements `IParsable<T>` |

### Arithmetic

| C++ (Portuguese) | C# (English) | Interface |
|---|---|---|
| `somar(B)` | `Add(B)` | `IAdditionOperators<T,T,T>`, `operator+` |
| `subtrair(B)` | `Subtract(B)` | `ISubtractionOperators<T,T,T>`, `operator-` |
| `multiplicar(B)` | `Multiply(B)` | `IMultiplyOperators<T,T,T>`, `operator*` |
| `multiplicar_burro(B)` | *(internal/private)* | Naïve repeated-addition; not exposed |
| `dividir(B, resultado, resto)` | `DivRem(B, out remainder)` | `IDivisionOperators<T,T,T>` |
| `dividir_burro(B, quocienteOuResto)` | *(internal/private)* | Naïve; not exposed |
| `exponenciar(X)` | `Pow(exponent)` | |
| `fatorial()` | `Factorial()` | |
| `incrementar()` | `Increment()` | `IIncrementOperators<T>`, `operator++` |
| `decrementar()` | `Decrement()` | `IDecrementOperators<T>`, `operator--` |
| `inverterSinal()` | `Negate()` | `IUnaryNegationOperators<T,T>`, `operator-` (unary) |
| `inverter()` | `Invert()` | Reciprocal (for `RealLovelace` only) |

### Comparison / Predicates

| C++ (Portuguese) | C# (English) | Interface |
|---|---|---|
| `eIgualA(B)` | `Equals(B)` | `IEquatable<T>`, `operator==` |
| `eDiferenteDe(B)` | `!Equals(B)` | `operator!=` |
| `eMaiorQue(B)` | `GreaterThan(B)` | `IComparisonOperators<T,T,bool>`, `operator>` |
| `eMenorQue(B)` | `LessThan(B)` | `operator<` |
| `eMaiorOuIgualA(B)` | `GreaterThanOrEqual(B)` | `operator>=` |
| `eMenorOuIgualA(B)` | `LessThanOrEqual(B)` | `operator<=` |
| `eZero()` | `IsZero` (static predicate in `INumber<T>`) | |
| `naoEZero()` | `!IsZero` | |
| `ePar()` | `IsEvenInteger` (static predicate in `INumber<T>`) | |
| `eImpar()` | `IsOddInteger` (static predicate in `INumber<T>`) | |
| `ePositivo()` | `IsPositive` (static predicate in `INumber<T>`) | `InteiroLovelace` only |
| `eNegativo()` | `IsNegative` (static predicate in `INumber<T>`) | `InteiroLovelace` only |
| `getSinal()` | `Sign` property or `IsNegative` | `InteiroLovelace` only |

### I/O / Formatting

| C++ (Portuguese) | C# (English) | Interface |
|---|---|---|
| `imprimir()` | `ToString()` | `IFormattable`, `ISpanFormattable` |
| `imprimir(char separador)` | `ToString(string format, IFormatProvider?)` | |
| `imprimirInfo(int opcao)` | `Dump()` or debug helper | Not part of public API |
| `ler()` | `Parse` / `TryParse` | `IParsable<T>`, `ISpanParsable<T>` |
| `operator<<` | `ISpanFormattable.TryFormat` | |
| `operator>>` | `IParsable<T>.Parse` | |

### Digit Storage (Representation layer only)

| C++ | C# | Notes |
|---|---|---|
| `getBitwise(pos, A, B)` | `GetBitwise(long pos, out byte high, out byte low)` | Splits a BCD byte into its two nibbles |
| `setBitwise(pos, A, B)` | `SetBitwise(long pos, byte high, byte low)` | Packs two nibbles into one BCD byte |
| `getDigito(pos)` | `GetDigit(long position)` | Returns a single decimal digit (0–9) |
| `setDigito(pos, char)` | `SetDigit(long position, byte digit)` | Stores a single decimal digit (0–9) |
| `getTamanho()` | `ByteCount` property | Number of backing bytes allocated |
| `setTamanho(n)` | *(internal resize)* | |
| `getQuantidadeAlgarismos()` | `DigitCount` property | Number of logical decimal digits |
| `setQuantidadeAlgarismos(n)` | *(internal)* | |
| `expandirAlgarismos()` | *(internal grow)* | |
| `reduzirAlgarismos()` | *(internal shrink)* | |

### RealLovelace-specific

| C++ (Portuguese) | C# (English) | Notes |
|---|---|---|
| `getExpoente()` | `Exponent` property | Decimal exponent (negative = fractional) |
| `setExpoente(X)` | `Exponent` setter | |
| `getCasasDecimaisExibicao()` | `DisplayDecimalPlaces` static property | |
| `setCasasDecimaisExibicao(n)` | `DisplayDecimalPlaces` static setter | |
| `toInteiroLovelace(zeros)` | `ToInteger(long zeros)` | Internal conversion |

---

## 3. Representation Contract

### BCD Packing (C++ → C#)

In C++, `setBitwise(pos, A, B)` packs two decimal digits into a single `char`:
- High nibble (bits 7–4): digit at **even** index `2*pos`
- Low nibble (bits 3–0): digit at **odd** index `2*pos + 1`

```cpp
// C++ reference implementation (Lovelace.cpp)
void Lovelace::setBitwise(long long int Posicao, char A, char B) {
    algarismos[Posicao] = (A << 4) | (B & 0x0F);
}
void Lovelace::getBitwise(long long int Posicao, char &A, char &B) const {
    A = (algarismos[Posicao] >> 4) & 0x0F;
    B =  algarismos[Posicao]       & 0x0F;
}
char Lovelace::getDigito(long long int Posicao) const {
    char A, B;
    getBitwise(Posicao / 2, A, B);
    return (Posicao % 2 == 0) ? A : B;
}
```

In C# (`Lovelace.Representation`):
- Backing store: `byte[]`
- `SetDigit(long position, byte digit)` and `GetDigit(long position)` are the **only** public surface
- `SetBitwise`/`GetBitwise` may exist as internal helpers but are not part of the contract visible to upper layers

### Sentinel / Padding Values
- `expandirAlgarismos()` pushes `0x0C` (12) — used as a "slot available" marker
- `reduzirAlgarismos()` sets the low nibble to `0x0F` (15) — represents "no digit" in the vacated half-byte
- C# equivalent: use the same sentinel values or document the chosen alternative clearly

---

## 4. Dependency Chain

```
Lovelace.Representation   (byte[] BCD store — GetDigit/SetDigit)
         ↑
Lovelace.Natural          (arbitrary-precision ℕ₀ arithmetic)
         ↑
Lovelace.Integer          (signed ℤ — adds bool IsNegative / Sign)
         ↑
Lovelace.Real             (fixed-point ℝ — adds long Exponent)
```

---

## 5. C# Interface Targets

| C# Project | Primary `.Numerics` Interfaces |
|---|---|
| `Lovelace.Natural` | `INumber<Natural>`, `IComparable<Natural>`, `IEquatable<Natural>`, `IParsable<Natural>`, `ISpanParsable<Natural>`, `ISpanFormattable`, `IIncrementOperators<Natural>`, `IDecrementOperators<Natural>` |
| `Lovelace.Integer` | All of Natural's interfaces + `ISignedNumber<Integer>`, `IUnaryNegationOperators<Integer,Integer>` |
| `Lovelace.Real` | All of Integer's interfaces + `IFloatingPoint<Real>` (or `INumber<Real>` if full IEEE compliance is not targeted) |

---

## 6. Static Members

| C++ | C# | Location |
|---|---|---|
| `Lovelace::algarismosExibicao` | `Natural.DisplayDigits` static property | `Lovelace.Natural` |
| `Lovelace::Precisao` | `Natural.Precision` static property | `Lovelace.Natural` |
| `Lovelace::TabelaDeConversao` | Not needed — use `(char)('0' + digit)` | — |
| `RealLovelace::casasDecimaisExibicao` | `Real.DisplayDecimalPlaces` static property | `Lovelace.Real` |
