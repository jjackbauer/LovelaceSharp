````prompt
---
agent: plan
description: Architecture analysis rule — pre-configures the codebase exploration workflow to produce architecture and design documentation.
---

#file:.github/prompts/workflow-codebase-exploration.prompt.md

# Rule: Architecture Analysis

## Purpose

Pre-configured specialisation of the `workflow-codebase-exploration` workflow for architecture and
design documentation. Supplies all workflow parameters automatically; the caller only needs to
invoke this rule with an optional `Scope`.

## Input (supplied by caller)

```
Scope:  <Optional: module or concern to focus on, e.g. "Lovelace.Representation" or "all" (default)>
```

## Bindings

Follow `workflow-codebase-exploration` with the parameter bindings below.

**ExplorationMode**
> `"broad"` — architecture analysis requires wide coverage across all modules and concerns before
> conclusions can be drawn.

**DistillIfNew**
> `"auto"` — distil automatically when ≥ 3 new observations are produced or any claim is validated.

**TargetDocs**
> `"system-overview.md,module-map.md,domain-concepts.md,dependencies.md,runtime-flows.md,invariants-and-risks.md"`

**Objective** (derived from `Scope`)
> If `Scope` is `"all"` or omitted: `"Explore the full LovelaceSharp solution — all C# projects and Legacy C++ files — to document architecture, module responsibilities, and dependency boundaries."`
> If `Scope` names a specific project: `"Explore <Scope> — read all source files and document its public API, internal structure, and dependency contracts."`

## Focus Areas for Architecture Analysis

When applying `skill-journal-observe`, prioritise observations that inform:

1. **Module responsibilities** — what each C# project does and does not do.
2. **Public API surfaces** — method signatures, constructors, and interface implementations.
3. **Dependency contracts** — which projects call which, and what types cross boundaries.
4. **BCD encoding invariants** — byte layout, nibble assignment, sentinel values.
5. **Execution flows** — end-to-end paths from user input to arithmetic result.
6. **Architectural constraints** — rules enforced by code (e.g., "only Representation reads `byte[]`").

When applying `skill-journal-validate`, prioritise boundary claims:

- "No project outside `Lovelace.Representation` reads the backing `byte[]` directly."
- "The dependency chain is strictly linear: Representation ← Natural ← Integer ← Real."
- "All arithmetic operations go through the `Natural`/`Integer`/`Real` public API."

## Stopping Note

Run this rule repeatedly until `skill-convergence-metrics` reports:
- Module coverage ≥ 100%
- All distilled documents `system-overview.md`, `module-map.md`, `domain-concepts.md`,
  `dependencies.md`, `runtime-flows.md`, and `invariants-and-risks.md` have Confidence ≥ Medium.
````
