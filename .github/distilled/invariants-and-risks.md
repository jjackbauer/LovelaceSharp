# Invariants and Risks

> **Scope**: Architectural invariants and known risks across the LovelaceSharp system
> **Confidence**: Medium
> **Last updated**: 2026-03-16
> **Source entries**: OBS-001, OBS-003, OBS-004, OBS-009, OBS-019, OBS-035, OBS-036, VAL-001, VAL-002, VAL-003, VAL-004, VAL-005, RISK-001, RISK-002

---

## Purpose

This document catalogues the architectural invariants that must hold across the LovelaceSharp
system — rules that, if violated, would compromise correctness or design integrity — and the
known risks that threaten those invariants or the project as a whole. Invariants are sourced
primarily from supported HYP entries; risks are sourced from RISK journal entries.

---

## Architectural Invariants

- ✅ **Encapsulation invariant**: No code outside `Lovelace.Representation` accesses the raw `byte[]` backing store or calls `GetBitwise`/`SetBitwise`. All upper layers use `GetDigit`/`SetDigit` exclusively (VAL-001, OBS-001, OBS-003).
- ✅ **Inheritance constraint**: `Integer` is the sole non-sealed numeric class; `Real` is its only subclass (VAL-002, OBS-008, OBS-011).
- ✅ **Immutable operator results**: All arithmetic operators (`+`, `-`, `*`, `/`, `%`) across Natural, Integer, and Real return fresh instances — no mutation of input operands (VAL-003).
- ✅ **No negative zero**: Integer's sign normalization ensures zero is always stored as positive (`_isNegative = isNegative && !Nat.IsZero(magnitude)`) (OBS-009).
- ✅ **Natural underflow throws**: `Natural.operator-` throws `InvalidOperationException` on underflow rather than auto-widening to Integer (VAL-005).
- ✅ **Generic conversion stubs**: All 18 `TryConvert*` methods across the three numeric types are stubs (throw or return false) — no cross-type generic math conversion works (VAL-004, OBS-019).

---

## Known Risks

- ⚠️ **RISK-001**: Natural's `TryConvert*` methods throw `NotImplementedException` while Integer/Real return `false`. Code using generic math APIs (`T.CreateChecked<int>()`) will crash for Natural but gracefully fail for Integer/Real — inconsistent behavior (RISK-001, OBS-019).
- ⚠️ **RISK-002**: `Real.Exponent` has a public setter, allowing external mutation that could violate representation invariants (changing exponent without adjusting magnitude digits) (RISK-002, OBS-011, OBS-033).
- ⚠️ **RISK-003** (from OBS-036, VAL-009): `Real.Pow` throws `NotImplementedException` for non-integer and negative exponents. These are effort stubs — negative exponents could be implemented immediately via `Invert() + Pow(abs(n))`; non-integer exponents require `Real.Log`. Callers unaware of these stubs may encounter unexpected exceptions.

---

## Mitigation Status

- ⚠️ RISK-001 mitigation: Change Natural's `TryConvert*` stubs to return `false` (matching Integer/Real) — tracked as a known implementation gap. No workaround required unless code uses `T.CreateChecked<int>()` with `T = Natural` (OBS-019).
- ⚠️ RISK-002 mitigation: Restrict `Real.Exponent` setter to `internal` or `init` to prevent external mutation. Currently unmitigated (OBS-033).
- ⚠️ RISK-003 mitigation: Implement negative exponents via `Invert() + Pow(abs(n))` — no dependencies missing. Non-integer exponents require designing `Real.Log` first (VAL-009).
