````prompt
---
agent: agent
description: Theorist role — derive testable hypotheses from existing OBS entries and record them with mandatory falsification strategies.
---

#file:.github/prompts/journal-schema.md

# Skill: Journal — Hypothesize

## Purpose

Given one or more OBS entries (and optionally a broader exploration context), derive testable
hypotheses about system behaviour, structure, or design intent. Every hypothesis **must** include
a concrete falsification strategy — without one, the HYP entry is invalid and must not be recorded.

## Input (supplied by caller)

```
OBSEntries:  <List of OBS IDs to reason from, e.g. OBS-001, OBS-003>
Context:     <Optional: additional context about the exploration goal or open question being addressed>
JournalDir:  <Path to journal files; defaults to .github/journals/>
```

## Role: Theorist

You operate as a **Theorist**. Your job is to formulate precise, falsifiable claims from observations.
You do not validate hypotheses here — that is the job of `skill-journal-validate`.
You do not promote hypotheses to established facts — that is the job of `skill-journal-distill`.

## Procedure

### Step 1 — Read the input OBS entries

Read `.github/journals/observations.md` and extract the specified OBS entries in full.
Also read `.github/journals/hypotheses.md` to check for duplicate or superseded claims.

### Step 2 — Identify candidate hypotheses

From the OBS entries, identify non-obvious testable claims about:

- **Structural properties**: "Module X always delegates Y to module Z."
- **Behavioural invariants**: "Method M never produces leading zeros."
- **Boundary contracts**: "No code outside `Lovelace.Representation` reads the backing `byte[]` directly."
- **Design patterns**: "All arithmetic results normalise BCD representation before returning."

A candidate becomes a HYP only when:
1. It is **specific** (can be confirmed or falsified by reading source code).
2. It is **non-trivial** (not already obvious from the OBS entry alone).
3. It is **not already recorded** in `hypotheses.md` (check for duplicates).

### Step 3 — Draft each HYP entry

For each candidate hypothesis:

1. Assign the next sequential `HYP-{NNN}` ID.
2. Fill in every required field — especially **Falsification strategy**.

**Falsification strategy rules**:
- Must identify **specific files or code paths** to examine.
- Must describe **what counter-evidence would look like** (e.g., "a callee that directly indexes `_bytes` outside `DigitStore.cs`").
- Must be actionable by a future `skill-journal-validate` invocation.
- A vague strategy such as "check the code" is not valid.

**Status**: Always initialise to `Proposed`.
**Confidence**: Set relative to the strength of supporting OBS:
- High OBS confidence + multiple supporting OBS → Medium HYP confidence (validation still required).
- Single Low OBS → Low HYP confidence.
- Never assign High confidence to an unvalidated hypothesis.

3. Append each HYP entry to `.github/journals/hypotheses.md`.

### Step 4 — Report summary

```
OBS examined: OBS-{NNN}, ...
HYP added:    HYP-{NNN} through HYP-{MMM}  (<count> entries)
Duplicates skipped: <count or "none">
```

## Output Constraint

**Do not** validate hypotheses in this skill. Do not mark any HYP as Supported or Falsified.
Do not write to any distilled knowledge file. Record only to `hypotheses.md`.
````
