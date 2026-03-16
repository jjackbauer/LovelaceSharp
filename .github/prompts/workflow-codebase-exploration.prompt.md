````prompt
---
agent: agent
description: Orchestrate one complete codebase exploration cycle in 7 steps using the journal-driven skill suite.
---

#file:.github/prompts/journal-schema.md
#file:.github/prompts/distilled-knowledge-schema.md
#file:.github/prompts/skill-journal-observe.prompt.md
#file:.github/prompts/skill-journal-hypothesize.prompt.md
#file:.github/prompts/skill-journal-validate.prompt.md
#file:.github/prompts/skill-journal-distill.prompt.md
#file:.github/prompts/skill-completeness-review.prompt.md
#file:.github/prompts/skill-convergence-metrics.prompt.md

# Workflow: Codebase Exploration

## Purpose

Orchestrate one complete journal-driven exploration cycle. Each cycle narrows the codebase's
unknown surface area by observing, hypothesising, validating, distilling, and measuring.

Run this workflow repeatedly until `skill-convergence-metrics` reports all five stopping criteria as met.

## Input (supplied by caller or rule)

```
Objective:     <Optional: narrow exploration target, e.g. "Lovelace.Representation public API">
               If omitted, the workflow selects the highest-priority TODO from journals/todos.md.
ExplorationMode: <"focused" | "broad" | "auto" (default)>
DistillIfNew:  <"yes" | "no" | "auto" (default: distill if ≥ 3 new OBS entries produced)>
TargetDocs:    <Optional: specific distilled docs to target, e.g. "module-map.md,system-overview.md">
               If omitted, skill-journal-distill selects targets automatically.
```

## Focused vs. Broad Exploration

Choose the mode based on context:

| Mode | When to use | What changes |
|---|---|---|
| **Focused** | A specific HYP needs validation; a P0 OQ has a known answer location; a known gap in one module | Use a narrow Objective; skip completeness review unless distillation occurs |
| **Broad** | Session start; after a completeness review reveals multiple None-status dimensions; after distillation of a major module | Use a wide Objective (e.g., "all projects"); run completeness review at Step 5 |
| **Auto** | Default | Use "focused" if ≥ 1 P0 TODO exists; otherwise use "broad" |

## Procedure

### Step 1 — READ current state

Read `.github/state/convergence-metrics.md` to understand current coverage.
Read `.github/journals/todos.md` and `.github/journals/open-questions.md` for pending tasks.
Read `.github/journals/hypotheses.md` to identify HYP entries still `Under review`.

If stopping criteria are all met, report convergence and stop.

### Step 2 — SELECT one narrow objective

Choose the exploration target for this cycle:

1. **If `Objective` is provided**: use it directly.
2. **Else if a P0 TODO exists**: select the highest-priority open TODO as the objective.
3. **Else if an `Under review` HYP exists**: focus on validating the oldest unresolved HYP.
4. **Else**: select the least-explored project or dimension from the convergence metrics.

Record the chosen objective explicitly before proceeding.

### Step 3 — EXPLORE (Observe)

Invoke `skill-journal-observe` with the chosen objective.

Wait for the observation summary:
- `OBS added: OBS-{NNN} through OBS-{MMM}`

If zero OBS entries were produced (e.g., objective was a duplicate), select a different objective
and repeat Step 3. Do not proceed with zero new observations.

Optionally invoke `skill-journal-hypothesize` if the observations strongly motivate new hypotheses.

### Step 4 — VALIDATE (Falsify)

If Step 3 produced at least one new HYP, or if `Under review` HYP entries reference the same
module just explored:

1. Collect the relevant HYP IDs.
2. Invoke `skill-journal-validate` with those IDs.
3. Wait for the validation summary (Supported / Falsified / Unresolved counts).

If no HYP entries are relevant to the current objective, skip this step and note it in the summary.

### Step 5 — ASSESS risks and open questions

Review any RISK or OQ entries that relate to the explored objective:

- If the exploration answered an open question, update or close the OQ entry in `open-questions.md`.
- If the exploration revealed a new risk, append a RISK entry to `risks.md`.
- If exploration mode is "broad", invoke `skill-completeness-review` with `Scope` set to the explored module.

### Step 6 — DISTILL (if warranted)

Distill when any of the following conditions hold:
- `DistillIfNew: "yes"` was specified.
- `DistillIfNew: "auto"` and ≥ 3 new OBS entries were produced in Step 3.
- Step 4 produced at least one Supported VAL entry.
- A P0 open question was resolved in Step 5.

Invoke `skill-journal-distill` with:
- `SourceEntries`: all new OBS, HYP (Supported), and VAL IDs from this cycle.
- `TargetDoc`: the value of `TargetDocs` if provided, else `"auto"`.

Wait for the distillation summary before proceeding.

### Step 7 — METRICS

Always invoke `skill-convergence-metrics` at the end of every cycle, regardless of whether
distillation occurred.

After receiving the metrics summary, report:

```
Cycle summary
─────────────
Objective:          <chosen objective>
Exploration mode:   <focused | broad>
OBS added:          <count>
HYP added:          <count>  |  Validated: <count Supported>, <count Falsified>, <count Unresolved>
Distilled:          <target docs updated, or "skipped">
Stopping criteria:  <N of 5 met>
Next recommended:   <suggested objective for the next cycle>
```

If all five stopping criteria are now met, output:
> **Convergence reached after this cycle. No further exploration is required.**
````
