# LovelaceSharp

An arbitrary-precision number library for .NET 10, migrated from a C++ implementation ([`Legacy/`](Legacy/)) to idiomatic C# with full xUnit test coverage.

Named after [Ada Lovelace](https://en.wikipedia.org/wiki/Ada_Lovelace), the first computer programmer.

---

## Overview

LovelaceSharp provides arbitrary-precision numeric types with no fixed size limit:

| Type | Range | Description |
|---|---|---|
| `Natural` | ≥ 0 | Arbitrary-precision natural numbers |
| `Integer` | ℤ | Signed arbitrary-precision integers |
| `Real` | ℝ | Arbitrary-precision fixed/floating-point reals |

All types implement the appropriate [`System.Numerics`](https://learn.microsoft.com/en-us/dotnet/standard/numerics) generic math interfaces, making them compatible with generic numeric algorithms out of the box.

---

## Architecture

The library is split into four focused projects, each building on the one below it.

```
Lovelace.Representation  ←  Lovelace.Natural  ←  Lovelace.Integer  ←  Lovelace.Real
```

### Projects

| Project | Class | Responsibility |
|---|---|---|
| `Lovelace.Representation` | `DigitStore` | **Internal BCD digit store.** Packs two decimal digits per `byte` (Binary Coded Decimal). The only project that may read or write the raw `byte[]` backing store. |
| `Lovelace.Natural` | `Natural` | Arbitrary-precision natural numbers (≥ 0). Arithmetic, comparison, parsing, and formatting via `INumber<T>`. |
| `Lovelace.Integer` | `Integer` | Signed arbitrary-precision integers. Adds a sign bit on top of `Natural`. |
| `Lovelace.Real` | `Real` | Arbitrary-precision real numbers. Adds a decimal exponent on top of `Integer`. |

Each project has a corresponding `*.Tests` project using xUnit.

---

## BCD Storage

`DigitStore` uses **Binary Coded Decimal (BCD)** packing — two decimal digits per `byte`:

```
Byte layout:
  bits 7–4 (high nibble) → even-indexed digit  (position 2n)
  bits 3–0 (low nibble)  → odd-indexed digit   (position 2n+1)
```

Sentinel values:
- `0x0C` — slot available (appended by `GrowDigits`)
- `0x0F` — slot freed (written by `ShrinkDigits` to the vacated half-byte)

Only `Lovelace.Representation` ever reads or writes these bytes. All higher layers call `GetDigit(position)` and `SetDigit(position, digit)`.

---

## Interfaces Implemented

`Natural`, `Integer`, and `Real` implement the standard .NET generic math interfaces:

- `INumber<T>`
- `IComparable<T>`, `IEquatable<T>`
- `IParsable<T>`, `ISpanParsable<T>`
- `ISpanFormattable`
- `IAdditionOperators<T,T,T>`, `ISubtractionOperators<T,T,T>`
- `IMultiplyOperators<T,T,T>`, `IDivisionOperators<T,T,T>`, `IModulusOperators<T,T,T>`
- `IIncrementOperators<T>`, `IDecrementOperators<T>`
- `IComparisonOperators<T,T,bool>`
- `ISignedNumber<T>` (`Integer` and `Real` only)

---

## Requirements & Status

| Project | Requirements Doc | Status |
|---|---|---|
| `Lovelace.Representation` | [`.github/requirements/Lovelace.Representation.md`](.github/requirements/Lovelace.Representation.md) | ✅ Complete |
| `Lovelace.Natural` | [`.github/requirements/Lovelace.Natural.md`](.github/requirements/Lovelace.Natural.md) | ✅ Complete |
| `Lovelace.Integer` | [`.github/requirements/Lovelace.Integer.md`](.github/requirements/Lovelace.Integer.md) | ✅ Complete |
| `Lovelace.Real` | *(pending)* | ⬜ Not started |

---

## Project Structure

```
LovelaceSharp.slnx
├── Legacy/                              # Original C++ source (reference only)
│   ├── Lovelace.hpp / .cpp              # BCD store + natural arithmetic
│   ├── InteiroLovelace.hpp / .cpp       # Signed integers
│   ├── RealLovelace.hpp / .cpp          # Real numbers
│   ├── VetorLovelace.hpp / .cpp         # Arbitrary-precision vector (not yet migrated)
│   └── VetorMultidimensionalLovelace.*  # Multi-dimensional array (not yet migrated)
│
├── Lovelace.Representation/             # BCD digit store (DigitStore)
├── Lovelace.Representation.Tests/
│
├── Lovelace.Natural/                    # Natural numbers (Natural)
├── Lovelace.Natural.Tests/
│
├── Lovelace.Integer/                    # Signed integers (Integer)
├── Lovelace.Integer.Tests/
│
├── Lovelace.Real/                       # Real numbers (Real)
└── Lovelace.Real.Tests/
```

---

## Building

Requires [.NET 10 SDK](https://dotnet.microsoft.com/download/dotnet/10.0).

```bash
dotnet build
```

---

## Testing

```bash
# Run all tests
dotnet test

# Run tests for a specific project
dotnet test Lovelace.Representation.Tests/
dotnet test Lovelace.Natural.Tests/
```

Test naming convention: `MethodName_GivenScenario_ExpectedResult`

---

## Legacy Migration

The C# codebase is a class-by-class migration from the C++ `Legacy/` source. The original code was written in Portuguese; all C# identifiers use English following .NET naming conventions (`PascalCase` for public members).

Key reference documents in `.github/`:

| File | Purpose |
|---|---|
| [`.github/prompts/legacy-knowledge-map.md`](.github/prompts/legacy-knowledge-map.md) | Full Portuguese → English method name mapping and representation contract |
| [`.github/prompts/skill-impl-completeness.prompt.md`](.github/prompts/skill-impl-completeness.prompt.md) | Audit a C++ class against its C# counterpart |
| [`.github/prompts/skill-test-standards.prompt.md`](.github/prompts/skill-test-standards.prompt.md) | Generate an xUnit test plan for a method |
| [`.github/prompts/skill-falsify-claims.prompt.md`](.github/prompts/skill-falsify-claims.prompt.md) | Verify or refute claims against the legacy source |
| [`.github/prompts/workflow-requirements-gathering.prompt.md`](.github/prompts/workflow-requirements-gathering.prompt.md) | Produce a checklist and test plan for a whole class |
| [`.github/prompts/workflow-iterative-implementation.prompt.md`](.github/prompts/workflow-iterative-implementation.prompt.md) | Implement one checklist item end-to-end |

---

## License

See [LICENSE](LICENSE) if present, or contact the repository owner.
