---
agent: agent
description: Produce a full requirements checklist and xUnit test plan for one C++ class being migrated to C#.
---

#file:.github/prompts/legacy-knowledge-map.md
#file:.github/prompts/skill-impl-completeness.prompt.md
#file:.github/prompts/skill-test-standards.prompt.md

# Workflow: Requirements Gathering

## Purpose
For a given C++ class and its target C# project, produce:
1. A **Functionality Worktree** — the completeness checklist and Mermaid class diagram from the Implementation Completeness skill.
2. A **Test Plan** — a named xUnit test list for every unchecked checklist item, produced by the Test Standards skill.

Deliver both artefacts in a single response so the developer can start the Iterative Implementation workflow immediately.

## Input (supplied by caller)

```
CppClass:    <C++ class name, e.g. InteiroLovelace>
CsProject:   <Target C# project name, e.g. Lovelace.Integer>
```

## Procedure

### Step 1 — Run the Implementation Completeness skill

Invoke the Implementation Completeness skill with the provided `CppClass` and `CsProject`.

Wait for the skill to finish (zero Falsified rows) and collect:
- The mapping table
- The Mermaid class diagram
- The completeness checklist (unchecked items only, ordered by dependency)

### Step 2 — Run the Test Standards skill for each checklist item

For every unchecked item in the checklist (in dependency order):
1. Derive a plain-English functional description from the C++ `.cpp` implementation.
2. Invoke the Test Standards skill with the C# method signature and that description.
3. Wait for the skill to finish (zero Falsified rows) and collect the named test list.

### Step 3 — Assemble the output

Combine both artefacts into a single structured document.

## Output Format

```markdown
# Requirements: `<CppClass>` → `<CsProject>`

---

## Functionality Worktree

### Class Diagram

<Mermaid diagram from Implementation Completeness skill>

### Completeness Checklist

- [ ] `IsZero` (static predicate — `INumber<T>`) [prerequisite for many others]
- [ ] `Add(Integer)` → `operator+` (`IAdditionOperators<T,T,T>`)
...

---

## Test Plan

### `IsZero`

1. `IsZero_GivenDefaultInstance_ReturnsTrue`  
   *Assumption*: ...
2. `IsZero_GivenNonZeroValue_ReturnsFalse`  
   *Assumption*: ...

### `Add`

1. `Add_GivenTwoPositiveIntegers_ReturnsCorrectSum`  
   *Assumption*: ...
...

---

*All assumptions verified by Falsify Claims. Zero Falsified rows.*
```

After delivering this document, tell the developer:
> "Requirements gathering complete. Run `workflow-iterative-implementation` and supply one checklist item at a time to implement and test it end-to-end."
