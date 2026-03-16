````prompt
---
agent: plan
description: Migration analysis rule — pre-configures the codebase exploration workflow for C++ → C# migration planning.
---

#file:.github/prompts/workflow-codebase-exploration.prompt.md
#file:.github/prompts/legacy-knowledge-map.md

# Rule: Migration Analysis

## Purpose

Pre-configured specialisation of the `workflow-codebase-exploration` workflow for C++ → C# migration
planning. Focuses the exploration on gaps, legacy behaviour differences, unresolved areas, and
migration decisions. Supplies all workflow parameters automatically; the caller only needs to
optionally supply a `CppClass` to focus on one class at a time.

## Input (supplied by caller)

```
CppClass:  <Optional: C++ class to focus on, e.g. "InteiroLovelace" or "all" (default)>
```

## Bindings

Follow `workflow-codebase-exploration` with the parameter bindings below.

**ExplorationMode**
> `"focused"` — migration analysis targets specific C++ ↔ C# class pairs to find gaps.

**DistillIfNew**
> `"yes"` — always distil after each migration-analysis cycle; migration decisions are high-value.

**TargetDocs**
> `"migration-findings.md,trusted-facts.md,unresolved-areas.md,module-map.md"`

**Objective** (derived from `CppClass`)
> If `CppClass` is `"all"` or omitted: `"Compare all Legacy/ C++ classes to their C# counterparts — identify missing methods, behavioural differences, and migration risks across the full solution."`
> If `CppClass` names a specific class: `"Compare Legacy/<CppClass> to its C# counterpart — read both the C++ header/implementation and the C# source, identify method gaps, stub implementations, and migration risks."`

## Focus Areas for Migration Analysis

When applying `skill-journal-observe`, prioritise:

1. **Missing C# methods** — C++ public methods with no C# equivalent (flag as RISK).
2. **Stub implementations** — C# methods that `throw new NotImplementedException()` or have empty bodies.
3. **Behavioural differences** — C++ methods whose logic differs from the C# counterpart (especially BCD handling, carry propagation, sign management).
4. **Naming divergence** — Portuguese method names in C++ that have not yet been mapped (consult `legacy-knowledge-map.md`).
5. **Interface compliance** — C# classes that do not implement the required `System.Numerics` interfaces.
6. **Test gap** — C++ behaviours that have no corresponding xUnit test in the C# test project.

When applying `skill-journal-validate`, prioritise:

- "The C# `Add` method produces the same result as C++ `somar` for all test cases."
- "The C# `Multiply` method handles BCD carry propagation identically to C++ `multiplicar`."
- "The C# `ToString` produces the same decimal string as C++ `paraString` for edge cases."

## Stopping Note

Run this rule class-by-class until `skill-convergence-metrics` reports:
- `migration-findings.md` has Confidence ≥ Medium.
- `unresolved-areas.md` has zero entries sourced from P0 OQ entries.
- `trusted-facts.md` has at least one ✅ entry per migrated C# project.
````
