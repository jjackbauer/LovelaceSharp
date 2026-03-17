# Module Map

> **Scope**: Per-project responsibilities, public interfaces, and inter-module boundaries
> **Confidence**: High
> **Last updated**: 2026-03-16
> **Source entries**: OBS-001, OBS-002, OBS-003, OBS-004, OBS-005, OBS-006, OBS-007, OBS-008, OBS-009, OBS-010, OBS-011, OBS-012, OBS-013, OBS-014, OBS-015, OBS-016, OBS-017, OBS-018, OBS-019, OBS-020, OBS-029, OBS-031, OBS-034, VAL-001, VAL-002, VAL-003, VAL-004, VAL-005, VAL-008

---

## Purpose

This document maps each C# project in the LovelaceSharp solution to its responsibilities,
public interface surface, and position in the dependency chain. Content is sourced from OBS
entries (class surface observations), supported HYP entries, and VAL entries that confirm
boundary claims.

---

## Dependency Chain

```
Lovelace.Representation  ←  Lovelace.Natural  ←  Lovelace.Integer  ←  Lovelace.Real
```

---

## Projects

### Lovelace.Representation

- ✅ Contains a single public class `DigitStore` that packs two decimal digits per byte using BCD encoding (high nibble = even position, low nibble = odd position) (OBS-001, OBS-002).
- ✅ Public API surface: `GetDigit(long)`, `SetDigit(long, byte)`, `DigitCount`, `ByteCount`, `IsZero`, `ToString()`, `ToString(char)`, `Dump()` (OBS-001).
- ✅ Internal API (visible to Natural and Tests only): `GetBitwise`, `SetBitwise`, `GrowDigits`, `ShrinkDigits`, `ClearDigits`, `CopyDigitsFrom`, `SnapshotDigits`, `RentDigitSnapshot`, `ReturnDigitSnapshot`, `TrimLeadingZeros`, `Reset`, `Initialize` (OBS-003, OBS-006).
- ✅ Thread-safe via per-instance `Monitor` (`_syncRoot`) with canonical lock ordering (by `_id`) for `CopyDigitsFrom` (OBS-004).
- ✅ Uses sentinel bytes: `0x0C` (allocated, not written) and `0x0F` (freed low nibble) during grow/shrink (OBS-002).

### Lovelace.Natural

- ✅ Sealed class implementing `INumber<Natural>` and related .NET generic math interfaces (OBS-005, OBS-019).
- ✅ Single private field `DigitStore _store` — all digit access goes through DigitStore's public and internal APIs (OBS-006, VAL-001).
- ✅ Arithmetic: grade-school addition/subtraction/multiplication with conditional `Parallel.For` for large operands; long division via `DivRem`; binary exponentiation via `Pow`; parallelized `Factorial` (OBS-007).
- ✅ Uses `RentDigitSnapshot()`/`ReturnDigitSnapshot()` (ArrayPool) in multiplication for zero-GC hot paths (OBS-006).
- ✅ Subtraction throws `InvalidOperationException` on underflow — no auto-widening to Integer (VAL-005, OBS-007).
- ✅ Static `DisplayDigits` and `Precision` use `Interlocked` for thread-safe 64-bit access (OBS-005).

### Lovelace.Integer

- ✅ Non-sealed class (to allow Real inheritance) implementing `ISignedNumber<Integer>` and `INumber<Integer>` (OBS-008, VAL-002).
- ✅ Two private readonly fields: `Nat _magnitude` and `bool _isNegative`. Zero is always positive (sign normalization) (OBS-008, OBS-009).
- ✅ Delegates all digit-level arithmetic to Natural operators; manages sign rules (XOR for multiplication, same-sign addition, magnitude comparison for subtraction) (OBS-010).
- ✅ `ToNatural()` returns magnitude directly (not a copy), exposing internal state (OBS-010).

### Lovelace.Real

- ✅ Extends Integer (sole subclass) with `Exponent`, `PeriodStart`, `PeriodLength`, and computed `IsPeriodic` (OBS-011, VAL-002).
- ✅ Division performs remainder-tracked period detection for exact rational representation (OBS-013).
- ✅ `AsyncLocal<long?>` enables per-call-stack precision scoping via `WithLocalPrecision()` for thread-safe Sqrt and Pi computation (OBS-012).
- ✅ Pi uses Chudnovsky algorithm with binary splitting and parallel dispatch; Sqrt uses Newton-Raphson with progressive precision doubling (OBS-014, OBS-015).
- ✅ `Exponent` has a public setter — could be mutated externally (OBS-011, RISK-002).
- ✅ `MaxComputationDecimalPlaces` defaults to 1000; configurable globally and per-call-stack (OBS-012).

### Lovelace.Console

- ✅ Interactive REPL with pipeline: LineEditor → Tokenizer → Parser (recursive descent, 8 precedence levels) → Evaluator → Value output (OBS-016).
- ✅ `Value` type system with automatic widening: Natural → Integer → Real. Literal type inference: text containing `.` or `(` → Real; otherwise → Natural (OBS-017, OBS-025).
- ✅ Built-in functions (exactly 8): `abs`, `inv`, `divrem`, `is_even`, `is_odd`, `sign`, `sqrt`, `pi` (OBS-016).
- ✅ References all three numeric libraries (Natural, Integer, Real) but uses only public APIs (OBS-016, OBS-020).

---

## Interface Boundaries

- ✅ Only `Lovelace.Representation` accesses the raw `byte[]` backing store. All other projects use `GetDigit`/`SetDigit` (VAL-001).
- ✅ `InternalsVisibleTo` grants: Representation → {Representation.Tests, Natural}; Real → {Real.Tests} (OBS-003, OBS-020).
- ✅ All arithmetic operators return new instances — no mutation of input operands (VAL-003).
- ✅ Generic type conversion stubs: Natural throws `NotImplementedException`; Integer and Real return `false` (VAL-004).

---

## Unmigrated C++ Classes

- ⚠️ `VetorLovelace` (C++ vector of `RealLovelace` elements with 9 public methods: imprimir, getElemento, setElemento, getDimensionalidade, setDimensionalidade, somar, subtrair, produtoInterno, multiplicar) and `VetorMuldimensionalLovelace` (multidimensional vector wrapper, 8 methods) are intentionally out of scope for the current migration — no C# project exists for either class (OBS-034, VAL-008).
