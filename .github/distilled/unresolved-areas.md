# Unresolved Areas

> **Scope**: Gaps, weak evidence, open questions, and unresolved validation results across all LovelaceSharp exploration
> **Confidence**: Medium
> **Last updated**: 2026-03-16
> **Source entries**: OQ-001, OQ-002, OQ-003, OQ-004, RISK-001, RISK-002, RISK-003, TODO-001, TODO-003, TODO-004

---

## Purpose

This document records areas of the codebase where knowledge is incomplete, evidence is weak,
or formal validation has not yet been performed. It is the designated home for:

- **Gaps**: parts of the codebase not yet explored (no OBS entries exist for them).
- **Weak evidence**: claims that carry only ⚠️ Tentative or ❓ Unverified status because no
  Supported VAL entry backs them.
- **Open questions**: OQ entries that remain unresolved (no VAL entry has closed them).
- **Unresolved validations**: VAL entries with `Result = Unresolved`.

Content here is sourced from `open-questions.md` (OQ entries) and `validations.md`
(VAL entries with `Result = Unresolved`). When a gap is closed — by a Supported VAL entry
or a matching OBS entry — the corresponding section is removed or promoted to the
appropriate distilled document.

---

## Unexplored Modules / Unexplored Areas

- ~~❗ Legacy C++ files — no systematic mapping to C# equivalents has been performed (TODO-001, OQ-001).~~ **Resolved** — OBS-029 through OBS-034 provide method-by-method mapping for all four C++ class groups.
- ~~❗ Edge-case and error-handling paths have not been documented (TODO-003).~~ **Resolved** — OBS-035, OBS-036 document all exception boundaries for Natural and Real.
- ~~❗ Execution flows have not been traced end-to-end (TODO-002).~~ **Resolved** — OBS-021 through OBS-028 trace all three key flows.
- ⚠️ REPL test coverage completeness has not been fully verified (TODO-004, OQ-003) — whether LineEditor and ReplSession have dedicated test files is not confirmed.

---

## Weak Evidence Claims

- ❓ Real's Chudnovsky Pi implementation details (binary splitting merge formula correctness) have not been formally validated against a reference implementation (OBS-014).
- ⚠️ `VetorLovelace` and `VetorMuldimensionalLovelace` are confirmed as out-of-scope for the current migration (OBS-034, VAL-008), but no explicit decision or design document records this scope boundary from the project level.

---

## Open Questions

- ~~**OQ-001** (P1): Which C++ methods have not yet been migrated to C#?~~ **Resolved** — OBS-029 through OBS-034 provide the full mapping.
- **OQ-002** (P2): Is `Real.Exponent`'s public setter thread-safe? Could concurrent mutation corrupt state?
- **OQ-003** (P2): Are there test files for all REPL components (LineEditor, ReplSession)?

---

## Known Implementation Stubs

- ⚠️ `Natural.TryConvert*` — all 6 methods throw `NotImplementedException` (inconsistent with Integer/Real which return `false`) — RISK-001.
- ⚠️ `Real.Pow` — throws `NotImplementedException` for non-integer and negative exponents — RISK-003. Negative exponent path could be closed immediately; non-integer path requires `Real.Log`.

---

## Unresolved Validations

- None — all 9 VAL entries have `Result = Supported`.
