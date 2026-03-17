# Domain Concepts

> **Scope**: Core domain concepts — BCD packing, periodic decimals, and exponent model
> **Confidence**: High
> **Last updated**: 2026-03-16
> **Source entries**: OBS-001, OBS-002, OBS-004, OBS-011, OBS-013, VAL-001

---

## Purpose

This document describes the foundational domain concepts that underpin the LovelaceSharp
arbitrary-precision number library. It covers BCD (Binary-Coded Decimal) packing used by
the representation layer, periodic decimal detection and notation used by the real number
layer, and the exponent model that bridges integer magnitude and fractional positioning.
Content is sourced from OBS entries and validated HYP entries.

---

## BCD Packing

- ✅ Each byte in the DigitStore backing array stores two decimal digits: high nibble (bits 7–4) = even-indexed digit, low nibble (bits 3–0) = odd-indexed digit (OBS-002, VAL-001).
- ✅ Position 0 is the least-significant digit (LSB-first ordering) (OBS-002).
- ✅ Sentinel values: `0x0C` marks an allocated but not-yet-written byte slot; `0x0F` marks a freed low nibble after shrinking an even-count store (OBS-002).
- ✅ The backing store is a `List<byte>`, allowing dynamic growth via `GrowDigits()` which appends the `0x0C` sentinel (OBS-001).

---

## Periodic Decimals

- ✅ Real stores period metadata via `PeriodStart` (zero-based index where repeating block begins) and `PeriodLength` (length of repeating block; 0 = non-periodic) (OBS-011).
- ✅ Division performs remainder-tracked period detection: when a previously-seen remainder recurs, the exact period is recorded. Irrationals are approximated to `MaxComputationDecimalPlaces` (OBS-013).
- ⚠️ Periodic formatting uses parenthesis notation: `0.(3)` = 0.333…, `0.1(6)` = 0.1666… (OBS-011).
- ⚠️ `GetDecimalDigit(position)` computes fractional digits on demand; for periodic values beyond stored range, position wraps modulo `PeriodLength` (OBS-011).

---

## Exponent Model

- ✅ `Real.Exponent` is a `long` representing the decimal exponent: negative means fractional digits, zero means integer, positive means scaled integer (OBS-011).
- ✅ Example: value 3.14 is stored as magnitude=314, exponent=-2 (OBS-011, OBS-021).
- ⚠️ Addition aligns exponents before converting to comparable Integer form for the actual arithmetic (OBS-011).

---

