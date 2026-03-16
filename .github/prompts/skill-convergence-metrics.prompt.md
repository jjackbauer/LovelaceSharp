````prompt
---
agent: agent
description: Metrics role — compute all convergence metrics from current journal and distilled state, evaluate stopping criteria, and update convergence-metrics.md.
---

#file:.github/prompts/journal-schema.md

# Skill: Convergence Metrics

## Purpose

Read the current state of all journal files and distilled documents. Compute the 14 convergence
metrics, evaluate the five stopping criteria, and write an updated snapshot to
`.github/state/convergence-metrics.md`.

## Input (supplied by caller)

```
JournalDir:   <Path to journal files; defaults to .github/journals/>
DistilledDir: <Path to distilled documents; defaults to .github/distilled/>
StateDir:     <Path to state files; defaults to .github/state/>
```

## Role: Metrics Reporter

You operate as a **Metrics Reporter**. Your job is to provide an honest, quantitative view of
exploration progress. Do not omit metrics that show low coverage — accurate metrics are more
valuable than optimistic ones.

## Metric Definitions

| Metric | How to compute |
|---|---|
| **Modules explored** | Count distinct C# projects that have at least one OBS entry with a Source in that project's directory |
| **Execution flows mapped** | Count OBS entries whose Fact explicitly describes a call chain, delegation path, or data flow |
| **Dependencies verified** | Count OBS or VAL entries that confirm an inter-project or external dependency |
| **Observations recorded (OBS)** | Count all entries in `observations.md` |
| **Hypotheses proposed (HYP)** | Count all entries in `hypotheses.md` |
| **Hypotheses supported** | Count HYP entries with `Status: Supported` |
| **Hypotheses falsified** | Count HYP entries with `Status: Falsified` |
| **Validations completed (VAL)** | Count all entries in `validations.md` |
| **Open questions — P0 (OQ)** | Count OQ entries in `open-questions.md` with `Priority: P0` and `Status` ≠ Resolved |
| **Risks identified (RISK)** | Count all entries in `risks.md` |
| **Distilled docs updated** | Count distilled documents with `Last updated` ≠ `—` |
| **Artifact confidence (avg)** | Average confidence of ART entries in `artifact-index.md` (High=3, Medium=2, Low=1); report as number and label |
| **Inferred vs. observed claims ratio** | Count ❓ claims across all distilled docs / total claim count; report as decimal |
| **Repeated uncertainty hotspots** | Count modules or concepts that appear in both `unresolved-areas.md` and `open-questions.md` |

## Stopping Criteria

| # | Criterion | Met when |
|---|---|---|
| 1 | Module coverage ≥ 100% | All C# projects in the solution have ≥ 1 OBS entry |
| 2 | Zero P0 open questions | No OQ entries with Priority: P0 and unresolved status |
| 3 | All distilled docs at ≥ Medium confidence | Every distilled document header shows Confidence: Medium or High |
| 4 | Inferred:observed ratio < 0.1 | ❓ claims < 10% of total distilled claims |
| 5 | All artifacts have ≥ 3 evidence links | Every ART entry in `artifact-index.md` has ≥ 3 Supporting evidence IDs |

## Procedure

### Step 1 — Read all journals and distilled documents

Read every file in `JournalDir` and `DistilledDir`. Count entries and claims as defined above.

### Step 2 — Compute trend values

Read the previous `.github/state/convergence-metrics.md` snapshot (if it exists).
For each metric, compare to the previous value:
- ↑ if the value increased
- ↓ if the value decreased
- → if unchanged
- — if no previous value exists

### Step 3 — Evaluate stopping criteria

For each of the five criteria, determine Met (✅) or Not met (❌) based on computed values.

### Step 4 — Write the updated metrics file

Overwrite `.github/state/convergence-metrics.md` with the full updated content, following the
structure in the existing file (Metrics Table + Stopping Criteria + Update Notes).

Set **Last updated** in the file header to today's date.

### Step 5 — Report summary

```
Metrics updated: <YYYY-MM-DD>
Stopping criteria met: <N> of 5
Blocking criteria:     <list of unmet criteria, or "none — exploration complete">
```

If all five stopping criteria are met, output:
> **Convergence reached. Exploration may stop. All distilled artifacts are at sufficient evidence depth.**
````
