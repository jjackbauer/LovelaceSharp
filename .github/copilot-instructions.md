# LovelaceSharp — Workspace Context for GitHub Copilot

## Project Purpose

LovelaceSharp is an arbitrary-precision number library being migrated from a C++ implementation (`Legacy/`) to idiomatic .NET 10 C# with xUnit tests. The migration is class-by-class; each C# project corresponds to exactly one C++ class.

## C# Project Responsibilities

| C# Project | Maps from C++ | Role |
|---|---|---|
| `Lovelace.Representation` | `Lovelace` (digit storage layer) | **Internal bitwise digit store.** Packs two decimal digits per `byte` (BCD). Exposes only `GetDigit(long position)` and `SetDigit(long position, byte digit)`. No other project accesses the raw `byte[]` directly. |
| `Lovelace.Natural` | `Lovelace` (arithmetic layer) | Arbitrary-precision natural numbers (≥ 0). Depends on `Lovelace.Representation`. |
| `Lovelace.Integer` | `InteiroLovelace` | Signed arbitrary-precision integers. Adds a sign bit on top of `Lovelace.Natural`. |
| `Lovelace.Real` | `RealLovelace` | Arbitrary-precision fixed-point/floating-point real numbers. Adds a decimal exponent on top of `Lovelace.Integer`. |

## Dependency Chain

```
Lovelace.Representation  ←  Lovelace.Natural  ←  Lovelace.Integer  ←  Lovelace.Real
```

## Key Architectural Rules

1. **`Lovelace.Representation` is the only project that may read or write the backing `byte[]`.**  
   All upper-layer classes call `GetDigit`/`SetDigit` and never touch raw bytes.

2. **BCD packing**: two decimal digits per byte — high nibble = even-indexed digit, low nibble = odd-indexed digit (mirrors `getBitwise`/`setBitwise` in C++).

3. **Naming**: all C# identifiers use English, following .NET conventions (`PascalCase` for public members).  
   See `.github/prompts/legacy-knowledge-map.md` for the full Portuguese → English translation table.

4. **Testing**: xUnit only; test naming convention is `MethodName_GivenScenario_ExpectedResult`.

5. **Interfaces**: C# types should implement the appropriate `System.Numerics` generic math interfaces (`INumber<T>`, `IComparable<T>`, `IEquatable<T>`, `IParsable<T>`, `ISpanFormattable`, etc.) where they apply.

## When Working on Migration Tasks

Always load the legacy knowledge map and codebase patterns reference before writing code or tests:

```
#file:.github/prompts/legacy-knowledge-map.md
#file:.github/prompts/codebase-patterns.md
```

## Available Prompts and Workflows

| File | Purpose |
|---|---|
| `.github/prompts/legacy-knowledge-map.md` | Reference: class/method mapping and representation contract |
| `.github/prompts/skill-falsify-claims.prompt.md` | Skill: verify or refute specific claims against source |
| `.github/prompts/skill-test-standards.prompt.md` | Skill: generate xUnit test plan for a method |
| `.github/prompts/skill-impl-completeness.prompt.md` | Skill: audit C++ class against C# counterpart |
| `.github/prompts/workflow-requirements-gathering.prompt.md` | Workflow: generic — produce a requirements checklist and xUnit test plan for any C# project, driven by a pluggable analysis source and mandatory-item rules |
| `.github/prompts/rule-migration.prompt.md` | Rule: migration-specific pre-configuration of the requirements-gathering workflow for C++ → C# class migration |
| `.github/prompts/workflow-iterative-implementation.prompt.md` | Workflow: implement one checklist item end-to-end |
| `.github/prompts/codebase-patterns.md` | Reference: implementation and test-writing conventions distilled from the codebase |
| `.github/prompts/skill-codebase-patterns.prompt.md` | Skill: regenerate/update the codebase patterns reference by re-analyzing source files |
| `.github/prompts/skill-plan-format-gate.prompt.md` | Skill: validate plan document structure; auto-invoked by the requirements-gathering workflow and self-heals violations autonomously |
