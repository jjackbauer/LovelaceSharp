# System Overview

> **Scope**: High-level architecture, project structure, and dependency topology of the LovelaceSharp solution
> **Confidence**: High
> **Last updated**: 2026-03-16
> **Source entries**: OBS-001, OBS-005, OBS-008, OBS-011, OBS-016, OBS-018, OBS-019, VAL-001, VAL-002

---

## Purpose

This document provides a high-level architectural description of the LovelaceSharp solution,
including project responsibilities, the dependency chain, and cross-cutting design decisions.
Content is sourced from OBS entries (project structure observations) and VAL entries (boundary
claim confirmations).

---

## Architecture Overview

- ✅ LovelaceSharp is an arbitrary-precision number library migrated from C++ to .NET 10 C#. It comprises four core numeric libraries and a console REPL application (OBS-018, VAL-001).
- ✅ The dependency chain is: `Lovelace.Representation` ← `Lovelace.Natural` ← `Lovelace.Integer` ← `Lovelace.Real`. Each layer adds exactly one abstraction level (OBS-018).
- ✅ `Lovelace.Representation` is the only project that reads/writes the raw `byte[]` backing store. All upper layers use `GetDigit`/`SetDigit` exclusively (VAL-001, OBS-001, OBS-003).
- ✅ `Real` inherits from `Integer` (the sole subclass), which wraps `Natural`, which wraps `DigitStore` (VAL-002, OBS-008, OBS-011).
- ✅ All three numeric types implement .NET generic math interfaces (`INumber<T>`, `IParsable<T>`, `ISpanFormattable`, etc.), but generic type conversion methods are all stubs — Natural throws `NotImplementedException`, Integer and Real return `false` (OBS-019, VAL-004).

---

## Project Structure

| Project | Type | Role |
|---|---|---|
| `Lovelace.Representation` | Class library | Internal BCD digit store — packs two decimal digits per byte |
| `Lovelace.Natural` | Class library | Arbitrary-precision natural numbers (ℕ₀) — arithmetic on unsigned magnitudes |
| `Lovelace.Integer` | Class library | Signed arbitrary-precision integers (ℤ) — sign flag wrapping Natural |
| `Lovelace.Real` | Class library | Arbitrary-precision reals — decimal exponent + periodic notation on top of Integer |
| `Lovelace.Console` | Console app | Interactive REPL with tokenizer → parser → evaluator pipeline |

---

## Dependency Chain

```
Lovelace.Representation  ←  Lovelace.Natural  ←  Lovelace.Integer  ←  Lovelace.Real
                                                                              ↑
                                                                    Lovelace.Console
```

---

## Cross-Cutting Design Decisions

*No content yet — populated by `skill-journal-distill` from DEC entries.*

---

## Key Architectural Invariants

*No content yet — populated by `skill-journal-distill` from Supported HYP and VAL entries.*
