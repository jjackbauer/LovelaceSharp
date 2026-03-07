---
agent: agent
description: Re-analyze all production and test C# files to extract or update the codebase patterns reference document.
---

#file:.github/prompts/legacy-knowledge-map.md
#file:.github/prompts/skill-falsify-claims.prompt.md

# Skill: Codebase Patterns Regeneration

## Purpose

Re-analyze all `*.cs` files (production and test) in the workspace to extract or
update the patterns reference document at `.github/prompts/codebase-patterns.md`.
Run this skill whenever significant new code is added, patterns evolve, or the
reference document drifts from reality.

## Depends On

- `.github/prompts/codebase-patterns.md` — the existing reference (if it exists; first run may create it from scratch)
- `.github/prompts/skill-falsify-claims.prompt.md` — used to verify extracted patterns against source

## Procedure

### Step 1 — Catalog production patterns

Read all `*.cs` files in `Lovelace.Representation/`, `Lovelace.Natural/`,
`Lovelace.Integer/`, and `Lovelace.Real/`. For each file, extract:

1. **Class layout** — section banner format and ordering
2. **Naming** — identifier conventions, type aliases, `InternalsVisibleTo`
3. **Interface implementation** — interfaces declared, trivial stubs, non-trivial predicates
4. **Operator style** — inline vs delegate-to-method vs delegate-to-static; `new` shadows
5. **Constructor patterns** — delegation chains, decomposition loops, mandatory set
6. **Static property patterns** — `Interlocked` usage, INumber constants
7. **Error handling** — exception-type-to-scenario mapping, guard clause placement
8. **Threading** — `lock`, `*Unsafe()`, `ArrayPool`, `Parallel.For`, lock ordering

### Step 2 — Catalog test patterns

Read all `*.cs` files in `Lovelace.Representation.Tests/`, `Lovelace.Natural.Tests/`,
`Lovelace.Integer.Tests/`, and `Lovelace.Real.Tests/`. For each file, extract:

1. **File layout** — namespace style, using directives, XML summary, banners
2. **Naming convention** — `MethodName_GivenScenario_ExpectedResult` conformance
3. **Attribute usage** — `[Fact]` vs `[Theory]+[InlineData]` ratio and formatting
4. **Variable naming** — `left`/`right`/`result`, `a`/`b`, literal suffixes
5. **Assertion patterns** — which `Assert` methods are used and in what context
6. **Coverage categories** — zero, identity, carry, beyond-native, algebraic, exception, round-trip
7. **Static property save/restore** — `try`/`finally` pattern

### Step 3 — Diff against existing reference

If `.github/prompts/codebase-patterns.md` exists, compare each extracted pattern
against the documented rule. Flag:

- **New patterns** — present in code but not yet documented
- **Contradictions** — documented rule contradicts actual code
- **Inconsistencies** — code varies across files for the same concern (document as known inconsistency)

### Step 4 — Falsify Claims

Take every pattern claim from Step 3 and run the Falsify Claims skill on it.
Only patterns with **Supported** status survive into the final document.

### Step 5 — Write the updated reference

Produce the updated `.github/prompts/codebase-patterns.md` following the section
structure defined in the plan:

§1 Project Structure Patterns → §2 Dependency Chain & Composition Model →
§3 Class Layout Template → §4 Constructor Patterns → §5 Static Property Patterns →
§6 Interface Implementation Patterns → §7 Operator Patterns →
§8 Error Handling Patterns → §9 Thread Safety & Parallelism Patterns →
§10 Test File Structure → §11 Test Naming Convention → §12 Test Method Anatomy →
§13 Assertion Patterns → §14 Test Coverage Checklist → §15 Anti-Patterns (Do NOT)

## Output

Updated `.github/prompts/codebase-patterns.md` with a trailing comment noting the
regeneration date and summary of changes (if this was an update rather than creation).
