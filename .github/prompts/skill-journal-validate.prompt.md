````prompt
---
agent: agent
description: Skeptic/Falsifier role — attempt to falsify HYP entries or artifact claims, record VAL entries, and update HYP status.
---

#file:.github/prompts/journal-schema.md
#file:.github/prompts/skill-falsify-claims.prompt.md

# Skill: Journal — Validate

## Purpose

Given a list of HYP entries (or artifact claims), attempt to falsify each one using at least two
tactics from the seven-tactic falsification library. Record the result as a VAL entry and update
the corresponding HYP status. Accept claims only after a genuine counter-evidence search.

## Input (supplied by caller)

```
Targets:     <Comma-separated list of HYP IDs to validate, e.g. HYP-001, HYP-003>
JournalDir:  <Path to journal files; defaults to .github/journals/>
```

## Role: Skeptic / Falsifier

You operate as a **Skeptic**. Your job is to try to break each hypothesis — not confirm it.
A hypothesis is Supported only when a genuine falsification attempt fails to find counter-evidence.
**Forbidden**: accepting any claim without first conducting a counter-evidence search.

## Falsification Tactic Library

For each target, apply **at least two** of the following seven tactics:

| # | Tactic | What to look for |
|---|---|---|
| 1 | **Alternate entrypoints** | Other methods or constructors that could produce the same outcome through a different path |
| 2 | **Bypass paths** | Code paths that skip the logic the hypothesis depends on (conditionals, early returns, overloads) |
| 3 | **Special-case conditionals** | Edge-case branches (null checks, zero guards, overflow handling) that violate the invariant |
| 4 | **Hidden write paths** | Indirect mutations (via delegates, events, ref parameters, or internal helpers) not visible at the call site |
| 5 | **Duplicate implementations** | Alternative implementations of the same operation in different classes that may not uphold the claim |
| 6 | **Contradicting tests** | Existing test cases that pass inputs the hypothesis would predict should fail, or vice versa |
| 7 | **Infrastructure divergence** | Build-time, reflection, or serialization paths that bypass the in-process call graph |

## Procedure

### Step 1 — Read the target HYP entries

Read `.github/journals/hypotheses.md` and extract each specified HYP in full.
Update their status from `Proposed` → `Under review` in `hypotheses.md`.

### Step 2 — Delegate to Falsify Claims skill

Format all HYP claims as a numbered list and invoke `skill-falsify-claims`:

```
1. <HYP-001 Claim text>
2. <HYP-002 Claim text>
...
```

Wait for the Falsify Claims result table (zero Falsified rows before proceeding).

### Step 3 — Apply additional tactics

For each HYP, apply at least two tactics from the library above that were **not** already covered
by the Falsify Claims invocation. Search the relevant source files for counter-evidence.
Record which tactics were applied and what was found.

**Minimum requirement**: ≥ 2 distinct tactics per HYP entry. A validation that applies only one
tactic is incomplete and must not produce a final VAL entry.

### Step 4 — Classify each result

- **Supported**: The falsification attempt failed — no counter-evidence found and the claim holds
  across all examined code paths. Assign High confidence only if ≥ 3 tactics were tried and all failed.
- **Falsified**: A concrete counterexample was found. Record the file:line of the counter-evidence.
- **Unresolved**: Evidence is incomplete (e.g., generated code, external library, or insufficient
  test coverage). Record what was checked and what was missing.

### Step 5 — Record VAL entries

For each HYP, append a VAL entry to `.github/journals/validations.md`:

```markdown
### VAL-{NNN}: Validation of {HYP-XXX}

- **Target HYP**: HYP-{XXX}
- **Method**: <Tactics applied: list the tactic names used>
- **Evidence examined**: <file:line references for all code inspected>
- **Result**: {Supported | Falsified | Unresolved}
- **Conclusion**: <One paragraph explaining the finding>
- **Related**: <HYP-XXX, OBS-YYY, ...>
```

### Step 6 — Update HYP status

In `.github/journals/hypotheses.md`, update each target HYP:

- **Supported** → set `Status: Supported`; upgrade confidence if warranted.
- **Falsified** → set `Status: Falsified`; append counter-evidence OBS reference to Related.
  Create a new OBS entry documenting the counter-evidence (append to `observations.md`).
- **Unresolved** → leave `Status: Under review`; add OQ entry for the gap.

### Step 7 — Report summary

```
HYP validated: HYP-{NNN}, ...
VAL added:     VAL-{NNN} through VAL-{MMM}
Results:       <N> Supported, <N> Falsified, <N> Unresolved
New OBS:       <list or "none">
New OQ:        <list or "none">
```

## Output Constraint

**Do not** write to distilled knowledge files in this skill.
**Do not** promote Supported findings to facts without going through `skill-journal-distill`.
````
