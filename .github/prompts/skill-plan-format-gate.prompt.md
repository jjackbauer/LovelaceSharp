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

---

## Extended Rules for Journal and Distilled Document Types

The following conditional rules activate when `PlanType` is `"journal-entry"` or `"distilled"`.
They are checked **in addition to** Rules 1–8 above (where applicable).

---

### When `PlanType = "journal-entry"`

These rules validate a document that contains OBS, HYP, VAL, DEC, TODO, RISK, OQ, or ART entries.

#### Rule J1 — Entry IDs follow `{TYPE}-{NNN}` format

All journal entry IDs in the document must match the pattern `^(OBS|HYP|VAL|DEC|TODO|RISK|OQ|ART)-\d{3,}$`.
IDs without a type prefix, with an invalid type, or with fewer than three digits are flagged.

#### Rule J2 — Required fields present per entry type

For each entry, check that all required fields defined in `journal-schema.md` are present:

| Entry type | Required fields |
|---|---|
| OBS | Source, Fact, Implications, Confidence, (Related optional) |
| HYP | Claim, Supporting OBS, Why it matters, Falsification strategy, Status, Confidence |
| VAL | Target HYP, Method, Evidence examined, Result, Conclusion, (Related optional) |
| DEC | Decision, Rationale, Alternatives considered, (Related optional) |
| TODO | Task, Priority, Depends on, Status, (Related optional) |
| RISK | Risk, Likelihood, Impact, Evidence, Mitigation |
| OQ | Question, Needed evidence, Priority, (Related optional) |
| ART | Artifact path, Type, Supporting evidence, Confidence, Last updated |

Entries missing a required field are flagged with the field name.

#### Rule J3 — HYP entries have a non-vague Falsification strategy

For every HYP entry, the **Falsification strategy** field must contain a specific file path or code
location to examine, **not** a generic phrase like "check the code" or "look at the source".
Strategies shorter than 20 words or containing only generic language are flagged.

#### Rule J4 — Confidence values are constrained

All Confidence fields must be exactly one of: `High`, `Medium`, or `Low`.
Any other value (e.g., `high`, `medium`, `Very High`) is flagged.

#### Rule J5 — HYP Status values follow the lifecycle

All HYP Status fields must be exactly one of: `Proposed`, `Under review`, `Supported`, `Falsified`,
`Superseded`. Any other value is flagged.

#### Rule J6 — VAL Result values are constrained

All VAL Result fields must be exactly one of: `Supported`, `Falsified`, `Unresolved`.
Any other value is flagged.

---

### When `PlanType = "distilled"`

These rules validate a distilled knowledge document under `.github/distilled/`.

#### Rule D1 — Standard header present

The document must begin with an H1 title followed immediately by a metadata blockquote containing
all four required fields: **Scope**, **Confidence**, **Last updated**, and **Source entries**.
A document missing any of these header fields is flagged.

#### Rule D2 — All claims carry exactly one uncertainty marker

Every bullet point or sentence within the document body that makes a factual claim must begin with
exactly one of: `✅`, `⚠️`, or `❓`. Claims without a marker are flagged.
Claims with more than one marker on the same line are also flagged.

#### Rule D3 — Claims cite source entries

Every claim marked with ✅ or ⚠️ must include at least one journal entry ID in parentheses
(e.g., `(OBS-001)` or `(OBS-003, VAL-005)`). Claims without parenthetical citations are flagged.

#### Rule D4 — `trusted-facts.md` contains only ✅ markers

If the document being validated is `trusted-facts.md`, any ⚠️ or ❓ markers found are flagged
as violations. That document is strictly ✅-only.

#### Rule D5 — Overall Confidence matches marker distribution

The document-level **Confidence** header field must be consistent with the marker distribution:

| Confidence | Required distribution |
|---|---|
| High | All claims are ✅; zero ⚠️ or ❓ |
| Medium | Majority ✅ or ⚠️; ❓ are a minority |
| Low | Significant ❓ present |

A mismatch between the header Confidence and the observed marker distribution is flagged.

---

### Extended Output Format

When `PlanType = "journal-entry"` or `"distilled"`, append a second results table after the main
Rules 1–8 table:

| # | Rule | Status | Violations |
|---|---|---|---|
| J1 | Entry ID format | ✅ Pass / ❌ Fail | — |
| J2 | Required fields per type | ✅ Pass / ❌ Fail | — |
| J3 | Non-vague falsification strategy | ✅ Pass / ❌ Fail | — |
| J4 | Confidence values | ✅ Pass / ❌ Fail | — |
| J5 | HYP status lifecycle | ✅ Pass / ❌ Fail | — |
| J6 | VAL result values | ✅ Pass / ❌ Fail | — |

Or for `"distilled"`:

| # | Rule | Status | Violations |
|---|---|---|---|
| D1 | Standard header present | ✅ Pass / ❌ Fail | — |
| D2 | All claims have one marker | ✅ Pass / ❌ Fail | — |
| D3 | Claims cite source entries | ✅ Pass / ❌ Fail | — |
| D4 | trusted-facts.md ✅-only | ✅ Pass / ⏭️ Skipped / ❌ Fail | — |
| D5 | Confidence matches markers | ✅ Pass / ❌ Fail | — |

The self-healing loop applies to these extended rules as well.

````
