````prompt
---
agent: agent
description: Reviewer role — audit current distilled knowledge for coverage gaps across 12 dimensions and produce OQ and TODO entries.
---

#file:.github/prompts/journal-schema.md
#file:.github/prompts/skill-impl-completeness.prompt.md

# Skill: Completeness Review

## Purpose

Given the current state of distilled knowledge documents and journal files, audit coverage across
12 dimensions. For each gap found, produce a new OQ entry (if the gap requires investigation) or
a TODO entry (if the gap requires a targeted exploration). Produce a summary coverage table.

**Forbidden**: Declaring completeness in any dimension without explicitly checking it.

## Input (supplied by caller)

```
Scope:        <Which modules or concerns to review, e.g. "all" or "Lovelace.Representation only">
JournalDir:   <Path to journal files; defaults to .github/journals/>
DistilledDir: <Path to distilled documents; defaults to .github/distilled/>
```

## Role: Reviewer

You operate as a **Reviewer**. Your job is to find what is missing, not to confirm what is present.
Approach each dimension with the assumption that coverage is incomplete until you find evidence otherwise.

## Coverage Dimensions

Evaluate coverage for each of the 12 dimensions below. For each one, determine:
- **Status**: Covered (≥ 2 OBS or ≥ 1 VAL addressing this dimension) / Partial (1 OBS, no VAL) / None (no entries)
- **Gap**: A concrete description of what is missing

| # | Dimension | Key questions |
|---|---|---|
| 1 | **Modules** | Have all C# projects been explored? Any project with zero OBS entries? |
| 2 | **Domain entities** | Are all major data types observed (DigitStore, Natural, Integer, Real, parser nodes, REPL values)? |
| 3 | **Execution flows** | Are key flows traced end-to-end (parse→evaluate, arithmetic dispatch, BCD encode/decode)? |
| 4 | **Critical dependencies** | Are all inter-project data flows and external library usages observed and verified? |
| 5 | **Configuration surfaces** | Are precision limits, BCD constants, and numeric bounds documented? |
| 6 | **Test coverage** | Are test suites cross-checked against implementation? Any missing test class? |
| 7 | **Thread safety** | Is there any evidence of shared mutable state? If found, is it documented? |
| 8 | **Edge cases** | Are zero, negative, overflow, max-precision, and empty-input cases observed in tests and source? |
| 9 | **Legacy gaps** | Are all C++ methods in `Legacy/` mapped to C# equivalents? Stubs? Missing? |
| 10 | **Scheduled / background operations** | Any async, timer, or background-task code? Documented if present? |
| 11 | **Auth / trust boundaries** | Any user-input parsing, file I/O, or network code? If present, is input validation observed? |
| 12 | **Failure behavior / deployment assumptions** | Are exception paths, error returns, and deployment constraints documented? |

## Procedure

### Step 1 — Read journal state

Read all journal files (`observations.md`, `hypotheses.md`, `validations.md`, `open-questions.md`,
`todos.md`) and all distilled documents.
Build a mental index: which dimensions have coverage, which are sparse, which have none.

### Step 2 — Optionally invoke skill-impl-completeness

If `Scope` targets specific C++ ↔ C# class pairs, invoke `skill-impl-completeness` for each pair
to populate dimension 9 (Legacy gaps). Collect the mapping table.

### Step 3 — Evaluate each dimension

For each of the 12 dimensions:
1. Search journal and distilled content for evidence addressing that dimension.
2. Assign Status (Covered / Partial / None).
3. Identify the specific gap (what is unobserved, unvalidated, or undocumented).

### Step 4 — Produce coverage table

| # | Dimension | Status | Gap Description |
|---|---|---|---|
| 1 | Modules | Covered / Partial / None | <description or "—"> |
| ... | ... | ... | ... |

### Step 5 — Record OQ and TODO entries

For each gap:

- **If the gap requires investigation** (the answer is not known and must be determined by reading code):
  Append an OQ entry to `open-questions.md`.
  Priority: P0 = gap blocks distillation or migration planning; P1 = important; P2 = nice-to-have.

- **If the gap requires exploration** (we know where to look but haven't looked):
  Append a TODO entry to `todos.md`.
  Priority: P0 = required for convergence; P1 = should be done; P2 = optional.

### Step 6 — Report summary

```
Scope reviewed:    <scope>
Dimensions:        <N> Covered, <N> Partial, <N> None
OQ added:          OQ-{NNN} through OQ-{MMM}  (<count> entries)
TODO added:        TODO-{NNN} through TODO-{MMM}  (<count> entries)
Blocking gaps (P0): <list or "none">
```
````
