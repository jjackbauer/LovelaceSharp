# Journal Schema

> **Usage**: Include this file in any prompt via `#file:.github/prompts/journal-schema.md`.
> This file is a reference-only document — it is not a runnable prompt.
>
> **Purpose**: Defines the canonical entry templates, field definitions, ID format, and
> confidence levels for all journal files under `.github/journals/`. Every skill that reads
> or writes journal entries must conform to this schema.

---

## 1. Entry ID Format

All journal entries use a sequential ID in the format:

```
{TYPE}-{NNN}
```

- **`{TYPE}`** — uppercase entry type prefix (see §2 for the full list).
- **`{NNN}`** — zero-padded three-digit sequence number, starting at `001` and incrementing per type.

Examples: `OBS-001`, `HYP-042`, `VAL-003`, `DEC-010`, `TODO-007`, `RISK-002`, `OQ-015`, `ART-001`.

When the sequence exceeds 999, extend to four digits (e.g., `OBS-1000`).

---

## 2. Entry Types

| Type Prefix | Full Name | Journal File | Purpose |
|---|---|---|---|
| `OBS` | Observation | `.github/journals/observations.md` | Grounded factual finding from source code |
| `HYP` | Hypothesis | `.github/journals/hypotheses.md` | Testable claim derived from observations |
| `VAL` | Validation | `.github/journals/validations.md` | Falsification or confirmation of a hypothesis |
| `DEC` | Decision | `.github/journals/decisions.md` | Architectural or implementation decision with rationale |
| `TODO` | Todo | `.github/journals/todos.md` | Actionable exploration or implementation task |
| `RISK` | Risk | `.github/journals/risks.md` | Identified risk with likelihood and impact |
| `OQ` | Open Question | `.github/journals/open-questions.md` | Unresolved question requiring further investigation |
| `ART` | Artifact Index | `.github/journals/artifact-index.md` | Registry of generated downstream artifacts |

---

## 3. Confidence Levels

Used by OBS, HYP, RISK, and ART entries to express certainty:

| Level | Meaning |
|---|---|
| **High** | Directly confirmed by source code, test output, or build artefact |
| **Medium** | Supported by multiple observations but not formally validated |
| **Low** | Inferred from naming, structure, analogy, or incomplete evidence |

---

## 4. Entry Templates

### 4.1 OBS — Observation

```markdown
### OBS-{NNN}: {Short title}

- **Source**: `{file}:{line}` (or `{file}:{startLine}-{endLine}`)
- **Fact**: {One or two sentence factual statement grounded in code}
- **Implications**: {What this fact means for the broader system understanding}
- **Confidence**: {High | Medium | Low}
- **Related**: {Comma-separated list of related entry IDs, e.g., HYP-003, OBS-012}
```

**Field definitions**:

| Field | Required | Description |
|---|---|---|
| Source | Yes | File path and line number(s) where the fact was observed. Format: `file:line` or `file:startLine-endLine`. |
| Fact | Yes | A concrete, verifiable statement about what the code does. Must be grounded in read source — no inferences. |
| Implications | Yes | What this observation means for architecture, migration, or system understanding. |
| Confidence | Yes | One of: High, Medium, Low (see §3). |
| Related | No | Cross-references to other journal entries. Comma-separated IDs. |

---

### 4.2 HYP — Hypothesis

```markdown
### HYP-{NNN}: {Short title}

- **Claim**: {Testable assertion about system behaviour or structure}
- **Supporting OBS**: {Comma-separated OBS IDs that motivate this hypothesis}
- **Why it matters**: {Impact on architecture, migration, or correctness if true/false}
- **Falsification strategy**: {Specific steps to disprove this claim — what to look for and where}
- **Status**: {Proposed | Under review | Supported | Falsified | Superseded}
- **Confidence**: {High | Medium | Low}
```

**Field definitions**:

| Field | Required | Description |
|---|---|---|
| Claim | Yes | A testable assertion. Must be specific enough to be falsified. |
| Supporting OBS | Yes | One or more OBS IDs that motivate the hypothesis. |
| Why it matters | Yes | The consequence of this claim being true or false. |
| Falsification strategy | Yes | Concrete steps an agent should take to attempt to disprove the claim. Without this field, the entry is invalid. |
| Status | Yes | Lifecycle state — see §5 below. |
| Confidence | Yes | One of: High, Medium, Low (see §3). |

**Status lifecycle**:

```
Proposed → Under review → Supported
                        → Falsified
                        → Superseded
```

- **Proposed**: Initial state when the hypothesis is first recorded.
- **Under review**: A validation attempt is in progress.
- **Supported**: Validation found confirming evidence and no counter-evidence.
- **Falsified**: Validation found a concrete counterexample or contradicting code.
- **Superseded**: A newer, more accurate hypothesis replaces this one (link to replacement in Related).

---

### 4.3 VAL — Validation

```markdown
### VAL-{NNN}: {Short title}

- **Target HYP**: {HYP-NNN being validated}
- **Method**: {Description of the validation approach — which tactics were used}
- **Evidence examined**: {List of files, lines, tests, or artifacts inspected}
- **Result**: {Supported | Falsified | Unresolved}
- **Conclusion**: {Summary of findings and their implications}
- **Related**: {Comma-separated list of related entry IDs}
```

**Field definitions**:

