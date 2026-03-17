# Trusted Facts

> **Scope**: Verified facts about LovelaceSharp with the highest evidential bar: High confidence, Supported validation only
> **Confidence**: High
> **Last updated**: 2026-03-16
> **Source entries**: OBS-001, OBS-003, OBS-005, OBS-006, OBS-007, OBS-008, OBS-009, OBS-011, OBS-019, VAL-001, VAL-002, VAL-003, VAL-004, VAL-005

---

## Purpose

This document is the **gold standard** distilled knowledge store. It accepts only claims
that satisfy **all** of the following criteria simultaneously:

1. The claim has at least one **Supported** VAL entry (Result = Supported) directly addressing it.
2. The claim carries **High** confidence in the journal.
3. No contradicting VAL or OBS entry exists.

As a consequence, only **✅ Verified** markers appear in this file.
**⚠️ Tentative and ❓ Unverified markers are strictly forbidden** — claims that cannot
meet all three criteria above must live in `unresolved-areas.md` instead.

---

## Admission Constraints

| Rule | Requirement |
|---|---|
| Marker | Exclusively ✅ — any ⚠️ or ❓ is a schema violation |
| Validation | Every claim must cite at least one VAL entry with Result = Supported |
| Confidence | Every claim must carry High confidence in the journal entry it traces from |
| Contradiction | Zero contradicting VAL or OBS entries for the claim |

When these constraints cannot be met, **do not add the claim here** — open an OQ entry
in `.github/journals/open-questions.md` and add the claim to `unresolved-areas.md` instead.

---

## Representation Layer Facts

- ✅ `DigitStore` is the sole type that directly accesses the raw `byte[]` backing store. No code in Natural, Integer, Real, or Console references `_bytes`, `GetBitwise`, or `SetBitwise` (VAL-001, OBS-001, OBS-003).
- ✅ BCD packing: each byte stores two decimal digits — high nibble (bits 7–4) = even-indexed digit, low nibble (bits 3–0) = odd-indexed digit. Position 0 = LSD (OBS-002, VAL-001).
- ✅ `InternalsVisibleTo` grants internal DigitStore access only to `Lovelace.Representation.Tests` and `Lovelace.Natural` (OBS-003, VAL-001).

---

## Natural Number Arithmetic Facts

- ✅ `Natural` is a sealed class with a single private `DigitStore _store` field implementing `INumber<Natural>` (OBS-005, VAL-001).
- ✅ `Natural.operator-` throws `InvalidOperationException` on underflow — no auto-widening to Integer at the library level (VAL-005, OBS-007).
- ✅ All Natural arithmetic operators return new instances — no mutation of input operands (VAL-003, OBS-007).

---

## Integer Facts

- ✅ `Integer` is non-sealed (public class, not sealed) specifically because `Real` inherits from it. `Real` is the sole subclass in the solution (VAL-002, OBS-008, OBS-011).
- ✅ Zero is always stored as positive: `_isNegative = isNegative && !Nat.IsZero(magnitude)` (OBS-009).
- ✅ Integer delegates all digit-level arithmetic to Natural operators and manages only sign rules (OBS-010).

---

## Real Number Facts

- ✅ `Real` extends `Integer` with `Exponent`, `PeriodStart`, `PeriodLength`, and computed `IsPeriodic` (OBS-011, VAL-002).

---

## Cross-Cutting Architectural Facts

- ✅ Dependency chain: Representation ← Natural ← Integer ← Real. Console references all three numeric libraries (OBS-018).
- ✅ All 18 generic `TryConvert*` methods across Natural, Integer, and Real are stubs — Natural throws `NotImplementedException`, Integer and Real return `false` (VAL-004, OBS-019).
- ✅ All arithmetic operators across Natural, Integer, and Real return fresh instances without mutating input operands (VAL-003).
