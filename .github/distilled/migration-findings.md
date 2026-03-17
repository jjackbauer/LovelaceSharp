# Migration Findings

> **Scope**: C++ → C# migration decisions and lessons learned
> **Confidence**: High
> **Last updated**: 2026-03-16
> **Source entries**: OBS-029, OBS-030, OBS-031, OBS-032, OBS-033, OBS-034, OBS-035, OBS-036, VAL-008, VAL-009

---

## Purpose

This document captures the decisions, trade-offs, and lessons learned during the migration
of the LovelaceSharp arbitrary-precision number library from its legacy C++ implementation
(`Legacy/`) to idiomatic .NET C#. It covers structural changes (class decomposition, naming),
semantic changes (BCD access patterns, operator conventions), and pitfalls encountered.
Content is sourced from OBS entries (observed differences between C++ and C# implementations),
VAL entries (validated assumptions), and HYP entries.

---

## Structural Decisions

### Class Decomposition — Lovelace → DigitStore + Natural

- ⚠️ The C++ `Lovelace` class conflated raw digit storage and arbitrary-precision arithmetic in a single class hierarchy. In C#, this was split into two distinct projects: `DigitStore` (storage, in `Lovelace.Representation`) and `Natural` (arithmetic, in `Lovelace.Natural`). The split enforces the encapsulation invariant at compile time (OBS-029).

- ⚠️ The migration maintained the inheritance-vs-composition choice at each level: `DigitStore` is used via composition in `Natural`, while `Integer` extends `Natural` (via `Nat` alias) and `Real` extends `Integer`. This mirrors the C++ class hierarchy (`Lovelace` → `InteiroLovelace` → `RealLovelace`), though the split of `Lovelace` itself into two C# projects is a deviation (OBS-029, OBS-033).

### Vector Classes — Not Migrated (Intentional)

- ⚠️ `VetorLovelace` and `VetorMuldimensionalLovelace` exist in the C++ Legacy codebase but have no C# counterparts. They are not referenced by any project in the C# solution and no future migration work is planned for them (OBS-034, VAL-008).

---

## Naming and API Conventions

### Portuguese → English Renaming

- ⚠️ All C++ identifiers are in Portuguese; all C# identifiers are in English. Key class-level renames: `Lovelace` → `DigitStore` (storage layer) + `Natural` (arithmetic layer); `InteiroLovelace` → `Integer`; `RealLovelace` → `Real`; `VetorLovelace` → (not migrated); `VetorMultidimensionalLovelace` → (not migrated) (OBS-029, OBS-034).

- ⚠️ Method-level renames: `somar` → `operator+`; `subtrair` → `operator-`; `multiplicar` → `operator*`; `dividir` → `DivRem` + `operator/` + `operator%`; `exponenciar` → `Pow`; `fatorial` → `Factorial`; `incrementar` → `operator++`; `decrementar` → `operator--`; `atribuir` → constructors + `Parse`; `eIgualA` → `operator==`; `eMaiorQue` → `operator>`; `ePar`/`eImpar` → `IsEvenInteger`/`IsOddInteger` (OBS-032, OBS-033).

### Removed C++ Idioms

- ⚠️ The following C++ methods and patterns were removed with no C# counterpart: `multiplicar_burro` and `dividir_burro` (naïve O(n²) algorithms); `errorMessage(string)` + `exit(1)` (process-fatal error handler); `imprimirInfo(int)` (diagnostic state printer); `inverteNumero(Lovelace &)` (internal reversal helper); `TabelaDeConversao` (char[10] digit-to-char lookup table, replaced by `'0' + digit`); `operator<<`/`operator>>` (C++ stream I/O, replaced by `ToString()`/`Parse()`); `operator=` (C++ assignment operator, not applicable in C#) (OBS-030).

### Visibility Change — getBitwise/setBitwise

- ✅ In C++, both `getBitwise`/`setBitwise` (raw BCD byte access) and `getDigito`/`setDigito` (digit-level access) were `public`. In C#, `GetBitwise`/`SetBitwise` are `internal` (accessible to Natural only via `InternalsVisibleTo`) while `GetDigit`/`SetDigit` remain `public`. This prevents Integer and Real from bypassing the digit-level API at compile time (OBS-031).

---

## Semantic Differences

### Sign Encoding Inversion — InteiroLovelace → Integer

- ✅ In C++, `sinal` was `true` for positive and `false` for negative. In C#, `_isNegative` has inverted semantics: `true` for negative. Reading legacy C++ code alongside C# code requires care, as the sign flag meaning is reversed (OBS-032).

- ✅ C++ exposed `getSinal()`/`setSinal()` as mutable public methods. C# replaces this with `IsNegative(value)` as a static read-only predicate. The sign field is set only at construction — no public setter (OBS-032).

### Error Handling — from exit(1) to Typed Exceptions

- ⚠️ C++ used `errorMessage(string)` followed by `exit(1)` for all error conditions, making error recovery impossible. C# uses typed .NET exceptions: `DivideByZeroException` (division by zero), `InvalidOperationException` (underflow, negation of Natural), `ArgumentOutOfRangeException` (negative constructor argument, Pi digits ≤ 0), `ArithmeticException` (sqrt of negative), `FormatException` (invalid parse input) (OBS-035, OBS-036).

### Period Metadata — C# Addition

- ✅ C++ `RealLovelace` tracked only `expoente` (decimal exponent). C# `Real` adds `PeriodStart`, `PeriodLength`, and computed `IsPeriodic`. This enables exact rational representation of periodic decimals (e.g., `1/3 = "0.(3)"`), a feature that did not exist in the C++ implementation (OBS-033, OBS-022).

### inverter() — Stub to Full Implementation

- ✅ C++ `RealLovelace::inverter()` was declared but had an empty body (an acknowledged stub in the legacy codebase). C# `Real.Invert()` fully implements the multiplicative inverse as `1 / this` using the existing Real division infrastructure (OBS-033).

### toInteiroLovelace — Pattern Change

- ⚠️ C++ `RealLovelace::toInteiroLovelace(zeros)` was a private BCD re-encoding helper used internally. In C#, this pattern is eliminated because `DigitStore` handles BCD encoding natively. The closest C# equivalent is `ToNatural()` (on Integer) + Integer constructor — the composition replaces the helper (OBS-033, OBS-032).

---

## Known Pitfalls

### Real.Pow — Two NotImplementedException Stubs

- ✅ `Real.Pow` throws `NotImplementedException` for two cases: non-integer exponents and negative exponents. Both stubs are effort stubs (not blocked by missing infrastructure). Negative exponents could be implemented immediately via `Invert() + Pow(abs(n))`; non-integer exponents require `Real.Log` which does not yet exist. The stubs' error messages do not cite the `Real.Log` dependency, which is a minor documentation gap (OBS-036, VAL-009).

### Natural.TryConvert* — Inconsistent Stub Behavior

- ✅ Natural's six `TryConvert*` methods throw `NotImplementedException`, while Integer and Real return `false`. This inconsistency means `T.CreateChecked<int>(42)` will behave differently for `T = Natural` vs `T = Integer` — a known risk (RISK-001, OBS-036).

### Real.Exponent — Public Setter Bypass Risk

- ✅ `Real.Exponent` has a public setter, allowing external code to change the decimal exponent independently of the magnitude digits. This can violate invariants such as trailing-zero normalization (RISK-002, OBS-033).

---

## Migration Completeness Summary

| C++ Class | C# Equivalent(s) | Status |
|---|---|---|
| `Lovelace` (storage layer) | `DigitStore` (`Lovelace.Representation`) | ✅ Complete |
| `Lovelace` (arithmetic layer) | `Natural` (`Lovelace.Natural`) | ✅ Complete |
| `InteiroLovelace` | `Integer` (`Lovelace.Integer`) | ✅ Complete |
| `RealLovelace` | `Real` (`Lovelace.Real`) | ⚠️ Mostly complete (Pow stubs, ler() not migrated) |
| `VetorLovelace` | — | ❓ Not migrated (intentionally out of scope) |
| `VetorMuldimensionalLovelace` | — | ❓ Not migrated (intentionally out of scope) |