| Field | Required | Description |
|---|---|---|
| Target HYP | Yes | The HYP entry ID being validated. |
| Method | Yes | The falsification tactics applied. Must reference at least the approach taken. |
| Evidence examined | Yes | Specific files, line ranges, test names, or build outputs inspected. Must include citations. |
| Result | Yes | One of: **Supported** (confirmed, no counter-evidence), **Falsified** (counter-evidence found), **Unresolved** (insufficient evidence either way). |
| Conclusion | Yes | A concise summary of what was found and what it means. |
| Related | No | Cross-references to OBS, HYP, or other VAL entries relevant to this validation. |

---

### 4.4 DEC — Decision

```markdown
### DEC-{NNN}: {Short title}

- **Decision**: {What was decided}
- **Rationale**: {Why — with links to supporting evidence (OBS, HYP, VAL IDs)}
- **Alternatives considered**: {What other options were evaluated and why they were rejected}
- **Related**: {Comma-separated list of related entry IDs}
```

**Field definitions**:

| Field | Required | Description |
|---|---|---|
| Decision | Yes | The architectural or implementation decision made. |
| Rationale | Yes | Justification grounded in journal evidence. Must reference at least one OBS, HYP, or VAL ID. |
| Alternatives considered | Yes | Other options that were evaluated, with brief reasons for rejection. |
| Related | No | Cross-references to entries that informed this decision. |

---

### 4.5 TODO — Todo

```markdown
### TODO-{NNN}: {Short title}

- **Task**: {Description of what needs to be done}
- **Priority**: {P0 | P1 | P2}
- **Depends on**: {Entry IDs or conditions that must be met first}
- **Status**: {Open | In progress | Done | Cancelled}
- **Related**: {Comma-separated list of related entry IDs}
```

**Field definitions**:

| Field | Required | Description |
|---|---|---|
| Task | Yes | Clear, actionable description of the work to be done. |
| Priority | Yes | **P0** = blocking / must do next, **P1** = important / do soon, **P2** = nice to have / defer. |
| Depends on | No | Entry IDs or named conditions that must be resolved before this task can start. |
| Status | Yes | **Open** (not started), **In progress** (actively being worked), **Done** (completed), **Cancelled** (no longer needed). |
| Related | No | Cross-references to entries that motivated this task. |

---

### 4.6 RISK — Risk

```markdown
### RISK-{NNN}: {Short title}

- **Risk**: {Description of what could go wrong}
- **Likelihood**: {High | Medium | Low}
- **Impact**: {High | Medium | Low}
- **Evidence**: {OBS/HYP/VAL IDs or file:line references that support the risk assessment}
- **Mitigation**: {Steps to reduce likelihood or impact}
```

**Field definitions**:

| Field | Required | Description |
|---|---|---|
| Risk | Yes | A clear statement of the risk. |
| Likelihood | Yes | One of: High, Medium, Low — probability of the risk materialising. |
| Impact | Yes | One of: High, Medium, Low — severity if the risk materialises. |
| Evidence | Yes | Journal entry IDs or source locations that support the risk assessment. |
| Mitigation | Yes | Concrete steps to reduce or eliminate the risk. |

---

### 4.7 OQ — Open Question

```markdown
### OQ-{NNN}: {Short title}

- **Question**: {The specific question to be resolved}
- **Needed evidence**: {What kind of evidence would answer the question}
- **Priority**: {P0 | P1 | P2}
- **Related**: {Comma-separated list of related entry IDs}
```

**Field definitions**:

| Field | Required | Description |
|---|---|---|
| Question | Yes | A specific, answerable question. Not a vague area of concern. |
| Needed evidence | Yes | Description of what evidence (code, test result, documentation) would resolve the question. |
| Priority | Yes | **P0** = blocking exploration, **P1** = important for accuracy, **P2** = low-priority curiosity. |
| Related | No | Cross-references to entries that raised or relate to this question. |

---

### 4.8 ART — Artifact Index

```markdown
### ART-{NNN}: {Artifact name}

- **Artifact path**: {Relative path to the generated artifact, e.g., `.github/artifacts/architecture-report.md`}
- **Type**: {architecture-report | migration-plan | risk-assessment | implementation-plan}
- **Supporting evidence**: {Comma-separated list of OBS, HYP, and VAL IDs that back this artifact}
- **Confidence**: {High | Medium | Low}
- **Last updated**: {YYYY-MM-DD}
```

**Field definitions**:

| Field | Required | Description |
|---|---|---|
| Artifact path | Yes | Relative path from repository root to the generated document. |
| Type | Yes | One of: **architecture-report**, **migration-plan**, **risk-assessment**, **implementation-plan**. |
| Supporting evidence | Yes | Journal entry IDs (OBS, HYP, VAL) that provide the evidence base for the artifact. |
| Confidence | Yes | One of: High, Medium, Low (see §3). Reflects the overall trustworthiness of the artifact. |
| Last updated | Yes | ISO 8601 date (`YYYY-MM-DD`) of the most recent update. |

---

## 5. Append-Only Convention

Journal files are **append-only**. New entries are added at the end of the file. Existing entries are never deleted.

The only permitted in-place edits are:
- Updating the **Status** field of a HYP entry (e.g., Proposed → Supported).
- Updating the **Status** field of a TODO entry (e.g., Open → Done).
- Adding a cross-reference to the **Related** field of an existing entry.

All other modifications require a new entry that supersedes the old one (with a link in Related).

---

## 6. Journal File Structure

Each journal file follows this structure:

```markdown
# {Journal Title}

> Append-only journal. See `.github/prompts/journal-schema.md` for entry format.

---

<!-- New entries go below this line -->
```

The `<!-- New entries go below this line -->` comment marks the append point. Skills append new entries immediately after this marker.
