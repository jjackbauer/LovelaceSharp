````prompt
---
agent: plan
description: Produce a requirements checklist and xUnit test plan for a C# project, driven by a pluggable analysis source and mandatory-item rules.
---

#file:.github/prompts/codebase-patterns.md
#file:.github/prompts/skill-test-standards.prompt.md
#file:.github/prompts/skill-plan-format-gate.prompt.md

# Workflow: Requirements Gathering

## Purpose
For a given C# project, produce:
1. A **Functionality Worktree** — a completeness checklist and (where applicable) a Mermaid class diagram, derived from the provided `AnalysisSource`.
2. A **Test Plan** — a named xUnit test list for every unchecked checklist item, produced by the Test Standards skill.

Deliver both artefacts in a single structured document so the developer can start implementation immediately.

## Input (supplied by caller or collected interactively)

```
CsProject:      <Target C# project name, e.g. Lovelace.Integer>
AnalysisSource: <How to derive the checklist — e.g. "invoke an analysis skill with specific inputs",
                 "audit an existing C# class against its interfaces", or a free-form description>
MandatoryItems: <List of mandatory checklist entries with tags, or "none">
PlanType:       <Value to pass to the Format Gate, e.g. "requirements" or "generic">
OutputPath:     <Save location; defaults to .github/requirements/<CsProject>.md>
OutputTitle:    <H1 title of the output document>
ClosingMessage: <Message to display after saving>
```

> **If invoked directly without a rule file supplying these parameters**, ask the user to provide each missing input before proceeding.

## Procedure

### Step 1 — Derive the completeness checklist

Use `AnalysisSource` to produce the checklist:
- If `AnalysisSource` references a named skill, invoke that skill with the appropriate inputs and wait for it to finish (zero Falsified rows). Collect the mapping table, Mermaid class diagram, and unchecked checklist items ordered by dependency.
- Otherwise, derive the checklist directly from the provided description, listing each distinct piece of functionality as an unchecked item.

### Step 2 — Run the Test Standards skill for each checklist item

For every unchecked item in the checklist (in dependency order):
1. Derive a plain-English functional description from `AnalysisSource` (e.g. from a `.cpp` implementation, interface contract, or provided description).
2. Invoke the Test Standards skill with the C# method signature and that description.
3. Wait for the skill to finish (zero Falsified rows) and collect the named test list.

### Step 3 — Enforce mandatory items

For each entry in `MandatoryItems`:
- If the item is already present in the checklist, leave it as-is.
- If it is missing, add it as a **mandatory unchecked item** with its specified tag.
- For each newly added mandatory item, generate test cases following Step 2 rules.

If `MandatoryItems` is `"none"`, skip this step.

### Step 4 — Assemble the output

Combine the checklist and test plan into a single structured document using `OutputTitle` as the H1 heading and a brief scope statement as the blockquote.

### Step 5 — Validate format

Invoke the Plan Format Gate skill with the value of `PlanType` against the assembled document from Step 4.

- If the verdict is **FAIL**, fix every listed violation in-place and re-run the gate until it returns **PASS**.
- Only proceed to Step 6 after a **PASS** verdict.

### Step 6 — Save to the output path

Write the assembled document to `OutputPath`.
Create the file if it does not exist; overwrite it if it does.
**This step is mandatory — do not skip it.**

## Output Format

```markdown
<OutputTitle>

> <Scope statement derived from caller inputs>

---

## Functionality Worktree

### Class Diagram

<Mermaid diagram, if produced by AnalysisSource; omit section if not applicable>

### Completeness Checklist

- [ ] <item 1> [<tag>]
- [ ] <item 2> [<tag>]
...

---

## Test Plan

### `<Method or feature name>`

1. `<TestName_GivenScenario_ExpectedResult>`
   *Assumption*: ...
...

---

*All assumptions verified by Falsify Claims. Zero Falsified rows.*
```

After saving the file, display `ClosingMessage`.

````