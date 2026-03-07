````prompt
---
agent: agent
description: Validate a plan document's structure against the meta-structural conventions established in .github/requirements/.
---

#file:.github/prompts/skill-falsify-claims.prompt.md

# Skill: Plan Format Gate

## Purpose
Given a plan document (migration requirements, parallelization audit, or generic), validate its Markdown structure against the formatting conventions used in `.github/requirements/`. Return a pass/fail verdict with a concrete fix-list when violations are found. This skill checks **structure only** — content correctness is the responsibility of the Falsify Claims skill.

## Input (supplied by caller)

```
Document:  <Full Markdown text of the plan document, or a file path to one>
PlanType:  <"migration requirements" | "parallelization audit" | "generic">
```

## Procedure

Apply the following gate checks to the document. Each rule produces a **Pass** or **Fail** result.

### Rule 1 — Heading structure & separators

- The document must contain **exactly one H1** (`# ...`).
- **H2** (`## ...`) for major sections, **H3** (`### ...`) for subsections.
- No heading-level skips (e.g., H1 → H3 without an intervening H2).
- A horizontal rule (`---`) must appear before each H2 section (except when the H2 immediately follows the H1 scope statement).

### Rule 2 — Scope statement

- A blockquote (`> ...`) or plain paragraph must appear **immediately after the H1**, stating the document's purpose or scope.
- An H2 appearing as the very next non-blank element after the H1 (with no prose in between) is a violation.

### Rule 3 — Pipe tables

- The document must contain **at least one** pipe table with a header row and an alignment row (`|---|`).
- Tables missing the alignment row are flagged.

### Rule 4 — Checklists with dependency tags

- Every `- [ ]` or `- [x]` item that describes a dependency or prerequisite must include a **bracketed tag** (e.g., `[prerequisite for many others]`, `[depends on X]`, `[mandatory — reason]`).
- Bare checklist items with no contextual annotation (no parenthesised interface reference, no bracketed tag, and no inline description following the method signature) are flagged.
- This rule is lenient for items that already carry sufficient context via an inline description or parenthesised interface note (e.g., `- [ ] \`Add(Integer)\` → \`operator+\` (\`IAdditionOperators<T,T,T>\`)`).

### Rule 5 — Mermaid diagrams (conditional)

- **Only checked when** the document contains a heading with "Class Diagram", "Dependency Graph", or "Sequential Dependency" in its text.
- When triggered: at least one fenced ` ```mermaid ` block must be present.

### Rule 6 — Verification gate

- The document must contain evidence that the Falsify Claims skill was run:
  - A blockquote or paragraph containing "Zero Falsified rows", **or**
  - A Falsify Claims result table where every row's Status column is marked Supported.
- Documents with no verification evidence are flagged.

### Rule 7 — Closing verification line

- Within the **last 5 lines** of the document, an italicised verification confirmation must appear (e.g., `*All assumptions verified... Zero Falsified rows.*` or `*All assumptions derived from... Zero Falsified rows...*`).
- The italicised line must contain the phrase "Zero Falsified" (case-insensitive).

### Rule 8 — Numbered test plan items (conditional)

- **Only checked when** the document contains a `## Test Plan` section.
- Each item under a Test Plan subsection must follow the pattern:
  ```
  N. `MethodName_GivenScenario_ExpectedResult`
     *Assumption*: <one-sentence assumption text>
  ```
- Items missing the numbered format, the backtick-wrapped test name, or the `*Assumption*:` line are flagged.

## Output Format

Produce a Markdown table summarising each rule:

| # | Rule | Status | Violations |
|---|---|---|---|
| 1 | Heading structure & separators | ✅ Pass | — |
| 2 | Scope statement under H1 | ✅ Pass | — |
| 3 | Pipe tables present | ✅ Pass | — |
| 4 | Checklists with dependency tags | ❌ Fail | 2 bare checklist items at lines 45, 52 |
| 5 | Mermaid diagrams | ⏭️ Skipped | No triggering heading found |
| 6 | Verification gate | ✅ Pass | — |
| 7 | Closing verification line | ❌ Fail | Missing italicised "Zero Falsified" in last 5 lines |
| 8 | Numbered test plan items | ✅ Pass | — |

Use these status indicators:
- **✅ Pass** — rule satisfied
- **❌ Fail** — rule violated; list specific violations
- **⏭️ Skipped** — conditional rule not applicable to this document

Follow the table with a verdict line:

- If all non-skipped rules pass: `**PASS** — document conforms to plan format.`
- If any rule fails: `**FAIL** — N violation(s) found. Fix the items above and re-run this gate.`

## Loop Instruction (Self-Healing)

If the verdict is **FAIL**, the gate **itself** must:

1. Apply fixes directly to the document — edit the Markdown in-place to resolve each listed violation.
2. Re-run all 8 rules against the corrected document and produce a new results table.
3. Repeat until the verdict is **PASS**, or until **3 iterations** have been exhausted (to prevent infinite loops).

After reaching **PASS** (or exhausting iterations), return:
- The **final verdict** (PASS or FAIL with remaining violations).
- The **corrected document content** with all applied fixes.

Do not ask the caller to fix violations manually — resolve them autonomously.

````
