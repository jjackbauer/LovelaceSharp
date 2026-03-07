---
agent: plan
description: Produce a full requirements checklist and xUnit test plan for one C++ class being migrated to C#.
---

#file:.github/prompts/legacy-knowledge-map.md
#file:.github/prompts/skill-use-digit-store.prompt.md
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

### Step 3 — Enforce mandatory constructors

Every numerical implementation class **must** expose the following constructors regardless of what the C++ source provides:

| Constructor | Signature | Purpose |
|---|---|---|
| String constructor | `ctor(string value)` | Parse a decimal string representation at runtime |
| ReadOnlySpan constructor | `ctor(ReadOnlySpan<char> value)` | Zero-allocation span-based parsing (preferred hot path) |

If either constructor is missing from the C# class, add it to the completeness checklist as a **mandatory unchecked item** with the tag `[mandatory — commodity parsing]`, and generate the corresponding test cases in the Test Plan (Step 2 rules apply).

### Step 4 — Enforce mandatory operator overloads

Every numerical implementation class **must** implement the following operator overloads via the corresponding `System.Numerics` generic math interfaces, regardless of what the C++ source provides:

| Operator | Interface | Tag |
|---|---|---|
| `operator+` (binary) | `IAdditionOperators<T,T,T>` | `[mandatory — arithmetic]` |
| `operator-` (binary) | `ISubtractionOperators<T,T,T>` | `[mandatory — arithmetic]` |
| `operator*` | `IMultiplyOperators<T,T,T>` | `[mandatory — arithmetic]` |
| `operator/` | `IDivisionOperators<T,T,T>` | `[mandatory — arithmetic]` |
| `operator%` | `IModulusOperators<T,T,T>` | `[mandatory — arithmetic]` |
| `operator+` (unary) | `IUnaryPlusOperators<T,T>` | `[mandatory — arithmetic]` |
| `operator-` (unary) | `IUnaryNegationOperators<T,T>` | `[mandatory — arithmetic]` |
| `operator++` | `IIncrementOperators<T>` | `[mandatory — arithmetic]` |
| `operator--` | `IDecrementOperators<T>` | `[mandatory — arithmetic]` |
| `operator==`, `operator!=` | `IEqualityOperators<T,T,bool>` | `[mandatory — comparison]` |
| `operator<`, `operator>`, `operator<=`, `operator>=` | `IComparisonOperators<T,T,bool>` | `[mandatory — comparison]` |

> **Note**: Omit `operator%` and unary `operator-` for `Natural` (unsigned type). Include all operators for `Integer` and `Real`.

For each missing operator, add a **mandatory unchecked item** to the completeness checklist with the corresponding tag, and generate test cases in the Test Plan (Step 2 rules apply).

### Step 5 — Assemble the output

Combine both artefacts into a single structured document.

### Step 6 — Save to the requirements folder

Write the assembled document to `.github/requirements/<CsProject>.md` (e.g. `.github/requirements/Lovelace.Integer.md`).  
Create the file if it does not exist; overwrite it if it does.  
**This step is mandatory — do not skip it.**

## Output Format

```markdown
# Requirements: `<CppClass>` → `<CsProject>`

---

## Functionality Worktree

### Class Diagram

<Mermaid diagram from Implementation Completeness skill>

### Completeness Checklist

- [ ] `ctor(string value)` [mandatory — commodity parsing]
- [ ] `ctor(ReadOnlySpan<char> value)` [mandatory — commodity parsing]
- [ ] `operator+` (binary) (`IAdditionOperators<T,T,T>`) [mandatory — arithmetic]
- [ ] `operator-` (binary) (`ISubtractionOperators<T,T,T>`) [mandatory — arithmetic]
- [ ] `operator*` (`IMultiplyOperators<T,T,T>`) [mandatory — arithmetic]
- [ ] `operator/` (`IDivisionOperators<T,T,T>`) [mandatory — arithmetic]
- [ ] `operator==`, `operator!=` (`IEqualityOperators<T,T,bool>`) [mandatory — comparison]
- [ ] `operator<`, `operator>`, `operator<=`, `operator>=` (`IComparisonOperators<T,T,bool>`) [mandatory — comparison]
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

After saving the file and delivering this document, tell the developer:
> "Requirements gathering complete. Output saved to `.github/requirements/<CsProject>.md`. Run `workflow-iterative-implementation` and supply one checklist item at a time to implement and test it end-to-end."
